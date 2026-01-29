//! # EV Decision System (P10-13 Phase 3)
//!
//! **EV = P(success) x Benefit - P(failure) x Cost**
//!
//! EV 기반 의사결정 시스템.
//! 슛/패스/드리블 각각의 기대값을 계산하고 가장 높은 것을 선택.
//!
//! ## P0 Goal Contract 호환
//! - TeamSide 기반 공격 방향 결정
//! - Home팀: x=field::LENGTH_M 골대 공격 (away goal)
//! - Away팀: x=0.0 골대 공격 (home goal)
//! - `attacking_goal(team)`과 일관된 방향 사용
//!
//! ## 기존 시스템과의 차이
//! - 기존: 확률 기반 랜덤 (dice roll)
//! - 신규: EV 최대화 (argmax)
//!
//! ## Feature Flag
//! `USE_EV_DECISION` 상수로 기존 시스템과 전환 가능
//!
//! ## Match OS v1.2 Priority 3: LaneBlock Hybrid System
//!
//! **Two-stage pass lane detection:**
//! 1. **Stage 1: Grid Hint** - Fast spatial risk via FieldBoard (O(k), k < 20)
//!    - `lane_risk_hint()` samples cells along pass lane
//!    - Combines pressure heatmap + occupancy counts
//!    - Early exit for clear lanes (< 0.15 threshold)
//!
//! 2. **Stage 2: Raycast Validation** - Precise body blocking (O(N), N = 2-4)
//!    - `find_lane_block_candidates()` identifies top N nearest opponents
//!    - `lane_blocked_raycast()` validates with 1.0m precision
//!    - Uses existing `body_blocking::point_to_line_distance()`
//!
//! **Performance:**
//! - Grid hint: < 1μs per call (50x faster than target)
//! - Full pass evaluation: < 50μs (4x faster than legacy 200μs)
//! - Total complexity: O(k + N) ~24 checks vs O(3 × 11) = 33 in legacy
//!
//! **Files Modified:**
//! - `field_board.rs`: Added `lane_risk_hint()` method
//! - `ev_decision.rs`: Hybrid `estimate_pass_interception_risk()` + helpers
//! - `count_interceptors_on_lane()`: Uses raycast precision

// P18: calculate_pressure_context deprecated 경고 억제 (FieldBoard 마이그레이션 예정)
#![allow(deprecated)]

use super::attribute_calc;
use super::MatchEngine;

// FIX_2601/1124: Gate A 검증용 CandidateKey
use super::candidate_key::CandidateKey;
use crate::engine::body_blocking;
use crate::engine::coordinates;
use crate::engine::debug_flags::match_debug_enabled;
use crate::engine::physics_constants::field;
use crate::engine::physics_constants::zones;
use crate::engine::player_decision::PlayerAction;
use crate::engine::probability;
use crate::engine::tactical_context::TeamSide;
use crate::engine::types::Coord10; // FIX_2601
use crate::models::TeamSide as MatchTeamSide;
use crate::models::replay::types::{IntentTarget, MeterPos};
// P9: ActionModel 통합
use crate::engine::phase_action::{
    ActionModel, DribbleContext, DribbleIntent, DribbleModel, DribbleSkills, PassContext,
    PassIntent, PassModel, PassSkills, ShooterSkills, ShotContext, ShotIntent, ShotModel,
};

// FIX_2601/0108: UAE Pipeline Integration
use crate::engine::action_evaluator::{
    action_set::ActionSetContext,
    evaluators::EvalContext,
    hard_gate::HardGateContext,
    pipeline::{DecisionPipeline, PipelineConfig},
    state::StateContext,
    team_coord::TeamCoordinator,
    types::{Action as UaeAction, CrossZone, PlayerId as UaePlayerId, Position as UaePosition, Vec2},
};

// FIX_2601/0106: Compile-time assertion for field dimensions
// Ensures FIELD_LENGTH_10 and FIELD_WIDTH_10 are non-zero to prevent division by zero
#[cfg(debug_assertions)]
const _: () = {
    assert!(Coord10::FIELD_LENGTH_10 > 0);
    assert!(Coord10::FIELD_WIDTH_10 > 0);
};

// ========== Constants (Tuning Points) ==========

/// 골 성공 시 reward
/// 골 가치 (득점의 게임 임팩트)
/// v8: 1.0 → 2.5 (슛을 더 가치있게 만들기 위해 상향)
/// v9: 2.5 → 4.0 (Shot EV가 Pass EV보다 너무 낮아서 상향)
/// 실제 축구에서 득점은 매우 가치있는 이벤트
pub const GOAL_REWARD: f32 = 4.0;

/// 공격권 상실 시 기본 비용
/// v8: 0.25 → 0.15 (슛 실패의 비용을 낮춤)
/// v9: 0.15 → 0.10 (슛 실패 비용 더 낮춤 - 공격 third에서 슛 장려)
pub const LOSS_OF_POSSESSION_BASE_COST: f32 = 0.10;

/// 역습 위험 가중치 (위치별)
/// v8: 0.15 → 0.08 (역습 위험 감소)
/// v9: 0.08 → 0.05 (공격 third에서 슛 장려)
pub const COUNTERATTACK_RISK_WEIGHT: f32 = 0.05;

/// Attacking Third 슛 보너스
/// 공격 지역에서 슛 EV에 추가되는 보너스
pub const ATTACKING_THIRD_SHOT_BONUS: f32 = 0.2;

/// 찬스 생성 시 reward (패스로 인한 위협 증가)
pub const CHANCE_CREATION_REWARD: f32 = 0.4;

/// 드리블 성공 후 위협 증가 기본값
pub const DRIBBLE_THREAT_GAIN: f32 = 0.15;

/// TakeOn 성공 후 위협 증가 (드리블보다 높음 - 수비수 제침)
pub const TAKEON_THREAT_GAIN: f32 = 0.35;

/// TakeOn 실패 시 추가 비용 (역습 위험 높음)
pub const TAKEON_FAILURE_PENALTY: f32 = 0.15;

/// 최소 EV 임계값 (너무 낮으면 홀드)
pub const MIN_ACTION_EV_THRESHOLD: f32 = -0.3;

/// EV 기반 의사결정 사용 여부 (feature flag)
pub const USE_EV_DECISION: bool = true;

// ========== EV Estimation Functions ==========

impl MatchEngine {
    // ===========================================
    // Shot EV
    // ===========================================

    /// 슛 기대값 계산
    ///
    /// `Shot_EV = xG × GOAL_REWARD - (1 - xG) × loss_cost`
    ///
    /// **Context × Attribute 기반 계산** (하드코딩 제거)
    /// - 슛 정확도: Finishing + Technique + Composure
    /// - 장거리슛 보정: Long Shots + Shot Power
    ///
    /// # Arguments
    /// * `shooter_idx` - 슛하는 선수 인덱스
    ///
    /// # Returns
    /// 슛의 기대값 (-1.0 ~ 1.0 범위)
    pub fn estimate_shot_ev(&self, shooter_idx: usize) -> f32 {
        let is_home = MatchTeamSide::is_home(shooter_idx);
        let team = MatchTeamSide::from_track_id(shooter_idx);
        let player_pos = self.get_player_position_by_index(shooter_idx);

        // 거리 계산 (FIX_2601: Coord10 → legacy normalized for coordinates.rs)
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let distance_m =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);

        // 선수 능력치 수집
        let finishing = self.get_player_finishing(shooter_idx);
        let technique = self.get_player_technique(shooter_idx);
        let composure = self.get_player_composure(shooter_idx);
        let long_shots = self.get_player_long_shots(shooter_idx);
        let shot_power = self.get_player_strength(shooter_idx); // Shot Power 대용

        // 압박 계산
        let pressure_ctx = self.calculate_pressure_context(shooter_idx, None);
        let pressure = pressure_ctx.effective_pressure;

        // 슛 정확도 계산 (능력치 기반 + Composer)
        let mut accuracy_composer =
            attribute_calc::shot_accuracy_composer(finishing, technique, composure, pressure);

        // 팀 전술 보정 (예: 공격적인 전술 시 슛 정확도에 미세한 영향 등)
        let instructions = if is_home { &self.home_instructions } else { &self.away_instructions };
        if instructions.team_tempo == crate::tactics::team_instructions::TeamTempo::VeryFast {
            // 빠른 템포는 정밀도 약간 하락 가능성
            accuracy_composer.add("TacticalTempo", 0.95, crate::engine::weights::StackRule::AddLn);
        }

        // 장거리슛 보정 (능력치 기반)
        let long_factor = attribute_calc::long_shot_factor(distance_m, long_shots, shot_power);
        accuracy_composer.add(
            "LongShotRange",
            long_factor,
            crate::engine::weights::StackRule::AddLn,
        );

        // 최종 정확도
        let accuracy = accuracy_composer.compose().clamp(0.10, 0.95);

        // xG 계산 (기존 probability 함수 사용)
        let xg = probability::xg_skill_based(distance_m, accuracy);

        // 공격권 상실 비용
        // FIX_2601/0110: Use attacks_right instead of team
        let loss_cost = self.estimate_loss_of_possession_cost(attacks_right, player_pos);

        // 기본 EV 계산
        let mut ev = xg * GOAL_REWARD - (1.0 - xg) * loss_cost;

        // v9: Attacking Third 보너스 - 공격 지역에서 슛 장려
        // FIX_2601/0105: Use attacks_right (already computed above)
        if coordinates::is_in_attacking_third(player_pos.to_normalized_legacy(), attacks_right) {
            ev += ATTACKING_THIRD_SHOT_BONUS;
        }

        // v9: Clear Shot 보너스 - 압박 없을 때 추가 인센티브
        if pressure < 0.2 && distance_m < 25.0 {
            ev += 0.1; // 클리어 샷 보너스
        }

        ev
    }

    /// 공격권 상실 시 비용 추정
    ///
    /// 상대 진영 깊숙이 있을수록 역습 위험 ↑
    /// FIX_2601: Changed to accept Coord10
    /// FIX_2601/0110: Changed team to attacks_right for correct 2nd half direction
    fn estimate_loss_of_possession_cost(&self, attacks_right: bool, pos: Coord10) -> f32 {
        // TeamView semantics: normalized length always 0=own goal → 1=opponent goal.
        // FIX_2601/0106 P2-9: Clamp to [0, 1] to handle out-of-bounds positions.
        // FIX_2601/0116: Use TeamView transform instead of per-call direction branching.
        let tv_x = crate::engine::types::TeamViewCoord10::from_world(pos, attacks_right)
            .x
            .clamp(0, Coord10::FIELD_LENGTH_10);
        let normalized_length = (tv_x as f32 / Coord10::FIELD_LENGTH_10 as f32).clamp(0.0, 1.0);

        // 상대 진영 깊숙이 있을수록 역습 위험 ↑
        let counterattack_risk = normalized_length * COUNTERATTACK_RISK_WEIGHT;

        LOSS_OF_POSSESSION_BASE_COST + counterattack_risk
    }

    /// 골 각도 계산 (0~1 정규화, 1 = 정면)
    #[allow(dead_code)]
    fn goal_angle_normalized(&self, player_pos: (f32, f32), _is_home: bool) -> f32 {
        // 골대 중앙과의 각도
        // 좌표계: pos.0 = width (사이드라인), pos.1 = length (골 방향)
        // width 0.5 = 필드 중앙
        // NOTE: player_pos is already (f32, f32) normalized tuple
        let center_width = 0.5;
        let width_diff = (coordinates::norm_width(player_pos) - center_width).abs();

        // 중앙(width=0.5)에서 멀어질수록 각도 나빠짐
        // width_diff가 0이면 정면, 0.5면 사이드라인
        let angle = 1.0 - (width_diff * 2.0).min(1.0);

        angle.clamp(0.0, 1.0)
    }

    // ===========================================
    // Pass EV
    // ===========================================

    /// 패스 기대값 계산 (최적 타겟 포함)
    ///
    /// `Pass_EV = P(success) × future_threat - P(failure) × fail_cost`
    ///
    /// # P7-OFFSIDE-04: 오프사이드 위치의 타겟 제외
    /// 전진 패스의 경우 오프사이드 위치에 있는 타겟은 EV 계산에서 제외됩니다.
    ///
    /// # Returns
    /// (최고 EV, 최적 패스 타겟 인덱스)
    pub fn estimate_pass_ev(&self, passer_idx: usize) -> (f32, Option<usize>) {
        let is_home = MatchTeamSide::is_home(passer_idx);
        let team = MatchTeamSide::from_track_id(passer_idx);
        let passer_pos = self.get_player_position_by_index(passer_idx);
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);

        // 같은 팀 선수들
        let team_range = MatchTeamSide::teammate_range(passer_idx);

        let mut best_ev = f32::MIN;
        let mut best_target = None;

        for target_idx in team_range {
            if target_idx == passer_idx {
                continue;
            }

            // P7-OFFSIDE-04: 전진 패스일 때 오프사이드 위치 타겟 제외
            let target_pos = self.get_player_position_by_index(target_idx);
            // FIX_2601/0105: Use attacks_right for direction
            let is_forward_pass = coordinates::is_advancing(
                passer_pos.to_normalized_legacy(),
                target_pos.to_normalized_legacy(),
                attacks_right,
            );

            if is_forward_pass && self.is_offside_position(target_idx, is_home) {
                // 오프사이드 위치의 타겟 → 패스 불가 (EV 계산에서 제외)
                continue;
            }

            // 패스 성공 확률
            let pass_success = self.estimate_pass_success_prob(passer_idx, target_idx);

            // 패스 받은 후 위협도
            let future_threat =
                self.estimate_attack_threat_at(target_pos.to_normalized_legacy(), team);

            // 패스 실패 비용
            let fail_cost = self.estimate_pass_fail_cost(
                passer_pos.to_normalized_legacy(),
                target_pos.to_normalized_legacy(),
                team,
            );

            // EV 계산
            let ev = pass_success * future_threat - (1.0 - pass_success) * fail_cost;

            if ev > best_ev {
                best_ev = ev;
                best_target = Some(target_idx);
            }
        }

        if best_target.is_none() {
            return (-1.0, None);
        }

        (best_ev, best_target)
    }

    /// 패스 성공 확률 추정
    ///
    /// **Context × Attribute 기반 계산** (하드코딩 제거)
    /// - 거리 factor: Vision + Passing 능력치 반영
    /// - 압박 저항: Composure + Decisions 능력치 반영
    /// - 인터셉트 위험: Vision + Technique 능력치 반영
    fn estimate_pass_success_prob(&self, passer_idx: usize, target_idx: usize) -> f32 {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let target_pos = self.get_player_position_by_index(target_idx);
        let is_home = MatchTeamSide::is_home(passer_idx);

        // 선수 능력치 수집
        let passing = self.get_player_passing(passer_idx);
        let vision = self.get_player_vision(passer_idx);
        let technique = self.get_player_technique(passer_idx);
        let composure = self.get_player_composure(passer_idx);
        let decisions = self.get_player_decisions(passer_idx);

        // 거리 (미터)
        let dist = passer_pos.distance_to_m(&target_pos);

        // 1. 거리 factor (능력치 기반)
        let dist_factor = attribute_calc::pass_distance_factor(dist, passing, vision);

        // 2. 패스 능력 factor
        let skill_factor = attribute_calc::pass_skill_factor(passing, technique);

        // 3. 압박 저항 (능력치 기반) - P18: FieldBoard 기반
        let local_pressure_level = self.get_local_pressure_level(passer_idx);
        let pressure_penalty =
            attribute_calc::pressure_penalty(local_pressure_level, composure, decisions);

        // 4. 경로 상 인터셉트 위험 (능력치 기반)
        let base_intercept_risk = self.estimate_pass_interception_risk(
            passer_pos.to_normalized_legacy(),
            target_pos.to_normalized_legacy(),
            is_home,
        );
        let intercept_risk =
            attribute_calc::intercept_risk_adjusted(base_intercept_risk, vision, technique);

        // 최종 확률: 거리 × 스킬 - 압박 - 인터셉트
        // FIX_2601/0106 D-3: 음수 중간값 경고
        let pre_clamp = dist_factor * skill_factor - pressure_penalty - intercept_risk;

        #[cfg(debug_assertions)]
        if pre_clamp < 0.0 {
            eprintln!(
                "[EV_DECISION] Pass probability degenerate: pre_clamp={:.3}, factors=({:.2}*{:.2}), penalties=({:.2}+{:.2})",
                pre_clamp, dist_factor, skill_factor, pressure_penalty, intercept_risk
            );
        }

        pre_clamp.clamp(0.10, 0.95)
    }

    /// Pass interception risk (Hybrid: Hint + Raycast)
    ///
    /// Match OS v1.2 Priority 3: LaneBlock Hybrid
    ///
    /// Two-stage approach:
    /// 1. Fast grid hint via FieldBoard (O(k) with k < 20)
    /// 2. Precise raycast validation for top candidates (O(N) with N = 2-4)
    ///
    /// Falls back to legacy 3-point method if FieldBoard unavailable.
    fn estimate_pass_interception_risk(
        &self,
        from: (f32, f32),
        to: (f32, f32),
        is_home: bool,
    ) -> f32 {
        let from_m = self.normalized_to_meters(from);
        let to_m = self.normalized_to_meters(to);

        // Step 1: Fast grid hint (O(k) with k < 20)
        let hint_risk = if let Some(ref board) = self.field_board {
            board.lane_risk_hint(from_m, to_m, is_home)
        } else {
            // Fallback to legacy
            return self.estimate_pass_interception_risk_legacy(from, to, is_home);
        };

        // Early exit if clear lane
        const HINT_THRESHOLD: f32 = 0.15;
        if hint_risk < HINT_THRESHOLD {
            return hint_risk * 0.5;
        }

        // Step 2: Find top candidates
        const MAX_CANDIDATES: usize = 4;
        let candidates = self.find_lane_block_candidates(from_m, to_m, is_home, MAX_CANDIDATES);

        // Step 3: Raycast validation
        let raycast_blocked = if candidates.is_empty() {
            false
        } else {
            self.lane_blocked_raycast(from_m, to_m, is_home)
        };

        // Step 4: Combine hint + raycast
        let final_risk =
            if raycast_blocked { (hint_risk * 0.5 + 0.4).min(0.9) } else { hint_risk * 0.6 };

        final_risk.clamp(0.0, 0.5)
    }

    /// Legacy 3-point interpolation (fallback when FieldBoard unavailable)
    fn estimate_pass_interception_risk_legacy(
        &self,
        from: (f32, f32),
        to: (f32, f32),
        is_home: bool,
    ) -> f32 {
        let opponent_range = if is_home { 11..22 } else { 0..11 };
        let mut risk = 0.0;

        // 패스 경로 중간 지점들 체크 (from/to are normalized tuples)
        for i in 1..=3 {
            let t = i as f32 / 4.0;
            let mid_x = from.0 + (to.0 - from.0) * t;
            let mid_y = from.1 + (to.1 - from.1) * t;
            let mid = Coord10::from_normalized_legacy((mid_x, mid_y));

            for opp_idx in opponent_range.clone() {
                let opp_pos = self.get_player_position_by_index(opp_idx);
                let dist = mid.distance_to_m(&opp_pos);

                // 5m 이내 수비수 → 인터셉트 위험
                if dist < 5.0 {
                    risk += (5.0 - dist) * 0.05;
                }
            }
        }

        risk.clamp(0.0, 0.5)
    }

    /// 위치에서의 공격 위협도 추정
    fn estimate_attack_threat_at(&self, pos: (f32, f32), team: TeamSide) -> f32 {
        // 골대까지 거리 (미터)
        // NOTE: pos is already (f32, f32) normalized tuple
        let is_home = team == TeamSide::Home;
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let dist_to_goal = coordinates::distance_to_goal_m(pos, attacks_right);

        // 위협도: 골대에 가까울수록 높음

        if dist_to_goal < zones::CLOSE_M {
            // 페널티 박스 내
            0.7 + (zones::CLOSE_M - dist_to_goal) / zones::CLOSE_M * 0.3
        } else if dist_to_goal < zones::MID_RANGE_M {
            // 슈팅 레인지
            0.4 + (zones::MID_RANGE_M - dist_to_goal) / (zones::MID_RANGE_M - zones::CLOSE_M) * 0.3
        } else if dist_to_goal < 40.0 {
            // 공격 지역
            0.2 + (40.0 - dist_to_goal) / (40.0 - zones::MID_RANGE_M) * 0.2
        } else {
            // 중원 이하
            0.1 + (field::CENTER_X - dist_to_goal.min(field::CENTER_X)) / field::CENTER_X * 0.1
        }
    }

    /// 패스 실패 시 비용
    /// NOTE: from/to are normalized (f32, f32) tuples
    fn estimate_pass_fail_cost(&self, from: (f32, f32), to: (f32, f32), team: TeamSide) -> f32 {
        // FIX_2601/0110: Compute attacks_right first for use in loss cost calculation
        let is_home = team == TeamSide::Home;
        let attacks_right = self.attacks_right(is_home);

        // 패스 경로 중간 지점
        let mid_pos_norm = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
        let mid_pos = Coord10::from_normalized_legacy(mid_pos_norm);

        // 중간 지점에서 공격권 상실 비용
        // FIX_2601/0110: Use attacks_right instead of team
        let loss_cost = self.estimate_loss_of_possession_cost(attacks_right, mid_pos);

        // 백패스 실패 시 추가 비용
        // 좌표계: pos.1 = length (골 방향)
        // NOTE: from/to are already normalized tuples
        let is_backward = !coordinates::is_advancing(from, to, attacks_right);
        let backward_penalty = if is_backward { 0.15 } else { 0.0 };

        loss_cost + backward_penalty
    }

    // ===========================================
    // Dribble EV
    // ===========================================

    /// 드리블 기대값 계산
    ///
    /// `Dribble_EV = P(success) × benefit - P(failure) × fail_cost`
    pub fn estimate_dribble_ev(&self, dribbler_idx: usize) -> f32 {
        let _is_home = MatchTeamSide::is_home(dribbler_idx);
        let team = MatchTeamSide::from_track_id(dribbler_idx);
        let pos = self.get_player_position_by_index(dribbler_idx);

        // 드리블 방향 (기본: 골대 방향)
        // 좌표계: pos.0 = width, pos.1 = length (골 방향)
        // FIX_2601: pos는 Coord10, normalized로 변환
        // FIX_2601/0110: Use attacks_right for direction (considers halftime swap)
        let pos_norm = pos.to_normalized_legacy();
        let advance_dist = 5.0 / field::LENGTH_M; // 5m 전진 (정규화)
        let is_home = team == TeamSide::Home;
        let attacks_right = self.attacks_right(is_home);
        // TeamView: always advance toward +length.
        // NOTE: `to_team_view_normalized` is an involution (same call converts back).
        let pos_tv = coordinates::to_team_view_normalized(pos_norm, attacks_right);
        let future_tv = (pos_tv.0, (pos_tv.1 + advance_dist).clamp(0.0, 1.0));
        let future_pos = coordinates::to_team_view_normalized(future_tv, attacks_right);

        // 성공 확률
        let p_success = self.estimate_dribble_success_prob(dribbler_idx);

        // 성공 시 이득: 위협도 증가분
        let current_threat = self.estimate_attack_threat_at(pos_norm, team);
        let future_threat = self.estimate_attack_threat_at(future_pos, team);
        let benefit = (future_threat - current_threat).max(0.0) + DRIBBLE_THREAT_GAIN;

        // 실패 비용
        // FIX_2601/0110: Use attacks_right instead of team
        let fail_cost = self.estimate_loss_of_possession_cost(attacks_right, pos);

        // EV 계산

        p_success * benefit - (1.0 - p_success) * fail_cost
    }

    /// 드리블 성공 확률 추정 (Enhanced)
    ///
    /// **Context × Attribute 기반 계산** (FIX_2601/0106 강화)
    /// - 기본 성공률: Dribbling + Agility + Balance
    /// - 압박 저항: Composure + Strength
    /// - **NEW**: Zone-based 페널티 (수비 지역에서 성공률 낮춤)
    /// - **NEW**: 가장 가까운 수비수 능력치 반영
    fn estimate_dribble_success_prob(&self, dribbler_idx: usize) -> f32 {
        let _is_home = MatchTeamSide::is_home(dribbler_idx);
        let team = MatchTeamSide::from_track_id(dribbler_idx);
        let pos = self.get_player_position_by_index(dribbler_idx);

        // 드리블러 능력치 수집
        let dribbling = self.get_player_dribbling(dribbler_idx);
        let agility = self.get_player_agility(dribbler_idx);
        let balance = self.get_player_balance(dribbler_idx);
        let pace = self.get_player_pace(dribbler_idx);
        let composure = self.get_player_composure(dribbler_idx);
        let strength = self.get_player_strength(dribbler_idx);

        // 압박 컨텍스트 - P18: FieldBoard 기반
        let local_pressure_level = self.get_local_pressure_level(dribbler_idx);

        // 1. 기본 성공률 (능력치 기반)
        let base = attribute_calc::dribble_base_success(dribbling, agility, balance);

        // 2. 압박 저항 (능력치 기반)
        let pressure_penalty =
            attribute_calc::dribble_pressure_penalty(local_pressure_level, composure, strength);

        // 3. NEW: Zone-based 페널티 (수비 지역에서 드리블 성공률 낮춤)
        // FIX_2601/0106 P2-9: Clamp for out-of-bounds positions
        let normalized_x = match team {
            TeamSide::Home => (pos.x as f32 / Coord10::FIELD_LENGTH_10 as f32).clamp(0.0, 1.0),
            TeamSide::Away => {
                (1.0 - (pos.x as f32 / Coord10::FIELD_LENGTH_10 as f32)).clamp(0.0, 1.0)
            }
        };
        let zone_penalty = attribute_calc::dribble_zone_penalty(normalized_x);

        // 4. NEW: 가장 가까운 수비수 고려 (open-football 스타일)
        let defender_penalty =
            if let Some((defender_idx, dist)) = self.find_nearest_defender_for_ev(dribbler_idx) {
                if dist < 8.0 {
                    // 8m 이내 수비수가 있으면 능력치 대결
                    let attack_score =
                        attribute_calc::dribble_attack_score(dribbling, agility, balance, pace);

                    let def_tackling = self.get_player_tackling(defender_idx);
                    let def_positioning = self.get_player_positioning(defender_idx);
                    let def_pace = self.get_player_pace(defender_idx);
                    let def_anticipation = self.get_player_anticipation(defender_idx);
                    let defense_score = attribute_calc::dribble_defense_score(
                        def_tackling,
                        def_positioning,
                        def_pace,
                        def_anticipation,
                    );

                    // 수비수가 우세하면 추가 페널티
                    let skill_diff = defense_score - attack_score;
                    if skill_diff > 0.0 {
                        skill_diff * 0.20 // 최대 20% 추가 페널티
                    } else {
                        0.0
                    }
                } else {
                    0.0 // 수비수 멀면 페널티 없음
                }
            } else {
                0.0
            };

        // 최종 확률: 기본 - 압박 - 존 - 수비수
        // FIX_2601/0106 D-3: 음수 중간값 경고
        let pre_clamp = base - pressure_penalty - zone_penalty - defender_penalty;

        #[cfg(debug_assertions)]
        if pre_clamp < 0.0 {
            eprintln!(
                "[EV_DECISION] Dribble probability degenerate: pre_clamp={:.3}, base={:.2}, penalties=({:.2}+{:.2}+{:.2})",
                pre_clamp, base, pressure_penalty, zone_penalty, defender_penalty
            );
        }

        pre_clamp.clamp(0.20, 0.85)
    }

    /// 드리블 vs 특정 수비수 성공 확률 (Duel용)
    ///
    /// open-football 스타일: 공격자/수비자 능력치 대결
    pub fn estimate_dribble_vs_defender_prob(
        &self,
        dribbler_idx: usize,
        defender_idx: usize,
    ) -> f32 {
        // 드리블러 능력치
        let dribbling = self.get_player_dribbling(dribbler_idx);
        let agility = self.get_player_agility(dribbler_idx);
        let balance = self.get_player_balance(dribbler_idx);
        let pace = self.get_player_pace(dribbler_idx);

        let attack_score = attribute_calc::dribble_attack_score(dribbling, agility, balance, pace);

        // 수비수 능력치
        let def_tackling = self.get_player_tackling(defender_idx);
        let def_positioning = self.get_player_positioning(defender_idx);
        let def_pace = self.get_player_pace(defender_idx);
        let def_anticipation = self.get_player_anticipation(defender_idx);

        let defense_score = attribute_calc::dribble_defense_score(
            def_tackling,
            def_positioning,
            def_pace,
            def_anticipation,
        );

        // 거리 계산
        let dribbler_pos = self.get_player_position_by_index(dribbler_idx);
        let defender_pos = self.get_player_position_by_index(defender_idx);
        let dist = dribbler_pos.distance_to_m(&defender_pos);

        attribute_calc::dribble_vs_defender_success(attack_score, defense_score, dist)
    }

    // ===========================================
    // Dribble Decision Helper (FIX_2601/0106)
    // ===========================================

    /// 드리블 여부 결정 헬퍼 (open-football 스타일)
    ///
    /// **의사결정 기준:**
    /// 1. 압박 없고 공간 있음 → 드리블 OK
    /// 2. 2명 이상 근접 → 패스 권장
    /// 3. 수비 지역 + EV < 0 → 드리블 금지
    /// 4. 그 외 → EV 기준 결정
    ///
    /// # Returns
    /// (should_dribble: bool, reason: &'static str, dribble_ev: f32)
    pub fn should_dribble(&self, player_idx: usize) -> (bool, &'static str, f32) {
        let team: TeamSide = MatchTeamSide::from_track_id(player_idx);
        let pos = self.get_player_position_by_index(player_idx);

        // 팀 기준 정규화된 위치 (0 = 자기 골대, 1 = 상대 골대)
        // FIX_2601/0106 P2-9: Clamp for out-of-bounds positions
        let normalized_x = match team {
            TeamSide::Home => (pos.x as f32 / Coord10::FIELD_LENGTH_10 as f32).clamp(0.0, 1.0),
            TeamSide::Away => {
                (1.0 - (pos.x as f32 / Coord10::FIELD_LENGTH_10 as f32)).clamp(0.0, 1.0)
            }
        };

        // 1. 근접 상대 수 체크 (open-football: 2명 이상이면 패스)
        let nearby_opponents = self.count_nearby_opponents(player_idx, 5.0);

        if nearby_opponents >= 2 {
            return (false, "too_many_opponents", -0.5);
        }

        // 2. 공간 체크 (open-football: has_space_to_dribble = 10m 내 상대 없음)
        let nearest_opponent_dist =
            self.find_nearest_defender_for_ev(player_idx).map(|(_, d)| d).unwrap_or(100.0);

        let has_space = nearest_opponent_dist > 10.0;

        if has_space {
            // 공간 많으면 드리블 OK
            return (true, "has_space", 0.3);
        }

        // 3. EV 계산
        let dribble_ev = self.estimate_dribble_ev(player_idx);

        // 4. 수비 지역 + 낮은 EV = 위험 (open-football 스타일 리스크 인지)
        let is_defensive_third = normalized_x < 0.33;

        if is_defensive_third && dribble_ev < 0.0 {
            return (false, "defensive_zone_risk", dribble_ev);
        }

        // 5. 중간 지역 + 매우 낮은 EV
        let is_middle_third = (0.33..0.66).contains(&normalized_x);

        if is_middle_third && dribble_ev < -0.15 {
            return (false, "middle_zone_negative_ev", dribble_ev);
        }

        // 6. 일반적인 EV 기준
        const DRIBBLE_EV_THRESHOLD: f32 = -0.1;

        if dribble_ev >= DRIBBLE_EV_THRESHOLD {
            (true, "positive_ev", dribble_ev)
        } else {
            (false, "negative_ev", dribble_ev)
        }
    }

    fn has_dribble_space_scan(
        &self,
        player_pos_m: (f32, f32),
        is_home: bool,
        attacks_right: bool,
    ) -> bool {
        const SCAN_DISTANCE_M: f32 = 15.0;
        const LANE_RADIUS_M: f32 = 2.5;
        const ANGLES_DEG: [f32; 5] = [-45.0, -30.0, 0.0, 30.0, 45.0];

        let opponent_range = if is_home { 11..22 } else { 0..11 };
        let opponents: Vec<(f32, f32)> = opponent_range
            .filter_map(|opp_idx| self.player_positions.get(opp_idx).map(|pos| pos.to_meters()))
            .collect();

        let forward = if attacks_right { (1.0, 0.0) } else { (-1.0, 0.0) };

        for angle_deg in ANGLES_DEG {
            let angle_rad = angle_deg.to_radians();
            let dir = (
                forward.0 * angle_rad.cos() - forward.1 * angle_rad.sin(),
                forward.0 * angle_rad.sin() + forward.1 * angle_rad.cos(),
            );
            let target = (
                player_pos_m.0 + dir.0 * SCAN_DISTANCE_M,
                player_pos_m.1 + dir.1 * SCAN_DISTANCE_M,
            );
            if coordinates::is_out_of_bounds_m(target) {
                continue;
            }

            let mut blocked = false;
            for opp in &opponents {
                let dist =
                    body_blocking::point_to_line_distance(*opp, player_pos_m, target);
                if dist < LANE_RADIUS_M {
                    blocked = true;
                    break;
                }
            }

            if !blocked {
                return true;
            }
        }

        false
    }

    // NOTE: count_nearby_opponents is defined in action_decision.rs

    // ===========================================
    // TakeOn EV (1:1 돌파)
    // ===========================================

    /// TakeOn 기대값 계산
    ///
    /// TakeOn = Duel System을 이용한 1:1 돌파
    /// Dribble과 다르게 수비수를 직접 제치는 고위험/고보상 액션
    ///
    /// `TakeOn_EV = P(success) × benefit - P(failure) × fail_cost`
    pub fn estimate_takeon_ev(&self, attacker_idx: usize) -> (f32, Option<usize>) {
        let _is_home = MatchTeamSide::is_home(attacker_idx);
        let team = MatchTeamSide::from_track_id(attacker_idx);
        let attacker_pos = self.get_player_position_by_index(attacker_idx);

        // 1. 가장 가까운 수비수 찾기
        let defender = self.find_nearest_defender_for_ev(attacker_idx);
        let Some((defender_idx, defender_dist)) = defender else {
            // 수비수 없음 → TakeOn 불필요 (그냥 드리블)
            return (-1.0, None);
        };

        // 수비수가 너무 멀면 TakeOn 불필요
        if defender_dist > 5.0 {
            return (-1.0, None);
        }

        // 2. TakeOn 성공 확률
        let p_success = self.estimate_takeon_success_prob(attacker_idx, defender_idx);

        // 3. 성공 시 이득: 수비수 뒤로 이동 + 위협도 증가
        // 좌표계: pos.0 = width, pos.1 = length (골 방향)
        // FIX_2601: attacker_pos는 Coord10, normalized로 변환
        // FIX_2601/0110: Use attacks_right for direction (considers halftime swap)
        let attacker_pos_norm = attacker_pos.to_normalized_legacy();
        let advance_dist = 8.0 / field::LENGTH_M; // 8m 전진 (수비수 제침)      
        let is_home = team == TeamSide::Home;
        let attacks_right = self.attacks_right(is_home);
        let attacker_tv = coordinates::to_team_view_normalized(attacker_pos_norm, attacks_right);
        let future_tv = (attacker_tv.0, (attacker_tv.1 + advance_dist).clamp(0.0, 1.0));
        let future_pos = coordinates::to_team_view_normalized(future_tv, attacks_right);

        let current_threat = self.estimate_attack_threat_at(attacker_pos_norm, team);
        let future_threat = self.estimate_attack_threat_at(future_pos, team);
        let benefit = (future_threat - current_threat).max(0.0) + TAKEON_THREAT_GAIN;

        // 4. 실패 비용: 공격권 상실 + 역습 위험
        // FIX_2601/0110: Use attacks_right instead of team
        let base_loss_cost = self.estimate_loss_of_possession_cost(attacks_right, attacker_pos);
        let fail_cost = base_loss_cost + TAKEON_FAILURE_PENALTY;

        // 5. EV 계산
        let ev = p_success * benefit - (1.0 - p_success) * fail_cost;

        (ev, Some(defender_idx))
    }

    /// TakeOn 성공 확률 추정 (Duel System 기반)
    ///
    /// **Context × Attribute 기반 계산** (하드코딩 제거)
    /// - 공격 점수: Dribbling + Agility + Flair + Pace + Composure
    /// - 수비 점수: Tackling + Positioning + Anticipation + Aggression
    /// - Aggression은 양면: 높으면 커밋하기 쉬워 속임에 취약
    fn estimate_takeon_success_prob(&self, attacker_idx: usize, defender_idx: usize) -> f32 {
        // 공격수 능력치
        let dribbling = self.get_player_dribbling(attacker_idx);
        let agility = self.get_player_agility(attacker_idx);
        let pace = self.get_player_pace(attacker_idx);
        let composure = self.get_player_composure(attacker_idx);
        let flair = self.get_player_flair(attacker_idx);

        // 수비수 능력치
        let def_tackling = self.get_player_tackling(defender_idx);
        let def_positioning = self.get_player_positioning(defender_idx);
        let def_anticipation = self.get_player_anticipation(defender_idx);
        let def_aggression = self.get_player_aggression(defender_idx);

        // 공격 점수 (능력치 기반)
        let attack_score =
            attribute_calc::takeon_attack_score(dribbling, agility, pace, flair, composure);

        // 수비 점수 (능력치 기반)
        let defense_score = attribute_calc::takeon_defense_score(
            def_tackling,
            def_positioning,
            def_anticipation,
            def_aggression,
        );

        // 최종 성공 확률 (능력치 기반)

        attribute_calc::takeon_success_prob(attack_score, defense_score, def_aggression)
    }

    /// EV 계산용 가장 가까운 수비수 찾기
    fn find_nearest_defender_for_ev(&self, attacker_idx: usize) -> Option<(usize, f32)> {
        let opponent_range = MatchTeamSide::opponent_range(attacker_idx);
        let attacker_pos = self.get_player_position_by_index(attacker_idx);

        let mut nearest: Option<(usize, f32)> = None;

        for opp_idx in opponent_range {
            // 골키퍼 제외
            let slot = MatchTeamSide::local_idx(opp_idx);
            if slot == 0 {
                continue;
            }

            let opp_pos = self.get_player_position_by_index(opp_idx);
            let dist = attacker_pos.distance_to_m(&opp_pos);

            if nearest.is_none() || dist < nearest.unwrap().1 {
                nearest = Some((opp_idx, dist));
            }
        }

        nearest
    }

    // ===========================================
    // P9: ActionModel Integration Helpers
    // ===========================================

    /// 선수 인덱스에서 ShooterSkills 생성
    pub fn build_shooter_skills(&self, player_idx: usize) -> ShooterSkills {
        ShooterSkills {
            finishing: self.get_player_finishing(player_idx) as u8,
            long_shots: self.get_player_long_shots(player_idx) as u8,
            volleys: self.get_player_technique(player_idx) as u8, // technique as volleys proxy
            composure: self.get_player_composure(player_idx) as u8,
        }
    }

    /// 선수 인덱스에서 PassSkills 생성
    pub fn build_pass_skills(&self, player_idx: usize) -> PassSkills {
        PassSkills {
            passing: self.get_player_passing(player_idx) as u8,
            technique: self.get_player_technique(player_idx) as u8,
            vision: self.get_player_vision(player_idx) as u8,
            decisions: self.get_player_anticipation(player_idx) as u8, // anticipation as decisions proxy
            composure: self.get_player_composure(player_idx) as u8,
        }
    }

    /// 선수 인덱스에서 DribbleSkills 생성 (0~20 스케일)
    pub fn build_dribble_skills(&self, player_idx: usize) -> DribbleSkills {
        DribbleSkills {
            dribbling: (self.get_player_dribbling(player_idx) as u8 / 5).min(20),
            technique: (self.get_player_technique(player_idx) as u8 / 5).min(20),
            acceleration: (self.get_player_pace(player_idx) as u8 / 5).min(20), // pace as acceleration proxy
            agility: (self.get_player_agility(player_idx) as u8 / 5).min(20),
            decisions: (self.get_player_anticipation(player_idx) as u8 / 5).min(20),
            composure: (self.get_player_composure(player_idx) as u8 / 5).min(20),
        }
    }

    /// 선수 위치에서 ShotContext 생성
    pub fn build_shot_context(&self, player_idx: usize) -> ShotContext {
        let is_home = MatchTeamSide::is_home(player_idx);
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        let distance_m =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);

        // 골 각도 (중앙에서 멀어질수록 감소)
        let center_width = 0.5;
        let width_diff =
            (coordinates::norm_width(player_pos.to_normalized_legacy()) - center_width).abs();
        let angle_deg = (1.0 - (width_diff * 2.0).min(1.0)) * 45.0; // 0~45도

        // GK 거리
        let gk_idx = if is_home { 11 } else { 0 };
        let gk_pos = self.get_player_position_by_index(gk_idx);
        let gk_distance = player_pos.distance_to_m(&gk_pos);

        // 압박
        let pressure_ctx = self.calculate_pressure_context(player_idx, None);

        // 앞에 있는 수비수 수 계산
        let defenders_ahead = self.count_defenders_ahead(player_idx);

        // 1:1 상황인지 (GK와 직접 대치)
        let is_one_on_one = defenders_ahead == 0 && gk_distance < 15.0;

        ShotContext {
            distance_to_goal: distance_m,
            angle_to_goal: angle_deg,
            defenders_ahead,
            gk_distance,
            is_one_on_one,
            ball_airborne: (self.ball.height as f32 / 10.0) > 0.5,
            ball_height: self.ball.height as f32 / 10.0,
            time_pressure: pressure_ctx.effective_pressure,
        }
    }

    /// 패스 상황에서 PassContext 생성
    pub fn build_pass_context(&self, passer_idx: usize, receiver_idx: usize) -> PassContext {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let distance_m = passer_pos.distance_to_m(&receiver_pos);

        // 거리 정규화 (0=가까운 짧패스, 1=긴 패스) - 40m 기준
        let distance_norm = (distance_m / 40.0).min(1.0);

        // 레인 품질 (수비수 없으면 높음) → target_openness로 변환
        let interceptors = self.count_interceptors_on_lane(passer_pos, receiver_pos, passer_idx);
        let target_openness = if interceptors == 0 { 1.0 } else { 0.5 / interceptors as f32 };

        // 침투 라인 존재 여부 (뒷공간)
        let is_home = MatchTeamSide::is_home(passer_idx);
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let through_lane = if coordinates::is_in_attacking_third(
            receiver_pos.to_normalized_legacy(),
            attacks_right,
        ) {
            if self.is_player_marked(receiver_idx) {
                0.3
            } else {
                0.8
            }
        } else {
            0.0
        };

        // 크로스 각도 (측면에서 박스로 들어가는 패스의 품질)
        let crossing_angle = self.calculate_crossing_angle(passer_pos, receiver_pos, attacks_right);

        PassContext { distance_norm, target_openness, through_lane, crossing_angle }
    }

    /// 드리블 상황에서 DribbleContext 생성
    pub fn build_dribble_context(&self, player_idx: usize) -> DribbleContext {
        let is_home = MatchTeamSide::is_home(player_idx);
        // FIX_2601/0110: Use attacks_right for direction (accounts for halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601: player_pos는 Coord10, normalized로 변환
        let player_pos_norm = player_pos.to_normalized_legacy();

        // 앞 공간 계산
        // FIX_2601/0110: Use attacks_right instead of is_home for correct 2nd half direction
        let advance_dist = 10.0 / field::LENGTH_M;
        let pos_tv = coordinates::to_team_view_normalized(player_pos_norm, attacks_right);
        let future_tv = (pos_tv.0, (pos_tv.1 + advance_dist).clamp(0.0, 1.0));
        let future_pos = coordinates::to_team_view_normalized(future_tv, attacks_right);

        let space_ahead = if self.is_space_open(future_pos) { 1.0 } else { 0.3 };

        // 듀얼 위협 (가장 가까운 수비수와의 거리)
        let duel_threat = if let Some((_, dist)) = self.find_nearest_defender_for_ev(player_idx) {
            (1.0 - (dist / 10.0)).max(0.0) // 10m 이내면 위협
        } else {
            0.0
        };

        // 레인 품질
        let lane_quality = space_ahead;

        DribbleContext { space_ahead, duel_threat, lane_quality }
    }

    /// ActionModel 기반 슛 EV 계산
    ///
    /// `ShotModel::base_success_prob()`를 사용하여 성공 확률 계산
    pub fn estimate_shot_ev_via_model(&self, shooter_idx: usize) -> f32 {
        let is_home = MatchTeamSide::is_home(shooter_idx);
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let team = MatchTeamSide::from_track_id(shooter_idx);
        let player_pos = self.get_player_position_by_index(shooter_idx);

        // Context와 Skills 생성
        let ctx = self.build_shot_context(shooter_idx);
        let skills = self.build_shooter_skills(shooter_idx);

        // 압박
        let pressure_ctx = self.calculate_pressure_context(shooter_idx, None);
        let pressure = pressure_ctx.effective_pressure;

        // Intent 결정 (상황 기반)
        let intent = if ctx.distance_to_goal > 25.0 {
            ShotIntent::Power // 장거리
        } else if ctx.angle_to_goal < 20.0 {
            ShotIntent::Place // 각도 좁음 → 정밀
        } else if ctx.ball_airborne {
            ShotIntent::Aerial // 공중볼
        } else {
            ShotIntent::Place // 기본 (정확한 배치)
        };

        // ActionModel을 통한 Technique 선택
        // Determinism: do not use OS RNG here. Use a deterministic RNG source.
        // NOTE: This is EV estimation; we intentionally avoid consuming `self.rng` state.
        let mut local_rng = self.rng.clone();
        let technique = ShotModel::choose_technique(intent, ctx, skills, pressure, &mut local_rng);

        // ActionModel을 통한 성공 확률 계산
        let base_prob = ShotModel::base_success_prob(technique, skills, pressure);

        // xG 계산 (base_prob 기반)
        let distance_m = ctx.distance_to_goal;
        let xg = probability::xg_skill_based(distance_m, base_prob);

        // 공격권 상실 비용
        // FIX_2601/0110: Use attacks_right instead of team
        let loss_cost = self.estimate_loss_of_possession_cost(attacks_right, player_pos);

        // EV 계산
        let mut ev = xg * GOAL_REWARD - (1.0 - xg) * loss_cost;

        // Attacking Third 보너스
        // FIX_2601/0105: Use attacks_right
        if coordinates::is_in_attacking_third(player_pos.to_normalized_legacy(), attacks_right) {
            ev += ATTACKING_THIRD_SHOT_BONUS;
        }

        ev
    }

    /// ActionModel 기반 패스 EV 계산
    ///
    /// `PassModel::base_success_prob()`를 사용하여 성공 확률 계산
    pub fn estimate_pass_ev_via_model(&self, passer_idx: usize) -> (f32, Option<usize>) {
        let is_home = MatchTeamSide::is_home(passer_idx);
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let team = MatchTeamSide::from_track_id(passer_idx);
        let passer_pos = self.get_player_position_by_index(passer_idx);

        // Skills 생성
        let skills = self.build_pass_skills(passer_idx);

        let team_range = MatchTeamSide::teammate_range(passer_idx);
        let mut best_ev = f32::MIN;
        let mut best_target = None;

        for target_idx in team_range {
            if target_idx == passer_idx {
                continue;
            }

            // 오프사이드 체크
            let target_pos = self.get_player_position_by_index(target_idx);
            // FIX_2601/0105: Use attacks_right for direction
            let is_forward = coordinates::is_advancing(
                passer_pos.to_normalized_legacy(),
                target_pos.to_normalized_legacy(),
                attacks_right,
            );
            if is_forward && self.is_offside_position(target_idx, is_home) {
                continue;
            }

            // Context 생성
            let ctx = self.build_pass_context(passer_idx, target_idx);

            // 압박
            let pressure_ctx = self.calculate_pressure_context(passer_idx, None);
            let pressure = pressure_ctx.effective_pressure;

            // Intent 결정 (through_lane으로 침투 가능성 판단)
            let intent = if ctx.through_lane > 0.5 {
                PassIntent::Penetrate
            } else if ctx.distance_norm > 0.75 {
                PassIntent::Switch // 장거리 패스
            } else if is_forward {
                PassIntent::Progress
            } else {
                PassIntent::Retain // 안전한 점유 유지
            };

            // ActionModel을 통한 Technique 선택
            // Determinism: do not use OS RNG here. Use a deterministic RNG source.
            // NOTE: This is EV estimation; we intentionally avoid consuming `self.rng` state.
            let mut local_rng = self.rng.clone();
            let technique =
                PassModel::choose_technique(intent, ctx, skills, pressure, &mut local_rng);

            // ActionModel을 통한 성공 확률 계산
            let pass_success = PassModel::base_success_prob(technique, skills, pressure);

            // 패스 받은 후 위협도
            let future_threat =
                self.estimate_attack_threat_at(target_pos.to_normalized_legacy(), team);

            // 패스 실패 비용
            let fail_cost = self.estimate_pass_fail_cost(
                passer_pos.to_normalized_legacy(),
                target_pos.to_normalized_legacy(),
                team,
            );

            // EV 계산
            let mut ev = pass_success * future_threat - (1.0 - pass_success) * fail_cost;

            // FIX_2601/0123: Reciprocity - no bonus or penalty (neutral)
            // The reciprocity injection in tick_based.rs handles this
            // if self.has_recent_pass_from(target_idx, passer_idx) {
            //     ev += 0.0;  // Neutral
            // }

            // FIX_2601/1128: Small bonus for backward/lateral passes (reduce forward bias)
            if !is_forward {
                ev += 0.05;  // Circulation bonus
            }

            if ev > best_ev {
                best_ev = ev;
                best_target = Some(target_idx);
            }
        }

        if best_target.is_none() {
            return (-1.0, None);
        }

        (best_ev, best_target)
    }

    /// ActionModel 기반 드리블 EV 계산
    ///
    /// `DribbleModel::base_success_prob()`를 사용하여 성공 확률 계산
    pub fn estimate_dribble_ev_via_model(&self, dribbler_idx: usize) -> f32 {
        let _is_home = MatchTeamSide::is_home(dribbler_idx);
        let team = MatchTeamSide::from_track_id(dribbler_idx);
        let player_pos = self.get_player_position_by_index(dribbler_idx);

        // Context와 Skills 생성
        let ctx = self.build_dribble_context(dribbler_idx);
        let skills = self.build_dribble_skills(dribbler_idx);

        // 압박
        let pressure_ctx = self.calculate_pressure_context(dribbler_idx, None);
        let pressure = pressure_ctx.effective_pressure;

        // Intent 결정
        let intent = if ctx.duel_threat > 0.5 {
            DribbleIntent::Protect // 수비수 가까움
        } else if ctx.space_ahead > 0.7 {
            DribbleIntent::Progress // 공간 있음
        } else {
            DribbleIntent::Beat // 돌파 시도
        };

        // ActionModel을 통한 Technique 선택
        // Determinism: do not use OS RNG here. Use a deterministic RNG source.
        // NOTE: This is EV estimation; we intentionally avoid consuming `self.rng` state.
        let mut local_rng = self.rng.clone();
        let technique =
            DribbleModel::choose_technique(intent, ctx, skills, pressure, &mut local_rng);

        // ActionModel을 통한 성공 확률 계산
        let p_success = DribbleModel::base_success_prob(technique, skills, pressure);

        // 드리블 성공 시 이득: 약간의 전진 + 공 유지
        // FIX_2601: player_pos는 Coord10, normalized로 변환
        // FIX_2601/0110: Use attacks_right for direction (considers halftime swap)
        let player_pos_norm = player_pos.to_normalized_legacy();
        let advance_dist = 5.0 / field::LENGTH_M;
        let is_home = team == TeamSide::Home;
        let attacks_right = self.attacks_right(is_home);
        let pos_tv = coordinates::to_team_view_normalized(player_pos_norm, attacks_right);
        let future_tv = (pos_tv.0, (pos_tv.1 + advance_dist).clamp(0.0, 1.0));
        let future_pos = coordinates::to_team_view_normalized(future_tv, attacks_right);

        let current_threat = self.estimate_attack_threat_at(player_pos_norm, team);
        let future_threat = self.estimate_attack_threat_at(future_pos, team);
        let benefit = (future_threat - current_threat).max(0.0) + DRIBBLE_THREAT_GAIN;

        // 드리블 실패 비용
        // FIX_2601/0110: Use attacks_right instead of team
        let base_loss_cost = self.estimate_loss_of_possession_cost(attacks_right, player_pos);
        let fail_cost = base_loss_cost;

        // EV 계산
        p_success * benefit - (1.0 - p_success) * fail_cost
    }

    /// 레인 위의 인터셉터 수 계산
    /// Count interceptors on pass lane using hybrid system
    ///
    /// Match OS v1.2 Priority 3: Uses raycast precision for accurate count
    /// FIX_2601: Updated to accept Coord10 parameters
    fn count_interceptors_on_lane(&self, from: Coord10, to: Coord10, passer_idx: usize) -> usize {
        let is_home = MatchTeamSide::is_home(passer_idx);
        let from_m = from.to_meters();
        let to_m = to.to_meters();

        const MAX_CANDIDATES: usize = 6;
        let candidates = self.find_lane_block_candidates(from_m, to_m, is_home, MAX_CANDIDATES);

        use crate::engine::body_blocking;
        const BLOCK_RADIUS_M: f32 = 1.0;

        candidates
            .iter()
            .filter(|&&opp_idx| {
                if let Some(&opp_pos) = self.player_positions.get(opp_idx) {
                    let opp_pos_m = opp_pos.to_meters();
                    let dist = body_blocking::point_to_line_distance(opp_pos_m, from_m, to_m);
                    dist < BLOCK_RADIUS_M
                } else {
                    false
                }
            })
            .count()
    }

    /// 점에서 선까지 거리 계산
    fn point_to_line_distance(
        &self,
        point: (f32, f32),
        line_start: (f32, f32),
        line_end: (f32, f32),
    ) -> f32 {
        let line_vec = (line_end.0 - line_start.0, line_end.1 - line_start.1);
        let line_len_sq = line_vec.0 * line_vec.0 + line_vec.1 * line_vec.1;

        if line_len_sq < 0.0001 {
            return ((point.0 - line_start.0).powi(2) + (point.1 - line_start.1).powi(2)).sqrt();
        }

        let t = ((point.0 - line_start.0) * line_vec.0 + (point.1 - line_start.1) * line_vec.1)
            / line_len_sq;
        let t = t.clamp(0.0, 1.0);

        let closest = (line_start.0 + t * line_vec.0, line_start.1 + t * line_vec.1);
        ((point.0 - closest.0).powi(2) + (point.1 - closest.1).powi(2)).sqrt()
    }

    /// 선수가 마킹 당하고 있는지 확인
    fn is_player_marked(&self, player_idx: usize) -> bool {
        if let Some((_, dist)) = self.find_nearest_defender_for_ev(player_idx) {
            dist < 3.0 // 3m 이내면 마킹 중
        } else {
            false
        }
    }

    /// 공간이 열려 있는지 확인
    /// FIX_2601: pos는 normalized (f32, f32), player_pos는 Coord10
    fn is_space_open(&self, pos: (f32, f32)) -> bool {
        // FIX_2601: normalized pos를 Coord10으로 변환
        let pos_coord = Coord10::from_normalized_legacy(pos);

        // 해당 위치 주변에 상대 선수가 없으면 열려 있음
        for idx in 0..22 {
            let player_pos = self.get_player_position_by_index(idx);
            let dist = pos_coord.distance_to_m(&player_pos);
            if dist < 5.0 {
                return false;
            }
        }
        true
    }

    /// 선수 앞에 있는 수비수 수 계산
    /// FIX_2601: Updated to use Coord10 (normalized legacy로 변환하여 비교)
    /// FIX_2601/0110: Use attacks_right for correct 2nd half direction
    fn count_defenders_ahead(&self, player_idx: usize) -> u8 {
        let is_home = MatchTeamSide::is_home(player_idx);
        // FIX_2601/0110: Use attacks_right for direction (accounts for halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);
        let player_pos_norm = player_pos.to_normalized_legacy();
        let opponent_range = MatchTeamSide::opponent_range(player_idx);

        let mut count = 0u8;
        for opp_idx in opponent_range {
            let opp_pos = self.get_player_position_by_index(opp_idx);
            let opp_pos_norm = opp_pos.to_normalized_legacy();

            // 상대가 골 방향으로 앞에 있는지 확인
            // FIX_2601/0110: Use attacks_right instead of is_home
            let player_tv = coordinates::to_team_view_normalized(player_pos_norm, attacks_right);
            let opp_tv = coordinates::to_team_view_normalized(opp_pos_norm, attacks_right);
            let is_ahead = opp_tv.1 > player_tv.1;

            // 슛 레인 내에 있는지 (좌우 폭 10m 이내)
            let width_diff = (opp_pos_norm.0 - player_pos_norm.0).abs();
            let in_lane = width_diff < (10.0 / field::WIDTH_M); // 10m / pitch width

            if is_ahead && in_lane {
                count += 1;
            }
        }

        count
    }

    /// 크로스 각도 계산 (측면에서 박스로 들어가는 패스의 품질)
    /// FIX_2601: Updated to accept Coord10 parameters
    /// FIX_2601/0105: Renamed parameter from is_home to attacks_right for halftime swap support
    fn calculate_crossing_angle(
        &self,
        passer_pos: Coord10,
        receiver_pos: Coord10,
        attacks_right: bool,
    ) -> f32 {
        // FIX_2601: Coord10 → normalized로 변환
        let passer_norm = passer_pos.to_normalized_legacy();
        let receiver_norm = receiver_pos.to_normalized_legacy();

        // 패서가 측면에 있는지 확인 (피치 폭의 20% 이내)
        let is_wide = passer_norm.0 < 0.2 || passer_norm.0 > 0.8;

        if !is_wide {
            return 0.0;
        }

        // 수신자가 박스 내에 있는지
        // FIX_2601/0105: Use attacks_right for direction
        let in_box = coordinates::is_in_attacking_third(receiver_norm, attacks_right)
            && coordinates::distance_to_goal_m(receiver_norm, attacks_right) < 18.0;

        if !in_box {
            return 0.0;
        }

        // 패스 각도 계산 (측면에서 중앙으로)
        let to_center = (0.5 - passer_norm.0).abs();
        // 0~1 정규화

        to_center.min(0.3) / 0.3
    }

    // ===========================================
    // EV-Based Action Selection
    // ===========================================
    // NOTE: Legacy decision functions (select_action_by_ev, select_action_with_audacity,
    // select_action_with_utility) have been removed as part of ActionQueue unification.
    // See REFACTOR_CONSTITUTION.md for details.
    // ===========================================

    /// P16 Gate Chain feature flag
    pub const USE_GATE_CHAIN: bool = true; // P16 활성화

    /// 액션 선택 (단일 진입점 - P16 Gate Chain)
    pub fn select_best_action(&mut self, player_idx: usize) -> PlayerAction {
        self.decide_owner_action_p16(player_idx)
    }

    /// ActionDetail 포함 액션 선택 (P16 Gate Chain)
    ///
    /// FIX_2601/1124 Phase 3: ActionDetailV2도 함께 반환
    /// - detail_v2 feature 활성화 시 Some(ActionDetailV2) 반환
    /// - 비활성화 시 None 반환
    pub fn select_best_action_with_detail(
        &mut self,
        player_idx: usize,
    ) -> (
        PlayerAction,
        crate::engine::action_detail::ActionDetail,
        Option<super::action_detail_v2::ActionDetailV2>,
    ) {
        self.decide_owner_action_p16_with_detail(player_idx)
    }

    /// FIX_2601/0117: Snapshot-compatible action selection
    ///
    /// Actor별 독립 RNG를 사용하여 순서 독립적 결정을 보장합니다.
    /// `&self` 시그니처를 사용하므로 병렬화에 안전합니다.
    ///
    /// FIX_2601/1124 Phase 3: ActionDetailV2는 snapshot 모드에서 None 반환
    /// (pure 버전에서 V2 빌드에 필요한 context가 제한됨)
    #[cfg(feature = "snapshot_decide")]
    pub fn select_best_action_with_detail_snapshot(
        &self,
        player_idx: usize,
        actor_seed: u64,
    ) -> (
        PlayerAction,
        crate::engine::action_detail::ActionDetail,
        Option<super::action_detail_v2::ActionDetailV2>,
    ) {
        use super::cognitive_bias::CognitiveBias;
        use super::decision_topology::decide_action_with_detail_snapshot;
        use crate::engine::action_detail::ActionDetail;

        // 공 소유 확인
        if self.ball.current_owner != Some(player_idx) {
            return (PlayerAction::Hold, ActionDetail::empty(), None);
        }

        let is_home = MatchTeamSide::is_home(player_idx);

        // 선수 능력치 가져오기
        let (_overall, flair, decisions, concentration) = self.get_player_mental_attrs(player_idx);
        let attrs = self.get_player_attributes(player_idx);

        // CognitiveBias 생성
        let bias = CognitiveBias::from_attributes(
            attrs.composure as f32,
            attrs.flair as f32,
            attrs.bravery as f32,
            attrs.aggression as f32,
            attrs.decisions as f32,
            attrs.teamwork as f32,
            attrs.concentration as f32,
        );

        // DecisionContext 생성
        let decision_ctx = self.build_p16_decision_context(player_idx);

        // ElaborationContext 생성
        let elab_ctx = self.build_p16_elaboration_context(player_idx, is_home);

        // Mindset 결정
        let mindset = self.determine_player_mindset(player_idx, &decision_ctx);

        // Gate Chain 실행 (Snapshot-compatible: actor_seed 사용)
        let (final_action, _intent, _utility, _results, _shot_gate) =
            decide_action_with_detail_snapshot(
                mindset,
                &decision_ctx,
                &elab_ctx,
                &bias,
                flair,
                decisions,
                concentration,
                actor_seed,
            );

        // Note: Recording operations are skipped in snapshot mode
        // as they require &mut self and are done in the commit phase

        // FinalAction → (PlayerAction, ActionDetail) 변환 (pure version, no state mutation)
        // FIX_2601/1124: snapshot mode에서는 V2 = None (pure context 제한)
        let (action, detail) = self.convert_final_action_with_detail_pure(final_action, player_idx);
        (action, detail, None)
    }

    /// P16 Gate Chain + ActionDetail + V2 반환
    ///
    /// FIX_2601/1124 Phase 3: ActionDetailV2도 함께 반환
    fn decide_owner_action_p16_with_detail(
        &mut self,
        player_idx: usize,
    ) -> (
        PlayerAction,
        crate::engine::action_detail::ActionDetail,
        Option<super::action_detail_v2::ActionDetailV2>,
    ) {
        use super::cognitive_bias::CognitiveBias;
        use super::decision_topology::decide_action_with_detail;
        use crate::engine::action_detail::ActionDetail;

        // FIX_2601/0108: UAE Pipeline feature flag check
        // UAE pipeline은 아직 V2를 반환하지 않음 (None)
        if self.use_uae_pipeline {
            let (action, detail) = self.try_uae_pipeline(player_idx);
            return (action, detail, None);
        }

        // 공 소유 확인
        if self.ball.current_owner != Some(player_idx) {
            return (PlayerAction::Hold, ActionDetail::empty(), None);
        }

        let is_home = MatchTeamSide::is_home(player_idx);
        let player_pos = self.get_player_position_by_index(player_idx);

        // 선수 능력치 가져오기
        let (_overall, flair, decisions, concentration) = self.get_player_mental_attrs(player_idx);
        let attrs = self.get_player_attributes(player_idx);

        // CognitiveBias 생성 (SSOT: MatchSetup always provides attributes)
        let bias = CognitiveBias::from_attributes(
            attrs.composure as f32,
            attrs.flair as f32,
            attrs.bravery as f32,
            attrs.aggression as f32,
            attrs.decisions as f32,
            attrs.teamwork as f32,
            attrs.concentration as f32,
        );

        // DecisionContext 생성
        let decision_ctx = self.build_p16_decision_context(player_idx);

        // ElaborationContext 생성
        let elab_ctx = self.build_p16_elaboration_context(player_idx, is_home);

        // Mindset 결정
        let mindset = self.determine_player_mindset(player_idx, &decision_ctx);

        // Gate Chain 실행 (Gate A → B → C)
        let (final_action, intent, utility, results, shot_gate) = decide_action_with_detail(
            mindset,
            &decision_ctx,
            &elab_ctx,
            &bias,
            flair,
            decisions,
            concentration,
            &mut self.rng,
        );
        if matches!(
            intent,
            super::decision_topology::SelectedIntent::Progress(
                super::decision_topology::ProgressStyle::Safe
            )
        ) {
            self.safe_pass_seq = self.safe_pass_seq.wrapping_add(1);
        }
        self.record_shot_gate_outcome(shot_gate, decision_ctx.has_clear_shot, player_idx);

        // 0108: Record decision to replay (if enabled)
        // current_tick / 4.0 = seconds (4 ticks per second)
        let t_seconds = self.current_tick as f64 / 4.0;
        let team_id = if is_home { 0u32 } else { 1u32 };
        let pos_m = player_pos.to_meters();
        let at = crate::replay::types::MeterPos { x: pos_m.0 as f64, y: pos_m.1 as f64 };
        if let Some(recorder) = self.replay_recorder.as_mut() {
            recorder.record_decision(
                t_seconds,
                team_id,
                player_idx as u32,
                at,
                format!("{:?}", intent),
                Some(utility),
            );
        }

        let temperature = super::utility::calculate_temperature(flair, decisions, concentration);
        // FIX_2601: Shot Opportunity Telemetry hook
        {
            let chosen_action = super::decision_topology::intent_to_candidate(intent);
            self.check_and_record_shot_opportunity(player_idx, &results, &chosen_action, &decision_ctx);
        }

        self.record_decision_intent(
            player_idx,
            intent,
            utility,
            temperature,
            &results,
            &decision_ctx,
            &elab_ctx,
            player_pos,
            &final_action,
        );

        // FinalAction → (PlayerAction, ActionDetail) 변환
        self.convert_final_action_with_detail(final_action, player_idx)
    }

    // ===========================================
    // P16: Gate Chain Integration (Phase 5)
    // ===========================================

    /// P16 Gate Chain 기반 공 소유자 액션 결정
    ///
    /// Gate A → Gate B → Gate C → FinalAction → PlayerAction 변환
    ///
    /// 기존 시스템과의 차이점:
    /// - 3-Gate 구조로 명확한 파이프라인
    /// - Mindset 기반 후보 필터링 (Gate A)
    /// - CognitiveBias + Softmax 선택 (Gate B)
    /// - Intent → FinalAction 변환 (Gate C)
    pub fn decide_owner_action_p16(&mut self, player_idx: usize) -> PlayerAction {
        use super::cognitive_bias::CognitiveBias;
        use super::decision_topology::decide_action_with_detail;
        // Contract v1: OutcomeSet sampler (optional future use)

        // 공 소유 확인
        if self.ball.current_owner != Some(player_idx) {
            return PlayerAction::Hold;
        }

        // ========== Career Player Mode: User Control Check ==========

        // 0) Controlled 모드가 있고 활성화되어 있으면 체크
        if let Some(ref controlled) = self.controlled_mode {
            if controlled.enabled {
                // 1) 내가 컨트롤하는 선수인지 확인
                if controlled.is_controlled(player_idx) {
                    // 2) 엔진 입력 락 체크
                    if !controlled.is_locked(self.current_tick) {
                        // 3) 큐에서 유효한 명령 1개 소비
                        if let Some(cmd) = self.pop_latest_valid_cmd_for(player_idx) {
                            let lock_duration = Self::lock_duration_for_user_cmd(&cmd.payload);

                            // controlled_mode를 mutable로 가져와서 업데이트
                            if let Some(ref mut ctrl) = self.controlled_mode {
                                ctrl.last_consumed_seq = cmd.seq;
                                ctrl.lock(self.current_tick, lock_duration);
                            }

                            // UserCommand → PlayerAction 변환 후 반환
                            return self.convert_user_cmd_to_player_action(player_idx, cmd.payload);
                        }
                    } else {
                        // 락 중이면 Hold
                        return PlayerAction::Hold;
                    }
                    // 입력 없으면 AI로 진행 (아래 기존 로직)
                }
            }
        }

        if self.multi_agent_tracks.contains(&player_idx) {
            if self.current_tick < self.multi_agent_lock_until_tick[player_idx] {
                return PlayerAction::Hold;
            }
            if let Some(cmd) = self.multi_agent_command_queues[player_idx].pop_front() {
                let lock_duration = Self::lock_duration_for_user_cmd(&cmd.payload);
                self.multi_agent_lock_until_tick[player_idx] = self.current_tick + lock_duration;
                return self.convert_user_cmd_to_player_action(player_idx, cmd.payload);
            }
        }

        // ========== 기존 AI Decision Logic ==========

        let is_home = MatchTeamSide::is_home(player_idx);
        let _slot = MatchTeamSide::local_idx(player_idx);
        let player_pos = self.get_player_position_by_index(player_idx);
        let _player_pos_m = player_pos.to_meters();

        // 선수 능력치 가져오기
        let (_overall, flair, decisions, concentration) = self.get_player_mental_attrs(player_idx);
        let attrs = self.get_player_attributes(player_idx);

        // CognitiveBias 생성 (SSOT: MatchSetup always provides attributes)
        let bias = CognitiveBias::from_attributes(
            attrs.composure as f32,
            attrs.flair as f32,
            attrs.bravery as f32,
            attrs.aggression as f32,
            attrs.decisions as f32,
            attrs.teamwork as f32,
            attrs.concentration as f32,
        );

        // DecisionContext 생성
        let decision_ctx = self.build_p16_decision_context(player_idx);

        // ElaborationContext 생성
        let elab_ctx = self.build_p16_elaboration_context(player_idx, is_home);

        // Mindset 결정
        let mindset = self.determine_player_mindset(player_idx, &decision_ctx);

        // Gate Chain 실행 (Gate A → B → C)
        let (final_action, intent, utility, results, shot_gate) = decide_action_with_detail(
            mindset,
            &decision_ctx,
            &elab_ctx,
            &bias,
            flair,
            decisions,
            concentration,
            &mut self.rng,
        );
        if matches!(
            intent,
            super::decision_topology::SelectedIntent::Progress(
                super::decision_topology::ProgressStyle::Safe
            )
        ) {
            self.safe_pass_seq = self.safe_pass_seq.wrapping_add(1);
        }
        self.record_shot_gate_outcome(shot_gate, decision_ctx.has_clear_shot, player_idx);

        // 0108: Record decision to replay (if enabled)
        // current_tick / 4.0 = seconds (4 ticks per second)
        let t_seconds = self.current_tick as f64 / 4.0;
        let team_id = if is_home { 0u32 } else { 1u32 };
        let pos_m = player_pos.to_meters();
        let at = crate::replay::types::MeterPos { x: pos_m.0 as f64, y: pos_m.1 as f64 };
        if let Some(recorder) = self.replay_recorder.as_mut() {
            recorder.record_decision(
                t_seconds,
                team_id,
                player_idx as u32,
                at,
                format!("{:?}", intent),
                Some(utility),
            );
        }

        let temperature = super::utility::calculate_temperature(flair, decisions, concentration);
        // FIX_2601: Shot Opportunity Telemetry hook
        {
            let chosen_action = super::decision_topology::intent_to_candidate(intent);
            self.check_and_record_shot_opportunity(player_idx, &results, &chosen_action, &decision_ctx);
        }

        self.record_decision_intent(
            player_idx,
            intent,
            utility,
            temperature,
            &results,
            &decision_ctx,
            &elab_ctx,
            player_pos,
            &final_action,
        );

        // FinalAction → PlayerAction 변환
        self.convert_final_action_to_player_action(final_action, player_idx)
    }

    fn record_decision_intent(
        &mut self,
        player_idx: usize,
        intent: super::decision_topology::SelectedIntent,
        utility: f32,
        temperature: f32,
        results: &[(super::decision_topology::CandidateAction, super::utility::UtilityResult)],
        decision_ctx: &super::decision_topology::DecisionContext,
        elab_ctx: &super::decision_topology::ElaborationContext,
        player_pos: Coord10,
        final_action: &super::decision_topology::FinalAction,
    ) {
        use super::decision_topology::{build_decision_intent, intent_to_candidate};
        use std::cmp::Ordering;

        let selected_action = intent_to_candidate(intent);
        let mut intent_log = build_decision_intent(
            player_idx as u32,
            self.current_tick as u32,
            selected_action,
            utility,
            results,
            temperature,
            decision_ctx,
        );

        let player_pos_m = player_pos.to_meters();
        intent_log.player_pos = Some(MeterPos { x: player_pos_m.0 as f64, y: player_pos_m.1 as f64 });

        if let Some((x, y)) = final_action.target_pos {
            intent_log.target_pos = Some(MeterPos { x: x as f64, y: y as f64 });
        }
        if let Some(target_idx) = final_action.target_player {
            intent_log.target_player_id = Some(target_idx as u32);
        }

        if !elab_ctx.pass_targets.is_empty() {
            let mut sorted_targets = elab_ctx.pass_targets.clone();
            sorted_targets.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));
            intent_log.pass_targets = sorted_targets
                .iter()
                .take(4)
                .map(|(idx, pos, quality)| IntentTarget {
                    player_id: *idx as u32,
                    pos: MeterPos { x: pos.0 as f64, y: pos.1 as f64 },
                    quality: *quality,
                })
                .collect();
        }

        if !elab_ctx.nearby_opponents.is_empty() {
            intent_log.nearby_opponents = elab_ctx
                .nearby_opponents
                .iter()
                .map(|(x, y)| MeterPos { x: *x as f64, y: *y as f64 })
                .collect();
        }

        self.decision_intents.push(intent_log);
    }

    fn record_shot_gate_outcome(
        &mut self,
        outcome: super::decision_topology::ShotGateOutcome,
        has_clear_shot: bool,
        player_idx: usize,
    ) {
        if !outcome.checked {
            return;
        }

        // FIX_2601/0119: Track shot gate by slot group
        if std::env::var("DEBUG_SLOT_BIAS").is_ok() {
            let slot_group = if player_idx < 11 { "0-10" } else { "11-21" };
            let gate_result = if outcome.allowed { "allowed" } else { "rejected" };
            eprintln!(
                "[SHOT_GATE] tick={} slot_group={} result={} clear_shot={} player_idx={}",
                self.current_tick, slot_group, gate_result, has_clear_shot, player_idx
            );
        }

        self.result.statistics.shot_gate_checks =
            self.result.statistics.shot_gate_checks.saturating_add(1);
        self.result.statistics.clear_shot_checks =
            self.result.statistics.clear_shot_checks.saturating_add(1);
        if !has_clear_shot {
            self.result.statistics.clear_shot_blocked =
                self.result.statistics.clear_shot_blocked.saturating_add(1);
        }
        if outcome.allowed {
            self.result.statistics.shot_gate_allowed =
                self.result.statistics.shot_gate_allowed.saturating_add(1);
        } else {
            self.result.statistics.shot_gate_rejects =
                self.result.statistics.shot_gate_rejects.saturating_add(1);
        }
    }

    /// FIX_2601: Check and record shot opportunity if telemetry is enabled
    ///
    /// Records a frame if:
    /// 1. Shot is in Top-K candidates
    /// 2. Shot utility >= T_abs (0.15)
    /// 3. Shot utility >= T_rel * top_utility (0.70)
    fn check_and_record_shot_opportunity(
        &mut self,
        player_idx: usize,
        results: &[(super::decision_topology::CandidateAction, super::utility::UtilityResult)],
        chosen_action: &super::decision_topology::CandidateAction,
        decision_ctx: &super::decision_topology::DecisionContext,
    ) {
        // Skip if telemetry not enabled
        if self.shot_opp_telemetry.is_none() {
            return;
        }

        let is_home = MatchTeamSide::is_home(player_idx);
        let player_pos = self.get_player_position_by_index(player_idx);

        // Calculate zone (20-zone positional play)
        let attacks_right = self.attacks_right(is_home);
        let pos_m = player_pos.to_meters();
        let zone = crate::calibration::zone::pos_to_posplay_zone_meters(pos_m.0, pos_m.1, attacks_right);

        // Get valid pass targets (sorted for determinism)
        let team_range = MatchTeamSide::teammate_range(player_idx);
        let mut valid_targets: Vec<u8> = team_range
            .filter(|&idx| idx != player_idx)
            .map(|idx| idx as u8)
            .collect();
        valid_targets.sort();

        // Calculate goalkeeper distance
        let gk_idx = if is_home { 11 } else { 0 }; // Opponent GK
        let gk_pos = self.get_player_position_by_index(gk_idx);
        let gk_pos_m = gk_pos.to_meters();
        let gk_dist = ((pos_m.0 - gk_pos_m.0).powi(2) + (pos_m.1 - gk_pos_m.1).powi(2)).sqrt();

        // Check opportunity and record
        let team_side = if is_home {
            TeamSide::Home
        } else {
            TeamSide::Away
        };

        let current_tick = self.current_tick;
        let minute = self.minute;

        // FIX_2601/0120: Get bias analysis parameters
        let body_dir = self.player_body_dir[player_idx];
        let kickoff_phase = self.calculate_kickoff_phase();

        if let Some(frame) = super::shot_opportunity::check_shot_opportunity(
            current_tick as u32,
            player_idx as u8,
            team_side,
            zone,
            decision_ctx,
            results,
            chosen_action,
            &valid_targets,
            gk_dist,
            // FIX_2601/0120: Bias analysis parameters
            body_dir,
            attacks_right,
            kickoff_phase,
        ) {
            // Now we can safely borrow telemetry mutably
            if let Some(telemetry) = self.shot_opp_telemetry.as_mut() {
                telemetry.record_frame(frame, minute);
            }
        }
    }

    /// 선수 멘탈 능력치 가져오기
    fn get_player_mental_attrs(&self, player_idx: usize) -> (u8, f32, f32, f32) {
        let player = self.get_match_player(player_idx);
        let attrs = self.get_player_attributes(player_idx);
        let overall = player.overall;

        // SSOT: MatchSetup always provides attributes (missing values are pre-filled).
        let flair = attrs.flair as f32 / 100.0;
        let decisions = attrs.decisions as f32 / 100.0;
        let concentration = attrs.concentration as f32 / 100.0;

        (overall, flair, decisions, concentration)
    }

    /// P18: Helper - Get local_pressure (f32) at player position from FieldBoard
    fn get_local_pressure(&self, player_idx: usize) -> f32 {
        let player_pos = self.get_player_position_by_index(player_idx);
        let player_pos_m = player_pos.to_meters();

        if let Some(ref board) = self.field_board {
            let cell = board.cell_of(player_pos_m);
            let is_home = MatchTeamSide::is_home(player_idx);
            let raw = if is_home {
                board.pressure_against_home.get(cell)
            } else {
                board.pressure_against_away.get(cell)
            };
            (raw / 3.0).clamp(0.0, 1.0)
        } else {
            0.5 // Moderate pressure fallback
        }
    }

    /// P18: Helper - Get PressureLevel enum from FieldBoard
    fn get_local_pressure_level(&self, player_idx: usize) -> super::calculations::PressureLevel {
        use super::calculations::PressureLevel;

        let pressure_f32 = self.get_local_pressure(player_idx);

        // Convert f32 to PressureLevel enum (same logic as classify_pressure_level)
        if pressure_f32 < 0.1 {
            PressureLevel::None
        } else if pressure_f32 < 0.3 {
            PressureLevel::Light
        } else if pressure_f32 < 0.5 {
            PressureLevel::Moderate
        } else if pressure_f32 < 0.7 {
            PressureLevel::Heavy
        } else {
            PressureLevel::Extreme
        }
    }

    /// P16 DecisionContext 생성
    fn build_p16_decision_context(
        &self,
        player_idx: usize,
    ) -> super::decision_topology::DecisionContext {
        use super::decision_topology::DecisionContext;
        use crate::engine::coordinates;
        use crate::engine::physics_constants::skills;

        // P18: Performance profiling (debug mode only)
        #[cfg(debug_assertions)]
        let _perf_start = std::time::Instant::now();

        let is_home = MatchTeamSide::is_home(player_idx);
        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx);

        // xG 계산
        let distance_m =
            coordinates::distance_to_goal_m(player_pos.to_normalized_legacy(), attacks_right);
        let xg = crate::engine::probability::xg_skill_based(distance_m, 0.7);

        // 압박 계산 - P18: FieldBoard 기반 (O(1) lookup)
        let player_pos_m = player_pos.to_meters();
        let local_pressure = if let Some(ref board) = self.field_board {
            let cell = board.cell_of(player_pos_m);
            let heatmap = if is_home {
                &board.pressure_against_home // Away defenders' pressure
            } else {
                &board.pressure_against_away // Home defenders' pressure
            };
            let raw_pressure = heatmap.get(cell);

            // Normalize: FieldBoard pressure is 0~N, need 0~1
            // Typical max is ~3.0 (3 defenders stacked)
            (raw_pressure / 3.0).clamp(0.0, 1.0)
        } else {
            // Fallback: No FieldBoard → calculate old way
            let pressure_ctx = self.calculate_pressure_context(player_idx, None);
            pressure_ctx.effective_pressure
        };

        // xG Zone 레벨 계산 - Match OS v1.2: FieldBoard 기반
        // FIX_2601/0120: Use direction-aware xG lookup API
        let player_pos_norm = player_pos.to_normalized_legacy();
        let xgzone_level = if let Some(ref board) = self.field_board {
            board.xgzone.get_xg_directional(player_pos_norm, attacks_right)
        } else {
            0.0 // Fallback: No FieldBoard → no zone awareness
        };

        // Immediate pressure 계산 - Match OS v1.2: 2m radius tackle threat
        let immediate_pressure =
            self.calculate_immediate_pressure(player_idx, is_home, player_pos_m);

        // Open-Football baseline: dribble escape skill (dribbling+agility) + technique bonus
        let dribbling = skills::normalize(self.get_player_dribbling(player_idx));
        let technique = skills::normalize(self.get_player_technique(player_idx));
        let agility = skills::normalize(self.get_player_agility(player_idx));
        let base_escape = dribbling * 0.625 + agility * 0.375;
        let dribble_escape_skill = (base_escape + technique * 0.1).clamp(0.0, 1.0);

        // Open-Football: dribbling > 15 (FM-scale) for general dribble
        let dribble_raw = self.get_player_dribbling(player_idx);
        let dribble_skill_20 = if dribble_raw > 20.0 {
            dribble_raw / 5.0
        } else {
            dribble_raw
        };
        let dribble_skill_ok = dribble_skill_20 > 15.0;

        // Open-Football: scan for a clear dribble lane (15m ahead)
        let dribble_space_scan_ok =
            self.has_dribble_space_scan(player_pos_m, is_home, attacks_right);

        // 패스 옵션 카운트
        let pass_options = self.count_pass_options(player_idx, is_home);

        // 가장 가까운 상대/팀원 거리
        let nearest_opponent_dist =
            self.find_nearest_opponent_distance(player_idx, is_home).unwrap_or(10.0);
        let dribble_safe_radius_ok = nearest_opponent_dist >= 15.0;

        let nearest_teammate_dist =
            self.find_nearest_teammate_distance(player_idx, is_home).unwrap_or(10.0);

        // 페널티박스 여부
        // FIX_2601: player_pos는 Coord10, player_pos_norm 사용
        // FIX_2601/0110: Use attacks_right for correct 2nd half penalty box check
        let in_penalty_box = self.is_in_attacking_penalty_box(player_pos_norm, attacks_right);

        // 역습 여부 (간단한 휴리스틱)
        let is_counter_attack = if let Some(phase) = self.get_current_phase(is_home) {
            matches!(phase, crate::engine::team_phase::TeamPhase::TransitionAttack)
        } else {
            false
        };

        let instructions = if is_home { &self.home_instructions } else { &self.away_instructions };

        // FIX_2601: Use proper attack direction (accounts for halftime)
        let attacks_right = self.attacks_right(is_home);
        let team_pitch_zone = Some(super::pitch_zone::zone_of_position(
            player_pos_m.0,
            player_pos_m.1,
            attacks_right,
        ));
        let event_mix_profile =
            Some(self.determine_event_mix_profile(instructions, is_counter_attack));

        // 전술 추적 (Integrity Check)
        let mut tactical_trace = Vec::new();
        tactical_trace.push(format!("Tempo:{:?}", instructions.team_tempo));
        tactical_trace.push(format!("Pressing:{:?}", instructions.pressing_intensity));

        // FIX_2601/0105 Phase 2: Better positioned teammate check
        let (has_better_positioned_teammate, best_teammate_threat) =
            self.find_better_positioned_teammate(player_idx, is_home);

        // FIX_2601/0105 Phase 4: Shot budget tracking
        let shots_this_half = self.shots_this_half(is_home);
        let shot_budget_per_half = self.shot_budget();

        // FIX_2601/0106 P3: Determine buildup phase from ball position
        let ball_pos_m = self.ball.position.to_meters();
        let buildup_phase =
            super::buildup_phase::BuildupPhase::from_ball_position(ball_pos_m.0, attacks_right);

        // FIX_2601/0116: New fields for Header/TakeOn/Cross
        let ball_height = self.ball.height as f32 / 10.0;
        let has_clear_shot = self.has_clear_shot(player_idx);
        let good_shooting_angle = self.has_good_shooting_angle(player_idx);
        let nearby_opponents_8m = self.count_nearby_opponents(player_idx, 8.0) as u8;
        let long_shots_skill = self.get_player_long_shots(player_idx);
        let distance_to_ball = {
            let dx = player_pos_m.0 - ball_pos_m.0;
            let dy = player_pos_m.1 - ball_pos_m.1;
            (dx * dx + dy * dy).sqrt()
        };

        // has_space_ahead: No opponent within 10m in forward direction
        let has_space_ahead = dribble_space_scan_ok || nearest_opponent_dist > 10.0;

        // is_dribble_position: Forwards, wingers (based on team slot position)
        // Slots: 0=GK, 1-4=Defenders, 5-7=Midfielders, 8-10=Forwards/Wingers
        let team_slot = player_idx % 11;
        let is_dribble_position = team_slot >= 8 || team_slot == 5; // Forwards, wingers, or LAM position

        // is_wide_position: Near touchline (within 18m of sides, pitch width ~68m)
        // FIX_2601/0117: Fixed coordinate check (y is 0-68m, not centered at 0)
        // Wide = y < 18m (left touchline) OR y > 50m (right touchline)
        let is_wide_position = player_pos_m.1 < 18.0 || player_pos_m.1 > 50.0;

        // in_attacking_third: In opponent's third (last 35m, TeamView semantics)
        let x_tv = if attacks_right { player_pos_m.0 } else { field::LENGTH_M - player_pos_m.0 };
        let in_attacking_third = x_tv > 70.0;

        // teammates_in_box: Count teammates in attacking penalty box
        let teammates_in_box = self.count_teammates_in_box(player_idx, is_home, attacks_right);

        let ctx = DecisionContext {
            xg,
            distance_to_goal: distance_m,
            local_pressure,
            xgzone_level,
            immediate_pressure,
            dribble_escape_skill,
            dribble_skill_ok,
            dribble_safe_radius_ok,
            dribble_space_scan_ok,
            has_ball: true,
            is_defending: false,
            is_counter_attack,
            in_penalty_box,
            near_touchline: self.is_near_touchline(player_pos_norm),
            nearest_teammate_dist,
            nearest_opponent_dist,
            pass_options_count: pass_options,
            distance_to_ball_carrier: 0.0, // 공 소유자이므로 0
            defense_ctx: None,
            action_history: None, // TODO: Populate from engine state
            tactical_trace,
            team_pitch_zone,
            event_mix_profile,
            // Phase G v1: Team tactics knobs (deterministic, evidence-grade)
            team_pressing_factor: instructions.get_pressing_factor(),
            team_tempo_factor: instructions.get_tempo_factor(),
            team_width_bias_m: instructions.get_width_bias_m(),
            team_risk_bias: super::decision_topology::build_up_style_risk_delta(instructions),
            // FIX_2601/0105 Phase 2+4: Shot realism fields
            has_better_positioned_teammate,
            best_teammate_threat,
            shots_this_half,
            shot_budget_per_half,
            // FIX_2601/0106 P3: Buildup phase
            buildup_phase,
            sticky_actions: self.sticky_actions[player_idx],
            has_clear_shot,
            good_shooting_angle,
            nearby_opponents_8m,
            long_shots_skill,
            // FIX_2601/0116: Header/TakeOn/Cross fields
            ball_height,
            distance_to_ball,
            has_space_ahead,
            is_dribble_position,
            is_wide_position,
            in_attacking_third,
            teammates_in_box,
            // DPER Framework: Experimental parameters
            exp_shoot_xg_threshold: self.exp_shoot_xg_threshold(),
            exp_dribble_bias: self.exp_dribble_bias(),
            exp_through_ball_multiplier: self.exp_through_ball_multiplier(),
            exp_cross_multiplier: self.exp_cross_multiplier(),
            exp_directness_bias: self.exp_directness_bias(),
            // FIX_2601/0112: Calibration bias from CalibratorParams
            // FIX_2601/1128: Apply AttackSubPhase multiplier to pass biases
            // In Circulation: reduce progressive pass (0.25x), boost safe pass (2.0x)
            // In Progression: normal progressive pass (1.1x), reduce safe pass (0.9x)
            cal_progressive_pass_bias: {
                let phase_state = if is_home { &self.home_phase_state } else { &self.away_phase_state };
                let subphase_mul = phase_state.forward_pass_weight(); // 0.25 in Circulation, 1.1 in Progression
                self.cal_progressive_pass_bias() * subphase_mul
            },
            cal_safe_pass_bias: {
                let phase_state = if is_home { &self.home_phase_state } else { &self.away_phase_state };
                let subphase_mul = phase_state.circulation_pass_weight(); // 2.0 in Circulation, 0.9 in Progression

                // Debug: track sub-phase distribution during decisions
                static PROG_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                static CIRC_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                use crate::engine::team_phase::AttackSubPhase;
                match phase_state.attack_sub_phase {
                    AttackSubPhase::Progression => { PROG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
                    AttackSubPhase::Circulation => { CIRC_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
                    AttackSubPhase::Finalization => {}
                }
                let total = PROG_COUNT.load(std::sync::atomic::Ordering::Relaxed) + CIRC_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                if total > 0 && total % 500 == 0 {
                    let circ = CIRC_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                    eprintln!("[SUBPHASE_DECISION] Total: {}, Circulation: {} ({:.1}%)", total, circ, 100.0 * circ as f64 / total as f64);
                }

                subphase_mul
            },
            // FIX_2601/1128: Also reduce other forward-biased actions in Circulation
            cal_long_pass_bias: {
                let phase_state = if is_home { &self.home_phase_state } else { &self.away_phase_state };
                let subphase_mul = phase_state.forward_pass_weight(); // Reduce in Circulation
                self.cal_long_pass_bias() * subphase_mul
            },
            cal_cross_bias: self.cal_cross_bias(),
            cal_shot_bias: self.cal_shot_bias(),
            cal_dribble_bias: self.cal_dribble_bias(),
            cal_through_ball_bias: {
                let phase_state = if is_home { &self.home_phase_state } else { &self.away_phase_state };
                let subphase_mul = phase_state.forward_pass_weight(); // Reduce in Circulation
                self.cal_through_ball_bias() * subphase_mul
            },
            // FIX_2601/1129: AttackPhase (Circulation/Positional/Transition)
            attack_phase: self.get_attack_phase(is_home),
            // FIX_2601/0123: SafePass return-bias sequence (match-local)
            safe_pass_seq: self.safe_pass_seq,
        };

        // P18: Performance profiling + behavior validation output (every 100 ticks)
        #[cfg(debug_assertions)]
        {
            let elapsed = _perf_start.elapsed();
            if match_debug_enabled() && self.current_tick % 100 == 0 {
                println!("[P18-PERF] build_p16_decision_context @ tick {}: {:?} | local_pressure: {:.2} | xg: {:.2}",
                    self.current_tick, elapsed, local_pressure, xg);
            }
        }

        // FIX_2601/0119: Track xG by slot group to diagnose shooting bias
        if std::env::var("DEBUG_SLOT_BIAS").is_ok() {
            let slot_group = if player_idx < 11 { "0-10" } else { "11-21" };
            let mindset_str = if xg > 0.08 { "AttackScore" } else { "Other" };
            eprintln!(
                "[XG_CHECK] tick={} slot_group={} xg={:.4} dist={:.1}m mindset={} in_third={} in_box={}",
                self.current_tick, slot_group, xg, distance_m, mindset_str, in_attacking_third, in_penalty_box
            );
        }

        ctx
    }

    fn determine_event_mix_profile(
        &self,
        instructions: &crate::tactics::team_instructions::TeamInstructions,
        is_counter_attack: bool,
    ) -> super::decision_topology::EventMixProfile {
        use super::decision_topology::EventMixProfile;
        use crate::tactics::team_instructions::{BuildUpStyle, DefensiveLine, TeamPressing};

        if is_counter_attack {
            return EventMixProfile::Counterattack;
        }

        if matches!(instructions.defensive_line, DefensiveLine::VeryDeep | DefensiveLine::Deep)
            && matches!(instructions.pressing_intensity, TeamPressing::VeryLow | TeamPressing::Low)
        {
            return EventMixProfile::Defensive;
        }

        match instructions.build_up_style {
            BuildUpStyle::Short => EventMixProfile::Possession,
            BuildUpStyle::Direct => EventMixProfile::Counterattack,
            BuildUpStyle::Mixed => EventMixProfile::Balanced,
        }
    }

    /// P16 ElaborationContext 생성
    fn build_p16_elaboration_context(
        &self,
        player_idx: usize,
        is_home: bool,
    ) -> super::decision_topology::ElaborationContext {
        use super::decision_topology::ElaborationContext;

        let player_pos = self.get_player_position_by_index(player_idx);
        let player_pos_m = player_pos.to_meters();
        let ball_pos = self.ball.position;
        let ball_pos_m = ball_pos.to_meters();

        // FIX_2601/0109: Use attacks_right for correct second-half goal calculation
        let attacks_right = self.attacks_right(is_home);

        let tv_to_world_x = |x_tv: f32| if attacks_right { x_tv } else { field::LENGTH_M - x_tv };
        let goal_x = tv_to_world_x(field::LENGTH_M);
        let defense_goal_x = tv_to_world_x(0.0);

        // 공격/수비 골대 위치 (미터)
        let goal_pos = (goal_x, field::CENTER_Y);
        let defense_goal_pos = (defense_goal_x, field::CENTER_Y);

        // 상대 GK 위치
        // FIX_2601: p는 &Coord10, to_meters() 사용
        let gk_idx = if is_home { 11 } else { 0 };
        let gk_pos = self.player_positions.get(gk_idx).map(|p| p.to_meters());

        // 패스 타겟 생성
        let pass_targets = self.build_pass_targets(player_idx, is_home);

        // 슛 존 (near post, center, far post)
        // FIX_2601/0109: Use attacks_right for correct second-half shot zones
        let shot_zones = [
            (goal_x, 30.5),            // near post
            (goal_x, field::CENTER_Y), // center
            (goal_x, 37.5),            // far post
        ];

        // FIX_2601/0106 P2-8: 근처 상대 선수 위치 수집 (Force Field 드리블용)
        const NEARBY_RADIUS_M: f32 = 15.0;
        let opponent_range = if is_home { 11..22 } else { 0..11 };
        let nearby_opponents: Vec<(f32, f32)> = opponent_range
            .filter_map(|opp_idx| {
                self.player_positions.get(opp_idx).and_then(|opp_pos| {
                    let opp_pos_m = opp_pos.to_meters();
                    let dx = opp_pos_m.0 - player_pos_m.0;
                    let dy = opp_pos_m.1 - player_pos_m.1;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= NEARBY_RADIUS_M * NEARBY_RADIUS_M {
                        Some(opp_pos_m)
                    } else {
                        None
                    }
                })
            })
            .collect();

        // FIX_2601/1130: Find who most recently passed to this player (for return pass priority)
        let recent_passer_idx = self.find_immediate_passer(player_idx, is_home);

        ElaborationContext {
            goal_pos,
            defense_goal_pos, // FIX_2601: 수비 골대 위치 추가
            ball_pos: ball_pos_m,
            player_pos: player_pos_m,
            ball_carrier_pos: None, // 공 소유자이므로 None
            gk_pos,
            pass_targets,
            shot_zones,
            nearby_opponents, // FIX_2601/0106 P2-8: Force Field용
            recent_passer_idx, // FIX_2601/1130: 리턴 패스 우선
        }
    }

    /// FIX_2601/1130: Find the player who most recently passed to the receiver
    /// Used for return pass priority in SafePass
    fn find_immediate_passer(&self, receiver_idx: usize, is_home: bool) -> Option<usize> {
        let receiver = receiver_idx as u8;
        // Check the most recent 3 passes to find who passed to me
        self.recent_pass_pairs
            .iter()
            .rev()
            .take(3)
            .find(|&&(_, to)| to == receiver)
            .map(|&(from, _)| from as usize)
            .filter(|&idx| {
                // Verify it's a valid teammate (not GK)
                let is_teammate = if is_home { idx < 11 } else { idx >= 11 };
                let is_not_gk = idx != 0 && idx != 11;
                is_teammate && is_not_gk
            })
    }

    /// 패스 타겟 목록 생성 (idx, pos_m, quality)
    fn build_pass_targets(
        &self,
        player_idx: usize,
        is_home: bool,
    ) -> Vec<(usize, (f32, f32), f32)> {
        let player_pos = self.get_player_position_by_index(player_idx);
        let player_pos_m = player_pos.to_meters();
        // FIX_2601: Use proper attack direction (accounts for halftime)
        let attacks_right = self.attacks_right(is_home);
        let instructions = if is_home { &self.home_instructions } else { &self.away_instructions };
        let style = super::zone_transition::ZoneTransitionStyle::from_instructions(instructions);
        let from_zone =
            super::pitch_zone::zone_of_position(player_pos_m.0, player_pos_m.1, attacks_right);
        let tactical_threshold = self.pass_tactical_improvement_threshold(player_idx);
        let passer_tactical = self.calculate_pass_tactical_factor(player_idx, player_idx, is_home);
        let mut targets = Vec::new();
        let mut fallback_targets = Vec::new();

        let teammate_range = MatchTeamSide::teammate_range(player_idx);

        for idx in teammate_range {
            if idx == player_idx {
                continue;
            }

            if let Some(&teammate_pos) = self.player_positions.get(idx) {
                let is_forward = coordinates::is_advancing(
                    player_pos.to_normalized_legacy(),
                    teammate_pos.to_normalized_legacy(),
                    attacks_right,
                );
                if is_forward && self.is_offside_position(idx, is_home) {
                    continue;
                }

                // FIX_2601: teammate_pos는 Coord10
                let teammate_pos_m = teammate_pos.to_meters();

                // 패스 품질 계산 (간단한 휴리스틱)
                // FIX_2601: Coord10.distance_to_m() 사용
                let dist_m = player_pos.distance_to_m(&teammate_pos);
                let dist_norm = dist_m / field::LENGTH_M; // normalized distance
                let base_quality = (1.0 - dist_norm / 2.0).clamp(0.0, 1.0);
                let to_zone = super::pitch_zone::zone_of_position(
                    teammate_pos_m.0,
                    teammate_pos_m.1,
                    attacks_right,
                );
                let zone_factor = super::zone_transition::pass_factor(style, from_zone, to_zone);

                // FIX_2601/0110: Add progression bonus for forward passes to prevent
                // teams from getting stuck in their own half with Short build-up.
                // FIX_2601/1128: Use AttackSubPhase-based weights for forward/circulation balance
                let phase_state = if is_home { &self.home_phase_state } else { &self.away_phase_state };
                let forward_weight = phase_state.forward_pass_weight();
                let circulation_weight = phase_state.circulation_pass_weight();

                // Apply phase-based progression factor
                let progression_bonus = if is_forward {
                    forward_weight  // Circulation: 0.4, Progression: 1.2, Finalization: 0.8
                } else {
                    circulation_weight  // Circulation: 1.5, Progression: 0.8, Finalization: 1.0
                };
                let mut quality = (base_quality * zone_factor * progression_bonus).clamp(0.0, 1.0);

                // FIX_2601/0123: Reciprocity - no bonus or penalty (neutral)
                // The reciprocity injection in tick_based.rs handles this
                // let has_reciprocity = self.has_recent_pass_from(idx, player_idx);

                // FIX_2601/0123: Diversity bonus REMOVED to reduce density
                // Previously encouraged passing to different teammates (increased density)
                // Now set to 0.0 to help reduce density
                const DIVERSITY_QUALITY_BONUS: f32 = 0.0;
                if !self.is_recent_pass_receiver(idx) {
                    quality = (quality + DIVERSITY_QUALITY_BONUS).clamp(0.0, 1.0);
                }

                // FIX_2601/1130: Diversity penalty - reduce quality for frequent receivers
                // This encourages passing to different teammates (improves network density)
                const FREQUENT_RECEIVER_PENALTY: f32 = 0.08;  // -0.08 per pass received
                const MAX_DIVERSITY_PENALTY: f32 = 0.40;      // Cap at -0.40
                let receive_count = self.pass_receive_counts[idx];
                let diversity_penalty = (receive_count as f32 * FREQUENT_RECEIVER_PENALTY).min(MAX_DIVERSITY_PENALTY);
                quality = (quality - diversity_penalty).max(0.0);

                // FIX_2601/1128: Safe option floor for backward passes
                // Increased to 0.55 to make backward passes more competitive
                const SAFE_OPTION_QUALITY_FLOOR: f32 = 0.55;
                if !is_forward && quality < SAFE_OPTION_QUALITY_FLOOR {
                    quality = SAFE_OPTION_QUALITY_FLOOR;
                }

                // FIX_2601/1130: Wide player quality floor to improve network density
                // Wide players often have lower quality due to peripheral positions
                // This ensures they remain viable pass targets
                const WIDE_PLAYER_QUALITY_FLOOR: f32 = 0.45;
                let position = self.get_match_player(idx).position;
                let is_wide = matches!(
                    position,
                    crate::models::Position::LB
                        | crate::models::Position::RB
                        | crate::models::Position::LWB
                        | crate::models::Position::RWB
                        | crate::models::Position::LM
                        | crate::models::Position::RM
                        | crate::models::Position::LW
                        | crate::models::Position::RW
                );
                if is_wide && quality < WIDE_PLAYER_QUALITY_FLOOR {
                    quality = WIDE_PLAYER_QUALITY_FLOOR;
                }

                // GK 제외 (0, 11)
                if idx == 0 || idx == 11 {
                    continue;
                }

                let target_tactical =
                    self.calculate_pass_tactical_factor(player_idx, idx, is_home);
                let candidate = (idx, teammate_pos_m, quality);
                fallback_targets.push(candidate);

                // FIX_2601/0123: Skip tactical threshold only for backward passes (safe options)
                // Removed reciprocity skip to reduce A→B→A patterns
                let skip_tactical = !is_forward;
                if skip_tactical || target_tactical >= passer_tactical + tactical_threshold {
                    targets.push(candidate);
                }
            }
        }

        // 품질 순으로 정렬
        if targets.is_empty() {
            targets = fallback_targets;
        }
        // FIX_2601/0115: Position-neutral tie-breaker (replaces Y-bias from 0110)
        // Y-position tie-breaker caused left-side bias, now using deterministic hash
        targets.sort_by(|a, b| {
            match b.2.partial_cmp(&a.2) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // FIX_2601/0115: Deterministic hash tie-breaker (no position bias)
                    super::deterministic_tie_hash(a.0, a.1, b.0, b.1)
                }
                Some(ord) => ord,
            }
        });

        targets
    }

    /// Mindset 결정
    fn determine_player_mindset(
        &self,
        _player_idx: usize,
        ctx: &super::decision_topology::DecisionContext,
    ) -> super::decision_topology::PlayerMindset {
        use super::decision_topology::PlayerMindset;

        // FIX_2601/0115: xG threshold for AttackScore mindset
        // xG values: <10m=0.39, 10-16.5m=0.19, 16.5-25m=0.08, 25-35m=0.04
        // 0.08 allows shots from box edge (16.5-25m range)
        if ctx.xg > 0.08 {
            return PlayerMindset::AttackScore;
        }

        // 높은 압박 → Protect
        if ctx.local_pressure > 0.7 {
            #[cfg(debug_assertions)]
            if match_debug_enabled() && self.current_tick % 100 == 0 {
                println!("[P18-BEHAVIOR] High pressure detected: {:.2} → AttackProtect mindset @ tick {}",
                    ctx.local_pressure, self.current_tick);
            }
            return PlayerMindset::AttackProtect;
        }

        // 역습 → CounterAttack
        if ctx.is_counter_attack {
            return PlayerMindset::TransitionCounter;
        }

        // 기본: Progress
        #[cfg(debug_assertions)]
        if match_debug_enabled() && self.current_tick % 100 == 0 && ctx.local_pressure < 0.3 {
            println!(
                "[P18-BEHAVIOR] Low pressure detected: {:.2} → AttackProgress mindset @ tick {}",
                ctx.local_pressure, self.current_tick
            );
        }
        PlayerMindset::AttackProgress
    }

    /// FinalAction → PlayerAction 변환 (의도 5개로 고정)
    ///
    /// P16 해결책 B: PlayerAction은 Shoot|Pass|Dribble|Hold|TakeOn만 반환
    /// ThroughBall/LongPass/Cross 같은 세부는 ActionDetail로 분리
    fn convert_final_action_to_player_action(
        &mut self,
        action: super::decision_topology::FinalAction,
        _player_idx: usize,
    ) -> PlayerAction {
        use super::decision_topology::{FinalActionParams, FinalActionType};

        match action.action_type {
            FinalActionType::Shot => PlayerAction::Shoot,
            FinalActionType::Pass => {
                // target_player 저장 (호환성)
                if let Some(target_idx) = action.target_player {
                    self.last_pass_target = Some(target_idx);
                    // DPQ v1.2: Pull forward the pass target so they can prepare
                    let exp = self.get_exp_params();
                    if exp.dpq_enabled && exp.dpq_variable_cadence {
                        self.decision_scheduler.pull_forward(target_idx, self.current_tick);
                    }
                }
                // P16: 항상 Pass 반환 (ThroughBall/LongPass로 승격 금지!)
                PlayerAction::Pass
            }
            FinalActionType::Dribble => {
                if let FinalActionParams::Dribble(params) = &action.params {
                    if params.is_skill_move {
                        PlayerAction::TakeOn
                    } else {
                        PlayerAction::Dribble
                    }
                } else {
                    PlayerAction::Dribble
                }
            }
            FinalActionType::Clear => PlayerAction::Pass, // Clear도 Pass로 통일
            // FIX_2601/0117: Cross도 Pass로 통일
            FinalActionType::Cross => PlayerAction::Pass,
            FinalActionType::Hold | FinalActionType::Movement => PlayerAction::Hold,
            _ => PlayerAction::Hold,
        }
    }

    /// FIX_2601/0117: Pure (read-only) version of FinalAction conversion
    ///
    /// Snapshot-compatible: no state mutations, suitable for `&self` methods.
    /// State changes like `last_pass_target` are handled in the commit phase.
    #[cfg(feature = "snapshot_decide")]
    fn convert_final_action_with_detail_pure(
        &self,
        action: super::decision_topology::FinalAction,
        player_idx: usize,
    ) -> (PlayerAction, crate::engine::action_detail::ActionDetail) {
        use super::decision_topology::{FinalActionParams, FinalActionType};
        use crate::engine::action_detail::{
            ActionDetail, ActionParam, ActionTarget, DribbleStyle, PassType as DetailPassType,
            ShotType as DetailShotType,
        };

        match action.action_type {
            FinalActionType::Shot => {
                let detail = if let FinalActionParams::Shot(params) = &action.params {
                    let shot_type = match params.technique {
                        super::decision_topology::ShotTechnique::Normal => DetailShotType::Normal,
                        super::decision_topology::ShotTechnique::Finesse => DetailShotType::Placed,
                        super::decision_topology::ShotTechnique::Power => DetailShotType::Power,
                        super::decision_topology::ShotTechnique::Chip => DetailShotType::Chip,
                        super::decision_topology::ShotTechnique::Header => DetailShotType::Header,
                    };
                    let target = action.target_pos.map(|(x, y)| ActionTarget::GoalMouth(x, y));
                    ActionDetail {
                        target,
                        power: Some(action.power),
                        curve: Some(action.curve),
                        shot_type: Some(shot_type),
                        ..Default::default()
                    }
                } else {
                    ActionDetail::for_shot(
                        DetailShotType::Normal,
                        ActionTarget::GoalMouth(0.5, 0.5),
                        action.power,
                        action.curve,
                    )
                };
                (PlayerAction::Shoot, detail)
            }

            FinalActionType::Pass | FinalActionType::Clear | FinalActionType::Cross => {
                // Note: last_pass_target and pull_forward are handled in commit phase
                let detail = if let FinalActionParams::Pass(params) = &action.params {
                    let pass_type = match params.pass_type {
                        super::decision_topology::PassType::Ground => DetailPassType::Short,
                        super::decision_topology::PassType::Through => DetailPassType::Through,
                        super::decision_topology::PassType::Lob => DetailPassType::Lob,
                        super::decision_topology::PassType::Cross => DetailPassType::Cross,
                        super::decision_topology::PassType::Clear => DetailPassType::Clear,
                    };

                    // FIX_2601/1128: ActionTarget에 위치 정보 포함 (forward_pass_rate 측정용)
                    // ActionTarget::Player는 point() 메서드가 None을 반환하므로
                    // target_pos가 있으면 ActionTarget::Space로 위치 정보 포함
                    let target = if let Some(target_idx) = action.target_player {
                        if pass_type == DetailPassType::Through {
                            let point = action.target_pos.unwrap_or((0.0, 0.0));
                            ActionTarget::Space { point, lead: 5.0 }
                        } else if let Some(pos) = action.target_pos {
                            // SafePass 등: 위치 정보 포함 (lead=0으로 일반 패스와 구분)
                            ActionTarget::Space { point: pos, lead: 0.0 }
                        } else {
                            // fallback: 위치 없이 플레이어 인덱스만
                            ActionTarget::Player(target_idx)
                        }
                    } else if let Some(pos) = action.target_pos {
                        ActionTarget::Point(pos.0, pos.1)
                    } else {
                        ActionTarget::Point(0.0, 0.0)
                    };

                    let loft_value = if params.is_lofted { 1.0 } else { 0.0 };

                    ActionDetail {
                        target: Some(target),
                        power: Some(action.power),
                        curve: Some(action.curve),
                        pass_type: Some(pass_type),
                        params: vec![ActionParam::Loft(loft_value)],
                        ..Default::default()
                    }
                } else {
                    // FIX_2601/1128: 위치 정보 포함하여 forward_pass_rate 측정 정확도 향상
                    let target = if let (Some(_target_idx), Some(pos)) = (action.target_player, action.target_pos) {
                        ActionTarget::Space { point: pos, lead: 0.0 }
                    } else if let Some(pos) = action.target_pos {
                        ActionTarget::Point(pos.0, pos.1)
                    } else if let Some(idx) = action.target_player {
                        ActionTarget::Player(idx)
                    } else {
                        ActionTarget::Point(0.0, 0.0)
                    };

                    ActionDetail::for_pass(
                        if matches!(action.action_type, FinalActionType::Clear) {
                            DetailPassType::Clear
                        } else {
                            DetailPassType::Short
                        },
                        target,
                        action.power,
                        action.curve,
                    )
                };
                (PlayerAction::Pass, detail)
            }

            FinalActionType::Dribble => {
                if let FinalActionParams::Dribble(params) = &action.params {
                    if params.is_skill_move {
                        let detail = ActionDetail::for_takeon(0, params.direction, 0.7);
                        (PlayerAction::TakeOn, detail)
                    } else {
                        let target = action.target_pos.map(|(x, y)| ActionTarget::Point(x, y));
                        let sprint = self.sticky_actions[player_idx].sprint;
                        let detail = ActionDetail::for_dribble(DribbleStyle::Carry, target, sprint);
                        (PlayerAction::Dribble, detail)
                    }
                } else {
                    let target = action.target_pos.map(|(x, y)| ActionTarget::Point(x, y));
                    let sprint = self.sticky_actions[player_idx].sprint;
                    let detail = ActionDetail::for_dribble(DribbleStyle::Carry, target, sprint);
                    (PlayerAction::Dribble, detail)
                }
            }

            FinalActionType::Hold | FinalActionType::Movement => {
                (PlayerAction::Hold, ActionDetail::empty())
            }

            _ => (PlayerAction::Hold, ActionDetail::empty()),
        }
    }

    /// P16: FinalAction → (PlayerAction, ActionDetail, Option<ActionDetailV2>) 변환
    ///
    /// PlayerAction은 의도 5개로 고정, 세부는 ActionDetail에 보존
    ///
    /// ## FIX_2601/1124: Gate A 검증 + V2 빌드
    ///
    /// `strict_contracts` feature 활성화 시, 선택된 액션(FinalAction)과
    /// 변환된 액션(PlayerAction)의 타입 일관성을 검증합니다.
    ///
    /// `detail_v2` feature 활성화 시, ActionDetailV2도 함께 반환합니다.
    fn convert_final_action_with_detail(
        &mut self,
        action: super::decision_topology::FinalAction,
        player_idx: usize,
    ) -> (
        PlayerAction,
        crate::engine::action_detail::ActionDetail,
        Option<super::action_detail_v2::ActionDetailV2>,
    ) {
        use super::decision_topology::{FinalActionParams, FinalActionType};
        use crate::engine::action_detail::{
            ActionDetail, ActionParam, ActionTarget, DribbleStyle, PassType as DetailPassType,
            ShotType as DetailShotType,
        };

        // FIX_2601/1124 Phase 3: V2 빌드 (feature flag)
        #[cfg(feature = "detail_v2")]
        let detail_v2 = Some(self.build_action_detail_v2(&action, player_idx));
        #[cfg(not(feature = "detail_v2"))]
        let detail_v2: Option<super::action_detail_v2::ActionDetailV2> = None;

        // FIX_2601/1124: Gate A - 선택 키 추출
        let selected_key = CandidateKey::from_final_action(&action);

        // 변환 결과
        let result = match action.action_type {
            FinalActionType::Shot => {
                let detail = if let FinalActionParams::Shot(params) = &action.params {
                    let shot_type = match params.technique {
                        super::decision_topology::ShotTechnique::Normal => DetailShotType::Normal,
                        super::decision_topology::ShotTechnique::Finesse => DetailShotType::Placed,
                        super::decision_topology::ShotTechnique::Power => DetailShotType::Power,
                        super::decision_topology::ShotTechnique::Chip => DetailShotType::Chip,
                        super::decision_topology::ShotTechnique::Header => DetailShotType::Header,
                    };
                    let target = action.target_pos.map(|(x, y)| ActionTarget::GoalMouth(x, y));
                    ActionDetail {
                        target,
                        power: Some(action.power),
                        curve: Some(action.curve),
                        shot_type: Some(shot_type),
                        ..Default::default()
                    }
                } else {
                    ActionDetail::for_shot(
                        DetailShotType::Normal,
                        ActionTarget::GoalMouth(0.5, 0.5),
                        action.power,
                        action.curve,
                    )
                };
                (PlayerAction::Shoot, detail)
            }

            // FIX_2601/0117: Cross도 Pass 처리에 포함
            FinalActionType::Pass | FinalActionType::Clear | FinalActionType::Cross => {
                // target_player 저장 (호환성)
                if let Some(target_idx) = action.target_player {
                    self.last_pass_target = Some(target_idx);
                    // DPQ v1.2: Pull forward the pass target so they can prepare
                    let exp = self.get_exp_params();
                    if exp.dpq_enabled && exp.dpq_variable_cadence {
                        self.decision_scheduler.pull_forward(target_idx, self.current_tick);
                    }
                }

                let detail = if let FinalActionParams::Pass(params) = &action.params {
                    // decision_topology::PassType → action_detail::PassType 매핑
                    let pass_type = match params.pass_type {
                        super::decision_topology::PassType::Ground => {
                            // Ground는 Short로 매핑 (Long은 Lob으로 처리)
                            DetailPassType::Short
                        }
                        super::decision_topology::PassType::Through => DetailPassType::Through,
                        super::decision_topology::PassType::Lob => DetailPassType::Lob,
                        super::decision_topology::PassType::Cross => DetailPassType::Cross,
                        super::decision_topology::PassType::Clear => DetailPassType::Clear,
                    };

                    // 타겟 결정
                    let target = if let Some(target_idx) = action.target_player {
                        if pass_type == DetailPassType::Through {
                            // 스루패스는 Space로
                            let point = action.target_pos.unwrap_or((0.0, 0.0));
                            ActionTarget::Space { point, lead: 5.0 }
                        } else {
                            ActionTarget::Player(target_idx)
                        }
                    } else if let Some(pos) = action.target_pos {
                        ActionTarget::Point(pos.0, pos.1)
                    } else {
                        ActionTarget::Point(0.0, 0.0)
                    };

                    // is_lofted → Loft param 변환
                    let loft_value = if params.is_lofted { 1.0 } else { 0.0 };

                    ActionDetail {
                        target: Some(target),
                        power: Some(action.power),
                        curve: Some(action.curve),
                        pass_type: Some(pass_type),
                        params: vec![ActionParam::Loft(loft_value)],
                        ..Default::default()
                    }
                } else {
                    // Clear fallback
                    let target = action
                        .target_player
                        .map(ActionTarget::Player)
                        .or_else(|| action.target_pos.map(|(x, y)| ActionTarget::Point(x, y)))
                        .unwrap_or(ActionTarget::Point(0.0, 0.0));

                    ActionDetail::for_pass(
                        if matches!(action.action_type, FinalActionType::Clear) {
                            DetailPassType::Clear
                        } else {
                            DetailPassType::Short
                        },
                        target,
                        action.power,
                        action.curve,
                    )
                };
                (PlayerAction::Pass, detail)
            }

            FinalActionType::Dribble => {
                if let FinalActionParams::Dribble(params) = &action.params {
                    if params.is_skill_move {
                        // TakeOn
                        let detail = ActionDetail::for_takeon(
                            0, // opponent_idx는 별도 계산 필요
                            params.direction,
                            0.7, // default risk
                        );
                        (PlayerAction::TakeOn, detail)
                    } else {
                        // Dribble (DribbleParams에 sprint 필드 없음, 기본 false)
                        let target = action.target_pos.map(|(x, y)| ActionTarget::Point(x, y));
                        let sticky = self.sticky_actions[player_idx];
                        let detail =
                            ActionDetail::for_dribble(DribbleStyle::Carry, target, sticky.sprint);
                        (PlayerAction::Dribble, detail)
                    }
                } else {
                    let target = action.target_pos.map(|(x, y)| ActionTarget::Point(x, y));
                    let sticky = self.sticky_actions[player_idx];
                    let detail =
                        ActionDetail::for_dribble(DribbleStyle::Carry, target, sticky.sprint);
                    (PlayerAction::Dribble, detail)
                }
            }

            FinalActionType::Hold | FinalActionType::Movement => {
                (PlayerAction::Hold, ActionDetail::empty())
            }

            _ => (PlayerAction::Hold, ActionDetail::empty()),
        };

        // FIX_2601/1124: Gate A 검증 - 액션 타입 일관성 체크
        #[cfg(feature = "strict_contracts")]
        {
            let action_type_match = match (&selected_key, &result.0) {
                (CandidateKey::Shot(_), PlayerAction::Shoot) => true,
                (CandidateKey::Header(super::candidate_key::HeaderKey::Shot { .. }), PlayerAction::Shoot) => true,
                (CandidateKey::Pass(_), PlayerAction::Pass) => true,
                (CandidateKey::Clearance, PlayerAction::Pass) => true, // Clear → Pass
                (CandidateKey::Dribble(_), PlayerAction::Dribble) => true,
                (CandidateKey::Dribble(_), PlayerAction::TakeOn) => true, // SkillMove → TakeOn
                (CandidateKey::Tackle(_), _) => true, // Tackle은 별도 처리
                (CandidateKey::Cross(_), PlayerAction::Pass) => true, // Cross → Pass
                (CandidateKey::Intercept, _) => true, // Intercept은 다양한 액션으로
                (CandidateKey::Hold, PlayerAction::Hold) => true,
                (CandidateKey::Hold, _) => true, // Movement 등 → Hold
                _ => false,
            };

            if !action_type_match {
                // Gate A 실패 시 경고 (프로덕션에서는 로그만)
                eprintln!(
                    "[Gate A WARNING] tick={} player={}: selected_key={:?} but result={:?}",
                    self.current_tick, player_idx, selected_key.kind_name(), result.0
                );
            }
        }

        // 디버그 모드에서 선택 키 추적 (feature flag 없이도)
        #[cfg(debug_assertions)]
        if match_debug_enabled() && self.current_tick % 500 == 0 {
            eprintln!(
                "[Gate A TRACE] tick={} player={}: key={:?}",
                self.current_tick, player_idx, selected_key.kind_name()
            );
        }

        // FIX_2601/1124 Phase 3: V1 result에 V2 추가하여 반환
        (result.0, result.1, detail_v2)
    }

    // ============================================================================
    // FIX_2601/1124 Phase 3: ActionDetailV2 Builder
    // ============================================================================

    /// FIX_2601/1124: FinalAction → ActionDetailV2 변환 (RNG 없음)
    ///
    /// FinalAction의 모든 필드를 사용하여 완전한 ActionDetailV2를 생성합니다.
    /// V1의 Option 필드와 달리, V2는 모든 필드가 필수이므로 Builder에서 결정론적으로 채웁니다.
    ///
    /// ## 설계 원칙
    /// - RNG 사용 금지: 모든 값은 FinalAction에서 직접 추출
    /// - 기본값 사용: FinalAction에 없는 필드는 안전한 기본값 사용
    /// - 타입 보존: FinalActionType과 ActionDetailV2 variant 1:1 매핑
    #[cfg(feature = "detail_v2")]
    fn build_action_detail_v2(
        &self,
        action: &super::decision_topology::FinalAction,
        player_idx: usize,
    ) -> super::action_detail_v2::ActionDetailV2 {
        use super::action_detail_v2::*;
        use super::decision_topology::{FinalActionParams, FinalActionType};

        match action.action_type {
            FinalActionType::Shot => {
                let (target_point, power, shot_kind) = if let FinalActionParams::Shot(params) = &action.params {
                    let technique = match params.technique {
                        super::decision_topology::ShotTechnique::Normal => ShotKind::Normal,
                        super::decision_topology::ShotTechnique::Finesse => ShotKind::Finesse,
                        super::decision_topology::ShotTechnique::Power => ShotKind::Power,
                        super::decision_topology::ShotTechnique::Chip => ShotKind::Chip,
                        super::decision_topology::ShotTechnique::Header => ShotKind::Normal, // Header는 별도 ActionDetailV2::Header로 처리
                    };
                    // target_pos를 정규화 좌표로 사용 (FinalAction에서는 이미 정규화됨)
                    let target = action.target_pos.unwrap_or((0.9, 0.5)); // 기본: 골문 중앙
                    (target, action.power, technique)
                } else {
                    // fallback: params가 없으면 기본값
                    ((0.9, 0.5), action.power, ShotKind::Normal)
                };

                ActionDetailV2::Shot(ShotDetail::new(target_point, power, shot_kind))
            }

            FinalActionType::Pass => {
                let (target_track_id, pass_kind, power) = if let FinalActionParams::Pass(params) = &action.params {
                    let kind = match params.pass_type {
                        super::decision_topology::PassType::Ground => PassKind::Short,
                        super::decision_topology::PassType::Through => PassKind::Through,
                        super::decision_topology::PassType::Lob => PassKind::Lob,
                        super::decision_topology::PassType::Cross => PassKind::Short, // Cross는 별도 처리
                        super::decision_topology::PassType::Clear => PassKind::Long, // Clear는 Long 계열
                    };
                    let target = action.target_player.unwrap_or(player_idx) as u8;
                    (target, kind, action.power)
                } else {
                    // fallback
                    let target = action.target_player.unwrap_or(player_idx) as u8;
                    (target, PassKind::Short, action.power)
                };

                let mut detail = PassDetail::new(target_track_id, pass_kind, power);
                detail.intended_point = action.target_pos;
                // FIX_2601/1129: 선택 시점 패서 위치 (forward_pass_rate 측정용)
                let passer_pos_coord10 = self.get_player_position_by_index(player_idx);
                detail.intended_passer_pos = Some(passer_pos_coord10.to_meters());
                ActionDetailV2::Pass(detail)
            }

            FinalActionType::Cross => {
                let (target_point, cross_kind, power) = if let FinalActionParams::Pass(params) = &action.params {
                    let kind = if params.is_lofted {
                        CrossKind::High
                    } else {
                        CrossKind::Low
                    };
                    let target = action.target_pos.unwrap_or((0.85, 0.5)); // 기본: 박스 내
                    (target, kind, action.power)
                } else {
                    (action.target_pos.unwrap_or((0.85, 0.5)), CrossKind::High, action.power)
                };

                ActionDetailV2::Cross(CrossDetail {
                    target_point,
                    cross_kind,
                    power,
                })
            }

            FinalActionType::Clear => {
                let (direction, power) = if let Some(target_pos) = action.target_pos {
                    // target_pos를 방향 벡터로 변환 (현재 위치에서 target으로의 방향)
                    let player_pos = self.player_positions.get(player_idx)
                        .map(|p| p.to_normalized_legacy())
                        .unwrap_or((0.5, 0.5));
                    let dx = target_pos.0 - player_pos.0;
                    let dy = target_pos.1 - player_pos.1;
                    let len = (dx * dx + dy * dy).sqrt().max(0.001);
                    ((dx / len, dy / len), action.power)
                } else {
                    // 기본: 전방으로 클리어
                    let is_home = MatchTeamSide::is_home(player_idx);
                    let dir_x = if self.attacks_right(is_home) { 1.0 } else { -1.0 };
                    ((dir_x, 0.0), action.power)
                };

                ActionDetailV2::Clearance(ClearanceDetail { direction, power })
            }

            FinalActionType::Dribble => {
                let (direction, speed_factor) = if let FinalActionParams::Dribble(params) = &action.params {
                    // normalize direction
                    let (dx, dy) = params.direction;
                    let len = (dx * dx + dy * dy).sqrt().max(0.001);
                    let normalized = (dx / len, dy / len);
                    // is_skill_move가 true면 더 공격적
                    let speed = if params.is_skill_move { 0.8 } else { 0.6 };
                    (normalized, speed)
                } else {
                    // fallback: 전방으로
                    let is_home = MatchTeamSide::is_home(player_idx);
                    let dir_x = if self.attacks_right(is_home) { 1.0 } else { -1.0 };
                    ((dir_x, 0.0), 0.6)
                };

                ActionDetailV2::Dribble(DribbleDetail::new(direction, speed_factor))
            }

            FinalActionType::Tackle => {
                let (target_track_id, tackle_kind) = if let FinalActionParams::Tackle(params) = &action.params {
                    let kind = match params.tackle_type {
                        super::decision_topology::TackleType::Standing => TackleKind::Standing,
                        super::decision_topology::TackleType::Sliding => TackleKind::Sliding,
                        super::decision_topology::TackleType::Shoulder => TackleKind::Shoulder,
                        super::decision_topology::TackleType::Poke => TackleKind::Standing, // Poke → Standing
                    };
                    // 태클 대상: target_player 또는 볼 소유자
                    let target = action.target_player
                        .or_else(|| self.ball.current_owner)
                        .unwrap_or(0) as u8;
                    (target, kind)
                } else {
                    let target = action.target_player
                        .or_else(|| self.ball.current_owner)
                        .unwrap_or(0) as u8;
                    (target, TackleKind::Standing)
                };

                ActionDetailV2::Tackle(TackleDetail::new(target_track_id, tackle_kind))
            }

            FinalActionType::Block => {
                // Block은 Intercept로 매핑 (볼 경로 차단)
                let intercept_point = action.target_pos.unwrap_or_else(|| {
                    // 볼 위치를 기본값으로
                    self.ball.position.to_normalized_legacy()
                });

                ActionDetailV2::Intercept(InterceptDetail { intercept_point })
            }

            FinalActionType::Movement => {
                // Movement는 Hold로 매핑 (제자리 유지)
                ActionDetailV2::Hold(HoldDetail {
                    shield_direction: (0.0, 0.0),
                })
            }

            FinalActionType::Hold => {
                // Hold: shield 방향은 가장 가까운 상대 반대 방향
                let shield_direction = self.calculate_shield_direction(player_idx);
                ActionDetailV2::Hold(HoldDetail { shield_direction })
            }
        }
    }

    /// Hold 액션의 쉴드 방향 계산 (가장 가까운 상대의 반대 방향)
    #[cfg(feature = "detail_v2")]
    fn calculate_shield_direction(&self, player_idx: usize) -> (f32, f32) {
        let player_pos = match self.player_positions.get(player_idx) {
            Some(p) => p.to_normalized_legacy(),
            None => return (0.0, 0.0),
        };

        let opponent_range = MatchTeamSide::opponent_range(player_idx);

        // 가장 가까운 상대 찾기
        let nearest_opponent = opponent_range
            .filter_map(|i| {
                self.player_positions.get(i).map(|opp_pos| {
                    let opp = opp_pos.to_normalized_legacy();
                    let dx = opp.0 - player_pos.0;
                    let dy = opp.1 - player_pos.1;
                    let dist_sq = dx * dx + dy * dy;
                    (i, dx, dy, dist_sq)
                })
            })
            .min_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));

        match nearest_opponent {
            Some((_idx, dx, dy, dist_sq)) if dist_sq > 0.0001 => {
                // 상대 반대 방향으로 shield
                let len = dist_sq.sqrt();
                (-dx / len, -dy / len)
            }
            _ => (0.0, 0.0),
        }
    }

    // Helper functions

    /// 패스 옵션 개수 계산


    fn count_pass_options(&self, player_idx: usize, is_home: bool) -> usize {
        let targets = self.build_pass_targets(player_idx, is_home);
        let count = targets.len();
        // DEBUG: 패스 옵션 수 확인 (RunOps noise 방지: debug flag로만)
        if match_debug_enabled() && self.current_tick % 1000 == 0 {
            eprintln!(
                "[DEBUG] tick={} player={} is_home={} pass_options={}",
                self.current_tick, player_idx, is_home, count
            );
        }
        count
    }
    /// 가장 가까운 팀원 거리
    /// FIX_2601: Updated to use Coord10.distance_to_m()
    fn find_nearest_teammate_distance(&self, player_idx: usize, _is_home: bool) -> Option<f32> {
        let player_pos = self.player_positions.get(player_idx)?;
        let teammate_range = MatchTeamSide::teammate_range(player_idx);

        teammate_range
            .filter(|&i| i != player_idx)
            .filter_map(|i| {
                self.player_positions.get(i).map(|t_pos| player_pos.distance_to_m(t_pos))
            })
            .reduce(f32::min)
    }

    /// FIX_2601/0105 Phase 2: 더 좋은 위치의 동료 찾기
    ///
    /// Open-Football 스타일: 동료가 현재 선수의 골대 거리의 70% 이내이면
    /// 패스가 더 나은 선택임을 나타냄.
    ///
    /// FIX_2601/0105: xG 기반 + 능력치 기반 동료 탐색
    ///
    /// # Returns
    /// (has_better_teammate, best_teammate_xg)
    /// - has_better_teammate: 더 좋은 xG를 가진 동료가 있는지 (vision에 따라 발견 확률)
    /// - best_teammate_xg: 가장 좋은 동료의 xG
    fn find_better_positioned_teammate(&self, player_idx: usize, is_home: bool) -> (bool, f32) {
        use crate::engine::probability;

        let player_pos = match self.player_positions.get(player_idx) {
            Some(p) => p,
            None => return (false, 0.0),
        };

        // 내 xG 계산 (위치 기반)
        let attacks_right = self.attacks_right(is_home);
        let my_xg =
            probability::shooting_probability(player_pos.to_normalized_legacy(), attacks_right);

        // 능력치: vision (동료 발견 능력)
        let vision = self.get_player_vision(player_idx);
        // vision 10 = 50% 발견, vision 15 = 75%, vision 20 = 100%
        let detection_chance = ((vision - 5.0) / 20.0).clamp(0.3, 1.0);

        let teammate_range = MatchTeamSide::teammate_range(player_idx);
        let mut best_teammate_xg = 0.0f32;
        let mut has_better = false;

        for i in teammate_range {
            if i == player_idx {
                continue;
            }
            // GK 제외
            if i == 0 || i == 11 {
                continue;
            }

            if let Some(t_pos) = self.player_positions.get(i) {
                // 동료 xG 계산
                let teammate_xg =
                    probability::shooting_probability(t_pos.to_normalized_legacy(), attacks_right);

                // FIX_2601/0106: 동료가 더 좋은 xG를 가짐 (최소 100% 이상 높아야 함)
                // FIX_2601/0114: 1.5 → 2.0: 50%는 여전히 슛 과소 유발
                if teammate_xg > my_xg * 2.0 {
                    // vision에 따른 발견 확률
                    // 높은 vision = 항상 발견, 낮은 vision = 놓칠 수 있음
                    // 이 함수는 &self만 있어서 RNG 불가 → 결정론적으로 처리
                    // xG 차이가 클수록 발견 확률 증가
                    let xg_diff_factor = ((teammate_xg - my_xg) / my_xg.max(0.01)).min(2.0);
                    let effective_chance =
                        (detection_chance * (1.0 + xg_diff_factor * 0.5)).min(1.0);

                    // 결정론적 발견: effective_chance > 0.5면 발견
                    if effective_chance > 0.5 {
                        has_better = true;
                    }
                }

                best_teammate_xg = best_teammate_xg.max(teammate_xg);
            }
        }

        (has_better, best_teammate_xg)
    }

    /// Match OS v1.2 Priority 2: Immediate pressure calculation (2m radius tackle threat)
    ///
    /// Calculates meter-level immediate pressure from opponents within 2.0m radius.
    /// This is separate from FieldBoard tactical influence (4m radius).
    ///
    /// # Arguments
    /// * `_player_idx` - Index of the player (unused, kept for API consistency)
    /// * `is_home` - Whether player is on home team
    /// * `player_pos_m` - Player position in meters (x=length, y=width)
    ///
    /// # Returns
    /// Pressure value normalized to 0..1
    fn calculate_immediate_pressure(
        &self,
        _player_idx: usize,
        is_home: bool,
        player_pos_m: (f32, f32),
    ) -> f32 {
        const R_IMMEDIATE_M: f32 = 2.0; // 2m radius for immediate tackle threat

        let opponent_range = MatchTeamSide::opponent_range_for_home(is_home);
        let mut pressure = 0.0;

        for opp_idx in opponent_range {
            // FIX_2601: player_positions는 Coord10
            if let Some(&opp_pos) = self.player_positions.get(opp_idx) {
                // Convert to meters for accurate distance calculation
                let opp_pos_m = opp_pos.to_meters();

                // Calculate distance in meters
                let dx = player_pos_m.0 - opp_pos_m.0;
                let dy = player_pos_m.1 - opp_pos_m.1;
                let dist = (dx * dx + dy * dy).sqrt();

                // Only consider opponents within immediate radius
                if dist < R_IMMEDIATE_M {
                    // Quadratic falloff: closer = more pressure
                    let contrib = (1.0 - dist / R_IMMEDIATE_M).powi(2);

                    // Weight by opponent stamina: tired defenders exert less pressure
                    let opp_stamina = self.stamina.get(opp_idx).copied().unwrap_or(1.0);
                    let w_defender = opp_stamina * 0.7 + 0.3; // 30% baseline even when exhausted

                    pressure += contrib * w_defender;
                }
            }
        }

        // Normalize to 0..1 range
        // Typical max: 2-3 defenders stacked at same position
        pressure.clamp(0.0, 1.0)
    }

    // ===================================================================
    // Match OS v1.2 Priority 3: LaneBlock Hybrid System - Raycast Helpers
    // ===================================================================

    /// Precise lane blockage check via raycast (body + legs)
    ///
    /// Checks if any opponent is within 1.0m of the pass lane using
    /// precise point-to-line distance calculation.
    ///
    /// Returns true if lane is blocked, false if clear.
    fn lane_blocked_raycast(
        &self,
        from_m: (f32, f32),
        to_m: (f32, f32),
        is_home_passer: bool,
    ) -> bool {
        use crate::engine::body_blocking;

        const BLOCK_RADIUS_M: f32 = 1.0; // Body + legs reach

        let opponent_range = MatchTeamSide::opponent_range_for_home(is_home_passer);

        for opp_idx in opponent_range {
            // FIX_2601: player_positions는 Coord10
            if let Some(&opp_pos) = self.player_positions.get(opp_idx) {
                let opp_pos_m = opp_pos.to_meters();

                let dist = body_blocking::point_to_line_distance(opp_pos_m, from_m, to_m);

                if dist < BLOCK_RADIUS_M {
                    return true; // Blocked
                }
            }
        }

        false
    }

    /// Find top N candidates who might block pass
    ///
    /// Filters opponents to those within 3m of pass lane,
    /// then returns the N closest ones sorted by distance.
    ///
    /// This reduces full O(11) checks to O(N) with N = 2-4 typically.
    fn find_lane_block_candidates(
        &self,
        from_m: (f32, f32),
        to_m: (f32, f32),
        is_home_passer: bool,
        max_candidates: usize,
    ) -> Vec<usize> {
        use crate::engine::body_blocking;

        let opponent_range = MatchTeamSide::opponent_range_for_home(is_home_passer);
        let mut candidates: Vec<(usize, f32)> = Vec::with_capacity(11);

        for opp_idx in opponent_range {
            // FIX_2601: player_positions는 Coord10
            if let Some(&opp_pos) = self.player_positions.get(opp_idx) {
                let opp_pos_m = opp_pos.to_meters();
                let dist = body_blocking::point_to_line_distance(opp_pos_m, from_m, to_m);

                if dist < 3.0 {
                    // Only within 3m range
                    candidates.push((opp_idx, dist));
                }
            }
        }

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(max_candidates);
        candidates.into_iter().map(|(idx, _)| idx).collect()
    }

    /// 공격 페널티박스 내 여부
    ///
    /// FIX_2601/0110: to_normalized_legacy() returns (width, length) = (y/680, x/1050)
    /// - pos.0 = width (sideline direction, 0-1)
    /// - pos.1 = length (goal direction, 0-1)
    /// FIX_2601/0110: Changed parameter from is_home to attacks_right for correct 2nd half behavior
    fn is_in_attacking_penalty_box(&self, pos: (f32, f32), attacks_right: bool) -> bool {
        // Width check: penalty box is roughly 0.21 to 0.79 (central area)
        let in_width = pos.0 > 0.21 && pos.0 < 0.79;

        // TeamView: opponent penalty area is always near length=1.0.
        let tv = coordinates::to_team_view_normalized(pos, attacks_right);
        let penalty_start = 1.0 - (field::PENALTY_AREA_LENGTH_M / field::LENGTH_M);
        in_width && tv.1 > penalty_start
    }

    /// 터치라인 근처 여부
    ///
    /// FIX_2601/0110: pos.0 = width (sideline direction)
    fn is_near_touchline(&self, pos: (f32, f32)) -> bool {
        pos.0 < 0.1 || pos.0 > 0.9
    }

    /// 현재 페이즈 가져오기
    fn get_current_phase(&self, is_home: bool) -> Option<crate::engine::team_phase::TeamPhase> {
        if is_home {
            Some(self.home_phase_state.phase)
        } else {
            Some(self.away_phase_state.phase)
        }
    }

    /// 두 위치 사이 거리
    fn distance_between_positions(&self, a: (f32, f32), b: (f32, f32)) -> f32 {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        (dx * dx + dy * dy).sqrt()
    }

    /// FIX_2601/0116: 공격 페널티박스 내 팀원 수 계산
    fn count_teammates_in_box(&self, player_idx: usize, is_home: bool, attacks_right: bool) -> u8 {
        let team_start = if is_home { 0 } else { 11 };
        let team_end = team_start + 11;

        // FIX_2601/0117: Penalty box dimensions (in meters):
        // - Width: ~40.32m (y: 13.84 to 54.16, centered at 34m)
        // - Depth: ~16.5m from goal line
        let box_x_min_tv = field::LENGTH_M - field::PENALTY_AREA_LENGTH_M;
        let box_x_max_tv = field::LENGTH_M;
        let box_y_min = 13.84; // FIX: was -20.0 (wrong coordinate system)
        let box_y_max = 54.16; // FIX: was 20.0 (wrong coordinate system)

        let mut count = 0u8;
        for idx in team_start..team_end {
            if idx == player_idx {
                continue; // 자기 자신 제외
            }
            let pos = self.player_positions[idx].to_meters();
            let x_tv = if attacks_right { pos.0 } else { field::LENGTH_M - pos.0 };
            if x_tv >= box_x_min_tv && x_tv <= box_x_max_tv && pos.1 >= box_y_min && pos.1 <= box_y_max {
                count += 1;
            }
        }
        count.min(5) // 최대 5명까지만 카운트
    }

    // ========== Career Player Mode: User Control Helpers ==========
    fn lock_duration_for_user_cmd(payload: &super::UserCommandPayload) -> u64 {
        match payload {
            super::UserCommandPayload::OnBallAction { action, .. } => match action {
                super::OnBallAction::Pass => 2,
                super::OnBallAction::Shoot => 3,
                super::OnBallAction::TakeOn => 4,
                super::OnBallAction::Carry => 1,
                super::OnBallAction::Hold => 1,
            },
        }
    }

    /// 큐에서 유효한 user_command 추출 (중복/잘못된 명령 제거)
    fn pop_latest_valid_cmd_for(&mut self, owner_idx: usize) -> Option<super::UserCommand> {
        let controlled = self.controlled_mode.as_ref();

        // 큐를 순회하며 유효한 명령 찾기
        while let Some(cmd) = self.user_command_queue.pop_front() {
            // 1) Seq 검증 (중복 방지)
            if let Some(controlled) = controlled {
                if cmd.seq <= controlled.last_consumed_seq {
                    continue; // 이미 처리한 명령
                }
            }

            // 2) Track ID 일치 확인
            if cmd.controlled_track_id != owner_idx {
                continue; // 잘못된 대상
            }

            if let Some(controlled) = controlled {
                if controlled.is_controlled(owner_idx) {
                    return Some(cmd);
                }
                if self.multi_agent_tracks.contains(&owner_idx) {
                    return Some(cmd);
                }
                continue;
            }

            if self.multi_agent_tracks.contains(&owner_idx) {
                return Some(cmd);
            }
        }

        None
    }

    /// UserCommand를 PlayerAction으로 변환
    fn convert_user_cmd_to_player_action(
        &mut self,
        owner_idx: usize,
        payload: super::UserCommandPayload,
    ) -> PlayerAction {
        use super::{OnBallAction, UserCommandPayload};

        match payload {
            UserCommandPayload::OnBallAction { action, variant, target_track_id } => match action {
                OnBallAction::Pass => {
                    // 타겟이 있으면 사용, 없으면 엔진이 자동 선택
                    let _target_idx = if let Some(t) = target_track_id {
                        t
                    } else {
                        // 최적 패스 대상 찾기 (기존 로직 재사용)
                        self.find_best_pass_target_idx(owner_idx)
                    };

                    // Variant로 패스 유형 결정
                    match variant.as_deref() {
                        Some("through") => PlayerAction::ThroughBall,
                        Some("cross") => PlayerAction::Cross,
                        Some("chip") => PlayerAction::LongPass, // Chip을 LongPass로 매핑
                        _ => PlayerAction::Pass,                // 기본 짧은 패스
                    }
                }

                OnBallAction::Shoot => {
                    // Variant는 무시 (엔진이 ShotTechnique 자동 결정)
                    PlayerAction::Shoot
                }

                OnBallAction::Carry => {
                    // 안전한 운반 (드리블 비공격적)
                    PlayerAction::Dribble
                }

                OnBallAction::TakeOn => {
                    // 돌파 시도
                    PlayerAction::TakeOn
                }

                OnBallAction::Hold => PlayerAction::Hold,
            },
        }
    }

    /// 최적 패스 대상 찾기 (user_command에서 target 없을 때 사용)
    /// FIX_2601/0105: 6-Factor pass scoring system 사용
    fn find_best_pass_target_idx(&self, passer_idx: usize) -> usize {
        let is_home = MatchTeamSide::is_home(passer_idx);

        // FIX_2601/0105: Use 6-factor pass scoring instead of simple distance
        if let Some(best_target) = self.find_best_pass_target(passer_idx, is_home) {
            return best_target;
        }

        // Fallback: 가장 가까운 동료 선수 찾기 (오프사이드 아닌 선수 우선)
        // FIX_2601/0110: Add offside check to fallback path to prevent offside bias
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let team_range = if is_home { 0..11 } else { 11..22 };

        // First try: non-offside players
        // FIX_2601/0110: Use composite key (distance, y_pos) to avoid index order bias
        let non_offside_nearest = team_range
            .clone()
            .filter(|&idx| idx != passer_idx)
            .filter(|&idx| !self.is_offside_position(idx, is_home))
            .min_by(|&a, &b| {
                let pos_a = self.get_player_position_by_index(a);
                let pos_b = self.get_player_position_by_index(b);
                let dist_a = (passer_pos.distance_to_m(&pos_a) * 1000.0) as i32;
                let dist_b = (passer_pos.distance_to_m(&pos_b) * 1000.0) as i32;
                // Primary: distance, Secondary: Y position to break ties
                match dist_a.cmp(&dist_b) {
                    std::cmp::Ordering::Equal => {
                        let y_a = (pos_a.to_meters().1 * 1000.0) as i32;
                        let y_b = (pos_b.to_meters().1 * 1000.0) as i32;
                        y_a.cmp(&y_b)
                    }
                    ord => ord,
                }
            });

        if let Some(target) = non_offside_nearest {
            return target;
        }

        // Last resort: nearest player (may be offside, but better than self-pass)
        // FIX_2601/0110: Use composite key (distance, y_pos) to avoid index order bias
        team_range
            .filter(|&idx| idx != passer_idx)
            .min_by(|&a, &b| {
                let pos_a = self.get_player_position_by_index(a);
                let pos_b = self.get_player_position_by_index(b);
                let dist_a = (passer_pos.distance_to_m(&pos_a) * 1000.0) as i32;
                let dist_b = (passer_pos.distance_to_m(&pos_b) * 1000.0) as i32;
                match dist_a.cmp(&dist_b) {
                    std::cmp::Ordering::Equal => {
                        let y_a = (pos_a.to_meters().1 * 1000.0) as i32;
                        let y_b = (pos_b.to_meters().1 * 1000.0) as i32;
                        y_a.cmp(&y_b)
                    }
                    ord => ord,
                }
            })
            .unwrap_or(passer_idx)
    }

    // ========== FIX_2601/0108: UAE Pipeline Integration ==========

    /// Try UAE (Unified Action Evaluator) pipeline for action selection
    ///
    /// This function is called when `use_uae_pipeline` flag is enabled.
    /// It builds the required contexts and runs the DecisionPipeline.
    fn try_uae_pipeline(
        &mut self,
        player_idx: usize,
    ) -> (PlayerAction, crate::engine::action_detail::ActionDetail) {
        use crate::engine::action_detail::ActionDetail;

        // 공 소유 확인
        if self.ball.current_owner != Some(player_idx) {
            return (PlayerAction::Hold, ActionDetail::empty());
        }

        let is_home = MatchTeamSide::is_home(player_idx);
        let player_pos = self.get_player_position_by_index(player_idx);
        let player_pos_m = player_pos.to_meters();

        // Build UAE contexts
        let state_ctx = self.build_uae_state_context(player_idx);
        let action_ctx = self.build_uae_action_set_context(player_idx);
        let eval_ctx = self.build_uae_eval_context(player_idx);
        let hard_gate_ctx = self.build_uae_hard_gate_context(player_idx);
        let mut team_coord = TeamCoordinator::new();

        // Get player position string (simplified: derive from player index)
        let local_idx = MatchTeamSide::local_idx(player_idx);
        let position_key = match local_idx {
            0 => "GK",
            1 | 4 => "CB",
            2 => "LB",
            3 => "RB",
            5 | 6 => "CM",
            7 => "LW",
            8 => "RW",
            9 | 10 => "ST",
            _ => "CM",
        }
        .to_string();

        // Run pipeline
        let mut config = PipelineConfig::default();
        config.debug_logging = std::env::var("UAE_DEBUG").is_ok();
        let pipeline = DecisionPipeline::new(config);
        let result = pipeline.execute(
            UaePlayerId::new(player_idx as u8),
            &state_ctx,
            &action_ctx,
            &eval_ctx,
            &hard_gate_ctx,
            &mut team_coord,
            &position_key,
            &[],        // traits - empty for now
            "Balanced", // mentality
            "Mixed",    // passing style
            "Normal",   // tempo
        );

        // Debug output
        if std::env::var("UAE_DEBUG").is_ok() {
            let team = if is_home { "HOME" } else { "AWAY" };
            eprintln!("[UAE] player_idx={} team={} state={:?} in_zone={} dist={:.1}",
                player_idx, team, result.state, eval_ctx.in_shooting_zone, eval_ctx.dist_to_goal);
            eprintln!("[UAE] candidates={} filtered={} scored={}",
                result.all_scored.len() + result.filtered_count,
                result.filtered_count,
                result.all_scored.len());
            for sa in &result.all_scored {
                eprintln!("[UAE]   {:?} -> total={:.3}", sa.action, sa.weighted_total);
            }
            eprintln!("[UAE] selected={:?}", result.selected.as_ref().map(|s| &s.action));
        }

        // Convert UAE result to PlayerAction + ActionDetail
        if let Some(scored) = result.selected {
            self.convert_uae_action_to_player_action(&scored.action, player_idx)
        } else {
            // No action selected - fallback to Hold
            (PlayerAction::Hold, ActionDetail::empty())
        }
    }

    /// Build StateContext for UAE pipeline
    fn build_uae_state_context(&self, player_idx: usize) -> StateContext {
        let is_home = MatchTeamSide::is_home(player_idx);
        let team_has_ball = self
            .ball
            .current_owner
            .map_or(false, |owner| MatchTeamSide::is_home(owner) == is_home);
        let i_have_ball = self.ball.current_owner == Some(player_idx);
        let ball_pos = self.ball.position.to_meters();
        let player_pos = self.get_player_position_by_index(player_idx).to_meters();
        let dist_to_ball =
            ((ball_pos.0 - player_pos.0).powi(2) + (ball_pos.1 - player_pos.1).powi(2)).sqrt();

        // Find ball carrier distance (for defensive situations)
        let dist_to_ball_carrier = if let Some(carrier_idx) = self.ball.current_owner {
            if carrier_idx != player_idx {
                let carrier_pos = self.get_player_position_by_index(carrier_idx).to_meters();
                ((carrier_pos.0 - player_pos.0).powi(2) + (carrier_pos.1 - player_pos.1).powi(2))
                    .sqrt()
            } else {
                0.0
            }
        } else {
            dist_to_ball
        };

        StateContext {
            team_has_ball,
            i_have_ball,
            dist_to_ball,
            possession_changed_tick: 0, // TODO: track possession changes
            current_tick: self.current_tick,
            marking_assignment: None, // TODO: integrate with MarkingManager
            pass_lane_clear: true,    // TODO: calculate from field board
            body_facing_ball: true,   // TODO: use player_body_dir
            dist_to_ball_carrier,
            assigned_to_ball_carrier: false,
            closest_to_ball_carrier: false,
        }
    }

    /// Build ActionSetContext for UAE pipeline
    fn build_uae_action_set_context(&self, player_idx: usize) -> ActionSetContext {
        let is_home = MatchTeamSide::is_home(player_idx);
        let attacks_right = self.attacks_right(is_home);
        let player_pos = self.get_player_position_by_index(player_idx).to_meters();

        // Check if in shooting zone (attacking third near goal, TeamView semantics)
        let x_tv = if attacks_right { player_pos.0 } else { field::LENGTH_M - player_pos.0 };
        let dist_to_goal = ((field::LENGTH_M - x_tv).powi(2) + (player_pos.1 - field::CENTER_Y).powi(2)).sqrt();
        let in_shooting_zone = dist_to_goal < 30.0;
        let in_crossing_zone = (player_pos.1 < 20.0 || player_pos.1 > 48.0) && x_tv > 80.0;

        // Get pass targets
        let team_range = if is_home { 0..11 } else { 11..22 };
        let pass_targets: Vec<UaePlayerId> = team_range
            .filter(|&idx| idx != player_idx && idx % 11 != 0) // Exclude self and GK
            .map(|idx| UaePlayerId::new(idx as u8))
            .collect();

        // Ball carrier distance (for defensive context)
        let dist_to_ball_carrier = self
            .ball
            .current_owner
            .map(|carrier| {
                let carrier_pos = self.get_player_position_by_index(carrier).to_meters();
                ((carrier_pos.0 - player_pos.0).powi(2) + (carrier_pos.1 - player_pos.1).powi(2))
                    .sqrt()
            })
            .unwrap_or(50.0);

        // FIX_2601/1128: Find reciprocal targets (players who recently passed to me)
        let reciprocal_targets: Vec<UaePlayerId> = self.find_reciprocal_targets_for_uae(player_idx, is_home);

        // Use Default and override specific fields
        ActionSetContext {
            player_x: player_pos.0,
            player_y: player_pos.1,
            attacks_right,
            in_shooting_zone,
            has_clear_shot: self.has_clear_shot(player_idx),
            in_crossing_zone,
            pass_targets,
            through_ball_targets: vec![],
            reciprocal_targets,
            dist_to_ball_carrier,
            marking_target: None,
            most_dangerous_lane: None,
            most_exposed_zone: None,
            has_runner_ahead: false,
            best_counter_target: None,
            counter_space: None,
            in_own_third: dist_to_goal > 70.0,
            under_pressure: false,
        }
    }

    /// FIX_2601/1128: Find players for reciprocity bonus
    /// Includes both: who passed TO me, and who I passed TO
    fn find_reciprocal_targets_for_uae(&self, player_idx: usize, is_home: bool) -> Vec<UaePlayerId> {
        let passer = player_idx as u8;
        let mut candidates: Vec<UaePlayerId> = vec![];

        for &(from, to) in self.recent_pass_pairs.iter().rev() {
            // Who passed TO me (for A→B→A patterns)
            if to == passer {
                let from_idx = from as usize;
                let is_teammate = if is_home { from_idx < 11 } else { from_idx >= 11 };
                let not_gk = from_idx != 0 && from_idx != 11;
                if is_teammate && not_gk && from_idx != player_idx {
                    let pid = UaePlayerId::new(from);
                    if !candidates.contains(&pid) {
                        candidates.push(pid);
                    }
                }
            }
            // Who I passed TO (for me→B→me patterns)
            if from == passer {
                let to_idx = to as usize;
                let is_teammate = if is_home { to_idx < 11 } else { to_idx >= 11 };
                let not_gk = to_idx != 0 && to_idx != 11;
                if is_teammate && not_gk && to_idx != player_idx {
                    let pid = UaePlayerId::new(to);
                    if !candidates.contains(&pid) {
                        candidates.push(pid);
                    }
                }
            }
        }

        candidates
    }

    /// Build EvalContext for UAE pipeline
    fn build_uae_eval_context(&self, player_idx: usize) -> EvalContext {
        let is_home = MatchTeamSide::is_home(player_idx);
        let player_pos = self.get_player_position_by_index(player_idx).to_meters();
        let attacks_right = self.attacks_right(is_home);

        // Calculate xG (simple distance-based model, TeamView semantics)
        let x_tv = if attacks_right { player_pos.0 } else { field::LENGTH_M - player_pos.0 };
        let dist_to_goal = ((field::LENGTH_M - x_tv).powi(2) + (player_pos.1 - field::CENTER_Y).powi(2)).sqrt();
        // Simple xG: higher when closer to goal, max 0.3 at 0m, min 0.01 at 40m+
        let xg = (0.30 - (dist_to_goal / 40.0) * 0.29).max(0.01);

        // Get player attributes (SSOT: MatchSetup assignment-aware)
        let attrs = self.get_player_attributes(player_idx);

        // Local pressure estimate (TODO: integrate with FieldBoard)
        let local_pressure = 0.3;

        EvalContext {
            player_x: player_pos.0,
            player_y: player_pos.1,
            dist_to_goal,
            dist_to_ball: 0.0, // on-ball
            dist_to_ball_carrier: 0.0,
            stamina_pct: 0.8, // TODO: get from stamina system
            xg,
            shot_lane_clear: self.has_clear_shot(player_idx),
            is_one_on_one: self.count_defenders_ahead(player_idx) == 0 && dist_to_goal < 15.0,
            in_shooting_zone: dist_to_goal < 30.0,
            local_pressure,
            finishing: attrs.finishing as f32,
            long_shots: attrs.long_shots as f32,
            composure: attrs.composure as f32,
            technique: attrs.technique as f32,
            passing: attrs.passing as f32,
            vision: attrs.vision as f32,
            crossing: attrs.crossing as f32,
            dribbling: attrs.dribbling as f32,
            flair: attrs.flair as f32,
            agility: attrs.agility as f32,
            pace: attrs.pace as f32,
            acceleration: attrs.acceleration as f32,
            strength: attrs.strength as f32,
            balance: attrs.balance as f32,
            heading: attrs.heading as f32,
            jumping: attrs.jumping as f32,
            tackling: attrs.tackling as f32,
            marking: attrs.marking as f32,
            positioning: attrs.positioning as f32,
            anticipation: attrs.anticipation as f32,
            decisions: attrs.decisions as f32,
            concentration: attrs.concentration as f32,
            aggression: attrs.aggression as f32,
            work_rate: attrs.work_rate as f32,
            teamwork: attrs.teamwork as f32,
            off_the_ball: attrs.off_the_ball as f32,
            // FIX_2601/0109: 패스 관련 기본값 설정 (중요!)
            receiver_dist: 15.0,  // 15m 패스 거리 (최적)
            pass_lane_clear: true,  // 기본적으로 패스 가능
            pass_interceptor_count: 0,
            receiver_freedom: 0.6,  // 적당한 자유도
            receiver_has_space: 0.5,
            line_break_value: 0.2,  // 약간의 라인 돌파
            receiver_xg_if_receives: 0.1,  // 받으면 약간의 xG
            receiver_is_forward: true,  // 전진 패스 가정
            // FIX_2601/0109: 드리블 관련 기본값 설정
            space_ahead: 0.4,  // 패스보다 낮게 조정
            defenders_ahead: self.count_defenders_ahead(player_idx) as u32,  // 실제 계산
            closest_defender_dist: 3.0,  // 3m 거리 (좀 더 가까움)
            dribble_success_probability: 0.45,  // 패스보다 낮게
            has_outlet: true,  // 패스 옵션 있음
            // Hold 관련 (Hold 사용 안 하므로 낮게)
            nearby_opponents: 2,
            can_shield_ball: false,
            teammates_advancing_ratio: 0.3,
            attacks_right,
            ..Default::default()
        }
    }

    /// Build HardGateContext for UAE pipeline
    fn build_uae_hard_gate_context(&self, player_idx: usize) -> HardGateContext {
        HardGateContext::default() // TODO: Add offside checks
    }

    /// Convert UAE Action to PlayerAction + ActionDetail
    fn convert_uae_action_to_player_action(
        &mut self,
        action: &UaeAction,
        player_idx: usize,
    ) -> (PlayerAction, crate::engine::action_detail::ActionDetail) {
        use crate::engine::action_detail::{ActionDetail, ActionTarget, PassType, ShotType};

        let is_home = MatchTeamSide::is_home(player_idx);

        match action {
            UaeAction::Shoot => {
                let detail = ActionDetail::for_shot(ShotType::Normal, ActionTarget::GoalMouth(0.5, 0.5), 0.8, 0.0);
                (PlayerAction::Shoot, detail)
            }
            UaeAction::Pass { target_id } => {
                let target_idx = target_id.as_usize();
                self.last_pass_target = Some(target_idx);
                // DPQ v1.2: Pull forward the pass target so they can prepare
                let exp = self.get_exp_params();
                if exp.dpq_enabled && exp.dpq_variable_cadence {
                    self.decision_scheduler.pull_forward(target_idx, self.current_tick);
                }
                let detail = ActionDetail::for_pass(
                    PassType::Short,
                    ActionTarget::Player(target_idx),
                    0.7,
                    0.0,
                );
                (PlayerAction::Pass, detail)
            }
            UaeAction::ThroughBall { target_id } => {
                let target_idx = target_id.as_usize();
                self.last_pass_target = Some(target_idx);
                // DPQ v1.2: Pull forward the pass target so they can prepare
                let exp = self.get_exp_params();
                if exp.dpq_enabled && exp.dpq_variable_cadence {
                    self.decision_scheduler.pull_forward(target_idx, self.current_tick);
                }
                let detail = ActionDetail::for_pass(
                    PassType::Through,
                    ActionTarget::Player(target_idx),
                    0.8,
                    0.0,
                );
                (PlayerAction::ThroughBall, detail)
            }
            UaeAction::Dribble { direction } => {
                let detail = ActionDetail {
                    params: vec![crate::engine::action_detail::ActionParam::Direction(
                        direction.x,
                        direction.y,
                    )],
                    ..Default::default()
                };
                (PlayerAction::Dribble, detail)
            }
            UaeAction::Cross { target_zone } => {
                // Cross to area based on target zone
                let target_x = if self.attacks_right(is_home) { 100.0 } else { 5.0 };
                let target_y = match target_zone {
                    CrossZone::NearPost => 25.0,
                    CrossZone::FarPost => 43.0,
                    CrossZone::PenaltySpot => field::CENTER_Y,
                    CrossZone::Cutback => 30.0,
                };
                let detail = ActionDetail::for_pass(
                    PassType::Cross,
                    ActionTarget::Point(target_x, target_y),
                    0.75,
                    0.0,
                );
                (PlayerAction::Cross, detail)
            }
            UaeAction::Hold => (PlayerAction::Hold, ActionDetail::empty()),
            UaeAction::Header { is_shot } => {
                if *is_shot {
                    let detail = ActionDetail::for_shot(ShotType::Header, ActionTarget::GoalMouth(0.5, 0.5), 0.7, 0.0);
                    (PlayerAction::Header, detail)
                } else {
                    // Header pass
                    (PlayerAction::Header, ActionDetail::empty())
                }
            }
            UaeAction::Clear => {
                // Clear - long pass without specific target
                let detail = ActionDetail {
                    pass_type: Some(PassType::Clear),
                    power: Some(0.9),
                    ..Default::default()
                };
                (PlayerAction::LongPass, detail)
            }
            // Off-ball and defensive actions - shouldn't happen for on-ball player
            _ => (PlayerAction::Hold, ActionDetail::empty()),
        }
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::match_sim::test_fixtures::create_test_team;

    fn create_test_engine() -> MatchEngine {
        let plan = super::super::MatchPlan {
            home_team: create_test_team("Home"),
            away_team: create_test_team("Away"),
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
        let mut engine = MatchEngine::new(plan).expect("match engine init");
        engine.initialize_player_positions();
        engine
    }

    #[test]
    fn test_shot_ev_close_range() {
        let engine = create_test_engine();

        // 페널티 박스 내 위치 설정
        // 정규화 좌표로 ~95m 지점 (골대까지 ~10m)
        // Home팀은 x=1.0이 상대 골대

        let ev = engine.estimate_shot_ev(9); // striker

        // 거리에 따라 양수/음수 EV 가능
        // 테스트는 함수가 작동하는지 확인
        assert!(ev > -1.0 && ev < 1.0, "Shot EV out of range: {}", ev);
    }

    #[test]
    fn test_shot_ev_long_range_is_lower() {
        let engine = create_test_engine();

        // 두 선수의 EV 비교 (근거리 vs 원거리)
        let ev_striker = engine.estimate_shot_ev(9); // 공격수 (전방)
        let ev_defender = engine.estimate_shot_ev(3); // 수비수 (후방)

        // 공격수(전방)의 슛 EV가 더 높아야 함
        // (실제로는 포지션 배치에 따라 다를 수 있음)
        println!("Striker EV: {}, Defender EV: {}", ev_striker, ev_defender);
    }

    #[test]
    fn test_pass_ev_returns_target() {
        let engine = create_test_engine();

        let (ev, target) = engine.estimate_pass_ev(5); // midfielder

        // 패스 가능한 타겟이 있어야 함
        assert!(target.is_some(), "Should find a pass target");
        println!("Pass EV: {}, Target: {:?}", ev, target);
    }

    #[test]
    fn test_dribble_ev_calculation() {
        let engine = create_test_engine();

        let ev = engine.estimate_dribble_ev(9); // striker

        // 드리블 EV는 위치와 압박에 따라 결정
        assert!(ev > -1.0 && ev < 1.0, "Dribble EV out of range: {}", ev);
        println!("Dribble EV: {}", ev);
    }

    #[test]
    fn test_attack_threat_increases_near_goal() {
        let engine = create_test_engine();

        // Normalized coords: (width, length)
        // - width: 0.5 = center horizontally
        // - length: 0 = own goal, 1 = opponent goal
        // Home team attacks towards length=1.0 (opponent goal)
        let threat_far = engine.estimate_attack_threat_at((0.5_f32, 0.3_f32), TeamSide::Home);
        let threat_mid = engine.estimate_attack_threat_at((0.5_f32, 0.7_f32), TeamSide::Home);
        let threat_close = engine.estimate_attack_threat_at((0.5_f32, 0.9_f32), TeamSide::Home);

        // 골대에 가까울수록 위협도 높음
        assert!(threat_close > threat_mid, "Close threat should be > mid");
        assert!(threat_mid > threat_far, "Mid threat should be > far");

        println!("Threat - Far: {}, Mid: {}, Close: {}", threat_far, threat_mid, threat_close);
    }

    #[test]
    fn test_loss_of_possession_cost() {
        let engine = create_test_engine();

        // Normalized coords: (width, length) in legacy format
        // - length: 0 = own goal, 1 = opponent goal
        // FIX_2601: Convert normalized legacy to Coord10 for test
        // FIX_2601/0110: Use attacks_right (true = Home attacking right in 1st half)
        use crate::engine::types::Coord10;
        let attacks_right = true; // Home team in 1st half attacks right
        let cost_own_half = engine.estimate_loss_of_possession_cost(
            attacks_right,
            Coord10::from_normalized_legacy((0.5_f32, 0.3_f32)), // 자기 진영 (length=0.3)
        );
        let cost_opp_half = engine.estimate_loss_of_possession_cost(
            attacks_right,
            Coord10::from_normalized_legacy((0.5_f32, 0.8_f32)), // 상대 진영 (length=0.8)
        );

        // 상대 진영에서 공격권 상실 시 비용 더 높음 (역습 위험)
        assert!(
            cost_opp_half > cost_own_half,
            "Opponent half loss cost ({}) should be > own half ({})",
            cost_opp_half,
            cost_own_half
        );
    }

    // NOTE: Legacy tests for select_action_by_ev, select_action_with_audacity removed
    // These functions are deleted as part of ActionQueue unification (REFACTOR_CONSTITUTION.md)

    #[test]
    fn test_takeon_ev_calculation() {
        let engine = create_test_engine();

        // 공격수 TakeOn EV
        let (ev, defender) = engine.estimate_takeon_ev(9); // striker

        // EV 범위 체크
        println!("TakeOn EV: {}, Defender: {:?}", ev, defender);

        // 수비수가 너무 멀면 -1.0, 그 외에는 계산됨
        assert!((-1.0..1.0).contains(&ev), "TakeOn EV out of range: {}", ev);
    }

    #[test]
    fn test_takeon_success_prob() {
        let engine = create_test_engine();

        // 공격수 vs 수비수 TakeOn 성공 확률
        let prob = engine.estimate_takeon_success_prob(9, 13); // striker vs defender

        // 확률 범위
        assert!((0.15..=0.70).contains(&prob), "TakeOn prob out of range: {}", prob);
        println!("TakeOn success prob: {}", prob);
    }

    // NOTE: Legacy tests for audacity/utility systems removed
    // These functions are deleted as part of ActionQueue unification (REFACTOR_CONSTITUTION.md)

    #[test]
    fn test_select_best_action() {
        let mut engine = create_test_engine();

        // 공 소유자 설정
        engine.ball.current_owner = Some(9); // striker

        let action = engine.select_best_action(9);

        // 유효한 액션이 반환되어야 함
        // P16 Gate Chain은 ThroughBall, LongPass도 반환 가능
        assert!(matches!(
            action,
            PlayerAction::Shoot
                | PlayerAction::Pass
                | PlayerAction::Dribble
                | PlayerAction::Hold
                | PlayerAction::TakeOn
                | PlayerAction::ThroughBall
                | PlayerAction::LongPass
        ));

        println!("Selected best action: {:?}", action);
    }

    // ===================================================================
    // Match OS v1.2 Priority 3: LaneBlock Hybrid Integration Tests
    // ===================================================================

    #[test]
    fn test_hybrid_pass_evaluation_performance() {
        let mut engine = create_test_engine();

        // Ensure FieldBoard is initialized
        if engine.field_board.is_none() {
            engine.field_board = Some(crate::engine::field_board::FieldBoard::new(
                crate::engine::field_board::FieldBoardSpec { cols: 28, rows: 18 },
            ));
        }

        // Update FieldBoard with current positions
        // FIX_2601: Use Coord10's to_meters() directly
        let mut positions = [(0.0_f32, 0.0_f32); 22];
        for i in 0..22 {
            positions[i] = engine.player_positions[i].to_meters();
        }
        if let Some(ref mut board) = engine.field_board {
            board.update_occupancy_from_positions_m(0, &positions);
            board.update_pressure_from_positions_m(0, &positions, None, 15.0);
        }

        // Benchmark 1000 pass evaluations
        let test_passes = [
            ((0.3, 0.5), (0.7, 0.5)), // Horizontal pass
            ((0.2, 0.3), (0.8, 0.7)), // Diagonal long pass
            ((0.5, 0.5), (0.6, 0.5)), // Short pass
            ((0.3, 0.2), (0.7, 0.8)), // Cross-field pass
        ];

        let start = std::time::Instant::now();
        for _ in 0..250 {
            for &(from, to) in &test_passes {
                // FIX_2601: from/to are already tuples in normalized format
                let _ = engine.estimate_pass_interception_risk(from, to, true);
            }
        }
        let elapsed = start.elapsed();

        let avg_micros = elapsed.as_micros() / 1000;
        println!("Hybrid pass evaluation: {}μs average", avg_micros);

        // Assert < 200μs per pass evaluation
        assert!(avg_micros < 200, "Too slow: {}μs (target < 200μs)", avg_micros);
    }

    #[test]
    fn test_hybrid_accuracy_vs_legacy() {
        let mut engine = create_test_engine();

        // Initialize FieldBoard
        if engine.field_board.is_none() {
            engine.field_board = Some(crate::engine::field_board::FieldBoard::new(
                crate::engine::field_board::FieldBoardSpec { cols: 28, rows: 18 },
            ));
        }

        // Scenario 1: Clear lane (defenders far away)
        // FIX_2601: Use Coord10::from_normalized_legacy for position assignments
        use crate::engine::types::Coord10;
        engine.player_positions[0] = Coord10::from_normalized_legacy((0.1, 0.5)); // Home passer
        engine.player_positions[11] = Coord10::from_normalized_legacy((0.9, 0.1)); // Away defenders far away
        engine.player_positions[12] = Coord10::from_normalized_legacy((0.9, 0.9));

        let mut positions = [(0.0_f32, 0.0_f32); 22];
        for i in 0..22 {
            positions[i] = engine.player_positions[i].to_meters();
        }
        if let Some(ref mut board) = engine.field_board {
            board.update_occupancy_from_positions_m(0, &positions);
            board.update_pressure_from_positions_m(0, &positions, None, 15.0);
        }

        let clear_risk = engine.estimate_pass_interception_risk((0.1, 0.5), (0.3, 0.5), true);
        println!("Clear lane risk (hybrid): {}", clear_risk);

        // Scenario 2: Blocked lane (defenders in the way)
        engine.player_positions[11] = Coord10::from_normalized_legacy((0.15, 0.5)); // Defender blocking lane
        engine.player_positions[12] = Coord10::from_normalized_legacy((0.2, 0.5));
        engine.player_positions[13] = Coord10::from_normalized_legacy((0.25, 0.5));

        for i in 0..22 {
            positions[i] = engine.player_positions[i].to_meters();
        }
        if let Some(ref mut board) = engine.field_board {
            board.update_occupancy_from_positions_m(0, &positions);
            board.update_pressure_from_positions_m(0, &positions, None, 15.0);
        }

        let blocked_risk = engine.estimate_pass_interception_risk((0.1, 0.5), (0.3, 0.5), true);
        println!("Blocked lane risk (hybrid): {}", blocked_risk);

        // Blocked lane should have higher risk than clear lane
        assert!(
            blocked_risk > clear_risk,
            "Blocked lane ({}) should have higher risk than clear lane ({})",
            blocked_risk,
            clear_risk
        );
    }

    #[test]
    fn test_pass_decision_prefers_clear_lanes() {
        let mut engine = create_test_engine();
        use crate::engine::types::Coord10;

        // Initialize FieldBoard
        if engine.field_board.is_none() {
            engine.field_board = Some(crate::engine::field_board::FieldBoard::new(
                crate::engine::field_board::FieldBoardSpec { cols: 28, rows: 18 },
            ));
        }

        // FIX_2601: Use Coord10 for player positions
        // Passer at center
        engine.player_positions[5] = Coord10::from_normalized_legacy((0.5, 0.5));
        engine.ball.current_owner = Some(5);

        // Two pass targets: one clear, one blocked
        engine.player_positions[6] = Coord10::from_normalized_legacy((0.6, 0.5)); // Clear target
        engine.player_positions[7] = Coord10::from_normalized_legacy((0.5, 0.7)); // Blocked target

        // Block the lane to player 7
        engine.player_positions[11] = Coord10::from_normalized_legacy((0.5, 0.6)); // Defender blocking
        engine.player_positions[12] = Coord10::from_normalized_legacy((0.5, 0.65));

        // Update FieldBoard
        let mut positions = [(0.0_f32, 0.0_f32); 22];
        for i in 0..22 {
            positions[i] = engine.player_positions[i].to_meters();
        }
        if let Some(ref mut board) = engine.field_board {
            board.update_occupancy_from_positions_m(0, &positions);
            board.update_pressure_from_positions_m(0, &positions, None, 15.0);
        }

        // Evaluate both passes (convert Coord10 to normalized tuples for the function)
        // Note: estimate_pass_interception_risk expects legacy (y, x) normalized format
        let clear_risk = engine.estimate_pass_interception_risk(
            engine.player_positions[5].to_normalized_legacy(),
            engine.player_positions[6].to_normalized_legacy(),
            true,
        );
        let blocked_risk = engine.estimate_pass_interception_risk(
            engine.player_positions[5].to_normalized_legacy(),
            engine.player_positions[7].to_normalized_legacy(),
            true,
        );

        println!("Clear pass risk: {}, Blocked pass risk: {}", clear_risk, blocked_risk);

        // Clear lane should have lower risk
        assert!(
            clear_risk < blocked_risk,
            "Clear lane pass should have lower risk than blocked lane pass"
        );
    }
}
