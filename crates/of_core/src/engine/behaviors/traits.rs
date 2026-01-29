//! Trait definitions for behavior state handling
//!
//! This module defines the interface for individual substate logic.

use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::PositionSubState;

/// Logic handler for a specific substate (e.g., Marking, HoldingLine)
///
/// Implement this trait for each detailed state to encapsulate its specific logic:
/// - Velocity calculation (Movement)
/// - State transitions (Logic)
/// - Timeout handling (reset)
pub trait StateBehavior: Send + Sync {
    /// Calculate velocity modifier for this specific state
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32);

    /// Check for transitions out of this specific state
    ///
    /// Returns `Some(new_state)` if a transition condition is met.
    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState>;

    /// Check timeout condition for this specific state
    fn should_timeout(&self, ctx: &PositionContext) -> bool;

    /// Get timeout transition target for this specific state
    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState;
}
