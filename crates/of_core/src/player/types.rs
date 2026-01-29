//! Core types for the player system
//!
//! This module contains the fundamental data structures:
//! - CorePlayer: The main player entity with CA/PA system
//! - HexagonStats: 6-sided ability visualization
//! - GrowthProfile: Growth rates and training responses

use crate::models::player::{PlayerAttributes, Position};
use crate::player::ca_model::{generate_attributes, CAParams, CAProfile};
use crate::player::ca_weights::get_ca_weights;
use crate::player::calculator::CACalculator;
use crate::player::growth_calculator::GrowthCalculator;
use crate::player::instructions::{PlayerInstructions, PlayerRole};
use crate::player::personality::PersonAttributes;
use crate::special_ability::{
    AbilityTier, SpecialAbility, SpecialAbilityCollection, SpecialAbilityType,
};
use chrono::{DateTime, Utc};
use rand::prelude::SliceRandom;
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Attribute type classification for position-specific generation
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum AttributeType {
    Technical,
    Mental,
    Physical,
    Pace,
    Shooting,
    Passing,
    Defending,
}

/// The main player entity with CA/PA system and comprehensive attributes
///
/// Fields are ordered by size (largest to smallest) to minimize padding:
/// - Large structs first (PlayerAttributes, GrowthProfile)
/// - Medium structs (HexagonStats, DateTime)
/// - Strings (String)
/// - Primitives by size (f32, Position enum, u8)
/// The main player entity with CA/PA system and comprehensive attributes
///
/// MIGRATION NOTE: Fields marked with `#[serde(default)]` were added after initial release.
/// This ensures backward compatibility when loading old save files that don't have these fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorePlayer {
    // ========== Original fields (required for all versions) ==========
    // Large structs first (minimize padding)
    pub detailed_stats: PlayerAttributes, // 42 * u8 = 42 bytes
    pub growth_profile: GrowthProfile,    // ~32+ bytes

    // Medium structs
    pub created_at: DateTime<Utc>,       // 12 bytes
    pub updated_at: DateTime<Utc>,       // 12 bytes
    pub hexagon_stats: HexagonStats,     // 6 * u8 = 6 bytes
    pub personality: PersonAttributes,   // 8 * u8 = 8 bytes

    // Strings (heap allocated, pointer + length + capacity)
    pub id: String,   // 24 bytes on 64-bit
    pub name: String, // 24 bytes on 64-bit

    // Primitives by size (f32 > enum > u8)
    pub age_months: f32,       // 4 bytes, 15.0 (고1) to 18.0 (고3)
    pub position: Position,    // 1-4 bytes (enum)
    pub ca: u8,                // 1 byte, Current Ability (0-200)
    pub pa: u8,                // 1 byte, Potential Ability (80-180)

    // ========== MIGRATION SAFE: Fields added after v1.0 ==========
    // These fields use #[serde(default)] to ensure old saves load correctly

    /// Special abilities collection (added v1.1)
    #[serde(default)]
    pub special_abilities: SpecialAbilityCollection, // Vec + history = ~64+ bytes

    /// Career statistics tracking (added v1.1)
    #[serde(default)]
    pub career_stats: PlayerCareerStats, // 경력 통계 추적

    /// Match instructions - Role + custom (added v1.2)
    #[serde(default)]
    pub instructions: Option<PlayerInstructions>,

    /// Current active injury (added v1.2)
    #[serde(default)]
    pub current_injury: Option<crate::training::Injury>,

    /// Injury history log (added v1.2)
    #[serde(default)]
    pub injury_history: Vec<crate::training::Injury>,

    /// Injury proneness factor 0.0-1.0 (added v1.2)
    #[serde(default = "default_injury_proneness")]
    pub injury_proneness: f32,
}

/// Default injury proneness value for migration (low risk)
fn default_injury_proneness() -> f32 {
    0.1
}

/// Result of an attribute modification operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeChange {
    pub attribute_name: String,
    pub old_value: u8,
    pub new_value: u8,
    pub before_ca: u8,
    pub after_ca: u8,
    pub before_hexagon: HexagonStats,
    pub after_hexagon: HexagonStats,
}

/// Training type for growth simulation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TrainingType {
    Technical,
    Physical,
    Mental,
    Shooting,
    Passing,
    Defending,
    Goalkeeping,
    General, // Position-specific training
}

/// Result of attribute growth from training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeGrowth {
    pub attribute_name: String,
    pub growth_amount: i8,
    pub old_value: u8,
    pub new_value: u8,
}

/// Monthly growth tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyGrowth {
    pub month: u32,
    pub age_months: f32,
    pub ca_start: u8,
    pub ca_end: u8,
    pub attribute_growths: Vec<AttributeGrowth>,
    pub growth_rate: f32,
}

/// Player's current growth potential analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthPotential {
    pub current_growth_rate: f32,
    pub sessions_to_potential: Option<u32>,
    pub ca_to_pa_ratio: f32,
    pub age_modifier: f32,
    pub years_of_development_left: f32,
}

/// Injury severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InjurySeverity {
    Minor,
    Moderate,
    Severe,
}

/// Result of injury application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjuryEffect {
    pub occurred: bool,
    pub severity: InjurySeverity,
    pub affected_attributes: Vec<String>,
    pub recovery_time_months: f32,
    pub growth_penalty: f32, // Multiplier for growth rate during recovery
}

/// 시즌별 통계 추적
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SeasonStats {
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub clean_sheets: u32,         // GK용
    pub perfect_rating_games: u32, // 9.0+ 평점 경기 수
}

impl SeasonStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// 경기 결과 기록
    pub fn record_game(&mut self, goals: u32, assists: u32, clean_sheet: bool, rating: f32) {
        self.games += 1;
        self.goals += goals;
        self.assists += assists;
        if clean_sheet {
            self.clean_sheets += 1;
        }
        if rating >= 9.0 {
            self.perfect_rating_games += 1;
        }
    }

    /// 완벽한 시즌인지 확인 (모든 경기 9.0+ 평점)
    pub fn is_perfect_season(&self) -> bool {
        self.games > 0 && self.perfect_rating_games == self.games
    }

    /// 시즌 초기화
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 선수 경력 통계 추적
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlayerCareerStats {
    /// 총 출전 경기 수
    pub games_played: u32,
    /// 총 트레이닝 세션 수
    pub training_sessions: u32,
    /// 팀 주장 여부
    pub is_team_captain: bool,
    /// 국가대표 여부
    pub is_national_team_player: bool,
    /// 주요 타이틀 수
    pub major_titles: u32,
    /// 완벽한 경기 수 (9.0+ 평점)
    pub perfect_games: u32,
    /// 현재 시즌 통계
    pub current_season: SeasonStats,
    /// 관계 점수 (0.0-1.0, 모든 관계의 평균)
    pub relationship_score: f32,
}

impl PlayerCareerStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// 경기 출전 기록
    pub fn record_game(&mut self, goals: u32, assists: u32, clean_sheet: bool, rating: f32) {
        self.games_played += 1;
        if rating >= 9.0 {
            self.perfect_games += 1;
        }
        self.current_season.record_game(goals, assists, clean_sheet, rating);
    }

    /// 트레이닝 세션 기록
    pub fn record_training_session(&mut self) {
        self.training_sessions += 1;
    }

    /// 트레이닝 일관성 계산 (최근 세션 기준)
    /// 간단한 구현: 세션 수가 많을수록 높은 일관성
    pub fn calculate_training_consistency(&self) -> f32 {
        // 100세션 = 1.0, 0세션 = 0.5
        let base = 0.5;
        let bonus = (self.training_sessions as f32 / 100.0).min(0.5);
        base + bonus
    }

    /// 모든 관계가 최대인지 확인
    pub fn are_all_relationships_maxed(&self) -> bool {
        self.relationship_score >= 0.95
    }

    /// 새 시즌 시작
    pub fn start_new_season(&mut self) {
        self.current_season.reset();
    }
}

impl CorePlayer {
    /// Create a new CorePlayer with generated ID and current timestamps
    pub fn new(
        name: String,
        position: Position,
        age_months: f32,
        ca: u8,
        pa: u8,
        detailed_stats: PlayerAttributes,
        growth_profile: GrowthProfile,
        personality: PersonAttributes,
    ) -> Self {
        let now = Utc::now();
        let hexagon_stats = HexagonStats::calculate_from_detailed(&detailed_stats, position);

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            position,
            age_months,
            ca,
            pa,
            detailed_stats,
            hexagon_stats,
            growth_profile,
            personality,
            special_abilities: SpecialAbilityCollection::new(),
            career_stats: PlayerCareerStats::new(),
            instructions: None,
            current_injury: None,
            injury_history: Vec::new(),
            injury_proneness: 0.1,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the player's timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Recalculate hexagon stats from detailed attributes
    pub fn recalculate_hexagon_stats(&mut self) {
        self.hexagon_stats =
            HexagonStats::calculate_from_detailed(&self.detailed_stats, self.position);
        self.touch();
    }

    /// Calculate CA from current detailed stats and position
    pub fn calculate_ca(&self) -> u8 {
        CACalculator::calculate(&self.detailed_stats, self.position)
    }

    /// Recalculate CA and update the stored value
    /// This should be called whenever attributes or position change
    pub fn recalculate_ca(&mut self) {
        self.ca = self.calculate_ca();
        self.touch();
    }

    /// Recalculate both CA and hexagon stats
    /// Convenience method for when detailed stats are modified
    pub fn recalculate_all(&mut self) {
        self.ca = self.calculate_ca();
        self.hexagon_stats =
            HexagonStats::calculate_from_detailed(&self.detailed_stats, self.position);
        self.touch();
    }

    /// Change position and recalculate CA and hexagon stats
    /// Position changes affect both CA weighting and hexagon calculations
    pub fn change_position(&mut self, new_position: Position) {
        self.position = new_position;
        self.recalculate_all();
    }

    /// Validate that CA is consistent with current detailed stats
    /// Returns true if stored CA matches calculated CA
    pub fn is_ca_consistent(&self) -> bool {
        self.ca == self.calculate_ca()
    }

    /// Get CA calculation details for debugging
    pub fn get_ca_breakdown(&self) -> crate::player::calculator::CACalculationDetails {
        CACalculator::calculate_detailed(&self.detailed_stats, self.position)
    }

    /// Add a special ability to the player
    pub fn add_special_ability(&mut self, ability_type: SpecialAbilityType, tier: AbilityTier) {
        let ability = SpecialAbility::new(ability_type, tier);
        self.special_abilities.add_ability(ability);
        self.touch();
    }

    /// Check if player has a specific special ability type (any tier)
    pub fn has_special_ability(&self, ability_type: SpecialAbilityType) -> bool {
        self.special_abilities.has_ability(ability_type)
    }

    /// Check if player has a specific special ability with exact tier
    pub fn has_exact_special_ability(
        &self,
        ability_type: SpecialAbilityType,
        tier: AbilityTier,
    ) -> bool {
        self.special_abilities.has_exact_ability(ability_type, tier)
    }

    /// Get all special abilities
    pub fn get_special_abilities(&self) -> &Vec<SpecialAbility> {
        &self.special_abilities.abilities
    }

    /// Get positive special abilities only
    pub fn get_positive_special_abilities(&self) -> Vec<&SpecialAbility> {
        self.special_abilities.positive_abilities()
    }

    /// Get negative special abilities only
    pub fn get_negative_special_abilities(&self) -> Vec<&SpecialAbility> {
        self.special_abilities.negative_abilities()
    }

    /// Process special abilities and return effects to apply to OpenFootball skills
    pub fn process_special_abilities(
        &mut self,
        activation_context: &crate::special_ability::AbilityActivationContext,
    ) -> crate::special_ability::ProcessingResult {
        use crate::special_ability::{PlayerContext, SpecialAbilityProcessor};

        // Create player context for combination conditions using actual career stats
        let player_context = PlayerContext {
            current_ability: self.ca,
            potential_ability: self.pa,
            games_played: self.career_stats.games_played,
            training_consistency: self.career_stats.calculate_training_consistency(),
            is_team_captain: self.career_stats.is_team_captain,
            is_national_team_player: self.career_stats.is_national_team_player,
            major_titles: self.career_stats.major_titles,
            perfect_games: self.career_stats.perfect_games,
            perfect_season: self.career_stats.current_season.is_perfect_season(),
            all_relationships_maxed: self.career_stats.are_all_relationships_maxed(),
        };

        let result = SpecialAbilityProcessor::process_abilities(
            &mut self.special_abilities,
            &player_context,
            activation_context,
        );

        if result.has_combinations {
            self.touch(); // Update timestamp if combinations occurred
        }

        result
    }

    /// Apply special ability effects to OpenFootball PlayerSkills
    pub fn apply_special_ability_effects_to_openfootball(
        &self,
        base_skills: &mut crate::special_ability::OpenFootballSkills,
    ) {
        use crate::special_ability::{AbilityEffectCalculator, SpecialAbilityProcessor};

        // Calculate combined effects from all special abilities
        let effects =
            AbilityEffectCalculator::calculate_combined_effects(&self.special_abilities.abilities);

        // Apply effects to OpenFootball skills
        SpecialAbilityProcessor::apply_effects_to_openfootball_skills(base_skills, &effects);
    }

    /// Modify a single attribute with bounds checking and auto-recalculation
    /// Returns the before/after values for UI feedback
    pub fn modify_attribute(
        &mut self,
        attribute_name: &str,
        change: i8,
    ) -> Result<AttributeChange, String> {
        let before_ca = self.ca;
        let before_hexagon = self.hexagon_stats.clone();

        // Apply the change to the specified attribute
        let old_value = self.apply_attribute_change(attribute_name, change)?;
        let new_value = self.get_attribute_value(attribute_name)?;

        // Recalculate CA and hexagon stats
        self.recalculate_all();

        Ok(AttributeChange {
            attribute_name: attribute_name.to_string(),
            old_value,
            new_value,
            before_ca,
            after_ca: self.ca,
            before_hexagon,
            after_hexagon: self.hexagon_stats.clone(),
        })
    }

    /// Modify multiple attributes atomically (all changes or none)
    pub fn modify_multiple_attributes(
        &mut self,
        changes: &[(&str, i8)],
    ) -> Result<Vec<AttributeChange>, String> {
        // Validate all changes first
        for (attr_name, change) in changes {
            let current_value = self.get_attribute_value(attr_name)?;
            let new_value = (current_value as i16 + *change as i16).clamp(0, 100) as u8;
            if new_value == current_value {
                continue; // No actual change
            }
        }

        // Create a backup in case we need to rollback
        let backup_stats = self.detailed_stats.clone();
        let backup_ca = self.ca;
        let backup_hexagon = self.hexagon_stats.clone();

        let mut results = Vec::new();

        // Apply all changes
        for (attr_name, change) in changes {
            match self.modify_attribute(attr_name, *change) {
                Ok(change_result) => results.push(change_result),
                Err(e) => {
                    // Rollback all changes
                    self.detailed_stats = backup_stats;
                    self.ca = backup_ca;
                    self.hexagon_stats = backup_hexagon;
                    return Err(format!(
                        "Failed to modify {}: {}. All changes rolled back.",
                        attr_name, e
                    ));
                }
            }
        }

        Ok(results)
    }

    /// Helper method to apply a change to a specific attribute
    fn apply_attribute_change(&mut self, attribute_name: &str, change: i8) -> Result<u8, String> {
        let old_value = self.get_attribute_value(attribute_name)?;
        let new_value = (old_value as i16 + change as i16).clamp(0, 100) as u8;

        match attribute_name {
            // Technical (14) - OpenFootball original
            "corners" => self.detailed_stats.corners = new_value,
            "crossing" => self.detailed_stats.crossing = new_value,
            "dribbling" => self.detailed_stats.dribbling = new_value,
            "finishing" => self.detailed_stats.finishing = new_value,
            "first_touch" => self.detailed_stats.first_touch = new_value,
            "free_kicks" => self.detailed_stats.free_kicks = new_value,
            "heading" => self.detailed_stats.heading = new_value,
            "long_shots" => self.detailed_stats.long_shots = new_value,
            "long_throws" => self.detailed_stats.long_throws = new_value,
            "marking" => self.detailed_stats.marking = new_value,
            "passing" => self.detailed_stats.passing = new_value,
            "penalty_taking" => self.detailed_stats.penalty_taking = new_value,
            "tackling" => self.detailed_stats.tackling = new_value,
            "technique" => self.detailed_stats.technique = new_value,

            // Mental (14) - OpenFootball original
            "aggression" => self.detailed_stats.aggression = new_value,
            "anticipation" => self.detailed_stats.anticipation = new_value,
            "bravery" => self.detailed_stats.bravery = new_value,
            "composure" => self.detailed_stats.composure = new_value,
            "concentration" => self.detailed_stats.concentration = new_value,
            "decisions" => self.detailed_stats.decisions = new_value,
            "determination" => self.detailed_stats.determination = new_value,
            "flair" => self.detailed_stats.flair = new_value,
            "leadership" => self.detailed_stats.leadership = new_value,
            "off_the_ball" => self.detailed_stats.off_the_ball = new_value,
            "positioning" => self.detailed_stats.positioning = new_value,
            "teamwork" => self.detailed_stats.teamwork = new_value,
            "vision" => self.detailed_stats.vision = new_value,
            "work_rate" => self.detailed_stats.work_rate = new_value,

            // Physical (8) - OpenFootball original
            "acceleration" => self.detailed_stats.acceleration = new_value,
            "agility" => self.detailed_stats.agility = new_value,
            "balance" => self.detailed_stats.balance = new_value,
            "jumping" => self.detailed_stats.jumping = new_value,
            "natural_fitness" => self.detailed_stats.natural_fitness = new_value,
            "pace" => self.detailed_stats.pace = new_value,
            "stamina" => self.detailed_stats.stamina = new_value,
            "strength" => self.detailed_stats.strength = new_value,

            _ => return Err(format!("Unknown attribute: {}", attribute_name)),
        }

        Ok(old_value)
    }

    /// Get the current value of an attribute by name
    pub fn get_attribute_value(&self, attribute_name: &str) -> Result<u8, String> {
        match attribute_name {
            // Technical (14) - OpenFootball original
            "corners" => Ok(self.detailed_stats.corners),
            "crossing" => Ok(self.detailed_stats.crossing),
            "dribbling" => Ok(self.detailed_stats.dribbling),
            "finishing" => Ok(self.detailed_stats.finishing),
            "first_touch" => Ok(self.detailed_stats.first_touch),
            "free_kicks" => Ok(self.detailed_stats.free_kicks),
            "heading" => Ok(self.detailed_stats.heading),
            "long_shots" => Ok(self.detailed_stats.long_shots),
            "long_throws" => Ok(self.detailed_stats.long_throws),
            "marking" => Ok(self.detailed_stats.marking),
            "passing" => Ok(self.detailed_stats.passing),
            "penalty_taking" => Ok(self.detailed_stats.penalty_taking),
            "tackling" => Ok(self.detailed_stats.tackling),
            "technique" => Ok(self.detailed_stats.technique),

            // Mental (14) - OpenFootball original
            "aggression" => Ok(self.detailed_stats.aggression),
            "anticipation" => Ok(self.detailed_stats.anticipation),
            "bravery" => Ok(self.detailed_stats.bravery),
            "composure" => Ok(self.detailed_stats.composure),
            "concentration" => Ok(self.detailed_stats.concentration),
            "decisions" => Ok(self.detailed_stats.decisions),
            "determination" => Ok(self.detailed_stats.determination),
            "flair" => Ok(self.detailed_stats.flair),
            "leadership" => Ok(self.detailed_stats.leadership),
            "off_the_ball" => Ok(self.detailed_stats.off_the_ball),
            "positioning" => Ok(self.detailed_stats.positioning),
            "teamwork" => Ok(self.detailed_stats.teamwork),
            "vision" => Ok(self.detailed_stats.vision),
            "work_rate" => Ok(self.detailed_stats.work_rate),

            // Physical (8) - OpenFootball original
            "acceleration" => Ok(self.detailed_stats.acceleration),
            "agility" => Ok(self.detailed_stats.agility),
            "balance" => Ok(self.detailed_stats.balance),
            "jumping" => Ok(self.detailed_stats.jumping),
            "natural_fitness" => Ok(self.detailed_stats.natural_fitness),
            "pace" => Ok(self.detailed_stats.pace),
            "stamina" => Ok(self.detailed_stats.stamina),
            "strength" => Ok(self.detailed_stats.strength),

            _ => Err(format!("Unknown attribute: {}", attribute_name)),
        }
    }

    /// Get list of all valid attribute names (36 OpenFootball original fields)
    pub fn get_all_attribute_names() -> Vec<&'static str> {
        vec![
            // Technical (14)
            "corners",
            "crossing",
            "dribbling",
            "finishing",
            "first_touch",
            "free_kicks",
            "heading",
            "long_shots",
            "long_throws",
            "marking",
            "passing",
            "penalty_taking",
            "tackling",
            "technique",
            // Mental (14)
            "aggression",
            "anticipation",
            "bravery",
            "composure",
            "concentration",
            "decisions",
            "determination",
            "flair",
            "leadership",
            "off_the_ball",
            "positioning",
            "teamwork",
            "vision",
            "work_rate",
            // Physical (8)
            "acceleration",
            "agility",
            "balance",
            "jumping",
            "natural_fitness",
            "pace",
            "stamina",
            "strength",
        ]
    }

    /// Generate a realistic player with position-appropriate attributes
    /// Uses deterministic random generation with ChaCha8Rng
    pub fn generate_player(
        name: String,
        position: Position,
        ca_range: (u8, u8),
        pa_range: (u8, u8),
        age_months: f32,
        seed: u64,
    ) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        // Generate target CA and PA
        let target_ca = rng.gen_range(ca_range.0..=ca_range.1);
        let target_pa = rng.gen_range(pa_range.0.max(target_ca)..=pa_range.1);

        // Generate position-appropriate attributes
        let detailed_stats = Self::generate_attributes_for_position(position, target_ca, &mut rng);

        // Calculate actual CA from generated attributes
        let actual_ca = CACalculator::calculate(&detailed_stats, position);

        // Generate growth profile
        let growth_profile = Self::generate_growth_profile(&mut rng);

        // Generate personality
        let personality = PersonAttributes::generate_random(rng.next_u64());

        // Create player
        let mut player = Self::new(
            name,
            position,
            age_months,
            actual_ca,
            target_pa,
            detailed_stats,
            growth_profile,
            personality,
        );

        // Generate starting special abilities (rare, low-tier only for new players)
        Self::generate_starting_special_abilities(&mut player, &mut rng);

        // Adjust attributes to better match target CA if needed
        if (actual_ca as i16 - target_ca as i16).abs() > 5 {
            player.adjust_attributes_to_target_ca(target_ca, &mut rng);
        }

        player
    }

    /// Generate position-appropriate attributes for a given position and target CA
    fn generate_attributes_for_position(
        position: Position,
        target_ca: u8,
        rng: &mut ChaCha8Rng,
    ) -> PlayerAttributes {
        let seed = rng.next_u64();
        let weights = get_ca_weights();
        let params = CAParams::default();
        generate_attributes(target_ca, position, CAProfile::Balanced, seed, weights, params)
    }

    /// Generate a single attribute with position-specific bias
    #[allow(dead_code)]
    fn generate_attribute_with_position_bias(
        base_value: u8,
        variation: u8,
        position: Position,
        attr_type: AttributeType,
        rng: &mut ChaCha8Rng,
    ) -> u8 {
        let bias = Self::get_position_attribute_bias(position, attr_type);
        let biased_base = ((base_value as f32) * bias).round() as u8;

        let min_val = biased_base.saturating_sub(variation);
        let max_val = (biased_base + variation).min(100);

        rng.gen_range(min_val..=max_val)
    }

    /// Get position-specific bias for different attribute types
    #[allow(dead_code)]
    fn get_position_attribute_bias(position: Position, attr_type: AttributeType) -> f32 {
        match (position, attr_type) {
            // Forwards
            (
                Position::FW | Position::ST | Position::CF | Position::LW | Position::RW,
                AttributeType::Shooting,
            ) => 1.4,
            (
                Position::FW | Position::ST | Position::CF | Position::LW | Position::RW,
                AttributeType::Pace,
            ) => 1.2,
            (
                Position::FW | Position::ST | Position::CF | Position::LW | Position::RW,
                AttributeType::Technical,
            ) => 1.1,
            (
                Position::FW | Position::ST | Position::CF | Position::LW | Position::RW,
                AttributeType::Defending,
            ) => 0.7,

            // Midfielders
            (
                Position::MF
                | Position::CM
                | Position::CAM
                | Position::CDM
                | Position::LM
                | Position::RM,
                AttributeType::Technical,
            ) => 1.3,
            (
                Position::MF
                | Position::CM
                | Position::CAM
                | Position::CDM
                | Position::LM
                | Position::RM,
                AttributeType::Passing,
            ) => 1.3,
            (
                Position::MF
                | Position::CM
                | Position::CAM
                | Position::CDM
                | Position::LM
                | Position::RM,
                AttributeType::Mental,
            ) => 1.2,

            // Defenders
            (
                Position::DF
                | Position::CB
                | Position::LB
                | Position::RB
                | Position::LWB
                | Position::RWB,
                AttributeType::Defending,
            ) => 1.4,
            (
                Position::DF
                | Position::CB
                | Position::LB
                | Position::RB
                | Position::LWB
                | Position::RWB,
                AttributeType::Physical,
            ) => 1.2,
            (
                Position::DF
                | Position::CB
                | Position::LB
                | Position::RB
                | Position::LWB
                | Position::RWB,
                AttributeType::Shooting,
            ) => 0.6,

            // Goalkeepers (handled separately in main function)
            (Position::GK, _) => 1.0,

            // Default
            _ => 1.0,
        }
    }

    /// Adjust player attributes to better match target CA
    fn adjust_attributes_to_target_ca(&mut self, target_ca: u8, rng: &mut ChaCha8Rng) {
        let weights = get_ca_weights();
        let params = CAParams::default();
        let seed = rng.next_u64();
        self.detailed_stats = generate_attributes(
            target_ca,
            self.position,
            CAProfile::Balanced,
            seed,
            weights,
            params,
        );
        self.recalculate_all();
    }

    /// Get key attributes for a position that should be adjusted for CA matching (OpenFootball original only)
    #[allow(dead_code)]
    fn get_key_attributes_for_position(position: Position) -> Vec<String> {
        match position {
            Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => {
                vec![
                    "finishing".to_string(),
                    "long_shots".to_string(),
                    "pace".to_string(),
                    "acceleration".to_string(),
                ]
            }
            Position::MF
            | Position::CM
            | Position::CAM
            | Position::CDM
            | Position::LM
            | Position::RM => {
                vec![
                    "passing".to_string(),
                    "vision".to_string(),
                    "technique".to_string(),
                    "first_touch".to_string(),
                ]
            }
            Position::DF
            | Position::CB
            | Position::LB
            | Position::RB
            | Position::LWB
            | Position::RWB => {
                vec![
                    "positioning".to_string(),
                    "anticipation".to_string(),
                    "strength".to_string(),
                    "heading".to_string(),
                ]
            }
            Position::GK => {
                // GK-specific: OpenFootball original doesn't have GK attributes, use adapted base attributes
                vec![
                    "first_touch".to_string(),
                    "concentration".to_string(),
                    "positioning".to_string(),
                    "anticipation".to_string(),
                ]
            }
        }
    }

    /// Generate a realistic growth profile
    fn generate_growth_profile(rng: &mut ChaCha8Rng) -> GrowthProfile {
        let growth_type = rng.gen_range(0..4);

        match growth_type {
            0 => GrowthProfile {
                growth_rate: 1.0,
                specialization: vec!["TECHNICAL".to_string()],
                training_response: TrainingResponse::technical_focused(),
                injury_prone: rng.gen_range(0.05..=0.15),
            },
            1 => GrowthProfile {
                growth_rate: 1.0,
                specialization: vec!["PHYSICAL".to_string()],
                training_response: TrainingResponse::physical_focused(),
                injury_prone: rng.gen_range(0.08..=0.20),
            },
            2 => GrowthProfile {
                growth_rate: 1.0,
                specialization: vec!["MENTAL".to_string()],
                training_response: TrainingResponse::mental_focused(),
                injury_prone: rng.gen_range(0.03..=0.10),
            },
            _ => GrowthProfile {
                growth_rate: 1.0,
                specialization: Vec::new(),
                training_response: TrainingResponse::balanced(),
                injury_prone: rng.gen_range(0.05..=0.15),
            },
        }
    }

    /// Generate starting special abilities for new players (rare occurrence)
    /// Only Bronze tier abilities, and only 5% chance per player
    fn generate_starting_special_abilities(player: &mut CorePlayer, rng: &mut ChaCha8Rng) {
        // Only 5% chance for new players to have a starting special ability
        if rng.gen::<f32>() > 0.05 {
            return;
        }

        // Select position-appropriate abilities
        let position_abilities = match player.position {
            Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => {
                vec![
                    SpecialAbilityType::ShootingStar,
                    SpecialAbilityType::DribblingMaster,
                    SpecialAbilityType::SpeedDemon,
                    SpecialAbilityType::AgilityMaster,
                ]
            }
            Position::MF
            | Position::CM
            | Position::CAM
            | Position::CDM
            | Position::LM
            | Position::RM => {
                vec![
                    SpecialAbilityType::PassingGenius,
                    SpecialAbilityType::TeamPlayer,
                    SpecialAbilityType::CaptainMaterial,
                    SpecialAbilityType::EnduranceKing,
                ]
            }
            Position::DF
            | Position::CB
            | Position::LB
            | Position::RB
            | Position::LWB
            | Position::RWB => {
                vec![
                    SpecialAbilityType::PowerHouse,
                    SpecialAbilityType::PressureHandler,
                    SpecialAbilityType::EnduranceKing,
                    SpecialAbilityType::AgilityMaster,
                ]
            }
            Position::GK => {
                vec![
                    SpecialAbilityType::ClutchPlayer,
                    SpecialAbilityType::PressureHandler,
                    SpecialAbilityType::CaptainMaterial,
                ]
            }
        };

        // Randomly select one ability (Bronze tier only)
        if let Some(&ability_type) = position_abilities.choose(rng) {
            player.add_special_ability(ability_type, AbilityTier::Bronze);
        }
    }

    /// Create preset players for common scenarios
    pub fn create_youth_prospect(name: String, position: Position, seed: u64) -> Self {
        Self::generate_player(name, position, (40, 70), (90, 150), 15.5, seed)
    }

    pub fn create_star_player(name: String, position: Position, seed: u64) -> Self {
        Self::generate_player(name, position, (120, 160), (140, 180), 16.5, seed)
    }

    pub fn create_average_player(name: String, position: Position, seed: u64) -> Self {
        Self::generate_player(name, position, (60, 100), (100, 130), 16.0, seed)
    }

    pub fn create_benchwarmer(name: String, position: Position, seed: u64) -> Self {
        Self::generate_player(name, position, (30, 60), (70, 100), 17.0, seed)
    }

    /// Apply growth based on training type and intensity
    /// Returns the attributes that were modified and their growth amounts
    pub fn apply_growth(
        &mut self,
        training_type: TrainingType,
        intensity: f32,
        seed: u64,
    ) -> Vec<AttributeGrowth> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut growths = Vec::new();

        // Calculate base growth rate
        let _base_growth_rate =
            GrowthCalculator::calculate_final_growth_rate(self.ca, self.pa, self.age_months);

        // Calculate growth points available for this training session
        let growth_points =
            GrowthCalculator::calculate_growth_points(self.ca, self.pa, self.age_months, intensity);

        if growth_points <= 0.0 {
            return growths; // No growth possible
        }

        // Get training-specific attribute list
        let training_attributes = self.get_training_attributes(training_type);

        // Apply training response multiplier
        let training_multiplier = self.get_training_multiplier(training_type);
        let adjusted_growth_points = growth_points * training_multiplier;

        // Distribute growth points among relevant attributes
        let points_per_attribute = adjusted_growth_points / training_attributes.len() as f32;

        for attr_name in training_attributes {
            // Add some randomness (±20% variation)
            let random_factor = rng.gen_range(0.8..=1.2);
            let attribute_growth = points_per_attribute * random_factor;

            if attribute_growth >= 0.5 {
                // Only apply if growth is meaningful
                let growth_amount = attribute_growth.round() as i8;

                // Check PA limits with diminishing returns
                let current_value = self.get_attribute_value(&attr_name).unwrap_or(0);
                let pa_limit_factor = Self::calculate_pa_limit_factor(current_value, self.pa);
                let final_growth = ((growth_amount as f32) * pa_limit_factor).round() as i8;

                if final_growth > 0 {
                    if let Ok(change) = self.modify_attribute(&attr_name, final_growth) {
                        growths.push(AttributeGrowth {
                            attribute_name: attr_name,
                            growth_amount: final_growth,
                            old_value: change.old_value,
                            new_value: change.new_value,
                        });
                    }
                }
            }
        }

        growths
    }

    /// Calculate PA limit factor for diminishing returns near potential
    fn calculate_pa_limit_factor(current_value: u8, pa: u8) -> f32 {
        let _progress = current_value as f32 / 100.0; // Progress towards max attribute
        let pa_factor = (pa as f32) / 180.0; // PA relative to max PA

        // Players with higher PA can grow attributes higher
        // Apply diminishing returns as attributes get closer to 100
        let effective_limit = (pa_factor * 100.0).min(95.0); // Even high PA players rarely get 100 in everything

        if current_value as f32 >= effective_limit {
            0.1 // Very slow growth near limit
        } else {
            let distance_from_limit = (effective_limit - current_value as f32) / effective_limit;
            (0.3 + distance_from_limit * 0.7).min(1.0) // Scale from 30% to 100%
        }
    }

    /// Get attributes relevant to training type (OpenFootball original only)
    fn get_training_attributes(&self, training_type: TrainingType) -> Vec<String> {
        match training_type {
            TrainingType::Technical => vec![
                "dribbling".to_string(),
                "first_touch".to_string(),
                "technique".to_string(),
                "flair".to_string(),
            ],
            TrainingType::Physical => vec![
                "pace".to_string(),
                "acceleration".to_string(),
                "strength".to_string(),
                "stamina".to_string(),
                "jumping".to_string(),
                "natural_fitness".to_string(),
            ],
            TrainingType::Mental => vec![
                "decisions".to_string(),
                "concentration".to_string(),
                "composure".to_string(),
                "leadership".to_string(),
                "determination".to_string(),
                "work_rate".to_string(),
            ],
            TrainingType::Shooting => vec![
                "finishing".to_string(),
                "long_shots".to_string(),
                "penalty_taking".to_string(),
                "composure".to_string(),
            ],
            TrainingType::Passing => vec![
                "passing".to_string(),
                "vision".to_string(),
                "crossing".to_string(),
                "free_kicks".to_string(),
                "corners".to_string(),
            ],
            TrainingType::Defending => vec![
                "positioning".to_string(),
                "anticipation".to_string(),
                "concentration".to_string(),
                "work_rate".to_string(),
                "aggression".to_string(),
            ],
            TrainingType::Goalkeeping => {
                if self.position.is_goalkeeper() {
                    // GK-specific: OpenFootball original doesn't have GK attributes, use adapted base attributes
                    vec![
                        "first_touch".to_string(),
                        "concentration".to_string(),
                        "positioning".to_string(),
                        "anticipation".to_string(),
                        "composure".to_string(),
                        "long_throws".to_string(),
                    ]
                } else {
                    vec![] // Non-GK players don't benefit from GK training
                }
            }
            TrainingType::General => {
                // General training affects attributes relevant to player's position
                self.position.get_primary_attributes().iter().map(|s| s.to_string()).collect()
            }
        }
    }

    /// Get training multiplier from growth profile
    fn get_training_multiplier(&self, training_type: TrainingType) -> f32 {
        match training_type {
            TrainingType::Technical => self.growth_profile.training_response.technical_multiplier,
            TrainingType::Physical => self.growth_profile.training_response.physical_multiplier,
            TrainingType::Mental => self.growth_profile.training_response.mental_multiplier,
            TrainingType::Shooting => self.growth_profile.training_response.technical_multiplier,
            TrainingType::Passing => self.growth_profile.training_response.technical_multiplier,
            TrainingType::Defending => self.growth_profile.training_response.mental_multiplier,
            TrainingType::Goalkeeping => self.growth_profile.training_response.technical_multiplier,
            TrainingType::General => {
                // Average of all multipliers
                let sum = self.growth_profile.training_response.technical_multiplier
                    + self.growth_profile.training_response.physical_multiplier
                    + self.growth_profile.training_response.mental_multiplier;
                sum / 3.0
            }
        }
    }

    /// Simulate player evolution over months of training
    pub fn evolve_over_time(
        &mut self,
        months: f32,
        training_schedule: &[(TrainingType, f32)],
        seed: u64,
    ) -> Vec<MonthlyGrowth> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut monthly_results: Vec<MonthlyGrowth> = Vec::new();

        let sessions_per_month = 8.0; // ~2 sessions per week
        let _total_sessions = (months * sessions_per_month) as u32;

        for month in 0..months.ceil() as u32 {
            let mut month_growths = Vec::new();

            // Age the player
            self.age_months += 1.0 / 12.0; // Age by 1 month

            let sessions_this_month = if month == months.ceil() as u32 - 1 {
                // Last month might be partial
                ((months.fract() * sessions_per_month) as u32).max(1)
            } else {
                sessions_per_month as u32
            };

            for _ in 0..sessions_this_month {
                // Pick training type based on schedule
                let (training_type, intensity) =
                    self.pick_training_from_schedule(training_schedule, &mut rng);

                // Apply some random variation to intensity (±10%)
                let varied_intensity = intensity * rng.gen_range(0.9..=1.1);

                let growths = self.apply_growth(training_type, varied_intensity, rng.next_u64());
                month_growths.extend(growths);
            }

            monthly_results.push(MonthlyGrowth {
                month: month + 1,
                age_months: self.age_months,
                ca_start: if month == 0 { self.ca } else { monthly_results.last().unwrap().ca_end },
                ca_end: self.ca,
                attribute_growths: month_growths,
                growth_rate: GrowthCalculator::calculate_final_growth_rate(
                    self.ca,
                    self.pa,
                    self.age_months,
                ),
            });
        }

        monthly_results
    }

    /// Pick training type from schedule with weighted randomness
    fn pick_training_from_schedule(
        &self,
        schedule: &[(TrainingType, f32)],
        rng: &mut ChaCha8Rng,
    ) -> (TrainingType, f32) {
        if schedule.is_empty() {
            return (TrainingType::General, 0.8); // Default training
        }

        let total_weight: f32 = schedule.iter().map(|(_, weight)| weight).sum();
        let random_value = rng.gen_range(0.0..total_weight);

        let mut current_weight = 0.0;
        for &(training_type, weight) in schedule {
            current_weight += weight;
            if random_value <= current_weight {
                return (training_type, 1.0); // Full intensity for selected training
            }
        }

        // Fallback (should never happen)
        schedule[0]
    }

    /// Get current growth potential (how much the player can still grow)
    pub fn get_growth_potential(&self) -> GrowthPotential {
        let current_growth_rate =
            GrowthCalculator::calculate_final_growth_rate(self.ca, self.pa, self.age_months);
        let sessions_to_potential =
            GrowthCalculator::estimate_sessions_to_potential(self.ca, self.pa, self.age_months);

        GrowthPotential {
            current_growth_rate,
            sessions_to_potential,
            ca_to_pa_ratio: (self.ca as f32) / (self.pa as f32),
            age_modifier: GrowthCalculator::age_modifier(self.age_months),
            years_of_development_left: if self.age_months < 18.0 {
                18.0 - self.age_months
            } else {
                0.0
            },
        }
    }

    /// Apply injury that might affect growth temporarily
    pub fn apply_injury(&mut self, severity: InjurySeverity, seed: u64) -> InjuryEffect {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let injury_chance = self.growth_profile.injury_prone;
        let injury_occurred = rng.gen::<f32>() < injury_chance;

        if !injury_occurred {
            return InjuryEffect {
                occurred: false,
                severity,
                affected_attributes: Vec::new(),
                recovery_time_months: 0.0,
                growth_penalty: 0.0,
            };
        }

        let (affected_attributes, recovery_time, growth_penalty) = match severity {
            InjurySeverity::Minor => {
                // Minor injuries affect 1-2 physical attributes slightly
                let attrs = vec!["stamina", "natural_fitness"];
                let selected: Vec<String> =
                    attrs.into_iter().take(rng.gen_range(1..=2)).map(|s| s.to_string()).collect();

                // Small temporary penalty
                for attr in &selected {
                    let _ = self.modify_attribute(attr, rng.gen_range(-2..=-1));
                }

                (selected, rng.gen_range(0.5..=1.5), 0.1)
            }
            InjurySeverity::Moderate => {
                // Moderate injuries affect 2-3 attributes more significantly
                let attrs = vec!["speed", "acceleration", "strength", "stamina", "agility"];
                let selected: Vec<String> =
                    attrs.into_iter().take(rng.gen_range(2..=3)).map(|s| s.to_string()).collect();

                for attr in &selected {
                    let _ = self.modify_attribute(attr, rng.gen_range(-4..=-2));
                }

                (selected, rng.gen_range(1.0..=3.0), 0.3)
            }
            InjurySeverity::Severe => {
                // Severe injuries can affect many attributes and have lasting impact
                let attrs = vec![
                    "speed",
                    "acceleration",
                    "strength",
                    "stamina",
                    "agility",
                    "balance",
                    "jumping",
                ];
                let selected: Vec<String> =
                    attrs.into_iter().take(rng.gen_range(3..=5)).map(|s| s.to_string()).collect();

                for attr in &selected {
                    let _ = self.modify_attribute(attr, rng.gen_range(-8..=-3));
                }

                (selected, rng.gen_range(2.0..=6.0), 0.5)
            }
        };

        InjuryEffect {
            occurred: true,
            severity,
            affected_attributes,
            recovery_time_months: recovery_time,
            growth_penalty,
        }
    }

    // ========== Player Instructions Methods ==========

    /// Set player instructions from a role preset
    /// Role preset automatically sets all 8 instruction values
    pub fn set_role(&mut self, role: PlayerRole) {
        self.instructions = Some(role.default_instructions());
        self.touch();
    }

    /// Set custom player instructions
    /// Allows manual control of all 8 instruction parameters
    pub fn set_instructions(&mut self, instructions: PlayerInstructions) {
        self.instructions = Some(instructions);
        self.touch();
    }

    /// Get current player instructions
    pub fn get_instructions(&self) -> Option<&PlayerInstructions> {
        self.instructions.as_ref()
    }

    /// Clear player instructions (revert to default behavior)
    pub fn clear_instructions(&mut self) {
        self.instructions = None;
        self.touch();
    }

    /// Get modified attributes with instructions applied
    /// Returns base attributes if no instructions are set
    /// Use this when simulating matches to get the effective player attributes
    pub fn get_modified_attributes(&self) -> PlayerAttributes {
        match &self.instructions {
            Some(instructions) => {
                use crate::player::instructions::apply_instructions_modifiers;
                apply_instructions_modifiers(&self.detailed_stats, instructions)
            }
            None => self.detailed_stats.clone(),
        }
    }

    /// Check if player has instructions set
    pub fn has_instructions(&self) -> bool {
        self.instructions.is_some()
    }

    // ===== Career Stats Methods =====

    /// 경기 출전 기록
    pub fn record_game_played(&mut self, goals: u32, assists: u32, clean_sheet: bool, rating: f32) {
        self.career_stats.record_game(goals, assists, clean_sheet, rating);
        self.touch();
    }

    /// 트레이닝 세션 기록
    pub fn record_training_session(&mut self) {
        self.career_stats.record_training_session();
        self.touch();
    }

    /// 주장 설정
    pub fn set_captain(&mut self, is_captain: bool) {
        self.career_stats.is_team_captain = is_captain;
        self.touch();
    }

    /// 국가대표 설정
    pub fn set_national_team_player(&mut self, is_national: bool) {
        self.career_stats.is_national_team_player = is_national;
        self.touch();
    }

    /// 타이틀 추가
    pub fn add_title(&mut self) {
        self.career_stats.major_titles = self.career_stats.major_titles.saturating_add(1);
        self.touch();
    }

    /// 관계 점수 업데이트
    pub fn update_relationship_score(&mut self, score: f32) {
        self.career_stats.relationship_score = score.clamp(0.0, 1.0);
        self.touch();
    }

    /// 새 시즌 시작
    pub fn start_new_season(&mut self) {
        self.career_stats.start_new_season();
        self.touch();
    }

    /// 경력 통계 조회
    pub fn get_career_stats(&self) -> &PlayerCareerStats {
        &self.career_stats
    }

    // ===== Injury Methods =====

    /// Check if player is currently injured
    pub fn is_injured(&self) -> bool {
        self.current_injury.is_some()
    }

    /// Check if player can train (not injured or minor injury)
    pub fn can_train(&self) -> bool {
        match &self.current_injury {
            None => true,
            Some(injury) => injury.severity == crate::training::InjurySeverity::Minor,
        }
    }

    /// Check if player can play match (not injured)
    pub fn can_play_match(&self) -> bool {
        self.current_injury.is_none()
    }

    /// Advance recovery by one day, returns recovered injury if healed
    pub fn advance_recovery(&mut self) -> Option<crate::training::Injury> {
        if let Some(injury) = &mut self.current_injury {
            if injury.advance_recovery() {
                // Fully recovered - move to history
                let recovered = self.current_injury.take().unwrap();
                self.injury_history.push(recovered.clone());
                self.touch();
                return Some(recovered);
            }
        }
        None
    }

    /// Set current injury (from training system)
    pub fn set_current_injury(&mut self, injury: crate::training::Injury) {
        self.current_injury = Some(injury);
        self.touch();
    }
}

/// 6-sided hexagon statistics for visual representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HexagonStats {
    pub pace: u8,      // 0-20 scale
    pub power: u8,     // 0-20 scale
    pub technical: u8, // 0-20 scale
    pub shooting: u8,  // 0-20 scale
    pub passing: u8,   // 0-20 scale
    pub defending: u8, // 0-20 scale
}

impl HexagonStats {
    /// Create new HexagonStats with all values set to the given value
    pub fn new(value: u8) -> Self {
        let capped_value = value.min(20);
        Self {
            pace: capped_value,
            power: capped_value,
            technical: capped_value,
            shooting: capped_value,
            passing: capped_value,
            defending: capped_value,
        }
    }

    /// Calculate hexagon stats from detailed attributes
    pub fn calculate_from_detailed(stats: &PlayerAttributes, position: Position) -> Self {
        if position.is_goalkeeper() {
            let (pace, power, technical, shooting, passing, defending) =
                stats.calculate_gk_hexagon();
            Self { pace, power, technical, shooting, passing, defending }
        } else {
            Self {
                pace: stats.calculate_pace(),
                power: stats.calculate_power(),
                technical: stats.calculate_technical(),
                shooting: stats.calculate_shooting(),
                passing: stats.calculate_passing(),
                defending: stats.calculate_defending(),
            }
        }
    }

    /// Calculate total hexagon score (sum of all 6 attributes)
    pub fn total(&self) -> u16 {
        self.pace as u16
            + self.power as u16
            + self.technical as u16
            + self.shooting as u16
            + self.passing as u16
            + self.defending as u16
    }

    /// Get all values as an array for easy iteration
    pub fn as_array(&self) -> [u8; 6] {
        [self.pace, self.power, self.technical, self.shooting, self.passing, self.defending]
    }
}

impl Default for HexagonStats {
    fn default() -> Self {
        Self::new(10) // Default to 10 (average)
    }
}

/// Growth profile defining how a player develops over time
///
/// Fields ordered by size to minimize padding:
/// - Vec<String> (largest - heap allocated)
/// - TrainingResponse struct
/// - f32 primitives
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GrowthProfile {
    pub specialization: Vec<String>, // ["SHOOTING", "PACE"] - focus areas (24 bytes + heap)
    pub training_response: TrainingResponse, // 12 bytes (3 * f32)
    pub growth_rate: f32,            // 4 bytes, 0.0-1.0 current growth speed
    pub injury_prone: f32,           // 4 bytes, 0.0-1.0 injury vulnerability
}

impl GrowthProfile {
    /// Create a new growth profile with default values
    pub fn new() -> Self {
        Self {
            growth_rate: 1.0,
            specialization: Vec::new(),
            training_response: TrainingResponse::default(),
            injury_prone: 0.1, // Low injury risk by default
        }
    }

    /// Create a specialized growth profile
    pub fn specialized(specializations: Vec<String>) -> Self {
        let mut profile = Self::new();
        profile.specialization = specializations;
        profile
    }
}

impl Default for GrowthProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Training response multipliers for different training types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrainingResponse {
    pub technical_multiplier: f32, // 0.5-2.0
    pub physical_multiplier: f32,  // 0.5-2.0
    pub mental_multiplier: f32,    // 0.5-2.0
}

impl TrainingResponse {
    /// Create balanced training response (all 1.0x)
    pub fn balanced() -> Self {
        Self { technical_multiplier: 1.0, physical_multiplier: 1.0, mental_multiplier: 1.0 }
    }

    /// Create technical-focused response (1.5x technical)
    pub fn technical_focused() -> Self {
        Self { technical_multiplier: 1.5, physical_multiplier: 0.8, mental_multiplier: 1.0 }
    }

    /// Create physical-focused response (1.5x physical)
    pub fn physical_focused() -> Self {
        Self { technical_multiplier: 0.8, physical_multiplier: 1.5, mental_multiplier: 1.0 }
    }

    /// Create mental-focused response (1.5x mental)
    pub fn mental_focused() -> Self {
        Self { technical_multiplier: 1.0, physical_multiplier: 0.8, mental_multiplier: 1.5 }
    }

    /// Validate that all multipliers are in valid range (0.5-2.0)
    pub fn is_valid(&self) -> bool {
        let in_range = |val: f32| (0.5..=2.0).contains(&val);
        in_range(self.technical_multiplier)
            && in_range(self.physical_multiplier)
            && in_range(self.mental_multiplier)
    }
}

impl Default for TrainingResponse {
    fn default() -> Self {
        Self::balanced()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexagon_stats_new() {
        let hexagon = HexagonStats::new(15);
        assert_eq!(hexagon.pace, 15);
        assert_eq!(hexagon.power, 15);
        assert_eq!(hexagon.total(), 90); // 15 * 6
    }

    #[test]
    fn test_hexagon_stats_capped() {
        let hexagon = HexagonStats::new(25); // Should be capped at 20
        assert_eq!(hexagon.pace, 20);
        assert_eq!(hexagon.total(), 120); // 20 * 6
    }

    #[test]
    fn test_training_response_validation() {
        assert!(TrainingResponse::balanced().is_valid());
        assert!(TrainingResponse::technical_focused().is_valid());

        let invalid = TrainingResponse {
            technical_multiplier: 3.0, // Too high
            physical_multiplier: 1.0,
            mental_multiplier: 1.0,
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_core_player_creation() {
        let stats = PlayerAttributes::default();
        let growth = GrowthProfile::new();
        let personality = PersonAttributes::generate_random(12345);
        let player = CorePlayer::new(
            "Test Player".to_string(),
            Position::FW,
            15.5,
            60,
            120,
            stats,
            growth,
            personality,
        );

        assert_eq!(player.name, "Test Player");
        assert_eq!(player.position, Position::FW);
        assert_eq!(player.ca, 60);
        assert_eq!(player.pa, 120);
        assert!(!player.id.is_empty());
        assert_eq!(player.special_abilities.abilities.len(), 0); // New players start with no special abilities
    }

    /// Migration test: Verify that old save files (v1.0 schema) can be loaded
    /// Old saves don't have: special_abilities, career_stats, instructions,
    /// current_injury, injury_history, injury_proneness
    #[test]
    fn test_migration_from_v1_0_schema() {
        // Simulate v1.0 JSON (missing all v1.1+ fields)
        let v1_0_json = r#"{
            "detailed_stats": {
                "corners": 50, "crossing": 50, "dribbling": 50, "finishing": 50,
                "first_touch": 50, "free_kicks": 50, "heading": 50, "long_shots": 50,
                "long_throws": 50, "marking": 50, "passing": 50, "penalty_taking": 50,
                "tackling": 50, "technique": 50,
                "aggression": 50, "anticipation": 50, "bravery": 50, "composure": 50,
                "concentration": 50, "decisions": 50, "determination": 50, "flair": 50,
                "leadership": 50, "off_the_ball": 50, "positioning": 50, "teamwork": 50,
                "vision": 50, "work_rate": 50,
                "acceleration": 50, "agility": 50, "balance": 50, "jumping": 50,
                "natural_fitness": 50, "pace": 50, "stamina": 50, "strength": 50
            },
            "growth_profile": {
                "specialization": [],
                "training_response": {"technical_multiplier": 1.0, "physical_multiplier": 1.0, "mental_multiplier": 1.0},
                "growth_rate": 1.0,
                "injury_prone": 0.1
            },
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z",
            "hexagon_stats": {"pace": 10, "power": 10, "technical": 10, "shooting": 10, "passing": 10, "defending": 10},
            "personality": {"adaptability": 50, "ambition": 50, "determination": 50, "discipline": 50, "loyalty": 50, "pressure": 50, "professionalism": 50, "temperament": 50},
            "id": "test-uuid-12345",
            "name": "Old Save Player",
            "age_months": 16.0,
            "position": "FW",
            "ca": 60,
            "pa": 120
        }"#;

        // Should deserialize without error, with default values for missing fields
        let player: CorePlayer = serde_json::from_str(v1_0_json)
            .expect("v1.0 schema should deserialize with serde(default)");

        // Verify original fields are intact
        assert_eq!(player.name, "Old Save Player");
        assert_eq!(player.ca, 60);
        assert_eq!(player.pa, 120);
        assert_eq!(player.position, Position::FW);

        // Verify new fields got their defaults
        assert!(player.special_abilities.abilities.is_empty(), "special_abilities should default to empty");
        assert_eq!(player.career_stats.games_played, 0, "career_stats should default");
        assert!(player.instructions.is_none(), "instructions should default to None");
        assert!(player.current_injury.is_none(), "current_injury should default to None");
        assert!(player.injury_history.is_empty(), "injury_history should default to empty");
        assert!((player.injury_proneness - 0.1).abs() < 0.001, "injury_proneness should default to 0.1");
    }

    /// Migration test: Verify roundtrip serialization preserves all fields
    #[test]
    fn test_migration_roundtrip() {
        let stats = PlayerAttributes::default();
        let growth = GrowthProfile::new();
        let personality = PersonAttributes::generate_random(12345);
        let mut player = CorePlayer::new(
            "Roundtrip Player".to_string(),
            Position::CM,
            17.0,
            80,
            140,
            stats,
            growth,
            personality,
        );

        // Add some v1.1+ data
        player.career_stats.games_played = 50;
        player.injury_proneness = 0.25;

        // Serialize
        let json = serde_json::to_string(&player).expect("serialize should work");

        // Deserialize
        let loaded: CorePlayer = serde_json::from_str(&json).expect("deserialize should work");

        // Verify all data preserved
        assert_eq!(loaded.name, "Roundtrip Player");
        assert_eq!(loaded.career_stats.games_played, 50);
        assert!((loaded.injury_proneness - 0.25).abs() < 0.001);
    }
}
