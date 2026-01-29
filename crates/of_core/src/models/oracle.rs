//! Oracle Test System Data Structures
//!
//! This module defines the snapshot format for oracle-based regression testing.
//! Oracle tests verify logical invariants rather than exact equality, allowing
//! engine improvements while catching breaking changes.
//!
//! ## Key Concepts
//!
//! - **Oracle Snapshot**: A saved match result with pre-computed invariants
//! - **Invariants**: Logical consistency checks (e.g., "score equals goal events")
//! - **Tolerance**: Flexible verification (allows improvements, catches bugs)
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Generate oracle snapshot
//! let snapshot = OracleSnapshot {
//!     oracle_id: "oracle_00_01".to_string(),
//!     schema_version: 1,
//!     fixture: FixtureInfo { home_team_id: 0, away_team_id: 1, seed: 12345 },
//!     match_result: result_snapshot,
//!     invariants: computed_invariants,
//! };
//!
//! // Verify invariants
//! assert!(snapshot.invariants.score_consistency);
//! assert!((snapshot.invariants.possession_sum - 100.0).abs() < 0.01);
//! ```

use super::{MatchEvent, Statistics};
use serde::{Deserialize, Serialize};

/// Oracle snapshot containing match result and pre-computed invariants
///
/// This is the top-level structure saved to JSON files in `tests/oracle/snapshots/`.
/// Each snapshot represents a single match simulation that can be used for
/// regression testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleSnapshot {
    /// Unique identifier (e.g., "oracle_00_01" for teams 0 vs 1)
    pub oracle_id: String,

    /// Schema version for backward compatibility
    /// Current version: 1
    pub schema_version: u8,

    /// Fixture information (teams and seed)
    pub fixture: FixtureInfo,

    /// Full match result snapshot
    pub match_result: MatchResultSnapshot,

    /// Pre-computed invariants for verification
    pub invariants: Invariants,
}

/// Fixture information identifying the match configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureInfo {
    /// Home team fixture ID (e.g., 0 for team_000.json)
    pub home_team_id: usize,

    /// Away team fixture ID (e.g., 1 for team_001.json)
    pub away_team_id: usize,

    /// Random seed used for this match
    pub seed: u64,
}

/// Compact match result snapshot
///
/// Stores the essential match outcome data needed for invariant verification.
/// This is a subset of the full `MatchResult` struct, excluding position_data
/// to keep snapshot files small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResultSnapshot {
    /// Final score for home team
    pub score_home: u8,

    /// Final score for away team
    pub score_away: u8,

    /// All match events in chronological order
    pub events: Vec<MatchEvent>,

    /// Match statistics (shots, possession, etc.)
    pub statistics: Statistics,
}

/// Pre-computed invariants for logical consistency checks
///
/// Each field represents a verification check that should hold true for
/// any valid match simulation, regardless of engine improvements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariants {
    /// Event count breakdown by type and team
    pub event_counts: EventCounts,

    /// I1: Score consistency - does score equal goal events?
    /// True if: score_home + score_away == count(Goal + OwnGoal events)
    pub score_consistency: bool,

    /// I2: Possession sum - does possession add up to 100%?
    /// Value should be exactly 100.0 (±0.01 tolerance)
    pub possession_sum: f32,

    /// I3: Timeline monotonicity - are events in chronological order?
    /// True if: all event minutes are non-decreasing
    pub timeline_monotonic: bool,

    /// I4: Statistics match events - do stats align with event counts?
    /// True if: stats.shots_home == count(Shot events, home team), etc.
    pub statistics_match_events: bool,

    /// I5: xG validity - are expected goals values valid?
    /// True if: all xG values > 0 and sum matches stats
    pub xg_sum_positive: bool,

    /// I6: Determinism - does re-simulation produce same invariants?
    /// True if: re-running with same seed yields identical invariants
    pub determinism_verified: bool,
}

/// Event count breakdown by type and team
///
/// Counts specific event types for invariant verification (I1, I4).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventCounts {
    /// Total number of events
    pub total_events: usize,

    /// Goals scored by home team
    pub goals_home: usize,

    /// Goals scored by away team
    pub goals_away: usize,

    /// Shots by home team (all types)
    pub shots_home: usize,

    /// Shots by away team (all types)
    pub shots_away: usize,

    /// Yellow cards issued to home team
    pub yellow_cards_home: usize,

    /// Yellow cards issued to away team
    pub yellow_cards_away: usize,

    /// Red cards issued to home team
    pub red_cards_home: usize,

    /// Red cards issued to away team
    pub red_cards_away: usize,

    /// Fouls committed by home team
    pub fouls_home: usize,

    /// Fouls committed by away team
    pub fouls_away: usize,
}

impl Default for Invariants {
    fn default() -> Self {
        Self {
            event_counts: EventCounts::default(),
            score_consistency: false,
            possession_sum: 0.0,
            timeline_monotonic: false,
            statistics_match_events: false,
            xg_sum_positive: false,
            determinism_verified: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oracle_snapshot_serialization() {
        let snapshot = OracleSnapshot {
            oracle_id: "test_oracle".to_string(),
            schema_version: 1,
            fixture: FixtureInfo { home_team_id: 0, away_team_id: 1, seed: 12345 },
            match_result: MatchResultSnapshot {
                score_home: 2,
                score_away: 1,
                events: Vec::new(),
                statistics: Statistics::default(),
            },
            invariants: Invariants::default(),
        };

        // Should serialize to JSON without errors
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        assert!(json.contains("test_oracle"));

        // Should deserialize back
        let deserialized: OracleSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.oracle_id, "test_oracle");
        assert_eq!(deserialized.fixture.seed, 12345);
    }

    #[test]
    fn test_event_counts_default() {
        let counts = EventCounts::default();
        assert_eq!(counts.total_events, 0);
        assert_eq!(counts.goals_home, 0);
        assert_eq!(counts.shots_away, 0);
    }

    #[test]
    fn test_invariants_default() {
        let inv = Invariants::default();
        assert!(!inv.score_consistency);
        assert!(!inv.timeline_monotonic);
        assert_eq!(inv.possession_sum, 0.0);
    }
}
