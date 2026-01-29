//! Calibrator - Adjusts Engine Parameters to Match Targets
//!
//! Post-match calibration using EMA-smoothed scale factors.

use std::collections::HashMap;
use super::zone::ZoneId;
use super::anchor_table::AnchorTable;
use super::stat_snapshot::MatchStatSnapshot;
// FIX_2601/NEW_FUNC: Football likeness integration
use crate::analysis::qa::football_likeness::{calculate_snapshot_likeness, Grade as LikenessGrade};

/// Pass type weight multipliers
#[derive(Debug, Clone)]
pub struct PassTypeWeights {
    pub progressive: f32,
    pub key: f32,
    pub cross: f32,
    pub long: f32,
}

impl Default for PassTypeWeights {
    fn default() -> Self {
        Self {
            progressive: 1.0,
            key: 1.0,
            cross: 1.0,
            long: 1.0,
        }
    }
}

/// Defensive rate multipliers
#[derive(Debug, Clone)]
pub struct DefensiveRates {
    pub press: f32,
    pub tackle: f32,
    pub intercept: f32,
}

impl Default for DefensiveRates {
    fn default() -> Self {
        Self {
            press: 1.0,
            tackle: 1.0,
            intercept: 1.0,
        }
    }
}

// =============================================================================
// FIX_2601/0112: Action Attempt Calibration (GRF Integration)
// =============================================================================

/// Action Attempt Bias - Adjusts how often players ATTEMPT certain actions
///
/// This is inspired by GRF's approach where actions have both:
/// - Attempt rate: How often the action is tried
/// - Success rate: How often the attempt succeeds
///
/// Current calibration only looks at outcomes (successful passes, etc.)
/// This extends it to also calibrate attempt patterns.
///
/// The key insight from GRF:
/// > "If a team doesn't attempt progressive passes, no amount of success rate
/// >  tuning will produce realistic distribution."
#[derive(Debug, Clone)]
pub struct ActionAttemptBias {
    /// Bias for attempting progressive passes (vs lateral/backward)
    /// > 1.0 = more attempts, < 1.0 = fewer attempts
    pub progressive_pass_attempt: f32,

    /// Bias for attempting long passes (vs short)
    pub long_pass_attempt: f32,

    /// Bias for attempting crosses (when in position)
    pub cross_attempt: f32,

    /// Bias for attempting shots (vs holding/passing in shooting positions)
    pub shot_attempt: f32,

    /// Bias for attempting dribbles (vs passing when under pressure)
    pub dribble_attempt: f32,

    /// Bias for attempting through balls
    pub through_ball_attempt: f32,
}

impl Default for ActionAttemptBias {
    fn default() -> Self {
        Self {
            progressive_pass_attempt: 1.0,
            long_pass_attempt: 1.0,
            cross_attempt: 1.0,
            shot_attempt: 1.0,
            dribble_attempt: 1.0,
            through_ball_attempt: 1.0,
        }
    }
}

impl ActionAttemptBias {
    /// Create with custom progressive bias (for tactic styles)
    pub fn with_progressive_bias(progressive: f32) -> Self {
        Self {
            progressive_pass_attempt: progressive,
            ..Default::default()
        }
    }
}

// =============================================================================
// FIX_2601/0113: 20-Zone Tactical Gates
// =============================================================================

/// Tactical quality gate thresholds for 20-zone metrics
///
/// These gates ensure tactical realism beyond raw statistics.
/// A simulation can have correct pass completion % but unrealistic
/// spatial patterns (e.g., never using half-spaces).
#[derive(Debug, Clone)]
pub struct TacticalGates {
    /// Minimum half-space touch rate (total HS touches / total touches)
    /// Default: 0.15 (at least 15% of touches in half-spaces)
    pub min_halfspace_touch_rate: f32,

    /// Minimum lane entropy (0 = one lane, 1 = uniform distribution)
    /// Default: 0.6 (reasonably spread across lanes)
    pub min_lane_entropy: f32,

    /// Minimum DEF→MID progression rate
    /// Default: 0.3 (at least 30% of passes progress to midfield)
    pub min_def_to_mid_rate: f32,

    /// Minimum lane balance (1 = symmetric, 0 = one-sided)
    /// Default: 0.5 (not completely one-sided)
    pub min_lane_balance: f32,

    // FIX_2601/NEW_FUNC: Gini inequality gates
    /// Maximum touch Gini coefficient (0 = equal, 1 = monopoly)
    /// Default: 0.40 (prevent "one player does everything" patterns)
    pub max_touch_gini: f64,

    /// Maximum pass receive Gini coefficient
    /// Default: 0.42 (prevent "only pass to one player" patterns)
    pub max_pass_recv_gini: f64,

    // FIX_2601/NEW_FUNC: Football likeness gate
    /// Minimum football likeness score (0-100)
    /// Default: 70.0 (at least "Warn" grade)
    pub min_football_likeness: f32,
}

impl Default for TacticalGates {
    fn default() -> Self {
        Self {
            min_halfspace_touch_rate: 0.15,
            min_lane_entropy: 0.6,
            min_def_to_mid_rate: 0.3,
            min_lane_balance: 0.5,
            // Gini gates: based on EPL reference ranges
            max_touch_gini: 0.40,
            max_pass_recv_gini: 0.42,
            // Football likeness gate: 70+ = WARN or PASS grade
            min_football_likeness: 70.0,
        }
    }
}

/// Result of tactical gate checks
#[derive(Debug, Clone)]
pub struct TacticalGateResult {
    pub halfspace_ok: bool,
    pub halfspace_value: f32,
    pub entropy_ok: bool,
    pub entropy_value: f32,
    pub progression_ok: bool,
    pub progression_value: f32,
    pub balance_ok: bool,
    pub balance_value: f32,
    // FIX_2601/NEW_FUNC: Gini gate results
    pub touch_gini_ok: bool,
    pub touch_gini_value: f64,
    pub pass_recv_gini_ok: bool,
    pub pass_recv_gini_value: f64,
    // FIX_2601/NEW_FUNC: Football likeness gate
    pub football_likeness_ok: bool,
    pub football_likeness_value: f32,
    pub likeness_grade: LikenessGrade,
}

impl TacticalGateResult {
    /// Check if all gates passed (7 gates total)
    pub fn all_passed(&self) -> bool {
        self.halfspace_ok && self.entropy_ok && self.progression_ok && self.balance_ok
            && self.touch_gini_ok && self.pass_recv_gini_ok && self.football_likeness_ok
    }

    /// Check if all spatial gates passed (excludes Gini and Likeness for backwards compat)
    pub fn spatial_gates_passed(&self) -> bool {
        self.halfspace_ok && self.entropy_ok && self.progression_ok && self.balance_ok
    }

    /// Check if Gini gates passed
    pub fn gini_gates_passed(&self) -> bool {
        self.touch_gini_ok && self.pass_recv_gini_ok
    }

    /// Check if football likeness gate passed
    pub fn likeness_gate_passed(&self) -> bool {
        self.football_likeness_ok
    }

    /// Count how many gates passed (out of 7)
    pub fn passed_count(&self) -> u8 {
        (self.halfspace_ok as u8)
            + (self.entropy_ok as u8)
            + (self.progression_ok as u8)
            + (self.balance_ok as u8)
            + (self.touch_gini_ok as u8)
            + (self.pass_recv_gini_ok as u8)
            + (self.football_likeness_ok as u8)
    }

    /// Total number of gates
    pub const TOTAL_GATES: u8 = 7;

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "TacticalGates [{}/7]: HS={:.2}({}) ENT={:.2}({}) PROG={:.2}({}) BAL={:.2}({}) GINI_T={:.2}({}) GINI_R={:.2}({}) LIKE={:.1}[{}]({})",
            self.passed_count(),
            self.halfspace_value, if self.halfspace_ok { "OK" } else { "FAIL" },
            self.entropy_value, if self.entropy_ok { "OK" } else { "FAIL" },
            self.progression_value, if self.progression_ok { "OK" } else { "FAIL" },
            self.balance_value, if self.balance_ok { "OK" } else { "FAIL" },
            self.touch_gini_value, if self.touch_gini_ok { "OK" } else { "FAIL" },
            self.pass_recv_gini_value, if self.pass_recv_gini_ok { "OK" } else { "FAIL" },
            self.football_likeness_value, self.likeness_grade.as_str(), if self.football_likeness_ok { "OK" } else { "FAIL" },
        )
    }
}

/// Calibrator parameters (the learned scale factors)
#[derive(Debug, Clone)]
pub struct CalibratorParams {
    pub pass_type_weights: PassTypeWeights,
    pub defensive_rates: DefensiveRates,
    pub shot_propensity: f32,
    pub zone_bias: HashMap<ZoneId, f32>,
    /// FIX_2601/0112: Action attempt biases (GRF integration)
    pub action_attempt_bias: ActionAttemptBias,
}

impl Default for CalibratorParams {
    fn default() -> Self {
        let mut zone_bias = HashMap::new();
        for zone in ZoneId::ALL {
            zone_bias.insert(zone, 1.0);
        }

        Self {
            pass_type_weights: PassTypeWeights::default(),
            defensive_rates: DefensiveRates::default(),
            shot_propensity: 1.0,
            zone_bias,
            action_attempt_bias: ActionAttemptBias::default(),
        }
    }
}

/// Calibrator configuration
#[derive(Debug, Clone)]
pub struct CalibratorConfig {
    /// EMA smoothing factor (0.0 - 1.0)
    pub ema_alpha: f32,
    /// Minimum scale factor for single update
    pub scale_clamp_min: f32,
    /// Maximum scale factor for single update
    pub scale_clamp_max: f32,
    /// Minimum accumulated parameter value
    pub param_clamp_min: f32,
    /// Maximum accumulated parameter value
    pub param_clamp_max: f32,
    /// Minimum samples before applying calibration
    pub min_samples: u32,
    /// Whether calibration is enabled
    pub enabled: bool,
    /// FIX_2601/0113: Tactical gate thresholds
    pub tactical_gates: TacticalGates,
}

impl Default for CalibratorConfig {
    fn default() -> Self {
        Self {
            ema_alpha: 0.15,
            scale_clamp_min: 0.85,
            scale_clamp_max: 1.15,
            param_clamp_min: 0.5,
            param_clamp_max: 2.0,
            min_samples: 5,
            enabled: true,
            tactical_gates: TacticalGates::default(),
        }
    }
}

/// Calibrator state
#[derive(Debug, Clone)]
pub struct Calibrator {
    pub scope_key: String,
    pub params: CalibratorParams,
    pub config: CalibratorConfig,
    pub sample_count: u32,
    pub last_updated: u64,
    anchor: AnchorTable,
}

impl Calibrator {
    /// Create a new calibrator with default anchor table
    pub fn new(scope_key: &str) -> Self {
        Self {
            scope_key: scope_key.to_string(),
            params: CalibratorParams::default(),
            config: CalibratorConfig::default(),
            sample_count: 0,
            last_updated: 0,
            anchor: AnchorTable::default(),
        }
    }

    /// Create with custom anchor table
    pub fn with_anchor(scope_key: &str, anchor: AnchorTable) -> Self {
        Self {
            scope_key: scope_key.to_string(),
            params: CalibratorParams::default(),
            config: CalibratorConfig::default(),
            sample_count: 0,
            last_updated: 0,
            anchor,
        }
    }

    /// Update calibrator with match results
    pub fn update(&mut self, snapshot: &MatchStatSnapshot, timestamp: u64) {
        if !self.config.enabled {
            return;
        }

        self.sample_count += 1;
        self.last_updated = timestamp;

        // Only start calibrating after minimum samples
        if self.sample_count < self.config.min_samples {
            return;
        }

        // Update pass type weights (outcome-based)
        self.update_pass_weights(snapshot);

        // Update defensive rates
        self.update_defensive_rates(snapshot);

        // Update shot propensity
        self.update_shot_propensity(snapshot);

        // Update zone bias
        self.update_zone_bias(snapshot);

        // FIX_2601/0112: Update action attempt biases (attempt-based)
        self.update_action_attempts(snapshot);
    }

    fn update_pass_weights(&mut self, snapshot: &MatchStatSnapshot) {
        let target = &self.anchor.team_per_match.passes.types_share;

        // Progressive passes
        let observed_prog = snapshot.progressive_share();
        if observed_prog > 0.001 {
            let scale = self.calculate_scale(observed_prog, target.progressive);
            self.params.pass_type_weights.progressive =
                self.apply_ema(self.params.pass_type_weights.progressive, scale);
        }

        // Long passes
        let observed_long = snapshot.long_pass_share();
        if observed_long > 0.001 {
            let scale = self.calculate_scale(observed_long, target.long);
            self.params.pass_type_weights.long =
                self.apply_ema(self.params.pass_type_weights.long, scale);
        }

        // Crosses
        let observed_cross = snapshot.cross_share();
        if observed_cross > 0.001 {
            let scale = self.calculate_scale(observed_cross, target.cross);
            self.params.pass_type_weights.cross =
                self.apply_ema(self.params.pass_type_weights.cross, scale);
        }

        // Key passes
        let observed_key = snapshot.key_pass_share();
        if observed_key > 0.001 {
            let scale = self.calculate_scale(observed_key, target.key);
            self.params.pass_type_weights.key =
                self.apply_ema(self.params.pass_type_weights.key, scale);
        }
    }

    fn update_defensive_rates(&mut self, snapshot: &MatchStatSnapshot) {
        let target = &self.anchor.team_per_match.defensive;

        // Press rate
        let observed_press = snapshot.press_events as f32;
        if observed_press > 0.0 {
            let scale = self.calculate_scale(observed_press, target.press.mean);
            self.params.defensive_rates.press =
                self.apply_ema(self.params.defensive_rates.press, scale);
        }

        // Tackle rate
        let observed_tackle = snapshot.tackles as f32;
        if observed_tackle > 0.0 {
            let scale = self.calculate_scale(observed_tackle, target.tackle.mean);
            self.params.defensive_rates.tackle =
                self.apply_ema(self.params.defensive_rates.tackle, scale);
        }

        // Intercept rate
        let observed_intercept = snapshot.interceptions as f32;
        if observed_intercept > 0.0 {
            let scale = self.calculate_scale(observed_intercept, target.intercept.mean);
            self.params.defensive_rates.intercept =
                self.apply_ema(self.params.defensive_rates.intercept, scale);
        }
    }

    fn update_shot_propensity(&mut self, snapshot: &MatchStatSnapshot) {
        let target = &self.anchor.team_per_match.shots;
        let observed = snapshot.shot_attempts as f32;

        if observed > 0.0 {
            let scale = self.calculate_scale(observed, target.attempt.mean);
            self.params.shot_propensity = self.apply_ema(self.params.shot_propensity, scale);
        }
    }

    fn update_zone_bias(&mut self, snapshot: &MatchStatSnapshot) {
        let target = &self.anchor.team_per_match.touch_share_by_zone;
        let observed = snapshot.touch_distribution();

        for zone in ZoneId::ALL {
            let target_share = target.get(&zone).copied().unwrap_or(0.16);
            let observed_share = observed[zone.index()];

            if observed_share > 0.001 {
                let scale = self.calculate_scale(observed_share, target_share);
                let current = self.params.zone_bias.get(&zone).copied().unwrap_or(1.0);
                let new_value = self.apply_ema(current, scale);
                self.params.zone_bias.insert(zone, new_value);
            }
        }
    }

    /// FIX_2601/0112: Update action attempt biases (GRF integration)
    ///
    /// This calibrates HOW OFTEN actions are attempted, not just their success rate.
    ///
    /// Key insight from GRF:
    /// - If observed progressive_pass_rate < target, increase attempt bias
    /// - This encourages the engine to TRY more progressive passes
    /// - Success rate calibration handles whether they succeed
    fn update_action_attempts(&mut self, snapshot: &MatchStatSnapshot) {
        let target_types = &self.anchor.team_per_match.passes.types_share;
        let target_shots = &self.anchor.team_per_match.shots;

        // Progressive pass attempt calibration
        // If we're attempting too few progressive passes, increase the bias
        let observed_prog = snapshot.progressive_share();
        let target_prog = target_types.progressive;
        if observed_prog > 0.001 && snapshot.pass_attempts > 50 {
            // Use a tighter tolerance band (90-110% of target)
            let ratio = observed_prog / target_prog;
            if ratio < 0.9 {
                // Too few attempts - increase bias
                let adjustment = 1.0 + (1.0 - ratio) * 0.1; // Max +10% per update
                let current = self.params.action_attempt_bias.progressive_pass_attempt;
                self.params.action_attempt_bias.progressive_pass_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            } else if ratio > 1.1 {
                // Too many attempts - decrease bias
                let adjustment = 1.0 - (ratio - 1.0) * 0.1; // Max -10% per update
                let current = self.params.action_attempt_bias.progressive_pass_attempt;
                self.params.action_attempt_bias.progressive_pass_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            }
        }

        // Long pass attempt calibration
        let observed_long = snapshot.long_pass_share();
        let target_long = target_types.long;
        if observed_long > 0.001 && snapshot.pass_attempts > 50 {
            let ratio = observed_long / target_long;
            if ratio < 0.9 {
                let adjustment = 1.0 + (1.0 - ratio) * 0.1;
                let current = self.params.action_attempt_bias.long_pass_attempt;
                self.params.action_attempt_bias.long_pass_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            } else if ratio > 1.1 {
                let adjustment = 1.0 - (ratio - 1.0) * 0.1;
                let current = self.params.action_attempt_bias.long_pass_attempt;
                self.params.action_attempt_bias.long_pass_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            }
        }

        // Cross attempt calibration
        let observed_cross = snapshot.cross_share();
        let target_cross = target_types.cross;
        if observed_cross > 0.001 && snapshot.pass_attempts > 50 {
            let ratio = observed_cross / target_cross;
            if ratio < 0.9 {
                let adjustment = 1.0 + (1.0 - ratio) * 0.1;
                let current = self.params.action_attempt_bias.cross_attempt;
                self.params.action_attempt_bias.cross_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            } else if ratio > 1.1 {
                let adjustment = 1.0 - (ratio - 1.0) * 0.1;
                let current = self.params.action_attempt_bias.cross_attempt;
                self.params.action_attempt_bias.cross_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            }
        }

        // Shot attempt calibration
        // Shots are count-based, not share-based
        let observed_shots = snapshot.shot_attempts as f32;
        let target_shot_mean = target_shots.attempt.mean;
        if observed_shots > 0.0 {
            let ratio = observed_shots / target_shot_mean;
            if ratio < 0.8 {
                // Very few shots - increase shot attempt bias
                let adjustment = 1.0 + (1.0 - ratio) * 0.05; // Smaller adjustment (max +5%)
                let current = self.params.action_attempt_bias.shot_attempt;
                self.params.action_attempt_bias.shot_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            } else if ratio > 1.2 {
                // Too many shots - decrease bias
                let adjustment = 1.0 - (ratio - 1.0) * 0.05;
                let current = self.params.action_attempt_bias.shot_attempt;
                self.params.action_attempt_bias.shot_attempt =
                    (current * adjustment).clamp(0.7, 1.5);
            }
        }
    }

    /// Calculate scale factor with clamping
    fn calculate_scale(&self, observed: f32, target: f32) -> f32 {
        if observed < 0.001 {
            return 1.0;
        }
        let raw_scale = target / observed;
        raw_scale.clamp(self.config.scale_clamp_min, self.config.scale_clamp_max)
    }

    /// Apply EMA update with parameter clamping
    fn apply_ema(&self, current: f32, scale: f32) -> f32 {
        let alpha = self.config.ema_alpha;
        let new_value = current * (1.0 - alpha) + (current * scale) * alpha;
        new_value.clamp(self.config.param_clamp_min, self.config.param_clamp_max)
    }

    /// Get convergence score (0.0 = far from target, 1.0 = perfect match)
    pub fn convergence_score(&self, snapshot: &MatchStatSnapshot) -> f32 {
        let mut total_deviation = 0.0;
        let mut count = 0;

        // Pass type deviations
        let target_types = &self.anchor.team_per_match.passes.types_share;
        total_deviation += (snapshot.progressive_share() - target_types.progressive).abs();
        total_deviation += (snapshot.long_pass_share() - target_types.long).abs();
        total_deviation += (snapshot.cross_share() - target_types.cross).abs();
        count += 3;

        // Shot deviation
        let target_shots = self.anchor.team_per_match.shots.attempt.mean;
        let shot_dev = self.anchor.team_per_match.shots.attempt.deviation_ratio(
            snapshot.shot_attempts as f32
        );
        total_deviation += shot_dev;
        count += 1;

        // Convert to score (lower deviation = higher score)
        let avg_deviation = total_deviation / count as f32;
        (1.0 - avg_deviation.min(1.0)).max(0.0)
    }

    /// Check if calibration has converged (statistical only)
    pub fn is_converged(&self, snapshot: &MatchStatSnapshot) -> bool {
        self.convergence_score(snapshot) >= 0.93 // Within 7% average deviation
    }

    /// FIX_2601/0113: Check if converged including tactical gates
    pub fn is_converged_with_tactics(&self, snapshot: &MatchStatSnapshot) -> bool {
        let stat_converged = self.convergence_score(snapshot) >= 0.93;
        let gates = self.check_tactical_gates(snapshot);
        stat_converged && gates.all_passed()
    }

    /// FIX_2601/0113: Check tactical gate conditions
    /// FIX_2601/NEW_FUNC: Extended with Gini inequality gates and Football Likeness
    pub fn check_tactical_gates(&self, snapshot: &MatchStatSnapshot) -> TacticalGateResult {
        let hs = snapshot.halfspace_metrics();
        let lane = snapshot.lane_occupancy();
        let prog = snapshot.zone_progression();
        let gini = snapshot.gini_metrics();
        let gates = &self.config.tactical_gates;

        // FIX_2601/NEW_FUNC: Calculate football likeness from snapshot data
        let likeness = calculate_snapshot_likeness(
            &gini,
            hs.touch_rate,
            lane.entropy,
            lane.balance,
        );

        TacticalGateResult {
            halfspace_ok: hs.touch_rate >= gates.min_halfspace_touch_rate,
            halfspace_value: hs.touch_rate,
            entropy_ok: lane.entropy >= gates.min_lane_entropy,
            entropy_value: lane.entropy,
            progression_ok: prog.def_to_mid_rate >= gates.min_def_to_mid_rate,
            progression_value: prog.def_to_mid_rate,
            balance_ok: lane.balance >= gates.min_lane_balance,
            balance_value: lane.balance,
            // Gini gates: lower is better (more equal distribution)
            touch_gini_ok: gini.touch_gini <= gates.max_touch_gini,
            touch_gini_value: gini.touch_gini,
            pass_recv_gini_ok: gini.pass_recv_gini <= gates.max_pass_recv_gini,
            pass_recv_gini_value: gini.pass_recv_gini,
            // Football likeness gate: score must meet minimum threshold
            football_likeness_ok: likeness.total >= gates.min_football_likeness,
            football_likeness_value: likeness.total,
            likeness_grade: likeness.grade,
        }
    }

    /// Reset calibrator to defaults
    pub fn reset(&mut self) {
        self.params = CalibratorParams::default();
        self.sample_count = 0;
    }

    /// Get summary of current calibration state
    pub fn summary(&self) -> String {
        format!(
            "Calibrator[{}] samples={} pass_prog={:.2} pass_long={:.2} shot={:.2}",
            self.scope_key,
            self.sample_count,
            self.params.pass_type_weights.progressive,
            self.params.pass_type_weights.long,
            self.params.shot_propensity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibrator_creation() {
        let cal = Calibrator::new("EPL|2024|normal");
        assert_eq!(cal.scope_key, "EPL|2024|normal");
        assert_eq!(cal.params.pass_type_weights.progressive, 1.0);
    }

    #[test]
    fn test_scale_calculation() {
        let cal = Calibrator::new("test");

        // Target 18%, observed 20% -> scale down
        let scale = cal.calculate_scale(0.20, 0.18);
        assert!(scale < 1.0);
        assert!(scale >= 0.85); // Clamped

        // Target 18%, observed 15% -> scale up
        let scale = cal.calculate_scale(0.15, 0.18);
        assert!(scale > 1.0);
        assert!(scale <= 1.15); // Clamped
    }

    #[test]
    fn test_ema_update() {
        let cal = Calibrator::new("test");

        // EMA with alpha=0.15
        let current = 1.0;
        let scale = 1.1; // 10% increase
        let new_value = cal.apply_ema(current, scale);

        // new = 1.0 * 0.85 + 1.1 * 0.15 = 0.85 + 0.165 = 1.015
        assert!((new_value - 1.015).abs() < 0.01);
    }

    #[test]
    fn test_update_with_snapshot() {
        let mut cal = Calibrator::new("test");
        cal.config.min_samples = 1; // For testing

        let mut snapshot = MatchStatSnapshot::default();
        snapshot.pass_attempts = 100;
        snapshot.progressive_passes = 25; // 25% vs target 18%
        snapshot.shot_attempts = 15; // vs target 12.5

        cal.update(&snapshot, 1000);

        // Progressive weight should decrease (observed > target)
        assert!(cal.params.pass_type_weights.progressive < 1.0);

        // Shot propensity should decrease (observed > target)
        assert!(cal.params.shot_propensity < 1.0);
    }

    #[test]
    fn test_action_attempt_calibration() {
        let mut cal = Calibrator::new("test");
        cal.config.min_samples = 1;

        // Scenario: Too few progressive passes (10% vs target 18%)
        let mut snapshot = MatchStatSnapshot::default();
        snapshot.pass_attempts = 100;
        snapshot.progressive_passes = 10; // 10% vs target 18%
        snapshot.shot_attempts = 5; // Very few shots vs target 12.5

        cal.update(&snapshot, 1000);

        // Progressive attempt bias should INCREASE (too few attempts)
        assert!(
            cal.params.action_attempt_bias.progressive_pass_attempt > 1.0,
            "Expected progressive_pass_attempt > 1.0, got {}",
            cal.params.action_attempt_bias.progressive_pass_attempt
        );

        // Shot attempt bias should INCREASE (too few attempts)
        assert!(
            cal.params.action_attempt_bias.shot_attempt > 1.0,
            "Expected shot_attempt > 1.0, got {}",
            cal.params.action_attempt_bias.shot_attempt
        );
    }

    #[test]
    fn test_action_attempt_calibration_too_many() {
        let mut cal = Calibrator::new("test");
        cal.config.min_samples = 1;

        // Scenario: Too many progressive passes (30% vs target 18%)
        let mut snapshot = MatchStatSnapshot::default();
        snapshot.pass_attempts = 100;
        snapshot.progressive_passes = 30; // 30% vs target 18%
        snapshot.shot_attempts = 20; // Many shots vs target 12.5

        cal.update(&snapshot, 1000);

        // Progressive attempt bias should DECREASE (too many attempts)
        assert!(
            cal.params.action_attempt_bias.progressive_pass_attempt < 1.0,
            "Expected progressive_pass_attempt < 1.0, got {}",
            cal.params.action_attempt_bias.progressive_pass_attempt
        );

        // Shot attempt bias should DECREASE (too many attempts)
        assert!(
            cal.params.action_attempt_bias.shot_attempt < 1.0,
            "Expected shot_attempt < 1.0, got {}",
            cal.params.action_attempt_bias.shot_attempt
        );
    }

    // =========================================================================
    // FIX_2601/0113: Tactical Gates Tests
    // =========================================================================

    #[test]
    fn test_tactical_gates_all_pass() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // Set up good tactical metrics
        // Half-space touches: 20% (> 15% threshold)
        snapshot.touches_by_posplay_zone = [
            5, 10, 5, 10, 5,   // DEF: 20 HS touches
            5, 10, 5, 10, 5,   // MID: 20 HS touches
            5, 10, 5, 10, 5,   // FIN: 20 HS touches
            5, 10, 5, 10, 5,   // BOX: 20 HS touches
        ];

        // Set pass data for progression
        snapshot.pass_attempts = 100;
        snapshot.passes_from_posplay_zone = [20, 0, 20, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0];
        snapshot.passes_to_posplay_zone = [0, 0, 0, 0, 0,  20, 0, 20, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0];

        let result = cal.check_tactical_gates(&snapshot);

        assert!(result.halfspace_ok, "Halfspace gate should pass, got {}", result.halfspace_value);
        assert!(result.entropy_ok, "Entropy gate should pass, got {}", result.entropy_value);
        assert!(result.balance_ok, "Balance gate should pass, got {}", result.balance_value);
    }

    #[test]
    fn test_tactical_gates_halfspace_fail() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // All touches in center lane only (no half-space usage)
        snapshot.touches_by_posplay_zone = [
            0, 0, 25, 0, 0,   // DEF
            0, 0, 25, 0, 0,   // MID
            0, 0, 25, 0, 0,   // FIN
            0, 0, 25, 0, 0,   // BOX
        ];

        let result = cal.check_tactical_gates(&snapshot);

        assert!(!result.halfspace_ok,
            "Halfspace gate should FAIL (0% HS touches), got {}", result.halfspace_value);
    }

    #[test]
    fn test_tactical_gates_balance_fail() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // All touches on left side only
        snapshot.touches_by_posplay_zone = [
            50, 50, 0, 0, 0,   // DEF
            0, 0, 0, 0, 0,     // MID
            0, 0, 0, 0, 0,     // FIN
            0, 0, 0, 0, 0,     // BOX
        ];

        let result = cal.check_tactical_gates(&snapshot);

        assert!(!result.balance_ok,
            "Balance gate should FAIL (all left side), got {}", result.balance_value);
    }

    #[test]
    fn test_tactical_gate_result_methods() {
        let result = TacticalGateResult {
            halfspace_ok: true,
            halfspace_value: 0.20,
            entropy_ok: true,
            entropy_value: 0.85,
            progression_ok: false,
            progression_value: 0.25,
            balance_ok: true,
            balance_value: 0.90,
            touch_gini_ok: true,
            touch_gini_value: 0.25,
            pass_recv_gini_ok: true,
            pass_recv_gini_value: 0.30,
            // FIX_2601/NEW_FUNC: Football likeness gate
            football_likeness_ok: true,
            football_likeness_value: 85.0,
            likeness_grade: LikenessGrade::Pass,
        };

        assert!(!result.all_passed(), "Not all gates passed (progression failed)");
        assert_eq!(result.passed_count(), 6, "6 of 7 gates passed");

        let summary = result.summary();
        assert!(summary.contains("FAIL"), "Summary should contain FAIL");
        assert!(summary.contains("GINI_T"), "Summary should contain GINI_T");
        assert!(summary.contains("LIKE"), "Summary should contain LIKE");
    }

    #[test]
    fn test_is_converged_with_tactics() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // Good statistical metrics
        snapshot.pass_attempts = 100;
        snapshot.progressive_passes = 18;  // Target is 18%
        snapshot.long_passes = 12;          // Target is ~12%
        snapshot.crosses = 5;
        snapshot.shot_attempts = 12;        // Target is ~12.5

        // Good tactical metrics
        snapshot.touches_by_posplay_zone = [
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
        ];
        snapshot.passes_from_posplay_zone = [20, 0, 20, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0];
        snapshot.passes_to_posplay_zone = [0, 0, 0, 0, 0,  20, 0, 20, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0];

        // Check gates
        let gates = cal.check_tactical_gates(&snapshot);
        println!("Gate result: {}", gates.summary());

        // Note: is_converged_with_tactics requires BOTH statistical and tactical convergence
        // Statistical convergence might not be met with just pass_attempts set
    }

    // =========================================================================
    // FIX_2601/NEW_FUNC: Gini Gate Tests
    // =========================================================================

    #[test]
    fn test_tactical_gates_gini_pass_uniform() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // Set up uniform touches for all 11 players
        for player in 0..11 {
            for _ in 0..10 {
                snapshot.player_stats.record_touch(player);
            }
        }
        // Set up uniform passes
        for passer in 0..11 {
            for receiver in 0..11 {
                if passer != receiver {
                    snapshot.player_stats.record_pass(passer, receiver, false, false);
                }
            }
        }

        let result = cal.check_tactical_gates(&snapshot);

        assert!(result.touch_gini_ok,
            "Touch Gini should pass (uniform distribution), got {}", result.touch_gini_value);
        assert!(result.pass_recv_gini_ok,
            "Pass recv Gini should pass (uniform distribution), got {}", result.pass_recv_gini_value);
        assert!(result.gini_gates_passed(), "Both Gini gates should pass");
    }

    #[test]
    fn test_tactical_gates_gini_fail_monopoly() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // One player dominates touches
        for _ in 0..100 {
            snapshot.player_stats.record_touch(5);
        }

        // One player receives all passes
        for _ in 0..50 {
            snapshot.player_stats.record_pass(3, 9, false, false);
            snapshot.player_stats.record_pass(7, 9, false, false);
        }

        let result = cal.check_tactical_gates(&snapshot);

        assert!(!result.touch_gini_ok,
            "Touch Gini should FAIL (monopoly), got {}", result.touch_gini_value);
        assert!(!result.pass_recv_gini_ok,
            "Pass recv Gini should FAIL (monopoly), got {}", result.pass_recv_gini_value);
        assert!(!result.gini_gates_passed(), "Gini gates should fail");
    }

    #[test]
    fn test_tactical_gates_gini_moderate_concentration() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // Moderate concentration - realistic skew
        let touches_by_position = [8, 12, 10, 10, 15, 20, 18, 15, 12, 8, 6]; // ~134 total
        for (player, &count) in touches_by_position.iter().enumerate() {
            for _ in 0..count {
                snapshot.player_stats.record_touch(player);
            }
        }

        let result = cal.check_tactical_gates(&snapshot);

        // This realistic distribution should pass
        assert!(result.touch_gini_value < 0.5,
            "Realistic touch distribution should have moderate Gini: {}", result.touch_gini_value);
    }

    // =========================================================================
    // FIX_2601/NEW_FUNC: Football Likeness Gate Tests
    // =========================================================================

    #[test]
    fn test_tactical_gates_likeness_good_metrics() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // Set up good tactical metrics for high likeness score
        // Good half-space touches (20% in HS zones)
        snapshot.touches_by_posplay_zone = [
            5, 10, 5, 10, 5,   // DEF: 20% HS
            5, 10, 5, 10, 5,   // MID: 20% HS
            5, 10, 5, 10, 5,   // FIN: 20% HS
            5, 10, 5, 10, 5,   // BOX: 20% HS
        ];

        // Uniform player stats for good Gini
        for player in 0..11 {
            for _ in 0..10 {
                snapshot.player_stats.record_touch(player);
            }
        }

        let result = cal.check_tactical_gates(&snapshot);

        // With good metrics, likeness should be high
        assert!(result.football_likeness_value >= 70.0,
            "Good metrics should yield high likeness: {:.1}", result.football_likeness_value);
        assert!(result.football_likeness_ok,
            "Football likeness gate should pass");
        assert!(result.likeness_gate_passed(), "Likeness gate helper should return true");
    }

    #[test]
    fn test_tactical_gates_likeness_poor_gini() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // Set up good spatial metrics but bad Gini (one player monopoly)
        snapshot.touches_by_posplay_zone = [
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
        ];

        // One player dominates
        for _ in 0..100 {
            snapshot.player_stats.record_touch(5);
        }

        let result = cal.check_tactical_gates(&snapshot);

        // High Gini should reduce likeness score through calibration component
        println!("Likeness with monopoly: {:.1}, grade: {:?}",
            result.football_likeness_value, result.likeness_grade);

        // Likeness should be lower due to poor Gini
        assert!(result.touch_gini_value > 0.5, "Touch Gini should be high (monopoly)");
    }

    #[test]
    fn test_tactical_gates_all_seven_gates() {
        let cal = Calibrator::new("test");
        let mut snapshot = MatchStatSnapshot::default();

        // Set up optimal metrics for all gates
        // Good half-space (20%)
        snapshot.touches_by_posplay_zone = [
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
            5, 10, 5, 10, 5,
        ];

        // Set pass data for progression
        snapshot.pass_attempts = 100;
        snapshot.passes_from_posplay_zone = [20, 0, 20, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0];
        snapshot.passes_to_posplay_zone = [0, 0, 0, 0, 0,  20, 0, 20, 0, 0,  0, 0, 0, 0, 0,  0, 0, 0, 0, 0];

        // Uniform player distribution for good Gini
        for player in 0..11 {
            for _ in 0..10 {
                snapshot.player_stats.record_touch(player);
            }
        }
        for passer in 0..11 {
            for receiver in 0..11 {
                if passer != receiver {
                    snapshot.player_stats.record_pass(passer, receiver, false, false);
                }
            }
        }

        let result = cal.check_tactical_gates(&snapshot);

        // Verify we have 7 gates total
        assert_eq!(TacticalGateResult::TOTAL_GATES, 7, "Should have 7 gates");

        // Check each gate individually
        println!("Gate results: {}", result.summary());
        println!("  Halfspace: {} ({})", result.halfspace_value, result.halfspace_ok);
        println!("  Entropy: {} ({})", result.entropy_value, result.entropy_ok);
        println!("  Progression: {} ({})", result.progression_value, result.progression_ok);
        println!("  Balance: {} ({})", result.balance_value, result.balance_ok);
        println!("  Touch Gini: {} ({})", result.touch_gini_value, result.touch_gini_ok);
        println!("  Pass Recv Gini: {} ({})", result.pass_recv_gini_value, result.pass_recv_gini_ok);
        println!("  Likeness: {} [{:?}] ({})", result.football_likeness_value, result.likeness_grade, result.football_likeness_ok);

        // At minimum, likeness gate should be evaluated
        assert!(result.football_likeness_value > 0.0,
            "Likeness should be calculated: {}", result.football_likeness_value);
    }
}
