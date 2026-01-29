//! Interactive Session Functions for MatchEngine
//!
//! This module contains functions for Phase E interactive simulation mode.
//! Extracted from mod.rs as part of P2-9 refactoring.
//!
//! ## Functions
//! - `build_user_decision_context()` - Build context for user decisions
//! - `check_and_build_intervention()` - Check if intervention needed
//! - `simulate_until_intervention()` - Run simulation until intervention
//! - `execute_direct_pass_to()` - Execute user-directed pass
//! - `resume_with_action()` - Resume after user action

use rand::Rng;

use super::super::physics_constants::{home_advantage, skills, zones};
use super::super::types::{ActionOptions, PassTarget, SimState, UserAction, UserDecisionContext};
use super::super::{coordinates, physics_constants};
use super::MatchEngine;
use crate::models::{MatchEvent, TeamSide};

impl MatchEngine {
    /// Internal helper for interactive mode: build a user decision context
    /// using existing skill/xG/pass/dribble calculations.
    pub(crate) fn build_user_decision_context(&self, player_idx: usize) -> UserDecisionContext {
        let is_home = TeamSide::is_home(player_idx);
        let attacks_right = self.attacks_right(is_home);
        let player_id = player_idx as u32;

        // FIX_2601: Position (Coord10 → meters)
        let pos_coord = self.get_player_position_by_index(player_idx);
        let pos_m = pos_coord.to_meters();

        // --- Shoot option: estimate xG based on distance + shooting-related skills ---
        // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
        let distance_m = coordinates::distance_to_goal_m(pos_coord.to_normalized_legacy(), attacks_right);

        // Derive an "accuracy" estimate from finishing/composure/technique, without RNG,
        // then feed it into the existing xG calculator.
        let finishing = skills::normalize(self.get_player_finishing(player_idx));
        let composure = skills::normalize(self.get_player_composure(player_idx));
        let technique = skills::normalize(self.get_player_technique(player_idx));

        let mut accuracy = (finishing * 0.5 + composure * 0.3 + technique * 0.2).clamp(0.2, 0.95);
        if is_home {
            accuracy = (accuracy + home_advantage::SHOT_ACCURACY_BONUS).min(0.95);
        }

        let shoot_prob =
            self.calculate_xg_skill_based(player_idx, distance_m, accuracy).clamp(0.0, 1.0);

        // --- Dribble option: success chance vs nearest defender (same as execute_dribble_action) ---
        let player_pos = self.get_player_position_by_index(player_idx);
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        let mut nearest_defender: Option<usize> = None;
        let mut nearest_dist = f32::MAX;
        for opp_idx in opponent_range.clone() {
            // FIX_2601: Use Coord10::distance_to_m()
            let opp_pos = self.get_player_position_by_index(opp_idx);
            let dist = player_pos.distance_to_m(&opp_pos);
            if dist < nearest_dist {
                nearest_dist = dist;
                nearest_defender = Some(opp_idx);
            }
        }

        let dribble_prob = if let Some(def_idx) = nearest_defender {
            // actions.rs의 dribble_success_probability 사용
            self.get_dribble_probability(player_idx, def_idx, is_home)
        } else {
            0.95 // 수비수 없으면 높은 성공률
        };

        // --- Pass options: success probability + "key pass" flag for advanced targets ---
        let mut pass_targets: Vec<PassTarget> = Vec::new();
        let teammate_range = if is_home { 0..11 } else { 11..22 };
        for mate_idx in teammate_range {
            if mate_idx == player_idx {
                continue;
            }

            let success_prob = self.calculate_pass_success(player_idx, mate_idx).clamp(0.0, 1.0);

            // Consider a pass "key" if the receiver is significantly closer to goal
            // and in a more advanced vertical position (y-axis) than the passer.
            let mate_pos = self.get_player_position_by_index(mate_idx);
            // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
            let receiver_dist_m =
                coordinates::distance_to_goal_m(mate_pos.to_normalized_legacy(), attacks_right);

            // FIX_2601: Compare positions using normalized legacy (length coordinate)
            // FIX_2601/0110: Use attacks_right for correct direction in both halves
            let mate_pos_norm = mate_pos.to_normalized_legacy();
            let player_pos_norm = player_pos.to_normalized_legacy();
            let is_more_advanced = if attacks_right {
                mate_pos_norm.1 > player_pos_norm.1
            } else {
                mate_pos_norm.1 < player_pos_norm.1
            };

            let is_close_to_goal = receiver_dist_m < zones::CLOSE_M;
            let is_key_pass = is_more_advanced && is_close_to_goal;

            pass_targets.push(PassTarget { id: mate_idx as u32, success_prob, is_key_pass });
        }

        UserDecisionContext {
            player_id,
            time_seconds: self.current_timestamp_ms as f32 / 1000.0,
            position_m: pos_m,
            options: ActionOptions { shoot_prob, dribble_prob, pass_targets },
        }
    }

    /// Phase E: check if we should pause for user intervention and, if so,
    /// build a `UserDecisionContext` for the current ball holder.
    pub(crate) fn check_and_build_intervention(&mut self) -> Option<UserDecisionContext> {
        // Require a configured user player
        self.user_player.as_ref()?;

        // Only intervene when someone has the ball
        let owner_idx = self.ball.current_owner?;

        let is_home = TeamSide::is_home(owner_idx);

        // Only consider moments when the ball holder IS the configured "my player"
        if !self.is_user_controlled_player(owner_idx, is_home) {
            return None;
        }

        // Basic cooldown: avoid pausing too frequently (e.g. every few seconds)
        const INTERVENTION_COOLDOWN_MS: u64 = 10_000;
        let approx_now_ms = if self.current_timestamp_ms > 0 {
            self.current_timestamp_ms
        } else {
            (self.minute as u64) * 60_000
        };

        if self.last_intervention_ms != 0
            && approx_now_ms < self.last_intervention_ms + INTERVENTION_COOLDOWN_MS
        {
            return None;
        }

        // Build decision context and derive heuristics from it
        let ctx = self.build_user_decision_context(owner_idx);

        let shoot_prob = ctx.options.shoot_prob;
        let dribble_prob = ctx.options.dribble_prob;

        let has_high_quality_key_pass =
            ctx.options.pass_targets.iter().any(|t| t.is_key_pass && t.success_prob >= 0.75);

        // Estimate defensive pressure around the ball holder (0.0 ~ 1.0)
        let pressure_penalty = self.calculate_pressure_penalty(owner_idx);
        let max_penalty = physics_constants::pressure::MAX_PENALTY;
        let pressure =
            if max_penalty > 0.0 { (pressure_penalty / max_penalty).clamp(0.0, 1.0) } else { 0.0 };

        // Simple attacking third heuristic based on X position (field length).
        let pos_m = ctx.position_m;
        // FIX_2601/0109: Use attacks_right for correct second-half direction
        let attacks_right = self.attacks_right(is_home);
        let is_attacking_third = if attacks_right { pos_m.0 > 70.0 } else { pos_m.0 < 35.0 };

        // Intervention triggers:
        //  - High xG chance in attacking third
        //  - Heavy pressure while dribble chance is still decent
        //  - Very strong key pass option
        let should_pause = (is_attacking_third && shoot_prob >= 0.12)
            || (pressure >= 0.8 && dribble_prob >= 0.35)
            || has_high_quality_key_pass;

        if !should_pause {
            return None;
        }

        // Record last intervention timestamp and return context
        self.last_intervention_ms = approx_now_ms;
        Some(ctx)
    }

    /// Phase E: experimental interactive simulation entry point.
    /// This method mirrors `simulate()` but is structured to allow
    /// insertion of intervention points. It uses the same init/step
    /// helpers as the budget API and will pause when
    /// `check_and_build_intervention()` returns a context.
    pub fn simulate_until_intervention(&mut self) -> SimState {
        let (home_strength, away_strength, possession_ratio, match_duration) = self.init();

        while self.step(home_strength, away_strength, possession_ratio, match_duration) {
            if let Some(ctx) = self.check_and_build_intervention() {
                return SimState::Paused(ctx);
            }
        }

        let result = self.finalize(possession_ratio);
        SimState::Finished(result)
    }

    /// Helper for interactive mode: execute a direct pass to a specific target
    /// index, using the same success model as `calculate_pass_success` but
    /// bypassing the automatic target selection in `execute_pass_action`.
    pub(crate) fn execute_direct_pass_to(&mut self, from_idx: usize, to_idx: usize, is_home: bool) {
        self.execute_direct_pass_to_internal(from_idx, to_idx, is_home, None);
    }

    /// Scenario/test helper: execute a direct pass with an optional success override.
    /// When `success_override` is Some(true/false), RNG is bypassed.
    pub fn execute_direct_pass_to_with_override(
        &mut self,
        from_idx: usize,
        to_idx: usize,
        is_home: bool,
        success_override: Option<bool>,
    ) {
        self.execute_direct_pass_to_internal(from_idx, to_idx, is_home, success_override);
    }

    fn execute_direct_pass_to_internal(
        &mut self,
        from_idx: usize,
        to_idx: usize,
        is_home: bool,
        success_override: Option<bool>,
    ) {
        // Basic bounds check
        if to_idx >= 22 {
            self.assign_possession_to_nearest_defender(is_home);
            return;
        }

        let success_rate = self.calculate_pass_success(from_idx, to_idx);
        let success = match success_override {
            Some(value) => value,
            None => self.rng.gen::<f32>() < success_rate,
        };

        if success {

            // Apply offside rule for direct passes (match_sim parity)
            if self.is_offside_pass(from_idx, to_idx, is_home) {
                let receiver_pos = self.get_player_position_by_index(to_idx);
                self.emit_event(MatchEvent::offside(
                    self.minute,
                    self.current_timestamp_ms(),
                    is_home,
                    to_idx,
                ));

                if let Some(recorder) = self.replay_recorder.as_mut() {
                    let t_seconds = self.current_tick as f64 * 0.5;
                    let team_id = if is_home { 0 } else { 1 };
                    let receiver_pos_m = receiver_pos.to_meters();
                    recorder.record_offside(
                        t_seconds,
                        team_id,
                        to_idx as u32,
                        crate::replay::types::MeterPos {
                            x: receiver_pos_m.0 as f64,
                            y: receiver_pos_m.1 as f64,
                        },
                    );
                }

                // FIX_2601/0112: Statistics updated via events in stats.update_from_events()
                // Only update internal counters here
                if is_home {
                    self.offside_count_home += 1;
                } else {
                    self.offside_count_away += 1;
                }

                self.apply_offside_restart(is_home, receiver_pos);
                return;
            }

            if self.track_positions {
                let from_pos = self.get_player_position_by_index(from_idx);
                let to_pos = self.get_player_position_by_index(to_idx);

                // FIX_2601: Convert to normalized for distance and height profile
                let from_pos_norm = from_pos.to_normalized_legacy();
                let to_pos_norm = to_pos.to_normalized_legacy();

                let distance = ((to_pos_norm.0 - from_pos_norm.0).powi(2)
                    + (to_pos_norm.1 - from_pos_norm.1).powi(2))
                .sqrt();
                let base_speed = 2.5; // Treat as a normal ground pass
                let flight_speed = base_speed / distance.max(0.1);

                // to_pos is already Coord10
                self.ball.height_profile = self.determine_pass_height_profile(
                    from_pos_norm,
                    to_pos_norm,
                    distance,
                    self.ball.height as f32 / 10.0,
                    is_home,
                );

                self.ball.start_flight(to_pos, flight_speed, Some(to_idx));
            } else {
                self.ball.current_owner = Some(to_idx);
            }
        } else {
            self.assign_possession_to_nearest_defender(is_home);
        }
    }

    /// Phase E: resume interactive simulation after a user action.
    /// Uses cached init() context to continue from the current state,
    /// applying the chosen `UserAction` first and then running until
    /// the next intervention point or the end of the match.
    pub fn resume_with_action(&mut self, action: UserAction) -> SimState {
        let home_strength = self.precomputed_home_strength;
        let away_strength = self.precomputed_away_strength;
        let possession_ratio = self.precomputed_possession_ratio;
        let match_duration = self.precomputed_match_duration;

        // Ensure added time is finalized before checking end-of-match.
        if !self.is_second_half && self.minute >= super::HALF_DURATION_MINUTES {
            self.maybe_finalize_first_half_stoppage_time();
        }
        if self.is_second_half && self.minute >= self.regulation_end_minute() {
            self.maybe_finalize_second_half_stoppage_time();
        }
        let match_end_minute = self.match_end_minute.min(match_duration);

        // Safety: if the match is already beyond duration, just finalize
        if self.minute > match_end_minute {
            let result = self.finalize(possession_ratio);
            return SimState::Finished(result);
        }

        // Determine the acting player: by default, the current ball owner.
        if let Some(owner_idx) = self.ball.current_owner {
            let is_home = TeamSide::is_home(owner_idx);

            match action {
                UserAction::Shoot => {
                    // Use the same pattern as execute_dribble_action when it
                    // decides to shoot directly, but force the shot now.
                    let player_name = self.get_match_player(owner_idx).name.clone();

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

                    self.execute_shot_action(
                        is_home,
                        &player_name,
                        attack_strength,
                        defense_strength,
                    );
                }
                UserAction::Dribble => {
                    self.execute_dribble_action(owner_idx, is_home);
                }
                UserAction::PassTo(target_id) => {
                    let target_idx = target_id as usize;
                    self.execute_direct_pass_to(owner_idx, target_idx, is_home);
                }
            }
        }

        // After applying the user action, continue simulation until the next
        // intervention point or the end of the match.
        while self.step(home_strength, away_strength, possession_ratio, match_duration) {
            if let Some(ctx) = self.check_and_build_intervention() {
                return SimState::Paused(ctx);
            }
        }

        let result = self.finalize(possession_ratio);
        SimState::Finished(result)
    }
}
