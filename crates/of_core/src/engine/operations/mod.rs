//! Operations Interface Pattern for Football Engine
//!
//! FIX_2512 Phase 7: Clean operations interface inspired by Open Football
//! FIX_2512 Phase 8: PlayerDistanceMatrix for O(1) distance queries
//! FIX_2512 Phase 9: SpaceAnalysis for grid-based spatial awareness
//!
//! Provides structured access to game state through trait-based operations:
//! - `BallOperations` - Ball state and queries
//! - `PlayerOperations` - Current player state
//! - `PlayersOperations` - Other players (teammates/opponents)
//! - `TeamOperations` - Team-level state
//! - `PlayerDistanceMatrix` - Pre-computed distance matrix (Phase 8)
//! - `SpaceAnalysis` - Grid-based space analysis (Phase 9)

mod ball_ops;
mod distance_matrix;
mod player_ops;
mod players_ops;
mod space_analysis;
mod team_ops;

pub use ball_ops::{BallOperations, BallOps};
pub use distance_matrix::PlayerDistanceMatrix;
pub use player_ops::{PlayerOperations, PlayerOps};
pub use players_ops::{PlayersOperations, PlayersOps};
pub use space_analysis::{CellOccupancy, SpaceAnalysis};
pub use team_ops::{TeamBallZone, TeamOperations, TeamOps};

use crate::engine::ball::Ball;
use crate::engine::types::Coord10;

/// Combined operations context for a single player tick
///
/// This provides a clean interface for handlers to query game state
/// without needing to know the internal data structures.
pub struct OperationsContext<'a> {
    pub ball: BallOps<'a>,
    pub player: PlayerOps<'a>,
    pub players: PlayersOps<'a>,
    pub team: TeamOps<'a>,
}

impl<'a> OperationsContext<'a> {
    /// Create operations context for a specific player
    pub fn new(
        player_idx: usize,
        ball: &'a Ball,
        positions: &'a [Coord10; 22],
        velocities: &'a [(f32, f32); 22],
        stamina: &'a [f32; 22],
        ball_owner: Option<u8>,
    ) -> Self {
        let is_home = player_idx < 11;

        Self {
            ball: BallOps::new(ball, positions),
            player: PlayerOps::new(player_idx, positions, velocities, stamina, ball_owner),
            players: PlayersOps::new(player_idx, positions, ball_owner),
            team: TeamOps::new(is_home, ball, positions, ball_owner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    fn make_test_positions() -> [Coord10; 22] {
        const CY: f32 = field::CENTER_Y;
        std::array::from_fn(|i| {
            let x = if i < 11 { 30.0 + (i as f32 * 5.0) } else { 70.0 - ((i - 11) as f32 * 5.0) };
            let y = CY + ((i % 5) as f32 * 5.0);
            Coord10::from_meters(x, y)
        })
    }

    #[test]
    fn test_operations_context_creation() {
        let positions = make_test_positions();
        let velocities = [(0.0, 0.0); 22];
        let stamina = [1.0; 22];
        let ball = Ball::default();

        let ctx = OperationsContext::new(5, &ball, &positions, &velocities, &stamina, None);

        assert_eq!(ctx.player.index(), 5);
        assert!(ctx.player.is_home());
        assert!(!ctx.ball.is_owned());
    }
}
