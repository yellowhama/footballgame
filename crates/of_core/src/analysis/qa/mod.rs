//! # QA (Quality Assurance) Module
//!
//! Validation tools for ensuring football simulation realism.
//!
//! - `physics_validator` - Physical constraint validation (speed, acceleration, bounds)
//! - `consistency_checker` - Event-to-statistic consistency validation
//! - `football_likeness` - Combined "football-likeness" score calculation
//! - `advanced_metrics` - High-level QA metrics (line spacing, passing network, PPDA)

pub mod physics_validator;
pub mod consistency_checker;
pub mod football_likeness;
pub mod advanced_metrics;

// Re-export physics validator types (avoid conflict with consistency_checker::quick_validate)
pub use physics_validator::{
    PhysicsAnomaly, EntityType, PhysicsValidationResult,
    validate_physics, is_in_bounds,
    MAX_PLAYER_SPEED_MPS, MAX_PLAYER_ACCEL_MPS2, MAX_BALL_SPEED_MPS,
    PITCH_LENGTH_M, PITCH_WIDTH_M,
};

// Re-export consistency checker types
pub use consistency_checker::{
    StatConsistency, ConsistencyReport, ConsistencyGrade, StatType, EventCounts,
    validate_event_stat_consistency, validate_match_consistency, analyze_mismatches,
    quick_validate as quick_validate_consistency,
};

// Re-export football likeness types
pub use football_likeness::*;

// Re-export advanced metrics types
pub use advanced_metrics::{
    // Core types
    LineRole, LineSpacingConfig, PassNetworkConfig, PpdaConfig,
    LineSpacingSummary, PassNetworkSummary, PpdaSummary,
    QaAdvancedMetrics, TeamMetrics,
    // Baseline types
    AdvancedBaseline, LineSpacingBaseline, PassNetworkBaseline, PpdaBaseline,
    // Scorecard types
    QaScorecard, QaSubscores, QaAlert, AlertLevel, AggregatedMetrics, ScoringConfig,
    // QA Grade types (FIX_2601/1127)
    QaGrade, CiGateResult, get_scorecard_grade, check_ci_gate,
    // Extended config
    AdvancedMetricsConfig,
    // Functions
    compute_line_spacing, compute_pass_network, compute_ppda, compute_advanced_metrics,
    aggregate_runs, score_against_baseline, generate_scorecard,
};
