//! Person Cache Builder - FM 2023 CSV → Binary Cache Pipeline
//!
//! Handles 8,452 players from FM 2023.csv (98 columns including 36 FM attributes)
//! CSV → Vec<Person> → FxHashMap<u32, Person> → MessagePack → LZ4
//!
//! ## Phase 21: Complete Data Extraction (2026-01-09)
//!
//! - Extracts ALL 98 columns from FM 2023.csv
//! - Schema v5: Person struct with 84 fields:
//!   - Basic (8): UID, Name, Nationality, Team, Position, CA, PA, Age
//!   - FM Attributes (36): Technical×14, Mental×14, Physical×8
//!   - Hidden/Physical (5): Stability, Foul, Contest, Injury, Versatility
//!   - Goalkeeper (11): Aerial Reach, Command, Communication, Eccentricity,
//!                      Handling, Kicking, 1v1, Reflexes, Rushing, Punching, Throwing
//!   - Personality (8): Adaptation, Ambition, Argue, Loyal, Pressure,
//!                      Professional, Sportsmanship, Temperament
//!   - Physical Info (4): Height, Weight, Left Foot, Right Foot
//!   - Career/Financial (6): Value, Reputation×3, Salary, Loan Club
//!   - Personal Info (6): Ethnicity, RCA, Skin, Birth Date, Caps, Goals
//!   - Position Ratings (14): GK through ST as comma-separated string
//!
//! ## Embedded Data Support (2025-12-05)
//!
//! For release builds, player cache can be embedded directly in the binary
//! using `include_bytes!`. This eliminates external file dependencies.
//! Use `load_person_cache_embedded()` for zero-file-IO loading.

use anyhow::{Context, Result};
use of_core::models::Person;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Runtime index for fast player lookup
///
/// FxHashMap provides O(1) lookup by UID
/// Optimized for performance over HashMap (30-40% faster for integer keys)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonIndex {
    /// Player lookup by UID
    pub players: FxHashMap<u32, Person>,
    /// Total player count
    pub count: u32,
    /// Schema version
    pub schema_version: String,
}

impl PersonIndex {
    /// Create new empty index
    pub fn new(schema_version: String) -> Self {
        Self {
            players: FxHashMap::default(),
            count: 0,
            schema_version,
        }
    }

    /// Add person to index
    pub fn insert(&mut self, person: Person) {
        self.players.insert(person.uid, person);
        self.count = self.players.len() as u32;
    }

    /// Get person by UID
    pub fn get(&self, uid: u32) -> Option<&Person> {
        self.players.get(&uid)
    }

    /// Get total count
    pub fn len(&self) -> usize {
        self.players.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }
}

/// CSV parsing statistics
#[derive(Debug, Clone)]
pub struct ParseStats {
    pub total_rows: u32,
    pub parsed: u32,
    pub failed: u32,
    pub skipped_header: bool,
}

impl ParseStats {
    fn new() -> Self {
        Self {
            total_rows: 0,
            parsed: 0,
            failed: 0,
            skipped_header: false,
        }
    }
}

/// Parse FM 2023 CSV file into PersonIndex
///
/// Expected CSV format (98 columns from FM 2023):
/// - Columns 0-6: Name, Position, Age, CA, PA, Nationality, Club
/// - Columns 7-20: Technical attributes (14)
/// - Columns 21-34: Mental attributes (14)
/// - Columns 35-42: Physical attributes (8)
/// - Columns 43-47: Hidden/Physical attributes (5)
/// - Columns 48-58: Goalkeeper attributes (11)
/// - Columns 59-66: Personality attributes (8)
/// - Columns 67-80: Position ratings (14)
/// - Columns 81-84: Physical info (Height, Weight, Left/Right Foot)
/// - Columns 85-88: Career/Financial (Value, Reputation×3)
/// - Columns 89-96: Personal info (Race, RCA, Skin, DOB, Caps, Goals, Salary, Loan)
/// - Column 97: UID (ignored - using row index instead)
///
/// Handles quoted fields and exports all 98 columns to Person struct v5.
///
/// # Arguments
///
/// * `csv_path` - Path to FM 2023.csv
/// * `schema_version` - Schema version string (e.g., "v4")
/// * `skip_header` - Whether to skip first row (default: true)
///
/// # Returns
///
/// * `Ok((PersonIndex, ParseStats))` - Successfully parsed index and statistics
/// * `Err(antml:Error)` - File I/O or parsing error
pub fn parse_csv_to_index(
    csv_path: &Path,
    schema_version: &str,
    skip_header: bool,
) -> Result<(PersonIndex, ParseStats)> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(skip_header)
        .from_path(csv_path)
        .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?;

    let mut index = PersonIndex::new(schema_version.to_string());
    let mut stats = ParseStats::new();
    stats.skipped_header = skip_header;

    // Helper macro for parsing u8 fields with error handling
    macro_rules! parse_u8 {
        ($record:expr, $idx:expr, $name:expr, $row:expr, $stats:expr) => {
            match $record[$idx].trim().parse::<u8>() {
                Ok(v) => v,
                Err(_) => {
                    $stats.failed += 1;
                    eprintln!(
                        "Warning: Line {} - Invalid {} value: '{}'",
                        $row,
                        $name,
                        $record[$idx].trim()
                    );
                    continue;
                }
            }
        };
    }

    // Helper macro for parsing u16 fields with error handling
    macro_rules! parse_u16 {
        ($record:expr, $idx:expr, $name:expr, $row:expr, $stats:expr) => {
            match $record[$idx].trim().parse::<u16>() {
                Ok(v) => v,
                Err(_) => {
                    $stats.failed += 1;
                    eprintln!(
                        "Warning: Line {} - Invalid {} value: '{}'",
                        $row,
                        $name,
                        $record[$idx].trim()
                    );
                    continue;
                }
            }
        };
    }

    // Helper macro for parsing u32 fields with error handling
    macro_rules! parse_u32 {
        ($record:expr, $idx:expr, $name:expr, $row:expr, $stats:expr) => {
            match $record[$idx].trim().parse::<u32>() {
                Ok(v) => v,
                Err(_) => {
                    $stats.failed += 1;
                    eprintln!(
                        "Warning: Line {} - Invalid {} value: '{}'",
                        $row,
                        $name,
                        $record[$idx].trim()
                    );
                    continue;
                }
            }
        };
    }

    // Helper macro for parsing optional u8 (returns 0 on empty/error - for non-GK attributes)
    macro_rules! parse_u8_optional {
        ($record:expr, $idx:expr) => {
            $record[$idx].trim().parse::<u8>().unwrap_or(0)
        };
    }

    // Helper macro for parsing optional u16 (returns 0 on empty/error)
    macro_rules! parse_u16_optional {
        ($record:expr, $idx:expr) => {
            $record[$idx].trim().parse::<u16>().unwrap_or(0)
        };
    }

    // Helper macro for parsing optional u32 (returns 0 on empty/error)
    macro_rules! parse_u32_optional {
        ($record:expr, $idx:expr) => {
            $record[$idx].trim().parse::<u32>().unwrap_or(0)
        };
    }

    for result in reader.records() {
        stats.total_rows += 1;
        // Stable UID policy: UID == 1-based CSV row index (excluding header).
        let uid = stats.total_rows;

        match result {
            Ok(record) => {
                // Validate FM 2023 CSV format (98 columns minimum)
                if record.len() < 98 {
                    stats.failed += 1;
                    eprintln!(
                        "Warning: Line {} has {} fields (expected 98 for FM 2023), skipping",
                        stats.total_rows,
                        record.len()
                    );
                    continue;
                }

                // Parse basic info (columns 0-6)
                let name = record[0].trim().trim_start_matches('\u{feff}').to_string(); // Strip BOM
                let position = record[1].trim().to_string();
                let age = parse_u8!(record, 2, "Age", stats.total_rows, stats);
                let ca = parse_u8!(record, 3, "CA", stats.total_rows, stats);

                // Parse PA with variable potential conversion
                // FM negative PA values represent variable potential ranges - use high-end values
                let pa_raw = record[4].trim();
                let pa = match pa_raw.parse::<i16>() {
                    Ok(v) if v >= 0 => v as u8,
                    Ok(v) => match v {
                        -10 => 200, // 170-200 → 200 (최고 엘리트, FM23에 1명만 존재)
                        -95 => 190, // 160-190 → 190 (슈퍼 원더키드, FM23에 5명)
                        -9 => 180,  // 150-180 → 180 (원더키드)
                        -85 => 170, // 140-170 → 170 (고잠재력)
                        -8 => 160,  // 130-160 → 160 (중급 잠재력)
                        -75 => 150, // 120-150 → 150 (기본 잠재력)
                        _ => {
                            // Unknown negative value, skip this row
                            stats.failed += 1;
                            eprintln!(
                                "Warning: Line {} - Unknown variable PA value: '{}', skipping",
                                stats.total_rows, pa_raw
                            );
                            continue;
                        }
                    },
                    Err(_) => {
                        stats.failed += 1;
                        eprintln!(
                            "Warning: Line {} - Invalid PA value: '{}'",
                            stats.total_rows, pa_raw
                        );
                        continue;
                    }
                };

                let nationality = record[5].trim().to_string();
                let team = record[6].trim().to_string();

                // Parse Technical attributes (columns 7-20, 14 attributes)
                let corners = parse_u8!(record, 7, "Corners", stats.total_rows, stats);
                let crossing = parse_u8!(record, 8, "Crossing", stats.total_rows, stats);
                let dribbling = parse_u8!(record, 9, "Dribbling", stats.total_rows, stats);
                let finishing = parse_u8!(record, 10, "Finishing", stats.total_rows, stats);
                let first_touch = parse_u8!(record, 11, "First Touch", stats.total_rows, stats);
                let free_kick_taking =
                    parse_u8!(record, 12, "Free Kick Taking", stats.total_rows, stats);
                let heading = parse_u8!(record, 13, "Heading", stats.total_rows, stats);
                let long_shots = parse_u8!(record, 14, "Long Shots", stats.total_rows, stats);
                let long_throws = parse_u8!(record, 15, "Long Throws", stats.total_rows, stats);
                let marking = parse_u8!(record, 16, "Marking", stats.total_rows, stats);
                let passing = parse_u8!(record, 17, "Passing", stats.total_rows, stats);
                let penalty_taking =
                    parse_u8!(record, 18, "Penalty Taking", stats.total_rows, stats);
                let tackling = parse_u8!(record, 19, "Tackling", stats.total_rows, stats);
                let technique = parse_u8!(record, 20, "Technique", stats.total_rows, stats);

                // Parse Mental attributes (columns 21-34, 14 attributes)
                // Note: Column 21 is "Aggressiion" (typo in CSV), column 27 is "Decision" (singular)
                let aggression = parse_u8!(record, 21, "Aggression", stats.total_rows, stats);
                let anticipation = parse_u8!(record, 22, "Anticipation", stats.total_rows, stats);
                let bravery = parse_u8!(record, 23, "Bravery", stats.total_rows, stats);
                let composure = parse_u8!(record, 24, "Composure", stats.total_rows, stats);
                let concentration = parse_u8!(record, 25, "Concentration", stats.total_rows, stats);
                let vision = parse_u8!(record, 26, "Vision", stats.total_rows, stats);
                let decisions = parse_u8!(record, 27, "Decision", stats.total_rows, stats);
                let determination = parse_u8!(record, 28, "Determination", stats.total_rows, stats);
                let flair = parse_u8!(record, 29, "Flair", stats.total_rows, stats);
                let leadership = parse_u8!(record, 30, "Leadership", stats.total_rows, stats);
                let off_the_ball = parse_u8!(record, 31, "Off The Ball", stats.total_rows, stats);
                let positioning = parse_u8!(record, 32, "Position", stats.total_rows, stats); // Duplicate column name
                let teamwork = parse_u8!(record, 33, "Teamwork", stats.total_rows, stats);
                let work_rate = parse_u8!(record, 34, "Work Rate", stats.total_rows, stats);

                // Parse Physical attributes (columns 35-42, 8 attributes)
                let acceleration = parse_u8!(record, 35, "Acceleration", stats.total_rows, stats);
                let agility = parse_u8!(record, 36, "Agility", stats.total_rows, stats);
                let balance = parse_u8!(record, 37, "Balance", stats.total_rows, stats);
                let jumping = parse_u8!(record, 38, "Jumping Reach", stats.total_rows, stats);
                let natural_fitness =
                    parse_u8!(record, 39, "Natural Fitness", stats.total_rows, stats);
                let pace = parse_u8!(record, 40, "Pace", stats.total_rows, stats);
                let stamina = parse_u8!(record, 41, "Stamina", stats.total_rows, stats);
                let strength = parse_u8!(record, 42, "Strength", stats.total_rows, stats);

                // Parse Hidden/Physical attributes (columns 43-47, 5 attributes) - v5
                let stability = parse_u8!(record, 43, "Stability", stats.total_rows, stats);
                let foul = parse_u8!(record, 44, "Foul", stats.total_rows, stats);
                let contest_performance =
                    parse_u8!(record, 45, "Contest performance", stats.total_rows, stats);
                let injury_proneness = parse_u8!(record, 46, "Injury", stats.total_rows, stats);
                let versatility = parse_u8!(record, 47, "diversity", stats.total_rows, stats);

                // Parse Goalkeeper attributes (columns 48-58, 11 attributes) - v5
                // Note: GK attributes may be 0 or empty for non-GK players
                let aerial_reach = parse_u8_optional!(record, 48);
                let command_of_area = parse_u8_optional!(record, 49);
                let communication = parse_u8_optional!(record, 50);
                let eccentricity = parse_u8_optional!(record, 51);
                let handling = parse_u8_optional!(record, 52);
                let gk_kicking = parse_u8_optional!(record, 53);
                let one_on_ones = parse_u8_optional!(record, 54);
                let reflexes = parse_u8_optional!(record, 55);
                let rushing_out = parse_u8_optional!(record, 56);
                let punching = parse_u8_optional!(record, 57);
                let throwing = parse_u8_optional!(record, 58);

                // Parse Personality attributes (columns 59-66, 8 attributes) - v5
                let adaptation = parse_u8!(record, 59, "Adaptation", stats.total_rows, stats);
                let ambition = parse_u8!(record, 60, "Ambition", stats.total_rows, stats);
                let controversy = parse_u8!(record, 61, "Argue", stats.total_rows, stats);
                let loyalty = parse_u8!(record, 62, "Loyal", stats.total_rows, stats);
                let pressure =
                    parse_u8!(record, 63, "Resistant to stress", stats.total_rows, stats);
                let professionalism = parse_u8!(record, 64, "Professional", stats.total_rows, stats);
                let sportsmanship = parse_u8!(record, 65, "Sportsmanship", stats.total_rows, stats);
                let temperament =
                    parse_u8!(record, 66, "Emotional control", stats.total_rows, stats);

                // Parse Position Ratings (columns 67-80, 14 positions)
                // Format: GK,DL,DC,DR,WBL,WBR,DM,ML,MC,MR,AML,AMC,AMR,ST
                let position_ratings_str = format!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    record[67].trim(),
                    record[68].trim(),
                    record[69].trim(),
                    record[70].trim(),
                    record[71].trim(),
                    record[72].trim(),
                    record[73].trim(),
                    record[74].trim(),
                    record[75].trim(),
                    record[76].trim(),
                    record[77].trim(),
                    record[78].trim(),
                    record[79].trim(),
                    record[80].trim()
                );

                // Parse Physical Info (columns 81-84) - v5
                let height_cm = parse_u16_optional!(record, 81);
                let weight_kg = parse_u8_optional!(record, 82);
                let left_foot = parse_u8_optional!(record, 83);
                let right_foot = parse_u8_optional!(record, 84);

                // Parse Career/Financial (columns 85-88) - v5
                let market_value = parse_u32_optional!(record, 85);
                let reputation_current = parse_u16_optional!(record, 86);
                let reputation_domestic = parse_u16_optional!(record, 87);
                let reputation_world = parse_u16_optional!(record, 88);

                // Parse Personal Info (columns 89-96) - v5
                let ethnicity = record[89].trim().to_string();
                let rca = parse_u16_optional!(record, 90);
                let skin_color = parse_u8_optional!(record, 91);
                let birth_date = record[92].trim().to_string();
                let national_team_caps = parse_u16_optional!(record, 93);
                let national_team_goals = parse_u16_optional!(record, 94);
                let salary = parse_u32_optional!(record, 95);
                let loan_club = record[96].trim().to_string();

                // Construct Person using struct literal (direct construction) - v5 schema
                let person = Person {
                    uid,
                    name,
                    nationality,
                    team,
                    position,
                    ca,
                    pa,
                    age,
                    // Technical (14)
                    corners,
                    crossing,
                    dribbling,
                    finishing,
                    first_touch,
                    free_kick_taking,
                    heading,
                    long_shots,
                    long_throws,
                    marking,
                    passing,
                    penalty_taking,
                    tackling,
                    technique,
                    // Mental (14)
                    aggression,
                    anticipation,
                    bravery,
                    composure,
                    concentration,
                    decisions,
                    determination,
                    flair,
                    leadership,
                    off_the_ball,
                    positioning,
                    teamwork,
                    vision,
                    work_rate,
                    // Physical (8)
                    acceleration,
                    agility,
                    balance,
                    jumping,
                    natural_fitness,
                    pace,
                    stamina,
                    strength,
                    // Hidden/Physical (5) - v5
                    stability,
                    foul,
                    contest_performance,
                    injury_proneness,
                    versatility,
                    // Goalkeeper (11) - v5
                    aerial_reach,
                    command_of_area,
                    communication,
                    eccentricity,
                    handling,
                    gk_kicking,
                    one_on_ones,
                    reflexes,
                    rushing_out,
                    punching,
                    throwing,
                    // Personality (8) - v5
                    adaptation,
                    ambition,
                    controversy,
                    loyalty,
                    pressure,
                    professionalism,
                    sportsmanship,
                    temperament,
                    // Physical Info (4) - v5
                    height_cm,
                    weight_kg,
                    left_foot,
                    right_foot,
                    // Career/Financial (6) - v5
                    market_value,
                    reputation_current,
                    reputation_domestic,
                    reputation_world,
                    salary,
                    loan_club,
                    // Personal Info (6) - v5
                    ethnicity,
                    rca,
                    skin_color,
                    birth_date,
                    national_team_caps,
                    national_team_goals,
                    // Position Ratings
                    position_ratings: Some(position_ratings_str),
                };

                index.insert(person);
                stats.parsed += 1;
            }
            Err(e) => {
                stats.failed += 1;
                eprintln!(
                    "Warning: Line {} - CSV parse error: {}",
                    stats.total_rows, e
                );
            }
        }
    }

    if stats.parsed == 0 {
        anyhow::bail!("No valid persons parsed from CSV");
    }

    Ok((index, stats))
}

/// Build binary cache from FM 2023 CSV file
///
/// Pipeline: CSV → PersonIndex → MessagePack → LZ4 → Binary file
///
/// # Arguments
///
/// * `csv_path` - Input CSV file (docs/FM 2023.csv/FM 2023.csv)
/// * `output_msgpack_lz4` - Output binary file (data/exports/cache_players.v5.msgpack.lz4)
/// * `schema_version` - Schema version (e.g., "v5")
///
/// # Returns
///
/// Cache metadata including checksum, sizes, compression ratio
pub fn build_person_cache(
    csv_path: &Path,
    output_msgpack_lz4: &Path,
    schema_version: &str,
) -> Result<crate::CacheMetadata> {
    // 1. Parse CSV → PersonIndex
    println!("Parsing CSV: {}", csv_path.display());
    let (index, stats) = parse_csv_to_index(csv_path, schema_version, true)?;

    println!(
        "✅ Parsed {} players (failed: {}, total rows: {})",
        stats.parsed, stats.failed, stats.total_rows
    );

    // 2. Serialize to MessagePack
    println!("Serializing to MessagePack...");
    let msgpack_bytes =
        rmp_serde::to_vec(&index).context("Failed to serialize PersonIndex to MessagePack")?;

    let original_size = msgpack_bytes.len() as u64;

    // 3. LZ4 compression
    println!("Compressing with LZ4...");
    let compressed = lz4_flex::compress_prepend_size(&msgpack_bytes);
    let compressed_size = compressed.len() as u64;

    // 4. Calculate checksum
    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let checksum = format!("{:x}", hasher.finalize());

    // 5. Write to file
    if let Some(parent) = output_msgpack_lz4.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    fs::write(output_msgpack_lz4, &compressed).with_context(|| {
        format!(
            "Failed to write output file: {}",
            output_msgpack_lz4.display()
        )
    })?;

    let compression_ratio = compressed_size as f64 / original_size as f64;

    println!(
        "✅ Cache built: {} → {} (ratio: {:.2}%)",
        human_bytes(original_size),
        human_bytes(compressed_size),
        compression_ratio * 100.0
    );

    Ok(crate::CacheMetadata {
        schema_version: schema_version.to_string(),
        checksum,
        created_at: chrono::Utc::now().to_rfc3339(),
        original_size,
        compressed_size,
        compression_ratio,
    })
}

/// Load binary cache into PersonIndex
///
/// Pipeline: Binary file → LZ4 decompress → MessagePack deserialize → PersonIndex
///
/// # Arguments
///
/// * `cache_file` - Binary cache file path
///
/// # Returns
///
/// Deserialized PersonIndex ready for use
pub fn load_person_cache(cache_file: &Path) -> Result<PersonIndex> {
    // 1. Read file
    let compressed = fs::read(cache_file)
        .with_context(|| format!("Failed to read cache file: {}", cache_file.display()))?;

    // 2. LZ4 decompress
    let msgpack_bytes =
        lz4_flex::decompress_size_prepended(&compressed).context("Failed to decompress LZ4")?;

    // 3. MessagePack deserialize
    let index: PersonIndex = rmp_serde::from_slice(&msgpack_bytes)
        .context("Failed to deserialize PersonIndex from MessagePack")?;

    Ok(index)
}

// ============================================================================
// Embedded Data Support (2025-12-05)
// ============================================================================

/// Embedded player cache binary (400KB, 8053 players)
///
/// Compiled into the binary at build time using `include_bytes!`.
/// This eliminates the need for external file loading at runtime.
///
/// To update: rebuild the cache_players.v3.msgpack.lz4 file and recompile.
#[cfg(feature = "embedded_players")]
pub const EMBEDDED_PLAYER_CACHE: &[u8] =
    include_bytes!("../../../data/exports/cache_players.v3.msgpack.lz4");

/// Load embedded player cache (no file I/O)
///
/// Pipeline: Embedded bytes → LZ4 decompress → MessagePack deserialize → PersonIndex
///
/// # Returns
///
/// Deserialized PersonIndex from embedded data
///
/// # Errors
///
/// Returns error if decompression or deserialization fails (should never happen
/// if the embedded data was built correctly)
#[cfg(feature = "embedded_players")]
pub fn load_person_cache_embedded() -> Result<PersonIndex> {
    // 1. LZ4 decompress from embedded bytes
    let msgpack_bytes = lz4_flex::decompress_size_prepended(EMBEDDED_PLAYER_CACHE)
        .context("Failed to decompress embedded LZ4 player cache")?;

    // 2. MessagePack deserialize
    let index: PersonIndex = rmp_serde::from_slice(&msgpack_bytes)
        .context("Failed to deserialize embedded PersonIndex from MessagePack")?;

    Ok(index)
}

/// Check if embedded player cache is available
#[cfg(feature = "embedded_players")]
pub fn has_embedded_player_cache() -> bool {
    true
}

#[cfg(not(feature = "embedded_players"))]
pub fn has_embedded_player_cache() -> bool {
    false
}

/// Human-readable byte size formatting
fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_index_operations() {
        let mut index = PersonIndex::new("v5".to_string());

        assert!(index.is_empty());
        assert_eq!(index.len(), 0);

        let person = Person::new(
            1,
            "Test".to_string(),
            "ENG".to_string(),
            "FC Test".to_string(),
            "ST".to_string(),
            150,
            160,
            25,
            None,
        );

        index.insert(person);

        assert!(!index.is_empty());
        assert_eq!(index.len(), 1);
        assert!(index.get(1).is_some());
        assert!(index.get(999).is_none());
    }

    #[test]
    fn test_human_bytes() {
        assert_eq!(human_bytes(0), "0.00 B");
        assert_eq!(human_bytes(1024), "1.00 KB");
        assert_eq!(human_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(human_bytes(1536), "1.50 KB"); // 1.5 KB
    }

    /// Integration test: Load v5 cache and verify new fields
    /// Run only when the v5 cache exists
    #[test]
    #[ignore = "requires v5 cache file to exist"]
    fn test_load_v5_cache_kevin_de_bruyne() {
        use std::path::Path;

        // Try multiple possible paths (relative to crate or project root)
        let possible_paths = [
            "../../data/exports/cache_players.v5.msgpack.lz4",
            "data/exports/cache_players.v5.msgpack.lz4",
        ];

        let cache_path = possible_paths
            .iter()
            .map(Path::new)
            .find(|p| p.exists())
            .expect("Cache file not found in any expected location");

        let index = load_person_cache(cache_path).expect("Failed to load cache");

        assert_eq!(index.schema_version, "v5");
        assert_eq!(index.len(), 8452);

        // Kevin De Bruyne is UID 1 (first row after header)
        let kdb = index.get(1).expect("Kevin De Bruyne not found");

        // Basic info
        assert_eq!(kdb.name, "Kevin De Bruyne");
        assert_eq!(kdb.position, "M/AM RLC");
        assert_eq!(kdb.age, 31);
        assert_eq!(kdb.ca, 189);
        assert_eq!(kdb.pa, 189);
        assert_eq!(kdb.nationality, "Belgium");
        assert_eq!(kdb.team, "Manchester City");

        // v5: Physical Info
        assert_eq!(kdb.height_cm, 181);
        assert_eq!(kdb.weight_kg, 68);
        assert_eq!(kdb.left_foot, 16);
        assert_eq!(kdb.right_foot, 20);

        // v5: Hidden/Physical attributes
        assert_eq!(kdb.stability, 15);
        assert_eq!(kdb.foul, 6);
        assert_eq!(kdb.contest_performance, 15);
        assert_eq!(kdb.injury_proneness, 7);
        assert_eq!(kdb.versatility, 14);

        // v5: Goalkeeper attributes (should be low for midfielder)
        assert_eq!(kdb.aerial_reach, 1);
        assert_eq!(kdb.command_of_area, 3);
        assert_eq!(kdb.handling, 3);
        assert_eq!(kdb.reflexes, 3);

        // v5: Personality
        assert_eq!(kdb.adaptation, 13);
        assert_eq!(kdb.ambition, 17);
        assert_eq!(kdb.controversy, 13);
        assert_eq!(kdb.loyalty, 14);
        assert_eq!(kdb.pressure, 11);
        assert_eq!(kdb.professionalism, 18);
        assert_eq!(kdb.sportsmanship, 16);
        assert_eq!(kdb.temperament, 13);

        // v5: Career/Financial
        assert_eq!(kdb.market_value, 347975206);
        assert_eq!(kdb.reputation_current, 9450);
        assert_eq!(kdb.reputation_domestic, 9400);
        assert_eq!(kdb.reputation_world, 9400);

        // v5: Personal Info
        assert_eq!(kdb.birth_date, "1991/6/28");
        assert_eq!(kdb.national_team_caps, 91);
        assert_eq!(kdb.national_team_goals, 24);

        println!("✅ All v5 fields verified for Kevin De Bruyne!");
    }
}
