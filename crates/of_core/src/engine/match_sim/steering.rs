//! Steering Behaviors Module
//!
//! FIX_2601/0107: Curved movement patterns based on open-football implementation.
//! Provides smooth, natural-looking player movements.
//!
//! Key features:
//! - 6 steering behaviors (Seek, Arrive, Pursuit, Evade, Wander, FollowPath)
//! - Movement patterns (DirectRun, DiagonalRun, ChannelRun, DriftWide, CheckToFeet, OppositeMovement)
//! - Skill-based speed adjustments

use crate::engine::types::Coord10;

/// Steering behavior types for player movement
#[derive(Debug, Clone)]
pub enum SteeringBehavior {
    /// Move directly toward target at full speed
    Seek { target: Coord10 },
    /// Move toward target, slowing down as approaching
    Arrive { target: Coord10, slowing_distance: i32 },
    /// Intercept moving target (anticipate position)
    Pursuit { target: Coord10, target_velocity: (i32, i32) },
    /// Move away from threat
    Evade { threat: Coord10, threat_velocity: (i32, i32) },
    /// Random wandering movement
    Wander { center: Coord10, radius: i32 },
    /// Follow a series of waypoints
    FollowPath { waypoints: Vec<Coord10>, current_index: usize },
}

/// Forward movement patterns from open-football
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardMovementPattern {
    /// Direct run behind defense
    DirectRun,
    /// Diagonal run to create space and angles
    DiagonalRun,
    /// Run between defenders (through channel)
    ChannelRun,
    /// Drift wide to create central space
    DriftWide,
    /// Come short to receive (check to feet)
    CheckToFeet,
    /// Move opposite to defensive shift
    OppositeMovement,
}

impl SteeringBehavior {
    /// Calculate next position based on steering behavior
    ///
    /// # Arguments
    /// * `current_pos` - Current player position
    /// * `max_speed` - Maximum movement speed (Coord10 units per tick)
    /// * `acceleration` - Skill-based acceleration factor (0.0-1.0)
    ///
    /// # Returns
    /// New position after applying steering
    pub fn calculate_next_position(
        &self,
        current_pos: Coord10,
        max_speed: i32,
        acceleration: f32,
    ) -> Coord10 {
        match self {
            SteeringBehavior::Seek { target } => {
                seek(current_pos, *target, max_speed, acceleration)
            }
            SteeringBehavior::Arrive { target, slowing_distance } => {
                arrive(current_pos, *target, *slowing_distance, max_speed, acceleration)
            }
            SteeringBehavior::Pursuit { target, target_velocity } => {
                pursuit(current_pos, *target, *target_velocity, max_speed, acceleration)
            }
            SteeringBehavior::Evade { threat, threat_velocity } => {
                evade(current_pos, *threat, *threat_velocity, max_speed, acceleration)
            }
            SteeringBehavior::Wander { center, radius } => {
                wander(current_pos, *center, *radius, max_speed)
            }
            SteeringBehavior::FollowPath { waypoints, current_index } => {
                if *current_index < waypoints.len() {
                    seek(current_pos, waypoints[*current_index], max_speed, acceleration)
                } else {
                    current_pos
                }
            }
        }
    }
}

/// Seek: Move directly toward target at full speed
fn seek(current: Coord10, target: Coord10, max_speed: i32, acceleration: f32) -> Coord10 {
    let dx = target.x - current.x;
    let dy = target.y - current.y;
    let dist = ((dx * dx + dy * dy) as f32).sqrt();

    if dist < 1.0 {
        return target;
    }

    // Normalize and scale by speed
    let speed = (max_speed as f32 * acceleration).max(1.0);
    let move_dist = speed.min(dist);

    let nx = (dx as f32 / dist * move_dist) as i32;
    let ny = (dy as f32 / dist * move_dist) as i32;

    Coord10 {
        x: (current.x + nx).clamp(0, Coord10::FIELD_LENGTH_10),
        y: (current.y + ny).clamp(0, Coord10::FIELD_WIDTH_10),
        z: 0,
    }
}

/// Arrive: Move toward target, slowing down as approaching
fn arrive(
    current: Coord10,
    target: Coord10,
    slowing_distance: i32,
    max_speed: i32,
    acceleration: f32,
) -> Coord10 {
    let dx = target.x - current.x;
    let dy = target.y - current.y;
    let dist = ((dx * dx + dy * dy) as f32).sqrt();

    if dist < 5.0 {
        return target;
    }

    // Scale speed based on distance to target
    let speed_factor = if dist < slowing_distance as f32 {
        dist / slowing_distance as f32
    } else {
        1.0
    };

    let speed = (max_speed as f32 * acceleration * speed_factor).max(1.0);
    let move_dist = speed.min(dist);

    let nx = (dx as f32 / dist * move_dist) as i32;
    let ny = (dy as f32 / dist * move_dist) as i32;

    Coord10 {
        x: (current.x + nx).clamp(0, Coord10::FIELD_LENGTH_10),
        y: (current.y + ny).clamp(0, Coord10::FIELD_WIDTH_10),
        z: 0,
    }
}

/// Pursuit: Intercept moving target by anticipating position
fn pursuit(
    current: Coord10,
    target: Coord10,
    target_velocity: (i32, i32),
    max_speed: i32,
    acceleration: f32,
) -> Coord10 {
    let dx = target.x - current.x;
    let dy = target.y - current.y;
    let dist = ((dx * dx + dy * dy) as f32).sqrt();

    // Predict target position based on distance/speed
    let prediction_ticks = (dist / max_speed as f32).min(10.0) as i32;
    let predicted_target = Coord10 {
        x: target.x + target_velocity.0 * prediction_ticks,
        y: target.y + target_velocity.1 * prediction_ticks,
        z: 0,
    };

    seek(current, predicted_target, max_speed, acceleration)
}

/// Evade: Move away from threat
fn evade(
    current: Coord10,
    threat: Coord10,
    threat_velocity: (i32, i32),
    max_speed: i32,
    acceleration: f32,
) -> Coord10 {
    // Predict threat position
    let prediction_ticks = 5;
    let predicted_threat = Coord10 {
        x: threat.x + threat_velocity.0 * prediction_ticks,
        y: threat.y + threat_velocity.1 * prediction_ticks,
        z: 0,
    };

    // Move in opposite direction
    let dx = current.x - predicted_threat.x;
    let dy = current.y - predicted_threat.y;
    let dist = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);

    let speed = (max_speed as f32 * acceleration).max(1.0);
    let nx = (dx as f32 / dist * speed) as i32;
    let ny = (dy as f32 / dist * speed) as i32;

    Coord10 {
        x: (current.x + nx).clamp(0, Coord10::FIELD_LENGTH_10),
        y: (current.y + ny).clamp(0, Coord10::FIELD_WIDTH_10),
        z: 0,
    }
}

/// Wander: Random movement within radius of center
fn wander(current: Coord10, center: Coord10, radius: i32, max_speed: i32) -> Coord10 {
    // Simple implementation: move toward center if too far, otherwise small random movement
    let dx = center.x - current.x;
    let dy = center.y - current.y;
    let dist = ((dx * dx + dy * dy) as f32).sqrt();

    if dist > radius as f32 {
        // Move back toward center
        seek(current, center, max_speed / 2, 0.5)
    } else {
        // Small random offset (deterministic based on position)
        let hash = ((current.x * 7 + current.y * 13) % 100) as f32 / 100.0;
        let angle = hash * std::f32::consts::TAU;
        let move_dist = (max_speed / 4) as f32;

        Coord10 {
            x: (current.x + (angle.cos() * move_dist) as i32).clamp(0, Coord10::FIELD_LENGTH_10),
            y: (current.y + (angle.sin() * move_dist) as i32).clamp(0, Coord10::FIELD_WIDTH_10),
            z: 0,
        }
    }
}

impl ForwardMovementPattern {
    /// Calculate target position for forward movement pattern
    ///
    /// # Arguments
    /// * `player_pos` - Current player position
    /// * `ball_pos` - Current ball position
    /// * `defenders` - Positions of opponent defenders
    /// * `attack_direction` - +1.0 for home (toward higher Y), -1.0 for away
    pub fn calculate_target(
        &self,
        player_pos: Coord10,
        ball_pos: Coord10,
        defenders: &[Coord10],
        attack_direction: f32,
    ) -> Coord10 {
        match self {
            ForwardMovementPattern::DirectRun => {
                // Sprint directly behind defensive line
                calculate_direct_run_target(player_pos, attack_direction)
            }
            ForwardMovementPattern::DiagonalRun => {
                // Diagonal run across defense
                calculate_diagonal_run_target(player_pos, ball_pos, attack_direction)
            }
            ForwardMovementPattern::ChannelRun => {
                // Find gap between defenders
                calculate_channel_run_target(player_pos, ball_pos, defenders, attack_direction)
            }
            ForwardMovementPattern::DriftWide => {
                // Drift to wing to create space
                calculate_drift_wide_target(player_pos, ball_pos)
            }
            ForwardMovementPattern::CheckToFeet => {
                // Come short toward ball
                calculate_check_to_feet_target(player_pos, ball_pos, attack_direction)
            }
            ForwardMovementPattern::OppositeMovement => {
                // Move opposite to defensive shift
                calculate_opposite_movement_target(player_pos, defenders)
            }
        }
    }

    /// Select best movement pattern based on context
    pub fn select_pattern(
        player_pos: Coord10,
        ball_pos: Coord10,
        defenders: &[Coord10],
        is_wide_player: bool,
        ball_holder_under_pressure: bool,
    ) -> Self {
        // If ball holder under pressure, check to feet for support
        if ball_holder_under_pressure {
            return ForwardMovementPattern::CheckToFeet;
        }

        // Wide players drift wide or make diagonal runs
        // FIX_2601/0109: Check lateral (Y) position, not length (X)
        if is_wide_player {
            // Y is width (0-680), center is ~340
            let central_distance = (player_pos.y - 340).abs();
            if central_distance < 100 {
                // Already central laterally, drift wide
                return ForwardMovementPattern::DriftWide;
            } else {
                // On wing, diagonal run toward center
                return ForwardMovementPattern::DiagonalRun;
            }
        }

        // Check for channel opportunity
        if let Some(_channel) = find_best_channel(defenders, player_pos, ball_pos) {
            return ForwardMovementPattern::ChannelRun;
        }

        // Default to direct run
        ForwardMovementPattern::DirectRun
    }
}

/// Calculate target for direct run behind defense
/// FIX_2601/0109: Fixed coordinate bug - X is length (forward), Y is width (lateral)
fn calculate_direct_run_target(player_pos: Coord10, attack_direction: f32) -> Coord10 {
    // X is length (0-1050), forward direction
    let target_x = if attack_direction > 0.0 { 950 } else { 100 };
    Coord10 {
        x: target_x,
        y: player_pos.y, // Keep same lateral position
        z: 0,
    }
}

/// Calculate target for diagonal run
/// Based on open-football: offset 20 units perpendicular + 15 units forward
/// FIX_2601/0109: Fixed coordinate bug - X is length (forward), Y is width (lateral)
fn calculate_diagonal_run_target(
    player_pos: Coord10,
    ball_pos: Coord10,
    attack_direction: f32,
) -> Coord10 {
    // Move toward center (laterally) and forward
    // Y is width (0-680), center is ~340
    let perpendicular_offset = if player_pos.y < 340 { 100 } else { -100 };
    // X is length (0-1050), forward direction
    let forward_offset = (150.0 * attack_direction) as i32;

    Coord10 {
        x: (ball_pos.x + forward_offset).clamp(100, 950),
        y: (player_pos.y + perpendicular_offset).clamp(100, 580),
        z: 0,
    }
}

/// Calculate target for channel run between defenders
/// FIX_2601/0109: Fixed coordinate bug - X is length (forward), Y is width (lateral)
fn calculate_channel_run_target(
    player_pos: Coord10,
    ball_pos: Coord10,
    defenders: &[Coord10],
    attack_direction: f32,
) -> Coord10 {
    // Channel is a lateral gap (Y direction) between defenders
    if let Some((channel_y, _width)) = find_best_channel(defenders, player_pos, ball_pos) {
        // X is length (0-1050), forward direction
        let forward_offset = (150.0 * attack_direction) as i32;
        Coord10 {
            x: (ball_pos.x + forward_offset).clamp(100, 950),
            y: channel_y, // Move to the channel gap
            z: 0,
        }
    } else {
        // Fallback to direct run
        calculate_direct_run_target(player_pos, attack_direction)
    }
}

/// Find best channel between defenders
/// FIX_2601/0109: Fixed coordinate bug - channels are lateral gaps (Y direction)
/// Returns (channel_center_y, channel_width) if found
fn find_best_channel(
    defenders: &[Coord10],
    _player_pos: Coord10,
    _ball_pos: Coord10,
) -> Option<(i32, i32)> {
    if defenders.len() < 2 {
        return None;
    }

    // Sort defenders by Y position (lateral/width direction)
    let mut sorted: Vec<_> = defenders.iter().map(|d| d.y).collect();
    sorted.sort();

    // Find widest lateral gap
    let mut best_gap: Option<(i32, i32)> = None;
    let mut max_width = 0;

    const MIN_CHANNEL_WIDTH: i32 = 80; // ~8m in Coord10 units (Y max is 680)

    for pair in sorted.windows(2) {
        let gap_center = (pair[0] + pair[1]) / 2;
        let gap_width = (pair[1] - pair[0]).abs();

        if gap_width > max_width && gap_width > MIN_CHANNEL_WIDTH {
            max_width = gap_width;
            best_gap = Some((gap_center, gap_width));
        }
    }

    best_gap
}

/// Calculate target for drifting wide
/// FIX_2601/0109: Fixed coordinate bug - "wide" is lateral (Y direction)
fn calculate_drift_wide_target(player_pos: Coord10, ball_pos: Coord10) -> Coord10 {
    // Drift to opposite side of ball (laterally) to create space
    // Y is width (0-680), center is ~340
    let wide_y = if ball_pos.y < 340 {
        // Ball on left side, drift right
        550
    } else {
        // Ball on right side, drift left
        130
    };

    Coord10 {
        x: player_pos.x, // Maintain forward position
        y: wide_y,
        z: 0,
    }
}

/// Calculate target for checking to feet (coming short)
/// FIX_2601/0109: Fixed coordinate bug - X is length (back offset), Y is width (lateral offset)
fn calculate_check_to_feet_target(
    player_pos: Coord10,
    ball_pos: Coord10,
    attack_direction: f32,
) -> Coord10 {
    // Come back toward ball holder (X direction), offset laterally (Y direction)
    // X is length (0-1050), back offset moves toward own goal
    let back_offset = (-80.0 * attack_direction) as i32;
    // Y is width (0-680), lateral offset to create passing angle
    let lateral_offset = if player_pos.y < ball_pos.y { -60 } else { 60 };

    Coord10 {
        x: (ball_pos.x + back_offset).clamp(100, 950),
        y: (ball_pos.y + lateral_offset).clamp(100, 580),
        z: 0,
    }
}

/// Calculate target for opposite movement
/// FIX_2601/0109: Fixed coordinate bug - opposite movement is lateral (Y direction)
fn calculate_opposite_movement_target(player_pos: Coord10, defenders: &[Coord10]) -> Coord10 {
    if defenders.is_empty() {
        return player_pos;
    }

    // Calculate average defender lateral position
    // Y is width (0-680), center is ~340
    let avg_y: i32 = defenders.iter().map(|d| d.y).sum::<i32>() / defenders.len() as i32;

    // Move opposite to defensive lateral concentration
    let target_y = if avg_y > 340 {
        // Defenders shifted right, move left
        (player_pos.y - 80).max(100)
    } else {
        // Defenders shifted left, move right
        (player_pos.y + 80).min(580)
    };

    Coord10 {
        x: player_pos.x, // Maintain forward position
        y: target_y,
        z: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seek_behavior() {
        let current = Coord10 { x: 500, y: 500, z: 0 };
        let target = Coord10 { x: 600, y: 500, z: 0 };

        let result = seek(current, target, 50, 1.0);
        assert!(result.x > current.x);
        assert_eq!(result.y, current.y);
    }

    #[test]
    fn test_arrive_slowing() {
        let current = Coord10 { x: 500, y: 500, z: 0 };
        let target = Coord10 { x: 520, y: 500, z: 0 };

        // Should slow down when close
        let result = arrive(current, target, 100, 50, 1.0);
        let dist_moved = result.x - current.x;

        // Should move less than max speed when slowing
        assert!(dist_moved < 50);
        assert!(dist_moved > 0);
    }

    #[test]
    fn test_channel_finding() {
        // FIX_2601/0109: Channels are lateral (Y) gaps between defenders
        let defenders = vec![
            Coord10 { x: 700, y: 150, z: 0 },
            Coord10 { x: 700, y: 350, z: 0 },
            Coord10 { x: 700, y: 550, z: 0 },
        ];

        let player = Coord10 { x: 600, y: 250, z: 0 };
        let ball = Coord10 { x: 500, y: 340, z: 0 };

        let channel = find_best_channel(&defenders, player, ball);
        assert!(channel.is_some());

        let (center, width) = channel.unwrap();
        // Should find lateral channel between y=150 and y=350 or y=350 and y=550
        assert!(center == 250 || center == 450);
        assert_eq!(width, 200);
    }

    #[test]
    fn test_diagonal_run() {
        // FIX_2601/0109: X is length (forward), Y is width (lateral)
        // Player on left side (y < 340), ball ahead
        let player = Coord10 { x: 400, y: 150, z: 0 };
        let ball = Coord10 { x: 500, y: 340, z: 0 };

        let target = calculate_diagonal_run_target(player, ball, 1.0);

        // Should move forward (X increases when attacking right)
        assert!(target.x > ball.x, "target.x {} should be > ball.x {}", target.x, ball.x);
        // Should move toward center (Y increases when on left side)
        assert!(target.y > player.y, "target.y {} should be > player.y {}", target.y, player.y);
    }

    #[test]
    fn test_pattern_selection() {
        // FIX_2601/0109: Channels are lateral (Y) gaps
        let player = Coord10 { x: 700, y: 340, z: 0 };
        let ball = Coord10 { x: 500, y: 340, z: 0 };
        let defenders = vec![
            Coord10 { x: 800, y: 150, z: 0 },
            Coord10 { x: 800, y: 500, z: 0 },
        ];

        // Under pressure -> CheckToFeet
        let pattern = ForwardMovementPattern::select_pattern(
            player, ball, &defenders, false, true,
        );
        assert_eq!(pattern, ForwardMovementPattern::CheckToFeet);

        // Not under pressure, channel available (gap between y=150 and y=500)
        let pattern = ForwardMovementPattern::select_pattern(
            player, ball, &defenders, false, false,
        );
        assert_eq!(pattern, ForwardMovementPattern::ChannelRun);
    }

    #[test]
    fn test_drift_wide() {
        // FIX_2601/0109: "Wide" is lateral (Y direction)
        let player = Coord10 { x: 500, y: 340, z: 0 };
        // Ball on left side (y < 340)
        let ball = Coord10 { x: 500, y: 200, z: 0 };

        let target = calculate_drift_wide_target(player, ball);

        // Ball on left, should drift right (toward higher Y)
        assert!(target.y > 340, "target.y {} should be > 340", target.y);
        // X should stay the same
        assert_eq!(target.x, player.x);
    }
}
