//! Coordinate system and distance calculations
//!
//! Converts between normalized (0-1) and meter coordinates.
//!
//! ## Coordinate Systems
//!
//! **Normalized/Waypoint Coordinates** (used in formations, player_positions):
//! - X: 0 = left touchline, 1 = right touchline (WIDTH direction, 68m)
//! - Y: 0 = own goal line, 1 = opponent goal line (LENGTH direction, 105m)
//!
//! **Meter/Field Coordinates** (used in physics, distance calculations):
//! - X: 0 = home goal, 105 = away goal (LENGTH direction)
//! - Y: 0 = touchline, 68 = touchline (WIDTH direction)
//!
//! The conversion functions swap X/Y to bridge these two systems.

use super::physics_constants::{field, goal};

/// Position in normalized coordinates (0-1)
/// - .0 = width (left-right, 0-1 maps to 0-68m)
/// - .1 = length (goal-goal, 0-1 maps to 0-105m)
///
/// Prefer `norm_width()` / `norm_length()` over `.0` / `.1` in new code to avoid axis confusion.
pub type NormalizedPos = (f32, f32);

/// Position in meters
/// - .0 = length (x-axis, 0-105m, home goal to away goal)
/// - .1 = width (y-axis, 0-68m, touchline to touchline)
pub type MeterPos = (f32, f32);

/// Third boundaries in normalized length units (TeamView semantics).
///
/// - 105m * (1/3) = 35m
/// - 105m * (2/3) = 70m
///
/// NOTE: Use exact fractions (not 0.33/0.66) to keep left/right symmetry.
const TWO_THIRDS_NORM: f32 = 2.0 / 3.0;

/// Convert normalized position to meters
///
/// Swaps X/Y because normalized uses (width, length) but meters uses (length, width).
pub fn to_meters(pos: NormalizedPos) -> MeterPos {
    // normalized.0 (width) -> meters.1 (y-axis, width)
    // normalized.1 (length) -> meters.0 (x-axis, length)
    (pos.1 * field::LENGTH_M, pos.0 * field::WIDTH_M)
}

/// Convert normalized position to meters with bounds enforcement
///
/// P0.75: Ball Coordinate Normalization Contract
/// Ensures the position is clamped to valid field bounds before and after conversion.
pub fn to_meters_clamped(pos: NormalizedPos) -> MeterPos {
    // First enforce normalized bounds (0-1)
    let normalized_safe = enforce_boundaries(pos);
    // Convert to meters
    let meters = to_meters(normalized_safe);
    // Defensive clamp at meter level (should be redundant but safe)
    (meters.0.clamp(0.0, field::LENGTH_M), meters.1.clamp(0.0, field::WIDTH_M))
}

/// Convert meter position to normalized
///
/// Swaps X/Y because normalized uses (width, length) but meters uses (length, width).
pub fn to_normalized(pos: MeterPos) -> NormalizedPos {
    // meters.0 (x-axis, length) -> normalized.1 (length)
    // meters.1 (y-axis, width) -> normalized.0 (width)
    (pos.1 / field::WIDTH_M, pos.0 / field::LENGTH_M)
}

/// Convert a world-normalized position into TeamView-normalized position.
///
/// TeamView semantics:
/// - length (pos.1): 0.0 = own goal line, 1.0 = opponent goal line (always forward)
/// - width (pos.0): unchanged
#[inline]
pub fn to_team_view_normalized(pos: NormalizedPos, attacks_right: bool) -> NormalizedPos {
    if attacks_right {
        pos
    } else {
        (pos.0, 1.0 - pos.1)
    }
}

/// Convert a world-normalized position into TeamView meters.
///
/// TeamView meters:
/// - x: 0..105m (0=own goal, 105=opponent goal)
/// - y: 0..68m (touchline..touchline)
#[inline]
pub fn to_team_view_meters(pos: NormalizedPos, attacks_right: bool) -> MeterPos {
    let world = to_meters(pos);
    if attacks_right {
        world
    } else {
        (field::LENGTH_M - world.0, world.1)
    }
}

/// Convert world meters X to TeamView meters X.
///
/// TeamView: 0 = own goal, 105 = opponent goal (always attacking toward higher X)
#[inline]
pub fn x_to_team_view_m(world_x: f32, attacks_right: bool) -> f32 {
    if attacks_right {
        world_x
    } else {
        field::LENGTH_M - world_x
    }
}

/// Calculate distance between two normalized positions in meters
pub fn distance_between_m(a: NormalizedPos, b: NormalizedPos) -> f32 {
    let a_m = to_meters(a);
    let b_m = to_meters(b);

    let dx = b_m.0 - a_m.0;
    let dy = b_m.1 - a_m.1;

    (dx * dx + dy * dy).sqrt()
}

/// Calculate distance from player to goal in meters
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
/// - attacks_right=true: attacking goal at x=105m (away goal)
/// - attacks_right=false: attacking goal at x=0m (home goal)
pub fn distance_to_goal_m(pos: NormalizedPos, attacks_right: bool) -> f32 {     
    let pos_m = to_team_view_meters(pos, attacks_right);

    // In TeamView, the attacking goal is always at x=field::LENGTH_M.
    let goal_x = field::LENGTH_M;
    let goal_y = field::WIDTH_M / 2.0;

    let dx = goal_x - pos_m.0;
    let dy = goal_y - pos_m.1;

    (dx * dx + dy * dy).sqrt()
}

/// Calculate angle to goal center (radians)
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
pub fn angle_to_goal(pos: NormalizedPos, attacks_right: bool) -> f32 {
    let pos_m = to_team_view_meters(pos, attacks_right);

    // In TeamView, the attacking goal is always at x=field::LENGTH_M.
    let goal_x = field::LENGTH_M;
    let goal_y = field::WIDTH_M / 2.0;

    let dx = goal_x - pos_m.0;
    let dy = goal_y - pos_m.1;

    dy.atan2(dx)
}

/// Check if position is inside penalty area
///
/// FIX_2601: Parameter renamed from `attacking_home` to `attacks_right` for clarity.
/// Uses TeamView semantics:
/// - opponent penalty area is always near x=105m
pub fn is_in_penalty_area(pos: NormalizedPos, attacks_right: bool) -> bool {
    let pos_m = to_team_view_meters(pos, attacks_right);

    // Penalty area dimensions
    let penalty_depth = field::PENALTY_AREA_LENGTH_M;
    let penalty_width = 40.66; // Standard width

    let y_min = (field::WIDTH_M - penalty_width) / 2.0;
    let y_max = y_min + penalty_width;

    // In TeamView, the opponent penalty area is always near x=field::LENGTH_M.
    pos_m.0 >= (field::LENGTH_M - penalty_depth) && pos_m.1 >= y_min && pos_m.1 <= y_max
}

/// Enforce field boundaries on position
pub fn enforce_boundaries(pos: NormalizedPos) -> NormalizedPos {
    (pos.0.clamp(0.0, 1.0), pos.1.clamp(0.0, 1.0))
}

/// Check if meter position is out of field bounds
///
/// P0.75: Out-of-bounds detection for telemetry
pub fn is_out_of_bounds_m(pos: MeterPos) -> bool {
    pos.0 < 0.0 || pos.0 > field::LENGTH_M || pos.1 < 0.0 || pos.1 > field::WIDTH_M
}

// ========== Normalized Position Accessors ==========
// These helpers make coordinate semantics explicit and prevent confusion.
// Normalized coords: pos.0 = width (sideline), pos.1 = length (goal direction)

/// Get the length component (goal direction) from normalized position
/// - 0.0 = own goal line
/// - 1.0 = opponent goal line
#[inline]
pub fn norm_length(pos: NormalizedPos) -> f32 {
    pos.1
}

/// Get the width component (sideline direction) from normalized position
/// - 0.0 = left touchline
/// - 1.0 = right touchline
#[inline]
pub fn norm_width(pos: NormalizedPos) -> f32 {
    pos.0
}

/// Check if position is in attacking third
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
/// Uses TeamView semantics:
/// - attacking third is length > 2/3 (x > 70m)
#[inline]
pub fn is_in_attacking_third(pos: NormalizedPos, attacks_right: bool) -> bool {
    let tv = to_team_view_normalized(pos, attacks_right);
    norm_length(tv) > TWO_THIRDS_NORM
}

/// Check if position is in own half
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
/// Uses TeamView semantics:
/// - own half is length < 0.5
#[inline]
pub fn is_in_own_half(pos: NormalizedPos, attacks_right: bool) -> bool {        
    let tv = to_team_view_normalized(pos, attacks_right);
    norm_length(tv) < 0.5
}

/// Check if position is in opponent half
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
#[inline]
pub fn is_in_opponent_half(pos: NormalizedPos, attacks_right: bool) -> bool {
    !is_in_own_half(pos, attacks_right)
}

/// Check if a pass/move is advancing toward opponent goal
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
/// Uses TeamView semantics:
/// - advancing when to.length > from.length (always forward)
#[inline]
pub fn is_advancing(from: NormalizedPos, to: NormalizedPos, attacks_right: bool) -> bool {
    let from_tv = to_team_view_normalized(from, attacks_right);
    let to_tv = to_team_view_normalized(to, attacks_right);
    norm_length(to_tv) > norm_length(from_tv)
}

/// Get the attacking goal's length coordinate (normalized)
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
/// - attacks_right=true: attacking goal at length=1.0 (x=105m)
/// - attacks_right=false: attacking goal at length=0.0 (x=0m)
#[inline]
pub fn attacking_goal_length(attacks_right: bool) -> f32 {
    if attacks_right {
        1.0
    } else {
        0.0
    }
}

/// Get the defending goal's length coordinate (normalized)
///
/// FIX_2601: Parameter renamed from `is_home` to `attacks_right` for clarity.
/// - attacks_right=true: defending goal at length=0.0 (x=0m)
/// - attacks_right=false: defending goal at length=1.0 (x=105m)
#[inline]
pub fn defending_goal_length(attacks_right: bool) -> f32 {
    if attacks_right {
        0.0
    } else {
        1.0
    }
}

/// Calculate squared distance between two normalized positions
/// (For comparison without sqrt overhead)
#[inline]
pub fn distance_squared_norm(a: NormalizedPos, b: NormalizedPos) -> f32 {
    let dw = b.0 - a.0;
    let dl = b.1 - a.1;
    dw * dw + dl * dl
}

/// Calculate distance between two normalized positions (in normalized units)
#[inline]
pub fn distance_norm(a: NormalizedPos, b: NormalizedPos) -> f32 {
    distance_squared_norm(a, b).sqrt()
}

// ============================================================
// FIX_2601/0106: Generic Distance Utilities
// 중복된 거리 계산 로직을 통합
// ============================================================

/// Generic distance between two meter positions
///
/// Use this instead of inline `((x2-x1).powi(2) + (y2-y1).powi(2)).sqrt()`.
///
/// # Example
/// ```ignore
/// let dist = distance_m((10.0, 20.0), (30.0, 40.0));
/// assert!((dist - 28.28).abs() < 0.1);
/// ```
#[inline]
pub fn distance_m(a: MeterPos, b: MeterPos) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}

/// Squared distance between two meter positions
///
/// Use for comparisons to avoid sqrt overhead.
/// ```ignore
/// // Instead of: if distance_m(a, b) < 5.0
/// // Use: if distance_squared_m(a, b) < 25.0
/// ```
#[inline]
pub fn distance_squared_m(a: MeterPos, b: MeterPos) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    dx * dx + dy * dy
}

/// Calculate magnitude (length) of a 2D vector
///
/// Use instead of inline `(dx.powi(2) + dy.powi(2)).sqrt()`.
#[inline]
pub fn magnitude(v: (f32, f32)) -> f32 {
    (v.0 * v.0 + v.1 * v.1).sqrt()
}

/// Calculate squared magnitude of a 2D vector
///
/// Use for comparisons to avoid sqrt overhead.
#[inline]
pub fn magnitude_squared(v: (f32, f32)) -> f32 {
    v.0 * v.0 + v.1 * v.1
}

/// Normalize a 2D vector to unit length
///
/// Returns (0.0, 0.0) for zero-length vectors.
#[inline]
pub fn normalize_vec(v: (f32, f32)) -> (f32, f32) {
    let mag = magnitude(v);
    if mag < 1e-6 {
        (0.0, 0.0)
    } else {
        (v.0 / mag, v.1 / mag)
    }
}

/// Calculate goal post positions in normalized coordinates
///
/// FIX_2601: Parameter renamed from `is_home_attacking` to `attacks_right` for clarity.
/// - attacks_right=true: goal posts at length=1.0 (x=105m)
/// - attacks_right=false: goal posts at length=0.0 (x=0m)
pub fn get_goal_posts(attacks_right: bool) -> (NormalizedPos, NormalizedPos) {
    let goal_half_width_norm = (goal::WIDTH_M / 2.0) / field::WIDTH_M;
    let center_y = 0.5;

    let x = if attacks_right { 1.0 } else { 0.0 };

    ((x, center_y - goal_half_width_norm), (x, center_y + goal_half_width_norm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    /// Test coordinate system conversion
    ///
    /// Normalized coords: (width 0-1, length 0-1)
    /// - width: 0 = left touchline, 1 = right touchline (68m)
    /// - length: 0 = own goal, 1 = opponent goal (105m)
    ///
    /// Meter coords: (length 0-105, width 0-68)
    /// - length (x): 0 = home goal, 105 = away goal
    /// - width (y): 0 = bottom touchline, 68 = top touchline
    #[test]
    fn test_to_meters() {
        // Center of field: normalized (0.5, 0.5)
        // After swap: meters (0.5*105, 0.5*68) = (CENTER_X, CENTER_Y)
        let pos = (0.5, 0.5);
        let m = to_meters(pos);
        assert!(
            (m.0 - field::CENTER_X).abs() < 0.1,
            "meters.x should be {} (center length), got {}",
            field::CENTER_X,
            m.0
        );
        assert!(
            (m.1 - field::CENTER_Y).abs() < 0.1,
            "meters.y should be {} (center width), got {}",
            field::CENTER_Y,
            m.1
        );
    }

    #[test]
    fn test_distance_between() {
        // Two points at same width (y=0.5) but different length (x=0 and x=1)
        // normalized (0.5, 0.0) -> meters (0.0, 34) = home goal line, center
        // normalized (0.5, 1.0) -> meters (105, 34) = away goal line, center
        // Distance should be 105m (full length)
        let a = (0.5, 0.0); // home goal line, center width
        let b = (0.5, 1.0); // away goal line, center width
        let dist = distance_between_m(a, b);
        assert!(
            (dist - field::LENGTH_M).abs() < 0.1,
            "Distance should be field length, got {}",
            dist
        );
    }

    #[test]
    fn test_distance_to_goal() {
        // Player at center: normalized (0.5, 0.5)
        // After swap: meters (CENTER_X, CENTER_Y) = center of field
        // Home team attacks towards x=105, so distance = 105 - CENTER_X = CENTER_X
        let pos = (0.5, 0.5);
        let dist = distance_to_goal_m(pos, true);
        assert!(
            (dist - field::CENTER_X).abs() < 0.1,
            "Distance to goal should be {}m, got {}",
            field::CENTER_X,
            dist
        );
    }

    #[test]
    fn test_penalty_area() {
        // Near opponent goal line: normalized (0.5, 0.95)
        // After swap: meters (0.95*105, 0.5*68) = (99.75, 34) - near away goal, center width
        // This should be in home team's attacking penalty area
        let pos_in = (0.5, 0.95);
        assert!(
            is_in_penalty_area(pos_in, true),
            "Position near opponent goal should be in penalty area"
        );

        // Midfield: normalized (0.5, 0.5) -> meters (CENTER_X, CENTER_Y)
        let pos_out = (0.5, 0.5);
        assert!(!is_in_penalty_area(pos_out, true), "Midfield should not be in penalty area");
    }

    // ========== P0.75: Ball Coordinate Normalization Tests ==========

    #[test]
    fn test_to_meters_clamped_valid_position() {
        // Valid center position should pass through unchanged
        let pos = (0.5, 0.5);
        let meters = to_meters_clamped(pos);
        assert!((meters.0 - field::CENTER_X).abs() < 0.1);
        assert!((meters.1 - field::CENTER_Y).abs() < 0.1);
    }

    #[test]
    fn test_to_meters_clamped_out_of_bounds() {
        // Out of bounds position should be clamped
        let pos = (1.5, -0.2); // Way out of bounds
        let meters = to_meters_clamped(pos);

        // Should be clamped to field edges
        assert!(
            meters.0 >= 0.0 && meters.0 <= field::LENGTH_M,
            "x should be clamped to field length"
        );
        assert!(
            meters.1 >= 0.0 && meters.1 <= field::WIDTH_M,
            "y should be clamped to field width"
        );
    }

    #[test]
    fn test_is_out_of_bounds_m() {
        // Valid positions
        assert!(!is_out_of_bounds_m((field::CENTER_X, field::CENTER_Y))); // Center
        assert!(!is_out_of_bounds_m((0.0, 0.0))); // Corner
        assert!(!is_out_of_bounds_m((field::LENGTH_M, field::WIDTH_M))); // Opposite corner

        // Out of bounds positions
        assert!(is_out_of_bounds_m((-1.0, field::CENTER_Y))); // Beyond left goal
        assert!(is_out_of_bounds_m((110.0, field::CENTER_Y))); // Beyond right goal
        assert!(is_out_of_bounds_m((field::CENTER_X, -5.0))); // Below touchline
        assert!(is_out_of_bounds_m((field::CENTER_X, 75.0))); // Above touchline
    }

    #[test]
    fn test_enforce_boundaries_idempotent() {
        // Clamping should be idempotent (applying twice = applying once)
        let pos = (1.5, -0.2);
        let once = enforce_boundaries(pos);
        let twice = enforce_boundaries(once);
        assert_eq!(once, twice);
    }

    // ========== P0.75: Property-Based Tests (Phase 1.4) ==========

    #[cfg(all(test, feature = "proptest"))]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Property: Any normalized position converts to valid meter coordinates
            #[test]
            fn prop_to_meters_always_in_bounds(
                x in -10.0f32..10.0f32,
                y in -10.0f32..10.0f32
            ) {
                let meters = to_meters_clamped((x, y));
                prop_assert!(meters.0 >= 0.0 && meters.0 <= field::LENGTH_M);
                prop_assert!(meters.1 >= 0.0 && meters.1 <= field::WIDTH_M);
            }

            /// Property: Clamping is idempotent
            #[test]
            fn prop_clamp_idempotent(
                x in -100.0f32..200.0f32,
                y in -100.0f32..200.0f32
            ) {
                let once = to_meters_clamped((x / field::WIDTH_M, y / field::LENGTH_M));
                let twice = to_meters_clamped(to_normalized(once));
                prop_assert!((once.0 - twice.0).abs() < 0.1);
                prop_assert!((once.1 - twice.1).abs() < 0.1);
            }

            /// Property: enforce_boundaries always returns valid normalized coords
            #[test]
            fn prop_enforce_boundaries_valid(
                x in -10.0f32..10.0f32,
                y in -10.0f32..10.0f32
            ) {
                let clamped = enforce_boundaries((x, y));
                prop_assert!(clamped.0 >= 0.0 && clamped.0 <= 1.0);
                prop_assert!(clamped.1 >= 0.0 && clamped.1 <= 1.0);
            }
        }
    }
}
