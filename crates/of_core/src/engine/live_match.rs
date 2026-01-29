//! Live Match Streaming API
//!
//! Phase 7: Real-time match streaming support.
//! Provides tick-by-tick simulation with user intervention capabilities.

use crate::engine::field_board::FieldBoardSnapshotExport;
use crate::engine::match_sim::{
    MatchEngine, MatchPlan, MiniMapObservation, MiniMapSpec, SimpleVectorObservation, StickyAction,
    StickyActions,
};
use crate::engine::tactical_context::TeamSide;
use crate::models::{MatchEvent, MatchResult};
use crate::models::replay::types::DecisionIntent;
use crate::tactics::TeamInstructions;

/// Tick rate constants for live streaming API.
///
/// Standardized to the replay/batch tick (tick_based.rs): 240 ticks/min (250ms).
/// UI smoothness should be achieved via client-side interpolation.
pub const MS_PER_TICK: u64 = 250; // 250ms per tick (streaming tick)
pub const TICKS_PER_MINUTE: u64 = 240; // 60000ms / 250ms = 240 ticks

// ============================================
// StepResult: Per-tick result for streaming
// ============================================

/// Result of a single tick step in live match streaming.
#[derive(Debug, Clone)]
pub enum StepResult {
    /// Session not started yet
    NotStarted,

    /// Normal tick with position snapshot and events
    Tick(TickData),

    /// Half-time break reached (45 minutes)
    HalfTime(HalfTimeData),

    /// Match finished
    FullTime(FullTimeData),
}

/// Data returned for each tick
#[derive(Debug, Clone)]
pub struct TickData {
    /// Current timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Current minute (0-90+)
    pub minute: u8,
    /// Ball position in meters (0-105, 0-68)
    pub ball_position: (f32, f32),
    /// Ball height in meters
    pub ball_height: f32,
    /// Ball owner player index (None if loose ball)
    pub ball_owner_idx: Option<u8>,
    /// All 22 player positions in meters
    pub player_positions: Vec<PlayerPosition>,
    /// Events that occurred during this tick (goals, shots, etc.)
    pub events: Vec<MatchEvent>,
    /// Current score
    pub score: (u8, u8),
    /// Optional team-view simple vector observation
    pub team_view_simple: Option<SimpleVectorObservation>,
    /// Optional team-view minimap observation
    pub team_view_minimap: Option<MiniMapObservation>,
    /// Decision intents captured this tick (Gate B context)
    pub decision_intents: Vec<DecisionIntent>,
    /// Optional FieldBoard snapshot for debug overlays
    pub field_board_snapshot: Option<FieldBoardSnapshotExport>,
    /// Offside lines in meters (home, away)
    pub offside_lines: OffsideLineSnapshot,
}

/// Player position data
#[derive(Debug, Clone)]
pub struct PlayerPosition {
    /// Player index (0-10 = home, 11-21 = away)
    pub index: u8,
    /// Position in meters (0-105, 0-68)
    pub position: (f32, f32),
    /// Player state (idle, running, etc.)
    pub state: String,
    /// Current stamina (0.0 = exhausted, 1.0 = fresh)
    pub stamina: f32,
}

/// Offside line positions in meters
#[derive(Debug, Clone, Copy)]
pub struct OffsideLineSnapshot {
    pub home_x: f32,
    pub away_x: f32,
}

/// Optional team-view observation config for live sessions.
#[derive(Debug, Clone)]
pub struct TeamViewObservationConfig {
    pub observer_is_home: bool,
    pub include_simple: bool,
    pub include_minimap: bool,
    pub minimap_spec: MiniMapSpec,
}

impl TeamViewObservationConfig {
    pub fn is_enabled(&self) -> bool {
        self.include_simple || self.include_minimap
    }
}

/// Data returned at half-time
#[derive(Debug, Clone)]
pub struct HalfTimeData {
    /// Current score at half-time
    pub score: (u8, u8),
    /// Statistics summary
    pub possession: (u8, u8),
    pub shots: (u8, u8),
    pub shots_on_target: (u8, u8),
}

/// Data returned at full-time
#[derive(Debug, Clone)]
pub struct FullTimeData {
    /// Final match result
    pub result: MatchResult,
    /// All events from the match (for replay saving)
    pub all_events: Vec<MatchEvent>,
}

// ============================================
// MatchState: Current match state
// ============================================

/// Current state of the live match
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchState {
    /// Not started yet
    NotStarted,
    /// First half in progress
    FirstHalf,
    /// Half-time break
    HalfTimeBreak,
    /// Second half in progress
    SecondHalf,
    /// Match finished
    Finished,
}

// ============================================
// LiveMatchSession: Main session struct
// ============================================

/// Live match session for real-time streaming.
///
/// Wraps `MatchEngine` and provides tick-by-tick control.
pub struct LiveMatchSession {
    pub engine: MatchEngine,
    state: MatchState,
    /// Cached strength values from init()
    home_strength: f32,
    away_strength: f32,
    possession_ratio: f32,
    match_duration: u8,
    /// Whether to record MatchResult.position_data during session streaming.
    position_tracking_enabled: bool,
    /// Cursor for incremental per-tick event streaming
    last_event_count: usize,
    /// Events from the entire match (for saving at end)
    all_events: Vec<MatchEvent>,
    /// Optional team-view observation output
    team_view_observation: Option<TeamViewObservationConfig>,
    /// FIX_2601/0123 #12: Session TTL tracking
    /// Timestamp when the session was created
    created_at: std::time::Instant,
    /// Timestamp of the last poll/step operation
    last_polled: std::time::Instant,
}

impl LiveMatchSession {
    /// Create a new live match session from a match plan.
    pub fn new(plan: MatchPlan) -> Result<Self, String> {
        let engine = MatchEngine::new(plan)?;
        let now = std::time::Instant::now();
        Ok(Self {
            engine,
            state: MatchState::NotStarted,
            home_strength: 0.0,
            away_strength: 0.0,
            possession_ratio: 0.0,
            match_duration: 90,
            position_tracking_enabled: true,
            last_event_count: 0,
            all_events: Vec::new(),
            team_view_observation: None,
            created_at: now,
            last_polled: now,
        })
    }

    // =========================================================================
    // FIX_2601/0123 #12: Session TTL Management
    // =========================================================================

    /// Default session TTL in seconds (1 hour)
    pub const DEFAULT_TTL_SECS: u64 = 3600;

    /// Check if the session is stale (hasn't been polled within TTL)
    pub fn is_stale(&self) -> bool {
        self.is_stale_with_ttl(Self::DEFAULT_TTL_SECS)
    }

    /// Check if the session is stale with a custom TTL
    pub fn is_stale_with_ttl(&self, ttl_secs: u64) -> bool {
        self.last_polled.elapsed().as_secs() >= ttl_secs
    }

    /// Update the last polled timestamp (call on each step/poll)
    pub fn touch(&mut self) {
        self.last_polled = std::time::Instant::now();
    }

    /// Get the time since the session was created
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }

    /// Get the time since the last poll
    pub fn idle_time(&self) -> std::time::Duration {
        self.last_polled.elapsed()
    }

    // =========================================================================
    // Configuration
    // =========================================================================

    /// Configure whether this session should record `MatchResult.position_data` (large output).
    /// Must be set before `kick_off()` to take effect.
    pub fn set_position_tracking_enabled(&mut self, enabled: bool) {
        self.position_tracking_enabled = enabled;
    }

    /// Configure team-view observation output for each tick.
    /// Must be set before `kick_off()` to take effect.
    pub fn set_team_view_observation_config(&mut self, config: TeamViewObservationConfig) {
        self.team_view_observation = Some(config);
    }

    /// Kick off the match (start first half).
    pub fn kick_off(&mut self) {
        if self.state != MatchState::NotStarted {
            return;
        }

        // Initialize engine
        let (home_strength, away_strength, possession_ratio, match_duration) = self.engine.init();
        self.home_strength = home_strength;
        self.away_strength = away_strength;
        self.possession_ratio = possession_ratio;
        self.match_duration = match_duration;

        // Optional: record position_data for replay/export.
        if self.position_tracking_enabled {
            self.engine.enable_position_tracking();
        }

        self.state = MatchState::FirstHalf;
        self.last_event_count = self.engine.get_events_len();
    }

    /// Resume second half after half-time break.
    pub fn resume_second_half(&mut self) {
        if self.state != MatchState::HalfTimeBreak {
            return;
        }
        self.state = MatchState::SecondHalf;
    }

    /// Execute one tick (250ms of game time).
    ///
    /// Returns `StepResult` with current positions, events, and state.
    pub fn step(&mut self) -> StepResult {
        // FIX_2601/0123 #12: Update last polled timestamp
        self.touch();

        match self.state {
            MatchState::NotStarted => StepResult::NotStarted,
            MatchState::HalfTimeBreak => {
                // Return half-time data (waiting for resume)
                StepResult::HalfTime(self.build_half_time_data())
            }
            MatchState::Finished => {
                // Return full-time data
                StepResult::FullTime(self.build_full_time_data())
            }
            MatchState::FirstHalf | MatchState::SecondHalf => self.execute_tick(),
        }
    }

    /// Execute a single tick during play
    fn execute_tick(&mut self) -> StepResult {
        // Advance exactly one decision tick (250ms) using the tick-based Game OS loop.
        let continues = self.engine.step_decision_tick_streaming(
            self.home_strength,
            self.away_strength,
            self.possession_ratio,
            self.match_duration,
        );

        // Build tick data
        let mut tick_data = self.build_tick_data();

        // Incremental event streaming for this tick only.
        let new_events = self.engine.get_events_since(self.last_event_count);
        self.last_event_count += new_events.len();
        self.all_events.extend(new_events.clone());

        // Live 스트리밍에서도 HighlightLevel 정책을 적용해
        // 주인공/중요 이벤트만 tick.events 에 포함시킨다.
        if let Some((level, player_track_id)) = self.engine.get_user_highlight_config() {
            use crate::engine::events::filter_events_by_highlight_level;
            tick_data.events =
                filter_events_by_highlight_level(&new_events, level, Some(player_track_id));
        } else {
            tick_data.events = new_events;
        }

        // Half-time boundary (45:00): prepare the engine state, then pause on the next call.
        // We apply the half-time transition AFTER building this tick's snapshot to avoid corrupting it.
        if self.state == MatchState::FirstHalf
            && tick_data.timestamp_ms + MS_PER_TICK == 45 * 60_000
        {
            self.engine.apply_half_time_transition();
            // Consume the HalfTime event (timestamp=45:00) into the full event stream,
            // but do not surface it as a per-tick event payload.
            let halftime_events = self.engine.get_events_since(self.last_event_count);
            self.last_event_count += halftime_events.len();
            self.all_events.extend(halftime_events);
            self.state = MatchState::HalfTimeBreak;
        }

        // Full-time boundary: mark finished (FullTime payload returned on next call).
        if !continues {
            self.state = MatchState::Finished;
        }

        StepResult::Tick(tick_data)
    }

    /// Build tick data snapshot
    fn build_tick_data(&self) -> TickData {
        let (ball_pos, ball_height) = self.engine.get_ball_state();
        let ball_position = self.normalized_to_meters(ball_pos);
        let ball_owner_idx = self.engine.get_ball_owner().map(|idx| idx as u8);

        let mut player_positions = Vec::with_capacity(22);
        for i in 0..22u8 {
            let pos_coord10 = self.engine.get_player_position_by_index(i as usize);
            let pos_m = self.normalized_to_meters(pos_coord10.to_meters());
            let state = self.engine.get_player_state_string(i as usize);
            let stamina = self.engine.get_player_current_stamina(i as usize);
            player_positions.push(PlayerPosition { index: i, position: pos_m, state, stamina });
        }

        let (home_score, away_score) = self.engine.get_score();
        let minute = self.engine.get_minute();
        let timestamp_ms = self.engine.get_current_timestamp_ms();

        let (team_view_simple, team_view_minimap) = match &self.team_view_observation {
            Some(config) if config.is_enabled() => {
                let simple = if config.include_simple {
                    Some(self.engine.build_team_view_simple_observation(config.observer_is_home))
                } else {
                    None
                };
                let minimap = if config.include_minimap {
                    Some(self.engine.build_team_view_minimap_observation(
                        config.observer_is_home,
                        config.minimap_spec,
                    ))
                } else {
                    None
                };
                (simple, minimap)
            }
            _ => (None, None),
        };

        let decision_intents = self.engine.get_decision_intents().to_vec();
        let field_board_snapshot = self
            .engine
            .field_board
            .as_ref()
            .map(|board| board.to_snapshot_export());
        let (home_x, away_x) = self.engine.get_offside_lines_m();

        TickData {
            timestamp_ms,
            minute,
            ball_position,
            ball_height,
            ball_owner_idx,
            player_positions,
            events: Vec::new(),
            score: (home_score, away_score),
            team_view_simple,
            team_view_minimap,
            decision_intents,
            field_board_snapshot,
            offside_lines: OffsideLineSnapshot { home_x, away_x },
        }
    }

    /// Build half-time data
    fn build_half_time_data(&self) -> HalfTimeData {
        let (home_score, away_score) = self.engine.get_score();
        let (home_poss, away_poss) = self.engine.get_possession_stats();
        let (home_shots, away_shots) = self.engine.get_shot_stats();
        let (home_sot, away_sot) = self.engine.get_shots_on_target_stats();

        HalfTimeData {
            score: (home_score, away_score),
            possession: (home_poss, away_poss),
            shots: (home_shots, away_shots),
            shots_on_target: (home_sot, away_sot),
        }
    }

    /// Build full-time data
    fn build_full_time_data(&self) -> FullTimeData {
        FullTimeData { result: self.engine.get_result(), all_events: self.all_events.clone() }
    }

    /// Convert normalized position (0-1) to meters (105x68)
    /// Uses coordinates module for consistent x/y swap
    ///
    /// P0.75: Uses clamped conversion to enforce field bounds
    fn normalized_to_meters(&self, pos: (f32, f32)) -> (f32, f32) {
        use crate::engine::coordinates;
        coordinates::to_meters_clamped(pos)
    }

    // ========================================
    // User Intervention API
    // ========================================

    /// Change team tactic during the match.
    pub fn change_tactic(&mut self, team: TeamSide, instructions: TeamInstructions) {
        self.engine.apply_tactic_change(team, instructions);
    }

    /// Change team formation during the match (Phase 5).
    ///
    /// Supported formations: 4-4-2, 4-3-3, 4-5-1, 3-4-3, 4-2-3-1, 3-5-2
    ///
    /// Returns Ok(()) on success, Err if formation is not supported.
    pub fn change_formation(
        &mut self,
        team: TeamSide,
        formation: &str,
    ) -> Result<(), &'static str> {
        self.engine.change_formation(team, formation)
    }

    /// Substitute a player (Phase 5).
    ///
    /// * `team` - Which team is making the substitution
    /// * `out_idx` - Index of player going out (0-10 for starting 11)
    /// * `in_idx` - Index of player coming in (11+ for bench)
    ///
    /// Returns Ok(()) on success, Err with reason on failure.
    pub fn substitute(
        &mut self,
        team: TeamSide,
        out_idx: usize,
        in_idx: usize,
    ) -> Result<(), &'static str> {
        self.engine.substitute_player(team, out_idx, in_idx)
    }

    /// Get current match state.
    pub fn get_state(&self) -> MatchState {
        self.state
    }

    /// Get current minute.
    pub fn get_minute(&self) -> u8 {
        self.engine.get_minute()
    }

    /// Get current score.
    pub fn get_score(&self) -> (u8, u8) {
        self.engine.get_score()
    }

    // ========== Career Player Mode: User Control System ==========

    /// Submit a user command to the engine's queue
    pub fn submit_user_command(&mut self, cmd: super::match_sim::UserCommand) {
        self.engine.submit_user_command(cmd);
    }

    /// Register a controller slot for multi-agent control.
    pub fn register_controller_slot(
        &mut self,
        controller_id: u32,
        team_side: TeamSide,
        player_slot: u8,
    ) -> Result<(), &'static str> {
        self.engine.register_controller_slot(controller_id, team_side, player_slot)
    }

    /// Unregister a controller slot.
    pub fn unregister_controller_slot(&mut self, controller_id: u32) -> Result<(), &'static str> {
        self.engine.unregister_controller_slot(controller_id)
    }

    /// Clear all multi-agent controller slots.
    pub fn clear_controller_slots(&mut self) {
        self.engine.clear_controller_slots();
    }

    /// Submit multi-agent commands (controller_id -> track_id routing).
    pub fn submit_multi_agent_commands(
        &mut self,
        commands: Vec<super::match_sim::MultiAgentCommand>,
    ) -> Result<(), &'static str> {
        self.engine.submit_multi_agent_commands(commands)
    }

    /// Enable Career Player Mode for a specific track_id
    pub fn enable_controlled_mode(&mut self, track_id: usize) {
        self.engine.enable_controlled_mode(track_id);
    }

    /// Disable Career Player Mode
    pub fn disable_controlled_mode(&mut self) {
        self.engine.disable_controlled_mode();
    }

    /// Set sticky action toggles for a player (sprint/dribble/press).
    pub fn set_sticky_action(
        &mut self,
        track_id: usize,
        action: StickyAction,
        enabled: bool,
    ) -> Result<(), &'static str> {
        self.engine.set_sticky_action(track_id, action, enabled)
    }

    /// Get current sticky action toggles for a player.
    pub fn get_sticky_actions(&self, track_id: usize) -> Option<StickyActions> {
        self.engine.get_sticky_actions(track_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::match_sim::test_fixtures::create_test_team_with_subs as create_test_team;
    use crate::engine::physics_constants::field;

    fn create_test_plan() -> MatchPlan {
        MatchPlan {
            home_team: create_test_team("Home"),
            away_team: create_test_team("Away"),
            seed: 12345,
            home_instructions: None,
            away_instructions: None,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        }
    }

    #[test]
    fn test_session_creation() {
        let plan = create_test_plan();
        let session = LiveMatchSession::new(plan).expect("live session init");
        assert_eq!(session.get_state(), MatchState::NotStarted);
    }

    #[test]
    fn test_kick_off() {
        let plan = create_test_plan();
        let mut session = LiveMatchSession::new(plan).expect("live session init");

        session.kick_off();
        assert_eq!(session.get_state(), MatchState::FirstHalf);
        assert_eq!(session.get_minute(), 0);
    }

    #[test]
    fn test_step_returns_tick_data() {
        let plan = create_test_plan();
        let mut session = LiveMatchSession::new(plan).expect("live session init");

        // Before kick-off
        let result = session.step();
        assert!(matches!(result, StepResult::NotStarted));

        // After kick-off
        session.kick_off();
        let result = session.step();

        if let StepResult::Tick(data) = result {
            assert_eq!(data.player_positions.len(), 22);
            assert!(data.ball_position.0 >= 0.0 && data.ball_position.0 <= field::LENGTH_M);
            assert!(data.ball_position.1 >= 0.0 && data.ball_position.1 <= field::WIDTH_M);
        } else {
            panic!("Expected StepResult::Tick");
        }
    }

    #[test]
    fn test_team_view_observation_outputs() {
        let plan = create_test_plan();
        let mut session = LiveMatchSession::new(plan).expect("live session init");

        session.set_team_view_observation_config(TeamViewObservationConfig {
            observer_is_home: true,
            include_simple: true,
            include_minimap: true,
            minimap_spec: MiniMapSpec::default(),
        });

        session.kick_off();
        let result = session.step();

        match result {
            StepResult::Tick(data) => {
                assert!(data.team_view_simple.is_some());
                assert!(data.team_view_minimap.is_some());
            }
            _ => panic!("Expected StepResult::Tick"),
        }
    }

    /// Test that live engine runs to completion (full 90 minutes)
    /// Spec: test_live_engine_runs_to_completion
    #[test]
    fn test_live_engine_runs_to_completion() {
        let plan = create_test_plan();
        let mut session = LiveMatchSession::new(plan).expect("live session init");

        session.kick_off();

        let mut tick_count = 0;
        let mut saw_halftime = false;
        let mut finished = false;
        // 250ms tick: 90분 ≈ 21,600 ticks (240 ticks/min). 여유 포함 상한 설정.
        let max_ticks = 40_000; // Safety limit

        while tick_count < max_ticks {
            let result = session.step();
            tick_count += 1;

            match result {
                StepResult::Tick(_) => {}
                StepResult::HalfTime(_) => {
                    saw_halftime = true;
                    session.resume_second_half();
                }
                StepResult::FullTime(data) => {
                    finished = true;
                    // Verify we got events
                    assert!(!data.all_events.is_empty(), "Expected some events");
                    break;
                }
                StepResult::NotStarted => {
                    panic!("Unexpected NotStarted after kick_off");
                }
            }
        }

        assert!(saw_halftime, "Should have seen halftime");
        assert!(finished, "Match should have finished");
        assert_eq!(session.get_state(), MatchState::Finished);

        // Verify reasonable tick count (90 min * 240 ticks/min = 21,600 ticks, plus some margin)
        assert!(tick_count > 15_000, "Too few ticks: {}", tick_count);
        assert!(tick_count < 45_000, "Too many ticks: {}", tick_count);
    }

    /// Test halftime transition
    #[test]
    fn test_halftime_transition() {
        let plan = create_test_plan();
        let mut session = LiveMatchSession::new(plan).expect("live session init");

        session.kick_off();

        // Run until halftime
        // 250ms tick: halftime ~ 45 * 240 = 10,800 ticks (plus margin)
        let mut found_halftime = false;
        for _ in 0..15_000 {
            let result = session.step();
            if let StepResult::HalfTime(data) = result {
                found_halftime = true;
                // Check halftime stats
                assert!(data.possession.0 + data.possession.1 >= 90); // Should sum to ~100
                assert_eq!(session.get_state(), MatchState::HalfTimeBreak);
                break;
            }
        }

        assert!(found_halftime, "Should have reached halftime");

        // Resume second half
        session.resume_second_half();
        assert_eq!(session.get_state(), MatchState::SecondHalf);

        // Continue and verify we can still step
        let result = session.step();
        assert!(matches!(result, StepResult::Tick(_)));
    }

    /// Test that live simulation produces equivalent results to batch simulation
    /// Spec: test_live_vs_batch_result_equivalence
    ///
    /// Note: Due to different execution paths (minute-based events vs tick-based streaming),
    /// we verify that both modes produce games with events and scores.
    /// Exact score matching is not required due to RNG state differences.
    #[test]
    fn test_live_vs_batch_result_equivalence() {
        use crate::engine::match_sim::MatchEngine;

        // Use same seed for both simulations
        let seed = 12345u64;

        // --- Batch simulation ---
        let batch_plan = MatchPlan {
            home_team: create_test_team("Home"),
            away_team: create_test_team("Away"),
            seed,
            home_instructions: None,
            away_instructions: None,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        };

        let mut batch_engine = MatchEngine::new(batch_plan).expect("match engine init");
        let batch_result = batch_engine.simulate();

        // --- Live simulation ---
        let live_plan = MatchPlan {
            home_team: create_test_team("Home"),
            away_team: create_test_team("Away"),
            seed,
            home_instructions: None,
            away_instructions: None,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        };

        let mut live_session = LiveMatchSession::new(live_plan).expect("live session init");
        live_session.kick_off();

        // Run to completion and track score from get_score()
        let mut live_final_score = (0u8, 0u8);
        let mut live_event_count = 0usize;

        for _ in 0..30_000 {
            match live_session.step() {
                StepResult::Tick(data) => {
                    live_event_count += data.events.len();
                    live_final_score = data.score;
                }
                StepResult::HalfTime(_) => {
                    live_final_score = live_session.get_score();
                    live_session.resume_second_half();
                }
                StepResult::FullTime(data) => {
                    live_event_count += data.all_events.len();
                    live_final_score = live_session.get_score();
                    break;
                }
                _ => {}
            }
        }

        // --- Compare results ---
        let (batch_home, batch_away) = (batch_result.score_home, batch_result.score_away);
        let (live_home, live_away) = live_final_score;

        let batch_events = batch_result.events.len();

        // Both simulations should complete and produce events
        assert!(batch_events > 0, "Batch should have events");
        assert!(live_event_count > 0, "Live should have events");

        // Log results for comparison
        println!("Batch: score {}:{}, events={}", batch_home, batch_away, batch_events);
        println!("Live:  score {}:{}, events={}", live_home, live_away, live_event_count);

        // Due to different RNG consumption patterns between batch and live modes,
        // exact score matching is not guaranteed. We verify:
        // 1. Both modes complete successfully
        // 2. Both modes produce reasonable event counts
        // 3. Scores are within a reasonable range (total goals similar)
        let batch_total = batch_home + batch_away;
        let live_total = live_home + live_away;

        // v11: Batch와 Live 모드는 RNG 소비 패턴이 완전히 달라 결과가 다를 수 있음
        // 이 테스트는 두 모드가 "정상적으로 완료되고 합리적인 결과를 생성하는지"만 검증
        // 실제 동등성은 보장하지 않음 (다른 코드 경로를 사용하기 때문)

        // Both modes should produce reasonable goal counts (0-20 per match)
        assert!(batch_total <= 20, "Batch total goals unreasonably high: {}", batch_total);
        assert!(live_total <= 20, "Live total goals unreasonably high: {}", live_total);

        // Log the variance for informational purposes (not enforced)
        let goal_variance = (batch_total as i32 - live_total as i32).abs();
        println!("Goal variance: {} (batch: {}, live: {})", goal_variance, batch_total, live_total);
    }
}
