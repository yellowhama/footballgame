//! # Physics Validator
//!
//! Layer 1 of Football Likeness QA - validates physical plausibility.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: REALTIME_SYSTEMS_ANALYSIS.md (Football Likeness QA)
//!
//! ## Checks
//! 1. Player speed <= 12.0 m/s (43 km/h)
//! 2. Ball speed <= 45.0 m/s (162 km/h)
//! 3. Position within bounds (0-105 × 0-68 with 2m margin)
//! 4. No teleportation (position jump > implied by max speed)

use crate::models::match_result::{MatchPositionData, PositionDataItem};

/// Maximum realistic player speed: 12.0 m/s (43.2 km/h)
pub const MAX_PLAYER_SPEED_MPS: f32 = 12.0;

/// Maximum realistic player acceleration: 8.0 m/s²
pub const MAX_PLAYER_ACCEL_MPS2: f32 = 8.0;

/// Maximum realistic ball speed: 45.0 m/s (162 km/h)
pub const MAX_BALL_SPEED_MPS: f32 = 45.0;

/// Pitch boundaries (standard pitch)
pub const PITCH_LENGTH_M: f32 = 105.0;
pub const PITCH_WIDTH_M: f32 = 68.0;

/// Physics anomaly types detected during validation.
#[derive(Debug, Clone)]
pub enum PhysicsAnomaly {
    /// Player exceeded maximum realistic speed
    PlayerSpeedExceeded {
        player_idx: u8,
        team_side: u8,
        timestamp_ms: u64,
        speed_mps: f32,
        threshold_mps: f32,
    },
    /// Player acceleration exceeded realistic limits
    PlayerAccelExceeded {
        player_idx: u8,
        team_side: u8,
        timestamp_ms: u64,
        accel_mps2: f32,
        threshold_mps2: f32,
    },
    /// Ball exceeded maximum realistic speed
    BallSpeedExceeded {
        timestamp_ms: u64,
        speed_mps: f32,
        threshold_mps: f32,
    },
    /// Entity is out of pitch bounds
    OutOfBounds {
        entity: EntityType,
        timestamp_ms: u64,
        position: (f32, f32),
    },
    /// Player teleported (position changed too much between frames)
    PlayerTeleport {
        player_idx: u8,
        team_side: u8,
        timestamp_ms: u64,
        distance_m: f32,
        time_delta_ms: u64,
    },
}

/// Entity type for anomaly reporting.
#[derive(Debug, Clone, Copy)]
pub enum EntityType {
    Player { idx: u8, team_side: u8 },
    Ball,
}

impl PhysicsAnomaly {
    /// Severity of the anomaly (0.0 = minor, 1.0 = critical).
    pub fn severity(&self) -> f32 {
        match self {
            PhysicsAnomaly::PlayerSpeedExceeded { speed_mps, threshold_mps, .. } => {
                ((speed_mps - threshold_mps) / threshold_mps).min(1.0).max(0.0)
            }
            PhysicsAnomaly::PlayerAccelExceeded { accel_mps2, threshold_mps2, .. } => {
                ((accel_mps2 - threshold_mps2) / threshold_mps2).min(1.0).max(0.0)
            }
            PhysicsAnomaly::BallSpeedExceeded { speed_mps, threshold_mps, .. } => {
                ((speed_mps - threshold_mps) / threshold_mps).min(1.0).max(0.0)
            }
            PhysicsAnomaly::OutOfBounds { position, .. } => {
                // How far out of bounds
                let x_excess = if position.0 < 0.0 {
                    -position.0
                } else if position.0 > PITCH_LENGTH_M {
                    position.0 - PITCH_LENGTH_M
                } else {
                    0.0
                };
                let y_excess = if position.1 < 0.0 {
                    -position.1
                } else if position.1 > PITCH_WIDTH_M {
                    position.1 - PITCH_WIDTH_M
                } else {
                    0.0
                };
                ((x_excess + y_excess) / 10.0).min(1.0)
            }
            PhysicsAnomaly::PlayerTeleport { distance_m, time_delta_ms, .. } => {
                // Speed implied by teleport
                let dt = *time_delta_ms as f32 / 1000.0;
                if dt > 0.0 {
                    let implied_speed = distance_m / dt;
                    ((implied_speed - MAX_PLAYER_SPEED_MPS) / MAX_PLAYER_SPEED_MPS).min(1.0).max(0.0)
                } else {
                    1.0
                }
            }
        }
    }
}

/// Physics validation configuration.
#[derive(Debug, Clone)]
pub struct PhysicsValidatorConfig {
    pub max_player_speed_mps: f32,
    pub max_player_accel_mps2: f32,
    pub max_ball_speed_mps: f32,
    pub pitch_length_m: f32,
    pub pitch_width_m: f32,
    /// Margin for out-of-bounds detection (allows slight overshoots)
    pub bounds_margin_m: f32,
}

impl Default for PhysicsValidatorConfig {
    fn default() -> Self {
        Self {
            max_player_speed_mps: MAX_PLAYER_SPEED_MPS,
            max_player_accel_mps2: MAX_PLAYER_ACCEL_MPS2,
            max_ball_speed_mps: MAX_BALL_SPEED_MPS,
            pitch_length_m: PITCH_LENGTH_M,
            pitch_width_m: PITCH_WIDTH_M,
            bounds_margin_m: 2.0,
        }
    }
}

/// Physics validation result.
#[derive(Debug, Clone, Default)]
pub struct PhysicsValidationResult {
    /// All detected anomalies
    pub anomalies: Vec<PhysicsAnomaly>,
    /// Number of frames validated
    pub frames_validated: u32,
    /// Number of anomaly-free frames
    pub clean_frames: u32,
    /// Physics integrity score (0-100)
    pub integrity_score: f32,
}

impl PhysicsValidationResult {
    /// Whether the result passes QA thresholds.
    pub fn passes(&self, min_score: f32) -> bool {
        self.integrity_score >= min_score
    }

    /// Calculate integrity score from anomalies.
    pub fn calculate_score(&mut self) {
        if self.frames_validated == 0 {
            self.integrity_score = 100.0;
            return;
        }

        // Weight anomalies by severity
        let total_severity: f32 = self.anomalies.iter()
            .map(|a| a.severity())
            .sum();

        // Score based on anomaly density and severity
        let anomaly_rate = total_severity / self.frames_validated as f32;
        self.integrity_score = (100.0 * (1.0 - anomaly_rate * 10.0)).max(0.0).min(100.0);

        self.clean_frames = self.frames_validated.saturating_sub(self.anomalies.len() as u32);
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

/// Validate physics plausibility of position data.
///
/// # Arguments
/// * `position_data` - Position/velocity snapshots over time
/// * `config` - Validation configuration
///
/// # Returns
/// PhysicsValidationResult with detected anomalies
pub fn validate_physics(
    position_data: &MatchPositionData,
    config: &PhysicsValidatorConfig,
) -> PhysicsValidationResult {
    let mut result = PhysicsValidationResult::default();

    // Validate ball
    validate_ball(&position_data.ball, config, &mut result);

    // Validate all players
    for player_idx in 0..22u8 {
        let player_data = &position_data.players[player_idx as usize];
        if player_data.is_empty() {
            continue;
        }

        let team_side = if player_idx < 11 { 0 } else { 1 };
        validate_player(player_idx, team_side, player_data, config, &mut result);
    }

    result.calculate_score();
    result
}

/// Validate ball physics.
fn validate_ball(
    data: &[PositionDataItem],
    config: &PhysicsValidatorConfig,
    result: &mut PhysicsValidationResult,
) {
    for (i, item) in data.iter().enumerate() {
        result.frames_validated += 1;

        // Check speed from velocity
        let speed = speed_from_velocity(item.velocity);
        if speed > config.max_ball_speed_mps {
            result.anomalies.push(PhysicsAnomaly::BallSpeedExceeded {
                timestamp_ms: item.timestamp,
                speed_mps: speed,
                threshold_mps: config.max_ball_speed_mps,
            });
        }

        // Check speed from position delta
        if i > 0 {
            let prev = &data[i - 1];
            let dt_ms = item.timestamp.saturating_sub(prev.timestamp);
            if dt_ms > 0 {
                let implied_speed = speed_from_positions(prev.position, item.position, dt_ms);
                if implied_speed > config.max_ball_speed_mps {
                    result.anomalies.push(PhysicsAnomaly::BallSpeedExceeded {
                        timestamp_ms: item.timestamp,
                        speed_mps: implied_speed,
                        threshold_mps: config.max_ball_speed_mps,
                    });
                }
            }
        }

        // Check bounds
        if !is_in_bounds(item.position, config) {
            result.anomalies.push(PhysicsAnomaly::OutOfBounds {
                entity: EntityType::Ball,
                timestamp_ms: item.timestamp,
                position: item.position,
            });
        }
    }
}

/// Validate player physics.
fn validate_player(
    player_idx: u8,
    team_side: u8,
    data: &[PositionDataItem],
    config: &PhysicsValidatorConfig,
    result: &mut PhysicsValidationResult,
) {
    let mut prev_speed: Option<f32> = None;
    let mut prev_timestamp: Option<u64> = None;

    for (i, item) in data.iter().enumerate() {
        result.frames_validated += 1;

        // Check speed from velocity
        let speed = if item.velocity.is_some() {
            speed_from_velocity(item.velocity)
        } else if i > 0 {
            let prev = &data[i - 1];
            let dt_ms = item.timestamp.saturating_sub(prev.timestamp);
            speed_from_positions(prev.position, item.position, dt_ms)
        } else {
            0.0
        };

        if speed > config.max_player_speed_mps {
            result.anomalies.push(PhysicsAnomaly::PlayerSpeedExceeded {
                player_idx,
                team_side,
                timestamp_ms: item.timestamp,
                speed_mps: speed,
                threshold_mps: config.max_player_speed_mps,
            });
        }

        // Check acceleration
        if let (Some(ps), Some(pt)) = (prev_speed, prev_timestamp) {
            let dt_s = (item.timestamp.saturating_sub(pt)) as f32 / 1000.0;
            if dt_s > 0.0 {
                let accel = (speed - ps).abs() / dt_s;
                if accel > config.max_player_accel_mps2 {
                    result.anomalies.push(PhysicsAnomaly::PlayerAccelExceeded {
                        player_idx,
                        team_side,
                        timestamp_ms: item.timestamp,
                        accel_mps2: accel,
                        threshold_mps2: config.max_player_accel_mps2,
                    });
                }
            }
        }

        // Check for teleportation
        if i > 0 {
            let prev = &data[i - 1];
            let dt_ms = item.timestamp.saturating_sub(prev.timestamp);
            let dx = item.position.0 - prev.position.0;
            let dy = item.position.1 - prev.position.1;
            let distance = (dx * dx + dy * dy).sqrt();

            // Max possible distance at max speed
            let max_distance = config.max_player_speed_mps * (dt_ms as f32 / 1000.0);
            if distance > max_distance * 1.5 {  // 50% tolerance
                result.anomalies.push(PhysicsAnomaly::PlayerTeleport {
                    player_idx,
                    team_side,
                    timestamp_ms: item.timestamp,
                    distance_m: distance,
                    time_delta_ms: dt_ms,
                });
            }
        }

        // Check bounds
        if !is_in_bounds(item.position, config) {
            result.anomalies.push(PhysicsAnomaly::OutOfBounds {
                entity: EntityType::Player { idx: player_idx, team_side },
                timestamp_ms: item.timestamp,
                position: item.position,
            });
        }

        prev_speed = Some(speed);
        prev_timestamp = Some(item.timestamp);
    }
}

/// Check if a position is within pitch bounds (with margin).
pub fn is_in_bounds(pos: (f32, f32), config: &PhysicsValidatorConfig) -> bool {
    let margin = config.bounds_margin_m;
    pos.0 >= -margin
        && pos.0 <= config.pitch_length_m + margin
        && pos.1 >= -margin
        && pos.1 <= config.pitch_width_m + margin
}

/// Quick validation that just returns pass/fail.
pub fn quick_validate(position_data: &MatchPositionData) -> bool {
    let config = PhysicsValidatorConfig::default();
    let result = validate_physics(position_data, &config);
    result.passes(70.0)
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
    fn test_bounds_check() {
        let config = PhysicsValidatorConfig::default();

        // In bounds
        assert!(is_in_bounds((52.5, 34.0), &config));
        assert!(is_in_bounds((0.0, 0.0), &config));
        assert!(is_in_bounds((105.0, 68.0), &config));

        // Slightly out (within margin)
        assert!(is_in_bounds((-1.0, 34.0), &config));

        // Clearly out
        assert!(!is_in_bounds((-5.0, 34.0), &config));
        assert!(!is_in_bounds((52.5, -5.0), &config));
    }

    #[test]
    fn test_anomaly_severity() {
        let anomaly = PhysicsAnomaly::PlayerSpeedExceeded {
            player_idx: 5,
            team_side: 0,
            timestamp_ms: 10000,
            speed_mps: 15.0,  // 25% over limit
            threshold_mps: 12.0,
        };

        let severity = anomaly.severity();
        assert!(severity > 0.2 && severity < 0.3);
    }

    #[test]
    fn test_speed_from_velocity() {
        assert!((speed_from_velocity(Some((6.0, 0.0))) - 6.0).abs() < 0.01);
        assert!((speed_from_velocity(Some((3.0, 4.0))) - 5.0).abs() < 0.01);
        assert_eq!(speed_from_velocity(None), 0.0);
    }

    #[test]
    fn test_detect_excessive_speed() {
        let config = PhysicsValidatorConfig::default();

        // Create position data with excessive speed
        let data = vec![
            make_position_item(0, 30.0, 34.0, 5.0, 0.0),      // Normal
            make_position_item(250, 31.25, 34.0, 5.0, 0.0),   // Normal
            make_position_item(500, 32.5, 34.0, 15.0, 0.0),   // Excessive!
            make_position_item(750, 36.25, 34.0, 5.0, 0.0),   // Normal
        ];

        let mut position_data = MatchPositionData::new();
        position_data.players[5] = data;

        let result = validate_physics(&position_data, &config);

        // Should detect at least one speed violation
        let speed_violations: Vec<_> = result.anomalies.iter()
            .filter(|a| matches!(a, PhysicsAnomaly::PlayerSpeedExceeded { .. }))
            .collect();

        assert!(!speed_violations.is_empty(), "Should detect excessive speed");
    }

    #[test]
    fn test_detect_out_of_bounds() {
        let config = PhysicsValidatorConfig::default();

        // Create position data with out-of-bounds position
        let data = vec![
            make_position_item(0, 30.0, 34.0, 5.0, 0.0),
            make_position_item(250, 110.0, 34.0, 0.0, 0.0),   // Out of bounds!
            make_position_item(500, 50.0, 34.0, 0.0, 0.0),
        ];

        let mut position_data = MatchPositionData::new();
        position_data.players[5] = data;

        let result = validate_physics(&position_data, &config);

        let oob_violations: Vec<_> = result.anomalies.iter()
            .filter(|a| matches!(a, PhysicsAnomaly::OutOfBounds { .. }))
            .collect();

        assert!(!oob_violations.is_empty(), "Should detect out of bounds");
    }

    #[test]
    fn test_detect_teleportation() {
        let config = PhysicsValidatorConfig::default();

        // Create position data with teleportation (50m in 250ms = 200m/s)
        let data = vec![
            make_position_item(0, 30.0, 34.0, 0.0, 0.0),
            make_position_item(250, 80.0, 34.0, 0.0, 0.0),   // Teleport!
            make_position_item(500, 82.0, 34.0, 0.0, 0.0),
        ];

        let mut position_data = MatchPositionData::new();
        position_data.players[5] = data;

        let result = validate_physics(&position_data, &config);

        let teleport_violations: Vec<_> = result.anomalies.iter()
            .filter(|a| matches!(a, PhysicsAnomaly::PlayerTeleport { .. }))
            .collect();

        assert!(!teleport_violations.is_empty(), "Should detect teleportation");
    }

    #[test]
    fn test_clean_data_passes() {
        let config = PhysicsValidatorConfig::default();

        // Create clean position data
        let data = vec![
            make_position_item(0, 30.0, 34.0, 5.0, 0.0),
            make_position_item(250, 31.25, 34.0, 5.0, 0.0),
            make_position_item(500, 32.5, 34.0, 5.0, 0.0),
            make_position_item(750, 33.75, 34.0, 5.0, 0.0),
        ];

        let mut position_data = MatchPositionData::new();
        position_data.players[5] = data;

        let result = validate_physics(&position_data, &config);

        assert!(result.anomalies.is_empty(), "Clean data should have no anomalies");
        assert!(result.integrity_score >= 90.0, "Clean data should have high score");
    }

    #[test]
    fn test_ball_speed_validation() {
        let config = PhysicsValidatorConfig::default();

        // Create ball data with excessive speed
        let ball_data = vec![
            make_position_item(0, 52.5, 34.0, 10.0, 0.0),     // Normal
            make_position_item(250, 55.0, 34.0, 50.0, 0.0),   // Excessive!
            make_position_item(500, 67.5, 34.0, 10.0, 0.0),
        ];

        let mut position_data = MatchPositionData::new();
        position_data.ball = ball_data;

        let result = validate_physics(&position_data, &config);

        let ball_violations: Vec<_> = result.anomalies.iter()
            .filter(|a| matches!(a, PhysicsAnomaly::BallSpeedExceeded { .. }))
            .collect();

        assert!(!ball_violations.is_empty(), "Should detect excessive ball speed");
    }

    #[test]
    fn test_validation_score() {
        let mut result = PhysicsValidationResult {
            anomalies: vec![
                PhysicsAnomaly::PlayerSpeedExceeded {
                    player_idx: 5,
                    team_side: 0,
                    timestamp_ms: 1000,
                    speed_mps: 13.0,
                    threshold_mps: 12.0,
                },
            ],
            frames_validated: 100,
            clean_frames: 0,
            integrity_score: 0.0,
        };

        result.calculate_score();

        // With 1 minor anomaly in 100 frames, score should be high
        assert!(result.integrity_score > 80.0);
        assert!(result.integrity_score < 100.0);
    }
}
