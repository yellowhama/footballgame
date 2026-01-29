//! Cadence policy helpers for DPQ.
//!
//! v1.1: schedule everyone every tick (no behavior change).
//! v1.2: variable cadence based on proximity to ball action.

/// v1.1 cadence: schedule the next decision for the next tick.
#[inline]
pub fn next_due_tick_every_tick(current_tick: u64) -> u64 {
    current_tick.saturating_add(1)
}

// =============================================================================
// v1.2: Variable Cadence (Binary: Active/Passive)
// =============================================================================

/// DPQ v1.2: Cadence level for variable decision frequency.
///
/// Binary approach chosen for simplicity (80% benefit with 20% complexity).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadenceLevel {
    /// Every tick (250ms) - ball owner, pass target, within 20m of ball
    Active,
    /// Every 4 ticks (1000ms) - distant players
    Passive,
}

/// Active zone radius in meters (squared for efficient comparison).
const ACTIVE_ZONE_RADIUS_SQ: f32 = 400.0; // 20m² = 400

/// Passive cadence interval in ticks.
const PASSIVE_CADENCE_TICKS: u64 = 4;

/// DPQ v1.2: Calculate cadence level based on player context.
///
/// # Arguments
/// * `player_idx` - The player's track_id (0-21)
/// * `player_pos_m` - Player position in meters (x, y)
/// * `ball_pos_m` - Ball position in meters (x, y)
/// * `ball_owner` - Current ball owner's track_id, if any
/// * `pass_target` - Current pass target's track_id, if pass in flight
///
/// # Returns
/// `CadenceLevel::Active` if player should decide every tick,
/// `CadenceLevel::Passive` otherwise.
#[inline]
pub fn calculate_cadence_level(
    player_idx: usize,
    player_pos_m: (f32, f32),
    ball_pos_m: (f32, f32),
    ball_owner: Option<usize>,
    pass_target: Option<usize>,
) -> CadenceLevel {
    // Ball owner is always active
    if ball_owner == Some(player_idx) {
        return CadenceLevel::Active;
    }

    // Pass target is always active (needs to prepare for reception)
    if pass_target == Some(player_idx) {
        return CadenceLevel::Active;
    }

    // Within 20m of ball is active
    let dx = player_pos_m.0 - ball_pos_m.0;
    let dy = player_pos_m.1 - ball_pos_m.1;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq <= ACTIVE_ZONE_RADIUS_SQ {
        CadenceLevel::Active
    } else {
        CadenceLevel::Passive
    }
}

/// DPQ v1.2: Calculate next due tick based on cadence level.
#[inline]
pub fn next_due_tick_v1_2(current_tick: u64, level: CadenceLevel) -> u64 {
    match level {
        CadenceLevel::Active => current_tick.saturating_add(1),
        CadenceLevel::Passive => current_tick.saturating_add(PASSIVE_CADENCE_TICKS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cadence_level_ball_owner() {
        let level = calculate_cadence_level(5, (50.0, 34.0), (50.0, 34.0), Some(5), None);
        assert_eq!(level, CadenceLevel::Active);
    }

    #[test]
    fn test_cadence_level_pass_target() {
        // Pass target at 30m from ball, but still active because they're the target
        let level = calculate_cadence_level(7, (80.0, 34.0), (50.0, 34.0), Some(5), Some(7));
        assert_eq!(level, CadenceLevel::Active);
    }

    #[test]
    fn test_cadence_level_within_20m() {
        // 15m from ball (within 20m threshold)
        let level = calculate_cadence_level(3, (65.0, 34.0), (50.0, 34.0), Some(5), None);
        assert_eq!(level, CadenceLevel::Active);
    }

    #[test]
    fn test_cadence_level_exactly_20m() {
        // Exactly 20m from ball (on boundary, should be active)
        let level = calculate_cadence_level(3, (70.0, 34.0), (50.0, 34.0), Some(5), None);
        assert_eq!(level, CadenceLevel::Active);
    }

    #[test]
    fn test_cadence_level_beyond_20m() {
        // 30m from ball (beyond threshold)
        let level = calculate_cadence_level(3, (80.0, 34.0), (50.0, 34.0), Some(5), None);
        assert_eq!(level, CadenceLevel::Passive);
    }

    #[test]
    fn test_next_due_tick_active() {
        assert_eq!(next_due_tick_v1_2(100, CadenceLevel::Active), 101);
    }

    #[test]
    fn test_next_due_tick_passive() {
        assert_eq!(next_due_tick_v1_2(100, CadenceLevel::Passive), 104);
    }
}
