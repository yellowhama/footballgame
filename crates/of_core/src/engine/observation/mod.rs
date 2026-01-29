//! Observation Module - TickSnapshot-based Observation Builders
//!
//! FIX_2601 Phase 4: SSOT-compliant observation generation from TickSnapshot.
//!
//! ## Design Principles
//!
//! 1. **SSOT Contract**: All observations are derived solely from TickSnapshot
//! 2. **No MatchEngine dependency**: ObservationBuilders don't import MatchEngine
//! 3. **TeamView coordinates**: Own goal at x=0, opponent goal at x=105m
//!
//! ## Available Builders
//!
//! - `SimpleVectorBuilder`: 115-float vector (Google Football simple115_v2 style)
//! - `MiniMapBuilder`: 4-channel spatial planes (72x96, SMM style)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use of_core::engine::observation::{ObservationBuilder, SimpleVectorBuilder, MiniMapBuilder, MiniMapSpec};
//!
//! // Build simple vector observation
//! let builder = SimpleVectorBuilder;
//! let obs = builder.build(&snapshot, true); // home perspective
//! let flat = obs.to_flat_vector(); // 115 floats
//!
//! // Build minimap observation
//! let builder = MiniMapBuilder::new(MiniMapSpec::default());
//! let obs = builder.build(&snapshot, false); // away perspective
//! let tensor = obs.to_flat_chw(); // 4 * 72 * 96 floats
//! ```

mod builder;
mod common;
mod minimap;
mod simple_vector;

pub use builder::ObservationBuilder;
pub use common::{find_active_player, normalize_direction, to_team_view_pos, to_team_view_vel};
pub use minimap::{MiniMapBuilder, MiniMapObservation, MiniMapSpec};
pub use simple_vector::{SimpleVectorBuilder, SimpleVectorObservation, TeamViewBallObs, TeamViewPlayerObs};
