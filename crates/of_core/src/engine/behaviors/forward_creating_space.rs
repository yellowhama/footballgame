//! Forward CreatingSpace State Logic
//!
//! Moves into open space to stretch the defense.

use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::steering;
use crate::engine::position_substates::{ForwardSubState, PositionSubState};

pub struct ForwardCreatingSpaceState;

impl StateBehavior for ForwardCreatingSpaceState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = ctx.best_space_direction;
        let target = (
            ctx.player_position.0 + dir.0 * 7.0,
            ctx.player_position.1 + dir.1 * 7.0,
        );
        let base = steering::arrive(ctx.player_position, target, 6.0, 3.5);
        let separation = ctx.separation_force;
        (base.0 + separation.0, base.1 + separation.1)
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
