//! ScenarioRunner - GRF Academy Style Micro-Test Executor
//!
//! FIX_2601/0112 Phase 2: Runs TestScenario definitions using MatchEngine.
//!
//! Key features:
//! - Creates minimal MatchEngine with scenario setup
//! - Runs tick-by-tick simulation up to max_ticks
//! - Evaluates success conditions
//! - Collects probe values for metrics

use std::collections::HashMap;

use super::scenarios::{
    ScenarioPlayer, ScenarioResult, ScenarioSetup,
    SuccessCondition, SymmetryVariant, TestScenario,
};
use crate::engine::match_sim::{MatchEngine, MatchPlan};
use crate::engine::types::Coord10;
use crate::models::player::PlayerAttributes;
use crate::models::team::Formation;
use crate::models::{Player, Position, Team};

/// ScenarioRunner executes TestScenario definitions using the MatchEngine.
pub struct ScenarioRunner {
    /// Random seed for deterministic execution
    seed: u64,
}

impl ScenarioRunner {
    /// Create a new ScenarioRunner with the given seed.
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Run a scenario and return the result.
    pub fn run(&self, scenario: &TestScenario) -> Result<ScenarioResult, String> {
        self.run_with_variant(scenario, None)
    }

    /// Run a scenario with an optional symmetry variant.
    pub fn run_with_variant(
        &self,
        scenario: &TestScenario,
        variant: Option<&SymmetryVariant>,
    ) -> Result<ScenarioResult, String> {
        // Create minimal teams for the scenario
        let (home_team, away_team) = self.create_minimal_teams(&scenario.setup);

        // Build MatchPlan
        let plan = MatchPlan {
            home_team,
            away_team,
            seed: self.seed,
            user_player: None,
            home_match_modifiers: Default::default(),
            away_match_modifiers: Default::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        };

        // Create MatchEngine
        let mut engine = MatchEngine::new(plan)?;

        // Initialize the engine (like simulate() does)
        let (home_strength, away_strength, possession_ratio, _match_duration) = engine.init();

        // Apply scenario overrides
        self.apply_scenario_setup(&mut engine, &scenario.setup, variant);

        // Run simulation tick by tick
        let max_ticks = scenario.setup.max_ticks;
        let mut ticks_elapsed: u64 = 0;
        let mut events_collected: Vec<String> = Vec::new();

        for tick in 0..max_ticks {
            // Run one decision tick
            // Note: We use a short match_duration (1 minute = 240 ticks)
            // The scenario controls termination via max_ticks
            let _ = engine.step_decision_tick_streaming(
                home_strength,
                away_strength,
                possession_ratio,
                10, // 10 minute max (2400 ticks), but we'll terminate earlier
            );

            ticks_elapsed = tick + 1;

            // Collect events from this tick
            self.collect_tick_events(&engine, &mut events_collected);

            // Check for early termination conditions
            if self.check_early_termination(&engine, &scenario.setup.success_conditions) {
                break;
            }
        }

        // Evaluate success conditions
        let condition_results = self.evaluate_conditions(&engine, &scenario.setup.success_conditions);
        let passed = condition_results.iter().all(|(_, success)| *success);

        // Collect probe values
        let probe_values = self.collect_probes(&engine, &scenario.probes);

        // Get final ball position
        let final_ball_position_m = engine.get_ball_position_meters();

        Ok(ScenarioResult {
            scenario_id: scenario.id.clone(),
            passed,
            condition_results,
            probe_values,
            final_ball_position_m,
            duration_ticks: ticks_elapsed,
        })
    }

    /// Create minimal teams with players positioned per scenario setup.
    fn create_minimal_teams(&self, setup: &ScenarioSetup) -> (Team, Team) {
        let home_team = self.create_team_from_scenario_players("ScenarioHome", &setup.home_players, true);
        let away_team = self.create_team_from_scenario_players("ScenarioAway", &setup.away_players, false);
        (home_team, away_team)
    }

    /// Create a team with scenario player definitions.
    fn create_team_from_scenario_players(
        &self,
        name: &str,
        scenario_players: &[ScenarioPlayer],
        is_home: bool,
    ) -> Team {
        let base_overall: u8 = 75;
        let mut players = Vec::new();

        // Create all 11 starting positions with defaults
        let default_positions = [
            Position::GK,
            Position::LB,
            Position::CB,
            Position::CB,
            Position::RB,
            Position::LM,
            Position::CM,
            Position::CM,
            Position::RM,
            Position::ST,
            Position::ST,
        ];

        // Build a map of scenario player overrides
        let mut overrides: HashMap<usize, &ScenarioPlayer> = HashMap::new();
        for sp in scenario_players {
            overrides.insert(sp.slot, sp);
        }

        // Create starting 11
        for (i, &default_pos) in default_positions.iter().enumerate() {
            let pos = if let Some(sp) = overrides.get(&i) {
                self.parse_position_role(&sp.position_role).unwrap_or(default_pos)
            } else {
                default_pos
            };

            let overall = base_overall + (i % 5) as u8 - 2;
            let attrs = self.create_default_attributes(overall, pos);

            players.push(Player {
                name: format!("{} Player {}", name, i + 1),
                position: pos,
                overall,
                condition: 3,
                attributes: Some(attrs),
                equipped_skills: Vec::new(),
                traits: Default::default(),
                personality: Default::default(),
            });
        }

        // Create 7 subs
        let sub_positions = [
            Position::GK,
            Position::CB,
            Position::CM,
            Position::CM,
            Position::ST,
            Position::LW,
            Position::RW,
        ];
        for (i, &pos) in sub_positions.iter().enumerate() {
            let overall = base_overall.saturating_sub(5) + (i % 3) as u8;
            let attrs = self.create_default_attributes(overall, pos);

            players.push(Player {
                name: format!("{} Sub {}", name, i + 1),
                position: pos,
                overall,
                condition: 3,
                attributes: Some(attrs),
                equipped_skills: Vec::new(),
                traits: Default::default(),
                personality: Default::default(),
            });
        }

        Team {
            name: name.to_string(),
            formation: Formation::F442,
            players,
        }
    }

    /// Parse position role string to Position enum.
    fn parse_position_role(&self, role: &str) -> Option<Position> {
        match role.to_uppercase().as_str() {
            "GK" => Some(Position::GK),
            "LB" => Some(Position::LB),
            "CB" => Some(Position::CB),
            "RB" => Some(Position::RB),
            "LM" => Some(Position::LM),
            "CM" | "CDM" | "CAM" => Some(Position::CM),
            "RM" => Some(Position::RM),
            "LW" => Some(Position::LW),
            "RW" => Some(Position::RW),
            "ST" | "CF" => Some(Position::ST),
            _ => None,
        }
    }

    /// Create default attributes for a position.
    fn create_default_attributes(&self, overall: u8, _pos: Position) -> PlayerAttributes {
        let mut attrs = PlayerAttributes::default();

        // Set all attributes to overall-based values
        attrs.corners = overall;
        attrs.crossing = overall;
        attrs.dribbling = overall;
        attrs.finishing = overall;
        attrs.first_touch = overall;
        attrs.free_kicks = overall;
        attrs.heading = overall;
        attrs.long_shots = overall.saturating_sub(5);
        attrs.long_throws = overall;
        attrs.marking = overall;
        attrs.passing = overall;
        attrs.penalty_taking = overall.saturating_sub(5);
        attrs.tackling = overall;
        attrs.technique = overall;
        attrs.aggression = overall;
        attrs.anticipation = overall;
        attrs.bravery = overall;
        attrs.composure = overall;
        attrs.concentration = overall;
        attrs.decisions = overall;
        attrs.determination = overall;
        attrs.flair = overall.saturating_sub(5);
        attrs.leadership = overall.saturating_sub(5);
        attrs.off_the_ball = overall;
        attrs.positioning = overall;
        attrs.teamwork = overall;
        attrs.vision = overall;
        attrs.work_rate = overall;
        attrs.acceleration = overall;
        attrs.agility = overall;
        attrs.balance = overall;
        attrs.jumping = overall;
        attrs.natural_fitness = overall;
        attrs.pace = overall;
        attrs.stamina = overall;
        attrs.strength = overall;

        attrs
    }

    /// Apply scenario setup to the engine.
    fn apply_scenario_setup(
        &self,
        engine: &mut MatchEngine,
        setup: &ScenarioSetup,
        variant: Option<&SymmetryVariant>,
    ) {
        // Determine direction and team swap based on variant
        let flip_direction = variant.map(|v| v.flip_direction).unwrap_or(false);
        let swap_teams = variant.map(|v| v.swap_teams).unwrap_or(false);

        // Calculate effective attacks_right
        let attacks_right = if flip_direction {
            !setup.attacks_right
        } else {
            setup.attacks_right
        };

        // Apply player positions (convert from meters to Coord10)
        let home_players = if swap_teams { &setup.away_players } else { &setup.home_players };
        let away_players = if swap_teams { &setup.home_players } else { &setup.away_players };

        for sp in home_players {
            let pos_m = self.transform_position(sp.position_m, flip_direction);
            let coord = Coord10::from_meters(pos_m.0, pos_m.1);
            engine.set_player_position(sp.slot, coord);
        }

        for sp in away_players {
            let pos_m = self.transform_position(sp.position_m, flip_direction);
            let coord = Coord10::from_meters(pos_m.0, pos_m.1);
            engine.set_player_position(11 + sp.slot, coord);
        }

        // Apply ball position
        let ball = &setup.ball;
        let ball_pos_m = self.transform_position(ball.position_m, flip_direction);
        let ball_coord = Coord10::from_meters(ball_pos_m.0, ball_pos_m.1);
        engine.set_ball_position(ball_coord);

        // Apply ball ownership
        if let Some(owner_team) = ball.owner_team {
            let effective_owner_team = if swap_teams { !owner_team } else { owner_team };
            if let Some(owner_slot) = ball.owner_slot {
                let track_id = if effective_owner_team {
                    owner_slot
                } else {
                    11 + owner_slot
                };
                engine.set_ball_owner(track_id);
            }
        }

        // Apply attack direction
        engine.set_home_attacks_right(attacks_right);
    }

    /// Transform position based on direction flip.
    fn transform_position(&self, pos: (f32, f32), flip: bool) -> (f32, f32) {
        if flip {
            // Flip X coordinate (105m pitch width)
            (105.0 - pos.0, pos.1)
        } else {
            pos
        }
    }

    /// Collect events from the current tick.
    fn collect_tick_events(&self, _engine: &MatchEngine, _events: &mut Vec<String>) {
        // TODO: Extract events from engine.result.events
        // For now, we skip event collection - will be implemented when needed
    }

    /// Check for early termination conditions.
    fn check_early_termination(
        &self,
        engine: &MatchEngine,
        conditions: &[SuccessCondition],
    ) -> bool {
        for condition in conditions {
            match condition {
                SuccessCondition::Score => {
                    // If we scored, we can terminate early
                    if engine.get_home_score() > 0 {
                        return true;
                    }
                }
                SuccessCondition::PreventScore => {
                    // If opponent scored, we failed - terminate
                    if engine.get_away_score() > 0 {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Evaluate all success conditions.
    fn evaluate_conditions(
        &self,
        engine: &MatchEngine,
        conditions: &[SuccessCondition],
    ) -> Vec<(SuccessCondition, bool)> {
        conditions
            .iter()
            .map(|cond| {
                let success = self.evaluate_single_condition(engine, cond);
                (cond.clone(), success)
            })
            .collect()
    }

    /// Evaluate a single success condition.
    fn evaluate_single_condition(
        &self,
        engine: &MatchEngine,
        condition: &SuccessCondition,
    ) -> bool {
        match condition {
            SuccessCondition::Score => engine.get_home_score() > 0,
            SuccessCondition::ProgressBeyond(x) => {
                let ball_pos = engine.get_ball_position_meters();
                ball_pos.0 > *x
            }
            SuccessCondition::MaintainPossession(_ticks) => {
                // TODO: Track possession duration
                true
            }
            SuccessCondition::PreventScore => engine.get_away_score() == 0,
            SuccessCondition::MustHaveEvent(event_name) => {
                // Check if event occurred
                engine.has_event_type(event_name)
            }
            SuccessCondition::MustNotHaveEvent(event_name) => {
                // Check that event did NOT occur
                !engine.has_event_type(event_name)
            }
            SuccessCondition::MetricInRange { metric, min, max } => {
                if let Some(value) = engine.get_metric(metric) {
                    let above_min = min.map(|m| value >= m).unwrap_or(true);
                    let below_max = max.map(|m| value <= m).unwrap_or(true);
                    above_min && below_max
                } else {
                    false
                }
            }
        }
    }

    /// Collect probe values from the engine.
    fn collect_probes(&self, engine: &MatchEngine, probes: &[String]) -> HashMap<String, f32> {
        let mut values = HashMap::new();

        for probe in probes {
            if let Some(value) = engine.get_metric(probe) {
                values.insert(probe.clone(), value);
            }
        }

        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_runner_creation() {
        let runner = ScenarioRunner::new(12345);
        assert_eq!(runner.seed, 12345);
    }

    #[test]
    fn test_transform_position() {
        let runner = ScenarioRunner::new(12345);

        // No flip
        let pos = runner.transform_position((50.0, 34.0), false);
        assert_eq!(pos, (50.0, 34.0));

        // With flip
        let pos = runner.transform_position((50.0, 34.0), true);
        assert_eq!(pos, (55.0, 34.0)); // 105 - 50 = 55
    }

    #[test]
    fn test_parse_position_role() {
        let runner = ScenarioRunner::new(12345);

        assert_eq!(runner.parse_position_role("GK"), Some(Position::GK));
        assert_eq!(runner.parse_position_role("CB"), Some(Position::CB));
        assert_eq!(runner.parse_position_role("ST"), Some(Position::ST));
        assert_eq!(runner.parse_position_role("cm"), Some(Position::CM));
        assert_eq!(runner.parse_position_role("unknown"), None);
    }

    #[test]
    fn test_create_team_from_scenario() {
        let runner = ScenarioRunner::new(12345);

        let scenario_players = vec![
            ScenarioPlayer {
                slot: 0,
                position_m: (5.0, 34.0),
                position_role: "GK".to_string(),
            },
            ScenarioPlayer {
                slot: 9,
                position_m: (75.0, 28.0),
                position_role: "ST".to_string(),
            },
        ];

        let team = runner.create_team_from_scenario_players("Test", &scenario_players, true);
        assert_eq!(team.name, "Test");
        assert_eq!(team.players.len(), 18); // 11 + 7 subs
        assert_eq!(team.players[0].position, Position::GK);
    }
}
