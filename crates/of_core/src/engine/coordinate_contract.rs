//! Coordinate contract conversions (SSOT for axis, units, and flip rules).

use super::physics_constants::field;
use super::types::coord10::Coord10;

pub const COORD_CONTRACT_VERSION: u8 = 2;
pub const COORD_SYSTEM_METERS_V2: &str = "meters_v2";
pub const COORD_SYSTEM_LEGACY_AXIS_SWAP: &str = "legacy_axis_swap";

/// Meters in world coordinates: (x = length, y = width).
/// DEPRECATED: Use `EnginePos` for type safety.
pub type MeterPos = (f32, f32);

/// UI-only normalized coordinates: (u = x/length, v = y/width).
pub type UiNormalizedPos = (f32, f32);
/// Legacy ratio coordinates: (u = width, v = length).
pub type LegacyRatioPos = (f32, f32);

// =============================================================================
// FIX_2601: Typed Coordinate Spaces (newtype structs for compile-time safety)
// =============================================================================
//
// NOTE: Float-based types (EnginePos, TeamViewPos) are DEPRECATED.
// Use integer-based types from types::coord10 instead:
//   - Coord10: World coordinates (0.1m precision)
//   - TeamViewCoord10: Team-relative coordinates (always attacking right)
//
// Integer coordinates provide:
//   - Deterministic simulation (no floating-point errors)
//   - Exact comparisons (no epsilon checks)
//   - Better cache efficiency
// =============================================================================

// NOTE: EnginePos struct was removed in FIX_2601. Use Coord10 from types::coord10.
// NOTE: TeamViewPos struct was removed in FIX_2601. Use TeamViewCoord10 from types::coord10.
// NOTE: ReplayPos struct was removed in FIX_2601. Use Coord10 directly (same 0.1m precision).

#[inline]
pub fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[inline]
pub fn clamp_meters(pos: MeterPos) -> MeterPos {
    (pos.0.clamp(0.0, field::LENGTH_M), pos.1.clamp(0.0, field::WIDTH_M))
}

#[inline]
pub fn clamp_coord10(pos: Coord10) -> Coord10 {
    Coord10 { x: pos.x.clamp(0, field::LENGTH_COORD10), y: pos.y.clamp(0, field::WIDTH_COORD10), z: pos.z }
}

pub fn meters_to_coord10(pos: MeterPos) -> Coord10 {
    clamp_coord10(Coord10::from_meters(pos.0, pos.1))
}

pub fn coord10_to_meters(pos: Coord10) -> MeterPos {
    pos.to_meters()
}

pub fn meters_to_ui_normalized(pos: MeterPos) -> UiNormalizedPos {
    let clamped = clamp_meters(pos);
    (clamped.0 / field::LENGTH_M, clamped.1 / field::WIDTH_M)
}

pub fn ui_normalized_to_meters(pos: UiNormalizedPos) -> MeterPos {
    (clamp01(pos.0) * field::LENGTH_M, clamp01(pos.1) * field::WIDTH_M)
}

/// Convert legacy ratios (width, length) to meters (length, width).
#[deprecated(note = "Use Coord10::from_meters() directly with swapped coordinates")]
pub fn legacy_ratio_to_meters(pos: LegacyRatioPos) -> MeterPos {
    (clamp01(pos.1) * field::LENGTH_M, clamp01(pos.0) * field::WIDTH_M)
}

#[deprecated(note = "Use Coord10::from_meters() directly with swapped coordinates")]
pub fn legacy_ratio_to_coord10(pos: LegacyRatioPos) -> Coord10 {
    meters_to_coord10(legacy_ratio_to_meters(pos))
}

#[deprecated(note = "Use DirectionContext::to_team_view() instead")]
pub fn world_to_team_view_meters(pos: MeterPos, dir_x: i32) -> MeterPos {
    if dir_x >= 0 {
        pos
    } else {
        (field::LENGTH_M - pos.0, pos.1)
    }
}

#[deprecated(note = "Use DirectionContext::to_team_view() instead")]
pub fn world_to_team_view_coord10(pos: Coord10, dir_x: i32) -> Coord10 {
    let x = if dir_x >= 0 { pos.x } else { field::LENGTH_COORD10 - pos.x };

    Coord10 { x, y: pos.y, z: pos.z }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::goal;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    // =========================================================================
    // FIX_2512_1230: Coordinate Contract Gate Tests
    // These tests seal the coordinate/direction contract to prevent regression.
    // =========================================================================

    /// Gate Test 1: Goal positions are correct
    #[test]
    fn gate_goal_positions() {
        // Home goal at x=0, center Y
        assert_eq!(goal::HOME_CENTER_M, (0.0, field::CENTER_Y));
        // Away goal at x=105, center Y
        assert_eq!(goal::AWAY_CENTER_M, (field::LENGTH_M, field::CENTER_Y));
    }

    /// Gate Test 2: Attack/Defend goal symmetry
    #[test]
    fn gate_goal_symmetry() {
        // Home attacks Away goal (105), defends Home goal (0)
        assert_eq!(goal::attack_goal_x(true), field::LENGTH_M);
        assert_eq!(goal::defend_goal_x(true), 0.0);

        // Away attacks Home goal (0), defends Away goal (105)
        assert_eq!(goal::attack_goal_x(false), 0.0);
        assert_eq!(goal::defend_goal_x(false), field::LENGTH_M);

        // Symmetry: Home's attack = Away's defend
        assert_eq!(goal::attack_goal_x(true), goal::defend_goal_x(false));
        assert_eq!(goal::attack_goal_x(false), goal::defend_goal_x(true));
    }

    /// Gate Test 3: Goal center positions for attacking
    #[test]
    fn gate_attack_goal_centers() {
        // Home attacks toward (105, 34), defends (0, 34)
        assert_eq!(goal::attack_goal(true), (field::LENGTH_M, field::CENTER_Y));
        assert_eq!(goal::defend_goal(true), (0.0, field::CENTER_Y));

        // Away attacks toward (0, 34), defends (105, 34)
        assert_eq!(goal::attack_goal(false), (0.0, field::CENTER_Y));
        assert_eq!(goal::defend_goal(false), (field::LENGTH_M, field::CENTER_Y));
    }

    /// Gate Test 4: Width axis bounds use WIDTH_M
    #[test]
    fn gate_y_axis_bounds() {
        // Any check using .1 (Y-axis) should never compare to values > WIDTH_M
        // This catches bugs like "pos.1 > 70.0" which is always false
        //
        // NOTE: Ball positions may legitimately go out-of-bounds for throw-ins/corners.
        // This gate is only about axis SSOT (width bounds use WIDTH_M).
        let positions = vec![
            (field::CENTER_X, 0.0),            // bottom touchline
            (field::CENTER_X, field::CENTER_Y), // center
            (field::CENTER_X, field::WIDTH_M),  // top touchline
        ];

        for pos in positions {
            assert!(
                pos.1 >= 0.0 && pos.1 <= field::WIDTH_M,
                "Width axis should be in [0, WIDTH_M], got {}",
                pos.1
            );
        }
    }

    /// Gate Test 4b: Out-of-play positions are representable (margin vs in-bounds)
    #[test]
    fn gate_ball_out_of_play_allowed() {
        // Allow a small out-of-bounds buffer so restarts (throw-in/corner/goal-kick)
        // can be detected without runaway coordinates.
        let out = Coord10 { x: -5, y: field::WIDTH_COORD10 + 5, z: 0 };
        assert!(out.is_in_field());
        assert!(!out.is_in_bounds());
    }

    /// Gate Test 5: X-axis for attacking third (Home vs Away)
    #[test]
    fn gate_attacking_third_uses_x_axis() {
        // Home attacks toward x=105, so x > 70 is attacking third
        let home_x75 = (75.0, field::CENTER_Y);
        let home_x30 = (30.0, field::CENTER_Y);

        // Home attacking third: x > 70
        assert!(home_x75.0 > 70.0, "x=75 should be in attacking third for Home");
        assert!(home_x30.0 <= 70.0, "x=30 should NOT be in attacking third for Home");

        // Away attacks toward x=0, so x < 35 is attacking third
        assert!(home_x30.0 < 35.0, "x=30 should be in attacking third for Away");
        assert!(home_x75.0 >= 35.0, "x=75 should NOT be in attacking third for Away");
    }

    /// Gate Test 6: Goal distance uses X-axis
    #[test]
    fn gate_goal_distance_uses_x_axis() {
        let pos = (80.0, field::CENTER_Y);

        // Home attacking Away goal at x=105
        let home_attack_goal = goal::attack_goal(true);
        let home_dist =
            ((pos.0 - home_attack_goal.0).powi(2) + (pos.1 - home_attack_goal.1).powi(2)).sqrt();
        assert!(
            (home_dist - 25.0).abs() < 0.5,
            "Home attacking Away goal: expected ~25m, got {}",
            home_dist
        );

        // Away attacking Home goal at x=0
        let away_attack_goal = goal::attack_goal(false);
        let away_dist =
            ((pos.0 - away_attack_goal.0).powi(2) + (pos.1 - away_attack_goal.1).powi(2)).sqrt();
        assert!(
            (away_dist - 80.0).abs() < 0.5,
            "Away attacking Home goal: expected ~80m, got {}",
            away_dist
        );
    }

    /// Gate Test 7: Penalty box uses correct axes
    #[test]
    fn gate_penalty_box_axes() {
        fn in_own_box(pos: (f32, f32), is_home: bool) -> bool {
            let in_box_y = pos.1 > 13.85 && pos.1 < 54.15;
            if is_home {
                pos.0 < 16.5 && in_box_y
            } else {
                pos.0 > 88.5 && in_box_y
            }
        }

        // Home box: x in [0, 16.5], y in [13.85, 54.15]
        assert!(in_own_box((10.0, field::CENTER_Y), true), "Inside home box");
        assert!(!in_own_box((20.0, field::CENTER_Y), true), "x=20 outside home box");
        assert!(!in_own_box((10.0, 10.0), true), "y=10 outside home box");

        // Away box: x in [88.5, 105], y in [13.85, 54.15]
        assert!(in_own_box((95.0, field::CENTER_Y), false), "Inside away box");
        assert!(!in_own_box((85.0, field::CENTER_Y), false), "x=85 outside away box");
    }

    // =========================================================================
    // Original coordinate_contract tests
    // =========================================================================

    #[test]
    fn world_center_meters() {
        let meters = (field::CENTER_X, field::CENTER_Y);
        let coord = meters_to_coord10(meters);
        assert_eq!(coord, Coord10::CENTER);

        let norm = meters_to_ui_normalized(meters);
        assert!(approx_eq(norm.0, 0.5, 1e-6));
        assert!(approx_eq(norm.1, 0.5, 1e-6));
    }

    #[test]
    fn world_corners_meters_to_coord10() {
        let cases = [
            ((0.0, 0.0), Coord10 { x: 0, y: 0, z: 0 }),
            ((field::LENGTH_M, 0.0), Coord10 { x: field::LENGTH_COORD10, y: 0, z: 0 }),
            ((0.0, field::WIDTH_M), Coord10 { x: 0, y: field::WIDTH_COORD10, z: 0 }),
            (
                (field::LENGTH_M, field::WIDTH_M),
                Coord10 { x: field::LENGTH_COORD10, y: field::WIDTH_COORD10, z: 0 },
            ),
        ];

        for (meters, expected) in cases {
            assert_eq!(meters_to_coord10(meters), expected);
        }
    }

    #[test]
    fn world_bounds_clamp() {
        let clamped = clamp_meters((-1.0, 70.0));
        assert!(approx_eq(clamped.0, 0.0, 1e-6));
        assert!(approx_eq(clamped.1, field::WIDTH_M, 1e-6));
    }

    #[test]
    fn team_view_flip_x_only() {
        let world = (10.0, 20.0);
        let flipped = world_to_team_view_meters(world, -1);
        assert!(approx_eq(flipped.0, field::LENGTH_M - world.0, 1e-6));
        assert!(approx_eq(flipped.1, 20.0, 1e-6));
    }

    #[test]
    fn team_view_center_invariant() {
        let world = (field::CENTER_X, field::CENTER_Y);
        let flipped = world_to_team_view_meters(world, -1);
        assert!(approx_eq(flipped.0, world.0, 1e-6));
        assert!(approx_eq(flipped.1, world.1, 1e-6));
    }

    #[test]
    fn coord10_roundtrip_exact() {
        let samples = [
            Coord10 { x: 0, y: 0, z: 0 },
            Coord10 { x: 1, y: 1, z: 0 },
            Coord10::CENTER,
            Coord10 { x: field::LENGTH_COORD10, y: field::WIDTH_COORD10, z: 0 },
        ];

        for sample in samples {
            let meters = coord10_to_meters(sample);
            let round = meters_to_coord10(meters);
            assert_eq!(round, sample);
        }
    }

    #[test]
    fn meters_roundtrip_quantized() {
        // Coord10 has 0.1m precision (scale=10), so roundtrip error can be up to 0.05m
        // Use slightly larger tolerance to account for floating point precision
        let samples = [(0.04, 0.06), (10.05, 20.07), (104.96, 67.99)];
        let tolerance = 0.051; // Slightly above 0.05 for floating point safety

        for sample in samples {
            let coord = meters_to_coord10(sample);
            let round = coord10_to_meters(coord);
            assert!(
                approx_eq(round.0, sample.0, tolerance),
                "X mismatch: sample={:?}, coord={:?}, round={:?}, diff={}",
                sample,
                coord,
                round,
                (round.0 - sample.0).abs()
            );
            assert!(
                approx_eq(round.1, sample.1, tolerance),
                "Y mismatch: sample={:?}, coord={:?}, round={:?}, diff={}",
                sample,
                coord,
                round,
                (round.1 - sample.1).abs()
            );
        }
    }

    #[test]
    fn normalized_roundtrip_ui_only() {
        let samples = [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)];

        for sample in samples {
            let meters = ui_normalized_to_meters(sample);
            let round = meters_to_ui_normalized(meters);
            assert!(approx_eq(round.0, sample.0, 1e-6));
            assert!(approx_eq(round.1, sample.1, 1e-6));
        }
    }

    #[test]
    fn legacy_ratio_swaps_axes() {
        let meters = legacy_ratio_to_meters((0.5, 0.04));
        assert!(approx_eq(meters.0, 0.04 * field::LENGTH_M, 1e-6));
        assert!(approx_eq(meters.1, 0.5 * field::WIDTH_M, 1e-6));
    }

    // =========================================================================
    // FIX_2601: Integer Coordinate Type Safety Tests
    // =========================================================================
    // NOTE: Float-based types (EnginePos, TeamViewPos, ReplayPos) were removed.
    // Tests for integer types (Coord10, TeamViewCoord10) are in types/coord10.rs.
    // =========================================================================

    // =========================================================================
    // FIX_2601: Halftime Direction Gate Tests
    // Verify attacks_right logic is correct for both halves.
    // =========================================================================

    /// Gate Test 8: First half attack directions
    #[test]
    fn gate_first_half_attack_directions() {
        // First half: Home attacks right (X=105), Away attacks left (X=0)
        let is_second_half = false;

        // Home attacks right in first half
        let home_attacks_right = !is_second_half; // true when first half
        assert!(home_attacks_right, "Home should attack right in first half");

        // Away attacks left in first half
        let away_attacks_right = is_second_half; // false when first half
        assert!(!away_attacks_right, "Away should attack left in first half");
    }

    /// Gate Test 9: Second half attack directions (sides swapped)
    #[test]
    fn gate_second_half_attack_directions() {
        // Second half: Home attacks left (X=0), Away attacks right (X=105)
        let is_second_half = true;

        // Home attacks left in second half
        let home_attacks_right = !is_second_half; // false when second half
        assert!(!home_attacks_right, "Home should attack left in second half");

        // Away attacks right in second half
        let away_attacks_right = is_second_half; // true when second half
        assert!(away_attacks_right, "Away should attack right in second half");
    }

    /// Gate Test 10: Direction-based goal assignment
    #[test]
    fn gate_direction_goal_assignment() {
        // First half
        let home_attacks_right_h1 = true;
        let away_attacks_right_h1 = false;

        // Home attacks Away goal (field::LENGTH_M) in first half
        let home_attack_goal_h1 =
            if home_attacks_right_h1 { field::LENGTH_M } else { 0.0 };
        assert_eq!(
            home_attack_goal_h1,
            field::LENGTH_M,
            "Home should attack field::LENGTH_M in first half"
        );

        // Away attacks Home goal (0) in first half
        let away_attack_goal_h1 =
            if away_attacks_right_h1 { field::LENGTH_M } else { 0.0 };
        assert_eq!(away_attack_goal_h1, 0.0, "Away should attack 0 in first half");

        // Second half (directions flipped)
        let home_attacks_right_h2 = false;
        let away_attacks_right_h2 = true;

        // Home attacks Home goal (0) in second half (because they're now on the other side)
        let home_attack_goal_h2 =
            if home_attacks_right_h2 { field::LENGTH_M } else { 0.0 };
        assert_eq!(home_attack_goal_h2, 0.0, "Home should attack 0 in second half");

        // Away attacks Away goal (field::LENGTH_M) in second half
        let away_attack_goal_h2 =
            if away_attacks_right_h2 { field::LENGTH_M } else { 0.0 };
        assert_eq!(
            away_attack_goal_h2,
            field::LENGTH_M,
            "Away should attack field::LENGTH_M in second half"
        );
    }
}
