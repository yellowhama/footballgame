//! Player Operations - Current player state queries
//!
//! FIX_2512 Phase 7

use crate::engine::types::Coord10;

/// Current player operations trait
pub trait PlayerOperations {
    /// Player index (0-21)
    fn index(&self) -> usize;

    /// Player position (meters)
    fn position(&self) -> (f32, f32);

    /// Player velocity (m/s)
    fn velocity(&self) -> (f32, f32);

    /// Does this player have the ball?
    fn has_ball(&self) -> bool;

    /// Distance to ball (meters)
    fn distance_to_ball(&self) -> f32;

    /// Is player tired (stamina < 0.3)?
    fn is_tired(&self) -> bool;

    /// Current stamina (0.0-1.0)
    fn stamina(&self) -> f32;

    /// Is this player on the home team?
    fn is_home(&self) -> bool;

    /// Player speed magnitude (m/s)
    fn speed(&self) -> f32;

    /// Is player moving fast (> 5 m/s)?
    fn is_sprinting(&self) -> bool;
}

/// Player operations implementation
pub struct PlayerOps<'a> {
    player_idx: usize,
    positions: &'a [Coord10; 22],
    velocities: &'a [(f32, f32); 22],
    stamina: &'a [f32; 22],
    ball_owner: Option<u8>,
}

impl<'a> PlayerOps<'a> {
    pub fn new(
        player_idx: usize,
        positions: &'a [Coord10; 22],
        velocities: &'a [(f32, f32); 22],
        stamina: &'a [f32; 22],
        ball_owner: Option<u8>,
    ) -> Self {
        Self {
            player_idx,
            positions,
            velocities,
            stamina,
            ball_owner,
        }
    }
}

impl<'a> PlayerOperations for PlayerOps<'a> {
    fn index(&self) -> usize {
        self.player_idx
    }

    fn position(&self) -> (f32, f32) {
        self.positions[self.player_idx].to_meters()
    }

    fn velocity(&self) -> (f32, f32) {
        self.velocities[self.player_idx]
    }

    fn has_ball(&self) -> bool {
        self.ball_owner == Some(self.player_idx as u8)
    }

    fn distance_to_ball(&self) -> f32 {
        // Ball owner has distance 0
        if let Some(owner) = self.ball_owner {
            if owner as usize == self.player_idx {
                return 0.0;
            }
            // Approximate: distance to ball owner
            let my_pos = self.positions[self.player_idx].to_meters();
            let owner_pos = self.positions[owner as usize].to_meters();
            ((my_pos.0 - owner_pos.0).powi(2) + (my_pos.1 - owner_pos.1).powi(2)).sqrt()
        } else {
            // No owner - would need ball position, return large value
            100.0
        }
    }

    fn is_tired(&self) -> bool {
        self.stamina[self.player_idx] < 0.3
    }

    fn stamina(&self) -> f32 {
        self.stamina[self.player_idx]
    }

    fn is_home(&self) -> bool {
        self.player_idx < 11
    }

    fn speed(&self) -> f32 {
        let (vx, vy) = self.velocities[self.player_idx];
        (vx * vx + vy * vy).sqrt()
    }

    fn is_sprinting(&self) -> bool {
        self.speed() > 5.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    fn make_test_data() -> ([Coord10; 22], [(f32, f32); 22], [f32; 22]) {
        const CY: f32 = field::CENTER_Y;
        let positions = std::array::from_fn(|i| {
            Coord10::from_meters(50.0 + (i as f32 * 2.0), CY)
        });
        let velocities = [(0.0, 0.0); 22];
        let stamina = [1.0; 22];
        (positions, velocities, stamina)
    }

    #[test]
    fn test_player_index() {
        let (positions, velocities, stamina) = make_test_data();
        let ops = PlayerOps::new(5, &positions, &velocities, &stamina, None);

        assert_eq!(ops.index(), 5);
    }

    #[test]
    fn test_is_home() {
        let (positions, velocities, stamina) = make_test_data();

        let home_ops = PlayerOps::new(5, &positions, &velocities, &stamina, None);
        assert!(home_ops.is_home());

        let away_ops = PlayerOps::new(15, &positions, &velocities, &stamina, None);
        assert!(!away_ops.is_home());
    }

    #[test]
    fn test_has_ball() {
        let (positions, velocities, stamina) = make_test_data();

        let ops_with_ball = PlayerOps::new(5, &positions, &velocities, &stamina, Some(5));
        assert!(ops_with_ball.has_ball());

        let ops_without_ball = PlayerOps::new(5, &positions, &velocities, &stamina, Some(3));
        assert!(!ops_without_ball.has_ball());

        let ops_no_owner = PlayerOps::new(5, &positions, &velocities, &stamina, None);
        assert!(!ops_no_owner.has_ball());
    }

    #[test]
    fn test_stamina() {
        let (positions, velocities, mut stamina) = make_test_data();
        stamina[5] = 0.2;

        let ops = PlayerOps::new(5, &positions, &velocities, &stamina, None);
        assert!(ops.is_tired());
        assert!((ops.stamina() - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_speed() {
        let (positions, mut velocities, stamina) = make_test_data();
        velocities[5] = (3.0, 4.0); // Speed = 5.0

        let ops = PlayerOps::new(5, &positions, &velocities, &stamina, None);
        assert!((ops.speed() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_is_sprinting() {
        let (positions, mut velocities, stamina) = make_test_data();
        velocities[5] = (4.0, 4.0); // Speed = 5.66

        let ops = PlayerOps::new(5, &positions, &velocities, &stamina, None);
        assert!(ops.is_sprinting());

        velocities[5] = (2.0, 2.0); // Speed = 2.83
        let ops_slow = PlayerOps::new(5, &positions, &velocities, &stamina, None);
        assert!(!ops_slow.is_sprinting());
    }
}
