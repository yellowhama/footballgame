//! Action Decision System
//!
//! This module contains action selection and decision logic for MatchEngine:
//! - Player action selection based on situation
//! - Shooting decision logic (Open Football style)
//! - Position-based skill rating
//!
//! Extracted from match_sim/mod.rs for better organization.

// P18: calculate_pressure_context deprecated 경고 억제 (FieldBoard 마이그레이션 예정)
#![allow(deprecated)]

use super::MatchEngine;
use super::RngCategory; // FIX_2601/0120: RNG tracking
use crate::engine::body_blocking::{self, TackleAttemptResult};
use crate::engine::physics_constants::{aerial, skills, zones};
use crate::engine::player_decision::{PlayerAction, PlayerDecision};
use crate::engine::weights::WeightBreakdown;
use crate::engine::{coordinates, physics_constants};
use crate::models::{Position, TeamSide};
use rand::Rng;

// Contract v1: OutcomeSet(상호배타) + 1회 softmax 샘플링
use super::decision_topology::select_outcome_softmax;

impl MatchEngine {
    /// Decide the next action for the player with the ball
    pub fn decide_action(&mut self, player_idx: usize) -> PlayerAction {
        let is_home = TeamSide::is_home(player_idx);
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let player = self.get_match_player(player_idx);

        // Extract position string
        let position_str = self.get_position_string(&player.position);

        // Check if in attacking third using standardized coordinates
        // Normalized: pos.0 = width (sideline), pos.1 = length (goal direction)
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601/0105: Use attacks_right for direction
        let in_attacking_third =
            coordinates::is_in_attacking_third(player_pos.to_normalized_legacy(), attacks_right);

        // NOTE: Legacy code path - better teammate check moved to decision_topology.rs (P16 Gate Chain)
        // FIX_2601/0105: This code path is NOT USED - P16 uses select_best_action_with_detail()
        // The actual shot decision now uses filter_shoot_if_better_teammate() in decision_topology.rs
        if self.can_attempt_shot(player_idx) {
            if self.should_shoot_over_pass(player_idx) {
                return PlayerAction::Shoot;
            }
            // Decided not to shoot - force pass instead
            return PlayerAction::Pass;
        }

        // A7: Header action handling (aerial situation)
        // aerial::AERIAL_DUEL_MIN_M (0.5m) or higher = aerial ball
        let ball_height_m = self.ball.height as f32 / 10.0;
        if ball_height_m > aerial::AERIAL_DUEL_MIN_M {
            // Ball must be below HEADER_MAX_M (2.9m) for header
            if ball_height_m <= aerial::HEADER_MAX_M {
                let player_pos = self.get_player_position_by_index(player_idx);
                let (ball_pos_x, ball_pos_y) = self.ball.position.to_meters();
                // FIX_2601: player_pos is now Coord10
                let player_pos_m = player_pos.to_meters();
                let dx = player_pos_m.0 - ball_pos_x;
                let dy = player_pos_m.1 - ball_pos_y;
                let dist_to_ball = (dx * dx + dy * dy).sqrt();

                // FIX_2601: threshold was 0.05 normalized = 5.25m, now use meters directly
                if dist_to_ball < 5.0 {
                    // Within 5m → attempt header
                    return PlayerAction::Header;
                }
            }
            // If ball is higher than HEADER_MAX_M, can't header - wait for ball to come down
        }

        // A8: Tackle action handling (ball possession contest)
        // P7 Body Blocking: Use can_attempt_tackle for realistic tackle decisions
        if let Some(ball_holder_idx) = self.ball.current_owner {
            let is_ball_holder = ball_holder_idx == player_idx;
            let is_same_team = TeamSide::same_team(player_idx, ball_holder_idx);

            if !is_ball_holder && !is_same_team {
                // Get positions in meters for body blocking calculations
                let player_pos = self.get_player_position_by_index(player_idx);
                let holder_pos = self.get_player_position_by_index(ball_holder_idx);
                let _ball_pos = self.ball.position;

                // FIX_2601: player_pos/holder_pos are now Coord10, use to_meters()
                let player_pos_m = player_pos.to_meters();
                let holder_pos_m = holder_pos.to_meters();
                let ball_pos_m = self.ball.position.to_meters();

                // Get all player positions in meters (FIX_2601: Coord10::to_meters())
                let all_positions_m: Vec<(f32, f32)> =
                    self.player_positions.iter().map(|p| p.to_meters()).collect();

                // Get player state and cooldown
                let player_state = self.player_states.get(player_idx).cloned().unwrap_or_default();
                let cooldown = self.tackle_cooldowns.get(player_idx).copied().unwrap_or(0);

                // Ball owner facing direction (approximate from velocity or default)
                let holder_facing = 0.0_f32; // TODO: track actual facing direction

                // Check if tackle attempt is valid using body blocking
                let tackle_result = body_blocking::can_attempt_tackle(
                    player_idx,
                    player_pos_m,
                    &player_state,
                    cooldown,
                    ball_pos_m,
                    ball_holder_idx,
                    holder_pos_m,
                    holder_facing,
                    &all_positions_m,
                );

                match tackle_result {
                    TackleAttemptResult::CanAttempt { .. } => {
                        // Can tackle - 80% chance to actually attempt
                        self.rng_tracker.record_for_player(player_idx, RngCategory::Decision);
                        if self.rng.gen::<f32>() < 0.8 {
                            return PlayerAction::Tackle;
                        }
                    }
                    TackleAttemptResult::BadAngle { .. } => {
                        // Bad angle - lower chance, higher foul risk
                        // 30% chance to attempt (risky tackle from behind)
                        self.rng_tracker.record_for_player(player_idx, RngCategory::Decision);
                        if self.rng.gen::<f32>() < 0.3 {
                            return PlayerAction::Tackle;
                        }
                    }
                    TackleAttemptResult::PathBlocked
                    | TackleAttemptResult::TooFar
                    | TackleAttemptResult::NotReady
                    | TackleAttemptResult::OnCooldown => {
                        // Cannot tackle - do nothing (will fall through to other actions)
                    }
                }
            }
        }

        // P1: Use player PlayerInstructions
        let instructions = self.get_player_instructions(player_idx);

        // Calculate action probabilities
        let probs = PlayerDecision::calculate_action_probabilities(
            &instructions,
            &position_str,
            in_attacking_third,
        );

        // Contract v1:
        // - 후보는 Weight space로만 표현
        // - 선택은 1회 softmax 샘플링 (상호배타)
        // - deterministic order 유지
        if probs.is_empty() {
            return PlayerAction::Pass;
        }

        let mut candidates: Vec<(PlayerAction, WeightBreakdown)> = probs
            .into_iter()
            .map(|(action, w)| {
                // probs의 값은 "확률"이 아니라 "가중치"로 취급한다.
                // ln(0) 방지: context에 최소값 부여
                let mut bd = WeightBreakdown::neutral();
                bd.context = w.max(0.0001);
                (action, bd)
            })
            .collect();

        candidates.sort_by_key(|(action, _)| format!("{:?}", action));

        // P0 Patch 4: Apply tempo from TeamInstructions to temperature
        // Fast tempo → higher temperature → more risky/exploratory decisions
        // Slow tempo → lower temperature → more conservative decisions
        let base_temp = 1.0;
        let instructions = if is_home { &self.home_instructions } else { &self.away_instructions };
        let adjusted_temp =
            super::decision_topology::apply_team_tempo_temperature(base_temp, instructions);

        // FIX_2601/0120: Track RNG usage for softmax sampling
        self.rng_tracker.record_for_player(player_idx, RngCategory::Decision);
        if let Some(choice) = select_outcome_softmax(&candidates, adjusted_temp, &mut self.rng) {
            return choice;
        }

        PlayerAction::Pass
    }

    /// Convert player position to string
    pub(crate) fn get_position_string(&self, position: &crate::models::Position) -> String {
        let pos_str = format!("{:?}", position);
        match pos_str.to_uppercase().as_str() {
            s if s.contains("GK") || s.contains("GOALKEEPER") => "GK".to_string(),
            s if s.contains("CB") || s.contains("CENTERBACK") => "CB".to_string(),
            s if s.contains("LB") || s.contains("LEFTBACK") => "LB".to_string(),
            s if s.contains("RB") || s.contains("RIGHTBACK") => "RB".to_string(),
            s if s.contains("CDM") || s.contains("DM") => "CDM".to_string(),
            s if s.contains("CM") => "CM".to_string(),
            s if s.contains("LM") => "LM".to_string(),
            s if s.contains("RM") => "RM".to_string(),
            s if s.contains("CAM") || s.contains("AM") => "CAM".to_string(),
            s if s.contains("LW") => "LW".to_string(),
            s if s.contains("RW") => "RW".to_string(),
            s if s.contains("ST") || s.contains("STRIKER") => "ST".to_string(),
            s if s.contains("CF") => "CF".to_string(),
            _ => "CM".to_string(),
        }
    }

    // NOTE: execute_player_action() removed (2026-01-05) - legacy code, never called
    // P16 Gate Chain + ActionQueue handles action execution now

    // ===========================================
    // A6: Position-based Skill Weighting
    // ===========================================

    /// Calculate position-specific skill rating
    pub(crate) fn get_positional_skill_rating(&self, player_idx: usize, action_type: &str) -> f32 {
        let player = match self.get_player(player_idx) {
            Some(p) => p,
            None => return 0.5, // Default
        };

        let position = &player.position;

        match action_type {
            "shooting" => {
                match position {
                    Position::ST | Position::CF | Position::FW => {
                        // Forwards: finishing is most important
                        let finishing = skills::normalize(self.get_player_finishing(player_idx));
                        let composure = skills::normalize(self.get_player_composure(player_idx));
                        finishing * 0.6 + composure * 0.4
                    }
                    Position::CAM | Position::LW | Position::RW => {
                        // Wingers/playmakers: long shots important
                        let finishing = skills::normalize(self.get_player_finishing(player_idx));
                        let long_shots = skills::normalize(self.get_player_long_shots(player_idx));
                        finishing * 0.5 + long_shots * 0.5
                    }
                    Position::CM | Position::CDM => {
                        // Midfielders: long shots focused
                        let long_shots = skills::normalize(self.get_player_long_shots(player_idx));
                        let technique = skills::normalize(self.get_player_technique(player_idx));
                        long_shots * 0.7 + technique * 0.3
                    }
                    _ => {
                        // Defenders/GK: basic ability
                        skills::normalize(self.get_player_finishing(player_idx))
                    }
                }
            }

            "passing" => {
                match position {
                    Position::CM | Position::CAM | Position::CDM => {
                        // Central midfielders: vision is important
                        let passing = skills::normalize(self.get_player_passing(player_idx));
                        let vision = skills::normalize(self.get_player_vision(player_idx));
                        let technique = skills::normalize(self.get_player_technique(player_idx));
                        passing * 0.4 + vision * 0.4 + technique * 0.2
                    }
                    Position::CB | Position::DF => {
                        // Center backs: long pass important
                        let passing = skills::normalize(self.get_player_passing(player_idx));
                        let vision = skills::normalize(self.get_player_vision(player_idx));
                        passing * 0.6 + vision * 0.4
                    }
                    _ => {
                        // Other positions: balanced
                        let passing = skills::normalize(self.get_player_passing(player_idx));
                        let vision = skills::normalize(self.get_player_vision(player_idx));
                        passing * 0.5 + vision * 0.5
                    }
                }
            }

            "defending" => {
                match position {
                    Position::CB | Position::DF => {
                        // Center backs: aerial + defensive positioning
                        let marking = skills::normalize(self.get_player_marking(player_idx));
                        let positioning =
                            skills::normalize(self.get_player_positioning(player_idx));
                        let heading = skills::normalize(self.get_player_heading(player_idx));
                        marking * 0.4 + positioning * 0.35 + heading * 0.25
                    }
                    Position::LB | Position::RB | Position::LWB | Position::RWB => {
                        // Fullbacks: pace + tackle
                        let tackling = skills::normalize(self.get_player_tackling(player_idx));
                        let pace = skills::normalize(self.get_player_pace(player_idx));
                        let positioning =
                            skills::normalize(self.get_player_positioning(player_idx));
                        tackling * 0.4 + pace * 0.3 + positioning * 0.3
                    }
                    Position::CDM => {
                        // Defensive midfielders: intercept + tackle
                        let tackling = skills::normalize(self.get_player_tackling(player_idx));
                        let anticipation =
                            skills::normalize(self.get_player_anticipation(player_idx));
                        let positioning =
                            skills::normalize(self.get_player_positioning(player_idx));
                        tackling * 0.35 + anticipation * 0.35 + positioning * 0.3
                    }
                    _ => {
                        // Other positions: basic tackling
                        skills::normalize(self.get_player_tackling(player_idx))
                    }
                }
            }

            _ => 0.5, // Unknown action type
        }
    }

    // ===========================================
    // Shooting Decision System (Open Football style)
    // ===========================================

    /// Check for nearby opponent
    pub(crate) fn has_nearby_opponent(&self, player_idx: usize, threshold_m: f32) -> bool {
        let player_pos = self.get_player_position_by_index(player_idx);
        let opponent_range = TeamSide::opponent_range(player_idx);

        for opponent_idx in opponent_range {
            let opponent_pos = self.get_player_position_by_index(opponent_idx);
            if player_pos.distance_to_m(&opponent_pos) < threshold_m {
                return true;
            }
        }

        false
    }

    /// FIX_2601/0105: Count nearby opponents within threshold distance
    /// Used for pressure-based shot decision (Open-Football style)
    pub(crate) fn count_nearby_opponents(&self, player_idx: usize, threshold_m: f32) -> usize {
        let player_pos = self.get_player_position_by_index(player_idx);
        let opponent_range = TeamSide::opponent_range(player_idx);

        opponent_range
            .filter(|&opponent_idx| {
                let opponent_pos = self.get_player_position_by_index(opponent_idx);
                player_pos.distance_to_m(&opponent_pos) < threshold_m
            })
            .count()
    }

    /// Determine excellent shooting opportunity (Open Football: has_excellent_shooting_opportunity)
    ///
    /// Relaxed conditions for realistic shot volume:
    /// - Very close (< 5m): Always excellent
    /// - Penalty box (5-16.5m): 2 of 3 conditions = excellent
    pub(crate) fn has_excellent_shooting_opportunity(&self, player_idx: usize) -> bool {
        let is_home = TeamSide::is_home(player_idx);
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
        let distance_m =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);

        use crate::engine::physics_constants::{action_thresholds, zones};

        // Very close (< 5m): Always excellent - just shoot!
        if distance_m < 5.0 {
            return true;
        }

        // Penalty box (5-16.5m): 2 of 3 conditions = excellent
        if distance_m < zones::CLOSE_M {
            let clear_shot = self.has_clear_shot(player_idx);
            let low_pressure =
                !self.has_nearby_opponent(player_idx, action_thresholds::NEARBY_OPPONENT_M);
            let good_angle = self.has_good_shooting_angle(player_idx);

            // 2 of 3 conditions = excellent opportunity
            let conditions_met = (clear_shot as u8) + (low_pressure as u8) + (good_angle as u8);
            return conditions_met >= 2;
        }

        false
    }

    /// Shooting vs pass decision - EV 기반 비교 (v9)
    ///
    /// 2025-12-12: 하드코딩된 확률 제거, 순수 EV Score 비교로 교체
    /// - Shot Score = xG × finishing × pressure_factor
    /// - Pass Score = best_pass_target의 threat value
    /// - shot_score × selfishness > pass_score → Shoot
    pub(crate) fn should_shoot_over_pass(&mut self, player_idx: usize) -> bool {
        let is_home = TeamSide::is_home(player_idx);
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
        let distance_m =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);

        // 1. Shot Score 계산
        let shot_score = self.calculate_shot_score(player_idx, distance_m);

        // 2. Pass Score 계산 (가장 좋은 패스 옵션)
        let pass_score = self.calculate_best_pass_score(player_idx);

        // 3. Selfishness (탐욕) - 결정력 기반
        // finishing 높은 선수는 슛을 더 선호
        let finishing = self.get_player_finishing(player_idx);
        let selfishness = 0.8 + (finishing / 100.0) * 0.4; // 0.8 ~ 1.2

        // 4. 최종 결정: Shot Score × Selfishness > Pass Score
        shot_score * selfishness > pass_score
    }

    /// 슛 점수 계산 (xG × 능력치 × 압박 보정)
    fn calculate_shot_score(&self, player_idx: usize, distance_m: f32) -> f32 {
        let _is_home = TeamSide::is_home(player_idx);
        let _player_pos = self.get_player_position_by_index(player_idx);

        // A. 기본 xG (거리 기반)
        // 0m = 0.85, 10m = 0.20, 20m = 0.04, 30m = 0.01
        let base_xg = (0.85 * (-0.12 * distance_m).exp()).clamp(0.01, 0.85);

        // B. 능력치 보정 (Finishing or Long Shots)
        let skill = if distance_m < 18.0 {
            self.get_player_finishing(player_idx)
        } else {
            self.get_player_long_shots(player_idx)
        };
        // 능력치 0~20 → 0.6 ~ 1.4 배율
        let skill_multiplier = 0.6 + (skill / 100.0) * 0.8;

        // C. 압박 보정 (수비수 있으면 점수 하락)
        let pressure_ctx = self.calculate_pressure_context(player_idx, None);
        let pressure_factor = 1.0 - (pressure_ctx.effective_pressure * 0.6);

        // D. 슛 각도 보정 (골대 정면 vs 측면)
        let angle_factor = if self.has_good_shooting_angle(player_idx) { 1.0 } else { 0.6 };

        // E. 클리어샷 보정
        let clear_factor = if self.has_clear_shot(player_idx) { 1.0 } else { 0.5 };

        base_xg * skill_multiplier * pressure_factor * angle_factor * clear_factor
    }

    /// 최고 패스 옵션의 점수 계산
    ///
    /// 패스 점수는 슛 점수와 비교 가능한 스케일이어야 함
    /// 슛: 즉시 득점 시도 (high risk, high reward)
    /// 패스: 공격권 유지 + 더 나은 위치로 이동 (lower risk, lower reward)
    fn calculate_best_pass_score(&self, player_idx: usize) -> f32 {
        let is_home = TeamSide::is_home(player_idx);
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
        let my_distance_to_goal =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);

        // 같은 팀 선수들 중 최고의 패스 타겟 찾기
        let teammates: Vec<usize> =
            TeamSide::teammate_range(player_idx).filter(|&i| i != player_idx).collect();

        let mut best_score = 0.0f32;

        for &target_idx in &teammates {
            let target_pos = self.get_player_position_by_index(target_idx);

            // 패스 성공 확률 추정 (미터 기준)
            let pass_distance_m = player_pos.distance_to_m(&target_pos);
            // 30m 이상 패스는 성공 확률 감소
            let pass_success = (1.0 - pass_distance_m / 60.0).clamp(0.4, 0.90);

            // 타겟 위치의 공격 위협도
            // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
            let target_distance_to_goal =
                coordinates::distance_to_goal_m(target_pos.to_normalized_legacy(), attacks_right);

            // 패스가 "더 나은 위치"로 가는지 확인
            // 상대 골대에 더 가까우면 가치 있음
            let position_improvement = if target_distance_to_goal < my_distance_to_goal {
                // 전진 패스: 개선 정도에 비례한 보너스
                let improvement =
                    (my_distance_to_goal - target_distance_to_goal) / my_distance_to_goal;
                improvement * 0.15 // 최대 0.15 보너스
            } else {
                // 후방/측면 패스: 공격권 유지만 (낮은 가치)
                0.02
            };

            // 패스 점수 = 성공확률 × 위치개선값
            // 슛과 비교 가능하도록 스케일 낮춤 (슛의 xG와 비슷한 범위)
            let score = pass_success * position_improvement;
            best_score = best_score.max(score);
        }

        // 패스의 최대 가치를 제한 (슛보다 낮아야 함)
        best_score.min(0.12)
    }

    // ==========================================================================
    // FIX_2601/0109: Unified Shot Decision System (Open-Football Style)
    // ==========================================================================

    /// Unified shot attempt decision - Open-Football style with stricter conditions
    ///
    /// FIX_2601/0105: Stricter shot conditions to reduce excessive shots
    /// Real EPL: ~13.6 shots/team/match, Our game was: ~29.5 (2x too many)
    ///
    /// Changes from original:
    /// - Zone 1 (< 16.5m): Requires 2 of 3 conditions (clear_shot, low_pressure, good_angle)
    /// - Zone 2 (16.5-25m): Requires clear_shot AND (skill OR no pressure)
    /// - Zone 3 (25-35m): Requires clear_shot AND high skill AND 5% random
    /// - Added pressure check: 2+ defenders within 8m blocks most shots
    pub(crate) fn can_attempt_shot(&mut self, player_idx: usize) -> bool {
        let is_home = TeamSide::is_home(player_idx);
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
        let distance_m =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);

        let long_shots = self.get_player_long_shots(player_idx);
        self.result.statistics.shot_gate_checks =
            self.result.statistics.shot_gate_checks.saturating_add(1);
        self.result.statistics.clear_shot_checks =
            self.result.statistics.clear_shot_checks.saturating_add(1);
        let has_clear = self.has_clear_shot(player_idx);
        if !has_clear {
            self.result.statistics.clear_shot_blocked =
                self.result.statistics.clear_shot_blocked.saturating_add(1);
        }
        let good_angle = self.has_good_shooting_angle(player_idx);

        // FIX_2601/0105: Count pressure (defenders within 8m)
        let pressure_count = self.count_nearby_opponents(player_idx, 8.0);

        // Zone 4: No Fly Zone (> 35m) - never shoot
        if distance_m > zones::LONG_RANGE_M {
            self.result.statistics.shot_gate_rejects =
                self.result.statistics.shot_gate_rejects.saturating_add(1);
            return false;
        }

        // FIX_2601/0105: Heavy pressure (2+ defenders) blocks shots except very close
        if pressure_count >= 2 && distance_m > 5.0 {
            self.result.statistics.shot_gate_rejects =
                self.result.statistics.shot_gate_rejects.saturating_add(1);
            return false;
        }

        // Zone 1: Box (< 16.5m) - need 2 of 3 conditions OR very close (< 5m)
        if distance_m < zones::CLOSE_M {
            // Very close (< 5m): always can shoot
            if distance_m < 5.0 {
                self.result.statistics.shot_gate_allowed =
                    self.result.statistics.shot_gate_allowed.saturating_add(1);
                return true;
            }

            // 5-16.5m: need 2 of 3 conditions (Open-Football style)
            let low_pressure = pressure_count == 0;
            let conditions_met = (has_clear as u8) + (low_pressure as u8) + (good_angle as u8);
            if conditions_met >= 2 {
                self.result.statistics.shot_gate_allowed =
                    self.result.statistics.shot_gate_allowed.saturating_add(1);
                return true;
            }
            self.result.statistics.shot_gate_rejects =
                self.result.statistics.shot_gate_rejects.saturating_add(1);
            return false;
        }

        // Zone 2: The Arc (16.5-25m) - need clear shot AND (skill OR no pressure)
        if distance_m < zones::MID_RANGE_M {
            if !has_clear {
                self.result.statistics.shot_gate_rejects =
                    self.result.statistics.shot_gate_rejects.saturating_add(1);
                return false;
            }
            // Need either high skill OR no pressure
            if long_shots >= 16.0 || pressure_count == 0 {
                self.result.statistics.shot_gate_allowed =
                    self.result.statistics.shot_gate_allowed.saturating_add(1);
                return true;
            }
            self.result.statistics.shot_gate_rejects =
                self.result.statistics.shot_gate_rejects.saturating_add(1);
            return false;
        }

        // Zone 3: Wonder Goal (25-35m) - specialists only, 5% chance, must have clear shot
        if has_clear && long_shots >= 18.0 && pressure_count == 0 {
            let allowed = self.rng.gen_bool(0.05);
            if allowed {
                self.result.statistics.shot_gate_allowed =
                    self.result.statistics.shot_gate_allowed.saturating_add(1);
            } else {
                self.result.statistics.shot_gate_rejects =
                    self.result.statistics.shot_gate_rejects.saturating_add(1);
            }
            return allowed;
        }

        self.result.statistics.shot_gate_rejects =
            self.result.statistics.shot_gate_rejects.saturating_add(1);
        false
    }

    /// Can shoot while dribbling (Open Football: can_shoot in dribbling state)
    ///
    /// Shot Volume Tuning (2025-12-07) v3:
    /// - Uses Advanced Pressure System with defender angle
    /// - Dribbling into shot requires space or beating defender angle
    pub(crate) fn can_shoot_while_dribbling(&mut self, player_idx: usize) -> bool {
        use super::calculations::PressureLevel;

        let is_home = TeamSide::is_home(player_idx);
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601/0110: Use attacks_right instead of is_home for correct goal direction
        let distance_m =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);

        let long_shots = self.get_player_long_shots(player_idx);
        let has_clear_shot = self.has_clear_shot(player_idx);

        // Advanced Pressure System v3: Use PressureContext
        let pressure_ctx = self.calculate_pressure_context(player_idx, None);
        let dribble_modifier = self.calculate_dribble_pressure_modifier(&pressure_ctx);

        // Heavy/Extreme pressure: almost never shoot while dribbling
        if pressure_ctx.pressure_level == PressureLevel::Heavy
            || pressure_ctx.pressure_level == PressureLevel::Extreme
        {
            // 정면 차단 시 슛 불가, 후방 추격 시 약간 가능
            if pressure_ctx.defender_angle > 0.5 {
                return false; // 정면 차단 = 슛 불가
            }
            // 후방 추격(angle < -0.3): 약간 가능
            if pressure_ctx.defender_angle < -0.3 && distance_m < zones::VERY_CLOSE_M {
                return self.rng.gen_bool(0.20);
            }
            return distance_m < zones::VERY_CLOSE_M && self.rng.gen_bool(0.10);
        }

        // Penalty box (< 16.5m)
        if distance_m < zones::CLOSE_M {
            if has_clear_shot {
                return self.rng.gen_bool((0.35 * dribble_modifier) as f64);
            }
            return distance_m < zones::VERY_CLOSE_M
                && self.rng.gen_bool((0.20 * dribble_modifier) as f64);
        }

        // Mid range (16.5-25m) - requires clear shot and space
        if distance_m < zones::MID_RANGE_M {
            if !has_clear_shot {
                return false;
            }
            // Need at least Light or None pressure for mid-range dribble shots
            if pressure_ctx.pressure_level != PressureLevel::None
                && pressure_ctx.pressure_level != PressureLevel::Light
            {
                return false;
            }
            if long_shots >= 15.0 {
                let prob = (0.08 + (long_shots - 15.0) / 140.0).clamp(0.0, 0.15);
                return self.rng.gen_bool(prob as f64);
            }
            return self.rng.gen_bool(0.05);
        }

        // Long shot (25-35m) - rare, need freedom
        if distance_m < zones::LONG_RANGE_M
            && long_shots >= 18.0
            && has_clear_shot
            && pressure_ctx.pressure_level == PressureLevel::None
        {
            return self.rng.gen_bool(0.03);
        }

        // Very long range (> 35m) - almost never shoot while dribbling
        if long_shots >= 19.0
            && has_clear_shot
            && pressure_ctx.pressure_level == PressureLevel::None
        {
            return self.rng.gen_bool(0.01);
        }

        false
    }

    /// Clear shot determination (Open Football: has_clear_shot with ray-cast)
    ///
    /// FIX_2601/0109: Improved ray-cast with perpendicular distance calculation
    /// Uses RADIUS_TIGHT_M (1.5m) instead of BLOCKING_M (5m) for body blocking
    pub(crate) fn has_clear_shot(&self, player_idx: usize) -> bool {
        let is_home = TeamSide::is_home(player_idx);
        // FIX_2601/0116: Use attacks_right for goal direction (not is_home)
        let attacks_right = self.attacks_right(is_home);
        let player_pos_coord = self.get_player_position_by_index(player_idx);
        // FIX_2601: Convert to normalized for vector calculations
        let player_pos = player_pos_coord.to_normalized_legacy();

        // Check for blocking opponents in goal direction
        // Normalized: pos.0 = width (sideline), pos.1 = length (goal direction)
        // attacks_right=true → pos.1 = 1.0, attacks_right=false → pos.1 = 0.0
        let goal_length = coordinates::attacking_goal_length(attacks_right);
        let goal_pos = (0.5, goal_length); // Center width, attacking goal length
        let opponent_range = TeamSide::opponent_range(player_idx);

        // Ray direction from player to goal (normalized coordinates)
        let to_goal = (goal_pos.0 - player_pos.0, goal_pos.1 - player_pos.1);
        let ray_length_sq = to_goal.0 * to_goal.0 + to_goal.1 * to_goal.1;

        // Skip if at goal (shouldn't happen)
        if ray_length_sq < 0.0001 {
            return true;
        }

        let _ray_length = ray_length_sq.sqrt(); // Reserved for future shot arc calculations

        for opponent_idx in opponent_range {
            let opponent_pos_coord = self.get_player_position_by_index(opponent_idx);
            let opponent_pos = opponent_pos_coord.to_normalized_legacy();

            // Vector from player to opponent
            let to_opponent = (opponent_pos.0 - player_pos.0, opponent_pos.1 - player_pos.1);

            // Project opponent onto shooting ray: proj = (to_opponent · to_goal) / |to_goal|²
            let dot_product = to_goal.0 * to_opponent.0 + to_goal.1 * to_opponent.1;
            let projection = dot_product / ray_length_sq;

            // Only check opponents between shooter and goal (0 < projection < 1)
            if projection <= 0.0 || projection >= 1.0 {
                continue;
            }

            // Calculate perpendicular distance to ray
            // projected_point = player_pos + to_goal * projection
            let projected_x = player_pos.0 + to_goal.0 * projection;
            let projected_y = player_pos.1 + to_goal.1 * projection;

            // Perpendicular distance (in normalized coordinates)
            let perp_dx = opponent_pos.0 - projected_x;
            let perp_dy = opponent_pos.1 - projected_y;
            let perp_dist_norm = (perp_dx * perp_dx + perp_dy * perp_dy).sqrt();

            // Convert to meters: normalized 1.0 ≈ 105m length, 68m width
            // Use average: (105 + 68) / 2 ≈ 86.5m per 1.0 normalized
            let perp_dist_m = perp_dist_norm * 86.5;

            // Body blocking radius: 1.5m (player body width + margin)
            if perp_dist_m < physics_constants::pressure::RADIUS_TIGHT_M {
                return false;
            }
        }

        true
    }

    /// Good shooting angle determination (Open Football: has_good_shooting_angle)
    pub(crate) fn has_good_shooting_angle(&self, player_idx: usize) -> bool {
        let player_pos = self.get_player_position_by_index(player_idx);

        // Angle from center (0.5 = center width)
        // Normalized: pos.0 = width (sideline), pos.1 = length (goal direction)
        let angle_deviation =
            (coordinates::norm_width(player_pos.to_normalized_legacy()) - 0.5).abs();

        // Within 30 degrees (approximately 0.25 normalized width)
        angle_deviation < 0.25
    }

    // NOTE: find_better_positioned_teammate moved to ev_decision.rs as part of FIX_2601/0105
    // The new implementation returns (bool, f32) for use with DecisionContext
}
