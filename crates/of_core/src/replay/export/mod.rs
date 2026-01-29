//! Broadcast Timeline Export Module
//!
//! Converts replay events into broadcast-ready highlight timelines
//! with automatic camera selection, importance scoring, and multi-clip generation.

pub mod clip_generator;
pub mod context;
pub mod importance;
pub mod timeline;
pub mod types;

// Re-export main types and functions
pub use clip_generator::*;
pub use context::*;
pub use importance::*;
pub use timeline::*;
pub use types::*;
