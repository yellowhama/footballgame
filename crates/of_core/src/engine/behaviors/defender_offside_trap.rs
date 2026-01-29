//! Defender OffsideTrap State Logic
//!
//! Handles coordinated step-up to spring offside.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

pub struct DefenderOffsideTrapState;

impl StateBehavior for DefenderOffsideTrapState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let step_up = if ctx.attacks_right { 3.0 } else { -3.0 };
        (step_up, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20
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
