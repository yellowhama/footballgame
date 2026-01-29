//! Goalkeeper ReturningToGoal State Logic
//!
//! Handles recovery back to the goal line.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::{distance, PositionContext};
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};
use crate::engine::steering;

pub struct GoalkeeperReturningToGoalState;

impl StateBehavior for GoalkeeperReturningToGoalState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        steering::arrive(ctx.player_position, ctx.goal_position, 5.0, 6.0)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        if let Some(next) = preempt_transition(ctx, GoalkeeperSubState::ReturningToGoal) {
            return Some(next);
        }

        if !ctx.shot_incoming && !ctx.player_has_ball && !ctx.ball_in_danger_zone {
            let dist_to_goal = distance(ctx.player_position, ctx.goal_position);
            if dist_to_goal < 8.0 && ctx.team_has_ball {
                return Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Positioning));
            }
        }

        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        distance(ctx.player_position, ctx.goal_position) < 3.0
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive)
    }
}
