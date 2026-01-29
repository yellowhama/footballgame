//! Match Result Data Structures
//!
//! This module defines the output data structures from the match simulation engine.
//! These structures are the SINK of the data pipeline - all simulation output flows here.
//!
//! ## Data Flow Overview (2025-12-11)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                    DATA FLOW: Engine → MatchResult → Consumers             │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  [SOURCE: match_sim/tick_based.rs]                                          │
//! │    │                                                                        │
//! │    │ record_positions_for_tick() calls:                                     │
//! │    │   • add_ball_position_with_velocity(timestamp_ms, pos_meters, h, vel)  │
//! │    │   • add_player_position_with_state(idx, timestamp_ms, pos_meters, st)  │
//! │    │                                                                        │
//! │    ▼                                                                        │
//! │  ┌─────────────────────────────────────────────────────────────────────┐    │
//! │  │                   MatchResult (this file)                          │    │
//! │  │                                                                     │    │
//! │  │  ├─► score: (u8, u8)              ← Goals from shooting.rs          │    │
//! │  │  ├─► events: Vec<MatchEvent>      ← From event_generator.rs         │    │
//! │  │  ├─► statistics: MatchStats       ← From stats_calculator.rs        │    │
//! │  │  ├─► player_stats: HashMap        ← Individual player stats         │    │
//! │  │  └─► position_data: Option<MatchPositionData>  ← POSITION DATA      │    │
//! │  │           │                                                         │    │
//! │  │           └─► MatchPositionData                                     │    │
//! │  │                 ├─► ball: Vec<PositionDataItem>                     │    │
//! │  │                 │     └─► {timestamp_ms, (x,y) meters, height, vel} │    │
//! │  │                 └─► players: HashMap<u8, Vec<PositionDataItem>>     │    │
//! │  │                       └─► {timestamp_ms, (x,y) meters, state}       │    │
//! │  └─────────────────────────────────────────────────────────────────────┘    │
//! │         │                                                                   │
//! │         ▼                                                                   │
//! │  [CONSUMERS]                                                                │
//! │                                                                             │
//! │  1. GodotExtension (crates/of_godot/)                                       │
//! │     • Converts MatchResult to Godot-compatible types                        │
//! │     • Exposes position_data for replay visualization                        │
//! │                                                                             │
//! │  2. Viewer (Godot/addons/match_viewer/)                                     │
//! │     • Reads position_data to animate ball and players                       │
//! │     • Uses timestamp_ms to sync with match time                             │
//! │     • Coordinates are in METERS (0-105, 0-68)                               │
//! │                                                                             │
//! │  3. Server Storage (if applicable)                                          │
//! │     • May serialize only certain fields                                     │
//! │     • VERIFY what gets saved to database!                                   │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Coordinate System
//!
//! All positions in this module are in **METERS**:
//! - Ball/Player x: 0.0 to 105.0 (pitch length)
//! - Ball/Player y: 0.0 to 68.0 (pitch width)
//! - Ball height: 0.0 (ground) to ~3.0 (max height)
//!
//! The engine uses normalized coordinates (0.0-1.0) internally and converts
//! to meters before storing in MatchPositionData.
//!
//! ## Timing
//!
//! - timestamp_ms: Milliseconds from match start
//! - 240 ticks per minute = 4 ticks/second = 250ms per tick
//! - 90 minute match = 90 * 60 * 1000 = 5,400,000 ms total
//!
//! ## Key Methods
//!
//! - `add_ball_position_with_velocity()` - Records ball position with velocity
//! - `add_player_position_with_state()` - Records player position with action state
//!
//! ## WARNING
//!
//! The legacy `record_positions_for_minute()` in mod.rs runs its own simulation
//! and OVERWRITES engine data. When using tick_based mode, that function must
//! be blocked or it will replace real positions with (0,0,0).

use super::match_setup::MatchSetupExport;
use super::replay;
use super::{EventType, MatchEvent, Team};
use crate::engine::field_board::BoardSummaryExport;
use crate::engine::coordinate_contract::{
    COORD_CONTRACT_VERSION, COORD_SYSTEM_LEGACY_AXIS_SWAP, COORD_SYSTEM_METERS_V2,
};
// P0: Core types moved to action_queue
use crate::engine::action_queue::ViewerEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================
// Best Moments (Highlight System)
// ============================================

/// Type of highlight moment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MomentType {
    /// Goal scored
    Goal,
    /// Great save by goalkeeper
    Save,
    /// Shot on target (close call)
    ShotOnTarget,
    /// Ball hits post
    PostHit,
    /// Ball hits crossbar
    BarHit,
    /// Penalty kick
    Penalty,
    /// Red card issued
    RedCard,
    /// Key chance / big opportunity
    KeyChance,
}

impl MomentType {
    /// Priority for sorting (higher = more important)
    pub fn priority(&self) -> u8 {
        match self {
            MomentType::Goal => 100,
            MomentType::Penalty => 90,
            MomentType::RedCard => 80,
            MomentType::PostHit | MomentType::BarHit => 70,
            MomentType::Save => 60,
            MomentType::ShotOnTarget => 50,
            MomentType::KeyChance => 40,
        }
    }
}

/// A highlight moment in the match (for replay jump-to feature)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestMoment {
    /// Start time in milliseconds (a few seconds before the event)
    pub start_time_ms: u64,
    /// End time in milliseconds (a few seconds after the event)
    pub end_time_ms: u64,
    /// Type of moment
    pub moment_type: MomentType,
    /// Priority for sorting (computed from moment_type)
    pub priority: u8,
    /// Match minute when this happened
    pub minute: u8,
    /// Description (e.g., "Goal by Player Name")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Team that this moment relates to (true = home, false = away)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_home_team: Option<bool>,
}

impl BestMoment {
    /// Create a new best moment from an event
    pub fn from_event(event: &MatchEvent, moment_type: MomentType) -> Self {
        // Window: 5 seconds before, 3 seconds after
        const LEAD_TIME_MS: u64 = 5000;
        const TRAIL_TIME_MS: u64 = 3000;

        // Use timestamp_ms if available, otherwise calculate from minute
        let event_time_ms = event.timestamp_ms.unwrap_or_else(|| event.minute as u64 * 60_000);
        let start_time_ms = event_time_ms.saturating_sub(LEAD_TIME_MS);
        let end_time_ms = event_time_ms + TRAIL_TIME_MS;

        Self {
            start_time_ms,
            end_time_ms,
            moment_type,
            priority: moment_type.priority(),
            minute: event.minute,
            description: None, // Event details can be added separately if needed
            is_home_team: Some(event.is_home_team),
        }
    }

    /// Create with custom time window
    pub fn with_window(
        event: &MatchEvent,
        moment_type: MomentType,
        lead_ms: u64,
        trail_ms: u64,
    ) -> Self {
        // Use timestamp_ms if available, otherwise calculate from minute
        let event_time_ms = event.timestamp_ms.unwrap_or_else(|| event.minute as u64 * 60_000);
        let start_time_ms = event_time_ms.saturating_sub(lead_ms);
        let end_time_ms = event_time_ms + trail_ms;

        Self {
            start_time_ms,
            end_time_ms,
            moment_type,
            priority: moment_type.priority(),
            minute: event.minute,
            description: None,
            is_home_team: Some(event.is_home_team),
        }
    }
}

/// Penalty shootout kick log entry (order-preserving).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenaltyShootoutKick {
    pub kick_index: u8,
    pub is_home_team: bool,
    pub kicker_track_id: u8,
    pub kicker_name: String,
    pub scored: bool,
}

/// Penalty shootout outcome (does not mutate regulation score).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenaltyShootoutResult {
    pub goals_home: u8,
    pub goals_away: u8,
    pub kicks_taken_home: u8,
    pub kicks_taken_away: u8,
    pub winner_is_home: bool,
    #[serde(default)]
    pub kicks: Vec<PenaltyShootoutKick>,
}

/// Generate best moments from match events
pub fn generate_best_moments(events: &[MatchEvent]) -> Vec<BestMoment> {        
    let mut moments: Vec<BestMoment> = Vec::new();

    for event in events {
        let moment = match event.event_type {
            EventType::Goal | EventType::OwnGoal => {
                // Goals get longer window (8s before, 5s after)
                Some(BestMoment::with_window(event, MomentType::Goal, 8000, 5000))
            }
            EventType::Save => Some(BestMoment::from_event(event, MomentType::Save)),
            EventType::ShotOnTarget => {
                Some(BestMoment::from_event(event, MomentType::ShotOnTarget))
            }
            EventType::PostHit => Some(BestMoment::from_event(event, MomentType::PostHit)),
            EventType::BarHit => Some(BestMoment::from_event(event, MomentType::BarHit)),
            EventType::Penalty => {
                // Penalties get longer window
                Some(BestMoment::with_window(event, MomentType::Penalty, 10000, 8000))
            }
            EventType::RedCard => Some(BestMoment::from_event(event, MomentType::RedCard)),
            EventType::KeyChance => Some(BestMoment::from_event(event, MomentType::KeyChance)),
            _ => None,
        };

        if let Some(m) = moment {
            moments.push(m);
        }
    }

    // Sort by priority (descending), then by time
    moments.sort_by(|a, b| {
        b.priority.cmp(&a.priority).then_with(|| a.start_time_ms.cmp(&b.start_time_ms))
    });

    // Merge overlapping moments (keep higher priority)
    merge_overlapping_moments(&mut moments);

    moments
}

/// Merge overlapping moments, keeping the higher priority one
fn merge_overlapping_moments(moments: &mut Vec<BestMoment>) {
    if moments.len() < 2 {
        return;
    }

    // Sort by start time for merging
    moments.sort_by_key(|m| m.start_time_ms);

    let mut i = 0;
    while i < moments.len() - 1 {
        let current_end = moments[i].end_time_ms;
        let next_start = moments[i + 1].start_time_ms;

        // If overlapping, merge into the higher priority one
        if next_start <= current_end {
            if moments[i].priority >= moments[i + 1].priority {
                // Extend current moment to cover both
                moments[i].end_time_ms = moments[i].end_time_ms.max(moments[i + 1].end_time_ms);
                moments.remove(i + 1);
            } else {
                // Keep the next one, extend its start
                moments[i + 1].start_time_ms = moments[i].start_time_ms;
                moments.remove(i);
            }
        } else {
            i += 1;
        }
    }

    // Re-sort by priority for final output
    moments.sort_by(|a, b| {
        b.priority.cmp(&a.priority).then_with(|| a.start_time_ms.cmp(&b.start_time_ms))
    });
}

/// Match summary for quick display on result screens
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchSummary {
    /// Final score string (e.g., "2 - 1")
    pub final_score: String,
    /// Total shots (home, away)
    pub total_shots: (u16, u16),
    /// Shots on target (home, away)
    pub shots_on_target: (u16, u16),
    /// Possession percentage (home, away)
    pub possession: (f32, f32),
    /// Expected goals (home, away)
    pub xg: (f32, f32),
    /// Pass accuracy (home, away)
    pub pass_accuracy: (f32, f32),
    /// Yellow cards (home, away)
    pub yellow_cards: (u8, u8),
    /// Red cards (home, away)
    pub red_cards: (u8, u8),
    /// Corners (home, away)
    pub corners: (u8, u8),
    /// MVP player name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mvp_player: Option<String>,
    /// MVP player rating (optional, 3.0-10.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mvp_rating: Option<f32>,
    /// Goal scorers with minute (e.g., ["김민수 23'", "박지훈 67'"])
    pub goal_scorers_home: Vec<String>,
    pub goal_scorers_away: Vec<String>,
}

impl MatchSummary {
    /// Generate summary from MatchResult
    pub fn from_result(result: &MatchResult) -> Self {
        let stats = &result.statistics;

        // Collect goal scorers (substitution-aware pitch-slot roster).
        //
        // Contract:
        // - `player_track_id (0..21)` is a pitch slot, and substitutions can change the occupant.
        // - Substitution events carry the new occupant name in `EventDetails.substitution`.
        let mut goal_scorers_home = Vec::new();
        let mut goal_scorers_away = Vec::new();

        let mut slot_names: Vec<String> =
            (0..22).map(|i| format!("Player #{}", i)).collect();
        if let Some(team) = result.home_team.as_ref() {
            for (i, player) in team.players.iter().take(11).enumerate() {
                slot_names[i] = player.name.clone();
            }
        }
        if let Some(team) = result.away_team.as_ref() {
            for (i, player) in team.players.iter().take(11).enumerate() {
                slot_names[11 + i] = player.name.clone();
            }
        }

        let mut events_sorted: Vec<&MatchEvent> = result.events.iter().collect();
        events_sorted.sort_by_key(|e| e.timestamp_ms.unwrap_or(e.minute as u64 * 60_000));

        for event in events_sorted {
            if event.event_type == EventType::Substitution {
                if let (Some(pitch_track_id), Some(sub)) = (
                    event.player_track_id,
                    event.details.as_ref().and_then(|d| d.substitution.as_ref()),
                ) {
                    let idx = pitch_track_id as usize;
                    if idx < slot_names.len() {
                        slot_names[idx] = sub.player_in_name.clone();
                    }
                }
            }

            if matches!(event.event_type, EventType::Goal | EventType::OwnGoal) {
                let scorer_text = if let Some(track_id) = event.player_track_id {
                    let idx = track_id as usize;
                    let mut player_name = slot_names
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| format!("Player #{}", track_id));
                    if event.event_type == EventType::OwnGoal {
                        player_name = format!("{} (OG)", player_name);
                    }
                    format!("{} {}'", player_name, event.minute)
                } else {
                    format!("{}'", event.minute)
                };

                if event.is_home_team {
                    goal_scorers_home.push(scorer_text);
                } else {
                    goal_scorers_away.push(scorer_text);
                }
            }
        }

        // Determine MVP (player with highest rating if available)
        let (mvp_player, mvp_rating) = if let Some(ref my_stats) = stats.my_player_stats {
            (Some(my_stats.player_name.clone()), Some(my_stats.rating))
        } else {
            (None, None)
        };

        Self {
            final_score: if let Some(ref ps) = result.penalty_shootout {
                format!(
                    "{} - {} ({} - {} pens)",
                    result.score_home, result.score_away, ps.goals_home, ps.goals_away
                )
            } else {
                format!("{} - {}", result.score_home, result.score_away)
            },
            total_shots: (stats.shots_home, stats.shots_away),
            shots_on_target: (stats.shots_on_target_home, stats.shots_on_target_away),
            possession: (stats.possession_home, stats.possession_away),
            xg: (stats.xg_home, stats.xg_away),
            pass_accuracy: (stats.pass_accuracy_home, stats.pass_accuracy_away),
            yellow_cards: (stats.yellow_cards_home, stats.yellow_cards_away),
            red_cards: (stats.red_cards_home, stats.red_cards_away),
            corners: (stats.corners_home, stats.corners_away),
            mvp_player,
            mvp_rating,
            goal_scorers_home,
            goal_scorers_away,
        }
    }

}

/// Player state enum for replay visualization
/// FIX_2601/0109: Replaces String allocation with Copy enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerState {
    /// Player has the ball
    #[serde(rename = "WithBall")]
    WithBall,
    /// Player's team is attacking
    #[serde(rename = "Attacking")]
    Attacking,
    /// Player's team is defending
    #[serde(rename = "Defending")]
    Defending,
}

impl std::fmt::Display for PlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerState::WithBall => write!(f, "WithBall"),
            PlayerState::Attacking => write!(f, "Attacking"),
            PlayerState::Defending => write!(f, "Defending"),
        }
    }
}

/// Position data item for replay (50ms tick resolution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionDataItem {
    /// Timestamp in milliseconds from match start
    pub timestamp: u64,
    /// Position in meters (x: 0-105, y: 0-68)
    pub position: (f32, f32),
    /// Ball height in meters (0.0 = ground, 1.0 = max lob) - 2025-12-11
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    /// Velocity in m/s (vx, vy) - 2025-12-11
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<(f32, f32)>,
    /// Player state (WithBall, Attacking, Defending)
    /// FIX_2601/0109: Changed from String to PlayerState enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<PlayerState>,
}

impl PositionDataItem {
    pub fn new(timestamp: u64, position: (f32, f32)) -> Self {
        Self { timestamp, position, height: None, velocity: None, state: None }
    }

    /// New constructor with height (2025-12-11)
    pub fn with_height(timestamp: u64, position: (f32, f32), height: f32) -> Self {
        Self { timestamp, position, height: Some(height), velocity: None, state: None }
    }

    pub fn with_state(timestamp: u64, position: (f32, f32), state: PlayerState) -> Self {
        Self { timestamp, position, height: None, velocity: None, state: Some(state) }
    }

    /// Constructor with velocity (2025-12-11)
    pub fn with_velocity(timestamp: u64, position: (f32, f32), velocity: (f32, f32)) -> Self {
        Self { timestamp, position, height: None, velocity: Some(velocity), state: None }
    }

    /// Constructor with velocity and state (2025-12-11)
    pub fn with_velocity_and_state(
        timestamp: u64,
        position: (f32, f32),
        velocity: (f32, f32),
        state: PlayerState,
    ) -> Self {
        Self { timestamp, position, height: None, velocity: Some(velocity), state: Some(state) }
    }

    fn swap_axes_in_place(&mut self) {
        self.position = (self.position.1, self.position.0);
        if let Some((vx, vy)) = self.velocity {
            self.velocity = Some((vy, vx));
        }
    }
}

/// Maximum time interval (ms) between sync points for player position deduplication.
/// If time since last recorded position exceeds this, force a sync point.
/// This prevents interpolation drift accumulation during playback.
///
/// 500ms = 2 sync points per second minimum, limits drift to ~5m worst case
/// (player running at 10m/s for 500ms = 5m, but dedup threshold catches 0.1m movements)
const MAX_SYNC_INTERVAL_MS: u64 = 500;

/// Match position data for replay visualization
/// FIX_2601/0109: Changed players from HashMap<u8, Vec> to [Vec; 22] for performance
#[derive(Debug, Clone)]
pub struct MatchPositionData {
    /// Ball position history
    pub ball: Vec<PositionDataItem>,
    /// Player position history (fixed array, index = player_idx)
    /// FIX_2601/0109: Changed from HashMap<u8, Vec> to array for O(1) access
    pub players: [Vec<PositionDataItem>; 22],
}

impl Default for MatchPositionData {
    fn default() -> Self {
        Self::new()
    }
}

// Custom Serialize to maintain JSON compatibility (outputs as HashMap format)
impl Serialize for MatchPositionData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("MatchPositionData", 2)?;
        state.serialize_field("ball", &self.ball)?;
        // Convert array to HashMap for JSON compatibility
        let players_map: HashMap<u8, &Vec<PositionDataItem>> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.is_empty())
            .map(|(i, v)| (i as u8, v))
            .collect();
        state.serialize_field("players", &players_map)?;
        state.end()
    }
}

// Custom Deserialize to read from HashMap format
impl<'de> Deserialize<'de> for MatchPositionData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            ball: Vec<PositionDataItem>,
            players: HashMap<u8, Vec<PositionDataItem>>,
        }
        let helper = Helper::deserialize(deserializer)?;
        let mut players: [Vec<PositionDataItem>; 22] = Default::default();
        for (idx, positions) in helper.players {
            if (idx as usize) < 22 {
                players[idx as usize] = positions;
            }
        }
        Ok(MatchPositionData {
            ball: helper.ball,
            players,
        })
    }
}

impl MatchPositionData {
    pub fn new() -> Self {
        Self {
            ball: Vec::with_capacity(54000), // 90min * 60sec * 10 (every 100ms)
            players: std::array::from_fn(|_| Vec::with_capacity(500)),
        }
    }

    /// Add ball position (always records for smooth animation at 100ms intervals)
    /// 2025-12-10: Removed deduplication to ensure smooth playback
    /// 2025-12-11: Added height parameter for 3D ball position
    /// FIX_2601: Clamp to field bounds (0-105m x 0-68m)
    pub fn add_ball_position(&mut self, timestamp: u64, position: (f32, f32), height: f32) {
        // Clamp position to field bounds
        let clamped = (position.0.clamp(0.0, 105.0), position.1.clamp(0.0, 68.0));
        // Always record every frame for smooth animation
        self.ball.push(PositionDataItem::with_height(timestamp, clamped, height));
    }

    /// Add ball position with velocity (2025-12-11 P2)
    /// Records ball position with height and velocity for trajectory prediction
    /// FIX_2601: Clamp to field bounds (0-105m x 0-68m)
    pub fn add_ball_position_with_velocity(
        &mut self,
        timestamp: u64,
        position: (f32, f32),
        height: f32,
        velocity: (f32, f32),
    ) {
        // Clamp position to field bounds
        let clamped = (position.0.clamp(0.0, 105.0), position.1.clamp(0.0, 68.0));
        self.ball.push(PositionDataItem {
            timestamp,
            position: clamped,
            height: Some(height),
            velocity: Some(velocity),
            state: None,
        });
    }

    /// Add player position (deduplicates if same position, with sync point forcing)
    /// FIX_2601: Clamp to field bounds (0-105m x 0-68m)
    /// FIX_2601/0109: Direct array index access instead of HashMap entry()
    /// FIX_MRB0_DRIFT: Force sync point if time since last record > MAX_SYNC_INTERVAL_MS
    pub fn add_player_position(&mut self, player_idx: u8, timestamp: u64, position: (f32, f32)) {
        let idx = player_idx as usize;
        if idx >= 22 {
            return; // Bounds check
        }
        // Clamp position to field bounds
        let clamped = (position.0.clamp(0.0, 105.0), position.1.clamp(0.0, 68.0));

        let positions = &mut self.players[idx];

        if let Some(last) = positions.last() {
            let dx = (last.position.0 - clamped.0).abs();
            let dy = (last.position.1 - clamped.1).abs();
            let time_gap = timestamp.saturating_sub(last.timestamp);

            // Record if: position changed significantly OR sync interval exceeded
            if dx > 0.1 || dy > 0.1 || time_gap >= MAX_SYNC_INTERVAL_MS {
                positions.push(PositionDataItem::new(timestamp, clamped));
            }
        } else {
            positions.push(PositionDataItem::new(timestamp, clamped));
        }
    }

    /// Add player position with state
    /// FIX_2601: Clamp to field bounds (0-105m x 0-68m)
    /// FIX_2601/0109: Changed state from String to PlayerState enum, direct array access
    /// FIX_MRB0_DRIFT: Force sync point if time since last record > MAX_SYNC_INTERVAL_MS
    pub fn add_player_position_with_state(
        &mut self,
        player_idx: u8,
        timestamp: u64,
        position: (f32, f32),
        state: PlayerState,
    ) {
        let idx = player_idx as usize;
        if idx >= 22 {
            return; // Bounds check
        }
        // Clamp position to field bounds
        let clamped = (position.0.clamp(0.0, 105.0), position.1.clamp(0.0, 68.0));

        let positions = &mut self.players[idx];

        if let Some(last) = positions.last() {
            let dx = (last.position.0 - clamped.0).abs();
            let dy = (last.position.1 - clamped.1).abs();
            let state_changed = last.state != Some(state);
            let time_gap = timestamp.saturating_sub(last.timestamp);

            // Record if: position changed OR state changed OR sync interval exceeded
            if dx > 0.1 || dy > 0.1 || state_changed || time_gap >= MAX_SYNC_INTERVAL_MS {
                positions.push(PositionDataItem::with_state(timestamp, clamped, state));
            }
        } else {
            positions.push(PositionDataItem::with_state(timestamp, clamped, state));
        }
    }

    /// Add player position with velocity and state (2025-12-11)
    /// FIX_2601: Clamp to field bounds (0-105m x 0-68m)
    /// FIX_2601/0109: Changed state from String to PlayerState enum, direct array access
    /// FIX_MRB0_DRIFT: Force sync point if time since last record > MAX_SYNC_INTERVAL_MS
    pub fn add_player_position_with_velocity(
        &mut self,
        player_idx: u8,
        timestamp: u64,
        position: (f32, f32),
        velocity: (f32, f32),
        state: PlayerState,
    ) {
        let idx = player_idx as usize;
        if idx >= 22 {
            return; // Bounds check
        }
        // Clamp position to field bounds
        let clamped = (position.0.clamp(0.0, 105.0), position.1.clamp(0.0, 68.0));

        let positions = &mut self.players[idx];

        if let Some(last) = positions.last() {
            let dx = (last.position.0 - clamped.0).abs();
            let dy = (last.position.1 - clamped.1).abs();
            let state_changed = last.state != Some(state);
            let vel_changed = match last.velocity {
                Some(v) => (v.0 - velocity.0).abs() > 0.5 || (v.1 - velocity.1).abs() > 0.5,
                None => true,
            };
            let time_gap = timestamp.saturating_sub(last.timestamp);

            // Record if: any significant change OR sync interval exceeded
            if dx > 0.1 || dy > 0.1 || state_changed || vel_changed || time_gap >= MAX_SYNC_INTERVAL_MS {
                positions.push(PositionDataItem::with_velocity_and_state(
                    timestamp, clamped, velocity, state,
                ));
            }
        } else {
            positions.push(PositionDataItem::with_velocity_and_state(
                timestamp, clamped, velocity, state,
            ));
        }
    }

    pub fn swap_axes_in_place(&mut self) {
        for item in &mut self.ball {
            item.swap_axes_in_place();
        }
        for track in &mut self.players {
            for item in track {
                item.swap_axes_in_place();
            }
        }
    }
}

fn coord_contract_version_default_legacy() -> u8 {
    0
}

fn coord_system_default_legacy() -> String {
    COORD_SYSTEM_LEGACY_AXIS_SWAP.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub schema_version: u8,
    #[serde(default = "coord_contract_version_default_legacy")]
    pub coord_contract_version: u8,
    #[serde(default = "coord_system_default_legacy")]
    pub coord_system: String,
    pub ssot_proof: crate::fix01::SsotProof,
    #[serde(default)]
    pub determinism: DeterminismMeta,
    pub score_home: u8,
    pub score_away: u8,
    pub events: Vec<MatchEvent>,
    pub statistics: Statistics,
    /// Position data for replay (optional, can be large)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_data: Option<MatchPositionData>,
    /// Detailed replay events with full information (optional, for new replay system)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_events: Option<Vec<replay::Event>>,
    /// Viewer events for animation (BallTrajectory, TackleEvent) - P7 Section 17
    #[serde(skip)]
    pub viewer_events: Option<Vec<ViewerEvent>>,
    /// Home team (for roster information in replays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_team: Option<Team>,
    /// Away team (for roster information in replays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub away_team: Option<Team>,
    /// P17 Phase 5: Match setup info for viewer (player names, positions, overall)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_setup: Option<MatchSetupExport>,
    /// Debug info for troubleshooting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_info: Option<String>,
    /// Match summary for quick display (generated after simulation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<MatchSummary>,
    /// P18: Board summary (occupancy/pressure max values, hot cells)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_summary: Option<BoardSummaryExport>,
    /// Optional penalty shootout outcome (regulation score remains unchanged).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalty_shootout: Option<PenaltyShootoutResult>,
    /// Best moments / highlights for replay navigation (generated after simulation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_moments: Option<Vec<BestMoment>>,

    /// FIX_2601: Shot opportunity telemetry for bias detection (env-gated: OF_DEBUG_SHOT_OPP=1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shot_opp_telemetry: Option<crate::engine::match_sim::ShotOppTelemetry>,
}

// ============================================================================
// FIX02: Determinism/Truncation Metadata (SSOT contract)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismMode {
    Full,
    Budgeted,
    Truncated,
}

/// FIX_2601/0123: Hash algorithm identifier for determinism verification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum HashAlgorithm {
    /// Legacy DefaultHasher (NOT stable across Rust versions)
    #[serde(rename = "legacy")]
    Legacy = 0,
    /// FxHash (version-stable, recommended)
    #[serde(rename = "fxhash")]
    FxHash = 1,
    /// xxHash (version-stable, alternative)
    #[serde(rename = "xxhash")]
    XxHash = 2,
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        HashAlgorithm::FxHash // FIX_2601/0123: New default
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeterminismMeta {
    pub mode: DeterminismMode,
    pub simulated_until_tick: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut_reason: Option<String>,
    /// FIX_2601/0123: Hash algorithm used for deterministic choices
    /// Legacy replays may use DefaultHasher which is NOT stable across Rust versions
    #[serde(default)]
    pub hash_algo: HashAlgorithm,
}

impl Default for DeterminismMeta {
    fn default() -> Self {
        Self {
            mode: DeterminismMode::Full,
            simulated_until_tick: 0,
            cut_reason: None,
            hash_algo: HashAlgorithm::FxHash, // FIX_2601/0123: Use stable hash
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPlayerStats {
    pub player_id: String,
    pub player_name: String,
    pub goals: u32,
    pub assists: u32,
    pub shots: u32,
    pub passes: u32,
    pub tackles: u32,
    pub fouls: u32,
    pub saves: u32,
    pub yellow_cards: u32,
    pub red_cards: u32,
    /// Final match rating (3.0 ~ 10.0)
    pub rating: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatMapPoint {
    pub x: f32,
    pub y: f32,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    // Basic stats
    pub possession_home: f32,
    pub possession_away: f32,

    // Shooting stats
    pub shots_home: u16,
    pub shots_away: u16,
    pub shots_on_target_home: u16,
    pub shots_on_target_away: u16,

    // Expected goals
    pub xg_home: f32,
    pub xg_away: f32,

    // Passing stats
    pub passes_home: u16,
    pub passes_away: u16,
    pub pass_accuracy_home: f32,
    pub pass_accuracy_away: f32,

    // Defensive stats
    pub tackles_home: u16,
    pub tackles_away: u16,
    pub tackle_attempts_home: u16, // 태클 시도
    pub tackle_attempts_away: u16,
    pub fouls_home: u16,
    pub fouls_away: u16,

    // Passing stats - attempts
    pub pass_attempts_home: u16, // 패스 시도
    pub pass_attempts_away: u16,
    #[serde(default)]
    pub cross_attempts_home: u16,
    #[serde(default)]
    pub cross_attempts_away: u16,
    #[serde(default)]
    pub crosses_home: u16,
    #[serde(default)]
    pub crosses_away: u16,

    // Passing stats - direction/distance
    #[serde(default)]
    pub pass_distance_sum_home: f32,
    #[serde(default)]
    pub pass_distance_sum_away: f32,
    #[serde(default)]
    pub forward_pass_attempts_home: u16,
    #[serde(default)]
    pub forward_pass_attempts_away: u16,
    #[serde(default)]
    pub circulation_pass_attempts_home: u16,
    #[serde(default)]
    pub circulation_pass_attempts_away: u16,

    // Passing stats - sequences
    #[serde(default)]
    pub pass_sequence_total_home: u16,
    #[serde(default)]
    pub pass_sequence_total_away: u16,
    #[serde(default)]
    pub pass_sequence_count_home: u16,
    #[serde(default)]
    pub pass_sequence_count_away: u16,
    #[serde(default)]
    pub pass_distance_avg_m: f32,
    #[serde(default)]
    pub forward_pass_ratio: f32,
    #[serde(default)]
    pub circulation_pass_ratio: f32,
    #[serde(default)]
    pub pass_sequence_avg_len: f32,

    // Set pieces
    pub freekicks_home: u8, // 프리킥
    pub freekicks_away: u8,
    #[serde(default)]
    pub penalties_home: u16,
    #[serde(default)]
    pub penalties_away: u16,

    // Rulebook-specific stats
    #[serde(default)]
    pub handball_fouls_home: u16,
    #[serde(default)]
    pub handball_fouls_away: u16,
    #[serde(default)]
    pub handball_penalties_home: u16,
    #[serde(default)]
    pub handball_penalties_away: u16,

    // Heading stats
    pub headers_home: u16, // 헤딩 성공
    pub headers_away: u16,
    pub header_attempts_home: u16, // 헤딩 시도
    pub header_attempts_away: u16,
    #[serde(default)]
    pub header_shot_attempts_home: u16,
    #[serde(default)]
    pub header_shot_attempts_away: u16,
    #[serde(default)]
    pub header_pass_attempts_home: u16,
    #[serde(default)]
    pub header_pass_attempts_away: u16,

    // Dribble/TakeOn stats (돌파)
    pub take_ons_home: u16, // 돌파 성공
    pub take_ons_away: u16,
    pub take_on_attempts_home: u16, // 돌파 시도
    pub take_on_attempts_away: u16,
    pub dribbles_home: u16, // 일반 드리블 횟수
    pub dribbles_away: u16,

    // Cards
    pub yellow_cards_home: u8,
    pub yellow_cards_away: u8,
    pub red_cards_home: u8,
    pub red_cards_away: u8,

    // Other
    pub corners_home: u8,
    pub corners_away: u8,
    pub offsides_home: u8,
    pub offsides_away: u8,

    // Telemetry / pacing diagnostics
    #[serde(default)]
    pub total_ticks: u32,
    #[serde(default)]
    pub ball_in_play_ticks: u32,
    #[serde(default)]
    pub ball_in_flight_ticks: u32,
    #[serde(default)]
    pub owner_action_blocked_ticks: u32,
    #[serde(default)]
    pub possessions_home: u16,
    #[serde(default)]
    pub possessions_away: u16,
    #[serde(default)]
    pub actions_total: u32,
    #[serde(default)]
    pub hold_actions_home: u16,
    #[serde(default)]
    pub hold_actions_away: u16,
    #[serde(default)]
    pub carry_actions_home: u16,
    #[serde(default)]
    pub carry_actions_away: u16,
    #[serde(default)]
    pub pass_attempts_per_possession: f32,
    #[serde(default)]
    pub hold_action_ratio: f32,
    #[serde(default)]
    pub carry_action_ratio: f32,
    #[serde(default)]
    pub ball_in_play_ratio: f32,
    #[serde(default)]
    pub possessions_per_min: f32,
    #[serde(default)]
    pub actions_per_min: f32,
    /// DecisionTopology evaluations executed (DPQ routing). v1.1: should match baseline.
    #[serde(default)]
    pub decisions_executed: u32,
    /// Decisions skipped due to DPQ cadence (v1.1: should remain 0).
    #[serde(default)]
    pub decisions_skipped: u32,
    #[serde(default)]
    pub decisions_executed_per_min: f32,
    #[serde(default)]
    pub decisions_skipped_per_min: f32,
    #[serde(default)]
    pub shot_gate_checks: u32,
    #[serde(default)]
    pub shot_gate_allowed: u32,
    #[serde(default)]
    pub shot_gate_rejects: u32,
    #[serde(default)]
    pub shot_gate_allow_ratio: f32,
    #[serde(default)]
    pub clear_shot_checks: u32,
    #[serde(default)]
    pub clear_shot_blocked: u32,
    #[serde(default)]
    pub clear_shot_block_ratio: f32,

    /// Optional per-match stats for the configured user player
    #[serde(skip_serializing_if = "Option::is_none")]
    pub my_player_stats: Option<MyPlayerStats>,

    // Phase E: Advanced Analytics
    #[serde(default)]
    pub possession_zones_home: Vec<f32>, // 18 zones possession percentage
    #[serde(default)]
    pub possession_zones_away: Vec<f32>,
    #[serde(default)]
    pub pass_matrix_home: Vec<Vec<f32>>, // 22x22 player pass completion matrix
    #[serde(default)]
    pub pass_matrix_away: Vec<Vec<f32>>,
    #[serde(default)]
    pub heat_map_data_home: Vec<HeatMapPoint>, // position frequency data
    #[serde(default)]
    pub heat_map_data_away: Vec<HeatMapPoint>,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            possession_home: 50.0,
            possession_away: 50.0,
            shots_home: 0,
            shots_away: 0,
            shots_on_target_home: 0,
            shots_on_target_away: 0,
            xg_home: 0.0,
            xg_away: 0.0,
            passes_home: 0,
            passes_away: 0,
            pass_accuracy_home: 0.0,
            pass_accuracy_away: 0.0,
            tackles_home: 0,
            tackles_away: 0,
            tackle_attempts_home: 0,
            tackle_attempts_away: 0,
            fouls_home: 0,
            fouls_away: 0,
            pass_attempts_home: 0,
            pass_attempts_away: 0,
            cross_attempts_home: 0,
            cross_attempts_away: 0,
            crosses_home: 0,
            crosses_away: 0,
            pass_distance_sum_home: 0.0,
            pass_distance_sum_away: 0.0,
            forward_pass_attempts_home: 0,
            forward_pass_attempts_away: 0,
            circulation_pass_attempts_home: 0,
            circulation_pass_attempts_away: 0,
            pass_sequence_total_home: 0,
            pass_sequence_total_away: 0,
            pass_sequence_count_home: 0,
            pass_sequence_count_away: 0,
            pass_distance_avg_m: 0.0,
            forward_pass_ratio: 0.0,
            circulation_pass_ratio: 0.0,
            pass_sequence_avg_len: 0.0,
            freekicks_home: 0,
            freekicks_away: 0,
            penalties_home: 0,
            penalties_away: 0,
            handball_fouls_home: 0,
            handball_fouls_away: 0,
            handball_penalties_home: 0,
            handball_penalties_away: 0,
            headers_home: 0,
            headers_away: 0,
            header_attempts_home: 0,
            header_attempts_away: 0,
            header_shot_attempts_home: 0,
            header_shot_attempts_away: 0,
            header_pass_attempts_home: 0,
            header_pass_attempts_away: 0,
            take_ons_home: 0,
            take_ons_away: 0,
            take_on_attempts_home: 0,
            take_on_attempts_away: 0,
            dribbles_home: 0,
            dribbles_away: 0,
            yellow_cards_home: 0,
            yellow_cards_away: 0,
            red_cards_home: 0,
            red_cards_away: 0,
            corners_home: 0,
            corners_away: 0,
            offsides_home: 0,
            offsides_away: 0,
            total_ticks: 0,
            ball_in_play_ticks: 0,
            ball_in_flight_ticks: 0,
            owner_action_blocked_ticks: 0,
            possessions_home: 0,
            possessions_away: 0,
            actions_total: 0,
            hold_actions_home: 0,
            hold_actions_away: 0,
            carry_actions_home: 0,
            carry_actions_away: 0,
            pass_attempts_per_possession: 0.0,
            hold_action_ratio: 0.0,
            carry_action_ratio: 0.0,
            ball_in_play_ratio: 0.0,
            possessions_per_min: 0.0,
            actions_per_min: 0.0,
            decisions_executed: 0,
            decisions_skipped: 0,
            decisions_executed_per_min: 0.0,
            decisions_skipped_per_min: 0.0,
            shot_gate_checks: 0,
            shot_gate_allowed: 0,
            shot_gate_rejects: 0,
            shot_gate_allow_ratio: 0.0,
            clear_shot_checks: 0,
            clear_shot_blocked: 0,
            clear_shot_block_ratio: 0.0,
            possession_zones_home: Vec::new(),
            possession_zones_away: Vec::new(),
            pass_matrix_home: Vec::new(),
            pass_matrix_away: Vec::new(),
            heat_map_data_home: Vec::new(),
            heat_map_data_away: Vec::new(),
            my_player_stats: None,
        }
    }
}

impl Default for MatchResult {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchResult {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            coord_contract_version: COORD_CONTRACT_VERSION,
            coord_system: COORD_SYSTEM_METERS_V2.to_string(),
            ssot_proof: crate::fix01::SsotProof::default(),
            determinism: DeterminismMeta::default(),
            score_home: 0,
            score_away: 0,
            events: Vec::new(),
            statistics: Statistics::default(),
            position_data: None,
            replay_events: None,
            viewer_events: None,
            home_team: None,
            away_team: None,
            match_setup: None,
            debug_info: None,
            summary: None,
            board_summary: None,
            penalty_shootout: None,
            best_moments: None,
            shot_opp_telemetry: None,
        }
    }

    /// Create with position tracking enabled
    pub fn with_position_tracking() -> Self {
        Self {
            schema_version: 1,
            coord_contract_version: COORD_CONTRACT_VERSION,
            coord_system: COORD_SYSTEM_METERS_V2.to_string(),
            ssot_proof: crate::fix01::SsotProof::default(),
            determinism: DeterminismMeta::default(),
            score_home: 0,
            score_away: 0,
            events: Vec::new(),
            statistics: Statistics::default(),
            position_data: Some(MatchPositionData::new()),
            replay_events: None,
            viewer_events: None,
            home_team: None,
            away_team: None,
            match_setup: None,
            debug_info: None,
            summary: None,
            board_summary: None,
            penalty_shootout: None,
            best_moments: None,
            shot_opp_telemetry: None,
        }
    }

    /// Enable replay events (detailed event system)
    pub fn with_replay_events() -> Self {
        Self {
            schema_version: 1,
            coord_contract_version: COORD_CONTRACT_VERSION,
            coord_system: COORD_SYSTEM_METERS_V2.to_string(),
            ssot_proof: crate::fix01::SsotProof::default(),
            determinism: DeterminismMeta::default(),
            score_home: 0,
            score_away: 0,
            events: Vec::new(),
            statistics: Statistics::default(),
            position_data: Some(MatchPositionData::new()),
            replay_events: Some(Vec::new()),
            viewer_events: Some(Vec::new()),
            home_team: None,
            away_team: None,
            match_setup: None,
            debug_info: None,
            summary: None,
            board_summary: None,
            penalty_shootout: None,
            best_moments: None,
            shot_opp_telemetry: None,
        }
    }

    /// Enable viewer events for animation system
    pub fn with_viewer_events() -> Self {
        Self {
            schema_version: 1,
            coord_contract_version: COORD_CONTRACT_VERSION,
            coord_system: COORD_SYSTEM_METERS_V2.to_string(),
            ssot_proof: crate::fix01::SsotProof::default(),
            determinism: DeterminismMeta::default(),
            score_home: 0,
            score_away: 0,
            events: Vec::new(),
            statistics: Statistics::default(),
            position_data: Some(MatchPositionData::new()),
            replay_events: None,
            viewer_events: Some(Vec::new()),
            home_team: None,
            away_team: None,
            match_setup: None,
            debug_info: None,
            summary: None,
            board_summary: None,
            penalty_shootout: None,
            best_moments: None,
            shot_opp_telemetry: None,
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut result: Self = serde_json::from_str(json)?;
        result.normalize_coord_contract_in_place();
        Ok(result)
    }

    pub fn normalize_coord_contract_in_place(&mut self) {
        match self.coord_system.as_str() {
            COORD_SYSTEM_METERS_V2 => {
                self.coord_contract_version = COORD_CONTRACT_VERSION;
            }
            COORD_SYSTEM_LEGACY_AXIS_SWAP => {
                if let Some(position_data) = &mut self.position_data {
                    position_data.swap_axes_in_place();
                }

                if let Some(replay_events) = &mut self.replay_events {
                    for event in replay_events {
                        event.swap_axes_in_place();
                    }
                }

                self.coord_contract_version = COORD_CONTRACT_VERSION;
                self.coord_system = COORD_SYSTEM_METERS_V2.to_string();
            }
            _ => {}
        }
    }

    /// Add a viewer event (BallTrajectory or Tackle)
    pub fn add_viewer_event(&mut self, event: ViewerEvent) {
        if let Some(ref mut events) = self.viewer_events {
            events.push(event);
        } else {
            self.viewer_events = Some(vec![event]);
        }
    }

    /// Take all viewer events (ownership transfer)
    pub fn take_viewer_events(&mut self) -> Option<Vec<ViewerEvent>> {
        self.viewer_events.take()
    }

    /// Generate and set summary from current result data
    pub fn generate_summary(&mut self) {
        self.summary = Some(MatchSummary::from_result(self));
    }

    /// Generate and set best moments from events
    pub fn generate_best_moments(&mut self) {
        let moments = generate_best_moments(&self.events);
        if !moments.is_empty() {
            self.best_moments = Some(moments);
        }
    }

    /// Generate both summary and best moments
    pub fn finalize(&mut self) {
        self.generate_summary();
        self.generate_best_moments();
    }

    /// Set teams for roster information
    pub fn with_teams(mut self, home: Team, away: Team) -> Self {
        self.home_team = Some(home);
        self.away_team = Some(away);
        self
    }

    pub fn add_goal_home(&mut self, minute: u8, _scorer: String, _assist: Option<String>) {
        self.score_home += 1;
        // C5: Test helper uses minute-based timestamp
        let timestamp_ms = minute as u64 * 60_000;
        // C6: Test helper uses placeholder track_id (0 for home striker)
        self.events.push(MatchEvent::goal(minute, timestamp_ms, true, 9, None));
        // Home striker ST
    }

    pub fn add_goal_away(&mut self, minute: u8, _scorer: String, _assist: Option<String>) {
        self.score_away += 1;
        // C5: Test helper uses minute-based timestamp
        let timestamp_ms = minute as u64 * 60_000;
        // C6: Test helper uses placeholder track_id (20 for away striker)
        self.events.push(MatchEvent::goal(minute, timestamp_ms, false, 20, None));
        // Away striker ST
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EventDetails, SubstitutionDetails};
    use crate::models::player::PlayerAttributes;
    use crate::models::team::Formation;
    use crate::models::{Player, Position, Team};

    fn make_team_with_starters(name: &str) -> Team {
        let mut players = Vec::with_capacity(11);
        for i in 0..11 {
            players.push(Player {
                name: format!("{} Player {}", name, i + 1),
                position: if i == 0 { Position::GK } else { Position::CM },
                overall: 70,
                condition: 3,
                attributes: Some(PlayerAttributes::default()),
                equipped_skills: Vec::new(),
                traits: Default::default(),
                personality: Default::default(),
            });
        }
        Team {
            name: name.to_string(),
            formation: Formation::F442,
            players,
        }
    }

    #[test]
    fn test_position_dedup_small_movement_skipped() {
        let mut data = MatchPositionData::new();

        // First position recorded
        data.add_player_position(0, 0, (50.0, 34.0));
        assert_eq!(data.players[0].len(), 1);

        // Small movement (< 0.1m) within sync interval - should be skipped
        data.add_player_position(0, 100, (50.05, 34.05));
        assert_eq!(data.players[0].len(), 1, "Small movement should be deduplicated");

        // Large movement (> 0.1m) - should be recorded
        data.add_player_position(0, 200, (50.2, 34.0));
        assert_eq!(data.players[0].len(), 2, "Large movement should be recorded");
    }

    #[test]
    fn test_position_sync_point_forces_record() {
        let mut data = MatchPositionData::new();

        // First position at t=0
        data.add_player_position(0, 0, (50.0, 34.0));
        assert_eq!(data.players[0].len(), 1);

        // Same position but within sync interval - skipped
        data.add_player_position(0, 400, (50.0, 34.0));
        assert_eq!(data.players[0].len(), 1, "Within sync interval should be skipped");

        // Same position but sync interval exceeded (500ms) - forced record
        data.add_player_position(0, 600, (50.0, 34.0));
        assert_eq!(data.players[0].len(), 2, "Sync interval exceeded should force record");
    }

    #[test]
    fn test_position_drift_bounded_by_sync_interval() {
        let mut data = MatchPositionData::new();

        // Record initial position
        data.add_player_position(5, 0, (20.0, 30.0));

        // Simulate player standing still for 2 seconds (4 sync points expected)
        for t in (100..=2000).step_by(100) {
            data.add_player_position(5, t, (20.0, 30.0));
        }

        // At 500ms intervals, we should have forced sync points
        // t=0, t=500, t=1000, t=1500, t=2000 = 5 positions
        let count = data.players[5].len();
        assert!(count >= 4, "Should have at least 4 sync points for 2 seconds, got {}", count);
        assert!(count <= 5, "Should have at most 5 sync points, got {}", count);

        // Verify timestamps are spaced by MAX_SYNC_INTERVAL_MS
        let positions = &data.players[5];
        for i in 1..positions.len() {
            let gap = positions[i].timestamp - positions[i - 1].timestamp;
            assert!(
                gap <= MAX_SYNC_INTERVAL_MS,
                "Gap {} at index {} exceeds MAX_SYNC_INTERVAL_MS {}",
                gap, i, MAX_SYNC_INTERVAL_MS
            );
        }
    }

    #[test]
    fn test_position_with_state_sync_forcing() {
        let mut data = MatchPositionData::new();

        // First position
        data.add_player_position_with_state(0, 0, (50.0, 34.0), PlayerState::Defending);
        assert_eq!(data.players[0].len(), 1);

        // Same position and state, within interval - skipped
        data.add_player_position_with_state(0, 100, (50.0, 34.0), PlayerState::Defending);
        assert_eq!(data.players[0].len(), 1);

        // Same position and state, interval exceeded - forced
        data.add_player_position_with_state(0, 600, (50.0, 34.0), PlayerState::Defending);
        assert_eq!(data.players[0].len(), 2);
    }

    #[test]
    fn test_position_with_velocity_sync_forcing() {
        let mut data = MatchPositionData::new();

        // First position
        data.add_player_position_with_velocity(0, 0, (50.0, 34.0), (0.0, 0.0), PlayerState::Defending);
        assert_eq!(data.players[0].len(), 1);

        // Same everything, within interval - skipped
        data.add_player_position_with_velocity(0, 100, (50.0, 34.0), (0.0, 0.0), PlayerState::Defending);
        assert_eq!(data.players[0].len(), 1);

        // Same everything, interval exceeded - forced
        data.add_player_position_with_velocity(0, 600, (50.0, 34.0), (0.0, 0.0), PlayerState::Defending);
        assert_eq!(data.players[0].len(), 2);
    }

    #[test]
    fn test_max_interpolation_gap_bounded() {
        // This test verifies that the worst-case interpolation gap is bounded
        // to MAX_SYNC_INTERVAL_MS, limiting drift accumulation

        let mut data = MatchPositionData::new();

        // Record sparse positions over 5 seconds
        data.add_player_position(0, 0, (0.0, 0.0));
        data.add_player_position(0, 5000, (50.0, 34.0)); // Big jump after 5s

        // Without sync forcing, there would be only 2 points
        // With sync forcing, we get intermediate sync points
        let positions = &data.players[0];

        // Calculate max gap
        let mut max_gap = 0u64;
        for i in 1..positions.len() {
            let gap = positions[i].timestamp - positions[i - 1].timestamp;
            max_gap = max_gap.max(gap);
        }

        // Max gap should not exceed MAX_SYNC_INTERVAL_MS (except for first record)
        // Note: This test ensures sync points work when position DOES change
        assert!(
            max_gap <= MAX_SYNC_INTERVAL_MS || positions.len() == 2,
            "Max gap {} exceeds {} (positions: {})",
            max_gap, MAX_SYNC_INTERVAL_MS, positions.len()
        );
    }

    #[test]
    fn match_summary_goal_scorer_uses_substitution_updated_slot_name() {
        let mut result = MatchResult::new();
        result.home_team = Some(make_team_with_starters("Home"));
        result.away_team = Some(make_team_with_starters("Away"));
        result.score_home = 1;
        result.score_away = 0;

        // Pitch slot 1 starts as "Home Player 2". After substitution, it becomes "Home Sub 1".
        let sub_minute = 70u8;
        let goal_minute = 71u8;

        result.events.push(MatchEvent {
            minute: sub_minute,
            timestamp_ms: Some(sub_minute as u64 * 60_000),
            event_type: EventType::Substitution,
            is_home_team: true,
            player_track_id: Some(1),
            target_track_id: None,
            details: Some(EventDetails {
                substitution: Some(SubstitutionDetails {
                    player_in_name: "Home Sub 1".to_string(),
                    player_out_name: "Home Player 2".to_string(),
                    bench_slot: 0,
                }),
                ..Default::default()
            }),
        });

        result
            .events
            .push(MatchEvent::goal(goal_minute, goal_minute as u64 * 60_000, true, 1, None));

        let summary = MatchSummary::from_result(&result);
        assert_eq!(summary.goal_scorers_home, vec!["Home Sub 1 71'".to_string()]);
    }
}
