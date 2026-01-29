//! Match Simulation Engine
//!
//! Core match simulation module for Open Football. This module orchestrates
//! the entire match simulation including:
//!
//! - Match state management (score, time, ball position, player positions)
//! - Turn-based action execution (step function)
//! - Event emission (goals, fouls, cards, substitutions)
//! - Replay data generation
//!
//! ## Architecture
//!
//! The simulation follows a 3-layer architecture:
//! - L1: `probability.rs` - Stateless math/probability functions
//! - L2: `actions.rs` - Pure action resolution (Context → Result)
//! - L3: `match_sim/` - Stateful gameplay with Gold Traits integration
//!
//! ## Data Flow Overview (2025-12-11, Updated for Legacy Cleanup)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         MATCH SIMULATION DATA FLOW                          │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  [INPUT]                                                                    │
//! │    MatchPlan { home_team, away_team, seed, instructions... }                │
//! │         │                                                                   │
//! │         ▼                                                                   │
//! │  ┌─────────────────────────────────────────────────────────────────────┐    │
//! │  │                    MatchEngine (this module)                        │    │
//! │  │                                                                     │    │
//! │  │  Simulation: tick_based.rs ONLY (240 ticks/min = 4 ticks/sec)       │    │
//! │  │  (Legacy minute-based simulation removed 2025-12-11)                │    │
//! │  │                                                                     │    │
//! │  │  Internal State (Normalized Coordinates 0.0-1.0):                   │    │
//! │  │    • ball: Ball { position: (f32, f32), velocity, height }          │    │
//! │  │    • player_positions: Vec<(f32, f32)> [22 players]                 │    │
//! │  │    • game_state: GameState (possession, phase)                      │    │
//! │  │                                                                     │    │
//! │  │  Position Recording:                                                │    │
//! │  │    • track_positions = true → enables position recording            │    │
//! │  │    • Engine records directly via record_positions_for_tick()        │    │
//! │  │      in tick_based.rs at each simulation tick                       │    │
//! │  └─────────────────────────────────────────────────────────────────────┘    │
//! │         │                                                                   │
//! │         ▼                                                                   │
//! │  [OUTPUT: MatchResult]                                                      │
//! │    │                                                                        │
//! │    ├─► score: (u8, u8)              → Final match score                     │
//! │    ├─► events: Vec<MatchEvent>      → Goals, fouls, cards, subs             │
//! │    ├─► stats: MatchStats            → Shots, passes, possession %           │
//! │    ├─► player_stats: HashMap        → Individual player statistics          │
//! │    └─► position_data: Option<MatchPositionData>                             │
//! │              │                                                              │
//! │              │  Contains (Meters: 105m x 68m):                              │
//! │              │    • ball_positions: Vec<BallPosition>                       │
//! │              │        - timestamp_ms, x, y, z, velocity                     │
//! │              │    • player_positions: HashMap<u8, Vec<PlayerPosition>>      │
//! │              │        - timestamp_ms, x, y, state                           │
//! │              │                                                              │
//! │              ▼                                                              │
//! │  ┌─────────────────────────────────────────────────────────────────────┐    │
//! │  │                      match_result.rs                                │    │
//! │  │  • MatchPositionData::add_ball_position_with_velocity()             │    │
//! │  │  • MatchPositionData::add_player_position_with_state()              │    │
//! │  │  • Stores position data in meters (converted from normalized)       │    │
//! │  └─────────────────────────────────────────────────────────────────────┘    │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        EXTERNAL CONSUMERS                                   │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  [GodotExtension] crates/of_godot/                                          │
//! │    • Exposes MatchResult to Godot via GDExtension                           │
//! │    • Provides replay data for visualization                                 │
//! │                                                                             │
//! │  [Viewer] Godot/addons/match_viewer/                                        │
//! │    • Receives position_data from GodotExtension                             │
//! │    • Renders ball/player positions at each timestamp                        │
//! │    • Expects coordinates in METERS (0-105, 0-68)                            │
//! │                                                                             │
//! │  [Server] (if applicable)                                                   │
//! │    • Stores MatchResult in database                                         │
//! │    • WARNING: Check what fields are actually saved!                         │
//! │      Some implementations only save position_data, not events/score         │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Coordinate Systems
//!
//! | Location | Coordinate System | Range |
//! |----------|-------------------|-------|
//! | Engine internal | Normalized | (0.0-1.0, 0.0-1.0) |
//! | MatchPositionData | Meters | (0-105m, 0-68m) |
//! | Godot Viewer | Meters | (0-105m, 0-68m) |
//!
//! Conversion: `normalized_to_meters()` in tick_based.rs
//!   - x_meters = normalized_x * field length (meters)
//!   - y_meters = normalized_y * field width (meters)
//!
//! ## Timing Constants
//!
//! | Constant | Value | Description |
//! |----------|-------|-------------|
//! | TICKS_PER_MINUTE | 240 | 4 ticks/second × 60 seconds |
//! | Tick interval | 250ms | Each tick = 250 milliseconds |
//! | Match duration | 90 min | Standard match (configurable) |
//!
//! ## Sub-modules
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `aerial_duel` | Header/aerial duel resolution |
//! | `ball_helpers` | Ball trajectory helpers (A4) |
//! | `ball_physics` | Ball physics simulation |
//! | `cross_through` | Cross and through ball execution |
//! | `dribbling` | Dribble action execution |
//! | `interception` | Pass interception logic |
//! | `movement` | Player position updates |
//! | `movement_helpers` | Separation and speed helpers |
//! | `offside` | Offside line and trap logic |
//! | `passing` | 6-Factor pass system |
//! | `set_pieces` | Restart helpers (goal kick, throw-in) |
//! | `shooting` | Shot execution and GK saves |
//! | `skill_system` | A13 deception and reaction states |
//! | `tackle` | Tackle, foul, injury system |
//! | `target_position` | 5-Layer target position calculation |
//! | `tick_based` | **Tick-based simulation (240 ticks/min)** - PRIMARY |

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Write};

use super::ball::Ball;
use super::ball_prediction::BallPrediction; // FIX_2601/0106: Ball prediction system
use super::formation_waypoints::get_formation_waypoints;
use super::movement::{get_fallback_position, slot_to_position_key};
use super::team_phase::TeamPhaseState;
use super::types::{DirectionContext, GameState, PlayerReactionState};
use super::{EventGenerator, StatsCalculator};
// P7: Phase-Based Action System imports
use super::player_state::{default_player_states, PlayerState, PlayerStates};
// P0: Goal Contract imports
use super::defensive_positioning::{
    CoverMovement, DefensiveLine, DefensiveRole, DefensiveTuning, MarkerMovement, PresserMovement,
};
// FIX_2601/0106 P4: Match Situation for dynamic tactics
use super::goal::{Goal, Goals};
use super::tactical_context::MatchSituation;
// P9: Set Piece FSM imports (Pass/Shot/Tackle/Dribble FSM 제거됨 2025-12-14)
use super::phase_action::SetPieceAction;
// Elastic Band Theory imports
use super::elastic_band::{ElasticTactics, FormationOffset, PositionLine, TeamPositioningState};
// P18: FieldBoard imports
use super::field_board::{FieldBoard, FieldBoardSpec};
use crate::models::{
    EventType, MatchEvent, MatchResult, MatchSetup, MyPlayerStats, Team, TeamSide,
};
use crate::models::replay::types::DecisionIntent;
use crate::player::instructions::PlayerInstructions;
use crate::replay::converter::ReplayConverter;
use crate::replay::recorder::ReplayRecorder;
use crate::replay::types::{PitchSpec, ReplayDoc, ReplayPlayer, ReplayRoster, ReplayRosters};
use crate::tactics::team_instructions::{BuildUpStyle, TeamInstructions};
// Phase 2: AI Tactical Manager Integration
use crate::tactics::{AIDifficulty, AITacticalManager, MatchState};        
use crate::calibration::{MatchStatSnapshot, CalibratorParams};

// =============================================================================
// FIX_2601/0106: Added Time (Stoppage Time, deterministic; v1.1)
// =============================================================================
const HALF_DURATION_MINUTES: u8 = 45;
const REGULATION_TOTAL_MINUTES: u8 = 90;
const STOPPAGE_TIME_MAX_MINUTES: u8 = 8;
const MATCH_DURATION_CAP_MINUTES: u8 = REGULATION_TOTAL_MINUTES + STOPPAGE_TIME_MAX_MINUTES * 2;

const STOPPAGE_SECONDS_SUBSTITUTION: u16 = 30;
const STOPPAGE_SECONDS_INJURY: u16 = 60;
const STOPPAGE_SECONDS_GOAL: u16 = 30;
const STOPPAGE_SECONDS_CARD: u16 = 15;
const STOPPAGE_SECONDS_VAR_REVIEW: u16 = 60;

mod action_decision;
pub mod action_detail_v2; // FIX_2601/1123: ActionDetailV2 완전성 계약
#[cfg(feature = "detail_v2")]
pub mod conversion_v2; // FIX_2601/1123: V2 Conversion (RNG 없음)
pub mod detail_builder; // FIX_2601/1123: Detail Builder (deterministic)
pub mod deterministic; // FIX_2601/1123: Deterministic 선택 함수
pub mod tactical_bias; // FIX_2601/1124: TacticalBias 시스템
pub mod candidate_key; // FIX_2601/1124: CandidateKey + Gate A 검증
pub mod attack_phase; // FIX_2601/1129: 팀 단위 공격 국면 (AttackPhase)
mod aerial_duel;
mod ball_helpers;
mod ball_physics;
mod calculations;
mod cross_through;

// Re-export pressure types for external use
pub use balance_diagnostics::DiagnosticReport;
pub use calculations::{PressureContext, PressureLevel};
pub use shot_opportunity::{ShotOppTelemetry, ShotOpportunityFrame}; // FIX_2601: Shot Opportunity Telemetry

// FIX_2601/1123: ActionDetailV2 re-exports
pub use action_detail_v2::{
    ActionDetailV2, ClearanceDetail, CrossDetail, CrossKind, DribbleDetail, HeaderDetail,
    HeaderTarget, HoldDetail, InterceptDetail, PassDetail, PassKind, ShotDetail, ShotKind,
    TackleDetail, TackleKind,
    // Intent types for builders
    ClearanceIntent, CrossIntent, DribbleIntent, HeaderIntent, HeaderTargetType, PassIntent,
    ShotIntent, TackleIntent,
};

// FIX_2601/1123: Deterministic functions and builders
pub use deterministic::{
    deterministic_bool, deterministic_choice, deterministic_f32, deterministic_f64,
    normalize_direction, subcase,
};
pub use detail_builder::{
    build_clearance_detail, build_cross_detail, build_dribble_detail, build_header_detail,
    build_hold_detail, build_intercept_detail, build_pass_detail, build_shot_detail,
    build_tackle_detail, DecisionContext,
};

// FIX_2601/1124: TacticalBias and CandidateKey re-exports
pub use tactical_bias::TacticalBias;
pub use candidate_key::{
    CandidateKey, DribbleChannel, DribbleKey, GateAResult, HeaderKey, PassKey, ShotBucket,
    ShotKey, SpeedBucket, TackleKey, CrossKey, validate_gate_a,
};

// FIX_2601/1129: AttackPhase re-exports
pub use attack_phase::{
    AttackPhase, TeamAttackState, PhaseTransitionContext, determine_phase,
    transition_constants,
};

// FIX_2601/1123: V2 Conversion (only with detail_v2 feature)
#[cfg(feature = "detail_v2")]
pub use conversion_v2::{
    convert_detail_v2_to_action_type,
    // Intent extractors
    extract_clearance_intent, extract_cross_intent, extract_dribble_intent,
    extract_header_intent, extract_pass_intent, extract_shot_intent, extract_tackle_intent,
};
pub use observation::{
    MiniMapObservation, MiniMapSpec, SimpleVectorObservation, TeamViewBallObservation,
    TeamViewPlayerObservation,
};
pub use sticky_actions::{StickyAction, StickyActions};
// FIX_2601/0123: Match State Machine exports
pub use match_state::{
    CornerSide, FreeKickType, GameFlowMachine, GameFlowState, MatchPlayerId, MatchPosition,
    MatchTime, PenaltyPhase, TeamId, TransitionAction, TransitionGuard, TransitionTrigger,
    VarReviewType,
};
pub mod attribute_calc; // FIX_2601/0109: Made public for unified GK save calculation
mod audacity; // P10-13 Phase 4: Audacity/Flair system
mod balance_diagnostics;
mod dribbling;
mod ev_decision; // P10-13 Phase 3: EV-based decision system
mod helpers;
mod interactive_session;
mod interception;
mod movement;
mod movement_helpers;
mod observation;
mod offside;
mod p7_test; // P7: Match simulation test
mod passing;
mod pitch_zone;
mod player_selection;
pub mod quality_metrics;
mod set_pieces;
mod shooting;
pub mod shot_opportunity; // FIX_2601: Shot Opportunity Telemetry System
mod simulation_logic;
mod skill_system;
mod state_accessors;
mod sticky_actions;
mod tackle;
mod target_position;
mod tick_based;
mod zone_transition; // Phase 3.5: Tick-based simulation // Attribute-based calculation (Context × Attribute)

// FIX_2601/0123: Match State Machine (Dead ball handling consistency)
pub mod match_state;

// FIX_2601/0123: Centralized Rule Dispatcher (Team bias reduction)
pub mod rules;

// FIX_2601/0123: Team Momentum System (leadership integration)
pub mod momentum;

// FIX_2601/0110: Dynamic Build-up System
pub mod attacking_runs; // Phase 3: Late box runs and attacking run types
pub mod buildup_phase; // FIX_2601/0106 P3: Field position-based tactical phases
pub mod channel_finder; // Phase 1: Dynamic channel identification
pub mod congestion; // Phase 4: Real-time congestion calculation
pub mod passing_triangles; // Phase 2: Passing triangle formation

// FIX_2601/0107: Advanced Movement & AI
pub mod steering; // Curved movement patterns (open-football based)
pub mod offside_trap; // Offside trap AI (defensive line control)
pub mod gk_sweeping; // GK sweeping/rushing behavior

// Career Player Mode: User control system
mod controlled_player;
mod user_command;

// Re-export Career Player Mode types
pub use controlled_player::ControlledPlayerMode;
pub use user_command::{
    ControllerSlot, MultiAgentCommand, MultiAgentCommandBatch, OnBallAction, UserCommand,
    UserCommandPayload, UserCommandQueue,
};

// P16: Subjective Utility System
pub mod ai_integration_tests; // Phase 2: AI Tactical Manager Integration Tests
pub mod cognitive_bias; // P16 Phase 0: Cognitive Bias (Gate B용)
pub mod contract_tests; // Contract Verification CI
pub mod decision_topology; // P16 Phase 2: Gate Chain Architecture
pub mod defense_intent; // P16 Phase 1: Defense Intent System
pub mod duel; // P16 Phase 3: Attack-Defense Duel Resolution
pub mod probability_validator;
#[cfg(test)]
pub mod test_fixtures; // Centralized test helpers (FIX_2601)
pub mod utility; // P16 Phase 0: Utility calculation + Softmax
pub mod weight_composer; // Contract v1.0: Weight aggregation // P2.2-A: Probability Distribution Validation

// ============================================================================
// FIX_2601/0115: Position-neutral tie-breaker utility
// ============================================================================

/// FIX_2601/0115: Position-neutral deterministic tie-breaker
///
/// Replaces Y-position tie-breaker that caused left-side bias in pass target
/// selection and other sorted lists. Uses hash of player index and position
/// for deterministic but unbiased ordering.
///
/// # Arguments
/// * `idx_a`, `pos_a` - First item's index and position (x, y) in meters
/// * `idx_b`, `pos_b` - Second item's index and position (x, y) in meters
///
/// # Returns
/// `std::cmp::Ordering` suitable for use in `sort_by()` closures
pub fn deterministic_tie_hash(
    _idx_a: usize,
    pos_a: (f32, f32),
    _idx_b: usize,
    pos_b: (f32, f32),
) -> std::cmp::Ordering {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // FIX_2601/0116: Position-based tie-breaker
    //
    // Must satisfy total ordering (reflexive, antisymmetric, transitive).
    // We hash each position independently and compare the hashes.
    // This has some positional bias but guarantees correct ordering.
    //
    // NOTE: Cannot include player index because Home (0-10) vs Away (11-21)
    // indices would create systematic team bias.
    fn pos_hash(pos: (f32, f32)) -> u64 {
        let mut h = DefaultHasher::new();
        ((pos.0 * 100.0) as i32).hash(&mut h);
        ((pos.1 * 100.0) as i32).hash(&mut h);
        h.finish()
    }

    pos_hash(pos_a).cmp(&pos_hash(pos_b))
}

// ========== FIX_2601/0120: RNG Consumption Tracker ==========
/// Tracks RNG consumption patterns by team and category
/// Used to diagnose remaining ~25% bias in shot generation
#[derive(Debug, Clone, Default)]
pub struct RngConsumptionTracker {
    /// RNG calls for Home team (by category)
    pub home_decision: u32,      // Ball owner decision (select_best_action)
    pub home_conversion: u32,    // Action conversion (convert_player_action_with_detail)
    pub home_resolve: u32,       // Action resolve (execute_*)
    pub home_other: u32,         // Other RNG calls

    /// RNG calls for Away team (by category)
    pub away_decision: u32,
    pub away_conversion: u32,
    pub away_resolve: u32,
    pub away_other: u32,

    /// Neutral RNG calls (not team-specific)
    pub neutral: u32,

    /// Total tick count for averaging
    pub tick_count: u32,
}

impl RngConsumptionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a Home team RNG consumption
    #[inline]
    pub fn record_home(&mut self, category: RngCategory) {
        match category {
            RngCategory::Decision => self.home_decision += 1,
            RngCategory::Conversion => self.home_conversion += 1,
            RngCategory::Resolve => self.home_resolve += 1,
            RngCategory::Other => self.home_other += 1,
        }
    }

    /// Record an Away team RNG consumption
    #[inline]
    pub fn record_away(&mut self, category: RngCategory) {
        match category {
            RngCategory::Decision => self.away_decision += 1,
            RngCategory::Conversion => self.away_conversion += 1,
            RngCategory::Resolve => self.away_resolve += 1,
            RngCategory::Other => self.away_other += 1,
        }
    }

    /// Record a neutral RNG consumption
    #[inline]
    pub fn record_neutral(&mut self) {
        self.neutral += 1;
    }

    /// Record RNG for a specific player index
    #[inline]
    pub fn record_for_player(&mut self, player_idx: usize, category: RngCategory) {
        if crate::models::TeamSide::is_home(player_idx) {
            self.record_home(category);
        } else {
            self.record_away(category);
        }
    }

    /// Get total Home RNG calls
    pub fn home_total(&self) -> u32 {
        self.home_decision + self.home_conversion + self.home_resolve + self.home_other
    }

    /// Get total Away RNG calls
    pub fn away_total(&self) -> u32 {
        self.away_decision + self.away_conversion + self.away_resolve + self.away_other
    }

    /// Get ratio (Home / Away), returns 1.0 if Away is 0
    pub fn ratio(&self) -> f32 {
        let home = self.home_total() as f32;
        let away = self.away_total() as f32;
        if away == 0.0 { 1.0 } else { home / away }
    }

    /// Print summary to stderr (for debugging)
    pub fn print_summary(&self) {
        eprintln!("========== RNG Consumption Summary ==========");
        eprintln!("Home: decision={}, conversion={}, resolve={}, other={} | Total={}",
            self.home_decision, self.home_conversion, self.home_resolve, self.home_other, self.home_total());
        eprintln!("Away: decision={}, conversion={}, resolve={}, other={} | Total={}",
            self.away_decision, self.away_conversion, self.away_resolve, self.away_other, self.away_total());
        eprintln!("Neutral: {}", self.neutral);
        eprintln!("Ratio (Home/Away): {:.3}", self.ratio());
        eprintln!("Ticks: {}", self.tick_count);
        eprintln!("=============================================");
    }
}

/// RNG consumption category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RngCategory {
    Decision,    // Ball owner decision making
    Conversion,  // Action type conversion
    Resolve,     // Action resolution
    Other,       // Everything else
}

// ========== FIX_2601/1121: Detail Completeness Tracker ==========
/// Tracks detail completeness and fallback decomposition by team
/// Used to diagnose causation direction: RNG→Shot↓ or Shot↓→RNG
#[derive(Debug, Clone, Default)]
pub struct DetailCompletenessTracker {
    // ========== Detail 완전성 (RNG 호출 전) ==========
    /// detail.target이 Some인 횟수
    pub home_target_complete: u32,
    pub away_target_complete: u32,

    /// detail.target이 None인 횟수
    pub home_target_none: u32,
    pub away_target_none: u32,

    /// detail.power가 Some인 횟수
    pub home_power_complete: u32,
    pub away_power_complete: u32,

    /// detail.power가 None인 횟수
    pub home_power_none: u32,
    pub away_power_none: u32,

    // ========== Fallback별 카운트 ==========
    /// random_target() 호출 (target None)
    pub home_fallback_target_none: u32,
    pub away_fallback_target_none: u32,

    /// target == owner로 인한 loop 재시도 횟수
    pub home_fallback_self_retry: u32,
    pub away_fallback_self_retry: u32,

    /// offside valid_targets 선택
    pub home_fallback_offside: u32,
    pub away_fallback_offside: u32,

    /// dribble direction fallback
    pub home_fallback_dribble_dir: u32,
    pub away_fallback_dribble_dir: u32,

    /// shot target y fallback
    pub home_fallback_shot_y: u32,
    pub away_fallback_shot_y: u32,

    /// shot power fallback
    pub home_fallback_shot_power: u32,
    pub away_fallback_shot_power: u32,

    /// tackle target fallback
    pub home_fallback_tackle: u32,
    pub away_fallback_tackle: u32,

    // ========== 샷 기회 컨텍스트 ==========
    /// 샷 시도 횟수
    pub home_shot_attempts: u32,
    pub away_shot_attempts: u32,
}

impl DetailCompletenessTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record target completeness for a player
    #[inline]
    pub fn record_target_completeness(&mut self, player_idx: usize, is_complete: bool) {
        let is_home = crate::models::TeamSide::is_home(player_idx);
        if is_home {
            if is_complete {
                self.home_target_complete += 1;
            } else {
                self.home_target_none += 1;
            }
        } else {
            if is_complete {
                self.away_target_complete += 1;
            } else {
                self.away_target_none += 1;
            }
        }
    }

    /// Record power completeness for a player
    #[inline]
    pub fn record_power_completeness(&mut self, player_idx: usize, is_complete: bool) {
        let is_home = crate::models::TeamSide::is_home(player_idx);
        if is_home {
            if is_complete {
                self.home_power_complete += 1;
            } else {
                self.home_power_none += 1;
            }
        } else {
            if is_complete {
                self.away_power_complete += 1;
            } else {
                self.away_power_none += 1;
            }
        }
    }

    /// Record a fallback by type
    #[inline]
    pub fn record_fallback(&mut self, player_idx: usize, fallback_type: FallbackType) {
        let is_home = crate::models::TeamSide::is_home(player_idx);
        match fallback_type {
            FallbackType::TargetNone => {
                if is_home { self.home_fallback_target_none += 1; }
                else { self.away_fallback_target_none += 1; }
            }
            FallbackType::SelfRetry(count) => {
                if is_home { self.home_fallback_self_retry += count; }
                else { self.away_fallback_self_retry += count; }
            }
            FallbackType::OffsideRedirect => {
                if is_home { self.home_fallback_offside += 1; }
                else { self.away_fallback_offside += 1; }
            }
            FallbackType::DribbleDirection => {
                if is_home { self.home_fallback_dribble_dir += 1; }
                else { self.away_fallback_dribble_dir += 1; }
            }
            FallbackType::ShotTargetY => {
                if is_home { self.home_fallback_shot_y += 1; }
                else { self.away_fallback_shot_y += 1; }
            }
            FallbackType::ShotPower => {
                if is_home { self.home_fallback_shot_power += 1; }
                else { self.away_fallback_shot_power += 1; }
            }
            FallbackType::TackleTarget => {
                if is_home { self.home_fallback_tackle += 1; }
                else { self.away_fallback_tackle += 1; }
            }
        }
    }

    /// Record a shot attempt
    #[inline]
    pub fn record_shot_attempt(&mut self, player_idx: usize) {
        if crate::models::TeamSide::is_home(player_idx) {
            self.home_shot_attempts += 1;
        } else {
            self.away_shot_attempts += 1;
        }
    }

    /// Get total fallbacks for Home
    pub fn home_total_fallbacks(&self) -> u32 {
        self.home_fallback_target_none
            + self.home_fallback_self_retry
            + self.home_fallback_offside
            + self.home_fallback_dribble_dir
            + self.home_fallback_shot_y
            + self.home_fallback_shot_power
            + self.home_fallback_tackle
    }

    /// Get total fallbacks for Away
    pub fn away_total_fallbacks(&self) -> u32 {
        self.away_fallback_target_none
            + self.away_fallback_self_retry
            + self.away_fallback_offside
            + self.away_fallback_dribble_dir
            + self.away_fallback_shot_y
            + self.away_fallback_shot_power
            + self.away_fallback_tackle
    }

    /// Print summary to stderr (for debugging)
    pub fn print_summary(&self) {
        let ratio = |h: u32, a: u32| -> f32 {
            if a == 0 { if h == 0 { 1.0 } else { f32::INFINITY } }
            else { h as f32 / a as f32 }
        };

        eprintln!("============ Detail Completeness Audit ============");
        eprintln!("                        Home      Away     Ratio");
        eprintln!("-------------------------------------------------");
        eprintln!("[Target Completeness]");
        eprintln!("  target_complete:    {:>6}    {:>6}    {:.3}",
            self.home_target_complete, self.away_target_complete,
            ratio(self.home_target_complete, self.away_target_complete));
        eprintln!("  target_none:        {:>6}    {:>6}    {:.3}  ← 핵심 지표",
            self.home_target_none, self.away_target_none,
            ratio(self.home_target_none, self.away_target_none));
        eprintln!();
        eprintln!("[Power Completeness]");
        eprintln!("  power_complete:     {:>6}    {:>6}    {:.3}",
            self.home_power_complete, self.away_power_complete,
            ratio(self.home_power_complete, self.away_power_complete));
        eprintln!("  power_none:         {:>6}    {:>6}    {:.3}",
            self.home_power_none, self.away_power_none,
            ratio(self.home_power_none, self.away_power_none));
        eprintln!();
        eprintln!("[Fallback Breakdown]");
        eprintln!("  random_target:      {:>6}    {:>6}    {:.3}",
            self.home_fallback_target_none, self.away_fallback_target_none,
            ratio(self.home_fallback_target_none, self.away_fallback_target_none));
        eprintln!("  self_retry:         {:>6}    {:>6}    {:.3}",
            self.home_fallback_self_retry, self.away_fallback_self_retry,
            ratio(self.home_fallback_self_retry, self.away_fallback_self_retry));
        eprintln!("  offside_redirect:   {:>6}    {:>6}    {:.3}",
            self.home_fallback_offside, self.away_fallback_offside,
            ratio(self.home_fallback_offside, self.away_fallback_offside));
        eprintln!("  dribble_direction:  {:>6}    {:>6}    {:.3}",
            self.home_fallback_dribble_dir, self.away_fallback_dribble_dir,
            ratio(self.home_fallback_dribble_dir, self.away_fallback_dribble_dir));
        eprintln!("  shot_target_y:      {:>6}    {:>6}    {:.3}",
            self.home_fallback_shot_y, self.away_fallback_shot_y,
            ratio(self.home_fallback_shot_y, self.away_fallback_shot_y));
        eprintln!("  shot_power:         {:>6}    {:>6}    {:.3}",
            self.home_fallback_shot_power, self.away_fallback_shot_power,
            ratio(self.home_fallback_shot_power, self.away_fallback_shot_power));
        eprintln!("  tackle_target:      {:>6}    {:>6}    {:.3}",
            self.home_fallback_tackle, self.away_fallback_tackle,
            ratio(self.home_fallback_tackle, self.away_fallback_tackle));
        eprintln!();
        eprintln!("[Total Fallbacks]");
        eprintln!("  TOTAL:              {:>6}    {:>6}    {:.3}",
            self.home_total_fallbacks(), self.away_total_fallbacks(),
            ratio(self.home_total_fallbacks(), self.away_total_fallbacks()));
        eprintln!();
        eprintln!("[Shot Context]");
        eprintln!("  shot_attempts:      {:>6}    {:>6}    {:.3}",
            self.home_shot_attempts, self.away_shot_attempts,
            ratio(self.home_shot_attempts, self.away_shot_attempts));
        eprintln!("=================================================");
    }
}

/// Fallback type for detail completeness tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackType {
    /// detail.target이 None → random_target 호출
    TargetNone,
    /// target이 owner 자신 → loop 재시도 (count = 재시도 횟수)
    SelfRetry(u32),
    /// offside 위치 → valid_targets 선택
    OffsideRedirect,
    /// dribble direction fallback
    DribbleDirection,
    /// shot target y fallback
    ShotTargetY,
    /// shot power fallback
    ShotPower,
    /// tackle target fallback
    TackleTarget,
}

// ========== FIX_2601/1122: Deterministic Fallback Functions ==========
// These functions provide RNG-free deterministic selection for causation analysis.
// When feature "deterministic_fallback" is enabled, these replace RNG-based fallbacks.

/// RNG 없이 결정론적으로 선택 (hash 기반)
///
/// 입력값들의 해시로 인덱스 결정 - 순서 독립적
/// Used to test if RNG consumption is the cause of slot bias.
#[cfg(feature = "deterministic_fallback")]
pub fn deterministic_choice(
    seed: u64,
    tick: u64,
    actor_idx: usize,
    subcase: u32,
    options_count: usize,
) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    tick.hash(&mut hasher);
    actor_idx.hash(&mut hasher);
    subcase.hash(&mut hasher);

    (hasher.finish() as usize) % options_count
}

/// f32 범위에서 결정론적 값 선택 (hash 기반)
///
/// 입력값들의 해시로 [min, max] 범위의 f32 값 결정
#[cfg(feature = "deterministic_fallback")]
pub fn deterministic_f32(
    seed: u64,
    tick: u64,
    actor_idx: usize,
    subcase: u32,
    min: f32,
    max: f32,
) -> f32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    tick.hash(&mut hasher);
    actor_idx.hash(&mut hasher);
    subcase.hash(&mut hasher);

    let hash = hasher.finish();
    let normalized = (hash as f64 / u64::MAX as f64) as f32;
    min + normalized * (max - min)
}

/// Subcase constants for deterministic fallback
#[cfg(feature = "deterministic_fallback")]
pub mod deterministic_subcase {
    pub const RANDOM_TARGET: u32 = 0x01;
    pub const OFFSIDE_REDIRECT: u32 = 0x02;
    pub const DRIBBLE_DIRECTION: u32 = 0x03;
    pub const SHOT_TARGET_Y: u32 = 0x04;
    pub const SHOT_POWER: u32 = 0x05;
    pub const TACKLE_TARGET: u32 = 0x06;
    /// FIX_2601/1128: Reciprocity injection probability check
    pub const RECIPROCITY_INJECT: u32 = 0x07;
}

#[derive(Debug, Clone)]
pub struct MatchPlan {
    pub home_team: Team,
    pub away_team: Team,
    pub seed: u64,
    pub user_player: Option<super::UserPlayerConfig>,
    /// Sparse scalar bundle for external match effects (deck/coach, etc.)
    /// v1: keep deterministic + clamp ranges at injection boundaries.
    pub home_match_modifiers: super::TeamMatchModifiers,
    pub away_match_modifiers: super::TeamMatchModifiers,
    pub home_instructions: Option<TeamInstructions>,
    pub away_instructions: Option<TeamInstructions>,
    pub home_player_instructions: Option<HashMap<String, PlayerInstructions>>,
    pub away_player_instructions: Option<HashMap<String, PlayerInstructions>>,
    // Phase 2: AI Tactical Integration
    pub home_ai_difficulty: Option<AIDifficulty>,
    pub away_ai_difficulty: Option<AIDifficulty>,
}

pub struct MatchEngine {
    rng: ChaCha8Rng,
    /// Original seed for RNG restoration in snapshots
    original_seed: u64,
    pub(crate) home_team: Team,
    pub(crate) away_team: Team,

    // ========== P17: MatchSetup - 경기 시뮬레이션 OS ==========
    /// 경기 셋업 (선수 정보, track_id 매핑 등)
    /// P17: 모든 선수 조회는 setup.get_player(track_id)를 통해 수행
    pub(crate) setup: MatchSetup,

    pub(crate) minute: u8,
    /// FIX_2601: Track if we're in the second half (affects attack direction)
    /// First half: Home attacks right (X=105), Away attacks left (X=0)
    /// Second half: Home attacks left (X=0), Away attacks right (X=105)
    is_second_half: bool,
    /// FIX_2601/0105: Explicit direction context for Home team
    /// Replaces Y-flip coordinate transformations with explicit direction vectors
    pub(crate) home_ctx: DirectionContext,
    /// FIX_2601/0105: Explicit direction context for Away team
    pub(crate) away_ctx: DirectionContext,
    result: MatchResult,
    event_generator: EventGenerator,
    stats_calculator: StatsCalculator,
    /// Phase 0: Minimal balance diagnostics
    balance_diagnostics: balance_diagnostics::BalanceDiagnostics,
    /// Pass sequence tracking for KPI (per team)
    current_pass_sequence_home: u16,
    current_pass_sequence_away: u16,
    /// FIX_2601/1128: Recent pass pairs for reciprocity bonus
    /// Stores (passer_idx, receiver_idx) for last N successful passes
    /// Used to boost pass score when reverse edge exists (A→B recent → B→A bonus)
    recent_pass_pairs: VecDeque<(u8, u8)>,
    /// FIX_2601/1130: Track how many times each player received a pass during the match
    /// Used to apply diversity penalty to frequent receivers
    pass_receive_counts: [u8; 22],
    /// FIX_2601/0123: SafePass return-bias sequence (match-local)
    /// Used by DecisionTopology to avoid global counters and keep seed determinism.
    safe_pass_seq: u64,
    /// Possession tracking for pass gating (decision ticks)
    possession_owner_idx: Option<usize>,
    possession_owner_since_tick: u64,
    pub(crate) user_player: Option<super::UserPlayerConfig>,
    home_instructions: TeamInstructions,
    away_instructions: TeamInstructions,
    home_match_modifiers: super::TeamMatchModifiers,
    away_match_modifiers: super::TeamMatchModifiers,

    // P0: Defensive Tuning (per-team tactical configuration)
    home_defensive_tuning: DefensiveTuning,
    away_defensive_tuning: DefensiveTuning,

    // Phase D: ?�이?�인???�합
    pub home_formation: String,
    pub away_formation: String,
    ball: Ball,
    game_state: GameState,
    /// Optional override for player positions (used in tests)
    test_player_positions: Option<Vec<(f32, f32)>>,
    /// Enable position tracking for replay (increases memory usage)
    track_positions: bool,
    /// Current timestamp in milliseconds
    current_timestamp_ms: u64,
    /// Cached context from the last `init()` call (used by step-based / interactive APIs)
    precomputed_home_strength: f32,
    precomputed_away_strength: f32,
    precomputed_possession_ratio: f32,
    precomputed_match_duration: u8,
    /// Phase E: last time (ms) we paused for interactive intervention
    last_intervention_ms: u64,
    /// Dynamic player positions in Coord10 (0.1m precision), 22 players total
    /// FIX_2601 Phase 3.6: Changed from Vec<(f32, f32)> normalized to Vec<Coord10>
    pub(crate) player_positions: Vec<super::types::Coord10>,
    /// ??A13: ?�수?반응 ?�태 (22?
    player_reaction_states: Vec<PlayerReactionState>,
    /// ??P1: 개인 ?�수 ?�술 지??(?�수 ?�름 ??Instructions)
    home_player_instructions: std::collections::HashMap<String, PlayerInstructions>,
    away_player_instructions: std::collections::HashMap<String, PlayerInstructions>,
    /// P3: 교체 횟수 (홈팀, 원정팀) - 최대 5회
    pub(crate) substitutions_made: (u8, u8),
    /// FIX_2601/0106 P2.1: First-half stoppage time accumulator (seconds).
    stoppage_seconds_first_half: u16,
    /// FIX_2601/0106 P2.1: Whether added time has been finalized at minute 45.
    stoppage_finalized_first_half: bool,
    /// FIX_2601/0106 P2.1: Actual first-half end minute (45 + added time).
    first_half_end_minute: u8,
    /// FIX_2601/0106 P2.2: Second-half stoppage time accumulator (seconds).
    stoppage_seconds_second_half: u16,
    /// FIX_2601/0106 P2.2: Whether added time has been finalized at regulation end.
    stoppage_finalized_second_half: bool,
    /// FIX_2601/0106 P2.2: Actual match end minute (regulation + added time).
    match_end_minute: u8,
    /// P3: 선수 피로도 (22명, 0.0=신선 ~ 1.0=지침)
    pub(crate) player_fatigue: Vec<f32>,
    /// P3: 부상 선수 인덱스 목록
    pub(crate) injured_players: Vec<usize>,

    // ========== P10-13: Stamina System ==========
    /// 선수별 현재 스태미나 (0.0 = 지침, 1.0 = 풀 컨디션)
    /// P3의 player_fatigue와 별개로 틱 단위 정밀 추적용
    pub(crate) stamina: [f32; 22],

    /// 선수별 스프린트 상태 (현재 틱에서 스프린트 중인지)
    sprint_state: [bool; 22],

    /// Sticky action toggles per player (sprint/dribble/press)
    sticky_actions: [StickyActions; 22],

    /// FIX_2601/0106 P3: 선수별 휴식 상태 (스태미나 부족으로 걷기 모드)
    /// true면 최대 속도가 걷기 속도(1.5 m/s)로 제한됨
    player_resting: [bool; 22],

    /// FIX_2601/0106 P4: 연속 달리기 틱 수 (시간 기반 피로 계산용)
    /// 휴식 시 0으로 리셋, 달리는 동안 증가
    continuous_running_ticks: [u32; 22],

    /// P10-13: EV Decision에서 선택된 패스 타겟
    pub(crate) last_pass_target: Option<usize>,

    /// FIX_2601/0102: 어시스트 후보 추적
    pub(crate) assist_candidate: Option<super::types::AssistCandidate>,

    // ========== Phase 3.5: ActionQueue Integration ==========
    /// 홈팀 페이즈 상태 (Attack, Defense, Transition)
    home_phase_state: super::TeamPhaseState,
    /// 어웨이팀 페이즈 상태
    away_phase_state: super::TeamPhaseState,

    // ========== FIX_2601/1129: AttackPhase System ==========
    /// 홈팀 공격 국면 상태 (Circulation/Positional/Transition)
    home_attack_state: attack_phase::TeamAttackState,
    /// 어웨이팀 공격 국면 상태
    away_attack_state: attack_phase::TeamAttackState,

    // ========== FIX_2601/0123: Match State Machine ==========
    /// 게임 흐름 상태 머신 (InPlay, DeadBall, Restart 등)
    /// 농구 RE GameData.GFPEGOGEAII 패턴 기반
    game_flow_machine: match_state::GameFlowMachine,

    // ========== FIX_2601/0123: Centralized Rule Dispatcher ==========
    /// 중앙 집중식 규칙 디스패처 (팀 바이어스 제거)
    /// 결정론적 평가 순서: Goal > OutOfPlay > Offside > Foul > Handball
    rule_dispatcher: rules::RuleDispatcher,
    /// Rule check mode for gradual migration (StatisticsOnly → DispatcherPrimary)
    rule_check_mode: rules::RuleCheckMode,

    /// 선수별 목표 (22명)
    player_objectives: Vec<super::PlayerObjective>,
    /// 틱 기반 액션 큐
    action_queue: super::ActionQueue,
    /// 5채널 포지셔닝 그리드
    pep_grid: super::PepGrid,
    /// Off-the-Ball 포지셔닝 엔진
    positioning_engine: super::PositioningEngine,
    /// 현재 글로벌 틱 (분 내부)
    current_tick: u64,
    /// FIX_2601/0119: Last kickoff tick for stabilization logic
    /// Tracks when the most recent kickoff occurred (game start or after halftime)
    last_kickoff_tick: u64,
    // 2025-12-11: use_tick_based_sim 필드 제거 - tick 기반 엔진만 사용

    // ========== Phase 5: Hero Growth System ==========
    /// 히어로 XP 누적 버킷 (경기 중)
    hero_xp_bucket: super::growth::HeroXpBucket,

    // ========== P6: ReplayRecorder Integration ==========
    /// Optional replay recorder for generating ReplayDoc
    /// When enabled, records all events during simulation
    replay_recorder: Option<ReplayRecorder>,

    /// Decision intent logs for the current tick (debug output)
    decision_intents: Vec<DecisionIntent>,

    /// Decision scheduler (DPQ) state.
    ///
    /// FIX_2601/0113 v1.1: routing layer only (no behavior change); cadence is
    /// "every tick" and is feature-gated by `ExpConfig.decision.dpq_enabled`.
    decision_scheduler: crate::engine::decision_scheduler::DecisionScheduler,

    /// FIX_2512 Phase 3: ReplayWriter v2
    /// Optional Coord10-based replay writer (v2 format)
    /// Enabled with `with_replay_v2_recording()`
    replay_writer_v2: Option<crate::replay::ReplayWriterV2>,

    // ========== P7: Phase-Based Action System ==========
    /// 선수 상태 (Idle, Moving, InAction, Recovering, etc.)
    player_states: PlayerStates,

    /// 태클 쿨다운 (틱 단위, 0이면 태클 가능)
    tackle_cooldowns: [u8; 22],

    /// 각 선수의 속도 (m/s)
    player_speeds: [f32; 22],

    /// 수비 역할 (팀별: [홈팀, 어웨이팀])
    defensive_roles: [Vec<DefensiveRole>; 2],

    /// Presser 움직임 FSM (팀별: [홈팀, 어웨이팀])
    presser_movements: [Vec<PresserMovement>; 2],

    /// FIX_2601/0106 P3: Marker 움직임 FSM (팀별: [홈팀, 어웨이팀])
    marker_movements: [Vec<MarkerMovement>; 2],

    /// FIX_2601/0106 P3: Cover 움직임 FSM (팀별: [홈팀, 어웨이팀])
    cover_movements: [Vec<CoverMovement>; 2],

    /// FIX_2601/0106 P4: 경기 상황 (스코어, 시간 기반 전술 조정)
    match_situation: MatchSituation,

    /// 기본 포메이션 위치 (팀별, 미터 단위)
    base_formations: [Vec<(f32, f32)>; 2],

    /// 수비 라인 설정 (팀별)
    defensive_lines: [DefensiveLine; 2],

    // ========== P7 Phase 9: Active FSM Storage ==========
    // NOTE: active_tackles/passes/shots/dribbles 삭제됨 (2025-12-14)
    // ActionQueue.active가 유일한 "진행 중 액션" 저장소
    /// P9: 활성 세트피스 FSM 목록 (세트피스만 FSM 유지)
    active_set_pieces: Vec<SetPieceAction>,

    /// 다음 FSM ID (세트피스용)
    next_fsm_id: u64,

    // ========== P7-OFFSIDE: Offside Counters ==========
    /// 홈팀 오프사이드 횟수
    pub(crate) offside_count_home: u16,

    /// 어웨이팀 오프사이드 횟수
    pub(crate) offside_count_away: u16,

    // ========== FIX_2601/0105: Shot Budget System ==========
    /// 홈팀 이번 하프 슛 횟수
    shots_this_half_home: u8,

    /// 어웨이팀 이번 하프 슛 횟수
    shots_this_half_away: u8,

    /// 하프당 슛 예산 (EPL 기준: 7-8개/하프 = ~14개/경기)
    /// 0이면 제한 없음
    shot_budget_per_half: u8,

    // ========== P10-13 Phase 7: Debug Logger ==========
    /// AI 의사결정 디버그 로거
    #[cfg(debug_assertions)]
    pub(crate) debug_logger: super::DebugLogger,

    // ========== Phase 2: AI Tactical Manager ==========
    /// AI 전술 관리자 (홈팀, CPU 팀만 Some)
    home_ai_manager: Option<AITacticalManager>,

    /// AI 전술 관리자 (원정팀, CPU 팀만 Some)
    away_ai_manager: Option<AITacticalManager>,

    /// 마지막 AI 업데이트 시간 (분)
    last_ai_update_minute: u32,

    /// 이전 틱의 점수 (변화 감지용)
    previous_score_home: u32,
    previous_score_away: u32,

    // ========== Elastic Band Theory: Relative Positioning ==========
    /// 홈팀 포지셔닝 상태 (팀 기준점, 라인 높이 등)
    elastic_home_state: TeamPositioningState,

    /// 어웨이팀 포지셔닝 상태
    elastic_away_state: TeamPositioningState,

    /// 홈팀 Elastic Band 전술 설정
    elastic_home_tactics: ElasticTactics,

    /// 어웨이팀 Elastic Band 전술 설정
    elastic_away_tactics: ElasticTactics,

    /// 홈팀 포메이션 오프셋 (11명)
    home_formation_offsets: Vec<FormationOffset>,

    /// 어웨이팀 포메이션 오프셋 (11명)
    away_formation_offsets: Vec<FormationOffset>,

    // ========== FIX_2601/0123: Team Momentum System (leadership) ==========
    /// 홈팀 모멘텀 상태 (골/카드 이벤트에 따라 변동)
    home_momentum: momentum::TeamMomentum,
    /// 어웨이팀 모멘텀 상태
    away_momentum: momentum::TeamMomentum,

    // ========== P0: Goal Contract ==========
    /// 경기장의 두 골대 (축구 세계의 헌법)
    /// Home 골대 (x=0) = Home 팀이 지킴
    /// Away 골대 (x=105) = Away 팀이 지킴
    goals: Goals,

    /// 현재 틱에서 골이 이미 처리되었는지 (중복 골 방지)
    /// 매 틱 시작 시 false로 리셋됨
    goal_scored_this_tick: bool,

    /// 현재 틱에서 점유가 변경되었는지 (MarkingManager T2 입력)
    /// 매 틱 시작 시 false로 리셋됨
    possession_changed_this_tick: bool,

    /// 현재 틱에서 재시작(킥오프/아웃/파울 등)이 발생했는지 (MarkingManager T1 입력)
    /// 매 틱 시작 시 false로 리셋됨
    restart_occurred_this_tick: bool,

    /// 현재 틱에서 재시작 종류가 확정되었는지 (MarkingManager T1 set-piece 분기용)
    /// - `None`이면 "재시작 없음" 또는 "종류 미상"
    /// - `Some(RestartType::...)`이면 set-piece 전용 마킹을 선택 가능
    /// 매 틱 시작 시 `None`으로 리셋됨
    restart_type_this_tick: Option<super::RestartType>,
    /// Offside restart needs indirect free kick (consumed when restart is applied)
    pending_indirect_free_kick: bool,

    // ========== P15: Player Inertia Physics System ==========
    /// 선수별 속도 벡터 (m/s) - 관성 물리 시스템
    /// player_speeds와 달리 방향 정보 포함 (vx, vy)
    player_velocities: [(f32, f32); 22],

    /// Phase 1.1: 선수 몸 방향 (정규화된 벡터)
    /// 압박 각도, 위협도 계산에 사용
    player_body_dir: [(f32, f32); 22],

    /// Phase 1.3: MarkingManager (full system)
    home_marking_manager: super::marking_manager::MarkingManager,
    away_marking_manager: super::marking_manager::MarkingManager,

    /// Phase 1.4: TransitionSystem (3s possession-change window)
    transition_system: super::transition_system::TransitionSystem,

    /// 선수별 이동 물리 파라미터 (ability 기반, 경기 시작 시 계산)
    player_motion_params: [super::player_motion_params::PlayerMotionParams; 22],

    // ========== P18: FieldBoard (A-Plan Board Layer) ==========
    /// 보드 레이어 (점유/압박 히트맵)
    /// Decision/Tactics의 보조 정보 제공
    pub field_board: Option<FieldBoard>,

    // ========== FIX_2601/0106: Ball Prediction System ==========
    /// 공 예측 시스템 (10ms 간격, 최대 3초 예측)
    /// 인터셉트 타이밍 계산 및 수비 위치 결정에 사용
    pub(crate) ball_prediction: BallPrediction,

    // ========== Career Player Mode: User Control System ==========
    /// Career Player Mode 상태 (None이면 비활성화)
    pub(crate) controlled_mode: Option<ControlledPlayerMode>,

    /// 유저 명령 큐 (FIFO)
    pub(crate) user_command_queue: UserCommandQueue,

    /// Multi-agent controller registry (controller_id -> slot)
    pub(crate) multi_agent_registry: HashMap<u32, ControllerSlot>,
    /// Multi-agent last seq per controller (duplicate guard)
    pub(crate) multi_agent_last_seq: HashMap<u32, u32>,
    /// Multi-agent controlled track_ids
    pub(crate) multi_agent_tracks: HashSet<usize>,
    /// Multi-agent command queues per track_id
    pub(crate) multi_agent_command_queues: Vec<UserCommandQueue>,
    /// Multi-agent input locks per track_id
    pub(crate) multi_agent_lock_until_tick: Vec<u64>,

    // ========== FIX_2601/0107: Open-Football Advanced Modules ==========
    /// Offside trap state per team [home, away]
    pub(crate) offside_trap_state: [offside_trap::DefensiveLineState; 2],

    /// GK sweeping state per team [home, away]
    pub(crate) gk_sweeping_state: [gk_sweeping::GKSweepingState; 2],

    // ========== FIX_2601/0108: UAE Pipeline Integration ==========
    /// Feature flag for Unified Action Evaluator pipeline
    /// When true, uses UAE DecisionPipeline instead of decision_topology.rs
    /// Default: false (legacy system)
    pub(crate) use_uae_pipeline: bool,

    // ========== DPER Framework: Experimental Configuration ==========
    /// Runtime experimental parameters for A/B testing
    /// Applied via `apply_exp_config()` before running simulation
    /// When None, uses default (stable) parameters
    pub(crate) exp_params: Option<crate::engine::experimental::RuntimeExpParams>,

    // ========== FIX_2601/0112: Statistical Anchor Calibration ==========
    /// Home team statistics snapshot (collected during match)
    pub(crate) home_stat_snapshot: MatchStatSnapshot,

    /// Away team statistics snapshot (collected during match)
    pub(crate) away_stat_snapshot: MatchStatSnapshot,

    /// Calibrator parameters to apply (optional, affects action probabilities)
    pub(crate) calibrator_params: Option<CalibratorParams>,

    // ========== FIX_2601/0115: Off-Ball Decision System v1 ==========
    /// Off-ball objectives for all 22 players (TTL-based)
    /// Updated each tick when offball_decisions_enabled is true in ExpConfig.
    /// Consumed by positioning_engine to guide off-ball movement.
    pub(crate) offball_objectives: [super::offball::OffBallObjective; 22],

    /// Off-ball system configuration (tuning points: TTL, Top-K, softmax_temperature)
    pub(crate) offball_config: super::offball::OffBallConfig,

    // ========== FIX_2601/0120: RNG Consumption Tracking ==========
    /// Tracks RNG consumption patterns by team for bias analysis
    pub(crate) rng_tracker: RngConsumptionTracker,

    // ========== FIX_2601/1121: Detail Completeness Tracking ==========
    /// Tracks detail completeness and fallback decomposition for causation analysis
    pub(crate) detail_tracker: DetailCompletenessTracker,

    // ========== FIX_2601: Shot Opportunity Telemetry ==========
    /// Shot opportunity telemetry for bias detection (env-gated: OF_DEBUG_SHOT_OPP=1)
    /// Records all decision frames where shot was in Top-K candidates with valid utility
    pub(crate) shot_opp_telemetry: Option<shot_opportunity::ShotOppTelemetry>,
}

impl MatchEngine {
    pub fn new(plan: MatchPlan) -> Result<Self, String> {
        let original_seed = plan.seed;
        let rng = ChaCha8Rng::seed_from_u64(original_seed);

        // Configure event generator based on user player config
        let mut event_generator = EventGenerator::new();
        if let Some(ref user_config) = plan.user_player {
            event_generator = event_generator
                .with_user_config(&user_config.player_name, user_config.highlight_level);
        }

        // P1: 개인 선수 전술 지시 - struct 초기화 전에 참조 사용을 준비
        let mut home_player_instructions = plan.home_player_instructions.unwrap_or_default();
        Self::populate_default_player_instructions(&mut home_player_instructions, &plan.home_team);

        let mut away_player_instructions = plan.away_player_instructions.unwrap_or_default();
        Self::populate_default_player_instructions(&mut away_player_instructions, &plan.away_team);

        // P17: MatchSetup 생성 (team move 전에 참조로 생성)
        let setup = MatchSetup::from_teams(&plan.home_team, &plan.away_team)?;

        // FIX_2512 Phase 0: Audit Gates - Validate match plan
        let home_formation_ref = plan.home_team.formation.code();
        let away_formation_ref = plan.away_team.formation.code();
        super::audit_gates::validate_match_plan(&setup, home_formation_ref, away_formation_ref)
            .map_err(|err| format!("Match plan validation failed: {}", err))?;

        let home_formation_code = home_formation_ref.to_string();
        let away_formation_code = away_formation_ref.to_string();

        // Phase 2: AI Tactical Manager 초기화
        let home_ai_manager = plan.home_ai_difficulty.map(|difficulty| {
            let profile = Self::select_ai_profile_for_team(&plan.home_team, difficulty);
            AITacticalManager::new(profile, difficulty)
        });

        let away_ai_manager = plan.away_ai_difficulty.map(|difficulty| {
            let profile = Self::select_ai_profile_for_team(&plan.away_team, difficulty);
            AITacticalManager::new(profile, difficulty)
        });

        let mut multi_agent_command_queues = Vec::with_capacity(22);
        for _ in 0..22 {
            multi_agent_command_queues.push(UserCommandQueue::new());
        }
        let multi_agent_lock_until_tick = vec![0; 22];

        Ok(Self {
            rng,
            original_seed,
            home_team: plan.home_team,
            away_team: plan.away_team,
            setup,
            minute: 0,
            is_second_half: false, // FIX_2601: Start in first half
            // FIX_2601/0105: Initialize direction contexts
            // First half: Home attacks right (x=1050), Away attacks left (x=0)
            home_ctx: DirectionContext::new(true), // home, attacks_right=true
            away_ctx: DirectionContext::new(false), // away, attacks_right=false
            result: MatchResult::new(),
            event_generator,
            stats_calculator: StatsCalculator::new(),
            balance_diagnostics: balance_diagnostics::BalanceDiagnostics::new(),
            current_pass_sequence_home: 0,
            current_pass_sequence_away: 0,
            recent_pass_pairs: VecDeque::with_capacity(20),
            pass_receive_counts: [0; 22],  // FIX_2601/1130
            safe_pass_seq: 0,
            possession_owner_idx: None,
            possession_owner_since_tick: 0,
            user_player: plan.user_player,
            home_instructions: plan.home_instructions.unwrap_or_default(),      
            away_instructions: plan.away_instructions.unwrap_or_default(),      
            home_match_modifiers: plan.home_match_modifiers,
            away_match_modifiers: plan.away_match_modifiers,

            // P0: Defensive Tuning (initialized to defaults, updated each tick)
            home_defensive_tuning: DefensiveTuning::default(),
            away_defensive_tuning: DefensiveTuning::default(),

            // Phase D 초기화
            home_formation: home_formation_code,
            away_formation: away_formation_code,
            ball: Ball::default(),
            game_state: GameState::default(),
            test_player_positions: None,
            track_positions: false,
            current_timestamp_ms: 0,
            precomputed_home_strength: 0.0,
            precomputed_away_strength: 0.0,
            precomputed_possession_ratio: 0.0,
            precomputed_match_duration: MATCH_DURATION_CAP_MINUTES,
            last_intervention_ms: 0,
            player_positions: Vec::new(), // Will be initialized in simulate()
            player_reaction_states: vec![PlayerReactionState::default(); 22],
            home_player_instructions,
            away_player_instructions,
            substitutions_made: (0, 0),
            stoppage_seconds_first_half: 0,
            stoppage_finalized_first_half: false,
            first_half_end_minute: HALF_DURATION_MINUTES,
            stoppage_seconds_second_half: 0,
            stoppage_finalized_second_half: false,
            match_end_minute: REGULATION_TOTAL_MINUTES,
            player_fatigue: vec![0.0; 22],
            injured_players: Vec::new(),

            // P10-13: Stamina System 초기화
            stamina: [1.0; 22], // 모두 풀 컨디션으로 시작
            sprint_state: [false; 22],
            sticky_actions: [StickyActions::default(); 22],
            // FIX_2601/0106 P3: 선수별 휴식 상태 (스태미나 < 30%면 걷기 모드)
            player_resting: [false; 22],
            // FIX_2601/0106 P4: 연속 달리기 틱 초기화
            continuous_running_ticks: [0; 22],
            last_pass_target: None,
            assist_candidate: None, // FIX_2601/0102

            // Phase 3.5 초기화
            home_phase_state: super::TeamPhaseState::new(super::TeamPhase::Defense),
            away_phase_state: super::TeamPhaseState::new(super::TeamPhase::Defense),

            // FIX_2601/1129: AttackPhase System
            home_attack_state: attack_phase::TeamAttackState::new(),
            away_attack_state: attack_phase::TeamAttackState::new(),

            // FIX_2601/0123: Match State Machine
            game_flow_machine: match_state::GameFlowMachine::with_state(
                match_state::GameFlowState::KickoffReady {
                    restart_team: match_state::TeamId::HOME,
                }
            ),

            // FIX_2601/0123: Centralized Rule Dispatcher
            rule_dispatcher: rules::RuleDispatcher::new(),
            rule_check_mode: rules::RuleCheckMode::from_env(),

            player_objectives: vec![super::PlayerObjective::MaintainShape; 22],
            action_queue: super::ActionQueue::new(),
            pep_grid: super::PepGrid::new().with_max_per_channel(2),
            positioning_engine: super::PositioningEngine::new(),
            current_tick: 0,
            last_kickoff_tick: 0,  // FIX_2601/0119: Initialize kickoff tick tracking
            // 2025-12-11: use_tick_based_sim 제거 - tick 기반 엔진만 사용

            // Phase 5: Hero Growth
            hero_xp_bucket: super::growth::HeroXpBucket::new(),

            // P6: ReplayRecorder - disabled by default, enable with with_replay_recording()
            replay_recorder: None,

            // FIX_2512 Phase 3: ReplayWriter v2 - disabled by default
            replay_writer_v2: None,

            // P7: Phase-Based Action System
            player_states: default_player_states(),
            tackle_cooldowns: [0; 22],
            player_speeds: [5.0; 22], // 기본 속도 5m/s
            defensive_roles: [vec![DefensiveRole::Cover; 11], vec![DefensiveRole::Cover; 11]],
            presser_movements: [Vec::new(), Vec::new()],
            // FIX_2601/0106 P3: Marker/Cover FSM 초기화
            marker_movements: [Vec::new(), Vec::new()],
            cover_movements: [Vec::new(), Vec::new()],
            // FIX_2601/0106 P4: 경기 상황 초기화
            match_situation: MatchSituation::default(),
            base_formations: [Vec::new(), Vec::new()], // Will be initialized from formation
            defensive_lines: [DefensiveLine::Normal, DefensiveLine::Normal],

            // P7 Phase 9: Active FSM Storage (세트피스만 유지)
            active_set_pieces: Vec::new(),
            next_fsm_id: 1,

            // P7-OFFSIDE: Offside Counters
            offside_count_home: 0,
            offside_count_away: 0,

            // FIX_2601/0105: Shot Budget System
            shots_this_half_home: 0,
            shots_this_half_away: 0,
            shot_budget_per_half: 3, // FIX_2601/0116: 결정-실행 갭 보상, 더 낮춤

            // P10-13 Phase 7: Debug Logger
            #[cfg(debug_assertions)]
            debug_logger: super::DebugLogger::default(),

            // Phase 2: AI Tactical Manager
            home_ai_manager,
            away_ai_manager,
            last_ai_update_minute: 0,
            previous_score_home: 0,
            previous_score_away: 0,

            // Elastic Band Theory: Relative Positioning
            elastic_home_state: TeamPositioningState::default(),
            elastic_away_state: TeamPositioningState::default(),
            elastic_home_tactics: ElasticTactics::default(),
            elastic_away_tactics: ElasticTactics::default(),
            home_formation_offsets: Self::generate_formation_offsets("4-4-2"),
            away_formation_offsets: Self::generate_formation_offsets("4-4-2"),

            // FIX_2601/0123: Team Momentum System (leadership integration)
            // Captain leadership will be set in init() when players are resolved
            home_momentum: momentum::TeamMomentum::default(),
            away_momentum: momentum::TeamMomentum::default(),

            // P0: Goal Contract - 축구 세계의 헌법
            goals: Goals::new(),
            goal_scored_this_tick: false,
            possession_changed_this_tick: false,
            restart_occurred_this_tick: false,
            restart_type_this_tick: None,
            pending_indirect_free_kick: false,

            // P15: Player Inertia Physics System
            player_velocities: [(0.0, 0.0); 22], // 모두 정지 상태로 시작
            // FIX_2601/0110: Away 팀은 왼쪽을 향해야 함
            player_body_dir: std::array::from_fn(|i| {
                if i < 11 { (1.0, 0.0) } else { (-1.0, 0.0) }
            }),
            home_marking_manager: super::marking_manager::MarkingManager::new(),
            away_marking_manager: super::marking_manager::MarkingManager::new(),
            transition_system: super::transition_system::TransitionSystem::new(),
            player_motion_params: std::array::from_fn(|_| {
                super::player_motion_params::PlayerMotionParams::default()
            }),

            // P18: FieldBoard (A-Plan Board Layer)
            field_board: Some(FieldBoard::new(FieldBoardSpec::default())),

            // FIX_2601/0106: Ball Prediction System
            ball_prediction: BallPrediction::new(),

            // Career Player Mode: User Control System
            controlled_mode: None,
            user_command_queue: UserCommandQueue::new(),
            multi_agent_registry: HashMap::new(),
            multi_agent_last_seq: HashMap::new(),
            multi_agent_tracks: HashSet::new(),
            multi_agent_command_queues,
            multi_agent_lock_until_tick,

            // FIX_2601/0107: DecisionIntent 초기화
            decision_intents: Vec::new(),

            // FIX_2601/0113 v1.1: DPQ scheduler (routing layer only).
            decision_scheduler: crate::engine::decision_scheduler::DecisionScheduler::new(),

            // FIX_2601/0107: Open-Football Advanced Modules
            offside_trap_state: [
                offside_trap::DefensiveLineState::default(),
                offside_trap::DefensiveLineState::default(),
            ],
            gk_sweeping_state: [
                gk_sweeping::GKSweepingState::default(),
                gk_sweeping::GKSweepingState::default(),
            ],
            // FIX_2601/0108: UAE Pipeline (disabled by default)
            use_uae_pipeline: false,
            // DPER Framework: Experimental parameters (None = stable baseline)
            exp_params: None,
            // FIX_2601/0112: Statistical Anchor Calibration
            home_stat_snapshot: MatchStatSnapshot::new(0, original_seed),
            away_stat_snapshot: MatchStatSnapshot::new(1, original_seed),
            calibrator_params: None,

            // FIX_2601/0115: Off-Ball Decision System v1
            offball_objectives: [super::offball::OffBallObjective::default(); 22],
            offball_config: super::offball::OffBallConfig::default(),

            // FIX_2601/0120: RNG Consumption Tracking
            rng_tracker: RngConsumptionTracker::new(),

            // FIX_2601/1121: Detail Completeness Tracking
            detail_tracker: DetailCompletenessTracker::new(),
            shot_opp_telemetry: if shot_opportunity::ShotOppTelemetry::is_enabled() {
                Some(shot_opportunity::ShotOppTelemetry::new())
            } else {
                None
            },
        })
    }

    /// P15: 선수별 이동 물리 파라미터 초기화 (경기 시작 시 호출)
    fn init_player_motion_params(&mut self) {
        use super::player_motion_params::ability_to_motion_params;

        // SSOT: Always derive from MatchSetup (supports substitutions).
        for track_id in 0..22 {
            let attrs = self.get_player_attributes(track_id);
            self.player_motion_params[track_id] = ability_to_motion_params(attrs);
        }
    }

    /// Refresh motion params for a single pitch slot (used by substitutions).
    pub(crate) fn refresh_player_motion_params_for_track_id(&mut self, track_id: usize) {
        use super::player_motion_params::ability_to_motion_params;
        if track_id >= 22 {
            return;
        }
        let attrs = self.get_player_attributes(track_id);
        self.player_motion_params[track_id] = ability_to_motion_params(attrs);
    }

    /// Reset runtime state for a pitch slot after a substitution roster swap.
    ///
    /// Note: The pitch slot (`track_id 0..21`) stays the same, but the occupying
    /// player identity (attributes/traits) changes via `MatchSetup` assignment.
    pub(crate) fn reset_pitch_slot_after_substitution(&mut self, track_id: usize) {
        if track_id >= 22 {
            return;
        }

        // Fatigue/stamina
        if track_id < self.player_fatigue.len() {
            self.player_fatigue[track_id] = 0.0;
        }
        self.stamina[track_id] = 1.0;
        self.player_resting[track_id] = false;
        self.continuous_running_ticks[track_id] = 0;

        // Action/locomotion state
        self.player_states[track_id] = PlayerState::Idle;
        self.tackle_cooldowns[track_id] = 0;
        self.player_speeds[track_id] = 0.0;
        self.player_velocities[track_id] = (0.0, 0.0);
        self.sprint_state[track_id] = false;
        self.sticky_actions[track_id] = StickyActions::default();

        // Injury flags are slot-based in v1; clear them for the new occupant.
        self.injured_players.retain(|&idx| idx != track_id);

        self.refresh_player_motion_params_for_track_id(track_id);
    }

    /// Phase 2: 팀과 난이도에 따라 적절한 AI 프로필 선택
    fn select_ai_profile_for_team(
        _team: &Team,
        difficulty: AIDifficulty,
    ) -> crate::tactics::AITacticalProfile {
        use crate::tactics::{ADAPTIVE_AI, BALANCED_AI};

        // 난이도별 기본 프로필 선택
        match difficulty {
            AIDifficulty::Easy | AIDifficulty::Medium => {
                // Easy/Medium: Balanced AI 사용
                BALANCED_AI.clone()
            }
            AIDifficulty::Hard | AIDifficulty::Expert => {
                // Hard/Expert: Adaptive AI 사용 (향후 팀 성향 반영 가능)
                // TODO: 팀 이름/스타일에 따라 다른 프로필 선택
                // 예: "Barcelona" → Tiki-Taka 선호 AI
                //     "Liverpool" → Gegenpressing 선호 AI
                ADAPTIVE_AI.clone()
            }
        }
    }

    // ========== Phase 2.2: MatchState 생성 로직 ==========

    /// 현재 경기 상태를 MatchState로 변환
    fn get_current_match_state(&self) -> MatchState {
        MatchState {
            home_score: self.result.score_home as i32,
            away_score: self.result.score_away as i32,
            current_minute: self.current_minute() as u32,
            average_stamina: self.calculate_average_stamina(),
        }
    }

    /// 평균 체력 계산 (22명 선수 전체)
    fn calculate_average_stamina(&self) -> f32 {
        let total: f32 = self.stamina.iter().sum();
        total / 22.0
    }

    // ========== FIX_2601/0123: Team Momentum Helpers ==========

    /// Find the highest leadership value among field players
    ///
    /// Field players are idx 1-10 for home, 12-21 for away (GK excluded).
    /// Returns the highest leadership found, defaulting to 10.0 if none.
    fn find_captain_leadership(&self, is_home: bool) -> f32 {
        let (start, end) = if is_home { (1, 11) } else { (12, 22) };
        let mut max_leadership = 10.0f32;

        for idx in start..end {
            let leadership = self.get_player_leadership(idx);
            if leadership > max_leadership {
                max_leadership = leadership;
            }
        }

        max_leadership
    }

    // ========== Phase 2.3: AI 업데이트 로직 ==========

    /// AI 전술 업데이트 필요 여부 확인 및 실행
    fn update_ai_tactics_if_needed(&mut self) {
        let match_state = self.get_current_match_state();

        // Phase 2.4: 점수가 변했는지 확인
        let score_changed = (match_state.home_score as u32) != self.previous_score_home
            || (match_state.away_score as u32) != self.previous_score_away;

        if score_changed {
            println!(
                "[{}min] Score changed! ({}-{} → {}-{}) Triggering AI update...",
                match_state.current_minute,
                self.previous_score_home,
                self.previous_score_away,
                match_state.home_score,
                match_state.away_score,
            );
        }

        // 홈팀 AI 업데이트 (점수 변화 시 강제 업데이트)
        if let Some(ai) = &mut self.home_ai_manager {
            if score_changed || ai.should_update(&match_state) {
                if let Some(new_tactics) =
                    ai.update_tactics(&match_state, &self.away_instructions, &mut self.rng)
                {
                    self.home_instructions = new_tactics.clone();
                    self.log_tactical_change("Home", &new_tactics);
                }
            }
        }

        // 원정팀 AI 업데이트 (점수 변화 시 강제 업데이트)
        if let Some(ai) = &mut self.away_ai_manager {
            if score_changed || ai.should_update(&match_state) {
                if let Some(new_tactics) =
                    ai.update_tactics(&match_state, &self.home_instructions, &mut self.rng)
                {
                    self.away_instructions = new_tactics.clone();
                    self.log_tactical_change("Away", &new_tactics);
                }
            }
        }

        // 마지막 업데이트 시간 및 점수 기록
        self.last_ai_update_minute = match_state.current_minute;
        self.previous_score_home = match_state.home_score as u32;
        self.previous_score_away = match_state.away_score as u32;
    }

    /// 전술 변경 로그 기록
    fn log_tactical_change(&self, team: &str, new_tactics: &TeamInstructions) {
        println!(
            "[{}min] {} team changed tactics: Tempo={:?}, Pressing={:?}, Width={:?}",
            self.current_minute(),
            team,
            new_tactics.team_tempo,
            new_tactics.pressing_intensity,
            new_tactics.team_width,
        );
    }

    /// 포메이션에 따른 기본 오프셋 생성
    fn generate_formation_offsets(formation: &str) -> Vec<FormationOffset> {
        // 기본 4-4-2 오프셋 (미터 단위, 팀 중심 기준)

        match formation {
            "4-4-2" => vec![
                // GK
                FormationOffset { x: -40.0, y: 0.0, line: PositionLine::Goalkeeper },
                // 수비 라인 (RB, CB, CB, LB)
                FormationOffset { x: 0.0, y: -25.0, line: PositionLine::Defender },
                FormationOffset { x: 0.0, y: -8.0, line: PositionLine::Defender },
                FormationOffset { x: 0.0, y: 8.0, line: PositionLine::Defender },
                FormationOffset { x: 0.0, y: 25.0, line: PositionLine::Defender },
                // 미드필더 라인 (RM, CM, CM, LM)
                FormationOffset { x: 0.0, y: -28.0, line: PositionLine::Midfielder },
                FormationOffset { x: 0.0, y: -10.0, line: PositionLine::Midfielder },
                FormationOffset { x: 0.0, y: 10.0, line: PositionLine::Midfielder },
                FormationOffset { x: 0.0, y: 28.0, line: PositionLine::Midfielder },
                // 공격 라인 (ST, ST)
                FormationOffset { x: 0.0, y: -12.0, line: PositionLine::Forward },
                FormationOffset { x: 0.0, y: 12.0, line: PositionLine::Forward },
            ],
            "4-3-3" => vec![
                // GK
                FormationOffset { x: -40.0, y: 0.0, line: PositionLine::Goalkeeper },
                // 수비 라인
                FormationOffset { x: 0.0, y: -25.0, line: PositionLine::Defender },
                FormationOffset { x: 0.0, y: -8.0, line: PositionLine::Defender },
                FormationOffset { x: 0.0, y: 8.0, line: PositionLine::Defender },
                FormationOffset { x: 0.0, y: 25.0, line: PositionLine::Defender },
                // 미드필더 라인 (3명)
                FormationOffset { x: 0.0, y: -15.0, line: PositionLine::Midfielder },
                FormationOffset { x: 0.0, y: 0.0, line: PositionLine::Midfielder },
                FormationOffset { x: 0.0, y: 15.0, line: PositionLine::Midfielder },
                // 공격 라인 (LW, ST, RW) - 미드필더 슬롯 8,9,10
                FormationOffset { x: 0.0, y: -28.0, line: PositionLine::Forward },
                FormationOffset { x: 0.0, y: 0.0, line: PositionLine::Forward },
                FormationOffset { x: 0.0, y: 28.0, line: PositionLine::Forward },
            ],
            _ => {
                // 기본: 4-4-2
                vec![
                    FormationOffset { x: -40.0, y: 0.0, line: PositionLine::Goalkeeper },
                    FormationOffset { x: 0.0, y: -25.0, line: PositionLine::Defender },
                    FormationOffset { x: 0.0, y: -8.0, line: PositionLine::Defender },
                    FormationOffset { x: 0.0, y: 8.0, line: PositionLine::Defender },
                    FormationOffset { x: 0.0, y: 25.0, line: PositionLine::Defender },
                    FormationOffset { x: 0.0, y: -28.0, line: PositionLine::Midfielder },
                    FormationOffset { x: 0.0, y: -10.0, line: PositionLine::Midfielder },
                    FormationOffset { x: 0.0, y: 10.0, line: PositionLine::Midfielder },
                    FormationOffset { x: 0.0, y: 28.0, line: PositionLine::Midfielder },
                    FormationOffset { x: 0.0, y: -12.0, line: PositionLine::Forward },
                    FormationOffset { x: 0.0, y: 12.0, line: PositionLine::Forward },
                ]
            }
        }
    }

    /// Populate per-player instructions with defaults if none provided.
    fn populate_default_player_instructions(
        map: &mut std::collections::HashMap<String, PlayerInstructions>,
        team: &Team,
    ) {
        if !map.is_empty() {
            return;
        }
        // Include substitutes too so late subs still get default instructions.
        for player in &team.players {
            map.entry(player.name.clone()).or_default();
        }
    }

    /// Emit event with automatic timestamp_ms synchronization
    /// P2: Ensures events are properly synchronized with position_data
    /// FIX_2601/0113: ball_position을 Coord10 단위로 통일 (항상 덮어씀)
    pub(crate) fn emit_event(&mut self, event: MatchEvent) {
        // C7: Event SSOT - All events must have track_id set by engine (no fallback)
        // After C6, all event constructors set track_id directly

        // FIX_2601/0113: Coord10 단위로 ball_position 항상 설정
        // - x, y: Coord10 단위 (0-1050, 0-680)
        // - z: 높이 (미터 단위)
        let pos = self.ball.position;
        let event_with_position = event.with_ball_position((
            pos.x as f32,
            pos.y as f32,
            self.ball.height as f32 / 10.0,
        ));

        let event_with_timestamp = event_with_position.with_timestamp(self.current_timestamp_ms);

        // VAR v0: emit an informational review event for high-impact decisions.
        // (No overturn yet; review adds stoppage time.)
        let var_payload = if Self::var_enabled()
            && Self::should_trigger_var_review(&event_with_timestamp.event_type)
        {
            Some((
                event_with_timestamp.minute,
                event_with_timestamp.is_home_team,
                event_with_timestamp.player_track_id,
                event_with_timestamp.event_type.clone(),
            ))
        } else {
            None
        };

        self.maybe_accumulate_stoppage_time(&event_with_timestamp);
        self.result.events.push(event_with_timestamp);

        if let Some((minute, is_home_team, player_track_id, reviewed_event_type)) = var_payload {
            let timestamp_ms = self.current_timestamp_ms;
            self.emit_event(MatchEvent::var_review(
                minute,
                timestamp_ms,
                is_home_team,
                player_track_id,
                reviewed_event_type,
                crate::models::VarReviewOutcome::Upheld,
            ));
        }
    }

    #[inline]
    fn regulation_end_minute(&self) -> u8 {
        self.first_half_end_minute.saturating_add(HALF_DURATION_MINUTES)
    }

    fn maybe_accumulate_stoppage_time(&mut self, event: &MatchEvent) {
        if self.is_second_half {
            self.maybe_accumulate_second_half_stoppage_time(event);
        } else {
            self.maybe_accumulate_first_half_stoppage_time(event);
        }
    }

    fn stoppage_seconds_for_event(event_type: &EventType) -> u16 {
        match event_type {
            EventType::Substitution => STOPPAGE_SECONDS_SUBSTITUTION,
            EventType::Injury => STOPPAGE_SECONDS_INJURY,
            EventType::Goal | EventType::OwnGoal => STOPPAGE_SECONDS_GOAL,
            EventType::YellowCard | EventType::RedCard => STOPPAGE_SECONDS_CARD,
            EventType::VarReview => STOPPAGE_SECONDS_VAR_REVIEW,
            _ => 0,
        }
    }

    fn maybe_accumulate_first_half_stoppage_time(&mut self, event: &MatchEvent) {
        // v1.1: finalize added time once at minute 45, do not keep accumulating
        // during the added minutes.
        if self.is_second_half
            || self.stoppage_finalized_first_half
            || self.minute >= HALF_DURATION_MINUTES
        {
            return;
        }

        let seconds = Self::stoppage_seconds_for_event(&event.event_type);
        if seconds == 0 {
            return;
        }

        self.stoppage_seconds_first_half =
            self.stoppage_seconds_first_half.saturating_add(seconds);
    }

    fn maybe_accumulate_second_half_stoppage_time(&mut self, event: &MatchEvent) {
        // v1.1: finalize added time once at regulation end, do not keep
        // accumulating during the added minutes.
        if !self.is_second_half
            || self.stoppage_finalized_second_half
            || self.minute >= self.regulation_end_minute()
        {
            return;
        }

        let seconds = Self::stoppage_seconds_for_event(&event.event_type);

        if seconds == 0 {
            return;
        }

        self.stoppage_seconds_second_half = self.stoppage_seconds_second_half.saturating_add(seconds);
    }

    fn maybe_finalize_first_half_stoppage_time(&mut self) {
        if self.stoppage_finalized_first_half || self.minute < HALF_DURATION_MINUTES {
            return;
        }

        let added_minutes = ((self.stoppage_seconds_first_half as u32 + 59) / 60) as u8;
        let added_minutes = added_minutes.min(STOPPAGE_TIME_MAX_MINUTES);

        self.first_half_end_minute = HALF_DURATION_MINUTES.saturating_add(added_minutes);
        self.stoppage_finalized_first_half = true;

        // Until 2H added time is finalized, match_end_minute tracks regulation end.
        if !self.stoppage_finalized_second_half {
            self.match_end_minute = self.regulation_end_minute();
        }
    }

    fn maybe_finalize_second_half_stoppage_time(&mut self) {
        let regulation_end_minute = self.regulation_end_minute();
        if self.stoppage_finalized_second_half || self.minute < regulation_end_minute {
            return;
        }

        let added_minutes = ((self.stoppage_seconds_second_half as u32 + 59) / 60) as u8;
        let added_minutes = added_minutes.min(STOPPAGE_TIME_MAX_MINUTES);

        self.match_end_minute = regulation_end_minute.saturating_add(added_minutes);
        self.stoppage_finalized_second_half = true;
    }

    fn var_enabled() -> bool {
        let Ok(value) = std::env::var("OF_ALLOW_VAR") else {
            return false;
        };
        matches!(value.as_str(), "1" | "true" | "TRUE" | "True")
    }

    fn should_trigger_var_review(event_type: &EventType) -> bool {
        matches!(
            event_type,
            EventType::Goal | EventType::OwnGoal | EventType::Penalty | EventType::RedCard
        )
    }

    fn penalty_shootout_enabled() -> bool {
        let Ok(value) = std::env::var("OF_ALLOW_PENALTY_SHOOTOUT") else {
            return false;
        };
        matches!(value.as_str(), "1" | "true" | "TRUE" | "True")
    }

    /// Optional penalty shootout after a draw (regulation score remains unchanged).
    fn maybe_run_penalty_shootout(&mut self) {
        if self.result.penalty_shootout.is_some() {
            return;
        }
        if self.result.score_home != self.result.score_away {
            return;
        }
        if !Self::penalty_shootout_enabled() {
            return;
        }

        self.result.penalty_shootout = Some(self.simulate_penalty_shootout());
    }

    fn simulate_penalty_shootout(&mut self) -> crate::models::match_result::PenaltyShootoutResult {
        use rand::Rng;
        use crate::engine::player_state::PlayerState;
        use crate::models::match_result::PenaltyShootoutKick;

        let collect_kickers = |is_home: bool| -> Vec<usize> {
            let (start, end) = if is_home { (0, 11) } else { (11, 22) };
            let mut kickers = Vec::new();
            for track_id in start..end {
                match self.player_states.get(track_id) {
                    Some(PlayerState::SentOff) | Some(PlayerState::Injured) => continue,
                    Some(_) => kickers.push(track_id),
                    None => {}
                }
            }
            if kickers.is_empty() {
                kickers.push(start);
            }
            kickers
        };

        let find_goalkeeper = |is_home: bool| -> usize {
            let (start, end) = if is_home { (0, 11) } else { (11, 22) };
            for track_id in start..end {
                if self.get_match_player(track_id).position.is_goalkeeper()
                    && !matches!(self.player_states.get(track_id), Some(PlayerState::SentOff))
                {
                    return track_id;
                }
            }
            start
        };

        let home_kickers = collect_kickers(true);
        let away_kickers = collect_kickers(false);
        let home_gk = find_goalkeeper(true);
        let away_gk = find_goalkeeper(false);

        let mut next_home = 0usize;
        let mut next_away = 0usize;

        let mut kicks_home: u8 = 0;
        let mut kicks_away: u8 = 0;
        let mut goals_home: u8 = 0;
        let mut goals_away: u8 = 0;
        let mut kicks: Vec<PenaltyShootoutKick> = Vec::new();

        let mut take_kick = |is_home_team: bool,
                             kicker_track_id: usize,
                             keeper_track_id: usize,
                             kick_log: &mut Vec<PenaltyShootoutKick>|
         -> bool {
            let (kicker_overall, kicker_name) = {
                let kicker = self.get_match_player(kicker_track_id);
                (kicker.overall as i32, kicker.name.clone())
            };
            let keeper_overall = self.get_match_player(keeper_track_id).overall as i32;
            let diff = kicker_overall - keeper_overall;
            let p_goal = (0.75 + diff as f32 * 0.002).clamp(0.55, 0.92);
            let scored = self.rng.gen::<f32>() < p_goal;
            let kick_index = (kick_log.len() + 1) as u8;
            kick_log.push(PenaltyShootoutKick {
                kick_index,
                is_home_team,
                kicker_track_id: kicker_track_id as u8,
                kicker_name,
                scored,
            });
            scored
        };

        // Initial 5 kicks each (early termination allowed).
        'initial: for _round in 0..5 {
            // Home kick
            kicks_home = kicks_home.saturating_add(1);
            let kicker = home_kickers[next_home % home_kickers.len()];
            next_home += 1;
            if take_kick(true, kicker, away_gk, &mut kicks) {
                goals_home = goals_home.saturating_add(1);
            }

            let away_remaining = 5u8.saturating_sub(kicks_away);
            if goals_home > goals_away.saturating_add(away_remaining) {
                break 'initial;
            }

            // Away kick
            kicks_away = kicks_away.saturating_add(1);
            let kicker = away_kickers[next_away % away_kickers.len()];
            next_away += 1;
            if take_kick(false, kicker, home_gk, &mut kicks) {
                goals_away = goals_away.saturating_add(1);
            }

            let home_remaining = 5u8.saturating_sub(kicks_home);
            if goals_away > goals_home.saturating_add(home_remaining) {
                break 'initial;
            }
        }

        // Sudden death (cap rounds to avoid infinite loops).
        if goals_home == goals_away {
            for _round in 0..10 {
                kicks_home = kicks_home.saturating_add(1);
                let kicker = home_kickers[next_home % home_kickers.len()];
                next_home += 1;
                if take_kick(true, kicker, away_gk, &mut kicks) {
                    goals_home = goals_home.saturating_add(1);
                }

                kicks_away = kicks_away.saturating_add(1);
                let kicker = away_kickers[next_away % away_kickers.len()];
                next_away += 1;
                if take_kick(false, kicker, home_gk, &mut kicks) {
                    goals_away = goals_away.saturating_add(1);
                }

                if goals_home != goals_away {
                    break;
                }
            }
        }

        let winner_is_home = if goals_home == goals_away {
            // Should be extremely rare; break ties deterministically with RNG (seeded).
            self.rng.gen::<bool>()
        } else {
            goals_home > goals_away
        };

        crate::models::match_result::PenaltyShootoutResult {
            goals_home,
            goals_away,
            kicks_taken_home: kicks_home,
            kicks_taken_away: kicks_away,
            winner_is_home,
            kicks,
        }
    }

    /// Initialize player positions based on formation waypoints
    /// FIX_2601 Phase 3.6: Convert normalized positions to Coord10
    fn initialize_player_positions(&mut self) {
        use super::types::Coord10;
        self.player_positions = Vec::with_capacity(22);
        self.base_formations = std::array::from_fn(|_| Vec::with_capacity(11));

        // Initialize all 22 players at their base formation positions
        for idx in 0..22 {
            let base_pos_normalized = self.get_base_position_for_index(idx);
            // Convert normalized (0-1) to Coord10 using legacy axis order
            let coord = Coord10::from_normalized_legacy(base_pos_normalized);
            self.player_positions.push(coord);

            let pos_m = coord.to_meters();
            if idx < 11 {
                self.base_formations[0].push(pos_m);
            } else {
                self.base_formations[1].push(pos_m);
            }
        }
    }

    fn fix01_sync_ssot_proof_after_kickoff_positions(&mut self) {
        // If setup already has the proof, keep it as the source of truth and
        // reattach it to the result if the result was reset (e.g. via builder helpers).
        if let Some(ref proof) = self.setup.debug.ssot_proof {
            if self.result.ssot_proof.formation.formation_layout_hash.is_empty() {
                self.result.ssot_proof = proof.clone();
            }
            return;
        }

        let mut levels_by_track_id: Vec<u8> = Vec::with_capacity(22);
        for track_id in 0..22 {
            levels_by_track_id.push(self.setup.get_player(track_id).condition_level);
        }

        for (track_id, level) in levels_by_track_id.iter().copied().enumerate() {
            if !crate::fix01::is_valid_condition_level(level) {
                panic!(
                    "{}: track_id {} has invalid condition_level {} (expected 1..=5)",
                    crate::fix01::error_codes::INVALID_CONDITION_RANGE,
                    track_id,
                    level
                );
            }
        }

        let mut proof = crate::fix01::build_ssot_proof_pre_kickoff(
            &self.setup.home.formation,
            &self.setup.away.formation,
            &levels_by_track_id,
        )
        .expect("FIX01: failed to build ssot_proof");

        crate::fix01::set_formation_layout_hash_from_positions(
            &mut proof,
            self.player_positions.as_slice(),
        )
        .expect("FIX01: failed to compute formation_layout_hash");

        self.setup.debug.ssot_proof = Some(proof.clone());
        self.result.ssot_proof = proof;
    }

    /// Get base formation position for player index (without dynamic adjustments)
    fn get_base_position_for_index(&self, idx: usize) -> (f32, f32) {
        let is_home = TeamSide::is_home(idx);
        let (slot, formation) = if is_home {
            (idx, &self.home_formation)
        } else {
            (TeamSide::local_idx(idx), &self.away_formation)
        };

        let waypoints = get_formation_waypoints(formation);
        let position_key = slot_to_position_key(slot, formation);

        let pos = if let Some(wp) = waypoints.get(&position_key) {
            wp.base
        } else {
            get_fallback_position(slot)
        };

        // FIX_2601/0110: Away team needs BOTH axes flipped (mirror across center)
        // Home attacks right (toward x=1.0), Away attacks left (toward x=0)
        // - Home GK at x=0.04 (near their goal at x=0)
        // - Away GK at x=0.96 (near their goal at x=1)
        if is_home {
            pos
        } else {
            (1.0 - pos.0, 1.0 - pos.1)
        }
    }

    /// Enable position tracking for replay generation
    pub fn with_position_tracking(mut self) -> Self {
        self.track_positions = true;
        self.result = MatchResult::with_replay_events();
        self
    }

    /// Enable replay recording for generating ReplayDoc with all events
    /// This creates a ReplayRecorder that captures events during simulation
    pub fn with_replay_recording(mut self) -> Self {
        use crate::engine::physics_constants::field;

        // Build rosters from teams
        let home_roster = self.build_replay_roster(&self.home_team, true);
        let away_roster = self.build_replay_roster(&self.away_team, false);

        let rosters = ReplayRosters { home: home_roster, away: away_roster };

        let pitch = PitchSpec {
            width_m: field::LENGTH_M as f64,
            height_m: field::WIDTH_M as f64,
        };
        self.replay_recorder = Some(ReplayRecorder::new(pitch, rosters));
        self
    }

    /// Build ReplayRoster from Team
    fn build_replay_roster(&self, team: &Team, _is_home: bool) -> ReplayRoster {
        let players: Vec<ReplayPlayer> = team
            .players
            .iter()
            .take(11)
            .enumerate()
            .map(|(idx, p)| ReplayPlayer {
                id: idx as u32,
                name: p.name.clone(),
                position: format!("{:?}", p.position),
                ca: p.overall as u32,
                condition: 1.0, // Full condition at start
                appearance: None,
            })
            .collect();

        ReplayRoster { name: team.name.clone(), players }
    }

    /// Take the replay document after simulation (consumes the recorder)
    pub fn take_replay_doc(&mut self) -> Option<ReplayDoc> {
        self.replay_recorder.take().map(|r| r.into_doc(1))
    }

    /// Get reference to replay recorder (for adding events during simulation)
    pub(crate) fn replay_recorder_mut(&mut self) -> Option<&mut ReplayRecorder> {
        self.replay_recorder.as_mut()
    }

    // ========== FIX_2512 Phase 3: Replay v2 Methods ==========

    /// Enable Replay v2 recording (Coord10-based format)
    ///
    /// Creates a ReplayWriterV2 with proper metadata
    pub fn with_replay_v2_recording(mut self, seed: u64) -> Self {
        use crate::engine::types::coord10::Coord10;
        use crate::replay::{MatchInfoV2, ReplayMetaV2, ReplayWriterV2};

        let meta = ReplayMetaV2 {
            coord_unit_mm: 100,                    // 0.1m unit
            sim_tick_ms: 50,                       // 50ms simulation tick
            view_tick_ms: 50,                      // 50ms viewer playback
            save_tick_ms: 100,                     // 100ms save interval
            field_x_max: Coord10::FIELD_LENGTH_10, // 1050 = 105.0m
            field_y_max: Coord10::FIELD_WIDTH_10,  // 680 = 68.0m
            track_count: 23,                       // 1 ball + 22 players
            match_info: MatchInfoV2 { seed, score_home: 0, score_away: 0, duration_minutes: 90 },
        };

        self.replay_writer_v2 = Some(ReplayWriterV2::new(meta));
        self
    }

    /// Take the Replay v2 after simulation
    ///
    /// Finalizes the replay and returns the ReplayV2 structure.
    /// Sets the final score before returning.
    pub fn take_replay_v2(&mut self) -> Option<crate::replay::ReplayV2> {
        self.replay_writer_v2.take().map(|mut writer| {
            writer.set_final_score(self.result.score_home, self.result.score_away);
            writer.finalize()
        })
    }

    /// Get mutable reference to ReplayWriter v2 (for internal use)
    pub(crate) fn replay_writer_v2_mut(&mut self) -> Option<&mut crate::replay::ReplayWriterV2> {
        self.replay_writer_v2.as_mut()
    }

    // ========== FIX_2601/0108: UAE Pipeline Configuration ==========

    /// Enable UAE (Unified Action Evaluator) pipeline
    ///
    /// When enabled, uses the new 6-factor evaluation system instead of
    /// the legacy decision_topology.rs EV system.
    ///
    /// # Example
    /// ```ignore
    /// let engine = MatchEngine::new(plan)?.with_uae_pipeline(true);
    /// ```
    pub fn with_uae_pipeline(mut self, enabled: bool) -> Self {
        self.use_uae_pipeline = enabled;
        self
    }

    /// Check if UAE pipeline is enabled
    pub fn is_uae_pipeline_enabled(&self) -> bool {
        self.use_uae_pipeline
    }

    // ========== DPER Framework: Experimental Configuration ==========

    /// Apply experimental configuration for A/B testing
    ///
    /// The ExpConfig parameters override default decision thresholds,
    /// audacity modifiers, and play style biases.
    ///
    /// # Example
    /// ```ignore
    /// let config = ExpConfig::load("experiments/aggressive_v1.json")?;
    /// let engine = MatchEngine::new(plan)?.with_exp_config(&config);
    /// ```
    pub fn with_exp_config(mut self, config: &crate::engine::experimental::ExpConfig) -> Self {
        self.exp_params = Some(crate::engine::experimental::RuntimeExpParams::from(config));
        self
    }

    /// Apply experimental configuration (mutable version)
    ///
    /// Use this when you already have a MatchEngine instance and want to
    /// apply experimental parameters before running simulation.
    pub fn apply_exp_config(&mut self, config: &crate::engine::experimental::ExpConfig) {
        self.exp_params = Some(crate::engine::experimental::RuntimeExpParams::from(config));
    }

    /// Clear experimental configuration (revert to stable baseline)
    pub fn clear_exp_config(&mut self) {
        self.exp_params = None;
    }

    /// Check if experimental configuration is applied
    pub fn has_exp_config(&self) -> bool {
        self.exp_params.is_some()
    }

    // ========================================
    // FIX_2601/0112: Calibrator Integration
    // ========================================

    /// Apply calibrator parameters (builder pattern)
    ///
    /// Calibrator params affect action probabilities and attempt rates
    /// based on statistical anchor targets.
    pub fn with_calibrator_params(mut self, params: CalibratorParams) -> Self {
        self.calibrator_params = Some(params);
        self
    }

    /// Apply calibrator parameters (mutable version)
    pub fn apply_calibrator_params(&mut self, params: CalibratorParams) {
        self.calibrator_params = Some(params);
    }

    /// Clear calibrator parameters
    pub fn clear_calibrator_params(&mut self) {
        self.calibrator_params = None;
    }

    /// Check if calibrator parameters are applied
    pub fn has_calibrator_params(&self) -> bool {
        self.calibrator_params.is_some()
    }

    /// Get calibrator parameters reference (if set)
    pub fn get_calibrator_params(&self) -> Option<&CalibratorParams> {
        self.calibrator_params.as_ref()
    }

    /// Get current experimental parameters (or baseline if none applied)
    pub fn get_exp_params(&self) -> crate::engine::experimental::RuntimeExpParams {
        self.exp_params.clone().unwrap_or_else(crate::engine::experimental::RuntimeExpParams::baseline)
    }

    /// Get shoot xG threshold (from exp_config or default)
    #[inline]
    pub fn exp_shoot_xg_threshold(&self) -> f32 {
        self.exp_params.as_ref().map_or(0.05, |p| p.shoot_xg_threshold)
    }

    /// Get pass risk tolerance (from exp_config or default)
    #[inline]
    pub fn exp_pass_risk_tolerance(&self) -> f32 {
        self.exp_params.as_ref().map_or(0.3, |p| p.pass_risk_tolerance)
    }

    /// Get dribble bias (from exp_config or default)
    #[inline]
    pub fn exp_dribble_bias(&self) -> f32 {
        self.exp_params.as_ref().map_or(0.0, |p| p.dribble_bias)
    }

    /// Get through ball multiplier (from exp_config or default)
    #[inline]
    pub fn exp_through_ball_multiplier(&self) -> f32 {
        self.exp_params.as_ref().map_or(1.0, |p| p.through_ball_multiplier)
    }

    /// Get cross multiplier (from exp_config or default)
    #[inline]
    pub fn exp_cross_multiplier(&self) -> f32 {
        self.exp_params.as_ref().map_or(1.0, |p| p.cross_multiplier)
    }

    /// Get audacity scale (from exp_config or default)
    #[inline]
    pub fn exp_audacity_scale(&self) -> f32 {
        self.exp_params.as_ref().map_or(1.0, |p| p.audacity_scale)
    }

    /// Get audacity losing boost (from exp_config or default)
    #[inline]
    pub fn exp_audacity_losing_boost(&self) -> f32 {
        self.exp_params.as_ref().map_or(0.1, |p| p.audacity_losing_boost)
    }

    /// Get audacity late game urgency (from exp_config or default)
    #[inline]
    pub fn exp_audacity_late_game_urgency(&self) -> f32 {
        self.exp_params.as_ref().map_or(1.2, |p| p.audacity_late_game_urgency)
    }

    /// Get tempo bias (from exp_config or default)
    #[inline]
    pub fn exp_tempo_bias(&self) -> f32 {
        self.exp_params.as_ref().map_or(0.0, |p| p.tempo_bias)
    }

    /// Get width bias (from exp_config or default)
    #[inline]
    pub fn exp_width_bias(&self) -> f32 {
        self.exp_params.as_ref().map_or(0.0, |p| p.width_bias)
    }

    /// Get directness bias (from exp_config or default)
    #[inline]
    pub fn exp_directness_bias(&self) -> f32 {
        self.exp_params.as_ref().map_or(0.0, |p| p.directness_bias)
    }

    // ========== FIX_2601/0112: Calibration Bias Getters ==========

    /// Get progressive pass attempt bias from calibrator params
    #[inline]
    pub fn cal_progressive_pass_bias(&self) -> f32 {
        self.calibrator_params.as_ref()
            .map_or(1.0, |p| p.action_attempt_bias.progressive_pass_attempt)
    }

    /// Get long pass attempt bias from calibrator params
    #[inline]
    pub fn cal_long_pass_bias(&self) -> f32 {
        self.calibrator_params.as_ref()
            .map_or(1.0, |p| p.action_attempt_bias.long_pass_attempt)
    }

    /// Get cross attempt bias from calibrator params
    #[inline]
    pub fn cal_cross_bias(&self) -> f32 {
        self.calibrator_params.as_ref()
            .map_or(1.0, |p| p.action_attempt_bias.cross_attempt)
    }

    /// Get shot attempt bias from calibrator params
    #[inline]
    pub fn cal_shot_bias(&self) -> f32 {
        self.calibrator_params.as_ref()
            .map_or(1.0, |p| p.action_attempt_bias.shot_attempt)
    }

    /// Get dribble attempt bias from calibrator params
    #[inline]
    pub fn cal_dribble_bias(&self) -> f32 {
        self.calibrator_params.as_ref()
            .map_or(1.0, |p| p.action_attempt_bias.dribble_attempt)
    }

    /// Get through ball attempt bias from calibrator params
    #[inline]
    pub fn cal_through_ball_bias(&self) -> f32 {
        self.calibrator_params.as_ref()
            .map_or(1.0, |p| p.action_attempt_bias.through_ball_attempt)
    }

    // ========== P0: Goal Contract Helpers ==========

    /// 팀이 지켜야 할 골대 (실점하면 안 되는 골대)
    ///
    /// Home 팀 → Home 골대 (x=0)
    /// Away 팀 → Away 골대 (x=105)
    pub fn defending_goal(&self, team: super::TeamSide) -> &Goal {
        self.goals.defending_goal(team)
    }

    /// 팀이 공격해야 할 골대 (득점해야 하는 골대)
    ///
    /// Home 팀 → Away 골대 (x=105)
    /// Away 팀 → Home 골대 (x=0)
    pub fn attacking_goal(&self, team: super::TeamSide) -> &Goal {
        self.goals.attacking_goal(team)
    }

    /// 선수가 공격해야 할 골대
    ///
    /// 선수 0~10: Home 팀 → Away 골대 (x=105)
    /// 선수 11~21: Away 팀 → Home 골대 (x=0)
    pub fn attacking_goal_for_player(&self, player_idx: usize) -> &Goal {
        self.goals.attacking_goal_for_player(player_idx)
    }

    /// 선수가 지켜야 할 골대
    ///
    /// 선수 0~10: Home 팀 → Home 골대 (x=0)
    /// 선수 11~21: Away 팀 → Away 골대 (x=105)
    pub fn defending_goal_for_player(&self, player_idx: usize) -> &Goal {
        self.goals.defending_goal_for_player(player_idx)
    }

    /// 공 위치로 골 체크 - 어느 팀이 득점했는지 반환
    ///
    /// Returns: Some(scoring_team) if goal scored, None otherwise
    pub fn check_goal_from_ball(
        &self,
        ball_pos: (f32, f32),
        ball_height: f32,
    ) -> Option<super::TeamSide> {
        self.goals.check_goal(ball_pos, ball_height)
    }

    /// Goals 구조체 참조 반환
    pub fn goals(&self) -> &Goals {
        &self.goals
    }

    // ========== P17: MatchSetup Delegate Methods ==========

    /// P17: track_id로 선수 정보 조회 (MatchSetup delegate)
    ///
    /// # Arguments
    /// * `track_id` - 선수 인덱스 (0-10: Home, 11-21: Away)
    ///
    /// # Returns
    /// 해당 선수의 MatchPlayer 참조
    #[inline]
    pub fn get_match_player(&self, track_id: usize) -> &crate::models::MatchPlayer {
        self.setup.get_player(track_id)
    }

    /// P17: track_id로 팀 판별 (Home/Away)
    ///
    /// # Arguments
    /// * `track_id` - 선수 인덱스 (0-10: Home, 11-21: Away)
    ///
    /// # Returns
    /// TeamSide::Home 또는 TeamSide::Away
    #[inline]
    pub fn team_side(&self, track_id: usize) -> crate::models::TeamSide {
        self.setup.get_team_side(track_id)
    }

    /// P17: track_id로 능력치 조회 (MatchSetup delegate)
    #[inline]
    pub fn get_player_attributes(
        &self,
        track_id: usize,
    ) -> &crate::models::player::PlayerAttributes {
        self.setup.get_attributes(track_id)
    }

    /// P17: track_id로 트레이트 조회 (MatchSetup delegate)
    #[inline]
    pub fn get_player_traits(&self, track_id: usize) -> &crate::models::trait_system::TraitSlots {
        self.setup.get_traits(track_id)
    }

    /// P1: ?�수?PlayerInstructions 조회
    /// - home/away_player_instructions HashMap?�서 ?�수 ?�름?�로 조회
    /// - ?�으?Default 반환
    fn get_player_instructions(&self, player_idx: usize) -> PlayerInstructions {
        let is_home = TeamSide::is_home(player_idx);
        let player_name = &self.get_match_player(player_idx).name;

        let instructions_map =
            if is_home { &self.home_player_instructions } else { &self.away_player_instructions };

        instructions_map.get(player_name).cloned().unwrap_or_default()
    }

    /// P2: player_idx로�???position 문자??추출
    fn get_position_string_by_idx(&self, player_idx: usize) -> String {
        self.get_position_string(&self.get_match_player(player_idx).position)
    }

    pub fn simulate(&mut self) -> MatchResult {
        println!("[MatchEngine] simulate() CALLED - START");
        let _ = io::stdout().flush();

        // FIX_2601/0113 v1.1: DPQ scheduler must start from a clean state for
        // batch simulation flows as well.
        self.decision_scheduler.reset();

        // Always initialize player positions (simulation logic needs them)
        self.initialize_player_positions();

        // P15: 선수별 관성 물리 파라미터 초기화 (스탯 기반)
        self.init_player_motion_params();

        // Pre-match calculations
        let home_strength = self.calculate_team_strength(&self.home_team, true);
        let away_strength = self.calculate_team_strength(&self.away_team, false);

        // Calculate possession based on team strengths
        let possession_ratio = self.calculate_possession(home_strength, away_strength);

        // ========== ENGINE CONTRACT: Initial Snapshot (Kickoff State) ==========
        // Per ENGINE_CONTRACT.md Section 1.2:
        // The engine MUST record a t=0 snapshot immediately after initialization.
        // This ensures replay always starts from the same kickoff formation.
        //
        // FIX_2601/1120: Apply kickoff positioning rules BEFORE recording snapshot
        // This is the SSOT for kickoff positions - enforces IFAB Laws of the Game:
        // - All players in own half
        // - Receiving team outside center circle
        // - Max 2 kickers from kicking team inside center circle
        let home_kicks_off = true;  // Home team kicks off first half (standard)
        self.apply_kickoff_positions(home_kicks_off);
        self.fix01_sync_ssot_proof_after_kickoff_positions();
        self.record_initial_kickoff_state(home_kicks_off);

        // FIX_2601/0106 P2: Reset added-time state for batch flow.
        self.stoppage_seconds_first_half = 0;
        self.stoppage_finalized_first_half = false;
        self.first_half_end_minute = HALF_DURATION_MINUTES;
        self.stoppage_seconds_second_half = 0;
        self.stoppage_finalized_second_half = false;
        self.match_end_minute = REGULATION_TOTAL_MINUTES;

        // Use a stable upper bound; the actual end minute is decided at regulation boundaries.
        let match_duration = MATCH_DURATION_CAP_MINUTES;

        for minute in 0..=match_duration {
            self.minute = minute;

            // 1H added time is finalized at minute 45.
            if !self.is_second_half && minute == HALF_DURATION_MINUTES {
                self.maybe_finalize_first_half_stoppage_time();
            }

            // Half-time boundary (second half kickoff).
            if !self.is_second_half && minute == self.first_half_end_minute {
                self.handle_half_time();
            }

            // Decide 2H added time at regulation end minute.
            if self.is_second_half && minute == self.regulation_end_minute() {
                self.maybe_finalize_second_half_stoppage_time();
            }

            // Stop once we pass the finalized match end minute.
            if minute > self.match_end_minute {
                break;
            }

            self.simulate_minute(home_strength, away_strength, possession_ratio);

            // 2025-12-11: record_positions_for_minute() 호출 제거
            // tick_based 엔진이 record_positions_for_tick()으로 직접 위치를 기록함
            // 레거시 record_positions_for_minute()는 위치를 재계산해서 엔진 결과를 덮어쓰는 문제가 있었음
        }

        if !self.result.events.iter().any(|event| matches!(event.event_type, EventType::FullTime)) {
            let match_end_minute = self.match_end_minute;
            let timestamp_ms = match_end_minute as u64 * 60_000;
            self.current_timestamp_ms = timestamp_ms;
            self.emit_event(MatchEvent::full_time(match_end_minute, timestamp_ms));
        }

        self.finalize_pass_sequences();

        // Calculate final statistics
        self.stats_calculator.finalize(&mut self.result, possession_ratio);

        // FIX02: Determinism SSOT metadata (full simulation path).
        self.result.determinism.mode = crate::models::DeterminismMode::Full;
        self.result.determinism.simulated_until_tick = self.result.statistics.total_ticks;
        self.result.determinism.cut_reason = None;

        // Aggregate per-match stats for the configured user player (MyPlayer)
        if let Some(user_stats) = self.build_user_player_stats() {
            self.result.statistics.my_player_stats = Some(user_stats);
        }

        // Filter events based on what user wants to see
        if self.user_player.is_some() {
            self.filter_events_for_display();
        }

        // Sort events by minute
        self.result.events.sort_by_key(|e| e.minute);

        // Convert to detailed replay events if tracking enabled
        println!("?��?��?�� [MatchEngine] DEBUG: track_positions={}, replay_events.is_some={}, events={} ?��?��?��",
                 self.track_positions, self.result.replay_events.is_some(), self.result.events.len());
        let _ = io::stdout().flush();
        if self.result.replay_events.is_some() {
            // Extract player rosters (first 11 players are starting lineup)
            let home_roster: Vec<String> =
                self.home_team.players.iter().take(11).map(|p| p.name.clone()).collect();

            let away_roster: Vec<String> =
                self.away_team.players.iter().take(11).map(|p| p.name.clone()).collect();

            let mut converter = ReplayConverter::with_rosters(
                self.home_team.formation.clone(),
                self.away_team.formation.clone(),
                home_roster,
                away_roster,
            );
            match converter.convert_events(&self.result) {
                Ok(replay_events) => {
                    let event_count = replay_events.len();
                    self.result.replay_events = Some(replay_events);
                    println!(
                        "?��?��?�� [MatchEngine] Generated {} replay events ?��?��?��",
                        event_count
                    );
                    let _ = io::stdout().flush();
                }
                Err(e) => {
                    println!(
                        "?��?��?�� [MatchEngine] Warning: Failed to convert replay events: {} ?��?��?��",
                        e
                    );
                    let _ = io::stdout().flush();
                    self.result.replay_events = Some(Vec::new());
                }
            }
        }

        // Store teams for roster information
        self.result.home_team = Some(self.home_team.clone());
        self.result.away_team = Some(self.away_team.clone());

        // P17 Phase 5: Store match setup for viewer (starting lineup snapshot).
        self.result.match_setup = Some(self.setup.to_export_starting_lineup());

        // Add debug info
        self.result.debug_info = Some(format!(
            "simulate() called | track_positions={} | replay_events={} | events={} | teams_stored=true",
            self.track_positions,
            self.result.replay_events.as_ref().map(|r| r.len()).unwrap_or(0),
            self.result.events.len()
        ));

        self.maybe_run_penalty_shootout();

        // Generate match summary for quick display on result screens
        self.result.generate_summary();

        // P18: Board summary (final snapshot of occupancy/pressure)
        if let Some(ref board) = self.field_board {
            self.result.board_summary = Some(board.to_summary_export(5));
        }

        // Phase 0: Minimal diagnostics summary (single-run)
        self.balance_diagnostics.print_phase0_summary();
        if std::env::var("OF_BALANCE_REPORT").is_ok() {
            self.balance_diagnostics.print_report();
        }

        // FIX_2601/0112: Output calibration snapshots for debugging
        if std::env::var("OF_DEBUG_CALIBRATION").is_ok() {
            println!("=== Home Team Calibration Snapshot ===");
            println!("  Pass attempts: {}, successes: {}",
                self.home_stat_snapshot.pass_attempts,
                self.home_stat_snapshot.pass_successes);
            println!("  Progressive: {}, Long: {}, Cross: {}",
                self.home_stat_snapshot.progressive_passes,
                self.home_stat_snapshot.long_passes,
                self.home_stat_snapshot.crosses);
            println!("  Shots: {}, On-target: {}, Goals: {}",
                self.home_stat_snapshot.shot_attempts,
                self.home_stat_snapshot.shots_on_target,
                self.home_stat_snapshot.goals);
            println!("  Tackles: {}, Intercepts: {}",
                self.home_stat_snapshot.tackles,
                self.home_stat_snapshot.interceptions);
            println!("  Touches by zone: {:?}", self.home_stat_snapshot.touches_by_zone);

            println!("=== Away Team Calibration Snapshot ===");
            println!("  Pass attempts: {}, successes: {}",
                self.away_stat_snapshot.pass_attempts,
                self.away_stat_snapshot.pass_successes);
            println!("  Progressive: {}, Long: {}, Cross: {}",
                self.away_stat_snapshot.progressive_passes,
                self.away_stat_snapshot.long_passes,
                self.away_stat_snapshot.crosses);
            println!("  Shots: {}, On-target: {}, Goals: {}",
                self.away_stat_snapshot.shot_attempts,
                self.away_stat_snapshot.shots_on_target,
                self.away_stat_snapshot.goals);
            println!("  Tackles: {}, Intercepts: {}",
                self.away_stat_snapshot.tackles,
                self.away_stat_snapshot.interceptions);
            println!("  Touches by zone: {:?}", self.away_stat_snapshot.touches_by_zone);
        }

        // FIX_2601/0120: RNG consumption tracking output
        if std::env::var("OF_DEBUG_RNG").is_ok() {
            self.rng_tracker.print_summary();
        }

        // FIX_2601/1121: Detail completeness tracking output
        if std::env::var("OF_DEBUG_DETAIL").is_ok() {
            self.detail_tracker.print_summary();
        }

        // FIX_2601: Shot opportunity telemetry - move from engine to result
        if let Some(telemetry) = self.shot_opp_telemetry.take() {
            if !telemetry.frames.is_empty() {
                // Print 4-table summary if debug mode
                if std::env::var("OF_DEBUG_SHOT_OPP").is_ok() {
                    telemetry.print_all_tables();
                }
                self.result.shot_opp_telemetry = Some(telemetry);
            }
        }

        // FIX_2601/0109: Use take() instead of clone() to avoid 600KB+ copy
        // Note: Engine cannot be reused after this call
        std::mem::take(&mut self.result)
    }

    // 2025-12-11: record_positions_for_minute() 삭제됨
    // - 레거시 함수: 자체적으로 600틱을 돌면서 위치를 재계산, 엔진 결과를 덮어씀
    // - tick_based.rs의 record_positions_for_tick()으로 대체됨
    // - 엔진이 시뮬레이션하면서 직접 위치를 기록하므로 더 이상 필요 없음

    // ========== FIX_2601/0120: Kickoff Positioning System ==========
    //
    // 문제: 기존 코드는 포메이션 위치를 SSOT로 사용하고 exclusion만 적용
    //       → Away 선수가 Home 진영에 위치 (룰 위반, 91:1 bias 발생)
    //
    // 해결: apply_kickoff_positions()가 SSOT, 축구 규칙 완전 적용
    //       1. 모든 선수 자기 진영(own half) 강제
    //       2. 수신팀 전원 센터서클 밖 투영
    //       3. 킥오프팀 키커 2명만 센터, 나머지 서클 밖

    /// FIX_2601/0120: 킥오프 규칙에 맞게 전원 위치 재배치
    ///
    /// IFAB Laws of the Game 준수:
    /// - 공은 센터 스폿
    /// - 킥오프팀: 센터서클 안에 최대 2명 (키커)
    /// - 수신팀: 전원 센터서클 밖
    /// - 모든 선수: 자기 진영(own half) 안
    fn apply_kickoff_positions(&mut self, kicking_team_is_home: bool) {
        use super::types::Coord10;

        const HALFWAY_X: f32 = 52.5;
        const CENTER_X: f32 = 52.5;
        const CENTER_Y: f32 = 34.0;
        const CIRCLE_RADIUS: f32 = 9.15;
        const MARGIN: f32 = 0.5; // 충돌 방지

        // 1. 키커 2명 선정 (deterministic - 리플레이 재현성 보장)
        let (kicker1_idx, kicker2_idx) = self.select_kickers(kicking_team_is_home);

        // 2. 전원 위치 교정
        for i in 0..22 {
            let mut pos = self.player_positions[i].to_meters();
            let is_home = i < 11;
            let is_kicker = i == kicker1_idx || i == kicker2_idx;

            // 3a. Own Half 강제 (하프라인 클램프) - 핵심 수정!
            pos = self.clamp_to_own_half(pos, is_home);

            // 3b. 센터서클 처리
            if is_kicker {
                // 키커: 센터 근처 고정 위치
                pos = self.kicker_position(i, kicker1_idx, kicker2_idx);
            } else {
                // 비키커: 서클 밖으로 투영
                pos = Self::project_outside_circle(pos, CENTER_X, CENTER_Y, CIRCLE_RADIUS + MARGIN);
            }

            // 3c. 위치 저장
            self.player_positions[i] = Coord10::from_meters(pos.0, pos.1);
        }

        #[cfg(debug_assertions)]
        {
            eprintln!(
                "[KICKOFF_POS] Applied kickoff positions: kicking_team_is_home={}, kickers=({}, {})",
                kicking_team_is_home, kicker1_idx, kicker2_idx
            );
            // 디버그: Away LF 위치 확인 (이전에 x=23m이던 문제)
            let away_lf_pos = self.player_positions[20].to_meters();
            eprintln!(
                "[KICKOFF_POS] Away LF (idx=20) now at x={:.1}m (should be > 52.5m)",
                away_lf_pos.0
            );
        }
    }

    /// FIX_2601/0120: 자기 진영으로 강제 (하프라인 클램프)
    ///
    /// - Home: own half = x < 52.5m
    /// - Away: own half = x > 52.5m
    fn clamp_to_own_half(&self, pos: (f32, f32), is_home: bool) -> (f32, f32) {
        const HALFWAY_X: f32 = 52.5;
        const EPS: f32 = 0.5; // 하프라인에서 약간 떨어지게

        let (x, y) = pos;
        let clamped_x = if is_home {
            // Home: own half = x < 52.5
            x.min(HALFWAY_X - EPS)
        } else {
            // Away: own half = x > 52.5
            x.max(HALFWAY_X + EPS)
        };
        (clamped_x, y)
    }

    /// FIX_2601/0120: 센터서클 밖으로 투영
    fn project_outside_circle(
        pos: (f32, f32),
        center_x: f32,
        center_y: f32,
        min_radius: f32,
    ) -> (f32, f32) {
        let dx = pos.0 - center_x;
        let dy = pos.1 - center_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < min_radius && dist > 0.1 {
            // 서클 외곽으로 투영
            let scale = min_radius / dist;
            (center_x + dx * scale, center_y + dy * scale)
        } else if dist <= 0.1 {
            // 거의 센터에 있으면 자기 진영 방향으로 밀어냄
            (center_x + min_radius, center_y)
        } else {
            pos
        }
    }

    /// FIX_2601/0120: 키커 2명 선정 (deterministic)
    ///
    /// 킥오프팀의 CF + RF/LF 선택 (포지션 기반, 랜덤 금지)
    fn select_kickers(&self, kicking_team_is_home: bool) -> (usize, usize) {
        // 4-4-2 기준: CF(10번 슬롯) + RF(9번 슬롯)
        // 4-3-3 기준: ST(10번 슬롯) + RW/LW(9번 또는 8번 슬롯)
        if kicking_team_is_home {
            (10, 9) // Home CF, Home RF/LF
        } else {
            (21, 20) // Away CF, Away RF/LF
        }
    }

    /// FIX_2601/0120: 키커 고정 위치 (센터 근처)
    fn kicker_position(&self, idx: usize, kicker1: usize, _kicker2: usize) -> (f32, f32) {
        const CENTER_X: f32 = 52.5;
        const CENTER_Y: f32 = 34.0;

        if idx == kicker1 {
            // 주 키커: 정확히 센터
            (CENTER_X, CENTER_Y)
        } else {
            // 보조 키커: 센터 약간 뒤 (1m)
            // Home 킥오프면 왼쪽으로, Away 킥오프면 오른쪽으로
            let is_home_kicker = idx < 11;
            if is_home_kicker {
                (CENTER_X - 1.0, CENTER_Y + 0.5)
            } else {
                (CENTER_X + 1.0, CENTER_Y + 0.5)
            }
        }
    }

    // ========== End of Kickoff Positioning System ==========

    /// Record initial kickoff state (t=0 snapshot)
    ///
    /// Per ENGINE_CONTRACT.md Section 1.2:
    /// The engine MUST record a t=0 snapshot immediately after initialization.
    /// This ensures replay always starts from the same kickoff formation.
    ///
    /// FIX_2601/0120: 리팩토링
    /// - 기존 exclusion 로직 제거 (apply_kickoff_positions()로 이동)
    /// - 이 함수는 공 설정, 시간 초기화, 스냅샷 기록만 담당
    ///
    /// 호출 전 필수: apply_kickoff_positions() 먼저 호출되어야 함
    fn record_initial_kickoff_state(&mut self, home_kicks_off: bool) {
        use crate::engine::physics_constants::field;
        use crate::engine::types::coord10::{Coord10, Vel10};

        // 1. Set ball to center spot
        self.ball.position = Coord10::CENTER;
        self.ball.height = 0;
        self.ball.velocity = Vel10::from_mps(0.0, 0.0);

        // 2. Set kickoff player as ball owner
        let kickoff_player_idx: usize = if home_kicks_off { 10 } else { 21 }; // CF position
        self.ball.current_owner = Some(kickoff_player_idx);

        // 3. Initialize time
        self.minute = 0;
        self.current_tick = 0;
        self.current_timestamp_ms = 0;
        self.last_kickoff_tick = 0;

        // 4. Record initial positions if tracking enabled
        if self.track_positions {
            let ball_pos_m = (field::CENTER_X, field::CENTER_Y);

            use crate::models::PlayerState;
            let player_data: Vec<((f32, f32), PlayerState)> = (0..22)
                .map(|idx| {
                    let pos_norm = self.get_player_position_by_index(idx);
                    let pos_m = pos_norm.to_meters();
                    let is_home_player = TeamSide::is_home(idx);
                    let state = if idx == kickoff_player_idx {
                        PlayerState::WithBall
                    } else if is_home_player == home_kicks_off {
                        PlayerState::Attacking
                    } else {
                        PlayerState::Defending
                    };
                    (pos_m, state)
                })
                .collect();

            if let Some(ref mut pos_data) = self.result.position_data {
                pos_data.add_ball_position_with_velocity(0, ball_pos_m, 0.0, (0.0, 0.0));

                for (idx, (pos_m, state)) in player_data.into_iter().enumerate() {
                    pos_data.add_player_position_with_state(idx as u8, 0, pos_m, state);
                }
            }
        }

        // 5. Emit KickOff event
        self.emit_event(crate::models::MatchEvent::kick_off(0, 0, home_kicks_off));

        println!(
            "[MatchEngine] Initial kickoff state recorded: ball at center, {} owns ball (home_kicks_off={})",
            kickoff_player_idx, home_kicks_off
        );
    }

    /// Convert normalized position (0-1) to meters (105x68)
    ///
    /// Swaps X/Y because normalized uses (width, length) but meters uses (length, width).
    /// See coordinates.rs for coordinate system documentation.
    fn normalized_to_meters(&self, pos: (f32, f32)) -> (f32, f32) {
        use crate::engine::coordinates;
        // P0-2: Use clamped version to prevent out-of-bounds coordinates
        coordinates::to_meters_clamped(pos)
    }

    /// Get player state for replay
    /// FIX_2601/0109: Returns PlayerState enum instead of String
    fn get_player_state(&self, player_idx: usize) -> crate::models::PlayerState {
        use crate::models::PlayerState;
        if Some(player_idx) == self.ball.current_owner {
            PlayerState::WithBall
        } else {
            let is_home = TeamSide::is_home(player_idx);
            let has_possession = if let Some(owner) = self.ball.current_owner {
                TeamSide::is_home(owner) == is_home
            } else {
                false
            };

            if has_possession {
                PlayerState::Attacking
            } else {
                PlayerState::Defending
            }
        }
    }

    // ===========================================
    // P5.6: Half-Time Processing
    // ===========================================

    /// Handle half-time transition
    /// - Emit HalfTime event
    /// - Swap player positions (X-axis flip for side change)
    /// - Reset ball to center
    /// - Change kickoff team to away
    fn handle_half_time(&mut self) {
        // FIX_2601: Mark second half start (affects attack direction)
        self.is_second_half = true;

        // FIX_2601/0105: Swap attack directions for second half
        // Home now attacks left (x=0), Away now attacks right (x=1050)
        self.home_ctx.swap_for_second_half();
        self.away_ctx.swap_for_second_half();

        self.finalize_pass_sequences();

        // Ensure boundary events have tick-aligned timestamps (emit_event uses current_timestamp_ms SSOT).
        self.current_timestamp_ms = self.minute as u64 * 60_000;

        // 1. Emit HalfTime event.
        let half_time_timestamp_ms = self.current_timestamp_ms;
        self.emit_event(crate::models::MatchEvent::half_time(
            self.minute,
            half_time_timestamp_ms,
        ));

        // FIX_2601: Use Coord10/Vel10 for position swap and ball reset
        use crate::engine::types::coord10::{Coord10, Vel10};

        // 2. Swap all player X positions (side change)
        // Players who were attacking right now attack left and vice versa
        // FIX_2601/0116: Also reset velocities - players are stationary after half-time break
        for i in 0..22 {
            let pos = self.player_positions[i];
            // Flip X coordinate: x -> (FIELD_LENGTH_10 - x) in Coord10 units (0.1m)
            self.player_positions[i] = Coord10 { x: Coord10::FIELD_LENGTH_10 - pos.x, y: pos.y, z: pos.z };
            // Reset velocity to prevent leftover momentum causing wrong-direction movement
            self.player_velocities[i] = (0.0, 0.0);
        }

        // FIX_2601/0116: Debug GK positions after half-time flip
        #[cfg(debug_assertions)]
        {
            let home_gk_pos = self.player_positions[0].to_meters();
            let away_gk_pos = self.player_positions[11].to_meters();
            eprintln!(
                "[HALFTIME_FLIP] Home GK (0) now at x={:.1}m, Away GK (11) now at x={:.1}m",
                home_gk_pos.0, away_gk_pos.0
            );
        }

        // FIX_2601/1120: Apply kickoff positioning rules for second half
        // Away team kicks off second half (kicking_team_is_home = false)
        // This enforces IFAB Laws of the Game:
        // - All players in own half (after X-flip)
        // - Home team (receiving) outside center circle
        // - Away team kickers (max 2) inside center circle
        let home_kicks_off_2h = false;  // Away kicks off second half
        self.apply_kickoff_positions(home_kicks_off_2h);

        // Reset ball to center spot
        self.ball.position = Coord10::CENTER;
        self.ball.velocity = Vel10::from_mps(0.0, 0.0);
        self.ball.height = 0;

        // Set ball owner to away kicker (index 21 = away CF)
        let away_kickoff_player: usize = 21;
        self.ball.current_owner = Some(away_kickoff_player);

        let kickoff_is_home = home_kicks_off_2h;
        let kickoff_timestamp_ms = self.minute as u64 * 60_000;
        self.emit_event(MatchEvent::kick_off(
            self.minute,
            kickoff_timestamp_ms,
            kickoff_is_home,
        ));

        // FIX_2601/0119: Track 2H kickoff tick for stabilization logic
        self.last_kickoff_tick = self.current_tick;

        // 5. Reset team phase states
        self.home_phase_state = TeamPhaseState::default();
        self.away_phase_state = TeamPhaseState::default();

        // 6. Clear action queue for fresh start
        self.action_queue.clear();

        // FIX_2601/0105: Reset shot counters for second half
        self.shots_this_half_home = 0;
        self.shots_this_half_away = 0;

        // FIX_2601/0123: Apply halftime momentum recovery (leadership bonus)
        self.home_momentum.apply_halftime_recovery();
        self.away_momentum.apply_halftime_recovery();

        println!(
            "[MatchEngine] Half-time: sides swapped, away team ({}) kicks off second half",
            away_kickoff_player
        );
    }

    /// Return the current position of a player in meters (105x68 field).
    fn get_player_position_m(&self, player_idx: usize) -> (f32, f32) {
        // FIX_2601: Coord10 has to_meters() directly
        self.get_player_position_by_index(player_idx).to_meters()
    }

    /// FIX_2601/0105: Determine if a team attacks right (toward X=105) this half.
    ///
    /// **KEY INSIGHT**: In real football, teams switch sides at halftime!
    /// - First half: Home attacks toward X=105, Away attacks toward X=0
    /// - Second half: Home attacks toward X=0, Away attacks toward X=105
    ///
    /// Combined with the X-flip of player positions at halftime:
    /// - Home players flip from x≈10 to x≈95, but now attack toward x=0
    /// - Away players flip from x≈95 to x≈10, but now attack toward x=105
    ///
    /// This creates symmetric gameplay where both teams have similar distances
    /// to their goals at the start of each half.
    #[inline]
    pub(crate) fn attacks_right(&self, is_home: bool) -> bool {
        if self.is_second_half {
            !is_home // Second half: Home attacks LEFT (x=0), Away attacks RIGHT (x=105)
        } else {
            is_home // First half: Home attacks RIGHT (x=105), Away attacks LEFT (x=0)
        }
    }

    /// FIX_2601/0120: Calculate current kickoff phase for bias analysis
    ///
    /// Returns the current game phase to help identify phase-specific bias:
    /// - GameStart: First 200 ticks of the match (first ~50 seconds)
    /// - AfterHalftime: First 200 ticks after halftime
    /// - AfterGoal: First 200 ticks after a goal restart (not yet implemented)
    /// - Normal: All other gameplay
    ///
    /// The 200 tick threshold (~50 seconds at 4 ticks/sec) captures the
    /// stabilization period where warmup effects or initialization issues
    /// would be most visible.
    pub(crate) fn calculate_kickoff_phase(&self) -> shot_opportunity::KickoffPhase {
        const KICKOFF_WINDOW_TICKS: u64 = 200;

        let ticks_since_kickoff = self.current_tick.saturating_sub(self.last_kickoff_tick);

        if ticks_since_kickoff < KICKOFF_WINDOW_TICKS {
            if self.is_second_half {
                shot_opportunity::KickoffPhase::AfterHalftime
            } else if self.last_kickoff_tick == 0 {
                // First half, last_kickoff_tick is 0 -> game start
                shot_opportunity::KickoffPhase::GameStart
            } else {
                // First half but last_kickoff_tick > 0 -> after goal
                shot_opportunity::KickoffPhase::AfterGoal
            }
        } else {
            shot_opportunity::KickoffPhase::Normal
        }
    }

    /// FIX_2601/0105: Record a shot for shot budget tracking
    /// Called whenever a shot is taken (alongside result.statistics update)
    #[inline]
    pub(crate) fn record_shot_for_budget(&mut self, is_home: bool) {
        if is_home {
            self.shots_this_half_home = self.shots_this_half_home.saturating_add(1);
        } else {
            self.shots_this_half_away = self.shots_this_half_away.saturating_add(1);
        }
    }

    /// FIX_2601/0105: Get shots this half for a team
    #[inline]
    pub(crate) fn shots_this_half(&self, is_home: bool) -> u8 {
        if is_home {
            self.shots_this_half_home
        } else {
            self.shots_this_half_away
        }
    }

    /// FIX_2601/0105: Get shot budget per half
    #[inline]
    pub(crate) fn shot_budget(&self) -> u8 {
        self.shot_budget_per_half
    }

    /// Simple helper to decide if the given index refers to the configured
    /// user player. This reuses the existing is_user_player logic.
    fn is_user_controlled_player(&self, player_idx: usize, is_home: bool) -> bool {
        self.is_user_player(player_idx, is_home)
    }

    // build_user_decision_context, check_and_build_intervention
    // → Moved to interactive_session.rs (P2-9 refactoring)

    /// Initialize simulation (must be called before step())
    pub fn init(&mut self) -> (f32, f32, f32, u8) {
        // FIX_2601/0113 v1.1: DPQ scheduler must start from a clean state for
        // step-based / interactive flows.
        self.decision_scheduler.reset();

        // P-1: Initialize player positions FIRST (same order as simulate())
        self.initialize_player_positions();

        // FIX_2601/0123: Initialize team momentum with captain leadership
        // Find highest leadership among field players (idx 1-10 for home, 12-21 for away)
        let home_captain_leadership = self.find_captain_leadership(true);
        let away_captain_leadership = self.find_captain_leadership(false);
        self.home_momentum = momentum::TeamMomentum::with_captain_leadership(home_captain_leadership);
        self.away_momentum = momentum::TeamMomentum::with_captain_leadership(away_captain_leadership);

        // P-1: Initialize player inertias (was missing, causing Live vs Batch difference)
        self.init_player_motion_params();

        // Pre-match calculations
        let home_strength = self.calculate_team_strength(&self.home_team, true);
        let away_strength = self.calculate_team_strength(&self.away_team, false);
        let possession_ratio = self.calculate_possession(home_strength, away_strength);

        // P-1: Record initial kickoff state (was missing, causing Live vs Batch difference)
        // Must be called BEFORE any match-duration logic to match simulate() order
        //
        // FIX_2601/1120: Apply kickoff positioning rules BEFORE recording snapshot
        let home_kicks_off = true;  // Home team kicks off first half
        self.apply_kickoff_positions(home_kicks_off);
        self.fix01_sync_ssot_proof_after_kickoff_positions();
        self.record_initial_kickoff_state(home_kicks_off);

        // FIX_2601/0106 P2: Reset added-time state for step-based flows.
        self.stoppage_seconds_first_half = 0;
        self.stoppage_finalized_first_half = false;
        self.first_half_end_minute = HALF_DURATION_MINUTES;
        self.stoppage_seconds_second_half = 0;
        self.stoppage_finalized_second_half = false;
        self.match_end_minute = REGULATION_TOTAL_MINUTES;

        // Step-based API needs a stable upper bound; the actual end minute is decided at
        // regulation boundaries.
        let match_duration = MATCH_DURATION_CAP_MINUTES;

        // Cache for step-based / interactive flows
        self.precomputed_home_strength = home_strength;
        self.precomputed_away_strength = away_strength;
        self.precomputed_possession_ratio = possession_ratio;
        self.precomputed_match_duration = match_duration;

        (home_strength, away_strength, possession_ratio, match_duration)
    }

    /// Apply scenario overrides after init().
    pub fn apply_scenario_overrides(
        &mut self,
        overrides: &crate::engine::scenario_loader::ScenarioOverrides,
    ) {
        if let Some(home_attacks_right) = overrides.home_attacks_right {
            self.is_second_half = !home_attacks_right;
            self.home_ctx = DirectionContext { is_home: true, attacks_right: home_attacks_right };
            self.away_ctx = DirectionContext { is_home: false, attacks_right: !home_attacks_right };
        }

        for (track_id, pos) in &overrides.player_positions {
            if *track_id < self.player_positions.len() {
                self.player_positions[*track_id] = *pos;
            }
        }

        if let Some(ball) = &overrides.ball {
            self.ball.position = ball.position;
            self.ball.height = ball.height;
            self.ball.velocity = ball.velocity;
            self.ball.velocity_z = ball.velocity_z;
            self.ball.current_owner = ball.owner;
            self.ball.previous_owner = None;
            self.ball.is_in_flight = matches!(
                ball.state,
                crate::engine::scenario_loader::ScenarioBallStateKind::InFlight
            );
            self.ball.flight_progress = 0.0;
            self.ball.from_position = None;
            self.ball.to_position = None;
            self.ball.pending_owner = None;
            match ball.state {
                crate::engine::scenario_loader::ScenarioBallStateKind::Loose => {
                    self.action_queue.set_ball_state(super::BallState::Loose {
                        position: ball.position,
                        velocity: ball.velocity,
                    });
                }
                crate::engine::scenario_loader::ScenarioBallStateKind::Controlled => {
                    if let Some(owner_idx) = ball.owner {
                        self.action_queue
                            .set_ball_state(super::BallState::Controlled { owner_idx });
                    } else {
                        self.action_queue.set_ball_state(super::BallState::Loose {
                            position: ball.position,
                            velocity: ball.velocity,
                        });
                    }
                }
                crate::engine::scenario_loader::ScenarioBallStateKind::InFlight => {
                    self.action_queue.set_ball_state(super::BallState::Loose {
                        position: ball.position,
                        velocity: ball.velocity,
                    });
                }
            }
        }

        for track_id in &overrides.lazy_players {
            if *track_id < self.player_states.len() {
                self.player_states[*track_id] = PlayerState::Injured;
                self.player_velocities[*track_id] = (0.0, 0.0);
                self.player_speeds[*track_id] = 0.0;
            }
        }

        if let Some(start_mode) = overrides.start_mode {
            self.apply_scenario_start_mode(
                start_mode,
                overrides.ball.as_ref(),
                overrides.start_team,
            );
        }
    }

    fn apply_scenario_start_mode(
        &mut self,
        start_mode: crate::engine::scenario_loader::ScenarioStartMode,
        ball_override: Option<&crate::engine::scenario_loader::ScenarioBallState>,
        start_team: Option<crate::engine::scenario_loader::ScenarioSide>,
    ) {
        use crate::engine::physics_constants::field;
        use crate::engine::types::coord10::{Coord10, Vel10};
        use crate::models::{EventType, MatchEvent};

        if matches!(start_mode, crate::engine::scenario_loader::ScenarioStartMode::Normal) {
            return;
        }

        let restart_type = match start_mode {
            crate::engine::scenario_loader::ScenarioStartMode::KickOff => {
                super::RestartType::KickOff
            }
            crate::engine::scenario_loader::ScenarioStartMode::GoalKick => {    
                super::RestartType::GoalKick
            }
            crate::engine::scenario_loader::ScenarioStartMode::FreeKick => {    
                super::RestartType::FreeKick
            }
            crate::engine::scenario_loader::ScenarioStartMode::Corner => super::RestartType::Corner,
            crate::engine::scenario_loader::ScenarioStartMode::ThrowIn => {     
                super::RestartType::ThrowIn
            }
            crate::engine::scenario_loader::ScenarioStartMode::Penalty => {     
                super::RestartType::Penalty
            }
            crate::engine::scenario_loader::ScenarioStartMode::DropBall => {
                super::RestartType::DropBall
            }
            crate::engine::scenario_loader::ScenarioStartMode::Normal => {      
                super::RestartType::KickOff
            }
        };

        self.restart_occurred_this_tick = true;
        self.restart_type_this_tick = Some(restart_type);
        self.action_queue.clear();

        let owner_idx = ball_override.and_then(|ball| ball.owner);
        let owner_is_home = owner_idx
            .map(TeamSide::is_home)
            .or(start_team.map(|side| side.is_home()))
            .unwrap_or(true);
        let attacks_right = self.attacks_right(owner_is_home);

        let default_owner = match start_mode {
            crate::engine::scenario_loader::ScenarioStartMode::KickOff => {
                if owner_is_home {
                    10
                } else {
                    21
                }
            }
            crate::engine::scenario_loader::ScenarioStartMode::GoalKick => {
                if owner_is_home {
                    0
                } else {
                    11
                }
            }
            crate::engine::scenario_loader::ScenarioStartMode::Corner
            | crate::engine::scenario_loader::ScenarioStartMode::FreeKick       
            | crate::engine::scenario_loader::ScenarioStartMode::ThrowIn
            | crate::engine::scenario_loader::ScenarioStartMode::DropBall => {   
                if owner_is_home {
                    7
                } else {
                    18
                }
            }
            crate::engine::scenario_loader::ScenarioStartMode::Penalty => {     
                if owner_is_home {
                    9
                } else {
                    20
                }
            }
            crate::engine::scenario_loader::ScenarioStartMode::Normal => {
                if owner_is_home {
                    10
                } else {
                    21
                }
            }
        };

        let default_position = match start_mode {
            crate::engine::scenario_loader::ScenarioStartMode::KickOff => Coord10::CENTER,
            crate::engine::scenario_loader::ScenarioStartMode::GoalKick => {
                let x = if attacks_right { 5.25 } else { 99.75 };
                Coord10::from_meters(x, field::CENTER_Y)
            }
            crate::engine::scenario_loader::ScenarioStartMode::Corner => {
                let x = if attacks_right { field::LENGTH_M } else { 0.0 };
                Coord10::from_meters(x, 0.0)
            }
            crate::engine::scenario_loader::ScenarioStartMode::ThrowIn => {
                Coord10::from_meters(field::CENTER_X, 0.0)
            }
            crate::engine::scenario_loader::ScenarioStartMode::Penalty => {
                let x = if attacks_right { 94.5 } else { 10.5 };
                Coord10::from_meters(x, field::CENTER_Y)
            }
            crate::engine::scenario_loader::ScenarioStartMode::DropBall => {
                Coord10::CENTER
            }
            crate::engine::scenario_loader::ScenarioStartMode::FreeKick
            | crate::engine::scenario_loader::ScenarioStartMode::Normal => Coord10::CENTER,
        };

        if ball_override.is_none() {
            self.ball.position = default_position;
        }

        if owner_idx.is_none() {
            self.ball.current_owner = Some(default_owner);
        }

        self.ball.is_in_flight = false;
        self.ball.pending_owner = None;
        self.ball.from_position = None;
        self.ball.to_position = None;
        self.ball.flight_progress = 0.0;

        if ball_override.is_none() {
            self.ball.velocity = Vel10::default();
            self.ball.velocity_z = 0;
            self.ball.height = 0;
            self.ball.height_profile = crate::engine::ball::HeightProfile::Flat;
        }

        let taker_idx = owner_idx.unwrap_or(default_owner);
        match start_mode {
            crate::engine::scenario_loader::ScenarioStartMode::KickOff => {
                if let Some(event) = self
                    .result
                    .events
                    .iter_mut()
                    .find(|event| matches!(event.event_type, EventType::KickOff))
                {
                    event.is_home_team = owner_is_home;
                } else {
                    self.emit_event(MatchEvent::kick_off(
                        self.minute,
                        self.current_timestamp_ms(),
                        owner_is_home,
                    ));
                }
            }
            crate::engine::scenario_loader::ScenarioStartMode::GoalKick => {
                self.emit_event(MatchEvent::goal_kick(
                    self.minute,
                    self.current_timestamp_ms(),
                    owner_is_home,
                    taker_idx,
                ));
            }
            crate::engine::scenario_loader::ScenarioStartMode::Corner => {
                self.emit_event(MatchEvent::corner(
                    self.minute,
                    self.current_timestamp_ms(),
                    owner_is_home,
                    taker_idx,
                ));
            }
            crate::engine::scenario_loader::ScenarioStartMode::FreeKick => {
                let pos_m = self.ball.position.to_meters();
                let pos_norm = crate::engine::coordinates::to_normalized(pos_m);
                self.emit_event(MatchEvent::freekick(
                    self.minute,
                    self.current_timestamp_ms(),
                    owner_is_home,
                    taker_idx,
                    (pos_norm.0, pos_norm.1, self.ball.height as f32 / 10.0),
                ));
            }
            crate::engine::scenario_loader::ScenarioStartMode::Penalty => {
                self.emit_event(MatchEvent::penalty(
                    self.minute,
                    self.current_timestamp_ms(),
                    owner_is_home,
                    taker_idx,
                ));
            }
            crate::engine::scenario_loader::ScenarioStartMode::ThrowIn => {
                self.emit_event(MatchEvent::throw_in(
                    self.minute,
                    self.current_timestamp_ms(),
                    owner_is_home,
                    taker_idx,
                ));
            }
            crate::engine::scenario_loader::ScenarioStartMode::DropBall => {}
            crate::engine::scenario_loader::ScenarioStartMode::Normal => {}
        }
    }

    /// Step one minute forward in simulation
    /// Returns true if simulation should continue, false if finished
    pub fn step(
        &mut self,
        home_strength: f32,
        away_strength: f32,
        possession_ratio: f32,
        match_duration: u8,
    ) -> bool {
        // Finalize 1H added time at minute 45 (before simulating 45++ minutes).
        if !self.is_second_half && self.minute >= HALF_DURATION_MINUTES {
            self.maybe_finalize_first_half_stoppage_time();
        }

        // Half-time boundary (second half kickoff).
        if !self.is_second_half && self.minute == self.first_half_end_minute {
            self.handle_half_time();
        }

        // Finalize 2H added time at regulation end (before simulating added minutes).
        if self.is_second_half && self.minute >= self.regulation_end_minute() {
            self.maybe_finalize_second_half_stoppage_time();
        }

        let match_end_minute = self.match_end_minute.min(match_duration);

        if self.minute > match_end_minute {
            if !self
                .result
                .events
                .iter()
                .any(|event| matches!(event.event_type, EventType::FullTime))
            {
                let timestamp_ms = match_end_minute as u64 * 60_000;
                self.current_timestamp_ms = timestamp_ms;
                self.emit_event(MatchEvent::full_time(match_end_minute, timestamp_ms));
            }
            return false;
        }

        // Simulate this minute
        self.simulate_minute(home_strength, away_strength, possession_ratio);
        self.minute += 1;

        // FIX_2601/0106 P4: Update match situation
        self.match_situation.update_minute(self.minute as u32);
        self.match_situation.update_score(self.result.score_home, self.result.score_away);

        // Approximate time at the end of this minute for interactive mode
        self.current_timestamp_ms = self.minute as u64 * 60_000;
        let continue_match = self.minute <= match_end_minute;
        if !continue_match
            && !self.result.events.iter().any(|event| matches!(event.event_type, EventType::FullTime))
        {
            let timestamp_ms = match_end_minute as u64 * 60_000;
            self.current_timestamp_ms = timestamp_ms;
            self.emit_event(MatchEvent::full_time(match_end_minute, timestamp_ms));
        }
        continue_match
    }

    /// Build aggregated stats for the configured user player (MyPlayer).
    /// This scans all generated events once and derives goals/assists/shots/etc.
    fn build_user_player_stats(&self) -> Option<MyPlayerStats> {
        let user_config = self.user_player.as_ref()?;
        let hero_track_id = user_config.player_index as u8; // C7: Use track_id instead of name
        let hero_is_home = user_config.is_home_team;

        let mut goals: u32 = 0;
        let mut assists: u32 = 0;
        let mut shots: u32 = 0;
        let mut passes: u32 = 0;
        let mut tackles: u32 = 0;
        let mut fouls: u32 = 0;
        let saves: u32 = 0;
        let mut yellow_cards: u32 = 0;
        let mut red_cards: u32 = 0;

        for event in &self.result.events {
            // C7: Use track_id for player identity (not name)
            let is_hero_player =
                event.player_track_id == Some(hero_track_id) && event.is_home_team == hero_is_home;

            match event.event_type {
                EventType::Goal => {
                    if is_hero_player {
                        goals += 1;
                        // A goal is also a shot
                        shots += 1;
                    }

                    // C7: Assist credit - check target_track_id (not name)
                    if event.target_track_id == Some(hero_track_id)
                        && event.is_home_team == hero_is_home
                    {
                        assists += 1;
                    }
                }
                EventType::Shot
                | EventType::ShotOnTarget
                | EventType::ShotOffTarget
                | EventType::ShotBlocked => {
                    if is_hero_player {
                        shots += 1;
                    }
                }
                EventType::Pass => {
                    if is_hero_player {
                        passes += 1;
                    }
                }
                EventType::Tackle => {
                    if is_hero_player {
                        tackles += 1;
                    }
                }
                EventType::Foul => {
                    if is_hero_player {
                        fouls += 1;
                    }
                }
                EventType::YellowCard => {
                    if is_hero_player {
                        yellow_cards += 1;
                    }
                }
                EventType::RedCard => {
                    if is_hero_player {
                        red_cards += 1;
                    }
                }
                EventType::Save => {
                    // NOTE: In the current event model, Save events record the
                    // shooter as the player, not the goalkeeper. We keep this
                    // counter wired for future engine updates but do not use
                    // it for rating yet.
                    // TODO: Re-enable when Save events correctly identify goalkeeper
                    // if is_hero_player {
                    //     saves += 1;
                    // }
                    let _ = saves; // silence unused warning until feature is enabled
                }
                _ => {}
            }
        }

        // Simple yet expressive rating formula (6.0 base, 3.0~10.0 clamp)
        let mut rating = 6.0f32;
        rating += goals as f32 * 0.75;
        rating += assists as f32 * 0.5;
        rating += (shots as f32 * 0.05).min(0.4);
        rating += (tackles as f32 * 0.08).min(0.4);
        rating += (passes as f32 * 0.01).min(0.3);

        rating -= fouls as f32 * 0.05;
        rating -= yellow_cards as f32 * 0.2;
        rating -= red_cards as f32 * 1.0;

        let rating = rating.clamp(3.0, 10.0);

        // C7: Get player name from user_config (not from events)
        let player_name = &user_config.player_name;

        Some(MyPlayerStats {
            player_id: player_name.clone(),
            player_name: player_name.clone(),
            goals,
            assists,
            shots,
            passes,
            tackles,
            fouls,
            saves,
            yellow_cards,
            red_cards,
            rating,
        })
    }

    /// Finalize the match (must be called after simulation)
    ///
    /// Includes P0 consistency validation: score_home + score_away == goal_event_count
    /// If mismatch detected (e.g., due to budget exhaustion mid-operation), events are SSOT
    /// and score is reconciled from events.
    pub fn finalize(&mut self, possession_ratio: f32) -> MatchResult {
        println!("🏁 [MatchEngine] finalize() CALLED - START 🏁");
        let _ = io::stdout().flush();

        // ========== P0 CONTRACT: Score/Event Consistency Check ==========
        // Validate that score matches goal event count. If mismatch, reconcile from events.
        // Events are the Single Source of Truth (SSOT) for match narrative.
        let (goal_events_home, goal_events_away) = self.count_goal_events();
        let score_sum = self.result.score_home + self.result.score_away;
        let event_sum = goal_events_home + goal_events_away;

        if self.result.score_home != goal_events_home || self.result.score_away != goal_events_away {
            println!(
                "⚠️ [MatchEngine] CONSISTENCY WARNING: Score mismatch detected! \
                 score={}:{} but events={}:{} | Reconciling from events (SSOT)",
                self.result.score_home, self.result.score_away,
                goal_events_home, goal_events_away
            );
            let _ = io::stdout().flush();

            // Events are SSOT - reconcile score from events
            self.result.score_home = goal_events_home;
            self.result.score_away = goal_events_away;
        } else if score_sum > 0 {
            println!(
                "✅ [MatchEngine] Score consistency OK: {}:{} ({} goal events)",
                self.result.score_home, self.result.score_away, event_sum
            );
            let _ = io::stdout().flush();
        }

        // FIX_2601/0106 P2: Ensure added time is finalized before emitting FullTime fallback.
        self.maybe_finalize_first_half_stoppage_time();
        self.maybe_finalize_second_half_stoppage_time();

        if !self.result.events.iter().any(|event| matches!(event.event_type, EventType::FullTime)) {
            let match_end_minute = self.match_end_minute;
            let timestamp_ms = match_end_minute as u64 * 60_000;
            self.current_timestamp_ms = timestamp_ms;
            self.emit_event(MatchEvent::full_time(match_end_minute, timestamp_ms));
        }
        self.finalize_pass_sequences();


        // Calculate final statistics
        self.stats_calculator.finalize(&mut self.result, possession_ratio);

        // Aggregate per-match stats for the configured user player (MyPlayer)
        if let Some(user_stats) = self.build_user_player_stats() {
            self.result.statistics.my_player_stats = Some(user_stats);
        }

        // Filter events based on what user wants to see
        if self.user_player.is_some() {
            self.filter_events_for_display();
        }

        // Sort events by minute
        self.result.events.sort_by_key(|e| e.minute);

        // Convert to detailed replay events if tracking enabled
        println!("?��?��?�� [MatchEngine] DEBUG: track_positions={}, replay_events.is_some={}, events={} ?��?��?��",
                 self.track_positions, self.result.replay_events.is_some(), self.result.events.len());
        let _ = io::stdout().flush();
        if self.result.replay_events.is_some() {
            // Extract player rosters (first 11 players are starting lineup)
            let home_roster: Vec<String> =
                self.home_team.players.iter().take(11).map(|p| p.name.clone()).collect();

            let away_roster: Vec<String> =
                self.away_team.players.iter().take(11).map(|p| p.name.clone()).collect();

            let mut converter = ReplayConverter::with_rosters(
                self.home_team.formation.clone(),
                self.away_team.formation.clone(),
                home_roster,
                away_roster,
            );
            match converter.convert_events(&self.result) {
                Ok(replay_events) => {
                    let event_count = replay_events.len();
                    self.result.replay_events = Some(replay_events);
                    println!(
                        "?��?��?�� [MatchEngine] Generated {} replay events ?��?��?��",
                        event_count
                    );
                    let _ = io::stdout().flush();
                }
                Err(e) => {
                    println!(
                        "?��?��?�� [MatchEngine] Warning: Failed to convert replay events: {} ?��?��?��",
                        e
                    );
                    let _ = io::stdout().flush();
                    self.result.replay_events = Some(Vec::new());
                }
            }
        }

        // Store teams for roster information
        self.result.home_team = Some(self.home_team.clone());
        self.result.away_team = Some(self.away_team.clone());

        // P17 Phase 5: Store match setup for viewer (starting lineup snapshot).
        self.result.match_setup = Some(self.setup.to_export_starting_lineup());

        // Add debug info
        self.result.debug_info = Some(format!(
            "simulate() called | track_positions={} | replay_events={} | events={} | teams_stored=true",
            self.track_positions,
            self.result.replay_events.as_ref().map(|r| r.len()).unwrap_or(0),
            self.result.events.len()
        ));

        self.maybe_run_penalty_shootout();

        // Generate match summary for quick display on result screens
        self.result.generate_summary();

        // P18: Board summary (final snapshot of occupancy/pressure)
        if let Some(ref board) = self.field_board {
            self.result.board_summary = Some(board.to_summary_export(5));
        }

        self.result.clone()
    }

    // simulate_until_intervention, execute_direct_pass_to, resume_with_action
    // → Moved to interactive_session.rs (P2-9 refactoring)

    /// Get current event count (for budget checking)
    pub fn event_count(&self) -> usize {
        self.result.events.len()
    }

    /// Get current minute
    pub fn current_minute(&self) -> u8 {
        self.minute
    }

    // simulate_minute, simulate_attack, score_goal, simulate_other_events
    // → Moved to simulation_logic.rs (P2-9 refactoring)

    // calculate_team_strength, get_tactical_context, calculate_possession, calculate_xg
    // → Moved to calculations.rs (P2-9 refactoring)

    // generate_user_player_involvement, filter_events_for_display, get_player_position_by_index,
    // find_player_idx_by_name
    // → Moved to helpers.rs (P2-9 refactoring)

    // calculate_shooting_probability, calculate_pass_success,
    // calculate_pressure_penalty, check_interception_risk, calculate_involvement_weight
    // → Moved to calculations.rs (P2-9 refactoring)

    // ========== Career Player Mode: User Control Methods ==========

    /// Enable Career Player Mode for a specific track_id
    pub fn enable_controlled_mode(&mut self, track_id: usize) {
        self.controlled_mode = Some(ControlledPlayerMode::new(track_id));
    }

    /// Disable Career Player Mode
    pub fn disable_controlled_mode(&mut self) {
        self.controlled_mode = None;
        self.user_command_queue.clear();
    }

    /// Check if a specific track_id is the controlled player
    pub fn is_controlled(&self, track_id: usize) -> bool {
        self.controlled_mode.as_ref().map(|m| m.is_controlled(track_id)).unwrap_or(false)
    }

    /// Submit a user command to the queue
    pub fn submit_user_command(&mut self, cmd: UserCommand) {
        self.user_command_queue.enqueue(cmd);
    }

    /// Register a controller slot for multi-agent control.
    pub fn register_controller_slot(
        &mut self,
        controller_id: u32,
        team_side: crate::models::TeamSide,
        player_slot: u8,
    ) -> Result<(), &'static str> {
        let track_id = crate::models::TeamSide::track_id_from_slot(team_side, player_slot)
            .ok_or("invalid_player_slot")?;
        if self.multi_agent_registry.contains_key(&controller_id) {
            return Err("controller_id_in_use");
        }
        if self.multi_agent_tracks.contains(&track_id) {
            return Err("track_id_in_use");
        }
        self.multi_agent_registry
            .insert(controller_id, ControllerSlot { controller_id, team_side, player_slot });
        self.multi_agent_tracks.insert(track_id);
        Ok(())
    }

    /// Unregister a controller slot.
    pub fn unregister_controller_slot(&mut self, controller_id: u32) -> Result<(), &'static str> {
        let slot =
            self.multi_agent_registry.remove(&controller_id).ok_or("unknown_controller_id")?;
        if let Some(track_id) =
            crate::models::TeamSide::track_id_from_slot(slot.team_side, slot.player_slot)
        {
            self.multi_agent_tracks.remove(&track_id);
            if track_id < self.multi_agent_command_queues.len() {
                self.multi_agent_command_queues[track_id].clear();
                self.multi_agent_lock_until_tick[track_id] = 0;
            }
        }
        self.multi_agent_last_seq.remove(&controller_id);
        Ok(())
    }

    /// Clear all controller slots and queued commands.
    pub fn clear_controller_slots(&mut self) {
        self.multi_agent_registry.clear();
        self.multi_agent_last_seq.clear();
        self.multi_agent_tracks.clear();
        for queue in &mut self.multi_agent_command_queues {
            queue.clear();
        }
        for lock in &mut self.multi_agent_lock_until_tick {
            *lock = 0;
        }
    }

    /// Submit multi-agent commands (controller_id -> track_id routing).
    pub fn submit_multi_agent_commands(
        &mut self,
        commands: Vec<MultiAgentCommand>,
    ) -> Result<(), &'static str> {
        for cmd in commands {
            let slot =
                self.multi_agent_registry.get(&cmd.controller_id).ok_or("unknown_controller_id")?;
            let track_id =
                crate::models::TeamSide::track_id_from_slot(slot.team_side, slot.player_slot)
                    .ok_or("invalid_player_slot")?;
            if track_id >= self.multi_agent_command_queues.len() {
                return Err("invalid_track_id");
            }
            let last_seq = self.multi_agent_last_seq.get(&cmd.controller_id).copied().unwrap_or(0);
            if cmd.seq <= last_seq {
                continue;
            }
            let user_cmd =
                UserCommand { seq: cmd.seq, controlled_track_id: track_id, payload: cmd.payload };
            self.multi_agent_command_queues[track_id].enqueue(user_cmd);
            self.multi_agent_last_seq.insert(cmd.controller_id, cmd.seq);
        }
        Ok(())
    }

    /// Update sticky action toggle for a player (sprint/dribble/press).
    pub fn set_sticky_action(
        &mut self,
        track_id: usize,
        action: StickyAction,
        enabled: bool,
    ) -> Result<(), &'static str> {
        if track_id >= self.sticky_actions.len() {
            return Err("invalid_track_id");
        }
        self.sticky_actions[track_id].set(action, enabled);
        Ok(())
    }

    /// Read sticky actions for a player.
    pub fn get_sticky_actions(&self, track_id: usize) -> Option<StickyActions> {
        if track_id >= self.sticky_actions.len() {
            None
        } else {
            Some(self.sticky_actions[track_id])
        }
    }

    // ========== State Snapshot API ==========

    /// Capture complete match state as a snapshot
    ///
    /// Returns a serializable snapshot that can be used to restore the match
    /// to this exact state. Useful for:
    /// - Save/Load functionality
    /// - AI training (state branching)
    /// - Deterministic replay from checkpoint
    /// - Debugging (reproduce exact state)
    ///
    /// # Example
    /// ```ignore
    /// let snapshot = engine.get_state();
    /// let json = snapshot.to_json()?;
    /// // ... later ...
    /// engine.set_state(MatchStateSnapshot::from_json(&json)?)?;
    /// ```
    pub fn get_state(&self) -> super::snapshot::MatchStateSnapshot {
        use super::snapshot::MatchStateSnapshot;

        // Convert player velocities from [(f32, f32); 22] to Vec<Vel10>
        let player_velocities: Vec<_> = self
            .player_velocities
            .iter()
            .map(|(vx, vy)| super::types::Vel10::from_mps(*vx, *vy))
            .collect();

        // Get player states as Vec
        let player_states: Vec<_> = self.player_states.to_vec();

        MatchStateSnapshot {
            // Time
            current_tick: self.current_tick,
            last_kickoff_tick: self.last_kickoff_tick,  // FIX_2601/0119
            minute: self.minute,
            is_second_half: self.is_second_half,
            current_timestamp_ms: self.current_timestamp_ms,
            stoppage_seconds_first_half: self.stoppage_seconds_first_half,
            stoppage_finalized_first_half: self.stoppage_finalized_first_half,
            first_half_end_minute: self.first_half_end_minute,
            stoppage_seconds_second_half: self.stoppage_seconds_second_half,
            stoppage_finalized_second_half: self.stoppage_finalized_second_half,
            match_end_minute: self.match_end_minute,

            // Ball
            ball: self.ball.clone(),

            // Players
            player_positions: self.player_positions.clone(),
            player_velocities,
            player_reaction_states: self.player_reaction_states.clone(),
            player_fatigue: self.player_fatigue.clone(),
            stamina: self.stamina,
            sprint_state: self.sprint_state,
            player_resting: self.player_resting,
            continuous_running_ticks: self.continuous_running_ticks,
            player_states,
            tackle_cooldowns: self.tackle_cooldowns,
            player_speeds: self.player_speeds,

            // Score
            score_home: self.result.score_home,
            score_away: self.result.score_away,
            injured_players: self.injured_players.clone(),
            substitutions_made: self.substitutions_made,

            // Game state
            game_state: self.game_state.clone(),
            offside_count_home: self.offside_count_home,
            offside_count_away: self.offside_count_away,
            shots_this_half_home: self.shots_this_half_home,
            shots_this_half_away: self.shots_this_half_away,
            possession_owner_idx: self.possession_owner_idx,
            possession_owner_since_tick: self.possession_owner_since_tick,

            // Action queue
            pending_actions: self.action_queue.to_snapshot().pending,
            active_actions: self.action_queue.to_snapshot().active,
            action_queue_ball_state: self.action_queue.ball_state().clone(),
            next_action_id: self.action_queue.to_snapshot().next_action_id,

            // RNG
            rng_seed: self.original_seed,
            rng_word_pos: self.rng.get_word_pos(),
        }
    }

    /// Restore match state from a snapshot
    ///
    /// Restores all mutable state from the snapshot, including RNG position
    /// for deterministic continuation.
    ///
    /// # Errors
    /// Returns error if snapshot data is incompatible (e.g., wrong player count)
    pub fn set_state(
        &mut self,
        snapshot: super::snapshot::MatchStateSnapshot,
    ) -> Result<(), super::snapshot::SnapshotError> {
        use super::snapshot::SnapshotError;

        // Validate player count
        if snapshot.player_positions.len() != 22 {
            return Err(SnapshotError::PlayerCountMismatch {
                expected: 22,
                actual: snapshot.player_positions.len(),
            });
        }

        // Restore time state
        self.current_tick = snapshot.current_tick;
        self.last_kickoff_tick = snapshot.last_kickoff_tick;  // FIX_2601/0119
        self.minute = snapshot.minute;
        self.is_second_half = snapshot.is_second_half;
        self.current_timestamp_ms = snapshot.current_timestamp_ms;
        self.stoppage_seconds_first_half = snapshot.stoppage_seconds_first_half;
        self.stoppage_finalized_first_half = snapshot.stoppage_finalized_first_half;
        self.first_half_end_minute = snapshot.first_half_end_minute;
        self.stoppage_seconds_second_half = snapshot.stoppage_seconds_second_half;
        self.stoppage_finalized_second_half = snapshot.stoppage_finalized_second_half;
        self.match_end_minute = snapshot.match_end_minute;

        // Restore ball
        self.ball = snapshot.ball;

        // Restore player state
        self.player_positions = snapshot.player_positions;

        // Convert Vec<Vel10> back to [(f32, f32); 22]
        for (i, vel) in snapshot.player_velocities.iter().enumerate() {
            if i < 22 {
                let (vx, vy) = vel.to_mps();
                self.player_velocities[i] = (vx, vy);
            }
        }

        self.player_reaction_states = snapshot.player_reaction_states;
        self.player_fatigue = snapshot.player_fatigue;
        self.stamina = snapshot.stamina;
        self.sprint_state = snapshot.sprint_state;
        self.player_resting = snapshot.player_resting;
        self.continuous_running_ticks = snapshot.continuous_running_ticks;

        // Restore player states
        for (i, state) in snapshot.player_states.iter().enumerate() {
            if i < 22 {
                self.player_states[i] = *state;
            }
        }

        self.tackle_cooldowns = snapshot.tackle_cooldowns;
        self.player_speeds = snapshot.player_speeds;
        self.decision_intents.clear();

        // Restore score
        self.result.score_home = snapshot.score_home;
        self.result.score_away = snapshot.score_away;
        self.injured_players = snapshot.injured_players;
        self.substitutions_made = snapshot.substitutions_made;

        // Restore game state
        self.game_state = snapshot.game_state;
        self.offside_count_home = snapshot.offside_count_home;
        self.offside_count_away = snapshot.offside_count_away;
        self.shots_this_half_home = snapshot.shots_this_half_home;
        self.shots_this_half_away = snapshot.shots_this_half_away;
        self.possession_owner_idx = snapshot.possession_owner_idx;
        self.possession_owner_since_tick = snapshot.possession_owner_since_tick;

        // Restore action queue
        let queue_snapshot = super::snapshot::ActionQueueSnapshot {
            pending: snapshot.pending_actions,
            active: snapshot.active_actions,
            ball_state: snapshot.action_queue_ball_state,
            next_action_id: snapshot.next_action_id,
            current_tick: snapshot.current_tick,
            last_shot_xg: None,     // Reset
            last_shooter_idx: None, // Reset
            last_passer_idx: None,  // Reset
            last_pass_receiver_idx: None, // Reset
            last_pass_type: None,
            last_header_outcome: None,
            in_flight_origin: None, // Reset
        };
        self.action_queue = super::ActionQueue::from_snapshot(queue_snapshot);

        // Restore RNG
        self.original_seed = snapshot.rng_seed;
        self.rng = ChaCha8Rng::seed_from_u64(snapshot.rng_seed);
        self.rng.set_word_pos(snapshot.rng_word_pos);

        Ok(())
    }

    // ========================================
    // FIX_2601/0112: ScenarioRunner Support
    // ========================================

    /// Set player position by track_id (0-21).
    pub fn set_player_position(&mut self, track_id: usize, pos: super::types::Coord10) {
        if track_id < self.player_positions.len() {
            self.player_positions[track_id] = pos;
        }
    }

    /// Set ball position.
    pub fn set_ball_position(&mut self, pos: super::types::Coord10) {
        self.ball.position = pos;
    }

    /// Set ball owner by track_id.
    pub fn set_ball_owner(&mut self, track_id: usize) {
        self.ball.current_owner = Some(track_id);
        self.action_queue.set_ball_state(super::action_queue::BallState::Controlled {
            owner_idx: track_id,
        });
    }

    /// Set home team attack direction.
    pub fn set_home_attacks_right(&mut self, attacks_right: bool) {
        self.is_second_half = !attacks_right;
        self.home_ctx = DirectionContext { is_home: true, attacks_right };
        self.away_ctx = DirectionContext { is_home: false, attacks_right: !attacks_right };
    }

    /// Get ball position in meters.
    pub fn get_ball_position_meters(&self) -> (f32, f32) {
        self.ball.position.to_meters()
    }

    /// Get home team score.
    pub fn get_home_score(&self) -> u8 {
        self.result.score_home
    }

    /// Get away team score.
    pub fn get_away_score(&self) -> u8 {
        self.result.score_away
    }

    /// Check if a specific event type occurred (by event name string).
    pub fn has_event_type(&self, event_name: &str) -> bool {
        self.result.events.iter().any(|e| {
            let type_name = format!("{:?}", e.event_type).to_lowercase();
            type_name.contains(&event_name.to_lowercase())
        })
    }

    /// Get a metric value by name (for calibration probes).
    pub fn get_metric(&self, metric_name: &str) -> Option<f32> {
        match metric_name {
            "shots" => Some((self.result.statistics.shots_home + self.result.statistics.shots_away) as f32),
            "goals" => Some((self.result.score_home + self.result.score_away) as f32),
            "offsides" => Some((self.offside_count_home + self.offside_count_away) as f32),
            "passes" => Some((self.home_stat_snapshot.pass_attempts + self.away_stat_snapshot.pass_attempts) as f32),
            "progressive_passes" => Some((self.home_stat_snapshot.progressive_passes + self.away_stat_snapshot.progressive_passes) as f32),
            "long_passes" => Some((self.home_stat_snapshot.long_passes + self.away_stat_snapshot.long_passes) as f32),
            "crosses" => Some((self.home_stat_snapshot.crosses + self.away_stat_snapshot.crosses) as f32),
            "key_passes" => Some((self.home_stat_snapshot.key_passes + self.away_stat_snapshot.key_passes) as f32),
            "interceptions" => Some((self.home_stat_snapshot.interceptions + self.away_stat_snapshot.interceptions) as f32),
            "backward_passes" => Some((self.home_stat_snapshot.backward_passes + self.away_stat_snapshot.backward_passes) as f32),
            "lateral_passes" => Some((self.home_stat_snapshot.lateral_passes + self.away_stat_snapshot.lateral_passes) as f32),
            "headers" => Some(0.0), // TODO: Track headers
            _ => None,
        }
    }

    /// Get reference to home team stat snapshot (for calibration).
    pub fn get_home_stat_snapshot(&self) -> &MatchStatSnapshot {
        &self.home_stat_snapshot
    }

    /// Get reference to away team stat snapshot (for calibration).
    pub fn get_away_stat_snapshot(&self) -> &MatchStatSnapshot {
        &self.away_stat_snapshot
    }

    // ========== FIX_2601/0123: GameFlowMachine Accessors ==========

    /// Get current game flow state
    pub fn game_flow_state(&self) -> &match_state::GameFlowState {
        self.game_flow_machine.current()
    }

    /// Check if clock should be running based on game flow state
    pub fn is_clock_running(&self) -> bool {
        self.game_flow_machine.current().is_clock_running()
    }

    /// Check if in dead ball state
    pub fn is_dead_ball(&self) -> bool {
        self.game_flow_machine.current().is_dead_ball()
    }

    /// Check if a restart is pending
    pub fn is_restart_pending(&self) -> bool {
        self.game_flow_machine.current().is_restart_pending()
    }

    /// Check if players can make decisions
    pub fn allows_player_decisions(&self) -> bool {
        self.game_flow_machine.current().allows_player_decisions()
    }

    // ========== FIX_2601/0123: RuleDispatcher Accessors ==========

    /// Get rule evaluation statistics
    pub fn rule_stats(&self) -> &rules::RuleEvaluationStats {
        self.rule_dispatcher.stats()
    }

    /// Check if team bias is within acceptable range
    pub fn is_rule_bias_acceptable(&self) -> bool {
        self.rule_dispatcher.stats().is_bias_acceptable()
    }

    /// Get team bias ratio (0.5 = perfect balance)
    pub fn rule_bias_ratio(&self) -> f32 {
        self.rule_dispatcher.stats().team_bias_ratio()
    }

    /// Get current rule check mode
    pub fn rule_check_mode(&self) -> rules::RuleCheckMode {
        self.rule_check_mode
    }

    /// Set rule check mode for testing or migration
    pub fn set_rule_check_mode(&mut self, mode: rules::RuleCheckMode) {
        self.rule_check_mode = mode;
    }
}

// ============================================
// Phase 7: Live Match Streaming Helper Methods
// ============================================
// (Moved to state_accessors.rs as part of P2-9 refactoring)

#[cfg(test)]
impl MatchEngine {
    fn set_test_player_positions(&mut self, positions: Vec<(f32, f32)>) {
        self.test_player_positions = Some(positions);
    }
}

fn instruction_strength_multiplier(instructions: &TeamInstructions) -> f32 {
    let defensive_line_mod = instructions.defensive_line.to_numeric() as f32 * 0.01;
    let tempo_mod = instructions.team_tempo.to_numeric() as f32 * 0.012;
    let pressing_mod = instructions.pressing_intensity.to_numeric() as f32 * 0.01;
    let width_mod = instructions.team_width.to_numeric() as f32 * 0.005;
    let build_mod = match instructions.build_up_style {
        BuildUpStyle::Short => -0.01,
        BuildUpStyle::Mixed => 0.0,
        BuildUpStyle::Direct => 0.015,
    };

    (1.0 + defensive_line_mod + tempo_mod + pressing_mod + width_mod + build_mod).clamp(0.9, 1.1)
}

// TODO: Fix tests.rs to use updated PlayerAttributes API
// #[cfg(test)]
// mod tests;

#[cfg(test)]
mod rulebook_stoppage_time_tests {
    use super::*;

    fn emit_simple_event(engine: &mut MatchEngine, minute: u8, event_type: EventType) {
        engine.minute = minute;
        engine.current_timestamp_ms = minute as u64 * 60_000;
        engine.emit_event(MatchEvent {
            minute,
            timestamp_ms: Some(engine.current_timestamp_ms),
            event_type,
            is_home_team: true,
            player_track_id: Some(0),
            target_track_id: None,
            details: None,
        });
    }

    #[test]
    fn first_half_stoppage_accumulates_and_finalizes_at_minute_45() {
        let mut engine = test_fixtures::create_test_engine();
        engine.is_second_half = false;
        engine.stoppage_seconds_first_half = 0;
        engine.stoppage_finalized_first_half = false;
        engine.first_half_end_minute = HALF_DURATION_MINUTES;
        engine.stoppage_finalized_second_half = false;
        engine.match_end_minute = REGULATION_TOTAL_MINUTES;

        emit_simple_event(&mut engine, 40, EventType::Substitution);
        emit_simple_event(&mut engine, 41, EventType::Substitution);
        emit_simple_event(&mut engine, 42, EventType::Substitution);

        assert_eq!(engine.stoppage_seconds_first_half, 90);

        engine.minute = HALF_DURATION_MINUTES;
        engine.maybe_finalize_first_half_stoppage_time();

        // 90 seconds => ceil(90/60) = 2 minutes
        assert_eq!(engine.first_half_end_minute, 47);
        assert_eq!(engine.regulation_end_minute(), 92);
        assert_eq!(engine.match_end_minute, 92);
        assert!(engine.stoppage_finalized_first_half);
    }

    #[test]
    fn first_half_stoppage_is_capped() {
        let mut engine = test_fixtures::create_test_engine();
        engine.is_second_half = false;
        engine.stoppage_seconds_first_half = 0;
        engine.stoppage_finalized_first_half = false;
        engine.first_half_end_minute = HALF_DURATION_MINUTES;
        engine.stoppage_finalized_second_half = false;
        engine.match_end_minute = REGULATION_TOTAL_MINUTES;

        // 20 injuries => 20 * 60s = 1200s => 20 minutes, but cap is 8 minutes.
        for m in 10..30 {
            emit_simple_event(&mut engine, m, EventType::Injury);
        }

        engine.minute = HALF_DURATION_MINUTES;
        engine.maybe_finalize_first_half_stoppage_time();

        assert_eq!(engine.first_half_end_minute, 53);
        assert_eq!(engine.regulation_end_minute(), 98);
        assert_eq!(engine.match_end_minute, 98);
        assert!(engine.stoppage_finalized_first_half);
    }

    #[test]
    fn second_half_stoppage_accumulates_and_finalizes_at_minute_90() {
        let mut engine = test_fixtures::create_test_engine();
        engine.is_second_half = true;
        engine.stoppage_seconds_second_half = 0;
        engine.stoppage_finalized_second_half = false;
        engine.first_half_end_minute = HALF_DURATION_MINUTES;
        engine.match_end_minute = engine.regulation_end_minute();

        emit_simple_event(&mut engine, 70, EventType::Substitution);
        emit_simple_event(&mut engine, 71, EventType::Substitution);
        emit_simple_event(&mut engine, 72, EventType::Substitution);

        assert_eq!(engine.stoppage_seconds_second_half, 90);

        engine.minute = engine.regulation_end_minute();
        engine.maybe_finalize_second_half_stoppage_time();

        // 90 seconds => ceil(90/60) = 2 minutes
        assert_eq!(engine.match_end_minute, 92);
        assert!(engine.stoppage_finalized_second_half);
    }

    #[test]
    fn second_half_stoppage_is_capped() {
        let mut engine = test_fixtures::create_test_engine();
        engine.is_second_half = true;
        engine.stoppage_seconds_second_half = 0;
        engine.stoppage_finalized_second_half = false;
        engine.first_half_end_minute = HALF_DURATION_MINUTES;
        engine.match_end_minute = engine.regulation_end_minute();

        // 20 injuries => 20 * 60s = 1200s => 20 minutes, but cap is 8 minutes.
        for m in 70..90 {
            emit_simple_event(&mut engine, m, EventType::Injury);
        }

        engine.minute = engine.regulation_end_minute();
        engine.maybe_finalize_second_half_stoppage_time();

        assert_eq!(engine.match_end_minute, 98);
        assert!(engine.stoppage_finalized_second_half);
    }
}




