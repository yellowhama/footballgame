//! Special Skills System
//!
//! RPG-style special skills that trigger based on attribute combinations

use serde::{Deserialize, Serialize};

/// 스킬 발동 액션 타입 (match_sim.rs에서 공유)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionType {
    DribblingSkill,
    FinishingSkill,
    LongShotSkill,
    ThroughBallSkill,
    ShortPassSkill,
    CrossSkill,
    HeaderSkill,
    TackleSkill,
    PenaltySkill,   // 페널티킥
    SpeedDuelSkill, // 순수 속도 경합
}

/// 스킬 발동 컨텍스트 (조건부 스킬용)
#[derive(Debug, Clone, Default)]
pub struct SkillContext {
    /// 칩슛 시도 여부 (파넨카용)
    pub is_chip_shot: bool,
    /// 오프사이드 라인 경합 여부 (포처용)
    pub is_offside_line_battle: bool,
    /// 1:1 상황 여부
    pub is_one_on_one: bool,
    /// 페널티킥 여부
    pub is_penalty_kick: bool,
    /// 공중볼 경합 여부
    pub is_aerial_duel: bool,
    /// 거리 (m) - 중거리 슛 등에서 사용
    pub distance_m: f32,
}

/// Special skills that can be equipped by players
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecialSkill {
    // === 멘탈/창의성 기반 스킬 (Flair-Based) ===
    /// 앵클 브레이커: 드리블 시 수비수 Frozen 확률 1.5배
    AnkleBreaker,

    /// 마에스트로: 스루패스 시 수비수 인터셉트 반응 저하
    Maestro,

    /// 파넨카: 1:1/PK에서 칩슛 시 키퍼 100% 주저앉음
    Panenka,

    /// 택배 크로스: 크로스/코너킥 정확도 +20%, curve_factor 2배
    CurveArtist,

    // === 물리/기술 기반 스킬 (Physical & Technical) ===
    /// 감아차기: 골키퍼 Reach 페널티, 구석 명중률 +25%
    FinesseShot,

    /// 파워 헤더: 공중볼 경합 +40%, 헤딩 속도 1.5배
    PowerHeader,

    /// 라인 브레이커: Marking 회피 +35%, 오프사이드 세이프 확률 증가
    Poacher,

    /// 완벽한 태클: 파울 확률 0%, 즉시 소유권 획득
    PerfectTackle,

    /// 캐논 슈터: 중거리 슛 시 키퍼 Fumble 확률 대폭 증가, 슛 파워 1.3배
    Cannon,

    /// 치고 달리기: 순수 속도 경합 항상 승리, Pace 차이 5+ 시 자동 돌파
    SpeedDemon,
}

impl SpecialSkill {
    /// 스킬 이름 (영문)
    pub fn name(&self) -> &'static str {
        match self {
            Self::AnkleBreaker => "Ankle Breaker",
            Self::Maestro => "The Maestro",
            Self::Panenka => "Panenka",
            Self::CurveArtist => "Curve Artist",
            Self::FinesseShot => "Finesse Shot",
            Self::PowerHeader => "Power Header",
            Self::Poacher => "Poacher",
            Self::PerfectTackle => "Perfect Tackle",
            Self::Cannon => "Cannon",
            Self::SpeedDemon => "Speed Demon",
        }
    }

    /// 스킬 아이콘 (이모지)
    pub fn icon(&self) -> &'static str {
        match self {
            Self::AnkleBreaker => "🌪️",
            Self::Maestro => "👁️",
            Self::Panenka => "🥄",
            Self::CurveArtist => "🎯",
            Self::FinesseShot => "⚽",
            Self::PowerHeader => "💥",
            Self::Poacher => "🏃",
            Self::PerfectTackle => "🛡️",
            Self::Cannon => "💣",
            Self::SpeedDemon => "💨",
        }
    }

    /// 이 스킬이 특정 액션에 주는 보너스 배율 반환
    /// - 1.0 = 보너스 없음
    /// - 1.5 = 50% 보너스 등
    pub fn get_bonus_multiplier(&self, action: ActionType) -> f32 {
        match (self, action) {
            // 앵클 브레이커: 드리블 시 1.5배
            (Self::AnkleBreaker, ActionType::DribblingSkill) => 1.5,

            // 마에스트로: 스루패스 시 1.4배
            (Self::Maestro, ActionType::ThroughBallSkill) => 1.4,

            // 파넨카: 페널티/마무리 시 2.0배 (칩슛 조건은 meets_activation_condition에서)
            (Self::Panenka, ActionType::PenaltySkill) => 2.0,
            (Self::Panenka, ActionType::FinishingSkill) => 1.5,

            // 커브 아티스트: 크로스 +30%, 숏패스 +10%
            (Self::CurveArtist, ActionType::CrossSkill) => 1.3,
            (Self::CurveArtist, ActionType::ShortPassSkill) => 1.1,

            // 피네세샷: 마무리 +25%
            (Self::FinesseShot, ActionType::FinishingSkill) => 1.25,

            // 파워헤더: 헤딩 +50%
            (Self::PowerHeader, ActionType::HeaderSkill) => 1.5,

            // 포처: 마무리 +20% (오프사이드 세이프는 별도 로직)
            (Self::Poacher, ActionType::FinishingSkill) => 1.2,

            // 퍼펙트 태클: 태클 성공률 +40%
            (Self::PerfectTackle, ActionType::TackleSkill) => 1.4,

            // 캐논: 롱샷 +30%
            (Self::Cannon, ActionType::LongShotSkill) => 1.3,

            // 스피드 데몬: 속도 경합 +50%
            (Self::SpeedDemon, ActionType::SpeedDuelSkill) => 1.5,
            (Self::SpeedDemon, ActionType::DribblingSkill) => 1.2,

            // 해당 없음
            _ => 1.0,
        }
    }

    /// 스킬 발동 조건 검사 (조건부 스킬용)
    /// - true: 발동 가능
    /// - false: 조건 미충족
    pub fn meets_activation_condition(&self, context: &SkillContext) -> bool {
        match self {
            // 파넨카: 칩슛 또는 1:1 상황에서만 발동
            Self::Panenka => {
                context.is_chip_shot || context.is_one_on_one || context.is_penalty_kick
            }

            // 포처: 오프사이드 라인 경합 상황에서 추가 효과
            Self::Poacher => context.is_offside_line_battle,

            // 파워 헤더: 공중볼 경합에서만 발동
            Self::PowerHeader => context.is_aerial_duel,

            // 캐논: 20m 이상 거리에서만 발동
            Self::Cannon => context.distance_m >= 20.0,

            // 기타 스킬은 항상 발동
            _ => true,
        }
    }

    /// 스킬이 주는 특수 효과 설명
    pub fn get_special_effect(&self) -> &'static str {
        match self {
            Self::AnkleBreaker => "수비수 Frozen 확률 증가",
            Self::Maestro => "수비수 인터셉트 반응 저하",
            Self::Panenka => "칩슛 시 키퍼 100% 주저앉음",
            Self::CurveArtist => "curve_factor 2배",
            Self::FinesseShot => "구석 명중률 +25%",
            Self::PowerHeader => "공중볼 경합 +40%",
            Self::Poacher => "오프사이드 세이프 확률 증가",
            Self::PerfectTackle => "파울 확률 0%",
            Self::Cannon => "키퍼 Fumble 확률 증가",
            Self::SpeedDemon => "순수 속도 경합 항상 승리",
        }
    }

    /// 스킬 발동 시 추가 Frozen 확률 (0.0 ~ 1.0)
    pub fn get_frozen_chance_bonus(&self) -> f32 {
        match self {
            Self::AnkleBreaker => 0.25, // +25% Frozen 확률
            Self::Maestro => 0.15,      // +15% 수비수 반응 저하
            Self::Panenka => 0.50,      // +50% 키퍼 주저앉음
            _ => 0.0,
        }
    }
}
