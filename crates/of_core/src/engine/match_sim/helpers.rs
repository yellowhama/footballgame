//! Helper Functions for MatchEngine
//!
//! This module contains utility and helper functions for MatchEngine.
//! Extracted from mod.rs as part of P2-9 refactoring.
//!
//! ## Functions
//! - `generate_user_player_involvement()` - Generate user player events
//! - `filter_events_for_display()` - Filter events by highlight level
//! - `get_player_position_by_index()` - Get player position
//! - `find_player_idx_by_name()` - Find player index by name

use rand::Rng;

use super::MatchEngine;
use crate::engine::physics_constants::field;
use crate::models::{EventType, MatchEvent};

impl MatchEngine {
    /// Generate user player involvement events
    pub(crate) fn generate_user_player_involvement(&mut self, team_has_ball: bool) {
        let user_config = match &self.user_player {
            Some(config) => config.clone(),
            None => return,
        };

        let user_team_has_ball = (user_config.is_home_team && team_has_ball)
            || (!user_config.is_home_team && !team_has_ball);

        // Always generate all possible user player events
        // They will be filtered later based on highlight level
        if user_team_has_ball {
            // 40% chance of user player involvement when team has ball
            if self.rng.gen::<f32>() < 0.4 {
                let action = self.rng.gen_range(0..100);
                let event = match action {
                    0..=40 => self.event_generator.generate_pass(
                        self.minute,
                        user_config.is_home_team,
                        user_config.player_name.clone(),
                    ),
                    41..=60 => self.event_generator.generate_dribble(
                        self.minute,
                        user_config.is_home_team,
                        user_config.player_name.clone(),
                    ),
                    61..=80 => self.event_generator.generate_key_chance(
                        self.minute,
                        user_config.is_home_team,
                        user_config.player_name.clone(),
                    ),
                    _ => {
                        let on_target = self.rng.gen::<f32>() < 0.4;
                        let xg = 0.05 + self.rng.gen::<f32>() * 0.25;
                        MatchEvent::shot(
                            self.minute,
                            self.current_timestamp_ms(),
                            user_config.is_home_team,
                            user_config.player_index, // C6: Use track_id
                            on_target,
                            xg,
                        )
                        .with_ball_position({
                            let (x_m, y_m) = self.ball.position.to_meters();
                            let h_m = self.ball.height as f32 / 10.0;
                            (x_m / field::LENGTH_M, y_m / field::WIDTH_M, h_m)
                        })
 					}
                };
                // P2: auto timestamp via emit_event
                self.emit_event(event);
            }
        } else {
            // 20% chance of defensive action when opponent has ball
            if self.rng.gen::<f32>() < 0.2 {
                let event = self.event_generator.generate_tackle(
                    self.minute,
                    user_config.is_home_team,
                    user_config.player_name.clone(),
                );
                // P2: auto timestamp via emit_event
                self.emit_event(event);
            }
        }
    }

    /// Filter events for display based on highlight level
    pub(crate) fn filter_events_for_display(&mut self) {
        use crate::engine::events::filter_events_by_highlight_level;

        let user_config = match &self.user_player {
            Some(config) => config,
            None => return,
        };

        // 공통 HighlightLevel 정책에 따라 배치 결과 이벤트를 필터링한다.
        let filtered = filter_events_by_highlight_level(
            &self.result.events,
            user_config.highlight_level,
            Some(user_config.player_index as u8),
        );

        self.result.events = filtered;
    }

    /// Get player position by index (uses dynamic position if available)
    ///
    /// FIX_2601 Phase 3.6: Returns Coord10 directly
    pub fn get_player_position_by_index(&self, idx: usize) -> super::super::types::Coord10 {
        use super::super::types::Coord10;

        // Test override takes priority (legacy: converts from meters)
        if let Some(ref overrides) = self.test_player_positions {
            if let Some(&pos) = overrides.get(idx) {
                return Coord10::from_meters(pos.0, pos.1);
            }
        }

        // Use dynamic position if available (already Coord10)
        if let Some(&pos) = self.player_positions.get(idx) {
            return pos;
        }

        // Fallback to base position (converts normalized → Coord10)
        let base = self.get_base_position_for_index(idx);
        Coord10::from_normalized_legacy(base)
    }

    /// Get player position in meters (for code that still needs (f32, f32))
    ///
    /// FIX_2601: Temporary helper during migration, prefer get_player_position_by_index()
    #[inline]
    pub fn get_player_position_meters(&self, idx: usize) -> (f32, f32) {
        self.get_player_position_by_index(idx).to_meters()
    }

    /// Find player index by name (Open Football setting decision integration)
    pub(crate) fn find_player_idx_by_name(&self, is_home: bool, name: &str) -> usize {
        let base_idx = if is_home { 0 } else { 11 };

        // SSOT: assignment-aware pitch roster (supports substitutions).
        let range = if is_home { 0..11 } else { 11..22 };
        for idx in range {
            if self.get_match_player(idx).name == name {
                return idx;
            }
        }

        // Fallback: first attacker
        base_idx + 9
    }

    /// Try to resolve a player track_id (0..21) from a roster name.
    /// Returns None when not found (no guessing / no fallback).
    pub(crate) fn try_find_player_track_id_by_name(
        &self,
        is_home: bool,
        name: &str,
    ) -> Option<usize> {
        // SSOT: assignment-aware pitch roster (supports substitutions).
        let range = if is_home { 0..11 } else { 11..22 };
        for idx in range {
            if self.get_match_player(idx).name == name {
                return Some(idx);
            }
        }

        None
    }

    /// Count goal events by team (home, away)
    ///
    /// Used by finalize() for P0 consistency validation.
    /// Returns (goals_home, goals_away) counting both Goal and OwnGoal events.
    /// OwnGoal by home team counts for away team and vice versa.
    pub(crate) fn count_goal_events(&self) -> (u8, u8) {
        let mut home_goals: u8 = 0;
        let mut away_goals: u8 = 0;

        for event in &self.result.events {
            match event.event_type {
                EventType::Goal => {
                    if event.is_home_team {
                        home_goals = home_goals.saturating_add(1);
                    } else {
                        away_goals = away_goals.saturating_add(1);
                    }
                }
                EventType::OwnGoal => {
                    // Own goal counts for the opposing team
                    if event.is_home_team {
                        away_goals = away_goals.saturating_add(1);
                    } else {
                        home_goals = home_goals.saturating_add(1);
                    }
                }
                _ => {}
            }
        }

        (home_goals, away_goals)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EventType, MatchEvent};

    /// Test helper: create a minimal Goal event
    fn make_goal_event(minute: u8, is_home: bool) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: Some(minute as u64 * 60_000),
            event_type: EventType::Goal,
            is_home_team: is_home,
            player_track_id: Some(if is_home { 9 } else { 20 }), // striker
            target_track_id: None,
            details: None,
        }
    }

    /// Test helper: create a minimal OwnGoal event
    fn make_own_goal_event(minute: u8, is_home: bool) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: Some(minute as u64 * 60_000),
            event_type: EventType::OwnGoal,
            is_home_team: is_home,
            player_track_id: Some(if is_home { 4 } else { 15 }), // defender
            target_track_id: None,
            details: None,
        }
    }

    #[test]
    fn test_count_goal_events_empty() {
        // No events = 0-0
        let events: Vec<MatchEvent> = vec![];
        let (home, away) = count_goals_from_events(&events);
        assert_eq!((home, away), (0, 0));
    }

    #[test]
    fn test_count_goal_events_home_only() {
        let events = vec![
            make_goal_event(15, true),
            make_goal_event(45, true),
            make_goal_event(78, true),
        ];
        let (home, away) = count_goals_from_events(&events);
        assert_eq!((home, away), (3, 0));
    }

    #[test]
    fn test_count_goal_events_away_only() {
        let events = vec![
            make_goal_event(22, false),
            make_goal_event(67, false),
        ];
        let (home, away) = count_goals_from_events(&events);
        assert_eq!((home, away), (0, 2));
    }

    #[test]
    fn test_count_goal_events_mixed() {
        let events = vec![
            make_goal_event(10, true),   // home 1
            make_goal_event(25, false),  // away 1
            make_goal_event(40, true),   // home 2
            make_goal_event(55, false),  // away 2
            make_goal_event(88, true),   // home 3
        ];
        let (home, away) = count_goals_from_events(&events);
        assert_eq!((home, away), (3, 2));
    }

    #[test]
    fn test_count_goal_events_with_own_goals() {
        // Own goal by home team = counts for away
        // Own goal by away team = counts for home
        let events = vec![
            make_goal_event(10, true),       // home 1
            make_own_goal_event(30, true),   // home OG → away 1
            make_goal_event(45, false),      // away 2
            make_own_goal_event(60, false),  // away OG → home 2
            make_goal_event(75, true),       // home 3
        ];
        let (home, away) = count_goals_from_events(&events);
        // home: 2 goals + 1 from away OG = 3
        // away: 1 goal + 1 from home OG = 2
        assert_eq!((home, away), (3, 2));
    }

    #[test]
    fn test_count_goal_events_saturates_at_255() {
        // Edge case: more than 255 goals (saturating_add should prevent overflow)
        let mut events: Vec<MatchEvent> = Vec::with_capacity(300);
        for i in 0..260 {
            events.push(make_goal_event((i % 90) as u8, true));
        }
        let (home, _) = count_goals_from_events(&events);
        assert_eq!(home, 255); // saturated at u8::MAX
    }

    /// Standalone helper to count goals (mirrors MatchEngine::count_goal_events logic)
    fn count_goals_from_events(events: &[MatchEvent]) -> (u8, u8) {
        let mut home_goals: u8 = 0;
        let mut away_goals: u8 = 0;

        for event in events {
            match event.event_type {
                EventType::Goal => {
                    if event.is_home_team {
                        home_goals = home_goals.saturating_add(1);
                    } else {
                        away_goals = away_goals.saturating_add(1);
                    }
                }
                EventType::OwnGoal => {
                    if event.is_home_team {
                        away_goals = away_goals.saturating_add(1);
                    } else {
                        home_goals = home_goals.saturating_add(1);
                    }
                }
                _ => {}
            }
        }

        (home_goals, away_goals)
    }
}
