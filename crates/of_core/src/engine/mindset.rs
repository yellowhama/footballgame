//! P14: Player Mindset System
//!
//! 마인드셋 시스템은 **상황별 의사결정 구조(Decision Topology)**를 정의합니다.
//! 마인드셋은 **"행동 후보를 제한하는 문지기"** 역할을 하며, 그 아래에서 EV/Audacity/Error가 선택·실행합니다.
//!
//! > **"행동을 나눈 게 아니라, '선수가 무엇을 보고 있느냐'를 나눈 구조다."**
//!
//! ## 마인드셋 역할
//! - 행동 후보를 제한 (Candidate Set)
//! - 행동별 가중치(prior) 제공
//! - EV/Audacity/Error는 후보 안에서만 작동
//!
//! ## 모듈 구조
//! - `PlayerMindset`: 개인 마인드셋 enum
//! - `CandidateAction`: 후보 행동 enum
//! - `build_candidates()`: 마인드셋 기반 후보 생성
//! - `determine_player_mindset()`: 상황 기반 마인드셋 결정

use crate::engine::physics_constants::field;
use crate::engine::team_phase::TeamPhase;
use serde::{Deserialize, Serialize};

// ============================================================================
// 마인드셋 상수
// ============================================================================

/// 패스 받기 위한 최소 거리 (m)
pub const RECEIVE_BALL_MIN_DISTANCE: f32 = 3.0;
/// 패스 받기 위한 최대 거리 (m)
pub const RECEIVE_BALL_MAX_DISTANCE: f32 = 35.0;
/// 침투 런을 위한 최소 전진 거리 (상대 골라인에서 m)
pub const PENETRATE_LINE_THRESHOLD: f32 = 35.0;
/// 폭 유지를 위한 터치라인 거리 (m)
pub const HOLD_WIDTH_TOUCHLINE_THRESHOLD: f32 = 10.0;
/// 수비 복귀를 위한 후방 임계값 (자기 골라인에서 m)
pub const REST_DEFENSE_THRESHOLD: f32 = 40.0;

/// 압박 범위 (m)
pub const PRESSER_RANGE: f32 = 8.0;
/// 컨테인 범위 (m)
pub const CONTAIN_RANGE: f32 = 15.0;
/// 마킹 범위 (m)
pub const MARK_RANGE: f32 = 12.0;
/// 슈팅 블록 범위 (m)
pub const BLOCK_SHOT_RANGE: f32 = 20.0;

/// 전환 상태 지속 시간 (틱)
pub const TRANSITION_DURATION_TICKS: u64 = 30;

// ============================================================================
// PlayerMindset - 개인 마인드셋
// ============================================================================

/// 플레이어 마인드셋 (개인 상태/인지)
///
/// 마인드셋은 선수가 현재 상황을 어떻게 인식하고 있는지를 나타냅니다.
/// 이에 따라 가능한 행동 후보가 결정됩니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PlayerMindset {
    // ========== 공격 ==========
    /// 내가 공 소유 - 드리블/패스/슈팅 결정
    #[default]
    AttackOnBall,
    /// 내가 공 없음 - 공간 침투/지원/폭 유지
    AttackOffBall,
    /// 패스 타겟으로 선정됨 - 첫 터치 결정
    AttackReceiveBall,
    /// 공 주변 삼각형 옵션 제공
    AttackSupport,
    /// 오프사이드 라인 뒤 침투
    AttackPenetrate,
    /// 터치라인 쪽 폭 넓히기
    AttackHoldWidth,
    /// 후방 균형 유지 (역습 대비)
    AttackRestDefense,

    // ========== 수비 ==========
    /// 공에 직접 압박 (사냥개)
    DefendPresser,
    /// 지연/컨테인 (Jockey)
    DefendContain,
    /// 마킹 (패스 차단)
    DefendMark,
    /// 라인 유지/뒷공간 커버
    DefendCover,
    /// 슈팅 라인 블록
    DefendBlockShot,
    /// 침투 추적 (오프볼 공격수)
    DefendTrackRunner,
    /// 위험지역에서 걷어내기
    DefendClearance,

    // ========== 전환 공격 (역습) ==========
    /// 공 잡은 애가 바로 전진
    TransitionCarry,
    /// 첫 패스 (탈압박/전환)
    TransitionOutletPass,
    /// 침투 러너들 전력질주
    TransitionSprintRun,
    /// 가까운 삼각형 제공
    TransitionSupportTriangle,

    // ========== 전환 수비 (게겐프레싱) ==========
    /// 근처 3명이 즉시 압박
    TransitionCounterpress,
    /// 나머지는 라인 복귀
    TransitionRecoverShape,
    /// 패스 길 차단
    TransitionCutPassLane,

    // ========== 세트피스 공격 ==========
    /// 킥오프 (백패스/롱볼)
    SetPieceKickoff,
    /// 프리킥 (직접슛/크로스/숏패스)
    SetPieceFreeKick,
    /// 코너킥 (인스윙/아웃스윙/숏코너)
    SetPieceCorner,
    /// 스로인 (숏/롱)
    SetPieceThrow,
    /// 페널티 (배치샷/파워샷/파넨카)
    SetPiecePenalty,

    // ========== 세트피스 수비 ==========
    /// 지역 방어
    SetPieceDefendZonal,
    /// 맨마킹
    SetPieceDefendManMark,
    /// 첫 접촉 클리어
    SetPieceDefendClear,
    /// GK 펀칭/캐칭
    SetPieceDefendGK,

    // ========== GK 전용 ==========
    /// 골키퍼 - 세이브 대기
    GKPositioning,
    /// 골키퍼 - 세이브 시도
    GKSave,
    /// 골키퍼 - 공 배급
    GKDistribution,
}

impl PlayerMindset {
    /// 공격적인 마인드셋인지
    pub fn is_attacking(&self) -> bool {
        matches!(
            self,
            PlayerMindset::AttackOnBall
                | PlayerMindset::AttackOffBall
                | PlayerMindset::AttackReceiveBall
                | PlayerMindset::AttackSupport
                | PlayerMindset::AttackPenetrate
                | PlayerMindset::AttackHoldWidth
                | PlayerMindset::AttackRestDefense
                | PlayerMindset::TransitionCarry
                | PlayerMindset::TransitionOutletPass
                | PlayerMindset::TransitionSprintRun
                | PlayerMindset::TransitionSupportTriangle
        )
    }

    /// 수비적인 마인드셋인지
    pub fn is_defending(&self) -> bool {
        matches!(
            self,
            PlayerMindset::DefendPresser
                | PlayerMindset::DefendContain
                | PlayerMindset::DefendMark
                | PlayerMindset::DefendCover
                | PlayerMindset::DefendBlockShot
                | PlayerMindset::DefendTrackRunner
                | PlayerMindset::DefendClearance
                | PlayerMindset::TransitionCounterpress
                | PlayerMindset::TransitionRecoverShape
                | PlayerMindset::TransitionCutPassLane
        )
    }

    /// 전환 상태인지
    pub fn is_transition(&self) -> bool {
        matches!(
            self,
            PlayerMindset::TransitionCarry
                | PlayerMindset::TransitionOutletPass
                | PlayerMindset::TransitionSprintRun
                | PlayerMindset::TransitionSupportTriangle
                | PlayerMindset::TransitionCounterpress
                | PlayerMindset::TransitionRecoverShape
                | PlayerMindset::TransitionCutPassLane
        )
    }

    /// 세트피스 상태인지
    pub fn is_set_piece(&self) -> bool {
        matches!(
            self,
            PlayerMindset::SetPieceKickoff
                | PlayerMindset::SetPieceFreeKick
                | PlayerMindset::SetPieceCorner
                | PlayerMindset::SetPieceThrow
                | PlayerMindset::SetPiecePenalty
                | PlayerMindset::SetPieceDefendZonal
                | PlayerMindset::SetPieceDefendManMark
                | PlayerMindset::SetPieceDefendClear
                | PlayerMindset::SetPieceDefendGK
        )
    }

    /// GK 전용 마인드셋인지
    pub fn is_goalkeeper(&self) -> bool {
        matches!(
            self,
            PlayerMindset::GKPositioning | PlayerMindset::GKSave | PlayerMindset::GKDistribution
        )
    }

    /// 온볼 상태인지
    pub fn is_on_ball(&self) -> bool {
        matches!(
            self,
            PlayerMindset::AttackOnBall
                | PlayerMindset::TransitionCarry
                | PlayerMindset::TransitionOutletPass
                | PlayerMindset::GKDistribution
        )
    }
}

// ============================================================================
// CandidateAction - 후보 행동
// ============================================================================

/// 후보 행동 enum
///
/// 마인드셋에 따라 선택 가능한 행동들의 집합입니다.
/// 각 행동은 EV 계산 후 최종 선택됩니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateAction {
    // ========== 드리블 ==========
    /// 키핑/턴/시간벌기
    DribbleKeep,
    /// 운반 (공간으로 몰고가기)
    DribbleCarry,
    /// 돌파 (1:1/1:2 도박)
    DribbleTakeOn,
    /// 안으로 방향 전환
    DribbleCutInside,
    /// 밖으로 방향 전환
    DribbleCutOutside,
    /// 몸으로 보호
    ShieldBall,

    // ========== 패스 ==========
    /// 숏패스
    PassShort,
    /// 백패스 (리셋/안전)
    PassBack,
    /// 사이드체인지
    PassSwitch,
    /// 스루패스
    PassThrough,
    /// 로빙/칩 패스
    PassLob,
    /// 얼리 크로스
    CrossEarly,
    /// 바이라인 크로스
    CrossByline,
    /// 컷백
    Cutback,

    // ========== 슈팅 ==========
    /// 일반 슈팅
    ShotNormal,
    /// 감아차기
    ShotFinesse,
    /// 강슛
    ShotPower,
    /// 칩샷
    ShotChip,
    /// 발리
    ShotVolley,
    /// 원터치 슛
    ShotFirstTime,

    // ========== 공 소유 유지 ==========
    /// 잠깐 멈추고 판단
    HoldBall,
    /// 파울 유도
    DrawFoul,

    // ========== 오프볼 공격 ==========
    /// 넓은 공간 침투
    RunIntoSpace,
    /// 오프사이드 라인 타기
    RunBehindLine,
    /// 대각 침투
    RunDiagonal,
    /// 반대 포스트로 이동
    AttackBackPost,
    /// 공 받으러 다가오기
    ShowForBall,
    /// 내려와서 연결
    DropDeep,
    /// 좌우로 벌려 패스각 만들기
    MoveLateral,
    /// 공 달라고 요청
    CallForBall,
    /// 여기로 찔러! 지시
    PointRun,
    /// 미끼 침투
    DummyRun,
    /// 측면 벌려주기
    HoldWidth,
    /// 후방에 남기
    RestDefenseAnchor,

    // ========== 공 받기 ==========
    /// 잡고 키핑
    FirstTouchControl,
    /// 미스컨트롤
    FirstTouchHeavy,
    /// 흘리기/더미
    DummyLetRun,
    /// 원터치 패스
    OneTouchPass,
    /// 원터치 슛
    OneTouchShot,
    /// 벽패스
    OneTouchLayoff,

    // ========== 수비 - 압박/접근 ==========
    /// 거리 좁히기
    CloseDown,
    /// 간격 유지하며 방향만 막기
    Jockey,
    /// 측면으로 몰기
    ForceWide,
    /// 중앙으로 몰기
    ForceInside,

    // ========== 수비 - 태클/인터셉트 ==========
    /// 스탠딩 태클
    TackleStand,
    /// 슬라이딩 태클
    TackleSlide,
    /// 패스 라인 차단
    InterceptLine,
    /// 슈팅/패스 블록
    Block,

    // ========== 수비 - 커버/라인 ==========
    /// 라인 내리기
    DropLine,
    /// 오프사이드 트랩
    StepUp,
    /// 공쪽으로 팀 슬라이드
    ShiftTeam,

    // ========== 수비 - 볼 처리 ==========
    /// 멀리 클리어
    ClearLong,
    /// 사이드로 클리어
    ClearWide,
    /// 안전하게 빌드업 시작
    PassOut,

    // ========== 전환 공격 ==========
    /// 빠른 운반
    CarryFast,
    /// 빠른 스루패스
    PassThroughEarly,
    /// 측면 릴리스
    PassWideRelease,
    /// 빠른 슈팅
    ShotEarly,
    /// 측면 전력질주
    RunWide,

    // ========== 전환 수비 ==========
    /// 공으로 전력질주
    SprintToBall,
    /// 아웃렛 패스 차단
    BlockOutlet,
    /// 백패스 강요
    ForceBackpass,
    /// 물러나서 리셋
    DropAndReset,

    // ========== 세트피스 공격 ==========
    /// 안전한 백패스
    KickoffSafeBackpass,
    /// 롱볼
    KickoffLongBall,
    /// 직접 프리킥
    FKDirectShot,
    /// 프리킥 크로스
    FKCross,
    /// 프리킥 숏패스
    FKShortPass,
    /// 인스윙 코너
    CornerInswing,
    /// 아웃스윙 코너
    CornerOutswing,
    /// 숏코너
    CornerShort,
    /// 니어포스트 코너
    CornerNearPost,
    /// 파포스트 코너
    CornerFarPost,
    /// 숏스로인
    ThrowShort,
    /// 롱스로인
    ThrowLong,
    /// PK 배치샷
    PKPlaced,
    /// PK 파워샷
    PKPower,
    /// PK 파넨카
    PKPanenka,

    // ========== 세트피스 수비 ==========
    /// 지역 방어
    DefSetPieceZonal,
    /// 맨마킹
    DefSetPieceManMark,
    /// 첫 접촉 클리어
    DefClearFirstContact,
    /// GK 펀칭/캐칭
    DefGKClaim,

    // ========== GK ==========
    /// 위치 조정
    GKPosition,
    /// 세이브 시도
    GKSaveAttempt,
    /// 펀칭
    GKPunch,
    /// 캐칭
    GKCatch,
    /// 롱킥 배급
    GKDistributeLong,
    /// 숏패스 배급
    GKDistributeShort,
    /// 스로 배급
    GKDistributeThrow,

    // ========== 특수 ==========
    /// 대기 (아무것도 안함)
    Wait,
    /// 포메이션 위치로 이동
    MoveToFormation,
}

impl CandidateAction {
    /// 행동 그룹 반환
    pub fn group(&self) -> CandidateGroup {
        match self {
            // 드리블
            CandidateAction::DribbleKeep
            | CandidateAction::DribbleCarry
            | CandidateAction::DribbleTakeOn
            | CandidateAction::DribbleCutInside
            | CandidateAction::DribbleCutOutside
            | CandidateAction::ShieldBall => CandidateGroup::Dribble,

            // 패스
            CandidateAction::PassShort
            | CandidateAction::PassBack
            | CandidateAction::PassSwitch
            | CandidateAction::PassThrough
            | CandidateAction::PassLob
            | CandidateAction::CrossEarly
            | CandidateAction::CrossByline
            | CandidateAction::Cutback => CandidateGroup::Pass,

            // 슈팅
            CandidateAction::ShotNormal
            | CandidateAction::ShotFinesse
            | CandidateAction::ShotPower
            | CandidateAction::ShotChip
            | CandidateAction::ShotVolley
            | CandidateAction::ShotFirstTime => CandidateGroup::Shot,

            // 공 소유 유지
            CandidateAction::HoldBall | CandidateAction::DrawFoul => CandidateGroup::Possession,

            // 오프볼 공격
            CandidateAction::RunIntoSpace
            | CandidateAction::RunBehindLine
            | CandidateAction::RunDiagonal
            | CandidateAction::AttackBackPost
            | CandidateAction::ShowForBall
            | CandidateAction::DropDeep
            | CandidateAction::MoveLateral
            | CandidateAction::CallForBall
            | CandidateAction::PointRun
            | CandidateAction::DummyRun
            | CandidateAction::HoldWidth
            | CandidateAction::RestDefenseAnchor => CandidateGroup::OffBallRun,

            // 공 받기
            CandidateAction::FirstTouchControl
            | CandidateAction::FirstTouchHeavy
            | CandidateAction::DummyLetRun
            | CandidateAction::OneTouchPass
            | CandidateAction::OneTouchShot
            | CandidateAction::OneTouchLayoff => CandidateGroup::ReceiveBall,

            // 수비
            CandidateAction::CloseDown
            | CandidateAction::Jockey
            | CandidateAction::ForceWide
            | CandidateAction::ForceInside
            | CandidateAction::TackleStand
            | CandidateAction::TackleSlide
            | CandidateAction::InterceptLine
            | CandidateAction::Block
            | CandidateAction::DropLine
            | CandidateAction::StepUp
            | CandidateAction::ShiftTeam
            | CandidateAction::ClearLong
            | CandidateAction::ClearWide
            | CandidateAction::PassOut => CandidateGroup::Defend,

            // 전환 공격
            CandidateAction::CarryFast
            | CandidateAction::PassThroughEarly
            | CandidateAction::PassWideRelease
            | CandidateAction::ShotEarly
            | CandidateAction::RunWide => CandidateGroup::TransitionAttack,

            // 전환 수비
            CandidateAction::SprintToBall
            | CandidateAction::BlockOutlet
            | CandidateAction::ForceBackpass
            | CandidateAction::DropAndReset => CandidateGroup::TransitionDefend,

            // 세트피스
            CandidateAction::KickoffSafeBackpass
            | CandidateAction::KickoffLongBall
            | CandidateAction::FKDirectShot
            | CandidateAction::FKCross
            | CandidateAction::FKShortPass
            | CandidateAction::CornerInswing
            | CandidateAction::CornerOutswing
            | CandidateAction::CornerShort
            | CandidateAction::CornerNearPost
            | CandidateAction::CornerFarPost
            | CandidateAction::ThrowShort
            | CandidateAction::ThrowLong
            | CandidateAction::PKPlaced
            | CandidateAction::PKPower
            | CandidateAction::PKPanenka
            | CandidateAction::DefSetPieceZonal
            | CandidateAction::DefSetPieceManMark
            | CandidateAction::DefClearFirstContact
            | CandidateAction::DefGKClaim => CandidateGroup::SetPiece,

            // GK
            CandidateAction::GKPosition
            | CandidateAction::GKSaveAttempt
            | CandidateAction::GKPunch
            | CandidateAction::GKCatch
            | CandidateAction::GKDistributeLong
            | CandidateAction::GKDistributeShort
            | CandidateAction::GKDistributeThrow => CandidateGroup::Goalkeeper,

            // 특수
            CandidateAction::Wait | CandidateAction::MoveToFormation => CandidateGroup::Special,
        }
    }

    /// 온볼 행동인지
    pub fn is_on_ball(&self) -> bool {
        matches!(
            self.group(),
            CandidateGroup::Dribble
                | CandidateGroup::Pass
                | CandidateGroup::Shot
                | CandidateGroup::Possession
                | CandidateGroup::ReceiveBall
        )
    }
}

/// 후보 행동 그룹 (로깅/튜닝용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateGroup {
    Dribble,
    Pass,
    Shot,
    Possession,
    OffBallRun,
    ReceiveBall,
    Defend,
    TransitionAttack,
    TransitionDefend,
    SetPiece,
    Goalkeeper,
    Special,
}

// ============================================================================
// MindsetContext - 마인드셋 결정 컨텍스트
// ============================================================================

/// 마인드셋 결정에 필요한 컨텍스트
#[derive(Debug, Clone)]
pub struct MindsetContext {
    /// 이 선수가 공을 가지고 있는지
    pub has_ball: bool,
    /// 이 선수가 패스 타겟인지
    pub is_pass_target: bool,
    /// 팀 페이즈
    pub team_phase: TeamPhase,
    /// 선수 위치 (x, y) - 피치 좌표
    pub position: (f32, f32),
    /// 공 위치 (x, y)
    pub ball_position: (f32, f32),
    /// 공까지 거리 (m)
    pub distance_to_ball: f32,
    /// 상대 골대까지 거리 (m)
    pub distance_to_goal: f32,
    /// 자기 골대까지 거리 (m)
    pub distance_to_own_goal: f32,
    /// 터치라인까지 거리 (m)
    pub distance_to_touchline: f32,
    /// 공 소유자까지 거리 (m) - 없으면 None
    pub distance_to_ball_owner: Option<f32>,
    /// GK인지
    pub is_goalkeeper: bool,
    /// 세트피스 상황인지
    pub is_set_piece: bool,
    /// 세트피스 킥커인지
    pub is_set_piece_kicker: bool,
    /// 현재 틱
    pub current_tick: u64,
    /// 페이즈 시작 틱
    pub phase_start_tick: u64,

    // ========================================================================
    // Tactical Knobs (P0 Patch 1) - Set by TeamInstructions in decision_topology
    // ========================================================================
    /// Pressing factor from TeamInstructions (0.0 = very low, 1.0 = very high)
    ///
    /// Affects pressing range and defensive behavior.
    pub pressing_factor: f32,

    /// Tempo factor from TeamInstructions (0.0 = very slow, 1.0 = very fast)
    ///
    /// Affects softmax temperature and decision timing.
    pub tempo_factor: f32,

    /// Width bias from TeamInstructions (-5.0 = very narrow, +5.0 = very wide)
    ///
    /// Affects player spacing and formation width.
    pub width_bias_m: f32,

    /// Risk bias from BuildUpStyle (-0.15 = short/conservative, +0.20 = direct/aggressive)
    ///
    /// Affects pass selection: negative = prefer safe passes, positive = prefer risky passes
    pub risk_bias: f32,
}

impl Default for MindsetContext {
    fn default() -> Self {
        Self {
            has_ball: false,
            is_pass_target: false,
            team_phase: TeamPhase::Defense,
            position: (field::CENTER_X, field::CENTER_Y),
            ball_position: (field::CENTER_X, field::CENTER_Y),
            distance_to_ball: 0.0,
            distance_to_goal: field::CENTER_X,
            distance_to_own_goal: field::CENTER_X,
            distance_to_touchline: field::CENTER_Y,
            distance_to_ball_owner: None,
            is_goalkeeper: false,
            is_set_piece: false,
            is_set_piece_kicker: false,
            current_tick: 0,
            phase_start_tick: 0,
            // Tactical knobs default to "Medium/Normal" values
            pressing_factor: 0.6, // Medium pressing
            tempo_factor: 0.6,    // Normal tempo
            width_bias_m: 0.0,    // Normal width
            risk_bias: 0.0,       // Mixed build-up (neutral)
        }
    }
}

// ============================================================================
// CandidateContext - 후보 생성 컨텍스트
// ============================================================================

/// 후보 생성에 필요한 상세 컨텍스트
#[derive(Debug, Clone)]
pub struct CandidateContext {
    /// 1:1 상황인지 (근처에 수비수 1명만 있음)
    pub is_1v1_situation: bool,
    /// 슈팅 각도가 있는지
    pub has_shooting_angle: bool,
    /// 와이드 존인지 (측면)
    pub is_wide_zone: bool,
    /// 스루패스 옵션이 있는지
    pub has_through_pass_option: bool,
    /// 박스 안인지
    pub in_penalty_box: bool,
    /// 압박받고 있는지
    pub under_pressure: bool,
    /// 가까운 지원 옵션이 있는지
    pub has_close_support: bool,
    /// GK가 전진해 있는지
    pub gk_advanced: bool,
    /// 크로스 상황인지 (공이 측면에서 날아오고 있음)
    pub is_cross_situation: bool,
    /// 루즈볼 상황인지
    pub is_loose_ball: bool,
    /// 공격수 스킬 (Dribbling, Vision, Finishing 등 - 0~100)
    pub dribbling: u8,
    pub vision: u8,
    pub technique: u8,
    pub flair: u8,
    pub composure: u8,
    pub finishing: u8,
    pub long_shots: u8,
}

impl Default for CandidateContext {
    fn default() -> Self {
        Self {
            is_1v1_situation: false,
            has_shooting_angle: false,
            is_wide_zone: false,
            has_through_pass_option: false,
            in_penalty_box: false,
            under_pressure: false,
            has_close_support: true,
            gk_advanced: false,
            is_cross_situation: false,
            is_loose_ball: false,
            dribbling: 50,
            vision: 50,
            technique: 50,
            flair: 50,
            composure: 50,
            finishing: 50,
            long_shots: 50,
        }
    }
}

// ============================================================================
// 마인드셋 결정 함수
// ============================================================================

/// 상황에 따라 선수의 마인드셋을 결정합니다.
///
/// # Arguments
/// * `ctx` - 마인드셋 결정 컨텍스트
///
/// # Returns
/// 결정된 PlayerMindset
pub fn determine_player_mindset(ctx: &MindsetContext) -> PlayerMindset {
    // GK 전용 처리
    if ctx.is_goalkeeper {
        if ctx.has_ball {
            return PlayerMindset::GKDistribution;
        }
        return PlayerMindset::GKPositioning;
    }

    // 세트피스 처리
    if ctx.is_set_piece {
        return determine_set_piece_mindset(ctx);
    }

    // 팀 페이즈에 따른 분기
    match ctx.team_phase {
        TeamPhase::Attack => determine_attack_mindset(ctx),
        TeamPhase::Defense => determine_defend_mindset(ctx),
        TeamPhase::TransitionAttack => determine_transition_attack_mindset(ctx),
        TeamPhase::TransitionDefense => determine_transition_defend_mindset(ctx),
    }
}

/// 공격 마인드셋 결정
fn determine_attack_mindset(ctx: &MindsetContext) -> PlayerMindset {
    // 온볼
    if ctx.has_ball {
        return PlayerMindset::AttackOnBall;
    }

    // 패스 타겟
    if ctx.is_pass_target {
        return PlayerMindset::AttackReceiveBall;
    }

    // 오프볼 세부 분류
    // 1. 역습 대비 (후방에 남기)
    if ctx.distance_to_own_goal < REST_DEFENSE_THRESHOLD {
        return PlayerMindset::AttackRestDefense;
    }

    // 2. 침투 (오프사이드 라인 근처)
    if ctx.distance_to_goal < PENETRATE_LINE_THRESHOLD {
        return PlayerMindset::AttackPenetrate;
    }

    // 3. 폭 유지 (터치라인 근처)
    if ctx.distance_to_touchline < HOLD_WIDTH_TOUCHLINE_THRESHOLD {
        return PlayerMindset::AttackHoldWidth;
    }

    // 4. 지원 (공 근처)
    if ctx.distance_to_ball < CONTAIN_RANGE {
        return PlayerMindset::AttackSupport;
    }

    // 5. 기본 오프볼
    PlayerMindset::AttackOffBall
}

/// 수비 마인드셋 결정
fn determine_defend_mindset(ctx: &MindsetContext) -> PlayerMindset {
    // 1. 압박 (공에 가장 가까운 선수)
    if ctx.distance_to_ball < PRESSER_RANGE {
        return PlayerMindset::DefendPresser;
    }

    // 2. 컨테인 (압박 지원)
    if ctx.distance_to_ball < CONTAIN_RANGE {
        return PlayerMindset::DefendContain;
    }

    // 3. 슈팅 블록 (골대 근처)
    if ctx.distance_to_own_goal < BLOCK_SHOT_RANGE {
        return PlayerMindset::DefendBlockShot;
    }

    // 4. 마킹 (공격수 근처)
    if ctx.distance_to_ball_owner.is_some_and(|d| d < MARK_RANGE) {
        return PlayerMindset::DefendMark;
    }

    // 5. 커버 (기본)
    PlayerMindset::DefendCover
}

/// 전환 공격 마인드셋 결정 (역습)
fn determine_transition_attack_mindset(ctx: &MindsetContext) -> PlayerMindset {
    // 온볼: 빠른 전진 또는 아웃렛 패스
    if ctx.has_ball {
        // 전방에 공간이 있으면 캐리
        if ctx.distance_to_goal > 30.0 {
            return PlayerMindset::TransitionCarry;
        }
        // 아니면 아웃렛 패스
        return PlayerMindset::TransitionOutletPass;
    }

    // 오프볼: 침투 또는 삼각형 지원
    if ctx.distance_to_goal < 40.0 {
        return PlayerMindset::TransitionSprintRun;
    }

    PlayerMindset::TransitionSupportTriangle
}

/// 전환 수비 마인드셋 결정 (게겐프레싱)
fn determine_transition_defend_mindset(ctx: &MindsetContext) -> PlayerMindset {
    // 공 근처: 즉시 압박
    if ctx.distance_to_ball < PRESSER_RANGE * 1.5 {
        return PlayerMindset::TransitionCounterpress;
    }

    // 패스 라인 차단
    if ctx.distance_to_ball < CONTAIN_RANGE * 1.5 {
        return PlayerMindset::TransitionCutPassLane;
    }

    // 나머지: 복귀
    PlayerMindset::TransitionRecoverShape
}

/// 세트피스 마인드셋 결정
fn determine_set_piece_mindset(ctx: &MindsetContext) -> PlayerMindset {
    // 킥커인 경우
    if ctx.is_set_piece_kicker {
        // 위치에 따라 세트피스 종류 결정 (간단한 휴리스틱)
        if ctx.distance_to_goal < 25.0 {
            return PlayerMindset::SetPieceFreeKick;
        }
        if ctx.position.0 < 5.0 || ctx.position.0 > 100.0 {
            return PlayerMindset::SetPieceCorner;
        }
        if ctx.distance_to_touchline < 2.0 {
            return PlayerMindset::SetPieceThrow;
        }
        return PlayerMindset::SetPieceKickoff;
    }

    // 수비 세트피스
    if ctx.team_phase == TeamPhase::Defense {
        if ctx.is_goalkeeper {
            return PlayerMindset::SetPieceDefendGK;
        }
        // 골대 근처면 맨마킹
        if ctx.distance_to_own_goal < 20.0 {
            return PlayerMindset::SetPieceDefendManMark;
        }
        return PlayerMindset::SetPieceDefendZonal;
    }

    // 공격 세트피스 (비킥커)
    PlayerMindset::AttackOffBall
}

// ============================================================================
// 후보 생성 함수
// ============================================================================

/// 마인드셋에 따라 후보 행동을 생성합니다.
///
/// # Arguments
/// * `mindset` - 현재 마인드셋
/// * `ctx` - 후보 생성 컨텍스트
///
/// # Returns
/// 후보 행동 리스트
pub fn build_candidates(mindset: PlayerMindset, ctx: &CandidateContext) -> Vec<CandidateAction> {
    match mindset {
        // ========== 공격 온볼 ==========
        PlayerMindset::AttackOnBall => build_attack_on_ball_candidates(ctx),

        // ========== 공격 오프볼 ==========
        PlayerMindset::AttackOffBall => build_attack_off_ball_candidates(ctx),

        // ========== 공 받기 ==========
        PlayerMindset::AttackReceiveBall => build_receive_ball_candidates(ctx),

        // ========== 공격 지원 ==========
        PlayerMindset::AttackSupport => vec![
            CandidateAction::ShowForBall,
            CandidateAction::MoveLateral,
            CandidateAction::CallForBall,
        ],

        // ========== 침투 ==========
        PlayerMindset::AttackPenetrate => vec![
            CandidateAction::RunBehindLine,
            CandidateAction::RunDiagonal,
            CandidateAction::CallForBall,
            CandidateAction::AttackBackPost,
        ],

        // ========== 폭 유지 ==========
        PlayerMindset::AttackHoldWidth => vec![
            CandidateAction::HoldWidth,
            CandidateAction::ShowForBall,
            CandidateAction::RunDiagonal,
        ],

        // ========== 역습 대비 ==========
        PlayerMindset::AttackRestDefense => {
            vec![CandidateAction::RestDefenseAnchor, CandidateAction::MoveToFormation]
        }

        // ========== 수비 ==========
        PlayerMindset::DefendPresser => build_presser_candidates(ctx),
        PlayerMindset::DefendContain => vec![
            CandidateAction::Jockey,
            CandidateAction::CloseDown,
            CandidateAction::ForceWide,
            CandidateAction::ForceInside,
        ],
        PlayerMindset::DefendMark => {
            vec![CandidateAction::InterceptLine, CandidateAction::CloseDown, CandidateAction::Block]
        }
        PlayerMindset::DefendCover => vec![
            CandidateAction::DropLine,
            CandidateAction::ShiftTeam,
            CandidateAction::MoveToFormation,
        ],
        PlayerMindset::DefendBlockShot => {
            vec![CandidateAction::Block, CandidateAction::CloseDown, CandidateAction::TackleStand]
        }
        PlayerMindset::DefendTrackRunner => vec![
            CandidateAction::CloseDown,
            CandidateAction::InterceptLine,
            CandidateAction::MoveToFormation,
        ],
        PlayerMindset::DefendClearance => {
            vec![CandidateAction::ClearLong, CandidateAction::ClearWide, CandidateAction::PassOut]
        }

        // ========== 전환 공격 ==========
        PlayerMindset::TransitionCarry => vec![
            CandidateAction::CarryFast,
            CandidateAction::PassThroughEarly,
            CandidateAction::PassWideRelease,
        ],
        PlayerMindset::TransitionOutletPass => vec![
            CandidateAction::PassThroughEarly,
            CandidateAction::PassWideRelease,
            CandidateAction::PassShort,
        ],
        PlayerMindset::TransitionSprintRun => vec![
            CandidateAction::RunBehindLine,
            CandidateAction::RunWide,
            CandidateAction::CallForBall,
        ],
        PlayerMindset::TransitionSupportTriangle => vec![
            CandidateAction::ShowForBall,
            CandidateAction::MoveLateral,
            CandidateAction::RunIntoSpace,
        ],

        // ========== 전환 수비 ==========
        PlayerMindset::TransitionCounterpress => vec![
            CandidateAction::SprintToBall,
            CandidateAction::TackleStand,
            CandidateAction::InterceptLine,
        ],
        PlayerMindset::TransitionRecoverShape => vec![
            CandidateAction::DropAndReset,
            CandidateAction::MoveToFormation,
            CandidateAction::ShiftTeam,
        ],
        PlayerMindset::TransitionCutPassLane => vec![
            CandidateAction::BlockOutlet,
            CandidateAction::InterceptLine,
            CandidateAction::ForceBackpass,
        ],

        // ========== 세트피스 공격 ==========
        PlayerMindset::SetPieceKickoff => {
            vec![CandidateAction::KickoffSafeBackpass, CandidateAction::KickoffLongBall]
        }
        PlayerMindset::SetPieceFreeKick => vec![
            CandidateAction::FKDirectShot,
            CandidateAction::FKCross,
            CandidateAction::FKShortPass,
        ],
        PlayerMindset::SetPieceCorner => vec![
            CandidateAction::CornerInswing,
            CandidateAction::CornerOutswing,
            CandidateAction::CornerShort,
            CandidateAction::CornerNearPost,
            CandidateAction::CornerFarPost,
        ],
        PlayerMindset::SetPieceThrow => {
            vec![CandidateAction::ThrowShort, CandidateAction::ThrowLong]
        }
        PlayerMindset::SetPiecePenalty => {
            vec![CandidateAction::PKPlaced, CandidateAction::PKPower, CandidateAction::PKPanenka]
        }

        // ========== 세트피스 수비 ==========
        PlayerMindset::SetPieceDefendZonal => {
            vec![CandidateAction::DefSetPieceZonal, CandidateAction::MoveToFormation]
        }
        PlayerMindset::SetPieceDefendManMark => {
            vec![CandidateAction::DefSetPieceManMark, CandidateAction::DefClearFirstContact]
        }
        PlayerMindset::SetPieceDefendClear => {
            vec![CandidateAction::DefClearFirstContact, CandidateAction::ClearLong]
        }
        PlayerMindset::SetPieceDefendGK => {
            vec![CandidateAction::DefGKClaim, CandidateAction::GKPunch, CandidateAction::GKCatch]
        }

        // ========== GK ==========
        PlayerMindset::GKPositioning => {
            vec![CandidateAction::GKPosition, CandidateAction::GKSaveAttempt]
        }
        PlayerMindset::GKSave => {
            vec![CandidateAction::GKSaveAttempt, CandidateAction::GKPunch, CandidateAction::GKCatch]
        }
        PlayerMindset::GKDistribution => vec![
            CandidateAction::GKDistributeShort,
            CandidateAction::GKDistributeLong,
            CandidateAction::GKDistributeThrow,
        ],
    }
}

/// 공격 온볼 후보 생성
fn build_attack_on_ball_candidates(ctx: &CandidateContext) -> Vec<CandidateAction> {
    let mut candidates = vec![
        CandidateAction::DribbleKeep,
        CandidateAction::DribbleCarry,
        CandidateAction::PassShort,
        CandidateAction::PassBack,
        CandidateAction::HoldBall,
    ];

    // 조건부 후보
    if ctx.is_1v1_situation && ctx.dribbling > 60 {
        candidates.push(CandidateAction::DribbleTakeOn);
    }

    if ctx.has_shooting_angle {
        candidates.push(CandidateAction::ShotNormal);
        if ctx.technique > 70 {
            candidates.push(CandidateAction::ShotFinesse);
        }
        if ctx.long_shots > 70 && !ctx.in_penalty_box {
            candidates.push(CandidateAction::ShotPower);
        }
    }

    if ctx.is_wide_zone {
        candidates.push(CandidateAction::CrossEarly);
        candidates.push(CandidateAction::DribbleCutInside);
        if ctx.in_penalty_box {
            candidates.push(CandidateAction::Cutback);
            candidates.push(CandidateAction::CrossByline);
        }
    }

    if ctx.has_through_pass_option && ctx.vision > 65 {
        candidates.push(CandidateAction::PassThrough);
    }

    if ctx.vision > 70 && ctx.technique > 70 {
        candidates.push(CandidateAction::PassLob);
        candidates.push(CandidateAction::PassSwitch);
    }

    if ctx.under_pressure {
        candidates.push(CandidateAction::ShieldBall);
        if ctx.flair > 70 && ctx.composure > 70 {
            candidates.push(CandidateAction::DrawFoul);
        }
    }

    if ctx.gk_advanced && ctx.has_shooting_angle && ctx.technique > 65 {
        candidates.push(CandidateAction::ShotChip);
    }

    candidates
}

/// 공격 오프볼 후보 생성
fn build_attack_off_ball_candidates(ctx: &CandidateContext) -> Vec<CandidateAction> {
    let mut candidates = vec![
        CandidateAction::RunIntoSpace,
        CandidateAction::ShowForBall,
        CandidateAction::MoveLateral,
        CandidateAction::MoveToFormation,
    ];

    // 침투 러닝
    candidates.push(CandidateAction::RunBehindLine);
    candidates.push(CandidateAction::RunDiagonal);

    // 크로스 상황
    if ctx.is_cross_situation {
        candidates.push(CandidateAction::AttackBackPost);
    }

    // 지원
    if ctx.has_close_support {
        candidates.push(CandidateAction::CallForBall);
        candidates.push(CandidateAction::PointRun);
    }

    // Flair가 높으면 미끼 침투
    if ctx.flair > 70 {
        candidates.push(CandidateAction::DummyRun);
    }

    // 딥 드롭
    if ctx.technique > 65 {
        candidates.push(CandidateAction::DropDeep);
    }

    candidates
}

/// 공 받기 후보 생성
fn build_receive_ball_candidates(ctx: &CandidateContext) -> Vec<CandidateAction> {
    let mut candidates = vec![CandidateAction::FirstTouchControl];

    // 원터치 옵션
    if ctx.vision > 65 && ctx.composure > 60 {
        candidates.push(CandidateAction::OneTouchPass);
        candidates.push(CandidateAction::OneTouchLayoff);
    }

    if ctx.has_shooting_angle && ctx.finishing > 65 && ctx.composure > 65 {
        candidates.push(CandidateAction::OneTouchShot);
    }

    // Flair가 높으면 더미
    if ctx.flair > 75 && ctx.technique > 70 {
        candidates.push(CandidateAction::DummyLetRun);
    }

    candidates
}

/// 수비 압박 후보 생성
fn build_presser_candidates(ctx: &CandidateContext) -> Vec<CandidateAction> {
    let mut candidates = vec![CandidateAction::CloseDown, CandidateAction::Jockey];

    // 태클 옵션
    candidates.push(CandidateAction::TackleStand);

    // 위험한 슬라이딩 태클 (상황에 따라)
    if ctx.in_penalty_box {
        // 박스 안에서는 신중하게
        // 슬라이딩 태클 추가하지 않음
    } else {
        candidates.push(CandidateAction::TackleSlide);
    }

    // 방향 몰기
    candidates.push(CandidateAction::ForceWide);
    candidates.push(CandidateAction::ForceInside);

    // 인터셉트
    candidates.push(CandidateAction::InterceptLine);

    candidates
}

// ============================================================================
// 테스트
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mindset_is_attacking() {
        assert!(PlayerMindset::AttackOnBall.is_attacking());
        assert!(PlayerMindset::AttackOffBall.is_attacking());
        assert!(PlayerMindset::TransitionCarry.is_attacking());
        assert!(!PlayerMindset::DefendPresser.is_attacking());
        assert!(!PlayerMindset::DefendCover.is_attacking());
    }

    #[test]
    fn test_mindset_is_defending() {
        assert!(!PlayerMindset::AttackOnBall.is_defending());
        assert!(PlayerMindset::DefendPresser.is_defending());
        assert!(PlayerMindset::DefendCover.is_defending());
        assert!(PlayerMindset::TransitionCounterpress.is_defending());
    }

    #[test]
    fn test_mindset_is_transition() {
        assert!(!PlayerMindset::AttackOnBall.is_transition());
        assert!(PlayerMindset::TransitionCarry.is_transition());
        assert!(PlayerMindset::TransitionCounterpress.is_transition());
        assert!(PlayerMindset::TransitionRecoverShape.is_transition());
    }

    #[test]
    fn test_determine_attack_on_ball() {
        let ctx =
            MindsetContext { has_ball: true, team_phase: TeamPhase::Attack, ..Default::default() };
        assert_eq!(determine_player_mindset(&ctx), PlayerMindset::AttackOnBall);
    }

    #[test]
    fn test_determine_defend_presser() {
        let ctx = MindsetContext {
            has_ball: false,
            team_phase: TeamPhase::Defense,
            distance_to_ball: 5.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx), PlayerMindset::DefendPresser);
    }

    #[test]
    fn test_determine_defend_cover() {
        let ctx = MindsetContext {
            has_ball: false,
            team_phase: TeamPhase::Defense,
            distance_to_ball: 30.0,
            distance_to_own_goal: 40.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx), PlayerMindset::DefendCover);
    }

    #[test]
    fn test_determine_gk_mindset() {
        let ctx = MindsetContext { is_goalkeeper: true, has_ball: false, ..Default::default() };
        assert_eq!(determine_player_mindset(&ctx), PlayerMindset::GKPositioning);

        let ctx_with_ball =
            MindsetContext { is_goalkeeper: true, has_ball: true, ..Default::default() };
        assert_eq!(determine_player_mindset(&ctx_with_ball), PlayerMindset::GKDistribution);
    }

    #[test]
    fn test_build_attack_on_ball_basic() {
        let ctx = CandidateContext::default();
        let candidates = build_candidates(PlayerMindset::AttackOnBall, &ctx);

        // 기본 후보들이 포함되어야 함
        assert!(candidates.contains(&CandidateAction::DribbleKeep));
        assert!(candidates.contains(&CandidateAction::DribbleCarry));
        assert!(candidates.contains(&CandidateAction::PassShort));
        assert!(candidates.contains(&CandidateAction::PassBack));
        assert!(candidates.contains(&CandidateAction::HoldBall));
    }

    #[test]
    fn test_build_attack_on_ball_with_shooting() {
        let ctx = CandidateContext { has_shooting_angle: true, ..Default::default() };
        let candidates = build_candidates(PlayerMindset::AttackOnBall, &ctx);

        assert!(candidates.contains(&CandidateAction::ShotNormal));
    }

    #[test]
    fn test_build_attack_on_ball_with_1v1() {
        let ctx = CandidateContext { is_1v1_situation: true, dribbling: 70, ..Default::default() };
        let candidates = build_candidates(PlayerMindset::AttackOnBall, &ctx);

        assert!(candidates.contains(&CandidateAction::DribbleTakeOn));
    }

    #[test]
    fn test_build_receive_ball_candidates() {
        let ctx = CandidateContext { vision: 70, composure: 70, ..Default::default() };
        let candidates = build_candidates(PlayerMindset::AttackReceiveBall, &ctx);

        assert!(candidates.contains(&CandidateAction::FirstTouchControl));
        assert!(candidates.contains(&CandidateAction::OneTouchPass));
        assert!(candidates.contains(&CandidateAction::OneTouchLayoff));
    }

    #[test]
    fn test_build_defend_presser_candidates() {
        let ctx = CandidateContext::default();
        let candidates = build_candidates(PlayerMindset::DefendPresser, &ctx);

        assert!(candidates.contains(&CandidateAction::CloseDown));
        assert!(candidates.contains(&CandidateAction::Jockey));
        assert!(candidates.contains(&CandidateAction::TackleStand));
        assert!(candidates.contains(&CandidateAction::TackleSlide));
    }

    #[test]
    fn test_build_defend_presser_in_box() {
        let ctx = CandidateContext { in_penalty_box: true, ..Default::default() };
        let candidates = build_candidates(PlayerMindset::DefendPresser, &ctx);

        // 박스 안에서는 슬라이딩 태클 없음
        assert!(!candidates.contains(&CandidateAction::TackleSlide));
    }

    #[test]
    fn test_candidate_action_group() {
        assert_eq!(CandidateAction::DribbleKeep.group(), CandidateGroup::Dribble);
        assert_eq!(CandidateAction::PassShort.group(), CandidateGroup::Pass);
        assert_eq!(CandidateAction::ShotNormal.group(), CandidateGroup::Shot);
        assert_eq!(CandidateAction::CloseDown.group(), CandidateGroup::Defend);
    }

    #[test]
    fn test_candidate_action_is_on_ball() {
        assert!(CandidateAction::DribbleKeep.is_on_ball());
        assert!(CandidateAction::PassShort.is_on_ball());
        assert!(CandidateAction::ShotNormal.is_on_ball());
        assert!(!CandidateAction::CloseDown.is_on_ball());
        assert!(!CandidateAction::RunIntoSpace.is_on_ball());
    }

    #[test]
    fn test_transition_attack_mindset() {
        // 온볼 - 전방에 공간
        let ctx = MindsetContext {
            has_ball: true,
            team_phase: TeamPhase::TransitionAttack,
            distance_to_goal: 50.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx), PlayerMindset::TransitionCarry);

        // 온볼 - 골대 근처
        let ctx2 = MindsetContext {
            has_ball: true,
            team_phase: TeamPhase::TransitionAttack,
            distance_to_goal: 20.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx2), PlayerMindset::TransitionOutletPass);

        // 오프볼 - 침투
        let ctx3 = MindsetContext {
            has_ball: false,
            team_phase: TeamPhase::TransitionAttack,
            distance_to_goal: 30.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx3), PlayerMindset::TransitionSprintRun);
    }

    #[test]
    fn test_transition_defend_mindset() {
        // 공 근처 - 압박
        let ctx = MindsetContext {
            has_ball: false,
            team_phase: TeamPhase::TransitionDefense,
            distance_to_ball: 8.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx), PlayerMindset::TransitionCounterpress);

        // 중간 거리 - 패스 차단
        let ctx2 = MindsetContext {
            has_ball: false,
            team_phase: TeamPhase::TransitionDefense,
            distance_to_ball: 18.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx2), PlayerMindset::TransitionCutPassLane);

        // 먼 거리 - 복귀
        let ctx3 = MindsetContext {
            has_ball: false,
            team_phase: TeamPhase::TransitionDefense,
            distance_to_ball: 40.0,
            ..Default::default()
        };
        assert_eq!(determine_player_mindset(&ctx3), PlayerMindset::TransitionRecoverShape);
    }
}
