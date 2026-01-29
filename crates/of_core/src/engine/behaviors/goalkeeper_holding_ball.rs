//! Goalkeeper HoldingBall State Logic
//!
//! Handles holding the ball before distribution.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperHoldingBallState;

impl StateBehavior for GoalkeeperHoldingBallState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        if let Some(next) = preempt_transition(ctx, GoalkeeperSubState::HoldingBall) {
            return Some(next);
        }

        if ctx.player_has_ball && ctx.local_pressure > 0.7 {
            return Some(PositionSubState::Goalkeeper(
                GoalkeeperSubState::UnderPressure,
            ));
        }
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 120
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Goalkeeper(GoalkeeperSubState::Distributing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::behaviors::traits::StateBehavior;

    #[test]
    fn holding_ball_transitions_to_under_pressure() {
        let state = GoalkeeperHoldingBallState;
        let mut ctx = PositionContext::default();
        ctx.player_has_ball = true;
        ctx.local_pressure = 0.8;

        let next = state.try_fast_transition(&ctx);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::UnderPressure))
        );
    }

    #[test]
    fn holding_ball_timeouts_to_distributing() {
        let state = GoalkeeperHoldingBallState;
        let mut ctx = PositionContext::default();
        ctx.in_substate_ticks = 121;

        assert!(state.should_timeout(&ctx));
        assert_eq!(
            state.timeout_transition(&ctx),
            PositionSubState::Goalkeeper(GoalkeeperSubState::Distributing)
        );
    }
}
