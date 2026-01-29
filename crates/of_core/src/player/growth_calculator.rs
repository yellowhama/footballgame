//! Growth rate calculation engine with quadratic decay
//!
//! Implements T018 specification:
//! - Quadratic decay formula: (1.0 - ca/pa)^2
//! - Age modifiers: 고1 1.2x, 고2 1.0x, 고3 0.8x
//! - Minimum 10% growth when CA >= PA

/// Growth calculation engine with quadratic decay formula
#[derive(Debug)]
pub struct GrowthCalculator;

impl GrowthCalculator {
    /// Calculate growth rate using quadratic decay formula
    /// Formula: (1.0 - progress)^2 where progress = ca/pa
    /// Minimum growth rate of 0.1 (10%) when CA >= PA
    pub fn calculate_growth_rate(ca: u8, pa: u8) -> f32 {
        if pa == 0 {
            return 0.0; // Cannot calculate growth without potential
        }

        let progress = (ca as f32) / (pa as f32);

        if progress >= 1.0 {
            // Player has reached or exceeded potential - minimum growth
            0.1 // 10% minimum
        } else {
            // Apply quadratic decay: (1.0 - progress)^2
            let remaining = 1.0 - progress;
            remaining * remaining
        }
    }

    /// Calculate age modifier based on school year
    /// - 고1 (15.0-16.0): 1.2x bonus (rapid growth phase)
    /// - 고2 (16.0-17.0): 1.0x normal
    /// - 고3 (17.0-18.0): 0.8x penalty (slower growth)
    pub fn age_modifier(age_months: f32) -> f32 {
        match age_months {
            age if age < 16.0 => 1.2,  // 고1 - rapid growth
            age if age < 17.0 => 1.0,  // 고2 - normal growth
            age if age <= 18.0 => 0.8, // 고3 - slower growth
            _ => 0.5,                  // Outside normal high school age - very slow growth
        }
    }

    /// Calculate final growth rate combining base growth and age modifier
    pub fn calculate_final_growth_rate(ca: u8, pa: u8, age_months: f32) -> f32 {
        let base_growth = Self::calculate_growth_rate(ca, pa);
        let age_mult = Self::age_modifier(age_months);

        (base_growth * age_mult).max(0.05) // Minimum 5% growth rate
    }

    /// Calculate potential growth points per training session
    /// Returns the expected attribute increase for intensive training
    pub fn calculate_growth_points(
        ca: u8,
        pa: u8,
        age_months: f32,
        training_intensity: f32,
    ) -> f32 {
        let growth_rate = Self::calculate_final_growth_rate(ca, pa, age_months);

        // Training intensity should be 0.0-1.0
        let intensity_clamped = training_intensity.clamp(0.0, 1.0);

        // Base growth points per session (can be tuned)
        let base_points = 0.5;

        base_points * growth_rate * intensity_clamped
    }

    /// Calculate time to reach potential (in training sessions)
    /// This gives an estimate of how many training sessions needed to max out
    pub fn estimate_sessions_to_potential(ca: u8, pa: u8, age_months: f32) -> Option<u32> {
        if ca >= pa {
            return None; // Already at or above potential
        }

        let points_needed = (pa - ca) as f32;
        let points_per_session = Self::calculate_growth_points(ca, pa, age_months, 1.0); // Max intensity

        if points_per_session <= 0.0 {
            return None; // No growth possible
        }

        Some((points_needed / points_per_session).ceil() as u32)
    }

    /// Get growth curve data for visualization (returns points at different CA/PA ratios)
    pub fn get_growth_curve(pa: u8, age_months: f32) -> Vec<(u8, f32)> {
        let mut curve = Vec::new();

        for ca in 0..=pa {
            let growth_rate = Self::calculate_final_growth_rate(ca, pa, age_months);
            curve.push((ca, growth_rate));
        }

        curve
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_decay_formula() {
        // Test progress = 0 (CA = 0, PA = 100) -> growth_rate = 1.0
        let growth_rate_zero = GrowthCalculator::calculate_growth_rate(0, 100);
        assert_eq!(growth_rate_zero, 1.0, "Zero progress should give maximum growth rate");

        // Test progress = 0.5 (CA = 50, PA = 100) -> growth_rate = 0.25
        let growth_rate_half = GrowthCalculator::calculate_growth_rate(50, 100);
        assert_eq!(growth_rate_half, 0.25, "50% progress should give 25% growth rate");

        // Test progress = 0.8 (CA = 80, PA = 100) -> growth_rate = 0.04
        let growth_rate_high = GrowthCalculator::calculate_growth_rate(80, 100);
        assert!((growth_rate_high - 0.04).abs() < 0.001, "80% progress should give 4% growth rate");

        // Test progress >= 1.0 (CA >= PA) -> minimum 10% growth
        let growth_rate_complete = GrowthCalculator::calculate_growth_rate(100, 100);
        assert_eq!(growth_rate_complete, 0.1, "Complete progress should give minimum 10% growth");

        let growth_rate_over = GrowthCalculator::calculate_growth_rate(120, 100);
        assert_eq!(growth_rate_over, 0.1, "Over-potential should give minimum 10% growth");
    }

    #[test]
    fn test_age_modifiers() {
        // 고1 (15.0-16.0): 1.2x
        assert_eq!(GrowthCalculator::age_modifier(15.0), 1.2);
        assert_eq!(GrowthCalculator::age_modifier(15.8), 1.2);

        // 고2 (16.0-17.0): 1.0x
        assert_eq!(GrowthCalculator::age_modifier(16.0), 1.0);
        assert_eq!(GrowthCalculator::age_modifier(16.5), 1.0);

        // 고3 (17.0-18.0): 0.8x
        assert_eq!(GrowthCalculator::age_modifier(17.0), 0.8);
        assert_eq!(GrowthCalculator::age_modifier(17.9), 0.8);

        // Outside range: 0.5x
        assert_eq!(GrowthCalculator::age_modifier(14.0), 1.2); // Still gets first year bonus
        assert_eq!(GrowthCalculator::age_modifier(19.0), 0.5); // Penalty for older
    }

    #[test]
    fn test_final_growth_rate_calculation() {
        // Young player (고1) with low CA
        let growth_young_low = GrowthCalculator::calculate_final_growth_rate(30, 120, 15.5);
        let expected_young_low = 0.5625 * 1.2; // (1 - 30/120)^2 * 1.2 = 0.675
        assert!(
            (growth_young_low - expected_young_low).abs() < 0.001,
            "Young player with low CA should have high growth: expected {}, got {}",
            expected_young_low,
            growth_young_low
        );

        // Old player (고3) with high CA - minimum 5% growth applies
        let growth_old_high = GrowthCalculator::calculate_final_growth_rate(90, 100, 17.5);
        let expected_old_high = 0.05; // Minimum growth rate is 5%
        assert!(
            (growth_old_high - expected_old_high).abs() < 0.001,
            "Old player with high CA should have minimum growth: expected {}, got {}",
            expected_old_high,
            growth_old_high
        );

        // Player at potential should get minimum growth
        let growth_at_potential = GrowthCalculator::calculate_final_growth_rate(100, 100, 16.5);
        assert_eq!(growth_at_potential, 0.1, "Player at potential should get minimum 10% growth");
    }

    #[test]
    fn test_growth_points_calculation() {
        // High-intensity training with good growth potential
        let points_high = GrowthCalculator::calculate_growth_points(50, 120, 15.5, 1.0);
        assert!(points_high > 0.0, "High-intensity training should give positive growth points");

        // Low-intensity training should give fewer points
        let points_low = GrowthCalculator::calculate_growth_points(50, 120, 15.5, 0.3);
        assert!(
            points_low < points_high,
            "Low-intensity training should give fewer points than high-intensity"
        );

        // Zero intensity should give no points
        let points_zero = GrowthCalculator::calculate_growth_points(50, 120, 15.5, 0.0);
        assert_eq!(points_zero, 0.0, "Zero intensity training should give no growth points");

        // Over-intensity should be clamped
        let points_over = GrowthCalculator::calculate_growth_points(50, 120, 15.5, 1.5);
        let points_max = GrowthCalculator::calculate_growth_points(50, 120, 15.5, 1.0);
        assert_eq!(points_over, points_max, "Over-intensity should be clamped to maximum");
    }

    #[test]
    fn test_sessions_to_potential_estimation() {
        // Player far from potential should need many sessions
        let sessions_far = GrowthCalculator::estimate_sessions_to_potential(50, 120, 15.5);
        assert!(
            sessions_far.is_some() && sessions_far.unwrap() > 10,
            "Player far from potential should need many sessions: {:?}",
            sessions_far
        );

        // Player close to potential should need fewer sessions
        let sessions_close = GrowthCalculator::estimate_sessions_to_potential(95, 100, 15.5);
        assert!(
            sessions_close.is_some() && sessions_close.unwrap() < sessions_far.unwrap(),
            "Player close to potential should need fewer sessions: {:?}",
            sessions_close
        );

        // Player at potential should return None
        let sessions_at_potential =
            GrowthCalculator::estimate_sessions_to_potential(100, 100, 16.5);
        assert!(
            sessions_at_potential.is_none(),
            "Player at potential should return None: {:?}",
            sessions_at_potential
        );

        // Player above potential should return None
        let sessions_over_potential =
            GrowthCalculator::estimate_sessions_to_potential(110, 100, 16.5);
        assert!(
            sessions_over_potential.is_none(),
            "Player over potential should return None: {:?}",
            sessions_over_potential
        );
    }

    #[test]
    fn test_growth_curve_generation() {
        let curve = GrowthCalculator::get_growth_curve(100, 16.0);

        assert_eq!(curve.len(), 101, "Curve should have 101 points (0-100)");

        // Growth rate should decrease as CA approaches PA
        assert!(curve[0].1 > curve[50].1, "Growth should be higher at CA=0 than CA=50");
        assert!(curve[50].1 > curve[90].1, "Growth should be higher at CA=50 than CA=90");

        // Last point should be minimum growth (10%)
        assert_eq!(curve[100].1, 0.1, "Growth at potential should be minimum 10%");
    }

    #[test]
    fn test_edge_cases() {
        // Zero potential should return 0 growth
        let zero_potential = GrowthCalculator::calculate_growth_rate(50, 0);
        assert_eq!(zero_potential, 0.0, "Zero potential should give zero growth");

        // Very young age should still work
        let very_young = GrowthCalculator::age_modifier(14.0);
        assert_eq!(very_young, 1.2, "Very young age should get first year bonus");

        // Very old age should have penalty
        let very_old = GrowthCalculator::age_modifier(25.0);
        assert_eq!(very_old, 0.5, "Very old age should have growth penalty");

        // Minimum growth rate enforcement
        let min_growth = GrowthCalculator::calculate_final_growth_rate(0, 1, 20.0);
        assert!(min_growth >= 0.05, "Growth rate should never be below 5%: {}", min_growth);
    }

    #[test]
    fn test_growth_rate_properties() {
        // Test that growth rate is always in valid range
        for ca in 0..=200u8 {
            for pa in ca..=200u8 {
                let growth = GrowthCalculator::calculate_growth_rate(ca, pa);
                assert!(growth >= 0.0, "Growth rate should be non-negative");
                assert!(growth <= 1.0, "Growth rate should not exceed 100%");
            }
        }

        // Test age modifier properties
        for age in [14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0] {
            let modifier = GrowthCalculator::age_modifier(age);
            assert!(modifier > 0.0, "Age modifier should be positive");
            assert!(modifier <= 1.5, "Age modifier should not be too extreme");
        }
    }
}
