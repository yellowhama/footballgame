//! # Event Extraction Module
//!
//! Extracts higher-level events from raw position/velocity data.
//!
//! - `carry_extractor` - Ball carry segment detection
//! - `sprint_extractor` - High-intensity sprint detection
//! - `run_extractor` - Off-ball and on-ball run detection

pub mod carry_extractor;
pub mod sprint_extractor;
pub mod run_extractor;

// Re-export main types (avoiding ambiguous reference modules)
pub use carry_extractor::{
    CarryOutcome, CarrySegment, TeamCarryStats,
    extract_carries, calculate_team_carry_stats,
    calculate_progressive_rate, calculate_carry_share,
    MIN_CARRY_DURATION_MS, MIN_CARRY_DISTANCE_M, PROGRESSIVE_CARRY_M,
};
pub use sprint_extractor::{
    SprintEvent, PlayerMovementMetrics, TeamMovementMetrics,
    extract_sprints, calculate_team_movement_metrics, calculate_player_metrics,
    SPRINT_THRESHOLD_MPS, HIGH_INTENSITY_MPS, MIN_SPRINT_DURATION_MS,
};
pub use run_extractor::{
    RunType, RunEvent, TeamRunStats,
    extract_runs, classify_run, calculate_team_run_stats,
    RUN_MIN_SPEED_MPS, MIN_RUN_DURATION_MS, MIN_RUN_DISTANCE_M,
};
