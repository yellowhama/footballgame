//! Ball Flight Resolver - Physics-Based Shot Resolution (Level 1)
//!
//! ## 개요
//! Outcome 선결정(pre-computed) → 물리 기반 후결정(physics-resolved)
//!
//! ## 핵심 원칙
//! - ShotAttempt: 슛 시작 (킥 시점)
//! - ShotResolved: 결과 판정 (교차 시점)
//! - 통계는 ShotResolved에서만 업데이트
//!
//! ## Level 1 범위
//! - 골라인 교차 → Goal/Wide
//! - 필드 경계 교차 → Out
//! - 골대 근처 → Woodwork
//! - Save/Blocked는 아직 확률 기반 (Level 2로 미룸)

use serde::{Deserialize, Serialize};

use super::physics_constants::{field, goal};

// ============================================================================
// Event Types
// ============================================================================

/// 슛 시도 이벤트 (킥 시점)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShotAttempt {
    /// 이벤트 발생 시각 (ms)
    pub t_ms: u64,
    /// 경기 분
    pub minute: u8,
    /// 홈팀 여부
    pub is_home: bool,
    /// 공격 방향 (true = 오른쪽, false = 왼쪽)
    pub attacks_right: Option<bool>,
    /// 슈터 인덱스 (0-21)
    pub shooter_idx: usize,
    /// 킥 위치 (미터)
    pub from_pos: (f32, f32),
    /// 목표 위치 (미터)
    pub aim_target: (f32, f32),
    /// 킥 힘 (0.0~1.0)
    pub force: f32,
    /// 초기 속도 벡터 (m/s)
    pub initial_velocity: (f32, f32),
    /// 예상 xG (참고용, 결과 판정에 사용 안 함)
    pub xg: f32,
}

/// 슛 결과 타입 (물리 기반 판정)
/// 기존 phase_action::ShotResult와 구분하기 위해 FlightShotResult로 명명
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlightShotResult {
    /// 골! (골라인 통과, 골문 범위 내)
    Goal,
    /// 빗나감 (골라인 통과, 골문 범위 밖)
    Wide,
    /// 골대 맞음 (포스트/크로스바)
    Woodwork,
    /// 필드 밖 (사이드라인/엔드라인)
    Out,
    /// GK 세이브 (Level 2에서 물리화)
    Saved,
    /// 수비수 블록 (Level 2에서 물리화)
    Blocked,
}

impl FlightShotResult {
    /// on_target 여부 (통계용)
    pub fn is_on_target(&self) -> bool {
        matches!(
            self,
            FlightShotResult::Goal | FlightShotResult::Saved | FlightShotResult::Woodwork
        )
    }
}

/// 슛 결과 판정 이벤트 (교차 시점)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShotResolved {
    /// 결과 판정 시각 (ms) - 교차 시점
    pub t_ms: u64,
    /// 경기 분
    pub minute: u8,
    /// 홈팀 여부
    pub is_home: bool,
    /// 슈터 인덱스
    pub shooter_idx: usize,
    /// 결과
    pub result: FlightShotResult,
    /// 판정 위치 (미터) - 교차 지점
    pub at_pos: (f32, f32),
    /// 원본 xG (Attempt에서 carry)
    pub xg: f32,
    /// 원본 Attempt 시각 (디버그용)
    pub attempt_t_ms: u64,
}

// ============================================================================
// Pitch Specification (Goal Area)
// ============================================================================

/// 필드/골 영역 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchSpec {
    /// 필드 길이 (미터) - 기본 105m
    pub length: f32,
    /// 필드 너비 (미터) - 기본 68m
    pub width: f32,
    /// 골문 너비 (미터) - 기본 7.32m
    pub goal_width: f32,
    /// 골문 높이 (미터) - 기본 2.44m (Level 2에서 사용)
    pub goal_height: f32,
}

impl Default for PitchSpec {
    fn default() -> Self {
        Self {
            length: field::LENGTH_M,
            width: field::WIDTH_M,
            goal_width: goal::WIDTH_M,
            goal_height: goal::HEIGHT_M,
        }
    }
}

impl PitchSpec {
    /// 홈팀 골라인 X 좌표 (홈팀이 방어하는 골)
    pub fn home_goal_line_x(&self) -> f32 {
        0.0
    }

    /// 원정팀 골라인 X 좌표 (원정팀이 방어하는 골 = 홈팀 공격 방향)
    pub fn away_goal_line_x(&self) -> f32 {
        self.length
    }

    /// 골문 Y 범위 (중앙 기준)
    pub fn goal_y_range(&self) -> (f32, f32) {
        let center = self.width / 2.0;
        let half_width = self.goal_width / 2.0;
        (center - half_width, center + half_width)
    }

    /// 공격 방향에 따른 타겟 골라인 X 좌표
    pub fn target_goal_line_x(&self, is_home_attacking: bool) -> f32 {
        if is_home_attacking {
            self.away_goal_line_x() // 홈팀이 공격 → 원정팀 골
        } else {
            self.home_goal_line_x() // 원정팀이 공격 → 홈팀 골
        }
    }
}

// ============================================================================
// Intersection Detection (Level 1 Core)
// ============================================================================

/// 교차 검사 결과
#[derive(Debug, Clone, Copy)]
pub struct IntersectionHit {
    /// 교차 시점 (0.0 = 시작, 1.0 = 끝)
    pub t_hit: f32,
    /// 교차 위치 (미터)
    pub position: (f32, f32),
}

/// 선분이 수직선과 교차하는지 검사
/// p0 → p1 선분이 x = line_x 와 교차하는 점을 찾는다
fn segment_intersect_vertical_line(
    p0: (f32, f32),
    p1: (f32, f32),
    line_x: f32,
) -> Option<IntersectionHit> {
    let dx = p1.0 - p0.0;

    // 수평 이동이 없으면 교차 불가
    if dx.abs() < 1e-6 {
        return None;
    }

    let t = (line_x - p0.0) / dx;

    // t가 [0, 1] 범위 밖이면 교차 안 함
    if !(0.0..=1.0).contains(&t) {
        return None;
    }

    let y = p0.1 + t * (p1.1 - p0.1);
    Some(IntersectionHit { t_hit: t, position: (line_x, y) })
}

/// 선분이 골라인과 교차하는지 검사
/// 반환: Some((hit, is_in_goal_frame))
pub fn segment_intersect_goal_line(
    p0: (f32, f32),
    p1: (f32, f32),
    goal_line_x: f32,
    goal_y_range: (f32, f32),
) -> Option<(IntersectionHit, bool)> {
    let hit = segment_intersect_vertical_line(p0, p1, goal_line_x)?;

    // 골문 범위 내인지 확인
    let is_in_goal = hit.position.1 >= goal_y_range.0 && hit.position.1 <= goal_y_range.1;

    Some((hit, is_in_goal))
}

/// 선분이 필드 경계와 교차하는지 검사
/// 사이드라인(y=0, y=width) 또는 엔드라인(x=0, x=length)과 교차
pub fn segment_intersect_boundary(
    p0: (f32, f32),
    p1: (f32, f32),
    pitch: &PitchSpec,
) -> Option<IntersectionHit> {
    let mut earliest: Option<IntersectionHit> = None;

    // 왼쪽 엔드라인 (x = 0)
    if let Some(hit) = segment_intersect_vertical_line(p0, p1, 0.0) {
        if earliest.is_none() || hit.t_hit < earliest.as_ref().unwrap().t_hit {
            earliest = Some(hit);
        }
    }

    // 오른쪽 엔드라인 (x = length)
    if let Some(hit) = segment_intersect_vertical_line(p0, p1, pitch.length) {
        if earliest.is_none() || hit.t_hit < earliest.as_ref().unwrap().t_hit {
            earliest = Some(hit);
        }
    }

    // 아래 사이드라인 (y = 0)
    if let Some(hit) = segment_intersect_horizontal_line(p0, p1, 0.0) {
        if earliest.is_none() || hit.t_hit < earliest.as_ref().unwrap().t_hit {
            earliest = Some(hit);
        }
    }

    // 위 사이드라인 (y = width)
    if let Some(hit) = segment_intersect_horizontal_line(p0, p1, pitch.width) {
        if earliest.is_none() || hit.t_hit < earliest.as_ref().unwrap().t_hit {
            earliest = Some(hit);
        }
    }

    earliest
}

/// 선분이 수평선과 교차하는지 검사
fn segment_intersect_horizontal_line(
    p0: (f32, f32),
    p1: (f32, f32),
    line_y: f32,
) -> Option<IntersectionHit> {
    let dy = p1.1 - p0.1;

    // 수직 이동이 없으면 교차 불가
    if dy.abs() < 1e-6 {
        return None;
    }

    let t = (line_y - p0.1) / dy;

    // t가 [0, 1] 범위 밖이면 교차 안 함
    if !(0.0..=1.0).contains(&t) {
        return None;
    }

    let x = p0.0 + t * (p1.0 - p0.0);
    Some(IntersectionHit { t_hit: t, position: (x, line_y) })
}

/// 선분이 골대(포스트) 근처와 교차하는지 검사
/// Level 1 단순화: 골라인 교차 but 골문 범위 밖 + 허용 오차 내 → Woodwork
pub fn segment_intersect_woodwork(
    p0: (f32, f32),
    p1: (f32, f32),
    goal_line_x: f32,
    goal_y_range: (f32, f32),
    post_tolerance: f32,
) -> Option<IntersectionHit> {
    let hit = segment_intersect_vertical_line(p0, p1, goal_line_x)?;
    let y = hit.position.1;

    // 골문 범위 밖이지만 포스트 허용 오차 내
    let near_post = (y < goal_y_range.0 && y >= goal_y_range.0 - post_tolerance)
        || (y > goal_y_range.1 && y <= goal_y_range.1 + post_tolerance);

    if near_post {
        Some(hit)
    } else {
        None
    }
}

// ============================================================================
// Main Resolver
// ============================================================================

/// 슛 궤적 시뮬레이션 설정
#[derive(Debug, Clone)]
pub struct FlightConfig {
    /// 시뮬레이션 최대 시간 (ms)
    pub max_flight_ms: u64,
    /// 서브스텝 시간 (ms) - 권장 50ms
    pub substep_ms: u64,
    /// 포스트 허용 오차 (미터)
    pub post_tolerance: f32,
    /// 공기 저항 계수 (Level 1에서는 0 = 직선)
    pub drag_coefficient: f32,
}

impl Default for FlightConfig {
    fn default() -> Self {
        Self {
            max_flight_ms: 2000,   // 최대 2초
            substep_ms: 50,        // 50ms 스텝
            post_tolerance: 0.3,   // 30cm
            drag_coefficient: 0.0, // Level 1: 직선 궤적
        }
    }
}

/// 슛 궤적을 계산하고 결과를 판정
///
/// Level 1: 직선 궤적으로 단순화 (drag = 0)
/// - 초기 속도로 목표 지점까지 직선 이동
/// - 교차 검사: 골라인 → 경계선 → 골대
/// - 최초 교차점에서 결과 결정
#[cfg(feature = "physics_resolve_shots")]
pub fn resolve_shot(
    attempt: &ShotAttempt,
    pitch: &PitchSpec,
    config: &FlightConfig,
) -> ShotResolved {
    let p0 = attempt.from_pos;

    // Level 1: 직선 궤적 - 초기 속도 방향으로 최대 비행 시간만큼 이동
    let flight_time_s = config.max_flight_ms as f32 / 1000.0;
    let p1 = (
        p0.0 + attempt.initial_velocity.0 * flight_time_s,
        p0.1 + attempt.initial_velocity.1 * flight_time_s,
    );

    // FIX_2601/0123: attacks_right must be set explicitly (halftime-aware)
    // Removed fallback to is_home which caused second-half direction bugs
    let attacks_right = attempt.attacks_right.expect(
        "FIX_2601: attacks_right must be set explicitly. Use DirectionContext.attacks_right"
    );
    let target_goal_x = pitch.target_goal_line_x(attacks_right);
    let goal_y_range = pitch.goal_y_range();

    // 1. 골라인 교차 검사 (최우선)
    if let Some((hit, is_in_goal)) =
        segment_intersect_goal_line(p0, p1, target_goal_x, goal_y_range)
    {
        // 골대 근처인지 확인
        if let Some(woodwork_hit) =
            segment_intersect_woodwork(p0, p1, target_goal_x, goal_y_range, config.post_tolerance)
        {
            // Woodwork가 먼저 발생했으면 Woodwork
            if woodwork_hit.t_hit <= hit.t_hit {
                let resolved_t_ms =
                    attempt.t_ms + (woodwork_hit.t_hit * config.max_flight_ms as f32) as u64;
                return ShotResolved {
                    t_ms: resolved_t_ms,
                    minute: attempt.minute,
                    is_home: attempt.is_home,
                    shooter_idx: attempt.shooter_idx,
                    result: FlightShotResult::Woodwork,
                    at_pos: woodwork_hit.position,
                    xg: attempt.xg,
                    attempt_t_ms: attempt.t_ms,
                };
            }
        }

        let resolved_t_ms = attempt.t_ms + (hit.t_hit * config.max_flight_ms as f32) as u64;

        if is_in_goal {
            // 골!
            return ShotResolved {
                t_ms: resolved_t_ms,
                minute: attempt.minute,
                is_home: attempt.is_home,
                shooter_idx: attempt.shooter_idx,
                result: FlightShotResult::Goal,
                at_pos: hit.position,
                xg: attempt.xg,
                attempt_t_ms: attempt.t_ms,
            };
        } else {
            // 빗나감 (Wide)
            return ShotResolved {
                t_ms: resolved_t_ms,
                minute: attempt.minute,
                is_home: attempt.is_home,
                shooter_idx: attempt.shooter_idx,
                result: FlightShotResult::Wide,
                at_pos: hit.position,
                xg: attempt.xg,
                attempt_t_ms: attempt.t_ms,
            };
        }
    }

    // 2. 필드 경계 교차 검사
    if let Some(hit) = segment_intersect_boundary(p0, p1, pitch) {
        let resolved_t_ms = attempt.t_ms + (hit.t_hit * config.max_flight_ms as f32) as u64;
        return ShotResolved {
            t_ms: resolved_t_ms,
            minute: attempt.minute,
            is_home: attempt.is_home,
            shooter_idx: attempt.shooter_idx,
            result: FlightShotResult::Out,
            at_pos: hit.position,
            xg: attempt.xg,
            attempt_t_ms: attempt.t_ms,
        };
    }

    // 3. 어떤 교차도 없으면 (이론적으로 발생하면 안 됨) Out 처리
    ShotResolved {
        t_ms: attempt.t_ms + config.max_flight_ms,
        minute: attempt.minute,
        is_home: attempt.is_home,
        shooter_idx: attempt.shooter_idx,
        result: FlightShotResult::Out,
        at_pos: p1,
        xg: attempt.xg,
        attempt_t_ms: attempt.t_ms,
    }
}

/// Feature flag OFF일 때 stub (기존 방식 유지용)
#[cfg(not(feature = "physics_resolve_shots"))]
pub fn resolve_shot(
    _attempt: &ShotAttempt,
    _pitch: &PitchSpec,
    _config: &FlightConfig,
) -> ShotResolved {
    panic!("physics_resolve_shots feature is not enabled. Use legacy path.");
}

// ============================================================================
// Helper: Create ShotAttempt from execution context
// ============================================================================

impl ShotAttempt {
    /// execute_shot 컨텍스트에서 ShotAttempt 생성
    pub fn from_execution(
        t_ms: u64,
        minute: u8,
        is_home: bool,
        attacks_right: bool,
        shooter_idx: usize,
        from_pos: (f32, f32),
        aim_target: (f32, f32),
        force: f32,
        xg: f32,
    ) -> Self {
        // 초기 속도 계산: 방향 × 힘 × 기본 속도
        let dx = aim_target.0 - from_pos.0;
        let dy = aim_target.1 - from_pos.1;
        let dist = (dx * dx + dy * dy).sqrt().max(0.01);
        let dir = (dx / dist, dy / dist);

        // 기본 슛 속도: 25 m/s (약 90 km/h), force로 조정
        let base_speed = 25.0;
        let speed = base_speed * (0.5 + 0.5 * force); // 12.5 ~ 25 m/s

        Self {
            t_ms,
            minute,
            is_home,
            attacks_right: Some(attacks_right),
            shooter_idx,
            from_pos,
            aim_target,
            force,
            initial_velocity: (dir.0 * speed, dir.1 * speed),
            xg,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_pitch() -> PitchSpec {
        PitchSpec::default()
    }

    fn default_config() -> FlightConfig {
        FlightConfig::default()
    }

    #[test]
    fn test_goal_line_intersection_goal() {
        let pitch = default_pitch();
        let goal_y = pitch.goal_y_range();

        // 중앙에서 골문 정중앙으로 향하는 궤적
        let p0 = (field::CENTER_X, field::CENTER_Y); // 필드 중앙
        let p1 = (field::LENGTH_M, field::CENTER_Y); // 골라인 중앙

        let result = segment_intersect_goal_line(p0, p1, field::LENGTH_M, goal_y);
        assert!(result.is_some());
        let (hit, is_in_goal) = result.unwrap();
        assert!(is_in_goal, "Should be in goal frame");
        assert!((hit.t_hit - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_goal_line_intersection_wide() {
        let pitch = default_pitch();
        let goal_y = pitch.goal_y_range();

        // 중앙에서 골문 밖으로 향하는 궤적
        let p0 = (field::CENTER_X, field::CENTER_Y);
        let p1 = (field::LENGTH_M, 10.0); // 골문 밖 (y=10은 goal_y_range 밖)

        let result = segment_intersect_goal_line(p0, p1, field::LENGTH_M, goal_y);
        assert!(result.is_some());
        let (_, is_in_goal) = result.unwrap();
        assert!(!is_in_goal, "Should be outside goal frame");
    }

    #[test]
    fn test_boundary_intersection() {
        let pitch = default_pitch();

        // 사이드라인 밖으로 나가는 궤적
        let p0 = (field::CENTER_X, field::CENTER_Y);
        let p1 = (field::CENTER_X, 100.0); // y > 68

        let result = segment_intersect_boundary(p0, p1, &pitch);
        assert!(result.is_some());
        let hit = result.unwrap();
        assert!(
            (hit.position.1 - field::WIDTH_M).abs() < 0.01,
            "Should hit top sideline"
        );
    }

    #[test]
    fn test_earliest_hit_wins() {
        let pitch = default_pitch();

        // 사이드라인과 엔드라인 모두 교차하는 궤적
        let p0 = (50.0, field::CENTER_Y);
        let p1 = (120.0, 80.0); // 두 경계 모두 교차

        let result = segment_intersect_boundary(p0, p1, &pitch);
        assert!(result.is_some());

        // 어떤 것이든 가장 먼저 교차한 것이 반환되어야 함
        let hit = result.unwrap();
        assert!(hit.t_hit > 0.0 && hit.t_hit < 1.0);
    }

    #[cfg(feature = "physics_resolve_shots")]
    #[test]
    fn test_resolve_shot_goal() {
        let pitch = default_pitch();
        let config = default_config();

        let attempt = ShotAttempt {
            t_ms: 1000,
            minute: 10,
            is_home: true,
            attacks_right: Some(true),
            shooter_idx: 9,
            from_pos: (90.0, field::CENTER_Y),    // 페널티 에어리어 근처
            aim_target: (field::LENGTH_M, field::CENTER_Y), // 골문 중앙
            force: 1.0,
            initial_velocity: (25.0, 0.0), // 직선으로 골문 향함
            xg: 0.5,
        };

        let resolved = resolve_shot(&attempt, &pitch, &config);
        assert_eq!(resolved.result, FlightShotResult::Goal);
        assert_eq!(resolved.shooter_idx, 9);
        assert!(resolved.t_ms > attempt.t_ms);
    }

    #[cfg(feature = "physics_resolve_shots")]
    #[test]
    fn test_resolve_shot_wide() {
        let pitch = default_pitch();
        let config = default_config();

        let attempt = ShotAttempt {
            t_ms: 1000,
            minute: 10,
            is_home: true,
            attacks_right: Some(true),
            shooter_idx: 9,
            from_pos: (90.0, field::CENTER_Y),
            aim_target: (field::LENGTH_M, 10.0), // 골문 밖
            force: 1.0,
            initial_velocity: (23.0, -12.0), // 골문 밖으로
            xg: 0.1,
        };

        let resolved = resolve_shot(&attempt, &pitch, &config);
        assert_eq!(resolved.result, FlightShotResult::Wide);
    }

    #[cfg(feature = "physics_resolve_shots")]
    #[test]
    fn test_resolve_shot_out() {
        let pitch = default_pitch();
        let config = default_config();

        let attempt = ShotAttempt {
            t_ms: 1000,
            minute: 10,
            is_home: true,
            attacks_right: Some(true),
            shooter_idx: 9,
            from_pos: (50.0, field::CENTER_Y),
            aim_target: (50.0, 100.0), // 사이드라인 밖
            force: 1.0,
            initial_velocity: (0.0, 25.0), // 사이드라인 방향
            xg: 0.0,
        };

        let resolved = resolve_shot(&attempt, &pitch, &config);
        assert_eq!(resolved.result, FlightShotResult::Out);
    }

    /// Phase 3-7: 결정론 테스트 - 동일 입력 → 동일 출력
    #[cfg(feature = "physics_resolve_shots")]
    #[test]
    fn test_resolve_shot_determinism() {
        let pitch = default_pitch();
        let config = default_config();

        // 동일한 ShotAttempt 10회 실행
        let attempt = ShotAttempt {
            t_ms: 5000,
            minute: 45,
            is_home: true,
            attacks_right: Some(true),
            shooter_idx: 10,
            from_pos: (85.0, 30.0),
            aim_target: (field::LENGTH_M, 35.0),
            force: 0.9,
            initial_velocity: (22.0, 5.5),
            xg: 0.15,
        };

        let first_result = resolve_shot(&attempt, &pitch, &config);

        // 10회 반복해서 동일한 결과인지 확인
        for _ in 0..10 {
            let result = resolve_shot(&attempt, &pitch, &config);
            assert_eq!(result.result, first_result.result, "Results should be deterministic");
            assert_eq!(result.at_pos, first_result.at_pos, "Positions should be deterministic");
            assert_eq!(result.t_ms, first_result.t_ms, "Timing should be deterministic");
        }
    }

    /// Phase 3-7: ShotAttempt::from_execution 헬퍼 테스트
    #[cfg(feature = "physics_resolve_shots")]
    #[test]
    fn test_shot_attempt_from_execution() {
        let attempt = ShotAttempt::from_execution(
            10000,         // t_ms
            45,            // minute
            true,          // is_home
            true,          // attacks_right
            9,             // shooter_idx
            (80.0, field::CENTER_Y),  // from_pos
            (field::LENGTH_M, field::CENTER_Y), // aim_target (골문 중앙)
            1.0,           // force
            0.2,           // xg
        );

        // 기본 필드 확인
        assert_eq!(attempt.t_ms, 10000);
        assert_eq!(attempt.minute, 45);
        assert!(attempt.is_home);
        assert_eq!(attempt.shooter_idx, 9);
        assert_eq!(attempt.from_pos, (80.0, field::CENTER_Y));
        assert_eq!(attempt.aim_target, (field::LENGTH_M, field::CENTER_Y));
        assert_eq!(attempt.force, 1.0);
        assert_eq!(attempt.xg, 0.2);

        // 초기 속도가 타겟 방향을 향하는지 확인
        assert!(attempt.initial_velocity.0 > 0.0, "Should move toward goal (x+)");
        // y 속도는 거의 0 (직선)
        assert!(attempt.initial_velocity.1.abs() < 0.1, "Should move straight");
    }
}
