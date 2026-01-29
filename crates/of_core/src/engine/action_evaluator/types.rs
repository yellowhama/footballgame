//! UAE Core Types
//!
//! FIX_2601/0108: Unified Action Evaluator 핵심 타입 정의
//! - Action enum (모든 액션 통합)
//! - ActionScore (6요소 점수)
//! - ActionWeights (가중치)
//! - ScoredAction (평가된 액션 + BehaviorIntent)

use crate::engine::behavior_intent::BehaviorIntent;

/// 선수 식별자 (track_id 기반)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PlayerId(pub u8);

impl PlayerId {
    pub fn new(id: u8) -> Self {
        Self(id)
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// 크로스 타겟 존
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossZone {
    NearPost,
    FarPost,
    PenaltySpot,
    Cutback,
}

/// 필드 존
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Zone {
    pub x: i8, // 0-9 (피치를 10등분)
    pub y: i8, // 0-6 (피치를 7등분)
}

/// 패스 레인
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassLane {
    pub from: (f32, f32),
    pub to: (f32, f32),
}

/// 2D 벡터 (방향)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn normalized(self) -> Self {
        let len = (self.x * self.x + self.y * self.y).sqrt();
        if len > 0.0001 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self::default()
        }
    }
}

/// 위치
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 모든 가능한 액션 (통합)
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // === On-Ball 액션 ===
    /// 슛
    Shoot,

    /// 패스
    Pass { target_id: PlayerId },

    /// 스루볼
    ThroughBall { target_id: PlayerId },

    /// 드리블
    Dribble { direction: Vec2 },

    /// 크로스
    Cross { target_zone: CrossZone },

    /// 홀드업
    Hold,

    /// 헤딩 (슛/패스)
    Header { is_shot: bool },

    /// 클리어
    Clear,

    // === Off-Ball 공격 액션 ===
    /// 공간 침투
    RunIntoSpace { target: Position },

    /// 서포트 (패스 옵션 제공)
    Support { position: Position },

    /// 오버래핑 런
    Overlap,

    /// 위치 유지
    HoldPosition,

    // === 수비 액션 ===
    /// 압박
    Press,

    /// 태클
    Tackle,

    /// 조키 (지연)
    Jockey,

    /// 마킹
    Mark { target_id: PlayerId },

    /// 커버
    Cover { zone: Zone },

    /// 인터셉트
    Intercept { lane: PassLane },

    /// 패스레인 차단
    BlockLane { lane: PassLane },

    // === 전환 액션 ===
    /// 역압박
    CounterPress,

    /// 지연
    Delay,

    /// 긴급 커버
    CoverEmergency { zone: Zone },

    /// 역습 첫 패스
    FirstPassForward { target_id: PlayerId },

    /// 캐리 (역습 드리블)
    Carry { direction: Vec2 },

    /// 역습 서포트 런
    RunSupport { target_space: Position },

    // === FIX_2601/0113 Phase 2: 새 액션 ===
    /// 파울 유도 (세트피스 획득 목적)
    DrawFoul,

    /// 수비적 복귀 (역습 저지)
    RecoveryRun { target: Position },
}

impl Action {
    /// On-Ball 액션인지 확인
    pub fn is_on_ball(&self) -> bool {
        matches!(
            self,
            Action::Shoot
                | Action::Pass { .. }
                | Action::ThroughBall { .. }
                | Action::Dribble { .. }
                | Action::Cross { .. }
                | Action::Hold
                | Action::Header { .. }
                | Action::Clear
        )
    }

    /// Off-Ball 액션인지 확인
    pub fn is_off_ball(&self) -> bool {
        !self.is_on_ball()
    }

    /// 수비 액션인지 확인
    pub fn is_defensive(&self) -> bool {
        matches!(
            self,
            Action::Press
                | Action::Tackle
                | Action::Jockey
                | Action::Mark { .. }
                | Action::Cover { .. }
                | Action::Intercept { .. }
                | Action::BlockLane { .. }
                | Action::CounterPress
                | Action::Delay
                | Action::CoverEmergency { .. }
        )
    }
}

/// UAE 6요소 점수
///
/// 모든 값은 [0.0, 1.0] 범위로 정규화됨
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionScore {
    /// Distance (거리 적합성) - 20%
    pub distance: f32,

    /// Safety (안전성) - 25% (가장 높은 비중)
    pub safety: f32,

    /// Readiness (준비 상태) - 15%
    pub readiness: f32,

    /// Progression (진행도) - 20%
    /// 슛: xG 기반, 패스: xG 델타
    pub progression: f32,

    /// Space (공간) - 10%
    pub space: f32,

    /// Tactical (전술) - 10%
    pub tactical: f32,
}

impl ActionScore {
    /// 가중치 적용하여 총점 계산
    pub fn apply_weights(&self, weights: &ActionWeights) -> f32 {
        self.distance * weights.distance
            + self.safety * weights.safety
            + self.readiness * weights.readiness
            + self.progression * weights.progression
            + self.space * weights.space
            + self.tactical * weights.tactical
    }

    /// 모든 값이 [0.0, 1.0] 범위인지 검증
    pub fn validate(&self) -> bool {
        (0.0..=1.0).contains(&self.distance)
            && (0.0..=1.0).contains(&self.safety)
            && (0.0..=1.0).contains(&self.readiness)
            && (0.0..=1.0).contains(&self.progression)
            && (0.0..=1.0).contains(&self.space)
            && (0.0..=1.0).contains(&self.tactical)
    }
}

/// UAE 가중치
///
/// 포지션/성향/전술에 따라 조정됨
/// Multiplicative 방식으로 적용 (additive 아님)
#[derive(Debug, Clone, Copy)]
pub struct ActionWeights {
    pub distance: f32,
    pub safety: f32,
    pub readiness: f32,
    pub progression: f32,
    pub space: f32,
    pub tactical: f32,
}

impl Default for ActionWeights {
    fn default() -> Self {
        // 기본 가중치 (합계 1.0)
        Self {
            distance: 0.20,
            safety: 0.25,
            readiness: 0.15,
            progression: 0.20,
            space: 0.10,
            tactical: 0.10,
        }
    }
}

impl ActionWeights {
    /// 모든 가중치를 [min, max] 범위로 클램프
    pub fn clamp_all(&mut self, min: f32, max: f32) {
        self.distance = self.distance.clamp(min, max);
        self.safety = self.safety.clamp(min, max);
        self.readiness = self.readiness.clamp(min, max);
        self.progression = self.progression.clamp(min, max);
        self.space = self.space.clamp(min, max);
        self.tactical = self.tactical.clamp(min, max);
    }

    /// Multiplicative 수정 적용
    pub fn apply_multiplier(&mut self, factor: WeightMultiplier) {
        self.distance *= factor.distance;
        self.safety *= factor.safety;
        self.readiness *= factor.readiness;
        self.progression *= factor.progression;
        self.space *= factor.space;
        self.tactical *= factor.tactical;
    }
}

/// 가중치 배율 (Multiplicative 수정용)
#[derive(Debug, Clone, Copy)]
pub struct WeightMultiplier {
    pub distance: f32,
    pub safety: f32,
    pub readiness: f32,
    pub progression: f32,
    pub space: f32,
    pub tactical: f32,
}

impl Default for WeightMultiplier {
    fn default() -> Self {
        Self {
            distance: 1.0,
            safety: 1.0,
            readiness: 1.0,
            progression: 1.0,
            space: 1.0,
            tactical: 1.0,
        }
    }
}

/// 평가된 액션 (점수 + BehaviorIntent 포함)
#[derive(Debug, Clone)]
pub struct ScoredAction {
    pub action: Action,
    pub score: ActionScore,
    pub weighted_total: f32,
    /// 세분화된 행동 의도 (FIX_2601)
    pub behavior_intent: BehaviorIntent,
}

impl ScoredAction {
    pub fn new(action: Action, score: ActionScore, weights: &ActionWeights) -> Self {
        let weighted_total = score.apply_weights(weights);
        // 기본 BehaviorIntent (컨텍스트 없이 Action만으로 추론)
        let behavior_intent = BehaviorIntent::from_action_simple(&action);
        Self {
            action,
            score,
            weighted_total,
            behavior_intent,
        }
    }

    /// BehaviorIntent를 명시적으로 지정하여 생성
    pub fn with_intent(
        action: Action,
        score: ActionScore,
        weights: &ActionWeights,
        behavior_intent: BehaviorIntent,
    ) -> Self {
        let weighted_total = score.apply_weights(weights);
        Self {
            action,
            score,
            weighted_total,
            behavior_intent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_score_validation() {
        let valid = ActionScore {
            distance: 0.8,
            safety: 0.6,
            readiness: 0.7,
            progression: 0.5,
            space: 0.9,
            tactical: 0.3,
        };
        assert!(valid.validate());

        let invalid = ActionScore {
            distance: 1.5, // out of range
            ..Default::default()
        };
        assert!(!invalid.validate());
    }

    #[test]
    fn test_weighted_score() {
        let score = ActionScore {
            distance: 1.0,
            safety: 1.0,
            readiness: 1.0,
            progression: 1.0,
            space: 1.0,
            tactical: 1.0,
        };
        let weights = ActionWeights::default();
        let total = score.apply_weights(&weights);
        assert!((total - 1.0).abs() < 0.001); // 모두 1.0이면 총합도 1.0
    }

    #[test]
    fn test_action_classification() {
        assert!(Action::Shoot.is_on_ball());
        assert!(!Action::Shoot.is_off_ball());

        assert!(Action::Press.is_off_ball());
        assert!(Action::Press.is_defensive());

        assert!(Action::RunIntoSpace {
            target: Position::default()
        }
        .is_off_ball());
        assert!(!Action::RunIntoSpace {
            target: Position::default()
        }
        .is_defensive());
    }
}
