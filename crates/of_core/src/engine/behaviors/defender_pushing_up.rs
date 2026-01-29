//! Defender PushingUp State Logic
//!
//! Handles stepping the line forward when in possession.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

pub struct DefenderPushingUpState;

impl StateBehavior for DefenderPushingUpState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let forward = if ctx.attacks_right { 2.0 } else { -2.0 };
        (forward, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        !ctx.team_has_ball
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::TrackingBack)
    }
}
