//! # Execution Error System (P10-13 Phase 2)
//!
//! **Intent + Error = Actual Result**
//!
//! 능력치 + 압박 + 피로 + 약발 → 오차 (각도/거리/높이)
//!
//! ## 기본 원리
//! - 모든 액션은 "의도(Intent)"와 "실제 결과(Result)"가 다를 수 있음
//! - 오차는 정규분포를 따르며, sigma는 상황에 따라 변동
//! - 높은 능력치, 낮은 압박, 낮은 피로 → 작은 오차

use rand::Rng;
use rand_distr::{Distribution, StandardNormal};

/// 실행 오차 (모든 액션에 적용)
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionError {
    /// 방향 오차 (degrees)
    /// 양수 = 시계 방향, 음수 = 반시계 방향
    pub dir_angle_deg: f32,

    /// 거리 배율
    /// 1.0 = 정확, >1.0 = 길게, <1.0 = 짧게
    pub dist_factor: f32,

    /// 높이 배율 (슛/크로스용)
    /// 1.0 = 정확, >1.0 = 높게, <1.0 = 낮게
    pub height_factor: f32,
}

impl ExecutionError {
    /// 오차 없음 (완벽한 실행)
    pub fn zero() -> Self {
        Self { dir_angle_deg: 0.0, dist_factor: 1.0, height_factor: 1.0 }
    }

    /// 오차 크기 (정규화된 magnitude)
    /// 30도/30% 기준으로 정규화
    pub fn magnitude(&self) -> f32 {
        let angle_norm = self.dir_angle_deg.abs() / 30.0;
        let dist_norm = (self.dist_factor - 1.0).abs() / 0.3;
        let height_norm = (self.height_factor - 1.0).abs() / 0.3;

        (angle_norm.powi(2) + dist_norm.powi(2) + height_norm.powi(2)).sqrt()
    }
}

/// 액션 종류별 오차 특성
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Pass,
    Shot,
    Cross,
    FirstTouch,
    DribbleTouch,
    Save,
}

/// 오차 계산을 위한 컨텍스트 (능력치 기반)
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 관련 기술 능력치 (0-100)
    pub tech_skill: u8,
    /// 침착함 (0-100)
    pub composure: u8,
    /// 판단력 (0-100)
    pub decisions: u8,
    /// 집중력 (0-100) - FIX_2601/0107: FM meta
    pub concentration: u8,
    /// 액션 종류
    pub action_kind: ActionKind,
    /// 압박 수준 (0.0 ~ 1.0)
    pub pressure: f32,
    /// 피로 수준 (0.0 ~ 1.0, 1.0 = 지침)
    pub fatigue: f32,
    /// FIX01 C1: condition/decision quality multiplier (1.0 = neutral)
    pub decision_quality_mult: f32,
    /// 약발 사용 여부
    pub weak_foot: bool,
}

impl ErrorContext {
    /// 능력치 기본값으로 컨텍스트 생성
    pub fn new(action_kind: ActionKind) -> Self {
        Self {
            tech_skill: 50,
            composure: 50,
            decisions: 50,
            concentration: 50,
            action_kind,
            pressure: 0.0,
            fatigue: 0.0,
            decision_quality_mult: 1.0,
            weak_foot: false,
        }
    }

    /// 능력치 설정 (기존 호환성)
    pub fn with_stats(mut self, tech: u8, composure: u8, decisions: u8) -> Self {
        self.tech_skill = tech;
        self.composure = composure;
        self.decisions = decisions;
        self
    }

    /// 능력치 설정 (FM meta 포함)
    /// FIX_2601/0107: concentration 추가
    pub fn with_stats_fm(
        mut self,
        tech: u8,
        composure: u8,
        decisions: u8,
        concentration: u8,
    ) -> Self {
        self.tech_skill = tech;
        self.composure = composure;
        self.decisions = decisions;
        self.concentration = concentration;
        self
    }

    /// 압박/피로 설정
    pub fn with_context(mut self, pressure: f32, fatigue: f32, weak_foot: bool) -> Self {
        self.pressure = pressure.clamp(0.0, 1.0);
        self.fatigue = fatigue.clamp(0.0, 1.0);
        self.weak_foot = weak_foot;
        self
    }

    pub fn with_decision_quality_mult(mut self, mult: f32) -> Self {
        self.decision_quality_mult = mult.clamp(0.5, 2.0);
        self
    }
}

// ========== Base Sigma Values (Tuning Points) ==========

mod base_sigma {
    use super::ActionKind;

    /// 기본 각도 오차 (degrees)
    /// v9: 전반적으로 50% 감소 - elite player가 더 정확해야 함
    pub fn angle(kind: ActionKind) -> f32 {
        match kind {
            ActionKind::Shot => 5.0,         // v9: 10 → 5
            ActionKind::Pass => 4.0,         // v9: 8 → 4
            ActionKind::Cross => 8.0,        // v9: 14 → 8
            ActionKind::FirstTouch => 3.0,   // v9: 6 → 3
            ActionKind::DribbleTouch => 3.0, // v9: 5 → 3
            ActionKind::Save => 4.0,         // v9: 8 → 4
        }
    }

    /// 기본 거리 오차 (배율 분산)
    /// v9: 전반적으로 40% 감소
    pub fn distance(kind: ActionKind) -> f32 {
        match kind {
            ActionKind::Shot => 0.10,         // v9: 0.18 → 0.10
            ActionKind::Pass => 0.08,         // v9: 0.12 → 0.08
            ActionKind::Cross => 0.12,        // v9: 0.20 → 0.12
            ActionKind::FirstTouch => 0.12,   // v9: 0.20 → 0.12
            ActionKind::DribbleTouch => 0.10, // v9: 0.15 → 0.10
            ActionKind::Save => 0.06,         // v9: 0.10 → 0.06
        }
    }

    /// 기본 높이 오차 (배율 분산)
    /// v9: 전반적으로 40% 감소
    pub fn height(kind: ActionKind) -> f32 {
        match kind {
            ActionKind::Shot => 0.15,         // v9: 0.25 → 0.15
            ActionKind::Cross => 0.15,        // v9: 0.25 → 0.15
            ActionKind::Pass => 0.05,         // v9: 0.08 → 0.05
            ActionKind::FirstTouch => 0.06,   // v9: 0.10 → 0.06
            ActionKind::DribbleTouch => 0.05, // v9: 0.08 → 0.05
            ActionKind::Save => 0.06,         // v9: 0.10 → 0.06
        }
    }
}

// ========== Core Sampling Function ==========

/// 실행 오차 샘플링
///
/// # Arguments
/// * `ctx` - 오차 계산 컨텍스트 (능력치, 압박, 피로 등)
/// * `rng` - 랜덤 생성기
///
/// # Returns
/// 방향/거리/높이 오차를 포함한 ExecutionError
///
/// # Example
/// ```ignore
/// let ctx = ErrorContext::new(ActionKind::Shot)
///     .with_stats(80, 75, 70)
///     .with_context(0.6, 0.3, false);
/// let error = sample_execution_error(&ctx, &mut rng);
/// ```
pub fn sample_execution_error(ctx: &ErrorContext, rng: &mut impl Rng) -> ExecutionError {
    // 1. Factor 계산 (0.0 ~ 1.0 범위)
    let skill_factor = (100.0 - ctx.tech_skill as f32) / 100.0; // 능력치 높으면 낮음
    let pressure_factor = ctx.pressure;
    let fatigue_factor = ctx.fatigue;
    let weak_foot_factor = if ctx.weak_foot { 1.0 } else { 0.0 };
    let decision_quality_mult = ctx.decision_quality_mult.clamp(0.5, 2.0);
    let decision_sigma_mult = (1.0 / decision_quality_mult).clamp(0.85, 1.20);

    // 침착함 + 판단력 → 압박 상황에서 오차 감소 효과
    let calm_factor = ((ctx.composure as f32 + ctx.decisions as f32) / 200.0).clamp(0.0, 1.0);

    // FIX_2601/0107: FM meta - concentration 기반 에러 보정
    #[cfg(feature = "fm_meta_attributes")]
    let concentration_error = {
        use crate::engine::match_sim::attribute_calc::concentration_error_modifier;
        concentration_error_modifier(ctx.concentration as f32, ctx.fatigue)
    };
    #[cfg(not(feature = "fm_meta_attributes"))]
    let concentration_error = 0.0;

    // 2. 최종 Sigma 계산
    // FM meta: concentration_error가 높으면 오차 증가 (최대 +15%)
    let concentration_mult = 1.0 + concentration_error;

    let angle_sigma = base_sigma::angle(ctx.action_kind)
        * (1.0
            + 0.8 * skill_factor
            + 0.7 * pressure_factor
            + 0.5 * fatigue_factor
            + 0.5 * weak_foot_factor)
        * (1.2 - 0.5 * calm_factor)
        * concentration_mult
        * decision_sigma_mult;

    let dist_sigma = base_sigma::distance(ctx.action_kind)
        * (1.0 + 0.5 * skill_factor + 0.4 * pressure_factor + 0.3 * fatigue_factor)
        * (1.2 - 0.4 * calm_factor)
        * concentration_mult
        * decision_sigma_mult;

    let height_sigma = base_sigma::height(ctx.action_kind)
        * (1.0 + 0.5 * skill_factor + 0.3 * pressure_factor)
        * (1.1 - 0.3 * calm_factor)
        * concentration_mult
        * decision_sigma_mult;

    // 3. 정규분포에서 샘플링
    let n_angle: f32 = StandardNormal.sample(rng);
    let n_dist: f32 = StandardNormal.sample(rng);
    let n_height: f32 = StandardNormal.sample(rng);

    ExecutionError {
        dir_angle_deg: n_angle * angle_sigma,
        dist_factor: 1.0 + n_dist * dist_sigma,
        height_factor: 1.0 + n_height * height_sigma,
    }
}

// ========== Error Application Functions ==========

/// 2D 목표점에 오차 적용
///
/// # Arguments
/// * `from` - 시작 위치 (미터)
/// * `intended` - 의도한 목표 위치 (미터)
/// * `err` - 적용할 오차
///
/// # Returns
/// 오차가 적용된 실제 목표 위치
pub fn apply_error_to_target(
    from: (f32, f32),
    intended: (f32, f32),
    err: &ExecutionError,
) -> (f32, f32) {
    let dir_x = intended.0 - from.0;
    let dir_y = intended.1 - from.1;
    let dist = (dir_x * dir_x + dir_y * dir_y).sqrt();

    if dist < 0.001 {
        return intended; // 거의 같은 위치
    }

    let norm_x = dir_x / dist;
    let norm_y = dir_y / dist;

    // 방향 오차 회전
    let rot_rad = err.dir_angle_deg.to_radians();
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();
    let rotated_x = norm_x * cos_r - norm_y * sin_r;
    let rotated_y = norm_x * sin_r + norm_y * cos_r;

    // 거리 오차 적용
    let final_dist = dist * err.dist_factor;

    (from.0 + rotated_x * final_dist, from.1 + rotated_y * final_dist)
}

/// 3D 슛 오차 적용
///
/// # Arguments
/// * `from` - 슛 시작 위치 (미터)
/// * `intended` - 의도한 목표 위치 (미터)
/// * `intended_height` - 의도한 높이 (미터)
/// * `err` - 적용할 오차
///
/// # Returns
/// (실제 목표 위치, 실제 높이)
pub fn apply_error_for_shot(
    from: (f32, f32),
    intended: (f32, f32),
    intended_height: f32,
    err: &ExecutionError,
) -> ((f32, f32), f32) {
    let actual_2d = apply_error_to_target(from, intended, err);
    let actual_height = (intended_height * err.height_factor).max(0.0);

    (actual_2d, actual_height)
}

/// 퍼스트 터치 품질
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstTouchQuality {
    /// < 0.5m - 완벽한 컨트롤
    Perfect,
    /// < 1.5m - 좋은 터치
    Good,
    /// < 2.5m - 무거운 터치, 압박 기회
    Heavy,
    /// >= 2.5m - 루즈볼, 탈취 가능
    Loose,
}

/// 퍼스트 터치 오차 적용
///
/// # Arguments
/// * `ball_pos` - 공 위치 (미터)
/// * `player_pos` - 선수 위치 (미터)
/// * `err` - 적용할 오차
///
/// # Returns
/// (공의 최종 위치, 터치 품질)
pub fn apply_error_for_first_touch(
    ball_pos: (f32, f32),
    player_pos: (f32, f32),
    err: &ExecutionError,
) -> ((f32, f32), FirstTouchQuality) {
    let dir_x = player_pos.0 - ball_pos.0;
    let dir_y = player_pos.1 - ball_pos.1;
    let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();

    let (norm_x, norm_y) = if dir_len > 0.001 {
        (dir_x / dir_len, dir_y / dir_len)
    } else {
        (1.0, 0.0) // 기본 방향
    };

    // 방향 오차 회전
    let rot_rad = err.dir_angle_deg.to_radians();
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();
    let bounce_x = norm_x * cos_r - norm_y * sin_r;
    let bounce_y = norm_x * sin_r + norm_y * cos_r;

    // 거리 오차 → 공이 얼마나 멀리 튀는지
    // dist_factor 1.0 → 정확히 발 밑 (0m)
    // dist_factor 1.5 → ~2m 튐
    let control_dist = (err.dist_factor - 1.0).abs() * 4.0; // 0 ~ 4m

    let final_ball_pos =
        (player_pos.0 + bounce_x * control_dist, player_pos.1 + bounce_y * control_dist);

    let quality = if control_dist < 0.5 {
        FirstTouchQuality::Perfect
    } else if control_dist < 1.5 {
        FirstTouchQuality::Good
    } else if control_dist < 2.5 {
        FirstTouchQuality::Heavy
    } else {
        FirstTouchQuality::Loose
    };

    (final_ball_pos, quality)
}

// ========== Weak Foot Detection ==========

/// 공이 선수의 어느 쪽에 있는지
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Center,
}

/// 선호 발
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferredFoot {
    Right,
    Left,
    Both,
}

/// 간단한 약발 감지
///
/// # Arguments
/// * `preferred` - 선수의 선호 발
/// * `ball_side` - 공이 선수의 왼쪽/오른쪽
///
/// # Returns
/// 약발 사용 여부
pub fn is_weak_foot(preferred: PreferredFoot, ball_side: Side) -> bool {
    match preferred {
        PreferredFoot::Both => false, // 양발잡이는 약발 없음
        PreferredFoot::Right => ball_side == Side::Left,
        PreferredFoot::Left => ball_side == Side::Right,
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn make_test_context(tech: u8, composure: u8, kind: ActionKind) -> ErrorContext {
        ErrorContext::new(kind).with_stats(tech, composure, composure)
    }

    #[test]
    fn test_execution_error_zero() {
        let err = ExecutionError::zero();
        assert_eq!(err.dir_angle_deg, 0.0);
        assert_eq!(err.dist_factor, 1.0);
        assert_eq!(err.height_factor, 1.0);
        assert!(err.magnitude() < 0.001);
    }

    #[test]
    fn test_error_distribution_elite() {
        // 능력치 100, 압박 0, 피로 0 → 오차 최소
        let ctx = make_test_context(100, 100, ActionKind::Shot);

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let errors: Vec<_> = (0..1000).map(|_| sample_execution_error(&ctx, &mut rng)).collect();

        let avg_angle = errors.iter().map(|e| e.dir_angle_deg.abs()).sum::<f32>() / 1000.0;
        let avg_dist = errors.iter().map(|e| (e.dist_factor - 1.0).abs()).sum::<f32>() / 1000.0;

        // Elite player: 평균 각도 오차 < 6도, 거리 오차 < 12%
        assert!(avg_angle < 6.0, "Elite angle error too high: {}", avg_angle);
        assert!(avg_dist < 0.12, "Elite dist error too high: {}", avg_dist);
    }

    #[test]
    fn test_error_distribution_under_pressure() {
        // 능력치 30, 압박 1.0, 피로 0.8, 약발 → 오차 큼
        let ctx = make_test_context(30, 30, ActionKind::Shot).with_context(1.0, 0.8, true);

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let errors: Vec<_> = (0..1000).map(|_| sample_execution_error(&ctx, &mut rng)).collect();

        let avg_angle = errors.iter().map(|e| e.dir_angle_deg.abs()).sum::<f32>() / 1000.0;

        // Under pressure: 평균 각도 오차 > 12도
        assert!(avg_angle > 12.0, "Pressured angle error too low: {}", avg_angle);
    }

    #[test]
    fn test_pressure_increases_error() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        // 압박 없음
        let ctx_calm = make_test_context(60, 60, ActionKind::Pass).with_context(0.0, 0.0, false);
        let errors_calm: Vec<_> =
            (0..500).map(|_| sample_execution_error(&ctx_calm, &mut rng)).collect();
        let avg_calm = errors_calm.iter().map(|e| e.dir_angle_deg.abs()).sum::<f32>() / 500.0;

        // 높은 압박
        let ctx_pressed = make_test_context(60, 60, ActionKind::Pass).with_context(1.0, 0.0, false);
        let errors_pressed: Vec<_> =
            (0..500).map(|_| sample_execution_error(&ctx_pressed, &mut rng)).collect();
        let avg_pressed = errors_pressed.iter().map(|e| e.dir_angle_deg.abs()).sum::<f32>() / 500.0;

        assert!(
            avg_pressed > avg_calm * 1.3,
            "Pressure should increase error: calm={}, pressed={}",
            avg_calm,
            avg_pressed
        );
    }

    #[test]
    fn test_fatigue_increases_error() {
        let mut rng = ChaCha8Rng::seed_from_u64(123);

        // 피로 없음
        let ctx_fresh = make_test_context(60, 60, ActionKind::Shot).with_context(0.0, 0.0, false);
        let errors_fresh: Vec<_> =
            (0..500).map(|_| sample_execution_error(&ctx_fresh, &mut rng)).collect();
        let avg_fresh = errors_fresh.iter().map(|e| e.dir_angle_deg.abs()).sum::<f32>() / 500.0;

        // 높은 피로
        let ctx_tired = make_test_context(60, 60, ActionKind::Shot).with_context(0.0, 1.0, false);
        let errors_tired: Vec<_> =
            (0..500).map(|_| sample_execution_error(&ctx_tired, &mut rng)).collect();
        let avg_tired = errors_tired.iter().map(|e| e.dir_angle_deg.abs()).sum::<f32>() / 500.0;

        assert!(
            avg_tired > avg_fresh * 1.2,
            "Fatigue should increase error: fresh={}, tired={}",
            avg_fresh,
            avg_tired
        );
    }

    #[test]
    fn test_decision_quality_mult_scales_error() {
        let base = make_test_context(60, 60, ActionKind::Pass).with_context(0.4, 0.2, false);

        let mut rng_good = ChaCha8Rng::seed_from_u64(999);
        let mut rng_bad = ChaCha8Rng::seed_from_u64(999);

        let err_good = sample_execution_error(&base.clone().with_decision_quality_mult(1.05), &mut rng_good);
        let err_bad = sample_execution_error(&base.clone().with_decision_quality_mult(0.92), &mut rng_bad);

        let mag_good = err_good.magnitude();
        let mag_bad = err_bad.magnitude();
        assert!(mag_good > 0.0);
        assert!(mag_bad > mag_good);

        let sigma_bad = (1.0_f32 / 0.92_f32).clamp(0.85, 1.20);
        let sigma_good = (1.0_f32 / 1.05_f32).clamp(0.85, 1.20);
        let expected = sigma_bad / sigma_good;
        let got = mag_bad / mag_good;
        assert!(
            (got - expected).abs() < 1e-4,
            "expected magnitude ratio ≈ {expected}, got {got} (good={mag_good}, bad={mag_bad})"
        );
    }

    #[test]
    fn test_apply_error_to_target_zero() {
        let from = (0.0, 0.0);
        let intended = (20.0, 0.0);
        let err = ExecutionError::zero();

        let actual = apply_error_to_target(from, intended, &err);

        assert!((actual.0 - 20.0).abs() < 0.001);
        assert!((actual.1 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_apply_error_to_target_rotation() {
        let from = (0.0, 0.0);
        let intended = (20.0, 0.0);

        // 10도 회전 + 10% 길게
        let err = ExecutionError { dir_angle_deg: 10.0, dist_factor: 1.1, height_factor: 1.0 };

        let actual = apply_error_to_target(from, intended, &err);

        assert!(actual.0 > 20.0, "Distance should be longer"); // 길어짐
        assert!(actual.1 > 0.0, "Should rotate clockwise"); // 시계방향
    }

    #[test]
    fn test_apply_error_for_shot_3d() {
        let from = (80.0, field::CENTER_Y);
        let intended = (field::LENGTH_M, field::CENTER_Y); // 골문 중앙
        let intended_height = 1.0; // 1m 높이

        let err = ExecutionError { dir_angle_deg: 5.0, dist_factor: 0.95, height_factor: 1.2 };

        let (actual_2d, actual_height) =
            apply_error_for_shot(from, intended, intended_height, &err);

        // 2D 오차 확인
        assert!(actual_2d.1 != intended.1, "Should have angle deviation");
        // 높이 오차 확인
        assert!((actual_height - 1.2).abs() < 0.001, "Height should be 1.2m");
    }

    #[test]
    fn test_first_touch_quality() {
        let ball = (10.0, 10.0);
        let player = (11.0, 10.0);

        // Perfect touch
        let err = ExecutionError { dir_angle_deg: 0.0, dist_factor: 1.0, height_factor: 1.0 };
        let (_, quality) = apply_error_for_first_touch(ball, player, &err);
        assert_eq!(quality, FirstTouchQuality::Perfect);

        // Heavy touch
        let err = ExecutionError { dir_angle_deg: 0.0, dist_factor: 1.5, height_factor: 1.0 };
        let (_, quality) = apply_error_for_first_touch(ball, player, &err);
        assert_eq!(quality, FirstTouchQuality::Heavy);

        // Loose ball
        let err = ExecutionError { dir_angle_deg: 0.0, dist_factor: 1.8, height_factor: 1.0 };
        let (_, quality) = apply_error_for_first_touch(ball, player, &err);
        assert_eq!(quality, FirstTouchQuality::Loose);
    }

    #[test]
    fn test_weak_foot_detection() {
        // 오른발 선수, 공이 왼쪽 → 약발
        assert!(is_weak_foot(PreferredFoot::Right, Side::Left));
        // 오른발 선수, 공이 오른쪽 → 주발
        assert!(!is_weak_foot(PreferredFoot::Right, Side::Right));
        // 양발잡이 → 약발 없음
        assert!(!is_weak_foot(PreferredFoot::Both, Side::Left));
        assert!(!is_weak_foot(PreferredFoot::Both, Side::Right));
        // 왼발 선수, 공이 오른쪽 → 약발
        assert!(is_weak_foot(PreferredFoot::Left, Side::Right));
    }

    #[test]
    fn test_action_kind_sigma_differences() {
        // Cross는 Pass보다 어려워야 함
        assert!(base_sigma::angle(ActionKind::Cross) > base_sigma::angle(ActionKind::Pass));
        // Shot은 높이 오차가 클 수 있음
        assert!(base_sigma::height(ActionKind::Shot) > base_sigma::height(ActionKind::Pass));
    }
}
