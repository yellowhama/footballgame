//! Tick-Based Simulation for MatchEngine
//!
//! Phase 3.5: ActionQueue Integration
//!
//! This module contains the tick-based simulation logic that integrates:
//! - ActionQueue for action scheduling/execution
//! - PositioningEngine for Off-the-Ball movement
//! - PepGrid for channel-based positioning
//! - TeamPhaseState for tactical phase management
//!
//! # Data Flow Overview (2025-12-11)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         TICK-BASED ENGINE                               │
//! │                      (tick_based.rs)                                    │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  simulate_minute_tick_based() - 240 ticks/minute (4 ticks/sec)          │
//! │     │                                                                   │
//! │     ├─► self.ball.position        (normalized 0-1)                      │
//! │     ├─► self.player_positions[]   (normalized 0-1, 22 players)          │
//! │     ├─► self.ball.velocity        (normalized)                          │
//! │     ├─► self.ball.height          (meters, 0-2)                         │
//! │     │                                                                   │
//! │     └─► record_positions_for_tick()                                     │
//! │            │                                                            │
//! │            ├─► normalized_to_meters() converts (0-1) → (0-105, 0-68)    │
//! │            │                                                            │
//! │            └─► self.result.position_data                                │
//! │                   ├─► ball: Vec<PositionDataItem>                       │
//! │                   │      • timestamp_ms (250ms intervals)               │
//! │                   │      • position: (f32, f32) in METERS               │
//! │                   │      • height: Option<f32>                          │
//! │                   │      • velocity: Option<(f32, f32)>                 │
//! │                   │                                                     │
//! │                   └─► players: HashMap<u8, Vec<PositionDataItem>>       │
//! │                          • 22 players (0-10 home, 11-21 away)           │
//! │                          • same fields as ball                          │
//! │                                                                         │
//! │  Events & Score (already working):                                      │
//! │     ├─► self.result.events.push()      → MatchEvent list                │
//! │     └─► self.result.score_home/away    → final score                    │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         MATCH RESULT                                    │
//! │                    (models/match_result.rs)                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  MatchResult {                                                          │
//! │     score_home: u8,                                                     │
//! │     score_away: u8,                                                     │
//! │     events: Vec<MatchEvent>,           → goals, cards, subs             │
//! │     position_data: Option<MatchPositionData>,  → replay positions       │
//! │     replay_events: Option<Vec<ReplayEvent>>,   → detailed replay        │
//! │     statistics: MatchStatistics,                                        │
//! │     ...                                                                 │
//! │  }                                                                      │
//! └─────────────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      GODOT EXTENSION                                    │
//! │                  (godot_extension crate)                                │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Serializes MatchResult to binary format:                               │
//! │     • Ball frames: [timestamp_ms, x, y, height, vx, vy] × N             │
//! │     • Player frames: per-player position arrays                         │
//! │     • Events: serialized MatchEvent list                                │
//! │                                                                         │
//! │  Passes to Godot via GDExtension bindings                               │
//! └─────────────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                       GODOT VIEWER                                      │
//! │                    (GDScript replay)                                    │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  ReplayLoader.gd:                                                       │
//! │     • Parses binary data                                                │
//! │     • Builds frame arrays for interpolation                             │
//! │                                                                         │
//! │  ReplayController.gd:                                                   │
//! │     • Interpolates positions between frames (250ms → smooth)            │
//! │     • Moves ball/player sprites on screen                               │
//! │     • Syncs events with timeline                                        │
//! │                                                                         │
//! │  CONTRACT:                                                              │
//! │     • timestamp_ms / 1000.0 == ReplayEvent.base.t (seconds)             │
//! │     • position in METERS (0-105 × 0-68), viewer scales to pixels        │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Coordinate Systems
//!
//! | Location | Coordinate System | Range |
//! |----------|-------------------|-------|
//! | `self.ball.position` | Normalized | (0-1, 0-1) |
//! | `self.player_positions[]` | Normalized | (0-1, 0-1) |
//! | `MatchPositionData` | Meters | (0-105, 0-68) |
//! | Godot Viewer | Pixels | scaled by viewer |
//!
//! # Timing
//!
//! | Constant | Value | Description |
//! |----------|-------|-------------|
//! | `TICKS_PER_MINUTE` | 240 | 4 ticks per second |
//! | `TICK_DURATION_MS` | 250 | milliseconds per tick |
//! | 90 min match | 21,600 frames | 240 × 90 |

use rand::Rng;

use super::MatchEngine;
use super::match_state::{
    GameFlowState, MatchPlayerId, MatchPosition, TeamId, TransitionTrigger,
};
use super::rules::{
    check_goal_wrapper, check_handball_wrapper, check_offside_wrapper, check_out_of_play_wrapper,
    HandballContactEvent, LegacyGoalResult, LegacyHandballResult, LegacyOffsideResult,
    LegacyOutOfPlayResult, PassEvent, RuleCheckMode, RuleDecision, RuleRestartType, RuleTeamId,
};
use crate::engine::action_queue::{
    execute_dribble,
    execute_header,
    execute_intercept,
    execute_move,
    execute_pass,
    execute_save,
    execute_shot,
    execute_tackle,
    execute_trap,
    ActionResult,
    ActionType,
    BallState,
    ExecutionContext,
    PlayerStats,
    RestartType,
    ScheduledAction,
    TackleEvent,
    // P0: Core types moved to action_queue
    TackleOutcome,
    TackleType,
    ViewerEvent,
};
use crate::engine::debug_flags::match_debug_enabled;
// P7 Phase 9: FSM imports (2025-12-14: ActionQueue 통합 후 최소화)
use crate::engine::coordinates;
use crate::engine::set_pieces::{check_ball_out, OutOfBoundsResult};
use crate::engine::duel::{
    decide_defensive_action, resolve_duel, AttackerAction, DefensiveAction, DuelOutcome,
};
use crate::engine::growth::{
    calculate_dribble_difficulty, calculate_pass_difficulty, calculate_pressure, HeroActionTag,
    HeroMatchGrowth, HeroXpEvent, PlayerAttribute,
};
use crate::engine::phase_action::{TACKLE_COOLDOWN_TICKS, TACKLE_MAX_DISTANCE};
use crate::engine::physics_constants::field;
use crate::engine::player_objective::PlayerObjective;
use crate::engine::player_objective::{assign_objective, ObjectiveContext};
use crate::engine::player_state::PlayerState;
use crate::engine::types::{Coord10, DirectionContext, TeamViewCoord10, Vel10}; // FIX_2512 Phase 4 - TASK_09
use crate::models::MatchEvent;
use crate::models::TeamSide;
use crate::replay::types::{MeterPos, PossessionChangeType};
// FIX_2601/0112: Statistical Anchor Calibration
use crate::calibration::{
    pos_to_zone_for_team, pos_to_posplay_zone_for_team, ClassifierThresholds, NormPos,
    PassType as CalibPassType,
};
use crate::calibration::pass_classifier::classify_pass_detailed;

/// 분당 틱 수: 4틱/초 × 60초 = 240틱/분
/// 선수들이 초당 4번 결정을 내림 (250ms 간격)
const TICKS_PER_MINUTE: u64 = 240;
const PHASE0_PRESSING_INTENSITY: [f32; 22] = [0.0; 22];

/// 두 점 사이 거리 계산
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}

#[inline]
fn team_view_context(engine: &MatchEngine, is_home: bool) -> DirectionContext {
    if is_home { engine.home_ctx } else { engine.away_ctx }
}

#[inline]
fn world_forward_dir_x(attacks_right: bool) -> f32 {
    if attacks_right { 1.0 } else { -1.0 }
}

impl MatchEngine {
    // ========== ExecutionContext Builder ==========

    /// ActionQueue ExecutionContext 생성
    /// Phase 2 최적화: original_seed 사용으로 RNG clone 제거
    /// (simple_random에서 current_tick과 결합되어 결정적)
    pub(crate) fn build_execution_context(&self) -> ExecutionContext {
        let exp = self.get_exp_params();
        ExecutionContext {
            player_positions: self.player_positions_in_meters(),
            player_stats: self.build_player_stats(),
            goalkeeper_indices: (0, 11), // GK는 항상 0번, 11번
            current_tick: self.current_tick,
            rng_seed: self.original_seed, // Phase 2: RNG clone 제거
            // FIX_2601/0110: DirectionContext 전달 (하프타임 스왑 반영)
            home_ctx: self.home_ctx,
            away_ctx: self.away_ctx,
            home_match_modifiers: self.home_match_modifiers,
            away_match_modifiers: self.away_match_modifiers,
            rulebook_non_gk_handball_enabled: exp.non_gk_handball_enabled,
            rulebook_non_gk_handball_prob_mult: exp.non_gk_handball_prob_mult,
            rulebook_advantage_play_enabled: exp.advantage_play_enabled,
            // FIX_2601/1120: Pass actual ball position to prevent InFlight origin teleportation
            ball_position: self.ball.position.to_meters(),
        }
    }

    /// 22명 PlayerStats 생성 (기존 Team 데이터에서)
    fn build_player_stats(&self) -> Vec<PlayerStats> {
        let mut stats = Vec::with_capacity(22);

        // Helper to extract stats from current pitch occupant (assignment-aware)
        let extract_stats = |player: &crate::models::MatchPlayer| -> PlayerStats {
            let attrs = &player.attributes;
            PlayerStats {
                passing: attrs.passing,
                first_touch: attrs.first_touch,
                dribbling: attrs.dribbling,
                finishing: attrs.finishing,
                long_shots: attrs.long_shots,
                tackling: attrs.tackling,
                anticipation: attrs.anticipation,
                composure: attrs.composure,
                agility: attrs.agility,
                // GK attributes - keep legacy fallback (overall-based) for now
                reflexes: player.overall,
                handling: player.overall,
                // FIX_2601/0109: GK positioning/diving
                positioning: attrs.positioning,
                diving: ((attrs.agility as u16 + attrs.jumping as u16) / 2) as u8,
                // 2025-12-11 P2: Header action stats
                heading: attrs.heading,
                jumping: attrs.jumping,
                strength: attrs.strength,
                // ActionModel Integration: Pass 추가 필드
                technique: attrs.technique,
                vision: attrs.vision,
                decisions: attrs.decisions,
                // FIX_2601/0102: Tackle Score Calculation
                aggression: attrs.aggression,
                bravery: attrs.bravery,
                // FIX_2601/0107: FM meta attributes
                concentration: attrs.concentration,
                pace: attrs.pace,
                acceleration: attrs.acceleration,
                balance: attrs.balance,
                teamwork: attrs.teamwork,
                flair: attrs.flair,
                condition_level: player.condition_level,
            }
        };

        // Pitch slots (0-21): starter slots remain stable, occupants can change via substitutions.
        for track_id in 0..22 {
            stats.push(extract_stats(self.get_match_player(track_id)));
        }

        stats
    }

    /// Coord10 좌표를 미터로 변환한 위치 배열
    fn player_positions_in_meters(&self) -> Vec<(f32, f32)> {
        self.player_positions.iter().map(|p| p.to_meters()).collect()
    }

    /// 가장 가까운 상대 선수까지의 거리 (미터 기준)
    /// 수비수가 없으면 None 반환
    pub(crate) fn find_nearest_opponent_distance(
        &self,
        player_idx: usize,
        is_home: bool,
    ) -> Option<f32> {
        use crate::engine::types::Coord10;
        let player_pos = self.player_positions.get(player_idx)?;
        let opponent_range = TeamSide::opponent_range_for_home(is_home);

        opponent_range
            .filter_map(|i| {
                self.player_positions.get(i).map(|opp_pos| {
                    // Coord10::distance_to returns 0.1m units, convert to meters
                    player_pos.distance_to(opp_pos) as f32 / Coord10::SCALE
                })
            })
            .reduce(f32::min)
    }

    // ========== ActionResult → MatchEvent Conversion ==========

    /// Calculate current time in seconds for replay recording
    pub(crate) fn current_time_seconds(&self) -> f64 {
        // current_tick is absolute tick since start of match
        // TICKS_PER_MINUTE = 240 ticks/minute = 4 ticks/second
        self.current_tick as f64 / 4.0
    }

    /// C5: Calculate current timestamp in milliseconds for Event SSOT
    /// Returns precise timestamp based on tick timing (250ms per tick)
    pub(crate) fn current_timestamp_ms(&self) -> u64 {
        let tick_within_minute = self.current_tick % TICKS_PER_MINUTE;
        self.minute as u64 * 60_000 + tick_within_minute * 250
    }

    /// Convert normalized position to MeterPos for replay (standalone function)
    /// Uses coordinates module for consistent x/y swap:
    /// - normalized.0 (width) -> meters.y (width)
    /// - normalized.1 (length) -> meters.x (length)
    /// FIX_2601: normalized to meters via Coord10
    fn normalized_to_meter_pos(normalized: (f32, f32)) -> MeterPos {
        use crate::engine::types::Coord10;
        let coord = Coord10::from_normalized_legacy(normalized);
        let meters = coord.to_meters();
        MeterPos { x: meters.0 as f64, y: meters.1 as f64 }
    }

    /// ActionResult를 MatchEvent로 변환하여 emit
    /// Also records to ReplayRecorder if enabled (P6)
    pub(crate) fn emit_action_result(&mut self, result: &ActionResult) {
        // Get time once for replay recording
        let t_seconds = self.current_time_seconds();
        self.record_action_result_metrics(result);

        match result {
            ActionResult::PassStarted { passer_idx, receiver_idx, intended_target_pos, intended_passer_pos, .. } => {
                let is_home = TeamSide::is_home(*passer_idx);
                let team_id = if is_home { 0 } else { 1 };

                // Phase 0: pass diagnostics (progress + options)
                // FIX_2601/1128: 선택 시점 위치(intended_target_pos) 전달
                self.record_phase0_pass(*passer_idx, *receiver_idx, *intended_target_pos);

                // Statistics: 패스 시도 기록
                self.record_pass_attempt(*passer_idx, *receiver_idx);

                // C6: Use passer_idx directly as track_id
                let ball_pos_m = self.ball.position_meters();
                // FIX_2601/1128: intended_target_pos를 Coord10 단위로 변환하여 이벤트에 전달
                let intended_pos_coord10 = intended_target_pos.map(|pos| (pos.x as f32, pos.y as f32));
                // FIX_2601/1129: intended_passer_pos를 Coord10 단위로 변환하여 이벤트에 전달
                let intended_passer_coord10 = intended_passer_pos.map(|pos| (pos.x as f32, pos.y as f32));

                // FIX_2601/0123: Compute is_forward_pass at decision time with correct attacks_right
                // This avoids halftime direction issues in QA metrics
                // Use intended_target_pos from action and passer's current position
                let attacks_right = self.attacks_right(is_home);

                // Get passer position - prefer intended but use current as fallback
                let passer_x = intended_passer_pos
                    .map(|p| p.x)
                    .or_else(|| self.player_positions.get(*passer_idx).map(|p| p.x));

                // Use ONLY intended_target_pos - this was computed at decision time
                let target_x = intended_target_pos.map(|p| p.x);

                let is_forward_pass = match (passer_x, target_x) {
                    (Some(px), Some(tx)) => {
                        // 7m threshold = 70 Coord10 units (field uses 0.1m units)
                        let dx = tx - px;
                        let forward_distance = if attacks_right { dx } else { -dx };
                        let is_forward = forward_distance >= 70;

                        Some(is_forward)
                    }
                    _ => {
                        None
                    }
                };

                self.emit_event(
                    MatchEvent::pass(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *passer_idx,
                        (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(Some(*receiver_idx))
                    .with_intended_target_pos(intended_pos_coord10)
                    .with_intended_passer_pos(intended_passer_coord10)
                    .with_is_forward_pass(is_forward_pass),
                );

                // P6: Record to ReplayRecorder
                // Note: from/to positions are derived from current ball and receiver positions
                // Compute values before borrowing recorder to avoid borrow checker issues
                use crate::engine::types::Coord10;
                let ball_pos_m = self.ball.position.to_meters();
                let receiver_pos =
                    self.player_positions.get(*receiver_idx).copied().unwrap_or(Coord10::CENTER);
                let receiver_m = receiver_pos.to_meters();
                let from_m = MeterPos { x: ball_pos_m.0 as f64, y: ball_pos_m.1 as f64 };
                let to_m = MeterPos { x: receiver_m.0 as f64, y: receiver_m.1 as f64 };
                let distance_m = ((to_m.x - from_m.x).powi(2) + (to_m.y - from_m.y).powi(2)).sqrt();

                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_pass(
                        t_seconds,
                        team_id,
                        *passer_idx as u32,
                        from_m,
                        to_m,
                        Some(*receiver_idx as u32),
                        distance_m,
                    );
                }
            }

            ActionResult::ShotTaken { shooter_idx, xg, target, .. } => {
                self.finalize_pass_sequences();
                let is_home = TeamSide::is_home(*shooter_idx);
                let team_id = if is_home { 0 } else { 1 };

                // Contract Probe:
                // 현재 구현은 ShotTaken을 on_target=true로 기록한다.
                // 만약 실제로 off-target/blocked 결과가 존재하는데 여기서 항상 true면,
                // 데이터 계약이 깨진 것이므로 디버그 빌드/CI에서 탐지할 수 있게 카운트한다.
                // (실제 ShotOutcome 결정은 ActionResult 생성 레이어에서 OutcomeSet으로 해야 맞음)
                if cfg!(debug_assertions) {
                    // self.result.statistics.debug_shot_taken_count += 1;
                }

                let ball_pos_m = self.ball.position_meters();
                self.emit_event(
                    MatchEvent::shot(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *shooter_idx,
                        true,
                        *xg,
                    )
                    .with_ball_position((
                        ball_pos_m.0,
                        ball_pos_m.1,
                        self.ball.height_meters(),
                    )),
                );

                // Balance diagnostics: shot attempt (on-target path)
                self.balance_diagnostics.record_shot(*shooter_idx);

                // 통계 업데이트
                if is_home {
                    self.result.statistics.shots_home += 1;
                    self.result.statistics.shots_on_target_home += 1;
                    self.result.statistics.xg_home += xg;
                } else {
                    self.result.statistics.shots_away += 1;
                    self.result.statistics.shots_on_target_away += 1;
                    self.result.statistics.xg_away += xg;
                }
                // FIX_2601/0112: Record to calibration snapshot
                self.record_shot_for_calibration(*shooter_idx, true, false, *xg);
                // NOTE: Shot budget tracking done in record_shot_attempt() only

                // P6: Record to ReplayRecorder
                // FIX_2601: Coord10.to_meters() already returns meters, convert to MeterPos directly
                let ball_pos_m = self.ball.position.to_meters();
                let from_m = MeterPos { x: ball_pos_m.0 as f64, y: ball_pos_m.1 as f64 };
                let target_meters = target.to_meters();
                let target_m = MeterPos { x: target_meters.0 as f64, y: target_meters.1 as f64 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_shot(
                        t_seconds,
                        team_id,
                        *shooter_idx as u32,
                        from_m,
                        target_m,
                        true, // on_target
                        Some(*xg as f64),
                    );
                }
            }

            ActionResult::GoalScored { scorer_idx, assist_idx, xg } => {
                // FIX_2601/0116: Debug GK goals via action system
                #[cfg(debug_assertions)]
                {
                    let is_gk = *scorer_idx == 0 || *scorer_idx == 11;
                    if is_gk {
                        let scorer_pos = self.player_positions[*scorer_idx].to_meters();
                        let ball_pos = self.ball.position.to_meters();
                        eprintln!(
                            "[GK_GOAL_ACTION] min={} is_2nd_half={} scorer={} scorer_pos=({:.1},{:.1}) ball_pos=({:.1},{:.1}) xg={:.3}",
                            self.minute, self.is_second_half, scorer_idx, scorer_pos.0, scorer_pos.1, ball_pos.0, ball_pos.1, xg
                        );
                    }
                }
                // v10: 바로 on_goal_scored 호출하여 점수 증가
                let is_home = TeamSide::is_home(*scorer_idx);
                let scoring_team = if is_home {
                    crate::engine::tactical_context::TeamSide::Home
                } else {
                    crate::engine::tactical_context::TeamSide::Away
                };

                // Balance diagnostics: goal implies a shot on target
                self.balance_diagnostics.record_shot(*scorer_idx);
                self.balance_diagnostics.record_shot_goal(*scorer_idx);

                // FIX_2601/0115b: 골도 슈팅 통계 + xG에 포함
                if is_home {
                    self.result.statistics.shots_home += 1;
                    self.result.statistics.shots_on_target_home += 1;
                    self.result.statistics.xg_home += xg;
                } else {
                    self.result.statistics.shots_away += 1;
                    self.result.statistics.shots_on_target_away += 1;
                    self.result.statistics.xg_away += xg;
                }
                // FIX_2601/0112: Record to calibration snapshot (goal = on-target + goal)
                self.record_shot_for_calibration(*scorer_idx, true, true, *xg);
                // NOTE: Shot budget tracking done in record_shot_attempt() only

                // 득점자/어시스트 정보를 공에 기록 (on_goal_scored에서 사용)
                self.ball.current_owner = Some(*scorer_idx);
                if let Some(assist) = assist_idx {
                    self.ball.previous_owner = Some(*assist);
                }

                // 점수 증가 + 이벤트 발생 + 킥오프 재시작
                self.on_goal_scored(scoring_team);
            }

            ActionResult::TackleFoul { tackler_idx, target_idx } => {     
                self.finalize_pass_sequences();
                // Restart pulse (set piece will follow)
                self.restart_occurred_this_tick = true;
                self.restart_type_this_tick = Some(RestartType::FreeKick);

                let is_home = TeamSide::is_home(*tackler_idx);
                let team_id = if is_home { 0 } else { 1 };
                // 파울 당한 팀이 프리킥 받음
                let receiving_team_id = if is_home { 1u32 } else { 0u32 };
                // C6: Use tackler_idx directly as track_id
                let ball_pos_m = self.ball.position_meters();
                self.emit_event(
                    MatchEvent::foul(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *tackler_idx,
                        (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(Some(*target_idx)),
                );
                if is_home {
                    self.result.statistics.fouls_home += 1;
                } else {
                    self.result.statistics.fouls_away += 1;
                }

                // P6: Record to ReplayRecorder (foul + free kick)
                let ball_pos_m2 = self.ball.position.to_meters();
                let at_m = MeterPos { x: ball_pos_m2.0 as f64, y: ball_pos_m2.1 as f64 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_foul(t_seconds, team_id, *tackler_idx as u32, at_m);
                    // 파울 위치에서 프리킥
                    recorder.record_free_kick(t_seconds + 0.1, receiving_team_id, at_m);
                }

                // FIX_2601/0109: 파울 후 프리킥 재개 실제 적용
                // 기존에는 ball_state만 OutOfPlay로 설정하고 실제 재개가 안 됨
                if let BallState::OutOfPlay {
                    restart_type,
                    position,
                    home_team,
                } = self.action_queue.ball_state()
                {
                    self.apply_restart_with_replay(
                        *restart_type,
                        *position,
                        *home_team,
                        false, // no boundary recording for foul
                        t_seconds,
                    );
                }
            }

            ActionResult::TackleFoulAdvantage { tackler_idx, target_idx } => {
                self.finalize_pass_sequences();

                // No restart pulse: play continues (advantage).
                let is_home = TeamSide::is_home(*tackler_idx);
                let team_id = if is_home { 0 } else { 1 };

                let ball_pos_m = self.ball.position_meters();
                self.emit_event(
                    MatchEvent::foul(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *tackler_idx,
                        (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(Some(*target_idx))
                    .with_advantage_played(true),
                );

                if is_home {
                    self.result.statistics.fouls_home += 1;
                } else {
                    self.result.statistics.fouls_away += 1;
                }

                // Replay: record foul only (no free kick restart).
                let ball_pos_m2 = self.ball.position.to_meters();
                let at_m = MeterPos { x: ball_pos_m2.0 as f64, y: ball_pos_m2.1 as f64 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_foul(t_seconds, team_id, *tackler_idx as u32, at_m);
                }
            }

            ActionResult::GoalkeeperHandlingViolation { goalkeeper_idx, last_touch_idx, is_indirect, xg } => {
                self.finalize_pass_sequences();
                // Restart pulse (set piece will follow)
                self.restart_occurred_this_tick = true;
                self.restart_type_this_tick = Some(RestartType::FreeKick);

                let is_home = TeamSide::is_home(*goalkeeper_idx);
                let team_id = if is_home { 0 } else { 1 };

                // If this originated from a shot, keep shot stats/xG consistent.
                if let (Some(shooter_idx), Some(shot_xg)) = (*last_touch_idx, *xg) {
                    self.balance_diagnostics.record_shot(shooter_idx);
                    self.balance_diagnostics.record_shot_saved(shooter_idx);

                    let shooter_is_home = TeamSide::is_home(shooter_idx);
                    if shooter_is_home {
                        self.result.statistics.shots_home += 1;
                        self.result.statistics.shots_on_target_home += 1;
                        self.result.statistics.xg_home += shot_xg;
                    } else {
                        self.result.statistics.shots_away += 1;
                        self.result.statistics.shots_on_target_away += 1;
                        self.result.statistics.xg_away += shot_xg;
                    }
                }

                let foul_pos = match self.action_queue.ball_state() {
                    BallState::OutOfPlay { position, .. } => *position,
                    _ => self.player_positions[*goalkeeper_idx],
                };
                let foul_pos_m = foul_pos.to_meters();

                self.emit_event(
                    MatchEvent::foul(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *goalkeeper_idx,
                        (foul_pos_m.0, foul_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(*last_touch_idx),
                );
                if is_home {
                    self.result.statistics.fouls_home += 1;
                } else {
                    self.result.statistics.fouls_away += 1;
                }

                // P6: Record to ReplayRecorder (foul + free kick)
                let at_m = MeterPos { x: foul_pos_m.0 as f64, y: foul_pos_m.1 as f64 };
                let receiving_team_id = if is_home { 1u32 } else { 0u32 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_foul(t_seconds, team_id, *goalkeeper_idx as u32, at_m);
                    recorder.record_free_kick(t_seconds + 0.1, receiving_team_id, at_m);
                }

                if *is_indirect {
                    self.pending_indirect_free_kick = true;
                }

                // Apply restart
                if let BallState::OutOfPlay { restart_type, position, home_team } =
                    self.action_queue.ball_state()
                {
                    self.apply_restart_with_replay(
                        *restart_type,
                        *position,
                        *home_team,
                        false, // no boundary recording for foul
                        t_seconds,
                    );
                }
            }

            ActionResult::HandballFoul { offender_idx, last_touch_idx } => {
                self.finalize_pass_sequences();
                // Restart pulse (set piece will follow)
                self.restart_occurred_this_tick = true;

                // Executor layer is expected to set the canonical restart in ActionQueue (SSOT).
                let restart_type = match self.action_queue.ball_state() {
                    BallState::OutOfPlay { restart_type, .. } => *restart_type,
                    _ => RestartType::FreeKick,
                };
                self.restart_type_this_tick = Some(restart_type);

                let is_home = TeamSide::is_home(*offender_idx);
                let team_id = if is_home { 0 } else { 1 };

                let foul_pos = match self.action_queue.ball_state() {
                    BallState::OutOfPlay { position, .. } => *position,
                    _ => self.player_positions[*offender_idx],
                };
                let foul_pos_m = foul_pos.to_meters();

                // FIX_2601/0123 Phase 6: Track handball using handball wrapper
                // Supports A/B comparison and DispatcherPrimary mode
                if self.rule_check_mode.tracking_enabled() || self.rule_check_mode.dispatcher_applies() {
                    let last_touch_team = if let Some(idx) = self.ball.current_owner {
                        RuleTeamId::from_player_index(idx)
                    } else {
                        RuleTeamId::Home
                    };

                    // Create legacy result for comparison
                    let legacy_result = LegacyHandballResult {
                        is_handball: true,
                        player_idx: *offender_idx,
                        position: foul_pos.clone(),
                        deliberate: true, // Legacy system assumes deliberate when called
                    };

                    // Create handball contact event for detailed evaluation
                    // Note: Full handball contact details not available from ActionResult,
                    // using basic estimation
                    let handball_contact = HandballContactEvent {
                        player_idx: *offender_idx,
                        position: foul_pos.clone(),
                        unnatural_position: true, // Assumed by legacy detection
                        arm_extension: 0.5, // Default estimate
                        deliberate_movement: restart_type == RestartType::Penalty, // Penalty suggests deliberate
                        gained_advantage: restart_type == RestartType::Penalty, // DOGSO
                    };

                    // Run through wrapper for A/B comparison and statistics
                    let _decision = check_handball_wrapper(
                        self.rule_check_mode,
                        &mut self.rule_dispatcher,
                        &legacy_result,
                        Some(&handball_contact),
                        &foul_pos,
                        last_touch_team,
                        self.rng.gen(),
                    );
                    // Note: In DispatcherPrimary mode, the decision would be used.
                    // However, handball detection is already done by ActionQueue,
                    // so we mainly track statistics here for bias monitoring.
                }

                self.emit_event(
                    MatchEvent::foul(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *offender_idx,
                        (foul_pos_m.0, foul_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(*last_touch_idx),
                );
                if is_home {
                    self.result.statistics.fouls_home += 1;
                    self.result.statistics.handball_fouls_home += 1;
                } else {
                    self.result.statistics.fouls_away += 1;
                    self.result.statistics.handball_fouls_away += 1;
                }

                if restart_type == RestartType::Penalty {
                    let receiving_is_home = match self.action_queue.ball_state() {
                        BallState::OutOfPlay { home_team, .. } => *home_team,
                        _ => !is_home,
                    };
                    if receiving_is_home {
                        self.result.statistics.handball_penalties_home += 1;
                    } else {
                        self.result.statistics.handball_penalties_away += 1;
                    }
                }

                let at_m = MeterPos { x: foul_pos_m.0 as f64, y: foul_pos_m.1 as f64 };
                let receiving_team_id = if is_home { 1u32 } else { 0u32 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_foul(t_seconds, team_id, *offender_idx as u32, at_m);
                    match restart_type {
                        RestartType::Penalty => {
                            let penalty_spot = if receiving_team_id == 0 {
                                // Home team receives → away goal
                                MeterPos { x: 94.0, y: field::CENTER_Y as f64 }
                            } else {
                                // Away team receives → home goal
                                MeterPos { x: 11.0, y: field::CENTER_Y as f64 }
                            };
                            recorder.record_penalty(t_seconds + 0.1, receiving_team_id, penalty_spot, false);
                        }
                        RestartType::FreeKick => {
                            recorder.record_free_kick(t_seconds + 0.1, receiving_team_id, at_m);
                        }
                        _ => {}
                    }
                }

                if let BallState::OutOfPlay { restart_type, position, home_team } =
                    self.action_queue.ball_state()
                {
                    self.apply_restart_with_replay(
                        *restart_type,
                        *position,
                        *home_team,
                        false, // no boundary recording for foul
                        t_seconds,
                    );
                }
            }

            ActionResult::TackleSuccess { tackler_idx, target_idx } => {
                self.finalize_pass_sequences();
                let is_home = TeamSide::is_home(*tackler_idx);
                let team_id = if is_home { 0 } else { 1 };
                // C6: Use tackler_idx directly as track_id
                let ball_pos_m = self.ball.position_meters();
                self.emit_event(
                    MatchEvent::tackle(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *tackler_idx,
                        (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(Some(*target_idx)),
                );

                // 통계 업데이트
                self.record_tackle_success(*tackler_idx);

                // P6: Record to ReplayRecorder
                let ball_pos_m2 = self.ball.position.to_meters();
                let at_m = MeterPos { x: ball_pos_m2.0 as f64, y: ball_pos_m2.1 as f64 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_tackle(t_seconds, team_id, *tackler_idx as u32, at_m, true);

                    // 0108: Record possession change (Tackle)
                    let prev_team_id = if TeamSide::is_home(*target_idx) { 0u32 } else { 1u32 };
                    recorder.record_possession(
                        t_seconds,
                        team_id,
                        *tackler_idx as u32,
                        at_m,
                        PossessionChangeType::Tackle,
                        Some(*target_idx as u32),
                        Some(prev_team_id),
                    );
                }
            }

            ActionResult::SaveMade { goalkeeper_idx, shooter_idx, xg, .. } => {
                self.finalize_pass_sequences();
                let is_home = TeamSide::is_home(*goalkeeper_idx);
                let team_id = if is_home { 0 } else { 1 };

                // Balance diagnostics: saved shot is still a shot attempt
                self.balance_diagnostics.record_shot(*shooter_idx);
                self.balance_diagnostics.record_shot_saved(*shooter_idx);

                // FIX_2601/0115b: 세이브도 슈팅 통계 + xG에 포함
                // shooter_idx의 팀 기준으로 통계 업데이트
                let shooter_is_home = TeamSide::is_home(*shooter_idx);
                if shooter_is_home {
                    self.result.statistics.shots_home += 1;
                    self.result.statistics.shots_on_target_home += 1;
                    self.result.statistics.xg_home += xg;
                } else {
                    self.result.statistics.shots_away += 1;
                    self.result.statistics.shots_on_target_away += 1;
                    self.result.statistics.xg_away += xg;
                }
                // NOTE: Shot budget tracking done in record_shot_attempt() only

                // C6: Use goalkeeper_idx directly as track_id
                self.emit_event(
                    MatchEvent::save(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        *goalkeeper_idx,
                    )
                    .with_target_track_id(Some(*shooter_idx)),
                );

                // P6: Record to ReplayRecorder
                let ball_pos_m = self.ball.position.to_meters();
                let at_m = MeterPos { x: ball_pos_m.0 as f64, y: ball_pos_m.1 as f64 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_save(t_seconds, team_id, *goalkeeper_idx as u32, at_m);

                    // 0108: Record possession change (GkCollect)
                    let prev_team_id = if TeamSide::is_home(*shooter_idx) { 0u32 } else { 1u32 };
                    recorder.record_possession(
                        t_seconds,
                        team_id,
                        *goalkeeper_idx as u32,
                        at_m,
                        PossessionChangeType::GkCollect,
                        Some(*shooter_idx as u32),
                        Some(prev_team_id),
                    );
                }

                // FIX_2601/0107: 세이브 후 공 상태 리셋 (자책골 방지)
                // 1. GK가 공 소유
                self.ball.current_owner = Some(*goalkeeper_idx);
                self.ball.previous_owner = Some(*shooter_idx);
                // 2. 공을 GK 위치로 이동 (골대에서 멀리)
                self.ball.position = self.player_positions[*goalkeeper_idx];
                // 3. 비행 상태 해제
                self.ball.is_in_flight = false;
                self.ball.velocity = crate::engine::types::coord10::Vel10::default();
                self.ball.height = 0;
                // 4. BallState도 Controlled로 변경
                self.action_queue.set_ball_state(BallState::Controlled { owner_idx: *goalkeeper_idx });
            }

            ActionResult::ShotMissed { shooter_idx, xg } => {
                self.finalize_pass_sequences();
                // Balance diagnostics: off-target shot
                self.balance_diagnostics.record_shot(*shooter_idx);
                self.balance_diagnostics.record_shot_off_target(*shooter_idx);

                // FIX_2601/0115b: 빗나간 슛도 슈팅 통계 + xG에 포함 (shots만, shots_on_target은 제외)
                let shooter_is_home = TeamSide::is_home(*shooter_idx);
                if shooter_is_home {
                    self.result.statistics.shots_home += 1;
                    self.result.statistics.xg_home += xg;
                } else {
                    self.result.statistics.shots_away += 1;
                    self.result.statistics.xg_away += xg;
                }
                // FIX_2601/0112: Record to calibration snapshot (missed = not on target, not goal)
                self.record_shot_for_calibration(*shooter_idx, false, false, *xg);
                // NOTE: Shot budget tracking done in record_shot_attempt() only
                if let BallState::OutOfPlay {
                    restart_type,
                    position,
                    home_team,
                } = self.action_queue.ball_state()
                {
                    let record_boundary = matches!(
                        restart_type,
                        RestartType::Corner | RestartType::GoalKick | RestartType::ThrowIn
                    );
                    self.apply_restart_with_replay(
                        *restart_type,
                        *position,
                        *home_team,
                        record_boundary,
                        t_seconds,
                    );
                }
                if std::env::var("OF_LOG_GOALKICK").is_ok() {
                    if let BallState::OutOfPlay {
                        restart_type: RestartType::GoalKick,
                        position,
                        home_team,
                    } = self.action_queue.ball_state()
                    {
                        let pos_m = position.to_meters();
                        eprintln!(
                            "[GOAL_KICK_LOG] shooter={} is_home={} restart_home={} pos=({:.1},{:.1})",
                            shooter_idx,
                            shooter_is_home,
                            home_team,
                            pos_m.0,
                            pos_m.1
                        );
                    }
                }
            }

            ActionResult::CarryComplete { player_idx, new_position } => {
                // Carry (운반) - 리플레이에는 기록하지만 통계에는 미기록
                self.record_carry_action(*player_idx);
                // 단순 공 운반이므로 드리블로 분류하지 않음
                let _ = (player_idx, new_position, t_seconds);
                // 필요시 리플레이 기록 가능 (현재는 미기록)
            }

            ActionResult::TakeOnComplete { player_idx, new_position, beaten_defender_idx } => {
                // Take-on (돌파) - 수비수를 제침. 드리블 통계에 기록
                self.record_take_on_success(*player_idx);
                self.record_dribble(*player_idx);
                // FIX_2601: Coord10.to_meters() already returns meters
                let ball_pos_m = self.ball.position.to_meters();
                let is_home = TeamSide::is_home(*player_idx);
                let team_id = if is_home { 0 } else { 1 };
                let from_m = MeterPos { x: ball_pos_m.0 as f64, y: ball_pos_m.1 as f64 };
                let new_pos_m = new_position.to_meters();
                let to_m = MeterPos { x: new_pos_m.0 as f64, y: new_pos_m.1 as f64 };

                // P6: Record dribble to ReplayRecorder (only Take-on, not Carry)
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_dribble(t_seconds, team_id, *player_idx as u32, from_m, to_m);
                }

                let _ = beaten_defender_idx; // 나중에 확장용
            }

            ActionResult::OutOfBounds { restart_type, position, home_team } => {
                self.finalize_pass_sequences();
                // home_team=true means the restart team is home.
                let record_boundary = matches!(
                    restart_type,
                    RestartType::Corner | RestartType::GoalKick | RestartType::ThrowIn
                );
                self.apply_restart_with_replay(
                    *restart_type,
                    *position,
                    *home_team,
                    record_boundary,
                    t_seconds,
                );

                // FIX_2601/0123: Transition to appropriate dead ball state
                let last_touch_team = if *home_team {
                    TeamId::AWAY // home_team=true means home restarts, so away touched last
                } else {
                    TeamId::HOME
                };
                let ball_pos_m = self.ball.position.to_meters();
                self.game_flow_machine.try_transition(
                    TransitionTrigger::OutOfPlay {
                        restart_type: *restart_type,
                        position: MatchPosition::new(
                            ball_pos_m.0 / 105.0,
                            ball_pos_m.1 / 68.0,
                        ),
                        last_touch_team,
                    },
                    self.current_tick,
                );
            }

            // FIX_2601/0102: 트랩 성공 시 어시스트 후보 업데이트
            ActionResult::TrapSuccess { player_idx } => {
                // 패스로 인한 트랩인 경우 어시스트 후보 저장
                let prev_owner_id = self.action_queue.last_passer_idx.take();
                let prev_target_id = self.action_queue.last_pass_receiver_idx.take();
                let mut pass_success_passer: Option<usize> = None;
                if let (Some(passer_idx), Some(target_idx)) = (prev_owner_id, prev_target_id) {
                    let same_team =
                        TeamSide::is_home(passer_idx) == TeamSide::is_home(*player_idx);
                    if same_team && *player_idx == target_idx {
                        pass_success_passer = Some(passer_idx);
                        self.record_pass_success(passer_idx);
                        // FIX_2601/1128: Record pass pair for reciprocity bonus
                        self.record_pass_pair(passer_idx, target_idx);
                        let receiver_team = TeamSide::from_player_idx(*player_idx);
                        self.assist_candidate = Some(super::super::types::AssistCandidate::new(
                            passer_idx,
                            *player_idx,
                            receiver_team,
                            self.current_tick as u32,
                        ));
                    }
                }

                // 0108: Record possession change (PassReceive)
                let is_home = TeamSide::is_home(*player_idx);
                let team_id = if is_home { 0 } else { 1 };
                let ball_pos_m = self.ball.position.to_meters();
                let at_m = MeterPos { x: ball_pos_m.0 as f64, y: ball_pos_m.1 as f64 };
                if let Some(prev_owner_id) = pass_success_passer {
                    if let Some(ref mut recorder) = self.replay_recorder {
                        let prev_team_id =
                            Some(if TeamSide::is_home(prev_owner_id) { 0u32 } else { 1u32 });
                        recorder.record_possession(
                            t_seconds,
                            team_id,
                            *player_idx as u32,
                            at_m,
                            PossessionChangeType::PassReceive,
                            Some(prev_owner_id as u32),
                            prev_team_id,
                        );
                    }
                }
            }

            // 0108: Interception - possession change but no MatchEvent
            ActionResult::InterceptSuccess { player_idx } => {
                self.finalize_pass_sequences();
                let is_home = TeamSide::is_home(*player_idx);
                let team_id = if is_home { 0 } else { 1 };
                let ball_pos_m = self.ball.position.to_meters();
                let at_m = MeterPos { x: ball_pos_m.0 as f64, y: ball_pos_m.1 as f64 };
                self.balance_diagnostics.record_interception(*player_idx);
                // Interceptor gains from opponent team
                let prev_team_id = if is_home { 1u32 } else { 0u32 };
                if let Some(ref mut recorder) = self.replay_recorder {
                    recorder.record_possession(
                        t_seconds,
                        team_id,
                        *player_idx as u32,
                        at_m,
                        PossessionChangeType::Interception,
                        None, // Unknown previous owner in current impl
                        Some(prev_team_id),
                    );
                }
            }

            // 내부 상태 변경만 (이벤트 미생성, 리플레이에도 기록 안함)
            ActionResult::TrapFailed { .. } => {
                self.finalize_pass_sequences();
            }
            ActionResult::DribbleTackled { .. } => {
                self.finalize_pass_sequences();
            }
            ActionResult::MoveComplete { .. }
            | ActionResult::HeaderWon { .. }
            | ActionResult::Cancelled { .. }
            => {}
        }
    }

    fn execute_drop_ball(&mut self, position: Coord10, restart_is_home: bool) {
        let (start_idx, end_idx) = if restart_is_home { (0, 11) } else { (11, 22) };
        let pos_m = position.to_meters();
        let mut nearest_idx = start_idx;
        let mut nearest_dist = f32::MAX;

        for idx in start_idx..end_idx {
            let player_pos = self.player_positions[idx].to_meters();
            let dist =
                ((player_pos.0 - pos_m.0).powi(2) + (player_pos.1 - pos_m.1).powi(2)).sqrt();
            if dist < nearest_dist {
                nearest_dist = dist;
                nearest_idx = idx;
            }
        }

        self.ball.position = position;
        self.ball.velocity = Vel10::default();
        self.ball.height = 0;
        self.ball.is_in_flight = false;
        self.ball.current_owner = Some(nearest_idx);
    }

    fn apply_restart_with_replay(
        &mut self,
        restart_type: RestartType,
        position: Coord10,
        restart_is_home: bool,
        record_boundary: bool,
        t_seconds: f64,
    ) {
        // Restart pulse (set piece will follow)
        self.restart_occurred_this_tick = true;
        self.restart_type_this_tick = Some(restart_type);

        let pos_m = position.to_meters();
        let pos_norm = position.to_normalized_legacy();

        let free_kick_is_indirect =
            restart_type == RestartType::FreeKick && self.pending_indirect_free_kick;
        self.pending_indirect_free_kick = false;

        match restart_type {
            RestartType::Corner => {
                self.start_corner_kick_fsm(restart_is_home);
            }
            RestartType::FreeKick => {
                self.start_free_kick_fsm(pos_norm, restart_is_home, free_kick_is_indirect);
            }
            RestartType::Penalty => {
                self.start_penalty_kick_fsm(restart_is_home);
            }
            RestartType::GoalKick => {
                let gk_idx = if restart_is_home { 0 } else { 11 };
                self.execute_goal_kick(gk_idx, restart_is_home);
            }
            RestartType::ThrowIn => {
                let touch_line_pos = (pos_m.0 / field::LENGTH_M, pos_m.1 / field::WIDTH_M);
                self.execute_throw_in(touch_line_pos, restart_is_home);
            }
            RestartType::KickOff => {}
            RestartType::DropBall => {
                self.execute_drop_ball(position, restart_is_home);
            }
        }

        // P6: Record boundary + set piece to ReplayRecorder
        let at_m = MeterPos { x: pos_m.0 as f64, y: pos_m.1 as f64 };
        let receiving_team_id = if restart_is_home { 0u32 } else { 1u32 };

        if let Some(ref mut recorder) = self.replay_recorder {
            if record_boundary {
                recorder.record_boundary(t_seconds, at_m);
            }

            // 세트피스 종류에 따라 추가 이벤트 기록
            match restart_type {
                RestartType::Corner => {
                    recorder.record_corner_kick(t_seconds, receiving_team_id, at_m);
                }
                RestartType::FreeKick => {
                    recorder.record_free_kick(t_seconds, receiving_team_id, at_m);
                }
                RestartType::Penalty => {
                    // 페널티는 골대 앞 스팟에서
                    let penalty_spot = if receiving_team_id == 0 {
                        // 홈팀이 받음 → 어웨이 골대 앞 (오른쪽)
                        crate::replay::types::MeterPos { x: 94.0, y: field::CENTER_Y as f64 }
                    } else {
                        // 어웨이팀이 받음 → 홈 골대 앞 (왼쪽)
                        crate::replay::types::MeterPos { x: 11.0, y: field::CENTER_Y as f64 }
                    };
                    recorder.record_penalty(
                        t_seconds,
                        receiving_team_id,
                        penalty_spot,
                        false,
                    );
                }
                RestartType::GoalKick | RestartType::ThrowIn | RestartType::KickOff | RestartType::DropBall => {}
            }
        }
    }

    /// Single SSOT for processing ActionResult
    ///
    /// FIX_2601/0123: Offside check integrated with RuleDispatcher based on rule_check_mode.
    pub(crate) fn handle_action_result(&mut self, result: ActionResult) {
        let mut offside_called = false;
        if let ActionResult::PassStarted { passer_idx, receiver_idx, .. } = &result {
            let is_home = TeamSide::is_home(*passer_idx);
            let passer_pos = self.get_player_position_by_index(*passer_idx);
            let receiver_pos = self.get_player_position_by_index(*receiver_idx);

            // Check offside using legacy method
            let legacy_offside = self.is_offside_pass(*passer_idx, *receiver_idx, is_home);

            // Create PassEvent for dispatcher comparison/decision
            let pass_event = PassEvent {
                passer_idx: *passer_idx,
                receiver_idx: *receiver_idx,
                origin: passer_pos.clone(),
                target: receiver_pos.clone(),
                attacking_team: if is_home { RuleTeamId::Home } else { RuleTeamId::Away },
            };

            // Determine final offside decision based on mode
            let is_offside = if self.rule_check_mode.dispatcher_applies() {
                // In DispatcherPrimary mode, use dispatcher decision
                let last_touch_team = if let Some(idx) = self.ball.current_owner {
                    RuleTeamId::from_player_index(idx)
                } else {
                    RuleTeamId::Home
                };
                let decisions = self.rule_dispatcher.evaluate_tick(
                    &self.ball.position,
                    None,
                    last_touch_team,
                    None,
                    None,
                    Some(&pass_event),
                    legacy_offside, // Pass the legacy detection to dispatcher
                    &[],
                    self.rng.gen(),
                );
                decisions.iter().any(|d| matches!(d, RuleDecision::Offside { .. }))
            } else {
                // In StatisticsOnly or LegacyWithTracking mode, use legacy decision
                if self.rule_check_mode.tracking_enabled() && legacy_offside {
                    // Track comparison
                    let legacy_result = LegacyOffsideResult {
                        is_offside: true,
                        player_idx: Some(*receiver_idx),
                        position: Some(receiver_pos.clone()),
                        pass_origin: Some(passer_pos.clone()),
                    };
                    let last_touch_team = if let Some(idx) = self.ball.current_owner {
                        RuleTeamId::from_player_index(idx)
                    } else {
                        RuleTeamId::Home
                    };
                    let _decision = check_offside_wrapper(
                        self.rule_check_mode,
                        &mut self.rule_dispatcher,
                        &legacy_result,
                        Some(&pass_event),
                        &self.ball.position,
                        last_touch_team,
                        self.rng.gen(),
                    );
                }
                legacy_offside
            };

            if is_offside {
                self.emit_event(MatchEvent::offside(
                    self.minute,
                    self.current_timestamp_ms(),
                    is_home,
                    *receiver_idx,
                ));
                // FIX_2601/0112: Statistics updated via events in stats.update_from_events()
                // Only update internal counters here
                if is_home {
                    self.offside_count_home += 1;
                } else {
                    self.offside_count_away += 1;
                }
                self.apply_offside_restart(is_home, receiver_pos);
                offside_called = true;
            }
        }

        let is_out_of_bounds = matches!(result, ActionResult::OutOfBounds { .. });
        self.emit_action_result(&result);
        self.record_action_result_xp(&result);
        self.action_queue.record_result(result);
        if offside_called {
            self.action_queue.last_passer_idx = None;
            self.action_queue.last_pass_receiver_idx = None;
        }
        if is_out_of_bounds {
            self.action_queue
                .sync_from_ball(self.current_tick, &self.ball);
        }
    }

    // ========== Phase 0 Diagnostics (Minimal Logs) ==========

    fn record_phase0_tick(&mut self) {
        let (ball_x_m, _) = self.ball.position.to_meters();
        let owner_idx = self.ball.current_owner;
        // FIX_2601: Use proper attack direction (accounts for halftime)
        let home_attacks_right = self.attacks_right(true);
        self.balance_diagnostics.record_tick(
            ball_x_m,
            owner_idx,
            home_attacks_right,
            self.minute,
            &PHASE0_PRESSING_INTENSITY,
        );
    }

    /// FIX_2601/1128: intended_target_pos 파라미터 추가
    /// 선택 시점의 타겟 위치를 사용하여 forward_pass_rate를 정확하게 측정
    fn record_phase0_pass(
        &mut self,
        passer_idx: usize,
        receiver_idx: usize,
        intended_target_pos: Option<Coord10>,
    ) {
        let is_home = TeamSide::is_home(passer_idx);
        // FIX_2601/0110: Use attacks_right for correct 2nd half direction
        let ctx = team_view_context(self, is_home);
        let attacks_right = ctx.attacks_right;
        let passer_pos = self.get_player_position_by_index(passer_idx);

        // FIX_2601/1128: 선택 시점 위치(intended_target_pos) 사용, 없으면 현재 위치
        let current_receiver_pos = self.get_player_position_by_index(receiver_idx);
        let receiver_pos = intended_target_pos.unwrap_or(current_receiver_pos);

        let pass_distance_m = passer_pos.distance_to_m(&receiver_pos);
        let progress_m = Self::pass_progress_m(passer_pos.x, receiver_pos.x, attacks_right);
        let is_forward = progress_m > 0.0;
        let max_forward_option_m = self.max_forward_option_m(passer_idx, attacks_right, passer_pos.x);

        self.balance_diagnostics.record_pass(
            passer_idx,
            is_forward,
            progress_m,
            pass_distance_m,
            max_forward_option_m,
        );
    }

    /// FIX_2601/0110: Changed is_home to attacks_right for correct 2nd half direction
    fn max_forward_option_m(&self, passer_idx: usize, attacks_right: bool, passer_x: i32) -> f32 {
        let is_home = TeamSide::is_home(passer_idx);
        let mut max_forward = 0.0;
        for target_idx in self.find_valid_pass_targets(passer_idx, is_home) {
            let target_pos = self.get_player_position_by_index(target_idx);
            let progress_m = Self::pass_progress_m(passer_x, target_pos.x, attacks_right);
            if progress_m > max_forward {
                max_forward = progress_m;
            }
        }
        max_forward
    }

    /// FIX_2601/0110: Changed is_home to attacks_right for correct 2nd half direction
    fn pass_progress_m(from_x: i32, to_x: i32, attacks_right: bool) -> f32 {
        let from_tv_x = TeamViewCoord10::from_world(Coord10 { x: from_x, y: 0, z: 0 }, attacks_right).x;
        let to_tv_x = TeamViewCoord10::from_world(Coord10 { x: to_x, y: 0, z: 0 }, attacks_right).x;
        (to_tv_x - from_tv_x) as f32 / Coord10::SCALE
    }

    // C6.3: Removed get_player_name_by_idx() - no longer needed for Event SSOT
    // Context structs use get_player_name() from player_attributes.rs instead

    // ========== Tick-Based Simulation ==========

    /// Execute exactly one decision tick (250ms) of the tick-based simulation.
    ///
    /// This is the shared tick body used by:
    /// - batch/minute simulation (`simulate_minute_tick_based`)
    /// - session/streaming per-tick stepping (Phase23.5)
    ///
    /// Assumptions:
    /// - `self.current_tick` is already set to the absolute tick index
    /// - `self.minute` is already set to the current minute (0-based)
    fn simulate_decision_tick(
        &mut self,
        home_strength: f32,
        away_strength: f32,
        possession_ratio: f32,
    ) {
        // Debug: check if this function is called
        static DEBUG_SDT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if DEBUG_SDT.load(std::sync::atomic::Ordering::Relaxed) < 3 {
            DEBUG_SDT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            #[cfg(debug_assertions)]
            if match_debug_enabled() {
                println!(
                    "[DEBUG-0105-SDT] simulate_decision_tick called, tick={}",
                    self.current_tick
                );
            }
        }

        // 0. 틱 시작 시 골 플래그 리셋 (중복 골 방지)
        self.goal_scored_this_tick = false;

        // FIX_2601/0120: RNG consumption tracking - increment tick count
        self.rng_tracker.tick_count += 1;

        // Phase 1.3 MarkingManager inputs (tick-scoped pulses)
        self.possession_changed_this_tick = false;
        self.restart_occurred_this_tick = false;
        self.restart_type_this_tick = None;
        self.pending_indirect_free_kick = false;
        self.decision_intents.clear();

        // FIX_2601/0123: Game Flow State Machine updates
        self.update_game_flow_state();

        // FIX_2601/0123: Centralized rule evaluation
        self.evaluate_rules();

        // Phase 2: AI 전술 업데이트 (점수 변화 또는 시간 경과 시)
        self.update_ai_tactics_if_needed();

        // P0: Update defensive tuning from TeamInstructions
        let mut home_mindset = crate::engine::mindset::MindsetContext::default();
        let mut away_mindset = crate::engine::mindset::MindsetContext::default();

        super::decision_topology::apply_team_instructions(
            Some(&self.home_instructions),
            &mut home_mindset,
            &mut self.home_defensive_tuning,
        );
        super::decision_topology::apply_team_instructions(
            Some(&self.away_instructions),
            &mut away_mindset,
            &mut self.away_defensive_tuning,
        );

        // FIX_2601/0109: Apply sparse deck/coach match modifiers at the last scalar layer.
        // Keep this additive and clamped so it can't break tactical ranges.
        self.home_defensive_tuning.pressing_factor = (self.home_defensive_tuning.pressing_factor
            + self.home_match_modifiers.press_intensity_add)
            .clamp(0.2, 1.0);
        self.away_defensive_tuning.pressing_factor = (self.away_defensive_tuning.pressing_factor
            + self.away_match_modifiers.press_intensity_add)
            .clamp(0.2, 1.0);

        // P0-W1: Update elastic tactics from TeamInstructions
        self.elastic_home_tactics =
            crate::engine::elastic_band::team_instructions_to_elastic_tactics(  
                &self.home_instructions,
            );
        self.elastic_away_tactics =
            crate::engine::elastic_band::team_instructions_to_elastic_tactics(
                &self.away_instructions,
            );

        // 1. TeamPhase 업데이트
        self.update_team_phases();

        // 1.5 FIX_2601/1129: AttackPhase 업데이트
        self.update_attack_phases();

        // 2. P7: PlayerState 틱 업데이트 (Recovering, Cooldown 감소)
        self.update_player_states_tick();

        // 3. PlayerObjective 할당
        self.assign_player_objectives_tick();

        // 4. 공 상태 동기화 (Ball → ActionQueue)
        self.action_queue
            .sync_from_ball(self.current_tick, &self.ball);

        // 4.5. Woodwork collision response (pre-action)
        // If the in-flight segment hits the post/crossbar at this tick, cancel any
        // ball-dependent actions (Trap/Save/etc) before we execute them.
        self.action_queue.resolve_in_flight_woodwork_pre_actions(
            self.current_tick,
            crate::engine::ball_physics_params::DEFAULT,
        );

        // 5-6. P7: Phase FSM 기반 액션 실행 (레거시 모드 제거 2025-12-12)
        self.execute_phase_tick();

        // 7. 새 액션 생성 (공 소유자가 있고 예약된 액션이 없으면)
        if let Some(owner_idx) = self.ball.current_owner {
            // FIX_2601/0112: Record ball touch for calibration zone distribution
            self.record_ball_touch(owner_idx);

            let owner_has_action = self.action_queue.is_player_active(owner_idx)
                || self.action_queue.has_pending_for_player(owner_idx);
            if owner_has_action {
                self.result.statistics.owner_action_blocked_ticks =
                    self.result.statistics.owner_action_blocked_ticks.saturating_add(1);
            } else {
                let exp = self.get_exp_params();
                if exp.dpq_enabled {
                    if self.decision_scheduler.is_due(owner_idx, self.current_tick) {
                        self.generate_initial_action_tick(
                            home_strength,
                            away_strength,
                            possession_ratio,
                        );
                        // DPQ v1.2: Use variable cadence if enabled
                        if exp.dpq_variable_cadence {
                            use crate::engine::decision_scheduler::calculate_cadence_level;
                            let player_pos_m = self.player_positions[owner_idx].to_meters();
                            let ball_pos_m = self.ball.position.to_meters();
                            let level = calculate_cadence_level(
                                owner_idx,
                                player_pos_m,
                                ball_pos_m,
                                self.ball.current_owner,
                                self.last_pass_target,
                            );
                            self.decision_scheduler
                                .mark_executed_v1_2(owner_idx, self.current_tick, level);
                        } else {
                            self.decision_scheduler
                                .mark_executed_v1_1(owner_idx, self.current_tick);
                        }
                    } else {
                        self.result.statistics.decisions_skipped =
                            self.result.statistics.decisions_skipped.saturating_add(1);
                    }
                } else {
                    self.generate_initial_action_tick(home_strength, away_strength, possession_ratio);
                }
            }
        }

        // 7.8. P9: 활성 세트피스 FSM 틱 업데이트
        self.update_active_set_pieces();

        // 8. P7: 수비수 태클 결정 (FSM 기반)
        self.decide_defender_tackles();

        // 8.5. 쿨다운 틱 감소
        for cooldown in self.tackle_cooldowns.iter_mut() {
            *cooldown = cooldown.saturating_sub(1);
        }

        // 8.9. 공 상태 진행 (InFlight arrival → Loose, post-action)
        self.action_queue
            .advance_ball_state_post_actions(self.current_tick);

        // 8.95. 루즈볼 물리 진행 (position integration + roll damping)
        self.action_queue.advance_loose_ball_physics(
            self.current_tick,
            crate::engine::ball_physics_params::DEFAULT,
        );

        // 9. 공 상태 역동기화 (ActionQueue → Ball)
        self.action_queue.sync_to_ball(&mut self.ball);

        // 9.5. 공 위치를 소유자 위치로 동기화 (Controlled 상태일 때)
        if let Some(owner_idx) = self.ball.current_owner {
            if owner_idx < self.player_positions.len() {
                // player_positions is now Vec<Coord10>, use directly
                self.ball.position = self.player_positions[owner_idx];
            }
        }

        if let Some(out_of_play) = self.detect_out_of_play_action() {
            self.handle_action_result(out_of_play);
        }

        self.update_possession_clock();
        self.record_tick_telemetry();

        // 9.8. FIX_2601/0115: Off-ball decision system update
        // Updates offball_objectives for off-ball players before positioning_engine consumes them.
        // Only runs when offball_decisions_enabled is true in ExpConfig.
        self.update_offball_decisions_tick();

        // 10. Off-the-Ball 포지셔닝 업데이트
        self.update_positioning_tick();

        // 10.5. PepGrid 과밀화 해결
        self.resolve_overcrowding_tick();

        // 11. P7: 수비 포지셔닝 업데이트
        self.update_defensive_positioning_tick();

        // 11.5. 루즈볼 체크
        self.check_loose_ball_tick();

        // 11.6. P0: 공 위치 기반 골 체크 (Goal Contract)
        self.check_goals_from_ball_position();

        // 11.7. P18: FieldBoard 업데이트 (occupancy 매틱, pressure 3틱마다)
        self.update_field_board_tick();

        // 11.8. FIX_2512 Phase 0: Audit Gates - Validate coordinates
        #[cfg(debug_assertions)]
        {
            super::super::audit_gates::validate_ball_coordinates(&self.ball);
            super::super::audit_gates::validate_player_coordinates(&self.player_positions);
        }

        // Phase 0: Minimal diagnostics (ball position + possession)
        self.record_phase0_tick();

        // 12. 위치 기록 (엔진이 계산한 실제 위치를 기록)
        self.record_positions_for_tick();

        // 13. P10-13: Stamina 업데이트
        self.update_all_sprint_states();
        self.decay_stamina_tick();

        // 14. FIX_2601/0123: Momentum tick (gradual decay toward neutral)
        self.home_momentum.tick();
        self.away_momentum.tick();
    }

    /// 새로운 틱 기반 시뮬레이션
    ///
    /// P7 Integration + P10-13 Stamina: 13단계 실행 순서
    /// 1. TeamPhase 업데이트
    /// 2. PlayerState 틱 업데이트 (Recovering, Cooldown 감소)
    /// 3. PlayerObjective 할당
    /// 4. 공 상태 동기화 (Ball → ActionQueue)
    /// 5. ActionQueue에서 실행할 액션 가져오기
    /// 6. 액션 실행
    /// 7. 새 액션 생성
    /// 8. 수비수 태클 체크
    /// 9. 공 상태 역동기화 (ActionQueue → Ball)
    /// 10. Off-the-Ball 포지셔닝 업데이트
    /// 11. 수비 포지셔닝 업데이트 (P7)
    /// 12. 위치 기록
    /// 13. Stamina 업데이트 (P10-13)
    pub(crate) fn simulate_minute_tick_based(
        &mut self,
        home_strength: f32,
        away_strength: f32,
        possession_ratio: f32,
    ) {
        for tick_offset in 0..TICKS_PER_MINUTE {
            self.current_tick = self.minute as u64 * TICKS_PER_MINUTE + tick_offset;
            self.simulate_decision_tick(home_strength, away_strength, possession_ratio);
        }

        // 기타 이벤트 (카드, 부상 등) - 분당 1회
        self.simulate_other_events();

        // 유저 플레이어 이벤트
        if self.user_player.is_some() {
            let home_has_ball = self.ball.current_owner.map(TeamSide::is_home).unwrap_or(false);
            self.generate_user_player_involvement(home_has_ball);
        }
    }

    /// Phase 23.5: Execute exactly one decision tick (250ms) for session/streaming.
    ///
    /// This uses the **same** tick body as the batch/tick-based simulation, so session
    /// streaming is Game OS-compliant (Threat/Transition/Marking ordering, dual timestep, etc).
    ///
    /// Returns:
    /// - `true` if the match should continue after this tick
    /// - `false` if the match is finished (next tick would exceed `match_duration`)
    pub fn step_decision_tick_streaming(
        &mut self,
        home_strength: f32,
        away_strength: f32,
        possession_ratio: f32,
        match_duration: u8,
    ) -> bool {
        // Derive minute from absolute tick index to keep invariants stable.
        let minute = (self.current_tick / TICKS_PER_MINUTE) as u8;
        self.minute = minute;
        let tick_within_minute = self.current_tick % TICKS_PER_MINUTE;

        // 1H added time is finalized at minute 45 (before simulating minute 45+).
        if !self.is_second_half && self.minute >= super::HALF_DURATION_MINUTES {
            self.maybe_finalize_first_half_stoppage_time();
        }

        // Half-time boundary (second half kickoff).
        if !self.is_second_half && tick_within_minute == 0 && self.minute == self.first_half_end_minute {
            self.handle_half_time();
        }

        // Decide 2H added time at regulation end minute.
        if self.is_second_half && self.minute >= self.regulation_end_minute() {
            self.maybe_finalize_second_half_stoppage_time();
        }
        let match_end_minute = self.match_end_minute.min(match_duration);

        // FIX_2601/0106 P4: Update match situation each tick
        self.match_situation.update_minute(self.minute as u32);
        self.match_situation.update_score(self.result.score_home, self.result.score_away);

        // Stop condition (match duration is in minutes, consistent with step()/simulate()).
        if self.minute > match_end_minute {
            return false;
        }

        // Cache "current tick time" for external accessors (session HUD/debug).
        let ms_per_tick = 60_000 / TICKS_PER_MINUTE;
        self.current_timestamp_ms = self.minute as u64 * 60_000 + tick_within_minute * ms_per_tick;

        // Run the shared tick body.
        self.simulate_decision_tick(home_strength, away_strength, possession_ratio);

        // End-of-minute hooks (once per minute).
        if tick_within_minute == TICKS_PER_MINUTE - 1 {
            self.simulate_other_events();

            if self.user_player.is_some() {
                let home_has_ball = self.ball.current_owner.map(TeamSide::is_home).unwrap_or(false);
                self.generate_user_player_involvement(home_has_ball);
            }
        }

        // Advance to the next tick.
        self.current_tick = self.current_tick.saturating_add(1);

        // Determine whether the next tick is still within match duration.      
        let next_minute = (self.current_tick / TICKS_PER_MINUTE) as u8;
        next_minute <= match_end_minute
    }

    // ========== Helper Methods ==========

    /// Detect if ball is out of play and determine restart type.
    ///
    /// FIX_2601/0123: When in DispatcherPrimary mode, out of play detection
    /// is handled through evaluate_rules() instead, so this function skips.
    fn detect_out_of_play_action(&mut self) -> Option<ActionResult> {
        // Skip when dispatcher is primary - out of play handled in evaluate_rules()
        if self.rule_check_mode.dispatcher_applies() {
            return None;
        }

        if !self.action_queue.ball_state().is_in_play() {
            return None;
        }

        let ball_pos_m = self.ball.position.to_meters();
        let out_of_bounds = coordinates::is_out_of_bounds_m(ball_pos_m);
        let ball_pos_norm = self.ball.position.to_normalized();
        let last_touch_home = self
            .ball
            .current_owner
            .or(self.ball.previous_owner)
            .map(TeamSide::is_home);
        let out_result = check_ball_out(ball_pos_norm, last_touch_home, self.home_ctx.attacks_right);

        if !out_of_bounds && matches!(out_result, OutOfBoundsResult::NotOut) {
            return None;
        }

        let result = match out_result {
            OutOfBoundsResult::Corner { attacking_home } => Some(ActionResult::OutOfBounds {
                restart_type: RestartType::Corner,
                position: self.ball.position.clamp_to_field(),
                home_team: attacking_home,
            }),
            OutOfBoundsResult::GoalKick { gk_idx, position } => {
                let restart_pos = Coord10::from_normalized(position);
                let home_team = gk_idx == 0;
                Some(ActionResult::OutOfBounds {
                    restart_type: RestartType::GoalKick,
                    position: restart_pos,
                    home_team,
                })
            }
            OutOfBoundsResult::ThrowIn { throwing_home, position } => {
                let restart_pos = Coord10::from_normalized(position);
                Some(ActionResult::OutOfBounds {
                    restart_type: RestartType::ThrowIn,
                    position: restart_pos,
                    home_team: throwing_home,
                })
            }
            OutOfBoundsResult::NotOut => None,
        };

        // A/B comparison tracking for LegacyWithTracking mode
        if self.rule_check_mode.tracking_enabled() && result.is_some() {
            if let Some(ActionResult::OutOfBounds { restart_type, home_team, .. }) = &result {
                let last_touch_team = if let Some(idx) = self.ball.current_owner.or(self.ball.previous_owner) {
                    RuleTeamId::from_player_index(idx)
                } else {
                    RuleTeamId::Home
                };
                let rule_restart = match restart_type {
                    RestartType::ThrowIn => Some(RuleRestartType::ThrowIn),
                    RestartType::GoalKick => Some(RuleRestartType::GoalKick),
                    RestartType::Corner => Some(RuleRestartType::CornerKick),
                    _ => None,
                };
                let legacy_result = LegacyOutOfPlayResult {
                    is_out: true,
                    last_touch_team: if *home_team { last_touch_team.opponent() } else { last_touch_team },
                    restart_type: rule_restart,
                    position: self.ball.position.clone(),
                };
                // This call logs the comparison but doesn't change behavior
                let _decision = check_out_of_play_wrapper(
                    self.rule_check_mode,
                    &mut self.rule_dispatcher,
                    &legacy_result,
                    &self.ball.position,
                    last_touch_team,
                    self.rng.gen(),
                );
            }
        }

        result
    }

    /// Derive "which team has possession" for TeamPhase/Transition decisions.
    ///
    /// This must be stable across `BallState::InFlight` / `Loose` frames:
    /// - Prefer `current_owner` when controlled
    /// - Otherwise fall back to `previous_owner` (last controller)
    /// - If still unknown, keep the previous phase possession (no flicker)
    fn derive_home_has_ball_for_phases(
        ball: &super::super::Ball,
        prev_home_has_ball: bool,
    ) -> bool {
        ball.current_owner
            .or(ball.previous_owner)
            .map(super::super::TeamSide::is_home)
            .unwrap_or(prev_home_has_ball)
    }

    /// FIX_2601/0123: Update game flow state machine
    ///
    /// Handles automatic state transitions based on elapsed time and ball state:
    /// - GoalCelebration → KickoffReady after timeout
    /// - Restart states → InPlay when ball is kicked
    /// - KickoffReady → InPlay when kickoff executed
    fn update_game_flow_state(&mut self) {
        let current_state = self.game_flow_machine.current().clone();
        let ticks_in_state = self.game_flow_machine.ticks_in_state(self.current_tick);

        match &current_state {
            // GoalCelebration timeout → KickoffReady
            GameFlowState::GoalCelebration { .. } => {
                // 3 seconds = 720 ticks (at 240 ticks/min = 4 ticks/sec)
                if ticks_in_state >= 12 {
                    // ~3 sec at 4 ticks/sec
                    self.game_flow_machine.try_transition(
                        TransitionTrigger::TimeElapsed { ticks: ticks_in_state },
                        self.current_tick,
                    );
                }
            }

            // Restart states → InPlay when ball is in play
            GameFlowState::ThrowInSetup { .. }
            | GameFlowState::GoalKickSetup { .. }
            | GameFlowState::CornerSetup { .. }
            | GameFlowState::FreeKickSetup { .. }
            | GameFlowState::DeadBall { .. } => {
                // Check if ball is now in play (has owner and is not in set piece)
                if self.ball.current_owner.is_some()
                    && self.action_queue.ball_state().is_in_play()
                    && ticks_in_state >= 4
                {
                    // Min 1 second before resuming
                    self.game_flow_machine.try_transition(
                        TransitionTrigger::BallPlayed,
                        self.current_tick,
                    );
                }
            }

            // KickoffReady → InPlay when kickoff executed
            GameFlowState::KickoffReady { .. } => {
                // Transition to InPlay when ball starts moving from center
                if self.ball.current_owner.is_some()
                    && self.action_queue.ball_state().is_in_play()
                    && ticks_in_state >= 2
                {
                    self.game_flow_machine.try_transition(
                        TransitionTrigger::KickExecuted,
                        self.current_tick,
                    );
                }
            }

            // HalfTime timeout → KickoffReady (2nd half)
            GameFlowState::HalfTime => {
                // 15 minutes = 3600 ticks - but in simulation we skip this
                // Transition immediately when half time handling is done
                if ticks_in_state >= 1 {
                    self.game_flow_machine.try_transition(
                        TransitionTrigger::TimeElapsed { ticks: 3600 },
                        self.current_tick,
                    );
                }
            }

            // InPlay - normal gameplay, no automatic transitions
            GameFlowState::InPlay => {}

            // Other states - no automatic transitions
            _ => {}
        }
    }

    /// FIX_2601/0123: Evaluate rules using centralized RuleDispatcher
    ///
    /// This function uses the RuleDispatcher to check for rule violations
    /// and track team-neutral statistics. Based on rule_check_mode:
    /// - StatisticsOnly: Dispatcher evaluates but legacy code decides
    /// - LegacyWithTracking: A/B comparison mode, logs discrepancies
    /// - DispatcherPrimary: Dispatcher makes all decisions
    fn evaluate_rules(&mut self) {
        // Only evaluate when in play
        if !matches!(self.game_flow_machine.current(), GameFlowState::InPlay) {
            return;
        }

        // Set current tick for deterministic seeding
        self.rule_dispatcher.set_tick(self.current_tick);

        // Determine last touch team
        let last_touch_team = if let Some(owner_idx) = self.ball.current_owner {
            RuleTeamId::from_player_index(owner_idx)
        } else if let Some(prev_idx) = self.ball.previous_owner {
            RuleTeamId::from_player_index(prev_idx)
        } else {
            RuleTeamId::Home // Default to home
        };

        // Check for goals (ball position check)
        let ball_in_goal = self.check_ball_in_goal_for_rules();

        // Get scorer info if goal (use previous_owner as scorer)
        let (scorer_idx, assister_idx) = if ball_in_goal.is_some() {
            (self.ball.previous_owner, None) // Assister tracking not available in Ball
        } else {
            (None, None)
        };

        // Generate RNG roll for foul decisions
        let rng_roll: f32 = self.rng.gen();

        // Evaluate rules
        let decisions = self.rule_dispatcher.evaluate_tick(
            &self.ball.position,
            ball_in_goal,
            last_touch_team,
            scorer_idx,
            assister_idx,
            None,  // Pass events handled separately in is_offside_pass
            false, // Offside checked separately
            &[],   // Contacts handled by existing tackle system
            rng_roll,
        );

        // Log rule decisions for debugging
        if std::env::var("OF_DEBUG_RULES").is_ok() {
            for decision in &decisions {
                if !matches!(decision, RuleDecision::Continue) {
                    eprintln!(
                        "[RULES] tick={} mode={:?} decision={:?}",
                        self.current_tick, self.rule_check_mode, decision
                    );
                }
            }
        }

        // Handle rule decisions when in DispatcherPrimary mode
        if self.rule_check_mode.dispatcher_applies() {
            for decision in &decisions {
                match decision {
                    RuleDecision::Goal { scorer_idx, .. } => {
                        // Skip if goal already scored this tick
                        if self.goal_scored_this_tick {
                            continue;
                        }
                        // Determine scoring team from scorer index
                        let scoring_team_rule = RuleTeamId::from_player_index(*scorer_idx);
                        // Apply 2nd half direction reversal
                        let actual_team = if self.is_second_half {
                            scoring_team_rule.opponent()
                        } else {
                            scoring_team_rule
                        };
                        // Convert to TeamSide and call on_goal_scored
                        let team_side = if actual_team.is_home() {
                            super::super::TeamSide::Home
                        } else {
                            super::super::TeamSide::Away
                        };
                        self.on_goal_scored(team_side);
                    }
                    RuleDecision::OutOfPlay {
                        last_touch_team,
                        position,
                        restart_type,
                    } => {
                        // Convert RuleRestartType to RestartType
                        let restart = match restart_type {
                            RuleRestartType::ThrowIn => RestartType::ThrowIn,
                            RuleRestartType::GoalKick => RestartType::GoalKick,
                            RuleRestartType::CornerKick => RestartType::Corner,
                            _ => continue, // Skip other restart types
                        };
                        // Determine which team takes the restart (opponent of last touch)
                        let home_team = last_touch_team.opponent().is_home();
                        let action_result = ActionResult::OutOfBounds {
                            restart_type: restart,
                            position: position.clone(),
                            home_team,
                        };
                        self.handle_action_result(action_result);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Check if ball is in goal for rule evaluation
    /// Returns the scoring team if goal, None otherwise
    fn check_ball_in_goal_for_rules(&self) -> Option<RuleTeamId> {
        use crate::engine::physics_constants::{field, goal};

        let (ball_x, ball_y) = self.ball.position.to_meters();

        // Check if ball is within goal y-bounds (centered on field width)
        let in_goal_y = ball_y >= goal::Y_MIN && ball_y <= goal::Y_MAX;
        if !in_goal_y {
            return None;
        }

        // Check home goal (x < 0)
        if ball_x < 0.0 {
            return Some(RuleTeamId::Away); // Away team scores at home goal
        }

        // Check away goal (x > field length)
        if ball_x > field::LENGTH_M {
            return Some(RuleTeamId::Home); // Home team scores at away goal
        }

        None
    }

    fn update_team_phases(&mut self) {
        let prev_home_has_ball = self.home_phase_state.has_possession;
        let home_has_ball = Self::derive_home_has_ball_for_phases(&self.ball, prev_home_has_ball);

        self.possession_changed_this_tick = home_has_ball != prev_home_has_ball;
        if self.possession_changed_this_tick {
            self.record_possession_change(home_has_ball);
        }
        self.transition_system.update(self.possession_changed_this_tick, prev_home_has_ball);

        self.home_phase_state.update(home_has_ball, self.current_tick);
        self.away_phase_state.update(!home_has_ball, self.current_tick);

        // FIX_2601/1128: Update attack sub-phase for the team with possession
        self.update_attack_sub_phases(home_has_ball);
    }

    /// FIX_2601/1128: Update attack sub-phases based on game state
    fn update_attack_sub_phases(&mut self, home_has_ball: bool) {
        use crate::engine::team_phase::TeamPhase;

        let is_home = home_has_ball;
        let phase = if home_has_ball {
            self.home_phase_state.phase
        } else {
            self.away_phase_state.phase
        };

        if phase != TeamPhase::Attack {
            return;
        }

        // Calculate current pressure on ball carrier
        let ball_owner = self.ball.current_owner;
        let pressure = ball_owner
            .map(|idx| self.calculate_pressure_context(idx, None).effective_pressure)
            .unwrap_or(0.3);

        // Count forward pass options
        let forward_options = ball_owner
            .map(|idx| self.count_forward_pass_options(idx, is_home))
            .unwrap_or(5);

        // Distance to goal
        let dist_to_goal_m = ball_owner
            .map(|idx| {
                let pos = self.get_player_position_by_index(idx);
                let attacks_right = self.attacks_right(is_home);
                let x_m = pos.to_meters().0;
                if attacks_right {
                    field::LENGTH_M - x_m
                } else {
                    x_m
                }
            })
            .unwrap_or(50.0);

        // Update sub-phase (no forward pass result tracking for now)
        if home_has_ball {
            self.home_phase_state.update_attack_sub_phase(pressure, forward_options, dist_to_goal_m, None);
        } else {
            self.away_phase_state.update_attack_sub_phase(pressure, forward_options, dist_to_goal_m, None);
        }
    }

    /// FIX_2601/1128: Count forward pass options
    fn count_forward_pass_options(&self, passer_idx: usize, is_home: bool) -> usize {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let attacks_right = self.attacks_right(is_home);
        let passer_x = passer_pos.to_meters().0;

        let team_range = if is_home { 1..11 } else { 12..22 }; // Exclude GK
        let mut count = 0;

        for idx in team_range {
            if idx == passer_idx {
                continue;
            }
            let target_pos = self.get_player_position_by_index(idx);
            let target_x = target_pos.to_meters().0;

            let is_forward = if attacks_right {
                target_x > passer_x + 5.0 // At least 5m forward
            } else {
                target_x < passer_x - 5.0
            };

            if is_forward && !self.is_offside_position(idx, is_home) {
                count += 1;
            }
        }

        count
    }

    /// FIX_2601/1129: AttackPhase 업데이트 (Circulation/Positional/Transition)
    ///
    /// 팀 단위 공격 국면 상태를 업데이트한다.
    /// - 점유팀: phase 전이 로직 수행
    /// - 비점유팀: Circulation으로 리셋
    fn update_attack_phases(&mut self) {
        use super::attack_phase::{determine_phase, AttackPhase};

        let home_has_ball = self.home_phase_state.has_possession;

        // Debug: Track phase distribution
        static CIRC_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        static POS_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        static TRANS_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        // 점유팀의 attack state만 활성화
        if home_has_ball {
            // Away는 점유 상실
            self.away_attack_state.on_possession_lost(self.current_tick);

            // Home의 phase 전이 판단
            let ctx = self.build_phase_transition_context(true);
            let new_phase = determine_phase(&self.home_attack_state, &ctx);
            self.home_attack_state.transition_to(new_phase, self.current_tick);
            self.home_attack_state.tick();

            // Track phase distribution
            match self.home_attack_state.phase {
                AttackPhase::Circulation => { CIRC_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
                AttackPhase::Positional => { POS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
                AttackPhase::Transition => { TRANS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
            }
        } else {
            // Home은 점유 상실
            self.home_attack_state.on_possession_lost(self.current_tick);

            // Away의 phase 전이 판단
            let ctx = self.build_phase_transition_context(false);
            let new_phase = determine_phase(&self.away_attack_state, &ctx);
            self.away_attack_state.transition_to(new_phase, self.current_tick);
            self.away_attack_state.tick();

            // Track phase distribution
            match self.away_attack_state.phase {
                AttackPhase::Circulation => { CIRC_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
                AttackPhase::Positional => { POS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
                AttackPhase::Transition => { TRANS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
            }
        }

        // Debug output every 10000 ticks
        let total = CIRC_COUNT.load(std::sync::atomic::Ordering::Relaxed)
            + POS_COUNT.load(std::sync::atomic::Ordering::Relaxed)
            + TRANS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        if total > 0 && total % 10000 == 0 {
            let circ = CIRC_COUNT.load(std::sync::atomic::Ordering::Relaxed);
            let pos = POS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
            let trans = TRANS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
            eprintln!(
                "[ATTACK_PHASE] Total: {} | Circulation: {} ({:.1}%) | Positional: {} ({:.1}%) | Transition: {} ({:.1}%)",
                total,
                circ, 100.0 * circ as f64 / total as f64,
                pos, 100.0 * pos as f64 / total as f64,
                trans, 100.0 * trans as f64 / total as f64
            );
        }
    }

    /// FIX_2601/1129: PhaseTransitionContext 생성
    fn build_phase_transition_context(&self, is_home: bool) -> super::attack_phase::PhaseTransitionContext {
        let ball_owner = self.ball.current_owner;

        // 전방 패스 옵션 수
        let forward_options = ball_owner
            .map(|idx| self.count_forward_pass_options(idx, is_home))
            .unwrap_or(0);

        // 팀 라인 길이 (수비-공격 간격)
        let team_length_m = self.calculate_team_length(is_home);

        // 볼 캐리어 압박 수준
        let local_pressure = ball_owner
            .map(|idx| self.calculate_pressure_context(idx, None).effective_pressure)
            .unwrap_or(0.5);

        // 전방 공격수/수비수 수
        let (attackers_ahead, defenders_ahead) = ball_owner
            .map(|idx| self.count_players_ahead(idx, is_home))
            .unwrap_or((0, 0));

        // 전방 공간 열림 여부
        let forward_space_open = ball_owner
            .map(|idx| self.is_forward_space_open(idx, is_home))
            .unwrap_or(false);

        super::attack_phase::PhaseTransitionContext {
            forward_options,
            team_length_m,
            local_pressure,
            attackers_ahead,
            defenders_ahead,
            forward_space_open,
            current_tick: self.current_tick,
        }
    }

    /// FIX_2601/1129: 팀 라인 길이 계산 (최전방 - 최후방 거리)
    fn calculate_team_length(&self, is_home: bool) -> f32 {
        let team_range = if is_home { 0..11 } else { 11..22 };
        let attacks_right = self.attacks_right(is_home);

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;

        for idx in team_range {
            let pos = self.get_player_position_by_index(idx);
            let x = pos.to_meters().0;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }

        max_x - min_x
    }

    /// FIX_2601/1129: 전방 공격수/수비수 수 카운트
    fn count_players_ahead(&self, passer_idx: usize, is_home: bool) -> (usize, usize) {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let passer_x = passer_pos.to_meters().0;
        let attacks_right = self.attacks_right(is_home);

        let teammate_range = if is_home { 1..11 } else { 12..22 };
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        let is_ahead = |x: f32| {
            if attacks_right { x > passer_x + 3.0 } else { x < passer_x - 3.0 }
        };

        let attackers_ahead = teammate_range
            .filter(|&idx| idx != passer_idx)
            .filter(|&idx| is_ahead(self.get_player_position_by_index(idx).to_meters().0))
            .count();

        let defenders_ahead = opponent_range
            .filter(|&idx| is_ahead(self.get_player_position_by_index(idx).to_meters().0))
            .count();

        (attackers_ahead, defenders_ahead)
    }

    /// FIX_2601/1129: 전방 공간 열림 여부 (간단한 휴리스틱)
    fn is_forward_space_open(&self, passer_idx: usize, is_home: bool) -> bool {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let (passer_x, passer_y) = passer_pos.to_meters();
        let attacks_right = self.attacks_right(is_home);

        let opponent_range = if is_home { 11..22 } else { 0..11 };

        // 전방 15m 내에 상대 수비수가 있는지 체크
        let forward_zone_clear = opponent_range.clone().all(|idx| {
            let (opp_x, opp_y) = self.get_player_position_by_index(idx).to_meters();
            let dx = if attacks_right { opp_x - passer_x } else { passer_x - opp_x };
            let dy = (opp_y - passer_y).abs();

            // 전방 5-15m, 좌우 10m 이내에 수비수가 없으면 열림
            !(dx > 5.0 && dx < 15.0 && dy < 10.0)
        });

        forward_zone_clear
    }

    /// FIX_2601/1129: 공 탈취 시 호출 (Transition 진입 기회)
    pub fn notify_turnover(&mut self, is_home_gained: bool) {
        if is_home_gained {
            self.home_attack_state.on_turnover(self.current_tick);
        } else {
            self.away_attack_state.on_turnover(self.current_tick);
        }
    }

    /// FIX_2601/1129: 전진 패스 실패 기록
    pub fn record_forward_pass_failure(&mut self, is_home: bool) {
        if is_home {
            self.home_attack_state.record_forward_failure();
        } else {
            self.away_attack_state.record_forward_failure();
        }
    }

    /// FIX_2601/1129: 전진 패스 성공 기록
    pub fn record_forward_pass_success(&mut self, is_home: bool) {
        if is_home {
            self.home_attack_state.record_forward_success();
        } else {
            self.away_attack_state.record_forward_success();
        }
    }

    /// FIX_2601/1129: 현재 공격 국면 반환 (decision_topology에서 사용)
    pub fn get_attack_phase(&self, is_home: bool) -> super::attack_phase::AttackPhase {
        if is_home {
            self.home_attack_state.phase
        } else {
            self.away_attack_state.phase
        }
    }

    /// P7: 선수 상태 업데이트 (Recovering, Cooldown 틱 감소)
    fn update_player_states_tick(&mut self) {
        use crate::engine::player_state::tick_update_all;

        // 1. PlayerState 틱 업데이트 (Recovering, Staggered, Cooldown 감소)
        tick_update_all(&mut self.player_states);

        // 2. 태클 쿨다운 감소
        for cooldown in self.tackle_cooldowns.iter_mut() {
            if *cooldown > 0 {
                *cooldown -= 1;
            }
        }
    }

    /// P0: 공 위치 기반 골 체크 (Goal Contract)
    ///
    /// 슛 결과가 아닌 공의 최종 위치로 골을 판정한다.
    /// deflection, 펌블, 헤더, 자책골도 자연스럽게 처리됨.
    ///
    /// FIX_2601/0109: 2nd half 골 판정 팀 수정
    /// 하프타임에 공격 방향이 바뀌므로, 골 판정 결과도 반전해야 함
    ///
    /// FIX_2601/0123: When in DispatcherPrimary mode, goals are handled
    /// through evaluate_rules() instead, so this function skips.
    fn check_goals_from_ball_position(&mut self) {
        // Skip when dispatcher is primary - goals handled in evaluate_rules()
        if self.rule_check_mode.dispatcher_applies() {
            return;
        }

        // 이번 틱에서 이미 골이 처리되었으면 스킵 (중복 방지)
        if self.goal_scored_this_tick {
            return;
        }

        // 공이 비행 중이면 스킵 (아직 골라인 통과 전)
        if self.ball.is_in_flight {
            return;
        }

        let (x_m, y_m) = self.ball.position.to_meters();
        let ball_height_m = self.ball.height_meters();

        if let Some(scoring_team) = self.goals.check_goal((x_m, y_m), ball_height_m) {
            // FIX_2601/0116: For physics-based goals, prefer previous_owner (the kicker) over
            // current_owner (who might have just won a loose ball contest but didn't kick).
            // This ensures proper goal attribution for shots/kicks that roll into the goal.
            if self.ball.previous_owner.is_some() {
                self.ball.current_owner = self.ball.previous_owner;
            }

            // A/B comparison tracking for LegacyWithTracking mode
            if self.rule_check_mode.tracking_enabled() {
                let legacy_result = LegacyGoalResult {
                    is_goal: true,
                    scoring_team: Some(RuleTeamId::from_player_index(
                        self.ball.current_owner.unwrap_or(9),
                    )),
                    scorer_idx: self.ball.current_owner,
                    assister_idx: None,
                    position: self.ball.position.clone(),
                };
                let last_touch = if let Some(idx) = self.ball.current_owner {
                    RuleTeamId::from_player_index(idx)
                } else {
                    RuleTeamId::Home
                };
                // This call logs the comparison but doesn't change behavior
                let _decision = check_goal_wrapper(
                    self.rule_check_mode,
                    &mut self.rule_dispatcher,
                    &legacy_result,
                    &self.ball.position,
                    last_touch,
                    self.rng.gen(),
                );
            }

            // FIX_2601/0109: 2nd half에는 공격 방향이 반대이므로 득점팀도 반전
            // 1st half: Home→x=105, Away→x=0 / 2nd half: Home→x=0, Away→x=105
            // Goals struct는 고정(Home goal at x=0, Away goal at x=105)이므로
            // 2nd half에서 x<0 골 = Home 득점, x>105 골 = Away 득점
            let actual_scoring_team = if self.is_second_half {
                scoring_team.opponent()
            } else {
                scoring_team
            };
            self.on_goal_scored(actual_scoring_team);
        }
    }

    /// P0: 골 득점 처리 (Goal Contract)
    ///
    /// 점수 증가 + 골 이벤트 발생 + 킥오프 재시작 예약
    /// 자책골 판정: 공을 마지막에 터치한 선수의 팀 ≠ 득점 팀
    fn on_goal_scored(&mut self, scoring_team: super::super::TeamSide) {
        use crate::engine::tactical_context::TeamSide;

        self.finalize_pass_sequences();

        // 중복 골 방지 플래그 설정
        self.goal_scored_this_tick = true;
        // Restart pulse (kickoff state is applied immediately)
        self.restart_occurred_this_tick = true;
        self.restart_type_this_tick = Some(RestartType::KickOff);

        let is_home = matches!(scoring_team, TeamSide::Home);

        // FIX_2601/0123: Transition to GoalCelebration state
        let scorer_team = if is_home { TeamId::HOME } else { TeamId::AWAY };
        let scorer = MatchPlayerId::new(
            scorer_team,
            self.ball.current_owner.unwrap_or(if is_home { 9 } else { 20 }) as u8 % 11,
        );
        self.game_flow_machine.try_transition(
            TransitionTrigger::GoalScored { scorer },
            self.current_tick,
        );

        // 1. 점수 증가
        #[cfg(debug_assertions)]
        eprintln!(
            "[GOAL] on_goal_scored called! is_home={}, current score={}:{}",
            is_home, self.result.score_home, self.result.score_away
        );
        if is_home {
            self.result.score_home = self.result.score_home.saturating_add(1);
        } else {
            self.result.score_away = self.result.score_away.saturating_add(1);
        }
        self.balance_diagnostics.record_goal(is_home);

        // 2. 마지막 터치 선수 확인
        let last_touch_idx = self
            .ball
            .current_owner
            .or(self.ball.previous_owner)
            .unwrap_or(if is_home { 9 } else { 20 });
        let last_touch_is_home = TeamSide::is_home(last_touch_idx);

        // FIX_2601/0116: Debug trace for goal attribution
        #[cfg(debug_assertions)]
        {
            let ball_pos_m = self.ball.position_meters();
            // If scorer is a GK in 2nd half, trace their actual position
            if (last_touch_idx == 0 || last_touch_idx == 11) && self.is_second_half {
                let gk_pos_m = self.player_positions[last_touch_idx].to_meters();
                eprintln!(
                    "[GK_GOAL_2H] min={} scorer_idx={} scorer_pos=({:.1},{:.1}) ball_pos=({:.1},{:.1}) is_home={}",
                    self.minute, last_touch_idx, gk_pos_m.0, gk_pos_m.1, ball_pos_m.0, ball_pos_m.1, is_home
                );
            }
            eprintln!(
                "[GOAL_TRACE] min={} is_2nd_half={} ball_x={:.1} scorer={} is_home={} current_owner={:?} previous_owner={:?}",
                self.minute, self.is_second_half, ball_pos_m.0, last_touch_idx, is_home,
                self.ball.current_owner, self.ball.previous_owner
            );
        }

        // 3. 자책골 판정: 마지막 터치 선수의 팀 ≠ 득점 팀
        // 예: 홈팀 골대에 들어감(is_home=true=홈팀 득점) 근데 마지막 터치가 홈 선수(last_touch_is_home=true)면 자책골
        let is_own_goal = last_touch_is_home != is_home;

        // 4. 골 이벤트 발생
        let ball_pos_m = self.ball.position_meters();
        let ball_position = (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters());

        if is_own_goal {
            // 자책골: OwnGoal 이벤트 발생
            if match_debug_enabled() {
                eprintln!(
                    "[OWN_GOAL] Player {} scored own goal! scoring_team_is_home={}",
                    last_touch_idx, is_home
                );
            }

            // C6: Use last_touch_idx directly as track_id
            self.emit_event(MatchEvent::own_goal_with_position(
                self.minute,
                self.current_timestamp_ms(),
                is_home, // 득점 팀 (자책골의 수혜 팀)
                last_touch_idx,
                ball_position,
            ));
        } else {
            // 일반 골
            // FIX_2601/0102: Assist Candidate System
            // 10초 윈도우 내 유효한 패스 어시스트 확인
            let assist_idx = self
                .assist_candidate
                .filter(|c| c.is_for_team(scoring_team))
                .filter(|c| c.is_valid(self.current_tick as u32, last_touch_idx))
                .map(|c| c.passer_idx);

            // 어시스트 후보 초기화 (사용 후)
            self.assist_candidate = None;

            // C6: Use last_touch_idx and assist_idx directly as track_id
            self.emit_event(MatchEvent::goal_with_position(
                self.minute,
                self.current_timestamp_ms(),
                is_home,
                last_touch_idx,
                assist_idx,
                ball_position,
            ));
        }

        // 5. P6: ReplayRecorder 기록
        let t_seconds = self.current_tick as f64 * 0.25; // 4 ticks/sec
        let team_id = if is_home { 0u32 } else { 1u32 };
        let ball_pos_m = self.ball.position.to_meters();
        let at_m = MeterPos { x: ball_pos_m.0 as f64, y: ball_pos_m.1 as f64 };
        if let Some(ref mut recorder) = self.replay_recorder {
            recorder.record_goal(
                t_seconds,
                team_id,
                last_touch_idx as u32,
                at_m,
                if is_own_goal { None } else { self.ball.previous_owner.map(|i| i as u32) },
            );
        }

        // 6. 킥오프 재시작 예약 (득점하지 않은 팀이 킥오프)
        // 공을 중앙으로 리셋
        self.ball.position = Coord10::CENTER; // 필드 중앙
        self.ball.height = 0;
        self.ball.velocity = Vel10::default();
        self.ball.is_in_flight = false;

        // 득점하지 않은 팀의 공격수가 킥오프
        let kickoff_team_is_home = !is_home;
        let kicker_idx = if kickoff_team_is_home { 10 } else { 21 }; // 스트라이커
        self.ball.current_owner = Some(kicker_idx);
        self.ball.previous_owner = None;

        // FIX_2601/0105: Move kickoff player to center spot
        // Without this, ball teleports to player's formation position on next tick
        if kicker_idx < self.player_positions.len() {
            self.player_positions[kicker_idx] = Coord10::CENTER;
        }

        self.emit_event(MatchEvent::kick_off(
            self.minute,
            self.current_timestamp_ms(),
            kickoff_team_is_home,
        ));

        // FIX_2601/0120: Track kickoff tick for bias analysis phase detection
        self.last_kickoff_tick = self.current_tick;

        // FIX_2601/0123: Update team momentum on goal
        {
            use super::momentum::events;
            let home_score = self.result.score_home;
            let away_score = self.result.score_away;

            if is_home {
                // Home scored
                let delta = if home_score > away_score {
                    events::LEAD_EXTENDED // Extended lead
                } else {
                    events::GOAL_SCORED
                };
                self.home_momentum.apply_event(delta);
                self.away_momentum.apply_event(events::GOAL_CONCEDED);
            } else {
                // Away scored
                let delta = if away_score > home_score {
                    events::LEAD_EXTENDED // Extended lead
                } else {
                    events::GOAL_SCORED
                };
                self.away_momentum.apply_event(delta);
                self.home_momentum.apply_event(events::GOAL_CONCEDED);
            }
        }

        // ActionQueue 리셋 (진행 중인 액션 취소)
        self.action_queue.clear();
    }

    /// P7: Phase FSM 기반 액션 실행
    ///
    /// 1. pending → active 전환 (PlayerState 체크)
    /// 2. active 액션 틱 업데이트 (Phase 진행)
    /// 3. Resolve Phase 액션 실행
    /// 4. tick_results 처리
    fn execute_phase_tick(&mut self) {
        use crate::engine::action_queue::ActionType;

        let tick = self.current_tick;

        // PlayerState 스냅샷 (borrow 문제 해결)
        let player_states_snapshot: Vec<_> = self.player_states.to_vec();
        let tackle_cooldowns_snapshot = self.tackle_cooldowns;

        // 1. pending → active 전환
        let activated = self.action_queue.activate_pending_actions(
            tick,
            TeamSide::team_id,
            |idx, action_type| {
                // can_start_action_for_queue 로직 인라인
                if idx >= player_states_snapshot.len() {
                    return false;
                }
                let state = &player_states_snapshot[idx];
                if !state.can_start_action() {
                    return false;
                }
                match action_type {
                    ActionType::Tackle { .. } => {
                        if idx < tackle_cooldowns_snapshot.len()
                            && tackle_cooldowns_snapshot[idx] > 0
                        {
                            return false;
                        }
                        state.can_tackle()
                    }
                    ActionType::Pass { .. } => state.can_pass(),
                    ActionType::Shot { .. } => state.can_shoot(),
                    ActionType::Dribble { .. } => state.can_dribble(),
                    _ => state.can_start_action(),
                }
            },
        );

        // 활성화된 액션의 PlayerState 업데이트
        for (player_idx, action_id) in &activated {
            self.on_action_started(*player_idx, *action_id);
        }

        // 2. active 액션 틱 업데이트 → Resolve Phase 진입 인덱스 반환
        let mut resolve_indices = self.action_queue.tick_active_actions(tick);

        // FIX_2601/0120: Shuffle resolve_indices for team-neutral order
        // This breaks the Home-first bias from sequential slot iteration
        {
            use rand::seq::SliceRandom;
            use rand::SeedableRng;
            // Use tick as seed for deterministic shuffle (same seed = same shuffle)
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(tick);
            resolve_indices.shuffle(&mut rng);
        }

        // 3. Resolve Phase 액션 실행 (실제 판정)
        for idx in resolve_indices {
            if let Some(active) = self.action_queue.get_active_action(idx) {
                let _player_idx = active.player_idx;
                let _action_type = active.action_type;

                // 실제 판정 실행 (기존 execute_* 로직 재사용)
                let result = self.execute_resolve_for_active(idx);

                // 결과 처리
                self.handle_action_result(result);

                // Resolve → Recover 전환
                self.action_queue.action_to_recover(idx, tick);
            }
        }

        // 4. Finished 액션 정리 및 PlayerState 업데이트
        let finished_indices = self.action_queue.get_finished_actions();
        for idx in &finished_indices {
            if let Some(active) = self.action_queue.get_active_action(*idx) {
                let player_idx = active.player_idx;
                let action_type = active.action_type;

                // 쿨다운/회복 시간 결정
                let (recovery, cooldown) = self.get_action_recovery_cooldown(action_type);
                self.on_action_finished(player_idx, action_type, recovery, cooldown);
            }
        }
        self.action_queue.remove_finished_actions();

        // 5. tick_results drain 및 처리 (이벤트/통계에 이미 반영됨)
        // tick_results는 record_result에서 이미 저장됨
    }

    /// P7: Resolve Phase 액션의 실제 판정 실행
    fn execute_resolve_for_active(&mut self, action_idx: usize) -> ActionResult {
        let ctx = self.build_execution_context();
        self.execute_resolve_for_active_with_ctx(action_idx, &ctx)
    }

    /// FIX_2601/0120: 2-Phase Resolve - execute with pre-built context
    /// This allows multiple actions to be resolved using the same snapshot
    fn execute_resolve_for_active_with_ctx(
        &mut self,
        action_idx: usize,
        ctx: &ExecutionContext,
    ) -> ActionResult {
        // P0: Core types moved to action_queue
        use crate::engine::action_queue::{ActionResult, PhaseActionType};

        let active = match self.action_queue.get_active_action(action_idx) {
            Some(a) => a.clone(), // borrow 문제 해결을 위해 clone
            None => {
                return ActionResult::Cancelled {
                    action_id: 0,
                    reason: "Action not found".to_string(),
                }
            }
        };

        // 기존 execute_* 함수들을 재사용하기 위해 ScheduledAction으로 변환
        let scheduled = self.active_to_scheduled(&active);

        match active.action_type {
            PhaseActionType::Pass => execute_pass(&scheduled, ctx, &mut self.action_queue),
            PhaseActionType::Shot => execute_shot(&scheduled, ctx, &mut self.action_queue),
            PhaseActionType::Tackle => execute_tackle(&scheduled, ctx, &mut self.action_queue),
            PhaseActionType::Dribble => execute_dribble(&scheduled, ctx, &mut self.action_queue),
            PhaseActionType::Trap => execute_trap(&scheduled, ctx, &mut self.action_queue),
            PhaseActionType::Intercept => {
                execute_intercept(&scheduled, ctx, &mut self.action_queue)
            }
            PhaseActionType::Move => execute_move(&scheduled, ctx, &mut self.action_queue),
            PhaseActionType::Header => {
                let result = execute_header(&scheduled, ctx, &mut self.action_queue);
                if let Some(outcome) = self.action_queue.take_last_header_outcome() {
                    self.record_header_attempt(outcome.player_idx, outcome.is_shot);
                    if outcome.success {
                        self.record_header_success(outcome.player_idx);
                    }
                }
                result
            }
            PhaseActionType::Save => execute_save(&scheduled, ctx, &mut self.action_queue, 0.15),
        }
    }

    /// P7: ActiveAction → ScheduledAction 변환 (레거시 execute_* 함수 호환용)
    // P0: Core types moved to action_queue
    fn active_to_scheduled(
        &self,
        active: &crate::engine::action_queue::ActiveAction,
    ) -> ScheduledAction {
        use crate::engine::action_detail::ActionDetail;
        use crate::engine::action_queue::{ActionMeta, PhaseActionType};

        let mut detail = ActionDetail::default();
        let action_type = match active.action_type {
            PhaseActionType::Tackle => {
                ActionType::Tackle { target_idx: active.target_player_idx.unwrap_or(0) }
            }
            PhaseActionType::Pass => {
                // FIX_2601/1128: target_pos 추출하여 intended_target_pos로 사용
                // FIX_2601/1129: intended_passer_pos도 ActionMeta::Pass에서 추출
                let (is_long, is_through, intended_target_pos, intended_passer_pos) = match &active.meta {
                    ActionMeta::Pass { pass_type, target_pos, intended_passer_pos, .. } => {
                        use crate::engine::action_queue::PassType;
                        let is_long = matches!(pass_type, PassType::Lofted | PassType::Cross);
                        let is_through = matches!(pass_type, PassType::ThroughBall);
                        detail.pass_type = Some(pass_type.to_detail());
                        (is_long, is_through, Some(*target_pos), *intended_passer_pos)
                    }
                    _ => (false, false, None, None),
                };
                ActionType::Pass {
                    target_idx: active.target_player_idx.unwrap_or(0),
                    is_long,
                    is_through,
                    intended_target_pos,
                    intended_passer_pos,
                }
            }
            PhaseActionType::Shot => {
                use crate::engine::types::coord10::Coord10;
                use crate::models::TeamSide;
                let (power, target) = match &active.meta {
                    ActionMeta::Shot { power, target_pos, .. } => (*power, *target_pos),
                    // FIX_2601: Use correct goal based on shooter's team (was hardcoded 105.0)
                    _ => {
                        let dir_ctx = if TeamSide::is_home(active.player_idx) {
                            &self.home_ctx
                        } else {
                            &self.away_ctx
                        };
                        let goal_x = dir_ctx.opponent_goal_x() * field::LENGTH_M;
                        (0.8, Coord10::from_meters(goal_x, field::CENTER_Y))
                    }
                };
                ActionType::Shot { power, target }
            }
            PhaseActionType::Dribble => {
                let (direction, aggressive) = match &active.meta {
                    ActionMeta::Dribble { direction, is_aggressive, .. } => {
                        (*direction, *is_aggressive)
                    }
                    _ => ((1.0, 0.0), false),
                };
                ActionType::Dribble { direction, aggressive }
            }
            PhaseActionType::Move => {
                use crate::engine::types::coord10::Coord10;
                let (target, sprint) = match &active.meta {
                    ActionMeta::Move { target, is_sprint } => (*target, *is_sprint),
                    _ => (Coord10::CENTER, false),
                };
                ActionType::Move { target, sprint }
            }
            PhaseActionType::Trap => ActionType::Trap { ball_speed: 15.0, ball_height: 0.5 },
            PhaseActionType::Intercept => {
                use crate::engine::types::coord10::Coord10;
                ActionType::Intercept {
                    ball_position: active
                        .target_position
                        .unwrap_or(Coord10::CENTER),
                }
            }
            PhaseActionType::Header => {
                use crate::engine::action_queue::ActionMeta;
                use crate::engine::types::coord10::Coord10;
                use crate::models::TeamSide;
                // FIX_2601/0115: Read is_shot from meta (was hardcoded true - caused all headers to become shots!)
                let is_shot = match &active.meta {
                    ActionMeta::Header { is_shot } => *is_shot,
                    _ => {
                        // Fallback: If not in penalty box, it's a pass
                        let dir_ctx = if TeamSide::is_home(active.player_idx) {
                            &self.home_ctx
                        } else {
                            &self.away_ctx
                        };
                        let goal_x = dir_ctx.opponent_goal_x() * field::LENGTH_M;
                        let target_m = active
                            .target_position
                            .map(|p| p.to_meters())
                            .unwrap_or((goal_x, field::CENTER_Y));
                        // In penalty box = shot
                        (target_m.0 - goal_x).abs() < 16.5
                            && target_m.1 > 13.84
                            && target_m.1 < 54.16
                    }
                };
                let dir_ctx = if TeamSide::is_home(active.player_idx) {
                    &self.home_ctx
                } else {
                    &self.away_ctx
                };
                let goal_x = dir_ctx.opponent_goal_x() * field::LENGTH_M;
                ActionType::Header {
                    target: active.target_position.unwrap_or(Coord10::from_meters(goal_x, field::CENTER_Y)),
                    is_shot,
                }
            }
            PhaseActionType::Save => {
                use crate::engine::types::coord10::Coord10;
                let target_pos = active.target_position.unwrap_or(Coord10::CENTER);
                ActionType::Save { direction: target_pos.to_meters() }
            }
        };

        ScheduledAction::new_with_detail(
            self.current_tick,
            action_type,
            active.player_idx,
            100,
            active.id,
            detail,
        )
    }

    /// P7: 액션 타입별 recovery/cooldown 틱 반환
    // P0: Core types moved to action_queue
    fn get_action_recovery_cooldown(
        &self,
        action_type: crate::engine::action_queue::PhaseActionType,
    ) -> (u8, u8) {
        use crate::engine::action_queue::PhaseActionType;
        use crate::engine::phase_action::{
            PASS_COOLDOWN_TICKS, PASS_RECOVERY_TICKS, SHOT_COOLDOWN_TICKS, SHOT_RECOVERY_TICKS,
            TACKLE_COOLDOWN_TICKS, TACKLE_RECOVERY_MISS_TICKS,
        };

        match action_type {
            PhaseActionType::Tackle => (TACKLE_RECOVERY_MISS_TICKS, TACKLE_COOLDOWN_TICKS),
            PhaseActionType::Pass => (PASS_RECOVERY_TICKS, PASS_COOLDOWN_TICKS),
            PhaseActionType::Shot => (SHOT_RECOVERY_TICKS, SHOT_COOLDOWN_TICKS),
            _ => (2, 0),
        }
    }

    fn assign_player_objectives_tick(&mut self) {
        let home_phase = self.home_phase_state.phase;
        let away_phase = self.away_phase_state.phase;
        let ball_pos_m = self.ball.position.to_meters(); // already in meters
        let ball_owner = self.ball.current_owner;

        // 거리 계산을 위한 미터 변환
        let positions_m = self.player_positions_in_meters();

        // 각 팀별 공까지 거리 순위 계산
        // FIX_2601/0115: Position-neutral tie-breaker (replaces Y-bias from 0110)
        let mut home_distances: Vec<(usize, f32)> = (0..11)
            .filter_map(|i| positions_m.get(i).map(|p| (i, distance(*p, ball_pos_m))))
            .collect();
        home_distances.sort_by(|a, b| {
            match a.1.partial_cmp(&b.1) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // FIX_2601/0115: Deterministic hash tie-breaker (no position bias)
                    let pos_a = positions_m.get(a.0).copied().unwrap_or((0.0, 0.0));
                    let pos_b = positions_m.get(b.0).copied().unwrap_or((0.0, 0.0));
                    super::deterministic_tie_hash(a.0, pos_a, b.0, pos_b)
                }
                Some(ord) => ord,
            }
        });

        let mut away_distances: Vec<(usize, f32)> = (11..22)
            .filter_map(|i| positions_m.get(i).map(|p| (i, distance(*p, ball_pos_m))))
            .collect();
        away_distances.sort_by(|a, b| {
            match a.1.partial_cmp(&b.1) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // FIX_2601/0115: Deterministic hash tie-breaker (no position bias)
                    let pos_a = positions_m.get(a.0).copied().unwrap_or((0.0, 0.0));
                    let pos_b = positions_m.get(b.0).copied().unwrap_or((0.0, 0.0));
                    super::deterministic_tie_hash(a.0, pos_a, b.0, pos_b)
                }
                Some(ord) => ord,
            }
        });

        // 각 선수에게 목표 할당
        for i in 0..22 {
            if i >= self.player_positions.len() {
                continue;
            }

            let is_home = TeamSide::is_home(i);
            let phase = if is_home { home_phase } else { away_phase };
            let pos = positions_m
                .get(i)
                .copied()
                .unwrap_or((field::CENTER_X, field::CENTER_Y));

            // 팀 내 순위 계산
            let proximity_rank = if is_home {
                home_distances.iter().position(|(idx, _)| *idx == i).unwrap_or(10)
            } else {
                away_distances.iter().position(|(idx, _)| *idx == i).unwrap_or(10)
            };

            // 골대까지 거리 (attacks_right이면 x=105, 아니면 x=0)
            // FIX_2601: Use attacks_right for correct halftime handling
            let goal_x = if self.attacks_right(is_home) { field::LENGTH_M } else { 0.0 };
            let distance_to_opponent_goal =
                ((pos.0 - goal_x).powi(2) + (pos.1 - field::CENTER_Y).powi(2)).sqrt();

            // 가장 가까운 상대 거리
            let opponent_range = TeamSide::opponent_range_for_home(is_home);
            let nearest_opponent_distance = opponent_range
                .filter_map(|j| positions_m.get(j).map(|op| distance(pos, *op)))
                .fold(f32::MAX, f32::min);

            // PositionKey 추정 (간단히 slot 기반)
            let slot = if is_home { i } else { i - 11 };
            let position_key = crate::engine::movement::slot_to_position_key(
                slot,
                if is_home { &self.home_formation } else { &self.away_formation },
            );

            let ctx = ObjectiveContext {
                team_phase: phase,
                has_ball: ball_owner == Some(i),
                position_key,
                distance_to_ball: distance(pos, ball_pos_m),
                distance_to_opponent_goal,
                nearest_opponent_distance,
                proximity_rank,
            };

            let mut objective = assign_objective(&ctx);
            if self.sticky_actions[i].press && phase.should_press() && !ctx.has_ball && slot != 0 {
                objective = PlayerObjective::RecoverBall;
            }
            self.player_objectives[i] = objective;
        }
    }

    fn execute_scheduled_action(
        &mut self,
        action: &ScheduledAction,
        ctx: &ExecutionContext,
    ) -> ActionResult {
        match &action.action_type {
            ActionType::Pass { .. } => execute_pass(action, ctx, &mut self.action_queue),
            ActionType::Trap { .. } => execute_trap(action, ctx, &mut self.action_queue),
            ActionType::Dribble { .. } => execute_dribble(action, ctx, &mut self.action_queue),
            ActionType::Shot { .. } => execute_shot(action, ctx, &mut self.action_queue),
            ActionType::Tackle { .. } => execute_tackle(action, ctx, &mut self.action_queue),
            ActionType::Intercept { .. } => execute_intercept(action, ctx, &mut self.action_queue),
            ActionType::Move { .. } => execute_move(action, ctx, &mut self.action_queue),
            ActionType::Save { .. } => {
                // Save는 기본 xG로 실행
                execute_save(action, ctx, &mut self.action_queue, 0.15)
            }
            ActionType::Header { .. } => {
                // 2025-12-11 P2: Header action implementation
                execute_header(action, ctx, &mut self.action_queue)
            }
        }
    }

    fn generate_initial_action_tick(
        &mut self,
        _home_strength: f32,
        _away_strength: f32,
        _possession_ratio: f32,
    ) {
        // 공 소유자가 없으면 루즈볼 처리 단계에서 결정한다.
        let owner_idx = match self.ball.current_owner {
            Some(idx) => idx,
            None => return,
        };

        self.ball.current_owner = Some(owner_idx);
        self.action_queue.set_ball_state(BallState::Controlled { owner_idx });

        // P16: Gate Chain 통합 진입점 사용 (ActionDetail 포함)
        // select_best_action_with_detail()은 USE_GATE_CHAIN 플래그에 따라
        // Gate Chain (P16) 또는 기존 시스템 (Audacity/Utility) 선택
        self.result.statistics.decisions_executed =
            self.result.statistics.decisions_executed.saturating_add(1);

        // FIX_2601/0120: Use actor-based RNG for order-independent ball owner decisions
        // FIX_2601/1124 Phase 3: select_best_action_with_detail now returns (action, detail, detail_v2)
        #[cfg(feature = "snapshot_decide")]
        let (action, detail, detail_v2, actor_seed) = {
            // Actor seed: base_seed XOR (tick shifted) XOR (owner_idx shifted) XOR stage marker
            let actor_seed = self.original_seed
                ^ (self.current_tick << 16)
                ^ ((owner_idx as u64) << 32)
                ^ (0xBA11 << 48); // Stage marker for ball owner decision (0xBA11 = "BALL")
            let (action, detail, detail_v2) = self.select_best_action_with_detail_snapshot(owner_idx, actor_seed);
            (action, detail, detail_v2, actor_seed)
        };

        #[cfg(not(feature = "snapshot_decide"))]
        let (action, detail, detail_v2) = self.select_best_action_with_detail(owner_idx);

        // PlayerAction + ActionDetail → ActionType 변환
        // NOTE: Actor-based conversion tested but showed regression (0.745 → 0.717)
        // Keeping original conversion for now - the decision phase with snapshot_decide is more important
        #[cfg(feature = "snapshot_decide")]
        let _ = actor_seed; // suppress unused warning

        // FIX_2601/1124 Phase 3: V2 파이프라인 분기
        // detail_v2_pipeline feature가 활성화되면 conversion_v2 사용
        #[cfg(feature = "detail_v2_pipeline")]
        let action_type = {
            use super::conversion_v2::convert_detail_v2_to_action_type;
            let is_home = TeamSide::is_home(owner_idx);
            let attacks_right = self.attacks_right(is_home);

            if let Some(ref v2) = detail_v2 {
                // V2 파이프라인: RNG 없이 변환
                convert_detail_v2_to_action_type(v2, attacks_right)
            } else {
                // V2가 없으면 V1 fallback (UAE pipeline 등)
                self.convert_player_action_with_detail_to_action_type(action, &detail, owner_idx)
            }
        };

        #[cfg(not(feature = "detail_v2_pipeline"))]
        let action_type = {
            // V1 파이프라인 (기존)
            let _ = &detail_v2; // suppress unused warning
            self.convert_player_action_with_detail_to_action_type(action, &detail, owner_idx)
        };

        // 다음 틱에 실행되도록 예약 (with detail)
        self.action_queue.schedule_new_with_detail(
            self.current_tick + 1,
            action_type,
            owner_idx,
            100, // 기본 우선순위
            detail,
        );
    }

    // NOTE: convert_player_action_to_action_type() removed - was never called
    // P16 uses convert_player_action_with_detail_to_action_type() instead

    /// P16: ActionDetail 기반 PlayerAction → ActionType 변환
    ///
    /// ActionDetail에서 pass_type/shot_type/target 등을 읽어 정확한 ActionType 생성
    /// - random_target() 대신 detail.target 사용
    /// - is_long/is_through는 detail.pass_type에서 결정
    fn convert_player_action_with_detail_to_action_type(
        &mut self,
        action: super::super::player_decision::PlayerAction,
        detail: &crate::engine::action_detail::ActionDetail,
        owner_idx: usize,
    ) -> ActionType {
        use super::super::player_decision::PlayerAction;
        use crate::engine::action_detail::{PassType, ShotType as DetailShotType};

        let is_home = TeamSide::is_home(owner_idx);
        let attacks_right = self.attacks_right(is_home);

        // FIX_2601/0120: RNG category for this function
        use super::RngCategory;
        // FIX_2601/1121: Fallback type for detail completeness tracking
        use super::FallbackType;
        // FIX_2601/1122: Deterministic fallback functions (when feature enabled)
        #[cfg(feature = "deterministic_fallback")]
        use super::{deterministic_choice, deterministic_f32, deterministic_subcase};

        match action {
            PlayerAction::Pass
            | PlayerAction::ShortPass
            | PlayerAction::LongPass
            | PlayerAction::ThroughBall
            | PlayerAction::Cross => {
                // FIX_2601/1121: Track target completeness BEFORE fallback
                let has_target = detail.target.as_ref().and_then(|t| t.player_idx()).is_some();
                self.detail_tracker.record_target_completeness(owner_idx, has_target);

                // ActionDetail에서 타겟 추출
                // FIX_2601/0120: Inline random_target to track RNG
                let mut target_idx = detail
                    .target
                    .as_ref()
                    .and_then(|t| t.player_idx())
                    .or(self.last_pass_target) // ev_decision에서 저장한 타겟
                    .unwrap_or_else(|| {
                        // FIX_2601/1121: Track TargetNone fallback
                        self.detail_tracker.record_fallback(owner_idx, FallbackType::TargetNone);

                        // ========== FIX_2601/1122: Deterministic Fallback ==========
                        #[cfg(feature = "deterministic_fallback")]
                        {
                            let teammate_range: Vec<usize> = if is_home {
                                (1..11).filter(|&i| i != owner_idx).collect()
                            } else {
                                (12..22).filter(|&i| i != owner_idx).collect()
                            };
                            let idx = deterministic_choice(
                                self.original_seed,
                                self.current_tick,
                                owner_idx,
                                deterministic_subcase::RANDOM_TARGET,
                                teammate_range.len(),
                            );
                            teammate_range[idx]
                        }

                        #[cfg(not(feature = "deterministic_fallback"))]
                        {
                            // random_target inline with self_retry tracking
                            let mut retry_count = 0u32;
                            let t = if is_home {
                                loop {
                                    let t = self.rng.gen_range(1..11);
                                    self.rng_tracker.record_for_player(owner_idx, RngCategory::Conversion);
                                    retry_count += 1;
                                    if t != owner_idx { break t; }
                                }
                            } else {
                                loop {
                                    let t = self.rng.gen_range(12..22);
                                    self.rng_tracker.record_for_player(owner_idx, RngCategory::Conversion);
                                    retry_count += 1;
                                    if t != owner_idx { break t; }
                                }
                            };
                            // FIX_2601/1121: Track self_retry (only count retries beyond first attempt)
                            if retry_count > 1 {
                                self.detail_tracker.record_fallback(owner_idx, FallbackType::SelfRetry(retry_count - 1));
                            }
                            t
                        }
                    });
                let initial_target_idx = target_idx;

                let target_pos = self.get_player_position_by_index(target_idx);
                let passer_pos = self.get_player_position_by_index(owner_idx);
                let is_forward = coordinates::is_advancing(
                    passer_pos.to_normalized_legacy(),
                    target_pos.to_normalized_legacy(),
                    attacks_right,
                );
                if is_forward && self.is_offside_position(target_idx, is_home) {
                    let valid_targets = self.find_valid_pass_targets(owner_idx, is_home);
                    if !valid_targets.is_empty() {
                        // FIX_2601/1121: Track offside redirect fallback
                        self.detail_tracker.record_fallback(owner_idx, FallbackType::OffsideRedirect);

                        #[cfg(feature = "deterministic_fallback")]
                        {
                            let idx = deterministic_choice(
                                self.original_seed,
                                self.current_tick,
                                owner_idx,
                                deterministic_subcase::OFFSIDE_REDIRECT,
                                valid_targets.len(),
                            );
                            target_idx = valid_targets[idx];
                        }

                        #[cfg(not(feature = "deterministic_fallback"))]
                        {
                            target_idx = valid_targets[self.rng.gen_range(0..valid_targets.len())];
                            self.rng_tracker.record_for_player(owner_idx, RngCategory::Conversion);
                        }
                    }
                }

                // FIX_2601/0123: Reciprocity injection at REDUCED rate to balance metrics
                // Original 100% was too high, now 0% causes density explosion (0.82)
                // Keeping injection helps LIMIT density by constraining targets
                if let Some(reciprocal_target) = self.find_reciprocal_pass_target(owner_idx, is_home) {
                    if reciprocal_target != target_idx {
                        target_idx = reciprocal_target;
                    }
                }

                // ActionDetail에서 pass_type 추출
                let pass_type = detail.pass_type.unwrap_or(PassType::Short);

                let is_long = pass_type.is_long();
                let is_through = pass_type.is_through();

                // FIX_2601/0123: ActionDetail.target에서 intended position 추출
                // Player(target_idx)인 경우 선수 위치 사용 (forward_pass_rate 정확도 향상)
                use crate::engine::action_detail::ActionTarget;
                let mut intended_target_pos = detail
                    .target
                    .as_ref()
                    .and_then(|t| match t {
                        ActionTarget::Player(idx) => Some(self.get_player_position_by_index(*idx)),
                        _ => t.point().map(|(x, y)| Coord10::from_meters(x, y)),
                    });
                if target_idx != initial_target_idx {
                    intended_target_pos = Some(self.get_player_position_by_index(target_idx));
                }

                // FIX_2601/0123: V1 경로에서도 passer position 설정 (forward_pass_rate 측정용)
                let passer_pos = self.get_player_position_by_index(owner_idx);
                ActionType::Pass {
                    target_idx,
                    is_long,
                    is_through,
                    intended_target_pos,
                    intended_passer_pos: Some(passer_pos),
                }
            }
            PlayerAction::Dribble => {
                // FIX_2601: Use attacks_right for correct halftime handling
                let dir_x = world_forward_dir_x(attacks_right);
                let dir_y = detail
                    .get_direction()
                    .map(|(_, y)| y)
                    .unwrap_or_else(|| {
                        // FIX_2601/1121: Track dribble direction fallback
                        self.detail_tracker.record_fallback(owner_idx, FallbackType::DribbleDirection);

                        #[cfg(feature = "deterministic_fallback")]
                        {
                            deterministic_f32(
                                self.original_seed,
                                self.current_tick,
                                owner_idx,
                                deterministic_subcase::DRIBBLE_DIRECTION,
                                -0.3,
                                0.3,
                            )
                        }

                        #[cfg(not(feature = "deterministic_fallback"))]
                        {
                            self.rng_tracker.record_for_player(owner_idx, RngCategory::Conversion);
                            self.rng.gen_range(-0.3..0.3)
                        }
                    });

                let aggressive = self.sticky_actions[owner_idx].dribble;
                ActionType::Dribble { direction: (dir_x, dir_y), aggressive }
            }
            PlayerAction::Shoot => {
                use crate::engine::types::coord10::Coord10;
                // FIX_2601/0105: Track shot for budget system BEFORE execution
                self.record_shot_for_budget(is_home);
                // FIX_2601/1121: Track shot attempt for detail completeness
                self.detail_tracker.record_shot_attempt(owner_idx);

                let shot_type = detail.shot_type.unwrap_or(DetailShotType::Normal);

                // ActionDetail에서 타겟 추출 (normalized -> Coord10)
                // FIX_2601: Use attacks_right for correct halftime handling
                let target_norm = if let Some(point) = detail.target.as_ref().and_then(|t| t.point()) {
                    point
                } else {
                    // FIX_2601/1121: Track shot target Y fallback
                    self.detail_tracker.record_fallback(owner_idx, FallbackType::ShotTargetY);
                    let ctx = team_view_context(self, is_home);
                    let goal = ctx.to_world(TeamViewCoord10::OPPONENT_GOAL);
                    let goal_x = goal.x as f32 / Coord10::FIELD_LENGTH_10 as f32;

                    #[cfg(feature = "deterministic_fallback")]
                    let goal_y = deterministic_f32(
                        self.original_seed,
                        self.current_tick,
                        owner_idx,
                        deterministic_subcase::SHOT_TARGET_Y,
                        0.4,
                        0.6,
                    );

                    #[cfg(not(feature = "deterministic_fallback"))]
                    let goal_y = {
                        self.rng_tracker.record_for_player(owner_idx, RngCategory::Conversion);
                        (0.5_f32 + self.rng.gen_range(-0.1_f32..0.1_f32)).clamp(0.0, 1.0)
                    };

                    (goal_x, goal_y)
                };
                let target = Coord10::from_normalized(target_norm);

                // FIX_2601/1121: Track power completeness
                let has_power = detail.power.is_some();
                self.detail_tracker.record_power_completeness(owner_idx, has_power);

                let power = if let Some(p) = detail.power {
                    p
                } else {
                    // FIX_2601/1121: Track shot power fallback
                    self.detail_tracker.record_fallback(owner_idx, FallbackType::ShotPower);

                    #[cfg(feature = "deterministic_fallback")]
                    {
                        deterministic_f32(
                            self.original_seed,
                            self.current_tick,
                            owner_idx,
                            deterministic_subcase::SHOT_POWER,
                            0.7,
                            1.0,
                        )
                    }

                    #[cfg(not(feature = "deterministic_fallback"))]
                    {
                        self.rng_tracker.record_for_player(owner_idx, RngCategory::Conversion);
                        0.7 + self.rng.gen::<f32>() * 0.3
                    }
                };

                if matches!(shot_type, DetailShotType::Header) {
                    return ActionType::Header { target, is_shot: true };
                }

                ActionType::Shot { power, target }
            }
            PlayerAction::Hold => {
                self.record_hold_action(owner_idx);
                ActionType::Dribble { direction: (0.0, 0.0), aggressive: false }
            }
            PlayerAction::Tackle => {
                let opponent_start = if is_home { 11 } else { 0 };
                // FIX_2601/1121: Track tackle target fallback
                self.detail_tracker.record_fallback(owner_idx, FallbackType::TackleTarget);

                #[cfg(feature = "deterministic_fallback")]
                let target_offset = deterministic_choice(
                    self.original_seed,
                    self.current_tick,
                    owner_idx,
                    deterministic_subcase::TACKLE_TARGET,
                    11,
                );

                #[cfg(not(feature = "deterministic_fallback"))]
                let target_offset = {
                    self.rng_tracker.record_for_player(owner_idx, RngCategory::Conversion);
                    self.rng.gen_range(0..11)
                };

                ActionType::Tackle { target_idx: opponent_start + target_offset }
            }
            PlayerAction::Header => {
                use crate::engine::types::coord10::Coord10;
                // FIX_2601/0105: Track header shot for budget system
                self.record_shot_for_budget(is_home);

                // FIX_2601: Use attacks_right for correct halftime handling
                let goal_x = if self.attacks_right(is_home) { 1.0 } else { 0.0 };
                ActionType::Header { target: Coord10::from_normalized((goal_x, 0.5)), is_shot: true }
            }
            PlayerAction::TakeOn => {
                let (action_type, _beaten_defender) = self.resolve_takeon_duel(owner_idx);
                action_type
            }
        }
    }

    /// FIX_2601/0120: Actor-based RNG version of action conversion
    ///
    /// Uses actor_seed to create independent RNG for each random choice,
    /// eliminating correlation between processing order and RNG values.
    #[cfg(feature = "snapshot_decide")]
    fn convert_player_action_with_detail_to_action_type_actor(
        &mut self,
        action: super::super::player_decision::PlayerAction,
        detail: &crate::engine::action_detail::ActionDetail,
        owner_idx: usize,
        actor_seed: u64,
    ) -> ActionType {
        use super::super::player_decision::PlayerAction;
        use crate::engine::action_detail::{PassType, ShotType as DetailShotType};
        use rand::SeedableRng;

        let is_home = TeamSide::is_home(owner_idx);
        let attacks_right = self.attacks_right(is_home);

        // Actor-based random target (uses seed + stage marker)
        let random_target_actor = |stage: u64| -> usize {
            let seed = actor_seed ^ (stage << 8);
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            if is_home {
                loop {
                    let t = rng.gen_range(1..11);
                    if t != owner_idx {
                        return t;
                    }
                }
            } else {
                loop {
                    let t = rng.gen_range(12..22);
                    if t != owner_idx {
                        return t;
                    }
                }
            }
        };

        match action {
            PlayerAction::Pass
            | PlayerAction::ShortPass
            | PlayerAction::LongPass
            | PlayerAction::ThroughBall
            | PlayerAction::Cross => {
                // ActionDetail에서 타겟 추출
                let mut target_idx = detail
                    .target
                    .as_ref()
                    .and_then(|t| t.player_idx())
                    .or(self.last_pass_target)
                    .unwrap_or_else(|| random_target_actor(0x01));
                let initial_target_idx = target_idx;

                let target_pos = self.get_player_position_by_index(target_idx);
                let passer_pos = self.get_player_position_by_index(owner_idx);
                let is_forward = coordinates::is_advancing(
                    passer_pos.to_normalized_legacy(),
                    target_pos.to_normalized_legacy(),
                    attacks_right,
                );
                if is_forward && self.is_offside_position(target_idx, is_home) {
                    let valid_targets = self.find_valid_pass_targets(owner_idx, is_home);
                    if !valid_targets.is_empty() {
                        // Actor-based RNG for offside fallback
                        let seed = actor_seed ^ (0x02 << 8);
                        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                        target_idx = valid_targets[rng.gen_range(0..valid_targets.len())];
                    }
                }

                // FIX_2601/0123: Reciprocity injection at REDUCED rate (actor variant)
                // Keeping injection helps LIMIT density by constraining targets
                if let Some(reciprocal_target) = self.find_reciprocal_pass_target(owner_idx, is_home) {
                    if reciprocal_target != target_idx {
                        target_idx = reciprocal_target;
                    }
                }

                let pass_type = detail.pass_type.unwrap_or(PassType::Short);
                let is_long = pass_type.is_long();
                let is_through = pass_type.is_through();

                // FIX_2601/0123: ActionDetail.target에서 intended position 추출
                // Player(target_idx)인 경우 선수 위치 사용 (forward_pass_rate 정확도 향상)
                use crate::engine::types::coord10::Coord10;
                use crate::engine::action_detail::ActionTarget;
                let mut intended_target_pos = detail
                    .target
                    .as_ref()
                    .and_then(|t| match t {
                        ActionTarget::Player(idx) => Some(self.get_player_position_by_index(*idx)),
                        _ => t.point().map(|(x, y)| Coord10::from_meters(x, y)),
                    });
                if target_idx != initial_target_idx {
                    intended_target_pos = Some(self.get_player_position_by_index(target_idx));
                }

                // FIX_2601/0123: V1 Actor 경로에서도 passer position 설정 (forward_pass_rate 측정용)
                let passer_pos = self.get_player_position_by_index(owner_idx);
                ActionType::Pass {
                    target_idx,
                    is_long,
                    is_through,
                    intended_target_pos,
                    intended_passer_pos: Some(passer_pos),
                }
            }
            PlayerAction::Dribble => {
                let dir_x = world_forward_dir_x(attacks_right);
                let dir_y = detail
                    .get_direction()
                    .map(|(_, y)| y)
                    .unwrap_or_else(|| {
                        // Actor-based RNG for dribble direction
                        let seed = actor_seed ^ (0x03 << 8);
                        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                        rng.gen_range(-0.3..0.3)
                    });

                let aggressive = self.sticky_actions[owner_idx].dribble;
                ActionType::Dribble { direction: (dir_x, dir_y), aggressive }
            }
            PlayerAction::Shoot => {
                use crate::engine::types::coord10::Coord10;
                self.record_shot_for_budget(is_home);

                let shot_type = detail.shot_type.unwrap_or(DetailShotType::Normal);

                let target_norm =
                    detail.target.as_ref().and_then(|t| t.point()).unwrap_or_else(|| {
                        let ctx = team_view_context(self, is_home);
                        let goal = ctx.to_world(TeamViewCoord10::OPPONENT_GOAL);
                        let goal_x = goal.x as f32 / Coord10::FIELD_LENGTH_10 as f32;
                        // Actor-based RNG for shot target Y
                        let seed = actor_seed ^ (0x04 << 8);
                        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                        let goal_y = (0.5_f32 + rng.gen_range(-0.1_f32..0.1_f32)).clamp(0.0, 1.0);
                        (goal_x, goal_y)
                    });
                let target = Coord10::from_normalized(target_norm);

                // Actor-based RNG for shot power
                let power = detail.power.unwrap_or_else(|| {
                    let seed = actor_seed ^ (0x05 << 8);
                    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                    0.7 + rng.gen::<f32>() * 0.3
                });

                if matches!(shot_type, DetailShotType::Header) {
                    return ActionType::Header { target, is_shot: true };
                }

                ActionType::Shot { power, target }
            }
            PlayerAction::Hold => {
                self.record_hold_action(owner_idx);
                ActionType::Dribble { direction: (0.0, 0.0), aggressive: false }
            }
            PlayerAction::Tackle => {
                let opponent_start = if is_home { 11 } else { 0 };
                // Actor-based RNG for tackle target
                let seed = actor_seed ^ (0x06 << 8);
                let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                ActionType::Tackle { target_idx: opponent_start + rng.gen_range(0..11) }
            }
            PlayerAction::Header => {
                use crate::engine::types::coord10::Coord10;
                self.record_shot_for_budget(is_home);
                let goal_x = if self.attacks_right(is_home) { 1.0 } else { 0.0 };
                ActionType::Header { target: Coord10::from_normalized((goal_x, 0.5)), is_shot: true }
            }
            PlayerAction::TakeOn => {
                let (action_type, _beaten_defender) = self.resolve_takeon_duel(owner_idx);
                action_type
            }
        }
    }

    /// FIX_2601/0115: Update off-ball decisions for all 22 players.
    ///
    /// This is called each tick after on-ball decisions but before positioning_engine.
    /// The offball_objectives are TTL-based and persist until expired (not recalculated every tick).
    ///
    /// Only runs when `offball_decisions_enabled` is true in ExpConfig.
    fn update_offball_decisions_tick(&mut self) {
        // Check feature flag
        let offball_enabled = self
            .exp_params
            .as_ref()
            .map(|p| p.offball_decisions_enabled)
            .unwrap_or(false);

        if !offball_enabled {
            return;
        }

        // Gather inputs for offball decision system
        let positions_m = self.player_positions_in_meters();
        if positions_m.len() < 22 {
            return;
        }

        // Convert to fixed-size arrays
        let mut player_positions: [(f32, f32); 22] = [(0.0, 0.0); 22];
        let mut player_staminas: [f32; 22] = [0.8; 22];
        let mut base_positions: [(f32, f32); 22] = [(52.5, 34.0); 22];

        for i in 0..22 {
            player_positions[i] = positions_m[i];
            player_staminas[i] = self.stamina[i];

            // Get base formation position
            let team_idx = if i < 11 { 0 } else { 1 };
            let player_idx_in_team = if i < 11 { i } else { i - 11 };
            if player_idx_in_team < self.base_formations[team_idx].len() {
                base_positions[i] = self.base_formations[team_idx][player_idx_in_team];
            }
        }

        let ball_pos = self.ball.position.to_meters();
        let ball_vel = self.ball.velocity.to_mps(); // (vx, vy) in m/s

        // Determine possession team
        let possession_team = self
            .ball
            .current_owner
            .map(|idx| if idx < 11 { 0u8 } else { 1u8 })
            .unwrap_or(0);

        // Determine attack direction for home team
        let home_attacks_right = self.attacks_right(true);

        // Check if possession changed this tick
        let possession_changed = self.possession_changed_this_tick;

        // Calculate ticks since transition from TransitionState
        // TRANSITION_WINDOW_MS = 3000ms, DECISION_TICK_MS = 250ms
        // ticks_since_transition = (3000 - remaining_ms) / 250
        let ticks_since_transition = match self.transition_system.state().remaining_ms() {
            Some(remaining_ms) => {
                let elapsed_ms =
                    crate::engine::transition_system::TRANSITION_WINDOW_MS.saturating_sub(remaining_ms);
                (elapsed_ms / crate::engine::transition_system::DECISION_TICK_MS) as u64
            }
            None => 100, // Not in transition - use large value
        };

        // Call offball decision system
        let _decisions_made = crate::engine::offball::update_offball_decisions(
            &mut self.offball_objectives,
            &player_positions,
            &player_staminas,
            &base_positions,
            ball_pos,
            ball_vel,
            self.ball.current_owner,
            possession_team,
            home_attacks_right,
            ticks_since_transition,
            possession_changed,
            self.current_tick,
            &self.offball_config,
        );

        // Debug logging (only in debug builds)
        #[cfg(debug_assertions)]
        if self.current_tick % 240 == 0 {
            // Once per minute
            let valid_count = self
                .offball_objectives
                .iter()
                .filter(|o| o.is_valid())
                .count();
            if match_debug_enabled() && valid_count > 0 {
                println!(
                    "[DEBUG-0115-OFFBALL] tick={} valid_objectives={}",
                    self.current_tick, valid_count
                );
            }
        }
    }

    fn update_positioning_tick(&mut self) {
        if self.player_positions.is_empty() {
            return;
        }

        let ball_pos_m = self.ball.position.to_meters();
        let positions_m = self.player_positions_in_meters();

        // 오프사이드 라인 업데이트 (FIX_2601/0105: is_second_half 전달)
        let home_positions_m: Vec<_> = positions_m[0..11].to_vec();
        let away_positions_m: Vec<_> = positions_m[11..22].to_vec();

        self.positioning_engine.update_offside_lines(
            &home_positions_m,
            &away_positions_m,
            self.is_second_half,
        );

        // FIX_2601/0107: Update offside trap state for both teams
        self.update_offside_trap_state(&home_positions_m, &away_positions_m, ball_pos_m);

        // FIX_2601/0107: Update GK sweeping state for goalkeepers
        self.update_gk_sweeping_state(ball_pos_m);

        // 역할 할당
        let home_objectives: Vec<_> = self.player_objectives[0..11].to_vec();
        let away_objectives: Vec<_> = self.player_objectives[11..22].to_vec();

        self.positioning_engine.assign_roles(
            self.home_phase_state.phase,
            &home_objectives,
            self.ball.current_owner.filter(|&i| TeamSide::is_home(i)),
            true,
        );
        self.positioning_engine.assign_roles(
            self.away_phase_state.phase,
            &away_objectives,
            self.ball.current_owner.filter(|&i| i >= 11).map(|i| i - 11),
            false,
        );

        // 위치 계산
        let mut cross_landing_home = None;
        let mut cross_landing_away = None;
        if let (Some(crate::engine::action_queue::PassType::Cross), Some(passer_idx)) =
            (self.action_queue.last_pass_type(), self.action_queue.last_passer_idx)
        {
            if let crate::engine::action_queue::BallState::InFlight { to_pos, .. } =
                self.action_queue.ball_state()
            {
                if TeamSide::is_home(passer_idx) {
                    cross_landing_home = Some(*to_pos);
                } else {
                    cross_landing_away = Some(*to_pos);
                }
            }
        }
        // FIX_2601/0109: Pass attacks_right for correct second-half positioning
        let home_attacks_right = self.attacks_right(true);
        let away_attacks_right = self.attacks_right(false);
        self.positioning_engine.calculate_target_positions(
            &positions_m,
            ball_pos_m,
            true,
            self.current_tick,
            cross_landing_home,
            home_attacks_right,
        );
        self.positioning_engine.calculate_target_positions(
            &positions_m,
            ball_pos_m,
            false,
            self.current_tick,
            cross_landing_away,
            away_attacks_right,
        );

        // FIX_2601/0112: Elastic Band 적용 - 팀 블록 이동
        self.apply_elastic_band_positioning(ball_pos_m);

        // FIX_2601/0112: 오프사이드 인지 적용 - 모든 공격 선수의 타겟 조정
        self.apply_offside_awareness(ball_pos_m);

        // P15: 관성 물리 기반 이동 적용 (UFO 움직임 제거)
        // 기존 apply_movement() 대신 관성 물리 사용
        self.apply_inertia_movement();
    }

    /// FIX_2601/0112: Elastic Band 포지셔닝 - 팀 블록 이동
    ///
    /// 현대 축구의 존 대 존 (zone-to-zone) 구현:
    /// - 수비-미드-공격 라인이 연결된 블록으로 이동
    /// - compactness에 따라 라인 간격 조절
    /// - 공 위치에 따라 팀 전체가 상하/좌우 슬라이드
    ///
    /// FIX_2601/0113 Phase 5: 확장 기능 통합
    /// - 실시간 TeamShape/OpponentShape 분석
    /// - 무브먼트 패턴 적용 (Forward/Midfielder)
    /// - 수비 라인 응집력 강제
    fn apply_elastic_band_positioning(&mut self, ball_pos_m: (f32, f32)) {
        use crate::engine::elastic_band::{
            analyze_opponent_shape, calculate_line_cohesion, calculate_team_shape,
            forward_movement_to_offset, midfielder_movement_to_offset, select_forward_movement,
            select_midfielder_movement, update_team_positioning_state, PositionLine,
            MAX_DEFENSIVE_LINE_DEVIATION,
        };

        // 1. 공 소유 상태 확인
        let home_has_possession = self
            .ball
            .current_owner
            .map_or(false, |owner| owner < 11);
        let away_has_possession = self
            .ball
            .current_owner
            .map_or(false, |owner| owner >= 11);

        // 2. 양팀 상태 업데이트
        // FIX_2601/0109: Pass attacks_right instead of is_home for correct second-half positioning
        let home_attacks_right = self.attacks_right(true);
        let away_attacks_right = self.attacks_right(false);
        update_team_positioning_state(
            &mut self.elastic_home_state,
            ball_pos_m,
            home_has_possession,
            &self.elastic_home_tactics,
            home_attacks_right,
        );
        update_team_positioning_state(
            &mut self.elastic_away_state,
            ball_pos_m,
            away_has_possession,
            &self.elastic_away_tactics,
            away_attacks_right,
        );

        // =====================================================================
        // FIX_2601/0113 Phase 5: 실시간 형태 분석
        // =====================================================================

        // 3. 필드 플레이어 위치 수집 (GK 제외)
        let home_positions: Vec<(f32, f32)> =
            (1..11).map(|i| self.player_positions[i].to_meters()).collect();
        let away_positions: Vec<(f32, f32)> = (12..22)
            .map(|i| self.player_positions[i].to_meters())
            .collect();

        // 4. 팀 형태 계산 (실시간)
        let _home_shape = calculate_team_shape(&home_positions);
        let _away_shape = calculate_team_shape(&away_positions);

        // 5. 상대 형태 분석 (무브먼트 결정에 사용)
        // FIX_2601/0117: Use dynamic attack direction (was hardcoded true/false)
        // The parameter indicates which direction the OBSERVER is attacking
        let home_sees_away = analyze_opponent_shape(&away_positions, home_attacks_right);
        let away_sees_home = analyze_opponent_shape(&home_positions, away_attacks_right);

        // 6. 수비 라인 응집력 계산
        let home_defenders: Vec<(f32, f32)> =
            (1..5).map(|i| self.player_positions[i].to_meters()).collect();
        let away_defenders: Vec<(f32, f32)> = (12..16)
            .map(|i| self.player_positions[i].to_meters())
            .collect();
        let home_cohesion = calculate_line_cohesion(&home_defenders);
        let away_cohesion = calculate_line_cohesion(&away_defenders);

        // =====================================================================
        // 7. 각 선수의 타겟 위치 조정
        // =====================================================================
        for player_idx in 0..22 {
            let is_home = player_idx < 11;
            let local_idx = if is_home { player_idx } else { player_idx - 11 };

            // GK는 제외 (별도 로직)
            if local_idx == 0 {
                continue;
            }

            // 공 소유자는 제외
            if Some(player_idx) == self.ball.current_owner {
                continue;
            }

            // 선수 라인 결정 (포메이션 슬롯 기반)
            let line = match local_idx {
                1..=4 => PositionLine::Defender,
                5..=7 => PositionLine::Midfielder,
                8..=10 => PositionLine::Forward,
                _ => continue,
            };

            // 상태와 전술 가져오기
            let (state, tactics, opponent_shape, line_cohesion, team_has_ball) = if is_home {
                (
                    &self.elastic_home_state,
                    &self.elastic_home_tactics,
                    &home_sees_away,
                    &home_cohesion,
                    home_has_possession,
                )
            } else {
                (
                    &self.elastic_away_state,
                    &self.elastic_away_tactics,
                    &away_sees_home,
                    &away_cohesion,
                    away_has_possession,
                )
            };

            // 현재 타겟 위치 가져오기
            if let Some(pos_state) = self.positioning_engine.get_player_state(player_idx) {
                let target_m = pos_state.target_position.to_meters();
                let role = pos_state.role;

                // 라인 X좌표 결정
                let line_x = match line {
                    PositionLine::Goalkeeper => continue,
                    PositionLine::Defender => state.defensive_line_x,
                    PositionLine::Midfielder => state.midfield_line_x,
                    PositionLine::Forward => state.attack_line_x,
                };

                // 역할에 따른 블렌딩 강도
                // FIX_2601/1120: 블렌드 값 상향 - 논문 기반 캘리브레이션
                // 기존 0.05~0.25 → 0.1~0.6 (팀이 실제로 라인에 붙어서 움직이도록)
                let blend = match role {
                    crate::engine::positioning_engine::PositioningRole::Cover => {
                        0.4 + tactics.compactness * 0.2 // 0.4 ~ 0.6
                    }
                    crate::engine::positioning_engine::PositioningRole::Support => {
                        0.3 + tactics.compactness * 0.2 // 0.3 ~ 0.5
                    }
                    crate::engine::positioning_engine::PositioningRole::Penetrate => {
                        0.15 // 침투는 자유롭게
                    }
                    crate::engine::positioning_engine::PositioningRole::Stretch => {
                        0.1 // 스트레치는 더 자유롭게
                    }
                    crate::engine::positioning_engine::PositioningRole::Marker => {
                        0.0 // 마커는 스냅 안함 (상대 따라감)
                    }
                    _ => 0.25,
                };

                // X좌표: 라인에 스냅
                let mut adjusted_x = line_x * blend + target_m.0 * (1.0 - blend);

                // Y좌표: 팀 쏠림 적용
                // FIX_2601/1120: 0.05 → 0.2 (측면 압축 체감되도록)
                let shift_y_scaled = state.team_shift_y * 0.2;
                let mut adjusted_y = (target_m.1 + shift_y_scaled).clamp(5.0, 63.0);

                // =============================================================
                // FIX_2601/0113 Phase 5: 무브먼트 패턴 적용
                // =============================================================
                if team_has_ball {
                    // 공격 시: 상대 형태 기반 무브먼트 적용
                    let off_the_ball =
                        self.get_player_off_the_ball(player_idx).clamp(1.0, 20.0) as u8;
                    let anticipation =
                        self.get_player_anticipation(player_idx).clamp(1.0, 20.0) as u8;

                    // FIX_2601/0109: Use attacks_right instead of is_home for movement offsets
                    let player_attacks_right = if is_home { home_attacks_right } else { away_attacks_right };

                    let (offset_x, offset_y) = match line {
                        PositionLine::Forward => {
                            // 공격수: 상대 형태 기반 무브먼트
                            let movement =
                                select_forward_movement(opponent_shape, off_the_ball, anticipation);
                            let offset =
                                forward_movement_to_offset(movement, player_attacks_right, adjusted_y);
                            // 블렌드 적용 (0.3 = 30%만 반영)
                            (offset.0 * 0.3, offset.1 * 0.3)
                        }
                        PositionLine::Midfielder => {
                            // 미드필더: 상대 형태 + 공 소유 기반
                            let movement = select_midfielder_movement(
                                opponent_shape,
                                off_the_ball,
                                team_has_ball,
                            );
                            let offset =
                                midfielder_movement_to_offset(movement, player_attacks_right, adjusted_y);
                            (offset.0 * 0.25, offset.1 * 0.25)
                        }
                        _ => (0.0, 0.0),
                    };

                    adjusted_x += offset_x;
                    adjusted_y = (adjusted_y + offset_y).clamp(5.0, 63.0);
                }

                // =============================================================
                // FIX_2601/0113 Phase 5: 수비 라인 응집력 강제
                // =============================================================
                if line == PositionLine::Defender && !line_cohesion.is_cohesive {
                    // 라인에서 너무 벗어난 수비수는 라인으로 복귀
                    let deviation = (adjusted_x - line_cohesion.line_x).abs();
                    if deviation > MAX_DEFENSIVE_LINE_DEVIATION {
                        // 라인 방향으로 50% 복귀
                        let correction = (line_cohesion.line_x - adjusted_x) * 0.5;
                        adjusted_x += correction;
                    }
                }

                // 필드 경계 클램핑
                adjusted_x = adjusted_x.clamp(5.0, 100.0);
                adjusted_y = adjusted_y.clamp(5.0, 63.0);

                // 새 타겟 설정
                let new_target =
                    crate::engine::types::Coord10::from_meters(adjusted_x, adjusted_y);
                self.positioning_engine
                    .set_player_target(player_idx, new_target);
            }
        }
    }

    /// FIX_2601/0112: 오프사이드 인지 시스템 (Open Football 방식)
    ///
    /// 모든 공격 선수가 오프사이드 규칙을 인지하고 이동 조정:
    /// - 오프사이드 리스크 평가 (Safe/Marginal/Risky)
    /// - 능력치 기반 안전 위치 계산 (anticipation, off_the_ball)
    /// - 런 타이밍 체크 (패서가 전방 볼 때만 침투)
    ///
    /// FIX_2601/0110: 2-Phase Pattern to eliminate index order bias
    /// - Phase 1: Snapshot positions, calculate all adjustments
    /// - Phase 2: Apply all adjustments in batch
    fn apply_offside_awareness(&mut self, _ball_pos_m: (f32, f32)) {
        use super::offside::{
            calculate_safe_run_position, evaluate_offside_risk, should_make_run, OffsideRisk,
        };

        // ========== Phase 1: Snapshot & Intent Collection ==========
        // Snapshot all positions at tick start (before any adjustments)
        let positions_snapshot: Vec<crate::engine::types::Coord10> = self.player_positions.clone();

        // 현재 공 소유자 확인
        let ball_owner = self.ball.current_owner;

        // 오프사이드 라인 스냅샷 (현재 위치 기준, 틱 시작 시점)
        let home_attacks_right = self.attacks_right(true);
        let home_offside_line = self.positioning_engine.get_offside_line(true);
        let away_offside_line = self.positioning_engine.get_offside_line(false);

        // 패서가 전방을 보고 있는지 확인 (스냅샷 기준)
        let passer_looking_forward = ball_owner.map_or(false, |owner_idx| {
            let owner_pos = positions_snapshot[owner_idx];
            let is_owner_home = owner_idx < 11;
            let ctx = team_view_context(self, is_owner_home);

            // TeamView semantics: "looking forward" ≈ in attacking half.
            ctx.to_team_view(owner_pos).x > Coord10::CENTER_X
        });

        // Collect all adjustment intents (player_idx, new_target)
        let mut adjustments: Vec<(usize, crate::engine::types::Coord10)> = Vec::new();

        for player_idx in 0..22 {
            let is_home = player_idx < 11;
            let is_gk = player_idx == 0 || player_idx == 11;

            // 골키퍼는 스킵
            if is_gk {
                continue;
            }

            // 공 소유자는 스킵
            if Some(player_idx) == ball_owner {
                continue;
            }

            // 현재 팀이 공격 중인지 (공 소유)
            let team_has_ball = match ball_owner {
                Some(owner) => (owner < 11) == is_home,
                None => false,
            };

            // 수비 중이면 스킵 (오프사이드 인지 불필요)
            if !team_has_ball {
                continue;
            }

            // 오프사이드 라인과 공격 방향
            let (offside_line, attacks_right) = if is_home {
                (home_offside_line, home_attacks_right)
            } else {
                (away_offside_line, !home_attacks_right)
            };

            // 현재 타겟 위치 가져오기
            if let Some(state) = self.positioning_engine.get_player_state(player_idx) {
                let target_m = state.target_position.to_meters();

                // 능력치 가져오기 (f32 → u8 변환, 범위 1-20)
                let anticipation =
                    self.get_player_anticipation(player_idx).clamp(1.0, 20.0) as u8;
                let off_the_ball =
                    self.get_player_off_the_ball(player_idx).clamp(1.0, 20.0) as u8;

                // 런 타이밍 체크: 패서가 전방 안보면 무조건 안전 위치로 후퇴
                let can_make_run =
                    should_make_run(anticipation, off_the_ball, passer_looking_forward);

                // 위치 조정 필요 여부
                let risk = evaluate_offside_risk(target_m.0, offside_line, attacks_right);

                // 리스크 있거나 런 타이밍 안되면 안전 위치로 (Intent 수집)
                if risk != OffsideRisk::Safe || !can_make_run {
                    let safe_x = calculate_safe_run_position(
                        target_m.0,
                        offside_line,
                        attacks_right,
                        anticipation,
                    );

                    // Intent 수집 (즉시 적용하지 않음)
                    let new_target =
                        crate::engine::types::Coord10::from_meters(safe_x, target_m.1);
                    adjustments.push((player_idx, new_target));
                }
            }
        }

        // ========== Phase 2: Batch Apply ==========
        // Apply all adjustments after intent collection is complete
        for (player_idx, new_target) in adjustments {
            self.positioning_engine.set_player_target(player_idx, new_target);
        }
    }

    /// P15: 관성 물리 기반 이동 적용
    ///
    /// 기존 UFO 움직임(즉각 정지/방향전환)을 Force 기반 이동으로 변경.
    /// - 스탯 기반 물리 파라미터 (pace→max_speed, acceleration→accel, etc.)
    /// - 피로에 따른 성능 저하
    /// - Arrival Steering (overshoot 방지)
    /// - Turn Penalty (고속 턴 시 속도 손실)
    ///
    /// FIX_2601/0116: 2-Phase Batch Update 적용
    /// - 기존: 순차 업데이트로 Away(11-21)가 Home(0-10)의 새 위치를 "먼저" 볼 수 있음
    /// - 수정: Phase 1에서 모든 계산(스냅샷 기반), Phase 2에서 일괄 적용
    fn apply_inertia_movement(&mut self) {
        use crate::engine::body_orientation::update_body_dir_from_velocity;
        use crate::engine::physics_constants::field::{LENGTH_M, WIDTH_M};
        use crate::engine::player_physics::update_player_motion;
        use crate::engine::timestep::{SUBSTEPS_PER_DECISION, SUBSTEP_DT};
        use crate::engine::types::Coord10;

        // ★ Phase 1.0.5: Dual Timestep Integration ★
        // 기존: DT=0.25 (250ms) 한 번 → 텔레포트 느낌
        // 새: DT=0.05 (50ms) 5번 → 부드러운 이동
        const SUBSTEPS: usize = SUBSTEPS_PER_DECISION as usize;

        // FIX_2601/0106 P3-P4: 스태미나 + 시간 기반 피로 상수
        const STAMINA_REST_THRESHOLD: f32 = 0.30;
        const STAMINA_RESUME_THRESHOLD: f32 = 0.70;
        const BALL_PROXIMITY_THRESHOLD: f32 = 10.0;
        const WALKING_SPEED: f32 = 1.5;
        const FATIGUED_SPEED_MULT: f32 = 0.6;
        const TIME_FATIGUE_TICKS: f32 = 500.0;
        const TIME_FATIGUE_MIN: f32 = 0.5;
        const STICKY_SPRINT_SPEED_MULT: f32 = 1.2;
        const STICKY_SPRINT_ACCEL_MULT: f32 = 1.1;

        // ★ FIX_2601/0116: 2-Phase Batch Update ★
        // 각 선수의 업데이트 결과를 저장할 구조체
        struct PlayerUpdate {
            new_pos: Coord10,
            new_vel: (f32, f32),
            new_speed: f32,
            new_body_dir: (f32, f32),
            new_resting: bool,
            new_running_ticks: u32,
        }

        // ★ 5번 substep 루프 ★
        for _substep in 0..SUBSTEPS {
            // ========== Phase 1: Calculate all new positions (Snapshot-based) ==========
            // 현재 위치 스냅샷 (이 substep 시작 시점의 위치)
            let positions_snapshot: Vec<Coord10> = self.player_positions.clone();
            let velocities_snapshot: [(f32, f32); 22] = self.player_velocities;
            let ball_pos_m = self.ball.position.to_meters();

            let mut updates: [Option<PlayerUpdate>; 22] = Default::default();

            for player_idx in 0..22 {
                // 선수 상태가 이동 불가한 경우 스킵
                if !self.can_player_move(player_idx) {
                    continue;
                }

                // 현재 위치 (스냅샷 기준)
                let pos_m = positions_snapshot[player_idx].to_meters();

                // 목표 위치 (PositioningEngine에서 계산됨)
                let raw_target_m = self.get_player_target_position_m(player_idx);

                // FIX_2601/0107 Phase 8.3: Apply steering behavior to target
                let steering_params = self.select_steering_behavior_params(player_idx, raw_target_m);
                let (target_m, steering_speed_mult) =
                    self.apply_steering_params(player_idx, raw_target_m, &steering_params);

                // 현재 속도 벡터 (스냅샷 기준)
                let vel = velocities_snapshot[player_idx];

                // Ability→MotionParams SSOT (base) + runtime stamina scaling
                let base_params = &self.player_motion_params[player_idx];
                let stamina01 = self.stamina[player_idx];
                let mut params =
                    crate::engine::player_motion_params::scale_by_stamina(base_params, stamina01, 0);

                // FIX_2601/0107 Phase 8.3: Apply steering speed multiplier
                params.max_speed *= steering_speed_mult;
                params.accel *= steering_speed_mult;

                if self.sticky_actions[player_idx].sprint {
                    params.max_speed *= STICKY_SPRINT_SPEED_MULT;
                    params.accel *= STICKY_SPRINT_ACCEL_MULT;
                }

                // 공과의 거리 계산
                let dist_to_ball =
                    ((pos_m.0 - ball_pos_m.0).powi(2) + (pos_m.1 - ball_pos_m.1).powi(2)).sqrt();
                let ball_is_near = dist_to_ball < BALL_PROXIMITY_THRESHOLD;

                // FIX_2601/0106 P4: 시간 기반 피로 계산
                let running_ticks = self.continuous_running_ticks[player_idx] as f32;
                let time_factor = (1.0 - running_ticks / TIME_FATIGUE_TICKS).max(TIME_FATIGUE_MIN);
                params.max_speed *= time_factor;
                params.accel *= time_factor;

                // 휴식/달리기 상태 계산 (새 값)
                let mut new_resting = self.player_resting[player_idx];
                let mut new_running_ticks = self.continuous_running_ticks[player_idx];

                if self.player_resting[player_idx] {
                    // 휴식 중: 연속 달리기 카운터 리셋
                    new_running_ticks = 0;

                    // 스태미나 회복 시 휴식 종료
                    if stamina01 >= STAMINA_RESUME_THRESHOLD {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "[RESTING] Player {} exits rest at tick {} (stamina={:.2}%)",
                            player_idx, self.current_tick, stamina01 * 100.0
                        );
                        new_resting = false;
                    } else if ball_is_near {
                        params.max_speed *= FATIGUED_SPEED_MULT;
                        params.accel *= FATIGUED_SPEED_MULT;
                    } else {
                        params.max_speed = params.max_speed.min(WALKING_SPEED);
                    }
                } else {
                    // 달리는 중: 연속 달리기 카운터 증가
                    new_running_ticks = new_running_ticks.saturating_add(1);

                    if stamina01 < STAMINA_REST_THRESHOLD {
                        if !self.player_resting[player_idx] {
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "[RESTING] Player {} enters rest at tick {} (stamina={:.2}%)",
                                player_idx, self.current_tick, stamina01 * 100.0
                            );
                        }
                        new_resting = true;
                        new_running_ticks = 0;
                        if ball_is_near {
                            params.max_speed *= FATIGUED_SPEED_MULT;
                            params.accel *= FATIGUED_SPEED_MULT;
                        } else {
                            params.max_speed = params.max_speed.min(WALKING_SPEED);
                        }
                    }
                }

                // ★ 관성 물리 업데이트 (50ms 단위) ★
                // FIX_2601/0116: Debug what target is actually used for GK movement
                // Detect when Away GK (11) moves past x=10 in 2nd half (should stay near x=5)
                if player_idx == 11 && self.is_second_half && pos_m.0 > 10.0 {
                    static AWAY_GK_PAST_10: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                    let count = AWAY_GK_PAST_10.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count < 10 {
                        eprintln!(
                            "[AWAY_GK_PAST_10] tick={} pos=({:.1},{:.1}) target=({:.1},{:.1}) vel=({:.2},{:.2})",
                            self.current_tick, pos_m.0, pos_m.1, target_m.0, target_m.1, vel.0, vel.1
                        );
                    }
                }
                let (new_pos_m, new_vel) = update_player_motion(
                    pos_m,
                    vel,
                    target_m,
                    SUBSTEP_DT,
                    &params,
                );

                // 경계 클램핑
                let clamped_pos =
                    (new_pos_m.0.clamp(0.0, LENGTH_M), new_pos_m.1.clamp(0.0, WIDTH_M));

                // 속도 크기
                let speed = (new_vel.0 * new_vel.0 + new_vel.1 * new_vel.1).sqrt();

                // Body direction 계산
                let min_speed_threshold = 0.3;
                let new_body_dir = update_body_dir_from_velocity(
                    self.player_body_dir[player_idx],
                    new_vel,
                    min_speed_threshold,
                );

                // Intent 저장 (즉시 적용하지 않음!)
                updates[player_idx] = Some(PlayerUpdate {
                    new_pos: Coord10::from_meters(clamped_pos.0, clamped_pos.1),
                    new_vel,
                    new_speed: speed,
                    new_body_dir,
                    new_resting,
                    new_running_ticks,
                });
            } // End of player_idx loop (Phase 1)

            // ========== Phase 2: Batch Apply ==========
            // 모든 계산 완료 후 일괄 적용
            for (player_idx, update) in updates.iter().enumerate() {
                if let Some(u) = update {
                    self.player_positions[player_idx] = u.new_pos;
                    self.player_velocities[player_idx] = u.new_vel;
                    self.player_speeds[player_idx] = u.new_speed;
                    self.player_body_dir[player_idx] = u.new_body_dir;
                    self.player_resting[player_idx] = u.new_resting;
                    self.continuous_running_ticks[player_idx] = u.new_running_ticks;
                }
            }
        } // End of substep loop
    } // End of apply_inertia_movement()

    /// P15: 선수가 이동 가능한 상태인지 확인
    fn can_player_move(&self, player_idx: usize) -> bool {
        // PlayerState의 can_move() 메서드 사용
        self.player_states[player_idx].can_move()
    }

    /// P15: 선수의 목표 위치 (미터 단위) 가져오기
    fn get_player_target_position_m(&self, player_idx: usize) -> (f32, f32) {
        // PositioningEngine에서 계산된 목표 위치 사용
        // FIX_2601 Phase 3.3: Coord10 → meters 변환
        let target = if let Some(state) = self.positioning_engine.get_player_state(player_idx) {
            state.target_position_meters()
        } else {
            // 폴백: 현재 위치 유지 - player_positions is now Vec<Coord10>
            self.player_positions[player_idx].to_meters()
        };

        // FIX_2601/0116: Debug GK target during movement (both Home and Away)
        if (player_idx == 0 || player_idx == 11) && self.is_second_half {
            static GK_MOVE_DEBUG_HOME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            static GK_MOVE_DEBUG_AWAY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let count = if player_idx == 0 {
                GK_MOVE_DEBUG_HOME.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            } else {
                GK_MOVE_DEBUG_AWAY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            };
            if count < 5 {
                let current = self.player_positions[player_idx].to_meters();
                let team = if player_idx == 0 { "HOME" } else { "AWAY" };
                eprintln!(
                    "[GK_MOVE_2H-{}] current=({:.1},{:.1}) target=({:.1},{:.1})",
                    team, current.0, current.1, target.0, target.1
                );
            }
        }

        target
    }

    fn resolve_overcrowding_tick(&mut self) {
        if self.player_positions.is_empty() {
            return;
        }

        let positions_m = self.player_positions_in_meters();

        // 홈팀만 (간단히)
        let home_indices: Vec<usize> = (1..11).collect(); // GK 제외
        self.pep_grid.update_from_positions(&positions_m, &home_indices, self.current_tick);

        // 과밀화 해결
        let moves = self.pep_grid.resolve_overcrowding();
        for (player_idx, _from, to) in moves {
            if player_idx < self.player_positions.len() {
                // to.center_y() returns meters (0-68), convert to Coord10 units (0.1m)
                let new_y_10 = (to.center_y() * 10.0).round() as i32;
                self.player_positions[player_idx].y = new_y_10;
            }
        }
    }

    /// P7: 수비 포지셔닝 업데이트
    ///
    /// 수비 팀의 역할 할당 및 Presser 이동 처리
    fn update_defensive_positioning_tick(&mut self) {
        use crate::engine::defensive_positioning::{
            update_defensive_positioning, DefensiveRole, TeamSide,
        };
        use crate::engine::threat_model;

        if self.player_positions.is_empty() {
            return;
        }

        // 공 소유 팀 결정 (TeamPhase 기반, ball flight/loose frames에서도 안정적)
        let attacking_team = if self.home_phase_state.has_possession { 0 } else { 1 };
        let defending_team = 1 - attacking_team;

        let defending_start = defending_team * 11;
        let attacking_start = attacking_team * 11;

        // 미터 단위 위치 (positions_m is the single mutable source for movement)
        let ball_pos_m = self.ball.position.to_meters();
        let mut positions_m = self.player_positions_in_meters();

        // Goal position (P0 Goal Contract)
        // NOTE: Must be computed before we take mutable borrows of other MatchEngine fields.
        let defending_team_side = if defending_team == 0 {
            super::super::TeamSide::Home
        } else {
            super::super::TeamSide::Away
        };
        let own_goal = {
            let goal = self.defending_goal(defending_team_side);
            (goal.center.0, goal.center.1) // MeterPos is (f32, f32)
        };
        // FIX_2601/0110: Compute attacks_right for defending team (before mutable borrows)
        let defending_is_home = defending_team == 0;
        let defending_attacks_right = self.attacks_right(defending_is_home);

        // Team position arrays (index 0 = GK)
        let defending_positions: [(f32, f32); 11] =
            std::array::from_fn(|i| positions_m[defending_start + i]);
        let attacking_positions: [(f32, f32); 11] =
            std::array::from_fn(|i| positions_m[attacking_start + i]);

        // Phase 1.3: MarkingManager input signals (tick-scoped pulses)
        let possession_changed = self.possession_changed_this_tick;
        let restart_occurred = self.restart_occurred_this_tick;
        let restart_type = self.restart_type_this_tick;
        let transition_state = self.transition_system.state();

        // Carrier info (track_id 0-21)
        let carrier_idx = self.ball.current_owner;
        let carrier_id_u8 = carrier_idx.map(|i| i as u8);

        // CarrierFreeScore (T6 input)
        // NOTE: Calculated against the defending team (11 defenders)
        let emergency_threshold: f32 = 0.62; // Balanced preset (see Phase23 spec)
        let free_score: f32 = if let Some(ci) = carrier_idx {
            let carrier_pos = positions_m[ci];
            let carrier_body_dir = self.player_body_dir[ci];
            let carrier_speed = self.player_speeds[ci];
            let is_home_team = TeamSide::is_home(ci);
            // FIX_2601/0109: Use attacks_right for direction-dependent calculations
            let attacks_right = self.attacks_right(is_home_team);

            let goal = self.attacking_goal_for_player(ci);
            let goal_pos = (goal.center.0, goal.center.1);

            let defender_speeds: [f32; 11] =
                std::array::from_fn(|i| self.player_speeds[defending_start + i]);
            let defender_body_dirs: [(f32, f32); 11] =
                std::array::from_fn(|i| self.player_body_dir[defending_start + i]);

            threat_model::calculate_carrier_free_score(
                carrier_pos,
                carrier_body_dir,
                carrier_speed,
                attacks_right,
                &defending_positions,
                &defender_speeds,
                &defender_body_dirs,
                goal_pos,
            )
        } else {
            0.0
        };

        // 1. 역할 재할당 (MarkingManager → DefensiveRole)
        let manager = if defending_team == 0 {
            &mut self.home_marking_manager
        } else {
            &mut self.away_marking_manager
        };

        manager.update(
            self.current_tick,
            ball_pos_m,
            carrier_id_u8,
            &defending_positions,
            &attacking_positions,
            free_score,
            emergency_threshold,
            possession_changed,
            defending_team_side,
            transition_state,
            restart_occurred,
            restart_type,
        );

        let team_side = if defending_team == 0 { TeamSide::Home } else { TeamSide::Away };

        // 2. 역할 재할당 (MarkingManager → DefensiveRole)
        // NOTE: Presser roles are now driven by defensive_positioning::update_defensive_positioning()
        // (so presser movement actually runs).
        let roles = &mut self.defensive_roles[defending_team];
        roles.clear();
        roles.push(DefensiveRole::Goalkeeper);
        for local_idx in 1..11 {
            let state = &manager.states[local_idx];

            let role = if state.is_emergency_presser {
                DefensiveRole::PresserPrimary
            } else if state.is_cover {
                DefensiveRole::PresserSecondary
            } else if state.primary_mark_id >= 0 {
                let target_idx = state.primary_mark_id as usize;
                if target_idx < 11 {
                    DefensiveRole::Marker { target_idx }
                } else {
                    DefensiveRole::Cover
                }
            } else {
                DefensiveRole::Cover
            };

            roles.push(role);
        }

        // 4. 역할별 이동 적용 (single path)
        // FIX_2601/0106 P3: 공 속도와 스태미나 전달
        let ball_velocity_m = (
            self.ball.velocity.vx as f32 * 0.1, // Vel10 (0.1m/s) → m/s
            self.ball.velocity.vy as f32 * 0.1,
        );

        let shape_params = self
            .setup
            .debug
            .ssot_proof
            .as_ref()
            .map(|proof| match team_side {
                TeamSide::Home => proof.shape.home_shape_params,
                TeamSide::Away => proof.shape.away_shape_params,
            })
            .unwrap_or_else(|| {
                let formation = match team_side {
                    TeamSide::Home => &self.setup.home.formation,
                    TeamSide::Away => &self.setup.away.formation,
                };
                crate::fix01::shape_params_from_formation(formation)
            });
        let _presser_events = update_defensive_positioning(
            positions_m.as_mut_slice(),
            &self.player_states,
            &self.tackle_cooldowns,
            roles.as_mut_slice(),
            &mut self.presser_movements[defending_team],
            &mut self.marker_movements[defending_team], // FIX_2601/0106 P3
            &mut self.cover_movements[defending_team],  // FIX_2601/0106 P3
            ball_pos_m,
            ball_velocity_m, // FIX_2601/0106 P3
            carrier_idx,
            shape_params,
            &self.base_formations[defending_team],
            team_side,
            self.defensive_lines[defending_team],
            own_goal,
            &attacking_positions,
            defending_start,
            &self.stamina, // FIX_2601/0106 P3
            defending_attacks_right, // FIX_2601/0110
            |_| 0_u64,     // no tackle scheduling here (handled elsewhere)
            |_| false,     // no tackle permission here (handled elsewhere)
        );

        // Write back defender positions (meters → Coord10)
        // FIX_2601/0117: Skip GK (local_idx=0) - GK position is managed by positioning_engine
        // and apply_inertia_movement. Writing it back here causes oscillation between
        // positioning_engine target and defensive_positioning target.
        use crate::engine::types::Coord10;
        for local_idx in 1..11 {  // Start from 1 to skip GK
            let global_idx = defending_start + local_idx;
            if global_idx < self.player_positions.len() {
                let pos_m = positions_m[global_idx];
                // Defensive positioning can push beyond bounds; keep players in-bounds.
                self.player_positions[global_idx] =
                    Coord10::from_meters(pos_m.0, pos_m.1).clamp_in_bounds();
            }
        }
    }

    /// [DEPRECATED - P7] 기존 ActionQueue 기반 태클 체크
    /// P7 Phase 9에서 decide_defender_tackles()로 대체됨
    #[allow(dead_code)]
    fn check_defender_tackles(&mut self) {
        // P7: FSM 기반 decide_defender_tackles()로 대체
        // 이 함수는 더 이상 호출되지 않음
    }

    fn check_loose_ball_tick(&mut self) {
        if self.player_positions.is_empty() {
            return;
        }

        let positions_m = self.player_positions_in_meters();
        let player_stats = self.build_player_stats();

        if let Some(contest) =
            self.action_queue.check_and_start_loose_ball(&positions_m, &player_stats)
        {
            // 경합 결과 처리
            if let Some(winner_idx) = contest.winner {
                // FIX_2601/1120: Update ball position to winner's position to prevent teleportation
                // Winner "captures" the ball at their location
                self.ball.current_owner = Some(winner_idx);
                self.ball.position = self.player_positions[winner_idx];
                self.action_queue.set_ball_state(BallState::Controlled { owner_idx: winner_idx });
            }
        }
    }

    /// Convert meter position to normalized
    ///
    /// Swaps X/Y because normalized uses (width, length) but meters uses (length, width).
    /// See coordinates.rs for coordinate system documentation.
    fn meters_to_normalized(&self, pos_m: (f32, f32)) -> (f32, f32) {
        use crate::engine::coordinates;
        let pos = coordinates::to_normalized(pos_m);
        // Clamp to valid range
        (pos.0.clamp(0.0, 1.0), pos.1.clamp(0.0, 1.0))
    }

    // 2025-12-11: enable/disable/is_tick_based_sim() 함수들 제거
    // tick 기반 엔진만 사용하므로 feature flag 불필요

    // ========== Phase 4: Hero Time ActionQueue Integration ==========

    /// UserAction을 ActionQueue에 적용 (Hero Time용)
    ///
    /// UserAction을 ActionType으로 변환하여 ActionQueue에 스케줄링.
    /// 2025-12-11: tick 기반 엔진만 사용하므로 feature flag 체크 제거
    pub fn apply_user_action_to_queue(&mut self, action: super::super::types::UserAction) -> bool {
        // 현재 공 소유자 확인
        let owner_idx = match self.ball.current_owner {
            Some(idx) => idx,
            None => return false, // 공 소유자 없으면 적용 불가
        };

        let is_home = TeamSide::is_home(owner_idx);

        // UserAction → ActionType 변환
        let action_type = match action {
            super::super::types::UserAction::Shoot => {
                use crate::engine::types::coord10::Coord10;
                // 슈팅 방향: 골대 중앙 (normalized -> Coord10)
                // FIX_2601/0116: Use DirectionContext+TeamView goal constant (no branching)
                let ctx = team_view_context(self, is_home);
                let goal = ctx.to_world(TeamViewCoord10::OPPONENT_GOAL);
                let goal_x = goal.x as f32 / Coord10::FIELD_LENGTH_10 as f32;
                let target = Coord10::from_normalized((goal_x, 0.5));
                let power = 0.8 + self.rng.gen::<f32>() * 0.2; // 0.8 ~ 1.0
                ActionType::Shot { power, target }
            }
            super::super::types::UserAction::Dribble => {
                // 드리블 방향: 공격 방향
                // FIX_2601: Use attacks_right for correct halftime handling
                let dir_x = world_forward_dir_x(self.attacks_right(is_home));
                let dir_y = self.rng.gen_range(-0.3..0.3);
                ActionType::Dribble {
                    direction: (dir_x, dir_y),
                    aggressive: true, // 유저 선택 = 공격적 드리블
                }
            }
            super::super::types::UserAction::PassTo(target_id) => {
                use crate::engine::types::Coord10;
                let target_idx = target_id as usize;
                // 긴 패스 여부: 거리 기반 (Coord10: 0.1m 단위)
                let owner_pos =
                    self.player_positions.get(owner_idx).copied().unwrap_or(Coord10::CENTER);
                let target_pos =
                    self.player_positions.get(target_idx).copied().unwrap_or(Coord10::CENTER);
                let dist = owner_pos.distance_to(&target_pos);
                let is_long = dist > 300; // 30m 이상이면 롱패스 (300 × 0.1m)

                // FIX_2601/1128: 유저 입력은 현재 위치가 의도한 위치
                // FIX_2601/1129: 유저 입력 경로도 현재 패서 위치 사용
                ActionType::Pass {
                    target_idx,
                    is_long,
                    is_through: false,
                    intended_target_pos: Some(target_pos),
                    intended_passer_pos: Some(owner_pos),
                }
            }
        };

        // schedule_new()로 액션 스케줄링 (ID 자동 할당)
        self.action_queue.schedule_new(
            self.current_tick, // 즉시 실행
            action_type,
            owner_idx,
            10, // 유저 액션 최우선
        );
        true
    }

    /// Hero Time pause 체크 (ActionResult 기반)
    ///
    /// 유저 선수가 트랩 성공 또는 드리블 완료 시 pause 트리거
    pub fn check_hero_time_pause(&self, result: &ActionResult) -> bool {
        // 유저 플레이어 설정 확인
        let _user_config = match &self.user_player {
            Some(config) => config,
            None => return false,
        };

        match result {
            ActionResult::TrapSuccess { player_idx } => {
                self.is_user_controlled_player(*player_idx, TeamSide::is_home(*player_idx))
            }
            ActionResult::CarryComplete { player_idx, .. } => {
                self.is_user_controlled_player(*player_idx, TeamSide::is_home(*player_idx))
            }
            ActionResult::TakeOnComplete { player_idx, .. } => {
                self.is_user_controlled_player(*player_idx, TeamSide::is_home(*player_idx))
            }
            _ => false,
        }
    }

    /// tick-based 시뮬레이션에서 Hero Time 지원 simulate
    ///
    /// pause가 필요하면 SimState::Paused 반환
    pub fn simulate_minute_tick_based_interactive(
        &mut self,
        home_strength: f32,
        away_strength: f32,
        possession_ratio: f32,
    ) -> super::super::types::SimState {
        use super::super::types::SimState;

        for tick_offset in 0..TICKS_PER_MINUTE {
            self.current_tick = self.minute as u64 * TICKS_PER_MINUTE + tick_offset;

            // 0. 틱 시작 시 골 플래그 리셋 (중복 골 방지)
            self.goal_scored_this_tick = false;

            // 1. TeamPhase 업데이트
            self.update_team_phases();

            // 2. PlayerObjective 할당
            self.assign_player_objectives_tick();

            // 3. 공 상태 동기화
            self.action_queue
                .sync_from_ball(self.current_tick, &self.ball);

            // 4. ActionQueue에서 실행할 액션 가져오기
            self.action_queue.resolve_in_flight_woodwork_pre_actions(
                self.current_tick,
                crate::engine::ball_physics_params::DEFAULT,
            );
            let actions = self.action_queue.get_actions_for_tick(self.current_tick);

            // 5. 액션 실행 및 Hero Time 체크
            for action in actions {
                let ctx = self.build_execution_context();
                let result = self.execute_scheduled_action(&action, &ctx);

                // Hero Time pause 체크
                if self.check_hero_time_pause(&result) {
                    if let Some(owner_idx) = self.ball.current_owner {
                        let decision_ctx = self.build_user_decision_context(owner_idx);
                        return SimState::Paused(decision_ctx);
                    }
                }

                self.handle_action_result(result);
            }

            // 6. 새 액션 없으면 생성
            if let Some(owner_idx) = self.ball.current_owner {
                let owner_has_action = self.action_queue.is_player_active(owner_idx)
                    || self.action_queue.has_pending_for_player(owner_idx);
                if owner_has_action {
                    self.result.statistics.owner_action_blocked_ticks =
                        self.result.statistics.owner_action_blocked_ticks.saturating_add(1);
                } else {
                    self.generate_initial_action_tick(home_strength, away_strength, possession_ratio);
                }
            }

            // 6.5. 수비수 태클 체크 (공 소유자 근처 수비수가 태클 시도)
            self.check_defender_tackles();

            // 7. Off-the-Ball 이동
            self.update_positioning_tick();

            // 8. PepGrid 과밀화 해결
            self.resolve_overcrowding_tick();

            // 9. 루즈볼 체크
            self.check_loose_ball_tick();

            // 9.9. 공 상태 진행 (InFlight arrival → Loose, post-action)
            self.action_queue
                .advance_ball_state_post_actions(self.current_tick);

            // 9.95. 루즈볼 물리 진행 (position integration + roll damping)
            self.action_queue.advance_loose_ball_physics(
                self.current_tick,
                crate::engine::ball_physics_params::DEFAULT,
            );

            // 10. 공 상태 역동기화
            self.action_queue.sync_to_ball(&mut self.ball);

            if let Some(out_of_play) = self.detect_out_of_play_action() {
                self.handle_action_result(out_of_play);
            }

            // 11. 위치 기록 (2025-12-11: 엔진이 계산한 실제 위치를 기록)       
            self.record_positions_for_tick();
        }

        SimState::Running
    }

    // ========== Phase 5: Hero Growth Integration ==========

    /// UserAction에서 HeroXpEvent 생성 및 버킷에 추가
    ///
    /// Hero Time에서 유저가 선택한 액션에 대해 XP 이벤트 생성
    pub fn record_hero_action_xp(
        &mut self,
        action: &super::super::types::UserAction,
        success: bool,
    ) {
        // 유저 플레이어가 아니면 무시
        if self.user_player.is_none() {
            return;
        }

        let owner_idx = match self.ball.current_owner {
            Some(idx) => idx,
            None => return,
        };

        // 유저 플레이어 확인
        if !self.is_user_controlled_player(owner_idx, TeamSide::is_home(owner_idx)) {
            return;
        }

        let positions_m = self.player_positions_in_meters();
        let owner_pos = positions_m
            .get(owner_idx)
            .copied()
            .unwrap_or((field::CENTER_X, field::CENTER_Y));

        // 상대 팀 위치들
        let opponent_range = TeamSide::opponent_range(owner_idx);
        let opponents: Vec<(f32, f32)> =
            opponent_range.filter_map(|i| positions_m.get(i).copied()).collect();

        // 압박 레벨 계산
        let pressure = calculate_pressure(owner_pos, &opponents);

        // 피로도
        let fatigue = self.player_fatigue.get(owner_idx).copied().unwrap_or(0.0);

        // 액션 태그와 난이도 계산
        let (tag, difficulty) = match action {
            super::super::types::UserAction::Shoot => {
                // 슈팅 난이도: 골대까지 거리 기반
                let dir_ctx =
                    if TeamSide::is_home(owner_idx) { &self.home_ctx } else { &self.away_ctx };
                let goal_x = dir_ctx.opponent_goal_x() * field::LENGTH_M;
                let dist_to_goal =
                    ((owner_pos.0 - goal_x).powi(2) + (owner_pos.1 - field::CENTER_Y).powi(2)).sqrt();
                let tag = if dist_to_goal < 16.5 {
                    HeroActionTag::BoxShot // 페널티 박스 내
                } else {
                    HeroActionTag::LongShot
                };
                let difficulty = (dist_to_goal / 30.0).min(1.0);
                (tag, difficulty)
            }
            super::super::types::UserAction::Dribble => {
                // 드리블 난이도: 가장 가까운 수비수 거리
                let nearest_dist =
                    opponents.iter().map(|opp| distance(owner_pos, *opp)).fold(f32::MAX, f32::min);
                let difficulty = calculate_dribble_difficulty(owner_pos, nearest_dist, true);
                (HeroActionTag::DribblePastOpponent, difficulty) // 유저 드리블 = 적극적
            }
            super::super::types::UserAction::PassTo(target_id) => {
                let target_idx = *target_id as usize;
                let target_pos = positions_m
                    .get(target_idx)
                    .copied()
                    .unwrap_or((field::CENTER_X, field::CENTER_Y));
                let dist = distance(owner_pos, target_pos);

                // 패스 난이도 계산
                let difficulty = calculate_pass_difficulty(owner_pos, target_pos, &opponents);

                // 패스 타입 결정
                let tag = if dist > 30.0 {
                    HeroActionTag::LobPass // 장거리 패스
                } else if difficulty > 0.5 {
                    HeroActionTag::ThroughPass // 어려운 패스 = 스루 패스
                } else {
                    HeroActionTag::SafePass
                };
                (tag, difficulty)
            }
        };

        // XP 이벤트 생성
        let event = HeroXpEvent::new(tag, success, self.minute)
            .with_pressure(pressure)
            .with_fatigue(fatigue)
            .with_difficulty(difficulty);

        // 버킷에 추가
        self.hero_xp_bucket.add_event(&event);
    }

    /// ActionResult에서 Hero XP 생성 (AI 액션도 포함)
    ///
    /// 유저 플레이어가 관련된 액션 결과에서 자동으로 XP 생성
    pub fn record_action_result_xp(&mut self, result: &ActionResult) {
        // 유저 플레이어 확인
        if self.user_player.is_none() {
            return;
        }

        let positions_m = self.player_positions_in_meters();

        // ActionResult에서 플레이어 인덱스와 성공 여부 추출
        let (player_idx, success, tag, difficulty) = match result {
            ActionResult::PassStarted { passer_idx, .. } => {
                // 패스 시작은 성공으로 간주 (결과는 나중에)
                let pos = positions_m
                    .get(*passer_idx)
                    .copied()
                    .unwrap_or((field::CENTER_X, field::CENTER_Y));
                let opponent_range = TeamSide::opponent_range(*passer_idx);
                let opponents: Vec<_> =
                    opponent_range.filter_map(|i| positions_m.get(i).copied()).collect();
                let pressure = calculate_pressure(pos, &opponents);
                let tag = if pressure > 0.5 {
                    HeroActionTag::ThroughPass
                } else {
                    HeroActionTag::SafePass
                };
                (*passer_idx, true, tag, pressure)
            }
            ActionResult::ShotTaken { shooter_idx, xg, .. } => {
                let tag = if *xg > 0.3 { HeroActionTag::BoxShot } else { HeroActionTag::LongShot };
                (*shooter_idx, true, tag, 1.0 - xg) // 높은 xG = 쉬운 슛
            }
            ActionResult::GoalScored { scorer_idx, .. } => {
                (*scorer_idx, true, HeroActionTag::BoxShot, 0.8)
            }
            ActionResult::TackleSuccess { tackler_idx, .. } => {
                (*tackler_idx, true, HeroActionTag::Tackle, 0.5)
            }
            ActionResult::TackleFoul { tackler_idx, .. } => {
                (*tackler_idx, false, HeroActionTag::Tackle, 0.7)
            }
            ActionResult::TackleFoulAdvantage { tackler_idx, .. } => {
                (*tackler_idx, false, HeroActionTag::Tackle, 0.7)
            }
            ActionResult::InterceptSuccess { player_idx, .. } => {
                (*player_idx, true, HeroActionTag::Interception, 0.5)
            }
            ActionResult::CarryComplete { player_idx, .. } => {
                // Carry (운반) - 낮은 XP (쉬운 행동)
                (*player_idx, true, HeroActionTag::SafeDribble, 0.1)
            }
            ActionResult::TakeOnComplete { player_idx, .. } => {
                // Take-on (돌파 성공) - 높은 XP (위험한 행동)
                (*player_idx, true, HeroActionTag::DribblePastOpponent, 0.6)
            }
            ActionResult::DribbleTackled { player_idx, .. } => {
                (*player_idx, false, HeroActionTag::DribblePastOpponent, 0.7) // 실패한 돌파
            }
            ActionResult::HeaderWon { player_idx, .. } => {
                (*player_idx, true, HeroActionTag::AerialDuel, 0.5)
            }
            // 기타 결과는 XP 없음
            _ => return,
        };

        // 유저 플레이어가 아니면 무시
        if !self.is_user_controlled_player(player_idx, TeamSide::is_home(player_idx)) {
            return;
        }

        // 압박/피로 계산
        let pos = positions_m
            .get(player_idx)
            .copied()
            .unwrap_or((field::CENTER_X, field::CENTER_Y));
        let opponent_range = TeamSide::opponent_range(player_idx);
        let opponents: Vec<_> =
            opponent_range.filter_map(|i| positions_m.get(i).copied()).collect();
        let pressure = calculate_pressure(pos, &opponents);
        let fatigue = self.player_fatigue.get(player_idx).copied().unwrap_or(0.0);

        // XP 이벤트 생성
        let event = HeroXpEvent::new(tag, success, self.minute)
            .with_pressure(pressure)
            .with_fatigue(fatigue)
            .with_difficulty(difficulty);

        self.hero_xp_bucket.add_event(&event);
    }

    /// 경기 종료 시 HeroMatchGrowth 계산
    ///
    /// # Arguments
    /// - `current_stats_fn`: 현재 선수 스탯 조회 함수
    ///
    /// # Returns
    /// Hero 성장 결과 (스탯 증가 + 이월 XP)
    pub fn calculate_hero_growth<F>(&self, current_stats_fn: F) -> HeroMatchGrowth
    where
        F: Fn(PlayerAttribute) -> i8,
    {
        HeroMatchGrowth::from_bucket(&self.hero_xp_bucket, current_stats_fn)
    }

    /// XP 버킷 참조 (읽기 전용)
    pub fn hero_xp_bucket(&self) -> &crate::engine::growth::HeroXpBucket {
        &self.hero_xp_bucket
    }

    /// XP 버킷에 이전 경기 이월 XP 적용
    pub fn apply_xp_overflow(
        &mut self,
        overflow: &std::collections::HashMap<PlayerAttribute, f32>,
    ) {
        self.hero_xp_bucket.apply_overflow(overflow);
    }

    /// 훈련 시너지 보너스 적용
    pub fn apply_training_synergy(&mut self, trained_attrs: &[PlayerAttribute], bonus_rate: f32) {
        self.hero_xp_bucket.apply_training_synergy(trained_attrs, bonus_rate);
    }

    // ========== Position Recording for Tick-Based Simulation ==========

    /// 2025-12-11: 현재 틱의 위치를 기록
    /// simulate_minute_tick_based() 루프 끝에서 호출됨
    ///
    /// 기존 record_positions_for_minute()는 자체적으로 위치를 재계산해서
    /// 엔진 결과를 덮어쓰는 문제가 있었음. 이 메서드는 엔진이 계산한
    /// 실제 위치를 그대로 기록함.
    fn record_positions_for_tick(&mut self) {
        if !self.track_positions {
            return;
        }

        if self.result.position_data.is_none() {
            return;
        }

        // 타임스탬프 계산
        // TICKS_PER_MINUTE = 240, 분당 240틱 (4틱/초)
        // 1틱 = 250ms
        let tick_within_minute = self.current_tick % TICKS_PER_MINUTE;
        let timestamp_ms = self.minute as u64 * 60_000 + tick_within_minute * 250;

        // 먼저 데이터 수집 (immutable borrow)
        let ball_pos_m = self.ball.position.to_meters();
        let ball_velocity = self.ball.velocity.to_mps();
        let ball_height = self.ball.height_meters();

        // 선수 데이터 수집 - WITH VELOCITY (defensive bounds check)
        // FIX_2601: pos is Coord10, use to_meters() directly
        // FIX_2601/0109: Use PlayerState enum instead of String
        use crate::models::PlayerState;
        let mut player_data: Vec<(u8, (f32, f32), (f32, f32), PlayerState)> = Vec::with_capacity(22);
        for (idx, &pos) in self.player_positions.iter().enumerate() {
            let pos_m = pos.to_meters();
            let vel = self.player_velocities.get(idx).copied().unwrap_or((0.0, 0.0));
            let state = self.get_player_state_for_tick(idx);
            player_data.push((idx as u8, pos_m, vel, state));
        }

        // 이제 position_data에 저장 (mutable borrow)
        if let Some(ref mut pos_data) = self.result.position_data {
            pos_data.add_ball_position_with_velocity(
                timestamp_ms,
                ball_pos_m,
                ball_height,
                ball_velocity,
            );

            for (idx, pos_m, vel, state) in player_data {
                pos_data.add_player_position_with_velocity(idx, timestamp_ms, pos_m, vel, state);
            }
        }
    }

    /// 틱 기반 기록용 선수 상태
    /// FIX_2601/0109: Returns PlayerState enum instead of String
    fn get_player_state_for_tick(&self, player_idx: usize) -> crate::models::PlayerState {
        use crate::models::PlayerState;
        if Some(player_idx) == self.ball.current_owner {
            PlayerState::WithBall
        } else {
            let is_home = TeamSide::is_home(player_idx);
            let team_has_ball = self
                .ball
                .current_owner
                .map(|owner| TeamSide::is_home(owner) == is_home)
                .unwrap_or(false);

            if team_has_ball {
                PlayerState::Attacking
            } else {
                PlayerState::Defending
            }
        }
    }

    // ========== P18: FieldBoard Helpers ==========

    /// P18: 22명 선수 위치를 미터 배열로 반환 (FieldBoard 업데이트용)
    /// FIX_2601: player_positions is now Vec<Coord10>
    fn player_positions_meters_array(&self) -> [(f32, f32); 22] {
        let mut arr = [(0.0_f32, 0.0_f32); 22];
        for (idx, &pos) in self.player_positions.iter().enumerate().take(22) {
            arr[idx] = pos.to_meters();
        }
        arr
    }

    /// P18: FieldBoard 틱 업데이트
    /// - occupancy: 매 틱 업데이트
    /// - pressure: 매 3틱 업데이트 (성능 최적화)
    fn update_field_board_tick(&mut self) {
        if self.field_board.is_none() {
            return;
        }

        // 먼저 데이터 수집 (immutable borrow)
        let pos_m = self.player_positions_meters_array();
        let current_tick = self.current_tick;

        // 스태미나 가중치 수집 (pressure 업데이트용)
        let weights: [f32; 22] = std::array::from_fn(|i| self.stamina[i].clamp(0.3, 1.0));

        // 이제 field_board 업데이트 (mutable borrow)
        let board = self.field_board.as_mut().unwrap();

        // Occupancy: 매 틱 업데이트 (cheap)
        board.update_occupancy_from_positions_m(current_tick, &pos_m);

        // Pressure: 매 3틱 업데이트 (비용 절감)
        if current_tick % 3 == 0 {
            board.update_pressure_from_positions_m(
                current_tick,
                &pos_m,
                Some(&weights),
                4.0, // tactical influence radius (NOT immediate tackle pressure)
            );
        }

        // XGZone: 매 10틱 업데이트 (Match OS v1.2)
        if current_tick % 10 == 0 {
            board.xgzone.maybe_update(current_tick);
        }
    }

    // ========== P7 Phase 9: Active FSM Methods ==========

    /// 특정 선수가 활성 태클 FSM을 가지고 있는지 확인
    fn has_active_tackle(&self, player_idx: usize) -> bool {
        // NOTE: ActionQueue 기반으로 변경 (2025-12-14)
        // FSM active_tackles → ActionQueue.is_player_active()
        self.action_queue.is_player_active(player_idx)
    }

    /// 두 선수 사이 거리 (미터 단위)
    /// FIX_2601: Use Coord10::distance_to_m() directly
    fn distance_between(&self, idx_a: usize, idx_b: usize) -> f32 {
        let pos_a = self.player_positions[idx_a];
        let pos_b = self.player_positions[idx_b];
        pos_a.distance_to_m(&pos_b)
    }

    /// 태클 FSM 시작
    ///
    /// 조건 체크 후 TackleAction FSM을 active_tackles에 추가
    /// 태클 시작 (ActionQueue 버전, 2025-12-14 리팩토링)
    ///
    /// FSM active_tackles 대신 ActionQueue에 직접 schedule
    pub(crate) fn start_tackle_fsm(
        &mut self,
        tackler_idx: usize,
        target_idx: usize,
        _tackle_type: TackleType, // ActionQueue는 Standing 기본값 사용
    ) -> bool {
        use crate::engine::action_queue::ActionType;

        // 쿨다운 체크
        if self.tackle_cooldowns[tackler_idx] > 0 {
            return false;
        }

        // PlayerState 체크 - Idle 또는 Moving만 태클 가능
        if !self.player_states[tackler_idx].can_start_action() {
            return false;
        }

        // 이미 액션 중이면 무시 (ActionQueue 기반)
        if self.action_queue.is_player_active(tackler_idx) {
            return false;
        }

        // 거리 체크
        let dist = self.distance_between(tackler_idx, target_idx);
        if dist > TACKLE_MAX_DISTANCE {
            return false;
        }

        let _ = dist; // suppress unused warning

        // ActionQueue에 태클 예약 (즉시 실행 = current_tick)
        let _action_id = self.action_queue.schedule_new(
            self.current_tick,
            ActionType::Tackle { target_idx },
            tackler_idx,
            80, // 태클 우선순위
        );

        // NOTE: PlayerState::InAction은 activate_pending_actions → on_action_started에서 설정
        // 여기서 설정하면 다음 틱의 can_start_action()이 false를 반환하여 activation이 블록됨

        // Statistics: 태클 시도 기록
        self.record_tackle_attempt(tackler_idx);

        true
    }

    /// 태클 결과 처리 (Viewer 이벤트 생성 포함)
    fn handle_tackle_outcome_with_event(
        &mut self,
        tackler_idx: usize,
        target_idx: usize,
        outcome: TackleOutcome,
        // P0: Core types moved to action_queue
        tackle_type: crate::engine::action_queue::TackleType,
        contact_pos: (f32, f32),
        ball_pos: (f32, f32),
    ) {
        // ball_owner 정보 캡처 (변경 전)
        let ball_owner_before = self.ball.current_owner.map(|idx| idx as u32);

        // 기존 handle_tackle_outcome 로직 호출
        self.handle_tackle_outcome(tackler_idx, target_idx, outcome);

        // ball_owner 정보 캡처 (변경 후)
        let ball_owner_after = self.ball.current_owner.map(|idx| idx as u32);

        // P7: Viewer TackleEvent 생성 (P0: types moved to action_queue)
        let viewer_outcome =
            crate::engine::action_queue::ViewerTackleOutcome::from_tackle_outcome(outcome);
        let action = crate::engine::action_queue::TackleActionKind::from_tackle_type(tackle_type);
        let lock_ms = crate::engine::action_queue::TackleEvent::calculate_lock_ms(
            tackle_type,
            viewer_outcome,
        );

        let tackle_event = TackleEvent {
            t_ms: self.current_tick * 250,
            kind: "tackle",
            actor_track_id: tackler_idx as u32,
            target_track_id: target_idx as u32,
            action,
            lock_ms,
            outcome: viewer_outcome,
            ball_owner_before,
            ball_owner_after,
            contact_pos: Some(contact_pos),
            ball_pos: Some((ball_pos.0, ball_pos.1, self.ball.height_meters())),
        };

        // P7: TackleEvent → MatchResult.viewer_events에 저장
        #[cfg(debug_assertions)]
        log::trace!("TackleEvent generated: {:?}", tackle_event);
        self.result.add_viewer_event(ViewerEvent::Tackle(tackle_event));
    }

    /// 퇴장 처리 (레드카드)
    pub(crate) fn send_off_player(&mut self, player_idx: usize) {
        if player_idx >= self.player_states.len() {
            return;
        }

        self.player_states[player_idx] = PlayerState::SentOff;
        self.player_velocities[player_idx] = (0.0, 0.0);
        self.player_speeds[player_idx] = 0.0;
        self.action_queue.cancel_active_for_player(player_idx);
        self.action_queue.cancel_pending_for_player(player_idx);

        if self.ball.current_owner == Some(player_idx) {
            self.ball.current_owner = None;
        }
    }

    /// 태클 결과 처리
    fn handle_tackle_outcome(
        &mut self,
        tackler_idx: usize,
        target_idx: usize,
        outcome: TackleOutcome,
    ) {
        let is_home = TeamSide::is_home(tackler_idx);

        match outcome {
            TackleOutcome::CleanWin => {
                // 공 소유권 이전
                // FIX_2601/1120: Update ball position to tackler's position to prevent teleportation
                self.ball.current_owner = Some(tackler_idx);
                self.ball.position = self.player_positions[tackler_idx];

                // 이벤트 발생
                // C5+C6: Use constructor instead of manual construction
                let ball_pos_m = self.ball.position_meters();
                self.emit_event(
                    MatchEvent::tackle(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        tackler_idx,
                        (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(Some(target_idx)),
                );

                // 통계 업데이트
                if is_home {
                    self.result.statistics.tackles_home += 1;
                } else {
                    self.result.statistics.tackles_away += 1;
                }

                // PlayerState 리셋
                self.player_states[tackler_idx] = PlayerState::Recovering { remaining_ticks: 4 };
                self.player_states[target_idx] = PlayerState::Staggered { remaining_ticks: 8 };
            }

            TackleOutcome::Foul | TackleOutcome::YellowCard | TackleOutcome::RedCard => {
                // 공 Out of Play
                self.ball.current_owner = None;

                // 파울 이벤트
                // C5+C6: Use constructor instead of manual construction
                let ball_pos_m = self.ball.position_meters();
                self.emit_event(
                    MatchEvent::foul(
                        self.minute,
                        self.current_timestamp_ms(),
                        is_home,
                        tackler_idx,
                        (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters()),
                    )
                    .with_target_track_id(Some(target_idx)),
                );

                // 통계
                if is_home {
                    self.result.statistics.fouls_home += 1;
                } else {
                    self.result.statistics.fouls_away += 1;
                }

                // 카드 처리
                match outcome {
                    TackleOutcome::YellowCard => {
                        // C5+C6: Use constructor instead of manual construction
                        self.emit_event(
                            MatchEvent::yellow_card(
                                self.minute,
                                self.current_timestamp_ms(),
                                is_home,
                                tackler_idx,
                            )
                            .with_target_track_id(Some(target_idx))
                            .with_ball_position({
                                let ball_pos_m = self.ball.position_meters();
                                (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters())
                            }),
                        );

                        // FIX_2601/0123: Update momentum on yellow card
                        {
                            use super::momentum::events;
                            if is_home {
                                self.home_momentum.apply_event(events::YELLOW_CARD);
                            } else {
                                self.away_momentum.apply_event(events::YELLOW_CARD);
                            }
                        }
                    }
                    TackleOutcome::RedCard => {
                        // C5+C6: Use constructor instead of manual construction
                        self.emit_event(
                            MatchEvent::red_card(
                                self.minute,
                                self.current_timestamp_ms(),
                                is_home,
                                tackler_idx,
                            )
                            .with_target_track_id(Some(target_idx))
                            .with_ball_position({
                                let ball_pos_m = self.ball.position_meters();
                                (ball_pos_m.0, ball_pos_m.1, self.ball.height_meters())
                            }),
                        );
                        self.send_off_player(tackler_idx);

                        // FIX_2601/0123: Update momentum on red card
                        {
                            use super::momentum::events;
                            if is_home {
                                self.home_momentum.apply_event(events::RED_CARD);
                            } else {
                                self.away_momentum.apply_event(events::RED_CARD);
                            }
                        }
                    }
                    _ => {}
                }

                // PlayerState
                if !matches!(outcome, TackleOutcome::RedCard) {
                    self.player_states[tackler_idx] = PlayerState::Recovering { remaining_ticks: 16 };
                }
            }

            TackleOutcome::Miss | TackleOutcome::Deflection => {
                // 실패 또는 공만 건드림 - 루즈볼 상태
                // 태클러만 회복 상태
                self.player_states[tackler_idx] = PlayerState::Recovering { remaining_ticks: 12 };
            }
        }

        // 쿨다운 설정
        self.tackle_cooldowns[tackler_idx] = TACKLE_COOLDOWN_TICKS;
    }

    /// 수비수들의 태클 결정
    ///
    /// 공 소유자 근처의 수비수들이 태클을 시도할지 결정
    /// FIX_2601/0110: 2-phase update 패턴 적용 (index order bias 제거)
    pub(crate) fn decide_defender_tackles(&mut self) {
        // 공 소유자가 없으면 무시
        let Some(ball_owner) = self.ball.current_owner else { return };

        // 수비 팀 선수 범위
        let defender_range = TeamSide::opponent_range(ball_owner);

        // P0: Determine which team is defending and get their defensive tuning
        let is_home_defending = TeamSide::is_home(defender_range.start);
        let defensive_tuning = if is_home_defending {
            &self.home_defensive_tuning
        } else {
            &self.away_defensive_tuning
        };

        // P0: Adjust tackle distance based on pressing intensity
        // FIX_2601/1126: Increased base distance and range for better PPDA
        // VeryLow (0.2) → 2.0m (passive)
        // Medium (0.6) → 3.0m (default)
        // VeryHigh (1.0) → 4.4m (aggressive)
        let base_distance = 3.0;  // 1.5 → 3.0 (doubled for PPDA fix)
        let tackle_initiate_distance =
            base_distance + (defensive_tuning.pressing_factor - 0.6) * 3.5;  // 2.5 → 3.5

        // ========== Phase 1: Intent Collection ==========
        // 모든 후보자의 태클 의도를 먼저 수집 (아직 적용하지 않음)
        let mut tackle_candidates: Vec<(usize, f32)> = Vec::new(); // (defender_idx, tackle_chance)

        for defender_idx in defender_range {
            // 이미 태클 중이면 무시
            if self.has_active_tackle(defender_idx) {
                continue;
            }

            // 쿨다운 중이면 무시
            if self.tackle_cooldowns[defender_idx] > 0 {
                continue;
            }

            // PlayerState가 행동 가능한지
            if !self.player_states[defender_idx].can_start_action() {
                continue;
            }

            // 거리 체크
            let dist = self.distance_between(defender_idx, ball_owner);
            if dist > tackle_initiate_distance {
                continue;
            }

            // 태클 확률 결정 (거리에 반비례)
            // FIX_2601/1126: Tuned tackle probability for PPDA normalization
            // Original: 0.015 (1.5% max) → PPDA=24.2 (too passive)
            // First try: 0.05 (5% max) → PPDA=3.27 (too aggressive)
            // Target: 0.025 (2.5% max) → PPDA ~10-12
            let tackle_chance = (1.0 - dist / tackle_initiate_distance) * 0.025; // 최대 2.5% 확률

            tackle_candidates.push((defender_idx, tackle_chance));
        }

        // ========== Phase 2: Batch Resolution ==========
        // FIX_2601/0120: Actor-based RNG for order-independent tackle decisions
        // Each defender gets their own RNG seeded by (base_seed, tick, defender_idx)
        // This eliminates correlation between iteration order and RNG values
        let mut successful_tacklers: Vec<usize> = Vec::new();

        for (defender_idx, tackle_chance) in &tackle_candidates {
            // Actor-based seed: base_seed XOR (tick shifted) XOR (defender_idx shifted) XOR stage marker
            let actor_seed = self.original_seed
                ^ (self.current_tick << 16)
                ^ ((*defender_idx as u64) << 32)
                ^ (0x7AC << 48); // Stage marker for tackle decisions (0x7AC = "TAC")
            use rand::SeedableRng;
            let mut actor_rng = rand_chacha::ChaCha8Rng::seed_from_u64(actor_seed);
            let roll: f32 = actor_rng.gen();
            if roll < *tackle_chance {
                successful_tacklers.push(*defender_idx);
            }
        }

        // ========== Phase 3: Batch Commit ==========
        // 여러 명이 동시에 태클 성공하면 deterministic하게 선택
        // FIX_2601/0120: Use tick-based seed instead of self.rng for order independence
        if !successful_tacklers.is_empty() {
            let commit_seed = self.original_seed ^ (self.current_tick << 16) ^ (0xC0 << 48); // Stage marker for commit
            use rand::SeedableRng;
            let mut commit_rng = rand_chacha::ChaCha8Rng::seed_from_u64(commit_seed);
            let chosen_idx = commit_rng.gen_range(0..successful_tacklers.len());
            let tackler = successful_tacklers[chosen_idx];
            self.start_tackle_fsm(tackler, ball_owner, TackleType::Standing);
        }
    }

    // ========== P9: Set Piece FSM ==========

    /// 활성 세트피스 FSM 업데이트
    pub(crate) fn update_active_set_pieces(&mut self) {
        use crate::engine::phase_action::{SetPiecePhase, SetPieceResult};

        if self.active_set_pieces.is_empty() {
            return;
        }

        // borrow checker 문제 해결
        let mut active_set_pieces = std::mem::take(&mut self.active_set_pieces);
        let mut completed_indices = Vec::new();

        for (idx, set_piece) in active_set_pieces.iter_mut().enumerate() {
            // FSM 업데이트
            let result = set_piece.update_tick(self.current_tick, &mut self.rng);

            // 결과 처리
            match result {
                SetPieceResult::InProgress => {
                    // 계속 진행
                }
                SetPieceResult::Goal { scorer_idx, scorer_name: _, assist_idx } => {
                    // 골 처리
                    // C6: Use scorer_idx directly as track_id
                    self.score_goal(set_piece.is_home_attacking, scorer_idx);

                    // 어시스트 기록
                    if let Some(_assist) = assist_idx {
                        // TODO: 어시스트 통계 추가
                    }
                    completed_indices.push(idx);
                }
                SetPieceResult::ShotOnTarget { shooter_idx, xg } => {
                    // 온타겟 슛 기록
                    // C6: Use shooter_idx directly as track_id
                    self.emit_event(MatchEvent::shot(
                        self.minute,
                        self.current_timestamp_ms(),
                        set_piece.is_home_attacking,
                        shooter_idx,
                        true,
                        xg,
                    ));

                    // GK 세이브 처리 (간단 버전)
                    let gk_idx = if set_piece.is_home_attacking { 11 } else { 0 };
                    // C6: Use gk_idx directly as track_id
                    self.emit_event(
                        MatchEvent::save(
                            self.minute,
                            self.current_timestamp_ms(),
                            !set_piece.is_home_attacking,
                            gk_idx,
                        )
                        .with_target_track_id(Some(shooter_idx)),
                    );
                    // FIX_2601/1120: Update ball position to GK's position to prevent teleportation
                    self.ball.current_owner = Some(gk_idx);
                    self.ball.position = self.player_positions[gk_idx];

                    completed_indices.push(idx);
                }
                SetPieceResult::ShotOffTarget { shooter_idx, xg } => {
                    // 오프타겟 슛 기록
                    // C6: Use shooter_idx directly as track_id
                    self.emit_event(MatchEvent::shot(
                        self.minute,
                        self.current_timestamp_ms(),
                        set_piece.is_home_attacking,
                        shooter_idx,
                        false,
                        xg,
                    ));
                    self.ball.current_owner = None;
                    completed_indices.push(idx);
                }
                SetPieceResult::Save { gk_idx, shooter_idx } => {
                    // C6: Use gk_idx directly as track_id
                    self.emit_event(
                        MatchEvent::save(
                            self.minute,
                            self.current_timestamp_ms(),
                            !set_piece.is_home_attacking,
                            gk_idx,
                        )
                        .with_target_track_id(Some(shooter_idx)),
                    );
                    // FIX_2601/1120: Update ball position to GK's position to prevent teleportation
                    self.ball.current_owner = Some(gk_idx);
                    self.ball.position = self.player_positions[gk_idx];
                    completed_indices.push(idx);
                }
                SetPieceResult::Cleared { defender_idx } => {
                    // 수비 클리어
                    // FIX_2601/1120: Update ball position to defender's position to prevent teleportation
                    self.ball.current_owner = Some(defender_idx);
                    self.ball.position = self.player_positions[defender_idx];
                    completed_indices.push(idx);
                }
                SetPieceResult::AttackRetain { receiver_idx } => {
                    // 공격팀 유지
                    // FIX_2601/1120: Update ball position to receiver's position to prevent teleportation
                    self.ball.current_owner = Some(receiver_idx);
                    self.ball.position = self.player_positions[receiver_idx];
                    completed_indices.push(idx);
                }
                SetPieceResult::DefenseWin { receiver_idx } => {
                    // 수비팀 전환
                    // FIX_2601/1120: Update ball position to receiver's position to prevent teleportation
                    self.ball.current_owner = Some(receiver_idx);
                    self.ball.position = self.player_positions[receiver_idx];
                    completed_indices.push(idx);
                }
                SetPieceResult::OutOfPlay => {
                    self.ball.current_owner = None;
                    completed_indices.push(idx);
                }
            }

            // 완료 체크
            if set_piece.phase == SetPiecePhase::Finished && !completed_indices.contains(&idx) {
                completed_indices.push(idx);
            }
        }

        // 완료된 FSM 제거 (역순)
        for idx in completed_indices.into_iter().rev() {
            active_set_pieces.swap_remove(idx);
        }

        self.active_set_pieces = active_set_pieces;
    }

    /// 세트피스 FSM이 활성화된 상태인지 확인
    pub(crate) fn has_active_set_piece(&self) -> bool {
        !self.active_set_pieces.is_empty()
    }

    /// P9: 코너킥 FSM 시작
    ///
    /// 기존 `execute_corner_kick()`을 FSM 기반으로 대체
    pub(crate) fn start_corner_kick_fsm(&mut self, is_home_attacking: bool) {
        use crate::engine::coordinates;
        use crate::engine::phase_action::{
            AerialDefender, AerialTarget, CornerKickContext, CornerTactic, SetPieceAction,
        };
        use crate::models::trait_system::TraitId;

        // 1. 코너킥 키커 선택 (corners + crossing 스킬 기준)
        // FIX_2601/0123: corners 속성을 주요 기준으로 사용, crossing은 보조
        let (start_idx, end_idx) = if is_home_attacking { (0, 11) } else { (11, 22) };
        let mut best_kicker = start_idx;
        let mut best_corner_score = 0.0f32;

        for idx in start_idx..end_idx {
            let corners = self.get_player_corners(idx);
            let crossing = self.get_player_crossing(idx);
            // corners가 주요 기준 (70%), crossing이 보조 (30%)
            let score = corners * 0.7 + crossing * 0.3;
            if score > best_corner_score {
                best_corner_score = score;
                best_kicker = idx;
            }
        }

        // 2. 코너킥 이벤트 발생
        // C6: Use best_kicker directly as track_id
        self.emit_event(MatchEvent::corner(
            self.minute,
            self.current_timestamp_ms(),
            is_home_attacking,
            best_kicker,
        ));

        // For CornerKickContext (not Event SSOT)
        let kicker_name = self.get_player_name(best_kicker);

        // 3. 공 위치를 코너로 이동
        // FIX_2601/0116: Use attacks_right for corner position (not is_home_attacking)
        // Corner is at opponent's goal line: if attacking right, corner at x=1.0
        let ctx = team_view_context(self, is_home_attacking);
        let attacks_right = ctx.attacks_right;
        let is_left_corner = self.rng.gen_bool(0.5);
        let corner_tv = TeamViewCoord10 {
            x: Coord10::FIELD_LENGTH_10,
            y: if is_left_corner { 0 } else { Coord10::FIELD_WIDTH_10 },
        };
        self.ball.position = ctx.to_world(corner_tv);
        self.ball.current_owner = Some(best_kicker);

        // 4. 공중볼 타겟 선정 (공격수 중 heading/jumping 좋은 선수들)
        let mut header_targets = Vec::new();
        for idx in start_idx..end_idx {
            if idx == best_kicker || idx == 0 || idx == 11 {
                continue; // 키커, GK 제외
            }

            let heading = self.get_player_heading(idx) as u8;
            let jumping = self.get_player_jumping(idx) as u8;
            let strength = self.get_player_strength(idx) as u8;
            let pos = self.player_positions[idx];
            // FIX_2601: Coord10 → normalized로 변환
            let pos_norm = pos.to_normalized_legacy();

            // 박스 안에 있거나 헤딩 스킬이 좋은 선수만
            // FIX_2601/0116: Use attacks_right for in_box check (not is_home_attacking)
            let pos_tv = coordinates::to_team_view_normalized(pos_norm, attacks_right);
            let in_box = pos_tv.1 > 0.83; // length > 87m (골라인에서 18m 이내)

            if in_box || heading > 12 {
                let name = self.get_player_name(idx);
                let dist = coordinates::distance_to_goal_m(pos_norm, attacks_right);
                header_targets.push(AerialTarget {
                    idx,
                    name,
                    heading,
                    jumping,
                    strength,
                    bravery: self.get_player_composure(idx) as u8, // use composure as bravery proxy
                    positioning: self.get_player_anticipation(idx) as u8,
                    position: pos_norm, // FIX_2601: normalized tuple
                    distance_to_goal: dist,
                    has_airraid_gold: self.player_has_gold_trait(idx, TraitId::AirRaid),
                });
            }
        }

        // 5. 수비수 선정 (상대팀)
        let (def_start, def_end) = if is_home_attacking { (11, 22) } else { (0, 11) };
        let mut defenders = Vec::new();
        for idx in def_start..def_end {
            let heading = self.get_player_heading(idx) as u8;
            let jumping = self.get_player_jumping(idx) as u8;
            let strength = self.get_player_strength(idx) as u8;
            let pos = self.player_positions[idx];
            // FIX_2601: Coord10 → normalized
            let pos_norm = pos.to_normalized_legacy();

            // 박스 안에 있는 수비수만
            // FIX_2601/0116: Use attacks_right for in_box check (not is_home_attacking)
            let pos_tv = coordinates::to_team_view_normalized(pos_norm, attacks_right);
            let in_box = pos_tv.1 > 0.83;

            if in_box || idx == def_start {
                // GK도 포함
                defenders.push(AerialDefender {
                    idx,
                    heading,
                    jumping,
                    strength,
                    bravery: self.get_player_composure(idx) as u8,
                    positioning: self.get_player_anticipation(idx) as u8,
                    marking: self.get_player_tackling(idx) as u8, // use tackling as marking proxy
                    marking_target: None,
                    position: pos_norm, // FIX_2601: normalized tuple
                });
            }
        }

        // 6. Context 생성
        // FIX_2601/0123: corners 속성 추가
        let ctx = CornerKickContext {
            is_home_attacking,
            kicker_idx: best_kicker,
            kicker_name,
            is_left_corner,
            tactic: CornerTactic::Inswing, // TODO: 전술 설정에서 가져오기
            corners: self.get_player_corners(best_kicker) as u8,  // FIX_2601/0123: 코너킥 전용 속성
            crossing: self.get_player_crossing(best_kicker) as u8,
            technique: self.get_player_technique(best_kicker) as u8,
            vision: self.get_player_vision(best_kicker) as u8,
            curve: self.get_player_technique(best_kicker) as u8, // use technique as curve proxy
            header_targets,
            defenders,
        };

        // 7. FSM 생성 및 등록
        let id = self.current_tick * 1000 + self.active_set_pieces.len() as u64;
        let action = SetPieceAction::new_corner(id, self.current_tick, ctx);

        self.active_set_pieces.push(action);

        // Statistics: 코너킥 기록
        self.record_corner(is_home_attacking);
    }

    /// P9: 프리킥 FSM 시작
    ///
    /// 기존 `execute_free_kick()`을 FSM 기반으로 대체
    pub(crate) fn start_free_kick_fsm(
        &mut self,
        position: (f32, f32),
        is_home_attacking: bool,
        is_indirect: bool,
    ) {
        use crate::engine::coordinates;
        use crate::engine::phase_action::{FreeKickContext, FreeKickTactic, SetPieceAction};
        use crate::models::trait_system::TraitId;

        // 1. 프리킥 키커 선택 (free_kicks 스킬 기준)
        let (start_idx, end_idx) = if is_home_attacking { (0, 11) } else { (11, 22) };
        let mut best_kicker = start_idx;
        let mut best_freekick = 0u8;

        for idx in start_idx..end_idx {
            if let Some(player) = self.get_player(idx) {
                let attrs = &player.attributes;
                if attrs.free_kicks > best_freekick {
                    best_freekick = attrs.free_kicks;
                    best_kicker = idx;
                }
            }
        }

        // 2. 거리 계산
        // FIX_2601/0116: Use attacks_right for distance calculation (not is_home_attacking)
        let attacks_right = self.attacks_right(is_home_attacking);
        let distance_to_goal = coordinates::distance_to_goal_m(position, attacks_right);
        let can_shoot_direct = !is_indirect && distance_to_goal <= 30.0;

        // 3. 프리킥 이벤트 발생
        // C6: Use best_kicker directly as track_id
        self.emit_event(MatchEvent::freekick(
            self.minute,
            self.current_timestamp_ms(),
            is_home_attacking,
            best_kicker,
            (position.0, position.1, 0.0),
        ));

        // 4. 공 위치 설정
        // FIX_2601/0104: position is in NormalizedPos (width, length) format
        // Use from_normalized_legacy() which properly swaps axes
        self.ball.position = Coord10::from_normalized_legacy(position);
        self.ball.current_owner = Some(best_kicker);

        // 5. 택틱 결정: 직접슛 가능 거리면 직접슛, 아니면 크로스
        let tactic = if is_indirect {
            if self.rng.gen::<f32>() < 0.6 {
                FreeKickTactic::ShortPass
            } else {
                FreeKickTactic::Cross
            }
        } else if can_shoot_direct && self.rng.gen::<f32>() < 0.6 {
            FreeKickTactic::DirectShot
        } else {
            FreeKickTactic::Cross
        };

        // 6. Context 생성
        let gk_idx = if is_home_attacking { 11 } else { 0 };
        let _gk_name = self.get_player_name(gk_idx);

        // For FreeKickContext (not Event SSOT)
        let kicker_name = self.get_player_name(best_kicker);

        let ctx = FreeKickContext {
            is_home_attacking,
            kicker_idx: best_kicker,
            kicker_name,
            position,
            distance_to_goal,
            can_shoot_direct,
            tactic,
            free_kicks: best_freekick,
            long_shots: self.get_player_long_shots(best_kicker) as u8,
            technique: self.get_player_technique(best_kicker) as u8,
            curve: self.get_player_technique(best_kicker) as u8, // technique as curve proxy
            shot_power: self.get_player_long_shots(best_kicker) as u8, // long_shots as power proxy
            composure: self.get_player_composure(best_kicker) as u8,
            has_deadball_gold: self.player_has_gold_trait(best_kicker, TraitId::DeadBall),
            gk_idx,
            gk_reflexes: self.get_player_gk_reflexes(gk_idx) as u8,
            gk_positioning: self.get_player_positioning(gk_idx) as u8,
        };

        // 7. FSM 생성 및 등록
        let id = self.current_tick * 1000 + self.active_set_pieces.len() as u64;
        let is_direct = !is_indirect;
        let action = SetPieceAction::new_freekick(id, self.current_tick, ctx, is_direct);

        self.active_set_pieces.push(action);

        // Statistics: 프리킥 기록
        self.record_freekick(is_home_attacking);
    }

    /// P9: 페널티킥 FSM 시작
    ///
    /// 기존 `execute_penalty_kick()`을 FSM 기반으로 대체
    pub(crate) fn start_penalty_kick_fsm(&mut self, is_home_attacking: bool) {  
        use crate::engine::phase_action::{PenaltyContext, SetPieceAction};      
        use crate::models::trait_system::TraitId;

        // 1. 페널티 키커 선택 (penalty_taking 스킬 기준)
        let (start_idx, end_idx) = if is_home_attacking { (0, 11) } else { (11, 22) };
        let mut best_kicker = start_idx;
        let mut best_penalty = 0.0f32;

        for idx in start_idx..end_idx {
            let penalty = self.get_player_penalty_taking(idx);
            if penalty > best_penalty {
                best_penalty = penalty;
                best_kicker = idx;
            }
        }

        // 2. 페널티 이벤트 발생
        // C6: Use best_kicker directly as track_id
        self.emit_event(MatchEvent::penalty(
            self.minute,
            self.current_timestamp_ms(),
            is_home_attacking,
            best_kicker,
        ));

        if is_home_attacking {
            self.result.statistics.penalties_home += 1;
        } else {
            self.result.statistics.penalties_away += 1;
        }

        // 3. 공 위치를 페널티 스팟으로 이동
        // FIX_2601/0116: Use attacks_right for penalty spot position (not is_home_attacking)
        let ctx = team_view_context(self, is_home_attacking);
        let penalty_x_tv =
            Coord10::FIELD_LENGTH_10 - (field::PENALTY_SPOT_M * Coord10::SCALE).round() as i32;
        let penalty_tv = TeamViewCoord10 { x: penalty_x_tv, y: Coord10::CENTER_Y };
        self.ball.position = ctx.to_world(penalty_tv);
        self.ball.current_owner = Some(best_kicker);

        // 4. GK 정보
        let gk_idx = if is_home_attacking { 11 } else { 0 };
        let gk_name = self.get_player_name(gk_idx);

        // For PenaltyContext (not Event SSOT)
        let kicker_name = self.get_player_name(best_kicker);

        // 5. Context 생성
        let ctx = PenaltyContext {
            is_home_attacking,
            kicker_idx: best_kicker,
            kicker_name,
            penalty_taking: best_penalty as u8,
            composure: self.get_player_composure(best_kicker) as u8,
            finishing: self.get_player_finishing(best_kicker) as u8,
            technique: self.get_player_technique(best_kicker) as u8,
            has_deadball_gold: self.player_has_gold_trait(best_kicker, TraitId::DeadBall),
            gk_idx,
            gk_name,
            gk_reflexes: self.get_player_gk_reflexes(gk_idx) as u8,
            gk_diving: self.get_player_agility(gk_idx) as u8, // No dedicated diving attr, use agility
            gk_anticipation: self.get_player_gk_one_on_ones(gk_idx) as u8, // one_on_ones for penalty situations
        };

        // 6. FSM 생성 및 등록
        let id = self.current_tick * 1000 + self.active_set_pieces.len() as u64;
        let action = SetPieceAction::new_penalty(id, self.current_tick, ctx);

        self.active_set_pieces.push(action);
    }

    /// 미터 델타를 정규화 델타로 변환
    fn meters_to_normalized_delta(&self, delta_m: (f32, f32)) -> (f32, f32) {
        (delta_m.0 / field::LENGTH_M, delta_m.1 / field::WIDTH_M)
    }

    /// GK의 reflexes 능력치 가져오기
    /// v5 캐시: 실제 gk_reflexes 사용, 없으면 다른 속성에서 유도
    fn get_player_reflexes(&self, player_idx: usize) -> u8 {
        if player_idx >= 22 {
            return 50;
        }

        let a = self.get_player_attributes(player_idx);

        // v5: Use real GK reflexes if available (non-zero)
        if a.gk_reflexes > 0 {
            a.gk_reflexes
        } else {
            // Fallback: derive from (anticipation + agility + concentration) / 3
            let sum = a.anticipation as u16 + a.agility as u16 + a.concentration as u16;
            (sum / 3) as u8
        }
    }

    /// GK의 handling 능력치 가져오기
    /// v5 캐시: 실제 gk_handling 사용, 없으면 다른 속성에서 유도
    fn get_player_handling(&self, player_idx: usize) -> u8 {
        if player_idx >= 22 {
            return 50;
        }

        let a = self.get_player_attributes(player_idx);

        // v5: Use real GK handling if available (non-zero)
        if a.gk_handling > 0 {
            a.gk_handling
        } else {
            // Fallback: derive from (first_touch + composure + concentration) / 3
            let sum = a.first_touch as u16 + a.composure as u16 + a.concentration as u16;
            (sum / 3) as u8
        }
    }

    // ========== P10-13: Stamina System ==========

    /// 선수의 stamina 능력치 가져오기 (0-100 범위, 없으면 overall로 대체)
    fn get_player_stamina_attr(&self, player_idx: usize) -> u8 {
        if player_idx >= 22 {
            return 50; // fallback
        }

        self.get_player_attributes(player_idx).stamina
    }

    /// 선수의 pace 능력치 가져오기 (0-100 스케일)
    /// Note: FM2023 1-20 → 0-100 변환은 fm_to_match_engine()에서 수행됨
    fn get_player_pace_attr(&self, player_idx: usize) -> u8 {
        if player_idx >= 22 {
            return 50; // fallback (average pace, 0-100 scale)
        }

        // PlayerAttributes.pace는 이미 0-100 스케일 (FM 1-20이 변환됨)
        self.get_player_attributes(player_idx).pace
    }

    /// 매 틱 호출: 모든 선수의 스태미나 감소/회복
    /// FIX_2601/0106 P5: 선수 스태미나 속성 활용 (회복률 적용)
    fn decay_stamina_tick(&mut self) {
        // FIX_2601/0106 P5: 휴식 시 회복 상수
        const BASE_RECOVERY_RATE: f32 = 0.0003; // 기본 회복률 (per tick)
        const REST_DECAY_MULT: f32 = 0.2; // 휴식 중 감소율 (걷기 = 20%)

        for i in 0..22 {
            let stamina_attr = self.get_player_stamina_attr(i) as f32 / 100.0; // 0~1 범위
            let condition_mult = crate::fix01::condition_drain_mult(
                self.setup.get_player(i).condition_level,
            );

            // P0-S1: Get tempo multiplier from team instructions
            let is_home = i < 11;
            let tempo_mult = if is_home {
                self.home_instructions.team_tempo.stamina_drain_modifier()
            } else {
                self.away_instructions.team_tempo.stamina_drain_modifier()
            };

            // FIX_2601/0106 P5: 휴식 중이면 회복 로직 적용
            if self.player_resting[i] {
                // 휴식 중: 감소율 대폭 감소 + 회복
                // 걷기 감소 (기본 감소의 20%)
                let walk_decay =
                    0.00002 * (1.5 - stamina_attr) * tempo_mult * REST_DECAY_MULT * condition_mult;

                // 회복: stamina 높을수록 빠르게 (0.5 + stamina_attr * 0.5)
                // stamina 100 = 1.0x, stamina 0 = 0.5x
                let recovery_mult = 0.5 + stamina_attr * 0.5;
                let recovery = BASE_RECOVERY_RATE * recovery_mult;

                // 순 변화 = 회복 - 걷기 감소
                let net_change = recovery - walk_decay;
                self.stamina[i] = (self.stamina[i] + net_change).clamp(0.0, 1.0);
            } else {
                // 활동 중: 기존 감소 로직
                // 기본 감소 (서있기만 해도) - stamina 높으면 덜 감소
                // P0-S1: Apply tempo multiplier (VeryFast=1.4x, Normal=1.0x, VerySlow=0.7x)
                let base_decay = 0.00002 * (1.5 - stamina_attr) * tempo_mult * condition_mult;

                // 스프린트 감소
                // P0-S1: Apply tempo multiplier to sprint cost as well
                let sprint_decay = if self.sprint_state[i] {
                    0.0004 * (1.5 - stamina_attr) * tempo_mult * condition_mult
                } else {
                    0.0
                };

                // 총 감소
                self.stamina[i] = (self.stamina[i] - base_decay - sprint_decay).max(0.0);
            }
        }
    }

    /// 선수가 스프린트 중인지 감지 (속도 기반)
    fn update_sprint_state(&mut self, player_idx: usize) {
        // player_speeds 배열 사용 (현재 속도 m/s)
        let current_speed = self.player_speeds[player_idx];

        // FIX: player_physics.rs와 동일한 공식 사용
        // MAX_SPEED_BASE=7.0, MAX_SPEED_RANGE=2.5
        // pace 0-100 (0-100 스케일) → pace_n 0-1 → max_speed 7.0-9.5 m/s
        let pace_attr = self.get_player_pace_attr(player_idx) as f32;
        let pace_normalized = pace_attr / 100.0; // 0-100 → 0-1
        let max_speed = 7.0 + pace_normalized * 2.5; // pace 100 = 9.5 m/s

        // 최대 속도의 40% 이상이면 스프린트
        // Note: 포지셔닝 엔진이 선수를 조깅 속도(~50%)로 제한하므로
        //       40%로 낮춰서 빠른 이동 시 스프린트로 감지
        // pace 100 → max_speed 9.5 m/s → threshold 3.8 m/s
        // pace 50 → max_speed 8.25 m/s → threshold 3.3 m/s
        self.sprint_state[player_idx] = current_speed > max_speed * 0.4;
    }

    /// 모든 선수의 스프린트 상태 업데이트
    fn update_all_sprint_states(&mut self) {
        for i in 0..22 {
            self.update_sprint_state(i);
        }
    }

    /// 액션 실행 시 스태미나 비용 적용
    pub(crate) fn apply_action_stamina_cost(&mut self, player_idx: usize, action_type: &str) {
        let stamina_attr = self.get_player_stamina_attr(player_idx) as f32 / 100.0;
        let condition_mult = crate::fix01::condition_drain_mult(
            self.setup.get_player(player_idx).condition_level,
        );

        // stamina 높은 선수는 비용 감소 (1.3 - stamina_attr)
        let cost_mult = 1.3 - stamina_attr;

        let base_cost = match action_type {
            "shot" => 0.008,
            "tackle" => 0.012,
            "header" => 0.010,
            "dribble" => 0.003,
            "pass" => 0.001,
            "intercept" => 0.006,
            "save" => 0.015, // 골키퍼 다이빙
            _ => 0.0,
        };

        // P0-S2: Apply pressing cost multiplier to defensive actions
        let is_defensive_action = matches!(action_type, "tackle" | "intercept");
        let pressing_mult = if is_defensive_action {
            let is_home = player_idx < 11;
            if is_home {
                self.home_instructions.pressing_intensity.stamina_cost_modifier()
            } else {
                self.away_instructions.pressing_intensity.stamina_cost_modifier()
            }
        } else {
            1.0 // No pressing cost for non-defensive actions
        };

        let stamina_drain_mult = if player_idx < 11 {
            self.home_match_modifiers.stamina_drain_mult
        } else {
            self.away_match_modifiers.stamina_drain_mult
        };
        let cost = base_cost * cost_mult * pressing_mult * stamina_drain_mult * condition_mult;
        self.stamina[player_idx] = (self.stamina[player_idx] - cost).max(0.0);  
    }

    /// ExecutionError용 fatigue factor 반환 (0.0 = 컨디션 좋음, 1.0 = 지침)
    pub fn get_fatigue_normalized(&self, player_idx: usize) -> f32 {
        if player_idx >= 22 {
            return 0.0;
        }
        1.0 - self.stamina[player_idx]
    }

    // ========== Duel System Integration ==========

    /// 공격수에게 가장 가까운 수비수 찾기
    fn find_nearest_defender(&self, attacker_idx: usize) -> Option<usize> {
        let is_home = TeamSide::is_home(attacker_idx);
        let defender_range = TeamSide::opponent_range_for_home(is_home);

        let attacker_pos = self.player_positions[attacker_idx];
        let mut nearest: Option<(usize, f32)> = None;

        for def_idx in defender_range {
            // GK 제외 (0, 11)
            if def_idx == 0 || def_idx == 11 {
                continue;
            }

            // FIX_2601: Coord10.distance_to_m() 사용
            let dist = attacker_pos.distance_to_m(&self.player_positions[def_idx]);
            match nearest {
                None => nearest = Some((def_idx, dist)),
                Some((_, best_dist)) if dist < best_dist => {
                    nearest = Some((def_idx, dist));
                }
                _ => {}
            }
        }

        nearest.map(|(idx, _)| idx)
    }

    /// 수비수 뒤에 커버가 있는지 확인
    fn check_has_cover(&self, defender_idx: usize, attacker_pos: (f32, f32)) -> bool {
        let is_home_defender = TeamSide::is_home(defender_idx);
        let defender_range = TeamSide::teammate_range_for_home(is_home_defender);

        let defender_pos = self.player_positions[defender_idx];

        // 수비수와 골대 사이에 다른 수비수가 있는지 확인
        for other_idx in defender_range {
            if other_idx == defender_idx || other_idx == 0 || other_idx == 11 {
                continue; // 자기 자신과 GK 제외
            }

            let other_pos = self.player_positions[other_idx];

            // 수비수보다 골대 쪽에 있고, 공격수-골대 라인 근처에 있으면 커버
            // 좌표계: pos.0 = width, pos.1 = length (골 방향)
            // Home defender guards x=0, Away defender guards x=1 (normalized length)
            let toward_goal = if is_home_defender {
                coordinates::norm_length(other_pos.to_normalized_legacy())
                    < coordinates::norm_length(defender_pos.to_normalized_legacy())
            } else {
                coordinates::norm_length(other_pos.to_normalized_legacy())
                    > coordinates::norm_length(defender_pos.to_normalized_legacy())
            };

            if toward_goal {
                // 간단한 커버 체크: width 거리가 5m 이내
                // FIX_2601: attacker_pos는 (f32, f32), other_pos는 Coord10
                let width_dist = (coordinates::norm_width(other_pos.to_normalized_legacy())
                    - coordinates::norm_width(attacker_pos))
                .abs();
                if width_dist < 0.07 {
                    // ~5m in normalized
                    return true;
                }
            }
        }

        false
    }

    /// 골대까지의 거리 계산 (미터 단위)
    /// FIX_2601: Use Coord10 directly
    /// FIX_2601/0109: Use attacks_right for correct second-half goal calculation
    fn distance_to_goal(&self, player_idx: usize) -> f32 {
        let is_home = TeamSide::is_home(player_idx);
        let ctx = team_view_context(self, is_home);
        let pos = self.player_positions[player_idx];

        // Goal position in Coord10 (0.1m units), TeamView semantics.
        let goal_pos = ctx.to_world(TeamViewCoord10::OPPONENT_GOAL);

        pos.distance_to_m(&goal_pos)
    }

    /// 선수의 속성값 가져오기
    fn get_player_attr(&self, player_idx: usize, attr_name: &str) -> u8 {
        if player_idx >= 22 {
            return 50;
        }

        let player = self.get_match_player(player_idx);
        let a = self.get_player_attributes(player_idx);

        match attr_name {
            "aggression" => a.aggression,
            "composure" => a.composure,
            "dribbling" => a.dribbling,
            "flair" => a.flair,
            "agility" => a.agility,
            "anticipation" => a.anticipation,
            "tackling" => a.tackling,
            _ => player.overall,
        }
    }

    /// TakeOn 시도: Duel 시스템을 사용한 돌파 처리
    fn resolve_takeon_duel(&mut self, attacker_idx: usize) -> (ActionType, Option<usize>) {
        let is_home = TeamSide::is_home(attacker_idx);
        self.record_take_on_attempt(attacker_idx);
        // FIX_2601: Use attacks_right for correct halftime handling
        let attacks_right = self.attacks_right(is_home);
        let attacker_pos = self.player_positions[attacker_idx];

        // 1. 가장 가까운 수비수 찾기
        let Some(defender_idx) = self.find_nearest_defender(attacker_idx) else {
            // 수비수가 없으면 그냥 드리블
            let dir_x = world_forward_dir_x(attacks_right);
            return (ActionType::Dribble { direction: (dir_x, 0.0), aggressive: true }, None);
        };

        // FIX_2601: Coord10.distance_to_m() 사용
        let defender_dist_m = attacker_pos.distance_to_m(&self.player_positions[defender_idx]);

        // 2. 수비수가 너무 멀면 그냥 드리블 (3m 이상)
        if defender_dist_m > 3.0 {
            // ~3m in normalized
            let dir_x = world_forward_dir_x(attacks_right);
            return (
                ActionType::Dribble {
                    direction: (dir_x, self.rng.gen_range(-0.2..0.2)),   
                    aggressive: true,
                },
                None,
            );
        }

        // 3. 수비수 정보 수집
        let dist_to_goal = self.distance_to_goal(defender_idx);
        // FIX_2601: attacker_pos를 normalized로 변환
        let has_cover = self.check_has_cover(defender_idx, attacker_pos.to_normalized_legacy());
        let bad_touch = self.rng.gen::<f32>() < 0.1; // 10% 확률로 배드터치

        let def_aggression = self.get_player_attr(defender_idx, "aggression");
        let def_composure = self.get_player_attr(defender_idx, "composure");

        // 4. 수비수 행동 결정 (Contain vs Commit)
        // 0108: Stamina-Aware Defense - 스태미나 30% 이하면 Contain 강제
        let def_stamina_pct = self.stamina[defender_idx];
        let def_action = decide_defensive_action(
            dist_to_goal,
            has_cover,
            bad_touch,
            def_aggression,
            def_composure,
            0.0,             // team_press_bonus (TODO: 전술 연동)
            def_stamina_pct, // 0108: Stamina-Aware Defense
        );

        // 5. 공격수/수비수 능력치 기반 성공률 계산
        let att_dribbling = self.get_player_attr(attacker_idx, "dribbling") as f32;
        let att_flair = self.get_player_attr(attacker_idx, "flair") as f32;
        let att_agility = self.get_player_attr(attacker_idx, "agility") as f32;

        let def_tackling = self.get_player_attr(defender_idx, "tackling") as f32;
        let def_anticipation = self.get_player_attr(defender_idx, "anticipation") as f32;

        // 역동작 계산을 위한 간단한 시뮬레이션
        let feint_success = (att_flair + self.rng.gen_range(0.0..20.0))
            > (def_anticipation + self.rng.gen_range(0.0..10.0));

        let wrong_foot_factor = if feint_success {
            -0.7 - self.rng.gen::<f32>() * 0.3 // -0.7 ~ -1.0 (수비수 역동작)
        } else {
            0.3 + self.rng.gen::<f32>() * 0.4 // 0.3 ~ 0.7 (수비수가 안 속음)
        };

        // 6. Duel 결과 계산
        let dribble_roll = (att_dribbling + att_agility) / 200.0 + self.rng.gen::<f32>() * 0.3;
        let tackle_roll = def_tackling / 100.0 + self.rng.gen::<f32>() * 0.3;

        let commit_level = if def_action == DefensiveAction::Commit { 0.8 } else { 0.2 };

        let outcome = resolve_duel(
            def_action,
            AttackerAction::TakeOn,
            wrong_foot_factor,
            commit_level,
            tackle_roll,
            dribble_roll,
        );

        // 7. 결과에 따른 ActionType 및 스턴 적용
        // FIX_2601: Use attacks_right (already computed above) for correct halftime handling
        let dir_x = world_forward_dir_x(attacks_right);

        match outcome {
            DuelOutcome::AnkleBreaker { stun_ticks } => {
                // 대성공: 수비수 스턴
                self.player_states[defender_idx] =
                    PlayerState::Recovering { remaining_ticks: stun_ticks };
                self.record_take_on_success(attacker_idx);
                (
                    ActionType::Dribble {
                        direction: (dir_x, self.rng.gen_range(-0.3..0.3)),
                        aggressive: true,
                    },
                    Some(defender_idx),
                )
            }
            DuelOutcome::Beaten { recovery_ticks } => {
                // 성공: 수비수 살짝 제침
                self.player_states[defender_idx] =
                    PlayerState::Recovering { remaining_ticks: recovery_ticks };
                self.record_take_on_success(attacker_idx);
                (
                    ActionType::Dribble {
                        direction: (dir_x, self.rng.gen_range(-0.2..0.2)),
                        aggressive: true,
                    },
                    Some(defender_idx),
                )
            }
            DuelOutcome::Stalemate => {
                // 교착: 그냥 드리블 유지
                (
                    ActionType::Dribble {
                        direction: (dir_x * 0.5, self.rng.gen_range(-0.1..0.1)),
                        aggressive: false,
                    },
                    None,
                )
            }
            DuelOutcome::LooseBall => {
                // 50:50 볼: 인터셉트 액션으로 전환 (루즈볼)
                self.ball.current_owner = None;
                (ActionType::Intercept { ball_position: self.ball.position }, None)
            }
            DuelOutcome::AttackerBlocked => {
                // 막힘 (Contain vs TakeOn): 수비수가 따라오며 막음
                (
                    ActionType::Dribble {
                        direction: (dir_x * 0.3, self.rng.gen_range(-0.1..0.1)),
                        aggressive: false,
                    },
                    None,
                )
            }
            DuelOutcome::DefenderWins { easy: _ } => {
                // 수비수 승리: 공 뺏김
                // FIX_2601/1120: Update ball position to defender's position to prevent teleportation
                self.ball.current_owner = Some(defender_idx);
                self.ball.position = self.player_positions[defender_idx];
                (ActionType::Dribble { direction: (-dir_x, 0.0), aggressive: false }, None)
            }
            DuelOutcome::Foul => {
                // 파울 처리
                self.result.statistics.fouls_home +=
                    if TeamSide::is_home(defender_idx) { 1 } else { 0 };
                self.result.statistics.fouls_away +=
                    if !TeamSide::is_home(defender_idx) { 1 } else { 0 };
                self.ball.current_owner = None;
                (ActionType::Intercept { ball_position: self.ball.position }, None)
            }
        }
    }

    // ========== Statistics Collection Methods ==========

    fn record_tick_telemetry(&mut self) {
        self.result.statistics.total_ticks = self.result.statistics.total_ticks.saturating_add(1);
        let in_play = !matches!(self.action_queue.ball_state(), BallState::OutOfPlay { .. });
        if in_play {
            self.result.statistics.ball_in_play_ticks =
                self.result.statistics.ball_in_play_ticks.saturating_add(1);
        }
        if self.ball.is_in_flight {
            self.result.statistics.ball_in_flight_ticks =
                self.result.statistics.ball_in_flight_ticks.saturating_add(1);
        }
    }

    fn record_possession_change(&mut self, home_has_ball: bool) {
        if home_has_ball {
            self.result.statistics.possessions_home =
                self.result.statistics.possessions_home.saturating_add(1);
        } else {
            self.result.statistics.possessions_away =
                self.result.statistics.possessions_away.saturating_add(1);
        }
    }

    fn record_action_result_metrics(&mut self, result: &ActionResult) {
        match result {
            ActionResult::Cancelled { .. }
            | ActionResult::OutOfBounds { .. }
            | ActionResult::GoalScored { .. } => {}
            _ => {
                self.result.statistics.actions_total =
                    self.result.statistics.actions_total.saturating_add(1);
            }
        }
    }

    fn record_hold_action(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.hold_actions_home =
                self.result.statistics.hold_actions_home.saturating_add(1);
        } else {
            self.result.statistics.hold_actions_away =
                self.result.statistics.hold_actions_away.saturating_add(1);
        }
    }

    fn record_carry_action(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.carry_actions_home =
                self.result.statistics.carry_actions_home.saturating_add(1);
        } else {
            self.result.statistics.carry_actions_away =
                self.result.statistics.carry_actions_away.saturating_add(1);
        }
    }

    /// 패스 시도 기록
    fn record_pass_attempt(&mut self, passer_idx: usize, receiver_idx: usize) {
        let is_home = TeamSide::is_home(passer_idx);
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let pass_distance_m = passer_pos.distance_to_m(&receiver_pos);
        let progress_m = Self::pass_progress_m(passer_pos.x, receiver_pos.x, is_home);
        let forward_threshold_m = field::LENGTH_M * 0.05;
        let is_forward = progress_m > forward_threshold_m;
        let is_cross =
            matches!(self.action_queue.last_pass_type(), Some(crate::engine::action_queue::PassType::Cross));

        if is_home {
            self.result.statistics.pass_attempts_home += 1;
            self.result.statistics.pass_distance_sum_home += pass_distance_m;
            if is_cross {
                self.result.statistics.cross_attempts_home += 1;
            }
            if is_forward {
                self.result.statistics.forward_pass_attempts_home += 1;
            } else {
                self.result.statistics.circulation_pass_attempts_home += 1;
            }
        } else {
            self.result.statistics.pass_attempts_away += 1;
            self.result.statistics.pass_distance_sum_away += pass_distance_m;
            if is_cross {
                self.result.statistics.cross_attempts_away += 1;
            }
            if is_forward {
                self.result.statistics.forward_pass_attempts_away += 1;
            } else {
                self.result.statistics.circulation_pass_attempts_away += 1;
            }
        }

        // FIX_2601/0112: Record pass type for calibration snapshot
        // FIX: Use classify_pass_detailed to get is_progressive independently
        // A long pass can ALSO be progressive (they're not mutually exclusive)
        let passer_pos_m = passer_pos.to_meters();
        let receiver_pos_m = receiver_pos.to_meters();
        let start_norm = NormPos::new(passer_pos_m.0 / field::LENGTH_M, passer_pos_m.1 / field::WIDTH_M);
        let end_norm = NormPos::new(receiver_pos_m.0 / field::LENGTH_M, receiver_pos_m.1 / field::WIDTH_M);
        let attacks_right = self.attacks_right(is_home);
        let thresholds = ClassifierThresholds::default();

        // Use detailed classification to get independent boolean flags
        let classification = classify_pass_detailed(start_norm, end_norm, attacks_right, &thresholds);

        let from_zone = pos_to_zone_for_team(start_norm.x, start_norm.y, attacks_right);
        let to_zone = pos_to_zone_for_team(end_norm.x, end_norm.y, attacks_right);

        // FIX_2601/0113: 20-zone tracking
        let from_zone20 = pos_to_posplay_zone_for_team(start_norm.x, start_norm.y, attacks_right);
        let to_zone20 = pos_to_posplay_zone_for_team(end_norm.x, end_norm.y, attacks_right);

        let snapshot = if is_home {
            &mut self.home_stat_snapshot
        } else {
            &mut self.away_stat_snapshot
        };
        snapshot.record_pass_posplay(
            false, // success determined later in record_pass_success
            classification.is_progressive,  // FIX: Now independent of primary type
            false, // key pass determined post-hoc (requires shot detection)
            classification.is_cross,
            classification.is_long,
            matches!(classification.primary, CalibPassType::Backward),
            from_zone,
            to_zone,
            from_zone20,
            to_zone20,
        );
    }

    /// 패스 성공 기록
    fn record_pass_success(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        let is_cross =
            matches!(self.action_queue.last_pass_type(), Some(crate::engine::action_queue::PassType::Cross));
        if is_home {
            self.result.statistics.passes_home += 1;
            self.current_pass_sequence_home = self.current_pass_sequence_home.saturating_add(1);
            if is_cross {
                self.result.statistics.crosses_home += 1;
            }
            // FIX_2601/0112: Increment calibration snapshot pass success
            self.home_stat_snapshot.pass_successes += 1;
        } else {
            self.result.statistics.passes_away += 1;
            self.current_pass_sequence_away = self.current_pass_sequence_away.saturating_add(1);
            if is_cross {
                self.result.statistics.crosses_away += 1;
            }
            // FIX_2601/0112: Increment calibration snapshot pass success
            self.away_stat_snapshot.pass_successes += 1;
        }
        self.action_queue.clear_last_pass_type();
    }

    /// FIX_2601/1128: Record pass pair for reciprocity bonus calculation
    /// Maintains a rolling window of recent (passer, receiver) pairs
    fn record_pass_pair(&mut self, passer_idx: usize, receiver_idx: usize) {
        // FIX_2601/1128: Increased from 20 to 40 to capture more reciprocal opportunities
        const MAX_RECENT_PAIRS: usize = 40;

        // Add new pair
        self.recent_pass_pairs.push_back((passer_idx as u8, receiver_idx as u8));

        // Keep only the most recent pairs
        while self.recent_pass_pairs.len() > MAX_RECENT_PAIRS {
            self.recent_pass_pairs.pop_front();
        }

        // FIX_2601/1130: Increment pass receive count for diversity penalty
        if receiver_idx < 22 {
            self.pass_receive_counts[receiver_idx] =
                self.pass_receive_counts[receiver_idx].saturating_add(1);
        }
    }

    /// FIX_2601/1128: Find a valid reciprocal pass target
    /// Returns a teammate who recently passed to the current passer (A→me→A pattern)
    /// OR a teammate who I recently passed to (me→B→me pattern encourages B to pass back)
    fn find_reciprocal_pass_target(&self, passer_idx: usize, is_home: bool) -> Option<usize> {
        let passer = passer_idx as u8;

        // Priority 1: Find who recently passed TO me (for A→B→A, where I am B)
        for &(from, to) in self.recent_pass_pairs.iter().rev() {
            if to == passer {
                let from_idx = from as usize;
                let is_teammate = if is_home { from_idx < 11 } else { from_idx >= 11 };
                let not_gk = from_idx != 0 && from_idx != 11;
                if is_teammate && not_gk && from_idx != passer_idx {
                    if !self.is_offside_position(from_idx, is_home) {
                        return Some(from_idx);
                    }
                }
            }
        }

        // Priority 2: Find who I recently passed TO (for me→B→me, encourage B to continue)
        // This helps when B immediately passes forward - I can pass back to B
        for &(from, to) in self.recent_pass_pairs.iter().rev() {
            if from == passer {
                let to_idx = to as usize;
                let is_teammate = if is_home { to_idx < 11 } else { to_idx >= 11 };
                let not_gk = to_idx != 0 && to_idx != 11;
                if is_teammate && not_gk && to_idx != passer_idx {
                    if !self.is_offside_position(to_idx, is_home) {
                        return Some(to_idx);
                    }
                }
            }
        }

        None
    }

    pub(crate) fn finalize_pass_sequences(&mut self) {
        if self.current_pass_sequence_home > 0 {
            self.result.statistics.pass_sequence_total_home =
                self.result.statistics.pass_sequence_total_home.saturating_add(
                    self.current_pass_sequence_home,
                );
            self.result.statistics.pass_sequence_count_home =
                self.result.statistics.pass_sequence_count_home.saturating_add(1);
            self.current_pass_sequence_home = 0;
        }
        if self.current_pass_sequence_away > 0 {
            self.result.statistics.pass_sequence_total_away =
                self.result.statistics.pass_sequence_total_away.saturating_add(
                    self.current_pass_sequence_away,
                );
            self.result.statistics.pass_sequence_count_away =
                self.result.statistics.pass_sequence_count_away.saturating_add(1);
            self.current_pass_sequence_away = 0;
        }
    }

    // NOTE: record_shot_attempt() removed (2026-01-05) - would cause double counting
    // Stats are now incremented in ActionResult handlers only
    // Budget tracking is done via record_shot_for_budget() at action scheduling time

    /// 슛 유효타 (on-target) 기록 (Goal 또는 Saved 시 호출)
    fn record_shot_on_target(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.shots_on_target_home += 1;
        } else {
            self.result.statistics.shots_on_target_away += 1;
        }
    }

    /// FIX_2601/0112: 슛 기록 (존별 xG 추적)
    /// FIX_2601/0113: 20-zone 추적 추가
    fn record_shot_for_calibration(&mut self, player_idx: usize, on_target: bool, is_goal: bool, xg: f32) {
        let is_home = TeamSide::is_home(player_idx);
        let pos = self.get_player_position_by_index(player_idx).to_meters();
        let norm_x = pos.0 / field::LENGTH_M;
        let norm_y = pos.1 / field::WIDTH_M;
        let attacks_right = self.attacks_right(is_home);
        let zone = pos_to_zone_for_team(norm_x, norm_y, attacks_right);
        let zone20 = pos_to_posplay_zone_for_team(norm_x, norm_y, attacks_right);

        let snapshot = if is_home {
            &mut self.home_stat_snapshot
        } else {
            &mut self.away_stat_snapshot
        };
        snapshot.record_shot_posplay(on_target, is_goal, xg, zone, zone20);
    }

    /// 헤더 성공 기록 (골 또는 유효 헤딩)
    fn record_header_success(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.headers_home += 1;
        } else {
            self.result.statistics.headers_away += 1;
        }
    }

    /// 헤더 시도 기록
    fn record_header_attempt(&mut self, player_idx: usize, is_shot: bool) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.header_attempts_home += 1;
            if is_shot {
                self.result.statistics.header_shot_attempts_home += 1;
            } else {
                self.result.statistics.header_pass_attempts_home += 1;
            }
        } else {
            self.result.statistics.header_attempts_away += 1;
            if is_shot {
                self.result.statistics.header_shot_attempts_away += 1;
            } else {
                self.result.statistics.header_pass_attempts_away += 1;
            }
        }
    }

    /// 태클 시도 기록
    fn record_tackle_attempt(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.tackle_attempts_home += 1;
        } else {
            self.result.statistics.tackle_attempts_away += 1;
        }
    }

    /// 태클 성공 기록
    fn record_tackle_success(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.tackles_home += 1;
            // FIX_2601/0112: Record to calibration snapshot
            self.home_stat_snapshot.record_tackle(true);
        } else {
            self.result.statistics.tackles_away += 1;
            self.away_stat_snapshot.record_tackle(true);
        }
    }

    /// FIX_2601/0112: 볼 터치 기록 (존별 분포 추적)
    /// FIX_2601/0113: 20-zone 추적 추가
    fn record_ball_touch(&mut self, owner_idx: usize) {
        let is_home = TeamSide::is_home(owner_idx);
        let pos = self.get_player_position_by_index(owner_idx).to_meters();
        let norm_x = pos.0 / field::LENGTH_M;
        let norm_y = pos.1 / field::WIDTH_M;
        let attacks_right = self.attacks_right(is_home);
        let zone = pos_to_zone_for_team(norm_x, norm_y, attacks_right);
        let zone20 = pos_to_posplay_zone_for_team(norm_x, norm_y, attacks_right);

        let snapshot = if is_home {
            &mut self.home_stat_snapshot
        } else {
            &mut self.away_stat_snapshot
        };
        snapshot.record_touch_posplay(zone, zone20);
    }

    /// 돌파(TakeOn) 시도 기록
    fn record_take_on_attempt(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.take_on_attempts_home += 1;
        } else {
            self.result.statistics.take_on_attempts_away += 1;
        }
    }

    /// 돌파(TakeOn) 성공 기록
    fn record_take_on_success(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.take_ons_home += 1;
        } else {
            self.result.statistics.take_ons_away += 1;
        }
    }

    /// 드리블 기록
    fn record_dribble(&mut self, player_idx: usize) {
        let is_home = TeamSide::is_home(player_idx);
        if is_home {
            self.result.statistics.dribbles_home += 1;
        } else {
            self.result.statistics.dribbles_away += 1;
        }
    }

    /// 프리킥 기록
    fn record_freekick(&mut self, is_home_team: bool) {
        if is_home_team {
            self.result.statistics.freekicks_home += 1;
        } else {
            self.result.statistics.freekicks_away += 1;
        }
    }

    /// 코너킥 기록
    fn record_corner(&mut self, is_home_team: bool) {
        if is_home_team {
            self.result.statistics.corners_home += 1;
        } else {
            self.result.statistics.corners_away += 1;
        }
    }

    // ========== FIX_2601/0107: Open-Football Advanced Module Integration ==========

    /// Update offside trap state for both teams
    ///
    /// Calculates defensive line positions and determines if offside trap should be active.
    fn update_offside_trap_state(
        &mut self,
        home_positions_m: &[(f32, f32)],
        away_positions_m: &[(f32, f32)],
        ball_pos_m: (f32, f32),
    ) {
        use super::offside_trap::OffsideTrapConfig;
        use crate::engine::types::Coord10;

        let ball_pos = Coord10::from_meters(ball_pos_m.0, ball_pos_m.1);
        let config = OffsideTrapConfig::default();

        // Home team defensive line (defenders are slots 1-4, excluding GK at 0)
        let home_defenders: Vec<Coord10> = home_positions_m[1..5]
            .iter()
            .map(|(x, y)| Coord10::from_meters(*x, *y))
            .collect();

        // Away team defensive line
        let away_defenders: Vec<Coord10> = away_positions_m[1..5]
            .iter()
            .map(|(x, y)| Coord10::from_meters(*x, *y))
            .collect();

        // Attack direction: Home attacks right (+1) in first half, left (-1) in second half
        let home_attack_dir = if self.is_second_half { -1.0 } else { 1.0 };
        let away_attack_dir = -home_attack_dir;

        // Update home team's defensive line (they defend against away attacks)
        self.offside_trap_state[0].calculate_line_height(
            &home_defenders,
            away_attack_dir, // Away attacks = Home defends
            self.current_tick,
        );

        // Update away team's defensive line
        self.offside_trap_state[1].calculate_line_height(
            &away_defenders,
            home_attack_dir, // Home attacks = Away defends
            self.current_tick,
        );

        // Check if trap should be activated (based on average teamwork/concentration)
        // For now, use default values - can be enhanced with actual player stats
        let avg_teamwork: u8 = 65;
        let avg_concentration: u8 = 65;

        // Get attacker positions
        let home_attackers: Vec<Coord10> = home_positions_m[7..11]
            .iter()
            .map(|(x, y)| Coord10::from_meters(*x, *y))
            .collect();
        let away_attackers: Vec<Coord10> = away_positions_m[7..11]
            .iter()
            .map(|(x, y)| Coord10::from_meters(*x, *y))
            .collect();

        // Home trap (against away attackers)
        self.offside_trap_state[0].trap_active = self.offside_trap_state[0].should_activate_trap(
            ball_pos,
            &away_attackers,
            avg_teamwork,
            avg_concentration,
            &config,
        );

        // Away trap (against home attackers)
        self.offside_trap_state[1].trap_active = self.offside_trap_state[1].should_activate_trap(
            ball_pos,
            &home_attackers,
            avg_teamwork,
            avg_concentration,
            &config,
        );
    }

    /// Update GK sweeping state for both goalkeepers
    ///
    /// Determines if GK should come out, stay, or return to goal.
    fn update_gk_sweeping_state(&mut self, ball_pos_m: (f32, f32)) {
        use super::gk_sweeping::{
            determine_next_state, GKMentalSkills, GKPhysicalSkills, SweepingContext,
        };
        use crate::engine::types::Coord10;

        let ball_pos = Coord10::from_meters(ball_pos_m.0, ball_pos_m.1);

        // Home GK (index 0)
        let home_gk_pos = self.player_positions[0];
        // FIX_2601/0116: Goal centers must flip in 2nd half!
        // 1st half: Home defends x=0, Away defends x=105
        // 2nd half: Home defends x=105, Away defends x=0
        let home_goal_center = if self.is_second_half {
            Coord10::from_meters(field::LENGTH_M, field::CENTER_Y) // Home defends x=105 in 2H
        } else {
            Coord10::from_meters(0.0, field::CENTER_Y) // Home defends x=0 in 1H
        };

        // Away GK (index 11)
        let away_gk_pos = self.player_positions[11];
        let away_goal_center = if self.is_second_half {
            Coord10::from_meters(0.0, field::CENTER_Y) // Away defends x=0 in 2H
        } else {
            Coord10::from_meters(field::LENGTH_M, field::CENTER_Y) // Away defends x=105 in 1H
        };

        // Get ball velocity in Coord10 units (Vel10 vx/vy are already in 0.1m/s units)
        let ball_velocity = (
            self.ball.velocity.vx,
            self.ball.velocity.vy,
        );

        // Check ball ownership
        let ball_owner = self.ball.current_owner;
        let opponent_has_ball_for_home = ball_owner.is_some_and(|idx| idx >= 11);
        let opponent_has_ball_for_away = ball_owner.is_some_and(|idx| idx < 11);

        // Find nearest opponent for each GK
        let nearest_to_home_gk = self.find_nearest_opponent_to_gk(0);
        let nearest_to_away_gk = self.find_nearest_opponent_to_gk(11);

        // Attack direction for GK context
        let home_attack_dir = if self.is_second_half { -1.0 } else { 1.0 };

        // Home GK context
        let home_ctx = SweepingContext {
            gk_pos: home_gk_pos,
            ball_pos,
            ball_velocity,
            opponent_has_ball: opponent_has_ball_for_home,
            nearest_opponent: nearest_to_home_gk,
            goal_center: home_goal_center,
            attack_direction: -home_attack_dir, // Home defends opposite direction
        };

        // Away GK context
        let away_ctx = SweepingContext {
            gk_pos: away_gk_pos,
            ball_pos,
            ball_velocity,
            opponent_has_ball: opponent_has_ball_for_away,
            nearest_opponent: nearest_to_away_gk,
            goal_center: away_goal_center,
            attack_direction: home_attack_dir, // Away defends opposite direction
        };

        // Get GK skills (use default for now, can be enhanced with actual stats)
        let mental = GKMentalSkills::default();
        let physical = GKPhysicalSkills::default();

        // Update states
        self.gk_sweeping_state[0] =
            determine_next_state(self.gk_sweeping_state[0], &home_ctx, &mental, &physical);
        self.gk_sweeping_state[1] =
            determine_next_state(self.gk_sweeping_state[1], &away_ctx, &mental, &physical);
    }

    /// Find nearest opponent to a goalkeeper
    fn find_nearest_opponent_to_gk(&self, gk_idx: usize) -> Option<Coord10> {
        use crate::engine::types::Coord10;

        let gk_pos = self.player_positions[gk_idx];
        let opponent_start = if gk_idx == 0 { 11 } else { 0 };
        let opponent_end = if gk_idx == 0 { 22 } else { 11 };

        let mut nearest: Option<(Coord10, i32)> = None;

        for idx in opponent_start..opponent_end {
            let opp_pos = self.player_positions[idx];
            let dist = gk_pos.distance_to(&opp_pos);

            match nearest {
                None => nearest = Some((opp_pos, dist)),
                Some((_, min_dist)) if dist < min_dist => nearest = Some((opp_pos, dist)),
                _ => {}
            }
        }

        nearest.map(|(pos, _)| pos)
    }

    /// Get GK target position based on sweeping state
    ///
    /// Returns optimal position for GK based on current state and ball position.
    pub fn get_gk_target_position(&self, gk_idx: usize) -> Coord10 {
        use super::gk_sweeping::{
            calculate_optimal_position, calculate_rushing_target, GKPhysicalSkills,
            GKSweepingState, SweepingContext,
        };
        use crate::engine::types::Coord10;

        let team_idx = if gk_idx == 0 { 0 } else { 1 };
        let state = self.gk_sweeping_state[team_idx];
        let gk_pos = self.player_positions[gk_idx];

        // FIX_2601/0116: GK goal position must flip in 2nd half!
        // In 1st half: Home GK (idx=0) defends LEFT goal (x=0), Away GK defends RIGHT (x=105)
        // In 2nd half: Home GK defends RIGHT goal (x=105), Away GK defends LEFT (x=0)
        let goal_center = if self.is_second_half {
            // 2nd half: positions are swapped
            if gk_idx == 0 {
                Coord10::from_meters(field::LENGTH_M, field::CENTER_Y) // Home GK: RIGHT goal
            } else {
                Coord10::from_meters(0.0, field::CENTER_Y) // Away GK: LEFT goal
            }
        } else {
            // 1st half: normal positions
            if gk_idx == 0 {
                Coord10::from_meters(0.0, field::CENTER_Y) // Home GK: LEFT goal
            } else {
                Coord10::from_meters(field::LENGTH_M, field::CENTER_Y) // Away GK: RIGHT goal
            }
        };

        let ball_pos = self.ball.position; // ball.position is already Coord10
        let positioning_skill: u8 = 60; // Default, can be enhanced with actual stats

        match state {
            GKSweepingState::Attentive => {
                // Position on optimal line between goal and ball
                calculate_optimal_position(goal_center, ball_pos, positioning_skill)
            }
            GKSweepingState::ComingOut => {
                // Rush toward ball or opponent (Vel10 vx/vy are already in 0.1m/s units)
                let ball_velocity = (
                    self.ball.velocity.vx,
                    self.ball.velocity.vy,
                );
                let opponent_has_ball = self.ball.current_owner.is_some_and(|idx| {
                    if gk_idx == 0 {
                        idx >= 11
                    } else {
                        idx < 11
                    }
                });

                // FIX_2601/0116: attack_direction must match goal_center
                // -1.0 = defending LEFT goal (x=0), +1.0 = defending RIGHT goal (x=105)
                let attack_direction = if goal_center.x < 500 { -1.0 } else { 1.0 };

                let ctx = SweepingContext {
                    gk_pos,
                    ball_pos,
                    ball_velocity,
                    opponent_has_ball,
                    nearest_opponent: self.find_nearest_opponent_to_gk(gk_idx),
                    goal_center,
                    attack_direction,
                };
                let physical = GKPhysicalSkills::default();
                calculate_rushing_target(&ctx, &physical)
            }
            GKSweepingState::ReturningToGoal => {
                // Return to optimal position
                calculate_optimal_position(goal_center, ball_pos, positioning_skill)
            }
            GKSweepingState::PreparingForSave => {
                // Stay in current position, ready for save
                gk_pos
            }
        }
    }

    // =========================================================================
    // FIX_2601/0107 Phase 8.3: Steering Behavior Integration
    // =========================================================================

    /// Select appropriate steering behavior based on player context
    ///
    /// Returns steering parameters to modify movement toward target.
    /// - Arrive: Slow down when close to target (for positioning)
    /// - Pursuit: Anticipate ball movement (for ball chasing)
    /// - Seek: Direct movement (default)
    fn select_steering_behavior_params(
        &self,
        player_idx: usize,
        target_m: (f32, f32),
    ) -> SteeringParams {
        let player_pos_m = self.player_positions[player_idx].to_meters();
        let ball_pos_m = self.ball.position.to_meters();
        let ball_vel = (
            self.ball.velocity.vx as f32 / 10.0, // Convert from Vel10 (0.1m/s) to m/s
            self.ball.velocity.vy as f32 / 10.0,
        );

        let dist_to_target =
            ((target_m.0 - player_pos_m.0).powi(2) + (target_m.1 - player_pos_m.1).powi(2)).sqrt();
        let dist_to_ball =
            ((ball_pos_m.0 - player_pos_m.0).powi(2) + (ball_pos_m.1 - player_pos_m.1).powi(2))
                .sqrt();

        // Check if this player is chasing the ball (no owner and close to ball)
        let is_chasing_ball = self.ball.current_owner.is_none() && dist_to_ball < 15.0;

        // Check if player is ball owner (should slow down / maintain control)
        let is_ball_owner = self.ball.current_owner == Some(player_idx);

        if is_ball_owner {
            // Ball owner: Use Arrive behavior for controlled movement
            SteeringParams {
                behavior_type: SteeringBehaviorType::Arrive,
                slowing_distance: 5.0, // Start slowing 5m from target
                speed_multiplier: 0.85, // Slightly slower with ball
                target_offset: (0.0, 0.0),
            }
        } else if is_chasing_ball {
            // Chasing loose ball: Use Pursuit to anticipate ball position
            let ball_speed = (ball_vel.0 * ball_vel.0 + ball_vel.1 * ball_vel.1).sqrt();
            if ball_speed > 1.0 {
                // Ball moving significantly: anticipate position
                let prediction_time = (dist_to_ball / 7.0).min(1.5); // Max 1.5 second prediction
                let predicted_offset = (
                    ball_vel.0 * prediction_time,
                    ball_vel.1 * prediction_time,
                );
                SteeringParams {
                    behavior_type: SteeringBehaviorType::Pursuit,
                    slowing_distance: 2.0,
                    speed_multiplier: 1.1, // Sprint to intercept
                    target_offset: predicted_offset,
                }
            } else {
                // Ball nearly stationary: direct seek
                SteeringParams {
                    behavior_type: SteeringBehaviorType::Seek,
                    slowing_distance: 0.0,
                    speed_multiplier: 1.0,
                    target_offset: (0.0, 0.0),
                }
            }
        } else if dist_to_target < 3.0 {
            // Very close to target: Arrive behavior to prevent oscillation
            SteeringParams {
                behavior_type: SteeringBehaviorType::Arrive,
                slowing_distance: 3.0,
                speed_multiplier: 0.7,
                target_offset: (0.0, 0.0),
            }
        } else {
            // Default: Seek (direct movement)
            SteeringParams {
                behavior_type: SteeringBehaviorType::Seek,
                slowing_distance: 0.0,
                speed_multiplier: 1.0,
                target_offset: (0.0, 0.0),
            }
        }
    }

    /// Apply steering behavior to modify target and physics params
    ///
    /// Returns modified (target_m, speed_multiplier)
    fn apply_steering_params(
        &self,
        player_idx: usize,
        original_target_m: (f32, f32),
        params: &SteeringParams,
    ) -> ((f32, f32), f32) {
        let player_pos_m = self.player_positions[player_idx].to_meters();

        // Apply target offset (for pursuit behavior)
        let modified_target = (
            original_target_m.0 + params.target_offset.0,
            original_target_m.1 + params.target_offset.1,
        );

        // Calculate distance to modified target
        let dist = ((modified_target.0 - player_pos_m.0).powi(2)
            + (modified_target.1 - player_pos_m.1).powi(2))
        .sqrt();

        // Apply Arrive slowing if appropriate
        let arrive_factor = match params.behavior_type {
            SteeringBehaviorType::Arrive if params.slowing_distance > 0.0 => {
                if dist < params.slowing_distance {
                    // Smooth deceleration curve
                    (dist / params.slowing_distance).max(0.3)
                } else {
                    1.0
                }
            }
            _ => 1.0,
        };

    let final_speed_mult = params.speed_multiplier * arrive_factor;

    (modified_target, final_speed_mult)
    }

    /// FIX_2601 Phase 2: Coord10 version of apply_steering_params
    fn apply_steering_params_coord10(
        &self,
        player_idx: usize,
        original_target: Coord10,
        params: &SteeringParams,
    ) -> (Coord10, f32) {
        let player_pos = self.player_positions[player_idx];

        // Apply target offset (Coord10 units)
        let offset_x = (params.target_offset.0 * 10.0) as i32; // m to Coord10
        let offset_y = (params.target_offset.1 * 10.0) as i32;
        let modified_target = Coord10 {
            x: original_target.x + offset_x,
            y: original_target.y + offset_y,
            z: original_target.z,
        };

        // Calculate distance to modified target (in Coord10 units)
        let dist = player_pos.distance_to(&modified_target) as f32 / 10.0; // Coord10 to meters

        // Apply Arrive slowing if appropriate
        let arrive_factor = match params.behavior_type {
            SteeringBehaviorType::Arrive if params.slowing_distance > 0.0 => {
                if dist < params.slowing_distance {
                    (dist / params.slowing_distance).max(0.3)
                } else {
                    1.0
                }
            }
            _ => 1.0,
        };

        let final_speed_mult = params.speed_multiplier * arrive_factor;

        (modified_target, final_speed_mult)
    }
}

/// Steering behavior type for player movement
#[derive(Debug, Clone, Copy, PartialEq)]
enum SteeringBehaviorType {
    /// Direct movement toward target
    Seek,
    /// Slow down when approaching target
    Arrive,
    /// Anticipate moving target position
    Pursuit,
}

/// Parameters for steering behavior
#[derive(Debug, Clone)]
struct SteeringParams {
    behavior_type: SteeringBehaviorType,
    slowing_distance: f32, // Distance to start slowing (meters)
    speed_multiplier: f32, // Speed adjustment factor
    target_offset: (f32, f32), // Offset to apply to target (meters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::match_sim::test_fixtures::{
        create_test_engine_uninit as create_test_engine, create_test_team,
    };
    use crate::engine::match_sim::MatchPlan;
    use crate::models::player::PlayerAttributes;

    #[allow(dead_code)]
    fn create_test_engine_with_home_attrs(track_id: usize, attrs: PlayerAttributes) -> MatchEngine {
        assert!(track_id < 11, "expected home starter track_id 0-10, got {}", track_id);

        let mut home_team = create_test_team("Home");
        home_team.players[track_id].attributes = Some(attrs);

        let plan = MatchPlan {
            home_team,
            away_team: create_test_team("Away"),
            seed: 12345,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        };
        MatchEngine::new(plan).expect("match engine init")
    }

    #[test]
    fn test_derive_home_has_ball_for_phases_is_stable_during_flight() {
        let mut ball = super::super::Ball::default();

        // In-flight / loose frames: current_owner is None, but previous_owner keeps possession stable.
        ball.current_owner = None;
        ball.previous_owner = Some(3); // home player
        assert!(MatchEngine::derive_home_has_ball_for_phases(&ball, false));

        ball.previous_owner = Some(13); // away player
        assert!(!MatchEngine::derive_home_has_ball_for_phases(&ball, true));

        // If still unknown, keep previous possession (no flicker).
        ball.previous_owner = None;
        assert!(MatchEngine::derive_home_has_ball_for_phases(&ball, true));
        assert!(!MatchEngine::derive_home_has_ball_for_phases(&ball, false));

        // Controlled frames: current_owner wins.
        ball.current_owner = Some(15); // away player
        ball.previous_owner = Some(3); // home last-touch
        assert!(!MatchEngine::derive_home_has_ball_for_phases(&ball, true));
    }

    #[test]
    fn test_build_execution_context() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        let ctx = engine.build_execution_context();

        assert_eq!(ctx.player_positions.len(), 22);
        assert_eq!(ctx.player_stats.len(), 22);
        assert_eq!(ctx.goalkeeper_indices, (0, 11));
    }

    #[test]
    fn test_goalkeeper_handling_violation_emits_foul_and_applies_free_kick() {
        use crate::engine::action_queue::{ActionResult, BallState, RestartType};
        use crate::models::events::EventType;

        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // Home GK (track_id=0) commits an illegal handling at this position.
        let foul_pos = Coord10::from_meters(17.0, field::CENTER_Y);
        engine.player_positions[0] = foul_pos;
        engine.ball.position = foul_pos;

        // Precondition: executor layer sets the restart in ActionQueue (SSOT),
        // then emits ActionResult into MatchEngine.
        engine.action_queue.set_ball_state(BallState::OutOfPlay {
            restart_type: RestartType::FreeKick,
            position: foul_pos,
            home_team: false, // away team receives the free kick
        });

        engine.handle_action_result(ActionResult::GoalkeeperHandlingViolation {
            goalkeeper_idx: 0,
            last_touch_idx: Some(20), // away shooter
            is_indirect: false,
            xg: Some(0.01),
        });

        assert!(
            engine
                .result
                .events
                .iter()
                .any(|e| matches!(e.event_type, EventType::Foul) && e.player_track_id == Some(0)),
            "expected a foul event for GK track_id=0"
        );
        assert!(
            engine
                .result
                .events
                .iter()
                .any(|e| matches!(e.event_type, EventType::Freekick) && !e.is_home_team),
            "expected an away free kick restart"
        );

        let kicker_idx = engine.ball.current_owner.expect("free kick should assign a kicker");
        assert!(!TeamSide::is_home(kicker_idx), "away team should take the free kick");
    }

    #[test]
    fn test_goalkeeper_handling_violation_indirect_starts_indirect_free_kick_fsm() {
        use crate::engine::action_queue::{ActionResult, BallState, RestartType};
        use crate::engine::phase_action::SetPieceType;

        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // Home GK (track_id=0) commits a technical handling offense that should restart
        // as an indirect free kick for away.
        let foul_pos = Coord10::from_meters(5.0, field::CENTER_Y);
        engine.player_positions[0] = foul_pos;
        engine.ball.position = foul_pos;

        // Precondition: executor layer sets the restart in ActionQueue (SSOT).
        engine.action_queue.set_ball_state(BallState::OutOfPlay {
            restart_type: RestartType::FreeKick,
            position: foul_pos,
            home_team: false, // away team receives the free kick
        });

        engine.handle_action_result(ActionResult::GoalkeeperHandlingViolation {
            goalkeeper_idx: 0,
            last_touch_idx: Some(5), // teammate passer (for event target linkage)
            is_indirect: true,
            xg: None,
        });

        let last_set_piece = engine
            .active_set_pieces
            .last()
            .expect("expected an indirect free kick SetPieceAction");
        assert_eq!(last_set_piece.set_piece_type, SetPieceType::FreeKickIndirect);

        let kicker_idx = engine.ball.current_owner.expect("free kick should assign a kicker");
        assert!(!TeamSide::is_home(kicker_idx), "away team should take the indirect free kick");
    }

    #[test]
    fn test_handball_foul_outside_penalty_area_applies_direct_free_kick() {
        use crate::engine::action_queue::{ActionResult, BallState, RestartType};
        use crate::engine::phase_action::SetPieceType;

        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // Home outfield player commits a handball outside the penalty area -> direct FK for away.
        let offender_idx = 5;
        let foul_pos = Coord10::from_meters(30.0, field::CENTER_Y);
        engine.player_positions[offender_idx] = foul_pos;
        engine.ball.position = foul_pos;

        engine.action_queue.set_ball_state(BallState::OutOfPlay {
            restart_type: RestartType::FreeKick,
            position: foul_pos,
            home_team: false, // away receives
        });

        engine.handle_action_result(ActionResult::HandballFoul {
            offender_idx,
            last_touch_idx: Some(20),
        });

        assert_eq!(engine.result.statistics.fouls_home, 1);
        assert_eq!(engine.result.statistics.handball_fouls_home, 1);
        assert_eq!(engine.result.statistics.penalties_home, 0);
        assert_eq!(engine.result.statistics.penalties_away, 0);

        let last_set_piece = engine
            .active_set_pieces
            .last()
            .expect("expected a direct free kick SetPieceAction");
        assert_eq!(last_set_piece.set_piece_type, SetPieceType::FreeKickDirect);

        let kicker_idx = engine.ball.current_owner.expect("free kick should assign a kicker");
        assert!(!TeamSide::is_home(kicker_idx), "away team should take the direct free kick");
    }

    #[test]
    fn test_handball_foul_inside_penalty_area_applies_penalty_kick() {
        use crate::engine::action_queue::{ActionResult, BallState, RestartType};
        use crate::engine::phase_action::SetPieceType;

        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // Home outfield player commits a handball inside own penalty area -> penalty for away.
        let offender_idx = 5;
        let foul_pos = Coord10::from_meters(5.0, field::CENTER_Y);
        engine.player_positions[offender_idx] = foul_pos;
        engine.ball.position = foul_pos;

        engine.action_queue.set_ball_state(BallState::OutOfPlay {
            restart_type: RestartType::Penalty,
            position: foul_pos,
            home_team: false, // away receives
        });

        engine.handle_action_result(ActionResult::HandballFoul {
            offender_idx,
            last_touch_idx: Some(20),
        });

        assert_eq!(engine.result.statistics.fouls_home, 1);
        assert_eq!(engine.result.statistics.handball_fouls_home, 1);
        assert_eq!(engine.result.statistics.penalties_home, 0);
        assert_eq!(engine.result.statistics.penalties_away, 1);
        assert_eq!(engine.result.statistics.handball_penalties_home, 0);
        assert_eq!(engine.result.statistics.handball_penalties_away, 1);

        let last_set_piece = engine
            .active_set_pieces
            .last()
            .expect("expected a penalty SetPieceAction");
        assert_eq!(last_set_piece.set_piece_type, SetPieceType::Penalty);

        let kicker_idx = engine.ball.current_owner.expect("penalty should assign a kicker");
        assert!(!TeamSide::is_home(kicker_idx), "away team should take the penalty");
    }

    #[test]
    fn test_build_player_stats() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        let stats = engine.build_player_stats();

        assert_eq!(stats.len(), 22);
        // 스탯 범위 확인 (u8 타입)
        for stat in &stats {
            assert!(stat.passing >= 1 && stat.passing <= 99);
            assert!(stat.dribbling >= 1 && stat.dribbling <= 99);
        }
    }

    // NOTE: test_attrs_flow_into_fsm_creation 삭제됨 (2025-12-14)
    // FSM 제거로 active_passes/shots/tackles/dribbles 접근 불가
    // ActionQueue 기반 속성 흐름 테스트는 별도 추가 필요

    #[test]
    fn test_tick_based_sim_always_enabled() {
        // 2025-12-11: Phase 3-5 활성화로 tick 기반 시뮬레이션만 사용
        // 레거시 플래그 제거됨 - 항상 tick 기반으로 동작
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // simulate_minute_tick_based가 호출 가능함을 확인
        // tick 기반 시뮬레이션이 항상 사용됨
        engine.simulate_minute_tick_based(70.0, 70.0, 0.5);

        // ActionQueue가 초기화되어 있음 (tick 기반 시스템의 핵심)
        let _ = engine.action_queue.pending_count();
    }

    #[test]
    fn test_single_tick_simulation() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // 1분 시뮬레이션
        engine.simulate_minute_tick_based(70.0, 70.0, 0.5);

        // 액션이 생성되었거나 이벤트가 있어야 함
        let _ = engine.action_queue.pending_count();
    }

    #[test]
    fn test_meters_normalized_roundtrip() {
        let engine = create_test_engine();

        let original = (0.5, 0.5);
        let meters = engine.normalized_to_meters(original);
        let back = engine.meters_to_normalized(meters);

        assert!((back.0 - original.0).abs() < 0.01);
        assert!((back.1 - original.1).abs() < 0.01);
    }
    #[test]
    fn test_detect_out_of_play_corner() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        let out_pos = Coord10::from_meters(field::LENGTH_M, 10.0);
        engine.ball.position = out_pos;
        engine.ball.current_owner = None;
        engine.ball.previous_owner = Some(12); // away last touch
        engine
            .action_queue
            .set_ball_state(BallState::Loose { position: out_pos, velocity: Vel10::default() });

        let result = engine.detect_out_of_play_action();
        assert!(matches!(
            result,
            Some(ActionResult::OutOfBounds {
                restart_type: RestartType::Corner,
                home_team: true,
                ..
            })
        ));
    }

    #[test]
    fn test_detect_out_of_play_goal_kick_with_restart_position() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // Ball out on the right goal line (away goal), outside the goal mouth. If home last touched,
        // away gets a goal kick at the fixed restart position.
        let out_pos = Coord10::from_meters(field::LENGTH_M, 10.0);
        engine.ball.position = out_pos;
        engine.ball.current_owner = None;
        engine.ball.previous_owner = Some(9); // home last touch
        engine.action_queue.set_ball_state(BallState::Loose {
            position: out_pos,
            velocity: Vel10::default(),
        });

        let result = engine.detect_out_of_play_action();
        let expected_restart = Coord10::from_normalized((0.95, 0.5));
        match result {
            Some(ActionResult::OutOfBounds {
                restart_type: RestartType::GoalKick,
                position,
                home_team,
            }) => {
                assert!(!home_team, "away team should take the goal kick");
                assert_eq!(position, expected_restart);
            }
            other => panic!("expected GoalKick out-of-bounds, got: {:?}", other),
        }
    }

    #[test]
    fn test_detect_out_of_play_throw_in_with_restart_position() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // Ball out on the top touchline. If away last touched, home gets the throw-in and the
        // restart position is clamped and snapped to the touchline band.
        let out_pos = Coord10::from_meters(2.0, field::WIDTH_M);
        engine.ball.position = out_pos;
        engine.ball.current_owner = None;
        engine.ball.previous_owner = Some(12); // away last touch
        engine.action_queue.set_ball_state(BallState::Loose {
            position: out_pos,
            velocity: Vel10::default(),
        });

        let result = engine.detect_out_of_play_action();
        let expected_restart = Coord10::from_normalized((0.05, 0.95));
        match result {
            Some(ActionResult::OutOfBounds {
                restart_type: RestartType::ThrowIn,
                position,
                home_team,
            }) => {
                assert!(home_team, "home team should take the throw-in");
                assert_eq!(position, expected_restart);
            }
            other => panic!("expected ThrowIn out-of-bounds, got: {:?}", other),
        }
    }

    // ========== Phase 5: Hero Growth Tests ==========

    #[test]
    fn test_hero_xp_bucket_initialized() {
        let engine = create_test_engine();
        assert_eq!(engine.hero_xp_bucket().total_events(), 0);
        assert_eq!(engine.hero_xp_bucket().total_xp(), 0.0);
    }

    #[test]
    fn test_record_action_result_xp_without_user() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // user_player가 없으면 XP 기록 안됨
        let result = ActionResult::GoalScored { scorer_idx: 9, assist_idx: Some(7), xg: 0.3 };
        engine.record_action_result_xp(&result);

        assert_eq!(engine.hero_xp_bucket().total_events(), 0);
    }

    #[test]
    fn test_calculate_hero_growth() {
        let engine = create_test_engine();

        // 빈 버킷에서 성장 계산
        let growth = engine.calculate_hero_growth(|_| 50);

        assert!(!growth.has_growth());
        assert_eq!(growth.total_gains(), 0);
    }

    #[test]
    fn test_apply_xp_overflow() {
        use crate::engine::growth::PlayerAttribute;
        use std::collections::HashMap;

        let mut engine = create_test_engine();

        let mut overflow = HashMap::new();
        overflow.insert(PlayerAttribute::Passing, 5.0);
        overflow.insert(PlayerAttribute::Finishing, 3.0);

        engine.apply_xp_overflow(&overflow);

        assert!((engine.hero_xp_bucket().get_xp(PlayerAttribute::Passing) - 5.0).abs() < 0.01);
        assert!((engine.hero_xp_bucket().get_xp(PlayerAttribute::Finishing) - 3.0).abs() < 0.01);
    }

    // ========== P10-13: Stamina System Tests ==========

    #[test]
    fn test_stamina_initial_values() {
        let engine = create_test_engine();

        // 모든 선수는 1.0 (풀 컨디션)으로 시작
        for i in 0..22 {
            assert_eq!(engine.stamina[i], 1.0);
            assert!(!engine.sprint_state[i]);
        }
    }

    #[test]
    fn test_stamina_decay_tick() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        let initial_stamina = engine.stamina[0];

        // 1틱 decay
        engine.decay_stamina_tick();

        // stamina가 감소해야 함
        assert!(engine.stamina[0] < initial_stamina);
        assert!(engine.stamina[0] > 0.99); // 한 틱에 많이 감소하지 않음
    }

    #[test]
    fn test_stamina_decay_90min_no_sprint() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // 90분 = 90 * 60 * 4 = 21600 ticks (스프린트 없이)
        for _ in 0..21600 {
            engine.decay_stamina_tick();
        }

        // 모든 선수 stamina가 감소해야 함 (attributes 없으면 overall 70 기준)
        for i in 0..22 {
            assert!(engine.stamina[i] < 1.0);
            assert!(engine.stamina[i] > 0.0);
            // overall 70 선수: 약 0.4-0.6 범위
            assert!(engine.stamina[i] > 0.3, "Player {} stamina too low: {}", i, engine.stamina[i]);
            assert!(
                engine.stamina[i] < 0.8,
                "Player {} stamina too high: {}",
                i,
                engine.stamina[i]
            );
        }
    }

    #[test]
    fn test_sprint_increases_stamina_decay() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // 두 선수 비교: 하나는 스프린트, 하나는 일반
        let sprinter_idx = 5;
        let walker_idx = 6;

        engine.stamina[sprinter_idx] = 1.0;
        engine.stamina[walker_idx] = 1.0;
        engine.sprint_state[sprinter_idx] = true;
        engine.sprint_state[walker_idx] = false;

        // 1000 ticks 시뮬레이션
        for _ in 0..1000 {
            engine.decay_stamina_tick();
        }

        // 스프린터가 더 많이 감소해야 함
        assert!(engine.stamina[sprinter_idx] < engine.stamina[walker_idx]);
    }

    #[test]
    fn test_action_stamina_cost() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        let player_idx = 9; // striker
        engine.stamina[player_idx] = 1.0;

        // Shot은 0.008 * cost_mult 비용
        engine.apply_action_stamina_cost(player_idx, "shot");

        assert!(engine.stamina[player_idx] < 1.0);
        assert!(engine.stamina[player_idx] > 0.98); // shot 비용 ~0.008

        // Tackle은 더 비쌈
        engine.stamina[player_idx] = 1.0;
        engine.apply_action_stamina_cost(player_idx, "tackle");

        assert!(engine.stamina[player_idx] < 1.0);
        assert!(engine.stamina[player_idx] > 0.97); // tackle 비용 ~0.012
    }

    #[test]
    fn test_action_stamina_cost_respects_stamina_drain_mult_per_side() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        engine.home_match_modifiers.stamina_drain_mult = 1.0;
        engine.away_match_modifiers.stamina_drain_mult = 1.2;

        let home_idx = 9;
        let away_idx = 20;

        engine.stamina[home_idx] = 1.0;
        engine.apply_action_stamina_cost(home_idx, "shot");
        let home_loss = 1.0 - engine.stamina[home_idx];

        engine.stamina[away_idx] = 1.0;
        engine.apply_action_stamina_cost(away_idx, "shot");
        let away_loss = 1.0 - engine.stamina[away_idx];

        assert!(home_loss > 0.0);
        assert!(away_loss > home_loss, "away should lose more stamina when stamina_drain_mult is higher");
        let ratio = away_loss / home_loss;
        assert!(
            (ratio - 1.2).abs() < 0.01,
            "expected away_loss/home_loss ≈ 1.2 (stamina_drain_mult), got {ratio} (home_loss={home_loss}, away_loss={away_loss})",
        );
    }

    #[test]
    fn test_action_stamina_cost_respects_condition_drain_mult() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // Eliminate other multipliers so the condition multiplier is isolated.
        engine.home_match_modifiers.stamina_drain_mult = 1.0;
        engine.away_match_modifiers.stamina_drain_mult = 1.0;

        let player_idx = 9; // home striker slot (stable in test fixtures)

        engine.setup.home.starters[player_idx].condition_level = 5;
        engine.stamina[player_idx] = 1.0;
        engine.apply_action_stamina_cost(player_idx, "shot");
        let loss_excellent = 1.0 - engine.stamina[player_idx];

        engine.setup.home.starters[player_idx].condition_level = 1;
        engine.stamina[player_idx] = 1.0;
        engine.apply_action_stamina_cost(player_idx, "shot");
        let loss_terrible = 1.0 - engine.stamina[player_idx];

        let expected_ratio =
            crate::fix01::condition_drain_mult(1) / crate::fix01::condition_drain_mult(5);
        let got_ratio = loss_terrible / loss_excellent;
        assert!(
            (got_ratio - expected_ratio).abs() < 0.01,
            "expected loss ratio ≈ {expected_ratio} (cond 1/cond 5), got {got_ratio} (loss1={loss_terrible}, loss5={loss_excellent})",
        );
    }

    #[test]
    fn test_stamina_recovery_while_resting() {
        // FIX_2601/0106 P5: 휴식 중 스태미나 회복 테스트
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        let resting_idx = 5;
        let active_idx = 6;

        // 둘 다 낮은 스태미나 시작
        engine.stamina[resting_idx] = 0.40;
        engine.stamina[active_idx] = 0.40;
        engine.player_resting[resting_idx] = true;
        engine.player_resting[active_idx] = false;

        // 500 ticks 시뮬레이션 (~125초)
        for _ in 0..500 {
            engine.decay_stamina_tick();
        }

        // 휴식 중인 선수가 더 높은 스태미나 보유해야 함
        assert!(
            engine.stamina[resting_idx] > engine.stamina[active_idx],
            "Resting player should have more stamina: {} vs {}",
            engine.stamina[resting_idx],
            engine.stamina[active_idx]
        );

        // 휴식 중인 선수는 회복되어야 함 (0.40 → 0.5+ 예상)
        assert!(
            engine.stamina[resting_idx] > 0.45,
            "Resting player should recover stamina: {}",
            engine.stamina[resting_idx]
        );

        // 활동 중인 선수는 감소해야 함
        assert!(
            engine.stamina[active_idx] < 0.40,
            "Active player should lose stamina: {}",
            engine.stamina[active_idx]
        );
    }

    #[test]
    fn test_high_stamina_recovers_faster() {
        // FIX_2601/0106 P5: 스태미나 속성 높은 선수가 더 빨리 회복
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // 두 선수 비교 (test_fixtures에서 같은 속성이므로 직접 조작하기 어려움)
        // 대신 회복률 공식 검증: recovery_mult = 0.5 + stamina_attr * 0.5

        let player_idx = 5;
        engine.stamina[player_idx] = 0.30;
        engine.player_resting[player_idx] = true;

        // 휴식 100 ticks
        for _ in 0..100 {
            engine.decay_stamina_tick();
        }

        // 회복되어야 함 (0.30 → 0.32+ 예상)
        // BASE_RECOVERY_RATE = 0.0003, 100틱 = 0.03 회복 (기본, 감쇠 제외)
        assert!(
            engine.stamina[player_idx] > 0.30,
            "Player should recover while resting: {}",
            engine.stamina[player_idx]
        );
    }

    #[test]
    fn test_get_fatigue_normalized() {
        let mut engine = create_test_engine();

        // stamina 1.0 → fatigue 0.0
        engine.stamina[0] = 1.0;
        assert!((engine.get_fatigue_normalized(0) - 0.0).abs() < 0.001);

        // stamina 0.7 → fatigue 0.3
        engine.stamina[1] = 0.7;
        assert!((engine.get_fatigue_normalized(1) - 0.3).abs() < 0.001);

        // stamina 0.0 → fatigue 1.0
        engine.stamina[2] = 0.0;
        assert!((engine.get_fatigue_normalized(2) - 1.0).abs() < 0.001);

        // out of bounds → 0.0
        assert_eq!(engine.get_fatigue_normalized(99), 0.0);
    }

    #[test]
    fn test_update_sprint_state() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        // 선수 속도 설정
        let player_idx = 5;

        // test_fixtures의 default_attributes()는 pace = 50 설정 (0-100 스케일)
        // pace = 50: pace_n = 0.5, max_speed = 7.0 + 0.5 * 2.5 = 8.25 m/s
        // sprint threshold (40%) = 8.25 * 0.4 = 3.3 m/s

        // 느린 속도 → 스프린트 아님
        engine.player_speeds[player_idx] = 3.0; // < 3.3 m/s
        engine.update_sprint_state(player_idx);
        assert!(!engine.sprint_state[player_idx]);

        // 빠른 속도 → 스프린트
        engine.player_speeds[player_idx] = 3.5; // > 3.3 m/s
        engine.update_sprint_state(player_idx);
        assert!(engine.sprint_state[player_idx]);
    }

    #[test]
    fn test_stamina_in_tick_loop() {
        let mut engine = create_test_engine();
        engine.initialize_player_positions();

        let initial_staminas: Vec<f32> = engine.stamina.to_vec();

        // 1분 시뮬레이션 (240 ticks)
        engine.simulate_minute_tick_based(50.0, 50.0, 0.5);

        // 모든 선수의 stamina가 감소해야 함
        for i in 0..22 {
            assert!(
                engine.stamina[i] < initial_staminas[i],
                "Player {} stamina didn't decrease: {} -> {}",
                i,
                initial_staminas[i],
                engine.stamina[i]
            );
        }
    }

    #[test]
    fn test_player_velocity_recorded() {
        let mut engine = create_test_engine().with_position_tracking();
        engine.initialize_player_positions();

        // Simulate 1 minute to generate position data
        engine.simulate_minute_tick_based(70.0, 70.0, 0.5);

        // Verify velocity was recorded
        let pos_data = engine.result.position_data.as_ref().expect("Position data should exist");

        // Check player 0's positions
        // FIX_2601/0109: players is now [Vec; 22] instead of HashMap
        let player_positions = &pos_data.players[0];
        assert!(!player_positions.is_empty(), "Player 0 should have recorded positions");

        // At least one position should have velocity
        let has_velocity = player_positions.iter().any(|item| item.velocity.is_some());
        assert!(has_velocity, "Expected at least one position with velocity recorded");

        // Verify velocity values are reasonable (< 10 m/s for football players)
        for item in player_positions.iter() {
            if let Some((vx, vy)) = item.velocity {
                let speed = (vx * vx + vy * vy).sqrt();
                assert!(speed < 10.0, "Speed {} m/s exceeds max realistic player speed", speed);
            }
        }
    }
}
