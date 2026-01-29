//! Coord10: 0.1m 단위 정수 좌표 시스템
//!
//! FIX_2512 Phase 1 - TASK_04
//!
//! ## 설계 목표
//! - 10cm 그리드 기반 고정소수점 좌표
//! - 부동소수점 오차 제거
//! - 결정론적 시뮬레이션 보장
//! - 50ms tick rate 지원

use serde::{Deserialize, Serialize};

// ============================================================================
// Coord10: 좌표 (0.1m 단위)
// ============================================================================

/// 0.1m 단위 정수 좌표 (고정소수점)
///
/// 필드 범위:
/// - x: 0..1050 (0m ~ 105m)
/// - y: 0..680  (0m ~ 68m)
///
/// 스케일: 1 unit = 0.1m = 100mm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Coord10 {
    pub x: i32,
    pub y: i32,
    /// Height in 0.1m units (0-500 = 0-50m). Default 0 for ground level.
    #[serde(default)]
    pub z: i32,
}

impl Coord10 {
    pub const SCALE: f32 = 10.0; // 1m = 10 units

    /// Origin (0, 0, 0)
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };

    /// Maximum height in Coord10 units (50m = 500)
    pub const MAX_HEIGHT_10: i32 = 500;

    // =========================================================================
    // Field dimensions (SSOT for Coord10 coordinates)
    // FIX_2601: 하드코딩 제거
    // =========================================================================

    /// In-bounds field length in Coord10 units (105.0m × 10 = 1050)
    pub const FIELD_LENGTH_10: i32 = 1050;
    /// In-bounds field width in Coord10 units (68.0m × 10 = 680)
    pub const FIELD_WIDTH_10: i32 = 680;

    /// Field center X (FIELD_LENGTH_10 / 2 = 525)
    pub const CENTER_X: i32 = Self::FIELD_LENGTH_10 / 2; // 525
    /// Field center Y (FIELD_WIDTH_10 / 2 = 340)
    pub const CENTER_Y: i32 = Self::FIELD_WIDTH_10 / 2; // 340

    /// Field center (x=CENTER_X, y=CENTER_Y, z=0) - derived from FIELD_LENGTH_10/FIELD_WIDTH_10
    pub const CENTER: Self = Self { x: Self::CENTER_X, y: Self::CENTER_Y, z: 0 };

    // =========================================================================
    // Bounds policy (in-bounds vs margin)
    // =========================================================================

    /// In-bounds (playable rectangle): players should be clamped here.
    pub const IN_BOUNDS_X_MIN: i32 = 0;
    pub const IN_BOUNDS_X_MAX: i32 = Self::FIELD_LENGTH_10;
    pub const IN_BOUNDS_Y_MIN: i32 = 0;
    pub const IN_BOUNDS_Y_MAX: i32 = Self::FIELD_WIDTH_10;

    /// Margin (0.1m units) used for ball physics/out-of-play detection buffer.
    pub const FIELD_MARGIN_10: i32 = 10; // 1.0m

    /// Margin-bounds: small buffer around the field.
    pub const FIELD_X_MIN: i32 = -Self::FIELD_MARGIN_10; // -1.0m
    pub const FIELD_X_MAX: i32 = Self::FIELD_LENGTH_10 + Self::FIELD_MARGIN_10; // 1060 = 106.0m
    pub const FIELD_Y_MIN: i32 = -Self::FIELD_MARGIN_10; // -1.0m
    pub const FIELD_Y_MAX: i32 = Self::FIELD_WIDTH_10 + Self::FIELD_MARGIN_10; // 690 = 69.0m

    /// 미터 단위 float → 0.1m 정수 (반올림)
    #[inline]
    pub fn from_meters(mx: f32, my: f32) -> Self {
        Self { x: (mx * Self::SCALE).round() as i32, y: (my * Self::SCALE).round() as i32, z: 0 }
    }

    /// 미터 단위 float (3D) → 0.1m 정수 (반올림)
    #[inline]
    pub fn from_meters_3d(mx: f32, my: f32, mz: f32) -> Self {
        Self {
            x: (mx * Self::SCALE).round() as i32,
            y: (my * Self::SCALE).round() as i32,
            z: (mz * Self::SCALE).round() as i32,
        }
    }

    /// 0.1m 정수 → 미터 단위 float
    #[inline]
    pub fn to_meters(&self) -> (f32, f32) {
        (self.x as f32 / Self::SCALE, self.y as f32 / Self::SCALE)
    }

    /// 0.1m 정수 → 미터 단위 float (3D)
    #[inline]
    pub fn to_meters_3d(&self) -> (f32, f32, f32) {
        (self.x as f32 / Self::SCALE, self.y as f32 / Self::SCALE, self.z as f32 / Self::SCALE)
    }

    /// Height in meters
    #[inline]
    pub fn height_m(&self) -> f32 {
        self.z as f32 / Self::SCALE
    }

    /// Create new Coord10 with specified height in meters
    #[inline]
    pub fn with_height_m(self, height_m: f32) -> Self {
        Self { x: self.x, y: self.y, z: (height_m * Self::SCALE).round() as i32 }
    }

    /// Create new Coord10 with specified height in Coord10 units
    #[inline]
    pub fn with_height(self, z: i32) -> Self {
        Self { x: self.x, y: self.y, z }
    }

    /// 필드 범위(마진 포함) 내로 클램프
    ///
    /// - 플레이어: `clamp_in_bounds()` 권장 (항상 인바운드 유지)
    /// - 공: `clamp_to_field()` 허용 (라인 밖 이벤트를 위해 마진 허용)
    pub fn clamp_to_field(self) -> Self {
        Self {
            x: self.x.clamp(Self::FIELD_X_MIN, Self::FIELD_X_MAX),
            y: self.y.clamp(Self::FIELD_Y_MIN, Self::FIELD_Y_MAX),
            z: self.z.clamp(0, Self::MAX_HEIGHT_10),
        }
    }

    /// 필드 인바운드(플레이 영역) 범위 내로 클램프
    #[inline]
    pub fn clamp_in_bounds(self) -> Self {
        Self {
            x: self.x.clamp(Self::IN_BOUNDS_X_MIN, Self::IN_BOUNDS_X_MAX),
            y: self.y.clamp(Self::IN_BOUNDS_Y_MIN, Self::IN_BOUNDS_Y_MAX),
            z: self.z.clamp(0, Self::MAX_HEIGHT_10),
        }
    }

    /// Normalized(0..1) → Coord10 변환.
    ///
    /// - `pos.0`: x normalized (length direction, 0..1 → 0..105m)
    /// - `pos.1`: y normalized (width direction, 0..1 → 0..68m)
    #[inline]
    pub fn from_normalized(pos: (f32, f32)) -> Self {
        Self {
            x: (pos.0 * Self::FIELD_LENGTH_10 as f32).round() as i32,
            y: (pos.1 * Self::FIELD_WIDTH_10 as f32).round() as i32,
            z: 0,
        }
    }

    /// Legacy normalized → Coord10 변환 (축 스왑 포함)
    ///
    /// FIX_2601 Phase 3.5: 레거시 normalized 좌표는 (width, length) 순서 사용
    /// - `pos.0`: width (y-axis, 0..1 → 0..68m)
    /// - `pos.1`: length (x-axis, 0..1 → 0..105m)
    ///
    /// coordinates.rs의 to_meters()와 동일한 축 스왑 로직 사용
    #[inline]
    pub fn from_normalized_legacy(pos: (f32, f32)) -> Self {
        Self {
            x: (pos.1 * Self::FIELD_LENGTH_10 as f32).round() as i32, // length from pos.1
            y: (pos.0 * Self::FIELD_WIDTH_10 as f32).round() as i32,  // width from pos.0
            z: 0,
        }
    }

    /// Coord10 → Normalized(0..1) 변환.
    ///
    /// 반환값: (x/1050, y/680) = (length, width)
    /// 반환값은 클램프하지 않음(경계 밖 값도 그대로 표현).
    #[inline]
    pub fn to_normalized(&self) -> (f32, f32) {
        (self.x as f32 / Self::FIELD_LENGTH_10 as f32, self.y as f32 / Self::FIELD_WIDTH_10 as f32)
    }

    /// Coord10 → Legacy Normalized(0..1) 변환 (축 스왑 포함)
    ///
    /// FIX_2601 Phase 3.6: 레거시 코드와 호환용
    ///
    /// 반환값: (width, length) = (y/680, x/1050)
    /// - .0 = width (sideline direction, 0-1)
    /// - .1 = length (goal direction, 0-1)
    ///
    /// coordinates.rs의 to_normalized()와 동일한 축 순서 사용
    #[inline]
    pub fn to_normalized_legacy(&self) -> (f32, f32) {
        (
            self.y as f32 / Self::FIELD_WIDTH_10 as f32, // width from y
            self.x as f32 / Self::FIELD_LENGTH_10 as f32, // length from x
        )
    }

    /// 인바운드(플레이 영역) 범위 체크
    #[inline]
    pub fn is_in_bounds(&self) -> bool {
        self.x >= Self::IN_BOUNDS_X_MIN
            && self.x <= Self::IN_BOUNDS_X_MAX
            && self.y >= Self::IN_BOUNDS_Y_MIN
            && self.y <= Self::IN_BOUNDS_Y_MAX
    }

    /// 범위 체크 (마진 포함)
    ///
    /// 플레이어의 엄격한 범위 체크는 `is_in_bounds()`를 사용.
    pub fn is_in_field(&self) -> bool {
        self.x >= Self::FIELD_X_MIN
            && self.x <= Self::FIELD_X_MAX
            && self.y >= Self::FIELD_Y_MIN
            && self.y <= Self::FIELD_Y_MAX
    }

    /// 유클리드 거리 (0.1m 단위)
    #[inline]
    pub fn distance_to(&self, other: &Self) -> i32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt().round() as i32
    }

    /// 유클리드 거리 (미터 단위, f32)
    /// FIX_2601 Phase 3.6: 미터 단위 거리가 필요한 경우 사용
    #[inline]
    pub fn distance_to_m(&self, other: &Self) -> f32 {
        self.distance_to(other) as f32 / Self::SCALE
    }

    /// 방향 벡터 (정규화됨, 단위 벡터)
    /// FIX_2601 Phase 3.6: 두 점 사이의 방향 계산
    #[inline]
    pub fn direction_to(&self, other: &Self) -> (f32, f32) {
        let dx = (other.x - self.x) as f32;
        let dy = (other.y - self.y) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            (0.0, 0.0)
        } else {
            (dx / len, dy / len)
        }
    }

    /// 선형 보간 (lerp)
    /// FIX_2601 Phase 3.6: t=0이면 self, t=1이면 target
    #[inline]
    pub fn lerp(&self, target: &Self, t: f32) -> Self {
        Self {
            x: self.x + ((target.x - self.x) as f32 * t).round() as i32,
            y: self.y + ((target.y - self.y) as f32 * t).round() as i32,
            z: self.z + ((target.z - self.z) as f32 * t).round() as i32,
        }
    }

    /// 두 점의 중간점
    #[inline]
    pub fn midpoint(&self, other: &Self) -> Self {
        Self { x: (self.x + other.x) / 2, y: (self.y + other.y) / 2, z: (self.z + other.z) / 2 }
    }

    // ============================================================================
    // FIX_2601 Phase 2: 페널티 에어리어 체크 (Coord10 직접 사용)
    // ============================================================================

    /// 페널티 에어리어 체크 (Coord10 단위)
    ///
    /// 페널티 에어리어 치수:
    /// - Depth: 16.5m from goal line
    /// - Width: y in [13.85m, 54.15m]
    /// Coord10 단위:
    /// - Home(attack right): x in [0, 165], y in [138, 541]
    /// - Away(attack left): x in [885, 1050], y in [138, 541]
    #[inline]
    pub fn in_own_penalty_area(&self, attacks_right: bool) -> bool {
        const PA_X_DEPTH: i32 = 165;
        const PA_X_OFFSET: i32 = 0;
        const PA_Y_MIN: i32 = 138;
        const PA_Y_MAX: i32 = 541;

        if attacks_right {
            self.x >= PA_X_OFFSET
                && self.x <= PA_X_OFFSET + PA_X_DEPTH
                && self.y >= PA_Y_MIN
                && self.y <= PA_Y_MAX
        } else {
            self.x >= Self::FIELD_LENGTH_10 - PA_X_OFFSET - PA_X_DEPTH
                && self.x <= Self::FIELD_LENGTH_10 - PA_X_OFFSET
                && self.y >= PA_Y_MIN
                && self.y <= PA_Y_MAX
        }
    }

    // ============================================================================
    // FIX_2601 Phase 1: Coord10 유니피케이션 - 필수 메서드 추가
    // ============================================================================

    /// 상대 골까지의 거리 (미터 단위)
    ///
    /// # Arguments
    /// * `attacks_right` - true면 오른쪽 골대(105m), false면 왼쪽 골대(0m)
    ///
    /// # Example
    /// ```ignore
    /// use crate::engine::physics_constants::field;
    /// let pos = Coord10::from_meters(80.0, field::CENTER_Y);
    /// let dist = pos.distance_to_goal_m(true); // 25.0m
    /// ```
    #[inline]
    pub fn distance_to_goal_m(&self, attacks_right: bool) -> f32 {
        let goal_x = if attacks_right { Self::FIELD_LENGTH_10 } else { 0 };
        let goal = Self { x: goal_x, y: Self::CENTER_Y, z: 0 };
        self.distance_to_m(&goal)
    }

    /// 공격 진영 1/3 이내에 있는지 판단
    ///
    /// 필드 길이의 1/3 지점:
    /// - attacks_right=true: x > 70m (700 단위)
    /// - attacks_right=false: x < 35m (350 단위)
    #[inline]
    pub fn is_in_attacking_third(&self, attacks_right: bool) -> bool {
        let third = Self::FIELD_LENGTH_10 / 3;
        if attacks_right {
            self.x > third * 2
        } else {
            self.x < third
        }
    }

    /// 자기 진영에 있는지 판단
    ///
    /// 필드 중앙(CENTER_X)을 기준으로:
    /// - attacks_right=true: x <= 525
    /// - attacks_right=false: x >= 525
    #[inline]
    pub fn is_in_own_half(&self, attacks_right: bool) -> bool {
        if attacks_right {
            self.x <= Self::CENTER_X
        } else {
            self.x >= Self::CENTER_X
        }
    }

    /// 상대 진영에 있는지 판단
    ///
    /// `is_in_own_half()`의 역
    #[inline]
    pub fn is_in_opponent_half(&self, attacks_right: bool) -> bool {
        !self.is_in_own_half(attacks_right)
    }

    /// 특정 위치로 전진하는지 판단
    ///
    /// # Arguments
    /// * `to` - 목표 위치
    /// * `attacks_right` - 공격 방향
    ///
    /// # Returns
    /// true면 `to` 방향으로 전진, false면 후진 또는 측면
    #[inline]
    pub fn is_advancing_to(&self, to: Coord10, attacks_right: bool) -> bool {
        if attacks_right {
            to.x > self.x
        } else {
            to.x < self.x
        }
    }
}

impl Default for Coord10 {
    fn default() -> Self {
        Self::CENTER // 필드 중앙
    }
}

impl std::ops::Add for Coord10 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl std::ops::Sub for Coord10 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

// ============================================================================
// TeamViewCoord10: 팀 기준 좌표 (FIX_2601 Phase 4)
// ============================================================================

/// 팀 기준 좌표 - 항상 x=1050 방향으로 공격
///
/// 필드 범위:
/// - x: 0..1050 (0 = 자기 골문, 1050 = 상대 골문)
/// - y: 0..680  (동일)
///
/// 특징:
/// - 모든 팀의 공격 방향이 동일 (x 증가 = 전진)
/// - 공격 로직 단순화
/// - is_home/attacks_right 분기 제거
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamViewCoord10 {
    pub x: i32,
    pub y: i32,
}

impl TeamViewCoord10 {
    /// Own goal (x=0, center Y) - derived from Coord10 constants
    pub const OWN_GOAL: Self = Self { x: 0, y: Coord10::CENTER_Y };

    /// Opponent goal (x=FIELD_LENGTH_10, center Y) - derived from Coord10 constants
    pub const OPPONENT_GOAL: Self = Self { x: Coord10::FIELD_LENGTH_10, y: Coord10::CENTER_Y };

    /// Field center - derived from Coord10 constants
    pub const CENTER: Self = Self { x: Coord10::CENTER_X, y: Coord10::CENTER_Y };

    /// World → TeamView 변환
    ///
    /// - attacks_right=true: 그대로
    /// - attacks_right=false: x축 반전 (1050 - x)
    pub fn from_world(world: Coord10, attacks_right: bool) -> Self {
        if attacks_right {
            Self { x: world.x, y: world.y }
        } else {
            Self { x: Coord10::FIELD_LENGTH_10 - world.x, y: world.y }
        }
    }

    /// TeamView → World 변환
    pub fn to_world(self, attacks_right: bool) -> Coord10 {
        if attacks_right {
            Coord10 { x: self.x, y: self.y, z: 0 }
        } else {
            Coord10 { x: Coord10::FIELD_LENGTH_10 - self.x, y: self.y, z: 0 }
        }
    }

    /// 미터 변환
    pub fn to_meters(&self) -> (f32, f32) {
        (self.x as f32 / Coord10::SCALE, self.y as f32 / Coord10::SCALE)
    }

    /// 유클리드 거리 (0.1m 단위)
    pub fn distance_to(&self, other: &Self) -> i32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt().round() as i32
    }

    /// 공격 진영 여부 (x > 525)
    #[inline]
    pub fn in_attacking_half(&self) -> bool {
        self.x > 525
    }

    /// 수비 진영 여부 (x <= 525)
    #[inline]
    pub fn in_defending_half(&self) -> bool {
        self.x <= 525
    }

    /// 상대 골문까지 거리
    #[inline]
    pub fn distance_to_opponent_goal(&self) -> i32 {
        self.distance_to(&Self::OPPONENT_GOAL)
    }
}

// ============================================================================
// DirectionContext: 방향 컨텍스트 (FIX_2601 Phase 4)
// ============================================================================

/// 팀의 방향 컨텍스트
///
/// 좌표 변환 + 방향 판단의 단일 진실점(SSOT)
///
/// ## 사용 예
/// ```ignore
/// let ctx = DirectionContext::new(true); // 홈팀
///
/// // World → TeamView 변환
/// use crate::engine::physics_constants::field;
/// let world_pos = Coord10::from_meters(80.0, field::CENTER_Y);
/// let team_view = ctx.to_team_view(world_pos);
///
/// // 전진 여부 (team view에서 x 증가 = 전진)
/// let is_forward = team_view.x > world_pos_before.x;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectionContext {
    /// 홈팀 여부 (하프타임에도 불변)
    pub is_home: bool,

    /// 현재 공격 방향 (하프타임에 스왑)
    pub attacks_right: bool,
}

impl DirectionContext {
    /// 새 컨텍스트 생성 (전반전)
    ///
    /// 전반전: 홈팀은 오른쪽 공격, 원정팀은 왼쪽 공격
    pub fn new(is_home: bool) -> Self {
        Self {
            is_home,
            attacks_right: is_home, // 전반전: 홈=오른쪽, 원정=왼쪽
        }
    }

    /// 하프타임 스왑
    pub fn swap_for_second_half(&mut self) {
        self.attacks_right = !self.attacks_right;
    }

    /// World 좌표를 TeamView로 변환
    #[inline]
    pub fn to_team_view(&self, world: Coord10) -> TeamViewCoord10 {
        TeamViewCoord10::from_world(world, self.attacks_right)
    }

    /// TeamView를 World 좌표로 변환
    #[inline]
    pub fn to_world(&self, tv: TeamViewCoord10) -> Coord10 {
        tv.to_world(self.attacks_right)
    }

    /// 공격 골문 (World 좌표) - derived from Coord10 constants
    pub fn attack_goal(&self) -> Coord10 {
        if self.attacks_right {
            Coord10 { x: Coord10::FIELD_LENGTH_10, y: Coord10::CENTER_Y, z: 0 } // Away goal
        } else {
            Coord10 { x: 0, y: Coord10::CENTER_Y, z: 0 } // Home goal
        }
    }

    /// 수비 골문 (World 좌표) - derived from Coord10 constants
    pub fn defend_goal(&self) -> Coord10 {
        if self.attacks_right {
            Coord10 { x: 0, y: Coord10::CENTER_Y, z: 0 } // Home goal
        } else {
            Coord10 { x: Coord10::FIELD_LENGTH_10, y: Coord10::CENTER_Y, z: 0 } // Away goal
        }
    }

    /// 전진 방향 부호 (World 좌표 기준)
    ///
    /// - attacks_right=true: +1 (x 증가 = 전진)
    /// - attacks_right=false: -1 (x 감소 = 전진)
    #[inline]
    pub fn forward_sign(&self) -> i32 {
        if self.attacks_right {
            1
        } else {
            -1
        }
    }

    // =========================================================================
    // FIX_2601/0105: Explicit Direction Methods (NO Y-flip!)
    // =========================================================================

    /// 공격 방향 (f32): Home = +1.0, Away = -1.0
    ///
    /// 명시적 방향 계산에 사용. Y-flip 대신 이 값으로 오프셋 계산.
    #[inline]
    pub fn attack_direction(&self) -> f32 {
        if self.attacks_right {
            1.0
        } else {
            -1.0
        }
    }

    /// 상대 골대 X 위치 (normalized 0.0-1.0)
    ///
    /// Home: 1.0 (x=105m), Away: 0.0 (x=0m)
    #[inline]
    pub fn opponent_goal_x(&self) -> f32 {
        if self.attacks_right {
            1.0
        } else {
            0.0
        }
    }

    /// 자기 골대 X 위치 (normalized 0.0-1.0)
    ///
    /// Home: 0.0 (x=0m), Away: 1.0 (x=105m)
    #[inline]
    pub fn own_goal_x(&self) -> f32 {
        if self.attacks_right {
            0.0
        } else {
            1.0
        }
    }

    /// 공격 방향으로 오프셋 계산
    ///
    /// ```ignore
    /// let offset = ctx.forward_offset(0.05);
    /// // Home: +0.05 (toward x=1.0)
    /// // Away: -0.05 (toward x=0.0)
    /// ```
    #[inline]
    pub fn forward_offset(&self, amount: f32) -> f32 {
        amount * self.attack_direction()
    }

    /// 현재 위치에서 상대 골대까지의 거리 (normalized 0.0-1.0)
    #[inline]
    pub fn distance_to_opponent_goal(&self, pos_x: f32) -> f32 {
        (self.opponent_goal_x() - pos_x).abs()
    }

    /// 위치가 공격적인지 (상대 진영) 판단
    ///
    /// Home: pos_x > 0.5 = attacking
    /// Away: pos_x < 0.5 = attacking
    #[inline]
    pub fn is_attacking_position(&self, pos_x: f32) -> bool {
        if self.attacks_right {
            pos_x > 0.5
        } else {
            pos_x < 0.5
        }
    }
}

// ============================================================================
// Vel10: 속도 (0.1m/s 단위)
// ============================================================================

/// 0.1m/s 단위 속도 벡터
///
/// 스케일: 1 unit = 0.1m/s = 100mm/s
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Vel10 {
    pub vx: i32,
    pub vy: i32,
}

impl Vel10 {
    pub const SCALE: f32 = 10.0; // 1 m/s = 10 units

    /// m/s → 0.1m/s 정수
    pub fn from_mps(vx_mps: f32, vy_mps: f32) -> Self {
        Self {
            vx: (vx_mps * Self::SCALE).round() as i32,
            vy: (vy_mps * Self::SCALE).round() as i32,
        }
    }

    /// 0.1m/s 정수 → m/s
    pub fn to_mps(&self) -> (f32, f32) {
        (self.vx as f32 / Self::SCALE, self.vy as f32 / Self::SCALE)
    }

    /// 50ms tick에서의 이동량 (delta 좌표)
    pub fn to_delta_per_tick_50ms(&self) -> Coord10 {
        // v * 0.05s = v / 20
        Coord10 {
            x: (self.vx as f32 / 20.0).round() as i32,
            y: (self.vy as f32 / 20.0).round() as i32,
            z: 0,
        }
    }

    /// 250ms tick에서의 이동량 (레거시)
    pub fn to_delta_per_tick_250ms(&self) -> Coord10 {
        // v * 0.25s = v / 4
        Coord10 {
            x: (self.vx as f32 / 4.0).round() as i32,
            y: (self.vy as f32 / 4.0).round() as i32,
            z: 0,
        }
    }

    /// 속력 (스칼라)
    pub fn magnitude(&self) -> i32 {
        let vx = self.vx as f32;
        let vy = self.vy as f32;
        (vx * vx + vy * vy).sqrt().round() as i32
    }

    /// 정규화 (단위 벡터, 스케일 유지)
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag == 0 {
            return *self;
        }
        Self {
            vx: (self.vx as f32 / mag as f32 * Self::SCALE).round() as i32,
            vy: (self.vy as f32 / mag as f32 * Self::SCALE).round() as i32,
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_coord10_from_meters() {
        let coord = Coord10::from_meters(field::CENTER_X, field::CENTER_Y);     
        assert_eq!(coord.x, Coord10::CENTER_X);
        assert_eq!(coord.y, Coord10::CENTER_Y);
    }

    #[test]
    fn test_coord10_to_meters() {
        let coord = Coord10 { x: Coord10::CENTER_X, y: Coord10::CENTER_Y, z: 0 };
        let (mx, my) = coord.to_meters();
        assert!((mx - field::CENTER_X).abs() < 0.01);
        assert!((my - field::CENTER_Y).abs() < 0.01);
    }

    #[test]
    fn test_coord10_clamp() {
        let out = Coord10 { x: 2000, y: -100, z: 600 };
        let clamped = out.clamp_to_field();
        assert_eq!(clamped.x, Coord10::FIELD_X_MAX);
        assert_eq!(clamped.y, Coord10::FIELD_Y_MIN);
        assert_eq!(clamped.z, Coord10::MAX_HEIGHT_10);
    }

    #[test]
    fn test_coord10_clamp_in_bounds() {
        let out = Coord10 { x: 2000, y: -100, z: 600 };
        let clamped = out.clamp_in_bounds();
        assert_eq!(clamped.x, Coord10::IN_BOUNDS_X_MAX);
        assert_eq!(clamped.y, Coord10::IN_BOUNDS_Y_MIN);
        assert_eq!(clamped.z, Coord10::MAX_HEIGHT_10);
    }

    #[test]
    fn test_coord10_distance() {
        let a = Coord10::from_meters(0.0, 0.0);
        let b = Coord10::from_meters(3.0, 4.0);
        let dist = a.distance_to(&b);
        assert_eq!(dist, 50); // 5.0m * 10 = 50 units
    }

    #[test]
    fn test_vel10_to_delta_50ms() {
        let vel = Vel10::from_mps(10.0, 0.0); // 10 m/s
        let delta = vel.to_delta_per_tick_50ms();
        assert_eq!(delta.x, 5); // 10 * 0.05 * 10 = 5 units (0.5m)
        assert_eq!(delta.y, 0);
    }

    #[test]
    fn test_vel10_magnitude() {
        let vel = Vel10::from_mps(3.0, 4.0);
        let mag = vel.magnitude();
        assert_eq!(mag, 50); // 5.0 m/s * 10 = 50 units
    }

    // =========================================================================
    // TeamViewCoord10 Tests (FIX_2601 Phase 4)
    // =========================================================================

    #[test]
    fn test_team_view_from_world_attacks_right() {
        let world = Coord10 { x: 800, y: 340, z: 0 };
        let tv = TeamViewCoord10::from_world(world, true);
        assert_eq!(tv.x, 800);
        assert_eq!(tv.y, 340);
    }

    #[test]
    fn test_team_view_from_world_attacks_left() {
        let world = Coord10 { x: 800, y: 340, z: 0 };
        let tv = TeamViewCoord10::from_world(world, false);
        assert_eq!(tv.x, 250); // 1050 - 800 = 250
        assert_eq!(tv.y, 340);
    }

    #[test]
    fn test_team_view_roundtrip() {
        let original = Coord10 { x: 300, y: 400, z: 0 };

        for attacks_right in [true, false] {
            let tv = TeamViewCoord10::from_world(original, attacks_right);
            let back = tv.to_world(attacks_right);
            assert_eq!(original, back, "Roundtrip failed for attacks_right={}", attacks_right);
        }
    }

    #[test]
    fn test_team_view_distance_invariant() {
        // 거리는 좌표계 변환 후에도 동일해야 함
        let a = Coord10 { x: 100, y: 200, z: 0 };
        let b = Coord10 { x: 400, y: 600, z: 0 };

        let d_world = a.distance_to(&b);

        for attacks_right in [true, false] {
            let a_tv = TeamViewCoord10::from_world(a, attacks_right);
            let b_tv = TeamViewCoord10::from_world(b, attacks_right);
            let d_tv = a_tv.distance_to(&b_tv);
            assert_eq!(
                d_world, d_tv,
                "Distance invariant violated for attacks_right={}",
                attacks_right
            );
        }
    }

    #[test]
    fn test_team_view_center_invariant() {
        // 중앙은 변환 후에도 중앙이어야 함
        let center = Coord10::CENTER;

        for attacks_right in [true, false] {
            let tv = TeamViewCoord10::from_world(center, attacks_right);
            assert_eq!(
                tv,
                TeamViewCoord10::CENTER,
                "Center invariant violated for attacks_right={}",
                attacks_right
            );
        }
    }

    #[test]
    fn test_team_view_attacking_half() {
        let attacking = TeamViewCoord10 { x: 600, y: 340 };
        let defending = TeamViewCoord10 { x: 400, y: 340 };

        assert!(attacking.in_attacking_half());
        assert!(!attacking.in_defending_half());

        assert!(!defending.in_attacking_half());
        assert!(defending.in_defending_half());
    }

    // =========================================================================
    // DirectionContext Tests (FIX_2601 Phase 4)
    // =========================================================================

    #[test]
    fn test_direction_context_home_first_half() {
        let ctx = DirectionContext::new(true);
        assert!(ctx.is_home);
        assert!(ctx.attacks_right);
        assert_eq!(ctx.forward_sign(), 1);
    }

    #[test]
    fn test_direction_context_away_first_half() {
        let ctx = DirectionContext::new(false);
        assert!(!ctx.is_home);
        assert!(!ctx.attacks_right);
        assert_eq!(ctx.forward_sign(), -1);
    }

    #[test]
    fn test_direction_context_swap_half() {
        let mut ctx = DirectionContext::new(true);
        assert!(ctx.attacks_right);

        ctx.swap_for_second_half();
        assert!(ctx.is_home); // 팀 정체성은 불변
        assert!(!ctx.attacks_right); // 공격 방향만 스왑
    }

    #[test]
    fn test_direction_context_attack_goal() {
        let home_ctx = DirectionContext::new(true);
        let away_ctx = DirectionContext::new(false);

        // 홈팀은 x=1050 방향 공격
        assert_eq!(home_ctx.attack_goal(), Coord10 { x: 1050, y: 340, z: 0 });
        assert_eq!(home_ctx.defend_goal(), Coord10 { x: 0, y: 340, z: 0 });

        // 원정팀은 x=0 방향 공격
        assert_eq!(away_ctx.attack_goal(), Coord10 { x: 0, y: 340, z: 0 });
        assert_eq!(away_ctx.defend_goal(), Coord10 { x: 1050, y: 340, z: 0 });
    }

    #[test]
    fn test_direction_context_to_team_view() {
        let ctx = DirectionContext::new(true);
        let world = Coord10 { x: 800, y: 340, z: 0 };

        let tv = ctx.to_team_view(world);
        let back = ctx.to_world(tv);

        assert_eq!(world, back);
    }
}
