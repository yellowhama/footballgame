//! Slow evaluation layer for complex decision-making
//!
//! FIX_2512 Phase 13: process_slow() pattern from Open Football
//! FIX_2512 Phase 15: Team-aware space quality integration
//! FIX_2512 Phase 16: Context Unification (uses CoreContext)
//! P3.1: Zone escape evaluation for crowded zone detection
//!
//! This module provides complex evaluation logic that runs less frequently
//! than try_fast_transition(), enabling context-aware decisions for:
//! - Pass target selection (distance + team-aware space + passing lane quality + pressure)
//! - Penetration timing (offside line + CB gap + ball holder distance)
//! - Pressing triggers (support + zone + opponent space quality)
//! - Zone escape (when current zone exceeds density threshold)

use crate::engine::core_context::CoreContext;
use crate::engine::match_sim::quality_metrics::FieldZone;
use crate::engine::operations::{PlayerDistanceMatrix, SpaceAnalysis};
use crate::engine::position_substates::PositionSubState;
use crate::engine::positioning::PositionKey;
use crate::engine::physics_constants::field;
use crate::engine::role_transition::{
    apply_role_weight_to_score, get_adjusted_role_weight, RoleTransitionMatrix,
};
use crate::engine::types::Coord10;
use crate::models::player::PlayerAttributes;

/// Context for slow evaluation (extends CoreContext with caches)
///
/// FIX_2512 Phase 16: Now uses CoreContext for shared player state,
/// reducing duplication with PositionContext.
///
/// Role Transition Matrix: Optional role-based pass weighting
pub struct SlowContext<'a> {
    /// Core player state (shared with PositionContext)
    pub core: CoreContext,

    // Cached data references
    pub distance_matrix: &'a PlayerDistanceMatrix,
    pub space_analysis: &'a SpaceAnalysis,

    // Player positions for detailed queries (slice allows Vec or array)
    pub positions: &'a [Coord10],

    // Role Transition Matrix (optional - for role-based pass weighting)
    /// Role transition matrix for pass weighting
    pub role_matrix: Option<&'a RoleTransitionMatrix>,

    /// Player attributes for role weight adjustments (22 players)
    pub player_attributes: Option<&'a [PlayerAttributes]>,

    /// Player role mapping (22 players: player_idx → PositionKey)
    pub player_roles: Option<&'a [PositionKey]>,
}

impl<'a> SlowContext<'a> {
    // ========================================================================
    // Convenience accessors for CoreContext fields (backward compatibility)
    // ========================================================================

    #[inline]
    pub fn player_idx(&self) -> usize {
        self.core.player_idx
    }

    #[inline]
    pub fn is_home(&self) -> bool {
        self.core.is_home
    }

    #[inline]
    pub fn player_position(&self) -> (f32, f32) {
        self.core.player_position.to_meters()
    }

    #[inline]
    pub fn ball_position(&self) -> (f32, f32) {
        self.core.ball_position.to_meters()
    }

    #[inline]
    pub fn team_has_ball(&self) -> bool {
        self.core.team_has_ball
    }

    #[inline]
    pub fn player_has_ball(&self) -> bool {
        self.core.player_has_ball
    }

    // ========================================================================
    // Team-related helpers
    // ========================================================================

    /// Get teammate indices for this player
    pub fn teammate_indices(&self) -> impl Iterator<Item = usize> + '_ {
        let range = if self.core.is_home { 0..11 } else { 11..22 };
        range.filter(move |&i| i != self.core.player_idx)
    }

    /// Get opponent indices for this player
    pub fn opponent_indices(&self) -> impl Iterator<Item = usize> + '_ {
        if self.core.is_home {
            11..22
        } else {
            0..11
        }
    }

    /// Get player position in meters
    pub fn get_player_position(&self, idx: usize) -> (f32, f32) {
        self.positions[idx].to_meters()
    }

    // ========================================================================
    // Role Transition Matrix helpers
    // ========================================================================

    /// Get player role (if role mapping is available)
    pub fn get_player_role(&self, idx: usize) -> Option<PositionKey> {
        self.player_roles.and_then(|roles| roles.get(idx).copied())
    }

    /// Get player attributes (if available)
    pub fn get_player_attrs(&self, idx: usize) -> Option<&PlayerAttributes> {
        self.player_attributes.and_then(|attrs| attrs.get(idx))
    }

    /// Check if role-based pass weighting is enabled
    pub fn has_role_matrix(&self) -> bool {
        self.role_matrix.is_some() && self.player_roles.is_some()
    }
}

/// Result of slow evaluation
#[derive(Debug, Clone, Default)]
pub struct SlowEvaluationResult {
    /// Suggested state transition (if any)
    pub suggested_state: Option<PositionSubState>,

    /// Best pass target (for ball holders)
    pub best_pass_target: Option<PassTarget>,

    /// Should trigger penetration run
    pub should_penetrate: bool,

    /// Penetration score (0.0 - 1.0)
    pub penetration_score: f32,

    /// Pressing intensity (0.0 = hold, 1.0 = full press)
    pub pressing_intensity: f32,

    /// P3.1: Zone escape urgency (0.0 = stay, > 1.0 = crowded, should escape)
    pub zone_escape_urgency: f32,

    /// P3.1: Direction toward least crowded adjacent zone
    pub zone_escape_direction: (f32, f32),
}

/// Evaluated pass target
#[derive(Debug, Clone, Copy)]
pub struct PassTarget {
    pub target_idx: usize,
    pub score: f32,        // 0.0 - 1.0
    pub distance: f32,
    pub space_quality: f32,
    pub path_blocked: bool,
}

// ============================================================================
// PassTargetEvaluator
// ============================================================================

/// Evaluates pass targets using cached data
pub struct PassTargetEvaluator;

impl PassTargetEvaluator {
    /// Evaluate all potential pass targets
    /// Returns sorted list by score (best first)
    pub fn evaluate_targets(ctx: &SlowContext) -> Vec<PassTarget> {
        let mut targets = Vec::with_capacity(10);

        for teammate_idx in ctx.teammate_indices() {
            let target = Self::evaluate_single(ctx, teammate_idx);
            if target.score > 0.1 {
                // Minimum viable target
                targets.push(target);
            }
        }

        // Sort by score descending
        targets.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        targets
    }

    fn evaluate_single(ctx: &SlowContext, target_idx: usize) -> PassTarget {
        // 1. Distance score (prefer 10-30m, penalize <5m and >40m)
        let distance = ctx.distance_matrix.get(ctx.core.player_idx, target_idx);
        let distance_score = Self::distance_score(distance);

        // 2. Space quality at target position (team-aware: opponents penalized more)
        let target_pos = ctx.get_player_position(target_idx);
        let space_quality = ctx
            .space_analysis
            .space_quality_for_team(target_pos, ctx.core.is_home);

        // 3. Passing lane quality (team-aware: considers opponent blocking)
        let lane_quality = ctx
            .space_analysis
            .passing_lane_quality(ctx.core.player_position.to_meters(), target_pos, ctx.core.is_home);
        let path_blocked = lane_quality < 0.4;

        // 4. Opponent proximity penalty
        let nearest_opp_dist = Self::nearest_opponent_to_target(ctx, target_idx);
        let pressure_penalty = if nearest_opp_dist < 3.0 {
            0.4
        } else if nearest_opp_dist < 6.0 {
            0.7
        } else {
            1.0
        };

        // FIX_2512_1230: 5. Progression score (forward passes get bonus, backward penalized)
        // HOME attacks toward X=105, AWAY attacks toward X=0
        let passer_x = ctx.core.player_position.to_meters().0;
        let target_x = target_pos.0;
        let progression = if ctx.core.attacks_right {
            target_x - passer_x  // Attacking right: positive = forward
        } else {
            passer_x - target_x  // Attacking left: positive = forward
        };
        let progression_score = if progression > 15.0 {
            1.0  // Big forward pass
        } else if progression > 5.0 {
            0.85  // Moderate forward pass
        } else if progression > 0.0 {
            0.6  // Slight forward pass
        } else if progression > -5.0 {
            0.35  // Lateral/slight backward
        } else {
            0.05  // Back pass (heavily penalized - forces forward play)
        };

        // FIX_2512_1230 v2: Increased progression weight to 40%, reduced distance
        // This encourages teams to play forward more aggressively
        let base_score = distance_score * 0.10
            + space_quality * 0.20
            + lane_quality * 0.15
            + pressure_penalty * 0.15
            + progression_score * 0.40;

        // Role Transition Matrix: Apply role-based pass weighting
        let score = if ctx.has_role_matrix() {
            Self::apply_role_weight(ctx, target_idx, base_score)
        } else {
            base_score
        };

        PassTarget {
            target_idx,
            score,
            distance,
            space_quality,
            path_blocked,
        }
    }

    /// Apply role-based weight to base score
    fn apply_role_weight(ctx: &SlowContext, target_idx: usize, base_score: f32) -> f32 {
        let Some(matrix) = ctx.role_matrix else {
            return base_score;
        };

        let Some(holder_role) = ctx.get_player_role(ctx.core.player_idx) else {
            return base_score;
        };

        let Some(target_role) = ctx.get_player_role(target_idx) else {
            return base_score;
        };

        // Get base weight from matrix
        let base_weight = matrix.get_weight(holder_role, target_role);

        // Adjust weight based on player attributes
        let holder_attrs = ctx.get_player_attrs(ctx.core.player_idx);
        let target_attrs = ctx.get_player_attrs(target_idx);

        let adjusted_weight = get_adjusted_role_weight(
            base_weight,
            holder_attrs,
            target_attrs,
            holder_role,
            target_role,
        );

        // Apply weight to score using additive formula
        apply_role_weight_to_score(base_score, adjusted_weight)
    }

    fn distance_score(dist: f32) -> f32 {
        match dist {
            d if d < 5.0 => 0.3,  // Too close
            d if d < 10.0 => 0.7, // Short pass
            d if d < 30.0 => 1.0, // Ideal range
            d if d < 40.0 => 0.6, // Long pass risk
            _ => 0.2,             // Very long
        }
    }

    fn nearest_opponent_to_target(ctx: &SlowContext, target_idx: usize) -> f32 {
        ctx.opponent_indices()
            .map(|opp| ctx.distance_matrix.get(target_idx, opp))
            .fold(f32::MAX, f32::min)
    }
}

// ============================================================================
// PenetrationEvaluator
// ============================================================================

/// Evaluates penetration run timing
pub struct PenetrationEvaluator;

impl PenetrationEvaluator {
    /// Check if player should start a penetrating run
    /// Returns (should_penetrate, score)
    pub fn should_penetrate(ctx: &SlowContext) -> (bool, f32) {
        // Only when team has ball but player doesn't
        if !ctx.core.team_has_ball || ctx.core.player_has_ball {
            return (false, 0.0);
        }

        // FIX_2512_1230: Offside is determined by X-axis (field length), not Y-axis
        // 1. Check defensive line position (estimate offside line)
        let offside_line_x = Self::estimate_offside_line(ctx);
        let player_pos = ctx.core.player_position.to_meters();
        let player_x = player_pos.0;

        // Can't be too far ahead (offside risk)
        // Attacks right: being ahead means higher X
        let offside_risk = if ctx.core.attacks_right {
            player_x > offside_line_x + 2.0
        } else {
            player_x < offside_line_x - 2.0
        };

        if offside_risk {
            return (false, 0.0);
        }

        // 2. Check space behind defense (team-aware)
        // "Behind defense" means closer to opponent's goal than the offside line
        let behind_defense_x = if ctx.core.attacks_right {
            (offside_line_x + 5.0).min(field::LENGTH_M)
        } else {
            (offside_line_x - 5.0).max(0.0)
        };
        let behind_defense_pos = (behind_defense_x, player_pos.1);
        let space_behind = ctx
            .space_analysis
            .space_quality_for_team(behind_defense_pos, ctx.core.is_home);

        // 3. Check ball holder distance (can they see the run?)
        let ball_holder_dist = Self::distance_to_ball(ctx);
        let visibility_score = if ball_holder_dist < 30.0 {
            1.0
        } else if ball_holder_dist < 45.0 {
            0.6
        } else {
            0.2
        };

        // 4. Check CB gap (simplified: check space quality in center, team-aware)
        // FIX_2512_1230: Use X for depth, Y=CENTER_Y for center of field (width)
        let center_behind = (behind_defense_x, field::CENTER_Y);
        let gap_quality = ctx
            .space_analysis
            .space_quality_for_team(center_behind, ctx.core.is_home);
        let gap_score = if gap_quality > 0.7 {
            1.0
        } else if gap_quality > 0.4 {
            0.7
        } else {
            0.3
        };

        // Combined score
        let penetration_score = space_behind * 0.4 + visibility_score * 0.3 + gap_score * 0.3;

        (penetration_score > 0.6, penetration_score)
    }

    fn estimate_offside_line(ctx: &SlowContext) -> f32 {
        // FIX_2512_1230: Offside is determined by X-axis (field length), not Y-axis
        // FIX_2601/0123: Include GK in defender range (bug: was 1..11, missing Home GK at index 0)
        // This fixes the offside bias where Away team had ~2x more offsides
        let defender_range = if ctx.core.is_home { 11..22 } else { 0..11 };

        // Attacks right: look for defender with highest X
        // Attacks left: look for defender with lowest X
        let mut last_defender_x = if ctx.core.attacks_right { 0.0f32 } else { field::LENGTH_M };

        for i in defender_range {
            let pos = ctx.get_player_position(i);
            if ctx.core.attacks_right {
                // Attacking right: opponent's deepest defender has highest X
                if pos.0 > last_defender_x {
                    last_defender_x = pos.0;
                }
            } else {
                // Attacking left: opponent's deepest defender has lowest X
                if pos.0 < last_defender_x {
                    last_defender_x = pos.0;
                }
            }
        }

        last_defender_x
    }

    fn distance_to_ball(ctx: &SlowContext) -> f32 {
        let player_pos = ctx.core.player_position.to_meters();
        let ball_pos = ctx.core.ball_position.to_meters();
        let dx = player_pos.0 - ball_pos.0;
        let dy = player_pos.1 - ball_pos.1;
        (dx * dx + dy * dy).sqrt()
    }
}

// ============================================================================
// PressingEvaluator
// ============================================================================

/// Field third for pressing zone calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldThird {
    Attacking,
    Middle,
    Defensive,
}

/// Evaluates pressing trigger and intensity
pub struct PressingEvaluator;

impl PressingEvaluator {
    /// Determine pressing intensity for this player
    /// Returns value between 0.0 (hold) and 1.0 (full press)
    pub fn evaluate_pressing(ctx: &SlowContext) -> f32 {
        if ctx.core.team_has_ball {
            return 0.0; // No pressing when in possession
        }

        // 1. Distance to ball
        let ball_dist = Self::distance_to_ball(ctx);
        let proximity_factor = if ball_dist < 8.0 {
            1.0
        } else if ball_dist < 15.0 {
            0.7
        } else if ball_dist < 25.0 {
            0.3
        } else {
            0.0
        };

        // 2. Teammate support (can we trap?)
        let nearby_teammates = Self::count_nearby_teammates(ctx, 10.0);
        let support_factor = if nearby_teammates >= 2 {
            1.0
        } else if nearby_teammates == 1 {
            0.6
        } else {
            0.3
        };

        // 3. Field position (press higher up the pitch)
        let field_third = Self::get_field_third(ctx);
        let zone_factor = match field_third {
            FieldThird::Attacking => 1.0,
            FieldThird::Middle => 0.7,
            FieldThird::Defensive => 0.4,
        };

        // 4. Opponent space (press if they're crowded - use opponent's team perspective)
        let opp_space = ctx
            .space_analysis
            .space_quality_for_team(ctx.core.ball_position.to_meters(), !ctx.core.is_home);
        let pressure_opportunity = 1.0 - opp_space; // Invert: crowded = good to press

        // Combined intensity
        let intensity = proximity_factor * 0.35
            + support_factor * 0.25
            + zone_factor * 0.20
            + pressure_opportunity * 0.20;

        intensity.clamp(0.0, 1.0)
    }

    fn distance_to_ball(ctx: &SlowContext) -> f32 {
        let player_pos = ctx.core.player_position.to_meters();
        let ball_pos = ctx.core.ball_position.to_meters();
        let dx = player_pos.0 - ball_pos.0;
        let dy = player_pos.1 - ball_pos.1;
        (dx * dx + dy * dy).sqrt()
    }

    fn count_nearby_teammates(ctx: &SlowContext, radius: f32) -> usize {
        ctx.teammate_indices()
            .filter(|&t| ctx.distance_matrix.get(ctx.core.player_idx, t) < radius)
            .count()
    }

    fn get_field_third(ctx: &SlowContext) -> FieldThird {
        // FIX_2512_1229: Use X-axis (length=105m), not Y-axis (width=68m)!
        // Field thirds are based on length: 0-35m, 35-70m, 70-105m
        let x = ctx.core.player_position.to_meters().0;
        if ctx.core.attacks_right {
            if x > 70.0 {
                FieldThird::Attacking
            } else if x > 35.0 {
                FieldThird::Middle
            } else {
                FieldThird::Defensive
            }
        } else if x < 35.0 {
            FieldThird::Attacking
        } else if x < 70.0 {
            FieldThird::Middle
        } else {
            FieldThird::Defensive
        }
    }
}

// ============================================================================
// ZoneEscapeEvaluator (P3.1)
// ============================================================================

/// P3.1: Evaluates zone escape triggers for crowded zones
///
/// When a player is in a zone with density exceeding the expected threshold,
/// this evaluator triggers zone escape behavior (CreatingSpace for midfielders/forwards).
pub struct ZoneEscapeEvaluator;

impl ZoneEscapeEvaluator {
    /// Evaluate zone escape urgency
    ///
    /// Returns (urgency, escape_direction):
    /// - urgency: 0.0 = no need to escape, > 1.0 = crowded, higher = more urgent
    /// - escape_direction: unit direction toward least crowded adjacent zone
    pub fn evaluate_zone_escape(ctx: &SlowContext) -> (f32, (f32, f32)) {
        // Only evaluate when team has ball but player doesn't (off-ball movement)
        if !ctx.core.team_has_ball || ctx.core.player_has_ball {
            return (0.0, (0.0, 0.0));
        }

        // P3.7: Use 5m radius (matches cluster anomaly measurement)
        // instead of 10m grid cells
        let nearby_count = ctx
            .distance_matrix
            .players_within(ctx.core.player_idx, 5.0)
            .len() as u32;

        // Get zone-specific threshold
        let player_pos_m = ctx.core.player_position.to_meters();
        let zone = FieldZone::from_position(
            player_pos_m.0,
            player_pos_m.1,
        );
        let max_density = zone.max_expected_density();

        // Calculate urgency (0 if under threshold, ratio if over)
        let urgency = if nearby_count > max_density {
            (nearby_count as f32 / max_density as f32).min(2.0) // Cap at 2.0
        } else {
            0.0
        };

        // Get escape direction (toward least crowded adjacent zone)
        let escape_direction = if urgency > 0.0 {
            ctx.space_analysis.least_crowded_zone_direction(ctx.core.player_position.to_meters())
        } else {
            (0.0, 0.0)
        };

        (urgency, escape_direction)
    }

    /// Check if zone escape should trigger CreatingSpace state
    ///
    /// Only triggers for midfielders and forwards when zone is crowded
    /// and team has possession but player doesn't have the ball.
    pub fn should_create_space(ctx: &SlowContext) -> bool {
        let (urgency, _) = Self::evaluate_zone_escape(ctx);
        urgency > 1.0 // Only trigger when density exceeds threshold
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_positions() -> [Coord10; 22] {
        std::array::from_fn(|i| {
            let x = if i < 11 {
                20.0 + (i as f32 * 7.0)
            } else {
                80.0 - ((i - 11) as f32 * 7.0)
            };
            let y = 30.0 + ((i % 5) as f32 * 10.0);
            Coord10::from_meters(x, y)
        })
    }

    /// Helper to create SlowContext with CoreContext (Phase 16)
    fn make_slow_context<'a>(
        player_idx: usize,
        player_position: (f32, f32),
        ball_position: (f32, f32),
        team_has_ball: bool,
        player_has_ball: bool,
        is_home: bool,
        distance_matrix: &'a PlayerDistanceMatrix,
        space_analysis: &'a SpaceAnalysis,
        positions: &'a [Coord10],
    ) -> SlowContext<'a> {
        let dx = player_position.0 - ball_position.0;
        let dy = player_position.1 - ball_position.1;
        let ball_distance = (dx * dx + dy * dy).sqrt();

        SlowContext {
            core: CoreContext {
                player_idx,
                is_home,
                attacks_right: is_home,
                player_position: Coord10::from_meters(player_position.0, player_position.1),
                ball_position: Coord10::from_meters(ball_position.0, ball_position.1),
                ball_distance,
                team_has_ball,
                player_has_ball,
            },
            distance_matrix,
            space_analysis,
            positions,
            // Role Transition Matrix fields (optional)
            role_matrix: None,
            player_attributes: None,
            player_roles: None,
        }
    }

    /// Helper to create SlowContext with role matrix for testing
    fn make_slow_context_with_roles<'a>(
        player_idx: usize,
        player_position: (f32, f32),
        ball_position: (f32, f32),
        team_has_ball: bool,
        player_has_ball: bool,
        is_home: bool,
        distance_matrix: &'a PlayerDistanceMatrix,
        space_analysis: &'a SpaceAnalysis,
        positions: &'a [Coord10],
        role_matrix: &'a RoleTransitionMatrix,
        player_roles: &'a [PositionKey],
    ) -> SlowContext<'a> {
        let dx = player_position.0 - ball_position.0;
        let dy = player_position.1 - ball_position.1;
        let ball_distance = (dx * dx + dy * dy).sqrt();

        SlowContext {
            core: CoreContext {
                player_idx,
                is_home,
                attacks_right: is_home,
                player_position: Coord10::from_meters(player_position.0, player_position.1),
                ball_position: Coord10::from_meters(ball_position.0, ball_position.1),
                ball_distance,
                team_has_ball,
                player_has_ball,
            },
            distance_matrix,
            space_analysis,
            positions,
            role_matrix: Some(role_matrix),
            player_attributes: None,
            player_roles: Some(player_roles),
        }
    }

    #[test]
    fn test_pass_target_distance_score() {
        assert!((PassTargetEvaluator::distance_score(3.0) - 0.3).abs() < 0.01);
        assert!((PassTargetEvaluator::distance_score(7.0) - 0.7).abs() < 0.01);
        assert!((PassTargetEvaluator::distance_score(20.0) - 1.0).abs() < 0.01);
        assert!((PassTargetEvaluator::distance_score(35.0) - 0.6).abs() < 0.01);
        assert!((PassTargetEvaluator::distance_score(50.0) - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_field_third_home() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let sa = SpaceAnalysis::new();

        // HOME attacks right: X=75 (>70) is attacking third
        let ctx = make_slow_context(
            0,
            (75.0, field::CENTER_Y),
            (70.0, field::CENTER_Y),
            false,
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        assert_eq!(PressingEvaluator::get_field_third(&ctx), FieldThird::Attacking);
    }

    #[test]
    fn test_field_third_away() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let sa = SpaceAnalysis::new();

        // AWAY attacks left: X=30 (<35) is attacking third
        let ctx = make_slow_context(
            15,
            (30.0, field::CENTER_Y),
            (35.0, field::CENTER_Y),
            false,
            false,
            false,
            &dm,
            &sa,
            &positions,
        );

        assert_eq!(PressingEvaluator::get_field_third(&ctx), FieldThird::Attacking);
    }

    #[test]
    fn test_pressing_zero_when_team_has_ball() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let sa = SpaceAnalysis::new();

        let ctx = make_slow_context(
            5,
            (field::CENTER_X, 50.0),
            (50.0, 50.0),
            true, // Team has ball
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        assert_eq!(PressingEvaluator::evaluate_pressing(&ctx), 0.0);
    }

    #[test]
    fn test_penetration_blocked_when_has_ball() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let sa = SpaceAnalysis::new();

        let ctx = make_slow_context(
            9,
            (field::CENTER_X, 70.0),
            (field::CENTER_X, 70.0),
            true,
            true, // Has ball - can't penetrate
            true,
            &dm,
            &sa,
            &positions,
        );

        let (should, _) = PenetrationEvaluator::should_penetrate(&ctx);
        assert!(!should);
    }

    #[test]
    fn test_pass_target_evaluator_returns_sorted() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[5].to_meters();
        let ctx = make_slow_context(
            5,
            pos_m,
            pos_m,
            true,
            true,
            true,
            &dm,
            &sa,
            &positions,
        );

        let targets = PassTargetEvaluator::evaluate_targets(&ctx);

        // Verify sorted by score descending
        for i in 1..targets.len() {
            assert!(targets[i - 1].score >= targets[i].score);
        }
    }

    #[test]
    fn test_pass_target_team_aware_space_quality() {
        // Phase 15: Test that pass targets use team-aware space quality
        // Create positions where one teammate is in open space and another is crowded by opponents

        // Custom positions: player 0 is passer at center
        // Player 1 is in open area (left wing, away from opponents)
        // Player 2 is crowded by opponents (center where away team is clustered)
        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Home team (0-10)
        positions[0] = Coord10::from_meters(field::CENTER_X, 50.0); // Passer at center
        positions[1] = Coord10::from_meters(15.0, 50.0); // Open target on left wing
        positions[2] = Coord10::from_meters(field::CENTER_X, 70.0); // Target in crowded area
        for i in 3..11 {
            positions[i] = Coord10::from_meters(30.0 + (i as f32 * 2.0), 30.0);
        }

        // Away team (11-21): clustered near center-forward area
        for i in 11..22 {
            positions[i] = Coord10::from_meters(
                50.0 + ((i - 11) as f32 * 2.0),
                field::WIDTH_M + ((i - 11) as f32 % 3.0),
            );
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[0].to_meters();
        let ctx = make_slow_context(
            0,
            pos_m,
            pos_m,
            true,
            true,
            true,
            &dm,
            &sa,
            &positions,
        );

        // Evaluate both targets individually
        let target1 = PassTargetEvaluator::evaluate_single(&ctx, 1); // Open target
        let target2 = PassTargetEvaluator::evaluate_single(&ctx, 2); // Crowded target

        // Target 1 (left wing, open) should have better space quality than target 2 (crowded)
        assert!(
            target1.space_quality > target2.space_quality,
            "Open target should have higher space quality: {} vs {}",
            target1.space_quality,
            target2.space_quality
        );
    }

    #[test]
    fn test_pressing_uses_opponent_space_quality() {
        // Phase 15: Test that pressing evaluator considers opponent's space quality
        // When opponents are crowded (by our team), pressing opportunity should be high

        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Home team (0-10): pressing player near ball, others nearby for support
        positions[0] = Coord10::from_meters(50.0, 50.0); // Pressing player
        positions[1] = Coord10::from_meters(45.0, 48.0); // Support 1
        positions[2] = Coord10::from_meters(55.0, 48.0); // Support 2
        for i in 3..11 {
            positions[i] = Coord10::from_meters(30.0, 30.0 + (i as f32 * 5.0));
        }

        // Away team (11-21): clustered around ball position (crowded)
        positions[11] = Coord10::from_meters(52.0, 52.0); // Ball carrier
        for i in 12..22 {
            positions[i] = Coord10::from_meters(50.0 + ((i - 12) as f32 * 1.5), 51.0 + ((i - 12) as f32 % 3.0));
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        // Away team has ball (home is pressing)
        let pos_m = positions[0].to_meters();
        let ctx = make_slow_context(
            0,
            pos_m,
            (52.0, 52.0), // Ball carrier position
            false,
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        let intensity = PressingEvaluator::evaluate_pressing(&ctx);

        // Pressing intensity should be high (close to ball, good support, opponent crowded)
        assert!(
            intensity > 0.5,
            "Pressing intensity should be high when opponents are crowded: {}",
            intensity
        );
    }

    // ========== P3.1: Zone Escape Evaluator Tests ==========

    #[test]
    fn test_zone_escape_no_trigger_when_has_ball() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[5].to_meters();
        let ctx = make_slow_context(
            5,
            pos_m,
            pos_m, // Player has ball
            true,
            true, // Has ball - should not escape
            true,
            &dm,
            &sa,
            &positions,
        );

        let (urgency, _) = ZoneEscapeEvaluator::evaluate_zone_escape(&ctx);
        assert_eq!(urgency, 0.0, "Should not trigger escape when player has ball");
    }

    #[test]
    fn test_zone_escape_no_trigger_when_opponent_has_ball() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[5].to_meters();
        let ctx = make_slow_context(
            5,
            pos_m,
            (80.0, 50.0), // Ball far away with opponent
            false,        // Team doesn't have ball
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        let (urgency, _) = ZoneEscapeEvaluator::evaluate_zone_escape(&ctx);
        assert_eq!(urgency, 0.0, "Should not trigger escape when opponent has ball");
    }

    #[test]
    fn test_zone_escape_triggers_in_crowded_zone() {
        // P3.7: Zone escape now uses 5m radius check (not 10m grid)
        // Create crowded wing zone with players within 5m of player 0
        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Wing zone (y < 20 or x < 20) has max_expected_density = 8 (P3.7 adjusted)
        // Put all 22 players very close together to guarantee threshold exceeded
        let center_x = 15.0_f32;
        let center_y = 15.0_f32;
        for i in 0..22 {
            // All players within 3m of center (well under 5m radius)
            let angle = (i as f32) * 0.3;
            let dist = 1.0 + (i as f32 * 0.1); // 1.0 - 3.1m
            positions[i] = Coord10::from_meters(
                center_x + dist * angle.cos(),
                center_y + dist * angle.sin(),
            );
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        // Verify distance matrix is working
        let nearby = dm.players_within(0, 5.0);
        assert!(
            nearby.len() > 8,
            "Should have >8 players within 5m, got {}",
            nearby.len()
        );

        // Player 0 in crowded wing, team has ball
        let pos_m = positions[0].to_meters();
        let ctx = make_slow_context(
            0,
            pos_m,
            (center_x + 1.0, center_y), // Ball nearby with teammate
            true,                        // Team has ball
            false,                       // Player doesn't have ball
            true,
            &dm,
            &sa,
            &positions,
        );

        let (urgency, direction) = ZoneEscapeEvaluator::evaluate_zone_escape(&ctx);

        // 21 players within 5m in wing zone (max 8) = should trigger
        assert!(
            urgency > 1.0,
            "Should trigger escape in crowded zone: urgency = {}, nearby = {}",
            urgency,
            nearby.len()
        );
        assert!(
            direction != (0.0, 0.0),
            "Should have escape direction: {:?}",
            direction
        );
    }

    #[test]
    fn test_zone_escape_no_trigger_in_sparse_zone() {
        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Put only 3 players in wing zone (max 5 expected)
        positions[0] = Coord10::from_meters(50.0, 10.0); // Wing zone
        positions[1] = Coord10::from_meters(55.0, 10.0);
        positions[2] = Coord10::from_meters(60.0, 10.0);
        // Rest spread out
        for i in 3..11 {
            positions[i] = Coord10::from_meters(20.0 + (i as f32 * 8.0), 40.0);
        }
        for i in 11..22 {
            positions[i] = Coord10::from_meters(80.0, 30.0 + (i as f32 * 3.0));
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[0].to_meters();
        let ctx = make_slow_context(
            0,
            pos_m,
            (55.0, 10.0),
            true,
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        let (urgency, _) = ZoneEscapeEvaluator::evaluate_zone_escape(&ctx);

        // 3 players in wing zone (max 5) = under threshold
        assert!(
            urgency == 0.0,
            "Should not trigger escape in sparse zone: urgency = {}",
            urgency
        );
    }

    #[test]
    fn test_should_create_space() {
        // P3.7: Zone escape uses 5m radius check
        // Create crowded zone scenario with all players within 5m
        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Put all 22 players very close together in wing zone
        let center_x = 15.0_f32;
        let center_y = 15.0_f32;
        for i in 0..22 {
            let angle = (i as f32) * 0.3;
            let dist = 1.0 + (i as f32 * 0.1); // 1.0 - 3.1m
            positions[i] = Coord10::from_meters(
                center_x + dist * angle.cos(),
                center_y + dist * angle.sin(),
            );
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[0].to_meters();
        let ctx = make_slow_context(
            0,
            pos_m,
            (center_x + 1.0, center_y),
            true,
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        assert!(
            ZoneEscapeEvaluator::should_create_space(&ctx),
            "Should trigger CreatingSpace in crowded zone"
        );
    }

    // ========== Additional Coverage Tests ==========

    #[test]
    fn test_penetration_positive_case() {
        // Create scenario where penetration should trigger:
        // - Team has ball, player doesn't
        // - Good space behind defense
        // - Player near offside line but not past it
        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Home team (attacking toward y=105)
        positions[0] = Coord10::from_meters(field::CENTER_X, 70.0); // Forward ready to run
        positions[1] = Coord10::from_meters(40.0, 50.0); // Ball holder
        for i in 2..11 {
            positions[i] = Coord10::from_meters(30.0 + (i as f32 * 5.0), 40.0);
        }

        // Away team (defending): high defensive line at y=75
        for i in 11..22 {
            positions[i] = Coord10::from_meters(30.0 + ((i - 11) as f32 * 5.0), 75.0);
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        // Player 0 ready to penetrate, teammate (1) has ball
        let pos_m = positions[0].to_meters();
        let ctx = make_slow_context(
            0,
            pos_m,
            (40.0, 50.0), // Ball with teammate
            true,         // Team has ball
            false,        // Player doesn't have ball
            true,
            &dm,
            &sa,
            &positions,
        );

        let (_should, score) = PenetrationEvaluator::should_penetrate(&ctx);

        // Should have reasonable penetration score (space behind, visible to ball holder)
        assert!(
            score > 0.3,
            "Should have reasonable penetration score: {}",
            score
        );
        // Note: may or may not trigger depending on exact conditions
    }

    #[test]
    fn test_pressing_high_intensity() {
        // Create scenario where pressing intensity should be high:
        // - Opponent has ball
        // - Player close to ball
        // - Good teammate support
        // - In attacking third
        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Home team pressing high
        positions[0] = Coord10::from_meters(50.0, 75.0); // Presser near ball
        positions[1] = Coord10::from_meters(45.0, 73.0); // Support 1
        positions[2] = Coord10::from_meters(55.0, 73.0); // Support 2
        for i in 3..11 {
            positions[i] = Coord10::from_meters(40.0 + (i as f32 * 3.0), 60.0);
        }

        // Away team with ball
        positions[11] = Coord10::from_meters(52.0, 77.0); // Ball carrier
        for i in 12..22 {
            positions[i] = Coord10::from_meters(50.0 + ((i - 12) as f32 * 2.0), 85.0);
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[0].to_meters();
        let ctx = make_slow_context(
            0,
            pos_m,
            (52.0, 77.0), // Ball with opponent
            false,        // Opponent has ball
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        let intensity = PressingEvaluator::evaluate_pressing(&ctx);

        // Should have high pressing intensity
        assert!(
            intensity > 0.6,
            "Should have high pressing intensity: {}",
            intensity
        );
    }

    #[test]
    fn test_slow_context_accessors() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let sa = SpaceAnalysis::new();

        let ctx = make_slow_context(
            5,
            (field::CENTER_X, 50.0),
            (50.0, 45.0),
            true,
            false,
            true,
            &dm,
            &sa,
            &positions,
        );

        // Test accessors
        assert_eq!(ctx.player_idx(), 5);
        assert!(ctx.is_home());
        assert_eq!(ctx.player_position(), (field::CENTER_X, 50.0));
        assert_eq!(ctx.ball_position(), (50.0, 45.0));
        assert!(ctx.team_has_ball());
        assert!(!ctx.player_has_ball());

        // Test team helpers
        let teammates: Vec<_> = ctx.teammate_indices().collect();
        assert_eq!(teammates.len(), 10); // 11 - 1 (self)
        assert!(!teammates.contains(&5)); // Excludes self

        let opponents: Vec<_> = ctx.opponent_indices().collect();
        assert_eq!(opponents.len(), 11);
        assert!(opponents.iter().all(|&i| i >= 11));
    }

    #[test]
    fn test_slow_context_away_team() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let sa = SpaceAnalysis::new();

        let ctx = make_slow_context(
            15,
            (field::CENTER_X, 50.0),
            (50.0, 45.0),
            true,
            false,
            false, // Away team
            &dm,
            &sa,
            &positions,
        );

        assert!(!ctx.is_home());

        let teammates: Vec<_> = ctx.teammate_indices().collect();
        assert_eq!(teammates.len(), 10);
        assert!(teammates.iter().all(|&i| i >= 11 && i != 15));

        let opponents: Vec<_> = ctx.opponent_indices().collect();
        assert_eq!(opponents.len(), 11);
        assert!(opponents.iter().all(|&i| i < 11));
    }

    #[test]
    fn test_slow_evaluation_result_default() {
        let result = SlowEvaluationResult::default();

        assert!(result.suggested_state.is_none());
        assert!(result.best_pass_target.is_none());
        assert!(!result.should_penetrate);
        assert_eq!(result.penetration_score, 0.0);
        assert_eq!(result.pressing_intensity, 0.0);
        assert_eq!(result.zone_escape_urgency, 0.0);
        assert_eq!(result.zone_escape_direction, (0.0, 0.0));
    }

    // ========== Role Transition Matrix Tests ==========

    #[test]
    fn test_role_matrix_not_applied_when_missing() {
        let positions = make_test_positions();
        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        let pos_m = positions[5].to_meters();
        let ctx = make_slow_context(
            5,
            pos_m,
            pos_m,
            true,
            true,
            true,
            &dm,
            &sa,
            &positions,
        );

        // Without role matrix, has_role_matrix should return false
        assert!(!ctx.has_role_matrix());
    }

    #[test]
    fn test_role_matrix_cm_to_st_preferred() {
        // Create positions where CM (player 5) is passing
        // Target 9 (forward) should be preferred over target 2 (CB) with role matrix
        let mut positions: [Coord10; 22] = std::array::from_fn(|_| Coord10::CENTER);

        // Set up a 4-4-2 formation scenario
        // Player 5 = CM at center
        positions[5] = Coord10::from_meters(field::CENTER_X, 45.0);
        // Player 2 = CB behind
        positions[2] = Coord10::from_meters(25.0, 35.0);
        // Player 9 = Forward ahead
        positions[9] = Coord10::from_meters(80.0, 45.0);

        // Spread out other players
        for i in 0..22 {
            if i != 2 && i != 5 && i != 9 {
                positions[i] = Coord10::from_meters(
                    20.0 + (i as f32 * 4.0),
                    20.0 + ((i % 5) as f32 * 10.0),
                );
            }
        }

        let mut dm = PlayerDistanceMatrix::new();
        dm.update(&positions, 0);
        let mut sa = SpaceAnalysis::new();
        sa.update(&positions, 0);

        // Create role mapping for 4-4-2
        let player_roles: [PositionKey; 22] = [
            // Home team (0-10)
            PositionKey::GK,
            PositionKey::LB,
            PositionKey::LCB,
            PositionKey::RCB,
            PositionKey::RB,
            PositionKey::LCM, // Player 5 = CM
            PositionKey::RCM,
            PositionKey::LM,
            PositionKey::RM,
            PositionKey::LF, // Player 9 = Forward
            PositionKey::RF,
            // Away team (11-21)
            PositionKey::GK,
            PositionKey::LB,
            PositionKey::LCB,
            PositionKey::RCB,
            PositionKey::RB,
            PositionKey::LCM,
            PositionKey::RCM,
            PositionKey::LM,
            PositionKey::RM,
            PositionKey::LF,
            PositionKey::RF,
        ];

        let role_matrix = RoleTransitionMatrix::new_442_balanced();

        let pos_m = positions[5].to_meters();
        let ctx = make_slow_context_with_roles(
            5,
            pos_m,
            pos_m,
            true,
            true,
            true,
            &dm,
            &sa,
            &positions,
            &role_matrix,
            &player_roles,
        );

        // Verify role matrix is active
        assert!(ctx.has_role_matrix());

        // Evaluate both targets
        let cb_target = PassTargetEvaluator::evaluate_single(&ctx, 2);
        let fw_target = PassTargetEvaluator::evaluate_single(&ctx, 9);

        // CM→LF should have higher role weight than CM→LCB
        // (LCM→LF = 1.3, LCM→LCB = 0.9 in 4-4-2 balanced)
        // Note: The final score also depends on other factors (distance, progression, etc.)
        // but the role weight should give a boost to the forward pass
        let cm_to_lf_weight = role_matrix.get_weight(PositionKey::LCM, PositionKey::LF);
        let cm_to_cb_weight = role_matrix.get_weight(PositionKey::LCM, PositionKey::LCB);

        assert!(
            cm_to_lf_weight > cm_to_cb_weight,
            "CM→LF should have higher weight than CM→CB: {} vs {}",
            cm_to_lf_weight,
            cm_to_cb_weight
        );

        // Log the scores for debugging (forward should generally score higher due to progression + role)
        eprintln!(
            "CB target score: {}, FW target score: {}",
            cb_target.score, fw_target.score
        );
    }

    #[test]
    fn test_role_weight_applies_additive() {
        let base_score = 0.5;

        // High weight (1.5) should add +0.15 (ROLE_INFLUENCE = 0.3)
        let high_score = super::apply_role_weight_to_score(base_score, 1.5);
        assert!(
            (high_score - 0.65).abs() < 0.01,
            "High weight should add ~0.15: {}",
            high_score
        );

        // Low weight (0.5) should subtract -0.15
        let low_score = super::apply_role_weight_to_score(base_score, 0.5);
        assert!(
            (low_score - 0.35).abs() < 0.01,
            "Low weight should subtract ~0.15: {}",
            low_score
        );
    }

    #[test]
    fn test_player_attributes_affect_role_weight() {
        use crate::models::player::PlayerAttributes;
        use super::get_adjusted_role_weight;

        // Test: High passing skill should boost long pass weight
        let high_passing = PlayerAttributes {
            passing: 90, // High passing
            vision: 90,  // High vision
            ..Default::default()
        };

        let low_passing = PlayerAttributes {
            passing: 30, // Low passing
            vision: 30,  // Low vision
            ..Default::default()
        };

        let high_off_ball = PlayerAttributes {
            off_the_ball: 90,
            ..Default::default()
        };

        // CB → ST is a long pass (2+ lines skipped)
        let base_weight = 1.0;

        // High passing holder should get boosted weight for long pass
        let high_weight = get_adjusted_role_weight(
            base_weight,
            Some(&high_passing),
            None,
            PositionKey::LCB,
            PositionKey::LF,
        );

        // Low passing holder should get reduced weight for long pass
        let low_weight = get_adjusted_role_weight(
            base_weight,
            Some(&low_passing),
            None,
            PositionKey::LCB,
            PositionKey::LF,
        );

        assert!(
            high_weight > low_weight,
            "High passing should boost long ball weight: {} vs {}",
            high_weight,
            low_weight
        );

        // Target with high off_the_ball should boost weight
        let with_otb = get_adjusted_role_weight(
            base_weight,
            None,
            Some(&high_off_ball),
            PositionKey::LCM,
            PositionKey::LF,
        );

        let without_otb = get_adjusted_role_weight(
            base_weight,
            None,
            None,
            PositionKey::LCM,
            PositionKey::LF,
        );

        assert!(
            with_otb > without_otb,
            "High off_the_ball target should boost weight: {} vs {}",
            with_otb,
            without_otb
        );
    }
}
