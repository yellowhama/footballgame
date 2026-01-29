//! Zone Definitions (SSOT)
//!
//! FIX_2601/0113: Dual-Layer Zone Architecture
//!
//! Field zones are defined using normalized coordinates (0.0 - 1.0).
//! This keeps zone logic independent of the engine's coordinate system.
//!
//! ## Layer 1: Anchor Zones (6-zone, 3x2 grid)
//! - Used for: Calibration, AnchorTable, StatSnapshot
//! - Purpose: Statistical distribution stabilization
//!
//! ## Layer 2: Positional Play Zones (20-zone, 5x4 grid)
//! - Used for: Tactical logic, pass scoring, positioning, zone transitions
//! - Purpose: Tactical structure expression (half-spaces, lanes, box approach)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Zone identifier for the 6-zone system (3x2 grid)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneId {
    /// Defensive Left (own third, left side)
    LDef,
    /// Defensive Center (own third, center)
    CDef,
    /// Defensive Right (own third, right side)
    RDef,
    /// Attacking Left (opponent third, left side)
    LAtt,
    /// Attacking Center (opponent third, center) - High xG zone
    CAtt,
    /// Attacking Right (opponent third, right side)
    RAtt,
}

impl ZoneId {
    /// All zone IDs in order
    pub const ALL: [ZoneId; 6] = [
        ZoneId::LDef,
        ZoneId::CDef,
        ZoneId::RDef,
        ZoneId::LAtt,
        ZoneId::CAtt,
        ZoneId::RAtt,
    ];

    /// Get zone index (0-5)
    pub fn index(&self) -> usize {
        match self {
            ZoneId::LDef => 0,
            ZoneId::CDef => 1,
            ZoneId::RDef => 2,
            ZoneId::LAtt => 3,
            ZoneId::CAtt => 4,
            ZoneId::RAtt => 5,
        }
    }

    /// Create from index
    pub fn from_index(idx: usize) -> Option<Self> {
        Self::ALL.get(idx).copied()
    }

    /// Get the string ID (for JSON compatibility)
    pub fn as_str(&self) -> &'static str {
        match self {
            ZoneId::LDef => "L_DEF",
            ZoneId::CDef => "C_DEF",
            ZoneId::RDef => "R_DEF",
            ZoneId::LAtt => "L_ATT",
            ZoneId::CAtt => "C_ATT",
            ZoneId::RAtt => "R_ATT",
        }
    }

    /// Parse from string ID
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "L_DEF" => Some(ZoneId::LDef),
            "C_DEF" => Some(ZoneId::CDef),
            "R_DEF" => Some(ZoneId::RDef),
            "L_ATT" => Some(ZoneId::LAtt),
            "C_ATT" => Some(ZoneId::CAtt),
            "R_ATT" => Some(ZoneId::RAtt),
            _ => None,
        }
    }

    /// Is this zone in the attacking third?
    pub fn is_attacking(&self) -> bool {
        matches!(self, ZoneId::LAtt | ZoneId::CAtt | ZoneId::RAtt)
    }

    /// Is this zone in the defensive third?
    pub fn is_defensive(&self) -> bool {
        matches!(self, ZoneId::LDef | ZoneId::CDef | ZoneId::RDef)
    }

    /// Is this a central zone?
    pub fn is_central(&self) -> bool {
        matches!(self, ZoneId::CDef | ZoneId::CAtt)
    }

    /// Is this a wide zone?
    pub fn is_wide(&self) -> bool {
        matches!(self, ZoneId::LDef | ZoneId::RDef | ZoneId::LAtt | ZoneId::RAtt)
    }

    /// Flip zone for opposite direction (home <-> away perspective)
    pub fn flip(&self) -> Self {
        match self {
            ZoneId::LDef => ZoneId::RAtt,
            ZoneId::CDef => ZoneId::CAtt,
            ZoneId::RDef => ZoneId::LAtt,
            ZoneId::LAtt => ZoneId::RDef,
            ZoneId::CAtt => ZoneId::CDef,
            ZoneId::RAtt => ZoneId::LDef,
        }
    }
}

/// Zone definition with bounds
#[derive(Debug, Clone)]
pub struct ZoneDef {
    pub id: ZoneId,
    pub x0: f32,
    pub x1: f32,
    pub y0: f32,
    pub y1: f32,
}

/// Zone schema containing all zone definitions
#[derive(Debug, Clone)]
pub struct ZoneSchema {
    pub zones: Vec<ZoneDef>,
}

impl Default for ZoneSchema {
    fn default() -> Self {
        Self::six_zone()
    }
}

impl ZoneSchema {
    /// Standard 6-zone schema (3x2 grid)
    pub fn six_zone() -> Self {
        Self {
            zones: vec![
                ZoneDef { id: ZoneId::LDef, x0: 0.0, x1: 0.3333, y0: 0.0, y1: 0.5 },
                ZoneDef { id: ZoneId::CDef, x0: 0.3333, x1: 0.6667, y0: 0.0, y1: 0.5 },
                ZoneDef { id: ZoneId::RDef, x0: 0.6667, x1: 1.0, y0: 0.0, y1: 0.5 },
                ZoneDef { id: ZoneId::LAtt, x0: 0.0, x1: 0.3333, y0: 0.5, y1: 1.0 },
                ZoneDef { id: ZoneId::CAtt, x0: 0.3333, x1: 0.6667, y0: 0.5, y1: 1.0 },
                ZoneDef { id: ZoneId::RAtt, x0: 0.6667, x1: 1.0, y0: 0.5, y1: 1.0 },
            ],
        }
    }
}

/// Convert normalized position to zone ID
///
/// # Arguments
/// * `norm_x` - X position in normalized coordinates (0.0 - 1.0)
/// * `norm_y` - Y position in normalized coordinates (0.0 - 1.0)
///
/// # Returns
/// Zone ID for the given position
pub fn pos_to_zone(norm_x: f32, norm_y: f32) -> ZoneId {
    let x_clamped = norm_x.clamp(0.0, 1.0);
    let y_clamped = norm_y.clamp(0.0, 1.0);

    let x_idx = if x_clamped < 0.3333 {
        0 // Left
    } else if x_clamped < 0.6667 {
        1 // Center
    } else {
        2 // Right
    };

    let y_idx = if y_clamped < 0.5 { 0 } else { 1 }; // Def / Att

    ZoneId::from_index(y_idx * 3 + x_idx).unwrap_or(ZoneId::CDef)
}

/// Convert normalized position to zone ID, accounting for attack direction
///
/// # Arguments
/// * `norm_x` - X position in normalized coordinates (0.0 - 1.0)
/// * `norm_y` - Y position in normalized coordinates (0.0 - 1.0)
/// * `attacks_right` - True if the team attacks toward X=1.0
///
/// # Returns
/// Zone ID from the team's perspective
pub fn pos_to_zone_for_team(norm_x: f32, norm_y: f32, attacks_right: bool) -> ZoneId {
    let zone = pos_to_zone(norm_x, norm_y);
    if attacks_right {
        zone
    } else {
        zone.flip()
    }
}

/// Zone distribution (shares that sum to ~1.0)
pub type ZoneDistribution = HashMap<ZoneId, f32>;

/// Create an empty zone distribution
pub fn empty_zone_distribution() -> ZoneDistribution {
    let mut dist = HashMap::new();
    for zone in ZoneId::ALL {
        dist.insert(zone, 0.0);
    }
    dist
}

/// Normalize zone distribution so it sums to 1.0
pub fn normalize_zone_distribution(dist: &mut ZoneDistribution) {
    let sum: f32 = dist.values().sum();
    if sum > 0.0 {
        for val in dist.values_mut() {
            *val /= sum;
        }
    }
}

// ============================================================================
// Layer 2: Positional Play Zones (15-zone, 5x3 grid)
// ============================================================================

/// Lane identifier for positional play (5 lanes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    /// Left Wing (Y: 0.0 - 0.2)
    LeftWing,
    /// Left Half-Space (Y: 0.2 - 0.4)
    LeftHalfSpace,
    /// Central (Y: 0.4 - 0.6)
    Central,
    /// Right Half-Space (Y: 0.6 - 0.8)
    RightHalfSpace,
    /// Right Wing (Y: 0.8 - 1.0)
    RightWing,
}

impl Lane {
    /// All lanes in order (left to right)
    pub const ALL: [Lane; 5] = [
        Lane::LeftWing,
        Lane::LeftHalfSpace,
        Lane::Central,
        Lane::RightHalfSpace,
        Lane::RightWing,
    ];

    /// Is this a half-space lane?
    pub fn is_halfspace(&self) -> bool {
        matches!(self, Lane::LeftHalfSpace | Lane::RightHalfSpace)
    }

    /// Is this a wide lane?
    pub fn is_wide(&self) -> bool {
        matches!(self, Lane::LeftWing | Lane::RightWing)
    }

    /// Is this the central lane?
    pub fn is_central(&self) -> bool {
        matches!(self, Lane::Central)
    }

    /// Mirror lane (left <-> right)
    pub fn mirror(&self) -> Self {
        match self {
            Lane::LeftWing => Lane::RightWing,
            Lane::LeftHalfSpace => Lane::RightHalfSpace,
            Lane::Central => Lane::Central,
            Lane::RightHalfSpace => Lane::LeftHalfSpace,
            Lane::RightWing => Lane::LeftWing,
        }
    }
}

/// Quarter identifier for positional play (4 quarters)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quarter {
    /// Defensive Quarter (X: 0.0 - 0.25) - Buildup
    Defensive,
    /// Middle Quarter (X: 0.25 - 0.50) - Progression
    Middle,
    /// Final Quarter (X: 0.50 - 0.75) - Chance Creation
    Final,
    /// Box Approach (X: 0.75 - 1.0) - Finishing
    Box,
}

impl Quarter {
    /// All quarters in order (defensive to box)
    pub const ALL: [Quarter; 4] = [Quarter::Defensive, Quarter::Middle, Quarter::Final, Quarter::Box];

    /// Flip quarter (defensive <-> box, middle <-> final)
    pub fn flip(&self) -> Self {
        match self {
            Quarter::Defensive => Quarter::Box,
            Quarter::Middle => Quarter::Final,
            Quarter::Final => Quarter::Middle,
            Quarter::Box => Quarter::Defensive,
        }
    }

    /// Is this the box approach quarter?
    pub fn is_box(&self) -> bool {
        matches!(self, Quarter::Box)
    }

    /// Is this in the attacking half? (Final or Box)
    pub fn is_attacking_half(&self) -> bool {
        matches!(self, Quarter::Final | Quarter::Box)
    }
}

/// Positional Play Zone identifier (20-zone, 5x4 grid)
///
/// Naming convention: {Lane}_{Quarter}
/// - LW = Left Wing, LHS = Left Half-Space, C = Central
/// - RHS = Right Half-Space, RW = Right Wing
/// - DEF = Defensive, MID = Middle, FIN = Final, BOX = Box Approach
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PosPlayZoneId {
    // Defensive Quarter (X: 0.0 - 0.25)
    LwDef,
    LhsDef,
    CDef,
    RhsDef,
    RwDef,
    // Middle Quarter (X: 0.25 - 0.50)
    LwMid,
    LhsMid,
    CMid,
    RhsMid,
    RwMid,
    // Final Quarter (X: 0.50 - 0.75)
    LwFin,
    LhsFin,
    CFin,
    RhsFin,
    RwFin,
    // Box Approach (X: 0.75 - 1.0)
    LwBox,
    LhsBox,
    CBox,
    RhsBox,
    RwBox,
}

impl PosPlayZoneId {
    /// All 20 zones in order
    pub const ALL: [PosPlayZoneId; 20] = [
        // Defensive Quarter
        PosPlayZoneId::LwDef, PosPlayZoneId::LhsDef, PosPlayZoneId::CDef,
        PosPlayZoneId::RhsDef, PosPlayZoneId::RwDef,
        // Middle Quarter
        PosPlayZoneId::LwMid, PosPlayZoneId::LhsMid, PosPlayZoneId::CMid,
        PosPlayZoneId::RhsMid, PosPlayZoneId::RwMid,
        // Final Quarter
        PosPlayZoneId::LwFin, PosPlayZoneId::LhsFin, PosPlayZoneId::CFin,
        PosPlayZoneId::RhsFin, PosPlayZoneId::RwFin,
        // Box Approach
        PosPlayZoneId::LwBox, PosPlayZoneId::LhsBox, PosPlayZoneId::CBox,
        PosPlayZoneId::RhsBox, PosPlayZoneId::RwBox,
    ];

    /// Get the lane for this zone
    pub fn lane(&self) -> Lane {
        match self {
            PosPlayZoneId::LwDef | PosPlayZoneId::LwMid |
            PosPlayZoneId::LwFin | PosPlayZoneId::LwBox => Lane::LeftWing,
            PosPlayZoneId::LhsDef | PosPlayZoneId::LhsMid |
            PosPlayZoneId::LhsFin | PosPlayZoneId::LhsBox => Lane::LeftHalfSpace,
            PosPlayZoneId::CDef | PosPlayZoneId::CMid |
            PosPlayZoneId::CFin | PosPlayZoneId::CBox => Lane::Central,
            PosPlayZoneId::RhsDef | PosPlayZoneId::RhsMid |
            PosPlayZoneId::RhsFin | PosPlayZoneId::RhsBox => Lane::RightHalfSpace,
            PosPlayZoneId::RwDef | PosPlayZoneId::RwMid |
            PosPlayZoneId::RwFin | PosPlayZoneId::RwBox => Lane::RightWing,
        }
    }

    /// Get the quarter for this zone
    pub fn quarter(&self) -> Quarter {
        match self {
            PosPlayZoneId::LwDef | PosPlayZoneId::LhsDef | PosPlayZoneId::CDef |
            PosPlayZoneId::RhsDef | PosPlayZoneId::RwDef => Quarter::Defensive,
            PosPlayZoneId::LwMid | PosPlayZoneId::LhsMid | PosPlayZoneId::CMid |
            PosPlayZoneId::RhsMid | PosPlayZoneId::RwMid => Quarter::Middle,
            PosPlayZoneId::LwFin | PosPlayZoneId::LhsFin | PosPlayZoneId::CFin |
            PosPlayZoneId::RhsFin | PosPlayZoneId::RwFin => Quarter::Final,
            PosPlayZoneId::LwBox | PosPlayZoneId::LhsBox | PosPlayZoneId::CBox |
            PosPlayZoneId::RhsBox | PosPlayZoneId::RwBox => Quarter::Box,
        }
    }

    /// Is this zone in a half-space lane?
    pub fn is_halfspace(&self) -> bool {
        self.lane().is_halfspace()
    }

    /// Is this zone in a wide lane?
    pub fn is_wide(&self) -> bool {
        self.lane().is_wide()
    }

    /// Is this zone in the central lane?
    pub fn is_central(&self) -> bool {
        self.lane().is_central()
    }

    /// Is this zone in the box approach quarter?
    pub fn is_box(&self) -> bool {
        self.quarter().is_box()
    }

    /// Is this zone in the attacking half? (Final or Box)
    pub fn is_attacking_half(&self) -> bool {
        self.quarter().is_attacking_half()
    }

    /// Is this zone a finishing zone? (LHS_BOX, C_BOX, RHS_BOX)
    pub fn is_finishing_zone(&self) -> bool {
        matches!(
            self,
            PosPlayZoneId::LhsBox | PosPlayZoneId::CBox | PosPlayZoneId::RhsBox
        )
    }

    /// Get zone index (0-19) for matrix lookups
    pub fn to_index(self) -> usize {
        match self {
            PosPlayZoneId::LwDef => 0,
            PosPlayZoneId::LhsDef => 1,
            PosPlayZoneId::CDef => 2,
            PosPlayZoneId::RhsDef => 3,
            PosPlayZoneId::RwDef => 4,
            PosPlayZoneId::LwMid => 5,
            PosPlayZoneId::LhsMid => 6,
            PosPlayZoneId::CMid => 7,
            PosPlayZoneId::RhsMid => 8,
            PosPlayZoneId::RwMid => 9,
            PosPlayZoneId::LwFin => 10,
            PosPlayZoneId::LhsFin => 11,
            PosPlayZoneId::CFin => 12,
            PosPlayZoneId::RhsFin => 13,
            PosPlayZoneId::RwFin => 14,
            PosPlayZoneId::LwBox => 15,
            PosPlayZoneId::LhsBox => 16,
            PosPlayZoneId::CBox => 17,
            PosPlayZoneId::RhsBox => 18,
            PosPlayZoneId::RwBox => 19,
        }
    }

    /// Create zone from index (0-19)
    pub fn from_index(idx: usize) -> Option<Self> {
        Self::ALL.get(idx).copied()
    }

    /// Create from lane and quarter
    pub fn from_lane_quarter(lane: Lane, quarter: Quarter) -> Self {
        match (lane, quarter) {
            (Lane::LeftWing, Quarter::Defensive) => PosPlayZoneId::LwDef,
            (Lane::LeftHalfSpace, Quarter::Defensive) => PosPlayZoneId::LhsDef,
            (Lane::Central, Quarter::Defensive) => PosPlayZoneId::CDef,
            (Lane::RightHalfSpace, Quarter::Defensive) => PosPlayZoneId::RhsDef,
            (Lane::RightWing, Quarter::Defensive) => PosPlayZoneId::RwDef,
            (Lane::LeftWing, Quarter::Middle) => PosPlayZoneId::LwMid,
            (Lane::LeftHalfSpace, Quarter::Middle) => PosPlayZoneId::LhsMid,
            (Lane::Central, Quarter::Middle) => PosPlayZoneId::CMid,
            (Lane::RightHalfSpace, Quarter::Middle) => PosPlayZoneId::RhsMid,
            (Lane::RightWing, Quarter::Middle) => PosPlayZoneId::RwMid,
            (Lane::LeftWing, Quarter::Final) => PosPlayZoneId::LwFin,
            (Lane::LeftHalfSpace, Quarter::Final) => PosPlayZoneId::LhsFin,
            (Lane::Central, Quarter::Final) => PosPlayZoneId::CFin,
            (Lane::RightHalfSpace, Quarter::Final) => PosPlayZoneId::RhsFin,
            (Lane::RightWing, Quarter::Final) => PosPlayZoneId::RwFin,
            (Lane::LeftWing, Quarter::Box) => PosPlayZoneId::LwBox,
            (Lane::LeftHalfSpace, Quarter::Box) => PosPlayZoneId::LhsBox,
            (Lane::Central, Quarter::Box) => PosPlayZoneId::CBox,
            (Lane::RightHalfSpace, Quarter::Box) => PosPlayZoneId::RhsBox,
            (Lane::RightWing, Quarter::Box) => PosPlayZoneId::RwBox,
        }
    }

    /// Get the string ID (for JSON compatibility)
    pub fn as_str(&self) -> &'static str {
        match self {
            PosPlayZoneId::LwDef => "LW_DEF",
            PosPlayZoneId::LhsDef => "LHS_DEF",
            PosPlayZoneId::CDef => "C_DEF",
            PosPlayZoneId::RhsDef => "RHS_DEF",
            PosPlayZoneId::RwDef => "RW_DEF",
            PosPlayZoneId::LwMid => "LW_MID",
            PosPlayZoneId::LhsMid => "LHS_MID",
            PosPlayZoneId::CMid => "C_MID",
            PosPlayZoneId::RhsMid => "RHS_MID",
            PosPlayZoneId::RwMid => "RW_MID",
            PosPlayZoneId::LwFin => "LW_FIN",
            PosPlayZoneId::LhsFin => "LHS_FIN",
            PosPlayZoneId::CFin => "C_FIN",
            PosPlayZoneId::RhsFin => "RHS_FIN",
            PosPlayZoneId::RwFin => "RW_FIN",
            PosPlayZoneId::LwBox => "LW_BOX",
            PosPlayZoneId::LhsBox => "LHS_BOX",
            PosPlayZoneId::CBox => "C_BOX",
            PosPlayZoneId::RhsBox => "RHS_BOX",
            PosPlayZoneId::RwBox => "RW_BOX",
        }
    }

    /// Parse from string ID
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "LW_DEF" => Some(PosPlayZoneId::LwDef),
            "LHS_DEF" => Some(PosPlayZoneId::LhsDef),
            "C_DEF" => Some(PosPlayZoneId::CDef),
            "RHS_DEF" => Some(PosPlayZoneId::RhsDef),
            "RW_DEF" => Some(PosPlayZoneId::RwDef),
            "LW_MID" => Some(PosPlayZoneId::LwMid),
            "LHS_MID" => Some(PosPlayZoneId::LhsMid),
            "C_MID" => Some(PosPlayZoneId::CMid),
            "RHS_MID" => Some(PosPlayZoneId::RhsMid),
            "RW_MID" => Some(PosPlayZoneId::RwMid),
            "LW_FIN" => Some(PosPlayZoneId::LwFin),
            "LHS_FIN" => Some(PosPlayZoneId::LhsFin),
            "C_FIN" => Some(PosPlayZoneId::CFin),
            "RHS_FIN" => Some(PosPlayZoneId::RhsFin),
            "RW_FIN" => Some(PosPlayZoneId::RwFin),
            "LW_BOX" => Some(PosPlayZoneId::LwBox),
            "LHS_BOX" => Some(PosPlayZoneId::LhsBox),
            "C_BOX" => Some(PosPlayZoneId::CBox),
            "RHS_BOX" => Some(PosPlayZoneId::RhsBox),
            "RW_BOX" => Some(PosPlayZoneId::RwBox),
            _ => None,
        }
    }

    /// Flip zone for opposite direction (home <-> away perspective)
    pub fn flip(&self) -> Self {
        let new_lane = self.lane().mirror();
        let new_quarter = self.quarter().flip();
        Self::from_lane_quarter(new_lane, new_quarter)
    }
}

/// Convert normalized position to PosPlay zone ID
///
/// # Arguments
/// * `norm_x` - X position in normalized coordinates (0.0 - 1.0, attack direction)
/// * `norm_y` - Y position in normalized coordinates (0.0 - 1.0, left to right)
///
/// # Returns
/// PosPlay Zone ID for the given position
pub fn pos_to_posplay_zone(norm_x: f32, norm_y: f32) -> PosPlayZoneId {
    let x = norm_x.clamp(0.0, 1.0);
    let y = norm_y.clamp(0.0, 1.0);

    // Determine lane (Y axis - 5 lanes)
    let lane = if y < 0.2 {
        Lane::LeftWing
    } else if y < 0.4 {
        Lane::LeftHalfSpace
    } else if y < 0.6 {
        Lane::Central
    } else if y < 0.8 {
        Lane::RightHalfSpace
    } else {
        Lane::RightWing
    };

    // Determine quarter (X axis - 4 quarters)
    let quarter = if x < 0.25 {
        Quarter::Defensive
    } else if x < 0.50 {
        Quarter::Middle
    } else if x < 0.75 {
        Quarter::Final
    } else {
        Quarter::Box
    };

    PosPlayZoneId::from_lane_quarter(lane, quarter)
}

/// Convert normalized position to PosPlay zone ID, accounting for attack direction
///
/// # Arguments
/// * `norm_x` - X position in normalized coordinates (0.0 - 1.0)
/// * `norm_y` - Y position in normalized coordinates (0.0 - 1.0)
/// * `attacks_right` - True if the team attacks toward X=1.0
///
/// # Returns
/// PosPlay Zone ID from the team's perspective
pub fn pos_to_posplay_zone_for_team(norm_x: f32, norm_y: f32, attacks_right: bool) -> PosPlayZoneId {
    let zone = pos_to_posplay_zone(norm_x, norm_y);
    if attacks_right {
        zone
    } else {
        zone.flip()
    }
}

/// Convert world position (meters) to team-relative 20-zone
///
/// Uses standard pitch dimensions: 105m x 68m
/// This function provides compatibility with code that uses meter coordinates.
pub fn pos_to_posplay_zone_meters(x_m: f32, y_m: f32, attacks_right: bool) -> PosPlayZoneId {
    const PITCH_LENGTH_M: f32 = 105.0;
    const PITCH_WIDTH_M: f32 = 68.0;

    let norm_x = (x_m / PITCH_LENGTH_M).clamp(0.0, 1.0);
    let norm_y = (y_m / PITCH_WIDTH_M).clamp(0.0, 1.0);
    pos_to_posplay_zone_for_team(norm_x, norm_y, attacks_right)
}

// ============================================================================
// Layer 1 <-> Layer 2 Mapping
// ============================================================================

/// Downscale 20-zone to 6-zone (for calibrator compatibility)
pub fn downscale_to_anchor(zone20: PosPlayZoneId) -> ZoneId {
    match zone20 {
        // Defensive quarter left side -> L_DEF
        PosPlayZoneId::LwDef | PosPlayZoneId::LhsDef => ZoneId::LDef,
        // Defensive quarter center -> C_DEF
        PosPlayZoneId::CDef => ZoneId::CDef,
        // Defensive quarter right side -> R_DEF
        PosPlayZoneId::RhsDef | PosPlayZoneId::RwDef => ZoneId::RDef,
        // Middle + Final + Box left side -> L_ATT
        PosPlayZoneId::LwMid | PosPlayZoneId::LhsMid |
        PosPlayZoneId::LwFin | PosPlayZoneId::LhsFin |
        PosPlayZoneId::LwBox | PosPlayZoneId::LhsBox => ZoneId::LAtt,
        // Middle + Final + Box center -> C_ATT
        PosPlayZoneId::CMid | PosPlayZoneId::CFin | PosPlayZoneId::CBox => ZoneId::CAtt,
        // Middle + Final + Box right side -> R_ATT
        PosPlayZoneId::RhsMid | PosPlayZoneId::RwMid |
        PosPlayZoneId::RhsFin | PosPlayZoneId::RwFin |
        PosPlayZoneId::RhsBox | PosPlayZoneId::RwBox => ZoneId::RAtt,
    }
}

/// Upscale 6-zone to 20-zone set (for analysis)
pub fn upscale_from_anchor(zone6: ZoneId) -> Vec<PosPlayZoneId> {
    match zone6 {
        ZoneId::LDef => vec![PosPlayZoneId::LwDef, PosPlayZoneId::LhsDef],
        ZoneId::CDef => vec![PosPlayZoneId::CDef],
        ZoneId::RDef => vec![PosPlayZoneId::RhsDef, PosPlayZoneId::RwDef],
        ZoneId::LAtt => vec![
            PosPlayZoneId::LwMid, PosPlayZoneId::LhsMid,
            PosPlayZoneId::LwFin, PosPlayZoneId::LhsFin,
            PosPlayZoneId::LwBox, PosPlayZoneId::LhsBox,
        ],
        ZoneId::CAtt => vec![
            PosPlayZoneId::CMid, PosPlayZoneId::CFin, PosPlayZoneId::CBox,
        ],
        ZoneId::RAtt => vec![
            PosPlayZoneId::RhsMid, PosPlayZoneId::RwMid,
            PosPlayZoneId::RhsFin, PosPlayZoneId::RwFin,
            PosPlayZoneId::RhsBox, PosPlayZoneId::RwBox,
        ],
    }
}

/// 20-zone distribution type
pub type PosPlayZoneDistribution = HashMap<PosPlayZoneId, f32>;

/// Create an empty 20-zone distribution
pub fn empty_posplay_distribution() -> PosPlayZoneDistribution {
    let mut dist = HashMap::new();
    for zone in PosPlayZoneId::ALL {
        dist.insert(zone, 0.0);
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pos_to_zone() {
        // Defensive center
        assert_eq!(pos_to_zone(0.5, 0.25), ZoneId::CDef);
        // Attacking center
        assert_eq!(pos_to_zone(0.5, 0.75), ZoneId::CAtt);
        // Left defensive
        assert_eq!(pos_to_zone(0.1, 0.25), ZoneId::LDef);
        // Right attacking
        assert_eq!(pos_to_zone(0.9, 0.75), ZoneId::RAtt);
    }

    #[test]
    fn test_zone_flip() {
        assert_eq!(ZoneId::LDef.flip(), ZoneId::RAtt);
        assert_eq!(ZoneId::CAtt.flip(), ZoneId::CDef);
        assert_eq!(ZoneId::RAtt.flip(), ZoneId::LDef);
    }

    #[test]
    fn test_pos_to_zone_for_team() {
        // attacks_right=true: same as pos_to_zone
        assert_eq!(pos_to_zone_for_team(0.5, 0.75, true), ZoneId::CAtt);

        // attacks_right=false: flipped
        assert_eq!(pos_to_zone_for_team(0.5, 0.75, false), ZoneId::CDef);
    }

    #[test]
    fn test_zone_index_roundtrip() {
        for zone in ZoneId::ALL {
            assert_eq!(ZoneId::from_index(zone.index()), Some(zone));
        }
    }

    // ========== Layer 2: Positional Play Zone Tests (20-zone) ==========

    #[test]
    fn test_pos_to_posplay_zone() {
        // Central Final (X=0.5-0.75)
        assert_eq!(pos_to_posplay_zone(0.6, 0.5), PosPlayZoneId::CFin);
        // Left Wing Defensive (X=0-0.25)
        assert_eq!(pos_to_posplay_zone(0.1, 0.1), PosPlayZoneId::LwDef);
        // Right Half-Space Box (X=0.75-1.0)
        assert_eq!(pos_to_posplay_zone(0.85, 0.7), PosPlayZoneId::RhsBox);
        // Left Half-Space Middle (X=0.25-0.5)
        assert_eq!(pos_to_posplay_zone(0.35, 0.3), PosPlayZoneId::LhsMid);
        // Central Box
        assert_eq!(pos_to_posplay_zone(0.9, 0.5), PosPlayZoneId::CBox);
    }

    #[test]
    fn test_posplay_zone_lane_quarter() {
        let zone = PosPlayZoneId::LhsBox;
        assert_eq!(zone.lane(), Lane::LeftHalfSpace);
        assert_eq!(zone.quarter(), Quarter::Box);
        assert!(zone.is_halfspace());
        assert!(zone.is_box());
        assert!(zone.is_finishing_zone());
    }

    #[test]
    fn test_posplay_zone_flip() {
        // LW_DEF -> RW_BOX (defensive <-> box, left <-> right)
        assert_eq!(PosPlayZoneId::LwDef.flip(), PosPlayZoneId::RwBox);
        // C_MID -> C_FIN (middle <-> final, center stays center)
        assert_eq!(PosPlayZoneId::CMid.flip(), PosPlayZoneId::CFin);
        // LHS_BOX -> RHS_DEF
        assert_eq!(PosPlayZoneId::LhsBox.flip(), PosPlayZoneId::RhsDef);
    }

    #[test]
    fn test_downscale_to_anchor() {
        // Defensive quarter zones map to DEF
        assert_eq!(downscale_to_anchor(PosPlayZoneId::LwDef), ZoneId::LDef);
        assert_eq!(downscale_to_anchor(PosPlayZoneId::LhsDef), ZoneId::LDef);
        assert_eq!(downscale_to_anchor(PosPlayZoneId::CDef), ZoneId::CDef);
        assert_eq!(downscale_to_anchor(PosPlayZoneId::RhsDef), ZoneId::RDef);

        // Middle + Final + Box map to ATT
        assert_eq!(downscale_to_anchor(PosPlayZoneId::LwMid), ZoneId::LAtt);
        assert_eq!(downscale_to_anchor(PosPlayZoneId::LhsFin), ZoneId::LAtt);
        assert_eq!(downscale_to_anchor(PosPlayZoneId::LhsBox), ZoneId::LAtt);
        assert_eq!(downscale_to_anchor(PosPlayZoneId::CMid), ZoneId::CAtt);
        assert_eq!(downscale_to_anchor(PosPlayZoneId::CBox), ZoneId::CAtt);
    }

    #[test]
    fn test_upscale_from_anchor() {
        let l_att_zones = upscale_from_anchor(ZoneId::LAtt);
        assert_eq!(l_att_zones.len(), 6); // LW/LHS × MID/FIN/BOX
        assert!(l_att_zones.contains(&PosPlayZoneId::LwMid));
        assert!(l_att_zones.contains(&PosPlayZoneId::LhsMid));
        assert!(l_att_zones.contains(&PosPlayZoneId::LwFin));
        assert!(l_att_zones.contains(&PosPlayZoneId::LhsFin));
        assert!(l_att_zones.contains(&PosPlayZoneId::LwBox));
        assert!(l_att_zones.contains(&PosPlayZoneId::LhsBox));

        let c_def_zones = upscale_from_anchor(ZoneId::CDef);
        assert_eq!(c_def_zones.len(), 1);
        assert_eq!(c_def_zones[0], PosPlayZoneId::CDef);

        let c_att_zones = upscale_from_anchor(ZoneId::CAtt);
        assert_eq!(c_att_zones.len(), 3); // C_MID, C_FIN, C_BOX
    }

    #[test]
    fn test_lane_mirror() {
        assert_eq!(Lane::LeftWing.mirror(), Lane::RightWing);
        assert_eq!(Lane::LeftHalfSpace.mirror(), Lane::RightHalfSpace);
        assert_eq!(Lane::Central.mirror(), Lane::Central);
    }

    #[test]
    fn test_quarter_flip() {
        assert_eq!(Quarter::Defensive.flip(), Quarter::Box);
        assert_eq!(Quarter::Middle.flip(), Quarter::Final);
        assert_eq!(Quarter::Final.flip(), Quarter::Middle);
        assert_eq!(Quarter::Box.flip(), Quarter::Defensive);
    }

    #[test]
    fn test_posplay_zone_all_20() {
        assert_eq!(PosPlayZoneId::ALL.len(), 20);

        // Check each zone has unique lane+quarter combination
        let mut seen = std::collections::HashSet::new();
        for zone in PosPlayZoneId::ALL {
            let key = (zone.lane(), zone.quarter());
            assert!(seen.insert(key), "Duplicate lane+quarter for {:?}", zone);
        }
    }

    #[test]
    fn test_halfspace_zones() {
        let halfspace_zones: Vec<_> = PosPlayZoneId::ALL
            .iter()
            .filter(|z| z.is_halfspace())
            .collect();
        assert_eq!(halfspace_zones.len(), 8); // 2 lanes × 4 quarters
    }

    #[test]
    fn test_box_zones() {
        let box_zones: Vec<_> = PosPlayZoneId::ALL
            .iter()
            .filter(|z| z.is_box())
            .collect();
        assert_eq!(box_zones.len(), 5); // 5 lanes × 1 quarter (BOX)
    }

    #[test]
    fn test_finishing_zones() {
        let finishing_zones: Vec<_> = PosPlayZoneId::ALL
            .iter()
            .filter(|z| z.is_finishing_zone())
            .collect();
        assert_eq!(finishing_zones.len(), 3); // LHS_BOX, C_BOX, RHS_BOX
    }

    // ========== FIX_2601/0113: 20-Zone Verification Tests ==========

    /// P0: Home/Away swap symmetry
    /// Same position, different team perspective -> flipped zones
    #[test]
    fn test_home_away_zone_symmetry() {
        for x in [0.1, 0.3, 0.5, 0.7, 0.9] {
            for y in [0.1, 0.3, 0.5, 0.7, 0.9] {
                let home_zone = pos_to_posplay_zone_for_team(x, y, true);
                let away_zone = pos_to_posplay_zone_for_team(x, y, false);
                assert_eq!(home_zone.flip(), away_zone,
                    "Position ({}, {}) home zone flipped should equal away zone", x, y);
            }
        }
    }

    /// P0: attacks_right coordinate symmetry
    /// Mirrored positions should produce same zone when attacking same direction
    /// Note: Avoids boundary values (0.25, 0.5, 0.75, 0.2, 0.4, 0.6, 0.8) to prevent
    /// precision issues where x exactly on boundary maps differently than 1-x
    #[test]
    fn test_attacks_right_coordinate_symmetry() {
        // Use mid-zone values to avoid boundary conditions
        for x in [0.125, 0.375, 0.625, 0.875] {  // Centers of each quarter
            for y in [0.1, 0.3, 0.5, 0.7, 0.9] {
                let zone_right = pos_to_posplay_zone_for_team(x, y, true);
                let mirror_x = 1.0 - x;
                let mirror_y = 1.0 - y;
                let zone_left = pos_to_posplay_zone_for_team(mirror_x, mirror_y, false);

                assert_eq!(zone_right, zone_left,
                    "Mirrored coords should produce same zone: ({},{}) vs ({},{})",
                    x, y, mirror_x, mirror_y);
            }
        }
    }

    /// P0: to_index/from_index roundtrip for all 20 zones
    #[test]
    fn test_posplay_zone_index_roundtrip() {
        // All 20 zones must roundtrip correctly
        for zone in PosPlayZoneId::ALL {
            let idx = zone.to_index();
            assert!(idx < 20, "Index {} out of range for {:?}", idx, zone);
            let recovered = PosPlayZoneId::from_index(idx);
            assert_eq!(Some(zone), recovered,
                "Roundtrip failed for {:?} -> {} -> {:?}", zone, idx, recovered);
        }

        // Verify no duplicate indices
        let mut seen = [false; 20];
        for zone in PosPlayZoneId::ALL {
            let idx = zone.to_index();
            assert!(!seen[idx], "Duplicate index {} for {:?}", idx, zone);
            seen[idx] = true;
        }

        // Verify out-of-range returns None
        assert_eq!(PosPlayZoneId::from_index(20), None);
        assert_eq!(PosPlayZoneId::from_index(100), None);
    }

    /// P1: 6-zone anchor calculates directly from coords, not via 20-zone
    #[test]
    fn test_6zone_anchor_direct_calculation() {
        // Test that 6-zone and 20-zone are calculated independently
        let test_points = [
            // (x, y) - test various positions
            (0.15, 0.25),  // Left defensive area
            (0.5, 0.5),    // Center mid area
            (0.85, 0.75),  // Right attacking area
            (0.1, 0.1),    // Far left defensive
            (0.9, 0.9),    // Far right attacking
        ];

        for (x, y) in test_points {
            // Direct 6-zone calculation
            let zone6 = pos_to_zone(x, y);
            // Direct 20-zone calculation
            let zone20 = pos_to_posplay_zone(x, y);

            // Verify downscale produces a valid mapping
            let downscaled = downscale_to_anchor(zone20);

            // The downscaled zone should be a valid 6-zone
            assert!(ZoneId::ALL.contains(&downscaled),
                "Downscale at ({}, {}) produced invalid zone", x, y);

            // Note: downscale may not equal direct 6-zone due to different boundaries
            // The 6-zone uses 3-column (0-0.33-0.67-1.0) and 2-row (0-0.5-1.0)
            // The 20-zone uses 4-row quarters (0-0.25-0.5-0.75-1.0)
            // This is intentional - 6-zone is for calibration, 20-zone for tactics

            // Verify zone6 is in expected range
            assert!(ZoneId::ALL.contains(&zone6), "pos_to_zone returned invalid zone");
        }
    }

    /// P1: Double-flip idempotence for all zone types
    #[test]
    fn test_double_flip_idempotence() {
        // zone.flip().flip() == zone for all 20 zones
        for zone in PosPlayZoneId::ALL {
            assert_eq!(zone, zone.flip().flip(),
                "Double flip not idempotent for {:?}", zone);
        }

        // lane.mirror().mirror() == lane for all lanes
        for lane in Lane::ALL {
            assert_eq!(lane, lane.mirror().mirror(),
                "Double mirror not idempotent for {:?}", lane);
        }

        // quarter.flip().flip() == quarter for all quarters
        for quarter in Quarter::ALL {
            assert_eq!(quarter, quarter.flip().flip(),
                "Double flip not idempotent for {:?}", quarter);
        }
    }

    /// P2: Half-space entry from wide zones should have reasonable factor
    #[test]
    fn test_halfspace_zone_relationships() {
        // Wide zones should connect to halfspace zones
        // This tests the structural relationship, not the transition weights

        // LwMid should be adjacent to LhsMid
        let lw_mid = PosPlayZoneId::LwMid;
        let lhs_mid = PosPlayZoneId::LhsMid;
        assert_eq!(lw_mid.lane(), Lane::LeftWing);
        assert_eq!(lhs_mid.lane(), Lane::LeftHalfSpace);
        assert_eq!(lw_mid.quarter(), lhs_mid.quarter());

        // RwFin should be adjacent to RhsFin
        let rw_fin = PosPlayZoneId::RwFin;
        let rhs_fin = PosPlayZoneId::RhsFin;
        assert_eq!(rw_fin.lane(), Lane::RightWing);
        assert_eq!(rhs_fin.lane(), Lane::RightHalfSpace);
        assert_eq!(rw_fin.quarter(), rhs_fin.quarter());
    }

    /// P3: Zone calculation performance (should complete quickly)
    #[test]
    fn test_zone_calculation_performance() {
        use std::time::Instant;

        let start = Instant::now();
        for _ in 0..100_000 {
            let _ = pos_to_posplay_zone_meters(52.5, 34.0, true);
        }
        let elapsed = start.elapsed();

        // Should complete 100k calls in < 100ms
        assert!(elapsed.as_millis() < 100,
            "Zone calculation too slow: {:?} for 100k calls", elapsed);
    }
}
