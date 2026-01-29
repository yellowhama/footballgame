//! Interception System
//!
//! This module contains interception-related logic for MatchEngine:
//! - Distance to pass path calculation
//! - Intercept chance calculation with Gold Traits
//! - Pass interception attempt logic
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use crate::engine::physics_constants::skills;
use crate::engine::types::coord10::Coord10;
use crate::models::trait_system::TraitId;
use crate::models::TeamSide;
use crate::player::skill_system::SkillCalculator;
use rand::Rng; // Used by actor_rng in try_intercept_pass

impl MatchEngine {
    // ===========================================
    // Interception System
    // ===========================================

    /// Calculate shortest distance from point to line segment (all Coord10)
    ///
    /// FIX_2601 Phase 3.6: All inputs are Coord10
    pub(crate) fn distance_to_line(
        &self,
        point: Coord10,
        line_start: Coord10,
        line_end: Coord10,
    ) -> f32 {
        // Use Coord10 directly (0.1m units)
        let (px, py) = (point.x as f32, point.y as f32);
        let (x1, y1) = (line_start.x as f32, line_start.y as f32);
        let (x2, y2) = (line_end.x as f32, line_end.y as f32);

        // Line segment length squared
        let line_length_sq = (x2 - x1).powi(2) + (y2 - y1).powi(2);

        if line_length_sq < 0.0001 {
            // Line segment is a point
            return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
        }

        // Projection ratio onto line segment (0.0 ~ 1.0)
        let t = (((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / line_length_sq).clamp(0.0, 1.0);

        // Closest point on line segment
        let closest_x = x1 + t * (x2 - x1);
        let closest_y = y1 + t * (y2 - y1);

        // Calculate distance in Coord10 units (0.1m)
        ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt()
    }

    /// Calculate interception probability
    pub(crate) fn calculate_intercept_chance(&self, interceptor_idx: usize) -> f32 {
        use crate::models::SpecialSkill;

        let anticipation = skills::normalize(self.get_player_anticipation(interceptor_idx));
        let positioning = skills::normalize(self.get_player_positioning(interceptor_idx));
        let pace = skills::normalize(self.get_player_pace(interceptor_idx));

        // Get pass path
        let (from_pos, to_pos) = match (self.ball.from_position, self.ball.to_position) {
            (Some(from), Some(to)) => (from, to),
            _ => return 0.0, // No path info, interception not possible
        };

        // Player position (already Coord10)
        let player_pos = self.player_positions[interceptor_idx];

        // Distance to pass path (Coord10 units: 0.1m)
        let distance_to_path = self.distance_to_line(player_pos, from_pos, to_pos);

        // Distance threshold: 30 = 3m (Coord10: 0.1m units)
        let max_intercept_range = 30.0;
        if distance_to_path > max_intercept_range {
            return 0.0;
        }

        let distance_factor = 1.0 - (distance_to_path / max_intercept_range);

        // Calculate intercept probability
        let base_chance = anticipation * 0.4 + positioning * 0.3 + pace * 0.3;
        let mut final_chance = (base_chance * distance_factor).clamp(0.0, 0.7); // max 70%

        // A13: Poacher - If pass target has Poacher skill, marking evasion +35%
        if let Some(target_idx) = self.ball.pending_owner {
            let has_poacher = if let Some(target) = self.get_player(target_idx) {
                target.has_skill(SpecialSkill::Poacher)
            } else {
                false
            };

            if has_poacher {
                // Marking evasion +35% = interception chance reduced by 35%
                final_chance *= 0.65;
            }

            // ============================================
            // Gold Trait Special Effects
            // ============================================

            // Gold Poacher (Infiltrator): Completely nullifies defender reaction - interception not possible
            if self.player_has_gold_trait(target_idx, TraitId::Poacher) {
                final_chance = 0.0;
            }
        }

        // Gold Reader (Reader): Pass path auto-interception chance doubled
        if self.player_has_gold_trait(interceptor_idx, TraitId::Reader) {
            final_chance = (final_chance + 0.25).min(0.80);
        }

        // Gold Architect: Long pass interception not possible
        // Long pass is determined by distance (30m+ in meters)
        let (to_x, to_y) = to_pos.to_meters();
        let (from_x, from_y) = from_pos.to_meters();
        let pass_distance = ((to_x - from_x).powi(2) + (to_y - from_y).powi(2)).sqrt();
        if pass_distance > 30.0 {
            // Long pass - check passer
            if let Some(receiver_idx) = self.ball.pending_owner {
                let _passer_is_home = TeamSide::is_home(receiver_idx);
                // Check if anyone on passer's team has Architect
                let passer_range = TeamSide::teammate_range(receiver_idx);
                for passer_idx in passer_range {
                    if self.player_has_gold_trait(passer_idx, TraitId::Architect) {
                        final_chance = 0.0;
                        break;
                    }
                }
            }
        }

        final_chance
    }

    /// Attempt to intercept pass during ball flight
    /// FIX_2601/0110: 2-phase update 패턴 적용 (index order bias 제거)
    pub(crate) fn try_intercept_pass(&mut self) -> Option<usize> {
        if !self.ball.is_in_flight {
            return None;
        }

        // Determine current ownership (based on pending_owner)
        let attacking_team_is_home = if let Some(pending_owner) = self.ball.pending_owner {
            TeamSide::is_home(pending_owner)
        } else {
            return None; // No ownership means interception not applicable
        };

        // Determine defending range
        let defending_range = if attacking_team_is_home {
            TeamSide::opponent_range(0) // Away team (11..22)
        } else {
            TeamSide::teammate_range(0) // Home team (0..11)
        };

        // ========== Phase 1: Intent Collection ==========
        // 모든 수비수의 인터셉트 확률 수집
        let mut intercept_candidates: Vec<(usize, f32)> = Vec::new();
        for defender_idx in defending_range {
            let intercept_chance = self.calculate_intercept_chance(defender_idx);
            if intercept_chance > 0.0 {
                intercept_candidates.push((defender_idx, intercept_chance));
            }
        }

        // ========== Phase 2: Batch Resolution ==========
        // FIX_2601/0118: Actor-based RNG for order-independent intercept resolution
        // 모든 수비수가 동시에 인터셉트 시도 (순서 독립적)
        let mut successful_interceptors: Vec<usize> = Vec::new();
        for (defender_idx, intercept_chance) in &intercept_candidates {
            // Actor-based seed: original_seed ^ (tick << 16) ^ (player_idx << 32) ^ (stage_marker << 48)
            let actor_seed = self.original_seed
                ^ (self.current_tick << 16)
                ^ ((*defender_idx as u64) << 32)
                ^ (0x1C3 << 48); // IC3 = intercept
            use rand::SeedableRng;
            let mut actor_rng = rand_chacha::ChaCha8Rng::seed_from_u64(actor_seed);
            let roll: f32 = actor_rng.gen();
            if roll < *intercept_chance {
                successful_interceptors.push(*defender_idx);
            }
        }

        // ========== Phase 3: Batch Commit ==========
        // FIX_2601/0118: Actor-based RNG for deterministic winner selection
        // 여러 명이 성공하면 결정론적으로 한 명 선택
        if successful_interceptors.is_empty() {
            None
        } else {
            let commit_seed = self.original_seed
                ^ (self.current_tick << 16)
                ^ (0x1CC << 48); // ICC = intercept commit
            use rand::SeedableRng;
            let mut commit_rng = rand_chacha::ChaCha8Rng::seed_from_u64(commit_seed);
            let chosen_idx = commit_rng.gen_range(0..successful_interceptors.len());
            Some(successful_interceptors[chosen_idx])
        }
    }
}
