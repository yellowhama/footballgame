//! Forward Pressing State Logic
//!
//! Handles first-line pressing behavior.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{ForwardSubState, PositionSubState};

pub struct ForwardPressingState;

impl StateBehavior for ForwardPressingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 60
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        }
    }
}
