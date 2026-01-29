//! Midfielder AttackSupporting State Logic
//!
//! Handles off-ball support positioning in possession.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{MidfielderSubState, PositionSubState};

pub struct MidfielderAttackSupportingState;

impl StateBehavior for MidfielderAttackSupportingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let support_y = ctx.ball_position.1.clamp(10.0, 58.0);
        let support_x = if ctx.attacks_right {
            (ctx.ball_position.0 + 10.0).min(95.0)
        } else {
            (ctx.ball_position.0 - 10.0).max(10.0)
        };
        let dir = direction_to(ctx.player_position, (support_x, support_y));
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, _ctx: &PositionContext) -> bool {
        false
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Midfielder(MidfielderSubState::Distributing)
    }
}
