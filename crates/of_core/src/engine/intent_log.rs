//! Intent Logging System
//!
//! FIX_2601: BehaviorIntent 로깅 및 분석을 위한 스키마
//!
//! ## 사용 목적
//! 1. CI Gate: Intent 분포/일관성 검증
//! 2. 분석: 경기 후 의사결정 패턴 분석
//! 3. 디버깅: 특정 상황에서의 의사결정 추적

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::action_evaluator::state::PlayerPhaseState;
use super::behavior_intent::BehaviorIntent;
use super::tick_snapshot::IntentKind;
use crate::models::TeamSide;

// ============================================================================
// IntentLogEntry - 단일 의도 로그
// ============================================================================

/// 단일 의도 로그 엔트리
///
/// DecisionPipeline에서 최종 선택된 의도만 기록됨.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentLogEntry {
    /// 게임 tick (0~)
    pub tick: u64,

    /// 경기 분 (0-90+)
    pub minute: u8,

    /// 팀 (Home/Away)
    pub team: TeamSide,

    /// 선수 track_id (0-21)
    pub track_id: u8,

    /// 분류된 PlayerPhaseState
    pub player_phase_state: PlayerPhaseState,

    /// 선택된 BehaviorIntent
    pub behavior_intent: BehaviorIntent,

    /// 실행용 IntentKind (BehaviorIntent에서 변환)
    pub intent_kind: IntentKind,

    /// 피치 영역 (옵션)
    pub pitch_zone: Option<PitchZone>,

    /// 압박 수준 (0.0-1.0, 옵션)
    pub pressure: Option<f32>,

    /// 최종 유틸리티 점수
    pub utility_score: f32,
}

// ============================================================================
// PitchZone - 피치 영역 분류
// ============================================================================

/// 피치 영역 (공간 분석용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PitchZone {
    /// 수비 박스
    DefensiveBox,
    /// 수비 지역 (박스 제외)
    DefensiveThird,
    /// 중앙 지역
    MiddleThird,
    /// 공격 지역 (박스 제외)
    AttackingThird,
    /// 공격 박스
    AttackingBox,
    /// 측면 (좌)
    WideLeft,
    /// 측면 (우)
    WideRight,
}

impl PitchZone {
    /// 위치에서 영역 판별 (공격 방향이 오른쪽일 때 기준)
    pub fn from_position(x: f32, y: f32, attacks_right: bool) -> Self {
        // 필드: 105m x 68m (표준)
        let norm_x = if attacks_right { x } else { 105.0 - x };
        let norm_y = y;

        // 박스 영역 (16.5m)
        if norm_x < 16.5 && norm_y > 13.84 && norm_y < 54.16 {
            return Self::DefensiveBox;
        }
        if norm_x > 88.5 && norm_y > 13.84 && norm_y < 54.16 {
            return Self::AttackingBox;
        }

        // 측면 (폭 10m)
        if norm_y < 10.0 {
            return Self::WideLeft;
        }
        if norm_y > 58.0 {
            return Self::WideRight;
        }

        // 3등분
        if norm_x < 35.0 {
            Self::DefensiveThird
        } else if norm_x < 70.0 {
            Self::MiddleThird
        } else {
            Self::AttackingThird
        }
    }
}

// ============================================================================
// IntentLogger - 로깅 매니저
// ============================================================================

/// Intent 로깅 매니저
///
/// 스레드 안전 (Arc<Mutex<>>)
/// 버퍼 기반 수집 후 배치 처리 가능
#[derive(Debug, Default)]
pub struct IntentLogger {
    /// 로그 버퍼
    entries: Vec<IntentLogEntry>,

    /// 로깅 활성화 여부
    enabled: bool,

    /// 최대 버퍼 크기 (0이면 무제한)
    max_buffer_size: usize,
}

impl IntentLogger {
    /// 새 로거 생성
    pub fn new(enabled: bool) -> Self {
        Self {
            entries: Vec::new(),
            enabled,
            max_buffer_size: 0,
        }
    }

    /// 버퍼 크기 제한 설정
    pub fn with_max_buffer(mut self, max_size: usize) -> Self {
        self.max_buffer_size = max_size;
        self
    }

    /// 로깅 활성화 여부 확인
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 로깅 활성화/비활성화
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 로그 엔트리 추가
    pub fn log(&mut self, entry: IntentLogEntry) {
        if !self.enabled {
            return;
        }

        // 버퍼 크기 제한 적용
        if self.max_buffer_size > 0 && self.entries.len() >= self.max_buffer_size {
            // 오래된 것부터 제거 (FIFO)
            self.entries.remove(0);
        }

        self.entries.push(entry);
    }

    /// 모든 로그 반환
    pub fn entries(&self) -> &[IntentLogEntry] {
        &self.entries
    }

    /// 로그 개수 반환
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 로그가 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 버퍼 비우기
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 로그 소비 (버퍼 비우고 반환)
    pub fn drain(&mut self) -> Vec<IntentLogEntry> {
        std::mem::take(&mut self.entries)
    }
}

// ============================================================================
// PipelineResult -> IntentLogEntry 변환
// ============================================================================

use super::action_evaluator::pipeline::PipelineResult;

impl IntentLogEntry {
    /// PipelineResult에서 IntentLogEntry 생성
    ///
    /// 선택된 액션이 있을 때만 Some 반환.
    pub fn from_pipeline_result(
        result: &PipelineResult,
        tick: u64,
        minute: u8,
        track_id: u8,
        player_x: f32,
        player_y: f32,
        pressure: f32,
        attacks_right: bool,
    ) -> Option<Self> {
        let selected = result.selected.as_ref()?;
        let team = if track_id < 11 {
            TeamSide::Home
        } else {
            TeamSide::Away
        };

        Some(Self {
            tick,
            minute,
            team,
            track_id,
            player_phase_state: result.state,
            behavior_intent: selected.behavior_intent,
            intent_kind: selected.behavior_intent.to_intent_kind(),
            pitch_zone: Some(PitchZone::from_position(player_x, player_y, attacks_right)),
            pressure: Some(pressure),
            utility_score: selected.weighted_total,
        })
    }
}

// ============================================================================
// 분석 헬퍼
// ============================================================================

/// Intent 분포 계산
pub fn calculate_intent_distribution(
    entries: &[IntentLogEntry],
) -> HashMap<BehaviorIntent, usize> {
    let mut counts = HashMap::new();
    for entry in entries {
        *counts.entry(entry.behavior_intent).or_insert(0) += 1;
    }
    counts
}

/// PlayerPhaseState별 Intent 분포 계산
pub fn calculate_intent_by_state(
    entries: &[IntentLogEntry],
) -> HashMap<PlayerPhaseState, HashMap<BehaviorIntent, usize>> {
    let mut by_state: HashMap<PlayerPhaseState, HashMap<BehaviorIntent, usize>> = HashMap::new();

    for entry in entries {
        let state_map = by_state.entry(entry.player_phase_state).or_default();
        *state_map.entry(entry.behavior_intent).or_insert(0) += 1;
    }

    by_state
}

/// 팀별 Intent 분포 계산
pub fn calculate_intent_by_team(
    entries: &[IntentLogEntry],
) -> HashMap<TeamSide, HashMap<BehaviorIntent, usize>> {
    let mut by_team: HashMap<TeamSide, HashMap<BehaviorIntent, usize>> = HashMap::new();

    for entry in entries {
        let team_map = by_team.entry(entry.team).or_default();
        *team_map.entry(entry.behavior_intent).or_insert(0) += 1;
    }

    by_team
}

/// 일관성 검증 (Phase-Intent 일치 확인)
pub fn validate_phase_intent_consistency(entries: &[IntentLogEntry]) -> Vec<IntentLogEntry> {
    use super::behavior_intent::is_allowed;

    entries
        .iter()
        .filter(|e| !is_allowed(e.player_phase_state, e.behavior_intent))
        .cloned()
        .collect()
}

// ============================================================================
// 스레드 안전 버전
// ============================================================================

/// 스레드 안전 IntentLogger (Arc<Mutex<>> 래퍼)
pub type SharedIntentLogger = Arc<Mutex<IntentLogger>>;

/// 스레드 안전 로거 생성
pub fn new_shared_logger(enabled: bool) -> SharedIntentLogger {
    Arc::new(Mutex::new(IntentLogger::new(enabled)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(tick: u64, intent: BehaviorIntent, state: PlayerPhaseState) -> IntentLogEntry {
        IntentLogEntry {
            tick,
            minute: (tick / 4) as u8, // rough conversion
            team: TeamSide::Home,
            track_id: 10,
            player_phase_state: state,
            behavior_intent: intent,
            intent_kind: intent.to_intent_kind(),
            pitch_zone: Some(PitchZone::MiddleThird),
            pressure: Some(0.3),
            utility_score: 0.75,
        }
    }

    #[test]
    fn test_logger_enabled() {
        let mut logger = IntentLogger::new(true);
        let entry = sample_entry(
            100,
            BehaviorIntent::OnBall_Shoot,
            PlayerPhaseState::OnBall,
        );

        logger.log(entry);
        assert_eq!(logger.len(), 1);
    }

    #[test]
    fn test_logger_disabled() {
        let mut logger = IntentLogger::new(false);
        let entry = sample_entry(
            100,
            BehaviorIntent::OnBall_Shoot,
            PlayerPhaseState::OnBall,
        );

        logger.log(entry);
        assert_eq!(logger.len(), 0);
    }

    #[test]
    fn test_logger_buffer_limit() {
        let mut logger = IntentLogger::new(true).with_max_buffer(3);

        for i in 0..5 {
            let entry = sample_entry(
                i,
                BehaviorIntent::OnBall_Shoot,
                PlayerPhaseState::OnBall,
            );
            logger.log(entry);
        }

        assert_eq!(logger.len(), 3);
        // 가장 오래된 것부터 제거됨
        assert_eq!(logger.entries()[0].tick, 2);
    }

    #[test]
    fn test_intent_distribution() {
        let entries = vec![
            sample_entry(1, BehaviorIntent::OnBall_Shoot, PlayerPhaseState::OnBall),
            sample_entry(2, BehaviorIntent::OnBall_Shoot, PlayerPhaseState::OnBall),
            sample_entry(3, BehaviorIntent::OnBall_SafeRecycle, PlayerPhaseState::OnBall),
        ];

        let dist = calculate_intent_distribution(&entries);

        assert_eq!(dist.get(&BehaviorIntent::OnBall_Shoot), Some(&2));
        assert_eq!(dist.get(&BehaviorIntent::OnBall_SafeRecycle), Some(&1));
    }

    #[test]
    fn test_phase_intent_consistency() {
        let entries = vec![
            // 일관성 있음
            sample_entry(1, BehaviorIntent::OnBall_Shoot, PlayerPhaseState::OnBall),
            // 일관성 위반 (OnBall 상태에서 Defend 의도)
            sample_entry(
                2,
                BehaviorIntent::Defend_TackleAttempt,
                PlayerPhaseState::OnBall,
            ),
        ];

        let violations = validate_phase_intent_consistency(&entries);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].tick, 2);
    }

    #[test]
    fn test_pitch_zone_classification() {
        // 공격 박스
        assert_eq!(
            PitchZone::from_position(95.0, 34.0, true),
            PitchZone::AttackingBox
        );

        // 수비 박스
        assert_eq!(
            PitchZone::from_position(10.0, 34.0, true),
            PitchZone::DefensiveBox
        );

        // 중앙
        assert_eq!(
            PitchZone::from_position(52.5, 34.0, true),
            PitchZone::MiddleThird
        );

        // 측면 (좌)
        assert_eq!(
            PitchZone::from_position(52.5, 5.0, true),
            PitchZone::WideLeft
        );
    }

    #[test]
    fn test_drain() {
        let mut logger = IntentLogger::new(true);
        logger.log(sample_entry(1, BehaviorIntent::OnBall_Shoot, PlayerPhaseState::OnBall));
        logger.log(sample_entry(2, BehaviorIntent::OnBall_SafeRecycle, PlayerPhaseState::OnBall));

        let drained = logger.drain();
        assert_eq!(drained.len(), 2);
        assert!(logger.is_empty());
    }
}
