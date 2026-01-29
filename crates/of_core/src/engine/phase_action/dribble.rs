//! Dribble Action FSM
//!
//! P7 Spec Section 2.3, 8: Dribble Phase, Ball-Player Separation
//! P7-DRIBBLE-TUNE: Intent → Technique → Physics 시스템
//!
//! ## 핵심 원칙: Intent → Technique → Physics
//! ```text
//! 상황 인식 → Intent 결정 (왜?) → Technique 선택 (어떻게?) → Physics 실행
//! ```
//!
//! ## Intent (3가지)
//! - Protect: 공을 지키고 싶다 (압박, 시간 벌기)
//! - Progress: 공간으로 전진하고 싶다 (패스 타겟 없음)
//! - Beat: 수비수를 제치고 싶다 (1:1 돌파)
//!
//! ## Technique (6가지 핵심 기술)
//! - Shielding (등지기): strength + balance
//! - Turn (터닝): dribbling + agility + balance
//! - FaceUp (마주보기): technique + first_touch
//! - KnockOn (치달): acceleration + pace
//! - Feint (속임수): dribbling + flair + technique
//! - Hesitation (스탑앤고): composure + flair + acceleration
//!
//! ## Phase Flow (조건 기반 전이)
//! ```text
//! Touch → Carry → SyncBall → Touch → ...  (기본 사이클)
//!           ↓
//!         Evade ←── (압박 + Beat intent)
//!           ↓
//!     KnockAndRun ←── (치달 조건: 공간 + Progress intent)
//! ```
//!
//! ## 조건 기반 전이 원칙
//! - `remaining_ticks`는 삭제 (타이머 카운트다운 제거)
//! - 각 상태는 **Exit 조건**으로 빠져나감
//! - **MaxTicks(상한)**만 가드레일로 유지 (무한 방지)
//! - Carry/KnockOn은 0~N틱 가변 (조건 충족 시 즉시 전이)

use super::action_common::{
    clamp01, lerp as common_lerp, skill01, weighted_choice_index, ActionModel,
};
use super::duration::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
// P0: Core types moved to action_queue
use super::super::action_queue::{DribbleTouchEvent, DribbleTouchType, TakeOnEvent, TakeOnOutcome};
use crate::models::player::PlayerAttributes;

// ============================================================================
// P7-DRIBBLE-TUNE: Intent → Technique System
// ============================================================================

/// 드리블 의도 (Intent) - "왜" 드리블하는가?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DribbleIntent {
    /// 공을 지키고 싶다 (Protect)
    /// - 압박 받는 상황, 시간 벌기, 동료 기다리기
    Protect,

    /// 공간으로 전진하고 싶다 (Progress)
    /// - 앞에 공간 있음, 패스 타겟 없음
    #[default]
    Progress,

    /// 수비수를 제치고 싶다 (Beat)
    /// - 1:1 상황, 돌파 시도
    Beat,
}

impl DribbleIntent {
    /// 이 의도로 사용 가능한 기술들
    pub fn available_techniques(&self) -> &[DribbleTechnique] {
        match self {
            Self::Protect => &[DribbleTechnique::Shielding, DribbleTechnique::Turn],
            Self::Progress => &[DribbleTechnique::FaceUp, DribbleTechnique::KnockOn],
            Self::Beat => &[
                DribbleTechnique::Feint,
                DribbleTechnique::KnockOn,
                DribbleTechnique::Hesitation,
                DribbleTechnique::Turn,
            ],
        }
    }
}

/// 드리블 기술 (Technique) - "어떻게" 드리블하는가?
///
/// 농구 비유: Backdown, Spin, Face-Up, Drive, Crossover, Hesitation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DribbleTechnique {
    /// 1. Shielding (등지기) - 수비수를 등지고 공 보호
    /// 농구의 Backdown
    /// 핵심 스탯: strength, balance
    Shielding,

    /// 2. Turn (터닝) - 등진 상태에서 회전하여 탈압박
    /// 농구의 Spin Move
    /// 핵심 스탯: dribbling, agility, balance
    Turn,

    /// 3. FaceUp (마주보기) - 수비수와 간격 두고 대치
    /// 농구의 Face-Up (탐색 상태)
    /// 핵심 스탯: technique, first_touch
    FaceUp,

    /// 4. KnockOn (치달) - 공을 치고 달림
    /// 농구의 Drive
    /// 핵심 스탯: acceleration, pace
    KnockOn,

    /// 5. Feint (속임수) - 방향 페이크로 무게중심 뺏기
    /// 농구의 Crossover
    /// 핵심 스탯: dribbling, flair, technique
    Feint,

    /// 6. Hesitation (스탑앤고) - 템포 속임수
    /// 농구의 Hesitation
    /// 핵심 스탯: composure, flair, acceleration
    Hesitation,
}

impl DribbleTechnique {
    /// 이동 속도 (m/s) - P15 관성 시스템 연동
    pub fn base_speed(&self) -> f32 {
        match self {
            Self::Shielding => 0.0,  // 정지
            Self::Turn => 2.0,       // 저속 (회전 중)
            Self::FaceUp => 1.0,     // 거의 정지 (간 보기)
            Self::KnockOn => 9.0,    // 전력질주
            Self::Feint => 4.0,      // 중속
            Self::Hesitation => 0.5, // 거의 정지 (멈칫)
        }
    }

    /// 공과의 거리 (m)
    pub fn ball_distance(&self) -> f32 {
        match self {
            Self::Shielding => 0.2, // 발에 붙임
            Self::Turn => 0.3,
            Self::FaceUp => 1.0,  // 컨트롤 범위
            Self::KnockOn => 5.0, // 3~5m 앞으로 치고 달림
            Self::Feint => 0.5,
            Self::Hesitation => 0.3,
        }
    }

    /// 실패 시 공 잃을 확률 (base)
    pub fn base_loss_risk(&self) -> f32 {
        match self {
            Self::Shielding => 0.10,
            Self::Turn => 0.15,
            Self::FaceUp => 0.05,  // 안전
            Self::KnockOn => 0.40, // 고위험
            Self::Feint => 0.30,
            Self::Hesitation => 0.20,
        }
    }

    /// 소요 시간 (틱)
    pub fn duration_ticks(&self) -> u8 {
        match self {
            Self::Shielding => 2,
            Self::Turn => 3,
            Self::FaceUp => 1, // 탐색 상태 (짧음)
            Self::KnockOn => 5,
            Self::Feint => 4,
            Self::Hesitation => 3,
        }
    }

    /// 속임수 유형
    pub fn deception_type(&self) -> Option<DeceptionType> {
        match self {
            Self::Feint => Some(DeceptionType::Direction),
            Self::Turn => Some(DeceptionType::Direction),
            Self::Hesitation => Some(DeceptionType::Tempo),
            Self::KnockOn => Some(DeceptionType::Speed),
            _ => None,
        }
    }
}

/// 속임수 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeceptionType {
    /// 방향 속임수 (Feint, Turn)
    Direction,
    /// 시간/템포 속임수 (Hesitation)
    Tempo,
    /// 속도 속임수 (KnockOn)
    Speed,
}

// ============================================================================
// P7-DRIBBLE-TUNE: Technique Selection Logic
// ============================================================================

/// 드리블 기술 선택 결과
#[derive(Debug, Clone, Copy)]
pub struct TechniqueSelection {
    pub technique: DribbleTechnique,
    pub success_chance: f32, // 0.0 ~ 1.0
}

/// 능력치 기반 기술 선택 (Intent → Technique)
///
/// 핵심 로직:
/// - 공격자/수비자 능력치 비교
/// - Intent에 따라 가능한 기술 필터링
/// - 가장 높은 성공 확률 기술 선택
pub fn choose_dribble_technique(
    intent: DribbleIntent,
    attacker: &PlayerAttributes,
    defender: Option<&PlayerAttributes>,
    has_space_ahead: bool,
) -> TechniqueSelection {
    // 수비자 없으면 기본 수비 능력치 사용
    let default_defender = PlayerAttributes::default();
    let def = defender.unwrap_or(&default_defender);

    match intent {
        DribbleIntent::Protect => choose_protect_technique(attacker, def),
        DribbleIntent::Progress => choose_progress_technique(attacker, def, has_space_ahead),
        DribbleIntent::Beat => choose_beat_technique(attacker, def),
    }
}

/// Protect Intent: Shielding or Turn
fn choose_protect_technique(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
) -> TechniqueSelection {
    // 힘+밸런스 vs 힘+aggression
    let att_shield = attacker.strength as f32 + attacker.balance as f32;
    let def_shield = defender.strength as f32 + defender.aggression as f32;
    let shield_advantage = (att_shield - def_shield) / 100.0;
    let shield_chance = (0.5 + shield_advantage * 0.3).clamp(0.2, 0.9);

    // 드리블+민첩성+밸런스 vs 예측+태클
    let att_turn = attacker.dribbling as f32 + attacker.agility as f32 + attacker.balance as f32;
    let def_turn = defender.anticipation as f32 + defender.tackling as f32;
    let turn_advantage = (att_turn - def_turn) / 150.0;
    let turn_chance = (0.4 + turn_advantage * 0.3).clamp(0.15, 0.85);

    // 더 높은 성공률 기술 선택
    if shield_chance >= turn_chance {
        TechniqueSelection { technique: DribbleTechnique::Shielding, success_chance: shield_chance }
    } else {
        TechniqueSelection { technique: DribbleTechnique::Turn, success_chance: turn_chance }
    }
}

/// Progress Intent: FaceUp or KnockOn
fn choose_progress_technique(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
    has_space: bool,
) -> TechniqueSelection {
    // FaceUp은 항상 안전 (낮은 리스크)
    let faceup_chance = 0.85;

    if !has_space {
        return TechniqueSelection {
            technique: DribbleTechnique::FaceUp,
            success_chance: faceup_chance,
        };
    }

    // KnockOn: 가속+속도 vs 수비 가속+속도
    let att_speed = attacker.acceleration as f32 + attacker.pace as f32;
    let def_speed = defender.acceleration as f32 + defender.pace as f32;
    let speed_advantage = (att_speed - def_speed) / 100.0;
    let knockon_chance = (0.35 + speed_advantage * 0.4).clamp(0.1, 0.75);

    // 속도 우위가 충분하면 KnockOn, 아니면 FaceUp
    if knockon_chance > 0.5 && speed_advantage > 0.05 {
        TechniqueSelection { technique: DribbleTechnique::KnockOn, success_chance: knockon_chance }
    } else {
        TechniqueSelection { technique: DribbleTechnique::FaceUp, success_chance: faceup_chance }
    }
}

/// Beat Intent: Feint, KnockOn, Hesitation, or Turn
fn choose_beat_technique(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
) -> TechniqueSelection {
    // 각 기술별 성공 확률 계산

    // 1. KnockOn: 가속+속도 경주
    let att_speed = attacker.acceleration as f32 + attacker.pace as f32;
    let def_speed = defender.acceleration as f32 + defender.pace as f32;
    let knockon_chance = (0.35 + (att_speed - def_speed) / 100.0 * 0.4).clamp(0.1, 0.75);

    // 2. Feint: 드리블+플레어+기술 vs 예측+태클
    let att_feint = attacker.dribbling as f32 + attacker.flair as f32 + attacker.technique as f32;
    let def_feint = defender.anticipation as f32 + defender.tackling as f32;
    let feint_chance = (0.3 + (att_feint - def_feint) / 150.0 * 0.4).clamp(0.1, 0.7);

    // 3. Hesitation: 침착함+플레어+가속 vs 집중력+예측
    let att_hes = attacker.composure as f32 + attacker.flair as f32 + attacker.acceleration as f32;
    let def_hes = defender.concentration as f32 + defender.anticipation as f32;
    let hes_chance = (0.35 + (att_hes - def_hes) / 150.0 * 0.35).clamp(0.15, 0.7);

    // 4. Turn: 드리블+민첩성+밸런스 vs 예측+태클
    let att_turn = attacker.dribbling as f32 + attacker.agility as f32 + attacker.balance as f32;
    let def_turn = defender.anticipation as f32 + defender.tackling as f32;
    let turn_chance = (0.35 + (att_turn - def_turn) / 150.0 * 0.35).clamp(0.15, 0.75);

    // 최고 성공률 기술 선택
    let techniques = [
        (DribbleTechnique::KnockOn, knockon_chance),
        (DribbleTechnique::Feint, feint_chance),
        (DribbleTechnique::Hesitation, hes_chance),
        (DribbleTechnique::Turn, turn_chance),
    ];

    let (best_tech, best_chance) = techniques
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .copied()
        .unwrap_or((DribbleTechnique::Feint, 0.3));

    TechniqueSelection { technique: best_tech, success_chance: best_chance }
}

// ============================================================================
// P7-DRIBBLE-TUNE: Technique Resolve Results
// ============================================================================

/// 기술 실행 결과
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TechniqueResult {
    /// 성공 - 수비 제침
    Success {
        /// 생성된 공간 (m)
        space_gained: f32,
        /// 새 방향 (정규화 벡터)
        new_direction: (f32, f32),
    },
    /// 부분 성공 - 공 유지하지만 제치지 못함
    Partial,
    /// 실패 - 공 잃음
    Failed {
        /// 볼 위치
        ball_position: (f32, f32),
    },
}

/// 기술 실행 판정 (메인 resolve 함수)
///
/// # Arguments
/// - `technique`: 실행할 기술
/// - `attacker`: 공격자 능력치
/// - `defender`: 수비자 능력치 (없으면 None)
/// - `rng_roll`: 0.0~1.0 난수 (테스트용 주입)
/// - `ball_pos`: 현재 공 위치
/// - `dribble_dir`: 드리블 방향
///
/// # Returns
/// 기술 실행 결과
pub fn resolve_technique(
    technique: DribbleTechnique,
    attacker: &PlayerAttributes,
    defender: Option<&PlayerAttributes>,
    rng_roll: f32,
    ball_pos: (f32, f32),
    dribble_dir: (f32, f32),
) -> TechniqueResult {
    let default_def = PlayerAttributes::default();
    let def = defender.unwrap_or(&default_def);

    match technique {
        DribbleTechnique::Shielding => resolve_shielding(attacker, def, rng_roll),
        DribbleTechnique::Turn => resolve_turn(attacker, def, rng_roll, dribble_dir),
        DribbleTechnique::FaceUp => resolve_faceup(rng_roll),
        DribbleTechnique::KnockOn => {
            resolve_knockon(attacker, def, rng_roll, ball_pos, dribble_dir)
        }
        DribbleTechnique::Feint => resolve_feint(attacker, def, rng_roll, dribble_dir),
        DribbleTechnique::Hesitation => resolve_hesitation(attacker, def, rng_roll, dribble_dir),
    }
}

/// Shielding (등지기) 판정
/// 판정: (strength + balance) vs (strength + aggression)
fn resolve_shielding(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
    rng_roll: f32,
) -> TechniqueResult {
    let att_score = attacker.strength as f32 + attacker.balance as f32;
    let def_score = defender.strength as f32 + defender.aggression as f32;

    // 랜덤 변동 ±10점 (rng_roll 0~1 → -10~+10)
    let variance = (rng_roll - 0.5) * 20.0;

    if att_score > def_score + variance {
        // 성공: 공 유지, 수비수 밀려남
        TechniqueResult::Success {
            space_gained: 0.5,
            new_direction: (0.0, 0.0), // 등지기는 방향 변화 없음
        }
    } else if att_score > def_score + variance - 15.0 {
        // 부분 성공: 버팀
        TechniqueResult::Partial
    } else {
        // 실패: 밸런스 무너짐
        TechniqueResult::Failed {
            ball_position: (0.0, 0.0), // 호출자가 업데이트
        }
    }
}

/// Turn (터닝) 판정
/// 판정: (dribbling + agility + balance) vs (anticipation + tackling)
fn resolve_turn(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
    rng_roll: f32,
    current_dir: (f32, f32),
) -> TechniqueResult {
    let att_score = attacker.dribbling as f32 + attacker.agility as f32 + attacker.balance as f32;
    let def_score = defender.anticipation as f32 + defender.tackling as f32;

    let variance = (rng_roll - 0.5) * 30.0; // ±15점

    if att_score > def_score + variance {
        // 성공: 180도 회전, 수비수 등 뒤에 남김
        let new_dir = (-current_dir.0, -current_dir.1);
        TechniqueResult::Success { space_gained: 1.5, new_direction: new_dir }
    } else if att_score > def_score + variance - 20.0 {
        TechniqueResult::Partial
    } else {
        TechniqueResult::Failed { ball_position: (0.0, 0.0) }
    }
}

/// FaceUp (마주보기) 판정
/// 안전한 탐색 상태 - 거의 항상 성공
fn resolve_faceup(rng_roll: f32) -> TechniqueResult {
    // 5% 실패 확률 (극단적 상황)
    if rng_roll < 0.05 {
        TechniqueResult::Failed { ball_position: (0.0, 0.0) }
    } else {
        TechniqueResult::Success {
            space_gained: 0.0, // 공간 생성 없음 (간 보기 상태)
            new_direction: (0.0, 0.0),
        }
    }
}

/// KnockOn (치달) 판정
/// 판정: (acceleration + pace) 경주
fn resolve_knockon(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
    rng_roll: f32,
    ball_pos: (f32, f32),
    dribble_dir: (f32, f32),
) -> TechniqueResult {
    // 공격수: 이미 움직이는 중 (관성 보너스 +10)
    let att_score = attacker.acceleration as f32 + attacker.pace as f32 + 10.0;
    let def_score = defender.acceleration as f32 + defender.pace as f32;

    let variance = (rng_roll - 0.5) * 10.0; // ±5점

    if att_score > def_score + variance {
        // 성공: 공 치고 수비수보다 먼저 도달
        let knock_dist = 4.0 + rng_roll * 2.0; // 4~6m
        TechniqueResult::Success { space_gained: knock_dist, new_direction: dribble_dir }
    } else {
        // 실패: 수비수가 먼저 도달
        let lost_pos = (ball_pos.0 + dribble_dir.0 * 3.0, ball_pos.1 + dribble_dir.1 * 3.0);
        TechniqueResult::Failed { ball_position: lost_pos }
    }
}

/// Feint (속임수) 판정
/// 판정: (dribbling + flair + technique) vs (anticipation + concentration)
fn resolve_feint(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
    rng_roll: f32,
    dribble_dir: (f32, f32),
) -> TechniqueResult {
    let att_score = attacker.dribbling as f32 + attacker.flair as f32 + attacker.technique as f32;
    let def_score = defender.anticipation as f32 + defender.concentration as f32;

    let variance = (rng_roll - 0.5) * 30.0; // ±15점

    if att_score > def_score + variance {
        // 성공: 수비수 무게중심 무너짐 (Wrong-footed)
        // 페이크 반대 방향으로 탈출
        let escape_dir = perpendicular(dribble_dir, rng_roll > 0.5);
        TechniqueResult::Success { space_gained: 2.0, new_direction: escape_dir }
    } else if att_score > def_score + variance - 20.0 {
        TechniqueResult::Partial
    } else {
        TechniqueResult::Failed { ball_position: (0.0, 0.0) }
    }
}

/// Hesitation (스탑앤고) 판정
/// 판정: (composure + flair + acceleration) vs (concentration + anticipation)
fn resolve_hesitation(
    attacker: &PlayerAttributes,
    defender: &PlayerAttributes,
    rng_roll: f32,
    dribble_dir: (f32, f32),
) -> TechniqueResult {
    let att_score =
        attacker.composure as f32 + attacker.flair as f32 + attacker.acceleration as f32;
    let def_score = defender.concentration as f32 + defender.anticipation as f32;

    let variance = (rng_roll - 0.5) * 20.0; // ±10점

    if att_score > def_score + variance {
        // 성공: 수비수 멈춤, 공격수 폭발 가속
        TechniqueResult::Success { space_gained: 2.5, new_direction: dribble_dir }
    } else if att_score > def_score + variance - 15.0 {
        TechniqueResult::Partial
    } else {
        TechniqueResult::Failed { ball_position: (0.0, 0.0) }
    }
}

/// 수직 방향 계산 (페인트용)
#[inline]
fn perpendicular(dir: (f32, f32), left: bool) -> (f32, f32) {
    if left {
        (-dir.1, dir.0)
    } else {
        (dir.1, -dir.0)
    }
}

// ============================================================================
// Dribble Constants
// ============================================================================

/// 드리블 속도 (m/tick)
pub const DRIBBLE_SPEED: f32 = 1.5;

/// 회피 속도 (m/tick)
pub const EVADE_SPEED: f32 = 1.2;

/// 공 롤링 속도 배율 (선수 속도 대비)
pub const BALL_ROLL_SPEED_MULTIPLIER: f32 = 1.2;

/// 회피 거리 트리거 (m)
pub const EVADE_TRIGGER_DISTANCE: f32 = 2.0;

/// 회피 지속 틱 (삭제 예정 - 하위 호환용)
pub const EVADE_DURATION_TICKS: u8 = 4;

// ============================================================================
// 조건 기반 전이: MaxTicks (상한)
// ============================================================================

/// Gather 상한 (틱)
pub const MAX_GATHER_TICKS: u8 = 2;

/// Carry 상한 (틱) - "7틱"은 상한이지 고정값이 아님
pub const MAX_CARRY_TICKS: u8 = 7;

/// Evade 상한 (틱)
pub const MAX_EVADE_TICKS: u8 = 4;

/// KnockAndRun 상한 (틱) - 치달 최대 8틱
pub const MAX_KNOCKON_TICKS: u8 = 8;

/// 터치 준비 거리 (m) - 공이 이 거리 이내면 Touch 가능
pub const TOUCH_READY_DIST: f32 = 0.45;

/// 치달 후 공 회수 거리 (m)
pub const KNOCKON_REGAIN_DIST: f32 = 1.5;

/// 공간 부족 임계값 (0~1)
pub const SPACE_LOW_THRESHOLD: f32 = 0.35;

/// 인터셉트 위험 임계값 (0~1)
pub const INTERCEPT_RISK_HIGH: f32 = 0.6;

// ============================================================================
// DribbleObs - 관측값 (조건 기반 전이용)
// ============================================================================

/// 드리블 관측값 (매 틱 계산)
#[derive(Debug, Clone, Copy, Default)]
pub struct DribbleObs {
    /// 공-선수 거리 (m)
    pub sep: f32,
    /// 가장 가까운 수비수 거리 (m)
    pub defender_dist: f32,
    /// 앞 공간 (0~1)
    pub space_ahead: f32,
    /// 인터셉트 위험도 (0~1)
    pub intercept_risk: f32,
    /// 현재 드리블 의도
    pub intent: DribbleIntent,
    /// 회피 가능 여부
    pub can_evade: bool,
    /// 선수 속도 비율 (0~1, 1.0 = 최대 속도)
    pub speed_ratio: f32,
    /// 가장 가까운 수비수 인덱스 (TakeOn 이벤트용)
    pub nearest_defender_idx: Option<usize>,
    /// 가장 가까운 수비수 위치 (TakeOn 이벤트용)
    pub nearest_defender_pos: Option<(f32, f32)>,
}

impl DribbleObs {
    /// 기본값 (안전한 상황)
    pub fn safe() -> Self {
        Self {
            sep: 0.5,
            defender_dist: 10.0,
            space_ahead: 0.8,
            intercept_risk: 0.1,
            intent: DribbleIntent::Progress,
            can_evade: true,
            speed_ratio: 0.8,
            nearest_defender_idx: None,
            nearest_defender_pos: None,
        }
    }
}

// ============================================================================
// 조건 기반 전이 함수
// ============================================================================

/// 치달(KnockOn) 조건 체크
///
/// Progress intent + 공간 넓음 + 압박 낮음 + 스프린트 의도
#[inline]
pub fn should_knockon(obs: &DribbleObs) -> bool {
    obs.intent == DribbleIntent::Progress
        && obs.space_ahead >= 0.65
        && obs.defender_dist > EVADE_TRIGGER_DISTANCE
        && obs.speed_ratio >= 0.80
}

/// 기술 실행 트리거 조건 체크
///
/// Intent에 따라 기술 실행이 필요한 상황인지 판단
#[inline]
pub fn should_execute_technique(obs: &DribbleObs) -> bool {
    match obs.intent {
        DribbleIntent::Protect => obs.defender_dist <= 1.4,
        DribbleIntent::Progress => obs.space_ahead < SPACE_LOW_THRESHOLD,
        DribbleIntent::Beat => obs.defender_dist <= 1.6,
    }
}

/// Evade 트리거 조건 체크
#[inline]
pub fn should_evade(obs: &DribbleObs) -> bool {
    obs.intent == DribbleIntent::Beat
        && obs.defender_dist <= EVADE_TRIGGER_DISTANCE
        && obs.can_evade
}

// ============================================================================
// DribblePhase Enum (조건 기반 전이용 - remaining_ticks 제거)
// ============================================================================

/// 드리블 Phase (FSM 상태) - 조건 기반 전이
///
/// **핵심**: remaining_ticks 제거됨. 상한은 DribbleAction.phase_enter_tick으로 계산.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DribblePhase {
    /// 첫 터치 준비 (볼 받는 순간)
    Gather,

    /// 볼을 찔러 전진 (공이 1~2m 앞으로 굴러감)
    Touch,

    /// 볼 없이 몸만 이동 (공을 따라감)
    /// Exit 조건: sep <= TOUCH_READY_DIST 또는 상한 도달
    Carry,

    /// 공 다시 터치 (주기적 재컨트롤)
    SyncBall,

    /// 상대 회피 동작
    /// Exit 조건: 압박 해소 + 컨트롤 범위 또는 상한 도달
    Evade { direction: (f32, f32) },

    /// 볼을 길게 치고 달리기 (치달)
    /// Exit 조건: sep <= KNOCKON_REGAIN_DIST 또는 위험 증가 또는 상한 도달
    KnockAndRun {
        /// 공을 치는 거리 (3~5m)
        knock_distance: f32,
    },

    /// 기술 실행 중 (Shielding, Turn, Feint, Hesitation 등)
    /// Exit 조건: 기술 목표 달성 또는 상한 도달
    ExecutingTechnique {
        /// 실행 중인 기술
        technique: DribbleTechnique,
        /// 기술 성공 여부 (resolve 후 설정)
        success: Option<bool>,
    },

    /// 완료 (패스/슈팅으로 전환 또는 공 잃음)
    Finished,
}

impl DribblePhase {
    /// Phase 이름
    pub fn name(&self) -> &'static str {
        match self {
            DribblePhase::Gather => "Gather",
            DribblePhase::Touch => "Touch",
            DribblePhase::Carry => "Carry",
            DribblePhase::SyncBall => "SyncBall",
            DribblePhase::Evade { .. } => "Evade",
            DribblePhase::KnockAndRun { .. } => "KnockAndRun",
            DribblePhase::ExecutingTechnique { technique, .. } => match technique {
                DribbleTechnique::Shielding => "Shielding",
                DribbleTechnique::Turn => "Turn",
                DribbleTechnique::FaceUp => "FaceUp",
                DribbleTechnique::KnockOn => "KnockOn",
                DribbleTechnique::Feint => "Feint",
                DribbleTechnique::Hesitation => "Hesitation",
            },
            DribblePhase::Finished => "Finished",
        }
    }

    /// 활성 상태인지
    pub fn is_active(&self) -> bool {
        !matches!(self, DribblePhase::Finished)
    }

    /// 해당 Phase의 상한 틱 (가드레일)
    pub fn max_ticks(&self) -> u8 {
        match self {
            DribblePhase::Gather => MAX_GATHER_TICKS,
            DribblePhase::Touch => 1, // Touch는 즉시 전이
            DribblePhase::Carry => MAX_CARRY_TICKS,
            DribblePhase::SyncBall => 1, // SyncBall도 즉시 전이
            DribblePhase::Evade { .. } => MAX_EVADE_TICKS,
            DribblePhase::KnockAndRun { .. } => MAX_KNOCKON_TICKS,
            DribblePhase::ExecutingTechnique { technique, .. } => technique.duration_ticks(),
            DribblePhase::Finished => 0,
        }
    }
}

// ============================================================================
// DribbleResult
// ============================================================================

/// 드리블 결과
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DribbleResult {
    /// 드리블 계속
    Continue,
    /// 공 잃음 (루즈볼)
    Lost { ball_position: (f32, f32) },
    /// 드리블 성공 완료
    Completed,
}

// ============================================================================
// DribbleAction Struct
// ============================================================================

/// 실행 중인 드리블 액션 (조건 기반 전이)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DribbleAction {
    /// 액션 ID
    pub id: u64,

    /// 드리블 Phase
    pub phase: DribblePhase,

    /// 드리블러 선수 인덱스
    pub dribbler_idx: usize,

    /// 드리블러 팀 ID
    pub dribbler_team: u32,

    /// 드리블 방향 (정규화된 벡터)
    pub direction: (f32, f32),

    /// 공격적 드리블 여부 (돌파 시도)
    pub is_aggressive: bool,

    /// 시작 틱
    pub start_tick: u64,

    /// 현재 Phase 진입 틱 (조건 기반 전이용)
    pub phase_enter_tick: u64,

    /// 마지막 터치 틱 (로그/연출용)
    pub last_touch_tick: u64,

    /// 공 속도 (Touch 후 Rolling 속도)
    ball_velocity: (f32, f32),

    // ========== P7-DRIBBLE-TUNE: Intent/Technique ==========
    /// 드리블 의도 (왜 드리블하는가?)
    pub intent: DribbleIntent,

    /// 현재 실행 중인 기술
    pub current_technique: Option<DribbleTechnique>,

    // ========== Viewer Event ==========
    /// 대기 중인 드리블 터치 이벤트 (일반 드리블)
    #[serde(skip)]
    pub pending_viewer_event: Option<DribbleTouchEvent>,

    /// 대기 중인 돌파 이벤트 (Beat intent 시)
    #[serde(skip)]
    pub pending_take_on_event: Option<TakeOnEvent>,

    // ========== P17 Phase 4: 스킬 필드 ==========
    /// 드리블 컨트롤 (0-99)
    pub dribbling: u8,
    /// 속도 (0-99)
    pub pace: u8,
    /// 민첩성 (0-99)
    pub agility: u8,
    /// 밸런스 (0-99)
    pub balance: u8,
    /// 힘 (0-99)
    pub strength: u8,
}

impl DribbleAction {
    /// 새 드리블 액션 생성 (기본 Intent: Progress)
    pub fn new(
        id: u64,
        dribbler_idx: usize,
        dribbler_team: u32,
        direction: (f32, f32),
        is_aggressive: bool,
        start_tick: u64,
    ) -> Self {
        let dir = normalize(direction);
        let intent = if is_aggressive { DribbleIntent::Beat } else { DribbleIntent::Progress };
        Self {
            id,
            phase: DribblePhase::Touch, // 바로 터치로 시작
            dribbler_idx,
            dribbler_team,
            direction: dir,
            is_aggressive,
            start_tick,
            phase_enter_tick: start_tick,
            last_touch_tick: start_tick,
            ball_velocity: (0.0, 0.0),
            intent,
            current_technique: None,
            pending_viewer_event: None,
            pending_take_on_event: None,
            // P17: 스킬 필드 기본값
            dribbling: 0,
            pace: 0,
            agility: 0,
            balance: 0,
            strength: 0,
        }
    }

    /// P17 Phase 4: 능력치와 함께 드리블 액션 생성
    pub fn new_with_attrs(
        id: u64,
        dribbler_idx: usize,
        dribbler_team: u32,
        direction: (f32, f32),
        is_aggressive: bool,
        start_tick: u64,
        attrs: &PlayerAttributes,
    ) -> Self {
        let mut action =
            Self::new(id, dribbler_idx, dribbler_team, direction, is_aggressive, start_tick);
        action.dribbling = attrs.dribbling;
        action.pace = attrs.pace;
        action.agility = attrs.agility;
        action.balance = attrs.balance;
        action.strength = attrs.strength;
        action
    }

    /// Intent 기반 드리블 생성 (P7-DRIBBLE-TUNE)
    pub fn new_with_intent(
        id: u64,
        dribbler_idx: usize,
        dribbler_team: u32,
        direction: (f32, f32),
        intent: DribbleIntent,
        start_tick: u64,
    ) -> Self {
        let dir = normalize(direction);
        let is_aggressive = matches!(intent, DribbleIntent::Beat);
        Self {
            id,
            phase: DribblePhase::Touch,
            dribbler_idx,
            dribbler_team,
            direction: dir,
            is_aggressive,
            start_tick,
            phase_enter_tick: start_tick,
            last_touch_tick: start_tick,
            ball_velocity: (0.0, 0.0),
            intent,
            current_technique: None,
            pending_viewer_event: None,
            pending_take_on_event: None,
            // P17: 스킬 필드 기본값
            dribbling: 0,
            pace: 0,
            agility: 0,
            balance: 0,
            strength: 0,
        }
    }

    /// Gather phase로 시작 (공을 받는 경우)
    pub fn new_with_gather(
        id: u64,
        dribbler_idx: usize,
        dribbler_team: u32,
        direction: (f32, f32),
        is_aggressive: bool,
        start_tick: u64,
    ) -> Self {
        let dir = normalize(direction);
        let intent = if is_aggressive { DribbleIntent::Beat } else { DribbleIntent::Progress };
        Self {
            id,
            phase: DribblePhase::Gather,
            dribbler_idx,
            dribbler_team,
            direction: dir,
            is_aggressive,
            start_tick,
            phase_enter_tick: start_tick,
            last_touch_tick: start_tick,
            ball_velocity: (0.0, 0.0),
            intent,
            current_technique: None,
            pending_viewer_event: None,
            pending_take_on_event: None,
            // P17: 스킬 필드 기본값
            dribbling: 0,
            pace: 0,
            agility: 0,
            balance: 0,
            strength: 0,
        }
    }

    /// 드리블 완료 여부
    pub fn is_finished(&self) -> bool {
        matches!(self.phase, DribblePhase::Finished)
    }

    /// 드리블 방향 변경
    pub fn change_direction(&mut self, new_direction: (f32, f32)) {
        self.direction = normalize(new_direction);
    }

    /// 드리블 종료 (패스/슈팅으로 전환 시)
    pub fn finish(&mut self) {
        self.phase = DribblePhase::Finished;
    }

    /// Phase 진입 틱 업데이트 헬퍼
    fn transition_to(&mut self, new_phase: DribblePhase, current_tick: u64) {
        self.phase = new_phase;
        self.phase_enter_tick = current_tick;
    }

    /// 현재 Phase에서 경과한 틱 수
    pub fn phase_age(&self, current_tick: u64) -> u8 {
        current_tick.saturating_sub(self.phase_enter_tick) as u8
    }

    /// 회피 시작
    pub fn start_evade(&mut self, evade_direction: (f32, f32), current_tick: u64) {
        self.transition_to(
            DribblePhase::Evade { direction: normalize(evade_direction) },
            current_tick,
        );
    }

    // ========================================================================
    // Viewer Event Methods
    // ========================================================================

    /// ViewerEvent 가져가기 (tick_based에서 호출)
    pub fn take_viewer_event(&mut self) -> Option<DribbleTouchEvent> {
        self.pending_viewer_event.take()
    }

    /// TakeOnEvent 가져가기 (tick_based에서 호출)
    pub fn take_take_on_event(&mut self) -> Option<TakeOnEvent> {
        self.pending_take_on_event.take()
    }

    /// 돌파 이벤트 생성 (Beat intent + 수비수 존재 시 호출)
    ///
    /// # Arguments
    /// - `t_ms`: 현재 시간 (ms)
    /// - `ball_from`, `ball_to`: 공 위치
    /// - `technique`: 사용된 기술
    /// - `outcome`: 돌파 결과
    /// - `dribbler_pos`: 드리블러 위치
    /// - `defender_track_id`: 수비수 track_id
    /// - `defender_pos`: 수비수 위치
    /// - `skill_factor`: 스킬 보정 (0~1)
    pub fn emit_take_on_event(
        &mut self,
        t_ms: u64,
        ball_from: (f32, f32),
        ball_to: (f32, f32),
        technique: DribbleTouchType,
        outcome: TakeOnOutcome,
        dribbler_pos: (f32, f32),
        defender_track_id: u32,
        defender_pos: (f32, f32),
        skill_factor: f32,
    ) {
        let lock_ms = TakeOnEvent::calculate_lock_ms(technique, skill_factor);

        self.pending_take_on_event = Some(TakeOnEvent {
            t_ms,
            dribbler_track_id: self.dribbler_idx as u32,
            defender_track_id,
            technique,
            outcome,
            lock_ms,
            ball_from,
            ball_to,
            dribbler_pos,
            defender_pos,
            direction: Some(self.direction),
        });
    }

    /// 터치 이벤트 생성 (Touch phase 진입 시 호출)
    ///
    /// # Arguments
    /// - `t_ms`: 현재 시간 (ms)
    /// - `ball_from`: 공 현재 위치
    /// - `ball_to`: 공 도착 예상 위치
    /// - `touch_type`: 터치 유형
    /// - `player_pos`: 선수 위치 (Tier1 옵션)
    /// - `defender_track_id`: 가장 가까운 수비수 (옵션)
    /// - `pressure_factor`: 압박 수준 (0~1)
    pub fn emit_touch_event(
        &mut self,
        t_ms: u64,
        ball_from: (f32, f32),
        ball_to: (f32, f32),
        touch_type: DribbleTouchType,
        player_pos: Option<(f32, f32)>,
        defender_track_id: Option<u32>,
        pressure_factor: f32,
    ) {
        let lock_ms = DribbleTouchEvent::calculate_lock_ms(touch_type, pressure_factor);

        self.pending_viewer_event = Some(DribbleTouchEvent {
            t_ms,
            dribbler_track_id: self.dribbler_idx as u32,
            touch_type,
            ball_from,
            ball_to,
            lock_ms,
            player_pos,
            defender_track_id,
            direction: Some(self.direction),
        });
    }

    /// 현재 Phase에서 적절한 터치 타입 결정
    fn determine_touch_type(&self) -> DribbleTouchType {
        match &self.phase {
            DribblePhase::Gather => DribbleTouchType::FirstTouch,
            DribblePhase::Touch => DribbleTouchType::Carry,
            DribblePhase::KnockAndRun { .. } => DribbleTouchType::KnockOn,
            DribblePhase::Evade { .. } => DribbleTouchType::Evade,
            DribblePhase::ExecutingTechnique { technique, .. } => match technique {
                DribbleTechnique::Shielding => DribbleTouchType::Shielding,
                DribbleTechnique::Turn => DribbleTouchType::Turn,
                DribbleTechnique::Feint => DribbleTouchType::Feint,
                DribbleTechnique::Hesitation => DribbleTouchType::Hesitation,
                DribbleTechnique::KnockOn => DribbleTouchType::KnockOn,
                DribbleTechnique::FaceUp => DribbleTouchType::Carry,
            },
            _ => DribbleTouchType::Carry,
        }
    }

    // ========================================================================
    // Tick Update (조건 기반 전이)
    // ========================================================================

    /// 매 틱 업데이트 (조건 기반 전이)
    ///
    /// # Arguments
    /// - `player_pos`: 선수 위치
    /// - `ball_pos`: 공 위치
    /// - `current_tick`: 현재 틱
    /// - `obs`: 관측값 (조건 기반 전이용, 없으면 기본값)
    ///
    /// # Returns
    /// (DribbleResult, ball_velocity, player_velocity)
    pub fn update_tick_with_obs(
        &mut self,
        player_pos: (f32, f32),
        ball_pos: (f32, f32),
        current_tick: u64,
        obs: &DribbleObs,
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        let age = self.phase_age(current_tick);
        let max = self.phase.max_ticks();

        match self.phase {
            DribblePhase::Gather => {
                self.tick_gather_cond(current_tick, obs, age, max, player_pos, ball_pos)
            }
            DribblePhase::Touch => self.tick_touch_cond(current_tick, obs, player_pos, ball_pos),
            DribblePhase::Carry => {
                self.tick_carry_cond(player_pos, ball_pos, current_tick, obs, age, max)
            }
            DribblePhase::SyncBall => self.tick_sync_ball_cond(player_pos, ball_pos, current_tick),
            DribblePhase::Evade { direction } => {
                self.tick_evade_cond(direction, current_tick, obs, age, max)
            }
            DribblePhase::KnockAndRun { knock_distance } => self.tick_knock_and_run_cond(
                knock_distance,
                player_pos,
                ball_pos,
                current_tick,
                obs,
                age,
                max,
            ),
            DribblePhase::ExecutingTechnique { technique, success } => self
                .tick_executing_technique_cond(
                    technique,
                    success,
                    player_pos,
                    ball_pos,
                    current_tick,
                    age,
                    max,
                ),
            DribblePhase::Finished => (DribbleResult::Completed, (0.0, 0.0), (0.0, 0.0)),
        }
    }

    /// update_tick (현재 틱 전달)
    ///
    /// # Arguments
    /// * `player_pos` - 드리블러 위치 (미터)
    /// * `ball_pos` - 공 위치 (미터)
    /// * `current_tick` - 현재 틱
    ///
    /// # Returns
    /// (DribbleResult, ball_velocity, player_velocity)
    pub fn update_tick(
        &mut self,
        player_pos: (f32, f32),
        ball_pos: (f32, f32),
        current_tick: u64,
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        // obs가 없으면 기본값으로 생성
        let sep = distance(player_pos, ball_pos);
        let obs = DribbleObs {
            sep,
            defender_dist: 10.0, // 기본값: 수비 없음
            space_ahead: 0.8,
            intercept_risk: 0.1,
            intent: self.intent,
            can_evade: true,
            speed_ratio: 0.8,
            nearest_defender_idx: None, // 기본값: 수비수 정보 없음
            nearest_defender_pos: None,
        };
        self.update_tick_with_obs(player_pos, ball_pos, current_tick, &obs)
    }

    // ========================================================================
    // P7-DRIBBLE-TUNE: Technique Phase Methods
    // ========================================================================

    /// 기술 시작 (ExecutingTechnique 페이즈 진입)
    pub fn start_technique(&mut self, technique: DribbleTechnique, current_tick: u64) {
        self.current_technique = Some(technique);
        self.transition_to(
            DribblePhase::ExecutingTechnique { technique, success: None },
            current_tick,
        );
    }

    /// KnockAndRun (치달) 시작
    pub fn start_knock_and_run(&mut self, knock_distance: f32, current_tick: u64) {
        self.current_technique = Some(DribbleTechnique::KnockOn);
        self.transition_to(DribblePhase::KnockAndRun { knock_distance }, current_tick);
    }

    // ========================================================================
    // 조건 기반 Phase Tick 함수들
    // ========================================================================

    /// Gather Phase - 공 받기 준비 (조건 기반)
    fn tick_gather_cond(
        &mut self,
        current_tick: u64,
        obs: &DribbleObs,
        age: u8,
        max: u8,
        player_pos: (f32, f32),
        ball_pos: (f32, f32),
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        // Exit 조건: 공이 컨트롤 범위 또는 상한 도달
        if obs.sep <= TOUCH_READY_DIST + 0.1 || age >= max {
            // Gather → Touch 전환 시 FirstTouch 이벤트 생성
            let t_ms = current_tick * 250; // tick → ms
            let touch_dist = DribbleTouchType::FirstTouch.ball_distance();
            let ball_to = (
                ball_pos.0 + self.direction.0 * touch_dist,
                ball_pos.1 + self.direction.1 * touch_dist,
            );
            let pressure = 1.0 - obs.defender_dist.min(5.0) / 5.0; // 가까울수록 압박 높음
            self.emit_touch_event(
                t_ms,
                ball_pos,
                ball_to,
                DribbleTouchType::FirstTouch,
                Some(player_pos),
                None,
                pressure,
            );
            self.transition_to(DribblePhase::Touch, current_tick);
        }
        // Gather 중에는 이동 없음
        (DribbleResult::Continue, (0.0, 0.0), (0.0, 0.0))
    }

    /// Touch Phase - 공을 전방으로 찔러줌 (조건 기반)
    fn tick_touch_cond(
        &mut self,
        current_tick: u64,
        obs: &DribbleObs,
        player_pos: (f32, f32),
        ball_pos: (f32, f32),
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        // 터치 방향
        let touch_dir = self.direction;

        // 공 속도 설정 (선수보다 약간 빠르게)
        let ball_speed = DRIBBLE_SPEED * BALL_ROLL_SPEED_MULTIPLIER;
        self.ball_velocity = (touch_dir.0 * ball_speed, touch_dir.1 * ball_speed);

        // 터치 기록
        self.last_touch_tick = current_tick;

        // 터치 타입 및 이동 거리 결정
        let touch_type =
            if should_knockon(obs) { DribbleTouchType::KnockOn } else { DribbleTouchType::Carry };
        let touch_dist = touch_type.ball_distance();
        let ball_to =
            (ball_pos.0 + touch_dir.0 * touch_dist, ball_pos.1 + touch_dir.1 * touch_dist);

        // ViewerEvent 생성 (Contact Frame!)
        let t_ms = current_tick * 250; // tick → ms
        let pressure = 1.0 - obs.defender_dist.min(5.0) / 5.0;

        // Intent에 따라 DribbleTouch 또는 TakeOn 이벤트 생성
        match self.intent {
            DribbleIntent::Beat => {
                // 돌파 시도 (TakeOn) - 수비수 정보가 있으면 TakeOn 이벤트
                if let (Some(defender_idx), Some(defender_pos)) =
                    (obs.nearest_defender_idx, obs.nearest_defender_pos)
                {
                    // TakeOn 결과: 수비수와의 거리에 따라 판정
                    let outcome = if obs.defender_dist < 1.5 {
                        TakeOnOutcome::InProgress // 아직 진행 중 (근접)
                    } else {
                        TakeOnOutcome::Success // 성공 (1.5m 이상 벗어남)
                    };

                    self.emit_take_on_event(
                        t_ms,
                        ball_pos,
                        ball_to,
                        touch_type,
                        outcome,
                        player_pos,
                        defender_idx as u32,
                        defender_pos,
                        0.7, // skill_factor
                    );
                } else {
                    // 수비수 정보 없으면 일반 드리블로 처리
                    self.emit_touch_event(
                        t_ms,
                        ball_pos,
                        ball_to,
                        touch_type,
                        Some(player_pos),
                        None,
                        pressure,
                    );
                }
            }
            DribbleIntent::Protect | DribbleIntent::Progress => {
                // 일반 드리블 (DribbleTouch)
                self.emit_touch_event(
                    t_ms,
                    ball_pos,
                    ball_to,
                    touch_type,
                    Some(player_pos),
                    obs.nearest_defender_idx.map(|i| i as u32),
                    pressure,
                );
            }
        }

        // 다음 Phase 결정 (조건 기반)
        if should_knockon(obs) {
            // 치달 조건 충족 → KnockAndRun
            let knock_dist = 3.0 + (obs.space_ahead * 2.0); // 3~5m
            self.transition_to(
                DribblePhase::KnockAndRun { knock_distance: knock_dist },
                current_tick,
            );
        } else {
            // 일반 → Carry
            self.transition_to(DribblePhase::Carry, current_tick);
        }

        // 공 속도 반환, 선수는 아직 움직이지 않음 (Touch 틱에는 공만 굴러감)
        (DribbleResult::Continue, self.ball_velocity, (0.0, 0.0))
    }

    /// Carry Phase - 선수가 공을 따라감 (조건 기반)
    fn tick_carry_cond(
        &mut self,
        player_pos: (f32, f32),
        ball_pos: (f32, f32),
        current_tick: u64,
        obs: &DribbleObs,
        age: u8,
        max: u8,
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        let dist_to_ball = distance(player_pos, ball_pos);

        // 전역 안전장치: 공이 너무 멀면 → 공 잃음
        if dist_to_ball > DRIBBLE_CONTROL_RANGE {
            self.transition_to(DribblePhase::Finished, current_tick);
            return (DribbleResult::Lost { ball_position: ball_pos }, (0.0, 0.0), (0.0, 0.0));
        }

        // 공 쪽으로 이동
        let player_vel = if dist_to_ball > DRIBBLE_MIN_SEPARATION {
            let dir = normalize((ball_pos.0 - player_pos.0, ball_pos.1 - player_pos.1));
            (dir.0 * DRIBBLE_SPEED * TICK_DT, dir.1 * DRIBBLE_SPEED * TICK_DT)
        } else {
            (0.0, 0.0)
        };

        // 공 속도 감소 (마찰)
        self.ball_velocity.0 *= GRASS_FRICTION;
        self.ball_velocity.1 *= GRASS_FRICTION;

        let ball_vel = (self.ball_velocity.0 * TICK_DT, self.ball_velocity.1 * TICK_DT);

        // ===== 조건 기반 전이 (우선순위 순) =====

        // 1. 공이 발 앞에 도달 → SyncBall (클로즈 컨트롤)
        if obs.sep <= TOUCH_READY_DIST {
            self.transition_to(DribblePhase::SyncBall, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 2. Beat intent + 압박 → Evade
        if should_evade(obs) {
            // 회피 방향: 수비 반대쪽
            let evade_dir = perpendicular(self.direction, true);
            self.transition_to(DribblePhase::Evade { direction: evade_dir }, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 3. 압박 높음 (Beat 아닌 경우) → 즉시 Touch (컨트롤 강화)
        if obs.defender_dist <= EVADE_TRIGGER_DISTANCE && obs.intent != DribbleIntent::Beat {
            self.transition_to(DribblePhase::Touch, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 4. 치달 조건 충족 → KnockAndRun
        if should_knockon(obs) {
            let knock_dist = 3.0 + (obs.space_ahead * 2.0);
            self.transition_to(
                DribblePhase::KnockAndRun { knock_distance: knock_dist },
                current_tick,
            );
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 5. 상한 도달 → Touch (가드레일)
        if age >= max {
            self.transition_to(DribblePhase::Touch, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 그 외: Carry 유지
        (DribbleResult::Continue, ball_vel, player_vel)
    }

    /// SyncBall Phase - 공을 다시 컨트롤 (조건 기반)
    fn tick_sync_ball_cond(
        &mut self,
        player_pos: (f32, f32),
        ball_pos: (f32, f32),
        current_tick: u64,
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        let dist_to_ball = distance(player_pos, ball_pos);

        if dist_to_ball < DRIBBLE_MIN_SEPARATION + 0.5 {
            // 공 컨트롤 성공 → 다시 Touch
            self.transition_to(DribblePhase::Touch, current_tick);
            (DribbleResult::Continue, (0.0, 0.0), (0.0, 0.0))
        } else if dist_to_ball < DRIBBLE_MAX_SEPARATION {
            // 아직 범위 밖 → Carry로 전환해서 따라가기
            self.transition_to(DribblePhase::Carry, current_tick);

            let dir = normalize((ball_pos.0 - player_pos.0, ball_pos.1 - player_pos.1));
            let player_vel = (dir.0 * DRIBBLE_SPEED * TICK_DT, dir.1 * DRIBBLE_SPEED * TICK_DT);

            (DribbleResult::Continue, (0.0, 0.0), player_vel)
        } else {
            // 공이 너무 멀어짐 → 드리블 실패
            self.transition_to(DribblePhase::Finished, current_tick);
            (DribbleResult::Lost { ball_position: ball_pos }, (0.0, 0.0), (0.0, 0.0))
        }
    }

    /// Evade Phase - 상대 회피 (조건 기반)
    fn tick_evade_cond(
        &mut self,
        evade_dir: (f32, f32),
        current_tick: u64,
        obs: &DribbleObs,
        age: u8,
        max: u8,
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        // 회피 방향으로 이동
        let player_vel = (evade_dir.0 * EVADE_SPEED * TICK_DT, evade_dir.1 * EVADE_SPEED * TICK_DT);

        // 공도 같이 이동 (같은 방향)
        let ball_vel = (
            evade_dir.0 * EVADE_SPEED * TICK_DT * 0.8, // 약간 느리게
            evade_dir.1 * EVADE_SPEED * TICK_DT * 0.8,
        );

        // 방향 업데이트
        self.direction = evade_dir;

        // ===== 조건 기반 전이 =====

        // 1. 압박 해소 + 컨트롤 범위 → Carry
        if !should_evade(obs) && obs.sep <= DRIBBLE_CONTROL_RANGE {
            self.transition_to(DribblePhase::Carry, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 2. 공이 발 앞 → Touch
        if obs.sep <= TOUCH_READY_DIST {
            self.transition_to(DribblePhase::Touch, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 3. 상한 도달 → Touch (가드레일)
        if age >= max {
            self.transition_to(DribblePhase::Touch, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 그 외: Evade 유지
        (DribbleResult::Continue, ball_vel, player_vel)
    }

    /// KnockAndRun Phase 틱 업데이트 (조건 기반)
    fn tick_knock_and_run_cond(
        &mut self,
        knock_distance: f32,
        player_pos: (f32, f32),
        ball_pos: (f32, f32),
        current_tick: u64,
        obs: &DribbleObs,
        age: u8,
        max: u8,
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        let dist_to_ball = distance(player_pos, ball_pos);

        // 스프린트 속도 (일반 드리블보다 빠름)
        const SPRINT_SPEED: f32 = 2.5;

        // 공 쪽으로 스프린트
        let player_vel = if dist_to_ball > 0.5 {
            let dir = normalize((ball_pos.0 - player_pos.0, ball_pos.1 - player_pos.1));
            (dir.0 * SPRINT_SPEED * TICK_DT, dir.1 * SPRINT_SPEED * TICK_DT)
        } else {
            (0.0, 0.0)
        };

        // 공 속도 감소 (마찰)
        let ball_vel = (
            self.direction.0 * DRIBBLE_SPEED * 0.3 * TICK_DT,
            self.direction.1 * DRIBBLE_SPEED * 0.3 * TICK_DT,
        );

        // ===== 조건 기반 전이 (우선순위 순) =====

        // 1. 공 회수 거리 도달 → Touch ("짧은 치달" 가능!)
        if obs.sep <= KNOCKON_REGAIN_DIST {
            self.transition_to(DribblePhase::Touch, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 2. 공이 너무 멀어지면 → 공 잃음
        if dist_to_ball > knock_distance + DRIBBLE_MAX_SEPARATION {
            self.transition_to(DribblePhase::Finished, current_tick);
            return (DribbleResult::Lost { ball_position: ball_pos }, (0.0, 0.0), (0.0, 0.0));
        }

        // 3. 위험 증가 / 공간 부족 / 압박 → Touch 또는 Evade
        if obs.intercept_risk >= INTERCEPT_RISK_HIGH
            || obs.space_ahead <= SPACE_LOW_THRESHOLD
            || should_evade(obs)
        {
            if should_evade(obs) {
                let evade_dir = perpendicular(self.direction, true);
                self.transition_to(DribblePhase::Evade { direction: evade_dir }, current_tick);
            } else {
                self.transition_to(DribblePhase::Touch, current_tick);
            }
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 4. 상한 도달 → Touch (가드레일)
        if age >= max {
            self.transition_to(DribblePhase::Touch, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 그 외: KnockAndRun 유지
        (DribbleResult::Continue, ball_vel, player_vel)
    }

    /// ExecutingTechnique Phase 틱 업데이트 (조건 기반)
    fn tick_executing_technique_cond(
        &mut self,
        technique: DribbleTechnique,
        success: Option<bool>,
        _player_pos: (f32, f32),
        ball_pos: (f32, f32),
        current_tick: u64,
        age: u8,
        max: u8,
    ) -> (DribbleResult, (f32, f32), (f32, f32)) {
        // 기술 실행 중 이동
        let speed = technique.base_speed();
        let player_vel = (self.direction.0 * speed * TICK_DT, self.direction.1 * speed * TICK_DT);

        // 공도 같이 이동 (기술에 따라 다름)
        let ball_dist = technique.ball_distance();
        let ball_vel = if ball_dist > 1.0 {
            // KnockOn 같이 공이 멀리 가는 경우
            (self.direction.0 * speed * 1.2 * TICK_DT, self.direction.1 * speed * 1.2 * TICK_DT)
        } else {
            // 공이 가까이 붙어있는 경우
            (self.direction.0 * speed * TICK_DT, self.direction.1 * speed * TICK_DT)
        };

        // ===== 조건 기반 전이 =====

        // 1. 기술 완료 판정 (성공/실패 설정됨)
        if let Some(succeeded) = success {
            if succeeded {
                self.transition_to(DribblePhase::Touch, current_tick);
            } else {
                self.transition_to(DribblePhase::Finished, current_tick);
                return (DribbleResult::Lost { ball_position: ball_pos }, (0.0, 0.0), (0.0, 0.0));
            }
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 2. 상한 도달 → Touch (가드레일, 판정 없으면 성공으로 간주)
        if age >= max {
            self.transition_to(DribblePhase::Touch, current_tick);
            return (DribbleResult::Continue, ball_vel, player_vel);
        }

        // 그 외: ExecutingTechnique 유지
        (DribbleResult::Continue, ball_vel, player_vel)
    }

    /// 기술 판정 결과 설정
    pub fn set_technique_result(&mut self, succeeded: bool) {
        if let DribblePhase::ExecutingTechnique { technique, .. } = self.phase {
            self.phase = DribblePhase::ExecutingTechnique { technique, success: Some(succeeded) };
        }
    }

    /// 기술 실행 중인지 확인
    pub fn is_executing_technique(&self) -> bool {
        matches!(self.phase, DribblePhase::ExecutingTechnique { .. })
    }

    /// KnockAndRun 중인지 확인
    pub fn is_knock_and_run(&self) -> bool {
        matches!(self.phase, DribblePhase::KnockAndRun { .. })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 두 점 사이 거리
#[inline]
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// 벡터 정규화
#[inline]
fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len > 0.001 {
        (v.0 / len, v.1 / len)
    } else {
        (1.0, 0.0) // 기본 방향
    }
}

/// 내적
#[inline]
fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

/// 상대 선수가 접근하면 회피 방향 결정
pub fn should_trigger_evade(
    dribbler_pos: (f32, f32),
    opponent_positions: &[(f32, f32)],
    dribble_direction: (f32, f32),
) -> Option<(f32, f32)> {
    for opp_pos in opponent_positions {
        let dist = distance(dribbler_pos, *opp_pos);

        // 설정된 거리 이내에 상대가 있으면
        if dist < EVADE_TRIGGER_DISTANCE {
            // 상대 방향
            let to_opp = normalize((opp_pos.0 - dribbler_pos.0, opp_pos.1 - dribbler_pos.1));

            // 수직 방향 (왼쪽 또는 오른쪽)
            let perp_left = (-to_opp.1, to_opp.0);
            let perp_right = (to_opp.1, -to_opp.0);

            // 드리블 방향에 더 가까운 쪽으로
            let dot_left = dot(perp_left, dribble_direction);
            let dot_right = dot(perp_right, dribble_direction);

            let evade_dir = if dot_left > dot_right { perp_left } else { perp_right };

            return Some(evade_dir);
        }
    }
    None
}

/// 드리블 시도 가능 여부
pub fn can_start_dribble(player_pos: (f32, f32), ball_pos: (f32, f32), has_ball: bool) -> bool {
    if !has_ball {
        return false;
    }

    let dist = distance(player_pos, ball_pos);
    dist < DRIBBLE_MAX_SEPARATION
}

// ============================================================================
// ActionModel Impl (공통 trait)
// ============================================================================

/// 드리블 물리 파라미터 (ActionModel용)
#[derive(Debug, Clone, Copy)]
pub struct DribblePhysicsParams {
    /// 이동 속도 (m/s)
    pub speed: f32,
    /// 공과의 거리 (m)
    pub ball_distance: f32,
    /// 실패 시 공 잃을 리스크 (0~1)
    pub risk: f32,
    /// 소요 시간 (틱)
    pub duration_ticks: u8,
}

/// 드리블 컨텍스트 (ActionModel용)
#[derive(Debug, Clone, Copy, Default)]
pub struct DribbleContext {
    /// 앞 공간 존재 (0~1)
    pub space_ahead: f32,
    /// 듀얼 위협 (0~1)
    pub duel_threat: f32,
    /// 라인 돌파 가능성 (0~1)
    pub lane_quality: f32,
}

/// 드리블 스킬 (ActionModel용, 0~20 스케일)
#[derive(Debug, Clone, Copy, Default)]
pub struct DribbleSkills {
    pub dribbling: u8,
    pub technique: u8,
    pub acceleration: u8,
    pub agility: u8,
    pub decisions: u8,
    pub composure: u8,
}

impl DribbleSkills {
    /// PlayerAttributes에서 생성 (0~99 → 0~20 스케일)
    pub fn from_attributes(attrs: &PlayerAttributes) -> Self {
        Self {
            dribbling: (attrs.dribbling / 5).min(20),
            technique: (attrs.technique / 5).min(20),
            acceleration: (attrs.acceleration / 5).min(20),
            agility: (attrs.agility / 5).min(20),
            decisions: (attrs.decisions / 5).min(20),
            composure: (attrs.composure / 5).min(20),
        }
    }
}

/// FM 메타 기반 드리블 스킬 (FIX_2601/0107)
///
/// FM 메타 분석 결과 반영:
/// - Pace: +21 points (Tier 1)
/// - Acceleration: +20 points (Tier 1)
/// - Balance: +15 points (Tier 1)
/// - Agility: +15 points (Tier 1)
/// - Flair: 창의적 돌파 시 중요
/// - Anticipation: 수비수 예측에 중요
#[derive(Debug, Clone, Copy)]
pub struct DribbleSkillsFM {
    /// 기본 스킬 (0~99 스케일)
    pub dribbling: u8,
    pub agility: u8,
    pub pace: u8,
    pub acceleration: u8,
    pub balance: u8,
    pub flair: u8,
    pub anticipation: u8,
}

impl DribbleSkillsFM {
    /// PlayerAttributes에서 생성 (0~99 스케일 유지)
    pub fn from_attributes(attrs: &PlayerAttributes) -> Self {
        Self {
            dribbling: attrs.dribbling,
            agility: attrs.agility,
            pace: attrs.pace,
            acceleration: attrs.acceleration,
            balance: attrs.balance,
            flair: attrs.flair,
            anticipation: attrs.anticipation,
        }
    }

    /// FM 메타 공격 점수 계산 (0.0 ~ 1.0)
    ///
    /// 가중치: dribbling(30%) + agility(20%) + pace(15%) + acceleration(15%)
    ///        + balance(10%) + flair(5%) + anticipation(5%)
    pub fn attack_score(&self) -> f32 {
        use crate::engine::match_sim::attribute_calc::dribble_attack_score_fm_meta;

        dribble_attack_score_fm_meta(
            self.dribbling as f32,
            self.agility as f32,
            self.pace as f32,
            self.acceleration as f32,
            self.balance as f32,
            self.flair as f32,
            self.anticipation as f32,
        )
    }
}

/// FM 메타 기반 드리블 성공 확률 계산 (FIX_2601/0107)
///
/// FM-Arena 테스트 결과 반영:
/// - Pace/Acceleration이 최상위 메타 속성
/// - 속도형 드리블러도 기술형과 경쟁 가능
pub fn dribble_success_prob_fm_meta(
    attacker: &DribbleSkillsFM,
    defender_tackling: f32,
    defender_anticipation: f32,
    defender_pace: f32,
    pressure: f32,
) -> f32 {
    let attack_score = attacker.attack_score();

    // 수비수 점수: tackling(35%) + anticipation(25%) + pace(25%) + positioning(15%)
    let def_t = defender_tackling / 100.0;
    let def_a = defender_anticipation / 100.0;
    let def_p = defender_pace / 100.0;
    let defender_score = def_t * 0.35 + def_a * 0.25 + def_p * 0.25 + 0.15 * 0.5; // default positioning

    // 기본 성공률: 공격점수 vs 수비점수
    let diff = attack_score - defender_score;
    let base = 0.50 + diff * 0.40;

    // 압박 페널티 (낮은 composure 가정 시)
    let pressure_penalty = pressure * 0.15;

    (base - pressure_penalty).clamp(0.15, 0.85)
}

impl DribbleTechnique {
    /// ActionModel용 물리 파라미터
    pub fn physics_params(&self) -> DribblePhysicsParams {
        DribblePhysicsParams {
            speed: self.base_speed(),
            ball_distance: self.ball_distance(),
            risk: self.base_loss_risk(),
            duration_ticks: self.duration_ticks(),
        }
    }
}

/// Zero-sized model type for trait impl
pub struct DribbleModel;

impl ActionModel for DribbleModel {
    type Intent = DribbleIntent;
    type Technique = DribbleTechnique;
    type PhysicsParams = DribblePhysicsParams;
    type Context = DribbleContext;
    type Skills = DribbleSkills;

    #[inline]
    fn available_techniques(intent: Self::Intent) -> &'static [Self::Technique] {
        match intent {
            DribbleIntent::Protect => &[DribbleTechnique::Shielding, DribbleTechnique::Turn],
            DribbleIntent::Progress => &[DribbleTechnique::FaceUp, DribbleTechnique::KnockOn],
            DribbleIntent::Beat => &[
                DribbleTechnique::Feint,
                DribbleTechnique::KnockOn,
                DribbleTechnique::Hesitation,
                DribbleTechnique::Turn,
            ],
        }
    }

    #[inline]
    fn physics_params(technique: Self::Technique) -> Self::PhysicsParams {
        technique.physics_params()
    }

    fn choose_technique<R: Rng + ?Sized>(
        intent: Self::Intent,
        context: Self::Context,
        skills: Self::Skills,
        pressure: f32,
        rng: &mut R,
    ) -> Self::Technique {
        let pressure = clamp01(pressure);

        let space = clamp01(context.space_ahead);
        let duel = clamp01(context.duel_threat);
        let _lane = clamp01(context.lane_quality);

        let drib = skill01(skills.dribbling);
        let tech = skill01(skills.technique);
        let acc = skill01(skills.acceleration);
        let agi = skill01(skills.agility);
        let dec = skill01(skills.decisions);
        let comp = skill01(skills.composure);

        let candidates = Self::available_techniques(intent);
        let mut weights: Vec<f32> = Vec::with_capacity(candidates.len());

        for &t in candidates {
            let mut w = 1.0;

            let skill_fit = match t {
                DribbleTechnique::Shielding => 0.30 * drib + 0.40 * comp + 0.30 * dec,
                DribbleTechnique::Turn => 0.40 * drib + 0.35 * agi + 0.25 * tech,
                DribbleTechnique::FaceUp => 0.35 * tech + 0.35 * drib + 0.30 * dec,
                DribbleTechnique::KnockOn => 0.35 * drib + 0.45 * acc + 0.20 * tech,
                DribbleTechnique::Feint => 0.40 * tech + 0.35 * agi + 0.25 * comp,
                DribbleTechnique::Hesitation => 0.35 * tech + 0.35 * agi + 0.30 * dec,
            };
            w *= common_lerp(0.6, 1.6, clamp01(skill_fit));

            w *= match t {
                DribbleTechnique::Shielding => {
                    common_lerp(1.1, 0.8, space) * common_lerp(0.8, 1.2, duel)
                }
                DribbleTechnique::Turn => common_lerp(0.9, 1.2, duel),
                DribbleTechnique::FaceUp => common_lerp(0.9, 1.2, space),
                DribbleTechnique::KnockOn => common_lerp(0.7, 1.5, space),
                DribbleTechnique::Feint => common_lerp(0.9, 1.3, duel),
                DribbleTechnique::Hesitation => common_lerp(0.9, 1.2, duel),
            };

            let complexity = match t {
                DribbleTechnique::Shielding => 0.25,
                DribbleTechnique::Turn => 0.40,
                DribbleTechnique::FaceUp => 0.20,
                DribbleTechnique::KnockOn => 0.40,
                DribbleTechnique::Feint => 0.55,
                DribbleTechnique::Hesitation => 0.50,
            };

            let pressure_resist = 0.55 * comp + 0.45 * dec;
            w *= common_lerp(
                1.0,
                0.45,
                clamp01(pressure * (1.0 + complexity) * (1.0 - pressure_resist)),
            );

            weights.push(w.max(0.001));
        }

        let idx = weighted_choice_index(&weights, rng);
        candidates[idx]
    }

    fn base_success_prob(technique: Self::Technique, skills: Self::Skills, pressure: f32) -> f32 {
        let pressure = clamp01(pressure);

        let drib = skill01(skills.dribbling);
        let tech = skill01(skills.technique);
        let agi = skill01(skills.agility);
        let dec = skill01(skills.decisions);
        let comp = skill01(skills.composure);

        let difficulty = match technique {
            DribbleTechnique::Shielding => 0.25,
            DribbleTechnique::Turn => 0.40,
            DribbleTechnique::FaceUp => 0.20,
            DribbleTechnique::KnockOn => 0.50,
            DribbleTechnique::Feint => 0.55,
            DribbleTechnique::Hesitation => 0.52,
        };

        let skill = match technique {
            DribbleTechnique::Shielding => 0.45 * comp + 0.30 * drib + 0.25 * dec,
            DribbleTechnique::Turn => 0.45 * drib + 0.30 * agi + 0.25 * tech,
            DribbleTechnique::FaceUp => 0.40 * tech + 0.35 * drib + 0.25 * dec,
            DribbleTechnique::KnockOn => 0.45 * drib + 0.35 * tech + 0.20 * agi,
            DribbleTechnique::Feint => 0.35 * tech + 0.35 * agi + 0.30 * comp,
            DribbleTechnique::Hesitation => 0.35 * tech + 0.35 * agi + 0.30 * dec,
        };

        let press_resist = 0.55 * comp + 0.45 * dec;

        let mut p = 0.60 + 0.55 * (skill - difficulty);
        p -= pressure * common_lerp(0.20, 0.08, press_resist);
        p.clamp(0.03, 0.97)
    }

    fn execution_error(
        technique: Self::Technique,
        skills: Self::Skills,
        pressure: f32,
    ) -> (f32, f32, f32) {
        let pressure = clamp01(pressure);

        let drib = skill01(skills.dribbling);
        let tech = skill01(skills.technique);
        let agi = skill01(skills.agility);
        let dec = skill01(skills.decisions);
        let comp = skill01(skills.composure);

        let control = 0.45 * drib + 0.30 * tech + 0.25 * agi;
        let calm = 0.60 * comp + 0.40 * dec;

        let (base_dir, base_speed, base_h) = match technique {
            DribbleTechnique::Shielding => (0.07, 0.05, 0.00),
            DribbleTechnique::Turn => (0.10, 0.07, 0.00),
            DribbleTechnique::FaceUp => (0.08, 0.06, 0.00),
            DribbleTechnique::KnockOn => (0.11, 0.10, 0.00),
            DribbleTechnique::Feint => (0.12, 0.08, 0.00),
            DribbleTechnique::Hesitation => (0.12, 0.09, 0.00),
        };

        let skill_factor = common_lerp(1.35, 0.60, clamp01(control));
        let press_factor = 1.0 + pressure * common_lerp(0.95, 0.35, clamp01(calm));

        (
            base_dir * skill_factor * press_factor,
            base_speed * skill_factor * press_factor,
            base_h * skill_factor * press_factor,
        )
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;
    const CY: f32 = field::CENTER_Y;

    #[test]
    fn test_dribble_action_creation() {
        let action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        assert_eq!(action.id, 1);
        assert_eq!(action.dribbler_idx, 5);
        assert!(matches!(action.phase, DribblePhase::Touch));
        assert_eq!(action.start_tick, 100);
        assert_eq!(action.phase_enter_tick, 100);
    }

    #[test]
    fn test_dribble_touch_to_carry() {
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        let player_pos = (50.0, CY);
        let ball_pos = (50.0, CY);

        // Touch 시 obs: 공간 적당, 치달 조건 불충분
        let obs = DribbleObs {
            sep: 0.0,
            defender_dist: 5.0, // 중간 거리
            space_ahead: 0.5,   // 공간 적당 (치달 조건 불충분)
            intercept_risk: 0.2,
            intent: DribbleIntent::Progress,
            can_evade: false,
            speed_ratio: 0.8,
            ..Default::default()
        };

        // Touch → Carry (should_knockon 조건 불충족 시)
        let (result, ball_vel, _player_vel) =
            action.update_tick_with_obs(player_pos, ball_pos, 101, &obs);

        assert!(matches!(result, DribbleResult::Continue));
        assert!(matches!(action.phase, DribblePhase::Carry));
        assert!(ball_vel.0 > 0.0); // 공이 앞으로 굴러감
    }

    #[test]
    fn test_dribble_carry_to_touch_via_condition() {
        // 조건 기반 전이: Carry에서 sep <= TOUCH_READY_DIST면 Touch로 전환
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        // 먼저 Touch → Carry
        let obs_touch = DribbleObs {
            sep: 0.5,
            defender_dist: 5.0,
            space_ahead: 0.5,
            intercept_risk: 0.2,
            intent: DribbleIntent::Progress,
            can_evade: false,
            speed_ratio: 0.8,
            ..Default::default()
        };
        let player_pos = (50.0, CY);
        let ball_pos = (50.5, CY);
        action.update_tick_with_obs(player_pos, ball_pos, 101, &obs_touch);
        assert!(matches!(action.phase, DribblePhase::Carry));

        // Carry에서 공이 발 앞에 도달 (sep <= TOUCH_READY_DIST)
        let obs_close = DribbleObs {
            sep: 0.3, // TOUCH_READY_DIST = 0.45 보다 작음
            defender_dist: 5.0,
            space_ahead: 0.5,
            intercept_risk: 0.2,
            intent: DribbleIntent::Progress,
            can_evade: false,
            speed_ratio: 0.8,
            ..Default::default()
        };
        let ball_pos_close = (50.3, CY);
        action.update_tick_with_obs(player_pos, ball_pos_close, 102, &obs_close);

        // sep <= TOUCH_READY_DIST → SyncBall로 전환
        assert!(matches!(action.phase, DribblePhase::SyncBall));
    }

    #[test]
    fn test_dribble_lost_when_ball_far() {
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        let player_pos = (50.0, CY);
        let ball_pos = (60.0, CY); // 공이 너무 멀리

        // Touch
        action.update_tick(player_pos, ball_pos, 100);

        // Carry에서 공이 너무 멀면 Lost
        let (result, _, _) = action.update_tick(player_pos, ball_pos, 101);

        assert!(matches!(result, DribbleResult::Lost { .. }));
        assert!(action.is_finished());
    }

    #[test]
    fn test_dribble_evade() {
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        // 회피 시작
        action.start_evade((0.0, 1.0), 101);

        assert!(matches!(action.phase, DribblePhase::Evade { .. }));

        // 회피 조건 기반: 압박 해소되면 Carry로 전환
        let player_pos = (50.0, CY);
        let ball_pos = (50.3, CY); // 공이 가까이 있음 (조건 충족)

        // 압박 해소된 obs로 업데이트
        let obs = DribbleObs {
            sep: 0.3,
            defender_dist: 10.0, // 수비 멀리
            space_ahead: 0.8,
            intercept_risk: 0.1,
            intent: DribbleIntent::Progress,
            can_evade: false, // 회피 필요 없음
            speed_ratio: 0.8,
            ..Default::default()
        };

        let _result = action.update_tick_with_obs(player_pos, ball_pos, 102, &obs);

        // 압박 해소 + 컨트롤 범위 → Carry 또는 Touch로 전환
        assert!(
            matches!(action.phase, DribblePhase::Carry)
                || matches!(action.phase, DribblePhase::Touch)
        );
    }

    #[test]
    fn test_should_trigger_evade() {
        let dribbler_pos = (50.0, CY);
        let dribble_dir = (1.0, 0.0);

        // 상대가 앞에 있으면
        let opponents = vec![(51.5, CY)];
        let evade = should_trigger_evade(dribbler_pos, &opponents, dribble_dir);
        assert!(evade.is_some());

        // 상대가 멀리 있으면
        let opponents_far = vec![(60.0, CY)];
        let evade_far = should_trigger_evade(dribbler_pos, &opponents_far, dribble_dir);
        assert!(evade_far.is_none());
    }

    #[test]
    fn test_dribble_gather_phase() {
        let mut action = DribbleAction::new_with_gather(1, 5, 0, (1.0, 0.0), false, 100);

        assert!(matches!(action.phase, DribblePhase::Gather));

        let player_pos = (50.0, CY);
        let ball_pos = (50.3, CY); // 가까이 있음

        // Gather → Touch (공이 컨트롤 범위에 들어오면)
        let obs = DribbleObs {
            sep: 0.3, // TOUCH_READY_DIST + 0.1 = 0.55 보다 작음
            defender_dist: 5.0,
            space_ahead: 0.5,
            intercept_risk: 0.2,
            intent: DribbleIntent::Progress,
            can_evade: false,
            speed_ratio: 0.8,
            ..Default::default()
        };
        action.update_tick_with_obs(player_pos, ball_pos, 101, &obs);

        // 공이 컨트롤 범위 → Touch로 전환
        assert!(matches!(action.phase, DribblePhase::Touch));
    }

    #[test]
    fn test_dribble_phase_names() {
        assert_eq!(DribblePhase::Touch.name(), "Touch");
        assert_eq!(DribblePhase::Carry.name(), "Carry");
        assert_eq!(DribblePhase::SyncBall.name(), "SyncBall");
        assert_eq!(DribblePhase::Finished.name(), "Finished");
    }

    #[test]
    fn test_can_start_dribble() {
        // 공이 가까이 있고 소유권 있음
        assert!(can_start_dribble((50.0, CY), (50.5, CY), true));

        // 소유권 없음
        assert!(!can_start_dribble((50.0, CY), (50.5, CY), false));

        // 공이 너무 멀리
        assert!(!can_start_dribble((50.0, CY), (60.0, CY), true));
    }

    #[test]
    fn test_full_dribble_cycle_with_obs() {
        // 조건 기반 전이를 통한 전체 드리블 사이클 테스트
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        let mut player_pos = (50.0, CY);
        let mut ball_pos = (50.0, CY);

        let mut phases_seen = vec![];
        let mut last_phase = "";

        // 15틱 동안 드리블
        for tick_offset in 0..15 {
            if action.is_finished() {
                break;
            }

            let current_phase = action.phase.name();
            if current_phase != last_phase {
                phases_seen.push(current_phase.to_string());
                last_phase = current_phase;
            }

            // sep은 공-선수 거리
            let sep = distance(ball_pos, player_pos);

            // 조건 기반 obs 생성
            let obs = DribbleObs {
                sep,
                defender_dist: 5.0, // 적당한 거리
                space_ahead: 0.5,
                intercept_risk: 0.2,
                intent: DribbleIntent::Progress,
                can_evade: false,
                speed_ratio: 0.8,
                ..Default::default()
            };

            let current_tick = 101 + tick_offset as u64;
            let (result, ball_vel, player_vel) =
                action.update_tick_with_obs(player_pos, ball_pos, current_tick, &obs);

            if matches!(result, DribbleResult::Lost { .. }) {
                break;
            }

            player_pos.0 += player_vel.0;
            player_pos.1 += player_vel.1;
            ball_pos.0 += ball_vel.0;
            ball_pos.1 += ball_vel.1;
        }

        // Touch가 나타나야 함 (시작 페이즈)
        assert!(phases_seen.contains(&"Touch".to_string()));
        // 조건 기반이므로 Carry 또는 다른 페이즈가 나타날 수 있음
        // (공간/위협에 따라 다름)
    }

    #[test]
    fn test_normalize() {
        let v = normalize((3.0, 4.0));
        let len = (v.0 * v.0 + v.1 * v.1).sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_distance() {
        let d = distance((0.0, 0.0), (3.0, 4.0));
        assert!((d - 5.0).abs() < 0.001);
    }

    // ========================================================================
    // P7-DRIBBLE-TUNE: Intent/Technique Tests
    // ========================================================================

    fn make_test_attacker() -> PlayerAttributes {
        PlayerAttributes {
            dribbling: 75,
            flair: 70,
            technique: 72,
            pace: 80,
            acceleration: 78,
            agility: 75,
            balance: 70,
            strength: 68,
            composure: 72,
            // 기본값
            ..PlayerAttributes::default()
        }
    }

    fn make_test_defender() -> PlayerAttributes {
        PlayerAttributes {
            tackling: 75,
            anticipation: 70,
            concentration: 68,
            aggression: 72,
            strength: 75,
            pace: 72,
            acceleration: 70,
            ..PlayerAttributes::default()
        }
    }

    #[test]
    fn test_intent_available_techniques() {
        // Protect intent
        let protect_techs = DribbleIntent::Protect.available_techniques();
        assert!(protect_techs.contains(&DribbleTechnique::Shielding));
        assert!(protect_techs.contains(&DribbleTechnique::Turn));

        // Progress intent
        let progress_techs = DribbleIntent::Progress.available_techniques();
        assert!(progress_techs.contains(&DribbleTechnique::FaceUp));
        assert!(progress_techs.contains(&DribbleTechnique::KnockOn));

        // Beat intent
        let beat_techs = DribbleIntent::Beat.available_techniques();
        assert!(beat_techs.contains(&DribbleTechnique::Feint));
        assert!(beat_techs.contains(&DribbleTechnique::KnockOn));
        assert!(beat_techs.contains(&DribbleTechnique::Hesitation));
        assert!(beat_techs.contains(&DribbleTechnique::Turn));
    }

    #[test]
    fn test_technique_physics_params() {
        // Shielding - 느림, 가까움
        assert_eq!(DribbleTechnique::Shielding.base_speed(), 0.0);
        assert!(DribbleTechnique::Shielding.ball_distance() < 0.5);

        // KnockOn - 빠름, 멀리
        assert!(DribbleTechnique::KnockOn.base_speed() > 8.0);
        assert!(DribbleTechnique::KnockOn.ball_distance() > 3.0);

        // 리스크 순서: FaceUp < Shielding < Turn < Hesitation < Feint < KnockOn
        assert!(
            DribbleTechnique::FaceUp.base_loss_risk()
                < DribbleTechnique::Shielding.base_loss_risk()
        );
        assert!(
            DribbleTechnique::Shielding.base_loss_risk()
                < DribbleTechnique::KnockOn.base_loss_risk()
        );
    }

    #[test]
    fn test_choose_technique_protect() {
        let attacker = make_test_attacker();
        let defender = make_test_defender();

        let selection =
            choose_dribble_technique(DribbleIntent::Protect, &attacker, Some(&defender), false);

        // Protect은 Shielding 또는 Turn 선택
        assert!(matches!(
            selection.technique,
            DribbleTechnique::Shielding | DribbleTechnique::Turn
        ));
        assert!(selection.success_chance > 0.0 && selection.success_chance < 1.0);
    }

    #[test]
    fn test_choose_technique_progress_no_space() {
        let attacker = make_test_attacker();
        let defender = make_test_defender();

        // 공간이 없으면 FaceUp 선택
        let selection = choose_dribble_technique(
            DribbleIntent::Progress,
            &attacker,
            Some(&defender),
            false, // no space
        );

        assert!(matches!(selection.technique, DribbleTechnique::FaceUp));
    }

    #[test]
    fn test_choose_technique_beat() {
        let attacker = make_test_attacker();
        let defender = make_test_defender();

        let selection =
            choose_dribble_technique(DribbleIntent::Beat, &attacker, Some(&defender), true);

        // Beat은 4가지 중 하나 선택
        assert!(matches!(
            selection.technique,
            DribbleTechnique::Feint
                | DribbleTechnique::KnockOn
                | DribbleTechnique::Hesitation
                | DribbleTechnique::Turn
        ));
    }

    #[test]
    fn test_resolve_faceup_usually_succeeds() {
        let attacker = make_test_attacker();

        // FaceUp은 95% 성공
        let success_count = (0..100)
            .filter(|i| {
                let roll = *i as f32 / 100.0;
                matches!(
                    resolve_technique(
                        DribbleTechnique::FaceUp,
                        &attacker,
                        None,
                        roll,
                        (50.0, CY),
                        (1.0, 0.0)
                    ),
                    TechniqueResult::Success { .. }
                )
            })
            .count();

        assert!(success_count >= 90, "FaceUp should succeed ~95% of time");
    }

    #[test]
    fn test_resolve_knockon_creates_space() {
        let attacker = make_test_attacker();
        let defender = make_test_defender();

        let result = resolve_technique(
            DribbleTechnique::KnockOn,
            &attacker,
            Some(&defender),
            0.5, // 중간 롤
            (50.0, CY),
            (1.0, 0.0),
        );

        // 성공 시 공간 생성
        if let TechniqueResult::Success { space_gained, .. } = result {
            assert!(space_gained >= 4.0);
        }
    }

    #[test]
    fn test_resolve_feint_changes_direction() {
        let attacker = make_test_attacker();

        // 높은 스탯으로 성공 확률 높임
        let result = resolve_technique(
            DribbleTechnique::Feint,
            &attacker,
            None, // 약한 수비
            0.3,  // 낮은 롤 (더 쉬움)
            (50.0, CY),
            (1.0, 0.0),
        );

        if let TechniqueResult::Success { new_direction, .. } = result {
            // 페인트 성공 시 방향 변경 (수직 방향)
            // 원래 방향 (1,0)에서 (0,1) 또는 (0,-1)로
            assert!(new_direction.1.abs() > 0.5);
        }
    }

    #[test]
    fn test_start_technique_phase() {
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        action.start_technique(DribbleTechnique::Feint, 101);

        assert!(action.is_executing_technique());
        assert!(matches!(
            action.phase,
            DribblePhase::ExecutingTechnique { technique: DribbleTechnique::Feint, .. }
        ));
        assert_eq!(action.current_technique, Some(DribbleTechnique::Feint));
    }

    #[test]
    fn test_knock_and_run_phase() {
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        action.start_knock_and_run(5.0, 101);

        assert!(action.is_knock_and_run());
        assert!(matches!(action.phase, DribblePhase::KnockAndRun { knock_distance: 5.0, .. }));
    }

    #[test]
    fn test_technique_execution_completes() {
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        // Shielding 시작 (duration 2 ticks)
        action.start_technique(DribbleTechnique::Shielding, 101);
        // 성공 판정 설정
        action.set_technique_result(true);

        let player_pos = (50.0, CY);
        let ball_pos = (50.0, CY);

        // update_tick_with_obs로 조건 기반 업데이트
        // success = Some(true)이면 즉시 Touch로 전환됨
        let obs = DribbleObs::default();
        action.update_tick_with_obs(player_pos, ball_pos, 102, &obs);

        // 성공 판정 후 Touch로 복귀
        assert!(matches!(action.phase, DribblePhase::Touch));
    }

    #[test]
    fn test_deception_types() {
        assert_eq!(DribbleTechnique::Feint.deception_type(), Some(DeceptionType::Direction));
        assert_eq!(DribbleTechnique::Hesitation.deception_type(), Some(DeceptionType::Tempo));
        assert_eq!(DribbleTechnique::KnockOn.deception_type(), Some(DeceptionType::Speed));
        assert_eq!(DribbleTechnique::Shielding.deception_type(), None);
    }

    #[test]
    fn test_dribble_touch_event_emission() {
        // DribbleTouchEvent가 Touch phase에서 생성되는지 테스트
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        // 초기 상태: pending_viewer_event 없음
        assert!(action.pending_viewer_event.is_none());

        let player_pos = (50.0, CY);
        let ball_pos = (50.0, CY);

        let obs = DribbleObs {
            sep: 0.0,
            defender_dist: 5.0,
            space_ahead: 0.5,
            intercept_risk: 0.2,
            intent: DribbleIntent::Progress,
            can_evade: false,
            speed_ratio: 0.8,
            ..Default::default()
        };

        // Touch phase에서 update 실행
        action.update_tick_with_obs(player_pos, ball_pos, 101, &obs);

        // DribbleTouchEvent가 생성되어야 함
        assert!(action.pending_viewer_event.is_some());

        let event = action.take_viewer_event().unwrap();
        assert_eq!(event.dribbler_track_id, 5); // dribbler_idx as u32
        assert_eq!(event.touch_type, DribbleTouchType::Carry);
        assert_eq!(event.t_ms, 101 * 250); // tick → ms
        assert!(event.lock_ms > 0);
    }

    #[test]
    fn test_dribble_knockon_event() {
        // KnockOn 터치 이벤트가 제대로 생성되는지 테스트
        let mut action = DribbleAction::new(1, 5, 0, (1.0, 0.0), false, 100);

        let player_pos = (50.0, CY);
        let ball_pos = (50.0, CY);

        // KnockOn 조건: space_ahead >= 0.65, defender_dist > 2.0, speed_ratio >= 0.8
        let obs = DribbleObs {
            sep: 0.0,
            defender_dist: 10.0, // 수비 멀리
            space_ahead: 0.8,    // 공간 넓음
            intercept_risk: 0.1,
            intent: DribbleIntent::Progress,
            can_evade: false,
            speed_ratio: 0.85, // 빠르게 달리는 중
            ..Default::default()
        };

        action.update_tick_with_obs(player_pos, ball_pos, 101, &obs);

        // KnockOn 조건 충족 시 KnockOn 터치 이벤트
        let event = action.take_viewer_event().unwrap();
        assert_eq!(event.touch_type, DribbleTouchType::KnockOn);

        // Phase도 KnockAndRun으로 전환되어야 함
        assert!(matches!(action.phase, DribblePhase::KnockAndRun { .. }));
    }

    #[test]
    fn test_dribble_first_touch_event() {
        // Gather → Touch 전환 시 FirstTouch 이벤트 생성
        let mut action = DribbleAction::new_with_gather(1, 5, 0, (1.0, 0.0), false, 100);

        let player_pos = (50.0, CY);
        let ball_pos = (50.2, CY); // 가까이

        let obs = DribbleObs {
            sep: 0.2, // TOUCH_READY_DIST보다 작음
            defender_dist: 5.0,
            space_ahead: 0.5,
            intercept_risk: 0.2,
            intent: DribbleIntent::Progress,
            can_evade: false,
            speed_ratio: 0.8,
            ..Default::default()
        };

        // Gather → Touch 전환
        action.update_tick_with_obs(player_pos, ball_pos, 101, &obs);

        // FirstTouch 이벤트가 생성되어야 함
        let event = action.take_viewer_event().unwrap();
        assert_eq!(event.touch_type, DribbleTouchType::FirstTouch);
    }
}
