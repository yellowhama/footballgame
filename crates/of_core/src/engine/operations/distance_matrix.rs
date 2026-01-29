//! Player Distance Matrix - O(1) distance lookups
//!
//! FIX_2512 Phase 8: Pre-computed distance matrix for efficient distance queries
//!
//! Inspired by Open Football's PlayerDistanceClosure pattern.

use crate::engine::types::Coord10;

/// Pre-computed distance matrix (22x22)
///
/// Stores distances between all player pairs for O(1) lookup.
/// Updated once per tick to avoid redundant calculations.
///
/// # Performance
/// - Update: 231 distance calculations per tick (22*21/2)
/// - Query: O(1) array lookup
///
/// # Example
/// ```ignore
/// let mut matrix = PlayerDistanceMatrix::new();
/// matrix.update(&positions);
/// let dist = matrix.get(5, 12); // O(1) lookup
/// ```
#[derive(Debug, Clone)]
pub struct PlayerDistanceMatrix {
    /// Symmetric distance matrix [i][j] = distance between player i and j
    distances: [[f32; 22]; 22],
    /// Tick when matrix was last updated (for staleness check)
    last_update_tick: u32,
}

impl Default for PlayerDistanceMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerDistanceMatrix {
    /// Create a new distance matrix (all zeros)
    pub fn new() -> Self {
        Self {
            distances: [[0.0; 22]; 22],
            last_update_tick: 0,
        }
    }

    /// Update all distances from current positions
    ///
    /// Should be called once per tick before any distance queries.
    /// Computes 231 unique distances (22*21/2) and mirrors them.
    pub fn update(&mut self, positions: &[Coord10; 22], current_tick: u32) {
        // Skip if already updated this tick
        if self.last_update_tick == current_tick && current_tick > 0 {
            return;
        }

        for i in 0..22 {
            // Diagonal is always 0
            self.distances[i][i] = 0.0;

            for j in (i + 1)..22 {
                let pos_i = positions[i].to_meters();
                let pos_j = positions[j].to_meters();

                let dx = pos_i.0 - pos_j.0;
                let dy = pos_i.1 - pos_j.1;
                let dist = (dx * dx + dy * dy).sqrt();

                // Symmetric: distances[i][j] = distances[j][i]
                self.distances[i][j] = dist;
                self.distances[j][i] = dist;
            }
        }

        self.last_update_tick = current_tick;
    }

    /// Get pre-computed distance between two players (O(1))
    ///
    /// # Panics
    /// Panics if player indices are out of bounds (>= 22)
    #[inline]
    pub fn get(&self, player_a: usize, player_b: usize) -> f32 {
        self.distances[player_a][player_b]
    }

    /// Get distance if indices are valid, None otherwise
    #[inline]
    pub fn get_checked(&self, player_a: usize, player_b: usize) -> Option<f32> {
        if player_a < 22 && player_b < 22 {
            Some(self.distances[player_a][player_b])
        } else {
            None
        }
    }

    /// Get the tick when this matrix was last updated
    #[inline]
    pub fn last_update_tick(&self) -> u32 {
        self.last_update_tick
    }

    /// Check if matrix is stale (not updated this tick)
    #[inline]
    pub fn is_stale(&self, current_tick: u32) -> bool {
        self.last_update_tick != current_tick
    }

    /// Find nearest teammate to a player
    ///
    /// Returns (teammate_idx, distance) or None if no teammates
    pub fn nearest_teammate(&self, player_idx: usize) -> Option<(usize, f32)> {
        let is_home = player_idx < 11;
        let team_range = if is_home { 0..11 } else { 11..22 };

        let mut nearest: Option<(usize, f32)> = None;

        for other_idx in team_range {
            if other_idx == player_idx {
                continue;
            }

            let dist = self.distances[player_idx][other_idx];

            match nearest {
                None => nearest = Some((other_idx, dist)),
                Some((_, best_dist)) if dist < best_dist => {
                    nearest = Some((other_idx, dist));
                }
                _ => {}
            }
        }

        nearest
    }

    /// Find nearest opponent to a player
    ///
    /// Returns (opponent_idx, distance) or None if no opponents
    pub fn nearest_opponent(&self, player_idx: usize) -> Option<(usize, f32)> {
        let is_home = player_idx < 11;
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        let mut nearest: Option<(usize, f32)> = None;

        for other_idx in opponent_range {
            let dist = self.distances[player_idx][other_idx];

            match nearest {
                None => nearest = Some((other_idx, dist)),
                Some((_, best_dist)) if dist < best_dist => {
                    nearest = Some((other_idx, dist));
                }
                _ => {}
            }
        }

        nearest
    }

    /// Count teammates within a radius
    pub fn teammates_within(&self, player_idx: usize, radius: f32) -> usize {
        let is_home = player_idx < 11;
        let team_range = if is_home { 0..11 } else { 11..22 };

        team_range
            .filter(|&i| i != player_idx && self.distances[player_idx][i] <= radius)
            .count()
    }

    /// Count opponents within a radius
    pub fn opponents_within(&self, player_idx: usize, radius: f32) -> usize {
        let is_home = player_idx < 11;
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        opponent_range
            .filter(|&i| self.distances[player_idx][i] <= radius)
            .count()
    }

    /// Get all players within a radius of a given player
    ///
    /// Returns Vec of (player_idx, distance) sorted by distance
    pub fn players_within(&self, player_idx: usize, radius: f32) -> Vec<(usize, f32)> {
        let mut result: Vec<(usize, f32)> = (0..22)
            .filter(|&i| i != player_idx && self.distances[player_idx][i] <= radius)
            .map(|i| (i, self.distances[player_idx][i]))
            .collect();

        result.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// Calculate local pressure (number of opponents within 5m / 3)
    ///
    /// Returns value in range [0.0, ~3.0+]
    pub fn local_pressure(&self, player_idx: usize) -> f32 {
        let opponents = self.opponents_within(player_idx, 5.0);
        opponents as f32 / 3.0
    }

    /// Get average distance to all teammates
    pub fn avg_teammate_distance(&self, player_idx: usize) -> f32 {
        let is_home = player_idx < 11;
        let team_range = if is_home { 0..11 } else { 11..22 };

        let (sum, count) = team_range
            .filter(|&i| i != player_idx)
            .map(|i| self.distances[player_idx][i])
            .fold((0.0f32, 0usize), |(sum, count), dist| (sum + dist, count + 1));

        if count > 0 {
            sum / count as f32
        } else {
            0.0
        }
    }

    /// Get average distance to all opponents
    pub fn avg_opponent_distance(&self, player_idx: usize) -> f32 {
        let is_home = player_idx < 11;
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        let (sum, count) = opponent_range
            .map(|i| self.distances[player_idx][i])
            .fold((0.0f32, 0usize), |(sum, count), dist| (sum + dist, count + 1));

        if count > 0 {
            sum / count as f32
        } else {
            0.0
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
                // Home team spread across left half
                let x = 10.0 + (i as f32 * 4.0); // 10m to 50m
                let y = 20.0 + ((i % 4) as f32 * 10.0); // 20m to 50m
                Coord10::from_meters(x, y)
            } else {
                // Away team spread across right half
                let x = 55.0 + ((i - 11) as f32 * 4.0); // 55m to 95m
                let y = 20.0 + (((i - 11) % 4) as f32 * 10.0);
                Coord10::from_meters(x, y)
            }
        })
    }

    #[test]
    fn test_new_matrix() {
        let matrix = PlayerDistanceMatrix::new();
        assert_eq!(matrix.last_update_tick(), 0);
        assert_eq!(matrix.get(0, 1), 0.0); // Not updated yet
    }

    #[test]
    fn test_update_and_get() {
        let positions = make_test_positions();
        let mut matrix = PlayerDistanceMatrix::new();

        matrix.update(&positions, 1);

        // Check diagonal is 0
        for i in 0..22 {
            assert_eq!(matrix.get(i, i), 0.0);
        }

        // Check symmetry
        for i in 0..22 {
            for j in 0..22 {
                assert_eq!(matrix.get(i, j), matrix.get(j, i));
            }
        }

        // Check actual distance (player 0 at 10,20 and player 1 at 14,30)
        // Distance should be sqrt(16 + 100) = sqrt(116) ≈ 10.77
        let dist_0_1 = matrix.get(0, 1);
        assert!(dist_0_1 > 10.0 && dist_0_1 < 11.0, "Expected ~10.77, got {}", dist_0_1);
    }

    #[test]
    fn test_skip_redundant_update() {
        let positions = make_test_positions();
        let mut matrix = PlayerDistanceMatrix::new();

        matrix.update(&positions, 5);
        assert_eq!(matrix.last_update_tick(), 5);

        // Should skip update for same tick
        let mut modified_positions = positions;
        modified_positions[0] = Coord10::from_meters(0.0, 0.0);

        matrix.update(&modified_positions, 5);
        // Distance should still be from original positions since update was skipped
        let dist = matrix.get(0, 1);
        assert!(dist > 10.0, "Update should have been skipped");
    }

    #[test]
    fn test_get_checked() {
        let matrix = PlayerDistanceMatrix::new();

        assert!(matrix.get_checked(0, 1).is_some());
        assert!(matrix.get_checked(21, 21).is_some());
        assert!(matrix.get_checked(22, 0).is_none());
        assert!(matrix.get_checked(0, 22).is_none());
    }

    #[test]
    fn test_is_stale() {
        let positions = make_test_positions();
        let mut matrix = PlayerDistanceMatrix::new();

        assert!(matrix.is_stale(1));

        matrix.update(&positions, 1);
        assert!(!matrix.is_stale(1));
        assert!(matrix.is_stale(2));
    }

    #[test]
    fn test_nearest_teammate() {
        let mut positions = make_test_positions();
        // Put player 1 very close to player 0
        positions[1] = Coord10::from_meters(11.0, 21.0); // 1.4m from player 0

        let mut matrix = PlayerDistanceMatrix::new();
        matrix.update(&positions, 1);

        let nearest = matrix.nearest_teammate(0);
        assert!(nearest.is_some());
        let (idx, dist) = nearest.unwrap();
        assert_eq!(idx, 1);
        assert!(dist < 2.0, "Expected ~1.4m, got {}", dist);
    }

    #[test]
    fn test_nearest_opponent() {
        let mut positions = make_test_positions();
        // Put player 11 (away) close to player 0 (home)
        positions[11] = Coord10::from_meters(12.0, 22.0); // ~2.8m from player 0

        let mut matrix = PlayerDistanceMatrix::new();
        matrix.update(&positions, 1);

        let nearest = matrix.nearest_opponent(0);
        assert!(nearest.is_some());
        let (idx, dist) = nearest.unwrap();
        assert_eq!(idx, 11);
        assert!(dist < 4.0, "Expected ~2.8m, got {}", dist);
    }

    #[test]
    fn test_teammates_within() {
        let mut positions = make_test_positions();
        // Put players 0, 1, 2, 3 at specific positions, others far away
        positions[0] = Coord10::from_meters(50.0, field::CENTER_Y);
        positions[1] = Coord10::from_meters(52.0, field::CENTER_Y); // 2m away
        positions[2] = Coord10::from_meters(54.0, field::CENTER_Y); // 4m away
        positions[3] = Coord10::from_meters(60.0, field::CENTER_Y); // 10m away
        // Move other home players far away
        for i in 4..11 {
            positions[i] = Coord10::from_meters(0.0, 0.0); // 60m+ away
        }

        let mut matrix = PlayerDistanceMatrix::new();
        matrix.update(&positions, 1);

        assert_eq!(matrix.teammates_within(0, 3.0), 1); // Only player 1
        assert_eq!(matrix.teammates_within(0, 5.0), 2); // Players 1 and 2
        assert_eq!(matrix.teammates_within(0, 15.0), 3); // Players 1, 2, and 3
    }

    #[test]
    fn test_opponents_within() {
        let mut positions = make_test_positions();
        // Put some away players close to home player 0
        positions[0] = Coord10::from_meters(50.0, field::CENTER_Y);
        positions[11] = Coord10::from_meters(52.0, field::CENTER_Y); // 2m
        positions[12] = Coord10::from_meters(55.0, field::CENTER_Y); // 5m
        positions[13] = Coord10::from_meters(60.0, field::CENTER_Y); // 10m

        let mut matrix = PlayerDistanceMatrix::new();
        matrix.update(&positions, 1);

        assert_eq!(matrix.opponents_within(0, 3.0), 1);
        assert_eq!(matrix.opponents_within(0, 6.0), 2);
        assert_eq!(matrix.opponents_within(0, 15.0), 3);
    }

    #[test]
    fn test_local_pressure() {
        let mut positions = make_test_positions();
        // Put 3 opponents within 5m of player 0
        positions[0] = Coord10::from_meters(50.0, field::CENTER_Y);
        positions[11] = Coord10::from_meters(51.0, field::CENTER_Y); // 1m
        positions[12] = Coord10::from_meters(52.0, field::CENTER_Y); // 2m
        positions[13] = Coord10::from_meters(53.0, field::CENTER_Y); // 3m

        let mut matrix = PlayerDistanceMatrix::new();
        matrix.update(&positions, 1);

        let pressure = matrix.local_pressure(0);
        assert!((pressure - 1.0).abs() < 0.01, "Expected 1.0 (3/3), got {}", pressure);
    }

    #[test]
    fn test_players_within() {
        let mut positions = make_test_positions();
        positions[0] = Coord10::from_meters(50.0, field::CENTER_Y);
        positions[1] = Coord10::from_meters(52.0, field::CENTER_Y); // 2m (teammate)
        positions[11] = Coord10::from_meters(53.0, field::CENTER_Y); // 3m (opponent)
        positions[2] = Coord10::from_meters(55.0, field::CENTER_Y); // 5m (teammate)
        // Move other players far away
        for i in 3..11 {
            positions[i] = Coord10::from_meters(0.0, 0.0); // far away
        }
        for i in 12..22 {
            positions[i] = Coord10::from_meters(100.0, 0.0); // far away
        }

        let mut matrix = PlayerDistanceMatrix::new();
        matrix.update(&positions, 1);

        let players = matrix.players_within(0, 6.0);
        assert_eq!(players.len(), 3);
        // Should be sorted by distance
        assert_eq!(players[0].0, 1); // 2m
        assert_eq!(players[1].0, 11); // 3m
        assert_eq!(players[2].0, 2); // 5m
    }

    #[test]
    fn test_avg_distances() {
        let mut positions = make_test_positions();
        // Simplified positions for easy calculation
        positions[0] = Coord10::from_meters(0.0, 0.0);
        positions[1] = Coord10::from_meters(10.0, 0.0); // 10m
        positions[2] = Coord10::from_meters(20.0, 0.0); // 20m
        // Rest of home team far away
        for i in 3..11 {
            positions[i] = Coord10::from_meters(100.0, 0.0);
        }

        let mut matrix = PlayerDistanceMatrix::new();
        matrix.update(&positions, 1);

        // Player 0's avg distance to teammates 1 and 2 should factor into overall avg
        let avg = matrix.avg_teammate_distance(0);
        assert!(avg > 0.0);
    }
}
