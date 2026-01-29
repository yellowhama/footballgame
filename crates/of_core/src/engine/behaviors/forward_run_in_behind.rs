//! Forward Running In Behind State Logic
//!
//! Handles the behavior of making runs behind the defensive line.

use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{ForwardSubState, PositionSubState};
use crate::engine::steering::{self, FollowPathState};

pub struct ForwardRunInBehindState;

impl StateBehavior for ForwardRunInBehindState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        // Sprint behind defensive line
        // Target: far post area or space behind defenders
        // FIX_2512_1230: Use team-relative goal direction
        // Home attacks toward x=105, Away attacks toward x=0
        use crate::engine::physics_constants::field;
        let attack_goal_x = if ctx.attacks_right { field::LENGTH_M } else { 0.0 };

        // Target X: 10m before opponent goal line
        let target_x = if ctx.attacks_right {
            attack_goal_x - 10.0  // 95.0 for Home
        } else {
            attack_goal_x + 10.0  // 10.0 for Away
        };

        // Target Y: near post or far post based on current position
        let target_y = if ctx.player_position.1 < field::CENTER_Y {
            25.0 // Near post run (lower Y)
        } else {
            43.0 // Far post run (higher Y)
        };

        let target = (target_x, target_y);
        let dx = target.0 - ctx.player_position.0;
        let dy = target.1 - ctx.player_position.1;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > 12.0 {
            let perp = (-dy / dist, dx / dist);
            let offset = (dist * 0.2).clamp(3.0, 8.0);
            let mid = (
                (ctx.player_position.0 + target.0) * 0.5,
                (ctx.player_position.1 + target.1) * 0.5,
            );
            let waypoint = clamp_to_field(
                (
                    mid.0 + perp.0 * offset,
                    mid.1 + perp.1 * offset,
                ),
                field::LENGTH_M,
                field::WIDTH_M,
            );
            let waypoints = [waypoint, target];
            let mut follow = FollowPathState::new(false);
            return follow.step(ctx.player_position, &waypoints, 8.0, 3.0);
        }

        steering::arrive(ctx.player_position, target, 8.0, 4.0) // Max sprint
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        // Evaluate if we should abort the run
        // Example: If ball goes out of play, or team loses possession?
        // Currently handled by timeout/global logic mostly
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        // Timeout if run takes too long or offside
        // FIX_2601/0123: Away team gets shorter timeout to reduce offside bias
        // Away team had ~2x more offsides (64.6% vs 35.4%), so shorter runs help
        let timeout_ticks = if ctx.is_home { 40 } else { 32 };  // Away gets 20% shorter timeout
        ctx.in_substate_ticks > timeout_ticks || ctx.offside_detected
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
         if ctx.offside_detected {
            // Got caught offside, drop back
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            // Run completed without receiving, create space
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        }
    }
}

fn clamp_to_field(pos: (f32, f32), length_m: f32, width_m: f32) -> (f32, f32) {
    let min_x = length_m * 0.05;
    let max_x = length_m * 0.95;
    let min_y = width_m * 0.05;
    let max_y = width_m * 0.95;
    (pos.0.clamp(min_x, max_x), pos.1.clamp(min_y, max_y))
}
