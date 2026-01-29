use serde::Serialize;
use serde_json;
use std::env;
use std::str::FromStr;
use std::time::Instant;

use crate::api::budget::SimBudget;
use crate::api::json_api::{HighlightLevel, MatchRequest, MatchResponse, TeamData};
use super::exp_config_env::apply_exp_config_from_env;
use crate::engine::{MatchEngine, MatchPlan};
use crate::fix01::{error_codes, is_valid_condition_level};
use crate::models::player::PlayerAttributes;
use crate::models::team::Formation;
use crate::models::MatchEvent;
use crate::models::{DeterminismMeta, DeterminismMode, HashAlgorithm, MatchResult, Player, Position, Team};
use serde_json::Value;

fn err_code(code: &str, message: impl std::fmt::Display) -> String {
    format!("{code}: {message}")
}

fn validate_condition_level(level: u8) -> Result<u8, String> {
    if is_valid_condition_level(level) {
        Ok(level)
    } else {
        Err(err_code(
            error_codes::INVALID_CONDITION_RANGE,
            format!("condition must be 1..=5, got {level}"),
        ))
    }
}

/// Budget overflow response
#[derive(Debug, Serialize)]
pub struct BudgetOverflowResponse {
    pub partial: bool,
    pub determinism: DeterminismMeta,
    pub reason: String,
    pub home_score: u32,
    pub away_score: u32,
    pub events: Vec<MatchEvent>,
    pub minutes_simulated: u16,
    pub wall_time_ms: u64,
}

/// Stats-only response for KPI runs (no events payload).
#[derive(Debug, Serialize)]
pub struct StatsOnlyResponse {
    pub schema_version: u8,
    pub determinism: DeterminismMeta,
    pub score_home: u8,
    pub score_away: u8,
    pub statistics: Value,
    /// Optional: DSA summary (authoritative) for QA gating in stats-only runs.
    /// Enabled via `OF_DSA_SUMMARY=1` to avoid slowing down default runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsa_summary: Option<crate::engine::dsa_summary::DsaSummary>,
    pub partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes_simulated: Option<u16>,
    pub wall_time_ms: u64,
}

struct BudgetRunResult {
    result: MatchResult,
    budget_exceeded: bool,
    overflow_reason: String,
    minutes_simulated: u16,
    wall_time_ms: u64,
}

fn env_truthy(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn run_match_with_budget(
    request_json: &str,
    mut budget: SimBudget,
) -> Result<BudgetRunResult, String> {
    let start_time = Instant::now();

    // Parse request
    let request: MatchRequest =
        serde_json::from_str(request_json).map_err(|e| format!("Invalid JSON request: {}", e))?;

    // Validate schema version
    if request.schema_version != 1 {
        return Err(format!("Unsupported schema version: {}", request.schema_version));
    }

    let MatchRequest {
        seed,
        home_team,
        away_team,
        user_player,
        home_instructions,
        away_instructions,
        ..
    } = request;

    // Convert to internal models
    let (home_team, home_player_instructions) = convert_team_internal(home_team)?;
    let (away_team, away_player_instructions) = convert_team_internal(away_team)?;

    // Validate teams
    home_team.validate().map_err(|e| format!("Home team validation failed: {}", e))?;
    away_team.validate().map_err(|e| format!("Away team validation failed: {}", e))?;

    // Create match plan with user config
    let user_player = user_player.map(|up| {
        let is_home = up.team == "home";
        // C6: Resolve player_index from player_name
        let players =
            if is_home { home_team.get_starting_11() } else { away_team.get_starting_11() };
        let base_idx = if is_home { 0 } else { 11 };
        let player_index = players
            .iter()
            .position(|p| p.name == up.player_name)
            .map(|i| base_idx + i)
            .unwrap_or(base_idx + 9); // Fallback to first attacker

        crate::engine::UserPlayerConfig {
            is_home_team: is_home,
            player_name: up.player_name,
            player_index,
            highlight_level: convert_highlight_level(up.highlight_level),
        }
    });

    let plan = MatchPlan {
        home_team,
        away_team,
        seed,
        user_player,
        home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        home_instructions,
        away_instructions,
        home_player_instructions,
        away_player_instructions,
        home_ai_difficulty: None,
        away_ai_difficulty: None,
    };

    // Create engine and initialize
    let mut engine = MatchEngine::new(plan)?;

    // Optional: enable tick-level position tracking for DSA summaries (QA gates)
    // while keeping the JSON response stats-only (no position_data serialization).
    //
    // Guardrail: must not affect match outcomes (telemetry only).
    if env_truthy("OF_DSA_SUMMARY") {
        engine = engine.with_position_tracking();
    }
    apply_exp_config_from_env(&mut engine)?;
    let (home_strength, away_strength, possession_ratio, match_duration) = engine.init();

    // Track simulation progress
    let mut budget_exceeded = false;
    let mut overflow_reason = String::new();

    // Step-based simulation with cooperative budget checking
    loop {
        // Check budget before each minute
        if !budget.tick_minute() {
            budget_exceeded = true;
            overflow_reason =
                budget.get_exceeded_reason().unwrap_or_else(|| "Budget exceeded".to_string());
            break;
        }

        // Check event count
        if engine.event_count() > 0 && !budget.tick_event() {
            budget_exceeded = true;
            overflow_reason =
                budget.get_exceeded_reason().unwrap_or_else(|| "Event budget exceeded".to_string());
            break;
        }

        // Simulate one minute
        let should_continue =
            engine.step(home_strength, away_strength, possession_ratio, match_duration);

        // If match finished naturally, exit
        if !should_continue {
            break;
        }

        // Double-check timeout (in case step() took too long)
        if budget.is_exceeded() {
            budget_exceeded = true;
            overflow_reason = budget
                .get_exceeded_reason()
                .unwrap_or_else(|| "Budget exceeded during simulation".to_string());
            break;
        }
    }

    // Finalize and get result
    let mut result = engine.finalize(possession_ratio);
    let wall_time_ms = start_time.elapsed().as_millis() as u64;
    let (minutes_done, _events_done, _) = budget.get_progress();

    // FIX02: determinism/truncation metadata for budget path.
    result.determinism.mode = if budget_exceeded { DeterminismMode::Truncated } else { DeterminismMode::Budgeted };
    result.determinism.simulated_until_tick = result.statistics.total_ticks;
    result.determinism.cut_reason = if budget_exceeded { Some(overflow_reason.clone()) } else { None };

    Ok(BudgetRunResult {
        result,
        budget_exceeded,
        overflow_reason,
        minutes_simulated: minutes_done,
        wall_time_ms,
    })
}

/// Simulate match with budget constraints using step-based API
pub fn simulate_match_json_budget(
    request_json: &str,
    budget: SimBudget,
) -> Result<String, String> {
    let run = run_match_with_budget(request_json, budget)?;

    // Return appropriate response based on budget status
    if run.budget_exceeded {
        let determinism = DeterminismMeta {
            mode: DeterminismMode::Truncated,
            simulated_until_tick: run.result.statistics.total_ticks,
            cut_reason: Some(run.overflow_reason.clone()),
            hash_algo: HashAlgorithm::FxHash, // FIX_2601/0123
        };
        let overflow_response = BudgetOverflowResponse {
            partial: true,
            determinism,
            reason: run.overflow_reason,
            home_score: run.result.score_home as u32,
            away_score: run.result.score_away as u32,
            events: run.result.events.clone(),
            minutes_simulated: run.minutes_simulated,
            wall_time_ms: run.wall_time_ms,
        };

        serde_json::to_string(&overflow_response)
            .map_err(|e| format!("Failed to serialize overflow response: {}", e))
    } else {
        // Normal response - convert result to JSON response format
        // Convert events and statistics to JSON values
        let events_json =
            run.result.events.iter().map(|e| serde_json::to_value(e).unwrap_or(Value::Null)).collect();
        let statistics_json = serde_json::to_value(&run.result.statistics).unwrap_or(Value::Null);

        let response = MatchResponse {
            schema_version: 1,
            determinism: DeterminismMeta {
                mode: DeterminismMode::Budgeted,
                simulated_until_tick: run.result.statistics.total_ticks,
                cut_reason: None,
                hash_algo: HashAlgorithm::FxHash, // FIX_2601/0123
            },
            score_home: run.result.score_home,
            score_away: run.result.score_away,
            events: events_json,
            statistics: statistics_json,
        };
        serde_json::to_string(&response).map_err(|e| format!("Failed to serialize response: {}", e))
    }
}

/// Simulate match with budget constraints, returning stats-only response.
pub fn simulate_match_json_budget_stats_only(
    request_json: &str,
    budget: SimBudget,
) -> Result<String, String> {
    let run = run_match_with_budget(request_json, budget)?;
    let statistics_json = serde_json::to_value(&run.result.statistics).unwrap_or(Value::Null);

    // Optional DSA summary for runops/CI gating.
    //
    // Only include when enabled AND when we actually recorded any ball samples.
    // (When position tracking is disabled, `position_data.ball` stays empty.)
    let include_dsa = env_truthy("OF_DSA_SUMMARY");
    let has_position_samples = run
        .result
        .position_data
        .as_ref()
        .map(|pos| !pos.ball.is_empty())
        .unwrap_or(false);
    let duration_minutes: u8 = 90; // regulation; extra-time support is vNext
    let dsa_summary = if include_dsa && has_position_samples {
        crate::engine::dsa_summary::analyze_dsa_summary(&run.result, duration_minutes)
    } else {
        None
    };

    let response = StatsOnlyResponse {
        schema_version: 1,
        determinism: DeterminismMeta {
            mode: if run.budget_exceeded { DeterminismMode::Truncated } else { DeterminismMode::Budgeted },
            simulated_until_tick: run.result.statistics.total_ticks,
            cut_reason: if run.budget_exceeded { Some(run.overflow_reason.clone()) } else { None },
            hash_algo: HashAlgorithm::FxHash, // FIX_2601/0123
        },
        score_home: run.result.score_home,
        score_away: run.result.score_away,
        statistics: statistics_json,
        dsa_summary,
        partial: run.budget_exceeded,
        reason: if run.budget_exceeded { Some(run.overflow_reason) } else { None },
        minutes_simulated: if run.budget_exceeded { Some(run.minutes_simulated) } else { None },
        wall_time_ms: run.wall_time_ms,
    };
    serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize stats-only response: {}", e))
}

fn convert_team_internal(
    data: TeamData,
) -> Result<
    (
        Team,
        Option<std::collections::HashMap<String, crate::player::instructions::PlayerInstructions>>,
    ),
    String,
> {
    if data.players.is_empty() {
        return Err("Team must have at least one player".to_string());
    }

    let players: Result<Vec<Player>, String> = data
        .players
        .into_iter()
        .map(|p| {
            let position = Position::from_str(&p.position)?;
            let condition = validate_condition_level(p.condition)?;

            // Create default attributes based on overall rating (OpenFootball 36-field)
            let base = p.overall.saturating_sub(10);
            let attributes = PlayerAttributes {
                // Technical (14) - OpenFootball standard
                corners: base + 3,
                crossing: base + 3,
                dribbling: base + 5,
                finishing: base + 5,
                first_touch: base + 4,
                free_kicks: base + 2,
                heading: base + 3,
                long_shots: base + 3,
                long_throws: base + 2,
                marking: base + 4,
                passing: base + 5,
                penalty_taking: base + 5,
                tackling: base + 4,
                technique: base + 5,

                // Mental (14) - OpenFootball standard
                aggression: base + 3,
                anticipation: base + 4,
                bravery: base + 4,
                composure: base + 4,
                concentration: base + 5,
                decisions: base + 5,
                determination: base + 5,
                flair: base + 2,
                leadership: base + 3,
                off_the_ball: base + 4,
                positioning: base + 5,
                teamwork: base + 5,
                vision: base + 5,
                work_rate: base + 5,

                // Physical (8) - OpenFootball standard
                acceleration: base + 5,
                agility: base + 5,
                balance: base + 4,
                jumping: base + 4,
                natural_fitness: base + 5,
                pace: base + 5,
                stamina: base + 5,
                strength: base + 4,

                // GK attributes - default 0 for budget API (typically outfield)
                gk_aerial_reach: 0,
                gk_command_of_area: 0,
                gk_communication: 0,
                gk_eccentricity: 0,
                gk_handling: 0,
                gk_kicking: 0,
                gk_one_on_ones: 0,
                gk_reflexes: 0,
                gk_rushing_out: 0,
                gk_punching: 0,
                gk_throwing: 0,
            };

            Ok(Player {
                name: p.name,
                position,
                overall: p.overall,
                condition,
                attributes: Some(attributes),
                equipped_skills: Vec::new(),
                traits: Default::default(),
                personality: Default::default(),
            })
        })
        .collect();

    // Parse formation
    let formation = match data.formation.as_str() {
        "4-4-2" => Formation::F442,
        "4-3-3" => Formation::F433,
        "4-4-1-1" => Formation::F4411,
        "4-3-2-1" => Formation::F4321,
        "4-2-2-2" => Formation::F4222,
        "4-5-1" => Formation::F451,
        "3-5-2" => Formation::F352,
        "3-4-2-1" => Formation::F3421,
        "3-4-1-2" => Formation::F3412,
        "4-2-3-1" => Formation::F4231,
        "5-3-2" => Formation::F532,
        "4-1-4-1" => Formation::F4141,
        "3-4-3" => Formation::F343,
        "5-4-1" => Formation::F541,
        other => {
            return Err(err_code(
                error_codes::UNSUPPORTED_FORMATION,
                format!("formation not in allowlist: {other}"),
            ))
        }
    };

    Ok((Team { name: data.name, formation, players: players? }, data.player_instructions))
}

fn convert_highlight_level(level: HighlightLevel) -> crate::engine::HighlightLevel {
    match level {
        HighlightLevel::Skip => crate::engine::HighlightLevel::Skip,
        HighlightLevel::Simple => crate::engine::HighlightLevel::Simple,
        HighlightLevel::MyPlayer => crate::engine::HighlightLevel::MyPlayer,
        HighlightLevel::Full => crate::engine::HighlightLevel::Full,
    }
}
