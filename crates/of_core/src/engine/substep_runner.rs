use super::physics_constants::field::{LENGTH_M, WIDTH_M};
/// substep_runner.rs
/// Phase 1.0.5: exec_substep() - Integration Phase (50ms)
///
/// Executes one 50ms physics substep from a PlanWindow.
/// Called 5 times per decision window (250ms / 50ms = 5).
use super::plan_window::{BallPlan, PlanWindow};
use super::player_physics::update_player_motion;
use super::timestep::SUBSTEP_DT;

/// Execute one 50ms substep
///
/// **Philosophy**: "Think slowly, move smoothly"
/// - Decision made once (250ms) → PlanWindow
/// - Execution 5 times (50ms each) → Smooth integration
///
/// **Parameters**:
/// - `plan`: Mutable PlanWindow (substeps_remaining will decrement)
/// - `player_positions`: Current positions (meters, will be updated)
/// - `player_velocities`: Current velocities (m/s, will be updated)
/// - `player_motion_params`: Physics parameters (ability-derived, base)
/// - `stamina`: Player stamina (0..1, affects physics)
/// - `ball_position`: Current ball position (meters, will be updated)
///
/// **Side Effects**:
/// - Updates player_positions via physics integration
/// - Updates player_velocities
/// - Updates ball_position (if in flight/rolling)
/// - Decrements plan.substeps_remaining
/// - Sets plan.end_now if ball goes out of play
///
/// **Returns**: true if substep completed, false if early termination
pub fn exec_substep(
    plan: &mut PlanWindow,
    player_positions: &mut [(f32, f32); 22],
    player_velocities: &mut [(f32, f32); 22],
    player_motion_params: &[crate::engine::player_motion_params::PlayerMotionParams; 22],
    stamina: &[f32; 22],
    ball_position: &mut (f32, f32),
) -> bool {
    // Check if plan is exhausted
    if plan.substeps_remaining == 0 {
        return false;
    }

    // ============================================================================
    // 1. Player Movement (50ms physics integration)
    // FIX_2601/0116: 2-Phase Batch Update to prevent index-based bias
    // ============================================================================

    // ========== Phase 1: Calculate all new positions (Snapshot-based) ==========
    let positions_snapshot = *player_positions;
    let velocities_snapshot = *player_velocities;

    let mut new_positions: [(f32, f32); 22] = [(0.0, 0.0); 22];
    let mut new_velocities: [(f32, f32); 22] = [(0.0, 0.0); 22];

    for player_idx in 0..22 {
        // Get target from plan
        let target_m = plan.player_plans[player_idx].target_pos;

        // Convert normalized target to meters
        let target_m_abs = (target_m.0 * LENGTH_M, target_m.1 * WIDTH_M);

        // Current position (snapshot-based)
        let pos_m = (
            positions_snapshot[player_idx].0 * LENGTH_M,
            positions_snapshot[player_idx].1 * WIDTH_M,
        );

        // Current velocity (snapshot-based)
        let vel = velocities_snapshot[player_idx];

        // Ability→MotionParams SSOT (base) + runtime stamina scaling
        let base_params = &player_motion_params[player_idx];
        let stamina01 = stamina[player_idx];
        let params = crate::engine::player_motion_params::scale_by_stamina(base_params, stamina01, 0);

        // ★ KEY DIFFERENCE FROM OLD SYSTEM ★
        // Old: DT = 0.25 (one big jump, teleportation feel)
        // New: DT = 0.05 (5 small steps, smooth movement)
        let (new_pos_m, new_vel) = update_player_motion(
            pos_m,
            vel,
            target_m_abs,
            SUBSTEP_DT, // ★ 0.05 instead of 0.25 ★
            &params,
        );

        // Clamp to field boundaries
        let clamped_pos_m = (new_pos_m.0.clamp(0.0, LENGTH_M), new_pos_m.1.clamp(0.0, WIDTH_M));

        // Store in temp arrays (don't apply yet!)
        new_positions[player_idx] = (clamped_pos_m.0 / LENGTH_M, clamped_pos_m.1 / WIDTH_M);
        new_velocities[player_idx] = new_vel;
    }

    // ========== Phase 2: Batch Apply ==========
    *player_positions = new_positions;
    *player_velocities = new_velocities;

    // ============================================================================
    // 2. Ball Physics (50ms integration)
    // ============================================================================

    match &plan.ball_plan {
        BallPlan::Controlled { owner_idx } => {
            // Ball follows owner
            *ball_position = player_positions[*owner_idx as usize];
        }
        BallPlan::InFlight { .. } => {
            // TODO: Integrate ball flight physics (parabola)
            // For MVP-1: Keep ball stationary
        }
        BallPlan::Rolling { pos, velocity, friction } => {
            // Simple friction-based rolling
            let v_mag = (velocity.0.powi(2) + velocity.1.powi(2)).sqrt();
            if v_mag > 0.01 {
                let v_dir = (velocity.0 / v_mag, velocity.1 / v_mag);
                let new_v_mag = (v_mag - friction * SUBSTEP_DT).max(0.0);
                let new_velocity = (v_dir.0 * new_v_mag, v_dir.1 * new_v_mag);

                let new_pos =
                    (pos.0 + new_velocity.0 * SUBSTEP_DT, pos.1 + new_velocity.1 * SUBSTEP_DT);

                *ball_position = new_pos;
                // TODO: Update plan.ball_plan.velocity (need mutable access)
            }
        }
        BallPlan::Bouncing { .. } => {
            // TODO: Integrate bounce physics
        }
        BallPlan::OutOfPlay { .. } => {
            // Ball is out, no movement
            plan.end_now = true;
            return false;
        }
    }

    // ============================================================================
    // 3. Decrement substeps counter
    // ============================================================================

    plan.substeps_remaining -= 1;

    // ============================================================================
    // 4. Early termination checks
    // ============================================================================

    // Check if ball went out of bounds
    if ball_position.0 < 0.0
        || ball_position.0 > 1.0
        || ball_position.1 < 0.0
        || ball_position.1 > 1.0
    {
        plan.end_now = true;
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::player_motion_params::PlayerMotionParams;

    fn default_motion_params() -> [PlayerMotionParams; 22] {
        std::array::from_fn(|_| PlayerMotionParams::default())
    }

    #[test]
    fn test_exec_substep_decrements_counter() {
        let mut plan = PlanWindow::default();
        plan.substeps_remaining = 5;

        let mut positions = [(0.5, 0.5); 22];
        let mut velocities = [(0.0, 0.0); 22];
        let motion_params = default_motion_params();
        let stamina = [1.0; 22];
        let mut ball_pos = (0.5, 0.5);

        let result = exec_substep(
            &mut plan,
            &mut positions,
            &mut velocities,
            &motion_params,
            &stamina,
            &mut ball_pos,
        );

        assert!(result);
        assert_eq!(plan.substeps_remaining, 4);
    }

    #[test]
    fn test_exec_substep_exhausted_plan() {
        let mut plan = PlanWindow::default();
        plan.substeps_remaining = 0;

        let mut positions = [(0.5, 0.5); 22];
        let mut velocities = [(0.0, 0.0); 22];
        let motion_params = default_motion_params();
        let stamina = [1.0; 22];
        let mut ball_pos = (0.5, 0.5);

        let result = exec_substep(
            &mut plan,
            &mut positions,
            &mut velocities,
            &motion_params,
            &stamina,
            &mut ball_pos,
        );

        assert!(!result);
    }

    #[test]
    fn test_exec_substep_ball_follows_owner() {
        let mut plan = PlanWindow::default();
        plan.substeps_remaining = 5;
        plan.ball_plan = BallPlan::Controlled { owner_idx: 5 };

        let mut positions = [(0.5, 0.5); 22];
        positions[5] = (0.6, 0.4); // Owner at different position
        let mut velocities = [(0.0, 0.0); 22];
        let motion_params = default_motion_params();
        let stamina = [1.0; 22];
        let mut ball_pos = (0.5, 0.5);

        exec_substep(
            &mut plan,
            &mut positions,
            &mut velocities,
            &motion_params,
            &stamina,
            &mut ball_pos,
        );

        // Ball should follow owner
        assert_eq!(ball_pos, positions[5]);
    }

    #[test]
    fn test_exec_substep_out_of_play() {
        let mut plan = PlanWindow::default();
        plan.substeps_remaining = 5;
        plan.ball_plan = BallPlan::OutOfPlay { reason: "goal".to_string() };

        let mut positions = [(0.5, 0.5); 22];
        let mut velocities = [(0.0, 0.0); 22];
        let motion_params = default_motion_params();
        let stamina = [1.0; 22];
        let mut ball_pos = (0.5, 0.5);

        let result = exec_substep(
            &mut plan,
            &mut positions,
            &mut velocities,
            &motion_params,
            &stamina,
            &mut ball_pos,
        );

        assert!(!result);
        assert!(plan.end_now);
    }
}
