//! Defender state implementations for remaining substates.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{DefenderSubState, PositionSubState};
use crate::{impl_timeout_ticks, impl_timeout_never};

pub struct DefenderTacklingState;

impl StateBehavior for DefenderTacklingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 4.0, dir.1 * 4.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    impl_timeout_ticks!(20);

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.at_defensive_position {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        } else if ctx.high_danger {
            PositionSubState::Defender(DefenderSubState::TrackingBack)
        } else {
            PositionSubState::Defender(DefenderSubState::Covering)
        }
    }
}

pub struct DefenderSlidingTackleState;

impl StateBehavior for DefenderSlidingTackleState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 6.0, dir.1 * 6.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    impl_timeout_ticks!(15);

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.at_defensive_position {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        } else if ctx.high_danger {
            PositionSubState::Defender(DefenderSubState::TrackingBack)
        } else {
            PositionSubState::Defender(DefenderSubState::Covering)
        }
    }
}

pub struct DefenderHeadingState;

impl StateBehavior for DefenderHeadingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    impl_timeout_ticks!(30);

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.at_defensive_position {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        } else if ctx.high_danger {
            PositionSubState::Defender(DefenderSubState::TrackingBack)
        } else {
            PositionSubState::Defender(DefenderSubState::Covering)
        }
    }
}

pub struct DefenderClearingState;

impl StateBehavior for DefenderClearingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    impl_timeout_ticks!(15);

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.at_defensive_position {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        } else if ctx.high_danger {
            PositionSubState::Defender(DefenderSubState::TrackingBack)
        } else {
            PositionSubState::Defender(DefenderSubState::Covering)
        }
    }
}

pub struct DefenderDribblingState;

impl StateBehavior for DefenderDribblingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let advance = if ctx.attacks_right { 2.0 } else { -2.0 };
        (advance, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.in_substate_ticks > 40 || !ctx.player_has_ball
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Defender(DefenderSubState::Dribbling)
        } else {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        }
    }
}

pub struct DefenderPassingState;

impl StateBehavior for DefenderPassingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    impl_timeout_ticks!(15);

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.player_has_ball {
            PositionSubState::Defender(DefenderSubState::Dribbling)
        } else {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        }
    }
}

pub struct DefenderRunningState;

impl StateBehavior for DefenderRunningState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        (dir.0 * 5.0, dir.1 * 5.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    impl_timeout_ticks!(40);

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::HoldingLine)
    }
}

pub struct DefenderReturningState;

impl StateBehavior for DefenderReturningState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.own_goal);
        (dir.0 * 4.0, dir.1 * 4.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.at_defensive_position
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::HoldingLine)
    }
}

pub struct DefenderRestingState;

impl StateBehavior for DefenderRestingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    impl_timeout_never!();

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::HoldingLine)
    }
}

pub struct DefenderStandingState;

impl StateBehavior for DefenderStandingState {
    fn calculate_velocity(&self, _ctx: &PositionContext) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 20.0 || ctx.local_pressure > 0.3
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.ball_distance < 15.0 {
            PositionSubState::Defender(DefenderSubState::Pressing)
        } else {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        }
    }
}

pub struct DefenderWalkingState;

impl StateBehavior for DefenderWalkingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.own_goal);
        (dir.0 * 1.5, dir.1 * 1.5)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 15.0 || ctx.local_pressure > 0.4
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.ball_distance < 15.0 {
            PositionSubState::Defender(DefenderSubState::Pressing)
        } else {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        }
    }
}

pub struct DefenderJoggingState;

impl StateBehavior for DefenderJoggingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = direction_to(ctx.player_position, ctx.own_goal);
        (dir.0 * 3.5, dir.1 * 3.5)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        ctx.ball_distance < 18.0 || ctx.local_pressure > 0.35
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        if ctx.ball_distance < 15.0 {
            PositionSubState::Defender(DefenderSubState::Pressing)
        } else {
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        }
    }
}
