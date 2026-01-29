//! Simulation Logic for MatchEngine
//!
//! This module contains core simulation functions for match execution.
//! Extracted from mod.rs as part of P2-9 refactoring.
//!
//! ## Functions
//! - `simulate_minute()` - Simulate a single minute of match (tick-based only)
//! - `simulate_attack()` - Simulate attack sequence
//! - `score_goal()` - Process goal scoring
//! - `simulate_other_events()` - Handle cards, injuries, substitutions
//!
//! ## 2025-12-11: Legacy Cleanup
//! - Removed minute-based legacy simulation
//! - Only tick-based engine (240 ticks/min) is now used
//! - See tick_based.rs for the main simulation loop

use rand::Rng;

use super::MatchEngine;
use crate::engine::action_queue::{ActionResult, RestartType};
use crate::engine::debug_flags::match_debug_enabled;
use crate::engine::physics_constants::field;
use crate::models::{MatchEvent, TeamSide};

impl MatchEngine {
    /// Simulate a single minute of match (tick-based only)
    ///
    /// 2025-12-11: Legacy minute-based simulation removed.
    /// All simulation now goes through tick_based.rs (240 ticks/min, 4 ticks/sec)
    pub(crate) fn simulate_minute(
        &mut self,
        home_strength: f32,
        away_strength: f32,
        possession_ratio: f32,
    ) {
        // Debug: unconditional print to confirm this function is called
        if self.minute == 0 {
            #[cfg(debug_assertions)]
            if match_debug_enabled() {
                println!(">>> [DEBUG-0105] simulate_minute CALLED for minute 0 <<<");
            }
            #[cfg(debug_assertions)]
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }

        // Tick-based simulation only (legacy removed 2025-12-11)
        self.simulate_minute_tick_based(home_strength, away_strength, possession_ratio);
    }

    // NOTE: simulate_attack() removed (2026-01-05) - legacy minute-based code, never called
    // Tick-based simulation uses ActionResult handlers instead

    /// Process goal scoring
    /// C6: Changed parameter from scorer: String to scorer_idx: usize
    ///
    /// CRITICAL: Score update MUST happen BEFORE event emission.
    /// If budget exhaustion occurs between these operations, we need the score
    /// to be consistent with events. By updating score first, even if emit_event
    /// fails or budget is exceeded afterward, the score remains correct.
    /// finalize() will validate score == goal_event_count as a safety net.
    pub(crate) fn score_goal(&mut self, is_home: bool, scorer_idx: usize) {
        // FIX_2601/0116: Prevent double-counting by check_goals_from_ball_position
        // Without this flag, the same goal could be detected twice with different scorers
        self.goal_scored_this_tick = true;

        // ========== ATOMIC FIX: Score update FIRST ==========
        // Update score before event emission to prevent inconsistency
        // if budget exhaustion occurs between these operations.
        if is_home {
            self.result.score_home += 1;
        } else {
            self.result.score_away += 1;
        }

        // Potential assist
        let assist_idx = if self.rng.gen::<f32>() < 0.6 {
            if is_home {
                Some(self.select_assister_home(scorer_idx))
            } else {
                Some(self.select_assister_away(scorer_idx))
            }
        } else {
            None
        };

        // 골 이벤트 생성 (3D 위치 포함 - 3D with height) (P2: auto timestamp)
        // C6: Use track_id directly
        let (ball_x_m, ball_y_m) = self.ball.position.to_meters();
        let ball_height_m = self.ball.height as f32 / 10.0;
        let event = MatchEvent::goal_with_position(
            self.minute,
            self.current_timestamp_ms(),
            is_home,
            scorer_idx,
            assist_idx,
            (ball_x_m / field::LENGTH_M, ball_y_m / field::WIDTH_M, ball_height_m),
        );
        self.emit_event(event);
    }

    /// Handle other events (cards, injuries, substitutions)
    pub(crate) fn simulate_other_events(&mut self) {
        // Yellow cards (1% chance per minute)
        if self.rng.gen::<f32>() < 0.01 {
            let is_home = self.rng.gen::<bool>();
            let player_idx = if is_home {
                self.select_random_player_home()
            } else {
                self.select_random_player_away()
            };

            // P2: auto timestamp via emit_event
            // C6: Use player_idx directly as track_id
            let (ball_x_m, ball_y_m) = self.ball.position.to_meters();
            let ball_height_m = self.ball.height as f32 / 10.0;
            self.emit_event(
                MatchEvent::yellow_card(
                    self.minute,
                    self.current_timestamp_ms(),
                    is_home,
                    player_idx,
                )
                .with_ball_position((
                    ball_x_m / field::LENGTH_M,
                    ball_y_m / field::WIDTH_M,
                    ball_height_m,
                )),
            );

            if is_home {
                self.result.statistics.yellow_cards_home += 1;
            } else {
                self.result.statistics.yellow_cards_away += 1;
            }
        }

        // Injuries (0.01% chance per minute = ~1% per match)
        if self.rng.gen::<f32>() < 0.0001 {
            let is_home = self.rng.gen::<bool>();
            let player_idx = if is_home {
                self.select_random_player_home()
            } else {
                self.select_random_player_away()
            };
            let weeks_out = self.rng.gen_range(1..=4);

            // P2: auto timestamp via emit_event
            // C6: Use player_idx directly as track_id
            let (ball_x_m, ball_y_m) = self.ball.position.to_meters();
            let ball_height_m = self.ball.height as f32 / 10.0;
            self.emit_event(
                MatchEvent::injury(
                    self.minute,
                    self.current_timestamp_ms(),
                    is_home,
                    player_idx,
                    weeks_out,
                )
                .with_ball_position((
                    ball_x_m / field::LENGTH_M,
                    ball_y_m / field::WIDTH_M,
                    ball_height_m,
                )),
            );

            if self.action_queue.ball_state().is_in_play() && !self.ball.is_in_flight {
                let last_touch_home = self
                    .ball
                    .current_owner
                    .or(self.ball.previous_owner)
                    .map(TeamSide::is_home)
                    .unwrap_or(true);
                self.handle_action_result(ActionResult::OutOfBounds {
                    restart_type: RestartType::DropBall,
                    position: self.ball.position,
                    home_team: last_touch_home,
                });
            }
        }

        // P3: Substitutions (max 5 per team, typically around 55-85 minutes)
        if self.minute >= 55 && self.minute <= 85 {
            self.process_substitutions();
        }
    }
}
