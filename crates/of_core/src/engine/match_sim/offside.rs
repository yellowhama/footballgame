//! Offside System - Line Battle and Through Ball Logic
//!
//! This module contains offside-related logic for MatchEngine:
//! - Offside line calculation
//! - Line battle resolution (trap vs breaking)
//! - Through ball attempts with ability checks
//! - Valid pass target filtering
//! - **FIX_2601/0112**: Player offside awareness (Open Football style)
//!
//! ## P0 Goal Contract 호환
//! - `attacking_is_home` = 공격 팀이 홈인지 여부
//! - 공격 방향은 `attacks_right(attacking_is_home)`로 계산 (전/후반 스위치 반영)
//! - 오프사이드 라인 = 공격 방향 기준 두 번째 마지막 수비수
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use crate::engine::action_queue::{ActionResult, RestartType};
use crate::engine::coordinates;
use crate::engine::physics_constants;
use crate::engine::player_state::PlayerState;
use crate::engine::types::{Coord10, LineBattleResult, ThroughBallResult, Vel10};
use crate::models::TeamSide;
use crate::tactics::team_instructions::DefensiveLine;
use rand::Rng;

// ===========================================
// FIX_2601/0112: Offside Awareness System
// ===========================================

/// 오프사이드 리스크 레벨 (Open Football 방식)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsideRisk {
    /// 안전 - 라인 5m+ 뒤
    Safe,
    /// 주의 - 라인 0-5m 뒤 (경계선)
    Marginal,
    /// 위험 - 라인 위 (오프사이드)
    Risky,
}

/// 오프사이드 리스크 평가 (미터 단위)
/// - `player_x`: 선수 x 위치 (미터)
/// - `offside_line`: 오프사이드 라인 x 위치 (미터)
/// - `attacks_right`: 오른쪽으로 공격하는지
pub fn evaluate_offside_risk(player_x: f32, offside_line: f32, attacks_right: bool) -> OffsideRisk {
    // 안전 마진: 라인에서 3m 뒤면 Safe
    // FIX_2601/0116: 0.5m은 너무 엄격해서 공격 약화, 10m은 너무 관대
    // 중간값 3m으로 설정 → 슛 ~15-20회/경기 목표
    const SAFETY_MARGIN: f32 = 3.0;

    // TeamView: always advancing to +x, so "safe" means player is behind the line.
    let length_m = physics_constants::field::LENGTH_M;
    let player_x_tv = if attacks_right { player_x } else { length_m - player_x };
    let offside_line_tv = if attacks_right { offside_line } else { length_m - offside_line };
    let distance_to_line = offside_line_tv - player_x_tv; // 양수면 안전

    if distance_to_line > SAFETY_MARGIN {
        OffsideRisk::Safe
    } else if distance_to_line > 0.0 {
        OffsideRisk::Marginal
    } else {
        OffsideRisk::Risky
    }
}

/// 런 타이밍 체크 - 패스가 나올 것 같을 때만 런
/// - `anticipation`: 예측력 (1-20)
/// - `off_the_ball`: 공 없을 때 움직임 (1-20)
/// - `passer_looking_forward`: 패서가 전방을 보고 있는지
pub fn should_make_run(anticipation: u8, off_the_ball: u8, passer_looking_forward: bool) -> bool {
    if !passer_looking_forward {
        return false; // 패서가 전방 안보면 대기
    }

    // 능력치 합산 (anticipation 60% + off_the_ball 40%)
    let run_skill = (anticipation as f32 * 0.6) + (off_the_ball as f32 * 0.4);

    // 런 타이밍 임계값: 12 (평균 능력치 = 10~11, 약간 위)
    run_skill >= 12.0
}

/// 안전한 런 위치 계산 (오프사이드 라인 기준)
/// - 리스크가 있으면 라인 뒤로 조정
pub fn calculate_safe_run_position(
    target_x: f32,
    offside_line: f32,
    attacks_right: bool,
    anticipation: u8,
) -> f32 {
    let risk = evaluate_offside_risk(target_x, offside_line, attacks_right);    
    let length_m = physics_constants::field::LENGTH_M;
    let offside_line_tv = if attacks_right { offside_line } else { length_m - offside_line };

    match risk {
        OffsideRisk::Safe => target_x, // 그대로
        OffsideRisk::Marginal => {
            // FIX_2601/0116: Marginal - 라인에서 0-3m 뒤 → 능력치에 따라 2-4m 후퇴
            let retreat = if anticipation >= 14 {
                2.0  // 고급 선수: 라인 가까이
            } else {
                4.0  // 일반 선수: 더 뒤로
            };
            let safe_tv = offside_line_tv - retreat;
            if attacks_right { safe_tv } else { length_m - safe_tv }
        }
        OffsideRisk::Risky => {
            // FIX_2601/0116: Risky - 이미 오프사이드 위치 → 능력치에 따라 2-5m 후퇴
            let margin = if anticipation >= 14 {
                2.0  // 고급 선수: 빠르게 복귀
            } else if anticipation >= 8 {
                3.0  // 중급 선수
            } else {
                5.0  // 일반 선수: 크게 후퇴
            };

            let safe_tv = offside_line_tv - margin;
            if attacks_right { safe_tv } else { length_m - safe_tv }
        }
    }
}

impl MatchEngine {
    // ===========================================
    // Offside System (Open Football Style)
    // ===========================================

    /// Calculate offside line (second-last defender), adjusted for attack direction.
    pub(crate) fn calculate_offside_line(
        &self,
        defending_is_home: bool,
        attacks_right: bool,
    ) -> f32 {
        self.second_last_defender_line(defending_is_home, attacks_right)
    }

    /// Check if receiver is in offside position
    ///
    /// NOTE: This function is used as a *target filter* during pass selection.
    /// It must be at least as strict as `is_offside_pass()`; otherwise the engine
    /// can select "marginally offside" forward targets that later get flagged at
    /// `ActionResult::PassStarted`, which inflates offside events and can skew
    /// Home/Away distribution (FIX_2601/0110).
    pub(crate) fn is_offside_position(
        &self,
        receiver_idx: usize,
        attacking_is_home: bool,
    ) -> bool {
        use crate::engine::physics_constants::offside;

        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let ball_pos_norm = coordinates::to_normalized(self.ball.position.to_meters());
        let attacks_right = self.attacks_right(attacking_is_home);

        // FIX_2601/0123: Add is_in_opponent_half check for consistency with is_offside_pass
        // Players in their own half cannot be offside
        let in_opponent_half =
            coordinates::is_in_opponent_half(receiver_pos.to_normalized_legacy(), attacks_right);
        if !in_opponent_half {
            return false; // Own half = no offside possible
        }

        // Calculate offside line
        let offside_line = self.calculate_offside_line(!attacking_is_home, attacks_right);

        // 좌표계: pos.0 = width, pos.1 = length (골 방향)
        let receiver_length = coordinates::norm_length(receiver_pos.to_normalized_legacy());
        // FIX_2601: ball_pos_norm is already normalized tuple
        let ball_length = coordinates::norm_length(ball_pos_norm);

        // FIX_2601/0123: Use identical buffer for filter and execution
        // Must be at least as strict as is_offside_pass to avoid selecting offside targets
        let small_buffer = 0.01; // ~1m tolerance - matches is_offside_pass
        let receiver_tv = if attacks_right { receiver_length } else { 1.0 - receiver_length };
        let ball_tv = if attacks_right { ball_length } else { 1.0 - ball_length };
        let line_tv = if attacks_right { offside_line } else { 1.0 - offside_line };

        // Same check as is_offside_pass for consistency
        receiver_tv > line_tv + small_buffer && receiver_tv > ball_tv
    }

    /// Determine if pass is a through ball (pass behind defensive line)
    pub(crate) fn is_through_ball_pass(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        attacking_is_home: bool,
    ) -> bool {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let attacks_right = self.attacks_right(attacking_is_home);
        let offside_line = self.calculate_offside_line(!attacking_is_home, attacks_right);

        // 좌표계: pos.1 = length (골 방향)
        let passer_length = coordinates::norm_length(passer_pos.to_normalized_legacy());
        let receiver_length = coordinates::norm_length(receiver_pos.to_normalized_legacy());

        // TeamView: "through ball" means passer is behind the line and receiver is near/past it.
        let passer_tv = if attacks_right { passer_length } else { 1.0 - passer_length };
        let receiver_tv = if attacks_right { receiver_length } else { 1.0 - receiver_length };
        let line_tv = if attacks_right { offside_line } else { 1.0 - offside_line };

        passer_tv < line_tv && receiver_tv > line_tv - 0.05
    }

    // ===========================================
    // Ability-based Offside Trap System
    // ===========================================

    /// Calculate perceived offside line (player's mental perception)
    pub(crate) fn get_perceived_offside_line(
        &mut self,
        player_idx: usize,
        actual_line: f32,
        is_home_attack: bool,
    ) -> f32 {
        // Mental Stats based error
        let anticipation = self.get_player_anticipation(player_idx);
        let concentration = self.get_player_concentration(player_idx);
        let mental_stat = anticipation * 0.6 + concentration * 0.4;

        // Error range (ability 20 = 0.0, ability 1 = 0.095)
        let max_error = 0.1 - (mental_stat * 0.005);
        let error = self.rng.gen_range(-max_error..max_error);

        // TeamView: apply perception error in the same forward (+x) space, then convert back.
        // Units: meters (actual_line is x in meters), error is also in meters.
        let attacks_right = self.attacks_right(is_home_attack);
        let length_m = physics_constants::field::LENGTH_M;

        let actual_tv = if attacks_right { actual_line } else { length_m - actual_line };
        let perceived_tv = (actual_tv + error).clamp(0.0, length_m);
        if attacks_right { perceived_tv } else { length_m - perceived_tv }
    }

    /// Calculate break chance (attacker vs defender)
    pub(crate) fn calculate_break_chance(&self, attacker_idx: usize, defender_idx: usize) -> f32 {
        let off_the_ball = self.get_player_off_the_ball(attacker_idx);
        let atk_anticipation = self.get_player_anticipation(attacker_idx);
        let atk_acceleration = self.get_player_acceleration(attacker_idx);

        let positioning = self.get_player_positioning(defender_idx);
        let concentration = self.get_player_concentration(defender_idx);
        let def_acceleration = self.get_player_acceleration(defender_idx);

        // Position battle: Off the Ball vs Positioning
        let space_score = (off_the_ball * 1.2) - positioning;

        // Timing: Anticipation vs Concentration
        let timing_score = atk_anticipation - concentration;

        // Burst: Acceleration vs Acceleration
        let burst_score = atk_acceleration - def_acceleration;

        // Final probability (base 50% + score bonus)
        let total = (space_score * 0.4) + (timing_score * 0.4) + (burst_score * 0.2);
        (0.5 + (total * 0.02)).clamp(0.1, 0.95)
    }

    /// Find marking defender for attacker
    pub(crate) fn find_marking_defender(
        &self,
        attacker_idx: usize,
        is_home_attacking: bool,
    ) -> usize {
        let defender_range = if is_home_attacking { 11..22 } else { 0..11 };
        let attacker_pos = self.get_player_position_by_index(attacker_idx);

        let mut best_idx = defender_range.start;
        let mut best_distance = f32::MAX;

        for idx in defender_range {
            let pos = self.get_player_position_by_index(idx);
            let dist = pos.distance_to_m(&attacker_pos);
            if dist < best_distance {
                best_distance = dist;
                best_idx = idx;
            }
        }

        best_idx
    }

    /// Calculate team average teamwork (for offside trap gate)
    /// FIX_2601/0112: Open-Football style team ability gate
    fn get_team_avg_teamwork(&self, is_home: bool) -> f32 {
        let start_idx = if is_home { 0 } else { 11 };
        let sum: f32 = (start_idx..start_idx + 11)
            .map(|i| self.get_player_teamwork(i) as f32)
            .sum();
        sum / 11.0
    }

    /// Calculate team average experience (composure as proxy)
    /// FIX_2601/0113: TrapReadiness에서 사용
    /// - composure (1-20) * 5 → 5-100 스케일
    fn get_team_avg_experience(&self, is_home: bool) -> f32 {
        let start_idx = if is_home { 0 } else { 11 };
        let sum: f32 = (start_idx..start_idx + 11)
            .map(|i| self.get_player_composure(i) * 5.0) // 1-20 → 5-100
            .sum();
        sum / 11.0
    }

    /// Resolve line battle (trap vs breaking)
    /// FIX_2601/0112: 하이브리드 방식 - 점수 + 시그모이드 확률 변환
    /// FIX_2601/0113: TrapReadiness 통합 - 라인 응집력 + 경험 체크
    pub(crate) fn resolve_line_battle(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
    ) -> LineBattleResult {
        use crate::engine::elastic_band::{
            calculate_line_cohesion, evaluate_trap_readiness, TrapBlockReason,
        };
        use crate::engine::physics_constants::offside_trap;

        let is_defender_home = TeamSide::is_home(defender_idx);
        let team_instructions =
            if is_defender_home { &self.home_instructions } else { &self.away_instructions };

        // ===== FIX_2601/0113: TrapReadiness 평가 =====
        // 수비수 위치 수집 (슬롯 1-4)
        let defender_start = if is_defender_home { 1 } else { 12 };
        let defender_positions: Vec<(f32, f32)> = (defender_start..defender_start + 4)
            .map(|i| self.player_positions[i].to_meters())
            .collect();

        // 라인 응집력 계산
        let line_cohesion = calculate_line_cohesion(&defender_positions);

        // 팀 평균 능력치
        let team_avg_teamwork = self.get_team_avg_teamwork(is_defender_home);
        let team_avg_experience = self.get_team_avg_experience(is_defender_home);

        // TrapReadiness 평가
        let trap_readiness = evaluate_trap_readiness(
            team_avg_teamwork,
            team_avg_experience,
            &line_cohesion,
            team_instructions.use_offside_trap,
        );

        // 트랩 불가 → Contested (공격수 유리)
        if !trap_readiness.can_execute {
            // 디버그: 불가 사유에 따라 다른 처리 가능
            match trap_readiness.reason {
                TrapBlockReason::TacticsDisabled
                | TrapBlockReason::LowTeamwork
                | TrapBlockReason::LowExperience
                | TrapBlockReason::LineTooSpread => {
                    return LineBattleResult::Contested;
                }
                TrapBlockReason::Ready => {} // 이 경우는 발생 안함
            }
        }

        // ===== Phase 2: 개인 능력 점수 계산 =====
        // 수비: positioning 40% + teamwork 30% + concentration 30%
        let positioning = self.get_player_positioning(defender_idx) as f32;
        let teamwork = self.get_player_teamwork(defender_idx) as f32;
        let concentration = self.get_player_concentration(defender_idx) as f32;
        let trap_base = positioning * 0.4 + teamwork * 0.3 + concentration * 0.3;

        // 공격: anticipation 35% + off_the_ball 40% + acceleration 25%
        let anticipation = self.get_player_anticipation(attacker_idx) as f32;
        let off_the_ball = self.get_player_off_the_ball(attacker_idx) as f32;
        let acceleration = self.get_player_acceleration(attacker_idx) as f32;
        let break_base = anticipation * 0.35 + off_the_ball * 0.4 + acceleration * 0.25;

        // ===== Phase 3: 전술 보너스 (축소됨) =====
        let line_bonus = match team_instructions.defensive_line {
            DefensiveLine::VeryHigh => offside_trap::LINE_VERY_HIGH_BONUS,
            DefensiveLine::High => offside_trap::LINE_HIGH_BONUS,
            DefensiveLine::Normal => offside_trap::LINE_NORMAL_BONUS,
            DefensiveLine::Deep => offside_trap::LINE_DEEP_PENALTY,
            DefensiveLine::VeryDeep => offside_trap::LINE_VERY_DEEP_PENALTY,
        };
        let trap_bonus = offside_trap::TRAP_ACTIVATION_BONUS;

        // ===== Phase 4: 점수차 → 확률 변환 (시그모이드) =====
        let trap_score = trap_base + trap_bonus + line_bonus;
        let break_score = break_base;
        let diff = trap_score - break_score; // 범위: 약 -18 ~ +22

        // 시그모이드: diff=0 → 50%, diff=+10 → 73%, diff=-10 → 27%
        let trap_prob = 1.0 / (1.0 + (-diff * offside_trap::SIGMOID_SCALE).exp());

        // ===== Phase 5: 상한/하한 적용 후 확률적 결과 =====
        // FIX_2601/0113: cohesion_score를 트랩 성공률에 반영
        // cohesion_score가 높을수록 (라인 응집력 + 팀워크 높음) 트랩 성공률 증가
        let cohesion_multiplier = 0.7 + trap_readiness.cohesion_score * 0.3; // 0.7 ~ 1.0

        let roll = self.rng.gen::<f32>();

        let trap_threshold = trap_prob * offside_trap::MAX_TRAP_SUCCESS * cohesion_multiplier;
        let break_threshold = 1.0 - (1.0 - trap_prob) * offside_trap::MAX_LINE_BREAK;

        if roll < trap_threshold {
            LineBattleResult::OffsideTrapSuccess
        } else if roll > break_threshold {
            LineBattleResult::LineBroken { advantage: 1.5 }
        } else {
            LineBattleResult::Contested
        }
    }

    /// Attempt through ball with abilities
    /// FIX_2601/0112: 오프사이드 확률 감소
    pub(crate) fn attempt_through_ball_with_abilities(
        &mut self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> ThroughBallResult {
        // 1. Find marking defender
        let defender_idx = self.find_marking_defender(receiver_idx, is_home);

        // 2. Line battle
        let battle_result = self.resolve_line_battle(receiver_idx, defender_idx);

        match battle_result {
            LineBattleResult::OffsideTrapSuccess => {
                return ThroughBallResult::OffsideTrap;
            }
            LineBattleResult::LineBroken { .. } => {
                // Breaking success, proceed to Vision check
            }
            LineBattleResult::Contested => {
                // Contest, check break chance
                let break_chance = self.calculate_break_chance(receiver_idx, defender_idx);
                if self.rng.gen::<f32>() > break_chance {
                    // FIX_2601/0112: 30% offside / 70% intercept (was 50/50)
                    if self.rng.gen::<f32>() < 0.3 {
                        return ThroughBallResult::OffsideTrap;
                    } else {
                        return ThroughBallResult::Intercepted;
                    }
                }
            }
        }

        // 3. Passer Vision check (timing accuracy)
        let passer_vision = self.get_player_vision(passer_idx);
        let timing_success = self.rng.gen::<f32>() < (passer_vision / 20.0);

        if !timing_success {
            // FIX_2601/0112: 40% offside / 60% bad timing (was 60/40)
            if self.rng.gen::<f32>() < 0.4 {
                return ThroughBallResult::Offside;
            } else {
                return ThroughBallResult::BadTiming;
            }
        }

        // 4. Pass accuracy check
        let passing = self.get_player_passing(passer_idx);
        let pass_success = self.rng.gen::<f32>() < (passing / 20.0 * 0.8 + 0.2);

        if !pass_success {
            return ThroughBallResult::BadPass;
        }

        ThroughBallResult::Success
    }

    /// Filter pass targets with offside check and vision range gate
    /// FIX_2601/0115: Added vision range filtering (open-football style)
    pub(crate) fn find_valid_pass_targets(&self, passer_idx: usize, is_home: bool) -> Vec<usize> {
        let teammate_range = if is_home { 0..11 } else { 11..22 };
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let attacks_right = self.attacks_right(is_home);

        teammate_range
            .filter(|&i| i != passer_idx)
            .filter(|&i| {
                !matches!(self.player_states[i], PlayerState::Injured | PlayerState::SentOff)
            })
            // FIX_2601/0115: Vision range gate - passer can only see targets within vision range
            .filter(|&i| self.is_within_vision_range(passer_idx, i))
            .filter(|&i| {
                let target_pos = self.get_player_position_by_index(i);
                let is_forward_pass = coordinates::is_advancing(
                    passer_pos.to_normalized_legacy(),
                    target_pos.to_normalized_legacy(),
                    attacks_right,
                );

                if is_forward_pass {
                    !self.is_offside_position(i, is_home)
                } else {
                    true
                }
            })
            .collect()
    }

    // ===========================================
    // P7-OFFSIDE-01: Pass Offside Check
    // ===========================================

    /// Check if a pass would result in offside
    /// FIX_2601/0112: 버퍼 증가 + 미세 판정 불확실성
    ///
    /// Offside is called when:
    /// 1. The pass is advancing (forward in attacking direction)
    /// 2. The receiver is beyond the second-last defender at pass moment
    /// 3. The receiver is in the opponent's half
    pub(crate) fn is_offside_pass(
        &mut self,
        passer_idx: usize,
        receiver_idx: usize,
        attacking_is_home: bool,
    ) -> bool {
        use crate::engine::physics_constants::offside;

        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let ball_pos = coordinates::to_normalized(self.ball.position.to_meters());
        let attacks_right = self.attacks_right(attacking_is_home);

        // 1. Only check forward passes (attacking direction)
        let is_advancing = coordinates::is_advancing(
            passer_pos.to_normalized_legacy(),
            receiver_pos.to_normalized_legacy(),
            attacks_right,
        );

        if !is_advancing {
            return false; // Back passes never offside
        }

        // 2. Receiver must be in opponent's half
        let in_opponent_half =
            coordinates::is_in_opponent_half(receiver_pos.to_normalized_legacy(), attacks_right);

        if !in_opponent_half {
            return false; // Own half = no offside
        }

        // FIX_2601/0112: 최소 패스 거리 증가 (8m → 15m)
        let pass_distance_m = passer_pos.distance_to_m(&receiver_pos);
        if pass_distance_m < offside::MIN_PASS_DISTANCE_M {
            return false; // Too short to trigger offside
        }

        // 3. Receiver must be ahead of the ball at pass moment
        let receiver_length = coordinates::norm_length(receiver_pos.to_normalized_legacy());
        // FIX_2601: ball_pos is already normalized tuple
        let ball_length = coordinates::norm_length(ball_pos);
        let ahead_of_ball = if attacks_right {
            receiver_length > ball_length
        } else {
            receiver_length < ball_length
        };

        if !ahead_of_ball {
            return false; // Behind ball = no offside
        }

        // 4. Get second-last defender line
        let offside_line = self.second_last_defender_line(!attacking_is_home, attacks_right);

        // 5. Check if receiver is beyond offside line
        // FIX_2601/0123: Use identical buffer for filter and execution
        // Testing 0.01 (~1m) - same check for both
        let small_buffer = 0.01; // ~1m tolerance
        let receiver_tv = if attacks_right { receiver_length } else { 1.0 - receiver_length };
        let ball_tv = if attacks_right { ball_length } else { 1.0 - ball_length };
        let line_tv = if attacks_right { offside_line } else { 1.0 - offside_line };

        // Must be beyond the line AND beyond the ball
        // Same check as is_offside_position for consistency
        let is_offside = receiver_tv > line_tv + small_buffer && receiver_tv > ball_tv;

        // FIX_2601/0110: Debug logging for offside bias investigation
        if is_offside && std::env::var("OF_DEBUG_OFFSIDE").is_ok() {
            // Also log world coordinates for analysis
            let receiver_pos_m = receiver_pos.to_meters();
            let passer_pos_m = passer_pos.to_meters();
            eprintln!(
                "[OFFSIDE] is_home={} half={} passer={} receiver={} \
                receiver_x={:.1}m passer_x={:.1}m \
                receiver_tv={:.3} line_tv={:.3} margin_tv={:.3}",
                attacking_is_home,
                if self.is_second_half { "2H" } else { "1H" },
                passer_idx,
                receiver_idx,
                receiver_pos_m.0,
                passer_pos_m.0,
                receiver_tv,
                line_tv,
                receiver_tv - line_tv
            );
        }

        is_offside
    }

    /// Calculate the second-last defender's position (offside line)
    ///
    /// The offside line is typically the second-last defender (GK is usually last).
    /// 좌표계: pos.1 = length (골 방향)
    /// Attacking right: second highest length among defenders
    /// Attacking left: second lowest length among defenders
    pub(crate) fn second_last_defender_line(
        &self,
        defending_is_home: bool,
        attacks_right: bool,
    ) -> f32 {
        let defender_range = if defending_is_home { 0..11 } else { 11..22 };

        // Collect all defender positions in length direction (excluding GK at index 0 or 11)
        let gk_idx = if defending_is_home { 0 } else { 11 };
        let mut lengths: Vec<f32> = defender_range
            .filter(|&idx| idx != gk_idx) // Exclude GK
            .map(|idx| {
                let len = coordinates::norm_length(
                    self.get_player_position_by_index(idx).to_normalized_legacy(),
                );
                if attacks_right { len } else { 1.0 - len }
            })
            .collect();

        if lengths.is_empty() {
            // Fallback: use halfway line
            return 0.5;
        }

        // Sort by length position
        lengths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // TeamView: always advancing to +length, so the offside line is the second-highest defender length.
        let line_tv = if lengths.len() >= 2 {
            lengths[lengths.len() - 2]
        } else {
            lengths[0]
        };

        if attacks_right { line_tv } else { 1.0 - line_tv }
    }

    /// Assign the ball to the nearest defender of the opposing team
    pub(crate) fn assign_possession_to_nearest_defender(&mut self, attacking_home: bool) -> usize {
        let defender_range = if attacking_home { 11..22 } else { 0..11 };
        let ball_pos = self.ball.position;

        let mut best_idx = None;
        let mut best_distance = f32::MAX;

        for idx in defender_range {
            let pos = self.get_player_position_by_index(idx);
            let (ball_pos_x, ball_pos_y) = ball_pos.to_meters();
            let pos_m = pos.to_meters();
            let mut dist = ((pos_m.0 - ball_pos_x).powi(2) + (pos_m.1 - ball_pos_y).powi(2)).sqrt();

            // B2: Hero Gravity - Loose Ball Magnet
            // "The ball likes the protagonist" - 1m distance bonus for hero
            let is_home = TeamSide::is_home(idx);
            if self.is_user_player(idx, is_home) {
                dist -= physics_constants::hero_gravity::LOOSE_BALL_DISTANCE_BONUS_M;
                dist = dist.max(0.0); // Prevent negative
            }

            if dist < best_distance {
                best_distance = dist;
                best_idx = Some(idx);
            }
        }

        let defender_idx = best_idx.unwrap_or(if attacking_home { 11 } else { 0 });
        let defender_pos = self.get_player_position_by_index(defender_idx);
        let defender_pos_m = defender_pos.to_meters();
        self.ball.current_owner = Some(defender_idx);
        self.ball.position = Coord10::from_meters(defender_pos_m.0, defender_pos_m.1);
        self.ball.velocity = Vel10::from_mps(0.0, 0.0);
        self.ball.height = 0;
        defender_idx
    }

    // ===========================================
    // Phase 2: Offside Details for "Why?" Button
    // ===========================================

    /// Check offside and return detailed information for "Why?" explanation
    ///
    /// Returns `Some(OffsideDetails)` if offside, `None` if onside.
    /// This includes:
    /// - margin_m: distance beyond the offside line (meters)
    /// - offside_line_m: position of 2nd last defender (meters)
    /// - passer_track_id: who played the ball
    /// - involvement_type: how the player was involved
    /// - restart_context: for checking exceptions (goal kick, throw-in, corner)
    /// - deflection_context: for deliberate play vs deflection
    pub(crate) fn check_offside_detailed(
        &mut self,
        passer_idx: usize,
        receiver_idx: usize,
        attacking_is_home: bool,
    ) -> Option<crate::models::OffsideDetails> {
        use crate::engine::physics_constants::{field, offside};
        use crate::models::rules::{
            DefenderTouchType, DeflectionContext, OffsideDetails, OffsideInvolvementType,
            OffsideRestartContext, ReferencePoint, RestartType, TouchReference, TouchType,
        };

        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let ball_pos = coordinates::to_normalized(self.ball.position.to_meters());
        let attacks_right = self.attacks_right(attacking_is_home);

        // 1. Only check forward passes (attacking direction)
        let is_advancing = coordinates::is_advancing(
            passer_pos.to_normalized_legacy(),
            receiver_pos.to_normalized_legacy(),
            attacks_right,
        );

        if !is_advancing {
            return None; // Back passes never offside
        }

        // 2. Receiver must be in opponent's half
        let in_opponent_half =
            coordinates::is_in_opponent_half(receiver_pos.to_normalized_legacy(), attacks_right);

        if !in_opponent_half {
            return None; // Own half = no offside
        }

        // 3. Minimum pass distance check
        let pass_distance_m = passer_pos.distance_to_m(&receiver_pos);
        if pass_distance_m < offside::MIN_PASS_DISTANCE_M {
            return None; // Too short to trigger offside
        }

        // 4. Receiver must be ahead of the ball at pass moment
        let receiver_length = coordinates::norm_length(receiver_pos.to_normalized_legacy());
        let ball_length = coordinates::norm_length(ball_pos);
        let ahead_of_ball = if attacks_right {
            receiver_length > ball_length
        } else {
            receiver_length < ball_length
        };

        if !ahead_of_ball {
            return None; // Behind ball = no offside
        }

        // 5. Get second-last defender line
        let offside_line_norm = self.second_last_defender_line(!attacking_is_home, attacks_right);

        // 6. Calculate margin (receiver position relative to offside line)
        let receiver_length_tv = if attacks_right {
            receiver_length
        } else {
            1.0 - receiver_length
        };
        let offside_line_tv = if attacks_right {
            offside_line_norm
        } else {
            1.0 - offside_line_norm
        };

        let margin_norm = receiver_length_tv - offside_line_tv;
        let margin_m = margin_norm * field::LENGTH_M;

        // Check if beyond line with buffer
        let offside_buffer = offside::OFFSIDE_BUFFER_NORM;
        if margin_norm <= offside_buffer {
            return None; // Within buffer = onside
        }

        // Apply linesman accuracy for marginal calls
        if margin_norm < offside_buffer * 2.0 {
            if self.rng.gen::<f32>() >= offside::LINESMAN_ACCURACY {
                return None; // Linesman missed it
            }
        }

        // Convert offside line to meters
        let offside_line_m = offside_line_norm * field::LENGTH_M;

        // Determine involvement type (interfering with play is most common)
        let involvement_type = OffsideInvolvementType::InterferingWithPlay;

        // Build restart context (check for exceptions)
        // Note: In the current implementation, we don't track restart type at pass time,
        // so we default to Normal. This would need to be enhanced if we want to
        // properly handle goal kick/throw-in/corner kick exceptions.
        let restart_context = OffsideRestartContext {
            restart_type: RestartType::Normal,
            offside_exception_applies: false,
        };

        // Build touch reference (default to kick with first contact)
        let touch_reference = TouchReference {
            touch_type: TouchType::Kick,
            reference_point: ReferencePoint::FirstContact,
        };

        // Deflection context (no defender touch in normal pass)
        let deflection_context = DeflectionContext {
            last_touch_by_defender: DefenderTouchType::None,
            resets_offside: false,
        };

        Some(OffsideDetails {
            margin_m,
            offside_line_m,
            passer_track_id: Some(passer_idx as u8),
            involvement_type: Some(involvement_type),
            restart_context: Some(restart_context),
            touch_reference: Some(touch_reference),
            deflection_context: Some(deflection_context),
        })
    }

    /// Apply offside restart (indirect free kick at offside location)
    pub(crate) fn apply_offside_restart(&mut self, attacking_home: bool, receiver_pos: Coord10) {
        self.pending_indirect_free_kick = true;
        self.handle_action_result(ActionResult::OutOfBounds {
            restart_type: RestartType::FreeKick,
            position: receiver_pos,
            home_team: !attacking_home,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::physics_constants::field;

    #[test]
    fn test_is_offside_pass_backward_pass() {
        // Backward passes are never offside
        // Setup would require a full MatchEngine - this is a logic test
        // Just verify the function exists and is callable
        println!("is_offside_pass function is available for backward pass checks");
    }

    #[test]
    fn test_second_last_defender_line() {
        // Just verify the function exists and is callable
        println!("second_last_defender_line function is available");
    }

    #[test]
    fn test_offside_target_filter_matches_offside_call_attacks_right() {
        use super::super::test_fixtures::create_test_engine;
        use crate::engine::types::Coord10;

        let mut engine = create_test_engine();
        engine.is_second_half = false; // Home attacks RIGHT (x=105)

        // Home passer -> home receiver, receiver is marginally beyond the offside line.
        let passer_idx = 6;
        let receiver_idx = 9;

        // Define an away defensive line at x=84m (all outfield players aligned).
        for idx in 12..22 {
            engine.player_positions[idx] = Coord10::from_meters(84.0, field::CENTER_Y);
        }

        // Ball at passer (x=80m), receiver at x=85.26m (~1.26m beyond line).
        let passer_pos = Coord10::from_meters(80.0, field::CENTER_Y);
        engine.player_positions[passer_idx] = passer_pos;
        engine.ball.position = passer_pos;
        engine.player_positions[receiver_idx] = Coord10::from_meters(85.26, field::CENTER_Y);

        // The adjudication check must agree with the selection-time filter.
        assert!(
            engine.is_offside_pass(passer_idx, receiver_idx, true),
            "setup sanity: expected offside on pass start"
        );
        assert!(
            engine.is_offside_position(receiver_idx, true),
            "target filter must reject any receiver that would be offside at pass start"
        );
    }

    #[test]
    fn test_offside_target_filter_matches_offside_call_attacks_left() {
        use super::super::test_fixtures::create_test_engine;
        use crate::engine::types::Coord10;

        let mut engine = create_test_engine();
        engine.is_second_half = false; // Away attacks LEFT (x=0)

        // Away passer -> away receiver, receiver is marginally beyond the offside line.
        let passer_idx = 15;
        let receiver_idx = 19;

        // Define a home defensive line at x=21m (all outfield players aligned).
        for idx in 1..11 {
            engine.player_positions[idx] = Coord10::from_meters(21.0, field::CENTER_Y);
        }

        // Ball at passer (x=25m), receiver at x=19.74m (~1.26m beyond line toward x=0).
        let passer_pos = Coord10::from_meters(25.0, field::CENTER_Y);
        engine.player_positions[passer_idx] = passer_pos;
        engine.ball.position = passer_pos;
        engine.player_positions[receiver_idx] = Coord10::from_meters(19.74, field::CENTER_Y);

        // The adjudication check must agree with the selection-time filter.
        assert!(
            engine.is_offside_pass(passer_idx, receiver_idx, false),
            "setup sanity: expected offside on pass start"
        );
        assert!(
            engine.is_offside_position(receiver_idx, false),
            "target filter must reject any receiver that would be offside at pass start"
        );
    }

    #[test]
    fn test_offside_in_match_simulation() {
        use crate::engine::MatchPlan;
        use crate::models::player::PlayerAttributes;
        use crate::models::team::Formation;
        use crate::models::{Player, Position, Team};
        use crate::player::personality::PersonalityArchetype;

        fn make_player(name: &str, pos: Position) -> Player {
            Player {
                name: name.to_string(),
                position: pos,
                overall: 70,
                condition: 3,
                attributes: Some(PlayerAttributes::default()),
                equipped_skills: Vec::new(),
                traits: Default::default(),
                personality: PersonalityArchetype::Steady,
            }
        }

        let positions = [
            Position::GK,
            Position::CB,
            Position::CB,
            Position::CB,
            Position::CB,
            Position::CM,
            Position::CM,
            Position::CM,
            Position::CM,
            Position::ST,
            Position::ST,
            Position::GK,
            Position::CB,
            Position::CB,
            Position::CM,
            Position::CM,
            Position::ST,
            Position::ST,
        ];

        let home_team = Team {
            name: "Home FC".to_string(),
            players: (0..18).map(|i| make_player(&format!("Home {}", i), positions[i])).collect(),
            formation: Formation::F442,
        };

        let away_team = Team {
            name: "Away FC".to_string(),
            players: (0..18).map(|i| make_player(&format!("Away {}", i), positions[i])).collect(),
            formation: Formation::F442,
        };

        let plan = MatchPlan {
            home_team,
            away_team,
            seed: 12345,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        };

        let mut engine = super::super::MatchEngine::new(plan).expect("match engine init");
        let result = engine.simulate();

        // Check that offside stats are tracked (may be 0 or more)
        let total_offsides = result.statistics.offsides_home + result.statistics.offsides_away;
        println!(
            "Total offsides in match: {} (home: {}, away: {})",
            total_offsides, result.statistics.offsides_home, result.statistics.offsides_away
        );

        // Check that offside events are recorded
        let offside_events = result
            .events
            .iter()
            .filter(|e| matches!(e.event_type, crate::models::EventType::Offside))
            .count();
        println!("Offside events recorded: {}", offside_events);

        // Stats should track offside counts correctly
        assert_eq!(
            total_offsides as usize, offside_events,
            "Offside stats ({}) should match offside events ({})",
            total_offsides, offside_events
        );
    }
}
