//! Deck modifiers for match simulation
//!
//! Provides scalar modifiers from the Deck system (cards) that affect
//! the quality, probability, and physics of match events.

use serde::{Deserialize, Serialize};

/// Impact of active deck/cards on the match engine.
/// These values are typically small scalars (e.g. 0.05 = 5% boost).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct DeckImpact {
    /// Bonus to tackle success probability (e.g., 0.1 for +10%)
    pub tackle_success_bonus: f32,
    
    /// Reducer for stamina drain (e.g., 0.1 for -10% drain)
    pub stamina_drain_reduction: f32,
    
    /// Bonus to through-ball success/vision (e.g., 5.0m extension to vision)
    pub vision_bonus_meters: f32,
    
    /// Bonus to shooting accuracy (e.g., 0.1 for +10% on target)
    pub shooting_accuracy_bonus: f32,
    
    /// Reducer for decision time (e.g. 50.0ms faster decisions)
    pub decision_speed_bonus_ms: f32,

    /// Bonus to marking tightness (e.g. 0.2 for stickier marking)
    pub marking_stickiness_bonus: f32,
}

impl DeckImpact {
    pub fn new() -> Self {
        Self::default()
    }

    /// Combine multiple card effects into a single impact
    pub fn combine(effects: &[DeckImpact]) -> Self {
        let mut total = Self::default();
        for effect in effects {
            total.tackle_success_bonus += effect.tackle_success_bonus;
            total.stamina_drain_reduction += effect.stamina_drain_reduction;
            total.vision_bonus_meters += effect.vision_bonus_meters;
            total.shooting_accuracy_bonus += effect.shooting_accuracy_bonus;
            total.decision_speed_bonus_ms += effect.decision_speed_bonus_ms;
            total.marking_stickiness_bonus += effect.marking_stickiness_bonus;
        }
        total
    }
}
