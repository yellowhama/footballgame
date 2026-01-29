//! Position-specific weighting system for CA calculations
//!
//! Implements detailed position weights according to T016 specification:
//! - FW: shooting 3x, pace 2x, technical 2x, power 2x
//! - MF: technical 3x, passing 3x, others 2x
//! - DF: defending 3x, power 3x, others 2x
//! - GK: special calculation

use crate::models::player::Position;
use serde::{Deserialize, Serialize};

/// Position-specific attribute weights for CA calculation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionWeights {
    pub pace_weight: f32,
    pub power_weight: f32,
    pub technical_weight: f32,
    pub shooting_weight: f32,
    pub passing_weight: f32,
    pub defending_weight: f32,
}

impl PositionWeights {
    /// Get position weights for a specific position
    pub fn get_for_position(position: Position) -> Self {
        match position {
            Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => {
                // Forwards: Shooting 3x, Pace 2x, Technical 2x, Power 2x
                Self {
                    pace_weight: 2.0,
                    power_weight: 2.0,
                    technical_weight: 2.0,
                    shooting_weight: 3.0, // CRITICAL for forwards
                    passing_weight: 1.0,
                    defending_weight: 0.5, // Minimal importance
                }
            }
            Position::MF
            | Position::CM
            | Position::CAM
            | Position::CDM
            | Position::LM
            | Position::RM => {
                // Midfielders: Technical 3x, Passing 3x, others 2x
                Self {
                    pace_weight: 2.0,
                    power_weight: 2.0,
                    technical_weight: 3.0, // CRITICAL for midfielders
                    shooting_weight: 2.0,
                    passing_weight: 3.0, // CRITICAL for distribution
                    defending_weight: 2.0,
                }
            }
            Position::DF
            | Position::CB
            | Position::LB
            | Position::RB
            | Position::LWB
            | Position::RWB => {
                // Defenders: Defending 3x, Power 3x, others 2x
                Self {
                    pace_weight: 2.0,
                    power_weight: 3.0, // CRITICAL for physical duels
                    technical_weight: 1.0,
                    shooting_weight: 0.5, // Minimal importance
                    passing_weight: 2.0,
                    defending_weight: 3.0, // CRITICAL for defenders
                }
            }
            Position::GK => {
                // Goalkeepers: special calculation
                Self::goalkeeper_weights()
            }
        }
    }

    /// Special goalkeeper weighting
    pub fn goalkeeper_weights() -> Self {
        Self {
            pace_weight: 0.5,      // Not critical for GK
            power_weight: 2.0,     // Important for commanding area, shot stopping
            technical_weight: 0.5, // Limited importance
            shooting_weight: 0.0,  // Completely irrelevant for GK
            passing_weight: 1.5,   // Distribution is somewhat important
            defending_weight: 3.0, // Shot stopping, commanding area, positioning
        }
    }

    /// Check if this is goalkeeper weighting
    pub fn is_goalkeeper(&self) -> bool {
        self.shooting_weight == 0.0
    }

    /// Get total weight sum for normalization
    pub fn total_weight(&self) -> f32 {
        self.pace_weight
            + self.power_weight
            + self.technical_weight
            + self.shooting_weight
            + self.passing_weight
            + self.defending_weight
    }

    /// Apply weights to hexagon values and return weighted sum
    pub fn apply_to_hexagon(
        &self,
        pace: u8,
        power: u8,
        technical: u8,
        shooting: u8,
        passing: u8,
        defending: u8,
    ) -> f32 {
        (pace as f32 * self.pace_weight)
            + (power as f32 * self.power_weight)
            + (technical as f32 * self.technical_weight)
            + (shooting as f32 * self.shooting_weight)
            + (passing as f32 * self.passing_weight)
            + (defending as f32 * self.defending_weight)
    }

    /// Get normalized weights (sum to 1.0)
    pub fn normalized(&self) -> Self {
        let total = self.total_weight();
        if total == 0.0 {
            return self.clone();
        }

        Self {
            pace_weight: self.pace_weight / total,
            power_weight: self.power_weight / total,
            technical_weight: self.technical_weight / total,
            shooting_weight: self.shooting_weight / total,
            passing_weight: self.passing_weight / total,
            defending_weight: self.defending_weight / total,
        }
    }
}

impl Default for PositionWeights {
    fn default() -> Self {
        // Balanced weighting for unknown positions
        Self {
            pace_weight: 1.0,
            power_weight: 1.0,
            technical_weight: 1.0,
            shooting_weight: 1.0,
            passing_weight: 1.0,
            defending_weight: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_weights() {
        let fw_weights = PositionWeights::get_for_position(Position::FW);

        assert_eq!(fw_weights.shooting_weight, 3.0, "Forwards should have 3x shooting weight");
        assert_eq!(fw_weights.pace_weight, 2.0, "Forwards should have 2x pace weight");
        assert_eq!(fw_weights.technical_weight, 2.0, "Forwards should have 2x technical weight");
        assert_eq!(fw_weights.power_weight, 2.0, "Forwards should have 2x power weight");
        assert_eq!(fw_weights.passing_weight, 1.0, "Forwards should have normal passing weight");
        assert_eq!(
            fw_weights.defending_weight, 0.5,
            "Forwards should have minimal defending weight"
        );

        assert!(!fw_weights.is_goalkeeper());
    }

    #[test]
    fn test_midfielder_weights() {
        let mf_weights = PositionWeights::get_for_position(Position::MF);

        assert_eq!(mf_weights.technical_weight, 3.0, "Midfielders should have 3x technical weight");
        assert_eq!(mf_weights.passing_weight, 3.0, "Midfielders should have 3x passing weight");
        assert_eq!(mf_weights.pace_weight, 2.0, "Midfielders should have 2x pace weight");
        assert_eq!(mf_weights.power_weight, 2.0, "Midfielders should have 2x power weight");
        assert_eq!(mf_weights.shooting_weight, 2.0, "Midfielders should have 2x shooting weight");
        assert_eq!(mf_weights.defending_weight, 2.0, "Midfielders should have 2x defending weight");

        assert!(!mf_weights.is_goalkeeper());
    }

    #[test]
    fn test_defender_weights() {
        let df_weights = PositionWeights::get_for_position(Position::DF);

        assert_eq!(df_weights.defending_weight, 3.0, "Defenders should have 3x defending weight");
        assert_eq!(df_weights.power_weight, 3.0, "Defenders should have 3x power weight");
        assert_eq!(df_weights.pace_weight, 2.0, "Defenders should have 2x pace weight");
        assert_eq!(df_weights.passing_weight, 2.0, "Defenders should have 2x passing weight");
        assert_eq!(
            df_weights.technical_weight, 1.0,
            "Defenders should have normal technical weight"
        );
        assert_eq!(
            df_weights.shooting_weight, 0.5,
            "Defenders should have minimal shooting weight"
        );

        assert!(!df_weights.is_goalkeeper());
    }

    #[test]
    fn test_goalkeeper_weights() {
        let gk_weights = PositionWeights::get_for_position(Position::GK);
        let gk_weights_direct = PositionWeights::goalkeeper_weights();

        assert_eq!(gk_weights, gk_weights_direct);
        assert_eq!(gk_weights.shooting_weight, 0.0, "GK should have zero shooting weight");
        assert_eq!(gk_weights.defending_weight, 3.0, "GK should have 3x defending weight");
        assert_eq!(gk_weights.power_weight, 2.0, "GK should have 2x power weight");
        assert_eq!(gk_weights.passing_weight, 1.5, "GK should have 1.5x passing weight");
        assert_eq!(gk_weights.pace_weight, 0.5, "GK should have minimal pace weight");
        assert_eq!(gk_weights.technical_weight, 0.5, "GK should have minimal technical weight");

        assert!(gk_weights.is_goalkeeper());
    }

    #[test]
    fn test_weight_application() {
        let fw_weights = PositionWeights::get_for_position(Position::FW);

        // Test with sample hexagon values
        let weighted_sum = fw_weights.apply_to_hexagon(15, 12, 18, 20, 10, 5);

        // Expected: 15*2 + 12*2 + 18*2 + 20*3 + 10*1 + 5*0.5 = 30+24+36+60+10+2.5 = 162.5
        assert_eq!(weighted_sum, 162.5);
    }

    #[test]
    fn test_normalization() {
        let weights = PositionWeights::get_for_position(Position::FW);
        let normalized = weights.normalized();

        let total = normalized.total_weight();
        assert!((total - 1.0).abs() < 0.001, "Normalized weights should sum to 1.0, got {}", total);

        // Check that relative proportions are maintained
        let original_shooting_ratio = weights.shooting_weight / weights.total_weight();
        assert!((normalized.shooting_weight - original_shooting_ratio).abs() < 0.001);
    }

    #[test]
    fn test_all_position_variants() {
        // Test that all position variants work
        let positions = [
            Position::FW,
            Position::ST,
            Position::CF,
            Position::LW,
            Position::RW,
            Position::MF,
            Position::CM,
            Position::CAM,
            Position::CDM,
            Position::LM,
            Position::RM,
            Position::DF,
            Position::CB,
            Position::LB,
            Position::RB,
            Position::LWB,
            Position::RWB,
            Position::GK,
        ];

        for position in positions {
            let weights = PositionWeights::get_for_position(position);
            assert!(
                weights.total_weight() > 0.0,
                "Position {:?} should have positive total weight",
                position
            );

            if position.is_goalkeeper() {
                assert!(weights.is_goalkeeper(), "GK position should be detected as goalkeeper");
                assert_eq!(weights.shooting_weight, 0.0, "GK should have zero shooting weight");
            } else {
                assert!(
                    !weights.is_goalkeeper(),
                    "Non-GK position should not be detected as goalkeeper"
                );
                assert!(
                    weights.shooting_weight > 0.0,
                    "Non-GK should have positive shooting weight"
                );
            }
        }
    }
}
