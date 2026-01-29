//! Midfielder TrackingRunner State Logic
//!
//! Handles following an opponent runner into space.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{MidfielderSubState, PositionSubState};
use crate::engine::steering::pursuit;

pub struct MidfielderTrackingRunnerState;

impl StateBehavior for MidfielderTrackingRunnerState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        if let Some(runner) = &ctx.runner_to_track {
            pursuit(
                ctx.player_position,
                runner.position,
                runner.velocity,
                6.0,
                1.0,
            )
        } else {
            (0.0, 0.0)
        }
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.runner_to_track.is_none() || ctx.team_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}
