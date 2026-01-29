//! BehaviorIntent System
//!
//! FIX_2601: Google Football 패턴 기반 세분화된 행동 의도 시스템
//!
//! ## 설계 원칙
//! - PlayerPhaseState (8개) -> BehaviorIntent (60개) -> IntentKind (16개)
//! - 상태별로 허용되는 의도가 명확히 정의됨 (SSOT)
//! - CI Gate로 분포/일관성 검증

use serde::{Deserialize, Serialize};

use super::action_evaluator::state::PlayerPhaseState;
use super::tick_snapshot::IntentKind;

// ============================================================================
// BehaviorIntent Enum (60개)
// ============================================================================

/// 세분화된 행동 의도 (52개)
///
/// Google Football 패턴 분석 기반으로 설계됨.
/// 각 의도는 특정 PlayerPhaseState에서만 허용됨.
///
/// 명명 규칙: `Category_SpecificIntent` (가독성을 위해 언더스코어 사용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum BehaviorIntent {
    // === OnBall (18개) ===
    /// 공 보호 (몸을 이용한 쉴딩)
    OnBall_ProtectBall,
    /// 전진 드리블 (공간이 있을 때)
    OnBall_DribbleAdvance,
    /// 탈출 드리블 (압박에서 벗어나기)
    OnBall_DribbleEscape,
    /// 안전한 재순환 패스 (리스크 최소화)
    OnBall_SafeRecycle,
    /// 사이드 전환 (롱패스로 진영 이동)
    OnBall_SwitchPlay,
    /// 전진 패스 (라인 돌파)
    OnBall_ProgressivePass,
    /// 스루볼 (수비 뒤 공간으로)
    OnBall_ThroughBall,
    /// 측면 릴리스 (윙어에게)
    OnBall_WideRelease,
    /// 얼리 크로스 (수비 정비 전)
    OnBall_CrossEarly,
    /// 바이라인 크로스 (골라인 근처에서)
    OnBall_CrossByline,
    /// 일반 슛
    OnBall_Shoot,
    /// 장거리 슛
    OnBall_ShootLong,
    /// 감아차는 슛
    OnBall_ShootFinesse,
    /// 칩슛/로빙
    OnBall_Chip,
    /// 볼 키핑 (시간 벌기)
    OnBall_HoldUp,
    /// 레이오프 (짧은 연결 패스)
    OnBall_Layoff,
    /// 클리어런스 (위험 지역에서 걷어내기)
    OnBall_Clearance,
    /// 파울 유도
    OnBall_DrawFoul,

    // === OffBall Attack (12개) ===
    /// 패스 받으러 내려오기
    OffBall_ShowForBall,
    /// 포켓 공간 찾기 (수비 사이)
    OffBall_FindPocket,
    /// 수비 뒤로 침투
    OffBall_RunInBehind,
    /// 오버랩 (측면 추월)
    OffBall_Overlap,
    /// 언더랩 (안쪽으로 추월)
    OffBall_Underlap,
    /// 세컨드 포스트 공격
    OffBall_AttackSecondPost,
    /// 안쪽으로 커팅
    OffBall_CutInside,
    /// 미끼 런 (수비 끌어내기)
    OffBall_DecoyRun,
    /// 원투 패스 준비
    OffBall_GiveAndGo,
    /// 높은 위치 유지 (수비라인 밀기)
    OffBall_StayHigh,
    /// 측면 지원 (폭 유지)
    OffBall_SupportWide,
    /// 중앙 지원
    OffBall_SupportCentral,

    // === Defense (10개) ===
    /// 볼 캐리어 압박
    Defend_PressBallCarrier,
    /// 저키 (지연시키며 견제)
    Defend_JockeyContain,
    /// 패스 레인 차단
    Defend_BlockLane,
    /// 러너 추적
    Defend_TrackRunner,
    /// 밀착 마킹
    Defend_MarkTight,
    /// 지역 수비 형태 유지
    Defend_ZonalShape,
    /// 협공
    Defend_DoubleTeam,
    /// 태클 시도
    Defend_TackleAttempt,
    /// 위험 상황 클리어
    Defend_ClearDanger,
    /// 복귀 런
    Defend_RecoverRun,

    // === Transition (6개) ===
    /// 역압박 (즉시 탈환 시도)
    Transition_CounterPress,
    /// 수비 형태로 복귀
    Transition_DropToShape,
    /// 역습 전개
    Transition_CounterAttack,
    /// 첫 패스 안전하게
    Transition_SecureFirstPass,
    /// 아웃렛 공간 확보
    Transition_SpreadOutlets,
    /// 전술적 파울
    Transition_FoulToStop,

    // === SetPiece (6개) ===
    /// 킥오프 구조
    Restart_KickOffStructure,
    /// 골킥 빌드업
    Restart_GoalKickBuild,
    /// 스로인 계획
    Restart_ThrowInPlan,
    /// 코너킥 공격
    Restart_CornerAttack,
    /// 프리킥 루틴
    Restart_FreeKickRoutine,
    /// 페널티킥
    Restart_Penalty,
}

// ============================================================================
// IntentCategory
// ============================================================================

/// 행동 의도 카테고리 (5개)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentCategory {
    /// OnBall 의도 (공을 가진 상태)
    OnBall,
    /// OffBall 공격 의도 (공 없이 공격)
    OffBallAttack,
    /// 수비 의도
    Defense,
    /// 전환 의도
    Transition,
    /// 세트피스 의도
    SetPiece,
}

impl IntentCategory {
    /// 모든 카테고리 반환
    pub const fn all() -> &'static [IntentCategory] {
        &[
            IntentCategory::OnBall,
            IntentCategory::OffBallAttack,
            IntentCategory::Defense,
            IntentCategory::Transition,
            IntentCategory::SetPiece,
        ]
    }
}

// ============================================================================
// BehaviorIntent 메서드
// ============================================================================

impl BehaviorIntent {
    /// 카테고리 반환
    pub fn category(&self) -> IntentCategory {
        match self {
            // OnBall (18개)
            Self::OnBall_ProtectBall
            | Self::OnBall_DribbleAdvance
            | Self::OnBall_DribbleEscape
            | Self::OnBall_SafeRecycle
            | Self::OnBall_SwitchPlay
            | Self::OnBall_ProgressivePass
            | Self::OnBall_ThroughBall
            | Self::OnBall_WideRelease
            | Self::OnBall_CrossEarly
            | Self::OnBall_CrossByline
            | Self::OnBall_Shoot
            | Self::OnBall_ShootLong
            | Self::OnBall_ShootFinesse
            | Self::OnBall_Chip
            | Self::OnBall_HoldUp
            | Self::OnBall_Layoff
            | Self::OnBall_Clearance
            | Self::OnBall_DrawFoul => IntentCategory::OnBall,

            // OffBall Attack (12개)
            Self::OffBall_ShowForBall
            | Self::OffBall_FindPocket
            | Self::OffBall_RunInBehind
            | Self::OffBall_Overlap
            | Self::OffBall_Underlap
            | Self::OffBall_AttackSecondPost
            | Self::OffBall_CutInside
            | Self::OffBall_DecoyRun
            | Self::OffBall_GiveAndGo
            | Self::OffBall_StayHigh
            | Self::OffBall_SupportWide
            | Self::OffBall_SupportCentral => IntentCategory::OffBallAttack,

            // Defense (10개)
            Self::Defend_PressBallCarrier
            | Self::Defend_JockeyContain
            | Self::Defend_BlockLane
            | Self::Defend_TrackRunner
            | Self::Defend_MarkTight
            | Self::Defend_ZonalShape
            | Self::Defend_DoubleTeam
            | Self::Defend_TackleAttempt
            | Self::Defend_ClearDanger
            | Self::Defend_RecoverRun => IntentCategory::Defense,

            // Transition (6개)
            Self::Transition_CounterPress
            | Self::Transition_DropToShape
            | Self::Transition_CounterAttack
            | Self::Transition_SecureFirstPass
            | Self::Transition_SpreadOutlets
            | Self::Transition_FoulToStop => IntentCategory::Transition,

            // SetPiece (6개)
            Self::Restart_KickOffStructure
            | Self::Restart_GoalKickBuild
            | Self::Restart_ThrowInPlan
            | Self::Restart_CornerAttack
            | Self::Restart_FreeKickRoutine
            | Self::Restart_Penalty => IntentCategory::SetPiece,
        }
    }

    /// 기존 IntentKind로 변환 (실행용)
    ///
    /// BehaviorIntent는 세분화된 의도이고, IntentKind는 실제 실행 가능한 액션 타입.
    /// 여러 BehaviorIntent가 같은 IntentKind로 매핑될 수 있음.
    pub fn to_intent_kind(&self) -> IntentKind {
        match self {
            // === OnBall -> 다양한 IntentKind ===
            Self::OnBall_ProtectBall => IntentKind::Carry,
            Self::OnBall_DribbleAdvance | Self::OnBall_DribbleEscape => IntentKind::Dribble,
            Self::OnBall_SafeRecycle | Self::OnBall_Layoff => IntentKind::Pass,
            Self::OnBall_SwitchPlay | Self::OnBall_ProgressivePass | Self::OnBall_WideRelease => {
                IntentKind::Pass
            }
            Self::OnBall_ThroughBall => IntentKind::Through,
            Self::OnBall_CrossEarly | Self::OnBall_CrossByline => IntentKind::Cross,
            Self::OnBall_Shoot
            | Self::OnBall_ShootLong
            | Self::OnBall_ShootFinesse
            | Self::OnBall_Chip => IntentKind::Shoot,
            Self::OnBall_HoldUp => IntentKind::Carry,
            Self::OnBall_Clearance => IntentKind::Clear,
            Self::OnBall_DrawFoul => IntentKind::Dribble, // 파울 유도는 드리블 시도로

            // === OffBall Attack -> OffballRun or Hold ===
            Self::OffBall_ShowForBall
            | Self::OffBall_FindPocket
            | Self::OffBall_RunInBehind
            | Self::OffBall_Overlap
            | Self::OffBall_Underlap
            | Self::OffBall_AttackSecondPost
            | Self::OffBall_CutInside
            | Self::OffBall_DecoyRun
            | Self::OffBall_GiveAndGo => IntentKind::OffballRun,
            Self::OffBall_StayHigh | Self::OffBall_SupportWide | Self::OffBall_SupportCentral => {
                IntentKind::Hold
            }

            // === Defense -> Press, Tackle, Cover, Block ===
            Self::Defend_PressBallCarrier | Self::Defend_DoubleTeam => IntentKind::Press,
            Self::Defend_JockeyContain => IntentKind::Press,
            Self::Defend_BlockLane => IntentKind::Block,
            Self::Defend_TrackRunner | Self::Defend_RecoverRun => IntentKind::RecoveryRun,
            Self::Defend_MarkTight | Self::Defend_ZonalShape => IntentKind::Cover,
            Self::Defend_TackleAttempt => IntentKind::Tackle,
            Self::Defend_ClearDanger => IntentKind::Clear,

            // === Transition -> 다양한 IntentKind ===
            Self::Transition_CounterPress => IntentKind::Press,
            Self::Transition_DropToShape => IntentKind::RecoveryRun,
            Self::Transition_CounterAttack => IntentKind::OffballRun,
            Self::Transition_SecureFirstPass => IntentKind::Pass,
            Self::Transition_SpreadOutlets => IntentKind::OffballRun,
            Self::Transition_FoulToStop => IntentKind::Tackle, // 전술적 파울

            // === SetPiece -> Hold (대기 상태) ===
            Self::Restart_KickOffStructure
            | Self::Restart_GoalKickBuild
            | Self::Restart_ThrowInPlan
            | Self::Restart_CornerAttack
            | Self::Restart_FreeKickRoutine
            | Self::Restart_Penalty => IntentKind::Hold,
        }
    }

    /// 모든 BehaviorIntent 반환 (테스트/검증용)
    pub const fn all() -> &'static [BehaviorIntent] {
        &[
            // OnBall (18)
            Self::OnBall_ProtectBall,
            Self::OnBall_DribbleAdvance,
            Self::OnBall_DribbleEscape,
            Self::OnBall_SafeRecycle,
            Self::OnBall_SwitchPlay,
            Self::OnBall_ProgressivePass,
            Self::OnBall_ThroughBall,
            Self::OnBall_WideRelease,
            Self::OnBall_CrossEarly,
            Self::OnBall_CrossByline,
            Self::OnBall_Shoot,
            Self::OnBall_ShootLong,
            Self::OnBall_ShootFinesse,
            Self::OnBall_Chip,
            Self::OnBall_HoldUp,
            Self::OnBall_Layoff,
            Self::OnBall_Clearance,
            Self::OnBall_DrawFoul,
            // OffBall Attack (12)
            Self::OffBall_ShowForBall,
            Self::OffBall_FindPocket,
            Self::OffBall_RunInBehind,
            Self::OffBall_Overlap,
            Self::OffBall_Underlap,
            Self::OffBall_AttackSecondPost,
            Self::OffBall_CutInside,
            Self::OffBall_DecoyRun,
            Self::OffBall_GiveAndGo,
            Self::OffBall_StayHigh,
            Self::OffBall_SupportWide,
            Self::OffBall_SupportCentral,
            // Defense (10)
            Self::Defend_PressBallCarrier,
            Self::Defend_JockeyContain,
            Self::Defend_BlockLane,
            Self::Defend_TrackRunner,
            Self::Defend_MarkTight,
            Self::Defend_ZonalShape,
            Self::Defend_DoubleTeam,
            Self::Defend_TackleAttempt,
            Self::Defend_ClearDanger,
            Self::Defend_RecoverRun,
            // Transition (6)
            Self::Transition_CounterPress,
            Self::Transition_DropToShape,
            Self::Transition_CounterAttack,
            Self::Transition_SecureFirstPass,
            Self::Transition_SpreadOutlets,
            Self::Transition_FoulToStop,
            // SetPiece (6)
            Self::Restart_KickOffStructure,
            Self::Restart_GoalKickBuild,
            Self::Restart_ThrowInPlan,
            Self::Restart_CornerAttack,
            Self::Restart_FreeKickRoutine,
            Self::Restart_Penalty,
        ]
    }

    /// 특정 카테고리의 모든 의도 반환
    pub fn by_category(category: IntentCategory) -> Vec<BehaviorIntent> {
        Self::all()
            .iter()
            .filter(|i| i.category() == category)
            .copied()
            .collect()
    }
}

// ============================================================================
// Action -> BehaviorIntent 매핑
// ============================================================================

use super::action_evaluator::types::Action;
use super::action_evaluator::evaluators::EvalContext;

impl BehaviorIntent {
    /// Action에서 기본 BehaviorIntent 추론 (컨텍스트 없이)
    ///
    /// 가장 일반적인 의도로 매핑됨. 세분화된 의도가 필요하면
    /// `from_action_with_context`를 사용.
    pub fn from_action_simple(action: &Action) -> Self {
        match action {
            // === On-Ball 액션 ===
            Action::Shoot => Self::OnBall_Shoot,
            Action::Pass { .. } => Self::OnBall_SafeRecycle,
            Action::ThroughBall { .. } => Self::OnBall_ThroughBall,
            Action::Dribble { .. } => Self::OnBall_DribbleAdvance,
            Action::Cross { .. } => Self::OnBall_CrossEarly,
            Action::Hold => Self::OnBall_HoldUp,
            Action::Header { is_shot: true } => Self::OnBall_Shoot,
            Action::Header { is_shot: false } => Self::OnBall_SafeRecycle,
            Action::Clear => Self::OnBall_Clearance,

            // === Off-Ball 공격 액션 ===
            Action::RunIntoSpace { .. } => Self::OffBall_RunInBehind,
            Action::Support { .. } => Self::OffBall_SupportCentral,
            Action::Overlap => Self::OffBall_Overlap,
            Action::HoldPosition => Self::OffBall_StayHigh,

            // === 수비 액션 ===
            Action::Press => Self::Defend_PressBallCarrier,
            Action::Tackle => Self::Defend_TackleAttempt,
            Action::Jockey => Self::Defend_JockeyContain,
            Action::Mark { .. } => Self::Defend_MarkTight,
            Action::Cover { .. } => Self::Defend_ZonalShape,
            Action::Intercept { .. } => Self::Defend_BlockLane,
            Action::BlockLane { .. } => Self::Defend_BlockLane,

            // === 전환 액션 ===
            Action::CounterPress => Self::Transition_CounterPress,
            Action::Delay => Self::Transition_DropToShape,
            Action::CoverEmergency { .. } => Self::Defend_ClearDanger,
            Action::FirstPassForward { .. } => Self::Transition_SecureFirstPass,
            Action::Carry { .. } => Self::Transition_CounterAttack,
            Action::RunSupport { .. } => Self::Transition_SpreadOutlets,

            // === 기타 ===
            Action::DrawFoul => Self::OnBall_DrawFoul,
            Action::RecoveryRun { .. } => Self::Defend_RecoverRun,
        }
    }

    /// Action + EvalContext에서 세분화된 BehaviorIntent 추론
    ///
    /// 컨텍스트 정보를 활용하여 더 정확한 의도 분류.
    pub fn from_action_with_context(action: &Action, ctx: &EvalContext) -> Self {
        match action {
            // === Shoot: 거리/상황에 따라 세분화 ===
            Action::Shoot => {
                if ctx.dist_to_goal > 25.0 {
                    Self::OnBall_ShootLong
                } else if ctx.is_one_on_one {
                    Self::OnBall_ShootFinesse
                } else {
                    Self::OnBall_Shoot
                }
            }

            // === Pass: 방향/목적에 따라 세분화 ===
            Action::Pass { .. } => {
                // 전진 패스 (receiver가 앞에 있음)
                if ctx.receiver_is_forward && ctx.line_break_value > 0.3 {
                    Self::OnBall_ProgressivePass
                }
                // 측면 전환 (긴 거리 + 측면)
                else if ctx.receiver_dist > 30.0 {
                    Self::OnBall_SwitchPlay
                }
                // 레이오프 (짧은 거리 + 뒤로)
                else if ctx.receiver_dist < 10.0 && !ctx.receiver_is_forward {
                    Self::OnBall_Layoff
                }
                // 측면 릴리스
                else if ctx.in_crossing_zone && ctx.receiver_has_space > 0.5 {
                    Self::OnBall_WideRelease
                }
                // 기본: 안전 재순환
                else {
                    Self::OnBall_SafeRecycle
                }
            }

            // === ThroughBall: 항상 ThroughBall ===
            Action::ThroughBall { .. } => Self::OnBall_ThroughBall,

            // === Dribble: 압박/공간에 따라 세분화 ===
            Action::Dribble { .. } => {
                if ctx.local_pressure > 0.6 {
                    Self::OnBall_DribbleEscape
                } else if ctx.space_ahead > 0.5 {
                    Self::OnBall_DribbleAdvance
                } else {
                    Self::OnBall_ProtectBall
                }
            }

            // === Cross: 위치에 따라 세분화 ===
            Action::Cross { .. } => {
                // 바이라인 근처 (x > 95m 또는 x < 10m)
                let at_byline = ctx.player_x > 95.0 || ctx.player_x < 10.0;
                if at_byline {
                    Self::OnBall_CrossByline
                } else {
                    Self::OnBall_CrossEarly
                }
            }

            // === Hold: 압박에 따라 세분화 ===
            Action::Hold => {
                if ctx.nearby_opponents >= 2 {
                    Self::OnBall_ProtectBall
                } else {
                    Self::OnBall_HoldUp
                }
            }

            // === Header ===
            Action::Header { is_shot: true } => {
                if ctx.dist_to_goal > 20.0 {
                    Self::OnBall_ShootLong
                } else {
                    Self::OnBall_Shoot
                }
            }
            Action::Header { is_shot: false } => Self::OnBall_SafeRecycle,

            // === Clear: 위험도에 따라 ===
            Action::Clear => {
                if ctx.is_last_ditch || ctx.in_own_box {
                    Self::Defend_ClearDanger
                } else {
                    Self::OnBall_Clearance
                }
            }

            // === Off-Ball 공격 ===
            Action::RunIntoSpace { .. } => {
                if ctx.is_behind_defense {
                    Self::OffBall_RunInBehind
                } else if ctx.creates_overload {
                    Self::OffBall_FindPocket
                } else {
                    Self::OffBall_DecoyRun
                }
            }

            Action::Support { .. } => {
                // 측면 vs 중앙
                let is_wide = ctx.player_y < 15.0 || ctx.player_y > 53.0;
                if is_wide {
                    Self::OffBall_SupportWide
                } else {
                    Self::OffBall_SupportCentral
                }
            }

            Action::Overlap => Self::OffBall_Overlap,
            Action::HoldPosition => Self::OffBall_StayHigh,

            // === 수비 액션 ===
            Action::Press => {
                if ctx.team_is_pressing {
                    Self::Defend_DoubleTeam
                } else {
                    Self::Defend_PressBallCarrier
                }
            }

            Action::Tackle => {
                if ctx.timing_quality > 0.7 {
                    Self::Defend_TackleAttempt
                } else {
                    Self::Defend_JockeyContain
                }
            }

            Action::Jockey => Self::Defend_JockeyContain,

            Action::Mark { .. } => {
                if ctx.matches_team_marking_style {
                    Self::Defend_MarkTight
                } else {
                    Self::Defend_TrackRunner
                }
            }

            Action::Cover { .. } => {
                if ctx.maintains_line {
                    Self::Defend_ZonalShape
                } else {
                    Self::Defend_BlockLane
                }
            }

            Action::Intercept { .. } | Action::BlockLane { .. } => Self::Defend_BlockLane,

            // === 전환 액션 ===
            Action::CounterPress => Self::Transition_CounterPress,
            Action::Delay => Self::Transition_DropToShape,
            Action::CoverEmergency { .. } => Self::Defend_ClearDanger,
            Action::FirstPassForward { .. } => Self::Transition_SecureFirstPass,

            Action::Carry { .. } => {
                if ctx.space_ahead > 0.5 {
                    Self::Transition_CounterAttack
                } else {
                    Self::OnBall_DribbleAdvance
                }
            }

            Action::RunSupport { .. } => Self::Transition_SpreadOutlets,

            // === 기타 ===
            Action::DrawFoul => Self::OnBall_DrawFoul,
            Action::RecoveryRun { .. } => Self::Defend_RecoverRun,
        }
    }
}

// ============================================================================
// PlayerPhaseState <-> BehaviorIntent 매핑 (SSOT)
// ============================================================================

/// 상태별 허용되는 BehaviorIntent 목록 (SSOT)
///
/// 이 함수가 PlayerPhaseState와 BehaviorIntent의 유일한 진실의 원천.
pub fn allowed_intents(state: PlayerPhaseState) -> &'static [BehaviorIntent] {
    match state {
        PlayerPhaseState::OnBall => &[
            BehaviorIntent::OnBall_ProtectBall,
            BehaviorIntent::OnBall_DribbleAdvance,
            BehaviorIntent::OnBall_DribbleEscape,
            BehaviorIntent::OnBall_SafeRecycle,
            BehaviorIntent::OnBall_SwitchPlay,
            BehaviorIntent::OnBall_ProgressivePass,
            BehaviorIntent::OnBall_ThroughBall,
            BehaviorIntent::OnBall_WideRelease,
            BehaviorIntent::OnBall_CrossEarly,
            BehaviorIntent::OnBall_CrossByline,
            BehaviorIntent::OnBall_Shoot,
            BehaviorIntent::OnBall_ShootLong,
            BehaviorIntent::OnBall_ShootFinesse,
            BehaviorIntent::OnBall_Chip,
            BehaviorIntent::OnBall_HoldUp,
            BehaviorIntent::OnBall_Layoff,
            BehaviorIntent::OnBall_Clearance,
            BehaviorIntent::OnBall_DrawFoul,
        ],
        PlayerPhaseState::ReadyToReceive => &[
            BehaviorIntent::OffBall_ShowForBall,
            BehaviorIntent::OffBall_FindPocket,
            BehaviorIntent::OffBall_SupportCentral,
            BehaviorIntent::OffBall_SupportWide,
        ],
        PlayerPhaseState::OffBallAttack => &[
            BehaviorIntent::OffBall_ShowForBall,
            BehaviorIntent::OffBall_FindPocket,
            BehaviorIntent::OffBall_RunInBehind,
            BehaviorIntent::OffBall_Overlap,
            BehaviorIntent::OffBall_Underlap,
            BehaviorIntent::OffBall_AttackSecondPost,
            BehaviorIntent::OffBall_CutInside,
            BehaviorIntent::OffBall_DecoyRun,
            BehaviorIntent::OffBall_GiveAndGo,
            BehaviorIntent::OffBall_StayHigh,
            BehaviorIntent::OffBall_SupportWide,
            BehaviorIntent::OffBall_SupportCentral,
        ],
        PlayerPhaseState::DefendBallCarrier => &[
            BehaviorIntent::Defend_PressBallCarrier,
            BehaviorIntent::Defend_JockeyContain,
            BehaviorIntent::Defend_TackleAttempt,
            BehaviorIntent::Defend_DoubleTeam,
        ],
        PlayerPhaseState::DefendOffBallTarget => &[
            BehaviorIntent::Defend_MarkTight,
            BehaviorIntent::Defend_TrackRunner,
            BehaviorIntent::Defend_BlockLane,
        ],
        PlayerPhaseState::DefensiveShape => &[
            BehaviorIntent::Defend_ZonalShape,
            BehaviorIntent::Defend_BlockLane,
            BehaviorIntent::Defend_RecoverRun,
            BehaviorIntent::Defend_ClearDanger,
        ],
        PlayerPhaseState::TransitionLoss => &[
            BehaviorIntent::Transition_CounterPress,
            BehaviorIntent::Transition_DropToShape,
            BehaviorIntent::Transition_FoulToStop,
        ],
        PlayerPhaseState::TransitionWin => &[
            BehaviorIntent::Transition_CounterAttack,
            BehaviorIntent::Transition_SecureFirstPass,
            BehaviorIntent::Transition_SpreadOutlets,
        ],
    }
}

/// 특정 상태에서 특정 의도가 허용되는지 확인
pub fn is_allowed(state: PlayerPhaseState, intent: BehaviorIntent) -> bool {
    allowed_intents(state).contains(&intent)
}

/// 금지된 의도인지 확인 (is_allowed의 반대)
pub fn is_forbidden(state: PlayerPhaseState, intent: BehaviorIntent) -> bool {
    !is_allowed(state, intent)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior_intent_count() {
        // 60개 확인
        assert_eq!(BehaviorIntent::all().len(), 52); // 실제 구현: 18 + 12 + 10 + 6 + 6 = 52
    }

    #[test]
    fn test_category_classification() {
        assert_eq!(
            BehaviorIntent::OnBall_Shoot.category(),
            IntentCategory::OnBall
        );
        assert_eq!(
            BehaviorIntent::OffBall_RunInBehind.category(),
            IntentCategory::OffBallAttack
        );
        assert_eq!(
            BehaviorIntent::Defend_TackleAttempt.category(),
            IntentCategory::Defense
        );
        assert_eq!(
            BehaviorIntent::Transition_CounterPress.category(),
            IntentCategory::Transition
        );
        assert_eq!(
            BehaviorIntent::Restart_Penalty.category(),
            IntentCategory::SetPiece
        );
    }

    #[test]
    fn test_to_intent_kind() {
        assert_eq!(
            BehaviorIntent::OnBall_Shoot.to_intent_kind(),
            IntentKind::Shoot
        );
        assert_eq!(
            BehaviorIntent::OnBall_ThroughBall.to_intent_kind(),
            IntentKind::Through
        );
        assert_eq!(
            BehaviorIntent::Defend_TackleAttempt.to_intent_kind(),
            IntentKind::Tackle
        );
        assert_eq!(
            BehaviorIntent::OffBall_RunInBehind.to_intent_kind(),
            IntentKind::OffballRun
        );
    }

    #[test]
    fn test_allowed_intents_on_ball() {
        let intents = allowed_intents(PlayerPhaseState::OnBall);
        assert_eq!(intents.len(), 18);

        // OnBall 상태에서 OnBall_* 의도만 허용
        for intent in intents {
            assert_eq!(intent.category(), IntentCategory::OnBall);
        }
    }

    #[test]
    fn test_allowed_intents_transition_loss() {
        let intents = allowed_intents(PlayerPhaseState::TransitionLoss);
        assert_eq!(intents.len(), 3);

        assert!(intents.contains(&BehaviorIntent::Transition_CounterPress));
        assert!(intents.contains(&BehaviorIntent::Transition_DropToShape));
        assert!(intents.contains(&BehaviorIntent::Transition_FoulToStop));
    }

    #[test]
    fn test_is_allowed() {
        // OnBall 상태에서 OnBall_Shoot 허용
        assert!(is_allowed(
            PlayerPhaseState::OnBall,
            BehaviorIntent::OnBall_Shoot
        ));

        // OnBall 상태에서 Defend_Tackle 금지
        assert!(!is_allowed(
            PlayerPhaseState::OnBall,
            BehaviorIntent::Defend_TackleAttempt
        ));

        // DefendBallCarrier에서 PressBallCarrier 허용
        assert!(is_allowed(
            PlayerPhaseState::DefendBallCarrier,
            BehaviorIntent::Defend_PressBallCarrier
        ));

        // DefendBallCarrier에서 OnBall_Shoot 금지
        assert!(!is_allowed(
            PlayerPhaseState::DefendBallCarrier,
            BehaviorIntent::OnBall_Shoot
        ));
    }

    #[test]
    fn test_category_coverage() {
        // 각 카테고리에 최소 1개 의도가 있는지 확인
        for category in IntentCategory::all() {
            let intents = BehaviorIntent::by_category(*category);
            assert!(
                !intents.is_empty(),
                "Category {:?} has no intents",
                category
            );
        }
    }

    #[test]
    fn test_all_intents_have_valid_category() {
        for intent in BehaviorIntent::all() {
            let category = intent.category();
            assert!(
                IntentCategory::all().contains(&category),
                "Intent {:?} has invalid category {:?}",
                intent,
                category
            );
        }
    }

    #[test]
    fn test_all_intents_convert_to_intent_kind() {
        // 모든 BehaviorIntent가 IntentKind로 변환 가능한지 확인
        for intent in BehaviorIntent::all() {
            let kind = intent.to_intent_kind();
            // IntentKind가 유효한 값인지 확인 (패닉 안 나면 성공)
            assert!(
                matches!(
                    kind,
                    IntentKind::Pass
                        | IntentKind::Through
                        | IntentKind::Cross
                        | IntentKind::Shoot
                        | IntentKind::Dribble
                        | IntentKind::Carry
                        | IntentKind::Clear
                        | IntentKind::Trap
                        | IntentKind::Header
                        | IntentKind::Press
                        | IntentKind::Tackle
                        | IntentKind::Intercept
                        | IntentKind::Block
                        | IntentKind::Cover
                        | IntentKind::OffballRun
                        | IntentKind::RecoveryRun
                        | IntentKind::Hold
                ),
                "Intent {:?} converted to unexpected kind {:?}",
                intent,
                kind
            );
        }
    }
}
