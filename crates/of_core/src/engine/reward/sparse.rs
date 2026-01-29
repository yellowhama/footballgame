//! SparseGoalReward - Goal-based Sparse Reward
//!
//! 골 득실에만 보상을 주는 가장 기본적인 보상 함수.
//!
//! - 골 득점: +1.0
//! - 골 실점: -1.0
//! - 자책골: 반대 부호
//!
//! ## 특징
//!
//! - **Sparse**: 골이 발생할 때만 비-0 보상
//! - **대칭성**: 홈/어웨이 양쪽 시점 지원
//! - **상태 없음**: 리셋 불필요

use super::RewardFunction;
use crate::models::{EventType, MatchEvent};
use crate::engine::tick_snapshot::TickSnapshot;

/// 골 기반 희소 보상 함수
///
/// Google Football의 기본 보상 함수와 동일한 방식.
/// 골 득점 시 +1, 실점 시 -1.
#[derive(Debug, Clone)]
pub struct SparseGoalReward {
    /// 홈팀 시점 여부
    ///
    /// - `true`: 홈팀 득점 +1, 어웨이팀 득점 -1
    /// - `false`: 어웨이팀 득점 +1, 홈팀 득점 -1
    home_perspective: bool,
}

impl SparseGoalReward {
    /// 새 보상 함수 생성
    ///
    /// # Arguments
    /// * `home_perspective` - 홈팀 시점 여부
    pub fn new(home_perspective: bool) -> Self {
        Self { home_perspective }
    }

    /// 홈팀 시점 보상 함수 생성 (편의 함수)
    pub fn home() -> Self {
        Self::new(true)
    }

    /// 어웨이팀 시점 보상 함수 생성 (편의 함수)
    pub fn away() -> Self {
        Self::new(false)
    }
}

impl RewardFunction for SparseGoalReward {
    fn compute(&mut self, _prev: &TickSnapshot, _curr: &TickSnapshot, events: &[MatchEvent]) -> f32 {
        let mut reward = 0.0;

        for event in events {
            match event.event_type {
                EventType::Goal => {
                    // 일반 골: 득점 팀에게 +1
                    if event.is_home_team == self.home_perspective {
                        reward += 1.0;
                    } else {
                        reward -= 1.0;
                    }
                }
                EventType::OwnGoal => {
                    // 자책골: 득점 팀(상대팀)에게 +1
                    // is_home_team은 득점 팀 (이득을 본 팀)
                    if event.is_home_team == self.home_perspective {
                        reward += 1.0;
                    } else {
                        reward -= 1.0;
                    }
                }
                _ => {}
            }
        }

        reward
    }

    fn is_sparse(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "sparse_goal"
    }

    // 상태 없음 - 리셋 불필요
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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

    fn make_own_goal_event(is_home_team: bool) -> MatchEvent {
        MatchEvent {
            minute: 45,
            timestamp_ms: Some(1000),
            event_type: EventType::OwnGoal,
            is_home_team,
            player_track_id: Some(15),
            target_track_id: None,
            details: None,
        }
    }

    fn make_dummy_snapshot() -> TickSnapshot {
        use crate::engine::tick_snapshot::{
            BallSnap, BallStateTag, GameModeTag, OffBallObjectiveSnap, PlayerSnap, StickyActionsSnap,
            TeamSnap,
        };
        use crate::engine::types::Coord10;

        TickSnapshot {
            tick: 0,
            minute: 45,
            seed: 42,
            ball: BallSnap {
                state: BallStateTag::Loose,
                pos: Coord10::from_meters(52.5, 34.0),
                owner: None,
                target_pos: None,
                eta_tick: None,
                intended_receiver: None,
                is_shot: false,
            },
            players: std::array::from_fn(|i| PlayerSnap {
                id: i as u8,
                is_home: i < 11,
                pos: Coord10::from_meters(52.5, 34.0),
                state: crate::engine::tick_snapshot::PlayerStateTag::Idle,
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
    fn test_home_scores_home_perspective() {
        let mut reward = SparseGoalReward::home();
        let snap = make_dummy_snapshot();
        let events = vec![make_goal_event(true)]; // 홈팀 득점

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_away_scores_home_perspective() {
        let mut reward = SparseGoalReward::home();
        let snap = make_dummy_snapshot();
        let events = vec![make_goal_event(false)]; // 어웨이팀 득점

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_home_scores_away_perspective() {
        let mut reward = SparseGoalReward::away();
        let snap = make_dummy_snapshot();
        let events = vec![make_goal_event(true)]; // 홈팀 득점

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_own_goal() {
        let mut reward = SparseGoalReward::home();
        let snap = make_dummy_snapshot();
        // 어웨이 선수가 자책골 → is_home_team=true (홈팀 이득)
        let events = vec![make_own_goal_event(true)];

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_multiple_goals() {
        let mut reward = SparseGoalReward::home();
        let snap = make_dummy_snapshot();
        let events = vec![
            make_goal_event(true),  // +1
            make_goal_event(false), // -1
            make_goal_event(true),  // +1
        ];

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - 1.0).abs() < 0.001); // 2 - 1 = 1
    }

    #[test]
    fn test_no_goals() {
        let mut reward = SparseGoalReward::home();
        let snap = make_dummy_snapshot();
        let events: Vec<MatchEvent> = vec![];

        let r = reward.compute(&snap, &snap, &events);
        assert!((r - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_is_sparse() {
        let reward = SparseGoalReward::home();
        assert!(reward.is_sparse());
    }

    #[test]
    fn test_name() {
        let reward = SparseGoalReward::home();
        assert_eq!(reward.name(), "sparse_goal");
    }
}
