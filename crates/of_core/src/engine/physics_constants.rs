//! Physics constants for match simulation
//!
//! Based on Open Football Engine approach with probability-based mechanics.

// ============================================================
// Ball Physics V2: Substep System
// ============================================================
pub mod substep {
    /// Physics substep duration (ms) - Open Football uses 10ms
    pub const SUBSTEP_MS: f32 = 10.0;

    /// Game tick duration (ms)
    pub const TICK_MS: f32 = 50.0;

    /// Substeps per game tick (50ms / 10ms = 5)
    pub const SUBSTEPS_PER_TICK: usize = 5;

    /// Substep duration in seconds (0.01s)
    pub const SUBSTEP_SEC: f32 = 0.01;
}

// ============================================================
// Ball Physics V2: Projectile Motion (Gravity)
// MVP: Flat/Arc/Lob 3개 프로파일
// ============================================================
pub mod projectile {
    //! 중력 기반 포물선 궤적 상수
    //!
    //! MVP: HeightProfile이 Flat/Arc/Lob으로 단순화됨
    //! - Driven은 Arc에 통합 (낮은 lift_ratio로 구현)
    //! - Header는 ContactType::Head로 분리

    /// 중력 가속도 (0.1m/s² 단위): 9.81 m/s² = 98.1 → 98
    pub const GRAVITY_01M: i16 = 98;

    // ========================================
    // HeightProfile별 초기 수직 속도 (0.1m/s)
    // v = √(2gh) 로 계산
    // ========================================

    /// Flat: 땅볼 - 수직 속도 없음
    pub const VZ_FLAT: i16 = 0;

    /// Arc: 일반 슛/패스 - max height ~3.5m
    /// v = √(2 × 9.81 × 3.5) ≈ 8.28 m/s → 83
    pub const VZ_ARC: i16 = 83;

    /// Lob: 로빙/칩/클리어 - max height ~10m
    /// v = √(2 × 9.81 × 10.0) ≈ 14.0 m/s → 140
    pub const VZ_LOB: i16 = 140;

    // ========================================
    // Height Caps (미터) - 참조용
    // ========================================
    pub const HEIGHT_CAP_FLAT: f32 = 0.0;
    pub const HEIGHT_CAP_ARC: f32 = 3.5;
    pub const HEIGHT_CAP_LOB: f32 = 10.0;
}

/// Ball Physics V2 Phase 3: Bounce constants
pub mod bounce {
    // ========================================
    // 반발 계수 (Coefficient of Restitution)
    // ========================================

    /// 잔디 반발 계수 (FIFA 규정: 0.6~0.9)
    /// 0.65 = 착지 시 65% 속도 유지
    pub const GRASS_COR: f32 = 0.65;

    /// 골포스트 반발 계수 (금속)
    pub const POST_COR: f32 = 0.75;

    /// 크로스바 반발 계수
    pub const CROSSBAR_COR: f32 = 0.70;

    /// 광고판 반발 계수 (낮음)
    pub const BOARD_COR: f32 = 0.40;

    // ========================================
    // 바운스 임계값
    // ========================================

    /// 최소 바운스 높이 (m) - 이하면 롤링 전환
    pub const MIN_BOUNCE_HEIGHT_M: f32 = 0.1;

    /// 최소 바운스 속도 (0.1m/s) - 이하면 롤링 전환
    /// √(2 × 9.81 × 0.1) ≈ 1.4 m/s → 14 (0.1m/s)
    pub const MIN_BOUNCE_VZ: i16 = 14;

    /// 바운스당 수평 속도 손실 비율
    pub const HORIZONTAL_LOSS: f32 = 0.12;

    /// 최대 바운스 횟수 - 초과 시 강제 롤링
    pub const MAX_BOUNCES: u8 = 5;

    // ========================================
    // 스핀 영향
    // ========================================

    /// 스핀 편향 계수 (curve_factor 영향)
    /// 바운스 시 스핀 방향으로 속도 편향
    pub const SPIN_DEFLECTION: f32 = 0.25;

    /// 바운스 후 스핀 감소율
    pub const SPIN_DECAY: f32 = 0.7;
}

/// Ball physics constants
pub mod ball {
    /// Ball mass (kg)
    pub const MASS_KG: f32 = 0.43;
    /// Ball drag coefficient (air resistance)
    pub const DRAG_COEFFICIENT: f32 = 0.25;
    /// Ball rolling resistance on grass
    pub const ROLLING_RESISTANCE: f32 = 0.02;
    /// Gravity constant (m/s²)
    pub const GRAVITY: f32 = 9.81;
    /// Minimum velocity before ball stops (m/s)
    pub const MIN_VELOCITY: f32 = 0.1;
    /// Ball ownership threshold distance (meters)
    pub const OWNERSHIP_THRESHOLD_M: f32 = 5.0;
    /// Maximum ball height (meters) - high clearances, goal kicks
    pub const HEIGHT_MAX_M: f32 = 10.0;
}

/// Aerial duel constants
pub mod aerial {
    /// FIFA goal crossbar height (meters)
    pub const GOAL_CROSSBAR_M: f32 = 2.44;
    /// Maximum header reach: player height (~1.8m) + jump (~0.8m) + head (~0.3m)
    pub const HEADER_MAX_M: f32 = 2.9;
    /// Maximum GK catch reach: keeper height (~1.95m) + arms (~0.5m) + jump (~0.85m)
    pub const GK_CATCH_MAX_M: f32 = 3.3;
    /// Minimum height for aerial duel trigger (meters)
    pub const AERIAL_DUEL_MIN_M: f32 = 0.5;
    /// GK reflexes skill weight for save calculation
    pub const GK_REFLEXES_WEIGHT: f32 = 0.4;
    /// GK diving skill weight for save calculation
    pub const GK_DIVING_WEIGHT: f32 = 0.3;
    /// GK positioning skill weight for save calculation
    pub const GK_POSITIONING_WEIGHT: f32 = 0.2;
    /// GK handling skill weight for save calculation
    pub const GK_HANDLING_WEIGHT: f32 = 0.1;
}

/// Field dimensions (meters)
/// Per ENGINE_CONTRACT.md Section 0.1 and 2
pub mod field {
    /// Field length in meters
    pub const LENGTH_M: f32 = 105.0;
    /// Field width in meters
    pub const WIDTH_M: f32 = 68.0;
    /// Center X coordinate (meters)
    pub const CENTER_X: f32 = LENGTH_M * 0.5;
    /// Center Y coordinate (meters)
    pub const CENTER_Y: f32 = WIDTH_M * 0.5;
    /// Center circle radius (meters)
    pub const CENTER_CIRCLE_RADIUS_M: f32 = 9.15;
    /// Penalty area length from goal line
    pub const PENALTY_AREA_LENGTH_M: f32 = 16.5;
    /// Penalty spot distance from goal line (meters)
    pub const PENALTY_SPOT_M: f32 = 11.0;

    // =========================================================================
    // Coord10 버전 (0.1m 단위 정수)
    // FIX_2601: 하드코딩 제거를 위한 SSOT
    // =========================================================================

    /// Field length in Coord10 units (105.0m × 10 = 1050)
    pub const LENGTH_COORD10: i32 = 1050;
    /// Field width in Coord10 units (68.0m × 10 = 680)
    pub const WIDTH_COORD10: i32 = 680;
    /// Center X in Coord10 units (derived)
    pub const CENTER_X_COORD10: i32 = LENGTH_COORD10 / 2;
    /// Center Y in Coord10 units (derived)
    pub const CENTER_Y_COORD10: i32 = WIDTH_COORD10 / 2;
}

/// Goal dimensions and collision detection
/// Per ENGINE_CONTRACT.md Section 3
pub mod goal {
    use super::field;

    /// Goal width in meters (FIFA standard)
    pub const WIDTH_M: f32 = 7.32;
    /// Goal height / crossbar height in meters (FIFA standard)
    pub const HEIGHT_M: f32 = 2.44;
    /// Goal post radius in meters (diameter ~12cm)
    pub const POST_RADIUS_M: f32 = 0.06;
    /// Goal half-width (for Y range calculation)
    pub const HALF_WIDTH_M: f32 = 3.66; // 7.32 / 2
    /// Goal Y-axis minimum (CENTER_Y - HALF_WIDTH)
    pub const Y_MIN: f32 = field::CENTER_Y - HALF_WIDTH_M;
    /// Goal Y-axis maximum (CENTER_Y + HALF_WIDTH)
    pub const Y_MAX: f32 = field::CENTER_Y + HALF_WIDTH_M;
    /// Left goal line X position (Home goal)
    pub const LEFT_X: f32 = 0.0;
    /// Right goal line X position (Away goal)
    pub const RIGHT_X: f32 = field::LENGTH_M;

    // =========================================================================
    // Goal center positions
    // FIX_2601: 하드코딩 제거를 위한 SSOT
    // =========================================================================

    /// Home goal center (x=0, y=CENTER_Y)
    pub const HOME_CENTER_M: (f32, f32) = (0.0, field::CENTER_Y);
    /// Away goal center (x=LENGTH_M, y=CENTER_Y)
    pub const AWAY_CENTER_M: (f32, f32) = (field::LENGTH_M, field::CENTER_Y);

    // =========================================================================
    // Coord10 버전 (0.1m 단위 정수)
    // =========================================================================

    /// Home goal center in Coord10
    pub const HOME_CENTER_COORD10: (i32, i32) = (0, field::CENTER_Y_COORD10);
    /// Away goal center in Coord10
    pub const AWAY_CENTER_COORD10: (i32, i32) = (field::LENGTH_COORD10, field::CENTER_Y_COORD10);

    // =========================================================================
    // Direction-aware goal functions
    // FIX_2601/0123: Renamed is_home → attacks_right for halftime awareness
    // =========================================================================

    /// Attack goal X position based on attack direction
    ///
    /// FIX_2601/0123: Use `attacks_right` instead of `is_home` for halftime-correct behavior.
    /// - `attacks_right = true`: attacking toward x=105 (RIGHT_X)
    /// - `attacks_right = false`: attacking toward x=0 (LEFT_X)
    #[inline]
    pub const fn attack_goal_x(attacks_right: bool) -> f32 {
        if attacks_right {
            RIGHT_X
        } else {
            LEFT_X
        }
    }

    /// Defend goal X position based on attack direction
    #[inline]
    pub const fn defend_goal_x(attacks_right: bool) -> f32 {
        if attacks_right {
            LEFT_X
        } else {
            RIGHT_X
        }
    }

    /// Attack goal center (x, y) based on attack direction
    ///
    /// FIX_2601/0123: Returns AWAY_CENTER when attacking right (toward x=105)
    #[inline]
    pub const fn attack_goal(attacks_right: bool) -> (f32, f32) {
        if attacks_right {
            AWAY_CENTER_M
        } else {
            HOME_CENTER_M
        }
    }

    /// Defend goal center (x, y) based on attack direction
    #[inline]
    pub const fn defend_goal(attacks_right: bool) -> (f32, f32) {
        if attacks_right {
            HOME_CENTER_M
        } else {
            AWAY_CENTER_M
        }
    }
}

/// Home advantage bonuses
pub mod home_advantage {
    /// Overall team strength multiplier: 5%
    pub const STRENGTH_MULTIPLIER: f32 = 1.05;
    /// Shot accuracy bonus
    pub const SHOT_ACCURACY_BONUS: f32 = 0.03;
    /// Pass success rate bonus
    pub const PASS_SUCCESS_BONUS: f32 = 0.02;
    /// Tackle success bonus
    pub const TACKLE_SUCCESS_BONUS: f32 = 0.04;
    /// Composure bonus under pressure
    pub const COMPOSURE_BONUS: f32 = 0.05;
}

/// Shot power constants
pub mod shot {
    /// Base shot power (m/s)
    pub const BASE_POWER_MPS: f32 = 15.0;
    /// Maximum additional power (m/s)
    pub const MAX_ADDITIONAL_POWER_MPS: f32 = 15.0;
    /// Minimum shot power (m/s)
    pub const MIN_POWER_MPS: f32 = 12.0;
    /// Maximum shot power (m/s)
    pub const MAX_POWER_MPS: f32 = 32.0;
}

/// Shooting zones and distances
pub mod zones {
    /// Very close range (penalty box inner)
    pub const VERY_CLOSE_M: f32 = 10.0;
    /// Close range (inside penalty area)
    pub const CLOSE_M: f32 = 16.5;
    /// Mid range (edge of box)
    pub const MID_RANGE_M: f32 = 25.0;
    /// Long range
    pub const LONG_RANGE_M: f32 = 35.0;
    /// Very long range (half-field shots)
    pub const VERY_LONG_M: f32 = 40.0;
}

/// Pass distance scoring
pub mod pass {
    /// Very short pass threshold
    pub const VERY_SHORT_M: f32 = 10.0;
    /// Short pass threshold
    pub const SHORT_M: f32 = 25.0;
    /// Optimal range end
    pub const OPTIMAL_MAX_M: f32 = 60.0;
    /// Long pass threshold
    pub const LONG_M: f32 = 100.0;
}

/// Pressure and interception
pub mod pressure {
    /// Very close opponent distance
    pub const VERY_CLOSE_M: f32 = 3.0;
    /// Close opponent distance
    pub const CLOSE_M: f32 = 7.0;
    /// Shot blocking distance
    pub const BLOCKING_M: f32 = 5.0;
    /// Maximum pressure penalty
    pub const MAX_PENALTY: f32 = 0.3;

    // === Advanced Pressure System (2025-12-07) ===

    /// Tight pressure radius - physical contact imminent
    pub const RADIUS_TIGHT_M: f32 = 1.5;
    /// Loose pressure radius - general marking
    pub const RADIUS_LOOSE_M: f32 = 3.0;
    /// Extended pressure radius - influences decisions
    pub const RADIUS_EXTENDED_M: f32 = 5.0;

    // === Defender Angle Thresholds ===
    // Based on dot product: 1.0 = front, 0.0 = side, -1.0 = back

    /// Defender blocking front (cos ~60°)
    pub const ANGLE_FRONT_BLOCK: f32 = 0.5;
    /// Defender chasing from behind (cos ~110°)
    pub const ANGLE_BEHIND_CHASE: f32 = -0.3;

    // === Pressure Penalties ===

    /// Dribble penalty when defender in front
    pub const DRIBBLE_FRONT_PENALTY: f32 = 0.9;
    /// Dribble bonus when defender behind (chase situation)
    pub const DRIBBLE_CHASE_BONUS: f32 = 2.0;
    /// Pass accuracy penalty under tight pressure
    pub const PASS_TIGHT_PENALTY: f32 = 0.3;
    /// Shot quality penalty under pressure
    pub const SHOT_PRESSURE_PENALTY: f32 = 0.4;

    // === Composure Mitigation ===

    /// Minimum composure mitigation (10%)
    pub const COMPOSURE_MIN_MITIGATION: f32 = 0.1;
    /// Maximum composure mitigation (50%)
    pub const COMPOSURE_MAX_MITIGATION: f32 = 0.5;
}

// ============================================================
// FIX_2601/0106 P3-11: Unified Attribute Normalization
// 다양한 능력치 스케일을 0.0-1.0으로 정규화
// ============================================================

/// Unified attribute normalization module
///
/// 두 가지 주요 능력치 스케일 지원:
/// - FM 스타일: 1-20 범위 (Football Manager)
/// - OFB 스타일: 0-100 범위 (Open Football)
///
/// # Example
/// ```ignore
/// use crate::engine::physics_constants::attribute;
///
/// // FM-style (1-20)
/// let pace = attribute::from_fm(15.0);  // → 0.75
///
/// // OFB-style (0-100)
/// let speed = attribute::from_100(75.0);  // → 0.75
/// ```
pub mod attribute {
    /// FM-style maximum (Football Manager: 1-20)
    pub const FM_MAX: f32 = 20.0;

    /// OFB-style maximum (Open Football: 0-100)
    pub const OFB_MAX: f32 = 100.0;

    /// Normalize FM-style attribute (1-20) to 0.0-1.0
    ///
    /// # Example
    /// ```ignore
    /// let normalized = attribute::from_fm(15.0);  // → 0.75
    /// ```
    #[inline]
    pub fn from_fm(value: f32) -> f32 {
        (value / FM_MAX).clamp(0.0, 1.0)
    }

    /// Normalize OFB-style attribute (0-100) to 0.0-1.0
    ///
    /// # Example
    /// ```ignore
    /// let normalized = attribute::from_100(75.0);  // → 0.75
    /// ```
    #[inline]
    pub fn from_100(value: f32) -> f32 {
        (value / OFB_MAX).clamp(0.0, 1.0)
    }

    /// Generic normalization with custom maximum
    ///
    /// # Example
    /// ```ignore
    /// let normalized = attribute::normalize(150.0, 200.0);  // → 0.75
    /// ```
    #[inline]
    pub fn normalize(value: f32, max: f32) -> f32 {
        if max <= 0.0 {
            return 0.0;
        }
        (value / max).clamp(0.0, 1.0)
    }
}

/// Legacy skill normalization (backward compatibility)
///
/// Use `attribute::from_fm()` for new code.
pub mod skills {
    use super::attribute;

    /// Maximum attribute value (FM style) - use `attribute::FM_MAX`
    pub const MAX_ATTRIBUTE: f32 = attribute::FM_MAX;

    /// Normalize attribute to 0-1 range
    ///
    /// Equivalent to `attribute::from_fm(value)`
    #[inline]
    pub fn normalize(value: f32) -> f32 {
        attribute::from_fm(value)
    }
}

// ============================================================
// FIX_2601/0106: Consolidated Skill Weights
// 중복된 가중치들을 SSOT로 통합
// ============================================================
pub mod weights {
    //! 스킬 가중치 상수
    //!
    //! 기존에 분산되어 있던 가중치들을 통합.
    //! 일관된 게임 밸런스를 위해 이 상수들을 사용.

    // ========================================
    // Tackle Weights (태클 계산)
    // ========================================

    /// 수비 태클 가중치 (위치 기반): tac(40%) + pos(25%) + ant(20%) + agg(15%)
    /// Used in: attribute_calc.rs, actions.rs (defensive positioning context)
    pub const TACKLE_DEFENSE: [f32; 4] = [0.40, 0.25, 0.20, 0.15];

    /// 물리 듀얼 태클 가중치: tac(40%) + agg(20%) + str(20%) + brav(10%) + agi(10%)
    /// Used in: duel.rs, ball_physics.rs (physical challenge context)
    pub const TACKLE_DUEL: [f32; 5] = [0.40, 0.20, 0.20, 0.10, 0.10];

    /// CB/RB/LB 태클 가중치: tac(40%) + pace(30%) + pos(30%)
    /// Used in: probability.rs, action_decision.rs (defender positioning)
    pub const TACKLE_FULLBACK: [f32; 3] = [0.40, 0.30, 0.30];

    /// CDM 태클 가중치: tac(35%) + ant(35%) + pos(30%)
    /// Used in: probability.rs, action_decision.rs (midfield shield)
    pub const TACKLE_CDM: [f32; 3] = [0.35, 0.35, 0.30];

    // ========================================
    // Header Weights (헤딩 계산)
    // ========================================

    /// 헤딩 성공률 가중치: head(40%) + jump(25%) + pos(20%) + str(15%)
    pub const HEADER_SUCCESS: [f32; 4] = [0.40, 0.25, 0.20, 0.15];

    /// 에어리얼 듀얼 강도: head(35%) + jump(30%) + str(25%) + agg(10%)
    pub const AERIAL_STRENGTH: [f32; 4] = [0.35, 0.30, 0.25, 0.10];

    // ========================================
    // Shot/Pass Weights (슛/패스 계산)
    // ========================================

    /// 슛 정확도 가중치: fin(50%) + comp(30%) + tech(20%)
    pub const SHOT_ACCURACY: [f32; 3] = [0.50, 0.30, 0.20];

    /// 패스 성공률 가중치: pass(50%) + vis(30%) + tech(20%)
    pub const PASS_SUCCESS: [f32; 3] = [0.50, 0.30, 0.20];

    // ========================================
    // Helper Functions
    // ========================================

    /// 4개 스킬 가중 평균 계산
    #[inline]
    pub fn weighted_4(skills: [f32; 4], weights: [f32; 4]) -> f32 {
        skills[0] * weights[0]
            + skills[1] * weights[1]
            + skills[2] * weights[2]
            + skills[3] * weights[3]
    }

    /// 5개 스킬 가중 평균 계산
    #[inline]
    pub fn weighted_5(skills: [f32; 5], weights: [f32; 5]) -> f32 {
        skills[0] * weights[0]
            + skills[1] * weights[1]
            + skills[2] * weights[2]
            + skills[3] * weights[3]
            + skills[4] * weights[4]
    }

    /// 3개 스킬 가중 평균 계산
    #[inline]
    pub fn weighted_3(skills: [f32; 3], weights: [f32; 3]) -> f32 {
        skills[0] * weights[0] + skills[1] * weights[1] + skills[2] * weights[2]
    }
}

/// Action distance thresholds (P1-8 consolidation)
pub mod action_thresholds {
    /// Pass interception path check distance (meters)
    pub const PASS_INTERCEPTION_M: f32 = 5.0;
    /// Marker "completely free" distance (meters)
    pub const MARKER_FREE_M: f32 = 10.0;
    /// Marker "some space" distance (meters)
    pub const MARKER_SPACE_M: f32 = 5.0;
    /// Marker "tight marking" distance (meters)
    pub const MARKER_TIGHT_M: f32 = 2.0;
    /// Nearby opponent check distance for pressure (meters)
    pub const NEARBY_OPPONENT_M: f32 = 10.0;
}

/// Optimal shooting distance thresholds
pub mod shooting_zones {
    /// Optimal shooting minimum distance (meters)
    pub const OPTIMAL_MIN_M: f32 = 13.0;
    /// Optimal shooting maximum distance (meters)
    pub const OPTIMAL_MAX_M: f32 = 23.0;
}

/// FIX_2601/0114: 강제 중/장거리 슈팅 상수
/// 슈팅 정확도를 35-40%로 낮추기 위해 저품질 슈팅 강제 발생
pub mod forced_shot {
    /// 중거리 슈팅 최소 거리 (m) - 더 먼 거리에서 슈팅
    pub const MEDIUM_RANGE_MIN_M: f32 = 20.0;  // 18→20: 더 먼 거리에서만
    /// 중거리 슈팅 최대 거리 (m) - 장거리 포함
    pub const MEDIUM_RANGE_MAX_M: f32 = 35.0;  // 32→35: 더 먼 장거리 포함
    /// 강제 슈팅 확률 (틱당, 조건 충족 시)
    /// FIX_2601/0116: 비활성화 - 예산 시스템으로 대체
    pub const FORCED_SHOT_PROBABILITY: f32 = 0.0;  // 비활성화
    /// 압박 임계값 (이 이상이면 강제 슈팅 불가)
    pub const MAX_PRESSURE_FOR_FORCED_SHOT: f32 = 0.55;  // 0.65→0.55: 압박 시 슈팅 억제
}

/// Offside trap tactical bonuses
/// FIX_2601/0112: 하이브리드 방식 - 점수 + 시그모이드 확률 변환
pub mod offside_trap {
    /// Trap tactic activation bonus (reduced from 5.0)
    pub const TRAP_ACTIVATION_BONUS: f32 = 1.0;
    /// Very high defensive line bonus (reduced from 3.0)
    pub const LINE_VERY_HIGH_BONUS: f32 = 1.0;
    /// High defensive line bonus (reduced from 2.0)
    pub const LINE_HIGH_BONUS: f32 = 0.5;
    /// Normal defensive line bonus
    pub const LINE_NORMAL_BONUS: f32 = 0.0;
    /// Deep defensive line penalty (reduced from -2.0)
    pub const LINE_DEEP_PENALTY: f32 = -0.5;
    /// Very deep defensive line penalty (reduced from -4.0)
    pub const LINE_VERY_DEEP_PENALTY: f32 = -1.0;

    // ========================================
    // FIX_2601/0112: 하이브리드 방식 추가 상수
    // Open-Football 참고 + 시그모이드 확률 변환
    // ========================================

    /// 팀 평균 팀워크 최소 임계값 (Open-Football 참고)
    /// 이 값 미만이면 오프사이드 트랩 비활성화
    pub const MIN_TEAM_TEAMWORK: f32 = 65.0;

    /// 시그모이드 스케일 팩터
    /// diff * SIGMOID_SCALE이 시그모이드 입력
    /// 0.1 = diff 10일 때 73% 트랩 확률
    pub const SIGMOID_SCALE: f32 = 0.1;

    /// 트랩 성공률 상한 (70%)
    /// 아무리 좋은 수비도 30%는 뚫림
    pub const MAX_TRAP_SUCCESS: f32 = 0.70;

    /// 라인 브레이크 성공률 상한 (70%)
    /// 아무리 좋은 공격도 30%는 걸림
    pub const MAX_LINE_BREAK: f32 = 0.70;
}

/// Offside detection constants
/// FIX_2601/0112: 실제 FIFA 룰 기준으로 수정
pub mod offside {
    /// 최소 패스 거리 (m) - FIFA 룰에는 없음
    /// 실제 룰: 어떤 거리의 패스든 오프사이드 가능
    pub const MIN_PASS_DISTANCE_M: f32 = 0.0;

    /// 오프사이드 버퍼 (정규화 좌표)
    /// FIFA 룰: 같은 라인 = 오프사이드 아님, VAR는 cm 단위 판정
    /// 0.005 = ~0.5m (측정 오차 + 같은 라인 허용)
    pub const OFFSIDE_BUFFER_NORM: f32 = 0.005;

    /// 라인즈맨 정확도 (미세 판정)
    /// 실제 부심 정확도 반영 (VAR 없는 경우)
    pub const LINESMAN_ACCURACY: f32 = 0.85;
}

/// ✅ Phase B: Hero Gravity System
/// "주인공 중심 플레이" - 육성 게임에서 주인공이 공기처럼 취급받지 않도록
pub mod hero_gravity {
    /// B1: Pass Priority Boost - "애매하면 주인공한테 줘라"
    pub const PASS_PRIORITY_MULTIPLIER: f32 = 1.3;

    /// B2: Loose Ball Magnet - "공은 주인공을 좋아한다"
    pub const LOOSE_BALL_DISTANCE_BONUS_M: f32 = 1.0; // 1m 차이는 투지로 극복

    /// B3: Hero's Will - "나에게 공을 다오"
    pub const MOVE_TO_BALL_MULTIPLIER: f32 = 1.5;
    pub const ATTACK_GOAL_MULTIPLIER: f32 = 1.2;
    pub const HOLD_BALL_MULTIPLIER: f32 = 0.5;
}

// ============================================================
// P15: Player Inertia Physics System
// 선수 관성 물리 시스템 - 모든 선수 움직임의 기반 레이어
// ============================================================
pub mod player_inertia {
    //! # 스탯 → 물리 파라미터 매핑
    //!
    //! 기존 36개 PlayerAttributes를 물리 파라미터로 해석.
    //! 새 파라미터를 만들지 않고 기존 스탯 활용.
    //!
    //! ## 매핑 규칙
    //! - pace → max_speed (최고 속도)
    //! - acceleration → accel (가속도)
    //! - balance + strength + agility → decel (감속력)
    //! - agility + balance → turn_skill (회전 능력)
    //! - stamina + natural_fitness → drag (마찰/드래그)

    // ============================================================
    // 최고속도 (pace → m/s)
    // 축구 선수 전력질주: 7.0~9.5 m/s (25~34 km/h)
    // ============================================================
    /// 최저 속도 (pace=0)
    pub const MAX_SPEED_BASE: f32 = 7.0;
    /// pace 100일 때 추가 속도
    pub const MAX_SPEED_RANGE: f32 = 2.5;

    // ============================================================
    // 가속도 (acceleration → m/s²)
    // 정지→전력 도달 시간: 2~4초 → 2.5~4.5 m/s²
    // ============================================================
    /// 최저 가속도 (acceleration=0)
    pub const ACCEL_BASE: f32 = 2.5;
    /// acceleration 100일 때 추가 가속도
    pub const ACCEL_RANGE: f32 = 2.0;

    // ============================================================
    // 감속도 (balance + strength + agility → m/s²)
    // 급정거/급턴에서 차이: 3.0~6.0 m/s²
    // ============================================================
    /// 최저 감속도 (decel=0)
    pub const DECEL_BASE: f32 = 3.0;
    /// 감속 스킬 100일 때 추가 감속도
    pub const DECEL_RANGE: f32 = 3.0;
    /// balance 가중치
    pub const DECEL_BALANCE_WEIGHT: f32 = 0.5;
    /// strength 가중치
    pub const DECEL_STRENGTH_WEIGHT: f32 = 0.3;
    /// agility 가중치
    pub const DECEL_AGILITY_WEIGHT: f32 = 0.2;

    // ============================================================
    // 턴 페널티 (agility + balance → turn_skill)
    // 고속에서 급격한 방향전환 시 속도 손실
    // ============================================================
    /// agility 가중치
    pub const TURN_AGILITY_WEIGHT: f32 = 0.7;
    /// balance 가중치
    pub const TURN_BALANCE_WEIGHT: f32 = 0.3;
    /// 최소 턴 페널티 (완전 정지 방지)
    pub const TURN_PENALTY_MIN: f32 = 0.55;

    // ============================================================
    // 드래그/마찰 (stamina + natural_fitness)
    // 지속 이동 시 자연 감속
    // ============================================================
    /// 기본 드래그 계수
    pub const DRAG_BASE: f32 = 0.06;
    /// 최소 드래그
    pub const DRAG_MIN: f32 = 0.02;
    /// 최대 드래그
    pub const DRAG_MAX: f32 = 0.08;
    /// stamina 가중치
    pub const DRAG_STAMINA_WEIGHT: f32 = 0.6;
    /// natural_fitness 가중치
    pub const DRAG_FITNESS_WEIGHT: f32 = 0.4;

    // ============================================================
    // 피로 영향 (fatigue → 물리 파라미터 감소)
    // ============================================================
    /// 피로 최대 시 최소 배율 (60%)
    pub const FATIGUE_MIN_MULT: f32 = 0.6;
    /// 피로 영향 범위 (0.6 + 0.4 * stamina01 = 0.6~1.0)
    pub const FATIGUE_RANGE: f32 = 0.4;

    // ============================================================
    // dt 안전 범위
    // ============================================================
    /// 최소 dt (너무 작으면 계산 낭비)
    pub const DT_MIN: f32 = 0.001;
    /// 최대 dt (너무 크면 물리 불안정)
    pub const DT_MAX: f32 = 0.1;
    /// 속도 임계값 (0에 가까운 속도 처리)
    pub const SPEED_EPSILON: f32 = 0.01;
    /// 도착 판정 거리 (m)
    pub const ARRIVAL_THRESHOLD: f32 = 0.3;

    // ============================================================
    // Arrival Steering (AI 브레이크)
    // 목표 근처에서 자동 감속하여 overshoot 방지
    // ============================================================
    /// 최소 감속 반경 (m)
    pub const ARRIVAL_SLOWING_MIN: f32 = 1.0;

    // ============================================================
    // PlayerState별 제한
    // ============================================================
    /// Recovering 상태: 속도 제한 배율
    pub const RECOVERING_SPEED_MULT: f32 = 0.5;
    /// Recovering 상태: 가속 제한 배율
    pub const RECOVERING_ACCEL_MULT: f32 = 0.3;

    // ============================================================
    // 정규화 헬퍼 (0~100 스탯 → 0.0~1.0)
    // ============================================================
    /// 0~100 스탯을 0.0~1.0으로 정규화
    #[inline]
    pub fn n100(v: u8) -> f32 {
        (v as f32 / 100.0).clamp(0.0, 1.0)
    }
}

// ============================================================
// FIX_2601/0112: Google Football Reference Constants
// Source: https://github.com/google-research/football
// Date: 2026-01-06
// ============================================================
pub mod google_football {
    //! Google Football 참조 상수
    //!
    //! 기존 상수와 비교/실험용으로 분리 관리.
    //! 일부 값은 우리 시스템과 차이가 있음 (주석 참조).

    // ========================================
    // Ball Physics (참조용 - 기존과 비교)
    // ========================================

    /// 바운스 에너지 보존율 (vs bounce::GRASS_COR 0.65)
    pub const BOUNCE: f32 = 0.62;
    /// 공기 저항 계수 (vs ball::DRAG_COEFFICIENT 0.25)
    /// Google Football은 더 낮은 저항 사용
    pub const DRAG: f32 = 0.015;
    /// 지면 마찰 계수 (vs ball::ROLLING_RESISTANCE 0.02)
    pub const FRICTION: f32 = 0.04;
    /// 잔디 영향 높이 (m)
    pub const GRASS_HEIGHT_M: f32 = 0.025;
    /// 공 반지름 (m)
    pub const BALL_RADIUS_M: f32 = 0.11;

    // ========================================
    // Ball Prediction System
    // ========================================

    /// 예측 윈도우 (ms) - 3초
    pub const PREDICTION_WINDOW_MS: u32 = 3000;
    /// 예측 스텝 (ms) - 10ms = 100fps
    pub const PREDICTION_STEP_MS: u32 = 10;
    /// 캐시된 예측 수
    pub const CACHED_PREDICTIONS: usize = 100;

    // ========================================
    // Time-to-Ball Calculation
    // ========================================

    /// 일반 도달 반경 (m) - vs ball::OWNERSHIP_THRESHOLD 5.0m
    /// Google Football은 훨씬 정밀한 반경 사용
    pub const RADIUS_USUAL_M: f32 = 0.28;
    /// 낙관적 도달 반경 (m) - 전력질주 시
    pub const RADIUS_OPTIMISTIC_M: f32 = 0.9;
    /// 발 앞 오프셋 (m)
    pub const FOOT_FRONT_OFFSET_M: f32 = 0.1;
    /// 현재 움직임 감쇠 시간 (ms)
    pub const MOVEMENT_DECAY_MS: u32 = 700;

    // ========================================
    // Formation - Microfocus System
    // ========================================

    /// Microfocus 피크 attraction (0.15 = 15% max pull)
    pub const MICROFOCUS_PEAK: f32 = 0.15;
    /// Microfocus 영향 범위 (normalized: 0.25 ≈ 26m)
    pub const MICROFOCUS_WIDTH: f32 = 0.25;

    // ========================================
    // Magnus Effect (Spin Physics)
    // ========================================

    /// Magnus 계수 (sin 함수 내 속도 스케일)
    pub const MAGNUS_COEFFICIENT: f32 = 0.94;
    /// Magnus 지수 (sin^power)
    pub const MAGNUS_POWER: f32 = 2.6;
    /// Magnus 배율
    pub const MAGNUS_MULTIPLIER: f32 = 30.0;
    /// 스핀 감쇠율 (per 10ms substep)
    /// 0.97 = 3% decay per step, ~85% after 50ms tick
    pub const SPIN_DECAY: f32 = 0.97;

    // ========================================
    // Velocity Thresholds
    // ========================================

    /// 정지 속도 (m/s)
    pub const IDLE_VELOCITY: f32 = 0.0;
    /// 드리블 속도 (m/s)
    pub const DRIBBLE_VELOCITY: f32 = 3.5;
    /// 걷기 속도 (m/s)
    pub const WALK_VELOCITY: f32 = 5.0;
    /// 전력질주 속도 (m/s)
    pub const SPRINT_VELOCITY: f32 = 8.0;

    // ========================================
    // Tactical Constants
    // ========================================

    /// 공격 시 depth factor
    pub const OFFENSE_DEPTH_FACTOR: f32 = 0.9;
    /// 공격 시 width factor
    pub const OFFENSE_WIDTH_FACTOR: f32 = 0.9;
    /// 수비 시 depth factor
    pub const DEFENSE_DEPTH_FACTOR: f32 = 0.75;
    /// 수비 시 width factor
    pub const DEFENSE_WIDTH_FACTOR: f32 = 0.8;
}
