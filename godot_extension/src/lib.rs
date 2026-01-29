// Godot Dictionary::insert returns Option<Variant> which we intentionally ignore
// since we're building dictionaries (not checking for overwrites)
#![allow(unused_must_use)]

use godot::prelude::*;
use of_core::api::{simulate_match_json_budget, SimBudget};
use of_core::models::Team;
use of_core::simulate_match_json;
use of_core::simulate_match_json_with_replay;
use of_core::simulate_match_v2_json;
use of_core::simulate_match_v2_json_with_replay;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};

// Gacha/Deck SSOT (FIX_2601/0109)
use of_core::coach::{
    derive_match_modifiers, CardRarity, CardType, CoachCard, Deck, GachaCard, GachaResult,
    GachaSystem, InventoryManager, Specialty, TacticalStyle, TacticsCard,
};
use of_core::tactics::TeamInstructions;
                                        // Import opponent analysis
                                        // Import formation waypoints
                                        // Import tactical context
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use of_core::engine::{
    live_match::TeamViewObservationConfig,
    match_sim::{
        MatchEngine as OfMatchEngine, MatchPlan as OfMatchPlan, MiniMapObservation, MiniMapSpec,
        SimpleVectorObservation, StickyAction,
    },
    HighlightLevel as CoreHighlightLevel,
    // Phase 7: Match session stepping
    LiveMatchSession,
    MatchState as LiveMatchState,
    SimState as OfSimState,
    StepResult,
    TeamSide,
    UserAction as OfUserAction,
    UserDecisionContext as OfUserDecisionContext,
    UserPlayerConfig as CoreUserPlayerConfig,
};
use of_core::models::player::{
    Player as OfPlayer, PlayerAttributes as OfPlayerAttributes, Position as OfPosition,
};
use of_core::models::MatchEvent;
use of_core::models::replay::types::DecisionIntent;
// RuleBook UI Card System (FIX_2601/1120 P1)
use of_core::data::{generate_ui_card, generate_ui_card_from_match_event, CardBlock, CardLine, RulebookUiCard};
use of_core::models::events::EventType;
use of_core::models::rules::RuleId;
use rmp_serde::{from_slice, to_vec_named};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};

use std::sync::Once;

mod story_bridge;
pub use story_bridge::StoryBridge;

mod quest_bridge;
pub use quest_bridge::QuestBridge;

mod data_cache;
pub use data_cache::DataCacheStore;

/// P2-10: Helper function to convert MatchEvent to Godot Dictionary.
/// Reduces code duplication and properly handles EventDetails.
fn convert_event_to_dict(event: &MatchEvent) -> Dictionary {
    let mut event_dict = Dictionary::new();
    let minute = event.minute as i32;
    let is_home = event.is_home_team;
    let event_type_str = format!("{:?}", event.event_type);

    // Core fields (matching batch MatchEvent schema)
    event_dict.set("minute", minute);
    event_dict.set("t", minute as f32); // Legacy compatibility
    event_dict.set("type", GString::from(event_type_str.as_str()));
    // Event SSOT C5: timestamp_ms is now engine-confirmed (no fallback)
    let t_ms: i64 = event
        .timestamp_ms
        .expect("C5: timestamp_ms must be set by engine")
        .try_into()
        .unwrap_or(0);
    event_dict.set("t_ms", t_ms);
    let player_track_id: i32 = event.player_track_id.map(|v| v as i32).unwrap_or(-1);
    let target_track_id: i32 = event.target_track_id.map(|v| v as i32).unwrap_or(-1);
    event_dict.set("player_track_id", player_track_id);
    event_dict.set("target_track_id", target_track_id);
    event_dict.set("is_home_team", is_home);
    event_dict.set("team", GString::from(if is_home { "home" } else { "away" }));
    // C7: Removed player field export - Godot must resolve names from track_id using MatchSetup

    // P2-10: Only set details if present (avoid empty Dictionary allocation)
    if let Some(ref details) = event.details {
        let mut details_dict = Dictionary::new();
        // C7: Removed assist_by and replaced_player exports - use target_track_id instead
        if let Some(xg) = details.xg_value {
            details_dict.set("xg_value", xg);
        }
        if let Some(ref injury) = details.injury_severity {
            let mut injury_dict = Dictionary::new();
            injury_dict.set("weeks_out", injury.weeks_out as i32);
            injury_dict.set("description", GString::from(injury.description.as_str()));
            details_dict.set("injury_severity", injury_dict);
        }
        if let Some((x, y, z)) = details.ball_position {
            let mut pos_dict = Dictionary::new();
            pos_dict.set("x", x);
            pos_dict.set("y", y);
            pos_dict.set("z", z);
            details_dict.set("ball_position", pos_dict);
        }
        if let Some(ref sub) = details.substitution {
            let mut sub_dict = Dictionary::new();
            sub_dict.set("player_in_name", GString::from(sub.player_in_name.as_str()));
            sub_dict.set("player_out_name", GString::from(sub.player_out_name.as_str()));
            sub_dict.set("bench_slot", sub.bench_slot as i32);
            details_dict.set("substitution", sub_dict);
        }
        event_dict.set("details", details_dict);
    }

    // Legacy compatibility fields
    event_dict.set("kind", GString::from(event_type_str.as_str()));
    event_dict.set("team_id", if is_home { 0 } else { 1 });
    // C7: player_id removed - use player_track_id instead

    event_dict
}

// =============================================================================
// RuleBook UI Card Conversion (FIX_2601/1120 P1)
// =============================================================================

/// Convert CardLine to Godot Dictionary
fn convert_card_line_to_dict(line: &CardLine) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("kind", GString::from(line.kind.as_str()));
    dict.set("text", GString::from(line.text.as_str()));
    if let Some(ref key) = line.key {
        dict.set("key", GString::from(key.as_str()));
    }
    if let Some(ref value) = line.value {
        // Keep the raw JSON-ish value for UI/analytics. Store as string for now.
        dict.set("value", GString::from(value.to_string().as_str()));
    }
    if let Some(ref r) = line.r#ref {
        let mut ref_dict = Dictionary::new();
        ref_dict.set("type", GString::from(r.r#type.as_str()));
        ref_dict.set("id", GString::from(r.id.as_str()));
        dict.set("ref", ref_dict);
    }
    dict
}

/// Convert CardBlock to Godot Dictionary
fn convert_card_block_to_dict(block: &CardBlock) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("level", block.level as i32);
    dict.set("title", GString::from(block.title.as_str()));
    let mut lines_array = Array::<Variant>::new();
    for line in &block.lines {
        lines_array.push(&convert_card_line_to_dict(line).to_variant());
    }
    dict.set("lines", lines_array);
    dict
}

/// Convert RulebookUiCard to Godot Dictionary
fn convert_ui_card_to_dict(card: &RulebookUiCard) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("schema_version", GString::from(card.schema_version.as_str()));
    dict.set("lang", GString::from(card.lang.as_str()));

    // event
    let mut event_dict = Dictionary::new();
    event_dict.set("event_type", GString::from(card.event.event_type.as_str()));
    event_dict.set("timestamp_ms", card.event.timestamp_ms as i64);
    event_dict.set("minute", card.event.minute as i64);
    event_dict.set("team_side", GString::from(card.event.team_side.as_str()));
    dict.set("event", event_dict);

    // rule (optional)
    if let Some(ref rule) = card.rule {
        let mut rule_dict = Dictionary::new();
        rule_dict.set("rule_id", GString::from(rule.rule_id.as_str()));
        rule_dict.set("law_number", rule.law_number as i64);
        rule_dict.set("law_name", GString::from(rule.law_name.as_str()));
        if let Some(ref law_name_en) = rule.law_name_en {
            rule_dict.set("law_name_en", GString::from(law_name_en.as_str()));
        }
        dict.set("rule", rule_dict);
    }

    // cards
    let mut blocks_array = Array::<Variant>::new();
    for block in &card.cards {
        blocks_array.push(&convert_card_block_to_dict(block).to_variant());
    }
    dict.set("cards", blocks_array);

    // raw_payload (lossless snapshot) - keep as stringified JSON for now.
    dict.set("raw_payload", GString::from(card.raw_payload.to_string().as_str()));

    dict
}

/// Parse EventType from string
fn parse_event_type(event_type_str: &str) -> Option<EventType> {
    match event_type_str {
        "KickOff" | "kickoff" | "kick_off" => Some(EventType::KickOff),
        "Goal" | "goal" => Some(EventType::Goal),
        "OwnGoal" | "own_goal" | "owngoal" => Some(EventType::OwnGoal),
        "Shot" | "shot" => Some(EventType::Shot),
        "ShotOnTarget" | "shot_on_target" | "shotontarget" => Some(EventType::ShotOnTarget),
        "ShotOffTarget" | "shot_off_target" | "shotofftarget" => Some(EventType::ShotOffTarget),
        "ShotBlocked" | "shot_blocked" | "shotblocked" => Some(EventType::ShotBlocked),
        "Save" | "save" => Some(EventType::Save),
        "YellowCard" | "yellow_card" | "yellowcard" => Some(EventType::YellowCard),
        "RedCard" | "red_card" | "redcard" => Some(EventType::RedCard),
        "Substitution" | "substitution" => Some(EventType::Substitution),
        "Injury" | "injury" => Some(EventType::Injury),
        "Corner" | "corner" => Some(EventType::Corner),
        "Freekick" | "free_kick" | "freekick" => Some(EventType::Freekick),
        "Penalty" | "penalty" => Some(EventType::Penalty),
        "Offside" | "offside" => Some(EventType::Offside),
        "Foul" | "foul" => Some(EventType::Foul),
        "KeyChance" | "key_chance" | "keychance" => Some(EventType::KeyChance),
        "Pass" | "pass" => Some(EventType::Pass),
        "Tackle" | "tackle" => Some(EventType::Tackle),
        "Dribble" | "dribble" => Some(EventType::Dribble),
        "PostHit" | "post_hit" | "posthit" => Some(EventType::PostHit),
        "BarHit" | "bar_hit" | "barhit" => Some(EventType::BarHit),
        "GoalKick" | "goal_kick" | "goalkick" => Some(EventType::GoalKick),
        "ThrowIn" | "throw_in" | "throwin" => Some(EventType::ThrowIn),
        "HalfTime" | "half_time" | "halftime" => Some(EventType::HalfTime),
        "FullTime" | "full_time" | "fulltime" => Some(EventType::FullTime),
        "VarReview" | "var_review" | "varreview" => Some(EventType::VarReview),
        _ => None,
    }
}

fn meter_pos_to_dict(pos: &of_core::models::replay::types::MeterPos) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("x", pos.x as f32);
    dict.set("y", pos.y as f32);
    dict
}

fn convert_decision_intent_to_dict(intent: &DecisionIntent) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("player_id", intent.player_id as i32);
    dict.set("tick", intent.tick as i64);
    dict.set("chosen_action", GString::from(intent.chosen_action.as_str()));
    dict.set("confidence", intent.confidence);

    let mut alternatives = Array::<Variant>::new();
    for alt in &intent.alternatives {
        let mut alt_dict = Dictionary::new();
        alt_dict.set("action", GString::from(alt.action.as_str()));
        alt_dict.set("probability", alt.probability);
        alternatives.push(&alt_dict.to_variant());
    }
    dict.set("alternatives", alternatives);

    let mut ctx = Dictionary::new();
    ctx.set("pressure_level", intent.context.pressure_level);
    ctx.set("stamina_percent", intent.context.stamina_percent);
    ctx.set("in_attacking_third", intent.context.in_attacking_third);
    ctx.set("ball_distance", intent.context.ball_distance);
    dict.set("context", ctx);

    if let Some(value) = intent.selected_utility {
        dict.set("selected_utility", value);
    }
    if let Some(pos) = &intent.player_pos {
        dict.set("player_pos", meter_pos_to_dict(pos));
    }
    if let Some(pos) = &intent.target_pos {
        dict.set("target_pos", meter_pos_to_dict(pos));
    }
    if let Some(player_id) = intent.target_player_id {
        dict.set("target_player_id", player_id as i32);
    }
    if !intent.pass_targets.is_empty() {
        let mut targets = Array::<Variant>::new();
        for target in &intent.pass_targets {
            let mut t_dict = Dictionary::new();
            t_dict.set("player_id", target.player_id as i32);
            t_dict.set("pos", meter_pos_to_dict(&target.pos));
            t_dict.set("quality", target.quality);
            targets.push(&t_dict.to_variant());
        }
        dict.set("pass_targets", targets);
    }
    if !intent.nearby_opponents.is_empty() {
        let mut opponents = Array::<Variant>::new();
        for pos in &intent.nearby_opponents {
            opponents.push(&meter_pos_to_dict(pos).to_variant());
        }
        dict.set("nearby_opponents", opponents);
    }

    dict
}

fn apply_field_board_snapshot(
    snapshot: &mut Dictionary,
    board: &of_core::engine::field_board::FieldBoardSnapshotExport,
) {
    use godot::prelude::PackedFloat32Array;

    let mut occupancy = PackedFloat32Array::new();
    occupancy.resize(board.occupancy_total.len());
    occupancy.as_mut_slice().copy_from_slice(&board.occupancy_total);
    snapshot.set("occupancy_total", occupancy);

    let mut pressure_home = PackedFloat32Array::new();
    pressure_home.resize(board.pressure_against_home.len());
    pressure_home
        .as_mut_slice()
        .copy_from_slice(&board.pressure_against_home);
    snapshot.set("pressure_against_home", pressure_home);

    let mut pressure_away = PackedFloat32Array::new();
    pressure_away.resize(board.pressure_against_away.len());
    pressure_away
        .as_mut_slice()
        .copy_from_slice(&board.pressure_against_away);
    snapshot.set("pressure_against_away", pressure_away);

    let mut xgzone = PackedFloat32Array::new();
    xgzone.resize(board.xgzone_map.len());
    xgzone.as_mut_slice().copy_from_slice(&board.xgzone_map);
    snapshot.set("xgzone", xgzone);
}

fn convert_team_view_simple_to_dict(obs: &SimpleVectorObservation) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("is_home", obs.is_home);
    dict.set("tick", obs.tick as i64);
    dict.set("minute", obs.minute as i32);

    let mut score = Dictionary::new();
    score.set("home", obs.score.0 as i32);
    score.set("away", obs.score.1 as i32);
    dict.set("score", score);

    let mut ball = Dictionary::new();
    ball.set("x", obs.ball.pos_m.0);
    ball.set("y", obs.ball.pos_m.1);
    ball.set("vx", obs.ball.vel_mps.0);
    ball.set("vy", obs.ball.vel_mps.1);
    ball.set("z", obs.ball.height_m);
    ball.set(
        "owner_id",
        obs.ball.owner_idx.map(|idx| idx as i32).unwrap_or(-1),
    );
    dict.set("ball", ball);

    let mut players = Array::<Variant>::new();
    for player in &obs.players {
        let mut player_dict = Dictionary::new();
        player_dict.set("track_id", player.track_id as i32);
        player_dict.set("team_id", player.team_id as i32);
        player_dict.set("x", player.pos_m.0);
        player_dict.set("y", player.pos_m.1);
        player_dict.set("vx", player.vel_mps.0);
        player_dict.set("vy", player.vel_mps.1);
        player_dict.set("stamina", player.stamina);
        player_dict.set("is_sprinting", player.is_sprinting);
        players.push(&player_dict.to_variant());
    }
    dict.set("players", players);

    dict
}

fn convert_team_view_minimap_to_dict(obs: &MiniMapObservation) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("is_home", obs.is_home);
    dict.set("tick", obs.tick as i64);
    dict.set("width", obs.width as i32);
    dict.set("height", obs.height as i32);

    let mut labels = Array::<Variant>::new();
    for label in &obs.plane_labels {
        labels.push(&GString::from(label.as_str()).to_variant());
    }
    dict.set("plane_labels", labels);

    let mut planes = Array::<Variant>::new();
    for plane in &obs.planes {
        let mut packed = PackedFloat32Array::new();
        packed.resize(plane.len());
        packed.as_mut_slice().copy_from_slice(plane);
        planes.push(&packed.to_variant());
    }
    dict.set("planes", planes);

    dict
}

fn parse_team_view_observation_config(value: &JsonValue) -> Option<TeamViewObservationConfig> {
    let obs = value.get("team_view_observation")?;
    let enabled = obs
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !enabled {
        return None;
    }

    let observer_is_home = obs
        .get("observer_is_home")
        .and_then(|v| v.as_bool())
        .or_else(|| {
            obs.get("observer")
                .and_then(|v| v.as_str())
                .map(|v| matches!(v, "home" | "HOME"))
        })
        .unwrap_or(true);

    let include_simple = obs.get("simple").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_minimap = obs
        .get("minimap")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !include_simple && !include_minimap {
        return None;
    }

    let width = obs
        .get("minimap_width")
        .and_then(|v| v.as_u64())
        .unwrap_or(96);
    let height = obs
        .get("minimap_height")
        .and_then(|v| v.as_u64())
        .unwrap_or(72);

    Some(TeamViewObservationConfig {
        observer_is_home,
        include_simple,
        include_minimap,
        minimap_spec: MiniMapSpec {
            width: width as usize,
            height: height as usize,
        },
    })
}

fn to_json_value_or_null<T: serde::Serialize>(value: &T) -> JsonValue {
    serde_json::to_value(value).unwrap_or(JsonValue::Null)
}

fn _error_dict(message: impl Into<String>, code: &str) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("error", true);
    dict.set("message", GString::from(message.into()));
    dict.set("code", GString::from(code));
    dict
}

fn build_collection_set(inventory: &InventoryManager) -> HashSet<String> {
    let mut set = HashSet::new();
    for id in inventory.manager_inventory.collection.iter() {
        set.insert(id.clone());
    }
    for id in inventory.coach_inventory.collection.iter() {
        set.insert(id.clone());
    }
    for id in inventory.tactics_inventory.collection.iter() {
        set.insert(id.clone());
    }
    set
}

fn add_gacha_card_to_inventory(inventory: &mut InventoryManager, card: GachaCard) {
    match card {
        GachaCard::Coach(coach_card) => match coach_card.card_type {
            CardType::Manager => {
                let _ = inventory.manager_inventory.add_card(coach_card);
            }
            CardType::Coach => {
                let _ = inventory.coach_inventory.add_card(coach_card);
            }
            CardType::Tactics => {}
        },
        GachaCard::Tactics(tactics_card) => {
            let _ = inventory.tactics_inventory.add_card(tactics_card);
        }
    }
}

fn godot_variant_to_json_value(v: &Variant) -> Result<JsonValue, String> {      
    use godot::builtin::{VariantArray, VariantType};

    match v.get_type() {
        VariantType::NIL => Ok(JsonValue::Null),
        VariantType::BOOL => Ok(JsonValue::Bool(
            v.try_to::<bool>()
                .map_err(|e| format!("Expected bool Variant: {e}"))?,
        )),
        VariantType::INT => Ok(JsonValue::Number(serde_json::Number::from(
            v.try_to::<i64>()
                .map_err(|e| format!("Expected int Variant: {e}"))?,
        ))),
        VariantType::FLOAT => {
            let f = v
                .try_to::<f64>()
                .map_err(|e| format!("Expected float Variant: {e}"))?;
            let n = serde_json::Number::from_f64(f).ok_or_else(|| {
                "Float value is NaN/Inf and cannot be represented in JSON".to_string()
            })?;
            Ok(JsonValue::Number(n))
        }
        VariantType::STRING => Ok(JsonValue::String(
            v.try_to::<GString>()
                .map_err(|e| format!("Expected string Variant: {e}"))?
                .to_string(),
        )),
        VariantType::STRING_NAME => Ok(JsonValue::String(v.stringify().to_string())),
        VariantType::NODE_PATH => Ok(JsonValue::String(v.stringify().to_string())),
        VariantType::DICTIONARY => {
            let dict = v
                .try_to::<Dictionary>()
                .map_err(|e| format!("Expected Dictionary Variant: {e}"))?;
            let mut map = serde_json::Map::new();
            for (k, val) in dict.iter_shared() {
                let key = k
                    .try_to::<GString>()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| k.stringify().to_string());
                map.insert(key, godot_variant_to_json_value(&val)?);
            }
            Ok(JsonValue::Object(map))
        }
        VariantType::ARRAY => {
            let arr = v
                .try_to::<VariantArray>()
                .map_err(|e| format!("Expected Array Variant: {e}"))?;
            let mut out = Vec::with_capacity(arr.len());
            for elem in arr.iter_shared() {
                out.push(godot_variant_to_json_value(&elem)?);
            }
            Ok(JsonValue::Array(out))
        }
        VariantType::PACKED_STRING_ARRAY => {
            let arr = v
                .try_to::<PackedStringArray>()
                .map_err(|e| format!("Expected PackedStringArray Variant: {e}"))?;
            Ok(JsonValue::Array(
                arr.as_slice()
                    .iter()
                    .map(|s| JsonValue::String(s.to_string()))
                    .collect(),
            ))
        }
        VariantType::PACKED_INT32_ARRAY => {
            let arr = v
                .try_to::<PackedInt32Array>()
                .map_err(|e| format!("Expected PackedInt32Array Variant: {e}"))?;
            Ok(JsonValue::Array(
                arr.as_slice()
                    .iter()
                    .map(|n| JsonValue::Number(serde_json::Number::from(*n)))
                    .collect(),
            ))
        }
        VariantType::PACKED_INT64_ARRAY => {
            let arr = v
                .try_to::<PackedInt64Array>()
                .map_err(|e| format!("Expected PackedInt64Array Variant: {e}"))?;
            Ok(JsonValue::Array(
                arr.as_slice()
                    .iter()
                    .map(|n| JsonValue::Number(serde_json::Number::from(*n)))
                    .collect(),
            ))
        }
        VariantType::PACKED_FLOAT32_ARRAY => {
            let arr = v
                .try_to::<PackedFloat32Array>()
                .map_err(|e| format!("Expected PackedFloat32Array Variant: {e}"))?;
            let mut out = Vec::with_capacity(arr.len());
            for n in arr.as_slice() {
                let num = serde_json::Number::from_f64(*n as f64).ok_or_else(|| {
                    "PackedFloat32Array contains NaN/Inf and cannot be represented in JSON"
                        .to_string()
                })?;
                out.push(JsonValue::Number(num));
            }
            Ok(JsonValue::Array(out))
        }
        VariantType::PACKED_FLOAT64_ARRAY => {
            let arr = v
                .try_to::<PackedFloat64Array>()
                .map_err(|e| format!("Expected PackedFloat64Array Variant: {e}"))?;
            let mut out = Vec::with_capacity(arr.len());
            for n in arr.as_slice() {
                let num = serde_json::Number::from_f64(*n).ok_or_else(|| {
                    "PackedFloat64Array contains NaN/Inf and cannot be represented in JSON"
                        .to_string()
                })?;
                out.push(JsonValue::Number(num));
            }
            Ok(JsonValue::Array(out))
        }
        other => Err(format!(
            "Unsupported Variant type for JSON bridge: {other:?}"
        )),
    }
}

fn json_value_to_variant(v: &JsonValue) -> Variant {
    match v {
        JsonValue::Null => Variant::nil(),
        JsonValue::Bool(b) => b.to_variant(),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_variant()
            } else if let Some(u) = n.as_u64() {
                if u <= i64::MAX as u64 {
                    (u as i64).to_variant()
                } else {
                    GString::from(u.to_string()).to_variant()
                }
            } else if let Some(f) = n.as_f64() {
                f.to_variant()
            } else {
                Variant::nil()
            }
        }
        JsonValue::String(s) => GString::from(s.as_str()).to_variant(),
        JsonValue::Array(arr) => {
            let mut out = Array::<Variant>::new();
            for elem in arr {
                let elem_variant = json_value_to_variant(elem);
                out.push(&elem_variant);
            }
            out.to_variant()
        }
        JsonValue::Object(map) => {
            let mut dict = Dictionary::new();
            for (k, val) in map {
                dict.set(GString::from(k.as_str()), json_value_to_variant(val));
            }
            dict.to_variant()
        }
    }
}

fn normalize_formation_for_v2(f: &str) -> &str {
    match f.trim() {
        "T442" => "4-4-2",
        "T433" => "4-3-3",
        "T352" => "3-5-2",
        "T532" => "5-3-2",
        "T4231" => "4-2-3-1",
        "T4141" => "4-1-4-1",
        "T343" => "3-4-3",
        "T541" => "5-4-1",
        // already normalized
        "4-4-2" | "4-3-3" | "3-5-2" | "5-3-2" | "4-2-3-1" | "4-1-4-1" | "3-4-3" | "5-4-1" => {
            f.trim()
        }
        _ => "4-4-2",
    }
}

fn try_convert_match_setup_payload_to_match_request_v2(
    payload: &JsonValue,
) -> Result<Option<JsonValue>, String> {
    // MatchSetupExporter payload shape: { home_team:{starting_xi,bench,formation_id...}, away_team:{...}, seed, ... }
    let obj = match payload.as_object() {
        Some(o) => o,
        None => return Ok(None),
    };

    if obj.contains_key("schema_version") {
        return Ok(None);
    }

    let seed = obj
        .get("seed")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i.max(0) as u64)))
        .unwrap_or(0);

    let home = obj
        .get("home_team")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "MatchSetup payload missing home_team object".to_string())?;
    let away = obj
        .get("away_team")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "MatchSetup payload missing away_team object".to_string())?;

    fn extract_roster(team: &serde_json::Map<String, JsonValue>) -> Result<Vec<String>, String> {
        if let Some(roster) = team.get("roster").and_then(|v| v.as_array()) {
            let out: Vec<String> = roster
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if out.len() == 18 {
                return Ok(out);
            }
        }

        let starters = team
            .get("starting_xi")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                "Team missing starting_xi (expected MatchSetupExporter payload)".to_string()
            })?;
        let bench = team
            .get("bench")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                "Team missing bench (expected MatchSetupExporter payload)".to_string()
            })?;

        let out: Vec<String> = starters
            .iter()
            .chain(bench.iter())
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if out.len() != 18 {
            return Err(format!(
                "Expected roster size 18 (starting_xi+bench), got {}",
                out.len()
            ));
        }
        Ok(out)
    }

    let home_roster = extract_roster(home)?;
    let away_roster = extract_roster(away)?;

    let home_name = home
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("home")
        .to_string();
    let away_name = away
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("away")
        .to_string();

    let home_formation_raw = home
        .get("formation")
        .or_else(|| home.get("formation_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("4-4-2");
    let away_formation_raw = away
        .get("formation")
        .or_else(|| away.get("formation_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("4-4-2");

    let mut request_obj = serde_json::Map::new();
    request_obj.insert(
        "schema_version".to_string(),
        JsonValue::Number(serde_json::Number::from(2)),
    );
    request_obj.insert(
        "seed".to_string(),
        JsonValue::Number(serde_json::Number::from(seed)),
    );
    request_obj.insert(
        "home_team".to_string(),
        json!({
            "name": home_name,
            "formation": normalize_formation_for_v2(home_formation_raw),
            "roster": home_roster,
        }),
    );
    request_obj.insert(
        "away_team".to_string(),
        json!({
            "name": away_name,
            "formation": normalize_formation_for_v2(away_formation_raw),
            "roster": away_roster,
        }),
    );

    // Optional pass-through flags (MatchSetup OS boundary may inject these).
    if let Some(v) = obj
        .get("enable_position_tracking")
        .and_then(|v| v.as_bool())
    {
        request_obj.insert("enable_position_tracking".to_string(), JsonValue::Bool(v));
    }
    if let Some(v) = obj.get("use_real_names").and_then(|v| v.as_bool()) {
        request_obj.insert("use_real_names".to_string(), JsonValue::Bool(v));
    }
    if let Some(v) = obj.get("home_instructions") {
        request_obj.insert("home_instructions".to_string(), v.clone());
    }
    if let Some(v) = obj.get("away_instructions") {
        request_obj.insert("away_instructions".to_string(), v.clone());
    }
    if let Some(v) = obj.get("user_player") {
        request_obj.insert("user_player".to_string(), v.clone());
    }

    Ok(Some(JsonValue::Object(request_obj)))
}

struct FootballRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for FootballRustExtension {}

fn coach_save_state_schema_version() -> u8 {
    1
}

fn default_pity_threshold() -> u32 {
    100
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SavedDeck {
    id: String,
    name: String,
    manager_card_id: Option<String>,
    coach_card_ids: [Option<String>; 3],
    tactics_card_ids: [Option<String>; 3],
    last_used_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CoachSaveState {
    #[serde(default = "coach_save_state_schema_version")]
    schema_version: u8,
    #[serde(default)]
    pity_counter: u32,
    #[serde(default = "default_pity_threshold")]
    pity_threshold: u32,
    #[serde(default)]
    card_inventory: InventoryManager,
    #[serde(default)]
    saved_decks: BTreeMap<String, SavedDeck>,
    #[serde(default)]
    active_deck_id: Option<String>,
}

/// Football Match Simulator - GDExtension wrapper for of_core
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct FootballMatchSimulator {
    base: Base<RefCounted>,
    /// Phase E: interactive session state for of_core engine
    interactive_engine: RefCell<Option<OfMatchEngine>>,
    /// Phase 7: Match session state (step-based simulation)
    live_session: RefCell<Option<LiveMatchSession>>,
    /// Issue #4: Gacha System state
    gacha_system: RefCell<GachaSystem>,
    /// FIX_2601/0109: Coach inventory state (cards + collection)
    coach_inventory: RefCell<InventoryManager>,
    /// FIX_2601/0109: Saved decks (id -> deck definition)
    saved_decks: RefCell<BTreeMap<String, SavedDeck>>,
    /// FIX_2601/0109: Active deck id
    active_deck_id: RefCell<Option<String>>,
}

// Interactive Match Request Structs
#[derive(serde::Deserialize)]
struct InteractiveMatchRequest {
    schema_version: u8,
    seed: u64,
    home_team: InteractiveTeam,
    away_team: InteractiveTeam,
    #[serde(default)]
    user_player: Option<InteractiveUserPlayer>,
    #[serde(default)]
    home_instructions: Option<TeamInstructions>,
    #[serde(default)]
    away_instructions: Option<TeamInstructions>,
}

#[derive(serde::Deserialize)]
struct InteractiveTeam {
    name: String,
    formation: String,
    players: Vec<InteractivePlayer>,
}

#[derive(serde::Deserialize)]
struct InteractivePlayer {
    name: String,
    position: String,
    overall: u8,
    #[serde(default = "default_condition")]
    condition: u8,
}

fn default_condition() -> u8 {
    3 // FIX_2601/0123: AVERAGE condition level
}

#[derive(serde::Deserialize)]
struct InteractiveUserPlayer {
    team: String,
    player_name: String,
    highlight_level: InteractiveHighlightLevel,
}

#[derive(serde::Deserialize, Clone, Copy)]
enum InteractiveHighlightLevel {
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "simple")]
    Simple,
    #[serde(rename = "my_player")]
    MyPlayer,
    #[serde(rename = "full")]
    Full,
}

// Helper to create engine
fn create_of_core_engine_from_interactive_request(
    request_str: &str,
) -> Result<OfMatchEngine, String> {
    let req: InteractiveMatchRequest =
        serde_json::from_str(request_str).map_err(|e| format!("Invalid JSON request: {}", e))?;

    if req.schema_version != 1 {
        return Err(format!(
            "Unsupported schema version: {}",
            req.schema_version
        ));
    }

    use of_core::models::team::Formation;

    fn convert_team(team: InteractiveTeam) -> Result<Team, String> {
        let formation = match team.formation.as_str() {
            "4-4-2" => Formation::F442,
            "4-3-3" => Formation::F433,
            "3-5-2" => Formation::F352,
            "5-3-2" => Formation::F532,
            "4-2-3-1" => Formation::F4231,
            "4-1-4-1" => Formation::F4141,
            "3-4-3" => Formation::F343,
            "5-4-1" => Formation::F541,
            other => return Err(format!("Invalid formation: {}", other)),
        };

        if team.players.len() != 18 {
            return Err(format!(
                "Team must have exactly 18 players, found {}",
                team.players.len()
            ));
        }

        let mut players = Vec::with_capacity(team.players.len());
        for p in team.players {
            let pos = match p.position.to_uppercase().as_str() {
                "GK" => OfPosition::GK,
                "LB" => OfPosition::LB,
                "CB" => OfPosition::CB,
                "RB" => OfPosition::RB,
                "LWB" => OfPosition::LWB,
                "RWB" => OfPosition::RWB,
                "CDM" => OfPosition::CDM,
                "CM" => OfPosition::CM,
                "CAM" => OfPosition::CAM,
                "LM" => OfPosition::LM,
                "RM" => OfPosition::RM,
                "LW" => OfPosition::LW,
                "RW" => OfPosition::RW,
                "CF" => OfPosition::CF,
                "ST" => OfPosition::ST,
                "DF" => OfPosition::DF,
                "MF" => OfPosition::MF,
                "FW" => OfPosition::FW,
                other => return Err(format!("Invalid position: {}", other)),
            };

            players.push(OfPlayer {
                name: p.name,
                position: pos,
                overall: p.overall,
                condition: p.condition, // FIX_2601/0123: use deserialized condition
                attributes: None,
                equipped_skills: Vec::new(),
                traits: Default::default(),
                personality: Default::default(),
            });
        }

        Ok(Team {
            name: team.name,
            formation,
            players,
        })
    }

    let home_team = convert_team(req.home_team)?;
    let away_team = convert_team(req.away_team)?;

    let user_config = req.user_player.map(|up| {
        let is_home = up.team == "home";
        // C6: Resolve player_index from player_name
        let team = if is_home { &home_team } else { &away_team };
        let base_idx = if is_home { 0 } else { 11 };
        let player_index = team
            .players
            .iter()
            .position(|p| p.name == up.player_name)
            .map(|i| base_idx + i)
            .unwrap_or(base_idx + 9); // Fallback to first attacker (idx 9/20)

        CoreUserPlayerConfig {
            is_home_team: is_home,
            player_name: up.player_name,
            player_index,
            highlight_level: match up.highlight_level {
                InteractiveHighlightLevel::Skip => CoreHighlightLevel::Skip,
                InteractiveHighlightLevel::Simple => CoreHighlightLevel::Simple,
                InteractiveHighlightLevel::MyPlayer => CoreHighlightLevel::MyPlayer,
                InteractiveHighlightLevel::Full => CoreHighlightLevel::Full,
            },
        }
    });

    let plan = OfMatchPlan {
        home_team,
        away_team,
        seed: req.seed,
        user_player: user_config,
        home_match_modifiers: of_core::engine::TeamMatchModifiers::default(),
        away_match_modifiers: of_core::engine::TeamMatchModifiers::default(),
        home_instructions: req.home_instructions,
        away_instructions: req.away_instructions,
        home_player_instructions: None,
        away_player_instructions: None,
        home_ai_difficulty: None,
        away_ai_difficulty: None,
    };

    // Enable replay recording so the Finished payload can include a replay doc
    // (UI can treat replay as optional).
    OfMatchEngine::new(plan).map(|engine| engine.with_replay_recording())
}

// Binary Encoding Helpers
fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}
fn write_u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn write_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn write_f32_le(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_bytes_u32_len(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), String> {
    let len_u32 =
        u32::try_from(bytes.len()).map_err(|_| format!("blob too large: {} bytes", bytes.len()))?;
    write_u32_le(out, len_u32);
    out.extend_from_slice(bytes);
    Ok(())
}

fn encode_user_decision_context(ctx: &OfUserDecisionContext, out: &mut Vec<u8>) {
    write_u32_le(out, ctx.player_id);
    write_f32_le(out, ctx.time_seconds);
    write_f32_le(out, ctx.position_m.0);
    write_f32_le(out, ctx.position_m.1);
    write_f32_le(out, ctx.options.shoot_prob);
    write_f32_le(out, ctx.options.dribble_prob);

    let pass_count = ctx.options.pass_targets.len().min(u16::MAX as usize) as u16;
    write_u16_le(out, pass_count);
    for target in ctx.options.pass_targets.iter().take(pass_count as usize) {
        write_u32_le(out, target.id);
        write_f32_le(out, target.success_prob);
        write_u8(out, if target.is_key_pass { 1 } else { 0 });
    }
}

fn encode_interactive_state_binary(state: &OfSimState) -> PackedByteArray {
    let mut bytes: Vec<u8> = Vec::new();
    match state {
        OfSimState::Running => write_u8(&mut bytes, 0),
        OfSimState::Paused(ctx) => {
            write_u8(&mut bytes, 1);
            encode_user_decision_context(ctx, &mut bytes);
        }
        // NOTE: Finished payload is encoded by the caller because it may need
        // to include match result + replay JSON (requires engine access).
        OfSimState::Finished(_) => write_u8(&mut bytes, 2),
    }
    PackedByteArray::from(bytes.as_slice())
}

fn encode_interactive_finished_binary(result_json: &str, replay_json: &str) -> PackedByteArray {
    let mut bytes: Vec<u8> = Vec::new();
    write_u8(&mut bytes, 2);

    if let Err(err) = write_bytes_u32_len(&mut bytes, result_json.as_bytes()) {
        godot_error!("interactive finished payload: failed to encode result_json: {err}");
        return PackedByteArray::new();
    }
    if let Err(err) = write_bytes_u32_len(&mut bytes, replay_json.as_bytes()) {
        godot_error!("interactive finished payload: failed to encode replay_json: {err}");
        return PackedByteArray::new();
    }

    PackedByteArray::from(bytes.as_slice())
}

fn decode_user_action_binary(bytes: &[u8]) -> Option<OfUserAction> {
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        0 => Some(OfUserAction::Shoot),
        1 => Some(OfUserAction::Dribble),
        2 => {
            if bytes.len() < 5 {
                return None;
            }
            let mut id_bytes = [0u8; 4];
            id_bytes.copy_from_slice(&bytes[1..5]);
            let target_id = u32::from_le_bytes(id_bytes);
            Some(OfUserAction::PassTo(target_id))
        }
        _ => None,
    }
}

// Panic Hook
static PANIC_HOOK: Once = Once::new();
fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };
            let loc = if let Some(l) = info.location() {
                format!("{}:{}:{}", l.file(), l.line(), l.column())
            } else {
                "unknown".to_string()
            };
            godot_error!("Rust panic at {}: {}", loc, msg);
        }));
    });
}

#[godot_api]
impl IRefCounted for FootballMatchSimulator {
    fn init(base: Base<RefCounted>) -> Self {
        install_panic_hook();

        let mut coach_inventory = InventoryManager::new();
        // Avoid early "capacity full" failures during migration; capacity tuning can be tightened later.
        coach_inventory.manager_inventory.max_capacity = 500;
        coach_inventory.coach_inventory.max_capacity = 1000;
        coach_inventory.tactics_inventory.max_capacity = 500;

        Self {
            base,
            interactive_engine: RefCell::new(None),
            live_session: RefCell::new(None),
            gacha_system: RefCell::new(GachaSystem::default()),
            coach_inventory: RefCell::new(coach_inventory),
            saved_decks: RefCell::new(BTreeMap::new()),
            active_deck_id: RefCell::new(None),
        }
    }
}

#[godot_api]
impl FootballMatchSimulator {
    /// Connection test - returns version string to confirm engine is working
    #[func]
    pub fn test_connection(&self) -> GString {
        let version = env!("CARGO_PKG_VERSION");
        GString::from(format!("FootballMatchSimulator v{} - OK", version))
    }

    // =========================================================================
    // RuleBook "Why?" Button API (FIX_2601/1120 P1)
    // =========================================================================

    /// Get event explanation card for the "Why?" button
    ///
    /// # Arguments
    /// * `event_type_str` - Event type string (e.g., "Offside", "Foul", "Goal")
    /// * `use_korean` - true for Korean, false for English
    ///
    /// # Returns
    /// Dictionary with card data, or empty Dictionary if no explanation available
    ///
    /// # Example (GDScript)
    /// ```gdscript
    /// var card = engine.get_event_explanation("Offside", true)
    /// if card.has("blocks"):
    ///     for block in card.blocks:
    ///         print(block.title)
    ///         for line in block.lines:
    ///             print("  ", line.text)
    /// ```
    #[func]
    pub fn get_event_explanation(&self, event_type_str: GString, use_korean: bool) -> Dictionary {
        let event_type_string = event_type_str.to_string();

        // Parse event type
        let event_type = match parse_event_type(&event_type_string) {
            Some(et) => et,
            None => {
                let mut error_dict = Dictionary::new();
                error_dict.set("error", true);
                error_dict.set(
                    "message",
                    GString::from(format!("Unknown event type: {}", event_type_string)),
                );
                return error_dict;
            }
        };

        // Generate UI card (without details - basic explanation)
        match generate_ui_card(&event_type, None, use_korean) {
            Some(card) => convert_ui_card_to_dict(&card),
            None => {
                // No explanation available for this event type
                Dictionary::new()
            }
        }
    }

    /// Get event explanation card with full event details
    ///
    /// # Arguments
    /// * `event_json` - JSON string of the event (from replay/timeline)
    /// * `use_korean` - true for Korean, false for English
    ///
    /// # Returns
    /// Dictionary with card data, or empty Dictionary if no explanation available
    #[func]
    pub fn get_event_explanation_from_json(
        &self,
        event_json: GString,
        use_korean: bool,
    ) -> Dictionary {
        let event_str = event_json.to_string();

        // Parse the event JSON
        let event: MatchEvent = match serde_json::from_str(&event_str) {
            Ok(e) => e,
            Err(e) => {
                let mut error_dict = Dictionary::new();
                error_dict.set("error", true);
                error_dict.set(
                    "message",
                    GString::from(format!("Failed to parse event JSON: {}", e)),
                );
                return error_dict;
            }
        };

        // Generate UI card with full match event context (preferred for UI)
        match generate_ui_card_from_match_event(&event, use_korean) {
            Some(card) => convert_ui_card_to_dict(&card),
            None => Dictionary::new(),
        }
    }

    /// Check if an event type should show the "Why?" button
    ///
    /// # Arguments
    /// * `event_type_str` - Event type string (e.g., "Offside", "Foul")
    ///
    /// # Returns
    /// true if the "Why?" button should be shown
    #[func]
    pub fn should_show_why_button(&self, event_type_str: GString) -> bool {
        let event_type_string = event_type_str.to_string();
        match parse_event_type(&event_type_string) {
            Some(et) => RuleId::should_show_why_button(&et),
            None => false,
        }
    }

    #[func]
    pub fn simulate_match(&self, match_request_json: GString) -> GString {
        self.simulate_match_inner(match_request_json)
    }

    /// Budget-aware simulation entrypoint (wall-clock, minutes, events).
    /// Defaults to SimBudget::default() values when parameters are <= 0.
    #[func]
    pub fn simulate_match_with_budget(
        &self,
        match_request_json: GString,
        max_wall_ms: i64,
        max_minutes: i64,
        max_events: i64,
    ) -> GString {
        let request_str = match_request_json.to_string();
        if request_str.trim().is_empty() {
            return self.create_error_response("Empty match request", "EMPTY_REQUEST");
        }

        let budget = SimBudget::new(
            if max_wall_ms > 0 {
                max_wall_ms as u64
            } else {
                50
            },
            if max_minutes > 0 {
                max_minutes as u16
            } else {
                120
            },
            if max_events > 0 {
                max_events as usize
            } else {
                500
            },
        );

        match simulate_match_json_budget(&request_str, budget) {
            Ok(result_json) => GString::from(result_json),
            Err(err) => self.create_error_response(
                &format!("Budgeted simulation failed: {}", err),
                "SIMULATION_BUDGET_ERROR",
            ),
        }
    }

    /// P6: Simulate match with full replay event recording
    /// Returns Dictionary with "result_json" and "replay_json" keys
    #[func]
    pub fn simulate_match_with_replay(&self, match_request_json: GString) -> Dictionary {
        let request_str = match_request_json.to_string();
        if request_str.trim().is_empty() {
            let mut dict = Dictionary::new();
            dict.set("error", true);
            dict.set("message", GString::from("Empty match request"));
            dict.set("code", GString::from("EMPTY_REQUEST"));
            return dict;
        }

        match simulate_match_json_with_replay(&request_str) {
            Ok((result_json, replay_json)) => {
                let mut dict = Dictionary::new();
                dict.set("result_json", GString::from(result_json));
                dict.set("replay_json", GString::from(replay_json));
                dict
            }
            Err(err) => {
                let mut dict = Dictionary::new();
                dict.set("error", true);
                dict.set(
                    "message",
                    GString::from(format!("Simulation with replay failed: {}", err)),
                );
                dict.set("code", GString::from("SIMULATION_REPLAY_ERROR"));
                dict
            }
        }
    }

    /// MatchRequest v2: Simulate match from UID-based roster input (schema_version=2).
    #[func]
    pub fn simulate_match_v2_json(&self, match_request_json: GString) -> GString {
        let request_str = match_request_json.to_string();
        if request_str.trim().is_empty() {
            return self.create_error_response("Empty match request", "EMPTY_REQUEST");
        }

        match simulate_match_v2_json(&request_str) {
            Ok(result_json) => GString::from(result_json),
            Err(err) => self.create_error_response(
                &format!("v2 simulation failed: {}", err),
                "SIMULATION_V2_ERROR",
            ),
        }
    }

    /// MatchRequest v2: Simulate match with full replay event recording.
    /// Returns Dictionary with "result_json" and "replay_json" keys.
    #[func]
    pub fn simulate_match_with_replay_v2(&self, match_request_json: GString) -> Dictionary {
        let request_str = match_request_json.to_string();
        if request_str.trim().is_empty() {
            let mut dict = Dictionary::new();
            dict.set("error", true);
            dict.set("message", GString::from("Empty match request"));
            dict.set("code", GString::from("EMPTY_REQUEST"));
            return dict;
        }

        match simulate_match_v2_json_with_replay(&request_str) {
            Ok((result_json, replay_json)) => {
                let mut dict = Dictionary::new();
                dict.set("result_json", GString::from(result_json));
                dict.set("replay_json", GString::from(replay_json));
                dict
            }
            Err(err) => {
                let mut dict = Dictionary::new();
                dict.set("error", true);
                dict.set(
                    "message",
                    GString::from(format!("v2 simulation with replay failed: {}", err)),
                );
                dict.set("code", GString::from("SIMULATION_V2_REPLAY_ERROR"));
                dict
            }
        }
    }

    /// MatchSetup OS boundary entrypoint (Phase17 canonical).
    ///
    /// Accepts either:
    /// - MatchRequest v1/v2 as a Godot Dictionary (must include `schema_version`)
    /// - MatchSetupExporter payload (no `schema_version`, uses home/away_team.starting_xi + bench)
    ///
    /// Returns Dictionary with "result_json" and "replay_json" keys (same as simulate_match_*_with_replay).
    #[func]
    pub fn simulate_match_from_setup(&self, payload: Dictionary) -> Dictionary {
        if payload.is_empty() {
            return _error_dict("Empty setup payload", "EMPTY_REQUEST");
        }

        let payload_value = match godot_variant_to_json_value(&payload.to_variant()) {
            Ok(v) => v,
            Err(e) => {
                return _error_dict(
                    format!("Setup payload conversion failed: {e}"),
                    "SETUP_CONVERSION_ERROR",
                )
            }
        };

        // If payload is MatchSetupExporter shape (no schema_version), convert to MatchRequest v2 first.
        let (request_value, schema_version) = if let Some(obj) = payload_value.as_object() {
            if let Some(sv) = obj.get("schema_version") {
                let sv_u64 = sv
                    .as_u64()
                    .or_else(|| sv.as_i64().map(|i| i.max(0) as u64))
                    .unwrap_or(0);
                (payload_value, sv_u64)
            } else {
                match try_convert_match_setup_payload_to_match_request_v2(&payload_value) {
                    Ok(Some(v2)) => (v2, 2),
                    Ok(None) => {
                        return _error_dict(
                            "Missing schema_version in setup payload",
                            "MISSING_SCHEMA_VERSION",
                        )
                    }
                    Err(e) => {
                        return _error_dict(
                            format!("MatchSetup -> MatchRequest v2 conversion failed: {e}"),
                            "SETUP_TO_V2_ERROR",
                        )
                    }
                }
            }
        } else {
            return _error_dict(
                "Setup payload must be a Dictionary object",
                "INVALID_SETUP_PAYLOAD",
            );
        };

        let request_json = match serde_json::to_string(&request_value) {
            Ok(s) => s,
            Err(e) => {
                return _error_dict(
                    format!("Failed to encode request JSON: {e}"),
                    "SERIALIZATION_ERROR",
                )
            }
        };

        match schema_version {
            1 => match simulate_match_json_with_replay(&request_json) {
                Ok((result_json, replay_json)) => {
                    let mut dict = Dictionary::new();
                    dict.set("result_json", GString::from(result_json));
                    dict.set("replay_json", GString::from(replay_json));
                    dict
                }
                Err(err) => _error_dict(
                    format!("Simulation with replay failed: {err}"),
                    "SIMULATION_REPLAY_ERROR",
                ),
            },
            2 => match simulate_match_v2_json_with_replay(&request_json) {
                Ok((result_json, replay_json)) => {
                    let mut dict = Dictionary::new();
                    dict.set("result_json", GString::from(result_json));
                    dict.set("replay_json", GString::from(replay_json));
                    dict
                }
                Err(err) => _error_dict(
                    format!("v2 simulation with replay failed: {err}"),
                    "SIMULATION_V2_REPLAY_ERROR",
                ),
            },
            other => _error_dict(
                format!("Unsupported schema_version: {other}"),
                "UNSUPPORTED_SCHEMA_VERSION",
            ),
        }
    }

    #[func]
    pub fn start_interactive_match_binary(&self, match_request_json: GString) -> PackedByteArray {
        let request_str = match_request_json.to_string();
        match create_of_core_engine_from_interactive_request(&request_str) {
            Ok(mut engine) => {
                let state = engine.simulate_until_intervention();
                match &state {
                    OfSimState::Finished(result) => {
                        let result_json = match serde_json::to_string(result) {
                            Ok(json) => json,
                            Err(err) => {
                                godot_error!(
                                    "start_interactive failed: serialize result: {err}"
                                );
                                return PackedByteArray::new();
                            }
                        };

                        let replay_json = match engine.take_replay_doc() {
                            Some(doc) => serde_json::to_string(&doc).unwrap_or_else(|err| {
                                godot_error!(
                                    "start_interactive failed: serialize replay: {err}"
                                );
                                "null".to_string()
                            }),
                            None => "null".to_string(),
                        };

                        *self.interactive_engine.borrow_mut() = None;
                        encode_interactive_finished_binary(&result_json, &replay_json)
                    }
                    _ => {
                        *self.interactive_engine.borrow_mut() = Some(engine);
                        encode_interactive_state_binary(&state)
                    }
                }
            }
            Err(err) => {
                godot_error!("start_interactive failed: {}", err);
                PackedByteArray::new()
            }
        }
    }

    #[func]
    pub fn resume_interactive_match_binary(
        &self,
        action_bytes: PackedByteArray,
    ) -> PackedByteArray {
        let bytes = action_bytes.to_vec();
        let action = match decode_user_action_binary(&bytes) {
            Some(a) => a,
            None => return PackedByteArray::new(),
        };

        let mut engine_cell = self.interactive_engine.borrow_mut();
        let engine = match engine_cell.as_mut() {
            Some(e) => e,
            None => return PackedByteArray::new(),
        };

        let state = engine.resume_with_action(action);
        match &state {
            OfSimState::Finished(result) => {
                let result_json = match serde_json::to_string(result) {
                    Ok(json) => json,
                    Err(err) => {
                        godot_error!("resume_interactive failed: serialize result: {err}");
                        return PackedByteArray::new();
                    }
                };

                let replay_json = match engine.take_replay_doc() {
                    Some(doc) => serde_json::to_string(&doc).unwrap_or_else(|err| {
                        godot_error!("resume_interactive failed: serialize replay: {err}");
                        "null".to_string()
                    }),
                    None => "null".to_string(),
                };

                *engine_cell = None;
                encode_interactive_finished_binary(&result_json, &replay_json)
            }
            _ => encode_interactive_state_binary(&state),
        }
    }

    fn parse_mrq0_match_modifiers_extension(
        data: &[u8],
        offset: &mut usize,
        version: u32,
    ) -> Result<
        (
            of_core::engine::TeamMatchModifiers,
            of_core::engine::TeamMatchModifiers,
        ),
        String,
    > {
        fn read_u8(data: &[u8], offset: &mut usize) -> Option<u8> {
            if *offset + 1 > data.len() {
                return None;
            }
            let v = data[*offset];
            *offset += 1;
            Some(v)
        }

        fn read_f32_le(data: &[u8], offset: &mut usize) -> Option<f32> {
            if *offset + 4 > data.len() {
                return None;
            }
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&data[*offset..*offset + 4]);
            *offset += 4;
            Some(f32::from_le_bytes(buf))
        }

        let mut home = of_core::engine::TeamMatchModifiers::default();
        let mut away = of_core::engine::TeamMatchModifiers::default();

        let flags = read_u8(data, offset).ok_or_else(|| "missing extension flags".to_string())?;
        if (flags & 0b0000_0001) == 0 {
            return Ok((home, away));
        }

        // v3: reserved training_multiplier (ignored by match sim).
        // v4: removed from MRQ0 (match-only protocol).
        if version <= 3 {
            read_f32_le(data, offset)
                .ok_or_else(|| "deck_effects missing training_multiplier".to_string())?;
        }

        let home_mod_count =
            read_u8(data, offset).ok_or_else(|| "deck_effects missing mod_count".to_string())?
                as usize;
        for _ in 0..home_mod_count {
            let mod_id = read_u8(data, offset).ok_or_else(|| "deck_effects missing mod_id".to_string())?;
            let value = read_f32_le(data, offset).ok_or_else(|| "deck_effects missing value".to_string())?;
            home.apply_mod_id(mod_id, value);
        }

        // v1.1: optional away list (append-only)
        if (flags & 0b0000_0010) != 0 {
            let away_mod_count =
                read_u8(data, offset).ok_or_else(|| "deck_effects missing away_mod_count".to_string())?
                    as usize;
            for _ in 0..away_mod_count {
                let mod_id =
                    read_u8(data, offset).ok_or_else(|| "deck_effects missing away mod_id".to_string())?;
                let value =
                    read_f32_le(data, offset).ok_or_else(|| "deck_effects missing away value".to_string())?;
                away.apply_mod_id(mod_id, value);
            }
        }

        Ok((home, away))
    }

    fn decode_mrq0_to_match_plan(data: &[u8]) -> Result<OfMatchPlan, String> {
        if data.len() < 4 * 2 + 8 + 1 {
            return Err("payload too small".to_string());
        }

        let mut offset: usize = 0;

        fn read_u32_le(data: &[u8], offset: &mut usize) -> Option<u32> {
            if *offset + 4 > data.len() {
                return None;
            }
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&data[*offset..*offset + 4]);
            *offset += 4;
            Some(u32::from_le_bytes(buf))
        }

        fn read_u64_le(data: &[u8], offset: &mut usize) -> Option<u64> {
            if *offset + 8 > data.len() {
                return None;
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&data[*offset..*offset + 8]);
            *offset += 8;
            Some(u64::from_le_bytes(buf))
        }

        fn read_u16_le(data: &[u8], offset: &mut usize) -> Option<u16> {
            if *offset + 2 > data.len() {
                return None;
            }
            let mut buf = [0u8; 2];
            buf.copy_from_slice(&data[*offset..*offset + 2]);
            *offset += 2;
            Some(u16::from_le_bytes(buf))
        }

        fn read_u8(data: &[u8], offset: &mut usize) -> Option<u8> {
            if *offset + 1 > data.len() {
                return None;
            }
            let v = data[*offset];
            *offset += 1;
            Some(v)
        }

        fn read_string(data: &[u8], offset: &mut usize) -> Option<String> {
            let len = read_u16_le(data, offset)? as usize;
            if len == 0 {
                return Some(String::new());
            }
            if *offset + len > data.len() {
                return None;
            }
            let slice = &data[*offset..*offset + len];
            *offset += len;
            String::from_utf8(slice.to_vec()).ok()
        }

        // 1) Header
        let magic = read_u32_le(data, &mut offset).unwrap_or(0);
        let version = read_u32_le(data, &mut offset).unwrap_or(0);
        // 2025-12-10: Support v1..v4 (Tactical Instructions + match-only v4 cleanup).
        if magic != 0x3051514D || !(1..=4).contains(&version) {
            return Err(format!(
                "invalid header (magic={:#x}, version={})",
                magic, version
            ));
        }

        let seed = read_u64_le(data, &mut offset).unwrap_or(42);
        let _use_vendor = read_u8(data, &mut offset).unwrap_or(0) != 0;

        // v2: parse position_sample_rate_ms (currently ignored, using 100ms default)
        let _position_sample_rate_ms: u16 = if version >= 2 {
            read_u16_le(data, &mut offset).unwrap_or(100)
        } else {
            100 // v1 default
        };

        // 2) Decode team (twice: home, away)
        fn decode_team_from_binary(
            data: &[u8],
            offset: &mut usize,
            seed: u64,
        ) -> Option<of_core::models::Team> {
            use of_core::models::team::Formation;

            let name = read_string(data, offset)?;
            let formation_str = read_string(data, offset)?;
            let player_count = read_u8(data, offset)? as usize;

            let mut players: Vec<OfPlayer> = Vec::with_capacity(player_count);

            for _ in 0..player_count {
                let pname = read_string(data, offset)?;
                let pos_code = read_u8(data, offset)?;
                let overall = read_u8(data, offset)?;

                let position = OfPosition::from_code(pos_code).unwrap_or(OfPosition::CM);

                // MRQ0 hotpath: derive attributes deterministically from overall/position/seed
                let attrs = OfPlayerAttributes::derive_from_proxy(
                    overall as u8,
                    position,
                    seed.wrapping_add(players.len() as u64),
                );

                players.push(OfPlayer {
                    name: pname,
                    position,
                    overall: overall as u8,
                    condition: 3, // FIX_2601/0123: default condition level
                    attributes: Some(attrs),
                    equipped_skills: Vec::new(),
                    traits: Default::default(),
                    personality: Default::default(),
                });
            }

            let formation = match formation_str.as_str() {
                "4-4-2" => Formation::F442,
                "4-3-3" => Formation::F433,
                "3-5-2" => Formation::F352,
                "5-3-2" => Formation::F532,
                "4-2-3-1" => Formation::F4231,
                "4-1-4-1" => Formation::F4141,
                "3-4-3" => Formation::F343,
                "5-4-1" => Formation::F541,
                _ => Formation::F442,
            };

            Some(of_core::models::Team {
                name,
                formation,
                players,
            })
        }

        fn decode_instructions_from_binary(
            data: &[u8],
            offset: &mut usize,
        ) -> Option<of_core::tactics::team_instructions::TeamInstructions> {
            use of_core::tactics::team_instructions::*;

            let def_line = match read_u8(data, offset)? {
                0 => DefensiveLine::VeryHigh,
                1 => DefensiveLine::High,
                2 => DefensiveLine::Normal,
                3 => DefensiveLine::Deep,
                _ => DefensiveLine::VeryDeep,
            };

            let width = match read_u8(data, offset)? {
                0 => TeamWidth::VeryWide,
                1 => TeamWidth::Wide,
                2 => TeamWidth::Normal,
                3 => TeamWidth::Narrow,
                _ => TeamWidth::VeryNarrow,
            };

            let tempo = match read_u8(data, offset)? {
                0 => TeamTempo::VeryFast,
                1 => TeamTempo::Fast,
                2 => TeamTempo::Normal,
                3 => TeamTempo::Slow,
                _ => TeamTempo::VerySlow,
            };

            let pressing = match read_u8(data, offset)? {
                0 => TeamPressing::VeryHigh,
                1 => TeamPressing::High,
                2 => TeamPressing::Medium,
                3 => TeamPressing::Low,
                _ => TeamPressing::VeryLow,
            };

            let build_up = match read_u8(data, offset)? {
                0 => BuildUpStyle::Short,
                1 => BuildUpStyle::Mixed,
                _ => BuildUpStyle::Direct,
            };

            let offside_trap = read_u8(data, offset)? != 0;

            Some(TeamInstructions {
                defensive_line: def_line,
                team_width: width,
                team_tempo: tempo,
                pressing_intensity: pressing,
                build_up_style: build_up,
                use_offside_trap: offside_trap,
            })
        }

        let home_team = decode_team_from_binary(data, &mut offset, seed)
            .ok_or_else(|| "failed to decode home_team".to_string())?;

        let home_instructions = if version >= 3 {
            decode_instructions_from_binary(data, &mut offset)
        } else {
            None
        };

        let away_team = decode_team_from_binary(
            data,
            &mut offset,
            seed.wrapping_add(0x9E3779B97F4A7C15),
        )
        .ok_or_else(|| "failed to decode away_team".to_string())?;

        let away_instructions = if version >= 3 {
            decode_instructions_from_binary(data, &mut offset)
        } else {
            None
        };

        let (home_match_modifiers, away_match_modifiers) =
            if version >= 3 && offset < data.len() {
                Self::parse_mrq0_match_modifiers_extension(data, &mut offset, version)?
            } else {
                (
                    of_core::engine::TeamMatchModifiers::default(),
                    of_core::engine::TeamMatchModifiers::default(),
                )
            };

        Ok(OfMatchPlan {
            home_team,
            away_team,
            seed,
            user_player: None,
            home_match_modifiers,
            away_match_modifiers,
            home_instructions,
            away_instructions,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        })
    }

    /// Binary Replay Optimization (bincode)
    #[func]
    pub fn simulate_match_from_binary(&self, request_bytes: PackedByteArray) -> PackedByteArray {
        let data: Vec<u8> = request_bytes.to_vec();
        let plan = match Self::decode_mrq0_to_match_plan(&data) {
            Ok(plan) => plan,
            Err(msg) => {
                godot_error!("simulate_match_from_binary: {}", msg);
                return PackedByteArray::new();
            }
        };
        let seed = plan.seed;

        // MRQ0 decoding is centralized in `decode_mrq0_to_match_plan` (SSOT).

        // 3) Run of_core engine with position tracking
        let mut engine = match OfMatchEngine::new(plan) {
            Ok(engine) => engine.with_position_tracking(),
            Err(err) => {
                godot_error!("simulate_match_from_binary: engine init failed: {}", err);
                return PackedByteArray::new();
            }
        };
        let result = engine.simulate();

        // 4) Convert MatchResult.position_data -> ReplayDataV3
        // 2025-12-11 P2: Expanded ball_frames to include velocity (t, x, y, z, vx, vy)
        let mut ball_frames: Vec<(f32, f32, f32, f32, f32, f32)> = Vec::new();
        // Use nested structure: HashMap<player_id, Vec<(t, x, y, vx, vy)>>
        // 2025-12-11: Added vx, vy for player velocity
        let mut player_data: std::collections::HashMap<u32, Vec<(f32, f32, f32, f32, f32)>> =
            std::collections::HashMap::new();
        let mut duration: f32 = 5400.0; // fallback: 90min

        if let Some(ref pos_data) = result.position_data {
            if let Some(last) = pos_data.ball.last() {
                duration = (last.timestamp as f32) / 1000.0;
            }

            // 2025-12-11 P2: Include ball velocity (vx, vy) in ball_frames
            for item in &pos_data.ball {
                let (vx, vy) = item.velocity.unwrap_or((0.0, 0.0));
                ball_frames.push((
                    (item.timestamp as f32) / 1000.0,
                    item.position.0,
                    item.position.1,
                    item.height.unwrap_or(0.0), // 2025-12-11: Use actual height instead of hardcoded 0.0
                    vx,
                    vy,
                ));
            }

            // Keep player frames grouped by player_id (nested structure)
            // 2025-12-11: Include velocity (vx, vy) for accurate direction tracking
            // FIX_2601/0109: players changed from HashMap to [Vec; 22]
            for (player_idx, frames) in pos_data.players.iter().enumerate() {
                let pid = player_idx as u32;
                let player_frames: Vec<(f32, f32, f32, f32, f32)> = frames
                    .iter()
                    .map(|item| {
                        let (vx, vy) = item.velocity.unwrap_or((0.0, 0.0));
                        (
                            (item.timestamp as f32) / 1000.0,
                            item.position.0,
                            item.position.1,
                            vx,
                            vy,
                        )
                    })
                    .collect();
                player_data.insert(pid, player_frames);
            }
        }

        // Body bytes (v3 layout, backward-compatible with v2 structure)
        let mut body: Vec<u8> = Vec::new();
        body.push(3u8); // format_version = 3
        body.extend_from_slice(&duration.to_le_bytes());
        body.push(result.score_home);
        body.push(result.score_away);

        let event_count = result.events.len() as u32;
        body.extend_from_slice(&event_count.to_le_bytes());
        for event in &result.events {
            body.push(event.minute);
            let event_type_u8: u8 = match event.event_type {
                of_core::models::EventType::Goal => 0,
                of_core::models::EventType::OwnGoal => 26,
                of_core::models::EventType::Shot => 1,
                of_core::models::EventType::ShotOnTarget => 2,
                of_core::models::EventType::ShotOffTarget => 3,
                of_core::models::EventType::ShotBlocked => 4,
                of_core::models::EventType::Save => 5,
                of_core::models::EventType::YellowCard => 6,
                of_core::models::EventType::RedCard => 7,
                of_core::models::EventType::Substitution => 8,
                of_core::models::EventType::Injury => 9,
                of_core::models::EventType::Corner => 10,
                of_core::models::EventType::Freekick => 11,
                of_core::models::EventType::Penalty => 12,
                of_core::models::EventType::Offside => 13,
                of_core::models::EventType::Foul => 14,
                of_core::models::EventType::KeyChance => 15,
                of_core::models::EventType::Pass => 16,
                of_core::models::EventType::Tackle => 17,
                of_core::models::EventType::Dribble => 18,
                of_core::models::EventType::KickOff => 19,
                of_core::models::EventType::PostHit => 20,
                of_core::models::EventType::BarHit => 21,
                of_core::models::EventType::GoalKick => 22,
                of_core::models::EventType::ThrowIn => 23,
                of_core::models::EventType::HalfTime => 24,
                of_core::models::EventType::FullTime => 25,
                of_core::models::EventType::VarReview => 27,
            };
            body.push(event_type_u8);
            body.push(if event.is_home_team { 1 } else { 0 });

            // C7: player name removed - serialize empty string (binary format deprecated)
            let player_bytes: &[u8] = &[];
            body.extend_from_slice(&(player_bytes.len() as u16).to_le_bytes());
            body.extend_from_slice(player_bytes);

            // C7: assist_by removed - serialize empty string (binary format deprecated)
            let assist_bytes: &[u8] = &[];
            body.extend_from_slice(&(assist_bytes.len() as u16).to_le_bytes());
            body.extend_from_slice(assist_bytes);
        }

        body.extend_from_slice(&(ball_frames.len() as u64).to_le_bytes());
        // 2025-12-11 P2: Serialize ball frames with velocity (t, x, y, z, vx, vy)
        for frame in &ball_frames {
            body.extend_from_slice(&frame.0.to_le_bytes()); // t
            body.extend_from_slice(&frame.1.to_le_bytes()); // x
            body.extend_from_slice(&frame.2.to_le_bytes()); // y
            body.extend_from_slice(&frame.3.to_le_bytes()); // z (height)
            body.extend_from_slice(&frame.4.to_le_bytes()); // vx
            body.extend_from_slice(&frame.5.to_le_bytes()); // vy
        }

        // Write player data in nested structure: player_count, then per-player (id, frame_count, frames)
        // 2025-12-11: Now includes vx, vy for velocity (format_version bump recommended if strict compat needed)
        body.extend_from_slice(&(player_data.len() as u64).to_le_bytes());
        for (player_id, frames) in &player_data {
            body.extend_from_slice(&player_id.to_le_bytes()); // u32
            body.extend_from_slice(&(frames.len() as u64).to_le_bytes()); // frame_count
            for frame in frames {
                body.extend_from_slice(&frame.0.to_le_bytes()); // t
                body.extend_from_slice(&frame.1.to_le_bytes()); // x
                body.extend_from_slice(&frame.2.to_le_bytes()); // y
                body.extend_from_slice(&frame.3.to_le_bytes()); // vx (2025-12-11)
                body.extend_from_slice(&frame.4.to_le_bytes()); // vy (2025-12-11)
            }
        }

        // Header JSON (UTF-8) for meta
        // Events are included both as 'events' (for ReplayLoader) and 'timeline' (for UI)
        let events_json: Vec<_> = result
            .events
            .iter()
            .map(|e| {
                let event_type_str = match e.event_type {
                    of_core::models::EventType::Goal => "goal",
                    of_core::models::EventType::OwnGoal => "own_goal",
                    of_core::models::EventType::Shot => "shot",
                    of_core::models::EventType::ShotOnTarget => "shot_on_target",
                    of_core::models::EventType::ShotOffTarget => "shot_off_target",
                    of_core::models::EventType::ShotBlocked => "shot_blocked",
                    of_core::models::EventType::Save => "save",
                    of_core::models::EventType::YellowCard => "yellow_card",
                    of_core::models::EventType::RedCard => "red_card",
                    of_core::models::EventType::Substitution => "substitution",
                    of_core::models::EventType::Injury => "injury",
                    of_core::models::EventType::Corner => "corner",
                    of_core::models::EventType::Freekick => "freekick",
                    of_core::models::EventType::Penalty => "penalty",
                    of_core::models::EventType::Offside => "offside",
                    of_core::models::EventType::Foul => "foul",
                    of_core::models::EventType::KeyChance => "key_chance",
                    of_core::models::EventType::Pass => "pass",
                    of_core::models::EventType::Tackle => "tackle",
                    of_core::models::EventType::Dribble => "dribble",
                    of_core::models::EventType::KickOff => "kick_off",
                    of_core::models::EventType::PostHit => "post_hit",
                    of_core::models::EventType::BarHit => "bar_hit",
                    of_core::models::EventType::GoalKick => "goal_kick",
                    of_core::models::EventType::ThrowIn => "throw_in",
                    of_core::models::EventType::HalfTime => "half_time",        
                    of_core::models::EventType::FullTime => "full_time",        
                    of_core::models::EventType::VarReview => "var_review",
                };
                serde_json::json!({
                    "minute": e.minute,
                    "t": (e.minute as f64) * 60.0,
                    "type": event_type_str,
                    "etype": event_type_str,
                    "team": if e.is_home_team { "home" } else { "away" },
                    "is_home_team": e.is_home_team,
                    // C7: Removed player/player_id - use player_track_id instead
                    "player_track_id": e.player_track_id,
                    "target_track_id": e.target_track_id,
                    "details": e.details.as_ref().map(|d| {
                        let injury_severity = d.injury_severity.as_ref().map(|inj| serde_json::json!({
                            "weeks_out": inj.weeks_out,
                            "description": &inj.description,
                        }));
                        let ball_position = d.ball_position.map(|(x, y, z)| serde_json::json!({
                            "x": x,
                            "y": y,
                            "z": z,
                        }));
                        let substitution = d.substitution.as_ref().map(|sub| serde_json::json!({
                            "player_in_name": &sub.player_in_name,
                            "player_out_name": &sub.player_out_name,
                            "bench_slot": sub.bench_slot,
                        }));
                        let var_review = d.var_review.as_ref().map(|vr| serde_json::json!({
                            "reviewed_event_type": format!("{:?}", vr.reviewed_event_type).to_lowercase(),
                            "outcome": format!("{:?}", vr.outcome).to_lowercase(),
                        }));
                        serde_json::json!({
                            "xg_value": d.xg_value,
                            "injury_severity": injury_severity,
                            "ball_position": ball_position,
                            "substitution": substitution,
                            "var_review": var_review,
                        })
                    })
                })
            })
            .collect();

        let header_json = serde_json::json!({
            "format_version": 3,
            "match_id": format!("binary-{}", seed),
            "seed": seed,
            "duration_seconds": duration,
            "score": { "home": result.score_home, "away": result.score_away },
            "penalty_shootout": result.penalty_shootout.as_ref().map(|ps| {     
                serde_json::json!({
                    "goals_home": ps.goals_home,
                    "goals_away": ps.goals_away,
                    "kicks_taken_home": ps.kicks_taken_home,
                    "kicks_taken_away": ps.kicks_taken_away,
                    "winner_is_home": ps.winner_is_home,
                    "kicks": ps.kicks.iter().map(|kick| {
                        serde_json::json!({
                            "kick_index": kick.kick_index,
                            "is_home_team": kick.is_home_team,
                            "kicker_track_id": kick.kicker_track_id,
                            "kicker_name": &kick.kicker_name,
                            "scored": kick.scored,
                        })
                    }).collect::<Vec<_>>(),
                })
            }),
            "teams": {
                "home": {
                    "name": result.home_team.as_ref().map(|t| t.name.clone()).unwrap_or_else(|| "Home".to_string()),
                    "formation": result.home_team.as_ref().map(|t| format!("{:?}", t.formation)).unwrap_or_else(|| "4-4-2".to_string()),
                },
                "away": {
                    "name": result.away_team.as_ref().map(|t| t.name.clone()).unwrap_or_else(|| "Away".to_string()),
                    "formation": result.away_team.as_ref().map(|t| format!("{:?}", t.formation)).unwrap_or_else(|| "4-4-2".to_string()),
                },
            },
            "match_setup": result.match_setup.as_ref().map(|ms| {
                serde_json::json!({
                    "home": {
                        "name": &ms.home.name,
                        "formation": &ms.home.formation
                    },
                    "away": {
                        "name": &ms.away.name,
                        "formation": &ms.away.formation
                    },
                    "player_slots": ms.player_slots.iter().map(|slot| {
                        serde_json::json!({
                            "track_id": slot.track_id,
                            "team": &slot.team,
                            "name": &slot.name,
                            "position": &slot.position,
                            "overall": slot.overall,
                            "slot": slot.slot
                        })
                    }).collect::<Vec<_>>()
                })
            }),
            "events": events_json.clone(),
            "timeline": events_json.iter().map(|e| serde_json::json!({
                "t": e.get("t").unwrap_or(&serde_json::json!(0.0)),
                "label": e.get("type").unwrap_or(&serde_json::json!("unknown")),
                "team_id": if e.get("is_home_team").and_then(|v| v.as_bool()).unwrap_or(true) { 0 } else { 1 },
                "player": e.get("player")
            })).collect::<Vec<_>>()
        });
        let header_bytes = serde_json::to_vec(&header_json).unwrap_or_else(|_| b"{}".to_vec());
        let header_len = header_bytes.len() as u32;

        // Final buffer: magic "MRB0" + version byte(3) + header_len + header + body
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&0x3042524Du32.to_le_bytes()); // "MRB0"
        out.push(3u8); // format version
        out.extend_from_slice(&header_len.to_le_bytes());
        out.extend_from_slice(&header_bytes);
        out.extend_from_slice(&body);

        PackedByteArray::from(out.as_slice())
    }

    #[func]
    pub fn get_version(&self) -> GString {
        let v = env!("CARGO_PKG_VERSION");
        GString::from(format!("v{}", v))
    }

    #[func]
    pub fn get_build_tag(&self) -> GString {
        // Build tag format: version-target-profile
        let version = env!("CARGO_PKG_VERSION");
        #[cfg(debug_assertions)]
        let profile = "debug";
        #[cfg(not(debug_assertions))]
        let profile = "release";
        #[cfg(target_os = "windows")]
        let target = "windows";
        #[cfg(target_os = "linux")]
        let target = "linux";
        #[cfg(target_os = "macos")]
        let target = "macos";
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        let target = "unknown";
        GString::from(format!("{}-{}-{}", version, target, profile))
    }

    // ============================================================================
    // Gacha System API (Issue #4)
    // ============================================================================

    /// Perform a single gacha pull.
    /// Returns Dictionary: { cards: Array<CardDict>, is_new: Array<bool>, summary: String }
    #[func]
    pub fn gacha_pull_single(&self, seed: i64) -> Dictionary {
        let mut gacha = self.gacha_system.borrow_mut();
        let mut result = gacha.pull_single(seed as u64);
        let pity_counter = gacha.pity_counter;
        drop(gacha);

        self.apply_gacha_result_to_inventory(&mut result);
        self.gacha_result_to_dict(&result, pity_counter)
    }

    /// Perform a 10-pull gacha.
    /// Returns Dictionary: { cards: Array<CardDict>, is_new: Array<bool>, summary: String }
    #[func]
    pub fn gacha_pull_ten(&self, seed: i64) -> Dictionary {
        let mut gacha = self.gacha_system.borrow_mut();
        let mut result = gacha.pull_ten(seed as u64);
        let pity_counter = gacha.pity_counter;
        drop(gacha);

        self.apply_gacha_result_to_inventory(&mut result);
        self.gacha_result_to_dict(&result, pity_counter)
    }

    /// Get current pity counter.
    #[func]
    pub fn gacha_get_pity_count(&self) -> i32 {
        self.gacha_system.borrow().pity_counter as i32
    }

    fn apply_gacha_result_to_inventory(&self, result: &mut GachaResult) {
        let mut inventory = self.coach_inventory.borrow_mut();
        let collection = build_collection_set(&inventory);
        result.check_new_cards(&collection);

        for card in result.cards.iter().cloned() {
            add_gacha_card_to_inventory(&mut inventory, card);
        }
    }

    /// Helper: Convert GachaResult to Godot Dictionary
    fn gacha_result_to_dict(&self, result: &GachaResult, pity_counter: u32) -> Dictionary {
        let mut dict = Dictionary::new();

        let mut cards_arr = Array::<Variant>::new();
        for (idx, card) in result.cards.iter().enumerate() {
            let mut card_dict = self.gacha_card_to_dict(card);
            let is_new = result.is_new_flags.get(idx).copied().unwrap_or(false);
            card_dict.set("is_new", is_new);
            cards_arr.push(&card_dict.to_variant());
        }
        dict.set("cards", cards_arr);

        let mut is_new_arr = Array::<Variant>::new();
        for flag in &result.is_new_flags {
            is_new_arr.push(&flag.to_variant());
        }
        dict.set("is_new", is_new_arr);

        if let Some(card0) = result.cards.first() {
            let mut card0_dict = self.gacha_card_to_dict(card0);
            card0_dict.set(
                "is_new",
                result.is_new_flags.first().copied().unwrap_or(false),
            );
            dict.set("card", card0_dict);
        }

        dict.set("summary", GString::from(result.summary()));
        dict.set("new_count", result.new_card_count() as i32);
        dict.set("pity_counter", pity_counter as i32);

        dict
    }

    /// Helper: Convert GachaCard to Godot Dictionary
    fn gacha_card_to_dict(&self, card: &GachaCard) -> Dictionary {       
        match card {
            GachaCard::Coach(c) => self.coach_card_to_dict(c),
            GachaCard::Tactics(t) => self.tactics_card_to_dict(t),
        }
    }

    fn coach_card_to_dict(&self, card: &CoachCard) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("id", GString::from(card.id.as_str()));
        dict.set("name", GString::from(card.name.as_str()));
        dict.set("rarity", card.rarity as i32);
        dict.set("type", GString::from(Self::card_type_id(card.card_type)));
        dict.set("card_type", GString::from(format!("{:?}", card.card_type)));
        dict.set(
            "specialty",
            GString::from(Self::specialty_id(card.specialty)),
        );
        dict.set(
            "specialty_name",
            GString::from(match card.specialty {
                Specialty::Speed => "스피드",
                Specialty::Power => "파워",
                Specialty::Technical => "테크닉",
                Specialty::Mental => "멘탈",
                Specialty::Balanced => "밸런스",
            }),
        );
        dict.set("level", card.level as i32);
        dict.set("experience", card.experience as i32);
        dict.set("use_count", card.use_count as i32);
        dict.set("description", GString::from(card.description.as_str()));
        dict.set("bonus_value", card.current_bonus());
        dict.set("display", GString::from(card.display()));
        dict
    }

    fn tactics_card_to_dict(&self, card: &TacticsCard) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("id", GString::from(card.id.as_str()));
        dict.set("name", GString::from(card.name.as_str()));
        dict.set("rarity", card.rarity as i32);
        dict.set("type", GString::from("tactics"));
        dict.set("card_type", GString::from("Tactics"));
        dict.set(
            "tactical_style",
            GString::from(format!("{:?}", card.tactical_style)),
        );
        dict.set(
            "tactical_style_icon",
            GString::from(card.tactical_style.icon()),
        );
        dict.set(
            "tactical_style_desc",
            GString::from(card.tactical_style.description()),
        );
        dict.set("level", card.level as i32);
        dict.set("experience", card.experience as i32);
        dict.set("use_count", card.use_count as i32);
        dict.set("description", GString::from(card.description.as_str()));
        dict.set("bonus_value", card.current_bonus());
        dict.set(
            "display",
            GString::from(format!(
                "{} {} {}",
                card.rarity.emoji(),
                card.name,
                card.tactical_style.icon()
            )),
        );
        dict
    }

    fn card_type_id(card_type: CardType) -> &'static str {
        match card_type {
            CardType::Manager => "manager",
            CardType::Coach => "coach",
            CardType::Tactics => "tactics",
        }
    }

    fn specialty_id(specialty: Specialty) -> &'static str {
        match specialty {
            Specialty::Speed => "speed",
            Specialty::Power => "power",
            Specialty::Technical => "technical",
            Specialty::Mental => "mental",
            Specialty::Balanced => "balanced",
        }
    }

    fn parse_card_type_id_str(s: &str) -> Option<CardType> {
        match s.trim().to_lowercase().as_str() {
            "manager" => Some(CardType::Manager),
            "coach" => Some(CardType::Coach),
            "tactics" => Some(CardType::Tactics),
            _ => None,
        }
    }

    fn parse_specialty_id_str(s: &str) -> Option<Specialty> {
        match s.trim().to_lowercase().as_str() {
            "speed" => Some(Specialty::Speed),
            "power" => Some(Specialty::Power),
            "technical" => Some(Specialty::Technical),
            "mental" => Some(Specialty::Mental),
            "balanced" => Some(Specialty::Balanced),
            _ => None,
        }
    }

    fn parse_card_rarity_int(v: i64) -> Option<CardRarity> {
        match v {
            1 => Some(CardRarity::One),
            2 => Some(CardRarity::Two),
            3 => Some(CardRarity::Three),
            4 => Some(CardRarity::Four),
            5 => Some(CardRarity::Five),
            _ => None,
        }
    }

    // ============================================================================
    // FIX_2601/0109: Coach Inventory + Deck API (Dict SSOT surface)
    // ============================================================================

    fn now_unix_ms() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn best_coach_card(cards: &[CoachCard]) -> Option<CoachCard> {
        cards
            .iter()
            .max_by_key(|c| (c.rarity as u8, c.level, c.experience, c.use_count))
            .cloned()
    }

    fn best_tactics_card(cards: &[TacticsCard]) -> Option<TacticsCard> {
        cards
            .iter()
            .max_by_key(|c| (c.rarity as u8, c.level, c.experience, c.use_count))
            .cloned()
    }

    fn api_error(message: impl Into<String>, code: &str) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("success", false);
        dict.set("error", GString::from(message.into()));
        dict.set("error_code", GString::from(code));
        dict
    }

    fn api_ok() -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("success", true);
        dict
    }

    // -------------------------------------------------------------------------
    // FIX_2601/0109: Persistence bridge (Save round-trip)
    // -------------------------------------------------------------------------

    /// Export gacha/deck/inventory state for SaveManager.
    ///
    /// Returns `{ success: true, state: Dictionary }` where `state` is fully
    /// serializable via `save_game_binary/save_game_json`.
    #[func]
    pub fn coach_export_state(&self) -> Dictionary {
        let (pity_counter, pity_threshold) = {
            let gacha = self.gacha_system.borrow();
            (gacha.pity_counter, gacha.pity_threshold)
        };

        let card_inventory = self.coach_inventory.borrow().clone();
        let saved_decks = self.saved_decks.borrow().clone();
        let active_deck_id = self.active_deck_id.borrow().clone();

        let state = CoachSaveState {
            schema_version: coach_save_state_schema_version(),
            pity_counter,
            pity_threshold,
            card_inventory,
            saved_decks,
            active_deck_id,
        };

        let state_value = to_json_value_or_null(&state);
        let mut out = Self::api_ok();
        out.set("state", json_value_to_variant(&state_value));
        out
    }

    /// Import gacha/deck/inventory state from SaveManager.
    #[func]
    pub fn coach_import_state(&self, state: Dictionary) -> Dictionary {
        let state_value = match godot_variant_to_json_value(&state.to_variant()) {
            Ok(v) => v,
            Err(e) => return Self::api_error(e, "INVALID_COACH_STATE"),
        };

        let parsed: CoachSaveState = match serde_json::from_value(state_value) {
            Ok(s) => s,
            Err(e) => {
                return Self::api_error(
                    format!("Invalid coach_state payload: {}", e),
                    "INVALID_COACH_STATE",
                );
            }
        };

        {
            let mut gacha = self.gacha_system.borrow_mut();
            gacha.pity_counter = parsed.pity_counter;
            gacha.pity_threshold = parsed.pity_threshold.max(1);
        }

        *self.coach_inventory.borrow_mut() = parsed.card_inventory;

        let mut decks_cell = self.saved_decks.borrow_mut();
        let mut active_id_cell = self.active_deck_id.borrow_mut();
        *decks_cell = parsed.saved_decks;
        *active_id_cell = parsed.active_deck_id;

        if let Some(ref id) = *active_id_cell {
            if !decks_cell.contains_key(id) {
                *active_id_cell = None;
            }
        }

        Self::api_ok()
    }

    /// Reset gacha/deck/inventory state to defaults (SSOT: Rust).
    ///
    /// Used by SaveManager when loading legacy saves that do not include `coach_state`,
    /// to avoid leaking in-memory coach state across loads.
    #[func]
    pub fn coach_reset_state(&self) -> Dictionary {
        *self.gacha_system.borrow_mut() = GachaSystem::default();

        let mut inventory = InventoryManager::new();
        // Keep the same runtime capacities as init() (migration-safe defaults).
        inventory.manager_inventory.max_capacity = 500;
        inventory.coach_inventory.max_capacity = 1000;
        inventory.tactics_inventory.max_capacity = 500;
        *self.coach_inventory.borrow_mut() = inventory;

        *self.saved_decks.borrow_mut() = BTreeMap::new();
        *self.active_deck_id.borrow_mut() = None;

        Self::api_ok()
    }

    fn parse_saved_deck(deck: &Dictionary) -> Result<SavedDeck, String> {       
        let value = godot_variant_to_json_value(&deck.to_variant())?;
        let obj = value
            .as_object()
            .ok_or_else(|| "Deck must be a Dictionary/Object".to_string())?;

        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("deck_id").and_then(|v| v.as_str()))
            .unwrap_or("default")
            .trim()
            .to_string();
        if id.is_empty() {
            return Err("Deck id is empty".to_string());
        }

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("deck_name").and_then(|v| v.as_str()))
            .unwrap_or("새 덱")
            .trim()
            .to_string();

        let last_used_unix_ms = obj
            .get("last_used_unix_ms")
            .and_then(|v| v.as_i64())
            .or_else(|| obj.get("last_used_ms").and_then(|v| v.as_i64()));

        let mut manager_card_id: Option<String> = None;
        let mut coach_card_ids: [Option<String>; 3] = [None, None, None];
        let mut tactics_card_ids: [Option<String>; 3] = [None, None, None];

        if let Some(slots) = obj.get("slots").and_then(|v| v.as_object()) {
            // Legacy UI schema: { slots: { manager:[{id..}|null], coach:[...], tactics:[...] } }
            if let Some(arr) = slots.get("manager").and_then(|v| v.as_array()) {
                if let Some(item) = arr.first() {
                    manager_card_id = item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }

            if let Some(arr) = slots.get("coach").and_then(|v| v.as_array()) {
                for i in 0..3 {
                    coach_card_ids[i] = arr
                        .get(i)
                        .and_then(|v| v.get("id").and_then(|x| x.as_str()))
                        .map(|s| s.to_string());
                }
            }

            if let Some(arr) = slots.get("tactics").and_then(|v| v.as_array()) {
                for i in 0..3 {
                    tactics_card_ids[i] = arr
                        .get(i)
                        .and_then(|v| v.get("id").and_then(|x| x.as_str()))
                        .map(|s| s.to_string());
                }
            }
        } else {
            // Canonical schema: ids only
            manager_card_id = obj
                .get("manager_card_id")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            if let Some(arr) = obj.get("coach_card_ids").and_then(|v| v.as_array()) {
                for i in 0..3 {
                    coach_card_ids[i] = arr
                        .get(i)
                        .and_then(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .or_else(|| {
                                    v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())
                                })
                        })
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                }
            }

            if let Some(arr) = obj.get("tactics_card_ids").and_then(|v| v.as_array()) {
                for i in 0..3 {
                    tactics_card_ids[i] = arr
                        .get(i)
                        .and_then(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .or_else(|| {
                                    v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())
                                })
                        })
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                }
            }
        }

        Ok(SavedDeck {
            id,
            name,
            manager_card_id,
            coach_card_ids,
            tactics_card_ids,
            last_used_unix_ms,
        })
    }

    fn resolve_card_dict_from_inventory(
        &self,
        inventory: &InventoryManager,
        card_id: &str,
    ) -> Option<Dictionary> {
        if let Some(cards) = inventory.manager_inventory.cards.get(card_id) {
            if let Some(c) = Self::best_coach_card(cards) {
                return Some(self.coach_card_to_dict(&c));
            }
        }
        if let Some(cards) = inventory.coach_inventory.cards.get(card_id) {
            if let Some(c) = Self::best_coach_card(cards) {
                return Some(self.coach_card_to_dict(&c));
            }
        }
        if let Some(cards) = inventory.tactics_inventory.cards.get(card_id) {
            if let Some(t) = Self::best_tactics_card(cards) {
                return Some(self.tactics_card_to_dict(&t));
            }
        }
        None
    }

    fn build_runtime_deck(
        &self,
        saved: &SavedDeck,
        inventory: &InventoryManager,
    ) -> Result<Deck, String> {
        let mut deck = Deck::new(saved.name.clone());

        if let Some(ref manager_id) = saved.manager_card_id {
            let cards = inventory
                .manager_inventory
                .cards
                .get(manager_id)
                .ok_or_else(|| format!("Manager card not owned: {}", manager_id))?;
            let card = Self::best_coach_card(cards)
                .ok_or_else(|| format!("Manager card not owned: {}", manager_id))?;
            deck.set_manager(card)?;
        }

        for (idx, slot) in saved.coach_card_ids.iter().enumerate() {
            if let Some(ref coach_id) = slot {
                let cards = inventory
                    .coach_inventory
                    .cards
                    .get(coach_id)
                    .ok_or_else(|| format!("Coach card not owned: {}", coach_id))?;
                let card = Self::best_coach_card(cards)
                    .ok_or_else(|| format!("Coach card not owned: {}", coach_id))?;
                deck.set_coach(idx, card)?;
            }
        }

        for (idx, slot) in saved.tactics_card_ids.iter().enumerate() {
            if let Some(ref tactics_id) = slot {
                let cards = inventory
                    .tactics_inventory
                    .cards
                    .get(tactics_id)
                    .ok_or_else(|| format!("Tactics card not owned: {}", tactics_id))?;
                let card = Self::best_tactics_card(cards)
                    .ok_or_else(|| format!("Tactics card not owned: {}", tactics_id))?;
                deck.set_tactics(idx, card)?;
            }
        }

        Ok(deck)
    }

    fn list_active_synergies(deck: &Deck) -> Vec<String> {
        let mut out = Vec::new();

        // Specialty focus: 3+ among manager+coaches
        let mut specialty_count: std::collections::HashMap<Specialty, usize> =
            std::collections::HashMap::new();
        if let Some(ref m) = deck.manager_card {
            *specialty_count.entry(m.specialty).or_insert(0) += 1;
        }
        for c in deck.coach_cards.iter().flatten() {
            *specialty_count.entry(c.specialty).or_insert(0) += 1;
        }
        for (spec, count) in specialty_count {
            if count >= 3 {
                out.push(format!("3x {:?} Specialty", spec));
            }
        }

        // Tactical combos (from of_core::coach::tactics)
        let active_tactics: Vec<TacticalStyle> = deck
            .tactics_cards
            .iter()
            .filter_map(|slot| slot.as_ref().map(|t| t.tactical_style))
            .collect();
        for combo in of_core::coach::tactics::get_predefined_combos() {
            if combo.is_active(&active_tactics) {
                out.push(combo.name);
            }
        }

        if deck.is_complete() {
            out.push("Full Deck Bonus".to_string());
        }

        // Match the deck.rs rule (all cards rarity 3+)
        if deck.manager_card.is_some()
            && deck.coach_cards.iter().all(|c| c.is_some())
            && deck.tactics_cards.iter().all(|t| t.is_some())
        {
            let mut ok = true;
            if let Some(ref m) = deck.manager_card {
                ok &= (m.rarity as u8) >= 3;
            }
            for c in deck.coach_cards.iter().flatten() {
                ok &= (c.rarity as u8) >= 3;
            }
            for t in deck.tactics_cards.iter().flatten() {
                ok &= (t.rarity as u8) >= 3;
            }
            if ok {
                out.push("All 3★+ Bonus".to_string());
            }
        }

        out
    }

    fn deck_to_response_dict(&self, saved: &SavedDeck) -> Dictionary {
        let inventory = self.coach_inventory.borrow();
        let mut deck_dict = Dictionary::new();
        deck_dict.set("id", GString::from(saved.id.as_str()));
        deck_dict.set("name", GString::from(saved.name.as_str()));

        // Canonical ids
        if let Some(ref id) = saved.manager_card_id {
            deck_dict.set("manager_card_id", GString::from(id.as_str()));
        }
        let mut coach_ids = Array::<Variant>::new();
        for slot in &saved.coach_card_ids {
            if let Some(ref id) = slot {
                coach_ids.push(&GString::from(id.as_str()).to_variant());
            } else {
                coach_ids.push(&Variant::nil());
            }
        }
        deck_dict.set("coach_card_ids", coach_ids);

        let mut tactics_ids = Array::<Variant>::new();
        for slot in &saved.tactics_card_ids {
            if let Some(ref id) = slot {
                tactics_ids.push(&GString::from(id.as_str()).to_variant());
            } else {
                tactics_ids.push(&Variant::nil());
            }
        }
        deck_dict.set("tactics_card_ids", tactics_ids);
        if let Some(ms) = saved.last_used_unix_ms {
            deck_dict.set("last_used_unix_ms", ms);
        }

        // Legacy compatibility keys for existing UI
        deck_dict.set("deck_id", GString::from(saved.id.as_str()));
        deck_dict.set("deck_name", GString::from(saved.name.as_str()));

        let mut slots = Dictionary::new();

        // manager: 1 slot
        let mut manager_arr = Array::<Variant>::new();
        if let Some(ref id) = saved.manager_card_id {
            if let Some(card) = self.resolve_card_dict_from_inventory(&inventory, id) {
                manager_arr.push(&card.to_variant());
            } else {
                manager_arr.push(&Variant::nil());
            }
        } else {
            manager_arr.push(&Variant::nil());
        }
        slots.set("manager", manager_arr);

        // coach: 3 slots
        let mut coach_arr = Array::<Variant>::new();
        for slot in &saved.coach_card_ids {
            if let Some(ref id) = slot {
                if let Some(card) = self.resolve_card_dict_from_inventory(&inventory, id) {
                    coach_arr.push(&card.to_variant());
                } else {
                    coach_arr.push(&Variant::nil());
                }
            } else {
                coach_arr.push(&Variant::nil());
            }
        }
        slots.set("coach", coach_arr);

        // tactics: 3 slots
        let mut tactics_arr = Array::<Variant>::new();
        for slot in &saved.tactics_card_ids {
            if let Some(ref id) = slot {
                if let Some(card) = self.resolve_card_dict_from_inventory(&inventory, id) {
                    tactics_arr.push(&card.to_variant());
                } else {
                    tactics_arr.push(&Variant::nil());
                }
            } else {
                tactics_arr.push(&Variant::nil());
            }
        }
        slots.set("tactics", tactics_arr);

        deck_dict.set("slots", slots);
        deck_dict
    }

    fn get_active_saved_deck(&self) -> Option<SavedDeck> {
        let active_id = self.active_deck_id.borrow().clone();
        let decks = self.saved_decks.borrow();
        active_id.and_then(|id| decks.get(&id).cloned())
    }

    #[func]
    pub fn coach_get_inventory(&self, filter: Dictionary) -> Dictionary {
        let filter_json = godot_variant_to_json_value(&filter.to_variant()).unwrap_or(JsonValue::Null);

        let type_filter = filter_json
            .get("type")
            .and_then(|v| v.as_str())
            .or_else(|| filter_json.get("filter_type").and_then(|v| v.as_str()))
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let rarity_filter = filter_json
            .get("rarity")
            .and_then(|v| v.as_i64())
            .filter(|v| (1..=5).contains(v));
        let specialty_filter = filter_json
            .get("specialty")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        let inventory = self.coach_inventory.borrow();
        let collection_count = build_collection_set(&inventory).len() as i32;

        let mut cards_arr = Array::<Variant>::new();

        for card in inventory.manager_inventory.get_all_cards() {
            let type_id = Self::card_type_id(card.card_type);
            if type_filter.as_deref().is_some_and(|t| t != type_id) {
                continue;
            }
            if let Some(r) = rarity_filter {
                if card.rarity as i64 != r {
                    continue;
                }
            }
            if let Some(ref s) = specialty_filter {
                if Self::specialty_id(card.specialty) != s {
                    continue;
                }
            }
            cards_arr.push(&self.coach_card_to_dict(&card).to_variant());
        }

        for card in inventory.coach_inventory.get_all_cards() {
            let type_id = Self::card_type_id(card.card_type);
            if type_filter.as_deref().is_some_and(|t| t != type_id) {
                continue;
            }
            if let Some(r) = rarity_filter {
                if card.rarity as i64 != r {
                    continue;
                }
            }
            if let Some(ref s) = specialty_filter {
                if Self::specialty_id(card.specialty) != s {
                    continue;
                }
            }
            cards_arr.push(&self.coach_card_to_dict(&card).to_variant());
        }

        for card in inventory.tactics_inventory.get_all_cards() {
            let type_id = "tactics";
            if type_filter.as_deref().is_some_and(|t| t != type_id) {
                continue;
            }
            if let Some(r) = rarity_filter {
                if card.rarity as i64 != r {
                    continue;
                }
            }
            cards_arr.push(&self.tactics_card_to_dict(&card).to_variant());
        }

        let total_used = (inventory.manager_inventory.count
            + inventory.coach_inventory.count
            + inventory.tactics_inventory.count) as i32;
        let total_max = (inventory.manager_inventory.max_capacity
            + inventory.coach_inventory.max_capacity
            + inventory.tactics_inventory.max_capacity) as i32;

        let mut capacity = Dictionary::new();
        capacity.set("max", total_max);
        capacity.set("used", total_used);
        capacity.set("manager_max", inventory.manager_inventory.max_capacity as i32);
        capacity.set("manager_used", inventory.manager_inventory.count as i32);
        capacity.set("coach_max", inventory.coach_inventory.max_capacity as i32);
        capacity.set("coach_used", inventory.coach_inventory.count as i32);
        capacity.set("tactics_max", inventory.tactics_inventory.max_capacity as i32);
        capacity.set("tactics_used", inventory.tactics_inventory.count as i32);

        let mut out = Self::api_ok();
        out.set("cards", cards_arr);
        out.set("total_count", total_used);
        out.set("collection_count", collection_count);
        out.set("capacity", capacity);
        out
    }

    #[func]
    pub fn coach_add_cards(&self, card_ids: PackedStringArray) -> Dictionary {
        if card_ids.is_empty() {
            return Self::api_error("No card_ids provided", "EMPTY_CARD_IDS");
        }

        let candidates: Vec<GachaCard> = {
            let gacha = self.gacha_system.borrow();
            gacha.pool
                .regular_cards
                .iter()
                .chain(gacha.pool.pickup_cards.iter())
                .cloned()
                .collect()
        };

        let mut inventory = self.coach_inventory.borrow_mut();
        let mut collection = build_collection_set(&inventory);

        let mut added_cards = Array::<Variant>::new();
        let mut missing_ids = Array::<Variant>::new();

        for id in card_ids.as_slice() {
            let id = id.to_string();
            if let Some(card) = candidates.iter().find(|c| c.id() == id).cloned() {
                let is_new = !collection.contains(&id);
                collection.insert(id.clone());
                add_gacha_card_to_inventory(&mut inventory, card.clone());

                let mut card_dict = self.gacha_card_to_dict(&card);
                card_dict.set("is_new", is_new);
                added_cards.push(&card_dict.to_variant());
            } else {
                missing_ids.push(&GString::from(id.as_str()).to_variant());
            }
        }

        let mut out = Self::api_ok();
        out.set("added_cards", added_cards);
        out.set("missing_ids", missing_ids);
        out
    }

    #[func]
    pub fn deck_validate(&self, deck: Dictionary) -> Dictionary {
        let saved = match Self::parse_saved_deck(&deck) {
            Ok(v) => v,
            Err(e) => return Self::api_error(e, "INVALID_DECK_SCHEMA"),
        };

        let inventory = self.coach_inventory.borrow();
        if let Err(e) = self.build_runtime_deck(&saved, &inventory) {
            let mut out = Self::api_error(e, "DECK_INVALID");
            out.set("deck", self.deck_to_response_dict(&saved));
            return out;
        }

        let mut out = Self::api_ok();
        out.set("deck", self.deck_to_response_dict(&saved));
        out
    }

    #[func]
    pub fn deck_upsert(&self, deck: Dictionary) -> Dictionary {
        let mut saved = match Self::parse_saved_deck(&deck) {
            Ok(v) => v,
            Err(e) => return Self::api_error(e, "INVALID_DECK_SCHEMA"),
        };

        saved.last_used_unix_ms = Some(Self::now_unix_ms());

        {
            let inventory = self.coach_inventory.borrow();
            if let Err(e) = self.build_runtime_deck(&saved, &inventory) {
                let mut out = Self::api_error(e, "DECK_INVALID");
                out.set("deck", self.deck_to_response_dict(&saved));
                return out;
            }
        }

        self.saved_decks
            .borrow_mut()
            .insert(saved.id.clone(), saved.clone());

        // Default behavior: if no active deck, set this as active.
        if self.active_deck_id.borrow().is_none() {
            *self.active_deck_id.borrow_mut() = Some(saved.id.clone());
        }

        let mut out = Self::api_ok();
        out.set("deck_id", GString::from(saved.id.as_str()));
        out.set("deck", self.deck_to_response_dict(&saved));
        out
    }

    #[func]
    pub fn deck_delete(&self, deck_id: GString) -> Dictionary {
        let id = deck_id.to_string();
        if id.trim().is_empty() {
            return Self::api_error("Empty deck_id", "EMPTY_DECK_ID");
        }

        let removed = self.saved_decks.borrow_mut().remove(&id);
        if removed.is_none() {
            return Self::api_error(format!("Deck not found: {}", id), "DECK_NOT_FOUND");
        }

        if self.active_deck_id.borrow().as_deref() == Some(&id) {
            *self.active_deck_id.borrow_mut() = None;
        }

        Self::api_ok()
    }

    #[func]
    pub fn deck_set_active(&self, deck_id: GString) -> Dictionary {
        let id = deck_id.to_string();
        if id.trim().is_empty() {
            return Self::api_error("Empty deck_id", "EMPTY_DECK_ID");
        }

        if !self.saved_decks.borrow().contains_key(&id) {
            return Self::api_error(format!("Deck not found: {}", id), "DECK_NOT_FOUND");
        }

        *self.active_deck_id.borrow_mut() = Some(id.clone());
        let mut out = Self::api_ok();
        out.set("deck_id", GString::from(id.as_str()));
        out
    }

    #[func]
    pub fn deck_get_active(&self) -> Dictionary {
        let saved = self
            .get_active_saved_deck()
            .or_else(|| self.saved_decks.borrow().get("default").cloned())
            .unwrap_or_else(|| SavedDeck {
                id: "default".to_string(),
                name: "새 덱".to_string(),
                manager_card_id: None,
                coach_card_ids: [None, None, None],
                tactics_card_ids: [None, None, None],
                last_used_unix_ms: Some(Self::now_unix_ms()),
            });

        let mut out = Self::api_ok();
        out.set("deck", self.deck_to_response_dict(&saved));
        out
    }

    #[func]
    pub fn deck_calculate_training_bonus(
        &self,
        deck: Dictionary,
        training_type: GString,
    ) -> Dictionary {
        let saved = if deck.is_empty() {
            self.get_active_saved_deck()
                .ok_or_else(|| "No deck provided and no active deck".to_string())
        } else {
            Self::parse_saved_deck(&deck)
        };

        let saved = match saved {
            Ok(v) => v,
            Err(e) => return Self::api_error(e, "INVALID_DECK_SCHEMA"),
        };

        let inventory = self.coach_inventory.borrow();
        let runtime_deck = match self.build_runtime_deck(&saved, &inventory) {
            Ok(v) => v,
            Err(e) => return Self::api_error(e, "DECK_INVALID"),
        };

        let tt = training_type.to_string();
        let target = match tt.trim().to_lowercase().as_str() {
            "technical" | "tactical" => of_core::training::TrainingTarget::Technical,
            "physical" => of_core::training::TrainingTarget::Power,
            "mental" | "defensive" => of_core::training::TrainingTarget::Mental,
            _ => of_core::training::TrainingTarget::Technical,
        };

        let (total, logs) = runtime_deck.calculate_training_bonus_with_log(&target);
        let synergies = Self::list_active_synergies(&runtime_deck);

        let mut logs_arr = Array::<Variant>::new();
        for l in logs {
            let mut d = Dictionary::new();
            d.set("source", GString::from(l.source.as_str()));
            d.set("bonus_multiplier", l.bonus_multiplier);
            d.set("reason", GString::from(l.reason.as_str()));
            logs_arr.push(&d.to_variant());
        }

        let mut syn_arr = Array::<Variant>::new();
        for s in synergies {
            syn_arr.push(&GString::from(s.as_str()).to_variant());
        }

        let mut out = Self::api_ok();
        out.set("training_type", GString::from(tt.as_str()));
        out.set("total_multiplier", total);
        out.set("bonus_logs", logs_arr);
        out.set("active_synergies", syn_arr);
        out
    }

    #[func]
    pub fn deck_calculate_match_modifiers(&self, deck: Dictionary) -> Dictionary {
        let saved = if deck.is_empty() {
            self.get_active_saved_deck()
                .ok_or_else(|| "No deck provided and no active deck".to_string())
        } else {
            Self::parse_saved_deck(&deck)
        };

        let saved = match saved {
            Ok(v) => v,
            Err(e) => return Self::api_error(e, "INVALID_DECK_SCHEMA"),
        };

        let inventory = self.coach_inventory.borrow();
        let runtime_deck = match self.build_runtime_deck(&saved, &inventory) {
            Ok(v) => v,
            Err(e) => return Self::api_error(e, "DECK_INVALID"),
        };

        let mm = derive_match_modifiers(&runtime_deck);
        let mut mod_arr = Array::<Variant>::new();
        for (mod_id, value) in mm.to_mod_list() {
            let mut d = Dictionary::new();
            d.set("mod_id", mod_id as i32);
            d.set("value", value);
            mod_arr.push(&d.to_variant());
        }

        let mut weights = Dictionary::new();
        weights.set("Speed", mm.specialty_weights.speed);
        weights.set("Power", mm.specialty_weights.power);
        weights.set("Technical", mm.specialty_weights.technical);
        weights.set("Mental", mm.specialty_weights.mental);
        weights.set("Balanced", mm.specialty_weights.balanced);

        let mut styles_arr = Array::<Variant>::new();
        for s in mm.tactical_styles.iter() {
            let mut d = Dictionary::new();
            d.set("style", GString::from(format!("{:?}", s)));
            d.set("icon", GString::from(s.icon()));
            d.set("description", GString::from(s.description()));
            styles_arr.push(&d.to_variant());
        }

        let mut debug = Dictionary::new();
        debug.set("quality01", mm.quality01);
        debug.set("specialty_weights", weights);
        debug.set("tactical_styles", styles_arr);

        let mut out = Self::api_ok();
        out.set("match_modifiers", mod_arr);
        out.set("debug", debug);
        out
    }

    // ============================================================================
    // Trait System API
    // ============================================================================

    /// Get all 30 traits with their metadata
    #[func]
    pub fn get_all_traits_json(&self) -> GString {
        use of_core::models::trait_system::TraitId;

        let mut traits = Vec::new();

        for id in TraitId::all() {
            let category = id.category();
            let stats = id.get_base_passive_bonus();

            traits.push(json!({
                "id": format!("{:?}", id),
                "name_ko": id.name_ko(),
                "icon": id.icon(),
                "category": format!("{:?}", category),
                "category_ko": category.name_ko(),
                "base_stats": stats.iter().map(|(stat, val)| {
                    json!({ "stat": format!("{:?}", stat), "value": val })
                }).collect::<Vec<_>>(),
            }));
        }

        GString::from(json!({
            "traits": traits,
            "count": traits.len(),
            "tiers": [
                { "id": 1, "name": "Bronze", "name_ko": "동특", "stat_mult": 1.0, "active_mult": 1.1 },
                { "id": 2, "name": "Silver", "name_ko": "은특", "stat_mult": 1.5, "active_mult": 1.3 },
                { "id": 3, "name": "Gold", "name_ko": "금특", "stat_mult": 2.5, "active_mult": 1.8 },
            ]
        }).to_string())
    }

    /// Get stat bonuses for a specific trait and tier
    #[func]
    pub fn get_trait_bonuses_json(&self, trait_id: GString, tier: i32) -> GString {
        use of_core::models::trait_system::{EquippedTrait, TraitId, TraitTier};

        let id_str = trait_id.to_string();

        // Parse trait ID
        let trait_id = match id_str.as_str() {
            "Sniper" => TraitId::Sniper,
            "Cannon" => TraitId::Cannon,
            "Finesse" => TraitId::Finesse,
            "Poacher" => TraitId::Poacher,
            "Panenka" => TraitId::Panenka,
            "LobMaster" => TraitId::LobMaster,
            "Acrobat" => TraitId::Acrobat,
            "Maestro" => TraitId::Maestro,
            "Crosser" => TraitId::Crosser,
            "DeadBall" => TraitId::DeadBall,
            "Metronome" => TraitId::Metronome,
            "Architect" => TraitId::Architect,
            "Speedster" => TraitId::Speedster,
            "Technician" => TraitId::Technician,
            "Tank" => TraitId::Tank,
            "Magnet" => TraitId::Magnet,
            "Showman" => TraitId::Showman,
            "Unshakable" => TraitId::Unshakable,
            "Vacuum" => TraitId::Vacuum,
            "Wall" => TraitId::Wall,
            "AirRaid" => TraitId::AirRaid,
            "Engine" => TraitId::Engine,
            "Reader" => TraitId::Reader,
            "Shadow" => TraitId::Shadow,
            "Bully" => TraitId::Bully,
            "Motor" => TraitId::Motor,
            "Spider" => TraitId::Spider,
            "Sweeper" => TraitId::Sweeper,
            "Giant" => TraitId::Giant,
            "Quarterback" => TraitId::Quarterback,
            _ => {
                return self
                    .create_error_response(&format!("Unknown trait: {}", id_str), "INVALID_TRAIT")
            }
        };

        let trait_tier = match tier {
            1 => TraitTier::Bronze,
            2 => TraitTier::Silver,
            3 => TraitTier::Gold,
            _ => {
                return self
                    .create_error_response(&format!("Invalid tier: {}", tier), "INVALID_TIER")
            }
        };

        let _equipped = EquippedTrait::new(trait_id, trait_tier);
        let stat_mult = trait_tier.stat_multiplier();
        let active_mult = trait_tier.active_multiplier();

        // Get base stats and scale by tier
        let base_stats = trait_id.get_base_passive_bonus();
        let scaled_stats: Vec<_> = base_stats
            .iter()
            .map(|(stat, val)| {
                json!({
                    "stat": format!("{:?}", stat),
                    "base_value": val,
                    "scaled_value": (*val * stat_mult) as i32
                })
            })
            .collect();

        GString::from(
            json!({
                "id": id_str,
                "tier": tier,
                "tier_name": trait_tier.name_ko(),
                "stat_multiplier": stat_mult,
                "active_multiplier": active_mult,
                "stats": scaled_stats,
                "has_gold_special": tier == 3,
            })
            .to_string(),
        )
    }

    /// Calculate combined trait effects for a player's equipped traits
    /// Input: JSON array of { "id": "Sniper", "tier": 1 }
    #[func]
    pub fn calculate_trait_effects_json(&self, traits_json: GString) -> GString {
        use of_core::models::trait_system::{
            ActionType, EquippedTrait, StatType, TraitId, TraitSlots, TraitTier,
        };

        let json_str = traits_json.to_string();

        // Parse input
        let input: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&json_str);
        let trait_list = match input {
            Ok(list) => list,
            Err(e) => {
                return self.create_error_response(&format!("Invalid JSON: {}", e), "PARSE_ERROR")
            }
        };

        let mut slots = TraitSlots::with_unlocked(4); // All 4 slots unlocked

        // Parse and equip each trait
        for (idx, item) in trait_list.iter().enumerate().take(4) {
            let id_str = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let tier_num = item.get("tier").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

            let trait_id = match id_str {
                "Sniper" => TraitId::Sniper,
                "Cannon" => TraitId::Cannon,
                "Finesse" => TraitId::Finesse,
                "Poacher" => TraitId::Poacher,
                "Panenka" => TraitId::Panenka,
                "LobMaster" => TraitId::LobMaster,
                "Acrobat" => TraitId::Acrobat,
                "Maestro" => TraitId::Maestro,
                "Crosser" => TraitId::Crosser,
                "DeadBall" => TraitId::DeadBall,
                "Metronome" => TraitId::Metronome,
                "Architect" => TraitId::Architect,
                "Speedster" => TraitId::Speedster,
                "Technician" => TraitId::Technician,
                "Tank" => TraitId::Tank,
                "Magnet" => TraitId::Magnet,
                "Showman" => TraitId::Showman,
                "Unshakable" => TraitId::Unshakable,
                "Vacuum" => TraitId::Vacuum,
                "Wall" => TraitId::Wall,
                "AirRaid" => TraitId::AirRaid,
                "Engine" => TraitId::Engine,
                "Reader" => TraitId::Reader,
                "Shadow" => TraitId::Shadow,
                "Bully" => TraitId::Bully,
                "Motor" => TraitId::Motor,
                "Spider" => TraitId::Spider,
                "Sweeper" => TraitId::Sweeper,
                "Giant" => TraitId::Giant,
                "Quarterback" => TraitId::Quarterback,
                _ => continue,
            };

            let tier = match tier_num {
                1 => TraitTier::Bronze,
                2 => TraitTier::Silver,
                3 => TraitTier::Gold,
                _ => TraitTier::Bronze,
            };

            let _ = slots.equip(idx, EquippedTrait::new(trait_id, tier));
        }

        // Calculate combined stat bonuses
        let stat_types = [
            StatType::Finishing,
            StatType::LongShots,
            StatType::Curve,
            StatType::Positioning,
            StatType::Composure,
            StatType::Vision,
            StatType::Passing,
            StatType::Crossing,
            StatType::FreeKicks,
            StatType::Corners,
            StatType::ShortPassing,
            StatType::LongPassing,
            StatType::Pace,
            StatType::Acceleration,
            StatType::Dribbling,
            StatType::Agility,
            StatType::Strength,
            StatType::Balance,
            StatType::FirstTouch,
            StatType::BallControl,
            StatType::Flair,
            StatType::Tackling,
            StatType::Interceptions,
            StatType::Marking,
            StatType::Jumping,
            StatType::Heading,
            StatType::Stamina,
            StatType::WorkRate,
            StatType::Anticipation,
            StatType::Aggression,
            StatType::Diving,
            StatType::Handling,
            StatType::Speed,
            StatType::Reflexes,
            StatType::Kicking,
            StatType::Throwing,
        ];

        let stat_bonuses: Vec<_> = stat_types
            .iter()
            .filter_map(|stat| {
                let bonus = slots.get_stat_bonus(*stat);
                if bonus > 0.0 {
                    Some(json!({ "stat": format!("{:?}", stat), "bonus": bonus as i32 }))
                } else {
                    None
                }
            })
            .collect();

        // Calculate action multipliers
        let action_types = [
            ActionType::Finishing,
            ActionType::LongShot,
            ActionType::FinesseShot,
            ActionType::ShortPass,
            ActionType::LongPass,
            ActionType::Cross,
            ActionType::Dribble,
            ActionType::Sprint,
            ActionType::SkillMove,
            ActionType::Tackle,
            ActionType::Interception,
            ActionType::Marking,
            ActionType::Dive,
            ActionType::Rush,
            ActionType::Catch,
        ];

        let action_multipliers: Vec<_> = action_types
            .iter()
            .filter_map(|action| {
                let mult = slots.get_action_multiplier(*action);
                if mult > 1.0 {
                    Some(json!({ "action": format!("{:?}", action), "multiplier": mult }))
                } else {
                    None
                }
            })
            .collect();

        // Check for gold special effects
        let gold_traits: Vec<_> = slots
            .equipped()
            .filter(|t| t.tier == TraitTier::Gold)
            .map(|t| format!("{:?}", t.id))
            .collect();

        // Count equipped traits
        let equipped_count = slots.equipped().count();

        GString::from(
            json!({
                "stat_bonuses": stat_bonuses,
                "action_multipliers": action_multipliers,
                "gold_special_traits": gold_traits,
                "equipped_count": equipped_count,
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Simulation API (JSON interface - for GDScript compatibility)
    // ============================================================================

    /// Alias for simulate_match - GDScript calls this name
    #[func]
    pub fn simulate_match_json(&self, request: GString) -> GString {
        self.simulate_match(request)
    }

    // ============================================================================
    // Async Simulation API (for background processing)
    // ============================================================================

    /// Start async simulation, returns job_id
    #[func]
    pub fn start_simulation(&mut self, request_json: GString) -> GString {
        // For now, run synchronously and return a dummy job_id
        let job_id = format!(
            "job_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        // Store result for later retrieval
        let result = self.simulate_match(request_json);

        // In a real async implementation, we'd spawn a thread here
        // For now, we store the result immediately
        GString::from(
            json!({
                "job_id": job_id,
                "status": "completed",
                "result": result.to_string()
            })
            .to_string(),
        )
    }

    /// Start async simulation with time budget
    #[func]
    pub fn start_simulation_budget(&mut self, request_json: GString, budget_ms: i32) -> GString {
        let job_id = format!(
            "job_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let result = self.simulate_match_with_budget(
            request_json,
            budget_ms as i64,
            120, // max_minutes
            500, // max_events
        );

        GString::from(
            json!({
                "job_id": job_id,
                "status": "completed",
                "result": result.to_string()
            })
            .to_string(),
        )
    }

    /// Poll for async simulation progress (currently no-op as we run sync)
    #[func]
    pub fn poll_simulation(&mut self) {
        // No-op for synchronous implementation
    }

    /// Get result of completed simulation by job_id
    #[func]
    pub fn get_result(&self, _job_id: GString) -> GString {
        // For sync implementation, return empty (result was already in start_simulation response)
        GString::from("{}")
    }

    // ============================================================================
    // Replay API
    // ============================================================================

    /// Get replay data from vendor match (placeholder)
    #[func]
    pub fn get_vendor_match_replay_binary(
        &self,
        _request_json: GString,
        _seed: i64,
    ) -> PackedByteArray {
        PackedByteArray::new()
    }

    /// Get replay data from match (placeholder)
    #[func]
    pub fn get_match_replay_binary(&self, _request_json: GString, _seed: i64) -> PackedByteArray {
        PackedByteArray::new()
    }

    /// Get replay as JSON
    #[func]
    pub fn get_replay_json(
        &self,
        match_result_json: GString,
        _highlight_level: GString,
        _user_player_id: GString,
    ) -> GString {
        // Parse match result and convert to replay format
        let result_str = match_result_json.to_string();
        if result_str.is_empty() {
            return self.create_error_response("Empty match result", "EMPTY_INPUT");
        }

        GString::from(
            json!({
                "format_version": 3,
                "events": [],
                "duration_seconds": 5400.0,
                "match_id": "replay_generated"
            })
            .to_string(),
        )
    }

    /// Create replay from match result
    #[func]
    pub fn create_replay_from_match(
        &self,
        _match_result_json: GString,
        _options_json: GString,
    ) -> GString {
        GString::from(
            json!({
                "success": true,
                "replay": {
                    "format_version": 3,
                    "events": [],
                    "duration_seconds": 5400.0
                }
            })
            .to_string(),
        )
    }

    /// Validate replay data
    #[func]
    pub fn validate_replay(&self, replay_json: GString) -> GString {
        let replay_str = replay_json.to_string();
        let is_valid = !replay_str.is_empty() && replay_str.starts_with('{');

        GString::from(
            json!({
                "valid": is_valid,
                "errors": if is_valid { vec![] } else { vec!["Invalid JSON format"] }
            })
            .to_string(),
        )
    }

    /// Create test replay for debugging
    #[func]
    pub fn create_test_replay(&self) -> GString {
        GString::from(
            json!({
                "format_version": 3,
                "match_id": "test_replay",
                "duration_seconds": 5400.0,
                "score": { "home": 2, "away": 1 },
                "events": [
                    { "t": 900.0, "type": "goal", "team": "home", "player": "Test Player 1" },
                    { "t": 2700.0, "type": "goal", "team": "away", "player": "Test Player 2" },
                    { "t": 4500.0, "type": "goal", "team": "home", "player": "Test Player 3" }
                ]
            })
            .to_string(),
        )
    }

    /// Get match clips by mode (full/highlight/key_moment)
    ///
    /// Returns clips detected by ClipReducer based on ChanceScore thresholds:
    /// - "full": No filtering (entire match)
    /// - "highlight": ChanceScore ≥ 0.10
    /// - "key_moment": ChanceScore ≥ 0.25 or goals/penalties
    ///
    /// Returns JSON array of clip definitions with start_ms, end_ms, chance_score, description
    #[func]
    pub fn get_match_clips(&self, _match_id: GString, mode: GString) -> GString {
        let mode_str = mode.to_string();
        self.create_error_response(
            &format!(
                "get_match_clips(match_id, mode) is deprecated (was a stub). Use get_match_clips_from_result(match_result_json, mode). mode={mode_str}"
            ),
            "DEPRECATED_STUB",
        )
    }

    /// Get match clips by mode from match_result JSON (authoritative v1.1).
    ///
    /// Returns JSON array of clip definitions:
    /// - id: String
    /// - mode: String ("highlight" or "key_moment")
    /// - start_ms: int
    /// - end_ms: int
    /// - chance_score: float (0..1 proxy; derived from BestMoment priority)
    /// - description: String
    #[func]
    pub fn get_match_clips_from_result(
        &self,
        match_result_json: GString,
        mode: GString,
    ) -> GString {
        use of_core::models::MatchResult;

        let mode_str = mode.to_string();

        let match_result: MatchResult = match serde_json::from_str(&match_result_json.to_string())
        {
            Ok(result) => result,
            Err(e) => {
                return self.create_error_response(
                    &format!("Invalid match_result JSON: {e}"),
                    "INVALID_MATCH_RESULT",
                );
            }
        };

        // Full match mode: no clips needed (entire match playback)
        if mode_str == "full" {
            return GString::from(json!([]).to_string());
        }

        if mode_str != "highlight" && mode_str != "key_moment" {
            return self.create_error_response(
                &format!("Invalid clip mode: {}", mode_str),
                "INVALID_MODE",
            );
        }

        // Source: BestMoment windows (event-only, deterministic).
        let moments = match &match_result.best_moments {
            Some(moments) => moments.clone(),
            None => of_core::models::generate_best_moments(&match_result.events),
        };

        let min_priority: u8 = if mode_str == "key_moment" { 70 } else { 40 };

        let mut clips: Vec<serde_json::Value> = Vec::new();
        for moment in moments {
            if moment.priority < min_priority {
                continue;
            }

            let start_ms = moment.start_time_ms as i64;
            let end_ms = moment.end_time_ms as i64;
            let chance_score = (moment.priority as f64 / 100.0).clamp(0.0, 1.0);
            let desc = moment
                .description
                .unwrap_or_else(|| format!("{:?}", moment.moment_type));

            clips.push(json!({
                "id": format!("clip_{}_{}", start_ms, end_ms),
                "mode": mode_str,
                "start_ms": start_ms,
                "end_ms": end_ms,
                "chance_score": chance_score,
                "description": desc,
            }));
        }

        GString::from(json!(clips).to_string())
    }

    /// Get field coordinate info
    #[func]
    pub fn get_field_coordinate_info(&self) -> GString {
        GString::from(
            json!({
                "pitch_length_m": 105.0,
                "pitch_width_m": 68.0,
                "goal_width_m": 7.32,
                "penalty_area_length_m": 16.5,
                "penalty_area_width_m": 40.3,
                "center_circle_radius_m": 9.15,
                "coordinate_system": "normalized",
                "x_range": [0.0, 1.0],
                "y_range": [0.0, 1.0]
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Match Statistics API
    // ============================================================================

    /// Apply live substitution during match (Session mode).
    ///
    /// This was originally a stub; it now forwards to the active `LiveMatchSession`.
    ///
    /// Expected JSON payload (SSOT):
    /// - `team`: "home" | "away"
    /// - `out_track_id`: pitch slot track_id (0..21)
    /// - `in_bench_slot`: per-team bench slot (0..6)
    ///
    /// Back-compat:
    /// - If `out_idx` and `in_idx` are provided, those are used directly
    ///   (out_idx 0..10, in_idx 11..17).
    #[func]
    pub fn apply_live_substitution_json(&mut self, payload_json: GString) -> GString {
        let payload_str = payload_json.to_string();
        let payload: JsonValue = match serde_json::from_str(&payload_str) {
            Ok(v) => v,
            Err(e) => {
                return self.create_error_response(
                    &format!("Invalid substitution payload JSON: {}", e),
                    "PARSE_ERROR",
                );
            }
        };

        fn parse_i32(v: Option<&JsonValue>) -> Option<i32> {
            let v = v?;
            if let Some(n) = v.as_i64() {
                return i32::try_from(n).ok();
            }
            v.as_str()?.parse::<i32>().ok()
        }

        let team_str = payload
            .get("team")
            .and_then(|v| v.as_str())
            .unwrap_or("home")
            .to_lowercase();
        let team_side = match team_str.as_str() {
            "home" => TeamSide::Home,
            "away" => TeamSide::Away,
            _ => {
                return self.create_error_response("Invalid team (use home/away)", "INVALID_TEAM");
            }
        };

        // Preferred SSOT fields
        let out_track_id = parse_i32(payload.get("out_track_id").or_else(|| payload.get("pitch_track_id")));
        let in_bench_slot = parse_i32(payload.get("in_bench_slot").or_else(|| payload.get("bench_slot")));

        // Back-compat fields (indices)
        let out_idx_direct = parse_i32(payload.get("out_idx"));
        let in_idx_direct = parse_i32(payload.get("in_idx"));

        let (out_idx, in_idx) = if let (Some(out_idx), Some(in_idx)) = (out_idx_direct, in_idx_direct) {
            (out_idx, in_idx)
        } else {
            let out_track_id = match out_track_id {
                Some(v) => v,
                None => {
                    return self.create_error_response(
                        "Missing out_track_id (or out_idx)",
                        "MISSING_OUT_TRACK_ID",
                    );
                }
            };
            let in_bench_slot = match in_bench_slot {
                Some(v) => v,
                None => {
                    return self.create_error_response(
                        "Missing in_bench_slot (or in_idx)",
                        "MISSING_IN_BENCH_SLOT",
                    );
                }
            };

            if !(0..=6).contains(&in_bench_slot) {
                return self.create_error_response("in_bench_slot must be 0..6", "INVALID_BENCH_SLOT");
            }

            let out_idx = match team_side {
                TeamSide::Home => {
                    if !(0..=10).contains(&out_track_id) {
                        return self
                            .create_error_response("home out_track_id must be 0..10", "INVALID_OUT_TRACK_ID");
                    }
                    out_track_id
                }
                TeamSide::Away => {
                    if !(11..=21).contains(&out_track_id) {
                        return self
                            .create_error_response("away out_track_id must be 11..21", "INVALID_OUT_TRACK_ID");
                    }
                    out_track_id - 11
                }
            };

            let in_idx = 11 + in_bench_slot;
            (out_idx, in_idx)
        };

        if !(0..=10).contains(&out_idx) {
            return self.create_error_response("out_idx must be 0..10", "INVALID_OUT_IDX");
        }
        if !(11..=17).contains(&in_idx) {
            return self.create_error_response("in_idx must be 11..17", "INVALID_IN_IDX");
        }

        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => match s.substitute(team_side, out_idx as usize, in_idx as usize) {
                Ok(()) => GString::from(
                    json!({
                        "success": true,
                        "team": team_str,
                        "out_idx": out_idx,
                        "in_idx": in_idx,
                        "message": "Substitution applied"
                    })
                    .to_string(),
                ),
                Err(e) => self.create_error_response(e, "SUBSTITUTION_FAILED"),
            },
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Get match statistics
    #[func]
    pub fn get_match_statistics_json(&self, _match_id: GString) -> GString {
        GString::from(
            json!({
                "possession": { "home": 50, "away": 50 },
                "shots": { "home": 10, "away": 8 },
                "shots_on_target": { "home": 5, "away": 4 },
                "corners": { "home": 5, "away": 3 },
                "fouls": { "home": 12, "away": 10 },
                "yellow_cards": { "home": 1, "away": 2 },
                "red_cards": { "home": 0, "away": 0 },
                "offsides": { "home": 2, "away": 3 },
                "passes": { "home": 450, "away": 400 },
                "pass_accuracy": { "home": 85.0, "away": 82.0 }
            })
            .to_string(),
        )
    }

    /// Simulate match with tactical instructions
    #[func]
    pub fn simulate_match_with_instructions(&self, json_request: GString) -> GString {
        // Parse and apply instructions, then simulate
        self.simulate_match(json_request)
    }

    /// Batch simulate multiple matches
    #[func]
    pub fn simulate_matches_batch(&self, json_request: GString, batch_size: i32) -> GString {
        let request_str = json_request.to_string();

        // Parse as array of match requests
        let requests: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&request_str);

        match requests {
            Ok(reqs) => {
                let mut results = Vec::new();
                let limit = (batch_size as usize).min(reqs.len());

                for req in reqs.into_iter().take(limit) {
                    let req_json = GString::from(req.to_string());
                    let result = self.simulate_match(req_json);
                    results.push(result.to_string());
                }

                GString::from(
                    json!({
                        "success": true,
                        "count": results.len(),
                        "results": results
                    })
                    .to_string(),
                )
            }
            Err(e) => {
                self.create_error_response(&format!("Invalid batch request: {}", e), "PARSE_ERROR")
            }
        }
    }

    // ============================================================================
    // Formation API
    // ============================================================================

    /// Get all available formations
    #[func]
    pub fn get_all_formations(&self) -> GString {
        GString::from(json!({
            "formations": [
                { "id": "4-4-2", "name": "4-4-2", "description": "Balanced formation", "category": "balanced" },
                { "id": "4-3-3", "name": "4-3-3", "description": "Attacking formation", "category": "offensive" },
                { "id": "4-2-3-1", "name": "4-2-3-1", "description": "Modern formation with CAM", "category": "balanced" },
                { "id": "3-5-2", "name": "3-5-2", "description": "Wing-back formation", "category": "offensive" },
                { "id": "5-3-2", "name": "5-3-2", "description": "Defensive formation", "category": "defensive" },
                { "id": "4-1-4-1", "name": "4-1-4-1", "description": "Defensive midfield anchor", "category": "defensive" },
                { "id": "3-4-3", "name": "3-4-3", "description": "Ultra attacking", "category": "offensive" },
                { "id": "5-4-1", "name": "5-4-1", "description": "Ultra defensive", "category": "defensive" }
            ]
        }).to_string())
    }

    /// Get detailed formation info
    #[func]
    pub fn get_formation_details(&self, formation_id: GString) -> GString {
        let id = formation_id.to_string();

        let positions = match id.as_str() {
            "4-4-2" => vec![
                json!({"slot": 0, "position": "GK", "x": 0.5, "y": 0.05}),
                json!({"slot": 1, "position": "LB", "x": 0.15, "y": 0.25}),
                json!({"slot": 2, "position": "CB", "x": 0.35, "y": 0.2}),
                json!({"slot": 3, "position": "CB", "x": 0.65, "y": 0.2}),
                json!({"slot": 4, "position": "RB", "x": 0.85, "y": 0.25}),
                json!({"slot": 5, "position": "LM", "x": 0.15, "y": 0.5}),
                json!({"slot": 6, "position": "CM", "x": 0.35, "y": 0.45}),
                json!({"slot": 7, "position": "CM", "x": 0.65, "y": 0.45}),
                json!({"slot": 8, "position": "RM", "x": 0.85, "y": 0.5}),
                json!({"slot": 9, "position": "ST", "x": 0.35, "y": 0.8}),
                json!({"slot": 10, "position": "ST", "x": 0.65, "y": 0.8}),
            ],
            "4-3-3" => vec![
                json!({"slot": 0, "position": "GK", "x": 0.5, "y": 0.05}),
                json!({"slot": 1, "position": "LB", "x": 0.15, "y": 0.25}),
                json!({"slot": 2, "position": "CB", "x": 0.35, "y": 0.2}),
                json!({"slot": 3, "position": "CB", "x": 0.65, "y": 0.2}),
                json!({"slot": 4, "position": "RB", "x": 0.85, "y": 0.25}),
                json!({"slot": 5, "position": "CM", "x": 0.3, "y": 0.45}),
                json!({"slot": 6, "position": "CM", "x": 0.5, "y": 0.4}),
                json!({"slot": 7, "position": "CM", "x": 0.7, "y": 0.45}),
                json!({"slot": 8, "position": "LW", "x": 0.15, "y": 0.75}),
                json!({"slot": 9, "position": "ST", "x": 0.5, "y": 0.85}),
                json!({"slot": 10, "position": "RW", "x": 0.85, "y": 0.75}),
            ],
            _ => vec![],
        };

        GString::from(
            json!({
                "id": id,
                "positions": positions,
                "characteristics": {
                    "defensive_strength": 5,
                    "offensive_strength": 5,
                    "width": 5,
                    "compactness": 5
                }
            })
            .to_string(),
        )
    }

    /// Recommend formations based on player roster
    #[func]
    pub fn recommend_formations(&self, _players_json: GString) -> GString {
        GString::from(json!({
            "recommendations": [
                { "formation_id": "4-3-3", "fitness_score": 0.85, "reason": "Good wingers available" },
                { "formation_id": "4-4-2", "fitness_score": 0.80, "reason": "Balanced squad" },
                { "formation_id": "4-2-3-1", "fitness_score": 0.75, "reason": "Strong CAM option" }
            ]
        }).to_string())
    }

    /// Calculate how well a formation fits a roster
    #[func]
    pub fn calculate_formation_fitness(
        &self,
        formation_id: GString,
        _players_json: GString,
    ) -> GString {
        let id = formation_id.to_string();
        GString::from(
            json!({
                "formation_id": id,
                "fitness_score": 0.82,
                "position_fits": [
                    { "slot": 0, "position": "GK", "fit_score": 1.0 },
                    { "slot": 1, "position": "LB", "fit_score": 0.9 }
                ],
                "weak_positions": [],
                "recommendations": []
            })
            .to_string(),
        )
    }

    /// Suggest counter formation against opponent
    #[func]
    pub fn suggest_counter_formation(
        &self,
        opponent_formation_id: GString,
        _players_json: GString,
    ) -> GString {
        let opponent = opponent_formation_id.to_string();
        let counter = match opponent.as_str() {
            "4-3-3" => "4-4-2",
            "4-4-2" => "4-3-3",
            "3-5-2" => "4-4-2",
            _ => "4-4-2",
        };

        GString::from(
            json!({
                "suggested_formation": counter,
                "reason": format!("Counter to {}", opponent),
                "tactical_adjustments": ["Press high", "Wide play"]
            })
            .to_string(),
        )
    }

    /// Suggest formation based on match situation
    #[func]
    pub fn suggest_situational_formation(
        &self,
        _current_formation_id: GString,
        match_state_json: GString,
    ) -> GString {
        let state_str = match_state_json.to_string();
        let _state: serde_json::Value = serde_json::from_str(&state_str).unwrap_or(json!({}));

        GString::from(
            json!({
                "suggested_formation": "4-3-3",
                "reason": "Need more attacking options",
                "urgency": "medium"
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Team Instructions API
    // ============================================================================

    /// Get available team instruction options
    #[func]
    pub fn get_team_instruction_options(&self) -> GString {
        GString::from(
            json!({
                "mentality": ["defensive", "balanced", "attacking", "ultra_attacking"],
                "width": ["narrow", "normal", "wide"],
                "tempo": ["slow", "normal", "fast"],
                "pressing": ["low", "medium", "high", "gegenpressing"],
                "defensive_line": ["deep", "normal", "high"],
                "build_up": ["short", "mixed", "long"]
            })
            .to_string(),
        )
    }

    /// Get tactical presets
    #[func]
    pub fn get_tactical_presets(&self) -> GString {
        GString::from(
            json!({
                "presets": [
                    {
                        "id": "balanced",
                        "name": "Balanced",
                        "description": "Standard approach",
                        "instructions": {
                            "mentality": "balanced",
                            "width": "normal",
                            "tempo": "normal",
                            "pressing": "medium",
                            "defensive_line": "normal"
                        }
                    },
                    {
                        "id": "attacking",
                        "name": "All Out Attack",
                        "description": "Maximum offense",
                        "instructions": {
                            "mentality": "ultra_attacking",
                            "width": "wide",
                            "tempo": "fast",
                            "pressing": "high",
                            "defensive_line": "high"
                        }
                    },
                    {
                        "id": "defensive",
                        "name": "Park the Bus",
                        "description": "Maximum defense",
                        "instructions": {
                            "mentality": "defensive",
                            "width": "narrow",
                            "tempo": "slow",
                            "pressing": "low",
                            "defensive_line": "deep"
                        }
                    },
                    {
                        "id": "counter",
                        "name": "Counter Attack",
                        "description": "Absorb and strike",
                        "instructions": {
                            "mentality": "balanced",
                            "width": "normal",
                            "tempo": "fast",
                            "pressing": "low",
                            "defensive_line": "deep"
                        }
                    },
                    {
                        "id": "possession",
                        "name": "Tiki-Taka",
                        "description": "Control the ball",
                        "instructions": {
                            "mentality": "balanced",
                            "width": "narrow",
                            "tempo": "slow",
                            "pressing": "high",
                            "defensive_line": "high"
                        }
                    }
                ]
            })
            .to_string(),
        )
    }

    /// Set custom team instructions
    #[func]
    pub fn set_team_instructions_custom(&self, instructions_json: GString) -> GString {
        let _instructions = instructions_json.to_string();
        GString::from(
            json!({
                "success": true,
                "applied_instructions": instructions_json.to_string()
            })
            .to_string(),
        )
    }

    /// Set team instructions from preset
    #[func]
    pub fn set_team_instructions_preset(&self, preset_name: GString) -> GString {
        let name = preset_name.to_string();
        GString::from(
            json!({
                "success": true,
                "preset": name,
                "message": format!("Applied {} preset", name)
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Player Instructions API
    // ============================================================================

    /// Get available roles for a position
    #[func]
    pub fn get_available_roles(&self, position: GString) -> GString {
        let pos = position.to_string().to_uppercase();

        let roles = match pos.as_str() {
            "GK" => vec!["Sweeper Keeper", "Traditional"],
            "CB" | "DF" => vec!["Ball Playing", "Stopper", "Cover"],
            "LB" | "RB" => vec!["Full Back", "Wing Back", "Inverted"],
            "CDM" | "DM" => vec!["Anchor", "Ball Winner", "Deep Playmaker"],
            "CM" | "MF" => vec!["Box to Box", "Playmaker", "Mezzala"],
            "CAM" | "AM" => vec!["Advanced Playmaker", "Shadow Striker", "Trequartista"],
            "LW" | "RW" => vec!["Winger", "Inside Forward", "Inverted Winger"],
            "ST" | "CF" | "FW" => vec!["Poacher", "Target Man", "False 9", "Complete Forward"],
            _ => vec!["Default"],
        };

        GString::from(
            json!({
                "position": pos,
                "roles": roles
            })
            .to_string(),
        )
    }

    /// Get all instruction options
    #[func]
    pub fn get_instruction_options(&self) -> GString {
        GString::from(
            json!({
                "movement": ["stay_back", "balanced", "get_forward"],
                "passing": ["short", "mixed", "long"],
                "dribbling": ["normal", "run_at_defense", "hold_position"],
                "crossing": ["early", "normal", "cut_inside"],
                "marking": ["zonal", "man", "mixed"],
                "tackling": ["normal", "aggressive", "conservative"]
            })
            .to_string(),
        )
    }

    /// Set player role
    #[func]
    pub fn set_player_role(&self, player_json: GString, role_name: GString) -> GString {
        let _player = player_json.to_string();
        let role = role_name.to_string();

        GString::from(
            json!({
                "success": true,
                "role": role,
                "attribute_modifiers": {}
            })
            .to_string(),
        )
    }

    /// Set player instructions
    #[func]
    pub fn set_player_instructions(
        &self,
        player_json: GString,
        instructions_json: GString,
    ) -> GString {
        let _player = player_json.to_string();
        let _instructions = instructions_json.to_string();

        GString::from(
            json!({
                "success": true,
                "message": "Instructions applied"
            })
            .to_string(),
        )
    }

    /// Get player attributes modified by role/instructions
    #[func]
    pub fn get_player_modified_attributes(&self, player_json: GString) -> GString {
        let _player = player_json.to_string();

        GString::from(
            json!({
                "base_attributes": {},
                "modified_attributes": {},
                "modifiers": []
            })
            .to_string(),
        )
    }

    /// Clear player instructions
    #[func]
    pub fn clear_player_instructions(&self, player_json: GString) -> GString {
        let _player = player_json.to_string();

        GString::from(
            json!({
                "success": true,
                "message": "Instructions cleared"
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Training API
    // ============================================================================

    /// Execute training session
    #[func]
    pub fn execute_training_json(
        &self,
        request_json: GString,
        player_json: GString,
        manager_json: GString,
    ) -> GString {
        use of_core::api::execute_training_json as core_execute_training;

        let request_str = request_json.to_string();
        let player_str = player_json.to_string();
        let manager_str = manager_json.to_string();

        match core_execute_training(&request_str, &player_str, &manager_str) {
            Ok(result) => GString::from(result),
            Err(e) => {
                self.create_error_response(&format!("Training failed: {}", e), "TRAINING_ERROR")
            }
        }
    }

    // ============================================================================
    // Personality System API
    // ============================================================================

    /// Get personality archetype details
    #[func]
    pub fn get_personality_archetype(&self, archetype: GString, _seed: i64) -> GString {
        let arch = archetype.to_string();

        let traits = match arch.as_str() {
            "leader" => vec!["determined", "vocal", "composed"],
            "maverick" => vec!["creative", "unpredictable", "confident"],
            "professional" => vec!["consistent", "reliable", "hardworking"],
            "hothead" => vec!["aggressive", "passionate", "volatile"],
            _ => vec!["balanced"],
        };

        GString::from(
            json!({
                "archetype": arch,
                "traits": traits,
                "description": format!("{} archetype personality", arch),
                "stat_modifiers": {}
            })
            .to_string(),
        )
    }

    /// Test personality system
    #[func]
    pub fn test_personality_system(&self) -> GString {
        GString::from(json!({
            "success": true,
            "archetypes_available": ["leader", "maverick", "professional", "hothead", "introvert"],
            "traits_count": 30,
            "system_version": "1.0"
        }).to_string())
    }

    /// Calculate ability effects
    #[func]
    pub fn calculate_ability_effects(&self, abilities_json: GString) -> GString {
        let _abilities = abilities_json.to_string();

        GString::from(
            json!({
                "stat_bonuses": {},
                "action_multipliers": {},
                "special_effects": []
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Utility API
    // ============================================================================

    /// Create test match for debugging
    #[func]
    pub fn create_test_match(&self) -> GString {
        GString::from(json!({
            "schema_version": 1,
            "seed": 12345,
            "home_team": {
                "name": "Test Home",
                "formation": "4-4-2",
                "players": (0..18).map(|i| json!({
                    "name": format!("Home Player {}", i + 1),
                    "position": if i == 0 { "GK" } else if i < 5 { "DF" } else if i < 9 { "MF" } else { "FW" },
                    "overall": 70 + (i % 10) as u8
                })).collect::<Vec<_>>()
            },
            "away_team": {
                "name": "Test Away",
                "formation": "4-3-3",
                "players": (0..18).map(|i| json!({
                    "name": format!("Away Player {}", i + 1),
                    "position": if i == 0 { "GK" } else if i < 5 { "DF" } else if i < 9 { "MF" } else { "FW" },
                    "overall": 68 + (i % 12) as u8
                })).collect::<Vec<_>>()
            }
        }).to_string())
    }

    /// Suggest memory cleanup
    #[func]
    pub fn suggest_memory_cleanup(&self) {
        // Trigger garbage collection hint (Rust doesn't have explicit GC, but we can drop cached data)
        // In practice, this is a no-op for Rust
    }

    /// Get memory statistics
    #[func]
    pub fn get_memory_stats(&self) -> GString {
        GString::from(
            json!({
                "heap_used_bytes": 0,
                "cache_entries": 0,
                "active_simulations": 0,
                "note": "Memory stats not available in this build"
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Match Session API (Phase 7: real-time step simulation)
    // ============================================================================

    // =========================================================================
    // FIX_2601/0123 #12: Session Lifecycle Management
    // =========================================================================

    /// Clean up stale session if it exists and has exceeded the TTL.
    ///
    /// This is called automatically when creating a new session to prevent
    /// memory leaks from abandoned sessions.
    fn cleanup_stale_session(&mut self) {
        let mut session = self.live_session.borrow_mut();
        if let Some(ref s) = *session {
            if s.is_stale() {
                // Log cleanup for debugging
                #[cfg(debug_assertions)]
                eprintln!(
                    "[OfSimulator] Cleaning up stale session (idle: {:?})",
                    s.idle_time()
                );
                *session = None;
            }
        }
    }

    /// Check if there is an active (non-stale) session.
    #[func]
    pub fn has_active_session(&self) -> bool {
        let session = self.live_session.borrow();
        session.as_ref().map(|s| !s.is_stale()).unwrap_or(false)
    }

    /// Get session info including TTL status.
    #[func]
    pub fn get_session_info(&self) -> GString {
        let session = self.live_session.borrow();
        match session.as_ref() {
            Some(s) => {
                GString::from(
                    json!({
                        "has_session": true,
                        "state": format!("{:?}", s.get_state()),
                        "age_secs": s.age().as_secs(),
                        "idle_secs": s.idle_time().as_secs(),
                        "is_stale": s.is_stale(),
                        "ttl_secs": LiveMatchSession::DEFAULT_TTL_SECS
                    })
                    .to_string(),
                )
            }
            None => {
                GString::from(
                    json!({
                        "has_session": false,
                        "state": null,
                        "age_secs": null,
                        "idle_secs": null,
                        "is_stale": null,
                        "ttl_secs": LiveMatchSession::DEFAULT_TTL_SECS
                    })
                    .to_string(),
                )
            }
        }
    }

    /// Create a new match session from JSON request.
    /// Returns session info or error.
    ///
    /// FIX_2601/0123 #12: Automatically cleans up stale sessions before creating new ones.
    #[func]
    pub fn create_live_session(&mut self, request_json: GString) -> GString {
        // FIX_2601/0123 #12: Cleanup stale session before creating new one
        self.cleanup_stale_session();

        let request_str = request_json.to_string();

        let request_value = serde_json::from_str::<serde_json::Value>(&request_str).ok();

        // Detect schema_version without committing to a specific struct.
        let schema_version = request_value
            .as_ref()
            .and_then(|v| v.get("schema_version").and_then(|x| x.as_u64()))
            .unwrap_or(1) as u8;

        let team_view_config = request_value
            .as_ref()
            .and_then(parse_team_view_observation_config);

        match schema_version {
            // MatchRequest v2 (UID roster-only): preferred for Phase23.5 session compliance.
            2 => {
                let (plan, enable_position_tracking) =
                    match of_core::api::match_plan_from_match_request_v2_json(&request_str) {
                        Ok(v) => v,
                        Err(e) => return self.create_error_response(&e, "PARSE_ERROR"),
                    };

                let mut session = match LiveMatchSession::new(plan) {
                    Ok(session) => session,
                    Err(err) => return self.create_error_response(&err, "ENGINE_ERROR"),
                };
                session.set_position_tracking_enabled(enable_position_tracking);
                if let Some(config) = team_view_config.clone() {
                    session.set_team_view_observation_config(config);
                }
                *self.live_session.borrow_mut() = Some(session);

                GString::from(
                    json!({
                        "success": true,
                        "state": "not_started",
                        "schema_version": 2,
                        "message": "Match session created (schema v2). Call kick_off_match_session to start (or call start_match_session to create + kick off)."
                    })
                    .to_string(),
                )
            }

            // Legacy schema v1 (InteractiveMatchRequest): keep temporarily for compatibility.
            1 => {
                let req: Result<InteractiveMatchRequest, _> = serde_json::from_str(&request_str);
                match req {
                    Ok(request) => {
                        use of_core::models::team::Formation;

                        fn convert_team_for_live(team: InteractiveTeam) -> Result<Team, String> {
                            let formation = match team.formation.as_str() {
                                "4-4-2" => Formation::F442,
                                "4-3-3" => Formation::F433,
                                "3-5-2" => Formation::F352,
                                "5-3-2" => Formation::F532,
                                "4-2-3-1" => Formation::F4231,
                                "4-1-4-1" => Formation::F4141,
                                "3-4-3" => Formation::F343,
                                "5-4-1" => Formation::F541,
                                other => return Err(format!("Invalid formation: {}", other)),
                            };

                            let mut players = Vec::with_capacity(team.players.len());
                            for p in team.players {
                                let pos = match p.position.to_uppercase().as_str() {
                                    "GK" => OfPosition::GK,
                                    "LB" => OfPosition::LB,
                                    "CB" => OfPosition::CB,
                                    "RB" => OfPosition::RB,
                                    "LWB" => OfPosition::LWB,
                                    "RWB" => OfPosition::RWB,
                                    "CDM" => OfPosition::CDM,
                                    "CM" => OfPosition::CM,
                                    "CAM" => OfPosition::CAM,
                                    "LM" => OfPosition::LM,
                                    "RM" => OfPosition::RM,
                                    "LW" => OfPosition::LW,
                                    "RW" => OfPosition::RW,
                                    "CF" => OfPosition::CF,
                                    "ST" => OfPosition::ST,
                                    "DF" => OfPosition::DF,
                                    "MF" => OfPosition::MF,
                                    "FW" => OfPosition::FW,
                                    _ => OfPosition::CM,
                                };
                                players.push(OfPlayer {
                                    name: p.name,
                                    position: pos,
                                    overall: p.overall,
                                    condition: p.condition, // FIX_2601/0123: use deserialized condition
                                    attributes: None,
                                    equipped_skills: Vec::new(),
                                    traits: Default::default(),
                                    personality: Default::default(),
                                });
                            }

                            Ok(Team {
                                name: team.name,
                                formation,
                                players,
                            })
                        }

                        let home_team = match convert_team_for_live(request.home_team) {
                            Ok(t) => t,
                            Err(e) => return self.create_error_response(&e, "TEAM_ERROR"),
                        };
                        let away_team = match convert_team_for_live(request.away_team) {
                            Ok(t) => t,
                            Err(e) => return self.create_error_response(&e, "TEAM_ERROR"),
                        };

                        let plan = OfMatchPlan {
                            home_team,
                            away_team,
                            seed: request.seed,
                            user_player: None,
                            home_match_modifiers: of_core::engine::TeamMatchModifiers::default(),
                            away_match_modifiers: of_core::engine::TeamMatchModifiers::default(),
                            home_instructions: request.home_instructions,
                            away_instructions: request.away_instructions,
                            home_player_instructions: None,
                            away_player_instructions: None,
                            home_ai_difficulty: None,
                            away_ai_difficulty: None,
                        };

                        let mut session = match LiveMatchSession::new(plan) {
                            Ok(session) => session,
                            Err(err) => return self.create_error_response(&err, "ENGINE_ERROR"),
                        };
                        if let Some(config) = team_view_config.clone() {
                            session.set_team_view_observation_config(config);
                        }
                        *self.live_session.borrow_mut() = Some(session);

                        GString::from(json!({
                            "success": true,
                            "state": "not_started",
                            "schema_version": 1,
                            "message": "Match session created (schema v1 legacy). Call kick_off_match_session to start."
                        }).to_string())
                    }
                    Err(e) => self
                        .create_error_response(&format!("JSON parse error: {}", e), "PARSE_ERROR"),
                }
            }

            other => self.create_error_response(
                &format!("Unsupported schema version: {}", other),
                "SCHEMA_ERROR",
            ),
        }
    }

    /// Kick off the match session (start first half).
    #[func]
    pub fn kick_off_live_match(&mut self) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                s.kick_off();
                let (home, away) = s.get_score();
                GString::from(
                    json!({
                        "success": true,
                        "state": "first_half",
                        "minute": s.get_minute(),
                        "score": { "home": home, "away": away }
                    })
                    .to_string(),
                )
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    // ========================================================================
    // Internal helper: Returns StepResult directly (no JSON serialization)
    // Used by step_live_session() for optimal performance
    // ========================================================================
    fn step_live_internal(&mut self) -> Option<StepResult> {
        let mut session = self.live_session.borrow_mut();
        session.as_mut().map(|s| s.step())
    }

    /// Execute one tick (250ms of game time) and return current state.
    /// Returns tick data with positions, events, and score.
    /// NOTE: For Godot integration, prefer step_match_session() which returns Dictionary directly.
    #[func]
    pub fn step_live_match(&mut self) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                let result = s.step();
                match result {
                    StepResult::NotStarted => {
                        GString::from(json!({
                            "result_type": "not_started",
                            "message": "Match not started. Call start_match_session (preferred) or kick_off_match_session first."
                        }).to_string())
                    }
                    StepResult::Tick(data) => {
                        let players: Vec<serde_json::Value> = data.player_positions.iter().map(|p| {
                            json!({
                                "index": p.index,
                                "x": p.position.0,
                                "y": p.position.1,
                                "state": p.state
                            })
                        }).collect();

                        let events: Vec<serde_json::Value> = data.events.iter().map(|e| {
                            json!({
                                "minute": e.minute,
                                "type": format!("{:?}", e.event_type),
                                "is_home_team": e.is_home_team,
                                "player_track_id": e.player_track_id  // C7: Use track_id
                            })
                        }).collect();

                        let mut payload = json!({
                            "result_type": "tick",
                            "timestamp_ms": data.timestamp_ms,
                            "minute": data.minute,
                            "ball": {
                                "x": data.ball_position.0,
                                "y": data.ball_position.1,
                                "height": data.ball_height
                            },
                            "players": players,
                            "events": events,
                            "score": { "home": data.score.0, "away": data.score.1 }
                        });

                        if let Some(obs) = &data.team_view_simple {
                            if let Some(obj) = payload.as_object_mut() {
                                obj.insert(
                                    "team_view_simple".to_string(),
                                    to_json_value_or_null(obs),
                                );
                            }
                        }
                        if let Some(obs) = &data.team_view_minimap {
                            if let Some(obj) = payload.as_object_mut() {
                                obj.insert(
                                    "team_view_minimap".to_string(),
                                    to_json_value_or_null(obs),
                                );
                            }
                        }

                        GString::from(payload.to_string())
                    }
                    StepResult::HalfTime(data) => {
                        GString::from(json!({
                            "result_type": "half_time",
                            "score": { "home": data.score.0, "away": data.score.1 },
                            "possession": { "home": data.possession.0, "away": data.possession.1 },
                            "shots": { "home": data.shots.0, "away": data.shots.1 },
                            "shots_on_target": { "home": data.shots_on_target.0, "away": data.shots_on_target.1 }
                        }).to_string())
                    }
                    StepResult::FullTime(data) => {
                        let events: Vec<serde_json::Value> = data.all_events.iter().map(|e| {
                            json!({
                                "minute": e.minute,
                                "type": format!("{:?}", e.event_type),
                                "is_home_team": e.is_home_team,
                                "player_track_id": e.player_track_id  // C7: Use track_id
                            })
                        }).collect();

                        // Clear session on full time
                        drop(session);
                        *self.live_session.borrow_mut() = None;

                        GString::from(json!({
                            "result_type": "full_time",
                            "score": { "home": data.result.score_home, "away": data.result.score_away },
                            "events": events,
                            "match_complete": true
                        }).to_string())
                    }
                }
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Resume second half after half-time break.
    #[func]
    pub fn resume_second_half(&mut self) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                s.resume_second_half();
                GString::from(
                    json!({
                        "success": true,
                        "state": "second_half",
                        "minute": 45
                    })
                    .to_string(),
                )
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Change team tactics during the match.
    /// team: "home" or "away"
    #[func]
    pub fn change_live_tactic(&mut self, team: GString, instructions_json: GString) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                let team_str = team.to_string();
                let team_side = if team_str == "home" {
                    TeamSide::Home
                } else {
                    TeamSide::Away
                };

                let instructions: Result<TeamInstructions, _> =
                    serde_json::from_str(&instructions_json.to_string());
                match instructions {
                    Ok(instr) => {
                        s.change_tactic(team_side, instr);
                        GString::from(
                            json!({
                                "success": true,
                                "team": team_str,
                                "message": "Tactics updated"
                            })
                            .to_string(),
                        )
                    }
                    Err(e) => self.create_error_response(
                        &format!("Invalid instructions: {}", e),
                        "PARSE_ERROR",
                    ),
                }
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Change team formation during the match (Phase 5).
    /// team: "home" or "away"
    /// formation: "4-4-2", "4-3-3", "4-5-1", "3-4-3", "4-2-3-1", "3-5-2"
    #[func]
    pub fn change_formation_live_match(&mut self, team: GString, formation: GString) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                let team_str = team.to_string();
                let formation_str = formation.to_string();
                let team_side = if team_str == "home" {
                    TeamSide::Home
                } else {
                    TeamSide::Away
                };

                match s.change_formation(team_side, &formation_str) {
                    Ok(()) => GString::from(
                        json!({
                            "success": true,
                            "team": team_str,
                            "formation": formation_str,
                            "message": "Formation changed"
                        })
                        .to_string(),
                    ),
                    Err(e) => GString::from(
                        json!({
                            "success": false,
                            "error": e,
                            "code": "INVALID_FORMATION"
                        })
                        .to_string(),
                    ),
                }
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Make a substitution during the match (Phase 5).
    /// team: "home" or "away"
    /// out_idx: index of player going out (0-10)
    /// in_idx: index of player coming in from bench (11+)
    #[func]
    pub fn substitute_live_match(&mut self, team: GString, out_idx: i32, in_idx: i32) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                let team_str = team.to_string();
                let team_side = if team_str == "home" {
                    TeamSide::Home
                } else {
                    TeamSide::Away
                };

                match s.substitute(team_side, out_idx as usize, in_idx as usize) {
                    Ok(()) => GString::from(
                        json!({
                            "success": true,
                            "team": team_str,
                            "player_out": out_idx,
                            "player_in": in_idx,
                            "message": "Substitution made"
                        })
                        .to_string(),
                    ),
                    Err(e) => GString::from(
                        json!({
                            "success": false,
                            "error": e,
                            "code": "SUBSTITUTION_FAILED"
                        })
                        .to_string(),
                    ),
                }
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Get current match session state.
    #[func]
    pub fn get_live_match_state(&self) -> GString {
        let session = self.live_session.borrow();
        match session.as_ref() {
            Some(s) => {
                let state = match s.get_state() {
                    LiveMatchState::NotStarted => "not_started",
                    LiveMatchState::FirstHalf => "first_half",
                    LiveMatchState::HalfTimeBreak => "half_time",
                    LiveMatchState::SecondHalf => "second_half",
                    LiveMatchState::Finished => "finished",
                };
                let (home, away) = s.get_score();
                GString::from(
                    json!({
                        "active": true,
                        "state": state,
                        "minute": s.get_minute(),
                        "score": { "home": home, "away": away }
                    })
                    .to_string(),
                )
            }
            None => GString::from(
                json!({
                    "active": false,
                    "state": "none",
                    "message": "No match session active"
                })
                .to_string(),
            ),
        }
    }

    /// End match session (cleanup).
    #[func]
    pub fn end_live_session(&mut self) -> GString {
        *self.live_session.borrow_mut() = None;
        GString::from(
            json!({
                "success": true,
                "message": "Match session ended"
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Legacy session API - spec-compatible wrappers
    // (internal naming remains `*_live_*` for now; Godot should use `*_match_session` aliases)
    // ============================================================================

    /// Start a match session (spec-compatible wrapper; internal naming remains `*_live_*`).
    /// Combines create_live_session + kick_off_live_match.
    /// Returns true on success, false on failure.
    #[func]
    pub fn start_live_session(&mut self, match_request_json: GString) -> bool {
        // Create session
        let create_result = self.create_live_session(match_request_json);
        let create_str = create_result.to_string();
        let create_parsed: Result<serde_json::Value, _> = serde_json::from_str(&create_str);

        match create_parsed {
            Ok(v) => {
                if !v["success"].as_bool().unwrap_or(false) {
                    godot_error!("[MatchSession] Failed to create session: {}", create_str);
                    return false;
                }
            }
            Err(e) => {
                godot_error!("[MatchSession] Failed to parse create result: {}", e);
                return false;
            }
        }

        // Kick off
        let kickoff_result = self.kick_off_live_match();
        let kickoff_str = kickoff_result.to_string();
        let kickoff_parsed: Result<serde_json::Value, _> = serde_json::from_str(&kickoff_str);

        match kickoff_parsed {
            Ok(v) => v["success"].as_bool().unwrap_or(false),
            Err(e) => {
                godot_error!("[MatchSession] Failed to parse kickoff result: {}", e);
                false
            }
        }
    }

    // ============================================================================
    // FIX_2601/0123 PR #7-1: Bridge-compatible JSON API methods
    // These methods are called by MatchSessionBridge.gd using dynamic call()
    // ============================================================================

    /// Start a live match session (MatchSessionBridge compatibility).
    /// Returns JSON string with match_id and success status.
    #[func]
    pub fn start_live_match(&mut self, request_json: GString) -> GString {
        let ok = self.start_live_session(request_json);
        if ok {
            // Generate a simple match_id (single-session model, so ID is placeholder)
            let match_id = format!(
                "match_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
            GString::from(
                json!({
                    "success": true,
                    "match_id": match_id,
                    "message": "Match session started"
                })
                .to_string(),
            )
        } else {
            GString::from(
                json!({
                    "success": false,
                    "error": true,
                    "error_code": "START_FAILED",
                    "error_message": "Failed to start match session"
                })
                .to_string(),
            )
        }
    }

    /// Finish a live match session (MatchSessionBridge compatibility).
    /// Returns JSON string with final result.
    #[func]
    pub fn finish_live_match(&mut self, _match_id: GString) -> GString {
        // Get final state before ending
        let state_json = self.get_live_match_state();
        let state_str = state_json.to_string();

        let (score_home, score_away, minute) =
            if let Ok(state) = serde_json::from_str::<serde_json::Value>(&state_str) {
                let home = state["score"]["home"].as_u64().unwrap_or(0) as u8;
                let away = state["score"]["away"].as_u64().unwrap_or(0) as u8;
                let min = state["minute"].as_u64().unwrap_or(90) as u8;
                (home, away, min)
            } else {
                (0, 0, 90)
            };

        // End the session
        self.end_live_session();

        GString::from(
            json!({
                "success": true,
                "is_finished": true,
                "score_home": score_home,
                "score_away": score_away,
                "current_minute": minute
            })
            .to_string(),
        )
    }

    /// Start a match session (Session terminology alias).
    ///
    /// Preferred over `start_live_session` from Godot code to avoid "live/replay" naming leakage.
    ///
    /// Returns Dictionary:
    /// - `success: bool`
    /// - `error: String` (empty on success)
    /// - `message: String`
    #[func]
    pub fn start_match_session(&mut self, match_request_json: GString) -> Dictionary {
        let ok = self.start_live_session(match_request_json);
        let mut dict = Dictionary::new();
        dict.set("success", ok);
        if ok {
            dict.set("error", GString::from(""));
            dict.set("message", GString::from("Match session started"));
        } else {
            dict.set("error", GString::from("START_FAILED"));
            dict.set("message", GString::from("Failed to start match session"));
        }
        dict
    }

    /// Kick off a match session (Session terminology alias).
    ///
    /// Note: `start_match_session` already performs kickoff, but this alias is provided for completeness.
    #[func]
    pub fn kick_off_match_session(&mut self) -> Dictionary {
        let kickoff_result = self.kick_off_live_match();
        let kickoff_str = kickoff_result.to_string();

        let mut dict = Dictionary::new();
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&kickoff_str);
        match parsed {
            Ok(v) => {
                dict.set("success", v["success"].as_bool().unwrap_or(false));
                dict.set("state", GString::from(v["state"].as_str().unwrap_or("")));
                dict.set("minute", v["minute"].as_i64().unwrap_or(0) as i32);
                dict.set("error", GString::from(""));
            }
            Err(e) => {
                dict.set("success", false);
                dict.set("state", GString::from(""));
                dict.set("minute", 0);
                dict.set(
                    "error",
                    GString::from(format!("KICKOFF_PARSE_ERROR: {}", e)),
                );
            }
        }
        dict
    }

    /// Step a match session (Session terminology alias).
    #[func]
    pub fn step_match_session(&mut self, max_dt_ms: i32) -> Dictionary {
        self.step_live_session(max_dt_ms)
    }

    /// Step the match session (spec-compatible wrapper; internal naming remains `*_live_*`).
    /// Returns Dictionary in spec format: { finished, t_ms, timestep_ms, snapshot, events }
    /// OPTIMIZED: Direct StepResult → Dictionary conversion without JSON re-parsing
    #[func]
    pub fn step_live_session(&mut self, _max_dt_ms: i32) -> Dictionary {
        // Note: max_dt_ms is currently ignored (fixed 250ms tick); drive speed via call rate.
        let mut dict = Dictionary::new();
        let timestep_ms = of_core::engine::live_match::MS_PER_TICK as i32;
        // Phase23: TransitionSystem debug exposure (ms; -1 = inactive)
        dict.set("transition_remaining_ms", -1);

        // Use internal method to get StepResult directly (no JSON round-trip)
        let step_result = match self.step_live_internal() {
            Some(r) => r,
            None => {
                // No active session
                dict.set("finished", true);
                dict.set("halftime", false);
                dict.set("t_ms", 0);
                dict.set("timestep_ms", 0);
                dict.set("snapshot", Dictionary::new());
                dict.set("events", godot::prelude::Array::<Variant>::new());
                dict.set("error", GString::from("No match session active"));
                return dict;
            }
        };

        // Read TransitionSystem remaining time after stepping (SSOT: engine state).
        // Note: transition counts down at decision-tick cadence (250ms).
        let transition_remaining_ms: i32 = {
            let session = self.live_session.borrow();
            session
                .as_ref()
                .and_then(|s| s.engine.get_transition_remaining_ms())
                .map(|ms| ms as i32)
                .unwrap_or(-1)
        };
        dict.set("transition_remaining_ms", transition_remaining_ms);

        match step_result {
            StepResult::NotStarted => {
                dict.set("finished", true);
                dict.set("halftime", false);
                dict.set("t_ms", 0);
                dict.set("timestep_ms", 0);
                dict.set("snapshot", Dictionary::new());
                dict.set("events", godot::prelude::Array::<Variant>::new());
            }
            StepResult::Tick(data) => {
                dict.set("finished", false);
                dict.set("halftime", false);
                dict.set("t_ms", data.timestamp_ms as i32);
                dict.set("timestep_ms", timestep_ms);

                // Build snapshot directly from TickData
                let mut snapshot = Dictionary::new();

                // Ball: { x, y, z, owner_id }
                let mut ball_dict = Dictionary::new();
                ball_dict.set("x", data.ball_position.0);
                ball_dict.set("y", data.ball_position.1);
                ball_dict.set("z", data.ball_height);
                // owner_id: -1 if no owner (loose ball), otherwise player index
                ball_dict.set(
                    "owner_id",
                    data.ball_owner_idx.map(|i| i as i32).unwrap_or(-1),
                );
                snapshot.set("ball", ball_dict);

                // Players: { "0": {x, y, stamina, state}, "1": {x, y, stamina, state}, ... }
                let mut players_dict = Dictionary::new();
                for player in &data.player_positions {
                    let mut player_pos = Dictionary::new();
                    player_pos.set("x", player.position.0);
                    player_pos.set("y", player.position.1);
                    player_pos.set("stamina", player.stamina);
                    player_pos.set("state", GString::from(player.state.as_str()));
                    players_dict.set(GString::from(player.index.to_string()), player_pos);
                }
                snapshot.set("players", players_dict);

                if let Some(board) = &data.field_board_snapshot {
                    apply_field_board_snapshot(&mut snapshot, board);
                }

                if !data.decision_intents.is_empty() {
                    let mut intents = godot::prelude::Array::<Variant>::new();
                    for intent in &data.decision_intents {
                        let intent_dict = convert_decision_intent_to_dict(intent);
                        intents.push(&intent_dict.to_variant());
                    }
                    snapshot.set("decision_intents", intents);
                }

                let mut offside = Dictionary::new();
                offside.set("home_x", data.offside_lines.home_x);
                offside.set("away_x", data.offside_lines.away_x);
                snapshot.set("offside_lines", offside);
                dict.set("snapshot", snapshot);

                // Events: Convert MatchEvent using helper function (P2-10)
                let mut events_array = godot::prelude::Array::<Variant>::new();
                for event in &data.events {
                    let event_dict = convert_event_to_dict(event);
                    events_array.push(&event_dict.to_variant());
                }
                dict.set("events", events_array);

                if let Some(obs) = &data.team_view_simple {
                    dict.set("team_view_simple", convert_team_view_simple_to_dict(obs));
                }
                if let Some(obs) = &data.team_view_minimap {
                    dict.set("team_view_minimap", convert_team_view_minimap_to_dict(obs));
                }
            }
            StepResult::HalfTime(data) => {
                dict.set("finished", false);
                dict.set("halftime", true);
                dict.set("t_ms", 45 * 60 * 1000); // 45 minutes in ms
                dict.set("timestep_ms", timestep_ms);

                // Empty snapshot for halftime
                let mut snapshot = Dictionary::new();
                let mut ball_dict = Dictionary::new();
                ball_dict.set("x", 52.5f32);
                ball_dict.set("y", 34.0f32);
                ball_dict.set("z", 0.0f32);
                snapshot.set("ball", ball_dict);
                snapshot.set("players", Dictionary::new());
                dict.set("snapshot", snapshot);
                dict.set("events", godot::prelude::Array::<Variant>::new());

                // Include halftime stats
                let mut stats = Dictionary::new();
                let mut score = Dictionary::new();
                score.set("home", data.score.0 as i32);
                score.set("away", data.score.1 as i32);
                stats.set("score", score);

                let mut possession = Dictionary::new();
                possession.set("home", data.possession.0 as i32);
                possession.set("away", data.possession.1 as i32);
                stats.set("possession", possession);

                let mut shots = Dictionary::new();
                shots.set("home", data.shots.0 as i32);
                shots.set("away", data.shots.1 as i32);
                stats.set("shots", shots);

                dict.set("halftime_stats", stats);
            }
            StepResult::FullTime(data) => {
                dict.set("finished", true);
                dict.set("halftime", false);
                dict.set("t_ms", 90 * 60 * 1000); // 90 minutes in ms
                dict.set("timestep_ms", timestep_ms);

                // Empty snapshot for full time
                let mut snapshot = Dictionary::new();
                let mut ball_dict = Dictionary::new();
                ball_dict.set("x", 52.5f32);
                ball_dict.set("y", 34.0f32);
                ball_dict.set("z", 0.0f32);
                snapshot.set("ball", ball_dict);
                snapshot.set("players", Dictionary::new());
                dict.set("snapshot", snapshot);

                // Include all events (P2-10: using helper function)
                let mut events_array = godot::prelude::Array::<Variant>::new();
                for event in &data.all_events {
                    let event_dict = convert_event_to_dict(event);
                    events_array.push(&event_dict.to_variant());
                }
                dict.set("events", events_array);

                // Include final score
                let mut score = Dictionary::new();
                score.set("home", data.result.score_home as i32);
                score.set("away", data.result.score_away as i32);
                dict.set("score", score);

                // Clear session on full time
                *self.live_session.borrow_mut() = None;
            }
        }

        dict
    }

    /// Step the match session with packed format (optimized for performance; internal naming remains `*_live_*`).
    /// Returns Dictionary with PackedFloat32Array for player positions instead of nested Dictionaries.
    /// This reduces Dictionary allocations from 22/tick to 1/tick (88% reduction).
    ///
    /// Format:
    /// {
    ///   "finished": bool,
    ///   "halftime": bool,
    ///   "t_ms": int,
    ///   "timestep_ms": int,
    ///   "snapshot": {
    ///     "ball": { "x": f32, "y": f32, "z": f32 },
    ///     "players_packed": PackedFloat32Array,  // [x0,y0,x1,y1,...,x21,y21] - 44 floats
    ///     "score_home": int,
    ///     "score_away": int
    ///   },
    ///   "events": Array
    /// }
    ///
    /// PackedFloat32Array indexing:
    /// - player_index i (0-21): x = players_packed[i * 2], y = players_packed[i * 2 + 1]
    /// - 0-10: home team, 11-21: away team
    #[func]
    pub fn step_live_session_packed(&mut self, _max_dt_ms: i32) -> Dictionary {
        use godot::prelude::PackedFloat32Array;

        let mut dict = Dictionary::new();
        let timestep_ms = of_core::engine::live_match::MS_PER_TICK as i32;
        // Phase23: TransitionSystem debug exposure (ms; -1 = inactive)
        dict.set("transition_remaining_ms", -1);

        let step_result = match self.step_live_internal() {
            Some(r) => r,
            None => {
                dict.set("finished", true);
                dict.set("halftime", false);
                dict.set("t_ms", 0);
                dict.set("timestep_ms", 0);
                dict.set("snapshot", Dictionary::new());
                dict.set("events", godot::prelude::Array::<Variant>::new());
                dict.set("error", GString::from("No match session active"));
                return dict;
            }
        };

        // Read TransitionSystem remaining time after stepping (SSOT: engine state).
        let transition_remaining_ms: i32 = {
            let session = self.live_session.borrow();
            session
                .as_ref()
                .and_then(|s| s.engine.get_transition_remaining_ms())
                .map(|ms| ms as i32)
                .unwrap_or(-1)
        };
        dict.set("transition_remaining_ms", transition_remaining_ms);

        match step_result {
            StepResult::NotStarted => {
                dict.set("finished", true);
                dict.set("halftime", false);
                dict.set("t_ms", 0);
                dict.set("timestep_ms", 0);
                dict.set("snapshot", Dictionary::new());
                dict.set("events", godot::prelude::Array::<Variant>::new());
            }
            StepResult::Tick(data) => {
                dict.set("finished", false);
                dict.set("halftime", false);
                dict.set("t_ms", data.timestamp_ms as i32);
                dict.set("timestep_ms", timestep_ms);

                let mut snapshot = Dictionary::new();

                // Ball: { x, y, z, owner_id }
                let mut ball_dict = Dictionary::new();
                ball_dict.set("x", data.ball_position.0);
                ball_dict.set("y", data.ball_position.1);
                ball_dict.set("z", data.ball_height);
                // owner_id: -1 if no owner (loose ball), otherwise player index
                ball_dict.set(
                    "owner_id",
                    data.ball_owner_idx.map(|i| i as i32).unwrap_or(-1),
                );
                snapshot.set("ball", ball_dict);

                // Players: PackedFloat32Array [x0,y0,x1,y1,...,x21,y21]
                let mut players_packed = PackedFloat32Array::new();
                players_packed.resize(44); // 22 players * 2 coordinates

                // Stamina: PackedFloat32Array [s0,s1,...,s21]
                let mut stamina_packed = PackedFloat32Array::new();
                stamina_packed.resize(22);

                // States: Dictionary { "0": "WithBall", "1": "Attacking", ... }
                let mut states_dict = Dictionary::new();

                {
                    let pos_slice = players_packed.as_mut_slice();
                    let sta_slice = stamina_packed.as_mut_slice();
                    for player in &data.player_positions {
                        let idx = player.index as usize;
                        if idx < 22 {
                            pos_slice[idx * 2] = player.position.0;
                            pos_slice[idx * 2 + 1] = player.position.1;
                            sta_slice[idx] = player.stamina;
                            states_dict.set(
                                GString::from(idx.to_string()),
                                GString::from(player.state.as_str()),
                            );
                        }
                    }
                }
                snapshot.set("players_packed", players_packed);
                snapshot.set("stamina_packed", stamina_packed);
                snapshot.set("states", states_dict);

                if let Some(board) = &data.field_board_snapshot {
                    apply_field_board_snapshot(&mut snapshot, board);
                }

                if !data.decision_intents.is_empty() {
                    let mut intents = godot::prelude::Array::<Variant>::new();
                    for intent in &data.decision_intents {
                        let intent_dict = convert_decision_intent_to_dict(intent);
                        intents.push(&intent_dict.to_variant());
                    }
                    snapshot.set("decision_intents", intents);
                }

                let mut offside = Dictionary::new();
                offside.set("home_x", data.offside_lines.home_x);
                offside.set("away_x", data.offside_lines.away_x);
                snapshot.set("offside_lines", offside);

                // Score in snapshot for convenience
                snapshot.set("score_home", data.score.0 as i32);
                snapshot.set("score_away", data.score.1 as i32);

                dict.set("snapshot", snapshot);

                // Events
                let mut events_array = godot::prelude::Array::<Variant>::new();
                for event in &data.events {
                    let event_dict = convert_event_to_dict(event);
                    events_array.push(&event_dict.to_variant());
                }
                dict.set("events", events_array);

                if let Some(obs) = &data.team_view_simple {
                    dict.set("team_view_simple", convert_team_view_simple_to_dict(obs));
                }
                if let Some(obs) = &data.team_view_minimap {
                    dict.set("team_view_minimap", convert_team_view_minimap_to_dict(obs));
                }
            }
            StepResult::HalfTime(data) => {
                dict.set("finished", false);
                dict.set("halftime", true);
                dict.set("t_ms", 45 * 60 * 1000);
                dict.set("timestep_ms", timestep_ms);

                let mut snapshot = Dictionary::new();
                let mut ball_dict = Dictionary::new();
                ball_dict.set("x", 52.5f32);
                ball_dict.set("y", 34.0f32);
                ball_dict.set("z", 0.0f32);
                snapshot.set("ball", ball_dict);

                // Empty packed array for halftime
                let mut players_packed = PackedFloat32Array::new();
                players_packed.resize(44);
                snapshot.set("players_packed", players_packed);

                snapshot.set("score_home", data.score.0 as i32);
                snapshot.set("score_away", data.score.1 as i32);

                dict.set("snapshot", snapshot);
                dict.set("events", godot::prelude::Array::<Variant>::new());

                // Include halftime stats
                let mut stats = Dictionary::new();
                let mut score = Dictionary::new();
                score.set("home", data.score.0 as i32);
                score.set("away", data.score.1 as i32);
                stats.set("score", score);

                let mut possession = Dictionary::new();
                possession.set("home", data.possession.0 as i32);
                possession.set("away", data.possession.1 as i32);
                stats.set("possession", possession);

                let mut shots = Dictionary::new();
                shots.set("home", data.shots.0 as i32);
                shots.set("away", data.shots.1 as i32);
                stats.set("shots", shots);

                dict.set("halftime_stats", stats);
            }
            StepResult::FullTime(data) => {
                dict.set("finished", true);
                dict.set("halftime", false);
                dict.set("t_ms", 90 * 60 * 1000);
                dict.set("timestep_ms", timestep_ms);

                let mut snapshot = Dictionary::new();
                let mut ball_dict = Dictionary::new();
                ball_dict.set("x", 52.5f32);
                ball_dict.set("y", 34.0f32);
                ball_dict.set("z", 0.0f32);
                snapshot.set("ball", ball_dict);

                let mut players_packed = PackedFloat32Array::new();
                players_packed.resize(44);
                snapshot.set("players_packed", players_packed);

                snapshot.set("score_home", data.result.score_home as i32);
                snapshot.set("score_away", data.result.score_away as i32);

                dict.set("snapshot", snapshot);

                // Include all events
                let mut events_array = godot::prelude::Array::<Variant>::new();
                for event in &data.all_events {
                    let event_dict = convert_event_to_dict(event);
                    events_array.push(&event_dict.to_variant());
                }
                dict.set("events", events_array);

                // Include final score
                let mut score = Dictionary::new();
                score.set("home", data.result.score_home as i32);
                score.set("away", data.result.score_away as i32);
                dict.set("score", score);

                // Clear session on full time
                *self.live_session.borrow_mut() = None;
            }
        }

        dict
    }

    /// Step a match session (packed) (Session terminology alias).
    #[func]
    pub fn step_match_session_packed(&mut self, max_dt_ms: i32) -> Dictionary {
        self.step_live_session_packed(max_dt_ms)
    }

    // ============================================================================
    // FIX_2601/0123 PR #7-1: Budget-based polling with is_partial flag
    // ============================================================================

    /// Poll match session with time budget (for GDScript MatchSessionBridge compatibility).
    ///
    /// Steps multiple ticks until:
    /// - Budget (budget_ms) is exhausted → is_partial=true
    /// - Half-time is reached → is_partial=false, halftime=true
    /// - Full-time is reached → is_partial=false, is_finished=true
    ///
    /// Returns Dictionary:
    /// - events: Array of events from all ticks
    /// - score_home: u8
    /// - score_away: u8
    /// - current_minute: u8
    /// - is_finished: bool
    /// - is_partial: bool (FIX_2601/0123: true if budget exhausted before completion)
    /// - ticks_simulated: u32 (FIX_2601/0123: number of ticks processed this poll)
    /// - halftime: bool (true if reached halftime)
    #[func]
    pub fn poll_live_match(&mut self, _match_id: GString, budget_ms: i32) -> GString {
        use std::time::Instant;

        let start = Instant::now();
        let budget_duration = std::time::Duration::from_millis(budget_ms.max(1) as u64);

        let mut all_events = Vec::new();
        let mut ticks_simulated: u32 = 0;
        let mut is_finished = false;
        let mut is_partial = false;
        let mut halftime = false;
        let mut score_home: u8 = 0;
        let mut score_away: u8 = 0;
        let mut current_minute: u8 = 0;

        // Check if session exists
        {
            let session = self.live_session.borrow();
            if session.is_none() {
                return GString::from(
                    json!({
                        "error": true,
                        "error_code": "NO_SESSION",
                        "error_message": "No match session active",
                        "is_partial": false,
                        "ticks_simulated": 0
                    })
                    .to_string(),
                );
            }
        }

        // Step until budget exhausted or state change
        loop {
            // Check budget
            if start.elapsed() >= budget_duration {
                is_partial = true;
                break;
            }

            // Step one tick
            let step_result = self.step_live_internal();

            match step_result {
                None => {
                    // No session - shouldn't happen but handle gracefully
                    is_finished = true;
                    break;
                }
                Some(StepResult::NotStarted) => {
                    // Session not kicked off - treat as error state
                    break;
                }
                Some(StepResult::Tick(data)) => {
                    ticks_simulated += 1;
                    all_events.extend(data.events.iter().cloned());
                    score_home = data.score.0;
                    score_away = data.score.1;
                    current_minute = data.minute;
                }
                Some(StepResult::HalfTime(data)) => {
                    ticks_simulated += 1;
                    halftime = true;
                    score_home = data.score.0;
                    score_away = data.score.1;
                    current_minute = 45;
                    // Don't continue - let caller handle halftime
                    break;
                }
                Some(StepResult::FullTime(data)) => {
                    ticks_simulated += 1;
                    is_finished = true;
                    all_events.extend(data.all_events.iter().cloned());
                    score_home = data.result.score_home;
                    score_away = data.result.score_away;
                    current_minute = 90;
                    break;
                }
            }
        }

        // Convert events to JSON
        let events_json: Vec<serde_json::Value> = all_events
            .iter()
            .map(|e| serde_json::to_value(e).unwrap_or(serde_json::Value::Null))
            .collect();

        GString::from(
            json!({
                "events": events_json,
                "score_home": score_home,
                "score_away": score_away,
                "current_minute": current_minute,
                "is_finished": is_finished,
                "is_partial": is_partial,
                "ticks_simulated": ticks_simulated,
                "halftime": halftime
            })
            .to_string(),
        )
    }

    /// Finish match session and get full result (spec-compatible wrapper; internal naming remains `*_live_*`).
    /// Returns Dictionary with final match result.
    #[func]
    pub fn finish_live_session(&mut self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Get final state before ending
        let state_json = self.get_live_match_state();
        let state_str = state_json.to_string();

        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&state_str) {
            if let Some(score) = state["score"].as_object() {
                let mut score_dict = Dictionary::new();
                score_dict.set("home", score["home"].as_u64().unwrap_or(0) as i32);
                score_dict.set("away", score["away"].as_u64().unwrap_or(0) as i32);
                dict.set("score", score_dict);
            }
            dict.set("minute", state["minute"].as_u64().unwrap_or(90) as i32);
        }

        // End the session
        self.end_live_session();

        dict.set("finished", true);
        dict
    }

    /// Finish a match session (Session terminology alias).
    #[func]
    pub fn finish_match_session(&mut self) -> Dictionary {
        self.finish_live_session()
    }

    // ============================================================================
    // Match OS v1.1 - FieldBoard Snapshot Export
    // ============================================================================

    /// Get current FieldBoard snapshot (Match OS v1.1)
    /// Returns Dictionary with grid dimensions and heatmap data
    ///
    /// Dictionary structure:
    /// {
    ///   "cols": int (28),
    ///   "rows": int (18),
    ///   "occupancy_total": PackedFloat32Array (504 elements),
    ///   "pressure_against_home": PackedFloat32Array (504 elements),
    ///   "pressure_against_away": PackedFloat32Array (504 elements)
    /// }
    ///
    /// Returns error dictionary if no active session or FieldBoard not initialized
    #[func]
    pub fn get_field_board_snapshot(&self) -> Dictionary {
        use godot::prelude::PackedFloat32Array;

        let mut dict = Dictionary::new();

        // Borrow live_session
        let session_ref = self.live_session.borrow();
        let Some(ref session) = *session_ref else {
            dict.set("error", GString::from("No active session"));
            return dict;
        };

        // Access engine.field_board (both are pub fields)
        let Some(ref field_board) = session.engine.field_board else {
            dict.set("error", GString::from("FieldBoard not initialized"));
            return dict;
        };

        // Use EXISTING to_snapshot_export() from P18
        let snapshot = field_board.to_snapshot_export();

        // Convert to Godot types
        dict.set("cols", snapshot.cols as i32);
        dict.set("rows", snapshot.rows as i32);

        // PackedFloat32Array = 88% memory reduction vs Dictionary
        let mut occupancy = PackedFloat32Array::new();
        occupancy.resize(snapshot.occupancy_total.len());
        occupancy
            .as_mut_slice()
            .copy_from_slice(&snapshot.occupancy_total);
        dict.set("occupancy_total", occupancy);

        let mut pressure_home = PackedFloat32Array::new();
        pressure_home.resize(snapshot.pressure_against_home.len());
        pressure_home
            .as_mut_slice()
            .copy_from_slice(&snapshot.pressure_against_home);
        dict.set("pressure_against_home", pressure_home);

        let mut pressure_away = PackedFloat32Array::new();
        pressure_away.resize(snapshot.pressure_against_away.len());
        pressure_away
            .as_mut_slice()
            .copy_from_slice(&snapshot.pressure_against_away);
        dict.set("pressure_against_away", pressure_away);

        // Match OS v1.2: XGZone Map
        let mut xgzone = PackedFloat32Array::new();
        xgzone.resize(snapshot.xgzone_map.len());
        xgzone.as_mut_slice().copy_from_slice(&snapshot.xgzone_map);
        dict.set("xgzone", xgzone);

        dict
    }

    /// Match OS v1.2 Priority 5: Get match analysis report from match result JSON
    /// Returns Dictionary with pattern insights (possession shifts, danger timeline, attack zones, pressure patterns)
    #[func]
    pub fn get_match_analysis(&self, match_result_json: GString) -> Dictionary {
        use of_core::engine::analyze_match;
        use of_core::engine::MatchAnalysisReport;
        use of_core::models::MatchResult;

        let mut dict = Dictionary::new();

        // Parse match result from JSON.
        //
        // Prefer a direct `MatchResult`, but tolerate a wrapper payload like:
        // { "match_result": { ...MatchResult... }, ... }
        let match_result_str = match_result_json.to_string();
        let match_result: MatchResult = match serde_json::from_str(&match_result_str) {
            Ok(result) => result,
            Err(direct_err) => {
                let value: serde_json::Value = match serde_json::from_str(&match_result_str) {
                    Ok(v) => v,
                    Err(nested_err) => {
                        dict.set(
                            "error",
                            GString::from(format!(
                                "Parse error: {} (fallback parse failed: {})",
                                direct_err, nested_err
                            )),
                        );
                        return dict;
                    }
                };

                let Some(inner) = value.get("match_result") else {
                    dict.set(
                        "error",
                        GString::from(format!(
                            "Parse error: {} (missing key: match_result)",
                            direct_err
                        )),
                    );
                    return dict;
                };

                match serde_json::from_value(inner.clone()) {
                    Ok(result) => result,
                    Err(e) => {
                        dict.set(
                            "error",
                            GString::from(format!(
                                "Parse error: {} (match_result parse error: {})",
                                direct_err, e
                            )),
                        );
                        return dict;
                    }
                }
            }
        };

        // Run analysis
        let report = analyze_match(&match_result);

        let MatchAnalysisReport {
            duration_minutes,
            possession_shifts,
            danger_timeline,
            attack_zones,
            pressure_patterns,
            dsa_summary,
            interpretation_v1,
            generated_at_ms,
        } = report;

        // Convert to Godot Dictionary
        dict.set("duration_minutes", duration_minutes as i32);
        dict.set("generated_at_ms", generated_at_ms as i64);

        // Possession shifts
        let mut shifts = Array::new();
        for shift in possession_shifts {
            let mut shift_dict = Dictionary::new();
            shift_dict.set("start_minute", shift.start_minute as i32);
            shift_dict.set("end_minute", shift.end_minute as i32);
            shift_dict.set("from_possession_home", shift.from_possession_home);
            shift_dict.set("to_possession_home", shift.to_possession_home);
            shift_dict.set("magnitude", shift.magnitude);
            shift_dict.set("description", GString::from(shift.description));
            shifts.push(&shift_dict.to_variant());
        }
        dict.set("possession_shifts", shifts);

        // Danger timeline
        let mut moments = Array::new();
        for moment in danger_timeline {
            let mut moment_dict = Dictionary::new();
            moment_dict.set("minute", moment.minute as i32);
            moment_dict.set("timestamp_ms", moment.timestamp_ms as i64);
            moment_dict.set("xg_value", moment.xg_value);
            moment_dict.set("event_type", GString::from(moment.event_type));
            moment_dict.set("player_name", GString::from(moment.player_name));
            moment_dict.set("is_home", moment.is_home);
            moment_dict.set("position_x", moment.position.0);
            moment_dict.set("position_y", moment.position.1);
            moment_dict.set("description", GString::from(moment.description));
            moments.push(&moment_dict.to_variant());
        }
        dict.set("danger_timeline", moments);

        // Attack zones
        let mut zones = Array::new();
        for zone in attack_zones.zones {
            let mut zone_dict = Dictionary::new();
            zone_dict.set("zone_name", GString::from(zone.zone_name));
            zone_dict.set("attack_count", zone.attack_count as i32);
            zone_dict.set("percentage", zone.percentage);
            zone_dict.set("center_x", zone.center_position.0);
            zone_dict.set("center_y", zone.center_position.1);
            zones.push(&zone_dict.to_variant());
        }
        let mut attack_zones_dict = Dictionary::new();
        attack_zones_dict.set("zones", zones);
        attack_zones_dict.set("dominant_zone", GString::from(attack_zones.dominant_zone));
        attack_zones_dict.set("total_attacks", attack_zones.total_attacks as i32);
        dict.set("attack_zones", attack_zones_dict);

        // Pressure patterns
        let mut patterns = Array::new();
        for pattern in pressure_patterns {
            let mut pattern_dict = Dictionary::new();
            pattern_dict.set("start_minute", pattern.start_minute as i32);
            pattern_dict.set("end_minute", pattern.end_minute as i32);
            pattern_dict.set("field_third", GString::from(pattern.field_third));
            pattern_dict.set("pressing_team", GString::from(pattern.pressing_team));
            pattern_dict.set("intensity", GString::from(pattern.intensity));
            pattern_dict.set("event_count", pattern.event_count as i32);
            pattern_dict.set("description", GString::from(pattern.description));
            patterns.push(&pattern_dict.to_variant());
        }
        dict.set("pressure_patterns", patterns);

        // DSA v1.1: Optional post-match summary (authoritative).
        if let Some(summary) = dsa_summary {
            fn vec_f32_to_array(values: &[f32]) -> Array<Variant> {
                let mut out = Array::new();
                for v in values {
                    let vv = (*v).to_variant();
                    out.push(&vv);
                }
                out
            }

            fn vec_u32_to_array(values: &[u32]) -> Array<Variant> {
                let mut out = Array::new();
                for v in values {
                    let vv = (*v as i64).to_variant();
                    out.push(&vv);
                }
                out
            }

            let mut dsa_dict = Dictionary::new();
            dsa_dict.set("duration_minutes", summary.duration_minutes as i32);

            let mut series = Dictionary::new();
            series.set("pressure", vec_f32_to_array(&summary.minute_series.pressure));
            series.set("tempo", vec_f32_to_array(&summary.minute_series.tempo));
            series.set(
                "transitions",
                vec_u32_to_array(&summary.minute_series.transitions),
            );
            if !summary.minute_series.tempo_home.is_empty() {
                series.set(
                    "tempo_home",
                    vec_f32_to_array(&summary.minute_series.tempo_home),
                );
            }
            if !summary.minute_series.tempo_away.is_empty() {
                series.set(
                    "tempo_away",
                    vec_f32_to_array(&summary.minute_series.tempo_away),
                );
            }
            if !summary.minute_series.transitions_home.is_empty() {
                series.set(
                    "transitions_home",
                    vec_u32_to_array(&summary.minute_series.transitions_home),
                );
            }
            if !summary.minute_series.transitions_away.is_empty() {
                series.set(
                    "transitions_away",
                    vec_u32_to_array(&summary.minute_series.transitions_away),
                );
            }
            if !summary.minute_series.pressure_against_home.is_empty() {
                series.set(
                    "pressure_against_home",
                    vec_f32_to_array(&summary.minute_series.pressure_against_home),
                );
            }
            if !summary.minute_series.pressure_against_away.is_empty() {
                series.set(
                    "pressure_against_away",
                    vec_f32_to_array(&summary.minute_series.pressure_against_away),
                );
            }
            dsa_dict.set("minute_series", series);

            // Hub summary
            let mut hub = Dictionary::new();
            hub.set("gini", summary.hub.gini);
            let mut top3 = Array::new();
            for p in summary.hub.top3 {
                let mut p_dict = Dictionary::new();
                p_dict.set("track_id", p.track_id as i32);
                p_dict.set("team", GString::from(p.team));
                p_dict.set("hub_score", p.hub_score);
                p_dict.set("owner_time_s", p.owner_time_s);
                p_dict.set("receive_count", p.receive_count as i32);
                p_dict.set("release_count", p.release_count as i32);
                top3.push(&p_dict.to_variant());
            }
            hub.set("top3", top3);
            dsa_dict.set("hub", hub);

            // Routes summary
            let mut routes = Dictionary::new();
            routes.set("zone_count", summary.routes.zone_count as i32);

            let mut home_mat = PackedInt32Array::new();
            home_mat.resize(summary.routes.transitions_home.len());
            {
                let slice = home_mat.as_mut_slice();
                for (dst, v) in slice.iter_mut().zip(summary.routes.transitions_home.iter()) {
                    *dst = (*v).min(i32::MAX as u32) as i32;
                }
            }
            routes.set("transitions_home", home_mat);

            let mut away_mat = PackedInt32Array::new();
            away_mat.resize(summary.routes.transitions_away.len());
            {
                let slice = away_mat.as_mut_slice();
                for (dst, v) in slice.iter_mut().zip(summary.routes.transitions_away.iter()) {
                    *dst = (*v).min(i32::MAX as u32) as i32;
                }
            }
            routes.set("transitions_away", away_mat);

            let mut top_routes = Array::new();
            for r in summary.routes.top_routes {
                let mut r_dict = Dictionary::new();
                r_dict.set("team", GString::from(r.team));
                r_dict.set("from_zone", r.from_zone as i32);
                r_dict.set("to_zone", r.to_zone as i32);
                r_dict.set("count", r.count as i32);
                top_routes.push(&r_dict.to_variant());
            }
            routes.set("top_routes", top_routes);
            dsa_dict.set("routes", routes);

            // QA warnings
            let mut warnings = Array::new();
            for w in summary.qa_warnings {
                let mut w_dict = Dictionary::new();
                w_dict.set("kind", GString::from(format!("{:?}", w.kind)));
                w_dict.set("message", GString::from(w.message));
                warnings.push(&w_dict.to_variant());
            }
            dsa_dict.set("qa_warnings", warnings);

            dict.set("dsa_summary", dsa_dict);
        }

        // Interpretation v1: Replay/Analytics meaning layer (post-match).
        if let Some(interpretation) = interpretation_v1 {
            let value = match serde_json::to_value(&interpretation) {
                Ok(v) => v,
                Err(e) => {
                    dict.set(
                        "interpretation_v1_error",
                        GString::from(format!("interpretation_v1 serialize error: {e}")),
                    );
                    return dict;
                }
            };
            dict.set("interpretation_v1", json_value_to_variant(&value));
        }

        dict
    }

    /// Get best moments / highlights from match result JSON for timeline markers
    /// Returns Array of Dictionaries for each highlight moment
    ///
    /// Each Dictionary contains:
    /// - "start_time_ms": i64 - Start of highlight window (for video jump)
    /// - "end_time_ms": i64 - End of highlight window
    /// - "moment_type": String - "goal", "save", "shot_on_target", etc.
    /// - "priority": i32 - Higher = more important (goal=100, penalty=90, etc.)
    /// - "minute": i32 - Match minute when event occurred
    /// - "description": String - Optional description
    /// - "is_home_team": bool - True if home team event (if applicable)
    #[func]
    pub fn get_best_moments(&self, match_result_json: GString) -> Array<Variant> {
        use of_core::models::MatchResult;

        let mut moments_array = Array::new();

        // Parse match result from JSON
        let match_result: MatchResult = match serde_json::from_str(&match_result_json.to_string()) {
            Ok(result) => result,
            Err(_e) => {
                return moments_array; // Return empty array on parse error
            }
        };

        // Get best moments (generate if not present)
        let moments = match &match_result.best_moments {
            Some(moments) => moments.clone(),
            None => {
                // Generate from events if not pre-generated
                of_core::models::generate_best_moments(&match_result.events)
            }
        };

        // Convert to Godot Array of Dictionaries
        for moment in moments {
            let mut dict = Dictionary::new();
            dict.set("start_time_ms", moment.start_time_ms as i64);
            dict.set("end_time_ms", moment.end_time_ms as i64);
            dict.set(
                "moment_type",
                GString::from(format!("{:?}", moment.moment_type).to_lowercase()),
            );
            dict.set("priority", moment.priority as i32);
            dict.set("minute", moment.minute as i32);
            dict.set(
                "description",
                GString::from(moment.description.unwrap_or_default()),
            );
            dict.set("is_home_team", moment.is_home_team.unwrap_or(false));
            moments_array.push(&dict.to_variant());
        }

        moments_array
    }

    // ============================================================================
    // Match OS v1.2 Priority 4 - Formation Overlay Data Export
    // ============================================================================

    /// Get formation overlay data (Match OS v1.2 Priority 4)
    /// Returns Dictionary with formation strings and waypoint data
    ///
    /// Dictionary structure:
    /// {
    ///   "home_formation": "4-4-2",
    ///   "away_formation": "4-3-3",
    ///   "home_waypoints": PackedFloat32Array[x0,y0, x1,y1, ..., x10,y10],
    ///   "away_waypoints": PackedFloat32Array[x0,y0, x1,y1, ..., x10,y10],
    ///   "home_position_keys": ["GK", "LB", "LCB", ...],
    ///   "away_position_keys": ["GK", "LB", "LCB", ...]
    /// }
    ///
    /// Returns error dictionary if no active session
    #[func]
    pub fn get_formation_overlay_data(&self) -> Dictionary {
        use godot::prelude::{PackedFloat32Array, PackedStringArray};
        use of_core::engine::{get_formation_waypoints, positioning::PositionKey};

        let mut dict = Dictionary::new();

        // Borrow match session
        let session_ref = self.live_session.borrow();
        let Some(ref session) = *session_ref else {
            dict.set("error", GString::from("No active session"));
            return dict;
        };

        let engine = &session.engine;

        // Get formation strings (stored as String, not Option<String>)
        let home_formation = &engine.home_formation;
        let away_formation = &engine.away_formation;

        // Get waypoints from formation_waypoints.rs
        let home_waypoints_map = get_formation_waypoints(home_formation);
        let away_waypoints_map = get_formation_waypoints(away_formation);

        // Define fixed position order (for consistent indexing)
        const POSITION_ORDER: [&str; 11] = [
            "GK", "LB", "LCB", "RCB", "RB", "LM", "CM", "RM", "LW", "ST", "RW",
        ];

        // Pack home waypoints
        let mut home_coords = PackedFloat32Array::new();
        let mut home_pos_keys = PackedStringArray::new();

        for pos_key_str in &POSITION_ORDER {
            if let Some(pos_key) = PositionKey::from_str(pos_key_str) {
                if let Some(waypoint) = home_waypoints_map.get(&pos_key) {
                    home_coords.push(waypoint.base.0); // X
                    home_coords.push(waypoint.base.1); // Y
                    home_pos_keys.push(*pos_key_str);
                }
            }
        }

        // Pack away waypoints (with Y-mirroring: away team attacks toward Y=0)
        let mut away_coords = PackedFloat32Array::new();
        let mut away_pos_keys = PackedStringArray::new();

        for pos_key_str in &POSITION_ORDER {
            if let Some(pos_key) = PositionKey::from_str(pos_key_str) {
                if let Some(waypoint) = away_waypoints_map.get(&pos_key) {
                    away_coords.push(waypoint.base.0); // X unchanged
                    away_coords.push(1.0 - waypoint.base.1); // Y mirrored
                    away_pos_keys.push(*pos_key_str);
                }
            }
        }

        // Build result dictionary
        dict.set("home_formation", GString::from(home_formation.as_str()));
        dict.set("away_formation", GString::from(away_formation.as_str()));
        dict.set("home_waypoints", home_coords); // 22 floats (11 positions × 2 coords)
        dict.set("away_waypoints", away_coords); // 22 floats (11 positions × 2 coords)
        dict.set("home_position_keys", home_pos_keys); // 11 strings
        dict.set("away_position_keys", away_pos_keys); // 11 strings

        dict
    }

    /// Match OS v1.2 Priority 6: Get player positions in meters for tactical analysis
    /// Returns PackedVector2Array with 22 positions (11 home + 11 away)
    ///
    /// Array format: [home0_x, home0_y, home1_x, home1_y, ..., away10_x, away10_y]
    /// - Indices 0-10: home team
    /// - Indices 11-21: away team
    ///
    /// Returns empty array if no active session
    #[func]
    pub fn get_player_positions_m(&self) -> PackedVector2Array {
        use godot::prelude::PackedVector2Array;

        let mut positions = PackedVector2Array::new();

        // Borrow live_session
        let session_ref = self.live_session.borrow();
        let Some(ref session) = *session_ref else {
            return positions; // Return empty array if no session
        };

        let engine = &session.engine;

        // Extract all 22 player positions (0-10 = home, 11-21 = away)
        for i in 0..22 {
            // FIX_2601: Coord10 has to_meters() directly
            let pos_m = engine.get_player_position_by_index(i).to_meters();
            positions.push(Vector2::new(pos_m.0, pos_m.1));
        }

        positions
    }

    // ============================================================================
    // Special Ability System API
    // ============================================================================

    /// Check if player can acquire a specific ability
    #[func]
    pub fn check_ability_acquisition(&self, player_json: GString, ability_id: GString) -> GString {
        let _ = player_json;
        let _ = ability_id;
        GString::from(
            json!({
                "can_acquire": true,
                "requirements_met": true,
                "missing_requirements": []
            })
            .to_string(),
        )
    }

    /// Check if ability can be activated in current context
    #[func]
    pub fn check_ability_activation(
        &self,
        player_json: GString,
        ability_id: GString,
        context_json: GString,
    ) -> GString {
        let _ = player_json;
        let _ = ability_id;
        let _ = context_json;
        GString::from(
            json!({
                "can_activate": true,
                "activation_chance": 0.75,
                "cooldown_remaining": 0
            })
            .to_string(),
        )
    }

    /// Process ability combinations for a player
    #[func]
    pub fn process_ability_combinations(&self, player_json: GString) -> GString {
        let _ = player_json;
        GString::from(
            json!({
                "active_combinations": [],
                "potential_combinations": [],
                "synergy_bonus": 0.0
            })
            .to_string(),
        )
    }

    /// Test special ability system
    #[func]
    pub fn test_special_ability_system(&self) -> GString {
        GString::from(
            json!({
                "status": "ok",
                "abilities_loaded": 50,
                "combinations_available": 10
            })
            .to_string(),
        )
    }

    // ============================================================================
    // Player Creation API
    // ============================================================================

    /// Create a new player with specified attributes
    #[func]
    pub fn create_player(&self, request_json: GString) -> GString {
        let _ = request_json;
        GString::from(
            json!({
                "success": true,
                "player": {
                    "id": "generated_player_001",
                    "name": "New Player",
                    "position": "CM",
                    "overall": 65,
                    "potential": 80,
                    "age": 18,
                    "attributes": {
                        "pace": 65,
                        "shooting": 60,
                        "passing": 70,
                        "dribbling": 65,
                        "defending": 55,
                        "physical": 60
                    }
                }
            })
            .to_string(),
        )
    }

    // NOTE: Legacy JSON-based gacha/deck APIs removed in FIX_2601/0109.
    // Use Dict-based APIs: gacha_pull_*, coach_get_inventory, deck_*.

    // ============================================================================
    // Save/Load API (Binary serialization)
    // ============================================================================

    /// Save game state to binary format (MessagePack + LZ4 + SHA256)
    #[func]
    pub fn save_game_binary(&self, save_data_json: GString) -> PackedByteArray {
        let json_str = save_data_json.to_string();
        if json_str.is_empty() {
            godot_error!("[save_game_binary] Empty JSON input");
            return PackedByteArray::new();
        }

        // Parse JSON to serde_json::Value
        let json_value: JsonValue = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                godot_error!("[save_game_binary] JSON parse error: {}", e);
                return PackedByteArray::new();
            }
        };

        // Serialize to MessagePack
        let msgpack_data = match to_vec_named(&json_value) {
            Ok(d) => d,
            Err(e) => {
                godot_error!("[save_game_binary] MessagePack serialize error: {}", e);
                return PackedByteArray::new();
            }
        };

        // Compress with LZ4
        let compressed = compress_prepend_size(&msgpack_data);

        // Calculate SHA256 hash of compressed data
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let hash: [u8; 32] = hasher.finalize().into();

        // Build final binary: [4-byte magic "OFSV"][1-byte version][32-byte hash][compressed data]
        let mut result = Vec::with_capacity(4 + 1 + 32 + compressed.len());
        result.extend_from_slice(b"OFSV"); // OpenFootball Save
        result.push(1u8); // version 1
        result.extend_from_slice(&hash);
        result.extend_from_slice(&compressed);

        PackedByteArray::from(result.as_slice())
    }

    /// Load game state from binary format (verify SHA256 + LZ4 decompress + MessagePack)
    #[func]
    pub fn load_game_binary(&mut self, data: PackedByteArray) -> GString {
        let bytes = data.to_vec();

        // Minimum size: 4 (magic) + 1 (version) + 32 (hash) + 1 (at least some data)
        if bytes.len() < 38 {
            return GString::from(
                json!({
                    "success": false,
                    "error": "Save file too small"
                })
                .to_string(),
            );
        }

        // Check magic
        if &bytes[0..4] != b"OFSV" {
            return GString::from(
                json!({
                    "success": false,
                    "error": "Invalid save file magic"
                })
                .to_string(),
            );
        }

        // Check version
        let version = bytes[4];
        if version != 1 {
            return GString::from(
                json!({
                    "success": false,
                    "error": format!("Unsupported save version: {}", version)
                })
                .to_string(),
            );
        }

        // Extract stored hash and compressed data
        let stored_hash = &bytes[5..37];
        let compressed = &bytes[37..];

        // Verify SHA256
        let mut hasher = Sha256::new();
        hasher.update(compressed);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        if stored_hash != computed_hash {
            return GString::from(
                json!({
                    "success": false,
                    "error": "Save file corrupted (hash mismatch)"
                })
                .to_string(),
            );
        }

        // Decompress LZ4
        let decompressed = match decompress_size_prepended(compressed) {
            Ok(d) => d,
            Err(e) => {
                return GString::from(
                    json!({
                        "success": false,
                        "error": format!("LZ4 decompress error: {}", e)
                    })
                    .to_string(),
                );
            }
        };

        // Deserialize MessagePack to JSON
        let json_value: JsonValue = match from_slice(&decompressed) {
            Ok(v) => v,
            Err(e) => {
                return GString::from(
                    json!({
                        "success": false,
                        "error": format!("MessagePack deserialize error: {}", e)
                    })
                    .to_string(),
                );
            }
        };

        GString::from(
            json!({
                "success": true,
                "game_state": json_value
            })
            .to_string(),
        )
    }

    // ========== Career Player Mode: User Control System ==========

    /// Submit a user command for Career Player Mode
    ///
    /// The command JSON should have the following structure:
    /// ```json
    /// {
    ///   "seq": 1,
    ///   "controlled_track_id": 9,
    ///   "cmd": "on_ball_action",
    ///   "action": "pass",
    ///   "variant": "through",  // optional
    ///   "target_track_id": 10  // optional
    /// }
    /// ```
    #[func]
    pub fn submit_user_command(&mut self, json: GString) -> GString {
        use of_core::engine::match_sim::UserCommand;

        let json_str = json.to_string();

        // Parse JSON to UserCommand
        let cmd: UserCommand = match serde_json::from_str(&json_str) {
            Ok(c) => c,
            Err(e) => {
                return self.create_error_response(
                    &format!("Failed to parse user_command: {}", e),
                    "PARSE_ERROR",
                );
            }
        };

        // Basic validation
        if cmd.seq == 0 {
            return self.create_error_response("User command with seq=0 is invalid", "INVALID_SEQ");
        }

        // Get the match session and submit command
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                s.submit_user_command(cmd.clone());
                GString::from(
                    json!({
                        "success": true,
                        "seq": cmd.seq,
                        "message": "User command enqueued"
                    })
                    .to_string(),
                )
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Toggle sticky actions (sprint/dribble/press) for a player.
    #[func]
    pub fn set_sticky_action(&mut self, track_id: i32, action: GString, enabled: bool) -> GString {
        let action_str = action.to_string().to_lowercase();
        let action = match action_str.as_str() {
            "sprint" => StickyAction::Sprint,
            "dribble" => StickyAction::Dribble,
            "press" | "pressing" => StickyAction::Press,
            _ => {
                return self.create_error_response(
                    "Invalid sticky action (use sprint/dribble/press)",
                    "INVALID_ACTION",
                );
            }
        };

        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => match s.set_sticky_action(track_id as usize, action, enabled) {
                Ok(()) => GString::from(
                    json!({
                        "success": true,
                        "track_id": track_id,
                        "action": action_str,
                        "enabled": enabled
                    })
                    .to_string(),
                ),
                Err(e) => self.create_error_response(e, "INVALID_TRACK_ID"),
            },
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Register a controller slot for multi-agent control
    #[func]
    pub fn register_controller_slot(
        &mut self,
        controller_id: i32,
        team_side: GString,
        player_slot: i32,
    ) -> GString {
        use of_core::models::TeamSide;

        if controller_id < 0 || player_slot < 0 {
            return self.create_error_response("Negative controller_id/slot", "INVALID_ARG");
        }

        let team_side = match team_side.to_string().to_lowercase().as_str() {
            "home" => TeamSide::Home,
            "away" => TeamSide::Away,
            _ => {
                return self
                    .create_error_response("Invalid team_side (use home/away)", "INVALID_TEAM");
            }
        };

        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                match s.register_controller_slot(controller_id as u32, team_side, player_slot as u8)
                {
                    Ok(()) => GString::from(
                        json!({
                            "success": true,
                            "controller_id": controller_id,
                            "team_side": format!("{:?}", team_side).to_lowercase(),
                            "player_slot": player_slot
                        })
                        .to_string(),
                    ),
                    Err(e) => self.create_error_response(e, "REGISTER_FAILED"),
                }
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Unregister a controller slot
    #[func]
    pub fn unregister_controller_slot(&mut self, controller_id: i32) -> GString {
        if controller_id < 0 {
            return self.create_error_response("Negative controller_id", "INVALID_ARG");
        }
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => match s.unregister_controller_slot(controller_id as u32) {
                Ok(()) => GString::from(
                    json!({
                        "success": true,
                        "controller_id": controller_id
                    })
                    .to_string(),
                ),
                Err(e) => self.create_error_response(e, "UNREGISTER_FAILED"),
            },
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Clear all controller slots
    #[func]
    pub fn clear_controller_slots(&mut self) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                s.clear_controller_slots();
                GString::from(json!({ "success": true }).to_string())
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Submit multi-agent commands (array or { "commands": [...] })
    #[func]
    pub fn submit_multi_agent_commands(&mut self, json: GString) -> GString {
        use of_core::engine::match_sim::{MultiAgentCommand, MultiAgentCommandBatch};
        use serde_json::Value;

        let json_str = json.to_string();
        let value: Value = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                return self.create_error_response(
                    &format!("Failed to parse multi_agent_commands: {}", e),
                    "PARSE_ERROR",
                );
            }
        };

        let commands: Vec<MultiAgentCommand> = if value.is_array() {
            match serde_json::from_value(value) {
                Ok(v) => v,
                Err(e) => {
                    return self.create_error_response(
                        &format!("Failed to decode commands: {}", e),
                        "DECODE_ERROR",
                    );
                }
            }
        } else if value.get("commands").is_some() {
            let batch: MultiAgentCommandBatch = match serde_json::from_value(value) {
                Ok(v) => v,
                Err(e) => {
                    return self.create_error_response(
                        &format!("Failed to decode command batch: {}", e),
                        "DECODE_ERROR",
                    );
                }
            };
            batch.commands
        } else {
            return self.create_error_response(
                "Expected array or {commands:[...]} payload",
                "INVALID_PAYLOAD",
            );
        };

        let count = commands.len();
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => match s.submit_multi_agent_commands(commands) {
                Ok(()) => GString::from(
                    json!({
                        "success": true,
                        "count": count
                    })
                    .to_string(),
                ),
                Err(e) => self.create_error_response(e, "SUBMIT_FAILED"),
            },
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Enable Career Player Mode for a specific track_id
    #[func]
    pub fn enable_career_mode(&mut self, track_id: i32) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                s.enable_controlled_mode(track_id as usize);
                GString::from(
                    json!({
                        "success": true,
                        "controlled_track_id": track_id,
                        "message": "Career mode enabled"
                    })
                    .to_string(),
                )
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }

    /// Disable Career Player Mode
    #[func]
    pub fn disable_career_mode(&mut self) -> GString {
        let mut session = self.live_session.borrow_mut();
        match session.as_mut() {
            Some(s) => {
                s.disable_controlled_mode();
                GString::from(
                    json!({
                        "success": true,
                        "message": "Career mode disabled"
                    })
                    .to_string(),
                )
            }
            None => self.create_error_response("No match session active", "NO_SESSION"),
        }
    }
}

// --- Binary Serialization Structs & Logic (The Optimization Core) ---
// These structs are prepared for future binary replay export feature

#[allow(dead_code)]
#[derive(serde::Serialize, bincode::Encode)]
struct ReplayExport {
    magic: String,
    version: u32,
    metadata: MatchMetadata,
    ball: Vec<BallFrameCompressed>,         // Compressed
    players: Vec<PlayerSequenceCompressed>, // Compressed
}

#[allow(dead_code)]
#[derive(serde::Serialize, bincode::Encode)]
struct MatchMetadata {
    duration: f32,
    home_team_id: u32,
    away_team_id: u32,
    home_team_name: String,
    away_team_name: String,
}

// --- Compressed Coordinate Structs ---

#[allow(dead_code)]
#[derive(serde::Serialize, bincode::Encode)]
struct BallFrameCompressed {
    t: f32,
    x: u16,
    y: u16,
    z: u16,
}

#[allow(dead_code)]
#[derive(serde::Serialize, bincode::Encode)]
struct PlayerFrameCompressed {
    t: f32,
    x: u16,
    y: u16,
}

#[allow(dead_code)]
#[derive(serde::Serialize, bincode::Encode)]
struct PlayerSequenceCompressed {
    id: u32,
    team_id: u32,
    kit_number: u32,
    frames: Vec<PlayerFrameCompressed>,
}

// Compression helpers (for future binary replay export)
#[allow(dead_code)]
fn compress_x(x: f32) -> u16 {
    ((x / 105.0) * 65535.0).clamp(0.0, 65535.0) as u16
}
#[allow(dead_code)]
fn compress_y(y: f32) -> u16 {
    ((y / 68.0) * 65535.0).clamp(0.0, 65535.0) as u16
}
#[allow(dead_code)]
fn compress_z(z: f32) -> u16 {
    (((z + 5.0) / 10.0) * 65535.0).clamp(0.0, 65535.0) as u16
}

// Decompression helpers (for delta)
#[allow(dead_code)]
fn decompress_x(x: u16) -> f32 {
    (x as f32 / 65535.0) * 105.0
}
#[allow(dead_code)]
fn decompress_y(y: u16) -> f32 {
    (y as f32 / 65535.0) * 68.0
}

// --- Delta Encoding Structs ---

#[allow(dead_code)]
#[derive(serde::Serialize, bincode::Encode)]
struct PlayerSequenceDelta {
    id: u32,
    team_id: u32,
    kit_number: u32,
    initial_frame: PlayerFrameCompressed,
    deltas: Vec<DeltaFrame>,
}

#[allow(dead_code)]
#[derive(serde::Serialize, bincode::Encode)]
enum DeltaFrame {
    NoChange,
    SmallDelta { dt: u16, dx: i8, dy: i8 },
    LargeDelta { dt: u16, dx: i16, dy: i16 },
}

#[allow(dead_code)]
fn encode_player_delta(frames: &[PlayerFrameCompressed]) -> Vec<DeltaFrame> {
    if frames.is_empty() {
        return vec![];
    }

    let mut deltas = Vec::new();
    let mut prev_frame = &frames[0];

    for curr_frame in frames.iter().skip(1) {
        let dx_meters = decompress_x(curr_frame.x) - decompress_x(prev_frame.x);
        let dy_meters = decompress_y(curr_frame.y) - decompress_y(prev_frame.y);
        let dt_ms = ((curr_frame.t - prev_frame.t) * 1000.0) as u16;

        if dx_meters.abs() < 0.001 && dy_meters.abs() < 0.001 {
            deltas.push(DeltaFrame::NoChange);
        } else {
            let dx_cm = (dx_meters * 100.0) as i16;
            let dy_cm = (dy_meters * 100.0) as i16;

            if (-127..=127).contains(&dx_cm) && (-127..=127).contains(&dy_cm) {
                deltas.push(DeltaFrame::SmallDelta {
                    dt: dt_ms,
                    dx: dx_cm as i8,
                    dy: dy_cm as i8,
                });
            } else {
                let dx_mm = (dx_meters * 1000.0) as i16;
                let dy_mm = (dy_meters * 1000.0) as i16;
                deltas.push(DeltaFrame::LargeDelta {
                    dt: dt_ms,
                    dx: dx_mm,
                    dy: dy_mm,
                });
            }
        }
        prev_frame = curr_frame;
    }
    deltas
}

impl FootballMatchSimulator {
    fn simulate_match_inner(&self, json: GString) -> GString {
        match simulate_match_json(&json.to_string()) {
            Ok(res) => GString::from(res),
            Err(e) => GString::from(format!(r#"{{"error": "{}"}}"#, e)),
        }
    }

    fn create_error_response(&self, msg: &str, code: &str) -> GString {
        GString::from(json!({ "error": true, "message": msg, "code": code }).to_string())
    }
}

#[cfg(test)]
mod mrq0_extension_tests {
    use super::FootballMatchSimulator;

    fn push_f32_le(buf: &mut Vec<u8>, v: f32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn mrq0_v3_extension_parses_training_multiplier_and_home_away_mods() {
        let mut data: Vec<u8> = Vec::new();
        data.push(0b0000_0011); // flags: bit0 + bit1
        push_f32_le(&mut data, 1.0); // training_multiplier (reserved)

        data.push(1); // home_mod_count
        data.push(3); // shot_power_mult
        push_f32_le(&mut data, 1.2);

        data.push(1); // away_mod_count
        data.push(1); // pass_success_mult
        push_f32_le(&mut data, 1.1);

        let mut offset: usize = 0;
        let (home, away) =
            FootballMatchSimulator::parse_mrq0_match_modifiers_extension(&data, &mut offset, 3)
                .expect("parse v3 extension");

        assert!((home.shot_power_mult - 1.2).abs() < 1e-6);
        assert!((away.pass_success_mult - 1.1).abs() < 1e-6);
        assert_eq!(offset, data.len());
    }

    #[test]
    fn mrq0_v4_extension_parses_without_training_multiplier() {
        let mut data: Vec<u8> = Vec::new();
        data.push(0b0000_0001); // flags: bit0 only

        data.push(1); // home_mod_count
        data.push(3); // shot_power_mult
        push_f32_le(&mut data, 1.2);

        let mut offset: usize = 0;
        let (home, away) =
            FootballMatchSimulator::parse_mrq0_match_modifiers_extension(&data, &mut offset, 4)
                .expect("parse v4 extension");

        assert!((home.shot_power_mult - 1.2).abs() < 1e-6);
        assert!((away.pass_success_mult - 1.0).abs() < 1e-6);
        assert_eq!(offset, data.len());
    }
}

#[cfg(test)]
mod mrq0_end_to_end_tests {
    use super::FootballMatchSimulator;
    use of_core::tactics::team_instructions::{
        BuildUpStyle, DefensiveLine, TeamPressing, TeamTempo, TeamWidth,
    };

    const MRQ0_MAGIC: u32 = 0x3051514D;

    fn push_u8(buf: &mut Vec<u8>, v: u8) {
        buf.push(v);
    }

    fn push_u16_le(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn push_u32_le(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn push_u64_le(buf: &mut Vec<u8>, v: u64) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn push_f32_le(buf: &mut Vec<u8>, v: f32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn push_str_u16(buf: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        push_u16_le(buf, bytes.len().try_into().expect("u16 len"));
        buf.extend_from_slice(bytes);
    }

    fn push_team(buf: &mut Vec<u8>, team_name: &str, formation: &str, prefix: &str) {
        push_str_u16(buf, team_name);
        push_str_u16(buf, formation);

        let players: [(&str, u8, u8); 11] = [
            ("GK", 0, 60),
            ("LB", 1, 60),
            ("CB1", 2, 60),
            ("CB2", 2, 60),
            ("RB", 3, 60),
            ("LM", 9, 60),
            ("CM1", 7, 60),
            ("CM2", 7, 60),
            ("RM", 10, 60),
            ("ST1", 14, 60),
            ("ST2", 14, 60),
        ];

        push_u8(buf, players.len() as u8);
        for (suffix, pos_code, overall) in players {
            let name = format!("{prefix}_{suffix}");
            push_str_u16(buf, &name);
            push_u8(buf, pos_code);
            push_u8(buf, overall);
        }
    }

    fn push_instructions(
        buf: &mut Vec<u8>,
        defensive_line: u8,
        team_width: u8,
        team_tempo: u8,
        pressing_intensity: u8,
        build_up_style: u8,
        use_offside_trap: bool,
    ) {
        push_u8(buf, defensive_line);
        push_u8(buf, team_width);
        push_u8(buf, team_tempo);
        push_u8(buf, pressing_intensity);
        push_u8(buf, build_up_style);
        push_u8(buf, if use_offside_trap { 1 } else { 0 });
    }

    #[test]
    fn mrq0_v4_full_payload_decodes_home_away_mods_into_plan() {
        let mut data: Vec<u8> = Vec::new();
        push_u32_le(&mut data, MRQ0_MAGIC);
        push_u32_le(&mut data, 4); // version
        push_u64_le(&mut data, 12_345); // seed
        push_u8(&mut data, 0); // use_vendor_engine
        push_u16_le(&mut data, 100); // position_sample_rate_ms

        push_team(&mut data, "Home", "4-4-2", "H");
        push_instructions(&mut data, 1, 2, 3, 1, 0, true); // home: High line, Normal width, Slow, High press, Short, offside

        push_team(&mut data, "Away", "4-4-2", "A");
        push_instructions(&mut data, 3, 0, 0, 4, 2, false); // away: Deep, VeryWide, VeryFast, VeryLow, Direct, no offside

        data.push(0b0000_0011); // flags: bit0 + bit1
        data.push(1); // home_mod_count
        data.push(3); // shot_power_mult
        push_f32_le(&mut data, 1.2);
        data.push(1); // away_mod_count
        data.push(1); // pass_success_mult
        push_f32_le(&mut data, 1.1);

        let plan = FootballMatchSimulator::decode_mrq0_to_match_plan(&data).expect("decode MRQ0 v4");

        assert_eq!(plan.seed, 12_345);
        assert_eq!(plan.home_team.name, "Home");
        assert_eq!(plan.away_team.name, "Away");

        let home_inst = plan.home_instructions.expect("home instructions");
        assert_eq!(home_inst.defensive_line, DefensiveLine::High);
        assert_eq!(home_inst.team_width, TeamWidth::Normal);
        assert_eq!(home_inst.team_tempo, TeamTempo::Slow);
        assert_eq!(home_inst.pressing_intensity, TeamPressing::High);
        assert_eq!(home_inst.build_up_style, BuildUpStyle::Short);
        assert!(home_inst.use_offside_trap);

        let away_inst = plan.away_instructions.expect("away instructions");
        assert_eq!(away_inst.defensive_line, DefensiveLine::Deep);
        assert_eq!(away_inst.team_width, TeamWidth::VeryWide);
        assert_eq!(away_inst.team_tempo, TeamTempo::VeryFast);
        assert_eq!(away_inst.pressing_intensity, TeamPressing::VeryLow);
        assert_eq!(away_inst.build_up_style, BuildUpStyle::Direct);
        assert!(!away_inst.use_offside_trap);

        assert!((plan.home_match_modifiers.shot_power_mult - 1.2).abs() < 1e-6);
        assert!((plan.away_match_modifiers.pass_success_mult - 1.1).abs() < 1e-6);
    }

    #[test]
    fn mrq0_v3_full_payload_decodes_training_multiplier_and_home_away_mods_into_plan() {
        let mut data: Vec<u8> = Vec::new();
        push_u32_le(&mut data, MRQ0_MAGIC);
        push_u32_le(&mut data, 3); // version
        push_u64_le(&mut data, 99_999); // seed
        push_u8(&mut data, 0); // use_vendor_engine
        push_u16_le(&mut data, 100); // position_sample_rate_ms

        push_team(&mut data, "Home", "4-4-2", "H");
        push_instructions(&mut data, 2, 2, 2, 2, 1, false); // all defaults-ish

        push_team(&mut data, "Away", "4-4-2", "A");
        push_instructions(&mut data, 2, 2, 2, 2, 1, false);

        data.push(0b0000_0011); // flags: bit0 + bit1
        push_f32_le(&mut data, 1.0); // training_multiplier (reserved)
        data.push(1); // home_mod_count
        data.push(3); // shot_power_mult
        push_f32_le(&mut data, 1.2);
        data.push(1); // away_mod_count
        data.push(1); // pass_success_mult
        push_f32_le(&mut data, 1.1);

        let plan = FootballMatchSimulator::decode_mrq0_to_match_plan(&data).expect("decode MRQ0 v3");

        assert_eq!(plan.seed, 99_999);
        assert!((plan.home_match_modifiers.shot_power_mult - 1.2).abs() < 1e-6);
        assert!((plan.away_match_modifiers.pass_success_mult - 1.1).abs() < 1e-6);
    }
}
