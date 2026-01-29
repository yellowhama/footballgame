//! RewardFunction - AI Training Reward Framework
//!
//! Google Football 스타일의 보상 함수 프레임워크.
//!
//! ## 구성 요소
//!
//! - `RewardFunction` trait: 모든 보상 함수의 공통 인터페이스
//! - `SparseGoalReward`: 골 득실 기반 보상 (+1/-1)
//! - `CheckpointReward`: 공 진행 거리 기반 보상 (Dense reward)
//! - `CompositeReward`: 여러 보상 함수 조합
//! - `EpisodeMetrics`: RL 훈련용 에피소드 메트릭 (Phase 3)
//!
//! ## 사용 예시
//!
//! ```rust,ignore
//! use of_core::engine::reward::{CompositeReward, SparseGoalReward, CheckpointReward, RewardFunction};
//!
//! // 골 보상 + 체크포인트 보상 조합
//! let mut reward = CompositeReward::new()
//!     .add(Box::new(SparseGoalReward::new(true)), 1.0)
//!     .add(Box::new(CheckpointReward::new(true)), 0.1);
//!
//! // 매 틱마다 보상 계산
//! let r = reward.compute(&prev_snapshot, &curr_snapshot, &events);
//!
//! // 에피소드 리셋
//! reward.reset();
//! ```

mod checkpoint;
mod composite;
mod episode;
mod sparse;

pub use checkpoint::CheckpointReward;
pub use composite::CompositeReward;
pub use episode::{ActionStats, EpisodeMetrics, TerminationReason};
pub use sparse::SparseGoalReward;

use crate::models::MatchEvent;

use super::tick_snapshot::TickSnapshot;

// ============================================================================
// RewardFunction Trait
// ============================================================================

/// 보상 함수 트레이트
///
/// AI 훈련용 보상 계산 인터페이스.
/// 모든 구현체는 스레드 안전(`Send + Sync`)해야 함.
pub trait RewardFunction: Send + Sync {
    /// 현재 상태에서 보상 계산
    ///
    /// # Arguments
    /// * `prev` - 이전 틱 스냅샷
    /// * `curr` - 현재 틱 스냅샷
    /// * `events` - 현재 틱에서 발생한 이벤트
    ///
    /// # Returns
    /// 보상 값 (양수/음수 모두 가능)
    fn compute(&mut self, prev: &TickSnapshot, curr: &TickSnapshot, events: &[MatchEvent]) -> f32;

    /// 이 함수가 sparse인지 여부
    ///
    /// Sparse reward: 에피소드당 1회만 비-0 반환 (예: 골 득실)
    /// Dense reward: 매 틱마다 값 반환 가능 (예: 체크포인트)
    fn is_sparse(&self) -> bool {
        false
    }

    /// 함수 이름 (디버깅/로깅용)
    fn name(&self) -> &str;

    /// 상태 리셋 (에피소드 시작 시 호출)
    ///
    /// 체크포인트 등 상태를 가진 보상 함수에서 사용.
    fn reset(&mut self) {}
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 공 위치에서 목표 골대까지의 거리 (정규화)
///
/// # Arguments
/// * `ball_x_m` - 공 x좌표 (미터)
/// * `home_perspective` - 홈팀 시점 여부 (true면 오른쪽 골대가 목표)
///
/// # Returns
/// 0.0 (목표 골대) ~ 1.0 (자기 골대) 범위의 거리
pub fn normalized_distance_to_goal(ball_x_m: f32, home_perspective: bool) -> f32 {
    const FIELD_LENGTH: f32 = 105.0;

    let target_x = if home_perspective { FIELD_LENGTH } else { 0.0 };
    (ball_x_m - target_x).abs() / FIELD_LENGTH
}

/// 공 소유 팀 확인
///
/// # Arguments
/// * `owner` - 공 소유자 track_id (0-21, None이면 루즈볼)
/// * `home_perspective` - 홈팀 시점 여부
///
/// # Returns
/// 우리 팀이 공을 소유하고 있는지 여부
pub fn is_our_ball(owner: Option<u8>, home_perspective: bool) -> bool {
    match owner {
        Some(id) => (id < 11) == home_perspective,
        None => false,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_distance() {
        // 홈팀 시점: 오른쪽 골대가 목표
        // 공이 목표 골대 앞에 있을 때
        let dist = normalized_distance_to_goal(100.0, true);
        assert!((dist - 0.0476).abs() < 0.01); // ~5m from goal

        // 공이 중앙에 있을 때
        let dist = normalized_distance_to_goal(52.5, true);
        assert!((dist - 0.5).abs() < 0.01);

        // 공이 자기 골대 앞에 있을 때
        let dist = normalized_distance_to_goal(5.0, true);
        assert!((dist - 0.952).abs() < 0.01);
    }

    #[test]
    fn test_is_our_ball() {
        // 홈팀 시점
        assert!(is_our_ball(Some(5), true)); // 홈팀 선수 (0-10)
        assert!(!is_our_ball(Some(15), true)); // 어웨이 선수 (11-21)
        assert!(!is_our_ball(None, true)); // 루즈볼

        // 어웨이팀 시점
        assert!(!is_our_ball(Some(5), false)); // 홈팀 선수
        assert!(is_our_ball(Some(15), false)); // 어웨이 선수
    }
}
