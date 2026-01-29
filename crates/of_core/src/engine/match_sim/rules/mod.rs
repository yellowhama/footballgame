//! Centralized Rule System
//!
//! This module implements the RuleDispatcher pattern based on basketball RE analysis
//! (FIX_2601/0123/01_RULE_DISPATCHER_PATTERN).
//!
//! ## Goals
//! - Deterministic rule evaluation order
//! - Team-neutral evaluation (reduce bias from 25% to <5%)
//! - Clear decision taxonomy
//!
//! ## Architecture
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                       RuleDispatcher                            │
//! │  (Centralized rule evaluation hub)                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  Evaluation Order (Fixed):                                      │
//! │  1. Goal check (highest priority)                               │
//! │  2. Out of play check                                           │
//! │  3. Offside check                                               │
//! │  4. Foul check                                                  │
//! │  5. Handball check                                              │
//! │                                                                 │
//! │                    RuleDecision[]                               │
//! │                          │                                      │
//! │                    StateTransition                              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Migration Strategy
//!
//! The module supports gradual migration via `RuleCheckMode`:
//!
//! - `StatisticsOnly` (default): Dispatcher evaluates but legacy code decides
//! - `LegacyWithTracking`: A/B comparison mode, logs discrepancies
//! - `DispatcherPrimary`: Dispatcher makes all decisions
//!
//! Set via environment variable: `OF_RULE_CHECK_MODE=dispatcher|tracking|stats`
//!
//! ## Usage
//! ```ignore
//! let mut dispatcher = RuleDispatcher::new();
//!
//! // Each tick:
//! let decisions = dispatcher.evaluate_tick(
//!     &ball_pos,
//!     ball_in_goal,
//!     last_touch_team,
//!     scorer_idx,
//!     assister_idx,
//!     pass_event.as_ref(),
//!     offside_detected,
//!     &contacts,
//!     rng.gen(),
//! );
//!
//! for decision in decisions {
//!     match decision {
//!         RuleDecision::Continue => { /* normal play */ }
//!         RuleDecision::Goal { .. } => { /* handle goal */ }
//!         RuleDecision::OutOfPlay { .. } => { /* handle restart */ }
//!         // ...
//!     }
//! }
//! ```

mod dispatcher;
mod types;
mod wrappers;

pub use dispatcher::{RuleConfig, RuleDispatcher};
pub use types::{
    Card, ContactEvent, ContactType, DefenseIntentInfo, DefenseIntentType, DispatcherContext,
    FieldBounds, FoulType, HandballContactEvent, PassEvent, RuleCheckMode, RuleDecision,
    RuleEvaluationStats, RuleRestartType, RuleTeamId, TechniqueType,
};
pub use wrappers::{
    check_duel_foul_wrapper, check_foul_wrapper, check_goal_wrapper, check_handball_wrapper,
    check_offside_wrapper, check_out_of_play_wrapper, comparison_stats, ComparisonStats,
    LegacyDuelFoulResult, LegacyFoulResult, LegacyGoalResult, LegacyHandballResult,
    LegacyOffsideResult, LegacyOutOfPlayResult, RuleContext,
};
