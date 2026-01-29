//! Candidate generation for off-ball decisions

use super::types::*;

/// Field dimensions
const FIELD_LENGTH: f32 = 105.0;
const FIELD_WIDTH: f32 = 68.0;
const FIELD_CENTER_Y: f32 = 34.0;

/// Penalty box x position (from goal line)
const PENALTY_BOX_DEPTH: f32 = 16.5;

/// Generate candidates for a player based on game phase.
///
/// Returns up to MAX_CANDIDATES_PER_PLAYER candidates.
pub fn generate_candidates(ctx: &OffBallContext) -> Vec<OffBallCandidate> {
    let mut candidates = Vec::with_capacity(MAX_CANDIDATES_PER_PLAYER);

    match ctx.phase {
        GamePhase::Attacking | GamePhase::TransitionWin => {
            generate_attacking_candidates(ctx, &mut candidates);
        }
        GamePhase::Defending | GamePhase::TransitionLoss => {
            generate_defending_candidates(ctx, &mut candidates);
        }
    }

    // Clamp all targets to field and reachability
    for c in &mut candidates {
        clamp_to_field(&mut c.target_x, &mut c.target_y);
        clamp_to_reachable(
            ctx.player_x,
            ctx.player_y,
            &mut c.target_x,
            &mut c.target_y,
            c.urgency,
            DEFAULT_TTL_TICKS as f32 * 0.25, // TTL in seconds
        );
    }

    candidates
}

fn generate_attacking_candidates(ctx: &OffBallContext, candidates: &mut Vec<OffBallCandidate>) {
    // 0. ShapeHolder - maintain formation line spacing (FIX_2601/1126)
    // Always generate this first to ensure proper team shape
    if let Some(c) = generate_shape_holder(ctx) {
        candidates.push(c);
    }

    // 1. LinkPlayer - provide passing option
    if let Some(c) = generate_link_player(ctx) {
        candidates.push(c);
    }

    // 2. SpaceAttacker - run behind line
    if let Some(c) = generate_space_attacker(ctx) {
        candidates.push(c);
    }

    // 3. Lurker - box edge / cutback
    if let Some(c) = generate_lurker(ctx) {
        candidates.push(c);
    }

    // 4. WidthHolder - maintain width
    if let Some(c) = generate_width_holder(ctx) {
        candidates.push(c);
    }
}

fn generate_defending_candidates(ctx: &OffBallContext, candidates: &mut Vec<OffBallCandidate>) {
    // 0. ShapeHolder - maintain defensive line spacing (FIX_2601/1126)
    if let Some(c) = generate_shape_holder(ctx) {
        candidates.push(c);
    }

    // 1. TrackBack - recover shape
    if let Some(c) = generate_track_back(ctx) {
        candidates.push(c);
    }

    // 2. Screen - block lane
    if let Some(c) = generate_screen(ctx) {
        candidates.push(c);
    }

    // 3. PressSupport - support press
    if let Some(c) = generate_press_support(ctx) {
        candidates.push(c);
    }
}

// ============================================================================
// Attacking Intent Generators
// ============================================================================

/// Generate ShapeHolder target: maintain formation line spacing (FIX_2601/1126)
///
/// This candidate ensures proper line gaps (DEF-MID: 14m, MID-FWD: 16m)
/// by targeting the line_anchor_x position calculated for this player's role.
fn generate_shape_holder(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    // Don't generate for GK (role 0)
    if ctx.player_line_role == 0 {
        return None;
    }

    // Target X: line anchor position (maintains proper spacing from other lines)
    let target_x = ctx.line_anchor_x;

    // Target Y: maintain lateral position relative to base
    // Blend between current y and base y to maintain width structure
    let target_y = ctx.base_y * 0.7 + ctx.player_y * 0.3;

    // Always generate ShapeHolder - let scorer decide if it's needed
    // The usefulness score in scorer.rs will be high when player is off their line
    Some(OffBallCandidate::new(
        OffBallIntent::ShapeHolder,
        target_x,
        target_y,
        Urgency::Jog,
    ))
}

/// Generate LinkPlayer target: provide passing option 8-14m from ball
fn generate_link_player(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    const MIN_DIST: f32 = 8.0;
    const MAX_DIST: f32 = 14.0;

    // Target direction: forward and slightly lateral
    let forward_x = ctx.ball_x + ctx.attack_direction * 10.0;

    // Determine lateral offset based on player's current y position
    let lateral_offset = if ctx.player_y < FIELD_CENTER_Y { 5.0 } else { -5.0 };
    let target_y = ctx.ball_y + lateral_offset;

    let target_x = forward_x;

    // Check distance constraint
    let dx = target_x - ctx.ball_x;
    let dy = target_y - ctx.ball_y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < MIN_DIST || dist > MAX_DIST {
        // Adjust to be within range
        let scale = ((MIN_DIST + MAX_DIST) / 2.0) / dist.max(0.1);
        let adjusted_x = ctx.ball_x + dx * scale;
        let adjusted_y = ctx.ball_y + dy * scale;
        return Some(OffBallCandidate::new(
            OffBallIntent::LinkPlayer,
            adjusted_x,
            adjusted_y,
            Urgency::Jog,
        ));
    }

    Some(OffBallCandidate::new(
        OffBallIntent::LinkPlayer,
        target_x,
        target_y,
        Urgency::Jog,
    ))
}

/// Generate SpaceAttacker target: run behind defensive line
fn generate_space_attacker(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    const PENETRATION_DIST: f32 = 5.0;

    // Target x: behind defensive line
    let target_x = ctx.defensive_line_x + ctx.attack_direction * PENETRATION_DIST;

    // Target y: half-space (center ± 12m)
    let target_y = if ctx.player_y < FIELD_CENTER_Y {
        FIELD_CENTER_Y - 12.0 // Left half-space
    } else {
        FIELD_CENTER_Y + 12.0 // Right half-space
    };

    // Urgency: sprint for penetration
    let urgency = if ctx.player_stamina > STAMINA_LOW_THRESHOLD {
        Urgency::Sprint
    } else {
        Urgency::Jog
    };

    Some(OffBallCandidate::new(
        OffBallIntent::SpaceAttacker,
        target_x,
        target_y,
        urgency,
    ))
}

/// Generate Lurker target: box edge for cutback/second ball
fn generate_lurker(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    const D_ZONE_OFFSET: f32 = 5.0;

    // Target near penalty box edge
    let box_x = if ctx.attack_direction > 0.0 {
        FIELD_LENGTH - PENALTY_BOX_DEPTH - D_ZONE_OFFSET
    } else {
        PENALTY_BOX_DEPTH + D_ZONE_OFFSET
    };

    // Y position: opposite side from ball for cutback
    let target_y = if ctx.ball_y < FIELD_CENTER_Y { 43.0 } else { 25.0 };

    Some(OffBallCandidate::new(
        OffBallIntent::Lurker,
        box_x,
        target_y,
        Urgency::Jog,
    ))
}

/// Generate WidthHolder target: maintain width near touchline
fn generate_width_holder(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    // Default offset from touchline
    let touchline_offset = 6.0;

    let target_y = if ctx.player_y < FIELD_CENTER_Y {
        touchline_offset // Left side
    } else {
        FIELD_WIDTH - touchline_offset // Right side
    };

    // X position: level with or ahead of ball
    let target_x = ctx.ball_x + ctx.attack_direction * 5.0;

    Some(OffBallCandidate::new(
        OffBallIntent::WidthHolder,
        target_x,
        target_y,
        Urgency::Jog,
    ))
}

// ============================================================================
// Defending Intent Generators
// ============================================================================

/// Generate TrackBack target: recover defensive shape
fn generate_track_back(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    // Return toward base position
    let mut target_x = ctx.base_x;
    let mut target_y = ctx.base_y;

    // Central lane correction: if center is exposed, shift toward center
    let center_exposure = (ctx.ball_y - FIELD_CENTER_Y).abs() / FIELD_CENTER_Y;
    if center_exposure < 0.3 {
        // Ball is central, protect center
        target_y = target_y * 0.7 + FIELD_CENTER_Y * 0.3;
    }

    // Urgency based on danger
    let dist_to_goal = if ctx.attack_direction > 0.0 {
        ctx.ball_x // Home goal at x=0
    } else {
        FIELD_LENGTH - ctx.ball_x // Away goal at x=105
    };

    let urgency = if dist_to_goal < 30.0 && ctx.player_stamina > STAMINA_LOW_THRESHOLD {
        Urgency::Sprint
    } else {
        Urgency::Jog
    };

    Some(OffBallCandidate::new(
        OffBallIntent::TrackBack,
        target_x,
        target_y,
        urgency,
    ))
}

/// Generate Screen target: block dangerous lane
fn generate_screen(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    // Position on the line between ball and goal
    // FIX_2601/0116: Use attack_direction for own goal position
    let goal_x = if ctx.attack_direction > 0.0 { 0.0 } else { FIELD_LENGTH };

    // Point 8m from ball toward goal
    let dx = goal_x - ctx.ball_x;
    let dist = dx.abs();
    let scale = 8.0 / dist.max(1.0);

    let target_x = ctx.ball_x + dx * scale;
    let target_y = ctx.ball_y; // Stay on ball's y-level

    // Urgency: sprint if ball is in dangerous area
    let urgency = if dist < 30.0 && ctx.player_stamina > STAMINA_LOW_THRESHOLD {
        Urgency::Sprint
    } else {
        Urgency::Jog
    };

    Some(OffBallCandidate::new(
        OffBallIntent::Screen,
        target_x,
        target_y,
        urgency,
    ))
}

/// Generate PressSupport target: support pressing without direct tackle
fn generate_press_support(ctx: &OffBallContext) -> Option<OffBallCandidate> {
    const OFFSET_DIST: f32 = 4.5;
    const MIN_DIST: f32 = 2.0;

    // Approach from an angle (45 degrees)
    let angle = 45.0_f32.to_radians();

    // Direction depends on which side of the ball we're on
    let angle_sign = if ctx.player_y < ctx.ball_y { 1.0 } else { -1.0 };

    let offset_x = -ctx.attack_direction * OFFSET_DIST * angle.cos();
    let offset_y = angle_sign * OFFSET_DIST * angle.sin();

    let target_x = ctx.ball_x + offset_x;
    let target_y = ctx.ball_y + offset_y;

    // Check minimum distance (don't get too close)
    let dx = target_x - ctx.ball_x;
    let dy = target_y - ctx.ball_y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < MIN_DIST {
        return None;
    }

    Some(OffBallCandidate::new(
        OffBallIntent::PressSupport,
        target_x,
        target_y,
        Urgency::Jog,
    ))
}

// ============================================================================
// Helpers
// ============================================================================

/// Clamp position to field bounds
fn clamp_to_field(x: &mut f32, y: &mut f32) {
    *x = x.clamp(1.0, FIELD_LENGTH - 1.0);
    *y = y.clamp(1.0, FIELD_WIDTH - 1.0);
}

/// Clamp target to reachable distance based on urgency and time
fn clamp_to_reachable(
    from_x: f32,
    from_y: f32,
    target_x: &mut f32,
    target_y: &mut f32,
    urgency: Urgency,
    ttl_secs: f32,
) {
    let max_dist = urgency.speed_m_per_s() * ttl_secs;

    let dx = *target_x - from_x;
    let dy = *target_y - from_y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist > max_dist && dist > 0.1 {
        let scale = max_dist / dist;
        *target_x = from_x + dx * scale;
        *target_y = from_y + dy * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_attacking_context() -> OffBallContext {
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

    fn make_defending_context() -> OffBallContext {
        let mut ctx = make_attacking_context();
        ctx.phase = GamePhase::Defending;
        ctx.possession_team = 1;
        ctx.attack_direction = -1.0; // Defending, opponent attacks
        ctx
    }

    #[test]
    fn test_generate_attacking_candidates() {
        let ctx = make_attacking_context();
        let candidates = generate_candidates(&ctx);

        assert!(!candidates.is_empty());
        assert!(candidates.len() <= MAX_CANDIDATES_PER_PLAYER);

        // Should have attacking intents
        let intents: Vec<_> = candidates.iter().map(|c| c.intent).collect();
        assert!(intents.iter().any(|i| i.is_attacking()));
    }

    #[test]
    fn test_generate_defending_candidates() {
        let ctx = make_defending_context();
        let candidates = generate_candidates(&ctx);

        assert!(!candidates.is_empty());

        // Should have defensive intents
        let intents: Vec<_> = candidates.iter().map(|c| c.intent).collect();
        assert!(intents.iter().any(|i| i.is_defensive()));
    }

    #[test]
    fn test_candidates_within_field() {
        let ctx = make_attacking_context();
        let candidates = generate_candidates(&ctx);

        for c in &candidates {
            assert!(c.target_x >= 0.0 && c.target_x <= FIELD_LENGTH);
            assert!(c.target_y >= 0.0 && c.target_y <= FIELD_WIDTH);
        }
    }

    #[test]
    fn test_link_player_distance() {
        let ctx = make_attacking_context();
        if let Some(c) = generate_link_player(&ctx) {
            let dx = c.target_x - ctx.ball_x;
            let dy = c.target_y - ctx.ball_y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Should be within 8-14m range
            assert!(dist >= 7.0 && dist <= 15.0); // Allow small tolerance
        }
    }

    #[test]
    fn test_clamp_to_reachable() {
        let mut target_x = 100.0;
        let mut target_y = 34.0;

        clamp_to_reachable(
            50.0,
            34.0,
            &mut target_x,
            &mut target_y,
            Urgency::Jog,
            3.0, // 3 seconds
        );

        // Jog = 5 m/s, 3s = 15m max
        let dx = target_x - 50.0;
        let dy = target_y - 34.0;
        let dist = (dx * dx + dy * dy).sqrt();

        assert!(dist <= 15.1); // Allow small tolerance
    }
}
