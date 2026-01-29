//! ActionQueue System
//!
//! P7 Phase-Based 틱 기반 액션 스케줄링 및 실행 시스템.
//! 패스 → 트랩 → 드리블 → 슛 같은 연속 액션 체인을 관리합니다.
//!
//! ## 핵심 개념
//! - **ScheduledAction**: 미래 틱에 실행될 액션 (pending)
//! - **ActiveAction**: 실행 중인 FSM 액션 (active)
//! - **BallState**: 공의 현재 상태 (소유, 비행 중, 루즈볼)
//! - **ActionQueue**: 액션 예약 및 실행 관리자
//!
//! ## P7 Phase Flow
//! ```text
//! pending → active (Approach → Commit → Resolve → Recover → Cooldown → Finished)
//! ```

use fxhash::FxHasher;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::hash::{Hash, Hasher};
#[cfg(debug_assertions)]
use std::sync::{Mutex, OnceLock};

// P16: ActionDetail import
use super::action_detail::ActionDetail;
// FIX_2601/0110: DirectionContext for halftime direction swap
use super::types::DirectionContext;

// P7: Phase-Based Action imports (duration constants + physics functions)
use super::phase_action::{
    choose_pass_technique,
    choose_shot_technique,
    pass_base_success_prob,
    PassContext,
    PassIntent,
    PassSkills,
    PassTechnique,
    PassTechniqueSelection,
    ShooterSkills,
    ShotContext,
    // ActionModel Integration: Shot/Pass physics
    ShotIntent,
    ShotTechnique,
    PASS_COOLDOWN_TICKS,
    PASS_KICK_TICKS,
    PASS_RECOVERY_TICKS,
    SHOT_COOLDOWN_TICKS,
    SHOT_RECOVERY_TICKS,
    SHOT_STRIKE_TICKS,
    TICKS_PER_SECOND,
    TACKLE_APPROACH_MAX_TICKS,
    TACKLE_APPROACH_SPEED,
    TACKLE_COMMIT_STANDING_TICKS,
    TACKLE_COOLDOWN_TICKS,
    TACKLE_RECOVERY_MISS_TICKS,
};
use crate::engine::ball::{
    compute_lift_ratio, get_ball_position_3d_with_endpoints, get_ball_position_3d_with_height,
    max_height_from_profile, HeightProfile,
};
use crate::engine::debug_flags::action_debug_enabled;
use crate::engine::physics_constants::{aerial, ball as physics_ball, goal, field};
use crate::models::TeamSide;

// FIX_2601: Coord10/Vel10 정수 좌표계
use super::types::coord10::{Coord10, Vel10};

// Contract v1: OutcomeSet sampler
use crate::engine::match_sim::decision_topology::{select_outcome_softmax, ShotOutcome};
use crate::engine::weights::WeightBreakdown;

#[cfg(debug_assertions)]
#[derive(Default)]
struct ThroughHeightStats {
    count: u64,
    sum: f64,
    max: f32,
}

#[cfg(debug_assertions)]
static THROUGH_HEIGHT_STATS: OnceLock<Mutex<ThroughHeightStats>> = OnceLock::new();

#[cfg(debug_assertions)]
fn record_through_height(height_m: f32) {
    let stats = THROUGH_HEIGHT_STATS.get_or_init(|| Mutex::new(ThroughHeightStats::default()));
    let mut stats = stats.lock().expect("through height stats lock poisoned");
    stats.count += 1;
    stats.sum += height_m as f64;
    if height_m > stats.max {
        stats.max = height_m;
    }
}

#[cfg(debug_assertions)]
pub fn debug_through_height_stats() -> Option<(u64, f32, f32)> {
    let stats = THROUGH_HEIGHT_STATS.get_or_init(|| Mutex::new(ThroughHeightStats::default()));
    let stats = stats.lock().ok()?;
    if stats.count == 0 {
        return None;
    }
    let avg = (stats.sum / stats.count as f64) as f32;
    Some((stats.count, avg, stats.max))
}

fn in_flight_ball_pos_at_tick(ball_state: &BallState, tick: u64) -> Option<(f32, f32, f32)> {
    let BallState::InFlight {
        from_pos,
        to_pos,
        start_tick,
        end_tick,
        height_profile,
        lift_ratio,
        start_height_01m,
        end_height_01m,
        ..
    } = ball_state
    else {
        return None;
    };

    let total_duration = end_tick.saturating_sub(*start_tick);
    let elapsed = tick.saturating_sub(*start_tick);
    let t = if total_duration > 0 {
        (elapsed as f32 / total_duration as f32).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let bump_height = max_height_from_profile(*height_profile, *lift_ratio);
    let start_height_m = *start_height_01m as f32 * 0.1;
    let end_height_m = *end_height_01m as f32 * 0.1;
    let (x, y, z) = get_ball_position_3d_with_endpoints(
        from_pos.to_meters(),
        to_pos.to_meters(),
        0.0, // curve_factor unavailable in BallState; linear is sufficient for logs
        bump_height,
        start_height_m,
        end_height_m,
        t,
    );
    Some((x, y, z))
}

/// FIX_2601/1120: Get current ball position (x, y) from BallState at given tick.
/// This is used to ensure InFlight transitions start from the actual ball position,
/// not the player position (Pattern B fix - InFlightOriginJump).
///
/// Returns None if the ball position cannot be determined (should not happen in normal play).
fn get_ball_position_at_tick(
    ball_state: &BallState,
    tick: u64,
    player_positions: &[(f32, f32)],
) -> Option<(f32, f32)> {
    match ball_state {
        BallState::InFlight { .. } => {
            // For InFlight, calculate the position at the given tick
            in_flight_ball_pos_at_tick(ball_state, tick).map(|(x, y, _z)| (x, y))
        }
        BallState::Controlled { owner_idx } => {
            // For Controlled, ball is at the owner's feet
            player_positions.get(*owner_idx).copied()
        }
        BallState::Loose { position, .. } => {
            // For Loose, ball is at its recorded position
            Some(position.to_meters())
        }
        BallState::OutOfPlay { position, .. } => {
            // For OutOfPlay, ball is at the restart position
            Some(position.to_meters())
        }
    }
}


// ============================================================================
// Phase-Based Action Core Types (Moved from phase_action/types.rs - P0)
// ============================================================================

/// 액션의 실행 단계
///
/// 모든 액션은 이 Phase들을 순서대로 거친다:
/// Pending → Approach → Commit → Resolve → Recover → Cooldown → Finished
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPhase {
    /// 대기 중 (아직 시작 안 함)
    Pending,
    /// 접근 중 (목표로 이동)
    Approach,
    /// 실행 중 (태클 동작, 킥 동작)
    Commit,
    /// 판정 중 (성공/실패/파울 결정)
    Resolve,
    /// 회복 중 (일어나기, 균형 잡기)
    Recover,
    /// 쿨다운 (다시 시도 불가)
    Cooldown,
    /// 완료
    Finished,
}

impl ActionPhase {
    /// 다음 Phase 반환
    pub fn next(&self) -> Self {
        match self {
            ActionPhase::Pending => ActionPhase::Approach,
            ActionPhase::Approach => ActionPhase::Commit,
            ActionPhase::Commit => ActionPhase::Resolve,
            ActionPhase::Resolve => ActionPhase::Recover,
            ActionPhase::Recover => ActionPhase::Cooldown,
            ActionPhase::Cooldown => ActionPhase::Finished,
            ActionPhase::Finished => ActionPhase::Finished,
        }
    }

    /// 이 Phase에서 선수가 이동 가능한지
    pub fn can_move(&self) -> bool {
        matches!(
            self,
            ActionPhase::Pending
                | ActionPhase::Approach
                | ActionPhase::Cooldown
                | ActionPhase::Finished
        )
    }

    /// 이 Phase에서 다른 액션을 시작할 수 있는지
    pub fn can_start_action(&self) -> bool {
        matches!(self, ActionPhase::Pending | ActionPhase::Finished)
    }

    /// 이 Phase가 활성 상태인지 (아직 진행 중)
    pub fn is_active(&self) -> bool {
        !matches!(self, ActionPhase::Finished)
    }
}

/// Phase 기반 액션 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhaseActionType {
    Tackle,
    Pass,
    Shot,
    Dribble,
    Move,
    Trap,
    Intercept,
    Header,
    Save,
}

impl PhaseActionType {
    /// 이 액션이 공을 필요로 하는지
    pub fn requires_ball(&self) -> bool {
        matches!(self, PhaseActionType::Pass | PhaseActionType::Shot | PhaseActionType::Dribble)
    }

    /// 이 액션에 쿨다운이 있는지
    pub fn has_cooldown(&self) -> bool {
        matches!(self, PhaseActionType::Tackle | PhaseActionType::Shot)
    }
}

/// 액션별 메타데이터
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ActionMeta {
    /// 태클 메타데이터
    Tackle { tackle_type: TackleType, approach_speed: f32 },

    /// 패스 메타데이터 (FIX_2601: Coord10)
    /// FIX_2601/1129: intended_passer_pos added for forward_pass_rate measurement
    Pass { pass_type: PassType, target_pos: Coord10, pass_speed: f32, intended_passer_pos: Option<Coord10> },

    /// 슈팅 메타데이터 (FIX_2601: Coord10)
    Shot { shot_type: ShotType, target_pos: Coord10, power: f32 },

    /// 드리블 메타데이터 (direction은 단위 벡터, 튜플 유지)
    Dribble { direction: (f32, f32), is_aggressive: bool, touch_timer: u8 },

    /// 이동 메타데이터 (FIX_2601: Coord10)
    Move { target: Coord10, is_sprint: bool },

    /// 헤더 메타데이터 (FIX_2601/0115: is_shot 손실 버그 수정)
    Header { is_shot: bool },

    /// 기타/없음
    #[default]
    None,
}

/// 태클 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TackleType {
    /// 스탠딩 태클
    Standing,
    /// 슬라이딩 태클
    Sliding,
    /// 어깨 밀치기
    Shoulder,
}

impl TackleType {
    /// 태클 도달 거리 (미터)
    pub fn reach(&self) -> f32 {
        match self {
            TackleType::Standing => 1.5,
            TackleType::Sliding => 2.2,
            TackleType::Shoulder => 0.8,
        }
    }

    /// Commit Phase 기본 틱 수
    pub fn commit_ticks(&self) -> u8 {
        match self {
            TackleType::Standing => 2,
            TackleType::Sliding => 4,
            TackleType::Shoulder => 1,
        }
    }

    /// Recovery 기본 틱 수
    pub fn recovery_ticks(&self) -> u8 {
        match self {
            TackleType::Standing => 4,
            TackleType::Sliding => 12,
            TackleType::Shoulder => 2,
        }
    }
}

/// 패스 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PassType {
    /// 땅볼 패스
    Ground,
    /// 띄우기 패스
    Lofted,
    /// 쓰루패스
    ThroughBall,
    /// 크로스
    Cross,
    /// 백패스
    BackPass,
}

/// Origin marker for the current in-flight delivery segment.
///
/// This is SSOT metadata used for rulebook enforcement (e.g., throw-in → GK handling restriction)
/// without routing through `PassStarted` (keeps offside exemptions structural).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InFlightOrigin {
    /// A throw-in restart delivery (team is identified, not the thrower).
    ThrowIn { throwing_home: bool },
}

impl PassType {
    /// 기본 속도 (m/s)
    pub fn base_speed(&self) -> f32 {
        match self {
            PassType::Ground => 12.0,
            PassType::Lofted => 15.0,
            PassType::ThroughBall => 18.0,
            PassType::Cross => 20.0,
            PassType::BackPass => 10.0,
        }
    }

    /// Windup 틱 수
    pub fn windup_ticks(&self) -> u8 {
        match self {
            PassType::Ground => 1,
            PassType::Lofted => 2,
            PassType::ThroughBall => 2,
            PassType::Cross => 3,
            PassType::BackPass => 1,
        }
    }

    /// Recovery 틱 수 (킥 후 회복)
    pub fn recovery_ticks(&self) -> u8 {
        match self {
            PassType::Ground => 1,
            PassType::Lofted => 2,
            PassType::ThroughBall => 2,
            PassType::Cross => 2,
            PassType::BackPass => 1,
        }
    }

    pub fn is_long(&self) -> bool {
        matches!(self, PassType::Lofted | PassType::Cross)
    }

    pub fn is_through(&self) -> bool {
        matches!(self, PassType::ThroughBall)
    }

    pub fn from_detail(detail: crate::engine::action_detail::PassType) -> Self {
        use crate::engine::action_detail::PassType as DetailPassType;
        match detail {
            DetailPassType::Short => PassType::Ground,
            DetailPassType::Through => PassType::ThroughBall,
            // FIX_2601/0115: Long/Switch now go through Ground scoring (was: all -> Lofted)
            // This allows choose_pass_technique() to select optimal technique based on context
            DetailPassType::Long | DetailPassType::Switch => PassType::Ground,
            // Lob/Clear explicitly require lofted technique (high ball intent)
            DetailPassType::Lob | DetailPassType::Clear => PassType::Lofted,
            DetailPassType::Cross => PassType::Cross,
            DetailPassType::Cutback => PassType::Ground,
            DetailPassType::Back => PassType::BackPass,
        }
    }

    pub fn to_detail(self) -> crate::engine::action_detail::PassType {
        use crate::engine::action_detail::PassType as DetailPassType;
        match self {
            PassType::Ground => DetailPassType::Short,
            PassType::Lofted => DetailPassType::Long,
            PassType::ThroughBall => DetailPassType::Through,
            PassType::Cross => DetailPassType::Cross,
            PassType::BackPass => DetailPassType::Back,
        }
    }
}

/// 슈팅 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShotType {
    /// 일반 슛
    Normal,
    /// 감아차기
    Finesse,
    /// 강슛
    Power,
    /// 칩슛
    Chip,
    /// 헤딩슛
    Header,
    /// 발리슛
    Volley,
    /// 원터치슛
    OneTouch,
}

impl ShotType {
    /// 기본 속도 (m/s)
    pub fn base_speed(&self) -> f32 {
        match self {
            ShotType::Normal => 25.0,
            ShotType::Finesse => 20.0,
            ShotType::Power => 35.0,
            ShotType::Chip => 15.0,
            ShotType::Header => 18.0,
            ShotType::Volley => 30.0,
            ShotType::OneTouch => 22.0,
        }
    }

    /// Windup 틱 수
    pub fn windup_ticks(&self) -> u8 {
        match self {
            ShotType::Normal => 3,
            ShotType::Finesse => 3,
            ShotType::Power => 4,
            ShotType::Chip => 2,
            ShotType::Header => 1,
            ShotType::Volley => 2,
            ShotType::OneTouch => 1,
        }
    }

    /// 기본 정확도 계수 (0.0 ~ 1.0)
    pub fn accuracy_factor(&self) -> f32 {
        match self {
            ShotType::Normal => 0.7,
            ShotType::Finesse => 0.85,
            ShotType::Power => 0.5,
            ShotType::Chip => 0.6,
            ShotType::Header => 0.55,
            ShotType::Volley => 0.45,
            ShotType::OneTouch => 0.5,
        }
    }

    /// Follow-through 틱 수 (슛 후 회복)
    pub fn follow_through_ticks(&self) -> u8 {
        match self {
            ShotType::Normal => 2,
            ShotType::Finesse => 2,
            ShotType::Power => 3,
            ShotType::Chip => 2,
            ShotType::Header => 1,
            ShotType::Volley => 2,
            ShotType::OneTouch => 1,
        }
    }
}

/// 태클 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TackleOutcome {
    /// 깔끔하게 공 탈취
    CleanWin,
    /// 공만 건드림 (루즈볼)
    Deflection,
    /// 헛발
    Miss,
    /// 파울
    Foul,
    /// 경고
    YellowCard,
    /// 퇴장
    RedCard,
}

/// 실행 중인 FSM 액션
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAction {
    /// 고유 액션 ID
    pub id: u64,

    /// 실행 주체 선수 인덱스
    pub player_idx: usize,

    /// 선수 팀 ID
    pub team_id: u32,

    /// 액션 타입
    pub action_type: PhaseActionType,

    /// 현재 Phase
    pub phase: ActionPhase,

    /// 현재 Phase 시작 틱
    pub phase_tick_start: u64,

    /// 현재 Phase 지속 시간 (틱)
    pub phase_duration: u64,

    /// 액션 전체 시작 틱
    pub total_started_tick: u64,

    /// 타겟 선수 인덱스 (태클, 패스 등)
    pub target_player_idx: Option<usize>,

    /// 타겟 위치 (패스, 슈팅 등) (FIX_2601: Coord10)
    pub target_position: Option<Coord10>,

    /// 액션별 메타데이터
    pub meta: ActionMeta,
}

impl ActiveAction {
    /// 새 액션 생성
    pub fn new(
        id: u64,
        player_idx: usize,
        team_id: u32,
        action_type: PhaseActionType,
        current_tick: u64,
    ) -> Self {
        Self {
            id,
            player_idx,
            team_id,
            action_type,
            phase: ActionPhase::Pending,
            phase_tick_start: current_tick,
            phase_duration: 0,
            total_started_tick: current_tick,
            target_player_idx: None,
            target_position: None,
            meta: ActionMeta::None,
        }
    }

    /// 현재 Phase에서 경과한 틱 수
    pub fn ticks_in_phase(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.phase_tick_start)
    }

    /// Phase 지속 시간이 끝났는지
    pub fn is_phase_complete(&self, current_tick: u64) -> bool {
        self.ticks_in_phase(current_tick) >= self.phase_duration
    }

    /// 다음 Phase로 전환
    pub fn advance_phase(&mut self, current_tick: u64, new_duration: u64) {
        self.phase = self.phase.next();
        self.phase_tick_start = current_tick;
        self.phase_duration = new_duration;
    }

    /// 액션 완료 처리
    pub fn finish(&mut self) {
        self.phase = ActionPhase::Finished;
    }

    /// 전체 경과 틱 수
    pub fn total_elapsed(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.total_started_tick)
    }
}

// ============================================================================
// Viewer Event Types
// ============================================================================

/// 공 궤적 인텐트 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BallIntentKind {
    /// 일반 패스
    Pass,
    /// 스루 패스
    Through,
    /// 크로스
    Cross,
    /// 일반 슛
    Shot,
    /// 칩샷
    Chip,
    /// 헤더
    Header,
}

impl BallIntentKind {
    /// 문자열로 변환 (Viewer용)
    pub fn as_str(&self) -> &'static str {
        match self {
            BallIntentKind::Pass => "pass",
            BallIntentKind::Through => "through",
            BallIntentKind::Cross => "cross",
            BallIntentKind::Shot => "shot",
            BallIntentKind::Chip => "chip",
            BallIntentKind::Header => "header",
        }
    }

    /// 기준 속도 (m/s)
    pub fn v_ref(&self) -> f32 {
        match self {
            BallIntentKind::Pass => 12.0,
            BallIntentKind::Through => 18.0,
            BallIntentKind::Cross => 16.0,
            BallIntentKind::Shot => 26.0,
            BallIntentKind::Chip => 14.0,
            BallIntentKind::Header => 12.0,
        }
    }
}

/// 속도 클래스
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpeedClass {
    Slow,
    #[default]
    Normal,
    Fast,
}

impl SpeedClass {
    /// 속도 배율
    pub fn multiplier(&self) -> f32 {
        match self {
            SpeedClass::Slow => 1.15,
            SpeedClass::Normal => 1.0,
            SpeedClass::Fast => 0.85,
        }
    }

    /// 문자열로 변환
    pub fn as_str(&self) -> &'static str {
        match self {
            SpeedClass::Slow => "slow",
            SpeedClass::Normal => "normal",
            SpeedClass::Fast => "fast",
        }
    }
}

/// 높이 클래스
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HeightClass {
    Ground,
    #[default]
    Low,
    High,
}

impl HeightClass {
    /// 문자열로 변환
    pub fn as_str(&self) -> &'static str {
        match self {
            HeightClass::Ground => "ground",
            HeightClass::Low => "low",
            HeightClass::High => "high",
        }
    }
}

/// 커브 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CurveDirection {
    #[default]
    None,
    In,
    Out,
}

impl CurveDirection {
    /// 문자열로 변환
    pub fn as_str(&self) -> &'static str {
        match self {
            CurveDirection::None => "none",
            CurveDirection::In => "in",
            CurveDirection::Out => "out",
        }
    }

    /// curve_factor (-1..1)에서 방향 결정
    pub fn from_factor(factor: f32) -> Self {
        if factor > 0.05 {
            CurveDirection::Out
        } else if factor < -0.05 {
            CurveDirection::In
        } else {
            CurveDirection::None
        }
    }
}

/// 공 궤적 인텐트 (Viewer용 이벤트)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallTrajectoryIntent {
    pub t_ms: u64,
    pub contact_offset_ms: u16,
    pub actor_track_id: u32,
    pub target_track_id: Option<u32>,
    pub kind: BallIntentKind,
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub travel_ms: u32,
    pub speed_class: SpeedClass,
    pub height_class: HeightClass,
    pub curve: CurveDirection,
    pub curve_amount: f32,
    pub outcome: &'static str,
    pub actor_pos: Option<(f32, f32)>,
}

/// 프로젝트 기본 스프라이트 FPS
pub const DEFAULT_SPRITE_FPS: u16 = 10;

impl BallTrajectoryIntent {
    /// 틱을 ms로 변환 (1 tick = 250ms)
    pub fn tick_to_ms(tick: u64) -> u64 {
        tick * 250
    }

    /// 두 점 사이 거리 계산
    pub fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
        ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
    }

    /// travel_ms 계산
    pub fn calculate_travel_ms(
        from: (f32, f32),
        to: (f32, f32),
        kind: BallIntentKind,
        speed_class: SpeedClass,
    ) -> u32 {
        let dist = Self::distance(from, to);
        let v_ref = kind.v_ref();
        let mul = speed_class.multiplier();
        (dist / v_ref * mul * 1000.0) as u32
    }

    /// contact_offset_ms 계산
    #[inline]
    pub fn contact_offset_ms(fps: u16, contact_frame: u16) -> u16 {
        ((1000 * contact_frame as u32 + fps as u32 / 2) / fps as u32) as u16
    }

    /// action_id별 contact_frame 반환
    pub fn contact_frame_for_action(action_id: &str) -> u16 {
        match action_id {
            "pass_short" | "pass_back" | "pass_loft" | "through" | "pass" => 1,
            "cross" | "pass_long" => 2,
            "shot" | "shot_finesse" | "shot_power" | "chip" => 2,
            "header" | "volley" | "onetouch" => 1,
            _ => 1,
        }
    }

    /// BallIntentKind에서 action_id 문자열 반환
    pub fn action_id_from_kind(kind: BallIntentKind) -> &'static str {
        match kind {
            BallIntentKind::Pass => "pass_short",
            BallIntentKind::Through => "through",
            BallIntentKind::Cross => "cross",
            BallIntentKind::Shot => "shot",
            BallIntentKind::Chip => "chip",
            BallIntentKind::Header => "header",
        }
    }

    /// BallIntentKind에서 직접 contact_offset_ms 계산 (기본 FPS 사용)
    #[inline]
    pub fn contact_offset_for_kind(kind: BallIntentKind) -> u16 {
        let action_id = Self::action_id_from_kind(kind);
        let frame = Self::contact_frame_for_action(action_id);
        Self::contact_offset_ms(DEFAULT_SPRITE_FPS, frame)
    }

    /// BallIntentKind에서 직접 contact_offset_ms 계산 (FPS 지정)
    #[inline]
    pub fn contact_offset_for_kind_with_fps(kind: BallIntentKind, fps: u16) -> u16 {
        let action_id = Self::action_id_from_kind(kind);
        let frame = Self::contact_frame_for_action(action_id);
        Self::contact_offset_ms(fps, frame)
    }
}

/// 태클 액션 종류 (Viewer용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TackleActionKind {
    TackleStand,
    TackleSlide,
    TackleShoulder,
}

impl TackleActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TackleActionKind::TackleStand => "tackle_stand",
            TackleActionKind::TackleSlide => "tackle_slide",
            TackleActionKind::TackleShoulder => "tackle_shoulder",
        }
    }

    pub fn from_tackle_type(tackle_type: TackleType) -> Self {
        match tackle_type {
            TackleType::Standing => TackleActionKind::TackleStand,
            TackleType::Sliding => TackleActionKind::TackleSlide,
            TackleType::Shoulder => TackleActionKind::TackleShoulder,
        }
    }
}

/// Viewer용 태클 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewerTackleOutcome {
    Clean,
    Deflect,
    Miss,
    Foul,
    Yellow,
    Red,
}

impl ViewerTackleOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewerTackleOutcome::Clean => "clean",
            ViewerTackleOutcome::Deflect => "deflect",
            ViewerTackleOutcome::Miss => "miss",
            ViewerTackleOutcome::Foul => "foul",
            ViewerTackleOutcome::Yellow => "yellow",
            ViewerTackleOutcome::Red => "red",
        }
    }

    pub fn from_tackle_outcome(outcome: TackleOutcome) -> Self {
        match outcome {
            TackleOutcome::CleanWin => ViewerTackleOutcome::Clean,
            TackleOutcome::Deflection => ViewerTackleOutcome::Deflect,
            TackleOutcome::Miss => ViewerTackleOutcome::Miss,
            TackleOutcome::Foul => ViewerTackleOutcome::Foul,
            TackleOutcome::YellowCard => ViewerTackleOutcome::Yellow,
            TackleOutcome::RedCard => ViewerTackleOutcome::Red,
        }
    }

    pub fn extra_lock_ms(&self) -> u32 {
        match self {
            ViewerTackleOutcome::Clean | ViewerTackleOutcome::Deflect => 350,
            ViewerTackleOutcome::Miss => 625,
            ViewerTackleOutcome::Foul | ViewerTackleOutcome::Yellow | ViewerTackleOutcome::Red => {
                1000
            }
        }
    }
}

/// 태클 이벤트 (Viewer용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TackleEvent {
    pub t_ms: u64,
    pub kind: &'static str,
    pub actor_track_id: u32,
    pub target_track_id: u32,
    pub action: TackleActionKind,
    pub lock_ms: u32,
    pub outcome: ViewerTackleOutcome,
    pub ball_owner_before: Option<u32>,
    pub ball_owner_after: Option<u32>,
    pub contact_pos: Option<(f32, f32)>,
    pub ball_pos: Option<(f32, f32, f32)>,
}

impl TackleEvent {
    pub fn calculate_lock_ms(tackle_type: TackleType, outcome: ViewerTackleOutcome) -> u32 {
        let commit_ticks = tackle_type.commit_ticks() as u32;
        let resolve_ticks = 1u32;
        let base = (commit_ticks + resolve_ticks) * 250;
        let extra = outcome.extra_lock_ms();
        (base + extra).clamp(500, 1400)
    }
}

/// 드리블 터치 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DribbleTouchType {
    Carry,
    KnockOn,
    Shielding,
    Turn,
    Feint,
    Hesitation,
    Evade,
    FirstTouch,
}

impl DribbleTouchType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DribbleTouchType::Carry => "carry",
            DribbleTouchType::KnockOn => "knock_on",
            DribbleTouchType::Shielding => "shielding",
            DribbleTouchType::Turn => "turn",
            DribbleTouchType::Feint => "feint",
            DribbleTouchType::Hesitation => "hesitation",
            DribbleTouchType::Evade => "evade",
            DribbleTouchType::FirstTouch => "first_touch",
        }
    }

    pub fn base_lock_ms(&self) -> u16 {
        match self {
            DribbleTouchType::Carry => 200,
            DribbleTouchType::KnockOn => 100,
            DribbleTouchType::Shielding => 400,
            DribbleTouchType::Turn => 350,
            DribbleTouchType::Feint => 300,
            DribbleTouchType::Hesitation => 250,
            DribbleTouchType::Evade => 200,
            DribbleTouchType::FirstTouch => 150,
        }
    }

    pub fn ball_distance(&self) -> f32 {
        match self {
            DribbleTouchType::Carry => 1.0,
            DribbleTouchType::KnockOn => 4.0,
            DribbleTouchType::Shielding => 0.2,
            DribbleTouchType::Turn => 0.5,
            DribbleTouchType::Feint => 0.8,
            DribbleTouchType::Hesitation => 0.3,
            DribbleTouchType::Evade => 1.2,
            DribbleTouchType::FirstTouch => 0.5,
        }
    }
}

/// 드리블 터치 이벤트 (Viewer용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DribbleTouchEvent {
    pub t_ms: u64,
    pub dribbler_track_id: u32,
    pub touch_type: DribbleTouchType,
    pub ball_from: (f32, f32),
    pub ball_to: (f32, f32),
    pub lock_ms: u16,
    pub player_pos: Option<(f32, f32)>,
    pub defender_track_id: Option<u32>,
    pub direction: Option<(f32, f32)>,
}

impl DribbleTouchEvent {
    pub fn calculate_lock_ms(touch_type: DribbleTouchType, pressure_factor: f32) -> u16 {
        let base = touch_type.base_lock_ms();
        let adjusted = base as f32 * (1.0 - pressure_factor * 0.2);
        adjusted.clamp(100.0, 500.0) as u16
    }
}

/// 돌파 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TakeOnOutcome {
    Success,
    Failed,
    InProgress,
    FoulWon,
}

impl TakeOnOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            TakeOnOutcome::Success => "success",
            TakeOnOutcome::Failed => "failed",
            TakeOnOutcome::InProgress => "in_progress",
            TakeOnOutcome::FoulWon => "foul_won",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, TakeOnOutcome::Success | TakeOnOutcome::FoulWon)
    }
}

/// 돌파 이벤트 (Viewer용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeOnEvent {
    pub t_ms: u64,
    pub dribbler_track_id: u32,
    pub defender_track_id: u32,
    pub technique: DribbleTouchType,
    pub outcome: TakeOnOutcome,
    pub lock_ms: u16,
    pub ball_from: (f32, f32),
    pub ball_to: (f32, f32),
    pub dribbler_pos: (f32, f32),
    pub defender_pos: (f32, f32),
    pub direction: Option<(f32, f32)>,
}

impl TakeOnEvent {
    pub fn calculate_lock_ms(technique: DribbleTouchType, skill_factor: f32) -> u16 {
        let base = technique.base_lock_ms();
        let take_on_base = (base as f32 * 1.5) as u16;
        let adjusted = take_on_base as f32 * (1.0 - skill_factor * 0.2);
        adjusted.clamp(200.0, 700.0) as u16
    }
}

/// Viewer용 통합 이벤트
#[derive(Debug, Clone)]
pub enum ViewerEvent {
    BallTrajectory(BallTrajectoryIntent),
    Tackle(TackleEvent),
    DribbleTouch(DribbleTouchEvent),
    TakeOn(TakeOnEvent),
}

impl ViewerEvent {
    pub fn t_ms(&self) -> u64 {
        match self {
            ViewerEvent::BallTrajectory(e) => e.t_ms,
            ViewerEvent::Tackle(e) => e.t_ms,
            ViewerEvent::DribbleTouch(e) => e.t_ms,
            ViewerEvent::TakeOn(e) => e.t_ms,
        }
    }

    pub fn actor_track_id(&self) -> u32 {
        match self {
            ViewerEvent::BallTrajectory(e) => e.actor_track_id,
            ViewerEvent::Tackle(e) => e.actor_track_id,
            ViewerEvent::DribbleTouch(e) => e.dribbler_track_id,
            ViewerEvent::TakeOn(e) => e.dribbler_track_id,
        }
    }
}

// ============================================================================
// ActionQueue System (Original action_queue.rs content continues)
// ============================================================================

/// 액션 타입
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionType {
    /// 트래핑 - 날아오는 공을 받음
    Trap {
        /// 공 속도 (m/s)
        ball_speed: f32,
        /// 공 높이 (미터)
        ball_height: f32,
    },

    /// 패스
    Pass {
        /// 패스 받을 선수 인덱스
        target_idx: usize,
        /// 롱패스 여부
        is_long: bool,
        /// 스루패스 여부
        is_through: bool,
        /// FIX_2601/1128: 선택 시점의 타겟 위치 (forward_pass_rate 측정용)
        intended_target_pos: Option<Coord10>,
        /// FIX_2601/1129: 선택 시점의 패서 위치 (forward_pass_rate 측정용)
        intended_passer_pos: Option<Coord10>,
    },

    /// 드리블 (direction은 정규화된 벡터, 튜플 유지)
    Dribble {
        /// 드리블 방향 (정규화된 벡터)
        direction: (f32, f32),
        /// 공격적 드리블 (돌파 시도)
        aggressive: bool,
    },

    /// 슛 (FIX_2601: Coord10)
    Shot {
        /// 슛 파워 (0.0 ~ 1.0)
        power: f32,
        /// 목표 위치 (골대 내 좌표)
        target: Coord10,
    },

    /// 태클
    Tackle {
        /// 태클 대상 선수 인덱스
        target_idx: usize,
    },

    /// 인터셉트 시도 (FIX_2601: Coord10)
    Intercept {
        /// 예상 공 위치
        ball_position: Coord10,
    },

    /// 헤딩 (FIX_2601: Coord10)
    Header {
        /// 헤딩 방향/목표
        target: Coord10,
        /// 슛인지 패스인지
        is_shot: bool,
    },

    /// 이동 (Off-the-Ball) (FIX_2601: Coord10)
    Move {
        /// 목표 위치
        target: Coord10,
        /// 스프린트 여부
        sprint: bool,
    },

    /// 골키퍼 세이브 (direction은 방향 벡터, 튜플 유지)
    Save {
        /// 다이브 방향
        direction: (f32, f32),
    },
}

impl ActionType {
    /// 이 액션이 공을 필요로 하는지
    pub fn requires_ball(&self) -> bool {
        matches!(
            self,
            ActionType::Pass { .. }
                | ActionType::Dribble { .. }
                | ActionType::Shot { .. }
                | ActionType::Header { is_shot: true, .. }
                | ActionType::Save { .. }
        )
    }

    /// 이 액션이 공 상태를 변경하는지
    pub fn changes_ball_state(&self) -> bool {
        matches!(
            self,
            ActionType::Pass { .. }
                | ActionType::Shot { .. }
                | ActionType::Trap { .. }
                | ActionType::Intercept { .. }
                | ActionType::Header { .. }
                | ActionType::Save { .. }
                | ActionType::Tackle { .. }
        )
    }

    /// 기본 실행 시간 (틱)
    pub fn base_duration(&self) -> u64 {
        match self {
            ActionType::Trap { .. } => 3, // 0.3초
            ActionType::Pass { is_long, .. } => {
                if *is_long {
                    2
                } else {
                    1
                }
            }
            ActionType::Dribble { .. } => 5,   // 0.5초
            ActionType::Shot { .. } => 3,      // 0.3초
            ActionType::Tackle { .. } => 4,    // 0.4초
            ActionType::Intercept { .. } => 2, // 0.2초
            ActionType::Header { .. } => 2,    // 0.2초
            ActionType::Move { sprint, .. } => {
                if *sprint {
                    3
                } else {
                    5
                }
            }
            ActionType::Save { .. } => 3, // 0.3초
        }
    }
}

// ============================================================================
// FIX_2601/0123 PR #2-1: Deterministic Tie-Breaker
// ============================================================================

/// Deterministic tie-breaker for action scheduling.
///
/// When two actions have the same tick and priority, this function
/// determines which player goes first using a hash-based approach.
///
/// # Why not `tick % 2`?
/// The previous `tick % 2` approach caused bias when:
/// - Ticks reset at halftime (both halves start with even tick)
/// - Consecutive ticks favor the same team systematically
///
/// # This approach
/// Uses FxHash of (tick, min_idx, max_idx) to produce unbiased distribution:
/// - Deterministic: same inputs → same output
/// - Symmetric: f(t, a, b) == f(t, b, a) for consistent comparison
/// - Unbiased: ~50% home-first over many ticks
///
/// # Returns
/// `true` if lower player_idx should go first, `false` otherwise
#[inline]
fn tiebreak_lower_idx_first(tick: u64, idx_a: usize, idx_b: usize) -> bool {
    let mut hasher = FxHasher::default();
    tick.hash(&mut hasher);
    // Use min/max to ensure symmetric hashing regardless of comparison order
    idx_a.min(idx_b).hash(&mut hasher);
    idx_a.max(idx_b).hash(&mut hasher);
    hasher.finish() % 2 == 0
}

/// 예약된 액션
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledAction {
    /// 실행될 틱
    pub execute_tick: u64,
    /// 액션 타입
    pub action_type: ActionType,
    /// 실행 주체 선수 인덱스
    pub player_idx: usize,
    /// 액션 우선순위 (높을수록 먼저 실행)
    pub priority: u8,
    /// 액션 ID (취소/추적용)
    pub action_id: u64,
    /// P16: 액션 세부 파라미터 (확장 포인트)
    /// 실행부에서 detail.pass_type, detail.shot_type 등으로 미세 분기
    #[serde(default)]
    pub detail: ActionDetail,
}

impl ScheduledAction {
    /// 기존 호환용 생성자 (detail = default)
    pub fn new(
        execute_tick: u64,
        action_type: ActionType,
        player_idx: usize,
        priority: u8,
        action_id: u64,
    ) -> Self {
        Self {
            execute_tick,
            action_type,
            player_idx,
            priority,
            action_id,
            detail: ActionDetail::default(),
        }
    }

    /// P16: ActionDetail 포함 생성자
    pub fn new_with_detail(
        execute_tick: u64,
        action_type: ActionType,
        player_idx: usize,
        priority: u8,
        action_id: u64,
        detail: ActionDetail,
    ) -> Self {
        Self { execute_tick, action_type, player_idx, priority, action_id, detail }
    }
}

// BinaryHeap을 위한 Ord 구현 (틱 오름차순, 우선순위 내림차순)
impl Eq for ScheduledAction {}

impl PartialOrd for ScheduledAction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledAction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 먼저 틱으로 정렬 (작은 틱 먼저)
        match self.execute_tick.cmp(&other.execute_tick) {
            std::cmp::Ordering::Equal => {
                // 같은 틱이면 우선순위로 (높은 우선순위 먼저)
                match other.priority.cmp(&self.priority) {
                    std::cmp::Ordering::Equal => {
                        // FIX_2601/0123 PR #2-1: Hash-based fair tie-breaker
                        // When tick and priority are equal, use hash-based selection
                        // to prevent systematic slot bias (0-10 vs 11-21).
                        //
                        // Previous `tick % 2` caused halftime bias.
                        // New approach: hash(tick, idx_a, idx_b) for unbiased distribution.
                        if tiebreak_lower_idx_first(
                            self.execute_tick,
                            self.player_idx,
                            other.player_idx,
                        ) {
                            // Lower player_idx wins (goes first in max-heap)
                            other.player_idx.cmp(&self.player_idx)
                        } else {
                            // Higher player_idx wins
                            self.player_idx.cmp(&other.player_idx)
                        }
                    }
                    ord => ord,
                }
            }
            other => other,
        }
    }
}

/// 공 상태
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BallState {
    /// 선수가 공을 소유 중
    Controlled {
        /// 소유 선수 인덱스
        owner_idx: usize,
    },

    /// 공이 비행 중
    InFlight {
        /// 출발 위치 (FIX_2601: Coord10)
        from_pos: Coord10,
        /// 도착 예정 위치 (FIX_2601: Coord10)
        to_pos: Coord10,
        /// 비행 시작 틱
        start_tick: u64,
        /// 도착 예정 틱
        end_tick: u64,
        /// 높이 프로파일
        height_profile: HeightProfile,
        /// 높이 비율 (0.0 ~ 1.0)
        lift_ratio: f32,
        /// 의도된 수신자 (있는 경우)
        intended_receiver: Option<usize>,
        /// 패스인지 슛인지
        is_shot: bool,
        /// vNext: 출발 높이 (0.1m 단위, 크로스바 리바운드용)
        #[serde(default)]
        start_height_01m: i16,
        /// vNext: 도착 높이 (0.1m 단위, 크로스바 리바운드용)
        #[serde(default)]
        end_height_01m: i16,
    },

    /// 루즈볼 (소유자 없음)
    Loose {
        /// 현재 위치 (FIX_2601: Coord10)
        position: Coord10,
        /// 속도 벡터 (FIX_2601: Vel10)
        velocity: Vel10,
    },

    /// 아웃 오브 플레이
    OutOfPlay {
        /// 재개 타입 (ThrowIn, GoalKick, Corner, etc.)
        restart_type: RestartType,
        /// 재개 위치 (FIX_2601: Coord10)
        position: Coord10,
        /// 재개 팀 (true = 홈)
        home_team: bool,
    },
}

impl Default for BallState {
    fn default() -> Self {
        BallState::Loose {
            position: Coord10::CENTER, // 센터 서클
            velocity: Vel10::default(),
        }
    }
}

impl BallState {
    /// 현재 공 소유자 반환
    pub fn owner(&self) -> Option<usize> {
        match self {
            BallState::Controlled { owner_idx } => Some(*owner_idx),
            _ => None,
        }
    }

    /// 공이 플레이 중인지
    pub fn is_in_play(&self) -> bool {
        !matches!(self, BallState::OutOfPlay { .. })
    }

    /// 공이 자유 상태인지 (인터셉트 가능)
    pub fn is_contestable(&self) -> bool {
        matches!(self, BallState::InFlight { .. } | BallState::Loose { .. })
    }
}

/// 경기 재개 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartType {
    KickOff,
    ThrowIn,
    GoalKick,
    Corner,
    FreeKick,
    Penalty,
    DropBall,
}

/// 액션 실행 결과
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionResult {
    /// 패스 시작됨
    /// FIX_2601/1128: intended_target_pos 추가 (선택 시점 타겟 위치)
    /// FIX_2601/1129: intended_passer_pos 추가 (선택 시점 패서 위치)
    PassStarted {
        passer_idx: usize,
        receiver_idx: usize,
        arrival_tick: u64,
        ball_speed: f32,
        intended_target_pos: Option<Coord10>,
        intended_passer_pos: Option<Coord10>,
    },

    /// 트랩 성공
    TrapSuccess { player_idx: usize },

    /// 트랩 실패 (공이 튀어나감) (FIX_2601: Coord10)
    TrapFailed { player_idx: usize, loose_ball_pos: Coord10 },

    /// 운반 완료 (Carry - 수비수 없는 공간으로 공 운반) (FIX_2601: Coord10)
    /// 통계에 기록하지 않음 (단순 이동)
    CarryComplete { player_idx: usize, new_position: Coord10 },

    /// 돌파 완료 (Take-on - 수비수를 제치고 드리블 성공) (FIX_2601: Coord10)
    /// '드리블' 통계에 기록됨
    TakeOnComplete { player_idx: usize, new_position: Coord10, beaten_defender_idx: Option<usize> },

    /// 드리블 중 태클당함 (Take-on 실패)
    DribbleTackled { player_idx: usize, tackler_idx: usize },

    /// 슛 시도 (FIX_2601: Coord10)
    ShotTaken { shooter_idx: usize, target: Coord10, xg: f32 },

    /// 골! (FIX_2601/0115b: xG 포함)
    GoalScored { scorer_idx: usize, assist_idx: Option<usize>, xg: f32 },

    /// 세이브 (FIX_2601/0115b: xG 포함)
    SaveMade { goalkeeper_idx: usize, save_type: SaveType, shooter_idx: usize, xg: f32 },

    /// 골키퍼 핸들링 위반 (핸드볼)
    ///
    /// - `is_indirect=false`: 페널티 에어리어 밖에서 손으로 처리(직접 프리킥)
    /// - `is_indirect=true`: 백패스/스로인 등으로 인한 핸들링(간접 프리킥, TODO)
    ///
    /// NOTE: 실제 재개 위치/팀은 `BallState::OutOfPlay`에 의해 결정된다.
    GoalkeeperHandlingViolation {
        goalkeeper_idx: usize,
        /// 마지막으로 공을 터치한 선수(예: 슈터/패서). 없으면 None.
        last_touch_idx: Option<usize>,
        is_indirect: bool,
        /// 슛 상황이면 xG 포함 (없으면 None)
        xg: Option<f32>,
    },

    /// (Optional) Non-GK handball foul.
    ///
    /// NOTE: Restart team/position/type is driven by `BallState::OutOfPlay` (SSOT).
    HandballFoul {
        offender_idx: usize,
        /// Last touch (attacker) for event linkage; may be None.
        last_touch_idx: Option<usize>,
    },

    /// 슛 빗나감 (오프타겟) (FIX_2601/0115b: xG 포함)
    ShotMissed { shooter_idx: usize, xg: f32 },

    /// 인터셉트 성공
    InterceptSuccess { player_idx: usize },

    /// 헤딩 (direction은 단위 벡터, 튜플 유지)
    HeaderWon { player_idx: usize, direction: (f32, f32) },

    /// 이동 완료 (FIX_2601: Coord10)
    MoveComplete { player_idx: usize, new_position: Coord10 },

    /// 태클 성공
    TackleSuccess { tackler_idx: usize, target_idx: usize },

    /// 태클 실패 (파울)
    TackleFoul { tackler_idx: usize, target_idx: usize },

    /// 태클 파울이지만 어드밴티지 적용(플레이 온)
    /// - 파울은 기록되나, 프리킥 재개로 끊지 않는다.
    TackleFoulAdvantage { tackler_idx: usize, target_idx: usize },

    /// 아웃 오브 바운즈 (FIX_2601: Coord10)
    OutOfBounds { restart_type: RestartType, position: Coord10, home_team: bool },

    /// 액션 취소됨 (인터럽트)
    Cancelled { action_id: u64, reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeaderOutcome {
    pub player_idx: usize,
    pub is_shot: bool,
    pub success: bool,
}

/// 세이브 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveType {
    Catch,
    Parry,
    Punch,
    Dive,
}

// ============================================================================
// Interrupt System - Phase 3.3
// ============================================================================

/// 액션 인터럽트 이유
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InterruptReason {
    /// 물리 충돌
    PhysicsCollision {
        /// 충돌한 객체 인덱스
        object_idx: usize,
        /// 충돌 힘 (0.0 ~ 1.0)
        impact_force: f32,
    },

    /// 파울 (FIX_2601: Coord10)
    Foul {
        /// 파울을 범한 선수
        offender_idx: usize,
        /// 파울 당한 선수
        victim_idx: usize,
        /// 파울 위치
        position: Coord10,
        /// 심각도 (옐로카드 가능성)
        severity: f32,
    },

    /// 아웃 오브 바운즈 (FIX_2601: Coord10)
    OutOfBounds {
        /// 아웃 위치
        position: Coord10,
        /// 마지막으로 터치한 팀 (true = 홈)
        last_touch_home: bool,
    },

    /// 골 득점
    GoalScored {
        /// 득점 팀 (true = 홈)
        home_team: bool,
        /// 득점자 인덱스
        scorer_idx: usize,
    },

    /// 오프사이드 (FIX_2601: Coord10)
    Offside {
        /// 오프사이드 선수
        player_idx: usize,
        /// 오프사이드 위치
        position: Coord10,
    },

    /// 볼 탈취 (태클 성공 등)
    BallWon {
        /// 공을 탈취한 선수
        winner_idx: usize,
        /// 공을 잃은 선수
        loser_idx: usize,
    },
}

impl InterruptReason {
    /// 경기 재개가 필요한 인터럽트인지
    pub fn requires_restart(&self) -> bool {
        matches!(
            self,
            InterruptReason::Foul { .. }
                | InterruptReason::OutOfBounds { .. }
                | InterruptReason::GoalScored { .. }
                | InterruptReason::Offside { .. }
        )
    }

    /// 이 인터럽트로 인한 재개 타입
    pub fn restart_type(&self) -> Option<RestartType> {
        match self {
            InterruptReason::Foul { position, .. } => {
                // 페널티 박스 내 파울 체크 (간단한 로직)
                // FIX_2601: Coord10 사용 (0.1m 단위, 885 = 88.5m, 165 = 16.5m)
                let (x, y) = position.to_meters();
                if !(16.5..=88.5).contains(&x) {
                    let in_box_y = y > 13.85 && y < 54.15;
                    if in_box_y {
                        return Some(RestartType::Penalty);
                    }
                }
                Some(RestartType::FreeKick)
            }
            InterruptReason::OutOfBounds { position, .. } => {
                // 골라인 vs 터치라인 판정
                // FIX_2601: Coord10 사용
                let (x, y) = position.to_meters();
                if y <= 0.0 || y >= field::WIDTH_M {
                    Some(RestartType::ThrowIn)
                } else if x <= 0.0 {
                    // 골라인 (홈 골대)
                    Some(RestartType::GoalKick) // 실제로는 코너/골킥 구분 필요
                } else {
                    // 골라인 (어웨이 골대)
                    Some(RestartType::GoalKick)
                }
            }
            InterruptReason::GoalScored { .. } => Some(RestartType::KickOff),
            InterruptReason::Offside { .. } => Some(RestartType::FreeKick),
            _ => None,
        }
    }
}

/// 루즈볼 경합 결과
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LooseBallContest {
    /// 경합 위치
    pub position: (f32, f32),
    /// 경합에 참여한 선수들 (인덱스, 거리)
    pub contestants: Vec<(usize, f32)>,
    /// 승자 (있으면)
    pub winner: Option<usize>,
    /// 50:50 여부
    pub is_fifty_fifty: bool,
}

/// 액션 큐
///
/// P7: Phase-Based Action Engine
/// - `pending`: 예약된 액션 (아직 시작 안 함)
/// - `active`: 실행 중인 FSM 액션
#[derive(Debug, Clone, Default)]
pub struct ActionQueue {
    /// 예약된 액션 (우선순위 큐) - pending
    pending: BinaryHeap<Reverse<ScheduledAction>>,
    /// 실행 중인 FSM 액션 - active (P7)
    active: Vec<ActiveAction>,
    /// 현재 공 상태
    ball_state: BallState,
    /// 이번 틱에 실행된/완료된 액션들
    pub tick_results: Vec<ActionResult>,
    /// 다음 액션 ID
    next_action_id: u64,
    /// 현재 틱
    current_tick: u64,
    /// 마지막 슛의 xG (Save 액션에서 사용)
    pub last_shot_xg: Option<f32>,
    /// 마지막 슈터 인덱스 (Save 액션에서 사용)
    pub last_shooter_idx: Option<usize>,

    // ========== FIX_2601/0102: Assist Candidate System ==========
    /// 마지막 패스한 선수 인덱스 (어시스트 추적용)
    /// PassStarted 시 설정, TrapSuccess 시 사용 후 초기화
    pub last_passer_idx: Option<usize>,
    /// 마지막 패스 타겟 (패스 성공 판정용)
    pub last_pass_receiver_idx: Option<usize>,
    /// 마지막 패스 타입 (크로스/스루 통계용)
    pub last_pass_type: Option<PassType>,
    /// 마지막 헤딩 결과 (헤딩 통계용)
    pub last_header_outcome: Option<HeaderOutcome>,
    /// In-flight origin marker (set-piece deliveries, etc.).
    pub in_flight_origin: Option<InFlightOrigin>,
}

impl ActionQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// 초기 공 상태 설정
    pub fn with_ball_state(ball_state: BallState) -> Self {
        Self { ball_state, ..Self::default() }
    }

    /// 현재 공 상태
    pub fn ball_state(&self) -> &BallState {
        &self.ball_state
    }

    /// 공 상태 직접 설정 (세트피스 등)
    pub fn set_ball_state(&mut self, state: BallState) {
        self.ball_state = state;
    }

    /// 현재 틱
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn last_pass_type(&self) -> Option<PassType> {
        self.last_pass_type
    }

    pub fn clear_last_pass_type(&mut self) {
        self.last_pass_type = None;
    }

    pub fn take_last_header_outcome(&mut self) -> Option<HeaderOutcome> {       
        self.last_header_outcome.take()
    }

    pub fn set_in_flight_origin(&mut self, origin: InFlightOrigin) {
        self.in_flight_origin = Some(origin);
    }

    pub fn clear_in_flight_origin(&mut self) {
        self.in_flight_origin = None;
    }

    // ========== Phase 3.5: Ball ↔ BallState 동기화 ==========

    /// Ball struct에서 BallState로 동기화
    ///
    /// 기존 Ball struct의 상태를 ActionQueue의 BallState로 변환합니다.
    /// FIX_2601: Ball과 BallState 모두 Coord10/Vel10 사용하므로 직접 복사
    pub fn sync_from_ball(&mut self, tick: u64, ball: &super::Ball) {
        // SSOT: caller passes sim tick explicitly (prevents stale-tick drift).
        self.current_tick = tick;
        if ball.is_in_flight {
            // Preserve the existing InFlight segment (Ball lacks full metadata like `is_shot`).
            // Arrival conversion is handled post-actions (see `advance_ball_state_post_actions`).
            if matches!(self.ball_state, BallState::InFlight { .. }) {
                return;
            }
            if let BallState::InFlight {
                end_tick,
                to_pos,
                height_profile,
                lift_ratio,
                ..
            } = self.ball_state
            {
                // 비행 종료 시점 도달 → 루즈볼로 전환 (낙하지점 경합 시작)
                if ball.flight_progress >= 1.0 || self.current_tick >= end_tick {
                    let max_height = max_height_from_profile(height_profile, lift_ratio);
                    let is_aerial = !matches!(height_profile, HeightProfile::Flat) && max_height >= 1.0;
                    if action_debug_enabled() && is_aerial {
                        let to_pos_m = to_pos.to_meters();
                        let reason = if ball.flight_progress >= 1.0 {
                            "progress"
                        } else {
                            "end_tick"
                        };
                        eprintln!(
                            "[AERIAL L6] tick={} end_tick={} reason={} progress={:.2} to=({:.1},{:.1}) max_height={:.2}",
                            self.current_tick,
                            end_tick,
                            reason,
                            ball.flight_progress,
                            to_pos_m.0,
                            to_pos_m.1,
                            max_height
                        );
                    }
                    self.ball_state = BallState::Loose {
                        position: to_pos,
                        velocity: Vel10::default(),
                    };
                }
                return;
            }
        }
        self.ball_state = if let Some(owner_idx) = ball.current_owner {
            BallState::Controlled { owner_idx }
        } else if ball.is_in_flight {
            // 비행 중인 경우 - FIX_2601: Coord10 직접 사용
            let from_pos = ball.from_position.unwrap_or(ball.position);
            let to_pos = ball.to_position.unwrap_or(ball.position);

            // 비행 시간 추정 (속도 기반) - FIX_2601: Coord10 거리 계산
            let distance = from_pos.distance_to(&to_pos) as f32 / 10.0; // 0.1m → m
            let flight_duration = if ball.flight_speed > 0.0 {
                (distance / ball.flight_speed * TICKS_PER_SECOND as f32) as u64
            } else {
                TICKS_PER_SECOND
            };

            // 현재 진행률로 시작/종료 틱 계산
            let elapsed = (ball.flight_progress * flight_duration as f32) as u64;
            let start_tick = self.current_tick.saturating_sub(elapsed);
            let end_tick = start_tick + flight_duration;

            let height_profile = ball.height_profile;
            let vz_cap = height_profile.vz_cap_mps();
            let vz = ball.velocity_z as f32 * 0.1;
            let lift_ratio = if vz_cap > 0.0 { (vz / vz_cap).clamp(0.0, 1.0) } else { 0.0 };

            BallState::InFlight {
                from_pos,
                to_pos,
                start_tick,
                end_tick,
                height_profile,
                lift_ratio,
                intended_receiver: ball.pending_owner,
                is_shot: false, // Ball struct에서는 알 수 없음
                // vNext: Copy height fields from Ball to prevent drift
                start_height_01m: ball.flight_start_height_01m,
                end_height_01m: ball.flight_end_height_01m,
            }
        } else {
            // 루즈볼 - FIX_2601: Coord10/Vel10 직접 사용
            BallState::Loose { position: ball.position, velocity: ball.velocity }
        };
    }

    /// Advance ball state after all actions for this tick have resolved.
    ///
    /// Contract ordering invariant:
    /// - Actions scheduled for `arrival_tick` (Trap/Header) must see the ball as `InFlight`
    ///   during that tick’s resolution.
    /// - Therefore the `InFlight → Loose` arrival conversion happens **after** actions,
    ///   not in the pre-action sync stage.
    ///
    /// Related ordering rule (woodwork):
    /// - If an `InFlight` segment hits the post/crossbar at `arrival_tick`, any
    ///   ball-dependent actions scheduled for that tick (Trap/Save/etc) must be cancelled
    ///   before execution. This is handled by `resolve_in_flight_woodwork_pre_actions`,
    ///   which must run **before** action execution.
    pub fn advance_ball_state_post_actions(&mut self, tick: u64) {
        self.current_tick = tick;

        let (end_tick, to_pos, height_profile, lift_ratio) = match self.ball_state {
            BallState::InFlight {
                end_tick,
                to_pos,
                height_profile,
                lift_ratio,
                ..
            } => (end_tick, to_pos, height_profile, lift_ratio),
            _ => return,
        };

        if tick < end_tick {
            return;
        }

        let max_height = max_height_from_profile(height_profile, lift_ratio);
        let is_aerial =
            !matches!(height_profile, HeightProfile::Flat) && max_height >= 1.0;
        if action_debug_enabled() && is_aerial {
            let to_pos_m = to_pos.to_meters();
            eprintln!(
                "[AERIAL L6] tick={} end_tick={} reason=end_tick to=({:.1},{:.1}) max_height={:.2}",
                tick,
                end_tick,
                to_pos_m.0,
                to_pos_m.1,
                max_height
            );
        }

        self.ball_state = BallState::Loose {
            position: to_pos,
            velocity: Vel10::default(),
        };
    }

    /// Resolve woodwork collisions for an `InFlight` segment **before** due actions run.
    ///
    /// v1 scope (minimal, deterministic):
    /// - Detects contact with the goal-line plane (`x=0` or `x=105`) near the posts.
    /// - Converts the ball to `Loose` with a reflected x-velocity (restitution + bounce threshold).
    /// - Cancels all pending ball-dependent actions (Trap/Header/Save/etc) so we don't
    ///   execute "arrival" actions after the flight path has changed.
    ///
    /// Notes:
    /// - `BallState::InFlight` currently has no `curve_factor`; we evaluate a linear segment
    ///   for collision purposes (good enough for v1, deterministic).
    pub fn resolve_in_flight_woodwork_pre_actions(
        &mut self,
        tick: u64,
        params: crate::engine::ball_physics_params::BallPhysicsParams,
    ) {
        self.current_tick = tick;

        let BallState::InFlight { start_tick, .. } = self.ball_state else {
            return;
        };

        // Need a segment to test: previous tick -> current tick.
        let prev_tick = tick.saturating_sub(1);
        if prev_tick == tick || tick < start_tick {
            return;
        }

        let Some((x0, y0, z0)) = in_flight_ball_pos_at_tick(&self.ball_state, prev_tick) else {
            return;
        };
        let Some((x1, y1, z1)) = in_flight_ball_pos_at_tick(&self.ball_state, tick) else {
            return;
        };

        let dx = x1 - x0;
        if dx.abs() < 1e-6 {
            return;
        }

        // Average velocity over the decision-tick slice (m/s).
        let vx = dx / crate::engine::ball_physics_params::DECISION_DT;
        let vy = (y1 - y0) / crate::engine::ball_physics_params::DECISION_DT;

        let tol = params.woodwork_tolerance_m;

        // Find the earliest woodwork intersection among both goal-line planes.
        let mut best_hit: Option<(f32, f32, f32, f32, bool)> = None; // (t_hit, goal_x, y, z, is_crossbar)
        for goal_x in [goal::LEFT_X, goal::RIGHT_X] {
            // Only consider segments that approach the goal-line plane from inside the pitch.
            // This prevents re-triggering on the next tick after a bounce (where x0==goal_x).
            let approaches_plane = if (goal_x - goal::RIGHT_X).abs() < 1e-6 {
                x0 < goal_x && x1 >= goal_x
            } else {
                x0 > goal_x && x1 <= goal_x
            };
            if !approaches_plane {
                continue;
            }

            let t_hit = (goal_x - x0) / dx;
            if !(0.0..=1.0).contains(&t_hit) {
                continue;
            }

            let y_hit = y0 + t_hit * (y1 - y0);
            let z_hit = z0 + t_hit * (z1 - z0);

            // Goal geometry: posts + crossbar detection
            // Check if ball is within goal width (between posts)
            let within_goal_width = y_hit >= goal::Y_MIN - tol && y_hit <= goal::Y_MAX + tol;

            // Post detection: near the post edges
            let near_post_y =
                (y_hit - goal::Y_MIN).abs() <= tol || (y_hit - goal::Y_MAX).abs() <= tol;

            // vNext: Crossbar detection - ball near crossbar height and within goal width
            let near_crossbar = within_goal_width
                && !near_post_y
                && z_hit >= goal::HEIGHT_M - tol
                && z_hit <= goal::HEIGHT_M + tol;

            // Above the crossbar (outside goal frame): no collision
            if z_hit > goal::HEIGHT_M + tol {
                continue;
            }

            // Below crossbar and not near posts: ball goes in/out (no woodwork)
            if !near_post_y && !near_crossbar {
                continue;
            }

            // Record the collision type for later processing
            let is_crossbar = near_crossbar;
            if best_hit.map_or(true, |(best_t, _, _, _, _)| t_hit < best_t) {
                best_hit = Some((t_hit, goal_x, y_hit, z_hit, is_crossbar));
            }
        }

        let Some((_t_hit, goal_x, y_hit, z_hit, is_crossbar)) = best_hit else {
            return;
        };

        // Collision response
        let vn = vx;
        let vx_out = if vn.abs() < params.woodwork_bounce_speed_threshold_mps {
            0.0
        } else {
            -params.woodwork_restitution * vn
        };

        if is_crossbar {
            // vNext: Crossbar rebound - create short InFlight segment preserving height
            // Ball bounces back from crossbar height (~2.44m) to ground
            let rebound_speed = (vx_out.powi(2) + vy.powi(2)).sqrt();

            // Estimate landing position based on velocity
            let rebound_distance = rebound_speed * 0.5; // ~0.5 seconds of flight
            let landing_x = goal_x + vx_out.signum() * rebound_distance.min(15.0);
            let landing_y = y_hit + vy * 0.5;

            // Flight duration: short rebound (2-4 decision ticks)
            let flight_ticks = if rebound_speed > 10.0 { 4 } else { 2 };

            self.ball_state = BallState::InFlight {
                from_pos: Coord10::from_meters(goal_x, y_hit),
                to_pos: Coord10::from_meters(landing_x.clamp(5.0, 100.0), landing_y.clamp(5.0, 63.0)),
                start_tick: tick,
                end_tick: tick + flight_ticks,
                height_profile: HeightProfile::Arc, // Minimal arc on descent
                lift_ratio: 0.0, // No additional height (pure descent)
                intended_receiver: None, // Contested loose ball after landing
                is_shot: false,
                start_height_01m: (z_hit * 10.0).round() as i16, // Crossbar height ~24
                end_height_01m: 0, // Land on ground
            };
        } else {
            // Post collision: convert to Loose (existing v1 behavior)
            self.ball_state = BallState::Loose {
                position: Coord10::from_meters(goal_x, y_hit),
                velocity: Vel10::from_mps(vx_out, vy),
            };
        }

        // Cancel all ball-dependent actions so we don't execute "arrival" actions after the bounce.
        let cancelled = self.cancel_ball_actions();
        for action in cancelled {
            self.tick_results.push(ActionResult::Cancelled {
                action_id: action.action_id,
                reason: "woodwork".to_string(),
            });
        }

        #[cfg(debug_assertions)]
        if action_debug_enabled() {
            let hit_type = if is_crossbar { "CROSSBAR" } else { "POST" };
            eprintln!(
                "[WOODWORK] tick={} type={} goal_x={:.1} y={:.2} z={:.2} v_in=({:.2},{:.2}) v_out=({:.2},{:.2})",
                tick,
                hit_type,
                goal_x,
                y_hit,
                z_hit,
                vx,
                vy,
                vx_out,
                vy
            );
        }
    }

    /// Advance loose-ball physics (decision tick integration + ground roll damping).
    ///
    /// Contract:
    /// - Runs in sim tick SSOT (after actions, before `sync_to_ball`).
    /// - Does not assign restarts; out-of-play detection remains in `tick_based.rs` for now.
    pub fn advance_loose_ball_physics(
        &mut self,
        tick: u64,
        params: crate::engine::ball_physics_params::BallPhysicsParams,
    ) {
        self.current_tick = tick;

        let (position, velocity) = match &mut self.ball_state {
            BallState::Loose { position, velocity } => (position, velocity),
            _ => return,
        };

        if velocity.vx == 0 && velocity.vy == 0 {
            return;
        }

        // Integrate position over the decision tick (250ms).
        *position = *position + velocity.to_delta_per_tick_250ms();

        // Apply dt-invariant roll damping to velocity.
        let m = params.roll_multiplier(crate::engine::ball_physics_params::DECISION_DT);
        velocity.vx = (velocity.vx as f32 * m).trunc() as i32;
        velocity.vy = (velocity.vy as f32 * m).trunc() as i32;

        // Stop threshold to prevent micro-jitter tails.
        let speed = velocity.magnitude();
        if speed < params.stop_speed_vel10() as i32 {
            *velocity = Vel10::default();
        }
    }

    /// BallState를 Ball struct로 동기화
    ///
    /// ActionQueue의 BallState를 기존 Ball struct로 변환합니다.
    /// FIX_2601: BallState와 Ball 모두 Coord10/Vel10 사용하므로 직접 복사
    pub fn sync_to_ball(&self, ball: &mut super::Ball) {
        match &self.ball_state {
            BallState::Controlled { owner_idx } => {
                if ball.current_owner.is_some() {
                    ball.previous_owner = ball.current_owner;
                }
                ball.current_owner = Some(*owner_idx);
                ball.is_in_flight = false;
                ball.pending_owner = None;
                ball.from_position = None;
                ball.to_position = None;
                ball.flight_progress = 0.0;
                ball.velocity_z = 0;
                ball.height = 0; // 지면 (2025-12-11) - FIX_2512: i16
                ball.height_profile = HeightProfile::Flat;
            }
            BallState::InFlight {
                from_pos,
                to_pos,
                start_tick,
                end_tick,
                height_profile,
                lift_ratio,
                intended_receiver,
                is_shot: _,
                start_height_01m,
                end_height_01m,
            } => {
                // FIX_2601: Coord10 직접 사용
                ball.from_position = Some(*from_pos);
                ball.to_position = Some(*to_pos);
                ball.is_in_flight = true;
                if ball.current_owner.is_some() {
                    ball.previous_owner = ball.current_owner;
                }
                ball.current_owner = None;
                ball.pending_owner = *intended_receiver;

                // 비행 진행률 계산
                let total_duration = end_tick.saturating_sub(*start_tick);
                let elapsed = self.current_tick.saturating_sub(*start_tick);
                ball.flight_progress = if total_duration > 0 {
                    (elapsed as f32 / total_duration as f32).min(1.0)
                } else {
                    1.0
                };

                ball.height_profile = *height_profile;
                ball.launch_with_ratio(*height_profile, *lift_ratio);

                // vNext: Copy height fields to Ball (for sync_from_ball round-trip)
                ball.flight_start_height_01m = *start_height_01m;
                ball.flight_end_height_01m = *end_height_01m;

                // 3D 위치 계산 (x, y, z) - height 포함
                // vNext: Use endpoint-aware height calculation for crossbar rebounds
                let t = ball.flight_progress;
                let bump_height = max_height_from_profile(*height_profile, *lift_ratio);
                let start_height_m = *start_height_01m as f32 * 0.1;
                let end_height_m = *end_height_01m as f32 * 0.1;
                let (x, y, z) = get_ball_position_3d_with_endpoints(
                    from_pos.to_meters(),
                    to_pos.to_meters(),
                    ball.curve_factor,
                    bump_height,
                    start_height_m,
                    end_height_m,
                    t,
                );
                ball.position = Coord10::from_meters(x, y);
                ball.height = (z * 10.0) as i16; // FIX_2512: meters → 0.1m units
            }
            BallState::Loose { position, velocity } => {
                // FIX_2601: Coord10/Vel10 직접 복사
                ball.position = *position;
                ball.velocity = *velocity;
                ball.is_in_flight = false;
                if ball.current_owner.is_some() {
                    ball.previous_owner = ball.current_owner;
                }
                ball.current_owner = None;
                ball.pending_owner = None;
                ball.velocity_z = 0;
                ball.height = 0; // 지면 (2025-12-11) - FIX_2512: i16
                ball.height_profile = HeightProfile::Flat;
            }
            BallState::OutOfPlay { position, .. } => {
                // FIX_2601: Coord10 직접 복사
                ball.position = *position;
                ball.velocity = Vel10::default();
                ball.is_in_flight = false;
                if ball.current_owner.is_some() {
                    ball.previous_owner = ball.current_owner;
                }
                ball.current_owner = None;
                ball.pending_owner = None;
                ball.velocity_z = 0;
                ball.height = 0; // 지면 (2025-12-11) - FIX_2512: i16
                ball.height_profile = HeightProfile::Flat;
            }
        }
    }

    /// Ball struct로부터 초기화 (new 대안)
    pub fn from_ball(ball: &super::Ball) -> Self {
        let mut queue = Self::new();
        queue.sync_from_ball(0, ball);
        queue
    }

    /// 액션 예약
    pub fn schedule(&mut self, action: ScheduledAction) -> u64 {
        let id = action.action_id;
        self.pending.push(Reverse(action));
        id
    }

    /// 새 액션 예약 (ID 자동 할당)
    pub fn schedule_new(
        &mut self,
        execute_tick: u64,
        action_type: ActionType,
        player_idx: usize,
        priority: u8,
    ) -> u64 {
        let action_id = self.next_action_id;
        self.next_action_id += 1;

        let action =
            ScheduledAction::new(execute_tick, action_type, player_idx, priority, action_id);

        self.schedule(action)
    }

    /// P16: ActionDetail 포함 액션 예약
    pub fn schedule_new_with_detail(
        &mut self,
        execute_tick: u64,
        action_type: ActionType,
        player_idx: usize,
        priority: u8,
        detail: ActionDetail,
    ) -> u64 {
        let action_id = self.next_action_id;
        self.next_action_id += 1;

        let action = ScheduledAction::new_with_detail(
            execute_tick,
            action_type,
            player_idx,
            priority,
            action_id,
            detail,
        );

        self.schedule(action)
    }

    /// 큐에 대기 중인 액션 수
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 특정 선수의 대기(pending) 액션 존재 여부
    pub fn has_pending_for_player(&self, player_idx: usize) -> bool {
        self.pending.iter().any(|action| action.0.player_idx == player_idx)
    }

    /// 다음에 실행될 액션 확인 (제거하지 않음)
    pub fn peek_next(&self) -> Option<&ScheduledAction> {
        self.pending.peek().map(|r| &r.0)
    }

    /// 특정 선수의 예약된 액션 취소
    pub fn cancel_player_actions(&mut self, player_idx: usize) -> Vec<ScheduledAction> {
        let mut cancelled = Vec::new();
        let mut remaining = BinaryHeap::new();

        while let Some(Reverse(action)) = self.pending.pop() {
            if action.player_idx == player_idx {
                cancelled.push(action);
            } else {
                remaining.push(Reverse(action));
            }
        }

        self.pending = remaining;
        cancelled
    }

    /// 공 관련 액션 모두 취소 (인터럽트 시)
    pub fn cancel_ball_actions(&mut self) -> Vec<ScheduledAction> {
        let mut cancelled = Vec::new();
        let mut remaining = BinaryHeap::new();

        while let Some(Reverse(action)) = self.pending.pop() {
            if action.action_type.requires_ball() || action.action_type.changes_ball_state() {
                cancelled.push(action);
            } else {
                remaining.push(Reverse(action));
            }
        }

        self.pending = remaining;
        cancelled
    }

    /// 특정 액션 ID 취소
    pub fn cancel_action(&mut self, action_id: u64) -> Option<ScheduledAction> {
        let mut cancelled = None;
        let mut remaining = BinaryHeap::new();

        while let Some(Reverse(action)) = self.pending.pop() {
            if action.action_id == action_id {
                cancelled = Some(action);
            } else {
                remaining.push(Reverse(action));
            }
        }

        self.pending = remaining;
        cancelled
    }

    /// 틱 실행 - 해당 틱의 모든 액션 실행 (레거시 모드)
    ///
    /// 실제 실행 로직은 MatchEngine에서 콜백으로 처리됨.
    /// 이 함수는 실행할 액션들을 반환만 함.
    ///
    /// P7 Phase 모드에서는 `execute_phase_tick()`을 사용해야 함.
    pub fn get_actions_for_tick(&mut self, tick: u64) -> Vec<ScheduledAction> {
        self.current_tick = tick;
        let mut actions = Vec::new();

        while let Some(Reverse(action)) = self.pending.peek() {
            if action.execute_tick <= tick {
                if let Some(Reverse(action)) = self.pending.pop() {
                    actions.push(action);
                }
            } else {
                break;
            }
        }

        actions
    }

    // ========== P7 Phase-Based FSM Methods ==========

    /// 실행 중인 active 액션 수
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// 특정 선수가 active 상태인지
    pub fn is_player_active(&self, player_idx: usize) -> bool {
        self.active.iter().any(|a| a.player_idx == player_idx)
    }

    /// 특정 선수의 active 액션 반환
    pub fn get_player_active_action(&self, player_idx: usize) -> Option<&ActiveAction> {
        self.active.iter().find(|a| a.player_idx == player_idx)
    }

    /// ScheduledAction → ActiveAction 변환
    ///
    /// pending에서 active로 이동 시 호출
    fn to_active_action(
        &mut self,
        scheduled: ScheduledAction,
        tick: u64,
        team_id: u32,
    ) -> ActiveAction {
        let action_type = match &scheduled.action_type {
            ActionType::Tackle { .. } => PhaseActionType::Tackle,
            ActionType::Pass { .. } => PhaseActionType::Pass,
            ActionType::Shot { .. } => PhaseActionType::Shot,
            ActionType::Dribble { .. } => PhaseActionType::Dribble,
            ActionType::Move { .. } => PhaseActionType::Move,
            ActionType::Trap { .. } => PhaseActionType::Trap,
            ActionType::Intercept { .. } => PhaseActionType::Intercept,
            ActionType::Header { .. } => PhaseActionType::Header,
            ActionType::Save { .. } => PhaseActionType::Save,
        };

        let mut active = ActiveAction::new(
            scheduled.action_id,
            scheduled.player_idx,
            team_id,
            action_type,
            tick,
        );

        // 타겟 설정
        match &scheduled.action_type {
            ActionType::Tackle { target_idx } => {
                active.target_player_idx = Some(*target_idx);
                active.meta = ActionMeta::Tackle {
                    tackle_type: TackleType::Standing,
                    approach_speed: TACKLE_APPROACH_SPEED,
                };
            }
            ActionType::Pass { target_idx, is_long, is_through, intended_target_pos, intended_passer_pos } => {
                active.target_player_idx = Some(*target_idx);
                let pass_type = scheduled
                    .detail
                    .pass_type
                    .map(PassType::from_detail)
                    .unwrap_or_else(|| {
                        if *is_through {
                            PassType::ThroughBall
                        } else if *is_long {
                            PassType::Lofted
                        } else {
                            PassType::Ground
                        }
                    });
                // FIX_2601/1128: Use intended_target_pos from ActionType::Pass
                // FIX_2601/1129: Store intended_passer_pos for forward_pass_rate measurement
                active.meta = ActionMeta::Pass {
                    pass_type,
                    target_pos: intended_target_pos.unwrap_or_default(),
                    pass_speed: pass_type.base_speed(),
                    intended_passer_pos: *intended_passer_pos,
                };
            }
            ActionType::Shot { power, target } => {
                active.target_position = Some(*target);
                active.meta = ActionMeta::Shot {
                    shot_type: ShotType::Normal,
                    target_pos: *target,
                    power: *power,
                };
            }
            ActionType::Dribble { direction, aggressive } => {
                active.meta = ActionMeta::Dribble {
                    direction: *direction,
                    is_aggressive: *aggressive,
                    touch_timer: 0,
                };
            }
            ActionType::Move { target, sprint } => {
                active.target_position = Some(*target);
                active.meta = ActionMeta::Move { target: *target, is_sprint: *sprint };
            }
            ActionType::Header { target, is_shot } => {
                active.target_position = Some(*target);
                // FIX_2601/0115: Store is_shot in meta to preserve it through conversions
                active.meta = ActionMeta::Header { is_shot: *is_shot };
            }
            ActionType::Save { direction: _ } => {
                // FIX_2601: direction은 (f32, f32) 단위벡터, Coord10가 아님 - 별도 처리 필요
                // target_position은 Coord10이므로 direction을 저장하지 않음
            }
            _ => {}
        }

        active
    }

    /// Phase 전환: 현재 Phase 완료 시 다음 Phase로 이동
    fn advance_action_phase(&mut self, action_idx: usize, tick: u64) {
        if action_idx >= self.active.len() {
            return;
        }

        // 먼저 immutable borrow로 필요한 정보 복사
        let (action_type, current_phase, meta_clone) = {
            let action = &self.active[action_idx];
            (action.action_type, action.phase, action.meta.clone())
        };

        // 다음 Phase duration 결정
        let next_duration = match current_phase {
            ActionPhase::Pending => self.get_approach_duration(action_type),
            ActionPhase::Approach => self.get_commit_duration(action_type, &meta_clone),
            ActionPhase::Commit => 1, // Resolve는 즉시
            ActionPhase::Resolve => self.get_recover_duration(action_type),
            ActionPhase::Recover => self.get_cooldown_duration(action_type),
            ActionPhase::Cooldown => 0, // Finished
            ActionPhase::Finished => 0,
        };

        // 이제 mutable borrow
        self.active[action_idx].advance_phase(tick, next_duration);
    }

    /// Approach Phase 지속 시간 (틱)
    fn get_approach_duration(&self, action_type: PhaseActionType) -> u64 {
        match action_type {
            PhaseActionType::Tackle => TACKLE_APPROACH_MAX_TICKS as u64, // 실제는 거리 기반
            PhaseActionType::Pass | PhaseActionType::Shot => 0,          // 패스/슛은 Approach 없음
            _ => 0,
        }
    }

    /// Commit Phase 지속 시간 (틱)
    fn get_commit_duration(&self, action_type: PhaseActionType, meta: &ActionMeta) -> u64 {
        match action_type {
            PhaseActionType::Tackle => {
                if let ActionMeta::Tackle { tackle_type, .. } = meta {
                    tackle_type.commit_ticks() as u64
                } else {
                    TACKLE_COMMIT_STANDING_TICKS as u64
                }
            }
            PhaseActionType::Pass => {
                if let ActionMeta::Pass { pass_type, .. } = meta {
                    pass_type.windup_ticks() as u64
                } else {
                    PASS_KICK_TICKS as u64
                }
            }
            PhaseActionType::Shot => {
                if let ActionMeta::Shot { shot_type, .. } = meta {
                    shot_type.windup_ticks() as u64
                } else {
                    SHOT_STRIKE_TICKS as u64
                }
            }
            _ => 1,
        }
    }

    /// Recover Phase 지속 시간 (틱)
    fn get_recover_duration(&self, action_type: PhaseActionType) -> u64 {
        match action_type {
            PhaseActionType::Tackle => TACKLE_RECOVERY_MISS_TICKS as u64,
            PhaseActionType::Pass => PASS_RECOVERY_TICKS as u64,
            PhaseActionType::Shot => SHOT_RECOVERY_TICKS as u64,
            _ => 2,
        }
    }

    /// Cooldown Phase 지속 시간 (틱)
    fn get_cooldown_duration(&self, action_type: PhaseActionType) -> u64 {
        match action_type {
            PhaseActionType::Tackle => TACKLE_COOLDOWN_TICKS as u64,
            PhaseActionType::Pass => PASS_COOLDOWN_TICKS as u64,
            PhaseActionType::Shot => SHOT_COOLDOWN_TICKS as u64,
            _ => 0,
        }
    }

    /// P7: Resolve Phase에서 실행할 액션 목록 반환
    ///
    /// 이 액션들은 즉발로 실행되어야 함 (판정 발생)
    pub fn get_resolve_actions(&self) -> Vec<usize> {
        self.active
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.phase == ActionPhase::Resolve && a.ticks_in_phase(self.current_tick) == 0
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// P7: Finished 상태인 액션 인덱스 목록
    pub fn get_finished_actions(&self) -> Vec<usize> {
        self.active
            .iter()
            .enumerate()
            .filter(|(_, a)| a.phase == ActionPhase::Finished)
            .map(|(i, _)| i)
            .collect()
    }

    /// P7: Finished 액션 제거
    pub fn remove_finished_actions(&mut self) {
        self.active.retain(|a| a.phase != ActionPhase::Finished);
    }

    /// P7: 특정 선수의 active 액션 취소
    pub fn cancel_active_for_player(&mut self, player_idx: usize) {
        self.active.retain(|a| a.player_idx != player_idx);
    }

    /// P7: 특정 선수의 pending 액션 취소
    pub fn cancel_pending_for_player(&mut self, player_idx: usize) {
        let mut retained = BinaryHeap::new();
        while let Some(action) = self.pending.pop() {
            if action.0.player_idx != player_idx {
                retained.push(action);
            }
        }
        self.pending = retained;
    }

    /// P7: pending → active 전환
    ///
    /// `can_start_fn`이 true를 반환하는 액션만 active로 전환
    /// Returns: 새로 활성화된 액션들의 (player_idx, action_id) 목록
    pub fn activate_pending_actions<F>(
        &mut self,
        tick: u64,
        get_team_id: impl Fn(usize) -> u32,
        can_start_fn: F,
    ) -> Vec<(usize, u64)>
    where
        F: Fn(usize, &ActionType) -> bool,
    {
        self.current_tick = tick;
        self.tick_results.clear();
        let mut activated = Vec::new();

        // pending에서 실행 시간이 된 액션들 추출
        let mut to_activate = Vec::new();
        let mut to_retry = Vec::new();

        while let Some(Reverse(scheduled)) = self.pending.peek() {
            if scheduled.execute_tick > tick {
                break;
            }
            let scheduled = self.pending.pop().unwrap().0;

            if can_start_fn(scheduled.player_idx, &scheduled.action_type) {
                to_activate.push(scheduled);
            } else {
                // 행동 불가면 다음 틱으로 미룸
                let mut retry = scheduled;
                retry.execute_tick = tick + 1;
                to_retry.push(retry);
            }
        }

        // 미룬 액션 다시 pending에 추가
        for retry in to_retry {
            self.pending.push(Reverse(retry));
        }

        // active로 전환
        for scheduled in to_activate {
            let player_idx = scheduled.player_idx;
            let action_id = scheduled.action_id;
            let team_id = get_team_id(player_idx);

            let mut active = self.to_active_action(scheduled, tick, team_id);

            // Pending → Approach (또는 Commit for Pass/Shot)
            active.phase = ActionPhase::Approach;
            active.phase_tick_start = tick;
            active.phase_duration = self.get_approach_duration(active.action_type);

            // Pass/Shot은 Approach 없이 바로 Commit
            if active.phase_duration == 0 {
                active.phase = ActionPhase::Commit;
                active.phase_duration = self.get_commit_duration(active.action_type, &active.meta);
            }

            self.active.push(active);
            activated.push((player_idx, action_id));
        }

        activated
    }

    /// P7: active 액션들의 Phase 틱 업데이트
    ///
    /// Returns: Resolve Phase에 진입한 액션들의 인덱스
    pub fn tick_active_actions(&mut self, tick: u64) -> Vec<usize> {
        self.current_tick = tick;
        let mut resolve_indices = Vec::new();

        for i in 0..self.active.len() {
            let action = &self.active[i];

            // Phase 완료 체크
            if action.is_phase_complete(tick) {
                let current_phase = action.phase;

                // Resolve 직전이면 인덱스 기록 (Commit → Resolve)
                if current_phase == ActionPhase::Commit {
                    resolve_indices.push(i);
                }

                // Phase 전환
                self.advance_action_phase(i, tick);
            }
        }

        resolve_indices
    }

    /// P7: active 액션에 접근 (index로)
    pub fn get_active_action(&self, index: usize) -> Option<&ActiveAction> {
        self.active.get(index)
    }

    /// P7: active 액션에 mutable 접근 (index로)
    pub fn get_active_action_mut(&mut self, index: usize) -> Option<&mut ActiveAction> {
        self.active.get_mut(index)
    }

    /// P7: active 액션을 Finished로 설정
    pub fn finish_action(&mut self, index: usize) {
        if let Some(action) = self.active.get_mut(index) {
            action.finish();
        }
    }

    /// P7: active 액션을 Recover로 전환 (Resolve 후)
    pub fn action_to_recover(&mut self, index: usize, tick: u64) {
        // 먼저 action_type을 복사
        let action_type = match self.active.get(index) {
            Some(action) => action.action_type,
            None => return,
        };
        let duration = self.get_recover_duration(action_type);

        // 이제 mutable borrow
        if let Some(action) = self.active.get_mut(index) {
            action.phase = ActionPhase::Recover;
            action.phase_tick_start = tick;
            action.phase_duration = duration;
        }
    }

    /// 이번 틱 결과 기록
    pub fn record_result(&mut self, result: ActionResult) {
        // 결과에 따라 공 상태 업데이트
        match &result {
            ActionResult::PassStarted { passer_idx, .. } => {
                // FIX_2601/0102: 어시스트 추적용 패서 저장
                self.last_passer_idx = Some(*passer_idx);
                if let ActionResult::PassStarted { receiver_idx, .. } = &result {
                    self.last_pass_receiver_idx = Some(*receiver_idx);
                }
            }
            ActionResult::TrapSuccess { player_idx } => {
                self.ball_state = BallState::Controlled { owner_idx: *player_idx };
                self.last_pass_type = None;
                self.in_flight_origin = None;
            }
            ActionResult::TrapFailed { loose_ball_pos, .. } => {
                self.last_passer_idx = None;
                self.last_pass_receiver_idx = None;
                self.last_pass_type = None;
                self.in_flight_origin = None;
                // FIX_2601: Vel10 사용
                self.ball_state =
                    BallState::Loose { position: *loose_ball_pos, velocity: Vel10::default() };
            }
            ActionResult::CarryComplete { player_idx, .. } => {
                // 운반 완료 - 통계 미기록
                self.ball_state = BallState::Controlled { owner_idx: *player_idx };
            }
            ActionResult::TakeOnComplete { player_idx, .. } => {
                // 돌파 성공 - 드리블 통계에 기록됨
                self.ball_state = BallState::Controlled { owner_idx: *player_idx };
            }
            ActionResult::DribbleTackled { tackler_idx, .. } => {
                self.ball_state = BallState::Controlled { owner_idx: *tackler_idx };
            }
            ActionResult::InterceptSuccess { player_idx } => {
                self.last_passer_idx = None;
                self.last_pass_receiver_idx = None;
                self.last_pass_type = None;
                self.ball_state = BallState::Controlled { owner_idx: *player_idx };
            }
            ActionResult::TackleSuccess { tackler_idx, .. } => {
                self.ball_state = BallState::Controlled { owner_idx: *tackler_idx };
            }
            ActionResult::OutOfBounds { restart_type, position, home_team } => {
                self.last_passer_idx = None;
                self.last_pass_receiver_idx = None;
                self.last_pass_type = None;
                self.in_flight_origin = None;
                self.ball_state = BallState::OutOfPlay {
                    restart_type: *restart_type,
                    position: *position,
                    home_team: *home_team,
                };
            }
            ActionResult::GoalkeeperHandlingViolation { .. } => {
                // A technical foul triggers a restart; clear stale pass metadata so the next
                // TrapSuccess cannot accidentally attribute a pass/assist.
                self.last_passer_idx = None;
                self.last_pass_receiver_idx = None;
                self.last_pass_type = None;
                self.in_flight_origin = None;
            }
            ActionResult::HandballFoul { .. } => {
                // A foul triggers a restart; clear stale pass/origin metadata so the next
                // TrapSuccess cannot accidentally attribute a pass/assist.
                self.last_passer_idx = None;
                self.last_pass_receiver_idx = None;
                self.last_pass_type = None;
                self.in_flight_origin = None;
            }
            ActionResult::GoalScored { .. } => {
                self.last_pass_type = None;
                // FIX_2601: Coord10 사용
                self.ball_state = BallState::OutOfPlay {
                    restart_type: RestartType::KickOff,
                    position: Coord10::CENTER, // 센터
                    home_team: true,                            // 득점 안 한 팀
                };
            }
            _ => {}
        }

        self.tick_results.push(result);
    }

    /// 이번 틱 결과 가져오기 및 초기화
    pub fn take_tick_results(&mut self) -> Vec<ActionResult> {
        std::mem::take(&mut self.tick_results)
    }

    /// 큐 초기화
    pub fn clear(&mut self) {
        self.pending.clear();
        self.tick_results.clear();
        self.ball_state = BallState::default();
        self.current_tick = 0;
        self.last_passer_idx = None;
        self.last_pass_receiver_idx = None;
        self.last_pass_type = None;
        self.last_header_outcome = None;
        self.in_flight_origin = None;
    }

    // ========== Phase 3.3: Interrupt Handling ==========

    /// 인터럽트 처리
    ///
    /// 공 관련 미래 액션을 취소하고, 필요시 경기 재개 상태로 전환합니다.
    ///
    /// # Returns
    /// 취소된 액션들
    /// FIX_2601: position 파라미터를 Coord10로 변경
    pub fn handle_interrupt(
        &mut self,
        reason: &InterruptReason,
        position: Coord10,
    ) -> Vec<ScheduledAction> {
        // 1. 공 관련 액션 취소
        let cancelled = self.cancel_ball_actions();

        // 2. 인터럽트 이유에 따라 공 상태 변경
        match reason {
            InterruptReason::PhysicsCollision { impact_force, .. } => {
                // 충돌 → 루즈볼 - FIX_2601: Vel10 사용
                let scatter_factor = impact_force * 3.0;
                self.ball_state = BallState::Loose {
                    position,
                    velocity: Vel10::from_mps(scatter_factor, scatter_factor * 0.5),
                };
            }

            InterruptReason::Foul { position: foul_pos, .. } => {
                let restart = reason.restart_type().unwrap_or(RestartType::FreeKick);
                // 파울 → 프리킥/페널티
                self.ball_state = BallState::OutOfPlay {
                    restart_type: restart,
                    position: *foul_pos,
                    home_team: true, // 파울 당한 팀이 재개
                };
            }

            InterruptReason::OutOfBounds { position: out_pos, last_touch_home } => {
                let restart = reason.restart_type().unwrap_or(RestartType::ThrowIn);
                // 아웃 → 스로인/골킥/코너
                self.ball_state = BallState::OutOfPlay {
                    restart_type: restart,
                    position: *out_pos,
                    home_team: !*last_touch_home, // 마지막 터치 안 한 팀이 재개
                };
            }

            InterruptReason::GoalScored { home_team, .. } => {
                // 골 → 킥오프 - FIX_2601: Coord10 사용
                self.ball_state = BallState::OutOfPlay {
                    restart_type: RestartType::KickOff,
                    position: Coord10::CENTER, // 센터
                    home_team: !*home_team,                     // 실점 팀이 킥오프
                };
            }

            InterruptReason::Offside { position: off_pos, .. } => {
                // 오프사이드 → 프리킥
                self.ball_state = BallState::OutOfPlay {
                    restart_type: RestartType::FreeKick,
                    position: *off_pos,
                    home_team: true, // 수비 팀이 재개
                };
            }

            InterruptReason::BallWon { winner_idx, .. } => {
                // 볼 탈취 → 탈취자가 소유
                self.ball_state = BallState::Controlled { owner_idx: *winner_idx };
            }
        }

        // 3. 취소 결과 기록
        for action in &cancelled {
            self.tick_results.push(ActionResult::Cancelled {
                action_id: action.action_id,
                reason: format!("Interrupted: {:?}", reason),
            });
        }

        cancelled
    }

    /// 루즈볼 경합 시작
    ///
    /// 가장 가까운 선수들이 공을 향해 경합합니다.
    ///
    /// FIX_2601/0102: Teammate Ownership Prevention
    /// - 현재 소유자가 근처에 있으면 유지
    /// - 같은 팀 선수에게 자동 이전 안함 (명시적 패스 필요)
    /// - 상대팀만 태클로 획득 가능
    ///
    /// # Returns
    /// 경합 결과와 생성된 인터셉트 액션들
    pub fn start_loose_ball_contest(
        &mut self,
        ball_pos: (f32, f32),
        player_positions: &[(f32, f32)],
        player_stats: &[PlayerStats],
    ) -> LooseBallContest {
        // FIX_2601/0102: 현재 소유자 확인
        let current_owner = self.ball_state.owner();
        let owner_is_home = current_owner.map(|idx| idx < 11);

        // 모든 선수의 거리 계산
        let mut distances: Vec<(usize, f32)> = player_positions
            .iter()
            .enumerate()
            .map(|(idx, pos)| {
                let dx = pos.0 - ball_pos.0;
                let dy = pos.1 - ball_pos.1;
                (idx, (dx * dx + dy * dy).sqrt())
            })
            .collect();

        // 거리순 정렬 (position hash for ties - FIX_2601/0116: no index-based bias)
        distances.sort_by(|a, b| {
            match a.1.partial_cmp(&b.1) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // FIX_2601/0116: Use position-based hash for fair tie-breaking
                    let pos_a = player_positions.get(a.0).copied().unwrap_or((0.0, 0.0));
                    let pos_b = player_positions.get(b.0).copied().unwrap_or((0.0, 0.0));
                    crate::engine::match_sim::deterministic_tie_hash(a.0, pos_a, b.0, pos_b)
                }
                Some(ord) => ord,
            }
        });

        // FIX_2601/0102: 규칙 1 - 현재 소유자가 5m 이내면 유지
        if let Some(owner) = current_owner {
            if let Some((_, dist)) = distances.iter().find(|(idx, _)| *idx == owner) {
                if *dist < 5.0 {
                    // 소유자가 근처 → 경합 없이 유지
                    return LooseBallContest {
                        position: ball_pos,
                        contestants: vec![(owner, *dist)],
                        winner: Some(owner),
                        is_fifty_fifty: false,
                    };
                }
            }
        }

        // FIX_2601/0102: 규칙 2 & 3 - 상대팀만 경합 가능
        let contestants: Vec<(usize, f32)> = if let Some(owner_home) = owner_is_home {
            // 상대팀만 필터링 (5m 이내, 최대 3명)
            distances
                .into_iter()
                .filter(|(idx, d)| {
                    let player_is_home = *idx < 11;
                    player_is_home != owner_home && *d < 5.0 // 상대팀만
                })
                .take(3)
                .collect()
        } else {
            // 소유자 없음 → 모든 선수 경합 가능
            distances.into_iter().take(3).filter(|(_, d)| *d < 5.0).collect()
        };

        // 50:50 여부 판정 (두 선수의 거리 차이가 0.5m 이내)
        let is_fifty_fifty =
            contestants.len() >= 2 && (contestants[0].1 - contestants[1].1).abs() < 0.5;

        // 승자 결정
        let winner = if contestants.is_empty() {
            None
        } else if is_fifty_fifty && contestants.len() >= 2 {
            // FIX_2601/0102: 50:50 경합 → 태클 점수 기반 판정
            let p1_idx = contestants[0].0;
            let p2_idx = contestants[1].0;

            // 태클 점수 계산 (tackling 40%, aggression 20%, strength 20%, bravery 10%, agility 10%)
            let p1_tackle = if p1_idx < player_stats.len() {
                player_stats[p1_idx].tackle_score()
            } else {
                0.5 // 기본값 (50점 기준)
            };

            let p2_tackle =
                if p2_idx < player_stats.len() { player_stats[p2_idx].tackle_score() } else { 0.5 };

            // 확률적 판정 (태클 점수 비율로 승률 결정)
            // FIX_2601/0116: XOR for symmetric salt (3^15 == 15^3, unlike addition)
            let random = simple_random(self.current_tick, self.current_tick, p1_idx ^ p2_idx);
            let p1_prob = p1_tackle / (p1_tackle + p2_tackle);

            if random < p1_prob {
                Some(p1_idx)
            } else {
                Some(p2_idx)
            }
        } else {
            // 가장 가까운 선수 승리
            Some(contestants[0].0)
        };

        // 경합 액션 예약 - FIX_2601: (f32, f32) → Coord10 변환
        let ball_pos_coord10 = Coord10::from_meters(ball_pos.0, ball_pos.1);
        for (idx, dist) in &contestants {
            let arrival_ticks = (*dist / 7.0 * TICKS_PER_SECOND as f32).ceil() as u64; // ~7m/s 속도
            self.schedule_new(
                self.current_tick + arrival_ticks.max(1),
                ActionType::Intercept { ball_position: ball_pos_coord10 },
                *idx,
                90, // 높은 우선순위
            );
        }

        // 승자가 있으면 공 소유 업데이트 예약
        if let Some(winner_idx) = winner {
            let winner_dist =
                contestants.iter().find(|(i, _)| *i == winner_idx).map(|(_, d)| *d).unwrap_or(1.0);
            let _arrival_ticks = (winner_dist / 7.0 * TICKS_PER_SECOND as f32).ceil() as u64;

            // 승자가 도착하면 공 소유 - FIX_2601: Coord10/Vel10 사용
            self.ball_state = BallState::Loose {
                position: ball_pos_coord10,
                velocity: Vel10::default(), // 정지
            };
        }

        LooseBallContest { position: ball_pos, contestants, winner, is_fifty_fifty }
    }

    /// 루즈볼 상태 확인 및 자동 경합 시작
    ///
    /// BallState::Loose일 때 호출하면 자동으로 경합을 시작합니다.
    pub fn check_and_start_loose_ball(
        &mut self,
        player_positions: &[(f32, f32)],
        player_stats: &[PlayerStats],
    ) -> Option<LooseBallContest> {
        if let BallState::Loose { position, .. } = self.ball_state {
            // 이미 인터셉트 액션이 예약되어 있는지 확인
            let has_intercept = self
                .pending
                .iter()
                .any(|Reverse(a)| matches!(a.action_type, ActionType::Intercept { .. }));

            if !has_intercept {
                // FIX_2601: Coord10 → meters 변환
                return Some(self.start_loose_ball_contest(
                    position.to_meters(),
                    player_positions,
                    player_stats,
                ));
            }
        }
        None
    }

    // ========== State Snapshot API ==========

    /// Export queue state as a serializable snapshot
    ///
    /// Converts BinaryHeap to sorted Vec for serialization.
    pub fn to_snapshot(&self) -> super::snapshot::ActionQueueSnapshot {
        // Convert BinaryHeap to sorted Vec
        let mut pending: Vec<_> = self.pending.iter().map(|Reverse(a)| a.clone()).collect();
        pending.sort_by_key(|a| a.execute_tick);

        super::snapshot::ActionQueueSnapshot {
            pending,
            active: self.active.clone(),
            ball_state: self.ball_state.clone(),
            next_action_id: self.next_action_id,
            current_tick: self.current_tick,
            last_shot_xg: self.last_shot_xg,
            last_shooter_idx: self.last_shooter_idx,
            last_passer_idx: self.last_passer_idx,
            last_pass_receiver_idx: self.last_pass_receiver_idx,
            last_pass_type: self.last_pass_type,
            last_header_outcome: self.last_header_outcome.clone(),
            in_flight_origin: self.in_flight_origin,
        }
    }

    /// Restore queue state from a snapshot
    ///
    /// Converts Vec back to BinaryHeap.
    pub fn from_snapshot(snapshot: super::snapshot::ActionQueueSnapshot) -> Self {
        // Convert Vec back to BinaryHeap
        let pending = snapshot.pending.into_iter().map(Reverse).collect();

        Self {
            pending,
            active: snapshot.active,
            ball_state: snapshot.ball_state,
            tick_results: Vec::new(), // Reset - these are per-tick
            next_action_id: snapshot.next_action_id,
            current_tick: snapshot.current_tick,
            last_shot_xg: snapshot.last_shot_xg,
            last_shooter_idx: snapshot.last_shooter_idx,
            last_passer_idx: snapshot.last_passer_idx,
            last_pass_receiver_idx: snapshot.last_pass_receiver_idx,
            last_pass_type: snapshot.last_pass_type,
            last_header_outcome: snapshot.last_header_outcome,
            in_flight_origin: snapshot.in_flight_origin,
        }
    }
}

// ============================================================================
// Action Executors - Phase 3.2
// ============================================================================

/// 액션 실행 컨텍스트 (MatchEngine에서 제공)
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// 선수 위치들 (인덱스 → (x, y))
    pub player_positions: Vec<(f32, f32)>,
    /// 선수 스탯들 (인덱스 → 능력치)
    pub player_stats: Vec<PlayerStats>,
    /// 골키퍼 인덱스 (홈, 어웨이)
    pub goalkeeper_indices: (usize, usize),
    /// 현재 틱
    pub current_tick: u64,
    /// 랜덤 시드 (재현성)
    pub rng_seed: u64,
    /// FIX_2601/0110: 홈팀 방향 컨텍스트 (하프타임 스왑 반영)
    pub home_ctx: DirectionContext,
    /// FIX_2601/0110: 원정팀 방향 컨텍스트 (하프타임 스왑 반영)
    pub away_ctx: DirectionContext,
    /// FIX_2601/0109: Sparse scalar modifiers for Home team (deck/coach, etc.).
    pub home_match_modifiers: super::TeamMatchModifiers,
    /// FIX_2601/0109: Sparse scalar modifiers for Away team (deck/coach, etc.).
    pub away_match_modifiers: super::TeamMatchModifiers,

    /// Rulebook: optional non-GK handball triggers enabled (executor-only).
    pub rulebook_non_gk_handball_enabled: bool,
    /// Rulebook: probability multiplier for non-GK handball triggers.
    pub rulebook_non_gk_handball_prob_mult: f32,
    /// Rulebook: advantage play enabled (executor-only).
    pub rulebook_advantage_play_enabled: bool,

    /// FIX_2601/1120: Current ball position (meters) for accurate InFlight origin.
    /// This prevents teleportation when starting passes/shots - the ball should start
    /// from its actual position, not the player's position.
    pub ball_position: (f32, f32),
}

/// 선수 기본 능력치 (액션 실행에 필요한 것만)
#[derive(Debug, Clone, Default)]
pub struct PlayerStats {
    pub passing: u8,
    pub first_touch: u8,
    pub dribbling: u8,
    pub finishing: u8,
    pub long_shots: u8,
    pub tackling: u8,
    pub anticipation: u8,
    pub composure: u8,
    pub agility: u8,
    pub reflexes: u8,    // GK
    pub handling: u8,    // GK
    pub positioning: u8, // GK (FIX_2601/0109)
    pub diving: u8,      // GK (FIX_2601/0109)
    pub heading: u8,     // 2025-12-11 P2: Header action
    pub jumping: u8,     // 2025-12-11 P2: Header action
    pub strength: u8,    // 2025-12-11 P2: Header action (aerial duels)
    // ActionModel Integration: Pass 추가 필드
    pub technique: u8,
    pub vision: u8,
    pub decisions: u8,
    // FIX_2601/0102: Tackle Score Calculation
    pub aggression: u8,
    pub bravery: u8,
    // FIX_2601/0107: FM meta attributes
    pub concentration: u8,
    pub pace: u8,
    pub acceleration: u8,
    pub balance: u8,
    pub teamwork: u8,
    pub flair: u8,
    /// FIX01 C1: 1..=5 (optional in tests; engine sets from MatchSetup)
    pub condition_level: u8,
}

impl PlayerStats {
    /// FIX_2601/0102: 태클 점수 계산
    ///
    /// 가중치: tackling 40%, aggression 20%, strength 20%, bravery 10%, agility 10%
    pub fn tackle_score(&self) -> f32 {
        self.tackling as f32 * 0.004      // 40% / 100
            + self.aggression as f32 * 0.002  // 20% / 100
            + self.strength as f32 * 0.002    // 20% / 100
            + self.bravery as f32 * 0.001     // 10% / 100
            + self.agility as f32 * 0.001 // 10% / 100
    }
}

/// 간단한 압박 계산 (ExecutionError용)
/// 5m 이내 상대 선수 수와 거리 기반
fn calculate_pressure_simple(player_pos: (f32, f32), nearby_opponents: &[(f32, f32)]) -> f32 {
    const PRESSURE_RADIUS: f32 = 5.0;
    let mut pressure = 0.0;

    for opp_pos in nearby_opponents {
        let dx = player_pos.0 - opp_pos.0;
        let dy = player_pos.1 - opp_pos.1;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < PRESSURE_RADIUS {
            pressure += (PRESSURE_RADIUS - dist) / PRESSURE_RADIUS;
        }
    }

    // 최대 2명 수비수의 영향 (1.0 cap)
    (pressure / 2.0).min(1.0)
}

/// 패스 실행
pub fn execute_pass(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Pass { target_idx, is_long, is_through, intended_target_pos, intended_passer_pos } = &action.action_type else {
        debug_assert!(false, "execute_pass called with non-Pass action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_pass called with non-Pass action".to_string(),
        };
    };

    let passer_pos = ctx.player_positions[action.player_idx];
    let receiver_pos = ctx.player_positions[*target_idx];
    let stats = &ctx.player_stats[action.player_idx];

    // FIX_2601/1120: Use actual ball position for InFlight origin (Pattern B fix)
    // This prevents teleportation when starting a pass - the ball should start
    // from its current position, not necessarily the passer's exact position.
    // - For InFlight: use interpolated position at current tick
    // - For Controlled: use player position (ball follows player)
    // - For Loose/OutOfPlay: use position from ball_state
    let ball_origin_m = get_ball_position_at_tick(&queue.ball_state, ctx.current_tick, &ctx.player_positions)
        .unwrap_or(passer_pos);

    let pass_type = action
        .detail
        .pass_type
        .map(PassType::from_detail)
        .unwrap_or_else(|| {
            if *is_through {
                PassType::ThroughBall
            } else if *is_long {
                PassType::Lofted
            } else {
                PassType::Ground
            }
        });

    queue.last_pass_type = Some(pass_type);

    // 거리 계산
    let dx = receiver_pos.0 - passer_pos.0;
    let dy = receiver_pos.1 - passer_pos.1;
    let distance = (dx * dx + dy * dy).sqrt();

    // ActionModel Integration: 상황 기반 Intent/Technique 선택
    // 압박 계산
    let passer_is_home = TeamSide::is_home(action.player_idx);
    let opponent_range = if passer_is_home { 11..22 } else { 0..11 };
    let nearby_opponents: Vec<(f32, f32)> = opponent_range
        .filter_map(|i| {
            let opp_pos = ctx.player_positions[i];
            let dist =
                ((opp_pos.0 - passer_pos.0).powi(2) + (opp_pos.1 - passer_pos.1).powi(2)).sqrt();
            if dist < 5.0 {
                Some(opp_pos)
            } else {
                None
            }
        })
        .collect();
    let pressure = calculate_pressure_simple(passer_pos, &nearby_opponents);

    // 리시버 열림 정도 계산
    let opponent_range_2 = if passer_is_home { 11..22 } else { 0..11 };
    let defenders_near_receiver: usize = opponent_range_2
        .filter(|&i| {
            let opp_pos = ctx.player_positions[i];
            let dist = ((opp_pos.0 - receiver_pos.0).powi(2)
                + (opp_pos.1 - receiver_pos.1).powi(2))
            .sqrt();
            dist < 5.0
        })
        .count();
    let target_openness =
        if defenders_near_receiver == 0 { 1.0 } else { 0.5 / defenders_near_receiver as f32 };

    // 거리 정규화 (0=가까운 짧패스, 1=긴 패스)
    let distance_norm = (distance / 50.0).clamp(0.0, 1.0);

    // 침투 라인 존재 (through pass 여부로 대체)
    let through_lane = if pass_type.is_through() { 0.8 } else { 0.2 };

    // 크로스 각도 (측면 여부)
    let crossing_angle = if passer_pos.1 < 20.0 || passer_pos.1 > 48.0 { 0.8 } else { 0.3 };

    let pass_ctx = PassContext { distance_norm, target_openness, through_lane, crossing_angle };
    let passer_skills = PassSkills {
        passing: stats.passing,
        technique: stats.technique,
        vision: stats.vision,
        decisions: stats.decisions,
        composure: stats.composure,
    };

    // Intent 추론
    let intent = match pass_type {
        PassType::ThroughBall => PassIntent::Penetrate,
        PassType::Cross => PassIntent::Progress,
        PassType::BackPass => PassIntent::Retain,
        PassType::Lofted => {
            if distance > 30.0 {
                PassIntent::Switch
            } else {
                PassIntent::Progress
            }
        }
        PassType::Ground => {
            if distance > 25.0 {
                PassIntent::Progress
            } else {
                PassIntent::Retain
            }
        }
    };

    // Technique 선택 및 물리 파라미터 적용
    let selection = match pass_type {
        PassType::Cross => PassTechniqueSelection {
            technique: PassTechnique::Cross,
            success_chance: pass_base_success_prob(
                PassTechnique::Cross,
                &passer_skills,
                pressure,
            ),
        },
        PassType::ThroughBall => PassTechniqueSelection {
            technique: PassTechnique::Through,
            success_chance: pass_base_success_prob(
                PassTechnique::Through,
                &passer_skills,
                pressure,
            ),
        },
        PassType::Lofted => PassTechniqueSelection {
            technique: PassTechnique::Lofted,
            success_chance: pass_base_success_prob(
                PassTechnique::Lofted,
                &passer_skills,
                pressure,
            ),
        },
        PassType::BackPass => PassTechniqueSelection {
            technique: PassTechnique::Ground,
            success_chance: pass_base_success_prob(
                PassTechnique::Ground,
                &passer_skills,
                pressure,
            ),
        },
        PassType::Ground => choose_pass_technique(intent, &pass_ctx, &passer_skills, pressure),
    };
    let physics = selection.technique.physics_params();

    let height_profile = match selection.technique {
        PassTechnique::Ground | PassTechnique::Driven | PassTechnique::Through => HeightProfile::Flat,
        PassTechnique::Lofted | PassTechnique::Cross | PassTechnique::Clear => HeightProfile::Lob,
    };
    let lift_intent = match selection.technique {
        PassTechnique::Ground => 0.0,
        PassTechnique::Through => {
            let base = 0.2 + 0.5 * distance_norm;
            let lane = 0.7 + 0.3 * through_lane;
            (base * lane).clamp(0.15, 0.75)
        }
        _ => 1.0,
    };
    let pass_skill = (stats.passing as f32 + stats.technique as f32) / 200.0;
    let lift_ratio = compute_lift_ratio(lift_intent, pass_skill, pressure);

    // 패스 속도 계산 (ActionModel 물리 적용)
    // physics.speed는 기술별 기본 속도 (m/s)
    let ball_speed = physics.speed + (distance * 0.1).min(5.0); // 거리에 따라 약간 보정

    // 비행 시간 계산 (틱 단위, 10 ticks/sec)
    let flight_ticks = ((distance / ball_speed) * TICKS_PER_SECOND as f32).ceil() as u64;
    let arrival_tick = ctx.current_tick + flight_ticks.max(2);

    // 공 상태 업데이트 - FIX_2601: (f32, f32) → Coord10 변환
    // FIX_2601/1120: Use ball_origin_m instead of passer_pos for Pattern B fix
    queue.ball_state = BallState::InFlight {
        from_pos: Coord10::from_meters(ball_origin_m.0, ball_origin_m.1),
        to_pos: Coord10::from_meters(receiver_pos.0, receiver_pos.1),
        start_tick: ctx.current_tick,
        end_tick: arrival_tick,
        height_profile,
        lift_ratio,
        intended_receiver: Some(*target_idx),
        is_shot: false,
        start_height_01m: 0,
        end_height_01m: 0,
    };

    // 수신자의 트랩/헤더 액션 예약
    let ball_height = max_height_from_profile(height_profile, lift_ratio);
    #[cfg(debug_assertions)]
    if matches!(selection.technique, PassTechnique::Through) {
        record_through_height(ball_height);
    }

    // FIX_2601/0116: Schedule Header only for aerial deliveries (cross/loft/clear).
    const HEADER_HEIGHT_THRESHOLD: f32 = 1.8; // meters
    let is_aerial_delivery = matches!(
        selection.technique,
        PassTechnique::Cross | PassTechnique::Lofted | PassTechnique::Clear
    );
    let receiver_is_gk = {
        let receiver_is_home = TeamSide::is_home(*target_idx);
        let receiver_gk_idx = if receiver_is_home {
            ctx.goalkeeper_indices.0
        } else {
            ctx.goalkeeper_indices.1
        };
        *target_idx == receiver_gk_idx
    };
    if action_debug_enabled() && is_aerial_delivery {
        eprintln!(
            "[AERIAL L1] tick={} passer={} receiver={} pass_type={:?} technique={:?} profile={:?} height={:.2} start={} end={} is_long={} is_through={}",
            ctx.current_tick,
            action.player_idx,
            *target_idx,
            pass_type,
            selection.technique,
            height_profile,
            ball_height,
            ctx.current_tick,
            arrival_tick,
            is_long,
            is_through
        );
    }

    // DEBUG: Track all pass heights
    if action_debug_enabled()
        && (!matches!(height_profile, HeightProfile::Flat) || ball_height > 0.1)
    {
        eprintln!(
            "[DEBUG PASS] technique={:?} profile={:?} lift_intent={:.2} lift_ratio={:.2} height={:.2}m",
            selection.technique, height_profile, lift_intent, lift_ratio, ball_height
        );
    }

    if is_aerial_delivery && ball_height > HEADER_HEIGHT_THRESHOLD && !receiver_is_gk {
        // Determine if this should be a headed shot or pass
        // Receiver in attacking box = headed shot attempt
        let receiver_pos_m = ctx.player_positions[*target_idx];
        let is_home = TeamSide::is_home(*target_idx);
        let dir_ctx = if is_home { &ctx.home_ctx } else { &ctx.away_ctx };      
        let goal_x = dir_ctx.opponent_goal_x() * field::LENGTH_M; // normalized → meters
        let goal_center_y = field::CENTER_Y;

        // Check if in attacking penalty box (within 16.5m of goal line, within box width)
        let in_box = (receiver_pos_m.0 - goal_x).abs() < 16.5
            && receiver_pos_m.1 > 13.84
            && receiver_pos_m.1 < 54.16;

        // FIX_2601/0115: Add probability gate for aerial-to-shot conversion
        // Real football: ~50% crosses cleared, ~30% contested, ~20% become shots
        // Previously: 100% of crosses to box = shot attempt (way too high!)
        let (target, is_shot) = if in_box {
            // Calculate probability of header becoming a shot
            // Base: 25% chance (realistic for contested headers)
            // Modifiers: passer skill, distance from goal
            let passer_stats = &ctx.player_stats[action.player_idx];
            // Use passing + technique as proxy for crossing quality
            let crossing_skill = (passer_stats.passing as f32 + passer_stats.technique as f32) / 200.0;
            let dist_to_goal = (receiver_pos_m.0 - goal_x).abs();
            let distance_bonus: f32 = if dist_to_goal < 6.0 { 0.15 } else if dist_to_goal < 11.0 { 0.05 } else { 0.0 };

            let header_shot_prob: f32 = (0.20 + crossing_skill * 0.15 + distance_bonus).min(0.45);
            let roll = simple_random(ctx.rng_seed, ctx.current_tick, *target_idx + 500);

            if roll < header_shot_prob {
                // Headed shot toward goal
                (Coord10::from_meters(goal_x, goal_center_y), true)
            } else {
                // Header contested/cleared - becomes a pass toward nearby space
                let forward_x = if dir_ctx.attacks_right { receiver_pos_m.0 + 5.0 } else { receiver_pos_m.0 - 5.0 };
                (
                    Coord10::from_meters(forward_x.clamp(0.0, field::LENGTH_M), receiver_pos_m.1),
                    false,
                )
            }
        } else {
            // Headed pass/clearance - aim forward or toward teammate
            // For simplicity, aim toward the passer's forward direction
            let forward_x = if dir_ctx.attacks_right { receiver_pos_m.0 + 10.0 } else { receiver_pos_m.0 - 10.0 };
            (
                Coord10::from_meters(forward_x.clamp(0.0, field::LENGTH_M), receiver_pos_m.1),
                false,
            )
        };

        if action_debug_enabled() {
            eprintln!(
                "[DEBUG HEADER] Scheduling Header for player {} at tick {}, is_shot={}, height={:.2}m",
                *target_idx, arrival_tick, is_shot, ball_height
            );
        }
        if action_debug_enabled() {
            eprintln!(
                "[AERIAL L2] tick={} action=Header player={} arrival={} height={:.2} is_shot={} intended_receiver={:?}",
                ctx.current_tick,
                *target_idx,
                arrival_tick,
                ball_height,
                is_shot,
                Some(*target_idx)
            );
        }

        queue.schedule_new(
            arrival_tick,
            ActionType::Header { target, is_shot },
            *target_idx,
            100, // 높은 우선순위
        );
    } else {
        if action_debug_enabled() && is_aerial_delivery {
            eprintln!(
                "[AERIAL L2] tick={} action=Trap player={} arrival={} height={:.2} intended_receiver={:?}",
                ctx.current_tick,
                *target_idx,
                arrival_tick,
                ball_height,
                Some(*target_idx)
            );
        }
        queue.schedule_new(
            arrival_tick,
            ActionType::Trap { ball_speed, ball_height },
            *target_idx,
            100, // 높은 우선순위
        );
    }

    ActionResult::PassStarted {
        passer_idx: action.player_idx,
        receiver_idx: *target_idx,
        arrival_tick,
        ball_speed,
        intended_target_pos: *intended_target_pos,
        intended_passer_pos: *intended_passer_pos,
    }
}

/// 트랩 실행
pub fn execute_trap(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Trap { ball_speed, ball_height } = &action.action_type else {
        debug_assert!(false, "execute_trap called with non-Trap action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_trap called with non-Trap action".to_string(),
        };
    };

    let stats = &ctx.player_stats[action.player_idx];
    // v10 FIX: ctx.player_positions는 이미 미터 좌표
    let player_pos_m = ctx.player_positions[action.player_idx];
    let trapper_is_home = TeamSide::is_home(action.player_idx);

    // FIX_2601/0106 Phase 1: Treat GK aerial Trap as a "claim/catch" handling surface.
    //
    // This enables rulebook back-pass enforcement without adding a new action type or
    // coupling to DecisionTopology.
    const GK_CATCH_MIN_HEIGHT_M: f32 = 1.0;
    let gk_idx = if trapper_is_home {
        ctx.goalkeeper_indices.0
    } else {
        ctx.goalkeeper_indices.1
    };
    let receiver_is_gk = action.player_idx == gk_idx;
    let is_aerial_arrival = *ball_height >= GK_CATCH_MIN_HEIGHT_M;
    if receiver_is_gk && is_aerial_arrival {
        use crate::engine::coordinates;

        let gk_ctx = if trapper_is_home { &ctx.home_ctx } else { &ctx.away_ctx };
        let gk_pos = Coord10::from_meters(player_pos_m.0, player_pos_m.1).clamp_to_field();
        let in_own_penalty_area = gk_pos.in_own_penalty_area(gk_ctx.attacks_right);

        // Sanction ordering (SSOT, deterministic):
        // 1) Outside own PA: direct FK (Phase 0 rule surface).
        // 2) Inside own PA: if this claim came from a teammate deliberate kick (v1 proxy:
        //    PassStarted metadata to GK) -> indirect FK.
        let last_passer_idx = queue.last_passer_idx;
        let last_receiver_idx = queue.last_pass_receiver_idx;
        if !in_own_penalty_area {
            queue.ball_state = BallState::OutOfPlay {
                restart_type: RestartType::FreeKick,
                position: gk_pos,
                home_team: !trapper_is_home, // opponent receives the free kick
            };
            return ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: action.player_idx,
                last_touch_idx: last_passer_idx,
                is_indirect: false,
                xg: None,
            };
        }

        let throw_in_handling_violation = matches!(
            queue.in_flight_origin,
            Some(InFlightOrigin::ThrowIn { throwing_home }) if throwing_home == trapper_is_home
        );
        if throw_in_handling_violation {
            queue.ball_state = BallState::OutOfPlay {
                restart_type: RestartType::FreeKick,
                position: gk_pos,
                home_team: !trapper_is_home, // opponent receives the free kick
            };
            return ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: action.player_idx,
                last_touch_idx: None,
                is_indirect: true,
                xg: None,
            };
        }

        let back_pass_violation = matches!(
            (last_passer_idx, last_receiver_idx),
            (Some(passer_idx), Some(receiver_idx))
                if receiver_idx == action.player_idx
                    && TeamSide::is_home(passer_idx) == trapper_is_home
        );
        if back_pass_violation {
            queue.ball_state = BallState::OutOfPlay {
                restart_type: RestartType::FreeKick,
                position: gk_pos,
                home_team: !trapper_is_home, // opponent receives the free kick
            };
            return ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: action.player_idx,
                last_touch_idx: last_passer_idx,
                is_indirect: true,
                xg: None,
            };
        }

        // Legal claim: model as a deterministic possession win (no first-touch error).
        queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
        return ActionResult::TrapSuccess { player_idx: action.player_idx };
    }
    let match_modifiers = if trapper_is_home {
        ctx.home_match_modifiers
    } else {
        ctx.away_match_modifiers
    };

    // Pass completion is currently resolved mostly at first touch time (trap quality).
    // Apply `pass_success_mult` only when this Trap is the intended receiver of a pass.
    let is_pass_trap = matches!(
        &queue.ball_state,
        BallState::InFlight {
            intended_receiver: Some(receiver_idx),
            is_shot: false,
            ..
        } if *receiver_idx == action.player_idx
    );
    let first_touch = if is_pass_trap {
        apply_attribute_multiplier(stats.first_touch, match_modifiers.pass_success_mult)
    } else {
        stats.first_touch
    };
    let composure = if is_pass_trap {
        apply_attribute_multiplier(stats.composure, match_modifiers.pass_success_mult)
    } else {
        stats.composure
    };
    let anticipation = if is_pass_trap {
        apply_attribute_multiplier(stats.anticipation, match_modifiers.pass_success_mult)
    } else {
        stats.anticipation
    };

    if action_debug_enabled() && *ball_height >= 1.0 {
        let prev_tick = ctx.current_tick.saturating_sub(1);
        match in_flight_ball_pos_at_tick(&queue.ball_state, prev_tick) {        
            Some((bx, by, bz)) => {
                let dx = bx - player_pos_m.0;
                let dy = by - player_pos_m.1;
                let dist = (dx * dx + dy * dy).sqrt();
                eprintln!(
                    "[AERIAL L3] tick={} prev_tick={} action=Trap ball=({:.1},{:.1},{:.2}) player=({:.1},{:.1}) dist={:.2}",
                    ctx.current_tick,
                    prev_tick,
                    bx,
                    by,
                    bz,
                    player_pos_m.0,
                    player_pos_m.1,
                    dist
                );
            }
            None => {
                eprintln!(
                    "[AERIAL L3] tick={} prev_tick={} action=Trap ball_state={:?}",
                    ctx.current_tick, prev_tick, queue.ball_state
                );
            }
        }
        match in_flight_ball_pos_at_tick(&queue.ball_state, ctx.current_tick) {
            Some((bx, by, bz)) => {
                let dx = bx - player_pos_m.0;
                let dy = by - player_pos_m.1;
                let dist = (dx * dx + dy * dy).sqrt();
                eprintln!(
                    "[AERIAL L4] tick={} action=Trap ball=({:.1},{:.1},{:.2}) player=({:.1},{:.1}) dist={:.2} ball_height_param={:.2}",
                    ctx.current_tick,
                    bx,
                    by,
                    bz,
                    player_pos_m.0,
                    player_pos_m.1,
                    dist,
                    ball_height
                );
            }
            None => {
                eprintln!(
                    "[AERIAL L4] tick={} action=Trap ball_state={:?} ball_height_param={:.2}",
                    ctx.current_tick, queue.ball_state, ball_height
                );
            }
        }
    }

    // P10-13: ExecutionError 시스템으로 First Touch 품질 결정

    use crate::engine::execution_error::{
        apply_error_for_first_touch, sample_execution_error, ActionKind, ErrorContext,
        FirstTouchQuality,
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    // 압박 계산: 근처 상대 선수 거리 기반
    // v10 FIX: 이중 변환 제거
    let opponent_range = TeamSide::opponent_range(action.player_idx);
    let nearby_opponents: Vec<(f32, f32)> = opponent_range
        .filter_map(|i| {
            let opp_pos_m = ctx.player_positions[i];
            let dist = ((opp_pos_m.0 - player_pos_m.0).powi(2)
                + (opp_pos_m.1 - player_pos_m.1).powi(2))
            .sqrt();
            if dist < 5.0 {
                Some(opp_pos_m)
            } else {
                None
            }
        })
        .collect();

    let pressure = calculate_pressure_simple(player_pos_m, &nearby_opponents);

    // 공 속도/높이에 따른 추가 난이도 (fatigue 파라미터로 사용)
    let ball_difficulty = (*ball_speed / 30.0 + *ball_height / 3.0).min(1.0);

    let decision_quality_mult = crate::fix01::condition_decision_mult(stats.condition_level);

    // ErrorContext 생성
    // FIX_2601/0107: FM meta에서는 concentration 포함
    #[cfg(feature = "fm_meta_attributes")]
    let err_ctx = ErrorContext::new(ActionKind::FirstTouch)
        .with_stats_fm(first_touch, composure, anticipation, stats.concentration)
        .with_context(pressure, ball_difficulty, false)
        .with_decision_quality_mult(decision_quality_mult);
    #[cfg(not(feature = "fm_meta_attributes"))]
    let err_ctx = ErrorContext::new(ActionKind::FirstTouch)
        .with_stats(first_touch, composure, anticipation)
        .with_context(pressure, ball_difficulty, false)
        .with_decision_quality_mult(decision_quality_mult);

    // RNG로 ExecutionError 샘플링
    let mut rng = ChaCha8Rng::seed_from_u64(
        ctx.rng_seed.wrapping_add(ctx.current_tick).wrapping_add(action.player_idx as u64),
    );
    let exec_error = sample_execution_error(&err_ctx, &mut rng);

    // 공의 도착 위치 (현재 비행 상태에서 가져오거나 선수 위치 근처로 가정)
    let ball_pos_m = player_pos_m; // 공이 선수 위치에 도착한다고 가정

    // First Touch 오차 적용
    let (final_ball_pos_m, quality) =
        apply_error_for_first_touch(ball_pos_m, player_pos_m, &exec_error);

    // 품질에 따른 결과 결정
    match quality {
        FirstTouchQuality::Perfect | FirstTouchQuality::Good => {
            // 성공적인 트랩
            queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
            ActionResult::TrapSuccess { player_idx: action.player_idx }
        }
        FirstTouchQuality::Heavy => {
            // 무거운 터치 - 공은 컨트롤하지만 상대가 압박할 기회
            // 일단 성공으로 처리하되, 근처 상대가 있으면 탈취 기회 발생 가능
            if nearby_opponents.is_empty() {
                queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
                ActionResult::TrapSuccess { player_idx: action.player_idx }
            } else {
                // FIX_2601/0107: FM meta에서는 teamwork가 루즈볼 확률 감소
                #[cfg(feature = "fm_meta_attributes")]
                let loose_threshold = {
                    use crate::engine::match_sim::attribute_calc::teamwork_reception_bonus;
                    let bonus = teamwork_reception_bonus(stats.teamwork as f32);
                    (0.3 - bonus).max(0.1) // 최소 10%는 유지
                };
                #[cfg(not(feature = "fm_meta_attributes"))]
                let loose_threshold = 0.3;

                let loose_roll =
                    simple_random(ctx.rng_seed, ctx.current_tick, action.player_idx + 500);
                if loose_roll < loose_threshold {
                    if let Some(handball) = maybe_non_gk_handball_on_aerial_trap_fail(
                        ctx,
                        queue,
                        action.player_idx,
                        trapper_is_home,
                        stats,
                        pressure,
                        *ball_height,
                        is_pass_trap,
                    ) {
                        handball
                    } else {
                        // FIX_2601: Coord10/Vel10 사용
                        let loose_pos =
                            Coord10::from_meters(final_ball_pos_m.0, final_ball_pos_m.1);
                        let vel_x = (final_ball_pos_m.0 - player_pos_m.0) * 0.5;
                        let vel_y = (final_ball_pos_m.1 - player_pos_m.1) * 0.5;
                        queue.ball_state = BallState::Loose {
                            position: loose_pos,
                            velocity: Vel10::from_mps(vel_x, vel_y),
                        };
                        ActionResult::TrapFailed {
                            player_idx: action.player_idx,
                            loose_ball_pos: loose_pos,
                        }
                    }
                } else {
                    queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
                    ActionResult::TrapSuccess { player_idx: action.player_idx }
                }
            }
        }
        FirstTouchQuality::Loose => {
            if let Some(handball) = maybe_non_gk_handball_on_aerial_trap_fail(
                ctx,
                queue,
                action.player_idx,
                trapper_is_home,
                stats,
                pressure,
                *ball_height,
                is_pass_trap,
            ) {
                handball
            } else {
                // 루즈볼 - 공이 멀리 튀어나감 - FIX_2601: Coord10/Vel10 사용
                let loose_pos =
                    Coord10::from_meters(final_ball_pos_m.0, final_ball_pos_m.1);
                let vel_x = (final_ball_pos_m.0 - player_pos_m.0) * 2.0;
                let vel_y = (final_ball_pos_m.1 - player_pos_m.1) * 2.0;

                queue.ball_state = BallState::Loose {
                    position: loose_pos,
                    velocity: Vel10::from_mps(vel_x, vel_y),
                };

                ActionResult::TrapFailed { player_idx: action.player_idx, loose_ball_pos: loose_pos }
            }
        }
    }
}

/// 드리블/운반 실행
/// - aggressive: false → Carry (운반) - 수비수 없는 공간으로 이동, 통계 미기록
/// - aggressive: true → Take-on (돌파) - 수비수 제치기 시도, '드리블' 통계에 기록
pub fn execute_dribble(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Dribble { direction, aggressive } = &action.action_type else {
        debug_assert!(false, "execute_dribble called with non-Dribble action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_dribble called with non-Dribble action".to_string(),
        };
    };

    let stats = &ctx.player_stats[action.player_idx];
    let player_pos = ctx.player_positions[action.player_idx];

    // 이동 거리 (공격적이면 더 멀리)
    let move_distance = if *aggressive { 5.0 } else { 3.0 };

    // 정규화된 방향
    let dir_len = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
    let norm_dir =
        if dir_len > 0.01 { (direction.0 / dir_len, direction.1 / dir_len) } else { (1.0, 0.0) };

    let new_pos_m =
        (player_pos.0 + norm_dir.0 * move_distance, player_pos.1 + norm_dir.1 * move_distance);
    let new_pos = Coord10::from_meters(new_pos_m.0, new_pos_m.1);

    // 근처 상대 선수 체크
    let nearby_opponent =
        find_nearest_opponent(player_pos, &ctx.player_positions, action.player_idx);

    // Carry (운반) - 수비수가 멀거나 공격적이지 않은 경우
    if !*aggressive {
        // 단순 운반 - 통계 미기록
        queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
        return ActionResult::CarryComplete {
            player_idx: action.player_idx,
            new_position: new_pos,
        };
    }

    // Take-on (돌파) - 공격적인 드리블 시도
    if let Some((opponent_idx, opponent_dist)) = nearby_opponent {
        // 수비수가 2m 이내에 있으면 실제 돌파 대결
        if opponent_dist < 2.0 {
            let opponent_stats = &ctx.player_stats[opponent_idx];

            // FIX_2601/0107: FM meta 사용 시 dribble_success_prob_fm_meta로 계산
            #[cfg(feature = "fm_meta_attributes")]
            let success_rate = {
                use crate::engine::phase_action::dribble::{
                    dribble_success_prob_fm_meta, DribbleSkillsFM,
                };
                let attacker_skills = DribbleSkillsFM {
                    dribbling: stats.dribbling,
                    agility: stats.agility,
                    pace: stats.pace,
                    acceleration: stats.acceleration,
                    balance: stats.balance,
                    flair: stats.flair,
                    anticipation: stats.anticipation,
                };
                dribble_success_prob_fm_meta(
                    &attacker_skills,
                    opponent_stats.tackling as f32,
                    opponent_stats.anticipation as f32,
                    opponent_stats.pace as f32,
                    0.0, // pressure (can be enhanced later)
                )
            };
            #[cfg(not(feature = "fm_meta_attributes"))]
            let success_rate = {
                let dribble_skill = stats.dribbling as f32 + stats.agility as f32 * 0.5;
                let tackle_skill =
                    opponent_stats.tackling as f32 + opponent_stats.anticipation as f32 * 0.5;
                dribble_skill / (dribble_skill + tackle_skill)
            };

            let random = simple_random(ctx.rng_seed, ctx.current_tick, action.player_idx + 100);

            if random > success_rate {
                // 태클당함 (돌파 실패)
                queue.ball_state = BallState::Controlled { owner_idx: opponent_idx };
                return ActionResult::DribbleTackled {
                    player_idx: action.player_idx,
                    tackler_idx: opponent_idx,
                };
            }

            // 돌파 성공 - 수비수를 제침
            queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
            return ActionResult::TakeOnComplete {
                player_idx: action.player_idx,
                new_position: new_pos,
                beaten_defender_idx: Some(opponent_idx),
            };
        }
    }

    // 공격적 드리블이지만 수비수가 없음 → Carry로 처리 (운반)
    // 이 경우는 "돌파 의도였지만 수비수가 없어서 그냥 운반"
    queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
    ActionResult::CarryComplete { player_idx: action.player_idx, new_position: new_pos }
}

/// 슛 실행
pub fn execute_shot(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Shot { power, target: _target } = &action.action_type else {
        debug_assert!(false, "execute_shot called with non-Shot action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_shot called with non-Shot action".to_string(),
        };
    };

    let stats = &ctx.player_stats[action.player_idx];
    // v10 FIX: ctx.player_positions는 이미 미터 좌표 (player_positions_in_meters()에서 변환됨)
    let player_pos_m = ctx.player_positions[action.player_idx];
    // BallState.from_pos는 정규화 좌표가 필요하므로 역변환
    use crate::engine::coordinates;
    let _player_pos_norm = coordinates::to_normalized(player_pos_m);

    // 골대 위치 (미터 좌표)
    // FIX_2601/0110: DirectionContext 사용 (하프타임 방향 전환 반영)
    let shooter_is_home = TeamSide::is_home(action.player_idx);
    let dir_ctx = if shooter_is_home { &ctx.home_ctx } else { &ctx.away_ctx };
    let defending_ctx = if shooter_is_home { &ctx.away_ctx } else { &ctx.home_ctx };
    let match_modifiers = if shooter_is_home {
        ctx.home_match_modifiers
    } else {
        ctx.away_match_modifiers
    };
    let goal_x = dir_ctx.opponent_goal_x() * field::LENGTH_M; // normalized → meters
    let goal_center_y = field::CENTER_Y;

    let dx = goal_x - player_pos_m.0;
    let dy = goal_center_y - player_pos_m.1;
    let distance = (dx * dx + dy * dy).sqrt();

    // xG 계산 using phase_action::calculate_xg_with_target for P0 Goal Contract compliance
    // goal_x is explicitly determined by attacking team's goal
    use crate::engine::phase_action::calculate_xg_with_target;
    let xg = calculate_xg_with_target(
        player_pos_m,
        ShotType::Normal, // P0: ShotType is now local to action_queue
        0,                // defenders_blocking (simplified)
        false,            // is_one_on_one
        stats.finishing,
        Some(goal_x), // P0: 공격 골대 명시적 지정
    );

    // GK 위치를 고려한 슛 타겟 결정
    // 슈터는 GK에서 먼 쪽 구석을 노림
    let gk_idx = TeamSide::opponent_gk(action.player_idx);
    // v10 FIX: ctx.player_positions는 이미 미터 좌표 (이중 변환 제거)
    let gk_pos_m = ctx.player_positions[gk_idx];

    // GK의 y 위치 기준으로 반대쪽 구석 선택
    let goal_half_width = 3.66;
    let intended_target_y = if gk_pos_m.1 > goal_center_y {
        // GK가 위쪽에 있으면 아래쪽 구석으로
        goal_center_y - goal_half_width * 0.7
    } else {
        // GK가 아래쪽에 있으면 위쪽 구석으로
        goal_center_y + goal_half_width * 0.7
    };
    // 의도한 타겟 높이는 HeightProfile 기반으로 계산

    // P10-13: ExecutionError 시스템 적용
    use crate::engine::execution_error::{
        apply_error_for_shot, sample_execution_error, ActionKind, ErrorContext,
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    // 압박 계산: 근처 상대 선수 거리 기반
    // v10 FIX: ctx.player_positions는 이미 미터 좌표 (이중 변환 제거)
    let opponent_range = if shooter_is_home { 11..22 } else { 0..11 };
    let nearby_opponents: Vec<(f32, f32)> = opponent_range
        .filter_map(|i| {
            let opp_pos_m = ctx.player_positions[i];
            let dist = ((opp_pos_m.0 - player_pos_m.0).powi(2)
                + (opp_pos_m.1 - player_pos_m.1).powi(2))
            .sqrt();
            if dist < 5.0 {
                Some(opp_pos_m)
            } else {
                None
            }
        })
        .collect();

    let pressure = calculate_pressure_simple(player_pos_m, &nearby_opponents);

    // 롱슛 여부에 따른 기술 능력치 결정
    let is_long_shot = distance > 25.0;
    let tech_skill = if is_long_shot { stats.long_shots } else { stats.finishing };
    let tech_skill = apply_attribute_multiplier(tech_skill, match_modifiers.shot_accuracy_mult);
    let composure = apply_attribute_multiplier(stats.composure, match_modifiers.shot_accuracy_mult);
    let anticipation =
        apply_attribute_multiplier(stats.anticipation, match_modifiers.shot_accuracy_mult);

    // FIX_2601/0117: Extract ball height dynamically from BallState
    let (ball_airborne, ball_height_m) = match &queue.ball_state {
        BallState::InFlight {
            start_tick,
            end_tick,
            height_profile,
            lift_ratio,
            ..
        } => {
            // Calculate flight progress
            let duration = end_tick.saturating_sub(*start_tick) as f32;
            let elapsed = ctx.current_tick.saturating_sub(*start_tick) as f32;
            let t = if duration > 0.0 { (elapsed / duration).clamp(0.0, 1.0) } else { 0.0 };

            // Height follows parabola: h = 4 * max_height * t * (1 - t)
            let max_height = height_profile.height_cap_m() * lift_ratio;
            let height = 4.0 * max_height * t * (1.0 - t);
            (height > 0.3, height) // airborne if height > 30cm
        }
        _ => (false, 0.0), // Ground ball
    };

    // ActionModel Integration: 상황 기반 Intent/Technique 선택
    let shot_ctx = ShotContext {
        distance_to_goal: distance,
        angle_to_goal: (dy.atan2(dx).to_degrees()).abs(),
        defenders_ahead: nearby_opponents.len() as u8,
        gk_distance: ((gk_pos_m.0 - player_pos_m.0).powi(2)
            + (gk_pos_m.1 - player_pos_m.1).powi(2))
        .sqrt(),
        is_one_on_one: nearby_opponents.is_empty() && distance < 20.0,
        ball_airborne,      // FIX_2601/0117: Dynamic
        ball_height: ball_height_m,  // FIX_2601/0117: Dynamic
        time_pressure: pressure,
    };
    let shooter_skills = ShooterSkills {
        finishing: stats.finishing,
        long_shots: stats.long_shots,
        volleys: stats.heading, // volley 대용
        composure: stats.composure,
    };

    // FIX_2601/0117: Intent 추론 - Aerial for airborne ball (headers/volleys)
    let intent = if ball_airborne && ball_height_m > 1.5 {
        ShotIntent::Aerial // Header or Volley
    } else if *power > 0.8 || distance > 25.0 {
        ShotIntent::Power
    } else {
        ShotIntent::Place
    };

    // Technique 선택 및 물리 파라미터 적용
    let technique = choose_shot_technique(intent, &shot_ctx, &shooter_skills);
    let physics = technique.physics_params();

    // HeightProfile + lift_ratio (Chip => Lob, Power => Flat)
    let height_profile = match technique {
        ShotTechnique::Chip => HeightProfile::Lob,
        ShotTechnique::Power => HeightProfile::Flat,
        _ => HeightProfile::Arc,
    };
    let lift_intent = match height_profile {
        HeightProfile::Flat => 0.0,
        _ => 1.0,
    };
    let skill = (stats.technique as f32 + stats.finishing as f32) / 200.0;
    let lift_ratio = compute_lift_ratio(lift_intent, skill, pressure);

    let base_speed = (25.0 + power * 10.0) * match_modifiers.shot_power_mult;
    let flight_time = if base_speed > 0.0 { distance / base_speed } else { 0.0 };
    let vz = height_profile.vz_cap_mps() * lift_ratio;
    let intended_height =
        (vz * flight_time - 0.5 * physics_ball::GRAVITY * flight_time * flight_time).max(0.0);

    let decision_quality_mult = crate::fix01::condition_decision_mult(stats.condition_level);

    // ErrorContext 생성
    // FIX_2601/0107: FM meta에서는 concentration 포함
    #[cfg(feature = "fm_meta_attributes")]
    let err_ctx = ErrorContext::new(ActionKind::Shot)
        .with_stats_fm(tech_skill, composure, anticipation, stats.concentration)
        .with_context(pressure, 0.0, false)
        .with_decision_quality_mult(decision_quality_mult);
    #[cfg(not(feature = "fm_meta_attributes"))]
    let err_ctx = ErrorContext::new(ActionKind::Shot)
        .with_stats(tech_skill, composure, anticipation)
        .with_context(pressure, 0.0, false)
        .with_decision_quality_mult(decision_quality_mult);

    // RNG로 ExecutionError 샘플링
    let mut rng = ChaCha8Rng::seed_from_u64(
        ctx.rng_seed.wrapping_add(ctx.current_tick).wrapping_add(action.player_idx as u64),
    );
    let exec_error = sample_execution_error(&err_ctx, &mut rng);

    // 의도한 타겟에 오차 적용
    let intended_target = (goal_x, intended_target_y);
    let (actual_target_2d, actual_height) =
        apply_error_for_shot(player_pos_m, intended_target, intended_height, &exec_error);
    #[cfg(debug_assertions)]
    if action_debug_enabled() {
        eprintln!(
            "[SHOT-ERROR] from=({:.1},{:.1}) intended=({:.1},{:.1}) actual=({:.1},{:.1}) err.dir={:.1}deg err.dist={:.3}",
            player_pos_m.0,
            player_pos_m.1,
            intended_target.0,
            intended_target.1,
            actual_target_2d.0,
            actual_target_2d.1,
            exec_error.dir_angle_deg,
            exec_error.dist_factor
        );
    }

    // 골대 범위 체크 (y + height)
    let goal_y_min = goal::Y_MIN;
    let goal_y_max = goal::Y_MAX;
    let goal_height = goal::HEIGHT_M;
    let is_on_target = actual_target_2d.1 >= goal_y_min
        && actual_target_2d.1 <= goal_y_max
        && actual_height >= 0.0
        && actual_height <= goal_height;
    if !is_on_target {
        let goal_kick_x =
            defending_ctx.own_goal_x() * field::LENGTH_M + defending_ctx.forward_offset(6.0);
        queue.ball_state = BallState::OutOfPlay {
            restart_type: RestartType::GoalKick,
            position: Coord10::from_meters(goal_kick_x, field::CENTER_Y),
            home_team: !shooter_is_home,
        };
        return ActionResult::ShotMissed { shooter_idx: action.player_idx, xg };
    }

    // 최종 타겟 (빗나가도 실제 궤적 유지)
    let _final_target = actual_target_2d;

    // 공 비행 상태 (ActionModel 물리 적용)
    let ball_speed = base_speed * physics.speed_mult;
    let flight_ticks = ((distance / ball_speed) * TICKS_PER_SECOND as f32).ceil() as u64;
    let _arrival_tick = ctx.current_tick + flight_ticks.max(2);

    // Contract v1: OutcomeSet Sampling for Shot Resolution
    // Instead of using ActionResult::ShotTaken immediately, we resolve the outcome here.

    // 1) Outcome 후보 구성
    let mut candidates: Vec<(ShotOutcome, WeightBreakdown)> = Vec::new();
    let neutral = WeightBreakdown::neutral();

    // xG를 기반으로 가중치 설정 (WeightBreakdown.context 활용)
    // Goal: xG가 높을수록 가중치 증가
    let mut w_goal = neutral;

    // FIX_2601/0107: FM meta 사용 시 shot_base_success_prob_fm_meta로 계산
    #[cfg(feature = "fm_meta_attributes")]
    {
        use crate::engine::phase_action::shot::{shot_base_success_prob_fm_meta, ShooterSkillsFM};
        let shooter_skills_fm = ShooterSkillsFM {
            base: shooter_skills,
            technique: stats.technique,
            pace: stats.pace,
            balance: stats.balance,
            concentration: stats.concentration,
        };
        let fm_success_prob = shot_base_success_prob_fm_meta(technique, &shot_ctx, &shooter_skills_fm);
        // FIX: 3.0 → 0.8 (골 폭발 방지 - 세이브가 더 많이 발생하도록)
        w_goal.context *= (fm_success_prob * 0.8).clamp(0.05, 1.0);
    }
    #[cfg(not(feature = "fm_meta_attributes"))]
    {
        // FIX_2601/0116: 골 전환율 상향 (xG 대비 실제 골 30% → 95% 목표)
        // 기존: xG ~0.12 * 1.5 = 0.18 → 경쟁에서 밀림
        // 수정: xG ~0.12 * 3.0 = 0.36 → Goal 가중치 상향
        w_goal.context *= (xg * 3.0).clamp(0.05, 1.0);
    }

    // FIX_2601/0109: GK 능력치 기반 세이브 확률 (통합 함수 사용)
    use crate::engine::match_sim::attribute_calc::calculate_gk_save_prob_unified;
    let gk_stats = &ctx.player_stats[gk_idx];
    let gk_save_prob = calculate_gk_save_prob_unified(
        gk_stats.reflexes as f32,
        gk_stats.positioning as f32,
        gk_stats.handling as f32,
        gk_stats.diving as f32,
        distance,
        ball_speed,
        actual_height,
        shot_ctx.is_one_on_one,
    );
    let mut w_saved = neutral;
    // FIX_2601/0116: GK 세이브 가중치 하향 (골 전환율 상향)
    // 기존: gk_save_prob * (1.5 - xg * 0.5) → 0.4 * 1.44 = 0.58
    // 수정: gk_save_prob * (1.2 - xg * 0.5) → 0.4 * 1.14 = 0.46
    w_saved.context *= gk_save_prob * (1.2 - xg * 0.5);
    if actual_height > aerial::GK_CATCH_MAX_M {
        w_saved.context *= 0.3; // 높은 슛은 세이브 어려움
    }
    // FIX_2601/0114: 장거리 슈팅은 GK에게 도달하기 전에 빗나갈 확률 높음
    if distance > 25.0 {
        w_saved.context *= 0.5;  // 장거리 슈팅은 세이브보다 빗나감이 많음
    }

    // FIX_2601/0114: Blocked - 압박 + 거리에 따라 증가
    // 멀리서 쏘면 수비 블록당할 확률 증가
    let mut w_blocked = neutral;
    let blocked_base = (pressure * 2.0).clamp(0.1, 2.0);
    let blocked_distance_bonus = if distance > 20.0 {
        ((distance - 20.0) / 15.0).min(1.0) * 0.5  // 20-35m: +0 ~ +0.5
    } else {
        0.0
    };
    w_blocked.context *= blocked_base + blocked_distance_bonus;

    // FIX_2601/0115 v4: OnTarget - 최종 조정 (목표: 35-40% 정확도)
    // v3: 32% → v4: 35-40% 목표 (페널티 박스 기준으로 올림)
    let mut w_on = neutral;
    let on_target_factor = if distance < 16.0 {
        0.65  // 페널티 박스 내: 50-55% 정확도 목표 (v3: 0.55)
    } else if distance < 25.0 {
        // 16-25m: 0.65 → 0.25로 감소 (v3: 0.55 → 0.20)
        let decay = (distance - 16.0) / 9.0 * 0.40;
        0.65 - decay
    } else {
        // 25m+: 낮음 (v3: 0.12)
        0.15
    };
    w_on.context *= on_target_factor;

    // FIX_2601/0116: OffTarget 가중치 하향 (골 전환율 상향 위해)
    // 기존: 25m+ → 3.0 (Goal 대비 압도적으로 높음)
    // 수정: 25m+ → 1.5 (Goal과 균형 맞춤)
    let mut w_off = neutral;
    let off_target_factor = if distance < 16.0 {
        0.60  // 페널티 박스 내: 빗나감 비율 (v4: 0.85 → 0.60)
    } else if distance < 25.0 {
        // 16-25m: 0.60 → 1.2로 증가 (v4: 0.85 → 2.5)
        let growth = (distance - 16.0) / 9.0 * 0.60;
        0.60 + growth
    } else {
        // 25m+: 중간 (v4: 3.0 → 1.5)
        1.5
    };
    w_off.context *= off_target_factor;

    candidates.push((ShotOutcome::Goal, w_goal));
    candidates.push((ShotOutcome::Saved, w_saved));
    candidates.push((ShotOutcome::Blocked, w_blocked));
    candidates.push((ShotOutcome::OnTarget, w_on));
    candidates.push((ShotOutcome::OffTarget, w_off));

    // 2) OutcomeSet 1회 샘플링
    let outcome =
        select_outcome_softmax(&candidates, 1.0, &mut rng).unwrap_or(ShotOutcome::OffTarget);

    // 3) 결과에 따라 ActionResult 생성
    // DEBUG: Track goal distribution by team
    #[cfg(debug_assertions)]
    if action_debug_enabled() {
        let is_home = TeamSide::is_home(action.player_idx);
        eprintln!(
            "[SHOT_OUTCOME] player={} is_home={} outcome={:?} xg={:.3} dist={:.1}m",
            action.player_idx,
            is_home,
            outcome,
            xg,
            distance
        );
    }
    match outcome {
        ShotOutcome::Goal => {
            // FIX_2601/0115b: xG 포함
            ActionResult::GoalScored { scorer_idx: action.player_idx, assist_idx: None, xg }
        }
        ShotOutcome::Saved => {
            // SaveMade requires GK index. Simplification: Assume opp GK.
            //
            // NOTE: v1에서는 Saved/OnTarget 경로가 "GK가 손으로 처리"를 의미한다.
            // 따라서 GK가 자기 페널티 에어리어 밖이면 핸들링 위반으로 처리한다.
            let gk_pos = Coord10::from_meters(gk_pos_m.0, gk_pos_m.1).clamp_to_field();
            let in_own_penalty_area =
                coordinates::is_in_penalty_area(gk_pos.to_normalized_legacy(), !defending_ctx.attacks_right);
            if !in_own_penalty_area {
                queue.ball_state = BallState::OutOfPlay {
                    restart_type: RestartType::FreeKick,
                    position: gk_pos,
                    home_team: shooter_is_home, // 공격팀이 프리킥
                };
                return ActionResult::GoalkeeperHandlingViolation {
                    goalkeeper_idx: gk_idx,
                    last_touch_idx: Some(action.player_idx),
                    is_indirect: false,
                    xg: Some(xg),
                };
            }
            // FIX_2601/0107: 세이브 시 ball_state를 Controlled로 설정 (자책골 방지)
            // sync_to_ball()이 호출될 때 InFlight 상태가 남아있으면      
            // 공이 골라인 너머로 이동해서 자책골로 처리됨
            queue.ball_state = BallState::Controlled { owner_idx: gk_idx };
            ActionResult::SaveMade {
                goalkeeper_idx: gk_idx,
                save_type: SaveType::Catch, // Default
                shooter_idx: action.player_idx,
                xg, // FIX_2601/0115b
            }
        }
        ShotOutcome::Blocked => {
            // Optional (rare): a blocked shot may be a non-GK handball foul.
            //
            // Keep this strictly executor-level (SSOT) and deterministic. Do not couple
            // to DecisionTopology scoring.
            if let Some(handball) = maybe_non_gk_handball_on_shot_block(
                ctx,
                queue,
                action.player_idx,
                shooter_is_home,
                defending_ctx,
                gk_idx,
                pressure,
                player_pos_m,
                (goal_x, actual_target_2d.1),
            ) {
                handball
            } else {
                let restart_pos = Coord10::from_meters(
                    goal_x,
                    actual_target_2d.1.clamp(0.0, field::WIDTH_M),
                );
                queue.ball_state = BallState::OutOfPlay {
                    restart_type: RestartType::Corner,
                    position: restart_pos,
                    home_team: shooter_is_home,
                };
                ActionResult::ShotMissed { shooter_idx: action.player_idx, xg } // FIX_2601/0115b
            }
         }
         ShotOutcome::OnTarget => {
             // OnTarget = 유효슈팅이지만 골이 아님 (GK가 처리했다고 가정)
             // FIX_2601/0107: ball_state를 Controlled로 설정 (자책골 방지)
             let gk_is_home = TeamSide::is_home(gk_idx);
             let gk_ctx = if gk_is_home { &ctx.home_ctx } else { &ctx.away_ctx };
             let gk_pos = Coord10::from_meters(gk_pos_m.0, gk_pos_m.1).clamp_to_field();
             let in_own_penalty_area = gk_pos.in_own_penalty_area(gk_ctx.attacks_right);
            if !in_own_penalty_area {
                queue.ball_state = BallState::OutOfPlay {
                    restart_type: RestartType::FreeKick,
                    position: gk_pos,
                    home_team: shooter_is_home, // 공격팀이 프리킥
                };
                return ActionResult::GoalkeeperHandlingViolation {
                    goalkeeper_idx: gk_idx,
                    last_touch_idx: Some(action.player_idx),
                    is_indirect: false,
                    xg: Some(xg),
                };
            }
            queue.ball_state = BallState::Controlled { owner_idx: gk_idx };
            ActionResult::ShotTaken {
                shooter_idx: action.player_idx,
                xg,
                target: Coord10::default(), // Placeholder
            }
        }
        ShotOutcome::OffTarget => {
            let goal_kick_x =
                defending_ctx.own_goal_x() * field::LENGTH_M + defending_ctx.forward_offset(6.0);
            let goal_kick_pos = Coord10::from_meters(goal_kick_x, field::CENTER_Y);
            queue.ball_state = BallState::OutOfPlay {
                restart_type: RestartType::GoalKick,
                position: goal_kick_pos,
                home_team: !shooter_is_home,
            };
            ActionResult::ShotMissed { shooter_idx: action.player_idx, xg } // FIX_2601/0115b
        }
    }
}

/// 골키퍼 세이브 실행
pub fn execute_save(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
    _shot_xg_param: f32, // 레거시 파라미터 (사용 안 함)
) -> ActionResult {
    let ActionType::Save { direction } = &action.action_type else {
        debug_assert!(false, "execute_save called with non-Save action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_save called with non-Save action".to_string(),
        };
    };

    let stats = &ctx.player_stats[action.player_idx];

    // 큐에 저장된 실제 xG 사용 (없으면 기본값 0.15)
    let shot_xg = queue.last_shot_xg.unwrap_or(0.15);
    let shooter_idx = queue.last_shooter_idx.unwrap_or(10); // 기본값: Home CF

    // 슈터 정보 사용 후 초기화
    queue.last_shot_xg = None;
    queue.last_shooter_idx = None;

    // 골키퍼 위치 (미터 좌표)
    // v10 FIX: ctx.player_positions는 이미 미터 좌표 (이중 변환 제거)

    let gk_pos_m = ctx.player_positions[action.player_idx];

    // 슛 타겟 (direction은 미터 좌표로 가정)
    let shot_target = *direction;

    // 슛온타겟 체크 (골대 y범위: 30.5 ~ 37.5)
    let is_on_target = shot_target.1 >= 30.5 && shot_target.1 <= 37.5;

    if !is_on_target {
        // 빗나간 슛 - 골킥
        if action_debug_enabled() {
            eprintln!(
                "[SHOT-MISS] off-target! y={:.2} (should be 30.5~37.5)",
                shot_target.1
            );
        }
        let scorer_is_home = TeamSide::is_home(shooter_idx);
        let defending_ctx = if scorer_is_home { &ctx.away_ctx } else { &ctx.home_ctx };
        let goal_kick_x =
            defending_ctx.own_goal_x() * field::LENGTH_M + defending_ctx.forward_offset(6.0);
        // FIX_2601: Coord10 사용
        let goal_kick_pos = Coord10::from_meters(goal_kick_x, field::CENTER_Y);
        queue.ball_state = BallState::OutOfPlay {
            restart_type: RestartType::GoalKick,
            position: goal_kick_pos,
            home_team: !scorer_is_home, // 공격 반대팀이 골킥
        };

        return ActionResult::ShotMissed { shooter_idx, xg: shot_xg }; // FIX_2601/0115b
    }

    // ========================================================================
    // 새로운 GK 위치 기반 세이브 확률 계산
    // ========================================================================
    //
    // 슛 궤적: shooter_pos -> shot_target
    // GK가 이 궤적에 얼마나 가까운지 (reach 가능한지)가 핵심
    //
    // 1) 슛 비행 시간 계산
    // 슈터 위치 (미터 좌표) 필요 - ball_state에서 from_pos 가져오기
    // FIX_2601: Coord10.to_meters() 사용
    let shooter_pos_m = match &queue.ball_state {
        BallState::InFlight { from_pos, .. } => from_pos.to_meters(),
        _ => gk_pos_m, // fallback
    };
    let shot_distance = ((shot_target.0 - shooter_pos_m.0).powi(2)
        + (shot_target.1 - shooter_pos_m.1).powi(2))
    .sqrt();
    let shot_speed = 25.0; // m/s (평균 슛 속도)
    let flight_time = shot_distance / shot_speed; // 초

    // 2) GK가 슛 궤적까지의 거리 계산 (point to line distance)
    let dist_to_trajectory = distance_point_to_line(gk_pos_m, shooter_pos_m, shot_target);

    // 3) GK 도달 가능 거리 계산 (반응시간 + 다이빙)
    // reflexes가 높을수록 반응이 빠름 → 더 멀리 도달 가능
    let base_dive_range = 2.5; // 기본 다이빙 거리 (m)
    let reflexes_factor = stats.reflexes as f32 / 100.0; // 0.0 ~ 1.0
    let reaction_time = 0.3 - reflexes_factor * 0.15; // 0.15s ~ 0.30s
    let available_time = (flight_time - reaction_time).max(0.0);
    let dive_speed = 4.0; // m/s
    let gk_reach = base_dive_range + available_time * dive_speed;

    // 4) 위치 기반 세이브 가능성 (pos_factor)
    let pos_factor = if dist_to_trajectory <= gk_reach {
        // 궤적에 도달 가능 - 거리가 가까울수록 세이브 쉬움
        let reach_ratio = 1.0 - (dist_to_trajectory / gk_reach);
        reach_ratio.clamp(0.0, 1.0)
    } else {
        // 궤적에 도달 불가 - 세이브 거의 불가능
        0.05 // 5% 최소 찬스 (예측 성공 등)
    };

    // 5) 슛 타겟의 구석 정도 (corner factor) - 구석으로 갈수록 막기 어려움
    let goal_center_y = field::CENTER_Y;
    let goal_half_width = 3.66; // 골대 반폭 (7.32m / 2)
    let corner_offset = (shot_target.1 - goal_center_y).abs() / goal_half_width;
    let corner_difficulty = corner_offset.clamp(0.0, 1.0) * 0.3; // 최대 30% 감소

    // 6) 최종 세이브 확률
    // v10 FIX: 실제 축구 통계 기반으로 조정
    // - 프리미어리그 평균: 온타겟 슛 중 약 67%가 세이브됨
    // - 전환율: 슛→골 약 10%, 온타겟슛→골 약 33%
    // GK 능력치 계산
    #[cfg(feature = "fm_meta_attributes")]
    let gk_skill = {
        use crate::engine::match_sim::attribute_calc::gk_save_skill_fm_meta;
        gk_save_skill_fm_meta(
            stats.reflexes as f32,
            stats.handling as f32,
            stats.positioning as f32,
            stats.diving as f32,
        ) / 100.0 // 0-100 → 0-1 정규화
    };
    #[cfg(not(feature = "fm_meta_attributes"))]
    let gk_skill = (stats.reflexes as f32 + stats.handling as f32) / 200.0; // 0.0 ~ 1.0

    // 기본 세이브 확률 (GK 능력 기반)
    // 낮은 능력 GK (skill=0.3): 55%, 평균 GK (skill=0.5): 65%, 엘리트 GK (skill=0.8): 80%
    let base_save = 0.45 + gk_skill * 0.45; // 0.45 ~ 0.90

    // 위치 보정: GK가 궤적에서 멀면 세이브 어려움
    // pos_factor가 낮으면(멀면) 패널티 적용
    // v10: 패널티를 완화하여 GK가 더 많이 막을 수 있도록
    let position_penalty = if pos_factor < 0.3 {
        // GK가 도달 불가능할 정도로 멀면 큰 패널티
        0.30
    } else if pos_factor < 0.5 {
        // 멀지만 가능한 거리
        (0.5 - pos_factor) * 0.40
    } else {
        // 도달 가능 범위 내
        0.0
    };

    // 구석 패널티: 이미 계산됨 (최대 30%)
    // v10: 구석 패널티도 완화 (최대 20%)
    let corner_difficulty_adjusted = corner_difficulty * 0.67; // 최대 20%

    // xG 패널티: xG가 높을수록 세이브 어려움 (최대 20%)
    let xg_penalty = shot_xg * 0.40; // xG 0.5 = 20% 패널티

    // 최종 세이브 확률
    let save_prob =
        (base_save - position_penalty - corner_difficulty_adjusted - xg_penalty).clamp(0.15, 0.85);

    let random = simple_random(ctx.rng_seed, ctx.current_tick, action.player_idx + 200);

    #[cfg(debug_assertions)]
    if action_debug_enabled() {
        eprintln!(
            "[SAVE] xG={:.3}, save_prob={:.3}, random={:.3}, on_target={}",
            shot_xg, save_prob, random, is_on_target
        );
    }

    if random < save_prob {
        // 세이브 성공
        // GK가 공을 "손으로 처리"한 것으로 간주되는 경로이므로,
        // 자기 페널티 에어리어 밖이면 핸들링 위반(직접 프리킥)으로 처리.
        use crate::engine::coordinates;
        let gk_is_home = TeamSide::is_home(action.player_idx);
        let gk_ctx = if gk_is_home { &ctx.home_ctx } else { &ctx.away_ctx };
        let gk_pos = Coord10::from_meters(gk_pos_m.0, gk_pos_m.1).clamp_to_field();
        let in_own_penalty_area =
            coordinates::is_in_penalty_area(gk_pos.to_normalized_legacy(), !gk_ctx.attacks_right);
        if !in_own_penalty_area {
            let shooter_is_home = TeamSide::is_home(shooter_idx);
            queue.ball_state = BallState::OutOfPlay {
                restart_type: RestartType::FreeKick,
                position: gk_pos,
                home_team: shooter_is_home, // 공격팀이 프리킥
            };
            return ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: action.player_idx,
                last_touch_idx: Some(shooter_idx),
                is_indirect: false,
                xg: Some(shot_xg),
            };
        }

        let save_type = if random < 0.3 {
            SaveType::Catch
        } else if random < 0.6 {
            SaveType::Parry
        } else {
            SaveType::Dive
        };

        // P10-13: ExecutionError로 GK 세이브 품질 결정 (펌블/패링 방향)
        use crate::engine::execution_error::{sample_execution_error, ActionKind, ErrorContext};
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        // GK에 대한 압박 계산 (슛 순간의 상황)
        // v10 FIX: ctx.player_positions는 이미 미터 좌표 (이중 변환 제거)
        let opponent_range = TeamSide::opponent_range(action.player_idx);
        let nearby_attackers: Vec<(f32, f32)> = opponent_range
            .filter_map(|i| {
                let opp_pos_m = ctx.player_positions[i];
                let dist = ((opp_pos_m.0 - gk_pos_m.0).powi(2)
                    + (opp_pos_m.1 - gk_pos_m.1).powi(2))
                .sqrt();
                if dist < 6.0 {
                    Some(opp_pos_m)
                } else {
                    None
                }
            })
            .collect();

        let pressure = calculate_pressure_simple(gk_pos_m, &nearby_attackers);

        let decision_quality_mult = crate::fix01::condition_decision_mult(stats.condition_level);

        // ErrorContext for Save action
        // FIX_2601/0107: FM meta에서는 concentration 포함
        #[cfg(feature = "fm_meta_attributes")]
        let err_ctx = ErrorContext::new(ActionKind::Save)
            .with_stats_fm(stats.handling, stats.composure, stats.anticipation, stats.concentration)
            .with_context(pressure, 0.0, false)
            .with_decision_quality_mult(decision_quality_mult);
        #[cfg(not(feature = "fm_meta_attributes"))]
        let err_ctx = ErrorContext::new(ActionKind::Save)
            .with_stats(stats.handling, stats.composure, stats.anticipation)
            .with_context(pressure, 0.0, false)
            .with_decision_quality_mult(decision_quality_mult);

        // RNG로 ExecutionError 샘플링
        let mut rng = ChaCha8Rng::seed_from_u64(
            ctx.rng_seed
                .wrapping_add(ctx.current_tick)
                .wrapping_add(action.player_idx as u64 + 100),
        );
        let exec_error = sample_execution_error(&err_ctx, &mut rng);

        // 세이브 후 공 상태 (ExecutionError 적용)
        match save_type {
            SaveType::Catch => {
                // 펌블 체크: dist_factor가 1.0에서 멀어질수록 펌블 확률 증가
                let fumble_chance = (exec_error.dist_factor - 1.0).abs() * 2.0; // 0.3 dist error → 60% fumble
                let fumble_roll =
                    simple_random(ctx.rng_seed, ctx.current_tick, action.player_idx + 600);

                if fumble_roll < fumble_chance && !nearby_attackers.is_empty() {
                    // 펌블! 공이 루즈볼이 됨
                    let fumble_dir_x = exec_error.dir_angle_deg.to_radians().sin() * 2.0;
                    let fumble_dir_y = exec_error.dir_angle_deg.to_radians().cos() * 2.0;
                    let player_pos = ctx.player_positions[action.player_idx];
                    queue.ball_state = BallState::Loose {
                        position: Coord10::from_meters(player_pos.0, player_pos.1),
                        velocity: Vel10::from_mps(fumble_dir_x, fumble_dir_y),
                    };
                    // 펌블은 Parry로 처리 (통계상)
                    return ActionResult::SaveMade {
                        goalkeeper_idx: action.player_idx,
                        save_type: SaveType::Parry,
                        shooter_idx,
                        xg: shot_xg, // FIX_2601/0115b
                    };
                } else {
                    queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
                }
            }
            _ => {
                // 펀칭/패링 - 루즈볼 (방향에 오차 적용)
                let base_parry_x = direction.0 * -5.0;
                let base_parry_y = direction.1 * -3.0;

                // 오차로 인해 패링 방향이 약간 벗어남
                let rot_rad = exec_error.dir_angle_deg.to_radians() * 0.5; // 절반만 적용
                let cos_r = rot_rad.cos();
                let sin_r = rot_rad.sin();
                let parry_x = base_parry_x * cos_r - base_parry_y * sin_r;
                let parry_y = base_parry_x * sin_r + base_parry_y * cos_r;

                let player_pos = ctx.player_positions[action.player_idx];
                queue.ball_state = BallState::Loose {
                    position: Coord10::from_meters(player_pos.0, player_pos.1),
                    velocity: Vel10::from_mps(
                        parry_x * exec_error.dist_factor,
                        parry_y * exec_error.dist_factor,
                    ),
                };
            }
        }

        ActionResult::SaveMade { goalkeeper_idx: action.player_idx, save_type, shooter_idx, xg: shot_xg } // FIX_2601/0115b
    } else {
        // 골!
        let scorer_is_home = TeamSide::is_home(shooter_idx);
        queue.ball_state = BallState::OutOfPlay {
            restart_type: RestartType::KickOff,
            position: Coord10::CENTER, // 센터 서클
            home_team: !scorer_is_home,                 // 득점하지 않은 팀이 킥오프
        };

        ActionResult::GoalScored { scorer_idx: shooter_idx, assist_idx: None, xg: shot_xg } // FIX_2601/0115b
    }
}

/// 2025-12-11 P2: 헤딩 실행
/// Header action execution - handles both headed shots and headed passes
pub fn execute_header(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Header { target, is_shot } = &action.action_type else {
        debug_assert!(false, "execute_header called with non-Header action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_header called with non-Header action".to_string(),
        };
    };

    let stats = &ctx.player_stats[action.player_idx];
    let player_pos_m = ctx.player_positions[action.player_idx]; // FIX_2601: meters
    let player_pos = Coord10::from_meters(player_pos_m.0, player_pos_m.1); // FIX_2601: Coord10

    // FIX_2601/1120: Use actual ball position for InFlight origin (Pattern B fix)
    // This prevents teleportation when starting a header - the ball should start
    // from its current position, not the player's exact position.
    // - For InFlight: use interpolated position at current tick (headers on aerial balls)
    // - For Controlled: use player position (ball follows player)
    // - For Loose/OutOfPlay: use position from ball_state
    let ball_origin_m = get_ball_position_at_tick(&queue.ball_state, ctx.current_tick, &ctx.player_positions)
        .unwrap_or(player_pos_m);
    let ball_origin = Coord10::from_meters(ball_origin_m.0, ball_origin_m.1);

    if action_debug_enabled() {
        let prev_tick = ctx.current_tick.saturating_sub(1);
        match in_flight_ball_pos_at_tick(&queue.ball_state, prev_tick) {
            Some((bx, by, bz)) => {
                let dx = bx - player_pos_m.0;
                let dy = by - player_pos_m.1;
                let dist = (dx * dx + dy * dy).sqrt();
                eprintln!(
                    "[AERIAL L3] tick={} prev_tick={} action=Header ball=({:.1},{:.1},{:.2}) player=({:.1},{:.1}) dist={:.2}",
                    ctx.current_tick,
                    prev_tick,
                    bx,
                    by,
                    bz,
                    player_pos_m.0,
                    player_pos_m.1,
                    dist
                );
            }
            None => {
                eprintln!(
                    "[AERIAL L3] tick={} prev_tick={} action=Header ball_state={:?}",
                    ctx.current_tick, prev_tick, queue.ball_state
                );
            }
        }
        match in_flight_ball_pos_at_tick(&queue.ball_state, ctx.current_tick) {
            Some((bx, by, bz)) => {
                let dx = bx - player_pos_m.0;
                let dy = by - player_pos_m.1;
                let dist = (dx * dx + dy * dy).sqrt();
                eprintln!(
                    "[AERIAL L4] tick={} action=Header ball=({:.1},{:.1},{:.2}) player=({:.1},{:.1}) dist={:.2}",
                    ctx.current_tick,
                    bx,
                    by,
                    bz,
                    player_pos_m.0,
                    player_pos_m.1,
                    dist
                );
            }
            None => {
                eprintln!(
                    "[AERIAL L4] tick={} action=Header ball_state={:?}",
                    ctx.current_tick, queue.ball_state
                );
            }
        }
    }

    // Calculate header success rate based on heading and jumping stats
    let heading_skill = stats.heading as f32 / 100.0;
    let jumping_skill = stats.jumping as f32 / 100.0;
    let height_profile = HeightProfile::Arc;
    let lift_skill = (heading_skill + jumping_skill) * 0.5;
    let lift_ratio = compute_lift_ratio(1.0, lift_skill, 0.0);

    // Base success rate (higher for better headers)
    #[cfg(feature = "fm_meta_attributes")]
    let success_rate = {
        use crate::engine::match_sim::attribute_calc::header_skill_fm_meta;
        let skill = header_skill_fm_meta(
            stats.heading as f32,
            stats.jumping as f32,
            stats.anticipation as f32,
            stats.strength as f32,
            stats.balance as f32,
        );
        (0.4 + skill * 0.55).min(0.95) // 0.4~0.95 range
    };
    #[cfg(not(feature = "fm_meta_attributes"))]
    let success_rate = {
        let composure_mod = stats.composure as f32 / 200.0;
        let base_success = 0.5 + heading_skill * 0.3 + jumping_skill * 0.15 + composure_mod;
        base_success.min(0.95)
    };

    // Random check for success
    let random = simple_random(ctx.rng_seed, ctx.current_tick, action.player_idx + 300);
    let success = random <= success_rate;
    queue.last_header_outcome = Some(HeaderOutcome {
        player_idx: action.player_idx,
        is_shot: *is_shot,
        success,
    });

    if !success {
        // Header failed - ball goes in random direction
        let random2 = simple_random(ctx.rng_seed, ctx.current_tick + 1, action.player_idx + 301);
        let random_angle = random2 * std::f32::consts::TAU;
        let random_dist = 5.0 + random2 * 10.0; // 5-15m
        let miss_target_m = (
            (player_pos_m.0 + random_angle.cos() * random_dist).clamp(0.0, field::LENGTH_M),
            (player_pos_m.1 + random_angle.sin() * random_dist).clamp(0.0, field::WIDTH_M),
        );
        let miss_target = Coord10::from_meters(miss_target_m.0, miss_target_m.1);

        let dx = miss_target_m.0 - player_pos_m.0;
        let dy = miss_target_m.1 - player_pos_m.1;
        let distance = (dx * dx + dy * dy).sqrt();
        let flight_ticks =
            ((distance / 15.0) * TICKS_PER_SECOND as f32).ceil() as u64;
        let arrival_tick = ctx.current_tick + flight_ticks.max(2);
        queue.ball_state = BallState::InFlight {
            from_pos: ball_origin, // FIX_2601/1120: Use ball position, not player position
            to_pos: miss_target,
            start_tick: ctx.current_tick,
            end_tick: arrival_tick,
            height_profile,
            lift_ratio,
            intended_receiver: None,
            is_shot: false,
            start_height_01m: 0,
            end_height_01m: 0,
        };
        // 헤딩 미스는 낙하지점 루즈볼로 이어짐 (trap 예약 없음)
        if action_debug_enabled() {
            eprintln!(
                "[AERIAL L5] tick={} action=Header result=Miss target=({:.1},{:.1}) arrival={} dist={:.2}",
                ctx.current_tick,
                miss_target_m.0,
                miss_target_m.1,
                arrival_tick,
                distance
            );
        }

        return ActionResult::HeaderWon {
            player_idx: action.player_idx,
            direction: (miss_target_m.0 - player_pos_m.0, miss_target_m.1 - player_pos_m.1),
        };
    }

    // Header succeeded
    if *is_shot {
        // Headed shot - target is goal
        // FIX_2601/0110: DirectionContext 사용 (하프타임 방향 전환 반영)
        let is_home = TeamSide::is_home(action.player_idx);
        let dir_ctx = if is_home { &ctx.home_ctx } else { &ctx.away_ctx };
        let goal_x = dir_ctx.opponent_goal_x() * field::LENGTH_M; // normalized → meters
        let goal_center_y = field::CENTER_Y;
        let dx = goal_x - player_pos_m.0;
        let dy = goal_center_y - player_pos_m.1;
        let distance = (dx * dx + dy * dy).sqrt();

        // xG calculation for headed shot (generally lower than regular shot)
        let base_xg = if distance < 6.0 {
            0.45 // Close header
        } else if distance < 12.0 {
            0.20 // Penalty box header
        } else {
            0.05 // Long range header
        };

        let header_mod = heading_skill * 0.3;
        let xg = (base_xg * (1.0 + header_mod)).min(0.80);

        // Ball flight
        let ball_speed = 18.0; // Headers are generally slower than shots
        let flight_ticks = ((distance / ball_speed) * TICKS_PER_SECOND as f32).ceil() as u64;
        let arrival_tick = ctx.current_tick + flight_ticks.max(2);

        let target_y_m = target.y as f32 / Coord10::SCALE;
        let shot_target = Coord10::from_meters(goal_x, target_y_m.clamp(30.5, 37.5));
        queue.ball_state = BallState::InFlight {
            from_pos: ball_origin, // FIX_2601/1120: Use ball position, not player position
            to_pos: shot_target,
            start_tick: ctx.current_tick,
            end_tick: arrival_tick,
            height_profile,
            lift_ratio,
            intended_receiver: None,
            is_shot: true,
            start_height_01m: 0,
            end_height_01m: 0,
        };

        if action_debug_enabled() {
            eprintln!(
                "[AERIAL L5] tick={} action=Header result=Shot target=({:.1},{:.1}) arrival={} xg={:.2}",
                ctx.current_tick,
                goal_x,
                target_y_m,
                arrival_tick,
                xg
            );
        }

        // FIX_2601/0115: Store shooter info for execute_save() to use
        // This is critical - without this, all goals default to player 10 (home CF)
        queue.last_shooter_idx = Some(action.player_idx);
        queue.last_shot_xg = Some(xg);

        // Schedule GK save - direction is normalized vector in meters
        let gk_idx = ctx.goalkeeper_indices.1; // Opponent GK
        let direction_m = shot_target.to_meters();
        queue.schedule_new(arrival_tick, ActionType::Save { direction: direction_m }, gk_idx, 150);

        ActionResult::ShotTaken { shooter_idx: action.player_idx, target: shot_target, xg }
    } else {
        // Headed pass - find nearest teammate in target direction
        let target_pos = *target;
        let target_pos_m = target_pos.to_meters();

        // Find nearest teammate to target position
        let _is_home = TeamSide::is_home(action.player_idx);
        let mut best_receiver: Option<(usize, f32)> = None;

        for (idx, pos) in ctx.player_positions.iter().enumerate() {
            let is_same_team = TeamSide::same_team(idx, action.player_idx);
            if !is_same_team || idx == action.player_idx {
                continue;
            }

            let dx = pos.0 - target_pos_m.0;
            let dy = pos.1 - target_pos_m.1;
            let dist = (dx * dx + dy * dy).sqrt();

            match best_receiver {
                Some((_, best_dist)) if dist < best_dist => {
                    best_receiver = Some((idx, dist));
                }
                None => {
                    best_receiver = Some((idx, dist));
                }
                _ => {}
            }
        }

        let receiver_idx = best_receiver.map(|(idx, _)| idx);
        let final_target_m =
            if let Some(idx) = receiver_idx { ctx.player_positions[idx] } else { target_pos_m };
        let final_target = Coord10::from_meters(final_target_m.0, final_target_m.1);

        // Ball flight for header pass (use meters for calculations)
        let dx = final_target_m.0 - player_pos_m.0;
        let dy = final_target_m.1 - player_pos_m.1;
        let distance = (dx * dx + dy * dy).sqrt();
        let flight_ticks = ((distance / 15.0) * TICKS_PER_SECOND as f32).ceil() as u64;
        let arrival_tick = ctx.current_tick + flight_ticks.max(2);

        queue.ball_state = BallState::InFlight {
            from_pos: ball_origin, // FIX_2601/1120: Use ball position, not player position
            to_pos: final_target,
            start_tick: ctx.current_tick,
            end_tick: arrival_tick,
            height_profile,
            lift_ratio,
            intended_receiver: receiver_idx,
            is_shot: false,
            start_height_01m: 0,
            end_height_01m: 0,
        };

        // Schedule trap action for receiver
        if let Some(recv_idx) = receiver_idx {
            // 헤딩으로 전달된 공은 중간 높이, 적당한 속도
            let ball_speed = distance * 0.1; // 거리 기반 속도 추정
            let ball_height = max_height_from_profile(height_profile, lift_ratio);
            queue.schedule_new(
                arrival_tick,
                ActionType::Trap { ball_speed, ball_height },
                recv_idx,
                100,
            );
            if action_debug_enabled() {
                eprintln!(
                    "[AERIAL L5] tick={} action=Header result=Pass receiver={} target=({:.1},{:.1}) arrival={} dist={:.2}",
                    ctx.current_tick,
                    recv_idx,
                    target_pos_m.0,
                    target_pos_m.1,
                    arrival_tick,
                    distance
                );
            }
        } else if action_debug_enabled() {
            eprintln!(
                "[AERIAL L5] tick={} action=Header result=Pass receiver=None target=({:.1},{:.1}) arrival={} dist={:.2}",
                ctx.current_tick,
                target_pos_m.0,
                target_pos_m.1,
                arrival_tick,
                distance
            );
        }

        ActionResult::HeaderWon { player_idx: action.player_idx, direction: (dx, dy) }
    }
}

/// 태클 실행
pub fn execute_tackle(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Tackle { target_idx } = &action.action_type else {
        debug_assert!(false, "execute_tackle called with non-Tackle action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_tackle called with non-Tackle action".to_string(),
        };
    };

    let tackler_stats = &ctx.player_stats[action.player_idx];
    let target_stats = &ctx.player_stats[*target_idx];
    let tackler_is_home = TeamSide::is_home(action.player_idx);
    let match_modifiers = if tackler_is_home {
        ctx.home_match_modifiers
    } else {
        ctx.away_match_modifiers
    };

    // 성공률 계산
    #[cfg(feature = "fm_meta_attributes")]
    let mut success_rate = {
        use crate::engine::match_sim::attribute_calc::{
            tackle_skill_fm_meta, dribble_attack_score_fm_meta, dribble_defend_score_fm_meta
        };
        // 태클 점수 (open-football 하이브리드 - aggression/bravery 포함)
        let tackle_score = tackle_skill_fm_meta(
            tackler_stats.tackling as f32,
            tackler_stats.anticipation as f32,
            tackler_stats.pace as f32,
            tackler_stats.strength as f32,
            tackler_stats.aggression as f32,  // 신규
            tackler_stats.bravery as f32,     // 신규
            tackler_stats.concentration as f32,
        );
        // 방어자 관점 드리블 방어 점수 (신규)
        let defend_score = dribble_defend_score_fm_meta(
            tackler_stats.tackling as f32,
            tackler_stats.positioning as f32,
            tackler_stats.anticipation as f32,
            tackler_stats.pace as f32,
            tackler_stats.strength as f32,
            tackler_stats.concentration as f32,
        );
        // 공격자 드리블 점수
        let attack_score = dribble_attack_score_fm_meta(
            target_stats.dribbling as f32,
            target_stats.agility as f32,
            target_stats.pace as f32,
            target_stats.acceleration as f32,
            target_stats.balance as f32,
            target_stats.flair as f32,
            target_stats.anticipation as f32,
        );
        // 태클 점수 + 방어 점수(0.5 가중) vs 공격 점수
        let total_defense = tackle_score + defend_score * 0.5;
        total_defense / (total_defense + attack_score)
    };
    #[cfg(not(feature = "fm_meta_attributes"))]
    let mut success_rate = {
        let tackle_skill = tackler_stats.tackling as f32 + tackler_stats.anticipation as f32 * 0.3;
        let resist_skill = target_stats.dribbling as f32 + target_stats.agility as f32 * 0.3;
        tackle_skill / (tackle_skill + resist_skill)
    };
    // FIX_2601/0109: 파울 확률 증가 (벤치마크: 경기당 22~28회)
    // 기존: 0.02 + (1 - tackling/100) * 0.06 = 2~8%
    // v2: 0.18 + (1 - tackling/100) * 0.18 + aggression/100 * 0.12 = 18~48%
    let aggression_factor = tackler_stats.aggression as f32 / 100.0 * 0.12;
    let foul_rate = 0.18 + (1.0 - tackler_stats.tackling as f32 / 100.0) * 0.18 + aggression_factor;

    // Apply sparse team-wide modifier without changing foul probability.
    let max_success_rate = (1.0 - foul_rate).max(0.0);
    success_rate =
        (success_rate * match_modifiers.tackle_success_mult).clamp(0.0, max_success_rate);

    let random = simple_random(ctx.rng_seed, ctx.current_tick, action.player_idx + 300);

    if random < foul_rate {
        // 파울
        let foul_pos_m = ctx.player_positions[*target_idx];

        // vNext(v2.1): Advantage play (executor-only, deterministic, gated).
        // If advantage is played, we keep play in motion and do not stop for a free kick.
        let should_play_advantage = if !ctx.rulebook_advantage_play_enabled {
            false
        } else {
            // Require victim is current ball owner (avoid inventing possession).
            let victim_is_owner = matches!(
                queue.ball_state,
                BallState::Controlled { owner_idx } if owner_idx == *target_idx
            );
            if !victim_is_owner {
                false
            } else {
                let target_is_home = TeamSide::is_home(*target_idx);
                let dir_ctx = if target_is_home { ctx.home_ctx } else { ctx.away_ctx };
                let foul_pos = Coord10::from_meters(foul_pos_m.0, foul_pos_m.1);
                let tv_foul_pos = dir_ctx.to_team_view(foul_pos);

                // Only play advantage in the attacking half (simple, conservative v1).
                if !tv_foul_pos.in_attacking_half() {
                    false
                } else {
                    // Require at least one teammate ahead (progression opportunity).
                    let teammate_range = if target_is_home { 0..11 } else { 11..22 };
                    let mut has_teammate_ahead = false;
                    for i in teammate_range {
                        if i == *target_idx {
                            continue;
                        }
                        let p = ctx.player_positions.get(i).copied().unwrap_or(foul_pos_m);
                        let tv = dir_ctx.to_team_view(Coord10::from_meters(p.0, p.1));
                        if tv.x >= tv_foul_pos.x + 40 {
                            // >= 4m ahead
                            has_teammate_ahead = true;
                            break;
                        }
                    }

                    if !has_teammate_ahead {
                        false
                    } else {
                        // Simple local pressure check: too many opponents nearby → take the FK.
                        let opp_range = TeamSide::opponent_range(*target_idx);
                        let mut opponents_near = 0usize;
                        let r2 = 4.0f32 * 4.0f32; // 4m radius
                        for oi in opp_range {
                            let op = ctx.player_positions.get(oi).copied().unwrap_or((0.0, 0.0));
                            let dx = op.0 - foul_pos_m.0;
                            let dy = op.1 - foul_pos_m.1;
                            if dx * dx + dy * dy <= r2 {
                                opponents_near += 1;
                                if opponents_near >= 2 {
                                    break;
                                }
                            }
                        }
                        opponents_near < 2
                    }
                }
            }
        };

        if should_play_advantage {
            // Keep victim in control and continue play.
            queue.ball_state = BallState::Controlled { owner_idx: *target_idx };
            ActionResult::TackleFoulAdvantage { tackler_idx: action.player_idx, target_idx: *target_idx }
        } else {
            // FIX_2601/0109: 프리킥은 파울 당한 팀(target)이 받음, 태클한 팀(tackler)이 아님
            let target_is_home = *target_idx < 11;
            queue.ball_state = BallState::OutOfPlay {
                restart_type: RestartType::FreeKick,
                position: Coord10::from_meters(foul_pos_m.0, foul_pos_m.1),
                home_team: target_is_home, // 파울 당한 선수의 팀이 프리킥
            };

            ActionResult::TackleFoul { tackler_idx: action.player_idx, target_idx: *target_idx }
        }
    } else if random < success_rate + foul_rate {
        // 태클 성공
        queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };

        ActionResult::TackleSuccess { tackler_idx: action.player_idx, target_idx: *target_idx }
    } else {
        // 태클 실패 (공 소유자 유지)
        let target_pos_m = ctx.player_positions[*target_idx];
        ActionResult::CarryComplete {
            player_idx: *target_idx,
            new_position: Coord10::from_meters(target_pos_m.0, target_pos_m.1),
        }
    }
}

/// 인터셉트 실행
pub fn execute_intercept(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Intercept { ball_position } = &action.action_type else {
        debug_assert!(false, "execute_intercept called with non-Intercept action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_intercept called with non-Intercept action".to_string(),
        };
    };

    let stats = &ctx.player_stats[action.player_idx];
    let player_pos_m = ctx.player_positions[action.player_idx];
    let ball_pos_m = ball_position.to_meters();

    // 공과의 거리 (meters)
    let dx = ball_pos_m.0 - player_pos_m.0;
    let dy = ball_pos_m.1 - player_pos_m.1;
    let distance = (dx * dx + dy * dy).sqrt();

    // 성공률 (거리 + 능력치)
    #[cfg(feature = "fm_meta_attributes")]
    let success_rate = {
        use crate::engine::match_sim::attribute_calc::intercept_skill_fm_meta;
        let skill = intercept_skill_fm_meta(
            stats.anticipation as f32,
            stats.positioning as f32,
            stats.pace as f32,
            stats.concentration as f32,
            stats.decisions as f32,
        );
        let distance_penalty = (distance / 5.0).min(0.5);
        (skill - distance_penalty).clamp(0.1, 0.9)
    };
    #[cfg(not(feature = "fm_meta_attributes"))]
    let success_rate = {
        let base_rate = stats.anticipation as f32 / 100.0;
        let distance_penalty = (distance / 5.0).min(0.5);
        (base_rate - distance_penalty).max(0.1)
    };

    let random = simple_random(ctx.rng_seed, ctx.current_tick, action.player_idx + 400);

    if random < success_rate {
        queue.ball_state = BallState::Controlled { owner_idx: action.player_idx };
        ActionResult::InterceptSuccess { player_idx: action.player_idx }
    } else {
        // 인터셉트 실패 - 공은 계속 진행
        ActionResult::MoveComplete { player_idx: action.player_idx, new_position: *ball_position }
    }
}

/// 이동 실행
pub fn execute_move(
    action: &ScheduledAction,
    ctx: &ExecutionContext,
    _queue: &mut ActionQueue,
) -> ActionResult {
    let ActionType::Move { target, sprint } = &action.action_type else {
        debug_assert!(false, "execute_move called with non-Move action");
        return ActionResult::Cancelled {
            action_id: action.action_id,
            reason: "execute_move called with non-Move action".to_string(),
        };
    };

    let player_pos_m = ctx.player_positions[action.player_idx];
    let target_m = target.to_meters();

    // 최대 이동 거리
    #[cfg(feature = "fm_meta_attributes")]
    let max_dist = {
        use crate::engine::match_sim::attribute_calc::pace_acceleration_bonus;
        let stats = &ctx.player_stats[action.player_idx];
        let bonus = pace_acceleration_bonus(stats.pace as f32, stats.acceleration as f32, "movement");
        let base = if *sprint { 8.0 } else { 5.0 };
        base * (1.0 + bonus) // 최대 15% 추가 이동
    };
    #[cfg(not(feature = "fm_meta_attributes"))]
    let max_dist = if *sprint { 8.0 } else { 5.0 };

    // 목표까지 거리 (meters)
    let dx = target_m.0 - player_pos_m.0;
    let dy = target_m.1 - player_pos_m.1;
    let dist = (dx * dx + dy * dy).sqrt();

    let new_pos = if dist <= max_dist {
        *target
    } else {
        let ratio = max_dist / dist;
        Coord10::from_meters(player_pos_m.0 + dx * ratio, player_pos_m.1 + dy * ratio)
    };

    ActionResult::MoveComplete { player_idx: action.player_idx, new_position: new_pos }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 간단한 결정론적 랜덤 (0.0 ~ 1.0)
/// 점에서 선분까지의 최단 거리 계산
/// line_start -> line_end 선분에서 point까지의 수직 거리
fn distance_point_to_line(point: (f32, f32), line_start: (f32, f32), line_end: (f32, f32)) -> f32 {
    let dx = line_end.0 - line_start.0;
    let dy = line_end.1 - line_start.1;
    let line_len_sq = dx * dx + dy * dy;

    if line_len_sq < 0.0001 {
        // 선분이 점에 가까움 - 그냥 점과 점 사이 거리
        let px = point.0 - line_start.0;
        let py = point.1 - line_start.1;
        return (px * px + py * py).sqrt();
    }

    // 선분 위의 가장 가까운 점 파라미터 t (0~1 범위로 클램프)
    let t = ((point.0 - line_start.0) * dx + (point.1 - line_start.1) * dy) / line_len_sq;
    let t = t.clamp(0.0, 1.0);

    // 가장 가까운 점
    let closest_x = line_start.0 + t * dx;
    let closest_y = line_start.1 + t * dy;

    // 거리 계산
    let dist_x = point.0 - closest_x;
    let dist_y = point.1 - closest_y;
    (dist_x * dist_x + dist_y * dist_y).sqrt()
}

// ============================================================================
// Optional Rulebook Triggers (Non-GK Handball)
//
// Keep these rare and deterministic; they must not depend on render/visual
// overlap. All restarts are expressed via BallState::OutOfPlay and handled by
// tick_based.rs (SSOT).
// ============================================================================
// NOTE: Our shot resolution can yield `ShotOutcome::Blocked` without an explicit
// blocker index (it is sampled from an outcome set). In that case, we infer the
// likely offender from geometry.
//
// In practice, player positions are coarse (250ms decision tick), so a too-tight
// line-distance gate will make the surface unreachable in KPI runs.
const NON_GK_HANDBALL_SHOT_BLOCK_MAX_LINE_DIST_M: f32 = 8.0;
const NON_GK_HANDBALL_SHOT_BLOCK_FALLBACK_MAX_SHOOTER_DIST_M: f32 = 12.0;
// Base rate is conservative; volume is controlled via:
// - `ExpConfig.rulebook.non_gk_handball_enabled` (default off)
// - `ExpConfig.rulebook.non_gk_handball_prob_mult` (0..10)
const NON_GK_HANDBALL_SHOT_BLOCK_BASE_PROB: f32 = 0.01;
const NON_GK_HANDBALL_AERIAL_TRAP_MIN_HEIGHT_M: f32 = 1.0;
const NON_GK_HANDBALL_AERIAL_TRAP_BASE_PROB: f32 = 0.002;
const NON_GK_HANDBALL_PROB_MAX: f32 = 0.15;

fn pick_non_gk_defender_closest_to_segment(
    player_positions: &[(f32, f32)],
    defender_range: std::ops::Range<usize>,
    gk_idx: usize,
    line_start_m: (f32, f32),
    line_end_m: (f32, f32),
    max_line_dist_m: f32,
) -> Option<usize> {
    let mut best_idx: Option<usize> = None;
    let mut best_dist = f32::INFINITY;

    for idx in defender_range {
        if idx >= player_positions.len() || idx == gk_idx {
            continue;
        }
        let dist = distance_point_to_line(player_positions[idx], line_start_m, line_end_m);
        if dist < best_dist {
            best_dist = dist;
            best_idx = Some(idx);
        }
    }

    if best_dist <= max_line_dist_m { best_idx } else { None }
}

fn pick_non_gk_defender_closest_to_point(
    player_positions: &[(f32, f32)],
    defender_range: std::ops::Range<usize>,
    gk_idx: usize,
    point_m: (f32, f32),
    max_dist_m: f32,
) -> Option<usize> {
    let mut best_idx: Option<usize> = None;
    let mut best_dist_sq = f32::INFINITY;
    let max_dist_sq = max_dist_m * max_dist_m;

    for idx in defender_range {
        if idx >= player_positions.len() || idx == gk_idx {
            continue;
        }
        let dx = player_positions[idx].0 - point_m.0;
        let dy = player_positions[idx].1 - point_m.1;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_idx = Some(idx);
        }
    }

    if best_dist_sq <= max_dist_sq { best_idx } else { None }
}

fn non_gk_handball_prob_on_shot_block(pressure: f32, defender_stats: &PlayerStats) -> f32 {
    let pressure = pressure.clamp(0.0, 1.0);
    let composure = (defender_stats.composure as f32 / 100.0).clamp(0.0, 1.0);
    let concentration = (defender_stats.concentration as f32 / 100.0).clamp(0.0, 1.0);
    let skill = 0.5 * composure + 0.5 * concentration;
    let pressure_factor = 0.5 + pressure; // 0.5..1.5
    let skill_factor = (1.0 - 0.8 * skill).clamp(0.2, 1.0);

    (NON_GK_HANDBALL_SHOT_BLOCK_BASE_PROB * pressure_factor * skill_factor).clamp(0.0, 0.02)
}

fn non_gk_handball_prob_on_aerial_trap_fail(pressure: f32, trapper_stats: &PlayerStats) -> f32 {
    let pressure = pressure.clamp(0.0, 1.0);
    let composure = (trapper_stats.composure as f32 / 100.0).clamp(0.0, 1.0);
    let first_touch = (trapper_stats.first_touch as f32 / 100.0).clamp(0.0, 1.0);
    let skill = 0.6 * composure + 0.4 * first_touch;
    let pressure_factor = pressure.powf(1.2);
    let skill_factor = (1.0 - 0.75 * skill).clamp(0.25, 1.0);

    (NON_GK_HANDBALL_AERIAL_TRAP_BASE_PROB * pressure_factor * skill_factor).clamp(0.0, 0.02)
}

fn maybe_non_gk_handball_on_shot_block(
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
    shooter_idx: usize,
    shooter_is_home: bool,
    defending_ctx: &DirectionContext,
    defending_gk_idx: usize,
    pressure: f32,
    line_start_m: (f32, f32),
    line_end_m: (f32, f32),
) -> Option<ActionResult> {
    use crate::engine::coordinates;

    if !ctx.rulebook_non_gk_handball_enabled {
        return None;
    }

    let offender_idx = pick_non_gk_defender_closest_to_segment(
        &ctx.player_positions,
        TeamSide::opponent_range(shooter_idx),
        defending_gk_idx,
        line_start_m,
        line_end_m,
        NON_GK_HANDBALL_SHOT_BLOCK_MAX_LINE_DIST_M,
    )
    .or_else(|| {
        pick_non_gk_defender_closest_to_point(
            &ctx.player_positions,
            TeamSide::opponent_range(shooter_idx),
            defending_gk_idx,
            line_start_m,
            NON_GK_HANDBALL_SHOT_BLOCK_FALLBACK_MAX_SHOOTER_DIST_M,
        )
    })?;

    let prob = (non_gk_handball_prob_on_shot_block(pressure, &ctx.player_stats[offender_idx])
        * ctx.rulebook_non_gk_handball_prob_mult)
        .clamp(0.0, NON_GK_HANDBALL_PROB_MAX);
    let roll = simple_random(ctx.rng_seed, ctx.current_tick, offender_idx + 9000);
    if roll >= prob {
        return None;
    }

    let offender_pos_m = ctx.player_positions[offender_idx];
    let foul_pos = Coord10::from_meters(offender_pos_m.0, offender_pos_m.1).clamp_to_field();
    let is_penalty =
        coordinates::is_in_penalty_area(foul_pos.to_normalized_legacy(), !defending_ctx.attacks_right);
    queue.ball_state = BallState::OutOfPlay {
        restart_type: if is_penalty { RestartType::Penalty } else { RestartType::FreeKick },
        position: foul_pos,
        home_team: shooter_is_home, // attacker receives
    };

    Some(ActionResult::HandballFoul { offender_idx, last_touch_idx: Some(shooter_idx) })
}

fn maybe_non_gk_handball_on_aerial_trap_fail(
    ctx: &ExecutionContext,
    queue: &mut ActionQueue,
    trapper_idx: usize,
    trapper_is_home: bool,
    trapper_stats: &PlayerStats,
    pressure: f32,
    ball_height_m: f32,
    is_pass_trap: bool,
) -> Option<ActionResult> {
    if !ctx.rulebook_non_gk_handball_enabled {
        return None;
    }
    if !is_pass_trap || ball_height_m < NON_GK_HANDBALL_AERIAL_TRAP_MIN_HEIGHT_M {
        return None;
    }

    let prob = (non_gk_handball_prob_on_aerial_trap_fail(pressure, trapper_stats)
        * ctx.rulebook_non_gk_handball_prob_mult)
        .clamp(0.0, NON_GK_HANDBALL_PROB_MAX);
    let roll = simple_random(ctx.rng_seed, ctx.current_tick, trapper_idx + 9100);
    if roll >= prob {
        return None;
    }

    use crate::engine::coordinates;
    let trapper_ctx = if trapper_is_home { &ctx.home_ctx } else { &ctx.away_ctx };
    let trapper_pos_m = ctx.player_positions[trapper_idx];
    let foul_pos = Coord10::from_meters(trapper_pos_m.0, trapper_pos_m.1).clamp_to_field();
    let is_penalty =
        coordinates::is_in_penalty_area(foul_pos.to_normalized_legacy(), !trapper_ctx.attacks_right);
    queue.ball_state = BallState::OutOfPlay {
        restart_type: if is_penalty { RestartType::Penalty } else { RestartType::FreeKick },
        position: foul_pos,
        home_team: !trapper_is_home, // opponent receives
    };

    Some(ActionResult::HandballFoul { offender_idx: trapper_idx, last_touch_idx: queue.last_passer_idx })
}

fn simple_random(seed: u64, tick: u64, salt: usize) -> f32 {
    let combined = seed
        .wrapping_mul(1103515245)
        .wrapping_add(tick.wrapping_mul(12345))
        .wrapping_add(salt as u64 * 7919);
    let hash = combined.wrapping_mul(2654435761);
    (hash % 10000) as f32 / 10000.0
}

fn apply_attribute_multiplier(value: u8, mult: f32) -> u8 {
    if !mult.is_finite() {
        return value;
    }
    ((value as f32) * mult).round().clamp(0.0, 100.0) as u8
}

/// 가장 가까운 상대 선수 찾기
fn find_nearest_opponent(
    pos: (f32, f32),
    all_positions: &[(f32, f32)],
    player_idx: usize,
) -> Option<(usize, f32)> {
    let opponent_range = TeamSide::opponent_range(player_idx);

    opponent_range
        .filter_map(|idx| {
            if idx >= all_positions.len() {
                return None;
            }
            let opp_pos = all_positions[idx];
            let dx = opp_pos.0 - pos.0;
            let dy = opp_pos.1 - pos.1;
            let dist = (dx * dx + dy * dy).sqrt();
            Some((idx, dist))
        })
        .min_by(|a, b| {
            match a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal) {
                std::cmp::Ordering::Equal => {
                    // FIX_2601/0116: Use position-based tie-breaker to avoid index bias
                    let pos_a = all_positions[a.0];
                    let pos_b = all_positions[b.0];
                    crate::engine::match_sim::deterministic_tie_hash(a.0, pos_a, b.0, pos_b)
                }
                other => other,
            }
        })
}

/// 후속 액션 예약 헬퍼
pub fn schedule_followup(queue: &mut ActionQueue, result: &ActionResult, ctx: &ExecutionContext) {
    match result {
        ActionResult::TrapSuccess { player_idx } => {
            // 트랩 성공 후 → 다음 액션 결정 (간단히 드리블로)
            queue.schedule_new(
                ctx.current_tick + 5,
                ActionType::Dribble {
                    direction: (1.0, 0.0), // 기본: 전방
                    aggressive: false,
                },
                *player_idx,
                80,
            );
        }
        ActionResult::TrapFailed { loose_ball_pos, .. } => {
            // 트랩 실패 → 가장 가까운 선수가 인터셉트 시도
            let loose_pos_m = loose_ball_pos.to_meters();
            if let Some((nearest_idx, _)) = find_nearest_opponent(
                loose_pos_m,
                &ctx.player_positions,
                999, // 모든 선수 대상
            ) {
                queue.schedule_new(
                    ctx.current_tick + 3,
                    ActionType::Intercept { ball_position: *loose_ball_pos },
                    nearest_idx,
                    90,
                );
            }
        }
        ActionResult::SaveMade { goalkeeper_idx, save_type, .. } => {
            if matches!(save_type, SaveType::Catch) {
                // 캐치 후 → 골킥 또는 스로우
                queue.schedule_new(
                    ctx.current_tick + 20,
                    ActionType::Pass {
                        target_idx: 2, // 기본: 수비수에게
                        is_long: false,
                        is_through: false,
                        intended_target_pos: None,
                        intended_passer_pos: None,
                    },
                    *goalkeeper_idx,
                    70,
                );
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::coord10::Coord10;

    #[test]
    fn test_schedule_execution() {
        let mut queue = ActionQueue::new();

        // 틱 10에 패스 예약
        queue.schedule_new(
            10,
            ActionType::Pass { target_idx: 5, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None },
            3,
            100,
        );

        // 틱 5에는 실행할 액션 없음
        let actions = queue.get_actions_for_tick(5);
        assert!(actions.is_empty());

        // 틱 10에 액션 실행
        let actions = queue.get_actions_for_tick(10);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].player_idx, 3);
    }

    #[test]
    fn test_pass_trap_sequence() {
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 3 });

        // 패스 예약 (틱 10)
        queue.schedule_new(
            10,
            ActionType::Pass { target_idx: 7, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None },
            3,
            100,
        );

        // 트랩 예약 (틱 15, 패스 도착 후)
        queue.schedule_new(15, ActionType::Trap { ball_speed: 15.0, ball_height: 0.5 }, 7, 100);

        // 패스 실행
        let actions = queue.get_actions_for_tick(10);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action_type, ActionType::Pass { .. }));

        // 트랩 실행
        let actions = queue.get_actions_for_tick(15);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action_type, ActionType::Trap { .. }));
    }

    #[test]
    fn test_action_priority() {
        let mut queue = ActionQueue::new();

        // 같은 틱에 여러 액션 (다른 우선순위)
        queue.schedule_new(
            10,
            ActionType::Move { target: Coord10::from_meters(50.0, 30.0), sprint: false },
            1,
            50, // 낮은 우선순위
        );

        queue.schedule_new(
            10,
            ActionType::Intercept { ball_position: Coord10::from_meters(45.0, 35.0) },
            2,
            100, // 높은 우선순위
        );

        queue.schedule_new(
            10,
            ActionType::Tackle { target_idx: 3 },
            3,
            75, // 중간 우선순위
        );

        let actions = queue.get_actions_for_tick(10);
        assert_eq!(actions.len(), 3);

        // 우선순위 순서 확인 (높은 것 먼저)
        assert!(matches!(actions[0].action_type, ActionType::Intercept { .. }));
        assert!(matches!(actions[1].action_type, ActionType::Tackle { .. }));
        assert!(matches!(actions[2].action_type, ActionType::Move { .. }));
    }

    #[test]
    fn test_cancel_ball_actions() {
        let mut queue = ActionQueue::new();

        // 공 관련 액션
        queue.schedule_new(
            10,
            ActionType::Pass { target_idx: 5, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None },
            3,
            100,
        );

        // 공 무관 액션
        queue.schedule_new(
            10,
            ActionType::Move { target: Coord10::from_meters(50.0, 30.0), sprint: false },
            4,
            50,
        );

        // 공 관련 액션 취소
        let cancelled = queue.cancel_ball_actions();
        assert_eq!(cancelled.len(), 1);
        assert!(matches!(cancelled[0].action_type, ActionType::Pass { .. }));

        // Move 액션만 남음
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn test_woodwork_pre_actions_bounces_and_cancels_ball_actions() {
        use crate::engine::ball_physics_params::DEFAULT;

        let mut queue = ActionQueue::new();

        // A short in-flight segment that reaches the goal line near the lower post.
        // Use a y that is within DEFAULT.woodwork_tolerance_m (0.3m) of goal::Y_MIN.
        let to_y = 30.4; // goal::Y_MIN=30.34, diff=0.06
        queue.ball_state = BallState::InFlight {
            from_pos: Coord10::from_meters(104.0, field::CENTER_Y),
            to_pos: Coord10::from_meters(field::LENGTH_M, to_y),
            start_tick: 0,
            end_tick: 2,
            height_profile: HeightProfile::Flat,
            lift_ratio: 0.0,
            intended_receiver: None,
            is_shot: true,
            start_height_01m: 0,
            end_height_01m: 0,
        };

        // Ball-dependent actions that would execute at arrival tick (must be cancelled).
        queue.schedule_new(
            2,
            ActionType::Save {
                direction: (field::LENGTH_M, to_y),
            },
            0,
            150,
        );
        queue.schedule_new(2, ActionType::Trap { ball_speed: 10.0, ball_height: 0.0 }, 7, 100);
        // Ball-independent action must remain.
        queue.schedule_new(
            2,
            ActionType::Move { target: Coord10::from_meters(50.0, 30.0), sprint: false },
            4,
            50,
        );

        queue.resolve_in_flight_woodwork_pre_actions(2, DEFAULT);

        // Ball becomes Loose and bounces back on x (vx < 0).
        match queue.ball_state() {
            BallState::Loose { position, velocity } => {
                let (x, y) = position.to_meters();
                assert!((x - field::LENGTH_M).abs() < 0.11);
                assert!((y - to_y).abs() < 0.11);
                assert!(velocity.vx < 0);
            }
            _ => panic!("Expected Loose after woodwork"),
        }

        // Save/Trap cancelled; Move remains.
        assert_eq!(queue.pending_count(), 1);
        let remaining = queue.peek_next().expect("expected remaining action");
        assert!(matches!(remaining.action_type, ActionType::Move { .. }));

        // Cancelled results recorded with reason=woodwork.
        let results = queue.take_tick_results();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| matches!(r, ActionResult::Cancelled { reason, .. } if reason == "woodwork")));

        // Sanity: velocity conversion matches our dt (no NaNs / weird values).
        if let BallState::Loose { velocity, .. } = queue.ball_state() {
            let (vx, vy) = velocity.to_mps();
            assert!(vx.is_finite() && vy.is_finite());
        }

    }

    #[test]
    fn test_woodwork_bounce_threshold_suppresses_low_speed_bounce() {
        use crate::engine::ball_physics_params::DEFAULT;

        let mut queue = ActionQueue::new();

        // Total dx = 0.4m over 2 ticks -> per-slice dx (tick1->tick2) = 0.2m
        // => vx ≈ 0.8m/s < DEFAULT.woodwork_bounce_speed_threshold_mps (1.5) -> no bounce.
        let to_y = 30.4;
        queue.ball_state = BallState::InFlight {
            from_pos: Coord10::from_meters(104.6, field::CENTER_Y),
            to_pos: Coord10::from_meters(field::LENGTH_M, to_y),
            start_tick: 0,
            end_tick: 2,
            height_profile: HeightProfile::Flat,
            lift_ratio: 0.0,
            intended_receiver: None,
            is_shot: true,
            start_height_01m: 0,
            end_height_01m: 0,
        };

        queue.resolve_in_flight_woodwork_pre_actions(2, DEFAULT);

        match queue.ball_state() {
            BallState::Loose { velocity, .. } => {
                // Normal component is suppressed to 0 (no micro-bounce).
                assert_eq!(velocity.vx, 0);
            }
            _ => panic!("Expected Loose after woodwork"),
        }
    }

    #[test]
    fn test_crossbar_collision_creates_inflight_with_height_continuity() {
        use crate::engine::ball_physics_params::DEFAULT;
        use crate::engine::physics_constants::goal;

        let mut queue = ActionQueue::new();

        // Shot heading toward the crossbar: y within goal posts, z at crossbar height
        // Flight starts at mid-pitch height rising to crossbar level at goal line
        let mid_y = (goal::Y_MIN + goal::Y_MAX) / 2.0; // center of goal
        queue.ball_state = BallState::InFlight {
            from_pos: Coord10::from_meters(100.0, mid_y),
            to_pos: Coord10::from_meters(field::LENGTH_M, mid_y),
            start_tick: 0,
            end_tick: 2,
            height_profile: HeightProfile::Arc,
            lift_ratio: 0.5, // Creates height that reaches crossbar at end
            intended_receiver: None,
            is_shot: true,
            start_height_01m: 20, // Start at 2.0m
            end_height_01m: 24,   // End at 2.4m (crossbar height)
        };

        // Add ball-dependent action that should be cancelled
        queue.schedule_new(
            2,
            ActionType::Save {
                direction: (field::LENGTH_M, mid_y),
            },
            0,
            150,
        );

        queue.resolve_in_flight_woodwork_pre_actions(2, DEFAULT);

        // Crossbar hit should create new InFlight segment (not Loose)
        match queue.ball_state() {
            BallState::InFlight {
                start_height_01m,
                end_height_01m,
                from_pos,
                ..
            } => {
                // Start height should be near crossbar height (~24 = 2.4m)
                assert!(
                    *start_height_01m >= 20,
                    "Crossbar rebound should preserve height, got start_height={}",
                    start_height_01m
                );
                // End height should be 0 (landing on ground)
                assert_eq!(*end_height_01m, 0, "Crossbar rebound should end at ground");
                // From position should be at/near the goal line
                let (x, _) = from_pos.to_meters();
                assert!(
                    (x - field::LENGTH_M).abs() < 0.5,
                    "Rebound should start at goal line, got x={}",
                    x
                );
            }
            BallState::Loose { .. } => {
                panic!("Crossbar hit should create InFlight, not Loose");
            }
            other => panic!("Unexpected ball state: {:?}", other),
        }

        // Save action should be cancelled
        let results = queue.take_tick_results();
        assert!(
            results.iter().any(|r| matches!(r, ActionResult::Cancelled { reason, .. } if reason == "woodwork")),
            "Expected cancelled action from woodwork"
        );
    }

    #[test]
    fn test_loose_ball_physics_roll_damping_is_monotonic_and_stops() {
        use crate::engine::ball_physics_params::DEFAULT;
        use crate::engine::types::coord10::Vel10;

        let mut queue = ActionQueue::new();
        queue.ball_state = BallState::Loose {
            position: Coord10::from_meters(50.0, field::CENTER_Y),
            velocity: Vel10::from_mps(10.0, 0.0),
        };

        let mut prev_speed = i32::MAX;
        for tick in 0..200u64 {
            queue.advance_loose_ball_physics(tick, DEFAULT);
            let BallState::Loose { velocity, .. } = queue.ball_state() else {
                panic!("Expected ball to remain Loose for this test");
            };
            let speed = velocity.magnitude();
            assert!(speed <= prev_speed, "speed increased: {speed} > {prev_speed} at tick={tick}");
            prev_speed = speed;
            if speed == 0 {
                break;
            }
        }

        assert_eq!(prev_speed, 0, "ball should have come to rest under damping + stop threshold");
    }

    #[test]
    fn test_ball_state_transitions() {
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 3 });

        // 패스 결과 기록
        queue.record_result(ActionResult::PassStarted {
            passer_idx: 3,
            receiver_idx: 7,
            arrival_tick: 15,
            ball_speed: 18.0,
            intended_target_pos: None,
            intended_passer_pos: None,
        });

        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 3 }));

        // 트랩 성공 기록
        queue.record_result(ActionResult::TrapSuccess { player_idx: 7 });

        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 7 }));
    }

    #[test]
    fn test_ball_state_owner() {
        use crate::engine::types::coord10::Vel10;

        let controlled = BallState::Controlled { owner_idx: 5 };
        assert_eq!(controlled.owner(), Some(5));

        let in_flight = BallState::InFlight {
            from_pos: Coord10::from_meters(0.0, 0.0),
            to_pos: Coord10::from_meters(10.0, 10.0),
            height_profile: HeightProfile::Arc,
            lift_ratio: 0.7,
            start_tick: 0,
            end_tick: 10,
            intended_receiver: Some(7),
            is_shot: false,
            start_height_01m: 0,
            end_height_01m: 0,
        };
        assert_eq!(in_flight.owner(), None);

        let loose = BallState::Loose {
            position: Coord10::from_meters(50.0, field::CENTER_Y),
            velocity: Vel10::from_mps(5.0, 0.0),
        };
        assert_eq!(loose.owner(), None);
    }

    #[test]
    fn test_action_type_requires_ball() {
        assert!(
            ActionType::Pass { target_idx: 5, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None }.requires_ball()
        );
        assert!(
            ActionType::Shot { power: 0.8, target: Coord10::from_meters(0.0, 0.0) }.requires_ball()
        );
        assert!(ActionType::Dribble { direction: (1.0, 0.0), aggressive: false }.requires_ball());

        assert!(!ActionType::Move { target: Coord10::from_meters(50.0, 30.0), sprint: false }
            .requires_ball());
        assert!(!ActionType::Intercept { ball_position: Coord10::from_meters(45.0, 35.0) }
            .requires_ball());
    }

    fn find_seed_for_roll_lt(prob: f32, tick: u64, salt: usize) -> u64 {
        for seed in 0..200_000u64 {
            if simple_random(seed, tick, salt) < prob {
                return seed;
            }
        }
        panic!("no seed found for prob={prob} tick={tick} salt={salt}");
    }

    #[test]
    fn test_pick_non_gk_defender_closest_to_segment_prefers_nearest_non_gk() {
        let mut positions = vec![(0.0, 0.0); 22];
        // Away GK (index 11) is on the shot line, but must be ignored.
        positions[11] = (100.0, field::CENTER_Y);
        // A non-GK defender is also on the line and should be selected.
        positions[15] = (100.0, field::CENTER_Y);
        positions[16] = (100.0, 36.5); // farther from the line

        let picked = pick_non_gk_defender_closest_to_segment(
            &positions,
            11..22,
            11,
            (90.0, field::CENTER_Y),
            (field::LENGTH_M, field::CENTER_Y),
            NON_GK_HANDBALL_SHOT_BLOCK_MAX_LINE_DIST_M,
        );
        assert_eq!(picked, Some(15));
    }

    #[test]
    fn test_non_gk_handball_prob_on_shot_block_scales_with_pressure_and_skill() {
        let mut low = PlayerStats::default();
        low.composure = 0;
        low.concentration = 0;

        let p0 = non_gk_handball_prob_on_shot_block(0.0, &low);
        let p1 = non_gk_handball_prob_on_shot_block(1.0, &low);
        assert!(p1 > p0);

        let mut high = PlayerStats::default();
        high.composure = 100;
        high.concentration = 100;
        let p1_high = non_gk_handball_prob_on_shot_block(1.0, &high);
        assert!(p1_high < p1);
    }

    #[test]
    fn test_non_gk_handball_prob_on_aerial_trap_fail_scales_with_pressure_and_skill() {
        let mut low = PlayerStats::default();
        low.composure = 0;
        low.first_touch = 0;

        let p0 = non_gk_handball_prob_on_aerial_trap_fail(0.0, &low);
        let p1 = non_gk_handball_prob_on_aerial_trap_fail(1.0, &low);
        assert!(p1 > p0);

        let mut high = PlayerStats::default();
        high.composure = 100;
        high.first_touch = 100;
        let p1_high = non_gk_handball_prob_on_aerial_trap_fail(1.0, &high);
        assert!(p1_high < p1);
    }

    #[test]
    fn test_maybe_non_gk_handball_on_shot_block_sets_penalty_restart() {        
        let mut ctx = create_test_context();
        ctx.current_tick = 0;
        ctx.rulebook_non_gk_handball_enabled = true;

        let shooter_idx = 3; // home
        let defending_gk_idx = 11; // away GK
        let offender_idx = 15; // away non-GK

        ctx.player_positions[shooter_idx] = (90.0, field::CENTER_Y);
        ctx.player_positions[defending_gk_idx] = (100.0, field::CENTER_Y); // ignored (GK)
        ctx.player_positions[offender_idx] = (100.0, field::CENTER_Y); // on shot line

        ctx.player_stats[offender_idx].composure = 0;
        ctx.player_stats[offender_idx].concentration = 0;

        let pressure = 1.0;
        let prob = non_gk_handball_prob_on_shot_block(pressure, &ctx.player_stats[offender_idx]);
        ctx.rng_seed = find_seed_for_roll_lt(prob, ctx.current_tick, offender_idx + 9000);

        let mut queue = ActionQueue::new();
        let res = maybe_non_gk_handball_on_shot_block(
            &ctx,
            &mut queue,
            shooter_idx,
            true,
            &ctx.away_ctx,
            defending_gk_idx,
            pressure,
            ctx.player_positions[shooter_idx],
            (field::LENGTH_M, field::CENTER_Y),
        );

        assert!(matches!(
            res,
            Some(ActionResult::HandballFoul {
                offender_idx: got_offender,
                last_touch_idx: Some(got_last_touch),
            }) if got_offender == offender_idx && got_last_touch == shooter_idx
        ));
        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, home_team, .. } => {
                assert_eq!(*restart_type, RestartType::Penalty);
                assert_eq!(*home_team, true);
            }
            other => panic!("expected OutOfPlay after handball, got {other:?}"),
        }
    }

    #[test]
    fn test_maybe_non_gk_handball_on_aerial_trap_fail_sets_penalty_restart() {  
        let mut ctx = create_test_context();
        ctx.current_tick = 0;
        ctx.rulebook_non_gk_handball_enabled = true;

        let trapper_idx = 15; // away non-GK
        ctx.player_positions[trapper_idx] = (100.0, field::CENTER_Y);
        ctx.player_stats[trapper_idx].composure = 0;
        ctx.player_stats[trapper_idx].first_touch = 0;

        let pressure = 1.0;
        let ball_height_m = NON_GK_HANDBALL_AERIAL_TRAP_MIN_HEIGHT_M;
        let prob = non_gk_handball_prob_on_aerial_trap_fail(pressure, &ctx.player_stats[trapper_idx]);
        ctx.rng_seed = find_seed_for_roll_lt(prob, ctx.current_tick, trapper_idx + 9100);

        let mut queue = ActionQueue::new();
        queue.last_passer_idx = Some(3);

        let res = maybe_non_gk_handball_on_aerial_trap_fail(
            &ctx,
            &mut queue,
            trapper_idx,
            false,
            &ctx.player_stats[trapper_idx],
            pressure,
            ball_height_m,
            true,
        );

        assert!(matches!(
            res,
            Some(ActionResult::HandballFoul {
                offender_idx: got_offender,
                last_touch_idx: Some(3),
            }) if got_offender == trapper_idx
        ));
        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, home_team, .. } => {
                assert_eq!(*restart_type, RestartType::Penalty);
                assert_eq!(*home_team, true);
            }
            other => panic!("expected OutOfPlay after handball, got {other:?}"),
        }
    }

    // ========== Phase 3.2 Executor Tests ==========

    fn create_test_context() -> ExecutionContext {
        let mut positions = Vec::with_capacity(22);
        let mut stats = Vec::with_capacity(22);

        // 홈팀 (0-10)
        for i in 0..11 {
            positions.push((20.0 + i as f32 * 5.0, 30.0 + (i % 3) as f32 * 5.0));
            stats.push(PlayerStats {
                passing: 70,
                first_touch: 75,
                dribbling: 65,
                finishing: 60,
                long_shots: 55,
                tackling: 60,
                anticipation: 65,
                composure: 70,
                agility: 70,
                reflexes: if i == 0 { 80 } else { 50 },
                handling: if i == 0 { 75 } else { 40 },
                positioning: if i == 0 { 78 } else { 60 }, // FIX_2601/0109
                diving: if i == 0 { 76 } else { 50 },      // FIX_2601/0109
                heading: 65,
                jumping: 60,
                strength: 65,
                technique: 70,
                vision: 65,
                decisions: 68,
                aggression: 60,
                bravery: 65,
                // FIX_2601/0107: FM meta attributes
                concentration: 70,
                pace: 72,
                acceleration: 70,
                balance: 68,
                teamwork: 72,
                flair: 60,
                condition_level: 3,
            });
        }

        // 어웨이팀 (11-21)
        for i in 0..11 {
            positions.push((80.0 - i as f32 * 5.0, 38.0 - (i % 3) as f32 * 5.0));
            stats.push(PlayerStats {
                passing: 68,
                first_touch: 72,
                dribbling: 63,
                finishing: 58,
                long_shots: 53,
                tackling: 62,
                anticipation: 63,
                composure: 68,
                agility: 68,
                reflexes: if i == 0 { 78 } else { 48 },
                handling: if i == 0 { 73 } else { 38 },
                positioning: if i == 0 { 76 } else { 58 }, // FIX_2601/0109
                diving: if i == 0 { 74 } else { 48 },      // FIX_2601/0109
                heading: 63,
                jumping: 58,
                strength: 63,
                technique: 68,
                vision: 63,
                decisions: 66,
                aggression: 62,
                bravery: 63,
                // FIX_2601/0107: FM meta attributes
                concentration: 68,
                pace: 70,
                acceleration: 68,
                balance: 66,
                teamwork: 70,
                flair: 58,
                condition_level: 3,
            });
        }

        ExecutionContext {
            player_positions: positions,
            player_stats: stats,
            goalkeeper_indices: (0, 11),
            current_tick: 100,
            rng_seed: 12345,
            // FIX_2601/0110: 테스트용 DirectionContext (전반전 기준)
            home_ctx: DirectionContext::new(true), // Home attacks right
            away_ctx: DirectionContext::new(false), // Away attacks left
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            rulebook_non_gk_handball_enabled: false,
            rulebook_non_gk_handball_prob_mult: 1.0,
            rulebook_advantage_play_enabled: false,
            // FIX_2601/1120: Default ball position for tests
            ball_position: (field::CENTER_X, field::CENTER_Y),
        }
    }

    #[test]
    fn test_execute_tackle_foul_advantage_play_keeps_ball_in_play() {
        let target_idx = 5; // home attacker (victim)
        let tackler_idx = 15; // away defender (offender)
        let execute_tick = 100;

        fn make_ctx(
            rng_seed: u64,
            execute_tick: u64,
            target_idx: usize,
            tackler_idx: usize,
            advantage_enabled: bool,
        ) -> ExecutionContext {
            let mut player_positions = vec![(0.0, 0.0); 22];
            player_positions[target_idx] = (60.0, field::CENTER_Y); // attacking half (home TV)
            player_positions[target_idx + 1] = (65.0, field::CENTER_Y); // teammate ahead (>=4m)
            player_positions[tackler_idx] = (60.5, field::CENTER_Y); // within 4m radius

            let mut player_stats = vec![PlayerStats::default(); 22];
            // Make foul probability high to find a deterministic seed quickly.
            player_stats[tackler_idx] = PlayerStats {
                tackling: 0,
                aggression: 100,
                bravery: 100,
                concentration: 100,
                ..Default::default()
            };

            ExecutionContext {
                player_positions,
                player_stats,
                goalkeeper_indices: (0, 11),
                current_tick: execute_tick,
                rng_seed,
                home_ctx: DirectionContext::new(true),
                away_ctx: DirectionContext::new(false),
                home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                rulebook_non_gk_handball_enabled: false,
                rulebook_non_gk_handball_prob_mult: 1.0,
                rulebook_advantage_play_enabled: advantage_enabled,
                ball_position: (60.0, field::CENTER_Y), // FIX_2601/1120
            }
        }

        let action = ScheduledAction::new(
            execute_tick,
            ActionType::Tackle { target_idx },
            tackler_idx,
            100,
            1,
        );

        // Find a deterministic seed where this becomes a foul and advantage is played.
        let mut found_seed: Option<u64> = None;
        for seed in 0u64..20_000 {
            let ctx = make_ctx(seed, execute_tick, target_idx, tackler_idx, true);
            let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: target_idx });
            let res = execute_tackle(&action, &ctx, &mut queue);
            if matches!(res, ActionResult::TackleFoulAdvantage { .. }) {
                found_seed = Some(seed);
                break;
            }
        }
        let seed =
            found_seed.expect("expected to find a deterministic seed that triggers advantage foul");

        let ctx = make_ctx(seed, execute_tick, target_idx, tackler_idx, true);
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: target_idx });

        let res = execute_tackle(&action, &ctx, &mut queue);
        assert!(matches!(
            res,
            ActionResult::TackleFoulAdvantage { tackler_idx: got_tackler, target_idx: got_target }
                if got_tackler == tackler_idx && got_target == target_idx
        ));
        assert!(
            matches!(queue.ball_state(), BallState::Controlled { owner_idx } if *owner_idx == target_idx),
            "advantage foul must keep ball in play with victim retaining control"
        );
    }

    #[test]
    fn test_execute_tackle_foul_without_advantage_stops_for_free_kick() {
        let target_idx = 5; // home attacker (victim)
        let tackler_idx = 15; // away defender (offender)
        let execute_tick = 100;

        fn make_ctx(
            rng_seed: u64,
            execute_tick: u64,
            target_idx: usize,
            tackler_idx: usize,
            advantage_enabled: bool,
        ) -> ExecutionContext {
            let mut player_positions = vec![(0.0, 0.0); 22];
            player_positions[target_idx] = (60.0, field::CENTER_Y);
            player_positions[target_idx + 1] = (65.0, field::CENTER_Y);
            player_positions[tackler_idx] = (60.5, field::CENTER_Y);

            let mut player_stats = vec![PlayerStats::default(); 22];
            player_stats[tackler_idx] = PlayerStats {
                tackling: 0,
                aggression: 100,
                bravery: 100,
                concentration: 100,
                ..Default::default()
            };

            ExecutionContext {
                player_positions,
                player_stats,
                goalkeeper_indices: (0, 11),
                current_tick: execute_tick,
                rng_seed,
                home_ctx: DirectionContext::new(true),
                away_ctx: DirectionContext::new(false),
                home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                rulebook_non_gk_handball_enabled: false,
                rulebook_non_gk_handball_prob_mult: 1.0,
                rulebook_advantage_play_enabled: advantage_enabled,
                ball_position: (60.0, field::CENTER_Y), // FIX_2601/1120
            }
        }

        let action = ScheduledAction::new(
            execute_tick,
            ActionType::Tackle { target_idx },
            tackler_idx,
            100,
            1,
        );

        // Find a deterministic seed where this becomes a foul and advantage would be played.
        // Then verify that disabling advantage still produces an OutOfPlay free kick.
        let mut found_seed: Option<u64> = None;
        for seed in 0u64..20_000 {
            let ctx = make_ctx(seed, execute_tick, target_idx, tackler_idx, true);
            let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: target_idx });
            let res = execute_tackle(&action, &ctx, &mut queue);
            if matches!(res, ActionResult::TackleFoulAdvantage { .. }) {
                found_seed = Some(seed);
                break;
            }
        }
        let seed = found_seed.expect("expected to find a deterministic seed that triggers foul");

        let ctx = make_ctx(seed, execute_tick, target_idx, tackler_idx, false);
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: target_idx });

        let res = execute_tackle(&action, &ctx, &mut queue);
        assert!(matches!(
            res,
            ActionResult::TackleFoul { tackler_idx: got_tackler, target_idx: got_target }
                if got_tackler == tackler_idx && got_target == target_idx
        ));

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, position, home_team } => {
                assert_eq!(*restart_type, RestartType::FreeKick);
                assert_eq!(*position, Coord10::from_meters(60.0, field::CENTER_Y));
                assert!(*home_team, "victim is home team, so home should receive the free kick");
            }
            other => panic!("expected OutOfPlay free kick after foul, got {other:?}"),
        }
    }

    #[test]
    fn test_execute_pass_creates_trap() {
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 3 });
        let ctx = create_test_context();

        let action = ScheduledAction::new(
            100,
            ActionType::Pass { target_idx: 5, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None },
            3,
            100,
            1,
        );

        let result = execute_pass(&action, &ctx, &mut queue);

        // 패스 시작됨
        assert!(matches!(result, ActionResult::PassStarted { passer_idx: 3, receiver_idx: 5, .. }));

        // 공이 비행 중
        assert!(matches!(
            queue.ball_state(),
            BallState::InFlight { intended_receiver: Some(5), .. }
        ));

        // 트랩 액션이 예약됨
        assert_eq!(queue.pending_count(), 1);
        let next = queue.peek_next().unwrap();
        assert!(matches!(next.action_type, ActionType::Trap { .. }));
        assert_eq!(next.player_idx, 5);
    }

    #[test]
    fn test_pass_success_modifier_applies_per_side_for_trap() {
        fn make_ctx(receiver_idx: usize, rng_seed: u64, away_pass_success_mult: f32) -> ExecutionContext {
            let mut player_positions = vec![(0.0, 0.0); 22];
            player_positions[receiver_idx] = (50.0, field::CENTER_Y);

            let mut player_stats = vec![PlayerStats::default(); 22];
            player_stats[receiver_idx] = PlayerStats {
                first_touch: 20,
                composure: 20,
                anticipation: 20,
                concentration: 100,
                ..Default::default()
            };

            let mut away_match_modifiers = crate::engine::TeamMatchModifiers::default();
            away_match_modifiers.pass_success_mult = away_pass_success_mult;

            ExecutionContext {
                player_positions,
                player_stats,
                goalkeeper_indices: (0, 11),
                current_tick: 100,
                rng_seed,
                home_ctx: DirectionContext::new(true),
                away_ctx: DirectionContext::new(false),
                home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                away_match_modifiers,
                rulebook_non_gk_handball_enabled: false,
                rulebook_non_gk_handball_prob_mult: 1.0,
                rulebook_advantage_play_enabled: false,
                ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
            }
        }

        fn make_in_flight_queue(intended_receiver: usize) -> ActionQueue {
            ActionQueue::with_ball_state(BallState::InFlight {
                from_pos: Coord10::from_meters(0.0, 0.0),
                to_pos: Coord10::from_meters(10.0, 10.0),
                height_profile: HeightProfile::Arc,
                lift_ratio: 0.7,
                start_tick: 0,
                end_tick: 10,
                intended_receiver: Some(intended_receiver),
                is_shot: false,
                start_height_01m: 0,
                end_height_01m: 0,
            })
        }

        // Find a deterministic seed where away `pass_success_mult` flips a Trap from failed -> success.
        let receiver_away = 12;
        let action_away = ScheduledAction::new(
            100,
            ActionType::Trap {
                // High ball_speed makes ball_difficulty=1.0, increasing error variance so the
                // multiplier has a measurable effect under a deterministic seed.
                ball_speed: 30.0,
                ball_height: 0.0,
            },
            receiver_away,
            100,
            1,
        );

        let mut found_seed: Option<u64> = None;
        for seed in 0u64..20_000 {
            let ctx_base = make_ctx(receiver_away, seed, 1.0);
            let ctx_mod = make_ctx(receiver_away, seed, 1.2);

            let mut q_base = make_in_flight_queue(receiver_away);
            let mut q_mod = make_in_flight_queue(receiver_away);

            let res_base = execute_trap(&action_away, &ctx_base, &mut q_base);
            let res_mod = execute_trap(&action_away, &ctx_mod, &mut q_mod);

            if matches!(
                res_base,
                ActionResult::TrapFailed { player_idx, .. } if player_idx == receiver_away
            ) && matches!(
                res_mod,
                ActionResult::TrapSuccess { player_idx } if player_idx == receiver_away
            ) {
                found_seed = Some(seed);
                break;
            }
        }

        let seed = found_seed.expect(
            "expected to find a deterministic seed where away pass_success_mult flips Trap outcome",
        );

        // Sanity: away modifier should not affect HOME traps (home modifiers stay default).
        let receiver_home = 5;
        let action_home = ScheduledAction::new(
            100,
            ActionType::Trap {
                ball_speed: 30.0,
                ball_height: 0.0,
            },
            receiver_home,
            100,
            1,
        );

        let ctx_home_base = make_ctx(receiver_home, seed, 1.0);
        let ctx_home_away_mod = make_ctx(receiver_home, seed, 1.2);

        let mut q_home_base = make_in_flight_queue(receiver_home);
        let mut q_home_away_mod = make_in_flight_queue(receiver_home);

        let res_home_base = execute_trap(&action_home, &ctx_home_base, &mut q_home_base);
        let res_home_away_mod = execute_trap(&action_home, &ctx_home_away_mod, &mut q_home_away_mod);
        assert_eq!(res_home_base, res_home_away_mod, "away modifiers must not affect home trap outcome");
    }

    #[test]
    fn test_execute_shot_generates_result() {
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 9 });
        let ctx = create_test_context();

        let action = ScheduledAction::new(
            100,
            ActionType::Shot {
                power: 0.8,
                target: Coord10::from_meters(field::LENGTH_M, field::CENTER_Y),
            },
            9,
            100,
            1,
        );

        let result = execute_shot(&action, &ctx, &mut queue);

        // 슛 결과가 유효한 타입인지 확인 (확률적 결과 시스템 사용)
        let is_valid_shot_result = matches!(
            result,
            ActionResult::GoalScored { scorer_idx: 9, .. }
                | ActionResult::ShotTaken { shooter_idx: 9, .. }
                | ActionResult::ShotMissed { shooter_idx: 9, .. }
                | ActionResult::SaveMade { shooter_idx: 9, .. }
                | ActionResult::GoalkeeperHandlingViolation {
                    last_touch_idx: Some(9),
                    ..
                }
        );
        assert!(is_valid_shot_result, "Expected valid shot result for player 9, got {:?}", result);

        // xG가 계산됨 (ShotTaken인 경우에만)
        if let ActionResult::ShotTaken { xg, .. } = result {
            assert!(xg > 0.0 && xg < 1.0);
        }
    }

    #[test]
    fn test_execute_dribble_complete() {
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 7 });
        let mut ctx = create_test_context();

        // 상대 선수를 멀리 배치
        ctx.player_positions[15] = (100.0, 100.0);

        let action = ScheduledAction::new(
            100,
            ActionType::Dribble { direction: (1.0, 0.0), aggressive: false },
            7,
            100,
            1,
        );

        let result = execute_dribble(&action, &ctx, &mut queue);

        // 운반 완료 (aggressive: false → CarryComplete)
        assert!(matches!(result, ActionResult::CarryComplete { player_idx: 7, .. }));

        // 공 소유 유지
        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 7 }));
    }

    #[test]
    fn test_execute_move() {
        let mut queue = ActionQueue::new();
        let ctx = create_test_context();

        let action = ScheduledAction::new(
            100,
            ActionType::Move { target: Coord10::from_meters(50.0, 40.0), sprint: false },
            5,
            50,
            1,
        );

        let result = execute_move(&action, &ctx, &mut queue);

        // 이동 완료
        assert!(matches!(result, ActionResult::MoveComplete { player_idx: 5, .. }));

        if let ActionResult::MoveComplete { new_position, .. } = result {
            // 위치가 변경됨 (정확한 값은 거리에 따라 다름)
            let pos_m = new_position.to_meters();
            assert!(pos_m.0 > 0.0 && pos_m.1 > 0.0);
        }
    }

    #[test]
    fn test_schedule_followup_after_trap() {
        let mut queue = ActionQueue::new();
        let ctx = create_test_context();

        let result = ActionResult::TrapSuccess { player_idx: 5 };
        schedule_followup(&mut queue, &result, &ctx);

        // 드리블 액션이 예약됨
        assert_eq!(queue.pending_count(), 1);
        let next = queue.peek_next().unwrap();
        assert!(matches!(next.action_type, ActionType::Dribble { .. }));
        assert_eq!(next.player_idx, 5);
    }

    // ========== Phase 3.3 Interrupt System Tests ==========

    #[test]
    fn test_collision_clears_queue() {
        // INT-01: 패스 중 충돌 → Trap 삭제, BallState=Loose
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 3 });

        // 패스와 트랩 예약
        queue.schedule_new(
            10,
            ActionType::Pass { target_idx: 7, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None },
            3,
            100,
        );
        queue.schedule_new(15, ActionType::Trap { ball_speed: 18.0, ball_height: 0.5 }, 7, 100);
        // Move (공 무관)
        queue.schedule_new(
            12,
            ActionType::Move { target: Coord10::from_meters(50.0, 40.0), sprint: false },
            5,
            50,
        );

        assert_eq!(queue.pending_count(), 3);

        // 충돌 인터럽트
        let reason = InterruptReason::PhysicsCollision { object_idx: 15, impact_force: 0.7 };
        let cancelled = queue.handle_interrupt(&reason, Coord10::from_meters(45.0, 35.0));

        // Pass와 Trap이 취소됨
        assert_eq!(cancelled.len(), 2);

        // Move만 남음
        assert_eq!(queue.pending_count(), 1);
        let remaining = queue.peek_next().unwrap();
        assert!(matches!(remaining.action_type, ActionType::Move { .. }));

        // 공 상태 = Loose
        assert!(matches!(queue.ball_state(), BallState::Loose { .. }));

        // Cancelled 결과 기록됨
        let results = queue.take_tick_results();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| matches!(r, ActionResult::Cancelled { .. })));
    }

    #[test]
    fn test_foul_interruption() {
        // INT-02: 드리블 중 파울 → 프리킥 상태
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 8 });

        // 드리블 예약
        queue.schedule_new(
            10,
            ActionType::Dribble { direction: (1.0, 0.0), aggressive: true },
            8,
            100,
        );

        // 파울 인터럽트
        let reason = InterruptReason::Foul {
            offender_idx: 15,
            victim_idx: 8,
            position: Coord10::from_meters(60.0, 40.0),
            severity: 0.5,
        };
        let cancelled = queue.handle_interrupt(&reason, Coord10::from_meters(60.0, 40.0));

        // 드리블이 취소됨
        assert_eq!(cancelled.len(), 1);

        // 공 상태 = OutOfPlay (FreeKick)
        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, .. } => {
                assert_eq!(*restart_type, RestartType::FreeKick);
            }
            _ => panic!("Expected OutOfPlay state"),
        }
    }

    #[test]
    fn test_penalty_in_box_foul() {
        // 페널티 박스 내 파울 → 페널티킥
        let mut queue = ActionQueue::new();

        let reason = InterruptReason::Foul {
            offender_idx: 4,
            victim_idx: 18,
            position: Coord10::from_meters(95.0, field::CENTER_Y), // 페널티 박스 내
            severity: 0.8,
        };

        // 재개 타입 확인
        assert_eq!(reason.restart_type(), Some(RestartType::Penalty));

        queue.handle_interrupt(&reason, Coord10::from_meters(95.0, field::CENTER_Y));

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, .. } => {
                assert_eq!(*restart_type, RestartType::Penalty);
            }
            _ => panic!("Expected OutOfPlay state"),
        }
    }

    #[test]
    fn test_loose_ball_contest() {
        use crate::engine::types::coord10::Vel10;
        // INT-03: 루즈볼 경합 로직
        let mut queue = ActionQueue::new();
        queue.current_tick = 100;

        let ctx = create_test_context();

        // 루즈볼 상태 설정
        let ball_pos = (40.0, 35.0);
        queue.ball_state = BallState::Loose {
            position: Coord10::from_meters(ball_pos.0, ball_pos.1),
            velocity: Vel10::from_mps(2.0, 1.0),
        };

        // 경합 시작
        let contest =
            queue.start_loose_ball_contest(ball_pos, &ctx.player_positions, &ctx.player_stats);

        // 경합자가 있음 (가장 가까운 선수들)
        assert!(!contest.contestants.is_empty());

        // 인터셉트 액션이 예약됨
        assert!(queue.pending_count() >= 1);

        // 모든 예약된 액션이 Intercept인지 확인
        let actions = queue.get_actions_for_tick(200); // 충분히 큰 틱
        for action in &actions {
            assert!(matches!(action.action_type, ActionType::Intercept { .. }));
        }
    }

    #[test]
    fn test_fifty_fifty_contest() {
        // 50:50 경합 (두 선수가 비슷한 거리)
        let mut queue = ActionQueue::new();
        queue.current_tick = 100;

        // 두 선수를 비슷한 거리에 배치
        let player_positions = vec![
            (40.1, 35.0),   // 선수 0 - 거리 ~0.1m
            (40.2, 35.0),   // 선수 1 - 거리 ~0.2m
            (100.0, 100.0), // 선수 2 - 멀리
        ];

        // FIX_2601/0102: 태클 점수 기반 경합 (tackling 40%, aggression 20%, strength 20%, bravery 10%, agility 10%)
        let player_stats = vec![
            PlayerStats {
                tackling: 70,
                aggression: 60,
                strength: 65,
                bravery: 60,
                agility: 75,
                ..Default::default()
            },
            PlayerStats {
                tackling: 80,
                aggression: 70,
                strength: 70,
                bravery: 65,
                agility: 65,
                ..Default::default()
            },
            PlayerStats::default(),
        ];

        let ball_pos = (40.0, 35.0);

        let contest = queue.start_loose_ball_contest(ball_pos, &player_positions, &player_stats);

        // 50:50 경합
        assert!(contest.is_fifty_fifty);
        assert_eq!(contest.contestants.len(), 2);

        // 승자가 결정됨
        assert!(contest.winner.is_some());
    }

    #[test]
    fn test_goal_scored_interrupt() {
        // 골 득점 → 킥오프 상태
        let mut queue = ActionQueue::new();

        let reason = InterruptReason::GoalScored { home_team: true, scorer_idx: 9 };

        queue.handle_interrupt(
            &reason,
            Coord10::from_meters(field::LENGTH_M, field::CENTER_Y),
        );

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, home_team, position } => {
                assert_eq!(*restart_type, RestartType::KickOff);
                assert!(!*home_team); // 실점 팀(어웨이)이 킥오프
                assert_eq!(*position, Coord10::CENTER); // 센터
            }
            _ => panic!("Expected OutOfPlay state"),
        }
    }

    #[test]
    fn test_execute_save_off_target_goal_kick_position() {
        fn run_case(shooter_idx: usize, gk_idx: usize, expected_goal_kick_x: f32) {
            let mut queue = ActionQueue::new();
            queue.last_shot_xg = Some(0.2);
            queue.last_shooter_idx = Some(shooter_idx);

            let mut player_positions = vec![(field::CENTER_X, field::CENTER_Y); 22];
            player_positions[gk_idx] = (expected_goal_kick_x, field::CENTER_Y);

            let ctx = ExecutionContext {
                player_positions,
                player_stats: vec![PlayerStats::default(); 22],
                goalkeeper_indices: (0, 11),
                current_tick: 0,
                rng_seed: 12345,
                home_ctx: DirectionContext::new(true),
                away_ctx: DirectionContext::new(false),
                home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                rulebook_non_gk_handball_enabled: false,
                rulebook_non_gk_handball_prob_mult: 1.0,
                rulebook_advantage_play_enabled: false,
                ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
            };

            let action = ScheduledAction::new(
                0,
                ActionType::Save {
                    direction: (field::CENTER_X, 40.0), // off-target (y > 37.5)
                },
                gk_idx,
                100,
                0,
            );

            let _ = execute_save(&action, &ctx, &mut queue, 0.15);

            match queue.ball_state() {
                BallState::OutOfPlay { restart_type, position, .. } => {
                    assert_eq!(*restart_type, RestartType::GoalKick);
                    assert_eq!(*position, Coord10::from_meters(expected_goal_kick_x, field::CENTER_Y));
                }
                _ => panic!("Expected OutOfPlay state"),
            }
        }

        // 홈 슈터 → 어웨이 골킥 (x=99m)
        run_case(9, 11, 99.0);
        // 어웨이 슈터 → 홈 골킥 (x=6m)
        run_case(20, 0, 6.0);
    }

    #[test]
    fn test_execute_save_gk_outside_own_penalty_area_is_handling_violation() {
        // Away team shoots to Home goal (x=0). Home GK attempts a save but is outside own penalty area.
        let mut queue = ActionQueue::new();
        queue.last_shot_xg = Some(0.01);
        queue.last_shooter_idx = Some(20); // away player
        queue.ball_state = BallState::InFlight {
            from_pos: Coord10::from_meters(25.0, field::CENTER_Y),
            to_pos: Coord10::from_meters(0.0, field::CENTER_Y),
            start_tick: 0,
            end_tick: 10,
            height_profile: HeightProfile::Flat,
            lift_ratio: 0.0,
            intended_receiver: None,
            is_shot: true,
            start_height_01m: 0,
            end_height_01m: 0,
        };

        let mut player_positions = vec![(field::CENTER_X, field::CENTER_Y); 22];
        player_positions[0] = (17.0, field::CENTER_Y); // Home GK (left goal) just outside 16.5m box
        player_positions[20] = (25.0, field::CENTER_Y); // Away shooter

        let mut player_stats = vec![PlayerStats::default(); 22];
        player_stats[0].reflexes = 100;
        player_stats[0].handling = 100;
        player_stats[0].positioning = 100;
        player_stats[0].diving = 100;
        player_stats[0].composure = 100;
        player_stats[0].anticipation = 100;
        player_stats[0].concentration = 100;

        let ctx = ExecutionContext {
            player_positions,
            player_stats,
            goalkeeper_indices: (0, 11),
            current_tick: 0,
            rng_seed: 0, // deterministic; ensures save path is reachable       
            home_ctx: DirectionContext::new(true), // Home attacks right -> defends left goal (x=0)
            away_ctx: DirectionContext::new(false),
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            rulebook_non_gk_handball_enabled: false,
            rulebook_non_gk_handball_prob_mult: 1.0,
            rulebook_advantage_play_enabled: false,
            ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
        };

        let action = ScheduledAction::new(
            0,
            ActionType::Save { direction: (0.0, field::CENTER_Y) }, // on-target center
            0,                                          // home GK
            100,
            0,
        );

        let result = execute_save(&action, &ctx, &mut queue, 0.15);
        assert!(matches!(
            result,
            ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: 0,
                last_touch_idx: Some(20),
                is_indirect: false,
                xg: Some(_),
            }
        ));

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, position, home_team } => {
                assert_eq!(*restart_type, RestartType::FreeKick);
                assert_eq!(*position, Coord10::from_meters(17.0, field::CENTER_Y));
                assert!(!*home_team, "away team should receive the free kick");
            }
            _ => panic!("Expected OutOfPlay free kick after GK handling violation"),
        }
    }

    #[test]
    fn test_execute_trap_gk_claim_backpass_inside_own_penalty_area_is_indirect_violation() {
        // Home GK claims an aerial back-pass inside own penalty area -> indirect FK.
        let mut queue = ActionQueue::new();
        queue.last_passer_idx = Some(5); // home teammate
        queue.last_pass_receiver_idx = Some(0); // intended for home GK

        let mut player_positions = vec![(field::CENTER_X, field::CENTER_Y); 22];
        player_positions[0] = (5.0, field::CENTER_Y); // inside left penalty area
        player_positions[5] = (20.0, field::CENTER_Y);

        let ctx = ExecutionContext {
            player_positions,
            player_stats: vec![PlayerStats::default(); 22],
            goalkeeper_indices: (0, 11),
            current_tick: 0,
            rng_seed: 12345,
            home_ctx: DirectionContext::new(true), // Home attacks right -> defends left goal (x=0)
            away_ctx: DirectionContext::new(false),
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            rulebook_non_gk_handball_enabled: false,
            rulebook_non_gk_handball_prob_mult: 1.0,
            rulebook_advantage_play_enabled: false,
            ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
        };

        let action = ScheduledAction::new(
            0,
            ActionType::Trap { ball_speed: 15.0, ball_height: 2.0 },
            0, // home GK
            100,
            0,
        );

        let result = execute_trap(&action, &ctx, &mut queue);
        assert!(matches!(
            result,
            ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: 0,
                last_touch_idx: Some(5),
                is_indirect: true,
                xg: None,
            }
        ));

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, position, home_team } => {
                assert_eq!(*restart_type, RestartType::FreeKick);
                assert_eq!(*position, Coord10::from_meters(5.0, field::CENTER_Y));
                assert!(!*home_team, "away team should receive the indirect free kick");
            }
            _ => panic!("expected an OutOfPlay free kick after back-pass handling violation"),
        }
    }

    #[test]
    fn test_execute_trap_gk_claim_outside_own_penalty_area_is_direct_violation() {
        // Home GK claims an aerial ball outside own penalty area -> direct FK (Phase 0 rule).
        let mut queue = ActionQueue::new();
        queue.last_passer_idx = Some(5); // home teammate (source is irrelevant outside PA)
        queue.last_pass_receiver_idx = Some(0); // intended for home GK

        let mut player_positions = vec![(field::CENTER_X, field::CENTER_Y); 22];
        player_positions[0] = (17.0, field::CENTER_Y); // just outside 16.5m box
        player_positions[5] = (20.0, field::CENTER_Y);

        let ctx = ExecutionContext {
            player_positions,
            player_stats: vec![PlayerStats::default(); 22],
            goalkeeper_indices: (0, 11),
            current_tick: 0,
            rng_seed: 12345,
            home_ctx: DirectionContext::new(true),
            away_ctx: DirectionContext::new(false),
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            rulebook_non_gk_handball_enabled: false,
            rulebook_non_gk_handball_prob_mult: 1.0,
            rulebook_advantage_play_enabled: false,
            ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
        };

        let action = ScheduledAction::new(
            0,
            ActionType::Trap { ball_speed: 15.0, ball_height: 2.0 },
            0, // home GK
            100,
            0,
        );

        let result = execute_trap(&action, &ctx, &mut queue);
        assert!(matches!(
            result,
            ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: 0,
                last_touch_idx: Some(5),
                is_indirect: false,
                xg: None,
            }
        ));

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, position, home_team } => {
                assert_eq!(*restart_type, RestartType::FreeKick);
                assert_eq!(*position, Coord10::from_meters(17.0, field::CENTER_Y));
                assert!(!*home_team, "away team should receive the direct free kick");
            }
            _ => panic!("expected an OutOfPlay free kick after GK handling violation"),
        }
    }

    #[test]
    fn test_execute_trap_gk_claim_without_pass_metadata_is_legal() {
        // If there is no PassStarted metadata (e.g., teammate header / loose ball proxy),
        // a GK aerial claim inside own PA should be legal and deterministic.
        let mut queue = ActionQueue::new();
        queue.last_passer_idx = None;
        queue.last_pass_receiver_idx = None;

        let mut player_positions = vec![(field::CENTER_X, field::CENTER_Y); 22];
        player_positions[0] = (5.0, field::CENTER_Y); // inside own PA

        let ctx = ExecutionContext {
            player_positions,
            player_stats: vec![PlayerStats::default(); 22],
            goalkeeper_indices: (0, 11),
            current_tick: 0,
            rng_seed: 12345,
            home_ctx: DirectionContext::new(true),
            away_ctx: DirectionContext::new(false),
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            rulebook_non_gk_handball_enabled: false,
            rulebook_non_gk_handball_prob_mult: 1.0,
            rulebook_advantage_play_enabled: false,
            ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
        };

        let action = ScheduledAction::new(
            0,
            ActionType::Trap { ball_speed: 15.0, ball_height: 2.0 },
            0, // home GK
            100,
            0,
        );

        let result = execute_trap(&action, &ctx, &mut queue);
        assert!(matches!(result, ActionResult::TrapSuccess { player_idx: 0 }));
        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 0 }));
    }

    #[test]
    fn test_execute_trap_gk_claim_throw_in_from_same_team_is_indirect_violation() {
        // Home GK claims an aerial throw-in from a teammate inside own penalty area -> indirect FK.
        let mut queue = ActionQueue::new();
        queue.in_flight_origin = Some(InFlightOrigin::ThrowIn { throwing_home: true });
        queue.last_passer_idx = None;
        queue.last_pass_receiver_idx = None;

        let mut player_positions = vec![(field::CENTER_X, field::CENTER_Y); 22];
        player_positions[0] = (5.0, field::CENTER_Y); // inside own PA

        let ctx = ExecutionContext {
            player_positions,
            player_stats: vec![PlayerStats::default(); 22],
            goalkeeper_indices: (0, 11),
            current_tick: 0,
            rng_seed: 12345,
            home_ctx: DirectionContext::new(true),
            away_ctx: DirectionContext::new(false),
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            rulebook_non_gk_handball_enabled: false,
            rulebook_non_gk_handball_prob_mult: 1.0,
            rulebook_advantage_play_enabled: false,
            ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
        };

        let action = ScheduledAction::new(
            0,
            ActionType::Trap { ball_speed: 12.0, ball_height: 2.0 },
            0, // home GK
            100,
            0,
        );

        let result = execute_trap(&action, &ctx, &mut queue);
        assert!(matches!(
            result,
            ActionResult::GoalkeeperHandlingViolation {
                goalkeeper_idx: 0,
                last_touch_idx: None,
                is_indirect: true,
                xg: None,
            }
        ));

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, position, home_team } => {
                assert_eq!(*restart_type, RestartType::FreeKick);
                assert_eq!(*position, Coord10::from_meters(5.0, field::CENTER_Y));
                assert!(!*home_team, "away team should receive the indirect free kick");
            }
            _ => panic!("expected an OutOfPlay free kick after throw-in handling violation"),
        }

        queue.record_result(result);
        assert_eq!(queue.in_flight_origin, None);
    }

    #[test]
    fn test_execute_trap_gk_claim_throw_in_from_opponent_is_legal() {
        // Home GK may legally handle a ball received directly from an opponent throw-in.
        let mut queue = ActionQueue::new();
        queue.in_flight_origin = Some(InFlightOrigin::ThrowIn { throwing_home: false });
        queue.last_passer_idx = None;
        queue.last_pass_receiver_idx = None;

        let mut player_positions = vec![(field::CENTER_X, field::CENTER_Y); 22];
        player_positions[0] = (5.0, field::CENTER_Y); // inside own PA

        let ctx = ExecutionContext {
            player_positions,
            player_stats: vec![PlayerStats::default(); 22],
            goalkeeper_indices: (0, 11),
            current_tick: 0,
            rng_seed: 12345,
            home_ctx: DirectionContext::new(true),
            away_ctx: DirectionContext::new(false),
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            rulebook_non_gk_handball_enabled: false,
            rulebook_non_gk_handball_prob_mult: 1.0,
            rulebook_advantage_play_enabled: false,
            ball_position: (field::CENTER_X, field::CENTER_Y), // FIX_2601/1120
        };

        let action = ScheduledAction::new(
            0,
            ActionType::Trap { ball_speed: 12.0, ball_height: 2.0 },
            0, // home GK
            100,
            0,
        );

        let result = execute_trap(&action, &ctx, &mut queue);
        assert!(matches!(result, ActionResult::TrapSuccess { player_idx: 0 }));
        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 0 }));

        queue.record_result(result);
        assert_eq!(queue.in_flight_origin, None);
    }

    #[test]
    fn test_out_of_bounds_throw_in() {
        // 터치라인 아웃 → 스로인
        let mut queue = ActionQueue::new();

        let reason = InterruptReason::OutOfBounds {
            position: Coord10::from_meters(50.0, 0.0), // 터치라인
            last_touch_home: true,
        };

        queue.handle_interrupt(&reason, Coord10::from_meters(50.0, 0.0));

        match queue.ball_state() {
            BallState::OutOfPlay { restart_type, home_team, .. } => {
                assert_eq!(*restart_type, RestartType::ThrowIn);
                assert!(!*home_team); // 홈팀이 마지막 터치 → 어웨이팀 스로인
            }
            _ => panic!("Expected OutOfPlay state"),
        }
    }

    #[test]
    fn test_ball_won_interrupt() {
        // 볼 탈취 → 소유권 변경
        let mut queue = ActionQueue::with_ball_state(BallState::Controlled { owner_idx: 8 });

        // 드리블 예약
        queue.schedule_new(
            10,
            ActionType::Dribble { direction: (1.0, 0.0), aggressive: true },
            8,
            100,
        );

        let reason = InterruptReason::BallWon { winner_idx: 15, loser_idx: 8 };

        let cancelled = queue.handle_interrupt(&reason, Coord10::from_meters(50.0, 35.0));

        // 드리블 취소
        assert_eq!(cancelled.len(), 1);

        // 공 소유권 변경
        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 15 }));
    }

    #[test]
    fn test_interrupt_reason_requires_restart() {
        // 재시작 필요 여부 확인
        assert!(InterruptReason::Foul {
            offender_idx: 0,
            victim_idx: 1,
            position: Coord10::from_meters(0.0, 0.0),
            severity: 0.5
        }
        .requires_restart());

        assert!(InterruptReason::OutOfBounds {
            position: Coord10::from_meters(0.0, 0.0),
            last_touch_home: true
        }
        .requires_restart());

        assert!(InterruptReason::GoalScored { home_team: true, scorer_idx: 0 }.requires_restart());

        // 충돌과 볼탈취는 재시작 불필요
        assert!(!InterruptReason::PhysicsCollision { object_idx: 0, impact_force: 0.5 }
            .requires_restart());

        assert!(!InterruptReason::BallWon { winner_idx: 0, loser_idx: 1 }.requires_restart());
    }

    #[test]
    fn test_check_and_start_loose_ball() {
        let mut queue = ActionQueue::new();
        queue.current_tick = 100;

        let ctx = create_test_context();

        // Controlled 상태 → None
        queue.ball_state = BallState::Controlled { owner_idx: 5 };
        let result = queue.check_and_start_loose_ball(&ctx.player_positions, &ctx.player_stats);
        assert!(result.is_none());

        // Loose 상태 → 경합 시작
        queue.ball_state = BallState::Loose {
            position: Coord10::from_meters(50.0, field::CENTER_Y),
            velocity: Vel10::default(),
        };
        let result = queue.check_and_start_loose_ball(&ctx.player_positions, &ctx.player_stats);
        assert!(result.is_some());

        // 이미 인터셉트 예약됨 → None
        let result2 = queue.check_and_start_loose_ball(&ctx.player_positions, &ctx.player_stats);
        assert!(result2.is_none());
    }

    // ========== Phase 3.5: Ball Sync Tests ==========

    #[test]
    fn test_sync_from_ball_controlled() {
        use super::super::Ball;

        let mut queue = ActionQueue::new();
        let mut ball = Ball::default();
        ball.current_owner = Some(5);

        queue.sync_from_ball(0, &ball);

        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 5 }));
    }

    #[test]
    fn test_sync_from_ball_in_flight() {
        use super::super::Ball;

        let mut queue = ActionQueue::new();
        let mut ball = Ball::default();
        ball.is_in_flight = true;
        ball.from_position = Some(Coord10::from_meters(31.5, 27.2)); // ~0.3*105=31.5m, ~0.4*68=27.2m
        ball.to_position = Some(Coord10::from_meters(63.0, field::CENTER_Y)); // ~0.6*105=63m, ~0.5*68=34m
        ball.flight_speed = 0.5;
        ball.flight_progress = 0.5;
        ball.pending_owner = Some(7);

        queue.sync_from_ball(0, &ball);

        match queue.ball_state() {
            BallState::InFlight { from_pos, to_pos, intended_receiver, .. } => {
                assert_eq!(*from_pos, Coord10::from_meters(31.5, 27.2));
                assert_eq!(*to_pos, Coord10::from_meters(63.0, field::CENTER_Y));
                assert_eq!(*intended_receiver, Some(7));
            }
            _ => panic!("Expected InFlight state"),
        }
    }

    #[test]
    fn test_sync_from_ball_loose() {
        use super::super::Ball;

        let mut queue = ActionQueue::new();
        let mut ball = Ball::default();
        ball.current_owner = None;
        ball.is_in_flight = false;
        ball.position = Coord10::from_meters(42.0, field::CENTER_Y); // ~0.4*105=42m, ~0.5*68=34m
        ball.velocity = Vel10::from_mps(1.0, 0.5); // 1.0 m/s, 0.5 m/s

        queue.sync_from_ball(0, &ball);

        match queue.ball_state() {
            BallState::Loose { position, velocity } => {
                assert_eq!(*position, Coord10::from_meters(42.0, field::CENTER_Y));
                assert_eq!(*velocity, Vel10::from_mps(1.0, 0.5));
            }
            _ => panic!("Expected Loose state"),
        }
    }

    #[test]
    fn test_sync_to_ball_controlled() {
        use super::super::Ball;

        let mut queue = ActionQueue::new();
        queue.ball_state = BallState::Controlled { owner_idx: 8 };

        let mut ball = Ball::default();
        ball.current_owner = Some(3); // 이전 소유자

        queue.sync_to_ball(&mut ball);

        assert_eq!(ball.current_owner, Some(8));
        assert_eq!(ball.previous_owner, Some(3));
        assert!(!ball.is_in_flight);
        assert!(ball.pending_owner.is_none());
    }

    #[test]
    fn test_sync_to_ball_in_flight() {
        use super::super::Ball;

        let mut queue = ActionQueue::new();
        queue.current_tick = 50;
        let from = Coord10::from_meters(21.0, 20.4); // ~0.2*105=21m, ~0.3*68=20.4m
        let to = Coord10::from_meters(84.0, 47.6); // ~0.8*105=84m, ~0.7*68=47.6m
        queue.ball_state = BallState::InFlight {
            from_pos: from,
            to_pos: to,
            height_profile: HeightProfile::Arc,
            lift_ratio: 1.0,
            start_tick: 40,
            end_tick: 60,
            intended_receiver: Some(5),
            is_shot: false,
            start_height_01m: 0,
            end_height_01m: 0,
        };

        let mut ball = Ball::default();
        queue.sync_to_ball(&mut ball);

        assert!(ball.is_in_flight);
        assert_eq!(ball.from_position, Some(from));
        assert_eq!(ball.to_position, Some(to));
        assert_eq!(ball.pending_owner, Some(5));
        assert!((ball.flight_progress - 0.5).abs() < 0.01); // 50%
                                                            // 위치 확인 (중간점) - Coord10 is i32, check in meters
        let (mx, my) = ball.position.to_meters();
        let expected_x = (21.0 + 84.0) / 2.0; // midpoint x
        let expected_y = (20.4 + 47.6) / 2.0; // midpoint y
        assert!((mx - expected_x).abs() < 0.2, "x={} expected={}", mx, expected_x);
        assert!((my - expected_y).abs() < 0.2, "y={} expected={}", my, expected_y);
    }

    #[test]
    fn test_inflight_arrival_conversion_is_post_actions_only() {
        use super::super::Ball;

        // Contract invariant:
        // - Pre-action sync must NOT convert `InFlight -> Loose` at `end_tick`.
        // - Post-action conversion (`advance_ball_state_post_actions`) owns that transition.
        let from = Coord10::from_meters(40.0, field::CENTER_Y);
        let to = Coord10::from_meters(65.0, field::CENTER_Y);
        let end_tick = 10;

        let mut queue = ActionQueue::new();
        queue.ball_state = BallState::InFlight {
            from_pos: from,
            to_pos: to,
            height_profile: HeightProfile::Arc,
            lift_ratio: 1.0,
            start_tick: 0,
            end_tick,
            intended_receiver: Some(7),
            is_shot: false,
            start_height_01m: 0,
            end_height_01m: 0,
        };

        let mut ball = Ball::default();
        ball.is_in_flight = true;
        ball.from_position = Some(from);
        ball.to_position = Some(to);
        ball.flight_speed = 20.0;
        ball.flight_progress = 1.0;
        ball.pending_owner = Some(7);
        ball.position = to;
        ball.height_profile = HeightProfile::Arc;

        // Pre-action stage: must preserve `InFlight` (even at end_tick / progress=1.0).
        queue.sync_from_ball(end_tick, &ball);
        assert!(matches!(queue.ball_state(), BallState::InFlight { .. }));

        // Post-action stage: commits arrival conversion.
        queue.advance_ball_state_post_actions(end_tick);
        assert!(matches!(
            queue.ball_state(),
            BallState::Loose { position, .. } if *position == to
        ));
    }

    #[test]
    fn test_sync_to_ball_loose() {
        use super::super::Ball;

        let mut queue = ActionQueue::new();
        let pos = Coord10::from_meters(field::CENTER_X, 27.2); // x=center, y≈0.4*WIDTH_M (27.2m)
        let vel = Vel10::from_mps(0.5, -0.2); // 0.5 m/s, -0.2 m/s
        queue.ball_state = BallState::Loose { position: pos, velocity: vel };

        let mut ball = Ball::default();
        ball.current_owner = Some(3);

        queue.sync_to_ball(&mut ball);

        assert!(ball.current_owner.is_none());
        assert_eq!(ball.previous_owner, Some(3));
        assert!(!ball.is_in_flight);
        assert_eq!(ball.position, pos);
        assert_eq!(ball.velocity, vel);
    }

    #[test]
    fn test_sync_roundtrip() {
        use super::super::Ball;

        // Controlled → Ball → BallState
        let mut queue = ActionQueue::new();
        let mut ball = Ball::default();
        ball.current_owner = Some(5);
        ball.position = Coord10::from_meters(31.5, 27.2); // ~0.3*105=31.5m, ~0.4*68=27.2m

        queue.sync_from_ball(0, &ball);
        queue.sync_to_ball(&mut ball);

        assert_eq!(ball.current_owner, Some(5));
    }

    #[test]
    fn test_from_ball_constructor() {
        use super::super::Ball;

        let mut ball = Ball::default();
        ball.current_owner = Some(10);

        let queue = ActionQueue::from_ball(&ball);

        assert!(matches!(queue.ball_state(), BallState::Controlled { owner_idx: 10 }));
    }

    // ========== P7 Phase-Based FSM Tests ==========
    // 레거시 모드 제거 (2025-12-12): FSM 전용으로 통합

    #[test]
    fn test_p7_activate_pending_actions() {
        let mut queue = ActionQueue::new();

        // Pass 액션 예약 (tick 10)
        queue.schedule_new(
            10,
            ActionType::Pass { target_idx: 7, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None },
            3,
            100,
        );

        assert_eq!(queue.pending_count(), 1);
        assert_eq!(queue.active_count(), 0);

        // tick 10에서 활성화
        let activated = queue.activate_pending_actions(
            10,
            |idx| if TeamSide::is_home(idx) { 0 } else { 1 },
            |_, _| true, // 항상 시작 가능
        );

        assert_eq!(activated.len(), 1);
        assert_eq!(activated[0].0, 3); // player_idx
        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.active_count(), 1);

        // 활성화된 액션 확인
        let active = queue.get_active_action(0).unwrap();
        assert_eq!(active.player_idx, 3);
        assert_eq!(active.action_type, PhaseActionType::Pass);
        // Pass는 Approach 없이 바로 Commit
        assert_eq!(active.phase, ActionPhase::Commit);
    }

    #[test]
    fn test_p7_activate_blocked_by_can_start() {
        let mut queue = ActionQueue::new();

        // Tackle 액션 예약
        queue.schedule_new(10, ActionType::Tackle { target_idx: 15 }, 5, 100);

        // can_start_fn이 false 반환 → 다음 틱으로 미룸
        let activated = queue.activate_pending_actions(10, |_| 0, |_, _| false);

        assert_eq!(activated.len(), 0);
        assert_eq!(queue.pending_count(), 1);

        // 다음 틱에 실행 시간이 11로 변경됨
        let next = queue.peek_next().unwrap();
        assert_eq!(next.execute_tick, 11);
    }

    #[test]
    fn test_p7_tick_active_actions() {
        let mut queue = ActionQueue::new();

        // Shot 액션 예약 (windup 3틱)
        queue.schedule_new(
            0,
            ActionType::Shot {
                power: 0.9,
                target: Coord10::from_meters(field::LENGTH_M, field::CENTER_Y),
            },
            9,
            100,
        );

        // tick 0에서 활성화
        queue.activate_pending_actions(0, |_| 0, |_, _| true);

        let active = queue.get_active_action(0).unwrap();
        assert_eq!(active.phase, ActionPhase::Commit);
        assert_eq!(active.phase_duration, 3); // Normal shot windup = 3 ticks

        // tick 1, 2: 아직 Commit 중
        let resolve = queue.tick_active_actions(1);
        assert!(resolve.is_empty());

        let resolve = queue.tick_active_actions(2);
        assert!(resolve.is_empty());

        // tick 3: Commit 완료 → Resolve로 전환
        let resolve = queue.tick_active_actions(3);
        assert_eq!(resolve.len(), 1);
        assert_eq!(resolve[0], 0);

        // 액션이 Resolve로 전환됨
        let active = queue.get_active_action(0).unwrap();
        assert_eq!(active.phase, ActionPhase::Resolve);
    }

    #[test]
    fn test_p7_action_to_recover() {
        let mut queue = ActionQueue::new();

        // Pass 액션 생성 및 활성화
        queue.schedule_new(
            0,
            ActionType::Pass { target_idx: 5, is_long: false, is_through: false, intended_target_pos: None, intended_passer_pos: None },
            3,
            100,
        );
        queue.activate_pending_actions(0, |_| 0, |_, _| true);

        // Resolve 단계로 전환 가정 후 action_to_recover 호출
        queue.action_to_recover(0, 5);

        let active = queue.get_active_action(0).unwrap();
        assert_eq!(active.phase, ActionPhase::Recover);
        assert_eq!(active.phase_duration, 2); // PASS_RECOVERY_TICKS = 2
    }

    #[test]
    fn test_p7_remove_finished_actions() {
        let mut queue = ActionQueue::new();

        // 액션 2개 생성
        queue.schedule_new(0, ActionType::Tackle { target_idx: 15 }, 5, 100);
        queue.schedule_new(0, ActionType::Tackle { target_idx: 16 }, 6, 100);
        queue.activate_pending_actions(0, |_| 0, |_, _| true);

        assert_eq!(queue.active_count(), 2);

        let first_player = queue.get_active_action(0).expect("active[0]").player_idx;
        let second_player = queue.get_active_action(1).expect("active[1]").player_idx;
        assert_ne!(first_player, second_player, "sanity: two distinct active players");

        // 첫 번째 액션만 Finished로 설정
        queue.finish_action(0);

        // Finished 액션 제거
        queue.remove_finished_actions();

        assert_eq!(queue.active_count(), 1);
        // 남은 액션은 이전 active[1]
        let remaining = queue.get_active_action(0).unwrap();
        assert_eq!(remaining.player_idx, second_player);
    }

    #[test]
    fn test_p7_cancel_active_for_player() {
        let mut queue = ActionQueue::new();

        queue.schedule_new(0, ActionType::Tackle { target_idx: 15 }, 5, 100);
        queue.schedule_new(0, ActionType::Tackle { target_idx: 16 }, 6, 100);
        queue.activate_pending_actions(0, |_| 0, |_, _| true);

        assert_eq!(queue.active_count(), 2);

        // player 5의 액션 취소
        queue.cancel_active_for_player(5);

        assert_eq!(queue.active_count(), 1);
        assert!(!queue.is_player_active(5));
        assert!(queue.is_player_active(6));
    }
}

