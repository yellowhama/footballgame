//! Offside Trap AI Module
//!
//! FIX_2601/0107: Defensive line control and offside trap based on open-football.
//!
//! Key features:
//! - Defensive line height calculation
//! - Offside trap activation/detection
//! - Position scoring with onside bonus
//! - Trap breaking for attackers

use crate::engine::tactical_context::TeamSide;
use crate::engine::types::Coord10;

/// Defensive line state for offside trap management
#[derive(Debug, Clone, Default)]
pub struct DefensiveLineState {
    /// Current defensive line Y position (Coord10 units)
    pub line_y: i32,
    /// Whether trap is currently active
    pub trap_active: bool,
    /// Teamwork-based coordination bonus (0.0-1.0)
    pub coordination: f32,
    /// Last tick when line was calculated
    pub last_update_tick: u64,
}

/// Offside trap configuration
#[derive(Debug, Clone)]
pub struct OffsideTrapConfig {
    /// Minimum line height from own goal (Coord10 units)
    pub min_line_height: i32,
    /// Maximum line height (how far to push up)
    pub max_line_height: i32,
    /// Teamwork threshold to enable trap (0-100)
    pub teamwork_threshold: u8,
    /// Concentration threshold for trap timing (0-100)
    pub concentration_threshold: u8,
}

impl Default for OffsideTrapConfig {
    fn default() -> Self {
        Self {
            min_line_height: 200,  // ~20m from goal
            max_line_height: 600,  // ~60m from goal (halfway line)
            teamwork_threshold: 65,
            concentration_threshold: 60,
        }
    }
}

impl DefensiveLineState {
    /// Create new defensive line state
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate defensive line Y position based on defender positions
    ///
    /// # Arguments
    /// * `defenders` - Positions of all defenders (excluding GK)
    /// * `attack_direction` - +1.0 if defending toward higher Y, -1.0 otherwise
    pub fn calculate_line_height(
        &mut self,
        defenders: &[Coord10],
        attack_direction: f32,
        current_tick: u64,
    ) {
        if defenders.is_empty() {
            return;
        }

        // Find the defensive line (average of back 4, or last defender)
        let mut defender_ys: Vec<i32> = defenders.iter().map(|d| d.y).collect();

        if attack_direction > 0.0 {
            // Defending toward higher Y (opponent attacks from low Y)
            defender_ys.sort(); // Lowest Y first
        } else {
            // Defending toward lower Y
            defender_ys.sort_by(|a, b| b.cmp(a)); // Highest Y first
        }

        // Take average of back 4 (or all if fewer)
        let count = defender_ys.len().min(4);
        let sum: i32 = defender_ys.iter().take(count).sum();
        self.line_y = sum / count as i32;
        self.last_update_tick = current_tick;
    }

    /// Check if offside trap should be activated
    ///
    /// # Arguments
    /// * `ball_pos` - Current ball position
    /// * `opponent_attackers` - Positions of opponent forwards
    /// * `avg_teamwork` - Average teamwork stat of defenders (0-100)
    /// * `avg_concentration` - Average concentration stat (0-100)
    pub fn should_activate_trap(
        &self,
        ball_pos: Coord10,
        opponent_attackers: &[Coord10],
        avg_teamwork: u8,
        avg_concentration: u8,
        config: &OffsideTrapConfig,
    ) -> bool {
        // Need sufficient teamwork and concentration
        if avg_teamwork < config.teamwork_threshold {
            return false;
        }
        if avg_concentration < config.concentration_threshold {
            return false;
        }

        // Ball should be in midfield or opponent half (not near our goal)
        let ball_in_safe_zone = ball_pos.y > 300 && ball_pos.y < 700;
        if !ball_in_safe_zone {
            return false;
        }

        // Check if attackers are near the line (trap opportunity)
        let attackers_near_line = opponent_attackers
            .iter()
            .filter(|a| (a.y - self.line_y).abs() < 50) // Within 5m of line
            .count();

        attackers_near_line >= 1
    }

    /// Calculate optimal line position for trap
    pub fn calculate_trap_line(
        &self,
        ball_pos: Coord10,
        attack_direction: f32,
        config: &OffsideTrapConfig,
    ) -> i32 {
        // Push line up based on ball position
        let base_line = if attack_direction > 0.0 {
            // Defending toward higher Y
            ball_pos.y.min(config.max_line_height)
        } else {
            // Defending toward lower Y
            ball_pos.y.max(1000 - config.max_line_height)
        };

        base_line.clamp(config.min_line_height, 1000 - config.min_line_height)
    }
}

/// Check if a position would be offside
///
/// # Arguments
/// * `pos` - Position to check
/// * `defensive_line_y` - Y position of defensive line
/// * `attack_direction` - +1.0 if attacker moves toward higher Y
/// * `ball_y` - Ball Y position (can't be offside if behind ball)
pub fn would_be_offside(
    pos: Coord10,
    defensive_line_y: i32,
    attack_direction: f32,
    ball_y: i32,
) -> bool {
    // Offside buffer (2m = 20 Coord10 units)
    const OFFSIDE_BUFFER: i32 = 20;

    if attack_direction > 0.0 {
        // Attacking toward higher Y
        // Offside if player is beyond defensive line AND beyond ball
        pos.y > defensive_line_y + OFFSIDE_BUFFER && pos.y > ball_y
    } else {
        // Attacking toward lower Y
        pos.y < defensive_line_y - OFFSIDE_BUFFER && pos.y < ball_y
    }
}

/// Calculate position score with offside consideration
///
/// # Arguments
/// * `pos` - Position to evaluate
/// * `defensive_line_y` - Defensive line position
/// * `attack_direction` - Attack direction
/// * `ball_y` - Ball Y position
///
/// # Returns
/// Score bonus/penalty for position (positive = better)
pub fn offside_position_score(
    pos: Coord10,
    defensive_line_y: i32,
    attack_direction: f32,
    ball_y: i32,
) -> f32 {
    if would_be_offside(pos, defensive_line_y, attack_direction, ball_y) {
        // Penalty for offside position
        -20.0
    } else {
        // Bonus for staying onside
        15.0
    }
}

/// Find best position to break offside trap
///
/// # Arguments
/// * `player_pos` - Current player position
/// * `defensive_line_y` - Defensive line position
/// * `attack_direction` - Attack direction
///
/// # Returns
/// Target position just onside of defensive line
pub fn find_trap_breaking_position(
    player_pos: Coord10,
    defensive_line_y: i32,
    attack_direction: f32,
) -> Coord10 {
    // Position just behind the defensive line
    const SAFE_MARGIN: i32 = 10; // 1m behind line

    let target_y = if attack_direction > 0.0 {
        defensive_line_y - SAFE_MARGIN
    } else {
        defensive_line_y + SAFE_MARGIN
    };

    Coord10 {
        x: player_pos.x,
        y: target_y.clamp(50, 950),
        z: 0,
    }
}

/// Get normalized defensive line height (0.0 to 1.0)
///
/// 0.0 = deep defending (near own goal)
/// 1.0 = high line (near halfway)
pub fn get_defensive_line_height_normalized(
    defenders: &[Coord10],
    team_side: TeamSide,
) -> f32 {
    if defenders.is_empty() {
        return 0.5;
    }

    let avg_y: f32 = defenders.iter().map(|d| d.y as f32).sum::<f32>() / defenders.len() as f32;

    match team_side {
        TeamSide::Home => {
            // Home defends toward Y=0, so lower Y = deeper
            (avg_y / 1000.0).clamp(0.0, 1.0)
        }
        TeamSide::Away => {
            // Away defends toward Y=1000, so higher Y = deeper
            (1.0 - avg_y / 1000.0).clamp(0.0, 1.0)
        }
    }
}

/// Check if player is behind defensive line
pub fn is_behind_defensive_line(
    player_y: i32,
    defensive_line_y: i32,
    attack_direction: f32,
) -> bool {
    if attack_direction > 0.0 {
        player_y > defensive_line_y + 20 // Behind = deeper into opponent territory
    } else {
        player_y < defensive_line_y - 20
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defensive_line_calculation() {
        let mut state = DefensiveLineState::new();
        let defenders = vec![
            Coord10 { x: 200, y: 300, z: 0 },
            Coord10 { x: 400, y: 320, z: 0 },
            Coord10 { x: 600, y: 280, z: 0 },
            Coord10 { x: 800, y: 310, z: 0 },
        ];

        state.calculate_line_height(&defenders, 1.0, 100);

        // Average of 280, 300, 310, 320 = 302.5 ≈ 302
        assert!((state.line_y - 302).abs() <= 1);
    }

    #[test]
    fn test_offside_detection() {
        let defensive_line = 500;
        let ball_y = 400;

        // Player ahead of line and ball - offside
        let offside_pos = Coord10 { x: 500, y: 550, z: 0 };
        assert!(would_be_offside(offside_pos, defensive_line, 1.0, ball_y));

        // Player behind line - onside
        let onside_pos = Coord10 { x: 500, y: 480, z: 0 };
        assert!(!would_be_offside(onside_pos, defensive_line, 1.0, ball_y));

        // Player ahead of line but behind ball - onside
        let behind_ball = Coord10 { x: 500, y: 380, z: 0 };
        assert!(!would_be_offside(behind_ball, defensive_line, 1.0, ball_y));
    }

    #[test]
    fn test_trap_breaking_position() {
        let player = Coord10 { x: 500, y: 600, z: 0 };
        let defensive_line = 500;

        let target = find_trap_breaking_position(player, defensive_line, 1.0);

        // Should be just behind the line
        assert!(target.y < defensive_line);
        assert_eq!(target.y, 490); // 500 - 10
    }

    #[test]
    fn test_trap_activation() {
        let state = DefensiveLineState {
            line_y: 500,
            trap_active: false,
            coordination: 0.8,
            last_update_tick: 0,
        };

        let ball = Coord10 { x: 500, y: 500, z: 0 };
        let attackers = vec![Coord10 { x: 500, y: 520, z: 0 }]; // Near line
        let config = OffsideTrapConfig::default();

        // High teamwork and concentration
        assert!(state.should_activate_trap(ball, &attackers, 70, 70, &config));

        // Low teamwork
        assert!(!state.should_activate_trap(ball, &attackers, 50, 70, &config));
    }

    #[test]
    fn test_position_score() {
        let defensive_line = 500;
        let ball_y = 400;

        // Onside position gets bonus
        let onside = Coord10 { x: 500, y: 480, z: 0 };
        let score = offside_position_score(onside, defensive_line, 1.0, ball_y);
        assert_eq!(score, 15.0);

        // Offside position gets penalty
        let offside = Coord10 { x: 500, y: 550, z: 0 };
        let score = offside_position_score(offside, defensive_line, 1.0, ball_y);
        assert_eq!(score, -20.0);
    }
}
