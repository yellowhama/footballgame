//! # Off-Ball Run Extractor
//!
//! Detects off-ball runs and attacking movements.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: DRIBBLE_MOVEMENT_ANALYSIS.md
//!
//! ## Algorithm
//! 1. For each player, scan position data for non-ball carrier states
//! 2. Track movement segments where speed >= 4.5 m/s
//! 3. Filter: duration >= 1000ms, distance >= 6m
//! 4. Classify run type based on direction

use crate::models::match_result::{MatchPositionData, PlayerState, PositionDataItem};

/// Minimum speed for run detection: 4.5 m/s (16.2 km/h)
pub const RUN_MIN_SPEED_MPS: f32 = 4.5;

/// Minimum run duration: 1000ms
pub const MIN_RUN_DURATION_MS: u64 = 1000;

/// Minimum run distance: 6m
pub const MIN_RUN_DISTANCE_M: f32 = 6.0;

/// Type of attacking run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    /// Straight run toward goal
    Forward,
    /// Diagonal run across the pitch
    Diagonal,
    /// Wide run overlapping teammate
    Overlap,
    /// Run to offer passing option (support)
    Support,
    /// Defensive recovery run
    Defensive,
    /// Run into space behind defense
    InBehind,
    /// Run to receive ball to feet
    ToFeet,
}

/// A detected off-ball run event.
#[derive(Debug, Clone)]
pub struct RunEvent {
    /// Player index (0-21)
    pub player_idx: u8,
    /// Team side (0=home, 1=away)
    pub team_side: u8,
    /// Start timestamp in milliseconds
    pub t0_ms: u64,
    /// End timestamp in milliseconds
    pub t1_ms: u64,
    /// Distance covered during run in meters
    pub distance_m: f32,
    /// Progress toward attacking goal (positive = forward)
    pub delta_x_attack_m: f32,
    /// Lateral movement (absolute) in meters
    pub lateral_movement_m: f32,
    /// Whether the player was off-ball during this run
    pub is_off_ball: bool,
    /// Classified run type
    pub run_type: RunType,
    /// Starting position (x, y) in meters
    pub start_pos_m: (f32, f32),
    /// Ending position (x, y) in meters
    pub end_pos_m: (f32, f32),
    /// Maximum speed during run in m/s
    pub max_speed_mps: f32,
}

impl RunEvent {
    /// Duration of the run in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.t1_ms.saturating_sub(self.t0_ms)
    }

    /// Whether this is an attacking run (positive progress toward goal).
    pub fn is_attacking(&self) -> bool {
        self.delta_x_attack_m > 0.0
    }
}

/// Aggregated run statistics for a team.
#[derive(Debug, Clone, Default)]
pub struct TeamRunStats {
    /// Total number of runs
    pub total_runs: u32,
    /// Off-ball runs only
    pub off_ball_runs: u32,
    /// Forward runs count
    pub forward_runs: u32,
    /// Diagonal runs count
    pub diagonal_runs: u32,
    /// Overlap runs count
    pub overlap_runs: u32,
    /// Support runs count
    pub support_runs: u32,
    /// Runs into space behind defense
    pub runs_in_behind: u32,
    /// Defensive runs count
    pub defensive_runs: u32,
    /// Average run distance in meters
    pub avg_run_distance_m: f32,
    /// Runs per 90 minutes (normalized)
    pub runs_per_90: f32,
}

/// Internal struct for building run events.
struct RunBuilder {
    player_idx: u8,
    is_off_ball: bool,
    positions: Vec<(u64, (f32, f32), f32)>, // (timestamp, position, speed)
}

impl RunBuilder {
    fn new(player_idx: u8, is_off_ball: bool, timestamp: u64, pos: (f32, f32), speed: f32) -> Self {
        Self {
            player_idx,
            is_off_ball,
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
        self.positions.iter().map(|(_, _, s)| *s).fold(0.0f32, f32::max)
    }

    fn build(self, team_side: u8) -> Option<RunEvent> {
        if self.positions.len() < 2 {
            return None;
        }

        let duration = self.duration_ms();
        if duration < MIN_RUN_DURATION_MS {
            return None;
        }

        let distance = self.calculate_distance();
        if distance < MIN_RUN_DISTANCE_M {
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

        let lateral = (end_pos.1 - start_pos.1).abs();

        let run_type = classify_run(delta_x, lateral, *start_pos, *end_pos);

        Some(RunEvent {
            player_idx: self.player_idx,
            team_side,
            t0_ms: *t0,
            t1_ms: *t1,
            distance_m: distance,
            delta_x_attack_m: delta_x,
            lateral_movement_m: lateral,
            is_off_ball: self.is_off_ball,
            run_type,
            start_pos_m: *start_pos,
            end_pos_m: *end_pos,
            max_speed_mps: self.max_speed(),
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

/// Extract run events from position data.
///
/// # Arguments
/// * `position_data` - Match position data containing player positions
///
/// # Returns
/// Vector of RunEvent for all players
///
/// # Algorithm
/// - Run starts when speed >= RUN_MIN_SPEED_MPS (4.5 m/s) and player state != WithBall
/// - Run ends when speed drops or player receives ball
/// - Minimum duration: MIN_RUN_DURATION_MS (1000ms)
/// - Minimum distance: MIN_RUN_DISTANCE_M (6m)
pub fn extract_runs(position_data: &MatchPositionData) -> Vec<RunEvent> {
    let mut runs = Vec::new();

    for player_idx in 0..22u8 {
        let player_data = &position_data.players[player_idx as usize];
        if player_data.is_empty() {
            continue;
        }

        let team_side = if player_idx < 11 { 0 } else { 1 };
        let player_runs = extract_player_runs(player_idx, team_side, player_data);
        runs.extend(player_runs);
    }

    // Sort by start time
    runs.sort_by_key(|r| r.t0_ms);
    runs
}

/// Extract runs for a single player.
fn extract_player_runs(
    player_idx: u8,
    team_side: u8,
    data: &[PositionDataItem],
) -> Vec<RunEvent> {
    let mut runs = Vec::new();
    let mut current_run: Option<RunBuilder> = None;

    // Calculate speeds for all frames
    let speeds: Vec<f32> = data.iter().enumerate().map(|(i, item)| {
        if let Some(vel) = item.velocity {
            speed_from_velocity(Some(vel))
        } else if i > 0 {
            let prev = &data[i - 1];
            let dt_ms = item.timestamp.saturating_sub(prev.timestamp);
            speed_from_positions(prev.position, item.position, dt_ms)
        } else {
            0.0
        }
    }).collect();

    for (item, &speed) in data.iter().zip(speeds.iter()) {
        let is_with_ball = item.state == Some(PlayerState::WithBall);
        let is_running = speed >= RUN_MIN_SPEED_MPS;
        let is_off_ball = !is_with_ball;

        match (&mut current_run, is_running, is_with_ball) {
            // Start new run (if running and not with ball)
            (None, true, false) => {
                current_run = Some(RunBuilder::new(player_idx, true, item.timestamp, item.position, speed));
            }
            // Continue run
            (Some(builder), true, false) => {
                builder.add(item.timestamp, item.position, speed);
            }
            // End run (speed dropped or got ball)
            (Some(_), false, _) | (Some(_), _, true) => {
                if let Some(builder) = current_run.take() {
                    if let Some(run) = builder.build(team_side) {
                        runs.push(run);
                    }
                }
            }
            // No run in progress
            (None, false, _) | (None, _, true) => {}
        }
    }

    // Handle run that extends to end of data
    if let Some(builder) = current_run.take() {
        if let Some(run) = builder.build(team_side) {
            runs.push(run);
        }
    }

    runs
}

/// Classify a run based on its characteristics.
pub fn classify_run(
    delta_x_attack_m: f32,
    lateral_movement_m: f32,
    start_pos: (f32, f32),
    end_pos: (f32, f32),
) -> RunType {
    // Defensive run if moving backward
    if delta_x_attack_m < -2.0 {
        return RunType::Defensive;
    }

    // Calculate ratios
    let forward_ratio = if lateral_movement_m > 1.0 {
        delta_x_attack_m / lateral_movement_m
    } else {
        if delta_x_attack_m > 0.0 { 10.0 } else { 0.0 }
    };

    // Run into space behind (final third, strong forward movement)
    if end_pos.0 > 78.0 && forward_ratio > 1.5 {
        return RunType::InBehind;
    }

    // Mostly forward run
    if forward_ratio > 2.0 {
        return RunType::Forward;
    }

    // Mostly lateral (overlap or support)
    if forward_ratio < 0.5 {
        if lateral_movement_m > 15.0 {
            return RunType::Overlap;
        } else {
            return RunType::Support;
        }
    }

    // Diagonal run (balanced forward and lateral)
    RunType::Diagonal
}

/// Calculate team run statistics from run events.
pub fn calculate_team_run_stats(runs: &[RunEvent], team_side: u8, match_duration_ms: u64) -> TeamRunStats {
    let team_runs: Vec<_> = runs.iter()
        .filter(|r| r.team_side == team_side)
        .collect();

    if team_runs.is_empty() {
        return TeamRunStats::default();
    }

    let total = team_runs.len() as u32;
    let off_ball = team_runs.iter().filter(|r| r.is_off_ball).count() as u32;

    let forward = team_runs.iter().filter(|r| r.run_type == RunType::Forward).count() as u32;
    let diagonal = team_runs.iter().filter(|r| r.run_type == RunType::Diagonal).count() as u32;
    let overlap = team_runs.iter().filter(|r| r.run_type == RunType::Overlap).count() as u32;
    let support = team_runs.iter().filter(|r| r.run_type == RunType::Support).count() as u32;
    let in_behind = team_runs.iter().filter(|r| r.run_type == RunType::InBehind).count() as u32;
    let defensive = team_runs.iter().filter(|r| r.run_type == RunType::Defensive).count() as u32;

    let total_distance: f32 = team_runs.iter().map(|r| r.distance_m).sum();
    let avg_distance = total_distance / total as f32;

    // Normalize to per 90 minutes
    let minutes_played = match_duration_ms as f32 / 60000.0;
    let runs_per_90 = if minutes_played > 0.0 {
        total as f32 * 90.0 / minutes_played
    } else {
        0.0
    };

    TeamRunStats {
        total_runs: total,
        off_ball_runs: off_ball,
        forward_runs: forward,
        diagonal_runs: diagonal,
        overlap_runs: overlap,
        support_runs: support,
        runs_in_behind: in_behind,
        defensive_runs: defensive,
        avg_run_distance_m: avg_distance,
        runs_per_90,
    }
}

/// Reference values for run metrics (EPL 2024-25)
pub mod reference {
    /// Normal off-ball runs per 90
    pub const OFF_BALL_RUNS_PER_90_LOW: f32 = 30.0;
    pub const OFF_BALL_RUNS_PER_90_HIGH: f32 = 120.0;

    /// Normal runs in behind per 90
    pub const RUNS_IN_BEHIND_PER_90_LOW: f32 = 5.0;
    pub const RUNS_IN_BEHIND_PER_90_HIGH: f32 = 25.0;

    /// Normal forward runs per 90
    pub const FORWARD_RUNS_PER_90_LOW: f32 = 10.0;
    pub const FORWARD_RUNS_PER_90_HIGH: f32 = 50.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position_item(timestamp: u64, x: f32, y: f32, vx: f32, vy: f32, state: Option<PlayerState>) -> PositionDataItem {
        PositionDataItem {
            timestamp,
            position: (x, y),
            height: None,
            velocity: Some((vx, vy)),
            state,
        }
    }

    #[test]
    fn test_run_event_duration() {
        let run = RunEvent {
            player_idx: 9,
            team_side: 0,
            t0_ms: 30000,
            t1_ms: 33000,
            distance_m: 15.0,
            delta_x_attack_m: 12.0,
            lateral_movement_m: 9.0,
            is_off_ball: true,
            run_type: RunType::Diagonal,
            start_pos_m: (45.0, 30.0),
            end_pos_m: (57.0, 39.0),
            max_speed_mps: 6.5,
        };

        assert_eq!(run.duration_ms(), 3000);
        assert!(run.is_attacking());
    }

    #[test]
    fn test_classify_run_forward() {
        // Strong forward run
        let rt = classify_run(20.0, 5.0, (40.0, 30.0), (60.0, 35.0));
        assert_eq!(rt, RunType::Forward);
    }

    #[test]
    fn test_classify_run_in_behind() {
        // Run in behind (final third)
        let rt = classify_run(20.0, 5.0, (65.0, 30.0), (85.0, 35.0));
        assert_eq!(rt, RunType::InBehind);
    }

    #[test]
    fn test_classify_run_defensive() {
        // Defensive run
        let rt = classify_run(-15.0, 5.0, (50.0, 30.0), (35.0, 35.0));
        assert_eq!(rt, RunType::Defensive);
    }

    #[test]
    fn test_classify_run_diagonal() {
        // Diagonal run
        let rt = classify_run(10.0, 10.0, (40.0, 30.0), (50.0, 40.0));
        assert_eq!(rt, RunType::Diagonal);
    }

    #[test]
    fn test_classify_run_overlap() {
        // Wide overlap run
        let rt = classify_run(5.0, 20.0, (50.0, 10.0), (55.0, 30.0));
        assert_eq!(rt, RunType::Overlap);
    }

    #[test]
    fn test_run_minimum_duration() {
        // Run only 800ms - should be rejected
        let data = vec![
            make_position_item(0, 30.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(400, 32.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(800, 34.0, 34.0, 2.0, 0.0, Some(PlayerState::Attacking)),
        ];

        let runs = extract_player_runs(5, 0, &data);
        assert!(runs.is_empty(), "Run < 1000ms should be rejected");
    }

    #[test]
    fn test_run_minimum_distance() {
        // Run long enough but only 4m
        let data = vec![
            make_position_item(0, 30.0, 34.0, 4.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(500, 31.0, 34.0, 4.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1000, 32.0, 34.0, 4.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1500, 33.0, 34.0, 4.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(2000, 34.0, 34.0, 2.0, 0.0, Some(PlayerState::Attacking)),
        ];

        let runs = extract_player_runs(5, 0, &data);
        assert!(runs.is_empty(), "Run < 6m should be rejected");
    }

    #[test]
    fn test_valid_run_extraction() {
        // Valid off-ball run: 1500ms, 7.5m
        let data = vec![
            make_position_item(0, 30.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(500, 32.5, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1000, 35.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1500, 37.5, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(2000, 40.0, 34.0, 2.0, 0.0, Some(PlayerState::Attacking)),
        ];

        let runs = extract_player_runs(5, 0, &data);
        assert_eq!(runs.len(), 1, "Should detect 1 run");

        let run = &runs[0];
        assert_eq!(run.player_idx, 5);
        assert!(run.is_off_ball);
        assert!(run.distance_m >= 7.0);
        assert_eq!(run.run_type, RunType::Forward);
    }

    #[test]
    fn test_run_ends_when_receiving_ball() {
        // Run that ends when player receives ball
        // Need: >= 1000ms duration, >= 6m distance
        let data = vec![
            make_position_item(0, 30.0, 34.0, 5.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(500, 32.75, 34.0, 5.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1000, 35.5, 34.0, 5.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1500, 38.25, 34.0, 5.5, 0.0, Some(PlayerState::Attacking)),
            make_position_item(2000, 41.0, 34.0, 5.5, 0.0, Some(PlayerState::WithBall)), // Got ball
            make_position_item(2500, 43.75, 34.0, 5.0, 0.0, Some(PlayerState::WithBall)),
        ];

        let runs = extract_player_runs(5, 0, &data);
        assert_eq!(runs.len(), 1, "Should detect 1 run, got {}", runs.len());
        // Run should end before receiving ball at 2000ms
        assert!(runs[0].t1_ms <= 1500, "Run should end before getting ball at 2000ms, ended at {}", runs[0].t1_ms);
    }

    #[test]
    fn test_multiple_runs() {
        // Two separate runs - each must be >= 1000ms, >= 6m
        let data = vec![
            // First run: 1500ms, 7.5m (30 → 37.5)
            make_position_item(0, 30.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(500, 32.5, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1000, 35.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(1500, 37.5, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(2000, 38.0, 34.0, 2.0, 0.0, Some(PlayerState::Attacking)), // End run
            // Walking
            make_position_item(2500, 38.5, 34.0, 2.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(3000, 39.0, 34.0, 2.0, 0.0, Some(PlayerState::Attacking)),
            // Second run: 1500ms, 10m (39 → 49)
            make_position_item(3500, 41.5, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(4000, 44.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(4500, 46.5, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(5000, 49.0, 34.0, 5.0, 0.0, Some(PlayerState::Attacking)),
            make_position_item(5500, 50.0, 34.0, 2.0, 0.0, Some(PlayerState::Attacking)), // End run
        ];

        let runs = extract_player_runs(5, 0, &data);
        assert_eq!(runs.len(), 2, "Should detect 2 runs, got {}", runs.len());
    }

    #[test]
    fn test_team_run_stats() {
        let runs = vec![
            RunEvent {
                player_idx: 5,
                team_side: 0,
                t0_ms: 1000,
                t1_ms: 3000,
                distance_m: 12.0,
                delta_x_attack_m: 10.0,
                lateral_movement_m: 6.6,
                is_off_ball: true,
                run_type: RunType::Forward,
                start_pos_m: (30.0, 34.0),
                end_pos_m: (40.0, 40.6),
                max_speed_mps: 6.0,
            },
            RunEvent {
                player_idx: 9,
                team_side: 0,
                t0_ms: 5000,
                t1_ms: 7000,
                distance_m: 8.0,
                delta_x_attack_m: 5.0,
                lateral_movement_m: 6.2,
                is_off_ball: true,
                run_type: RunType::Diagonal,
                start_pos_m: (50.0, 20.0),
                end_pos_m: (55.0, 26.2),
                max_speed_mps: 5.5,
            },
        ];

        // 7 minutes played
        let stats = calculate_team_run_stats(&runs, 0, 7 * 60 * 1000);
        assert_eq!(stats.total_runs, 2);
        assert_eq!(stats.off_ball_runs, 2);
        assert_eq!(stats.forward_runs, 1);
        assert_eq!(stats.diagonal_runs, 1);
        assert!(stats.runs_per_90 > 20.0); // 2 runs in 7 min → ~25.7/90
    }
}
