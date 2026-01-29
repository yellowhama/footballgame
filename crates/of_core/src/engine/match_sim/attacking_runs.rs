//! Attacking Runs Module
//!
//! FIX_2601/0110: Types of attacking runs with timing and movement patterns.
//! Implements Late Box Runs and other penetrating movements.
//!
//! Key features:
//! - 5 attacking run types (ThroughBall, Overlap, LateBoxRun, Support, Diagonal)
//! - Late run timing system (hold position until trigger)
//! - Run-specific target calculation

use crate::engine::movement::PositionRole;
use crate::engine::types::{BallZone, Coord10};

/// Types of attacking runs with different timing/movement patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackingRunType {
    /// Through ball run - sprint behind defense
    ThroughBall,
    /// Overlap run - fullback overlap on wing
    OverlapRun,
    /// Late box run - enter box late to avoid offside trap
    LateBoxRun,
    /// Support run - check back toward ball holder
    SupportRun,
    /// Diagonal run - cut across defense
    DiagonalRun,
}

/// State for managing late box runs
#[derive(Debug, Clone, Default)]
pub struct LateRunState {
    /// Whether player is in "holding" phase (waiting for trigger)
    pub is_holding: bool,
    /// Tick when holding started
    pub hold_started: u32,
    /// Maximum hold time before abandoning
    pub max_hold_ticks: u32,
}

impl LateRunState {
    /// Default maximum hold time (~20 seconds at 4 ticks/sec)
    pub const DEFAULT_MAX_HOLD: u32 = 80;

    /// Create new late run state in holding phase
    pub fn start_holding(current_tick: u32) -> Self {
        Self {
            is_holding: true,
            hold_started: current_tick,
            max_hold_ticks: Self::DEFAULT_MAX_HOLD,
        }
    }

    /// Check if should trigger the run
    ///
    /// Triggers when:
    /// - Ball in attacking third AND ball is in flight (pass being made)
    /// - OR timeout reached (abandon hold)
    pub fn should_trigger(
        &self,
        current_tick: u32,
        ball_zone: BallZone,
        ball_is_in_flight: bool,
        player_in_box: bool,
    ) -> bool {
        // Already in box - don't re-trigger
        if player_in_box {
            return false;
        }

        // Timeout - abandon hold
        if current_tick.saturating_sub(self.hold_started) > self.max_hold_ticks {
            return true; // Trigger to reset
        }

        // Trigger conditions:
        // Ball in attacking third AND ball is being played (pass in progress)
        ball_zone == BallZone::Attacking && ball_is_in_flight
    }

    /// End holding phase
    pub fn trigger(&mut self) {
        self.is_holding = false;
    }

    /// Reset for new run
    pub fn reset(&mut self, current_tick: u32) {
        self.is_holding = true;
        self.hold_started = current_tick;
    }
}

/// Determine best attacking run type based on context
///
/// # Arguments
/// * `player_pos` - Current player position
/// * `ball_pos` - Current ball position
/// * `player_role` - Player's position role (Forward, Midfielder, Defender)
/// * `ball_zone` - Current zone of the ball
/// * `is_wide_player` - Whether the player is in a wide position (LW/RW/LB/RB)
/// * `attack_direction` - +1.0 for attacking toward Y=1000, -1.0 for attacking toward Y=0
///   FIX_2601/0110: Added attack_direction parameter for correct 2nd half behavior
pub fn select_run_type(
    player_pos: Coord10,
    ball_pos: Coord10,
    player_role: PositionRole,
    ball_zone: BallZone,
    is_wide_player: bool, // LW/RW/LB/RB
    attack_direction: f32, // FIX_2601/0110: +1.0 or -1.0
) -> AttackingRunType {
    // Forwards in attacking third
    if player_role == PositionRole::Forward && ball_zone == BallZone::Attacking {
        // Wide forwards do diagonal runs
        if is_wide_player {
            return AttackingRunType::DiagonalRun;
        }
        // Central forwards do late box runs
        return AttackingRunType::LateBoxRun;
    }

    // Fullbacks do overlaps when ball is on their side
    if player_role == PositionRole::Defender && is_wide_player {
        let same_side = (player_pos.x < 500) == (ball_pos.x < 500);
        if same_side && ball_zone != BallZone::Defensive {
            return AttackingRunType::OverlapRun;
        }
    }

    // Midfielders and forwards behind ball do through ball runs
    if matches!(player_role, PositionRole::Forward | PositionRole::Midfielder) {
        // FIX_2601/0110: Use attack_direction to determine if player is behind ball
        // Player is "behind" the ball if they are in the opposite direction of attack
        let player_behind_ball = if attack_direction > 0.0 {
            player_pos.y < ball_pos.y // Attacking toward Y=1000: behind = lower Y
        } else {
            player_pos.y > ball_pos.y // Attacking toward Y=0: behind = higher Y
        };

        if player_behind_ball {
            return AttackingRunType::ThroughBall;
        }
    }

    // Default: support run
    AttackingRunType::SupportRun
}

/// Calculate target position for attacking run
///
/// # Arguments
/// * `run_type` - Type of run to execute
/// * `player_pos` - Current player position
/// * `ball_pos` - Current ball position
/// * `attack_direction` - +1.0 for home (toward Y=1000), -1.0 for away
pub fn calculate_run_target(
    run_type: AttackingRunType,
    player_pos: Coord10,
    ball_pos: Coord10,
    attack_direction: f32,
) -> Coord10 {
    match run_type {
        AttackingRunType::ThroughBall => {
            // Sprint behind defensive line
            let target_y = if attack_direction > 0.0 { 900 } else { 100 };
            Coord10 { x: player_pos.x, y: target_y, z: 0 }
        }

        AttackingRunType::OverlapRun => {
            // Run past ball carrier on the outside
            let wing_x = if player_pos.x < 500 { 80 } else { 920 };
            let ahead_y = ball_pos.y + (100.0 * attack_direction) as i32;
            Coord10 { x: wing_x, y: ahead_y.clamp(50, 950), z: 0 }
        }

        AttackingRunType::LateBoxRun => {
            // Target: 6-yard box area
            let box_y = if attack_direction > 0.0 { 920 } else { 80 };
            // Slight offset from center to create angle
            let offset = if player_pos.x < 500 { -80 } else { 80 };
            Coord10 { x: (500 + offset).clamp(350, 650), y: box_y, z: 0 }
        }

        AttackingRunType::SupportRun => {
            // Check back toward ball for support
            let back_y = ball_pos.y - (80.0 * attack_direction) as i32;
            // Offset laterally for passing angle
            let offset_x = if player_pos.x < ball_pos.x { -60 } else { 60 };
            Coord10 { x: (ball_pos.x + offset_x).clamp(100, 900), y: back_y.clamp(100, 900), z: 0 }
        }

        AttackingRunType::DiagonalRun => {
            // Cut across from wide to central
            let central_x = 500;
            let ahead_y = ball_pos.y + (150.0 * attack_direction) as i32;
            Coord10 { x: central_x, y: ahead_y.clamp(50, 950), z: 0 }
        }
    }
}

/// Check if player position is considered "in the box"
pub fn is_in_penalty_box(pos: Coord10, attack_direction: f32) -> bool {
    // Penalty box: X from 185 to 815 (roughly), Y from goal line to 16.5m
    let in_width = pos.x >= 185 && pos.x <= 815;

    if attack_direction > 0.0 {
        // Attacking toward Y=1000
        in_width && pos.y >= 835 // 16.5m from goal
    } else {
        // Attacking toward Y=0
        in_width && pos.y <= 165
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_late_run_timing() {
        let state = LateRunState::start_holding(100);

        // Should not trigger in midfield
        assert!(!state.should_trigger(110, BallZone::Midfield, false, false));

        // Should not trigger if ball not in flight
        assert!(!state.should_trigger(120, BallZone::Attacking, false, false));

        // Should trigger when ball in attacking third AND in flight
        assert!(state.should_trigger(130, BallZone::Attacking, true, false));

        // Should not trigger if already in box
        assert!(!state.should_trigger(140, BallZone::Attacking, true, true));
    }

    #[test]
    fn test_late_run_timeout() {
        let state = LateRunState::start_holding(100);

        // Before timeout
        assert!(!state.should_trigger(150, BallZone::Midfield, false, false));

        // After timeout
        assert!(state.should_trigger(200, BallZone::Midfield, false, false));
    }

    #[test]
    fn test_run_type_selection() {
        let ball = Coord10 { x: 500, y: 800, z: 0 };
        let attack_dir = 1.0; // Home team attacking toward Y=1000

        // Forward in attacking third -> Late box run
        let forward = Coord10 { x: 500, y: 750, z: 0 };
        let run = select_run_type(forward, ball, PositionRole::Forward, BallZone::Attacking, false, attack_dir);
        assert_eq!(run, AttackingRunType::LateBoxRun);

        // Wide forward -> Diagonal run
        let winger = Coord10 { x: 100, y: 750, z: 0 };
        let run = select_run_type(winger, ball, PositionRole::Forward, BallZone::Attacking, true, attack_dir);
        assert_eq!(run, AttackingRunType::DiagonalRun);

        // Fullback same side -> Overlap
        let fullback = Coord10 { x: 150, y: 400, z: 0 };
        let ball_left = Coord10 { x: 200, y: 500, z: 0 };
        let run =
            select_run_type(fullback, ball_left, PositionRole::Defender, BallZone::Midfield, true, attack_dir);
        assert_eq!(run, AttackingRunType::OverlapRun);
    }

    #[test]
    fn test_run_targets() {
        let player = Coord10 { x: 400, y: 600, z: 0 };
        let ball = Coord10 { x: 500, y: 700, z: 0 };
        let attack_dir = 1.0;

        // Through ball -> deep position
        let target = calculate_run_target(AttackingRunType::ThroughBall, player, ball, attack_dir);
        assert_eq!(target.y, 900);

        // Late box run -> near goal
        let target = calculate_run_target(AttackingRunType::LateBoxRun, player, ball, attack_dir);
        assert_eq!(target.y, 920);

        // Support run -> behind ball
        let target = calculate_run_target(AttackingRunType::SupportRun, player, ball, attack_dir);
        assert!(target.y < ball.y);
    }

    #[test]
    fn test_in_penalty_box() {
        // In box (attacking toward Y=1000)
        assert!(is_in_penalty_box(Coord10 { x: 500, y: 900, z: 0 }, 1.0));

        // Not in box (too far from goal)
        assert!(!is_in_penalty_box(Coord10 { x: 500, y: 700, z: 0 }, 1.0));

        // Not in box (outside width)
        assert!(!is_in_penalty_box(Coord10 { x: 100, y: 900, z: 0 }, 1.0));
    }
}
