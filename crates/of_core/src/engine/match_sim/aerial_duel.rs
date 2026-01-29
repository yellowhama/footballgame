//! Aerial Duel System
//!
//! This module contains aerial/heading related logic for MatchEngine:
//! - Header action execution
//! - Aerial duel resolution (P3)
//! - Header direction determination (shot vs pass)
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use crate::engine::actions::{self, AerialDefender, AerialDuelContext};
use crate::engine::ball::HeightProfile;
use crate::engine::physics_constants::field;
use crate::engine::types::coord10::Coord10;
use crate::models::trait_system::TraitId;
use crate::models::TeamSide;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // Aerial Duel System (P3)
    // ===========================================

    /// Execute header action in aerial situation (P3: Aerial duel implementation)
    pub(crate) fn execute_header_action(&mut self, header_idx: usize) {
        let ball_pos = self.ball.position;
        let is_home = TeamSide::is_home(header_idx);

        // 1. Find opponents in range (radius 3m)
        // FIX_2601: pos is now Coord10
        let (ball_pos_x, ball_pos_y) = ball_pos.to_meters();
        let opponents_in_range: Vec<usize> = self
            .player_positions
            .iter()
            .enumerate()
            .filter(|(idx, pos)| {
                let is_opponent = TeamSide::is_home(*idx) != is_home;
                if !is_opponent {
                    return false;
                }

                // Within 3m of ball position
                let pos_m = pos.to_meters();
                let dx = pos_m.0 - ball_pos_x;
                let dy = pos_m.1 - ball_pos_y;
                let dist = (dx * dx + dy * dy).sqrt();
                dist < 3.0
            })
            .map(|(idx, _)| idx)
            .collect();

        let is_contested = !opponents_in_range.is_empty();

        // P3: Determine aerial duel result
        if is_contested {
            // Find the best opponent for aerial duel
            let (winner_idx, won_duel) = self.resolve_aerial_duel(header_idx, &opponents_in_range);

            if !won_duel {
                // Lost the aerial duel - opponent wins the header
                // Opponent now controls the situation
                // FIX_2601/1120: Update ball position to winner's position to prevent teleportation
                self.ball.current_owner = Some(winner_idx);
                self.ball.position = self.player_positions[winner_idx];
                self.determine_header_direction(winner_idx, TeamSide::is_home(winner_idx));
                return;
            }
            // Won the duel - continue with header action
        }

        // 2. Calculate header success rate (using A5 factor) - duel is won or uncontested
        let success_rate = self.calculate_header_success(header_idx, is_contested);
        let success = self.rng.gen::<f32>() < success_rate;

        if !success {
            // Header failed: ball goes in random direction
            let random_angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let random_dist = self.rng.gen_range(2.0..5.0); // 2-5m
            let (ball_x, ball_y) = ball_pos.to_meters();
            let target_x = (ball_x + random_angle.cos() * random_dist).clamp(-1.0, 106.0);
            let target_y = (ball_y + random_angle.sin() * random_dist).clamp(-1.0, 69.0);
            let target = Coord10::from_meters(target_x, target_y);
            self.ball.start_flight(target, 2.0, None);
            self.ball.height_profile = HeightProfile::Arc;
            return;
        }

        // 3. Header success: determine direction
        self.determine_header_direction(header_idx, is_home);
    }

    /// P3: Resolve aerial duel - pure function pattern applied
    /// Returns (winner_idx, did_original_player_win)
    pub(crate) fn resolve_aerial_duel(
        &mut self,
        player_idx: usize,
        opponents: &[usize],
    ) -> (usize, bool) {
        // Build defenders list
        let defenders: Vec<AerialDefender> = opponents
            .iter()
            .map(|&opp_idx| AerialDefender {
                idx: opp_idx,
                name: self.get_player_name(opp_idx),
                heading: self.get_player_heading(opp_idx),
                jumping: self.get_player_jumping(opp_idx),
                strength: self.get_player_strength(opp_idx),
                bravery: self.get_player_aggression(opp_idx), // bravery = aggression
                has_airraid: self.player_has_gold_trait(opp_idx, TraitId::AirRaid),
                has_bully: self.player_has_gold_trait(opp_idx, TraitId::Bully),
                position: self.player_positions[opp_idx].to_normalized_legacy(), // FIX_2601/0116
            })
            .collect();

        // Create AerialDuelContext
        let ctx = AerialDuelContext {
            attacker_idx: player_idx,
            attacker_name: self.get_player_name(player_idx),
            attacker_heading: self.get_player_heading(player_idx),
            attacker_jumping: self.get_player_jumping(player_idx),
            attacker_strength: self.get_player_strength(player_idx),
            attacker_bravery: self.get_player_aggression(player_idx),
            has_airraid: self.player_has_gold_trait(player_idx, TraitId::AirRaid),
            has_bully: self.player_has_gold_trait(player_idx, TraitId::Bully),
            defenders,
        };

        // Call pure function
        let roll = self.rng.gen::<f32>();
        let result = actions::resolve_aerial_duel(&ctx, roll);

        (result.winner_idx, result.attacker_won)
    }

    /// Determine header direction (pass vs shot/clearance)
    /// FIX_2601: Updated to use Coord10 directly
    /// FIX_2601/0109: Use attacks_right for correct second-half goal calculation
    pub(crate) fn determine_header_direction(&mut self, header_idx: usize, is_home: bool) {
        let header_pos = self.player_positions[header_idx];
        let attacks_right = self.attacks_right(is_home);
        // Goal position in Coord10 (0.1m units)
        let goal_pos = if attacks_right {
            Coord10::from_meters(field::LENGTH_M, field::CENTER_Y)
        } else {
            Coord10::from_meters(0.0, field::CENTER_Y)
        };

        // Distance to goal in meters
        let dist_to_goal = header_pos.distance_to_m(&goal_pos);

        // Decide shot vs pass based on distance (was 15.0 normalized ≈ 15.75m, now 15m)
        if dist_to_goal < 15.0 {
            // Inside penalty box - attempt shot
            // FIX_2601: Use attacks_right for correct halftime handling
            let goal_x = if self.attacks_right(is_home) { field::LENGTH_M } else { 0.0 };
            let goal_y = field::CENTER_Y;
            let target_x = goal_x + self.rng.gen_range(-2.0..2.0);
            let target_y = goal_y + self.rng.gen_range(-2.0..2.0);
            let target = Coord10::from_meters(target_x, target_y);
            self.ball.start_flight(target, 3.5, None);
            self.ball.height_profile = HeightProfile::Arc;
        } else {
            // Far from goal: pass to teammate (FIX_2601: collect Coord10 directly)
            let teammates: Vec<(usize, Coord10)> = self
                .player_positions
                .iter()
                .enumerate()
                .filter(|(idx, _)| TeamSide::is_home(*idx) == is_home && *idx != header_idx)
                .map(|(idx, &pos)| (idx, pos))
                .collect();

            // FIX_2601/0110: Use random selection instead of .first() to avoid index order bias
            // Previously: always picked lowest-indexed teammate (Home: 1-10, Away: 12-21)
            if !teammates.is_empty() {
                let random_idx = self.rng.gen_range(0..teammates.len());
                let (target_idx, target_pos) = teammates[random_idx];
                self.ball.start_flight(target_pos, 2.5, Some(target_idx));
                self.ball.height_profile = HeightProfile::Lob;
            } else {
                // Clearance: forward direction
                // FIX_2601/0109: Use attacks_right for correct second-half clearance direction
                let header_m = header_pos.to_meters();
                let clear_x = if attacks_right { header_m.0 + 20.0 } else { header_m.0 - 20.0 };
                let clear_y = header_m.1;
                let clear_target = Coord10::from_meters(clear_x, clear_y);
                self.ball.start_flight(clear_target, 3.0, None);
                self.ball.height_profile = HeightProfile::Lob;
            }
        }
    }
}
