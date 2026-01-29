//! Steering helpers (minimal set)
//!
//! These helpers return velocity vectors that work with the inertia-based
//! movement system (velocity modifiers, not direct position jumps).
//!
//! P3a: Now exported from engine module for use in defensive_positioning.

/// Normalize a 2D vector (local copy to avoid circular dependencies)
#[inline]
fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 0.0001 {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

/// Seek: move toward a target at the given speed.
pub fn seek(current: (f32, f32), target: (f32, f32), speed: f32) -> (f32, f32) {
    if speed <= 0.0 {
        return (0.0, 0.0);
    }
    let dir = normalize((target.0 - current.0, target.1 - current.1));
    (dir.0 * speed, dir.1 * speed)
}

/// Arrive: slow down when approaching the target.
pub fn arrive(
    current: (f32, f32),
    target: (f32, f32),
    max_speed: f32,
    slowing_distance: f32,
) -> (f32, f32) {
    if max_speed <= 0.0 {
        return (0.0, 0.0);
    }

    let to_target = (target.0 - current.0, target.1 - current.1);
    let distance = (to_target.0 * to_target.0 + to_target.1 * to_target.1).sqrt();
    if distance < 0.0001 {
        return (0.0, 0.0);
    }

    let speed = if slowing_distance > 0.0 {
        let ratio = (distance / slowing_distance).clamp(0.0, 1.0);
        max_speed * ratio
    } else {
        max_speed
    };
    let dir = (to_target.0 / distance, to_target.1 / distance);
    (dir.0 * speed, dir.1 * speed)
}

/// Pursuit: chase a moving target by predicting its future position.
pub fn pursuit(
    current: (f32, f32),
    target_pos: (f32, f32),
    target_vel: (f32, f32),
    speed: f32,
    max_lookahead_s: f32,
) -> (f32, f32) {
    if speed <= 0.0 {
        return (0.0, 0.0);
    }

    let to_target = (target_pos.0 - current.0, target_pos.1 - current.1);
    let distance = (to_target.0 * to_target.0 + to_target.1 * to_target.1).sqrt();
    let lookahead = (distance / speed).min(max_lookahead_s.max(0.0));
    let future = (target_pos.0 + target_vel.0 * lookahead, target_pos.1 + target_vel.1 * lookahead);
    seek(current, future, speed)
}

/// Separation: push away from nearby neighbors to avoid clustering.
pub fn separation(
    current: (f32, f32),
    neighbors: &[(f32, f32)],
    separation_radius: f32,
    separation_strength: f32,
) -> (f32, f32) {
    if separation_radius <= 0.0 || separation_strength <= 0.0 {
        return (0.0, 0.0);
    }

    let mut force = (0.0, 0.0);
    for neighbor in neighbors {
        let to_neighbor = (current.0 - neighbor.0, current.1 - neighbor.1);
        let distance = (to_neighbor.0 * to_neighbor.0 + to_neighbor.1 * to_neighbor.1).sqrt();
        if distance > 0.0 && distance < separation_radius {
            let strength = (1.0 - distance / separation_radius) * separation_strength;
            let dir = (to_neighbor.0 / distance, to_neighbor.1 / distance);
            force.0 += dir.0 * strength;
            force.1 += dir.1 * strength;
        }
    }
    force
}

// FIX_2601/0106 P3-13: WanderState removed (unused)
// See git history for previous implementation if needed

/// State holder for follow-path movement.
#[derive(Debug, Clone, Copy)]
pub struct FollowPathState {
    waypoint_idx: usize,
    looped: bool,
}

impl FollowPathState {
    pub fn new(looped: bool) -> Self {
        Self { waypoint_idx: 0, looped }
    }

    pub fn waypoint_idx(&self) -> usize {
        self.waypoint_idx
    }

    /// Step toward the current waypoint; advances when close enough.
    pub fn step(
        &mut self,
        current: (f32, f32),
        waypoints: &[(f32, f32)],
        speed: f32,
        arrive_distance: f32,
    ) -> (f32, f32) {
        if waypoints.is_empty() || speed <= 0.0 {
            return (0.0, 0.0);
        }

        if self.waypoint_idx >= waypoints.len() {
            self.waypoint_idx = waypoints.len().saturating_sub(1);
        }

        let mut target = waypoints[self.waypoint_idx];
        let dx = target.0 - current.0;
        let dy = target.1 - current.1;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= arrive_distance {
            if self.waypoint_idx + 1 < waypoints.len() {
                self.waypoint_idx += 1;
                target = waypoints[self.waypoint_idx];
            } else if self.looped {
                self.waypoint_idx = 0;
                target = waypoints[self.waypoint_idx];
            } else {
                return (0.0, 0.0);
            }
        }

        arrive(current, target, speed, arrive_distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seek_direction() {
        let vel = seek((0.0, 0.0), (10.0, 0.0), 5.0);
        assert!(vel.0 > 0.0);
        assert_eq!(vel.1, 0.0);
    }

    #[test]
    fn test_pursuit_lookahead() {
        let vel = pursuit((0.0, 0.0), (10.0, 0.0), (1.0, 0.0), 5.0, 1.0);
        assert!(vel.0 > 0.0);
    }

    #[test]
    fn test_arrive_slows_down_near_target() {
        let vel = arrive((0.0, 0.0), (1.0, 0.0), 10.0, 5.0);
        assert!(vel.0 > 0.0);
        assert!(vel.0 < 10.0);
    }

    #[test]
    fn test_arrive_full_speed_far() {
        let vel = arrive((0.0, 0.0), (10.0, 0.0), 6.0, 5.0);
        assert!((vel.0 - 6.0).abs() < 0.001);
        assert_eq!(vel.1, 0.0);
    }

    #[test]
    fn test_separation_pushes_away() {
        let force = separation((0.0, 0.0), &[(1.0, 0.0), (3.0, 0.0)], 2.0, 1.0);
        assert!(force.0 < 0.0);
    }

    // FIX_2601/0106 P3-13: test_wander_updates_target_and_moves_forward removed (WanderState deleted)

    #[test]
    fn test_follow_path_advances_index() {
        let mut state = FollowPathState::new(false);
        let waypoints = [(0.0, 0.0), (10.0, 0.0)];
        let vel = state.step((0.0, 0.0), &waypoints, 5.0, 1.0);
        assert_eq!(state.waypoint_idx(), 1);
        assert!(vel.0 > 0.0);
    }
}
