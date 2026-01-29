//! Tackle System
//!
//! This module contains tackle-related logic for MatchEngine:
//! - Tackle action execution (A8 mechanic)
//! - Foul and card handling
//! - Injury from foul check (P3)
//!
//! Extracted from match_sim/mod.rs for better organization.
//!
//! FIX_2601/0123: Integrated with RuleDispatcher for A/B comparison tracking.

use super::rules::{
    check_foul_wrapper, Card, ContactEvent, FoulType as RuleFoulType,
    LegacyFoulResult, RuleDecision, RuleTeamId,
};
use super::MatchEngine;
use crate::engine::actions::{self, TackleContext, TackleResult, TackleRolls};
use crate::engine::physics_constants::{field, skills};
use crate::engine::player_decision::PlayerDecision;
use crate::engine::types::coord10::Coord10;
use crate::models::rules::{FoulDetails, FoulSeverity, FoulType};
use crate::models::trait_system::TraitId;
use crate::models::{MatchEvent, SpecialSkill, TeamSide};
use crate::player::skill_system::SkillCalculator;
use rand::Rng;

// =============================================================================
// Phase 3: Foul Severity Calculation (Law 12)
// =============================================================================

/// Calculate foul severity based on tackle result and context
///
/// IFAB Law 12 defines three levels:
/// - Careless: lack of attention or consideration (Direct FK only)
/// - Reckless: disregard for danger or consequences (Yellow card)
/// - Excessive Force: far exceeds necessary force (Red card)
fn calculate_foul_severity(result: &TackleResult) -> FoulSeverity {
    if result.red_card {
        FoulSeverity::ExcessiveForce
    } else if result.yellow_card {
        FoulSeverity::Reckless
    } else {
        FoulSeverity::Careless
    }
}

/// Build FoulDetails from tackle context and result
fn build_foul_details(
    result: &TackleResult,
    victim_track_id: usize,
    in_penalty_area: bool,
    aggression: f32,
) -> FoulDetails {
    let severity = calculate_foul_severity(result);

    // Determine if this could be DOGSO (Denying Obvious Goal-Scoring Opportunity)
    // Simplified: check if severity is high enough and in dangerous area
    // In a more complete implementation, this would check:
    // - Distance to goal
    // - Number of defenders between attacker and goal
    // - Direction of play
    // - Likelihood of keeping/gaining control
    let is_dogso = result.red_card && !in_penalty_area; // DOGSO outside penalty area

    // Attempted to play ball: inversely related to aggression
    // High aggression = less likely to be playing the ball
    let attempted_to_play_ball = aggression < 0.6;

    FoulDetails {
        severity,
        foul_type: Some(FoulType::Tackling), // Tackle fouls are tackling type
        is_dogso,
        in_penalty_area,
        victim_track_id: Some(victim_track_id as u8),
        attempted_to_play_ball,
    }
}

impl MatchEngine {
    // ===========================================
    // A8: Tackle Game Mechanic
    // ===========================================

    /// Execute tackle action (ball possession contest)
    /// Refactored: uses actions::resolve_tackle pure function (2025-12-06)
    pub(crate) fn execute_tackle_action(&mut self, tackler_idx: usize, ball_holder_idx: usize) {
        // ============================================
        // 1. Create TackleContext (data collection)
        // ============================================

        // A13: PerfectTackle skill check
        let has_perfect_tackle = if let Some(tackler) = self.get_player(tackler_idx) {
            tackler.has_skill(SpecialSkill::PerfectTackle)
        } else {
            false
        };

        // Calculate defensive_contribution based on PlayerInstructions
        let instructions = self.get_player_instructions(tackler_idx);
        let position_str = self.get_position_string_by_idx(tackler_idx);
        let def_contribution =
            PlayerDecision::calculate_defensive_contribution(&instructions, &position_str);
        let instruction_modifier = 0.8 + (def_contribution * 0.2); // 0.8 ~ 1.1 range

        let ctx = TackleContext {
            tackler_idx,
            tackler_name: self.get_player_name(tackler_idx),
            ball_holder_idx,
            ball_holder_name: self.get_player_name(ball_holder_idx),
            is_home: TeamSide::is_home(tackler_idx),
            // Tackler stats
            tackling: self.get_player_tackling(tackler_idx),
            anticipation: self.get_player_anticipation(tackler_idx),
            aggression: self.get_player_aggression(tackler_idx),
            positioning: self.get_player_positioning(tackler_idx),
            // Ball holder stats
            dribbling: self.get_player_dribbling(ball_holder_idx),
            balance: self.get_player_balance(ball_holder_idx),
            // PlayerInstructions result
            instruction_modifier,
            // Gold Traits
            has_vacuum: self.player_has_gold_trait(tackler_idx, TraitId::Vacuum),
            has_wall: self.player_has_gold_trait(tackler_idx, TraitId::Wall),
            has_reader: self.player_has_gold_trait(tackler_idx, TraitId::Reader),
            has_bully: self.player_has_gold_trait(tackler_idx, TraitId::Bully),
            // A13: PerfectTackle
            has_perfect_tackle,
        };

        // ============================================
        // 2. Create TackleRolls (RNG calls)
        // ============================================
        let rolls = TackleRolls {
            success_roll: self.rng.gen(),
            foul_roll: self.rng.gen(),
            card_roll: self.rng.gen(),
            injury_roll: self.rng.gen(),
        };

        // ============================================
        // 3. Call pure function
        // ============================================
        let result = actions::resolve_tackle(&ctx, &rolls);

        // ============================================
        // 4. Apply result + game logic
        // ============================================
        if result.success {
            // Emit successful tackle event (for event tracking/replay)
            let is_home = TeamSide::is_home(tackler_idx);
            let (ball_x_m, ball_y_m) = self.ball.position.to_meters();
            let ball_height_m = self.ball.height as f32 / 10.0;
            // C6: Use tackler_idx directly as track_id
            self.emit_event(
                MatchEvent::tackle(
                    self.minute,
                    self.current_timestamp_ms(),
                    is_home,
                    tackler_idx,
                    (ball_x_m / field::LENGTH_M, ball_y_m / field::WIDTH_M, ball_height_m),
                )
                .with_target_track_id(Some(ball_holder_idx)),
            );

            self.ball.current_owner = Some(tackler_idx);
            self.ball.previous_owner = Some(ball_holder_idx);

            // A13: PerfectTackle - instant perfect ball possession
            if has_perfect_tackle {
                // FIX_2601: tackler_pos is already Coord10, use directly
                let tackler_pos = self.player_positions[tackler_idx];
                self.ball.position = tackler_pos;
            } else {
                // Ball bounces slightly
                // FIX_2601: Convert to normalized for offset, then back to Coord10
                let tackler_pos = self.player_positions[tackler_idx].to_normalized_legacy();
                let random_offset = self.rng.gen_range(-0.01..0.01);
                let new_ball_pos = (
                    (tackler_pos.0 + random_offset).clamp(0.0, 1.0),
                    (tackler_pos.1 + random_offset).clamp(0.0, 1.0),
                );
                self.ball.position = Coord10::from_normalized_legacy(new_ball_pos);
            }
        } else if result.foul {
            // Foul occurred - free kick/penalty kick judgment
            let foul_position = self.ball.position;
            // FIX_2601/0104: Use to_normalized_legacy() for NormalizedPos (width, length) format
            // This is expected by distance_to_goal_m() and other coordinate functions
            let foul_pos_norm = foul_position.to_normalized_legacy();
            let is_home = TeamSide::is_home(tackler_idx);
            let attacking_home = !is_home;

            // Calculate penalty area status BEFORE emitting event (needed for FoulDetails)
            let (foul_x_m, foul_y_m) = foul_position.to_meters();
            let penalty_width_m = 40.66;
            let y_min = (field::WIDTH_M - penalty_width_m) / 2.0;
            let y_max = y_min + penalty_width_m;
            let in_penalty_area = if attacking_home {
                foul_x_m >= (field::LENGTH_M - field::PENALTY_AREA_LENGTH_M)
                    && foul_y_m >= y_min
                    && foul_y_m <= y_max
            } else {
                foul_x_m <= field::PENALTY_AREA_LENGTH_M && foul_y_m >= y_min && foul_y_m <= y_max
            };

            // Phase 3: Build FoulDetails for "Why?" button
            let aggression_norm = skills::normalize(ctx.aggression);
            let foul_details = build_foul_details(&result, ball_holder_idx, in_penalty_area, aggression_norm);

            // FIX_2601/0123 Phase 6: A/B comparison and DispatcherPrimary support
            // Run dispatcher for both tracking and primary modes
            let dispatcher_card: Option<Card> = if self.rule_check_mode.tracking_enabled() || self.rule_check_mode.dispatcher_applies() {
                // Create ContactEvent for dispatcher comparison
                let contact_event = ContactEvent {
                    tackler_idx,
                    ball_carrier_idx: ball_holder_idx,
                    position: foul_position.clone(),
                    contact_angle: 0.0, // Not tracked in current system
                    ball_won_first: false, // Foul implies ball was not won cleanly
                    intensity: aggression_norm,
                };

                // Determine foul type for comparison
                let rule_foul_type = if in_penalty_area {
                    RuleFoulType::Penalty
                } else if result.red_card {
                    RuleFoulType::SeriousFoulPlay
                } else {
                    RuleFoulType::Direct
                };

                // Determine card for comparison
                let card = if result.red_card {
                    Some(Card::Red)
                } else if result.yellow_card {
                    Some(Card::Yellow)
                } else {
                    None
                };

                let legacy_result = LegacyFoulResult {
                    is_foul: true,
                    offender_idx: Some(tackler_idx),
                    victim_idx: Some(ball_holder_idx),
                    foul_type: Some(rule_foul_type),
                    position: Some(foul_position.clone()),
                    card,
                };

                let last_touch_team = if let Some(idx) = self.ball.current_owner {
                    RuleTeamId::from_player_index(idx)
                } else {
                    RuleTeamId::Home
                };

                // Run dispatcher comparison - logs comparison and returns appropriate decision
                let decision = check_foul_wrapper(
                    self.rule_check_mode,
                    &mut self.rule_dispatcher,
                    &legacy_result,
                    &[contact_event],
                    &self.ball.position,
                    last_touch_team,
                    self.rng.gen(),
                );

                // Extract card from dispatcher decision if it exists
                if let Some(super::rules::RuleDecision::Foul { card, .. }) = decision {
                    card
                } else {
                    None
                }
            } else {
                None
            };

            // Determine effective cards to use (DispatcherPrimary uses dispatcher decision)
            let (effective_yellow, effective_red) = if self.rule_check_mode.dispatcher_applies() {
                match dispatcher_card {
                    Some(Card::Red) => (false, true),
                    Some(Card::Yellow) => (true, false),
                    Some(Card::SecondYellow) => (true, true), // Second yellow leads to red
                    None => (false, false),
                }
            } else {
                (result.yellow_card, result.red_card)
            };

            // Record foul event with FoulDetails
            // C6: Use tackler_idx directly as track_id
            self.emit_event(
                MatchEvent::foul(
                    self.minute,
                    self.current_timestamp_ms(),
                    is_home,
                    tackler_idx,
                    (foul_pos_norm.0, foul_pos_norm.1, self.ball.height as f32 / 10.0),
                )
                .with_target_track_id(Some(ball_holder_idx))
                .with_foul_details(foul_details),
            );

            // Card events (use effective values which may come from dispatcher in Primary mode)
            if effective_red {
                // C6: Use tackler_idx directly as track_id
                self.emit_event(
                    MatchEvent::red_card(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        tackler_idx,
                    )
                    .with_target_track_id(Some(ball_holder_idx)),
                );
                self.send_off_player(tackler_idx);
            } else if effective_yellow {
                // C6: Use tackler_idx directly as track_id
                self.emit_event(
                    MatchEvent::yellow_card(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        tackler_idx,
                    )
                    .with_target_track_id(Some(ball_holder_idx)),
                );
            }

            // Injury check
            if result.injury {
                self.check_injury_from_foul(ball_holder_idx, aggression_norm);
            }

            // Trigger free kick/penalty kick
            if in_penalty_area {
                self.start_penalty_kick_fsm(attacking_home);
            } else {
                self.start_free_kick_fsm(foul_pos_norm, attacking_home, false);
            }
        }
        // Tackle failed + no foul: ball possession maintained
    }

    /// P3: Injury check from foul
    pub(crate) fn check_injury_from_foul(&mut self, victim_idx: usize, aggression: f32) {
        // Skip if already injured
        if self.injured_players.contains(&victim_idx) {
            return;
        }

        // Injury chance based on: aggression (30%), fatigue (40%), random (30%)
        let fatigue = self.player_fatigue.get(victim_idx).copied().unwrap_or(0.0);
        let base_chance = 0.02; // 2% base injury chance
        let aggression_factor = aggression * 0.03; // Up to +3% from aggression
        let fatigue_factor = fatigue * 0.04; // Up to +4% from fatigue

        let injury_chance = base_chance + aggression_factor + fatigue_factor;

        if self.rng.gen::<f32>() < injury_chance {
            // Injury occurred!
            let is_home = TeamSide::is_home(victim_idx);

            // Random injury severity (1-4 weeks)
            let weeks_out = self.rng.gen_range(1..=4);

            // P2: auto timestamp via emit_event
            // C6: Use victim_idx directly as track_id
            self.emit_event(MatchEvent::injury(
                self.minute,
                self.current_timestamp_ms(),
                is_home,
                victim_idx,
                weeks_out,
            ));

            // Mark player as injured
            self.injured_players.push(victim_idx);

            // Injured player can't play - trigger substitution if available
            self.force_injury_substitution(victim_idx, is_home);
        }
    }
}
