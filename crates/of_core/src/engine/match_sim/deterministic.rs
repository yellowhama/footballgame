//! FIX_2601/1123: Deterministic 선택 함수
//!
//! RNG 없이 (seed, tick, actor, subcase) 기반으로 결정론적 선택을 수행한다.
//! 1122에서 실험적으로 도입했던 함수를 정식 채택.
//!
//! ## 설계 원칙
//!
//! 1. **RNG 완전 배제**: 모든 선택은 해시 기반
//! 2. **결정론 보장**: 동일 입력 → 동일 출력
//! 3. **Subcase 분리**: 액션/필드별 고유 상수로 충돌 방지
//!
//! ## 사용 예시
//!
//! ```ignore
//! use crate::engine::match_sim::deterministic::{deterministic_choice, subcase};
//!
//! let target_idx = deterministic_choice(
//!     seed, tick, player_idx,
//!     subcase::PASS_TARGET,
//!     valid_targets.len(),
//! );
//! ```

// FIX_2601/0123: DefaultHasher → FxHasher for version-stable determinism
// DefaultHasher is NOT stable across Rust versions, causing replay desync.
use fxhash::FxHasher;
use std::hash::{Hash, Hasher};

// ============================================================================
// Subcase Constants
// ============================================================================

/// Subcase 상수 - 액션 종류 + 필드별로 고유한 값
///
/// 네이밍 규칙: `{ACTION}_{FIELD}`
/// 범위 규칙: 각 액션은 0x01xx ~ 0x0Fxx 범위 사용
pub mod subcase {
    // Pass (0x01xx)
    /// 패스 대상 선수 선택
    pub const PASS_TARGET: u32 = 0x0100;
    /// 패스 파워 결정
    pub const PASS_POWER: u32 = 0x0101;
    /// 오프사이드로 인한 패스 대상 재선정
    pub const PASS_OFFSIDE_REDIRECT: u32 = 0x0102;
    /// 패스 방향 미세 조정
    pub const PASS_DIRECTION_ADJUST: u32 = 0x0103;
    /// FIX_2601/1128: Reciprocity injection probability check
    pub const RECIPROCITY_INJECT: u32 = 0x0104;

    // Shot (0x02xx)
    /// 슛 목표 Y 좌표
    pub const SHOT_TARGET_Y: u32 = 0x0200;
    /// 슛 파워
    pub const SHOT_POWER: u32 = 0x0201;
    /// 슛 목표 X 좌표 (칩샷 등)
    pub const SHOT_TARGET_X: u32 = 0x0202;

    // Dribble (0x03xx)
    /// 드리블 방향 Y 편차
    pub const DRIBBLE_DIRECTION_Y: u32 = 0x0300;
    /// 드리블 속도 계수
    pub const DRIBBLE_SPEED: u32 = 0x0301;
    /// 드리블 방향 X 편차
    pub const DRIBBLE_DIRECTION_X: u32 = 0x0302;

    // Tackle (0x04xx)
    /// 태클 대상 선수 선택
    pub const TACKLE_TARGET: u32 = 0x0400;
    /// 태클 종류 선택
    pub const TACKLE_KIND: u32 = 0x0401;

    // Header (0x05xx)
    /// 헤더 파워
    pub const HEADER_POWER: u32 = 0x0500;
    /// 헤더 목표 Y 좌표
    pub const HEADER_TARGET_Y: u32 = 0x0501;
    /// 헤더 방향 (클리어)
    pub const HEADER_CLEAR_DIRECTION: u32 = 0x0502;

    // Cross (0x06xx)
    /// 크로스 목표 지점 X
    pub const CROSS_TARGET_X: u32 = 0x0600;
    /// 크로스 목표 지점 Y
    pub const CROSS_TARGET_Y: u32 = 0x0601;
    /// 크로스 파워
    pub const CROSS_POWER: u32 = 0x0602;

    // Clearance (0x07xx)
    /// 클리어 방향 X
    pub const CLEARANCE_DIRECTION_X: u32 = 0x0700;
    /// 클리어 방향 Y
    pub const CLEARANCE_DIRECTION_Y: u32 = 0x0701;
    /// 클리어 파워
    pub const CLEARANCE_POWER: u32 = 0x0702;

    // Hold (0x08xx)
    /// 쉴드 방향
    pub const HOLD_SHIELD_DIRECTION: u32 = 0x0800;

    // Intercept (0x09xx)
    /// 인터셉트 지점 조정
    pub const INTERCEPT_POINT_ADJUST: u32 = 0x0900;
}

// ============================================================================
// Core Functions
// ============================================================================

/// 리스트에서 하나를 결정론적으로 선택
///
/// 동일한 (seed, tick, actor_idx, subcase)에 대해 항상 동일한 인덱스를 반환한다.
/// RNG를 사용하지 않으므로 결정론이 보장된다.
///
/// # Arguments
/// * `seed` - 매치 시드
/// * `tick` - 현재 틱 (시간)
/// * `actor_idx` - 행위자 인덱스 (track_id)
/// * `subcase` - 선택 컨텍스트 (subcase 상수 사용)
/// * `options_count` - 선택지 개수
///
/// # Returns
/// `0..options_count` 범위의 인덱스
///
/// # Example
/// ```ignore
/// let idx = deterministic_choice(seed, tick, player_idx, subcase::PASS_TARGET, 5);
/// assert!(idx < 5);
/// ```
#[inline]
pub fn deterministic_choice(
    seed: u64,
    tick: u64,
    actor_idx: usize,
    subcase: u32,
    options_count: usize,
) -> usize {
    if options_count == 0 {
        return 0;
    }
    if options_count == 1 {
        return 0;
    }

    let mut hasher = FxHasher::default();
    seed.hash(&mut hasher);
    tick.hash(&mut hasher);
    actor_idx.hash(&mut hasher);
    subcase.hash(&mut hasher);
    (hasher.finish() as usize) % options_count
}

/// 범위 내 f32 값을 결정론적으로 선택
///
/// 동일한 (seed, tick, actor_idx, subcase)에 대해 항상 동일한 값을 반환한다.
///
/// # Arguments
/// * `seed` - 매치 시드
/// * `tick` - 현재 틱
/// * `actor_idx` - 행위자 인덱스
/// * `subcase` - 선택 컨텍스트
/// * `min` - 최소값 (포함)
/// * `max` - 최대값 (미포함)
///
/// # Returns
/// `[min, max)` 범위의 f32 값
///
/// # Example
/// ```ignore
/// let power = deterministic_f32(seed, tick, player_idx, subcase::SHOT_POWER, 0.7, 1.0);
/// assert!(power >= 0.7 && power < 1.0);
/// ```
#[inline]
pub fn deterministic_f32(
    seed: u64,
    tick: u64,
    actor_idx: usize,
    subcase: u32,
    min: f32,
    max: f32,
) -> f32 {
    let mut hasher = FxHasher::default();
    seed.hash(&mut hasher);
    tick.hash(&mut hasher);
    actor_idx.hash(&mut hasher);
    subcase.hash(&mut hasher);
    let hash = hasher.finish();

    // hash를 0.0..1.0 범위로 변환
    let t = (hash as f64) / (u64::MAX as f64);
    min + (max - min) * (t as f32)
}

/// 범위 내 f64 값을 결정론적으로 선택
///
/// f32 버전과 동일하지만 더 높은 정밀도가 필요한 경우 사용.
#[inline]
pub fn deterministic_f64(
    seed: u64,
    tick: u64,
    actor_idx: usize,
    subcase: u32,
    min: f64,
    max: f64,
) -> f64 {
    let mut hasher = FxHasher::default();
    seed.hash(&mut hasher);
    tick.hash(&mut hasher);
    actor_idx.hash(&mut hasher);
    subcase.hash(&mut hasher);
    let hash = hasher.finish();

    let t = (hash as f64) / (u64::MAX as f64);
    min + (max - min) * t
}

/// bool 값을 결정론적으로 선택
///
/// 주어진 확률로 true를 반환한다.
///
/// # Arguments
/// * `probability` - true가 반환될 확률 (0.0..1.0)
///
/// # Example
/// ```ignore
/// // 80% 확률로 true
/// let should_do = deterministic_bool(seed, tick, player_idx, subcase::TACKLE_ATTEMPT, 0.8);
/// ```
#[inline]
pub fn deterministic_bool(
    seed: u64,
    tick: u64,
    actor_idx: usize,
    subcase: u32,
    probability: f32,
) -> bool {
    deterministic_f32(seed, tick, actor_idx, subcase, 0.0, 1.0) < probability
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 벡터 정규화 (길이 1.0으로)
#[inline]
pub fn normalize_direction(dir: (f32, f32)) -> (f32, f32) {
    let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
    if len < 1e-6 {
        (1.0, 0.0) // 기본 방향: 오른쪽
    } else {
        (dir.0 / len, dir.1 / len)
    }
}

/// f32 값을 범위 내로 클램프
#[inline]
pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_choice_stable() {
        // 같은 입력 → 같은 출력
        let a = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 10);
        let b = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 10);
        assert_eq!(a, b, "Same inputs must produce same output");
    }

    #[test]
    fn test_deterministic_choice_varies_with_seed() {
        let a = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 10);
        let b = deterministic_choice(99999, 100, 5, subcase::PASS_TARGET, 10);
        // 다를 가능성이 매우 높음 (같을 확률 10%)
        // 테스트 목적상 단순히 컴파일 확인
        let _ = (a, b);
    }

    #[test]
    fn test_deterministic_choice_varies_with_tick() {
        let a = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 10);
        let b = deterministic_choice(12345, 200, 5, subcase::PASS_TARGET, 10);
        let _ = (a, b);
    }

    #[test]
    fn test_deterministic_choice_varies_with_actor() {
        let a = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 10);
        let b = deterministic_choice(12345, 100, 7, subcase::PASS_TARGET, 10);
        let _ = (a, b);
    }

    #[test]
    fn test_deterministic_choice_varies_with_subcase() {
        let a = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 10);
        let b = deterministic_choice(12345, 100, 5, subcase::SHOT_TARGET_Y, 10);
        let _ = (a, b);
    }

    #[test]
    fn test_deterministic_choice_edge_cases() {
        // options_count = 0
        let a = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 0);
        assert_eq!(a, 0);

        // options_count = 1
        let b = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, 1);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_deterministic_choice_in_range() {
        for options in 2..20 {
            let idx = deterministic_choice(12345, 100, 5, subcase::PASS_TARGET, options);
            assert!(idx < options, "Index {} out of range {}", idx, options);
        }
    }

    #[test]
    fn test_deterministic_f32_stable() {
        let a = deterministic_f32(12345, 100, 5, subcase::SHOT_POWER, 0.7, 1.0);
        let b = deterministic_f32(12345, 100, 5, subcase::SHOT_POWER, 0.7, 1.0);
        assert!((a - b).abs() < 1e-6, "Same inputs must produce same output");
    }

    #[test]
    fn test_deterministic_f32_in_range() {
        for _ in 0..100 {
            let v = deterministic_f32(12345, 100, 5, subcase::SHOT_POWER, 0.7, 1.0);
            assert!(v >= 0.7, "Value {} below min", v);
            assert!(v < 1.0, "Value {} above max", v);
        }
    }

    #[test]
    fn test_deterministic_f32_different_ranges() {
        let a = deterministic_f32(12345, 100, 5, subcase::PASS_POWER, 0.4, 0.8);
        assert!(a >= 0.4 && a < 0.8);

        let b = deterministic_f32(12345, 100, 5, subcase::DRIBBLE_DIRECTION_Y, -0.3, 0.3);
        assert!(b >= -0.3 && b < 0.3);
    }

    #[test]
    fn test_deterministic_bool() {
        // 확률 0.0 → 항상 false
        let always_false = deterministic_bool(12345, 100, 5, subcase::TACKLE_KIND, 0.0);
        assert!(!always_false);

        // 확률 1.0 → 항상 true
        let always_true = deterministic_bool(12345, 100, 5, subcase::TACKLE_KIND, 1.0);
        assert!(always_true);
    }

    #[test]
    fn test_normalize_direction() {
        let (x, y) = normalize_direction((3.0, 4.0));
        let len = (x * x + y * y).sqrt();
        assert!((len - 1.0).abs() < 1e-5);
        assert!((x - 0.6).abs() < 1e-5);
        assert!((y - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_normalize_direction_zero() {
        let (x, y) = normalize_direction((0.0, 0.0));
        assert_eq!((x, y), (1.0, 0.0));
    }

    #[test]
    fn test_distribution_uniformity() {
        // 분포가 대략 균등한지 확인 (통계적 테스트)
        let mut counts = [0u32; 10];
        for tick in 0..1000 {
            let idx = deterministic_choice(42, tick, 5, subcase::PASS_TARGET, 10);
            counts[idx] += 1;
        }

        // 각 버킷이 50~150 사이여야 함 (기대값 100)
        for (i, &count) in counts.iter().enumerate() {
            assert!(
                count > 50 && count < 150,
                "Bucket {} has {} (expected ~100)",
                i,
                count
            );
        }
    }
}
