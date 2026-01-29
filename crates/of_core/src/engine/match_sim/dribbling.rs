//! Dribbling System
//!
//! This module contains dribble-related logic for MatchEngine:
//! - Dribble action execution (pure function pattern)
//! - Shot opportunity check during dribble
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use crate::engine::actions::{self, DribbleContext, DribbleRolls};
use crate::engine::player_decision::PlayerDecision;
use crate::engine::physics_constants::field;
use crate::models::trait_system::TraitId;
use crate::models::{MatchEvent, SpecialSkill};
use crate::player::skill_system::SkillCalculator;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // Dribbling System
    // ===========================================

    /// Execute dribble action (pure function pattern applied)
    pub(crate) fn execute_dribble_action(&mut self, player_idx: usize, is_home: bool) {
        // Check shot opportunity during dribble (Open Football style)
        if self.can_shoot_while_dribbling(player_idx) {
            let player_name = self.get_match_player(player_idx).name.clone();

            // Calculate team strength
            let attack_strength = if is_home {
                self.calculate_team_strength(&self.home_team.clone(), true)
            } else {
                self.calculate_team_strength(&self.away_team.clone(), false)
            };
            let defense_strength = if is_home {
                self.calculate_team_strength(&self.away_team.clone(), false)
            } else {
                self.calculate_team_strength(&self.home_team.clone(), true)
            };

            self.execute_shot_action(is_home, &player_name, attack_strength, defense_strength);
            return;
        }

        // ============================================
        // 1. Find nearest defender
        // ============================================
        // FIX_2601: Convert Coord10 to normalized for calculations
        let dribbler_pos_coord = self.player_positions[player_idx];
        let dribbler_pos = dribbler_pos_coord.to_normalized_legacy();
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        // Emit dribble event (for event tracking/replay)
        // C6: Use player_idx directly as track_id
        let (ball_x_m, ball_y_m) = self.ball.position.to_meters();
        let ball_height_m = self.ball.height as f32 / 10.0;
        self.emit_event(MatchEvent::dribble(
            self.minute,
            self.current_timestamp_ms(),
            is_home,
            player_idx,
            (ball_x_m / field::LENGTH_M, ball_y_m / field::WIDTH_M, ball_height_m),
        ));

        // FIX_2601/0115: Position-neutral tie-breaker (replaces Y-bias from 0110)
        let nearest_defender = opponent_range
            .clone()
            .min_by(|&a, &b| {
                // FIX_2601: Use Coord10::distance_to_m() for comparison
                let dist_a = dribbler_pos_coord.distance_to_m(&self.player_positions[a]);
                let dist_b = dribbler_pos_coord.distance_to_m(&self.player_positions[b]);
                match dist_a.partial_cmp(&dist_b) {
                    Some(std::cmp::Ordering::Equal) | None => {
                        // FIX_2601/0115: Deterministic hash tie-breaker (no position bias)
                        let pos_a = self.player_positions[a].to_meters();
                        let pos_b = self.player_positions[b].to_meters();
                        super::deterministic_tie_hash(a, pos_a, b, pos_b)
                    }
                    Some(ord) => ord,
                }
            })
            .unwrap();

        // ============================================
        // 2. Create DribbleContext
        // ============================================

        // A13: SpeedDemon skill check
        let has_speed_demon = if let Some(dribbler) = self.get_player(player_idx) {
            dribbler.has_skill(SpecialSkill::SpeedDemon)
        } else {
            false
        };

        // PlayerInstructions contribution calculation
        let dribbler_instr = self.get_player_instructions(player_idx);
        let dribbler_pos_str = self.get_position_string_by_idx(player_idx);
        let off_contrib =
            PlayerDecision::calculate_offensive_contribution(&dribbler_instr, &dribbler_pos_str);

        let defender_instr = self.get_player_instructions(nearest_defender);
        let defender_pos_str = self.get_position_string_by_idx(nearest_defender);
        let def_contrib =
            PlayerDecision::calculate_defensive_contribution(&defender_instr, &defender_pos_str);

        let off_mod = 0.9 + (off_contrib * 0.1); // 0.9 ~ 1.05
        let def_mod = 0.9 + (def_contrib * 0.1); // 0.9 ~ 1.05

        // Calculate target position (forward direction)
        // FIX_2601/0110: Use attacks_right instead of is_home for correct 2nd half direction
        let attacks_right = self.attacks_right(is_home);
        let target_pos = if attacks_right {
            (dribbler_pos.0, (dribbler_pos.1 + 0.05).min(1.0))
        } else {
            (dribbler_pos.0, (dribbler_pos.1 - 0.05).max(0.0))
        };

        let ctx = DribbleContext {
            dribbler_idx: player_idx,
            dribbler_name: self.get_player_name(player_idx),
            is_home,
            current_pos: dribbler_pos,
            target_pos,
            dribbling: self.get_player_dribbling(player_idx),
            agility: self.get_player_agility(player_idx),
            balance: self.get_player_balance(player_idx),
            pace: self.get_player_pace(player_idx),
            defender_idx: Some(nearest_defender),
            defender_marking: Some(self.get_player_marking(nearest_defender)),
            defender_positioning: Some(self.get_player_positioning(nearest_defender)),
            defender_pace: Some(self.get_player_pace(nearest_defender)),
            offensive_modifier: off_mod,
            defensive_modifier: def_mod,
            has_speed_demon,
            has_speedster: self.player_has_gold_trait(player_idx, TraitId::Speedster),
            has_technician: self.player_has_gold_trait(player_idx, TraitId::Technician),
            has_tank: self.player_has_gold_trait(player_idx, TraitId::Tank),
            defender_has_shadow: self.player_has_gold_trait(nearest_defender, TraitId::Shadow),
            defender_has_bully: self.player_has_gold_trait(nearest_defender, TraitId::Bully),
        };

        // ============================================
        // 3. Create DribbleRolls
        // ============================================
        let rolls = DribbleRolls { success_roll: self.rng.gen() };

        // ============================================
        // 4. Call pure function
        // ============================================
        let result = actions::resolve_dribble(&ctx, &rolls);

        // ============================================
        // 5. Apply result
        // ============================================
        if result.success {
            // Success: maintain possession, update position
            // FIX_2601: Convert normalized tuple to Coord10
            use crate::engine::types::coord10::Coord10;
            self.player_positions[player_idx] =
                Coord10::from_normalized_legacy(result.new_position);
        } else {
            // Failure: transfer possession to defender
            // FIX_2601/1120: Update ball position to defender's position to prevent teleportation
            // Without this, ball stays at dribbler's position and Step 9.5 snaps it to defender
            self.ball.current_owner = Some(nearest_defender);
            self.ball.position = self.player_positions[nearest_defender];
        }
    }
}
