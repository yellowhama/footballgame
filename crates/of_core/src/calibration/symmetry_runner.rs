//! SymmetryMetaRunner - Multi-Variant Symmetry Tester
//!
//! FIX_2601/0112 Phase 2: Tests scenario symmetry across 4 variants:
//! - Base case (no flip, no swap)
//! - Direction flipped (home attacks left instead of right)
//! - Teams swapped (home/away exchanged)
//! - Both flipped and swapped
//!
//! This detects bias in simulation logic that favors home/away or attack direction.

use std::collections::HashMap;

use super::scenario_runner::ScenarioRunner;
use super::scenarios::{ScenarioResult, SymmetryVariant, TestScenario};

/// Result from running symmetry tests on a scenario.
#[derive(Debug, Clone)]
pub struct SymmetryReport {
    /// Scenario ID
    pub scenario_id: String,

    /// Results for each variant
    pub variant_results: Vec<(SymmetryVariant, ScenarioResult)>,

    /// Whether the scenario is symmetric (no significant bias)
    pub is_symmetric: bool,

    /// List of detected symmetry violations
    pub violations: Vec<SymmetryViolation>,

    /// Aggregate statistics
    pub stats: SymmetryStats,
}

/// A detected symmetry violation.
#[derive(Debug, Clone)]
pub struct SymmetryViolation {
    /// Type of violation
    pub violation_type: ViolationType,

    /// Description of the violation
    pub description: String,

    /// Severity (0.0 = minor, 1.0 = severe)
    pub severity: f32,
}

/// Types of symmetry violations.
#[derive(Debug, Clone, PartialEq)]
pub enum ViolationType {
    /// Direction flip changes outcome significantly
    DirectionBias,

    /// Team swap changes outcome significantly
    TeamBias,

    /// Home team has systematic advantage
    HomeAdvantage,

    /// Metric variance too high across variants
    HighVariance,
}

/// Aggregate statistics across variants.
#[derive(Debug, Clone, Default)]
pub struct SymmetryStats {
    /// Pass rate across variants
    pub pass_rate: f32,

    /// Standard deviation of pass rates
    pub pass_rate_std: f32,

    /// Average duration in ticks
    pub avg_duration_ticks: f32,

    /// Duration variance
    pub duration_variance: f32,

    /// Metric averages across variants
    pub metric_averages: HashMap<String, f32>,

    /// Metric variances
    pub metric_variances: HashMap<String, f32>,
}

/// SymmetryMetaRunner runs a scenario with all symmetry variants.
pub struct SymmetryMetaRunner {
    /// Random seed base
    seed: u64,

    /// Tolerance for considering results symmetric (0.0 - 1.0)
    tolerance: f32,
}

impl SymmetryMetaRunner {
    /// Create a new SymmetryMetaRunner with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            tolerance: 0.2, // 20% tolerance by default
        }
    }

    /// Set the tolerance for symmetry checks.
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance.clamp(0.0, 1.0);
        self
    }

    /// Run a scenario with all 4 symmetry variants.
    pub fn run_all_variants(&self, scenario: &TestScenario) -> Result<SymmetryReport, String> {
        let runner = ScenarioRunner::new(self.seed);
        let variants = SymmetryVariant::all();

        let mut variant_results = Vec::new();

        for variant in &variants {
            let result = runner.run_with_variant(scenario, Some(variant))?;
            variant_results.push((variant.clone(), result));
        }

        // Analyze results for symmetry
        let violations = self.detect_violations(&variant_results);
        let stats = self.compute_stats(&variant_results);
        let is_symmetric = violations.iter().all(|v| v.severity < 0.5);

        Ok(SymmetryReport {
            scenario_id: scenario.id.clone(),
            variant_results,
            is_symmetric,
            violations,
            stats,
        })
    }

    /// Run and check symmetry, returning true if symmetric.
    pub fn check_symmetry(&self, scenario: &TestScenario) -> Result<bool, String> {
        let report = self.run_all_variants(scenario)?;
        Ok(report.is_symmetric)
    }

    /// Detect symmetry violations from variant results.
    fn detect_violations(
        &self,
        results: &[(SymmetryVariant, ScenarioResult)],
    ) -> Vec<SymmetryViolation> {
        let mut violations = Vec::new();

        // Check pass/fail consistency
        let pass_count: usize = results.iter().filter(|(_, r)| r.passed).count();
        if pass_count > 0 && pass_count < results.len() {
            // Some variants pass, some fail - this is a violation
            let failing_variants: Vec<_> = results
                .iter()
                .filter(|(_, r)| !r.passed)
                .map(|(v, _)| format!("({}, {})", v.flip_direction, v.swap_teams))
                .collect();

            violations.push(SymmetryViolation {
                violation_type: ViolationType::HighVariance,
                description: format!(
                    "Inconsistent pass/fail: {} of {} passed. Failing: {:?}",
                    pass_count,
                    results.len(),
                    failing_variants
                ),
                severity: 0.8,
            });
        }

        // Check for direction bias
        let base_result = results.iter().find(|(v, _)| !v.flip_direction && !v.swap_teams);
        let flipped_result = results.iter().find(|(v, _)| v.flip_direction && !v.swap_teams);

        if let (Some((_, base)), Some((_, flipped))) = (base_result, flipped_result) {
            if base.passed != flipped.passed {
                violations.push(SymmetryViolation {
                    violation_type: ViolationType::DirectionBias,
                    description: format!(
                        "Direction flip changes outcome: base={}, flipped={}",
                        base.passed, flipped.passed
                    ),
                    severity: 0.9,
                });
            }
        }

        // Check for team swap bias
        let swap_result = results.iter().find(|(v, _)| !v.flip_direction && v.swap_teams);

        if let (Some((_, base)), Some((_, swapped))) = (base_result, swap_result) {
            if base.passed != swapped.passed {
                violations.push(SymmetryViolation {
                    violation_type: ViolationType::TeamBias,
                    description: format!(
                        "Team swap changes outcome: base={}, swapped={}",
                        base.passed, swapped.passed
                    ),
                    severity: 0.9,
                });
            }
        }

        // Check duration variance
        let durations: Vec<f32> = results.iter().map(|(_, r)| r.duration_ticks as f32).collect();
        if durations.len() >= 2 {
            let avg: f32 = durations.iter().sum::<f32>() / durations.len() as f32;
            let variance: f32 = durations.iter().map(|d| (d - avg).powi(2)).sum::<f32>()
                / durations.len() as f32;
            let std_dev = variance.sqrt();

            // If std_dev is more than 20% of avg, flag it
            if avg > 0.0 && std_dev / avg > self.tolerance {
                violations.push(SymmetryViolation {
                    violation_type: ViolationType::HighVariance,
                    description: format!(
                        "High duration variance: avg={:.1}, std={:.1} ({:.1}%)",
                        avg,
                        std_dev,
                        std_dev / avg * 100.0
                    ),
                    severity: (std_dev / avg).min(1.0),
                });
            }
        }

        violations
    }

    /// Compute aggregate statistics from variant results.
    fn compute_stats(&self, results: &[(SymmetryVariant, ScenarioResult)]) -> SymmetryStats {
        let n = results.len() as f32;
        if n == 0.0 {
            return SymmetryStats::default();
        }

        // Pass rate
        let pass_count = results.iter().filter(|(_, r)| r.passed).count() as f32;
        let pass_rate = pass_count / n;

        // Duration stats
        let durations: Vec<f32> = results.iter().map(|(_, r)| r.duration_ticks as f32).collect();
        let avg_duration: f32 = durations.iter().sum::<f32>() / n;
        let duration_variance: f32 = durations.iter().map(|d| (d - avg_duration).powi(2)).sum::<f32>() / n;

        // Collect all probe names
        let mut all_probes: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (_, result) in results {
            for key in result.probe_values.keys() {
                all_probes.insert(key.clone());
            }
        }

        // Compute metric averages and variances
        let mut metric_averages = HashMap::new();
        let mut metric_variances = HashMap::new();

        for probe in all_probes {
            let values: Vec<f32> = results
                .iter()
                .filter_map(|(_, r)| r.probe_values.get(&probe).copied())
                .collect();

            if !values.is_empty() {
                let avg = values.iter().sum::<f32>() / values.len() as f32;
                let variance = values.iter().map(|v| (v - avg).powi(2)).sum::<f32>() / values.len() as f32;
                metric_averages.insert(probe.clone(), avg);
                metric_variances.insert(probe, variance);
            }
        }

        SymmetryStats {
            pass_rate,
            pass_rate_std: if pass_rate > 0.0 && pass_rate < 1.0 {
                (pass_rate * (1.0 - pass_rate)).sqrt()
            } else {
                0.0
            },
            avg_duration_ticks: avg_duration,
            duration_variance,
            metric_averages,
            metric_variances,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symmetry_runner_creation() {
        let runner = SymmetryMetaRunner::new(12345);
        assert_eq!(runner.seed, 12345);
        assert!((runner.tolerance - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_symmetry_runner_with_tolerance() {
        let runner = SymmetryMetaRunner::new(12345).with_tolerance(0.5);
        assert!((runner.tolerance - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_detect_violations_consistent_pass() {
        let runner = SymmetryMetaRunner::new(12345);

        let results = vec![
            (
                SymmetryVariant { flip_direction: false, swap_teams: false },
                make_result(true, 100),
            ),
            (
                SymmetryVariant { flip_direction: true, swap_teams: false },
                make_result(true, 105),
            ),
            (
                SymmetryVariant { flip_direction: false, swap_teams: true },
                make_result(true, 98),
            ),
            (
                SymmetryVariant { flip_direction: true, swap_teams: true },
                make_result(true, 102),
            ),
        ];

        let violations = runner.detect_violations(&results);
        // All passed consistently with similar durations - should be minimal violations
        let severe_violations: Vec<_> = violations.iter().filter(|v| v.severity > 0.5).collect();
        assert!(severe_violations.is_empty(), "Should have no severe violations");
    }

    #[test]
    fn test_detect_violations_direction_bias() {
        let runner = SymmetryMetaRunner::new(12345);

        let results = vec![
            (
                SymmetryVariant { flip_direction: false, swap_teams: false },
                make_result(true, 100),
            ),
            (
                SymmetryVariant { flip_direction: true, swap_teams: false },
                make_result(false, 100), // Fails when flipped
            ),
            (
                SymmetryVariant { flip_direction: false, swap_teams: true },
                make_result(true, 100),
            ),
            (
                SymmetryVariant { flip_direction: true, swap_teams: true },
                make_result(false, 100),
            ),
        ];

        let violations = runner.detect_violations(&results);
        let direction_bias = violations.iter().any(|v| v.violation_type == ViolationType::DirectionBias);
        assert!(direction_bias, "Should detect direction bias");
    }

    fn make_result(passed: bool, duration: u64) -> ScenarioResult {
        ScenarioResult {
            scenario_id: "test".to_string(),
            passed,
            condition_results: Vec::new(),
            probe_values: HashMap::new(),
            final_ball_position_m: (50.0, 34.0),
            duration_ticks: duration,
        }
    }
}
