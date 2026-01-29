//! # Carry Segment Extractor
//!
//! Extracts continuous ball possession segments into carry events.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: DRIBBLE_MOVEMENT_ANALYSIS.md
//!
//! ## Algorithm
//! 1. For each player, scan position data for PlayerState::WithBall
//! 2. Group consecutive WithBall frames into segments
//! 3. Filter: duration >= 300ms, distance >= 1.5m
//! 4. Calculate delta_x for progressive carry detection

use crate::models::match_result::{MatchPositionData, PlayerState, PositionDataItem};

/// Minimum carry duration in milliseconds.
pub const MIN_CARRY_DURATION_MS: u64 = 300;

/// Minimum carry distance in meters.
pub const MIN_CARRY_DISTANCE_M: f32 = 1.5;

/// Progressive carry threshold in meters (forward advancement).
pub const PROGRESSIVE_CARRY_M: f32 = 10.0;

/// Outcome of a carry segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarryOutcome {
    /// Pass completed
    Pass,
    /// Shot taken
    Shot,
    /// Lost possession (tackle, interception)
    Dispossessed,
    /// Ball went out of play
    OutOfBounds,
    /// Foul committed or received
    Foul,
    /// Unknown/other
    Other,
}

/// A continuous ball possession segment.
#[derive(Debug, Clone)]
pub struct CarrySegment {
    /// Player index (0-21)
    pub player_idx: u8,
    /// Team side (0=home, 1=away)
    pub team_side: u8,
    /// Start timestamp in milliseconds
    pub t0_ms: u64,
    /// End timestamp in milliseconds
    pub t1_ms: u64,
    /// Starting position (x, y) in meters
    pub start_pos_m: (f32, f32),
    /// Ending position (x, y) in meters
    pub end_pos_m: (f32, f32),
    /// Total distance covered in meters
    pub distance_m: f32,
    /// Progress toward attacking goal (positive = forward)
    pub delta_x_attack_m: f32,
    /// Maximum speed during carry in m/s
    pub max_speed_mps: f32,
    /// How the carry ended
    pub outcome: CarryOutcome,
}

impl CarrySegment {
    /// Duration of the carry in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.t1_ms.saturating_sub(self.t0_ms)
    }

    /// Whether this is a progressive carry (advances >= 10m toward goal).
    pub fn is_progressive(&self) -> bool {
        self.delta_x_attack_m >= PROGRESSIVE_CARRY_M
    }

    /// Average speed during carry in m/s.
    pub fn avg_speed_mps(&self) -> f32 {
        let duration_s = self.duration_ms() as f32 / 1000.0;
        if duration_s > 0.0 {
            self.distance_m / duration_s
        } else {
            0.0
        }
    }
}

/// Aggregated carry statistics for a team.
#[derive(Debug, Clone, Default)]
pub struct TeamCarryStats {
    /// Total number of carries
    pub total_carries: u32,
    /// Total carry distance in meters
    pub total_distance_m: f32,
    /// Progressive carries (>= 10m advance)
    pub progressive_carries: u32,
    /// Progressive carry distance in meters
    pub progressive_distance_m: f32,
    /// Average carry duration in milliseconds
    pub avg_duration_ms: f32,
    /// Average carry distance in meters
    pub avg_distance_m: f32,
    /// Carries resulting in shot
    pub carries_to_shot: u32,
    /// Carries ending in dispossession
    pub dispossessions: u32,
    /// Per-player carry counts [0-10]
    pub player_carry_counts: [u32; 11],
    /// Per-player carry distance [0-10]
    pub player_carry_distance: [f32; 11],
}

/// Internal struct for tracking carry segments during extraction.
struct CarryBuilder {
    player_idx: u8,
    start_idx: usize,
    positions: Vec<(u64, (f32, f32), Option<(f32, f32)>)>, // (timestamp, position, velocity)
}

impl CarryBuilder {
    fn new(player_idx: u8, start_idx: usize, item: &PositionDataItem) -> Self {
        Self {
            player_idx,
            start_idx,
            positions: vec![(item.timestamp, item.position, item.velocity)],
        }
    }

    fn add(&mut self, item: &PositionDataItem) {
        self.positions.push((item.timestamp, item.position, item.velocity));
    }

    fn duration_ms(&self) -> u64 {
        if self.positions.len() < 2 {
            return 0;
        }
        let first = self.positions.first().unwrap().0;
        let last = self.positions.last().unwrap().0;
        last.saturating_sub(first)
    }

    fn calculate_distance(&self) -> f32 {
        if self.positions.len() < 2 {
            return 0.0;
        }

        let mut total = 0.0f32;
        for i in 1..self.positions.len() {
            let (_, (x0, y0), _) = self.positions[i - 1];
            let (_, (x1, y1), _) = self.positions[i];
            let dx = x1 - x0;
            let dy = y1 - y0;
            total += (dx * dx + dy * dy).sqrt();
        }
        total
    }

    fn max_speed(&self) -> f32 {
        self.positions
            .iter()
            .filter_map(|(_, _, vel)| {
                vel.map(|(vx, vy)| (vx * vx + vy * vy).sqrt())
            })
            .fold(0.0f32, f32::max)
    }

    fn build(self, team_side: u8, outcome: CarryOutcome) -> Option<CarrySegment> {
        if self.positions.len() < 2 {
            return None;
        }

        let duration = self.duration_ms();
        if duration < MIN_CARRY_DURATION_MS {
            return None;
        }

        let distance = self.calculate_distance();
        if distance < MIN_CARRY_DISTANCE_M {
            return None;
        }

        let (t0, start_pos, _) = self.positions.first().unwrap();
        let (t1, end_pos, _) = self.positions.last().unwrap();

        // Calculate delta_x (forward progress)
        // Home team attacks toward x=105, Away team attacks toward x=0
        let delta_x = if team_side == 0 {
            end_pos.0 - start_pos.0
        } else {
            start_pos.0 - end_pos.0
        };

        Some(CarrySegment {
            player_idx: self.player_idx,
            team_side,
            t0_ms: *t0,
            t1_ms: *t1,
            start_pos_m: *start_pos,
            end_pos_m: *end_pos,
            distance_m: distance,
            delta_x_attack_m: delta_x,
            max_speed_mps: self.max_speed(),
            outcome,
        })
    }
}

/// Extract carry segments from position data.
///
/// # Arguments
/// * `position_data` - Match position data containing player positions
///
/// # Returns
/// Vector of CarrySegment for both teams
///
/// # Algorithm
/// 1. For each player (0-21), scan position data
/// 2. Find consecutive frames where state == Some(PlayerState::WithBall)
/// 3. Group into segments, filter by duration >= 300ms, distance >= 1.5m
/// 4. Infer outcome from position/state changes
pub fn extract_carries(position_data: &MatchPositionData) -> Vec<CarrySegment> {
    let mut carries = Vec::new();

    for player_idx in 0..22u8 {
        let player_data = &position_data.players[player_idx as usize];
        if player_data.is_empty() {
            continue;
        }

        // Team side: players 0-10 = home (0), players 11-21 = away (1)
        let team_side = if player_idx < 11 { 0 } else { 1 };

        let player_carries = extract_player_carries(player_idx, team_side, player_data);
        carries.extend(player_carries);
    }

    // Sort by start time
    carries.sort_by_key(|c| c.t0_ms);
    carries
}

/// Extract carries for a single player.
fn extract_player_carries(
    player_idx: u8,
    team_side: u8,
    data: &[PositionDataItem],
) -> Vec<CarrySegment> {
    let mut carries = Vec::new();
    let mut current_carry: Option<CarryBuilder> = None;

    for (idx, item) in data.iter().enumerate() {
        let is_with_ball = item.state == Some(PlayerState::WithBall);

        match (&mut current_carry, is_with_ball) {
            // Start new carry
            (None, true) => {
                current_carry = Some(CarryBuilder::new(player_idx, idx, item));
            }
            // Continue carry
            (Some(builder), true) => {
                builder.add(item);
            }
            // End carry
            (Some(_), false) => {
                if let Some(builder) = current_carry.take() {
                    // Infer outcome from current state
                    let outcome = infer_outcome(item);
                    if let Some(carry) = builder.build(team_side, outcome) {
                        carries.push(carry);
                    }
                }
            }
            // No carry in progress
            (None, false) => {}
        }
    }

    // Handle carry that extends to end of data
    if let Some(builder) = current_carry.take() {
        if let Some(carry) = builder.build(team_side, CarryOutcome::Other) {
            carries.push(carry);
        }
    }

    carries
}

/// Infer carry outcome from the position item following the carry.
fn infer_outcome(next_item: &PositionDataItem) -> CarryOutcome {
    // Based on the player state after losing possession
    match next_item.state {
        Some(PlayerState::Attacking) => CarryOutcome::Pass,
        Some(PlayerState::Defending) => CarryOutcome::Dispossessed,
        _ => CarryOutcome::Other,
    }
}

/// Calculate team carry statistics from carry segments.
pub fn calculate_team_carry_stats(carries: &[CarrySegment], team_side: u8) -> TeamCarryStats {
    let team_carries: Vec<_> = carries.iter()
        .filter(|c| c.team_side == team_side)
        .collect();

    if team_carries.is_empty() {
        return TeamCarryStats::default();
    }

    let total = team_carries.len() as u32;
    let total_dist: f32 = team_carries.iter().map(|c| c.distance_m).sum();
    let total_duration: u64 = team_carries.iter().map(|c| c.duration_ms()).sum();

    let progressive: Vec<_> = team_carries.iter()
        .filter(|c| c.is_progressive())
        .collect();
    let prog_dist: f32 = progressive.iter().map(|c| c.distance_m).sum();

    let shots = team_carries.iter()
        .filter(|c| c.outcome == CarryOutcome::Shot)
        .count() as u32;
    let dispossessed = team_carries.iter()
        .filter(|c| c.outcome == CarryOutcome::Dispossessed)
        .count() as u32;

    // Per-player stats (index within team: 0-10)
    let mut player_counts = [0u32; 11];
    let mut player_distance = [0.0f32; 11];

    for carry in &team_carries {
        // Convert player_idx to team-local index
        let local_idx = if team_side == 0 {
            carry.player_idx as usize
        } else {
            (carry.player_idx as usize).saturating_sub(11)
        };

        if local_idx < 11 {
            player_counts[local_idx] += 1;
            player_distance[local_idx] += carry.distance_m;
        }
    }

    TeamCarryStats {
        total_carries: total,
        total_distance_m: total_dist,
        progressive_carries: progressive.len() as u32,
        progressive_distance_m: prog_dist,
        avg_duration_ms: total_duration as f32 / total as f32,
        avg_distance_m: total_dist / total as f32,
        carries_to_shot: shots,
        dispossessions: dispossessed,
        player_carry_counts: player_counts,
        player_carry_distance: player_distance,
    }
}

/// Calculate progressive carry rate (progressive distance / total distance).
pub fn calculate_progressive_rate(stats: &TeamCarryStats) -> f32 {
    if stats.total_distance_m > 0.0 {
        stats.progressive_distance_m / stats.total_distance_m
    } else {
        0.0
    }
}

/// Calculate carry share (carry distance / total ball possession distance).
/// This is a simplification - in reality would need pass distance too.
pub fn calculate_carry_share(carries: &[CarrySegment], team_side: u8) -> f32 {
    let team_carries: Vec<_> = carries.iter()
        .filter(|c| c.team_side == team_side)
        .collect();

    if team_carries.is_empty() {
        return 0.0;
    }

    // Rough estimate: assume average pass is 15m
    let total_carry_dist: f32 = team_carries.iter().map(|c| c.distance_m).sum();
    let num_carries = team_carries.len() as f32;
    let estimated_pass_dist = num_carries * 15.0;

    total_carry_dist / (total_carry_dist + estimated_pass_dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position_item(timestamp: u64, x: f32, y: f32, state: Option<PlayerState>) -> PositionDataItem {
        PositionDataItem {
            timestamp,
            position: (x, y),
            height: None,
            velocity: Some((1.0, 0.0)),
            state,
        }
    }

    #[test]
    fn test_carry_segment_duration() {
        let carry = CarrySegment {
            player_idx: 5,
            team_side: 0,
            t0_ms: 1000,
            t1_ms: 3500,
            start_pos_m: (30.0, 34.0),
            end_pos_m: (45.0, 34.0),
            distance_m: 15.0,
            delta_x_attack_m: 15.0,
            max_speed_mps: 6.5,
            outcome: CarryOutcome::Pass,
        };

        assert_eq!(carry.duration_ms(), 2500);
        assert!(carry.is_progressive());
    }

    #[test]
    fn test_carry_minimum_duration() {
        // 200ms carry should be rejected
        let data = vec![
            make_position_item(0, 30.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(200, 32.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(400, 34.0, 34.0, Some(PlayerState::Attacking)),
        ];

        let carries = extract_player_carries(5, 0, &data);
        assert!(carries.is_empty(), "Carry < 300ms should be rejected");
    }

    #[test]
    fn test_carry_minimum_distance() {
        // 500ms carry but only 1m distance should be rejected
        let data = vec![
            make_position_item(0, 30.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(250, 30.3, 34.0, Some(PlayerState::WithBall)),
            make_position_item(500, 30.6, 34.0, Some(PlayerState::WithBall)),
            make_position_item(750, 31.0, 34.0, Some(PlayerState::Attacking)),
        ];

        let carries = extract_player_carries(5, 0, &data);
        assert!(carries.is_empty(), "Carry < 1.5m should be rejected");
    }

    #[test]
    fn test_valid_carry_extraction() {
        // 750ms carry covering ~4.5m (30 → 34.5)
        let data = vec![
            make_position_item(0, 30.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(250, 31.5, 34.0, Some(PlayerState::WithBall)),
            make_position_item(500, 33.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(750, 34.5, 34.0, Some(PlayerState::WithBall)),
            make_position_item(1000, 36.0, 34.0, Some(PlayerState::Attacking)),
        ];

        let carries = extract_player_carries(5, 0, &data);
        assert_eq!(carries.len(), 1);

        let carry = &carries[0];
        assert_eq!(carry.player_idx, 5);
        assert_eq!(carry.team_side, 0);
        assert_eq!(carry.t0_ms, 0);
        assert_eq!(carry.t1_ms, 750);
        // Distance: 1.5 + 1.5 + 1.5 = 4.5m
        assert!(carry.distance_m >= 4.0 && carry.distance_m <= 5.0,
            "Expected ~4.5m, got {}", carry.distance_m);
        assert_eq!(carry.outcome, CarryOutcome::Pass);
    }

    #[test]
    fn test_progressive_carry() {
        // 2 second carry covering 12m forward
        let data = vec![
            make_position_item(0, 30.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(500, 34.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(1000, 38.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(1500, 42.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(2000, 42.5, 34.0, Some(PlayerState::Attacking)),
        ];

        let carries = extract_player_carries(5, 0, &data);
        assert_eq!(carries.len(), 1);
        assert!(carries[0].is_progressive());
        assert!(carries[0].delta_x_attack_m >= 10.0);
    }

    #[test]
    fn test_team_carry_stats() {
        let carries = vec![
            CarrySegment {
                player_idx: 5,
                team_side: 0,
                t0_ms: 1000,
                t1_ms: 2000,
                start_pos_m: (30.0, 34.0),
                end_pos_m: (42.0, 34.0),
                distance_m: 12.0,
                delta_x_attack_m: 12.0,
                max_speed_mps: 6.0,
                outcome: CarryOutcome::Pass,
            },
            CarrySegment {
                player_idx: 7,
                team_side: 0,
                t0_ms: 5000,
                t1_ms: 5500,
                start_pos_m: (50.0, 20.0),
                end_pos_m: (52.0, 22.0),
                distance_m: 3.0,
                delta_x_attack_m: 2.0,
                max_speed_mps: 4.5,
                outcome: CarryOutcome::Dispossessed,
            },
        ];

        let stats = calculate_team_carry_stats(&carries, 0);
        assert_eq!(stats.total_carries, 2);
        assert_eq!(stats.progressive_carries, 1);
        assert_eq!(stats.dispossessions, 1);
        assert_eq!(stats.player_carry_counts[5], 1);
        assert_eq!(stats.player_carry_counts[7], 1);
    }

    #[test]
    fn test_multiple_carries_single_player() {
        // Two separate carries
        let data = vec![
            // First carry: 500ms, 4m (meets thresholds)
            make_position_item(0, 30.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(250, 32.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(500, 34.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(750, 34.5, 34.0, Some(PlayerState::Attacking)),
            // Gap without ball
            make_position_item(1000, 40.0, 34.0, Some(PlayerState::Attacking)),
            make_position_item(1250, 42.0, 34.0, Some(PlayerState::Attacking)),
            // Second carry: 750ms, 6m
            make_position_item(2000, 45.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(2250, 47.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(2500, 49.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(2750, 51.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(3000, 52.0, 34.0, Some(PlayerState::Defending)),
        ];

        let carries = extract_player_carries(5, 0, &data);
        assert_eq!(carries.len(), 2, "Expected 2 carries, got {}", carries.len());

        // First carry: 500ms, ~4m
        assert!(carries[0].distance_m >= 3.5, "First carry distance {}", carries[0].distance_m);
        // Second carry: 750ms, ~6m
        assert!(carries[1].distance_m >= 5.5, "Second carry distance {}", carries[1].distance_m);
    }

    #[test]
    fn test_away_team_delta_x() {
        // Away team attacks toward x=0
        let data = vec![
            make_position_item(0, 70.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(250, 67.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(500, 64.0, 34.0, Some(PlayerState::WithBall)),
            make_position_item(750, 61.0, 34.0, Some(PlayerState::Attacking)),
        ];

        // team_side=1 (away)
        let carries = extract_player_carries(15, 1, &data);
        assert_eq!(carries.len(), 1);
        // Moving from x=70 to x=61 is FORWARD for away team
        assert!(carries[0].delta_x_attack_m > 0.0);
    }
}
