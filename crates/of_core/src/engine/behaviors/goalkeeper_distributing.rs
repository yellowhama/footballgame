//! Goalkeeper Distributing State Logic
//!
//! Handles quick distribution after a save.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperDistributingState;

impl StateBehavior for GoalkeeperDistributingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let forward = if ctx.attacks_right { 1.0 } else { -1.0 };
        (forward, 0.0)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Distributing)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 60
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive)
    }
}
