//! Ball physics parameters (SSOT for the match contract path).
//!
//! Purpose:
//! - Provide a single place for ball-physics *parameters* that are referenced by the
//!   ActionQueue + tick-based match simulation contract.
//! - Avoid “magic constants” duplication across legacy (`phase_action/*`) and the new
//!   `ActionQueue` pipeline.
//! - Keep parameters dt-invariant where possible (use per-second coefficients + per-step
//!   multipliers computed from dt).
//!
//! Non-goals (v1):
//! - Do not rewrite legacy ball physics (`phase_action/ball_physics.rs`).
//! - Do not introduce new gameplay behavior yet; this module is an SSOT surface.

use crate::engine::timestep;

/// Decision tick duration (s). This is the authoritative sim tick for ActionQueue scheduling.
pub const DECISION_DT: f32 = timestep::DECISION_DT;

/// Integration substep duration (s). Used by player locomotion today; ball may migrate later.
pub const SUBSTEP_DT: f32 = timestep::SUBSTEP_DT;

/// Decision ticks per second (derived from `DECISION_DT`).
///
/// Keep this explicit (instead of float division) because we use it in tick math (flight ticks).
pub const DECISION_TICKS_PER_SECOND: u64 = 4;

/// Gravity (m/s²).
pub const GRAVITY_MPS2: f32 = 9.81;

/// Legacy per-decision-tick velocity retention factor for ground roll.
///
/// This is the historical value from `phase_action::duration::GRASS_FRICTION` and is kept as a
/// *compatibility anchor*. The contract path should prefer `k_roll_1ps` + `exp(-k*dt)`.
pub const LEGACY_GRASS_FRICTION_PER_DECISION_TICK: f32 = 0.95;

/// Ball physics parameters for the contract path.
///
/// Notes:
/// - `k_roll_1ps` is dt-invariant. Per-step damping uses `exp(-k * dt)`.
/// - `stop_speed_mps` is an absolute speed threshold; below this we consider the ball stopped
///   for gameplay purposes (prevents micro jitter in discrete ticks).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BallPhysicsParams {
    /// Ground roll damping coefficient (1/s).
    pub k_roll_1ps: f32,
    /// Stop threshold (m/s).
    pub stop_speed_mps: f32,
    /// Post/crossbar proximity tolerance used for woodwork hit detection (meters).
    ///
    /// This is intentionally “gamey” to compensate for discrete ticks (250ms)
    /// and simplified goal geometry in v1.
    pub woodwork_tolerance_m: f32,
    /// Woodwork restitution coefficient (0..1).
    pub woodwork_restitution: f32,
    /// Bounce threshold on the collision normal (m/s).
    pub woodwork_bounce_speed_threshold_mps: f32,
    /// Max number of bounces (legacy support; not used by ActionQueue yet).
    pub max_bounces: u8,
}

impl BallPhysicsParams {
    /// Convert `k_roll` to a per-step velocity multiplier.
    #[inline]
    pub fn roll_multiplier(&self, dt: f32) -> f32 {
        (-self.k_roll_1ps * dt).exp()
    }

    /// Stop threshold in Vel10 units (0.1m/s).
    #[inline]
    pub fn stop_speed_vel10(&self) -> i16 {
        (self.stop_speed_mps * 10.0).round() as i16
    }
}

/// Default contract parameters.
///
/// Baseline mapping:
/// - Legacy `phase_action::duration::GRASS_FRICTION = 0.95` was defined as a **per-decision-tick**
///   velocity retention factor at `DECISION_DT = 0.25s`.
/// - Equivalent dt-invariant `k_roll`:
///   `k_roll = -ln(0.95) / 0.25 ≈ 0.20517 1/s`
pub const DEFAULT: BallPhysicsParams = BallPhysicsParams {
    k_roll_1ps: 0.205_17,
    // Keep legacy stop threshold for now (matches `phase_action::duration::BALL_MIN_VELOCITY`).
    stop_speed_mps: 0.5,
    // Matches `ball_flight_resolver::FlightConfig::post_tolerance` (30cm).
    woodwork_tolerance_m: 0.3,
    // Keep legacy bounce coefficient for now; woodwork bounce is not wired yet in ActionQueue.
    woodwork_restitution: 0.6,
    // Contract doc suggested 0.8..2.5; pick a neutral default for now.
    woodwork_bounce_speed_threshold_mps: 1.5,
    max_bounces: 3,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_tick_constants_match_contract() {
        assert_eq!(DECISION_DT, 0.25);
        assert_eq!(DECISION_TICKS_PER_SECOND, 4);
    }

    #[test]
    fn test_roll_multiplier_matches_legacy_factor_at_decision_dt() {
        // Legacy GRASS_FRICTION was 0.95 per decision tick.
        let expected = LEGACY_GRASS_FRICTION_PER_DECISION_TICK;
        let m = DEFAULT.roll_multiplier(DECISION_DT);
        assert!((m - expected).abs() < 0.01, "m={m} expected~{expected}");
    }

    #[test]
    fn test_stop_speed_vel10_rounding() {
        assert_eq!(DEFAULT.stop_speed_vel10(), 5);
    }

    #[test]
    fn test_woodwork_tolerance_default() {
        assert!((DEFAULT.woodwork_tolerance_m - 0.3).abs() < 1e-6);
    }
}
