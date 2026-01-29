//! Statistical Anchor Calibration System
//!
//! FIX_2601/0112: Real match data-based calibration for realistic match simulation.
//!
//! This module provides:
//! - Zone definitions (SSOT for field zones)
//! - Anchor tables (target statistics from real matches)
//! - Calibrator (adjusts engine parameters to match targets)
//! - StatSnapshot (collects per-match statistics)
//! - PassClassifier (categorizes passes by type)
//! - Scenarios (GRF-style micro-tests for bug reproduction)

pub mod zone;
pub mod anchor_table;
pub mod calibrator;
pub mod stat_snapshot;
pub mod pass_classifier;
pub mod scenarios;
pub mod scenario_runner;
pub mod symmetry_runner;

pub use zone::{
    ZoneId, ZoneSchema, pos_to_zone, pos_to_zone_for_team,
    // Layer 2: Positional Play (20-zone)
    Lane, Quarter, PosPlayZoneId,
    pos_to_posplay_zone, pos_to_posplay_zone_for_team, pos_to_posplay_zone_meters,
    downscale_to_anchor, upscale_from_anchor,
    PosPlayZoneDistribution, empty_posplay_distribution,
};
pub use anchor_table::{AnchorTable, TeamStats};
pub use calibrator::{
    Calibrator, CalibratorParams, CalibratorConfig, ActionAttemptBias,
    // FIX_2601/0113: Tactical gates
    TacticalGates, TacticalGateResult,
};
pub use stat_snapshot::{
    MatchStatSnapshot,
    // FIX_2601/0113: Tactical metrics
    HalfSpaceMetrics, LaneOccupancy, ZoneProgression,
};
pub use pass_classifier::{PassType, classify_pass, classify_pass_detailed, PassClassification, NormPos, ClassifierThresholds};
pub use scenarios::{TestScenario, ScenarioSetup, ScenarioResult, SuccessCondition, SymmetryVariant};
pub use scenario_runner::ScenarioRunner;
pub use symmetry_runner::{SymmetryMetaRunner, SymmetryReport, SymmetryViolation, SymmetryStats, ViolationType};
