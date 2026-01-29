//! Rule System Types
//!
//! Core types for the centralized rule dispatcher pattern.
//! Based on basketball RE analysis (FIX_2601/0123/01_RULE_DISPATCHER_PATTERN).
//!
//! ## Design Goals
//! - Deterministic evaluation order
//! - Team-neutral rule checking
//! - Clear decision taxonomy

use crate::engine::types::Coord10;

/// Rule check mode for gradual migration
///
/// This enum controls how rules are evaluated during the migration
/// from scattered checks to the centralized RuleDispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuleCheckMode {
    /// Statistics-only mode (current behavior)
    /// RuleDispatcher evaluates but doesn't affect game state
    #[default]
    StatisticsOnly,

    /// Legacy with tracking mode
    /// Legacy code makes decisions, Dispatcher runs in parallel for A/B comparison
    /// Logs discrepancies between legacy and dispatcher decisions
    LegacyWithTracking,

    /// Dispatcher primary mode
    /// RuleDispatcher makes all decisions, legacy code disabled
    DispatcherPrimary,
}

impl RuleCheckMode {
    /// Check if dispatcher decisions should be applied
    pub fn dispatcher_applies(&self) -> bool {
        matches!(self, RuleCheckMode::DispatcherPrimary)
    }

    /// Check if legacy decisions should be applied
    pub fn legacy_applies(&self) -> bool {
        matches!(
            self,
            RuleCheckMode::StatisticsOnly | RuleCheckMode::LegacyWithTracking
        )
    }

    /// Check if comparison tracking is enabled
    pub fn tracking_enabled(&self) -> bool {
        matches!(self, RuleCheckMode::LegacyWithTracking)
    }

    /// Create from environment variable OF_RULE_CHECK_MODE
    pub fn from_env() -> Self {
        match std::env::var("OF_RULE_CHECK_MODE")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "dispatcher" | "primary" => RuleCheckMode::DispatcherPrimary,
            "tracking" | "compare" | "ab" => RuleCheckMode::LegacyWithTracking,
            _ => RuleCheckMode::StatisticsOnly,
        }
    }
}

/// Team identifier for rule decisions (neutral representation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleTeamId {
    /// Home team (indices 0-10)
    Home,
    /// Away team (indices 11-21)
    Away,
}

impl RuleTeamId {
    /// Create from player index
    pub fn from_player_index(idx: usize) -> Self {
        if idx < 11 {
            RuleTeamId::Home
        } else {
            RuleTeamId::Away
        }
    }

    /// Get opponent team
    pub fn opponent(&self) -> Self {
        match self {
            RuleTeamId::Home => RuleTeamId::Away,
            RuleTeamId::Away => RuleTeamId::Home,
        }
    }

    /// Check if this is home team
    pub fn is_home(&self) -> bool {
        matches!(self, RuleTeamId::Home)
    }
}

/// Rule evaluation result
#[derive(Debug, Clone)]
pub enum RuleDecision {
    /// No violation, continue play
    Continue,

    /// Offside violation
    Offside {
        player_idx: usize,
        position: Coord10,
        pass_origin: Coord10,
    },

    /// Foul committed
    Foul {
        offender_idx: usize,
        victim_idx: usize,
        foul_type: FoulType,
        position: Coord10,
        card: Option<Card>,
    },

    /// Ball out of play
    OutOfPlay {
        last_touch_team: RuleTeamId,
        position: Coord10,
        restart_type: RuleRestartType,
    },

    /// Goal scored
    Goal {
        scorer_idx: usize,
        assister_idx: Option<usize>,
        position: Coord10,
    },

    /// Handball violation
    Handball {
        player_idx: usize,
        position: Coord10,
        deliberate: bool,
    },
}

impl RuleDecision {
    /// Check if this decision stops play
    pub fn stops_play(&self) -> bool {
        !matches!(self, RuleDecision::Continue)
    }

    /// Get the team that committed the violation (if any)
    pub fn violating_team(&self) -> Option<RuleTeamId> {
        match self {
            RuleDecision::Offside { player_idx, .. } => {
                Some(RuleTeamId::from_player_index(*player_idx))
            }
            RuleDecision::Foul { offender_idx, .. } => {
                Some(RuleTeamId::from_player_index(*offender_idx))
            }
            RuleDecision::Handball { player_idx, .. } => {
                Some(RuleTeamId::from_player_index(*player_idx))
            }
            _ => None,
        }
    }

    /// Get the team that benefits from this decision (if any)
    pub fn benefiting_team(&self) -> Option<RuleTeamId> {
        match self {
            RuleDecision::Goal { scorer_idx, .. } => {
                Some(RuleTeamId::from_player_index(*scorer_idx))
            }
            RuleDecision::OutOfPlay { last_touch_team, .. } => Some(last_touch_team.opponent()),
            _ => self.violating_team().map(|t| t.opponent()),
        }
    }
}

/// Foul type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoulType {
    /// Standard foul (direct free kick)
    Direct,
    /// Indirect foul (indirect free kick)
    Indirect,
    /// Foul in penalty area (penalty kick)
    Penalty,
    /// Denying obvious goal-scoring opportunity
    DenyingGoalOpportunity,
    /// Serious foul play
    SeriousFoulPlay,
    /// Violent conduct
    ViolentConduct,
}

impl FoulType {
    /// Get the restart type for this foul
    pub fn restart_type(&self) -> RuleRestartType {
        match self {
            FoulType::Penalty => RuleRestartType::PenaltyKick,
            FoulType::Indirect => RuleRestartType::IndirectFreeKick,
            _ => RuleRestartType::DirectFreeKick,
        }
    }
}

/// Restart type for dead ball situations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleRestartType {
    /// Kickoff after goal
    Kickoff,
    /// Goal kick
    GoalKick,
    /// Corner kick
    CornerKick,
    /// Throw-in
    ThrowIn,
    /// Direct free kick
    DirectFreeKick,
    /// Indirect free kick
    IndirectFreeKick,
    /// Penalty kick
    PenaltyKick,
    /// Dropped ball (rare)
    DroppedBall,
}

/// Card type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    /// Yellow card (caution)
    Yellow,
    /// Second yellow (results in red)
    SecondYellow,
    /// Direct red card
    Red,
}

/// Contact event for foul checking
#[derive(Debug, Clone)]
pub struct ContactEvent {
    /// Player making the tackle/contact
    pub tackler_idx: usize,
    /// Player with the ball
    pub ball_carrier_idx: usize,
    /// Position of contact
    pub position: Coord10,
    /// Contact direction (angle in radians)
    pub contact_angle: f32,
    /// Was the ball won first?
    pub ball_won_first: bool,
    /// Tackle intensity (0.0-1.0)
    pub intensity: f32,
}

/// Contact type classification for foul assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactType {
    /// Clean tackle (ball won first)
    BallWon,
    /// Clean contact but no ball
    Clean,
    /// From behind (more likely foul)
    FromBehind,
    /// Dangerous (studs up, two-footed, etc.)
    Dangerous,
}

impl ContactEvent {
    /// Classify the contact type
    pub fn contact_type(&self) -> ContactType {
        if self.ball_won_first {
            return ContactType::BallWon;
        }

        // Behind = contact angle > 135 degrees from front
        let is_from_behind = self.contact_angle.abs() > std::f32::consts::FRAC_PI_2 * 1.5;
        let is_dangerous = self.intensity > 0.8;

        if is_dangerous {
            ContactType::Dangerous
        } else if is_from_behind {
            ContactType::FromBehind
        } else {
            ContactType::Clean
        }
    }
}

/// Pass event for offside checking
#[derive(Debug, Clone)]
pub struct PassEvent {
    /// Passer player index
    pub passer_idx: usize,
    /// Receiver player index
    pub receiver_idx: usize,
    /// Pass origin position
    pub origin: Coord10,
    /// Pass target position
    pub target: Coord10,
    /// Attacking team
    pub attacking_team: RuleTeamId,
}

// ============================================================================
// FIX_2601/0123 Phase 6: Defense Intent Integration
// ============================================================================

/// Defense intent type classification
/// Maps to defense_intent.rs DefenseIntent for foul probability calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenseIntentType {
    /// Contain: distance maintenance, no direct challenge (low foul risk)
    Contain,
    /// Press: closing down, intercept attempts (medium foul risk)
    Press,
    /// Challenge: direct ball contest (high foul risk)
    Challenge,
}

/// Technique type for foul probability calculation
/// Fine-grained technique classification within each DefenseIntentType
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechniqueType {
    // Challenge techniques
    /// Standing tackle - safe, controlled (10% base foul)
    StandingTackle,
    /// Sliding tackle - risky, wide coverage (25% base foul)
    SlidingTackle,
    /// Shoulder charge - physical contest (15% base foul)
    ShoulderCharge,
    /// Poke away - quick ball touch (5% base foul)
    PokeAway,

    // Press techniques
    /// Closing down - fast approach (8% base foul)
    ClosingDown,
    /// Intercept attempt - cutting pass lane (5% base foul)
    InterceptAttempt,
    /// Force touchline - pushing to sideline (10% base foul)
    ForceTouchline,
    /// Track runner - following off-ball movement (6% base foul)
    TrackRunner,

    // Simple fallback
    /// Simple tackle - generic challenge (15% base foul)
    SimpleTackle,
}

impl TechniqueType {
    /// Get base foul probability for this technique
    pub fn base_foul_probability(&self) -> f32 {
        match self {
            // Challenge techniques
            TechniqueType::StandingTackle => 0.10,
            TechniqueType::SlidingTackle => 0.25,
            TechniqueType::ShoulderCharge => 0.15,
            TechniqueType::PokeAway => 0.05,
            // Press techniques
            TechniqueType::ClosingDown => 0.08,
            TechniqueType::InterceptAttempt => 0.05,
            TechniqueType::ForceTouchline => 0.10,
            TechniqueType::TrackRunner => 0.06,
            // Fallback
            TechniqueType::SimpleTackle => 0.15,
        }
    }

    /// Convert from defense_intent ChallengeTechnique
    pub fn from_challenge_technique_idx(idx: u8) -> Self {
        match idx {
            0 => TechniqueType::StandingTackle,
            1 => TechniqueType::SlidingTackle,
            2 => TechniqueType::ShoulderCharge,
            3 => TechniqueType::PokeAway,
            _ => TechniqueType::SimpleTackle,
        }
    }

    /// Convert from defense_intent PressTechnique
    pub fn from_press_technique_idx(idx: u8) -> Self {
        match idx {
            0 => TechniqueType::ClosingDown,
            1 => TechniqueType::InterceptAttempt,
            2 => TechniqueType::ForceTouchline,
            3 => TechniqueType::TrackRunner,
            _ => TechniqueType::ClosingDown,
        }
    }

    /// Get base yellow card probability for this technique
    pub fn base_yellow_probability(&self) -> f32 {
        match self {
            TechniqueType::StandingTackle => 0.15,
            TechniqueType::SlidingTackle => 0.30,
            TechniqueType::ShoulderCharge => 0.10,
            TechniqueType::PokeAway => 0.05,
            TechniqueType::ClosingDown => 0.08,
            TechniqueType::InterceptAttempt => 0.05,
            TechniqueType::ForceTouchline => 0.12,
            TechniqueType::TrackRunner => 0.06,
            TechniqueType::SimpleTackle => 0.15,
        }
    }

    /// Get base red card probability for this technique
    pub fn base_red_probability(&self) -> f32 {
        match self {
            TechniqueType::StandingTackle => 0.02,
            TechniqueType::SlidingTackle => 0.05,
            TechniqueType::ShoulderCharge => 0.01,
            TechniqueType::PokeAway => 0.00,
            TechniqueType::ClosingDown => 0.005,
            TechniqueType::InterceptAttempt => 0.002,
            TechniqueType::ForceTouchline => 0.01,
            TechniqueType::TrackRunner => 0.005,
            TechniqueType::SimpleTackle => 0.02,
        }
    }
}

/// Defense intent information for foul calculation
/// Provides context from the defense intent system for dispatcher evaluation
#[derive(Debug, Clone)]
pub struct DefenseIntentInfo {
    /// Type of defense intent (Contain/Press/Challenge)
    pub intent_type: DefenseIntentType,
    /// Specific technique used
    pub technique: TechniqueType,
    /// Whether defender won the ball contest
    pub defense_won: bool,
    /// Defender tackling skill (0-100)
    pub defender_tackling: f32,
    /// Defender aggression (0-100)
    pub defender_aggression: f32,
}

impl DefenseIntentInfo {
    /// Create from challenge technique
    pub fn from_challenge(
        technique: TechniqueType,
        defense_won: bool,
        tackling: f32,
        aggression: f32,
    ) -> Self {
        Self {
            intent_type: DefenseIntentType::Challenge,
            technique,
            defense_won,
            defender_tackling: tackling,
            defender_aggression: aggression,
        }
    }

    /// Create from press technique
    pub fn from_press(
        technique: TechniqueType,
        defense_won: bool,
        tackling: f32,
        aggression: f32,
    ) -> Self {
        Self {
            intent_type: DefenseIntentType::Press,
            technique,
            defense_won,
            defender_tackling: tackling,
            defender_aggression: aggression,
        }
    }

    /// Create simple tackle info (fallback for legacy code)
    pub fn simple_tackle(defense_won: bool, tackling: f32, aggression: f32) -> Self {
        Self {
            intent_type: DefenseIntentType::Challenge,
            technique: TechniqueType::SimpleTackle,
            defense_won,
            defender_tackling: tackling,
            defender_aggression: aggression,
        }
    }
}

// ============================================================================
// FIX_2601/0123 Phase 6: Handball Contact Event
// ============================================================================

/// Handball contact event for dispatcher evaluation
#[derive(Debug, Clone)]
pub struct HandballContactEvent {
    /// Player who committed the handball
    pub player_idx: usize,
    /// Position where handball occurred
    pub position: Coord10,
    /// Whether arm was in unnatural position
    pub unnatural_position: bool,
    /// Arm extension level (0.0 = at side, 1.0 = fully extended)
    pub arm_extension: f32,
    /// Whether player deliberately moved arm toward ball
    pub deliberate_movement: bool,
    /// Whether player gained advantage (scoring opportunity, possession)
    pub gained_advantage: bool,
}

impl HandballContactEvent {
    /// Calculate deliberate handball probability based on context
    pub fn deliberate_probability(&self) -> f32 {
        let mut prob = 0.0;

        // Unnatural position strongly suggests deliberate
        if self.unnatural_position {
            prob += 0.40;
        }

        // Arm extension affects probability
        prob += self.arm_extension * 0.30;

        // Deliberate movement is strong evidence
        if self.deliberate_movement {
            prob += 0.50;
        }

        // Gaining advantage increases likelihood of being called
        if self.gained_advantage {
            prob += 0.20;
        }

        prob.clamp(0.0, 1.0)
    }
}

// ============================================================================
// FIX_2601/0123 Phase 6: Dispatcher Context for Duel Integration
// ============================================================================

/// Context for optional dispatcher integration in duel resolution
///
/// When provided, duel.rs can optionally use the RuleDispatcher for
/// foul decisions instead of the legacy calculation. This enables
/// A/B testing and gradual migration to the centralized rule system.
#[derive(Debug, Clone)]
pub struct DispatcherContext {
    /// Rule check mode (StatisticsOnly, LegacyWithTracking, DispatcherPrimary)
    pub mode: RuleCheckMode,
    /// Defender player index
    pub defender_idx: usize,
    /// Attacker player index
    pub attacker_idx: usize,
    /// Contact position
    pub position: Coord10,
    /// Pre-generated RNG roll for deterministic evaluation
    pub rng_roll: f32,
}

impl DispatcherContext {
    /// Create a new dispatcher context
    pub fn new(
        mode: RuleCheckMode,
        defender_idx: usize,
        attacker_idx: usize,
        position: Coord10,
        rng_roll: f32,
    ) -> Self {
        Self {
            mode,
            defender_idx,
            attacker_idx,
            position,
            rng_roll,
        }
    }

    /// Create from environment mode
    pub fn from_env(
        defender_idx: usize,
        attacker_idx: usize,
        position: Coord10,
        rng_roll: f32,
    ) -> Self {
        Self {
            mode: RuleCheckMode::from_env(),
            defender_idx,
            attacker_idx,
            position,
            rng_roll,
        }
    }
}

/// Rule evaluation statistics (for debugging and QA)
#[derive(Debug, Clone, Default)]
pub struct RuleEvaluationStats {
    /// Total ticks evaluated
    pub ticks_evaluated: u64,
    /// Goals detected
    pub goals_detected: u32,
    /// Out of play detected
    pub out_of_play_detected: u32,
    /// Offsides detected
    pub offsides_detected: u32,
    /// Fouls detected
    pub fouls_detected: u32,
    /// Handballs detected
    pub handballs_detected: u32,
    /// Home team decisions
    pub home_decisions: u32,
    /// Away team decisions
    pub away_decisions: u32,
}

impl RuleEvaluationStats {
    /// Calculate team bias ratio (0.5 = perfect balance)
    pub fn team_bias_ratio(&self) -> f32 {
        let total = self.home_decisions + self.away_decisions;
        if total == 0 {
            return 0.5;
        }
        self.home_decisions as f32 / total as f32
    }

    /// Check if bias is within acceptable range (<= 5% deviation from 50%)
    pub fn is_bias_acceptable(&self) -> bool {
        let ratio = self.team_bias_ratio();
        // Add small epsilon for floating-point comparison (0.05 + epsilon for ~5% tolerance)
        (ratio - 0.5).abs() <= 0.0501
    }
}

/// Field bounds for out-of-play checking
#[derive(Debug, Clone, Copy)]
pub struct FieldBounds {
    /// Field length (x-direction) in meters
    pub length_m: f32,
    /// Field width (y-direction) in meters
    pub width_m: f32,
    /// Goal width in meters
    pub goal_width_m: f32,
    /// Penalty area length in meters
    pub penalty_area_length_m: f32,
    /// Penalty area width in meters
    pub penalty_area_width_m: f32,
}

impl Default for FieldBounds {
    fn default() -> Self {
        use crate::engine::physics_constants::{field, goal};
        Self {
            length_m: field::LENGTH_M,
            width_m: field::WIDTH_M,
            goal_width_m: goal::WIDTH_M,
            penalty_area_length_m: field::PENALTY_AREA_LENGTH_M,
            // FIFA standard: 40.32m (16.5m from goal line × 2 sides + goal width)
            // But more accurately: 2 × 16.5m from each post + 7.32m goal = 40.32m
            // Simplified: ~40m penalty area width
            penalty_area_width_m: 40.32,
        }
    }
}

impl FieldBounds {
    /// Check if position is out of bounds
    pub fn is_out_of_bounds(&self, pos: &Coord10) -> bool {
        let (x, y) = pos.to_meters();
        x < 0.0 || x > self.length_m || y < 0.0 || y > self.width_m
    }

    /// Check if position is in penalty area (either end)
    pub fn is_in_penalty_area(&self, pos: &Coord10) -> Option<RuleTeamId> {
        let (x, y) = pos.to_meters();
        let half_width = self.width_m / 2.0;
        let pa_half_width = self.penalty_area_width_m / 2.0;

        // Y must be within penalty area width (centered on goal)
        if y < half_width - pa_half_width || y > half_width + pa_half_width {
            return None;
        }

        // Check home end (x < penalty_area_length)
        if x < self.penalty_area_length_m {
            return Some(RuleTeamId::Home);
        }

        // Check away end (x > length - penalty_area_length)
        if x > self.length_m - self.penalty_area_length_m {
            return Some(RuleTeamId::Away);
        }

        None
    }

    /// Determine restart type for out of bounds
    pub fn out_of_bounds_restart(
        &self,
        pos: &Coord10,
        last_touch_team: RuleTeamId,
    ) -> Option<RuleRestartType> {
        let (x, y) = pos.to_meters();
        let half_width = self.width_m / 2.0;
        let goal_half_width = self.goal_width_m / 2.0;

        // Touchline (y out of bounds) -> throw-in
        if y < 0.0 || y > self.width_m {
            return Some(RuleRestartType::ThrowIn);
        }

        // Goal line at home end (x < 0)
        if x < 0.0 {
            // Check if in goal area
            let in_goal = y > half_width - goal_half_width && y < half_width + goal_half_width;
            if in_goal {
                return None; // Could be a goal, checked elsewhere
            }
            // Last touch by attacking team (away) -> goal kick
            // Last touch by defending team (home) -> corner
            return Some(if last_touch_team == RuleTeamId::Away {
                RuleRestartType::GoalKick
            } else {
                RuleRestartType::CornerKick
            });
        }

        // Goal line at away end (x > length)
        if x > self.length_m {
            let in_goal = y > half_width - goal_half_width && y < half_width + goal_half_width;
            if in_goal {
                return None; // Could be a goal
            }
            // Last touch by attacking team (home) -> goal kick
            // Last touch by defending team (away) -> corner
            return Some(if last_touch_team == RuleTeamId::Home {
                RuleRestartType::GoalKick
            } else {
                RuleRestartType::CornerKick
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_team_id_from_player_index() {
        assert_eq!(RuleTeamId::from_player_index(0), RuleTeamId::Home);
        assert_eq!(RuleTeamId::from_player_index(10), RuleTeamId::Home);
        assert_eq!(RuleTeamId::from_player_index(11), RuleTeamId::Away);
        assert_eq!(RuleTeamId::from_player_index(21), RuleTeamId::Away);
    }

    #[test]
    fn test_rule_team_id_opponent() {
        assert_eq!(RuleTeamId::Home.opponent(), RuleTeamId::Away);
        assert_eq!(RuleTeamId::Away.opponent(), RuleTeamId::Home);
    }

    #[test]
    fn test_contact_type_classification() {
        // Ball won = clean
        let contact = ContactEvent {
            tackler_idx: 0,
            ball_carrier_idx: 11,
            position: Coord10::from_meters(50.0, 34.0),
            contact_angle: 0.0,
            ball_won_first: true,
            intensity: 0.5,
        };
        assert_eq!(contact.contact_type(), ContactType::BallWon);

        // From behind
        let contact_behind = ContactEvent {
            ball_won_first: false,
            contact_angle: std::f32::consts::PI, // 180 degrees
            intensity: 0.5,
            ..contact.clone()
        };
        assert_eq!(contact_behind.contact_type(), ContactType::FromBehind);

        // Dangerous
        let contact_dangerous = ContactEvent {
            ball_won_first: false,
            contact_angle: 0.0,
            intensity: 0.9,
            ..contact
        };
        assert_eq!(contact_dangerous.contact_type(), ContactType::Dangerous);
    }

    #[test]
    fn test_foul_type_restart() {
        assert_eq!(FoulType::Direct.restart_type(), RuleRestartType::DirectFreeKick);
        assert_eq!(FoulType::Indirect.restart_type(), RuleRestartType::IndirectFreeKick);
        assert_eq!(FoulType::Penalty.restart_type(), RuleRestartType::PenaltyKick);
    }

    #[test]
    fn test_rule_evaluation_stats_bias() {
        let mut stats = RuleEvaluationStats::default();

        // 50-50 split = no bias
        stats.home_decisions = 50;
        stats.away_decisions = 50;
        assert!((stats.team_bias_ratio() - 0.5).abs() < 0.001);
        assert!(stats.is_bias_acceptable());

        // 55-45 split = acceptable (5% bias)
        stats.home_decisions = 55;
        stats.away_decisions = 45;
        assert!(stats.is_bias_acceptable());

        // 60-40 split = not acceptable (10% bias)
        stats.home_decisions = 60;
        stats.away_decisions = 40;
        assert!(!stats.is_bias_acceptable());
    }

    #[test]
    fn test_field_bounds_out_of_bounds() {
        let bounds = FieldBounds::default();

        // In bounds
        assert!(!bounds.is_out_of_bounds(&Coord10::from_meters(50.0, 34.0)));

        // Out of bounds
        assert!(bounds.is_out_of_bounds(&Coord10::from_meters(-1.0, 34.0)));
        assert!(bounds.is_out_of_bounds(&Coord10::from_meters(50.0, -1.0)));
        assert!(bounds.is_out_of_bounds(&Coord10::from_meters(106.0, 34.0)));
        assert!(bounds.is_out_of_bounds(&Coord10::from_meters(50.0, 69.0)));
    }

    #[test]
    fn test_field_bounds_penalty_area() {
        let bounds = FieldBounds::default();

        // In home penalty area
        let home_pa = bounds.is_in_penalty_area(&Coord10::from_meters(8.0, 34.0));
        assert_eq!(home_pa, Some(RuleTeamId::Home));

        // In away penalty area
        let away_pa = bounds.is_in_penalty_area(&Coord10::from_meters(97.0, 34.0));
        assert_eq!(away_pa, Some(RuleTeamId::Away));

        // Not in penalty area
        let midfield = bounds.is_in_penalty_area(&Coord10::from_meters(52.5, 34.0));
        assert_eq!(midfield, None);
    }

    #[test]
    fn test_rule_check_mode_default() {
        let mode = RuleCheckMode::default();
        assert_eq!(mode, RuleCheckMode::StatisticsOnly);
    }

    #[test]
    fn test_rule_check_mode_applies() {
        // StatisticsOnly: legacy applies, dispatcher doesn't
        let stats = RuleCheckMode::StatisticsOnly;
        assert!(stats.legacy_applies());
        assert!(!stats.dispatcher_applies());
        assert!(!stats.tracking_enabled());

        // LegacyWithTracking: legacy applies, tracking enabled
        let tracking = RuleCheckMode::LegacyWithTracking;
        assert!(tracking.legacy_applies());
        assert!(!tracking.dispatcher_applies());
        assert!(tracking.tracking_enabled());

        // DispatcherPrimary: dispatcher applies, legacy doesn't
        let primary = RuleCheckMode::DispatcherPrimary;
        assert!(!primary.legacy_applies());
        assert!(primary.dispatcher_applies());
        assert!(!primary.tracking_enabled());
    }

    // ========== FIX_2601/0123 Phase 6: Defense Intent Tests ==========

    #[test]
    fn test_technique_base_foul_probability() {
        // Challenge techniques
        assert!((TechniqueType::StandingTackle.base_foul_probability() - 0.10).abs() < 0.001);
        assert!((TechniqueType::SlidingTackle.base_foul_probability() - 0.25).abs() < 0.001);
        assert!((TechniqueType::ShoulderCharge.base_foul_probability() - 0.15).abs() < 0.001);
        assert!((TechniqueType::PokeAway.base_foul_probability() - 0.05).abs() < 0.001);

        // Press techniques
        assert!((TechniqueType::ClosingDown.base_foul_probability() - 0.08).abs() < 0.001);
        assert!((TechniqueType::InterceptAttempt.base_foul_probability() - 0.05).abs() < 0.001);
        assert!((TechniqueType::ForceTouchline.base_foul_probability() - 0.10).abs() < 0.001);
        assert!((TechniqueType::TrackRunner.base_foul_probability() - 0.06).abs() < 0.001);
    }

    #[test]
    fn test_defense_intent_info_creation() {
        let info = DefenseIntentInfo::from_challenge(
            TechniqueType::SlidingTackle,
            false,
            80.0,
            70.0,
        );

        assert_eq!(info.intent_type, DefenseIntentType::Challenge);
        assert_eq!(info.technique, TechniqueType::SlidingTackle);
        assert!(!info.defense_won);
        assert!((info.defender_tackling - 80.0).abs() < 0.001);
        assert!((info.defender_aggression - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_defense_intent_info_simple_tackle() {
        let info = DefenseIntentInfo::simple_tackle(true, 75.0, 65.0);

        assert_eq!(info.intent_type, DefenseIntentType::Challenge);
        assert_eq!(info.technique, TechniqueType::SimpleTackle);
        assert!(info.defense_won);
    }

    #[test]
    fn test_handball_contact_deliberate_probability() {
        // Accidental handball - low probability
        let accidental = HandballContactEvent {
            player_idx: 5,
            position: Coord10::from_meters(50.0, 34.0),
            unnatural_position: false,
            arm_extension: 0.2,
            deliberate_movement: false,
            gained_advantage: false,
        };
        let prob = accidental.deliberate_probability();
        assert!(prob < 0.2, "Accidental handball should have low probability: {}", prob);

        // Deliberate handball - high probability
        let deliberate = HandballContactEvent {
            player_idx: 5,
            position: Coord10::from_meters(50.0, 34.0),
            unnatural_position: true,
            arm_extension: 0.8,
            deliberate_movement: true,
            gained_advantage: true,
        };
        let prob = deliberate.deliberate_probability();
        assert!(prob > 0.8, "Deliberate handball should have high probability: {}", prob);
    }
}
