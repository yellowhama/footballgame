//! # MatchMetrics - Unified Metrics Module
//!
//! FIX_2601 Phase 5: SSOT-compliant unified metrics container.
//!
//! ## SSOT Hierarchy
//!
//! This module provides a unified view of match metrics while respecting
//! the SSOT (Single Source of Truth) layer hierarchy:
//!
//! - **Layer 1 (Product SSOT)**: `MatchResult.statistics` - Reference only, no duplication
//! - **Layer 2 (QA)**: `QaAdvancedMetrics` - Line spacing, pass network, PPDA
//! - **Layer 3 (RL)**: `EpisodeMetrics` - Training metrics (optional)
//! - **Engine**: `QualityReport` - Simulation quality verification
//!
//! ## Key Design Principles
//!
//! 1. **No Product Stat Duplication**: `MatchMetrics` does NOT duplicate
//!    `MatchResult.statistics`. It provides `ProductStatsSummary` as an
//!    on-demand computed view when needed.
//!
//! 2. **QA Metrics from MatchResult**: `compute_qa_from_result()` takes
//!    `&MatchResult` and computes QA metrics without copying Product data.
//!
//! 3. **RL Layer Optional**: `EpisodeMetrics` can be added later via
//!    `set_rl_episode()` without affecting QA/Product layers.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use of_core::engine::metrics::{MatchMetrics, compute_all_metrics};
//!
//! // After match completion
//! let metrics = compute_all_metrics(&result, Some(&setup));
//!
//! // Check QA grade
//! if let Some(grade) = metrics.overall_grade() {
//!     println!("QA Grade: {}", grade.label());
//! }
//!
//! // Add RL metrics for training
//! metrics.set_rl_episode(episode_metrics);
//! ```

mod compute;
mod unified;

pub use compute::{
    compute_all_metrics,
    compute_qa_from_result,
};

pub use unified::{
    MatchMetrics,
    MetricsMetadata,
    ProductStatsSummary,
};
