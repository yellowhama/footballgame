use serde::{Deserialize, Serialize};

/// Position Rating enum for FM 2023's 14-position system
///
/// Maps to position_ratings string indices (0-13)
/// Format: GK, DL, DC, DR, WBL, WBR, DM, ML, MC, MR, AML, AMC, AMR, ST
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PositionRating {
    GK = 0,   // Goalkeeper
    DL = 1,   // Defender Left (Left Back)
    DC = 2,   // Defender Center (Center Back)
    DR = 3,   // Defender Right (Right Back)
    WBL = 4,  // Wing Back Left
    WBR = 5,  // Wing Back Right
    DM = 6,   // Defensive Midfielder
    ML = 7,   // Midfielder Left
    MC = 8,   // Midfielder Center
    MR = 9,   // Midfielder Right
    AML = 10, // Attacking Midfielder Left
    AMC = 11, // Attacking Midfielder Center
    AMR = 12, // Attacking Midfielder Right
    ST = 13,  // Striker
}

impl PositionRating {
    /// Get all position ratings in order
    pub fn all() -> &'static [PositionRating] {
        &[
            PositionRating::GK,
            PositionRating::DL,
            PositionRating::DC,
            PositionRating::DR,
            PositionRating::WBL,
            PositionRating::WBR,
            PositionRating::DM,
            PositionRating::ML,
            PositionRating::MC,
            PositionRating::MR,
            PositionRating::AML,
            PositionRating::AMC,
            PositionRating::AMR,
            PositionRating::ST,
        ]
    }

    /// Get position name as string
    pub fn name(&self) -> &'static str {
        match self {
            PositionRating::GK => "GK",
            PositionRating::DL => "DL",
            PositionRating::DC => "DC",
            PositionRating::DR => "DR",
            PositionRating::WBL => "WBL",
            PositionRating::WBR => "WBR",
            PositionRating::DM => "DM",
            PositionRating::ML => "ML",
            PositionRating::MC => "MC",
            PositionRating::MR => "MR",
            PositionRating::AML => "AML",
            PositionRating::AMC => "AMC",
            PositionRating::AMR => "AMR",
            PositionRating::ST => "ST",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GK" => Some(PositionRating::GK),
            "DL" => Some(PositionRating::DL),
            "DC" => Some(PositionRating::DC),
            "DR" => Some(PositionRating::DR),
            "WBL" => Some(PositionRating::WBL),
            "WBR" => Some(PositionRating::WBR),
            "DM" => Some(PositionRating::DM),
            "ML" => Some(PositionRating::ML),
            "MC" => Some(PositionRating::MC),
            "MR" => Some(PositionRating::MR),
            "AML" => Some(PositionRating::AML),
            "AMC" => Some(PositionRating::AMC),
            "AMR" => Some(PositionRating::AMR),
            "ST" => Some(PositionRating::ST),
            _ => None,
        }
    }

    /// Map match engine Position to PositionRating
    ///
    /// Match engine uses different position naming:
    /// - LB/RB/CB → DL/DR/DC (Defender Left/Right/Center)
    /// - LWB/RWB → WBL/WBR (Wing Back Left/Right)
    /// - CDM → DM (Defensive Midfielder)
    /// - CM → MC (Midfielder Center)
    /// - CAM → AMC (Attacking Midfielder Center)
    /// - LM/RM → ML/MR (Midfielder Left/Right)
    /// - LW/RW → AML/AMR (Attacking Midfielder Left/Right)
    /// - CF/ST → ST (Striker)
    pub fn from_engine_position(pos: &crate::models::player::Position) -> Self {
        use crate::models::player::Position;

        match pos {
            Position::GK => PositionRating::GK,
            Position::LB => PositionRating::DL,
            Position::CB => PositionRating::DC,
            Position::RB => PositionRating::DR,
            Position::LWB => PositionRating::WBL,
            Position::RWB => PositionRating::WBR,
            Position::CDM => PositionRating::DM,
            Position::LM => PositionRating::ML,
            Position::CM => PositionRating::MC,
            Position::RM => PositionRating::MR,
            Position::LW => PositionRating::AML,
            Position::CAM => PositionRating::AMC,
            Position::RW => PositionRating::AMR,
            Position::CF => PositionRating::ST,
            Position::ST => PositionRating::ST,
            // Generic positions → closest central position
            Position::DF => PositionRating::DC,
            Position::MF => PositionRating::MC,
            Position::FW => PositionRating::ST,
        }
    }
}

/// Person struct for player database (optimized for caching and lookup)
///
/// This is separate from the `Player` struct used in match simulation.
/// Source: docs/FM 2023.csv/FM 2023.csv (8,452 players from FM 2023)
///
/// Schema v5 Fields (98 total from CSV):
/// - Basic (8): UID, Name, Nationality, Team, Position, CA, PA, Age
/// - FM Attributes (36): Technical×14, Mental×14, Physical×8
/// - Hidden/Physical (5): Stability, Foul, Contest, Injury, Versatility
/// - Goalkeeper (11): Aerial Reach, Command, Communication, Eccentricity,
///                    Handling, Kicking, 1v1, Reflexes, Rushing, Punching, Throwing
/// - Personality (8): Adaptation, Ambition, Argue, Loyal, Pressure,
///                    Professional, Sportsmanship, Temperament
/// - Physical Info (4): Height, Weight, Left Foot, Right Foot
/// - Career/Financial (6): Value, Reputation×3, Salary, Loan Club
/// - Personal Info (6): Ethnicity, RCA, Skin, Birth Date, Caps, Goals
/// - Position Ratings (14): GK through ST as comma-separated string
///
/// Usage:
/// - CSV → RuntimeIndex (FxHashMap<u32, Person>) → MessagePack + LZ4 → Binary cache
/// - Godot loads via GDExtension for player roster, scouting, transfers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Person {
    // ========== Basic Info (7 fields) ==========
    /// Unique identifier (auto-generated from CSV row index, 1-based)
    pub uid: u32,

    /// Real player name (e.g., "Lionel Messi")
    pub name: String,

    /// Nationality (3-letter country code, e.g., "ARG", "POR", "FRA")
    pub nationality: String,

    /// Current team (e.g., "Barcelona", "Juventus")
    pub team: String,

    /// Position (can be multiple, e.g., "AM (RC), ST (C)")
    /// Note: This is a string because CSV has compound positions like "AM (RL), ST (C)"
    /// For match simulation, parse this into Position enum
    pub position: String,

    /// Current Ability (0-200, representing current skill level)
    pub ca: u8,

    /// Potential Ability (0-200, representing maximum achievable skill)
    pub pa: u8,

    /// Age (in years)
    pub age: u8,

    // ========== FM2023 Attributes (36 fields, 1-20 scale) ==========
    // Technical (14)
    pub corners: u8,
    pub crossing: u8,
    pub dribbling: u8,
    pub finishing: u8,
    pub first_touch: u8,
    pub free_kick_taking: u8,
    pub heading: u8,
    pub long_shots: u8,
    pub long_throws: u8,
    pub marking: u8,
    pub passing: u8,
    pub penalty_taking: u8,
    pub tackling: u8,
    pub technique: u8,

    // Mental (14)
    pub aggression: u8,
    pub anticipation: u8,
    pub bravery: u8,
    pub composure: u8,
    pub concentration: u8,
    pub decisions: u8,
    pub determination: u8,
    pub flair: u8,
    pub leadership: u8,
    pub off_the_ball: u8,
    pub positioning: u8,
    pub teamwork: u8,
    pub vision: u8,
    pub work_rate: u8,

    // Physical (8)
    pub acceleration: u8,
    pub agility: u8,
    pub balance: u8,
    pub jumping: u8,
    pub natural_fitness: u8,
    pub pace: u8,
    pub stamina: u8,
    pub strength: u8,

    // ========== Hidden/Physical Attributes (5 fields, v5) ==========
    /// Stability (1-20) - 안정성
    pub stability: u8,
    /// Foul proneness (1-20) - 파울 성향
    pub foul: u8,
    /// Contest performance (1-20) - 경쟁 성과
    pub contest_performance: u8,
    /// Injury proneness (1-20) - 부상 성향
    pub injury_proneness: u8,
    /// Versatility (1-20) - 다재다능
    pub versatility: u8,

    // ========== Goalkeeper Attributes (11 fields, v5) ==========
    /// Aerial Reach (1-20) - GK 공중볼 범위
    pub aerial_reach: u8,
    /// Command Of Area (1-20) - GK 지역 장악
    pub command_of_area: u8,
    /// Communication (1-20) - GK 의사소통
    pub communication: u8,
    /// Eccentricity (1-20) - GK 기행
    pub eccentricity: u8,
    /// Handling (1-20) - GK 핸들링
    pub handling: u8,
    /// Kicking (1-20) - GK 킥
    pub gk_kicking: u8,
    /// One On Ones (1-20) - GK 1대1
    pub one_on_ones: u8,
    /// Reflexes (1-20) - GK 반사신경
    pub reflexes: u8,
    /// Rushing Out (1-20) - GK 전진 수비
    pub rushing_out: u8,
    /// Punching (1-20) - GK 펀칭
    pub punching: u8,
    /// Throwing (1-20) - GK 스로잉
    pub throwing: u8,

    // ========== Personality Attributes (8 fields, v5) ==========
    /// Adaptation (1-20) - 적응력
    pub adaptation: u8,
    /// Ambition (1-20) - 야망
    pub ambition: u8,
    /// Controversy/Argue (1-20) - 논쟁 성향
    pub controversy: u8,
    /// Loyalty (1-20) - 충성도
    pub loyalty: u8,
    /// Pressure handling (1-20) - 압박 내성
    pub pressure: u8,
    /// Professionalism (1-20) - 프로의식
    pub professionalism: u8,
    /// Sportsmanship (1-20) - 스포츠맨십
    pub sportsmanship: u8,
    /// Temperament (1-20) - 감정 조절
    pub temperament: u8,

    // ========== Physical Info (4 fields, v5) ==========
    /// Height in cm (e.g., 181)
    pub height_cm: u16,
    /// Weight in kg (e.g., 68)
    pub weight_kg: u8,
    /// Left foot ability (1-20)
    pub left_foot: u8,
    /// Right foot ability (1-20)
    pub right_foot: u8,

    // ========== Career/Financial (6 fields, v5) ==========
    /// Market value in currency units
    pub market_value: u32,
    /// Current reputation (1-10000)
    pub reputation_current: u16,
    /// Domestic reputation (1-10000)
    pub reputation_domestic: u16,
    /// World reputation (1-10000)
    pub reputation_world: u16,
    /// Weekly salary in currency units
    pub salary: u32,
    /// Loan club (empty if not on loan)
    pub loan_club: String,

    // ========== Personal Info (6 fields, v5) ==========
    /// Ethnicity/Race category
    pub ethnicity: String,
    /// RCA value
    pub rca: u16,
    /// Skin color value
    pub skin_color: u8,
    /// Birth date (YYYY/MM/DD format)
    pub birth_date: String,
    /// Number of national team appearances
    pub national_team_caps: u16,
    /// Goals scored for national team
    pub national_team_goals: u16,

    // ========== Position Ratings (1 field) ==========
    /// Position ratings as compact string (14 ratings, 0-20 each)
    /// Format: "GK,DL,DC,DR,WBL,WBR,DM,ML,MC,MR,AML,AMC,AMR,ST"
    /// Example: "1,5,7,6,3,4,10,12,15,13,18,20,17,19"
    /// None for players without position ratings (backward compatibility)
    pub position_ratings: Option<String>,
}

impl Person {
    /// Create a new Person (mainly for testing)
    ///
    /// For actual data loading, use cache_builder with direct struct construction.
    /// All FM attributes default to 10 (1-20 scale).
    ///
    /// # Arguments
    ///
    /// * `uid` - Unique identifier (CSV row index, 1-based)
    /// * `name` - Player name
    /// * `nationality` - Country code
    /// * `team` - Current team
    /// * `position` - Position string (may contain multiple positions)
    /// * `ca` - Current Ability (0-200)
    /// * `pa` - Potential Ability (0-200)
    /// * `age` - Age in years
    /// * `position_ratings` - Position ratings string (14 comma-separated values)
    pub fn new(
        uid: u32,
        name: String,
        nationality: String,
        team: String,
        position: String,
        ca: u8,
        pa: u8,
        age: u8,
        position_ratings: Option<String>,
    ) -> Self {
        Self {
            uid,
            name,
            nationality,
            team,
            position,
            ca,
            pa,
            age,
            // Default all FM attributes to 10 (average on 1-20 scale)
            corners: 10,
            crossing: 10,
            dribbling: 10,
            finishing: 10,
            first_touch: 10,
            free_kick_taking: 10,
            heading: 10,
            long_shots: 10,
            long_throws: 10,
            marking: 10,
            passing: 10,
            penalty_taking: 10,
            tackling: 10,
            technique: 10,
            aggression: 10,
            anticipation: 10,
            bravery: 10,
            composure: 10,
            concentration: 10,
            decisions: 10,
            determination: 10,
            flair: 10,
            leadership: 10,
            off_the_ball: 10,
            positioning: 10,
            teamwork: 10,
            vision: 10,
            work_rate: 10,
            acceleration: 10,
            agility: 10,
            balance: 10,
            jumping: 10,
            natural_fitness: 10,
            pace: 10,
            stamina: 10,
            strength: 10,
            // Hidden/Physical (v5)
            stability: 10,
            foul: 10,
            contest_performance: 10,
            injury_proneness: 10,
            versatility: 10,
            // Goalkeeper (v5) - default to 1 for non-GK
            aerial_reach: 1,
            command_of_area: 1,
            communication: 1,
            eccentricity: 10,
            handling: 1,
            gk_kicking: 1,
            one_on_ones: 1,
            reflexes: 1,
            rushing_out: 1,
            punching: 1,
            throwing: 1,
            // Personality (v5)
            adaptation: 10,
            ambition: 10,
            controversy: 10,
            loyalty: 10,
            pressure: 10,
            professionalism: 10,
            sportsmanship: 10,
            temperament: 10,
            // Physical Info (v5)
            height_cm: 175,
            weight_kg: 70,
            left_foot: 10,
            right_foot: 10,
            // Career/Financial (v5)
            market_value: 0,
            reputation_current: 1000,
            reputation_domestic: 1000,
            reputation_world: 1000,
            salary: 0,
            loan_club: String::new(),
            // Personal Info (v5)
            ethnicity: String::new(),
            rca: 0,
            skin_color: 0,
            birth_date: String::new(),
            national_team_caps: 0,
            national_team_goals: 0,
            position_ratings,
        }
    }

    /// Get display name (returns real name from FM data)
    pub fn display_name(&self) -> &str {
        &self.name
    }

    /// Get display team (returns real team from FM data)
    pub fn display_team(&self) -> &str {
        &self.team
    }

    /// Check if player is young talent (age <= 21 and PA significantly higher than CA)
    pub fn is_wonderkid(&self) -> bool {
        self.age <= 21 && self.pa >= self.ca + 20
    }

    /// Check if player is a veteran (age >= 32)
    pub fn is_veteran(&self) -> bool {
        self.age >= 32
    }

    /// Check if player is in prime age (23-29)
    pub fn is_prime_age(&self) -> bool {
        self.age >= 23 && self.age <= 29
    }

    /// Get growth potential (difference between PA and CA)
    pub fn growth_potential(&self) -> i16 {
        self.pa as i16 - self.ca as i16
    }

    /// Check if player is world-class (CA >= 170)
    pub fn is_world_class(&self) -> bool {
        self.ca >= 170
    }

    /// Check if player is top talent (CA >= 150)
    pub fn is_top_talent(&self) -> bool {
        self.ca >= 150
    }

    /// Parse primary position from compound position string
    ///
    /// Example: "AM (RC), ST (C)" → "AM"
    pub fn primary_position(&self) -> &str {
        self.position
            .split(',')
            .next()
            .unwrap_or(&self.position)
            .split_whitespace()
            .next()
            .unwrap_or(&self.position)
    }

    /// Get all positions this player can play
    ///
    /// Example: "AM (RC), ST (C)" → vec!["AM", "ST"]
    pub fn all_positions(&self) -> Vec<&str> {
        self.position
            .split(',')
            .map(|s| s.split_whitespace().next().unwrap_or(""))
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Parse position ratings string into array of 14 values
    ///
    /// Format: "1,5,7,6,3,4,10,12,15,13,18,20,17,19"
    /// Order: GK, DL, DC, DR, WBL, WBR, DM, ML, MC, MR, AML, AMC, AMR, ST
    ///
    /// Returns None if position_ratings is None or parsing fails
    pub fn get_position_ratings(&self) -> Option<[u8; 14]> {
        let ratings_str = self.position_ratings.as_ref()?;

        let values: Vec<u8> =
            ratings_str.split(',').filter_map(|s| s.trim().parse::<u8>().ok()).collect();

        if values.len() != 14 {
            return None;
        }

        let mut array = [0u8; 14];
        array.copy_from_slice(&values);
        Some(array)
    }

    /// Get position rating for a specific position
    ///
    /// Returns None if position_ratings is None or index out of bounds
    pub fn get_position_rating(&self, position: PositionRating) -> Option<u8> {
        let ratings = self.get_position_ratings()?;
        Some(ratings[position as usize])
    }

    /// Calculate position suitability (0.0-1.0) for a given position
    ///
    /// Suitability determines performance penalty:
    /// - Rating 0:     0.0 (100% penalty, cannot play)
    /// - Rating 1-5:   0.3 (70% penalty, very poor)
    /// - Rating 6-10:  0.6 (40% penalty, adequate)
    /// - Rating 11-14: 0.85 (15% penalty, good)
    /// - Rating 15-20: 1.0 (no penalty, natural position)
    ///
    /// Returns 0.5 (50% penalty) if position_ratings is None (backward compatibility)
    pub fn position_suitability(&self, position: PositionRating) -> f32 {
        match self.get_position_rating(position) {
            Some(rating) => match rating {
                0 => 0.0,
                1..=5 => 0.3,
                6..=10 => 0.6,
                11..=14 => 0.85,
                15..=20 => 1.0,
                _ => 0.5, // Fallback for invalid ratings
            },
            None => 0.5, // Fallback when position_ratings is None
        }
    }

    /// Get best positions for this player (sorted by rating, descending)
    ///
    /// Returns vector of (PositionRating, rating) tuples with rating >= min_rating
    pub fn best_positions(&self, min_rating: u8) -> Vec<(PositionRating, u8)> {
        let Some(ratings) = self.get_position_ratings() else {
            return Vec::new();
        };

        let mut positions: Vec<(PositionRating, u8)> = PositionRating::all()
            .iter()
            .map(|&pos| (pos, ratings[pos as usize]))
            .filter(|&(_, rating)| rating >= min_rating)
            .collect();

        positions.sort_by(|a, b| b.1.cmp(&a.1)); // Sort descending by rating
        positions
    }

    /// Convert FM attributes (1-20 scale) to PlayerAttributes (0-100 scale)
    ///
    /// Conversion: attr_100 = attr_20 * 5
    /// - FM 1 → Engine 5
    /// - FM 10 → Engine 50
    /// - FM 20 → Engine 100
    ///
    /// Returns PlayerAttributes for use in match simulation.
    pub fn to_player_attributes(&self) -> crate::models::player::PlayerAttributes {
        // FM 1-20 → Engine 0-100: multiply by 5
        let scale = |v: u8| (v.saturating_mul(5)).clamp(1, 100);

        crate::models::player::PlayerAttributes {
            // Technical (14)
            corners: scale(self.corners),
            crossing: scale(self.crossing),
            dribbling: scale(self.dribbling),
            finishing: scale(self.finishing),
            first_touch: scale(self.first_touch),
            free_kicks: scale(self.free_kick_taking),
            heading: scale(self.heading),
            long_shots: scale(self.long_shots),
            long_throws: scale(self.long_throws),
            marking: scale(self.marking),
            passing: scale(self.passing),
            penalty_taking: scale(self.penalty_taking),
            tackling: scale(self.tackling),
            technique: scale(self.technique),

            // Mental (14)
            aggression: scale(self.aggression),
            anticipation: scale(self.anticipation),
            bravery: scale(self.bravery),
            composure: scale(self.composure),
            concentration: scale(self.concentration),
            decisions: scale(self.decisions),
            determination: scale(self.determination),
            flair: scale(self.flair),
            leadership: scale(self.leadership),
            off_the_ball: scale(self.off_the_ball),
            positioning: scale(self.positioning),
            teamwork: scale(self.teamwork),
            vision: scale(self.vision),
            work_rate: scale(self.work_rate),

            // Physical (8)
            acceleration: scale(self.acceleration),
            agility: scale(self.agility),
            balance: scale(self.balance),
            jumping: scale(self.jumping),
            natural_fitness: scale(self.natural_fitness),
            pace: scale(self.pace),
            stamina: scale(self.stamina),
            strength: scale(self.strength),

            // Goalkeeper (11) - v5 schema
            gk_aerial_reach: scale(self.aerial_reach),
            gk_command_of_area: scale(self.command_of_area),
            gk_communication: scale(self.communication),
            gk_eccentricity: scale(self.eccentricity),
            gk_handling: scale(self.handling),
            gk_kicking: scale(self.gk_kicking),
            gk_one_on_ones: scale(self.one_on_ones),
            gk_reflexes: scale(self.reflexes),
            gk_rushing_out: scale(self.rushing_out),
            gk_punching: scale(self.punching),
            gk_throwing: scale(self.throwing),
        }
    }
}

impl Default for Person {
    fn default() -> Self {
        Self {
            uid: 0,
            name: String::from("Unknown Player"),
            nationality: String::from("UNK"),
            team: String::from("Free Agent"),
            position: String::from("MF"),
            ca: 50,
            pa: 50,
            age: 18,
            // Default all FM attributes to 10 (1-20 scale)
            corners: 10,
            crossing: 10,
            dribbling: 10,
            finishing: 10,
            first_touch: 10,
            free_kick_taking: 10,
            heading: 10,
            long_shots: 10,
            long_throws: 10,
            marking: 10,
            passing: 10,
            penalty_taking: 10,
            tackling: 10,
            technique: 10,
            aggression: 10,
            anticipation: 10,
            bravery: 10,
            composure: 10,
            concentration: 10,
            decisions: 10,
            determination: 10,
            flair: 10,
            leadership: 10,
            off_the_ball: 10,
            positioning: 10,
            teamwork: 10,
            vision: 10,
            work_rate: 10,
            acceleration: 10,
            agility: 10,
            balance: 10,
            jumping: 10,
            natural_fitness: 10,
            pace: 10,
            stamina: 10,
            strength: 10,
            // Hidden/Physical (v5)
            stability: 10,
            foul: 10,
            contest_performance: 10,
            injury_proneness: 10,
            versatility: 10,
            // Goalkeeper (v5)
            aerial_reach: 1,
            command_of_area: 1,
            communication: 1,
            eccentricity: 10,
            handling: 1,
            gk_kicking: 1,
            one_on_ones: 1,
            reflexes: 1,
            rushing_out: 1,
            punching: 1,
            throwing: 1,
            // Personality (v5)
            adaptation: 10,
            ambition: 10,
            controversy: 10,
            loyalty: 10,
            pressure: 10,
            professionalism: 10,
            sportsmanship: 10,
            temperament: 10,
            // Physical Info (v5)
            height_cm: 175,
            weight_kg: 70,
            left_foot: 10,
            right_foot: 10,
            // Career/Financial (v5)
            market_value: 0,
            reputation_current: 1000,
            reputation_domestic: 1000,
            reputation_world: 1000,
            salary: 0,
            loan_club: String::new(),
            // Personal Info (v5)
            ethnicity: String::new(),
            rca: 0,
            skin_color: 0,
            birth_date: String::new(),
            national_team_caps: 0,
            national_team_goals: 0,
            // Position Ratings
            position_ratings: None,
        }
    }
}

impl Person {
    /// Get all attributes as HashMap (for conversion to PlayerAttributes)
    ///
    /// Returns all 36 FM attributes in 1-20 scale.
    /// Use with ScaleConverter to convert to match engine 0-100 scale.
    pub fn get_attributes_map(&self) -> std::collections::HashMap<String, u8> {
        let mut attrs = std::collections::HashMap::new();

        // Technical (14)
        attrs.insert("corners".to_string(), self.corners);
        attrs.insert("crossing".to_string(), self.crossing);
        attrs.insert("dribbling".to_string(), self.dribbling);
        attrs.insert("finishing".to_string(), self.finishing);
        attrs.insert("first_touch".to_string(), self.first_touch);
        attrs.insert("free_kick_taking".to_string(), self.free_kick_taking);
        attrs.insert("heading".to_string(), self.heading);
        attrs.insert("long_shots".to_string(), self.long_shots);
        attrs.insert("long_throws".to_string(), self.long_throws);
        attrs.insert("marking".to_string(), self.marking);
        attrs.insert("passing".to_string(), self.passing);
        attrs.insert("penalty_taking".to_string(), self.penalty_taking);
        attrs.insert("tackling".to_string(), self.tackling);
        attrs.insert("technique".to_string(), self.technique);

        // Mental (14)
        attrs.insert("aggression".to_string(), self.aggression);
        attrs.insert("anticipation".to_string(), self.anticipation);
        attrs.insert("bravery".to_string(), self.bravery);
        attrs.insert("composure".to_string(), self.composure);
        attrs.insert("concentration".to_string(), self.concentration);
        attrs.insert("decisions".to_string(), self.decisions);
        attrs.insert("determination".to_string(), self.determination);
        attrs.insert("flair".to_string(), self.flair);
        attrs.insert("leadership".to_string(), self.leadership);
        attrs.insert("off_the_ball".to_string(), self.off_the_ball);
        attrs.insert("positioning".to_string(), self.positioning);
        attrs.insert("teamwork".to_string(), self.teamwork);
        attrs.insert("vision".to_string(), self.vision);
        attrs.insert("work_rate".to_string(), self.work_rate);

        // Physical (8)
        attrs.insert("acceleration".to_string(), self.acceleration);
        attrs.insert("agility".to_string(), self.agility);
        attrs.insert("balance".to_string(), self.balance);
        attrs.insert("jumping".to_string(), self.jumping);
        attrs.insert("natural_fitness".to_string(), self.natural_fitness);
        attrs.insert("pace".to_string(), self.pace);
        attrs.insert("stamina".to_string(), self.stamina);
        attrs.insert("strength".to_string(), self.strength);

        // Goalkeeper (11) - v5 schema
        attrs.insert("gk_aerial_reach".to_string(), self.aerial_reach);
        attrs.insert("gk_command_of_area".to_string(), self.command_of_area);
        attrs.insert("gk_communication".to_string(), self.communication);
        attrs.insert("gk_eccentricity".to_string(), self.eccentricity);
        attrs.insert("gk_handling".to_string(), self.handling);
        attrs.insert("gk_kicking".to_string(), self.gk_kicking);
        attrs.insert("gk_one_on_ones".to_string(), self.one_on_ones);
        attrs.insert("gk_reflexes".to_string(), self.reflexes);
        attrs.insert("gk_rushing_out".to_string(), self.rushing_out);
        attrs.insert("gk_punching".to_string(), self.punching);
        attrs.insert("gk_throwing".to_string(), self.throwing);

        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_creation() {
        let person = Person::new(
            1,
            "Lionel Messi".to_string(),
            "ARG".to_string(),
            "Barcelona".to_string(),
            "AM (RC), ST (C)".to_string(),
            195,
            200,
            32,
            None,
        );

        assert_eq!(person.uid, 1);
        assert_eq!(person.name, "Lionel Messi");
        assert_eq!(person.display_name(), "Lionel Messi");
        assert_eq!(person.ca, 195);
        assert_eq!(person.pa, 200);
        assert_eq!(person.age, 32);
    }

    #[test]
    fn test_wonderkid_detection() {
        let wonderkid = Person::new(
            1,
            "Young Talent".to_string(),
            "ENG".to_string(),
            "Academy".to_string(),
            "ST".to_string(),
            150,
            185,
            19,
            None,
        );

        assert!(wonderkid.is_wonderkid());
        assert!(!wonderkid.is_veteran());
    }

    #[test]
    fn test_position_parsing() {
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test Team".to_string(),
            "AM (RC), ST (C)".to_string(),
            150,
            160,
            25,
            None,
        );

        assert_eq!(person.primary_position(), "AM");

        let positions = person.all_positions();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&"AM"));
        assert!(positions.contains(&"ST"));
    }

    #[test]
    fn test_world_class_detection() {
        let world_class = Person::new(
            1,
            "Star Player".to_string(),
            "ESP".to_string(),
            "Real Madrid".to_string(),
            "ST".to_string(),
            180,
            185,
            27,
            None,
        );

        assert!(world_class.is_world_class());
        assert!(world_class.is_top_talent());
        assert!(world_class.is_prime_age());
    }

    #[test]
    fn test_growth_potential() {
        let person = Person::new(
            1,
            "Developing Player".to_string(),
            "BRA".to_string(),
            "Santos".to_string(),
            "AM".to_string(),
            140,
            175,
            20,
            None,
        );

        assert_eq!(person.growth_potential(), 35);
    }

    #[test]
    fn test_position_ratings_field_storage() {
        // Test that position_ratings field is properly stored
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            Some("1,5,7,6,3,4,10,12,15,13,18,20,17,19".to_string()),
        );

        assert!(person.position_ratings.is_some());
        assert_eq!(person.position_ratings.unwrap(), "1,5,7,6,3,4,10,12,15,13,18,20,17,19");
    }

    #[test]
    fn test_position_ratings_none() {
        // Test that position_ratings can be None
        let person = Person::default();

        assert!(person.position_ratings.is_none());
        assert_eq!(person.name, "Unknown Player");
        assert_eq!(person.ca, 50);
    }

    #[test]
    fn test_get_position_ratings() {
        // Test parsing position ratings string to array
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            Some("1,5,7,6,3,4,10,12,15,13,18,20,17,19".to_string()),
        );

        let ratings = person.get_position_ratings().unwrap();

        assert_eq!(ratings[0], 1); // GK
        assert_eq!(ratings[1], 5); // DL
        assert_eq!(ratings[2], 7); // DC
        assert_eq!(ratings[8], 15); // MC
        assert_eq!(ratings[11], 20); // AMC
        assert_eq!(ratings[13], 19); // ST
    }

    #[test]
    fn test_get_position_rating() {
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            Some("1,5,7,6,3,4,10,12,15,13,18,20,17,19".to_string()),
        );

        assert_eq!(person.get_position_rating(PositionRating::GK), Some(1));
        assert_eq!(person.get_position_rating(PositionRating::MC), Some(15));
        assert_eq!(person.get_position_rating(PositionRating::AMC), Some(20));
        assert_eq!(person.get_position_rating(PositionRating::ST), Some(19));
    }

    #[test]
    fn test_position_suitability_calculation() {
        // Position order: GK, DL, DC, DR, WBL, WBR, DM, ML, MC, MR, AML, AMC, AMR, ST
        // Test data:       0,  3,  8,  13, 2,   5,   6,  12, 15, 18, 10,  14,  19,  4
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            Some("0,3,8,13,2,5,6,12,15,18,10,14,19,4".to_string()),
        );

        // Rating 0: 0.0 (cannot play)
        assert_eq!(person.position_suitability(PositionRating::GK), 0.0);

        // Rating 1-5: 0.3 (very poor)
        assert_eq!(person.position_suitability(PositionRating::DL), 0.3); // 3
        assert_eq!(person.position_suitability(PositionRating::WBL), 0.3); // 2
        assert_eq!(person.position_suitability(PositionRating::WBR), 0.3); // 5
        assert_eq!(person.position_suitability(PositionRating::ST), 0.3); // 4

        // Rating 6-10: 0.6 (adequate)
        assert_eq!(person.position_suitability(PositionRating::DC), 0.6); // 8
        assert_eq!(person.position_suitability(PositionRating::DM), 0.6); // 6
        assert_eq!(person.position_suitability(PositionRating::AML), 0.6); // 10

        // Rating 11-14: 0.85 (good)
        assert_eq!(person.position_suitability(PositionRating::DR), 0.85); // 13
        assert_eq!(person.position_suitability(PositionRating::ML), 0.85); // 12
        assert_eq!(person.position_suitability(PositionRating::AMC), 0.85); // 14

        // Rating 15-20: 1.0 (natural)
        assert_eq!(person.position_suitability(PositionRating::MC), 1.0); // 15
        assert_eq!(person.position_suitability(PositionRating::MR), 1.0); // 18
        assert_eq!(person.position_suitability(PositionRating::AMR), 1.0); // 19
    }

    #[test]
    fn test_position_suitability_fallback() {
        // Person without position_ratings should return 0.5 (50% penalty)
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            None,
        );

        assert_eq!(person.position_suitability(PositionRating::MC), 0.5);
        assert_eq!(person.position_suitability(PositionRating::GK), 0.5);
    }

    #[test]
    fn test_best_positions() {
        let person = Person::new(
            1,
            "Kevin De Bruyne".to_string(),
            "Belgium".to_string(),
            "Manchester City".to_string(),
            "MC".to_string(),
            189,
            189,
            31,
            Some("1,1,1,1,1,1,12,15,20,15,14,20,14,12".to_string()),
        );

        // Get best positions with rating >= 15
        let best = person.best_positions(15);

        assert_eq!(best.len(), 4);
        assert_eq!(best[0], (PositionRating::MC, 20));
        assert_eq!(best[1], (PositionRating::AMC, 20));
        // ML and MR both have 15, order may vary
        assert!(best.iter().any(|&(pos, rating)| pos == PositionRating::ML && rating == 15));
        assert!(best.iter().any(|&(pos, rating)| pos == PositionRating::MR && rating == 15));
    }

    #[test]
    fn test_position_rating_enum() {
        assert_eq!(PositionRating::GK as u8, 0);
        assert_eq!(PositionRating::MC as u8, 8);
        assert_eq!(PositionRating::ST as u8, 13);

        assert_eq!(PositionRating::MC.name(), "MC");
        assert_eq!(PositionRating::ST.name(), "ST");

        assert_eq!(PositionRating::from_str("MC"), Some(PositionRating::MC));
        assert_eq!(PositionRating::from_str("mc"), Some(PositionRating::MC));
        assert_eq!(PositionRating::from_str("INVALID"), None);

        assert_eq!(PositionRating::all().len(), 14);
    }
}
