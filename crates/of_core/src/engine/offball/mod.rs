//! # Off-Ball Decision System v1
//!
//! Provides intelligent off-ball movement decisions for players without the ball.
//! Integrates with DPQ v1.2 for efficient scheduling via TTL-based objectives.
//!
//! ## Key Concepts
//!
//! - **OffBallObjective**: Target position + intent + TTL (not actions)
//! - **TTL-based scheduling**: Decisions persist 12 ticks (~3s), not recalculated every tick
//! - **Score6 evaluation**: 6-factor scoring similar to UAE but lighter weight
//! - **Collision resolution**: Prevents multiple players targeting same position
//!
//! ## Usage
//!
//! ```rust,ignore
//! // In tick_based.rs, after on-ball decisions
//! if exp.offball_decisions_enabled {
//!     update_offball_decisions(&mut state, &setup, &mut dpq, tick, &exp);
//! }
//! // positioning_engine then consumes objectives
//! ```

pub mod types;
pub mod scheduler;
pub mod candidates;
pub mod scorer;
pub mod resolver;
pub mod decision;

// Re-exports
pub use types::{
    GamePhase, OffBallCandidate, OffBallConfig, OffBallContext, OffBallIntent, OffBallObjective,
    Score6, ShapeBias, TacticalPreset, Urgency,
};
pub use scheduler::{apply_force_expire_triggers, select_players_for_decision};
pub use candidates::generate_candidates;
pub use scorer::evaluate_candidate;
pub use resolver::{check_collision, resolve_collision};
pub use decision::update_offball_decisions;
