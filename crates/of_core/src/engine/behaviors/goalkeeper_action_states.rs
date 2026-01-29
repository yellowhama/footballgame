//! Goalkeeper action states that delegate to grouped behaviors.

use crate::engine::behaviors::goalkeeper_distributing::GoalkeeperDistributingState;
use crate::engine::behaviors::goalkeeper_preempt::preempt_transition;
use crate::engine::behaviors::goalkeeper_preparing_for_save::GoalkeeperPreparingForSaveState;
use crate::engine::behaviors::goalkeeper_save_action::GoalkeeperSaveActionState;
use crate::engine::behaviors::goalkeeper_sweeping::GoalkeeperSweepingState;
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub struct GoalkeeperJumpingState;

impl StateBehavior for GoalkeeperJumpingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperSaveActionState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Jumping)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperSaveActionState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        GoalkeeperSaveActionState.timeout_transition(ctx)
    }
}

pub struct GoalkeeperDivingState;

impl StateBehavior for GoalkeeperDivingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperSaveActionState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Diving)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperSaveActionState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        GoalkeeperSaveActionState.timeout_transition(ctx)
    }
}

pub struct GoalkeeperCatchingState;

impl StateBehavior for GoalkeeperCatchingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperSaveActionState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Catching)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperSaveActionState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        GoalkeeperSaveActionState.timeout_transition(ctx)
    }
}

pub struct GoalkeeperPunchingState;

impl StateBehavior for GoalkeeperPunchingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperSaveActionState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Punching)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperSaveActionState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        GoalkeeperSaveActionState.timeout_transition(ctx)
    }
}

pub struct GoalkeeperComingOutState;

impl StateBehavior for GoalkeeperComingOutState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperSweepingState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        if let Some(next) = preempt_transition(ctx, GoalkeeperSubState::ComingOut) {
            return Some(next);
        }

        if ctx.opponent_with_ball_nearby && ctx.ball_distance < 3.0 {
            return Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Tackling));
        }

        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperSweepingState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        GoalkeeperSweepingState.timeout_transition(ctx)
    }
}

pub struct GoalkeeperThrowingState;

impl StateBehavior for GoalkeeperThrowingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperDistributingState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Throwing)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperDistributingState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        GoalkeeperDistributingState.timeout_transition(ctx)
    }
}

pub struct GoalkeeperKickingState;

impl StateBehavior for GoalkeeperKickingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperDistributingState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::Kicking)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperDistributingState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        GoalkeeperDistributingState.timeout_transition(ctx)
    }
}

pub struct GoalkeeperPenaltySaveState;

impl StateBehavior for GoalkeeperPenaltySaveState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        GoalkeeperPreparingForSaveState.calculate_velocity(ctx)
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        preempt_transition(ctx, GoalkeeperSubState::PenaltySave)
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        GoalkeeperPreparingForSaveState.should_timeout(ctx)
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {   
        GoalkeeperPreparingForSaveState.timeout_transition(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coming_out_prefers_preempt_over_tackle() {
        let state = GoalkeeperComingOutState;
        let mut ctx = PositionContext::default();
        ctx.shot_incoming = true;
        ctx.opponent_with_ball_nearby = true;
        ctx.ball_distance = 2.0;

        let next = state.try_fast_transition(&ctx);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::PreparingForSave))
        );
    }

    #[test]
    fn coming_out_can_tackle_when_close() {
        let state = GoalkeeperComingOutState;
        let mut ctx = PositionContext::default();
        ctx.opponent_with_ball_nearby = true;
        ctx.ball_distance = 2.0;

        let next = state.try_fast_transition(&ctx);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::Tackling))
        );
    }
}
