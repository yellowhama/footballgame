//! Position-specific behavior handlers
//!
//! This module contains handlers for each position group (GK, DEF, MID, FWD).
//! Each handler implements the PositionStateHandler trait.

// FIX_2601/0106 P2-10: Timeout macros to reduce boilerplate
// These macros generate `should_timeout` implementations for StateBehavior trait

/// Generate a tick-based timeout check.
///
/// # Example
/// ```ignore
/// impl StateBehavior for MyState {
///     impl_timeout_ticks!(30); // timeout after 30 ticks in substate
///     // ... other methods
/// }
/// ```
#[macro_export]
macro_rules! impl_timeout_ticks {
    ($threshold:expr) => {
        fn should_timeout(&self, ctx: &$crate::engine::position_behavior::PositionContext) -> bool {
            ctx.in_substate_ticks > $threshold
        }
    };
}

/// Generate a never-timeout implementation.
///
/// # Example
/// ```ignore
/// impl StateBehavior for MyState {
///     impl_timeout_never!(); // never timeout
///     // ... other methods
/// }
/// ```
#[macro_export]
macro_rules! impl_timeout_never {
    () => {
        fn should_timeout(&self, _ctx: &$crate::engine::position_behavior::PositionContext) -> bool {
            false
        }
    };
}

/// Generate a tick-based timeout with additional condition.
///
/// # Example
/// ```ignore
/// impl StateBehavior for MyState {
///     impl_timeout_ticks_or!(30, ctx.team_has_ball); // timeout after 30 ticks OR when team has ball
///     // ... other methods
/// }
/// ```
#[macro_export]
macro_rules! impl_timeout_ticks_or {
    ($threshold:expr, $condition:expr) => {
        fn should_timeout(&self, ctx: &$crate::engine::position_behavior::PositionContext) -> bool {
            ctx.in_substate_ticks > $threshold || $condition
        }
    };
}

pub mod defender;
pub mod defender_blocking;
pub mod defender_covering;
pub mod defender_holding_line;
pub mod defender_intercepting;
pub mod defender_marking;
pub mod defender_offside_trap;
pub mod defender_pressing;
pub mod defender_pushing_up;
pub mod defender_states;
pub mod defender_tracking_back;
pub mod forward_assisting;
pub mod forward_creating_space;
pub mod forward;
pub mod forward_offside_trap_breaking;
pub mod forward_holding_up_play;
pub mod forward_pressing;
pub mod forward_receiving_cross;
pub mod forward_run_in_behind;
pub mod forward_states;
pub mod goalkeeper_action_states;
pub mod goalkeeper_attentive;
pub mod goalkeeper;
pub mod goalkeeper_distributing;
pub mod goalkeeper_holding_ball;
pub mod goalkeeper_passing;
pub mod goalkeeper_picking_up_ball;
pub mod goalkeeper_preparing_for_save;
pub mod goalkeeper_preempt;
pub mod goalkeeper_positioning;
pub mod goalkeeper_returning_to_goal;
pub mod goalkeeper_running;
pub mod goalkeeper_save_action;
pub mod goalkeeper_shooting;
pub mod goalkeeper_sweeping;
pub mod goalkeeper_tackling;
pub mod goalkeeper_under_pressure;
pub mod midfielder_attack_supporting;
pub mod midfielder_creating_space;
pub mod midfielder;
pub mod midfielder_passing;
pub mod midfielder_pressing;
pub mod midfielder_states;
pub mod midfielder_tracking_runner;
pub mod traits;
pub mod levers;

pub use defender::DefenderHandler;
pub use forward::ForwardHandler;
pub use goalkeeper::GoalkeeperHandler;
pub use midfielder::MidfielderHandler;

use crate::engine::position_behavior::PositionStateHandler;
use crate::models::player::Position;

/// Get the appropriate handler for a position
pub fn get_handler_for_position(position: Position) -> &'static dyn PositionStateHandler {
    match position {
        Position::GK => &GoalkeeperHandler,
        Position::LB | Position::CB | Position::RB | Position::LWB | Position::RWB | Position::DF => {
            &DefenderHandler
        }
        Position::CDM
        | Position::CM
        | Position::CAM
        | Position::LM
        | Position::RM
        | Position::MF => &MidfielderHandler,
        Position::LW | Position::RW | Position::CF | Position::ST | Position::FW => &ForwardHandler,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_selection() {
        assert!(matches!(
            get_handler_for_position(Position::GK).default_substate(),
            crate::engine::position_substates::PositionSubState::Goalkeeper(_)
        ));
        assert!(matches!(
            get_handler_for_position(Position::CB).default_substate(),
            crate::engine::position_substates::PositionSubState::Defender(_)
        ));
        assert!(matches!(
            get_handler_for_position(Position::CM).default_substate(),
            crate::engine::position_substates::PositionSubState::Midfielder(_)
        ));
        assert!(matches!(
            get_handler_for_position(Position::ST).default_substate(),
            crate::engine::position_substates::PositionSubState::Forward(_)
        ));
    }
}
