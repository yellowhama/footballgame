//! Hero Action Tag System
//!
//! Phase 5: Hero Time 액션을 스탯 성장에 연결하는 태그 시스템
//!
//! ## 핵심 개념
//! - HeroActionTag: Hero Time 액션 유형 (패스, 드리블, 슈팅 등)
//! - HeroXpEvent: XP 획득 이벤트 (성공/실패, 압박, 피로 등)
//! - affected_attributes(): 액션이 영향 주는 스탯 매핑

use serde::{Deserialize, Serialize};

/// Hero Time 액션 유형
///
/// 각 태그는 특정 스탯 조합에 XP를 부여함
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeroActionTag {
    // ========== 패스 계열 ==========
    /// 안전한 패스 (횡패스, 후방 패스)
    SafePass,
    /// 전진 패스 (평범한 전방 패스)
    ForwardPass,
    /// 스루 패스 (수비 라인 뚫기)
    ThroughPass,
    /// 롱패스 / 크로스
    LobPass,

    // ========== 드리블 계열 ==========
    /// 안전한 드리블 (공간으로)
    SafeDribble,
    /// 1v1 돌파 성공
    DribblePastOpponent,

    // ========== 슈팅 계열 ==========
    /// 박스 내 슈팅
    BoxShot,
    /// 중거리 슛
    LongShot,
    /// 헤딩 슛
    HeaderShot,

    // ========== 수비 계열 ==========
    /// 패스 차단
    Interception,
    /// 태클 성공
    Tackle,
    /// 공중볼 경합 승리
    AerialDuel,
}

/// 스탯 속성 (XP가 영향 주는 대상)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerAttribute {
    // 테크니컬
    Passing,
    Dribbling,
    Finishing,
    LongShots,
    Technique,
    FirstTouch,

    // 멘탈
    Vision,
    Composure,
    Decisions,
    Anticipation,
    Positioning,
    Flair,

    // 피지컬
    Pace,
    Agility,
    Strength,
    Jumping,

    // 수비
    Tackling,
    Aggression,
    Marking,
}

impl HeroActionTag {
    /// 이 액션이 영향 주는 스탯들 (비중)
    ///
    /// 반환: Vec<(속성, 가중치)> - 가중치 합계는 1.0
    pub fn affected_attributes(&self) -> Vec<(PlayerAttribute, f32)> {
        use PlayerAttribute::*;

        match self {
            // 패스 계열
            HeroActionTag::SafePass => vec![(Passing, 0.6), (Composure, 0.4)],
            HeroActionTag::ForwardPass => vec![(Passing, 0.5), (Vision, 0.3), (Decisions, 0.2)],
            HeroActionTag::ThroughPass => vec![(Passing, 0.4), (Vision, 0.4), (Decisions, 0.2)],
            HeroActionTag::LobPass => vec![(Passing, 0.5), (Technique, 0.3), (Vision, 0.2)],

            // 드리블 계열
            HeroActionTag::SafeDribble => {
                vec![(Dribbling, 0.5), (Composure, 0.3), (FirstTouch, 0.2)]
            }
            HeroActionTag::DribblePastOpponent => {
                vec![(Dribbling, 0.5), (Agility, 0.3), (Flair, 0.2)]
            }

            // 슈팅 계열
            HeroActionTag::BoxShot => vec![(Finishing, 0.6), (Composure, 0.3), (Technique, 0.1)],
            HeroActionTag::LongShot => vec![(LongShots, 0.5), (Technique, 0.3), (Composure, 0.2)],
            HeroActionTag::HeaderShot => vec![(Finishing, 0.4), (Jumping, 0.4), (Strength, 0.2)],

            // 수비 계열
            HeroActionTag::Interception => {
                vec![(Anticipation, 0.5), (Positioning, 0.3), (Decisions, 0.2)]
            }
            HeroActionTag::Tackle => vec![(Tackling, 0.6), (Strength, 0.2), (Aggression, 0.2)],
            HeroActionTag::AerialDuel => vec![(Jumping, 0.5), (Strength, 0.3), (Marking, 0.2)],
        }
    }

    /// 기본 XP 값
    ///
    /// 더 어려운/위험한 액션일수록 높은 기본 XP
    pub fn base_xp(&self) -> f32 {
        match self {
            HeroActionTag::SafePass => 1.0,
            HeroActionTag::ForwardPass => 2.0,
            HeroActionTag::ThroughPass => 5.0,
            HeroActionTag::LobPass => 3.0,
            HeroActionTag::SafeDribble => 2.0,
            HeroActionTag::DribblePastOpponent => 6.0,
            HeroActionTag::BoxShot => 4.0,
            HeroActionTag::LongShot => 3.0,
            HeroActionTag::HeaderShot => 4.0,
            HeroActionTag::Interception => 4.0,
            HeroActionTag::Tackle => 3.0,
            HeroActionTag::AerialDuel => 3.0,
        }
    }
}

/// Hero Time XP 이벤트
///
/// 단일 액션에서 발생한 XP 획득 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroXpEvent {
    /// 액션 유형
    pub tag: HeroActionTag,
    /// 성공 여부
    pub success: bool,
    /// 경기 시간 (분)
    pub minute: u8,
    /// 주변 수비수 압박 강도 (0.0 ~ 1.0)
    pub pressure_level: f32,
    /// 피로도 (0.0 ~ 1.0)
    pub fatigue_level: f32,
    /// 상황 난이도 (0.0 ~ 1.0)
    pub context_difficulty: f32,
}

impl HeroXpEvent {
    /// 새 XP 이벤트 생성
    pub fn new(tag: HeroActionTag, success: bool, minute: u8) -> Self {
        Self {
            tag,
            success,
            minute,
            pressure_level: 0.0,
            fatigue_level: 0.0,
            context_difficulty: 0.0,
        }
    }

    /// 압박 레벨 설정
    pub fn with_pressure(mut self, pressure: f32) -> Self {
        self.pressure_level = pressure.clamp(0.0, 1.0);
        self
    }

    /// 피로 레벨 설정
    pub fn with_fatigue(mut self, fatigue: f32) -> Self {
        self.fatigue_level = fatigue.clamp(0.0, 1.0);
        self
    }

    /// 상황 난이도 설정
    pub fn with_difficulty(mut self, difficulty: f32) -> Self {
        self.context_difficulty = difficulty.clamp(0.0, 1.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affected_attributes_weight_sum() {
        // 모든 태그의 가중치 합이 1.0인지 확인
        let tags = [
            HeroActionTag::SafePass,
            HeroActionTag::ForwardPass,
            HeroActionTag::ThroughPass,
            HeroActionTag::LobPass,
            HeroActionTag::SafeDribble,
            HeroActionTag::DribblePastOpponent,
            HeroActionTag::BoxShot,
            HeroActionTag::LongShot,
            HeroActionTag::HeaderShot,
            HeroActionTag::Interception,
            HeroActionTag::Tackle,
            HeroActionTag::AerialDuel,
        ];

        for tag in tags {
            let attrs = tag.affected_attributes();
            let sum: f32 = attrs.iter().map(|(_, w)| w).sum();
            assert!((sum - 1.0).abs() < 0.001, "{:?} weight sum is {}, expected 1.0", tag, sum);
        }
    }

    #[test]
    fn test_base_xp_values() {
        // 위험한 액션이 더 높은 XP
        assert!(HeroActionTag::ThroughPass.base_xp() > HeroActionTag::SafePass.base_xp());
        assert!(
            HeroActionTag::DribblePastOpponent.base_xp() > HeroActionTag::SafeDribble.base_xp()
        );
    }

    #[test]
    fn test_hero_xp_event_builder() {
        let event = HeroXpEvent::new(HeroActionTag::ThroughPass, true, 45)
            .with_pressure(0.6)
            .with_fatigue(0.3)
            .with_difficulty(0.5);

        assert_eq!(event.tag, HeroActionTag::ThroughPass);
        assert!(event.success);
        assert_eq!(event.minute, 45);
        assert!((event.pressure_level - 0.6).abs() < 0.001);
        assert!((event.fatigue_level - 0.3).abs() < 0.001);
        assert!((event.context_difficulty - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_clamp_values() {
        let event = HeroXpEvent::new(HeroActionTag::SafePass, true, 10)
            .with_pressure(1.5) // 초과
            .with_fatigue(-0.5); // 미만

        assert!((event.pressure_level - 1.0).abs() < 0.001);
        assert!((event.fatigue_level - 0.0).abs() < 0.001);
    }
}
