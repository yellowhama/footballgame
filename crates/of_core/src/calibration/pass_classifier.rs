//! Pass Type Classifier
//!
//! Classifies passes into categories based on SSOT definitions.

use super::zone::{ZoneId, pos_to_zone_for_team};

/// Pass type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassType {
    /// Forward pass advancing toward opponent goal (Δx ≥ 0.15)
    Progressive,
    /// Pass that leads to a shot attempt
    Key,
    /// Wide to penalty area pass
    Cross,
    /// Long distance pass (distance ≥ 30m)
    Long,
    /// Backward pass (Δx < -0.05)
    Backward,
    /// Sideways/lateral pass (-0.05 ≤ Δx < 0.15)
    Lateral,
}

impl PassType {
    /// All pass types
    pub const ALL: [PassType; 6] = [
        PassType::Progressive,
        PassType::Key,
        PassType::Cross,
        PassType::Long,
        PassType::Backward,
        PassType::Lateral,
    ];

    /// Is this a forward-oriented pass?
    pub fn is_forward(&self) -> bool {
        matches!(self, PassType::Progressive | PassType::Key)
    }
}

/// Normalized position for pass classification
#[derive(Debug, Clone, Copy)]
pub struct NormPos {
    pub x: f32,
    pub y: f32,
}

/// Field dimensions for distance calculation
const FIELD_LENGTH_M: f32 = 105.0;
const FIELD_WIDTH_M: f32 = 68.0;

impl NormPos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Distance to another position (normalized, legacy - biased for lateral passes)
    pub fn distance_to(&self, other: &NormPos) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    /// Distance to another position in actual meters
    /// FIX_2601/0113: Correctly accounts for different x/y field scales
    pub fn distance_meters(&self, other: &NormPos) -> f32 {
        let dx_m = (self.x - other.x) * FIELD_LENGTH_M;
        let dy_m = (self.y - other.y) * FIELD_WIDTH_M;
        (dx_m.powi(2) + dy_m.powi(2)).sqrt()
    }
}

/// Classification thresholds (SSOT)
pub struct ClassifierThresholds {
    /// Minimum forward progress for progressive pass (normalized)
    pub progressive_delta: f32,
    /// Minimum distance for long pass in meters (FIX_2601/0113)
    /// Default: 30.0m - matches standard football analytics definition
    pub long_distance_meters: f32,
    /// Y threshold for wide zones
    pub wide_y_threshold: f32,
    /// Backward pass threshold (negative delta)
    pub backward_delta: f32,
}

impl Default for ClassifierThresholds {
    fn default() -> Self {
        Self {
            progressive_delta: 0.15,
            // FIX_2601/0113: Use actual meters instead of normalized distance
            // 30m is standard threshold for "long pass" in football analytics
            long_distance_meters: 30.0,
            wide_y_threshold: 0.25,
            backward_delta: -0.05,
        }
    }
}

/// Classify a pass based on start/end positions
///
/// # Arguments
/// * `start` - Pass origin in normalized coordinates
/// * `end` - Pass destination in normalized coordinates
/// * `attacks_right` - True if the passing team attacks toward X=1.0
/// * `thresholds` - Classification thresholds
///
/// # Returns
/// Primary pass type classification
pub fn classify_pass(
    start: NormPos,
    end: NormPos,
    attacks_right: bool,
    thresholds: &ClassifierThresholds,
) -> PassType {
    // FIX_2601/0113: Use actual meters for long pass classification
    let distance_m = start.distance_meters(&end);

    // Calculate forward progress (toward opponent goal)
    let delta_x = if attacks_right {
        end.x - start.x
    } else {
        start.x - end.x
    };

    // Long pass takes priority (using actual meters)
    if distance_m >= thresholds.long_distance_meters {
        return PassType::Long;
    }

    // Cross: from wide zone to central attacking zone
    if is_cross(&start, &end, attacks_right, thresholds) {
        return PassType::Cross;
    }

    // Progressive: significant forward movement
    if delta_x >= thresholds.progressive_delta {
        return PassType::Progressive;
    }

    // Backward: moving away from opponent goal
    if delta_x < thresholds.backward_delta {
        return PassType::Backward;
    }

    // Default: lateral
    PassType::Lateral
}

/// Check if pass is a cross
fn is_cross(
    start: &NormPos,
    end: &NormPos,
    attacks_right: bool,
    thresholds: &ClassifierThresholds,
) -> bool {
    // Start must be in wide zone
    let in_wide_zone = start.y < thresholds.wide_y_threshold
        || start.y > (1.0 - thresholds.wide_y_threshold);

    if !in_wide_zone {
        return false;
    }

    // End must be in attacking central zone (penalty area corridor)
    let end_zone = pos_to_zone_for_team(end.x, end.y, attacks_right);
    end_zone == ZoneId::CAtt
}

/// Pass classification result with all applicable types
#[derive(Debug, Clone)]
pub struct PassClassification {
    /// Primary type
    pub primary: PassType,
    /// Is this pass progressive?
    pub is_progressive: bool,
    /// Is this a long pass?
    pub is_long: bool,
    /// Is this a cross?
    pub is_cross: bool,
    /// Forward progress (normalized)
    pub delta_x: f32,
    /// Distance in meters (FIX_2601/0113: changed from normalized)
    pub distance_m: f32,
    /// Start zone
    pub start_zone: ZoneId,
    /// End zone
    pub end_zone: ZoneId,
}

/// Detailed pass classification
pub fn classify_pass_detailed(
    start: NormPos,
    end: NormPos,
    attacks_right: bool,
    thresholds: &ClassifierThresholds,
) -> PassClassification {
    // FIX_2601/0113: Use actual meters for distance calculation
    let distance_m = start.distance_meters(&end);
    let delta_x = if attacks_right {
        end.x - start.x
    } else {
        start.x - end.x
    };

    let is_progressive = delta_x >= thresholds.progressive_delta;
    // FIX_2601/0113: Use meters threshold for long pass
    let is_long = distance_m >= thresholds.long_distance_meters;
    let is_cross_pass = is_cross(&start, &end, attacks_right, thresholds);

    let primary = classify_pass(start, end, attacks_right, thresholds);

    PassClassification {
        primary,
        is_progressive,
        is_long,
        is_cross: is_cross_pass,
        delta_x,
        distance_m,
        start_zone: pos_to_zone_for_team(start.x, start.y, attacks_right),
        end_zone: pos_to_zone_for_team(end.x, end.y, attacks_right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progressive_pass() {
        let thresholds = ClassifierThresholds::default();
        let start = NormPos::new(0.3, 0.5);
        let end = NormPos::new(0.5, 0.5); // +0.2 forward = 21m

        let pass_type = classify_pass(start, end, true, &thresholds);
        assert_eq!(pass_type, PassType::Progressive);
    }

    #[test]
    fn test_long_pass() {
        let thresholds = ClassifierThresholds::default();
        // 52.5m horizontal pass (0.7-0.2)*105 = 52.5m > 30m threshold
        let start = NormPos::new(0.2, 0.5);
        let end = NormPos::new(0.7, 0.5);

        let pass_type = classify_pass(start, end, true, &thresholds);
        assert_eq!(pass_type, PassType::Long);

        // Verify distance calculation
        let distance_m = start.distance_meters(&end);
        assert!((distance_m - 52.5).abs() < 0.1, "Expected ~52.5m, got {}", distance_m);
    }

    #[test]
    fn test_backward_pass() {
        let thresholds = ClassifierThresholds::default();
        let start = NormPos::new(0.5, 0.5);
        let end = NormPos::new(0.3, 0.5); // -0.2 backward = -21m

        let pass_type = classify_pass(start, end, true, &thresholds);
        assert_eq!(pass_type, PassType::Backward);
    }

    #[test]
    fn test_lateral_pass() {
        let thresholds = ClassifierThresholds::default();
        let start = NormPos::new(0.5, 0.3);
        let end = NormPos::new(0.52, 0.5); // small forward, mainly sideways

        let pass_type = classify_pass(start, end, true, &thresholds);
        assert_eq!(pass_type, PassType::Lateral);
    }

    #[test]
    fn test_attacks_left_progressive() {
        let thresholds = ClassifierThresholds::default();
        let start = NormPos::new(0.7, 0.5);
        let end = NormPos::new(0.5, 0.5); // -0.2 in X, but forward for attacks_left

        let pass_type = classify_pass(start, end, false, &thresholds);
        assert_eq!(pass_type, PassType::Progressive);
    }

    // FIX_2601/0113: Test that lateral passes are NOT incorrectly classified as Long
    #[test]
    fn test_lateral_not_long_with_meters() {
        let thresholds = ClassifierThresholds::default();
        // Lateral pass: dx=20m, dy=22m → actual distance = 29.7m < 30m
        // Previously with normalized distance: 0.19² + 0.32² = 0.37 > 0.35 → wrongly Long
        let start = NormPos::new(0.5, 0.5);
        // dx = 20m/105 = 0.19, dy = 22m/68 = 0.32
        let end = NormPos::new(0.5 + 20.0/105.0, 0.5 + 22.0/68.0);

        let distance_m = start.distance_meters(&end);
        assert!((distance_m - 29.7).abs() < 0.5, "Expected ~29.7m, got {}", distance_m);

        let pass_type = classify_pass(start, end, true, &thresholds);
        // Should NOT be Long anymore with meters-based threshold
        assert_ne!(pass_type, PassType::Long, "29.7m pass should not be Long");
    }

    #[test]
    fn test_long_threshold_31m() {
        let thresholds = ClassifierThresholds::default();
        // 31m horizontal pass (slightly above threshold to avoid floating point issues)
        let start = NormPos::new(0.5, 0.5);
        let end = NormPos::new(0.5 + 31.0/105.0, 0.5); // 31m forward

        let distance_m = start.distance_meters(&end);
        assert!((distance_m - 31.0).abs() < 0.1, "Expected ~31m, got {}", distance_m);

        let pass_type = classify_pass(start, end, true, &thresholds);
        assert_eq!(pass_type, PassType::Long, "31m pass should be Long");
    }

    #[test]
    fn test_not_long_at_29m() {
        let thresholds = ClassifierThresholds::default();
        // 29m horizontal pass (below 30m threshold)
        let start = NormPos::new(0.5, 0.5);
        let end = NormPos::new(0.5 + 29.0/105.0, 0.5); // 29m forward

        let distance_m = start.distance_meters(&end);
        assert!((distance_m - 29.0).abs() < 0.1, "Expected ~29m, got {}", distance_m);

        let pass_type = classify_pass(start, end, true, &thresholds);
        // 29m with +0.276 forward is Progressive, not Long
        assert_eq!(pass_type, PassType::Progressive, "29m forward pass should be Progressive");
    }

    #[test]
    fn test_distance_meters_calculation() {
        // Diagonal pass: 30m horizontal + 30m vertical
        let start = NormPos::new(0.0, 0.0);
        let end = NormPos::new(30.0/105.0, 30.0/68.0);

        let distance_m = start.distance_meters(&end);
        // sqrt(30² + 30²) = 42.4m
        assert!((distance_m - 42.4).abs() < 0.5, "Expected ~42.4m, got {}", distance_m);
    }
}
