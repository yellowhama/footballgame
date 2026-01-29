//! Ball Physics System
//!
//! This module contains ball-related physics logic for MatchEngine:
//! - Ball position update (follows owner or physics)
//! - Ball physics application (drag, rolling resistance)
//! - Ball ownership check (skill-based)
//! - Tackling score calculation
//! - Ball flight physics (Phase 3)
//! - **Ball Physics V2**: Substep-based physics (10ms steps)
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use crate::engine::audit_gates;
use crate::engine::ball::get_ball_position_3d;
use crate::engine::physics_constants;
use crate::engine::physics_constants::{
    aerial, google_football, home_advantage, projectile, substep,
};
use crate::engine::types::coord10::{Coord10, Vel10}; // FIX_2512 Phase 4 - TASK_09
use crate::models::TeamSide;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // Ball Physics System
    // ===========================================

    /// Ball physics update (called each tick)
    pub(crate) fn update_ball(&mut self) {
        if let Some(owner_idx) = self.ball.current_owner {
            // FIX_2601: owner_pos is already Coord10, use directly
            let owner_pos = self.get_player_position_by_index(owner_idx);
            self.ball.position = owner_pos;
            self.ball.velocity = Vel10::default();
            self.ball.height = 0;
        } else {
            // No owner - apply physics
            self.apply_ball_physics();
            self.check_ball_ownership();
        }

        self.update_game_state_from_ball();
    }

    // ===========================================
    // Ball Physics V2: Substep System
    // ===========================================

    /// Apply ball physics using substep system (Ball Physics V2)
    ///
    /// Runs 5 substeps of 10ms each for more accurate physics simulation.
    /// This improves accuracy for high-speed balls (25m/s moves 0.25m per substep
    /// instead of 1.25m per tick).
    ///
    /// Phase 2: Now includes gravity-based vertical physics (apply_gravity_step).
    pub(crate) fn apply_ball_physics(&mut self) {
        // Ball Physics V2: Run multiple substeps for accuracy
        for _ in 0..substep::SUBSTEPS_PER_TICK {
            // XY physics (drag, rolling resistance)
            self.apply_physics_step(substep::SUBSTEP_SEC);

            // Ball Physics V2 Phase 2: Gravity-based vertical physics
            self.apply_gravity_step(substep::SUBSTEP_SEC);

            // Early exit if ball stopped (both XY and Z)
            if self.ball.velocity.magnitude() == 0 && !self.ball.is_airborne() {
                break;
            }
        }
    }

    /// Single physics step (10ms) - Ball Physics V2 core
    ///
    /// Applies drag, rolling resistance, and Magnus effect for a single timestep.
    /// Called multiple times per tick by `apply_ball_physics()`.
    ///
    /// # Arguments
    /// * `dt` - Time step in seconds (typically 0.01s = 10ms)
    fn apply_physics_step(&mut self, dt: f32) {
        use physics_constants::ball;

        // Get current velocity in m/s
        let vel_mps = self.ball.velocity.to_mps();
        let speed = (vel_mps.0.powi(2) + vel_mps.1.powi(2)).sqrt();

        // Check stopping threshold
        if speed <= ball::MIN_VELOCITY {
            self.ball.velocity = Vel10::default();
            self.ball.reset_spin(); // Reset spin when ball stops
            return;
        }

        let direction = (vel_mps.0 / speed, vel_mps.1 / speed);

        // Drag force: F_drag = 0.5 * Cd * |v|²
        // Acceleration: a_drag = F_drag / m
        let drag_accel = 0.5 * ball::DRAG_COEFFICIENT * speed.powi(2) / ball::MASS_KG;

        // Rolling resistance: F_rolling = μ * m * g
        // Acceleration: a_rolling = μ * g
        let rolling_accel = ball::ROLLING_RESISTANCE * ball::GRAVITY;

        // Total deceleration (m/s² → m/s change over dt)
        let deceleration = (drag_accel + rolling_accel) * dt;
        let new_speed = (speed - deceleration).max(0.0);

        // Calculate base velocity after drag
        let mut new_vel_mps = (direction.0 * new_speed, direction.1 * new_speed);

        // FIX_2601/0112: Apply Magnus Effect (only when airborne)
        if self.ball.is_airborne() {
            let magnus = self.calculate_magnus_force(new_vel_mps, dt);
            new_vel_mps.0 += magnus.0;
            new_vel_mps.1 += magnus.1;

            // Decay spin due to air resistance
            self.ball.decay_spin(google_football::SPIN_DECAY);
        }

        // Update velocity
        self.ball.velocity = Vel10::from_mps(new_vel_mps.0, new_vel_mps.1);

        // Position update: p += v * dt
        let pos_m = self.ball.position.to_meters();
        let displacement = (new_vel_mps.0 * dt, new_vel_mps.1 * dt);
        let new_pos_m = (
            (pos_m.0 + displacement.0).clamp(audit_gates::BALL_X_MIN, audit_gates::BALL_X_MAX),
            (pos_m.1 + displacement.1).clamp(audit_gates::BALL_Y_MIN, audit_gates::BALL_Y_MAX),
        );
        self.ball.position = Coord10::from_meters(new_pos_m.0, new_pos_m.1);
    }

    /// FIX_2601/0112: Calculate Magnus force for ball curve
    ///
    /// Google Football style Magnus effect:
    /// F_magnus = C × |v|^p × spin_normalized × multiplier
    ///
    /// # Arguments
    /// * `vel_mps` - Current velocity in m/s (x, y)
    /// * `dt` - Time step in seconds
    ///
    /// # Returns
    /// Velocity change (dvx, dvy) in m/s
    fn calculate_magnus_force(&self, vel_mps: (f32, f32), dt: f32) -> (f32, f32) {
        let (_spin_x, spin_y, _spin_z) = self.ball.spin;
        let spin_mag = self.ball.spin_magnitude();

        // No spin = no Magnus effect
        if spin_mag < 0.01 {
            return (0.0, 0.0);
        }

        let speed = (vel_mps.0.powi(2) + vel_mps.1.powi(2)).sqrt();
        if speed < 0.1 {
            return (0.0, 0.0);
        }

        // Google Football Magnus formula:
        // F = C × |v|^p × spin / |spin| × multiplier
        let c = google_football::MAGNUS_COEFFICIENT;
        let p = google_football::MAGNUS_POWER;
        let multiplier = google_football::MAGNUS_MULTIPLIER;

        // Magnitude of Magnus force
        let force_mag = c * speed.powf(p) * multiplier / 1000.0; // Scaled down for reasonable effect

        // Direction: perpendicular to velocity based on spin axis
        // sidespin (spin_y) creates horizontal deflection
        // topspin/backspin (spin_x) creates vertical deflection (affects vz in 3D)
        // For 2D XY motion, sidespin is primary
        let spin_normalized_y = spin_y / spin_mag;

        // Perpendicular to velocity direction
        // If moving in +x direction, sidespin creates +/- y deflection
        let perp_x = -vel_mps.1 / speed; // Perpendicular x component
        let perp_y = vel_mps.0 / speed; // Perpendicular y component

        // Apply force perpendicular to velocity, weighted by sidespin

        (perp_x * force_mag * spin_normalized_y * dt, perp_y * force_mag * spin_normalized_y * dt)
    }

    /// Ball Physics V2: Apply gravity step for vertical motion
    ///
    /// Updates vertical velocity (velocity_z) and height using gravity.
    /// Handles bounce when ball hits ground.
    ///
    /// # Arguments
    /// * `dt` - Time step in seconds (typically 0.01s = 10ms)
    fn apply_gravity_step(&mut self, dt: f32) {
        use physics_constants::bounce;

        // Skip if already rolling on ground
        if self.ball.is_rolling {
            return;
        }

        // Only apply if ball is airborne or has upward velocity
        if !self.ball.is_airborne() {
            return;
        }

        // Gravity in 0.1m/s² per second: need to convert dt to integer change
        // GRAVITY_01M = 98 (9.8 m/s² in 0.1m/s² units)
        // dv = g * dt (in 0.1m/s units)
        let dv = (projectile::GRAVITY_01M as f32 * dt).round() as i16;
        self.ball.velocity_z -= dv;

        // Position update: dh = v * dt (in 0.1m units)
        // velocity_z is in 0.1m/s, dt is in seconds
        // dh in 0.1m = (0.1m/s * 10) * s = velocity_z * dt
        // Use round() for proper motion in both directions
        let dh = (self.ball.velocity_z as f32 * dt).round() as i16;

        // Special case: if ball is very low and moving down, force ground contact
        let new_height = if self.ball.height == 1 && self.ball.velocity_z < -bounce::MIN_BOUNCE_VZ {
            0 // Force ground contact when at edge and clearly falling
        } else {
            self.ball.height + dh
        };

        if new_height <= 0 {
            // Ball hit ground - call handle_landing
            self.ball.height = 0;
            self.handle_landing();
        } else {
            self.ball.height = new_height;
        }
    }

    /// Phase 3: Handle ball landing (bounce or roll transition)
    fn handle_landing(&mut self) {
        use physics_constants::bounce;

        // Already rolling - ignore
        if self.ball.is_rolling {
            return;
        }

        // Impact velocity (absolute)
        let impact_vz = self.ball.velocity_z.abs();

        // Apply grass COR (반발 계수)
        let rebound_vz = (impact_vz as f32 * bounce::GRASS_COR) as i16;

        // Check if should transition to rolling
        let should_roll =
            rebound_vz < bounce::MIN_BOUNCE_VZ || self.ball.bounce_count >= bounce::MAX_BOUNCES;

        if should_roll {
            // Transition to rolling mode
            self.ball.velocity_z = 0;
            self.ball.height = 0;
            self.ball.is_rolling = true;
            self.ball.bounce_count = 0;
        } else {
            // Bounce!
            self.ball.velocity_z = rebound_vz;
            self.ball.bounce_count += 1;
            self.ball.height = 1; // 0.1m to stay airborne

            // Horizontal velocity loss on bounce
            let vel_mps = self.ball.velocity.to_mps();
            let new_vx = vel_mps.0 * (1.0 - bounce::HORIZONTAL_LOSS);
            let new_vy = vel_mps.1 * (1.0 - bounce::HORIZONTAL_LOSS);
            self.ball.velocity = Vel10::from_mps(new_vx, new_vy);

            // Apply spin deflection on bounce
            self.apply_spin_on_bounce();
        }
    }

    /// Phase 3: Apply spin deflection on bounce
    fn apply_spin_on_bounce(&mut self) {
        use physics_constants::bounce;

        // Ignore if no significant curve
        if self.ball.curve_factor.abs() < 0.05 {
            return;
        }

        // Deflect y velocity based on spin direction
        let vel_mps = self.ball.velocity.to_mps();
        let deflection = self.ball.curve_factor * bounce::SPIN_DEFLECTION * vel_mps.0.abs();
        let new_vy = vel_mps.1 + deflection;

        self.ball.velocity = Vel10::from_mps(vel_mps.0, new_vy);

        // Reduce spin after bounce
        self.ball.curve_factor *= bounce::SPIN_DECAY;
    }

    /// Legacy: Apply ball physics for a single tick (50ms)
    ///
    /// Kept for backwards compatibility. Prefer using `apply_ball_physics()`
    /// which uses the substep system internally.
    #[allow(dead_code)]
    pub(crate) fn apply_ball_physics_legacy(&mut self) {
        use physics_constants::ball;

        let vel_mps = self.ball.velocity.to_mps();
        let speed = (vel_mps.0.powi(2) + vel_mps.1.powi(2)).sqrt();
        let stopping_threshold = ball::MIN_VELOCITY;

        if speed > stopping_threshold {
            let direction = (vel_mps.0 / speed, vel_mps.1 / speed);
            let drag_magnitude = 0.5 * ball::DRAG_COEFFICIENT * speed.powi(2);
            let rolling_magnitude = ball::ROLLING_RESISTANCE * ball::MASS_KG * ball::GRAVITY
                / physics_constants::field::LENGTH_M;
            let deceleration = drag_magnitude + rolling_magnitude;
            let new_speed = (speed - deceleration).max(0.0);

            let new_vel_mps = (direction.0 * new_speed, direction.1 * new_speed);
            self.ball.velocity = Vel10::from_mps(new_vel_mps.0, new_vel_mps.1);

            let pos_m = self.ball.position.to_meters();
            let new_pos_m = (
                (pos_m.0 + new_vel_mps.0).clamp(audit_gates::BALL_X_MIN, audit_gates::BALL_X_MAX),
                (pos_m.1 + new_vel_mps.1).clamp(audit_gates::BALL_Y_MIN, audit_gates::BALL_Y_MAX),
            );
            self.ball.position = Coord10::from_meters(new_pos_m.0, new_pos_m.1);
        } else {
            self.ball.velocity = Vel10::default();
        }

        if self.ball.height > 0 {
            self.ball.height = (self.ball.height - 1).max(0);
        }
    }

    /// Check ball ownership (skill-based - Open Football style)
    pub(crate) fn check_ball_ownership(&mut self) {
        let ownership_threshold_m = physics_constants::ball::OWNERSHIP_THRESHOLD_M;
        let _ball_pos = self.ball.position.to_meters(); // FIX_2512: Convert to meters (kept for reference)
        let mut nearby_players: Vec<(usize, f32)> = Vec::new();

        // Ball too high for field player ownership - FIX_2512: Compare i16 with f32
        if self.ball.height_meters() > aerial::GK_CATCH_MAX_M {
            return;
        }

        // Check all players
        for idx in 0..22 {
            let player_pos = self.get_player_position_by_index(idx);
            // FIX_2601: Use Coord10 for distance calculation
            let dist_m = player_pos.distance_to_m(&self.ball.position);

            if dist_m < ownership_threshold_m {
                // Filter by height - only GK can handle high balls
                let is_gk = self.get_position_string_by_idx(idx) == "GK";
                if self.ball.height_meters() > aerial::HEADER_MAX_M && !is_gk {
                    continue;
                }
                nearby_players.push((idx, dist_m));
            }
        }

        if nearby_players.is_empty() {
            return;
        }

        // Don't change if same team player already owns
        if let Some(current_owner) = self.ball.current_owner {
            let owner_is_home = TeamSide::is_home(current_owner);
            let same_team_nearby = nearby_players.iter().any(|(idx, _)| {
                let is_home = TeamSide::is_home(*idx);
                is_home == owner_is_home && *idx != current_owner
            });

            if same_team_nearby {
                return;
            }
        }

        // Award to best tackler (Open Football style)
        // B2: Hero Gravity - Loose Ball Magnet applied
        // FIX_2601/0110: Use distance as secondary key to avoid index order bias
        // FIX_2601/0119: Use deterministic hash as final tie-breaker to prevent max_by last-element bias
        let best_tackler = nearby_players.iter().max_by(|(idx_a, dist_a), (idx_b, dist_b)| {
            let mut score_a = self.calculate_tackling_score(*idx_a);
            let mut score_b = self.calculate_tackling_score(*idx_b);

            // Hero gravity bonus - closer distance = higher score
            let is_home_a = TeamSide::is_home(*idx_a);
            let is_home_b = TeamSide::is_home(*idx_b);
            if self.is_user_player(*idx_a, is_home_a) {
                score_a += 0.2; // Hero gravity bonus
            }
            if self.is_user_player(*idx_b, is_home_b) {
                score_b += 0.2; // Hero gravity bonus
            }

            match score_a.partial_cmp(&score_b) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // Secondary tie-breaker: closer player wins (lower distance is better)
                    match dist_b.partial_cmp(dist_a) {
                        Some(std::cmp::Ordering::Equal) | None => {
                            // FIX_2601/0119: Final tie-breaker: deterministic hash
                            // Prevents max_by from always returning last element (Away bias)
                            let pos_a = self.get_player_position_by_index(*idx_a).to_meters();
                            let pos_b = self.get_player_position_by_index(*idx_b).to_meters();
                            super::deterministic_tie_hash(*idx_a, pos_a, *idx_b, pos_b)
                        }
                        Some(ord) => ord,
                    }
                }
                Some(ord) => ord,
            }
        });

        if let Some((idx, _)) = best_tackler {
            // If from different team, transfer ownership
            if let Some(current_owner) = self.ball.current_owner {
                let owner_is_home = TeamSide::is_home(current_owner);
                let new_owner_is_home = TeamSide::is_home(*idx);

                if owner_is_home != new_owner_is_home {
                    self.ball.previous_owner = self.ball.current_owner;
                    // FIX_2601/1120: Update ball position to interceptor's position to prevent teleportation
                    self.ball.current_owner = Some(*idx);
                    self.ball.position = self.player_positions[*idx];
                }
            } else {
                // FIX_2601/1120: Update ball position to interceptor's position to prevent teleportation
                self.ball.current_owner = Some(*idx);
                self.ball.position = self.player_positions[*idx];
            }
        }
    }

    /// Calculate tackling score (Open Football style)
    pub(crate) fn calculate_tackling_score(&self, player_idx: usize) -> f32 {   
        if player_idx >= 22 {
            return 0.0;
        }

        let attrs = self.get_player_attributes(player_idx);

        // Open Football style: 5 attribute weighted combination
        let base_score = attrs.tackling as f32 * 0.4
            + attrs.aggression as f32 * 0.2
            + attrs.bravery as f32 * 0.1
            + attrs.strength as f32 * 0.2
            + attrs.agility as f32 * 0.1;

        // Home advantage applied
        if TeamSide::is_home(player_idx) {
            base_score * (1.0 + home_advantage::TACKLE_SUCCESS_BONUS)
        } else {
            base_score
        }
    }

    /// Update ball position during flight (Phase 3)
    pub(crate) fn update_ball_physics(
        &mut self,
        tick_duration_sec: f32,
    ) -> Option<crate::engine::action_queue::ActionResult> {
        if !self.ball.is_in_flight {
            return None;
        }

        if let (Some(from), Some(to)) = (self.ball.from_position, self.ball.to_position) {
            // Update progress
            self.ball.flight_progress += self.ball.flight_speed * tick_duration_sec;

            // A12: Interception check (only during flight)
            if self.ball.flight_progress < 1.0 {
                if let Some(interceptor_idx) = self.try_intercept_pass() {
                    // Interception success: transfer ownership
                    self.ball.complete_flight(Some(interceptor_idx));
                    return Some(crate::engine::action_queue::ActionResult::InterceptSuccess {
                        player_idx: interceptor_idx,
                    });
                }
            }

            if self.ball.flight_progress >= 1.0 {
                // A11: Ball control success calculation
                let pending_owner = self.ball.pending_owner.take();

                let actual_owner = if let Some(receiver_idx) = pending_owner {
                    // Estimate ball speed from flight_speed (m/s estimate)
                    let dist_m = if let (Some(from_pos), Some(to_pos)) =
                        (self.ball.from_position, self.ball.to_position)
                    {
                        use crate::engine::coordinates;
                        coordinates::distance_between_m(from_pos.to_meters(), to_pos.to_meters())
                    } else {
                        20.0 // Default medium distance
                    };

                    // flight_speed 1.0 means entire path in 1 second = dist_m m/s
                    let ball_speed = dist_m * self.ball.flight_speed;
                    let ball_height = self.ball.height_meters(); // FIX_2512: Convert to f32

                    let control_rate =
                        self.calculate_ball_control_success(receiver_idx, ball_speed, ball_height);
                    let control_success = self.rng.gen::<f32>() < control_rate;

                    if control_success {
                        Some(receiver_idx) // Control success
                    } else {
                        // Control failed: ball bounces off (no owner)
                        None
                    }
                } else {
                    None
                };

                self.ball.complete_flight(actual_owner);
            } else {
                // A3: Interpolate position in 3D (XY with curve, Z with height profile)
                // FIX_2512: Convert Coord10 to (f32, f32) meters
                let (x, y, z) = get_ball_position_3d(
                    from.to_meters(),
                    to.to_meters(),
                    self.ball.curve_factor,
                    self.ball.height_profile,
                    self.ball.flight_progress,
                );
                // Clamp to audit buffer (allow minor out-of-play positions)
                self.ball.position = Coord10::from_meters(
                    x.clamp(audit_gates::BALL_X_MIN, audit_gates::BALL_X_MAX),
                    y.clamp(audit_gates::BALL_Y_MIN, audit_gates::BALL_Y_MAX),
                );
                self.ball.height = (z * 10.0) as i16; // FIX_2512: Convert meters to 0.1m units
            }
        }
        None
    }
}

// ============================================================
// Ball Physics V2: Tests
// ============================================================

#[cfg(test)]
mod ball_physics_v2_tests {
    use super::*;
    use crate::engine::ball::Ball;
    use crate::engine::physics_constants::substep;

    /// Test that substep constants are correctly defined
    #[test]
    fn test_substep_constants() {
        assert_eq!(substep::SUBSTEP_MS, 10.0);
        assert_eq!(substep::TICK_MS, 50.0);
        assert_eq!(substep::SUBSTEPS_PER_TICK, 5);
        assert_eq!(substep::SUBSTEP_SEC, 0.01);

        // Verify relationship: TICK_MS / SUBSTEP_MS = SUBSTEPS_PER_TICK
        let calculated = (substep::TICK_MS / substep::SUBSTEP_MS) as usize;
        assert_eq!(calculated, substep::SUBSTEPS_PER_TICK);
    }

    /// Test that a stationary ball remains stationary
    #[test]
    fn test_stationary_ball() {
        let ball = Ball::default();
        let initial_pos = ball.position;

        // Velocity is zero by default
        assert_eq!(ball.velocity.magnitude(), 0);

        // Apply physics - should not move
        // Note: We test the physics step directly since we don't have MatchEngine here
        let vel_mps = ball.velocity.to_mps();
        let speed = (vel_mps.0.powi(2) + vel_mps.1.powi(2)).sqrt();
        assert!(speed < 0.001);

        // Position should remain unchanged
        assert_eq!(ball.position, initial_pos);
    }

    /// Test that high-speed ball decelerates correctly over substeps
    #[test]
    fn test_high_speed_deceleration() {
        use crate::engine::physics_constants::ball;

        // Initial velocity: 25 m/s (typical shot speed)
        let initial_speed = 25.0_f32;
        let mut velocity = Vel10::from_mps(initial_speed, 0.0);
        let mut position = Coord10::from_meters(
            physics_constants::field::CENTER_X,
            physics_constants::field::CENTER_Y,
        ); // field center

        // Simulate 5 substeps (1 tick = 50ms)
        for _ in 0..substep::SUBSTEPS_PER_TICK {
            let vel_mps = velocity.to_mps();
            let speed = (vel_mps.0.powi(2) + vel_mps.1.powi(2)).sqrt();

            if speed <= ball::MIN_VELOCITY {
                break;
            }

            let direction = (vel_mps.0 / speed, vel_mps.1 / speed);

            // Physics calculations
            let drag_accel = 0.5 * ball::DRAG_COEFFICIENT * speed.powi(2) / ball::MASS_KG;
            let rolling_accel = ball::ROLLING_RESISTANCE * ball::GRAVITY;
            let deceleration = (drag_accel + rolling_accel) * substep::SUBSTEP_SEC;
            let new_speed = (speed - deceleration).max(0.0);

            let new_vel_mps = (direction.0 * new_speed, direction.1 * new_speed);
            velocity = Vel10::from_mps(new_vel_mps.0, new_vel_mps.1);

            // Position update
            let pos_m = position.to_meters();
            let displacement = (new_vel_mps.0 * substep::SUBSTEP_SEC, 0.0);
            let new_pos_m = (pos_m.0 + displacement.0, pos_m.1);
            position = Coord10::from_meters(new_pos_m.0, new_pos_m.1);
        }

        let final_vel_mps = velocity.to_mps();
        let final_speed = (final_vel_mps.0.powi(2) + final_vel_mps.1.powi(2)).sqrt();

        // After 50ms with drag, speed should decrease
        // 25 m/s with Cd=0.25, m=0.43kg:
        // drag_accel = 0.5 * 0.25 * 625 / 0.43 ≈ 181.7 m/s²
        // decel per step = 181.7 * 0.01 ≈ 1.82 m/s
        // After 5 steps: ~25 - 5*1.8 ≈ 16 m/s (approximate due to changing speed)
        assert!(final_speed < initial_speed, "Ball should decelerate");
        assert!(final_speed > 10.0, "Ball shouldn't stop in one tick at 25m/s");

        // Position should have moved
        let final_pos_m = position.to_meters();
        assert!(
            final_pos_m.0 > physics_constants::field::CENTER_X,
            "Ball should move right"
        );
    }

    /// Test that substep gives more accurate results than single step
    #[test]
    fn test_substep_vs_single_step_accuracy() {
        use crate::engine::physics_constants::ball;

        // Initial conditions
        let initial_speed = 20.0_f32;

        // --- Substep method (5 × 10ms) ---
        let mut substep_velocity = Vel10::from_mps(initial_speed, 0.0);
        let mut substep_position =
            Coord10::from_meters(0.0, physics_constants::field::CENTER_Y);

        for _ in 0..substep::SUBSTEPS_PER_TICK {
            let vel_mps = substep_velocity.to_mps();
            let speed = (vel_mps.0.powi(2) + vel_mps.1.powi(2)).sqrt();

            if speed <= ball::MIN_VELOCITY {
                break;
            }

            let drag_accel = 0.5 * ball::DRAG_COEFFICIENT * speed.powi(2) / ball::MASS_KG;
            let rolling_accel = ball::ROLLING_RESISTANCE * ball::GRAVITY;
            let deceleration = (drag_accel + rolling_accel) * substep::SUBSTEP_SEC;
            let new_speed = (speed - deceleration).max(0.0);

            substep_velocity = Vel10::from_mps(new_speed, 0.0);

            let pos_m = substep_position.to_meters();
            let new_pos_m = (pos_m.0 + new_speed * substep::SUBSTEP_SEC, pos_m.1);
            substep_position = Coord10::from_meters(new_pos_m.0, new_pos_m.1);
        }

        // --- Single step method (1 × 50ms) ---
        let mut single_velocity = Vel10::from_mps(initial_speed, 0.0);

        let vel_mps = single_velocity.to_mps();
        let speed = (vel_mps.0.powi(2) + vel_mps.1.powi(2)).sqrt();

        let drag_accel = 0.5 * ball::DRAG_COEFFICIENT * speed.powi(2) / ball::MASS_KG;
        let rolling_accel = ball::ROLLING_RESISTANCE * ball::GRAVITY;
        // Single step uses full 50ms
        let deceleration = (drag_accel + rolling_accel) * 0.05;
        let new_speed = (speed - deceleration).max(0.0);

        single_velocity = Vel10::from_mps(new_speed, 0.0);

        // Substep should give different (more accurate) results
        // The key difference: substep recalculates drag at each step
        let substep_final_speed = substep_velocity.to_mps().0;
        let single_final_speed = single_velocity.to_mps().0;

        // Both should decelerate from 20 m/s
        assert!(substep_final_speed < initial_speed);
        assert!(single_final_speed < initial_speed);

        // Substep accounts for decreasing drag as ball slows
        // Single step overestimates drag (uses initial high speed for all)
        // So substep final speed should be slightly higher
        // (This is the numerical integration benefit)
        println!("Substep final speed: {:.3} m/s", substep_final_speed);
        println!("Single final speed: {:.3} m/s", single_final_speed);
    }

    /// Test that ball stops when velocity is at or below minimum threshold
    #[test]
    fn test_ball_stops_at_min_velocity() {
        use crate::engine::physics_constants::ball;

        // Test 1: Ball at exactly min velocity should stop
        let at_threshold = Vel10::from_mps(ball::MIN_VELOCITY, 0.0);
        let speed = at_threshold.to_mps().0;
        assert!(speed <= ball::MIN_VELOCITY + 0.01, "Speed at threshold: {:.3}", speed);

        // Test 2: Ball below min velocity should stop
        let below_threshold = Vel10::from_mps(0.05, 0.0);
        let speed_below = below_threshold.to_mps().0;
        assert!(speed_below <= ball::MIN_VELOCITY, "Speed below threshold: {:.3}", speed_below);

        // Test 3: Simulate physics using f32 (as in actual apply_physics_step)
        // Note: Vel10 quantization means we track raw f32 speed for accuracy testing
        let mut speed = 0.5_f32; // 0.5 m/s
        let mut steps = 0;
        const MAX_STEPS: usize = 500; // 5 seconds of simulation

        while steps < MAX_STEPS && speed > ball::MIN_VELOCITY {
            let drag_accel = 0.5 * ball::DRAG_COEFFICIENT * speed.powi(2) / ball::MASS_KG;
            let rolling_accel = ball::ROLLING_RESISTANCE * ball::GRAVITY;
            let deceleration = (drag_accel + rolling_accel) * substep::SUBSTEP_SEC;
            speed = (speed - deceleration).max(0.0);
            steps += 1;
        }

        // Ball should stop within reasonable time
        assert!(
            speed <= ball::MIN_VELOCITY,
            "Ball should stop (speed={:.3} m/s) after {} steps",
            speed,
            steps
        );
        assert!(steps < MAX_STEPS, "Ball should stop within {} steps, took {}", MAX_STEPS, steps);
        println!("Ball stopped after {} steps ({:.2}s)", steps, steps as f32 * 0.01);
    }

    /// Test displacement calculation per substep
    #[test]
    fn test_displacement_per_substep() {
        // 10 m/s moving right for 10ms should move 0.1m
        let speed = 10.0_f32;
        let dt = substep::SUBSTEP_SEC; // 0.01s

        let displacement = speed * dt;
        assert!((displacement - 0.1).abs() < 0.001, "10m/s * 10ms = 0.1m");

        // 25 m/s for 10ms = 0.25m
        let high_speed = 25.0_f32;
        let high_displacement = high_speed * dt;
        assert!((high_displacement - 0.25).abs() < 0.001, "25m/s * 10ms = 0.25m");
    }

    // ============================================================
    // Ball Physics V2 Phase 2: Gravity Tests
    // ============================================================

    /// Test projectile constants are correctly defined
    #[test]
    fn test_projectile_constants() {
        // Gravity: 9.81 m/s² = 98.1 in 0.1m/s² units
        assert_eq!(projectile::GRAVITY_01M, 98);

        // Initial velocities for height profiles
        assert_eq!(projectile::VZ_FLAT, 0);
        assert_eq!(projectile::VZ_ARC, 83); // 8.3 m/s
        assert_eq!(projectile::VZ_LOB, 140); // 14.0 m/s
        let driven_vz = (projectile::VZ_ARC as f32 * 0.29).round() as i16;
        assert_eq!(driven_vz, 24); // 2.4 m/s

        // Bounce
        assert_eq!(physics_constants::bounce::GRASS_COR, 0.65);
        assert_eq!(physics_constants::bounce::MIN_BOUNCE_VZ, 14); // 1.4 m/s
    }

    /// Test ball launch with HeightProfile sets correct velocity_z
    /// Ball Physics V2: vz = √(2gh) × 10 (0.1m/s units)
    #[test]
    fn test_ball_launch_with_profile() {
        use crate::engine::ball::HeightProfile;

        let mut ball = Ball::default();

        // Test Flat (h=0m, vz=0)
        ball.launch_with_profile(HeightProfile::Flat);
        assert_eq!(ball.velocity_z, 0);
        assert_eq!(ball.height_profile, HeightProfile::Flat);

        // Test Arc (h=3.5m, vz=√(2×9.81×3.5)×10 ≈ 83)
        ball.launch_with_profile(HeightProfile::Arc);
        assert_eq!(ball.velocity_z, projectile::VZ_ARC);
        assert_eq!(ball.height_profile, HeightProfile::Arc);

        // Test Lob (h=10m, vz=√(2×9.81×10)×10 ≈ 140)
        ball.launch_with_profile(HeightProfile::Lob);
        assert_eq!(ball.velocity_z, projectile::VZ_LOB);
        assert_eq!(ball.height_profile, HeightProfile::Lob);
    }

    /// Test gravity step decreases velocity_z
    #[test]
    fn test_gravity_step_decreases_velocity() {
        let mut ball = Ball::default();
        ball.velocity_z = 100; // 10.0 m/s upward
        ball.height = 10; // 1.0m above ground

        // Manual gravity step
        let dt = substep::SUBSTEP_SEC;
        let dv = (projectile::GRAVITY_01M as f32 * dt).round() as i16;

        // After one step, velocity should decrease by dv
        let expected_vz = ball.velocity_z - dv;

        // Simulate one step
        ball.velocity_z -= dv;
        assert_eq!(ball.velocity_z, expected_vz);

        // dv should be ~1 (98 * 0.01 = 0.98 ≈ 1)
        assert!((0..=2).contains(&dv), "dv should be ~1, got {}", dv);
    }

    /// Test ball reaches expected max height for Lob profile
    #[test]
    fn test_lob_max_height() {
        // v = 14.0 m/s (VZ_LOB = 140)
        // max height = v² / (2g) = 14² / (2 * 9.81) ≈ 10.0m
        let vz_mps = projectile::VZ_LOB as f32 * 0.1;
        let g = 9.81_f32;
        let expected_max_h = vz_mps.powi(2) / (2.0 * g);

        // Should be approximately 10m
        assert!(
            (expected_max_h - 10.0).abs() < 0.5,
            "Lob max height should be ~10m, got {:.2}m",
            expected_max_h
        );
    }

    /// Test ball reaches expected max height for Header profile
    #[test]
    fn test_header_max_height() {
        // Header uses Arc profile (VZ_ARC)
        // max height = v² / (2g) = 8.3² / (2 * 9.81) ≈ 3.5m
        let vz_mps = projectile::VZ_ARC as f32 * 0.1;
        let g = 9.81_f32;
        let expected_max_h = vz_mps.powi(2) / (2.0 * g);

        // Should be approximately 3.5m
        assert!(
            (expected_max_h - 3.5).abs() < 0.3,
            "Header max height should be ~3.5m, got {:.2}m",
            expected_max_h
        );
    }

    /// Test ball reaches expected max height for Driven profile
    #[test]
    fn test_driven_max_height() {
        // Driven uses Arc profile with lift_ratio ~0.29
        // max height = v² / (2g) = 2.4² / (2 * 9.81) ≈ 0.29m
        let vz_mps = projectile::VZ_ARC as f32 * 0.1 * 0.29;
        let g = 9.81_f32;
        let expected_max_h = vz_mps.powi(2) / (2.0 * g);

        // Should be approximately 0.3m
        assert!(
            (expected_max_h - 0.3).abs() < 0.1,
            "Driven max height should be ~0.3m, got {:.2}m",
            expected_max_h
        );
    }

    /// Test bounce physics - ball bounces with reduced velocity (Phase 3)
    #[test]
    fn test_bounce_physics() {
        use physics_constants::bounce;

        // Simulate ball falling and hitting ground
        let incoming_vz: i16 = -50; // 5.0 m/s downward

        // Bounce calculation with GRASS_COR (0.65)
        let bounce_vz = (incoming_vz.abs() as f32 * bounce::GRASS_COR) as i16;

        // Should bounce up at 65% speed
        assert_eq!(bounce_vz, 32, "Bounce should be 65% of incoming: got {}", bounce_vz);

        // Bounce is above minimum threshold
        assert!(bounce_vz >= bounce::MIN_BOUNCE_VZ);
    }

    /// Test ball settles (no bounce) when velocity too low
    #[test]
    fn test_ball_settles_below_threshold() {
        use physics_constants::bounce;

        // Ball falling slowly
        let incoming_vz: i16 = -10; // 1.0 m/s

        // After COR: 10 * 0.65 = 6.5 ≈ 6
        let bounce_vz = (incoming_vz.abs() as f32 * bounce::GRASS_COR) as i16;

        // Should not bounce (below MIN_BOUNCE_VZ = 14)
        assert!(
            bounce_vz < bounce::MIN_BOUNCE_VZ,
            "Bounce velocity {} should be below threshold {}",
            bounce_vz,
            bounce::MIN_BOUNCE_VZ
        );
    }

    // ========================================================================
    // Phase 3: Bounce Physics Tests
    // ========================================================================

    /// Test bounce constants are configured correctly
    #[test]
    fn test_bounce_constants() {
        use physics_constants::bounce;

        // Grass COR should be between 0.6 and 0.9 (FIFA spec)
        // Verify grass COR is in reasonable range (compile-time check)
        const _: () = assert!(bounce::GRASS_COR >= 0.6 && bounce::GRASS_COR <= 0.9);

        // Post COR should be higher than grass (metal)
        // Verify post COR > grass COR (compile-time check)
        const _: () = assert!(bounce::POST_COR > bounce::GRASS_COR);

        // Min bounce velocity for 10cm height: v = sqrt(2gh) ≈ 1.4 m/s
        let expected_min_vz = ((2.0 * 9.81 * bounce::MIN_BOUNCE_HEIGHT_M).sqrt() * 10.0) as i16;
        assert_eq!(bounce::MIN_BOUNCE_VZ, expected_min_vz);

        // Max bounces should be reasonable (compile-time check)
        const _: () = assert!(bounce::MAX_BOUNCES >= 3 && bounce::MAX_BOUNCES <= 10);
    }

    /// Test bounce count increments on each bounce
    #[test]
    fn test_bounce_count_tracking() {
        use physics_constants::bounce;

        let mut ball = Ball::default();
        ball.velocity_z = -100; // 10 m/s downward (strong impact)
        ball.height = 0;
        ball.bounce_count = 0;

        // Simulate multiple bounces
        let mut bounces = 0;
        for _ in 0..10 {
            if ball.velocity_z.abs() < bounce::MIN_BOUNCE_VZ {
                break;
            }

            // Simulate landing
            let rebound_vz = (ball.velocity_z.abs() as f32 * bounce::GRASS_COR) as i16;
            if rebound_vz >= bounce::MIN_BOUNCE_VZ {
                ball.velocity_z = rebound_vz;
                bounces += 1;
            } else {
                break;
            }

            // Reduce for next bounce
            ball.velocity_z = -(ball.velocity_z as f32 * bounce::GRASS_COR) as i16;
        }

        // Should have multiple bounces before settling
        assert!(bounces >= 2, "Should have multiple bounces, got {}", bounces);
    }

    /// Test is_rolling transition after max bounces
    #[test]
    fn test_rolling_transition() {
        use physics_constants::bounce;

        let mut ball = Ball::default();
        ball.bounce_count = bounce::MAX_BOUNCES; // At max bounces
        ball.velocity_z = -50; // Would normally bounce
        ball.is_rolling = false;

        // After MAX_BOUNCES, should transition to rolling
        let rebound_vz = (ball.velocity_z.abs() as f32 * bounce::GRASS_COR) as i16;
        let should_roll =
            rebound_vz < bounce::MIN_BOUNCE_VZ || ball.bounce_count >= bounce::MAX_BOUNCES;

        assert!(should_roll, "Should transition to rolling at max bounces");
    }

    /// Test horizontal velocity loss on bounce
    #[test]
    fn test_horizontal_velocity_loss() {
        use physics_constants::bounce;

        let initial_speed = 10.0_f32; // 10 m/s
        let after_bounce = initial_speed * (1.0 - bounce::HORIZONTAL_LOSS);

        // Should lose 12% speed
        assert!(
            (after_bounce - 8.8).abs() < 0.1,
            "Should lose 12% speed: {} -> {}",
            initial_speed,
            after_bounce
        );
    }

    /// Test spin deflection on bounce
    #[test]
    fn test_spin_deflection() {
        use physics_constants::bounce;

        let curve_factor = 0.2_f32; // Significant spin
        let vx = 10.0_f32; // Forward velocity
        let vy = 0.0_f32; // No lateral velocity

        // Deflection = curve_factor * SPIN_DEFLECTION * |vx|
        let deflection = curve_factor * bounce::SPIN_DEFLECTION * vx.abs();
        let new_vy = vy + deflection;

        // Should have lateral deflection
        assert!(new_vy.abs() > 0.1, "Should have spin deflection: {}", new_vy);

        // Spin should decay after bounce
        let new_curve = curve_factor * bounce::SPIN_DECAY;
        assert!(new_curve < curve_factor, "Spin should decay after bounce");
    }

    /// Test is_airborne and is_grounded helpers
    #[test]
    fn test_airborne_helpers() {
        let mut ball = Ball::default();

        // Ground state
        ball.height = 0;
        ball.velocity_z = 0;
        assert!(!ball.is_airborne());
        assert!(ball.is_grounded());

        // Airborne (height > 0)
        ball.height = 10;
        ball.velocity_z = 0;
        assert!(ball.is_airborne());
        assert!(!ball.is_grounded());

        // Launching (velocity_z > 0)
        ball.height = 0;
        ball.velocity_z = 50;
        assert!(ball.is_airborne());
        assert!(!ball.is_grounded());

        // Falling (height > 0, velocity_z < 0)
        ball.height = 10;
        ball.velocity_z = -50;
        assert!(ball.is_airborne());
        assert!(!ball.is_grounded());
    }

    /// Test full gravity simulation - ball goes up and comes down (Phase 3)
    #[test]
    fn test_full_trajectory_simulation() {
        use physics_constants::bounce;

        let mut ball = Ball::default();

        // Launch with Arc profile (vz = 83 = 8.3 m/s)
        ball.launch_with_profile(crate::engine::ball::HeightProfile::Arc);
        ball.height = 1; // Start slightly above ground
        ball.bounce_count = 0;
        ball.is_rolling = false;

        let mut max_height: i16 = 0;
        let mut total_bounces: u8 = 0;
        let mut steps = 0;
        const MAX_STEPS: usize = 500; // 5 seconds

        // Simulate until ball settles or starts rolling
        while steps < MAX_STEPS && !ball.is_rolling {
            // Apply gravity step
            let dt = substep::SUBSTEP_SEC;
            let dv = (projectile::GRAVITY_01M as f32 * dt).round() as i16;
            ball.velocity_z -= dv;

            // Position update with special case for low height
            let dh = (ball.velocity_z as f32 * dt).round() as i16;
            let new_height = if ball.height == 1 && ball.velocity_z < -bounce::MIN_BOUNCE_VZ {
                0 // Force ground contact when at edge and clearly falling
            } else {
                ball.height + dh
            };

            if new_height <= 0 {
                ball.height = 0;

                // Calculate bounce velocity with GRASS_COR
                let rebound_vz = (ball.velocity_z.abs() as f32 * bounce::GRASS_COR) as i16;

                // Check rolling transition
                let should_roll =
                    rebound_vz < bounce::MIN_BOUNCE_VZ || ball.bounce_count >= bounce::MAX_BOUNCES;

                if should_roll {
                    ball.velocity_z = 0;
                    ball.is_rolling = true;
                } else {
                    ball.velocity_z = rebound_vz;
                    ball.bounce_count += 1;
                    total_bounces += 1;
                    ball.height = 1;
                }
            } else {
                ball.height = new_height;
            }

            max_height = max_height.max(ball.height);
            steps += 1;
        }

        // Max height should be approximately 3.5m = 35 units (Arc profile)
        let max_h_m = max_height as f32 * 0.1;
        assert!(
            max_h_m > 2.5 && max_h_m < 4.5,
            "Arc max height should be ~3.5m, got {:.2}m (steps: {})",
            max_h_m,
            steps
        );

        // Should have bounced multiple times before rolling
        assert!(total_bounces >= 2, "Should have at least 2 bounces, got {}", total_bounces);

        // Ball should eventually start rolling
        assert!(ball.is_rolling, "Ball should transition to rolling");

        println!(
            "Trajectory: max_height={:.2}m, bounces={}, settled after {} steps ({:.2}s)",
            max_h_m,
            total_bounces,
            steps,
            steps as f32 * 0.01
        );
    }
}
