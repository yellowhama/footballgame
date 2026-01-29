//! ActionMetadata System
//!
//! FIX_2601 Phase 3: RL 친화적 액션 메타데이터 시스템
//!
//! Google Football의 CoreAction 패턴을 참조하여 설계:
//! - sticky: 상태 유지 여부 (명시적 해제 전까지 지속)
//! - directional: 방향 입력 필요 여부
//! - cancellable: 실행 중 취소 가능 여부
//!
//! ## 참조
//! ```python
//! # gfootball/env/football_action_set.py
//! class CoreAction(object):
//!     def __init__(self, backend_action, name, sticky=False, directional=False):
//!         ...
//! ```

use serde::{Deserialize, Serialize};

use super::action_queue::PhaseActionType;
use super::behavior_intent::{BehaviorIntent, IntentCategory};

// ============================================================================
// ActionCategory
// ============================================================================

/// 액션 카테고리 (Google Football 참조)
///
/// RL 에이전트가 액션 공간을 이해하는 데 활용.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionCategory {
    /// 이동 관련 (move, dribble)
    Movement,
    /// 패스 관련 (pass, through ball, cross)
    Pass,
    /// 슈팅 관련 (shot, header shot)
    Shot,
    /// 수비 관련 (tackle, intercept, save)
    Defense,
    /// 해제 관련 (release_sprint, release_dribble)
    Release,
    /// 기타/전환 (trap, header clear)
    Other,
}

impl ActionCategory {
    /// 모든 카테고리 반환
    pub const fn all() -> &'static [ActionCategory] {
        &[
            ActionCategory::Movement,
            ActionCategory::Pass,
            ActionCategory::Shot,
            ActionCategory::Defense,
            ActionCategory::Release,
            ActionCategory::Other,
        ]
    }
}

// ============================================================================
// ActionMetadata
// ============================================================================

/// 액션 메타데이터 (Google Football CoreAction 참조)
///
/// RL 에이전트가 액션 특성을 이해하고 적절히 처리하도록 지원.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionMetadata {
    /// Sticky 액션 여부 (상태 유지)
    ///
    /// - `true`: 명시적 해제 전까지 지속 (예: sprint, dribble)
    /// - `false`: 1회성 액션 (예: pass, shot)
    ///
    /// Google Football의 `sticky` 속성과 동일.
    pub sticky: bool,

    /// 방향성 액션 여부
    ///
    /// - `true`: 방향 입력 필요 (예: move, dribble direction)
    /// - `false`: 방향 불필요 (예: shot, tackle)
    ///
    /// Google Football의 `directional` 속성과 동일.
    pub directional: bool,

    /// 취소 가능 여부
    ///
    /// - `true`: 실행 중 취소 가능 (예: move, dribble)
    /// - `false`: 커밋 후 취소 불가 (예: shot, pass)
    ///
    /// P7 Phase-Based 시스템에서 Commit 단계 전 취소 가능 여부.
    pub cancellable: bool,

    /// 액션 카테고리
    pub category: ActionCategory,
}

impl ActionMetadata {
    /// 새 메타데이터 생성
    pub const fn new(
        sticky: bool,
        directional: bool,
        cancellable: bool,
        category: ActionCategory,
    ) -> Self {
        Self { sticky, directional, cancellable, category }
    }

    /// 기본 1회성 액션 (sticky=false, directional=false, cancellable=false)
    pub const fn one_shot(category: ActionCategory) -> Self {
        Self::new(false, false, false, category)
    }

    /// Sticky 이동 액션 (sticky=true, directional=true, cancellable=true)
    pub const fn sticky_movement() -> Self {
        Self::new(true, true, true, ActionCategory::Movement)
    }

    /// PhaseActionType에서 메타데이터 파생
    ///
    /// P7 Phase-Based 시스템의 액션 타입을 Google Football 스타일 메타데이터로 변환.
    pub const fn from_phase_action(action: PhaseActionType) -> Self {
        match action {
            // === Movement ===
            // Move: sticky (계속 이동), directional (방향 필요), cancellable
            PhaseActionType::Move => Self::new(true, true, true, ActionCategory::Movement),

            // Dribble: sticky (드리블 유지), directional (방향), cancellable
            PhaseActionType::Dribble => Self::new(true, true, true, ActionCategory::Movement),

            // === Pass ===
            // Pass: one-shot, 커밋 후 취소 불가
            PhaseActionType::Pass => Self::one_shot(ActionCategory::Pass),

            // === Shot ===
            // Shot: one-shot, 커밋 후 취소 불가
            PhaseActionType::Shot => Self::one_shot(ActionCategory::Shot),

            // Header: 상황에 따라 Shot 또는 Pass (기본 Shot)
            PhaseActionType::Header => Self::one_shot(ActionCategory::Shot),

            // === Defense ===
            // Tackle: one-shot, 즉시 실행
            PhaseActionType::Tackle => Self::one_shot(ActionCategory::Defense),

            // Intercept: cancellable (위치 조정 중 취소 가능)
            PhaseActionType::Intercept => Self::new(false, false, true, ActionCategory::Defense),

            // Save (GK): directional (방향으로 다이브), one-shot
            PhaseActionType::Save => Self::new(false, true, false, ActionCategory::Defense),

            // === Other ===
            // Trap: 공 받기, cancellable
            PhaseActionType::Trap => Self::new(false, false, true, ActionCategory::Other),
        }
    }

    /// BehaviorIntent에서 메타데이터 파생
    ///
    /// 세분화된 의도를 메타데이터로 변환.
    /// IntentCategory 기반으로 기본 메타데이터 결정 후 세부 조정.
    pub fn from_intent(intent: BehaviorIntent) -> Self {
        let category = intent.category();

        match category {
            // OnBall: 대부분 one-shot (Pass, Shot), 일부 sticky (Dribble)
            IntentCategory::OnBall => Self::from_onball_intent(intent),

            // OffBall: 대부분 sticky movement (지속적인 움직임)
            IntentCategory::OffBallAttack => Self::sticky_movement(),

            // Defense: 대부분 one-shot 또는 cancellable
            IntentCategory::Defense => Self::from_defense_intent(intent),

            // Transition: 상황에 따라 다름
            IntentCategory::Transition => Self::from_transition_intent(intent),

            // SetPiece: Hold (대기) 상태
            IntentCategory::SetPiece => Self::new(true, false, true, ActionCategory::Other),
        }
    }

    /// OnBall 의도에서 메타데이터 파생
    fn from_onball_intent(intent: BehaviorIntent) -> Self {
        match intent {
            // Dribble 관련: sticky, directional
            BehaviorIntent::OnBall_DribbleAdvance
            | BehaviorIntent::OnBall_DribbleEscape
            | BehaviorIntent::OnBall_ProtectBall => Self::sticky_movement(),

            // Pass 관련: one-shot
            BehaviorIntent::OnBall_SafeRecycle
            | BehaviorIntent::OnBall_SwitchPlay
            | BehaviorIntent::OnBall_ProgressivePass
            | BehaviorIntent::OnBall_ThroughBall
            | BehaviorIntent::OnBall_WideRelease
            | BehaviorIntent::OnBall_CrossEarly
            | BehaviorIntent::OnBall_CrossByline
            | BehaviorIntent::OnBall_Layoff => Self::one_shot(ActionCategory::Pass),

            // Shot 관련: one-shot
            BehaviorIntent::OnBall_Shoot
            | BehaviorIntent::OnBall_ShootLong
            | BehaviorIntent::OnBall_ShootFinesse
            | BehaviorIntent::OnBall_Chip => Self::one_shot(ActionCategory::Shot),

            // Hold: sticky, cancellable
            BehaviorIntent::OnBall_HoldUp => Self::new(true, false, true, ActionCategory::Movement),

            // Clear: one-shot
            BehaviorIntent::OnBall_Clearance => Self::one_shot(ActionCategory::Other),

            // DrawFoul: sticky (드리블 시도)
            BehaviorIntent::OnBall_DrawFoul => Self::sticky_movement(),

            // 기타: 기본값
            _ => Self::one_shot(ActionCategory::Other),
        }
    }

    /// Defense 의도에서 메타데이터 파생
    fn from_defense_intent(intent: BehaviorIntent) -> Self {
        match intent {
            // Tackle: one-shot
            BehaviorIntent::Defend_TackleAttempt => Self::one_shot(ActionCategory::Defense),

            // Press/Jockey: sticky (계속 압박), cancellable
            BehaviorIntent::Defend_PressBallCarrier
            | BehaviorIntent::Defend_JockeyContain
            | BehaviorIntent::Defend_DoubleTeam => {
                Self::new(true, true, true, ActionCategory::Defense)
            }

            // Mark/Cover: sticky positioning
            BehaviorIntent::Defend_MarkTight
            | BehaviorIntent::Defend_TrackRunner
            | BehaviorIntent::Defend_ZonalShape
            | BehaviorIntent::Defend_BlockLane => {
                Self::new(true, false, true, ActionCategory::Defense)
            }

            // Clear: one-shot
            BehaviorIntent::Defend_ClearDanger => Self::one_shot(ActionCategory::Defense),

            // Recovery: sticky movement
            BehaviorIntent::Defend_RecoverRun => Self::sticky_movement(),

            // 기타
            _ => Self::new(false, false, true, ActionCategory::Defense),
        }
    }

    /// Transition 의도에서 메타데이터 파생
    fn from_transition_intent(intent: BehaviorIntent) -> Self {
        match intent {
            // CounterPress: sticky (계속 압박)
            BehaviorIntent::Transition_CounterPress => {
                Self::new(true, true, true, ActionCategory::Defense)
            }

            // DropToShape: sticky movement
            BehaviorIntent::Transition_DropToShape => Self::sticky_movement(),

            // CounterAttack: sticky movement
            BehaviorIntent::Transition_CounterAttack => Self::sticky_movement(),

            // SecureFirstPass: one-shot pass
            BehaviorIntent::Transition_SecureFirstPass => Self::one_shot(ActionCategory::Pass),

            // SpreadOutlets: sticky movement
            BehaviorIntent::Transition_SpreadOutlets => Self::sticky_movement(),

            // FoulToStop: one-shot defense
            BehaviorIntent::Transition_FoulToStop => Self::one_shot(ActionCategory::Defense),

            // 기타
            _ => Self::new(false, false, true, ActionCategory::Other),
        }
    }

    /// Sticky 액션인지 확인
    pub const fn is_sticky(&self) -> bool {
        self.sticky
    }

    /// 방향성 액션인지 확인
    pub const fn is_directional(&self) -> bool {
        self.directional
    }

    /// 취소 가능한지 확인
    pub const fn is_cancellable(&self) -> bool {
        self.cancellable
    }
}

// ============================================================================
// Release Action Mapping
// ============================================================================

/// Sticky 액션의 Release 액션 매핑 (Google Football 패턴)
///
/// Google Football에서는 sticky 액션을 해제하기 위한 명시적 release_* 액션이 존재.
/// - `release_sprint`: 스프린트 해제
/// - `release_dribble`: 드리블 해제
/// - `release_direction`: 방향 입력 해제 (정지)
///
/// of_core에서는 PhaseActionType::Move가 기본 상태이므로,
/// sticky 액션 해제 시 Move로 전환.
pub fn get_release_action(action: PhaseActionType) -> Option<PhaseActionType> {
    match action {
        // Dribble 해제 → Move (드리블 없이 이동)
        PhaseActionType::Dribble => Some(PhaseActionType::Move),

        // Move는 기본 상태 → 해제 액션 없음 (idle로 전환은 별도 처리)
        PhaseActionType::Move => None,

        // 나머지는 sticky가 아니므로 해제 액션 없음
        _ => None,
    }
}

/// BehaviorIntent에 해당하는 Release Intent 반환
///
/// Sticky intent를 해제할 때 사용할 intent 반환.
/// None이면 해제 액션이 불필요한 one-shot 액션.
pub fn get_release_intent(intent: BehaviorIntent) -> Option<BehaviorIntent> {
    match intent {
        // Dribble → Support (드리블 해제, 지원 위치로)
        BehaviorIntent::OnBall_DribbleAdvance | BehaviorIntent::OnBall_DribbleEscape => {
            Some(BehaviorIntent::OnBall_HoldUp)
        }

        // Press → Zonal (압박 해제, 수비 형태로)
        BehaviorIntent::Defend_PressBallCarrier | BehaviorIntent::Defend_DoubleTeam => {
            Some(BehaviorIntent::Defend_ZonalShape)
        }

        // CounterPress → DropToShape (역압박 해제, 복귀)
        BehaviorIntent::Transition_CounterPress => Some(BehaviorIntent::Transition_DropToShape),

        // 나머지는 해제 불필요
        _ => None,
    }
}

/// 액션이 Sticky인지 확인 (PhaseActionType 기준)
pub const fn is_sticky_action(action: PhaseActionType) -> bool {
    ActionMetadata::from_phase_action(action).sticky
}

/// Intent가 Sticky인지 확인
pub fn is_sticky_intent(intent: BehaviorIntent) -> bool {
    ActionMetadata::from_intent(intent).sticky
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === ActionMetadata Tests ===

    #[test]
    fn test_from_phase_action_move() {
        let meta = ActionMetadata::from_phase_action(PhaseActionType::Move);
        assert!(meta.sticky);
        assert!(meta.directional);
        assert!(meta.cancellable);
        assert_eq!(meta.category, ActionCategory::Movement);
    }

    #[test]
    fn test_from_phase_action_dribble() {
        let meta = ActionMetadata::from_phase_action(PhaseActionType::Dribble);
        assert!(meta.sticky);
        assert!(meta.directional);
        assert!(meta.cancellable);
        assert_eq!(meta.category, ActionCategory::Movement);
    }

    #[test]
    fn test_from_phase_action_pass() {
        let meta = ActionMetadata::from_phase_action(PhaseActionType::Pass);
        assert!(!meta.sticky);
        assert!(!meta.directional);
        assert!(!meta.cancellable);
        assert_eq!(meta.category, ActionCategory::Pass);
    }

    #[test]
    fn test_from_phase_action_shot() {
        let meta = ActionMetadata::from_phase_action(PhaseActionType::Shot);
        assert!(!meta.sticky);
        assert!(!meta.directional);
        assert!(!meta.cancellable);
        assert_eq!(meta.category, ActionCategory::Shot);
    }

    #[test]
    fn test_from_phase_action_tackle() {
        let meta = ActionMetadata::from_phase_action(PhaseActionType::Tackle);
        assert!(!meta.sticky);
        assert!(!meta.directional);
        assert!(!meta.cancellable);
        assert_eq!(meta.category, ActionCategory::Defense);
    }

    #[test]
    fn test_from_phase_action_intercept() {
        let meta = ActionMetadata::from_phase_action(PhaseActionType::Intercept);
        assert!(!meta.sticky);
        assert!(!meta.directional);
        assert!(meta.cancellable); // Intercept is cancellable
        assert_eq!(meta.category, ActionCategory::Defense);
    }

    #[test]
    fn test_from_phase_action_save() {
        let meta = ActionMetadata::from_phase_action(PhaseActionType::Save);
        assert!(!meta.sticky);
        assert!(meta.directional); // Save requires direction
        assert!(!meta.cancellable);
        assert_eq!(meta.category, ActionCategory::Defense);
    }

    #[test]
    fn test_all_phase_actions_have_metadata() {
        let actions = [
            PhaseActionType::Tackle,
            PhaseActionType::Pass,
            PhaseActionType::Shot,
            PhaseActionType::Dribble,
            PhaseActionType::Move,
            PhaseActionType::Trap,
            PhaseActionType::Intercept,
            PhaseActionType::Header,
            PhaseActionType::Save,
        ];

        for action in actions {
            let meta = ActionMetadata::from_phase_action(action);
            // Just verify it doesn't panic and has valid category
            assert!(ActionCategory::all().contains(&meta.category));
        }
    }

    // === Intent Metadata Tests ===

    #[test]
    fn test_from_intent_onball_shoot() {
        let meta = ActionMetadata::from_intent(BehaviorIntent::OnBall_Shoot);
        assert!(!meta.sticky);
        assert_eq!(meta.category, ActionCategory::Shot);
    }

    #[test]
    fn test_from_intent_onball_dribble() {
        let meta = ActionMetadata::from_intent(BehaviorIntent::OnBall_DribbleAdvance);
        assert!(meta.sticky);
        assert!(meta.directional);
        assert_eq!(meta.category, ActionCategory::Movement);
    }

    #[test]
    fn test_from_intent_defend_press() {
        let meta = ActionMetadata::from_intent(BehaviorIntent::Defend_PressBallCarrier);
        assert!(meta.sticky);
        assert!(meta.cancellable);
        assert_eq!(meta.category, ActionCategory::Defense);
    }

    #[test]
    fn test_from_intent_offball() {
        let meta = ActionMetadata::from_intent(BehaviorIntent::OffBall_RunInBehind);
        assert!(meta.sticky);
        assert!(meta.directional);
        assert_eq!(meta.category, ActionCategory::Movement);
    }

    #[test]
    fn test_all_intents_have_valid_metadata() {
        for intent in BehaviorIntent::all() {
            let meta = ActionMetadata::from_intent(*intent);
            assert!(
                ActionCategory::all().contains(&meta.category),
                "Intent {:?} has invalid category {:?}",
                intent,
                meta.category
            );
        }
    }

    // === Release Action Tests ===

    #[test]
    fn test_release_dribble() {
        let release = get_release_action(PhaseActionType::Dribble);
        assert_eq!(release, Some(PhaseActionType::Move));
    }

    #[test]
    fn test_release_move_none() {
        let release = get_release_action(PhaseActionType::Move);
        assert_eq!(release, None);
    }

    #[test]
    fn test_release_pass_none() {
        let release = get_release_action(PhaseActionType::Pass);
        assert_eq!(release, None);
    }

    #[test]
    fn test_release_intent_dribble() {
        let release = get_release_intent(BehaviorIntent::OnBall_DribbleAdvance);
        assert_eq!(release, Some(BehaviorIntent::OnBall_HoldUp));
    }

    #[test]
    fn test_release_intent_press() {
        let release = get_release_intent(BehaviorIntent::Defend_PressBallCarrier);
        assert_eq!(release, Some(BehaviorIntent::Defend_ZonalShape));
    }

    #[test]
    fn test_release_intent_shoot_none() {
        let release = get_release_intent(BehaviorIntent::OnBall_Shoot);
        assert_eq!(release, None);
    }

    // === Sticky Check Tests ===

    #[test]
    fn test_is_sticky_action() {
        assert!(is_sticky_action(PhaseActionType::Move));
        assert!(is_sticky_action(PhaseActionType::Dribble));
        assert!(!is_sticky_action(PhaseActionType::Pass));
        assert!(!is_sticky_action(PhaseActionType::Shot));
        assert!(!is_sticky_action(PhaseActionType::Tackle));
    }

    #[test]
    fn test_is_sticky_intent() {
        assert!(is_sticky_intent(BehaviorIntent::OnBall_DribbleAdvance));
        assert!(is_sticky_intent(BehaviorIntent::Defend_PressBallCarrier));
        assert!(!is_sticky_intent(BehaviorIntent::OnBall_Shoot));
        assert!(!is_sticky_intent(BehaviorIntent::OnBall_SafeRecycle));
    }

    // === Category Tests ===

    #[test]
    fn test_action_category_all() {
        let categories = ActionCategory::all();
        assert_eq!(categories.len(), 6);
        assert!(categories.contains(&ActionCategory::Movement));
        assert!(categories.contains(&ActionCategory::Pass));
        assert!(categories.contains(&ActionCategory::Shot));
        assert!(categories.contains(&ActionCategory::Defense));
        assert!(categories.contains(&ActionCategory::Release));
        assert!(categories.contains(&ActionCategory::Other));
    }
}
