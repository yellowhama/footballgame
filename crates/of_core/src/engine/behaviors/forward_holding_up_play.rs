//! Forward HoldingUpPlay State Logic
//!
//! Handles shielding the ball with back to goal.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{ForwardSubState, PositionSubState};

pub struct ForwardHoldingUpPlayState;

impl StateBehavior for ForwardHoldingUpPlayState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        // Shield ball, slight backward lean to protect
        (0.0, -0.5)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 30 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}
