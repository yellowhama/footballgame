//! Goalkeeper Tackling State Logic
//!
//! Handles sweeper-keeper tackling attempts.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperTacklingState;

impl StateBehavior for GoalkeeperTacklingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Tackling)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall)
        } else {
            PositionSubState::Goalkeeper(GoalkeeperSubState::ReturningToGoal)
        }
    }
}
