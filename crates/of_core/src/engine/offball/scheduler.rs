//! TTL and trigger-based scheduling for off-ball decisions

use super::types::*;

/// Apply force-expire triggers to objectives.
///
/// Triggers:
/// 1. Possession transition
/// 2. Ball proximity (within 6m)
/// 3. Player state change (low stamina)
/// 4. Target invalidation
pub fn apply_force_expire_triggers(
    objectives: &mut [OffBallObjective; 22],
    player_positions: &[(f32, f32); 22],
    player_staminas: &[f32; 22],
    ball_pos: (f32, f32),
    possession_changed: bool,
    current_tick: u64,
) {
    for (idx, obj) in objectives.iter_mut().enumerate() {
        if !obj.is_valid() {
            continue;
        }

        let mut should_expire = false;

        // Trigger 1: Possession transition
        if possession_changed {
            should_expire = true;
        }

        // Trigger 2: Ball proximity
        let (px, py) = player_positions[idx];
        let dx = px - ball_pos.0;
        let dy = py - ball_pos.1;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < BALL_PROXIMITY_TRIGGER_M * BALL_PROXIMITY_TRIGGER_M {
            should_expire = true;
        }

        // Trigger 3: Low stamina (if was sprinting)
        if obj.urgency == Urgency::Sprint && player_staminas[idx] < STAMINA_LOW_THRESHOLD {
            should_expire = true;
        }

        // Trigger 4: Target out of bounds or too close (collapsed objective)
        let tx = obj.target_x;
        let ty = obj.target_y;
        if tx < 0.0 || tx > 105.0 || ty < 0.0 || ty > 68.0 {
            should_expire = true;
        }
        // If target is essentially at player position, objective is meaningless
        let target_dx = tx - px;
        let target_dy = ty - py;
        let target_dist_sq = target_dx * target_dx + target_dy * target_dy;
        if target_dist_sq < 1.0 {
            // Less than 1m away
            should_expire = true;
        }

        if should_expire {
            obj.force_expire(current_tick);
        }
    }
}

/// Select players who need to make decisions this tick.
///
/// Priority:
/// 1. Ball-proximate players (Top N)
/// 2. One per line (defense/midfield/attack)
/// 3. TTL-expired players
///
/// Returns indices of players to decide, limited by K.
pub fn select_players_for_decision(
    objectives: &[OffBallObjective; 22],
    player_positions: &[(f32, f32); 22],
    ball_pos: (f32, f32),
    ball_owner: Option<usize>,
    is_transition: bool,
    current_tick: u64,
) -> Vec<usize> {
    let max_decisions = if is_transition {
        MAX_DECISIONS_TRANSITION
    } else {
        MAX_DECISIONS_NORMAL
    };

    let mut candidates: Vec<(usize, f32)> = Vec::with_capacity(22);

    // Calculate distance to ball for all players
    for idx in 0..22 {
        // Skip ball owner (uses on-ball decisions)
        if ball_owner == Some(idx) {
            continue;
        }

        let (px, py) = player_positions[idx];
        let dx = px - ball_pos.0;
        let dy = py - ball_pos.1;
        let dist = (dx * dx + dy * dy).sqrt();

        candidates.push((idx, dist));
    }

    // Sort by distance (closest first) with position-based tie-breaker (FIX_2601/0116)
    candidates.sort_by(|a, b| {
        match a.1.partial_cmp(&b.1) {
            Some(std::cmp::Ordering::Equal) | None => {
                // FIX_2601/0116: Use position-based hash for fair tie-breaking
                crate::engine::match_sim::deterministic_tie_hash(
                    a.0,
                    player_positions[a.0],
                    b.0,
                    player_positions[b.0],
                )
            }
            Some(ord) => ord,
        }
    });

    let mut selected: Vec<usize> = Vec::with_capacity(max_decisions);
    let mut selected_set = [false; 22];

    // Priority 1: Ball-proximate players (Top N)
    for &(idx, _) in candidates.iter().take(BALL_PROXIMITY_TOP_N) {
        if selected.len() >= max_decisions {
            break;
        }
        selected.push(idx);
        selected_set[idx] = true;
    }

    // Priority 2: One per line (simplified: by x position range)
    // Defense: x < 35, Midfield: 35-70, Attack: x > 70
    let lines = [(0.0, 35.0), (35.0, 70.0), (70.0, 105.0)];

    for (x_min, x_max) in lines {
        if selected.len() >= max_decisions {
            break;
        }

        // Find closest non-selected player in this line
        let mut best: Option<(usize, f32)> = None;
        for &(idx, dist) in &candidates {
            if selected_set[idx] {
                continue;
            }
            let (px, _) = player_positions[idx];
            if px >= x_min && px < x_max {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((idx, dist));
                }
            }
        }

        if let Some((idx, _)) = best {
            selected.push(idx);
            selected_set[idx] = true;
        }
    }

    // Priority 3: TTL-expired players
    for &(idx, _) in &candidates {
        if selected.len() >= max_decisions {
            break;
        }
        if selected_set[idx] {
            continue;
        }

        let obj = &objectives[idx];
        if !obj.is_valid() || obj.is_expired(current_tick) {
            selected.push(idx);
            selected_set[idx] = true;
        }
    }

    selected
}

/// Determine if we're in a transition window
pub fn is_transition_window(ticks_since_change: u64) -> bool {
    ticks_since_change < TRANSITION_WINDOW_TICKS
}

/// Get appropriate TTL based on game phase
pub fn get_ttl_for_phase(phase: GamePhase, config: &OffBallConfig) -> u64 {
    if phase.is_transition() {
        config.ttl_ticks_transition
    } else {
        config.ttl_ticks_default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_positions() -> [(f32, f32); 22] {
        let mut pos = [(52.5, 34.0); 22];
        // Spread players across field
        for i in 0..11 {
            pos[i] = (10.0 + i as f32 * 8.0, 20.0 + (i % 3) as f32 * 14.0);
        }
        for i in 11..22 {
            pos[i] = (95.0 - (i - 11) as f32 * 8.0, 20.0 + (i % 3) as f32 * 14.0);
        }
        pos
    }

    #[test]
    fn test_select_players_respects_limit() {
        let objectives = [OffBallObjective::default(); 22];
        let positions = make_positions();
        let ball_pos = (52.5, 34.0);

        let selected = select_players_for_decision(
            &objectives,
            &positions,
            ball_pos,
            Some(5), // ball owner
            false,
            100,
        );

        assert!(selected.len() <= MAX_DECISIONS_NORMAL);
        assert!(!selected.contains(&5)); // ball owner excluded
    }

    #[test]
    fn test_select_players_transition_allows_more() {
        let objectives = [OffBallObjective::default(); 22];
        let positions = make_positions();
        let ball_pos = (52.5, 34.0);

        let normal = select_players_for_decision(&objectives, &positions, ball_pos, None, false, 100);

        let transition =
            select_players_for_decision(&objectives, &positions, ball_pos, None, true, 100);

        // Transition should allow more decisions
        assert!(transition.len() >= normal.len());
    }

    #[test]
    fn test_force_expire_on_possession_change() {
        let mut objectives = [OffBallObjective::default(); 22];
        objectives[0] = OffBallObjective::new(
            OffBallIntent::LinkPlayer,
            50.0,
            30.0,
            Urgency::Jog,
            200,
            0.8,
        );

        let positions = make_positions();
        let staminas = [1.0; 22];

        apply_force_expire_triggers(
            &mut objectives,
            &positions,
            &staminas,
            (50.0, 34.0),
            true, // possession changed
            100,
        );

        assert!(objectives[0].is_expired(100));
    }

    #[test]
    fn test_force_expire_on_ball_proximity() {
        let mut objectives = [OffBallObjective::default(); 22];
        let mut positions = make_positions();

        // Place player 0 at (50, 34) with ball at (52, 34) = 2m away
        positions[0] = (50.0, 34.0);
        objectives[0] = OffBallObjective::new(
            OffBallIntent::LinkPlayer,
            60.0,
            30.0,
            Urgency::Jog,
            200,
            0.8,
        );

        let staminas = [1.0; 22];

        apply_force_expire_triggers(
            &mut objectives,
            &positions,
            &staminas,
            (52.0, 34.0), // ball close to player 0
            false,
            100,
        );

        assert!(objectives[0].is_expired(100));
    }

    #[test]
    fn test_is_transition_window() {
        assert!(is_transition_window(0));
        assert!(is_transition_window(7));
        assert!(!is_transition_window(8));
        assert!(!is_transition_window(100));
    }
}
