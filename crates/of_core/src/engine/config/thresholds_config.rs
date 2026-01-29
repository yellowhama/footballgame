//! Centralized Thresholds Configuration (FIX_2601/0123 Phase 6)
//!
//! This module provides a centralized configuration for all threshold values
//! used across the engine. Instead of hardcoded magic numbers, thresholds
//! can be configured via presets or environment variables.
//!
//! ## Threshold Categories
//!
//! | Category | Description |
//! |----------|-------------|
//! | Rules | Foul detection, card thresholds |
//! | Defense | Stamina, distance thresholds |
//! | Duel | Technique-specific foul rates |
//! | Physics | Movement, ownership thresholds |
//!
//! ## Usage
//!
//! ```rust
//! use of_core::engine::config::ThresholdsConfig;
//!
//! // Default thresholds
//! let config = ThresholdsConfig::default();
//!
//! // Arcade preset (more fouls, more action)
//! let arcade = ThresholdsConfig::arcade();
//!
//! // From environment variable
//! let from_env = ThresholdsConfig::from_env_or_default();
//! ```
//!
//! ## Environment Variables
//!
//! - `OF_THRESHOLD_PROFILE`: Select preset (arcade, simulation, default)

use serde::{Deserialize, Serialize};
use std::env;

/// Centralized thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdsConfig {
    /// Rule system thresholds
    pub rules: RuleThresholds,
    /// Defense intent thresholds
    pub defense: DefenseThresholds,
    /// Duel system thresholds
    pub duel: DuelThresholds,
    /// Physics thresholds
    pub physics: PhysicsThresholds,
}

impl Default for ThresholdsConfig {
    fn default() -> Self {
        Self {
            rules: RuleThresholds::default(),
            defense: DefenseThresholds::default(),
            duel: DuelThresholds::default(),
            physics: PhysicsThresholds::default(),
        }
    }
}

impl ThresholdsConfig {
    /// Arcade preset - more action, more fouls, more cards
    pub fn arcade() -> Self {
        Self {
            rules: RuleThresholds {
                tackle_foul_base_chance: 0.20, // Higher foul chance
                yellow_card_threshold: 0.50,   // Lower threshold = more cards
                red_card_threshold: 0.75,
                ..RuleThresholds::default()
            },
            defense: DefenseThresholds {
                stamina_exhausted_threshold: 0.20, // Earlier exhaustion
                challenge_max_distance_m: 4.0,     // Wider challenge range
                ..DefenseThresholds::default()
            },
            duel: DuelThresholds {
                standing_tackle_foul_rate: 0.12,
                sliding_tackle_foul_rate: 0.30, // Higher sliding foul rate
                shoulder_charge_foul_rate: 0.18,
                ..DuelThresholds::default()
            },
            physics: PhysicsThresholds::default(),
        }
    }

    /// Simulation preset - more realistic, fewer fouls
    pub fn simulation() -> Self {
        Self {
            rules: RuleThresholds {
                tackle_foul_base_chance: 0.12, // Lower foul chance
                yellow_card_threshold: 0.70,   // Higher threshold = fewer cards
                red_card_threshold: 0.90,
                ..RuleThresholds::default()
            },
            defense: DefenseThresholds {
                stamina_exhausted_threshold: 0.35, // Later exhaustion
                challenge_max_distance_m: 2.5,     // Tighter challenge range
                ..DefenseThresholds::default()
            },
            duel: DuelThresholds {
                standing_tackle_foul_rate: 0.08,
                sliding_tackle_foul_rate: 0.20,
                shoulder_charge_foul_rate: 0.12,
                ..DuelThresholds::default()
            },
            physics: PhysicsThresholds::default(),
        }
    }

    /// Load from environment variable OF_THRESHOLD_PROFILE or use default
    pub fn from_env_or_default() -> Self {
        match env::var("OF_THRESHOLD_PROFILE")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "arcade" => Self::arcade(),
            "simulation" => Self::simulation(),
            _ => Self::default(),
        }
    }
}

/// Rule system thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleThresholds {
    /// Base chance of foul on tackle (0.0 - 1.0)
    pub tackle_foul_base_chance: f32,
    /// Threshold for yellow card decision (0.0 - 1.0)
    pub yellow_card_threshold: f32,
    /// Threshold for red card decision (0.0 - 1.0)
    pub red_card_threshold: f32,
    /// Base handball detection probability (0.0 - 1.0)
    pub handball_base_probability: f32,
    /// Offside tolerance in meters (for marginal calls)
    pub offside_tolerance_m: f32,
}

impl Default for RuleThresholds {
    fn default() -> Self {
        Self {
            tackle_foul_base_chance: 0.15,
            yellow_card_threshold: 0.60,
            red_card_threshold: 0.85,
            handball_base_probability: 0.30,
            offside_tolerance_m: 0.1,
        }
    }
}

/// Defense intent thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseThresholds {
    /// Stamina level below which player is considered exhausted (0.0 - 1.0)
    pub stamina_exhausted_threshold: f32,
    /// Maximum distance for Challenge intent (meters)
    pub challenge_max_distance_m: f32,
    /// Maximum distance for Press intent (meters)
    pub press_max_distance_m: f32,
    /// Distance at which Contain becomes default (meters)
    pub contain_distance_m: f32,
}

impl Default for DefenseThresholds {
    fn default() -> Self {
        Self {
            stamina_exhausted_threshold: 0.30,
            challenge_max_distance_m: 3.0,
            press_max_distance_m: 8.0,
            contain_distance_m: 8.0,
        }
    }
}

/// Duel system thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelThresholds {
    /// Standing tackle base foul rate
    pub standing_tackle_foul_rate: f32,
    /// Sliding tackle base foul rate
    pub sliding_tackle_foul_rate: f32,
    /// Shoulder charge base foul rate
    pub shoulder_charge_foul_rate: f32,
    /// Poke away base foul rate
    pub poke_away_foul_rate: f32,
    /// Closing down base foul rate
    pub closing_down_foul_rate: f32,
    /// Intercept attempt base foul rate
    pub intercept_attempt_foul_rate: f32,
    /// Force touchline base foul rate
    pub force_touchline_foul_rate: f32,
    /// Track runner base foul rate
    pub track_runner_foul_rate: f32,
}

impl Default for DuelThresholds {
    fn default() -> Self {
        Self {
            standing_tackle_foul_rate: 0.10,
            sliding_tackle_foul_rate: 0.25,
            shoulder_charge_foul_rate: 0.15,
            poke_away_foul_rate: 0.05,
            closing_down_foul_rate: 0.08,
            intercept_attempt_foul_rate: 0.05,
            force_touchline_foul_rate: 0.10,
            track_runner_foul_rate: 0.06,
        }
    }
}

/// Physics thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsThresholds {
    /// Ball ownership distance threshold (meters)
    pub ownership_threshold_m: f32,
    /// Minimum velocity for ball to be considered moving (m/s)
    pub ball_moving_threshold_ms: f32,
    /// Player collision radius (meters)
    pub player_collision_radius_m: f32,
}

impl Default for PhysicsThresholds {
    fn default() -> Self {
        Self {
            ownership_threshold_m: 5.0,
            ball_moving_threshold_ms: 0.1,
            player_collision_radius_m: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_thresholds() {
        let config = ThresholdsConfig::default();
        assert!((config.rules.tackle_foul_base_chance - 0.15).abs() < 0.001);
        assert!((config.defense.stamina_exhausted_threshold - 0.30).abs() < 0.001);
        assert!((config.duel.sliding_tackle_foul_rate - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_arcade_preset() {
        let config = ThresholdsConfig::arcade();
        // Arcade has higher foul chances
        assert!(config.rules.tackle_foul_base_chance > 0.15);
        // Arcade has lower card thresholds (more cards)
        assert!(config.rules.yellow_card_threshold < 0.60);
        // Arcade has wider challenge range
        assert!(config.defense.challenge_max_distance_m > 3.0);
    }

    #[test]
    fn test_simulation_preset() {
        let config = ThresholdsConfig::simulation();
        // Simulation has lower foul chances
        assert!(config.rules.tackle_foul_base_chance < 0.15);
        // Simulation has higher card thresholds (fewer cards)
        assert!(config.rules.yellow_card_threshold > 0.60);
        // Simulation has tighter challenge range
        assert!(config.defense.challenge_max_distance_m < 3.0);
    }

    #[test]
    fn test_from_env_default() {
        // Without env var, should return default
        let config = ThresholdsConfig::from_env_or_default();
        assert!((config.rules.tackle_foul_base_chance - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_serialization() {
        let config = ThresholdsConfig::default();
        let json = serde_json::to_string(&config).expect("Should serialize");
        let parsed: ThresholdsConfig = serde_json::from_str(&json).expect("Should deserialize");
        assert!((parsed.rules.tackle_foul_base_chance - config.rules.tackle_foul_base_chance).abs() < 0.001);
    }
}
