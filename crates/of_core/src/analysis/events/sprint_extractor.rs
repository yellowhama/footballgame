//! # Sprint Extractor
//!
//! Detects high-intensity running events from position/velocity data.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: DRIBBLE_MOVEMENT_ANALYSIS.md
//!
//! ## Algorithm
//! 1. For each player, scan velocity data
//! 2. Detect segments where speed >= 7.0 m/s
//! 3. Sprint ends when speed < 5.5 m/s
//! 4. Filter: duration >= 500ms

use crate::models::match_result::{MatchPositionData, PositionDataItem};

/// Sprint threshold: 7.0 m/s (25.2 km/h)
pub const SPRINT_THRESHOLD_MPS: f32 = 7.0;

/// High intensity threshold: 5.5 m/s (19.8 km/h)
pub const HIGH_INTENSITY_MPS: f32 = 5.5;

/// Minimum sprint duration: 500ms
pub const MIN_SPRINT_DURATION_MS: u64 = 500;

/// A detected sprint event.
#[derive(Debug, Clone)]
pub struct SprintEvent {
    /// Player index (0-21)
    pub player_idx: u8,
    /// Team side (0=home, 1=away)
    pub team_side: u8,
    /// Start timestamp in milliseconds
    pub t0_ms: u64,
    /// End timestamp in milliseconds
    pub t1_ms: u64,
    /// Distance covered during sprint in meters
    pub distance_m: f32,
    /// Maximum speed achieved in m/s
    pub max_speed_mps: f32,
    /// Average speed during sprint in m/s
    pub avg_speed_mps: f32,
    /// Starting position (x, y) in meters
    pub start_pos_m: (f32, f32),
    /// Ending position (x, y) in meters
    pub end_pos_m: (f32, f32),
}

impl SprintEvent {
    /// Duration of the sprint in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.t1_ms.saturating_sub(self.t0_ms)
    }
}

/// Movement intensity metrics for a player.
#[derive(Debug, Clone, Default)]
pub struct PlayerMovementMetrics {
    /// Total distance covered in meters
    pub total_distance_m: f32,
    /// Sprint distance (>= 7.0 m/s) in meters
    pub sprint_distance_m: f32,
    /// High intensity distance (>= 5.5 m/s) in meters
    pub high_intensity_distance_m: f32,
    /// Number of sprints
    pub sprint_count: u32,
    /// Sprint distance as ratio of total distance
    pub sprint_ratio: f32,
    /// High intensity ratio
    pub high_intensity_ratio: f32,
}

/// Team-level movement intensity aggregation.
#[derive(Debug, Clone, Default)]
pub struct TeamMovementMetrics {
    /// Total team distance in meters
    pub total_distance_m: f32,
    /// Total sprint distance in meters
    pub sprint_distance_m: f32,
    /// Total high intensity distance in meters
    pub high_intensity_distance_m: f32,
    /// Total sprint count
    pub sprint_count: u32,
    /// Team sprint ratio
    pub sprint_ratio: f32,
    /// Per-player sprint distances [11]
    pub player_sprint_distance: [f32; 11],
    /// Per-player total distances [11]
    pub player_total_distance: [f32; 11],
}

/// Internal struct for building sprint events.
struct SprintBuilder {
    player_idx: u8,
    positions: Vec<(u64, (f32, f32), f32)>, // (timestamp, position, speed)
}

impl SprintBuilder {
    fn new(player_idx: u8, timestamp: u64, pos: (f32, f32), speed: f32) -> Self {
        Self {
            player_idx,
            positions: vec![(timestamp, pos, speed)],
        }
    }

    fn add(&mut self, timestamp: u64, pos: (f32, f32), speed: f32) {
        self.positions.push((timestamp, pos, speed));
    }

    fn duration_ms(&self) -> u64 {
        if self.positions.len() < 2 {
            return 0;
        }
        let first = self.positions.first().unwrap().0;
        let last = self.positions.last().unwrap().0;
        last.saturating_sub(first)
    }

    fn build(self, team_side: u8) -> Option<SprintEvent> {
        if self.positions.len() < 2 {
            return None;
        }

        let duration = self.duration_ms();
        if duration < MIN_SPRINT_DURATION_MS {
            return None;
        }

        let (t0, start_pos, _) = self.positions.first().unwrap();
        let (t1, end_pos, _) = self.positions.last().unwrap();

        // Calculate distance and speeds
        let mut total_dist = 0.0f32;
        let mut max_speed = 0.0f32;
        let mut speed_sum = 0.0f32;

        for i in 1..self.positions.len() {
            let (_, (x0, y0), s0) = self.positions[i - 1];
            let (_, (x1, y1), s1) = self.positions[i];

            let dx = x1 - x0;
            let dy = y1 - y0;
            total_dist += (dx * dx + dy * dy).sqrt();
            max_speed = max_speed.max(s0).max(s1);
            speed_sum += s0;
        }
        speed_sum += self.positions.last().unwrap().2;

        let avg_speed = speed_sum / self.positions.len() as f32;

        Some(SprintEvent {
            player_idx: self.player_idx,
            team_side,
            t0_ms: *t0,
            t1_ms: *t1,
            distance_m: total_dist,
            max_speed_mps: max_speed,
            avg_speed_mps: avg_speed,
            start_pos_m: *start_pos,
            end_pos_m: *end_pos,
        })
    }
}

/// Calculate speed from velocity tuple.
#[inline]
fn speed_from_velocity(vel: Option<(f32, f32)>) -> f32 {
    match vel {
        Some((vx, vy)) => (vx * vx + vy * vy).sqrt(),
        None => 0.0,
    }
}

/// Calculate speed from position delta and time delta.
#[inline]
fn speed_from_positions(pos0: (f32, f32), pos1: (f32, f32), dt_ms: u64) -> f32 {
    if dt_ms == 0 {
        return 0.0;
    }
    let dx = pos1.0 - pos0.0;
    let dy = pos1.1 - pos0.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let dt_s = dt_ms as f32 / 1000.0;
    dist / dt_s
}

/// Extract sprint events from position data.
///
/// # Arguments
/// * `position_data` - Match position data containing player positions
///
/// # Returns
/// Vector of SprintEvent for all players
///
/// # Algorithm
/// - Sprint starts when speed >= SPRINT_THRESHOLD_MPS (7.0 m/s)
/// - Sprint continues while speed >= HIGH_INTENSITY_MPS (5.5 m/s)
/// - Sprint ends when speed drops below HIGH_INTENSITY_MPS
/// - Minimum duration: MIN_SPRINT_DURATION_MS (500ms)
pub fn extract_sprints(position_data: &MatchPositionData) -> Vec<SprintEvent> {
    let mut sprints = Vec::new();

    for player_idx in 0..22u8 {
        let player_data = &position_data.players[player_idx as usize];
        if player_data.is_empty() {
            continue;
        }

        let team_side = if player_idx < 11 { 0 } else { 1 };
        let player_sprints = extract_player_sprints(player_idx, team_side, player_data);
        sprints.extend(player_sprints);
    }

    // Sort by start time
    sprints.sort_by_key(|s| s.t0_ms);
    sprints
}

/// Extract sprints for a single player.
fn extract_player_sprints(
    player_idx: u8,
    team_side: u8,
    data: &[PositionDataItem],
) -> Vec<SprintEvent> {
    let mut sprints = Vec::new();
    let mut current_sprint: Option<SprintBuilder> = None;

    // Calculate speeds for all frames
    let speeds: Vec<f32> = data.iter().enumerate().map(|(i, item)| {
        // Prefer velocity field if available
        if let Some(vel) = item.velocity {
            speed_from_velocity(Some(vel))
        } else if i > 0 {
            // Calculate from position delta
            let prev = &data[i - 1];
            let dt_ms = item.timestamp.saturating_sub(prev.timestamp);
            speed_from_positions(prev.position, item.position, dt_ms)
        } else {
            0.0
        }
    }).collect();

    for (idx, (item, &speed)) in data.iter().zip(speeds.iter()).enumerate() {
        let is_sprinting = speed >= SPRINT_THRESHOLD_MPS;
        let is_high_intensity = speed >= HIGH_INTENSITY_MPS;

        match (&mut current_sprint, is_sprinting, is_high_intensity) {
            // Start new sprint
            (None, true, _) => {
                current_sprint = Some(SprintBuilder::new(player_idx, item.timestamp, item.position, speed));
            }
            // Continue sprint (still at sprint speed or at least high intensity)
            (Some(builder), _, true) => {
                builder.add(item.timestamp, item.position, speed);
            }
            // End sprint (dropped below high intensity)
            (Some(_), false, false) => {
                if let Some(builder) = current_sprint.take() {
                    if let Some(sprint) = builder.build(team_side) {
                        sprints.push(sprint);
                    }
                }
            }
            // No sprint in progress, not sprinting
            (None, false, _) => {}
            // Edge case: speed in between thresholds, no current sprint
            _ => {}
        }
    }

    // Handle sprint that extends to end of data
    if let Some(builder) = current_sprint.take() {
        if let Some(sprint) = builder.build(team_side) {
            sprints.push(sprint);
        }
    }

    sprints
}

/// Calculate player movement metrics from position data.
pub fn calculate_player_metrics(data: &[PositionDataItem]) -> PlayerMovementMetrics {
    if data.len() < 2 {
        return PlayerMovementMetrics::default();
    }

    let mut total_dist = 0.0f32;
    let mut sprint_dist = 0.0f32;
    let mut high_intensity_dist = 0.0f32;
    let mut sprint_count = 0u32;
    let mut in_sprint = false;

    for i in 1..data.len() {
        let prev = &data[i - 1];
        let curr = &data[i];

        let dx = curr.position.0 - prev.position.0;
        let dy = curr.position.1 - prev.position.1;
        let segment_dist = (dx * dx + dy * dy).sqrt();

        // Calculate speed
        let speed = if let Some(vel) = curr.velocity {
            speed_from_velocity(Some(vel))
        } else {
            let dt_ms = curr.timestamp.saturating_sub(prev.timestamp);
            speed_from_positions(prev.position, curr.position, dt_ms)
        };

        total_dist += segment_dist;

        if speed >= SPRINT_THRESHOLD_MPS {
            sprint_dist += segment_dist;
            high_intensity_dist += segment_dist;
            if !in_sprint {
                sprint_count += 1;
                in_sprint = true;
            }
        } else if speed >= HIGH_INTENSITY_MPS {
            high_intensity_dist += segment_dist;
            in_sprint = false;
        } else {
            in_sprint = false;
        }
    }

    PlayerMovementMetrics {
        total_distance_m: total_dist,
        sprint_distance_m: sprint_dist,
        high_intensity_distance_m: high_intensity_dist,
        sprint_count,
        sprint_ratio: if total_dist > 0.0 { sprint_dist / total_dist } else { 0.0 },
        high_intensity_ratio: if total_dist > 0.0 { high_intensity_dist / total_dist } else { 0.0 },
    }
}

/// Calculate team movement metrics from position data.
pub fn calculate_team_movement_metrics(
    position_data: &MatchPositionData,
    team_side: u8,
) -> TeamMovementMetrics {
    let (start_idx, end_idx) = if team_side == 0 { (0, 11) } else { (11, 22) };

    let mut total_dist = 0.0f32;
    let mut sprint_dist = 0.0f32;
    let mut high_intensity_dist = 0.0f32;
    let mut sprint_count = 0u32;
    let mut player_sprint = [0.0f32; 11];
    let mut player_total = [0.0f32; 11];

    for player_idx in start_idx..end_idx {
        let local_idx = if team_side == 0 { player_idx } else { player_idx - 11 };
        let data = &position_data.players[player_idx];

        if data.is_empty() {
            continue;
        }

        let metrics = calculate_player_metrics(data);

        total_dist += metrics.total_distance_m;
        sprint_dist += metrics.sprint_distance_m;
        high_intensity_dist += metrics.high_intensity_distance_m;
        sprint_count += metrics.sprint_count;

        player_sprint[local_idx] = metrics.sprint_distance_m;
        player_total[local_idx] = metrics.total_distance_m;
    }

    TeamMovementMetrics {
        total_distance_m: total_dist,
        sprint_distance_m: sprint_dist,
        high_intensity_distance_m: high_intensity_dist,
        sprint_count,
        sprint_ratio: if total_dist > 0.0 { sprint_dist / total_dist } else { 0.0 },
        player_sprint_distance: player_sprint,
        player_total_distance: player_total,
    }
}

/// Reference values for sprint metrics (EPL 2024-25)
pub mod reference {
    /// Normal sprint ratio range
    pub const SPRINT_RATIO_LOW: f32 = 0.03;
    pub const SPRINT_RATIO_HIGH: f32 = 0.15;

    /// Normal high intensity ratio range
    pub const HIGH_INTENSITY_RATIO_LOW: f32 = 0.10;
    pub const HIGH_INTENSITY_RATIO_HIGH: f32 = 0.30;

    /// Normal sprints per 90 minutes
    pub const SPRINTS_PER_90_LOW: u32 = 100;
    pub const SPRINTS_PER_90_HIGH: u32 = 250;

    /// Normal total distance per player (meters)
    pub const PLAYER_DISTANCE_LOW: f32 = 9000.0;
    pub const PLAYER_DISTANCE_HIGH: f32 = 13000.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::match_result::PlayerState;

    fn make_position_item(timestamp: u64, x: f32, y: f32, vx: f32, vy: f32) -> PositionDataItem {
        PositionDataItem {
            timestamp,
            position: (x, y),
            height: None,
            velocity: Some((vx, vy)),
            state: Some(PlayerState::Attacking),
        }
    }

    #[test]
    fn test_sprint_event_duration() {
        let sprint = SprintEvent {
            player_idx: 7,
            team_side: 0,
            t0_ms: 10000,
            t1_ms: 12500,
            distance_m: 18.0,
            max_speed_mps: 8.5,
            avg_speed_mps: 7.2,
            start_pos_m: (40.0, 30.0),
            end_pos_m: (55.0, 35.0),
        };

        assert_eq!(sprint.duration_ms(), 2500);
    }

    #[test]
    fn test_threshold_constants() {
        assert!(SPRINT_THRESHOLD_MPS > HIGH_INTENSITY_MPS);
        assert!(HIGH_INTENSITY_MPS > 0.0);
    }

    #[test]
    fn test_speed_from_velocity() {
        // 6 m/s in x direction
        assert!((speed_from_velocity(Some((6.0, 0.0))) - 6.0).abs() < 0.01);

        // 3-4-5 triangle
        assert!((speed_from_velocity(Some((3.0, 4.0))) - 5.0).abs() < 0.01);

        // None velocity
        assert_eq!(speed_from_velocity(None), 0.0);
    }

    #[test]
    fn test_sprint_minimum_duration() {
        // Sprint only 400ms - should be rejected
        let data = vec![
            make_position_item(0, 30.0, 34.0, 7.5, 0.0),
            make_position_item(200, 31.5, 34.0, 7.5, 0.0),
            make_position_item(400, 33.0, 34.0, 3.0, 0.0),
        ];

        let sprints = extract_player_sprints(5, 0, &data);
        assert!(sprints.is_empty(), "Sprint < 500ms should be rejected");
    }

    #[test]
    fn test_valid_sprint_detection() {
        // 750ms sprint at ~7.5 m/s
        let data = vec![
            make_position_item(0, 30.0, 34.0, 7.5, 0.0),
            make_position_item(250, 31.875, 34.0, 7.5, 0.0),
            make_position_item(500, 33.75, 34.0, 7.5, 0.0),
            make_position_item(750, 35.625, 34.0, 7.5, 0.0),
            make_position_item(1000, 37.0, 34.0, 3.0, 0.0), // Drop below threshold
        ];

        let sprints = extract_player_sprints(5, 0, &data);
        assert_eq!(sprints.len(), 1, "Should detect 1 sprint");

        let sprint = &sprints[0];
        assert_eq!(sprint.player_idx, 5);
        assert!(sprint.max_speed_mps >= 7.5);
        assert!(sprint.distance_m > 5.0);
    }

    #[test]
    fn test_sprint_continuation_at_high_intensity() {
        // Sprint that drops to high intensity but continues
        let data = vec![
            make_position_item(0, 30.0, 34.0, 8.0, 0.0),     // Sprint
            make_position_item(250, 32.0, 34.0, 6.0, 0.0),   // High intensity (still in sprint)
            make_position_item(500, 33.5, 34.0, 7.5, 0.0),   // Back to sprint
            make_position_item(750, 35.375, 34.0, 6.5, 0.0), // High intensity
            make_position_item(1000, 37.0, 34.0, 4.0, 0.0),  // End sprint
        ];

        let sprints = extract_player_sprints(5, 0, &data);
        assert_eq!(sprints.len(), 1, "Should be 1 continuous sprint");
        assert!(sprints[0].duration_ms() >= 750);
    }

    #[test]
    fn test_multiple_sprints() {
        // Two separate sprints
        let data = vec![
            // First sprint
            make_position_item(0, 30.0, 34.0, 7.5, 0.0),
            make_position_item(250, 31.875, 34.0, 7.5, 0.0),
            make_position_item(500, 33.75, 34.0, 7.5, 0.0),
            make_position_item(750, 35.0, 34.0, 3.0, 0.0),   // End first sprint
            // Walking
            make_position_item(1000, 35.5, 34.0, 2.0, 0.0),
            make_position_item(1250, 36.0, 34.0, 2.0, 0.0),
            // Second sprint
            make_position_item(1500, 36.5, 34.0, 8.0, 0.0),
            make_position_item(1750, 38.5, 34.0, 8.0, 0.0),
            make_position_item(2000, 40.5, 34.0, 8.0, 0.0),
            make_position_item(2250, 42.0, 34.0, 4.0, 0.0),
        ];

        let sprints = extract_player_sprints(5, 0, &data);
        assert_eq!(sprints.len(), 2, "Should detect 2 sprints");
    }

    #[test]
    fn test_player_metrics_calculation() {
        let data = vec![
            make_position_item(0, 0.0, 0.0, 3.0, 0.0),      // Walking
            make_position_item(1000, 3.0, 0.0, 6.0, 0.0),   // High intensity
            make_position_item(2000, 9.0, 0.0, 8.0, 0.0),   // Sprint
            make_position_item(3000, 17.0, 0.0, 7.5, 0.0),  // Sprint
            make_position_item(4000, 24.5, 0.0, 3.0, 0.0),  // Walking
        ];

        let metrics = calculate_player_metrics(&data);
        assert!(metrics.total_distance_m > 20.0);
        assert!(metrics.sprint_distance_m > 10.0);
        assert!(metrics.high_intensity_distance_m >= metrics.sprint_distance_m);
        assert!(metrics.sprint_count >= 1);
    }
}
