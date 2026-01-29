//! Defender TrackingBack State Logic
//!
//! Handles sprint recovery toward the defensive line.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

pub struct DefenderTrackingBackState;

impl StateBehavior for DefenderTrackingBackState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        // Sprint toward own goal to recover shape
        let dir = direction_to(ctx.player_position, ctx.own_goal);
        (dir.0 * 7.0, dir.1 * 7.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.at_defensive_position
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::HoldingLine)
    }
}
