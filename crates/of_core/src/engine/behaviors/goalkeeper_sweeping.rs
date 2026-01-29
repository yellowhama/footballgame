//! Goalkeeper Sweeping State Logic
//!
//! Handles sweeper-keeper rushes toward the ball.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::{direction_to, distance, PositionContext};
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperSweepingState;

impl StateBehavior for GoalkeeperSweepingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 8.0, dir.1 * 8.0)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        if let Some(next) = preempt_transition(ctx, GoalkeeperSubState::Sweeping) {
            return Some(next);
        }

        if ctx.opponent_with_ball_nearby && ctx.ball_distance < 3.0 {
            return Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Tackling));
        }

        if !ctx.player_has_ball && !ctx.shot_incoming && ctx.ball_distance > 30.0 {
            let dist_to_goal = distance(ctx.player_position, ctx.goal_position);
            if dist_to_goal > 5.0 {
                return Some(PositionSubState::Goalkeeper(
                    GoalkeeperSubState::ReturningToGoal,
                ));
            }
        }

        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance > 15.0 || ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall)
        } else {
            PositionSubState::Goalkeeper(GoalkeeperSubState::ReturningToGoal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::behaviors::traits::StateBehavior;

    #[test]
    fn sweeping_transitions_to_tackling_when_close() {
        let state = GoalkeeperSweepingState;
        let mut ctx = PositionContext::default();
        ctx.opponent_with_ball_nearby = true;
        ctx.ball_distance = 2.5;

        let next = state.try_fast_transition(&ctx);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Tackling))
        );
    }

    #[test]
    fn sweeping_timeout_returns_to_goal_without_ball() {
        let state = GoalkeeperSweepingState;
        let mut ctx = PositionContext::default();
        ctx.ball_distance = 20.0;
        ctx.player_has_ball = false;

        assert!(state.should_timeout(&ctx));
        assert_eq!(
            state.timeout_transition(&ctx),
            PositionSubState::Goalkeeper(GoalkeeperSubState::ReturningToGoal)
        );
    }

    #[test]
    fn sweeping_timeout_holding_ball_returns_holding() {
        let state = GoalkeeperSweepingState;
        let mut ctx = PositionContext::default();
        ctx.ball_distance = 20.0;
        ctx.player_has_ball = true;

        assert!(state.should_timeout(&ctx));
        assert_eq!(
            state.timeout_transition(&ctx),
            PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall)
        );
    }
}
