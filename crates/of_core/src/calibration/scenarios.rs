//! Test Scenarios - GRF Academy Style Micro-Tests
//!
//! FIX_2601/0112: Academy-style scenario testing for bug reproduction and regression.
//!
//! Core philosophy from GRF:
//! - Full matches are too big → bugs hide
//! - Break football into micro-games (10-30 seconds)
//! - Fast, repeatable, deterministic

use std::collections::HashMap;

/// Success condition for a scenario
#[derive(Debug, Clone)]
pub enum SuccessCondition {
    /// Must score within duration
    Score,
    /// Ball must progress beyond X coordinate (in meters)
    ProgressBeyond(f32),
    /// Must maintain possession for N ticks
    MaintainPossession(u64),
    /// Must prevent opponent from scoring
    PreventScore,
    /// Specific event must occur
    MustHaveEvent(String),
    /// Specific event must NOT occur
    MustNotHaveEvent(String),
    /// Metric must be in range
    MetricInRange {
        metric: String,
        min: Option<f32>,
        max: Option<f32>,
    },
}

/// Player setup for a scenario
#[derive(Debug, Clone)]
pub struct ScenarioPlayer {
    /// Slot index (0-10 for home, 11-21 for away)
    pub slot: usize,
    /// Position in meters
    pub position_m: (f32, f32),
    /// Position role (GK, CB, CM, ST, etc.)
    pub position_role: String,
}

/// Ball setup for a scenario
#[derive(Debug, Clone)]
pub struct ScenarioBall {
    /// Position in meters
    pub position_m: (f32, f32),
    /// Owning team (None for loose ball)
    pub owner_team: Option<bool>, // true = home, false = away
    /// Owning slot within team (0-10)
    pub owner_slot: Option<usize>,
}

/// Restart mode for scenario
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestartMode {
    /// Normal play
    Play,
    /// Kickoff
    Kickoff,
    /// Corner kick
    Corner,
    /// Free kick
    FreeKick,
    /// Goal kick
    GoalKick,
    /// Throw in
    ThrowIn,
}

/// Scenario setup configuration
#[derive(Debug, Clone)]
pub struct ScenarioSetup {
    /// Ball initial state
    pub ball: ScenarioBall,
    /// Home team players
    pub home_players: Vec<ScenarioPlayer>,
    /// Away team players
    pub away_players: Vec<ScenarioPlayer>,
    /// Restart mode
    pub restart_mode: RestartMode,
    /// Which direction home team attacks (true = right, false = left)
    pub attacks_right: bool,
    /// Maximum duration in ticks
    pub max_ticks: u64,
    /// Success conditions
    pub success_conditions: Vec<SuccessCondition>,
}

impl Default for ScenarioSetup {
    fn default() -> Self {
        Self {
            ball: ScenarioBall {
                position_m: (52.5, 34.0),
                owner_team: Some(true),
                owner_slot: Some(6),
            },
            home_players: Vec::new(),
            away_players: Vec::new(),
            restart_mode: RestartMode::Play,
            attacks_right: true,
            max_ticks: 120, // 30 seconds at 4 ticks/sec
            success_conditions: Vec::new(),
        }
    }
}

/// Test scenario definition
#[derive(Debug, Clone)]
pub struct TestScenario {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Scenario setup
    pub setup: ScenarioSetup,
    /// Probes to collect
    pub probes: Vec<String>,
}

/// Predefined test scenarios
impl TestScenario {
    /// Offside trap line step - tests offside symmetry
    pub fn offside_trap_line_step() -> Self {
        let mut setup = ScenarioSetup::default();
        setup.ball = ScenarioBall {
            position_m: (52.5, 34.0),
            owner_team: Some(true),
            owner_slot: Some(6), // CM
        };
        setup.home_players = vec![
            ScenarioPlayer { slot: 0, position_m: (5.0, 34.0), position_role: "GK".into() },
            ScenarioPlayer { slot: 6, position_m: (50.0, 34.0), position_role: "CM".into() },
            ScenarioPlayer { slot: 9, position_m: (75.0, 28.0), position_role: "ST".into() },
            ScenarioPlayer { slot: 10, position_m: (76.0, 40.0), position_role: "ST".into() },
        ];
        setup.away_players = vec![
            ScenarioPlayer { slot: 0, position_m: (100.0, 34.0), position_role: "GK".into() },
            ScenarioPlayer { slot: 3, position_m: (72.0, 25.0), position_role: "CB".into() },
            ScenarioPlayer { slot: 4, position_m: (72.0, 34.0), position_role: "CB".into() },
            ScenarioPlayer { slot: 5, position_m: (72.0, 43.0), position_role: "CB".into() },
        ];
        setup.max_ticks = 60; // 15 seconds

        Self {
            id: "offside_trap_line_step".into(),
            name: "Offside Trap - Line Step".into(),
            description: "Tests offside detection symmetry with high defensive line".into(),
            setup,
            probes: vec![
                "offsides".into(),
                "passes".into(),
                "progressive_passes".into(),
                "shots".into(),
            ],
        }
    }

    /// Counter attack 3v2 - tests forward pass flow
    pub fn counterattack_3v2() -> Self {
        let mut setup = ScenarioSetup::default();
        setup.ball = ScenarioBall {
            position_m: (52.5, 34.0),
            owner_team: Some(true),
            owner_slot: Some(7), // CM
        };
        setup.home_players = vec![
            ScenarioPlayer { slot: 7, position_m: (52.5, 34.0), position_role: "CM".into() },
            ScenarioPlayer { slot: 9, position_m: (65.0, 25.0), position_role: "ST".into() },
            ScenarioPlayer { slot: 10, position_m: (65.0, 43.0), position_role: "ST".into() },
        ];
        setup.away_players = vec![
            ScenarioPlayer { slot: 0, position_m: (100.0, 34.0), position_role: "GK".into() },
            ScenarioPlayer { slot: 4, position_m: (75.0, 30.0), position_role: "CB".into() },
            ScenarioPlayer { slot: 5, position_m: (75.0, 38.0), position_role: "CB".into() },
        ];
        setup.max_ticks = 40; // 10 seconds
        setup.success_conditions = vec![
            SuccessCondition::MustHaveEvent("shot".into()),
        ];

        Self {
            id: "counterattack_3v2".into(),
            name: "Counter Attack 3v2".into(),
            description: "Tests fast break finishing with numerical advantage".into(),
            setup,
            probes: vec![
                "progressive_passes".into(),
                "key_passes".into(),
                "shots".into(),
                "goals".into(),
            ],
        }
    }

    /// Buildout under press - tests safe play under pressure
    pub fn buildout_under_press() -> Self {
        let mut setup = ScenarioSetup::default();
        setup.ball = ScenarioBall {
            position_m: (20.0, 34.0),
            owner_team: Some(true),
            owner_slot: Some(3), // CB
        };
        setup.home_players = vec![
            ScenarioPlayer { slot: 0, position_m: (5.0, 34.0), position_role: "GK".into() },
            ScenarioPlayer { slot: 3, position_m: (20.0, 25.0), position_role: "CB".into() },
            ScenarioPlayer { slot: 4, position_m: (20.0, 43.0), position_role: "CB".into() },
            ScenarioPlayer { slot: 6, position_m: (35.0, 34.0), position_role: "CM".into() },
        ];
        setup.away_players = vec![
            ScenarioPlayer { slot: 9, position_m: (25.0, 30.0), position_role: "ST".into() },
            ScenarioPlayer { slot: 10, position_m: (25.0, 38.0), position_role: "ST".into() },
            ScenarioPlayer { slot: 7, position_m: (40.0, 25.0), position_role: "CM".into() },
            ScenarioPlayer { slot: 8, position_m: (40.0, 43.0), position_role: "CM".into() },
        ];
        setup.max_ticks = 80; // 20 seconds
        setup.success_conditions = vec![
            SuccessCondition::ProgressBeyond(52.5),
        ];

        Self {
            id: "buildout_under_press".into(),
            name: "Buildout Under Press".into(),
            description: "Tests safe buildup from own third under high press".into(),
            setup,
            probes: vec![
                "long_passes".into(),
                "turnovers".into(),
                "progressive_passes".into(),
            ],
        }
    }

    /// Wing cross to box - tests crossing mechanics
    pub fn wing_cross_to_box() -> Self {
        let mut setup = ScenarioSetup::default();
        setup.ball = ScenarioBall {
            position_m: (85.0, 5.0),
            owner_team: Some(true),
            owner_slot: Some(8), // RW
        };
        setup.home_players = vec![
            ScenarioPlayer { slot: 8, position_m: (85.0, 5.0), position_role: "RW".into() },
            ScenarioPlayer { slot: 9, position_m: (88.0, 30.0), position_role: "ST".into() },
            ScenarioPlayer { slot: 10, position_m: (88.0, 40.0), position_role: "ST".into() },
        ];
        setup.away_players = vec![
            ScenarioPlayer { slot: 0, position_m: (100.0, 34.0), position_role: "GK".into() },
            ScenarioPlayer { slot: 3, position_m: (92.0, 28.0), position_role: "CB".into() },
            ScenarioPlayer { slot: 4, position_m: (92.0, 40.0), position_role: "CB".into() },
        ];
        setup.max_ticks = 40; // 10 seconds
        setup.success_conditions = vec![
            SuccessCondition::MustHaveEvent("cross".into()),
        ];

        Self {
            id: "wing_cross_to_box".into(),
            name: "Wing Cross to Box".into(),
            description: "Tests crossing from wide positions into the box".into(),
            setup,
            probes: vec![
                "crosses".into(),
                "headers".into(),
                "shots".into(),
            ],
        }
    }

    /// Own third buildup - FIX_2601/0110 regression test
    pub fn own_third_buildup() -> Self {
        let mut setup = ScenarioSetup::default();
        setup.ball = ScenarioBall {
            position_m: (20.0, 34.0),
            owner_team: Some(true),
            owner_slot: Some(3),
        };
        setup.max_ticks = 100; // 25 seconds
        setup.success_conditions = vec![
            SuccessCondition::ProgressBeyond(52.5), // Must reach midfield
        ];

        Self {
            id: "own_third_buildup".into(),
            name: "Own Third Buildup (FIX_2601/0110)".into(),
            description: "Regression test: ball should progress from own third, not cycle".into(),
            setup,
            probes: vec![
                "progressive_passes".into(),
                "backward_passes".into(),
                "lateral_passes".into(),
            ],
        }
    }

    /// Get all predefined scenarios
    pub fn all() -> Vec<Self> {
        vec![
            Self::offside_trap_line_step(),
            Self::counterattack_3v2(),
            Self::buildout_under_press(),
            Self::wing_cross_to_box(),
            Self::own_third_buildup(),
        ]
    }
}

/// Result from running a scenario
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// Scenario ID
    pub scenario_id: String,
    /// Whether all conditions passed
    pub passed: bool,
    /// Individual condition results
    pub condition_results: Vec<(SuccessCondition, bool)>,
    /// Collected probe values
    pub probe_values: HashMap<String, f32>,
    /// Final ball position
    pub final_ball_position_m: (f32, f32),
    /// Duration in ticks
    pub duration_ticks: u64,
}

/// Scenario runner for symmetry testing
#[derive(Debug, Clone)]
pub struct SymmetryVariant {
    /// Flip attacks_right
    pub flip_direction: bool,
    /// Swap home/away
    pub swap_teams: bool,
}

impl SymmetryVariant {
    /// All 4 variants for symmetry testing
    pub fn all() -> Vec<Self> {
        vec![
            Self { flip_direction: false, swap_teams: false },
            Self { flip_direction: true, swap_teams: false },
            Self { flip_direction: false, swap_teams: true },
            Self { flip_direction: true, swap_teams: true },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_creation() {
        let scenario = TestScenario::offside_trap_line_step();
        assert_eq!(scenario.id, "offside_trap_line_step");
        assert!(!scenario.setup.home_players.is_empty());
        assert!(!scenario.setup.away_players.is_empty());
    }

    #[test]
    fn test_all_scenarios() {
        let scenarios = TestScenario::all();
        assert!(scenarios.len() >= 5);

        for scenario in &scenarios {
            assert!(!scenario.id.is_empty());
            assert!(!scenario.name.is_empty());
        }
    }

    #[test]
    fn test_symmetry_variants() {
        let variants = SymmetryVariant::all();
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_own_third_buildup_scenario() {
        let scenario = TestScenario::own_third_buildup();
        assert_eq!(scenario.id, "own_third_buildup");

        // Must have progress condition
        let has_progress = scenario.setup.success_conditions.iter().any(|c| {
            matches!(c, SuccessCondition::ProgressBeyond(x) if *x > 50.0)
        });
        assert!(has_progress, "Own third buildup must require progression");
    }
}
