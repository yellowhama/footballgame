//! Goalkeeper PickingUpBall State Logic
//!
//! Handles ball pickup before holding.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperPickingUpBallState;

impl StateBehavior for GoalkeeperPickingUpBallState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::PickingUpBall)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 15
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall)
    }
}
