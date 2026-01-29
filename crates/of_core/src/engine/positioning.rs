//! Dynamic positioning system with waypoints
//!
//! Provides position waypoints that allow players to adjust their positions
//! based on instructions, game state, and tactical situations.

use crate::player::instructions::{Depth, PlayerInstructions, Width};

/// Position waypoints for dynamic movement
///
/// Each position has multiple waypoints representing different tactical positions
/// the player can take based on instructions and game state.
#[derive(Clone, Debug)]
pub struct PositionWaypoints {
    /// Base/default position
    pub base: (f32, f32),
    /// Defensive position (deeper)
    pub defensive: (f32, f32),
    /// Offensive position (higher up)
    pub offensive: (f32, f32),
    /// Left shift position
    pub left_shift: (f32, f32),
    /// Right shift position
    pub right_shift: (f32, f32),
}

impl PositionWaypoints {
    /// Create new position waypoints
    pub fn new(
        base: (f32, f32),
        defensive: (f32, f32),
        offensive: (f32, f32),
        left_shift: (f32, f32),
        right_shift: (f32, f32),
    ) -> Self {
        Self { base, defensive, offensive, left_shift, right_shift }
    }

    /// Create waypoints with automatic offsets from base position
    pub fn from_base(base: (f32, f32), depth_offset: f32, width_offset: f32) -> Self {
        Self {
            base,
            defensive: (base.0, (base.1 - depth_offset).max(0.05)),
            offensive: (base.0, (base.1 + depth_offset).min(0.95)),
            left_shift: ((base.0 - width_offset).max(0.05), base.1),
            right_shift: ((base.0 + width_offset).min(0.95), base.1),
        }
    }

    /// Select appropriate waypoint based on instructions and game state
    ///
    /// # Arguments
    /// * `instructions` - Player's individual instructions
    /// * `ball_position` - Current ball position (normalized 0-1)
    /// * `score_diff` - Score difference (positive = winning)
    /// * `minute` - Current match minute
    ///
    /// # Returns
    /// Selected position as (x, y) coordinates (normalized 0-1)
    pub fn select_waypoint(
        &self,
        instructions: &PlayerInstructions,
        ball_position: (f32, f32),
        score_diff: i8,
        minute: u8,
    ) -> (f32, f32) {
        // Start with depth-based position
        let depth_pos = match instructions.depth {
            Depth::StayBack => self.defensive,
            Depth::Balanced => self.base,
            Depth::GetForward => self.offensive,
        };

        // Apply width adjustment
        let width_adjusted = match instructions.width {
            Width::StayWide => {
                // Move towards the nearest sideline
                if depth_pos.0 < 0.5 {
                    ((depth_pos.0 + self.left_shift.0) / 2.0, depth_pos.1)
                } else {
                    ((depth_pos.0 + self.right_shift.0) / 2.0, depth_pos.1)
                }
            }
            Width::CutInside => {
                // Move towards center
                (depth_pos.0 * 0.7 + 0.5 * 0.3, depth_pos.1)
            }
            Width::Roam => {
                // Slight movement towards ball
                let ball_influence = 0.1;
                (
                    depth_pos.0 * (1.0 - ball_influence) + ball_position.0 * ball_influence,
                    depth_pos.1,
                )
            }
            Width::Normal => depth_pos,
        };

        // Apply game situation adjustments
        let situation_adjusted =
            self.apply_situation_adjustment(width_adjusted, score_diff, minute);

        // Clamp to valid field positions
        (situation_adjusted.0.clamp(0.05, 0.95), situation_adjusted.1.clamp(0.05, 0.95))
    }

    /// Apply adjustments based on game situation
    fn apply_situation_adjustment(
        &self,
        pos: (f32, f32),
        score_diff: i8,
        minute: u8,
    ) -> (f32, f32) {
        let mut adjusted = pos;

        // Late game adjustments
        if minute >= 75 {
            if score_diff < 0 {
                // Losing late - push forward
                let urgency = ((minute - 75) as f32 / 15.0) * 0.08;
                adjusted.1 = (adjusted.1 + urgency).min(0.95);
            } else if score_diff > 0 && minute >= 85 {
                // Winning very late - drop back
                let caution = ((minute - 85) as f32 / 10.0) * 0.06;
                adjusted.1 = (adjusted.1 - caution).max(0.05);
            }
        }

        // Ball position influence (slight)
        // Players naturally shift towards ball's side of field
        // This is subtle - major movement is handled by select_waypoint

        adjusted
    }

    /// Get interpolated position between two waypoints
    pub fn interpolate(&self, from: (f32, f32), to: (f32, f32), t: f32) -> (f32, f32) {
        let t = t.clamp(0.0, 1.0);
        (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
    }
}

/// Position key for identifying field positions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PositionKey {
    GK,
    LB,
    LCB,
    CB,
    RCB,
    RB,
    LWB,
    RWB,
    CDM,
    LDM,
    RDM,
    LM,
    LCM,
    CM,
    RCM,
    RM,
    LAM,
    CAM,
    RAM,
    LW,
    RW,
    LF,
    CF,
    RF,
    ST,
}

impl PositionKey {
    /// Parse position key from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GK" => Some(Self::GK),
            "LB" => Some(Self::LB),
            "LCB" => Some(Self::LCB),
            "CB" => Some(Self::CB),
            "RCB" => Some(Self::RCB),
            "RB" => Some(Self::RB),
            "LWB" => Some(Self::LWB),
            "RWB" => Some(Self::RWB),
            "CDM" | "DM" => Some(Self::CDM),
            "LDM" => Some(Self::LDM),
            "RDM" => Some(Self::RDM),
            "LM" => Some(Self::LM),
            "LCM" => Some(Self::LCM),
            "CM" => Some(Self::CM),
            "RCM" => Some(Self::RCM),
            "RM" => Some(Self::RM),
            "LAM" => Some(Self::LAM),
            "CAM" | "AM" => Some(Self::CAM),
            "RAM" => Some(Self::RAM),
            "LW" => Some(Self::LW),
            "RW" => Some(Self::RW),
            "LF" => Some(Self::LF),
            "CF" => Some(Self::CF),
            "RF" => Some(Self::RF),
            "ST" => Some(Self::ST),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_waypoints() -> PositionWaypoints {
        PositionWaypoints::new(
            (0.5, 0.5), // base
            (0.5, 0.4), // defensive
            (0.5, 0.6), // offensive
            (0.3, 0.5), // left
            (0.7, 0.5), // right
        )
    }

    #[test]
    fn test_depth_selection() {
        let waypoints = create_test_waypoints();

        let stay_back = PlayerInstructions { depth: Depth::StayBack, ..Default::default() };
        let pos = waypoints.select_waypoint(&stay_back, (0.5, 0.5), 0, 45);
        assert!(pos.1 < 0.5, "StayBack should select defensive position");

        let get_forward = PlayerInstructions { depth: Depth::GetForward, ..Default::default() };
        let pos = waypoints.select_waypoint(&get_forward, (0.5, 0.5), 0, 45);
        assert!(pos.1 > 0.5, "GetForward should select offensive position");
    }

    #[test]
    fn test_width_stay_wide() {
        let waypoints = PositionWaypoints::new(
            (0.3, 0.5), // base (left side)
            (0.3, 0.4),
            (0.3, 0.6),
            (0.15, 0.5), // left shift
            (0.45, 0.5),
        );

        let stay_wide = PlayerInstructions { width: Width::StayWide, ..Default::default() };
        let pos = waypoints.select_waypoint(&stay_wide, (0.5, 0.5), 0, 45);
        assert!(pos.0 < 0.3, "StayWide on left should move further left");
    }

    #[test]
    fn test_cut_inside() {
        let waypoints = PositionWaypoints::new(
            (0.2, 0.7), // base (left wing)
            (0.2, 0.6),
            (0.2, 0.8),
            (0.1, 0.7),
            (0.35, 0.7),
        );

        let cut_inside = PlayerInstructions { width: Width::CutInside, ..Default::default() };
        let pos = waypoints.select_waypoint(&cut_inside, (0.5, 0.5), 0, 45);
        assert!(pos.0 > 0.2, "CutInside should move towards center");
    }

    #[test]
    fn test_late_game_losing_adjustment() {
        let waypoints = create_test_waypoints();
        let instructions = PlayerInstructions::default();

        let early_pos = waypoints.select_waypoint(&instructions, (0.5, 0.5), -1, 60);
        let late_pos = waypoints.select_waypoint(&instructions, (0.5, 0.5), -1, 88);

        assert!(
            late_pos.1 > early_pos.1,
            "Losing late should push forward: {} vs {}",
            late_pos.1,
            early_pos.1
        );
    }

    #[test]
    fn test_late_game_winning_adjustment() {
        let waypoints = create_test_waypoints();
        let instructions = PlayerInstructions::default();

        let early_pos = waypoints.select_waypoint(&instructions, (0.5, 0.5), 2, 80);
        let late_pos = waypoints.select_waypoint(&instructions, (0.5, 0.5), 2, 90);

        assert!(
            late_pos.1 < early_pos.1,
            "Winning late should drop back: {} vs {}",
            late_pos.1,
            early_pos.1
        );
    }

    #[test]
    fn test_from_base_creation() {
        let waypoints = PositionWaypoints::from_base((0.5, 0.5), 0.1, 0.15);

        assert_eq!(waypoints.base, (0.5, 0.5));
        assert_eq!(waypoints.defensive, (0.5, 0.4));
        assert_eq!(waypoints.offensive, (0.5, 0.6));
        assert_eq!(waypoints.left_shift, (0.35, 0.5));
        assert_eq!(waypoints.right_shift, (0.65, 0.5));
    }

    #[test]
    fn test_position_clamping() {
        let waypoints = PositionWaypoints::new(
            (0.02, 0.98), // Near corner
            (0.02, 0.95),
            (0.02, 1.0), // Would exceed bounds
            (0.0, 0.98), // Would exceed bounds
            (0.1, 0.98),
        );

        let instructions = PlayerInstructions {
            depth: Depth::GetForward,
            width: Width::StayWide,
            ..Default::default()
        };
        let pos = waypoints.select_waypoint(&instructions, (0.5, 0.5), 0, 45);

        assert!(pos.0 >= 0.05 && pos.0 <= 0.95, "X should be clamped: {}", pos.0);
        assert!(pos.1 >= 0.05 && pos.1 <= 0.95, "Y should be clamped: {}", pos.1);
    }

    #[test]
    fn test_interpolation() {
        let waypoints = create_test_waypoints();
        let from = (0.0, 0.0);
        let to = (1.0, 1.0);

        let mid = waypoints.interpolate(from, to, 0.5);
        assert!((mid.0 - 0.5).abs() < 0.001);
        assert!((mid.1 - 0.5).abs() < 0.001);
    }
}
