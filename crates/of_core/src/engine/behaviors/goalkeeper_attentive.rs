//! Goalkeeper Attentive State Logic
//!
//! Handles small adjustments based on ball angle.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::{distance, PositionContext};
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperAttentiveState;

impl StateBehavior for GoalkeeperAttentiveState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let ball_offset = ctx.ball_position.0 - ctx.goal_position.0;
        let dx = (ball_offset * 0.1).clamp(-1.0, 1.0);
        (dx, 0.0)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        if let Some(next) = preempt_transition(ctx, GoalkeeperSubState::Attentive) {
            return Some(next);
        }

        if ctx.ball_in_danger_zone && !ctx.shot_incoming {
            return Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Sweeping));
        }

        if !ctx.player_has_ball && !ctx.shot_incoming && ctx.ball_distance > 30.0 {
            let dist_to_goal = distance(ctx.player_position, ctx.goal_position);
            if dist_to_goal > 5.0 {
                return Some(PositionSubState::Goalkeeper(
                    GoalkeeperSubState::ReturningToGoal,
                ));
            }
        }

        if !ctx.shot_incoming && !ctx.player_has_ball && !ctx.ball_in_danger_zone {
            let dist_to_goal = distance(ctx.player_position, ctx.goal_position);
            if dist_to_goal < 8.0 && ctx.team_has_ball {
                return Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Positioning));
            }
        }

        None
    }

    fn should_timeout(&self, _ctx: &PositionContext) -> bool {
        false
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {  
        PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn attentive_moves_to_sweeping_in_danger_zone() {
        let state = GoalkeeperAttentiveState;
        let mut ctx = PositionContext::default();
        ctx.ball_in_danger_zone = true;
        ctx.shot_incoming = false;
        ctx.player_has_ball = false;

        let next = state.try_fast_transition(&ctx);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Sweeping))
        );
    }

    #[test]
    fn attentive_returns_to_goal_when_far_and_ball_is_far() {
        let state = GoalkeeperAttentiveState;
        let mut ctx = PositionContext::default();
        ctx.ball_distance = 35.0;
        ctx.player_position = (20.0, field::CENTER_Y);
        ctx.goal_position = (0.0, field::CENTER_Y);
        ctx.shot_incoming = false;
        ctx.player_has_ball = false;
        ctx.ball_in_danger_zone = false;

        let next = state.try_fast_transition(&ctx);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::ReturningToGoal))
        );
    }

    #[test]
    fn attentive_moves_to_positioning_when_team_has_ball_and_near_goal() {
        let state = GoalkeeperAttentiveState;
        let mut ctx = PositionContext::default();
        ctx.team_has_ball = true;
        ctx.ball_distance = 10.0;
        ctx.player_position = (0.0, field::CENTER_Y);
        ctx.goal_position = (0.0, field::CENTER_Y);
        ctx.shot_incoming = false;
        ctx.player_has_ball = false;
        ctx.ball_in_danger_zone = false;

        let next = state.try_fast_transition(&ctx);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Positioning))
        );
    }
}
