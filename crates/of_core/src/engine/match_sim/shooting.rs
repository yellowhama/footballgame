//! Shooting System - Shot Execution and GK Save Mechanics
//!
//! This module contains shooting-related logic for MatchEngine:
//! - Shot execution with pure function pattern
//! - xG calculation
//! - GK save probability
//! - Ball control and skill calculations
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::{attribute_calc, MatchEngine};
use crate::engine::actions::{self, DribbleContext, ShotContext, ShotRolls};
use crate::engine::ball::{CurveLevel, HeightProfile};
use crate::engine::coordinates;
use crate::engine::physics_constants::{aerial, field, goal, home_advantage, skills};
use crate::engine::player_decision::PlayerDecision;
use crate::engine::probability;
use crate::engine::types::coord10::{Coord10, Vel10};
use crate::models::trait_balance::get_gold_balance;
use crate::models::trait_system::TraitId;
use crate::models::MatchEvent;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // Shot Execution (Pure Function Pattern)
    // ===========================================

    /// Execute shot action with pure function pattern
    pub(crate) fn execute_shot_action(
        &mut self,
        is_home: bool,
        shooter: &str,
        _attack_strength: f32,
        _defense_strength: f32,
    ) {
        // Find shooter index
        let player_idx = self.find_player_idx_by_name(is_home, shooter);

        // Calculate distance to goal
        // FIX_2601: Convert Coord10 to normalized for legacy functions
        // FIX_2601/0109: Use attacks_right for correct second-half goal calculation
        let attacks_right = self.attacks_right(is_home);
        let player_pos_coord = self.get_player_position_by_index(player_idx);
        let player_pos = player_pos_coord.to_normalized_legacy();
        let distance_m = coordinates::distance_to_goal_m(player_pos, attacks_right);

        // GK index (home shooting -> away GK(11), away shooting -> home GK(0))
        let gk_idx = if is_home { 11 } else { 0 };

        // ============================================
        // Create ShotContext (Pure Function Pattern)
        // ============================================
        let ctx = ShotContext {
            shooter_idx: player_idx,
            shooter_name: self.get_player_name(player_idx),
            is_home,
            player_pos,
            distance_m,

            finishing: self.get_player_finishing(player_idx),
            composure: self.get_player_composure(player_idx),
            technique: self.get_player_technique(player_idx),
            long_shots: self.get_player_long_shots(player_idx),
            strength: self.get_player_strength(player_idx),

            positional_modifier: self.get_positional_skill_rating(player_idx, "shooting"),
            under_pressure: false, // Pressure handling in separate logic
            ball_height: self.ball.height as f32 / 10.0,

            has_cannon: self.player_has_gold_trait(player_idx, TraitId::Cannon),
            has_sniper: self.player_has_gold_trait(player_idx, TraitId::Sniper),
            has_lob_master: self.player_has_gold_trait(player_idx, TraitId::LobMaster),
            has_acrobat: self.player_has_gold_trait(player_idx, TraitId::Acrobat),

            gk_idx,
            // GK stat mapping - v5 캐시: 실제 GK 속성 사용
            gk_reflexes: self.get_player_gk_reflexes(gk_idx),
            gk_positioning: self.get_player_positioning(gk_idx),
            gk_handling: self.get_player_gk_handling(gk_idx),
            gk_diving: self.get_player_agility(gk_idx), // No dedicated diving attr, use agility
        };

        // ============================================
        // Create ShotRolls
        // ============================================
        let rolls = ShotRolls {
            power_variance: self.rng.gen(),
            accuracy_variance: self.rng.gen(),
            on_target_roll: self.rng.gen(),
            goal_roll: self.rng.gen(),
            save_roll: self.rng.gen(),
        };

        // ============================================
        // Pure Function Call
        // ============================================
        let result = actions::resolve_shot(&ctx, &rolls);

        // Extract values from result
        let shot_power = result.shot_power;
        let accuracy = if is_home {
            (result.accuracy + home_advantage::SHOT_ACCURACY_BONUS).min(0.95)
        } else {
            result.accuracy
        };
        let on_target = result.on_target;
        let xg = result.xg;

        // P2: auto timestamp via emit_event
        let (ball_x_m, ball_y_m) = self.ball.position.to_meters();
        let ball_height_m = self.ball.height as f32 / 10.0;
        self.emit_event(MatchEvent::shot_with_position(
            self.minute,
            self.current_timestamp_ms(),
            is_home,
            player_idx, // C6: Use track_id directly
            on_target,
            xg,
            (
                ball_x_m / field::LENGTH_M,
                ball_y_m / field::WIDTH_M,
                ball_height_m,
            ),
        ));

        // Statistics update - shots count (xG recorded later, after height/post checks)
        if is_home {
            self.result.statistics.shots_home += 1;
            if on_target {
                self.result.statistics.shots_on_target_home += 1;
            }
        } else {
            self.result.statistics.shots_away += 1;
            if on_target {
                self.result.statistics.shots_on_target_away += 1;
            }
        }
        // NOTE: Shot budget tracking done in record_shot_attempt() only

        // Phase 3: Start ball flight to goal (only if position tracking enabled)
        if self.track_positions {
            // Goal position (center of goal) in legacy (width, length) format
            // This matches player_pos from to_normalized_legacy()
            // FIX_2601/0109: Use attacks_right for correct second-half goal position
            let goal_pos_legacy = if attacks_right {
                (0.5, 1.0) // Attacking right: width=0.5 (center), length=1.0 (far end)
            } else {
                (0.5, 0.0) // Attacking left: width=0.5 (center), length=0.0 (near end)
            };

            // Shot speed based on power and distance
            let speed_multiplier = 3.0 + shot_power; // Faster than passes
            let distance_to_goal = ((goal_pos_legacy.0 - player_pos.0).powi(2)
                + (goal_pos_legacy.1 - player_pos.1).powi(2))
            .sqrt();
            let flight_speed = speed_multiplier / distance_to_goal.max(0.1);

            // A1: Apply curve_factor based on shooter's skill level
            if let Some(shooter_player) = self.get_player(player_idx) {
                let curve_level = shooter_player.get_curve_level();

                // Generate random curve_factor within the level's range
                let curve_factor = match curve_level {
                    CurveLevel::None => self.rng.gen_range(-0.05..=0.05),
                    CurveLevel::Lv1 => self.rng.gen_range(-0.10..=0.10),
                    CurveLevel::Lv2 => self.rng.gen_range(-0.20..=0.20),
                    CurveLevel::Lv3 => self.rng.gen_range(-0.35..=0.35),
                };

                self.ball.set_curve_factor(curve_factor); // D5-2: Use validated setter

                // FIX_2601/0112: Set spin for Magnus effect based on curve level
                let spin_direction = if curve_factor >= 0.0 { 1.0 } else { -1.0 };
                self.ball.set_spin_from_curve(curve_level, spin_direction);
            } else {
                // Fallback: no curve for invalid player
                self.ball.set_curve_factor(0.0); // D5-2: Use validated setter
                self.ball.reset_spin(); // FIX_2601/0112: No spin for fallback
            }

            // A4: Apply height_profile based on shot situation
            // FIX_2601/0104: Convert legacy (width, length) to from_meters(x, y) = (length*105, width*68)
            // goal_pos_legacy.1 is length (0 or 1) → x in meters (0 or 105)
            // goal_pos_legacy.0 is width (0.5) → y in meters (34)
            let goal_coord = Coord10::from_normalized_legacy(goal_pos_legacy);
            self.ball.height_profile = self.determine_shot_height_profile(
                player_pos,
                goal_pos_legacy,
                distance_to_goal,
                self.ball.height as f32 / 10.0,
                is_home,
            );

            self.ball.start_flight(goal_coord, flight_speed, None);
        }

        // FIX_2601/0109 v5: xG-based goal decision
        // xG = P(goal | shot), already accounts for distance, accuracy, and shot difficulty
        // Goal decision uses xG directly with small GK adjustment
        //
        // on_target shots: additional height check, post/bar check, save events
        // off_target shots: just xG-based goal probability
        let gk_idx = if is_home { 11 } else { 0 };
        let balance = get_gold_balance();

        // Shot height (used for on_target shot processing)
        let shot_height = match self.ball.height_profile {
            HeightProfile::Flat => 0.5,
            HeightProfile::Arc => 1.5,
            HeightProfile::Lob => 2.2,
        };

        // GK save probability for modifying goal chance
        let mut save_prob =
            self.calculate_gk_save_probability(gk_idx, shot_height, shot_power, distance_m);

        // Gold Trait effects on save probability
        if distance_m >= balance.cannon_min_distance
            && self.player_has_gold_trait(player_idx, TraitId::Cannon)
        {
            save_prob = balance.cannon_gk_save_prob;
        }
        if matches!(self.ball.height_profile, HeightProfile::Lob)
            && self.player_has_gold_trait(player_idx, TraitId::LobMaster)
        {
            save_prob = balance.lob_master_gk_save_prob;
        }

        // Process on_target shots: height check, post/bar check
        if on_target {
            // Height check - shots over crossbar cannot score
            if shot_height > goal::HEIGHT_M {
                let opponent_gk_idx = if is_home { 11 } else { 0 };
                self.ball.current_owner = Some(opponent_gk_idx);
                let gk_pos = self.get_player_position_by_index(opponent_gk_idx);
                self.ball.position = gk_pos;
                self.ball.velocity = Vel10::from_mps(0.0, 0.0);
                return;
            }

            // Post/bar collision check
            let post_bar_chance =
                self.calculate_post_bar_collision_chance(accuracy, shot_height, distance_m);
            if self.rng.gen::<f32>() < post_bar_chance {
                let is_crossbar = shot_height > 1.8;
                if is_crossbar {
                    self.emit_event(MatchEvent::bar_hit(
                        self.minute, self.current_timestamp_ms(), is_home, player_idx,
                    ));
                } else {
                    self.emit_event(MatchEvent::post_hit(
                        self.minute, self.current_timestamp_ms(), is_home, player_idx,
                    ));
                }
                // 15% rebound goal
                if self.rng.gen::<f32>() < 0.15 {
                    self.score_goal(is_home, player_idx);
                }
                return;
            }
        }

        // Gold Sniper/Acrobat effects
        let mut effective_save_prob = save_prob;
        if accuracy >= balance.sniper_accuracy_threshold
            && self.player_has_gold_trait(player_idx, TraitId::Sniper)
        {
            effective_save_prob *= 0.5;
        }
        let ball_height_m = self.ball.height as f32 / 10.0;
        if self.ball.height_profile == HeightProfile::Arc
            && ball_height_m > 1.0
            && self.player_has_gold_trait(player_idx, TraitId::Acrobat)
        {
            effective_save_prob *= 0.7;
        }

        // Goal decision: xG-based with GK adjustment (only for on_target shots)
        // FIX_2601/0109 v6: xG recorded here (after height/post checks) for accurate conversion
        // GK quality adjusts goal probability by ±10% (save_prob 0.40 = neutral)
        if on_target {
            // Record xG AFTER height/post checks pass (for accurate goals/xG ratio)
            if is_home {
                self.result.statistics.xg_home += xg;
            } else {
                self.result.statistics.xg_away += xg;
            }

            let gk_factor = 1.0 - (effective_save_prob - 0.40) * 0.25;
            let goal_prob = (xg * gk_factor).clamp(0.02, 0.95);

            if self.rng.gen::<f32>() < goal_prob {
                self.score_goal(is_home, player_idx);
            } else {
                // Save event for shots that didn't score
                let save_event_prob = (effective_save_prob * 0.6).min(0.50);
                if self.rng.gen::<f32>() < save_event_prob {
                    self.emit_event(
                        MatchEvent::save(self.minute, self.current_timestamp_ms(), !is_home, gk_idx)
                            .with_target_track_id(Some(player_idx)),
                    );
                }
            }
        }
        // off_target shots just end here (no goal possible, no xG recorded)

        // State reset: After a shot (that wasn't a goal), possession changes
        // Shot Volume Tuning v4: 50% opponent GK, 50% loose ball
        // More balanced - allows for rebounds but prevents pure shot spam
        if self.rng.gen::<f32>() < 0.5 {
            let opponent_gk_idx = if is_home { 11 } else { 0 };
            self.ball.current_owner = Some(opponent_gk_idx);
            let gk_pos = self.get_player_position_by_index(opponent_gk_idx);
            // FIX_2601: gk_pos is already Coord10, use directly
            self.ball.position = gk_pos;
        } else {
            // Loose ball - next minute will assign based on possession ratio
            self.ball.current_owner = None;
        }
        self.ball.velocity = Vel10::from_mps(0.0, 0.0);
    }

    // ===========================================
    // xG and Skill Calculations
    // ===========================================

    /// xG calculation (skill based) - probability module wrapper
    pub(crate) fn calculate_xg_skill_based(
        &self,
        _player_idx: usize,
        distance_m: f32,
        accuracy: f32,
    ) -> f32 {
        probability::xg_skill_based(distance_m, accuracy)
    }

    /// Header success rate calculation - probability module wrapper
    pub(crate) fn calculate_header_success(&self, player_idx: usize, contest: bool) -> f32 {
        probability::header_success(
            self.get_player_heading(player_idx),
            self.get_player_jumping(player_idx),
            self.get_player_positioning(player_idx),
            self.get_player_strength(player_idx),
            contest,
        )
    }

    /// Dribble success probability calculation (for ActionOptions etc.)
    /// Wrapper for actions.rs dribble_success_probability
    pub(crate) fn get_dribble_probability(
        &self,
        dribbler_idx: usize,
        defender_idx: usize,
        is_home: bool,
    ) -> f32 {
        use crate::models::SpecialSkill;
        use crate::player::skill_system::SkillCalculator;

        // A13: SpeedDemon skill check
        let has_speed_demon = if let Some(dribbler) = self.get_player(dribbler_idx) {
            dribbler.has_skill(SpecialSkill::SpeedDemon)
        } else {
            false
        };

        // PlayerInstructions contribution calculation
        let dribbler_instr = self.get_player_instructions(dribbler_idx);
        let dribbler_pos_str = self.get_position_string_by_idx(dribbler_idx);
        let off_contrib =
            PlayerDecision::calculate_offensive_contribution(&dribbler_instr, &dribbler_pos_str);

        let defender_instr = self.get_player_instructions(defender_idx);
        let defender_pos_str = self.get_position_string_by_idx(defender_idx);
        let def_contrib =
            PlayerDecision::calculate_defensive_contribution(&defender_instr, &defender_pos_str);

        // FIX_2601: Convert Coord10 to normalized for DribbleContext
        let dribbler_pos = self.player_positions[dribbler_idx].to_normalized_legacy();
        let ctx = DribbleContext {
            dribbler_idx,
            dribbler_name: self.get_player_name(dribbler_idx),
            is_home,
            current_pos: dribbler_pos,
            target_pos: dribbler_pos, // Same for probability calculation
            dribbling: self.get_player_dribbling(dribbler_idx),
            agility: self.get_player_agility(dribbler_idx),
            balance: self.get_player_balance(dribbler_idx),
            pace: self.get_player_pace(dribbler_idx),
            defender_idx: Some(defender_idx),
            defender_marking: Some(self.get_player_marking(defender_idx)),
            defender_positioning: Some(self.get_player_positioning(defender_idx)),
            defender_pace: Some(self.get_player_pace(defender_idx)),
            offensive_modifier: 0.9 + (off_contrib * 0.1),
            defensive_modifier: 0.9 + (def_contrib * 0.1),
            has_speed_demon,
            has_speedster: self.player_has_gold_trait(dribbler_idx, TraitId::Speedster),
            has_technician: self.player_has_gold_trait(dribbler_idx, TraitId::Technician),
            has_tank: self.player_has_gold_trait(dribbler_idx, TraitId::Tank),
            defender_has_shadow: self.player_has_gold_trait(defender_idx, TraitId::Shadow),
            defender_has_bully: self.player_has_gold_trait(defender_idx, TraitId::Bully),
        };

        actions::dribble_success_probability(&ctx)
    }

    /// A11: Ball control success calculation
    pub(crate) fn calculate_ball_control_success(
        &self,
        player_idx: usize,
        ball_speed: f32,
        ball_height: f32,
    ) -> f32 {
        let first_touch = skills::normalize(self.get_player_first_touch(player_idx));
        let technique = skills::normalize(self.get_player_technique(player_idx));
        let composure = skills::normalize(self.get_player_composure(player_idx));

        let base_control = first_touch * 0.5 + technique * 0.3 + composure * 0.2;

        // Speed penalty (faster = harder, max -30%)
        let speed_penalty = (ball_speed / 30.0).min(0.3);

        // Height penalty (higher = harder, max -20%)
        let height_penalty = (ball_height / 2.0).min(0.2);

        let mut success = (base_control - speed_penalty - height_penalty).clamp(0.2, 0.95);

        // ============================================
        // Gold Trait Balance Effects
        // ============================================

        // Gold Magnet: Perfect control of any pass
        if self.player_has_gold_trait(player_idx, TraitId::Magnet) {
            success = 0.98; // Near perfect control
        }

        success
    }

    // ===========================================
    // GK Save Mechanics (aerial constants sync)
    // ===========================================

    /// GK save probability calculation (FIX_2601/0109: 통합 함수 사용)
    ///
    /// - Shot height > GK_CATCH_MAX_M (3.3m): Can't save (overhead shot)
    /// - 통합 함수 사용 + Gold Trait 효과 적용
    pub(crate) fn calculate_gk_save_probability(
        &self,
        gk_idx: usize,
        shot_height: f32,
        shot_power: f32,
        distance_m: f32,
    ) -> f32 {
        // Ball exceeds GK reach range -> can't save
        if shot_height > aerial::GK_CATCH_MAX_M {
            return 0.0;
        }

        // Extract GK skills - v5 캐시: 실제 GK 속성 사용
        let reflexes = self.get_player_gk_reflexes(gk_idx);
        let positioning = self.get_player_positioning(gk_idx);
        let handling = self.get_player_gk_handling(gk_idx);
        let diving = self.get_player_agility(gk_idx); // No dedicated diving, use agility

        // shot_power를 속도로 변환 (power 35 ≈ 30 m/s)
        let ball_speed_mps = shot_power * 0.85;

        // FIX_2601/0109: 통합 함수 사용
        let mut final_save = attribute_calc::calculate_gk_save_prob_unified(
            reflexes,
            positioning,
            handling,
            diving,
            distance_m,
            ball_speed_mps,
            shot_height,
            distance_m < 10.0, // is_one_on_one: 근거리는 1v1로 간주
        );

        // ============================================
        // GK Gold Trait Balance Effects
        // ============================================

        // Gold Spider: Excellent at saving curving shots (+30%)
        // Curving shots tend to be mid height (1.0~2.5m range)
        if shot_height > 1.0
            && shot_height < 2.5
            && self.player_has_gold_trait(gk_idx, TraitId::Spider)
        {
            final_save = (final_save + 0.30).min(0.95);
        }

        // Gold Sweeper: Close range 1v1 save success rate bonus
        // FIX_2601/0106: Use additive +15% instead of multiplicative 2x to avoid overflow
        if distance_m < 10.0 && self.player_has_gold_trait(gk_idx, TraitId::Sweeper) {
            final_save = (final_save + 0.15).min(0.85);
        }

        final_save
    }

    // ===========================================
    // P5.2: Post/Bar Collision Calculation
    // ===========================================

    /// Calculate probability of hitting post or crossbar
    /// Higher accuracy = aiming for corners = higher chance of hitting woodwork
    /// Real football: ~3-5% of shots hit post/bar
    fn calculate_post_bar_collision_chance(
        &self,
        accuracy: f32,
        shot_height: f32,
        distance_m: f32,
    ) -> f32 {
        // Base chance: accurate shots aim for corners -> more likely to hit post
        let base_chance: f32 = if accuracy > 0.75 {
            0.08 // Very accurate shot aiming for corner
        } else if accuracy > 0.55 {
            0.05 // Decent accuracy
        } else {
            0.02 // Low accuracy - more likely to miss entirely
        };

        // Distance factor: longer shots less likely to hit post precisely
        let distance_factor: f32 = if distance_m > 25.0 {
            0.7 // Long range: harder to hit post
        } else if distance_m < 10.0 {
            1.2 // Close range: easier to hit post
        } else {
            1.0
        };

        // Height factor: shots near crossbar height more likely to hit bar
        let height_factor: f32 = if shot_height > 2.0 && shot_height <= goal::HEIGHT_M {
            1.3 // Near crossbar
        } else if shot_height < 0.5 {
            1.1 // Low shot near post base
        } else {
            1.0
        };

        (base_chance * distance_factor * height_factor).min(0.12) // Cap at 12%
    }
}
