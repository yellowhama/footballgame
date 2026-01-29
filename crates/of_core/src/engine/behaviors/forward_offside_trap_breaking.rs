//! Forward OffsideTrapBreaking State Logic
//!
//! Handles timing runs to beat the offside line.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{ForwardSubState, PositionSubState};

pub struct ForwardOffsideTrapBreakingState;

impl StateBehavior for ForwardOffsideTrapBreakingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        if ctx.in_substate_ticks < 10 {
            let back = if ctx.attacks_right { -1.0 } else { 1.0 };
            (back, 0.0)
        } else {
            let forward = if ctx.attacks_right { 8.0 } else { -8.0 };
            (forward, 0.0)
        }
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 30
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_attacking && ctx.can_break_offside {
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}
