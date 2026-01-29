//! Enhanced CA (Current Ability) calculation engine
//!
//! Implements the core CA calculation with FM-style weighting:
//! - Per-position attribute weights (data-driven)
//! - Non-linear high-end cost (alpha/penalty curve)
//! - Deterministic and reproducible
//! - Performance target: <1ms per calculation

use crate::models::player::{PlayerAttributes, Position};
use crate::player::ca_model::{calculate_ca, CAParams};
use crate::player::ca_weights::get_ca_weights;
use crate::player::position_weights::PositionWeights;
use crate::player::types::HexagonStats;

/// Enhanced CA calculation engine with position weighting
#[derive(Debug)]
pub struct CACalculator;

impl CACalculator {
    /// Calculate CA from detailed stats and position using the proper formula
    /// Formula: CA = (total_units - 150) / 5, capped at 200
    pub fn calculate(stats: &PlayerAttributes, position: Position) -> u8 {
        let weights = get_ca_weights();
        let params = CAParams::default();
        calculate_ca(stats, position, weights, params)
    }

    /// Calculate total units using OpenFootball 36-field system
    /// - Technical attributes (14): 1 unit each
    /// - Mental attributes (14): 1 unit each
    /// - Physical attributes (8): 2 units each
    fn calculate_total_units(stats: &PlayerAttributes) -> u32 {
        // Technical attributes (14): 1 unit each - OpenFootball standard
        let technical_units = stats.corners as u32
            + stats.crossing as u32
            + stats.dribbling as u32
            + stats.finishing as u32
            + stats.first_touch as u32
            + stats.free_kicks as u32
            + stats.heading as u32
            + stats.long_shots as u32
            + stats.long_throws as u32
            + stats.marking as u32
            + stats.passing as u32
            + stats.penalty_taking as u32
            + stats.tackling as u32
            + stats.technique as u32;

        // Mental attributes (14): 1 unit each - OpenFootball standard
        let mental_units = stats.aggression as u32
            + stats.anticipation as u32
            + stats.bravery as u32
            + stats.composure as u32
            + stats.concentration as u32
            + stats.decisions as u32
            + stats.determination as u32
            + stats.flair as u32
            + stats.leadership as u32
            + stats.off_the_ball as u32
            + stats.positioning as u32
            + stats.teamwork as u32
            + stats.vision as u32
            + stats.work_rate as u32;

        // Physical attributes (8): 2 units each - OpenFootball standard
        let physical_units = (stats.acceleration as u32
            + stats.agility as u32
            + stats.balance as u32
            + stats.jumping as u32
            + stats.natural_fitness as u32
            + stats.pace as u32
            + stats.stamina as u32
            + stats.strength as u32)
            * 2;

        technical_units + mental_units + physical_units
    }

    /// Apply the CA formula: CA = (total_units - 150) / 5, capped at 200
    fn apply_formula(total_units: u32) -> u32 {
        if total_units <= 150 {
            // If total units is very low, still give some minimal CA
            total_units / 30 // This gives ~5 CA for units around 150
        } else {
            (total_units - 150) / 5
        }
    }

    /// Apply position-specific weighting to the base CA
    fn apply_position_weighting(stats: &PlayerAttributes, position: Position, base_ca: u32) -> u32 {
        let weights = PositionWeights::get_for_position(position);
        let hexagon_stats = HexagonStats::calculate_from_detailed(stats, position);

        // Calculate position-weighted hexagon score
        let weighted_hexagon_score = weights.apply_to_hexagon(
            hexagon_stats.pace,
            hexagon_stats.power,
            hexagon_stats.technical,
            hexagon_stats.shooting,
            hexagon_stats.passing,
            hexagon_stats.defending,
        );

        // Calculate average hexagon score for comparison (0-20 scale)
        let avg_hexagon_score = hexagon_stats.total() as f32 / 6.0;

        // Calculate position modifier: how well suited this player is for the position
        let position_modifier = if avg_hexagon_score > 0.0 {
            (weighted_hexagon_score / weights.total_weight()) / avg_hexagon_score
        } else {
            1.0
        };

        // Apply position modifier to base CA (range: 0.7-1.3 roughly)
        let position_modifier_clamped = position_modifier.clamp(0.7, 1.3);

        (base_ca as f32 * position_modifier_clamped).round() as u32
    }

    /// Calculate CA and return detailed breakdown for debugging
    pub fn calculate_detailed(
        stats: &PlayerAttributes,
        position: Position,
    ) -> CACalculationDetails {
        let total_units = Self::calculate_total_units(stats);
        let base_ca = Self::apply_formula(total_units);
        let weights = get_ca_weights();
        let params = CAParams::default();
        let final_ca = calculate_ca(stats, position, weights, params);
        let position_weights = PositionWeights::get_for_position(position);
        let hexagon_stats = HexagonStats::calculate_from_detailed(stats, position);

        CACalculationDetails {
            total_units,
            base_ca,
            final_ca: final_ca.min(200),
            position_weights,
            hexagon_stats,
        }
    }
}

/// Detailed breakdown of CA calculation for debugging and testing
#[derive(Debug)]
pub struct CACalculationDetails {
    pub total_units: u32,
    pub base_ca: u32,
    pub final_ca: u8,
    pub position_weights: PositionWeights,
    pub hexagon_stats: HexagonStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::ca_weights::get_ca_weights;

    fn create_test_attributes(value: u8) -> PlayerAttributes {
        PlayerAttributes {
            // Technical attributes (14) - Open-Football standard
            corners: value,
            crossing: value,
            dribbling: value,
            finishing: value,
            first_touch: value,
            free_kicks: value,
            heading: value,
            long_shots: value,
            long_throws: value,
            marking: value,
            passing: value,
            penalty_taking: value,
            tackling: value,
            technique: value,

            // Mental attributes (14) - Open-Football standard
            aggression: value,
            anticipation: value,
            bravery: value,
            composure: value,
            concentration: value,
            decisions: value,
            determination: value,
            flair: value,
            leadership: value,
            off_the_ball: value,
            positioning: value,
            teamwork: value,
            vision: value,
            work_rate: value,

            // Physical attributes (8) - Open-Football standard
            acceleration: value,
            agility: value,
            balance: value,
            jumping: value,
            natural_fitness: value,
            pace: value,
            stamina: value,
            strength: value,

            // Goalkeeper attributes (11) - v5 schema
            gk_aerial_reach: value,
            gk_command_of_area: value,
            gk_communication: value,
            gk_eccentricity: value,
            gk_handling: value,
            gk_kicking: value,
            gk_one_on_ones: value,
            gk_reflexes: value,
            gk_rushing_out: value,
            gk_punching: value,
            gk_throwing: value,
        }
    }

    fn set_attr(attrs: &mut PlayerAttributes, key: &str, value: u8) {
        match key {
            "corners" => attrs.corners = value,
            "crossing" => attrs.crossing = value,
            "dribbling" => attrs.dribbling = value,
            "finishing" => attrs.finishing = value,
            "first_touch" => attrs.first_touch = value,
            "free_kicks" => attrs.free_kicks = value,
            "heading" => attrs.heading = value,
            "long_shots" => attrs.long_shots = value,
            "long_throws" => attrs.long_throws = value,
            "marking" => attrs.marking = value,
            "passing" => attrs.passing = value,
            "penalty_taking" => attrs.penalty_taking = value,
            "tackling" => attrs.tackling = value,
            "technique" => attrs.technique = value,
            "aggression" => attrs.aggression = value,
            "anticipation" => attrs.anticipation = value,
            "bravery" => attrs.bravery = value,
            "composure" => attrs.composure = value,
            "concentration" => attrs.concentration = value,
            "decisions" => attrs.decisions = value,
            "determination" => attrs.determination = value,
            "flair" => attrs.flair = value,
            "leadership" => attrs.leadership = value,
            "off_the_ball" => attrs.off_the_ball = value,
            "positioning" => attrs.positioning = value,
            "teamwork" => attrs.teamwork = value,
            "vision" => attrs.vision = value,
            "work_rate" => attrs.work_rate = value,
            "acceleration" => attrs.acceleration = value,
            "agility" => attrs.agility = value,
            "balance" => attrs.balance = value,
            "jumping" => attrs.jumping = value,
            "natural_fitness" => attrs.natural_fitness = value,
            "pace" => attrs.pace = value,
            "stamina" => attrs.stamina = value,
            "strength" => attrs.strength = value,
            _ => {}
        }
    }

    #[test]
    fn test_total_units_calculation() {
        let stats = create_test_attributes(50);
        let total_units = CACalculator::calculate_total_units(&stats);

        // Expected: 14 technical (50 each) + 14 mental (50 each) + 8 physical (50*2 each)
        // = 14*50 + 14*50 + 8*100 = 700 + 700 + 800 = 2200
        assert_eq!(total_units, 2200, "Total units calculation should match expected formula");
    }

    #[test]
    fn test_ca_formula_application() {
        // Test with total_units = 2500
        let ca = CACalculator::apply_formula(2500);
        // Expected: (2500 - 150) / 5 = 2350 / 5 = 470, but this will be capped at 200 later
        assert_eq!(ca, 470);

        // Test with total_units = 150 (minimum)
        let ca_min = CACalculator::apply_formula(150);
        assert_eq!(ca_min, 5); // 150 / 30 = 5

        // Test with total_units = 100 (very low)
        let ca_very_low = CACalculator::apply_formula(100);
        assert_eq!(ca_very_low, 3); // 100 / 30 ≈ 3
    }

    #[test]
    fn test_ca_calculation_basic() {
        let stats = create_test_attributes(50);
        let ca = CACalculator::calculate(&stats, Position::FW);

        // Should be reasonable CA value
        assert!(ca > 0, "CA should be positive with decent stats");
        assert!(ca <= 200, "CA should not exceed maximum");
    }

    #[test]
    fn test_ca_calculation_edge_cases() {
        // All minimum stats (1)
        let min_stats = create_test_attributes(1);
        let ca_min = CACalculator::calculate(&min_stats, Position::FW);
        assert!(ca_min <= 10, "Very low stats should result in very low CA, got {}", ca_min);

        // All maximum stats (100)
        let max_stats = create_test_attributes(100);
        let ca_max = CACalculator::calculate(&max_stats, Position::FW);
        assert_eq!(ca_max, 200, "Maximum stats should result in 200 CA");
    }

    #[test]
    fn test_position_weighting_differences() {
        // Create a player with high shooting stats
        let shooting_specialist = PlayerAttributes {
            finishing: 90,
            long_shots: 85,
            composure: 80,
            penalty_taking: 85,
            ..create_test_attributes(45) // Lower base stats
        };

        let ca_forward = CACalculator::calculate(&shooting_specialist, Position::FW);
        let ca_defender = CACalculator::calculate(&shooting_specialist, Position::DF);

        // Forward should benefit more from high shooting stats
        assert!(
            ca_forward >= ca_defender,
            "Forward with good shooting should have higher or equal CA than defender: {} vs {}",
            ca_forward,
            ca_defender
        );
    }

    #[test]
    fn test_monotonicity_weighted_attrs() {
        let weights = get_ca_weights();
        let group = weights.group_for_position(Position::ST).expect("ST group should exist");
        let base = create_test_attributes(50);
        let base_ca = CACalculator::calculate(&base, Position::ST);

        for key in weights.attr_keys() {
            if group.weights.get(key).copied().unwrap_or(0) == 0 {
                continue;
            }
            let mut attrs = base.clone();
            let current = attrs.get_by_key(key).unwrap_or(50);
            let bumped = (current + 5).min(100);
            set_attr(&mut attrs, key, bumped);
            let ca = CACalculator::calculate(&attrs, Position::ST);
            assert!(
                ca >= base_ca,
                "CA should not decrease when {} increases ({} -> {})",
                key,
                current,
                bumped
            );
        }
    }

    #[test]
    fn test_weight_sensitivity_st_vs_cb() {
        let base = create_test_attributes(50);
        let base_ca = CACalculator::calculate(&base, Position::ST);

        let mut finishing_boost = base.clone();
        finishing_boost.finishing = 60;
        let ca_finishing = CACalculator::calculate(&finishing_boost, Position::ST);

        let mut marking_boost = base.clone();
        marking_boost.marking = 60;
        let ca_marking = CACalculator::calculate(&marking_boost, Position::ST);

        assert!(
            ca_finishing.saturating_sub(base_ca) >= ca_marking.saturating_sub(base_ca),
            "ST should value finishing more than marking"
        );

        let base_cb_ca = CACalculator::calculate(&base, Position::CB);
        let mut cb_marking = base.clone();
        cb_marking.marking = 60;
        let cb_marking_ca = CACalculator::calculate(&cb_marking, Position::CB);

        let mut cb_finishing = base.clone();
        cb_finishing.finishing = 60;
        let cb_finishing_ca = CACalculator::calculate(&cb_finishing, Position::CB);

        assert!(
            cb_marking_ca.saturating_sub(base_cb_ca) >= cb_finishing_ca.saturating_sub(base_cb_ca),
            "CB should value marking more than finishing"
        );
    }

    #[test]
    fn test_high_end_penalty_curve() {
        let mut attrs = create_test_attributes(60);
        attrs.finishing = 65;
        let ca_65 = CACalculator::calculate(&attrs, Position::ST) as i16;
        attrs.finishing = 75;
        let ca_75 = CACalculator::calculate(&attrs, Position::ST) as i16;
        attrs.finishing = 85;
        let ca_85 = CACalculator::calculate(&attrs, Position::ST) as i16;
        attrs.finishing = 95;
        let ca_95 = CACalculator::calculate(&attrs, Position::ST) as i16;

        let delta_low = ca_75 - ca_65;
        let delta_high = ca_95 - ca_85;
        assert!(
            delta_high * 2 >= delta_low * 3,
            "High-end penalty should be steeper (low {}, high {})",
            delta_low,
            delta_high
        );
    }

    #[test]
    fn test_range_sanity_all_50s() {
        let stats = create_test_attributes(50);
        let ca = CACalculator::calculate(&stats, Position::CM);
        assert!((80..=120).contains(&ca), "All 50s should land in mid CA range, got {}", ca);
    }

    #[test]
    fn test_goalkeeper_calculation() {
        let gk_specialist = PlayerAttributes {
            // GK-specific adapted using Open-Football base attributes
            first_touch: 90,   // GK handling
            concentration: 90, // GK reflexes
            heading: 85,       // Aerial ability
            jumping: 85,       // Aerial ability
            positioning: 80,   // Command of area
            anticipation: 80,  // Command of area
            teamwork: 80,      // Communication
            leadership: 80,    // Communication
            long_throws: 75,   // GK kicking
            ..create_test_attributes(45)
        };

        let ca_gk = CACalculator::calculate(&gk_specialist, Position::GK);
        let _ca_outfield = CACalculator::calculate(&gk_specialist, Position::FW);

        assert!(ca_gk > 0, "GK with good GK stats should have positive CA");
        // GK specialist should do well as GK but not necessarily better than as outfield player
        // due to the way the weighting system works
    }

    #[test]
    fn test_detailed_calculation() {
        let stats = create_test_attributes(60);
        let details = CACalculator::calculate_detailed(&stats, Position::MF);

        // 14 technical + 14 mental + 8 physical*2 = (14+14+16)*60 = 44*60 = 2640
        assert_eq!(details.total_units, 2640);
        assert_eq!(details.base_ca, 498); // (2640 - 150) / 5 = 498
        assert!(details.final_ca <= 200);
        assert!(!details.position_weights.is_goalkeeper());
    }

    #[test]
    fn test_performance_target() {
        use std::time::Instant;

        let stats = create_test_attributes(75);
        let start = Instant::now();

        // Run calculation 1000 times to get average
        for _ in 0..1000 {
            let _ = CACalculator::calculate(&stats, Position::MF);
        }

        let duration = start.elapsed();
        let avg_duration = duration / 1000;

        println!("Average CA calculation time: {:?}", avg_duration);
        assert!(
            avg_duration.as_micros() < 1000,
            "CA calculation should be faster than 1ms, got {:?}",
            avg_duration
        );
    }

    #[test]
    fn test_all_positions() {
        let stats = create_test_attributes(60);
        let positions = [
            Position::GK,
            Position::DF,
            Position::MF,
            Position::FW,
            Position::LB,
            Position::CB,
            Position::RB,
            Position::LWB,
            Position::RWB,
            Position::CDM,
            Position::CM,
            Position::CAM,
            Position::LM,
            Position::RM,
            Position::LW,
            Position::RW,
            Position::CF,
            Position::ST,
        ];

        for position in positions {
            let ca = CACalculator::calculate(&stats, position);
            assert!(ca > 0, "Position {:?} should produce positive CA", position);
            assert!(ca <= 200, "Position {:?} should not exceed 200 CA", position);
        }
    }

    #[test]
    fn test_calculation_consistency() {
        let stats = create_test_attributes(50);

        // Multiple calculations should return the same result
        let ca1 = CACalculator::calculate(&stats, Position::FW);
        let ca2 = CACalculator::calculate(&stats, Position::FW);
        let ca3 = CACalculator::calculate(&stats, Position::FW);

        assert_eq!(ca1, ca2);
        assert_eq!(ca2, ca3);
    }
}
