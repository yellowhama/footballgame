//! Players Operations - Teammates and opponents queries
//!
//! FIX_2512 Phase 7

use crate::engine::types::Coord10;

/// Operations for other players (teammates/opponents)
pub trait PlayersOperations {
    /// Get teammate indices (excluding self)
    fn teammate_indices(&self) -> Vec<usize>;

    /// Get opponent indices
    fn opponent_indices(&self) -> Vec<usize>;

    /// Nearest teammate index (if any)
    fn nearest_teammate(&self) -> Option<usize>;

    /// Nearest opponent index (if any)
    fn nearest_opponent(&self) -> Option<usize>;

    /// Distance to specific player (meters)
    fn distance_to(&self, player_idx: usize) -> f32;

    /// Is opponent with ball nearby (<3m)?
    fn opponent_with_ball_nearby(&self) -> bool;

    /// Count opponents within radius
    fn opponents_within(&self, radius: f32) -> usize;

    /// Count teammates within radius
    fn teammates_within(&self, radius: f32) -> usize;

    /// Get nearest N teammates
    fn nearest_teammates(&self, n: usize) -> Vec<usize>;

    /// Get nearest N opponents
    fn nearest_opponents(&self, n: usize) -> Vec<usize>;

    /// Local pressure (0.0-1.0) based on nearby opponents
    fn local_pressure(&self) -> f32;
}

/// Players operations implementation
pub struct PlayersOps<'a> {
    player_idx: usize,
    positions: &'a [Coord10; 22],
    ball_owner: Option<u8>,
    is_home: bool,
}

impl<'a> PlayersOps<'a> {
    pub fn new(player_idx: usize, positions: &'a [Coord10; 22], ball_owner: Option<u8>) -> Self {
        Self {
            player_idx,
            positions,
            ball_owner,
            is_home: player_idx < 11,
        }
    }

    fn my_position(&self) -> (f32, f32) {
        self.positions[self.player_idx].to_meters()
    }

    fn distance_between(&self, a: usize, b: usize) -> f32 {
        let pos_a = self.positions[a].to_meters();
        let pos_b = self.positions[b].to_meters();
        ((pos_a.0 - pos_b.0).powi(2) + (pos_a.1 - pos_b.1).powi(2)).sqrt()
    }
}

impl<'a> PlayersOperations for PlayersOps<'a> {
    fn teammate_indices(&self) -> Vec<usize> {
        if self.is_home {
            (0..11).filter(|&i| i != self.player_idx).collect()
        } else {
            (11..22).filter(|&i| i != self.player_idx).collect()
        }
    }

    fn opponent_indices(&self) -> Vec<usize> {
        if self.is_home {
            (11..22).collect()
        } else {
            (0..11).collect()
        }
    }

    fn nearest_teammate(&self) -> Option<usize> {
        self.teammate_indices()
            .into_iter()
            .min_by(|&a, &b| {
                let dist_a = self.distance_between(self.player_idx, a);
                let dist_b = self.distance_between(self.player_idx, b);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
    }

    fn nearest_opponent(&self) -> Option<usize> {
        self.opponent_indices()
            .into_iter()
            .min_by(|&a, &b| {
                let dist_a = self.distance_between(self.player_idx, a);
                let dist_b = self.distance_between(self.player_idx, b);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
    }

    fn distance_to(&self, player_idx: usize) -> f32 {
        self.distance_between(self.player_idx, player_idx)
    }

    fn opponent_with_ball_nearby(&self) -> bool {
        if let Some(owner) = self.ball_owner {
            let owner_idx = owner as usize;
            // Check if owner is opponent
            let owner_is_opponent = if self.is_home {
                owner_idx >= 11
            } else {
                owner_idx < 11
            };

            if owner_is_opponent {
                let dist = self.distance_between(self.player_idx, owner_idx);
                return dist < 3.0;
            }
        }
        false
    }

    fn opponents_within(&self, radius: f32) -> usize {
        self.opponent_indices()
            .into_iter()
            .filter(|&opp| self.distance_between(self.player_idx, opp) < radius)
            .count()
    }

    fn teammates_within(&self, radius: f32) -> usize {
        self.teammate_indices()
            .into_iter()
            .filter(|&tm| self.distance_between(self.player_idx, tm) < radius)
            .count()
    }

    fn nearest_teammates(&self, n: usize) -> Vec<usize> {
        let mut teammates: Vec<usize> = self.teammate_indices();
        teammates.sort_by(|&a, &b| {
            let dist_a = self.distance_between(self.player_idx, a);
            let dist_b = self.distance_between(self.player_idx, b);
            dist_a.partial_cmp(&dist_b).unwrap()
        });
        teammates.into_iter().take(n).collect()
    }

    fn nearest_opponents(&self, n: usize) -> Vec<usize> {
        let mut opponents: Vec<usize> = self.opponent_indices();
        opponents.sort_by(|&a, &b| {
            let dist_a = self.distance_between(self.player_idx, a);
            let dist_b = self.distance_between(self.player_idx, b);
            dist_a.partial_cmp(&dist_b).unwrap()
        });
        opponents.into_iter().take(n).collect()
    }

    fn local_pressure(&self) -> f32 {
        // Pressure based on nearby opponents
        // 0-2 opponents within 5m: low pressure
        // 3+ opponents: high pressure
        let nearby = self.opponents_within(5.0);

        match nearby {
            0 => 0.0,
            1 => 0.3,
            2 => 0.5,
            3 => 0.7,
            _ => 0.9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    fn make_test_positions() -> [Coord10; 22] {
        // Home team (0-10) on left side, Away team (11-21) on right side
        std::array::from_fn(|i| {
            if i < 11 {
                // Home team spread around x=30
                Coord10::from_meters(25.0 + (i as f32 * 3.0), 30.0 + (i as f32 * 2.0))
            } else {
                // Away team spread around x=75
                Coord10::from_meters(70.0 + ((i - 11) as f32 * 3.0), 30.0 + ((i - 11) as f32 * 2.0))
            }
        })
    }

    #[test]
    fn test_teammate_indices() {
        let positions = make_test_positions();
        let ops = PlayersOps::new(5, &positions, None);

        let teammates = ops.teammate_indices();
        assert_eq!(teammates.len(), 10); // 11 - 1 (self)
        assert!(!teammates.contains(&5)); // Self excluded
        assert!(teammates.contains(&0));
        assert!(teammates.contains(&10));
    }

    #[test]
    fn test_opponent_indices() {
        let positions = make_test_positions();
        let ops = PlayersOps::new(5, &positions, None);

        let opponents = ops.opponent_indices();
        assert_eq!(opponents.len(), 11);
        assert!(opponents.contains(&11));
        assert!(opponents.contains(&21));
    }

    #[test]
    fn test_opponent_with_ball_nearby() {
        let mut positions = make_test_positions();
        // Put player 5 and opponent 15 close together
        positions[5] = Coord10::from_meters(50.0, field::CENTER_Y);
        positions[15] = Coord10::from_meters(52.0, field::CENTER_Y);

        // Opponent has ball and is nearby
        let ops_nearby = PlayersOps::new(5, &positions, Some(15));
        assert!(ops_nearby.opponent_with_ball_nearby());

        // Teammate has ball
        let ops_teammate = PlayersOps::new(5, &positions, Some(3));
        assert!(!ops_teammate.opponent_with_ball_nearby());

        // No one has ball
        let ops_none = PlayersOps::new(5, &positions, None);
        assert!(!ops_none.opponent_with_ball_nearby());
    }

    #[test]
    fn test_local_pressure() {
        let mut positions = make_test_positions();

        // Put player 5 far from everyone
        positions[5] = Coord10::from_meters(0.0, 0.0);

        let ops = PlayersOps::new(5, &positions, None);
        assert_eq!(ops.local_pressure(), 0.0);

        // Now put some opponents nearby
        positions[11] = Coord10::from_meters(2.0, 0.0);
        positions[12] = Coord10::from_meters(0.0, 3.0);

        let ops_pressure = PlayersOps::new(5, &positions, None);
        assert!(ops_pressure.local_pressure() > 0.0);
    }
}
