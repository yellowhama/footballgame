//! Forward state implementations for remaining substates.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::physics_constants::field;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{ForwardSubState, PositionSubState};
use crate::engine::steering::pursuit;

pub struct ForwardFinishingState;

impl StateBehavior for ForwardFinishingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        if ctx.ball_distance > 2.0 {
            let dir = direction_to(ctx.player_position, ctx.ball_position);
            (dir.0 * 3.0, dir.1 * 3.0)
        } else {
            (0.0, 0.0)
        }
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardShootingState;

impl StateBehavior for ForwardShootingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        if ctx.ball_distance > 2.0 {
            let dir = direction_to(ctx.player_position, ctx.ball_position);
            (dir.0 * 3.0, dir.1 * 3.0)
        } else {
            (0.0, 0.0)
        }
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardDribblingState;

impl StateBehavior for ForwardDribblingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let advance = if ctx.attacks_right { 4.0 } else { -4.0 };
        (advance, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 40 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardHeadingState;

impl StateBehavior for ForwardHeadingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 6.0, dir.1 * 6.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 30
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Forward(ForwardSubState::Finishing)
        } else {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        }
    }
}

pub struct ForwardInterceptingState;

impl StateBehavior for ForwardInterceptingState {
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
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        }
    }
}

pub struct ForwardTacklingState;

impl StateBehavior for ForwardTacklingState {
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
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        }
    }
}

pub struct ForwardRunningState;

impl StateBehavior for ForwardRunningState {
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
        if ctx.team_attacking && ctx.can_break_offside {
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardReturningState;

impl StateBehavior for ForwardReturningState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let mid_pos = (field::CENTER_X, 60.0);
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
        if ctx.team_attacking && ctx.can_break_offside {
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardPassingState;

impl StateBehavior for ForwardPassingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 20 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardRestingState;

impl StateBehavior for ForwardRestingState {
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
        PositionSubState::Forward(ForwardSubState::CreatingSpace)
    }
}

pub struct ForwardStandingState;

impl StateBehavior for ForwardStandingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 30.0 || ctx.local_pressure > 0.3
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball && ctx.can_break_offside {
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardWalkingState;

impl StateBehavior for ForwardWalkingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let attack_pos = (field::CENTER_X, 75.0);
        let dir = direction_to(ctx.player_position, attack_pos);
        (dir.0 * 1.5, dir.1 * 1.5)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 25.0 || ctx.local_pressure > 0.4
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball && ctx.can_break_offside {
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}

pub struct ForwardJoggingState;

impl StateBehavior for ForwardJoggingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let attack_pos = (field::CENTER_X, 75.0);
        let dir = direction_to(ctx.player_position, attack_pos);
        (dir.0 * 3.5, dir.1 * 3.5)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 27.0 || ctx.local_pressure > 0.35
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.team_has_ball && ctx.can_break_offside {
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        } else if ctx.team_has_ball {
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        } else {
            PositionSubState::Forward(ForwardSubState::Pressing)
        }
    }
}
