//! Replay Migration Module (FIX_2601/0123 #10)
//!
//! This module handles backward compatibility for replay files created with
//! older versions of the engine.
//!
//! # Version History
//!
//! | coord_contract_version | coord_system | Notes |
//! |------------------------|--------------|-------|
//! | 0 (default) | legacy_axis_swap | Pre-FIX_2601, Y=length, X=width |
//! | 1 | meters_v1 | Intermediate (unused in practice) |
//! | 2 | meters_v2 | Current: X=length, Y=width |
//!
//! # Migration Chain
//!
//! ```text
//! v0 (legacy) → swap axes → v2 (current)
//! v1 (if any) → verify → v2 (current)
//! ```

use crate::engine::coordinate_contract::{COORD_CONTRACT_VERSION, COORD_SYSTEM_METERS_V2};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Migration error types
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationError {
    /// Unsupported replay version
    UnsupportedVersion { found: u8, supported: Vec<u8> },
    /// Unknown coordinate system
    UnknownCoordSystem(String),
    /// Data integrity issue during migration
    DataIntegrity(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::UnsupportedVersion { found, supported } => {
                write!(
                    f,
                    "Unsupported coord_contract_version: {}. Supported: {:?}",
                    found, supported
                )
            }
            MigrationError::UnknownCoordSystem(system) => {
                write!(f, "Unknown coordinate system: {}", system)
            }
            MigrationError::DataIntegrity(msg) => {
                write!(f, "Data integrity error: {}", msg)
            }
        }
    }
}

impl std::error::Error for MigrationError {}

/// Migration result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    /// Whether any migration was applied
    pub migrated: bool,
    /// Original version before migration
    pub from_version: u8,
    /// Final version after migration
    pub to_version: u8,
    /// Original coordinate system
    pub from_coord_system: String,
    /// Final coordinate system
    pub to_coord_system: String,
    /// Number of events migrated
    pub events_migrated: usize,
}

impl MigrationResult {
    /// Create a result indicating no migration was needed
    pub fn no_migration(version: u8, coord_system: &str) -> Self {
        Self {
            migrated: false,
            from_version: version,
            to_version: version,
            from_coord_system: coord_system.to_string(),
            to_coord_system: coord_system.to_string(),
            events_migrated: 0,
        }
    }
}

/// Check if a replay needs migration
pub fn needs_migration(coord_contract_version: u8, coord_system: &str) -> bool {
    coord_contract_version != COORD_CONTRACT_VERSION
        || coord_system != COORD_SYSTEM_METERS_V2
}

/// Get supported versions for migration
pub fn supported_versions() -> Vec<u8> {
    vec![0, 1, 2]
}

/// Migrate replay coordinates from legacy format
///
/// This is used by `OfReplay::normalize_coord_contract_in_place()` internally.
/// The swap operation converts (Y=length, X=width) to (X=length, Y=width).
pub fn swap_coordinates(x: f64, y: f64) -> (f64, f64) {
    (y, x)
}

/// Verify migration can be performed
pub fn verify_migration(coord_contract_version: u8, coord_system: &str) -> Result<(), MigrationError> {
    let supported = supported_versions();

    if !supported.contains(&coord_contract_version) {
        return Err(MigrationError::UnsupportedVersion {
            found: coord_contract_version,
            supported,
        });
    }

    // Known coordinate systems
    let known_systems = [
        "legacy_axis_swap",
        "meters_v1",
        "meters_v2",
        "", // Empty for very old replays
    ];

    if !known_systems.contains(&coord_system) {
        return Err(MigrationError::UnknownCoordSystem(coord_system.to_string()));
    }

    Ok(())
}

/// Migration context for tracking changes
#[derive(Debug, Clone)]
pub struct MigrationContext {
    pub original_version: u8,
    pub original_coord_system: String,
    pub target_version: u8,
    pub target_coord_system: String,
}

impl MigrationContext {
    pub fn new(from_version: u8, from_coord_system: &str) -> Self {
        Self {
            original_version: from_version,
            original_coord_system: from_coord_system.to_string(),
            target_version: COORD_CONTRACT_VERSION,
            target_coord_system: COORD_SYSTEM_METERS_V2.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_migration_current_version() {
        assert!(!needs_migration(2, "meters_v2"));
    }

    #[test]
    fn test_needs_migration_legacy_version() {
        assert!(needs_migration(0, "legacy_axis_swap"));
        assert!(needs_migration(1, "meters_v1"));
    }

    #[test]
    fn test_needs_migration_version_mismatch() {
        assert!(needs_migration(0, "meters_v2"));
        assert!(needs_migration(2, "legacy_axis_swap"));
    }

    #[test]
    fn test_supported_versions() {
        let versions = supported_versions();
        assert!(versions.contains(&0));
        assert!(versions.contains(&1));
        assert!(versions.contains(&2));
        assert!(!versions.contains(&99));
    }

    #[test]
    fn test_verify_migration_supported() {
        assert!(verify_migration(0, "legacy_axis_swap").is_ok());
        assert!(verify_migration(1, "meters_v1").is_ok());
        assert!(verify_migration(2, "meters_v2").is_ok());
    }

    #[test]
    fn test_verify_migration_unsupported_version() {
        let result = verify_migration(99, "meters_v2");
        assert!(matches!(result, Err(MigrationError::UnsupportedVersion { .. })));
    }

    #[test]
    fn test_verify_migration_unknown_coord_system() {
        let result = verify_migration(2, "unknown_system");
        assert!(matches!(result, Err(MigrationError::UnknownCoordSystem(_))));
    }

    #[test]
    fn test_swap_coordinates() {
        let (x, y) = swap_coordinates(68.0, 105.0);
        assert_eq!(x, 105.0);
        assert_eq!(y, 68.0);
    }

    #[test]
    fn test_migration_result_no_migration() {
        let result = MigrationResult::no_migration(2, "meters_v2");
        assert!(!result.migrated);
        assert_eq!(result.from_version, 2);
        assert_eq!(result.to_version, 2);
    }

    #[test]
    fn test_migration_context() {
        let ctx = MigrationContext::new(0, "legacy_axis_swap");
        assert_eq!(ctx.original_version, 0);
        assert_eq!(ctx.original_coord_system, "legacy_axis_swap");
        assert_eq!(ctx.target_version, COORD_CONTRACT_VERSION);
        assert_eq!(ctx.target_coord_system, COORD_SYSTEM_METERS_V2);
    }
}
