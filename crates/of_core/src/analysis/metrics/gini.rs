//! # Gini Coefficient Module
//!
//! Measures inequality in distributions - useful for detecting "monopoly" patterns
//! in touch/pass distributions.
//!
//! ## Background
//!
//! The Gini coefficient is a standard measure of inequality:
//! - 0.0 = Perfect equality (everyone has equal share)
//! - 1.0 = Perfect inequality (one person has everything)
//!
//! For football analysis:
//! - Low Gini (< 0.30): Well-distributed play
//! - Medium Gini (0.30-0.40): Some concentration
//! - High Gini (> 0.40): "Monopoly" pattern (one-two players dominate)
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: PASS_NETWORK_ENTROPY_ANALYSIS.md

/// Calculate the Gini coefficient from a slice of values.
///
/// Uses the sorted-values formula:
/// G = (2 * Σ(i * x_i)) / (n * Σx_i) - (n+1)/n
///
/// # Arguments
/// * `values` - Slice of non-negative integer values
///
/// # Returns
/// * `Some(gini)` - Gini coefficient in [0.0, 1.0]
/// * `None` - If input is empty
///
/// # Examples
/// ```
/// use of_core::analysis::metrics::gini::gini_coefficient;
///
/// // Uniform distribution → Gini ≈ 0
/// let uniform = [10u32, 10, 10, 10, 10];
/// assert!(gini_coefficient(&uniform).unwrap() < 0.05);
///
/// // Complete monopoly → Gini ≈ 0.8 (for n=5)
/// let monopoly = [100u32, 0, 0, 0, 0];
/// assert!(gini_coefficient(&monopoly).unwrap() > 0.75);
/// ```
pub fn gini_coefficient(values: &[u32]) -> Option<f64> {
    let n = values.len();
    if n == 0 {
        return None;
    }

    let sum: u64 = values.iter().map(|&v| v as u64).sum();
    if sum == 0 {
        // All zeros - consider this as perfect equality
        return Some(0.0);
    }

    // Sort values in ascending order
    let mut sorted: Vec<u64> = values.iter().map(|&v| v as u64).collect();
    sorted.sort_unstable();

    // Calculate using the formula:
    // G = (2 * Σ(i * x_i)) / (n * Σx_i) - (n+1)/n
    // where i is 1-indexed rank
    let mut weighted_sum: u64 = 0;
    for (i, &val) in sorted.iter().enumerate() {
        // i+1 because formula uses 1-indexed ranks
        weighted_sum += (i as u64 + 1) * val;
    }

    let n64 = n as f64;
    let sum_f = sum as f64;
    let weighted_sum_f = weighted_sum as f64;

    let gini = (2.0 * weighted_sum_f) / (n64 * sum_f) - (n64 + 1.0) / n64;

    // Clamp to [0, 1] to handle floating point errors
    Some(gini.clamp(0.0, 1.0))
}

/// Calculate Gini from f32 values (for when input is already float)
pub fn gini_coefficient_f32(values: &[f32]) -> Option<f64> {
    let n = values.len();
    if n == 0 {
        return None;
    }

    let sum: f64 = values.iter().map(|&v| v as f64).sum();
    if sum <= 0.0 {
        return Some(0.0);
    }

    // Sort values in ascending order
    let mut sorted: Vec<f64> = values.iter().map(|&v| v as f64).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut weighted_sum: f64 = 0.0;
    for (i, &val) in sorted.iter().enumerate() {
        weighted_sum += (i as f64 + 1.0) * val;
    }

    let n64 = n as f64;
    let gini = (2.0 * weighted_sum) / (n64 * sum) - (n64 + 1.0) / n64;

    Some(gini.clamp(0.0, 1.0))
}

/// Aggregated Gini metrics for a team's distribution patterns.
#[derive(Debug, Clone, Default)]
pub struct GiniMetrics {
    /// Gini coefficient for touch distribution (0=equal, 1=monopoly)
    pub touch_gini: f64,
    /// Gini coefficient for pass sending distribution
    pub pass_sent_gini: f64,
    /// Gini coefficient for pass receiving distribution
    pub pass_recv_gini: f64,
    /// Gini coefficient for progressive pass distribution
    pub progressive_gini: f64,
}

impl GiniMetrics {
    /// Create GiniMetrics from player statistics arrays.
    ///
    /// # Arguments
    /// * `touches` - Touch counts per player [11]
    /// * `passes_sent` - Passes sent per player [11]
    /// * `passes_received` - Passes received per player [11]
    /// * `progressive_sent` - Progressive passes sent per player [11]
    pub fn from_player_stats(
        touches: &[u32; 11],
        passes_sent: &[u32; 11],
        passes_received: &[u32; 11],
        progressive_sent: &[u32; 11],
    ) -> Self {
        Self {
            touch_gini: gini_coefficient(touches).unwrap_or(0.0),
            pass_sent_gini: gini_coefficient(passes_sent).unwrap_or(0.0),
            pass_recv_gini: gini_coefficient(passes_received).unwrap_or(0.0),
            progressive_gini: gini_coefficient(progressive_sent).unwrap_or(0.0),
        }
    }

    /// Check if any metric indicates a monopoly pattern.
    ///
    /// # Arguments
    /// * `threshold` - Gini value above which is considered monopoly (e.g., 0.40)
    pub fn has_monopoly(&self, threshold: f64) -> bool {
        self.touch_gini >= threshold
            || self.pass_sent_gini >= threshold
            || self.pass_recv_gini >= threshold
    }

    /// Get the maximum Gini value across all metrics.
    pub fn max_gini(&self) -> f64 {
        self.touch_gini
            .max(self.pass_sent_gini)
            .max(self.pass_recv_gini)
            .max(self.progressive_gini)
    }
}

/// QA flags related to Gini coefficients.
#[derive(Debug, Clone)]
pub enum GiniFlag {
    /// Touch distribution is too concentrated
    TouchMonopoly { value: f64, threshold: f64 },
    /// Pass receiving is too concentrated
    ReceiverMonopoly { value: f64, threshold: f64 },
    /// Pass sending is too concentrated
    SenderMonopoly { value: f64, threshold: f64 },
    /// Progressive passes too concentrated
    ProgressiveMonopoly { value: f64, threshold: f64 },
}

impl GiniFlag {
    /// Human-readable description of the flag.
    pub fn description(&self) -> String {
        match self {
            GiniFlag::TouchMonopoly { value, threshold } => {
                format!(
                    "Touch distribution too concentrated (Gini {:.2} >= {:.2})",
                    value, threshold
                )
            }
            GiniFlag::ReceiverMonopoly { value, threshold } => {
                format!(
                    "Pass receiving too concentrated (Gini {:.2} >= {:.2})",
                    value, threshold
                )
            }
            GiniFlag::SenderMonopoly { value, threshold } => {
                format!(
                    "Pass sending too concentrated (Gini {:.2} >= {:.2})",
                    value, threshold
                )
            }
            GiniFlag::ProgressiveMonopoly { value, threshold } => {
                format!(
                    "Progressive passes too concentrated (Gini {:.2} >= {:.2})",
                    value, threshold
                )
            }
        }
    }
}

/// Check Gini metrics against QA thresholds.
///
/// # Arguments
/// * `metrics` - GiniMetrics to check
/// * `touch_threshold` - Max acceptable touch Gini (default: 0.40)
/// * `recv_threshold` - Max acceptable receiver Gini (default: 0.42)
pub fn check_gini_flags(
    metrics: &GiniMetrics,
    touch_threshold: f64,
    recv_threshold: f64,
) -> Vec<GiniFlag> {
    let mut flags = vec![];

    if metrics.touch_gini >= touch_threshold {
        flags.push(GiniFlag::TouchMonopoly {
            value: metrics.touch_gini,
            threshold: touch_threshold,
        });
    }

    if metrics.pass_recv_gini >= recv_threshold {
        flags.push(GiniFlag::ReceiverMonopoly {
            value: metrics.pass_recv_gini,
            threshold: recv_threshold,
        });
    }

    if metrics.pass_sent_gini >= recv_threshold {
        flags.push(GiniFlag::SenderMonopoly {
            value: metrics.pass_sent_gini,
            threshold: recv_threshold,
        });
    }

    if metrics.progressive_gini >= 0.50 {
        flags.push(GiniFlag::ProgressiveMonopoly {
            value: metrics.progressive_gini,
            threshold: 0.50,
        });
    }

    flags
}

/// Reference Gini ranges from real football (EPL 2024-25 estimates)
pub mod reference {
    /// Normal range for touch Gini
    pub const TOUCH_NORMAL_LOW: f64 = 0.15;
    pub const TOUCH_NORMAL_HIGH: f64 = 0.35;
    /// Monopoly suspicion threshold
    pub const TOUCH_MONOPOLY: f64 = 0.45;

    /// Normal range for pass receiver Gini
    pub const RECV_NORMAL_LOW: f64 = 0.18;
    pub const RECV_NORMAL_HIGH: f64 = 0.38;
    /// Monopoly suspicion threshold
    pub const RECV_MONOPOLY: f64 = 0.48;

    /// Normal range for pass sender Gini
    pub const SENT_NORMAL_LOW: f64 = 0.20;
    pub const SENT_NORMAL_HIGH: f64 = 0.40;
    /// Monopoly suspicion threshold
    pub const SENT_MONOPOLY: f64 = 0.50;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gini_uniform() {
        // All equal values → Gini should be 0
        let uniform = [10u32, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10];
        let gini = gini_coefficient(&uniform).unwrap();
        assert!(gini < 0.05, "Uniform distribution should have Gini ≈ 0, got {}", gini);
    }

    #[test]
    fn test_gini_monopoly() {
        // One player has everything → Gini should be high
        let monopoly = [100u32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let gini = gini_coefficient(&monopoly).unwrap();
        assert!(
            gini > 0.85,
            "Complete monopoly should have Gini > 0.85, got {}",
            gini
        );
    }

    #[test]
    fn test_gini_moderate() {
        // Skewed but not extreme distribution
        let skewed = [30u32, 20, 15, 10, 8, 6, 5, 3, 2, 1, 0];
        let gini = gini_coefficient(&skewed).unwrap();
        assert!(
            gini > 0.30 && gini < 0.60,
            "Moderate skew should have Gini in 0.30-0.60, got {}",
            gini
        );
    }

    #[test]
    fn test_gini_empty() {
        let empty: [u32; 0] = [];
        assert!(gini_coefficient(&empty).is_none());
    }

    #[test]
    fn test_gini_all_zeros() {
        // All zeros should be treated as perfect equality
        let zeros = [0u32, 0, 0, 0, 0];
        let gini = gini_coefficient(&zeros).unwrap();
        assert_eq!(gini, 0.0);
    }

    #[test]
    fn test_gini_single_element() {
        // Single element is trivially equal to itself
        let single = [42u32];
        let gini = gini_coefficient(&single).unwrap();
        assert_eq!(gini, 0.0);
    }

    #[test]
    fn test_gini_two_elements() {
        // Equal pair
        let equal_pair = [50u32, 50];
        assert!(gini_coefficient(&equal_pair).unwrap() < 0.01);

        // Unequal pair (100, 0) → Gini = 0.5 for n=2
        let unequal = [100u32, 0];
        let gini = gini_coefficient(&unequal).unwrap();
        assert!((gini - 0.5).abs() < 0.01, "Expected 0.5, got {}", gini);
    }

    #[test]
    fn test_gini_metrics() {
        let touches = [20u32, 18, 15, 12, 10, 8, 7, 5, 3, 1, 1];
        let passes_sent = [25u32, 22, 18, 15, 10, 5, 3, 1, 1, 0, 0];
        let passes_recv = [30u32, 25, 20, 10, 8, 4, 2, 1, 0, 0, 0];
        let progressive = [15u32, 10, 8, 5, 3, 2, 1, 1, 0, 0, 0];

        let metrics = GiniMetrics::from_player_stats(&touches, &passes_sent, &passes_recv, &progressive);

        assert!(metrics.touch_gini > 0.0);
        assert!(metrics.pass_sent_gini > 0.0);
        assert!(metrics.pass_recv_gini > 0.0);
        assert!(metrics.progressive_gini > 0.0);

        // Recv should be most concentrated based on the distribution
        assert!(
            metrics.pass_recv_gini > metrics.touch_gini,
            "Receiver Gini should be higher"
        );
    }

    #[test]
    fn test_check_gini_flags() {
        let metrics = GiniMetrics {
            touch_gini: 0.45,    // Above threshold
            pass_sent_gini: 0.35,
            pass_recv_gini: 0.50, // Above threshold
            progressive_gini: 0.30,
        };

        let flags = check_gini_flags(&metrics, 0.40, 0.42);

        assert_eq!(flags.len(), 2, "Should have 2 flags");
        assert!(matches!(flags[0], GiniFlag::TouchMonopoly { .. }));
        assert!(matches!(flags[1], GiniFlag::ReceiverMonopoly { .. }));
    }

    #[test]
    fn test_gini_f32() {
        let values = [10.5f32, 10.5, 10.5, 10.5];
        let gini = gini_coefficient_f32(&values).unwrap();
        assert!(gini < 0.01, "Uniform f32 should have low Gini");
    }
}
