//! Decision Topology (P16 Phase 2: Gate Chain Architecture)
//!
//! 3-Gate 의사결정 파이프라인:
//! - Gate A: Mindset Filter (후보 필터링)
//! - Gate B: Utility Selection (Softmax 기반 선택)
//! - Gate C: ActionModel Elaboration (Intent → Technique)
//!
//! ## 핵심 원칙
//! - Bias는 Gate B에서만 적용
//! - Gate C는 순수 실행 (Bias 없음)
//! - EV 계산은 Gate B까지만 (이후에는 없음)

use rand::Rng;

use super::cognitive_bias::CognitiveBias;
use super::defense_intent::{ChallengeTechnique, ContainTechnique, DefenseContext, PressTechnique};
use super::pitch_zone::PitchZone;
use super::utility::{
    calculate_temperature, calculate_utility_result, softmax_select, CandidateFacts, UtilityResult,
};
use super::zone_transition;
use super::StickyActions;

use crate::engine::force_field; // FIX_2601/0112: Force Field Navigation
use crate::engine::physics_constants::field;
use crate::engine::weights::WeightBreakdown;

// FIX_2601/0113: UAE-GateB Integration
use crate::engine::action_evaluator::evaluators::{EvalContext, EvaluatorRegistry};
use crate::engine::action_evaluator::types::{
    Action, ActionScore, CrossZone, PassLane, PlayerId, Position, Vec2, Zone,
};

// ============================================================================
// CandidateAction
// ============================================================================

/// 모든 가능한 행동 후보
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateAction {
    // === Attack: Protect ===
    ShieldBall,
    HoldUpPlay,
    DrawFoul,

    // === Attack: Progress ===
    SafePass,
    ProgressivePass,
    SwitchPlay,
    CarryBall,

    // === Attack: Beat ===
    TakeOn,
    OneTwo,
    ThroughBall,

    // === Attack: Score ===
    ShootNormal,
    ShootFinesse,
    ShootPower,
    ShootChip,
    Header,
    Cross,  // FIX_2601/0117: Cross action for aerial deliveries

    // === Defense: Contain ===
    Jockey,
    DelayPass,
    CoverSpace,
    BlockLane,

    // === Defense: Press ===
    ClosingDown,
    InterceptAttempt,
    ForceTouchline,
    TrackRunner,

    // === Defense: Challenge ===
    StandingTackle,
    SlidingTackle,
    ShoulderCharge,
    PokeAway,

    // === Transition ===
    CounterAttackRun,
    RecoveryRun,

    // === Special ===
    ClearBall,
    Hold,
}

impl CandidateAction {
    /// 이 액션이 이기적(개인 플레이)인지 여부
    pub fn is_selfish(&self) -> bool {
        matches!(
            self,
            Self::ShootNormal
                | Self::ShootFinesse
                | Self::ShootPower
                | Self::ShootChip
                | Self::Header
                | Self::TakeOn
                | Self::OneTwo
        )
    }

    /// 이 액션이 패스 계열인지 여부
    pub fn is_pass_like(&self) -> bool {
        matches!(
            self,
            Self::SafePass
                | Self::ProgressivePass
                | Self::SwitchPlay
                | Self::ThroughBall
                | Self::ClearBall
        )
    }

    /// 이 액션의 기본 위험 수준 (0~1, 높을수록 위험)
    pub fn risk_level(&self) -> f32 {
        match self {
            // High risk
            Self::ShootPower => 0.8,
            Self::SlidingTackle => 0.75,
            Self::ThroughBall => 0.7,
            Self::TakeOn => 0.65,
            Self::OneTwo => 0.6,
            Self::ShootChip => 0.6,
            Self::ShoulderCharge => 0.55,

            // Medium risk
            Self::ShootNormal => 0.5,
            Self::ShootFinesse => 0.5,
            Self::Header => 0.5,
            Self::ProgressivePass => 0.4,
            Self::StandingTackle => 0.4,
            Self::InterceptAttempt => 0.35,

            // Low risk
            Self::SafePass => 0.2,
            Self::CarryBall => 0.25,
            Self::Jockey => 0.15,
            Self::CoverSpace => 0.15,
            Self::BlockLane => 0.2,
            Self::ClosingDown => 0.2,
            Self::ShieldBall => 0.1,
            Self::HoldUpPlay => 0.15,
            Self::Hold => 0.05,

            _ => 0.3,
        }
    }
}

// ============================================================================
// SelectedIntent (Gate B Output)
// ============================================================================

/// Gate B 출력: 선택된 의도
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectedIntent {
    // Attack Intents
    Protect(ProtectStyle),
    Progress(ProgressStyle),
    Beat(BeatStyle),
    Score(ScoreStyle),

    // Defense Intents
    Contain(ContainTechnique),
    Press(PressTechnique),
    Challenge(ChallengeTechnique),

    // Transition
    CounterAttack,
    Recovery,

    // Special
    Clear,
    Wait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectStyle {
    Shield,
    HoldUp,
    DrawFoul,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStyle {
    Safe,
    Progressive,
    Switch,
    Carry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeatStyle {
    TakeOn,
    OneTwo,
    Through,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreStyle {
    Normal,
    Finesse,
    Power,
    Chip,
    Header,
    Cross,  // FIX_2601/0117: Cross for aerial deliveries
}

// ============================================================================
// PlayerMindset
// ============================================================================

/// 선수의 현재 마인드셋 (Gate A 입력)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerMindset {
    // Attack
    AttackScore,    // 득점 시도
    AttackProgress, // 전진
    AttackProtect,  // 공 보호

    // Defense
    DefendContain,   // 지연/봉쇄
    DefendPress,     // 압박
    DefendChallenge, // 도전/탈취

    // Transition
    TransitionCounter, // 역습
    TransitionRecover, // 복귀

    // Special
    GkDistribute, // GK 배급
    GkSave,       // GK 세이브
}

// ============================================================================
// Outcome Sets (Contract v1.0)
// ============================================================================

/// 점유권 관련 결과 (상호배타)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossessionOutcome {
    Continue,
    Turnover,
    Foul,
    Offside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotOutcome {
    Goal,
    Saved,
    Blocked,
    OnTarget,
    OffTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotAccuracyOutcome {
    OnTarget,
    OffTarget,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotFinishOutcome {
    Goal,
    Saved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TackleOutcome {
    WinBall,
    Miss,
    Foul,
    InjuryRare,
}

// =======================================================================
// Event Mix Profile
// =======================================================================

/// Team-level event mix profile for Gate B biasing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventMixProfile {
    #[default]
    Balanced,
    Possession,
    Counterattack,
    Defensive,
}

// ============================================================================
// Decision Context
// ============================================================================

/// 의사결정에 필요한 컨텍스트
#[derive(Debug, Clone)]
pub struct DecisionContext {
    /// xG (Expected Goals)
    pub xg: f32,
    /// 골대까지 거리 (미터)
    pub distance_to_goal: f32,
    /// 압박 레벨 (0~1) - FieldBoard 기반, 4m tactical influence
    pub local_pressure: f32,
    /// xG Zone 레벨 (0~0.30) - Match OS v1.2
    pub xgzone_level: f32,
    /// 즉시 압박 (0~1) - Match OS v1.2, 2m immediate tackle threat
    pub immediate_pressure: f32,
    /// 드리블 탈압박 스킬 (0~1) - Open-Football baseline + technique bonus
    pub dribble_escape_skill: f32,
    /// 드리블 스킬 기준 (Open-Football: dribbling > 15/20)
    pub dribble_skill_ok: bool,
    /// 드리블 안전거리 (Open-Football: 15m 내 상대 없음)
    pub dribble_safe_radius_ok: bool,
    /// 드리블 공간 스캔 (Open-Football: find dribbling space)
    pub dribble_space_scan_ok: bool,
    /// 공 소유 여부
    pub has_ball: bool,
    /// 수비 상황 여부
    pub is_defending: bool,
    /// 역습 상황 여부
    pub is_counter_attack: bool,
    /// 페널티박스 내 여부
    pub in_penalty_box: bool,
    /// 터치라인 근처 여부
    pub near_touchline: bool,
    /// 가장 가까운 팀 동료까지 거리
    pub nearest_teammate_dist: f32,
    /// 가장 가까운 상대까지 거리
    pub nearest_opponent_dist: f32,
    /// 패스 옵션 수
    pub pass_options_count: usize,
    /// 볼 캐리어까지 거리 (수비 시)
    pub distance_to_ball_carrier: f32,
    /// Defense Context (수비 시)
    pub defense_ctx: Option<DefenseContext>,
    /// 예산/빈도 관리용 (Budget Gate)
    pub action_history: Option<ActionHistory>,
    /// 전술 적용 여부 추적 (Contract v1.0 감시 로직)
    pub tactical_trace: Vec<String>,
    /// 팀 기준 피치 존 (공격 방향 기준 좌표계)
    pub team_pitch_zone: Option<PitchZone>,
    /// 팀 전술 기반 이벤트 믹스 프로필
    pub event_mix_profile: Option<EventMixProfile>,
    // ========================================================================
    // Team Tactical Knobs (TeamInstructions → deterministic factors)
    // ========================================================================
    /// Pressing factor from TeamInstructions (0.2..1.0).
    ///
    /// Note: defensive pressing behavior is primarily driven by DefensiveTuning.
    /// This value is kept here for decision-layer soft gates / evidence.
    pub team_pressing_factor: f32,
    /// Tempo factor from TeamInstructions (0.2..1.0).
    ///
    /// Used to adjust softmax temperature (Gate B) deterministically.
    pub team_tempo_factor: f32,
    /// Width bias from TeamInstructions (-5.0..+5.0 meters).
    ///
    /// Used to bias wide-play (Cross/SwitchPlay) without changing UI meaning.
    pub team_width_bias_m: f32,
    /// Risk bias from BuildUpStyle (-0.15..+0.20).
    ///
    /// Used to bias SafePass vs Progressive/Through/Switch (directionality contract).
    pub team_risk_bias: f32,
    /// FIX_2601/0105 Phase 2: 더 좋은 위치의 동료가 있는지 여부
    /// Open-Football 스타일: 동료가 현재 선수보다 70% 이내 거리에 있으면 true
    pub has_better_positioned_teammate: bool,
    /// FIX_2601/0105 Phase 2: 가장 좋은 위치 동료의 threat level (0-1)
    pub best_teammate_threat: f32,
    /// FIX_2601/0105 Phase 4: 이번 하프 슛 횟수 (팀)
    pub shots_this_half: u8,
    /// FIX_2601/0105 Phase 4: 슛 예산 (하프당 목표 슛 수)
    pub shot_budget_per_half: u8,
    /// FIX_2601/0106 P3: 빌드업 페이즈 (필드 위치 기반)
    pub buildup_phase: super::buildup_phase::BuildupPhase,
    /// Sticky action toggles for this player
    pub sticky_actions: StickyActions,
    /// 클리어 슛 라인 여부 (골대까지 레이캐스트 차단 없음)
    pub has_clear_shot: bool,
    /// 슛 각도 적합 여부
    pub good_shooting_angle: bool,
    /// 8m 이내 상대 수
    pub nearby_opponents_8m: u8,
    /// 롱슈팅 능력치 (0~20 or 0~100 스케일)
    pub long_shots_skill: f32,
    // FIX_2601/0116: Header/TakeOn/Cross fields
    /// 현재 공 높이 (미터)
    pub ball_height: f32,
    /// 공까지 거리 (미터)
    pub distance_to_ball: f32,
    /// 전방 공간 여부 (드리블 가능)
    pub has_space_ahead: bool,
    /// 드리블 포지션 여부 (공격수/윙어)
    pub is_dribble_position: bool,
    /// 측면 위치 여부 (터치라인 근처)
    pub is_wide_position: bool,
    /// 공격 1/3 영역 여부
    pub in_attacking_third: bool,
    /// 박스 내 동료 수
    pub teammates_in_box: u8,

    // ========== DPER Framework: Experimental Parameters ==========
    /// Experimental: Minimum xG threshold for shooting (default 0.05)
    /// Lower = more shots attempted, Higher = only high-quality chances
    pub exp_shoot_xg_threshold: f32,
    /// Experimental: Dribble EV bias (default 0.0)
    /// Positive = more dribbles, Negative = fewer dribbles
    pub exp_dribble_bias: f32,
    /// Experimental: Through ball multiplier (default 1.0)
    pub exp_through_ball_multiplier: f32,
    /// Experimental: Cross multiplier (default 1.0)
    pub exp_cross_multiplier: f32,
    /// Experimental: Directness bias (default 0.0)
    /// Positive = more direct play, Negative = more patient buildup
    pub exp_directness_bias: f32,

    // ========== FIX_2601/0112: ActionAttemptBias (Calibration) ==========
    /// Calibration: Progressive pass attempt bias (default 1.0)
    /// > 1.0 = more progressive passes attempted
    pub cal_progressive_pass_bias: f32,
    /// Calibration: Long pass attempt bias (default 1.0)
    /// > 1.0 = more long passes attempted
    pub cal_long_pass_bias: f32,
    /// Calibration: Cross attempt bias (default 1.0)
    pub cal_cross_bias: f32,
    /// Calibration: Shot attempt bias (default 1.0)
    /// > 1.0 = more shots attempted
    pub cal_shot_bias: f32,
    /// Calibration: Dribble attempt bias (default 1.0)
    pub cal_dribble_bias: f32,
    /// Calibration: Through ball attempt bias (default 1.0)
    pub cal_through_ball_bias: f32,
    /// FIX_2601/1128: Safe pass attempt bias (default 1.0)
    /// Increases in Circulation sub-phase to encourage backward/lateral passes
    pub cal_safe_pass_bias: f32,

    // ========== FIX_2601/1129: AttackPhase System ==========
    /// Current attack phase (Circulation/Positional/Transition)
    /// Controls forward pass availability and weight multipliers
    pub attack_phase: super::attack_phase::AttackPhase,
    /// FIX_2601/0123: SafePass return-bias sequence (match-local)
    /// Deterministic alternative to global counters for return-pass preference.
    pub safe_pass_seq: u64,
}

impl Default for DecisionContext {
    fn default() -> Self {
        Self {
            xg: 0.0,
            distance_to_goal: 50.0,
            local_pressure: 0.0,
            xgzone_level: 0.0,
            immediate_pressure: 0.0,
            dribble_escape_skill: 0.5,
            dribble_skill_ok: false,
            dribble_safe_radius_ok: false,
            dribble_space_scan_ok: false,
            has_ball: false,
            is_defending: false,
            is_counter_attack: false,
            in_penalty_box: false,
            near_touchline: false,
            nearest_teammate_dist: 10.0,
            nearest_opponent_dist: 10.0,
            pass_options_count: 0,
            distance_to_ball_carrier: 0.0,
            defense_ctx: None,
            action_history: None,
            tactical_trace: vec![],
            team_pitch_zone: None,
            event_mix_profile: None,
            team_pressing_factor: 0.6,
            team_tempo_factor: 0.6,
            team_width_bias_m: 0.0,
            team_risk_bias: 0.0,
            has_better_positioned_teammate: false,
            best_teammate_threat: 0.0,
            shots_this_half: 0,
            shot_budget_per_half: 0,
            buildup_phase: super::buildup_phase::BuildupPhase::MiddleThird,
            sticky_actions: StickyActions::default(),
            has_clear_shot: false,
            good_shooting_angle: false,
            nearby_opponents_8m: 0,
            long_shots_skill: 10.0,
            ball_height: 0.0,
            distance_to_ball: 0.0,
            has_space_ahead: false,
            is_dribble_position: false,
            is_wide_position: false,
            in_attacking_third: false,
            teammates_in_box: 0,
            // DPER: Baseline experimental parameters
            exp_shoot_xg_threshold: 0.05, // Default baseline
            exp_dribble_bias: 0.0,        // Neutral
            exp_through_ball_multiplier: 1.0, // No change
            exp_cross_multiplier: 1.0,    // No change
            exp_directness_bias: 0.0,     // Neutral
            // FIX_2601/0112: ActionAttemptBias defaults (neutral)
            cal_progressive_pass_bias: 1.0,
            cal_long_pass_bias: 1.0,
            cal_cross_bias: 1.0,
            cal_shot_bias: 1.0,
            cal_dribble_bias: 1.0,
            cal_through_ball_bias: 1.0,
            cal_safe_pass_bias: 1.0,
            // FIX_2601/1129: AttackPhase default (Circulation = most restrictive)
            attack_phase: super::attack_phase::AttackPhase::Circulation,
            safe_pass_seq: 0,
        }
    }
}

// ============================================================================
// P0-1: TeamInstructions Integration (Tempo / Risk Bias)
// ============================================================================

/// Apply team tempo to temperature (affects decision randomness)
///
/// VeryFast tempo → lower temperature (more greedy, quick decisions)
/// VerySlow tempo → higher temperature (more exploratory, patient build-up)
pub fn apply_team_tempo_temperature(
    base_temperature: f32,
    instructions: &crate::tactics::team_instructions::TeamInstructions,
) -> f32 {
    apply_team_tempo_temperature_factor(base_temperature, instructions.get_tempo_factor())
}

/// Apply team tempo factor (0.2..1.0) to softmax temperature.
///
/// Higher tempo → higher temperature (more exploratory/risky decisions).
/// Lower tempo → lower temperature (more conservative).
pub fn apply_team_tempo_temperature_factor(base_temperature: f32, tempo_factor: f32) -> f32 {
    // Map 0.2..1.0 → -2..+2 (VerySlow..VeryFast)
    let tempo_numeric = ((tempo_factor - 0.6) / 0.2).clamp(-2.0, 2.0);
    let tempo_modifier = tempo_numeric * 0.08; // ±0.16 at extremes
    (base_temperature * (1.0 + tempo_modifier)).clamp(0.15, 0.95)
}

/// Apply build-up style to risk bias
///
/// Short passing → negative risk bias (more conservative)
/// Direct → positive risk bias (more aggressive, long balls)
pub fn build_up_style_risk_delta(
    instructions: &crate::tactics::team_instructions::TeamInstructions,
) -> f32 {
    use crate::tactics::team_instructions::BuildUpStyle;

    match instructions.build_up_style {
        BuildUpStyle::Short => -0.15, // Conservative, reduce risky actions
        BuildUpStyle::Mixed => 0.0,   // Neutral
        BuildUpStyle::Direct => 0.20, // Aggressive, boost long passes/through balls
    }
}

/// Get stamina drain multiplier from team tempo
///
/// Faster tempo → higher stamina drain per action
pub fn get_stamina_drain_multiplier(
    instructions: &crate::tactics::team_instructions::TeamInstructions,
) -> f32 {
    instructions.team_tempo.stamina_drain_modifier()
}

/// Get pressing stamina cost multiplier
///
/// Higher pressing → higher stamina cost for defensive actions
pub fn get_pressing_stamina_cost(
    instructions: &crate::tactics::team_instructions::TeamInstructions,
) -> f32 {
    instructions.pressing_intensity.stamina_cost_modifier()
}

// ============================================================================
// P0 Patch 3: SINGLE INJECTION POINT for TeamInstructions → Engine Knobs
// ============================================================================

/// SINGLE INJECTION POINT: Convert TeamInstructions to execution knobs
///
/// Called once per team per decision cycle to populate MindsetContext and DefensiveTuning
/// from TeamInstructions. This is the ONLY place where tactics enter the engine.
///
/// # Arguments
/// * `instructions` - Optional TeamInstructions (None = use defaults)
/// * `base_context` - MindsetContext to populate with tactical knobs
/// * `defensive_tuning` - DefensiveTuning to populate with pressing/offside trap settings
///
/// # Example
/// ```ignore
/// use of_core::engine::mindset::MindsetContext;
/// use of_core::engine::defensive_positioning::DefensiveTuning;
/// use of_core::tactics::team_instructions::TeamInstructions;
/// use of_core::engine::match_sim::decision_topology::apply_team_instructions;
///
/// let mut context = MindsetContext::default();
/// let mut tuning = DefensiveTuning::default();
///
/// // Apply team instructions (if available)
/// apply_team_instructions(
///     Some(&TeamInstructions::default()),
///     &mut context,
///     &mut tuning,
/// );
/// ```
pub fn apply_team_instructions(
    instructions: Option<&crate::tactics::team_instructions::TeamInstructions>,
    base_context: &mut crate::engine::mindset::MindsetContext,
    defensive_tuning: &mut crate::engine::defensive_positioning::DefensiveTuning,
) {
    if let Some(ti) = instructions {
        // Update mindset knobs (affects decision pipeline)
        base_context.pressing_factor = ti.get_pressing_factor();
        base_context.tempo_factor = ti.get_tempo_factor();
        base_context.width_bias_m = ti.get_width_bias_m();

        // P0-B2: Apply build-up style risk bias
        base_context.risk_bias = build_up_style_risk_delta(ti);

        // Update defensive tuning (affects presser selection and defensive line)
        defensive_tuning.pressing_factor = ti.get_pressing_factor();
        defensive_tuning.offside_trap_enabled = ti.use_offside_trap;
    }
    // If instructions is None, base_context/defensive_tuning keep their defaults
}

// ============================================================================
// Weight helpers (local)
// ============================================================================

#[inline]
fn ln_weight(bd: &WeightBreakdown) -> f32 {
    // ln(0) 방지 포함
    bd.to_weight().ln()
}

/// 액션 이력 (Budget Gate용)
#[derive(Debug, Clone, Default)]
pub struct ActionHistory {
    pub recent_shots: u32,
    pub recent_tackles: u32,
    pub stamina: f32,
}

// ============================================================================
// Gate A: Mindset Filter
// ============================================================================

/// Gate A: Mindset 기반 후보 필터링
pub fn filter_candidates_by_mindset(
    mindset: PlayerMindset,
    ctx: &DecisionContext,
) -> Vec<CandidateAction> {
    let mut candidates = Vec::new();

    match mindset {
        PlayerMindset::AttackScore => {
            // 득점 우선
            candidates.extend(&[
                CandidateAction::ShootNormal,
                CandidateAction::ShootFinesse,
                CandidateAction::ShootPower,
            ]);
            // FIX_2601/0116: TakeOn 조건 완화 (목표 30-40회/경기)
            // 측면(사이드)에서 돌파가 자주 발생 - 무조건 허용
            // 중앙에서는 공간 있거나 상대 근접 시 허용
            if ctx.is_wide_position {
                candidates.push(CandidateAction::TakeOn); // 사이드 = 항상 돌파 가능
            } else {
                let can_dribble = ctx.has_space_ahead || ctx.nearest_opponent_dist < 10.0;
                if can_dribble {
                    candidates.push(CandidateAction::TakeOn);
                }
            }
            // FIX_2601/1129: Only add ProgressivePass if AttackPhase allows forward pass
            // Circulation phase: forward almost forbidden
            // Positional phase: conditional forward allowed
            // Transition phase: forward preferred
            if ctx.pass_options_count > 0 && ctx.attack_phase.allows_forward_pass() {
                candidates.push(CandidateAction::ProgressivePass);
            }
            candidates.push(CandidateAction::SafePass); // 항상 SafePass 가능
            // FIX_2601/0117: Cross for wide players in attacking third with teammates in box
            if ctx.is_wide_position && ctx.in_attacking_third && ctx.teammates_in_box > 0 {
                candidates.push(CandidateAction::Cross);
            }
        }

        PlayerMindset::AttackProgress => {
            // FIX_2601/1129: Only add ProgressivePass if AttackPhase allows forward pass
            if ctx.attack_phase.allows_forward_pass() {
                candidates.push(CandidateAction::ProgressivePass);
            }
            candidates.extend(&[
                CandidateAction::SafePass,
                CandidateAction::CarryBall,
            ]);
            // FIX_2601/0117: SwitchPlay 조건 완화 (2명 이상이면 허용)
            if ctx.pass_options_count > 1 {
                candidates.push(CandidateAction::SwitchPlay);
            }
            // FIX_2601/0116: TakeOn 조건 완화 (드리블 증가)
            // 측면에서는 무조건 돌파 가능
            if ctx.is_wide_position {
                candidates.push(CandidateAction::TakeOn);
            } else {
                let can_dribble = ctx.has_space_ahead || ctx.nearest_opponent_dist < 12.0;
                if can_dribble {
                    candidates.push(CandidateAction::TakeOn);
                }
            }
            // xG 높으면 슛 옵션 (FIX_2601/0115: threshold 0.08 allows shots from box edge)
            if ctx.xg > 0.08 {
                candidates.push(CandidateAction::ShootNormal);
            }
            // FIX_2601/0117: Cross for wide players in attacking third
            if ctx.is_wide_position && ctx.in_attacking_third && ctx.teammates_in_box > 0 {
                candidates.push(CandidateAction::Cross);
            }
        }

        PlayerMindset::AttackProtect => {
            candidates.extend(&[
                CandidateAction::ShieldBall,
                CandidateAction::HoldUpPlay,
                CandidateAction::SafePass,
            ]);
            // FIX_2601/0116: TakeOn - 탈압박 드리블 옵션
            // 드리블 능력 있거나 공간 있으면 허용
            if ctx.dribble_skill_ok || ctx.has_space_ahead {
                candidates.push(CandidateAction::TakeOn);
            }
            if ctx.local_pressure > 0.7 {
                candidates.push(CandidateAction::DrawFoul);
            }
        }

        PlayerMindset::DefendContain => {
            candidates.extend(&[
                CandidateAction::Jockey,
                CandidateAction::CoverSpace,
                CandidateAction::BlockLane,
                CandidateAction::DelayPass,
            ]);
        }

        PlayerMindset::DefendPress => {
            candidates.extend(&[
                CandidateAction::ClosingDown,
                CandidateAction::InterceptAttempt,
                CandidateAction::TrackRunner,
            ]);
            if ctx.near_touchline {
                candidates.push(CandidateAction::ForceTouchline);
            }
            // Contain 옵션도 일부
            candidates.push(CandidateAction::Jockey);
        }

        PlayerMindset::DefendChallenge => {
            candidates.extend(&[CandidateAction::StandingTackle, CandidateAction::PokeAway]);
            // 조건부 슬라이딩
            if !ctx.in_penalty_box && ctx.distance_to_ball_carrier < 2.0 {
                candidates.push(CandidateAction::SlidingTackle);
            }
            // 피지컬 대결
            candidates.push(CandidateAction::ShoulderCharge);
        }

        PlayerMindset::TransitionCounter => {
            // FIX_2601/0123: TransitionCounter must respect AttackPhase
            // Only allow forward passes if phase permits (Positional or Transition)
            candidates.push(CandidateAction::CounterAttackRun);
            if ctx.attack_phase.allows_forward_pass() {
                candidates.push(CandidateAction::ProgressivePass);
                candidates.push(CandidateAction::ThroughBall);
            }
            if ctx.xg > 0.1 {
                candidates.push(CandidateAction::ShootNormal);
            }
            candidates.push(CandidateAction::SafePass); // FIX_2601/0116: fallback
        }

        PlayerMindset::TransitionRecover => {
            candidates.extend(&[
                CandidateAction::RecoveryRun,
                CandidateAction::TrackRunner,
                CandidateAction::CoverSpace,
            ]);
        }

        PlayerMindset::GkDistribute | PlayerMindset::GkSave => {
            // GK 전용 액션 (추후 확장)
            candidates.push(CandidateAction::SafePass);
            candidates.push(CandidateAction::ClearBall);
        }
    }

    // 컨텍스트 기반 추가 필터
    apply_context_filters(&mut candidates, ctx);

    // 중복 제거
    candidates.sort_by_key(|c| *c as i32);
    candidates.dedup();

    candidates
}

/// 컨텍스트 기반 추가/제거 필터
fn apply_context_filters(candidates: &mut Vec<CandidateAction>, ctx: &DecisionContext) {
    // 압박 높으면 Protect 옵션 추가 (4m tactical influence)
    if ctx.local_pressure >= 0.7 && ctx.has_ball {
        if !candidates.contains(&CandidateAction::ShieldBall) {
            candidates.push(CandidateAction::ShieldBall);
        }
        if !candidates.contains(&CandidateAction::SafePass) {
            candidates.push(CandidateAction::SafePass);
        }
    }

    // FIX_2601/0123: Relaxed CarryBall conditions to increase dribbling (density/reciprocity ↓)
    // OLD: Required ALL THREE: dribble_skill_ok && dribble_safe_radius_ok && dribble_space_scan_ok
    // NEW: Allow CarryBall if ANY of: skill_ok, space_ahead, or safe_radius (more permissive)
    let dribble_allowed = ctx.dribble_skill_ok || ctx.has_space_ahead || ctx.dribble_safe_radius_ok;

    if ctx.has_ball && !dribble_allowed {
        candidates.retain(|c| !matches!(c, CandidateAction::CarryBall));
    }

    const TAKEON_ESCAPE_SKILL_THRESHOLD: f32 = 0.6;

    // Match OS v1.2 Priority 2: Immediate pressure filter (2m tackle threat)
    if ctx.immediate_pressure > 0.7 && ctx.has_ball {
        // Open-Football baseline: allow escape dribble if skill + space scan are sufficient
        let keep_takeon = ctx.dribble_escape_skill >= TAKEON_ESCAPE_SKILL_THRESHOLD
            && ctx.dribble_space_scan_ok;

        // Remove risky dribble options when tackle threat is imminent
        candidates.retain(|c| {
            if matches!(c, CandidateAction::TakeOn) {
                return keep_takeon;
            }
            !matches!(
                c,
                CandidateAction::CarryBall | CandidateAction::CounterAttackRun
            )
        });

        // Add protective actions if not already present
        if !candidates.contains(&CandidateAction::ShieldBall) {
            candidates.push(CandidateAction::ShieldBall);
        }
        if !candidates.contains(&CandidateAction::SafePass) {
            candidates.push(CandidateAction::SafePass);
        }
        if !candidates.contains(&CandidateAction::DrawFoul) {
            candidates.push(CandidateAction::DrawFoul);
        }
    }

    // 수비 상황에서 볼 소유 시
    if ctx.is_defending && ctx.has_ball && !candidates.contains(&CandidateAction::ClearBall) {
        candidates.push(CandidateAction::ClearBall);
    }

    // 후보가 비어있으면 Hold 추가
    if candidates.is_empty() {
        candidates.push(CandidateAction::Hold);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ShotGateOutcome {
    pub checked: bool,
    pub allowed: bool,
}

impl ShotGateOutcome {
    pub fn unchecked() -> Self {
        Self {
            checked: false,
            allowed: false,
        }
    }

    pub fn allowed() -> Self {
        Self {
            checked: true,
            allowed: true,
        }
    }

    pub fn rejected() -> Self {
        Self {
            checked: true,
            allowed: false,
        }
    }
}

/// FIX_2601/0106: 통합 슛 필터
///
/// 기존 3개 필터 통합: XG Zone + Better Teammate + Budget
/// - 중복 xG 체크 제거
/// - 명확한 우선순위 (Tier 시스템)
/// - 튜닝 용이성 향상
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    슛 허용 판정 흐름                         │
/// │                                                              │
/// │  Tier 1: xG >= 0.40  ──────────────────────→ 무조건 허용     │
/// │      │                                                       │
/// │      ▼                                                       │
/// │  Tier 2: xG >= 0.25 + (RED존 OR 동료없음) ──→ 허용           │
/// │      │                                                       │
/// │      ▼                                                       │
/// │  Tier 3: xG >= 0.15 + 압박낮음 + 예산내 + 동료없음 → 허용    │
/// │      │                                                       │
/// │      ▼                                                       │
/// │  Tier 4: 그 외 ────────────────────────────→ 슛 차단         │
/// └─────────────────────────────────────────────────────────────┘
/// ```
fn filter_shoot_unified(
    candidates: &mut Vec<CandidateAction>,
    ctx: &DecisionContext,
) -> ShotGateOutcome {
    use super::buildup_phase::BuildupPhase;
    use crate::engine::physics_constants::zones;
    use crate::engine::xgzone_map::{XG_THRESHOLD_RED, XG_THRESHOLD_YELLOW};

    let shoot_actions =
        [CandidateAction::ShootNormal, CandidateAction::ShootFinesse, CandidateAction::ShootPower];
    let has_shot_candidates = candidates
        .iter()
        .any(|c| shoot_actions.contains(c));
    if !has_shot_candidates {
        return ShotGateOutcome::unchecked();
    }

    let reject_shots = |candidates: &mut Vec<CandidateAction>| {
        candidates.retain(|c| !shoot_actions.contains(c));
        ShotGateOutcome::rejected()
    };

    // ========== DPER: Experimental xG Threshold Check ==========
    // If experimental config sets a higher threshold, respect it (unless < 5m from goal)
    // Default: 0.05, Conservative: 0.08+, Aggressive: 0.03-
    if ctx.distance_to_goal >= 5.0 && ctx.xg < ctx.exp_shoot_xg_threshold {
        return reject_shots(candidates);
    }

    // ========== Tier 0: 빌드업 페이즈 체크 (FIX_2601/0106 P3) ==========
    // FIX_2601/0114: 임계값 완화 - OwnThird 0.40→0.35, MiddleThird 0.35→0.28
    match ctx.buildup_phase {
        BuildupPhase::OwnThird => {
            // 자기 진영: xG >= 0.35 (완화: was 0.40)
            if ctx.xg < 0.35 {
                return reject_shots(candidates);
            }
        }
        BuildupPhase::MiddleThird => {
            // 중앙: xG >= 0.28 (완화: was 0.35)
            if ctx.xg < 0.28 {
                return reject_shots(candidates);
            }
        }
        BuildupPhase::FinalThird => {
            // 상대 진영: 기존 Tier 로직 사용
        }
    }

    // ========== Clear Shot Gate (Open-Football style) ==========
    let pressure_count = ctx.nearby_opponents_8m;
    // FIX_2601/0115: Relaxed low_pressure from ==0 to <=1 (more realistic)
    let low_pressure = pressure_count <= 1;
    let good_angle = ctx.good_shooting_angle;

    // 35m+ : 무조건 차단
    if ctx.distance_to_goal > zones::LONG_RANGE_M {
        return reject_shots(candidates);
    }

    // FIX_2601/0115: Relaxed pressure block from 2+ to 3+ within 8m
    // FIX_2601/0115b: Only block if also no clear shot (both conditions)
    // 3명+ 압박 AND 클리어 슛 없음 AND 8m+ 거리에서 슛 차단
    if pressure_count >= 3 && ctx.distance_to_goal > 8.0 && !ctx.has_clear_shot {
        return reject_shots(candidates);
    }

    // FIX_2601/0116: 슛 예산 체크 - 완전히 비활성화하고 xG 기반만 사용
    // 예산 시스템은 결정-실행 갭 때문에 제대로 작동 안 함
    // let within_budget = ctx.shot_budget_per_half == 0 || ctx.shots_this_half < ctx.shot_budget_per_half;

    // FIX_2601/0115b: Integrated Zone + Tier system
    // Zone 조건은 Tier 조건에 통합, has_clear_shot는 soft modifier로 사용
    // Actual xG: <10m=0.39, 10-16.5m=0.19, 16.5-25m=0.08, 25-35m=0.04

    // ========== Tier 1: 절대 허용 (xG >= 0.30) ==========
    // 페널티박스 내 (< 10m) 기회 - 무조건 허용
    if ctx.xg >= 0.30 {
        return ShotGateOutcome::allowed();
    }

    // ========== Tier 2: 좋은 기회 (xG >= 0.15) ==========
    // 박스 내 (10-16.5m) + 조건 완화
    if ctx.xg >= 0.15 && (ctx.has_clear_shot || low_pressure || ctx.local_pressure < 0.5) {
        return ShotGateOutcome::allowed();
    }

    // ========== Tier 3: 보통 기회 (xG >= 0.08) ==========
    // 박스 경계 (16.5-25m) + 좋은 조건 필요
    if ctx.xg >= 0.08 {
        // 16.5m 미만 (박스 내): 각도/압박 중 하나만 충족
        if ctx.distance_to_goal < zones::CLOSE_M {
            if good_angle || low_pressure {
                return ShotGateOutcome::allowed();
            }
        }
        // 16.5-25m: 클리어 슛 또는 (장거리 스킬 + 저압박)
        else if ctx.distance_to_goal < zones::MID_RANGE_M {
            if ctx.has_clear_shot || (ctx.long_shots_skill >= 14.0 && low_pressure) {
                return ShotGateOutcome::allowed();
            }
        }
    }

    // ========== Tier 4: 장거리 스페셜리스트 (xG >= 0.04) ==========
    // 25-35m 장거리: 높은 장거리 스킬 + 좋은 조건 필요
    if ctx.xg >= 0.04 && ctx.distance_to_goal < zones::LONG_RANGE_M {
        if ctx.long_shots_skill >= 16.0 && (ctx.has_clear_shot || low_pressure) {
            return ShotGateOutcome::allowed();
        }
    }

    // ========== Fallback: 박스 내 5m 미만은 항상 허용 ==========
    if ctx.distance_to_goal < 5.0 {
        return ShotGateOutcome::allowed();
    }

    // 조건 미충족 시 슛 거부
    reject_shots(candidates)
}

// ============================================================================
// Gate Chain Pipeline (Contract v1.0)
// ============================================================================

/// Gate Chain 결과
pub struct GateResult {
    pub candidate: CandidateAction,
    pub hard_passed: bool,
    pub budget_modifier: f32,
    pub soft_modifier: f32,
}

/// 1. Hard Gate: 절대 불가 조건 필터링
///
/// FIX_2601/0106: 통합 슛 필터 사용
/// - 기존 3개 필터 (xgzone, teammate, budget) → filter_shoot_unified 하나로 통합
/// - Tier 기반 우선순위로 명확한 판정
pub fn apply_hard_gates(
    mindset: PlayerMindset,
    ctx: &DecisionContext,
) -> (Vec<CandidateAction>, ShotGateOutcome) {
    let mut candidates = filter_candidates_by_mindset(mindset, ctx);

    // FIX_2601/0106: 통합 슛 필터 (xG zone + teammate + budget 통합)
    let shot_gate = filter_shoot_unified(&mut candidates, ctx);

    (candidates, shot_gate)
}

/// 2. Budget Gate: 빈도/예산 기반 보정 (No RNG)
pub fn apply_budget_gates(
    candidates: &[CandidateAction],
    ctx: &DecisionContext,
) -> Vec<(CandidateAction, WeightBreakdown)> {
    candidates
        .iter()
        .map(|&c| {
            let mut bd = WeightBreakdown::neutral();

            if let Some(history) = &ctx.action_history {
                match c {
                    CandidateAction::ShootNormal
                    | CandidateAction::ShootPower
                    | CandidateAction::ShootFinesse => {
                        // 슛 연타 방지
                        if history.recent_shots > 0 {
                            let m = 0.1 / (history.recent_shots as f32);
                            bd.context *= m.clamp(0.05, 1.0);
                        }
                    }
                    CandidateAction::StandingTackle | CandidateAction::SlidingTackle => {
                        // 태클 연타 방지
                        if history.recent_tackles > 1 {
                            let m = 0.5; // Fixed penalty for now
                            bd.context *= m;
                        }
                    }
                    _ => {}
                }

                // 스태미나 기반 보정
                if history.stamina < 0.3 && c.risk_level() > 0.6 {
                    bd.context *= 0.7; // 고위험 액션 기피
                }
            }

            (c, bd)
        })
        .collect()
}

// =======================================================================
// Event Mix Bias (zone-based, team-view)
// =======================================================================

const EVENT_MIX_STRENGTH: f32 = 0.35;
const EVENT_MIX_CLAMP_MIN: f32 = 0.6;
const EVENT_MIX_CLAMP_MAX: f32 = 1.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventMixZone {
    DefBox,
    DefMid,
    Mid,
    OffMid,
    OffWide,
    OffBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventMixBucket {
    Pass,
    Cross,
    Dribble,
    Shot,
}

#[derive(Debug, Clone, Copy)]
struct EventMixWeights {
    pass: f32,
    cross: f32,
    dribble: f32,
    shot: f32,
}

impl EventMixWeights {
    fn mean(self) -> f32 {
        (self.pass + self.cross + self.dribble + self.shot) / 4.0
    }

    fn normalize_by_mean(self) -> Self {
        let denom = self.mean().max(0.0001);
        Self {
            pass: self.pass / denom,
            cross: self.cross / denom,
            dribble: self.dribble / denom,
            shot: self.shot / denom,
        }
    }

    fn mul(self, other: Self) -> Self {
        Self {
            pass: self.pass * other.pass,
            cross: self.cross * other.cross,
            dribble: self.dribble * other.dribble,
            shot: self.shot * other.shot,
        }
    }

    fn for_bucket(self, bucket: EventMixBucket) -> f32 {
        match bucket {
            EventMixBucket::Pass => self.pass,
            EventMixBucket::Cross => self.cross,
            EventMixBucket::Dribble => self.dribble,
            EventMixBucket::Shot => self.shot,
        }
    }
}

fn event_mix_zone_from_pitch_zone(zone: PitchZone) -> EventMixZone {
    use crate::calibration::zone::{Lane, Quarter};

    let quarter = zone.quarter();
    let lane = zone.lane();

    match quarter {
        Quarter::Defensive => {
            // CDef maps to DefBox (like old penalty area), others to DefMid
            if lane.is_central() {
                EventMixZone::DefBox
            } else {
                EventMixZone::DefMid
            }
        }
        Quarter::Middle => EventMixZone::Mid,
        Quarter::Final => EventMixZone::OffMid,
        Quarter::Box => {
            // Wide box zones are OffWide (crossing areas), others are OffBox
            if lane.is_wide() {
                EventMixZone::OffWide
            } else {
                EventMixZone::OffBox
            }
        }
    }
}

fn event_mix_base_weights(zone: EventMixZone) -> EventMixWeights {
    match zone {
        EventMixZone::DefBox => {
            EventMixWeights { pass: 0.82, cross: 0.03, dribble: 0.12, shot: 0.03 }
        }
        EventMixZone::DefMid => {
            EventMixWeights { pass: 0.75, cross: 0.05, dribble: 0.15, shot: 0.05 }
        }
        EventMixZone::Mid => EventMixWeights { pass: 0.60, cross: 0.10, dribble: 0.20, shot: 0.10 },
        EventMixZone::OffMid => {
            EventMixWeights { pass: 0.45, cross: 0.15, dribble: 0.20, shot: 0.20 }
        }
        EventMixZone::OffWide => {
            EventMixWeights { pass: 0.30, cross: 0.35, dribble: 0.15, shot: 0.20 }
        }
        EventMixZone::OffBox => {
            EventMixWeights { pass: 0.20, cross: 0.10, dribble: 0.15, shot: 0.55 }
        }
    }
}

fn event_mix_profile_weights(profile: EventMixProfile) -> EventMixWeights {
    match profile {
        EventMixProfile::Balanced => {
            EventMixWeights { pass: 1.00, cross: 1.00, dribble: 1.00, shot: 1.00 }
        }
        EventMixProfile::Possession => {
            EventMixWeights { pass: 1.25, cross: 0.85, dribble: 1.05, shot: 0.75 }
        }
        EventMixProfile::Counterattack => {
            EventMixWeights { pass: 0.95, cross: 1.05, dribble: 1.10, shot: 1.25 }
        }
        EventMixProfile::Defensive => {
            EventMixWeights { pass: 1.10, cross: 0.80, dribble: 0.90, shot: 0.65 }
        }
    }
}

fn zone_transition_style_for_profile(
    profile: EventMixProfile,
) -> zone_transition::ZoneTransitionStyle {
    match profile {
        EventMixProfile::Balanced => zone_transition::ZoneTransitionStyle::Normal,
        EventMixProfile::Possession => zone_transition::ZoneTransitionStyle::Possession,
        EventMixProfile::Counterattack => zone_transition::ZoneTransitionStyle::Counter,
        EventMixProfile::Defensive => zone_transition::ZoneTransitionStyle::Possession,
    }
}

fn ofm_event_mix_weights(profile: EventMixProfile, zone: PitchZone) -> EventMixWeights {
    let style = zone_transition_style_for_profile(profile);
    let weights = zone_transition::ofm_event_mix_weights(style, zone);
    EventMixWeights {
        pass: weights.pass,
        cross: weights.cross,
        dribble: weights.dribble,
        shot: weights.shot,
    }
}

fn event_mix_bucket(action: CandidateAction, ctx: &DecisionContext) -> Option<EventMixBucket> {
    match action {
        CandidateAction::SafePass
        | CandidateAction::ProgressivePass
        | CandidateAction::ThroughBall => Some(EventMixBucket::Pass),
        CandidateAction::SwitchPlay => {
            if ctx.near_touchline {
                Some(EventMixBucket::Cross)
            } else {
                Some(EventMixBucket::Pass)
            }
        }
        CandidateAction::CarryBall | CandidateAction::TakeOn | CandidateAction::OneTwo => {
            Some(EventMixBucket::Dribble)
        }
        CandidateAction::ShootNormal
        | CandidateAction::ShootFinesse
        | CandidateAction::ShootPower
        | CandidateAction::ShootChip
        | CandidateAction::Header => Some(EventMixBucket::Shot),
        // FIX_2601/0117: Cross uses Cross bucket
        CandidateAction::Cross => Some(EventMixBucket::Cross),
        _ => None,
    }
}

fn event_mix_factor_for_action(action: CandidateAction, ctx: &DecisionContext) -> Option<f32> {
    if !ctx.has_ball || ctx.is_defending {
        return None;
    }

    let bucket = event_mix_bucket(action, ctx)?;
    if matches!(bucket, EventMixBucket::Pass | EventMixBucket::Cross) && ctx.pass_options_count == 0
    {
        return None;
    }

    let zone = ctx.team_pitch_zone?;
    let profile = ctx.event_mix_profile.unwrap_or(EventMixProfile::Balanced);
    let base = event_mix_base_weights(event_mix_zone_from_pitch_zone(zone));
    let tuned =
        base.mul(event_mix_profile_weights(profile)).mul(ofm_event_mix_weights(profile, zone));
    let normalized = tuned.normalize_by_mean();
    let weight = normalized.for_bucket(bucket).max(0.0001);
    let factor = weight.powf(EVENT_MIX_STRENGTH).clamp(EVENT_MIX_CLAMP_MIN, EVENT_MIX_CLAMP_MAX);
    Some(factor)
}

/// 3. Soft Gate: 전술/상황별 선호도 보정 (No RNG)
pub fn apply_soft_gates(
    candidates: &[(CandidateAction, WeightBreakdown)],
    ctx: &DecisionContext,
) -> Vec<(CandidateAction, WeightBreakdown)> {
    candidates
        .iter()
        .map(|&(c, prev_bd)| {
            let mut bd = prev_bd;
            const STICKY_DRIBBLE_MULT: f32 = 1.5;
            const STICKY_PRESS_MULT: f32 = 1.4;

            // 상황별 선호도 (예: 압박 높을 때 안전 패스 선호)
            if ctx.local_pressure > 0.8 {
                if c == CandidateAction::SafePass || c == CandidateAction::ShieldBall {
                    bd.context *= 1.5;
                } else if c.is_selfish() {
                    bd.context *= 0.6;
                }
            }

            // 터치라인 근처에서 크로스/사이드 플레이 선호 (추후 확장)
            if ctx.near_touchline && c == CandidateAction::SwitchPlay {
                bd.context *= 1.3;
            }

            // FIX_2601/0117: SwitchPlay 추가 부스트 (옵션 많으면 Switch 유리)
            if c == CandidateAction::SwitchPlay && ctx.pass_options_count >= 3 {
                bd.context *= 1.15;
            }

            // ----------------------------------------------------------------
            // Phase G v1: Team tactics → decision distribution (directionality)
            // ----------------------------------------------------------------
            // Build-up risk bias (Short/Mixed/Direct) should affect pass selection
            // without changing the meaning layer (UI/Replay).
            if ctx.has_ball && !ctx.is_defending {
                // Normalize: -0.15..+0.20 → roughly -1..+1
                let risk_norm = (ctx.team_risk_bias / 0.20).clamp(-1.0, 1.0);
                if risk_norm.abs() > 1e-6 {
                    match c {
                        // Conservative build-up prefers safe/hold actions
                        CandidateAction::SafePass => {
                            bd.tactics *= (1.0 - 0.20 * risk_norm).clamp(0.8, 1.2);
                        }
                        CandidateAction::ShieldBall | CandidateAction::HoldUpPlay => {
                            bd.tactics *= (1.0 - 0.10 * risk_norm).clamp(0.85, 1.15);
                        }
                        // Direct build-up prefers forward/risky progress options
                        CandidateAction::ProgressivePass => {
                            bd.tactics *= (1.0 + 0.15 * risk_norm).clamp(0.8, 1.2);
                        }
                        CandidateAction::ThroughBall => {
                            bd.tactics *= (1.0 + 0.20 * risk_norm).clamp(0.75, 1.25);
                        }
                        CandidateAction::SwitchPlay | CandidateAction::Cross => {
                            bd.tactics *= (1.0 + 0.10 * risk_norm).clamp(0.8, 1.2);
                        }
                        CandidateAction::ShootNormal
                        | CandidateAction::ShootFinesse
                        | CandidateAction::ShootPower
                        | CandidateAction::ShootChip
                        | CandidateAction::Header => {
                            bd.tactics *= (1.0 + 0.08 * risk_norm).clamp(0.85, 1.15);
                        }
                        _ => {}
                    }
                }

                // Width bias: wide teams should prefer Cross/SwitchPlay more often.
                let width_norm = (ctx.team_width_bias_m / 5.0).clamp(-1.0, 1.0);
                if width_norm.abs() > 1e-6 && matches!(c, CandidateAction::SwitchPlay | CandidateAction::Cross) {
                    // Stronger effect when already in wide context.
                    let zone_mul = if ctx.near_touchline || ctx.is_wide_position { 1.0 } else { 0.5 };
                    bd.tactics *= (1.0 + 0.15 * width_norm * zone_mul).clamp(0.8, 1.2);
                }
            }

            if ctx.sticky_actions.dribble
                && ctx.has_ball
                && !ctx.is_defending
                && matches!(
                    c,
                    CandidateAction::CarryBall | CandidateAction::TakeOn | CandidateAction::OneTwo
                )
            {
                bd.context *= STICKY_DRIBBLE_MULT;
            }

            if ctx.sticky_actions.press
                && ctx.is_defending
                && matches!(
                    c,
                    CandidateAction::ClosingDown
                        | CandidateAction::InterceptAttempt
                        | CandidateAction::ForceTouchline
                        | CandidateAction::TrackRunner
                        | CandidateAction::StandingTackle
                        | CandidateAction::SlidingTackle
                )
            {
                bd.context *= STICKY_PRESS_MULT;
            }

            if let Some(factor) = event_mix_factor_for_action(c, ctx) {
                bd.tactics *= factor;
            }

            (c, bd)
        })
        .collect()
}

// ============================================================================
// Gate B: Utility Selection (Softmax)
// ============================================================================

// ============================================================================
// FIX_2601/0113: UAE-GateB Bridge Functions
// ============================================================================

/// CandidateAction(32 variants) → UAE Action enum 변환
/// FIX_2601/0113 Phase 2: 모든 CandidateAction 완전 매핑 (fallback 없음)
fn candidate_to_uae_action(c: CandidateAction) -> Action {
    match c {
        // === Attack: Score ===
        CandidateAction::ShootNormal
        | CandidateAction::ShootFinesse
        | CandidateAction::ShootPower
        | CandidateAction::ShootChip => Action::Shoot,
        CandidateAction::Header => Action::Header { is_shot: true },
        CandidateAction::Cross => Action::Cross {
            target_zone: CrossZone::FarPost,
        },

        // === Attack: Progress ===
        CandidateAction::SafePass
        | CandidateAction::ProgressivePass
        | CandidateAction::SwitchPlay => Action::Pass {
            target_id: PlayerId::new(0),
        },
        CandidateAction::ThroughBall => Action::ThroughBall {
            target_id: PlayerId::new(0),
        },
        CandidateAction::CarryBall => Action::Carry {
            direction: Vec2::default(),
        },

        // === Attack: Beat ===
        CandidateAction::TakeOn | CandidateAction::OneTwo => Action::Dribble {
            direction: Vec2::default(),
        },

        // === Attack: Protect (NEW - Phase 2) ===
        CandidateAction::ShieldBall | CandidateAction::HoldUpPlay => Action::Hold,
        CandidateAction::DrawFoul => Action::DrawFoul,

        // === Defense: Contain ===
        CandidateAction::Jockey | CandidateAction::DelayPass => Action::Jockey,
        CandidateAction::CoverSpace | CandidateAction::BlockLane => Action::Cover {
            zone: Zone { x: 5, y: 3 },
        },

        // === Defense: Press ===
        CandidateAction::ClosingDown | CandidateAction::ForceTouchline => Action::Press,
        CandidateAction::InterceptAttempt => Action::Intercept {
            lane: PassLane {
                from: (0.0, 0.0),
                to: (1.0, 0.0),
            },
        },
        CandidateAction::TrackRunner => Action::Mark {
            target_id: PlayerId::new(0),
        },

        // === Defense: Challenge ===
        CandidateAction::StandingTackle
        | CandidateAction::SlidingTackle
        | CandidateAction::ShoulderCharge
        | CandidateAction::PokeAway => Action::Tackle,

        // === Transition ===
        CandidateAction::CounterAttackRun => Action::RunIntoSpace {
            target: Position::default(),
        },
        CandidateAction::RecoveryRun => Action::RecoveryRun {
            target: Position::default(),
        },

        // === Special ===
        CandidateAction::ClearBall => Action::Clear,
        CandidateAction::Hold => Action::Hold,
    }
}

/// DecisionContext → EvalContext 변환
/// Uses available fields from DecisionContext to populate EvalContext
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EvalCtxFillMask(u32);

impl EvalCtxFillMask {
    pub const DIST_TO_GOAL: u32 = 1 << 0;
    pub const DIST_TO_BALL: u32 = 1 << 1;
    pub const DIST_TO_BALL_CARRIER: u32 = 1 << 2;
    pub const XG: u32 = 1 << 3;
    pub const LOCAL_PRESSURE: u32 = 1 << 4;
    pub const IN_SHOOTING_ZONE: u32 = 1 << 5;
    pub const SHOT_LANE_CLEAR: u32 = 1 << 6;
    pub const DRIBBLE_SUCCESS_PROBABILITY: u32 = 1 << 7;
    pub const CLOSEST_DEFENDER_DIST: u32 = 1 << 8;
    pub const SPACE_AHEAD: u32 = 1 << 9;
    pub const PASS_INTERCEPTOR_COUNT: u32 = 1 << 10;
    pub const LONG_SHOTS: u32 = 1 << 11;

    pub const FINISHING: u32 = 1 << 12;
    pub const COMPOSURE: u32 = 1 << 13;
    pub const TECHNIQUE: u32 = 1 << 14;
    pub const PASSING: u32 = 1 << 15;
    pub const VISION: u32 = 1 << 16;
    pub const DRIBBLING: u32 = 1 << 17;
    pub const TACKLING: u32 = 1 << 18;

    // Tier-1 wiring (v1.2): fields that must not silently fall back to Default
    pub const STAMINA_PCT: u32 = 1 << 19;
    pub const RECEIVER_DIST: u32 = 1 << 20;
    pub const RECEIVER_FREEDOM: u32 = 1 << 21;
    pub const RECEIVER_HAS_SPACE: u32 = 1 << 22;
    pub const PASS_LANE_CLEAR: u32 = 1 << 23;
    pub const NEARBY_OPPONENTS: u32 = 1 << 24;

    #[inline]
    pub fn has(self, bit: u32) -> bool {
        (self.0 & bit) != 0
    }

    #[inline]
    pub fn bits(self) -> u32 {
        self.0
    }
}

#[inline(always)]
fn eval_ctx_mark<const WITH_MASK: bool>(mask: &mut EvalCtxFillMask, bit: u32) {
    if WITH_MASK {
        mask.0 |= bit;
    }
}

#[inline]
fn build_eval_ctx_from_decision_impl<const WITH_MASK: bool>(
    ctx: &DecisionContext,
    mask: &mut EvalCtxFillMask,
) -> EvalContext {
    let mut eval = EvalContext::default();

    // Position/State
    eval.dist_to_goal = ctx.distance_to_goal;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::DIST_TO_GOAL);
    eval.dist_to_ball = ctx.distance_to_ball;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::DIST_TO_BALL);
    eval.dist_to_ball_carrier = ctx.distance_to_ball_carrier;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::DIST_TO_BALL_CARRIER);    
    eval.stamina_pct = ctx
        .defense_ctx
        .as_ref()
        .map(|d| d.stamina_percent)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::STAMINA_PCT);

    // Situational (from DecisionContext)
    eval.xg = ctx.xg;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::XG);
    eval.local_pressure = ctx.local_pressure;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::LOCAL_PRESSURE);
    eval.in_shooting_zone = ctx.distance_to_goal < 25.0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::IN_SHOOTING_ZONE);        
    eval.shot_lane_clear = ctx.has_clear_shot;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::SHOT_LANE_CLEAR);

    // Dribble-related
    eval.dribble_success_probability = if ctx.dribble_skill_ok { 0.65 } else { 0.4 };
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::DRIBBLE_SUCCESS_PROBABILITY);
    eval.closest_defender_dist = ctx.nearest_opponent_dist;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::CLOSEST_DEFENDER_DIST);   
    eval.space_ahead = if ctx.has_space_ahead { 0.7 } else { 0.3 };
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::SPACE_AHEAD);

    // Pass-related
    // v1.2: Use nearest teammate distance as a stable "typical pass distance"
    // proxy, so PassEvaluator doesn't always see Default(0.0).
    eval.receiver_dist = ctx.nearest_teammate_dist.max(0.0);
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::RECEIVER_DIST);
    eval.receiver_freedom = (ctx.pass_options_count as f32 / 6.0).clamp(0.0, 1.0);
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::RECEIVER_FREEDOM);
    eval.receiver_has_space = if ctx.has_space_ahead { 0.6 } else { 0.3 };
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::RECEIVER_HAS_SPACE);
    eval.pass_lane_clear = ctx.pass_options_count > 0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::PASS_LANE_CLEAR);
    eval.pass_interceptor_count = if ctx.local_pressure > 0.5 { 2 } else { 0 }; 
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::PASS_INTERCEPTOR_COUNT);  

    // Use long_shots_skill if available (assumed 0-100 scale)
    eval.long_shots = ctx.long_shots_skill;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::LONG_SHOTS);

    // Hold/Clear-related
    eval.nearby_opponents = ctx.nearby_opponents_8m as u32;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::NEARBY_OPPONENTS);

    // Defaults for attributes not in DecisionContext
    // These use reasonable midpoint values
    eval.finishing = 50.0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::FINISHING);
    eval.composure = 50.0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::COMPOSURE);
    eval.technique = 50.0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::TECHNIQUE);
    eval.passing = 50.0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::PASSING);
    eval.vision = 50.0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::VISION);
    eval.dribbling = if ctx.dribble_skill_ok { 70.0 } else { 50.0 };
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::DRIBBLING);
    eval.tackling = 50.0;
    eval_ctx_mark::<WITH_MASK>(mask, EvalCtxFillMask::TACKLING);

    eval
}
fn build_eval_ctx_from_decision(ctx: &DecisionContext) -> EvalContext {
    let mut mask = EvalCtxFillMask::default();
    build_eval_ctx_from_decision_impl::<false>(ctx, &mut mask)
}

/// CI Gate helper: expose EvalContext fill coverage without mutating SSOT.
#[doc(hidden)]
pub fn ci_gate_build_eval_ctx_with_mask(ctx: &DecisionContext) -> (EvalContext, EvalCtxFillMask) {
    let mut mask = EvalCtxFillMask::default();
    let eval = build_eval_ctx_from_decision_impl::<true>(ctx, &mut mask);
    (eval, mask)
}

/// UAE ActionScore6 → CandidateFacts 변환
/// Converts 6-factor score to Gate B expected format with calibration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactsCategory {
    Attack,
    Defend,
    OffBall,
}

fn candidate_facts_category(candidate: CandidateAction) -> FactsCategory {
    match candidate {
        // Attack / on-ball choices
        CandidateAction::ShootNormal
        | CandidateAction::ShootFinesse
        | CandidateAction::ShootPower
        | CandidateAction::ShootChip
        | CandidateAction::Header
        | CandidateAction::Cross
        | CandidateAction::SafePass
        | CandidateAction::ProgressivePass
        | CandidateAction::SwitchPlay
        | CandidateAction::ThroughBall
        | CandidateAction::CarryBall
        | CandidateAction::TakeOn
        | CandidateAction::OneTwo
        | CandidateAction::ShieldBall
        | CandidateAction::HoldUpPlay
        | CandidateAction::DrawFoul
        | CandidateAction::Hold => FactsCategory::Attack,

        // Defense / out-of-possession actions
        CandidateAction::Jockey
        | CandidateAction::DelayPass
        | CandidateAction::CoverSpace
        | CandidateAction::BlockLane
        | CandidateAction::ClosingDown
        | CandidateAction::ForceTouchline
        | CandidateAction::InterceptAttempt
        | CandidateAction::TrackRunner
        | CandidateAction::StandingTackle
        | CandidateAction::SlidingTackle
        | CandidateAction::ShoulderCharge
        | CandidateAction::PokeAway
        | CandidateAction::ClearBall => FactsCategory::Defend,

        // Off-ball / transition runs
        CandidateAction::CounterAttackRun | CandidateAction::RecoveryRun => FactsCategory::OffBall,
    }
}

/// CI Gate helper: expose the candidate→facts category mapping for snapshot/tests.
#[doc(hidden)]
pub fn ci_gate_candidate_facts_category(candidate: CandidateAction) -> &'static str {
    match candidate_facts_category(candidate) {
        FactsCategory::Attack => "Attack",
        FactsCategory::Defend => "Defend",
        FactsCategory::OffBall => "OffBall",
    }
}

fn score6_to_candidate_facts(
    score: &ActionScore,
    ctx: &DecisionContext,
    candidate: CandidateAction,
) -> CandidateFacts {
    let category = candidate_facts_category(candidate);

    let (p_true, v_win_base, v_lose) = match category {
        FactsCategory::Attack => {
            // Attack: prefer on-ball execution quality and turnover risk.
            // p_true = readiness(55%) + safety(45%)
            let p_true = (score.readiness * 0.55 + score.safety * 0.45).clamp(0.0, 1.0);

            // v_win = progression(55%) + tactical(25%) + space(20%)
            let v_win_base =
                (score.progression * 0.55 + score.tactical * 0.25 + score.space * 0.20)
                    .clamp(0.0, 1.0);

            // v_lose = (1-safety)(70%) + (1-space)(30%)
            let v_lose =
                ((1.0 - score.safety) * 0.70 + (1.0 - score.space) * 0.30).clamp(0.0, 1.0);

            (p_true, v_win_base, v_lose)
        }
        FactsCategory::Defend => {
            // Defend: feasibility(distance) and stakes(tactical) matter more.
            // p_true = readiness(45%) + distance(35%) + safety(20%)
            let p_true =
                (score.readiness * 0.45 + score.distance * 0.35 + score.safety * 0.20)
                    .clamp(0.0, 1.0);

            // v_win = tactical(40%) + progression(25%) + safety(20%) + space(15%)
            let v_win_base =
                (score.tactical * 0.40 + score.progression * 0.25 + score.safety * 0.20
                    + score.space * 0.15)
                    .clamp(0.0, 1.0);

            // v_lose: "miss" on high-stakes defensive action is costly.
            // v_lose = (1-safety)(35%) + tactical(40%) + (1-space)(25%)
            let v_lose =
                ((1.0 - score.safety) * 0.35 + score.tactical * 0.40 + (1.0 - score.space) * 0.25)
                    .clamp(0.0, 1.0);

            (p_true, v_win_base, v_lose)
        }
        FactsCategory::OffBall => {
            // OffBall: distance(feasibility) and space/progression dominate; miss costs are lower.
            // p_true = readiness(40%) + distance(35%) + safety(25%)
            let p_true =
                (score.readiness * 0.40 + score.distance * 0.35 + score.safety * 0.25)
                    .clamp(0.0, 1.0);

            // v_win = progression(35%) + space(35%) + tactical(30%)
            let v_win_base =
                (score.progression * 0.35 + score.space * 0.35 + score.tactical * 0.30)
                    .clamp(0.0, 1.0);

            // v_lose: scaled down (wasted time/effort, not immediate turnover).
            let v_lose_base =
                ((1.0 - score.safety) * 0.50 + (1.0 - score.space) * 0.50).clamp(0.0, 1.0);
            let v_lose = (v_lose_base * 0.60).clamp(0.0, 1.0);

            (p_true, v_win_base, v_lose)
        }
    };

    // Apply calibration factors from DecisionContext
    let cal_bias = match candidate {
        CandidateAction::ShootNormal
        | CandidateAction::ShootFinesse
        | CandidateAction::ShootPower
        | CandidateAction::ShootChip => ctx.cal_shot_bias,
        CandidateAction::ProgressivePass => ctx.cal_progressive_pass_bias,
        CandidateAction::ThroughBall => ctx.cal_through_ball_bias,
        CandidateAction::Cross => ctx.cal_cross_bias,
        CandidateAction::TakeOn => ctx.cal_dribble_bias,
        CandidateAction::SwitchPlay => ctx.cal_long_pass_bias,
        // FIX_2601/1128: SafePass bias - boosted in Circulation mode
        CandidateAction::SafePass => ctx.cal_safe_pass_bias,
        _ => 1.0,
    };

    // FIX_2601/1129: Apply AttackPhase weight multipliers
    // ProgressivePass, ThroughBall, Cross = forward progression
    // SafePass, SwitchPlay = circulation/lateral
    let phase_multiplier = match candidate {
        CandidateAction::ProgressivePass
        | CandidateAction::ThroughBall
        | CandidateAction::Cross => ctx.attack_phase.progression_weight_multiplier(),
        CandidateAction::SafePass
        | CandidateAction::SwitchPlay
        | CandidateAction::ShieldBall
        | CandidateAction::HoldUpPlay => ctx.attack_phase.circulation_weight_multiplier(),
        _ => 1.0,
    };

    let v_win = v_win_base * cal_bias * phase_multiplier;

    CandidateFacts::new(
        p_true,
        v_win.clamp(0.0, 1.0),
        v_lose,
        ctx.local_pressure,
        candidate.is_selfish(),
        candidate.is_pass_like(),
    )
}

/// CI Gate helper: expose the SSOT score6→facts mapping for snapshot tests.
#[doc(hidden)]
pub fn ci_gate_score6_to_candidate_facts(
    score: ActionScore,
    ctx: &DecisionContext,
    candidate: CandidateAction,
) -> CandidateFacts {
    score6_to_candidate_facts(&score, ctx, candidate)
}

/// 후보 액션의 CandidateFacts 생성
/// FIX_2601/0113 Phase 2: UAE-only (fallback 제거, 32개 완전 매핑)
pub fn build_candidate_facts(candidate: CandidateAction, ctx: &DecisionContext) -> CandidateFacts {
    // UAE 파이프라인 (항상 사용, fallback 없음)
    let uae_action = candidate_to_uae_action(candidate);
    let eval_ctx = build_eval_ctx_from_decision(ctx);
    let score = EvaluatorRegistry::evaluate(&uae_action, &eval_ctx);
    score6_to_candidate_facts(&score, ctx, candidate)
}

/// Gate B: Utility 기반 최선의 Intent 선택
pub fn select_best_intent(
    candidates: &[(CandidateAction, WeightBreakdown)],
    ctx: &DecisionContext,
    bias: &CognitiveBias,
    temperature: f32,
    rng: &mut impl Rng,
) -> (SelectedIntent, f32, Vec<(CandidateAction, UtilityResult)>) {
    if candidates.is_empty() {
        return (SelectedIntent::Wait, 0.0, vec![]);
    }

    // 각 후보의 Utility 계산
    let mut results: Vec<(CandidateAction, UtilityResult)> = Vec::with_capacity(candidates.len());
    let mut scores: Vec<f32> = Vec::with_capacity(candidates.len());

    // FIX_2601/1128: Debug utility comparison
    static UTIL_DEBUG_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    for &(candidate, bd) in candidates {
        let facts = build_candidate_facts(candidate, ctx);
        let mut result = calculate_utility_result(facts, bias, rng);

        // Gate Modifier 적용 (Weight Space에서 적용)
        // Contract v1.0: ln(W_total) = ln(W_base) + Σ ln(factor_i)
        // 여기서는 utility가 이미 로그성(EU)을 띄므로 가산하거나,
        // weight 선택 시점에 적용해야 함.

        // Add ln(weight) to utility/score
        let score = result.utility + ln_weight(&bd);

        // FIX_2601/1128: Debug - track bias distribution
        static ATTACK_PHASE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        static TRANS_ATK_PHASE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        static OTHER_PHASE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        if matches!(candidate, CandidateAction::SafePass) {
            if ctx.cal_safe_pass_bias > 1.5 {
                ATTACK_PHASE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            } else if ctx.cal_progressive_pass_bias > 1.2 {
                TRANS_ATK_PHASE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            } else {
                OTHER_PHASE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }

            let total = ATTACK_PHASE_COUNT.load(std::sync::atomic::Ordering::Relaxed)
                + TRANS_ATK_PHASE_COUNT.load(std::sync::atomic::Ordering::Relaxed)
                + OTHER_PHASE_COUNT.load(std::sync::atomic::Ordering::Relaxed);
            if total > 0 && total % 500 == 0 {
                let atk = ATTACK_PHASE_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                let trans = TRANS_ATK_PHASE_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                let other = OTHER_PHASE_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                eprintln!(
                    "[PHASE_DIST] Total: {} | Attack/Circ: {} ({:.1}%) | TransAtk: {} ({:.1}%) | Other: {} ({:.1}%)",
                    total, atk, 100.0 * atk as f64 / total as f64,
                    trans, 100.0 * trans as f64 / total as f64,
                    other, 100.0 * other as f64 / total as f64
                );
            }
        }

        // For debugging/results, we can also modify result.utility if desired,
        // but let's keep result.utility as the "biased utility" and use `score` for selection.
        // However, the caller expects `utility` to be the selected value.
        // Let's bake it in for now.
        result.utility = score;

        results.push((candidate, result));
        scores.push(score);
    }

    // Softmax 선택
    let selected_idx = softmax_select(&scores, temperature, rng);

    let (selected_action, selected_result) = &results[selected_idx];
    let intent = candidate_to_intent(*selected_action);

    // FIX_2601/1128: Debug - track ALL pass-like action selections
    static SAFE_PASS_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static PROG_PASS_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static SWITCH_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static THROUGH_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static ONE_TWO_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    match selected_action {
        CandidateAction::SafePass => {
            SAFE_PASS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        CandidateAction::ProgressivePass => {
            PROG_PASS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        CandidateAction::SwitchPlay => {
            SWITCH_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        CandidateAction::ThroughBall => {
            THROUGH_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        CandidateAction::OneTwo => {
            ONE_TWO_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        _ => {}
    }
    let total = SAFE_PASS_COUNT.load(std::sync::atomic::Ordering::Relaxed)
        + PROG_PASS_COUNT.load(std::sync::atomic::Ordering::Relaxed)
        + SWITCH_COUNT.load(std::sync::atomic::Ordering::Relaxed)
        + THROUGH_COUNT.load(std::sync::atomic::Ordering::Relaxed)
        + ONE_TWO_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    if total > 0 && total % 500 == 0 {
        let safe_count = SAFE_PASS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let prog_count = PROG_PASS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let switch_count = SWITCH_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let through_count = THROUGH_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let onetwo_count = ONE_TWO_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        eprintln!(
            "[PASS_SELECT] Safe:{} ({:.0}%) Prog:{} ({:.0}%) Switch:{} Through:{} OneTwo:{}",
            safe_count, 100.0 * safe_count as f64 / total as f64,
            prog_count, 100.0 * prog_count as f64 / total as f64,
            switch_count, through_count, onetwo_count
        );
    }

    (intent, selected_result.utility, results)
}

/// CandidateAction → SelectedIntent 변환
pub fn candidate_to_intent(candidate: CandidateAction) -> SelectedIntent {
    match candidate {
        // Protect
        CandidateAction::ShieldBall => SelectedIntent::Protect(ProtectStyle::Shield),
        CandidateAction::HoldUpPlay => SelectedIntent::Protect(ProtectStyle::HoldUp),
        CandidateAction::DrawFoul => SelectedIntent::Protect(ProtectStyle::DrawFoul),

        // Progress
        CandidateAction::SafePass => SelectedIntent::Progress(ProgressStyle::Safe),
        CandidateAction::ProgressivePass => SelectedIntent::Progress(ProgressStyle::Progressive),
        CandidateAction::SwitchPlay => SelectedIntent::Progress(ProgressStyle::Switch),
        CandidateAction::CarryBall => SelectedIntent::Progress(ProgressStyle::Carry),

        // Beat
        CandidateAction::TakeOn => SelectedIntent::Beat(BeatStyle::TakeOn),
        CandidateAction::OneTwo => SelectedIntent::Beat(BeatStyle::OneTwo),
        CandidateAction::ThroughBall => SelectedIntent::Beat(BeatStyle::Through),

        // Score
        CandidateAction::ShootNormal => SelectedIntent::Score(ScoreStyle::Normal),
        CandidateAction::ShootFinesse => SelectedIntent::Score(ScoreStyle::Finesse),
        CandidateAction::ShootPower => SelectedIntent::Score(ScoreStyle::Power),
        CandidateAction::ShootChip => SelectedIntent::Score(ScoreStyle::Chip),
        CandidateAction::Header => SelectedIntent::Score(ScoreStyle::Header),
        // FIX_2601/0117: Cross maps to Score(Cross)
        CandidateAction::Cross => SelectedIntent::Score(ScoreStyle::Cross),

        // Contain
        CandidateAction::Jockey => SelectedIntent::Contain(ContainTechnique::Jockey),
        CandidateAction::DelayPass => SelectedIntent::Contain(ContainTechnique::DelayPass),
        CandidateAction::CoverSpace => SelectedIntent::Contain(ContainTechnique::CoverSpace),
        CandidateAction::BlockLane => SelectedIntent::Contain(ContainTechnique::BlockLane),

        // Press
        CandidateAction::ClosingDown => SelectedIntent::Press(PressTechnique::ClosingDown),
        CandidateAction::InterceptAttempt => {
            SelectedIntent::Press(PressTechnique::InterceptAttempt)
        }
        CandidateAction::ForceTouchline => SelectedIntent::Press(PressTechnique::ForceTouchline),
        CandidateAction::TrackRunner => SelectedIntent::Press(PressTechnique::TrackRunner),

        // Challenge
        CandidateAction::StandingTackle => {
            SelectedIntent::Challenge(ChallengeTechnique::StandingTackle)
        }
        CandidateAction::SlidingTackle => {
            SelectedIntent::Challenge(ChallengeTechnique::SlidingTackle)
        }
        CandidateAction::ShoulderCharge => {
            SelectedIntent::Challenge(ChallengeTechnique::ShoulderCharge)
        }
        CandidateAction::PokeAway => SelectedIntent::Challenge(ChallengeTechnique::PokeAway),

        // Transition
        CandidateAction::CounterAttackRun => SelectedIntent::CounterAttack,
        CandidateAction::RecoveryRun => SelectedIntent::Recovery,

        // Special
        CandidateAction::ClearBall => SelectedIntent::Clear,
        CandidateAction::Hold => SelectedIntent::Wait,
    }
}

// ============================================================================
// Outcome Sets (Mutually Exclusive Templates)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutcomeSetId {
    PossessionOutcome,
    ShotOutcome,
    TackleOutcome,
}

#[derive(Clone, Debug)]
pub struct OutcomeCandidate<T> {
    pub id: T,
    pub w: WeightBreakdown,
    pub debug_label: &'static str,
}

/// OutcomeSet 샘플링 표준 API:
/// - 입력: 후보(outcome, WeightBreakdown)
/// - 출력: outcome 하나
/// - RNG는 여기서 1회만 사용 (softmax_select 내부)
pub fn select_outcome_softmax<T: Copy>(
    candidates: &[(T, WeightBreakdown)],
    temperature: f32,
    rng: &mut impl Rng,
) -> Option<T> {
    if candidates.is_empty() {
        return None;
    }
    // Calculate ln(weight) for each candidate
    let scores: Vec<f32> = candidates.iter().map(|(_, bd)| ln_weight(bd)).collect();

    // Use softmax_select (from utility module) which handles exp(score/temp) internally
    // Wait, utility::softmax_select expects utilities (scores), not probabilities.
    // If scores are ln(W), then softmax_select(scores, temp) computes exp(ln(W)/temp) = W^(1/T).
    // This is consistent with p ~ W^(1/T).

    let idx = softmax_select(&scores, temperature, rng);
    Some(candidates[idx].0)
}

// ============================================================================
// FinalAction (Gate C Output)
// ============================================================================

/// Gate C 출력: 실행 가능한 최종 액션
#[derive(Debug, Clone)]
pub struct FinalAction {
    /// 액션 유형
    pub action_type: FinalActionType,
    /// 목표 위치 (옵션)
    pub target_pos: Option<(f32, f32)>,
    /// 목표 선수 인덱스 (옵션)
    pub target_player: Option<usize>,
    /// 힘 (0.0~1.0)
    pub power: f32,
    /// 커브 (-1.0~1.0)
    pub curve: f32,
    /// 추가 파라미터
    pub params: FinalActionParams,
}

impl Default for FinalAction {
    fn default() -> Self {
        Self {
            action_type: FinalActionType::Hold,
            target_pos: None,
            target_player: None,
            power: 0.5,
            curve: 0.0,
            params: FinalActionParams::None,
        }
    }
}

/// 최종 액션 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalActionType {
    // Attack
    Shot,
    Pass,
    Dribble,
    Cross,

    // Defense
    Tackle,
    Movement,
    Block,

    // Special
    Clear,
    Hold,
}

/// 최종 액션 추가 파라미터
#[derive(Debug, Clone)]
pub enum FinalActionParams {
    None,
    Shot(ShotParams),
    Pass(PassParams),
    Dribble(DribbleParams),
    Tackle(TackleParams),
    Movement(MovementParams),
}

#[derive(Debug, Clone, Copy)]
pub struct ShotParams {
    pub technique: ShotTechnique,
    pub foot: Foot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotTechnique {
    Normal,
    Finesse,
    Power,
    Chip,
    Header,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foot {
    Left,
    Right,
    Head,
}

#[derive(Debug, Clone, Copy)]
pub struct PassParams {
    pub pass_type: PassType,
    pub is_lofted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassType {
    Ground,
    Through,
    Lob,
    Cross,
    Clear,
}

#[derive(Debug, Clone, Copy)]
pub struct DribbleParams {
    pub direction: (f32, f32),
    pub is_skill_move: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TackleParams {
    pub tackle_type: TackleType,
    pub commit_level: f32, // 0.0~1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TackleType {
    Standing,
    Sliding,
    Shoulder,
    Poke,
}

#[derive(Debug, Clone, Copy)]
pub struct MovementParams {
    pub movement_type: MovementType,
    pub speed_factor: f32, // 0.0~1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementType {
    Walk,
    Jog,
    Sprint,
    Jockey,
    Track,
}

// ============================================================================
// Elaboration Context
// ============================================================================

/// Gate C에서 사용하는 추가 컨텍스트
#[derive(Debug, Clone, Default)]
pub struct ElaborationContext {
    /// 공격 골대 위치 (상대 골대)
    pub goal_pos: (f32, f32),
    /// 수비 골대 위치 (자기 골대) - FIX_2601: 수비 위치 계산용
    pub defense_goal_pos: (f32, f32),
    /// 볼 위치
    pub ball_pos: (f32, f32),
    /// 선수 위치
    pub player_pos: (f32, f32),
    /// 볼 캐리어 위치 (수비 시)
    pub ball_carrier_pos: Option<(f32, f32)>,
    /// GK 위치
    pub gk_pos: Option<(f32, f32)>,
    /// 패스 타겟 후보들 (idx, pos, quality)
    pub pass_targets: Vec<(usize, (f32, f32), f32)>,
    /// 슛 가능 공간 (near post, center, far post)
    pub shot_zones: [(f32, f32); 3],
    /// FIX_2601/0106 P2-8: 근처 상대 선수 위치 (Force Field 계산용)
    /// 드리블 방향 결정 시 상대 회피에 사용
    pub nearby_opponents: Vec<(f32, f32)>,
    /// FIX_2601/1130: 현재 볼 소유자에게 가장 최근 패스한 선수
    /// SafePass에서 리턴 패스 우선 선택에 사용
    pub recent_passer_idx: Option<usize>,
}

/// SelectedIntent → CandidateAction 변환
pub fn intent_to_candidate(intent: SelectedIntent) -> CandidateAction {
    match intent {
        // Protect
        SelectedIntent::Protect(ProtectStyle::Shield) => CandidateAction::ShieldBall,
        SelectedIntent::Protect(ProtectStyle::HoldUp) => CandidateAction::HoldUpPlay,
        SelectedIntent::Protect(ProtectStyle::DrawFoul) => CandidateAction::DrawFoul,

        // Progress
        SelectedIntent::Progress(ProgressStyle::Safe) => CandidateAction::SafePass,
        SelectedIntent::Progress(ProgressStyle::Progressive) => CandidateAction::ProgressivePass,
        SelectedIntent::Progress(ProgressStyle::Switch) => CandidateAction::SwitchPlay,
        SelectedIntent::Progress(ProgressStyle::Carry) => CandidateAction::CarryBall,

        // Beat
        SelectedIntent::Beat(BeatStyle::TakeOn) => CandidateAction::TakeOn,
        SelectedIntent::Beat(BeatStyle::OneTwo) => CandidateAction::OneTwo,
        SelectedIntent::Beat(BeatStyle::Through) => CandidateAction::ThroughBall,

        // Score
        SelectedIntent::Score(ScoreStyle::Normal) => CandidateAction::ShootNormal,
        SelectedIntent::Score(ScoreStyle::Finesse) => CandidateAction::ShootFinesse,
        SelectedIntent::Score(ScoreStyle::Power) => CandidateAction::ShootPower,
        SelectedIntent::Score(ScoreStyle::Chip) => CandidateAction::ShootChip,
        SelectedIntent::Score(ScoreStyle::Header) => CandidateAction::Header,
        // FIX_2601/0117: Reverse mapping for Cross
        SelectedIntent::Score(ScoreStyle::Cross) => CandidateAction::Cross,

        // Contain
        SelectedIntent::Contain(ContainTechnique::Jockey) => CandidateAction::Jockey,
        SelectedIntent::Contain(ContainTechnique::DelayPass) => CandidateAction::DelayPass,
        SelectedIntent::Contain(ContainTechnique::CoverSpace) => CandidateAction::CoverSpace,
        SelectedIntent::Contain(ContainTechnique::BlockLane) => CandidateAction::BlockLane,

        // Press
        SelectedIntent::Press(PressTechnique::ClosingDown) => CandidateAction::ClosingDown,
        SelectedIntent::Press(PressTechnique::InterceptAttempt) => CandidateAction::InterceptAttempt,
        SelectedIntent::Press(PressTechnique::ForceTouchline) => CandidateAction::ForceTouchline,
        SelectedIntent::Press(PressTechnique::TrackRunner) => CandidateAction::TrackRunner,

        // Challenge
        SelectedIntent::Challenge(ChallengeTechnique::StandingTackle) => CandidateAction::StandingTackle,
        SelectedIntent::Challenge(ChallengeTechnique::SlidingTackle) => CandidateAction::SlidingTackle,
        SelectedIntent::Challenge(ChallengeTechnique::ShoulderCharge) => CandidateAction::ShoulderCharge,
        SelectedIntent::Challenge(ChallengeTechnique::PokeAway) => CandidateAction::PokeAway,

        // Transition
        SelectedIntent::CounterAttack => CandidateAction::CounterAttackRun,
        SelectedIntent::Recovery => CandidateAction::RecoveryRun,

        // Special
        SelectedIntent::Clear => CandidateAction::ClearBall,
        SelectedIntent::Wait => CandidateAction::Hold,
    }
}

// ============================================================================
// Gate C: Intent → FinalAction Elaboration
// ============================================================================

/// Gate C: Intent를 실행 가능한 FinalAction으로 변환
pub fn elaborate_intent(
    intent: SelectedIntent,
    decision_ctx: &DecisionContext,
    elab_ctx: &ElaborationContext,
) -> FinalAction {
    match intent {
        // Attack Intents
        SelectedIntent::Protect(style) => elaborate_protect(style, elab_ctx),
        SelectedIntent::Progress(style) => elaborate_progress(style, decision_ctx, elab_ctx),
        SelectedIntent::Beat(style) => elaborate_beat(style, elab_ctx),
        SelectedIntent::Score(style) => elaborate_score(style, elab_ctx),

        // Defense Intents
        SelectedIntent::Contain(technique) => elaborate_contain(technique, elab_ctx),
        SelectedIntent::Press(technique) => elaborate_press(technique, elab_ctx),
        SelectedIntent::Challenge(technique) => elaborate_challenge(technique, elab_ctx),

        // Transition
        SelectedIntent::CounterAttack => elaborate_counter(elab_ctx),
        SelectedIntent::Recovery => elaborate_recovery(elab_ctx),

        // Special
        SelectedIntent::Clear => elaborate_clear(elab_ctx),
        SelectedIntent::Wait => FinalAction::default(),
    }
}

// ============================================================================
// Protect Elaboration
// ============================================================================

fn elaborate_protect(style: ProtectStyle, ctx: &ElaborationContext) -> FinalAction {
    match style {
        ProtectStyle::Shield => {
            // 볼을 보호하며 상대 반대쪽으로 몸 돌리기
            FinalAction {
                action_type: FinalActionType::Movement,
                target_pos: Some(ctx.player_pos), // 제자리
                power: 0.3,
                params: FinalActionParams::Movement(MovementParams {
                    movement_type: MovementType::Walk,
                    speed_factor: 0.2,
                }),
                ..Default::default()
            }
        }
        ProtectStyle::HoldUp => {
            // 볼을 지키며 팀원 올라올 때까지 대기
            FinalAction {
                action_type: FinalActionType::Hold,
                target_pos: Some(ctx.player_pos),
                power: 0.4,
                params: FinalActionParams::Movement(MovementParams {
                    movement_type: MovementType::Jog,
                    speed_factor: 0.3,
                }),
                ..Default::default()
            }
        }
        ProtectStyle::DrawFoul => {
            // 파울 유도 (상대 방향으로 살짝 이동)
            FinalAction {
                action_type: FinalActionType::Dribble,
                target_pos: ctx.ball_carrier_pos, // 상대 방향
                power: 0.5,
                params: FinalActionParams::Dribble(DribbleParams {
                    direction: (0.0, 0.0),
                    is_skill_move: false,
                }),
                ..Default::default()
            }
        }
    }
}

// ============================================================================
// Progress Elaboration
// ============================================================================

fn elaborate_progress(
    style: ProgressStyle,
    decision_ctx: &DecisionContext,
    ctx: &ElaborationContext,
) -> FinalAction {
    // FIX_2601/1129: Track pass style distribution
    static SAFE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static PROG_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static OTHER_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    match &style {
        ProgressStyle::Safe => { SAFE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
        ProgressStyle::Progressive => { PROG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
        _ => { OTHER_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
    }

    let total = SAFE_COUNT.load(std::sync::atomic::Ordering::Relaxed)
        + PROG_COUNT.load(std::sync::atomic::Ordering::Relaxed)
        + OTHER_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    if total > 0 && total % 500 == 0 {
        let safe = SAFE_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let prog = PROG_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let other = OTHER_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        eprintln!(
            "[PASS_STYLE] Total: {} | SafePass: {} ({:.1}%) | ProgressivePass: {} ({:.1}%) | Other: {} ({:.1}%)",
            total,
            safe, 100.0 * safe as f64 / total as f64,
            prog, 100.0 * prog as f64 / total as f64,
            other, 100.0 * other as f64 / total as f64
        );
    }

    match style {
        ProgressStyle::Safe => {
            // FIX_2601/1128: SafePass MUST prefer backward targets to reduce forward_pass_rate
            // Target metric: forward_pass_rate 48% → 22%
            // Analysis counts forward pass as >=7m in attack direction
            // FIX_2601/0123: Phase-aware target selection
            // - Circulation: backward > return > lateral > fallback (minimize forward)
            // - Other phases: return > backward > lateral > fallback (prioritize reciprocity)
            let attacks_right = ctx.goal_pos.0 > ctx.player_pos.0;
            let in_circulation = decision_ctx.attack_phase == crate::engine::match_sim::attack_phase::AttackPhase::Circulation;

            // Helper: check if a target position is forward (would count as forward pass in QA metrics)
            let is_forward_target = |pos: (f32, f32)| -> bool {
                let dx = if attacks_right {
                    pos.0 - ctx.player_pos.0
                } else {
                    ctx.player_pos.0 - pos.0
                };
                dx >= 7.0 // Forward by analysis threshold
            };

            // Find return target (recent passer)
            let return_target = ctx.recent_passer_idx.and_then(|passer_idx| {
                ctx.pass_targets
                    .iter()
                    .find(|(idx, _, _)| *idx == passer_idx)
            });

            // FIX_2601/0123: Check if return target is forward
            let return_is_forward = return_target.map_or(false, |t| is_forward_target(t.1));

            // Get the passer index to exclude from non-return options
            let passer_idx_to_exclude = ctx.recent_passer_idx;

            // Find backward target (7m+ behind) - EXCLUDING the recent passer
            let backward_target = ctx.pass_targets
                .iter()
                .filter(|(idx, pos, _)| {
                    // FIX_2601/0123: Exclude recent passer to reduce reciprocity
                    if passer_idx_to_exclude == Some(*idx) {
                        return false;
                    }
                    if attacks_right {
                        pos.0 < ctx.player_pos.0 - 7.0 // At least 7m backward
                    } else {
                        pos.0 > ctx.player_pos.0 + 7.0 // At least 7m backward
                    }
                })
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

            // Find lateral target (not forward by threshold) - EXCLUDING the recent passer
            let lateral_target = ctx.pass_targets
                .iter()
                .filter(|(idx, pos, _)| {
                    // FIX_2601/0123: Exclude recent passer to reduce reciprocity
                    if passer_idx_to_exclude == Some(*idx) {
                        return false;
                    }
                    !is_forward_target(*pos)
                })
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

            // FIX_2601/0123: Debug counters
            static RETURN_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            static BACKWARD_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            static LATERAL_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            static FALLBACK_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            // FIX_2601/0123: Phase-aware target selection (balanced for 20-25% forward)
            // V6: Use ~70% return (was 50%) to maintain forward_pass_rate while reducing reciprocity.
            //
            // Determinism: Use match-local sequence (provided by MatchEngine) instead of global counters.
            let use_return_pass = decision_ctx.safe_pass_seq % 10 < 7; // ~70% true

            let target = if in_circulation {
                // Circulation phase: balanced approach
                // 1. Forward returns are OK (reciprocity + forward progress) - but only 50%
                // 2. If return is backward, use it
                // 3. Otherwise, allow some forward lateral passes
                if let Some(ret) = return_target {
                    if use_return_pass {
                        // Return pass - but only 50% of the time
                        RETURN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(ret)
                    } else if let Some(bwd) = backward_target {
                        // Skip return, use backward instead
                        BACKWARD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(bwd)
                    } else if let Some(lat) = lateral_target {
                        LATERAL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(lat)
                    } else {
                        // No alternative, use return as fallback
                        RETURN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(ret)
                    }
                } else {
                    // No return target: balance backward vs lateral
                    // Use player position hash for deterministic ~30% forward allowance
                    let allow_forward_lateral = {
                        let hash = ((ctx.player_pos.0 * 1000.0) as u64).wrapping_mul(31)
                            .wrapping_add((ctx.player_pos.1 * 1000.0) as u64);
                        hash % 100 < 30 // ~30% chance to allow non-backward
                    };

                    if let Some(bwd) = backward_target {
                        if !allow_forward_lateral {
                            BACKWARD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            Some(bwd)
                        } else if let Some(lat) = lateral_target {
                            // Allow lateral (may be slightly forward)
                            LATERAL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            Some(lat)
                        } else {
                            BACKWARD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            Some(bwd)
                        }
                    } else if let Some(lat) = lateral_target {
                        LATERAL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(lat)
                    } else {
                        FALLBACK_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        ctx.pass_targets.first()
                    }
                }
            } else {
                // Positional/Transition: return pass only 50% of the time
                if let Some(ret) = return_target {
                    if use_return_pass {
                        RETURN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(ret)
                    } else if let Some(bwd) = backward_target {
                        BACKWARD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(bwd)
                    } else if let Some(lat) = lateral_target {
                        LATERAL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(lat)
                    } else {
                        // No alternative, use return as fallback
                        RETURN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(ret)
                    }
                } else if let Some(bwd) = backward_target {
                    BACKWARD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Some(bwd)
                } else if let Some(lat) = lateral_target {
                    LATERAL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Some(lat)
                } else {
                    FALLBACK_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    ctx.pass_targets.first()
                }
            };

            let total = RETURN_COUNT.load(std::sync::atomic::Ordering::Relaxed)
                + BACKWARD_COUNT.load(std::sync::atomic::Ordering::Relaxed)
                + LATERAL_COUNT.load(std::sync::atomic::Ordering::Relaxed)
                + FALLBACK_COUNT.load(std::sync::atomic::Ordering::Relaxed);
            if total > 0 && total % 500 == 0 {
                let ret = RETURN_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                let bwd = BACKWARD_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                let lat = LATERAL_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                let fb = FALLBACK_COUNT.load(std::sync::atomic::Ordering::Relaxed);
                eprintln!(
                    "[SAFE_TARGET] Total: {} | Return: {} ({:.1}%) | Backward: {} ({:.1}%) | Lateral: {} ({:.1}%) | Fallback: {} ({:.1}%)",
                    total,
                    ret, 100.0 * ret as f64 / total as f64,
                    bwd, 100.0 * bwd as f64 / total as f64,
                    lat, 100.0 * lat as f64 / total as f64,
                    fb, 100.0 * fb as f64 / total as f64
                );
            }
            FinalAction {
                action_type: FinalActionType::Pass,
                target_pos: target.map(|t| t.1),
                target_player: target.map(|t| t.0),
                power: 0.5,
                params: FinalActionParams::Pass(PassParams {
                    pass_type: PassType::Ground,
                    is_lofted: false,
                }),
                ..Default::default()
            }
        }
        ProgressStyle::Progressive => {
            // FIX_2601/1128: Quality-weighted forward pass selection
            // Previously: picked the most forward target by position (ignored quality)
            // Now: among forward targets, pick the highest QUALITY target
            // This allows AttackSubPhase to influence pass selection via quality values
            let attacks_right = ctx.goal_pos.0 > ctx.player_pos.0;
            let target = ctx
                .pass_targets
                .iter()
                .filter(|(_, pos, _)| {
                    if attacks_right {
                        pos.0 > ctx.player_pos.0 // Home: forward = higher x
                    } else {
                        pos.0 < ctx.player_pos.0 // Away: forward = lower x
                    }
                })
                .max_by(|a, b| {
                    // FIX_2601/1128: Use quality (field index 2) instead of position
                    a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal)
                });
            FinalAction {
                action_type: FinalActionType::Pass,
                target_pos: target.map(|t| t.1),
                target_player: target.map(|t| t.0),
                power: 0.65,
                params: FinalActionParams::Pass(PassParams {
                    pass_type: PassType::Ground,
                    is_lofted: decision_ctx.local_pressure > 0.6,
                }),
                ..Default::default()
            }
        }
        ProgressStyle::Switch => {
            // 가장 먼 측면 패스 타겟
            let target = ctx.pass_targets.iter().max_by(|a, b| {
                let dist_a = (a.1 .1 - ctx.player_pos.1).abs();
                let dist_b = (b.1 .1 - ctx.player_pos.1).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            });
            FinalAction {
                action_type: FinalActionType::Pass,
                target_pos: target.map(|t| t.1),
                target_player: target.map(|t| t.0),
                power: 0.85,
                params: FinalActionParams::Pass(PassParams {
                    pass_type: PassType::Lob,
                    is_lofted: true,
                }),
                ..Default::default()
            }
        }
        ProgressStyle::Carry => {
            // 드리블로 전진
            // FIX_2601/0106 P2-8: Full Force Field (골 유인 + 상대 회피 + 측면 회피)
            let direction = force_field::calculate_dribble_direction(
                ctx.player_pos,
                ctx.goal_pos,
                &ctx.nearby_opponents,
            );
            let offset = 5.0; // 5m 전진
            FinalAction {
                action_type: FinalActionType::Dribble,
                target_pos: Some((
                    ctx.player_pos.0 + direction.0 * offset,
                    ctx.player_pos.1 + direction.1 * offset,
                )),
                power: 0.6,
                params: FinalActionParams::Dribble(DribbleParams {
                    direction,
                    is_skill_move: false,
                }),
                ..Default::default()
            }
        }
    }
}

// ============================================================================
// Beat Elaboration
// ============================================================================

fn elaborate_beat(style: BeatStyle, ctx: &ElaborationContext) -> FinalAction {
    match style {
        BeatStyle::TakeOn => {
            // 상대 제치기
            // FIX_2601/0106 P2-8: Full Force Field (골 유인 + 상대 회피 + 측면 회피)
            let direction = force_field::calculate_dribble_direction(
                ctx.player_pos,
                ctx.goal_pos,
                &ctx.nearby_opponents,
            );
            let offset = 3.0; // 3m 전진 (돌파)
            FinalAction {
                action_type: FinalActionType::Dribble,
                target_pos: Some((
                    ctx.player_pos.0 + direction.0 * offset,
                    ctx.player_pos.1 + direction.1 * offset,
                )),
                power: 0.75,
                params: FinalActionParams::Dribble(DribbleParams {
                    direction,
                    is_skill_move: true,
                }),
                ..Default::default()
            }
        }
        BeatStyle::OneTwo => {
            // 원투 패스
            let target = ctx.pass_targets.first();
            FinalAction {
                action_type: FinalActionType::Pass,
                target_pos: target.map(|t| t.1),
                target_player: target.map(|t| t.0),
                power: 0.6,
                params: FinalActionParams::Pass(PassParams {
                    pass_type: PassType::Ground,
                    is_lofted: false,
                }),
                ..Default::default()
            }
        }
        BeatStyle::Through => {
            // 쓰루 패스
            // FIX_2601/0105: Direction-aware through pass selection
            let attacks_right = ctx.goal_pos.0 > ctx.player_pos.0;
            let target = ctx
                .pass_targets
                .iter()
                .filter(|(_, pos, _)| {
                    if attacks_right {
                        pos.0 > ctx.player_pos.0 + 10.0 // Home: target ahead
                    } else {
                        pos.0 < ctx.player_pos.0 - 10.0 // Away: target ahead (lower x)
                    }
                })
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
            // FIX_2601/0105: Direction-aware running space offset
            let run_offset = if attacks_right { 8.0 } else { -8.0 };
            FinalAction {
                action_type: FinalActionType::Pass,
                target_pos: target.map(|t| (t.1 .0 + run_offset, t.1 .1)), // 러닝 공간으로
                target_player: target.map(|t| t.0),
                power: 0.75,
                params: FinalActionParams::Pass(PassParams {
                    pass_type: PassType::Through,
                    is_lofted: false,
                }),
                ..Default::default()
            }
        }
    }
}

// ============================================================================
// Score Elaboration
// ============================================================================

fn elaborate_score(style: ScoreStyle, ctx: &ElaborationContext) -> FinalAction {
    // FIX_2601/0117: Cross is handled separately (not a shot)
    if style == ScoreStyle::Cross {
        // Find best target in the box (attacking penalty area)
        // Box bounds: x within 16.5m of goal line, y within 13.84-54.16
        let target = ctx
            .pass_targets
            .iter()
            .filter(|(_, pos, _)| {
                // Attacking box: x > 88.5 for home, x < 16.5 for away
                // y: 13.84 to 54.16 (40.32m width centered at 34)
                let in_box_x =
                    if ctx.goal_pos.0 > field::CENTER_X { pos.0 > 80.0 } else { pos.0 < 25.0 };
                let in_box_y = pos.1 > 10.0 && pos.1 < 58.0; // Slightly wider for crosses
                in_box_x && in_box_y
            })
            .max_by(|a, b| {
                // 골대 가까운 순
                let dist_a =
                    ((a.1 .0 - ctx.goal_pos.0).powi(2) + (a.1 .1 - field::CENTER_Y).powi(2)).sqrt();
                let dist_b =
                    ((b.1 .0 - ctx.goal_pos.0).powi(2) + (b.1 .1 - field::CENTER_Y).powi(2)).sqrt();
                dist_b.partial_cmp(&dist_a).unwrap() // 가까울수록 우선
            });

        // If no target in box, fall back to nearest forward teammate
        let final_target = target.or_else(|| {
            ctx.pass_targets
                .iter()
                .filter(|(_, pos, _)| {
                    let forward_x =
                        if ctx.goal_pos.0 > field::CENTER_X { pos.0 > 70.0 } else { pos.0 < 35.0 };
                    forward_x
                })
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });

        return FinalAction {
            action_type: FinalActionType::Cross,
            target_pos: final_target.map(|t| t.1),
            target_player: final_target.map(|t| t.0),
            power: 0.75,
            params: FinalActionParams::Pass(PassParams {
                pass_type: PassType::Cross,
                is_lofted: true,
            }),
            ..Default::default()
        };
    }

    let (technique, target_zone) = match style {
        ScoreStyle::Normal => (ShotTechnique::Normal, ctx.shot_zones[1]), // center
        ScoreStyle::Finesse => (ShotTechnique::Finesse, ctx.shot_zones[2]), // far post
        ScoreStyle::Power => (ShotTechnique::Power, ctx.shot_zones[1]),   // center
        ScoreStyle::Chip => (ShotTechnique::Chip, ctx.shot_zones[1]),
        ScoreStyle::Header => (ShotTechnique::Header, ctx.shot_zones[1]),
        ScoreStyle::Cross => unreachable!(), // Handled above
    };

    let power = match style {
        ScoreStyle::Power => 0.95,
        ScoreStyle::Finesse => 0.65,
        ScoreStyle::Chip => 0.55,
        ScoreStyle::Header => 0.7,
        ScoreStyle::Normal => 0.8,
        ScoreStyle::Cross => unreachable!(),
    };

    let curve = match style {
        ScoreStyle::Finesse => 0.6,
        ScoreStyle::Chip => 0.3,
        _ => 0.0,
    };

    FinalAction {
        action_type: FinalActionType::Shot,
        target_pos: Some(target_zone),
        power,
        curve,
        params: FinalActionParams::Shot(ShotParams {
            technique,
            foot: Foot::Right, // 실제 구현에서는 선수 정보 반영
        }),
        ..Default::default()
    }
}

// ============================================================================
// Defense Elaboration
// ============================================================================

fn elaborate_contain(technique: ContainTechnique, ctx: &ElaborationContext) -> FinalAction {
    let movement_type = match technique {
        ContainTechnique::Jockey => MovementType::Jockey,
        ContainTechnique::DelayPass => MovementType::Walk,
        ContainTechnique::CoverSpace => MovementType::Jog,
        ContainTechnique::BlockLane => MovementType::Walk,
    };

    let target_pos = ctx.ball_carrier_pos.map(|carrier| {
        // 볼 캐리어와 골대 사이에 위치
        // FIX_2601: 수비 골대 위치를 컨텍스트에서 가져옴 (was hardcoded 0.0)
        let goal_x = ctx.defense_goal_pos.0;
        let mid_x = (carrier.0 + goal_x) / 2.0;
        (mid_x.max(carrier.0 - 3.0), carrier.1)
    });

    FinalAction {
        action_type: FinalActionType::Movement,
        target_pos,
        power: 0.5,
        params: FinalActionParams::Movement(MovementParams { movement_type, speed_factor: 0.5 }),
        ..Default::default()
    }
}

fn elaborate_press(technique: PressTechnique, ctx: &ElaborationContext) -> FinalAction {
    let (movement_type, speed_factor) = match technique {
        PressTechnique::ClosingDown => (MovementType::Sprint, 0.9),
        PressTechnique::InterceptAttempt => (MovementType::Sprint, 0.95),
        PressTechnique::ForceTouchline => (MovementType::Jog, 0.7),
        PressTechnique::TrackRunner => (MovementType::Track, 0.8),
    };

    FinalAction {
        action_type: FinalActionType::Movement,
        target_pos: ctx.ball_carrier_pos.or(Some(ctx.ball_pos)),
        power: speed_factor,
        params: FinalActionParams::Movement(MovementParams { movement_type, speed_factor }),
        ..Default::default()
    }
}

fn elaborate_challenge(technique: ChallengeTechnique, ctx: &ElaborationContext) -> FinalAction {
    let tackle_type = match technique {
        ChallengeTechnique::StandingTackle => TackleType::Standing,
        ChallengeTechnique::SlidingTackle => TackleType::Sliding,
        ChallengeTechnique::ShoulderCharge => TackleType::Shoulder,
        ChallengeTechnique::PokeAway => TackleType::Poke,
    };

    let commit_level = match technique {
        ChallengeTechnique::SlidingTackle => 1.0,
        ChallengeTechnique::ShoulderCharge => 0.8,
        ChallengeTechnique::StandingTackle => 0.7,
        ChallengeTechnique::PokeAway => 0.4,
    };

    FinalAction {
        action_type: FinalActionType::Tackle,
        target_pos: ctx.ball_carrier_pos.or(Some(ctx.ball_pos)),
        power: commit_level,
        params: FinalActionParams::Tackle(TackleParams { tackle_type, commit_level }),
        ..Default::default()
    }
}

// ============================================================================
// Transition & Special Elaboration
// ============================================================================

fn elaborate_counter(ctx: &ElaborationContext) -> FinalAction {
    FinalAction {
        action_type: FinalActionType::Movement,
        target_pos: Some((ctx.goal_pos.0, ctx.player_pos.1)),
        power: 0.95,
        params: FinalActionParams::Movement(MovementParams {
            movement_type: MovementType::Sprint,
            speed_factor: 1.0,
        }),
        ..Default::default()
    }
}

fn elaborate_recovery(ctx: &ElaborationContext) -> FinalAction {
    // FIX_2601/0105: Direction-aware recovery target
    FinalAction {
        action_type: FinalActionType::Movement,
        target_pos: Some((ctx.defense_goal_pos.0, ctx.player_pos.1)), // 수비 진영으로
        power: 0.9,
        params: FinalActionParams::Movement(MovementParams {
            movement_type: MovementType::Sprint,
            speed_factor: 0.95,
        }),
        ..Default::default()
    }
}

fn elaborate_clear(ctx: &ElaborationContext) -> FinalAction {
    // FIX_2601/0105: Direction-aware clearance target
    let attacks_right = ctx.goal_pos.0 > ctx.player_pos.0;
    let clear_offset = if attacks_right { 30.0 } else { -30.0 };
    FinalAction {
        action_type: FinalActionType::Clear,
        target_pos: Some((ctx.player_pos.0 + clear_offset, ctx.player_pos.1)), // 앞으로 걷어내기
        power: 0.9,
        params: FinalActionParams::Pass(PassParams { pass_type: PassType::Clear, is_lofted: true }),
        ..Default::default()
    }
}

// ============================================================================
// Full Pipeline (Gate A → Gate B → Gate C)
// ============================================================================

/// P16 완전한 의사결정 파이프라인
///
/// Gate A (Hard/Budget/Soft) → Gate B (Choice) → Gate C (Elaboration)
pub fn decide_action_with_detail(
    mindset: PlayerMindset,
    decision_ctx: &DecisionContext,
    elab_ctx: &ElaborationContext,
    bias: &CognitiveBias,
    flair: f32,
    decisions: f32,
    concentration: f32,
    rng: &mut impl Rng,
) -> (
    FinalAction,
    SelectedIntent,
    f32,
    Vec<(CandidateAction, UtilityResult)>,
    ShotGateOutcome,
) {
    // FIX_2601/0117: Entry point debug (guarded; keep RunOps quiet)
    if crate::engine::debug_flags::match_debug_enabled() {
        static ENTRY_COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let cnt = ENTRY_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if cnt % 100 == 0 {
            eprintln!("[DECISION ENTRY #{}] mindset={:?}", cnt, mindset);
        }
    }

    use crate::engine::physics_constants::forced_shot;

    // FIX_2601/0114: 강제 중거리 슈팅 체크
    // 슈팅 정확도를 35-40%로 낮추기 위해 저품질 슈팅 강제 발생
    if decision_ctx.has_ball {
        let dist = decision_ctx.distance_to_goal;
        let in_medium_range = dist >= forced_shot::MEDIUM_RANGE_MIN_M
            && dist <= forced_shot::MEDIUM_RANGE_MAX_M;
        let low_pressure = decision_ctx.local_pressure < forced_shot::MAX_PRESSURE_FOR_FORCED_SHOT;

        if in_medium_range && low_pressure {
            let roll: f32 = rng.gen();
            if roll < forced_shot::FORCED_SHOT_PROBABILITY {
                // 강제 중거리 슈팅 발동!
                let forced_action = FinalAction {
                    action_type: FinalActionType::Shot,
                    target_pos: Some(elab_ctx.goal_pos), // 골대 중앙
                    target_player: None,
                    power: 0.7 + rng.gen::<f32>() * 0.2, // 0.7-0.9
                    curve: 0.0,
                    params: FinalActionParams::Shot(ShotParams {
                        technique: ShotTechnique::Normal,
                        foot: Foot::Right, // 기본값
                    }),
                };
                let forced_intent = SelectedIntent::Score(ScoreStyle::Normal);
                // 낮은 xG의 강제 슈팅이므로 utility도 낮게 설정
                let forced_utility = 0.15 + decision_ctx.xg * 0.3;
                return (
                    forced_action,
                    forced_intent,
                    forced_utility,
                    vec![],
                    ShotGateOutcome::unchecked(),
                );
            }
        }
    }

    // Contract v1.0 Integrity Check: Tactics MUST be applied
    if decision_ctx.tactical_trace.is_empty() {
        // In production, we might just log, but according to contract, this is
        // a FAIL
        #[cfg(debug_assertions)]
        panic!("[CONTRACT VIOLATION] Decision Gate reached without tactical trace! Tactics are not being passed through the hot-path.");
    }

    // 1. GateChain (Gate A)
    let (hard_candidates, shot_gate) = apply_hard_gates(mindset, decision_ctx);
    let budget_candidates = apply_budget_gates(&hard_candidates, decision_ctx);
    let soft_candidates = apply_soft_gates(&budget_candidates, decision_ctx);

    // FIX_2601/0117: Debug logging for Cross/SwitchPlay candidates
    #[cfg(debug_assertions)]
    {
        let has_cross = soft_candidates.iter().any(|(c, _)| *c == CandidateAction::Cross);
        let has_switch = soft_candidates.iter().any(|(c, _)| *c == CandidateAction::SwitchPlay);

        // Sample every 50 decisions to avoid spam
        static DEBUG_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = DEBUG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if count % 50 == 0 {
            eprintln!(
                "[GATE-A #{count}] mindset={:?} opts={} | Cross={} Switch={} | wide={} atk3rd={} box={}",
                mindset,
                decision_ctx.pass_options_count,
                has_cross, has_switch,
                decision_ctx.is_wide_position, decision_ctx.in_attacking_third,
                decision_ctx.teammates_in_box
            );
        }
    }

    // 2. Utility Selection (Gate B)
    let base_temperature = calculate_temperature(flair, decisions, concentration);
    let temperature =
        apply_team_tempo_temperature_factor(base_temperature, decision_ctx.team_tempo_factor);
    let (intent, utility, results) =
        select_best_intent(&soft_candidates, decision_ctx, bias, temperature, rng);

    // FIX_2601/0117: Debug logging for Cross/SwitchPlay selection result
    #[cfg(debug_assertions)]
    if crate::engine::debug_flags::action_debug_enabled() {
        let selected = intent_to_candidate(intent);
        if selected == CandidateAction::Cross || selected == CandidateAction::SwitchPlay {
            eprintln!(
                "[AERIAL DEBUG] Gate B SELECTED: {:?} utility={:.3}",
                selected, utility
            );
        }
    }

    // 3. Elaboration (Gate C)
    let action = elaborate_intent(intent, decision_ctx, elab_ctx);

    (action, intent, utility, results, shot_gate)
}

/// P16 완전한 의사결정 파이프라인
///
/// Gate A (Hard/Budget/Soft) → Gate B (Choice) → Gate C (Elaboration)
pub fn decide_action(
    mindset: PlayerMindset,
    decision_ctx: &DecisionContext,
    elab_ctx: &ElaborationContext,
    bias: &CognitiveBias,
    flair: f32,
    decisions: f32,
    concentration: f32,
    rng: &mut impl Rng,
) -> (FinalAction, SelectedIntent, f32) {
    let (action, intent, utility, _results, _shot_gate) = decide_action_with_detail(
        mindset,
        decision_ctx,
        elab_ctx,
        bias,
        flair,
        decisions,
        concentration,
        rng,
    );

    (action, intent, utility)
}

// ============================================================================
// FIX_2601/0117: Snapshot-compatible decision functions
// ============================================================================

/// P16 완전한 의사결정 파이프라인 (Snapshot-compatible)
///
/// Actor별 독립 RNG를 사용하여 순서 독립적 결정을 보장합니다.
/// `actor_seed`는 `TickSnapshot::derive_actor_seed(actor_id)`로 생성됩니다.
#[cfg(feature = "snapshot_decide")]
pub fn decide_action_with_detail_snapshot(
    mindset: PlayerMindset,
    decision_ctx: &DecisionContext,
    elab_ctx: &ElaborationContext,
    bias: &CognitiveBias,
    flair: f32,
    decisions: f32,
    concentration: f32,
    actor_seed: u64,
) -> (
    FinalAction,
    SelectedIntent,
    f32,
    Vec<(CandidateAction, UtilityResult)>,
    ShotGateOutcome,
) {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    // Actor별 독립 RNG 생성 - 결정론적이며 순서 독립적
    let mut rng = ChaCha8Rng::seed_from_u64(actor_seed);

    decide_action_with_detail(
        mindset,
        decision_ctx,
        elab_ctx,
        bias,
        flair,
        decisions,
        concentration,
        &mut rng,
    )
}

/// P16 의사결정 파이프라인 (Snapshot-compatible, 간단 버전)
#[cfg(feature = "snapshot_decide")]
pub fn decide_action_snapshot(
    mindset: PlayerMindset,
    decision_ctx: &DecisionContext,
    elab_ctx: &ElaborationContext,
    bias: &CognitiveBias,
    flair: f32,
    decisions: f32,
    concentration: f32,
    actor_seed: u64,
) -> (FinalAction, SelectedIntent, f32) {
    let (action, intent, utility, _results, _shot_gate) = decide_action_with_detail_snapshot(
        mindset,
        decision_ctx,
        elab_ctx,
        bias,
        flair,
        decisions,
        concentration,
        actor_seed,
    );

    (action, intent, utility)
}

// ============================================================================
// Main Entry Point (Legacy)// ============================================================================
// Main Entry Point (Legacy)
// ============================================================================

/// P16 의사결정 파이프라인 (Main Entry Point)
///
/// Gate A (Hard/Budget/Soft) → Gate B (Choice)
pub fn decide_intent(
    mindset: PlayerMindset,
    ctx: &DecisionContext,
    bias: &CognitiveBias,
    flair: f32,
    decisions: f32,
    concentration: f32,
    rng: &mut impl Rng,
) -> (SelectedIntent, f32) {
    // 1. GateChain (Gate A)
    let (hard_candidates, _shot_gate) = apply_hard_gates(mindset, ctx);
    let budget_candidates = apply_budget_gates(&hard_candidates, ctx);
    let soft_candidates = apply_soft_gates(&budget_candidates, ctx);

    // 2. Utility Selection (Gate B)
    let temperature = calculate_temperature(flair, decisions, concentration);
    let (intent, utility, _results) =
        select_best_intent(&soft_candidates, ctx, bias, temperature, rng);

    (intent, utility)
}

// ============================================================================
// 0108: Decision Intent Logging (Phase 2)
// ============================================================================

use crate::models::replay::types::{ActionAlternative, DecisionIntent, IntentContext};

/// Build a DecisionIntent from Gate B results
///
/// Captures the decision-making context for debugging, analysis, and replay.
/// Call this after `select_best_intent` to log why a particular action was chosen.
pub fn build_decision_intent(
    player_id: u32,
    tick: u32,
    selected_action: CandidateAction,
    selected_utility: f32, // Gate B utility score
    all_results: &[(CandidateAction, UtilityResult)],
    temperature: f32,
    ctx: &DecisionContext,
) -> DecisionIntent {
    // Calculate softmax probabilities from utilities
    let utilities: Vec<f32> = all_results.iter().map(|(_, r)| r.utility).collect();
    let probabilities = compute_softmax_probabilities(&utilities, temperature);

    // Find index of selected action to get its probability
    let selected_idx = all_results.iter().position(|(a, _)| *a == selected_action).unwrap_or(0);
    let confidence = probabilities.get(selected_idx).copied().unwrap_or(0.5);

    // Build top alternatives (exclude selected, take top N)
    let mut indexed_probs: Vec<(usize, f32)> = probabilities.iter().copied().enumerate().collect();
    indexed_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    const ALT_TOP_N: usize = 5;
    let mut alternatives: Vec<ActionAlternative> = indexed_probs
        .iter()
        .filter(|(i, _)| *i != selected_idx)
        .take(ALT_TOP_N)
        .map(|(i, prob)| {
            let action_name = format!("{:?}", all_results[*i].0);
            ActionAlternative { action: action_name, probability: *prob }       
        })
        .collect();

    // Keep visibility into Cross/SwitchPlay competition even when they lose hard.
    let mut ensure_action = |action: CandidateAction| {
        if action == selected_action {
            return;
        }
        let idx = match all_results.iter().position(|(a, _)| *a == action) {
            Some(i) => i,
            None => return,
        };
        if idx == selected_idx {
            return;
        }
        let name = format!("{:?}", action);
        if alternatives.iter().any(|alt| alt.action == name) {
            return;
        }
        let prob = probabilities.get(idx).copied().unwrap_or(0.0);
        alternatives.push(ActionAlternative { action: name, probability: prob });
    };

    ensure_action(CandidateAction::Cross);
    ensure_action(CandidateAction::SwitchPlay);

    // Build context (stamina from defense_ctx if available)
    let stamina = ctx.defense_ctx.as_ref().map(|d| d.stamina_percent).unwrap_or(1.0);

    let intent_ctx = IntentContext {
        pressure_level: ctx.local_pressure,
        stamina_percent: stamina,
        in_attacking_third: ctx.in_attacking_third,
        ball_distance: ctx.distance_to_ball,
    };

    DecisionIntent {
        player_id,
        tick,
        chosen_action: format!("{:?}", selected_action),
        confidence,
        alternatives,
        context: intent_ctx,
        selected_utility: Some(selected_utility),
        player_pos: None,
        target_pos: None,
        target_player_id: None,
        pass_targets: Vec::new(),
        nearby_opponents: Vec::new(),
    }
}

/// Compute softmax probabilities from utilities
fn compute_softmax_probabilities(utilities: &[f32], temperature: f32) -> Vec<f32> {
    if utilities.is_empty() {
        return vec![];
    }

    // Find max for numerical stability
    let max_u = utilities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    // Compute exp((u - max) / temp)
    let exps: Vec<f32> = utilities.iter().map(|u| ((u - max_u) / temperature).exp()).collect();

    let sum: f32 = exps.iter().sum();

    if sum > 0.0 {
        exps.iter().map(|e| e / sum).collect()
    } else {
        vec![1.0 / utilities.len() as f32; utilities.len()]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn default_ctx() -> DecisionContext {
        DecisionContext {
            xg: 0.1,
            distance_to_goal: 30.0,
            local_pressure: 0.3,
            has_ball: true,
            is_defending: false,
            pass_options_count: 3,
            nearest_opponent_dist: 5.0,
            nearest_teammate_dist: 10.0,
            tactical_trace: vec!["test_default".to_string()], // Contract v1.0 requirement
            ..Default::default()
        }
    }

    #[test]
    fn test_filter_candidates_attack_score() {
        let ctx = DecisionContext {
            xg: 0.3,
            nearest_opponent_dist: 2.0,
            pass_options_count: 2,
            ..default_ctx()
        };

        let candidates = filter_candidates_by_mindset(PlayerMindset::AttackScore, &ctx);

        assert!(candidates.contains(&CandidateAction::ShootNormal));
        assert!(candidates.contains(&CandidateAction::TakeOn));
        assert!(candidates.len() >= 4);
    }

    #[test]
    fn test_filter_candidates_defend_contain() {
        let ctx = default_ctx();

        let candidates = filter_candidates_by_mindset(PlayerMindset::DefendContain, &ctx);

        assert!(candidates.contains(&CandidateAction::Jockey));
        assert!(candidates.contains(&CandidateAction::CoverSpace));
        assert!(!candidates.contains(&CandidateAction::ShootNormal));
    }

    #[test]
    fn test_filter_candidates_high_pressure_adds_protect() {
        let ctx = DecisionContext { local_pressure: 0.8, has_ball: true, ..default_ctx() };

        let candidates = filter_candidates_by_mindset(PlayerMindset::AttackProgress, &ctx);

        assert!(candidates.contains(&CandidateAction::ShieldBall));
        assert!(candidates.contains(&CandidateAction::SafePass));
    }

    #[test]
    fn test_select_best_intent() {
        let mut rng = test_rng();
        let ctx = DecisionContext {
            xg: 0.35, // 높은 xG
            distance_to_goal: 15.0,
            ..default_ctx()
        };

        let candidates = vec![
            (CandidateAction::ShootNormal, crate::engine::weights::WeightBreakdown::neutral()),
            (CandidateAction::SafePass, crate::engine::weights::WeightBreakdown::neutral()),
            (CandidateAction::TakeOn, crate::engine::weights::WeightBreakdown::neutral()),
        ];

        let bias = CognitiveBias::default();
        let temperature = 0.3;

        let (intent, utility, results) =
            select_best_intent(&candidates, &ctx, &bias, temperature, &mut rng);

        // 높은 xG에서는 슛이 선호되어야 함
        assert!(results.len() == 3);
        assert!(utility > -1.0 && utility < 1.0);

        // 결과가 유효한 Intent인지 확인
        assert!(matches!(
            intent,
            SelectedIntent::Score(_) | SelectedIntent::Progress(_) | SelectedIntent::Beat(_)
        ));
    }

    #[test]
    fn test_decide_intent_pipeline() {
        let mut rng = test_rng();
        let ctx = default_ctx();
        let bias = CognitiveBias::default();

        let (intent, utility) = decide_intent(
            PlayerMindset::AttackProgress,
            &ctx,
            &bias,
            0.6, // flair
            0.7, // decisions
            0.6, // concentration
            &mut rng,
        );

        // Progress mindset에서는 Progress Intent가 나올 가능성 높음
        assert!(utility > -1.0);
        println!("Intent: {:?}, Utility: {}", intent, utility);
    }

    #[test]
    fn test_candidate_action_risk_levels() {
        // 고위험 액션들
        assert!(CandidateAction::ShootPower.risk_level() > 0.7);
        assert!(CandidateAction::SlidingTackle.risk_level() > 0.7);

        // 저위험 액션들
        assert!(CandidateAction::SafePass.risk_level() < 0.3);
        assert!(CandidateAction::Jockey.risk_level() < 0.2);
    }

    #[test]
    fn test_candidate_to_intent_mapping() {
        // 모든 CandidateAction이 유효한 Intent로 매핑되는지 확인
        let actions = vec![
            CandidateAction::ShootNormal,
            CandidateAction::SafePass,
            CandidateAction::TakeOn,
            CandidateAction::Jockey,
            CandidateAction::StandingTackle,
            CandidateAction::Hold,
        ];

        for action in actions {
            let intent = candidate_to_intent(action);
            // 매핑이 실패하지 않음
            println!("{:?} -> {:?}", action, intent);
        }
    }

    #[test]
    fn test_different_mindsets_produce_different_candidates() {
        let ctx = default_ctx();

        let attack_candidates = filter_candidates_by_mindset(PlayerMindset::AttackScore, &ctx);
        let defend_candidates = filter_candidates_by_mindset(PlayerMindset::DefendContain, &ctx);

        // 공격과 수비 후보가 다름
        assert!(attack_candidates.contains(&CandidateAction::ShootNormal));
        assert!(!defend_candidates.contains(&CandidateAction::ShootNormal));

        assert!(defend_candidates.contains(&CandidateAction::Jockey));
        assert!(!attack_candidates.contains(&CandidateAction::Jockey));
    }

    #[test]
    fn test_sticky_actions_bias_applied() {
        let candidates = vec![
            (CandidateAction::CarryBall, crate::engine::weights::WeightBreakdown::neutral()),
            (CandidateAction::ClosingDown, crate::engine::weights::WeightBreakdown::neutral()),
        ];

        let mut dribble_ctx = default_ctx();
        dribble_ctx.has_ball = true;
        dribble_ctx.is_defending = false;
        dribble_ctx.sticky_actions = StickyActions { sprint: false, dribble: true, press: false };

        let dribble_out = apply_soft_gates(&candidates, &dribble_ctx);
        let dribble_weight = dribble_out
            .iter()
            .find(|(c, _)| *c == CandidateAction::CarryBall)
            .map(|(_, bd)| bd.context)
            .unwrap_or(1.0);
        assert!(dribble_weight > 1.0);

        let mut press_ctx = default_ctx();
        press_ctx.has_ball = false;
        press_ctx.is_defending = true;
        press_ctx.sticky_actions = StickyActions { sprint: false, dribble: false, press: true };

        let press_out = apply_soft_gates(&candidates, &press_ctx);
        let press_weight = press_out
            .iter()
            .find(|(c, _)| *c == CandidateAction::ClosingDown)
            .map(|(_, bd)| bd.context)
            .unwrap_or(1.0);
        assert!(press_weight > 1.0);
    }

    // ========================================
    // Gate C Tests
    // ========================================

    fn default_elab_ctx() -> ElaborationContext {
        ElaborationContext {
            goal_pos: (field::LENGTH_M, field::CENTER_Y),
            defense_goal_pos: (0.0, field::CENTER_Y), // FIX_2601: 수비 골대 위치 추가
            ball_pos: (60.0, field::CENTER_Y),
            player_pos: (60.0, field::CENTER_Y),
            ball_carrier_pos: Some((55.0, 30.0)),
            gk_pos: Some((field::LENGTH_M, field::CENTER_Y)),
            pass_targets: vec![
                (5, (70.0, 30.0), 0.7),
                (7, (80.0, 40.0), 0.6),
                (9, (90.0, field::CENTER_Y), 0.5),
            ],
            shot_zones: [
                (field::LENGTH_M, 30.0), // near post
                (field::LENGTH_M, field::CENTER_Y), // center
                (field::LENGTH_M, 38.0), // far post
            ],
            nearby_opponents: Vec::new(),
            recent_passer_idx: None, // FIX_2601/1130
        }
    }

    #[test]
    fn test_elaborate_score_normal() {
        let elab_ctx = default_elab_ctx();
        let decision_ctx = default_ctx();

        let action =
            elaborate_intent(SelectedIntent::Score(ScoreStyle::Normal), &decision_ctx, &elab_ctx);

        assert_eq!(action.action_type, FinalActionType::Shot);
        assert!(action.target_pos.is_some());
        assert!(action.power > 0.5);

        // Shot params 확인
        if let FinalActionParams::Shot(params) = action.params {
            assert_eq!(params.technique, ShotTechnique::Normal);
        } else {
            panic!("Expected Shot params");
        }
    }

    #[test]
    fn test_elaborate_score_finesse() {
        let elab_ctx = default_elab_ctx();
        let decision_ctx = default_ctx();

        let action =
            elaborate_intent(SelectedIntent::Score(ScoreStyle::Finesse), &decision_ctx, &elab_ctx);

        assert_eq!(action.action_type, FinalActionType::Shot);
        assert!(action.curve > 0.0, "Finesse shot should have curve");

        if let FinalActionParams::Shot(params) = action.params {
            assert_eq!(params.technique, ShotTechnique::Finesse);
        } else {
            panic!("Expected Shot params");
        }
    }

    #[test]
    fn test_elaborate_progress_safe() {
        let elab_ctx = default_elab_ctx();
        let decision_ctx = default_ctx();

        let action = elaborate_intent(
            SelectedIntent::Progress(ProgressStyle::Safe),
            &decision_ctx,
            &elab_ctx,
        );

        assert_eq!(action.action_type, FinalActionType::Pass);
        assert!(action.target_player.is_some());

        if let FinalActionParams::Pass(params) = action.params {
            assert_eq!(params.pass_type, PassType::Ground);
            assert!(!params.is_lofted);
        } else {
            panic!("Expected Pass params");
        }
    }

    #[test]
    fn test_elaborate_beat_takeon() {
        let elab_ctx = default_elab_ctx();
        let decision_ctx = default_ctx();

        let action =
            elaborate_intent(SelectedIntent::Beat(BeatStyle::TakeOn), &decision_ctx, &elab_ctx);

        assert_eq!(action.action_type, FinalActionType::Dribble);

        if let FinalActionParams::Dribble(params) = action.params {
            assert!(params.is_skill_move);
        } else {
            panic!("Expected Dribble params");
        }
    }

    #[test]
    fn test_elaborate_contain_jockey() {
        let elab_ctx = default_elab_ctx();
        let decision_ctx = default_ctx();

        let action = elaborate_intent(
            SelectedIntent::Contain(ContainTechnique::Jockey),
            &decision_ctx,
            &elab_ctx,
        );

        assert_eq!(action.action_type, FinalActionType::Movement);

        if let FinalActionParams::Movement(params) = action.params {
            assert_eq!(params.movement_type, MovementType::Jockey);
        } else {
            panic!("Expected Movement params");
        }
    }

    #[test]
    fn test_elaborate_challenge_sliding() {
        let elab_ctx = default_elab_ctx();
        let decision_ctx = default_ctx();

        let action = elaborate_intent(
            SelectedIntent::Challenge(ChallengeTechnique::SlidingTackle),
            &decision_ctx,
            &elab_ctx,
        );

        assert_eq!(action.action_type, FinalActionType::Tackle);

        if let FinalActionParams::Tackle(params) = action.params {
            assert_eq!(params.tackle_type, TackleType::Sliding);
            assert_eq!(params.commit_level, 1.0); // 최대 커밋
        } else {
            panic!("Expected Tackle params");
        }
    }

    #[test]
    fn test_decide_action_full_pipeline() {
        let mut rng = test_rng();
        let decision_ctx = DecisionContext { xg: 0.25, distance_to_goal: 20.0, ..default_ctx() };
        let elab_ctx = default_elab_ctx();
        let bias = CognitiveBias::default();

        let (action, intent, utility) = decide_action(
            PlayerMindset::AttackScore,
            &decision_ctx,
            &elab_ctx,
            &bias,
            0.6,
            0.7,
            0.6,
            &mut rng,
        );

        // 전체 파이프라인이 유효한 결과 반환
        assert!(utility > -1.0 && utility < 1.0);
        println!("Intent: {:?}, Action: {:?}, Utility: {:.3}", intent, action.action_type, utility);

        // AttackScore mindset이므로 Score 관련 액션 가능성 높음
        assert!(matches!(
            action.action_type,
            FinalActionType::Shot | FinalActionType::Pass | FinalActionType::Dribble
        ));
    }

    #[test]
    fn test_all_intents_can_elaborate() {
        let decision_ctx = default_ctx();
        let elab_ctx = default_elab_ctx();

        // 모든 Intent가 FinalAction으로 변환 가능한지 확인
        let intents = vec![
            SelectedIntent::Protect(ProtectStyle::Shield),
            SelectedIntent::Progress(ProgressStyle::Safe),
            SelectedIntent::Beat(BeatStyle::TakeOn),
            SelectedIntent::Score(ScoreStyle::Normal),
            SelectedIntent::Contain(ContainTechnique::Jockey),
            SelectedIntent::Press(PressTechnique::ClosingDown),
            SelectedIntent::Challenge(ChallengeTechnique::StandingTackle),
            SelectedIntent::CounterAttack,
            SelectedIntent::Recovery,
            SelectedIntent::Clear,
            SelectedIntent::Wait,
        ];

        for intent in intents {
            let action = elaborate_intent(intent, &decision_ctx, &elab_ctx);
            println!("{:?} -> {:?}", intent, action.action_type);
            // 어떤 Intent든 유효한 ActionType을 가져야 함
            assert!(matches!(
                action.action_type,
                FinalActionType::Shot
                    | FinalActionType::Pass
                    | FinalActionType::Dribble
                    | FinalActionType::Tackle
                    | FinalActionType::Movement
                    | FinalActionType::Clear
                    | FinalActionType::Hold
                    | FinalActionType::Block
                    | FinalActionType::Cross
            ));
        }
    }

    // ============================================================================
    // 0108: Decision Intent Tests (Phase 2)
    // ============================================================================

    #[test]
    fn test_build_decision_intent() {
        let ctx = default_ctx();

        // Simulate Gate B results
        let default_facts = CandidateFacts::default();
        let results = vec![
            (
                CandidateAction::SafePass,
                UtilityResult {
                    utility: 2.0,
                    p_hat: 0.8,
                    v_win_hat: 0.5,
                    v_lose_hat: 0.2,
                    facts: default_facts,
                },
            ),
            (
                CandidateAction::TakeOn,
                UtilityResult {
                    utility: 1.5,
                    p_hat: 0.6,
                    v_win_hat: 0.4,
                    v_lose_hat: 0.3,
                    facts: default_facts,
                },
            ),
            (
                CandidateAction::ShootNormal,
                UtilityResult {
                    utility: 0.5,
                    p_hat: 0.2,
                    v_win_hat: 0.8,
                    v_lose_hat: 0.5,
                    facts: default_facts,
                },
            ),
        ];

        let intent = build_decision_intent(
            7,   // player_id
            100, // tick
            CandidateAction::SafePass,
            2.0, // utility
            &results,
            0.25, // temperature
            &ctx,
        );

        assert_eq!(intent.player_id, 7);
        assert_eq!(intent.tick, 100);
        assert_eq!(intent.chosen_action, "SafePass");
        assert!(intent.confidence > 0.0 && intent.confidence <= 1.0);
        assert!(intent.alternatives.len() <= 5);

        // SafePass should have highest confidence
        for alt in &intent.alternatives {
            assert!(alt.probability < intent.confidence);
        }
    }

    #[test]
    fn test_softmax_probabilities() {
        let utilities = vec![1.0, 2.0, 3.0];
        let probs = compute_softmax_probabilities(&utilities, 1.0);

        assert_eq!(probs.len(), 3);
        // Higher utility = higher probability
        assert!(probs[2] > probs[1]);
        assert!(probs[1] > probs[0]);
        // Sum should be ~1.0
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }
}
