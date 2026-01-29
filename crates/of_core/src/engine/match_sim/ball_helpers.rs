//! Ball Height Profile Helpers
//!
//! This module contains helper functions for determining ball trajectory:
//! - Shot height profile (A4 system)
//! - Pass height profile (A4 system)
//! - Opponent counting between positions
//!
//! Extracted from match_sim/mod.rs for better organization.
//!
//! FIX_2601: Added Coord10 overloads for direct coordinate usage.

use super::MatchEngine;
use crate::engine::ball::HeightProfile;
use crate::engine::types::Coord10;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // A4: Ball Height Profile System
    // ===========================================

    /// A4: Determine shot height profile based on situation
    pub(crate) fn determine_shot_height_profile(
        &mut self,
        shooter_pos: (f32, f32),
        goal_pos: (f32, f32),
        distance: f32,
        ball_height: f32,
        is_home: bool,
    ) -> HeightProfile {
        // Header shot (ball height > 0.3m)
        if ball_height > 0.3 {
            return HeightProfile::Arc;
        }

        // Calculate GK position (GK is at slot 0 for away, 11 for home)
        let gk_idx = if is_home { 11 } else { 0 }; // Opponent's GK
                                                   // FIX_2601: Convert Coord10 to normalized for calculations
        let gk_pos = self.player_positions[gk_idx].to_normalized_legacy();
        let _gk_distance =
            ((gk_pos.0 - shooter_pos.0).powi(2) + (gk_pos.1 - shooter_pos.1).powi(2)).sqrt();

        // Close range (< 15m = ~0.15 normalized): Driven (powerful low shot)
        if distance < 0.15 {
            return HeightProfile::Arc;
        }

        // GK off the line (far from goal): Try Lob
        let gk_to_goal_distance =
            ((gk_pos.0 - goal_pos.0).powi(2) + (gk_pos.1 - goal_pos.1).powi(2)).sqrt();
        if gk_to_goal_distance > 0.05 && distance < 0.25 {
            return HeightProfile::Lob;
        }

        // Mid-range (15-25m): Random (70% Driven, 30% Flat)
        if distance < 0.25 {
            if self.rng.gen::<f32>() < 0.7 {
                HeightProfile::Arc
            } else {
                HeightProfile::Flat
            }
        }
        // Long-range (> 25m): Lob or Flat
        else if self.rng.gen::<f32>() < 0.4 {
            HeightProfile::Lob
        } else {
            HeightProfile::Flat
        }
    }

    /// A4: Determine pass height profile based on situation
    pub(crate) fn determine_pass_height_profile(
        &mut self,
        passer_pos: (f32, f32),
        receiver_pos: (f32, f32),
        distance: f32,
        ball_height: f32,
        is_home: bool,
    ) -> HeightProfile {
        // Header pass
        if ball_height > 0.3 {
            return HeightProfile::Arc;
        }

        // Short pass (< 15m): Flat (ground ball)
        if distance < 0.15 {
            return HeightProfile::Flat;
        }

        // Check if opponents are blocking the pass path
        let opponents_between = self.count_opponents_between(passer_pos, receiver_pos, is_home);

        // Mid-range pass with opponents blocking: Lob
        if opponents_between > 0 && distance < 0.30 {
            return HeightProfile::Lob;
        }

        // Mid-range pass (15-30m): Driven (fast and accurate)
        if distance < 0.30 {
            return HeightProfile::Arc;
        }

        // Long pass (> 30m): Lob (cross, side change)
        HeightProfile::Lob
    }

    /// Helper: Count opponents between two positions
    pub(crate) fn count_opponents_between(
        &self,
        from: (f32, f32),
        to: (f32, f32),
        is_home: bool,
    ) -> usize {
        let (start_idx, end_idx) = if is_home { (11, 22) } else { (0, 11) };

        let mut count = 0;
        for idx in start_idx..end_idx {
            // FIX_2601: Convert Coord10 to normalized for calculations
            let opponent_pos = self.player_positions[idx].to_normalized_legacy();

            // Calculate perpendicular distance from point to line
            let dx = to.0 - from.0;
            let dy = to.1 - from.1;
            let line_length_sq = dx * dx + dy * dy;

            if line_length_sq < 0.0001 {
                continue; // Too short pass
            }

            let t =
                ((opponent_pos.0 - from.0) * dx + (opponent_pos.1 - from.1) * dy) / line_length_sq;

            // t in range 0~1 means point is within the line segment
            if !(0.0..=1.0).contains(&t) {
                continue;
            }

            let closest_x = from.0 + t * dx;
            let closest_y = from.1 + t * dy;
            let dist_to_line = ((opponent_pos.0 - closest_x).powi(2)
                + (opponent_pos.1 - closest_y).powi(2))
            .sqrt();

            // Within 0.05 (5m) of pass path is considered blocking
            if dist_to_line < 0.05 {
                count += 1;
            }
        }

        count
    }

    // ===========================================
    // FIX_2601: Coord10 Overloads
    // ===========================================

    /// A4: Determine shot height profile (Coord10 version)
    ///
    /// FIX_2601: Direct Coord10 input for cleaner API
    pub(crate) fn determine_shot_height_profile_coord10(
        &mut self,
        shooter_pos: Coord10,
        goal_pos: Coord10,
        ball_height: f32,
        is_home: bool,
    ) -> HeightProfile {
        let shooter_norm = shooter_pos.to_normalized_legacy();
        let goal_norm = goal_pos.to_normalized_legacy();
        let distance = shooter_pos.distance_to(&goal_pos) as f32 / 1000.0; // Coord10 units to normalized ~

        self.determine_shot_height_profile(shooter_norm, goal_norm, distance, ball_height, is_home)
    }

    /// A4: Determine pass height profile (Coord10 version)
    ///
    /// FIX_2601: Direct Coord10 input for cleaner API
    pub(crate) fn determine_pass_height_profile_coord10(
        &mut self,
        passer_pos: Coord10,
        receiver_pos: Coord10,
        ball_height: f32,
        is_home: bool,
    ) -> HeightProfile {
        let passer_norm = passer_pos.to_normalized_legacy();
        let receiver_norm = receiver_pos.to_normalized_legacy();
        let distance = passer_pos.distance_to(&receiver_pos) as f32 / 1000.0;

        self.determine_pass_height_profile(
            passer_norm,
            receiver_norm,
            distance,
            ball_height,
            is_home,
        )
    }

    /// Helper: Count opponents between two positions (Coord10 version)
    ///
    /// FIX_2601: Direct Coord10 input for cleaner API
    pub(crate) fn count_opponents_between_coord10(
        &self,
        from: Coord10,
        to: Coord10,
        is_home: bool,
    ) -> usize {
        let from_norm = from.to_normalized_legacy();
        let to_norm = to.to_normalized_legacy();

        self.count_opponents_between(from_norm, to_norm, is_home)
    }
}
