//! Core types for Off-Ball Decision System v1

use serde::{Deserialize, Serialize};

// ============================================================================
// Constants
// ============================================================================

/// Default TTL in ticks (~3 seconds at 4 ticks/sec)
pub const DEFAULT_TTL_TICKS: u64 = 12;

/// Shorter TTL during transition phases
pub const TRANSITION_TTL_TICKS: u64 = 6;

/// Window after possession change where transition rules apply
pub const TRANSITION_WINDOW_TICKS: u64 = 8;

/// Ball proximity that triggers immediate re-decision
pub const BALL_PROXIMITY_TRIGGER_M: f32 = 6.0;

/// Stamina threshold below which sprint is forbidden
pub const STAMINA_LOW_THRESHOLD: f32 = 0.2;

/// Maximum decisions per tick (normal play)
pub const MAX_DECISIONS_NORMAL: usize = 6;

/// Maximum decisions per tick (transition)
pub const MAX_DECISIONS_TRANSITION: usize = 8;

/// Number of ball-proximate players to always include
pub const BALL_PROXIMITY_TOP_N: usize = 4;

/// Collision detection radius in meters
pub const COLLISION_RADIUS_M: f32 = 2.0;

/// Shift amount for collision resolution
pub const COLLISION_SHIFT_M: f32 = 3.0;

/// Maximum candidates per player
pub const MAX_CANDIDATES_PER_PLAYER: usize = 5;

/// Movement speeds by urgency
pub const SPEED_WALK_M_PER_S: f32 = 2.0;
pub const SPEED_JOG_M_PER_S: f32 = 5.0;
pub const SPEED_SPRINT_M_PER_S: f32 = 8.0;

// ============================================================================
// Line Spacing Constants (FIX_2601/1126)
// ============================================================================

/// Target DEF-MID gap in meters (12-16m range)
pub const DEF_MID_GAP_TARGET_M: f32 = 14.0;

/// Target MID-FWD gap in meters (14-18m range)
pub const MID_FWD_GAP_TARGET_M: f32 = 16.0;

/// Radius within which line deviation is tolerated
pub const LINE_DEVIATION_PENALTY_RADIUS_M: f32 = 8.0;

// ============================================================================
// OffBallObjective - Core state stored per player
// ============================================================================

/// Off-ball player's movement objective with TTL.
///
/// This is the main state stored in MatchEngine. positioning_engine reads this
/// to determine where each off-ball player should move.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct OffBallObjective {
    /// What the player intends to do
    pub intent: OffBallIntent,
    /// Target position in meters (field coordinates: 0-105 x 0-68)
    pub target_x: f32,
    pub target_y: f32,
    /// Movement urgency (affects speed)
    pub urgency: Urgency,
    /// Tick when this objective expires (TTL)
    pub expire_tick: u64,
    /// Priority for conflict resolution (higher = keep)
    pub priority: u8,
    /// Confidence score from evaluation (0.0..1.0)
    pub confidence: f32,
}

impl OffBallObjective {
    /// Check if objective has expired
    #[inline]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        current_tick >= self.expire_tick
    }

    /// Check if objective is valid (has an active intent)
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.intent != OffBallIntent::None
    }

    /// Get target position as tuple
    #[inline]
    pub fn target_pos(&self) -> (f32, f32) {
        (self.target_x, self.target_y)
    }

    /// Create a new objective
    pub fn new(
        intent: OffBallIntent,
        target_x: f32,
        target_y: f32,
        urgency: Urgency,
        expire_tick: u64,
        confidence: f32,
    ) -> Self {
        Self {
            intent,
            target_x,
            target_y,
            urgency,
            expire_tick,
            priority: get_intent_priority(intent),
            confidence,
        }
    }

    /// Force expire this objective
    pub fn force_expire(&mut self, current_tick: u64) {
        self.expire_tick = current_tick;
    }

    /// Clear the objective (no active intent)
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

// ============================================================================
// OffBallIntent - What the player intends to do
// ============================================================================

/// Off-ball intent types (v1: 8 types + None)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum OffBallIntent {
    /// No active objective - use default formation position
    #[default]
    None,

    // === Attacking (team has ball) ===
    /// Provide passing option (triangle/wall pass angle)
    LinkPlayer,
    /// Run behind defensive line / into half-space
    SpaceAttacker,
    /// Wait for second ball / cutback / box edge
    Lurker,
    /// Maintain width / provide overlap channel
    WidthHolder,
    /// Maintain formation shape / line spacing (FIX_2601/1126)
    ShapeHolder,

    // === Defending (opponent has ball) ===
    /// Recover defensive shape
    TrackBack,
    /// Block dangerous lane (ball-to-goal axis)
    Screen,
    /// Support press (angle block + approach, not direct tackle)
    PressSupport,
}

impl OffBallIntent {
    /// Check if this is an attacking intent
    pub fn is_attacking(&self) -> bool {
        matches!(
            self,
            Self::LinkPlayer | Self::SpaceAttacker | Self::Lurker | Self::WidthHolder | Self::ShapeHolder
        )
    }

    /// Check if this is a defensive intent
    pub fn is_defensive(&self) -> bool {
        matches!(self, Self::TrackBack | Self::Screen | Self::PressSupport)
    }
}

/// Get priority for an intent (higher = more important to keep)
pub fn get_intent_priority(intent: OffBallIntent) -> u8 {
    match intent {
        OffBallIntent::SpaceAttacker => 3,
        OffBallIntent::LinkPlayer | OffBallIntent::Lurker => 2,
        OffBallIntent::Screen | OffBallIntent::PressSupport | OffBallIntent::TrackBack => 2,
        OffBallIntent::ShapeHolder => 2, // FIX_2601/1126: shape maintenance is important
        OffBallIntent::WidthHolder => 1,
        OffBallIntent::None => 0,
    }
}

// ============================================================================
// Urgency - Movement speed mode
// ============================================================================

/// Movement urgency affecting speed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum Urgency {
    Walk,
    #[default]
    Jog,
    Sprint,
}

impl Urgency {
    /// Get speed in meters per second
    pub fn speed_m_per_s(&self) -> f32 {
        match self {
            Urgency::Walk => SPEED_WALK_M_PER_S,
            Urgency::Jog => SPEED_JOG_M_PER_S,
            Urgency::Sprint => SPEED_SPRINT_M_PER_S,
        }
    }
}

// ============================================================================
// GamePhase - Current phase of play
// ============================================================================

/// Current phase of play for off-ball decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GamePhase {
    /// Team has possession, building attack
    #[default]
    Attacking,
    /// Opponent has possession, defending
    Defending,
    /// Just won possession (first 8 ticks)
    TransitionWin,
    /// Just lost possession (first 8 ticks)
    TransitionLoss,
}

impl GamePhase {
    pub fn is_transition(&self) -> bool {
        matches!(self, Self::TransitionWin | Self::TransitionLoss)
    }

    pub fn is_attacking(&self) -> bool {
        matches!(self, Self::Attacking | Self::TransitionWin)
    }

    pub fn is_defending(&self) -> bool {
        matches!(self, Self::Defending | Self::TransitionLoss)
    }
}

// ============================================================================
// OffBallCandidate - Candidate for selection
// ============================================================================

/// A candidate objective being evaluated
#[derive(Debug, Clone, Copy)]
pub struct OffBallCandidate {
    pub intent: OffBallIntent,
    pub target_x: f32,
    pub target_y: f32,
    pub urgency: Urgency,
}

impl OffBallCandidate {
    pub fn new(intent: OffBallIntent, target_x: f32, target_y: f32, urgency: Urgency) -> Self {
        Self {
            intent,
            target_x,
            target_y,
            urgency,
        }
    }

    pub fn target_pos(&self) -> (f32, f32) {
        (self.target_x, self.target_y)
    }
}

// ============================================================================
// Score6 - 6-factor evaluation score
// ============================================================================

/// 6-factor score for off-ball candidate evaluation (UAE-lite)
#[derive(Debug, Clone, Copy, Default)]
pub struct Score6 {
    /// Team benefit (support/block/transition)
    pub usefulness: f32,
    /// Structure preservation / counter-attack safety
    pub safety: f32,
    /// Pass option / lane blocking connectivity
    pub availability: f32,
    /// Forward progress / threat increase
    pub progress: f32,
    /// Line/width/spacing contribution
    pub structure: f32,
    /// Stamina/distance cost (inverted: 1.0 = low cost)
    pub cost: f32,
}

impl Score6 {
    /// Sum all factors
    pub fn total(&self) -> f32 {
        self.usefulness + self.safety + self.availability + self.progress + self.structure + self.cost
    }

    /// Weighted sum
    pub fn weighted_total(&self, w: &Score6Weights) -> f32 {
        self.usefulness * w.usefulness
            + self.safety * w.safety
            + self.availability * w.availability
            + self.progress * w.progress
            + self.structure * w.structure
            + self.cost * w.cost
    }
}

/// Weights for Score6 factors
#[derive(Debug, Clone, Copy)]
pub struct Score6Weights {
    pub usefulness: f32,
    pub safety: f32,
    pub availability: f32,
    pub progress: f32,
    pub structure: f32,
    pub cost: f32,
}

impl Default for Score6Weights {
    fn default() -> Self {
        Self {
            usefulness: 1.0,
            safety: 1.0,
            availability: 1.0,
            progress: 1.0,
            structure: 1.0,
            cost: 1.0,
        }
    }
}

// ============================================================================
// OffBallContext - Context for decision making
// ============================================================================

/// Context provided to off-ball decision functions
#[derive(Debug, Clone)]
pub struct OffBallContext {
    /// Ball position in meters
    pub ball_x: f32,
    pub ball_y: f32,
    /// Ball velocity
    pub ball_vx: f32,
    pub ball_vy: f32,
    /// Team with possession (0 = home, 1 = away)
    pub possession_team: u8,
    /// Current game phase
    pub phase: GamePhase,
    /// Player being evaluated
    pub player_idx: usize,
    /// Player position
    pub player_x: f32,
    pub player_y: f32,
    /// Player velocity
    pub player_vx: f32,
    pub player_vy: f32,
    /// Player stamina (0.0-1.0)
    pub player_stamina: f32,
    /// Player's base formation position
    pub base_x: f32,
    pub base_y: f32,
    /// Is player on home team?
    pub is_home_team: bool,
    /// Attacking direction (1.0 = right, -1.0 = left)
    pub attack_direction: f32,
    /// Estimated defensive line x position
    pub defensive_line_x: f32,
    /// Ticks since last possession change
    pub ticks_since_transition: u64,
    /// Current tick
    pub current_tick: u64,
    /// Player's line role (FIX_2601/1126): 0=GK, 1=DEF, 2=MID, 3=FWD
    pub player_line_role: u8,
    /// Target X anchor for this player's line (FIX_2601/1126)
    pub line_anchor_x: f32,
}

impl OffBallContext {
    pub fn ball_pos(&self) -> (f32, f32) {
        (self.ball_x, self.ball_y)
    }

    pub fn player_pos(&self) -> (f32, f32) {
        (self.player_x, self.player_y)
    }

    pub fn base_pos(&self) -> (f32, f32) {
        (self.base_x, self.base_y)
    }

    /// Distance from player to ball
    pub fn dist_to_ball(&self) -> f32 {
        let dx = self.player_x - self.ball_x;
        let dy = self.player_y - self.ball_y;
        (dx * dx + dy * dy).sqrt()
    }
}

// ============================================================================
// OffBallConfig - Tunable parameters
// ============================================================================

/// Configuration for off-ball decision system (3 tuning points)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OffBallConfig {
    /// Default TTL in ticks
    pub ttl_ticks_default: u64,
    /// TTL during transition
    pub ttl_ticks_transition: u64,
    /// Max decisions per tick (normal)
    pub max_decisions_normal: usize,
    /// Max decisions per tick (transition)
    pub max_decisions_transition: usize,
    /// Softmax temperature (0.0 = argmax)
    pub softmax_temperature: f32,
}

impl Default for OffBallConfig {
    fn default() -> Self {
        Self {
            ttl_ticks_default: DEFAULT_TTL_TICKS,
            ttl_ticks_transition: TRANSITION_TTL_TICKS,
            max_decisions_normal: MAX_DECISIONS_NORMAL,
            max_decisions_transition: MAX_DECISIONS_TRANSITION,
            softmax_temperature: 0.0, // argmax by default
        }
    }
}

// ============================================================================
// Tactical Preset & ShapeBias (FIX_2601/1127)
// ============================================================================

/// Tactical preset types for team play style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum TacticalPreset {
    /// Balanced approach (default)
    #[default]
    Balanced,
    /// Possession-focused play
    Possession,
    /// High pressing / gegenpressing
    HighPress,
    /// Counter-attacking style
    Counter,
    /// Defensive / park the bus
    ParkTheBus,
}

/// Shape bias parameters that vary by tactical preset.
///
/// FIX_2601/1127: These parameters control how offball intents are blended
/// with positioning_engine's base targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShapeBias {
    /// Weight for blending intent target with base target (0.0-1.0)
    /// Higher = more influence from offball intent
    pub intent_blend_weight: f32,
    /// DEF-MID gap target in meters
    pub def_mid_gap_m: f32,
    /// MID-FWD gap target in meters
    pub mid_fwd_gap_m: f32,
    /// Line spacing enforcement strength (0.0-1.0)
    pub line_spacing_strength: f32,
    /// Width maintenance factor (0.0-1.0)
    pub width_factor: f32,
    /// Pressing trigger distance in meters
    pub press_trigger_distance_m: f32,
    /// Compactness target (lower = more compact)
    pub compactness_target_m: f32,
}

impl Default for ShapeBias {
    fn default() -> Self {
        Self::for_preset(TacticalPreset::Balanced)
    }
}

impl ShapeBias {
    /// Create ShapeBias for a specific tactical preset
    pub fn for_preset(preset: TacticalPreset) -> Self {
        match preset {
            TacticalPreset::Balanced => Self {
                intent_blend_weight: 0.5,
                def_mid_gap_m: 14.0,
                mid_fwd_gap_m: 16.0,
                line_spacing_strength: 0.7,
                width_factor: 0.6,
                press_trigger_distance_m: 8.0,
                compactness_target_m: 32.0,
            },
            TacticalPreset::Possession => Self {
                intent_blend_weight: 0.6,
                def_mid_gap_m: 12.0,      // Tighter gaps for triangles
                mid_fwd_gap_m: 12.0,
                line_spacing_strength: 0.8,
                width_factor: 0.8,         // More width for passing lanes
                press_trigger_distance_m: 6.0,
                compactness_target_m: 28.0, // More compact
            },
            TacticalPreset::HighPress => Self {
                intent_blend_weight: 0.7,
                def_mid_gap_m: 10.0,       // Very tight for pressing
                mid_fwd_gap_m: 10.0,
                line_spacing_strength: 0.9,
                width_factor: 0.5,
                press_trigger_distance_m: 12.0, // Aggressive press
                compactness_target_m: 25.0,     // Very compact
            },
            TacticalPreset::Counter => Self {
                intent_blend_weight: 0.4,
                def_mid_gap_m: 18.0,       // Larger gaps for quick transitions
                mid_fwd_gap_m: 20.0,
                line_spacing_strength: 0.5,
                width_factor: 0.4,
                press_trigger_distance_m: 5.0, // Conservative press
                compactness_target_m: 38.0,    // More stretched
            },
            TacticalPreset::ParkTheBus => Self {
                intent_blend_weight: 0.3,
                def_mid_gap_m: 8.0,        // Very tight defensive block
                mid_fwd_gap_m: 22.0,       // FWD isolated
                line_spacing_strength: 1.0,
                width_factor: 0.3,         // Narrow
                press_trigger_distance_m: 3.0, // Only press when very close
                compactness_target_m: 22.0,    // Extremely compact defense
            },
        }
    }

    /// Get effective DEF-MID gap considering current preset
    pub fn effective_def_mid_gap(&self) -> f32 {
        self.def_mid_gap_m
    }

    /// Get effective MID-FWD gap considering current preset
    pub fn effective_mid_fwd_gap(&self) -> f32 {
        self.mid_fwd_gap_m
    }

    /// Calculate blend weight for a given confidence score
    /// Higher confidence from offball system = more weight to intent target
    pub fn calc_blend_weight(&self, confidence: f32, distance_to_target: f32) -> f32 {
        // Base weight from preset
        let base = self.intent_blend_weight;

        // Confidence modifier (0.5-1.5x)
        let conf_mod = 0.5 + confidence;

        // Distance modifier: closer targets get more weight (0.7-1.0x)
        let dist_mod = if distance_to_target < 5.0 {
            1.0
        } else if distance_to_target < 15.0 {
            0.85
        } else {
            0.7
        };

        (base * conf_mod * dist_mod).clamp(0.0, 0.9)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_expiry() {
        let obj = OffBallObjective {
            intent: OffBallIntent::LinkPlayer,
            expire_tick: 100,
            ..Default::default()
        };

        assert!(!obj.is_expired(99));
        assert!(obj.is_expired(100));
        assert!(obj.is_expired(101));
    }

    #[test]
    fn test_objective_validity() {
        let valid = OffBallObjective {
            intent: OffBallIntent::SpaceAttacker,
            ..Default::default()
        };
        let invalid = OffBallObjective::default();

        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_intent_priority() {
        assert_eq!(get_intent_priority(OffBallIntent::SpaceAttacker), 3);
        assert_eq!(get_intent_priority(OffBallIntent::LinkPlayer), 2);
        assert_eq!(get_intent_priority(OffBallIntent::WidthHolder), 1);
        assert_eq!(get_intent_priority(OffBallIntent::None), 0);
    }

    #[test]
    fn test_urgency_speed() {
        assert_eq!(Urgency::Walk.speed_m_per_s(), 2.0);
        assert_eq!(Urgency::Jog.speed_m_per_s(), 5.0);
        assert_eq!(Urgency::Sprint.speed_m_per_s(), 8.0);
    }

    #[test]
    fn test_score6_total() {
        let score = Score6 {
            usefulness: 0.8,
            safety: 0.7,
            availability: 0.6,
            progress: 0.5,
            structure: 0.4,
            cost: 0.9,
        };
        let total = score.total();
        assert!((total - 3.9).abs() < 0.001);
    }

    #[test]
    fn test_game_phase() {
        assert!(GamePhase::TransitionWin.is_transition());
        assert!(GamePhase::TransitionLoss.is_transition());
        assert!(!GamePhase::Attacking.is_transition());

        assert!(GamePhase::Attacking.is_attacking());
        assert!(GamePhase::TransitionWin.is_attacking());
        assert!(!GamePhase::Defending.is_attacking());
    }
}
