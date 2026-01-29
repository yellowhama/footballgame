//! Forward Assisting State Logic
//!
//! Handles support play around the ball.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{ForwardSubState, PositionSubState};

pub struct ForwardAssistingState;

impl StateBehavior for ForwardAssistingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 40 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}
