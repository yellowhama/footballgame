//! Midfielder CreatingSpace State Logic
//!
//! Handles movement away from crowded zones.
use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::PositionContext;
use crate::engine::steering;
use crate::engine::position_substates::{MidfielderSubState, PositionSubState};

pub struct MidfielderCreatingSpaceState;

impl StateBehavior for MidfielderCreatingSpaceState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        let dir = ctx.least_crowded_direction;
        let target = (
            ctx.player_position.0 + dir.0 * 6.0,
            ctx.player_position.1 + dir.1 * 6.0,
        );
        let base = steering::arrive(ctx.player_position, target, 5.0, 3.0);
        let separation = ctx.separation_force;
        (base.0 + separation.0, base.1 + separation.1)
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
