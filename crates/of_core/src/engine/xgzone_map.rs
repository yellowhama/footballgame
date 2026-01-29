use crate::engine::field_board::HeatmapF32;
use serde::{Deserialize, Serialize};

// Constants for xG zone classification
pub const XG_THRESHOLD_RED: f32 = 0.15;
pub const XG_THRESHOLD_YELLOW: f32 = 0.05;
const GOAL_WIDTH: f32 = 7.32; // meters
const UPDATE_INTERVAL_TICKS: u64 = 10;

/// XGZoneMap provides spatial expected goals (xG) calculation across the FieldBoard grid.
///
/// Each cell contains an xG value representing the probability of scoring from that position.
/// Zones are classified as RED (high xG >0.15), YELLOW (medium 0.05-0.15), or GREEN (low <0.05).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XGZoneMap {
    cells: HeatmapF32,
    last_update_tick: u64,
}

impl XGZoneMap {
    /// Creates a new XGZoneMap and initializes xG values for all cells.
    pub fn new(cols: u8, rows: u8) -> Self {
        let mut map = Self { cells: HeatmapF32::new(cols, rows), last_update_tick: 0 };
        map.initialize_zones(cols, rows);
        map
    }

    /// Initialize xG values for all cells based on geometric position.
    /// This is called once at creation and produces static zone values.
    fn initialize_zones(&mut self, cols: u8, rows: u8) {
        let mut red_count = 0;
        let mut yellow_count = 0;
        let mut green_count = 0;

        for row in 0..rows {
            for col in 0..cols {
                let cell_center = self.cell_to_normalized_pos(col, row, cols, rows);
                let xg = self.calculate_cell_xg(cell_center);
                let idx = (row as usize) * (cols as usize) + (col as usize);
                self.cells.values[idx] = xg;

                // Track zone distribution for logging
                if xg >= XG_THRESHOLD_RED {
                    red_count += 1;
                } else if xg >= XG_THRESHOLD_YELLOW {
                    yellow_count += 1;
                } else {
                    green_count += 1;
                }
            }
        }

        let total = (cols as usize) * (rows as usize);
        println!(
            "[XGZone] Initialized: RED={}%, YELLOW={}%, GREEN={}%",
            red_count * 100 / total,
            yellow_count * 100 / total,
            green_count * 100 / total
        );
    }

    /// Convert cell coordinates to normalized field position (0.0-1.0).
    fn cell_to_normalized_pos(&self, col: u8, row: u8, cols: u8, rows: u8) -> (f32, f32) {
        let norm_x = (col as f32 + 0.5) / cols as f32;
        let norm_y = (row as f32 + 0.5) / rows as f32;
        (norm_x, norm_y)
    }

    /// Calculate xG for a given normalized position using geometric hybrid model.
    ///
    /// Factors:
    /// - Distance to goal (exponential decay)
    /// - Angle to goal (wider angle = better)
    /// - Zone penalty (defensive half = 0.3x multiplier)
    fn calculate_cell_xg(&self, pos: (f32, f32)) -> f32 {
        // Attacking toward (0.5, 1.0) - top goal
        let goal_pos = (0.5, 1.0);
        let dx = goal_pos.0 - pos.0;
        let dy = goal_pos.1 - pos.1;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.001 {
            return 0.30; // Max xG at goal center
        }

        // Angle factor: wider angle to goal = better shooting opportunity
        let angle_factor = ((GOAL_WIDTH / 2.0) / dist).atan().sin();

        // Distance decay: exponential falloff with distance
        let dist_factor = (-2.0 * dist).exp();

        // Defensive zone penalty: own half (y < 0.4) has reduced xG
        let zone_penalty = if pos.1 < 0.4 { 0.3 } else { 1.0 };

        // Weighted combination: distance matters more than angle
        let base_xg = 0.3 * angle_factor + 0.7 * dist_factor;
        (base_xg * zone_penalty).clamp(0.0, 0.30)
    }

    /// Update xG zones if enough ticks have passed.
    /// Currently zones are static, but this allows future dynamic factors.
    pub fn maybe_update(&mut self, current_tick: u64) {
        if current_tick - self.last_update_tick >= UPDATE_INTERVAL_TICKS {
            // Future: Add dynamic factors (fatigue, weather, tactical pressure)
            // Currently: No-op, zones are static after initialization
            self.last_update_tick = current_tick;
        }
    }

    /// Get xG value at a normalized position (0.0-1.0).
    ///
    /// NOTE: This method assumes the caller has already flipped the position
    /// based on attack direction. For a safer API, use `get_xg_directional`.
    pub fn get_xg_at_normalized_pos(&self, pos: (f32, f32)) -> f32 {
        let cols = self.cells.cols as usize;
        let rows = self.cells.rows as usize;
        let x = (pos.0 * cols as f32).floor() as usize;
        let y = (pos.1 * rows as f32).floor() as usize;
        let idx = y.min(rows - 1) * cols + x.min(cols - 1);
        self.cells.values[idx]
    }

    /// Get xG value at a normalized position with explicit direction handling.
    ///
    /// This method automatically flips the y-coordinate based on attack direction,
    /// ensuring correct xG lookup for teams attacking toward either goal.
    ///
    /// # Arguments
    /// * `pos` - Normalized world position (x, y) where y is field length (0..1)
    /// * `attacks_right` - true if attacking toward y=1.0 (right/top goal), false if toward y=0.0
    ///
    /// # Returns
    /// xG value (0.0-0.30) representing expected goal probability from that position
    pub fn get_xg_directional(&self, pos: (f32, f32), attacks_right: bool) -> f32 {
        // The xG map is pre-computed assuming goal at y=1.0.
        // For teams attacking toward y=0.0, flip the y-coordinate.
        let lookup_pos = if attacks_right {
            pos // Goal at y=1.0, position correct
        } else {
            (pos.0, 1.0 - pos.1) // Goal at y=0.0, flip y
        };
        self.get_xg_at_normalized_pos(lookup_pos)
    }

    /// Get direct access to the underlying heatmap for export.
    pub fn get_cells(&self) -> &HeatmapF32 {
        &self.cells
    }
}

/// Zone color classification for UI rendering and decision gating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XGZoneColor {
    Red,    // High xG (>0.15) - always allow SHOOT
    Yellow, // Medium xG (0.05-0.15) - conditional SHOOT
    Green,  // Low xG (<0.05) - block SHOOT
}

impl XGZoneColor {
    /// Classify an xG value into a zone color.
    pub fn from_xg(xg: f32) -> Self {
        if xg >= XG_THRESHOLD_RED {
            XGZoneColor::Red
        } else if xg >= XG_THRESHOLD_YELLOW {
            XGZoneColor::Yellow
        } else {
            XGZoneColor::Green
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xg_directional_symmetry() {
        let map = XGZoneMap::new(20, 20);

        // Position close to y=1.0 goal (high xG for attacks_right=true)
        let pos_near_y1 = (0.5, 0.9);
        // Position close to y=0.0 goal (high xG for attacks_right=false)
        let pos_near_y0 = (0.5, 0.1);

        // Team attacking right (toward y=1.0): near y=1.0 should have high xG
        let xg_right_near = map.get_xg_directional(pos_near_y1, true);
        // Team attacking left (toward y=0.0): near y=0.0 should have high xG
        let xg_left_near = map.get_xg_directional(pos_near_y0, false);

        // Both should be similar (symmetric positions relative to goal)
        assert!(
            (xg_right_near - xg_left_near).abs() < 0.01,
            "xG should be symmetric: attacks_right near goal = {:.4}, attacks_left near goal = {:.4}",
            xg_right_near, xg_left_near
        );

        // Team attacking right: near y=0.0 should have LOW xG (far from goal)
        let xg_right_far = map.get_xg_directional(pos_near_y0, true);
        // Team attacking left: near y=1.0 should have LOW xG (far from goal)
        let xg_left_far = map.get_xg_directional(pos_near_y1, false);

        // Both should be similar (symmetric far positions)
        assert!(
            (xg_right_far - xg_left_far).abs() < 0.01,
            "Far xG should be symmetric: attacks_right far = {:.4}, attacks_left far = {:.4}",
            xg_right_far, xg_left_far
        );

        // Near goal should have higher xG than far from goal
        assert!(
            xg_right_near > xg_right_far,
            "Near goal should have higher xG than far: near={:.4}, far={:.4}",
            xg_right_near, xg_right_far
        );
    }
}
