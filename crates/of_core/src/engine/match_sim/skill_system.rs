//! A13 Skill System - Deception and Reaction State Management
//!
//! This module contains skill-related logic for MatchEngine:
//! - Deception actions (feint, skill moves, body feints)
//! - Reaction state management (Frozen, OffBalance, Normal)
//! - Skill bonus calculations
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use crate::engine::actions::{self, DeceptionContext};
use crate::engine::physics_constants::skills;
use crate::engine::types::{PlayerReactionState, ReactionState};
use crate::models::match_setup::MatchPlayer;
use crate::models::skill::{ActionType, SkillContext};
use crate::models::trait_system::{ActionType as TraitActionType, TraitId};
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // A13: Skill System - Deception Actions
    // ===========================================

    /// Resolve deception action using 3-Stat Combo logic
    pub fn resolve_deception_action(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
        action: ActionType,
    ) -> bool {
        // 1. Calculate attacker score
        let att_score = self.calculate_attacker_deception_score(attacker_idx, action);

        // 2. Calculate defender score
        let def_score = self.calculate_defender_reaction_score(defender_idx, action);

        // 3. Roll dice (biased towards attacker)
        let roll = self.rng.gen_range(0.9..1.15);
        let final_att = att_score * roll;

        // 4. Determine result and apply state
        if final_att > (def_score * 1.25) {
            // Critical success: Frozen (1 sec)
            self.apply_reaction_state(defender_idx, ReactionState::Frozen, 60);
            true
        } else if final_att > def_score {
            // Success: OffBalance (0.5 sec)
            self.apply_reaction_state(defender_idx, ReactionState::OffBalance, 30);
            true
        } else {
            // Failure: defender steals ball
            false
        }
    }

    /// Calculate attacker deception score (3-Stat Combo + skill bonuses)
    fn calculate_attacker_deception_score(&self, attacker_idx: usize, action: ActionType) -> f32 {
        let att = self.get_match_player(attacker_idx);
        let attr = &att.attributes;

        // Base 3-Stat Combo score
        let flair = skills::normalize(attr.flair as f32);

        let mut base_score = match action {
            ActionType::DribblingSkill => {
                let technique = skills::normalize(attr.technique as f32);
                let dribbling = skills::normalize(attr.dribbling as f32);
                (flair * 0.4) + (technique * 0.3) + (dribbling * 0.3)
            }

            ActionType::FinishingSkill | ActionType::LongShotSkill => {
                let technique = skills::normalize(attr.technique as f32);
                let composure = skills::normalize(attr.composure as f32);
                (flair * 0.4) + (technique * 0.3) + (composure * 0.3)
            }

            ActionType::ThroughBallSkill | ActionType::ShortPassSkill => {
                let vision = skills::normalize(attr.vision as f32);
                let passing = skills::normalize(attr.passing as f32);
                (flair * 0.4) + (vision * 0.3) + (passing * 0.3)
            }

            ActionType::CrossSkill => {
                let technique = skills::normalize(attr.technique as f32);
                let crossing = skills::normalize(attr.crossing as f32);
                (flair * 0.4) + (technique * 0.3) + (crossing * 0.3)
            }

            ActionType::HeaderSkill => {
                let heading = skills::normalize(attr.heading as f32);
                let strength = skills::normalize(attr.strength as f32);
                (flair * 0.4) + (heading * 0.3) + (strength * 0.3)
            }

            _ => flair,
        };

        // Apply skill bonuses
        base_score = self.apply_skill_bonuses(att, action, base_score);

        base_score
    }

    /// Apply skill bonuses (SkillStrategy pattern)
    /// - Each skill defines its own bonus multiplier (SpecialSkill::get_bonus_multiplier)
    /// - Extension point: add skills in skill.rs
    /// - 2025-12-03: Integrated Trait active bonuses
    fn apply_skill_bonuses(
        &self,
        player: &MatchPlayer,
        action: ActionType,
        base_score: f32,
    ) -> f32 {
        let mut multiplier = 1.0;

        // 1. Legacy SpecialSkill bonuses (backward compat)
        for skill in &player.equipped_skills {
            let skill_multiplier = skill.get_bonus_multiplier(action);
            if skill_multiplier > 1.0 {
                multiplier *= skill_multiplier;
            }
        }

        // 2. NEW: Trait active bonuses
        let trait_action = Self::map_action_to_trait_action(action);
        let trait_multiplier = player.traits.get_action_multiplier(trait_action);
        multiplier *= trait_multiplier;

        base_score * multiplier
    }

    /// Map ActionType to TraitActionType
    /// skill.rs ActionType variants: DribblingSkill, FinishingSkill, LongShotSkill,
    /// ThroughBallSkill, ShortPassSkill, CrossSkill, HeaderSkill, TackleSkill, PenaltySkill, SpeedDuelSkill
    fn map_action_to_trait_action(action: ActionType) -> TraitActionType {
        match action {
            ActionType::FinishingSkill => TraitActionType::Finishing,
            ActionType::LongShotSkill => TraitActionType::LongShot,
            ActionType::ThroughBallSkill => TraitActionType::ThroughBall,
            ActionType::ShortPassSkill => TraitActionType::ShortPass,
            ActionType::CrossSkill => TraitActionType::Cross,
            ActionType::DribblingSkill => TraitActionType::Dribble,
            ActionType::HeaderSkill => TraitActionType::Header,
            ActionType::TackleSkill => TraitActionType::Tackle,
            ActionType::PenaltySkill => TraitActionType::PenaltyKick,
            ActionType::SpeedDuelSkill => TraitActionType::Sprint,
        }
    }

    /// Apply skill bonuses with context (for conditional skills)
    pub(crate) fn apply_skill_bonuses_with_context(
        &self,
        player: &MatchPlayer,
        action: ActionType,
        context: &SkillContext,
        base_score: f32,
    ) -> f32 {
        let mut multiplier = 1.0;

        // Legacy skills
        for skill in &player.equipped_skills {
            if skill.meets_activation_condition(context) {
                let skill_multiplier = skill.get_bonus_multiplier(action);
                if skill_multiplier > 1.0 {
                    multiplier *= skill_multiplier;
                }
            }
        }

        // NEW: Trait active bonuses
        let trait_action = Self::map_action_to_trait_action(action);
        multiplier *= player.traits.get_action_multiplier(trait_action);

        base_score * multiplier
    }

    /// Calculate defender reaction score
    fn calculate_defender_reaction_score(&self, defender_idx: usize, action: ActionType) -> f32 {
        let def = self.get_match_player(defender_idx);
        let attr = &def.attributes;

        let anticipation = skills::normalize(attr.anticipation as f32);
        let concentration = skills::normalize(attr.concentration as f32);

        match action {
            ActionType::DribblingSkill => {
                let balance = skills::normalize(attr.balance as f32);
                (anticipation * 0.4) + (concentration * 0.3) + (balance * 0.3)
            }

            _ => {
                // Composure helps resist feints
                let composure = skills::normalize(attr.composure as f32);
                (anticipation * 0.4) + (concentration * 0.3) + (composure * 0.3)
            }
        }
    }

    // ===========================================
    // A13: Reaction State Management
    // ===========================================

    /// Apply reaction state to player
    pub(crate) fn apply_reaction_state(
        &mut self,
        player_idx: usize,
        state: ReactionState,
        ticks: u32,
    ) {
        self.player_reaction_states[player_idx] =
            PlayerReactionState { state, remaining_ticks: ticks };
    }

    /// Update reaction states (called every frame)
    pub(crate) fn update_reaction_states(&mut self) {
        for reaction in &mut self.player_reaction_states {
            if reaction.remaining_ticks > 0 {
                reaction.remaining_ticks -= 1;

                if reaction.remaining_ticks == 0 {
                    reaction.state = ReactionState::Normal;
                }
            }
        }
    }

    /// Get current reaction state for player
    pub fn get_reaction_state(&self, player_idx: usize) -> ReactionState {
        self.player_reaction_states[player_idx].state
    }

    // ===========================================
    // A13: Execute Deception Action (Pure Function Pattern)
    // ===========================================

    /// Execute deception action using pure function pattern
    /// action_type: "feint", "skill_move", "body_feint"
    pub(crate) fn execute_deception_action(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
        action_type: &str,
        _is_home: bool,
    ) -> bool {
        // ============================================
        // 1. Create DeceptionContext
        // ============================================
        let ctx = DeceptionContext {
            attacker_idx,
            defender_idx,
            action_type: action_type.to_string(),
            // Attacker stats
            flair: self.get_player_flair(attacker_idx),
            technique: self.get_player_technique(attacker_idx),
            dribbling: self.get_player_dribbling(attacker_idx),
            agility: self.get_player_agility(attacker_idx),
            balance: self.get_player_balance(attacker_idx),
            composure: self.get_player_composure(attacker_idx),
            // Defender stats
            anticipation: self.get_player_anticipation(defender_idx),
            concentration: self.get_player_concentration(defender_idx),
            positioning: self.get_player_positioning(defender_idx),
            decisions: self.get_player_decisions(defender_idx),
            defender_agility: self.get_player_agility(defender_idx),
        };

        // ============================================
        // 2. Gold Trait Bonus (Technician)
        // ============================================
        let technician_bonus = if self.player_has_gold_trait(attacker_idx, TraitId::Technician) {
            0.10 // +10% success rate
        } else {
            0.0
        };

        // ============================================
        // 3. Call pure function
        // ============================================
        let roll: f32 = self.rng.gen();
        let adjusted_roll = (roll - technician_bonus).max(0.0);
        let result = actions::resolve_deception(&ctx, adjusted_roll);

        // ============================================
        // 4. Apply result
        // ============================================
        if result.success {
            // Success: keep possession, defender reaction delay
            // (Speedster trait adds extra benefit - handled in dribble)
            true
        } else {
            // Failure: transfer possession to defender
            // FIX_2601/1120: Update ball position to defender's position to prevent teleportation
            self.ball.current_owner = Some(defender_idx);
            self.ball.position = self.player_positions[defender_idx];
            false
        }
    }
}
