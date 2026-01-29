pub mod action_detail; // NEW: P16 - ActionDetail (리팩토링 없이 확장 가능한 액션 세부 파라미터)
pub mod metrics; // FIX_2601 Phase 5: MatchMetrics (SSOT-compliant unified metrics container)
pub mod action_evaluator; // NEW: FIX_2601/0108 - Unified Action Evaluator (6-factor scoring)
pub mod action_metadata; // NEW: FIX_2601 Phase 3 - ActionMetadata (Google Football CoreAction style)
pub mod action_queue; // NEW: Phase 3.1 - Action scheduling system
pub mod action_scoring; // NEW: Phase 1.0.12 - ACTION_SCORING_SSOT core functions
pub mod action_scoring_types; // NEW: Phase 1.0.12 - YAML structures
pub mod actions;
pub mod actor_state_validator; // NEW: P2.1-B - Actor State FSM Validation
pub mod audit_gates; // NEW: FIX_2512 Phase 0 - Audit Gates (coordinate/state/ID validation)
pub mod ball;
pub mod ball_physics_params; // NEW: FIX_2601/0109 - Ball physics params SSOT for contract path
pub mod behavior_intent; // NEW: FIX_2601 - BehaviorIntent system (Google Football patterns)
pub mod ball_flight_resolver; // NEW: FIX_2601/0106 - Physics-based shot resolution (Level 1)
pub mod ball_prediction; // NEW: FIX_2601/0112 - Ball Prediction System (Google Football style)
pub mod body_blocking; // NEW: P7 Phase 6 - Body Blocking & Physical Interference
pub mod body_orientation; // NEW: Phase 1.1 - Body Orientation Model
pub mod config; // NEW: P10-13 Phase 6 - Tuning Configuration
pub mod coordinate_contract; // FIX_2601: Coordinate contract SSOT
pub mod coordinates;
pub mod debug_logger; // NEW: P10-13 Phase 7 - Debug Visualizer
pub mod debug_flags; // Debug output gating (env-based)
pub mod decision_scheduler; // FIX_2601/0113 - Decision Priority Queue (DPQ) skeleton
pub mod defensive_positioning; // NEW: P7 Phase 7 - Defensive Positioning System
pub mod duel; // NEW: P3 Phase 3 - 1:1 Duel System (Take-on, Defender's Dilemma)
pub mod dsa_summary; // NEW: FIX_2601/0114 - DSA v1.1 authoritative post-match summary (telemetry)
pub mod elastic_band; // NEW: Elastic Band Theory - Relative Coordinate Positioning
pub mod events;
pub mod execution_error; // NEW: P10-13 Phase 2 - Execution Error System
pub mod experimental; // NEW: DPER Framework - Experimental Configuration
pub mod field_board; // NEW: P18 - FieldBoard (A-Plan Board Layer)
pub mod force_field; // NEW: FIX_2601/0112 - Force Field Navigation (Google Football style)
pub mod formation_waypoints;
pub mod goal; // NEW: P0 - Goal Contract (축구 세계의 헌법)
pub mod growth; // NEW: Phase 5 - Hero Growth System
pub mod intent_arbiter; // FIX_2601/0117 - Intent conflict resolution (Arbiter)
pub mod intent_log; // NEW: FIX_2601 - Intent logging for CI gates and analysis
pub mod live_match;
pub mod marking_manager; // NEW: Phase 1.3 - MarkingManager (Budget Enforcement)
pub mod match_analysis; // NEW: Match OS v1.2 Priority 5 - Post-Match Pattern Detection
pub mod interpretation_v1; // FIX_2601/0115 - Replay/Analytics Interpretation Layer v1 (post-match)
pub mod match_modifiers; // NEW: FIX_2601/0109 - Sparse scalar modifiers (deck/coach effects)
pub mod match_sim;
pub mod mindset; // NEW: P14 - Player Mindset System
pub mod movement;
pub mod observation; // FIX_2601 Phase 4: SSOT-compliant Observation Builders
pub mod offball; // FIX_2601/0115 - Off-Ball Decision System v1
pub mod opponent_analysis;
pub mod pep_grid; // NEW: Phase 3.4 - 5-channel positioning
pub mod phase_action; // NEW: P7 - Phase-Based Action System
pub mod reward; // NEW: FIX_2601 - RewardFunction (Google Football style AI training)
pub mod physics_constants;
pub mod plan_builder; // NEW: Phase 1.0.5 - build_plan_window() (prepared for full integration)
pub mod plan_window; // NEW: Phase 1.0.2 - PlanWindow structure
pub mod player_attributes; // NEW: Extracted from match_sim.rs
pub mod player_decision;
pub mod player_objective; // NEW: Phase 3.0 - Player objectives
pub mod player_motion_params; // FIX_2601/0109 - Ability→MotionParams SSOT
pub mod player_physics; // NEW: P15 - Player Inertia Physics System
pub mod player_state; // NEW: P7 - Player State Machine
pub mod positioning;
pub mod positioning_engine; // NEW: Phase 3.4 - Off-the-Ball movement
pub mod probability;
pub mod scenario_builder; // NEW: FIX_2601 - ScenarioBuilder (Google Football style declarative API)
pub mod scenario_loader; // FIX_2601/0106 - Scenario test harness
pub mod set_pieces;
pub mod snapshot; // State Snapshot API for checkpoint/restore
pub mod sort_keys; // FIX_2601/0123 PR #9-1: Stable sort tie-breaker keys
pub mod stats;
pub mod steering; // P3a: Steering behaviors (seek, arrive, pursuit, separation)
pub mod substep_runner; // NEW: Phase 1.0.5 - exec_substep() (prepared for full integration)
pub mod substitutions; // NEW: Extracted from match_sim.rs
pub mod tactical_brain;
pub mod tactical_context;
pub mod team_phase; // NEW: Phase 3.0 - Team phase state machine
pub mod threat_model; // NEW: Phase 1.2 - ThreatModel (CarrierFreeScore)
pub mod tick_snapshot; // FIX_2601/0117 - Snapshot-based 2-Phase decision system
pub mod timestep; // NEW: Phase 1.0.1 - Dual timestep constants
pub mod trace_dump; // FIX_2601/0106 - Trace dump utilities
pub mod transition_system; // NEW: Phase 1.4 - TransitionSystem (3s possession-change window)
pub mod types;
pub mod weights;
pub mod xgzone_map; // NEW: Match OS v1.2 - XGZone Map (Spatial xG Awareness)

pub use action_queue::{
    execute_dribble,
    execute_intercept,
    execute_move,
    execute_pass,
    execute_save,
    execute_shot,
    execute_tackle,
    execute_trap,
    schedule_followup,
    ActionMeta,
    // P0: Core Phase types moved from phase_action::types
    ActionPhase,
    ActionQueue,
    ActionResult,
    ActionType,
    ActiveAction,
    // P0: Viewer event types moved from phase_action::types
    BallIntentKind,
    BallState,
    BallTrajectoryIntent,
    CurveDirection,
    DribbleTouchEvent,
    DribbleTouchType,
    ExecutionContext,
    HeightClass,
    InterruptReason,
    LooseBallContest, // Phase 3.3
    PassType,
    PhaseActionType,
    PlayerStats,
    RestartType,
    SaveType,
    ScheduledAction,
    ShotType,
    SpeedClass,
    TackleActionKind,
    TackleEvent,
    TackleOutcome,
    TackleType,
    TakeOnEvent,
    TakeOnOutcome,
    ViewerEvent,
    ViewerTackleOutcome,
    DEFAULT_SPRITE_FPS,
};
pub use action_scoring::{
    apply_situational,
    compute_score,
    compute_score_combo,
    error_scale,
    map_peak,
    mix_quality,
    // Core functions
    normalize,
    prob_link,
}; // Phase 1.0.12: ACTION_SCORING_SSOT core functions
pub use action_scoring_types::{
    // Functions
    load_action_scoring_ssot,
    // Types
    ActionScoringSSOT,
    ActionSpec,
    ErrorLinkSpec,
    PeakRange,
    ProbLinkSpec,
    ScoreComboSpec,
    ScoreSpec,
    StatScale,
}; // Phase 1.0.12: ACTION_SCORING_SSOT types
pub use actor_state_validator::{ActorState, ActorStateValidator}; // P2.1-B: Actor State FSM Validation
pub use ball::{Ball, CurveLevel, HeightProfile};
pub use ball_flight_resolver::{
    FlightConfig, FlightShotResult, IntersectionHit, PitchSpec, ShotAttempt, ShotResolved,
}; // FIX_2601/0106: Physics-based shot resolution
pub use ball_prediction::BallPrediction; // FIX_2601/0112: Ball Prediction System
pub use behavior_intent::{
    allowed_intents, is_allowed, is_forbidden, BehaviorIntent, IntentCategory,
}; // FIX_2601: BehaviorIntent system (Google Football patterns)
pub use body_blocking::{
    calculate_approach_angle as body_blocking_approach_angle,
    can_attempt_tackle as body_blocking_can_attempt_tackle,
    check_player_collision,
    // Constants
    constants as body_blocking_constants,
    distance as body_blocking_distance,
    find_blockers,
    find_closest_point_on_line,
    find_interceptors,
    find_shot_blockers,
    is_path_blocked,
    is_path_blocked_except,
    normalize as body_blocking_normalize,
    // Functions
    point_to_line_distance,
    resolve_player_collisions,
    BlockCandidate,
    CollisionInfo,
    InterceptCandidate,
    // Types
    PlayerPhysics,
    TackleAttemptResult as BodyBlockingTackleResult,
    BODY_RADIUS,
    INFLUENCE_RADIUS,
    INTERCEPT_RADIUS,
}; // P7 Phase 6: Body Blocking
pub use config::{AudacityConfig, DecisionConfig, EngineConfig, ExecutionConfig, StaminaConfig}; // P10-13 Phase 6: Tuning Configuration
pub use debug_logger::{
    ActionEvaluation, DebugLogger, DecisionContext, DecisionLog, EvaluationBreakdown, ExecutionLog,
}; // P10-13 Phase 7: Debug Visualizer
pub use intent_log::{
    calculate_intent_by_state, calculate_intent_by_team, calculate_intent_distribution,
    new_shared_logger, validate_phase_intent_consistency, IntentLogEntry, IntentLogger, PitchZone,
    SharedIntentLogger,
}; // FIX_2601: Intent logging for CI gates
pub use defensive_positioning::{
    apply_team_slide,
    // Functions
    assign_defensive_roles,
    calculate_team_slide,
    gk_constants,
    movement_constants,
    // Constants
    presser_constants,
    should_swap_presser,
    slide_constants,
    swap_presser_roles,
    update_cover_movement,
    update_defensive_positioning,
    update_goalkeeper_position,
    update_marker_movement,
    DefensiveLine,
    DefensiveRole,
    PresserEvent,
    PresserMovement,
    PresserPhase,
    // Types (TeamSide is aliased to avoid conflict with tactical_context::TeamSide)
    TeamSide as DefensiveTeamSide,
}; // P7 Phase 7: Defensive Positioning
pub use duel::{
    apply_pressure_with_composure,
    calculate_accuracy_penalty,
    calculate_block_chance,
    calculate_evasive_trajectory,
    calculate_power_penalty,
    // Passive Interference System
    calculate_shot_pressure,
    calculate_xg_with_pressure,
    decide_defensive_action,
    is_in_shot_cone,
    resolve_duel,
    AttackerAction,
    DefensiveAction,
    DuelOutcome,
    DuelPhase,
    DuelState,
    TackleDuelOutcome,
}; // P3 Phase 3: 1:1 Duel System + Passive Interference
pub use elastic_band::{
    calculate_cover_shift,
    calculate_elastic_snapback,
    calculate_line_heights,
    calculate_player_target,
    calculate_role_offset,
    // Functions
    calculate_team_center,
    calculate_team_shift,
    // Constants
    constants as elastic_band_constants,
    find_threats_in_zone,
    update_team_positioning_state,
    ElasticTactics,
    FormationOffset,
    // Types
    PositionLine,
    RoleInstruction,
    TeamPositioningState,
    ThreatInfo,
}; // Elastic Band Theory: Relative Coordinate Positioning
pub use events::EventGenerator;
pub use execution_error::{
    apply_error_for_first_touch, apply_error_for_shot, apply_error_to_target, is_weak_foot,
    sample_execution_error, ActionKind, ErrorContext, ExecutionError, FirstTouchQuality,
    PreferredFoot, Side,
}; // P10-13 Phase 2: Execution Error
pub use experimental::{
    AudacityParams, DecisionParams, DiffReport, ExpConfig, ExpConfigError, MatchStats,
    RuntimeExpParams, StatsDelta, StyleParams,
}; // DPER Framework: Experimental Configuration
pub use field_board::{
    BoardSummaryExport,
    CellIndex,
    // Types
    FieldBoard,
    FieldBoardSpec,
    FieldBounds, // P2.1-A: Coordinate Bounds Validation
    HeatmapF32,
    HotCellExport,
    NeighborMode,
    OccupancyCell,
    // Constants
    FIELD_LENGTH_M,
    FIELD_WIDTH_M,
}; // P18: FieldBoard (A-Plan Board Layer)
pub use force_field::{calculate_dribble_direction, DecayType, ForceSpot, ForceType}; // FIX_2601/0112: Force Field Navigation
pub use formation_waypoints::get_formation_waypoints;
pub use goal::{Goal, Goals}; // P0: Goal Contract
pub use growth::{
    calculate_dribble_difficulty, calculate_pass_difficulty, calculate_pressure, calculate_xp,
    growth_threshold, HeroActionTag, HeroMatchGrowth, HeroXpBucket, HeroXpEvent, PlayerAttribute,
}; // Phase 5: Hero Growth
pub use live_match::{
    FullTimeData, HalfTimeData, LiveMatchSession, MatchState, PlayerPosition, StepResult, TickData,
};
pub use match_analysis::{
    // Functions
    analyze_match,
    AttackZone,
    AttackZoneAnalysis,
    DangerMoment,
    // Types
    MatchAnalysisReport,
    PossessionShift,
    PressurePeriod,
}; // Match OS v1.2 Priority 5: Post-Match Pattern Detection
pub use dsa_summary::{
    DsaHubPlayer,
    DsaHubSummary,
    DsaMinuteSeries,
    DsaQaWarning,
    DsaQaWarningKind,
    DsaRoute,
    DsaRouteSummary,
    DsaSummary,
}; // FIX_2601/0114: Distributed Sensing Analytics (DSA) v1.1 summary
pub use match_sim::{
    MatchEngine, MatchPlan, MiniMapObservation, MiniMapSpec, SimpleVectorObservation,
    TeamViewBallObservation, TeamViewPlayerObservation,
};
pub use match_modifiers::TeamMatchModifiers;
pub use mindset::{
    build_candidates,
    // Functions
    determine_player_mindset,
    CandidateAction,
    CandidateContext,
    CandidateGroup,
    MindsetContext,
    // Types
    PlayerMindset,
    BLOCK_SHOT_RANGE,
    CONTAIN_RANGE,
    HOLD_WIDTH_TOUCHLINE_THRESHOLD,
    MARK_RANGE,
    PENETRATE_LINE_THRESHOLD,
    // Constants
    PRESSER_RANGE,
    REST_DEFENSE_THRESHOLD,
    TRANSITION_DURATION_TICKS,
}; // P14: Player Mindset System
pub use movement::{
    calculate_offensive_target_with_buildup, get_fallback_position, get_position_role,
    slot_to_position_key, BuildupContext, PositionRole,
};
pub use offball::{
    update_offball_decisions, GamePhase as OffBallGamePhase, OffBallConfig, OffBallContext,
    OffBallIntent, OffBallObjective, Score6 as OffBallScore6, Urgency as OffBallUrgency,
}; // FIX_2601/0115: Off-Ball Decision System v1
pub use opponent_analysis::{CounterTactic, OpponentAnalysis, Weakness};
pub use pep_grid::{Channel, GridCell, PepGrid, ZoneDepth}; // Phase 3.4
pub use phase_action::{
    calculate_approach_angle,
    calculate_pass_difficulty as p7_calculate_pass_difficulty, // Alias to avoid conflict with growth module
    calculate_save_probability,                                // P7 Phase 5: Shot
    calculate_xg,
    calculate_xg_with_target,
    can_attempt_tackle,
    can_start_dribble,
    should_trigger_evade,
    BallPhysics,
    BallPhysicsState,
    DribbleAction,
    DribblePhase,
    DribbleResult,
    HeightCurve,
    PassAction,
    PassPhase,
    PassResult,                   // P7 Phase 5: Pass
    RestartType as P7RestartType, // Alias to avoid conflict with action_queue::RestartType
    ShotAction,
    ShotPhase,
    ShotResult,
    TackleAction,
    TackleAttemptResult,
    TacklePhase,
}; // P7: Phase-Based Action System
pub use plan_window::{
    BallPlan,
    DebugSlice,
    // Types
    PlanWindow,
    PlannedEvent,
    PlayerPlan,
    PlayerPlanKind,
};
pub use player_decision::{PlayerAction, PlayerDecision};
pub use player_objective::{assign_objective, ObjectiveContext, PlayerObjective};
pub use player_motion_params::{ability_to_motion_params, scale_by_stamina, PlayerMotionParams};
pub use player_physics::{
    calc_turn_severity,
    calculate_arrival_speed,
    calculate_slowing_radius,
    calculate_turn_penalty,
    // Functions
    update_player_motion,
}; // P15: Player Inertia Physics System
pub use player_state::{default_player_states, tick_update_all, PlayerState, PlayerStates}; // P7: Player State
pub use positioning::{PositionKey, PositionWaypoints};
pub use positioning_engine::{
    PlayerPositioningState, PositioningConfig, PositioningEngine, PositioningRole,
}; // Phase 3.4
pub use snapshot::{ActionQueueSnapshot, MatchStateSnapshot, SnapshotError}; // State Snapshot API
pub use stats::StatsCalculator;
pub use tactical_brain::{DefensiveGoal, OffensiveGoal};
pub use tactical_context::{TacticalContext, TacticalEventType, TeamSide};
pub use team_phase::{TeamPhase, TeamPhaseState};
pub use timestep::{DECISION_DT, SUBSTEPS_PER_DECISION, SUBSTEP_DT}; // Phase 1.0.1: Dual timestep constants
pub use trace_dump::TraceDump; // FIX_2601/0106 - Trace dump output
pub use transition_system::{TransitionState, TransitionSystem, TRANSITION_WINDOW_MS};
pub use scenario_builder::{
    MatchScenarioBuilder, ScenarioBuilder, ScenarioError,
    // Coordinate conversion utilities
    normalize_to_meters, meters_to_normalize,
    NORM_X_MIN, NORM_X_MAX, NORM_Y_MIN, NORM_Y_MAX,
    METERS_X_MIN, METERS_X_MAX, METERS_Y_MIN, METERS_Y_MAX,
}; // FIX_2601: ScenarioBuilder (Google Football style)
pub use reward::{
    RewardFunction, SparseGoalReward, CheckpointReward, CompositeReward,
    // Helper functions
    normalized_distance_to_goal, is_our_ball,
}; // FIX_2601: RewardFunction (Google Football style)
pub use observation::{
    // Trait
    ObservationBuilder,
    // SimpleVector (115-float vector, simple115_v2 style) - SSOT versions
    SimpleVectorBuilder,
    SimpleVectorObservation as SsotSimpleVectorObs,
    TeamViewBallObs, TeamViewPlayerObs,
    // MiniMap (4-channel spatial planes, SMM style) - SSOT versions
    MiniMapBuilder,
    MiniMapObservation as SsotMiniMapObs,
    MiniMapSpec as SsotMiniMapSpec,
}; // FIX_2601 Phase 4: SSOT-compliant Observation Builders (from TickSnapshot only)
pub use action_metadata::{
    ActionMetadata, ActionCategory,
    // Release action utilities
    get_release_action, get_release_intent,
    is_sticky_action, is_sticky_intent,
}; // FIX_2601 Phase 3: ActionMetadata (Google Football CoreAction style)
pub use types::{
    ActionOptions,
    BallZone,
    Coord10,
    GameState,
    LineBattleResult,
    PassTarget,
    PlayerReactionState,
    ReactionState,
    SimState,
    ThroughBallResult,
    UserAction,
    UserDecisionContext,
    Vel10, // FIX_2512 Phase 1: Coord10 좌표 시스템
}; // Phase 1.0.2-1.0.4: PlanWindow structures (HeightCurve already exported from phase_action)
pub use metrics::{
    MatchMetrics, MetricsMetadata, ProductStatsSummary,
    compute_all_metrics, compute_qa_from_result,
}; // FIX_2601 Phase 5: MatchMetrics (SSOT-compliant unified metrics container)

use crate::models::{EventType, MatchEvent};

// User player highlight configuration
#[derive(Debug, Clone)]
pub struct UserPlayerConfig {
    pub is_home_team: bool,
    pub player_name: String, // C7: Will be removed
    pub player_index: usize, // C6: Engine-confirmed track_id (0-21)
    pub highlight_level: HighlightLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum HighlightLevel {
    Skip,     // 스킵 - 바로 결과로 (이벤트 0개)
    Simple,   // 간단히 - 골 + 주요 장면만
    MyPlayer, // 내 선수 - 내 선수 활약 + 골 + 주요 장면
    Full,     // 전체 - 모든 하이라이트
}

impl HighlightLevel {
    pub fn min_events(&self) -> usize {
        match self {
            HighlightLevel::Skip => 0,      // 이벤트 없음
            HighlightLevel::Simple => 3,    // 골 + 카드 정도
            HighlightLevel::MyPlayer => 10, // 내 선수 중심
            HighlightLevel::Full => 30,     // 전체 하이라이트
        }
    }

    pub fn max_events(&self) -> usize {
        match self {
            HighlightLevel::Skip => 0,      // 이벤트 없음
            HighlightLevel::Simple => 10,   // 주요 장면만
            HighlightLevel::MyPlayer => 25, // 내 선수 + 중요 이벤트
            HighlightLevel::Full => 60,     // 모든 것
        }
    }

    pub fn user_weight_multiplier(&self) -> f32 {
        match self {
            HighlightLevel::Skip => 0.0,
            HighlightLevel::Simple => 1.0,
            HighlightLevel::MyPlayer => 3.0, // 내 선수 강조
            HighlightLevel::Full => 1.5,
        }
    }

    /// 이벤트 타입 + 주인공 관여 여부(track_id)를 기반으로
    /// 현재 HighlightLevel에서 이 이벤트를 노출할지 여부를 판단한다.
    pub fn allows(&self, event: &MatchEvent, my_player_track_id: Option<u8>) -> bool {
        use EventType::*;

        let is_my_player = my_player_track_id.is_some_and(|track_id| {
            event.player_track_id == Some(track_id) || event.target_track_id == Some(track_id)
        });

        // C7 TODO: Re-implement player matching using track_id:
        // let is_my_player = my_player_track_id.map_or(false, |track_id| {
        //     event.player_track_id == Some(track_id) ||
        //     event.target_track_id == Some(track_id)
        // });

        match self {
            HighlightLevel::Skip => false,
            HighlightLevel::Full => true,
            HighlightLevel::Simple => match event.event_type {
                // 골 관련/직접적인 스코어링 장면
                Goal | Penalty => true,
                // 주요 세트피스/찬스
                Corner | Freekick | KeyChance => true,
                // 카드/부상
                RedCard | YellowCard | Injury => true,
                _ => false,
            },
            HighlightLevel::MyPlayer => {
                if is_my_player {
                    // 주인공이 연관된 모든 이벤트
                    true
                } else {
                    // 그 외에는 Simple 기준만 따른다
                    HighlightLevel::Simple.allows(event, my_player_track_id)
                }
            }
        }
    }
}
