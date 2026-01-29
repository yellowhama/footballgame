//! # Scout Report Model
//!
//! Core data structures for the scouting system.
//!
//! ## Features
//! - `ScoutedValue<T>` with Bayesian uncertainty updates
//! - Scout levels (L0-L4) with different accuracy multipliers
//! - Integration with match metrics for automatic scouting
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: SCOUT_REPORT_SYSTEM.md

use std::time::SystemTime;

/// Scout level determining report detail and accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScoutLevel {
    /// L0: Rumor - basic tags only, high uncertainty
    Rumor = 0,
    /// L1: Basic - qualitative grades
    Basic = 1,
    /// L2: Report - range bars for stats
    Report = 2,
    /// L3: Detail - point estimates with error bars
    Detail = 3,
    /// L4: Elite - near-exact values, hidden stats revealed
    Elite = 4,
}

impl ScoutLevel {
    /// Uncertainty multiplier for this level.
    pub fn uncertainty_mult(&self) -> f32 {
        match self {
            ScoutLevel::Rumor => 2.2,
            ScoutLevel::Basic => 1.6,
            ScoutLevel::Report => 1.2,
            ScoutLevel::Detail => 0.9,
            ScoutLevel::Elite => 0.7,
        }
    }

    /// Number of style tags revealed at this level.
    pub fn style_tags_count(&self) -> usize {
        match self {
            ScoutLevel::Rumor => 3,
            ScoutLevel::Basic => 4,
            ScoutLevel::Report => 6,
            ScoutLevel::Detail => 8,
            ScoutLevel::Elite => 10,
        }
    }

    /// Number of player stats revealed at this level.
    pub fn player_stats_count(&self) -> usize {
        match self {
            ScoutLevel::Rumor => 1,  // OVR only
            ScoutLevel::Basic => 3,
            ScoutLevel::Report => 6,
            ScoutLevel::Detail => 10,
            ScoutLevel::Elite => 15, // Including hidden
        }
    }

    /// Create from numeric level (0-4).
    pub fn from_level(level: u8) -> Self {
        match level {
            0 => ScoutLevel::Rumor,
            1 => ScoutLevel::Basic,
            2 => ScoutLevel::Report,
            3 => ScoutLevel::Detail,
            _ => ScoutLevel::Elite,
        }
    }
}

impl Default for ScoutLevel {
    fn default() -> Self {
        ScoutLevel::Basic
    }
}

// ============================================================================
// ScoutedValue<T> - Core uncertainty model
// ============================================================================

/// A scouted value with Bayesian uncertainty modeling.
///
/// Uses a normal distribution model where:
/// - `estimate` is the mean (point estimate)
/// - `sigma` is the standard deviation (uncertainty)
/// - Observations update the estimate using Bayesian inference
#[derive(Debug, Clone)]
pub struct ScoutedValue<T> {
    /// Point estimate of the value
    pub estimate: T,
    /// Standard deviation (uncertainty)
    pub sigma: f32,
    /// Confidence level (0.0 - 1.0), derived from sigma
    pub confidence: f32,
    /// Number of observations used
    pub sample_count: u32,
    /// When this value was last observed
    pub last_observed: Option<SystemTime>,
    /// Freshness factor (1.0 = fresh, decays over time)
    pub freshness: f32,
}

impl<T: Default> Default for ScoutedValue<T> {
    fn default() -> Self {
        Self {
            estimate: T::default(),
            sigma: 15.0, // High initial uncertainty
            confidence: 0.0,
            sample_count: 0,
            last_observed: None,
            freshness: 0.0,
        }
    }
}

impl<T: Clone> ScoutedValue<T> {
    /// Create a new scouted value with known estimate.
    pub fn new(estimate: T, sigma: f32) -> Self {
        Self {
            estimate,
            sigma,
            confidence: calculate_confidence(sigma),
            sample_count: 1,
            last_observed: Some(SystemTime::now()),
            freshness: 1.0,
        }
    }

    /// Create with full parameters.
    pub fn with_samples(estimate: T, sigma: f32, sample_count: u32) -> Self {
        Self {
            estimate,
            sigma,
            confidence: calculate_confidence(sigma),
            sample_count,
            last_observed: Some(SystemTime::now()),
            freshness: 1.0,
        }
    }

    /// Update freshness based on time elapsed.
    ///
    /// # Arguments
    /// * `decay_per_day` - How much freshness decays per day (0.0-1.0)
    pub fn update_freshness(&mut self, decay_per_day: f32) {
        if let Some(observed) = self.last_observed {
            if let Ok(elapsed) = SystemTime::now().duration_since(observed) {
                let days = elapsed.as_secs_f32() / 86400.0;
                self.freshness = (1.0 - decay_per_day * days).max(0.0);
            }
        }
    }

    /// Check if the value is stale (freshness below threshold).
    pub fn is_stale(&self, threshold: f32) -> bool {
        self.freshness < threshold
    }
}

// ============================================================================
// Bayesian Update for f32
// ============================================================================

impl ScoutedValue<f32> {
    /// Bayesian update with a new observation.
    ///
    /// Uses conjugate normal prior update:
    /// - Prior: N(estimate, sigma²)
    /// - Likelihood: N(observation, observation_sigma²)
    /// - Posterior: N(new_estimate, new_sigma²)
    ///
    /// # Arguments
    /// * `observation` - The observed value
    /// * `observation_sigma` - Uncertainty of the observation
    pub fn update_with_observation(&mut self, observation: f32, observation_sigma: f32) {
        // Precision (inverse variance)
        let prior_precision = 1.0 / (self.sigma * self.sigma);
        let obs_precision = 1.0 / (observation_sigma * observation_sigma);

        // Posterior precision
        let posterior_precision = prior_precision + obs_precision;

        // Posterior mean (precision-weighted average)
        let posterior_mean = (prior_precision * self.estimate + obs_precision * observation)
            / posterior_precision;

        // Posterior sigma
        let posterior_sigma = (1.0 / posterior_precision).sqrt();

        // Update state
        self.estimate = posterior_mean;
        self.sigma = posterior_sigma;
        self.confidence = calculate_confidence(posterior_sigma);
        self.sample_count += 1;
        self.last_observed = Some(SystemTime::now());
        self.freshness = 1.0;
    }

    /// Get the uncertainty range at given confidence interval.
    /// Returns (lower, upper) bounds.
    pub fn range(&self, z_score: f32) -> (f32, f32) {
        let margin = self.sigma * z_score;
        (self.estimate - margin, self.estimate + margin)
    }

    /// 95% confidence interval.
    pub fn range_95(&self) -> (f32, f32) {
        self.range(1.96)
    }

    /// 68% confidence interval (1 sigma).
    pub fn range_68(&self) -> (f32, f32) {
        self.range(1.0)
    }

    /// Combine with another scouted value (e.g., from different scouts).
    pub fn combine(&self, other: &ScoutedValue<f32>) -> ScoutedValue<f32> {
        let p1 = 1.0 / (self.sigma * self.sigma);
        let p2 = 1.0 / (other.sigma * other.sigma);
        let total_p = p1 + p2;

        let combined_mean = (p1 * self.estimate + p2 * other.estimate) / total_p;
        let combined_sigma = (1.0 / total_p).sqrt();

        ScoutedValue {
            estimate: combined_mean,
            sigma: combined_sigma,
            confidence: calculate_confidence(combined_sigma),
            sample_count: self.sample_count + other.sample_count,
            last_observed: Some(SystemTime::now()),
            freshness: self.freshness.max(other.freshness),
        }
    }
}

// ============================================================================
// Bayesian Update for u8 (Player Attributes)
// ============================================================================

impl ScoutedValue<u8> {
    /// Bayesian update with a new observation for integer values.
    ///
    /// Internally uses f32 for calculation, then rounds to u8.
    pub fn update_with_observation(&mut self, observation: u8, observation_sigma: f32) {
        let prior_precision = 1.0 / (self.sigma * self.sigma);
        let obs_precision = 1.0 / (observation_sigma * observation_sigma);
        let posterior_precision = prior_precision + obs_precision;

        let posterior_mean = (prior_precision * self.estimate as f32
            + obs_precision * observation as f32)
            / posterior_precision;

        let posterior_sigma = (1.0 / posterior_precision).sqrt();

        self.estimate = posterior_mean.round().clamp(0.0, 99.0) as u8;
        self.sigma = posterior_sigma;
        self.confidence = calculate_confidence(posterior_sigma);
        self.sample_count += 1;
        self.last_observed = Some(SystemTime::now());
        self.freshness = 1.0;
    }

    /// Get the uncertainty range for integer stats.
    pub fn range(&self, z_score: f32) -> (u8, u8) {
        let margin = (self.sigma * z_score) as i32;
        let lower = (self.estimate as i32 - margin).max(0) as u8;
        let upper = (self.estimate as i32 + margin).min(99) as u8;
        (lower, upper)
    }

    /// 95% confidence interval.
    pub fn range_95(&self) -> (u8, u8) {
        self.range(1.96)
    }

    /// 68% confidence interval.
    pub fn range_68(&self) -> (u8, u8) {
        self.range(1.0)
    }

    /// Combine with another scouted value.
    pub fn combine(&self, other: &ScoutedValue<u8>) -> ScoutedValue<u8> {
        let p1 = 1.0 / (self.sigma * self.sigma);
        let p2 = 1.0 / (other.sigma * other.sigma);
        let total_p = p1 + p2;

        let combined_mean = (p1 * self.estimate as f32 + p2 * other.estimate as f32) / total_p;
        let combined_sigma = (1.0 / total_p).sqrt();

        ScoutedValue {
            estimate: combined_mean.round().clamp(0.0, 99.0) as u8,
            sigma: combined_sigma,
            confidence: calculate_confidence(combined_sigma),
            sample_count: self.sample_count + other.sample_count,
            last_observed: Some(SystemTime::now()),
            freshness: self.freshness.max(other.freshness),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate confidence from sigma (higher sigma = lower confidence).
fn calculate_confidence(sigma: f32) -> f32 {
    // Confidence approaches 1.0 as sigma approaches 0
    // At sigma=10, confidence ≈ 0.5
    // At sigma=1, confidence ≈ 0.9
    1.0 / (1.0 + sigma * 0.1)
}

/// Calculate scout uncertainty based on various factors.
///
/// # Arguments
/// * `base_sigma` - Base uncertainty for the stat type
/// * `sample_count` - Number of observations
/// * `scout_level` - Scout's expertise level
/// * `quality_factor` - Data quality (1.0 = perfect)
/// * `disguise_factor` - How well the subject hides true values
pub fn calculate_scout_uncertainty(
    base_sigma: f32,
    sample_count: u32,
    scout_level: ScoutLevel,
    quality_factor: f32,
    disguise_factor: f32,
) -> f32 {
    // Reduce uncertainty with more samples (sqrt law)
    let sample_reduction = 1.0 / (sample_count as f32).sqrt().max(1.0);

    // Apply scout level multiplier
    let level_mult = scout_level.uncertainty_mult();

    // Quality reduces uncertainty, disguise increases it
    let adjusted = base_sigma * sample_reduction * level_mult * quality_factor * disguise_factor;

    // Minimum uncertainty floor
    adjusted.max(0.5)
}

/// Base sigma values for different attribute categories.
#[derive(Debug, Clone, Copy)]
pub enum AttributeCategory {
    /// Physical attributes (Pace, Stamina) - easier to observe
    Physical,
    /// Technical attributes (Passing, Shooting) - moderately observable
    Technical,
    /// Mental attributes (Composure, Vision) - harder to observe
    Mental,
    /// Hidden attributes (Consistency, Big Games) - very hard to observe
    Hidden,
}

impl AttributeCategory {
    /// Base sigma for this category.
    pub fn base_sigma(&self) -> f32 {
        match self {
            AttributeCategory::Physical => 5.0,
            AttributeCategory::Technical => 8.0,
            AttributeCategory::Mental => 12.0,
            AttributeCategory::Hidden => 15.0,
        }
    }

    /// Minimum observations needed for reliable estimate.
    pub fn min_observations(&self) -> u32 {
        match self {
            AttributeCategory::Physical => 1,
            AttributeCategory::Technical => 2,
            AttributeCategory::Mental => 3,
            AttributeCategory::Hidden => 5,
        }
    }
}

// ============================================================================
// Qualitative Grade
// ============================================================================

/// Qualitative grade for low-detail reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualitativeGrade {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
    Elite,
}

impl QualitativeGrade {
    /// Convert a numeric value to qualitative grade.
    pub fn from_value(value: f32, max_value: f32) -> Self {
        let ratio = value / max_value;
        if ratio >= 0.95 {
            QualitativeGrade::Elite
        } else if ratio >= 0.80 {
            QualitativeGrade::VeryHigh
        } else if ratio >= 0.65 {
            QualitativeGrade::High
        } else if ratio >= 0.45 {
            QualitativeGrade::Medium
        } else if ratio >= 0.25 {
            QualitativeGrade::Low
        } else {
            QualitativeGrade::VeryLow
        }
    }

    /// Convert from u8 attribute value.
    pub fn from_attribute(value: u8) -> Self {
        Self::from_value(value as f32, 99.0)
    }

    /// Display name for UI.
    pub fn display(&self) -> &'static str {
        match self {
            QualitativeGrade::VeryLow => "Very Low",
            QualitativeGrade::Low => "Low",
            QualitativeGrade::Medium => "Medium",
            QualitativeGrade::High => "High",
            QualitativeGrade::VeryHigh => "Very High",
            QualitativeGrade::Elite => "Elite",
        }
    }

    /// Short code for compact display.
    pub fn code(&self) -> &'static str {
        match self {
            QualitativeGrade::VeryLow => "F",
            QualitativeGrade::Low => "D",
            QualitativeGrade::Medium => "C",
            QualitativeGrade::High => "B",
            QualitativeGrade::VeryHigh => "A",
            QualitativeGrade::Elite => "S",
        }
    }
}

// ============================================================================
// Player Scouting Model
// ============================================================================

/// Scouted player attributes with uncertainty.
#[derive(Debug, Clone, Default)]
pub struct ScoutedPlayerAttributes {
    // Physical
    pub pace: ScoutedValue<u8>,
    pub acceleration: ScoutedValue<u8>,
    pub stamina: ScoutedValue<u8>,
    pub strength: ScoutedValue<u8>,
    pub jumping: ScoutedValue<u8>,

    // Technical
    pub passing: ScoutedValue<u8>,
    pub shooting: ScoutedValue<u8>,
    pub dribbling: ScoutedValue<u8>,
    pub first_touch: ScoutedValue<u8>,
    pub crossing: ScoutedValue<u8>,
    pub heading: ScoutedValue<u8>,
    pub tackling: ScoutedValue<u8>,

    // Mental
    pub composure: ScoutedValue<u8>,
    pub vision: ScoutedValue<u8>,
    pub positioning: ScoutedValue<u8>,
    pub decisions: ScoutedValue<u8>,
    pub work_rate: ScoutedValue<u8>,

    // Hidden
    pub consistency: ScoutedValue<u8>,
    pub big_games: ScoutedValue<u8>,
    pub injury_prone: ScoutedValue<u8>,
}

impl ScoutedPlayerAttributes {
    /// Create from actual player attributes with scout-level uncertainty.
    pub fn from_actual(
        actual: &PlayerAttributeSnapshot,
        scout_level: ScoutLevel,
        sample_count: u32,
    ) -> Self {
        let physical_sigma = calculate_scout_uncertainty(
            AttributeCategory::Physical.base_sigma(),
            sample_count,
            scout_level,
            1.0,
            1.0,
        );
        let technical_sigma = calculate_scout_uncertainty(
            AttributeCategory::Technical.base_sigma(),
            sample_count,
            scout_level,
            1.0,
            1.0,
        );
        let mental_sigma = calculate_scout_uncertainty(
            AttributeCategory::Mental.base_sigma(),
            sample_count,
            scout_level,
            1.0,
            1.0,
        );
        let hidden_sigma = calculate_scout_uncertainty(
            AttributeCategory::Hidden.base_sigma(),
            sample_count,
            scout_level,
            1.0,
            1.0,
        );

        Self {
            // Physical
            pace: ScoutedValue::with_samples(actual.pace, physical_sigma, sample_count),
            acceleration: ScoutedValue::with_samples(actual.acceleration, physical_sigma, sample_count),
            stamina: ScoutedValue::with_samples(actual.stamina, physical_sigma, sample_count),
            strength: ScoutedValue::with_samples(actual.strength, physical_sigma, sample_count),
            jumping: ScoutedValue::with_samples(actual.jumping, physical_sigma, sample_count),

            // Technical
            passing: ScoutedValue::with_samples(actual.passing, technical_sigma, sample_count),
            shooting: ScoutedValue::with_samples(actual.shooting, technical_sigma, sample_count),
            dribbling: ScoutedValue::with_samples(actual.dribbling, technical_sigma, sample_count),
            first_touch: ScoutedValue::with_samples(actual.first_touch, technical_sigma, sample_count),
            crossing: ScoutedValue::with_samples(actual.crossing, technical_sigma, sample_count),
            heading: ScoutedValue::with_samples(actual.heading, technical_sigma, sample_count),
            tackling: ScoutedValue::with_samples(actual.tackling, technical_sigma, sample_count),

            // Mental
            composure: ScoutedValue::with_samples(actual.composure, mental_sigma, sample_count),
            vision: ScoutedValue::with_samples(actual.vision, mental_sigma, sample_count),
            positioning: ScoutedValue::with_samples(actual.positioning, mental_sigma, sample_count),
            decisions: ScoutedValue::with_samples(actual.decisions, mental_sigma, sample_count),
            work_rate: ScoutedValue::with_samples(actual.work_rate, mental_sigma, sample_count),

            // Hidden (only visible at higher scout levels)
            consistency: ScoutedValue::with_samples(actual.consistency, hidden_sigma, sample_count),
            big_games: ScoutedValue::with_samples(actual.big_games, hidden_sigma, sample_count),
            injury_prone: ScoutedValue::with_samples(actual.injury_prone, hidden_sigma, sample_count),
        }
    }

    /// Calculate overall rating with uncertainty.
    pub fn overall(&self) -> ScoutedValue<u8> {
        // Weighted average of key attributes
        let weights = [
            (self.pace.estimate as f32, 0.10),
            (self.passing.estimate as f32, 0.15),
            (self.shooting.estimate as f32, 0.15),
            (self.dribbling.estimate as f32, 0.15),
            (self.tackling.estimate as f32, 0.10),
            (self.composure.estimate as f32, 0.10),
            (self.positioning.estimate as f32, 0.10),
            (self.stamina.estimate as f32, 0.08),
            (self.strength.estimate as f32, 0.07),
        ];

        let sum: f32 = weights.iter().map(|(v, w)| v * w).sum();
        let overall = sum.round().clamp(0.0, 99.0) as u8;

        // Combine uncertainties (rough approximation)
        let avg_sigma = (self.pace.sigma + self.passing.sigma + self.shooting.sigma
            + self.dribbling.sigma + self.composure.sigma) / 5.0;

        ScoutedValue::with_samples(overall, avg_sigma, self.pace.sample_count)
    }

    /// Get attributes visible at given scout level.
    pub fn visible_at_level(&self, level: ScoutLevel) -> Vec<(&'static str, &ScoutedValue<u8>)> {
        let mut attrs = Vec::new();

        // Always visible
        attrs.push(("Overall", &self.pace)); // Placeholder, use overall() in practice

        if level >= ScoutLevel::Basic {
            attrs.push(("Pace", &self.pace));
            attrs.push(("Shooting", &self.shooting));
            attrs.push(("Passing", &self.passing));
        }

        if level >= ScoutLevel::Report {
            attrs.push(("Dribbling", &self.dribbling));
            attrs.push(("Tackling", &self.tackling));
            attrs.push(("Stamina", &self.stamina));
        }

        if level >= ScoutLevel::Detail {
            attrs.push(("Vision", &self.vision));
            attrs.push(("Composure", &self.composure));
            attrs.push(("Positioning", &self.positioning));
            attrs.push(("Strength", &self.strength));
        }

        if level >= ScoutLevel::Elite {
            attrs.push(("Consistency", &self.consistency));
            attrs.push(("Big Games", &self.big_games));
            attrs.push(("Injury Prone", &self.injury_prone));
        }

        attrs
    }
}

/// Snapshot of actual player attributes for scouting.
#[derive(Debug, Clone, Default)]
pub struct PlayerAttributeSnapshot {
    pub pace: u8,
    pub acceleration: u8,
    pub stamina: u8,
    pub strength: u8,
    pub jumping: u8,
    pub passing: u8,
    pub shooting: u8,
    pub dribbling: u8,
    pub first_touch: u8,
    pub crossing: u8,
    pub heading: u8,
    pub tackling: u8,
    pub composure: u8,
    pub vision: u8,
    pub positioning: u8,
    pub decisions: u8,
    pub work_rate: u8,
    pub consistency: u8,
    pub big_games: u8,
    pub injury_prone: u8,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scout_level_ordering() {
        assert!(ScoutLevel::Elite > ScoutLevel::Rumor);
        assert!(ScoutLevel::Detail > ScoutLevel::Basic);
    }

    #[test]
    fn test_scouted_value_range() {
        let value = ScoutedValue::new(75.0f32, 5.0);
        let (lower, upper) = value.range_68();
        assert!((lower - 70.0).abs() < 0.1);
        assert!((upper - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_uncertainty() {
        // More samples = less uncertainty
        let sigma1 = calculate_scout_uncertainty(10.0, 1, ScoutLevel::Basic, 1.0, 1.0);
        let sigma10 = calculate_scout_uncertainty(10.0, 10, ScoutLevel::Basic, 1.0, 1.0);
        assert!(sigma10 < sigma1);

        // Higher scout level = less uncertainty
        let rumor = calculate_scout_uncertainty(10.0, 5, ScoutLevel::Rumor, 1.0, 1.0);
        let elite = calculate_scout_uncertainty(10.0, 5, ScoutLevel::Elite, 1.0, 1.0);
        assert!(elite < rumor);
    }

    #[test]
    fn test_qualitative_grade() {
        assert_eq!(QualitativeGrade::from_value(95.0, 100.0), QualitativeGrade::Elite);
        assert_eq!(QualitativeGrade::from_value(50.0, 100.0), QualitativeGrade::Medium);
        assert_eq!(QualitativeGrade::from_value(20.0, 100.0), QualitativeGrade::VeryLow);
    }

    #[test]
    fn test_bayesian_update_f32() {
        // Start with prior
        let mut value = ScoutedValue::new(70.0f32, 10.0);
        let initial_sigma = value.sigma;

        // Update with observation
        value.update_with_observation(80.0, 10.0);

        // Posterior should be between prior and observation
        assert!(value.estimate > 70.0 && value.estimate < 80.0);
        // Uncertainty should decrease
        assert!(value.sigma < initial_sigma);
        // Sample count should increase
        assert_eq!(value.sample_count, 2);
    }

    #[test]
    fn test_bayesian_update_u8() {
        let mut value = ScoutedValue::new(70u8, 10.0);
        let initial_sigma = value.sigma;

        value.update_with_observation(80, 10.0);

        assert!(value.estimate > 70 && value.estimate < 80);
        assert!(value.sigma < initial_sigma);
    }

    #[test]
    fn test_combine_scouted_values() {
        let v1 = ScoutedValue::new(70.0f32, 10.0);
        let v2 = ScoutedValue::new(80.0f32, 10.0);

        let combined = v1.combine(&v2);

        // Combined should be midpoint (equal uncertainties)
        assert!((combined.estimate - 75.0).abs() < 0.5);
        // Combined uncertainty should be lower
        assert!(combined.sigma < 10.0);
    }

    #[test]
    fn test_scouted_player_attributes() {
        let actual = PlayerAttributeSnapshot {
            pace: 80,
            acceleration: 78,
            stamina: 75,
            strength: 70,
            jumping: 72,
            passing: 82,
            shooting: 85,
            dribbling: 80,
            first_touch: 79,
            crossing: 76,
            heading: 65,
            tackling: 45,
            composure: 77,
            vision: 78,
            positioning: 76,
            decisions: 74,
            work_rate: 80,
            consistency: 75,
            big_games: 72,
            injury_prone: 30,
        };

        let scouted = ScoutedPlayerAttributes::from_actual(&actual, ScoutLevel::Report, 3);

        // Check that estimates match actual (with some noise in sigma)
        assert_eq!(scouted.pace.estimate, 80);
        assert_eq!(scouted.shooting.estimate, 85);

        // Physical should have lower sigma than mental
        assert!(scouted.pace.sigma < scouted.composure.sigma);

        // Overall should be reasonable
        let overall = scouted.overall();
        assert!(overall.estimate > 60 && overall.estimate < 90);
    }

    #[test]
    fn test_visible_attributes_by_level() {
        let actual = PlayerAttributeSnapshot {
            pace: 80,
            shooting: 85,
            passing: 82,
            dribbling: 80,
            tackling: 45,
            stamina: 75,
            vision: 78,
            composure: 77,
            positioning: 76,
            consistency: 75,
            ..Default::default()
        };

        let scouted = ScoutedPlayerAttributes::from_actual(&actual, ScoutLevel::Elite, 5);

        let rumor_attrs = scouted.visible_at_level(ScoutLevel::Rumor);
        let elite_attrs = scouted.visible_at_level(ScoutLevel::Elite);

        assert!(elite_attrs.len() > rumor_attrs.len());
        // Elite should see hidden attributes
        assert!(elite_attrs.iter().any(|(name, _)| *name == "Consistency"));
    }

    #[test]
    fn test_confidence_calculation() {
        // Low sigma = high confidence
        let high_conf = calculate_confidence(1.0);
        let low_conf = calculate_confidence(20.0);

        assert!(high_conf > low_conf);
        assert!(high_conf > 0.8);
        assert!(low_conf < 0.5);
    }
}
