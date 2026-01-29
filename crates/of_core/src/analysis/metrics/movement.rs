//! # Movement Entropy Module
//!
//! Calculates position-based occupancy entropy for teams.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: DRIBBLE_MOVEMENT_ANALYSIS.md

/// Occupancy entropy metrics.
#[derive(Debug, Clone, Default)]
pub struct OccupancyEntropy {
    /// Distribution of time spent in each of 20 zones
    pub zone_distribution: [f32; 20],
    /// Shannon entropy of zone distribution (normalized 0-1)
    pub zone_entropy: f32,
    /// Per-player zone entropy values
    pub player_zone_entropy: [f32; 11],
    /// Average entropy across all players
    pub team_avg_entropy: f32,
    /// Variance in player entropies
    pub team_entropy_variance: f32,
}

/// Calculate Shannon entropy from a distribution.
///
/// # Arguments
/// * `counts` - Array of counts
/// * `max_categories` - Maximum possible categories (for normalization)
///
/// # Returns
/// Normalized entropy in [0, 1]
pub fn shannon_entropy_normalized(counts: &[u32], max_categories: usize) -> f32 {
    let total: u32 = counts.iter().sum();
    if total == 0 {
        return 0.0;
    }

    let mut entropy = 0.0f64;
    for &count in counts {
        if count > 0 {
            let p = count as f64 / total as f64;
            entropy -= p * p.log2();
        }
    }

    // Normalize by max entropy (log2(max_categories))
    let max_entropy = (max_categories as f64).log2();
    if max_entropy > 0.0 {
        (entropy / max_entropy) as f32
    } else {
        0.0
    }
}

/// Convert position to 20-zone index.
///
/// Zones: 5 lanes (LW, LHS, C, RHS, RW) x 4 quarters (DEF, MID, FIN, BOX)
pub fn pos_to_zone_index(x_m: f32, y_m: f32) -> usize {
    // Lane (y-axis): 0-13.6=LW, 13.6-27.2=LHS, 27.2-40.8=C, 40.8-54.4=RHS, 54.4-68=RW
    let lane = match y_m {
        y if y < 13.6 => 0,
        y if y < 27.2 => 1,
        y if y < 40.8 => 2,
        y if y < 54.4 => 3,
        _ => 4,
    };

    // Quarter (x-axis): 0-26.25=DEF, 26.25-52.5=MID, 52.5-78.75=FIN, 78.75-105=BOX
    // Use <= for upper bounds to handle boundary values consistently
    let quarter = match x_m {
        x if x <= 26.25 => 0,
        x if x <= 52.5 => 1,
        x if x <= 78.75 => 2,
        _ => 3,
    };

    quarter * 5 + lane
}

/// Calculate occupancy entropy from position samples.
///
/// # Arguments
/// * `position_samples` - Time series of 11-player position snapshots
pub fn calculate_occupancy_entropy(
    position_samples: &[[(f32, f32); 11]],
) -> OccupancyEntropy {
    if position_samples.is_empty() {
        return OccupancyEntropy::default();
    }

    // Count zone occupancy per player
    let mut player_zone_counts = [[0u32; 20]; 11];
    let mut team_zone_counts = [0u32; 20];

    for sample in position_samples {
        for (player_idx, &(x, y)) in sample.iter().enumerate() {
            let zone = pos_to_zone_index(x, y);
            player_zone_counts[player_idx][zone] += 1;
            team_zone_counts[zone] += 1;
        }
    }

    // Calculate per-player entropy
    let mut player_entropies = [0.0f32; 11];
    for (i, counts) in player_zone_counts.iter().enumerate() {
        player_entropies[i] = shannon_entropy_normalized(counts, 20);
    }

    // Team average entropy
    let avg_entropy = player_entropies.iter().sum::<f32>() / 11.0;

    // Variance
    let variance = player_entropies.iter()
        .map(|&e| (e - avg_entropy).powi(2))
        .sum::<f32>() / 11.0;

    // Zone distribution (normalized)
    let total: u32 = team_zone_counts.iter().sum();
    let mut zone_dist = [0.0f32; 20];
    if total > 0 {
        for (i, &count) in team_zone_counts.iter().enumerate() {
            zone_dist[i] = count as f32 / total as f32;
        }
    }

    OccupancyEntropy {
        zone_distribution: zone_dist,
        zone_entropy: shannon_entropy_normalized(&team_zone_counts, 20),
        player_zone_entropy: player_entropies,
        team_avg_entropy: avg_entropy,
        team_entropy_variance: variance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy() {
        // Uniform distribution
        let uniform = [10u32, 10, 10, 10, 10];
        let entropy = shannon_entropy_normalized(&uniform, 5);
        assert!(entropy > 0.95, "Uniform should have high entropy: {}", entropy);

        // Single category
        let single = [100u32, 0, 0, 0, 0];
        let entropy = shannon_entropy_normalized(&single, 5);
        assert!(entropy < 0.05, "Single should have low entropy: {}", entropy);
    }

    #[test]
    fn test_pos_to_zone() {
        // Center of field
        assert_eq!(pos_to_zone_index(52.5, 34.0), 7); // MID quarter, C lane

        // Bottom left (defensive, left wing)
        assert_eq!(pos_to_zone_index(10.0, 5.0), 0); // DEF quarter, LW lane

        // Top right (attacking, right wing)
        assert_eq!(pos_to_zone_index(90.0, 60.0), 19); // BOX quarter, RW lane
    }
}
