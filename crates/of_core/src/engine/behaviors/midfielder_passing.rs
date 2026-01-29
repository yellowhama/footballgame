//! Midfielder Passing State Logic
//!
//! Minimal movement while executing a pass.

use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{MidfielderSubState, PositionSubState};

pub struct MidfielderPassingState;

impl StateBehavior for MidfielderPassingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        }
    }
}
