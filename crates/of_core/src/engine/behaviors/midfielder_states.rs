//! Midfielder state implementations for remaining substates.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{MidfielderSubState, PositionSubState};
use crate::engine::steering::pursuit;
use crate::engine::physics_constants::field;

pub struct MidfielderDistributingState;

impl StateBehavior for MidfielderDistributingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
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

pub struct MidfielderSwitchingPlayState;

impl StateBehavior for MidfielderSwitchingPlayState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let target_x = if ctx.attacks_right {
            if ctx.ball_position.0 < field::CENTER_X { 85.0 } else { 20.0 }
        } else {
            if ctx.ball_position.0 > field::CENTER_X { 20.0 } else { 85.0 }
        };
        let dir = direction_to(ctx.player_position, (target_x, ctx.player_position.1));
        (dir.0 * 5.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 60
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderHoldingPossessionState;

impl StateBehavior for MidfielderHoldingPossessionState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.local_pressure < 0.3 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        }
    }
}

pub struct MidfielderShootingState;

impl StateBehavior for MidfielderShootingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        }
    }
}

pub struct MidfielderDistanceShootingState;

impl StateBehavior for MidfielderDistanceShootingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        }
    }
}

pub struct MidfielderRecyclingState;

impl StateBehavior for MidfielderRecyclingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.own_goal);
        (dir.0 * 2.0, dir.1 * 2.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 30
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderInterceptingState;

impl StateBehavior for MidfielderInterceptingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        pursuit(
            ctx.player_position,
            ctx.ball_position,
            ctx.ball_velocity,
            6.0,
            0.8,
        )
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20 || ctx.team_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderDribblingState;

impl StateBehavior for MidfielderDribblingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let advance_x = if ctx.attacks_right { 3.0 } else { -3.0 };
        (advance_x, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 40 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        }
    }
}

pub struct MidfielderCrossingState;

impl StateBehavior for MidfielderCrossingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let (target_x, target_y) = if ctx.attacks_right {
            let wide_y = if ctx.player_position.1 < field::CENTER_Y { 10.0 } else { 58.0 };
            (90.0, wide_y)
        } else {
            let wide_y = if ctx.player_position.1 < field::CENTER_Y { 10.0 } else { 58.0 };
            (15.0, wide_y)
        };
        let dir = direction_to(ctx.player_position, (target_x, target_y));
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 30
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        }
    }
}

pub struct MidfielderTacklingState;

impl StateBehavior for MidfielderTacklingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderRunningState;

impl StateBehavior for MidfielderRunningState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 40
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderReturningState;

impl StateBehavior for MidfielderReturningState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let mid_pos = (field::CENTER_X, field::CENTER_Y);
        let dir = direction_to(ctx.player_position, mid_pos);
        (dir.0 * 4.0, dir.1 * 4.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 60
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderRestingState;

impl StateBehavior for MidfielderRestingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
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

pub struct MidfielderStandingState;

impl StateBehavior for MidfielderStandingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 25.0 || ctx.local_pressure > 0.3
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.ball_distance < 20.0 {
            PositionSubState::Midfielder(MidfielderSubState::Pressing)
        } else if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderWalkingState;

impl StateBehavior for MidfielderWalkingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let mid_pos = (field::CENTER_X, field::CENTER_Y);
        let dir = direction_to(ctx.player_position, mid_pos);
        (dir.0 * 1.5, dir.1 * 1.5)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 20.0 || ctx.local_pressure > 0.4
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.ball_distance < 20.0 {
            PositionSubState::Midfielder(MidfielderSubState::Pressing)
        } else if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}

pub struct MidfielderJoggingState;

impl StateBehavior for MidfielderJoggingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let mid_pos = (field::CENTER_X, field::CENTER_Y);
        let dir = direction_to(ctx.player_position, mid_pos);
        (dir.0 * 3.5, dir.1 * 3.5)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 22.0 || ctx.local_pressure > 0.35
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.ball_distance < 20.0 {
            PositionSubState::Midfielder(MidfielderSubState::Pressing)
        } else if ctx.team_has_ball {
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        } else {
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        }
    }
}
