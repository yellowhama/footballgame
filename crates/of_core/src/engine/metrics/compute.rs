//! # Metrics Computation Functions
//!
//! FIX_2601 Phase 5: Functions to compute metrics from MatchResult.
//!
//! ## SSOT Compliance
//!
//! These functions take `&MatchResult` (Layer 1 Product SSOT) as input
//! and compute Layer 2 (QA) metrics without duplicating Product data.

use crate::analysis::qa::advanced_metrics::{
    compute_advanced_metrics, QaAdvancedMetrics,
};
use crate::models::match_result::MatchResult;
use crate::models::match_setup::MatchSetup;

use super::unified::{MatchMetrics, MetricsMetadata};

// ============================================================================
// compute_all_metrics
// ============================================================================

/// Compute all available metrics from a MatchResult.
///
/// This is the main entry point for post-match metric computation.
///
/// ## Arguments
///
/// * `result` - The completed match result (Layer 1 Product SSOT)
/// * `setup` - Optional match setup for role-based analysis
///
/// ## Returns
///
/// A `MatchMetrics` container with:
/// - QA advanced metrics (if position data available)
/// - Metadata about the computation
///
/// ## SSOT Compliance
///
/// This function does NOT copy `result.statistics`. It computes QA metrics
/// from position data and events, referencing but not duplicating Product data.
///
/// ## Example
///
/// ```rust,ignore
/// let metrics = compute_all_metrics(&result, Some(&setup));
///
/// if let Some(advanced) = &metrics.qa_advanced {
///     println!("Home PPDA: {:.1}", advanced.ppda.home.ppda);
/// }
/// ```
pub fn compute_all_metrics(result: &MatchResult, setup: Option<&MatchSetup>) -> MatchMetrics {
    let has_position_data = result.position_data.is_some();
    let has_match_setup = setup.is_some();

    // Create metadata
    let metadata = MetricsMetadata::new(
        result.statistics.total_ticks,
        result.statistics.total_ticks,
    )
    .with_position_data(has_position_data)
    .with_match_setup(has_match_setup);

    let mut metrics = MatchMetrics::with_metadata(metadata);

    // Compute QA metrics if we have the required data
    if let Some(qa) = compute_qa_from_result(result, setup) {
        metrics.set_qa_advanced(qa);
    }

    metrics
}

// ============================================================================
// compute_qa_from_result
// ============================================================================

/// Compute QA advanced metrics from MatchResult.
///
/// ## Requirements
///
/// - `result.position_data` must be Some
/// - `setup` must be provided for accurate role-based line spacing
///
/// ## Returns
///
/// - `Some(QaAdvancedMetrics)` if computation succeeded
/// - `None` if required data is missing
///
/// ## SSOT Compliance
///
/// This function uses:
/// - `result.position_data` (for line spacing, pass receiver inference)
/// - `result.events` (for pass network, PPDA calculation)
/// - `setup` (for player positions → line roles)
///
/// It does NOT copy `result.statistics`.
pub fn compute_qa_from_result(
    result: &MatchResult,
    setup: Option<&MatchSetup>,
) -> Option<QaAdvancedMetrics> {
    // Require position data
    let position_data = result.position_data.as_ref()?;

    // Require setup for accurate role mapping
    let setup = setup?;

    // Compute advanced metrics using the existing analysis function
    let advanced = compute_advanced_metrics(
        position_data,
        &result.events,
        setup,
    );

    Some(advanced)
}

// ============================================================================
// Helper: compute_qa_grade_from_result
// ============================================================================

/// Compute QA grade from a single match result.
///
/// Note: For accurate grade calculation, use `generate_scorecard()` with
/// multiple match results. Single-match grades are less reliable.
///
/// ## Returns
///
/// - `Some(QaGrade)` based on a simple heuristic
/// - `None` if metrics couldn't be computed
pub fn compute_qa_grade_from_result(
    result: &MatchResult,
    setup: Option<&MatchSetup>,
) -> Option<crate::analysis::qa::advanced_metrics::QaGrade> {
    use crate::analysis::qa::advanced_metrics::QaGrade;

    let advanced = compute_qa_from_result(result, setup)?;

    // Simple heuristic for single-match grade
    // Real grading should use batch aggregation (generate_scorecard)
    let score = compute_single_match_score(&advanced);

    Some(QaGrade::from_score(score))
}

/// Compute a simple score for a single match's QA metrics.
///
/// This is a simplified version of the batch scoring system.
/// For production use, prefer `aggregate_runs()` + `score_against_baseline()`.
fn compute_single_match_score(advanced: &QaAdvancedMetrics) -> f32 {
    let score = 100.0f32;
    let mut penalties = 0.0f32;

    // Line spacing check: df_mean should be around 32m (±8m)
    let home_df = advanced.line_spacing.home.df_mean;
    let away_df = advanced.line_spacing.away.df_mean;
    if home_df > 0.0 && away_df > 0.0 {
        let avg_df = (home_df + away_df) / 2.0;
        let df_deviation = (avg_df - 32.0).abs();
        if df_deviation > 8.0 {
            penalties += (df_deviation - 8.0) * 2.0; // 2 points per meter beyond threshold
        }
    }

    // Pass network check: density should be 0.2-0.6
    let home_density = advanced.pass_network.home.density;
    let away_density = advanced.pass_network.away.density;
    if home_density > 0.0 && away_density > 0.0 {
        let avg_density = (home_density + away_density) / 2.0;
        if avg_density < 0.2 {
            penalties += (0.2 - avg_density) * 50.0;
        } else if avg_density > 0.6 {
            penalties += (avg_density - 0.6) * 50.0;
        }
    }

    // PPDA check: should be around 10.5 (±5)
    let home_ppda = advanced.ppda.home.ppda;
    let away_ppda = advanced.ppda.away.ppda;
    if home_ppda < f32::MAX && away_ppda < f32::MAX && home_ppda > 0.0 && away_ppda > 0.0 {
        let avg_ppda = (home_ppda + away_ppda) / 2.0;
        let ppda_deviation = (avg_ppda - 10.5).abs();
        if ppda_deviation > 5.0 {
            penalties += (ppda_deviation - 5.0) * 3.0;
        }
    }

    (score - penalties).clamp(0.0, 100.0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::match_result::{MatchPositionData, Statistics};

    fn create_test_result_without_position_data() -> MatchResult {
        MatchResult {
            schema_version: 1,
            coord_contract_version: crate::engine::coordinate_contract::COORD_CONTRACT_VERSION,
            coord_system: crate::engine::coordinate_contract::COORD_SYSTEM_METERS_V2.to_string(),
            ssot_proof: crate::fix01::SsotProof::default(),
            determinism: Default::default(),
            score_home: 2,
            score_away: 1,
            events: vec![],
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

    fn create_test_result_with_position_data() -> MatchResult {
        let mut result = create_test_result_without_position_data();
        result.position_data = Some(MatchPositionData::new());
        result.statistics.total_ticks = 5000;
        result
    }

    #[test]
    fn test_compute_all_metrics_no_position_data() {
        let result = create_test_result_without_position_data();
        let metrics = compute_all_metrics(&result, None);

        // Should have metadata but no QA
        assert!(!metrics.metadata.has_position_data);
        assert!(!metrics.metadata.has_match_setup);
        assert!(metrics.qa_advanced.is_none());
    }

    #[test]
    fn test_compute_all_metrics_with_position_data_no_setup() {
        let result = create_test_result_with_position_data();
        let metrics = compute_all_metrics(&result, None);

        // Has position data but no setup → can't compute QA
        assert!(metrics.metadata.has_position_data);
        assert!(!metrics.metadata.has_match_setup);
        // QA requires setup for role mapping
        assert!(metrics.qa_advanced.is_none());
    }

    #[test]
    fn test_compute_qa_from_result_returns_none_without_data() {
        let result = create_test_result_without_position_data();
        let qa = compute_qa_from_result(&result, None);

        assert!(qa.is_none());
    }

    #[test]
    fn test_compute_single_match_score_perfect() {
        // Create metrics with ideal values
        let mut advanced = QaAdvancedMetrics::default();

        // Set ideal line spacing (around 32m)
        advanced.line_spacing.home.df_mean = 32.0;
        advanced.line_spacing.home.sample_count = 100;
        advanced.line_spacing.away.df_mean = 32.0;
        advanced.line_spacing.away.sample_count = 100;

        // Set ideal pass network density (0.4)
        advanced.pass_network.home.density = 0.4;
        advanced.pass_network.home.total_passes = 200;
        advanced.pass_network.away.density = 0.4;
        advanced.pass_network.away.total_passes = 180;

        // Set ideal PPDA (10.5)
        advanced.ppda.home.ppda = 10.5;
        advanced.ppda.away.ppda = 10.5;

        let score = compute_single_match_score(&advanced);
        assert!(score > 95.0, "Perfect metrics should score > 95, got {}", score);
    }

    #[test]
    fn test_compute_single_match_score_poor_line_spacing() {
        let mut advanced = QaAdvancedMetrics::default();

        // Set poor line spacing (50m instead of 32m)
        advanced.line_spacing.home.df_mean = 50.0;
        advanced.line_spacing.home.sample_count = 100;
        advanced.line_spacing.away.df_mean = 50.0;
        advanced.line_spacing.away.sample_count = 100;

        // Good pass network
        advanced.pass_network.home.density = 0.4;
        advanced.pass_network.home.total_passes = 200;
        advanced.pass_network.away.density = 0.4;
        advanced.pass_network.away.total_passes = 180;

        // Good PPDA
        advanced.ppda.home.ppda = 10.5;
        advanced.ppda.away.ppda = 10.5;

        let score = compute_single_match_score(&advanced);
        // 50-32 = 18m deviation, beyond 8m threshold by 10m
        // Penalty = 10 * 2 = 20 points
        assert!(score < 85.0, "Poor line spacing should reduce score, got {}", score);
    }
}
