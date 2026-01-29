//! Player personality attributes system
//!
//! This module contains personality traits that affect player behavior,
//! training response, and story interactions.
//!
//! Based on Power Pro style personality system with OpenFootball compatibility.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// 행동 결정에 영향을 주는 가중치 모음
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionModifiers {
    // 공격
    pub move_to_ball: f32,
    pub attack_goal: f32,
    pub find_space: f32,
    pub support: f32,
    pub hold_offense: f32,
    // 수비
    pub press: f32,
    pub track_back: f32,
    pub mark: f32,
    pub block: f32,
    pub hold_defense: f32,
}

impl Default for DecisionModifiers {
    fn default() -> Self {
        Self {
            move_to_ball: 1.0,
            attack_goal: 1.0,
            find_space: 1.0,
            support: 1.0,
            hold_offense: 1.0,
            press: 1.0,
            track_back: 1.0,
            mark: 1.0,
            block: 1.0,
            hold_defense: 1.0,
        }
    }
}

/// Player personality attributes (8 traits)
///
/// Each attribute ranges from 0-100:
/// - 0-30: Low
/// - 31-70: Average
/// - 71-100: High
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersonAttributes {
    /// 적응력 - Ability to adapt to new situations and environments
    pub adaptability: u8,

    /// 야망 - Drive to succeed and reach higher levels
    pub ambition: u8,

    /// 결단력 - Determination to overcome challenges
    pub determination: u8,

    /// 규율 - Self-discipline and following rules/training
    pub discipline: u8,

    /// 충성도 - Loyalty to club and teammates
    pub loyalty: u8,

    /// 압박 처리 - Ability to handle pressure situations
    pub pressure: u8,

    /// 프로정신 - Professional attitude and work ethic
    pub professionalism: u8,

    /// 성격/기질 - Overall temperament and emotional stability
    pub temperament: u8,
}

impl Default for PersonAttributes {
    /// Create average personality (all attributes at 50)
    fn default() -> Self {
        Self {
            adaptability: 50,
            ambition: 50,
            determination: 50,
            discipline: 50,
            loyalty: 50,
            pressure: 50,
            professionalism: 50,
            temperament: 50,
        }
    }
}

impl PersonAttributes {
    /// Create new PersonAttributes with specified values
    pub fn new(
        adaptability: u8,
        ambition: u8,
        determination: u8,
        discipline: u8,
        loyalty: u8,
        pressure: u8,
        professionalism: u8,
        temperament: u8,
    ) -> Self {
        Self {
            adaptability: adaptability.min(100),
            ambition: ambition.min(100),
            determination: determination.min(100),
            discipline: discipline.min(100),
            loyalty: loyalty.min(100),
            pressure: pressure.min(100),
            professionalism: professionalism.min(100),
            temperament: temperament.min(100),
        }
    }

    /// Generate random personality with seed for deterministic results
    pub fn generate_random(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        Self {
            adaptability: rng.gen_range(20..=80),
            ambition: rng.gen_range(20..=80),
            determination: rng.gen_range(20..=80),
            discipline: rng.gen_range(20..=80),
            loyalty: rng.gen_range(20..=80),
            pressure: rng.gen_range(20..=80),
            professionalism: rng.gen_range(20..=80),
            temperament: rng.gen_range(20..=80),
        }
    }

    /// Generate personality based on archetype
    pub fn generate_archetype(archetype: PersonalityArchetype, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let variation = 15; // ±15 variation from base values

        let base = match archetype {
            PersonalityArchetype::Leader => Self {
                adaptability: 65,
                ambition: 80,
                determination: 80,
                discipline: 70,
                loyalty: 75,
                pressure: 85,
                professionalism: 80,
                temperament: 75,
            },
            PersonalityArchetype::Genius => Self {
                adaptability: 85,
                ambition: 70,
                determination: 60,
                discipline: 50,
                loyalty: 60,
                pressure: 40,
                professionalism: 65,
                temperament: 45,
            },
            PersonalityArchetype::Workhorse => Self {
                adaptability: 55,
                ambition: 60,
                determination: 85,
                discipline: 90,
                loyalty: 80,
                pressure: 70,
                professionalism: 85,
                temperament: 75,
            },
            PersonalityArchetype::Rebel => Self {
                adaptability: 75,
                ambition: 85,
                determination: 70,
                discipline: 35,
                loyalty: 45,
                pressure: 60,
                professionalism: 40,
                temperament: 30,
            },
            PersonalityArchetype::Steady => Self {
                adaptability: 45,
                ambition: 50,
                determination: 65,
                discipline: 75,
                loyalty: 85,
                pressure: 80,
                professionalism: 75,
                temperament: 85,
            },
        };

        // Apply random variation
        Self {
            adaptability: apply_variation(base.adaptability, variation, &mut rng),
            ambition: apply_variation(base.ambition, variation, &mut rng),
            determination: apply_variation(base.determination, variation, &mut rng),
            discipline: apply_variation(base.discipline, variation, &mut rng),
            loyalty: apply_variation(base.loyalty, variation, &mut rng),
            pressure: apply_variation(base.pressure, variation, &mut rng),
            professionalism: apply_variation(base.professionalism, variation, &mut rng),
            temperament: apply_variation(base.temperament, variation, &mut rng),
        }
    }

    /// Convert to OpenFootball PersonAttributes (f32 values, 0.0-20.0 range)
    pub fn to_openfoot_person_attributes(&self) -> OpenFootPersonAttributes {
        OpenFootPersonAttributes {
            adaptability: (self.adaptability as f32) * 0.2, // 0-100 → 0-20
            ambition: (self.ambition as f32) * 0.2,
            controversy: 10.0 - (self.temperament as f32) * 0.1, // Inverse temperament
            loyalty: (self.loyalty as f32) * 0.2,
            pressure: (self.pressure as f32) * 0.2,
            professionalism: (self.professionalism as f32) * 0.2,
            sportsmanship: (self.discipline as f32) * 0.2, // Map discipline to sportsmanship
            temperament: (self.temperament as f32) * 0.2,
        }
    }

    /// Calculate training efficiency multiplier based on personality
    /// Returns value between 0.7-1.3
    pub fn training_efficiency_multiplier(&self) -> f32 {
        let base_efficiency = (self.discipline as f32 * 0.4
            + self.professionalism as f32 * 0.3
            + self.determination as f32 * 0.2
            + self.ambition as f32 * 0.1)
            / 100.0; // 0.0-1.0 range

        // Map to 0.7-1.3 range
        0.7 + (base_efficiency * 0.6)
    }

    /// Calculate injury resistance based on personality
    /// Returns value between 0.0-1.0 (higher = less injury prone)
    pub fn injury_resistance(&self) -> f32 {
        let resistance = (self.discipline as f32 * 0.3
            + self.professionalism as f32 * 0.3
            + self.temperament as f32 * 0.2
            + self.adaptability as f32 * 0.2)
            / 100.0;

        resistance.clamp(0.0, 1.0)
    }

    /// Calculate pressure handling ability for match situations
    /// Returns value between 0.0-1.0
    pub fn pressure_handling(&self) -> f32 {
        let handling = (self.pressure as f32 * 0.5
            + self.temperament as f32 * 0.3
            + self.determination as f32 * 0.2)
            / 100.0;

        handling.clamp(0.0, 1.0)
    }

    /// Get all attribute names for UI/debugging
    pub fn get_all_attribute_names() -> Vec<&'static str> {
        vec![
            "adaptability",
            "ambition",
            "determination",
            "discipline",
            "loyalty",
            "pressure",
            "professionalism",
            "temperament",
        ]
    }

    /// Get attribute value by name
    pub fn get_attribute_value(&self, name: &str) -> Option<u8> {
        match name {
            "adaptability" => Some(self.adaptability),
            "ambition" => Some(self.ambition),
            "determination" => Some(self.determination),
            "discipline" => Some(self.discipline),
            "loyalty" => Some(self.loyalty),
            "pressure" => Some(self.pressure),
            "professionalism" => Some(self.professionalism),
            "temperament" => Some(self.temperament),
            _ => None,
        }
    }

    /// Set attribute value by name (with bounds checking)
    pub fn set_attribute_value(&mut self, name: &str, value: u8) -> bool {
        let clamped_value = value.min(100);
        match name {
            "adaptability" => {
                self.adaptability = clamped_value;
                true
            }
            "ambition" => {
                self.ambition = clamped_value;
                true
            }
            "determination" => {
                self.determination = clamped_value;
                true
            }
            "discipline" => {
                self.discipline = clamped_value;
                true
            }
            "loyalty" => {
                self.loyalty = clamped_value;
                true
            }
            "pressure" => {
                self.pressure = clamped_value;
                true
            }
            "professionalism" => {
                self.professionalism = clamped_value;
                true
            }
            "temperament" => {
                self.temperament = clamped_value;
                true
            }
            _ => false,
        }
    }
}

/// Personality archetypes for generating realistic personalities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PersonalityArchetype {
    /// Natural leader, high pressure handling
    Leader,
    /// Talented but moody, high adaptability but low discipline
    Genius,
    /// Hard worker, high discipline and determination
    Workhorse,
    /// Unpredictable, high ambition but low discipline/loyalty
    Rebel,
    /// Reliable and consistent, high temperament and loyalty
    #[default]
    Steady,
}

impl PersonalityArchetype {
    /// Return behavior modifiers for tactical/decision systems
    pub fn decision_modifiers(&self) -> DecisionModifiers {
        match self {
            PersonalityArchetype::Leader => DecisionModifiers {
                move_to_ball: 1.1,
                attack_goal: 1.05,
                find_space: 1.05,
                support: 1.1,
                hold_offense: 0.95,
                press: 1.05,
                track_back: 1.05,
                mark: 1.05,
                block: 1.05,
                hold_defense: 0.95,
            },
            PersonalityArchetype::Genius => DecisionModifiers {
                move_to_ball: 1.05,
                attack_goal: 1.2,
                find_space: 1.1,
                support: 0.9,
                hold_offense: 0.8,
                press: 0.9,
                track_back: 0.9,
                mark: 0.9,
                block: 0.95,
                hold_defense: 0.9,
            },
            PersonalityArchetype::Workhorse => DecisionModifiers {
                move_to_ball: 1.05,
                attack_goal: 1.0,
                find_space: 1.0,
                support: 1.1,
                hold_offense: 1.05,
                press: 1.1,
                track_back: 1.2,
                mark: 1.05,
                block: 1.05,
                hold_defense: 1.1,
            },
            PersonalityArchetype::Rebel => DecisionModifiers {
                move_to_ball: 1.1,
                attack_goal: 1.2,
                find_space: 1.0,
                support: 0.8,
                hold_offense: 0.7,
                press: 1.0,
                track_back: 0.9,
                mark: 0.9,
                block: 0.95,
                hold_defense: 0.85,
            },
            PersonalityArchetype::Steady => DecisionModifiers::default(),
        }
    }
}

/// OpenFootball compatible PersonAttributes (f32, 0.0-20.0 range)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenFootPersonAttributes {
    pub adaptability: f32,
    pub ambition: f32,
    pub controversy: f32,
    pub loyalty: f32,
    pub pressure: f32,
    pub professionalism: f32,
    pub sportsmanship: f32,
    pub temperament: f32,
}

/// Apply random variation to a base value
fn apply_variation(base: u8, variation: u8, rng: &mut ChaCha8Rng) -> u8 {
    let min_val = base.saturating_sub(variation);
    let max_val = (base + variation).min(100);
    rng.gen_range(min_val..=max_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personality_default() {
        let personality = PersonAttributes::default();
        assert_eq!(personality.adaptability, 50);
        assert_eq!(personality.ambition, 50);
        assert_eq!(personality.determination, 50);
        assert_eq!(personality.discipline, 50);
        assert_eq!(personality.loyalty, 50);
        assert_eq!(personality.pressure, 50);
        assert_eq!(personality.professionalism, 50);
        assert_eq!(personality.temperament, 50);
    }

    #[test]
    fn test_personality_new_clamping() {
        let personality = PersonAttributes::new(150, 200, 80, 60, 40, 20, 90, 75);
        assert_eq!(personality.adaptability, 100); // Clamped to 100
        assert_eq!(personality.ambition, 100); // Clamped to 100
        assert_eq!(personality.determination, 80);
        assert_eq!(personality.loyalty, 40);
    }

    #[test]
    fn test_random_generation_deterministic() {
        let p1 = PersonAttributes::generate_random(12345);
        let p2 = PersonAttributes::generate_random(12345);
        assert_eq!(p1, p2); // Same seed = same result
    }

    #[test]
    fn test_archetype_generation() {
        let leader = PersonAttributes::generate_archetype(PersonalityArchetype::Leader, 123);
        let genius = PersonAttributes::generate_archetype(PersonalityArchetype::Genius, 123);

        // Leaders should have higher pressure handling than geniuses generally
        assert!(leader.pressure >= genius.pressure - 20); // Allow some variation
        assert!(leader.determination >= genius.determination - 20);
    }

    #[test]
    fn test_training_efficiency_multiplier() {
        let high_discipline = PersonAttributes::new(50, 50, 50, 90, 50, 50, 90, 50);
        let low_discipline = PersonAttributes::new(50, 50, 50, 30, 50, 50, 30, 50);

        assert!(
            high_discipline.training_efficiency_multiplier()
                > low_discipline.training_efficiency_multiplier()
        );
        assert!(high_discipline.training_efficiency_multiplier() >= 0.7);
        assert!(high_discipline.training_efficiency_multiplier() <= 1.3);
    }

    #[test]
    fn test_openfoot_conversion() {
        let personality = PersonAttributes::new(80, 60, 70, 90, 50, 40, 85, 75);
        let openfoot = personality.to_openfoot_person_attributes();

        assert_eq!(openfoot.adaptability, 16.0); // 80 * 0.2
        assert_eq!(openfoot.ambition, 12.0); // 60 * 0.2
        assert_eq!(openfoot.professionalism, 17.0); // 85 * 0.2
    }

    #[test]
    fn test_attribute_getter_setter() {
        let mut personality = PersonAttributes::default();

        assert_eq!(personality.get_attribute_value("adaptability"), Some(50));
        assert_eq!(personality.get_attribute_value("invalid"), None);

        assert!(personality.set_attribute_value("ambition", 75));
        assert_eq!(personality.ambition, 75);
        assert!(!personality.set_attribute_value("invalid", 50));
    }
}
