use crate::engine::physics_constants::field;

const TARGET_REPLAN_DISTANCE: f32 = 1.5;
const WAYPOINT_REACHED_DISTANCE: f32 = 1.5;
const MIN_CURVE_DISTANCE: f32 = 12.0;
const CURVE_OFFSET_RATIO: f32 = 0.25;
const CURVE_OFFSET_MIN: f32 = 3.0;
const CURVE_OFFSET_MAX: f32 = 8.0;
const MIN_ROUTE_POINT_DISTANCE: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
pub enum WaypointProfile {
    Direct,
    Curved,
}

#[derive(Debug, Clone)]
pub struct WaypointManager {
    waypoints: Vec<(f32, f32)>,
    current_index: usize,
    final_target: Option<(f32, f32)>,
}

impl Default for WaypointManager {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            current_index: 0,
            final_target: None,
        }
    }
}

impl WaypointManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_target(&self) -> Option<(f32, f32)> {
        self.waypoints.get(self.current_index).copied()
    }

    pub fn final_target(&self) -> Option<(f32, f32)> {
        self.final_target
    }

    pub fn update(
        &mut self,
        current_pos: (f32, f32),
        target: (f32, f32),
        profile: WaypointProfile,
    ) {
        if self.should_replan(target) {
            self.rebuild_path(current_pos, target, profile);
        } else {
            self.refresh_final_target(target);
        }

        self.advance_if_reached(current_pos);
    }

    pub fn update_with_route(
        &mut self,
        current_pos: (f32, f32),
        target: (f32, f32),
        route: &[(f32, f32)],
    ) {
        if self.should_replan(target) {
            self.rebuild_path_with_route(current_pos, target, route);
        } else {
            self.refresh_final_target(target);
        }

        self.advance_if_reached(current_pos);
    }

    fn should_replan(&self, target: (f32, f32)) -> bool {
        match self.final_target {
            None => true,
            Some(final_target) => distance(final_target, target) > TARGET_REPLAN_DISTANCE,
        }
    }

    fn refresh_final_target(&mut self, target: (f32, f32)) {
        if let Some(last) = self.waypoints.last_mut() {
            *last = target;
        } else {
            self.waypoints.push(target);
            self.current_index = 0;
        }
        self.final_target = Some(target);
    }

    fn rebuild_path(
        &mut self,
        current_pos: (f32, f32),
        target: (f32, f32),
        profile: WaypointProfile,
    ) {
        self.waypoints.clear();
        self.current_index = 0;
        self.final_target = Some(target);

        let distance_to_target = distance(current_pos, target);
        if matches!(profile, WaypointProfile::Direct) || distance_to_target < MIN_CURVE_DISTANCE {
            self.waypoints.push(target);
            return;
        }

        let dx = target.0 - current_pos.0;
        let dy = target.1 - current_pos.1;
        let dir_len = (dx * dx + dy * dy).sqrt();
        if dir_len < 0.01 {
            self.waypoints.push(target);
            return;
        }

        let perp = (-dy / dir_len, dx / dir_len);
        let offset_mag =
            (distance_to_target * CURVE_OFFSET_RATIO).clamp(CURVE_OFFSET_MIN, CURVE_OFFSET_MAX);
        let mid = ((current_pos.0 + target.0) * 0.5, (current_pos.1 + target.1) * 0.5);

        let candidate_a = clamp_to_field((mid.0 + perp.0 * offset_mag, mid.1 + perp.1 * offset_mag));
        let candidate_b = clamp_to_field((mid.0 - perp.0 * offset_mag, mid.1 - perp.1 * offset_mag));
        let pick_a = (candidate_a.1 - current_pos.1).abs() <= (candidate_b.1 - current_pos.1).abs();
        let waypoint = if pick_a { candidate_a } else { candidate_b };

        self.waypoints.push(waypoint);
        self.waypoints.push(target);
    }

    fn rebuild_path_with_route(
        &mut self,
        current_pos: (f32, f32),
        target: (f32, f32),
        route: &[(f32, f32)],
    ) {
        self.waypoints.clear();
        self.current_index = 0;
        self.final_target = Some(target);

        let mut last = current_pos;
        for point in route {
            let clamped = clamp_to_field(*point);
            if distance(last, clamped) > MIN_ROUTE_POINT_DISTANCE {
                self.waypoints.push(clamped);
                last = clamped;
            }
        }

        if distance(last, target) > MIN_ROUTE_POINT_DISTANCE || self.waypoints.is_empty() {
            self.waypoints.push(target);
        }
    }

    fn advance_if_reached(&mut self, current_pos: (f32, f32)) {
        if self.waypoints.is_empty() {
            return;
        }

        if self.current_index >= self.waypoints.len() {
            self.current_index = self.waypoints.len() - 1;
            return;
        }

        let target = self.waypoints[self.current_index];
        if self.current_index + 1 < self.waypoints.len()
            && distance(current_pos, target) <= WAYPOINT_REACHED_DISTANCE
        {
            self.current_index += 1;
        }
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}

fn clamp_to_field(pos: (f32, f32)) -> (f32, f32) {
    let min_x = field::LENGTH_M * 0.05;
    let max_x = field::LENGTH_M * 0.95;
    let min_y = field::WIDTH_M * 0.05;
    let max_y = field::WIDTH_M * 0.95;
    (pos.0.clamp(min_x, max_x), pos.1.clamp(min_y, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_profile_uses_target() {
        let mut manager = WaypointManager::new();
        manager.update((10.0, 10.0), (30.0, 20.0), WaypointProfile::Direct);
        assert_eq!(manager.current_target(), Some((30.0, 20.0)));
        assert_eq!(manager.final_target(), Some((30.0, 20.0)));
    }

    #[test]
    fn curved_profile_advances_to_final_target() {
        let mut manager = WaypointManager::new();
        let target = (40.0, 10.0);
        manager.update((10.0, 10.0), target, WaypointProfile::Curved);

        let first = manager.current_target().expect("expected first waypoint");
        assert_ne!(first, target);

        manager.update(first, target, WaypointProfile::Curved);
        assert_eq!(manager.current_target(), Some(target));
    }

    #[test]
    fn route_profile_uses_first_waypoint() {
        let mut manager = WaypointManager::new();
        let target = (40.0, 40.0);
        let route = vec![(20.0, 20.0), (30.0, 30.0)];
        manager.update_with_route((10.0, 10.0), target, &route);

        assert_eq!(manager.current_target(), Some((20.0, 20.0)));
        assert_eq!(manager.final_target(), Some(target));
    }
}
