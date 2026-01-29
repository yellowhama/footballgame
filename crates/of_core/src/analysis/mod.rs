//! # Analysis Module
//!
//! Post-match analysis and QA validation tools for football simulation.
//!
//! ## Submodules
//!
//! - `events` - Event extraction (carry, sprint, run)
//! - `metrics` - Statistical metrics (gini, shape, movement)
//! - `qa` - Quality assurance validators (physics, consistency, likeness)
//! - `scout` - Scout report generation (model, style_tags, report)
//!
//! ## FIX_2601/NEW_FUNC
//!
//! This module implements the following specs:
//! - PASS_NETWORK_ENTROPY_ANALYSIS.md
//! - DRIBBLE_MOVEMENT_ANALYSIS.md
//! - REALTIME_SYSTEMS_ANALYSIS.md
//! - SCOUT_REPORT_SYSTEM.md

pub mod events;
pub mod metrics;
pub mod qa;
pub mod scout;
