//! Position-specific behavior handlers
//!
//! Inspired by Open Football's StateProcessingHandler pattern.
//! Each position (GK, DEF, MID, FWD) has a dedicated handler that:
//! - Evaluates fast state transitions (O(1) per tick)
//! - Calculates velocity modifiers based on current substate
//! - Manages timeout-based transitions

use crate::engine::position_substates::PositionSubState;
use crate::engine::physics_constants::field;
use crate::engine::types::coord10::Coord10;
use crate::models::player::Position;
use crate::engine::behaviors::levers::BehaviorLevers;

/// Context for position-specific behavior calculations
///
/// Contains all information needed to make substate decisions.
/// This is a read-only view of the game state from a player's perspective.
///
/// FIX_2601 Phase 3.4: Position fields migrated to Coord10
#[derive(Debug, Clone)]
pub struct PositionContext {
    // Player identification
    pub player_idx: usize,
    pub position: Position,
    pub is_home: bool, // FIX_2512 Phase 16: Added for CoreContext compatibility
    pub attacks_right: bool,

    // Current state
    pub current_substate: PositionSubState,
    pub in_substate_ticks: u16,

    // Spatial awareness (FIX_2601: Coord10 for positions, f32 for velocities/directions)
    pub player_position: Coord10,
    pub ball_position: Coord10,
    pub ball_velocity: (f32, f32),  // velocity - keep as f32
    pub ball_distance: f32,         // scalar distance - keep as f32 for now
    pub goal_distance: f32,         // scalar distance - keep as f32 for now
    pub own_goal: Coord10,

    // Game context
    pub team_has_ball: bool,
    pub player_has_ball: bool,
    pub local_pressure: f32, // 0.0 - 1.0

    // Goalkeeper-specific
    pub shot_incoming: bool,
    pub ball_in_danger_zone: bool,
    pub goal_position: Coord10,

    // Defender-specific
    pub opponent_with_ball_nearby: bool,
    pub aerial_ball_incoming: bool,
    pub ball_in_own_box: bool,
    pub high_danger: bool,
    pub instruction_high_line: bool,
    pub defensive_line_y: f32,
    pub at_defensive_position: bool,

    // Midfielder-specific
    pub clear_shot: bool,
    pub runner_to_track: Option<RunnerInfo>,
    pub crowded_area: bool,
    pub least_crowded_direction: (f32, f32),  // direction vector - keep as f32
    pub separation_force: (f32, f32),          // force vector - keep as f32

    // Forward-specific
    pub in_scoring_position: bool,
    pub cross_incoming: bool,
    pub in_box: bool,
    pub can_break_offside: bool,
    pub team_attacking: bool,
    pub ball_coming_to_me: bool,
    pub back_to_goal: bool,
    pub in_pressing_zone: bool,
    pub offside_detected: bool,
    pub cross_resolved: bool,
    pub predicted_cross_landing: Coord10,
    pub best_space_direction: (f32, f32),      // direction vector - keep as f32

    // Phase 2: Tactical Levers
    pub levers: BehaviorLevers,
}

/// Information about a runner to track (for midfielders)
#[derive(Debug, Clone, Copy)]
pub struct RunnerInfo {
    pub player_idx: usize,
    pub position: Coord10,       // FIX_2601: position → Coord10
    pub velocity: (f32, f32),    // velocity - keep as f32
}

impl Default for PositionContext {
    fn default() -> Self {
        Self {
            player_idx: 0,
            position: Position::CM,
            is_home: true, // FIX_2512 Phase 16
            attacks_right: true,
            current_substate: PositionSubState::default(),
            in_substate_ticks: 0,
            player_position: Coord10::ZERO,                    // FIX_2601
            ball_position: Coord10::ZERO,                      // FIX_2601
            ball_velocity: (0.0, 0.0),
            ball_distance: 0.0,
            goal_distance: 0.0,
            own_goal: Coord10::from_meters(0.0, field::CENTER_Y), // FIX_2601: Home goal
            team_has_ball: false,
            player_has_ball: false,
            local_pressure: 0.0,
            shot_incoming: false,
            ball_in_danger_zone: false,
            goal_position: Coord10::from_meters(0.0, field::CENTER_Y), // FIX_2601: Home goal
            opponent_with_ball_nearby: false,
            aerial_ball_incoming: false,
            ball_in_own_box: bool::default(),
            high_danger: false,
            instruction_high_line: false,
            defensive_line_y: 20.0,
            at_defensive_position: false,
            clear_shot: false,
            runner_to_track: None,
            crowded_area: false,
            least_crowded_direction: (0.0, 0.0),
            separation_force: (0.0, 0.0),
            in_scoring_position: false,
            cross_incoming: false,
            in_box: false,
            can_break_offside: false,
            team_attacking: false,
            ball_coming_to_me: false,
            back_to_goal: false,
            in_pressing_zone: false,
            offside_detected: false,
            cross_resolved: false,
            predicted_cross_landing: Coord10::ZERO,            // FIX_2601
            best_space_direction: (0.0, 0.0),
            levers: BehaviorLevers::default(),
        }
    }
}

impl PositionContext {
    /// FIX_2512 Phase 16: Extract CoreContext from PositionContext
    ///
    /// Creates a CoreContext containing the shared fields.
    /// Used when building SlowContext from PositionContext.
    ///
    /// FIX_2601 Phase 3.4: Now uses Coord10 directly (no conversion needed)
    #[inline]
    pub fn to_core(&self) -> crate::engine::CoreContext {
        crate::engine::CoreContext {
            player_idx: self.player_idx,
            is_home: self.is_home,
            attacks_right: self.attacks_right,
            player_position: self.player_position,  // FIX_2601: already Coord10
            ball_position: self.ball_position,      // FIX_2601: already Coord10
            ball_distance: self.ball_distance,
            team_has_ball: self.team_has_ball,
            player_has_ball: self.player_has_ball,
        }
    }

    /// FIX_2601: Helper to get player position as meters tuple
    #[inline]
    pub fn player_position_m(&self) -> (f32, f32) {
        self.player_position.to_meters()
    }

    /// FIX_2601: Helper to get ball position as meters tuple
    #[inline]
    pub fn ball_position_m(&self) -> (f32, f32) {
        self.ball_position.to_meters()
    }

    /// FIX_2601: Helper to get own goal as meters tuple
    #[inline]
    pub fn own_goal_m(&self) -> (f32, f32) {
        self.own_goal.to_meters()
    }

    /// FIX_2601: Helper to get goal position as meters tuple
    #[inline]
    pub fn goal_position_m(&self) -> (f32, f32) {
        self.goal_position.to_meters()
    }
}

/// Interface for position-specific behavior handlers
///
/// Inspired by Open Football's StateProcessingHandler.
/// Each handler must be stateless and O(1) per tick for performance.
pub trait PositionStateHandler: Send + Sync {
    /// Fast path: check for immediate state transitions
    ///
    /// Called every tick. Must be O(1) complexity.
    /// Returns `Some(new_state)` if transition should occur, `None` otherwise.
    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState>;

    /// Calculate velocity modifier for current substate
    ///
    /// Returns (dx, dy) adjustment to apply on top of base positioning velocity.
    /// Positive y = toward opponent goal, negative y = toward own goal.
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32);

    /// Check if current substate should timeout
    ///
    /// Based on time in state or external conditions (e.g., ball cleared).
    fn should_timeout(&self, ctx: &PositionContext) -> bool;

    /// Get timeout transition target
    ///
    /// Called when `should_timeout()` returns true.
    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState;

    /// Get default substate for this position
    fn default_substate(&self) -> PositionSubState;

    /// FIX_2512 Phase 13: Complex decision-making (called less frequently)
    ///
    /// This method handles evaluations that are too expensive for every tick:
    /// - Pass target selection (distance + space + path + pressure)
    /// - Penetration timing (offside line + CB gap + ball holder distance)
    /// - Pressing triggers (support + zone + opponent crowding)
    ///
    /// Default implementation returns empty result (no evaluation).
    fn process_slow(
        &self,
        _ctx: &crate::engine::slow_evaluation::SlowContext,
    ) -> crate::engine::slow_evaluation::SlowEvaluationResult {
        crate::engine::slow_evaluation::SlowEvaluationResult::default()
    }
}

/// Normalize a 2D vector
#[inline]
pub fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 0.0001 {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

/// Calculate direction from point A to point B (Coord10 version)
/// FIX_2601: Primary version that takes Coord10 directly
#[inline]
pub fn direction_to(from: Coord10, to: Coord10) -> (f32, f32) {
    let dx = (to.x - from.x) as f32;
    let dy = (to.y - from.y) as f32;
    normalize((dx, dy))
}

/// Calculate direction from point A to point B (meters version)
/// FIX_2601: Backward compatibility version for code still using meters
#[inline]
pub fn direction_to_m(from: (f32, f32), to: (f32, f32)) -> (f32, f32) {
    normalize((to.0 - from.0, to.1 - from.1))
}

/// Calculate distance between two Coord10 points (returns meters)
/// FIX_2601: Primary version that takes Coord10 directly
#[inline]
pub fn distance(a: Coord10, b: Coord10) -> f32 {
    let dx = (b.x - a.x) as f32 / Coord10::SCALE;
    let dy = (b.y - a.y) as f32 / Coord10::SCALE;
    (dx * dx + dy * dy).sqrt()
}

/// Calculate distance between two points in meters
/// FIX_2601: Backward compatibility version for code still using meters
#[inline]
pub fn distance_m(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let v = normalize((3.0, 4.0));
        assert!((v.0 - 0.6).abs() < 0.001);
        assert!((v.1 - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_normalize_zero() {
        let v = normalize((0.0, 0.0));
        assert_eq!(v, (0.0, 0.0));
    }

    #[test]
    fn test_direction_to_coord10() {
        // FIX_2601: Test Coord10 version
        let from = Coord10::from_meters(0.0, 0.0);
        let to = Coord10::from_meters(3.0, 4.0);
        let dir = direction_to(from, to);
        assert!((dir.0 - 0.6).abs() < 0.001);
        assert!((dir.1 - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_direction_to_m() {
        // FIX_2601: Test meters version
        let dir = direction_to_m((0.0, 0.0), (3.0, 4.0));
        assert!((dir.0 - 0.6).abs() < 0.001);
        assert!((dir.1 - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_distance_coord10() {
        // FIX_2601: Test Coord10 version
        let a = Coord10::from_meters(0.0, 0.0);
        let b = Coord10::from_meters(3.0, 4.0);
        let d = distance(a, b);
        assert!((d - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_distance_m() {
        // FIX_2601: Test meters version
        let d = distance_m((0.0, 0.0), (3.0, 4.0));
        assert!((d - 5.0).abs() < 0.001);
    }
}
