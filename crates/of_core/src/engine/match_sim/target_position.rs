//! Target Position Calculation System
//!
//! This module contains the 5-layer target position calculation for players:
//! - Layer 1: Tactical Goal-based Target (PlayerInstructions)
//! - Layer 2: Context-based Ball Attraction
//! - Layer 2.5: Formation Retention Force
//! - Layer 3: Micro - Separation from Teammates
//! - Layer 4: Defensive Line Maintenance
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::channel_finder::get_opponent_defenders;
use super::MatchEngine;
// Note: offside_trap_state is accessed via self.offside_trap_state (MatchEngine field)
use crate::engine::formation_waypoints::get_formation_waypoints;
use crate::engine::debug_flags::match_debug_enabled;
use crate::engine::movement::{
    // FIX_2601/0105: Use EXPLICIT direction functions - NO Y-flip!
    calculate_defensive_target_explicit,
    // FIX_2601/0110: Dynamic Build-up System integration
    calculate_offensive_target_with_buildup,
    get_fallback_position,
    get_position_role,
    slot_to_position_key,
    BuildupContext,
    PositionRole,
};
use crate::engine::physics_constants::google_football; // FIX_2601/0112: Microfocus constants
use crate::engine::types::{Coord10, DirectionContext};

/// FIX_2601/0112: Microfocus sin_curve - Google Football 스타일 거리 기반 attraction
///
/// 공에 가까울수록 attraction이 강하고, width 이상에서는 0
///
/// # Arguments
/// - `distance`: 공까지 거리 (normalized, 0~1)
/// - `peak`: 최대 attraction 값 (default: 0.15)
/// - `width`: attraction이 0이 되는 거리 (default: 0.25)
fn microfocus_sin_curve(distance: f32, peak: f32, width: f32) -> f32 {
    if distance >= width {
        return 0.0;
    }
    let normalized = distance / width;
    // sin curve: peaks at distance=0, falls to 0 at distance=width
    ((1.0 - normalized) * std::f32::consts::FRAC_PI_2).sin().powi(2) * peak
}

impl MatchEngine {
    // ===========================================
    // Target Position System (5-Layer)
    // ===========================================

    /// Calculate target position for a player based on game state
    ///
    /// FIX_2601/0105: Changed from `is_home: bool` to `ctx: &DirectionContext`
    /// to enable explicit direction-based calculations instead of Y-flip transformations.
    pub(crate) fn calculate_target_position(
        &self,
        player_idx: usize,
        ctx: &DirectionContext,
        home_has_possession: bool,
        score_diff: i8,
    ) -> (f32, f32) {
        let is_home = ctx.is_home; // Extract for backward compatibility during migration

        let (slot, formation) = if is_home {
            (player_idx, &self.home_formation)
        } else {
            (player_idx - 11, &self.away_formation)
        };

        let waypoints = get_formation_waypoints(formation);
        let position_key = slot_to_position_key(slot, formation);

        // FIX_2601/0116: Debug - check if GK is processed
        static CTP_DEBUG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let debug_count = CTP_DEBUG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if (player_idx == 0 || player_idx == 11) && debug_count < 20 {
            let role = get_position_role(position_key);
            eprintln!(
                "[CTP_DEBUG] idx={} slot={} position_key={:?} role={:?} min={}",
                player_idx, slot, position_key, role, self.minute
            );
        }

        let Some(wp) = waypoints.get(&position_key) else {
            return get_fallback_position(slot);
        };

        // Determine if this player's team is attacking
        let team_attacking = (is_home && home_has_possession) || (!is_home && !home_has_possession);

        // Get player for tactical decision (substitution-aware via MatchSetup)
        let player = self.get_match_player(player_idx);

        // ===== LAYER 1: Tactical Goal-based Target =====
        // FIX_2601/0105: Use WORLD coordinates throughout - NO Y-flips!
        // to_normalized_legacy() returns (width, length) = (y/680, x/1050)
        // width.0 = lateral position (0=left, 1=right sideline)
        // length.1 = goal-to-goal position (0=home goal, 1=away goal)
        let player_pos = self.player_positions[player_idx].to_normalized_legacy();
        let ball_pos = self.ball.position.to_normalized_legacy();

        // Calculate distance to ball for defensive decisions
        let dx = player_pos.0 - ball_pos.0;
        let dy = player_pos.1 - ball_pos.1;
        let dist_to_ball = ((dx * dx + dy * dy).sqrt() * 100.0).clamp(0.0, 150.0); // Convert to meters

        // B3: Hero's Will - check if user player
        let is_user = self.is_user_player(player_idx, is_home);

        // Decide tactical goal based on possession
        // FIX_2601/0105: Use EXPLICIT direction functions - returns WORLD coords directly!
        // FIX_2601/0110: Use Dynamic Build-up System for attacking movements
        let mut target = if team_attacking {
            let goal = player.decide_offensive_goal(ball_pos, is_user);

            // FIX_2601/0110: Build context for dynamic channel/triangle calculations
            let buildup_ctx = {
                // Get opponent defender positions
                let opponent_defenders = get_opponent_defenders(is_home, &self.player_positions);

                // Get ball holder index
                let ball_holder_idx = self.ball.current_owner;

                // Build teammate list for triangle formation
                let team_start = if is_home { 0 } else { 11 };
                let teammates: Vec<(usize, _)> = (team_start..team_start + 11)
                    .filter(|&i| i != player_idx) // Exclude self
                    .map(|i| (i, self.player_positions[i]))
                    .collect();

                BuildupContext { opponent_defenders, ball_holder_idx, player_idx, teammates }
            };

            // Use dynamic build-up system for offensive positioning
            calculate_offensive_target_with_buildup(
                goal,
                wp,
                ball_pos,
                player_pos,
                ctx,
                Some(&buildup_ctx),
            )
        } else {
            let goal = player.decide_defensive_goal(dist_to_ball);
            // New explicit function returns WORLD coords - NO conversion needed!
            calculate_defensive_target_explicit(goal, wp, ball_pos, player_pos, ctx)
        };

        // Late game adjustments
        // FIX_2601/0105: Use explicit direction - forward_sign gives +1 (attacks Y=1) or -1 (attacks Y=0)
        let effective_score = if is_home { score_diff } else { -score_diff };
        let forward = ctx.forward_sign() as f32;
        if self.minute >= 75 {
            if effective_score < 0 {
                // Losing - push forward more (toward opponent goal)
                let urgency = ((self.minute - 75) as f32 / 15.0) * 0.08;
                target.1 = (target.1 + urgency * forward).clamp(0.05, 0.95);
            } else if effective_score > 0 && self.minute >= 85 {
                // Winning late - drop back (toward own goal)
                let caution = ((self.minute - 85) as f32 / 10.0) * 0.06;
                target.1 = (target.1 - caution * forward).clamp(0.05, 0.95);
            }
        }

        // ===== P1: PlayerInstructions based positioning =====
        let pi = self.get_player_instructions(player_idx);

        // Width (lateral positioning)
        use crate::player::instructions::Width;
        match pi.width {
            Width::StayWide => {
                // Stay wide: move away from center by 5%
                if target.0 < 0.5 {
                    target.0 = (target.0 - 0.05).max(0.05);
                } else {
                    target.0 = (target.0 + 0.05).min(0.95);
                }
            }
            Width::CutInside => {
                // Cut inside: move toward 0.5 by 10%
                target.0 = target.0 * 0.9 + 0.5 * 0.1;
            }
            Width::Roam => {
                // Roam: follow ball position
                target.0 = target.0 * 0.7 + ball_pos.0 * 0.3;
            }
            Width::Normal => {} // No change
        }

        // Depth (vertical positioning) - only during attack
        // FIX_2601/0105: Use explicit direction for depth adjustments
        use crate::player::instructions::Depth;
        if team_attacking {
            match pi.depth {
                Depth::GetForward => {
                    // Push forward by 8% (toward opponent goal)
                    target.1 = (target.1 + 0.08 * forward).clamp(0.08, 0.92);
                }
                Depth::StayBack => {
                    // Stay back by 8% (toward own goal)
                    target.1 = (target.1 - 0.08 * forward).clamp(0.08, 0.92);
                }
                Depth::Balanced => {} // No change
            }
        }

        // ===== LAYER 2: Microfocus Ball Attraction =====
        // FIX_2601/0112: Google Football 스타일 Microfocus 시스템
        // sin_curve 기반 - 공에 가까울수록 강한 attraction, 멀면 0
        let role = get_position_role(position_key);

        // FIX_2601/0105: All coords are now in WORLD space - no conversion needed
        // Calculate distance to ball (Euclidean) in WORLD coords
        let dx = player_pos.0 - ball_pos.0;
        let dy = player_pos.1 - ball_pos.1;
        let dist = (dx * dx + dy * dy).sqrt();

        // FIX_2601/0112: Microfocus sin_curve 기반 attraction
        // peak=0.15 (최대 15% 끌림), width=0.25 (25% 거리 이상에서 0)
        let base_attraction = microfocus_sin_curve(
            dist,
            google_football::MICROFOCUS_PEAK,  // 0.15
            google_football::MICROFOCUS_WIDTH, // 0.25
        );

        // Role-based modifiers (Microfocus strength per role)
        let ball_influence = base_attraction
            * match role {
                PositionRole::Goalkeeper => 0.0, // GK stays in position
                PositionRole::Defender => 0.7,   // Defenders: conservative (70%)
                PositionRole::Midfielder => 0.8, // Midfielders: medium (80%)
                PositionRole::Forward => {
                    // FIX_2601/0105: Use explicit direction for "ball is ahead" check
                    let ball_is_ahead = if ctx.attacks_right {
                        ball_pos.1 > player_pos.1
                    } else {
                        ball_pos.1 < player_pos.1
                    };
                    if ball_is_ahead {
                        0.9
                    } else {
                        0.8
                    } // Forwards: aggressive (80-90%)
                }
            };

        // Apply ball attraction to BOTH X and Y axes
        target.0 = target.0 * (1.0 - ball_influence) + ball_pos.0 * ball_influence;
        target.1 = target.1 * (1.0 - ball_influence) + ball_pos.1 * ball_influence;

        // ===== LAYER 2.5: Formation Retention Force =====
        // FIX_2601/0110: Only active when team LOST possession (transition defense)
        // When attacking: OFF - allow free attacking runs
        // When defending: ON - pull back to formation for defensive shape
        // FIX_2601/0105: Convert waypoint to WORLD coords
        let formation_base_y = if ctx.attacks_right {
            wp.defensive.1 // Home: use as-is
        } else {
            1.0 - wp.defensive.1 // Away: flip to world
        };
        let formation_base = (wp.defensive.0, formation_base_y);

        // Formation pull ONLY when team doesn't have ball
        let formation_pull = if !team_attacking {
            // Lost possession - pull back to formation
            // dist > 0.2 (20m) = start pulling, stronger than before (50% max)
            if dist > 0.2 {
                ((dist - 0.2) / 0.3).clamp(0.0, 0.5) // max 50% pull toward formation
            } else {
                0.0
            }
        } else {
            // Team has ball - NO formation pull, allow free attacking runs
            0.0
        };

        if formation_pull > 0.0 {
            target.0 = target.0 * (1.0 - formation_pull) + formation_base.0 * formation_pull;
            target.1 = target.1 * (1.0 - formation_pull) + formation_base.1 * formation_pull;
        }

        // ===== LAYER 3: Micro - Separation from Teammates =====
        let mut separation = (0.0_f32, 0.0_f32);
        let sep_radius = 0.04_f32; // ~4m on normalized field

        // D5-5: Single-pass separation (bounded: exactly 11 iterations)
        // Future multi-iteration solvers should use MAX_SEPARATION_ITERATIONS
        // Check distance to all teammates
        let team_start = if is_home { 0 } else { 11 };
        let team_end = team_start + 11;

        for other_idx in team_start..team_end {
            if other_idx == player_idx {
                continue;
            }

            // FIX_2601/0105: Use WORLD coords for separation calculation (no Y-flip needed)
            // Separation is distance-based, works the same in world coords for both teams
            let other_pos = self.player_positions[other_idx].to_normalized_legacy();
            let ox = player_pos.0 - other_pos.0;
            let oy = player_pos.1 - other_pos.1;
            let other_dist = (ox * ox + oy * oy).sqrt();

            if other_dist < sep_radius && other_dist > 0.001 {
                // Push away proportional to overlap
                let strength = (sep_radius - other_dist) / sep_radius;
                separation.0 += (ox / other_dist) * strength;
                separation.1 += (oy / other_dist) * strength;
            }
        }

        // Apply separation with small factor to avoid breaking formation
        let sep_factor = 0.15;
        target.0 += separation.0 * sep_factor;
        target.1 += separation.1 * sep_factor;

        // ===== LAYER 4: Defensive Line Maintenance =====
        // FIX_2601/0107 Phase 8: Enhanced with offside trap integration
        // Defenders should maintain a consistent line (same y-axis position)
        if role == PositionRole::Defender {
            // Get defender indices for this team (slots 1-4 for 4-back, 1-5 for 5-back)
            let team_start = if is_home { 0 } else { 11 };
            let team_idx = if is_home { 0 } else { 1 };
            let defender_indices: Vec<usize> = (1..=5)
                .map(|i| team_start + i)
                .filter(|&idx| {
                    idx < self.player_positions.len()
                        && get_position_role(slot_to_position_key(
                            if is_home { idx } else { idx - 11 },
                            if is_home { &self.home_formation } else { &self.away_formation },
                        )) == PositionRole::Defender
                })
                .collect();

            if defender_indices.len() >= 2 {
                // FIX_2601/0107 Phase 8: Check offside trap state
                let trap_state = &self.offside_trap_state[team_idx];

                // Determine line position: use trap line if active, else average
                let (line_y, line_strength) = if trap_state.trap_active {
                    // Offside trap active: use calculated trap line
                    // Convert Coord10 to normalized (0-1000 → 0.0-1.0)
                    let trap_line_normalized = trap_state.line_y as f32 / 1000.0;

                    // Strength based on coordination (higher teamwork = tighter line)
                    // Base 60% + up to 30% from coordination
                    let strength = 0.6 + trap_state.coordination * 0.3;

                    (trap_line_normalized, strength)
                } else {
                    // No trap: use average defender position
                    // FIX_2601/0105: Use WORLD coords - defensive line works in world space
                    let avg_y: f32 = defender_indices
                        .iter()
                        .map(|&idx| self.player_positions[idx].to_normalized_legacy().1)
                        .sum::<f32>()
                        / defender_indices.len() as f32;

                    (avg_y, 0.6)
                };

                // Pull this defender's target toward the line
                target.1 = target.1 * (1.0 - line_strength) + line_y * line_strength;

                // FIX_2601/0107 Phase 8: Also align X position for compact shape when trap active
                if trap_state.trap_active && trap_state.coordination > 0.7 {
                    // High coordination: compress defensive width
                    let center_x = 0.5;
                    let compress_strength = (trap_state.coordination - 0.7) * 0.5; // 0-15% compression
                    target.0 = target.0 * (1.0 - compress_strength) + center_x * compress_strength;
                }
            }
        }

        // ===== LAYER 4.5: GK Sweeping State =====
        // FIX_2601/0107 Phase 8: Goalkeeper positioning based on sweeping state
        if role == PositionRole::Goalkeeper {
            let team_idx = if is_home { 0 } else { 1 };
            let sweeping_state = self.gk_sweeping_state[team_idx];

            // FIX_2601/0116: Must pass GK player index (0 or 11), not team index!
            let gk_player_idx = if is_home { 0 } else { 11 };
            let sweeping_target = self.get_gk_target_position(gk_player_idx);

            // FIX_2601/0117: Convert Coord10 to normalized_legacy format (width, length)
            // Coord10: x = length (0-1050), y = width (0-680)
            // normalized_legacy: (width, length) = (y/680, x/1050)
            // THIS WAS SWAPPED BEFORE - caused GK to wander toward wrong axis!
            // Use to_normalized_legacy() for correct conversion.
            let sweeping_target_normalized = sweeping_target.to_normalized_legacy();

            // Blend weight based on sweeping state
            use super::gk_sweeping::GKSweepingState;
            let sweeping_weight = match sweeping_state {
                GKSweepingState::Attentive => 0.3,        // Slight adjustment toward optimal
                GKSweepingState::ComingOut => 0.9,        // Strong pull toward rushing target
                GKSweepingState::ReturningToGoal => 0.7,  // Pull back toward goal
                GKSweepingState::PreparingForSave => 0.5, // Moderate adjustment
            };

            // Apply sweeping position blend
            target.0 = target.0 * (1.0 - sweeping_weight)
                + sweeping_target_normalized.0 * sweeping_weight;
            target.1 = target.1 * (1.0 - sweeping_weight)
                + sweeping_target_normalized.1 * sweeping_weight;

            // FIX_2601/0117: Debug GK target position (temporary)
            static GK_DEBUG_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let count = GK_DEBUG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 10 {
                let is_second_half_flag = if self.minute >= 45 { "2H" } else { "1H" };
                eprintln!(
                    "[GK_TARGET] {} min={} gk_idx={} sweeping_target=({:.3},{:.3}) target=({:.3},{:.3}) weight={:.1}",
                    is_second_half_flag, self.minute, gk_player_idx,
                    sweeping_target_normalized.0, sweeping_target_normalized.1,
                    target.0, target.1, sweeping_weight
                );
            }
        }

        // Clamp to valid range
        target.0 = target.0.clamp(0.05, 0.95);
        target.1 = target.1.clamp(0.05, 0.95);

        // FIX_2601/0105: No final Y-flip needed - all calculations use WORLD coords consistently.

        // DEBUG: Print first call info (once)
        static PRINTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if player_idx == 0 && !PRINTED.load(std::sync::atomic::Ordering::Relaxed) {
            PRINTED.store(true, std::sync::atomic::Ordering::Relaxed);
            #[cfg(debug_assertions)]
            if match_debug_enabled() {
                eprintln!(
                    "[DEBUG-0105] player_idx=0 is_home={} target=({:.3},{:.3}) pos=({:.3},{:.3}) ctx.attacks_right={}",
                    is_home, target.0, target.1, player_pos.0, player_pos.1, ctx.attacks_right
                );
            }
        }

        target
    }

    /// FIX_2601: Calculate target position returning Coord10
    ///
    /// Wrapper that converts the result to Coord10 for direct use.
    pub(crate) fn calculate_target_position_coord10(
        &self,
        player_idx: usize,
        ctx: &DirectionContext,
        home_has_possession: bool,
        score_diff: i8,
    ) -> Coord10 {
        let target = self.calculate_target_position(player_idx, ctx, home_has_possession, score_diff);
        // Convert normalized (width, length) to Coord10 (x=length, y=width)
        Coord10::from_normalized_legacy(target)
    }
}
