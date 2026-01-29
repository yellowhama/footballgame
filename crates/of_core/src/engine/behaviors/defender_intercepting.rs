//! Defender Intercepting State Logic
//!
//! Handles interception attempts toward the ball.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{DefenderSubState, PositionSubState};
use crate::engine::steering::pursuit;

pub struct DefenderInterceptingState;

impl StateBehavior for DefenderInterceptingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        pursuit(
            ctx.player_position,
            ctx.ball_position,
            ctx.ball_velocity,
            6.0,
            0.8,
        )
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20 || ctx.team_has_ball
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
