//! # Unified MatchMetrics Container
//!
//! FIX_2601 Phase 5: SSOT-compliant unified metrics structure.
//!
//! ## Design
//!
//! `MatchMetrics` aggregates metrics from multiple SSOT layers without
//! duplicating the authoritative Product statistics in `MatchResult.statistics`.

use serde::{Deserialize, Serialize};

use crate::analysis::qa::advanced_metrics::{QaAdvancedMetrics, QaGrade, QaScorecard};
use crate::engine::match_sim::quality_metrics::QualityReport;
use crate::engine::reward::EpisodeMetrics;
use crate::models::match_result::{MatchResult, Statistics};

// ============================================================================
// MetricsMetadata
// ============================================================================

/// Metadata about when and how metrics were computed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsMetadata {
    /// Tick at which metrics were computed (usually end of match)
    pub computed_at_tick: u32,

    /// Total match duration in ticks
    pub match_duration_ticks: u32,

    /// Whether position data was available for QA metrics
    pub has_position_data: bool,

    /// Whether match setup was available for role-based analysis
    pub has_match_setup: bool,

    /// Schema version for forward compatibility
    pub schema_version: u8,
}

impl MetricsMetadata {
    /// Current schema version
    pub const CURRENT_VERSION: u8 = 1;

    /// Create metadata with current schema version
    pub fn new(computed_at_tick: u32, match_duration_ticks: u32) -> Self {
        Self {
            computed_at_tick,
            match_duration_ticks,
            has_position_data: false,
            has_match_setup: false,
            schema_version: Self::CURRENT_VERSION,
        }
    }

    /// Set position data availability
    pub fn with_position_data(mut self, has_data: bool) -> Self {
        self.has_position_data = has_data;
        self
    }

    /// Set match setup availability
    pub fn with_match_setup(mut self, has_setup: bool) -> Self {
        self.has_match_setup = has_setup;
        self
    }
}

// ============================================================================
// ProductStatsSummary
// ============================================================================

/// Read-only summary of Product SSOT statistics.
///
/// This is computed on-demand from `&MatchResult.statistics`, NOT stored.
/// It provides a convenience view for consumers who need a quick summary
/// without accessing the full Statistics struct.
///
/// **IMPORTANT**: This is NOT a copy of Product data. It's a derived view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductStatsSummary {
    /// Total goals (home + away)
    pub total_goals: u8,

    /// Total shots (home + away)
    pub total_shots: u16,

    /// Home team possession percentage (0.0 - 1.0)
    pub possession_home: f32,

    /// Total passes (home + away)
    pub total_passes: u16,

    /// Average pass accuracy ((home + away) / 2)
    pub pass_accuracy: f32,
}

impl ProductStatsSummary {
    /// Create a summary from MatchResult statistics.
    ///
    /// This is the ONLY way to create a ProductStatsSummary - from the
    /// authoritative Product SSOT.
    pub fn from_statistics(stats: &Statistics) -> Self {
        Self {
            total_goals: 0, // Goals come from MatchResult.score_home/away, not Statistics
            total_shots: stats.shots_home + stats.shots_away,
            possession_home: stats.possession_home,
            total_passes: stats.passes_home + stats.passes_away,
            pass_accuracy: (stats.pass_accuracy_home + stats.pass_accuracy_away) / 2.0,
        }
    }

    /// Create a summary from MatchResult (includes goals from score)
    pub fn from_match_result(result: &MatchResult) -> Self {
        let stats = &result.statistics;
        Self {
            total_goals: result.score_home + result.score_away,
            total_shots: stats.shots_home + stats.shots_away,
            possession_home: stats.possession_home,
            total_passes: stats.passes_home + stats.passes_away,
            pass_accuracy: (stats.pass_accuracy_home + stats.pass_accuracy_away) / 2.0,
        }
    }
}

// ============================================================================
// MatchMetrics
// ============================================================================

/// Unified metrics container respecting SSOT layer hierarchy.
///
/// ## Layer Structure
///
/// - **Layer 1 (Product SSOT)**: `MatchResult.statistics` - NOT stored here.
///   Use `summarize_product_stats()` to get an on-demand summary.
///
/// - **Layer 2 (QA)**: `qa_advanced` and `qa_grade` - Computed from
///   MatchResult position data and events.
///
/// - **Layer 3 (RL)**: `rl_episode` - Training metrics, optional.
///
/// - **Engine**: `quality_report` - Simulation quality verification.
///
/// ## SSOT Compliance
///
/// This struct intentionally does NOT contain:
/// - `Statistics` (owned by MatchResult)
/// - Any field that duplicates Product SSOT data
///
/// All Product data access should go through the original `MatchResult`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchMetrics {
    /// Computation metadata
    pub metadata: MetricsMetadata,

    /// Layer 2: QA advanced metrics (line spacing, pass network, PPDA)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_advanced: Option<QaAdvancedMetrics>,

    /// Layer 2: QA scorecard (batch analysis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_scorecard: Option<QaScorecard>,

    /// Layer 2: QA grade (derived from scorecard)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_grade: Option<QaGrade>,

    /// Engine: Quality report (simulation health)
    /// Note: QualityReport doesn't implement Serialize, so we skip it
    #[serde(skip)]
    pub quality_report: Option<QualityReport>,

    /// Layer 3: RL episode metrics (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rl_episode: Option<EpisodeMetrics>,
}

impl MatchMetrics {
    /// Create an empty metrics container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create metrics container with metadata.
    pub fn with_metadata(metadata: MetricsMetadata) -> Self {
        Self {
            metadata,
            ..Default::default()
        }
    }

    /// Get overall QA grade if available.
    ///
    /// Returns the grade from:
    /// 1. `qa_grade` if directly set
    /// 2. Computed from `qa_scorecard` if available
    /// 3. None otherwise
    pub fn overall_grade(&self) -> Option<QaGrade> {
        self.qa_grade.or_else(|| {
            self.qa_scorecard.as_ref().map(|sc| QaGrade::from_score(sc.overall_score))
        })
    }

    /// Check if QA gate is passed (MARGINAL or better).
    ///
    /// Returns true if:
    /// - No QA metrics computed (assume pass)
    /// - Grade is MARGINAL, PASS, or GREAT
    pub fn passes_qa_gate(&self) -> bool {
        match self.overall_grade() {
            Some(grade) => grade.passes_ci_gate(),
            None => true, // No QA data = assume pass
        }
    }

    /// Set QA advanced metrics (Layer 2).
    pub fn set_qa_advanced(&mut self, advanced: QaAdvancedMetrics) {
        self.qa_advanced = Some(advanced);
    }

    /// Set QA scorecard (Layer 2).
    pub fn set_qa_scorecard(&mut self, scorecard: QaScorecard) {
        self.qa_grade = Some(QaGrade::from_score(scorecard.overall_score));
        self.qa_scorecard = Some(scorecard);
    }

    /// Set QA grade directly (Layer 2).
    pub fn set_qa_grade(&mut self, grade: QaGrade) {
        self.qa_grade = Some(grade);
    }

    /// Set quality report (Engine layer).
    pub fn set_quality_report(&mut self, report: QualityReport) {
        self.quality_report = Some(report);
    }

    /// Set RL episode metrics (Layer 3).
    ///
    /// This is separate from QA/Product layers and can be set
    /// independently during RL training.
    pub fn set_rl_episode(&mut self, episode: EpisodeMetrics) {
        self.rl_episode = Some(episode);
    }

    /// Generate Product stats summary on-demand.
    ///
    /// **IMPORTANT**: This does NOT store the summary. It computes it
    /// from the authoritative `MatchResult` each time.
    pub fn summarize_product_stats(result: &MatchResult) -> ProductStatsSummary {
        ProductStatsSummary::from_match_result(result)
    }

    /// Check if RL metrics are available.
    pub fn has_rl_metrics(&self) -> bool {
        self.rl_episode.is_some()
    }

    /// Check if QA metrics are available.
    pub fn has_qa_metrics(&self) -> bool {
        self.qa_advanced.is_some() || self.qa_scorecard.is_some() || self.qa_grade.is_some()
    }

    /// Check if quality report is available.
    pub fn has_quality_report(&self) -> bool {
        self.quality_report.is_some()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_metadata_creation() {
        let metadata = MetricsMetadata::new(1000, 5000)
            .with_position_data(true)
            .with_match_setup(true);

        assert_eq!(metadata.computed_at_tick, 1000);
        assert_eq!(metadata.match_duration_ticks, 5000);
        assert!(metadata.has_position_data);
        assert!(metadata.has_match_setup);
        assert_eq!(metadata.schema_version, MetricsMetadata::CURRENT_VERSION);
    }

    #[test]
    fn test_match_metrics_new() {
        let metrics = MatchMetrics::new();

        assert!(metrics.qa_advanced.is_none());
        assert!(metrics.qa_grade.is_none());
        assert!(metrics.rl_episode.is_none());
        assert!(metrics.quality_report.is_none());
    }

    #[test]
    fn test_match_metrics_passes_qa_gate_no_data() {
        let metrics = MatchMetrics::new();

        // No QA data = assume pass
        assert!(metrics.passes_qa_gate());
    }

    #[test]
    fn test_match_metrics_overall_grade_from_scorecard() {
        use crate::analysis::qa::advanced_metrics::QaSubscores;

        let mut metrics = MatchMetrics::new();

        // Set scorecard with score >= 70 (PASS threshold)
        let scorecard = QaScorecard {
            runs: 10,
            league_baseline: "EPL_2023_24".to_string(),
            overall_score: 75.0,
            subscores: QaSubscores::default(),
            alerts: vec![],
        };
        metrics.set_qa_scorecard(scorecard);

        let grade = metrics.overall_grade();
        assert!(grade.is_some());
        assert_eq!(grade.unwrap(), QaGrade::Pass);
        assert!(metrics.passes_qa_gate());
    }

    #[test]
    fn test_match_metrics_rl_layer_optional() {
        let mut metrics = MatchMetrics::new();

        // RL not required for QA
        assert!(!metrics.has_rl_metrics());
        assert!(metrics.passes_qa_gate());

        // Add RL metrics later
        let episode = EpisodeMetrics::new();
        metrics.set_rl_episode(episode);

        assert!(metrics.has_rl_metrics());
        // Still passes QA (RL doesn't affect QA gate)
        assert!(metrics.passes_qa_gate());
    }

    #[test]
    fn test_match_metrics_has_methods() {
        let mut metrics = MatchMetrics::new();

        assert!(!metrics.has_qa_metrics());
        assert!(!metrics.has_rl_metrics());
        assert!(!metrics.has_quality_report());

        metrics.set_qa_advanced(QaAdvancedMetrics::default());
        assert!(metrics.has_qa_metrics());

        metrics.set_rl_episode(EpisodeMetrics::new());
        assert!(metrics.has_rl_metrics());
    }
}
