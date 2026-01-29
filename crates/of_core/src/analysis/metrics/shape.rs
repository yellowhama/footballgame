//! # Team Shape Metrics Module
//!
//! Calculates geometric properties of team formations.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: DRIBBLE_MOVEMENT_ANALYSIS.md

/// Team shape metrics computed from player positions.
#[derive(Debug, Clone, Default)]
pub struct TeamShapeMetrics {
    /// Maximum horizontal spread (y-axis) in meters
    pub width_m: f32,
    /// Maximum vertical spread (x-axis) in meters
    pub depth_m: f32,
    /// Area of convex hull in square meters
    pub convex_hull_area_m2: f32,
    /// Team centroid position (x, y) in meters
    pub centroid: (f32, f32),
    /// Distance from centroid to ball in meters
    pub centroid_to_ball_m: f32,
    /// Average defensive line x-position
    pub defensive_line_x: f32,
    /// Average midfield line x-position
    pub midfield_line_x: f32,
    /// Average attacking line x-position
    pub attacking_line_x: f32,
    /// Spacing between defensive and midfield lines
    pub line_spacing_def_mid_m: f32,
    /// Spacing between midfield and attacking lines
    pub line_spacing_mid_att_m: f32,
    /// Average width over time
    pub avg_width: f32,
    /// Average depth over time
    pub avg_depth: f32,
    /// Width variance over time
    pub width_variance: f32,
    /// Depth variance over time
    pub depth_variance: f32,
}

/// Calculate team shape from a snapshot of 11 player positions.
///
/// # Arguments
/// * `positions` - Array of 11 (x, y) positions in meters
pub fn calculate_team_shape(positions: &[(f32, f32); 11]) -> TeamShapeMetrics {
    // Calculate width (max_y - min_y)
    let ys: Vec<f32> = positions.iter().map(|p| p.1).collect();
    let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let width = max_y - min_y;

    // Calculate depth (max_x - min_x)
    let xs: Vec<f32> = positions.iter().map(|p| p.0).collect();
    let min_x = xs.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_x = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let depth = max_x - min_x;

    // Calculate centroid
    let cx = xs.iter().sum::<f32>() / 11.0;
    let cy = ys.iter().sum::<f32>() / 11.0;

    // Simple bounding box area as approximation (convex hull is more complex)
    let area = width * depth;

    TeamShapeMetrics {
        width_m: width,
        depth_m: depth,
        convex_hull_area_m2: area * 0.7, // Rough approximation
        centroid: (cx, cy),
        avg_width: width,
        avg_depth: depth,
        ..Default::default()
    }
}

/// QA flags for team shape anomalies.
#[derive(Debug, Clone)]
pub enum ShapeFlag {
    /// Team width is too narrow
    NarrowTeamWidth { value: f32, threshold: f32 },
    /// Team width is too wide
    WideTeamWidth { value: f32, threshold: f32 },
    /// Team depth is excessive (lines too spread)
    ExcessiveTeamDepth { value: f32, threshold: f32 },
    /// Line spacing is unbalanced
    UnbalancedLineSpacing { def_mid: f32, mid_att: f32 },
}

/// Check team shape metrics against QA thresholds.
pub fn check_shape_flags(metrics: &TeamShapeMetrics) -> Vec<ShapeFlag> {
    let mut flags = vec![];

    if metrics.avg_width < 35.0 {
        flags.push(ShapeFlag::NarrowTeamWidth {
            value: metrics.avg_width,
            threshold: 35.0,
        });
    }

    if metrics.avg_width > 60.0 {
        flags.push(ShapeFlag::WideTeamWidth {
            value: metrics.avg_width,
            threshold: 60.0,
        });
    }

    if metrics.avg_depth > 50.0 {
        flags.push(ShapeFlag::ExcessiveTeamDepth {
            value: metrics.avg_depth,
            threshold: 50.0,
        });
    }

    let line_ratio = metrics.line_spacing_def_mid_m
        / metrics.line_spacing_mid_att_m.max(0.1);
    if line_ratio > 2.5 || line_ratio < 0.4 {
        flags.push(ShapeFlag::UnbalancedLineSpacing {
            def_mid: metrics.line_spacing_def_mid_m,
            mid_att: metrics.line_spacing_mid_att_m,
        });
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_shape_basic() {
        // 11 players spread across the field
        let positions: [(f32, f32); 11] = [
            (10.0, 34.0),  // GK
            (25.0, 10.0),  // LB
            (25.0, 25.0),  // CB
            (25.0, 43.0),  // CB
            (25.0, 58.0),  // RB
            (40.0, 20.0),  // LM
            (40.0, 34.0),  // CM
            (40.0, 48.0),  // RM
            (55.0, 25.0),  // LW
            (55.0, 34.0),  // ST
            (55.0, 43.0),  // RW
        ];

        let shape = calculate_team_shape(&positions);

        assert!(shape.width_m > 40.0, "Width should be > 40m");
        assert!(shape.depth_m > 40.0, "Depth should be > 40m");
        assert!(shape.centroid.0 > 30.0 && shape.centroid.0 < 45.0);
    }
}
