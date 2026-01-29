//! # Football Likeness Score
//!
//! Composite QA score combining all validation layers.
//!
//! ## Scoring Breakdown
//! - **Integrity (40 points)**: Physics validation
//! - **Motion (25 points)**: Movement patterns
//! - **Shape (20 points)**: Team formation metrics
//! - **Calibration (15 points)**: Statistical accuracy
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: REALTIME_SYSTEMS_ANALYSIS.md

use super::physics_validator::{PhysicsValidationResult, PhysicsAnomaly};
use super::consistency_checker::ConsistencyReport;
use crate::analysis::metrics::shape::{TeamShapeMetrics, ShapeFlag};
use crate::analysis::metrics::movement::OccupancyEntropy;
use crate::analysis::events::{TeamCarryStats, TeamMovementMetrics, TeamRunStats};

/// QA grade based on total score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// Score >= 85: Production ready
    Pass,
    /// Score 70-84: Minor issues, usable
    Warn,
    /// Score < 70: Significant issues
    Fail,
}

impl Grade {
    /// Determine grade from total score.
    pub fn from_score(score: f32) -> Self {
        if score >= 85.0 {
            Grade::Pass
        } else if score >= 70.0 {
            Grade::Warn
        } else {
            Grade::Fail
        }
    }

    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Grade::Pass => "PASS",
            Grade::Warn => "WARN",
            Grade::Fail => "FAIL",
        }
    }
}

/// Component scores for football likeness.
#[derive(Debug, Clone, Default)]
pub struct ComponentScores {
    /// Physics integrity (0-40)
    pub integrity: f32,
    /// Movement patterns (0-25)
    pub motion: f32,
    /// Team shape metrics (0-20)
    pub shape: f32,
    /// Statistical calibration (0-15)
    pub calibration: f32,
}

impl ComponentScores {
    /// Total score (0-100).
    pub fn total(&self) -> f32 {
        self.integrity + self.motion + self.shape + self.calibration
    }

    /// All scores as percentages of their maximums.
    pub fn as_percentages(&self) -> (f32, f32, f32, f32) {
        (
            self.integrity / 40.0 * 100.0,
            self.motion / 25.0 * 100.0,
            self.shape / 20.0 * 100.0,
            self.calibration / 15.0 * 100.0,
        )
    }
}

/// Full football likeness score with breakdown.
#[derive(Debug, Clone)]
pub struct FootballLikenessScore {
    /// Total score (0-100)
    pub total: f32,
    /// Component breakdown
    pub components: ComponentScores,
    /// Grade based on total
    pub grade: Grade,
    /// Detailed findings
    pub findings: Vec<Finding>,
}

impl Default for FootballLikenessScore {
    fn default() -> Self {
        Self {
            total: 0.0,
            components: ComponentScores::default(),
            grade: Grade::Fail,
            findings: Vec::new(),
        }
    }
}

impl FootballLikenessScore {
    /// Create a new score from components.
    pub fn new(components: ComponentScores) -> Self {
        let total = components.total();
        Self {
            total,
            components,
            grade: Grade::from_score(total),
            findings: Vec::new(),
        }
    }

    /// Whether the score passes minimum threshold.
    pub fn passes(&self) -> bool {
        self.grade != Grade::Fail
    }

    /// Add a finding to the score.
    pub fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Get findings by category.
    pub fn findings_by_category(&self, category: FindingCategory) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.category == category).collect()
    }

    /// Get the most severe findings.
    pub fn critical_findings(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.severity >= 0.8).collect()
    }

    /// Format as summary string.
    pub fn summary(&self) -> String {
        format!(
            "Football Likeness: {:.1}/100 [{}] - I:{:.0} M:{:.0} S:{:.0} C:{:.0}",
            self.total,
            self.grade.as_str(),
            self.components.integrity,
            self.components.motion,
            self.components.shape,
            self.components.calibration,
        )
    }
}

/// Individual finding from QA validation.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Category of the finding
    pub category: FindingCategory,
    /// Severity (0.0 = info, 1.0 = critical)
    pub severity: f32,
    /// Human-readable description
    pub description: String,
    /// Suggested fix if applicable
    pub suggestion: Option<String>,
}

impl Finding {
    /// Create a new finding.
    pub fn new(category: FindingCategory, severity: f32, description: impl Into<String>) -> Self {
        Self {
            category,
            severity: severity.clamp(0.0, 1.0),
            description: description.into(),
            suggestion: None,
        }
    }

    /// Add a suggestion to the finding.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Check if this is a critical finding.
    pub fn is_critical(&self) -> bool {
        self.severity >= 0.8
    }
}

/// Category of QA finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCategory {
    Physics,
    Movement,
    Shape,
    Calibration,
    Consistency,
}

/// Configuration for score calculation.
#[derive(Debug, Clone)]
pub struct LikenessConfig {
    /// Weight for integrity component (default: 40)
    pub integrity_weight: f32,
    /// Weight for motion component (default: 25)
    pub motion_weight: f32,
    /// Weight for shape component (default: 20)
    pub shape_weight: f32,
    /// Weight for calibration component (default: 15)
    pub calibration_weight: f32,

    // Thresholds for motion scoring
    /// Minimum sprint ratio for full motion score
    pub min_sprint_ratio: f32,
    /// Maximum sprint ratio (too much is unrealistic)
    pub max_sprint_ratio: f32,
    /// Minimum runs per 90 minutes
    pub min_runs_per_90: f32,
    /// Maximum runs per 90 minutes
    pub max_runs_per_90: f32,

    // Thresholds for shape scoring
    /// Minimum team width in meters
    pub min_team_width_m: f32,
    /// Maximum team width in meters
    pub max_team_width_m: f32,
    /// Minimum team depth in meters
    pub min_team_depth_m: f32,
    /// Maximum team depth in meters
    pub max_team_depth_m: f32,
}

impl Default for LikenessConfig {
    fn default() -> Self {
        Self {
            integrity_weight: 40.0,
            motion_weight: 25.0,
            shape_weight: 20.0,
            calibration_weight: 15.0,

            // Motion thresholds (based on real football data)
            min_sprint_ratio: 0.03,   // 3% minimum
            max_sprint_ratio: 0.15,   // 15% maximum
            min_runs_per_90: 30.0,
            max_runs_per_90: 120.0,

            // Shape thresholds
            min_team_width_m: 35.0,
            max_team_width_m: 60.0,
            min_team_depth_m: 25.0,
            max_team_depth_m: 50.0,
        }
    }
}

// ============================================================================
// Movement Metrics Input Structure
// ============================================================================

/// Combined movement metrics for likeness scoring.
#[derive(Debug, Clone, Default)]
pub struct MovementMetricsInput {
    /// Team movement metrics (sprints)
    pub movement: Option<TeamMovementMetrics>,
    /// Team carry stats (dribbles)
    pub carries: Option<TeamCarryStats>,
    /// Team run stats (off-ball runs)
    pub runs: Option<TeamRunStats>,
    /// Occupancy entropy
    pub entropy: Option<OccupancyEntropy>,
}

impl MovementMetricsInput {
    /// Create from individual metrics.
    pub fn new(
        movement: TeamMovementMetrics,
        carries: TeamCarryStats,
        runs: TeamRunStats,
    ) -> Self {
        Self {
            movement: Some(movement),
            carries: Some(carries),
            runs: Some(runs),
            entropy: None,
        }
    }

    /// Add entropy metrics.
    pub fn with_entropy(mut self, entropy: OccupancyEntropy) -> Self {
        self.entropy = Some(entropy);
        self
    }
}

// ============================================================================
// Main Score Calculation
// ============================================================================

/// Calculate football likeness score from all validation inputs.
pub fn calculate_likeness_score(
    physics_result: &PhysicsValidationResult,
    consistency_report: &ConsistencyReport,
    shape_metrics: Option<&TeamShapeMetrics>,
    movement_input: Option<&MovementMetricsInput>,
    config: &LikenessConfig,
) -> FootballLikenessScore {
    let mut findings = Vec::new();

    // ========================================================================
    // 1. INTEGRITY SCORE (40 points) - Physics validation
    // ========================================================================
    let integrity_raw = physics_result.integrity_score / 100.0;
    let integrity = integrity_raw * config.integrity_weight;

    // Add findings for physics issues
    if integrity_raw < 0.9 {
        findings.push(
            Finding::new(
                FindingCategory::Physics,
                1.0 - integrity_raw,
                format!("Physics integrity: {:.1}%", physics_result.integrity_score),
            )
            .with_suggestion("Review speed/acceleration limits"),
        );
    }

    if !physics_result.anomalies.is_empty() {
        findings.push(Finding::new(
            FindingCategory::Physics,
            (physics_result.anomalies.len() as f32 / 100.0).min(0.8),
            format!("{} physics anomalies detected", physics_result.anomalies.len()),
        ));
    }

    // ========================================================================
    // 2. MOTION SCORE (25 points) - Movement patterns
    // ========================================================================
    let motion = calculate_motion_score(movement_input, config, &mut findings);

    // ========================================================================
    // 3. SHAPE SCORE (20 points) - Team formation
    // ========================================================================
    let shape = calculate_shape_score(shape_metrics, config, &mut findings);

    // ========================================================================
    // 4. CALIBRATION SCORE (15 points) - Statistical consistency
    // ========================================================================
    let calibration_raw = consistency_report.consistency_score / 100.0;
    let calibration = calibration_raw * config.calibration_weight;

    // Add findings for consistency issues
    if !consistency_report.all_critical_match() {
        findings.push(
            Finding::new(
                FindingCategory::Consistency,
                0.9, // Critical
                format!(
                    "{} critical stat mismatches",
                    consistency_report.critical_mismatches.len()
                ),
            )
            .with_suggestion("Check event-to-stat recording logic"),
        );
    }

    if !consistency_report.mismatches.is_empty() {
        findings.push(Finding::new(
            FindingCategory::Calibration,
            0.5,
            format!(
                "{} event-stat mismatches (score: {:.1}%)",
                consistency_report.mismatches.len(),
                consistency_report.consistency_score
            ),
        ));
    }

    // ========================================================================
    // Build final score
    // ========================================================================
    let components = ComponentScores {
        integrity,
        motion,
        shape,
        calibration,
    };

    let mut score = FootballLikenessScore::new(components);
    score.findings = findings;
    score
}

/// Calculate motion sub-score.
fn calculate_motion_score(
    input: Option<&MovementMetricsInput>,
    config: &LikenessConfig,
    findings: &mut Vec<Finding>,
) -> f32 {
    let Some(metrics) = input else {
        // No metrics provided, give default score
        return config.motion_weight * 0.8;
    };

    let mut sub_scores: Vec<f32> = Vec::new();

    // Sprint ratio scoring
    if let Some(movement) = &metrics.movement {
        let sprint_ratio = movement.sprint_ratio;
        let sprint_score = if sprint_ratio < config.min_sprint_ratio {
            // Too few sprints
            findings.push(Finding::new(
                FindingCategory::Movement,
                0.5,
                format!("Low sprint ratio: {:.1}%", sprint_ratio * 100.0),
            ));
            (sprint_ratio / config.min_sprint_ratio).min(1.0)
        } else if sprint_ratio > config.max_sprint_ratio {
            // Too many sprints
            findings.push(Finding::new(
                FindingCategory::Movement,
                0.5,
                format!("High sprint ratio: {:.1}%", sprint_ratio * 100.0),
            ));
            1.0 - ((sprint_ratio - config.max_sprint_ratio) / 0.1).min(1.0) * 0.3
        } else {
            1.0
        };
        sub_scores.push(sprint_score);
    }

    // Carry stats scoring
    if let Some(carries) = &metrics.carries {
        let progressive_rate = if carries.total_carries > 0 {
            carries.progressive_carries as f32 / carries.total_carries as f32
        } else {
            0.0
        };
        let carry_score = if progressive_rate >= 0.08 && progressive_rate <= 0.25 {
            1.0 // Good range
        } else if progressive_rate < 0.05 {
            findings.push(Finding::new(
                FindingCategory::Movement,
                0.4,
                format!("Low progressive carry rate: {:.1}%", progressive_rate * 100.0),
            ));
            0.7
        } else {
            0.85
        };
        sub_scores.push(carry_score);
    }

    // Run stats scoring
    if let Some(runs) = &metrics.runs {
        let run_count = runs.total_runs as f32;
        // Assuming 90-minute match
        let runs_per_90 = run_count; // Already per-match

        let run_score = if runs_per_90 >= config.min_runs_per_90 && runs_per_90 <= config.max_runs_per_90 {
            1.0
        } else if runs_per_90 < config.min_runs_per_90 {
            findings.push(Finding::new(
                FindingCategory::Movement,
                0.4,
                format!("Low off-ball run count: {}", run_count as u32),
            ));
            (runs_per_90 / config.min_runs_per_90).min(1.0)
        } else {
            findings.push(Finding::new(
                FindingCategory::Movement,
                0.3,
                format!("High off-ball run count: {}", run_count as u32),
            ));
            0.85
        };
        sub_scores.push(run_score);
    }

    // Entropy scoring
    if let Some(entropy) = &metrics.entropy {
        let entropy_score = if entropy.team_avg_entropy > 0.4 {
            1.0 // Good distribution
        } else {
            findings.push(Finding::new(
                FindingCategory::Movement,
                0.3,
                format!("Low movement entropy: {:.2}", entropy.team_avg_entropy),
            ));
            0.7 + entropy.team_avg_entropy * 0.5
        };
        sub_scores.push(entropy_score);
    }

    // Average sub-scores
    let avg_score = if sub_scores.is_empty() {
        0.85
    } else {
        sub_scores.iter().sum::<f32>() / sub_scores.len() as f32
    };

    avg_score * config.motion_weight
}

/// Calculate shape sub-score.
fn calculate_shape_score(
    metrics: Option<&TeamShapeMetrics>,
    config: &LikenessConfig,
    findings: &mut Vec<Finding>,
) -> f32 {
    let Some(shape) = metrics else {
        return config.shape_weight * 0.85;
    };

    let mut deductions: f32 = 0.0;

    // Width check
    if shape.avg_width < config.min_team_width_m {
        findings.push(Finding::new(
            FindingCategory::Shape,
            0.5,
            format!("Team too narrow: {:.1}m (min: {})", shape.avg_width, config.min_team_width_m),
        ));
        deductions += 0.15;
    } else if shape.avg_width > config.max_team_width_m {
        findings.push(Finding::new(
            FindingCategory::Shape,
            0.4,
            format!("Team too wide: {:.1}m (max: {})", shape.avg_width, config.max_team_width_m),
        ));
        deductions += 0.1;
    }

    // Depth check
    if shape.avg_depth < config.min_team_depth_m {
        findings.push(Finding::new(
            FindingCategory::Shape,
            0.4,
            format!("Team too compact: {:.1}m depth", shape.avg_depth),
        ));
        deductions += 0.1;
    } else if shape.avg_depth > config.max_team_depth_m {
        findings.push(Finding::new(
            FindingCategory::Shape,
            0.5,
            format!("Team too stretched: {:.1}m depth", shape.avg_depth),
        ));
        deductions += 0.15;
    }

    // Check shape flags
    let flags = crate::analysis::metrics::shape::check_shape_flags(shape);
    for flag in flags {
        match flag {
            ShapeFlag::NarrowTeamWidth { value, .. } => {
                if value < 30.0 {
                    deductions += 0.1;
                }
            }
            ShapeFlag::ExcessiveTeamDepth { value, .. } => {
                if value > 55.0 {
                    deductions += 0.1;
                }
            }
            ShapeFlag::UnbalancedLineSpacing { def_mid, mid_att } => {
                findings.push(Finding::new(
                    FindingCategory::Shape,
                    0.3,
                    format!("Unbalanced line spacing: D-M={:.1}m, M-A={:.1}m", def_mid, mid_att),
                ));
                deductions += 0.05;
            }
            _ => {}
        }
    }

    (1.0 - deductions.min(0.5)) * config.shape_weight
}

/// Quick pass/fail check without full scoring.
pub fn quick_validation(
    physics_result: &PhysicsValidationResult,
    consistency_report: &ConsistencyReport,
) -> bool {
    physics_result.passes(60.0) && consistency_report.all_critical_match()
}

/// Calculate a simplified score from just physics and consistency.
pub fn quick_likeness_score(
    physics_result: &PhysicsValidationResult,
    consistency_report: &ConsistencyReport,
) -> FootballLikenessScore {
    calculate_likeness_score(
        physics_result,
        consistency_report,
        None,
        None,
        &LikenessConfig::default(),
    )
}

// ============================================================================
// FIX_2601/NEW_FUNC: MatchStatSnapshot Bridge
// ============================================================================

use crate::analysis::metrics::gini::GiniMetrics;

/// Calculate football likeness score from MatchStatSnapshot data.
///
/// This is a simplified calculation that uses:
/// - Gini metrics for calibration component (15 points)
/// - Snapshot touch data for basic shape estimation (20 points)
/// - Default values for physics (40 points, assumed passed)
/// - Default values for motion (25 points, assumed average)
///
/// For full-featured validation, use `calculate_likeness_score()` with
/// actual physics and consistency data from simulation.
pub fn calculate_snapshot_likeness(
    gini: &GiniMetrics,
    halfspace_rate: f32,
    lane_entropy: f32,
    lane_balance: f32,
) -> FootballLikenessScore {
    let mut findings = Vec::new();

    // ========================================================================
    // 1. INTEGRITY SCORE (40 points) - Assumed passed for snapshot data
    // ========================================================================
    // Snapshot data is post-simulation, so physics is assumed valid
    let integrity_score = 40.0;

    // ========================================================================
    // 2. MOTION SCORE (25 points) - Estimated from lane metrics
    // ========================================================================
    // Use lane entropy and balance as proxy for movement quality
    let entropy_factor = lane_entropy.clamp(0.0, 1.0);
    let balance_factor = lane_balance.clamp(0.0, 1.0);
    let motion_score = 25.0 * (entropy_factor * 0.6 + balance_factor * 0.4);

    // ========================================================================
    // 3. SHAPE SCORE (20 points) - From halfspace usage and balance
    // ========================================================================
    let mut shape_deductions: f32 = 0.0;

    // Half-space usage (should be at least 15%)
    if halfspace_rate < 0.15 {
        shape_deductions += 0.2;
        findings.push(Finding::new(
            FindingCategory::Shape,
            0.5,
            &format!("Low half-space usage: {:.1}%", halfspace_rate * 100.0),
        ));
    }

    // Lane balance check
    if lane_balance < 0.5 {
        shape_deductions += 0.15;
        findings.push(Finding::new(
            FindingCategory::Shape,
            0.4,
            &format!("Unbalanced lane usage: {:.2}", lane_balance),
        ));
    }

    let shape_score = 20.0 * (1.0 - shape_deductions.min(0.5));

    // ========================================================================
    // 4. CALIBRATION SCORE (15 points) - From Gini metrics
    // ========================================================================
    let mut calib_deductions: f32 = 0.0;

    // Touch Gini check (threshold: 0.40)
    if gini.touch_gini > 0.40 {
        let excess = ((gini.touch_gini - 0.40) / 0.60) as f32; // 0-1 scale
        calib_deductions += excess * 0.3;
        if gini.touch_gini > 0.50 {
            findings.push(Finding::new(
                FindingCategory::Calibration,
                0.6,
                &format!("High touch concentration (Gini: {:.2})", gini.touch_gini),
            ).with_suggestion("Reduce hub player dependency"));
        }
    }

    // Pass receive Gini check (threshold: 0.42)
    if gini.pass_recv_gini > 0.42 {
        let excess = ((gini.pass_recv_gini - 0.42) / 0.58) as f32;
        calib_deductions += excess * 0.3;
        if gini.pass_recv_gini > 0.55 {
            findings.push(Finding::new(
                FindingCategory::Calibration,
                0.6,
                &format!("High pass receive concentration (Gini: {:.2})", gini.pass_recv_gini),
            ).with_suggestion("Distribute passes more evenly"));
        }
    }

    // Pass sent Gini check (threshold: 0.38)
    if gini.pass_sent_gini > 0.38 {
        let excess = ((gini.pass_sent_gini - 0.38) / 0.62) as f32;
        calib_deductions += excess * 0.2;
    }

    let calibration_score = 15.0 * (1.0 - calib_deductions.min(0.6));

    // ========================================================================
    // ASSEMBLE FINAL SCORE
    // ========================================================================
    let components = ComponentScores {
        integrity: integrity_score,
        motion: motion_score,
        shape: shape_score,
        calibration: calibration_score,
    };

    let mut score = FootballLikenessScore::new(components);
    score.findings = findings;
    score
}

/// Convenience struct for snapshot likeness calculation inputs.
#[derive(Debug, Clone, Default)]
pub struct SnapshotLikenessInput {
    pub gini: GiniMetrics,
    pub halfspace_rate: f32,
    pub lane_entropy: f32,
    pub lane_balance: f32,
}

impl SnapshotLikenessInput {
    /// Calculate likeness score from this input.
    pub fn calculate(&self) -> FootballLikenessScore {
        calculate_snapshot_likeness(
            &self.gini,
            self.halfspace_rate,
            self.lane_entropy,
            self.lane_balance,
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_from_score() {
        assert_eq!(Grade::from_score(90.0), Grade::Pass);
        assert_eq!(Grade::from_score(85.0), Grade::Pass);
        assert_eq!(Grade::from_score(75.0), Grade::Warn);
        assert_eq!(Grade::from_score(70.0), Grade::Warn);
        assert_eq!(Grade::from_score(60.0), Grade::Fail);
    }

    #[test]
    fn test_component_scores() {
        let components = ComponentScores {
            integrity: 36.0,   // 90% of 40
            motion: 22.5,      // 90% of 25
            shape: 18.0,       // 90% of 20
            calibration: 13.5, // 90% of 15
        };

        assert!((components.total() - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_football_likeness_score() {
        let components = ComponentScores {
            integrity: 35.0,
            motion: 22.0,
            shape: 17.0,
            calibration: 12.0,
        };

        let score = FootballLikenessScore::new(components);
        assert_eq!(score.total, 86.0);
        assert_eq!(score.grade, Grade::Pass);
        assert!(score.passes());
    }

    #[test]
    fn test_finding_creation() {
        let finding = Finding::new(FindingCategory::Physics, 0.8, "Test finding")
            .with_suggestion("Fix it");

        assert_eq!(finding.category, FindingCategory::Physics);
        assert!(finding.is_critical());
        assert_eq!(finding.suggestion, Some("Fix it".to_string()));
    }

    #[test]
    fn test_finding_severity_clamp() {
        let low = Finding::new(FindingCategory::Physics, -0.5, "Low");
        assert_eq!(low.severity, 0.0);

        let high = Finding::new(FindingCategory::Physics, 1.5, "High");
        assert_eq!(high.severity, 1.0);
    }

    #[test]
    fn test_score_summary() {
        let components = ComponentScores {
            integrity: 36.0,
            motion: 22.0,
            shape: 17.0,
            calibration: 13.0,
        };

        let score = FootballLikenessScore::new(components);
        let summary = score.summary();

        assert!(summary.contains("88.0"));
        assert!(summary.contains("PASS"));
    }

    #[test]
    fn test_calculate_likeness_perfect() {
        // Perfect physics
        let physics = PhysicsValidationResult {
            integrity_score: 100.0,
            anomalies: Vec::new(),
            frames_validated: 100,
            clean_frames: 100,
        };

        // Perfect consistency
        let consistency = ConsistencyReport {
            checks: Vec::new(),
            consistency_score: 100.0,
            mismatches: Vec::new(),
            critical_mismatches: Vec::new(),
        };

        let score = calculate_likeness_score(
            &physics,
            &consistency,
            None,
            None,
            &LikenessConfig::default(),
        );

        // With no shape/motion input, we get defaults
        assert!(score.total >= 70.0, "Score {} should be >= 70", score.total);
        assert!(score.passes());
    }

    #[test]
    fn test_calculate_likeness_with_issues() {
        // Poor physics
        let physics = PhysicsValidationResult {
            integrity_score: 60.0,
            anomalies: vec![
                PhysicsAnomaly::PlayerSpeedExceeded {
                    player_idx: 0, team_side: 0, timestamp_ms: 1000,
                    speed_mps: 15.0, threshold_mps: 12.0,
                },
            ],
            frames_validated: 100,
            clean_frames: 60,
        };

        // Poor consistency
        let mut consistency = ConsistencyReport::default();
        consistency.consistency_score = 70.0;
        consistency.mismatches.push("Goals mismatch".to_string());
        consistency.critical_mismatches.push("Goals mismatch".to_string());

        let score = calculate_likeness_score(
            &physics,
            &consistency,
            None,
            None,
            &LikenessConfig::default(),
        );

        // Should have findings
        assert!(!score.findings.is_empty());
        assert!(score.findings.iter().any(|f| f.category == FindingCategory::Physics));
    }

    #[test]
    fn test_quick_validation_pass() {
        let physics = PhysicsValidationResult {
            integrity_score: 80.0,
            anomalies: Vec::new(),
            frames_validated: 100,
            clean_frames: 80,
        };

        let consistency = ConsistencyReport {
            checks: Vec::new(),
            consistency_score: 100.0,
            mismatches: Vec::new(),
            critical_mismatches: Vec::new(),
        };

        assert!(quick_validation(&physics, &consistency));
    }

    #[test]
    fn test_quick_validation_fail_physics() {
        let physics = PhysicsValidationResult {
            integrity_score: 50.0,
            anomalies: vec![
                PhysicsAnomaly::PlayerSpeedExceeded {
                    player_idx: 0, team_side: 0, timestamp_ms: 1000,
                    speed_mps: 15.0, threshold_mps: 12.0,
                },
            ],
            frames_validated: 100,
            clean_frames: 50,
        };

        let consistency = ConsistencyReport::default();

        assert!(!quick_validation(&physics, &consistency));
    }

    #[test]
    fn test_quick_validation_fail_consistency() {
        let physics = PhysicsValidationResult {
            integrity_score: 90.0,
            anomalies: Vec::new(),
            frames_validated: 100,
            clean_frames: 90,
        };

        let mut consistency = ConsistencyReport::default();
        consistency.critical_mismatches.push("Goal mismatch".to_string());

        assert!(!quick_validation(&physics, &consistency));
    }

    #[test]
    fn test_motion_score_with_metrics() {
        let mut findings = Vec::new();
        let config = LikenessConfig::default();

        let movement = TeamMovementMetrics {
            total_distance_m: 10000.0,
            sprint_distance_m: 1000.0,
            high_intensity_distance_m: 2000.0,
            sprint_count: 50,
            sprint_ratio: 0.10, // 10% - within range
            player_sprint_distance: [100.0; 11],
            player_total_distance: [1000.0; 11],
        };

        let input = MovementMetricsInput {
            movement: Some(movement),
            carries: None,
            runs: None,
            entropy: None,
        };

        let score = calculate_motion_score(Some(&input), &config, &mut findings);

        // Should get good score with no findings
        assert!(score >= config.motion_weight * 0.9);
    }

    #[test]
    fn test_shape_score_with_metrics() {
        let mut findings = Vec::new();
        let config = LikenessConfig::default();

        let shape = TeamShapeMetrics {
            width_m: 45.0,
            depth_m: 35.0,
            avg_width: 45.0,
            avg_depth: 35.0,
            line_spacing_def_mid_m: 15.0, // Reasonable spacing
            line_spacing_mid_att_m: 15.0, // Balanced with def-mid
            ..Default::default()
        };

        let score = calculate_shape_score(Some(&shape), &config, &mut findings);

        // Good shape should get near-full score
        assert!(score >= config.shape_weight * 0.9);
        // May have minor findings from shape flags check, but score should be high
    }

    #[test]
    fn test_shape_score_too_narrow() {
        let mut findings = Vec::new();
        let config = LikenessConfig::default();

        let shape = TeamShapeMetrics {
            width_m: 25.0, // Too narrow
            depth_m: 35.0,
            avg_width: 25.0,
            avg_depth: 35.0,
            ..Default::default()
        };

        let score = calculate_shape_score(Some(&shape), &config, &mut findings);

        // Should have deduction and finding
        assert!(score < config.shape_weight);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.description.contains("narrow")));
    }

    // =========================================================================
    // FIX_2601/NEW_FUNC: Snapshot Bridge Tests
    // =========================================================================

    #[test]
    fn test_snapshot_likeness_good_metrics() {
        let gini = GiniMetrics {
            touch_gini: 0.25,
            pass_sent_gini: 0.20,
            pass_recv_gini: 0.30,
            progressive_gini: 0.30,
        };

        let score = calculate_snapshot_likeness(
            &gini,
            0.20,  // 20% halfspace
            0.85,  // Good entropy
            0.90,  // Good balance
        );

        // Should get high score with good metrics
        assert!(score.total >= 85.0, "Good metrics should yield high score: {}", score.total);
        assert_eq!(score.grade, Grade::Pass);
        assert!(score.findings.is_empty(), "Should have no findings");
    }

    #[test]
    fn test_snapshot_likeness_poor_halfspace() {
        let gini = GiniMetrics {
            touch_gini: 0.25,
            pass_sent_gini: 0.20,
            pass_recv_gini: 0.30,
            progressive_gini: 0.30,
        };

        let score = calculate_snapshot_likeness(
            &gini,
            0.05,  // Poor halfspace (5%)
            0.85,
            0.90,
        );

        // Should have finding for low halfspace
        assert!(score.findings.iter().any(|f|
            f.category == FindingCategory::Shape && f.description.contains("half-space")));
    }

    #[test]
    fn test_snapshot_likeness_poor_gini() {
        let gini = GiniMetrics {
            touch_gini: 0.60,      // High (monopoly)
            pass_sent_gini: 0.30,
            pass_recv_gini: 0.60,  // High
            progressive_gini: 0.30,
        };

        let score = calculate_snapshot_likeness(
            &gini,
            0.20,
            0.85,
            0.90,
        );

        // Should have findings for high Gini
        assert!(score.findings.iter().any(|f|
            f.category == FindingCategory::Calibration),
            "Should have calibration findings");

        // Score should be lower than perfect (100) due to Gini deductions
        // Calibration component is only 15 points, so max deduction is ~9 points
        assert!(score.total < 100.0, "Poor Gini should reduce score: {}", score.total);
        assert!(score.components.calibration < 15.0,
            "Calibration score should be reduced: {}", score.components.calibration);
    }

    #[test]
    fn test_snapshot_likeness_input_struct() {
        let input = SnapshotLikenessInput {
            gini: GiniMetrics {
                touch_gini: 0.25,
                pass_sent_gini: 0.20,
                pass_recv_gini: 0.30,
                progressive_gini: 0.30,
            },
            halfspace_rate: 0.20,
            lane_entropy: 0.85,
            lane_balance: 0.90,
        };

        let score = input.calculate();
        assert!(score.total >= 85.0);
    }
}
