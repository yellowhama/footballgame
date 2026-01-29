// ============================================================================
// P2.1-B: Actor State FSM Validation
// ============================================================================
//
// Contract: State transitions must follow valid FSM paths
//
// Valid Transition Diagram:
// ```
// Idle → Running → Tackling → Idle
//       ↓       → Passing → Idle
//       ↓       → Shooting → Idle
//       ↓       → Dribbling → Idle
//       ↓
//    Injured (can only recover to Idle)
//
// Invalid Examples:
// - Tackling → Shooting (can't shoot while tackling)
// - Injured → Tackling (can't tackle while injured)
// - Passing → Shooting (must return to Idle first)
// ```
//
// Purpose: Prevent invalid state transitions that indicate bugs in action logic
// Created: 2025-12-23 (P2.1-B)

use serde::{Deserialize, Serialize};

/// Actor state enumeration for FSM validation
///
/// Note: This is a simplified version for validation purposes.
/// The actual game may have more granular states in different modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActorState {
    /// Player is idle (no specific action)
    Idle,
    /// Player is running/moving
    Running,
    /// Player is passing the ball
    Passing,
    /// Player is shooting
    Shooting,
    /// Player is tackling an opponent
    Tackling,
    /// Player is dribbling
    Dribbling,
    /// Player is injured and cannot perform actions
    Injured,
}

/// Validator for actor state transitions
pub struct ActorStateValidator;

impl ActorStateValidator {
    /// Validate a state transition according to FSM rules
    ///
    /// # Arguments
    /// * `from` - Current state
    /// * `to` - Desired next state
    ///
    /// # Returns
    /// * `Ok(())` if transition is valid
    /// * `Err(String)` with error message if transition is invalid
    ///
    /// # Examples
    /// ```
    /// use of_core::engine::actor_state_validator::{ActorState, ActorStateValidator};
    ///
    /// // Valid transition
    /// assert!(ActorStateValidator::validate_transition(
    ///     ActorState::Idle,
    ///     ActorState::Running
    /// ).is_ok());
    ///
    /// // Invalid transition
    /// assert!(ActorStateValidator::validate_transition(
    ///     ActorState::Tackling,
    ///     ActorState::Shooting
    /// ).is_err());
    /// ```
    pub fn validate_transition(from: ActorState, to: ActorState) -> Result<(), String> {
        use ActorState::*;

        // No-op transitions (state unchanged) are always valid
        if from == to {
            return Ok(());
        }

        let valid = match (from, to) {
            // From Idle: can transition to any active state except Injured
            (Idle, Running) => true,
            (Idle, Passing) => true,
            (Idle, Shooting) => true,
            (Idle, Tackling) => true,
            (Idle, Dribbling) => true,
            (Idle, Injured) => false, // Can't spontaneously become injured from idle

            // From Running: can transition to action states or back to Idle
            (Running, Idle) => true,
            (Running, Passing) => true,
            (Running, Shooting) => true,
            (Running, Tackling) => true,
            (Running, Dribbling) => true,
            (Running, Injured) => true, // Can get injured while running

            // From action states: can only return to Idle or become Injured
            (Passing, Idle) => true,
            (Passing, Injured) => true,
            (Shooting, Idle) => true,
            (Shooting, Injured) => true,
            (Tackling, Idle) => true,
            (Tackling, Injured) => true,
            (Dribbling, Idle) => true,
            (Dribbling, Running) => true, // Dribbling can continue to running
            (Dribbling, Injured) => true,

            // From Injured: can only recover to Idle
            (Injured, Idle) => true,

            // All other transitions are invalid
            _ => false,
        };

        if !valid {
            return Err(format!("Invalid state transition: {:?} → {:?}", from, to));
        }

        Ok(())
    }

    /// Check if a state is an "action" state (not idle/running/injured)
    pub fn is_action_state(state: ActorState) -> bool {
        matches!(
            state,
            ActorState::Passing
                | ActorState::Shooting
                | ActorState::Tackling
                | ActorState::Dribbling
        )
    }

    /// Check if a state is a "recovery" state (idle/running)
    pub fn is_recovery_state(state: ActorState) -> bool {
        matches!(state, ActorState::Idle | ActorState::Running)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // P2.1-B: Actor State FSM Tests
    // ========================================================================

    #[test]
    fn test_valid_transition_idle_to_running() {
        assert!(
            ActorStateValidator::validate_transition(ActorState::Idle, ActorState::Running).is_ok()
        );
    }

    #[test]
    fn test_valid_transition_running_to_action() {
        assert!(ActorStateValidator::validate_transition(ActorState::Running, ActorState::Passing)
            .is_ok());

        assert!(ActorStateValidator::validate_transition(
            ActorState::Running,
            ActorState::Shooting
        )
        .is_ok());

        assert!(ActorStateValidator::validate_transition(
            ActorState::Running,
            ActorState::Tackling
        )
        .is_ok());

        assert!(ActorStateValidator::validate_transition(
            ActorState::Running,
            ActorState::Dribbling
        )
        .is_ok());
    }

    #[test]
    fn test_valid_transition_action_to_idle() {
        assert!(
            ActorStateValidator::validate_transition(ActorState::Passing, ActorState::Idle).is_ok()
        );

        assert!(ActorStateValidator::validate_transition(ActorState::Shooting, ActorState::Idle)
            .is_ok());

        assert!(ActorStateValidator::validate_transition(ActorState::Tackling, ActorState::Idle)
            .is_ok());

        assert!(ActorStateValidator::validate_transition(ActorState::Dribbling, ActorState::Idle)
            .is_ok());
    }

    #[test]
    fn test_invalid_transition_tackling_to_shooting() {
        let result =
            ActorStateValidator::validate_transition(ActorState::Tackling, ActorState::Shooting);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Invalid state transition"));
        assert!(err_msg.contains("Tackling"));
        assert!(err_msg.contains("Shooting"));
    }

    #[test]
    fn test_invalid_transition_injured_to_tackling() {
        let result =
            ActorStateValidator::validate_transition(ActorState::Injured, ActorState::Tackling);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid state transition"));
    }

    #[test]
    fn test_invalid_transition_passing_to_shooting() {
        let result =
            ActorStateValidator::validate_transition(ActorState::Passing, ActorState::Shooting);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid state transition"));
    }

    #[test]
    fn test_invalid_transition_idle_to_injured() {
        let result =
            ActorStateValidator::validate_transition(ActorState::Idle, ActorState::Injured);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid state transition"));
    }

    #[test]
    fn test_valid_transition_injured_recovery() {
        assert!(
            ActorStateValidator::validate_transition(ActorState::Injured, ActorState::Idle).is_ok()
        );
    }

    #[test]
    fn test_valid_transition_dribbling_to_running() {
        // Dribbling can transition to running (continuing movement)
        assert!(ActorStateValidator::validate_transition(
            ActorState::Dribbling,
            ActorState::Running
        )
        .is_ok());
    }

    #[test]
    fn test_valid_transition_can_get_injured() {
        // Can get injured from running or action states
        assert!(ActorStateValidator::validate_transition(ActorState::Running, ActorState::Injured)
            .is_ok());

        assert!(ActorStateValidator::validate_transition(
            ActorState::Tackling,
            ActorState::Injured
        )
        .is_ok());

        assert!(ActorStateValidator::validate_transition(
            ActorState::Dribbling,
            ActorState::Injured
        )
        .is_ok());
    }

    #[test]
    fn test_no_op_transitions() {
        // Same state transitions are always valid
        assert!(
            ActorStateValidator::validate_transition(ActorState::Idle, ActorState::Idle).is_ok()
        );

        assert!(ActorStateValidator::validate_transition(ActorState::Running, ActorState::Running)
            .is_ok());

        assert!(ActorStateValidator::validate_transition(ActorState::Injured, ActorState::Injured)
            .is_ok());
    }

    #[test]
    fn test_is_action_state() {
        assert!(!ActorStateValidator::is_action_state(ActorState::Idle));
        assert!(!ActorStateValidator::is_action_state(ActorState::Running));
        assert!(!ActorStateValidator::is_action_state(ActorState::Injured));

        assert!(ActorStateValidator::is_action_state(ActorState::Passing));
        assert!(ActorStateValidator::is_action_state(ActorState::Shooting));
        assert!(ActorStateValidator::is_action_state(ActorState::Tackling));
        assert!(ActorStateValidator::is_action_state(ActorState::Dribbling));
    }

    #[test]
    fn test_is_recovery_state() {
        assert!(ActorStateValidator::is_recovery_state(ActorState::Idle));
        assert!(ActorStateValidator::is_recovery_state(ActorState::Running));

        assert!(!ActorStateValidator::is_recovery_state(ActorState::Injured));
        assert!(!ActorStateValidator::is_recovery_state(ActorState::Passing));
    }
}
