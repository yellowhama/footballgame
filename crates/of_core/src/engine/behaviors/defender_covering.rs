//! Defender Covering State Logic
//!
//! Positions between ball and own goal to protect space.

use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{DefenderSubState, PositionSubState};
use crate::engine::steering;

pub struct DefenderCoveringState;

impl StateBehavior for DefenderCoveringState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let cover_x = (ctx.ball_position.0 + ctx.own_goal.0) / 2.0;
        let cover_y = (ctx.ball_position.1 + ctx.own_goal.1) / 2.0;
        let base = steering::arrive(ctx.player_position, (cover_x, cover_y), 4.0, 6.0);
        let separation = ctx.separation_force;
        (base.0 + separation.0, base.1 + separation.1)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        if ctx.opponent_with_ball_nearby
            && ctx.ball_distance < ctx.levers.press_trigger_distance
        {
            return Some(PositionSubState::Defender(DefenderSubState::Pressing));
        }
        None
    }

    fn should_timeout(&self, _ctx: &PositionContext) -> bool {
        false
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::Covering)
    }
}
