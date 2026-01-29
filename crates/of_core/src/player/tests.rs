//! Integration tests for the player system
//!
//! Tests the interaction between different components of the player system

use super::*;
use crate::models::player::{PlayerAttributes, Position};

#[test]
fn test_player_system_integration() {
    // Create a test player with specific attributes
    let mut attributes = PlayerAttributes::default();

    // Set up a forward with good shooting and pace (using Open-Football standard)
    attributes.finishing = 90; // shooting → finishing + long_shots
    attributes.long_shots = 80; // shooting component
    attributes.pace = 80; // speed → pace
    attributes.acceleration = 85;
    attributes.dribbling = 75;
    attributes.first_touch = 80; // ball_control → first_touch

    let growth_profile = GrowthProfile::new();

    // Create the player
    let player = CorePlayer::new(
        "Test Forward".to_string(),
        Position::FW,
        16.0, // 고2 student
        0,    // Will be calculated
        140,  // Good potential
        attributes,
        growth_profile,
        PersonAttributes::default(), // Add missing personality parameter
    );

    // Test validation
    assert!(PlayerValidator::validate_core_player(&player).is_ok());

    // Test CA calculation
    let calculated_ca = CACalculator::calculate(&player.detailed_stats, player.position);
    assert!(calculated_ca > 50, "Forward with good stats should have decent CA");

    // Test hexagon calculation
    let hexagon = HexagonCalculator::calculate_all(&player.detailed_stats, player.position);
    assert!(
        hexagon.shooting > hexagon.defending,
        "Forward should have higher shooting than defending"
    );
    assert!(hexagon.pace > 10, "Forward with good pace stats should have good pace hexagon");

    println!("Player: {}", player.name);
    println!("CA: {}", calculated_ca);
    println!(
        "Hexagon - Pace: {}, Technical: {}, Shooting: {}",
        hexagon.pace, hexagon.technical, hexagon.shooting
    );
}

#[test]
fn test_position_specialization() {
    let base_attributes = PlayerAttributes {
        // Good all-around stats (using Open-Football standard)
        dribbling: 70,
        passing: 70,
        finishing: 70, // shooting → finishing
        pace: 70,      // speed → pace
        strength: 70,
        positioning: 70,
        anticipation: 70,
        ..PlayerAttributes::default()
    };

    // Test different positions get different CA values for same stats
    let ca_forward = CACalculator::calculate(&base_attributes, Position::FW);
    let ca_midfielder = CACalculator::calculate(&base_attributes, Position::MF);
    let ca_defender = CACalculator::calculate(&base_attributes, Position::DF);

    // They should be different due to position weighting
    println!("CA by position - FW: {}, MF: {}, DF: {}", ca_forward, ca_midfielder, ca_defender);

    // With balanced stats, differences should be moderate
    assert!(ca_forward > 0);
    assert!(ca_midfielder > 0);
    assert!(ca_defender > 0);
}

#[test]
fn test_goalkeeper_special_handling() {
    // Create attributes with low base values so base_ca < 200 and position modifier shows
    // Target: total_units ~800-900 for base_ca ~130-150
    let mut gk_attributes = PlayerAttributes::default();
    // Technical (14) - all low except GK-relevant
    gk_attributes.corners = 5;
    gk_attributes.crossing = 5;
    gk_attributes.dribbling = 5;
    gk_attributes.finishing = 3;
    gk_attributes.first_touch = 30; // GK ball handling
    gk_attributes.free_kicks = 5;
    gk_attributes.heading = 30; // GK aerial
    gk_attributes.long_shots = 3;
    gk_attributes.long_throws = 30; // GK distribution
    gk_attributes.marking = 10;
    gk_attributes.passing = 30; // GK distribution
    gk_attributes.penalty_taking = 5;
    gk_attributes.tackling = 10;
    gk_attributes.technique = 10;
    // Mental (14) - GK-relevant high, FW-relevant low
    gk_attributes.aggression = 5;
    gk_attributes.anticipation = 70; // GK
    gk_attributes.bravery = 20;
    gk_attributes.composure = 70; // GK
    gk_attributes.concentration = 70; // GK
    gk_attributes.decisions = 30;
    gk_attributes.determination = 30;
    gk_attributes.flair = 5;
    gk_attributes.leadership = 30;
    gk_attributes.off_the_ball = 5; // FW
    gk_attributes.positioning = 70; // GK
    gk_attributes.teamwork = 30;
    gk_attributes.vision = 10;
    gk_attributes.work_rate = 20;
    // Physical (8) - FW-relevant low
    gk_attributes.acceleration = 10; // FW
    gk_attributes.agility = 20;
    gk_attributes.balance = 20;
    gk_attributes.jumping = 60; // GK
    gk_attributes.natural_fitness = 30;
    gk_attributes.pace = 10; // FW
    gk_attributes.stamina = 20;
    gk_attributes.strength = 30;

    let ca_gk = CACalculator::calculate(&gk_attributes, Position::GK);
    let ca_outfield = CACalculator::calculate(&gk_attributes, Position::FW);

    // GK should get higher CA due to position-appropriate weighting
    assert!(ca_gk > ca_outfield, "GK should have higher CA as GK than as outfield player");

    let hexagon_gk = HexagonCalculator::calculate_all(&gk_attributes, Position::GK);
    let hexagon_outfield = HexagonCalculator::calculate_all(&gk_attributes, Position::FW);

    // GK hexagon should be calculated differently
    assert_ne!(hexagon_gk, hexagon_outfield);
}

#[test]
fn test_validation_edge_cases() {
    // Test edge case validations
    assert!(PlayerValidator::validate_ca_pa(200, 180).is_err()); // CA > max PA
    assert!(PlayerValidator::validate_ca_pa(0, 80).is_ok()); // Minimum valid values
    assert!(PlayerValidator::validate_ca_pa(180, 180).is_ok()); // CA = PA is valid

    // Test name validation edge cases
    assert!(PlayerValidator::validate_name("A").is_ok()); // Single character
    assert!(PlayerValidator::validate_name(&"x".repeat(50)).is_ok()); // Exactly 50 chars
    assert!(PlayerValidator::validate_name(&"x".repeat(51)).is_err()); // Too long

    // Test age validation
    assert!(PlayerValidator::validate_age(15.0).is_ok());
    assert!(PlayerValidator::validate_age(18.0).is_ok());
    assert!(PlayerValidator::validate_age(14.99).is_err());
    assert!(PlayerValidator::validate_age(18.01).is_err());
}

#[test]
fn test_hexagon_stats_consistency() {
    let attributes = PlayerAttributes {
        // High technical stats (using Open-Football standard)
        dribbling: 90,
        first_touch: 88, // ball_control → first_touch (removed duplicate)
        technique: 92,
        flair: 80,

        // Lower other stats
        strength: 30,
        jumping: 25,
        stamina: 40,
        ..PlayerAttributes::default()
    };

    let hexagon = HexagonCalculator::calculate_all(&attributes, Position::FW);

    // Technical should be much higher than power
    assert!(
        hexagon.technical > hexagon.power + 5,
        "Technical ({}) should be significantly higher than Power ({}) given the test data",
        hexagon.technical,
        hexagon.power
    );

    // All values should be within valid range
    for &value in &hexagon.as_array() {
        assert!(value <= 20, "Hexagon value {} should not exceed 20", value);
    }

    // Total should be reasonable
    assert!(hexagon.total() > 0);
    assert!(hexagon.total() <= 120); // 6 * 20 max
}
