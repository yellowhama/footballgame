//! Goalkeeper UnderPressure State Logic
//!
//! Handles quick outlet posture when pressured.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperUnderPressureState;

impl StateBehavior for GoalkeeperUnderPressureState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.5)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::UnderPressure)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 40 || ctx.local_pressure < 0.3
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Goalkeeper(GoalkeeperSubState::Distributing)
        } else {
            PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive)
        }
    }
}
