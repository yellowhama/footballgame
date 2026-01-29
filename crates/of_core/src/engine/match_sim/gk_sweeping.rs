//! GK Sweeping/Rushing Module
//!
//! FIX_2601/0107: Goalkeeper sweeping and rushing out behavior based on open-football.
//!
//! Key features:
//! - Skill-based rushing decisions
//! - Optimal positioning calculation
//! - Reach-before-opponent estimation
//! - Danger area detection

use crate::engine::types::Coord10;

/// GK sweeping state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GKSweepingState {
    /// Normal positioning in goal
    Attentive,
    /// Rushing out to intercept ball/opponent
    ComingOut,
    /// Returning to goal position
    ReturningToGoal,
    /// Preparing for shot save
    PreparingForSave,
}

impl Default for GKSweepingState {
    fn default() -> Self {
        Self::Attentive
    }
}

/// GK mental skills for decision making
#[derive(Debug, Clone, Copy)]
pub struct GKMentalSkills {
    /// Decisions skill (0-100)
    pub decisions: u8,
    /// Positioning skill (0-100)
    pub positioning: u8,
    /// Anticipation skill (0-100)
    pub anticipation: u8,
    /// Bravery skill (0-100)
    pub bravery: u8,
}

impl Default for GKMentalSkills {
    fn default() -> Self {
        Self {
            decisions: 50,
            positioning: 50,
            anticipation: 50,
            bravery: 50,
        }
    }
}

/// GK physical skills for rushing
#[derive(Debug, Clone, Copy)]
pub struct GKPhysicalSkills {
    /// Pace/speed (0-100)
    pub pace: u8,
    /// Acceleration (0-100)
    pub acceleration: u8,
    /// Agility (0-100)
    pub agility: u8,
}

impl Default for GKPhysicalSkills {
    fn default() -> Self {
        Self {
            pace: 50,
            acceleration: 50,
            agility: 50,
        }
    }
}

/// Context for GK sweeping decisions
#[derive(Debug, Clone)]
pub struct SweepingContext {
    /// GK position
    pub gk_pos: Coord10,
    /// Ball position
    pub ball_pos: Coord10,
    /// Ball velocity (Coord10 units per tick)
    pub ball_velocity: (i32, i32),
    /// Whether ball is controlled by opponent
    pub opponent_has_ball: bool,
    /// Nearest opponent position (if any)
    pub nearest_opponent: Option<Coord10>,
    /// Goal center position
    pub goal_center: Coord10,
    /// Attack direction (-1.0 or 1.0)
    pub attack_direction: f32,
}

/// Constants for GK sweeping
pub mod constants {
    /// Base distance threshold for coming out (Coord10 units, ~10m)
    pub const BASE_COMING_OUT_DISTANCE: i32 = 100;
    /// Danger area width from goal (Coord10 units, ~20m)
    pub const DANGER_AREA_WIDTH: i32 = 200;
    /// Minimum ball speed to trigger rushing (Coord10 units per tick)
    pub const MIN_BALL_SPEED_THREAT: i32 = 50;
    /// Very close distance (must intercept, ~10m)
    pub const VERY_CLOSE_DISTANCE: i32 = 100;
    /// Penalty area depth (Coord10 units, ~16.5m)
    pub const PENALTY_AREA_DEPTH: i32 = 165;
}

/// Determine if GK should come out (rush)
///
/// Based on open-football's `should_come_out()` function.
///
/// # Arguments
/// * `ctx` - Sweeping context with positions and velocities
/// * `mental` - GK mental skills
/// * `physical` - GK physical skills
///
/// # Returns
/// (should_come_out, urgency_score)
pub fn should_come_out(
    ctx: &SweepingContext,
    mental: &GKMentalSkills,
    physical: &GKPhysicalSkills,
) -> (bool, f32) {
    // Calculate rushing skill (combination of decisions and positioning)
    let rushing_skill = (mental.decisions as f32 + mental.positioning as f32) / 200.0;

    // Adjust distance threshold based on skill (70-130% of base)
    let skill_factor = 0.7 + rushing_skill * 0.6;
    let adjusted_threshold =
        (constants::BASE_COMING_OUT_DISTANCE as f32 * skill_factor * 10.0) as i32;

    // Calculate ball distance from GK
    let ball_distance = ctx.gk_pos.distance_to(&ctx.ball_pos);

    // Calculate ball speed
    let ball_speed = ((ctx.ball_velocity.0.pow(2) + ctx.ball_velocity.1.pow(2)) as f32).sqrt();

    // Case 1: Very close, no possession (must intercept)
    if ball_distance < constants::VERY_CLOSE_DISTANCE && !ctx.opponent_has_ball {
        return (true, 1.0);
    }

    // Case 2: Ball moving toward goal at high speed
    let ball_moving_toward_goal = is_ball_moving_toward_goal(ctx);
    if ball_moving_toward_goal && ball_speed > constants::MIN_BALL_SPEED_THREAT as f32 {
        // Urgency based on distance and speed
        let urgency = (ball_speed / 100.0).min(1.0);
        return (true, urgency);
    }

    // Case 3: Loose ball in danger area
    if !ctx.opponent_has_ball && is_ball_in_danger_area(ctx) {
        let urgency = 0.7;
        return (true, urgency);
    }

    // Case 4: 1v1 situation - opponent approaching with ball
    if let Some(opponent_pos) = ctx.nearest_opponent {
        let opponent_distance = ctx.gk_pos.distance_to(&opponent_pos) as f32;

        // Check if can reach before opponent
        if opponent_distance < adjusted_threshold as f32 {
            let can_reach = can_reach_before_opponent(ctx, &opponent_pos, mental, physical);
            if can_reach {
                let urgency = 1.0 - (opponent_distance / adjusted_threshold as f32);
                return (true, urgency.clamp(0.3, 1.0));
            }
        }
    }

    // Default: stay in goal
    (false, 0.0)
}

/// Check if ball is moving toward goal
///
/// FIX_2601/0117: Fixed coordinate system - goals are at x=0 and x=1000, not y.
fn is_ball_moving_toward_goal(ctx: &SweepingContext) -> bool {
    // Check ball velocity X component toward goal
    // If defending left goal (x=0), ball moving toward goal means vx < 0
    // If defending right goal (x=1000), ball moving toward goal means vx > 0
    let defending_left_goal = ctx.goal_center.x < 500;
    if defending_left_goal {
        ctx.ball_velocity.0 < 0
    } else {
        ctx.ball_velocity.0 > 0
    }
}

/// Check if ball is in danger area (near goal)
///
/// FIX_2601/0117: Fixed coordinate system - goals are at x=0 and x=1050, not y.
fn is_ball_in_danger_area(ctx: &SweepingContext) -> bool {
    let goal_x = ctx.goal_center.x;
    let field_length = Coord10::FIELD_LENGTH_10; // 1050
    let defending_left_goal = goal_x < field_length / 2;

    if defending_left_goal {
        // Defending left goal (x=0): danger area is x < DANGER_AREA_WIDTH
        ctx.ball_pos.x < constants::DANGER_AREA_WIDTH
    } else {
        // Defending right goal (x=1050): danger area is x > 1050 - DANGER_AREA_WIDTH
        ctx.ball_pos.x > field_length - constants::DANGER_AREA_WIDTH
    }
}

/// Check if GK can reach ball before opponent
///
/// Based on open-football's `can_reach_before_opponent()`.
fn can_reach_before_opponent(
    ctx: &SweepingContext,
    opponent_pos: &Coord10,
    mental: &GKMentalSkills,
    physical: &GKPhysicalSkills,
) -> bool {
    // GK speed with acceleration bonus
    let gk_speed = physical.acceleration as f32 * 1.2;

    // Assume opponent speed (using average)
    let opponent_speed = 70.0; // Average pace

    // Calculate distances
    let dist_gk_to_ball = ctx.gk_pos.distance_to(&ctx.ball_pos) as f32;
    let dist_opponent_to_ball = opponent_pos.distance_to(&ctx.ball_pos) as f32;

    // Calculate time to reach ball
    let time_gk = if gk_speed > 0.0 {
        dist_gk_to_ball / gk_speed
    } else {
        f32::MAX
    };
    let time_opponent = if opponent_speed > 0.0 {
        dist_opponent_to_ball / opponent_speed
    } else {
        f32::MAX
    };

    // Decision advantage (better decisions = react faster)
    let decision_advantage = mental.decisions as f32 / 200.0; // 0 to 0.5

    // GK wins if they can reach in less time (with decision advantage)
    time_gk < time_opponent * (1.0 - decision_advantage)
}

/// Calculate optimal GK position on the arc
///
/// Positions GK on line between goal center and ball.
///
/// # Arguments
/// * `goal_center` - Goal center position
/// * `ball_pos` - Ball position
/// * `positioning_skill` - GK positioning skill (0-100)
///
/// # Returns
/// Optimal position on the goal arc
pub fn calculate_optimal_position(
    goal_center: Coord10,
    ball_pos: Coord10,
    positioning_skill: u8,
) -> Coord10 {
    // Calculate direction from goal to ball
    let dx = ball_pos.x - goal_center.x;
    let dy = ball_pos.y - goal_center.y;
    let dist = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);

    // Distance from goal based on positioning skill
    // Better positioning = further out to cut angles
    let skill_factor = 0.8 + (positioning_skill as f32 / 100.0) * 0.4; // 0.8 to 1.2
    let optimal_distance = (50.0 * skill_factor) as i32; // 40-60 Coord10 units (~4-6m)

    // Calculate position on line between goal and ball
    let nx = (dx as f32 / dist * optimal_distance as f32) as i32;
    let ny = (dy as f32 / dist * optimal_distance as f32) as i32;

    // FIX_2601/0117: Clamp to penalty area based on which goal is being defended
    // Coord10: x = 0-1050 (length), y = 0-680 (width)
    // Left goal at x=0, Right goal at x=1050
    // Penalty area depth is ~165 Coord10 units (~16.5m)
    // Penalty area width is roughly 185-495 (goal area center ± some margin)
    let field_length = Coord10::FIELD_LENGTH_10; // 1050
    let defending_left_goal = goal_center.x < field_length / 2;
    let (x_min, x_max) = if defending_left_goal {
        // GK defending left goal (x=0): stay within x=0 to penalty area depth
        (0, constants::PENALTY_AREA_DEPTH)
    } else {
        // GK defending right goal (x=1050): stay within penalty area depth to x=1050
        (field_length - constants::PENALTY_AREA_DEPTH, field_length)
    };

    // Y clamped to penalty area width (centered around goal)
    // Goal is at y=340 (center), penalty area extends ~200 units each side
    let field_width = Coord10::FIELD_WIDTH_10; // 680
    let y_center = field_width / 2; // 340
    let y_margin = 200; // ~20m each side of center

    Coord10 {
        x: (goal_center.x + nx).clamp(x_min, x_max),
        y: (goal_center.y + ny).clamp(y_center - y_margin, y_center + y_margin),
        z: 0,
    }
}

/// Calculate rushing target position
///
/// When GK decides to come out, calculate where to rush to.
pub fn calculate_rushing_target(ctx: &SweepingContext, physical: &GKPhysicalSkills) -> Coord10 {
    // If opponent has ball, intercept the opponent
    if let Some(opponent_pos) = ctx.nearest_opponent {
        if ctx.opponent_has_ball {
            // Rush toward opponent
            return predict_interception_point(ctx.gk_pos, opponent_pos, physical);
        }
    }

    // Otherwise rush toward ball
    predict_interception_point(ctx.gk_pos, ctx.ball_pos, physical)
}

/// Predict interception point
fn predict_interception_point(
    gk_pos: Coord10,
    target_pos: Coord10,
    physical: &GKPhysicalSkills,
) -> Coord10 {
    // Simple prediction: move toward target with speed consideration
    let gk_speed = physical.pace as f32;

    let dx = target_pos.x - gk_pos.x;
    let dy = target_pos.y - gk_pos.y;
    let dist = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);

    // Move at most half the distance per decision
    let max_move = (gk_speed / 2.0).min(dist);

    Coord10 {
        x: gk_pos.x + (dx as f32 / dist * max_move) as i32,
        y: gk_pos.y + (dy as f32 / dist * max_move) as i32,
        z: 0,
    }
}

/// Determine next GK state based on current situation
pub fn determine_next_state(
    current_state: GKSweepingState,
    ctx: &SweepingContext,
    mental: &GKMentalSkills,
    physical: &GKPhysicalSkills,
) -> GKSweepingState {
    match current_state {
        GKSweepingState::Attentive => {
            let (should_rush, _urgency) = should_come_out(ctx, mental, physical);
            if should_rush {
                GKSweepingState::ComingOut
            } else {
                GKSweepingState::Attentive
            }
        }
        GKSweepingState::ComingOut => {
            let ball_distance = ctx.gk_pos.distance_to(&ctx.ball_pos);

            // Close enough to prepare for save
            if ball_distance < constants::VERY_CLOSE_DISTANCE {
                return GKSweepingState::PreparingForSave;
            }

            // Ball moved away, return to goal
            if !is_ball_in_danger_area(ctx) {
                return GKSweepingState::ReturningToGoal;
            }

            GKSweepingState::ComingOut
        }
        GKSweepingState::ReturningToGoal => {
            let goal_distance = ctx.gk_pos.distance_to(&ctx.goal_center);

            // Back in position (50 Coord10 units = ~5m)
            if goal_distance < 50 {
                return GKSweepingState::Attentive;
            }

            // New threat while returning
            let (should_rush, _) = should_come_out(ctx, mental, physical);
            if should_rush {
                return GKSweepingState::ComingOut;
            }

            GKSweepingState::ReturningToGoal
        }
        GKSweepingState::PreparingForSave => {
            // Stay in save preparation until ball is dealt with
            if !is_ball_in_danger_area(ctx) {
                return GKSweepingState::ReturningToGoal;
            }
            GKSweepingState::PreparingForSave
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // FIX_2601/0117: Updated tests for correct coordinate system
    // - Field is 1050x680 Coord10 units (105m x 68m)
    // - Goals are at x=0 (left) and x=1050 (right)
    // - GK defends by staying near their goal_center.x

    fn default_context_left_goal() -> SweepingContext {
        // GK defending left goal (x=0)
        // FIX_2601/0117: Use correct field dimensions (1050x680)
        SweepingContext {
            gk_pos: Coord10 { x: 50, y: 340, z: 0 },     // GK near left goal
            ball_pos: Coord10 { x: 300, y: 340, z: 0 },   // Ball in midfield
            ball_velocity: (0, 0),
            opponent_has_ball: false,
            nearest_opponent: None,
            goal_center: Coord10 { x: 0, y: 340, z: 0 },  // Left goal (x=0, y=center)
            attack_direction: 1.0, // Opponents attack toward x=0
        }
    }

    fn default_context_right_goal() -> SweepingContext {
        // GK defending right goal (x=1050)
        // FIX_2601/0117: Use correct field dimensions (1050x680)
        SweepingContext {
            gk_pos: Coord10 { x: 1000, y: 340, z: 0 },    // GK near right goal
            ball_pos: Coord10 { x: 700, y: 340, z: 0 },   // Ball in midfield
            ball_velocity: (0, 0),
            opponent_has_ball: false,
            nearest_opponent: None,
            goal_center: Coord10 { x: 1050, y: 340, z: 0 }, // Right goal (x=1050, y=center)
            attack_direction: -1.0, // Opponents attack toward x=1050
        }
    }

    #[test]
    fn test_should_not_come_out_when_ball_far() {
        let ctx = default_context_left_goal();
        let mental = GKMentalSkills::default();
        let physical = GKPhysicalSkills::default();

        let (should_rush, _) = should_come_out(&ctx, &mental, &physical);
        assert!(!should_rush);
    }

    #[test]
    fn test_should_come_out_for_very_close_ball() {
        let mut ctx = default_context_left_goal();
        ctx.ball_pos = Coord10 { x: 80, y: 340, z: 0 }; // Very close to GK at x=50

        let mental = GKMentalSkills::default();
        let physical = GKPhysicalSkills::default();

        let (should_rush, urgency) = should_come_out(&ctx, &mental, &physical);
        assert!(should_rush);
        assert_eq!(urgency, 1.0);
    }

    #[test]
    fn test_should_come_out_for_loose_ball_in_danger() {
        let mut ctx = default_context_left_goal();
        ctx.ball_pos = Coord10 { x: 150, y: 340, z: 0 }; // In danger area (x < 200)
        ctx.opponent_has_ball = false;

        let mental = GKMentalSkills::default();
        let physical = GKPhysicalSkills::default();

        let (should_rush, _) = should_come_out(&ctx, &mental, &physical);
        assert!(should_rush);
    }

    #[test]
    fn test_optimal_positioning_left_goal() {
        // GK defending left goal (x=0)
        // FIX_2601/0117: Use correct field dimensions (1050x680)
        let goal = Coord10 { x: 0, y: 340, z: 0 };
        let ball = Coord10 { x: 300, y: 400, z: 0 };

        let pos = calculate_optimal_position(goal, ball, 70);

        // Should be between goal and ball in X direction
        assert!(pos.x > goal.x);
        assert!(pos.x < ball.x);
        // Should be clamped to penalty area depth
        assert!(pos.x <= constants::PENALTY_AREA_DEPTH);
    }

    #[test]
    fn test_optimal_positioning_right_goal() {
        // GK defending right goal (x=1050)
        // FIX_2601/0117: Use correct field dimensions (1050x680)
        let goal = Coord10 { x: 1050, y: 340, z: 0 };
        let ball = Coord10 { x: 700, y: 300, z: 0 };

        let pos = calculate_optimal_position(goal, ball, 70);

        // Should be between goal and ball in X direction
        assert!(pos.x < goal.x);
        assert!(pos.x > ball.x);
        // Should be clamped to penalty area depth from right goal
        assert!(pos.x >= Coord10::FIELD_LENGTH_10 - constants::PENALTY_AREA_DEPTH);
    }

    #[test]
    fn test_state_transitions() {
        let mut ctx = default_context_left_goal();
        let mental = GKMentalSkills::default();
        let physical = GKPhysicalSkills::default();

        // Attentive with far ball -> stay Attentive
        let next = determine_next_state(GKSweepingState::Attentive, &ctx, &mental, &physical);
        assert_eq!(next, GKSweepingState::Attentive);

        // Attentive with close ball -> ComingOut
        ctx.ball_pos = Coord10 { x: 80, y: 340, z: 0 };
        let next = determine_next_state(GKSweepingState::Attentive, &ctx, &mental, &physical);
        assert_eq!(next, GKSweepingState::ComingOut);
    }

    #[test]
    fn test_ball_moving_toward_left_goal() {
        let mut ctx = default_context_left_goal();

        // Ball moving toward left goal (x=0), so vx < 0
        ctx.ball_velocity = (-50, 0);
        assert!(is_ball_moving_toward_goal(&ctx));

        // Ball moving away from left goal
        ctx.ball_velocity = (50, 0);
        assert!(!is_ball_moving_toward_goal(&ctx));
    }

    #[test]
    fn test_ball_moving_toward_right_goal() {
        let mut ctx = default_context_right_goal();

        // Ball moving toward right goal (x=1050), so vx > 0
        ctx.ball_velocity = (50, 0);
        assert!(is_ball_moving_toward_goal(&ctx));

        // Ball moving away from right goal
        ctx.ball_velocity = (-50, 0);
        assert!(!is_ball_moving_toward_goal(&ctx));
    }

    #[test]
    fn test_ball_in_danger_area_left_goal() {
        let mut ctx = default_context_left_goal();

        // Ball in danger area (x < 200)
        ctx.ball_pos = Coord10 { x: 150, y: 340, z: 0 };
        assert!(is_ball_in_danger_area(&ctx));

        // Ball out of danger area
        ctx.ball_pos = Coord10 { x: 300, y: 340, z: 0 };
        assert!(!is_ball_in_danger_area(&ctx));
    }

    #[test]
    fn test_ball_in_danger_area_right_goal() {
        let mut ctx = default_context_right_goal();

        // Ball in danger area (x > 1050 - 200 = 850)
        ctx.ball_pos = Coord10 { x: 900, y: 340, z: 0 };
        assert!(is_ball_in_danger_area(&ctx));

        // Ball out of danger area
        ctx.ball_pos = Coord10 { x: 700, y: 340, z: 0 };
        assert!(!is_ball_in_danger_area(&ctx));
    }
}
