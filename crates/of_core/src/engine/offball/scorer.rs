//! Score6 evaluation for off-ball candidates

use super::types::*;

/// Evaluate a candidate and return Score6
pub fn evaluate_candidate(candidate: &OffBallCandidate, ctx: &OffBallContext) -> Score6 {
    Score6 {
        usefulness: calc_usefulness(candidate, ctx),
        safety: calc_safety(candidate, ctx),
        availability: calc_availability(candidate, ctx),
        progress: calc_progress(candidate, ctx),
        structure: calc_structure(candidate, ctx),
        cost: calc_cost(candidate, ctx),
    }
}

/// Select best candidate using argmax or softmax
pub fn select_best_candidate(
    candidates: &[OffBallCandidate],
    scores: &[Score6],
    temperature: f32,
    rng_seed: u64,
) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }

    if temperature <= 0.0 {
        // Argmax selection
        scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total().partial_cmp(&b.1.total()).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    } else {
        // Softmax sampling with deterministic pseudo-random
        softmax_select(scores, temperature, rng_seed)
    }
}

fn softmax_select(scores: &[Score6], temperature: f32, seed: u64) -> Option<usize> {
    if scores.is_empty() {
        return None;
    }

    let totals: Vec<f32> = scores.iter().map(|s| s.total() / temperature).collect();

    // Stable softmax
    let max_t = totals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = totals.iter().map(|t| (t - max_t).exp()).collect();
    let sum: f32 = exps.iter().sum();

    if sum <= 0.0 {
        return Some(0);
    }

    let probs: Vec<f32> = exps.iter().map(|e| e / sum).collect();

    // Deterministic pseudo-random using seed
    let r = deterministic_rand(seed);

    let mut cumsum = 0.0;
    for (i, p) in probs.iter().enumerate() {
        cumsum += p;
        if r < cumsum {
            return Some(i);
        }
    }

    Some(scores.len() - 1)
}

/// Simple deterministic random in [0, 1) based on seed
fn deterministic_rand(seed: u64) -> f32 {
    // Simple LCG-style hash
    let hash = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (hash as f32) / (u64::MAX as f32)
}

// ============================================================================
// Score6 Components
// ============================================================================

fn calc_usefulness(c: &OffBallCandidate, ctx: &OffBallContext) -> f32 {
    match c.intent {
        OffBallIntent::LinkPlayer | OffBallIntent::Lurker => {
            // Useful if at good passing distance from ball
            let dx = c.target_x - ctx.ball_x;
            let dy = c.target_y - ctx.ball_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if (8.0..=14.0).contains(&dist) {
                0.8
            } else if (5.0..=18.0).contains(&dist) {
                0.5
            } else {
                0.3
            }
        }
        OffBallIntent::Screen | OffBallIntent::PressSupport => {
            // Useful if blocking lane well
            let lane_dist = calc_lane_distance(c.target_x, c.target_y, ctx);
            (1.0 - lane_dist / 10.0).clamp(0.0, 1.0)
        }
        OffBallIntent::SpaceAttacker => {
            // Useful if actually behind defensive line
            let behind = (c.target_x - ctx.defensive_line_x) * ctx.attack_direction;
            if behind > 0.0 {
                0.9
            } else {
                0.4
            }
        }
        OffBallIntent::WidthHolder => {
            // Useful if providing actual width
            let width_from_center = (c.target_y - 34.0).abs();
            if width_from_center > 25.0 {
                0.8
            } else {
                0.5
            }
        }
        OffBallIntent::TrackBack => {
            // Useful based on distance to base position
            let dx = c.target_x - ctx.base_x;
            let dy = c.target_y - ctx.base_y;
            let dist = (dx * dx + dy * dy).sqrt();
            (1.0 - dist / 20.0).clamp(0.3, 1.0)
        }
        OffBallIntent::ShapeHolder => {
            // FIX_2601/1126: Useful if maintaining line spacing
            // High usefulness when player is significantly off their line
            let line_deviation = (ctx.player_x - ctx.line_anchor_x).abs();
            if line_deviation > 10.0 {
                0.95  // Very useful when far from line
            } else if line_deviation > 5.0 {
                0.7   // Moderately useful
            } else {
                0.3   // Not needed when already on line
            }
        }
        OffBallIntent::None => 0.0,
    }
}

fn calc_safety(c: &OffBallCandidate, ctx: &OffBallContext) -> f32 {
    // Base: deviation from base position
    let dx = c.target_x - ctx.base_x;
    let dy = c.target_y - ctx.base_y;
    let base_dist = (dx * dx + dy * dy).sqrt();
    let base_score = (1.0 - base_dist / 30.0).clamp(0.0, 1.0);

    // Penalty for leaving defensive line exposed
    // FIX_2601/0116: Use attack_direction for own goal position
    // Own goal is opposite to attack direction: if attacking right (>0), own goal is at x=0
    let goal_x = if ctx.attack_direction > 0.0 { 0.0 } else { 105.0 };
    let target_to_goal = (c.target_x - goal_x).abs();
    let base_to_goal = (ctx.base_x - goal_x).abs();

    if c.intent.is_attacking() && target_to_goal > base_to_goal + 20.0 {
        // Moving far from defensive responsibility
        base_score * 0.7
    } else {
        base_score
    }
}

fn calc_availability(c: &OffBallCandidate, ctx: &OffBallContext) -> f32 {
    match ctx.phase {
        GamePhase::Attacking | GamePhase::TransitionWin => {
            // Pass option availability
            let dx = c.target_x - ctx.ball_x;
            let dy = c.target_y - ctx.ball_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if (8.0..=14.0).contains(&dist) {
                0.9
            } else if (5.0..=20.0).contains(&dist) {
                0.6
            } else {
                0.3
            }
        }
        GamePhase::Defending | GamePhase::TransitionLoss => {
            // Lane blocking availability
            let lane_dist = calc_lane_distance(c.target_x, c.target_y, ctx);
            (1.0 - lane_dist / 8.0).clamp(0.0, 1.0)
        }
    }
}

fn calc_progress(c: &OffBallCandidate, ctx: &OffBallContext) -> f32 {
    // Forward progress relative to ball
    let progress = (c.target_x - ctx.ball_x) * ctx.attack_direction;

    match c.intent {
        OffBallIntent::SpaceAttacker => {
            // Extra bonus for penetration
            (progress / 20.0).clamp(0.0, 1.0) * 1.2
        }
        OffBallIntent::ShapeHolder => {
            // FIX_2601/1126: ShapeHolder shouldn't be penalized for backward movement
            // Progress is measured by how close target is to line_anchor (ideal position)
            let anchor_dist = (c.target_x - ctx.line_anchor_x).abs();
            // Closer to anchor = better "progress" towards tactical shape
            (1.0 - anchor_dist / 15.0).clamp(0.5, 1.0)
        }
        _ => (progress / 20.0 + 0.5).clamp(0.0, 1.0),
    }
}

fn calc_structure(c: &OffBallCandidate, ctx: &OffBallContext) -> f32 {
    use crate::engine::offball::types::LINE_DEVIATION_PENALTY_RADIUS_M;

    // FIX_2601/1126: Enhanced structure scoring with line enforcement

    // 1. Base position deviation (original logic)
    let dx = c.target_x - ctx.base_x;
    let dy = c.target_y - ctx.base_y;
    let base_deviation = (dx * dx + dy * dy).sqrt();
    let base_score = (1.0 - base_deviation / 25.0).clamp(0.0, 1.0);

    // 2. Line anchor enforcement (new for line spacing fix)
    let line_deviation = (c.target_x - ctx.line_anchor_x).abs();
    let line_score = if line_deviation < LINE_DEVIATION_PENALTY_RADIUS_M {
        // Within tolerance: gradual penalty
        1.0 - (line_deviation / LINE_DEVIATION_PENALTY_RADIUS_M) * 0.3
    } else {
        // Beyond tolerance: heavy penalty
        0.5
    };

    // 3. Combine: 50% base position, 50% line anchor
    base_score * 0.5 + line_score * 0.5
}

fn calc_cost(c: &OffBallCandidate, ctx: &OffBallContext) -> f32 {
    // Movement cost
    let dx = c.target_x - ctx.player_x;
    let dy = c.target_y - ctx.player_y;
    let dist = (dx * dx + dy * dy).sqrt();

    let stamina = ctx.player_stamina.max(0.1);
    let cost_raw = dist / (stamina * 50.0);

    // Invert: lower cost = higher score
    (1.0 - cost_raw).clamp(0.0, 1.0)
}

/// Calculate distance from point to ball-to-goal lane
fn calc_lane_distance(x: f32, y: f32, ctx: &OffBallContext) -> f32 {
    // FIX_2601/0116: Use attack_direction for own goal position
    let goal_x = if ctx.attack_direction > 0.0 { 0.0 } else { 105.0 };
    let goal_y = 34.0; // Center of goal

    // Vector from ball to goal
    let lane_dx = goal_x - ctx.ball_x;
    let lane_dy = goal_y - ctx.ball_y;
    let lane_len = (lane_dx * lane_dx + lane_dy * lane_dy).sqrt();

    if lane_len < 0.1 {
        return 0.0;
    }

    // Point from ball to target
    let px = x - ctx.ball_x;
    let py = y - ctx.ball_y;

    // Cross product gives perpendicular distance
    let cross = (lane_dx * py - lane_dy * px).abs();
    cross / lane_len
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> OffBallContext {
        OffBallContext {
            ball_x: 50.0,
            ball_y: 34.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            possession_team: 0,
            phase: GamePhase::Attacking,
            player_idx: 5,
            player_x: 45.0,
            player_y: 30.0,
            player_vx: 0.0,
            player_vy: 0.0,
            player_stamina: 0.8,
            base_x: 40.0,
            base_y: 30.0,
            is_home_team: true,
            attack_direction: 1.0,
            defensive_line_x: 75.0,
            ticks_since_transition: 20,
            current_tick: 100,
            player_line_role: 2,  // MID
            line_anchor_x: 50.0,  // At ball position for attacking
        }
    }

    #[test]
    fn test_evaluate_returns_valid_scores() {
        let ctx = make_context();
        let candidate = OffBallCandidate::new(OffBallIntent::LinkPlayer, 60.0, 34.0, Urgency::Jog);

        let score = evaluate_candidate(&candidate, &ctx);

        assert!(score.usefulness >= 0.0 && score.usefulness <= 1.5);
        assert!(score.safety >= 0.0 && score.safety <= 1.0);
        assert!(score.availability >= 0.0 && score.availability <= 1.0);
        assert!(score.progress >= 0.0 && score.progress <= 1.5);
        assert!(score.structure >= 0.0 && score.structure <= 1.0);
        assert!(score.cost >= 0.0 && score.cost <= 1.0);
    }

    #[test]
    fn test_select_best_argmax() {
        let candidates = vec![
            OffBallCandidate::new(OffBallIntent::LinkPlayer, 60.0, 34.0, Urgency::Jog),
            OffBallCandidate::new(OffBallIntent::SpaceAttacker, 80.0, 34.0, Urgency::Sprint),
        ];

        let scores = vec![
            Score6 {
                usefulness: 0.8,
                safety: 0.7,
                availability: 0.6,
                progress: 0.5,
                structure: 0.4,
                cost: 0.5,
            },
            Score6 {
                usefulness: 0.9,
                safety: 0.6,
                availability: 0.7,
                progress: 0.8,
                structure: 0.3,
                cost: 0.4,
            },
        ];

        let best = select_best_candidate(&candidates, &scores, 0.0, 0);
        assert!(best.is_some());

        // Second candidate has higher total
        let idx = best.unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_deterministic_rand_range() {
        for seed in 0..100 {
            let r = deterministic_rand(seed);
            assert!(r >= 0.0 && r < 1.0);
        }
    }

    #[test]
    fn test_lane_distance() {
        let ctx = make_context();

        // Point directly on lane should have distance ~0
        let on_lane = calc_lane_distance(25.0, 34.0, &ctx);
        assert!(on_lane < 1.0);

        // Point off lane should have larger distance
        let off_lane = calc_lane_distance(25.0, 50.0, &ctx);
        assert!(off_lane > 10.0);
    }
}
