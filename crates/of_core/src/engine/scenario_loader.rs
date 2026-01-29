use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use super::audit_gates::{
    BALL_X_MAX, BALL_X_MIN, BALL_Y_MAX, BALL_Y_MIN, PLAYER_X_MAX, PLAYER_X_MIN, PLAYER_Y_MAX,
    PLAYER_Y_MIN,
};
use super::match_sim::{MatchEngine, MatchPlan};
use super::types::coord10::{Coord10, Vel10};
use crate::engine::action_queue::{ActionResult, RestartType};
use crate::models::player::{Player, PlayerAttributes, Position};
use crate::models::team::{Formation, Team};
use crate::models::{EventType, TraitSlots};
use crate::player::PersonalityArchetype;
use crate::tactics::AIDifficulty;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioSide {
    Home,
    Away,
}

impl ScenarioSide {
    pub fn is_home(self) -> bool {
        matches!(self, ScenarioSide::Home)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioPlayer {
    pub role: Position,
    pub pos_m: [f32; 2],
    #[serde(default)]
    pub slot: Option<u8>,
    #[serde(default)]
    pub lazy: bool,
    #[serde(default)]
    pub overall: Option<u8>,
    #[serde(default)]
    pub attributes: Option<HashMap<String, u8>>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioTeam {
    pub side: ScenarioSide,
    #[serde(default)]
    pub difficulty: Option<f32>,
    #[serde(default)]
    pub formation: Option<Formation>,
    #[serde(default)]
    pub players: Vec<ScenarioPlayer>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioBallStateKind {
    Loose,
    Controlled,
    InFlight,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioStartMode {
    Normal,
    KickOff,
    GoalKick,
    FreeKick,
    Corner,
    ThrowIn,
    Penalty,
    DropBall,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioPlayerRef {
    pub side: ScenarioSide,
    pub slot: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioBall {
    pub pos_m: [f32; 3],
    #[serde(default)]
    pub vel_mps: Option<[f32; 3]>,
    #[serde(default)]
    pub owner: Option<ScenarioPlayerRef>,
    #[serde(default)]
    pub state: Option<ScenarioBallStateKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioAction {
    ForcePass {
        from: ScenarioPlayerRef,
        to: ScenarioPlayerRef,
        #[serde(default)]
        success: Option<bool>,
    },
    ForceOutOfPlay {
        restart_type: ScenarioStartMode,
        position_m: [f32; 2],
        last_touch: ScenarioSide,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioAssertion {
    pub event: EventType,
    #[serde(default)]
    pub team: Option<ScenarioSide>,
    #[serde(default)]
    pub count_min: Option<u32>,
    #[serde(default)]
    pub count_max: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioStateAssertion {
    #[serde(default)]
    pub ball_owner: Option<Option<ScenarioPlayerRef>>,
    #[serde(default)]
    pub ball_pos_m: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioSpec {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub seed: u64,
    #[serde(default)]
    pub deterministic: bool,
    #[serde(default)]
    pub game_duration_s: Option<u32>,
    #[serde(default)]
    pub second_half_s: Option<u32>,
    #[serde(default)]
    pub start_mode: Option<ScenarioStartMode>,
    #[serde(default)]
    pub start_team: Option<ScenarioSide>,
    #[serde(default)]
    pub simulate_ticks: Option<u32>,
    #[serde(default)]
    pub home_attacks_right: Option<bool>,
    pub teams: Vec<ScenarioTeam>,
    #[serde(default)]
    pub ball: Option<ScenarioBall>,
    #[serde(default)]
    pub actions: Vec<ScenarioAction>,
    #[serde(default)]
    pub assertions: Vec<ScenarioAssertion>,
    #[serde(default)]
    pub state_assertions: Vec<ScenarioStateAssertion>,
}

#[derive(Debug, Clone)]
pub struct ScenarioOverrides {
    pub player_positions: Vec<(usize, Coord10)>,
    pub ball: Option<ScenarioBallState>,
    pub home_attacks_right: Option<bool>,
    pub lazy_players: Vec<usize>,
    pub start_mode: Option<ScenarioStartMode>,
    pub start_team: Option<ScenarioSide>,
}

#[derive(Debug, Clone)]
pub struct ScenarioBallState {
    pub position: Coord10,
    pub height: i16,
    pub velocity: Vel10,
    pub velocity_z: i16,
    pub owner: Option<usize>,
    pub state: ScenarioBallStateKind,
}

#[derive(Debug, Clone)]
pub struct ScenarioPlan {
    pub plan: MatchPlan,
    pub overrides: ScenarioOverrides,
    pub actions: Vec<ScenarioAction>,
    pub assertions: Vec<ScenarioAssertion>,
    pub state_assertions: Vec<ScenarioStateAssertion>,
    pub simulate_ticks: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ScenarioReport {
    pub events_by_type: HashMap<EventType, usize>,
    pub assertion_failures: Vec<String>,
}

pub struct ScenarioRunResult {
    pub report: ScenarioReport,
    pub engine: MatchEngine,
}

impl ScenarioSpec {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .map_err(|err| format!("Failed to read scenario {}: {}", path.display(), err))?;
        serde_json::from_str(&raw)
            .map_err(|err| format!("Failed to parse scenario {}: {}", path.display(), err))
    }

    pub fn to_plan(&self) -> Result<ScenarioPlan, String> {
        let (home_spec, away_spec) = split_team_specs(&self.teams)?;
        let (home_team, home_positions, home_lazy) = build_team_from_spec(home_spec, "Home")?;
        let (away_team, away_positions, away_lazy) = build_team_from_spec(away_spec, "Away")?;

        let plan = MatchPlan {
            home_team,
            away_team,
            seed: self.seed,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: home_spec.difficulty.map(map_ai_difficulty),
            away_ai_difficulty: away_spec.difficulty.map(map_ai_difficulty),
        };

        let mut player_positions = Vec::new();
        player_positions.extend(home_positions.iter().map(|(slot, pos)| (*slot as usize, *pos)));
        player_positions
            .extend(away_positions.iter().map(|(slot, pos)| (11 + *slot as usize, *pos)));

        let ball_state = match self.ball.as_ref() {
            Some(ball) => Some(build_ball_state(ball)?),
            None => None,
        };
        let mut lazy_players: Vec<usize> = Vec::new();
        lazy_players.extend(home_lazy.iter().map(|slot| *slot as usize));
        lazy_players.extend(away_lazy.iter().map(|slot| 11 + *slot as usize));

        Ok(ScenarioPlan {
            plan,
            overrides: ScenarioOverrides {
                player_positions,
                ball: ball_state,
                home_attacks_right: self.home_attacks_right,
                lazy_players,
                start_mode: self.start_mode,
                start_team: self.start_team,
            },
            actions: self.actions.clone(),
            assertions: self.assertions.clone(),
            state_assertions: self.state_assertions.clone(),
            simulate_ticks: self.simulate_ticks,
        })
    }
}

pub fn run_scenario(spec: &ScenarioSpec) -> Result<ScenarioReport, String> {
    let (report, _engine) = run_scenario_internal(spec)?;
    Ok(report)
}

pub fn run_scenario_with_engine(spec: &ScenarioSpec) -> Result<ScenarioRunResult, String> {
    let (report, engine) = run_scenario_internal(spec)?;
    Ok(ScenarioRunResult { report, engine })
}

fn run_scenario_internal(spec: &ScenarioSpec) -> Result<(ScenarioReport, MatchEngine), String> {
    const POS_TOLERANCE_M: f32 = 0.1;
    let ScenarioPlan { plan, overrides, actions, assertions, state_assertions, simulate_ticks } =
        spec.to_plan()?;
    let mut engine = MatchEngine::new(plan)?;
    let (home_strength, away_strength, possession_ratio, match_duration) = engine.init();
    engine.apply_scenario_overrides(&overrides);

    for action in actions {
        match action {
            ScenarioAction::ForcePass { from, to, success } => {
                let from_idx = player_ref_to_track_id(&from)?;
                let to_idx = player_ref_to_track_id(&to)?;
                let is_home = from.side.is_home();
                engine.execute_direct_pass_to_with_override(from_idx, to_idx, is_home, success);
            }
            ScenarioAction::ForceOutOfPlay { restart_type, position_m, last_touch } => {
                let restart_type_enum = match restart_type {
                    ScenarioStartMode::KickOff => RestartType::KickOff,
                    ScenarioStartMode::GoalKick => RestartType::GoalKick,
                    ScenarioStartMode::FreeKick => RestartType::FreeKick,
                    ScenarioStartMode::Corner => RestartType::Corner,
                    ScenarioStartMode::ThrowIn => RestartType::ThrowIn,
                    ScenarioStartMode::Penalty => RestartType::Penalty,
                    ScenarioStartMode::DropBall => RestartType::DropBall,
                    ScenarioStartMode::Normal => {
                        return Err(
                            "Scenario action force_out_of_play cannot use restart_type=normal"
                                .to_string(),
                        );
                    }
                };
                let position = coord10_from_meters_checked(
                    [position_m[0], position_m[1]],
                    "out_of_play",
                    BALL_X_MIN,
                    BALL_X_MAX,
                    BALL_Y_MIN,
                    BALL_Y_MAX,
                )?;
                // For corner/throw-in/goal-kick: the team that did NOT touch last takes the restart
                // last_touch indicates who touched last, so we invert it
                let restart_team_is_home = !last_touch.is_home();
                engine.handle_action_result(ActionResult::OutOfBounds {
                    restart_type: restart_type_enum,
                    position,
                    home_team: restart_team_is_home,
                });
            }
        }
    }

    if let Some(ticks) = simulate_ticks {
        for _ in 0..ticks {
            if !engine.step_decision_tick_streaming(
                home_strength,
                away_strength,
                possession_ratio,
                match_duration,
            ) {
                break;
            }
        }
    }

    let result = engine.get_result();
    let mut events_by_type: HashMap<EventType, usize> = HashMap::new();
    for event in &result.events {
        *events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
    }

    let mut assertion_failures = Vec::new();
    for assertion in assertions {
        let min = assertion.count_min.unwrap_or(0);
        let max = assertion.count_max.unwrap_or(u32::MAX);
        let count = result
            .events
            .iter()
            .filter(|event| event.event_type == assertion.event)
            .filter(|event| match assertion.team {
                Some(side) => event.is_home_team == side.is_home(),
                None => true,
            })
            .count() as u32;

        if count < min || count > max {
            assertion_failures.push(format!(
                "Event {:?} count {} outside [{}, {}]",
                assertion.event, count, min, max
            ));
        }
    }

    let (ball_norm, ball_height_m) = engine.get_ball_state();
    let ball_pos_m = crate::engine::coordinates::to_meters_clamped(ball_norm);
    let ball_owner = engine.get_ball_owner();
    for assertion in state_assertions {
        if let Some(expected_owner) = assertion.ball_owner {
            match expected_owner {
                Some(owner_ref) => {
                    let expected_idx = player_ref_to_track_id(&owner_ref)?;
                    if ball_owner != Some(expected_idx) {
                        assertion_failures.push(format!(
                            "Ball owner {:?} did not match expected {:?}",
                            ball_owner, expected_idx
                        ));
                    }
                }
                None => {
                    if ball_owner.is_some() {
                        assertion_failures.push(format!(
                            "Ball owner {:?} did not match expected None",
                            ball_owner
                        ));
                    }
                }
            }
        }

        if let Some(expected_pos) = assertion.ball_pos_m {
            let dx = (ball_pos_m.0 - expected_pos[0]).abs();
            let dy = (ball_pos_m.1 - expected_pos[1]).abs();
            let dz = (ball_height_m - expected_pos[2]).abs();
            if dx > POS_TOLERANCE_M || dy > POS_TOLERANCE_M || dz > POS_TOLERANCE_M {
                assertion_failures.push(format!(
                    "Ball position ({:.2},{:.2},{:.2}) outside tolerance (expected {:.2},{:.2},{:.2})",
                    ball_pos_m.0,
                    ball_pos_m.1,
                    ball_height_m,
                    expected_pos[0],
                    expected_pos[1],
                    expected_pos[2]
                ));
            }
        }
    }

    Ok((ScenarioReport { events_by_type, assertion_failures }, engine))
}

fn split_team_specs(teams: &[ScenarioTeam]) -> Result<(&ScenarioTeam, &ScenarioTeam), String> {
    let mut home = None;
    let mut away = None;
    for team in teams {
        match team.side {
            ScenarioSide::Home => {
                if home.is_some() {
                    return Err("Scenario has multiple Home teams".to_string());
                }
                home = Some(team);
            }
            ScenarioSide::Away => {
                if away.is_some() {
                    return Err("Scenario has multiple Away teams".to_string());
                }
                away = Some(team);
            }
        }
    }

    match (home, away) {
        (Some(home_spec), Some(away_spec)) => Ok((home_spec, away_spec)),
        _ => Err("Scenario must contain one Home and one Away team".to_string()),
    }
}

fn build_team_from_spec(
    spec: &ScenarioTeam,
    default_name: &str,
) -> Result<(Team, Vec<(u8, Coord10)>, Vec<u8>), String> {
    let formation = spec.formation.clone().unwrap_or(Formation::F442);
    let mut slots: Vec<Option<Player>> = vec![None; 11];
    let mut positions: Vec<(u8, Coord10)> = Vec::new();
    let mut lazy_slots: Vec<u8> = Vec::new();
    let mut next_slot = 0u8;
    let mut gk_seen = false;

    for player in &spec.players {
        let slot = match player.slot {
            Some(slot) => slot,
            None => {
                while next_slot < 11 && slots[next_slot as usize].is_some() {
                    next_slot += 1;
                }
                let assigned = next_slot;
                next_slot += 1;
                assigned
            }
        };

        if slot >= 11 {
            return Err(format!("Scenario player slot {} is out of range", slot));
        }
        if slots[slot as usize].is_some() {
            return Err(format!("Scenario player slot {} is duplicated", slot));
        }
        if player.role == Position::GK && slot != 0 {
            return Err("Scenario GK must use slot 0".to_string());
        }
        if player.role == Position::GK && gk_seen {
            return Err("Scenario has multiple goalkeepers".to_string());
        }
        if player.role == Position::GK {
            gk_seen = true;
        }
        if player.lazy {
            lazy_slots.push(slot);
        }

        let name = player.name.clone().unwrap_or_else(|| format!("{} P{}", default_name, slot));
        let overall = player.overall.unwrap_or(80);
        slots[slot as usize] =
            Some(make_player(&name, player.role, overall, player.attributes.as_ref())?);
        let pos = coord10_from_meters_checked(
            player.pos_m,
            &format!("{} player slot {}", default_name, slot),
            PLAYER_X_MIN,
            PLAYER_X_MAX,
            PLAYER_Y_MIN,
            PLAYER_Y_MAX,
        )?;
        positions.push((slot, pos));
    }

    if slots[0].is_none() {
        slots[0] = Some(make_player(&format!("{} GK", default_name), Position::GK, 80, None)?);
    }

    for slot in 0..11 {
        if slots[slot].is_none() {
            slots[slot] =
                Some(make_player(&format!("{} P{}", default_name, slot), Position::CM, 80, None)?);
        }
    }

    let players: Vec<Player> = slots.into_iter().map(|p| p.expect("slot filled")).collect();
    Ok((Team { name: default_name.to_string(), formation, players }, positions, lazy_slots))
}

fn make_player(
    name: &str,
    position: Position,
    overall: u8,
    overrides: Option<&HashMap<String, u8>>,
) -> Result<Player, String> {
    let mut attrs = PlayerAttributes::from_uniform(overall);
    if let Some(overrides) = overrides {
        apply_attribute_overrides(&mut attrs, overrides)?;
    }
    Ok(Player {
        name: name.to_string(),
        position,
        overall,
        condition: 3,
        attributes: Some(attrs),
        equipped_skills: Vec::new(),
        traits: TraitSlots::default(),
        personality: PersonalityArchetype::default(),
    })
}

fn apply_attribute_overrides(
    attrs: &mut PlayerAttributes,
    overrides: &HashMap<String, u8>,
) -> Result<(), String> {
    for (key, value) in overrides {
        let v = (*value).clamp(1, 100);
        match key.as_str() {
            "corners" => attrs.corners = v,
            "crossing" => attrs.crossing = v,
            "dribbling" => attrs.dribbling = v,
            "finishing" => attrs.finishing = v,
            "first_touch" => attrs.first_touch = v,
            "free_kicks" => attrs.free_kicks = v,
            "heading" => attrs.heading = v,
            "long_shots" => attrs.long_shots = v,
            "long_throws" => attrs.long_throws = v,
            "marking" => attrs.marking = v,
            "passing" => attrs.passing = v,
            "penalty_taking" => attrs.penalty_taking = v,
            "tackling" => attrs.tackling = v,
            "technique" => attrs.technique = v,

            "aggression" => attrs.aggression = v,
            "anticipation" => attrs.anticipation = v,
            "bravery" => attrs.bravery = v,
            "composure" => attrs.composure = v,
            "concentration" => attrs.concentration = v,
            "decisions" => attrs.decisions = v,
            "determination" => attrs.determination = v,
            "flair" => attrs.flair = v,
            "leadership" => attrs.leadership = v,
            "off_the_ball" => attrs.off_the_ball = v,
            "positioning" => attrs.positioning = v,
            "teamwork" => attrs.teamwork = v,
            "vision" => attrs.vision = v,
            "work_rate" => attrs.work_rate = v,

            "acceleration" => attrs.acceleration = v,
            "agility" => attrs.agility = v,
            "balance" => attrs.balance = v,
            "jumping" => attrs.jumping = v,
            "natural_fitness" => attrs.natural_fitness = v,
            "pace" => attrs.pace = v,
            "stamina" => attrs.stamina = v,
            "strength" => attrs.strength = v,

            _ => return Err(format!("Unknown attribute override: {}", key)),
        }
    }
    Ok(())
}

fn build_ball_state(ball: &ScenarioBall) -> Result<ScenarioBallState, String> {
    let pos = coord10_from_meters_checked(
        [ball.pos_m[0], ball.pos_m[1]],
        "ball",
        BALL_X_MIN,
        BALL_X_MAX,
        BALL_Y_MIN,
        BALL_Y_MAX,
    )?;
    let height_m = ball.pos_m[2];
    if !height_m.is_finite() {
        return Err(format!("Scenario ball height must be finite, got {}", height_m));
    }
    let height = (height_m * 10.0).round() as i16;
    let vel = ball.vel_mps.unwrap_or([0.0, 0.0, 0.0]);
    if !vel[0].is_finite() || !vel[1].is_finite() || !vel[2].is_finite() {
        return Err(format!(
            "Scenario ball velocity must be finite, got ({}, {}, {})",
            vel[0], vel[1], vel[2]
        ));
    }
    let velocity = Vel10::from_mps(vel[0], vel[1]);
    let velocity_z = (vel[2] * 10.0).round() as i16;
    let owner = match ball.owner.as_ref() {
        Some(owner_ref) => Some(player_ref_to_track_id(owner_ref)?),
        None => None,
    };
    let state = ball.state.unwrap_or(ScenarioBallStateKind::Loose);

    Ok(ScenarioBallState { position: pos, height, velocity, velocity_z, owner, state })
}

fn coord10_from_meters_checked(
    pos: [f32; 2],
    label: &str,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
) -> Result<Coord10, String> {
    let x = pos[0];
    let y = pos[1];
    if !x.is_finite() || !y.is_finite() {
        return Err(format!("Scenario {} position must be finite, got ({}, {})", label, x, y));
    }
    if !(x_min..=x_max).contains(&x) || !(y_min..=y_max).contains(&y) {
        return Err(format!(
            "Scenario {} position out of bounds: ({:.2}, {:.2}) allowed x=[{:.1},{:.1}] y=[{:.1},{:.1}]",
            label, x, y, x_min, x_max, y_min, y_max
        ));
    }
    Ok(Coord10::from_meters(x, y))
}

fn player_ref_to_track_id(player: &ScenarioPlayerRef) -> Result<usize, String> {
    if player.slot >= 11 {
        return Err(format!("Scenario player slot {} is out of range", player.slot));
    }
    Ok(if player.side.is_home() { player.slot as usize } else { 11 + player.slot as usize })
}

fn map_ai_difficulty(value: f32) -> AIDifficulty {
    if value >= 0.9 {
        AIDifficulty::Expert
    } else if value >= 0.7 {
        AIDifficulty::Hard
    } else if value >= 0.4 {
        AIDifficulty::Medium
    } else {
        AIDifficulty::Easy
    }
}
