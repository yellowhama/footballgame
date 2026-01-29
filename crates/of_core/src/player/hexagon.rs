//! Hexagon statistics calculation
//!
//! Handles the conversion from 42 detailed attributes to 6-sided hexagon stats
//! for visual representation and simplified player comparison.

use crate::models::player::{PlayerAttributes, Position};
use crate::player::types::HexagonStats;

/// Calculator for hexagon statistics
pub struct HexagonCalculator;

impl HexagonCalculator {
    /// Calculate all hexagon stats from detailed attributes
    pub fn calculate_all(stats: &PlayerAttributes, position: Position) -> HexagonStats {
        HexagonStats::calculate_from_detailed(stats, position)
    }

    /// Calculate individual PACE stat
    pub fn calculate_pace(stats: &PlayerAttributes) -> u8 {
        stats.calculate_pace()
    }

    /// Calculate individual POWER stat
    pub fn calculate_power(stats: &PlayerAttributes) -> u8 {
        stats.calculate_power()
    }

    /// Calculate individual TECHNICAL stat
    pub fn calculate_technical(stats: &PlayerAttributes) -> u8 {
        stats.calculate_technical()
    }

    /// Calculate individual SHOOTING stat
    pub fn calculate_shooting(stats: &PlayerAttributes) -> u8 {
        stats.calculate_shooting()
    }

    /// Calculate individual PASSING stat
    pub fn calculate_passing(stats: &PlayerAttributes) -> u8 {
        stats.calculate_passing()
    }

    /// Calculate individual DEFENDING stat
    pub fn calculate_defending(stats: &PlayerAttributes) -> u8 {
        stats.calculate_defending()
    }

    /// Calculate hexagon stats for goalkeepers using GK-specific logic
    pub fn calculate_goalkeeper_hexagon(stats: &PlayerAttributes) -> HexagonStats {
        let (pace, power, technical, shooting, passing, defending) = stats.calculate_gk_hexagon();
        HexagonStats { pace, power, technical, shooting, passing, defending }
    }

    /// Get attribute contributors for a specific hexagon stat (for UI tooltips)
    pub fn get_pace_contributors() -> Vec<&'static str> {
        vec!["pace", "acceleration", "agility", "balance", "off_the_ball"]
    }

    /// Get attribute contributors for POWER hexagon stat
    pub fn get_power_contributors() -> Vec<&'static str> {
        vec!["strength", "jumping", "stamina", "natural_fitness", "heading", "bravery"]
    }

    /// Get attribute contributors for TECHNICAL hexagon stat
    pub fn get_technical_contributors() -> Vec<&'static str> {
        vec!["dribbling", "first_touch", "technique", "flair"]
    }

    /// Get attribute contributors for SHOOTING hexagon stat
    pub fn get_shooting_contributors() -> Vec<&'static str> {
        vec!["finishing", "long_shots", "composure", "penalty_taking"]
    }

    /// Get attribute contributors for PASSING hexagon stat
    pub fn get_passing_contributors() -> Vec<&'static str> {
        vec!["passing", "vision", "crossing", "teamwork", "free_kicks", "corners"]
    }

    /// Get attribute contributors for DEFENDING hexagon stat
    pub fn get_defending_contributors() -> Vec<&'static str> {
        vec![
            "positioning",
            "anticipation",
            "concentration",
            "aggression",
            "work_rate",
            "determination",
        ]
    }

    /// Get GK-specific contributors for each hexagon stat
    pub fn get_gk_contributors(hexagon_stat: &str) -> Vec<&'static str> {
        match hexagon_stat.to_lowercase().as_str() {
            "pace" => vec!["first_touch", "agility"], // GK reflexes → first_touch
            "power" => vec!["heading", "strength", "jumping"], // aerial_ability → heading + jumping
            "technical" => vec!["first_touch", "concentration"], // handling/reflexes → first_touch + concentration
            "shooting" => vec!["long_throws"],                   // kicking → long_throws
            "passing" => vec!["long_throws", "passing"],         // kicking → long_throws + passing
            "defending" => vec!["positioning", "anticipation", "teamwork", "concentration"], // command_of_area/communication → positioning + anticipation + teamwork
            _ => vec![],
        }
    }

    /// Compare two hexagon stats and return differences
    pub fn compare_hexagon_stats(stats1: &HexagonStats, stats2: &HexagonStats) -> HexagonStatsDiff {
        HexagonStatsDiff {
            pace_diff: stats1.pace as i16 - stats2.pace as i16,
            power_diff: stats1.power as i16 - stats2.power as i16,
            technical_diff: stats1.technical as i16 - stats2.technical as i16,
            shooting_diff: stats1.shooting as i16 - stats2.shooting as i16,
            passing_diff: stats1.passing as i16 - stats2.passing as i16,
            defending_diff: stats1.defending as i16 - stats2.defending as i16,
        }
    }
}

/// Difference between two hexagon stats (for comparison UI)
#[derive(Debug, Clone, PartialEq)]
pub struct HexagonStatsDiff {
    pub pace_diff: i16,
    pub power_diff: i16,
    pub technical_diff: i16,
    pub shooting_diff: i16,
    pub passing_diff: i16,
    pub defending_diff: i16,
}

impl HexagonStatsDiff {
    /// Get the total absolute difference
    pub fn total_diff(&self) -> u16 {
        self.pace_diff.unsigned_abs()
            + self.power_diff.unsigned_abs()
            + self.technical_diff.unsigned_abs()
            + self.shooting_diff.unsigned_abs()
            + self.passing_diff.unsigned_abs()
            + self.defending_diff.unsigned_abs()
    }

    /// Get the largest positive difference (biggest strength)
    pub fn biggest_strength(&self) -> (&'static str, i16) {
        let diffs = [
            ("pace", self.pace_diff),
            ("power", self.power_diff),
            ("technical", self.technical_diff),
            ("shooting", self.shooting_diff),
            ("passing", self.passing_diff),
            ("defending", self.defending_diff),
        ];

        diffs
            .iter()
            .filter(|(_, diff)| *diff > 0)
            .max_by_key(|(_, diff)| *diff)
            .map(|(name, diff)| (*name, *diff))
            .unwrap_or(("none", 0))
    }

    /// Get the largest negative difference (biggest weakness)
    pub fn biggest_weakness(&self) -> (&'static str, i16) {
        let diffs = [
            ("pace", self.pace_diff),
            ("power", self.power_diff),
            ("technical", self.technical_diff),
            ("shooting", self.shooting_diff),
            ("passing", self.passing_diff),
            ("defending", self.defending_diff),
        ];

        diffs
            .iter()
            .filter(|(_, diff)| *diff < 0)
            .min_by_key(|(_, diff)| *diff)
            .map(|(name, diff)| (*name, *diff))
            .unwrap_or(("none", 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::PlayerAttributes;

    fn create_test_attributes() -> PlayerAttributes {
        PlayerAttributes {
            // Technical attributes (14) - Open-Football standard
            corners: 50,
            crossing: 60,
            dribbling: 85, // High technical
            finishing: 60,
            first_touch: 75, // High technical
            free_kicks: 55,
            heading: 30, // Lower power
            long_shots: 50,
            long_throws: 50,
            marking: 50,
            passing: 65,
            penalty_taking: 55,
            tackling: 50,
            technique: 80, // High technical

            // Mental attributes (14) - Open-Football standard
            aggression: 50,
            anticipation: 50,
            bravery: 40, // Lower power
            composure: 65,
            concentration: 50,
            decisions: 50,
            determination: 50,
            flair: 70, // High technical
            leadership: 50,
            off_the_ball: 60, // High pace
            positioning: 50,
            teamwork: 65,
            vision: 70,
            work_rate: 50,

            // Physical attributes (8) - Open-Football standard
            acceleration: 80,    // High pace
            agility: 70,         // High pace
            balance: 65,         // High pace
            jumping: 35,         // Lower power
            natural_fitness: 45, // Lower power
            pace: 75,            // High pace
            stamina: 50,
            strength: 40, // Lower power
            ..PlayerAttributes::default()
        }
    }

    #[test]
    fn test_hexagon_calculation_outfield() {
        let stats = create_test_attributes();
        let hexagon = HexagonCalculator::calculate_all(&stats, Position::FW);

        // Should have high technical due to high dribbling/first_touch/technique
        assert!(
            hexagon.technical > hexagon.power,
            "Technical ({}) should be higher than Power ({}) based on test stats",
            hexagon.technical,
            hexagon.power
        );

        // Should have decent pace due to high pace/acceleration/agility
        assert!(hexagon.pace > 10, "Pace should be above 10 with good pace stats");

        // All values should be within valid range
        assert!(hexagon.pace <= 20);
        assert!(hexagon.power <= 20);
        assert!(hexagon.technical <= 20);
        assert!(hexagon.shooting <= 20);
        assert!(hexagon.passing <= 20);
        assert!(hexagon.defending <= 20);
    }

    #[test]
    fn test_hexagon_calculation_goalkeeper() {
        let gk_stats = PlayerAttributes {
            // GK-specific adapted using Open-Football base attributes
            first_touch: 90,   // GK handling/reflexes
            concentration: 80, // GK reflexes/command
            heading: 80,       // Aerial ability
            jumping: 75,       // Aerial ability
            positioning: 75,   // Command of area
            anticipation: 75,  // Command of area
            teamwork: 70,      // Communication
            leadership: 70,    // Communication
            long_throws: 65,   // GK kicking
            passing: 60,       // GK kicking
            agility: 75,
            strength: 70,
            // Other attributes default
            ..PlayerAttributes::default()
        };

        let hexagon = HexagonCalculator::calculate_goalkeeper_hexagon(&gk_stats);

        // GK should have different stat distribution
        assert!(hexagon.technical > 0, "GK technical (handling → first_touch) should be positive");
        assert!(hexagon.defending > 0, "GK defending (command → positioning) should be positive");

        // All values should be capped at 20
        assert!(hexagon.pace <= 20);
        assert!(hexagon.power <= 20);
        assert!(hexagon.technical <= 20);
    }

    #[test]
    fn test_individual_calculations() {
        let stats = create_test_attributes();

        let pace = HexagonCalculator::calculate_pace(&stats);
        let power = HexagonCalculator::calculate_power(&stats);
        let technical = HexagonCalculator::calculate_technical(&stats);
        let shooting = HexagonCalculator::calculate_shooting(&stats);
        let passing = HexagonCalculator::calculate_passing(&stats);
        let defending = HexagonCalculator::calculate_defending(&stats);

        assert!(technical > power, "Technical should be higher than power in test data");
        assert!(pace > 0, "Pace should be positive");
        assert!(shooting > 0, "Shooting should be positive");
        assert!(passing > 0, "Passing should be positive");
        assert!(defending > 0, "Defending should be positive");
    }

    #[test]
    fn test_contributors() {
        let pace_contributors = HexagonCalculator::get_pace_contributors();
        assert!(pace_contributors.contains(&"pace"));
        assert!(pace_contributors.contains(&"acceleration"));

        let gk_technical = HexagonCalculator::get_gk_contributors("technical");
        assert!(gk_technical.contains(&"first_touch")); // GK handling/reflexes → first_touch
        assert!(gk_technical.contains(&"concentration")); // GK reflexes → concentration
    }

    #[test]
    fn test_hexagon_comparison() {
        let stats1 = HexagonStats {
            pace: 15,
            power: 12,
            technical: 18,
            shooting: 10,
            passing: 14,
            defending: 8,
        };

        let stats2 = HexagonStats {
            pace: 12,
            power: 15,
            technical: 16,
            shooting: 12,
            passing: 16,
            defending: 10,
        };

        let diff = HexagonCalculator::compare_hexagon_stats(&stats1, &stats2);

        assert_eq!(diff.pace_diff, 3); // 15 - 12
        assert_eq!(diff.power_diff, -3); // 12 - 15
        assert_eq!(diff.technical_diff, 2); // 18 - 16

        let (strength_stat, strength_value) = diff.biggest_strength();
        assert_eq!(strength_stat, "pace");
        assert_eq!(strength_value, 3);

        let (weakness_stat, weakness_value) = diff.biggest_weakness();
        assert_eq!(weakness_stat, "power");
        assert_eq!(weakness_value, -3);
    }

    #[test]
    fn test_hexagon_stats_total() {
        let hexagon = HexagonStats {
            pace: 15,
            power: 12,
            technical: 18,
            shooting: 10,
            passing: 14,
            defending: 8,
        };

        assert_eq!(hexagon.total(), 77);

        let array = hexagon.as_array();
        assert_eq!(array, [15, 12, 18, 10, 14, 8]);
    }
}
