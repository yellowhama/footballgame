//! Player Movement System
//!
//! This module contains player movement update logic for MatchEngine:
//! - Tick-based position update for all 22 players
//! - Game state update based on ball position
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use crate::engine::ball::lerp_position;
use crate::engine::types::{BallZone, Coord10, GameState};
use crate::models::TeamSide;

/// Direction-aware fallback when the ball has no owner.
///
/// FIX_2601/0116:
/// - First half: home attacks RIGHT => home defends LEFT (ball_length < 0.5)
/// - Second half: home attacks LEFT  => home defends RIGHT (ball_length >= 0.5)
fn derive_home_possession_without_owner(ball_length: f32, home_attacks_right: bool) -> bool {
    if home_attacks_right {
        ball_length < 0.5
    } else {
        ball_length >= 0.5
    }
}

/// Compute BallZone in the possession team's view (0=own goal, 1=opponent goal).
///
/// This prevents "Attacking/Defensive" from flipping incorrectly after halftime.
fn derive_ball_zone_team_view(ball_length: f32, possession_attacks_right: bool) -> BallZone {
    let tv_length = if possession_attacks_right { ball_length } else { 1.0 - ball_length };

    if tv_length < 0.33 {
        BallZone::Defensive
    } else if tv_length < 0.66 {
        BallZone::Midfield
    } else {
        BallZone::Attacking
    }
}

impl MatchEngine {
    // ===========================================
    // Player Movement System
    // ===========================================

    /// Update all player positions based on game state (called each tick)
    pub(crate) fn update_player_positions_for_tick(&mut self, tick_duration_sec: f32) {
        if self.player_positions.is_empty() {
            return;
        }

        // Determine which team has possession
        let home_has_possession = if let Some(owner) = self.ball.current_owner {
            TeamSide::is_home(owner)
        } else {
            // FIX_2601/0104: Use .0 (length) for goal-to-goal direction, not .1 (width)
            // to_normalized() returns (length, width) where length is 0=home goal, 1=away goal
            // FIX_2601/0116: Must be direction-aware (halftime side swap)
            let ball_length = self.ball.position.to_normalized().0.clamp(0.0, 1.0);
            derive_home_possession_without_owner(ball_length, self.home_ctx.attacks_right)
        };

        let score_diff = self.result.score_home as i8 - self.result.score_away as i8;

        // FIX_2601/0116: 2-Phase Batch Update to prevent index-based bias
        // ========== Phase 1: Calculate all new positions (Snapshot-based) ==========
        let positions_snapshot: Vec<Coord10> = self.player_positions.clone();
        let mut new_positions: Vec<Coord10> = positions_snapshot.clone();

        for idx in 0..22 {
            let is_home = TeamSide::is_home(idx);

            // FIX_2601/0105: Use explicit direction context instead of is_home boolean
            let ctx = if is_home { &self.home_ctx } else { &self.away_ctx };

            // Get target position based on game state
            let target = self.calculate_target_position(idx, ctx, home_has_possession, score_diff);

            // DEBUG: Print first Away player's target
            static DEBUG_COUNTER: std::sync::atomic::AtomicU32 =
                std::sync::atomic::AtomicU32::new(0);
            if idx >= 11 && DEBUG_COUNTER.load(std::sync::atomic::Ordering::Relaxed) < 3 {
                DEBUG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                #[cfg(debug_assertions)]
                let current = positions_snapshot[idx].to_normalized_legacy();
                #[cfg(debug_assertions)]
                println!(
                    "[DEBUG-0105-MOV] idx={} is_home={} ctx.attacks_right={} target=({:.3},{:.3}) current=({:.3},{:.3}) home_poss={}",
                    idx, is_home, ctx.attacks_right, target.0, target.1, current.0, current.1, home_has_possession
                );
            }

            // Calculate movement speed (normalized units per second)
            // Base speed: ~0.05 units/sec, adjusted by player pace
            let player_speed = self.get_player_movement_speed(idx) * tick_duration_sec;

            // Interpolate towards target (Snapshot-based)
            // FIX_2601: Convert Coord10 to normalized for lerp, then back
            let current = positions_snapshot[idx].to_normalized_legacy();
            let new_pos = lerp_position(current, target, player_speed);
            new_positions[idx] = Coord10::from_normalized_legacy(new_pos);
        }

        // ========== Phase 2: Batch Apply ==========
        self.player_positions = new_positions;

        // Iterative Separation: Push players apart to avoid overlap
        // Runs for MAX_SEPARATION_ITERATIONS iterations for fine-grained adjustment
        self.apply_iterative_separation();

        // Phase 3: Update ball physics (flight simulation)
        if let Some(result) = self.update_ball_physics(tick_duration_sec) {
            self.handle_action_result(result);
        }

        // Update ball position to follow the owner (only if not in flight)
        if !self.ball.is_in_flight {
            if let Some(owner) = self.ball.current_owner {
                if let Some(pos) = self.player_positions.get(owner) {
                    // FIX_2601: pos is already Coord10, use directly
                    self.ball.position = *pos;
                }
            }
        }
    }

    /// Update game state based on ball position
    pub(crate) fn update_game_state_from_ball(&mut self) {
        let score_diff = self.result.score_home as i8 - self.result.score_away as i8;

        // FIX_2601/0104: Use .0 (length) for goal-to-goal direction
        let ball_length = self.ball.position.to_normalized().0.clamp(0.0, 1.0);

        // Determine possession
        let is_home_possession = self
            .ball
            .current_owner
            .map(TeamSide::is_home)
            .unwrap_or_else(|| derive_home_possession_without_owner(ball_length, self.home_ctx.attacks_right));

        // FIX_2601/0116: BallZone must be computed in the possession team's view (team-view).
        let possession_attacks_right = if is_home_possession { self.home_ctx.attacks_right } else { self.away_ctx.attacks_right };
        let ball_zone = derive_ball_zone_team_view(ball_length, possession_attacks_right);

        self.game_state =
            GameState { minute: self.minute, score_diff, is_home_possession, ball_zone };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_home_possession_without_owner_direction_aware() {
        // First half (home attacks right): home defends left => left half defaults to home.
        assert!(derive_home_possession_without_owner(0.49, true));
        assert!(!derive_home_possession_without_owner(0.51, true));

        // Second half (home attacks left): home defends right => right half defaults to home.
        assert!(!derive_home_possession_without_owner(0.49, false));
        assert!(derive_home_possession_without_owner(0.51, false));
    }

    #[test]
    fn test_derive_ball_zone_team_view_flips_with_direction() {
        // If a team attacks right, length directly maps to team-view.
        assert!(matches!(derive_ball_zone_team_view(0.2, true), BallZone::Defensive));
        assert!(matches!(derive_ball_zone_team_view(0.5, true), BallZone::Midfield));
        assert!(matches!(derive_ball_zone_team_view(0.9, true), BallZone::Attacking));

        // If a team attacks left, team-view length is flipped.
        assert!(matches!(derive_ball_zone_team_view(0.9, false), BallZone::Defensive)); // 1-0.9 = 0.1
        assert!(matches!(derive_ball_zone_team_view(0.5, false), BallZone::Midfield)); // 1-0.5 = 0.5
        assert!(matches!(derive_ball_zone_team_view(0.1, false), BallZone::Attacking)); // 1-0.1 = 0.9
    }
}
