//! FIX01 — Input-Application Enforcement (A1 + A2)
//!
//! Centralized contracts used across API/model/engine boundaries.

use crate::engine::types::Coord10;
use crate::models::team::Formation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod error_codes {
    pub const UNSUPPORTED_FORMATION: &str = "UNSUPPORTED_FORMATION";
    pub const UNSUPPORTED_POSITION_MAPPING: &str = "UNSUPPORTED_POSITION_MAPPING";
    pub const INVALID_CONDITION_RANGE: &str = "INVALID_CONDITION_RANGE";
    pub const INPUT_NOT_APPLIED_FORMATION: &str = "INPUT_NOT_APPLIED_FORMATION";
    pub const INPUT_NOT_APPLIED_POSITION: &str = "INPUT_NOT_APPLIED_POSITION";
    pub const INPUT_NOT_APPLIED_CONDITION: &str = "INPUT_NOT_APPLIED_CONDITION";
}

pub const CONDITION_MODEL_ID: &str = "cond_v1_5step";

/// FIX01 C1: ConditionLevel = u8 (1..=5)
#[inline]
pub fn is_valid_condition_level(level: u8) -> bool {
    (1..=5).contains(&level)
}

/// FIX01 C1: stamina drain multiplier (lower condition => faster drain).
#[inline]
pub fn condition_drain_mult(level: u8) -> f32 {
    match level {
        5 => 0.80,
        4 => 0.90,
        3 => 1.00,
        2 => 1.20,
        1 => 1.50,
        _ => 1.00, // invalid levels should be rejected at boundaries
    }
}

/// FIX01 C1: decision quality multiplier (higher condition => slightly better execution/decisions).
#[inline]
pub fn condition_decision_mult(level: u8) -> f32 {
    match level {
        5 => 1.05,
        4 => 1.02,
        3 => 1.00,
        2 => 0.97,
        1 => 0.92,
        _ => 1.00, // invalid levels should be rejected at boundaries
    }
}

/// FIX01 F2: supported formation allowlist (14).
#[inline]
pub fn is_supported_formation(formation: &Formation) -> bool {
    matches!(
        formation,
        Formation::F442
            | Formation::F433
            | Formation::F4411
            | Formation::F4321
            | Formation::F4222
            | Formation::F451
            | Formation::F352
            | Formation::F3421
            | Formation::F3412
            | Formation::F532
            | Formation::F4231
            | Formation::F4141
            | Formation::F343
            | Formation::F541
    )
}

// ============================================================================
// SSOT Proof (P1: included in result JSON)
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TeamShapeParams {
    pub width: f32,         // 0..1
    pub depth: f32,         // 0..1
    pub line_height: f32,   // 0..1
    pub pressing_bias: f32, // 0..1
    pub compactness: f32,   // 0..1
}

impl TeamShapeParams {
    pub fn clamped_01(self) -> Self {
        Self {
            width: self.width.clamp(0.0, 1.0),
            depth: self.depth.clamp(0.0, 1.0),
            line_height: self.line_height.clamp(0.0, 1.0),
            pressing_bias: self.pressing_bias.clamp(0.0, 1.0),
            compactness: self.compactness.clamp(0.0, 1.0),
        }
    }
}

impl Default for TeamShapeParams {
    fn default() -> Self {
        Self { width: 0.6, depth: 0.6, line_height: 0.6, pressing_bias: 0.6, compactness: 0.6 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormationProof {
    pub home_formation_code: String,
    pub away_formation_code: String,
    pub formation_layout_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShapeProof {
    pub home_shape_params: TeamShapeParams,
    pub away_shape_params: TeamShapeParams,
    pub shape_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConditionProof {
    pub condition_model_id: String,
    pub condition_applied_count: u8,
    pub condition_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SsotProof {
    pub formation: FormationProof,
    pub shape: ShapeProof,
    pub condition: ConditionProof,
}

impl Default for SsotProof {
    fn default() -> Self {
        Self {
            formation: FormationProof {
                home_formation_code: String::new(),
                away_formation_code: String::new(),
                formation_layout_hash: String::new(),
            },
            shape: ShapeProof {
                home_shape_params: TeamShapeParams::default(),
                away_shape_params: TeamShapeParams::default(),
                shape_hash: String::new(),
            },
            condition: ConditionProof {
                condition_model_id: CONDITION_MODEL_ID.to_string(),
                condition_applied_count: 0,
                condition_hash: String::new(),
            },
        }
    }
}

#[inline]
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[inline]
fn quantize_1e4(v: f32) -> i32 {
    (v.clamp(0.0, 1.0) * 10_000.0).round() as i32
}

pub fn shape_params_from_formation(formation: &Formation) -> TeamShapeParams {
    match formation {
        Formation::F442 => TeamShapeParams {
            width: 0.60,
            depth: 0.58,
            line_height: 0.55,
            pressing_bias: 0.55,
            compactness: 0.58,
        },
        Formation::F433 => TeamShapeParams {
            width: 0.72,
            depth: 0.62,
            line_height: 0.60,
            pressing_bias: 0.60,
            compactness: 0.52,
        },
        Formation::F4231 => TeamShapeParams {
            width: 0.68,
            depth: 0.60,
            line_height: 0.58,
            pressing_bias: 0.58,
            compactness: 0.56,
        },
        Formation::F4141 => TeamShapeParams {
            width: 0.62,
            depth: 0.58,
            line_height: 0.56,
            pressing_bias: 0.56,
            compactness: 0.60,
        },
        Formation::F4411 => TeamShapeParams {
            width: 0.60,
            depth: 0.57,
            line_height: 0.55,
            pressing_bias: 0.55,
            compactness: 0.58,
        },
        Formation::F4321 => TeamShapeParams {
            width: 0.58,
            depth: 0.62,
            line_height: 0.58,
            pressing_bias: 0.58,
            compactness: 0.55,
        },
        Formation::F4222 => TeamShapeParams {
            width: 0.62,
            depth: 0.60,
            line_height: 0.56,
            pressing_bias: 0.57,
            compactness: 0.58,
        },
        Formation::F451 => TeamShapeParams {
            width: 0.64,
            depth: 0.55,
            line_height: 0.52,
            pressing_bias: 0.52,
            compactness: 0.62,
        },
        Formation::F352 => TeamShapeParams {
            width: 0.78,
            depth: 0.55,
            line_height: 0.54,
            pressing_bias: 0.54,
            compactness: 0.62,
        },
        Formation::F343 => TeamShapeParams {
            width: 0.80,
            depth: 0.64,
            line_height: 0.62,
            pressing_bias: 0.62,
            compactness: 0.50,
        },
        Formation::F3421 => TeamShapeParams {
            width: 0.74,
            depth: 0.60,
            line_height: 0.58,
            pressing_bias: 0.58,
            compactness: 0.54,
        },
        Formation::F3412 => TeamShapeParams {
            width: 0.70,
            depth: 0.58,
            line_height: 0.56,
            pressing_bias: 0.56,
            compactness: 0.58,
        },
        Formation::F532 => TeamShapeParams {
            width: 0.74,
            depth: 0.52,
            line_height: 0.48,
            pressing_bias: 0.48,
            compactness: 0.66,
        },
        Formation::F541 => TeamShapeParams {
            width: 0.68,
            depth: 0.50,
            line_height: 0.46,
            pressing_bias: 0.46,
            compactness: 0.68,
        },
    }
}

pub fn shape_hash(home: TeamShapeParams, away: TeamShapeParams) -> String {
    let home = home.clamped_01();
    let away = away.clamped_01();

    let mut buf = Vec::with_capacity(10 * 4);
    for v in [
        home.width,
        home.depth,
        home.line_height,
        home.pressing_bias,
        home.compactness,
        away.width,
        away.depth,
        away.line_height,
        away.pressing_bias,
        away.compactness,
    ] {
        buf.extend_from_slice(&quantize_1e4(v).to_le_bytes());
    }

    sha256_hex(&buf)
}

pub fn condition_hash(levels_by_track_id: &[u8]) -> Result<String, String> {
    if levels_by_track_id.len() != 22 {
        return Err(format!(
            "condition_hash: expected 22 levels, got {}",
            levels_by_track_id.len()
        ));
    }
    let mut buf = Vec::with_capacity(22 * 2);
    for (track_id, level) in levels_by_track_id.iter().copied().enumerate() {
        buf.push(track_id as u8);
        buf.push(level);
    }
    Ok(sha256_hex(&buf))
}

pub fn formation_layout_hash(positions_by_track_id: &[Coord10]) -> Result<String, String> {
    if positions_by_track_id.len() != 22 {
        return Err(format!(
            "formation_layout_hash: expected 22 positions, got {}",
            positions_by_track_id.len()
        ));
    }

    let mut buf = Vec::with_capacity(22 * (1 + 4 + 4));
    for (track_id, pos) in positions_by_track_id.iter().enumerate() {
        // FIX01 7.2: [track_id:u8, x_cm:i32LE, y_cm:i32LE] × 22
        buf.push(track_id as u8);
        let x_cm: i32 = pos.x.saturating_mul(10); // 0.1m -> 10cm
        let y_cm: i32 = pos.y.saturating_mul(10);
        buf.extend_from_slice(&x_cm.to_le_bytes());
        buf.extend_from_slice(&y_cm.to_le_bytes());
    }
    Ok(sha256_hex(&buf))
}

pub fn build_ssot_proof_pre_kickoff(
    home_formation: &Formation,
    away_formation: &Formation,
    levels_by_track_id: &[u8],
) -> Result<SsotProof, String> {
    let home_shape = shape_params_from_formation(home_formation);
    let away_shape = shape_params_from_formation(away_formation);
    let shape_hash = shape_hash(home_shape, away_shape);

    Ok(SsotProof {
        formation: FormationProof {
            home_formation_code: home_formation.code().to_string(),
            away_formation_code: away_formation.code().to_string(),
            formation_layout_hash: String::new(),
        },
        shape: ShapeProof { home_shape_params: home_shape, away_shape_params: away_shape, shape_hash },
        condition: ConditionProof {
            condition_model_id: CONDITION_MODEL_ID.to_string(),
            condition_applied_count: 22,
            condition_hash: condition_hash(levels_by_track_id)?,
        },
    })
}

pub fn set_formation_layout_hash_from_positions(
    proof: &mut SsotProof,
    positions_by_track_id: &[Coord10],
) -> Result<(), String> {
    proof.formation.formation_layout_hash = formation_layout_hash(positions_by_track_id)?;
    Ok(())
}
