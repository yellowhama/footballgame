//! Goalkeeper Positioning State Logic
//!
//! Handles lateral adjustments on the goal line.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperPositioningState;

impl StateBehavior for GoalkeeperPositioningState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let goal_center_x = ctx.goal_position.0;
        let ball_offset = ctx.ball_position.0 - goal_center_x;
        let target_x = goal_center_x + ball_offset * 0.3;
        let dx = (target_x - ctx.player_position.0).clamp(-3.0, 3.0);
        let dy = (ctx.goal_position.1 - ctx.player_position.1) * 0.3;

        (dx, dy.clamp(-1.0, 1.0))
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Positioning)
    }

    fn should_timeout(&self, _ctx: &PositionContext) -> bool {
        false
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive)
    }
}
