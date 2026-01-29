//! field_board.rs
//!
//! A-Plan Board Layer:
//! - Truth = meters (105x68)
//! - Board = view/query layer (cells)
//! - Provides occupancy + pressure heatmaps for debug, decision hints, replay summary.
//!
//! Design goals:
//! - cheap to update every tick (occupancy)
//! - pressure can be updated every N ticks
//! - never mutates "world truth": caller provides player positions in meters
//!
//! ## Match OS v1.2 Priority 3: LaneBlock Hybrid
//!
//! `lane_risk_hint()` provides O(k) grid-based pass lane detection:
//! - Sample k cells along lane (k < 20)
//! - Combine pressure (60%) + occupancy (40%) → normalized risk 0..1
//! - Performance: < 50μs per call (measured: < 1μs)
//! - Enables hybrid pass evaluation: Grid hint + raycast validation
//!
//! **Design Philosophy**: Avoid O(cells²) = 254,016 checks/tick by using:
//! 1. Fast grid sampling (O(k) with k < 20)
//! 2. Adaptive sampling: 1 sample per ~7m of lane length
//! 3. Combines spatial context from FieldBoard heatmaps

use serde::{Deserialize, Serialize};

use crate::engine::physics_constants::field;
use crate::engine::xgzone_map::XGZoneMap;

pub const FIELD_LENGTH_M: f32 = field::LENGTH_M;
pub const FIELD_WIDTH_M: f32 = field::WIDTH_M;

/// Neighbor lookup mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborMode {
    VonNeumann4,
    Moore8,
}

/// A cell index in (col,row). Both are 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellIndex {
    pub col: u8,
    pub row: u8,
}

impl CellIndex {
    #[inline]
    pub fn id(self, cols: u8) -> usize {
        (self.row as usize) * (cols as usize) + (self.col as usize)
    }
}

/// A-Plan board configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FieldBoardSpec {
    pub cols: u8,
    pub rows: u8,
}

impl Default for FieldBoardSpec {
    fn default() -> Self {
        // Recommended default: 28x18 (~3.75m per cell)
        Self { cols: 28, rows: 18 }
    }
}

/// Occupancy counts per cell.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct OccupancyCell {
    pub total: u8,
    pub home: u8,
    pub away: u8,
}

/// Scalar heatmap container (0..=N), stored row-major.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapF32 {
    pub cols: u8,
    pub rows: u8,
    pub values: Vec<f32>, // len = cols*rows
}

impl HeatmapF32 {
    pub fn new(cols: u8, rows: u8) -> Self {
        let len = (cols as usize) * (rows as usize);
        Self { cols, rows, values: vec![0.0; len] }
    }

    #[inline]
    pub fn idx(&self, c: CellIndex) -> usize {
        c.id(self.cols)
    }

    #[inline]
    pub fn get(&self, c: CellIndex) -> f32 {
        self.values[self.idx(c)]
    }

    #[inline]
    pub fn set(&mut self, c: CellIndex, v: f32) {
        let i = self.idx(c);
        self.values[i] = v;
    }

    pub fn clear(&mut self) {
        for v in &mut self.values {
            *v = 0.0;
        }
    }

    pub fn max_value(&self) -> f32 {
        self.values.iter().cloned().fold(0.0_f32, f32::max)
    }
}

// ============================================================================
// P2.1-A: Coordinate Bounds Validation
// ============================================================================

/// Field boundaries for coordinate validation (P2.1-A)
///
/// Contract: All player positions must be within field bounds
/// - X axis: 0.0 to FIELD_LENGTH_M (105.0m)
/// - Y axis: 0.0 to FIELD_WIDTH_M (68.0m)
///
/// Purpose: Detect out-of-bounds bugs, teleportation, invalid positions
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FieldBounds {
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
}

impl FieldBounds {
    /// Standard football field bounds (105m × 68m)
    pub fn standard() -> Self {
        Self { x_min: 0.0, x_max: FIELD_LENGTH_M, y_min: 0.0, y_max: FIELD_WIDTH_M }
    }

    /// Validate a position against field bounds
    ///
    /// Returns Ok(()) if position is within bounds, Err with details otherwise
    pub fn validate_position(&self, x: f32, y: f32) -> Result<(), String> {
        if x < self.x_min || x > self.x_max {
            return Err(format!("x={:.2} out of bounds [{:.2}, {:.2}]", x, self.x_min, self.x_max));
        }
        if y < self.y_min || y > self.y_max {
            return Err(format!("y={:.2} out of bounds [{:.2}, {:.2}]", y, self.y_min, self.y_max));
        }
        Ok(())
    }

    /// Check if position is within bounds (boolean check)
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }

    /// Clamp position to bounds (defensive fallback)
    pub fn clamp_position(&self, x: f32, y: f32) -> (f32, f32) {
        (x.clamp(self.x_min, self.x_max), y.clamp(self.y_min, self.y_max))
    }
}

impl Default for FieldBounds {
    fn default() -> Self {
        Self::standard()
    }
}

// ============================================================================

/// Board runtime storage:
/// - occupancy (counts)
/// - pressure maps (against home / against away)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldBoard {
    pub spec: FieldBoardSpec,
    pub cell_w_m: f32,
    pub cell_h_m: f32,

    // Update cadence tracking
    pub occupancy_version_tick: u64,
    pub pressure_version_tick: u64,

    // Layers
    pub occupancy: Vec<OccupancyCell>,     // len = cols*rows
    pub pressure_against_home: HeatmapF32, // when Home has ball, pressure from Away
    pub pressure_against_away: HeatmapF32, // when Away has ball, pressure from Home
    pub xgzone: XGZoneMap,                 // Match OS v1.2: Expected goals (xG) map
}

impl FieldBoard {
    pub fn new(spec: FieldBoardSpec) -> Self {
        let cell_w_m = FIELD_LENGTH_M / spec.cols as f32;
        let cell_h_m = FIELD_WIDTH_M / spec.rows as f32;
        let len = (spec.cols as usize) * (spec.rows as usize);
        Self {
            spec,
            cell_w_m,
            cell_h_m,
            occupancy_version_tick: 0,
            pressure_version_tick: 0,
            occupancy: vec![OccupancyCell::default(); len],
            pressure_against_home: HeatmapF32::new(spec.cols, spec.rows),
            pressure_against_away: HeatmapF32::new(spec.cols, spec.rows),
            xgzone: XGZoneMap::new(spec.cols, spec.rows),
        }
    }

    #[inline]
    pub fn cols(&self) -> u8 {
        self.spec.cols
    }
    #[inline]
    pub fn rows(&self) -> u8 {
        self.spec.rows
    }

    /// Clamp a meter coordinate into field bounds.
    #[inline]
    pub fn clamp_to_field(pos_m: (f32, f32)) -> (f32, f32) {
        (pos_m.0.clamp(0.0, FIELD_LENGTH_M), pos_m.1.clamp(0.0, FIELD_WIDTH_M))
    }

    /// Convert meter position to cell.
    /// This is a view mapping; it clamps out-of-bounds into nearest valid cell.
    #[inline]
    pub fn cell_of(&self, pos_m: (f32, f32)) -> CellIndex {
        let (x, y) = Self::clamp_to_field(pos_m);
        let mut col = (x / self.cell_w_m).floor() as i32;
        let mut row = (y / self.cell_h_m).floor() as i32;
        // edge case: x == length => last col; y == width => last row
        if col >= self.spec.cols as i32 {
            col = self.spec.cols as i32 - 1;
        }
        if row >= self.spec.rows as i32 {
            row = self.spec.rows as i32 - 1;
        }
        if col < 0 {
            col = 0;
        }
        if row < 0 {
            row = 0;
        }
        CellIndex { col: col as u8, row: row as u8 }
    }

    /// Cell center in meters.
    #[inline]
    pub fn cell_center(&self, cell: CellIndex) -> (f32, f32) {
        let cx = (cell.col as f32 + 0.5) * self.cell_w_m;
        let cy = (cell.row as f32 + 0.5) * self.cell_h_m;
        (cx, cy)
    }

    /// Neighbor cells.
    pub fn neighbors(&self, cell: CellIndex, mode: NeighborMode) -> Vec<CellIndex> {
        let c = cell.col as i32;
        let r = cell.row as i32;
        let cols = self.spec.cols as i32;
        let rows = self.spec.rows as i32;

        let capacity = match mode {
            NeighborMode::VonNeumann4 => 4,
            NeighborMode::Moore8 => 8,
        };
        let mut out = Vec::with_capacity(capacity);

        let push_if = |out: &mut Vec<CellIndex>, cc: i32, rr: i32| {
            if cc >= 0 && cc < cols && rr >= 0 && rr < rows {
                out.push(CellIndex { col: cc as u8, row: rr as u8 });
            }
        };

        match mode {
            NeighborMode::VonNeumann4 => {
                push_if(&mut out, c + 1, r);
                push_if(&mut out, c - 1, r);
                push_if(&mut out, c, r + 1);
                push_if(&mut out, c, r - 1);
            }
            NeighborMode::Moore8 => {
                for dr in -1..=1 {
                    for dc in -1..=1 {
                        if dc == 0 && dr == 0 {
                            continue;
                        }
                        push_if(&mut out, c + dc, r + dr);
                    }
                }
            }
        }

        out
    }

    /// Clear occupancy.
    pub fn clear_occupancy(&mut self) {
        for c in &mut self.occupancy {
            *c = OccupancyCell::default();
        }
    }

    /// Update occupancy map from player positions.
    /// Contract: track_id 0..10 = Home, 11..21 = Away.
    pub fn update_occupancy_from_positions_m(
        &mut self,
        current_tick: u64,
        player_positions_m: &[(f32, f32); 22],
    ) {
        self.clear_occupancy();
        let cols = self.spec.cols;
        for track_id in 0..22 {
            let cell = self.cell_of(player_positions_m[track_id]);
            let idx = cell.id(cols);
            let entry = &mut self.occupancy[idx];
            entry.total = entry.total.saturating_add(1);
            if track_id < 11 {
                entry.home = entry.home.saturating_add(1);
            } else {
                entry.away = entry.away.saturating_add(1);
            }
        }
        self.occupancy_version_tick = current_tick;
    }

    /// Update pressure maps (against home/away).
    ///
    /// Pressure is computed on cells (centers), summing influence from defenders.
    /// Two maps:
    /// - pressure_against_home: away defenders influence
    /// - pressure_against_away: home defenders influence
    ///
    /// This is a "hint" map; it does not need perfect physical accuracy.
    pub fn update_pressure_from_positions_m(
        &mut self,
        current_tick: u64,
        player_positions_m: &[(f32, f32); 22],
        // Optional per-player weights (0..1), e.g., stamina/pressing/workrate scaling.
        // If None, weight=1 for all defenders.
        defender_weights: Option<&[f32; 22]>,
        influence_radius_m: f32,
    ) {
        let r = influence_radius_m.max(0.1);
        let inv_r = 1.0 / r;

        self.pressure_against_home.clear();
        self.pressure_against_away.clear();

        let cols = self.spec.cols;
        let rows = self.spec.rows;

        // helper: squared distance
        #[inline]
        fn dist2(a: (f32, f32), b: (f32, f32)) -> f32 {
            let dx = a.0 - b.0;
            let dy = a.1 - b.1;
            dx * dx + dy * dy
        }

        // Iterate all cells and sum influences. This is O(cells * players).
        // With 28x18=504 cells and 22 players => ~11k ops per update; OK if done every few ticks.
        for row in 0..rows {
            for col in 0..cols {
                let cell = CellIndex { col, row };
                let center = self.cell_center(cell);

                let mut away_influence = 0.0_f32; // pressure against home
                let mut home_influence = 0.0_f32; // pressure against away

                for track_id in 0..22 {
                    let w = defender_weights.map(|ws| ws[track_id]).unwrap_or(1.0).clamp(0.0, 2.0);
                    if w <= 0.0 {
                        continue;
                    }

                    let p = player_positions_m[track_id];
                    let d2 = dist2(p, center);
                    if d2 >= r * r {
                        continue;
                    }

                    // base influence: (1 - d/r)^2
                    let d = d2.sqrt();
                    let t = 1.0 - d * inv_r;
                    let base = t * t;

                    if track_id < 11 {
                        // home defender contributes to pressure against away
                        home_influence += base * w;
                    } else {
                        // away defender contributes to pressure against home
                        away_influence += base * w;
                    }
                }

                self.pressure_against_home.set(cell, away_influence);
                self.pressure_against_away.set(cell, home_influence);
            }
        }

        self.pressure_version_tick = current_tick;
    }

    /// Convenience: returns occupancy as scalar float heatmap (total counts),
    /// useful for viewer overlays.
    pub fn occupancy_total_heatmap(&self) -> HeatmapF32 {
        let mut hm = HeatmapF32::new(self.spec.cols, self.spec.rows);
        for row in 0..self.spec.rows {
            for col in 0..self.spec.cols {
                let cell = CellIndex { col, row };
                let idx = cell.id(self.spec.cols);
                hm.set(cell, self.occupancy[idx].total as f32);
            }
        }
        hm
    }

    /// Export full snapshot for viewer overlay (v1.0 requirement).
    /// Returns complete heatmap data for rendering.
    pub fn to_snapshot_export(&self) -> FieldBoardSnapshotExport {
        FieldBoardSnapshotExport {
            cols: self.spec.cols,
            rows: self.spec.rows,
            occupancy_total: self.occupancy.iter().map(|c| c.total as f32).collect(),
            pressure_against_home: self.pressure_against_home.values.clone(),
            pressure_against_away: self.pressure_against_away.values.clone(),
            xgzone_map: self.xgzone.get_cells().values.clone(),
        }
    }

    /// Create a small export summary from the current board state.
    /// Note: If you want full heatmaps, export them through replay/telemetry stream, not MatchResult.
    pub fn to_summary_export(&self, top_k: usize) -> BoardSummaryExport {
        let mut occ_max: u8 = 0;
        for c in &self.occupancy {
            if c.total > occ_max {
                occ_max = c.total;
            }
        }

        let p_home = self.pressure_against_home.max_value();
        let p_away = self.pressure_against_away.max_value();

        // Collect top occupancy cells (by total count at this moment).
        let mut hot: Vec<(CellIndex, f32)> =
            Vec::with_capacity((self.cols() as usize) * (self.rows() as usize));
        for row in 0..self.rows() {
            for col in 0..self.cols() {
                let cell = CellIndex { col, row };
                let idx = cell.id(self.cols());
                let v = self.occupancy[idx].total as f32;
                if v > 0.0 {
                    hot.push((cell, v));
                }
            }
        }
        hot.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        hot.truncate(top_k.min(hot.len()));

        let hottest_occupancy_cells = hot
            .into_iter()
            .map(|(c, v)| HotCellExport { col: c.col, row: c.row, value: v })
            .collect();

        BoardSummaryExport {
            cols: self.cols(),
            rows: self.rows(),
            cell_w_m: self.cell_w_m,
            cell_h_m: self.cell_h_m,
            occupancy_max: occ_max,
            pressure_max_against_home: p_home,
            pressure_max_against_away: p_away,
            hottest_occupancy_cells,
        }
    }

    // ===================================================================
    // Match OS v1.2 Priority 3: LaneBlock Hybrid
    // ===================================================================

    /// Lane occlusion risk hint (grid-based, O(k) with k < 20)
    ///
    /// Samples cells along pass lane and combines:
    /// - Pressure heatmap (opponent density)
    /// - Occupancy counts (bodies in the way)
    ///
    /// Returns normalized risk 0..1 (0 = clear lane, 1 = heavily blocked)
    ///
    /// # Performance
    /// - Adaptive sampling: 1 sample per ~7m (ensures k < 20 samples)
    /// - Target: < 50μs per call
    /// - Complexity: O(k) with k = clamp(lane_length_m / 7.0, 3, 18)
    ///
    /// # Algorithm
    /// 1. Calculate lane length and determine sample count
    /// 2. For each sample point along the lane:
    ///    - Convert to cell index
    ///    - Get pressure (60% weight) + occupancy (40% weight)
    /// 3. Return average risk across all samples
    pub fn lane_risk_hint(
        &self,
        from_m: (f32, f32),
        to_m: (f32, f32),
        is_home_passer: bool,
    ) -> f32 {
        let pressure_map = if is_home_passer {
            &self.pressure_against_home // Away defenders
        } else {
            &self.pressure_against_away // Home defenders
        };

        let dx = to_m.0 - from_m.0;
        let dy = to_m.1 - from_m.1;
        let lane_length_m = (dx * dx + dy * dy).sqrt();

        // Adaptive sampling: 1 sample per ~7m (ensures k < 20)
        let num_samples = ((lane_length_m / 7.0).ceil() as usize).clamp(3, 18);

        let mut total_risk = 0.0_f32;
        let mut sample_count = 0;

        for i in 0..num_samples {
            let t = if num_samples == 1 { 0.5 } else { i as f32 / (num_samples - 1) as f32 };

            let sample_x = from_m.0 + t * dx;
            let sample_y = from_m.1 + t * dy;
            let sample_pos = (sample_x, sample_y);

            let cell = self.cell_of(sample_pos);

            // Pressure contribution (0..3 typical)
            let pressure = pressure_map.get(cell);
            let pressure_norm = (pressure / 3.0).clamp(0.0, 1.0);

            // Occupancy contribution
            let occ = &self.occupancy[cell.id(self.cols())];
            let occupancy_risk = if is_home_passer {
                (occ.away as f32 * 0.3).min(1.0)
            } else {
                (occ.home as f32 * 0.3).min(1.0)
            };

            // Combine: pressure (60%) + occupancy (40%)
            let cell_risk = pressure_norm * 0.6 + occupancy_risk * 0.4;

            total_risk += cell_risk;
            sample_count += 1;
        }

        let avg_risk = if sample_count > 0 { total_risk / sample_count as f32 } else { 0.0 };

        avg_risk.clamp(0.0, 1.0)
    }
}

/// Full board snapshot for viewer overlay (v1.0 Match OS).
/// Contains complete heatmap data for real-time visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldBoardSnapshotExport {
    pub cols: u8,
    pub rows: u8,
    pub occupancy_total: Vec<f32>, // len = cols*rows (504 for 28×18)
    pub pressure_against_home: Vec<f32>, // len = cols*rows
    pub pressure_against_away: Vec<f32>, // len = cols*rows
    pub xgzone_map: Vec<f32>,      // Match OS v1.2: xG values per cell
}

/// Minimal board summary for MatchResult export.
/// This is intentionally small (no full heatmap dump in MatchResult by default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSummaryExport {
    pub cols: u8,
    pub rows: u8,
    pub cell_w_m: f32,
    pub cell_h_m: f32,

    // simple scalar summaries
    pub occupancy_max: u8,
    pub pressure_max_against_home: f32,
    pub pressure_max_against_away: f32,

    // optional: top hot cells (for quick replay summaries)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hottest_occupancy_cells: Vec<HotCellExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotCellExport {
    pub col: u8,
    pub row: u8,
    pub value: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    const CX: f32 = field::CENTER_X;
    const CY: f32 = field::CENTER_Y;

    #[test]
    fn test_snapshot_export_performance() {
        // Create realistic board with default spec
        let spec = FieldBoardSpec { cols: 28, rows: 18 };
        let mut board = FieldBoard::new(spec);

        // Populate with 22 players (fixed array as expected by the API)
        let positions: [(f32, f32); 22] = [
            (10.0, 0.0),
            (20.0, 0.0),
            (30.0, 0.0),
            (40.0, 0.0),
            (50.0, 0.0),
            (60.0, 0.0),
            (70.0, 0.0),
            (80.0, 0.0),
            (90.0, 0.0),
            (100.0, 0.0),
            (10.0, CY),
            (10.0, CY),
            (20.0, CY),
            (30.0, CY),
            (40.0, CY),
            (50.0, CY),
            (60.0, CY),
            (70.0, CY),
            (80.0, CY),
            (90.0, CY),
            (100.0, CY),
            (10.0, field::WIDTH_M),
        ];

        board.update_occupancy_from_positions_m(0, &positions);
        board.update_pressure_from_positions_m(0, &positions, None, 15.0);

        // Benchmark 1000 exports
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = board.to_snapshot_export();
        }
        let elapsed = start.elapsed();

        let avg_micros = elapsed.as_micros() / 1000;
        println!("Snapshot export: {}μs average", avg_micros);

        // Assert < 1ms (1000μs)
        assert!(avg_micros < 1000, "Too slow: {}μs average", avg_micros);
    }

    #[test]
    fn test_lane_risk_hint_performance() {
        let spec = FieldBoardSpec { cols: 28, rows: 18 };
        let mut board = FieldBoard::new(spec);

        // Populate with realistic scenario (11 home + 11 away)
        let positions: [(f32, f32); 22] = [
            // Home team (spread across defensive half)
            (CX, 17.0),
            (35.0, 10.0),
            (35.0, 24.0),
            (70.0, 10.0),
            (70.0, 24.0),
            (CX, CY),
            (40.0, CY),
            (65.0, CY),
            (25.0, CY),
            (80.0, CY),
            (CX, 50.0),
            // Away team (spread across attacking half)
            (CX, 51.0),
            (35.0, 44.0),
            (35.0, 58.0),
            (70.0, 44.0),
            (70.0, 58.0),
            (CX, CY),
            (40.0, CY),
            (65.0, CY),
            (25.0, CY),
            (80.0, CY),
            (CX, 17.0),
        ];

        board.update_occupancy_from_positions_m(0, &positions);
        board.update_pressure_from_positions_m(0, &positions, None, 15.0);

        // Benchmark 10,000 calls with varied pass lanes
        let start = std::time::Instant::now();
        for i in 0..10000 {
            let from = (10.0 + (i % 10) as f32, 20.0);
            let to = (90.0, 40.0 + (i % 10) as f32);
            let _ = board.lane_risk_hint(from, to, true);
        }
        let elapsed = start.elapsed();

        let avg_micros = elapsed.as_micros() / 10000;
        println!("Lane risk hint: {}μs average", avg_micros);

        // Assert < 50μs per call
        assert!(avg_micros < 50, "Too slow: {}μs", avg_micros);
    }

    #[test]
    fn test_lane_risk_hint_correctness() {
        let spec = FieldBoardSpec { cols: 28, rows: 18 };
        let mut board = FieldBoard::new(spec);

        // Scenario 1: Clear lane (no opponents)
        let clear_positions: [(f32, f32); 22] = [
            // Home team far left
            (10.0, 10.0),
            (10.0, 15.0),
            (10.0, 20.0),
            (10.0, 25.0),
            (10.0, 30.0),
            (10.0, 35.0),
            (10.0, 40.0),
            (10.0, 45.0),
            (10.0, 50.0),
            (10.0, 55.0),
            (10.0, 60.0),
            // Away team far right
            (95.0, 10.0),
            (95.0, 15.0),
            (95.0, 20.0),
            (95.0, 25.0),
            (95.0, 30.0),
            (95.0, 35.0),
            (95.0, 40.0),
            (95.0, 45.0),
            (95.0, 50.0),
            (95.0, 55.0),
            (95.0, 60.0),
        ];

        board.update_occupancy_from_positions_m(0, &clear_positions);
        board.update_pressure_from_positions_m(0, &clear_positions, None, 15.0);

        // Pass through middle should have low risk
        let clear_risk = board.lane_risk_hint((CX, 20.0), (CX, 50.0), true);
        println!("Clear lane risk: {}", clear_risk);

        // Scenario 2: Blocked lane (opponents in the way)
        let blocked_positions: [(f32, f32); 22] = [
            // Home team far left
            (10.0, 10.0),
            (10.0, 15.0),
            (10.0, 20.0),
            (10.0, 25.0),
            (10.0, 30.0),
            (10.0, 35.0),
            (10.0, 40.0),
            (10.0, 45.0),
            (10.0, 50.0),
            (10.0, 55.0),
            (10.0, 60.0),
            // Away defenders blocking the lane
            (CX, 25.0),
            (CX, 30.0),
            (CX, 35.0),
            (CX, 40.0),
            (CX, 45.0),
            (50.0, 35.0),
            (55.0, 35.0),
            (50.0, 30.0),
            (55.0, 30.0),
            (50.0, 40.0),
            (55.0, 40.0),
        ];

        board.update_occupancy_from_positions_m(0, &blocked_positions);
        board.update_pressure_from_positions_m(0, &blocked_positions, None, 15.0);

        // Same pass should now have high risk
        let blocked_risk = board.lane_risk_hint((CX, 20.0), (CX, 50.0), true);
        println!("Blocked lane risk: {}", blocked_risk);

        // Blocked lane should have significantly higher risk
        assert!(
            blocked_risk > clear_risk,
            "Blocked lane ({}) should have higher risk than clear lane ({})",
            blocked_risk,
            clear_risk
        );

        // Blocked lane should have moderate to high risk (> 0.3)
        assert!(blocked_risk > 0.3, "Blocked lane risk should be > 0.3, got {}", blocked_risk);

        // Clear lane should have low risk (< 0.2)
        assert!(clear_risk < 0.2, "Clear lane risk should be < 0.2, got {}", clear_risk);
    }

    // ========================================================================
    // P2.1-A: Coordinate Bounds Tests
    // ========================================================================

    #[test]
    fn test_field_bounds_valid_position() {
        let bounds = FieldBounds::standard();

        // Center field
        assert!(bounds.validate_position(CX, CY).is_ok());
        assert!(bounds.contains(CX, CY));

        // Corners
        assert!(bounds.validate_position(0.0, 0.0).is_ok());
        assert!(bounds.validate_position(field::LENGTH_M, field::WIDTH_M).is_ok());
        assert!(bounds.validate_position(0.0, field::WIDTH_M).is_ok());
        assert!(bounds.validate_position(field::LENGTH_M, 0.0).is_ok());

        // Edges
        assert!(bounds.validate_position(CX, 0.0).is_ok());
        assert!(bounds.validate_position(CX, field::WIDTH_M).is_ok());
        assert!(bounds.validate_position(0.0, CY).is_ok());
        assert!(bounds.validate_position(field::LENGTH_M, CY).is_ok());
    }

    #[test]
    fn test_field_bounds_out_of_x() {
        let bounds = FieldBounds::standard();

        // X too large
        let result = bounds.validate_position(110.0, CY);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("x=110.00 out of bounds"));        
        assert!(!bounds.contains(110.0, CY));

        // X negative
        let result = bounds.validate_position(-5.0, CY);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("x=-5.00 out of bounds"));
        assert!(!bounds.contains(-5.0, CY));
    }

    #[test]
    fn test_field_bounds_out_of_y() {
        let bounds = FieldBounds::standard();

        // Y too large
        let result = bounds.validate_position(CX, 75.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("y=75.00 out of bounds"));
        assert!(!bounds.contains(CX, 75.0));

        // Y negative
        let result = bounds.validate_position(CX, -10.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("y=-10.00 out of bounds"));        
        assert!(!bounds.contains(CX, -10.0));
    }

    #[test]
    fn test_field_bounds_clamp_position() {
        let bounds = FieldBounds::standard();

        // Clamp out-of-bounds X
        let (x, y) = bounds.clamp_position(110.0, CY);
        assert_eq!(x, field::LENGTH_M);
        assert_eq!(y, CY);

        let (x, y) = bounds.clamp_position(-5.0, CY);
        assert_eq!(x, 0.0);
        assert_eq!(y, CY);

        // Clamp out-of-bounds Y
        let (x, y) = bounds.clamp_position(CX, 75.0);
        assert_eq!(x, CX);
        assert_eq!(y, field::WIDTH_M);

        let (x, y) = bounds.clamp_position(CX, -10.0);
        assert_eq!(x, CX);
        assert_eq!(y, 0.0);

        // Clamp both out-of-bounds
        let (x, y) = bounds.clamp_position(120.0, 80.0);
        assert_eq!(x, field::LENGTH_M);
        assert_eq!(y, field::WIDTH_M);

        // In-bounds should not change
        let (x, y) = bounds.clamp_position(CX, CY);
        assert_eq!(x, CX);
        assert_eq!(y, CY);
    }

    #[test]
    fn test_field_bounds_default() {
        let bounds = FieldBounds::default();
        assert_eq!(bounds.x_min, 0.0);
        assert_eq!(bounds.x_max, field::LENGTH_M);
        assert_eq!(bounds.y_min, 0.0);
        assert_eq!(bounds.y_max, field::WIDTH_M);
    }
}
