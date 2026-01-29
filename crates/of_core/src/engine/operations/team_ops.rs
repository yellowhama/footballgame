//! Team Operations - Team-level state queries
//!
//! FIX_2512 Phase 7

use crate::engine::ball::Ball;
use crate::engine::types::Coord10;

/// Team-level operations trait
pub trait TeamOperations {
    /// Does team have the ball?
    fn has_ball(&self) -> bool;

    /// Is team attacking (ball in opponent half)?
    fn is_attacking(&self) -> bool;

    /// Is team defending (ball in own half)?
    fn is_defending(&self) -> bool;

    /// Defensive line Y position (average of defenders)
    fn defensive_line_y(&self) -> f32;

    /// Attacking line Y position (average of forwards)
    fn attacking_line_y(&self) -> f32;

    /// Team compactness (average distance between players)
    fn compactness(&self) -> f32;

    /// Ball position relative to team (attacking/midfield/defending third)
    fn ball_zone(&self) -> TeamBallZone;

    /// Is ball in own penalty area?
    fn ball_in_own_box(&self) -> bool;

    /// Is ball in opponent penalty area?
    fn ball_in_opponent_box(&self) -> bool;
}

/// Ball zone relative to team
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamBallZone {
    DefendingThird,
    MiddleThird,
    AttackingThird,
}

/// Team operations implementation
pub struct TeamOps<'a> {
    is_home: bool,
    ball: &'a Ball,
    positions: &'a [Coord10; 22],
    ball_owner: Option<u8>,
}

impl<'a> TeamOps<'a> {
    pub fn new(
        is_home: bool,
        ball: &'a Ball,
        positions: &'a [Coord10; 22],
        ball_owner: Option<u8>,
    ) -> Self {
        Self {
            is_home,
            ball,
            positions,
            ball_owner,
        }
    }

    fn team_indices(&self) -> std::ops::Range<usize> {
        if self.is_home {
            0..11
        } else {
            11..22
        }
    }

    fn defender_indices(&self) -> Vec<usize> {
        // Defenders are typically indices 1-4 for home, 12-15 for away
        // (index 0/11 is goalkeeper)
        if self.is_home {
            vec![1, 2, 3, 4]
        } else {
            vec![12, 13, 14, 15]
        }
    }

    fn forward_indices(&self) -> Vec<usize> {
        // Forwards are typically last few outfield players
        if self.is_home {
            vec![8, 9, 10]
        } else {
            vec![19, 20, 21]
        }
    }
}

impl<'a> TeamOperations for TeamOps<'a> {
    fn has_ball(&self) -> bool {
        if let Some(owner) = self.ball_owner {
            let owner_idx = owner as usize;
            if self.is_home {
                owner_idx < 11
            } else {
                owner_idx >= 11
            }
        } else {
            false
        }
    }

    fn is_attacking(&self) -> bool {
        let (ball_x, _) = self.ball.position.to_meters();

        if self.is_home {
            // Home attacks right (high x)
            ball_x > field::CENTER_X
        } else {
            // Away attacks left (low x)
            ball_x < field::CENTER_X
        }
    }

    fn is_defending(&self) -> bool {
        !self.is_attacking()
    }

    fn defensive_line_y(&self) -> f32 {
        let defender_indices = self.defender_indices();
        let total_y: f32 = defender_indices
            .iter()
            .map(|&i| self.positions[i].to_meters().1)
            .sum();

        total_y / defender_indices.len() as f32
    }

    fn attacking_line_y(&self) -> f32 {
        let forward_indices = self.forward_indices();
        let total_y: f32 = forward_indices
            .iter()
            .map(|&i| self.positions[i].to_meters().1)
            .sum();

        total_y / forward_indices.len() as f32
    }

    fn compactness(&self) -> f32 {
        // Calculate average distance between all team players
        let indices: Vec<usize> = self.team_indices().collect();
        let mut total_dist = 0.0;
        let mut count = 0;

        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let pos_a = self.positions[indices[i]].to_meters();
                let pos_b = self.positions[indices[j]].to_meters();
                let dist = ((pos_a.0 - pos_b.0).powi(2) + (pos_a.1 - pos_b.1).powi(2)).sqrt();
                total_dist += dist;
                count += 1;
            }
        }

        if count > 0 {
            total_dist / count as f32
        } else {
            0.0
        }
    }

    fn ball_zone(&self) -> TeamBallZone {
        let (ball_x, _) = self.ball.position.to_meters();

        if self.is_home {
            // Home defends left (low x), attacks right (high x)
            if ball_x < 35.0 {
                TeamBallZone::DefendingThird
            } else if ball_x > 70.0 {
                TeamBallZone::AttackingThird
            } else {
                TeamBallZone::MiddleThird
            }
        } else {
            // Away defends right (high x), attacks left (low x)
            if ball_x > 70.0 {
                TeamBallZone::DefendingThird
            } else if ball_x < 35.0 {
                TeamBallZone::AttackingThird
            } else {
                TeamBallZone::MiddleThird
            }
        }
    }

    fn ball_in_own_box(&self) -> bool {
        let (ball_x, ball_y) = self.ball.position.to_meters();

        // Penalty box: 16.5m from goal, 40.3m wide
        let in_y = (13.85..=54.15).contains(&ball_y);

        if self.is_home {
            // Home defends left goal (x < 16.5)
            ball_x <= 16.5 && in_y
        } else {
            // Away defends right goal (x > 88.5)
            ball_x >= 88.5 && in_y
        }
    }

    fn ball_in_opponent_box(&self) -> bool {
        let (ball_x, ball_y) = self.ball.position.to_meters();

        let in_y = (13.85..=54.15).contains(&ball_y);

        if self.is_home {
            // Home attacks right goal (x > 88.5)
            ball_x >= 88.5 && in_y
        } else {
            // Away attacks left goal (x < 16.5)
            ball_x <= 16.5 && in_y
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_positions() -> [Coord10; 22] {
        std::array::from_fn(|i| {
            if i < 11 {
                // Home team on left side
                Coord10::from_meters(20.0 + (i as f32 * 5.0), field::CENTER_Y)
            } else {
                // Away team on right side
                Coord10::from_meters(50.0 + ((i - 11) as f32 * 5.0), field::CENTER_Y)
            }
        })
    }

    #[test]
    fn test_has_ball() {
        let positions = make_test_positions();
        let ball = Ball::default();

        // Home player has ball
        let ops_home = TeamOps::new(true, &ball, &positions, Some(5));
        assert!(ops_home.has_ball());

        // Away player has ball
        let ops_away = TeamOps::new(true, &ball, &positions, Some(15));
        assert!(!ops_away.has_ball());

        // No one has ball
        let ops_none = TeamOps::new(true, &ball, &positions, None);
        assert!(!ops_none.has_ball());
    }

    #[test]
    fn test_is_attacking() {
        let positions = make_test_positions();
        let mut ball = Ball::default();

        // Ball in right half (home attacking)
        ball.position = Coord10::from_meters(70.0, field::CENTER_Y);
        let ops_home = TeamOps::new(true, &ball, &positions, None);
        assert!(ops_home.is_attacking());

        // Ball in left half (away attacking)
        let ops_away = TeamOps::new(false, &ball, &positions, None);
        assert!(!ops_away.is_attacking());
    }

    #[test]
    fn test_ball_zone() {
        let positions = make_test_positions();
        let mut ball = Ball::default();

        // Ball in home defending third
        ball.position = Coord10::from_meters(20.0, field::CENTER_Y);
        let ops = TeamOps::new(true, &ball, &positions, None);
        assert_eq!(ops.ball_zone(), TeamBallZone::DefendingThird);

        // Ball in middle third
        ball.position = Coord10::CENTER;
        let ops_mid = TeamOps::new(true, &ball, &positions, None);
        assert_eq!(ops_mid.ball_zone(), TeamBallZone::MiddleThird);

        // Ball in home attacking third
        ball.position = Coord10::from_meters(80.0, field::CENTER_Y);
        let ops_att = TeamOps::new(true, &ball, &positions, None);
        assert_eq!(ops_att.ball_zone(), TeamBallZone::AttackingThird);
    }

    #[test]
    fn test_ball_in_box() {
        let positions = make_test_positions();
        let mut ball = Ball::default();

        // Ball in home penalty box
        ball.position = Coord10::from_meters(10.0, field::CENTER_Y);
        let ops = TeamOps::new(true, &ball, &positions, None);
        assert!(ops.ball_in_own_box());
        assert!(!ops.ball_in_opponent_box());

        // Ball in away penalty box (home's opponent box)
        ball.position = Coord10::from_meters(95.0, field::CENTER_Y);
        let ops_opp = TeamOps::new(true, &ball, &positions, None);
        assert!(!ops_opp.ball_in_own_box());
        assert!(ops_opp.ball_in_opponent_box());
    }
}
