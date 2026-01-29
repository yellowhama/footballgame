//! Channel Finder Module
//!
//! FIX_2601/0110: Dynamic channel identification between defenders.
//! Finds gaps in defensive lines for attacking runs.
//!
//! Key features:
//! - Find channels (gaps) between defenders
//! - Score channels by width and position
//! - Select best channel for player's run
//!
//! Coord10 axis convention:
//! - x = length (0-1050, goal direction)
//! - y = width (0-680, sideline direction)

use crate::engine::types::Coord10;

/// Minimum gap width to consider a channel (3m in Coord10 units)
const MIN_CHANNEL_WIDTH: i32 = 30;

/// Represents a gap between defenders that can be exploited
#[derive(Debug, Clone, Copy)]
pub struct Channel {
    /// Center position of the channel
    pub center: Coord10,
    /// Width of the channel (larger = easier to exploit)
    pub width: f32,
    /// Quality score (0.0-1.0) based on width and position
    pub quality: f32,
}

/// Find channels (gaps) between defenders for attacking runs
///
/// # Arguments
/// * `defenders` - Positions of defending team players (excluding GK)
/// * `attack_direction` - 1.0 for home attacking toward x=1050, -1.0 for away
/// * `ball_x` - Current ball X position (length) for filtering relevant defenders
///
/// # Returns
/// List of channels sorted by quality (best first)
pub fn find_channels(defenders: &[Coord10], attack_direction: f32, ball_x: i32) -> Vec<Channel> {
    // Filter defenders ahead of ball (in attack direction)
    // FIX_2601/0110: Use x (length) for goal direction, not y
    let relevant: Vec<Coord10> = defenders
        .iter()
        .filter(|d| {
            if attack_direction > 0.0 {
                d.x > ball_x // Ahead of ball (home attacking toward x=1050)
            } else {
                d.x < ball_x // Ahead of ball (away attacking toward x=0)
            }
        })
        .copied()
        .collect();

    if relevant.len() < 2 {
        return vec![];
    }

    // Sort by Y position (width/lateral) to find gaps
    // FIX_2601/0110: y is width (sideline direction)
    let mut sorted = relevant;
    sorted.sort_by_key(|d| d.y);

    // Find gaps between adjacent defenders
    let mut channels = Vec::new();

    for pair in sorted.windows(2) {
        // Gap width is lateral (y) distance
        let gap_width = (pair[1].y - pair[0].y).abs();

        if gap_width >= MIN_CHANNEL_WIDTH {
            let center_x = (pair[0].x + pair[1].x) / 2; // Depth (length)
            let center_y = (pair[0].y + pair[1].y) / 2; // Lateral center

            // Quality based on width and centrality
            let width_score = (gap_width as f32 / 100.0).min(1.0);
            // Center of field (y=CENTER_Y) is best for channels
            let central_score = (1.0
                - ((center_y - Coord10::CENTER_Y).abs() as f32 / Coord10::CENTER_Y as f32))
                .clamp(0.0, 1.0);
            let quality = width_score * 0.7 + central_score * 0.3;

            channels.push(Channel {
                center: Coord10 { x: center_x, y: center_y, z: 0 },
                width: gap_width as f32,
                quality,
            });
        }
    }

    // Also check edges (between sideline and first/last defender)
    // FIX_2601/0110: Sidelines are at y=0 and y=Coord10::FIELD_WIDTH_10
    if let Some(first) = sorted.first() {
        let left_gap = first.y; // Distance from left sideline (y=0)
        if left_gap > MIN_CHANNEL_WIDTH {
            channels.push(Channel {
                center: Coord10 { x: first.x, y: left_gap / 2, z: 0 },
                width: left_gap as f32,
                quality: (left_gap as f32 / 100.0).min(0.6), // Edge channels slightly lower quality
            });
        }
    }

    if let Some(last) = sorted.last() {
        let right_gap = Coord10::FIELD_WIDTH_10 - last.y; // Distance from right sideline
        if right_gap > MIN_CHANNEL_WIDTH {
            channels.push(Channel {
                center: Coord10 { x: last.x, y: last.y + right_gap / 2, z: 0 },
                width: right_gap as f32,
                quality: (right_gap as f32 / 100.0).min(0.6),
            });
        }
    }

    // Sort by quality (best first)
    channels.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap_or(std::cmp::Ordering::Equal));

    channels
}

/// Find the best channel for a specific player to run into
///
/// Prefers channels close to player's current lane (lateral proximity)
pub fn find_best_channel_for_player(
    player_pos: Coord10,
    defenders: &[Coord10],
    attack_direction: f32,
    ball_x: i32,
) -> Option<Channel> {
    let channels = find_channels(defenders, attack_direction, ball_x);

    if channels.is_empty() {
        return None;
    }

    // Balance channel quality with lateral proximity to player
    // FIX_2601/0110: Use y (width) for lateral proximity, not x
    channels.into_iter().max_by(|a, b| {
        let dist_a = (a.center.y - player_pos.y).abs() as f32;
        let dist_b = (b.center.y - player_pos.y).abs() as f32;

        // Proximity bonus (closer = better, but capped)
        let proximity_a = 1.0 - (dist_a / 200.0).min(0.5);
        let proximity_b = 1.0 - (dist_b / 200.0).min(0.5);

        let score_a = a.quality * 0.6 + proximity_a * 0.4;
        let score_b = b.quality * 0.6 + proximity_b * 0.4;

        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Get opponent defender positions for channel finding
///
/// Filters out goalkeeper and returns outfield defenders
pub fn get_opponent_defenders(is_home_attacking: bool, all_positions: &[Coord10]) -> Vec<Coord10> {
    let opp_start = if is_home_attacking { 11 } else { 0 };
    let opp_end = opp_start + 11;

    // Skip GK (index 0 or 11), take defenders (typically 1-4 or 12-15)
    // Also include defensive midfielders
    all_positions
        .get(opp_start + 1..opp_start + 7.min(opp_end - opp_start))
        .map(|slice| slice.to_vec())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_channels_basic() {
        // Defenders spread laterally (y=width) at same depth (x=length)
        // x=700 means 70m up field, y values are lateral positions
        let defenders = vec![
            Coord10 { x: 700, y: 200, z: 0 },
            Coord10 { x: 700, y: 400, z: 0 },
            Coord10 { x: 700, y: 550, z: 0 }, // Big lateral gap between y=400 and y=550
        ];

        // Ball at x=400 (40m), home attacking toward x=1050
        let channels = find_channels(&defenders, 1.0, 400);
        assert!(!channels.is_empty());

        // Should find the gap between y=400 and y=550 (150 units)
        let big_gap = channels.iter().find(|c| c.width >= 100.0);
        assert!(big_gap.is_some());
    }

    #[test]
    fn test_find_channels_no_gap() {
        // Defenders very close laterally
        let defenders = vec![
            Coord10 { x: 700, y: 330, z: 0 },
            Coord10 { x: 700, y: 340, z: 0 }, // Only 10 units apart
            Coord10 { x: 700, y: 350, z: 0 },
        ];

        let channels = find_channels(&defenders, 1.0, 400);
        // Should find edge channels only (sideline gaps at y~0 and y~680)
        assert!(channels.iter().all(|c| c.center.y < 330 || c.center.y > 350));
    }

    #[test]
    fn test_find_channels_behind_ball() {
        // Defenders behind the ball (x < ball_x)
        let defenders = vec![
            Coord10 { x: 300, y: 200, z: 0 }, // Behind ball (x=300 < ball_x=400)
            Coord10 { x: 350, y: 500, z: 0 },
        ];

        let channels = find_channels(&defenders, 1.0, 400);
        // Defenders behind ball should be filtered out, no channels found
        assert!(channels.is_empty());
    }

    #[test]
    fn test_best_channel_proximity() {
        // Defenders at x=700 with lateral gaps
        let defenders = vec![
            Coord10 { x: 700, y: 150, z: 0 },
            Coord10 { x: 700, y: 340, z: 0 }, // Gap at ~y=245
            Coord10 { x: 700, y: 530, z: 0 }, // Gap at ~y=435
        ];

        // Player on left side (y=180) vs right side (y=500)
        let player_left = Coord10 { x: 500, y: 180, z: 0 };
        let player_right = Coord10 { x: 500, y: 500, z: 0 };

        let best_for_left = find_best_channel_for_player(player_left, &defenders, 1.0, 400);
        let best_for_right = find_best_channel_for_player(player_right, &defenders, 1.0, 400);

        // Left player should prefer left gap (y~245), right player should prefer right gap (y~435)
        if let (Some(left), Some(right)) = (best_for_left, best_for_right) {
            assert!(left.center.y < right.center.y);
        }
    }
}
