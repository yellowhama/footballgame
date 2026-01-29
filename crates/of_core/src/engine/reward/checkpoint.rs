//! CheckpointReward - Distance-based Dense Reward
//!
//! 공을 상대 골대 방향으로 진행시킬 때 보상을 주는 Dense reward.
//!
//! ## 메커니즘
//!
//! 필드를 10개 체크포인트로 분할:
//! - 체크포인트 0: 자기 골대 근처 (가장 뒤)
//! - 체크포인트 9: 상대 골대 근처 (가장 앞)
//!
//! 공을 소유하고 있을 때 새로운 체크포인트를 통과하면 보상.
//! 같은 체크포인트는 에피소드당 1회만 보상.
//!
//! ## Google Football 참조
//!
//! ```python
//! # gfootball/env/wrappers.py - CheckpointRewardWrapper
//! d = ((o['ball'][0] - 1) ** 2 + o['ball'][1] ** 2) ** 0.5
//! # threshold 통과 시 +0.1 누적
//! ```

use super::{is_our_ball, normalized_distance_to_goal, RewardFunction};
use crate::engine::tick_snapshot::TickSnapshot;
use crate::models::MatchEvent;

/// 체크포인트 기반 Dense 보상 함수
///
/// 공을 상대 골대 방향으로 진행시킬 때 보상.
#[derive(Debug, Clone)]
pub struct CheckpointReward {
    /// 홈팀 시점 여부
    home_perspective: bool,

    /// 체크포인트 개수 (기본 10)
    num_checkpoints: usize,

    /// 체크포인트당 보상 (기본 0.1)
    checkpoint_reward: f32,

    /// 각 체크포인트 통과 여부
    collected: Vec<bool>,
}

impl CheckpointReward {
    /// 새 보상 함수 생성
    ///
    /// # Arguments
    /// * `home_perspective` - 홈팀 시점 여부
    pub fn new(home_perspective: bool) -> Self {
        Self {
            home_perspective,
            num_checkpoints: 10,
            checkpoint_reward: 0.1,
            collected: vec![false; 10],
        }
    }

    /// 체크포인트 개수 설정
    pub fn with_num_checkpoints(mut self, num: usize) -> Self {
        self.num_checkpoints = num.max(1);
        self.collected = vec![false; self.num_checkpoints];
        self
    }

    /// 체크포인트당 보상 설정
    pub fn with_checkpoint_reward(mut self, reward: f32) -> Self {
        self.checkpoint_reward = reward;
        self
    }

    /// 홈팀 시점 보상 함수 생성 (편의 함수)
    pub fn home() -> Self {
        Self::new(true)
    }

    /// 어웨이팀 시점 보상 함수 생성 (편의 함수)
    pub fn away() -> Self {
        Self::new(false)
    }

    /// 공 위치에서 체크포인트 인덱스 계산
    fn position_to_checkpoint(&self, ball_x_m: f32) -> usize {
        // 목표 골대까지의 정규화 거리 (0.0 = 목표 골대, 1.0 = 자기 골대)
        let dist = normalized_distance_to_goal(ball_x_m, self.home_perspective);

        // 거리가 가까울수록 높은 체크포인트
        // progress = 1.0 - dist (0.0 = 자기 골대, 1.0 = 목표 골대)
        let progress = 1.0 - dist;

        // 체크포인트 인덱스 (0 ~ num_checkpoints-1)
        let idx = (progress * self.num_checkpoints as f32).floor() as usize;
        idx.min(self.num_checkpoints - 1)
    }
}

impl RewardFunction for CheckpointReward {
    fn compute(&mut self, _prev: &TickSnapshot, curr: &TickSnapshot, _events: &[MatchEvent]) -> f32 {
        // 공 소유 확인
        if !is_our_ball(curr.ball.owner, self.home_perspective) {
            return 0.0;
        }

        // 공 위치에서 체크포인트 계산
        let (ball_x_m, _ball_y_m) = curr.ball.pos.to_meters();
        let checkpoint_idx = self.position_to_checkpoint(ball_x_m);

        // 새 체크포인트 통과 시 보상
        if !self.collected[checkpoint_idx] {
            self.collected[checkpoint_idx] = true;
            self.checkpoint_reward
        } else {
            0.0
        }
    }

    fn is_sparse(&self) -> bool {
        false // Dense reward
    }

    fn name(&self) -> &str {
        "checkpoint"
    }

    fn reset(&mut self) {
        self.collected.fill(false);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tick_snapshot::{BallSnap, BallStateTag};
    use crate::engine::types::Coord10;

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
                pos: Coord10::from_meters(ball_x_m, 34.0), // 중앙선
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

    #[test]
    fn test_checkpoint_progress_home() {
        let mut reward = CheckpointReward::home();

        // 홈팀 선수(track_id=5)가 공 소유
        // 중앙에서 시작
        let snap1 = make_snapshot_with_ball(52.5, Some(5));
        let r1 = reward.compute(&snap1, &snap1, &[]);
        assert!(r1 > 0.0); // 체크포인트 ~5 통과

        // 더 앞으로 진행
        let snap2 = make_snapshot_with_ball(80.0, Some(5));
        let r2 = reward.compute(&snap1, &snap2, &[]);
        assert!(r2 > 0.0); // 새 체크포인트 통과

        // 같은 위치 - 보상 없음
        let r3 = reward.compute(&snap2, &snap2, &[]);
        assert!((r3 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_no_reward_without_possession() {
        let mut reward = CheckpointReward::home();

        // 어웨이팀 선수(track_id=15)가 공 소유
        let snap = make_snapshot_with_ball(80.0, Some(15));
        let r = reward.compute(&snap, &snap, &[]);
        assert!((r - 0.0).abs() < 0.001);

        // 루즈볼
        let snap_loose = make_snapshot_with_ball(80.0, None);
        let r_loose = reward.compute(&snap_loose, &snap_loose, &[]);
        assert!((r_loose - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_away_perspective() {
        let mut reward = CheckpointReward::away();

        // 어웨이팀 선수(track_id=15)가 공 소유
        // 어웨이팀 시점에서는 x=0 방향이 목표
        let snap = make_snapshot_with_ball(30.0, Some(15));
        let r = reward.compute(&snap, &snap, &[]);
        assert!(r > 0.0);
    }

    #[test]
    fn test_reset() {
        let mut reward = CheckpointReward::home();

        // 체크포인트 통과
        let snap = make_snapshot_with_ball(80.0, Some(5));
        let r1 = reward.compute(&snap, &snap, &[]);
        assert!(r1 > 0.0);

        // 같은 위치 - 보상 없음
        let r2 = reward.compute(&snap, &snap, &[]);
        assert!((r2 - 0.0).abs() < 0.001);

        // 리셋 후 다시 보상
        reward.reset();
        let r3 = reward.compute(&snap, &snap, &[]);
        assert!(r3 > 0.0);
    }

    #[test]
    fn test_full_field_progress() {
        let mut reward = CheckpointReward::home().with_checkpoint_reward(0.1);

        let mut total_reward = 0.0;
        let home_owner = Some(5u8);

        // 자기 골대에서 상대 골대까지 진행
        for x in (10..=100).step_by(10) {
            let snap = make_snapshot_with_ball(x as f32, home_owner);
            total_reward += reward.compute(&snap, &snap, &[]);
        }

        // 대부분의 체크포인트 통과했으므로 0.9 이상
        assert!(total_reward >= 0.9);
    }

    #[test]
    fn test_custom_checkpoints() {
        let mut reward = CheckpointReward::home()
            .with_num_checkpoints(5)
            .with_checkpoint_reward(0.2);

        let snap = make_snapshot_with_ball(90.0, Some(5));
        let r = reward.compute(&snap, &snap, &[]);
        assert!((r - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_is_dense() {
        let reward = CheckpointReward::home();
        assert!(!reward.is_sparse());
    }

    #[test]
    fn test_name() {
        let reward = CheckpointReward::home();
        assert_eq!(reward.name(), "checkpoint");
    }
}
