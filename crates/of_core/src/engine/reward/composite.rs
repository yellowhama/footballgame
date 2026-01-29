//! CompositeReward - Combined Reward Functions
//!
//! 여러 보상 함수를 가중 합산하는 조합 보상 함수.
//!
//! ## 사용 예시
//!
//! ```rust,ignore
//! let reward = CompositeReward::new()
//!     .add(Box::new(SparseGoalReward::home()), 1.0)   // 골 보상: 가중치 1.0
//!     .add(Box::new(CheckpointReward::home()), 0.1); // 체크포인트: 가중치 0.1
//! ```
//!
//! ## Google Football 참조
//!
//! Google Football에서는 래퍼 패턴으로 보상 조합:
//! ```python
//! env = CheckpointRewardWrapper(env)  # Dense reward 추가
//! # 기본 환경의 sparse reward + checkpoint dense reward
//! ```

use super::RewardFunction;
use crate::engine::tick_snapshot::TickSnapshot;
use crate::models::MatchEvent;

/// 조합 보상 함수
///
/// 여러 보상 함수를 가중치와 함께 조합.
#[derive(Default)]
pub struct CompositeReward {
    /// (보상 함수, 가중치) 목록
    functions: Vec<(Box<dyn RewardFunction>, f32)>,
}

impl CompositeReward {
    /// 새 조합 보상 함수 생성
    pub fn new() -> Self {
        Self { functions: Vec::new() }
    }

    /// 보상 함수 추가
    ///
    /// # Arguments
    /// * `func` - 추가할 보상 함수
    /// * `weight` - 가중치 (1.0 = 원래 값 그대로)
    pub fn add(mut self, func: Box<dyn RewardFunction>, weight: f32) -> Self {
        self.functions.push((func, weight));
        self
    }

    /// 보상 함수 추가 (Builder 패턴)
    ///
    /// `add()`와 동일하지만 &mut Self 반환.
    pub fn add_mut(&mut self, func: Box<dyn RewardFunction>, weight: f32) -> &mut Self {
        self.functions.push((func, weight));
        self
    }

    /// 포함된 보상 함수 개수
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// 보상 함수가 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

impl RewardFunction for CompositeReward {
    fn compute(&mut self, prev: &TickSnapshot, curr: &TickSnapshot, events: &[MatchEvent]) -> f32 {
        self.functions.iter_mut().map(|(func, weight)| func.compute(prev, curr, events) * *weight).sum()
    }

    fn is_sparse(&self) -> bool {
        // 모든 함수가 sparse여야 composite도 sparse
        !self.functions.is_empty() && self.functions.iter().all(|(func, _)| func.is_sparse())
    }

    fn name(&self) -> &str {
        "composite"
    }

    fn reset(&mut self) {
        for (func, _) in &mut self.functions {
            func.reset();
        }
    }
}

impl std::fmt::Debug for CompositeReward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeReward")
            .field(
                "functions",
                &self
                    .functions
                    .iter()
                    .map(|(func, weight)| (func.name(), *weight))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::reward::{CheckpointReward, SparseGoalReward};
    use crate::engine::tick_snapshot::{BallSnap, BallStateTag};
    use crate::engine::types::Coord10;
    use crate::models::EventType;

    fn make_snapshot_with_ball(ball_x_m: f32, owner: Option<u8>) -> TickSnapshot {
        use crate::engine::tick_snapshot::{
            GameModeTag, OffBallObjectiveSnap, PlayerSnap, PlayerStateTag, StickyActionsSnap,
            TeamSnap,
        };

        TickSnapshot {
            tick: 0,
            minute: 45,
            seed: 42,
            ball: BallSnap {
                state: BallStateTag::Controlled,
                pos: Coord10::from_meters(ball_x_m, 34.0),
                owner,
                target_pos: None,
                eta_tick: None,
                intended_receiver: None,
                is_shot: false,
            },
            players: std::array::from_fn(|i| PlayerSnap {
                id: i as u8,
                is_home: i < 11,
                pos: Coord10::from_meters(52.5, 34.0),
                state: PlayerStateTag::Idle,
                stamina: 1.0,
                dist_to_ball: 100,
            }),
            teams: TeamSnap { home_attacks_right: true, home_has_possession: true },
            tackle_cooldowns: [0; 22],
            offball_objectives: std::array::from_fn(|_| OffBallObjectiveSnap::default()),
            last_pass_target: None,
            home_attacks_right: true,
            // Phase 4: Observation fields
            player_velocities: [(0.0, 0.0); 22],
            score: (0, 0),
            game_mode: GameModeTag::Normal,
            sticky_actions: [StickyActionsSnap::default(); 22],
        }
    }

    fn make_dummy_snapshot() -> TickSnapshot {
        make_snapshot_with_ball(52.5, None)
    }

    fn make_goal_event(is_home_team: bool) -> MatchEvent {
        MatchEvent {
            minute: 45,
            timestamp_ms: Some(1000),
            event_type: EventType::Goal,
            is_home_team,
            player_track_id: Some(10),
            target_track_id: None,
            details: None,
        }
    }

    #[test]
    fn test_weighted_sum() {
        let mut reward = CompositeReward::new()
            .add(Box::new(SparseGoalReward::home()), 10.0) // 골: 가중치 10
            .add(Box::new(CheckpointReward::home()), 1.0); // 체크포인트: 가중치 1

        // 골 + 체크포인트 동시 발생
        let snap = make_snapshot_with_ball(80.0, Some(5));
        let events = vec![make_goal_event(true)];

        let r = reward.compute(&snap, &snap, &events);

        // 골 보상 (10.0 * 1.0 = 10.0) + 체크포인트 보상 (1.0 * 0.1 = 0.1)
        assert!(r > 10.0);
        assert!(r < 11.0);
    }

    #[test]
    fn test_only_sparse() {
        let mut reward = CompositeReward::new().add(Box::new(SparseGoalReward::home()), 1.0);

        let snap = make_dummy_snapshot();
        let events = vec![make_goal_event(true)];

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_only_dense() {
        let mut reward = CompositeReward::new().add(Box::new(CheckpointReward::home()), 1.0);

        let snap = make_snapshot_with_ball(80.0, Some(5));

        let r = reward.compute(&snap, &snap, &[]);
        assert!((r - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_reset_all() {
        let mut reward = CompositeReward::new()
            .add(Box::new(CheckpointReward::home()), 1.0)
            .add(Box::new(CheckpointReward::home()), 1.0);

        let snap = make_snapshot_with_ball(80.0, Some(5));

        // 첫 번째 호출
        let r1 = reward.compute(&snap, &snap, &[]);
        assert!(r1 > 0.0);

        // 두 번째 호출 - 같은 체크포인트, 보상 없음
        let r2 = reward.compute(&snap, &snap, &[]);
        assert!((r2 - 0.0).abs() < 0.001);

        // 리셋 후 다시 보상
        reward.reset();
        let r3 = reward.compute(&snap, &snap, &[]);
        assert!(r3 > 0.0);
    }

    #[test]
    fn test_is_sparse_mixed() {
        // Sparse + Dense 조합 → Dense
        let reward = CompositeReward::new()
            .add(Box::new(SparseGoalReward::home()), 1.0)
            .add(Box::new(CheckpointReward::home()), 1.0);

        assert!(!reward.is_sparse());
    }

    #[test]
    fn test_is_sparse_only_sparse() {
        // Sparse만 → Sparse
        let reward = CompositeReward::new()
            .add(Box::new(SparseGoalReward::home()), 1.0)
            .add(Box::new(SparseGoalReward::away()), 1.0);

        assert!(reward.is_sparse());
    }

    #[test]
    fn test_empty() {
        let mut reward = CompositeReward::new();
        let snap = make_dummy_snapshot();

        assert!(reward.is_empty());
        assert_eq!(reward.len(), 0);

        let r = reward.compute(&snap, &snap, &[]);
        assert!((r - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_negative_weight() {
        // 음수 가중치 테스트 (페널티)
        let mut reward = CompositeReward::new().add(Box::new(SparseGoalReward::home()), -1.0);

        let snap = make_dummy_snapshot();
        let events = vec![make_goal_event(true)];

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_name() {
        let reward = CompositeReward::new();
        assert_eq!(reward.name(), "composite");
    }

    #[test]
    fn test_debug() {
        let reward = CompositeReward::new()
            .add(Box::new(SparseGoalReward::home()), 1.0)
            .add(Box::new(CheckpointReward::home()), 0.1);

        let debug_str = format!("{:?}", reward);
        assert!(debug_str.contains("sparse_goal"));
        assert!(debug_str.contains("checkpoint"));
    }
}
