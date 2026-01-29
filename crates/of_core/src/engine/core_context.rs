//! Core context shared between fast and slow evaluation
//!
//! FIX_2512 Phase 16: Context Unification
//! FIX_2601: Migrated to Coord10
//!
//! This module provides a shared context struct to reduce duplication
//! between PositionContext (fast evaluation) and SlowContext (slow evaluation).

use crate::engine::types::coord10::Coord10;

/// Core context shared between fast and slow evaluation
///
/// Contains the essential player state information needed by both
/// PositionContext and SlowContext. This is a Copy type for efficient passing.
#[derive(Debug, Clone, Copy)]
pub struct CoreContext {
    /// Player index (0-21)
    pub player_idx: usize,

    /// Whether player is on home team (idx < 11)
    pub is_home: bool,
    /// Whether player's team attacks toward x=105
    pub attacks_right: bool,

    /// Player position (FIX_2601: Coord10)
    pub player_position: Coord10,

    /// Ball position (FIX_2601: Coord10)
    pub ball_position: Coord10,

    /// Distance from player to ball in meters
    pub ball_distance: f32,

    /// Whether player's team currently has possession
    pub team_has_ball: bool,

    /// Whether this specific player has the ball
    pub player_has_ball: bool,
}

impl CoreContext {
    /// Create a new CoreContext
    ///
    /// # Arguments
    /// * `player_idx` - Player index (0-21)
    /// * `player_position` - Player position (Coord10)
    /// * `ball_position` - Ball position (Coord10)
    /// * `ball_owner` - Index of current ball owner (if any)
    #[inline]
    pub fn new(
        player_idx: usize,
        player_position: Coord10,
        ball_position: Coord10,
        ball_owner: Option<usize>,
        attacks_right: bool,
    ) -> Self {
        let is_home = player_idx < 11;
        let team_has_ball = ball_owner.is_some_and(|o| (o < 11) == is_home);
        let player_has_ball = ball_owner == Some(player_idx);
        // Calculate distance in meters from Coord10
        let (px, py) = player_position.to_meters();
        let (bx, by) = ball_position.to_meters();
        let ball_distance = ((px - bx).powi(2) + (py - by).powi(2)).sqrt();

        Self {
            player_idx,
            is_home,
            attacks_right,
            player_position,
            ball_position,
            ball_distance,
            team_has_ball,
            player_has_ball,
        }
    }

    /// Get opponent goal position (FIX_2601: Returns Coord10)
    /// FIX_2512_1229: Goals are at (x, y) where x=0 or 105 (ends), y=34 (center)
    #[inline]
    pub fn opponent_goal(&self) -> Coord10 {
        if self.attacks_right {
            Coord10::new(goal::AWAY_CENTER_COORD10.0, goal::AWAY_CENTER_COORD10.1) // Attacks toward right goal
        } else {
            Coord10::new(goal::HOME_CENTER_COORD10.0, goal::HOME_CENTER_COORD10.1) // Attacks toward left goal
        }
    }

    /// Get own goal position (FIX_2601: Returns Coord10)
    /// FIX_2512_1229: Goals are at (x, y) where x=0 or 105 (ends), y=34 (center)
    #[inline]
    pub fn own_goal(&self) -> Coord10 {
        if self.attacks_right {
            Coord10::new(goal::HOME_CENTER_COORD10.0, goal::HOME_CENTER_COORD10.1) // Defends left goal
        } else {
            Coord10::new(goal::AWAY_CENTER_COORD10.0, goal::AWAY_CENTER_COORD10.1) // Defends right goal
        }
    }

    /// Check if player is in attacking half (FIX_2601: Uses Coord10)
    #[inline]
    pub fn in_attacking_half(&self) -> bool {
        // FIX_2512_1229: Use X-axis for length direction
        if self.attacks_right {
            self.player_position.x > Coord10::CENTER_X
        } else {
            self.player_position.x < Coord10::CENTER_X
        }
    }
}

impl Default for CoreContext {
    fn default() -> Self {
        Self {
            player_idx: 0,
            is_home: true,
            attacks_right: true,
            player_position: Coord10::ZERO,
            ball_position: Coord10::ZERO,
            ball_distance: 0.0,
            team_has_ball: false,
            player_has_ball: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_core_context_new() {
        let player_pos = Coord10::from_meters(30.0, 40.0);
        let ball_pos = Coord10::from_meters(35.0, 40.0);
        let ctx = CoreContext::new(5, player_pos, ball_pos, Some(5), true);

        assert_eq!(ctx.player_idx, 5);
        assert!(ctx.is_home);
        assert_eq!(ctx.player_position, player_pos);
        assert_eq!(ctx.ball_position, ball_pos);
        assert!((ctx.ball_distance - 5.0).abs() < 0.001);
        assert!(ctx.team_has_ball);
        assert!(ctx.player_has_ball);
    }

    #[test]
    fn test_core_context_away_team() {
        let pos = Coord10::from_meters(50.0, 50.0);
        let ctx = CoreContext::new(15, pos, pos, Some(11), false);

        assert_eq!(ctx.player_idx, 15);
        assert!(!ctx.is_home);
        assert!(ctx.team_has_ball); // Player 11 is on away team
        assert!(!ctx.player_has_ball); // Player 15 doesn't have ball
    }

    #[test]
    fn test_core_context_no_ball_owner() {
        let player_pos = Coord10::from_meters(25.0, 25.0);
        let ball_pos = Coord10::from_meters(50.0, 50.0);
        let ctx = CoreContext::new(0, player_pos, ball_pos, None, true);

        assert!(!ctx.team_has_ball);
        assert!(!ctx.player_has_ball);
    }

    #[test]
    fn test_opponent_goal() {
        let origin = Coord10::ZERO;
        let home = CoreContext::new(0, origin, origin, None, true);
        let away = CoreContext::new(11, origin, origin, None, false);

        // FIX_2512_1229: Goals at (x, y) = (0/105, 34) → Coord10
        assert_eq!(home.opponent_goal(), Coord10::new(goal::AWAY_CENTER_COORD10.0, goal::AWAY_CENTER_COORD10.1));
        assert_eq!(away.opponent_goal(), Coord10::new(goal::HOME_CENTER_COORD10.0, goal::HOME_CENTER_COORD10.1));
    }

    #[test]
    fn test_own_goal() {
        let origin = Coord10::ZERO;
        let home = CoreContext::new(0, origin, origin, None, true);
        let away = CoreContext::new(11, origin, origin, None, false);

        // FIX_2512_1229: Goals at (x, y) = (0/105, 34) → Coord10
        assert_eq!(home.own_goal(), Coord10::new(goal::HOME_CENTER_COORD10.0, goal::HOME_CENTER_COORD10.1));
        assert_eq!(away.own_goal(), Coord10::new(goal::AWAY_CENTER_COORD10.0, goal::AWAY_CENTER_COORD10.1));
    }

    #[test]
    fn test_in_attacking_half() {
        let origin = Coord10::ZERO;
        // FIX_2512_1229: Attacking half is based on X-axis
        // Home attacks toward x=field::LENGTH_M, so x > field::CENTER_X is attacking half
        let home_attacking = CoreContext::new(5, Coord10::from_meters(60.0, field::CENTER_Y), origin, None, true);
        assert!(home_attacking.in_attacking_half());

        // Home player in defensive half (x < field::CENTER_X)
        let home_defensive = CoreContext::new(5, Coord10::from_meters(40.0, field::CENTER_Y), origin, None, true);
        assert!(!home_defensive.in_attacking_half());

        // Away attacks toward x=0, so x < field::CENTER_X is attacking half
        let away_attacking = CoreContext::new(15, Coord10::from_meters(40.0, field::CENTER_Y), origin, None, false);
        assert!(away_attacking.in_attacking_half());

        // Away player in defensive half (x > field::CENTER_X)
        let away_defensive = CoreContext::new(15, Coord10::from_meters(60.0, field::CENTER_Y), origin, None, false);
        assert!(!away_defensive.in_attacking_half());
    }
}
