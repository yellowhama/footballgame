// ============================================================================
// P2.2-A: Probability Distribution Validation
// ============================================================================
//
// Contract: Outcome probabilities must sum to 1.0 (within tolerance)
//
// Purpose:
// - Detect weight calculation bugs before sampling
// - Prevent invalid probability distributions
// - Catch normalization errors
//
// Integration Points:
// - select_outcome_softmax() - validate before selection
// - WeightComposer::normalize() - validate after normalization
//
// Created: 2025-12-23 (P2.2-A)

use serde::{Deserialize, Serialize};

/// Probability distribution validator
///
/// Validates that probability distributions sum to 1.0 within a tolerance
/// and that all probabilities are non-negative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityValidator {
    /// Tolerance for deviation from 1.0
    /// Default: 0.001 (0.1% deviation allowed)
    tolerance: f64,
}

impl ProbabilityValidator {
    /// Create a new validator with default tolerance (0.001)
    pub fn new() -> Self {
        Self { tolerance: 0.001 }
    }

    /// Create a validator with custom tolerance
    ///
    /// # Arguments
    /// * `tolerance` - Maximum allowed deviation from 1.0
    ///
    /// # Examples
    /// ```
    /// use of_core::engine::match_sim::probability_validator::ProbabilityValidator;
    ///
    /// let strict = ProbabilityValidator::with_tolerance(0.0001);  // 0.01% tolerance
    /// let lenient = ProbabilityValidator::with_tolerance(0.01);   // 1% tolerance
    /// ```
    pub fn with_tolerance(tolerance: f64) -> Self {
        Self { tolerance }
    }

    /// Validate a probability distribution
    ///
    /// # Arguments
    /// * `probabilities` - Slice of probabilities to validate
    ///
    /// # Returns
    /// * `Ok(())` if distribution is valid
    /// * `Err(String)` with error message if invalid
    ///
    /// # Validation Rules
    /// 1. Distribution must not be empty
    /// 2. All probabilities must be non-negative (>= 0.0)
    /// 3. Sum must be within tolerance of 1.0
    ///
    /// # Examples
    /// ```
    /// use of_core::engine::match_sim::probability_validator::ProbabilityValidator;
    ///
    /// let validator = ProbabilityValidator::new();
    ///
    /// // Valid distribution
    /// assert!(validator.validate_distribution(&[0.25, 0.50, 0.25]).is_ok());
    ///
    /// // Invalid: sum != 1.0
    /// assert!(validator.validate_distribution(&[0.25, 0.50, 0.30]).is_err());
    ///
    /// // Invalid: negative probability
    /// assert!(validator.validate_distribution(&[0.25, -0.10, 0.85]).is_err());
    /// ```
    pub fn validate_distribution(&self, probabilities: &[f64]) -> Result<(), String> {
        // Check for empty distribution
        if probabilities.is_empty() {
            return Err("Empty probability distribution".to_string());
        }

        // Check for negative probabilities
        if let Some(neg) = probabilities.iter().find(|&&p| p < 0.0) {
            return Err(format!("Negative probability: {}", neg));
        }

        // Check sum
        let sum: f64 = probabilities.iter().sum();
        let deviation = (sum - 1.0).abs();

        if deviation > self.tolerance {
            return Err(format!(
                "Probability sum {:.6} deviates from 1.0 by {:.6} (tolerance: {:.6})",
                sum, deviation, self.tolerance
            ));
        }

        Ok(())
    }

    /// Validate and return normalized distribution if sum is close enough
    ///
    /// This is a convenience method that validates and normalizes in one step.
    /// Use this when you want to be lenient about small deviations.
    ///
    /// # Arguments
    /// * `probabilities` - Slice of probabilities to validate
    ///
    /// # Returns
    /// * `Ok(Vec<f64>)` - Normalized distribution
    /// * `Err(String)` - If distribution is invalid (negative, empty, or too far from 1.0)
    pub fn validate_and_normalize(&self, probabilities: &[f64]) -> Result<Vec<f64>, String> {
        if probabilities.is_empty() {
            return Err("Empty probability distribution".to_string());
        }

        if let Some(neg) = probabilities.iter().find(|&&p| p < 0.0) {
            return Err(format!("Negative probability: {}", neg));
        }

        let sum: f64 = probabilities.iter().sum();

        // Check for zero sum first (cannot normalize)
        if sum == 0.0 {
            return Err("Cannot normalize: sum is zero".to_string());
        }

        // Check if sum is too far from 1.0 (using a more lenient threshold for normalization)
        let max_deviation_for_normalization = 0.1; // 10% deviation allowed for normalization
        if (sum - 1.0).abs() > max_deviation_for_normalization {
            return Err(format!(
                "Probability sum {:.6} too far from 1.0 for normalization (max: {:.1})",
                sum, max_deviation_for_normalization
            ));
        }

        Ok(probabilities.iter().map(|&p| p / sum).collect())
    }

    /// Get the current tolerance value
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }
}

impl Default for ProbabilityValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // P2.2-A: Probability Distribution Tests
    // ========================================================================

    #[test]
    fn test_valid_distribution() {
        let validator = ProbabilityValidator::new();

        // Exact sum = 1.0
        assert!(validator.validate_distribution(&[0.25, 0.50, 0.25]).is_ok());

        // Within tolerance
        assert!(validator.validate_distribution(&[0.3333, 0.3333, 0.3334]).is_ok());

        // Binary distribution
        assert!(validator.validate_distribution(&[0.6, 0.4]).is_ok());

        // Single outcome
        assert!(validator.validate_distribution(&[1.0]).is_ok());
    }

    #[test]
    fn test_distribution_sum_not_one() {
        let validator = ProbabilityValidator::new();

        // Sum = 1.05 (too high)
        let result = validator.validate_distribution(&[0.25, 0.50, 0.30]);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("deviates from 1.0"));
        assert!(err_msg.contains("1.05"));

        // Sum = 0.95 (too low)
        let result = validator.validate_distribution(&[0.25, 0.50, 0.20]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("deviates from 1.0"));
    }

    #[test]
    fn test_negative_probability() {
        let validator = ProbabilityValidator::new();

        // Negative probability
        let result = validator.validate_distribution(&[0.25, -0.10, 0.85]);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Negative probability"));
        assert!(err_msg.contains("-0.1"));
    }

    #[test]
    fn test_empty_distribution() {
        let validator = ProbabilityValidator::new();

        let result = validator.validate_distribution(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty probability distribution"));
    }

    #[test]
    fn test_custom_tolerance() {
        // Strict validator (0.005% tolerance = 0.00005)
        let strict = ProbabilityValidator::with_tolerance(0.00005);

        // This would pass with default tolerance but fails with strict
        // sum = 0.9999, deviation = 0.0001, which is > 0.00005
        let slightly_off = [0.3333, 0.3333, 0.3333]; // sum = 0.9999
        assert!(strict.validate_distribution(&slightly_off).is_err());

        // Lenient validator (2% tolerance)
        let lenient = ProbabilityValidator::with_tolerance(0.02);

        // This would fail with default but passes with lenient
        // sum = 0.99, deviation = 0.01, which is < 0.02
        let more_off = [0.33, 0.33, 0.33]; // sum = 0.99
        assert!(lenient.validate_distribution(&more_off).is_ok());
    }

    #[test]
    fn test_validate_and_normalize_valid() {
        let validator = ProbabilityValidator::new();

        // Slightly off distribution (within 10% of 1.0)
        // Using values that sum reliably in floating point: 0.31 + 0.31 + 0.31 = 0.93
        let probs = vec![0.31, 0.31, 0.31]; // sum = 0.93, deviation = 7% < 10%
        let normalized = validator.validate_and_normalize(&probs).unwrap();

        // Check that it's normalized
        let sum: f64 = normalized.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);

        // Check proportions are preserved (each should be 1/3)
        assert!((normalized[0] - 1.0 / 3.0).abs() < 1e-10);
        assert!((normalized[1] - 1.0 / 3.0).abs() < 1e-10);
        assert!((normalized[2] - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_validate_and_normalize_negative() {
        let validator = ProbabilityValidator::new();

        let result = validator.validate_and_normalize(&[0.5, -0.2, 0.7]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Negative probability"));
    }

    #[test]
    fn test_validate_and_normalize_too_far() {
        let validator = ProbabilityValidator::new();

        // Sum too far from 1.0 (2.0 is beyond 10% threshold)
        let result = validator.validate_and_normalize(&[1.0, 0.5, 0.5]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too far from 1.0"));
    }

    #[test]
    fn test_validate_and_normalize_zero_sum() {
        let validator = ProbabilityValidator::new();

        let result = validator.validate_and_normalize(&[0.0, 0.0, 0.0]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sum is zero"));
    }

    #[test]
    fn test_default_tolerance() {
        let validator = ProbabilityValidator::default();
        assert_eq!(validator.tolerance(), 0.001);
    }

    #[test]
    fn test_tolerance_boundary() {
        let validator = ProbabilityValidator::new();

        // Exactly at tolerance boundary (1.001)
        let at_boundary = [0.3334, 0.3333, 0.3334]; // sum = 1.0001
        let result = validator.validate_distribution(&at_boundary);
        // Should pass (within tolerance)
        assert!(result.is_ok());

        // Just beyond tolerance (1.002)
        let beyond_boundary = [0.334, 0.334, 0.334]; // sum = 1.002
        let result = validator.validate_distribution(&beyond_boundary);
        // Should fail (beyond tolerance)
        assert!(result.is_err());
    }

    #[test]
    fn test_all_zero_except_one() {
        let validator = ProbabilityValidator::new();

        // All probability on one outcome
        assert!(validator.validate_distribution(&[0.0, 1.0, 0.0]).is_ok());
        assert!(validator.validate_distribution(&[0.0, 0.0, 0.0, 1.0]).is_ok());
    }

    #[test]
    fn test_many_outcomes() {
        let validator = ProbabilityValidator::new();

        // 10 outcomes, equal probability
        let equal_ten = vec![0.1; 10];
        assert!(validator.validate_distribution(&equal_ten).is_ok());

        // 100 outcomes
        let equal_hundred = vec![0.01; 100];
        assert!(validator.validate_distribution(&equal_hundred).is_ok());
    }
}
