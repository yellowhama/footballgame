use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregated statistics for a simulated match.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchStatistics {
    pub match_id: String,
    pub home_team: String,
    pub away_team: String,
    pub goals_home: u8,
    pub goals_away: u8,
    pub possession_home: f32,
    pub possession_away: f32,
    pub shots_home: u16,
    pub shots_away: u16,
    pub shots_on_target_home: u16,
    pub shots_on_target_away: u16,
    pub xg_home: f32,
    pub xg_away: f32,
    pub passes_home: u16,
    pub passes_away: u16,
    pub pass_accuracy_home: f32,
    pub pass_accuracy_away: f32,
    pub corners_home: u8,
    pub corners_away: u8,
    pub fouls_home: u8,
    pub fouls_away: u8,
    /// Shot-oriented events (includes goals, shots on/off target).
    pub shot_events: Vec<ShotEvent>,
    /// All timeline events relevant to UI (mirrors `shot_events` by default).
    pub events: Vec<ShotEvent>,
    /// Player ratings keyed by roster identifier.
    pub player_ratings: HashMap<String, f32>,
    /// Ordered roster identifiers for UI panels.
    pub roster_home: Vec<String>,
    pub roster_away: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,
}

impl MatchStatistics {
    /// Create an empty statistics payload with the given identifier.
    pub fn empty(match_id: impl Into<String>) -> Self {
        Self { match_id: match_id.into(), ..Default::default() }
    }
}

/// Simplified event payload exposed to Godot layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotEvent {
    pub event_type: String,
    pub minute: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player: Option<String>,
    pub team: String,
    pub coordinates: EventCoordinates,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xg: Option<f32>,
}

/// Normalized coordinates in FIFA 105x68 metre space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct EventCoordinates {
    pub x: f32,
    pub y: f32,
}
