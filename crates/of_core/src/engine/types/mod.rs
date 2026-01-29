//! Engine Types
//!
//! This module contains type definitions used by the match simulation engine.
//! These types are separated from match_sim.rs for better organization.

use crate::models::MatchResult;

// FIX_2512 Phase 1: Coord10 좌표 시스템
// FIX_2601 Phase 4: TeamViewCoord10, DirectionContext 추가
pub mod coord10;
pub use coord10::{Coord10, DirectionContext, TeamViewCoord10, Vel10};

// FIX_2601/0102: Assist Candidate System
pub mod assist;
pub use assist::AssistCandidate;

// ===========================================
// Phase E: Interactive match support types
// ===========================================

/// Minimal representation of a potential pass target for user decisions.
#[derive(Debug, Clone)]
pub struct PassTarget {
    pub id: u32,
    pub success_prob: f32,
    pub is_key_pass: bool,
}

/// Aggregated options for a single user decision moment.
#[derive(Debug, Clone)]
pub struct ActionOptions {
    pub shoot_prob: f32,
    pub dribble_prob: f32,
    pub pass_targets: Vec<PassTarget>,
}

/// Context returned to the frontend when the engine pauses
/// for a potential user decision.
#[derive(Debug, Clone)]
pub struct UserDecisionContext {
    pub player_id: u32,
    pub time_seconds: f32,
    pub position_m: (f32, f32),
    pub options: ActionOptions,
}

/// High-level interactive simulation state used by the Phase E spec.
#[derive(Debug, Clone)]
pub enum SimState {
    /// Simulation should continue without user intervention.
    Running,
    /// Engine paused at an interesting moment; frontend may inspect
    /// `UserDecisionContext` and decide how to resume.
    Paused(UserDecisionContext),
    /// Match finished; contains the final `MatchResult`.
    Finished(MatchResult),
}

/// User-driven action applied when resuming an interactive match.
#[derive(Debug, Clone)]
pub enum UserAction {
    Shoot,
    Dribble,
    PassTo(u32),
}

// ===========================================
// Game state types
// ===========================================

/// Ball zone for game state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BallZone {
    Defensive,
    #[default]
    Midfield,
    Attacking,
}

/// Line battle result (offside trap vs penetration)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineBattleResult {
    /// Offside trap success
    OffsideTrapSuccess,
    /// Line breaking success (with speed bonus)
    LineBroken { advantage: f32 },
    /// Contested situation
    Contested,
}

/// Through ball attempt result
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThroughBallResult {
    /// Through ball success, clear chance
    Success,
    /// Offside foul
    Offside,
    /// Caught in offside trap
    OffsideTrap,
    /// Intercepted
    Intercepted,
    /// Bad pass
    BadPass,
    /// Bad timing
    BadTiming,
}

/// Game state
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    pub minute: u8,
    pub score_diff: i8,
    pub is_home_possession: bool,
    pub ball_zone: BallZone,
}

// ===========================================
// A13: Skill system - Reaction state
// ===========================================

/// Defender's psychological state (used for feint attacks)
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReactionState {
    /// Normal state
    Normal,
    /// Off balance (50% effectiveness, 0.5s)
    OffBalance,
    /// Completely frozen (stumbled, 1s)
    Frozen,
}

/// Player reaction state info
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerReactionState {
    pub state: ReactionState,
    pub remaining_ticks: u32, // 60 ticks = 1 second
}

impl Default for PlayerReactionState {
    fn default() -> Self {
        Self { state: ReactionState::Normal, remaining_ticks: 0 }
    }
}
