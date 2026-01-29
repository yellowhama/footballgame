//! Passing Triangles Module
//!
//! FIX_2601/0110: Dynamic passing triangle formation for build-up play.
//! Creates triangular passing options around the ball holder.
//!
//! Key features:
//! - Calculate ideal triangle positions (forward + lateral support)
//! - Assign players to triangle vertices
//! - Maintain passing options during possession
//!
//! Coord10 axis convention:
//! - x = length (0-1050, goal direction)
//! - y = width (0-680, sideline direction)

use crate::engine::types::Coord10;

/// Represents a passing triangle formation
#[derive(Debug, Clone)]
pub struct PassingTriangle {
    /// Ball holder position (vertex 1)
    pub ball_holder: Coord10,
    /// Forward support position (vertex 2) - ahead of ball in attack direction
    pub forward_support: Coord10,
    /// Lateral support position (vertex 3) - to the side
    pub lateral_support: Coord10,
}

impl PassingTriangle {
    /// Calculate ideal passing triangle positions
    ///
    /// # Arguments
    /// * `ball_pos` - Current ball position (Coord10: x=length, y=width)
    /// * `attack_direction` - +1 for home attacking toward x=1050, -1 for away
    /// * `ball_y` - Ball's Y position (width, 0-680 scale) for lateral decision
    pub fn calculate(ball_pos: Coord10, attack_direction: f32, ball_y: i32) -> Self {
        // Forward support: 10m ahead of ball in attack direction (x-axis)
        // FIX_2601/0110: x is length/goal direction
        let forward_offset: i32 = (100.0 * attack_direction) as i32;
        let forward_x = (ball_pos.x + forward_offset).clamp(100, 950);

        // Lateral support: 8m to the side (y-axis, opposite of ball side)
        // FIX_2601/0110: y is width/lateral direction, center is 340
        let lateral_offset: i32 = if ball_y < 340 { 80 } else { -80 };
        let lateral_y = (ball_pos.y + lateral_offset).clamp(50, 630);

        // Slight backward offset for lateral (safety)
        let lateral_x = (ball_pos.x - (30.0 * attack_direction) as i32).clamp(50, 1000);

        Self {
            ball_holder: ball_pos,
            forward_support: Coord10 { x: forward_x, y: ball_pos.y, z: 0 },
            lateral_support: Coord10 { x: lateral_x, y: lateral_y, z: 0 },
        }
    }

    /// Get the vertex closest to a given position
    pub fn closest_vertex(&self, pos: Coord10) -> TriangleVertex {
        let dist_forward = pos.distance_to(&self.forward_support);
        let dist_lateral = pos.distance_to(&self.lateral_support);

        if dist_forward < dist_lateral {
            TriangleVertex::Forward
        } else {
            TriangleVertex::Lateral
        }
    }
}

/// Which vertex of the triangle a player should occupy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriangleVertex {
    Forward,
    Lateral,
}

/// Assignment of players to triangle vertices
#[derive(Debug, Clone)]
pub struct TriangleAssignment {
    /// Player index for forward support
    pub forward_player: Option<usize>,
    /// Player index for lateral support
    pub lateral_player: Option<usize>,
}

impl TriangleAssignment {
    /// Assign teammates to triangle vertices
    ///
    /// # Arguments
    /// * `triangle` - The triangle to fill
    /// * `ball_holder_idx` - Index of ball holder (excluded from assignment)
    /// * `teammates` - List of (player_idx, position) for team
    pub fn assign(
        triangle: &PassingTriangle,
        ball_holder_idx: usize,
        teammates: &[(usize, Coord10)],
    ) -> Self {
        let mut forward_player = None;
        let mut lateral_player = None;
        let mut best_forward_dist = i32::MAX;
        let mut best_lateral_dist = i32::MAX;

        for (idx, pos) in teammates {
            if *idx == ball_holder_idx {
                continue;
            }

            let dist_to_forward = pos.distance_to(&triangle.forward_support);
            let dist_to_lateral = pos.distance_to(&triangle.lateral_support);

            // Assign to closest unfilled vertex
            if dist_to_forward < dist_to_lateral {
                if dist_to_forward < best_forward_dist {
                    // Check if we should swap with lateral
                    if forward_player.is_some() {
                        // Current forward is closer to forward than this player
                        // but this player might be better for lateral
                        if dist_to_lateral < best_lateral_dist {
                            lateral_player = Some(*idx);
                            best_lateral_dist = dist_to_lateral;
                        }
                    } else {
                        best_forward_dist = dist_to_forward;
                        forward_player = Some(*idx);
                    }
                }
            } else if dist_to_lateral < best_lateral_dist {
                if lateral_player.is_some() {
                    if dist_to_forward < best_forward_dist {
                        forward_player = Some(*idx);
                        best_forward_dist = dist_to_forward;
                    }
                } else {
                    best_lateral_dist = dist_to_lateral;
                    lateral_player = Some(*idx);
                }
            }
        }

        // Second pass: fill any remaining slots
        for (idx, pos) in teammates {
            if *idx == ball_holder_idx {
                continue;
            }
            if forward_player == Some(*idx) || lateral_player == Some(*idx) {
                continue;
            }

            if forward_player.is_none() {
                let dist = pos.distance_to(&triangle.forward_support);
                if dist < best_forward_dist {
                    best_forward_dist = dist;
                    forward_player = Some(*idx);
                }
            }

            if lateral_player.is_none() {
                let dist = pos.distance_to(&triangle.lateral_support);
                if dist < best_lateral_dist {
                    best_lateral_dist = dist;
                    lateral_player = Some(*idx);
                }
            }
        }

        Self { forward_player, lateral_player }
    }

    /// Get target position for a player based on their triangle role
    pub fn get_target_for_player(
        &self,
        player_idx: usize,
        triangle: &PassingTriangle,
    ) -> Option<Coord10> {
        if self.forward_player == Some(player_idx) {
            Some(triangle.forward_support)
        } else if self.lateral_player == Some(player_idx) {
            Some(triangle.lateral_support)
        } else {
            None
        }
    }
}

/// Check if a good triangle is formed (vertices are occupied and spaced)
pub fn is_good_triangle(
    ball_holder: Coord10,
    forward_player: Option<Coord10>,
    lateral_player: Option<Coord10>,
) -> bool {
    let Some(forward) = forward_player else {
        return false;
    };
    let Some(lateral) = lateral_player else {
        return false;
    };

    // Check minimum distances (in Coord10 units, 1 unit = 0.1m)
    const MIN_DISTANCE: i32 = 50; // 5m

    let dist_forward = ball_holder.distance_to(&forward);
    let dist_lateral = ball_holder.distance_to(&lateral);
    let dist_between = forward.distance_to(&lateral);

    dist_forward >= MIN_DISTANCE && dist_lateral >= MIN_DISTANCE && dist_between >= MIN_DISTANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangle_calculation() {
        // Ball at (500, 340) = 50m up field, center width
        // Coord10: x=length (0-1050), y=width (0-680)
        let ball = Coord10 { x: 500, y: 340, z: 0 };
        let triangle = PassingTriangle::calculate(ball, 1.0, ball.y);

        // Forward should be ahead in x (length/goal direction)
        assert!(triangle.forward_support.x > ball.x);

        // Lateral should be to the side (different y/width)
        assert_ne!(triangle.lateral_support.y, ball.y);
    }

    #[test]
    fn test_triangle_assignment() {
        // Ball at center (500, 340)
        // With ball_y = 340 (not < 340), lateral_offset = -80
        // Triangle will have:
        //   forward_support at (600, 340) - 10m ahead in x
        //   lateral_support at (470, 260) - offset in y direction
        let ball = Coord10 { x: 500, y: 340, z: 0 };
        let triangle = PassingTriangle::calculate(ball, 1.0, ball.y);

        // Verify triangle positions
        assert!(triangle.forward_support.x > ball.x); // Forward ahead in x
        assert_ne!(triangle.lateral_support.y, ball.y); // Lateral offset in y

        let teammates = vec![
            (1, Coord10 { x: 610, y: 340, z: 0 }), // Near forward position (ahead in x)
            (2, Coord10 { x: 475, y: 255, z: 0 }), // Near lateral position (close to y=260)
            (3, Coord10 { x: 300, y: 500, z: 0 }), // Far from both
        ];

        let assignment = TriangleAssignment::assign(&triangle, 0, &teammates);

        assert_eq!(assignment.forward_player, Some(1));
        assert_eq!(assignment.lateral_player, Some(2));
    }

    #[test]
    fn test_good_triangle_check() {
        let holder = Coord10 { x: 500, y: 340, z: 0 };

        // Good triangle - properly spaced
        let forward = Some(Coord10 { x: 600, y: 340, z: 0 }); // 10m ahead
        let lateral = Some(Coord10 { x: 480, y: 420, z: 0 }); // Offset laterally
        assert!(is_good_triangle(holder, forward, lateral));

        // Bad triangle - too close
        let close_forward = Some(Coord10 { x: 510, y: 340, z: 0 }); // Only 1m ahead
        assert!(!is_good_triangle(holder, close_forward, lateral));

        // Missing vertex
        assert!(!is_good_triangle(holder, forward, None));
    }

    #[test]
    fn test_get_target_for_player() {
        let ball = Coord10 { x: 500, y: 340, z: 0 };
        let triangle = PassingTriangle::calculate(ball, 1.0, ball.y);

        let assignment = TriangleAssignment { forward_player: Some(1), lateral_player: Some(2) };

        let target1 = assignment.get_target_for_player(1, &triangle);
        let target2 = assignment.get_target_for_player(2, &triangle);
        let target3 = assignment.get_target_for_player(3, &triangle);

        assert_eq!(target1, Some(triangle.forward_support));
        assert_eq!(target2, Some(triangle.lateral_support));
        assert_eq!(target3, None);
    }
}
