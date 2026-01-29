//! action_common.rs
//!
//! Common ActionModel trait + helper math/selection utilities.
//!
//! ## Intent → Technique → Physics 패턴
//! 모든 액션(Pass/Shot/Dribble)을 통일된 인터페이스로 처리:
//! - Intent: "왜 이 행동을 하냐"
//! - Technique: "어떻게 실행하냐"
//! - PhysicsParams: 실행 파라미터 (속도/거리/리스크/지속 등)

use rand::Rng;

/// 0~1 범위로 클램프
#[inline]
pub fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// 선형 보간
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 0~20 스킬값을 0~1로 정규화
#[inline]
pub fn skill01(v_0_20: u8) -> f32 {
    (v_0_20 as f32 / 20.0).clamp(0.0, 1.0)
}

/// 가중치 기반 랜덤 선택. weights 배열의 인덱스 반환.
#[inline]
pub fn weighted_choice_index<R: Rng + ?Sized>(weights: &[f32], rng: &mut R) -> usize {
    let mut total = 0.0;
    for &w in weights {
        total += w.max(0.0);
    }
    if total <= 0.0 {
        return 0;
    }

    let mut r = rng.gen::<f32>() * total;
    for (i, &w) in weights.iter().enumerate() {
        r -= w.max(0.0);
        if r <= 0.0 {
            return i;
        }
    }
    weights.len().saturating_sub(1)
}

/// 통합 액션 모델 인터페이스
///
/// Pass/Shot/Dribble을 동일한 패턴으로 처리:
/// - Intent: "왜 이 행동을 하냐"
/// - Technique: "어떻게 실행하냐"
/// - PhysicsParams: 실행 파라미터 (속도/거리/리스크/지속 등)
/// - base_success_prob: "이 기술이 정상 수행될 확률"
/// - execution_error: 실행 오차(방향/속도/높이) 표준편차
pub trait ActionModel {
    type Intent: Copy + Eq;
    type Technique: Copy + Eq;
    type PhysicsParams: Copy;
    type Context: Copy;
    type Skills: Copy;

    /// Intent → 후보 Technique 목록
    fn available_techniques(intent: Self::Intent) -> &'static [Self::Technique];

    /// Technique → Physics params 테이블
    fn physics_params(technique: Self::Technique) -> Self::PhysicsParams;

    /// Intent + Context + Skills + Pressure → 최종 Technique 선택
    fn choose_technique<R: Rng + ?Sized>(
        intent: Self::Intent,
        context: Self::Context,
        skills: Self::Skills,
        pressure: f32,
        rng: &mut R,
    ) -> Self::Technique;

    /// Technique + Skills + Pressure → 성공 베이스 확률
    fn base_success_prob(technique: Self::Technique, skills: Self::Skills, pressure: f32) -> f32;

    /// Technique + Skills + Pressure → 실행 오차(표준편차)
    /// (dir_sigma_rad, speed_sigma_ratio, height_sigma_ratio)
    fn execution_error(
        technique: Self::Technique,
        skills: Self::Skills,
        pressure: f32,
    ) -> (f32, f32, f32);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp01() {
        assert_eq!(clamp01(0.5), 0.5);
        assert_eq!(clamp01(-0.5), 0.0);
        assert_eq!(clamp01(1.5), 1.0);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
    }

    #[test]
    fn test_skill01() {
        assert_eq!(skill01(0), 0.0);
        assert_eq!(skill01(20), 1.0);
        assert_eq!(skill01(10), 0.5);
        assert_eq!(skill01(100), 1.0); // 클램프됨
    }

    #[test]
    fn test_weighted_choice_index() {
        struct DummyRng(f32);
        impl rand::RngCore for DummyRng {
            fn next_u32(&mut self) -> u32 {
                (self.0 * u32::MAX as f32) as u32
            }
            fn next_u64(&mut self) -> u64 {
                (self.0 * u64::MAX as f32) as u64
            }
            fn fill_bytes(&mut self, _: &mut [u8]) {}
            fn try_fill_bytes(&mut self, _: &mut [u8]) -> Result<(), rand::Error> {
                Ok(())
            }
        }

        // 가중치 [1.0, 2.0, 1.0] → 총 4.0
        // 0.0~0.25 → 0, 0.25~0.75 → 1, 0.75~1.0 → 2
        let weights = [1.0, 2.0, 1.0];

        let mut rng = DummyRng(0.1); // 0.1 * 4.0 = 0.4 < 1.0 → 0
        assert_eq!(weighted_choice_index(&weights, &mut rng), 0);

        let mut rng = DummyRng(0.4); // 0.4 * 4.0 = 1.6 → 0 + 1.0 = 1.0, 1.6 - 1.0 = 0.6 < 2.0 → 1
        assert_eq!(weighted_choice_index(&weights, &mut rng), 1);

        let mut rng = DummyRng(0.9); // 0.9 * 4.0 = 3.6 → 1.0 + 2.0 = 3.0, 3.6 - 3.0 = 0.6 < 1.0 → 2
        assert_eq!(weighted_choice_index(&weights, &mut rng), 2);
    }

    #[test]
    fn test_weighted_choice_empty_weights() {
        struct DummyRng;
        impl rand::RngCore for DummyRng {
            fn next_u32(&mut self) -> u32 {
                0
            }
            fn next_u64(&mut self) -> u64 {
                0
            }
            fn fill_bytes(&mut self, _: &mut [u8]) {}
            fn try_fill_bytes(&mut self, _: &mut [u8]) -> Result<(), rand::Error> {
                Ok(())
            }
        }

        let weights: [f32; 0] = [];
        assert_eq!(weighted_choice_index(&weights, &mut DummyRng), 0);

        let zero_weights = [0.0, 0.0];
        assert_eq!(weighted_choice_index(&zero_weights, &mut DummyRng), 0);
    }
}
