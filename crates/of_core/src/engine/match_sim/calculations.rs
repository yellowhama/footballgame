//! Statistical Calculations for MatchEngine
//!
//! This module contains calculation functions for team strength, possession,
//! xG, pass success, and other statistical computations.
//! Extracted from mod.rs as part of P2-9 refactoring.

use super::super::{
    opponent_analysis::OpponentAnalysis, physics_constants, probability,
    tactical_context::TacticalContext,
};
use super::{instruction_strength_multiplier, MatchEngine};
use crate::models::trait_system::TraitId;
use crate::models::TeamSide;

/// Advanced Pressure Context - 압박 상황 종합 정보
/// 수비수의 거리, 각도, 선수 침착성을 종합하여 압박 지수 계산
#[derive(Debug, Clone, Default)]
pub struct PressureContext {
    /// 가장 가까운 수비수와의 거리 (meters)
    pub nearest_defender_distance: f32,
    /// 수비수의 상대 각도 (dot product: 1.0=정면, 0.0=측면, -1.0=후방)
    pub defender_angle: f32,
    /// 압박 반경 내 수비수 수
    pub defenders_in_radius: u8,
    /// 원시 압박 지수 (0.0-1.0, 침착성 미적용)
    pub raw_pressure: f32,
    /// 침착성 적용 후 유효 압박 지수
    pub effective_pressure: f32,
    /// 압박 상황 판정
    pub pressure_level: PressureLevel,
}

/// 압박 수준 분류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PressureLevel {
    #[default]
    None, // 압박 없음 (자유로운 상태)
    Light,    // 가벼운 압박 (여유 있음)
    Moderate, // 중간 압박
    Heavy,    // 강한 압박
    Extreme,  // 극심한 압박 (정면 밀착)
}

// ==========================================================================
// FIX_2601/0109: Pressure Response Methods (Open-Football Style)
// ==========================================================================

impl PressureContext {
    /// Open-Football: Should player release ball immediately?
    ///
    /// Returns true when:
    /// - 2+ defenders are within pressure radius, OR
    /// - Effective pressure exceeds 60%
    ///
    /// Used to force dribbling → passing transition under heavy pressure
    pub fn should_release_ball(&self) -> bool {
        self.defenders_in_radius >= 2 || self.effective_pressure > 0.6
    }

    /// Is player under heavy or extreme pressure?
    pub fn is_heavily_pressured(&self) -> bool {
        matches!(self.pressure_level, PressureLevel::Heavy | PressureLevel::Extreme)
    }

    /// Get pressure penalty for action success (0.0-0.3 range)
    pub fn action_penalty(&self) -> f32 {
        match self.pressure_level {
            PressureLevel::None => 0.0,
            PressureLevel::Light => 0.05,
            PressureLevel::Moderate => 0.10,
            PressureLevel::Heavy => 0.20,
            PressureLevel::Extreme => 0.30,
        }
    }
}

impl MatchEngine {
    /// Calculate team strength with instruction modifiers
    pub(crate) fn calculate_team_strength(&self, team: &crate::models::Team, is_home: bool) -> f32 {
        let mut strength = team.average_overall();

        // Apply base instruction multiplier
        let instruction_multiplier = if is_home {
            instruction_strength_multiplier(&self.home_instructions)
        } else {
            instruction_strength_multiplier(&self.away_instructions)
        };
        strength *= instruction_multiplier;

        // Apply counter-tactics bonus
        let counter_bonus = if is_home {
            OpponentAnalysis::calculate_counter_bonus(
                &self.home_instructions,
                &self.away_instructions,
            )
        } else {
            OpponentAnalysis::calculate_counter_bonus(
                &self.away_instructions,
                &self.home_instructions,
            )
        };
        strength *= counter_bonus;

        // Apply home advantage (5% strength boost)
        if is_home {
            strength *= physics_constants::home_advantage::STRENGTH_MULTIPLIER;
        }
        strength
    }

    /// Get tactical context for event generation
    pub(crate) fn get_tactical_context(&self) -> TacticalContext {
        TacticalContext::new(self.home_instructions.clone(), self.away_instructions.clone())
    }

    /// Calculate possession ratio based on team strengths
    pub(crate) fn calculate_possession(&mut self, home_strength: f32, away_strength: f32) -> f32 {
        use rand::Rng;
        // Base possession on team strengths with some randomness
        let base_ratio = home_strength / (home_strength + away_strength);
        let randomness = (self.rng.gen::<f32>() - 0.5) * 0.1; // ±5% random factor

        (base_ratio + randomness).clamp(0.3, 0.7) // Clamp between 30-70%
    }

    /// Calculate expected goals (xG)
    pub(crate) fn calculate_xg(&mut self, attack_strength: f32, defense_strength: f32) -> f32 {
        use rand::Rng;
        // Base xG calculation
        let strength_ratio = attack_strength / (attack_strength + defense_strength);
        let base_xg = 0.05 + (strength_ratio - 0.5) * 0.15; // 0.05 to 0.20 range

        // Add some randomness
        let random_factor = self.rng.gen::<f32>() * 0.1;

        (base_xg + random_factor).clamp(0.01, 0.5)
    }

    /// 선수의 슈팅 확률 계산 (위치 기반) - probability 모듈 래퍼
    pub(crate) fn calculate_shooting_probability(&self, player_idx: usize) -> f32 {
        let player_pos = self.get_player_position_by_index(player_idx);
        let is_home = TeamSide::is_home(player_idx);
        // FIX_2601/0109: Use attacks_right for correct second-half goal direction
        let attacks_right = self.attacks_right(is_home);
        probability::shooting_probability(player_pos.to_normalized_legacy(), attacks_right)
    }

    /// 선수의 패스 성공 확률 계산 (Open Football 방식)
    pub(crate) fn calculate_pass_success(&self, from_idx: usize, to_idx: usize) -> f32 {
        let from_pos = self.get_player_position_by_index(from_idx);
        let to_pos = self.get_player_position_by_index(to_idx);
        let distance_m = from_pos.distance_to_m(&to_pos);

        // 선수 패싱 속성
        let passing = physics_constants::skills::normalize(self.get_player_passing(from_idx));
        let vision = physics_constants::skills::normalize(self.get_player_vision(from_idx));
        let technique = physics_constants::skills::normalize(self.get_player_technique(from_idx));

        // 거리 기반 점수 (Open Football 방식)
        let distance_score = if distance_m < physics_constants::pass::VERY_SHORT_M {
            0.6 + 0.3 * (distance_m / physics_constants::pass::VERY_SHORT_M)
        } else if distance_m < physics_constants::pass::SHORT_M {
            0.9 - 0.1 * (1.0 - (distance_m - physics_constants::pass::VERY_SHORT_M) / 15.0)
        } else if distance_m < physics_constants::pass::OPTIMAL_MAX_M {
            1.0 // 최적
        } else if distance_m < physics_constants::pass::LONG_M {
            0.9 - 0.4 * ((distance_m - physics_constants::pass::OPTIMAL_MAX_M) / 40.0)
        } else {
            0.3 // 매우 김
        };

        // A6: 포지션별 가중치 적용
        let positional_modifier = self.get_positional_skill_rating(from_idx, "passing");

        // 스킬 기반 성공률 (기존 70% + 포지션별 30%)
        let base_skill = passing * 0.5 + vision * 0.3 + technique * 0.2;
        let skill_factor = base_skill * 0.7 + positional_modifier * 0.3;

        // 압박 체크
        let pressure_penalty = self.calculate_pressure_penalty(from_idx);

        // 인터셉트 위험도
        let interception_risk = self.check_interception_risk(from_idx, to_idx);

        // 기본 성공률
        let base_success = distance_score * (0.6 + skill_factor * 0.4);

        // 페널티 적용
        let mut success_rate = base_success - pressure_penalty - interception_risk;

        // Gold Traits 적용
        let is_long_pass = distance_m > 30.0;
        if is_long_pass && self.player_has_gold_trait(from_idx, TraitId::Architect) {
            success_rate = (success_rate + 0.15).min(0.95); // 롱패스 +15%
        }
        if self.player_has_gold_trait(from_idx, TraitId::Maestro) {
            success_rate = (success_rate + 0.10).min(0.95); // 마에스트로 +10%
        }

        // 홈 어드밴티지
        let is_home = TeamSide::is_home(from_idx);
        let success_rate = if is_home {
            (success_rate + physics_constants::home_advantage::PASS_SUCCESS_BONUS).min(0.95)
        } else {
            success_rate
        };

        success_rate.clamp(0.15, 0.95)
    }

    // ========================================================================
    // Advanced Pressure System (2025-12-07)
    // 수비수의 거리와 각도를 종합한 압박 지수 계산
    // ========================================================================

    /// 종합 압박 상황 계산 - PressureContext 반환
    /// 거리, 각도, 침착성을 종합하여 압박 지수 산출
    ///
    /// # DEPRECATED (P18)
    ///
    /// **This function performs O(22) distance calculations.**
    ///
    /// Replaced by O(1) FieldBoard lookup in `build_p16_decision_context()`.
    /// Only used as fallback when FieldBoard is None.
    ///
    /// ## Migration
    /// Instead of:
    /// ```ignore
    /// let pressure_ctx = self.calculate_pressure_context(player_idx, None);
    /// let pressure = pressure_ctx.effective_pressure;
    /// ```
    ///
    /// Use:
    /// ```ignore
    /// let pressure = self.get_local_pressure(player_idx);  // O(1) FieldBoard lookup
    /// ```
    ///
    /// See: `ev_decision.rs::get_local_pressure()` and `ev_decision.rs::get_local_pressure_level()`
    #[deprecated(
        since = "0.2.0",
        note = "Use FieldBoard local_pressure instead (P18). O(22) → O(1) optimization."
    )]
    pub(crate) fn calculate_pressure_context(
        &self,
        player_idx: usize,
        movement_direction: Option<(f32, f32)>,
    ) -> PressureContext {
        use physics_constants::pressure;

        let player_pos = self.get_player_position_by_index(player_idx);
        let _is_home = TeamSide::is_home(player_idx);
        let opponent_range = TeamSide::opponent_range(player_idx);

        // 선수 침착성 (0.0-1.0)
        let composure = physics_constants::skills::normalize(self.get_player_composure(player_idx));

        let mut nearest_distance = f32::MAX;
        let mut nearest_defender_idx: Option<usize> = None;
        let mut defenders_in_radius: u8 = 0;
        let mut total_pressure: f32 = 0.0;

        // 모든 상대 수비수 검사
        for opp_idx in opponent_range {
            let opp_pos = self.get_player_position_by_index(opp_idx);
            let distance = player_pos.distance_to_m(&opp_pos);

            // 확장 반경 내 수비수 체크
            if distance < pressure::RADIUS_EXTENDED_M {
                defenders_in_radius += 1;

                // 거리 기반 압박 지수 (선형 감쇠)
                let distance_pressure = self.calculate_distance_pressure(distance);
                total_pressure += distance_pressure;

                if distance < nearest_distance {
                    nearest_distance = distance;
                    nearest_defender_idx = Some(opp_idx);
                }
            }
        }

        // 가장 가까운 수비수가 없으면 압박 없음
        if nearest_defender_idx.is_none() {
            return PressureContext {
                nearest_defender_distance: f32::MAX,
                defender_angle: 0.0,
                defenders_in_radius: 0,
                raw_pressure: 0.0,
                effective_pressure: 0.0,
                pressure_level: PressureLevel::None,
            };
        }

        let nearest_idx = nearest_defender_idx.unwrap();

        // 수비수 각도 계산 (이동 방향 기준)
        let defender_angle =
            self.calculate_defender_angle(player_idx, nearest_idx, movement_direction);

        // 각도 가중치 적용 (정면일수록 압박 증가)
        let angle_multiplier = self.calculate_angle_pressure_multiplier(defender_angle);
        let raw_pressure = (total_pressure * angle_multiplier).clamp(0.0, 1.0);

        // 침착성으로 압박 완화
        let mitigation = pressure::COMPOSURE_MIN_MITIGATION
            + composure * (pressure::COMPOSURE_MAX_MITIGATION - pressure::COMPOSURE_MIN_MITIGATION);
        let effective_pressure = (raw_pressure - mitigation).max(0.0);

        // 압박 수준 판정
        let pressure_level = self.classify_pressure_level(effective_pressure, defender_angle);

        PressureContext {
            nearest_defender_distance: nearest_distance,
            defender_angle,
            defenders_in_radius,
            raw_pressure,
            effective_pressure,
            pressure_level,
        }
    }

    /// 거리 기반 압박 지수 계산 (0.0-1.0)
    /// RADIUS_TIGHT_M 이하: 1.0 (최대 압박)
    /// RADIUS_EXTENDED_M 이상: 0.0 (압박 없음)
    fn calculate_distance_pressure(&self, distance: f32) -> f32 {
        use physics_constants::pressure;

        if distance <= pressure::RADIUS_TIGHT_M {
            1.0
        } else if distance >= pressure::RADIUS_EXTENDED_M {
            0.0
        } else {
            // 선형 감쇠: tight에서 extended까지
            let range = pressure::RADIUS_EXTENDED_M - pressure::RADIUS_TIGHT_M;
            1.0 - (distance - pressure::RADIUS_TIGHT_M) / range
        }
    }

    /// 수비수 각도 계산 (dot product)
    /// 반환값: 1.0 = 정면 차단, 0.0 = 측면, -1.0 = 후방 추격
    fn calculate_defender_angle(
        &self,
        player_idx: usize,
        defender_idx: usize,
        movement_direction: Option<(f32, f32)>,
    ) -> f32 {
        let player_pos = self.get_player_position_by_index(player_idx);
        let defender_pos = self.get_player_position_by_index(defender_idx);

        // FIX_2601: Coord10 → meters for vector calculation
        let player_pos_m = player_pos.to_meters();
        let defender_pos_m = defender_pos.to_meters();

        // 수비수 방향 벡터 (정규화)
        let to_defender = (defender_pos_m.0 - player_pos_m.0, defender_pos_m.1 - player_pos_m.1);
        let defender_dist = (to_defender.0.powi(2) + to_defender.1.powi(2)).sqrt();
        if defender_dist < 0.001 {
            return 1.0; // 거의 같은 위치 = 정면 차단
        }
        let defender_dir = (to_defender.0 / defender_dist, to_defender.1 / defender_dist);

        // 이동 방향 (제공되지 않으면 골문 방향 사용)
        let move_dir = match movement_direction {
            Some(dir) => {
                let len = (dir.0.powi(2) + dir.1.powi(2)).sqrt();
                if len < 0.001 {
                    self.get_goal_direction(player_idx)
                } else {
                    (dir.0 / len, dir.1 / len)
                }
            }
            None => self.get_goal_direction(player_idx),
        };

        // Dot product: 이동방향과 수비수방향의 내적
        move_dir.0 * defender_dir.0 + move_dir.1 * defender_dir.1
    }

    /// 골문 방향 벡터 반환 (정규화)
    fn get_goal_direction(&self, player_idx: usize) -> (f32, f32) {
        let player_pos_norm = self.get_player_position_by_index(player_idx);
        let player_pos_m = player_pos_norm.to_meters();

        // 방향 스왑을 반영한 공격 골문 사용
        let dir_ctx = if TeamSide::is_home(player_idx) { &self.home_ctx } else { &self.away_ctx };
        let goal_x = dir_ctx.opponent_goal_x() * physics_constants::field::LENGTH_M;
        let goal_y = physics_constants::field::WIDTH_M / 2.0;

        let to_goal = (goal_x - player_pos_m.0, goal_y - player_pos_m.1);
        let dist = (to_goal.0.powi(2) + to_goal.1.powi(2)).sqrt();
        if dist < 0.001 {
            (dir_ctx.attack_direction(), 0.0)
        } else {
            (to_goal.0 / dist, to_goal.1 / dist)
        }
    }

    /// 각도에 따른 압박 배율 계산
    /// 정면(1.0): 1.5배 압박
    /// 측면(0.0): 1.0배 (기본)
    /// 후방(-1.0): 0.5배 (추격 상황 = 유리)
    fn calculate_angle_pressure_multiplier(&self, angle: f32) -> f32 {
        use physics_constants::pressure;

        if angle > pressure::ANGLE_FRONT_BLOCK {
            // 정면 차단: 압박 증가
            1.0 + (angle - pressure::ANGLE_FRONT_BLOCK) * 1.0
        } else if angle < pressure::ANGLE_BEHIND_CHASE {
            // 후방 추격: 압박 감소
            0.5 + (angle - (-1.0)) * 0.25
        } else {
            // 측면: 기본 압박
            1.0
        }
    }

    /// 압박 수준 분류
    fn classify_pressure_level(&self, effective_pressure: f32, angle: f32) -> PressureLevel {
        use physics_constants::pressure;

        if effective_pressure < 0.1 {
            PressureLevel::None
        } else if effective_pressure < 0.3 {
            PressureLevel::Light
        } else if effective_pressure < 0.5 {
            PressureLevel::Moderate
        } else if effective_pressure < 0.7 || angle < pressure::ANGLE_FRONT_BLOCK {
            PressureLevel::Heavy
        } else {
            PressureLevel::Extreme
        }
    }

    /// 드리블 성공률 수정자 계산 (압박 상황 기반)
    /// 정면 차단: 큰 페널티, 후방 추격: 보너스
    pub(crate) fn calculate_dribble_pressure_modifier(&self, ctx: &PressureContext) -> f32 {
        use physics_constants::pressure;

        match ctx.pressure_level {
            PressureLevel::None => 1.0,
            PressureLevel::Light => 0.95,
            PressureLevel::Moderate => 0.85,
            PressureLevel::Heavy => {
                if ctx.defender_angle > pressure::ANGLE_FRONT_BLOCK {
                    // 정면 차단: 심각한 페널티
                    1.0 - pressure::DRIBBLE_FRONT_PENALTY * ctx.effective_pressure
                } else if ctx.defender_angle < pressure::ANGLE_BEHIND_CHASE {
                    // 후방 추격: 보너스 (도주 유리)
                    1.0 + (pressure::DRIBBLE_CHASE_BONUS - 1.0) * 0.3
                } else {
                    0.7
                }
            }
            PressureLevel::Extreme => {
                // 극심한 압박: 드리블 거의 불가능
                0.3
            }
        }
    }

    /// 슛 성공률 수정자 계산 (압박 상황 기반)
    pub(crate) fn calculate_shot_pressure_modifier(&self, ctx: &PressureContext) -> f32 {
        use physics_constants::pressure;

        if ctx.pressure_level == PressureLevel::None {
            return 1.0;
        }

        // 정면 차단 시 슛 블록 위험 증가
        let block_penalty = if ctx.defender_angle > pressure::ANGLE_FRONT_BLOCK {
            pressure::SHOT_PRESSURE_PENALTY * ctx.effective_pressure
        } else {
            0.0
        };

        (1.0 - ctx.effective_pressure * 0.3 - block_penalty).clamp(0.3, 1.0)
    }

    /// 패스 성공률 수정자 계산 (압박 상황 기반)
    pub(crate) fn calculate_pass_pressure_modifier(&self, ctx: &PressureContext) -> f32 {
        use physics_constants::pressure;

        match ctx.pressure_level {
            PressureLevel::None => 1.0,
            PressureLevel::Light => 0.95,
            PressureLevel::Moderate => 0.85,
            PressureLevel::Heavy | PressureLevel::Extreme => {
                // 강한 압박 시 패스 정확도 감소
                (1.0 - pressure::PASS_TIGHT_PENALTY * ctx.effective_pressure).clamp(0.5, 0.8)
            }
        }
    }

    // ========================================================================
    // End of Advanced Pressure System
    // ========================================================================
    // Note: get_player_composure is defined in player_attributes.rs

    /// 압박 페널티 계산 - probability 모듈 래퍼
    ///
    /// TODO(FIX_2601/D-4): Migrate to FieldBoard + attribute_calc::pressure_penalty()
    /// Prefer `ev_decision::get_local_pressure_level()` + `attribute_calc::pressure_penalty()`
    #[allow(deprecated)] // probability::pressure_penalty - see D-4 migration plan
    pub(crate) fn calculate_pressure_penalty(&self, player_idx: usize) -> f32 {
        let player_pos = self.get_player_position_by_index(player_idx);
        let opponent_range = TeamSide::opponent_range(player_idx);
        // FIX_2601: Convert Coord10 to normalized tuples
        let opponent_positions: Vec<(f32, f32)> = opponent_range
            .map(|i| self.get_player_position_by_index(i).to_normalized_legacy())
            .collect();
        probability::pressure_penalty(player_pos.to_normalized_legacy(), &opponent_positions)
    }

    /// 인터셉트 위험도 계산 - probability 모듈 래퍼
    pub(crate) fn check_interception_risk(&self, from_idx: usize, to_idx: usize) -> f32 {
        let from_pos = self.get_player_position_by_index(from_idx);
        let to_pos = self.get_player_position_by_index(to_idx);
        let opponent_range = TeamSide::opponent_range(from_idx);
        // FIX_2601: Convert Coord10 to normalized tuples
        let opponent_positions: Vec<(f32, f32)> = opponent_range
            .map(|i| self.get_player_position_by_index(i).to_normalized_legacy())
            .collect();
        probability::interception_risk(
            from_pos.to_normalized_legacy(),
            to_pos.to_normalized_legacy(),
            &opponent_positions,
        )
    }

    /// 공과의 거리로 이벤트 참여 가중치 계산 - probability 모듈 래퍼
    pub(crate) fn calculate_involvement_weight(&self, player_idx: usize) -> f32 {
        let player_pos = self.get_player_position_by_index(player_idx);
        // FIX_2601/0104: Use consistent coordinate format (width, length)
        let ball_pos_norm = self.ball.position.to_normalized_legacy();
        probability::involvement_weight(player_pos.to_normalized_legacy(), ball_pos_norm)
    }
}
