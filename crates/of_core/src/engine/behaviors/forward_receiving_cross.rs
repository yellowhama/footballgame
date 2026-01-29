//! Forward ReceivingCross State Logic
//!
//! Handles runs to the predicted cross landing spot.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{ForwardSubState, PositionSubState};

pub struct ForwardReceivingCrossState;

impl StateBehavior for ForwardReceivingCrossState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.predicted_cross_landing);
        (dir.0 * 7.0, dir.1 * 7.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.cross_resolved || ctx.in_substate_ticks > 40
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Forward(ForwardSubState::Finishing)
        } else {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        }
    }
}
