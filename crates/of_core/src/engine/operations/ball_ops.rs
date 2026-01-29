//! Ball Operations - Ball state queries
//!
//! FIX_2512 Phase 7

use crate::engine::ball::Ball;
use crate::engine::types::Coord10;

/// Ball-related operations trait
pub trait BallOperations {
    /// Is the ball owned by any player?
    fn is_owned(&self) -> bool;

    /// Get the ball owner index (if any)
    fn owner(&self) -> Option<usize>;

    /// Distance from ball to specific player (meters)
    fn distance_to(&self, player_idx: usize) -> f32;

    /// Ball velocity (m/s)
    fn velocity(&self) -> (f32, f32);

    /// Is ball in flight (pass/shot)?
    fn is_in_flight(&self) -> bool;

    /// Ball position (meters)
    fn position(&self) -> (f32, f32);

    /// Is ball in penalty area?
    /// - `is_home_area`: true for home team's penalty area (left side)
    fn is_in_penalty_area(&self, is_home_area: bool) -> bool;

    /// Is ball in the attacking third for given team?
    fn is_in_attacking_third(&self, is_home_team: bool) -> bool;

    /// Is ball aerial (height > 0.5m)?
    fn is_aerial(&self) -> bool;
}

/// Ball operations implementation
pub struct BallOps<'a> {
    ball: &'a Ball,
    player_positions: &'a [Coord10; 22],
}

impl<'a> BallOps<'a> {
    pub fn new(ball: &'a Ball, positions: &'a [Coord10; 22]) -> Self {
        Self {
            ball,
            player_positions: positions,
        }
    }
}

impl<'a> BallOperations for BallOps<'a> {
    fn is_owned(&self) -> bool {
        self.ball.current_owner.is_some()
    }

    fn owner(&self) -> Option<usize> {
        self.ball.current_owner
    }

    fn distance_to(&self, player_idx: usize) -> f32 {
        let ball_m = self.ball.position.to_meters();
        let player_m = self.player_positions[player_idx].to_meters();
        ((ball_m.0 - player_m.0).powi(2) + (ball_m.1 - player_m.1).powi(2)).sqrt()
    }

    fn velocity(&self) -> (f32, f32) {
        self.ball.velocity.to_mps()
    }

    fn is_in_flight(&self) -> bool {
        self.ball.is_in_flight
    }

    fn position(&self) -> (f32, f32) {
        self.ball.position.to_meters()
    }

    fn is_in_penalty_area(&self, is_home_area: bool) -> bool {
        let (x, y) = self.ball.position.to_meters();

        // Penalty area: 16.5m from goal line, 40.3m wide (centered at 34m)
        let in_y = (13.85..=54.15).contains(&y); // 34 - 20.15 to 34 + 20.15

        if is_home_area {
            // Home penalty area (left side, x: 0-16.5)
            x <= 16.5 && in_y
        } else {
            // Away penalty area (right side, x: 88.5-105)
            x >= 88.5 && in_y
        }
    }

    fn is_in_attacking_third(&self, is_home_team: bool) -> bool {
        let (x, _) = self.ball.position.to_meters();

        if is_home_team {
            // Home team attacks right (x > 70)
            x > 70.0
        } else {
            // Away team attacks left (x < 35)
            x < 35.0
        }
    }

    fn is_aerial(&self) -> bool {
        // height is in 0.1m units, so 5 = 0.5m
        self.ball.height > 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    fn make_test_positions() -> [Coord10; 22] {
        const CY: f32 = field::CENTER_Y;
        std::array::from_fn(|i| {
            let x = 50.0 + (i as f32 * 2.0);
            let y = CY;
            Coord10::from_meters(x, y)
        })
    }

    #[test]
    fn test_ball_not_owned() {
        let positions = make_test_positions();
        let ball = Ball::default();

        let ops = BallOps::new(&ball, &positions);
        assert!(!ops.is_owned());
        assert!(ops.owner().is_none());
    }

    #[test]
    fn test_ball_owned() {
        let positions = make_test_positions();
        let mut ball = Ball::default();
        ball.current_owner = Some(5);

        let ops = BallOps::new(&ball, &positions);
        assert!(ops.is_owned());
        assert_eq!(ops.owner(), Some(5));
    }

    #[test]
    fn test_distance_to_player() {
        let mut positions = make_test_positions();
        positions[0] = Coord10::from_meters(50.0, field::CENTER_Y);

        let mut ball = Ball::default();
        ball.position = Coord10::from_meters(53.0, 38.0);

        let ops = BallOps::new(&ball, &positions);
        let dist = ops.distance_to(0);

        // Distance should be 5.0 (3-4-5 triangle)
        assert!((dist - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_penalty_area() {
        let positions = make_test_positions();
        let mut ball = Ball::default();

        // Ball in home penalty area
        ball.position = Coord10::from_meters(10.0, field::CENTER_Y);
        let ops = BallOps::new(&ball, &positions);
        assert!(ops.is_in_penalty_area(true));
        assert!(!ops.is_in_penalty_area(false));

        // Ball in away penalty area
        ball.position = Coord10::from_meters(95.0, field::CENTER_Y);
        let ops = BallOps::new(&ball, &positions);
        assert!(!ops.is_in_penalty_area(true));
        assert!(ops.is_in_penalty_area(false));
    }
}
