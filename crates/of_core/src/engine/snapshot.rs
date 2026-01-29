//! State Snapshot API for MatchEngine
//!
//! Google Football-style `get_state()`/`set_state()` for checkpointing and restoring
//! match state. Enables deterministic replay, AI training, and save/load functionality.
//!
//! # Example
//! ```ignore
//! let mut engine = MatchEngine::new(/* ... */);
//! engine.tick_n(100);
//!
//! // Capture state
//! let snapshot = engine.get_state();
//!
//! // Continue simulation
//! engine.tick_n(50);
//!
//! // Restore to checkpoint
//! engine.set_state(snapshot)?;
//! ```

use serde::{Deserialize, Serialize};

use super::action_queue::{ActiveAction, BallState, ScheduledAction};
use super::ball::Ball;
use super::player_state::PlayerState;
use super::types::coord10::{Coord10, Vel10};
use super::types::PlayerReactionState;
use super::GameState;

/// Error type for snapshot operations
#[derive(Debug, Clone)]
pub enum SnapshotError {
    /// Player count mismatch
    PlayerCountMismatch { expected: usize, actual: usize },
    /// RNG restoration failed
    RngRestoreError(String),
    /// Invalid snapshot data
    InvalidData(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::PlayerCountMismatch { expected, actual } => {
                write!(f, "Player count mismatch: expected {}, got {}", expected, actual)
            }
            SnapshotError::RngRestoreError(msg) => write!(f, "RNG restore error: {}", msg),
            SnapshotError::InvalidData(msg) => write!(f, "Invalid snapshot data: {}", msg),
        }
    }
}

impl std::error::Error for SnapshotError {}

fn default_first_half_end_minute() -> u8 {
    45
}

fn default_match_end_minute() -> u8 {
    90
}

/// Complete match state snapshot for checkpoint/restore
///
/// Contains all mutable state needed to deterministically restore a match.
/// Static configuration (teams, tactics) is not included as it doesn't change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchStateSnapshot {
    // ========== Time State ==========
    /// Current tick within the match
    pub current_tick: u64,
    /// FIX_2601/0119: Last kickoff tick for stabilization logic
    #[serde(default)]
    pub last_kickoff_tick: u64,
    /// Current match minute
    pub minute: u8,
    /// Is this the second half?
    pub is_second_half: bool,
    /// Timestamp in milliseconds
    pub current_timestamp_ms: u64,
    /// Added-time accumulator for first half (seconds).
    #[serde(default)]
    pub stoppage_seconds_first_half: u16,
    /// Whether 1H added time has been finalized at minute 45.
    #[serde(default)]
    pub stoppage_finalized_first_half: bool,
    /// Actual first half end minute (45 + added time).
    #[serde(default = "default_first_half_end_minute")]
    pub first_half_end_minute: u8,
    /// Added-time accumulator for second half (seconds).
    #[serde(default)]
    pub stoppage_seconds_second_half: u16,
    /// Whether 2H added time has been finalized at regulation end.
    #[serde(default)]
    pub stoppage_finalized_second_half: bool,
    /// Actual match end minute (regulation + added time).
    #[serde(default = "default_match_end_minute")]
    pub match_end_minute: u8,

    // ========== Ball State ==========
    /// Complete ball state
    pub ball: Ball,

    // ========== Player State ==========
    /// Player positions (22 players, Coord10)
    pub player_positions: Vec<Coord10>,
    /// Player velocities (22 players, Vel10)
    pub player_velocities: Vec<Vel10>,
    /// Player reaction states
    pub player_reaction_states: Vec<PlayerReactionState>,
    /// Player fatigue levels (0.0 = fresh, 1.0 = exhausted)
    pub player_fatigue: Vec<f32>,
    /// Stamina levels (0.0 = empty, 1.0 = full)
    pub stamina: [f32; 22],
    /// Sprint state per player
    pub sprint_state: [bool; 22],
    /// Resting state per player
    pub player_resting: [bool; 22],
    /// Continuous running ticks per player
    pub continuous_running_ticks: [u32; 22],
    /// Player state machine states
    pub player_states: Vec<PlayerState>,
    /// Tackle cooldowns per player
    pub tackle_cooldowns: [u8; 22],
    /// Player speeds
    pub player_speeds: [f32; 22],

    // ========== Score & Result ==========
    /// Home team score
    pub score_home: u8,
    /// Away team score
    pub score_away: u8,
    /// Injured player indices
    pub injured_players: Vec<usize>,
    /// Substitutions made (home, away)
    pub substitutions_made: (u8, u8),

    // ========== Game State ==========
    /// Current game state
    pub game_state: GameState,
    /// Offside counts
    pub offside_count_home: u16,
    pub offside_count_away: u16,
    /// Shot counts this half
    pub shots_this_half_home: u8,
    pub shots_this_half_away: u8,
    /// Possession owner tracking (decision ticks)
    #[serde(default)]
    pub possession_owner_idx: Option<usize>,
    #[serde(default)]
    pub possession_owner_since_tick: u64,

    // ========== Action Queue State ==========
    /// Pending scheduled actions (converted from BinaryHeap)
    pub pending_actions: Vec<ScheduledAction>,
    /// Active FSM actions
    pub active_actions: Vec<ActiveAction>,
    /// Ball state in action queue
    pub action_queue_ball_state: BallState,
    /// Next action ID
    pub next_action_id: u64,

    // ========== RNG State ==========
    /// Original seed used to create the RNG
    pub rng_seed: u64,
    /// Current word position in the RNG stream (for restoration)
    pub rng_word_pos: u128,
}

impl MatchStateSnapshot {
    /// Serialize snapshot to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize snapshot to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize snapshot from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Snapshot of ActionQueue for serialization
///
/// BinaryHeap cannot be directly serialized, so we convert to Vec
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionQueueSnapshot {
    /// Pending actions (sorted by execute_tick)
    pub pending: Vec<ScheduledAction>,
    /// Active FSM actions
    pub active: Vec<ActiveAction>,
    /// Current ball state
    pub ball_state: BallState,
    /// Next action ID
    pub next_action_id: u64,
    /// Current tick
    pub current_tick: u64,
    /// Last shot xG
    pub last_shot_xg: Option<f32>,
    /// Last shooter index
    pub last_shooter_idx: Option<usize>,
    /// Last passer index (for assist tracking)
    pub last_passer_idx: Option<usize>,
    /// Last pass receiver index (for pass success tracking)
    pub last_pass_receiver_idx: Option<usize>,
    /// Last pass type (for cross/through tracking)
    #[serde(default)]
    pub last_pass_type: Option<crate::engine::action_queue::PassType>,
    /// Last header outcome (ephemeral)
    #[serde(default)]
    pub last_header_outcome: Option<crate::engine::action_queue::HeaderOutcome>,
    /// In-flight origin marker (set-piece deliveries, etc.).
    #[serde(default)]
    pub in_flight_origin: Option<crate::engine::action_queue::InFlightOrigin>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_error_display() {
        let err = SnapshotError::PlayerCountMismatch { expected: 22, actual: 20 };
        assert!(err.to_string().contains("22"));
        assert!(err.to_string().contains("20"));
    }

    #[test]
    fn test_snapshot_json_roundtrip() {
        // This test requires a full MatchStateSnapshot which needs many types
        // For now, just test that the types compile correctly
    }
}
