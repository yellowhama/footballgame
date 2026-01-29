use serde::{Deserialize, Serialize};
use serde_json;

use crate::data::resolve_person_by_player_uid;
use crate::engine::{MatchEngine, MatchPlan};
use super::exp_config_env::apply_exp_config_from_env;
use crate::fix01::{error_codes, is_valid_condition_level};
use crate::models::player::PlayerAttributes;
use crate::models::trait_system::{EquippedTrait, TraitId, TraitSlots, TraitTier};
use crate::models::{Player, Team};
use crate::player::instructions::PlayerInstructions;
use crate::player::personality::PersonalityArchetype;
use crate::tactics::ai_profiles::AIDifficulty;
use crate::tactics::team_instructions::TeamInstructions;
use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Deserialize)]
pub struct MatchRequest {
    pub schema_version: u8,
    pub seed: u64,
    pub home_team: TeamData,
    pub away_team: TeamData,
    pub user_player: Option<UserPlayerConfig>,
    #[serde(default)]
    pub home_instructions: Option<TeamInstructions>,
    #[serde(default)]
    pub away_instructions: Option<TeamInstructions>,
    /// Enable position tracking for replay (increases data size ~1.4MB)
    #[serde(default)]
    pub enable_position_tracking: bool,
}

#[derive(Debug, Deserialize)]
pub struct UserPlayerConfig {
    pub team: String, // "home" or "away"
    pub player_name: String,
    pub highlight_level: HighlightLevel,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum HighlightLevel {
    #[serde(rename = "skip")]
    Skip, // 스킵 - 바로 결과로
    #[serde(rename = "simple")]
    Simple, // 간단히 - 골 + 주요 장면
    #[serde(rename = "my_player")]
    MyPlayer, // 내 선수 중심
    #[serde(rename = "full")]
    Full, // 전체 하이라이트
}

#[derive(Debug, Deserialize)]
pub struct TeamData {
    pub name: String,
    pub formation: String,
    pub players: Vec<PlayerData>,
    #[serde(default)]
    pub player_instructions:
        Option<std::collections::HashMap<String, crate::player::instructions::PlayerInstructions>>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerData {
    pub name: String,
    pub position: String,
    pub overall: u8,
    /// FIX01: ConditionLevel (1..=5)
    pub condition: u8,
}

// ============================================================================
// MatchRequest v2 (UID / PlayerLibrary-based) — schema_version = 2
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct MatchRequestV2 {
    pub schema_version: u8,
    pub seed: u64,
    pub home_team: TeamDataV2,
    pub away_team: TeamDataV2,
    pub user_player: Option<UserPlayerConfigV2>,
    #[serde(default)]
    pub home_instructions: Option<TeamInstructions>,
    #[serde(default)]
    pub away_instructions: Option<TeamInstructions>,
    /// Enable position tracking for MatchResult.position_data (increases output size)
    #[serde(default)]
    pub enable_position_tracking: bool,
    /// When true, use real names instead of pseudonyms for Player.name
    #[serde(default)]
    pub use_real_names: bool,
    /// AI difficulty for home team: "Easy" | "Medium" | "Hard" | "Expert"
    #[serde(default)]
    pub home_ai_difficulty: Option<String>,
    /// AI difficulty for away team: "Easy" | "Medium" | "Hard" | "Expert"
    #[serde(default)]
    pub away_ai_difficulty: Option<String>,
}

/// Roster entry: either a UID string or embedded player data
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RosterEntry {
    /// UID reference (e.g. "csv:123")
    Uid(String),
    /// UID reference with FIX01 match-time metadata
    UidWithMeta(UidRosterEntry),
    /// Embedded player data with full attributes
    Embedded(EmbeddedPlayerData),
}

/// UID roster entry with required FIX01 metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct UidRosterEntry {
    pub uid: String,
    /// FIX01: ConditionLevel (1..=5)
    pub condition: u8,
}

/// Embedded player data for JSON v2 roster (MRQ0 v3 compatible)
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddedPlayerData {
    pub name: String,
    pub position: String,
    pub overall: u8,
    /// FIX01: ConditionLevel (1..=5)
    pub condition: u8,
    #[serde(default)]
    pub attributes: Option<EmbeddedPlayerAttributes>,
    #[serde(default)]
    pub track_id: Option<u32>,
    /// Personality archetype: "Leader" | "Genius" | "Workhorse" | "Rebel" | "Steady"
    #[serde(default)]
    pub personality: Option<String>,
    /// Equipped traits (max 4): [{id: "Sniper", tier: "Gold"}, ...]
    #[serde(default)]
    pub traits: Option<Vec<EmbeddedTrait>>,
}

/// Trait entry for embedded player data
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddedTrait {
    /// Trait ID: "Sniper" | "Cannon" | "Finesse" | ... (30 total)
    pub id: String,
    /// Trait tier: "Bronze" | "Silver" | "Gold"
    #[serde(default = "default_bronze")]
    pub tier: String,
}

fn default_bronze() -> String {
    "Bronze".to_string()
}

/// Player attributes for embedded roster entries (36 fields, 0-100 scale)
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddedPlayerAttributes {
    // Technical (14)
    #[serde(default = "default_50")]
    pub corners: u8,
    #[serde(default = "default_50")]
    pub crossing: u8,
    #[serde(default = "default_50")]
    pub dribbling: u8,
    #[serde(default = "default_50")]
    pub finishing: u8,
    #[serde(default = "default_50")]
    pub first_touch: u8,
    #[serde(default = "default_50")]
    pub free_kick_taking: u8,
    #[serde(default = "default_50")]
    pub heading: u8,
    #[serde(default = "default_50")]
    pub long_shots: u8,
    #[serde(default = "default_50")]
    pub long_throws: u8,
    #[serde(default = "default_50")]
    pub marking: u8,
    #[serde(default = "default_50")]
    pub passing: u8,
    #[serde(default = "default_50")]
    pub penalty_taking: u8,
    #[serde(default = "default_50")]
    pub tackling: u8,
    #[serde(default = "default_50")]
    pub technique: u8,
    // Mental (14)
    #[serde(default = "default_50")]
    pub aggression: u8,
    #[serde(default = "default_50")]
    pub anticipation: u8,
    #[serde(default = "default_50")]
    pub bravery: u8,
    #[serde(default = "default_50")]
    pub composure: u8,
    #[serde(default = "default_50")]
    pub concentration: u8,
    #[serde(default = "default_50")]
    pub decisions: u8,
    #[serde(default = "default_50")]
    pub determination: u8,
    #[serde(default = "default_50")]
    pub flair: u8,
    #[serde(default = "default_50")]
    pub leadership: u8,
    #[serde(default = "default_50")]
    pub off_the_ball: u8,
    #[serde(default = "default_50")]
    pub positioning: u8,
    #[serde(default = "default_50")]
    pub teamwork: u8,
    #[serde(default = "default_50")]
    pub vision: u8,
    #[serde(default = "default_50")]
    pub work_rate: u8,
    // Physical (8)
    #[serde(default = "default_50")]
    pub acceleration: u8,
    #[serde(default = "default_50")]
    pub agility: u8,
    #[serde(default = "default_50")]
    pub balance: u8,
    #[serde(default = "default_50")]
    pub jumping_reach: u8,
    #[serde(default = "default_50")]
    pub natural_fitness: u8,
    #[serde(default = "default_50")]
    pub pace: u8,
    #[serde(default = "default_50")]
    pub stamina: u8,
    #[serde(default = "default_50")]
    pub strength: u8,
}

fn default_50() -> u8 {
    50
}

#[derive(Debug, Deserialize)]
pub struct TeamDataV2 {
    pub name: String,
    pub formation: String,
    /// 18 roster entries: either UID strings or embedded player data
    pub roster: Vec<RosterEntry>,
    /// Optional per-roster-slot instructions (slot index 0..17 encoded as JSON object keys)
    #[serde(default)]
    pub player_instructions: Option<HashMap<String, PlayerInstructions>>,
}

#[derive(Debug, Deserialize)]
pub struct UserPlayerConfigV2 {
    pub team: String, // "home" or "away"
    pub highlight_level: HighlightLevel,
    #[serde(default)]
    pub player_uid: Option<String>,
    #[serde(default)]
    pub roster_slot: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct MatchResponse {
    pub schema_version: u8,
    #[serde(default)]
    pub determinism: crate::models::DeterminismMeta,
    pub score_home: u8,
    pub score_away: u8,
    pub events: Vec<serde_json::Value>,
    pub statistics: serde_json::Value,
}

/// Build a `MatchPlan` from a MatchRequest v2 JSON payload (schema_version=2).
///
/// Shared by:
/// - `simulate_match_v2_json*` (batch/replay)
/// - Godot session/streaming entrypoints (Phase23.5)
///
/// Returns:
/// - `MatchPlan`
/// - `enable_position_tracking` request flag
pub fn match_plan_from_match_request_v2_json(
    request_json: &str,
) -> Result<(MatchPlan, bool), String> {
    let request: MatchRequestV2 =
        serde_json::from_str(request_json).map_err(|e| format!("Invalid JSON request: {}", e))?;

    if request.schema_version != 2 {
        return Err(format!("Unsupported schema version: {}", request.schema_version));
    }

    let MatchRequestV2 {
        seed,
        home_team: home_team_data,
        away_team: away_team_data,
        user_player,
        home_instructions,
        away_instructions,
        enable_position_tracking,
        use_real_names,
        home_ai_difficulty,
        away_ai_difficulty,
        ..
    } = request;

    let (home_team, home_uid_to_name, home_player_instructions) =
        convert_team_v2(home_team_data, use_real_names)?;
    let (away_team, away_uid_to_name, away_player_instructions) =
        convert_team_v2(away_team_data, use_real_names)?;

    home_team.validate().map_err(|e| format!("Home team validation failed: {}", e))?;
    away_team.validate().map_err(|e| format!("Away team validation failed: {}", e))?;

    let user_config = user_player
        .map(|up| {
            convert_user_player_v2(up, &home_team, &away_team, &home_uid_to_name, &away_uid_to_name)
        })
        .transpose()?;

    // Parse AI difficulty settings
    let home_ai = parse_ai_difficulty(home_ai_difficulty.as_deref());
    let away_ai = parse_ai_difficulty(away_ai_difficulty.as_deref());

    let plan = MatchPlan {
        home_team,
        away_team,
        seed,
        user_player: user_config,
        home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        home_instructions,
        away_instructions,
        home_player_instructions,
        away_player_instructions,
        home_ai_difficulty: home_ai,
        away_ai_difficulty: away_ai,
    };

    Ok((plan, enable_position_tracking))
}

/// Parse AI difficulty string to enum
fn parse_ai_difficulty(s: Option<&str>) -> Option<AIDifficulty> {
    match s? {
        "Easy" => Some(AIDifficulty::Easy),
        "Medium" => Some(AIDifficulty::Medium),
        "Hard" => Some(AIDifficulty::Hard),
        "Expert" => Some(AIDifficulty::Expert),
        _ => None,
    }
}

/// Parse trait ID string to enum (30 traits total)
fn parse_trait_id(s: &str) -> Option<TraitId> {
    match s {
        // Shooting & Scoring (7)
        "Sniper" => Some(TraitId::Sniper),
        "Cannon" => Some(TraitId::Cannon),
        "Finesse" => Some(TraitId::Finesse),
        "Poacher" => Some(TraitId::Poacher),
        "Panenka" => Some(TraitId::Panenka),
        "LobMaster" => Some(TraitId::LobMaster),
        "Acrobat" => Some(TraitId::Acrobat),
        // Passing & Playmaking (5)
        "Maestro" => Some(TraitId::Maestro),
        "Crosser" => Some(TraitId::Crosser),
        "DeadBall" => Some(TraitId::DeadBall),
        "Metronome" => Some(TraitId::Metronome),
        "Architect" => Some(TraitId::Architect),
        // Dribbling & Ball Control (6)
        "Speedster" => Some(TraitId::Speedster),
        "Technician" => Some(TraitId::Technician),
        "Tank" => Some(TraitId::Tank),
        "Magnet" => Some(TraitId::Magnet),
        "Showman" => Some(TraitId::Showman),
        "Unshakable" => Some(TraitId::Unshakable),
        // Defense & Physical (8)
        "Vacuum" => Some(TraitId::Vacuum),
        "Wall" => Some(TraitId::Wall),
        "AirRaid" => Some(TraitId::AirRaid),
        "Engine" => Some(TraitId::Engine),
        "Reader" => Some(TraitId::Reader),
        "Shadow" => Some(TraitId::Shadow),
        "Bully" => Some(TraitId::Bully),
        "Motor" => Some(TraitId::Motor),
        // Goalkeeper (4)
        "Spider" => Some(TraitId::Spider),
        "Sweeper" => Some(TraitId::Sweeper),
        "Giant" => Some(TraitId::Giant),
        "Quarterback" => Some(TraitId::Quarterback),
        _ => None,
    }
}

/// Parse trait tier string to enum
fn parse_trait_tier(s: &str) -> TraitTier {
    match s {
        "Silver" => TraitTier::Silver,
        "Gold" => TraitTier::Gold,
        _ => TraitTier::Bronze, // default
    }
}

/// Build TraitSlots from embedded trait list
fn build_trait_slots(traits: Option<&Vec<EmbeddedTrait>>) -> TraitSlots {
    let Some(trait_list) = traits else {
        return TraitSlots::default();
    };

    let mut slots = TraitSlots::with_unlocked(4); // All slots unlocked for JSON input
    for (i, t) in trait_list.iter().take(4).enumerate() {
        if let Some(id) = parse_trait_id(&t.id) {
            let tier = parse_trait_tier(&t.tier);
            let _ = slots.equip(i, EquippedTrait::new(id, tier));
        }
    }
    slots
}

/// Main entry point for JSON API - simulates a match from JSON request
pub fn simulate_match_json(request_json: &str) -> Result<String, String> {
    use std::io::{self, Write};
    println!("🔴🔴🔴 [simulate_match_json] ENTRY POINT CALLED 🔴🔴🔴");
    io::stdout().flush().unwrap();

    // Parse request
    let request: MatchRequest =
        serde_json::from_str(request_json).map_err(|e| format!("Invalid JSON request: {}", e))?;

    // Validate schema version
    if request.schema_version != 1 {
        return Err(format!("Unsupported schema version: {}", request.schema_version));
    }

    let MatchRequest {
        seed,
        home_team: home_team_data,
        away_team: away_team_data,
        user_player,
        home_instructions,
        away_instructions,
        enable_position_tracking,
        ..
    } = request;

    // Extract player instructions before converting team data
    let home_player_instructions = home_team_data.player_instructions.clone();
    let away_player_instructions = away_team_data.player_instructions.clone();

    // Convert to internal models
    let home_team = convert_team(home_team_data)?;
    let away_team = convert_team(away_team_data)?;

    // Validate teams
    home_team.validate().map_err(|e| format!("Home team validation failed: {}", e))?;
    away_team.validate().map_err(|e| format!("Away team validation failed: {}", e))?;

    // Create match plan with user config
    let user_config = user_player.map(|up| {
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
            highlight_level: match up.highlight_level {
                HighlightLevel::Skip => crate::engine::HighlightLevel::Skip,
                HighlightLevel::Simple => crate::engine::HighlightLevel::Simple,
                HighlightLevel::MyPlayer => crate::engine::HighlightLevel::MyPlayer,
                HighlightLevel::Full => crate::engine::HighlightLevel::Full,
            },
        }
    });

    let plan = MatchPlan {
        home_team,
        away_team,
        seed,
        user_player: user_config,
        home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        home_instructions,
        away_instructions,
        home_player_instructions,
        away_player_instructions,
        home_ai_difficulty: None,
        away_ai_difficulty: None,
    };

    // Run simulation
    let mut engine = MatchEngine::new(plan)?;
    apply_exp_config_from_env(&mut engine)?;

    // Enable UAE pipeline if USE_UAE=1 environment variable is set
    if std::env::var("USE_UAE").map(|v| v == "1").unwrap_or(false) {
        engine = engine.with_uae_pipeline(true);
        println!("🟢🟢🟢 [simulate_match_json] UAE pipeline ENABLED 🟢🟢🟢");
        io::stdout().flush().unwrap();
    }

    // Enable position tracking if requested (for replay)
    println!(
        "🔴🔴🔴 [simulate_match_json] enable_position_tracking={} 🔴🔴🔴",
        enable_position_tracking
    );
    io::stdout().flush().unwrap();
    if enable_position_tracking {
        engine = engine.with_position_tracking();
        println!("🔴🔴🔴 [simulate_match_json] with_position_tracking() called 🔴🔴🔴");
        io::stdout().flush().unwrap();
    }

    println!("🔴🔴🔴 [simulate_match_json] About to call engine.simulate() 🔴🔴🔴");
    io::stdout().flush().unwrap();
    let result = engine.simulate();
    println!("🔴🔴🔴 [simulate_match_json] engine.simulate() returned 🔴🔴🔴");
    io::stdout().flush().unwrap();

    // Convert result to JSON
    let response_json =
        serde_json::to_string(&result).map_err(|e| format!("Failed to serialize result: {}", e))?;

    Ok(response_json)
}

/// JSON API with replay recording - returns both match result and replay events
/// Returns tuple: (result_json, replay_json)
pub fn simulate_match_json_with_replay(request_json: &str) -> Result<(String, String), String> {
    // Parse request
    let request: MatchRequest =
        serde_json::from_str(request_json).map_err(|e| format!("Invalid JSON request: {}", e))?;

    // Validate schema version
    if request.schema_version != 1 {
        return Err(format!("Unsupported schema version: {}", request.schema_version));
    }

    let MatchRequest {
        seed,
        home_team: home_team_data,
        away_team: away_team_data,
        user_player,
        home_instructions,
        away_instructions,
        ..
    } = request;

    // Extract player instructions before converting team data
    let home_player_instructions = home_team_data.player_instructions.clone();
    let away_player_instructions = away_team_data.player_instructions.clone();

    // Convert to internal models
    let home_team = convert_team(home_team_data)?;
    let away_team = convert_team(away_team_data)?;

    // Validate teams
    home_team.validate().map_err(|e| format!("Home team validation failed: {}", e))?;
    away_team.validate().map_err(|e| format!("Away team validation failed: {}", e))?;

    // Create match plan with user config
    let user_config = user_player.map(|up| {
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
            highlight_level: match up.highlight_level {
                HighlightLevel::Skip => crate::engine::HighlightLevel::Skip,
                HighlightLevel::Simple => crate::engine::HighlightLevel::Simple,
                HighlightLevel::MyPlayer => crate::engine::HighlightLevel::MyPlayer,
                HighlightLevel::Full => crate::engine::HighlightLevel::Full,
            },
        }
    });

    let plan = MatchPlan {
        home_team,
        away_team,
        seed,
        user_player: user_config,
        home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        home_instructions,
        away_instructions,
        home_player_instructions,
        away_player_instructions,
        home_ai_difficulty: None,
        away_ai_difficulty: None,
    };

    // Run simulation with position tracking and replay recording
    let mut engine = MatchEngine::new(plan)?;
    apply_exp_config_from_env(&mut engine)?;
    engine = engine.with_position_tracking().with_replay_recording();

    let result = engine.simulate();

    // Get replay document
    let replay_doc = engine.take_replay_doc();

    // Convert result to JSON
    let result_json =
        serde_json::to_string(&result).map_err(|e| format!("Failed to serialize result: {}", e))?;

    // Convert replay to JSON
    let replay_json = match replay_doc {
        Some(doc) => {
            serde_json::to_string(&doc).map_err(|e| format!("Failed to serialize replay: {}", e))?
        }
        None => "null".to_string(),
    };

    Ok((result_json, replay_json))
}

/// JSON API v2 - simulates a match from UID-based roster input (schema_version=2)
pub fn simulate_match_v2_json(request_json: &str) -> Result<String, String> {   
    let (plan, enable_position_tracking) = match_plan_from_match_request_v2_json(request_json)?;

    let mut engine = MatchEngine::new(plan)?;
    apply_exp_config_from_env(&mut engine)?;
    if enable_position_tracking {
        engine = engine.with_position_tracking();
    }

    let result = engine.simulate();
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize result: {}", e))
}

/// JSON API v2 - simulates a match and returns (result_json, replay_json)
pub fn simulate_match_v2_json_with_replay(request_json: &str) -> Result<(String, String), String> {
    let (plan, _enable_position_tracking) = match_plan_from_match_request_v2_json(request_json)?;

    // Mirror v1 behavior: with_replay always enables both position tracking + replay recording.
    let mut engine = MatchEngine::new(plan)?;
    apply_exp_config_from_env(&mut engine)?;
    engine = engine.with_position_tracking().with_replay_recording();

    let result = engine.simulate();
    let replay_doc = engine.take_replay_doc();

    let result_json =
        serde_json::to_string(&result).map_err(|e| format!("Failed to serialize result: {}", e))?;

    let replay_json = match replay_doc {
        Some(doc) => {
            serde_json::to_string(&doc).map_err(|e| format!("Failed to serialize replay: {}", e))?
        }
        None => "null".to_string(),
    };

    Ok((result_json, replay_json))
}

fn convert_user_player_v2(
    up: UserPlayerConfigV2,
    home_team: &Team,
    away_team: &Team,
    home_uid_to_name: &HashMap<String, String>,
    away_uid_to_name: &HashMap<String, String>,
) -> Result<crate::engine::UserPlayerConfig, String> {
    let is_home_team = match up.team.as_str() {
        "home" => true,
        "away" => false,
        other => return Err(format!("Invalid user_player.team (expected 'home'|'away'): {other}")),
    };

    let has_uid = up.player_uid.is_some();
    let has_slot = up.roster_slot.is_some();
    if has_uid == has_slot {
        return Err("user_player must specify exactly one of player_uid or roster_slot".to_string());
    }

    let player_name = if let Some(uid) = up.player_uid {
        let map = if is_home_team { home_uid_to_name } else { away_uid_to_name };
        map.get(&uid).cloned().ok_or_else(|| {
            format!("user_player.player_uid not found in selected team roster: {uid}")
        })?
    } else if let Some(slot) = up.roster_slot {
        if slot >= 18 {
            return Err(format!("user_player.roster_slot out of range (0..17): {slot}"));
        }
        let team = if is_home_team { home_team } else { away_team };
        team.players
            .get(slot)
            .map(|p| p.name.clone())
            .ok_or_else(|| format!("user_player.roster_slot out of range (0..17): {slot}"))?
    } else {
        return Err("user_player missing selector".to_string());
    };

    // C6: Resolve player_index from player_name
    let team = if is_home_team { home_team } else { away_team };
    let base_idx = if is_home_team { 0 } else { 11 };
    let player_index = team
        .get_starting_11()
        .iter()
        .position(|p| p.name == player_name)
        .map(|i| base_idx + i)
        .unwrap_or(base_idx + 9); // Fallback to first attacker

    Ok(crate::engine::UserPlayerConfig {
        is_home_team,
        player_name,
        player_index,
        highlight_level: match up.highlight_level {
            HighlightLevel::Skip => crate::engine::HighlightLevel::Skip,
            HighlightLevel::Simple => crate::engine::HighlightLevel::Simple,
            HighlightLevel::MyPlayer => crate::engine::HighlightLevel::MyPlayer,
            HighlightLevel::Full => crate::engine::HighlightLevel::Full,
        },
    })
}

fn convert_team_v2(
    data: TeamDataV2,
    _use_real_names: bool,
) -> Result<(Team, HashMap<String, String>, Option<HashMap<String, PlayerInstructions>>), String> {
    let TeamDataV2 { name, formation: formation_str, roster, player_instructions } = data;

    let formation = parse_formation(&formation_str)?;

    if roster.len() != 18 {
        return Err(format!("Team must have exactly 18 roster entries, found {}", roster.len()));
    }

    // Duplicate check: use UID for Uid entries, slot index for Embedded entries
    let mut seen_uids = HashSet::<String>::new();
    for (i, entry) in roster.iter().enumerate() {
        match entry {
            RosterEntry::Uid(uid) => {
                if !seen_uids.insert(uid.clone()) {
                    return Err(format!("Duplicate player UID in roster: {uid}"));
                }
            }
            RosterEntry::UidWithMeta(meta) => {
                if !seen_uids.insert(meta.uid.clone()) {
                    return Err(format!("Duplicate player UID in roster: {}", meta.uid));
                }
            }
            RosterEntry::Embedded(_) => {
                // For embedded entries, use slot index as unique key
                seen_uids.insert(format!("__embedded_slot_{i}"));
            }
        }
    }

    let mut resolved: Vec<(String, Player)> = Vec::with_capacity(18);
    for (slot_idx, entry) in roster.into_iter().enumerate() {
        let (uid_key, player) = match entry {
            RosterEntry::Uid(uid) => {
                // FIX01: condition is required for match-time determinism & CI proof.
                return Err(err_code(
                    error_codes::INVALID_CONDITION_RANGE,
                    format!(
                        "missing condition for UID roster entry '{uid}' (use object form {{\"uid\":\"...\",\"condition\":3}})"
                    ),
                ));
            }
            RosterEntry::UidWithMeta(meta) => {
                let condition = validate_condition_level(meta.condition)?;
                let uid = meta.uid;

                // Resolve from CSV/DB
                let person = resolve_person_by_player_uid(&uid)?;
                let name = person.name.clone();
                let position_token = primary_position_token(&person.position);
                let position = map_person_position(&position_token);
                let overall = ca_to_overall(person.ca);

                let fm_attrs = person.get_attributes_map();
                let match_attrs = crate::data::ScaleConverter::fm_to_match_engine_attrs(&fm_attrs);

                let player_attributes = PlayerAttributes {
                    // Technical (14)
                    corners: *match_attrs.get("corners").unwrap_or(&50),
                    crossing: *match_attrs.get("crossing").unwrap_or(&50),
                    dribbling: *match_attrs.get("dribbling").unwrap_or(&50),
                    finishing: *match_attrs.get("finishing").unwrap_or(&50),
                    first_touch: *match_attrs.get("first_touch").unwrap_or(&50),
                    free_kicks: *match_attrs.get("free_kick_taking").unwrap_or(&50),
                    heading: *match_attrs.get("heading").unwrap_or(&50),
                    long_shots: *match_attrs.get("long_shots").unwrap_or(&50),
                    long_throws: *match_attrs.get("long_throws").unwrap_or(&50),
                    marking: *match_attrs.get("marking").unwrap_or(&50),
                    passing: *match_attrs.get("passing").unwrap_or(&50),
                    penalty_taking: *match_attrs.get("penalty_taking").unwrap_or(&50),
                    tackling: *match_attrs.get("tackling").unwrap_or(&50),
                    technique: *match_attrs.get("technique").unwrap_or(&50),
                    // Mental (14)
                    aggression: *match_attrs.get("aggression").unwrap_or(&50),
                    anticipation: *match_attrs.get("anticipation").unwrap_or(&50),
                    bravery: *match_attrs.get("bravery").unwrap_or(&50),
                    composure: *match_attrs.get("composure").unwrap_or(&50),
                    concentration: *match_attrs.get("concentration").unwrap_or(&50),
                    decisions: *match_attrs.get("decisions").unwrap_or(&50),
                    determination: *match_attrs.get("determination").unwrap_or(&50),
                    flair: *match_attrs.get("flair").unwrap_or(&50),
                    leadership: *match_attrs.get("leadership").unwrap_or(&50),
                    off_the_ball: *match_attrs.get("off_the_ball").unwrap_or(&50),
                    positioning: *match_attrs.get("positioning").unwrap_or(&50),
                    teamwork: *match_attrs.get("teamwork").unwrap_or(&50),
                    vision: *match_attrs.get("vision").unwrap_or(&50),
                    work_rate: *match_attrs.get("work_rate").unwrap_or(&50),
                    // Physical (8)
                    acceleration: *match_attrs.get("acceleration").unwrap_or(&50),
                    agility: *match_attrs.get("agility").unwrap_or(&50),
                    balance: *match_attrs.get("balance").unwrap_or(&50),
                    jumping: *match_attrs.get("jumping").unwrap_or(&50),
                    natural_fitness: *match_attrs.get("natural_fitness").unwrap_or(&50),
                    pace: *match_attrs.get("pace").unwrap_or(&50),
                    stamina: *match_attrs.get("stamina").unwrap_or(&50),
                    strength: *match_attrs.get("strength").unwrap_or(&50),
                    // Goalkeeper (11) - v5 schema, FM scale (1-20) converted to match engine scale
                    gk_aerial_reach: *match_attrs.get("gk_aerial_reach").unwrap_or(&0),
                    gk_command_of_area: *match_attrs.get("gk_command_of_area").unwrap_or(&0),
                    gk_communication: *match_attrs.get("gk_communication").unwrap_or(&0),
                    gk_eccentricity: *match_attrs.get("gk_eccentricity").unwrap_or(&0),
                    gk_handling: *match_attrs.get("gk_handling").unwrap_or(&0),
                    gk_kicking: *match_attrs.get("gk_kicking").unwrap_or(&0),
                    gk_one_on_ones: *match_attrs.get("gk_one_on_ones").unwrap_or(&0),
                    gk_reflexes: *match_attrs.get("gk_reflexes").unwrap_or(&0),
                    gk_rushing_out: *match_attrs.get("gk_rushing_out").unwrap_or(&0),
                    gk_punching: *match_attrs.get("gk_punching").unwrap_or(&0),
                    gk_throwing: *match_attrs.get("gk_throwing").unwrap_or(&0),
                };

                (
                    uid,
                    Player {
                        name,
                        position,
                        overall,
                        condition,
                        attributes: Some(player_attributes),
                        equipped_skills: Vec::new(),
                        traits: Default::default(),
                        personality: Default::default(),
                    },
                )
            }
            RosterEntry::Embedded(embedded) => {
                // NEW: Embedded player data with full attributes (MRQ0 v3)
                let position = parse_position(&embedded.position)
                    .unwrap_or(crate::models::player::Position::MF);
                let condition = validate_condition_level(embedded.condition)?;

                // Build attributes from embedded data or derive from overall
                let player_attributes = if let Some(ref attrs) = embedded.attributes {
                    PlayerAttributes {
                        // Technical (14)
                        corners: attrs.corners,
                        crossing: attrs.crossing,
                        dribbling: attrs.dribbling,
                        finishing: attrs.finishing,
                        first_touch: attrs.first_touch,
                        free_kicks: attrs.free_kick_taking,
                        heading: attrs.heading,
                        long_shots: attrs.long_shots,
                        long_throws: attrs.long_throws,
                        marking: attrs.marking,
                        passing: attrs.passing,
                        penalty_taking: attrs.penalty_taking,
                        tackling: attrs.tackling,
                        technique: attrs.technique,
                        // Mental (14)
                        aggression: attrs.aggression,
                        anticipation: attrs.anticipation,
                        bravery: attrs.bravery,
                        composure: attrs.composure,
                        concentration: attrs.concentration,
                        decisions: attrs.decisions,
                        determination: attrs.determination,
                        flair: attrs.flair,
                        leadership: attrs.leadership,
                        off_the_ball: attrs.off_the_ball,
                        positioning: attrs.positioning,
                        teamwork: attrs.teamwork,
                        vision: attrs.vision,
                        work_rate: attrs.work_rate,
                        // Physical (8)
                        acceleration: attrs.acceleration,
                        agility: attrs.agility,
                        balance: attrs.balance,
                        jumping: attrs.jumping_reach,
                        natural_fitness: attrs.natural_fitness,
                        pace: attrs.pace,
                        stamina: attrs.stamina,
                        strength: attrs.strength,
                        // Goalkeeper (11) - embedded data doesn't include GK attrs, default 0
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
                    }
                } else {
                    // Fallback: derive from overall
                    PlayerAttributes::from_uniform(embedded.overall)
                };

                // Parse personality archetype
                let personality = match embedded.personality.as_deref() {
                    Some("Leader") => PersonalityArchetype::Leader,
                    Some("Genius") => PersonalityArchetype::Genius,
                    Some("Workhorse") => PersonalityArchetype::Workhorse,
                    Some("Rebel") => PersonalityArchetype::Rebel,
                    _ => PersonalityArchetype::Steady, // default
                };

                // Build trait slots from embedded traits
                let trait_slots = build_trait_slots(embedded.traits.as_ref());

                let uid_key = embedded
                    .track_id
                    .map(|id| format!("embedded:{id}"))
                    .unwrap_or_else(|| format!("embedded:slot_{slot_idx}"));

                (
                    uid_key,
                    Player {
                        name: embedded.name,
                        position,
                        overall: embedded.overall,
                        condition,
                        attributes: Some(player_attributes),
                        equipped_skills: Vec::new(),
                        traits: trait_slots,
                        personality,
                    },
                )
            }
        };
        resolved.push((uid_key, player));
    }

    // Deterministic per-team name disambiguation (only if duplicates exist).
    let base_names: Vec<String> = resolved.iter().map(|(_, p)| p.name.clone()).collect();
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for n in &base_names {
        *name_counts.entry(n.clone()).or_insert(0) += 1;
    }
    let mut occurrence: HashMap<String, usize> = HashMap::new();
    for (i, (uid, player)) in resolved.iter_mut().enumerate() {
        let base = base_names[i].clone();
        if name_counts.get(&base).copied().unwrap_or(0) > 1 {
            let c = occurrence.entry(base.clone()).or_insert(0);
            *c += 1;
            if *c > 1 {
                player.name = format!("{base}#{uid}");
            } else {
                player.name = base;
            }
        }
    }

    let mut uid_to_name: HashMap<String, String> = HashMap::new();
    for (uid, player) in &resolved {
        uid_to_name.insert(uid.clone(), player.name.clone());
    }

    let player_instructions_by_name = match player_instructions {
        None => None,
        Some(map) => {
            let mut out: HashMap<String, PlayerInstructions> = HashMap::new();
            for (slot_key, instr) in map {
                let slot: usize = slot_key.parse().map_err(|_| {
                    format!("Invalid player_instructions key (expected 0..17): {slot_key}")
                })?;
                if slot >= 18 {
                    return Err(format!("player_instructions slot out of range (0..17): {slot}"));
                }
                let name = resolved[slot].1.name.clone();
                out.insert(name, instr);
            }
            Some(out)
        }
    };

    let players = resolved.into_iter().map(|(_, p)| p).collect::<Vec<_>>();
    Ok((Team { name, formation, players }, uid_to_name, player_instructions_by_name))
}

fn ca_to_overall(ca: u8) -> u8 {
    // Person.ca is 0..200, while engine Player.overall expects 0..100-ish.
    // round(ca / 2) == (ca + 1) / 2 for integer ca.
    let raw: u16 = (ca as u16).div_ceil(2);
    raw.clamp(1, 100) as u8
}

fn primary_position_token(position: &str) -> String {
    let cleaned = position.replace('"', "");
    let first_segment = cleaned.split(',').next().unwrap_or("").trim();
    let base = first_segment.split('(').next().unwrap_or("").trim();
    if base.is_empty() {
        String::from("MF")
    } else {
        base.to_string()
    }
}

fn map_person_position(token: &str) -> crate::models::player::Position {
    use crate::models::player::Position;
    let upper = token.trim().to_uppercase();

    match upper.as_str() {
        "GK" | "GKP" => Position::GK,

        // Defenders
        "DL" | "LB" => Position::LB,
        "DR" | "RB" => Position::RB,
        "DC" | "CB" => Position::CB,
        "WBL" | "LWB" => Position::LWB,
        "WBR" | "RWB" => Position::RWB,
        "WB" | "D" => Position::DF,

        // Midfielders
        "DM" | "CDM" => Position::CDM,
        "MC" | "CM" => Position::CM,
        "AM" | "AMC" | "CAM" => Position::CAM,
        "ML" | "LM" => Position::LM,
        "MR" | "RM" => Position::RM,
        "M" => Position::MF,

        // Forwards
        "AML" | "LW" => Position::LW,
        "AMR" | "RW" => Position::RW,
        "CF" => Position::CF,
        "FW" => Position::FW,
        "ST" | "SS" => Position::ST,

        // Already-generic tokens
        "DF" => Position::DF,
        "MF" => Position::MF,

        _ => {
            if upper.starts_with('G') {
                Position::GK
            } else if upper.starts_with('D') || upper.starts_with("WB") {
                Position::DF
            } else if upper.starts_with('M') || upper.starts_with('A') {
                Position::MF
            } else if upper.starts_with('S') || upper.starts_with('F') {
                Position::FW
            } else {
                Position::MF
            }
        }
    }
}

fn convert_team(data: TeamData) -> Result<Team, String> {
    // Parse formation
    let formation = parse_formation(&data.formation)?;

    // Convert players
    if data.players.len() != 18 {
        return Err(format!("Team must have exactly 18 players, found {}", data.players.len()));
    }

    let players = data.players.into_iter().map(convert_player).collect::<Result<Vec<_>, _>>()?;

    Ok(Team { name: data.name, formation, players })
}

fn convert_player(data: PlayerData) -> Result<Player, String> {
    let position = parse_position(&data.position)?;
    let attributes = Some(PlayerAttributes::from_uniform(data.overall));
    let condition = validate_condition_level(data.condition)?;

    Ok(Player {
        name: data.name,
        position,
        overall: data.overall,
        condition,
        attributes,
        equipped_skills: Vec::new(),
        traits: Default::default(),
        personality: Default::default(),
    })
}

fn parse_formation(formation_str: &str) -> Result<crate::models::team::Formation, String> {
    use crate::models::team::Formation;

    match formation_str {
        "4-4-2" => Ok(Formation::F442),
        "4-3-3" => Ok(Formation::F433),
        "4-4-1-1" => Ok(Formation::F4411),
        "4-3-2-1" => Ok(Formation::F4321),
        "4-2-2-2" => Ok(Formation::F4222),
        "4-5-1" => Ok(Formation::F451),
        "3-5-2" => Ok(Formation::F352),
        "3-4-2-1" => Ok(Formation::F3421),
        "3-4-1-2" => Ok(Formation::F3412),
        "5-3-2" => Ok(Formation::F532),
        "4-2-3-1" => Ok(Formation::F4231),
        "4-1-4-1" => Ok(Formation::F4141),
        "3-4-3" => Ok(Formation::F343),
        "5-4-1" => Ok(Formation::F541),
        _ => Err(err_code(
            error_codes::UNSUPPORTED_FORMATION,
            format!("formation not in allowlist: {formation_str}"),
        )),
    }
}

fn parse_position(position_str: &str) -> Result<crate::models::player::Position, String> {
    use crate::models::player::Position;

    match position_str.to_uppercase().as_str() {
        "GK" => Ok(Position::GK),
        "LB" => Ok(Position::LB),
        "CB" => Ok(Position::CB),
        "RB" => Ok(Position::RB),
        "LWB" => Ok(Position::LWB),
        "RWB" => Ok(Position::RWB),
        "CDM" => Ok(Position::CDM),
        "CM" => Ok(Position::CM),
        "CAM" => Ok(Position::CAM),
        "LM" => Ok(Position::LM),
        "RM" => Ok(Position::RM),
        "LW" => Ok(Position::LW),
        "RW" => Ok(Position::RW),
        "CF" => Ok(Position::CF),
        "ST" => Ok(Position::ST),
        "DF" => Ok(Position::DF),
        "MF" => Ok(Position::MF),
        "FW" => Ok(Position::FW),
        _ => Err(format!("Invalid position: {}", position_str)),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_player_sets_attributes_from_overall() {
        let data =
            PlayerData { name: "Test".to_string(), position: "ST".to_string(), overall: 130, condition: 3 };
        let player = convert_player(data).expect("convert_player should succeed");
        assert!(player.attributes.is_some(), "attributes must be injected for JSON API players");
        let attrs = player.attributes.unwrap();
        // from_uniform clamps to 1..=100
        assert_eq!(attrs.pace, 100);
        assert_eq!(attrs.finishing, 100);
    }
}
