//! Movement Helper Functions
//!
//! This module contains movement-related helper functions for MatchEngine:
//! - Player separation algorithm (collision avoidance)
//! - Movement speed calculation
//!
//! Extracted from match_sim/mod.rs for better organization.
//!
//! FIX_2601: Updated to use Coord10 directly

use super::MatchEngine;
use crate::models::TeamSide;

/// Maximum iterations for player separation algorithm
const MAX_SEPARATION_ITERATIONS: usize = 10;

impl MatchEngine {
    // ===========================================
    // Movement Helper System
    // ===========================================

    /// Iterative player separation algorithm
    /// Prevents player overlap by pushing overlapping players apart
    /// FIX_2601: Now works with Coord10 directly (0.1m units)
    pub(crate) fn apply_iterative_separation(&mut self) {
        // Minimum separation distance: 5 = 0.5m (Coord10: 0.1m units)
        const MIN_DIST: i32 = 5;
        const MIN_DIST_SQ: i32 = MIN_DIST * MIN_DIST;
        // Early exit threshold: 1 = 0.1m
        const EARLY_EXIT_THRESHOLD: i32 = 1;

        for _ in 0..MAX_SEPARATION_ITERATIONS {
            let mut displacements = [(0i32, 0i32); 22];
            let mut max_displacement: i32 = 0;

            // 1. Calculate overlap between all player pairs
            for i in 0..22 {
                for j in (i + 1)..22 {
                    let pos_i = self.player_positions[i];
                    let pos_j = self.player_positions[j];

                    let dx = pos_i.x - pos_j.x;
                    let dy = pos_i.y - pos_j.y;
                    let dist_sq = dx * dx + dy * dy;

                    // Overlap detected (below minimum distance)
                    if dist_sq < MIN_DIST_SQ && dist_sq > 0 {
                        let dist = (dist_sq as f32).sqrt();
                        let push = ((MIN_DIST as f32 - dist) * 0.5) as i32; // Push half distance each

                        let push_x = ((dx as f32 / dist) * push as f32).round() as i32;
                        let push_y = ((dy as f32 / dist) * push as f32).round() as i32;

                        displacements[i].0 += push_x;
                        displacements[i].1 += push_y;
                        displacements[j].0 -= push_x;
                        displacements[j].1 -= push_y;
                    }
                }
            }

            // 2. Update positions and track max displacement
            for i in 0..22 {
                let (dx, dy) = displacements[i];
                self.player_positions[i].x += dx;
                self.player_positions[i].y += dy;

                // Field boundary clamp (players stay in-bounds)
                self.player_positions[i] = self.player_positions[i].clamp_in_bounds();

                let disp = dx.abs() + dy.abs();
                if disp > max_displacement {
                    max_displacement = disp;
                }
            }

            // 3. Early exit: if no one moved significantly, stop
            if max_displacement < EARLY_EXIT_THRESHOLD {
                break;
            }
        }
    }

    /// Calculate movement speed for a player based on pace attribute
    pub(crate) fn get_player_movement_speed(&self, player_idx: usize) -> f32 {
        // Base movement: 0.08 units/sec (~8.4m/s on 105m pitch, realistic sprint speed)
        let base_speed = 0.08;

        // Get player's pace attribute if available
        let is_home = TeamSide::is_home(player_idx);
        let (team, local_idx) = if is_home {
            (&self.home_team, player_idx)
        } else {
            (&self.away_team, TeamSide::local_idx(player_idx))
        };

        let pace_modifier = if let Some(player) = team.players.get(local_idx) {
            if let Some(attrs) = &player.attributes {
                // Use acceleration for position changes
                let pace = attrs.acceleration as f32;
                pace / 70.0 // Normalized around 70
            } else {
                player.overall as f32 / 70.0
            }
        } else {
            1.0
        };

        base_speed * pace_modifier
    }
}
