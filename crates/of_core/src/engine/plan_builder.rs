/// plan_builder.rs
/// Phase 1.0.5: build_plan_window() - Decision Phase (250ms)
///
/// Extracts existing decision logic from update_positioning_tick()
/// into a PlanWindow structure for dual timestep execution.
use super::plan_window::{BallPlan, PlanWindow, PlayerPlan, PlayerPlanKind};
use super::timestep::SUBSTEPS_PER_DECISION;

/// Build PlanWindow from current match state (MVP-1: Minimal change)
///
/// **Philosophy**: Keep decision logic IDENTICAL to current 250ms tick.
/// Only change: Output goes into PlanWindow instead of direct execution.
///
/// **Parameters**:
/// - `current_tick`: Current 250ms tick number
/// - `player_positions`: Current player positions (normalized 0..1)
/// - `ball_position`: Current ball position (normalized 0..1)
/// - `ball_owner`: Current ball owner (None if loose ball)
///
/// **Returns**: PlanWindow containing:
/// - player_plans[22]: Target positions for each player
/// - ball_plan: Ball state (Controlled/InFlight/Rolling/etc)
/// - substeps_remaining: Always 5 for new plan
///
/// **Implementation Note**: For MVP-1, we copy existing positioning engine logic
/// without changes. The only difference is we store results in PlanWindow
/// instead of applying them immediately.
pub fn build_plan_window(
    current_tick: u64,
    player_positions: &[(f32, f32); 22],
    ball_position: (f32, f32),
    ball_owner: Option<u8>,
    // TODO: Add positioning_engine, player_objectives, etc. when integrating
) -> PlanWindow {
    let mut plan = PlanWindow::default();
    plan.start_tick_250 = current_tick;
    plan.substeps_remaining = SUBSTEPS_PER_DECISION;

    // ============================================================================
    // MVP-1: Stub implementation (to be replaced with actual positioning logic)
    // ============================================================================

    // For now, each player's target is their current position (no movement)
    // This will be replaced with actual positioning_engine.calculate_target_positions()
    for (i, &pos) in player_positions.iter().enumerate() {
        plan.player_plans[i] = PlayerPlan {
            kind: PlayerPlanKind::IdleHold,
            target_pos: pos,            // MVP-1: Stay in place
            desired_facing: (1.0, 0.0), // Default: Face right
            max_speed: 7.0,             // Average player speed (will use actual stats later)
            accel: 5.0,
            decel: 8.0,
            flags: 0,
            mark_target_idx: -1,
        };
    }

    // Ball plan
    plan.ball_plan = if let Some(owner_idx) = ball_owner {
        BallPlan::Controlled { owner_idx }
    } else {
        // Loose ball (simplified for MVP-1)
        BallPlan::Rolling {
            pos: ball_position,
            velocity: (0.0, 0.0), // Stationary for now
            friction: 3.0,        // m/s²
        }
    };

    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_plan_window_creates_valid_plan() {
        let positions = [(0.5, 0.5); 22];
        let ball_pos = (0.5, 0.5);
        let ball_owner = Some(0);

        let plan = build_plan_window(100, &positions, ball_pos, ball_owner);

        assert_eq!(plan.start_tick_250, 100);
        assert_eq!(plan.substeps_remaining, 5);
        assert_eq!(plan.player_plans.len(), 22);

        // Ball should be controlled
        match plan.ball_plan {
            BallPlan::Controlled { owner_idx } => assert_eq!(owner_idx, 0),
            _ => panic!("Expected Controlled ball"),
        }
    }

    #[test]
    fn test_build_plan_window_loose_ball() {
        let positions = [(0.5, 0.5); 22];
        let ball_pos = (0.6, 0.4);
        let ball_owner = None;

        let plan = build_plan_window(50, &positions, ball_pos, ball_owner);

        // Ball should be rolling
        match plan.ball_plan {
            BallPlan::Rolling { pos, .. } => assert_eq!(pos, ball_pos),
            _ => panic!("Expected Rolling ball"),
        }
    }

    #[test]
    fn test_mvp1_players_stay_in_place() {
        let positions = [(0.3, 0.4); 22];
        let plan = build_plan_window(0, &positions, (0.5, 0.5), Some(0));

        // MVP-1: Each player's target should be their current position
        for (i, player_plan) in plan.player_plans.iter().enumerate() {
            assert_eq!(player_plan.target_pos, positions[i]);
            assert_eq!(player_plan.kind, PlayerPlanKind::IdleHold);
        }
    }
}
