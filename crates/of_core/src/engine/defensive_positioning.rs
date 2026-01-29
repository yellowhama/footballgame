//! Defensive Positioning System
//!
//! P7 Phase 7: 수비 팀의 포지셔닝을 Phase 기반으로 관리
//!
//! ## 핵심 기능
//! - 역할 분담 (Presser, Marker, Cover)
//! - 팀 슬라이드 (공 위치에 따른 전체 이동)
//! - Presser Phase FSM (접근 → 압박 → 태클 → 회복)
//! - PlayerState 존중

use crate::engine::body_blocking::distance;
use crate::engine::physics_constants::field;
use crate::engine::player_state::PlayerState;
use crate::engine::steering::{arrive, pursuit, seek, separation};
use crate::engine::tactical_context::MatchSituation;
use crate::fix01::TeamShapeParams;

// ============================================================================
// Constants
// ============================================================================

/// Presser 상수
pub mod presser_constants {
    /// 압박 목표 거리 (m)
    pub const PRESS_TARGET_DISTANCE: f32 = 1.8;

    /// 압박 유지 최소 거리 (m)
    pub const PRESS_MIN_DISTANCE: f32 = 1.2;

    /// 압박 유지 최대 거리 (m)
    pub const PRESS_MAX_DISTANCE: f32 = 2.5;

    /// 접근 속도 (m/s)
    pub const PRESS_APPROACH_SPEED: f32 = 4.5;

    /// 압박 유지 시 따라가는 속도 (m/s)
    pub const PRESS_FOLLOW_SPEED: f32 = 3.5;

    /// 위치 조정 속도 (m/s)
    pub const PRESS_ADJUST_SPEED: f32 = 2.0;

    /// 태클 시도 거리 (m)
    pub const TACKLE_ATTEMPT_DISTANCE: f32 = 1.5;

    /// 압박 유지 최대 틱 (이후 강제 회복)
    pub const MAX_PRESS_TICKS: u16 = 80; // 20초

    /// 태클 후 회복 틱
    pub const PRESS_RECOVERY_TICKS: u8 = 16; // 4초

    /// 틱당 시간 (s)
    pub const TICK_DT: f32 = 0.25;

    // FIX_2601/0106 P2: 스태미나 휴식 상수 (0-1 스케일로 수정)
    /// 스태미나 임계값 (0-1) - 이 이하면 휴식 필요
    pub const STAMINA_REST_THRESHOLD: f32 = 0.30;

    /// 휴식 시작 후 복귀 스태미나 임계값 (0-1)
    /// 70%까지 회복해야 복귀 - open-football은 90%, 우리는 70%로 절충
    pub const STAMINA_RESUME_THRESHOLD: f32 = 0.70;

    /// 휴식 틱 (기본값) - 40틱 = 10초
    pub const RESTING_TICKS: u16 = 40;

    /// 휴식 중 이동 속도 (m/s) - 걷기
    pub const RESTING_WALK_SPEED: f32 = 1.5;

    /// 공 근접 임계값 (m) - 이 이내면 휴식 중이라도 정상 행동
    pub const BALL_PROXIMITY_THRESHOLD: f32 = 10.0;
}

/// Cover/Marker 상수
pub mod movement_constants {
    /// Cover 이동 속도 (m/s)
    pub const COVER_MOVE_SPEED: f32 = 4.0;

    /// Marker 이동 속도 (m/s)
    pub const MARKER_MOVE_SPEED: f32 = 4.5;

    /// 마킹 오프셋 (m)
    pub const MARKING_OFFSET: f32 = 1.5;

    /// 마킹 범위 (m)
    pub const MARKING_RANGE: f32 = 15.0;

    /// 틱당 시간 (s)
    pub const TICK_DT: f32 = 0.25;
}

/// 팀 슬라이드 상수
pub mod slide_constants {
    /// 좌우 최대 슬라이드 거리 (m)
    pub const SIDE_SHIFT_MAX: f32 = 12.0;
}

/// GK 상수
pub mod gk_constants {
    /// GK 최소 전진 거리 (m)
    pub const GK_MIN_ADVANCE: f32 = 2.0;

    /// GK 최대 전진 거리 (m)
    pub const GK_MAX_ADVANCE: f32 = 12.0;

    /// GK 이동 속도 (m/s)
    pub const GK_MOVE_SPEED: f32 = 5.0;

    /// 틱당 시간 (s)
    pub const TICK_DT: f32 = 0.25;
}

// TICK_DT는 하나만 사용 (중복 방지)
pub use gk_constants::{GK_MAX_ADVANCE, GK_MIN_ADVANCE, GK_MOVE_SPEED};
pub use movement_constants::{COVER_MOVE_SPEED, MARKER_MOVE_SPEED, MARKING_OFFSET, MARKING_RANGE};
pub use presser_constants::{
    BALL_PROXIMITY_THRESHOLD,
    MAX_PRESS_TICKS,
    PRESS_ADJUST_SPEED,
    PRESS_APPROACH_SPEED,
    PRESS_FOLLOW_SPEED,
    PRESS_MAX_DISTANCE,
    PRESS_MIN_DISTANCE,
    PRESS_RECOVERY_TICKS,
    PRESS_TARGET_DISTANCE,
    RESTING_TICKS,
    RESTING_WALK_SPEED,
    STAMINA_REST_THRESHOLD,
    STAMINA_RESUME_THRESHOLD,
    TACKLE_ATTEMPT_DISTANCE,
    TICK_DT, // This is the canonical TICK_DT
};
pub use slide_constants::SIDE_SHIFT_MAX;

// ============================================================================
// Types
// ============================================================================

// P17: TeamSide를 models에서 re-export
pub use crate::models::TeamSide;

// ========================================================================
// DefensiveTuning (P0 Patch 2) - Tactical configuration from TeamInstructions
// ========================================================================

/// Defensive tactical configuration from TeamInstructions
///
/// Applied from decision_topology::apply_team_instructions() to control
/// pressing intensity and offside trap behavior.
///
/// P4: Extended with MatchSituation modifiers
#[derive(Debug, Clone, Copy)]
pub struct DefensiveTuning {
    /// Pressing factor (0.0 = very low, 1.0 = very high)
    ///
    /// Affects pressing range and presser selection threshold.
    pub pressing_factor: f32,

    /// Offside trap enabled (from TeamInstructions.use_offside_trap)
    ///
    /// Affects defensive line positioning and step-up behavior.
    pub offside_trap_enabled: bool,

    // === P4: MatchSituation 기반 동적 조정 ===
    /// Situation pressing modifier (from MatchSituation.get_pressing_modifier())
    /// Range: 0.4 - 1.5
    pub situation_pressing_mod: f32,

    /// Situation defensive line adjustment (from MatchSituation.get_defensive_line_adjustment())
    /// Range: -0.15 to +0.2 (meters adjustment factor)
    pub situation_line_adjust: f32,

    /// High pressure situation flag (from MatchSituation.is_high_pressure())
    /// True when team is trailing and needs to score
    pub is_high_pressure: bool,
}

impl Default for DefensiveTuning {
    fn default() -> Self {
        Self {
            pressing_factor: 0.6,        // Medium pressing
            offside_trap_enabled: false, // No offside trap by default
            // P4: Default situation modifiers (neutral)
            situation_pressing_mod: 1.0,
            situation_line_adjust: 0.0,
            is_high_pressure: false,
        }
    }
}

impl DefensiveTuning {
    /// P4: Create tuning with MatchSituation modifiers applied
    ///
    /// # Arguments
    /// * `base` - Base tuning from TeamInstructions
    /// * `situation` - Current match situation
    /// * `side` - Team side (Home/Away)
    pub fn with_situation(base: Self, situation: &MatchSituation, side: TeamSide) -> Self {
        Self {
            pressing_factor: base.pressing_factor,
            offside_trap_enabled: base.offside_trap_enabled,
            situation_pressing_mod: situation.get_pressing_modifier(side),
            situation_line_adjust: situation.get_defensive_line_adjustment(side),
            is_high_pressure: situation.is_high_pressure(side),
        }
    }

    /// P4: Calculate effective pressing factor
    /// Combines base pressing_factor with situation modifier
    pub fn effective_pressing(&self) -> f32 {
        (self.pressing_factor * self.situation_pressing_mod).clamp(0.3, 1.5)
    }

    /// P4: Calculate effective defensive line depth adjustment (in meters)
    /// Positive = higher line, Negative = deeper line
    pub fn effective_line_adjust_meters(&self) -> f32 {
        // Scale situation adjustment to meters (±0.2 * 10 = ±2m max)
        self.situation_line_adjust * 10.0
    }
}

// ========================================================================
// End DefensiveTuning
// ========================================================================

/// 수비 라인 높이 설정
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefensiveLine {
    VeryHigh,
    High,
    #[default]
    Normal,
    Deep,
    VeryDeep,
}

/// 수비 시 선수의 역할
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefensiveRole {
    /// 1순위 압박자 (공에 직접 붙음)
    PresserPrimary,

    /// 2순위 압박자 (골-볼 사이 커버, 1순위 실패 시 대기)
    PresserSecondary,

    /// 마커 (특정 상대 따라다님)
    Marker { target_idx: usize },

    /// 커버 (라인 유지 + 팀 슬라이드)
    #[default]
    Cover,

    /// 골키퍼 (특수)
    Goalkeeper,
}

/// 압박자의 Phase
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresserPhase {
    /// 공 쪽으로 접근 중
    Closing { target_distance: f32 },

    /// 압박 유지 (공과 일정 거리 유지, 태클 타이밍 대기)
    Pressing { hold_distance: f32, hold_ticks: u16 },

    /// 태클 시도 중 (TackleAction에 위임)
    Tackling { tackle_action_id: u64 },

    /// 태클 후 회복/재정비
    Recovering { remaining_ticks: u8 },

    /// FIX_2601/0106 P2: 스태미나 부족으로 휴식 중
    /// 휴식 중에는 천천히 걸으며 팀 슬라이드만 따라감
    Resting {
        /// 남은 휴식 틱
        remaining_ticks: u16,
        /// 휴식 시작 전 복귀할 Phase
        return_to_closing: bool,
    },
}

/// Presser에서 발생하는 이벤트
#[derive(Debug, Clone)]
pub enum PresserEvent {
    /// 압박 시작
    StartedPressing,
    /// 태클 시작
    StartedTackle { action_id: u64 },
    /// 압박 시간 초과
    PressTimeout,
    /// 회복 완료
    RecoveryComplete,
    /// FIX_2601/0106 P2: 휴식 시작 (스태미나 부족)
    StartedResting { stamina: f32 },
    /// FIX_2601/0106 P2: 휴식 완료
    RestingComplete,
}

// ============================================================================
// FIX_2601/0106 P3: MarkerPhase FSM
// ============================================================================

/// 마커의 Phase (상태 세분화)
///
/// FIX_2601/0106 P3: 마킹 상태를 세분화하여 더 현실적인 수비 행동 구현
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkerPhase {
    /// 마킹 대상을 따라가는 중 (기본)
    /// 목표: 대상과 골문 사이에 위치
    Following {
        /// 목표 오프셋 거리 (m)
        offset_distance: f32,
    },

    /// 패스 경로 차단 중 (볼이 근처에 있을 때)
    /// 목표: 공과 대상 사이 가로막기
    Shadowing {
        /// 차단 위치까지 남은 거리 (m)
        intercept_distance: f32,
    },

    /// 인터셉트 시도 중 (공이 대상에게 향할 때)
    /// 목표: 공을 가로채기
    Intercepting {
        /// 예상 인터셉트 지점
        intercept_point: (f32, f32),
        /// 남은 도달 시간 (ticks)
        arrival_ticks: u16,
    },

    /// 휴식 중 (스태미나 부족)
    Resting { remaining_ticks: u16 },
}

/// 마커 이벤트
#[derive(Debug, Clone)]
pub enum MarkerEvent {
    /// Shadowing 시작
    StartedShadowing,
    /// 인터셉트 시도 시작
    StartedIntercept { intercept_point: (f32, f32) },
    /// 인터셉트 성공
    InterceptSuccess,
    /// 인터셉트 실패 (대상이 먼저 받음)
    InterceptFailed,
    /// 휴식 시작
    StartedResting { stamina: f32 },
    /// 휴식 완료
    RestingComplete,
}

// ============================================================================
// FIX_2601/0106 P3: CoverPhase FSM
// ============================================================================

/// 커버의 Phase (상태 세분화)
///
/// FIX_2601/0106 P3: 커버 수비 상태를 세분화하여 라인 유지 및 공간 커버 구현
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverPhase {
    /// 라인 유지 (기본)
    /// 목표: 수비 라인에서 팀 슬라이드 따라 이동
    Holding {
        /// 목표 위치
        target_pos: (f32, f32),
    },

    /// 위치 조정 중 (팀 슬라이드 대응)
    Adjusting {
        /// 이동 목표
        target_pos: (f32, f32),
        /// 조정 속도 배율
        speed_mult: f32,
    },

    /// 공간 커버 (위험 공간 차단)
    /// 목표: 공과 골문 사이 위험 공간 커버
    SpaceCovering {
        /// 커버 중심점
        cover_center: (f32, f32),
        /// 커버 반경 (m)
        cover_radius: f32,
    },

    /// 긴급 복귀 (공이 뒤로 돌파했을 때)
    Recovering {
        /// 복귀 목표 위치
        fallback_pos: (f32, f32),
    },

    /// 휴식 중 (스태미나 부족)
    Resting { remaining_ticks: u16 },
}

/// 커버 이벤트
#[derive(Debug, Clone)]
pub enum CoverEvent {
    /// 위치 조정 시작
    StartedAdjusting,
    /// 공간 커버 시작
    StartedSpaceCover { cover_center: (f32, f32) },
    /// 긴급 복귀 시작
    StartedRecovering { fallback_pos: (f32, f32) },
    /// 라인 복귀 완료
    HoldingComplete,
    /// 휴식 시작
    StartedResting { stamina: f32 },
    /// 휴식 완료
    RestingComplete,
}

// ============================================================================
// P3b: Unified Defensive State System
// ============================================================================

/// P3b: 상태 종류 (return_to에 사용)
/// 휴식 후 복귀할 상태를 지정
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefensiveStateKind {
    // Presser states
    PresserClosing,
    PresserPressing,
    // Marker states
    MarkerFollowing,
    MarkerShadowing,
    // Cover states
    CoverHolding,
    CoverAdjusting,
}

/// P3b: 통합 수비 상태 (3개 FSM → 1개)
///
/// 기존 PresserPhase, MarkerPhase, CoverPhase를 하나로 통합.
/// 공통 상태(Resting, Recovering)는 한 번만 정의하여 중복 제거.
#[derive(Clone, Debug, PartialEq)]
pub enum DefensiveState {
    // === 공통 상태 (모든 역할) ===
    /// 휴식 중 (스태미나 부족)
    Resting { remaining_ticks: u16, return_to: DefensiveStateKind },
    /// 회복 중 (태클 후, 돌파당함 후 등)
    Recovering { remaining_ticks: u8, fallback_pos: Option<(f32, f32)> },

    // === Presser 전용 ===
    /// 공 쪽으로 접근 중
    PresserClosing { target_distance: f32 },
    /// 압박 유지 (공과 일정 거리 유지)
    PresserPressing { hold_distance: f32, hold_ticks: u16 },
    /// 태클 시도 중
    PresserTackling { tackle_action_id: u64 },

    // === Marker 전용 ===
    /// 마킹 대상 따라가는 중
    MarkerFollowing { offset_distance: f32 },
    /// 패스 경로 차단 중
    MarkerShadowing { intercept_distance: f32 },
    /// 인터셉트 시도 중
    MarkerIntercepting { intercept_point: (f32, f32), arrival_ticks: u16 },

    // === Cover 전용 ===
    /// 라인 유지 (기본)
    CoverHolding { target_pos: (f32, f32) },
    /// 위치 조정 중
    CoverAdjusting { target_pos: (f32, f32), speed_mult: f32 },
    /// 공간 커버 (위험 공간 차단)
    CoverSpaceCovering { cover_center: (f32, f32), cover_radius: f32 },
}

impl DefensiveState {
    /// 현재 상태를 StateKind로 변환 (Resting 복귀용)
    pub fn to_kind(&self) -> DefensiveStateKind {
        match self {
            Self::PresserClosing { .. } => DefensiveStateKind::PresserClosing,
            Self::PresserPressing { .. } => DefensiveStateKind::PresserPressing,
            Self::PresserTackling { .. } => DefensiveStateKind::PresserPressing, // 태클 후는 Pressing으로
            Self::MarkerFollowing { .. } => DefensiveStateKind::MarkerFollowing,
            Self::MarkerShadowing { .. } => DefensiveStateKind::MarkerShadowing,
            Self::MarkerIntercepting { .. } => DefensiveStateKind::MarkerFollowing, // 인터셉트 후는 Following으로
            Self::CoverHolding { .. } => DefensiveStateKind::CoverHolding,
            Self::CoverAdjusting { .. } => DefensiveStateKind::CoverAdjusting,
            Self::CoverSpaceCovering { .. } => DefensiveStateKind::CoverHolding, // 커버 후는 Holding으로
            Self::Resting { return_to, .. } => *return_to,
            Self::Recovering { .. } => DefensiveStateKind::CoverHolding, // 복구 후 기본은 Holding
        }
    }

    /// 역할과 상태가 일치하는지 검증 (debug only)
    #[cfg(debug_assertions)]
    pub fn is_valid_for_role(&self, role: &DefensiveRole) -> bool {
        match (role, self) {
            // Presser 역할은 Presser 상태 또는 공통 상태만 가능
            (
                DefensiveRole::PresserPrimary | DefensiveRole::PresserSecondary,
                Self::PresserClosing { .. }
                | Self::PresserPressing { .. }
                | Self::PresserTackling { .. }
                | Self::Resting { .. }
                | Self::Recovering { .. },
            ) => true,

            // Marker 역할
            (
                DefensiveRole::Marker { .. },
                Self::MarkerFollowing { .. }
                | Self::MarkerShadowing { .. }
                | Self::MarkerIntercepting { .. }
                | Self::Resting { .. },
            ) => true,

            // Cover 역할
            (
                DefensiveRole::Cover,
                Self::CoverHolding { .. }
                | Self::CoverAdjusting { .. }
                | Self::CoverSpaceCovering { .. }
                | Self::Resting { .. }
                | Self::Recovering { .. },
            ) => true,

            // GK는 Cover로 처리
            (DefensiveRole::Goalkeeper, _) => true,

            _ => false,
        }
    }
}

/// P3b: 통합 수비 이벤트
#[derive(Debug, Clone)]
pub enum DefensiveEvent {
    // === Presser 이벤트 ===
    StartedPressing,
    StartedTackle { action_id: u64 },
    PressTimeout,

    // === Marker 이벤트 ===
    StartedShadowing,
    StartedIntercept { intercept_point: (f32, f32) },
    InterceptSuccess,
    InterceptFailed,

    // === Cover 이벤트 ===
    StartedAdjusting,
    StartedSpaceCover { cover_center: (f32, f32) },
    HoldingComplete,

    // === 공통 이벤트 ===
    StartedResting { stamina: f32, role: DefensiveRole },
    RestingComplete { role: DefensiveRole },
    RecoveryComplete,
    StartedRecovering { fallback_pos: (f32, f32) },
}

/// P3b: 통합 수비 이동 구조체
///
/// 기존 PresserMovement, MarkerMovement, CoverMovement를 하나로 통합
#[derive(Debug, Clone)]
pub struct DefensiveMovement {
    /// 선수 인덱스 (0-21)
    pub player_idx: usize,
    /// 수비 역할
    pub role: DefensiveRole,
    /// 현재 상태
    pub state: DefensiveState,
    /// 마킹 대상 인덱스 (Marker 역할에서만 사용)
    pub target_mark_idx: Option<usize>,
}

impl DefensiveMovement {
    /// Presser 역할로 생성
    pub fn new_presser(player_idx: usize, is_primary: bool) -> Self {
        Self {
            player_idx,
            role: if is_primary {
                DefensiveRole::PresserPrimary
            } else {
                DefensiveRole::PresserSecondary
            },
            state: DefensiveState::PresserClosing { target_distance: PRESS_TARGET_DISTANCE },
            target_mark_idx: None,
        }
    }

    /// Marker 역할로 생성
    pub fn new_marker(player_idx: usize, target_idx: usize) -> Self {
        Self {
            player_idx,
            role: DefensiveRole::Marker { target_idx },
            state: DefensiveState::MarkerFollowing { offset_distance: MARKING_OFFSET },
            target_mark_idx: Some(target_idx),
        }
    }

    /// Cover 역할로 생성
    pub fn new_cover(player_idx: usize, target_pos: (f32, f32)) -> Self {
        Self {
            player_idx,
            role: DefensiveRole::Cover,
            state: DefensiveState::CoverHolding { target_pos },
            target_mark_idx: None,
        }
    }

    /// P3b: 공통 스태미나 체크 및 Resting 진입
    ///
    /// 3개 FSM에서 중복되던 로직을 하나로 통합
    pub fn check_stamina_and_enter_resting(
        &mut self,
        stamina: f32,
        ball_distance: f32,
    ) -> Option<DefensiveEvent> {
        // 이미 휴식 중이면 무시
        if matches!(self.state, DefensiveState::Resting { .. }) {
            return None;
        }

        // 스태미나 충분하면 무시
        if stamina >= STAMINA_REST_THRESHOLD {
            return None;
        }

        // 공이 가까우면 휴식 금지
        if ball_distance <= BALL_PROXIMITY_THRESHOLD {
            return None;
        }

        // 휴식 상태로 전환
        let return_to = self.state.to_kind();
        self.state = DefensiveState::Resting { remaining_ticks: RESTING_TICKS, return_to };

        Some(DefensiveEvent::StartedResting { stamina, role: self.role })
    }
}

// ============================================================================
// End P3b: Unified Defensive State System
// ============================================================================

// ============================================================================
// PresserMovement
// ============================================================================

/// 압박자 움직임 관리
#[derive(Debug, Clone)]
pub struct PresserMovement {
    pub player_idx: usize,
    pub phase: PresserPhase,
    pub role: DefensiveRole,
}

impl PresserMovement {
    /// 새 PresserMovement 생성
    pub fn new(player_idx: usize, role: DefensiveRole) -> Self {
        Self {
            player_idx,
            phase: PresserPhase::Closing { target_distance: PRESS_TARGET_DISTANCE },
            role,
        }
    }

    /// 매 틱 업데이트 (스태미나 없이 - 레거시 호환)
    pub fn update_tick<F, G>(
        &mut self,
        player_pos: &mut (f32, f32),
        player_state: &PlayerState,
        ball_pos: (f32, f32),
        ball_owner_idx: Option<usize>,
        tackle_cooldown: u8,
        can_tackle_fn: F,
        schedule_tackle_fn: G,
    ) -> Option<PresserEvent>
    where
        F: Fn() -> bool,
        G: FnOnce() -> u64,
    {
        // 스태미나 100%로 호출 (레거시 호환)
        self.update_tick_with_stamina(
            player_pos,
            player_state,
            ball_pos,
            ball_owner_idx,
            tackle_cooldown,
            1.0,  // 스태미나 100% (0-1 스케일)
            None, // 휴식 목표 위치 없음
            can_tackle_fn,
            schedule_tackle_fn,
        )
    }

    /// FIX_2601/0106 P2: 매 틱 업데이트 (스태미나 포함)
    ///
    /// # Arguments
    /// * `player_pos` - 선수 위치 (미터)
    /// * `player_state` - 선수 상태
    /// * `ball_pos` - 공 위치 (미터)
    /// * `ball_owner_idx` - 공 소유자 인덱스
    /// * `tackle_cooldown` - 태클 쿨다운
    /// * `stamina` - 스태미나 (0-100%)
    /// * `rest_target_pos` - 휴식 시 이동할 목표 위치 (팀 슬라이드 위치)
    /// * `can_tackle_fn` - 태클 가능 여부 함수
    /// * `schedule_tackle_fn` - 태클 예약 함수
    pub fn update_tick_with_stamina<F, G>(
        &mut self,
        player_pos: &mut (f32, f32),
        player_state: &PlayerState,
        ball_pos: (f32, f32),
        _ball_owner_idx: Option<usize>,
        tackle_cooldown: u8,
        stamina: f32,
        rest_target_pos: Option<(f32, f32)>,
        can_tackle_fn: F,
        schedule_tackle_fn: G,
    ) -> Option<PresserEvent>
    where
        F: Fn() -> bool,
        G: FnOnce() -> u64,
    {
        // PlayerState가 Idle/Moving이 아니면 아무것도 안 함
        if !player_state.can_move() {
            return None;
        }

        // Extract phase data to avoid borrow issues
        let phase = self.phase;

        // 공 근접 체크
        let dist_to_ball = distance(*player_pos, ball_pos);
        let ball_is_near = dist_to_ball < BALL_PROXIMITY_THRESHOLD;

        // FIX_2601/0106 P2: 스태미나 체크 (Resting이 아닐 때만, 공이 멀 때만 휴식 진입)
        if !matches!(phase, PresserPhase::Resting { .. })
            && stamina < STAMINA_REST_THRESHOLD
            && !ball_is_near
        {
            let return_to_closing = matches!(phase, PresserPhase::Closing { .. });
            self.phase =
                PresserPhase::Resting { remaining_ticks: RESTING_TICKS, return_to_closing };
            return Some(PresserEvent::StartedResting { stamina });
        }

        match phase {
            PresserPhase::Closing { target_distance } => {
                self.tick_closing(player_pos, ball_pos, target_distance)
            }

            PresserPhase::Pressing { hold_distance, hold_ticks } => self.tick_pressing(
                player_pos,
                ball_pos,
                hold_distance,
                hold_ticks,
                tackle_cooldown,
                can_tackle_fn,
                schedule_tackle_fn,
            ),

            PresserPhase::Tackling { .. } => {
                // TackleAction FSM이 처리 → 여기서는 대기
                None
            }

            PresserPhase::Recovering { remaining_ticks } => self.tick_recovering(remaining_ticks),

            PresserPhase::Resting { remaining_ticks, return_to_closing } => self.tick_resting(
                player_pos,
                ball_pos,
                rest_target_pos,
                remaining_ticks,
                return_to_closing,
                stamina,
            ),
        }
    }

    /// Closing Phase
    fn tick_closing(
        &mut self,
        player_pos: &mut (f32, f32),
        ball_pos: (f32, f32),
        target_distance: f32,
    ) -> Option<PresserEvent> {
        let dist = distance(*player_pos, ball_pos);

        if dist <= target_distance {
            // 목표 거리 도달 → Pressing으로 전환
            self.phase = PresserPhase::Pressing { hold_distance: target_distance, hold_ticks: 0 };
            return Some(PresserEvent::StartedPressing);
        }

        // 공 쪽으로 이동 (P3a: steering seek 사용)
        let velocity = seek(*player_pos, ball_pos, PRESS_APPROACH_SPEED);
        player_pos.0 += velocity.0 * TICK_DT;
        player_pos.1 += velocity.1 * TICK_DT;

        None
    }

    /// Pressing Phase
    fn tick_pressing<F, G>(
        &mut self,
        player_pos: &mut (f32, f32),
        ball_pos: (f32, f32),
        hold_distance: f32,
        hold_ticks: u16,
        tackle_cooldown: u8,
        can_tackle_fn: F,
        schedule_tackle_fn: G,
    ) -> Option<PresserEvent>
    where
        F: Fn() -> bool,
        G: FnOnce() -> u64,
    {
        let new_hold_ticks = hold_ticks + 1;

        let dist = distance(*player_pos, ball_pos);

        // === 거리 조정 (P3a: steering seek 사용) ===
        if dist > hold_distance + 0.5 {
            // 공이 멀어졌으면 따라감
            let velocity = seek(*player_pos, ball_pos, PRESS_FOLLOW_SPEED);
            player_pos.0 += velocity.0 * TICK_DT;
            player_pos.1 += velocity.1 * TICK_DT;
        } else if dist < hold_distance - 0.3 {
            // 너무 가까우면 살짝 물러남 (flee = seek 반대 방향)
            let velocity = seek(ball_pos, *player_pos, PRESS_ADJUST_SPEED);
            player_pos.0 += velocity.0 * TICK_DT;
            player_pos.1 += velocity.1 * TICK_DT;
        }

        // === 태클 시도 조건 체크 ===
        if dist <= TACKLE_ATTEMPT_DISTANCE && tackle_cooldown == 0 && can_tackle_fn() {
            let tackle_id = schedule_tackle_fn();
            self.phase = PresserPhase::Tackling { tackle_action_id: tackle_id };
            return Some(PresserEvent::StartedTackle { action_id: tackle_id });
        }

        // === 압박 시간 초과 체크 ===
        if new_hold_ticks >= MAX_PRESS_TICKS {
            self.phase = PresserPhase::Recovering { remaining_ticks: PRESS_RECOVERY_TICKS };
            return Some(PresserEvent::PressTimeout);
        }

        // Update phase with new hold_ticks
        self.phase = PresserPhase::Pressing { hold_distance, hold_ticks: new_hold_ticks };

        None
    }

    /// Recovering Phase
    fn tick_recovering(&mut self, remaining_ticks: u8) -> Option<PresserEvent> {
        if remaining_ticks > 1 {
            self.phase = PresserPhase::Recovering { remaining_ticks: remaining_ticks - 1 };
            return None;
        }

        // 회복 완료 → 다시 Closing으로
        self.phase = PresserPhase::Closing { target_distance: PRESS_TARGET_DISTANCE };
        Some(PresserEvent::RecoveryComplete)
    }

    /// FIX_2601/0106 P2: Resting Phase
    ///
    /// 스태미나 부족 시 휴식. 걷기 속도로 팀 슬라이드 위치로 이동하며 회복.
    /// 스태미나가 STAMINA_RESUME_THRESHOLD 이상 회복되거나 시간이 지나면 복귀.
    fn tick_resting(
        &mut self,
        player_pos: &mut (f32, f32),
        ball_pos: (f32, f32),
        rest_target_pos: Option<(f32, f32)>,
        remaining_ticks: u16,
        return_to_closing: bool,
        stamina: f32,
    ) -> Option<PresserEvent> {
        // 공 근접 체크 - 공이 가까우면 즉시 압박 복귀
        let dist_to_ball = distance(*player_pos, ball_pos);
        let ball_is_near = dist_to_ball < BALL_PROXIMITY_THRESHOLD;

        // 스태미나가 충분히 회복되었거나 공이 가까우면 조기 복귀
        if ball_is_near || stamina >= STAMINA_RESUME_THRESHOLD {
            if return_to_closing {
                self.phase = PresserPhase::Closing { target_distance: PRESS_TARGET_DISTANCE };
            } else {
                self.phase =
                    PresserPhase::Pressing { hold_distance: PRESS_TARGET_DISTANCE, hold_ticks: 0 };
            }
            return Some(PresserEvent::RestingComplete);
        }

        // 시간 체크
        if remaining_ticks > 1 {
            // 휴식 목표 위치가 있으면 천천히 이동 (P3a: steering arrive 사용)
            if let Some(target) = rest_target_pos {
                let dist = distance(*player_pos, target);
                if dist > 0.5 {
                    let velocity = arrive(*player_pos, target, RESTING_WALK_SPEED, 2.0);
                    player_pos.0 += velocity.0 * TICK_DT;
                    player_pos.1 += velocity.1 * TICK_DT;
                }
            }

            self.phase =
                PresserPhase::Resting { remaining_ticks: remaining_ticks - 1, return_to_closing };
            return None;
        }

        // 휴식 완료
        if return_to_closing {
            self.phase = PresserPhase::Closing { target_distance: PRESS_TARGET_DISTANCE };
        } else {
            self.phase =
                PresserPhase::Pressing { hold_distance: PRESS_TARGET_DISTANCE, hold_ticks: 0 };
        }
        Some(PresserEvent::RestingComplete)
    }

    /// 태클 액션 완료 시 호출
    pub fn on_tackle_complete(&mut self, success: bool) {
        self.phase = PresserPhase::Recovering {
            remaining_ticks: if success {
                PRESS_RECOVERY_TICKS / 2 // 성공하면 짧은 회복
            } else {
                PRESS_RECOVERY_TICKS
            },
        };
    }
}

// ============================================================================
// FIX_2601/0106 P3: MarkerMovement FSM
// ============================================================================

/// 마커 움직임 관리
///
/// FIX_2601/0106 P3: 마킹 상태 세분화로 현실적인 맨마킹 구현
#[derive(Debug, Clone)]
pub struct MarkerMovement {
    pub player_idx: usize,
    pub phase: MarkerPhase,
    pub target_idx: usize, // 마킹 대상
}

impl MarkerMovement {
    /// 새 MarkerMovement 생성
    pub fn new(player_idx: usize, target_idx: usize) -> Self {
        Self {
            player_idx,
            phase: MarkerPhase::Following { offset_distance: MARKING_OFFSET },
            target_idx,
        }
    }

    /// 매 틱 업데이트
    ///
    /// # Arguments
    /// * `marker_pos` - 마커 위치 (미터)
    /// * `target_pos` - 마킹 대상 위치 (미터)
    /// * `ball_pos` - 공 위치 (미터)
    /// * `ball_velocity` - 공 속도 (m/s)
    /// * `own_goal_pos` - 자기 골문 위치 (미터)
    /// * `stamina` - 스태미나 (0-1)
    pub fn update_tick(
        &mut self,
        marker_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        ball_velocity: (f32, f32),
        own_goal_pos: (f32, f32),
        stamina: f32,
    ) -> Option<MarkerEvent> {
        // 공과의 거리 계산
        let dist_to_ball = distance(*marker_pos, ball_pos);
        let ball_is_near = dist_to_ball < BALL_PROXIMITY_THRESHOLD;

        // 스태미나 체크 (공이 멀 때만 휴식 진입)
        if !matches!(self.phase, MarkerPhase::Resting { .. })
            && stamina < STAMINA_REST_THRESHOLD
            && !ball_is_near
        {
            self.phase = MarkerPhase::Resting { remaining_ticks: RESTING_TICKS };
            return Some(MarkerEvent::StartedResting { stamina });
        }

        match self.phase {
            MarkerPhase::Following { offset_distance } => self.tick_following(
                marker_pos,
                target_pos,
                ball_pos,
                ball_velocity,
                own_goal_pos,
                offset_distance,
            ),
            MarkerPhase::Shadowing { intercept_distance } => {
                self.tick_shadowing(marker_pos, target_pos, ball_pos, intercept_distance)
            }
            MarkerPhase::Intercepting { intercept_point, arrival_ticks } => {
                self.tick_intercepting(marker_pos, ball_pos, intercept_point, arrival_ticks)
            }
            MarkerPhase::Resting { remaining_ticks } => {
                self.tick_resting(remaining_ticks, stamina, ball_is_near)
            }
        }
    }

    fn tick_following(
        &mut self,
        marker_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        ball_velocity: (f32, f32),
        own_goal_pos: (f32, f32),
        offset_distance: f32,
    ) -> Option<MarkerEvent> {
        // 공이 대상 방향으로 날아오면 인터셉트 전환
        let ball_speed = (ball_velocity.0.powi(2) + ball_velocity.1.powi(2)).sqrt();
        if ball_speed > 5.0 {
            let ball_to_target = (target_pos.0 - ball_pos.0, target_pos.1 - ball_pos.1);
            let dot = ball_velocity.0 * ball_to_target.0 + ball_velocity.1 * ball_to_target.1;
            if dot > 0.0 {
                let ball_to_target_dist =
                    (ball_to_target.0.powi(2) + ball_to_target.1.powi(2)).sqrt();
                if ball_to_target_dist < 30.0 {
                    // 인터셉트 가능 여부 체크
                    let intercept_point =
                        (ball_pos.0 + ball_velocity.0 * 0.5, ball_pos.1 + ball_velocity.1 * 0.5);
                    self.phase = MarkerPhase::Intercepting { intercept_point, arrival_ticks: 8 };
                    return Some(MarkerEvent::StartedIntercept { intercept_point });
                }
            }
        }

        // 공이 가까우면 Shadowing 전환
        let ball_dist = distance(*marker_pos, ball_pos);
        if ball_dist < 15.0 {
            self.phase = MarkerPhase::Shadowing { intercept_distance: ball_dist };
            return Some(MarkerEvent::StartedShadowing);
        }

        // Following: 대상과 골문 사이에 위치
        let target_to_goal = (own_goal_pos.0 - target_pos.0, own_goal_pos.1 - target_pos.1);
        let dist_to_goal = (target_to_goal.0.powi(2) + target_to_goal.1.powi(2)).sqrt();
        let dir = if dist_to_goal > 0.001 {
            (target_to_goal.0 / dist_to_goal, target_to_goal.1 / dist_to_goal)
        } else {
            (0.0, 0.0)
        };

        let ideal_pos =
            (target_pos.0 + dir.0 * offset_distance, target_pos.1 + dir.1 * offset_distance);

        // 이동 (P3a: steering arrive 사용)
        let dist = distance(*marker_pos, ideal_pos);
        if dist > 0.3 {
            let velocity = arrive(*marker_pos, ideal_pos, MARKER_MOVE_SPEED, 2.0);
            marker_pos.0 += velocity.0 * TICK_DT;
            marker_pos.1 += velocity.1 * TICK_DT;
        }

        None
    }

    fn tick_shadowing(
        &mut self,
        marker_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        _intercept_distance: f32,
    ) -> Option<MarkerEvent> {
        // 공이 멀어지면 Following으로 복귀
        let ball_dist = distance(*marker_pos, ball_pos);
        if ball_dist > 20.0 {
            self.phase = MarkerPhase::Following { offset_distance: MARKING_OFFSET };
            return None;
        }

        // 공과 대상 사이에 위치
        let ball_to_target = (target_pos.0 - ball_pos.0, target_pos.1 - ball_pos.1);
        let dist = (ball_to_target.0.powi(2) + ball_to_target.1.powi(2)).sqrt();
        let shadow_pos = if dist > 0.001 {
            let t = 0.3; // 공 쪽 30% 지점
            (ball_pos.0 + ball_to_target.0 * t, ball_pos.1 + ball_to_target.1 * t)
        } else {
            *marker_pos
        };

        // 이동 (P3a: steering seek 사용 - 빠르게 따라감)
        let move_dist = distance(*marker_pos, shadow_pos);
        if move_dist > 0.3 {
            let velocity = seek(*marker_pos, shadow_pos, MARKER_MOVE_SPEED * 1.1);
            marker_pos.0 += velocity.0 * TICK_DT;
            marker_pos.1 += velocity.1 * TICK_DT;
        }

        self.phase = MarkerPhase::Shadowing { intercept_distance: ball_dist };
        None
    }

    fn tick_intercepting(
        &mut self,
        marker_pos: &mut (f32, f32),
        ball_pos: (f32, f32),
        intercept_point: (f32, f32),
        arrival_ticks: u16,
    ) -> Option<MarkerEvent> {
        // 공을 잡았는지 체크
        let ball_dist = distance(*marker_pos, ball_pos);
        if ball_dist < 1.0 {
            self.phase = MarkerPhase::Following { offset_distance: MARKING_OFFSET };
            return Some(MarkerEvent::InterceptSuccess);
        }

        // 시간 초과
        if arrival_ticks == 0 {
            self.phase = MarkerPhase::Following { offset_distance: MARKING_OFFSET };
            return Some(MarkerEvent::InterceptFailed);
        }

        // 인터셉트 지점으로 이동 (P3a: steering seek 사용 - 전력질주)
        let dist = distance(*marker_pos, intercept_point);
        if dist > 0.3 {
            let velocity = seek(*marker_pos, intercept_point, MARKER_MOVE_SPEED * 1.3);
            marker_pos.0 += velocity.0 * TICK_DT;
            marker_pos.1 += velocity.1 * TICK_DT;
        }

        self.phase = MarkerPhase::Intercepting {
            intercept_point,
            arrival_ticks: arrival_ticks.saturating_sub(1),
        };
        None
    }

    fn tick_resting(
        &mut self,
        remaining_ticks: u16,
        stamina: f32,
        ball_is_near: bool,
    ) -> Option<MarkerEvent> {
        // 공이 가까우면 즉시 휴식 종료 (능력치 저하 상태로 행동)
        if ball_is_near || stamina >= STAMINA_RESUME_THRESHOLD || remaining_ticks <= 1 {
            self.phase = MarkerPhase::Following { offset_distance: MARKING_OFFSET };
            return Some(MarkerEvent::RestingComplete);
        }

        self.phase = MarkerPhase::Resting { remaining_ticks: remaining_ticks - 1 };
        None
    }
}

// ============================================================================
// FIX_2601/0106 P3: CoverMovement FSM
// ============================================================================

/// 커버 움직임 관리
///
/// FIX_2601/0106 P3: 커버 상태 세분화로 라인 유지 및 공간 커버 구현
#[derive(Debug, Clone)]
pub struct CoverMovement {
    pub player_idx: usize,
    pub phase: CoverPhase,
}

impl CoverMovement {
    /// 새 CoverMovement 생성
    pub fn new(player_idx: usize, initial_pos: (f32, f32)) -> Self {
        Self { player_idx, phase: CoverPhase::Holding { target_pos: initial_pos } }
    }

    /// 매 틱 업데이트
    ///
    /// # Arguments
    /// * `cover_pos` - 커버 위치 (미터)
    /// * `target_pos` - 팀 슬라이드 적용된 목표 위치 (미터)
    /// * `ball_pos` - 공 위치 (미터)
    /// * `own_goal_pos` - 자기 골문 위치 (미터)
    /// * `stamina` - 스태미나 (0-1)
    pub fn update_tick(
        &mut self,
        cover_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        own_goal_pos: (f32, f32),
        stamina: f32,
    ) -> Option<CoverEvent> {
        // 공과의 거리 계산
        let dist_to_ball = distance(*cover_pos, ball_pos);
        let ball_is_near = dist_to_ball < BALL_PROXIMITY_THRESHOLD;

        // 스태미나 체크 (공이 멀 때만 휴식 진입)
        if !matches!(self.phase, CoverPhase::Resting { .. })
            && stamina < STAMINA_REST_THRESHOLD
            && !ball_is_near
        {
            self.phase = CoverPhase::Resting { remaining_ticks: RESTING_TICKS };
            return Some(CoverEvent::StartedResting { stamina });
        }

        match self.phase {
            CoverPhase::Holding { target_pos: current_target } => {
                self.tick_holding(cover_pos, target_pos, ball_pos, own_goal_pos, current_target)
            }
            CoverPhase::Adjusting { target_pos: adj_target, speed_mult } => {
                self.tick_adjusting(cover_pos, target_pos, adj_target, speed_mult)
            }
            CoverPhase::SpaceCovering { cover_center, cover_radius } => self.tick_space_covering(
                cover_pos,
                target_pos,
                ball_pos,
                cover_center,
                cover_radius,
            ),
            CoverPhase::Recovering { fallback_pos } => {
                self.tick_recovering_cover(cover_pos, fallback_pos)
            }
            CoverPhase::Resting { remaining_ticks } => {
                self.tick_resting_cover(remaining_ticks, stamina, ball_is_near)
            }
        }
    }

    fn tick_holding(
        &mut self,
        cover_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        own_goal_pos: (f32, f32),
        current_target: (f32, f32),
    ) -> Option<CoverEvent> {
        // 공이 뒤로 돌파했는지 체크 (긴급 복귀 필요)
        let ball_behind = ball_pos.0 < cover_pos.0 - 10.0; // 공이 10m 이상 뒤에
        if ball_behind {
            let fallback = (own_goal_pos.0 + 5.0, cover_pos.1);
            self.phase = CoverPhase::Recovering { fallback_pos: fallback };
            return Some(CoverEvent::StartedRecovering { fallback_pos: fallback });
        }

        // 목표 위치가 크게 바뀌었으면 Adjusting으로
        let target_diff = distance(current_target, target_pos);
        if target_diff > 3.0 {
            self.phase = CoverPhase::Adjusting { target_pos, speed_mult: 1.0 };
            return Some(CoverEvent::StartedAdjusting);
        }

        // 공이 가까이 오면 공간 커버
        let ball_dist = distance(*cover_pos, ball_pos);
        if ball_dist < 20.0 {
            let cover_center =
                ((cover_pos.0 + own_goal_pos.0) / 2.0, (cover_pos.1 + ball_pos.1) / 2.0);
            self.phase = CoverPhase::SpaceCovering { cover_center, cover_radius: 8.0 };
            return Some(CoverEvent::StartedSpaceCover { cover_center });
        }

        // 라인 유지: 목표로 이동
        let dist = distance(*cover_pos, target_pos);
        if dist > 0.5 {
            let move_dir = normalize((target_pos.0 - cover_pos.0, target_pos.1 - cover_pos.1));
            let speed = COVER_MOVE_SPEED * TICK_DT;
            cover_pos.0 += move_dir.0 * speed;
            cover_pos.1 += move_dir.1 * speed;
        }

        self.phase = CoverPhase::Holding { target_pos };
        None
    }

    fn tick_adjusting(
        &mut self,
        cover_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        adj_target: (f32, f32),
        speed_mult: f32,
    ) -> Option<CoverEvent> {
        // 목표 도달 체크
        let dist = distance(*cover_pos, adj_target);
        if dist < 1.0 {
            self.phase = CoverPhase::Holding { target_pos };
            return Some(CoverEvent::HoldingComplete);
        }

        // 이동
        let move_dir = normalize((adj_target.0 - cover_pos.0, adj_target.1 - cover_pos.1));
        let speed = COVER_MOVE_SPEED * speed_mult * TICK_DT;
        cover_pos.0 += move_dir.0 * speed;
        cover_pos.1 += move_dir.1 * speed;

        // 목표가 바뀌었으면 업데이트
        self.phase = CoverPhase::Adjusting { target_pos, speed_mult };
        None
    }

    fn tick_space_covering(
        &mut self,
        cover_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        cover_center: (f32, f32),
        cover_radius: f32,
    ) -> Option<CoverEvent> {
        // 공이 멀어지면 Holding으로
        let ball_dist = distance(*cover_pos, ball_pos);
        if ball_dist > 25.0 {
            self.phase = CoverPhase::Holding { target_pos };
            return Some(CoverEvent::HoldingComplete);
        }

        // 커버 중심으로 이동 (공간 차단)
        let dist = distance(*cover_pos, cover_center);
        if dist > cover_radius {
            let move_dir = normalize((cover_center.0 - cover_pos.0, cover_center.1 - cover_pos.1));
            let speed = COVER_MOVE_SPEED * 1.1 * TICK_DT;
            cover_pos.0 += move_dir.0 * speed;
            cover_pos.1 += move_dir.1 * speed;
        }

        // 커버 중심 업데이트 (공 위치 반영)
        let new_center = (cover_center.0, (cover_center.1 + ball_pos.1) / 2.0);
        self.phase = CoverPhase::SpaceCovering { cover_center: new_center, cover_radius };
        None
    }

    fn tick_recovering_cover(
        &mut self,
        cover_pos: &mut (f32, f32),
        fallback_pos: (f32, f32),
    ) -> Option<CoverEvent> {
        // 복귀 완료
        let dist = distance(*cover_pos, fallback_pos);
        if dist < 2.0 {
            self.phase = CoverPhase::Holding { target_pos: fallback_pos };
            return Some(CoverEvent::HoldingComplete);
        }

        // 전력질주로 복귀
        let move_dir = normalize((fallback_pos.0 - cover_pos.0, fallback_pos.1 - cover_pos.1));
        let speed = COVER_MOVE_SPEED * 1.4 * TICK_DT; // 빠르게
        cover_pos.0 += move_dir.0 * speed;
        cover_pos.1 += move_dir.1 * speed;

        None
    }

    fn tick_resting_cover(
        &mut self,
        remaining_ticks: u16,
        stamina: f32,
        ball_is_near: bool,
    ) -> Option<CoverEvent> {
        // 공이 가까우면 즉시 휴식 종료 (능력치 저하 상태로 행동)
        if ball_is_near || stamina >= STAMINA_RESUME_THRESHOLD || remaining_ticks <= 1 {
            self.phase = CoverPhase::Holding { target_pos: (0.0, 0.0) }; // Will be updated next tick
            return Some(CoverEvent::RestingComplete);
        }

        self.phase = CoverPhase::Resting { remaining_ticks: remaining_ticks - 1 };
        None
    }
}

// ============================================================================
// Role Assignment
// ============================================================================

/// 수비 역할 할당
pub fn assign_defensive_roles(
    ball_pos: (f32, f32),
    defending_team_positions: &[(f32, f32)],
    attacking_team_positions: &[(f32, f32)],
    player_states: &[PlayerState],
    defending_team_start_idx: usize,
) -> Vec<DefensiveRole> {
    let n = defending_team_positions.len();
    let mut roles = vec![DefensiveRole::Cover; n];

    if n == 0 {
        return roles;
    }

    // GK는 항상 Goalkeeper
    roles[0] = DefensiveRole::Goalkeeper;

    // 공에 가장 가까운 선수 중 Idle/Moving인 선수를 Primary Presser로
    let mut candidates: Vec<(usize, f32)> = defending_team_positions
        .iter()
        .enumerate()
        .skip(1) // GK 제외
        .filter(|(idx, _)| {
            let state_idx = defending_team_start_idx + *idx;
            if state_idx < player_states.len() {
                player_states[state_idx].can_start_action()
            } else {
                false
            }
        })
        .map(|(idx, pos)| (idx, distance(*pos, ball_pos)))
        .collect();

    candidates.sort_by(|a, b| {
        match a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal) {
            std::cmp::Ordering::Equal => {
                // FIX_2601/0116: Use position-based tie-breaker to avoid index bias
                let pos_a = defending_team_positions[a.0];
                let pos_b = defending_team_positions[b.0];
                crate::engine::match_sim::deterministic_tie_hash(a.0, pos_a, b.0, pos_b)
            }
            other => other,
        }
    });

    // Primary Presser
    if let Some((primary_idx, _)) = candidates.first() {
        roles[*primary_idx] = DefensiveRole::PresserPrimary;
    }

    // Secondary Presser (두 번째로 가까운 선수)
    if let Some((secondary_idx, _)) = candidates.get(1) {
        roles[*secondary_idx] = DefensiveRole::PresserSecondary;
    }

    // 나머지는 위험한 공격수 마킹 또는 커버
    for (idx, _) in candidates.iter().skip(2) {
        if let Some(attacker_idx) =
            find_nearest_attacker(defending_team_positions[*idx], attacking_team_positions)
        {
            roles[*idx] = DefensiveRole::Marker { target_idx: attacker_idx };
        }
    }

    roles
}

/// 가장 가까운 공격수 찾기
fn find_nearest_attacker(defender_pos: (f32, f32), attackers: &[(f32, f32)]) -> Option<usize> {
    attackers
        .iter()
        .enumerate()
        .map(|(idx, pos)| (idx, distance(defender_pos, *pos)))
        .filter(|(_, dist)| *dist < MARKING_RANGE)
        .min_by(|a, b| {
            match a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal) {
                std::cmp::Ordering::Equal => {
                    // FIX_2601/0116: Use position-based tie-breaker to avoid index bias
                    let pos_a = attackers[a.0];
                    let pos_b = attackers[b.0];
                    crate::engine::match_sim::deterministic_tie_hash(a.0, pos_a, b.0, pos_b)
                }
                other => other,
            }
        })
        .map(|(idx, _)| idx)
}

// ============================================================================
// Team Slide
// ============================================================================

/// 공 위치 기준 팀 전체 슬라이드 벡터
/// FIX_2601/0110: Added attacks_right parameter for correct second-half positioning
pub fn calculate_team_slide(
    ball_pos: (f32, f32),
    team_side: TeamSide,
    defensive_line: DefensiveLine,
    attacks_right: bool,
) -> (f32, f32) {
    let field_center = (field::CENTER_X, field::CENTER_Y);

    // === 가로 슬라이드 (좌우) ===
    let dx = (ball_pos.0 - field_center.0) / field::CENTER_X; // -1 ~ +1
    let side_shift = dx * SIDE_SHIFT_MAX;

    // === 세로 슬라이드 (전후) ===
    // FIX_2601/0110: Use attacks_right to determine own goal position
    // Defending team's goal is opposite to attack direction
    let own_goal_x = if attacks_right {
        0.0 // Attacking right means defending left (x=0)
    } else {
        field::LENGTH_M // Attacking left means defending right (x=field length)
    };
    let _ = team_side; // Silence unused warning, kept for API compatibility
    let depth_ratio = 1.0 - ((ball_pos.0 - own_goal_x).abs() / field::LENGTH_M);

    let base_depth = match defensive_line {
        DefensiveLine::VeryHigh => -8.0,
        DefensiveLine::High => -4.0,
        DefensiveLine::Normal => 0.0,
        DefensiveLine::Deep => 5.0,
        DefensiveLine::VeryDeep => 10.0,
    };

    let depth_shift = base_depth * depth_ratio;

    (side_shift, depth_shift)
}

/// 팀 슬라이드를 각 선수의 기본 포지션에 적용
pub fn apply_team_slide(
    base_positions: &[(f32, f32)],
    team_slide: (f32, f32),
    player_states: &[PlayerState],
    roles: &[DefensiveRole],
) -> Vec<(f32, f32)> {
    base_positions
        .iter()
        .enumerate()
        .map(|(idx, base_pos)| {
            // Presser는 슬라이드 적용 안 함 (독자적으로 움직임)
            if matches!(
                roles.get(idx),
                Some(DefensiveRole::PresserPrimary | DefensiveRole::PresserSecondary)
            ) {
                return *base_pos;
            }

            // Recovering/Staggered 상태면 이동 못함
            if let Some(state) = player_states.get(idx) {
                if !state.can_move() {
                    return *base_pos;
                }
            }

            // 슬라이드 적용
            (base_pos.0 + team_slide.0, base_pos.1 + team_slide.1)
        })
        .collect()
}

// ============================================================================
// Movement Functions
// ============================================================================

/// Cover 역할 선수의 움직임 (P3a: steering arrive 사용)
pub fn update_cover_movement(
    player_pos: &mut (f32, f32),
    player_state: &PlayerState,
    target_pos: (f32, f32),
) {
    if !player_state.can_move() {
        return;
    }

    let dist = distance(*player_pos, target_pos);

    if dist > 0.5 {
        // P3a: 도착 시 자연스럽게 감속하는 arrive behavior
        let velocity = arrive(*player_pos, target_pos, COVER_MOVE_SPEED, 3.0);
        player_pos.0 += velocity.0 * movement_constants::TICK_DT;
        player_pos.1 += velocity.1 * movement_constants::TICK_DT;
    }
}

/// Cover 역할 선수의 움직임 with separation (P3a)
///
/// 다른 Cover 선수들과의 separation을 적용하여 클러스터링 방지
pub fn update_cover_movement_with_separation(
    player_pos: &mut (f32, f32),
    player_state: &PlayerState,
    target_pos: (f32, f32),
    other_cover_positions: &[(f32, f32)],
) {
    if !player_state.can_move() {
        return;
    }

    let dist = distance(*player_pos, target_pos);

    if dist > 0.5 {
        // P3a: arrive + separation
        let arrive_vel = arrive(*player_pos, target_pos, COVER_MOVE_SPEED, 3.0);
        let sep_vel = separation(*player_pos, other_cover_positions, 5.0, 2.0);

        // Combine: arrive dominates (70%), separation assists (30%)
        let combined = (arrive_vel.0 + sep_vel.0 * 0.3, arrive_vel.1 + sep_vel.1 * 0.3);

        player_pos.0 += combined.0 * movement_constants::TICK_DT;
        player_pos.1 += combined.1 * movement_constants::TICK_DT;
    }
}

/// Marker 역할 선수의 움직임
pub fn update_marker_movement(
    marker_pos: &mut (f32, f32),
    marker_state: &PlayerState,
    target_attacker_pos: (f32, f32),
    own_goal_pos: (f32, f32),
    ball_pos: (f32, f32),
) {
    // 공 속도가 없는 기본 마킹
    update_marker_movement_with_ball_velocity(
        marker_pos,
        marker_state,
        target_attacker_pos,
        own_goal_pos,
        ball_pos,
        (0.0, 0.0), // no ball velocity
        DEFAULT_PLAYER_SPEED,
    );
}

/// Marker 역할 선수의 움직임 (공 속도 포함, 인터셉트 예측 활성화)
///
/// FIX_2601/0106 P1: 인터셉트 예측 기능 통합
///
/// # Arguments
/// * `marker_pos` - 마커 위치 (미터)
/// * `marker_state` - 마커 상태
/// * `target_attacker_pos` - 마킹 대상 위치 (미터)
/// * `own_goal_pos` - 자기 골문 위치 (미터)
/// * `ball_pos` - 공 위치 (미터)
/// * `ball_velocity` - 공 속도 (m/s)
/// * `target_attacker_speed` - 마킹 대상 속도 (m/s)
pub fn update_marker_movement_with_ball_velocity(
    marker_pos: &mut (f32, f32),
    marker_state: &PlayerState,
    target_attacker_pos: (f32, f32),
    own_goal_pos: (f32, f32),
    ball_pos: (f32, f32),
    ball_velocity: (f32, f32),
    target_attacker_speed: f32,
) {
    if !marker_state.can_move() {
        return;
    }

    // FIX_2601/0106 P1: 인터셉트 예측
    let prediction = should_marker_intercept(
        *marker_pos,
        MARKER_MOVE_SPEED,
        ball_pos,
        ball_velocity,
        target_attacker_pos,
        target_attacker_speed,
    );

    // 인터셉트 가능하면 인터셉트 지점으로 이동 (P3a: steering seek 사용)
    if prediction.can_intercept {
        let dist = distance(*marker_pos, prediction.intercept_point);
        if dist > 0.3 {
            let velocity = seek(*marker_pos, prediction.intercept_point, MARKER_MOVE_SPEED + 1.0);
            marker_pos.0 += velocity.0 * movement_constants::TICK_DT;
            marker_pos.1 += velocity.1 * movement_constants::TICK_DT;
        }
        return;
    }

    // 기본 마킹: 공격수와 골 사이
    let goal_to_attacker =
        normalize((target_attacker_pos.0 - own_goal_pos.0, target_attacker_pos.1 - own_goal_pos.1));

    // 공이 가까우면 더 타이트하게
    let ball_dist = distance(target_attacker_pos, ball_pos);
    let tightness = if ball_dist < 10.0 { 0.5 } else { 1.0 };

    let final_target = (
        target_attacker_pos.0 - goal_to_attacker.0 * MARKING_OFFSET * tightness,
        target_attacker_pos.1 - goal_to_attacker.1 * MARKING_OFFSET * tightness,
    );

    let dist = distance(*marker_pos, final_target);
    if dist > 0.3 {
        // P3a: steering arrive 사용
        let velocity = arrive(*marker_pos, final_target, MARKER_MOVE_SPEED, 2.0);
        marker_pos.0 += velocity.0 * movement_constants::TICK_DT;
        marker_pos.1 += velocity.1 * movement_constants::TICK_DT;
    }
}

/// Marker 역할 선수의 움직임 with pursuit (P3a)
///
/// 공격수의 속도를 예측하여 마킹
pub fn update_marker_movement_with_pursuit(
    marker_pos: &mut (f32, f32),
    marker_state: &PlayerState,
    target_attacker_pos: (f32, f32),
    target_attacker_velocity: (f32, f32),
    own_goal_pos: (f32, f32),
    ball_pos: (f32, f32),
) {
    if !marker_state.can_move() {
        return;
    }

    // 기본 마킹 위치 계산: 공격수와 골 사이
    let goal_to_attacker =
        normalize((target_attacker_pos.0 - own_goal_pos.0, target_attacker_pos.1 - own_goal_pos.1));

    // 공이 가까우면 더 타이트하게
    let ball_dist = distance(target_attacker_pos, ball_pos);
    let tightness = if ball_dist < 10.0 { 0.5 } else { 1.0 };

    let marking_offset = (
        -goal_to_attacker.0 * MARKING_OFFSET * tightness,
        -goal_to_attacker.1 * MARKING_OFFSET * tightness,
    );

    // P3a: pursuit를 사용하여 공격수의 이동을 예측
    // 마킹 목표 위치도 공격수와 함께 이동하므로 동일한 속도 사용
    let predicted_marking_pos =
        (target_attacker_pos.0 + marking_offset.0, target_attacker_pos.1 + marking_offset.1);

    let dist = distance(*marker_pos, predicted_marking_pos);
    if dist > 0.3 {
        // pursuit로 공격수 예측 + arrive로 도착 시 감속
        let velocity = pursuit(
            *marker_pos,
            predicted_marking_pos,
            target_attacker_velocity,
            MARKER_MOVE_SPEED,
            0.5, // 0.5초 미래 예측
        );
        marker_pos.0 += velocity.0 * movement_constants::TICK_DT;
        marker_pos.1 += velocity.1 * movement_constants::TICK_DT;
    }
}

/// 골키퍼 포지셔닝
pub fn update_goalkeeper_position(
    gk_pos: &mut (f32, f32),
    gk_state: &PlayerState,
    ball_pos: (f32, f32),
    own_goal_pos: (f32, f32),
    goal_width: f32,
) {
    if !gk_state.can_move() {
        return;
    }

    let goal_to_ball = (ball_pos.0 - own_goal_pos.0, ball_pos.1 - own_goal_pos.1);
    let dist_to_ball = (goal_to_ball.0.powi(2) + goal_to_ball.1.powi(2)).sqrt();

    // 공과의 거리에 따라 골라인에서 얼마나 나올지 결정
    let advance_ratio = (dist_to_ball / 50.0).min(1.0);
    let advance_distance = GK_MIN_ADVANCE + (GK_MAX_ADVANCE - GK_MIN_ADVANCE) * advance_ratio;

    // 목표 위치
    let dir = normalize(goal_to_ball);
    let target_x = own_goal_pos.0 + dir.0 * advance_distance;

    // Y 위치: 골대 폭 내에서 공의 Y에 맞춤
    let goal_y_min = own_goal_pos.1 - goal_width / 2.0;
    let goal_y_max = own_goal_pos.1 + goal_width / 2.0;
    let target_y = ball_pos.1.clamp(goal_y_min + 1.0, goal_y_max - 1.0);

    // 이동 (P3a: steering arrive 사용)
    let target = (target_x, target_y);
    let dist = distance(*gk_pos, target);
    if dist > 0.2 {
        let velocity = arrive(*gk_pos, target, GK_MOVE_SPEED, 2.0);
        gk_pos.0 += velocity.0 * gk_constants::TICK_DT;
        gk_pos.1 += velocity.1 * gk_constants::TICK_DT;
    }
}

// ============================================================================
// Presser Swap
// ============================================================================

/// 압박자 교대가 필요한지 체크
pub fn should_swap_presser(
    _current_presser_idx: usize,
    current_presser_pos: (f32, f32),
    current_presser_state: &PlayerState,
    current_presser_cooldown: u8,
    ball_pos: (f32, f32),
    _secondary_presser_idx: usize,
    secondary_presser_pos: (f32, f32),
    secondary_presser_state: &PlayerState,
) -> bool {
    // 현재 Presser가 Recovering이면 교대
    if matches!(current_presser_state, PlayerState::Recovering { .. }) {
        return true;
    }

    if current_presser_cooldown > 0 {
        return true;
    }

    // Secondary가 이동 가능해야 함
    if !secondary_presser_state.can_start_action() {
        return false;
    }

    // Secondary가 3m 이상 가까우면 교대
    let current_dist = distance(current_presser_pos, ball_pos);
    let secondary_dist = distance(secondary_presser_pos, ball_pos);

    secondary_dist + 3.0 < current_dist
}

/// 압박자 역할 교대 실행
pub fn swap_presser_roles(roles: &mut [DefensiveRole], primary_idx: usize, secondary_idx: usize) {
    if primary_idx < roles.len() && secondary_idx < roles.len() {
        roles[primary_idx] = DefensiveRole::PresserSecondary;
        roles[secondary_idx] = DefensiveRole::PresserPrimary;
    }
}

// ============================================================================
// Interception Prediction (FIX_2601/0106 P1)
// ============================================================================

/// 인터셉트 예측 상수
pub mod interception_constants {
    /// 기본 선수 속도 (m/s)
    pub const DEFAULT_PLAYER_SPEED: f32 = 6.0;

    /// 반응 지연 (초) - 선수가 인터셉트를 시작하기까지의 지연
    pub const REACTION_DELAY: f32 = 0.2;

    /// 최소 인터셉트 성공 마진 (초) - 이 시간 이상 빨리 도착해야 성공
    pub const MIN_INTERCEPT_MARGIN: f32 = 0.3;

    /// 최대 인터셉트 거리 (m) - 이 거리 이상은 인터셉트 시도 안함
    pub const MAX_INTERCEPT_DISTANCE: f32 = 25.0;

    /// 최대 예측 시간 (초) - 이 시간 이상의 예측은 신뢰도 낮음
    pub const MAX_PREDICTION_TIME: f32 = 3.0;
}

pub use interception_constants::{
    DEFAULT_PLAYER_SPEED, MAX_INTERCEPT_DISTANCE, MAX_PREDICTION_TIME, MIN_INTERCEPT_MARGIN,
    REACTION_DELAY,
};

/// 인터셉트 예측 결과
#[derive(Debug, Clone, Copy)]
pub struct InterceptionPrediction {
    /// 예측된 인터셉트 지점 (미터)
    pub intercept_point: (f32, f32),

    /// 인터셉트 지점까지 걸리는 시간 (초)
    pub time_to_intercept: f32,

    /// 인터셉트 가능 여부
    pub can_intercept: bool,

    /// 가장 가까운 상대보다 빠른 시간 마진 (양수면 성공, 음수면 실패)
    pub time_margin: f32,
}

/// 공이 특정 시간 후에 있을 위치 계산
///
/// # Arguments
/// * `ball_pos` - 현재 공 위치 (미터)
/// * `ball_velocity` - 공 속도 (m/s)
/// * `time_seconds` - 예측할 시간 (초)
///
/// # Returns
/// 예측된 공 위치 (미터)
pub fn predict_ball_position(
    ball_pos: (f32, f32),
    ball_velocity: (f32, f32),
    time_seconds: f32,
) -> (f32, f32) {
    // 단순 선형 예측 (감속 없음)
    // TODO: 공의 감속 모델 추가 고려
    (ball_pos.0 + ball_velocity.0 * time_seconds, ball_pos.1 + ball_velocity.1 * time_seconds)
}

/// 선수가 특정 지점까지 도달하는 시간 추정
///
/// # Arguments
/// * `player_pos` - 선수 위치 (미터)
/// * `target_pos` - 목표 지점 (미터)
/// * `player_speed` - 선수 속도 (m/s)
///
/// # Returns
/// 도달 시간 (초), 반응 지연 포함
pub fn estimate_time_to_point(
    player_pos: (f32, f32),
    target_pos: (f32, f32),
    player_speed: f32,
) -> f32 {
    let dist = distance(player_pos, target_pos);
    if player_speed <= 0.0 {
        return f32::MAX;
    }
    REACTION_DELAY + dist / player_speed
}

/// 인터셉트 지점 계산 (공과 수비수의 만남점)
///
/// 공의 이동 경로와 수비수가 도달 가능한 지점을 반복적으로 계산하여
/// 최적의 인터셉트 지점을 찾음.
///
/// # Arguments
/// * `ball_pos` - 공 위치 (미터)
/// * `ball_velocity` - 공 속도 (m/s)
/// * `defender_pos` - 수비수 위치 (미터)
/// * `defender_speed` - 수비수 속도 (m/s)
///
/// # Returns
/// 예측된 인터셉트 지점 (미터)과 소요 시간 (초)
pub fn calculate_interception_point(
    ball_pos: (f32, f32),
    ball_velocity: (f32, f32),
    defender_pos: (f32, f32),
    defender_speed: f32,
) -> ((f32, f32), f32) {
    let ball_speed = (ball_velocity.0.powi(2) + ball_velocity.1.powi(2)).sqrt();

    // 공이 정지해 있으면 공 위치가 인터셉트 지점
    if ball_speed < 0.1 {
        let time = estimate_time_to_point(defender_pos, ball_pos, defender_speed);
        return (ball_pos, time);
    }

    // 반복적 근사법: 시간 t에서 공 위치를 예측하고,
    // 수비수가 그 위치에 도달하는 시간을 계산
    let mut t = 0.5; // 초기 예측 시간
    let dt = 0.1; // 시간 증분

    for _ in 0..30 {
        // 최대 30회 반복
        let predicted_ball = predict_ball_position(ball_pos, ball_velocity, t);
        let time_to_reach = estimate_time_to_point(defender_pos, predicted_ball, defender_speed);

        // 수비수가 예측 시간보다 빨리 도착하면 더 가까운 지점 탐색
        if time_to_reach < t {
            t -= dt;
            if t <= 0.0 {
                break;
            }
        }
        // 수비수가 더 오래 걸리면 더 먼 지점 탐색
        else if time_to_reach > t + dt {
            t += dt;
            if t > MAX_PREDICTION_TIME {
                break;
            }
        }
        // 수렴했으면 종료
        else {
            break;
        }
    }

    let intercept_point = predict_ball_position(ball_pos, ball_velocity, t);
    let final_time = estimate_time_to_point(defender_pos, intercept_point, defender_speed);

    (intercept_point, final_time)
}

/// 수비수가 상대보다 먼저 인터셉트할 수 있는지 확인
///
/// Open-Football 방식 참조: 모든 상대의 도달 시간과 비교하여
/// 수비수가 먼저 도착할 수 있는지 판단
///
/// # Arguments
/// * `defender_pos` - 수비수 위치 (미터)
/// * `defender_speed` - 수비수 속도 (m/s)
/// * `ball_pos` - 공 위치 (미터)
/// * `ball_velocity` - 공 속도 (m/s)
/// * `opponents` - 상대 선수들의 (위치, 속도) 목록
///
/// # Returns
/// 인터셉트 예측 결과
pub fn can_intercept_before_opponent(
    defender_pos: (f32, f32),
    defender_speed: f32,
    ball_pos: (f32, f32),
    ball_velocity: (f32, f32),
    opponents: &[((f32, f32), f32)], // (position, speed)
) -> InterceptionPrediction {
    // 1. 인터셉트 지점 계산
    let (intercept_point, defender_time) =
        calculate_interception_point(ball_pos, ball_velocity, defender_pos, defender_speed);

    // 2. 거리 체크 - 너무 멀면 포기
    let intercept_dist = distance(defender_pos, intercept_point);
    if intercept_dist > MAX_INTERCEPT_DISTANCE {
        return InterceptionPrediction {
            intercept_point,
            time_to_intercept: defender_time,
            can_intercept: false,
            time_margin: f32::NEG_INFINITY,
        };
    }

    // 3. 각 상대의 인터셉트 지점 도달 시간 계산
    let mut min_opponent_time = f32::MAX;
    for (opp_pos, opp_speed) in opponents {
        let opp_time = estimate_time_to_point(*opp_pos, intercept_point, *opp_speed);
        if opp_time < min_opponent_time {
            min_opponent_time = opp_time;
        }
    }

    // 4. 마진 계산 (수비수가 얼마나 먼저 도착하는지)
    let time_margin = min_opponent_time - defender_time;
    let can_intercept = time_margin >= MIN_INTERCEPT_MARGIN;

    InterceptionPrediction {
        intercept_point,
        time_to_intercept: defender_time,
        can_intercept,
        time_margin,
    }
}

/// Marker 역할에서 인터셉트 여부 판단
///
/// 마킹 중인 선수가 패스 경로를 인터셉트할 수 있는지 확인
///
/// # Arguments
/// * `marker_pos` - 마커 위치 (미터)
/// * `marker_speed` - 마커 속도 (m/s)
/// * `ball_pos` - 공 위치 (미터)
/// * `ball_velocity` - 공 속도 (m/s)
/// * `marked_player_pos` - 마킹 대상 위치 (미터)
/// * `marked_player_speed` - 마킹 대상 속도 (m/s)
///
/// # Returns
/// 인터셉트 시도 여부 및 예측 정보
pub fn should_marker_intercept(
    marker_pos: (f32, f32),
    marker_speed: f32,
    ball_pos: (f32, f32),
    ball_velocity: (f32, f32),
    marked_player_pos: (f32, f32),
    marked_player_speed: f32,
) -> InterceptionPrediction {
    // 공이 움직이지 않으면 인터셉트 불가
    let ball_speed = (ball_velocity.0.powi(2) + ball_velocity.1.powi(2)).sqrt();
    if ball_speed < 0.5 {
        return InterceptionPrediction {
            intercept_point: ball_pos,
            time_to_intercept: f32::MAX,
            can_intercept: false,
            time_margin: f32::NEG_INFINITY,
        };
    }

    // 공이 마킹 대상 쪽으로 가고 있는지 확인
    let ball_to_marked = (marked_player_pos.0 - ball_pos.0, marked_player_pos.1 - ball_pos.1);
    let dot = ball_velocity.0 * ball_to_marked.0 + ball_velocity.1 * ball_to_marked.1;

    // 공이 마킹 대상 반대 방향으로 가면 인터셉트 불필요
    if dot < 0.0 {
        return InterceptionPrediction {
            intercept_point: ball_pos,
            time_to_intercept: f32::MAX,
            can_intercept: false,
            time_margin: f32::NEG_INFINITY,
        };
    }

    // 마킹 대상만 상대로 인터셉트 가능 여부 판단
    can_intercept_before_opponent(
        marker_pos,
        marker_speed,
        ball_pos,
        ball_velocity,
        &[(marked_player_pos, marked_player_speed)],
    )
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 벡터 정규화
#[inline]
fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0.powi(2) + v.1.powi(2)).sqrt();
    if len > 0.001 {
        (v.0 / len, v.1 / len)
    } else {
        (0.0, 0.0)
    }
}

// ============================================================================
// Full Update Function
// ============================================================================

/// 수비 포지셔닝 전체 업데이트
/// FIX_2601/0106 P3: 통합 수비 포지셔닝 업데이트
/// FIX_2601/0110: Added attacks_right for correct second-half positioning
///
/// Presser, Marker, Cover 모든 FSM을 업데이트합니다.
#[allow(clippy::too_many_arguments)]
pub fn update_defensive_positioning<F, G>(
    positions: &mut [(f32, f32)],
    player_states: &[PlayerState],
    tackle_cooldowns: &[u8],
    roles: &mut [DefensiveRole],
    presser_movements: &mut Vec<PresserMovement>,
    marker_movements: &mut Vec<MarkerMovement>, // FIX_2601/0106 P3
    cover_movements: &mut Vec<CoverMovement>,   // FIX_2601/0106 P3
    ball_pos: (f32, f32),
    ball_velocity: (f32, f32), // FIX_2601/0106 P3: 공 속도
    ball_owner_idx: Option<usize>,
    shape_params: TeamShapeParams,
    base_formation_positions: &[(f32, f32)],
    team_side: TeamSide,
    defensive_line: DefensiveLine,
    own_goal_pos: (f32, f32),
    attacking_team_positions: &[(f32, f32)],
    defending_team_start_idx: usize,
    stamina: &[f32], // FIX_2601/0106 P3: 스태미나 배열
    attacks_right: bool, // FIX_2601/0110: defending team's attack direction
    schedule_tackle_fn: F,
    can_tackle_fn: G,
) -> Vec<PresserEvent>
where
    F: Fn(usize) -> u64,
    G: Fn(usize) -> bool,
{
    let mut events = Vec::new();

    // 1. 팀 슬라이드 계산
    let press_mult = 0.85 + shape_params.pressing_bias.clamp(0.0, 1.0) * 0.30;
    let team_slide = calculate_team_slide(ball_pos, team_side, defensive_line, attacks_right);
    let team_slide = (team_slide.0 * press_mult, team_slide.1 * press_mult);

    // 2. 슬라이드 적용된 목표 위치
    // NOTE: apply_team_slide() expects team-local indexing (0..roles.len()).
    // update_defensive_positioning() receives global player_states (0..22),
    // so we must slice to the defending team to avoid away-team indexing bugs.
    let team_player_states: &[PlayerState] = player_states
        .get(defending_team_start_idx..defending_team_start_idx + roles.len())
        .unwrap_or(player_states);
    let mut shaped_base_positions: [(f32, f32); 11] = [(0.0, 0.0); 11];
    let base_positions_for_slide: &[(f32, f32)] =
        if base_formation_positions.len() == roles.len() && base_formation_positions.len() <= 11 {
            let own_goal_x = if attacks_right { 0.0 } else { field::LENGTH_M };
            let forward_sign = if attacks_right { 1.0 } else { -1.0 };

            let width_scale = 0.75 + shape_params.width.clamp(0.0, 1.0) * 0.50; // 0.75..1.25
            let compact_scale = 1.15 - shape_params.compactness.clamp(0.0, 1.0) * 0.30; // 1.15..0.85
            let lateral_scale = (width_scale * compact_scale).clamp(0.55, 1.35);

            let depth_scale =
                (0.85 + shape_params.depth.clamp(0.0, 1.0) * 0.30).clamp(0.70, 1.15);
            let line_height_offset_m =
                (shape_params.line_height.clamp(0.0, 1.0) - 0.5) * 14.0; // -7..+7m

            for (idx, base_pos) in base_formation_positions.iter().enumerate() {
                let x_scaled = own_goal_x + (base_pos.0 - own_goal_x) * depth_scale;
                let x =
                    (x_scaled + forward_sign * line_height_offset_m).clamp(0.0, field::LENGTH_M);

                let y_scaled = field::CENTER_Y + (base_pos.1 - field::CENTER_Y) * lateral_scale;
                let y = y_scaled.clamp(0.0, field::WIDTH_M);

                shaped_base_positions[idx] = (x, y);
            }

            &shaped_base_positions[..roles.len()]
        } else {
            base_formation_positions
        };

    let target_positions =
        apply_team_slide(base_positions_for_slide, team_slide, team_player_states, roles);

    // 3. 역할별 업데이트
    for (local_idx, role) in roles.iter().enumerate() {
        let global_idx = defending_team_start_idx + local_idx;

        if global_idx >= positions.len() || global_idx >= player_states.len() {
            continue;
        }

        let player_state = &player_states[global_idx];

        match role {
            DefensiveRole::PresserPrimary | DefensiveRole::PresserSecondary => {
                // Presser FSM 찾기/생성
                let movement = presser_movements.iter_mut().find(|m| m.player_idx == global_idx);

                if let Some(movement) = movement {
                    let player_pos = &mut positions[global_idx];
                    let cooldown = tackle_cooldowns.get(global_idx).copied().unwrap_or(0);

                    if let Some(event) = movement.update_tick(
                        player_pos,
                        player_state,
                        ball_pos,
                        ball_owner_idx,
                        cooldown,
                        || can_tackle_fn(global_idx),
                        || schedule_tackle_fn(global_idx),
                    ) {
                        events.push(event);
                    }
                } else {
                    // 새 PresserMovement 생성
                    presser_movements.push(PresserMovement::new(global_idx, *role));
                }
            }

            DefensiveRole::Marker { target_idx } => {
                if *target_idx < attacking_team_positions.len() {
                    // FIX_2601/0106 P3: MarkerMovement FSM 사용
                    let player_stamina = stamina.get(global_idx).copied().unwrap_or(1.0);
                    let movement = marker_movements.iter_mut().find(|m| m.player_idx == global_idx);

                    if let Some(movement) = movement {
                        let _event = movement.update_tick(
                            &mut positions[global_idx],
                            attacking_team_positions[*target_idx],
                            ball_pos,
                            ball_velocity,
                            own_goal_pos,
                            player_stamina,
                        );
                        // MarkerEvent는 별도 처리 필요시 events에 추가
                    } else {
                        // 새 MarkerMovement 생성
                        marker_movements.push(MarkerMovement::new(global_idx, *target_idx));
                    }
                }
            }

            DefensiveRole::Cover => {
                if local_idx < target_positions.len() {
                    // FIX_2601/0106 P3: CoverMovement FSM 사용
                    let player_stamina = stamina.get(global_idx).copied().unwrap_or(1.0);
                    let movement = cover_movements.iter_mut().find(|m| m.player_idx == global_idx);

                    if let Some(movement) = movement {
                        let _event = movement.update_tick(
                            &mut positions[global_idx],
                            target_positions[local_idx],
                            ball_pos,
                            own_goal_pos,
                            player_stamina,
                        );
                        // CoverEvent는 별도 처리 필요시 events에 추가
                    } else {
                        // 새 CoverMovement 생성
                        cover_movements
                            .push(CoverMovement::new(global_idx, target_positions[local_idx]));
                    }
                }
            }

            DefensiveRole::Goalkeeper => {
                update_goalkeeper_position(
                    &mut positions[global_idx],
                    player_state,
                    ball_pos,
                    own_goal_pos,
                    7.32, // 골대 폭
                );
            }
        }
    }

    events
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_slide_left() {
        // 공이 왼쪽에 있으면 팀도 왼쪽으로
        // Home team in first half attacks right
        let slide = calculate_team_slide((20.0, field::CENTER_Y), TeamSide::Home, DefensiveLine::Normal, true);
        assert!(slide.0 < 0.0); // 왼쪽으로 슬라이드
    }

    #[test]
    fn test_team_slide_right() {
        // 공이 오른쪽에 있으면 팀도 오른쪽으로
        // Home team in first half attacks right
        let slide = calculate_team_slide((80.0, field::CENTER_Y), TeamSide::Home, DefensiveLine::Normal, true);
        assert!(slide.0 > 0.0);
    }

    #[test]
    fn test_defensive_line_depth() {
        // Home team in first half attacks right
        let high =
            calculate_team_slide((50.0, field::CENTER_Y), TeamSide::Home, DefensiveLine::VeryHigh, true);
        let deep =
            calculate_team_slide((50.0, field::CENTER_Y), TeamSide::Home, DefensiveLine::VeryDeep, true);

        // Very High은 더 높이 올라감 (음수), Very Deep은 더 내려감 (양수)
        assert!(high.1 < deep.1);
    }

    #[test]
    fn test_role_assignment_goalkeeper() {
        let ball_pos = (50.0, field::CENTER_Y);
        let defending_positions = vec![
            (5.0, field::CENTER_Y), // GK
            (20.0, field::CENTER_Y),
        ];
        let attacking_positions = vec![(50.0, field::CENTER_Y)];
        let states = vec![PlayerState::Idle; 2];

        let roles = assign_defensive_roles(
            ball_pos,
            &defending_positions,
            &attacking_positions,
            &states,
            0,
        );

        assert!(matches!(roles[0], DefensiveRole::Goalkeeper));
    }

    #[test]
    fn test_role_assignment_presser() {
        let ball_pos = (50.0, field::CENTER_Y);
        let defending_positions = vec![
            (5.0, field::CENTER_Y),  // GK
            (40.0, field::CENTER_Y), // Closest to ball
            (25.0, field::CENTER_Y),
        ];
        let attacking_positions = vec![(50.0, field::CENTER_Y)];
        let states = vec![PlayerState::Idle; 3];

        let roles = assign_defensive_roles(
            ball_pos,
            &defending_positions,
            &attacking_positions,
            &states,
            0,
        );

        assert!(matches!(roles[1], DefensiveRole::PresserPrimary));
    }

    #[test]
    fn test_presser_closing() {
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        let mut pos = (10.0, 0.0);
        let ball_pos = (5.0, 0.0);

        // 여러 틱 진행
        for _ in 0..10 {
            movement.update_tick(
                &mut pos,
                &PlayerState::Idle,
                ball_pos,
                Some(1),
                0,
                || false,
                || 0,
            );
        }

        // 공에 가까워져야 함
        let new_dist = distance(pos, ball_pos);
        let old_dist = distance((10.0, 0.0), ball_pos);
        assert!(new_dist < old_dist);
    }

    #[test]
    fn test_presser_closing_to_pressing() {
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        // Distance: 2.5 - 1.0 = 1.5m, which is <= PRESS_TARGET_DISTANCE (1.8m)
        let mut pos = (2.5, 0.0); // Already close enough
        let ball_pos = (1.0, 0.0);

        let result = movement.update_tick(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            || false,
            || 0,
        );

        assert!(matches!(result, Some(PresserEvent::StartedPressing)));
        assert!(matches!(movement.phase, PresserPhase::Pressing { .. }));
    }

    #[test]
    fn test_presser_swap_on_recovering() {
        let should = should_swap_presser(
            0,
            (20.0, field::CENTER_Y),
            &PlayerState::Recovering { remaining_ticks: 5 },
            0,
            (50.0, field::CENTER_Y),
            1,
            (40.0, field::CENTER_Y),
            &PlayerState::Idle,
        );

        assert!(should);
    }

    #[test]
    fn test_presser_swap_on_cooldown() {
        let should = should_swap_presser(
            0,
            (20.0, field::CENTER_Y),
            &PlayerState::Idle,
            10, // On cooldown
            (50.0, field::CENTER_Y),
            1,
            (40.0, field::CENTER_Y),
            &PlayerState::Idle,
        );

        assert!(should);
    }

    #[test]
    fn test_presser_no_swap_when_secondary_busy() {
        let should = should_swap_presser(
            0,
            (40.0, field::CENTER_Y),
            &PlayerState::Idle,
            0,
            (50.0, field::CENTER_Y),
            1,
            (30.0, field::CENTER_Y),                                    // Much closer     
            &PlayerState::Recovering { remaining_ticks: 5 }, // But recovering  
        );

        assert!(!should);
    }

    #[test]
    fn test_swap_presser_roles() {
        let mut roles = vec![
            DefensiveRole::Goalkeeper,
            DefensiveRole::PresserPrimary,
            DefensiveRole::PresserSecondary,
        ];

        swap_presser_roles(&mut roles, 1, 2);

        assert!(matches!(roles[1], DefensiveRole::PresserSecondary));
        assert!(matches!(roles[2], DefensiveRole::PresserPrimary));
    }

    #[test]
    fn test_apply_team_slide() {
        let base = vec![(20.0, field::CENTER_Y), (25.0, 20.0)];
        let slide = (5.0, 2.0);
        let states = vec![PlayerState::Idle; 2];
        let roles = vec![DefensiveRole::Cover; 2];

        let result = apply_team_slide(&base, slide, &states, &roles);

        assert_eq!(result[0], (25.0, 36.0));
        assert_eq!(result[1], (30.0, 22.0));
    }

    #[test]
    fn test_apply_team_slide_presser_excluded() {
        let base = vec![(20.0, field::CENTER_Y)];
        let slide = (5.0, 2.0);
        let states = vec![PlayerState::Idle];
        let roles = vec![DefensiveRole::PresserPrimary];

        let result = apply_team_slide(&base, slide, &states, &roles);

        // Presser는 슬라이드 적용 안됨
        assert_eq!(result[0], (20.0, field::CENTER_Y));
    }

    #[test]
    fn test_on_tackle_complete_success() {
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        movement.phase = PresserPhase::Tackling { tackle_action_id: 1 };

        movement.on_tackle_complete(true);

        if let PresserPhase::Recovering { remaining_ticks } = movement.phase {
            assert_eq!(remaining_ticks, PRESS_RECOVERY_TICKS / 2);
        } else {
            panic!("Expected Recovering phase");
        }
    }

    #[test]
    fn test_on_tackle_complete_failure() {
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        movement.phase = PresserPhase::Tackling { tackle_action_id: 1 };

        movement.on_tackle_complete(false);

        if let PresserPhase::Recovering { remaining_ticks } = movement.phase {
            assert_eq!(remaining_ticks, PRESS_RECOVERY_TICKS);
        } else {
            panic!("Expected Recovering phase");
        }
    }

    // =========================================================================
    // Interception Prediction Tests (FIX_2601/0106 P1)
    // =========================================================================

    #[test]
    fn test_predict_ball_position_linear() {
        let ball_pos = (50.0, field::CENTER_Y);
        let ball_vel = (10.0, 0.0); // 10 m/s 오른쪽
        let predicted = predict_ball_position(ball_pos, ball_vel, 1.0);

        assert!((predicted.0 - 60.0).abs() < 0.01);
        assert!((predicted.1 - field::CENTER_Y).abs() < 0.01);
    }

    #[test]
    fn test_predict_ball_position_diagonal() {
        let ball_pos = (0.0, 0.0);
        let ball_vel = (3.0, 4.0); // 5 m/s 대각선
        let predicted = predict_ball_position(ball_pos, ball_vel, 2.0);

        assert!((predicted.0 - 6.0).abs() < 0.01);
        assert!((predicted.1 - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_time_to_point_basic() {
        let player_pos = (0.0, 0.0);
        let target_pos = (6.0, 0.0);
        let speed = 6.0; // 6 m/s

        let time = estimate_time_to_point(player_pos, target_pos, speed);

        // 6m / 6m/s = 1s + 0.2s (반응시간) = 1.2s
        assert!((time - 1.2).abs() < 0.01);
    }

    #[test]
    fn test_estimate_time_to_point_zero_speed() {
        let player_pos = (0.0, 0.0);
        let target_pos = (6.0, 0.0);
        let speed = 0.0;

        let time = estimate_time_to_point(player_pos, target_pos, speed);

        assert_eq!(time, f32::MAX);
    }

    #[test]
    fn test_calculate_interception_point_stationary_ball() {
        let ball_pos = (50.0, field::CENTER_Y);
        let ball_vel = (0.0, 0.0);
        let defender_pos = (40.0, field::CENTER_Y);
        let defender_speed = 6.0;

        let (intercept_point, time) =
            calculate_interception_point(ball_pos, ball_vel, defender_pos, defender_speed);

        // 공이 정지해 있으면 공 위치가 인터셉트 지점
        assert!((intercept_point.0 - ball_pos.0).abs() < 0.01);
        assert!((intercept_point.1 - ball_pos.1).abs() < 0.01);

        // 10m / 6m/s + 0.2s = 1.87s
        assert!(time > 1.5 && time < 2.5);
    }

    #[test]
    fn test_calculate_interception_point_moving_ball() {
        let ball_pos = (30.0, field::CENTER_Y);
        let ball_vel = (10.0, 0.0); // 10 m/s 오른쪽
        let defender_pos = (50.0, field::CENTER_Y); // 공보다 오른쪽에 있음
        let defender_speed = 6.0;

        let (intercept_point, _time) =
            calculate_interception_point(ball_pos, ball_vel, defender_pos, defender_speed);

        // 인터셉트 지점은 수비수와 공 사이 어딘가
        assert!(intercept_point.0 > ball_pos.0);
        assert!(intercept_point.0 < defender_pos.0 + 10.0);
    }

    #[test]
    fn test_can_intercept_before_opponent_defender_wins() {
        // 수비수가 공에 더 가까움
        let defender_pos = (45.0, field::CENTER_Y);
        let defender_speed = 6.0;
        let ball_pos = (50.0, field::CENTER_Y);
        let ball_vel = (0.0, 0.0); // 정지
        let opponents = vec![((60.0, field::CENTER_Y), 6.0)]; // 상대는 더 멀리

        let result = can_intercept_before_opponent(
            defender_pos,
            defender_speed,
            ball_pos,
            ball_vel,
            &opponents,
        );

        // 수비수가 5m 더 가까우므로 인터셉트 가능
        assert!(result.can_intercept);
        assert!(result.time_margin > 0.0);
    }

    #[test]
    fn test_can_intercept_before_opponent_opponent_wins() {
        // 상대가 공에 더 가까움
        let defender_pos = (30.0, field::CENTER_Y);
        let defender_speed = 6.0;
        let ball_pos = (50.0, field::CENTER_Y);
        let ball_vel = (0.0, 0.0);
        let opponents = vec![((52.0, field::CENTER_Y), 6.0)]; // 상대가 훨씬 가까움        

        let result = can_intercept_before_opponent(
            defender_pos,
            defender_speed,
            ball_pos,
            ball_vel,
            &opponents,
        );

        // 상대가 훨씬 가까우므로 인터셉트 불가
        assert!(!result.can_intercept);
        assert!(result.time_margin < 0.0);
    }

    #[test]
    fn test_can_intercept_too_far() {
        // 인터셉트 지점이 너무 멀면 포기
        let defender_pos = (10.0, field::CENTER_Y);
        let defender_speed = 6.0;
        let ball_pos = (80.0, field::CENTER_Y);
        let ball_vel = (5.0, 0.0); // 더 멀어지는 방향
        let opponents = vec![((90.0, field::CENTER_Y), 6.0)];

        let result = can_intercept_before_opponent(
            defender_pos,
            defender_speed,
            ball_pos,
            ball_vel,
            &opponents,
        );

        // 거리가 MAX_INTERCEPT_DISTANCE 초과
        assert!(!result.can_intercept);
    }

    #[test]
    fn test_should_marker_intercept_ball_toward_target() {
        // 공이 마킹 대상 쪽으로 가고 있음
        let marker_pos = (40.0, field::CENTER_Y);
        let marker_speed = 6.0;
        let ball_pos = (30.0, field::CENTER_Y);
        let ball_vel = (10.0, 0.0); // 마킹 대상 쪽으로
        let marked_player_pos = (60.0, field::CENTER_Y);
        let marked_player_speed = 6.0;

        let result = should_marker_intercept(
            marker_pos,
            marker_speed,
            ball_pos,
            ball_vel,
            marked_player_pos,
            marked_player_speed,
        );

        // 마커가 공과 마킹 대상 사이에 있으므로 인터셉트 시도 가능
        // (시간 마진에 따라 can_intercept 결정)
        assert!(result.time_to_intercept < f32::MAX);
    }

    #[test]
    fn test_should_marker_intercept_ball_away_from_target() {
        // 공이 마킹 대상 반대 방향으로 가고 있음
        let marker_pos = (40.0, field::CENTER_Y);
        let marker_speed = 6.0;
        let ball_pos = (50.0, field::CENTER_Y);
        let ball_vel = (10.0, 0.0); // 마킹 대상 반대 방향
        let marked_player_pos = (30.0, field::CENTER_Y);
        let marked_player_speed = 6.0;

        let result = should_marker_intercept(
            marker_pos,
            marker_speed,
            ball_pos,
            ball_vel,
            marked_player_pos,
            marked_player_speed,
        );

        // 공이 반대로 가면 인터셉트 불필요
        assert!(!result.can_intercept);
    }

    #[test]
    fn test_should_marker_intercept_stationary_ball() {
        // 공이 정지해 있으면 인터셉트 불가
        let marker_pos = (40.0, field::CENTER_Y);
        let marker_speed = 6.0;
        let ball_pos = (50.0, field::CENTER_Y);
        let ball_vel = (0.0, 0.0);
        let marked_player_pos = (60.0, field::CENTER_Y);
        let marked_player_speed = 6.0;

        let result = should_marker_intercept(
            marker_pos,
            marker_speed,
            ball_pos,
            ball_vel,
            marked_player_pos,
            marked_player_speed,
        );

        assert!(!result.can_intercept);
    }

    // =========================================================================
    // Marker Integration Tests (FIX_2601/0106 P1)
    // =========================================================================

    #[test]
    fn test_marker_movement_basic_marking() {
        // 기본 마킹: 공이 정지해 있을 때
        let mut marker_pos = (40.0, field::CENTER_Y);
        let target_attacker_pos = (60.0, field::CENTER_Y);
        let own_goal_pos = (0.0, field::CENTER_Y);
        let ball_pos = (50.0, field::CENTER_Y);

        update_marker_movement(
            &mut marker_pos,
            &PlayerState::Idle,
            target_attacker_pos,
            own_goal_pos,
            ball_pos,
        );

        // 마커가 공격수 쪽으로 이동
        assert!(marker_pos.0 > 40.0);
    }

    #[test]
    fn test_marker_movement_intercept_mode() {
        // 인터셉트 모드: 공이 마킹 대상 쪽으로 이동 중
        let mut marker_pos = (45.0, field::CENTER_Y);
        let target_attacker_pos = (70.0, field::CENTER_Y);
        let own_goal_pos = (0.0, field::CENTER_Y);
        let ball_pos = (30.0, field::CENTER_Y);
        let ball_velocity = (15.0, 0.0); // 빠르게 마킹 대상 쪽으로
        let target_speed = 6.0;

        let original_pos = marker_pos;

        update_marker_movement_with_ball_velocity(
            &mut marker_pos,
            &PlayerState::Idle,
            target_attacker_pos,
            own_goal_pos,
            ball_pos,
            ball_velocity,
            target_speed,
        );

        // 마커가 움직임 (인터셉트 또는 마킹)
        assert!(marker_pos.0 != original_pos.0 || marker_pos.1 != original_pos.1);
    }

    #[test]
    fn test_marker_movement_no_move_when_recovering() {
        // Recovering 상태면 움직이지 않음
        let mut marker_pos = (40.0, field::CENTER_Y);
        let target_attacker_pos = (60.0, field::CENTER_Y);
        let own_goal_pos = (0.0, field::CENTER_Y);
        let ball_pos = (50.0, field::CENTER_Y);
        let ball_velocity = (10.0, 0.0);

        let original_pos = marker_pos;

        update_marker_movement_with_ball_velocity(
            &mut marker_pos,
            &PlayerState::Recovering { remaining_ticks: 5 },
            target_attacker_pos,
            own_goal_pos,
            ball_pos,
            ball_velocity,
            6.0,
        );

        // 위치 변화 없음
        assert_eq!(marker_pos, original_pos);
    }

    // =========================================================================
    // Stamina Resting Tests (FIX_2601/0106 P2)
    // =========================================================================

    #[test]
    fn test_stamina_threshold_constants() {
        // 상수 값 확인 (0-1 스케일)
        assert_eq!(STAMINA_REST_THRESHOLD, 0.30);
        assert_eq!(STAMINA_RESUME_THRESHOLD, 0.70);
        assert_eq!(RESTING_TICKS, 40);
        assert_eq!(RESTING_WALK_SPEED, 1.5);
        assert_eq!(BALL_PROXIMITY_THRESHOLD, 10.0);
    }

    #[test]
    fn test_presser_enters_resting_on_low_stamina() {
        // 스태미나가 낮고 공이 멀면 Resting으로 전환
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        let mut pos = (10.0, 0.0);
        let ball_pos = (50.0, 0.0); // 40m 떨어짐 (> 10m threshold)

        let result = movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.25, // 스태미나 25% (임계값 30% 미만), 0-1 스케일
            Some((20.0, field::CENTER_Y)),
            || false,
            || 0,
        );

        // 휴식 진입 확인 (stamina 값도 0-1 스케일로 검증)
        assert!(matches!(result, Some(PresserEvent::StartedResting { .. })));
        assert!(matches!(
            movement.phase,
            PresserPhase::Resting { remaining_ticks: 40, return_to_closing: true }
        ));
    }

    #[test]
    fn test_presser_stays_active_with_good_stamina() {
        // 스태미나가 충분하면 정상 동작
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        let mut pos = (10.0, 0.0);
        let ball_pos = (5.0, 0.0);

        let result = movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.80, // 스태미나 80% (충분), 0-1 스케일
            None,
            || false,
            || 0,
        );

        // Resting 이벤트가 아님
        assert!(!matches!(result, Some(PresserEvent::StartedResting { .. })));
        // Resting Phase가 아님
        assert!(!matches!(movement.phase, PresserPhase::Resting { .. }));
    }

    #[test]
    fn test_presser_stays_active_when_ball_near_despite_low_stamina() {
        // 스태미나가 낮아도 공이 가까우면 휴식 진입 안함
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        let mut pos = (10.0, 0.0);
        let ball_pos = (15.0, 0.0); // 5m 떨어짐 (< 10m threshold)

        let result = movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.25, // 스태미나 25% (임계값 30% 미만), 0-1 스케일
            Some((20.0, field::CENTER_Y)),
            || false,
            || 0,
        );

        // 공이 가까우므로 Resting 진입 안함
        assert!(!matches!(result, Some(PresserEvent::StartedResting { .. })));
        assert!(!matches!(movement.phase, PresserPhase::Resting { .. }));
    }

    #[test]
    fn test_presser_resting_countdown() {
        // 휴식 중 카운트다운 (공이 멀고 스태미나 아직 낮음)
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        movement.phase = PresserPhase::Resting { remaining_ticks: 10, return_to_closing: true };
        let mut pos = (10.0, field::CENTER_Y);
        let ball_pos = (50.0, field::CENTER_Y); // 40m 떨어짐 (> 10m)

        let result = movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.35, // 스태미나 35% (회복 중이지만 아직 50% 미만), 0-1 스케일
            Some((20.0, field::CENTER_Y)),
            || false,
            || 0,
        );

        assert!(result.is_none());
        if let PresserPhase::Resting { remaining_ticks, .. } = movement.phase {
            assert_eq!(remaining_ticks, 9);
        } else {
            panic!("Expected Resting phase");
        }
    }

    #[test]
    fn test_presser_resting_walks_to_target() {
        // 휴식 중 목표 위치로 걷기
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        movement.phase = PresserPhase::Resting { remaining_ticks: 20, return_to_closing: true };
        let mut pos = (10.0, field::CENTER_Y);
        let ball_pos = (50.0, field::CENTER_Y); // 40m 떨어짐 (> 10m)
        let target_pos = Some((20.0, field::CENTER_Y));

        let original_x = pos.0;

        movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.35, // 0-1 스케일
            target_pos,
            || false,
            || 0,
        );

        // 걷기 속도로 목표 쪽으로 이동
        assert!(pos.0 > original_x);
        // 걷기 속도 체크 (RESTING_WALK_SPEED * TICK_DT = 1.5 * 0.25 = 0.375m)
        assert!((pos.0 - original_x - 0.375).abs() < 0.01);
    }

    #[test]
    fn test_presser_resting_early_recovery() {
        // 스태미나가 회복되면 조기 복귀
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        movement.phase = PresserPhase::Resting { remaining_ticks: 30, return_to_closing: true };
        let mut pos = (10.0, field::CENTER_Y);
        let ball_pos = (50.0, field::CENTER_Y); // 40m 떨어짐 (> 10m)

        let result = movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.75, // 스태미나 75% (복귀 임계값 70% 초과), 0-1 스케일
            None,
            || false,
            || 0,
        );

        assert!(matches!(result, Some(PresserEvent::RestingComplete)));
        assert!(matches!(movement.phase, PresserPhase::Closing { .. }));
    }

    #[test]
    fn test_presser_resting_complete_returns_to_pressing() {
        // 휴식 완료 후 Pressing으로 복귀 (시간 만료)
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        movement.phase = PresserPhase::Resting {
            remaining_ticks: 1,
            return_to_closing: false, // Pressing에서 휴식 시작
        };
        let mut pos = (10.0, field::CENTER_Y);
        let ball_pos = (50.0, field::CENTER_Y); // 40m 떨어짐 (> 10m)

        let result = movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.40, // 아직 낮지만 시간 만료, 0-1 스케일
            None,
            || false,
            || 0,
        );

        assert!(matches!(result, Some(PresserEvent::RestingComplete)));
        assert!(matches!(movement.phase, PresserPhase::Pressing { .. }));
    }

    #[test]
    fn test_presser_resting_no_stamina_check_during_rest() {
        // 이미 Resting 중이면 스태미나 체크 안함 (무한 루프 방지)
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        movement.phase = PresserPhase::Resting { remaining_ticks: 20, return_to_closing: true };
        let mut pos = (10.0, field::CENTER_Y);
        let ball_pos = (50.0, field::CENTER_Y); // 40m 떨어짐 (> 10m)

        let result = movement.update_tick_with_stamina(
            &mut pos,
            &PlayerState::Idle,
            ball_pos,
            Some(1),
            0,
            0.20, // 스태미나 여전히 낮음 (20%), 0-1 스케일
            None,
            || false,
            || 0,
        );

        // StartedResting 이벤트가 아님 (이미 휴식 중이므로)
        assert!(!matches!(result, Some(PresserEvent::StartedResting { .. })));
        // 여전히 Resting Phase (공이 멀고 스태미나가 아직 낮으므로)
        assert!(matches!(movement.phase, PresserPhase::Resting { .. }));
    }

    #[test]
    fn test_legacy_update_tick_no_stamina_issue() {
        // 레거시 update_tick은 스태미나 100%로 동작
        let mut movement = PresserMovement::new(0, DefensiveRole::PresserPrimary);
        let mut pos = (10.0, 0.0);
        let ball_pos = (5.0, 0.0);

        // 여러 틱 실행해도 Resting으로 가지 않음
        for _ in 0..10 {
            movement.update_tick(
                &mut pos,
                &PlayerState::Idle,
                ball_pos,
                Some(1),
                0,
                || false,
                || 0,
            );
        }

        // Resting Phase가 아님
        assert!(!matches!(movement.phase, PresserPhase::Resting { .. }));
    }

    // ========================================================================
    // P3b: Unified Defensive State Tests
    // ========================================================================

    #[test]
    fn test_defensive_state_to_kind() {
        // Presser states
        let state = DefensiveState::PresserClosing { target_distance: 2.0 };
        assert_eq!(state.to_kind(), DefensiveStateKind::PresserClosing);

        let state = DefensiveState::PresserPressing { hold_distance: 1.8, hold_ticks: 10 };
        assert_eq!(state.to_kind(), DefensiveStateKind::PresserPressing);

        // Marker states
        let state = DefensiveState::MarkerFollowing { offset_distance: 1.5 };
        assert_eq!(state.to_kind(), DefensiveStateKind::MarkerFollowing);

        let state = DefensiveState::MarkerShadowing { intercept_distance: 5.0 };
        assert_eq!(state.to_kind(), DefensiveStateKind::MarkerShadowing);

        // Cover states
        let state = DefensiveState::CoverHolding { target_pos: (30.0, field::CENTER_Y) };
        assert_eq!(state.to_kind(), DefensiveStateKind::CoverHolding);
    }

    #[test]
    fn test_defensive_movement_new_presser() {
        let movement = DefensiveMovement::new_presser(5, true);
        assert_eq!(movement.player_idx, 5);
        assert!(matches!(movement.role, DefensiveRole::PresserPrimary));
        assert!(matches!(movement.state, DefensiveState::PresserClosing { .. }));
        assert_eq!(movement.target_mark_idx, None);
    }

    #[test]
    fn test_defensive_movement_new_marker() {
        let movement = DefensiveMovement::new_marker(3, 15);
        assert_eq!(movement.player_idx, 3);
        assert!(matches!(movement.role, DefensiveRole::Marker { target_idx: 15 }));
        assert!(matches!(movement.state, DefensiveState::MarkerFollowing { .. }));
        assert_eq!(movement.target_mark_idx, Some(15));
    }

    #[test]
    fn test_defensive_movement_new_cover() {
        let movement = DefensiveMovement::new_cover(7, (25.0, 30.0));
        assert_eq!(movement.player_idx, 7);
        assert!(matches!(movement.role, DefensiveRole::Cover));
        assert!(matches!(
            movement.state,
            DefensiveState::CoverHolding { target_pos: (25.0, 30.0) }
        ));
        assert_eq!(movement.target_mark_idx, None);
    }

    #[test]
    fn test_defensive_movement_stamina_check_enters_resting() {
        let mut movement = DefensiveMovement::new_presser(5, true);

        // 스태미나 낮고 공이 멀면 Resting 진입
        let result = movement.check_stamina_and_enter_resting(0.25, 20.0);

        assert!(result.is_some());
        assert!(matches!(movement.state, DefensiveState::Resting { .. }));
    }

    #[test]
    fn test_defensive_movement_stamina_check_no_resting_when_ball_near() {
        let mut movement = DefensiveMovement::new_presser(5, true);

        // 스태미나 낮아도 공이 가까우면 Resting 안함
        let result = movement.check_stamina_and_enter_resting(0.25, 5.0); // ball_distance < 10m

        assert!(result.is_none());
        assert!(matches!(movement.state, DefensiveState::PresserClosing { .. }));
    }

    #[test]
    fn test_defensive_movement_stamina_check_no_resting_when_stamina_ok() {
        let mut movement = DefensiveMovement::new_presser(5, true);

        // 스태미나 충분하면 Resting 안함
        let result = movement.check_stamina_and_enter_resting(0.50, 20.0); // stamina >= 30%

        assert!(result.is_none());
        assert!(matches!(movement.state, DefensiveState::PresserClosing { .. }));
    }

    #[test]
    fn test_defensive_movement_stamina_check_already_resting() {
        let mut movement = DefensiveMovement::new_presser(5, true);
        movement.state = DefensiveState::Resting {
            remaining_ticks: 20,
            return_to: DefensiveStateKind::PresserClosing,
        };

        // 이미 Resting 중이면 중복 진입 안함
        let result = movement.check_stamina_and_enter_resting(0.25, 20.0);

        assert!(result.is_none());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_defensive_state_role_validation() {
        // Presser can have Presser states
        let state = DefensiveState::PresserClosing { target_distance: 2.0 };
        assert!(state.is_valid_for_role(&DefensiveRole::PresserPrimary));
        assert!(state.is_valid_for_role(&DefensiveRole::PresserSecondary));
        assert!(!state.is_valid_for_role(&DefensiveRole::Cover));

        // Marker can have Marker states
        let state = DefensiveState::MarkerFollowing { offset_distance: 1.5 };
        assert!(state.is_valid_for_role(&DefensiveRole::Marker { target_idx: 0 }));
        assert!(!state.is_valid_for_role(&DefensiveRole::Cover));

        // Cover can have Cover states
        let state = DefensiveState::CoverHolding { target_pos: (30.0, field::CENTER_Y) };
        assert!(state.is_valid_for_role(&DefensiveRole::Cover));
        assert!(!state.is_valid_for_role(&DefensiveRole::PresserPrimary));

        // All roles can be Resting
        let state = DefensiveState::Resting {
            remaining_ticks: 20,
            return_to: DefensiveStateKind::PresserClosing,
        };
        assert!(state.is_valid_for_role(&DefensiveRole::PresserPrimary));
        assert!(state.is_valid_for_role(&DefensiveRole::Marker { target_idx: 0 }));
        assert!(state.is_valid_for_role(&DefensiveRole::Cover));
    }
}
