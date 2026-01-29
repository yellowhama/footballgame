//! Pitch Zone - Deprecated, use calibration::zone types directly
//!
//! FIX_2601/0113: Migrated from 15-zone PitchZone to 20-zone PosPlayZoneId.
//! This module provides backward compatibility aliases.

pub use crate::calibration::zone::{
    pos_to_posplay_zone_meters as zone_of_position,
    PosPlayZoneId as PitchZone,
};

// Note: PosPlayZoneId already implements:
// - to_index() -> usize (0-19)
// - from_index(usize) -> Option<Self>
// - flip() (for team perspective)
// - Serialize, Deserialize
