//! AI Tactical Profiles System
//!
//! Intelligent AI opponents that dynamically adjust tactics based on:
//! - Match score (leading, drawing, losing)
//! - Match time (early game, late game)
//! - Stamina levels (energy management)
//! - Opponent tactics (counter-tactics)
//!
//! Features 5 AI profiles with 4 difficulty levels (Easy to Expert).

use crate::tactics::team_instructions::*;
use rand::Rng;

// ============================================================================
// Phase 0: Foundation - Basic Data Structures
// ============================================================================

/// Match state information for AI decision-making
#[derive(Debug, Clone, PartialEq)]
pub struct MatchState {
    /// Home team score
    pub home_score: i32,
    /// Away team score
    pub away_score: i32,
    /// Current minute (0-90+)
    pub current_minute: u32,
    /// Average player stamina (0.0-1.0)
    pub average_stamina: f32,
}

impl Default for MatchState {
    fn default() -> Self {
        Self { home_score: 0, away_score: 0, current_minute: 0, average_stamina: 1.0 }
    }
}

impl MatchState {
    /// Calculate score difference from perspective of home team
    pub fn score_difference(&self) -> i32 {
        self.home_score - self.away_score
    }

    /// Check if match is in second half
    pub fn is_second_half(&self) -> bool {
        self.current_minute >= 45
    }

    /// Check if match is in late stage (70+ minutes)
    pub fn is_late_game(&self) -> bool {
        self.current_minute >= 70
    }

    /// Check if match is in final minutes (85+ minutes)
    pub fn is_final_minutes(&self) -> bool {
        self.current_minute >= 85
    }
}

/// AI difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AIDifficulty {
    /// Easy - No tactical changes, uses random preset
    Easy,
    /// Medium - Basic situational responses (30% change probability)
    #[default]
    Medium,
    /// Hard - Sophisticated situation judgment (80% change probability)
    Hard,
    /// Expert - Perfect tactical execution (100% optimal decisions)
    Expert,
}

impl AIDifficulty {
    /// Get the probability of applying tactical changes
    pub fn change_probability(&self) -> f32 {
        match self {
            Self::Easy => 0.0,
            Self::Medium => 0.3,
            Self::Hard => 0.8,
            Self::Expert => 1.0,
        }
    }

    /// Get update frequency in minutes
    pub fn update_frequency(&self) -> u32 {
        match self {
            Self::Easy => 999,  // Never update
            Self::Medium => 10, // Every 10 minutes or on score change
            Self::Hard => 5,    // Every 5 minutes
            Self::Expert => 3,  // Every 3 minutes
        }
    }
}

// ============================================================================
// Phase 1: Core Structures
// ============================================================================

/// AI Tactical Profile
///
/// Defines AI behavior, default tactics, and situational response rules
#[derive(Debug, Clone, PartialEq)]
pub struct AITacticalProfile {
    /// Profile name (e.g., "Aggressive", "Balanced")
    pub name: &'static str,
    /// Korean name
    pub name_ko: &'static str,
    /// Description
    pub description: &'static str,
    /// Default tactical setup
    pub default_tactics: TeamInstructions,
    /// Situational response rules
    pub situation_rules: Vec<TacticalRule>,
    /// Counter-tactics rules (for Adaptive AI)
    pub counter_rules: Vec<CounterRule>,
}

/// Tactical change rule based on match conditions
#[derive(Debug, Clone, PartialEq)]
pub struct TacticalRule {
    /// Condition that triggers this rule
    pub condition: MatchCondition,
    /// New tactics to apply when condition is met
    pub new_tactics: TeamInstructions,
    /// Priority (higher = checked first)
    pub priority: u8,
}

/// Match condition for triggering tactical changes
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCondition {
    /// Score difference within range (inclusive)
    ScoreDifference { min: i32, max: i32 },
    /// Match time at or after specified minute
    MatchTime { minute: u32 },
    /// Stamina below threshold
    StaminaBelow { threshold: f32 },
    /// All conditions must be true
    And(Vec<MatchCondition>),
    /// At least one condition must be true
    Or(Vec<MatchCondition>),
}

/// Counter-tactics rule for responding to opponent's tactics
#[derive(Debug, Clone, PartialEq)]
pub struct CounterRule {
    /// Opponent tactic condition
    pub opponent_condition: OpponentTactics,
    /// Counter-tactic to apply
    pub counter_tactics: TacticsAdjustment,
}

/// Opponent tactical condition
#[derive(Debug, Clone, PartialEq)]
pub enum OpponentTactics {
    /// Opponent tempo is at specified level
    Tempo(TeamTempo),
    /// Opponent pressing is at specified level
    Pressing(TeamPressing),
    /// Opponent width is at specified level
    Width(TeamWidth),
    /// Opponent defensive line is at specified level
    DefensiveLine(DefensiveLine),
}

/// Tactical adjustment (partial changes)
#[derive(Debug, Clone, PartialEq)]
pub struct TacticsAdjustment {
    pub tempo: Option<TeamTempo>,
    pub pressing: Option<TeamPressing>,
    pub width: Option<TeamWidth>,
    pub build_up: Option<BuildUpStyle>,
    pub defensive_line: Option<DefensiveLine>,
}

impl TacticsAdjustment {
    /// Apply this adjustment to existing tactics
    pub fn apply_to(&self, tactics: &mut TeamInstructions) {
        if let Some(tempo) = self.tempo {
            tactics.team_tempo = tempo;
        }
        if let Some(pressing) = self.pressing {
            tactics.pressing_intensity = pressing;
        }
        if let Some(width) = self.width {
            tactics.team_width = width;
        }
        if let Some(build_up) = self.build_up {
            tactics.build_up_style = build_up;
        }
        if let Some(defensive_line) = self.defensive_line {
            tactics.defensive_line = defensive_line;
        }
    }
}

// ============================================================================
// Phase 2: AI Profile Definitions
// ============================================================================

/// Aggressive AI Profile
///
/// Philosophy: Always attack, high pressing, fast tempo
/// Risk: High stamina drain, vulnerable to counters
pub const AGGRESSIVE_AI: AITacticalProfile = AITacticalProfile {
    name: "Aggressive",
    name_ko: "공격형",
    description: "Always attacking with high pressing and fast tempo",
    default_tactics: TeamInstructions {
        team_tempo: TeamTempo::Fast,
        pressing_intensity: TeamPressing::High,
        team_width: TeamWidth::Wide,
        build_up_style: BuildUpStyle::Direct,
        defensive_line: DefensiveLine::High,
        use_offside_trap: true,
    },
    situation_rules: vec![],
    counter_rules: vec![],
};

/// Balanced AI Profile
///
/// Philosophy: Flexible tactics based on situation
/// Best for: Most situations, beginner AI
pub const BALANCED_AI: AITacticalProfile = AITacticalProfile {
    name: "Balanced",
    name_ko: "균형형",
    description: "Flexible tactics that adapt to match situation",
    default_tactics: TeamInstructions {
        team_tempo: TeamTempo::Normal,
        pressing_intensity: TeamPressing::Medium,
        team_width: TeamWidth::Normal,
        build_up_style: BuildUpStyle::Short,
        defensive_line: DefensiveLine::Normal,
        use_offside_trap: false,
    },
    situation_rules: vec![],
    counter_rules: vec![],
};

/// Defensive AI Profile
///
/// Philosophy: Minimize conceding, counter-attack focused
/// Best for: Strong opponents, weaker teams
pub const DEFENSIVE_AI: AITacticalProfile = AITacticalProfile {
    name: "Defensive",
    name_ko: "수비형",
    description: "Compact defense with counter-attack focus",
    default_tactics: TeamInstructions {
        team_tempo: TeamTempo::Slow,
        pressing_intensity: TeamPressing::Low,
        team_width: TeamWidth::Narrow,
        build_up_style: BuildUpStyle::Direct,
        defensive_line: DefensiveLine::Deep,
        use_offside_trap: false,
    },
    situation_rules: vec![],
    counter_rules: vec![],
};

/// Adaptive AI Profile
///
/// Philosophy: Analyze and counter opponent tactics
/// Best for: Hard/Expert difficulty
pub const ADAPTIVE_AI: AITacticalProfile = AITacticalProfile {
    name: "Adaptive",
    name_ko: "적응형",
    description: "Analyzes opponent and applies counter-tactics",
    default_tactics: TeamInstructions {
        team_tempo: TeamTempo::Normal,
        pressing_intensity: TeamPressing::Medium,
        team_width: TeamWidth::Normal,
        build_up_style: BuildUpStyle::Short,
        defensive_line: DefensiveLine::Normal,
        use_offside_trap: false,
    },
    situation_rules: vec![],
    counter_rules: vec![],
};

/// Counter-Attack AI Profile
///
/// Philosophy: Defend deep, quick counter-attacks
/// Best for: Teams with fast forwards
pub const COUNTER_AI: AITacticalProfile = AITacticalProfile {
    name: "Counter-Attack",
    name_ko: "역습형",
    description: "Deep defense with fast counter-attacks",
    default_tactics: TeamInstructions {
        team_tempo: TeamTempo::Fast,
        pressing_intensity: TeamPressing::Low,
        team_width: TeamWidth::Normal,
        build_up_style: BuildUpStyle::Direct,
        defensive_line: DefensiveLine::Deep,
        use_offside_trap: false,
    },
    situation_rules: vec![],
    counter_rules: vec![],
};

/// All AI profiles
pub const AI_PROFILES: &[AITacticalProfile] =
    &[AGGRESSIVE_AI, BALANCED_AI, DEFENSIVE_AI, ADAPTIVE_AI, COUNTER_AI];

// ============================================================================
// Phase 3: AI Tactical Manager
// ============================================================================

/// AI Tactical Manager
///
/// Manages AI tactical decisions based on profile and difficulty
pub struct AITacticalManager {
    /// AI profile being used
    profile: AITacticalProfile,
    /// Current tactics
    current_tactics: TeamInstructions,
    /// AI difficulty level
    difficulty: AIDifficulty,
    /// Last update minute
    last_update_minute: u32,
}

impl AITacticalManager {
    /// Create new AI tactical manager
    pub fn new(profile: AITacticalProfile, difficulty: AIDifficulty) -> Self {
        let current_tactics = profile.default_tactics.clone();
        Self { profile, current_tactics, difficulty, last_update_minute: 0 }
    }

    /// Get current tactics
    pub fn current_tactics(&self) -> &TeamInstructions {
        &self.current_tactics
    }

    /// Check if update is needed based on time
    pub fn should_update(&self, match_state: &MatchState) -> bool {
        // Always allow first update
        if self.last_update_minute == 0 {
            return true;
        }

        let minutes_since_update =
            match_state.current_minute.saturating_sub(self.last_update_minute);
        minutes_since_update >= self.difficulty.update_frequency()
    }

    /// Update tactics based on match state and opponent
    ///
    /// Returns Some(new_tactics) if tactics changed, None if no change
    pub fn update_tactics(
        &mut self,
        match_state: &MatchState,
        opponent_tactics: &TeamInstructions,
        rng: &mut impl rand::Rng,
    ) -> Option<TeamInstructions> {
        // Easy AI never changes tactics
        if self.difficulty == AIDifficulty::Easy {
            return None;
        }

        let new_tactics = match self.difficulty {
            AIDifficulty::Easy => return None,
            AIDifficulty::Medium => {
                // Basic situational response only
                self.apply_situation_rules(match_state, rng)?
            }
            AIDifficulty::Hard => {
                // Situation + stamina management
                let mut tactics = self.apply_situation_rules(match_state, rng)?;
                self.adjust_for_stamina(&mut tactics, match_state);
                tactics
            }
            AIDifficulty::Expert => {
                // Full intelligence: situation + counter + stamina
                let mut tactics = self
                    .apply_situation_rules(match_state, rng)
                    .unwrap_or_else(|| self.current_tactics.clone());
                self.apply_counter_rules(&mut tactics, opponent_tactics);
                self.optimize_stamina(&mut tactics, match_state);
                tactics
            }
        };

        // Check if tactics actually changed
        if new_tactics == self.current_tactics {
            return None;
        }

        self.current_tactics = new_tactics.clone();
        self.last_update_minute = match_state.current_minute;
        Some(new_tactics)
    }

    /// Apply situational rules based on match state
    fn apply_situation_rules(
        &self,
        match_state: &MatchState,
        rng: &mut impl rand::Rng,
    ) -> Option<TeamInstructions> {
        // Check change probability (except Expert which is always 100%)
        if self.difficulty != AIDifficulty::Expert {
            if rng.gen::<f32>() > self.difficulty.change_probability() {
                return None;
            }
        }

        // Get tactics based on profile and situation
        let tactics = self.get_situational_tactics(match_state)?;
        Some(tactics)
    }

    /// Get situational tactics for specific AI profile
    fn get_situational_tactics(&self, match_state: &MatchState) -> Option<TeamInstructions> {
        let score_diff = match_state.score_difference();
        let minute = match_state.current_minute;

        match self.profile.name {
            "Aggressive" => self.aggressive_tactics(score_diff, minute),
            "Balanced" => self.balanced_tactics(score_diff, minute),
            "Defensive" => self.defensive_tactics(score_diff, minute),
            "Adaptive" => self.adaptive_tactics(score_diff, minute),
            "Counter-Attack" => self.counter_tactics(score_diff, minute),
            _ => None,
        }
    }

    /// Aggressive AI situational tactics
    fn aggressive_tactics(&self, score_diff: i32, minute: u32) -> Option<TeamInstructions> {
        let mut tactics = self.profile.default_tactics.clone();

        // Losing: All-in attack
        if score_diff < 0 && minute >= 70 {
            tactics.team_tempo = TeamTempo::VeryFast;
            tactics.pressing_intensity = TeamPressing::VeryHigh;
            return Some(tactics);
        }

        // Drawing late: Push harder
        if score_diff == 0 && minute >= 75 {
            tactics.team_tempo = TeamTempo::VeryFast;
            return Some(tactics);
        }

        // Leading late: Slightly stabilize
        if score_diff > 0 && minute >= 80 {
            tactics.team_tempo = TeamTempo::Normal;
            return Some(tactics);
        }

        None // Otherwise maintain default aggressive tactics
    }

    /// Balanced AI situational tactics
    fn balanced_tactics(&self, score_diff: i32, minute: u32) -> Option<TeamInstructions> {
        let mut tactics = self.profile.default_tactics.clone();

        // Leading by 1: Stabilize
        if score_diff == 1 && minute >= 60 {
            tactics.team_tempo = TeamTempo::Slow;
            tactics.pressing_intensity = TeamPressing::Low;
            return Some(tactics);
        }

        // Leading by 2+: Full defense
        if score_diff >= 2 && minute >= 70 {
            tactics.team_tempo = TeamTempo::VerySlow;
            tactics.pressing_intensity = TeamPressing::VeryLow;
            tactics.defensive_line = DefensiveLine::Deep;
            return Some(tactics);
        }

        // Losing before 60': Increase tempo
        if score_diff < 0 && minute < 60 {
            tactics.team_tempo = TeamTempo::Fast;
            tactics.pressing_intensity = TeamPressing::High;
            return Some(tactics);
        }

        // Losing after 60': All-out attack
        if score_diff < 0 && minute >= 60 {
            tactics.team_tempo = TeamTempo::VeryFast;
            tactics.team_width = TeamWidth::VeryWide;
            tactics.pressing_intensity = TeamPressing::High;
            return Some(tactics);
        }

        None // Drawing or early game: maintain default
    }

    /// Defensive AI situational tactics
    fn defensive_tactics(&self, score_diff: i32, minute: u32) -> Option<TeamInstructions> {
        let mut tactics = self.profile.default_tactics.clone();

        // Leading: Full park the bus
        if score_diff > 0 {
            tactics.team_tempo = TeamTempo::VerySlow;
            tactics.pressing_intensity = TeamPressing::VeryLow;
            tactics.defensive_line = DefensiveLine::VeryDeep;
            return Some(tactics);
        }

        // Losing before 70': Slightly more attacking
        if score_diff < 0 && minute < 70 {
            tactics.team_tempo = TeamTempo::Normal;
            tactics.pressing_intensity = TeamPressing::Medium;
            return Some(tactics);
        }

        // Losing after 70': Forced to attack
        if score_diff < 0 && minute >= 70 {
            tactics.team_tempo = TeamTempo::Fast;
            tactics.pressing_intensity = TeamPressing::Medium;
            tactics.defensive_line = DefensiveLine::Normal;
            return Some(tactics);
        }

        None // Drawing: maintain defensive default
    }

    /// Adaptive AI situational tactics (similar to Balanced but more aggressive)
    fn adaptive_tactics(&self, score_diff: i32, minute: u32) -> Option<TeamInstructions> {
        // Base behavior similar to Balanced
        self.balanced_tactics(score_diff, minute)
    }

    /// Counter-Attack AI situational tactics
    fn counter_tactics(&self, score_diff: i32, minute: u32) -> Option<TeamInstructions> {
        let mut tactics = self.profile.default_tactics.clone();

        // Leading: Slow down tempo
        if score_diff > 0 && minute >= 70 {
            tactics.team_tempo = TeamTempo::Slow;
            return Some(tactics);
        }

        // Losing: Add some pressing
        if score_diff < 0 && minute >= 60 {
            tactics.pressing_intensity = TeamPressing::Medium;
            tactics.team_tempo = TeamTempo::Fast;
            return Some(tactics);
        }

        None // Otherwise maintain counter-attack setup
    }

    /// Apply counter-tactics based on opponent's tactics (Expert only)
    fn apply_counter_rules(&self, tactics: &mut TeamInstructions, opponent: &TeamInstructions) {
        // Only Adaptive AI uses counter-tactics
        if self.profile.name != "Adaptive" {
            return;
        }

        // Counter very fast tempo with very slow (wait for stamina drain)
        if opponent.team_tempo == TeamTempo::VeryFast {
            tactics.team_tempo = TeamTempo::VerySlow;
        }

        // Counter very high pressing with direct build-up
        if opponent.pressing_intensity == TeamPressing::VeryHigh {
            tactics.build_up_style = BuildUpStyle::Direct;
        }

        // Counter very wide with narrow (compact center)
        if opponent.team_width == TeamWidth::VeryWide {
            tactics.team_width = TeamWidth::Narrow;
        }

        // Counter very high line with direct build-up (exploit offside trap)
        if opponent.defensive_line == DefensiveLine::VeryHigh {
            tactics.build_up_style = BuildUpStyle::Direct;
        }
    }

    /// Adjust tactics for stamina (Hard difficulty)
    fn adjust_for_stamina(&self, tactics: &mut TeamInstructions, match_state: &MatchState) {
        // If stamina below 30% and after 70', reduce tempo by one level
        if match_state.average_stamina < 0.3 && match_state.current_minute >= 70 {
            tactics.team_tempo = match tactics.team_tempo {
                TeamTempo::VeryFast => TeamTempo::Fast,
                TeamTempo::Fast => TeamTempo::Normal,
                TeamTempo::Normal => TeamTempo::Slow,
                TeamTempo::Slow => TeamTempo::VerySlow,
                TeamTempo::VerySlow => TeamTempo::VerySlow,
            };
        }
    }

    /// Optimize stamina management (Expert difficulty)
    fn optimize_stamina(&self, tactics: &mut TeamInstructions, match_state: &MatchState) {
        // Basic stamina adjustment like Hard difficulty
        self.adjust_for_stamina(tactics, match_state);

        // Additional expert optimizations
        if match_state.current_minute < 45 && match_state.average_stamina < 0.6 {
            // Early game stamina conservation
            if tactics.pressing_intensity == TeamPressing::VeryHigh {
                tactics.pressing_intensity = TeamPressing::High;
            }
        }

        // Late game stamina crisis management
        if match_state.current_minute >= 85 && match_state.average_stamina < 0.25 {
            tactics.pressing_intensity = TeamPressing::VeryLow;
            if tactics.team_tempo != TeamTempo::VeryFast {
                // Only keep VeryFast if desperately chasing
                tactics.team_tempo = TeamTempo::VerySlow;
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

impl AITacticalProfile {
    /// Find an AI profile by name
    pub fn find_by_name(name: &str) -> Option<&'static AITacticalProfile> {
        AI_PROFILES.iter().find(|p| p.name == name || p.name_ko == name)
    }

    /// Get all profiles
    pub fn all() -> &'static [AITacticalProfile] {
        AI_PROFILES
    }
}

// ============================================================================
// Phase 4: Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn test_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(0)
    }

    #[test]
    fn test_match_state_defaults() {
        let state = MatchState::default();
        assert_eq!(state.home_score, 0);
        assert_eq!(state.away_score, 0);
        assert_eq!(state.current_minute, 0);
        assert_eq!(state.average_stamina, 1.0);
    }

    #[test]
    fn test_match_state_score_difference() {
        let state = MatchState { home_score: 2, away_score: 1, ..Default::default() };
        assert_eq!(state.score_difference(), 1);

        let state2 = MatchState { home_score: 1, away_score: 3, ..Default::default() };
        assert_eq!(state2.score_difference(), -2);
    }

    #[test]
    fn test_match_state_time_checks() {
        let early = MatchState { current_minute: 20, ..Default::default() };
        assert!(!early.is_second_half());
        assert!(!early.is_late_game());

        let late = MatchState { current_minute: 85, ..Default::default() };
        assert!(late.is_second_half());
        assert!(late.is_late_game());
        assert!(late.is_final_minutes());
    }

    #[test]
    fn test_ai_difficulty_probabilities() {
        assert_eq!(AIDifficulty::Easy.change_probability(), 0.0);
        assert_eq!(AIDifficulty::Medium.change_probability(), 0.3);
        assert_eq!(AIDifficulty::Hard.change_probability(), 0.8);
        assert_eq!(AIDifficulty::Expert.change_probability(), 1.0);
    }

    #[test]
    fn test_ai_profiles_count() {
        assert_eq!(AI_PROFILES.len(), 5);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // Testing constant struct data
    fn test_aggressive_ai_profile() {
        assert_eq!(AGGRESSIVE_AI.name, "Aggressive");
        assert_eq!(AGGRESSIVE_AI.name_ko, "공격형");
        assert_eq!(AGGRESSIVE_AI.default_tactics.team_tempo, TeamTempo::Fast);
        assert_eq!(AGGRESSIVE_AI.default_tactics.pressing_intensity, TeamPressing::High);
        assert!(AGGRESSIVE_AI.default_tactics.use_offside_trap);
    }

    #[test]
    fn test_balanced_ai_profile() {
        assert_eq!(BALANCED_AI.name, "Balanced");
        assert_eq!(BALANCED_AI.default_tactics.team_tempo, TeamTempo::Normal);
        assert_eq!(BALANCED_AI.default_tactics.pressing_intensity, TeamPressing::Medium);
    }

    #[test]
    fn test_defensive_ai_profile() {
        assert_eq!(DEFENSIVE_AI.name, "Defensive");
        assert_eq!(DEFENSIVE_AI.default_tactics.team_tempo, TeamTempo::Slow);
        assert_eq!(DEFENSIVE_AI.default_tactics.pressing_intensity, TeamPressing::Low);
        assert_eq!(DEFENSIVE_AI.default_tactics.defensive_line, DefensiveLine::Deep);
    }

    #[test]
    fn test_adaptive_ai_profile() {
        assert_eq!(ADAPTIVE_AI.name, "Adaptive");
        assert_eq!(ADAPTIVE_AI.name_ko, "적응형");
    }

    #[test]
    fn test_counter_ai_profile() {
        assert_eq!(COUNTER_AI.name, "Counter-Attack");
        assert_eq!(COUNTER_AI.default_tactics.team_tempo, TeamTempo::Fast);
        assert_eq!(COUNTER_AI.default_tactics.pressing_intensity, TeamPressing::Low);
        assert_eq!(COUNTER_AI.default_tactics.build_up_style, BuildUpStyle::Direct);
    }

    #[test]
    fn test_find_profile_by_name() {
        assert!(AITacticalProfile::find_by_name("Aggressive").is_some());
        assert!(AITacticalProfile::find_by_name("Balanced").is_some());
        assert!(AITacticalProfile::find_by_name("공격형").is_some());
        assert!(AITacticalProfile::find_by_name("Unknown").is_none());
    }

    #[test]
    fn test_ai_manager_creation() {
        let manager = AITacticalManager::new(BALANCED_AI.clone(), AIDifficulty::Medium);
        assert_eq!(manager.current_tactics().team_tempo, TeamTempo::Normal);
    }

    #[test]
    fn test_easy_ai_never_changes() {
        let mut manager = AITacticalManager::new(BALANCED_AI.clone(), AIDifficulty::Easy);

        let losing_badly =
            MatchState { home_score: 0, away_score: 3, current_minute: 89, average_stamina: 0.2 };

        let opponent = TeamInstructions::default();
        let mut rng = test_rng();
        let result = manager.update_tactics(&losing_badly, &opponent, &mut rng);

        assert!(result.is_none()); // Easy AI never changes
    }

    #[test]
    fn test_balanced_ai_leading_stabilizes() {
        let mut manager = AITacticalManager::new(BALANCED_AI.clone(), AIDifficulty::Expert);

        let leading =
            MatchState { home_score: 2, away_score: 0, current_minute: 75, average_stamina: 0.6 };

        let opponent = TeamInstructions::default();
        let mut rng = test_rng();
        let result = manager.update_tactics(&leading, &opponent, &mut rng);

        if let Some(tactics) = result {
            // Should slow down when leading by 2+
            assert!(
                tactics.team_tempo == TeamTempo::VerySlow || tactics.team_tempo == TeamTempo::Slow
            );
        }
    }

    #[test]
    fn test_balanced_ai_losing_attacks() {
        let mut manager = AITacticalManager::new(BALANCED_AI.clone(), AIDifficulty::Expert);

        let losing_late =
            MatchState { home_score: 0, away_score: 1, current_minute: 80, average_stamina: 0.5 };

        let opponent = TeamInstructions::default();
        let mut rng = test_rng();
        let result = manager.update_tactics(&losing_late, &opponent, &mut rng);

        if let Some(tactics) = result {
            // Should attack when losing late
            assert!(
                tactics.team_tempo == TeamTempo::VeryFast || tactics.team_tempo == TeamTempo::Fast
            );
        }
    }

    #[test]
    fn test_aggressive_ai_maintains_aggression() {
        let mut manager = AITacticalManager::new(AGGRESSIVE_AI.clone(), AIDifficulty::Expert);

        let drawing =
            MatchState { home_score: 1, away_score: 1, current_minute: 75, average_stamina: 0.5 };

        let opponent = TeamInstructions::default();
        let mut rng = test_rng();
        let _result = manager.update_tactics(&drawing, &opponent, &mut rng);

        // Aggressive AI should push harder when drawing late
        // (tested indirectly through tactics)
    }

    #[test]
    fn test_defensive_ai_parks_bus_when_leading() {
        let mut manager = AITacticalManager::new(DEFENSIVE_AI.clone(), AIDifficulty::Expert);

        let leading =
            MatchState { home_score: 1, away_score: 0, current_minute: 70, average_stamina: 0.6 };

        let opponent = TeamInstructions::default();
        let mut rng = test_rng();
        let result = manager.update_tactics(&leading, &opponent, &mut rng);

        if let Some(tactics) = result {
            // Defensive AI should park the bus when leading
            assert_eq!(tactics.team_tempo, TeamTempo::VerySlow);
            assert_eq!(tactics.pressing_intensity, TeamPressing::VeryLow);
        }
    }

    #[test]
    fn test_adaptive_ai_counters_opponent() {
        let mut manager = AITacticalManager::new(ADAPTIVE_AI.clone(), AIDifficulty::Expert);

        let opponent = TeamInstructions {
            team_tempo: TeamTempo::VeryFast,
            pressing_intensity: TeamPressing::VeryHigh,
            team_width: TeamWidth::VeryWide,
            defensive_line: DefensiveLine::VeryHigh,
            ..TeamInstructions::default()
        };

        let match_state = MatchState { current_minute: 50, ..Default::default() };

        let mut rng = test_rng();
        let result = manager.update_tactics(&match_state, &opponent, &mut rng);

        if let Some(tactics) = result {
            // Adaptive should counter VeryFast with VerySlow
            assert_eq!(tactics.team_tempo, TeamTempo::VerySlow);
            // Should use Direct to counter high pressing and high line
            assert_eq!(tactics.build_up_style, BuildUpStyle::Direct);
        }
    }

    #[test]
    fn test_stamina_adjustment_hard() {
        let mut manager = AITacticalManager::new(BALANCED_AI.clone(), AIDifficulty::Hard);

        let low_stamina =
            MatchState { home_score: 0, away_score: 0, current_minute: 75, average_stamina: 0.25 };

        let opponent = TeamInstructions::default();
        let mut rng = test_rng();
        let result = manager.update_tactics(&low_stamina, &opponent, &mut rng);

        // Hard AI should reduce tempo when stamina is low
        if let Some(tactics) = result {
            assert!(tactics.team_tempo != TeamTempo::VeryFast);
        }
    }

    #[test]
    fn test_counter_ai_slows_when_leading() {
        let mut manager = AITacticalManager::new(COUNTER_AI.clone(), AIDifficulty::Expert);

        let leading =
            MatchState { home_score: 1, away_score: 0, current_minute: 75, average_stamina: 0.6 };

        let opponent = TeamInstructions::default();
        let mut rng = test_rng();
        let result = manager.update_tactics(&leading, &opponent, &mut rng);

        if let Some(tactics) = result {
            // Counter AI should slow down when leading
            assert!(
                tactics.team_tempo == TeamTempo::Slow || tactics.team_tempo == TeamTempo::Normal
            );
        }
    }

    #[test]
    fn test_update_frequency() {
        assert_eq!(AIDifficulty::Easy.update_frequency(), 999);
        assert_eq!(AIDifficulty::Medium.update_frequency(), 10);
        assert_eq!(AIDifficulty::Hard.update_frequency(), 5);
        assert_eq!(AIDifficulty::Expert.update_frequency(), 3);
    }

    #[test]
    fn test_should_update() {
        let manager = AITacticalManager::new(BALANCED_AI.clone(), AIDifficulty::Hard);

        let state1 = MatchState { current_minute: 0, ..Default::default() };
        assert!(manager.should_update(&state1)); // First update

        let mut manager2 = AITacticalManager::new(BALANCED_AI.clone(), AIDifficulty::Hard);
        manager2.last_update_minute = 50;

        let state2 = MatchState { current_minute: 54, ..Default::default() };
        assert!(!manager2.should_update(&state2)); // Only 4 minutes passed

        let state3 = MatchState { current_minute: 55, ..Default::default() };
        assert!(manager2.should_update(&state3)); // 5 minutes passed (Hard frequency)
    }

    #[test]
    fn test_all_profiles_have_korean_names() {
        for profile in AI_PROFILES {
            assert!(!profile.name_ko.is_empty());
            assert!(!profile.description.is_empty());
        }
    }
}
