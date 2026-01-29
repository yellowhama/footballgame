//! Collision resolution for off-ball objectives

use super::types::*;

/// Check if a target position collides with existing objectives
pub fn check_collision(
    target_x: f32,
    target_y: f32,
    existing_objectives: &[OffBallObjective],
    exclude_idx: usize,
) -> bool {
    for (idx, obj) in existing_objectives.iter().enumerate() {
        if idx == exclude_idx {
            continue;
        }
        if !obj.is_valid() {
            continue;
        }

        let dx = target_x - obj.target_x;
        let dy = target_y - obj.target_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < COLLISION_RADIUS_M * COLLISION_RADIUS_M {
            return true;
        }
    }
    false
}

/// Attempt to resolve collision by shifting the target position.
///
/// Returns Some(new_x, new_y) if resolved, None if unresolvable.
pub fn resolve_collision(
    target_x: f32,
    target_y: f32,
    existing_objectives: &[OffBallObjective],
    exclude_idx: usize,
) -> Option<(f32, f32)> {
    // If no collision, return original
    if !check_collision(target_x, target_y, existing_objectives, exclude_idx) {
        return Some((target_x, target_y));
    }

    // Try shifting in Y direction (positive)
    let shifted_y_pos = (target_y + COLLISION_SHIFT_M).min(67.0);
    if !check_collision(target_x, shifted_y_pos, existing_objectives, exclude_idx) {
        return Some((target_x, shifted_y_pos));
    }

    // Try shifting in Y direction (negative)
    let shifted_y_neg = (target_y - COLLISION_SHIFT_M).max(1.0);
    if !check_collision(target_x, shifted_y_neg, existing_objectives, exclude_idx) {
        return Some((target_x, shifted_y_neg));
    }

    // Try shifting in X direction (forward)
    let shifted_x_pos = (target_x + COLLISION_SHIFT_M).min(104.0);
    if !check_collision(shifted_x_pos, target_y, existing_objectives, exclude_idx) {
        return Some((shifted_x_pos, target_y));
    }

    // Try shifting in X direction (backward)
    let shifted_x_neg = (target_x - COLLISION_SHIFT_M).max(1.0);
    if !check_collision(shifted_x_neg, target_y, existing_objectives, exclude_idx) {
        return Some((shifted_x_neg, target_y));
    }

    // Collision unresolvable
    None
}

/// Resolve collision between two objectives based on priority.
///
/// Returns the index of the objective that should be invalidated,
/// or None if no conflict.
pub fn resolve_priority_conflict(
    obj_a_idx: usize,
    obj_a: &OffBallObjective,
    obj_b_idx: usize,
    obj_b: &OffBallObjective,
) -> Option<usize> {
    if !obj_a.is_valid() || !obj_b.is_valid() {
        return None;
    }

    let dx = obj_a.target_x - obj_b.target_x;
    let dy = obj_a.target_y - obj_b.target_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq >= COLLISION_RADIUS_M * COLLISION_RADIUS_M {
        return None; // No collision
    }

    // Lower priority loses
    if obj_a.priority < obj_b.priority {
        Some(obj_a_idx)
    } else if obj_b.priority < obj_a.priority {
        Some(obj_b_idx)
    } else {
        // Same priority: lower confidence loses
        if obj_a.confidence < obj_b.confidence {
            Some(obj_a_idx)
        } else {
            Some(obj_b_idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_objective(x: f32, y: f32, intent: OffBallIntent) -> OffBallObjective {
        OffBallObjective::new(intent, x, y, Urgency::Jog, 100, 0.8)
    }

    #[test]
    fn test_check_collision_no_collision() {
        let mut objectives = [OffBallObjective::default(); 22];
        objectives[0] = make_objective(50.0, 34.0, OffBallIntent::LinkPlayer);

        // Point far from existing objective
        assert!(!check_collision(70.0, 34.0, &objectives, 5));
    }

    #[test]
    fn test_check_collision_with_collision() {
        let mut objectives = [OffBallObjective::default(); 22];
        objectives[0] = make_objective(50.0, 34.0, OffBallIntent::LinkPlayer);

        // Point within collision radius
        assert!(check_collision(51.0, 34.0, &objectives, 5));
    }

    #[test]
    fn test_check_collision_excludes_self() {
        let mut objectives = [OffBallObjective::default(); 22];
        objectives[5] = make_objective(50.0, 34.0, OffBallIntent::LinkPlayer);

        // Same position but excluded
        assert!(!check_collision(50.0, 34.0, &objectives, 5));
    }

    #[test]
    fn test_resolve_collision_no_conflict() {
        let objectives = [OffBallObjective::default(); 22];

        let result = resolve_collision(50.0, 34.0, &objectives, 0);
        assert_eq!(result, Some((50.0, 34.0)));
    }

    #[test]
    fn test_resolve_collision_shifts() {
        let mut objectives = [OffBallObjective::default(); 22];
        objectives[0] = make_objective(50.0, 34.0, OffBallIntent::LinkPlayer);

        // Try to place at conflicting position
        let result = resolve_collision(51.0, 34.0, &objectives, 5);

        // Should be shifted
        assert!(result.is_some());
        let (new_x, new_y) = result.unwrap();

        // New position should not collide
        assert!(!check_collision(new_x, new_y, &objectives, 5));
    }

    #[test]
    fn test_priority_conflict_resolution() {
        let high_priority = OffBallObjective::new(
            OffBallIntent::SpaceAttacker, // priority 3
            50.0,
            34.0,
            Urgency::Sprint,
            100,
            0.9,
        );
        let low_priority = OffBallObjective::new(
            OffBallIntent::WidthHolder, // priority 1
            51.0,
            34.0,
            Urgency::Jog,
            100,
            0.8,
        );

        let loser = resolve_priority_conflict(0, &high_priority, 1, &low_priority);

        // Low priority should lose
        assert_eq!(loser, Some(1));
    }
}
