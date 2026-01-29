use super::error::SaveError;
use super::SAVE_VERSION;
use crate::coach::{CardInventory, Deck};
use crate::player::types::CorePlayer;
use crate::quest::QuestManagerState;
use crate::training::session::TrainingManager;
use serde::{Deserialize, Serialize};

use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use rmp_serde::{from_slice, to_vec_named};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

/// Main game save structure with all persistent data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameSave {
    /// Save format version for migration
    pub version: u32,

    /// Save timestamp (unix milliseconds)
    pub timestamp: u64,

    /// Player roster - all recruited players
    pub players: Vec<CorePlayer>,

    /// Coach card collection and inventory
    pub card_inventory: CardInventory,

    /// Saved deck configurations
    pub saved_decks: Vec<Deck>,

    /// Currently active deck ID for training
    pub active_deck_id: Option<String>,

    /// Training system state
    pub training_manager: TrainingManager,

    /// Match history and results
    pub match_history: Vec<MatchRecord>,

    /// Game progress and achievements
    pub progress: GameProgress,

    /// Player preferences and settings (separate from system settings)
    pub game_settings: GameSettings,

    /// Quest system state (M2: Youth Academy Mode)
    pub quest_manager: QuestManagerState,

    /// Avatar appearance configuration (kit colors, pattern, etc.)
    #[serde(default)]
    pub player_appearance: Option<PlayerAppearance>,
}

impl Default for GameSave {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSave {
    pub fn new() -> Self {
        Self {
            version: SAVE_VERSION,
            timestamp: current_timestamp(),
            players: Vec::new(),
            card_inventory: CardInventory::new(),
            saved_decks: Vec::new(),
            active_deck_id: None,
            training_manager: TrainingManager::new(),
            match_history: Vec::new(),
            progress: GameProgress::default(),
            game_settings: GameSettings::default(),
            quest_manager: QuestManagerState::default(),
            player_appearance: None,
        }
    }

    pub fn update_timestamp(&mut self) {
        self.timestamp = current_timestamp();
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        // Basic validation checks
        if self.players.len() > 1000 {
            return Err(SaveError::DataTooLarge { size: self.players.len() });
        }

        if self.card_inventory.total_card_count > 500 {
            return Err(SaveError::DataTooLarge { size: self.card_inventory.total_card_count });
        }

        // Check for duplicate player IDs
        let mut player_ids = std::collections::HashSet::new();
        for player in &self.players {
            if !player_ids.insert(&player.id) {
                return Err(SaveError::Corrupted);
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MatchRecord {
    pub id: u32,
    pub opponent: String,
    pub result: MatchResult,
    pub score_home: u8,
    pub score_away: u8,
    pub date: u64, // timestamp
    pub week: u16,
    pub season: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum MatchResult {
    Win,
    #[default]
    Draw,
    Loss,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameProgress {
    /// Current game week
    pub current_week: u16,

    /// Current season
    pub current_season: u16,

    /// Total matches played
    pub total_matches: u32,

    /// Total cards collected
    pub total_cards_collected: u32,

    /// Achievements unlocked
    pub achievements: Vec<String>,

    /// Statistics
    pub stats: PlayerStats,
}

impl Default for GameProgress {
    fn default() -> Self {
        Self {
            current_week: 1,
            current_season: 1,
            total_matches: 0,
            total_cards_collected: 0,
            achievements: Vec::new(),
            stats: PlayerStats::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerStats {
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub training_sessions: u32,
    pub cards_drawn: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameSettings {
    /// Auto-save enabled
    pub auto_save: bool,

    /// Auto-save interval (minutes)
    pub auto_save_interval: u32,

    /// Training difficulty
    pub training_difficulty: TrainingDifficulty,

    /// Match simulation speed
    pub match_speed: MatchSpeed,

    /// Show detailed statistics
    pub show_detailed_stats: bool,

    /// Language preference (separate from system locale)
    pub preferred_language: String,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_save: true,
            auto_save_interval: 5,
            training_difficulty: TrainingDifficulty::Normal,
            match_speed: MatchSpeed::Normal,
            show_detailed_stats: true,
            preferred_language: "korean".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TrainingDifficulty {
    Easy,
    Normal,
    Hard,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MatchSpeed {
    Slow,
    Normal,
    Fast,
    Instant,
}

/// Player avatar appearance for kit and character visualization
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerAppearance {
    /// Hair color type: "black", "blonde", "redhead", "other"
    pub hair_color: String,
    /// Skin tone index (0-4)
    pub skin_tone: u8,
    /// Kit primary color RGB
    pub kit_primary: [u8; 3],
    /// Kit secondary color RGB
    pub kit_secondary: [u8; 3],
    /// Kit pattern type: 0=Solid, 1=Hoops, 2=Stripes, 3=Checker, 4=Diagonal
    pub kit_pattern: u8,
}

/// Serialize and compress game save data
pub fn serialize_and_compress(save: &GameSave) -> Result<Vec<u8>, SaveError> {
    // Validate before serialization
    save.validate()?;

    // 1. Serialize to MessagePack with field names
    let msgpack = to_vec_named(save).map_err(SaveError::Serialization)?;

    // 2. Compress with LZ4 (size prepended for easy decompression)
    let compressed = compress_prepend_size(&msgpack);

    // 3. Add SHA256 checksum at the end
    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let checksum = hasher.finalize();

    let mut result = compressed;
    result.extend_from_slice(&checksum);

    Ok(result)
}

/// Decompress and deserialize game save data
pub fn decompress_and_deserialize(bytes: &[u8]) -> Result<GameSave, SaveError> {
    // Check minimum size (header + checksum)
    if bytes.len() < 4 + 32 {
        return Err(SaveError::Corrupted);
    }

    // Split payload and checksum
    let (payload, checksum_bytes) = bytes.split_at(bytes.len() - 32);

    // Verify checksum
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let calculated_checksum = hasher.finalize();

    if &calculated_checksum[..] != checksum_bytes {
        return Err(SaveError::ChecksumMismatch);
    }

    // Decompress
    let msgpack = decompress_size_prepended(payload).map_err(|_| SaveError::Decompression)?;

    // Deserialize
    let save: GameSave = from_slice(&msgpack).map_err(SaveError::Deserialization)?;

    // Validate version
    if save.version > SAVE_VERSION {
        return Err(SaveError::VersionMismatch { found: save.version, expected: SAVE_VERSION });
    }

    Ok(save)
}

pub fn current_timestamp() -> u64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let save = GameSave::new();

        let serialized = serialize_and_compress(&save).unwrap();
        let deserialized = decompress_and_deserialize(&serialized).unwrap();

        assert_eq!(save.version, deserialized.version);
        assert_eq!(save.players.len(), deserialized.players.len());
    }

    #[test]
    fn test_checksum_validation() {
        let save = GameSave::new();
        let mut serialized = serialize_and_compress(&save).unwrap();

        // Corrupt the checksum
        if let Some(last) = serialized.last_mut() {
            *last = last.wrapping_add(1);
        }

        let result = decompress_and_deserialize(&serialized);
        assert!(matches!(result, Err(SaveError::ChecksumMismatch)));
    }

    #[test]
    fn test_compression_ratio() {
        let mut save = GameSave::new();

        // Add some test data
        for i in 0..100 {
            save.players.push(CorePlayer::generate_player(
                format!("Player {}", i),
                crate::models::player::Position::FW,
                (60, 80),
                (120, 150),
                16.0,
                i as u64,
            ));
        }

        let uncompressed = to_vec_named(&save).unwrap();
        let compressed = serialize_and_compress(&save).unwrap();

        let ratio = compressed.len() as f32 / uncompressed.len() as f32;
        println!(
            "Compression ratio: {:.2}% ({} -> {} bytes)",
            ratio * 100.0,
            uncompressed.len(),
            compressed.len()
        );

        // Should achieve reasonable compression
        assert!(ratio < 0.8); // Less than 80% of original size
    }
}
