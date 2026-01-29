//! Main decision function for off-ball system

use super::candidates::generate_candidates;
use super::resolver::{check_collision, resolve_collision};
use super::scheduler::{apply_force_expire_triggers, get_ttl_for_phase, select_players_for_decision};
use super::scorer::{evaluate_candidate, select_best_candidate};
use super::types::*;

/// Main entry point for off-ball decision updates.
///
/// Called each tick after on-ball decisions but before positioning_engine.
pub fn update_offball_decisions(
    objectives: &mut [OffBallObjective; 22],
    player_positions: &[(f32, f32); 22],
    player_staminas: &[f32; 22],
    base_positions: &[(f32, f32); 22],
    ball_pos: (f32, f32),
    ball_vel: (f32, f32),
    ball_owner: Option<usize>,
    possession_team: u8,
    is_home_attacking_right: bool,
    ticks_since_transition: u64,
    possession_changed: bool,
    current_tick: u64,
    config: &OffBallConfig,
) -> usize {
    // Step 1: Apply force expire triggers
    apply_force_expire_triggers(
        objectives,
        player_positions,
        player_staminas,
        ball_pos,
        possession_changed,
        current_tick,
    );

    // Step 2: Determine game phase
    let is_transition = ticks_since_transition < TRANSITION_WINDOW_TICKS;
    let phase = determine_phase(possession_team, is_transition, ticks_since_transition);

    // Step 3: Select players who need to decide
    let players_to_decide = select_players_for_decision(
        objectives,
        player_positions,
        ball_pos,
        ball_owner,
        is_transition,
        current_tick,
    );

    let decisions_made = players_to_decide.len();

    // Step 4: Make decisions for selected players
    for &player_idx in &players_to_decide {
        let is_home_player = player_idx < 11;
        let player_possession = if is_home_player {
            possession_team == 0
        } else {
            possession_team == 1
        };

        // Determine attack direction for this player
        let attack_direction = if is_home_player {
            if is_home_attacking_right { 1.0 } else { -1.0 }
        } else {
            if is_home_attacking_right { -1.0 } else { 1.0 }
        };

        // Adjust phase based on player's team
        let player_phase = if player_possession {
            if is_transition && ticks_since_transition < 8 {
                GamePhase::TransitionWin
            } else {
                GamePhase::Attacking
            }
        } else {
            if is_transition && ticks_since_transition < 8 {
                GamePhase::TransitionLoss
            } else {
                GamePhase::Defending
            }
        };

        // Build context
        let (px, py) = player_positions[player_idx];
        let (bx, by) = base_positions[player_idx];

        // Estimate defensive line (simplified)
        let defensive_line_x = estimate_defensive_line(
            player_positions,
            if is_home_player { 1 } else { 0 }, // Opponent's positions
            attack_direction,
        );

        // FIX_2601/1126: Calculate line role and anchor for line spacing enforcement
        let player_line_role = get_player_line_role(player_idx);
        let is_attacking = player_phase == GamePhase::Attacking || player_phase == GamePhase::TransitionWin;
        let line_anchor_x = calculate_line_anchor_x(
            ball_pos.0,
            player_line_role,
            is_attacking,
            attack_direction,
        );

        let ctx = OffBallContext {
            ball_x: ball_pos.0,
            ball_y: ball_pos.1,
            ball_vx: ball_vel.0,
            ball_vy: ball_vel.1,
            possession_team,
            phase: player_phase,
            player_idx,
            player_x: px,
            player_y: py,
            player_vx: 0.0, // Not used in v1
            player_vy: 0.0,
            player_stamina: player_staminas[player_idx],
            base_x: bx,
            base_y: by,
            is_home_team: is_home_player,
            attack_direction,
            defensive_line_x,
            ticks_since_transition,
            current_tick,
            player_line_role,  // FIX_2601/1126
            line_anchor_x,     // FIX_2601/1126
        };

        // Generate and evaluate candidates
        let objective = decide_for_player(&ctx, objectives, current_tick, config);

        // Store result
        objectives[player_idx] = objective;
    }

    decisions_made
}

/// Make a decision for a single player
fn decide_for_player(
    ctx: &OffBallContext,
    existing_objectives: &[OffBallObjective; 22],
    current_tick: u64,
    config: &OffBallConfig,
) -> OffBallObjective {
    // Generate candidates
    let candidates = generate_candidates(ctx);

    if candidates.is_empty() {
        return OffBallObjective::default();
    }

    // Evaluate candidates
    let scores: Vec<Score6> = candidates.iter().map(|c| evaluate_candidate(c, ctx)).collect();

    // Select best candidate
    let rng_seed = current_tick.wrapping_mul(ctx.player_idx as u64 + 1);
    let best_idx = match select_best_candidate(&candidates, &scores, config.softmax_temperature, rng_seed) {
        Some(idx) => idx,
        None => return OffBallObjective::default(),
    };

    let best = &candidates[best_idx];
    let best_score = &scores[best_idx];

    // Try to resolve collision
    let (target_x, target_y) = match resolve_collision(
        best.target_x,
        best.target_y,
        existing_objectives,
        ctx.player_idx,
    ) {
        Some((x, y)) => (x, y),
        None => {
            // Collision unresolvable - fall back to no objective
            return OffBallObjective::default();
        }
    };

    // Determine urgency (may be adjusted based on phase/stamina)
    let urgency = determine_urgency(best.intent, ctx);

    // Calculate TTL
    let ttl = get_ttl_for_phase(ctx.phase, config);
    let expire_tick = current_tick.saturating_add(ttl);

    OffBallObjective::new(
        best.intent,
        target_x,
        target_y,
        urgency,
        expire_tick,
        best_score.total() / 6.0, // Normalize to 0-1ish
    )
}

/// Determine game phase
fn determine_phase(possession_team: u8, is_transition: bool, ticks_since: u64) -> GamePhase {
    if is_transition && ticks_since < 8 {
        // During transition, we don't know which way yet
        // This will be refined per-player
        GamePhase::Attacking
    } else {
        GamePhase::Attacking
    }
}

/// Determine urgency based on context
fn determine_urgency(intent: OffBallIntent, ctx: &OffBallContext) -> Urgency {
    // Transition: sprint if stamina allows
    if ctx.phase.is_transition() && ctx.player_stamina > STAMINA_LOW_THRESHOLD {
        return Urgency::Sprint;
    }

    // Defensive intents in danger zone: sprint
    if intent.is_defensive() {
        // FIX_2601/0116: Use attack_direction for own goal position
        let goal_x = if ctx.attack_direction > 0.0 { 0.0 } else { 105.0 };
        let ball_to_goal = (ctx.ball_x - goal_x).abs();

        if ball_to_goal < 30.0 && ctx.player_stamina > STAMINA_LOW_THRESHOLD {
            return Urgency::Sprint;
        }
    }

    // Low stamina: force jog
    if ctx.player_stamina < STAMINA_LOW_THRESHOLD {
        return Urgency::Jog;
    }

    // Check distance to target
    // This is a simplified check - actual target not available here
    // Default to Jog for normal situations
    Urgency::Jog
}

/// Estimate defensive line x position
fn estimate_defensive_line(
    positions: &[(f32, f32); 22],
    opponent_team: u8, // 0 = home, 1 = away
    attack_direction: f32,
) -> f32 {
    let start = if opponent_team == 0 { 0 } else { 11 };
    let end = if opponent_team == 0 { 11 } else { 22 };

    // Find the deepest defender (excluding GK at index 0 or 11)
    let gk_idx = if opponent_team == 0 { 0 } else { 11 };

    let mut deepest_x = if attack_direction > 0.0 { 105.0 } else { 0.0 };

    for idx in start..end {
        if idx == gk_idx {
            continue;
        }
        let (x, _) = positions[idx];

        if attack_direction > 0.0 {
            // Attacking right, defensive line is lower x
            if x < deepest_x {
                deepest_x = x;
            }
        } else {
            // Attacking left, defensive line is higher x
            if x > deepest_x {
                deepest_x = x;
            }
        }
    }

    deepest_x
}

// ============================================================================
// Line Spacing Helpers (FIX_2601/1126)
// ============================================================================

/// Get player's line role based on position index
/// Uses standard 4-4-2 slot mapping:
/// - 0: GK
/// - 1-4: DEF (CB, CB, LB, RB)
/// - 5-8: MID (CM, CM, LM, RM)
/// - 9-10: FWD (ST, ST)
fn get_player_line_role(player_idx: usize) -> u8 {
    let local_idx = player_idx % 11;
    match local_idx {
        0 => 0,      // GK
        1..=4 => 1,  // DEF
        5..=8 => 2,  // MID
        _ => 3,      // FWD (9, 10)
    }
}

/// Calculate line anchor X based on ball position and role
fn calculate_line_anchor_x(
    ball_x: f32,
    player_line_role: u8,
    is_attacking: bool,
    attack_direction: f32,
) -> f32 {
    // Line spacing targets (from types.rs constants)
    let def_mid_gap = DEF_MID_GAP_TARGET_M;
    let mid_fwd_gap = MID_FWD_GAP_TARGET_M;

    // Base X depends on game state
    let base_x = if is_attacking {
        ball_x  // Attacking: lines follow the ball
    } else {
        // Defending: anchor to defensive position
        if attack_direction > 0.0 { 30.0 } else { 75.0 }
    };

    // Calculate line-specific anchor
    match player_line_role {
        0 => {
            // GK: stays deep
            if attack_direction > 0.0 { 5.0 } else { 100.0 }
        }
        1 => {
            // DEF: behind midfield by gap distance
            (base_x - def_mid_gap * attack_direction).clamp(5.0, 100.0)
        }
        2 => {
            // MID: at base position
            base_x.clamp(15.0, 90.0)
        }
        3 => {
            // FWD: ahead of midfield by gap distance
            (base_x + mid_fwd_gap * attack_direction).clamp(10.0, 100.0)
        }
        _ => base_x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_state() -> (
        [OffBallObjective; 22],
        [(f32, f32); 22],
        [f32; 22],
        [(f32, f32); 22],
    ) {
        let objectives = [OffBallObjective::default(); 22];

        let mut positions = [(52.5, 34.0); 22];
        let mut base_positions = [(52.5, 34.0); 22];

        // Spread home team (0-10)
        for i in 0..11 {
            positions[i] = (10.0 + i as f32 * 8.0, 20.0 + (i % 3) as f32 * 14.0);
            base_positions[i] = positions[i];
        }
        // Spread away team (11-21)
        for i in 11..22 {
            positions[i] = (95.0 - (i - 11) as f32 * 8.0, 20.0 + (i % 3) as f32 * 14.0);
            base_positions[i] = positions[i];
        }

        let staminas = [0.8; 22];

        (objectives, positions, staminas, base_positions)
    }

    #[test]
    fn test_update_offball_decisions_runs() {
        let (mut objectives, positions, staminas, base_positions) = make_test_state();
        let config = OffBallConfig::default();

        let decisions = update_offball_decisions(
            &mut objectives,
            &positions,
            &staminas,
            &base_positions,
            (50.0, 34.0),
            (0.0, 0.0),
            Some(5),
            0,
            true,
            20,
            false,
            100,
            &config,
        );

        assert!(decisions > 0);
        assert!(decisions <= MAX_DECISIONS_NORMAL);
    }

    #[test]
    fn test_update_creates_valid_objectives() {
        let (mut objectives, positions, staminas, base_positions) = make_test_state();
        let config = OffBallConfig::default();

        update_offball_decisions(
            &mut objectives,
            &positions,
            &staminas,
            &base_positions,
            (50.0, 34.0),
            (0.0, 0.0),
            None,
            0,
            true,
            20,
            false,
            100,
            &config,
        );

        // Check some objectives were created
        let valid_count = objectives.iter().filter(|o| o.is_valid()).count();
        assert!(valid_count > 0);

        // Check objectives are within field
        for obj in &objectives {
            if obj.is_valid() {
                assert!(obj.target_x >= 0.0 && obj.target_x <= 105.0);
                assert!(obj.target_y >= 0.0 && obj.target_y <= 68.0);
            }
        }
    }

    #[test]
    fn test_transition_allows_more_decisions() {
        let (mut objectives, positions, staminas, base_positions) = make_test_state();
        let config = OffBallConfig::default();

        let normal = update_offball_decisions(
            &mut objectives,
            &positions,
            &staminas,
            &base_positions,
            (50.0, 34.0),
            (0.0, 0.0),
            None,
            0,
            true,
            20, // Not transition
            false,
            100,
            &config,
        );

        // Reset
        let mut objectives2 = [OffBallObjective::default(); 22];

        let transition = update_offball_decisions(
            &mut objectives2,
            &positions,
            &staminas,
            &base_positions,
            (50.0, 34.0),
            (0.0, 0.0),
            None,
            0,
            true,
            2, // Transition window
            true,
            100,
            &config,
        );

        // Transition should allow more decisions
        assert!(transition >= normal);
    }

    #[test]
    fn test_estimate_defensive_line() {
        let mut positions = [(52.5, 34.0); 22];

        // Place away team defenders (all on their side of the field)
        positions[11] = (90.0, 34.0); // GK at goal line
        positions[12] = (75.0, 20.0); // Defender
        positions[13] = (72.0, 34.0); // Deepest defender
        positions[14] = (78.0, 48.0); // Defender
        // Set other away players further up the field
        for i in 15..22 {
            positions[i] = (85.0, 34.0); // Midfielders and forwards
        }

        // Attacking right (home team perspective)
        let line = estimate_defensive_line(&positions, 1, 1.0);

        // Should be the lowest x defender (excluding GK)
        assert!((line - 72.0).abs() < 0.1);
    }
}
