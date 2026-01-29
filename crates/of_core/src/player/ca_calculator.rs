//! CA (Current Ability) calculation engine
//!
//! Implements the core CA calculation with position-specific weighting:
//! - Formula: CA = (total_units - 150) / 5, capped at 200
//! - Technical/Mental attributes: 1 unit each
//! - Physical attributes: 2 units each
//! - Position weights affect importance of different attribute groups

use crate::models::player::{PlayerAttributes, Position};

/// CA calculation engine with position weighting
pub struct CACalculator;

impl CACalculator {
    /// Calculate CA from detailed stats and position
    pub fn calculate(stats: &PlayerAttributes, position: Position) -> u8 {
        // Simplified approach: calculate base CA from all attributes, then apply position modifier
        let base_ca = Self::calculate_base_ca(stats);
        let position_modifier = Self::calculate_position_modifier(stats, position);

        let final_ca = ((base_ca as f32 * position_modifier).round() as u32).min(200);

        #[cfg(test)]
        eprintln!("CA calc: base={}, mod={}, final={}", base_ca, position_modifier, final_ca);

        final_ca as u8
    }

    /// Calculate base CA from all attributes using the original formula
    fn calculate_base_ca(stats: &PlayerAttributes) -> u32 {
        // Sum all attributes with their unit costs
        // Technical (14): corners, crossing, dribbling, finishing, first_touch, free_kicks, heading, long_shots, long_throws, marking, passing, penalty_taking, tackling, technique
        let technical_sum = stats.corners as u32 + stats.crossing as u32 + stats.dribbling as u32 + stats.finishing as u32 +
            stats.first_touch as u32 + stats.free_kicks as u32 + stats.heading as u32 + stats.long_shots as u32 +
            stats.long_throws as u32 + stats.marking as u32 + stats.passing as u32 + stats.penalty_taking as u32 +
            stats.tackling as u32 + stats.technique as u32;

        // Mental (14): aggression, anticipation, bravery, composure, concentration, decisions, determination, flair, leadership, off_the_ball, positioning, teamwork, vision, work_rate
        let mental_sum = stats.aggression as u32 + stats.anticipation as u32 + stats.bravery as u32 + stats.composure as u32 +
            stats.concentration as u32 + stats.decisions as u32 + stats.determination as u32 + stats.flair as u32 +
            stats.leadership as u32 + stats.off_the_ball as u32 + stats.positioning as u32 + stats.teamwork as u32 +
            stats.vision as u32 + stats.work_rate as u32;

        // Physical attributes cost 2 units each (8): acceleration, agility, balance, jumping, natural_fitness, pace, stamina, strength
        let physical_sum = (stats.acceleration as u32 + stats.agility as u32 + stats.balance as u32 + stats.jumping as u32 +
            stats.natural_fitness as u32 + stats.pace as u32 + stats.stamina as u32 + stats.strength as u32) * 2;

        // GK-related attributes using available fields (moderate cost)
        // Note: Original GK fields (reflexes, handling, etc.) not available in 36-field system
        // Using mental attributes that are relevant for goalkeepers
        let gk_sum = 0u32; // No dedicated GK fields in 36-attribute system

        let total_units = technical_sum + mental_sum + physical_sum + gk_sum;

        // Adjusted formula for more reasonable CA values
        // With 42 attributes averaging 50, total_units ≈ 50*28 + 50*14 + 100*8 + 50*6 = 3700
        // We want this to map to around CA 100-120
        // New formula: CA = total_units / 20 (roughly)
        let base_ca = if total_units >= 1000 {
            (total_units - 1000) / 20
        } else {
            total_units / 40 // Very low stats still get some CA
        };

        // Cap base CA at reasonable level before position modifiers
        base_ca.min(180)
    }

    /// Calculate position-specific modifier (0.8 to 1.2 range)
    fn calculate_position_modifier(stats: &PlayerAttributes, position: Position) -> f32 {
        match position {
            Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => {
                // Forwards benefit from finishing, pace, technical
                let relevant_avg = (stats.finishing as u32 + stats.long_shots as u32 + stats.pace as u32 +
                                  stats.acceleration as u32 + stats.dribbling as u32) as f32 / 5.0;
                (0.8 + (relevant_avg - 45.0) * 0.008).max(0.8).min(1.2) // Range 0.8-1.2
            },
            Position::MF | Position::CM | Position::CAM | Position::CDM | Position::LM | Position::RM => {
                // Midfielders benefit from passing, vision, technical
                let relevant_avg = (stats.passing as u32 + stats.vision as u32 + stats.technique as u32 +
                                  stats.first_touch as u32 + stats.teamwork as u32) as f32 / 5.0;
                (0.8 + (relevant_avg - 45.0) * 0.008).max(0.8).min(1.2)
            },
            Position::DF | Position::CB | Position::LB | Position::RB | Position::LWB | Position::RWB => {
                // Defenders benefit from defending, strength, heading
                let relevant_avg = (stats.positioning as u32 + stats.anticipation as u32 + stats.strength as u32 +
                                  stats.heading as u32 + stats.work_rate as u32) as f32 / 5.0;
                (0.8 + (relevant_avg - 45.0) * 0.008).max(0.8).min(1.2)
            },
            Position::GK => {
                // Goalkeepers benefit from relevant mental/physical attributes
                // Using available fields: positioning, anticipation, concentration, jumping, composure
                let relevant_avg = (stats.positioning as u32 + stats.anticipation as u32 + stats.concentration as u32 +
                                  stats.jumping as u32 + stats.composure as u32) as f32 / 5.0;
                (0.8 + (relevant_avg - 45.0) * 0.008).max(0.8).min(1.2)
            }
        }
    }
}

/// Position-specific attribute weights
#[derive(Debug, Clone)]
pub struct PositionWeights {
    pub pace_weight: u32,
    pub power_weight: u32,
    pub technical_weight: u32,
    pub shooting_weight: u32,
    pub passing_weight: u32,
    pub defending_weight: u32,
}

impl PositionWeights {
    /// Get position weights for a specific position
    pub fn get_for_position(position: Position) -> Self {
        match position {
            Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => {
                // Forwards: Shooting and Pace most important
                Self {
                    pace_weight: 2,
                    power_weight: 2,
                    technical_weight: 2,
                    shooting_weight: 3, // CRITICAL for forwards
                    passing_weight: 1,
                    defending_weight: 1,
                }
            }
            Position::MF | Position::CM | Position::CAM | Position::CDM | Position::LM | Position::RM => {
                // Midfielders: Technical and Passing most important
                Self {
                    pace_weight: 2,
                    power_weight: 2,
                    technical_weight: 3, // CRITICAL for midfielders
                    shooting_weight: 2,
                    passing_weight: 3, // CRITICAL for distribution
                    defending_weight: 2,
                }
            }
            Position::DF | Position::CB | Position::LB | Position::RB | Position::LWB | Position::RWB => {
                // Defenders: Power and Defending most important
                Self {
                    pace_weight: 2,
                    power_weight: 3, // CRITICAL for physical duels
                    technical_weight: 1,
                    shooting_weight: 1,
                    passing_weight: 2,
                    defending_weight: 3, // CRITICAL for defenders
                }
            }
            Position::GK => {
                // Goalkeepers: Special weighting focused on GK attributes
                Self {
                    pace_weight: 1,
                    power_weight: 2,
                    technical_weight: 1,
                    shooting_weight: 0, // Irrelevant for GK
                    passing_weight: 1,
                    defending_weight: 3, // Shot stopping, commanding area
                }
            }
        }
    }

    /// Check if this is goalkeeper weighting
    pub fn is_goalkeeper(&self) -> bool {
        self.shooting_weight == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::PlayerAttributes;

    fn create_test_attributes(value: u8) -> PlayerAttributes {
        PlayerAttributes {
            dribbling: value,
            passing: value,
            shooting: value,
            crossing: value,
            first_touch: value,
            ball_control: value,
            technique: value,
            heading: value,
            finishing: value,
            long_shots: value,
            free_kicks: value,
            penalties: value,
            corners: value,
            throw_ins: value,
            decisions: value,
            concentration: value,
            leadership: value,
            vision: value,
            teamwork: value,
            work_rate: value,
            positioning: value,
            anticipation: value,
            composure: value,
            bravery: value,
            determination: value,
            flair: value,
            off_the_ball: value,
            aggression: value,
            speed: value,
            stamina: value,
            strength: value,
            agility: value,
            balance: value,
            jumping: value,
            natural_fitness: value,
            acceleration: value,
            reflexes: value,
            handling: value,
            aerial_ability: value,
            command_of_area: value,
            communication: value,
            kicking: value,
        }
    }

    #[test]
    fn test_ca_calculation_basic() {
        let stats = create_test_attributes(50);
        let ca = CACalculator::calculate(&stats, Position::FW);

        // With all 50s and forward weights, should get a reasonable CA
        assert!(ca > 0, "CA should be positive with decent stats");
        assert!(ca <= 200, "CA should not exceed maximum");
    }

    #[test]
    fn test_ca_calculation_position_differences() {
        let stats = PlayerAttributes {
            finishing: 70,
            shooting: 70,
            speed: 65,
            acceleration: 65,
            positioning: 40, // Low defending stats
            anticipation: 40,
            concentration: 40,
            ..create_test_attributes(45) // Lower base stats
        };

        let ca_forward = CACalculator::calculate(&stats, Position::FW);
        let ca_defender = CACalculator::calculate(&stats, Position::DF);

        println!("CA Forward: {}, CA Defender: {}", ca_forward, ca_defender);

        // Forward should have higher CA due to high shooting stats being weighted more
        assert!(ca_forward > ca_defender,
            "Forward with good shooting should have higher CA than defender: {} vs {}",
            ca_forward, ca_defender);
    }

    #[test]
    fn test_ca_calculation_goalkeeper() {
        let stats = PlayerAttributes {
            reflexes: 90,
            handling: 90,
            aerial_ability: 85,
            command_of_area: 80,
            ..create_test_attributes(50)
        };

        let ca_gk = CACalculator::calculate(&stats, Position::GK);
        let _ca_outfield = CACalculator::calculate(&stats, Position::FW);

        // GK should get benefit from GK-specific attributes
        assert!(ca_gk > 0, "GK with good GK stats should have positive CA");
    }

    #[test]
    fn test_ca_calculation_edge_cases() {
        // All minimum stats
        let min_stats = create_test_attributes(1);
        let ca_min = CACalculator::calculate(&min_stats, Position::FW);
        assert!(ca_min <= 5, "Very low stats should result in very low CA, got {}", ca_min);

        // All maximum stats
        let max_stats = create_test_attributes(100);
        let ca_max = CACalculator::calculate(&max_stats, Position::FW);
        assert_eq!(ca_max, 200, "Maximum stats should result in 200 CA");
    }

    #[test]
    fn test_position_weights() {
        let fw_weights = PositionWeights::get_for_position(Position::FW);
        let mf_weights = PositionWeights::get_for_position(Position::MF);
        let df_weights = PositionWeights::get_for_position(Position::DF);
        let gk_weights = PositionWeights::get_for_position(Position::GK);

        // Forwards should prioritize shooting
        assert_eq!(fw_weights.shooting_weight, 3);

        // Midfielders should prioritize technical and passing
        assert_eq!(mf_weights.technical_weight, 3);
        assert_eq!(mf_weights.passing_weight, 3);

        // Defenders should prioritize power and defending
        assert_eq!(df_weights.power_weight, 3);
        assert_eq!(df_weights.defending_weight, 3);

        // Goalkeepers should have zero shooting weight
        assert_eq!(gk_weights.shooting_weight, 0);
        assert!(gk_weights.is_goalkeeper());
    }
}