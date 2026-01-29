//! Ball physics and trajectory calculations
//!
//! This module contains:
//! - Ball struct and its physics state (FIX_2512: Coord10 integration)
//! - Curve levels for shot bending
//! - Height profiles for ball trajectories
//! - Bezier curve interpolation for realistic ball movement

use crate::engine::types::coord10::{Coord10, Vel10};
use serde::{Deserialize, Serialize};

/// Linear interpolation between two positions
pub fn lerp_position(from: (f32, f32), to: (f32, f32), t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
}

/// Curved interpolation between two positions using quadratic Bezier curve
/// curve_factor: -0.35 ~ +0.35 (0 = straight line, + = left curve, - = right curve)
pub fn lerp_curve(from: (f32, f32), to: (f32, f32), curve_factor: f32, t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);

    if curve_factor.abs() < 0.001 {
        // No curve - use straight line
        return lerp_position(from, to, t);
    }

    // Calculate perpendicular vector for curve control point
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;

    // Perpendicular vector (rotate 90 degrees)
    let perp_x = -dy;
    let perp_y = dx;

    // Midpoint
    let mid_x = (from.0 + to.0) / 2.0;
    let mid_y = (from.1 + to.1) / 2.0;

    // Control point offset from midpoint
    let control_x = mid_x + perp_x * curve_factor;
    let control_y = mid_y + perp_y * curve_factor;

    // Quadratic Bezier: P(t) = (1-t)²*P0 + 2(1-t)t*P1 + t²*P2
    let one_minus_t = 1.0 - t;
    let weight_from = one_minus_t * one_minus_t;
    let weight_control = 2.0 * one_minus_t * t;
    let weight_to = t * t;

    (
        from.0 * weight_from + control_x * weight_control + to.0 * weight_to,
        from.1 * weight_from + control_y * weight_control + to.1 * weight_to,
    )
}

/// 감아차기 스킬 레벨
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveLevel {
    None, // 0.0 ~ ±0.05: 거의 직선
    Lv1,  // ±0.05 ~ ±0.10: 살짝 감기
    Lv2,  // ±0.10 ~ ±0.20: 중간 커브
    Lv3,  // ±0.20 ~ ±0.35: 극한 감아차기
}

impl CurveLevel {
    /// Get base curve factor for this level
    pub fn base_curve_factor(&self) -> f32 {
        match self {
            CurveLevel::None => 0.0,
            CurveLevel::Lv1 => 0.05,
            CurveLevel::Lv2 => 0.15,
            CurveLevel::Lv3 => 0.30,
        }
    }

    /// Get max curve factor for this level
    pub fn max_curve_factor(&self) -> f32 {
        match self {
            CurveLevel::None => 0.05,
            CurveLevel::Lv1 => 0.10,
            CurveLevel::Lv2 => 0.20,
            CurveLevel::Lv3 => 0.35,
        }
    }
}

/// 공의 높이 프로파일 (MVP: 3개)
/// - Flat: 땅볼 (z = 0)
/// - Arc: 일반 발슛/패스 (max ~3.5m) - 중거리슛 포함
/// - Lob: 로빙/칩/클리어 (max ~10m)
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HeightProfile {
    Flat, // 땅볼 (z = 0.0)
    Arc,  // 일반 슛/패스 (max 3.5m) - 중거리슛, 일반 킥
    Lob,  // 로빙/칩/클리어 (max 10m)
}

impl HeightProfile {
    /// 프로파일별 최대 높이 캡 (미터)
    /// Arc 3.5m = vz_cap 8.28 m/s (sqrt(2 * 9.81 * 3.5))
    /// Lob 10m = vz_cap 14.0 m/s
    pub fn height_cap_m(&self) -> f32 {
        match self {
            HeightProfile::Flat => 0.0,
            HeightProfile::Arc => 3.5,
            HeightProfile::Lob => 10.0,
        }
    }

    /// 프로파일별 vz_cap 계산 (m/s)
    /// vz_cap = sqrt(2 * g * h_cap)
    pub fn vz_cap_mps(&self) -> f32 {
        let h = self.height_cap_m();
        if h <= 0.0 {
            0.0
        } else {
            (2.0 * 9.81 * h).sqrt()
        }
    }
}

/// Compute lift ratio from intent/skill/pressure (0.0~1.0).
pub fn compute_lift_ratio(intent: f32, skill: f32, pressure: f32) -> f32 {
    let intent = intent.clamp(0.0, 1.0);
    let skill = skill.clamp(0.0, 1.0);
    let pressure = pressure.clamp(0.0, 1.0);
    (intent * skill * (1.0 - pressure * 0.4)).clamp(0.0, 1.0)
}

/// Convert height profile + lift ratio to max height (meters).
pub fn max_height_from_profile(profile: HeightProfile, lift_ratio: f32) -> f32 {
    let ratio = lift_ratio.clamp(0.0, 1.0);
    let cap = profile.height_cap_m();
    if cap <= 0.0 {
        0.0
    } else {
        cap * ratio * ratio.sqrt()
    }
}

/// 접촉 방식 (MVP: 2개)
/// Header를 HeightProfile에서 분리 → 여기로
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ContactType {
    #[default]
    Foot, // 발 (기본)
    Head, // 헤딩 (jumping/heading 스탯 영향)
}

/// Get ball position at time t (0.0 ~ 1.0) including height
/// Returns (x, y, z) where z is the ball height based on HeightProfile
///
/// MVP: Flat/Arc/Lob 3개 프로파일
pub fn get_ball_position_3d(
    from: (f32, f32),
    to: (f32, f32),
    curve_factor: f32,
    height_profile: HeightProfile,
    t: f32,
) -> (f32, f32, f32) {
    let max_height = height_profile.height_cap_m();
    get_ball_position_3d_with_height(from, to, curve_factor, max_height, t)
}

/// Get ball position at time t (0.0 ~ 1.0) including height.
/// Returns (x, y, z) where z is the ball height based on max_height.
/// Note: This assumes start_height=0 and end_height=0.
/// For non-zero endpoints (crossbar rebounds), use get_ball_position_3d_with_endpoints.
pub fn get_ball_position_3d_with_height(
    from: (f32, f32),
    to: (f32, f32),
    curve_factor: f32,
    max_height: f32,
    t: f32,
) -> (f32, f32, f32) {
    get_ball_position_3d_with_endpoints(from, to, curve_factor, max_height, 0.0, 0.0, t)
}

/// vNext: Get ball position at time t with non-zero start/end heights.
/// Supports crossbar rebounds where ball starts at height > 0.
///
/// Formula:
/// - z_base(t) = lerp(start_height, end_height, t)  // linear interpolation
/// - z_bump(t) = 4 * bump_height * t * (1 - t)      // parabola overlay
/// - z(t) = z_base(t) + z_bump(t)
///
/// When start_height = end_height = 0, this is equivalent to the original formula.
pub fn get_ball_position_3d_with_endpoints(
    from: (f32, f32),
    to: (f32, f32),
    curve_factor: f32,
    bump_height: f32,
    start_height: f32,
    end_height: f32,
    t: f32,
) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);

    // XY: Use existing Bezier curve logic
    let (x, y) = lerp_curve(from, to, curve_factor, t);

    // Z: Endpoint-aware height calculation
    // Linear interpolation from start to end
    let z_base = start_height + (end_height - start_height) * t;

    // Parabola bump overlay (peak at t=0.5)
    let z_bump = if bump_height <= 0.0 { 0.0 } else { 4.0 * bump_height * t * (1.0 - t) };

    // Combined height (clamped to non-negative)
    let z = (z_base + z_bump).max(0.0);

    (x, y, z)
}

/// 공 상태 (물리 기반)
/// FIX_2512 Phase 4 - TASK_09: Coord10 통합
/// Ball Physics V2: velocity_z 추가 (중력 기반 포물선)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ball {
    pub position: Coord10,            // FIX_2512: 0.1m 단위 정수 좌표
    pub velocity: Vel10,              // FIX_2512: 0.1m/s 단위 속도 (XY)
    pub velocity_z: i16,              // Ball Physics V2: 0.1m/s 단위 수직 속도
    pub height: i16,                  // FIX_2512: 0.1m 단위 (0-100 = 0-10m)
    pub current_owner: Option<usize>, // 소유 선수 인덱스
    pub previous_owner: Option<usize>,
    // Flight fields
    pub from_position: Option<Coord10>, // FIX_2512: 비행 시작 위치
    pub to_position: Option<Coord10>,   // FIX_2512: 비행 목표 위치
    pub flight_progress: f32,           // 비행 진행률 (0.0-1.0)
    pub flight_speed: f32,              // 비행 속도 (m/s, float 유지)
    pub is_in_flight: bool,             // 비행 중 여부
    pub pending_owner: Option<usize>,   // 비행 종료 후 소유자
    pub curve_factor: f32,              // 곡선 궤적 (-0.35 ~ +0.35, 0=직선)
    pub height_profile: HeightProfile,  // 높이 프로파일 (초기 vz 결정용)
    // vNext: Crossbar height continuity (0.1m 단위)
    // sync_from_ball에서 드리프트 방지를 위해 Ball에도 저장
    pub flight_start_height_01m: i16,   // 비행 시작 높이
    pub flight_end_height_01m: i16,     // 비행 종료 높이
    // Ball Physics V2 Phase 3: Bounce fields
    pub bounce_count: u8, // 바운스 횟수 (MAX_BOUNCES 초과 시 롤링)
    pub is_rolling: bool, // 롤링 모드 (바운스 종료 후)
    // FIX_2601/0112: Magnus Effect - spin vector (rad/s)
    // spin.0 = x-axis rotation (backspin/topspin)
    // spin.1 = y-axis rotation (sidespin)
    // spin.2 = z-axis rotation (rifle spin - 공 자체 회전)
    pub spin: (f32, f32, f32),
}

impl Default for Ball {
    fn default() -> Self {
        Self {
            position: Coord10::CENTER, // FIX_2512: 필드 중앙
            velocity: Vel10::default(),                 // FIX_2512: 정지 상태
            velocity_z: 0,                              // Ball Physics V2: 수직 속도 0
            height: 0,                                  // FIX_2512: 지면
            current_owner: None,
            previous_owner: None,
            from_position: None,
            to_position: None,
            flight_progress: 0.0,
            flight_speed: 0.0,
            is_in_flight: false,
            pending_owner: None,
            curve_factor: 0.0,
            height_profile: HeightProfile::Flat,
            // vNext: Crossbar height continuity
            flight_start_height_01m: 0,
            flight_end_height_01m: 0,
            // Phase 3: Bounce
            bounce_count: 0,
            is_rolling: false,
            // FIX_2601/0112: Magnus Effect
            spin: (0.0, 0.0, 0.0),
        }
    }
}

impl Ball {
    /// FIX_2512: Start ball flight from current position to target
    /// FIX_2601/0116: Save previous_owner before clearing current_owner
    pub fn start_flight(&mut self, to: Coord10, speed: f32, landing_owner: Option<usize>) {
        self.from_position = Some(self.position);
        self.to_position = Some(to);
        self.flight_progress = 0.0;
        self.flight_speed = speed;
        self.is_in_flight = true;
        // FIX_2601/0116: Preserve shooter for goal attribution
        // Without this, previous_owner retains wrong player and GK/CB can be credited for goals
        if self.current_owner.is_some() {
            self.previous_owner = self.current_owner;
        }
        self.current_owner = None; // Ball released
        self.pending_owner = landing_owner;
    }

    /// Complete ball flight and set new owner
    pub fn complete_flight(&mut self, new_owner: Option<usize>) {
        if let Some(to) = self.to_position {
            self.position = to;
        }
        self.from_position = None;
        self.to_position = None;
        self.flight_progress = 0.0;
        self.flight_speed = 0.0;
        self.is_in_flight = false;
        self.pending_owner = None;
        if self.current_owner.is_some() {
            self.previous_owner = self.current_owner;
        }
        self.current_owner = new_owner;
    }

    /// D5-2: Set curve factor with range validation (-0.35 ~ 0.35)
    pub fn set_curve_factor(&mut self, cf: f32) {
        self.curve_factor = cf.clamp(-0.35, 0.35);
    }

    /// Ball Physics V2: Launch ball with gravity-based trajectory
    ///
    /// Sets initial vertical velocity based on HeightProfile.
    /// vz = vz_cap * 10 (0.1m/s 단위)
    ///
    /// MVP: Flat/Arc/Lob 3개
    pub fn launch_with_profile(&mut self, profile: HeightProfile) {
        self.height_profile = profile;
        // vz_cap_mps() * 10 = 0.1m/s 단위
        self.velocity_z = (profile.vz_cap_mps() * 10.0).round() as i16;
    }

    /// Ball Physics V2: Launch with custom lift ratio
    ///
    /// lift_ratio: 0.0 ~ 1.0 (캡 대비 몇 %를 쓸지)
    /// - 0.0 = 땅볼
    /// - 0.3 = 낮은 탄도 (구 Driven 수준)
    /// - 1.0 = 캡 최대 (Arc면 3.5m, Lob이면 10m)
    pub fn launch_with_ratio(&mut self, profile: HeightProfile, lift_ratio: f32) {
        self.height_profile = profile;
        let vz_cap = profile.vz_cap_mps();
        let vz = vz_cap * lift_ratio.clamp(0.0, 1.0);
        self.velocity_z = (vz * 10.0).round() as i16;
    }

    /// Ball Physics V2: Combined launch - start flight with profile
    ///
    /// Combines start_flight() with launch_with_profile() for convenience.
    pub fn launch_flight(
        &mut self,
        to: Coord10,
        speed: f32,
        profile: HeightProfile,
        landing_owner: Option<usize>,
    ) {
        self.start_flight(to, speed, landing_owner);
        self.launch_with_profile(profile);
    }

    /// FIX_2512: Get position in meters (for display/debugging)
    pub fn position_meters(&self) -> (f32, f32) {
        self.position.to_meters()
    }

    /// FIX_2512: Get velocity in m/s (for display/debugging)
    pub fn velocity_mps(&self) -> (f32, f32) {
        self.velocity.to_mps()
    }

    /// FIX_2512: Get height in meters
    pub fn height_meters(&self) -> f32 {
        self.height as f32 * 0.1
    }

    // =========================================
    // Ball Physics V2: Vertical velocity methods
    // =========================================

    /// Get vertical velocity in m/s
    pub fn velocity_z_mps(&self) -> f32 {
        self.velocity_z as f32 * 0.1
    }

    /// Set vertical velocity from m/s
    pub fn set_velocity_z_mps(&mut self, vz_mps: f32) {
        self.velocity_z = (vz_mps * 10.0).round() as i16;
    }

    /// Check if ball is airborne (height > 0 or moving upward)
    pub fn is_airborne(&self) -> bool {
        self.height > 0 || self.velocity_z > 0
    }

    /// Check if ball is on ground and stationary vertically
    pub fn is_grounded(&self) -> bool {
        self.height == 0 && self.velocity_z <= 0
    }

    // =========================================
    // FIX_2601/0112: Magnus Effect methods
    // =========================================

    /// Set spin from CurveLevel and direction
    ///
    /// Google Football 스타일: 곡선 레벨에 따른 스핀 설정
    /// - direction: +1.0 = 왼쪽으로 휘어짐, -1.0 = 오른쪽으로 휘어짐
    pub fn set_spin_from_curve(&mut self, curve_level: CurveLevel, direction: f32) {
        use super::physics_constants::google_football;

        let base_spin = match curve_level {
            CurveLevel::None => 0.0,
            CurveLevel::Lv1 => 2.0, // ~2 rad/s (약한 커브)
            CurveLevel::Lv2 => 5.0, // ~5 rad/s (중간 커브)
            CurveLevel::Lv3 => 8.0, // ~8 rad/s (강한 커브)
        };

        // y-axis spin for sidespin (horizontal curve)
        let sidespin = base_spin * direction.signum();
        // x-axis spin for topspin/backspin (slight topspin for finesse)
        let topspin = base_spin * 0.3 * google_football::MAGNUS_COEFFICIENT;

        self.spin = (topspin, sidespin, 0.0);
    }

    /// Set spin directly (for advanced control)
    pub fn set_spin(&mut self, spin: (f32, f32, f32)) {
        self.spin = spin;
    }

    /// Get spin magnitude (total rotation speed in rad/s)
    pub fn spin_magnitude(&self) -> f32 {
        let (sx, sy, sz) = self.spin;
        (sx * sx + sy * sy + sz * sz).sqrt()
    }

    /// Apply spin decay (air resistance reduces spin over time)
    ///
    /// Google Football decay: spin *= 0.97 per tick
    pub fn decay_spin(&mut self, factor: f32) {
        self.spin.0 *= factor;
        self.spin.1 *= factor;
        self.spin.2 *= factor;
    }

    /// Reset spin to zero (after ball stops or is caught)
    pub fn reset_spin(&mut self) {
        self.spin = (0.0, 0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_lerp_position() {
        let from = (0.0, 0.0);
        let to = (10.0, 10.0);

        let mid = lerp_position(from, to, 0.5);
        assert!((mid.0 - 5.0).abs() < 0.001);
        assert!((mid.1 - 5.0).abs() < 0.001);

        let start = lerp_position(from, to, 0.0);
        assert!((start.0 - 0.0).abs() < 0.001);

        let end = lerp_position(from, to, 1.0);
        assert!((end.0 - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_lerp_curve_straight() {
        let from = (0.0, 0.0);
        let to = (10.0, 0.0);

        // No curve should be same as lerp_position
        let mid = lerp_curve(from, to, 0.0, 0.5);
        assert!((mid.0 - 5.0).abs() < 0.001);
        assert!((mid.1 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_lerp_curve_with_bend() {
        let from = (0.0, 0.0);
        let to = (10.0, 0.0);

        // Positive curve should bend left (positive y)
        let mid = lerp_curve(from, to, 0.2, 0.5);
        assert!(mid.1 > 0.0, "Positive curve should bend upward");

        // Negative curve should bend right (negative y)
        let mid_neg = lerp_curve(from, to, -0.2, 0.5);
        assert!(mid_neg.1 < 0.0, "Negative curve should bend downward");
    }

    #[test]
    fn test_curve_level() {
        assert_eq!(CurveLevel::None.base_curve_factor(), 0.0);
        assert_eq!(CurveLevel::Lv3.max_curve_factor(), 0.35);
    }

    #[test]
    fn test_height_profile_flat() {
        let pos = get_ball_position_3d((0.0, 0.0), (10.0, 0.0), 0.0, HeightProfile::Flat, 0.5);
        assert_eq!(pos.2, 0.0);
    }

    #[test]
    fn test_height_profile_arc_sanity() {
        let start =
            get_ball_position_3d((0.0, 0.0), (10.0, 0.0), 0.0, HeightProfile::Arc, 0.0);
        let mid =
            get_ball_position_3d((0.0, 0.0), (10.0, 0.0), 0.0, HeightProfile::Arc, 0.5);
        let end =
            get_ball_position_3d((0.0, 0.0), (10.0, 0.0), 0.0, HeightProfile::Arc, 1.0);

        assert!((start.2 - 0.0).abs() < 1e-6);
        assert!((end.2 - 0.0).abs() < 1e-6);
        assert!((mid.2 - 3.5).abs() < 1e-3, "Arc should peak at 3.5 at t=0.5");

        // Symmetry: z(t) == z(1-t)
        let q1 =
            get_ball_position_3d((0.0, 0.0), (10.0, 0.0), 0.0, HeightProfile::Arc, 0.25);
        let q3 =
            get_ball_position_3d((0.0, 0.0), (10.0, 0.0), 0.0, HeightProfile::Arc, 0.75);
        assert!((q1.2 - q3.2).abs() < 1e-6);
    }

    #[test]
    fn test_height_profile_lob() {
        let pos = get_ball_position_3d((0.0, 0.0), (10.0, 0.0), 0.0, HeightProfile::Lob, 0.5);
        assert!((pos.2 - 10.0).abs() < 0.001, "Lob should peak at 10.0 at t=0.5");
    }

    #[test]
    fn test_max_height_from_profile_scales_with_lift_ratio() {
        let full = max_height_from_profile(HeightProfile::Arc, 1.0);
        assert!((full - 3.5).abs() < 1e-6);

        let ratio = 0.5f32;
        let expected = HeightProfile::Arc.height_cap_m() * ratio * ratio.sqrt();
        let scaled = max_height_from_profile(HeightProfile::Arc, ratio);
        assert!((scaled - expected).abs() < 1e-6);
    }

    #[test]
    fn test_ball_flight() {
        let mut ball = Ball::default();
        ball.position = Coord10::from_meters(21.0, 31.5); // FIX_2601: ~2.1m, 3.15m

        let to = Coord10::from_meters(84.0, 73.5); // FIX_2601
        ball.start_flight(to, 0.5, Some(5));

        assert!(ball.is_in_flight);
        assert_eq!(ball.from_position, Some(Coord10::from_meters(21.0, 31.5)));
        assert_eq!(ball.to_position, Some(to));
        assert_eq!(ball.pending_owner, Some(5));
        assert_eq!(ball.current_owner, None);

        ball.complete_flight(Some(5));

        assert!(!ball.is_in_flight);
        assert_eq!(ball.position, to);
        assert_eq!(ball.current_owner, Some(5));
    }

    #[test]
    fn test_ball_curve_factor_clamping() {
        let mut ball = Ball::default();

        ball.set_curve_factor(0.5);
        assert_eq!(ball.curve_factor, 0.35);

        ball.set_curve_factor(-0.5);
        assert_eq!(ball.curve_factor, -0.35);

        ball.set_curve_factor(0.2);
        assert_eq!(ball.curve_factor, 0.2);
    }

    // vNext: Crossbar height continuity tests
    #[test]
    fn test_height_endpoints_zero_preserves_original_behavior() {
        // When start_height=0 and end_height=0, should match original behavior
        let original = get_ball_position_3d_with_height(
            (0.0, 0.0),
            (10.0, 0.0),
            0.0,
            3.5, // max_height
            0.5,
        );
        let with_endpoints = get_ball_position_3d_with_endpoints(
            (0.0, 0.0),
            (10.0, 0.0),
            0.0,
            3.5, // bump_height
            0.0, // start_height
            0.0, // end_height
            0.5,
        );
        assert!((original.0 - with_endpoints.0).abs() < 1e-6);
        assert!((original.1 - with_endpoints.1).abs() < 1e-6);
        assert!((original.2 - with_endpoints.2).abs() < 1e-6);
    }

    #[test]
    fn test_height_endpoints_crossbar_rebound() {
        // Crossbar rebound scenario: ball starts at 2.44m (crossbar height), ends at 0
        let crossbar_height = 2.44;

        // At t=0, height should be exactly crossbar_height
        let start = get_ball_position_3d_with_endpoints(
            (100.0, field::CENTER_Y),
            (90.0, field::CENTER_Y),
            0.0,
            0.0, // no bump (descending trajectory)
            crossbar_height,
            0.0,
            0.0,
        );
        assert!(
            (start.2 - crossbar_height).abs() < 0.01,
            "At t=0, height should be crossbar height: got {}",
            start.2
        );

        // At t=1, height should be 0
        let end = get_ball_position_3d_with_endpoints(
            (100.0, field::CENTER_Y),
            (90.0, field::CENTER_Y),
            0.0,
            0.0,
            crossbar_height,
            0.0,
            1.0,
        );
        assert!(end.2 < 0.01, "At t=1, height should be ~0: got {}", end.2);

        // At t=0.5, height should be halfway
        let mid = get_ball_position_3d_with_endpoints(
            (100.0, field::CENTER_Y),
            (90.0, field::CENTER_Y),
            0.0,
            0.0,
            crossbar_height,
            0.0,
            0.5,
        );
        assert!(
            (mid.2 - crossbar_height / 2.0).abs() < 0.1,
            "At t=0.5, height should be ~1.22m: got {}",
            mid.2
        );
    }

    #[test]
    fn test_height_endpoints_with_bump() {
        // Test that bump_height adds correctly to the linear interpolation
        let start_h = 2.0;
        let end_h = 0.0;
        let bump_h = 1.0;

        // At t=0.5: z_base = 1.0, z_bump = 1.0 * 4 * 0.5 * 0.5 = 1.0, total = 2.0
        let mid = get_ball_position_3d_with_endpoints(
            (0.0, 0.0),
            (10.0, 0.0),
            0.0,
            bump_h,
            start_h,
            end_h,
            0.5,
        );
        assert!(
            (mid.2 - 2.0).abs() < 0.01,
            "At t=0.5 with bump, height should be 2.0: got {}",
            mid.2
        );

        // At t=0: z_base = 2.0, z_bump = 0, total = 2.0
        let start = get_ball_position_3d_with_endpoints(
            (0.0, 0.0),
            (10.0, 0.0),
            0.0,
            bump_h,
            start_h,
            end_h,
            0.0,
        );
        assert!(
            (start.2 - 2.0).abs() < 0.01,
            "At t=0, height should be start_h=2.0: got {}",
            start.2
        );
    }
}
