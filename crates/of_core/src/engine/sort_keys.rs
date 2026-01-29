//! Stable Sort Key Utilities
//!
//! FIX_2601/0123 PR #9-1: Deterministic tie-breaker keys for sorting.
//!
//! ## Problem
//! When sorting by floating-point scores (e.g., threat_score, pressure_score),
//! ties can result in unstable ordering that varies across platforms/Rust versions.
//!
//! ## Solution
//! Provide stable secondary keys based on immutable identifiers (track_id, team_side).
//! These keys are:
//! - **Platform-independent**: No hash functions involved
//! - **Deterministic**: Same input always produces same key
//! - **Simple**: Based on integer comparisons only
//!
//! ## Usage
//! ```ignore
//! // Instead of:
//! threats.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Equal));
//!
//! // Use:
//! threats.sort_by(|a, b| {
//!     match b.score.total_cmp(&a.score) {
//!         Ordering::Equal => stable_key(a.idx).cmp(&stable_key(b.idx)),
//!         other => other,
//!     }
//! });
//! ```

use std::cmp::Ordering;

// ============================================================================
// Stable Sort Key
// ============================================================================

/// Stable tie-breaker key for track_id-based sorting.
///
/// Returns a u32 key that provides deterministic ordering when primary scores are equal.
/// The key is simply the track_id itself (0..21 for standard matches).
///
/// # Why track_id?
/// - **Immutable**: track_id doesn't change during a match
/// - **Unique**: Each player has exactly one track_id
/// - **Simple**: No computation, no hash, no platform variance
///
/// # Arguments
/// * `track_id` - Player's track ID (0..10 = home, 11..21 = away)
///
/// # Returns
/// A u32 key for comparison (lower = higher priority in tie-breaks)
#[inline]
pub const fn stable_key_for_track(track_id: usize) -> u32 {
    track_id as u32
}

/// Stable tie-breaker key with team separation.
///
/// Returns a composite key: `(team_priority, track_id_within_team)`.
/// This ensures home team players are grouped together in ties.
///
/// # Arguments
/// * `track_id` - Player's track ID (0..10 = home, 11..21 = away)
/// * `home_first` - If true, home team has lower key (sorted first)
///
/// # Returns
/// A u32 key where bits are: `[team_bit:1][track_in_team:7][unused:24]`
#[inline]
pub const fn stable_key_with_team(track_id: usize, home_first: bool) -> u32 {
    let is_home = track_id < 11;
    let track_in_team = if is_home { track_id } else { track_id - 11 };

    let team_bit = if home_first {
        if is_home { 0u32 } else { 1u32 }
    } else {
        if is_home { 1u32 } else { 0u32 }
    };

    // Composite: team_bit in high bits, track_in_team in low bits
    (team_bit << 16) | (track_in_team as u32)
}

// ============================================================================
// Float Comparison Helpers
// ============================================================================

/// Compare two f32 values with stable tie-breaking.
///
/// Uses `total_cmp` for NaN-safe ordering, then falls back to stable key.
///
/// # Ordering
/// - Primary: score descending (higher score = earlier in sort)
/// - Secondary: track_id ascending (lower track_id = earlier in sort)
#[inline]
pub fn compare_score_desc_stable(
    a_score: f32,
    a_track: usize,
    b_score: f32,
    b_track: usize,
) -> Ordering {
    // Primary: score descending (b vs a for desc)
    match b_score.total_cmp(&a_score) {
        Ordering::Equal => {
            // Secondary: track_id ascending (a vs b for asc)
            stable_key_for_track(a_track).cmp(&stable_key_for_track(b_track))
        }
        other => other,
    }
}

/// Compare two f32 values ascending with stable tie-breaking.
///
/// # Ordering
/// - Primary: score ascending (lower score = earlier in sort)
/// - Secondary: track_id ascending (lower track_id = earlier in sort)
#[inline]
pub fn compare_score_asc_stable(
    a_score: f32,
    a_track: usize,
    b_score: f32,
    b_track: usize,
) -> Ordering {
    // Primary: score ascending (a vs b for asc)
    match a_score.total_cmp(&b_score) {
        Ordering::Equal => {
            // Secondary: track_id ascending
            stable_key_for_track(a_track).cmp(&stable_key_for_track(b_track))
        }
        other => other,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_key_for_track_deterministic() {
        // Same input always produces same output
        for track_id in 0..22 {
            let key1 = stable_key_for_track(track_id);
            let key2 = stable_key_for_track(track_id);
            assert_eq!(key1, key2, "Key should be deterministic for track_id={}", track_id);
        }
    }

    #[test]
    fn test_stable_key_for_track_ordering() {
        // Lower track_id = lower key
        for i in 0..21 {
            let key_i = stable_key_for_track(i);
            let key_next = stable_key_for_track(i + 1);
            assert!(key_i < key_next, "track_id {} should have lower key than {}", i, i + 1);
        }
    }

    #[test]
    fn test_stable_key_with_team_home_first() {
        // Home team (0..10) should have lower keys than away (11..21)
        let home_key = stable_key_with_team(5, true);
        let away_key = stable_key_with_team(15, true);
        assert!(home_key < away_key, "Home should sort before away when home_first=true");
    }

    #[test]
    fn test_stable_key_with_team_away_first() {
        // Away team should have lower keys when home_first=false
        let home_key = stable_key_with_team(5, false);
        let away_key = stable_key_with_team(15, false);
        assert!(home_key > away_key, "Away should sort before home when home_first=false");
    }

    #[test]
    fn test_compare_score_desc_stable() {
        // Higher score wins
        assert_eq!(
            compare_score_desc_stable(0.8, 5, 0.6, 3),
            Ordering::Less, // a(0.8) should come before b(0.6) in desc order
        );

        // Equal scores: lower track_id wins
        assert_eq!(
            compare_score_desc_stable(0.5, 3, 0.5, 7),
            Ordering::Less, // a(track=3) should come before b(track=7)
        );

        // Equal scores, equal tracks: equal
        assert_eq!(
            compare_score_desc_stable(0.5, 3, 0.5, 3),
            Ordering::Equal,
        );
    }

    #[test]
    fn test_compare_score_handles_nan() {
        // NaN handling via total_cmp
        let nan = f32::NAN;
        let normal = 0.5;

        // NaN is greater than all other values in total_cmp
        // So in descending order, NaN comes first
        let result = compare_score_desc_stable(nan, 1, normal, 2);
        assert_eq!(result, Ordering::Less, "NaN should sort before normal in desc");
    }
}
