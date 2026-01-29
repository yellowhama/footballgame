//! Goalkeeper Save Action State Logic
//!
//! Handles Diving/Catching/Punching/Jumping actions.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperSaveActionState;

impl StateBehavior for GoalkeeperSaveActionState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall)
        } else {
            PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive)
        }
    }
}
