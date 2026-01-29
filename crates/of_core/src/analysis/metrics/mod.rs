//! # Metrics Module
//!
//! Statistical metrics for match analysis.
//!
//! - `gini` - Gini coefficient for inequality measurement
//! - `shape` - Team shape metrics (width, depth, convex hull)
//! - `movement` - Movement entropy and occupancy distribution

pub mod gini;
pub mod shape;
pub mod movement;

pub use gini::*;
pub use shape::*;
pub use movement::*;
