/// threat_model.rs
/// Phase 1.2: ThreatModel - Calculate CarrierFreeScore
///
/// Purpose: Detect "free ball carriers" who need emergency pressing
///
/// CarrierFreeScore (0.0-1.0):
/// - High score = carrier is dangerous (free, facing goal, open lane)
/// - Low score = carrier is contained (pressure, blocked lanes)
///
/// Emergency Presser Trigger:
/// - If free_score >= threshold → Assign emergency presser (ignores cooldown)
use crate::engine::body_orientation::{
    facing_to_goal, facing_to_progress, is_frontal_pressure, is_rear_pressure, pressure_angle,
};

/// Weight constants for CarrierFreeScore formula
pub mod weights {
    pub const SPACE: f32 = 0.28; // Nearest defender distance
    pub const TIME: f32 = 0.22; // Time to intercept
    pub const LANE: f32 = 0.26; // Shot/forward lanes clear
    pub const FACING: f32 = 0.16; // Facing goal/progress
    pub const PRESSURE: f32 = 0.08; // Pressure angle (frontal/rear)
}

/// Tactic-specific thresholds for emergency presser trigger
pub mod thresholds {
    pub const BALANCED: f32 = 0.62;
    pub const HIGH_PRESS: f32 = 0.55; // More sensitive
    pub const LOW_BLOCK: f32 = 0.68; // Less sensitive
}

/// Calculate space term (nearest defender distance with log curve)
///
/// **Parameters**:
/// - `carrier_pos`: Ball carrier position (meters)
/// - `defender_positions`: All defender positions (meters)
///
/// **Returns**: 0.0 (defenders very close) to 1.0 (no defenders nearby)
pub fn calculate_space_term(carrier_pos: (f32, f32), defender_positions: &[(f32, f32)]) -> f32 {
    if defender_positions.is_empty() {
        return 1.0; // No defenders = max space
    }

    // Find nearest defender
    let nearest_dist = defender_positions
        .iter()
        .map(|&def_pos| {
            let dx = carrier_pos.0 - def_pos.0;
            let dy = carrier_pos.1 - def_pos.1;
            (dx * dx + dy * dy).sqrt()
        })
        .fold(f32::MAX, f32::min);

    // Log curve: 0m→0.0, 5m→0.5, 10m→0.75, 20m→1.0
    const MAX_DIST: f32 = 20.0;
    let normalized_dist = (nearest_dist / MAX_DIST).min(1.0);

    // Log scaling (asymptotic to 1.0)
    // k chosen so that with MAX_DIST=20m, f(5m)=0.5 (=> k=ln(16)).
    const K: f32 = 4.0 * std::f32::consts::LN_2;
    1.0 - (-K * normalized_dist).exp()
}

/// Calculate time term (time to intercept in seconds)
///
/// **Parameters**:
/// - `carrier_pos`: Ball carrier position
/// - `defender_positions`: Defender positions
/// - `defender_speeds`: Defender speeds (m/s)
/// - `carrier_speed`: Carrier speed (m/s)
///
/// **Returns**: 0.0 (instant intercept) to 1.0 (no intercept possible)
pub fn calculate_time_term(
    carrier_pos: (f32, f32),
    defender_positions: &[(f32, f32)],
    defender_speeds: &[f32],
    carrier_speed: f32,
) -> f32 {
    if defender_positions.is_empty() {
        return 1.0; // No defenders = infinite time
    }

    // Calculate time to intercept for each defender
    let min_intercept_time = defender_positions
        .iter()
        .zip(defender_speeds.iter())
        .map(|(&def_pos, &def_speed)| {
            let dist = {
                let dx = carrier_pos.0 - def_pos.0;
                let dy = carrier_pos.1 - def_pos.1;
                (dx * dx + dy * dy).sqrt()
            };

            // Simple chase model (ignores angles, conservative estimate)
            if def_speed <= carrier_speed {
                f32::MAX // Defender can't catch up
            } else {
                dist / (def_speed - carrier_speed)
            }
        })
        .fold(f32::MAX, f32::min);

    // Convert to 0..1 scale (0s→0.0, 2s→0.5, 4s→1.0)
    const MAX_TIME: f32 = 4.0;
    (min_intercept_time / MAX_TIME).min(1.0)
}

/// Calculate lane term (shot lane + forward lane clearance)
///
/// **Parameters**:
/// - `carrier_pos`: Ball carrier position
/// - `goal_pos`: Goal center position
/// - `forward_dir`: Forward progress direction (normalized)
/// - `defender_positions`: Defender positions
/// - `defender_body_dirs`: Defender body directions
///
/// **Returns**: 0.0 (lanes completely blocked) to 1.0 (lanes clear)
pub fn calculate_lane_term(
    carrier_pos: (f32, f32),
    goal_pos: (f32, f32),
    forward_dir: (f32, f32),
    defender_positions: &[(f32, f32)],
    defender_body_dirs: &[(f32, f32)],
) -> f32 {
    if defender_positions.is_empty() {
        return 1.0; // No defenders = clear lanes
    }

    // Shot lane (to goal)
    let shot_target = (goal_pos.0, goal_pos.1 + 10.0); // 10m ahead of goal (conservative)
    let shot_blocked =
        lane_blocked(carrier_pos, shot_target, defender_positions, defender_body_dirs);

    // Forward lane (progress direction)
    let forward_target =
        (carrier_pos.0 + forward_dir.0 * 15.0, carrier_pos.1 + forward_dir.1 * 15.0);
    let forward_blocked =
        lane_blocked(carrier_pos, forward_target, defender_positions, defender_body_dirs);

    // Both lanes clear = 1.0, one blocked = 0.5, both blocked = 0.0
    let shot_clear = if shot_blocked { 0.0 } else { 0.5 };
    let forward_clear = if forward_blocked { 0.0 } else { 0.5 };
    shot_clear + forward_clear
}

/// Check if a lane is blocked by any defender
///
/// **Blocking Criteria**:
/// - Defender within 5m of lane
/// - Defender body oriented toward lane (±60°)
///
/// **Parameters**:
/// - `from`: Lane start position
/// - `to`: Lane target position
/// - `defender_positions`: Defender positions
/// - `defender_body_dirs`: Defender body directions
///
/// **Returns**: true if lane is blocked
pub fn lane_blocked(
    from: (f32, f32),
    to: (f32, f32),
    defender_positions: &[(f32, f32)],
    defender_body_dirs: &[(f32, f32)],
) -> bool {
    const BLOCK_DISTANCE: f32 = 5.0; // 5m from lane
    const HARD_BLOCK_DISTANCE: f32 = 1.0; // Physical obstruction even if not facing lane
    const BLOCK_ANGLE: f32 = 60.0; // ±60° body angle tolerance

    for (i, &def_pos) in defender_positions.iter().enumerate() {
        // Distance from defender to lane (point-to-line)
        let dist_to_lane = point_to_line_distance(def_pos, from, to);

        if dist_to_lane <= HARD_BLOCK_DISTANCE {
            return true; // Physical obstruction even if not facing the lane
        }

        if dist_to_lane > BLOCK_DISTANCE {
            continue; // Too far to block
        }

        // Check if defender is facing lane
        let to_lane_center = ((from.0 + to.0) / 2.0 - def_pos.0, (from.1 + to.1) / 2.0 - def_pos.1);
        let body_dir = defender_body_dirs[i];
        let angle = crate::engine::body_orientation::angle_deg(body_dir, to_lane_center);

        if angle < BLOCK_ANGLE {
            return true; // Defender blocking lane
        }
    }

    false
}

/// Point-to-line distance (helper for lane blocking)
fn point_to_line_distance(point: (f32, f32), line_a: (f32, f32), line_b: (f32, f32)) -> f32 {
    let dx = line_b.0 - line_a.0;
    let dy = line_b.1 - line_a.1;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-6 {
        // Line is a point
        let pdx = point.0 - line_a.0;
        let pdy = point.1 - line_a.1;
        return (pdx * pdx + pdy * pdy).sqrt();
    }

    // Project point onto line
    let t = ((point.0 - line_a.0) * dx + (point.1 - line_a.1) * dy) / len_sq;
    let t_clamped = t.clamp(0.0, 1.0);

    let closest = (line_a.0 + t_clamped * dx, line_a.1 + t_clamped * dy);

    let pdx = point.0 - closest.0;
    let pdy = point.1 - closest.1;
    (pdx * pdx + pdy * pdy).sqrt()
}

/// Calculate facing term (facing goal/progress bonus)
///
/// **Parameters**:
/// - `carrier_body_dir`: Carrier's body direction
/// - `carrier_pos`: Carrier position
/// - `attacks_right`: true if attacking right goal
///   FIX_2601/0110: Changed from is_home_team to attacks_right for correct 2nd half behavior
///
/// **Returns**: 0.0 (facing away) to 1.0 (facing goal+progress)
pub fn calculate_facing_term(
    carrier_body_dir: (f32, f32),
    carrier_pos: (f32, f32),
    attacks_right: bool,
) -> f32 {
    let facing_goal = facing_to_goal(carrier_body_dir, carrier_pos, attacks_right);
    let facing_progress = facing_to_progress(carrier_body_dir, attacks_right);

    // Average (both contribute equally)
    (facing_goal + facing_progress) / 2.0
}

/// Calculate pressure term (inverted pressure angle)
///
/// **Logic**:
/// - Frontal pressure (<35°) → 0.0 (reduces free score)
/// - Rear pressure (>120°) → 1.0 (increases free score)
/// - Side pressure → 0.5
///
/// **Parameters**:
/// - `carrier_pos`: Ball carrier position
/// - `carrier_body_dir`: Carrier's body direction
/// - `defender_positions`: Defender positions
///
/// **Returns**: 0.0 (frontal pressure) to 1.0 (rear/no pressure)
pub fn calculate_pressure_term(
    carrier_pos: (f32, f32),
    carrier_body_dir: (f32, f32),
    defender_positions: &[(f32, f32)],
) -> f32 {
    if defender_positions.is_empty() {
        return 1.0; // No pressure
    }

    // Find nearest defender
    let nearest_def = defender_positions
        .iter()
        .min_by(|a, b| {
            let dist_a = {
                let dx = carrier_pos.0 - a.0;
                let dy = carrier_pos.1 - a.1;
                dx * dx + dy * dy
            };
            let dist_b = {
                let dx = carrier_pos.0 - b.0;
                let dy = carrier_pos.1 - b.1;
                dx * dx + dy * dy
            };
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .copied()
        .unwrap();

    let angle = pressure_angle(nearest_def, carrier_pos, carrier_body_dir);

    if is_frontal_pressure(angle) {
        0.0 // Strong pressure (reduces free score)
    } else if is_rear_pressure(angle) {
        1.0 // Weak pressure (increases free score)
    } else {
        0.5 // Side pressure (neutral)
    }
}

/// Calculate CarrierFreeScore (main function)
///
/// **5-Term Formula**:
/// ```text
/// free_score = space * 0.28 + time * 0.22 + lane * 0.26 + facing * 0.16 + pressure * 0.08
/// ```
///
/// **Parameters**:
/// - `carrier_pos`: Ball carrier position (meters)
/// - `carrier_body_dir`: Carrier's body direction
/// - `carrier_speed`: Carrier speed (m/s)
/// - `attacks_right`: true if carrier is attacking right goal
///   FIX_2601/0110: Changed from is_home_team to attacks_right for correct 2nd half behavior
/// - `defender_positions`: All defender positions
/// - `defender_speeds`: Defender speeds
/// - `defender_body_dirs`: Defender body directions
/// - `goal_pos`: Goal center position
///
/// **Returns**: 0.0 (contained) to 1.0 (completely free)
#[allow(clippy::too_many_arguments)]
pub fn calculate_carrier_free_score(
    carrier_pos: (f32, f32),
    carrier_body_dir: (f32, f32),
    carrier_speed: f32,
    attacks_right: bool,
    defender_positions: &[(f32, f32)],
    defender_speeds: &[f32],
    defender_body_dirs: &[(f32, f32)],
    goal_pos: (f32, f32),
) -> f32 {
    let space = calculate_space_term(carrier_pos, defender_positions);
    let time = calculate_time_term(carrier_pos, defender_positions, defender_speeds, carrier_speed);

    // FIX_2601/0110: Use attacks_right for forward direction
    let forward_dir = if attacks_right { (1.0, 0.0) } else { (-1.0, 0.0) };
    let lane = calculate_lane_term(
        carrier_pos,
        goal_pos,
        forward_dir,
        defender_positions,
        defender_body_dirs,
    );

    let facing = calculate_facing_term(carrier_body_dir, carrier_pos, attacks_right);
    let pressure = calculate_pressure_term(carrier_pos, carrier_body_dir, defender_positions);

    // Weighted sum
    space * weights::SPACE
        + time * weights::TIME
        + lane * weights::LANE
        + facing * weights::FACING
        + pressure * weights::PRESSURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;
    const CY: f32 = field::CENTER_Y;

    #[test]
    fn test_space_term() {
        let carrier = (50.0, CY);

        // No defenders = max space
        let space_empty = calculate_space_term(carrier, &[]);
        assert_eq!(space_empty, 1.0);

        // Defender very close (1m)
        let defenders_close = vec![(51.0, CY)];
        let space_close = calculate_space_term(carrier, &defenders_close);
        assert!(space_close < 0.2); // Very low space

        // Defender far (20m+)
        let defenders_far = vec![(70.0, CY)];
        let space_far = calculate_space_term(carrier, &defenders_far);
        assert!(space_far > 0.9); // High space
    }

    #[test]
    fn test_time_term() {
        let carrier = (50.0, CY);
        let carrier_speed = 5.0;

        let defenders = vec![(40.0, CY)]; // 10m away
        let speeds = vec![7.0]; // Faster than carrier

        let time = calculate_time_term(carrier, &defenders, &speeds, carrier_speed);
        // time_to_intercept = 10 / (7-5) = 5s → normalized = 5/4 = 1.0 (capped)
        assert!((time - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_lane_blocked() {
        let from = (50.0, CY);
        let to = (60.0, CY);

        // Defender in lane, facing it
        let defenders = vec![(55.0, CY)];
        let body_dirs = vec![(1.0, 0.0)]; // Facing right (along lane)
        assert!(lane_blocked(from, to, &defenders, &body_dirs));

        // Defender far from lane
        let defenders_far = vec![(55.0, 50.0)]; // 16m away vertically
        assert!(!lane_blocked(from, to, &defenders_far, &body_dirs));
    }

    #[test]
    fn test_carrier_free_score() {
        let carrier_pos = (50.0, CY);
        let carrier_body_dir = (1.0, 0.0); // Facing right
        let carrier_speed = 6.0;
        let attacks_right = true; // FIX_2601/0110: Changed from is_home to attacks_right
        let goal_pos = (field::LENGTH_M, CY);

        // Scenario 1: No defenders (completely free)
        let free_score_empty = calculate_carrier_free_score(
            carrier_pos,
            carrier_body_dir,
            carrier_speed,
            attacks_right,
            &[],
            &[],
            &[],
            goal_pos,
        );
        assert!(free_score_empty > 0.8); // Should be very high

        // Scenario 2: Defender very close with frontal pressure
        let defenders = vec![(52.0, CY)];
        let speeds = vec![7.0];
        let body_dirs = vec![(-1.0, 0.0)]; // Facing carrier
        let free_score_pressured = calculate_carrier_free_score(
            carrier_pos,
            carrier_body_dir,
            carrier_speed,
            attacks_right,
            &defenders,
            &speeds,
            &body_dirs,
            goal_pos,
        );
        assert!(free_score_pressured < 0.5); // Should be much lower
    }
}
