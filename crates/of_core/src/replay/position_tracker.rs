//! Position Tracker for Replay System
//!
//! Estimates player positions during match events based on formation and event context.
//! Required because MatchEvent data contains no positional information.

use crate::models::events::{EventType, MatchEvent};
use crate::models::replay::types::MeterPos;
use crate::models::team::Formation;
use std::collections::HashMap;

/// FIFA standard field dimensions (meters)
pub const FIELD_WIDTH: f64 = 105.0; // Length (x-axis)
pub const FIELD_HEIGHT: f64 = 68.0; // Width (y-axis)

/// Penalty box dimensions
pub const PENALTY_BOX_DEPTH: f64 = 16.5; // Distance from goal line
pub const PENALTY_BOX_WIDTH: f64 = 40.3; // Width of penalty box

/// Penalty spot distance from goal line
pub const PENALTY_SPOT_DISTANCE: f64 = 11.0;

/// Center circle radius
pub const CENTER_CIRCLE_RADIUS: f64 = 9.15;

/// Position tracking state for a single match
#[derive(Debug, Clone)]
pub struct PositionTracker {
    /// Current estimated positions: (team_id, player_idx) → position
    positions: HashMap<(u8, u8), MeterPos>,

    /// Base positions from formation: (team_id, player_idx) → formation position
    base_positions: HashMap<(u8, u8), MeterPos>,

    /// Formation configurations
    home_formation: Formation,
    away_formation: Formation,

    /// Player name → index mapping for home team
    home_roster: HashMap<String, u8>,
    /// Player name → index mapping for away team
    away_roster: HashMap<String, u8>,
}

impl PositionTracker {
    /// Create new position tracker with formations
    pub fn new(home_formation: Formation, away_formation: Formation) -> Self {
        let mut tracker = Self {
            positions: HashMap::new(),
            base_positions: HashMap::new(),
            home_formation,
            away_formation,
            home_roster: HashMap::new(),
            away_roster: HashMap::new(),
        };

        // Initialize base positions from formations
        tracker.initialize_formations();

        tracker
    }

    /// Create new position tracker with formations and player rosters
    pub fn with_rosters(
        home_formation: Formation,
        away_formation: Formation,
        home_names: Vec<String>,
        away_names: Vec<String>,
    ) -> Self {
        // Build roster maps
        let home_roster: HashMap<String, u8> =
            home_names.iter().enumerate().map(|(i, name)| (name.clone(), i as u8)).collect();

        let away_roster: HashMap<String, u8> =
            away_names.iter().enumerate().map(|(i, name)| (name.clone(), i as u8)).collect();

        let mut tracker = Self {
            positions: HashMap::new(),
            base_positions: HashMap::new(),
            home_formation,
            away_formation,
            home_roster,
            away_roster,
        };

        // Initialize base positions from formations
        tracker.initialize_formations();

        tracker
    }

    /// Initialize base positions for both teams based on their formations
    fn initialize_formations(&mut self) {
        // Home team (team_id = 0) attacks right (high x values)
        self.initialize_team_formation(0, &self.home_formation.clone(), false);

        // Away team (team_id = 1) attacks left (low x values)
        self.initialize_team_formation(1, &self.away_formation.clone(), true);
    }

    /// Initialize formation positions for one team
    fn initialize_team_formation(&mut self, team_id: u8, formation: &Formation, flip: bool) {
        let (defenders, midfielders, forwards) = formation.get_positions();

        // Calculate base x positions (depth from own goal)
        let base_x = if flip { FIELD_WIDTH * 0.75 } else { FIELD_WIDTH * 0.25 };
        let def_x = if flip { base_x + 20.0 } else { base_x - 20.0 };
        let mid_x = base_x; // midfielders at base_x for both teams
        let fwd_x = if flip { base_x - 20.0 } else { base_x + 20.0 };

        let mut player_idx = 0u8;

        // Goalkeeper (always at goal line)
        let gk_x = if flip { FIELD_WIDTH - 5.0 } else { 5.0 };
        self.set_base_position(team_id, player_idx, MeterPos { x: gk_x, y: FIELD_HEIGHT / 2.0 });
        player_idx += 1;

        // Defenders
        for i in 0..defenders {
            let y = Self::calculate_lateral_position(i, defenders);
            self.set_base_position(team_id, player_idx, MeterPos { x: def_x, y });
            player_idx += 1;
        }

        // Midfielders
        for i in 0..midfielders {
            let y = Self::calculate_lateral_position(i, midfielders);
            self.set_base_position(team_id, player_idx, MeterPos { x: mid_x, y });
            player_idx += 1;
        }

        // Forwards
        for i in 0..forwards {
            let y = Self::calculate_lateral_position(i, forwards);
            self.set_base_position(team_id, player_idx, MeterPos { x: fwd_x, y });
            player_idx += 1;
        }
    }

    /// Calculate lateral (y-axis) position for player in a line
    fn calculate_lateral_position(index: u8, total: u8) -> f64 {
        if total == 1 {
            return FIELD_HEIGHT / 2.0; // Center
        }

        // Distribute evenly across width with margins
        let margin = FIELD_HEIGHT * 0.15;
        let usable_width = FIELD_HEIGHT - (2.0 * margin);
        let spacing = usable_width / (total - 1) as f64;

        margin + (index as f64 * spacing)
    }

    /// Set base position for a player
    fn set_base_position(&mut self, team_id: u8, player_idx: u8, pos: MeterPos) {
        self.base_positions.insert((team_id, player_idx), pos.clone());
        self.positions.insert((team_id, player_idx), pos);
    }

    /// Update positions based on event (basic implementation)
    pub fn update(&mut self, event: &MatchEvent) {
        let team_id = if event.is_home_team { 0 } else { 1 };

        // C7: Move event subject player to event position (using track_id)
        if let Some(track_id) = event.player_track_id {
            // Convert track_id (0-21) to team-local index (0-10)
            let player_idx_u8 =
                if event.is_home_team { track_id } else { track_id.saturating_sub(11) };
            let event_pos = self.estimate_event_position(event, player_idx_u8);
            self.positions.insert((team_id, player_idx_u8), event_pos);
        }

        // 2. Apply ball attraction (players move towards ball)
        let ball_pos = self.estimate_ball_position_from_event(event);
        self.apply_ball_attraction(ball_pos, 15.0); // 15m radius

        // 3. Apply formation pull (elastic band to base position)
        self.apply_formation_pull(0.3); // 30% pull back
    }

    /// Find player index by name from roster mapping
    fn find_player_by_name(&self, team_id: u8, player_name: &str) -> Option<u8> {
        let roster = if team_id == 0 { &self.home_roster } else { &self.away_roster };

        roster.get(player_name).copied()
    }

    /// Estimate ball position from event context
    fn estimate_ball_position_from_event(&self, event: &MatchEvent) -> MeterPos {
        let team_id = if event.is_home_team { 0 } else { 1 };
        let attacking_right = team_id == 0;

        match event.event_type {
            EventType::Goal => {
                let x = if attacking_right { FIELD_WIDTH - 5.0 } else { 5.0 };
                MeterPos { x, y: FIELD_HEIGHT / 2.0 }
            }
            EventType::Shot | EventType::ShotOnTarget | EventType::ShotOffTarget => {
                let x = if attacking_right {
                    FIELD_WIDTH - PENALTY_BOX_DEPTH - 5.0
                } else {
                    PENALTY_BOX_DEPTH + 5.0
                };
                MeterPos { x, y: FIELD_HEIGHT / 2.0 }
            }
            EventType::Corner => {
                let x = if attacking_right { FIELD_WIDTH - 1.0 } else { 1.0 };
                MeterPos { x, y: FIELD_HEIGHT / 2.0 }
            }
            _ => {
                // Default: center of field
                MeterPos { x: FIELD_WIDTH / 2.0, y: FIELD_HEIGHT / 2.0 }
            }
        }
    }

    /// Apply ball attraction: players within radius move towards ball
    fn apply_ball_attraction(&mut self, ball_pos: MeterPos, radius: f64) {
        let positions_snapshot: Vec<_> = self.positions.keys().cloned().collect();

        for key in positions_snapshot {
            if let Some(pos) = self.positions.get(&key).cloned() {
                let distance = Self::distance(pos.clone(), ball_pos.clone());

                // Only move players within radius
                if distance < radius && distance > 0.1 {
                    let direction = Self::normalize_vector(ball_pos.x - pos.x, ball_pos.y - pos.y);

                    // Move 2m towards ball
                    let new_pos =
                        MeterPos { x: pos.x + direction.0 * 2.0, y: pos.y + direction.1 * 2.0 };

                    self.positions.insert(key, new_pos);
                }
            }
        }
    }

    /// Apply formation pull: players pulled back towards base position
    fn apply_formation_pull(&mut self, strength: f64) {
        let positions_snapshot: Vec<_> = self.positions.keys().cloned().collect();

        for key in positions_snapshot {
            if let Some(pos) = self.positions.get(&key).cloned() {
                if let Some(base_pos) = self.base_positions.get(&key) {
                    let distance_from_base = Self::distance(pos.clone(), base_pos.clone());

                    // Only pull if more than 10m from base
                    if distance_from_base > 10.0 {
                        let direction =
                            Self::normalize_vector(base_pos.x - pos.x, base_pos.y - pos.y);

                        // Pull towards base by strength factor
                        let pull_distance = distance_from_base * strength;
                        let new_pos = MeterPos {
                            x: pos.x + direction.0 * pull_distance,
                            y: pos.y + direction.1 * pull_distance,
                        };

                        self.positions.insert(key, new_pos);
                    }
                }
            }
        }
    }

    /// Calculate distance between two positions
    fn distance(a: MeterPos, b: MeterPos) -> f64 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Normalize a 2D vector
    fn normalize_vector(x: f64, y: f64) -> (f64, f64) {
        let length = (x * x + y * y).sqrt();
        if length > 0.001 {
            (x / length, y / length)
        } else {
            (0.0, 0.0)
        }
    }

    /// Get current position for a player
    pub fn get_position(&self, team_id: u8, player_idx: u8) -> MeterPos {
        self.positions
            .get(&(team_id, player_idx))
            .cloned()
            .unwrap_or_else(|| MeterPos { x: FIELD_WIDTH / 2.0, y: FIELD_HEIGHT / 2.0 })
    }

    /// Estimate position for an event (event-specific logic)
    pub fn estimate_event_position(&self, event: &MatchEvent, player_idx: u8) -> MeterPos {
        let team_id = if event.is_home_team { 0 } else { 1 };
        let attacking_right = team_id == 0; // Home team attacks right

        // Get base position
        let base = self.get_position(team_id, player_idx);

        // Apply event-specific adjustments
        match event.event_type {
            EventType::Goal => {
                // Goals happen in penalty box
                let goal_x = if attacking_right {
                    FIELD_WIDTH - PENALTY_SPOT_DISTANCE
                } else {
                    PENALTY_SPOT_DISTANCE
                };
                MeterPos { x: goal_x, y: base.y }
            }

            EventType::Shot
            | EventType::ShotOnTarget
            | EventType::ShotOffTarget
            | EventType::ShotBlocked => {
                // Shots from attacking third
                let shot_x = if attacking_right {
                    FIELD_WIDTH - PENALTY_BOX_DEPTH - 5.0 // Just outside penalty box
                } else {
                    PENALTY_BOX_DEPTH + 5.0
                };
                MeterPos { x: shot_x, y: base.y }
            }

            EventType::Save => {
                // Saves happen at goal line
                // Save is performed by the goalkeeper (event.is_home_team == goalkeeper team).
                // Home team defends the left goal, away team defends the right goal.
                let save_x = if attacking_right { 5.0 } else { FIELD_WIDTH - 5.0 };
                MeterPos { x: save_x, y: FIELD_HEIGHT / 2.0 }
            }

            EventType::Tackle | EventType::Foul => {
                // Defensive actions in own half
                let def_x = if attacking_right {
                    FIELD_WIDTH * 0.33 // Defending left third
                } else {
                    FIELD_WIDTH * 0.67 // Defending right third
                };
                MeterPos { x: def_x, y: base.y }
            }

            EventType::Corner => {
                // Corners at corner flag
                let corner_x = if attacking_right { FIELD_WIDTH - 1.0 } else { 1.0 };
                let corner_y = if base.y > FIELD_HEIGHT / 2.0 { FIELD_HEIGHT - 1.0 } else { 1.0 };
                MeterPos { x: corner_x, y: corner_y }
            }

            EventType::Freekick | EventType::Penalty => {
                // Set pieces in attacking third
                let set_x = if attacking_right {
                    FIELD_WIDTH - PENALTY_BOX_DEPTH - 10.0
                } else {
                    PENALTY_BOX_DEPTH + 10.0
                };
                MeterPos { x: set_x, y: base.y }
            }

            EventType::Pass => {
                // Passes from current position
                base
            }

            EventType::Offside => {
                // Offside in attacking third
                let offside_x =
                    if attacking_right { FIELD_WIDTH * 0.75 } else { FIELD_WIDTH * 0.25 };
                MeterPos { x: offside_x, y: base.y }
            }

            _ => {
                // Default to base position
                base
            }
        }
    }

    /// Enforce field boundaries (clamp to valid field area)
    pub fn enforce_boundaries(pos: MeterPos) -> MeterPos {
        MeterPos { x: pos.x.clamp(0.0, FIELD_WIDTH), y: pos.y.clamp(0.0, FIELD_HEIGHT) }
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
    // - PositionTracker needs to test position updates in isolation
    // - Creating a full MatchEngine instance is heavyweight
    // - Production code MUST use emit_event() for all event registration
    //
    // Reference: /crates/of_core/src/engine/match_sim/mod.rs:618
    // ========================================

    #[test]
    fn test_tracker_initialization() {
        let tracker = PositionTracker::new(Formation::F442, Formation::F433);

        // Check home GK position (team 0, player 0)
        let home_gk = tracker.get_position(0, 0);
        assert!(home_gk.x < 10.0, "Home GK should be near left goal");
        assert!((home_gk.y - FIELD_HEIGHT / 2.0).abs() < 1.0, "GK should be centered");

        // Check away GK position (team 1, player 0)
        let away_gk = tracker.get_position(1, 0);
        assert!(away_gk.x > FIELD_WIDTH - 10.0, "Away GK should be near right goal");
    }

    #[test]
    fn test_formation_442_positions() {
        let tracker = PositionTracker::new(Formation::F442, Formation::F442);

        // 4-4-2: 1 GK + 4 DEF + 4 MID + 2 FWD = 11 players
        // Check that we have positions for all 11 home team players
        for i in 0..11 {
            let pos = tracker.get_position(0, i);
            assert!(pos.x >= 0.0 && pos.x <= FIELD_WIDTH);
            assert!(pos.y >= 0.0 && pos.y <= FIELD_HEIGHT);
        }
    }

    #[test]
    fn test_lateral_position_calculation() {
        // Single player should be centered
        let pos1 = PositionTracker::calculate_lateral_position(0, 1);
        assert_eq!(pos1, FIELD_HEIGHT / 2.0);

        // Two players should be symmetrically placed
        let left = PositionTracker::calculate_lateral_position(0, 2);
        let right = PositionTracker::calculate_lateral_position(1, 2);
        assert!(left < FIELD_HEIGHT / 2.0);
        assert!(right > FIELD_HEIGHT / 2.0);

        // Four players should be evenly distributed
        let positions: Vec<f64> =
            (0..4).map(|i| PositionTracker::calculate_lateral_position(i, 4)).collect();

        // Check monotonic increase
        for i in 0..3 {
            assert!(positions[i] < positions[i + 1]);
        }
    }

    #[test]
    fn test_goal_event_position() {
        let tracker = PositionTracker::new(Formation::F442, Formation::F442);

        let goal_event = MatchEvent {
            minute: 25,
            timestamp_ms: None,
            event_type: EventType::Goal,
            is_home_team: true,
            player_track_id: Some(10),
            target_track_id: None,
            details: None,
        };

        let pos = tracker.estimate_event_position(&goal_event, 10); // Forward player

        // Home team attacks right, so goal should be near right penalty spot
        assert!(pos.x > FIELD_WIDTH - 20.0, "Goal position should be in attacking penalty area");
    }

    #[test]
    fn test_shot_event_position() {
        let tracker = PositionTracker::new(Formation::F442, Formation::F442);

        let shot_event = MatchEvent {
            minute: 30,
            timestamp_ms: None,
            event_type: EventType::ShotOnTarget,
            is_home_team: false,
            player_track_id: Some(18), // Away: local 7
            target_track_id: None,
            details: None,
        };

        let pos = tracker.estimate_event_position(&shot_event, 7); // Midfielder

        // Away team attacks left, shot should be in left attacking third
        assert!(pos.x < FIELD_WIDTH / 3.0, "Shot should be in attacking third");
    }

    #[test]
    fn test_save_event_position() {
        let tracker = PositionTracker::new(Formation::F442, Formation::F442);

        let save_event = MatchEvent {
            minute: 35,
            timestamp_ms: None,
            event_type: EventType::Save,
            is_home_team: true,       // Home goalkeeper save
            player_track_id: Some(0), // Home GK
            target_track_id: None,
            details: None,
        };

        let pos = tracker.estimate_event_position(&save_event, 0); // GK

        // Home goalkeeper save should be at left goal
        assert!(pos.x < 10.0, "Save should be at goal line");
    }

    #[test]
    fn test_corner_event_position() {
        let tracker = PositionTracker::new(Formation::F442, Formation::F442);

        let corner_event = MatchEvent {
            minute: 40,
            timestamp_ms: None,
            event_type: EventType::Corner,
            is_home_team: true,
            player_track_id: Some(8),
            target_track_id: None,
            details: None,
        };

        let pos = tracker.estimate_event_position(&corner_event, 8); // Winger

        // Corner should be near corner flag
        assert!(pos.x > FIELD_WIDTH - 5.0 || pos.x < 5.0, "Corner x near edge");
        assert!(pos.y > FIELD_HEIGHT - 5.0 || pos.y < 5.0, "Corner y near edge");
    }

    #[test]
    fn test_boundary_enforcement() {
        // Test clamping to field boundaries
        let out_of_bounds = MeterPos { x: -10.0, y: 80.0 };
        let clamped = PositionTracker::enforce_boundaries(out_of_bounds);
        assert_eq!(clamped.x, 0.0);
        assert_eq!(clamped.y, FIELD_HEIGHT);

        // Test valid position unchanged
        let valid = MeterPos { x: 52.5, y: 34.0 };
        let unchanged = PositionTracker::enforce_boundaries(valid.clone());
        assert_eq!(unchanged.x, valid.x);
        assert_eq!(unchanged.y, valid.y);
    }

    #[test]
    fn test_tackle_defensive_position() {
        let tracker = PositionTracker::new(Formation::F442, Formation::F442);

        let tackle_event = MatchEvent {
            minute: 45,
            timestamp_ms: None,
            event_type: EventType::Tackle,
            is_home_team: false,
            player_track_id: Some(13), // Away: local 2
            target_track_id: None,
            details: None,
        };

        let pos = tracker.estimate_event_position(&tackle_event, 2); // Defender

        // Away team tackle should be in their defensive third (right side)
        assert!(pos.x > FIELD_WIDTH * 0.5, "Tackle in defensive half");
    }

    #[test]
    fn test_update_moves_player_to_event() {
        let mut tracker = PositionTracker::new(Formation::F442, Formation::F442);

        // Event for home player 9 (forward)
        let goal_event = MatchEvent {
            minute: 25,
            timestamp_ms: None,
            event_type: EventType::Goal,
            is_home_team: true,
            player_track_id: Some(9),
            target_track_id: None,
            details: None,
        };

        let before = tracker.get_position(0, 9); // Forward
        tracker.update(&goal_event);
        let after = tracker.get_position(0, 9);

        // Position should have changed after update (player moved to goal area)
        let moved = (before.x - after.x).abs() > 0.1 || (before.y - after.y).abs() > 0.1;
        assert!(moved, "Player should have moved to event position: before=({:.1}, {:.1}), after=({:.1}, {:.1})",
            before.x, before.y, after.x, after.y);
    }

    #[test]
    fn test_ball_attraction() {
        let mut tracker = PositionTracker::new(Formation::F442, Formation::F442);

        // Place ball on right side
        let ball_pos = MeterPos { x: 80.0, y: 34.0 };

        let before = tracker.get_position(0, 5); // Midfielder at ~26.25
        tracker.apply_ball_attraction(ball_pos, 60.0); // Large radius to ensure effect
        let after = tracker.get_position(0, 5);

        // Midfielder should move towards ball (x should increase)
        assert!(after.x > before.x, "Player should move towards ball");
    }

    #[test]
    fn test_formation_elastic_band() {
        let mut tracker = PositionTracker::new(Formation::F442, Formation::F442);

        // Move player far from base position
        tracker.positions.insert((0, 5), MeterPos { x: 95.0, y: 34.0 });

        let base = tracker.base_positions.get(&(0, 5)).unwrap().clone();
        let before = tracker.get_position(0, 5);

        tracker.apply_formation_pull(0.3);
        let after = tracker.get_position(0, 5);

        // Player should be pulled towards base position
        assert!(after.x < before.x, "Player should be pulled back towards base");

        // Distance from base should decrease
        let dist_before = PositionTracker::distance(before, base.clone());
        let dist_after = PositionTracker::distance(after, base);
        assert!(dist_after < dist_before, "Distance from base should decrease");
    }

    #[test]
    fn test_distance_calculation() {
        let pos1 = MeterPos { x: 0.0, y: 0.0 };
        let pos2 = MeterPos { x: 3.0, y: 4.0 };

        let distance = PositionTracker::distance(pos1, pos2);
        assert!((distance - 5.0).abs() < 0.001, "Distance should be 5.0");
    }

    #[test]
    fn test_normalize_vector() {
        let (nx, ny) = PositionTracker::normalize_vector(3.0, 4.0);
        let length = (nx * nx + ny * ny).sqrt();

        assert!((length - 1.0).abs() < 0.001, "Normalized vector should have length 1.0");
        assert!((nx - 0.6).abs() < 0.001, "X component should be 0.6");
        assert!((ny - 0.8).abs() < 0.001, "Y component should be 0.8");
    }
}
