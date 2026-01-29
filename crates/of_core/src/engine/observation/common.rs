//! Common Observation Utilities
//!
//! FIX_2601 Phase 4: Shared helper functions for observation builders.

use crate::engine::physics_constants::field;
use crate::engine::tick_snapshot::TickSnapshot;
use crate::engine::types::Coord10;

// =============================================================================
// TeamView Coordinate Transformations
// =============================================================================

/// Convert position to TeamView coordinates (meters)
///
/// TeamView: Own goal at x=0, opponent goal at x=105m
///
/// # Arguments
/// * `pos` - Position in world coordinates (Coord10)
/// * `attacks_right` - True if the observer's team attacks right
///
/// # Returns
/// Position in meters (x, y) in TeamView coordinates
#[inline]
pub fn to_team_view_pos(pos: Coord10, attacks_right: bool) -> (f32, f32) {
    let (x_m, y_m) = pos.to_meters();
    if attacks_right {
        (x_m, y_m)
    } else {
        (field::LENGTH_M - x_m, y_m)
    }
}

/// Convert velocity to TeamView coordinates
///
/// # Arguments
/// * `vel` - Velocity in world coordinates (m/s)
/// * `attacks_right` - True if the observer's team attacks right
///
/// # Returns
/// Velocity in TeamView coordinates (m/s)
#[inline]
pub fn to_team_view_vel(vel: (f32, f32), attacks_right: bool) -> (f32, f32) {
    if attacks_right {
        vel
    } else {
        (-vel.0, vel.1)
    }
}

// =============================================================================
// Direction Utilities
// =============================================================================

/// Normalize a velocity vector to a unit direction vector
///
/// Returns (0, 0) for near-zero velocity (magnitude < 0.001)
#[inline]
pub fn normalize_direction(vel: (f32, f32)) -> (f32, f32) {
    let mag = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    if mag < 0.001 {
        (0.0, 0.0)
    } else {
        (vel.0 / mag, vel.1 / mag)
    }
}

// =============================================================================
// Active Player Detection
// =============================================================================

/// Find the active player for the given team
///
/// Active player is determined by:
/// 1. Ball owner (if on this team)
/// 2. Nearest player to ball (fallback)
///
/// # Arguments
/// * `snapshot` - Current tick snapshot
/// * `is_home` - True for home team perspective
///
/// # Returns
/// Track ID (0-21) of the active player, or None if no players
pub fn find_active_player(snapshot: &TickSnapshot, is_home: bool) -> Option<u8> {
    // If ball owner is on this team, they're active
    if let Some(owner_idx) = snapshot.ball.owner {
        let is_home_player = owner_idx < 11;
        if is_home_player == is_home {
            return Some(owner_idx);
        }
    }

    // Otherwise, find nearest player to ball on this team
    let team_range = if is_home { 0..11usize } else { 11..22usize };
    let ball_pos = snapshot.ball.pos;

    let mut min_dist = i32::MAX;
    let mut nearest_idx = None;

    for idx in team_range {
        let dist = snapshot.players[idx].pos.distance_to(&ball_pos);
        if dist < min_dist {
            min_dist = dist;
            nearest_idx = Some(idx as u8);
        }
    }

    nearest_idx
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tick_snapshot::{
        BallSnap, BallStateTag, GameModeTag, OffBallObjectiveSnap, PlayerSnap, StickyActionsSnap,
        TeamSnap,
    };

    fn create_test_snapshot_with_ball_owner(owner: Option<u8>) -> TickSnapshot {
        TickSnapshot {
            tick: 100,
            minute: 45,
            seed: 42,
            ball: BallSnap {
                state: BallStateTag::Controlled,
                pos: Coord10::from_meters(52.5, 34.0),
                owner,
                target_pos: None,
                eta_tick: None,
                intended_receiver: None,
                is_shot: false,
            },
            players: std::array::from_fn(|i| PlayerSnap {
                id: i as u8,
                is_home: i < 11,
                pos: Coord10::from_meters(52.5 + (i as f32 * 2.0), 34.0),
                state: crate::engine::tick_snapshot::PlayerStateTag::Idle,
                stamina: 1.0,
                dist_to_ball: (i as i32 - 5).abs() * 20,
            }),
            teams: TeamSnap {
                home_attacks_right: true,
                home_has_possession: owner.map(|o| o < 11).unwrap_or(false),
            },
            tackle_cooldowns: [0; 22],
            offball_objectives: [OffBallObjectiveSnap::default(); 22],
            last_pass_target: None,
            home_attacks_right: true,
            player_velocities: [(0.0, 0.0); 22],
            score: (0, 0),
            game_mode: GameModeTag::Normal,
            sticky_actions: [StickyActionsSnap::default(); 22],
        }
    }

    #[test]
    fn test_to_team_view_pos_attacks_right() {
        let pos = Coord10::from_meters(80.0, 34.0);
        let tv = to_team_view_pos(pos, true);
        assert!((tv.0 - 80.0).abs() < 0.1);
        assert!((tv.1 - 34.0).abs() < 0.1);
    }

    #[test]
    fn test_to_team_view_pos_attacks_left() {
        let pos = Coord10::from_meters(80.0, 34.0);
        let tv = to_team_view_pos(pos, false);
        assert!((tv.0 - 25.0).abs() < 0.1); // 105 - 80 = 25
        assert!((tv.1 - 34.0).abs() < 0.1);
    }

    #[test]
    fn test_to_team_view_vel() {
        let vel = (5.0, 3.0);

        // Attacks right: no change
        let tv = to_team_view_vel(vel, true);
        assert_eq!(tv, (5.0, 3.0));

        // Attacks left: x flipped
        let tv = to_team_view_vel(vel, false);
        assert_eq!(tv, (-5.0, 3.0));
    }

    #[test]
    fn test_normalize_direction() {
        // 3-4-5 triangle
        let dir = normalize_direction((3.0, 4.0));
        assert!((dir.0 - 0.6).abs() < 0.001);
        assert!((dir.1 - 0.8).abs() < 0.001);

        // Zero velocity
        let dir = normalize_direction((0.0, 0.0));
        assert_eq!(dir, (0.0, 0.0));

        // Near-zero velocity
        let dir = normalize_direction((0.0001, 0.0));
        assert_eq!(dir, (0.0, 0.0));
    }

    #[test]
    fn test_find_active_player_with_owner() {
        // Ball owned by home player 5
        let snapshot = create_test_snapshot_with_ball_owner(Some(5));
        assert_eq!(find_active_player(&snapshot, true), Some(5));

        // Away team should find nearest (not owner)
        let away_active = find_active_player(&snapshot, false);
        assert!(away_active.is_some());
        assert!(away_active.unwrap() >= 11);
    }

    #[test]
    fn test_find_active_player_no_owner() {
        // No ball owner - find nearest
        let snapshot = create_test_snapshot_with_ball_owner(None);

        let home_active = find_active_player(&snapshot, true);
        assert!(home_active.is_some());
        assert!(home_active.unwrap() < 11);

        let away_active = find_active_player(&snapshot, false);
        assert!(away_active.is_some());
        assert!(away_active.unwrap() >= 11);
    }
}
