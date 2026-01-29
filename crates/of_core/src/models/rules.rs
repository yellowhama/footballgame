//! IFAB Laws of the Game - Rule Types
//!
//! Rule-as-Data 시스템을 위한 Rust 타입 정의.
//! YAML 데이터 파일과 매핑되어 규칙 설명/분류를 제공합니다.
//!
//! - Law 7: Duration of Match
//! - Law 8: Start and Restart of Play
//! - Law 9: Ball In and Out of Play
//! - Law 10: Determining the Outcome (Goal)
//! - Law 11: Offside
//! - Law 12: Fouls and Misconduct
//! - Law 13: Free Kicks
//! - Law 14: Penalty Kick
//! - Law 15: Throw-In
//! - Law 16: Goal Kick
//! - Law 17: Corner Kick

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::events::EventType;

// =============================================================================
// Rule Identification
// =============================================================================

/// IFAB Law 기반 규칙 ID
///
/// 이벤트에 어떤 규칙이 적용되었는지 식별합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleId {
    // Law 7: Duration of Match
    /// 경기 시간
    Duration,

    // Law 8: Start and Restart of Play
    /// 킥오프
    KickOff,

    // Law 9: Ball In and Out of Play
    /// 볼 인플레이/아웃오브플레이
    BallInOut,

    // Law 10: Determining the Outcome
    /// 골 판정
    Goal,

    // Law 11: Offside
    /// 오프사이드 위치에서 플레이 관여
    OffsidePosition,
    /// 오프사이드 위치에서 상대방 방해
    OffsideInterferingWithOpponent,
    /// 오프사이드 위치에서 이익 획득
    OffsideGainingAdvantage,

    // Law 12: Fouls - Severity levels
    /// 부주의한 파울 (직접 프리킥)
    FoulCareless,
    /// 무모한 파울 (옐로카드)
    FoulReckless,
    /// 과도한 힘 사용 (레드카드)
    FoulExcessiveForce,

    // Law 12: Specific offences
    /// 명백한 득점 기회 저지 (DOGSO)
    Dogso,
    /// 심각한 반칙
    SeriousFoulPlay,
    /// 난폭 행위
    ViolentConduct,
    /// 핸드볼
    Handball,
    /// 시뮬레이션
    Simulation,

    // Law 13: Free Kicks
    /// 직접 프리킥
    DirectFreeKick,
    /// 간접 프리킥
    IndirectFreeKick,

    // Law 14: Penalty Kick
    /// 페널티킥
    PenaltyKick,

    // Law 15: Throw-In
    /// 스로인
    ThrowIn,

    // Law 16: Goal Kick
    /// 골킥
    GoalKick,

    // Law 17: Corner Kick
    /// 코너킥
    CornerKick,
}

impl RuleId {
    /// IFAB Law 번호 반환
    pub fn law_number(&self) -> u8 {
        match self {
            RuleId::Duration => 7,
            RuleId::KickOff => 8,
            RuleId::BallInOut => 9,
            RuleId::Goal => 10,

            RuleId::OffsidePosition
            | RuleId::OffsideInterferingWithOpponent
            | RuleId::OffsideGainingAdvantage => 11,

            RuleId::FoulCareless
            | RuleId::FoulReckless
            | RuleId::FoulExcessiveForce
            | RuleId::Dogso
            | RuleId::SeriousFoulPlay
            | RuleId::ViolentConduct
            | RuleId::Handball
            | RuleId::Simulation => 12,

            RuleId::DirectFreeKick | RuleId::IndirectFreeKick => 13,
            RuleId::PenaltyKick => 14,
            RuleId::ThrowIn => 15,
            RuleId::GoalKick => 16,
            RuleId::CornerKick => 17,
        }
    }

    /// 규칙 이름 (한국어)
    pub fn name_ko(&self) -> &'static str {
        match self {
            RuleId::Duration => "경기 시간",
            RuleId::KickOff => "킥오프",
            RuleId::BallInOut => "볼 인/아웃",
            RuleId::Goal => "골 판정",
            RuleId::OffsidePosition => "오프사이드 위치",
            RuleId::OffsideInterferingWithOpponent => "오프사이드 - 상대방 방해",
            RuleId::OffsideGainingAdvantage => "오프사이드 - 이익 획득",
            RuleId::FoulCareless => "부주의한 파울",
            RuleId::FoulReckless => "무모한 파울",
            RuleId::FoulExcessiveForce => "과도한 힘",
            RuleId::Dogso => "명백한 득점 기회 저지",
            RuleId::SeriousFoulPlay => "심각한 반칙",
            RuleId::ViolentConduct => "난폭 행위",
            RuleId::Handball => "핸드볼",
            RuleId::Simulation => "시뮬레이션",
            RuleId::DirectFreeKick => "직접 프리킥",
            RuleId::IndirectFreeKick => "간접 프리킥",
            RuleId::PenaltyKick => "페널티킥",
            RuleId::ThrowIn => "스로인",
            RuleId::GoalKick => "골킥",
            RuleId::CornerKick => "코너킥",
        }
    }

    /// 규칙 이름 (영어)
    pub fn name_en(&self) -> &'static str {
        match self {
            RuleId::Duration => "Duration of the Match",
            RuleId::KickOff => "Kick-off",
            RuleId::BallInOut => "Ball In and Out of Play",
            RuleId::Goal => "Goal",
            RuleId::OffsidePosition => "Offside position",
            RuleId::OffsideInterferingWithOpponent => "Offside - Interfering with opponent",
            RuleId::OffsideGainingAdvantage => "Offside - Gaining advantage",
            RuleId::FoulCareless => "Careless foul",
            RuleId::FoulReckless => "Reckless foul",
            RuleId::FoulExcessiveForce => "Excessive force",
            RuleId::Dogso => "Denying obvious goal-scoring opportunity",
            RuleId::SeriousFoulPlay => "Serious foul play",
            RuleId::ViolentConduct => "Violent conduct",
            RuleId::Handball => "Handball",
            RuleId::Simulation => "Simulation",
            RuleId::DirectFreeKick => "Direct free kick",
            RuleId::IndirectFreeKick => "Indirect free kick",
            RuleId::PenaltyKick => "Penalty kick",
            RuleId::ThrowIn => "Throw-in",
            RuleId::GoalKick => "Goal kick",
            RuleId::CornerKick => "Corner kick",
        }
    }

    /// EventType에서 RuleId로 매핑
    ///
    /// "왜?" 버튼 표시를 위한 규칙 매핑.
    /// 매핑되는 규칙이 없으면 None 반환.
    pub fn from_event_type(event_type: &EventType) -> Option<Self> {
        match event_type {
            EventType::KickOff => Some(RuleId::KickOff),
            EventType::HalfTime | EventType::FullTime => Some(RuleId::Duration),
            EventType::Offside => Some(RuleId::OffsidePosition),
            EventType::Foul => Some(RuleId::FoulCareless), // 기본값, FoulDetails에서 세분화
            EventType::Handball => Some(RuleId::Handball), // FIX_2601/0123 Phase 6
            EventType::YellowCard => Some(RuleId::FoulReckless),
            EventType::RedCard => Some(RuleId::FoulExcessiveForce),
            EventType::ThrowIn => Some(RuleId::ThrowIn),
            EventType::GoalKick => Some(RuleId::GoalKick),
            EventType::Corner => Some(RuleId::CornerKick),
            EventType::Freekick => Some(RuleId::DirectFreeKick), // 기본값
            EventType::Penalty => Some(RuleId::PenaltyKick),
            EventType::Goal | EventType::OwnGoal => Some(RuleId::Goal),
            EventType::Shot
            | EventType::ShotOnTarget
            | EventType::ShotOffTarget
            | EventType::ShotBlocked
            | EventType::Save
            | EventType::PostHit
            | EventType::BarHit => Some(RuleId::Goal),
            // 규칙 매핑 없음
            EventType::Pass
            | EventType::Tackle
            | EventType::Dribble
            | EventType::KeyChance
            | EventType::Substitution
            | EventType::Injury
            | EventType::VarReview => None,
        }
    }

    /// "왜?" 버튼을 표시할지 여부
    pub fn should_show_why_button(event_type: &EventType) -> bool {
        matches!(
            event_type,
            EventType::Offside
                | EventType::Foul
                | EventType::YellowCard
                | EventType::RedCard
                | EventType::ThrowIn
                | EventType::GoalKick
                | EventType::Corner
                | EventType::Freekick
                | EventType::Penalty
                | EventType::Goal
                | EventType::OwnGoal
                | EventType::PostHit
                | EventType::BarHit
                | EventType::VarReview
        )
    }
}

/// FromStr implementation for parsing RuleId from YAML strings
///
/// Supports both SCREAMING_SNAKE_CASE and LAW_XX_XXX formats.
impl FromStr for RuleId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // Law 7: Duration
            "DURATION" | "LAW_07_DURATION" => Ok(RuleId::Duration),

            // Law 8: Start and Restart
            "KICK_OFF" | "LAW_08_START_RESTART" => Ok(RuleId::KickOff),

            // Law 9: Ball In/Out
            "BALL_IN_OUT" | "LAW_09_BALL_IN_OUT" => Ok(RuleId::BallInOut),

            // Law 10: Goal
            "GOAL" | "LAW_10_GOAL" => Ok(RuleId::Goal),

            // Law 11: Offside
            "OFFSIDE_POSITION" | "LAW_11_OFFSIDE" => Ok(RuleId::OffsidePosition),
            "OFFSIDE_INTERFERING_WITH_OPPONENT" => Ok(RuleId::OffsideInterferingWithOpponent),
            "OFFSIDE_GAINING_ADVANTAGE" => Ok(RuleId::OffsideGainingAdvantage),

            // Law 12: Fouls - Severity levels
            "FOUL_CARELESS" | "LAW_12_FOULS_MISCONDUCT" | "LAW_12_FOULS" => Ok(RuleId::FoulCareless),
            "FOUL_RECKLESS" => Ok(RuleId::FoulReckless),
            "FOUL_EXCESSIVE_FORCE" => Ok(RuleId::FoulExcessiveForce),

            // Law 12: Specific offences
            "DOGSO" => Ok(RuleId::Dogso),
            "SERIOUS_FOUL_PLAY" => Ok(RuleId::SeriousFoulPlay),
            "VIOLENT_CONDUCT" => Ok(RuleId::ViolentConduct),
            "HANDBALL" => Ok(RuleId::Handball),
            "SIMULATION" => Ok(RuleId::Simulation),

            // Law 13: Free Kicks
            "DIRECT_FREE_KICK" | "LAW_13_FREE_KICK" => Ok(RuleId::DirectFreeKick),
            "INDIRECT_FREE_KICK" => Ok(RuleId::IndirectFreeKick),

            // Law 14: Penalty Kick
            "PENALTY_KICK" | "LAW_14_PENALTY" => Ok(RuleId::PenaltyKick),

            // Law 15: Throw-In
            "THROW_IN" | "LAW_15_THROW_IN" => Ok(RuleId::ThrowIn),

            // Law 16: Goal Kick
            "GOAL_KICK" | "LAW_16_GOAL_KICK" => Ok(RuleId::GoalKick),

            // Law 17: Corner Kick
            "CORNER_KICK" | "LAW_17_CORNER_KICK" => Ok(RuleId::CornerKick),

            // Law 3: Players (Substitution fallback)
            "LAW_03_PLAYERS" => Ok(RuleId::Duration),

            _ => Err(format!("Unknown RuleId: {}", s)),
        }
    }
}

// =============================================================================
// Restart Types (Law 8, 13-17)
// =============================================================================

/// 재시작 유형
///
/// 오프사이드 예외 판단 및 재시작 설명에 사용됩니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RestartType {
    /// 일반 플레이 중
    #[default]
    Normal,
    /// 킥오프
    KickOff,
    /// 골킥 (Law 16) - 오프사이드 예외
    GoalKick,
    /// 스로인 (Law 15) - 오프사이드 예외
    ThrowIn,
    /// 코너킥 (Law 17) - 오프사이드 예외
    CornerKick,
    /// 프리킥 (Law 13)
    FreeKick,
    /// 페널티킥 (Law 14)
    Penalty,
    /// 드롭볼
    DropBall,
}

impl RestartType {
    /// 오프사이드 예외가 적용되는지 여부 (Law 11)
    ///
    /// 골킥, 스로인, 코너킥에서 직접 받은 경우 오프사이드 예외
    pub fn is_offside_exception(&self) -> bool {
        matches!(self, RestartType::GoalKick | RestartType::ThrowIn | RestartType::CornerKick)
    }

    /// 재시작 유형 이름 (한국어)
    pub fn name_ko(&self) -> &'static str {
        match self {
            RestartType::Normal => "일반 플레이",
            RestartType::KickOff => "킥오프",
            RestartType::GoalKick => "골킥",
            RestartType::ThrowIn => "스로인",
            RestartType::CornerKick => "코너킥",
            RestartType::FreeKick => "프리킥",
            RestartType::Penalty => "페널티킥",
            RestartType::DropBall => "드롭볼",
        }
    }

    /// 재시작 유형 이름 (영어)
    pub fn name_en(&self) -> &'static str {
        match self {
            RestartType::Normal => "Normal play",
            RestartType::KickOff => "Kick-off",
            RestartType::GoalKick => "Goal kick",
            RestartType::ThrowIn => "Throw-in",
            RestartType::CornerKick => "Corner kick",
            RestartType::FreeKick => "Free kick",
            RestartType::Penalty => "Penalty kick",
            RestartType::DropBall => "Drop ball",
        }
    }
}

// =============================================================================
// Law 11: Offside
// =============================================================================

/// 오프사이드 상세 정보
///
/// 오프사이드 판정의 근거를 설명하기 위한 데이터.
/// "Why?" 질문에 답할 수 있도록 마진, 라인 위치 등을 포함합니다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OffsideDetails {
    /// 오프사이드 마진 (미터 단위)
    /// 양수: 오프사이드, 음수: 온사이드
    pub margin_m: f32,

    /// 오프사이드 라인 위치 (미터 단위, 골라인 기준)
    /// 수비 두 번째 선수의 위치
    pub offside_line_m: f32,

    /// 패서의 track_id (누가 볼을 플레이했는지)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passer_track_id: Option<u8>,

    /// 오프사이드 관여 유형
    #[serde(skip_serializing_if = "Option::is_none")]
    pub involvement_type: Option<OffsideInvolvementType>,

    /// 재시작 컨텍스트 (오프사이드 예외 판단용)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_context: Option<OffsideRestartContext>,

    /// 터치 기준점 (GK throw 처리용 - IFAB 25/26)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touch_reference: Option<TouchReference>,

    /// 굴절/세이브 컨텍스트 (deliberate play 판단용)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deflection_context: Option<DeflectionContext>,
}

/// 오프사이드 재시작 컨텍스트
///
/// 직전 재시작 유형과 예외 적용 여부를 추적합니다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffsideRestartContext {
    /// 직전 재시작 유형
    pub restart_type: RestartType,

    /// 오프사이드 예외 적용 여부
    /// (골킥/스로인/코너킥에서 직접 받은 경우 true)
    #[serde(default)]
    pub offside_exception_applies: bool,
}

impl OffsideRestartContext {
    /// 새로운 재시작 컨텍스트 생성
    pub fn new(restart_type: RestartType) -> Self {
        let offside_exception_applies = restart_type.is_offside_exception();
        Self {
            restart_type,
            offside_exception_applies,
        }
    }
}

/// 터치 유형 (오프사이드 판정 기준점)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TouchType {
    /// 킥
    #[default]
    Kick,
    /// 헤더
    Header,
    /// 골키퍼 스로
    GkThrow,
}

/// 터치 기준점 (오프사이드 판정 시점)
///
/// IFAB 25/26: GK throw는 last_contact 기준
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReferencePoint {
    /// 첫 접촉 시점
    #[default]
    FirstContact,
    /// 마지막 접촉 시점 (GK throw에 사용)
    LastContact,
}

/// 터치 기준점 정보
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchReference {
    /// 터치 유형
    pub touch_type: TouchType,

    /// 판정 기준점
    pub reference_point: ReferencePoint,
}

impl TouchReference {
    /// 새로운 터치 기준점 생성
    pub fn new(touch_type: TouchType) -> Self {
        let reference_point = match touch_type {
            // GK throw는 last contact 기준 (IFAB 25/26 변경점)
            TouchType::GkThrow => ReferencePoint::LastContact,
            // 나머지는 first contact 기준
            _ => ReferencePoint::FirstContact,
        };
        Self {
            touch_type,
            reference_point,
        }
    }
}

/// 수비수 마지막 터치 유형 (deliberate play 판단)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DefenderTouchType {
    /// 터치 없음
    #[default]
    None,
    /// 의도적 플레이 → 오프사이드 리셋
    DeliberatePlay,
    /// 단순 굴절 → 오프사이드 리셋 안 됨
    Deflection,
    /// 세이브 → 오프사이드 리셋 안 됨
    Save,
}

impl DefenderTouchType {
    /// 오프사이드 리셋 여부
    ///
    /// deliberate play만 오프사이드를 리셋합니다.
    /// deflection/save는 리셋하지 않습니다.
    pub fn resets_offside(&self) -> bool {
        matches!(self, DefenderTouchType::DeliberatePlay)
    }

    /// 터치 유형 이름 (한국어)
    pub fn name_ko(&self) -> &'static str {
        match self {
            DefenderTouchType::None => "없음",
            DefenderTouchType::DeliberatePlay => "의도적 플레이",
            DefenderTouchType::Deflection => "굴절",
            DefenderTouchType::Save => "세이브",
        }
    }

    /// 터치 유형 이름 (영어)
    pub fn name_en(&self) -> &'static str {
        match self {
            DefenderTouchType::None => "None",
            DefenderTouchType::DeliberatePlay => "Deliberate play",
            DefenderTouchType::Deflection => "Deflection",
            DefenderTouchType::Save => "Save",
        }
    }
}

/// 굴절/세이브 컨텍스트
///
/// 수비수의 마지막 터치 유형과 오프사이드 리셋 여부를 추적합니다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeflectionContext {
    /// 수비수의 마지막 터치 유형
    pub last_touch_by_defender: DefenderTouchType,

    /// 오프사이드 리셋 여부
    #[serde(default)]
    pub resets_offside: bool,
}

impl DeflectionContext {
    /// 새로운 굴절 컨텍스트 생성
    pub fn new(last_touch_by_defender: DefenderTouchType) -> Self {
        let resets_offside = last_touch_by_defender.resets_offside();
        Self {
            last_touch_by_defender,
            resets_offside,
        }
    }
}

/// 오프사이드 관여 유형 (Law 11)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OffsideInvolvementType {
    /// 플레이 관여 - 볼을 만지거나 플레이하려고 함
    InterferingWithPlay,
    /// 상대방 방해 - 상대방의 볼 플레이를 방해
    InterferingWithOpponent,
    /// 이익 획득 - 골대나 상대방에 맞고 나온 볼을 플레이
    GainingAdvantage,
}

impl OffsideInvolvementType {
    /// 관여 유형 이름 (한국어)
    pub fn name_ko(&self) -> &'static str {
        match self {
            OffsideInvolvementType::InterferingWithPlay => "플레이 관여",
            OffsideInvolvementType::InterferingWithOpponent => "상대방 방해",
            OffsideInvolvementType::GainingAdvantage => "이익 획득",
        }
    }

    /// 관여 유형 이름 (영어)
    pub fn name_en(&self) -> &'static str {
        match self {
            OffsideInvolvementType::InterferingWithPlay => "Interfering with play",
            OffsideInvolvementType::InterferingWithOpponent => "Interfering with opponent",
            OffsideInvolvementType::GainingAdvantage => "Gaining advantage",
        }
    }
}

// =============================================================================
// Law 12: Fouls and Misconduct
// =============================================================================

/// 파울 심각도 (Law 12)
///
/// IFAB 기준 3단계 심각도:
/// 1. Careless - 직접 프리킥
/// 2. Reckless - 옐로카드
/// 3. Excessive Force - 레드카드
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u8)]
pub enum FoulSeverity {
    /// 부주의 - 직접 프리킥만
    Careless = 1,
    /// 무모함 - 옐로카드
    Reckless = 2,
    /// 과도한 힘 - 레드카드
    ExcessiveForce = 3,
}

impl FoulSeverity {
    /// 심각도 레벨 (1-3)
    pub fn level(&self) -> u8 {
        *self as u8
    }

    /// 심각도 이름 (한국어)
    pub fn name_ko(&self) -> &'static str {
        match self {
            FoulSeverity::Careless => "부주의",
            FoulSeverity::Reckless => "무모함",
            FoulSeverity::ExcessiveForce => "과도한 힘",
        }
    }

    /// 심각도 이름 (영어)
    pub fn name_en(&self) -> &'static str {
        match self {
            FoulSeverity::Careless => "Careless",
            FoulSeverity::Reckless => "Reckless",
            FoulSeverity::ExcessiveForce => "Excessive force",
        }
    }

    /// 예상 제재
    pub fn expected_sanction(&self) -> FoulSanction {
        match self {
            FoulSeverity::Careless => FoulSanction::DirectFreeKick,
            FoulSeverity::Reckless => FoulSanction::YellowCard,
            FoulSeverity::ExcessiveForce => FoulSanction::RedCard,
        }
    }
}

/// 파울에 대한 제재
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FoulSanction {
    /// 직접 프리킥만
    DirectFreeKick,
    /// 간접 프리킥
    IndirectFreeKick,
    /// 옐로카드 + 직접 프리킥
    YellowCard,
    /// 레드카드 + 직접 프리킥
    RedCard,
    /// 옐로카드 + 페널티킥 (페널티 에어리어 내 DOGSO 감경)
    YellowCardAndPenalty,
    /// 레드카드 + 페널티킥
    RedCardAndPenalty,
}

impl FoulSanction {
    /// 제재 이름 (한국어)
    pub fn name_ko(&self) -> &'static str {
        match self {
            FoulSanction::DirectFreeKick => "직접 프리킥",
            FoulSanction::IndirectFreeKick => "간접 프리킥",
            FoulSanction::YellowCard => "옐로카드",
            FoulSanction::RedCard => "레드카드",
            FoulSanction::YellowCardAndPenalty => "옐로카드 + 페널티킥",
            FoulSanction::RedCardAndPenalty => "레드카드 + 페널티킥",
        }
    }

    /// 제재 이름 (영어)
    pub fn name_en(&self) -> &'static str {
        match self {
            FoulSanction::DirectFreeKick => "Direct free kick",
            FoulSanction::IndirectFreeKick => "Indirect free kick",
            FoulSanction::YellowCard => "Yellow card",
            FoulSanction::RedCard => "Red card",
            FoulSanction::YellowCardAndPenalty => "Yellow card + Penalty kick",
            FoulSanction::RedCardAndPenalty => "Red card + Penalty kick",
        }
    }

    /// 카드가 필요한지 여부
    pub fn requires_card(&self) -> bool {
        matches!(
            self,
            FoulSanction::YellowCard
                | FoulSanction::RedCard
                | FoulSanction::YellowCardAndPenalty
                | FoulSanction::RedCardAndPenalty
        )
    }

    /// 레드카드 여부
    pub fn is_red_card(&self) -> bool {
        matches!(self, FoulSanction::RedCard | FoulSanction::RedCardAndPenalty)
    }
}

/// 파울 유형 (직접 프리킥 파울)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FoulType {
    /// 차징 - 상대방에게 돌진
    Charging,
    /// 점핑 - 상대방을 향해 뛰어오름
    Jumping,
    /// 킥 - 상대방을 차거나 차려고 함
    Kicking,
    /// 푸싱 - 상대방을 밈
    Pushing,
    /// 가격 - 상대방을 치거나 치려고 함
    Striking,
    /// 태클 - 볼보다 먼저 상대방에 접촉
    Tackling,
    /// 트리핑 - 상대방을 넘어뜨림
    Tripping,
    /// 핸드볼 - 손이나 팔로 볼을 만짐
    Handball,
    /// 홀딩 - 상대방을 붙잡음
    Holding,
    /// 방해 - 볼 없이 상대방 진로 방해
    Impeding,
}

impl FoulType {
    /// 파울 유형 이름 (한국어)
    pub fn name_ko(&self) -> &'static str {
        match self {
            FoulType::Charging => "차징",
            FoulType::Jumping => "점핑",
            FoulType::Kicking => "킥",
            FoulType::Pushing => "푸싱",
            FoulType::Striking => "가격",
            FoulType::Tackling => "태클",
            FoulType::Tripping => "트리핑",
            FoulType::Handball => "핸드볼",
            FoulType::Holding => "홀딩",
            FoulType::Impeding => "방해",
        }
    }

    /// 파울 유형 이름 (영어)
    pub fn name_en(&self) -> &'static str {
        match self {
            FoulType::Charging => "Charging",
            FoulType::Jumping => "Jumping at",
            FoulType::Kicking => "Kicking",
            FoulType::Pushing => "Pushing",
            FoulType::Striking => "Striking",
            FoulType::Tackling => "Tackling",
            FoulType::Tripping => "Tripping",
            FoulType::Handball => "Handball",
            FoulType::Holding => "Holding",
            FoulType::Impeding => "Impeding",
        }
    }
}

/// 파울 상세 정보
///
/// 파울 이벤트의 상세 정보를 담습니다.
/// 심각도, 파울 유형, DOGSO 여부 등을 포함합니다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoulDetails {
    /// 파울 심각도
    pub severity: FoulSeverity,

    /// 파울 유형
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foul_type: Option<FoulType>,

    /// DOGSO (명백한 득점 기회 저지) 여부
    #[serde(default)]
    pub is_dogso: bool,

    /// 페널티 에어리어 내 파울 여부
    #[serde(default)]
    pub in_penalty_area: bool,

    /// 피해자 track_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub victim_track_id: Option<u8>,

    /// 볼 플레이 시도 여부 (DOGSO 감경 판단용)
    #[serde(default)]
    pub attempted_to_play_ball: bool,
}

impl FoulDetails {
    /// 예상 제재 계산
    ///
    /// DOGSO 규정 및 페널티 에어리어 감경을 고려합니다.
    pub fn expected_sanction(&self) -> FoulSanction {
        // DOGSO + 페널티 에어리어 + 볼 플레이 시도 = 옐로카드 감경
        if self.is_dogso && self.in_penalty_area && self.attempted_to_play_ball {
            return FoulSanction::YellowCardAndPenalty;
        }

        // DOGSO는 레드카드 (페널티 에어리어 내에서도 볼 플레이 시도 안 하면)
        if self.is_dogso {
            if self.in_penalty_area {
                return FoulSanction::RedCardAndPenalty;
            }
            return FoulSanction::RedCard;
        }

        // 일반 파울: 심각도에 따른 제재
        let base_sanction = self.severity.expected_sanction();

        // 페널티 에어리어 내 파울은 페널티킥
        if self.in_penalty_area {
            match base_sanction {
                FoulSanction::DirectFreeKick => FoulSanction::DirectFreeKick, // 페널티킥으로 변환은 별도 처리
                FoulSanction::YellowCard => FoulSanction::YellowCardAndPenalty,
                FoulSanction::RedCard => FoulSanction::RedCardAndPenalty,
                other => other,
            }
        } else {
            base_sanction
        }
    }
}

// =============================================================================
// YAML Data Structures (for serde deserialization)
// =============================================================================

/// Law 11 YAML 데이터 구조
#[derive(Debug, Clone, Deserialize)]
pub struct OffsideRuleData {
    pub rule_id: String,
    pub law_number: u8,
    pub name: String,
    pub name_en: String,
    pub summary: String,
    pub summary_en: String,
    #[serde(default)]
    pub offside_position: Option<OffsidePositionData>,
    #[serde(default)]
    pub offside_offence: Option<OffsideOffenceData>,
    #[serde(default)]
    pub no_offside: Option<NoOffsideData>,
    #[serde(default)]
    pub explanation_templates: Option<ExplanationTemplates>,
    #[serde(default)]
    pub restart: Option<RestartInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OffsidePositionData {
    pub description: String,
    pub description_en: String,
    #[serde(default)]
    pub conditions: Vec<ConditionData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConditionData {
    pub id: String,
    pub name: String,
    pub name_en: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub description_en: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OffsideOffenceData {
    pub description: String,
    pub description_en: String,
    #[serde(default)]
    pub involvement_types: Vec<ConditionData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NoOffsideData {
    pub description: String,
    pub description_en: String,
    #[serde(default)]
    pub exceptions: Vec<ConditionData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExplanationTemplates {
    #[serde(default)]
    pub basic: Option<TemplateData>,
    #[serde(default)]
    pub detailed: Option<TemplateData>,
    #[serde(default)]
    pub why_offside: Option<TemplateData>,
    #[serde(default)]
    pub foul_basic: Option<TemplateData>,
    #[serde(default)]
    pub foul_with_severity: Option<TemplateData>,
    #[serde(default)]
    pub yellow_card: Option<TemplateData>,
    #[serde(default)]
    pub red_card: Option<TemplateData>,
    #[serde(default)]
    pub dogso: Option<TemplateData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateData {
    pub template: String,
    pub template_en: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RestartInfo {
    #[serde(rename = "type")]
    pub restart_type: Option<String>,
    pub name: Option<String>,
    pub name_en: Option<String>,
    pub position: Option<String>,
    pub position_en: Option<String>,
    // For law_12 multiple restart types
    #[serde(default)]
    pub direct_free_kick: Option<RestartTypeInfo>,
    #[serde(default)]
    pub indirect_free_kick: Option<RestartTypeInfo>,
    #[serde(default)]
    pub penalty_kick: Option<RestartTypeInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RestartTypeInfo {
    #[serde(rename = "type")]
    pub restart_type: String,
    pub name: String,
    pub name_en: String,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub condition_en: Option<String>,
}

/// Law 12 YAML 데이터 구조
#[derive(Debug, Clone, Deserialize)]
pub struct FoulsRuleData {
    pub rule_id: String,
    pub law_number: u8,
    pub name: String,
    pub name_en: String,
    pub summary: String,
    pub summary_en: String,
    #[serde(default)]
    pub foul_severity: Vec<FoulSeverityData>,
    #[serde(default)]
    pub direct_free_kick_offences: Option<OffencesData>,
    #[serde(default)]
    pub yellow_card_offences: Option<OffencesData>,
    #[serde(default)]
    pub red_card_offences: Option<OffencesData>,
    #[serde(default)]
    pub dogso_criteria: Option<DogsoCriteriaData>,
    #[serde(default)]
    pub dogso_in_penalty_area: Option<DogsoPenaltyAreaData>,
    #[serde(default)]
    pub explanation_templates: Option<ExplanationTemplates>,
    #[serde(default)]
    pub restart: Option<RestartInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FoulSeverityData {
    pub id: String,
    pub level: u8,
    pub name: String,
    pub name_en: String,
    pub description: String,
    pub description_en: String,
    pub sanction: String,
    pub card: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OffencesData {
    pub description: String,
    pub description_en: String,
    #[serde(default)]
    pub offences: Vec<OffenceData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OffenceData {
    pub id: String,
    pub name: String,
    pub name_en: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub description_en: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DogsoCriteriaData {
    pub name: String,
    pub name_en: String,
    #[serde(default)]
    pub factors: Vec<ConditionData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DogsoPenaltyAreaData {
    pub name: String,
    pub name_en: String,
    pub description: String,
    pub description_en: String,
    pub sanction: String,
    pub condition: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_id_law_number() {
        assert_eq!(RuleId::OffsidePosition.law_number(), 11);
        assert_eq!(RuleId::FoulCareless.law_number(), 12);
        assert_eq!(RuleId::Dogso.law_number(), 12);
    }

    #[test]
    fn test_foul_severity_ordering() {
        assert!(FoulSeverity::Careless < FoulSeverity::Reckless);
        assert!(FoulSeverity::Reckless < FoulSeverity::ExcessiveForce);
    }

    #[test]
    fn test_foul_severity_sanction() {
        assert_eq!(
            FoulSeverity::Careless.expected_sanction(),
            FoulSanction::DirectFreeKick
        );
        assert_eq!(
            FoulSeverity::Reckless.expected_sanction(),
            FoulSanction::YellowCard
        );
        assert_eq!(
            FoulSeverity::ExcessiveForce.expected_sanction(),
            FoulSanction::RedCard
        );
    }

    #[test]
    fn test_dogso_in_penalty_area_reduction() {
        let foul = FoulDetails {
            severity: FoulSeverity::Careless,
            foul_type: Some(FoulType::Tackling),
            is_dogso: true,
            in_penalty_area: true,
            victim_track_id: Some(10),
            attempted_to_play_ball: true,
        };
        // DOGSO + penalty area + attempted ball = yellow card reduction
        assert_eq!(foul.expected_sanction(), FoulSanction::YellowCardAndPenalty);
    }

    #[test]
    fn test_dogso_no_ball_attempt_is_red() {
        let foul = FoulDetails {
            severity: FoulSeverity::Careless,
            foul_type: Some(FoulType::Holding),
            is_dogso: true,
            in_penalty_area: true,
            victim_track_id: Some(10),
            attempted_to_play_ball: false,
        };
        // DOGSO without ball attempt = red card
        assert_eq!(foul.expected_sanction(), FoulSanction::RedCardAndPenalty);
    }
}
