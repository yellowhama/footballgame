/// body_orientation.rs
/// Phase 1.1: Body Orientation Model
///
/// Track player facing direction for pressure/threat calculations

const EPS: f32 = 1e-6;

/// Normalize a 2D vector
pub fn normalize(v: (f32, f32)) -> (f32, f32) {
    let mag = (v.0 * v.0 + v.1 * v.1).sqrt();
    if mag < EPS {
        (1.0, 0.0) // Default: face right
    } else {
        (v.0 / mag, v.1 / mag)
    }
}

/// Dot product of two 2D vectors
pub fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

/// Calculate angle between two vectors in degrees
pub fn angle_deg(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dot_product = dot(normalize(a), normalize(b));
    let clamped = dot_product.clamp(-1.0, 1.0);
    clamped.acos().to_degrees()
}

/// How much the player is facing toward a target (0.0 = opposite, 1.0 = directly facing)
///
/// **Parameters**:
/// - `body_dir`: Player's body direction (normalized)
/// - `target_pos`: Target position
/// - `player_pos`: Player's current position
///
/// **Returns**: 0.0 (completely opposite) to 1.0 (directly facing)
pub fn facing_to_target(
    body_dir: (f32, f32),
    target_pos: (f32, f32),
    player_pos: (f32, f32),
) -> f32 {
    let to_target = (target_pos.0 - player_pos.0, target_pos.1 - player_pos.1);
    let to_target_norm = normalize(to_target);

    // dot product: -1 (opposite) to 1 (same direction)
    // Convert to 0..1 range
    let facing_dot = dot(normalize(body_dir), to_target_norm);
    (facing_dot + 1.0) / 2.0
}

/// How much the player is facing toward goal (0.0 = away from goal, 1.0 = directly at goal)
///
/// **Parameters**:
/// - `body_dir`: Player's body direction
/// - `player_pos`: Player's current position
/// - `attacks_right`: true if attacking right goal (x=1.0), false if attacking left (x=0.0)
///   FIX_2601/0110: Changed from is_home_team to attacks_right for correct 2nd half behavior
///
/// **Returns**: 0.0 (facing away) to 1.0 (facing goal)
pub fn facing_to_goal(body_dir: (f32, f32), player_pos: (f32, f32), attacks_right: bool) -> f32 {
    let goal_x = if attacks_right { 1.0 } else { 0.0 };
    let goal_pos = (goal_x, 0.5); // Goal center (normalized coords)
    facing_to_target(body_dir, goal_pos, player_pos)
}

/// How much the player is facing toward progress direction (0.0 = backwards, 1.0 = forward)
///
/// **Parameters**:
/// - `body_dir`: Player's body direction
/// - `attacks_right`: true if attacking right, false if attacking left
///   FIX_2601/0110: Changed from is_home_team to attacks_right for correct 2nd half behavior
///
/// **Returns**: 0.0 (facing backwards) to 1.0 (facing forward)
pub fn facing_to_progress(body_dir: (f32, f32), attacks_right: bool) -> f32 {
    let attack_dir = if attacks_right { (1.0, 0.0) } else { (-1.0, 0.0) };
    let facing_dot = dot(normalize(body_dir), attack_dir);
    (facing_dot + 1.0) / 2.0
}

/// Calculate pressure angle from defender to ball carrier
///
/// **Pressure Angle Categories**:
/// - **Frontal** (<35°): Real pressure, increases execution error
/// - **Side** (35°-120°): Moderate pressure
/// - **Rear** (>120°): Weak pressure, easier dribbling
///
/// **Parameters**:
/// - `def_pos`: Defender position
/// - `carrier_pos`: Ball carrier position
/// - `carrier_body_dir`: Ball carrier's facing direction
///
/// **Returns**: Angle in degrees (0°-180°)
pub fn pressure_angle(
    def_pos: (f32, f32),
    carrier_pos: (f32, f32),
    carrier_body_dir: (f32, f32),
) -> f32 {
    // We want the angle of the defender relative to the carrier's facing.
    // If the defender is "in front" of the carrier, the carrier→defender vector
    // should align with `carrier_body_dir` (small angle).
    let carrier_to_def = (def_pos.0 - carrier_pos.0, def_pos.1 - carrier_pos.1);
    angle_deg(carrier_body_dir, carrier_to_def)
}

/// Check if pressure is frontal (<35°)
pub fn is_frontal_pressure(angle: f32) -> bool {
    angle < 35.0
}

/// Check if pressure is from rear (>120°)
pub fn is_rear_pressure(angle: f32) -> bool {
    angle > 120.0
}

/// Update player body direction based on velocity
///
/// **Logic**:
/// - If moving: Face movement direction
/// - If stationary: Keep previous direction
///
/// **Parameters**:
/// - `current_body_dir`: Current body direction
/// - `velocity`: Current velocity vector (m/s)
/// - `min_speed_threshold`: Minimum speed to update direction (m/s)
///
/// **Returns**: New body direction (normalized)
pub fn update_body_dir_from_velocity(
    current_body_dir: (f32, f32),
    velocity: (f32, f32),
    min_speed_threshold: f32,
) -> (f32, f32) {
    let speed = (velocity.0 * velocity.0 + velocity.1 * velocity.1).sqrt();

    if speed < min_speed_threshold {
        // Stationary: Keep current direction
        current_body_dir
    } else {
        // Moving: Face velocity direction
        normalize(velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let v = normalize((3.0, 4.0));
        assert!((v.0 - 0.6).abs() < 0.01);
        assert!((v.1 - 0.8).abs() < 0.01);

        let zero = normalize((0.0, 0.0));
        assert_eq!(zero, (1.0, 0.0)); // Default direction
    }

    #[test]
    fn test_angle_deg() {
        let a = (1.0, 0.0);
        let b = (0.0, 1.0);
        let angle = angle_deg(a, b);
        assert!((angle - 90.0).abs() < 0.1);

        let same = angle_deg(a, a);
        assert!(same.abs() < 0.1); // 0°

        let opposite = angle_deg(a, (-1.0, 0.0));
        assert!((opposite - 180.0).abs() < 0.1);
    }

    #[test]
    fn test_facing_to_goal() {
        let body_dir = (1.0, 0.0); // Facing right
        let player_pos = (0.3, 0.5); // Left side of field
        let attacks_right = true; // FIX_2601/0110: Changed from is_home to attacks_right

        let facing = facing_to_goal(body_dir, player_pos, attacks_right);
        assert!(facing > 0.9); // Should be nearly 1.0 (directly facing)

        // Facing away from goal
        let body_dir_away = (-1.0, 0.0); // Facing left
        let facing_away = facing_to_goal(body_dir_away, player_pos, attacks_right);
        assert!(facing_away < 0.1); // Should be nearly 0.0 (facing away)
    }

    #[test]
    fn test_pressure_angle() {
        let carrier_pos = (0.5, 0.5);
        let carrier_body_dir = (1.0, 0.0); // Facing right

        // Frontal pressure (defender in front)
        let def_front = (0.6, 0.5);
        let angle_front = pressure_angle(def_front, carrier_pos, carrier_body_dir);
        assert!(angle_front < 35.0);
        assert!(is_frontal_pressure(angle_front));

        // Rear pressure (defender behind)
        let def_rear = (0.4, 0.5);
        let angle_rear = pressure_angle(def_rear, carrier_pos, carrier_body_dir);
        assert!(angle_rear > 120.0);
        assert!(is_rear_pressure(angle_rear));
    }

    #[test]
    fn test_update_body_dir_from_velocity() {
        let current = (1.0, 0.0);
        let min_threshold = 0.5;

        // Moving: Should face velocity direction
        let vel_moving = (0.0, 3.0); // Moving up
        let new_dir = update_body_dir_from_velocity(current, vel_moving, min_threshold);
        assert!((new_dir.1 - 1.0).abs() < 0.01); // Should face up

        // Stationary: Keep current direction
        let vel_still = (0.2, 0.0); // Below threshold
        let same_dir = update_body_dir_from_velocity(current, vel_still, min_threshold);
        assert_eq!(same_dir, current);
    }
}
