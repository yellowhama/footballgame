//! Replay Converter - Convert MatchResult to ReplayDoc
//!
//! Converts match simulation output (MatchResult with MatchEvents) to
//! replay visualization format (ReplayDoc with detailed Events).
//!
//! Key conversions:
//! - 18 MatchEvent types → 8 ReplayDoc Event types
//! - bool team → Team enum (Home/Away)
//! - player name → player_id (with roster mapping)
//! - minute → time in seconds + minute
//! - NO position data → estimated positions via PositionTracker

use crate::models::events::{EventType, MatchEvent};
use crate::models::match_result::MatchResult;
use crate::models::replay::events::{
    BaseEvent, CardType, DribbleEvent, DribbleOutcome, Event, FoulEvent, PassEvent, PassOutcome,
    SaveEvent, SetPieceEvent, SetPieceKind, ShotEvent, ShotOutcome, SubstitutionEvent, TackleEvent,
};
use crate::models::replay::types::{BallState, CurveType, MeterPos, Team};
use crate::models::team::Formation;
use crate::replay::position_tracker::PositionTracker;
use std::collections::HashMap;

/// Replay converter with position tracking
pub struct ReplayConverter {
    home_formation: Formation,
    away_formation: Formation,
    position_tracker: PositionTracker,
    /// Player index mapping: (team_id, player_name) → player_idx
    player_indices: HashMap<(u8, String), u8>,
    /// Home team roster (player names in order)
    home_roster: Vec<String>,
    /// Away team roster (player names in order)
    away_roster: Vec<String>,
}

impl ReplayConverter {
    /// Create new converter with team formations (backward compatibility)
    pub fn new(home_formation: Formation, away_formation: Formation) -> Self {
        let position_tracker = PositionTracker::new(home_formation.clone(), away_formation.clone());

        Self {
            home_formation,
            away_formation,
            position_tracker,
            player_indices: HashMap::new(),
            home_roster: Vec::new(),
            away_roster: Vec::new(),
        }
    }

    /// Create new converter with team formations and player rosters
    pub fn with_rosters(
        home_formation: Formation,
        away_formation: Formation,
        home_roster: Vec<String>,
        away_roster: Vec<String>,
    ) -> Self {
        // Build player index mapping
        let mut player_indices = HashMap::new();
        for (idx, name) in home_roster.iter().enumerate() {
            player_indices.insert((0, name.clone()), idx as u8);
        }
        for (idx, name) in away_roster.iter().enumerate() {
            player_indices.insert((1, name.clone()), idx as u8);
        }

        // Create position tracker with rosters
        let position_tracker = PositionTracker::with_rosters(
            home_formation.clone(),
            away_formation.clone(),
            home_roster.clone(),
            away_roster.clone(),
        );

        Self {
            home_formation,
            away_formation,
            position_tracker,
            player_indices,
            home_roster,
            away_roster,
        }
    }

    /// Build player index mapping from match result teams (deprecated - use with_rosters instead)
    ///
    /// This method is kept for backward compatibility but does nothing if rosters are already set
    pub fn build_player_indices(&mut self, _match_result: &MatchResult) {
        // If rosters are already set via with_rosters(), player_indices is already populated
        // This method is now a no-op for backward compatibility
    }

    /// Convert entire MatchResult to ReplayDoc events
    pub fn convert_events(&mut self, match_result: &MatchResult) -> Result<Vec<Event>, String> {
        self.build_player_indices(match_result);

        let mut replay_events = Vec::new();

        for event in &match_result.events {
            match self.convert_event(event) {
                Ok(replay_event) => {
                    replay_events.push(replay_event);
                    // Update position tracker for next event
                    self.position_tracker.update(event);
                }
                Err(e) => {
                    // Log error but continue conversion
                    eprintln!("Warning: Failed to convert event at minute {}: {}", event.minute, e);
                }
            }
        }

        Ok(replay_events)
    }

    /// Convert single MatchEvent to ReplayDoc Event
    fn convert_event(&self, event: &MatchEvent) -> Result<Event, String> {
        // Convert team
        let team = if event.is_home_team { Team::Home } else { Team::Away };
        let team_id = if event.is_home_team { 0 } else { 1 };

        // C7: Get player ID from track_id (not name)
        let player_idx = event.player_track_id.unwrap_or(0) as usize;
        let local_idx = if event.is_home_team {
            player_idx
        } else {
            player_idx.saturating_sub(11) // Away team: 11-21 -> 0-10
        };
        let player_id = format!("{}{}", if event.is_home_team { "H" } else { "A" }, local_idx);

        // Calculate time (assume each minute = 60 seconds, add random seconds within minute)
        let t = event.minute as f64 * 60.0 + 30.0; // Mid-minute approximation
        let minute = event.minute as u32;

        // Get position from tracker
        let pos = self.position_tracker.estimate_event_position(event, local_idx as u8);

        // Create base event
        let base = BaseEvent::new(t, minute, team, player_id.clone(), pos.clone());

        // Convert based on event type
        match event.event_type {
            EventType::Goal | EventType::OwnGoal => {
                let xg = event.details.as_ref().and_then(|d| d.xg_value).unwrap_or(0.9) as f64;

                let target = self.calculate_goal_target(&pos, team_id);
                let ball = self.create_ball_state(&pos, &target, 25.0);

                // For own goals, we still create a Shot event but mark it as a goal
                // The own goal player info is in event.details.own_goal_by
                Ok(Event::Shot(ShotEvent {
                    base,
                    target,
                    xg,
                    on_target: true,
                    ball,
                    outcome: ShotOutcome::Goal,
                    // Phase 3 fields - populated from event details if available
                    shot_type: None,
                    defender_pressure: None,
                    angle_to_goal: None,
                    distance_to_goal: None,
                    composure: None,
                    finishing_skill: None,
                    curve_factor: None,
                }))
            }

            EventType::Shot
            | EventType::ShotOnTarget
            | EventType::ShotOffTarget
            | EventType::ShotBlocked => {
                let xg = event.details.as_ref().and_then(|d| d.xg_value).unwrap_or(0.1) as f64;

                let on_target = matches!(event.event_type, EventType::ShotOnTarget);
                let target = self.calculate_shot_target(&pos, team_id, on_target);
                let ball = self.create_ball_state(&pos, &target, 22.0);

                let outcome = match event.event_type {
                    EventType::ShotOnTarget => ShotOutcome::Saved, // Assume saved if on target but not goal
                    EventType::ShotBlocked => ShotOutcome::Off,    // Blocked counts as off
                    _ => ShotOutcome::Off,
                };

                Ok(Event::Shot(ShotEvent {
                    base,
                    target,
                    xg,
                    on_target,
                    ball,
                    outcome,
                    // Phase 3 fields
                    shot_type: None,
                    defender_pressure: None,
                    angle_to_goal: None,
                    distance_to_goal: None,
                    composure: None,
                    finishing_skill: None,
                    curve_factor: None,
                }))
            }

            EventType::Save => {
                // Save event - goalkeeper saving a shot
                let target = pos.clone(); // Ball target is roughly where GK is
                let from = MeterPos { x: if team_id == 0 { 85.0 } else { 20.0 }, y: pos.y };
                let ball = self.create_ball_state(&from, &target, 24.0);

                Ok(Event::Save(SaveEvent {
                    base,
                    ball,
                    parry_to: None,
                    // Phase 3 fields
                    shot_from: Some(from),
                    shot_power: None,
                    save_difficulty: None,
                    reflexes_skill: None,
                    handling_skill: None,
                    diving_skill: None,
                }))
            }

            EventType::Pass => {
                let end_pos = MeterPos { x: pos.x + 10.0, y: pos.y + 5.0 };
                let receiver_id = format!(
                    "{}{}",
                    if event.is_home_team { "H" } else { "A" },
                    (player_idx + 1) % 11
                );
                let ball = self.create_ball_state(&pos, &end_pos, 15.0);

                Ok(Event::Pass(PassEvent {
                    base,
                    end_pos: PositionTracker::enforce_boundaries(end_pos),
                    receiver_id,
                    ground: true,
                    ball,
                    outcome: PassOutcome::Complete,
                    // Phase 3 fields
                    distance_m: None,
                    passing_skill: None,
                    vision: None,
                    technique: None,
                    force: None,
                    // 0108 Phase 4 fields
                    danger_level: None,
                    is_switch_of_play: None,
                    is_line_breaking: None,
                    is_through_ball: None,
                    intended_target_pos: None,
                }))
            }

            EventType::Dribble => {
                let end_pos = MeterPos { x: pos.x + 8.0, y: pos.y };

                Ok(Event::Dribble(DribbleEvent {
                    base,
                    path: vec![pos.clone(), end_pos.clone()],
                    end_pos: PositionTracker::enforce_boundaries(end_pos),
                    beats: vec![],
                    outcome: DribbleOutcome::Kept,
                    // Phase 3 fields
                    success: Some(true),
                    opponents_evaded: None,
                    space_gained: None,
                    pressure_level: None,
                    dribbling_skill: None,
                    agility: None,
                }))
            }

            EventType::Tackle => {
                let opponent_id =
                    format!("{}{}", if event.is_home_team { "A" } else { "H" }, player_idx);

                Ok(Event::Tackle(TackleEvent { base, opponent_id, success: true }))
            }

            EventType::Foul => {
                let opponent_id =
                    format!("{}{}", if event.is_home_team { "A" } else { "H" }, player_idx);

                Ok(Event::Foul(FoulEvent { base, opponent_id, card: CardType::None }))
            }

            // FIX_2601/0123 Phase 6: Handball events map to Foul for replay
            EventType::Handball => {
                let opponent_id =
                    format!("{}{}", if event.is_home_team { "A" } else { "H" }, player_idx);

                Ok(Event::Foul(FoulEvent { base, opponent_id, card: CardType::None }))
            }

            EventType::YellowCard | EventType::RedCard => {
                let opponent_id =
                    format!("{}{}", if event.is_home_team { "A" } else { "H" }, player_idx);
                let card = if matches!(event.event_type, EventType::YellowCard) {
                    CardType::Yellow
                } else {
                    CardType::Red
                };

                Ok(Event::Foul(FoulEvent { base, opponent_id, card }))
            }

            EventType::Substitution => {
                // C7: Use target_track_id (player being replaced)
                let out_id = event
                    .target_track_id
                    .map(|track_id| {
                        let local_idx = if event.is_home_team {
                            track_id as usize
                        } else {
                            (track_id as usize).saturating_sub(11)
                        };
                        format!("{}{}", if event.is_home_team { "H" } else { "A" }, local_idx)
                    })
                    .unwrap_or_else(|| {
                        format!("{}{}", if event.is_home_team { "H" } else { "A" }, 10)
                    });

                Ok(Event::Substitution(SubstitutionEvent { base, out_id, in_id: player_id }))
            }

            EventType::Corner => {
                Ok(Event::SetPiece(SetPieceEvent { base, kind: SetPieceKind::Corner, ball: None }))
            }

            EventType::Freekick => Ok(Event::SetPiece(SetPieceEvent {
                base,
                kind: SetPieceKind::FreeKick,
                ball: None,
            })),

            EventType::Penalty => {
                Ok(Event::SetPiece(SetPieceEvent { base, kind: SetPieceKind::Penalty, ball: None }))
            }

            EventType::Offside => {
                // Map offside to SetPiece(FreeKick) for now
                Ok(Event::SetPiece(SetPieceEvent {
                    base,
                    kind: SetPieceKind::FreeKick,
                    ball: None,
                }))
            }

            EventType::KeyChance | EventType::Injury => {
                // Map key chance and injury to Pass events as fallback
                let end_pos = MeterPos { x: pos.x + 5.0, y: pos.y };
                let receiver_id = format!(
                    "{}{}",
                    if event.is_home_team { "H" } else { "A" },
                    (player_idx + 1) % 11
                );
                let ball = self.create_ball_state(&pos, &end_pos, 12.0);

                Ok(Event::Pass(PassEvent {
                    base,
                    end_pos: PositionTracker::enforce_boundaries(end_pos),
                    receiver_id,
                    ground: true,
                    ball,
                    outcome: PassOutcome::Complete,
                    // Phase 3 fields
                    distance_m: None,
                    passing_skill: None,
                    vision: None,
                    technique: None,
                    force: None,
                    // 0108 Phase 4 fields
                    danger_level: None,
                    is_switch_of_play: None,
                    is_line_breaking: None,
                    is_through_ball: None,
                    intended_target_pos: None,
                }))
            }

            // ENGINE_CONTRACT events (2025-12-11)
            EventType::KickOff => {
                // KickOff maps to SetPiece for replay visualization
                Ok(Event::SetPiece(SetPieceEvent { base, kind: SetPieceKind::KickOff, ball: None }))
            }

            EventType::GoalKick => Ok(Event::SetPiece(SetPieceEvent {
                base,
                kind: SetPieceKind::GoalKick,
                ball: None,
            })),

            EventType::ThrowIn => {
                Ok(Event::SetPiece(SetPieceEvent { base, kind: SetPieceKind::ThrowIn, ball: None }))
            }

            EventType::PostHit | EventType::BarHit => {
                // Post/Bar hits are shots that didn't result in goal
                let xg = event.details.as_ref().and_then(|d| d.xg_value).unwrap_or(0.3) as f64;
                let target = self.calculate_goal_target(&pos, team_id);
                let ball = self.create_ball_state(&pos, &target, 28.0);

                Ok(Event::Shot(ShotEvent {
                    base,
                    target,
                    xg,
                    on_target: true, // Post/bar is on target but not in
                    ball,
                    outcome: ShotOutcome::Off, // Rebounded off frame
                    shot_type: None,
                    defender_pressure: None,
                    angle_to_goal: None,
                    distance_to_goal: None,
                    composure: None,
                    finishing_skill: None,
                    curve_factor: None,
                }))
            }

            EventType::HalfTime | EventType::FullTime | EventType::VarReview => {
                // Match phase events - map to a minimal pass event (no actual ball movement)
                let end_pos = pos.clone();
                let ball = self.create_ball_state(&pos, &end_pos, 0.0);

                Ok(Event::Pass(PassEvent {
                    base,
                    end_pos,
                    receiver_id: player_id,
                    ground: true,
                    ball,
                    outcome: PassOutcome::Complete,
                    distance_m: None,
                    passing_skill: None,
                    vision: None,
                    technique: None,
                    force: None,
                    // 0108 Phase 4 fields
                    danger_level: None,
                    is_switch_of_play: None,
                    is_line_breaking: None,
                    is_through_ball: None,
                    intended_target_pos: None,
                }))
            }
        }
    }

    /// Get player index from team roster, default to 0 if not found
    fn get_player_index(&self, team_id: u8, player_name: &str) -> u8 {
        self.player_indices.get(&(team_id, player_name.to_string())).copied().unwrap_or(0)
    }

    /// Calculate goal target position
    fn calculate_goal_target(&self, from: &MeterPos, team_id: u8) -> MeterPos {
        let goal_x = if team_id == 0 { 105.0 } else { 0.0 };
        let y_variation = (from.y - 34.0) * 0.3; // Slight angle variation

        MeterPos { x: goal_x, y: (34.0 + y_variation).clamp(30.0, 38.0) }
    }

    /// Calculate shot target position
    fn calculate_shot_target(&self, from: &MeterPos, team_id: u8, on_target: bool) -> MeterPos {
        let goal_x = if team_id == 0 { 105.0 } else { 0.0 };

        if on_target {
            MeterPos { x: goal_x, y: from.y.clamp(28.0, 40.0) }
        } else {
            MeterPos {
                x: goal_x,
                y: if from.y > 34.0 { 70.0 } else { -2.0 }, // Off target
            }
        }
    }

    /// Create ball state between two positions
    fn create_ball_state(&self, from: &MeterPos, to: &MeterPos, speed_mps: f64) -> BallState {
        BallState { from: from.clone(), to: to.clone(), speed_mps, curve: CurveType::None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // NOTE: SSOT Bypass for Testing
    // ========================================
    // These tests create MatchEvent directly, bypassing the centralized
    // emit_event() function in MatchEngine (mod.rs:618).
    //
    // This is INTENTIONAL for unit testing:
    // - ReplayConverter needs to test event conversion in isolation
    // - Creating a full MatchEngine instance is heavyweight
    // - Production code MUST use emit_event() for all event registration
    //
    // Reference: /crates/of_core/src/engine/match_sim/mod.rs:618
    // ========================================

    #[test]
    fn test_converter_creation() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F433);
        assert_eq!(converter.home_formation, Formation::F442);
        assert_eq!(converter.away_formation, Formation::F433);
    }

    #[test]
    fn test_goal_event_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 23,
            timestamp_ms: None,
            event_type: EventType::Goal,
            is_home_team: true,
            player_track_id: Some(9),
            target_track_id: None,
            details: Some(crate::models::events::EventDetails {
                xg_value: Some(0.85),
                ..Default::default()
            }),
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::Shot(_)));

        if let Event::Shot(shot) = event {
            assert_eq!(shot.outcome, ShotOutcome::Goal);
            assert!(shot.on_target);
            assert_eq!(shot.base.minute, 23);
            assert_eq!(shot.base.team, Team::Home);
            assert!((shot.xg - 0.85).abs() < 0.01);
        }
    }

    #[test]
    fn test_substitution_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 65,
            timestamp_ms: None,
            event_type: EventType::Substitution,
            is_home_team: false,
            player_track_id: Some(13), // Away team: local 2
            target_track_id: Some(16), // Away team: local 5 (replaced)
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::Substitution(_)));

        if let Event::Substitution(sub) = event {
            assert_eq!(sub.base.minute, 65);
            assert_eq!(sub.base.team, Team::Away);
        }
    }

    #[test]
    fn test_corner_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 45,
            timestamp_ms: None,
            event_type: EventType::Corner,
            is_home_team: true,
            player_track_id: Some(8),
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::SetPiece(_)));

        if let Event::SetPiece(sp) = event {
            assert_eq!(sp.kind, SetPieceKind::Corner);
            assert_eq!(sp.base.team, Team::Home);
        }
    }

    #[test]
    fn test_card_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let yellow_card = MatchEvent {
            minute: 30,
            timestamp_ms: None,
            event_type: EventType::YellowCard,
            is_home_team: true,
            player_track_id: Some(2),
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&yellow_card);
        assert!(result.is_ok());

        if let Event::Foul(foul) = result.unwrap() {
            assert_eq!(foul.card, CardType::Yellow);
        }
    }

    #[test]
    fn test_kickoff_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 0,
            timestamp_ms: None,
            event_type: EventType::KickOff,
            is_home_team: true,
            player_track_id: Some(6),
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::SetPiece(_)));

        if let Event::SetPiece(sp) = event {
            assert_eq!(sp.kind, SetPieceKind::KickOff);
            assert_eq!(sp.base.team, Team::Home);
            assert_eq!(sp.base.minute, 0);
            assert!(sp.ball.is_none());
        }
    }

    #[test]
    fn test_goal_kick_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 12,
            timestamp_ms: None,
            event_type: EventType::GoalKick,
            is_home_team: false,
            player_track_id: Some(11), // Away GK
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::SetPiece(_)));

        if let Event::SetPiece(sp) = event {
            assert_eq!(sp.kind, SetPieceKind::GoalKick);
            assert_eq!(sp.base.team, Team::Away);
            assert_eq!(sp.base.minute, 12);
            assert!(sp.ball.is_none());
        }
    }

    #[test]
    fn test_throw_in_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 25,
            timestamp_ms: None,
            event_type: EventType::ThrowIn,
            is_home_team: true,
            player_track_id: Some(4),
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::SetPiece(_)));

        if let Event::SetPiece(sp) = event {
            assert_eq!(sp.kind, SetPieceKind::ThrowIn);
            assert_eq!(sp.base.team, Team::Home);
            assert_eq!(sp.base.minute, 25);
            assert!(sp.ball.is_none());
        }
    }

    #[test]
    fn test_post_hit_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 38,
            timestamp_ms: None,
            event_type: EventType::PostHit,
            is_home_team: true,
            player_track_id: Some(9),
            target_track_id: None,
            details: Some(crate::models::events::EventDetails {
                xg_value: Some(0.65),
                ..Default::default()
            }),
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::Shot(_)));

        if let Event::Shot(shot) = event {
            assert_eq!(shot.outcome, ShotOutcome::Off);
            assert!(shot.on_target);
            assert_eq!(shot.base.minute, 38);
            assert_eq!(shot.base.team, Team::Home);
            assert!((shot.xg - 0.65).abs() < 0.01);
        }
    }

    #[test]
    fn test_bar_hit_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 67,
            timestamp_ms: None,
            event_type: EventType::BarHit,
            is_home_team: false,
            player_track_id: Some(20),
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::Shot(_)));

        if let Event::Shot(shot) = event {
            assert_eq!(shot.outcome, ShotOutcome::Off);
            assert!(shot.on_target);
            assert_eq!(shot.base.minute, 67);
            assert_eq!(shot.base.team, Team::Away);
            // Default xg when details is None
            assert!((shot.xg - 0.3).abs() < 0.01);
        }
    }

    #[test]
    fn test_halftime_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 45,
            timestamp_ms: None,
            event_type: EventType::HalfTime,
            is_home_team: true,
            player_track_id: None,
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::Pass(_)));

        if let Event::Pass(pass) = event {
            assert_eq!(pass.outcome, PassOutcome::Complete);
            assert_eq!(pass.base.minute, 45);
            assert_eq!(pass.base.team, Team::Home);
            assert!(pass.ground);
        }
    }

    #[test]
    fn test_fulltime_conversion() {
        let converter = ReplayConverter::new(Formation::F442, Formation::F442);

        let match_event = MatchEvent {
            minute: 90,
            timestamp_ms: None,
            event_type: EventType::FullTime,
            is_home_team: false,
            player_track_id: None,
            target_track_id: None,
            details: None,
        };

        let result = converter.convert_event(&match_event);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert!(matches!(event, Event::Pass(_)));

        if let Event::Pass(pass) = event {
            assert_eq!(pass.outcome, PassOutcome::Complete);
            assert_eq!(pass.base.minute, 90);
            assert_eq!(pass.base.team, Team::Away);
            assert!(pass.ground);
        }
    }
}
