//! State Accessors for MatchEngine
//!
//! This module contains getter methods and state access functions for MatchEngine.
//! Extracted from mod.rs as part of P2-9 refactoring.
//!
//! ## Functions
//! - `get_minute()`, `get_score()`, `get_current_timestamp_ms()`
//! - `get_ball_state()`, `get_player_state_string()`
//! - `get_recent_events()`, `get_user_highlight_config()`
//! - `get_possession_stats()`, `get_shot_stats()`, `get_shots_on_target_stats()`
//! - `get_result()`
//! - `enable_position_tracking()`, `update_positions_for_tick()`
//! - `apply_tactic_change()`, `substitute_player()`

use crate::models::MatchResult;
use crate::models::replay::types::DecisionIntent;
use std::path::Path;

use super::MatchEngine;

// ============================================
// Phase 7: Live Match Streaming Helper Methods
// ============================================

impl MatchEngine {
    /// Enable position tracking after initialization
    pub fn enable_position_tracking(&mut self) {
        self.track_positions = true;
        if self.result.position_data.is_none() {
            self.result = MatchResult::with_replay_events();
        }
    }

    /// Get current minute
    pub fn get_minute(&self) -> u8 {
        self.minute
    }

    /// Get current score (home, away)
    pub fn get_score(&self) -> (u8, u8) {
        (self.result.score_home, self.result.score_away)
    }

    /// CI Gate helper: force a score state for integration tests.
    ///
    /// This intentionally updates both `result` and `match_situation` to keep
    /// engine-side "score-dependent" logic consistent (audacity, etc.).
    #[doc(hidden)]
    pub fn ci_gate_set_score(&mut self, score_home: u8, score_away: u8) {
        self.result.score_home = score_home;
        self.result.score_away = score_away;
        self.match_situation.update_score(score_home, score_away);
    }

    /// Get current timestamp in milliseconds
    pub fn get_current_timestamp_ms(&self) -> u64 {
        self.current_timestamp_ms
    }

    /// Total number of events emitted so far (for incremental streaming).
    pub fn get_events_len(&self) -> usize {
        self.result.events.len()
    }

    /// Return events emitted since the given cursor (for incremental streaming).
    pub fn get_events_since(&self, start: usize) -> Vec<crate::models::MatchEvent> {
        self.result.events.get(start..).unwrap_or(&[]).to_vec()
    }

    /// Apply half-time transition (side swap + kickoff reset).
    ///
    /// This is primarily used by the session/streaming API which steps the match per tick.
    pub fn apply_half_time_transition(&mut self) {
        self.handle_half_time();
    }

    /// Get remaining transition window time (ms), if active.
    ///
    /// - `None` means TransitionSystem is inactive.
    /// - When active, this counts down from 3000ms in 250ms steps (decision tick cadence).
    pub fn get_transition_remaining_ms(&self) -> Option<u32> {
        self.transition_system.state().remaining_ms()
    }

    /// Get decision intents recorded in the current decision tick.
    pub fn get_decision_intents(&self) -> &[DecisionIntent] {
        &self.decision_intents
    }

    /// Get current offside lines in meters (home, away).
    pub fn get_offside_lines_m(&self) -> (f32, f32) {
        (
            self.positioning_engine.get_offside_line(true),
            self.positioning_engine.get_offside_line(false),
        )
    }

    /// Get ball state (position normalized in legacy format, height in meters)
    /// FIX_2601/0104: Use to_normalized_legacy() to match NormalizedPos (width, length) format
    pub fn get_ball_state(&self) -> ((f32, f32), f32) {
        let pos_norm = self.ball.position.to_normalized_legacy();
        let height_m = self.ball.height as f32 / 10.0;
        (pos_norm, height_m)
    }

    /// Get ball owner index (None if loose ball)
    pub fn get_ball_owner(&self) -> Option<usize> {
        self.ball.current_owner
    }

    /// Get player current stamina (0.0 = exhausted, 1.0 = fresh)
    pub fn get_player_current_stamina(&self, player_idx: usize) -> f32 {
        if player_idx < self.player_fatigue.len() {
            // fatigue is 0.0 (fresh) to 1.0 (exhausted), invert for stamina
            (1.0 - self.player_fatigue[player_idx]).clamp(0.0, 1.0)
        } else {
            1.0 // Default to fresh if index out of bounds
        }
    }

    /// Get player state as string (public accessor)
    /// FIX_2601/0109: Converts PlayerState enum to String for API compatibility
    pub fn get_player_state_string(&self, player_idx: usize) -> String {
        self.get_player_state(player_idx).to_string()
    }

    /// Get recent events from the match result
    pub fn get_recent_events(&self) -> Vec<crate::models::MatchEvent> {
        // Return events from the current/last minute
        let min_time = if self.minute > 0 { self.minute - 1 } else { 0 };
        self.result.events.iter().filter(|e| e.minute >= min_time).cloned().collect()
    }

    /// Get user highlight configuration (level + player track_id), if any.
    pub fn get_user_highlight_config(&self) -> Option<(super::super::HighlightLevel, u8)> {
        self.user_player.as_ref().map(|cfg| (cfg.highlight_level, cfg.player_index as u8))
    }

    /// Get possession statistics (home%, away%)
    pub fn get_possession_stats(&self) -> (u8, u8) {
        // Simple approximation from possession_ratio
        let home_poss = (self.precomputed_possession_ratio * 100.0) as u8;
        let away_poss = 100 - home_poss;
        (home_poss, away_poss)
    }

    /// Get shot statistics (home, away)
    pub fn get_shot_stats(&self) -> (u8, u8) {
        use crate::models::EventType;
        let home_shots = self
            .result
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e.event_type,
                    EventType::Shot
                        | EventType::ShotOnTarget
                        | EventType::ShotOffTarget
                        | EventType::ShotBlocked
                ) && e.is_home_team
            })
            .count() as u8;
        let away_shots = self
            .result
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e.event_type,
                    EventType::Shot
                        | EventType::ShotOnTarget
                        | EventType::ShotOffTarget
                        | EventType::ShotBlocked
                ) && !e.is_home_team
            })
            .count() as u8;
        (home_shots, away_shots)
    }

    /// Get shots on target statistics (home, away)
    pub fn get_shots_on_target_stats(&self) -> (u8, u8) {
        use crate::models::EventType;
        let home_sot = self
            .result
            .events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::ShotOnTarget) && e.is_home_team)
            .count() as u8;
        let away_sot = self
            .result
            .events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::ShotOnTarget) && !e.is_home_team)
            .count() as u8;
        (home_sot, away_sot)
    }

    /// Get the final match result
    pub fn get_result(&self) -> crate::models::MatchResult {
        self.result.clone()
    }

    /// Phase0 diagnostics report (structured).
    pub fn get_diagnostics_report(&self) -> super::balance_diagnostics::DiagnosticReport {
        self.balance_diagnostics.generate_report()
    }

    /// Build a trace dump (MatchResult + diagnostics + optional analysis).
    pub fn build_trace_dump(&self, include_analysis: bool) -> crate::engine::TraceDump {
        crate::engine::TraceDump::new(
            self.result.clone(),
            self.get_diagnostics_report(),
            include_analysis,
        )
    }

    /// Write a trace dump JSON to the given path.
    pub fn write_trace_dump(
        &self,
        path: impl AsRef<Path>,
        include_analysis: bool,
    ) -> Result<(), String> {
        let dump = self.build_trace_dump(include_analysis);
        dump.write_json(path)
    }

    /// Update positions for a single tick (for live streaming)
    pub fn update_positions_for_tick(&mut self) {
        // Standard tick: 250ms (4 ticks/sec) to match tick-based simulation.
        const TICK_DURATION_SEC: f32 = 0.25;
        if self.track_positions {
            self.update_player_positions_for_tick(TICK_DURATION_SEC);
        }
        self.update_reaction_states();
    }

    /// Apply a tactic change during the match
    pub fn apply_tactic_change(
        &mut self,
        team: crate::engine::tactical_context::TeamSide,
        new_instructions: crate::tactics::TeamInstructions,
    ) {
        use crate::engine::tactical_context::TeamSide;

        match team {
            TeamSide::Home => self.home_instructions = new_instructions,
            TeamSide::Away => self.away_instructions = new_instructions,
        };
    }

    /// Change formation during the match (Phase 5)
    ///
    /// Supported formations: 4-4-2, 4-3-3, 4-5-1, 3-4-3, 4-2-3-1, 3-5-2
    ///
    /// Returns Ok(()) on success, Err if formation is not supported.
    pub fn change_formation(
        &mut self,
        team: crate::engine::tactical_context::TeamSide,
        formation: &str,
    ) -> Result<(), &'static str> {
        use crate::engine::tactical_context::TeamSide;

        // Validate formation
        let supported = ["4-4-2", "4-3-3", "4-5-1", "3-4-3", "4-2-3-1", "3-5-2"];
        if !supported.contains(&formation) {
            return Err("Unsupported formation");
        }

        // Update formation string
        match team {
            TeamSide::Home => {
                println!(
                    "[{}min] User changed formation: Home {} -> {}",
                    self.minute, self.home_formation, formation
                );
                self.home_formation = formation.to_string();
            }
            TeamSide::Away => {
                println!(
                    "[{}min] User changed formation: Away {} -> {}",
                    self.minute, self.away_formation, formation
                );
                self.away_formation = formation.to_string();
            }
        }

        Ok(())
    }

    /// Substitute a player (Phase 5)
    ///
    /// * `team` - Which team is making the substitution
    /// * `out_idx` - Index of player going out (0-10 for starting 11)
    /// * `in_idx` - Index of player coming in (11+ for bench)
    ///
    /// Returns Ok(()) on success, Err with reason on failure.
    pub fn substitute_player(
        &mut self,
        team: crate::engine::tactical_context::TeamSide,
        out_idx: usize,
        in_idx: usize,
    ) -> Result<(), &'static str> {
        use crate::engine::tactical_context::TeamSide;
        use crate::engine::player_state::PlayerState;

        // Validate indices first
        if out_idx > 10 {
            return Err("out_idx must be 0-10 (starting player)");
        }
        if in_idx < 11 {
            return Err("in_idx must be 11+ (bench player)");
        }

        let global_out_idx = match team {
            TeamSide::Home => out_idx,
            TeamSide::Away => 11 + out_idx,
        };
        if matches!(self.player_states.get(global_out_idx), Some(PlayerState::SentOff)) {
            return Err("Cannot substitute a sent-off player");
        }

        // Check substitution limits (max 5)
        let (home_subs, away_subs) = self.substitutions_made;
        let current_subs = match team {
            TeamSide::Home => home_subs,
            TeamSide::Away => away_subs,
        };

        if current_subs >= 5 {
            return Err("Maximum substitutions reached (5)");
        }

        // FIX_2601/0106 P1: SSOT substitution roster swap via MatchSetup.
        // `in_idx` is a bench index (11+) from the external API; convert to a bench_slot (0..6).
        use crate::models::match_setup::MAX_SUBSTITUTES;

        let bench_slot_usize = in_idx - 11;
        if bench_slot_usize >= MAX_SUBSTITUTES {
            return Err("in_idx must be within the first 7 bench slots (11-17)");
        }
        let bench_slot = bench_slot_usize as u8;
        if self.setup.is_sub_used(team, bench_slot) {
            return Err("Substitute already used (cannot re-enter)");
        }

        let subs_len = match team {
            TeamSide::Home => self.setup.home.substitutes.len(),
            TeamSide::Away => self.setup.away.substitutes.len(),
        };
        if bench_slot_usize >= subs_len {
            return Err("in_idx exceeds available substitutes");
        }

        let pitch_track_id = match team {
            TeamSide::Home => out_idx,
            TeamSide::Away => 11 + out_idx,
        };
        self.execute_substitution(pitch_track_id, bench_slot, matches!(team, TeamSide::Home));

        Ok(())
    }

    // ========== P7 Phase-Based Action: PlayerState Accessors ==========

    /// P7: 선수가 새 액션을 시작할 수 있는지 확인
    ///
    /// PlayerState가 Idle/Moving이고, 특정 액션에 대한 쿨다운이 없어야 함
    pub fn can_start_action_for_queue(
        &self,
        player_idx: usize,
        action_type: &crate::engine::action_queue::ActionType,
    ) -> bool {
        use crate::engine::action_queue::ActionType;

        if player_idx >= self.player_states.len() {
            return false;
        }

        let state = &self.player_states[player_idx];

        // 기본 상태 체크
        if !state.can_start_action() {
            return false;
        }

        // 액션별 추가 체크
        match action_type {
            ActionType::Tackle { .. } => {
                // 태클 쿨다운 체크
                if player_idx < self.tackle_cooldowns.len() && self.tackle_cooldowns[player_idx] > 0
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
    }

    /// P7: 액션 시작 시 호출 - PlayerState를 InAction으로 전환
    pub fn on_action_started(&mut self, player_idx: usize, action_id: u64) {
        if player_idx < self.player_states.len() {
            self.player_states[player_idx].start_action(action_id);
        }
    }

    /// P7: 액션 완료 시 호출 - PlayerState를 Idle/Recovering/Cooldown으로 전환
    pub fn on_action_finished(
        &mut self,
        player_idx: usize,
        // P0: Core types moved to action_queue
        action_type: crate::engine::action_queue::PhaseActionType,
        recovery_ticks: u8,
        cooldown_ticks: u8,
    ) {
        use crate::engine::action_queue::PhaseActionType;

        if player_idx >= self.player_states.len() {
            return;
        }

        // 쿨다운이 있는 액션
        if cooldown_ticks > 0 {
            self.player_states[player_idx].finish_action_cooldown(action_type, cooldown_ticks);

            // 태클은 별도 쿨다운 배열도 업데이트
            if action_type == PhaseActionType::Tackle && player_idx < self.tackle_cooldowns.len() {
                self.tackle_cooldowns[player_idx] = cooldown_ticks;
            }
        } else if recovery_ticks > 0 {
            self.player_states[player_idx].finish_action_recovering(recovery_ticks);
        } else {
            self.player_states[player_idx].finish_action_idle();
        }
    }

    // Note: update_player_states_tick() is defined in tick_based.rs

    /// P7: 특정 선수의 PlayerState 반환
    pub fn get_player_fsm_state(
        &self,
        player_idx: usize,
    ) -> Option<&super::super::player_state::PlayerState> {
        self.player_states.get(player_idx)
    }

    /// FIX_2601/0117: Debug method to get pass score for target analysis
    #[doc(hidden)]
    pub fn debug_pass_score_6factor(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> f32 {
        self.calculate_pass_score_6factor(passer_idx, receiver_idx, is_home)
    }

    /// FIX_2601/0117: Debug method to get all 6 factors for a pass
    #[doc(hidden)]
    pub fn debug_pass_factors(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let distance = self.calculate_pass_distance_factor(passer_idx, receiver_idx);
        let safety = self.calculate_pass_safety_factor(passer_idx, receiver_idx, is_home);
        let readiness = self.calculate_pass_readiness_factor(receiver_idx, is_home);
        let progression = self.calculate_pass_progression_factor(passer_idx, receiver_idx, is_home);
        let progression_delta =
            self.calculate_pass_progression_delta(passer_idx, receiver_idx, is_home);
        let space = self.calculate_pass_space_factor(receiver_idx, is_home);
        let tactical = self.calculate_pass_tactical_factor(passer_idx, receiver_idx, is_home);
        (distance, safety, readiness, progression, progression_delta, space, tactical)
    }
}
