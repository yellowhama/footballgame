//! Congestion Calculation Module
//!
//! FIX_2601/0110: Real-time congestion calculation for dynamic space finding.
//! Replaces static SpaceCreationType with actual opponent density analysis.
//!
//! Key features:
//! - Calculate congestion at any position based on nearby opponents
//! - Consider opponent velocities (closing in = higher congestion)
//! - Find low-congestion positions for space creation
//!
//! Coord10 axis convention:
//! - x = length (0-1050, goal direction)
//! - y = width (0-680, sideline direction)

use crate::engine::types::Coord10;

/// Radius for congestion calculation (10m in Coord10 units)
const CONGESTION_RADIUS: f32 = 100.0;

/// Calculate congestion level at a position
///
/// Returns 0.0 (open space) to 1.0 (heavily congested)
///
/// # Arguments
/// * `position` - Position to evaluate
/// * `opponents` - Opponent positions
/// * `opponent_velocities` - Optional: opponent movement vectors for closing speed
pub fn calculate_congestion(
    position: Coord10,
    opponents: &[Coord10],
    opponent_velocities: Option<&[Coord10]>,
) -> f32 {
    let mut congestion = 0.0f32;

    for (i, opp_pos) in opponents.iter().enumerate() {
        let dist = position.distance_to(opp_pos) as f32;

        if dist < CONGESTION_RADIUS && dist > 0.1 {
            // Base congestion: closer = higher (inverse linear)
            let base = 1.0 - (dist / CONGESTION_RADIUS);

            // Velocity factor: opponents closing in increase congestion
            let velocity_factor = if let Some(velocities) = opponent_velocities {
                if let Some(vel) = velocities.get(i) {
                    // Direction from opponent to position
                    let to_pos_x = position.x - opp_pos.x;
                    let to_pos_y = position.y - opp_pos.y;

                    // Dot product: positive = closing in
                    let closing = (vel.x as f32 * to_pos_x as f32 + vel.y as f32 * to_pos_y as f32)
                        / (dist + 1.0);

                    if closing > 0.0 {
                        1.0 + (closing * 0.01).min(0.5) // Up to 50% increase
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            } else {
                1.0
            };

            congestion += base * velocity_factor;
        }
    }

    congestion.min(1.0)
}

/// Find position with lowest congestion in a search area
///
/// # Arguments
/// * `search_center` - Center of search area
/// * `search_radius` - Radius to search within (Coord10 units)
/// * `opponents` - Opponent positions
/// * `num_samples` - Number of sample points per axis (total = num_samples^2)
///
/// # Returns
/// Position with lowest congestion
pub fn find_low_congestion_position(
    search_center: Coord10,
    search_radius: i32,
    opponents: &[Coord10],
    num_samples: usize,
) -> Coord10 {
    if num_samples == 0 {
        return search_center;
    }

    let mut best_pos = search_center;
    let mut lowest_congestion = f32::MAX;

    // Sample positions in a grid
    let step = (search_radius * 2) / (num_samples as i32).max(1);

    for dx_idx in 0..num_samples {
        for dy_idx in 0..num_samples {
            let sample_x = search_center.x - search_radius + (dx_idx as i32 * step);
            let sample_y = search_center.y - search_radius + (dy_idx as i32 * step);

            // Clamp to field bounds: x=0-1050 (length), y=0-680 (width)
            let sample_pos = Coord10 { x: sample_x.clamp(50, 1000), y: sample_y.clamp(50, 630), z: 0 };

            let congestion = calculate_congestion(sample_pos, opponents, None);

            if congestion < lowest_congestion {
                lowest_congestion = congestion;
                best_pos = sample_pos;
            }
        }
    }

    best_pos
}

/// Check if a position has enough space for receiving a pass
///
/// # Arguments
/// * `position` - Position to check
/// * `opponents` - Opponent positions
/// * `_min_space_radius` - Minimum clear radius needed (unused, kept for API compatibility)
///
/// # Returns
/// true if position has sufficient space
pub fn has_receiving_space(
    position: Coord10,
    opponents: &[Coord10],
    _min_space_radius: f32,
) -> bool {
    let congestion = calculate_congestion(position, opponents, None);

    // Low congestion = good receiving space
    // Threshold: 0.3 means at most ~30% of nearby space occupied
    congestion < 0.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_space() {
        let pos = Coord10 { x: 500, y: 500, z: 0 };
        let opponents: Vec<Coord10> = vec![];

        let congestion = calculate_congestion(pos, &opponents, None);
        assert_eq!(congestion, 0.0);
    }

    #[test]
    fn test_single_opponent_close() {
        let pos = Coord10 { x: 500, y: 500, z: 0 };
        let opponents = vec![Coord10 { x: 510, y: 500, z: 0 }]; // 10 units = 1m away

        let congestion = calculate_congestion(pos, &opponents, None);
        // Very close = high congestion
        assert!(congestion > 0.8);
    }

    #[test]
    fn test_single_opponent_far() {
        let pos = Coord10 { x: 500, y: 500, z: 0 };
        let opponents = vec![Coord10 { x: 600, y: 500, z: 0 }]; // 100 units = 10m away (at edge)

        let congestion = calculate_congestion(pos, &opponents, None);
        // At edge of radius = very low congestion
        assert!(congestion < 0.1);
    }

    #[test]
    fn test_multiple_opponents() {
        let pos = Coord10 { x: 500, y: 500, z: 0 };
        let opponents = vec![
            Coord10 { x: 520, y: 500, z: 0 }, // 2m
            Coord10 { x: 480, y: 500, z: 0 }, // 2m
            Coord10 { x: 500, y: 520, z: 0 }, // 2m
        ];

        let congestion = calculate_congestion(pos, &opponents, None);
        // Multiple close opponents = high congestion (but capped at 1.0)
        assert!(congestion > 0.5);
        assert!(congestion <= 1.0);
    }

    #[test]
    fn test_find_low_congestion() {
        let center = Coord10 { x: 500, y: 500, z: 0 };
        let opponents = vec![
            Coord10 { x: 500, y: 500, z: 0 }, // At center
            Coord10 { x: 520, y: 520, z: 0 }, // Northeast
        ];

        let best = find_low_congestion_position(center, 100, &opponents, 5);

        // Best position should be away from opponents
        assert!(best.x < 500 || best.y < 500); // Southwest quadrant
    }

    #[test]
    fn test_receiving_space() {
        let pos = Coord10 { x: 500, y: 500, z: 0 };

        // No opponents = good space
        assert!(has_receiving_space(pos, &[], 50.0));

        // Close opponent = bad space
        let close_opp = vec![Coord10 { x: 510, y: 500, z: 0 }];
        assert!(!has_receiving_space(pos, &close_opp, 50.0));

        // Far opponent = good space
        let far_opp = vec![Coord10 { x: 700, y: 500, z: 0 }];
        assert!(has_receiving_space(pos, &far_opp, 50.0));
    }
}
