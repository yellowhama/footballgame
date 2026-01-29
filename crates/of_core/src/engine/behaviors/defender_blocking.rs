//! Defender Blocking State Logic
//!
//! Handles stepping into the shot lane to block.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

pub struct DefenderBlockingState;

impl StateBehavior for DefenderBlockingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 3.0, dir.1 * 3.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 10
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.at_defensive_position {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        } else if ctx.high_danger {
            PositionSubState::Defender(DefenderSubState::TrackingBack)
        } else {
            PositionSubState::Defender(DefenderSubState::Covering)
        }
    }
}
