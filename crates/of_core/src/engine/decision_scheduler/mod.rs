//! Decision scheduling layer (DPQ / cadence / perception).
//!
//! v1.1 (FIX_2601/0113): DPQ is introduced as a routing layer only.
//! - Sim tick remains SSOT (`DECISION_DT=0.25s`).
//! - DPQ must not change match outcomes compared to baseline.
//! - Executor semantics (ActionQueue) remain unchanged.
//!
//! v1.2 (FIX_2601/0115): Variable cadence based on proximity to ball.
//! - Active zone (20m): every tick (250ms)
//! - Passive zone: every 4 ticks (1000ms)
//! - Pull-forward mechanism for sudden context changes.

pub mod cadence;
pub mod dpq;

pub use cadence::{calculate_cadence_level, CadenceLevel};
pub use dpq::DecisionScheduler;
