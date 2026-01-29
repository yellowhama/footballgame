//! Defender HoldingLine State Logic
//!
//! Maintains the defensive line position with minimal lateral movement.

use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

pub struct DefenderHoldingLineState;

impl StateBehavior for DefenderHoldingLineState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dy = (ctx.defensive_line_y - ctx.player_position.1) * 0.5;
        (0.0, dy.clamp(-2.0, 2.0))
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        if ctx.opponent_with_ball_nearby
            && ctx.ball_distance < ctx.levers.press_trigger_distance
        {
            return Some(PositionSubState::Defender(DefenderSubState::Pressing));
        }
        None
    }

    fn should_timeout(&self, _ctx: &PositionContext) -> bool {
        false
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::HoldingLine)
    }
}
