//! EpisodeMetrics - RL Episode Statistics
//!
//! FIX_2601 Phase 3: RL 훈련용 에피소드 메트릭
//!
//! ## 메트릭 구성
//!
//! - 기본 통계: 틱 수, 누적 보상, 최종 스코어
//! - 종료 사유: 시간 종료, 득점, 점유 변경 등
//! - 액션 통계: 타입별 횟수, 성공률
//!
//! ## 사용 예시
//!
//! ```rust,ignore
//! use of_core::engine::reward::{EpisodeMetrics, TerminationReason};
//!
//! let mut metrics = EpisodeMetrics::new();
//! metrics.record_tick(reward);
//! metrics.record_goal(true);
//! metrics.set_termination(TerminationReason::GoalScored);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::engine::action_queue::PhaseActionType;

// ============================================================================
// TerminationReason
// ============================================================================

/// 에피소드 종료 사유
///
/// Google Football의 episode termination 조건 참조.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    /// 시간 종료 (기본)
    #[default]
    TimeUp,
    /// 골 득점 (end_on_score 옵션)
    GoalScored,
    /// 점유 변경 (end_on_possession_change 옵션)
    PossessionChange,
    /// 공 아웃 (out_of_play)
    OutOfBounds,
    /// 시나리오 특정 조건
    CustomCondition,
    /// 진행 중 (아직 종료되지 않음)
    InProgress,
}

impl TerminationReason {
    /// 에피소드가 종료되었는지 확인
    pub fn is_terminal(&self) -> bool {
        !matches!(self, TerminationReason::InProgress)
    }
}

// ============================================================================
// EpisodeMetrics
// ============================================================================

/// 에피소드 메트릭 (RL 훈련용)
///
/// 에피소드 동안 수집된 통계.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpisodeMetrics {
    /// 총 틱 수
    pub total_ticks: u64,

    /// 에피소드 보상 누적
    pub cumulative_reward: f32,

    /// 최종 스코어 (home, away)
    pub final_score: (u32, u32),

    /// 에피소드 종료 사유
    pub termination_reason: TerminationReason,

    /// 홈팀 점유 시간 (틱)
    pub home_possession_ticks: u64,

    /// 어웨이팀 점유 시간 (틱)
    pub away_possession_ticks: u64,

    /// 점유 변경 횟수
    pub possession_changes: u32,

    /// 액션 통계 (Phase 3에서 확장)
    pub action_stats: ActionStats,
}

impl EpisodeMetrics {
    /// 새 에피소드 메트릭 생성
    pub fn new() -> Self {
        Self::default()
    }

    /// 틱 기록
    pub fn record_tick(&mut self, reward: f32) {
        self.total_ticks += 1;
        self.cumulative_reward += reward;
    }

    /// 점유 기록
    pub fn record_possession(&mut self, is_home: bool) {
        if is_home {
            self.home_possession_ticks += 1;
        } else {
            self.away_possession_ticks += 1;
        }
    }

    /// 점유 변경 기록
    pub fn record_possession_change(&mut self) {
        self.possession_changes += 1;
    }

    /// 골 기록
    pub fn record_goal(&mut self, is_home: bool) {
        if is_home {
            self.final_score.0 += 1;
        } else {
            self.final_score.1 += 1;
        }
    }

    /// 종료 사유 설정
    pub fn set_termination(&mut self, reason: TerminationReason) {
        self.termination_reason = reason;
    }

    /// 액션 기록
    pub fn record_action(&mut self, action_type: PhaseActionType, success: bool) {
        self.action_stats.record(action_type, success);
    }

    /// 점유율 계산 (홈팀 기준, 0.0 ~ 1.0)
    pub fn home_possession_rate(&self) -> f32 {
        let total = self.home_possession_ticks + self.away_possession_ticks;
        if total == 0 {
            0.5
        } else {
            self.home_possession_ticks as f32 / total as f32
        }
    }

    /// 평균 점유 시간 (틱)
    pub fn avg_possession_duration(&self) -> f32 {
        if self.possession_changes == 0 {
            self.total_ticks as f32
        } else {
            self.total_ticks as f32 / self.possession_changes as f32
        }
    }

    /// 에피소드 리셋
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ============================================================================
// ActionStats
// ============================================================================

/// 액션 통계
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionStats {
    /// 액션 타입별 시도 횟수
    pub attempts: HashMap<PhaseActionType, u32>,

    /// 액션 타입별 성공 횟수
    pub successes: HashMap<PhaseActionType, u32>,
}

impl ActionStats {
    /// 새 액션 통계 생성
    pub fn new() -> Self {
        Self::default()
    }

    /// 액션 기록
    pub fn record(&mut self, action_type: PhaseActionType, success: bool) {
        *self.attempts.entry(action_type).or_insert(0) += 1;
        if success {
            *self.successes.entry(action_type).or_insert(0) += 1;
        }
    }

    /// 특정 액션의 시도 횟수
    pub fn attempt_count(&self, action_type: PhaseActionType) -> u32 {
        self.attempts.get(&action_type).copied().unwrap_or(0)
    }

    /// 특정 액션의 성공 횟수
    pub fn success_count(&self, action_type: PhaseActionType) -> u32 {
        self.successes.get(&action_type).copied().unwrap_or(0)
    }

    /// 특정 액션의 성공률
    pub fn success_rate(&self, action_type: PhaseActionType) -> f32 {
        let attempts = self.attempt_count(action_type);
        if attempts == 0 {
            0.0
        } else {
            self.success_count(action_type) as f32 / attempts as f32
        }
    }

    /// 패스 성공률
    pub fn pass_success_rate(&self) -> f32 {
        self.success_rate(PhaseActionType::Pass)
    }

    /// 슛 유효율 (on target)
    pub fn shot_on_target_rate(&self) -> f32 {
        self.success_rate(PhaseActionType::Shot)
    }

    /// 태클 성공률
    pub fn tackle_success_rate(&self) -> f32 {
        self.success_rate(PhaseActionType::Tackle)
    }

    /// 총 액션 수
    pub fn total_actions(&self) -> u32 {
        self.attempts.values().sum()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_metrics_new() {
        let metrics = EpisodeMetrics::new();
        assert_eq!(metrics.total_ticks, 0);
        assert_eq!(metrics.cumulative_reward, 0.0);
        assert_eq!(metrics.final_score, (0, 0));
        assert_eq!(metrics.termination_reason, TerminationReason::TimeUp);
    }

    #[test]
    fn test_record_tick() {
        let mut metrics = EpisodeMetrics::new();
        metrics.record_tick(0.5);
        metrics.record_tick(0.3);
        metrics.record_tick(-0.1);

        assert_eq!(metrics.total_ticks, 3);
        assert!((metrics.cumulative_reward - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_record_possession() {
        let mut metrics = EpisodeMetrics::new();
        for _ in 0..60 {
            metrics.record_possession(true);
        }
        for _ in 0..40 {
            metrics.record_possession(false);
        }

        assert_eq!(metrics.home_possession_ticks, 60);
        assert_eq!(metrics.away_possession_ticks, 40);
        assert!((metrics.home_possession_rate() - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_record_goal() {
        let mut metrics = EpisodeMetrics::new();
        metrics.record_goal(true);
        metrics.record_goal(true);
        metrics.record_goal(false);

        assert_eq!(metrics.final_score, (2, 1));
    }

    #[test]
    fn test_termination_reason() {
        assert!(!TerminationReason::InProgress.is_terminal());
        assert!(TerminationReason::TimeUp.is_terminal());
        assert!(TerminationReason::GoalScored.is_terminal());
        assert!(TerminationReason::PossessionChange.is_terminal());
    }

    #[test]
    fn test_action_stats() {
        let mut stats = ActionStats::new();

        // 패스: 10 시도, 8 성공
        for _ in 0..8 {
            stats.record(PhaseActionType::Pass, true);
        }
        for _ in 0..2 {
            stats.record(PhaseActionType::Pass, false);
        }

        // 슛: 5 시도, 3 성공
        for _ in 0..3 {
            stats.record(PhaseActionType::Shot, true);
        }
        for _ in 0..2 {
            stats.record(PhaseActionType::Shot, false);
        }

        assert_eq!(stats.attempt_count(PhaseActionType::Pass), 10);
        assert_eq!(stats.success_count(PhaseActionType::Pass), 8);
        assert!((stats.pass_success_rate() - 0.8).abs() < 0.001);

        assert_eq!(stats.attempt_count(PhaseActionType::Shot), 5);
        assert!((stats.shot_on_target_rate() - 0.6).abs() < 0.001);

        assert_eq!(stats.total_actions(), 15);
    }

    #[test]
    fn test_avg_possession_duration() {
        let mut metrics = EpisodeMetrics::new();
        metrics.total_ticks = 1000;
        metrics.possession_changes = 10;

        assert!((metrics.avg_possession_duration() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let mut metrics = EpisodeMetrics::new();
        metrics.record_tick(1.0);
        metrics.record_goal(true);
        metrics.set_termination(TerminationReason::GoalScored);

        metrics.reset();

        assert_eq!(metrics.total_ticks, 0);
        assert_eq!(metrics.cumulative_reward, 0.0);
        assert_eq!(metrics.final_score, (0, 0));
        assert_eq!(metrics.termination_reason, TerminationReason::TimeUp);
    }
}
