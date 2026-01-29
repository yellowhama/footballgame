//! Player Selection System
//!
//! This module contains player selection logic for MatchEngine:
//! - Shooter selection (weighted by position)
//! - Assister selection (weighted by position, excludes scorer)
//! - Random player selection
//! - Pass target selection
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::MatchEngine;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // Player Selection System
    // ===========================================

    /// Select shooter from home team (weighted by position)
    /// C6: Returns track_id (0-10) instead of name
    pub(crate) fn select_shooter_home(&mut self) -> usize {
        let weights: Vec<f32> = (0..11)
            .map(|track_id| {
                if !self.setup.is_active(track_id) {
                    return 0.0;
                }
                let p = self.get_match_player(track_id);
                if p.position.is_forward() {
                    3.0
                } else if p.position.is_midfielder() {
                    2.0
                } else if p.position.is_defender() {
                    0.5
                } else {
                    0.1 // Goalkeeper
                }
            })
            .collect();

        let total: f32 = weights.iter().sum();
        let mut random = self.rng.gen::<f32>() * total;

        for (i, weight) in weights.iter().enumerate() {
            random -= weight;
            if random <= 0.0 {
                return i; // C6: Return track_id (0-10 for home)
            }
        }

        10 // C6: Return last pitch slot
    }

    /// Select shooter from away team (weighted by position)
    /// C6: Returns track_id (11-21) instead of name
    pub(crate) fn select_shooter_away(&mut self) -> usize {
        let weights: Vec<f32> = (11..22)
            .map(|track_id| {
                if !self.setup.is_active(track_id) {
                    return 0.0;
                }
                let p = self.get_match_player(track_id);
                if p.position.is_forward() {
                    3.0
                } else if p.position.is_midfielder() {
                    2.0
                } else if p.position.is_defender() {
                    0.5
                } else {
                    0.1 // Goalkeeper
                }
            })
            .collect();

        let total: f32 = weights.iter().sum();
        let mut random = self.rng.gen::<f32>() * total;

        for (i, weight) in weights.iter().enumerate() {
            random -= weight;
            if random <= 0.0 {
                return 11 + i; // C6: Return track_id (11-21 for away)
            }
        }

        21 // C6: Return last pitch slot
    }

    /// Select assister from home team (weighted by position, excludes scorer)
    /// C6: Returns track_id (0-10) instead of name, accepts scorer_idx instead of scorer name
    pub(crate) fn select_assister_home(&mut self, scorer_idx: usize) -> usize {
        let weights: Vec<f32> = (0..11)
            .map(|track_id| {
                if !self.setup.is_active(track_id) {
                    return 0.0;
                }
                let p = self.get_match_player(track_id);
                if track_id == scorer_idx {
                    0.0 // Can't assist yourself
                } else if p.position.is_midfielder() {
                    3.0
                } else if p.position.is_defender() || p.position.is_forward() {
                    1.5
                } else {
                    0.1 // Goalkeeper
                }
            })
            .collect();

        let total: f32 = weights.iter().sum();
        let mut random = self.rng.gen::<f32>() * total;

        for (i, weight) in weights.iter().enumerate() {
            random -= weight;
            if random <= 0.0 {
                return i; // C6: Return track_id (0-10 for home)
            }
        }

        10 // C6: Return last pitch slot
    }

    /// Select assister from away team (weighted by position, excludes scorer)
    /// C6: Returns track_id (11-21) instead of name, accepts scorer_idx instead of scorer name
    pub(crate) fn select_assister_away(&mut self, scorer_idx: usize) -> usize {
        let weights: Vec<f32> = (11..22)
            .map(|track_id| {
                if !self.setup.is_active(track_id) {
                    return 0.0;
                }
                let p = self.get_match_player(track_id);
                if track_id == scorer_idx {
                    0.0 // Can't assist yourself
                } else if p.position.is_midfielder() {
                    3.0
                } else if p.position.is_defender() || p.position.is_forward() {
                    1.5
                } else {
                    0.1 // Goalkeeper
                }
            })
            .collect();

        let total: f32 = weights.iter().sum();
        let mut random = self.rng.gen::<f32>() * total;

        for (i, weight) in weights.iter().enumerate() {
            random -= weight;
            if random <= 0.0 {
                return 11 + i; // C6: Return track_id (11-21 for away)
            }
        }

        21 // C6: Return last pitch slot
    }

    /// Select random player from home team
    /// C6: Returns track_id (0-10) instead of name
    pub(crate) fn select_random_player_home(&mut self) -> usize {
        let active_track_ids: Vec<usize> = (0..11).filter(|&i| self.setup.is_active(i)).collect();
        if active_track_ids.is_empty() {
            return 0;
        }
        active_track_ids[self.rng.gen_range(0..active_track_ids.len())] // C6: Return track_id (0-10 for home)
    }

    /// Select random player from away team
    /// C6: Returns track_id (11-21) instead of name
    pub(crate) fn select_random_player_away(&mut self) -> usize {
        let active_track_ids: Vec<usize> = (11..22).filter(|&i| self.setup.is_active(i)).collect();
        if active_track_ids.is_empty() {
            return 11;
        }
        active_track_ids[self.rng.gen_range(0..active_track_ids.len())] // C6: Return track_id (11-21 for away)
    }

    /// Select pass target from valid targets
    pub(crate) fn select_pass_target(
        &mut self,
        from_idx: usize,
        is_home: bool,
        is_long: bool,
    ) -> Option<usize> {
        let valid_targets = self.find_valid_pass_targets(from_idx, is_home);
        if valid_targets.is_empty() {
            return None;
        }

        if is_long {
            let mut forward_targets: Vec<usize> = valid_targets
                .iter()
                .copied()
                .filter(|&i| {
                    let slot = if is_home { i } else { i - 11 };
                    slot >= 7
                })
                .collect();
            if forward_targets.is_empty() {
                forward_targets = valid_targets;
            }
            if forward_targets.is_empty() {
                None
            } else {
                Some(forward_targets[self.rng.gen_range(0..forward_targets.len())])
            }
        } else {
            Some(valid_targets[self.rng.gen_range(0..valid_targets.len())])
        }
    }

    /// Select shooter by position (improved version with shooting probability)
    /// C6: Returns track_id (0-21) instead of name
    pub(crate) fn select_shooter_by_position(&mut self, is_home: bool) -> usize {
        let track_ids: std::ops::Range<usize> = if is_home { 0..11 } else { 11..22 };
        let base_idx = if is_home { 0 } else { 11 };

        // Calculate shooting probability for each player
        let weights: Vec<f32> = track_ids
            .clone()
            .map(|idx| {
                if !self.setup.is_active(idx) {
                    return 0.0;
                }
                let shoot_prob = self.calculate_shooting_probability(idx);
                let involvement = self.calculate_involvement_weight(idx);

                // Position weight
                let p = self.get_match_player(idx);
                let pos_weight = if p.position.is_forward() {
                    3.0
                } else if p.position.is_midfielder() {
                    1.5
                } else if p.position.is_defender() {
                    0.3
                } else {
                    0.05 // GK
                };

                shoot_prob * involvement * pos_weight
            })
            .collect();

        let total: f32 = weights.iter().sum();
        if total == 0.0 {
            return base_idx + 10; // C6: Return last pitch slot
        }

        let mut random = self.rng.gen::<f32>() * total;

        for (i, weight) in weights.iter().enumerate() {
            random -= weight;
            if random <= 0.0 {
                return base_idx + i; // C6: Return track_id
            }
        }

        base_idx + 10 // C6: Return last pitch slot
    }
}
