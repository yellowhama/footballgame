//! ObservationBuilder Trait
//!
//! FIX_2601 Phase 4: SSOT Contract - all observations from TickSnapshot only.

use crate::engine::tick_snapshot::TickSnapshot;

/// Observation builder trait
///
/// SSOT Contract: All observations are derived solely from TickSnapshot.
/// Implementations must NOT import or depend on MatchEngine.
pub trait ObservationBuilder {
    /// Output observation type
    type Output;

    /// Build observation from TickSnapshot
    ///
    /// # Arguments
    /// * `snapshot` - Immutable tick snapshot (SSOT)
    /// * `is_home` - Observer perspective (true = home team, false = away team)
    ///
    /// # Returns
    /// Observation in TeamView coordinates (own goal at x=0)
    fn build(&self, snapshot: &TickSnapshot, is_home: bool) -> Self::Output;
}
