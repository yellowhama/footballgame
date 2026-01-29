//! # Scout Report Module
//!
//! Scout report generation for game features.
//!
//! - `model` - ScoutedValue<T> and uncertainty calculations
//! - `style_tags` - Team style tag generation
//! - `report` - Scout report structure and generation

pub mod model;
pub mod style_tags;
pub mod report;

pub use model::*;
pub use style_tags::*;
pub use report::*;
