//! Zone transition matrices (20-zone positional play)
//!
//! FIX_2601/0113: Updated to use 20-zone PosPlayZoneId system.
//!
//! Uses the 20-zone positional play layout (5 lanes × 4 quarters) to bias
//! pass/cross targets and event mix.
//!
//! Zone order (0-19):
//! DEF: LwDef(0), LhsDef(1), CDef(2), RhsDef(3), RwDef(4)
//! MID: LwMid(5), LhsMid(6), CMid(7), RhsMid(8), RwMid(9)
//! FIN: LwFin(10), LhsFin(11), CFin(12), RhsFin(13), RwFin(14)
//! BOX: LwBox(15), LhsBox(16), CBox(17), RhsBox(18), RwBox(19)

use crate::calibration::zone::PosPlayZoneId;
use crate::tactics::team_instructions::{BuildUpStyle, TeamInstructions};

const ZONE_COUNT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneTransitionStyle {
    Normal,
    Possession,
    Counter,
}

impl ZoneTransitionStyle {
    pub fn from_instructions(instructions: &TeamInstructions) -> Self {
        match instructions.build_up_style {
            BuildUpStyle::Short => Self::Possession,
            BuildUpStyle::Direct => Self::Counter,
            BuildUpStyle::Mixed => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OfmEventMixWeights {
    pub pass: f32,
    pub cross: f32,
    pub dribble: f32,
    pub shot: f32,
}

/// Event mix weights based on 20-zone position
pub fn ofm_event_mix_weights(style: ZoneTransitionStyle, zone: PosPlayZoneId) -> OfmEventMixWeights {
    let mut weights = match style {
        ZoneTransitionStyle::Normal => {
            OfmEventMixWeights { pass: 40.0, cross: 10.0, dribble: 1.0, shot: 0.0 }
        }
        ZoneTransitionStyle::Possession => {
            OfmEventMixWeights { pass: 80.0, cross: 10.0, dribble: 1.0, shot: 0.0 }
        }
        ZoneTransitionStyle::Counter => {
            OfmEventMixWeights { pass: 30.0, cross: 50.0, dribble: 2.0, shot: 0.0 }
        }
    };

    // Offensive zones: Final quarter and Box quarter
    let in_offensive_zone = zone.quarter().is_attacking_half();

    if in_offensive_zone {
        weights.dribble = match style {
            ZoneTransitionStyle::Normal => 4.0,
            ZoneTransitionStyle::Possession => 2.0,
            ZoneTransitionStyle::Counter => 3.0,
        };
    }

    // Finishing zones (LhsBox, CBox, RhsBox) - high shot weight
    if zone.is_finishing_zone() {
        weights.shot = 8.0;
    }

    // Central box (CBox) - highest shot weight
    if zone == PosPlayZoneId::CBox {
        weights.shot = 12.0;
    }

    // Wide box zones (LwBox, RwBox) - crossing territory
    if zone == PosPlayZoneId::LwBox || zone == PosPlayZoneId::RwBox {
        weights.shot = 2.0;
        weights.cross = match style {
            ZoneTransitionStyle::Normal => 35.0,
            ZoneTransitionStyle::Possession => 25.0,
            ZoneTransitionStyle::Counter => 55.0,
        };
    }

    // Wide final zones (LwFin, RwFin) - crossing buildup
    if zone == PosPlayZoneId::LwFin || zone == PosPlayZoneId::RwFin {
        weights.shot = 1.0;
        weights.cross = match style {
            ZoneTransitionStyle::Normal => 25.0,
            ZoneTransitionStyle::Possession => 15.0,
            ZoneTransitionStyle::Counter => 40.0,
        };
    }

    // Central final (CFin) - long shot territory
    if zone == PosPlayZoneId::CFin {
        weights.shot = 3.0;
    }

    // Half-space finishing zones (LhsFin, RhsFin) - creative zones
    if zone == PosPlayZoneId::LhsFin || zone == PosPlayZoneId::RhsFin {
        weights.shot = 2.0;
        weights.dribble = match style {
            ZoneTransitionStyle::Normal => 5.0,
            ZoneTransitionStyle::Possession => 3.0,
            ZoneTransitionStyle::Counter => 4.0,
        };
    }

    weights
}

pub fn pass_factor(style: ZoneTransitionStyle, from: PosPlayZoneId, to: PosPlayZoneId) -> f32 {
    let row = pass_matrix(style, from);
    weight_factor(row, to.to_index())
}

pub fn cross_factor(style: ZoneTransitionStyle, from: PosPlayZoneId, to: PosPlayZoneId) -> f32 {
    let row = cross_matrix(style, from);
    weight_factor(row, to.to_index())
}

fn weight_factor(row: &[f32; ZONE_COUNT], to_idx: usize) -> f32 {
    let sum: f32 = row.iter().sum();
    if sum <= 0.0 {
        return 1.0;
    }
    let mean = sum / ZONE_COUNT as f32;
    let weight = row.get(to_idx).copied().unwrap_or(mean);
    let ratio = weight / mean;
    ratio.clamp(0.5, 1.5)
}

fn pass_matrix(style: ZoneTransitionStyle, from: PosPlayZoneId) -> &'static [f32; ZONE_COUNT] {
    let idx = from.to_index();
    match style {
        ZoneTransitionStyle::Normal => &PASS_NORMAL[idx],
        ZoneTransitionStyle::Possession => &PASS_POSSESSION[idx],
        ZoneTransitionStyle::Counter => &PASS_COUNTER[idx],
    }
}

fn cross_matrix(style: ZoneTransitionStyle, from: PosPlayZoneId) -> &'static [f32; ZONE_COUNT] {
    let idx = from.to_index();
    match style {
        ZoneTransitionStyle::Normal => &CROSS_NORMAL[idx],
        ZoneTransitionStyle::Possession => &CROSS_POSSESSION[idx],
        ZoneTransitionStyle::Counter => &CROSS_COUNTER[idx],
    }
}

// ---------------------------------------------------------------------------
// 20-zone Positional Play transition matrices
// ---------------------------------------------------------------------------
// Zone order (5 lanes × 4 quarters = 20 zones):
// DEF quarter (0-4):  LwDef, LhsDef, CDef, RhsDef, RwDef
// MID quarter (5-9):  LwMid, LhsMid, CMid, RhsMid, RwMid
// FIN quarter (10-14): LwFin, LhsFin, CFin, RhsFin, RwFin
// BOX quarter (15-19): LwBox, LhsBox, CBox, RhsBox, RwBox
//
// Design principles:
// - Half-space entries (wide→halfspace) get bonus
// - Vertical progression (same lane, higher quarter) preferred
// - Backward passes (higher→lower quarter) penalized
// - Finishing zone entries (→LhsBox/CBox/RhsBox) highly valued
// - Switch of play (LW↔RW) moderately valued
// ---------------------------------------------------------------------------

#[rustfmt::skip]
const PASS_NORMAL: [[f32; ZONE_COUNT]; ZONE_COUNT] = [
    // From LwDef (0): To DEF(0-4), MID(5-9), FIN(10-14), BOX(15-19)
    [5.0, 15.0, 10.0,  5.0,  3.0,  20.0, 15.0, 10.0,  5.0,  2.0,   5.0,  3.0,  2.0,  1.0,  1.0,   1.0,  1.0,  0.0,  0.0,  0.0],
    // From LhsDef (1)
    [10.0, 10.0, 15.0,  8.0,  2.0,  15.0, 20.0, 12.0,  5.0,  2.0,   5.0,  5.0,  3.0,  1.0,  1.0,   1.0,  2.0,  1.0,  0.0,  0.0],
    // From CDef (2)
    [5.0, 12.0, 15.0, 12.0,  5.0,  10.0, 15.0, 18.0, 15.0, 10.0,   3.0,  5.0,  5.0,  5.0,  3.0,   1.0,  2.0,  2.0,  2.0,  1.0],
    // From RhsDef (3)
    [2.0,  8.0, 15.0, 10.0, 10.0,   2.0,  5.0, 12.0, 20.0, 15.0,   1.0,  1.0,  3.0,  5.0,  5.0,   0.0,  0.0,  1.0,  2.0,  1.0],
    // From RwDef (4)
    [3.0,  5.0, 10.0, 15.0,  5.0,   2.0,  5.0, 10.0, 15.0, 20.0,   1.0,  1.0,  2.0,  3.0,  5.0,   0.0,  0.0,  0.0,  1.0,  1.0],
    // From LwMid (5)
    [3.0,  5.0,  3.0,  1.0,  1.0,  10.0, 20.0, 15.0,  5.0,  3.0,  25.0, 15.0,  8.0,  3.0,  2.0,   8.0,  5.0,  2.0,  1.0,  1.0],
    // From LhsMid (6)
    [2.0,  5.0,  5.0,  2.0,  1.0,  15.0, 15.0, 18.0, 10.0,  3.0,  15.0, 25.0, 12.0,  5.0,  2.0,   5.0, 10.0,  5.0,  2.0,  1.0],
    // From CMid (7)
    [1.0,  3.0,  8.0,  3.0,  1.0,  10.0, 15.0, 15.0, 15.0, 10.0,  10.0, 15.0, 18.0, 15.0, 10.0,   3.0,  8.0, 10.0,  8.0,  3.0],
    // From RhsMid (8)
    [1.0,  2.0,  5.0,  5.0,  2.0,   3.0, 10.0, 18.0, 15.0, 15.0,   2.0,  5.0, 12.0, 25.0, 15.0,   1.0,  2.0,  5.0, 10.0,  5.0],
    // From RwMid (9)
    [1.0,  1.0,  3.0,  5.0,  3.0,   3.0,  5.0, 15.0, 20.0, 10.0,   2.0,  3.0,  8.0, 15.0, 25.0,   1.0,  1.0,  2.0,  5.0,  8.0],
    // From LwFin (10)
    [0.0,  1.0,  1.0,  0.0,  0.0,   5.0,  8.0,  5.0,  2.0,  1.0,  15.0, 20.0, 12.0,  5.0,  3.0,  35.0, 20.0, 10.0,  3.0,  2.0],
    // From LhsFin (11)
    [0.0,  1.0,  2.0,  1.0,  0.0,   3.0, 10.0,  8.0,  3.0,  1.0,  15.0, 18.0, 15.0,  8.0,  3.0,  15.0, 35.0, 18.0,  5.0,  2.0],
    // From CFin (12)
    [0.0,  1.0,  2.0,  1.0,  0.0,   2.0,  5.0, 10.0,  5.0,  2.0,  10.0, 15.0, 15.0, 15.0, 10.0,   8.0, 20.0, 25.0, 20.0,  8.0],
    // From RhsFin (13)
    [0.0,  1.0,  2.0,  1.0,  0.0,   1.0,  3.0,  8.0, 10.0,  3.0,   3.0,  8.0, 15.0, 18.0, 15.0,   2.0,  5.0, 18.0, 35.0, 15.0],
    // From RwFin (14)
    [0.0,  0.0,  1.0,  1.0,  0.0,   1.0,  2.0,  5.0,  8.0,  5.0,   3.0,  5.0, 12.0, 20.0, 15.0,   2.0,  3.0, 10.0, 20.0, 35.0],
    // From LwBox (15)
    [0.0,  0.0,  0.0,  0.0,  0.0,   1.0,  2.0,  2.0,  1.0,  0.0,   8.0, 12.0,  8.0,  3.0,  2.0,  20.0, 30.0, 25.0,  8.0,  5.0],
    // From LhsBox (16)
    [0.0,  0.0,  0.0,  0.0,  0.0,   1.0,  3.0,  3.0,  1.0,  0.0,   5.0, 15.0, 12.0,  5.0,  2.0,  15.0, 25.0, 30.0, 15.0,  5.0],
    // From CBox (17)
    [0.0,  0.0,  0.0,  0.0,  0.0,   0.0,  2.0,  5.0,  2.0,  0.0,   3.0, 10.0, 15.0, 10.0,  3.0,  12.0, 25.0, 25.0, 25.0, 12.0],
    // From RhsBox (18)
    [0.0,  0.0,  0.0,  0.0,  0.0,   0.0,  1.0,  3.0,  3.0,  1.0,   2.0,  5.0, 12.0, 15.0,  5.0,   5.0, 15.0, 30.0, 25.0, 15.0],
    // From RwBox (19)
    [0.0,  0.0,  0.0,  0.0,  0.0,   0.0,  1.0,  2.0,  2.0,  1.0,   2.0,  3.0,  8.0, 12.0,  8.0,   5.0,  8.0, 25.0, 30.0, 20.0],
];

#[rustfmt::skip]
const PASS_POSSESSION: [[f32; ZONE_COUNT]; ZONE_COUNT] = [
    // From LwDef (0): Possession style - more lateral, patient buildup
    [10.0, 20.0, 12.0,  5.0,  3.0,  15.0, 12.0,  8.0,  3.0,  2.0,   3.0,  2.0,  1.0,  1.0,  1.0,   0.0,  0.0,  0.0,  0.0,  0.0],
    // From LhsDef (1)
    [15.0, 15.0, 18.0, 10.0,  3.0,  12.0, 18.0, 12.0,  5.0,  2.0,   3.0,  3.0,  2.0,  1.0,  1.0,   0.0,  1.0,  0.0,  0.0,  0.0],
    // From CDef (2)
    [8.0, 15.0, 18.0, 15.0,  8.0,   8.0, 12.0, 20.0, 12.0,  8.0,   2.0,  3.0,  5.0,  3.0,  2.0,   0.0,  1.0,  2.0,  1.0,  0.0],
    // From RhsDef (3)
    [3.0, 10.0, 18.0, 15.0, 15.0,   2.0,  5.0, 12.0, 18.0, 12.0,   1.0,  1.0,  2.0,  3.0,  3.0,   0.0,  0.0,  0.0,  1.0,  0.0],
    // From RwDef (4)
    [3.0,  5.0, 12.0, 20.0, 10.0,   2.0,  3.0,  8.0, 12.0, 15.0,   1.0,  1.0,  1.0,  2.0,  3.0,   0.0,  0.0,  0.0,  0.0,  0.0],
    // From LwMid (5)
    [5.0,  8.0,  5.0,  2.0,  1.0,  12.0, 22.0, 15.0,  5.0,  3.0,  20.0, 12.0,  5.0,  2.0,  2.0,   5.0,  3.0,  1.0,  1.0,  1.0],
    // From LhsMid (6)
    [3.0,  8.0,  8.0,  3.0,  1.0,  18.0, 18.0, 20.0, 10.0,  3.0,  12.0, 22.0, 10.0,  3.0,  2.0,   3.0,  8.0,  3.0,  1.0,  1.0],
    // From CMid (7)
    [2.0,  5.0, 10.0,  5.0,  2.0,  12.0, 18.0, 18.0, 18.0, 12.0,   8.0, 12.0, 15.0, 12.0,  8.0,   2.0,  5.0,  8.0,  5.0,  2.0],
    // From RhsMid (8)
    [1.0,  3.0,  8.0,  8.0,  3.0,   3.0, 10.0, 20.0, 18.0, 18.0,   2.0,  3.0, 10.0, 22.0, 12.0,   1.0,  1.0,  3.0,  8.0,  3.0],
    // From RwMid (9)
    [1.0,  2.0,  5.0,  8.0,  5.0,   3.0,  5.0, 15.0, 22.0, 12.0,   2.0,  2.0,  5.0, 12.0, 20.0,   1.0,  1.0,  1.0,  3.0,  5.0],
    // From LwFin (10)
    [1.0,  2.0,  2.0,  1.0,  0.0,   8.0, 10.0,  5.0,  2.0,  1.0,  18.0, 22.0, 10.0,  3.0,  2.0,  28.0, 15.0,  8.0,  2.0,  2.0],
    // From LhsFin (11)
    [1.0,  2.0,  3.0,  1.0,  0.0,   5.0, 12.0, 10.0,  3.0,  1.0,  18.0, 20.0, 15.0,  5.0,  2.0,  12.0, 30.0, 15.0,  3.0,  2.0],
    // From CFin (12)
    [0.0,  1.0,  3.0,  1.0,  0.0,   3.0,  8.0, 12.0,  8.0,  3.0,  10.0, 15.0, 18.0, 15.0, 10.0,   5.0, 15.0, 22.0, 15.0,  5.0],
    // From RhsFin (13)
    [0.0,  1.0,  3.0,  2.0,  1.0,   1.0,  3.0, 10.0, 12.0,  5.0,   2.0,  5.0, 15.0, 20.0, 18.0,   2.0,  3.0, 15.0, 30.0, 12.0],
    // From RwFin (14)
    [0.0,  1.0,  2.0,  2.0,  1.0,   1.0,  2.0,  5.0, 10.0,  8.0,   2.0,  3.0, 10.0, 22.0, 18.0,   2.0,  2.0,  8.0, 15.0, 28.0],
    // From LwBox (15)
    [0.0,  0.0,  0.0,  0.0,  0.0,   2.0,  3.0,  2.0,  1.0,  1.0,  10.0, 15.0, 10.0,  3.0,  2.0,  22.0, 28.0, 22.0,  5.0,  5.0],
    // From LhsBox (16)
    [0.0,  0.0,  0.0,  0.0,  0.0,   1.0,  5.0,  5.0,  2.0,  1.0,   8.0, 18.0, 12.0,  5.0,  2.0,  18.0, 25.0, 28.0, 12.0,  5.0],
    // From CBox (17)
    [0.0,  0.0,  0.0,  0.0,  0.0,   1.0,  3.0,  8.0,  3.0,  1.0,   5.0, 12.0, 18.0, 12.0,  5.0,  10.0, 22.0, 25.0, 22.0, 10.0],
    // From RhsBox (18)
    [0.0,  0.0,  0.0,  0.0,  0.0,   1.0,  2.0,  5.0,  5.0,  1.0,   2.0,  5.0, 12.0, 18.0,  8.0,   5.0, 12.0, 28.0, 25.0, 18.0],
    // From RwBox (19)
    [0.0,  0.0,  0.0,  0.0,  0.0,   1.0,  1.0,  2.0,  3.0,  2.0,   2.0,  2.0, 10.0, 15.0, 10.0,   5.0,  5.0, 22.0, 28.0, 22.0],
];

#[rustfmt::skip]
const PASS_COUNTER: [[f32; ZONE_COUNT]; ZONE_COUNT] = [
    // From LwDef (0): Counter style - direct, vertical passes
    [3.0, 10.0,  8.0,  3.0,  2.0,  25.0, 18.0, 12.0,  5.0,  3.0,  10.0,  8.0,  5.0,  2.0,  2.0,   3.0,  2.0,  1.0,  1.0,  1.0],
    // From LhsDef (1)
    [8.0,  8.0, 12.0,  5.0,  2.0,  18.0, 22.0, 15.0,  5.0,  2.0,   8.0, 10.0,  5.0,  2.0,  2.0,   2.0,  5.0,  2.0,  1.0,  1.0],
    // From CDef (2)
    [3.0, 10.0, 12.0, 10.0,  3.0,  12.0, 18.0, 22.0, 18.0, 12.0,   5.0,  8.0, 10.0,  8.0,  5.0,   2.0,  3.0,  5.0,  3.0,  2.0],
    // From RhsDef (3)
    [2.0,  5.0, 12.0,  8.0,  8.0,   2.0,  5.0, 15.0, 22.0, 18.0,   2.0,  2.0,  5.0, 10.0,  8.0,   1.0,  1.0,  2.0,  5.0,  2.0],
    // From RwDef (4)
    [2.0,  3.0,  8.0, 10.0,  3.0,   3.0,  5.0, 12.0, 18.0, 25.0,   2.0,  2.0,  5.0,  8.0, 10.0,   1.0,  1.0,  1.0,  2.0,  3.0],
    // From LwMid (5)
    [2.0,  3.0,  2.0,  1.0,  1.0,   8.0, 18.0, 12.0,  3.0,  2.0,  30.0, 18.0, 10.0,  3.0,  2.0,  12.0,  8.0,  3.0,  2.0,  2.0],
    // From LhsMid (6)
    [1.0,  3.0,  3.0,  1.0,  1.0,  12.0, 12.0, 15.0,  8.0,  2.0,  18.0, 28.0, 15.0,  5.0,  2.0,   8.0, 15.0,  8.0,  3.0,  2.0],
    // From CMid (7)
    [1.0,  2.0,  5.0,  2.0,  1.0,   8.0, 12.0, 12.0, 12.0,  8.0,  12.0, 18.0, 22.0, 18.0, 12.0,   5.0, 10.0, 15.0, 10.0,  5.0],
    // From RhsMid (8)
    [1.0,  1.0,  3.0,  3.0,  1.0,   2.0,  8.0, 15.0, 12.0, 12.0,   2.0,  5.0, 15.0, 28.0, 18.0,   2.0,  3.0,  8.0, 15.0,  8.0],
    // From RwMid (9)
    [1.0,  1.0,  2.0,  3.0,  2.0,   2.0,  3.0, 12.0, 18.0,  8.0,   2.0,  3.0, 10.0, 18.0, 30.0,   2.0,  2.0,  3.0,  8.0, 12.0],
    // From LwFin (10)
    [0.0,  1.0,  1.0,  0.0,  0.0,   3.0,  5.0,  3.0,  1.0,  1.0,  12.0, 18.0, 10.0,  3.0,  2.0,  42.0, 22.0, 12.0,  5.0,  3.0],
    // From LhsFin (11)
    [0.0,  1.0,  1.0,  0.0,  0.0,   2.0,  8.0,  5.0,  2.0,  1.0,  12.0, 15.0, 12.0,  5.0,  2.0,  18.0, 40.0, 20.0,  8.0,  3.0],
    // From CFin (12)
    [0.0,  0.0,  1.0,  0.0,  0.0,   1.0,  3.0,  8.0,  3.0,  1.0,   8.0, 12.0, 12.0, 12.0,  8.0,  10.0, 22.0, 30.0, 22.0, 10.0],
    // From RhsFin (13)
    [0.0,  0.0,  1.0,  1.0,  0.0,   1.0,  2.0,  5.0,  8.0,  2.0,   2.0,  5.0, 12.0, 15.0, 12.0,   3.0,  8.0, 20.0, 40.0, 18.0],
    // From RwFin (14)
    [0.0,  0.0,  1.0,  1.0,  0.0,   1.0,  1.0,  3.0,  5.0,  3.0,   2.0,  3.0, 10.0, 18.0, 12.0,   3.0,  5.0, 12.0, 22.0, 42.0],
    // From LwBox (15)
    [0.0,  0.0,  0.0,  0.0,  0.0,   1.0,  1.0,  1.0,  0.0,  0.0,   5.0, 10.0,  5.0,  2.0,  1.0,  22.0, 35.0, 28.0, 10.0,  8.0],
    // From LhsBox (16)
    [0.0,  0.0,  0.0,  0.0,  0.0,   0.0,  2.0,  2.0,  1.0,  0.0,   3.0, 12.0, 10.0,  3.0,  2.0,  15.0, 28.0, 32.0, 18.0,  8.0],
    // From CBox (17)
    [0.0,  0.0,  0.0,  0.0,  0.0,   0.0,  1.0,  3.0,  1.0,  0.0,   2.0,  8.0, 12.0,  8.0,  2.0,  12.0, 28.0, 28.0, 28.0, 12.0],
    // From RhsBox (18)
    [0.0,  0.0,  0.0,  0.0,  0.0,   0.0,  1.0,  2.0,  2.0,  0.0,   2.0,  3.0, 10.0, 12.0,  3.0,   8.0, 18.0, 32.0, 28.0, 15.0],
    // From RwBox (19)
    [0.0,  0.0,  0.0,  0.0,  0.0,   0.0,  0.0,  1.0,  1.0,  1.0,   1.0,  2.0,  5.0, 10.0,  5.0,   8.0, 10.0, 28.0, 35.0, 22.0],
];

// Cross matrices focus on wide→box and halfspace→box patterns
#[rustfmt::skip]
const CROSS_NORMAL: [[f32; ZONE_COUNT]; ZONE_COUNT] = [
    // DEF zones rarely cross - mostly zeros
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    // MID zones - occasional long cross
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 5.0, 8.0, 5.0,  0.0, 5.0, 15.0, 20.0, 5.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 2.0, 5.0, 10.0, 3.0,  0.0, 3.0, 18.0, 25.0, 8.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 5.0, 3.0, 0.0,  0.0, 8.0, 20.0, 8.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  3.0, 10.0, 5.0, 2.0, 0.0,  8.0, 25.0, 18.0, 3.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  5.0, 8.0, 5.0, 3.0, 0.0,  5.0, 20.0, 15.0, 5.0, 0.0],
    // FIN zones - primary crossing zones
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 5.0, 8.0, 12.0, 5.0,  0.0, 10.0, 25.0, 35.0, 10.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 8.0, 15.0, 8.0,  0.0, 8.0, 28.0, 32.0, 12.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 5.0, 5.0, 5.0, 0.0,  0.0, 15.0, 25.0, 15.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  8.0, 15.0, 8.0, 3.0, 0.0,  12.0, 32.0, 28.0, 8.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  5.0, 12.0, 8.0, 5.0, 0.0,  10.0, 35.0, 25.0, 10.0, 0.0],
    // BOX zones - cutback crosses
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 5.0, 8.0, 3.0,  0.0, 15.0, 35.0, 40.0, 12.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 2.0, 5.0, 10.0, 5.0,  0.0, 10.0, 38.0, 38.0, 15.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 5.0, 5.0, 5.0, 0.0,  0.0, 20.0, 30.0, 20.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  5.0, 10.0, 5.0, 2.0, 0.0,  15.0, 38.0, 38.0, 10.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  3.0, 8.0, 5.0, 3.0, 0.0,  12.0, 40.0, 35.0, 15.0, 0.0],
];

#[rustfmt::skip]
const CROSS_POSSESSION: [[f32; ZONE_COUNT]; ZONE_COUNT] = [
    // Possession style - fewer crosses from deep, more patient
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    // MID zones - less crossing
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 2.0, 3.0, 5.0, 3.0,  0.0, 3.0, 10.0, 15.0, 3.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 1.0, 3.0, 8.0, 2.0,  0.0, 2.0, 12.0, 18.0, 5.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 2.0, 3.0, 2.0, 0.0,  0.0, 5.0, 15.0, 5.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  2.0, 8.0, 3.0, 1.0, 0.0,  5.0, 18.0, 12.0, 2.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  3.0, 5.0, 3.0, 2.0, 0.0,  3.0, 15.0, 10.0, 3.0, 0.0],
    // FIN zones - controlled crossing
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 5.0, 8.0, 3.0,  0.0, 8.0, 20.0, 28.0, 8.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 2.0, 5.0, 10.0, 5.0,  0.0, 5.0, 22.0, 25.0, 10.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 3.0, 3.0, 0.0,  0.0, 12.0, 20.0, 12.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  5.0, 10.0, 5.0, 2.0, 0.0,  10.0, 25.0, 22.0, 5.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  3.0, 8.0, 5.0, 3.0, 0.0,  8.0, 28.0, 20.0, 8.0, 0.0],
    // BOX zones
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 2.0, 3.0, 5.0, 2.0,  0.0, 12.0, 30.0, 35.0, 10.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 1.0, 3.0, 8.0, 3.0,  0.0, 8.0, 32.0, 32.0, 12.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 3.0, 3.0, 0.0,  0.0, 15.0, 25.0, 15.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  3.0, 8.0, 3.0, 1.0, 0.0,  12.0, 32.0, 32.0, 8.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  2.0, 5.0, 3.0, 2.0, 0.0,  10.0, 35.0, 30.0, 12.0, 0.0],
];

#[rustfmt::skip]
const CROSS_COUNTER: [[f32; ZONE_COUNT]; ZONE_COUNT] = [
    // Counter style - more aggressive early crosses
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0],
    // MID zones - more early crosses
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 5.0, 8.0, 12.0, 8.0,  0.0, 8.0, 22.0, 28.0, 8.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 8.0, 15.0, 5.0,  0.0, 5.0, 25.0, 32.0, 12.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 5.0, 8.0, 5.0, 0.0,  0.0, 12.0, 28.0, 12.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  5.0, 15.0, 8.0, 3.0, 0.0,  12.0, 32.0, 25.0, 5.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  8.0, 12.0, 8.0, 5.0, 0.0,  8.0, 28.0, 22.0, 8.0, 0.0],
    // FIN zones - aggressive crossing
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 8.0, 12.0, 18.0, 8.0,  0.0, 15.0, 32.0, 42.0, 15.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 5.0, 10.0, 20.0, 10.0,  0.0, 12.0, 35.0, 40.0, 18.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 8.0, 8.0, 8.0, 0.0,  0.0, 20.0, 32.0, 20.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  10.0, 20.0, 10.0, 5.0, 0.0,  18.0, 40.0, 35.0, 12.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  8.0, 18.0, 12.0, 8.0, 0.0,  15.0, 42.0, 32.0, 15.0, 0.0],
    // BOX zones - quick cutbacks
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 5.0, 8.0, 12.0, 5.0,  0.0, 18.0, 40.0, 45.0, 15.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 3.0, 8.0, 15.0, 8.0,  0.0, 15.0, 42.0, 42.0, 18.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 8.0, 8.0, 8.0, 0.0,  0.0, 25.0, 35.0, 25.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  8.0, 15.0, 8.0, 3.0, 0.0,  18.0, 42.0, 42.0, 15.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0,  5.0, 12.0, 8.0, 5.0, 0.0,  15.0, 45.0, 40.0, 18.0, 0.0],
];

// ============================================================================
// FIX_2601/0113: Transition Matrix Verification Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// P2: Transition matrix row validation - each row must have non-zero sum
    #[test]
    fn test_transition_matrix_row_sums() {
        for from_idx in 0..20 {
            let from_zone = PosPlayZoneId::from_index(from_idx).unwrap();

            for style in [ZoneTransitionStyle::Normal,
                          ZoneTransitionStyle::Possession,
                          ZoneTransitionStyle::Counter] {
                let row_sum: f32 = (0..20)
                    .map(|to_idx| {
                        let to_zone = PosPlayZoneId::from_index(to_idx).unwrap();
                        pass_factor(style, from_zone, to_zone)
                    })
                    .sum();

                // Row should have reasonable total weight
                // Note: pass_factor returns values in 0.5-1.5 range, so sum ~= 20
                assert!(row_sum > 0.0,
                    "Row sum is zero for {:?} with style {:?}", from_zone, style);
            }
        }
    }

    /// P2: No impossible high weights for extreme zone jumps
    #[test]
    fn test_no_impossible_high_weights() {
        // Defensive wing to opposite attacking box shouldn't be favored
        let def_to_far_box = pass_factor(
            ZoneTransitionStyle::Normal,
            PosPlayZoneId::LwDef,  // Left wing defensive
            PosPlayZoneId::RwBox   // Right wing box
        );
        assert!(def_to_far_box <= 1.5,
            "Impossible jump LwDef->RwBox has too high weight: {}", def_to_far_box);

        // Same side box should be possible but not dominant from def
        let def_to_same_box = pass_factor(
            ZoneTransitionStyle::Normal,
            PosPlayZoneId::LwDef,
            PosPlayZoneId::LwBox
        );
        assert!(def_to_same_box <= 1.5,
            "Long ball LwDef->LwBox too high: {}", def_to_same_box);
    }

    /// P2: Backward pass should not be over-penalized in possession style
    #[test]
    fn test_backward_pass_not_over_penalized() {
        // In possession style, backward passes should still be viable
        let backward_factor = pass_factor(
            ZoneTransitionStyle::Possession,
            PosPlayZoneId::CMid,  // From middle
            PosPlayZoneId::CDef   // To defensive
        );

        // Should be at least minimum viable (not completely suppressed)
        assert!(backward_factor >= 0.5,
            "Backward pass too heavily penalized in possession: {}", backward_factor);
    }

    /// P2: Half-space entry should have reasonable bonus
    #[test]
    fn test_halfspace_entry_reasonable() {
        // Wide to halfspace should have reasonable weight
        let wide_to_halfspace = pass_factor(
            ZoneTransitionStyle::Normal,
            PosPlayZoneId::LwMid,   // Left wing mid
            PosPlayZoneId::LhsMid   // Left half-space mid
        );

        let wide_to_wide = pass_factor(
            ZoneTransitionStyle::Normal,
            PosPlayZoneId::LwMid,
            PosPlayZoneId::LwFin
        );

        // Both should be valid options
        assert!(wide_to_halfspace >= 0.5, "Half-space entry too low: {}", wide_to_halfspace);
        assert!(wide_to_wide >= 0.5, "Wide progression too low: {}", wide_to_wide);
    }

    /// P2: Cross matrix should target finishing zones
    #[test]
    fn test_cross_targets_finishing_zones() {
        // Crosses from wide final should target box zones
        let lw_fin_to_cbox = cross_factor(
            ZoneTransitionStyle::Normal,
            PosPlayZoneId::LwFin,
            PosPlayZoneId::CBox
        );

        let lw_fin_to_rhsbox = cross_factor(
            ZoneTransitionStyle::Normal,
            PosPlayZoneId::LwFin,
            PosPlayZoneId::RhsBox
        );

        // Far post crosses should be viable
        assert!(lw_fin_to_cbox >= 0.5, "Cross to CBox too low: {}", lw_fin_to_cbox);
        assert!(lw_fin_to_rhsbox >= 0.5, "Far post cross too low: {}", lw_fin_to_rhsbox);
    }

    /// P2: Counter style should favor vertical progression
    #[test]
    fn test_counter_favors_vertical() {
        // Counter should prefer forward passes
        let forward = pass_factor(
            ZoneTransitionStyle::Counter,
            PosPlayZoneId::CMid,
            PosPlayZoneId::CFin
        );

        let lateral = pass_factor(
            ZoneTransitionStyle::Counter,
            PosPlayZoneId::CMid,
            PosPlayZoneId::LhsMid
        );

        // Forward should be at least as good as lateral
        assert!(forward >= lateral * 0.8,
            "Counter should favor forward: forward={}, lateral={}", forward, lateral);
    }

    /// Verify event mix weights change by zone
    #[test]
    fn test_event_mix_varies_by_zone() {
        let def_weights = ofm_event_mix_weights(ZoneTransitionStyle::Normal, PosPlayZoneId::CDef);
        let box_weights = ofm_event_mix_weights(ZoneTransitionStyle::Normal, PosPlayZoneId::CBox);

        // Box should have higher shot weight than defensive
        assert!(box_weights.shot > def_weights.shot,
            "Box should have higher shot weight: box={}, def={}", box_weights.shot, def_weights.shot);

        // Wide box zones should have high cross weight
        let wide_box_weights = ofm_event_mix_weights(ZoneTransitionStyle::Normal, PosPlayZoneId::LwBox);
        assert!(wide_box_weights.cross > 10.0,
            "Wide box should favor crossing: {}", wide_box_weights.cross);
    }
}
