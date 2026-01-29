## OpenFootballAPI.gd
## Autoload wrapper for OpenFootball Rust engine training and match simulation APIs
## Provides type-safe Godot interface for execute_training_json() and simulate_match_json()
extends Node

## ============================================================================
## ENGINE REFERENCE
## ============================================================================

var rust_engine: Object = null  ## FootballRustEngine singleton

## ============================================================================
## API CONFIGURATION
## ============================================================================

const SCHEMA_VERSION: int = 1
# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _FieldSpec = preload("res://scripts/core/FieldSpec.gd")
const _MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const _InteractiveMatchSetup = preload("res://scripts/core/InteractiveMatchSetup.gd")
const _InteractiveMatchController = preload("res://autoload/domain/InteractiveMatchController.gd")
const _PlayerLibrary = preload("res://scripts/core/PlayerLibrary.gd")
const _MatchSetupExporter = preload("res://scripts/core/MatchSetupExporter.gd")
const FIELD_LENGTH_M := _FieldSpec.FIELD_LENGTH_M
const FIELD_WIDTH_M := _FieldSpec.FIELD_WIDTH_M
const DEFAULT_INTENSITY: String = "normal"
const DEFAULT_SEED_BASE: int = 10000
const MATCH_DEBUG_LOG_DIR := "user://logs"
const MAX_TEXT_EVENT_LOG := 50
const POSITION_TRACKING_DISABLE_ENV := "DISABLE_POSITION_TRACKING"
const SETTINGS_VERBOSE_LOG := "of/dev_verbose_match_log"
const SETTINGS_VENDOR_FLAG := "of/vendor_skills_enabled"
const SETTINGS_FORCE_STORED_EVENTS := "of/dev_force_stored_events"
const ENV_VERBOSE_LOG := "OF_VERBOSE_MATCH_LOG"
const ENV_VENDOR_FLAG := "OF_VENDOR_SKILLS"
const ENV_FORCE_STORED_EVENTS := "OF_FORCE_STORED_EVENTS"

var _position_tracking_enabled: bool = true

## Phase C: MRQ0 v3 Migration Flags
var use_mrq0_v3_instructions: bool = false
var report_mrq0_json_diff: bool = false

## ============================================================================
## TYPE MAPPINGS
## ============================================================================

## Godot TrainingType → OpenFootball target
const TRAINING_TYPE_MAP = {
	"technical": "technical",
	"physical": "physical",
	"tactical": "defending",
	"mental": "mental",
	"balanced": "balanced",
	"passing": "passing",
	"shooting": "shooting",
	"defending": "defending",
	"pace": "pace",
	"recovery": "power",
	# Physical Training
	"physical_endurance": "power",
	"physical_strength": "power",
	"physical_agility": "pace",
	"physical_speed": "pace",
	"physical_recovery": "power",
	# Technical Training
	"technical_ball_control": "technical",
	"technical_passing": "passing",
	"technical_shooting": "shooting",
	"technical_crossing": "passing",
	"technical_set_pieces": "shooting",
	# Tactical Training
	"tactical_positioning": "defending",
	"tactical_team_shape": "defending",
	"tactical_pressing": "defending",
	"tactical_pressing_drills": "defending",
	"tactical_transition": "pace",
	"tactical_transition_play": "pace",
	"tactical_set_pieces_def": "defending",
	"tactical_set_pieces_defensive": "defending",
	# Mental Training
	"mental_concentration": "mental",
	"mental_decision": "mental",
	"mental_decision_making": "mental",
	"mental_leadership": "mental",
	# Match Preparation
	"match_prep": "technical",
	"match_preparation": "technical",
	"match_video": "technical",
	"match_video_analysis": "technical",
	"match_opponent": "technical",
	"match_opponent_specific": "technical"
}

## Godot intensity → OpenFootball intensity
const INTENSITY_MAP = {
	"very_light": "light", "light": "light", "moderate": "normal", "high": "intensive", "very_high": "intensive"
}

## Accepted OpenFootball position codes and aliases
const POSITION_DEFAULT := "MF"
const POSITION_ALIAS_MAP := {
	"GK": "GK",
	"G": "GK",
	"GOALKEEPER": "GK",
	"LB": "LB",
	"DL": "LB",
	"RB": "RB",
	"DR": "RB",
	"CB": "CB",
	"DC": "CB",
	"LCB": "CB",
	"RCB": "CB",
	"LWB": "LWB",
	"RWB": "RWB",
	"WB": "DF",
	"CDM": "CDM",
	"DM": "CDM",
	"DMC": "CDM",
	"CM": "CM",
	"MC": "CM",
	"CAM": "CAM",
	"AM": "CAM",
	"AMC": "CAM",
	"AML": "LW",
	"AMR": "RW",
	"LM": "LM",
	"ML": "LM",
	"RM": "RM",
	"MR": "RM",
	"LW": "LW",
	"WL": "LW",
	"RW": "RW",
	"WR": "RW",
	"W": "LW",
	"WINGER": "LW",
	"CF": "CF",
	"FC": "CF",
	"ST": "ST",
	"SC": "ST",
	"F": "FW",
	"FW": "FW",
	"STRIKER": "ST",
	"ATTACKER": "ST",
	"DF": "DF",
	"D": "DF",
	"DEF": "DF",
	"DEFENDER": "DF",
	"MF": "MF",
	"M": "MF",
	"MID": "MF",
	"MIDFIELDER": "MF"
}
const POSITION_CODES := [
	"GK", "LB", "CB", "RB", "LWB", "RWB", "CDM", "CM", "CAM", "LM", "RM", "LW", "RW", "CF", "ST", "DF", "MF", "FW"
]
const TEAM_REQUIRED_PLAYER_COUNT := 18
const DEFAULT_BENCH_TEMPLATE := ["GK", "DF", "DF", "MF", "MF", "FW", "FW"]
const OPENFOOTBALL_ATTRIBUTE_KEYS := [
	# Technical (14)
	"corners",
	"crossing",
	"dribbling",
	"finishing",
	"first_touch",
	"free_kicks",
	"heading",
	"long_shots",
	"long_throws",
	"marking",
	"passing",
	"penalty_taking",
	"tackling",
	"technique",
	# Mental (14)
	"aggression",
	"anticipation",
	"bravery",
	"composure",
	"concentration",
	"decisions",
	"determination",
	"flair",
	"leadership",
	"off_the_ball",
	"positioning",
	"teamwork",
	"vision",
	"work_rate",
	# Physical (8)
	"acceleration",
	"agility",
	"balance",
	"jumping",
	"natural_fitness",
	"pace",
	"stamina",
	"strength",
	# Goalkeeping (6)
	"aerial_reach",
	"command_of_area",
	"communication",
	"eccentricity",
	"handling",
	"kicking"
]

## Valid formation strings supported by OpenFootball engine
const VALID_FORMATIONS := [
	"4-4-2", "4-3-3", "4-2-3-1", "3-5-2", "3-4-3", "5-3-2", "4-5-1", "4-1-4-1", "3-4-2-1", "5-4-1"
]

## ============================================================================
## ERROR TRACKING
## ============================================================================

var last_error: String = ""
var error_count: int = 0
var _fix02_coord_clamp_warned: bool = false

## ============================================================================
## INITIALIZATION
## ============================================================================


func _ready() -> void:
	_position_tracking_enabled = not OS.has_environment(POSITION_TRACKING_DISABLE_ENV)
	_ensure_engine()
	print("[OpenFootballAPI] Initialized (position_tracking=%s)" % ["ON" if _position_tracking_enabled else "OFF"])


func _process(_delta: float) -> void:
	# Async polling path is disabled while using synchronous simulation.
	# See docs/async_api_plan.md for background.
	pass


func is_position_tracking_enabled() -> bool:
	return _position_tracking_enabled


## ============================================================================
## VALIDATION METHODS
## ============================================================================


## Validate formation string against supported formations
## Returns: true if valid, false otherwise
func validate_formation(formation: String) -> bool:
	return formation in VALID_FORMATIONS


## Sanitize formation string - returns valid formation or default
## Returns: Valid formation string (original or "4-4-2" if invalid)
func sanitize_formation(formation: String) -> String:
	if validate_formation(formation):
		return formation
	else:
		push_warning("[OpenFootballAPI] Invalid formation '%s', using default '4-4-2'" % formation)
		return "4-4-2"


## ============================================================================
## TRAINING API METHODS
## ============================================================================


## Execute personal training for a player
## Returns: { success: bool, response: Dictionary, error: String }
func execute_training(
	player_data: Dictionary, training_type: String, intensity: String = "normal", custom_seed: int = -1
) -> Dictionary:
	# Validate engine
	if not _ensure_engine():
		return _error_result("FootballRustEngine not available")

	# Convert training type
	var of_target = _convert_training_type(training_type)
	if of_target == "":
		return _error_result("Invalid training type: %s" % training_type)

	# Convert intensity
	var of_intensity = _convert_intensity(intensity)

	var seed_value: int = custom_seed if custom_seed > 0 else _generate_seed()
	var resolved_player_id := _resolve_player_id(player_data, seed_value)

	# Build request
	var request = {
		"schema_version": SCHEMA_VERSION,
		"request_type": {"type": "ExecutePersonalTraining", "target": of_target, "intensity": of_intensity},
		"player_id": resolved_player_id,
		"seed": seed_value
	}

	# Convert player data to CorePlayer format
	var player_payload: Dictionary = _build_player_json(player_data)
	if player_payload.is_empty():
		return _error_result("Failed to build training player payload")

	# Build manager state
	var manager_payload: Dictionary = _build_manager_json(player_data)

	# Call Rust API
	var result: Variant = rust_engine.execute_training_json(request, player_payload, manager_payload)

	if typeof(result) != TYPE_DICTIONARY:
		return _error_result("Invalid training response payload")

	var result_dict: Dictionary = result
	if not result_dict.get("success", false):
		var err_message := str(result_dict.get("message", result_dict.get("error", "Unknown training error")))
		return _error_result(err_message)

	var response_variant = result_dict.get("response_type", {})
	if typeof(response_variant) != TYPE_DICTIONARY:
		return _error_result("Invalid training response structure")
	var response_type: Dictionary = response_variant
	if response_type.get("type", "") != "TrainingResult":
		return _error_result("Unexpected response type: %s" % response_type.get("type", "unknown"))

	# Build success result
	return {"success": true, "response": _parse_training_result(response_type), "error": ""}


## Execute rest/recovery for a player
## Returns: { success: bool, response: Dictionary, error: String }
func execute_rest(player_data: Dictionary, forced: bool = false) -> Dictionary:
	if not _ensure_engine():
		return _error_result("FootballRustEngine not available")

	var seed_value: int = _generate_seed()
	var resolved_player_id := _resolve_player_id(player_data, seed_value)

	var request = {
		"schema_version": SCHEMA_VERSION,
		"request_type": {"type": "ExecuteRest", "forced": forced},
		"player_id": resolved_player_id,
		"seed": seed_value
	}

	var player_payload: Dictionary = _build_player_json(player_data)
	if player_payload.is_empty():
		return _error_result("Failed to build rest player payload")

	var manager_payload: Dictionary = _build_manager_json(player_data)

	var result: Variant = rust_engine.execute_training_json(request, player_payload, manager_payload)

	if typeof(result) != TYPE_DICTIONARY:
		return _error_result("Invalid rest response payload")

	var result_dict: Dictionary = result
	if not result_dict.get("success", false):
		var err_message := str(result_dict.get("message", result_dict.get("error", "Failed to execute rest")))
		return _error_result(err_message)

	var response_variant = result_dict.get("response_type", {})
	if typeof(response_variant) != TYPE_DICTIONARY:
		return _error_result("Invalid rest response structure")
	var response_type: Dictionary = response_variant
	if response_type.get("type", "") != "RestResult":
		return _error_result("Unexpected response type: %s" % response_type.get("type", "unknown"))

	return {"success": true, "response": _parse_rest_result(response_type), "error": ""}


## Get training recommendation for player
## Returns: { success: bool, response: Dictionary, error: String }
func get_training_recommendation(player_data: Dictionary) -> Dictionary:
	if not _ensure_engine():
		return _error_result("FootballRustEngine not available")

	var seed_value := _generate_seed()
	var resolved_player_id := _resolve_player_id(player_data, seed_value)

	var request = {
		"schema_version": SCHEMA_VERSION,
		"request_type": {"type": "GetRecommendation"},
		"player_id": resolved_player_id,
		"seed": seed_value
	}

	var player_payload: Dictionary = _build_player_json(player_data)
	if player_payload.is_empty():
		return _error_result("Failed to build recommendation player payload")

	var manager_payload: Dictionary = _build_manager_json(player_data)

	var result: Variant = rust_engine.execute_training_json(request, player_payload, manager_payload)

	if typeof(result) != TYPE_DICTIONARY:
		return _error_result("Invalid recommendation response payload")

	var result_dict: Dictionary = result
	if not result_dict.get("success", false):
		var err_message := str(result_dict.get("message", result_dict.get("error", "Failed to get recommendation")))
		return _error_result(err_message)

	var response_variant = result_dict.get("response_type", {})
	if typeof(response_variant) != TYPE_DICTIONARY:
		return _error_result("Invalid recommendation response structure")
	var response_type: Dictionary = response_variant
	if response_type.get("type", "") != "Recommendation":
		return _error_result("Unexpected response type: %s" % response_type.get("type", "unknown"))

	return {
		"success": true,
		"response": {"recommended": response_type.recommended, "reason": response_type.reason},
		"error": ""
	}


## Get player training status
## Returns: { success: bool, response: Dictionary, error: String }
func get_training_status(player_data: Dictionary) -> Dictionary:
	if not _ensure_engine():
		return _error_result("FootballRustEngine not available")

	var seed_value := _generate_seed()
	var resolved_player_id := _resolve_player_id(player_data, seed_value)

	var request = {
		"schema_version": SCHEMA_VERSION,
		"request_type": {"type": "GetStatus"},
		"player_id": resolved_player_id,
		"seed": seed_value
	}

	var player_payload: Dictionary = _build_player_json(player_data)
	if player_payload.is_empty():
		return _error_result("Failed to build status player payload")

	var manager_payload: Dictionary = _build_manager_json(player_data)

	var result: Variant = rust_engine.execute_training_json(request, player_payload, manager_payload)

	if typeof(result) != TYPE_DICTIONARY:
		return _error_result("Invalid status response payload")

	var result_dict: Dictionary = result
	if not result_dict.get("success", false):
		var err_message := str(result_dict.get("message", result_dict.get("error", "Failed to get status")))
		return _error_result(err_message)

	var response_variant = result_dict.get("response_type", {})
	if typeof(response_variant) != TYPE_DICTIONARY:
		return _error_result("Invalid status response structure")
	var response_type: Dictionary = response_variant
	if response_type.get("type", "") != "Status":
		return _error_result("Unexpected response type: %s" % response_type.get("type", "unknown"))

	return {"success": true, "response": _parse_status_result(response_type), "error": ""}


## ============================================================================
## MATCH SIMULATION API METHODS
## ============================================================================
##
## DEPRECATED METHODS REMOVED (v1.0):
## - simulate_youth_match() - Use simulate_match_with_setup() instead
## - simulate_youth_match_v2_uid_roster() - Use simulate_match_with_setup() instead
## - simulate_quick_match() - Use simulate_match_with_setup() instead
##
## All game modes MUST use:
## - simulate_match_with_setup(match_setup, seed, highlight_level, fast_mode)
## ============================================================================


## Simulate match with MatchSetup SSOT (Game OS mode - Phase 17)
## Returns: { success: bool, response: Dictionary, error: String }
## CRITICAL: Always attaches match_setup to result (Risk 3 mitigation)
func simulate_match_with_setup(
	match_setup: _MatchSetup, fixture_seed: int = -1, highlight_level: String = "full", _fast_mode: bool = false
) -> Dictionary:
	print("[OpenFootballAPI] simulate_match_with_setup (OS mode - Phase 17)")

	if not _ensure_engine():
		return _error_result("FootballRustEngine not available")

	if match_setup == null:
		push_error("[OpenFootballAPI] MatchSetup is null")
		return _error_result("MatchSetup is null")

	# Export MatchSetup to engine format (v2)
	var engine_payload = _MatchSetupExporter.to_json(match_setup)

	# ============================================================
	# MANDATORY VALIDATION CHECKPOINT (P0.5 UID SSOT ENFORCEMENT)
	# Engine boundary: validate payload BEFORE calling Rust
	# Rejects non-engine UIDs (grad:*), accepts csv:<u32>/csv_<u32>/<u32>
	# See: docs/specs/FIX_2512/1223/PLAYER_CACHE_P0_5_STABILIZATION.md
	# ============================================================
	var validation = _MatchSetupExporter.validate_payload(engine_payload)
	if not validation.valid:
		var errors = validation.get("errors", [])
		push_error("[OpenFootballAPI] Payload validation FAILED: %s" % str(errors))
		return {
			"success": false,
			"error": "Payload validation failed: %s" % ("\n".join(errors)),
			"validation_errors": errors
		}

	# Add metadata
	engine_payload["seed"] = fixture_seed if fixture_seed > 0 else _generate_seed()
	engine_payload["include_stored_events"] = _should_include_stored_events()
	engine_payload["enable_position_tracking"] = _position_tracking_enabled
	engine_payload["highlight_level"] = highlight_level

	# Hero info (for career mode)
	if typeof(PlayerData) == TYPE_OBJECT and PlayerData:
		if PlayerData.has_method("get_hero_uid_raw"):
			engine_payload["hero_uid"] = PlayerData.get_hero_uid_raw()
		elif PlayerData.has_method("get_uid"):
			# Fallback: try to parse from string UID
			var uid_str = str(PlayerData.get_uid())
			if uid_str.begins_with("grad:"):
				engine_payload["hero_uid"] = int(uid_str.substr(5))
			elif uid_str.is_valid_int():
				engine_payload["hero_uid"] = int(uid_str)

	print("[OpenFootballAPI] Calling Rust engine with MatchSetup (seed: %d)" % engine_payload["seed"])

	if report_mrq0_json_diff:
		return _simulate_and_diff_mrq0_vs_json(engine_payload, match_setup)

	var rust_result: Dictionary
	if use_mrq0_v3_instructions:
		print("[OpenFootballAPI] Using MRQ0 v3 Binary hotpath")
		rust_result = rust_engine.simulate_match_binary(engine_payload)
	else:
		# Call engine (JSON default)
		rust_result = rust_engine.simulate_match_pure_binary(engine_payload)

	if typeof(rust_result) != TYPE_DICTIONARY or not rust_result.get("success", false):
		push_error("[OpenFootballAPI] Simulation failed")
		return _error_result("Simulation failed")

	# Parse result
	var parsed = _parse_match_result(rust_result)

	# CRITICAL: Attach match_setup to result (Risk 3 mitigation - Condition 4)
	# This ensures match_setup persists through the entire pipeline:
	# MatchManager → Engine → Result → Viewer → UI
	parsed["match_setup"] = match_setup

	print("[OpenFootballAPI] ✅ Match simulation complete with MatchSetup attached")

	return {"success": true, "response": parsed, "error": ""}


func _simulate_and_diff_mrq0_vs_json(engine_payload: Dictionary, match_setup: _MatchSetup) -> Dictionary:
	print("[OpenFootballAPI] Running Diff: MRQ0 v3 (Binary) vs JSON")

	# 1. Run JSON Simulation (Baseline)
	var json_start = Time.get_ticks_msec()
	var json_result = rust_engine.simulate_match_pure_binary(engine_payload)
	var json_time = Time.get_ticks_msec() - json_start

	if not json_result.get("success", false):
		return _error_result("Baseline JSON simulation failed: " + str(json_result.get("error")))

	# 2. Run MRQ0 Simulation (Candidate)
	var mrq0_start = Time.get_ticks_msec()
	var mrq0_result = rust_engine.simulate_match_binary(engine_payload)
	var mrq0_time = Time.get_ticks_msec() - mrq0_start

	if not mrq0_result.get("success", false):
		return _error_result("Candidate MRQ0 simulation failed: " + str(mrq0_result.get("error")))

	# 3. Compare Results
	var diff_report = _compare_simulation_results(json_result, mrq0_result)
	diff_report["timings"] = {"json_ms": json_time, "mrq0_ms": mrq0_time}

	# Use JSON result as the authoritative return value for the game,
	# but attach the report.
	var parsed = _parse_match_result(json_result)
	parsed["match_setup"] = match_setup
	parsed["path_diff_report"] = diff_report

	return {"success": true, "response": parsed, "error": ""}


func _compare_simulation_results(baseline: Dictionary, candidate: Dictionary) -> Dictionary:
	var diff = {"status": "ok", "diff": {}}

	# Compare metadata
	var b_meta = baseline.get("metadata", {})
	var c_meta = candidate.get("metadata", {})
	if b_meta.get("duration") != c_meta.get("duration"):
		diff.status = "fail"
		diff.diff["duration"] = {"baseline": b_meta.get("duration"), "mrq0": c_meta.get("duration")}

	# Compare position data (Sample first 10 frames of ball)
	var b_pos = baseline.get("position_data", {})
	var c_pos = candidate.get("position_data", {})

	var b_ball = b_pos.get("ball", [])
	var c_ball = c_pos.get("ball", [])

	if b_ball.size() != c_ball.size():
		diff.status = "fail"
		diff.diff["ball_frames_count"] = {"baseline": b_ball.size(), "mrq0": c_ball.size()}
	else:
		for i in range(min(b_ball.size(), 5)):  # Check first 5 frames
			var b = b_ball[i]
			var c = c_ball[i]
			if not _approx_eq_pos(b, c):
				diff.status = "fail"
				diff.diff["ball_frame_%d" % i] = {"baseline": b, "mrq0": c}
				break

	return diff


func _approx_eq_pos(a: Dictionary, b: Dictionary) -> bool:
	return is_equal_approx(a.x, b.x) and is_equal_approx(a.y, b.y) and is_equal_approx(a.z, b.z)


## Start interactive match with InteractiveMatchSetup (Game OS Phase E)
## Returns: InteractiveMatchController instance for pause/resume flow
## CRITICAL: This is the Phase E entry point for Interactive Mode (Bullet-Time)
func start_interactive_match_with_setup(
	interactive_setup: _InteractiveMatchSetup, rng_seed: int
) -> _InteractiveMatchController:
	print("[OpenFootballAPI] start_interactive_match_with_setup (Phase E - Interactive Mode)")

	if not _ensure_engine():
		push_error("[OpenFootballAPI] FootballRustEngine not available")
		return null

	if interactive_setup == null:
		push_error("[OpenFootballAPI] InteractiveMatchSetup is null")
		return null

	# Validate interactive setup
	var validation = interactive_setup.validate_interactive()
	if not validation.valid:
		push_error("[OpenFootballAPI] InteractiveMatchSetup validation failed: %s" % str(validation.errors))
		return null

	# Get PlayerLibrary (needed for UID resolution)
	var player_library = _PlayerLibrary.new()

	# Convert InteractiveMatchSetup → InteractiveMatchRequest
	var request = interactive_setup.to_interactive_request(rng_seed, player_library)

	if request.is_empty():
		push_error("[OpenFootballAPI] Failed to convert InteractiveMatchSetup to request")
		return null

	# Create InteractiveMatchController
	var controller = _InteractiveMatchController.new()

	# Start interactive match
	var success = controller.start_interactive_match(request)

	if not success:
		push_error("[OpenFootballAPI] Failed to start interactive match")
		return null

	print("[OpenFootballAPI] ✅ Interactive match started with InteractiveMatchSetup")
	return controller


## Fetch post-match statistics prepared by Rust engine
func get_match_statistics(match_id: String = "") -> Dictionary:
	if not _ensure_engine():
		return _error_result("FootballRustEngine not available")

	var raw_result = rust_engine.get_match_statistics(match_id)
	if not (raw_result is Dictionary):
		return _error_result("Invalid statistics payload from FootballRustEngine")

	var success = bool(raw_result.get("success", false))
	if not success:
		return {"success": false, "response": {}, "error": str(raw_result.get("error", "Unknown statistics error"))}

	var response_payload = raw_result.get("response", {})
	if response_payload is Dictionary:
		return {"success": true, "response": response_payload.duplicate(true), "error": ""}

	return {"success": false, "response": {}, "error": "Statistics payload missing or invalid"}


## ============================================================================
## DATA CONVERSION METHODS
## ============================================================================


## Resolve player identifier for training API (must be string)
func _resolve_player_id(player_data: Dictionary, rng_seed: int) -> String:
	var raw_id = player_data.get("uid", player_data.get("id", ""))
	var resolved = str(raw_id)

	if resolved == "" or resolved == "0" or resolved.to_lower() == "null":
		var base_name := str(player_data.get("name", "Player")).strip_edges()
		if base_name == "":
			base_name = "Player"
		resolved = "%s_%d" % [base_name.replace(" ", "_"), rng_seed]

	return resolved


func _resolve_condition_enum(condition_value: int) -> String:
	if condition_value >= 85:
		return "PerfectForm"
	elif condition_value >= 70:
		return "GoodForm"
	elif condition_value >= 40:
		return "Normal"
	elif condition_value >= 25:
		return "PoorForm"
	else:
		return "TerribleForm"


func _build_detailed_stats(player_data: Dictionary) -> Dictionary:
	var detailed_stats: Dictionary = {}
	if player_data.has("detailed_stats") and player_data.detailed_stats is Dictionary:
		for attr_key in player_data.detailed_stats.keys():
			var normalized := str(attr_key).to_lower()
			if normalized in OPENFOOTBALL_ATTRIBUTE_KEYS:
				detailed_stats[normalized] = int(round(float(player_data.detailed_stats[attr_key])))

	var ca_value := int(player_data.get("ca", 60))
	var default_value := clampi(ca_value, 10, 100)

	for attr_key in OPENFOOTBALL_ATTRIBUTE_KEYS:
		if not detailed_stats.has(attr_key):
			detailed_stats[attr_key] = default_value

	return detailed_stats


## ============================================================================
## DEV Preflight (진단 전용; 동작 변경 없음)
## ============================================================================


func _dev_preflight_check(request: Dictionary) -> void:
	print("[DEV][OpenFootballAPI] Preflight start")
	# 팀 인원 확인(정확히 18명)
	var ht: Dictionary = request.get("home_team", {})
	var at: Dictionary = request.get("away_team", {})
	var hsize := (ht.get("players", []) as Array).size()
	var asize := (at.get("players", []) as Array).size()
	if hsize != 18 or asize != 18:
		push_warning("[DEV][OpenFootballAPI] Team size not 18 (home=%d, away=%d)" % [hsize, asize])
	# 포메이션 유효성
	var hf := str(ht.get("formation", ""))
	var af := str(at.get("formation", ""))
	if not validate_formation(hf):
		push_warning("[DEV][OpenFootballAPI] Invalid home formation: %s" % hf)
	if not validate_formation(af):
		push_warning("[DEV][OpenFootballAPI] Invalid away formation: %s" % af)
	# 요청 크기(1MB 이하)
	var req_str: String = JSON.stringify(request)
	if req_str.length() > 1_000_000:
		push_warning("[DEV][OpenFootballAPI] Request too large: %d bytes (>1MB)" % req_str.length())
	# 엔진 메서드/폴링 존재
	if not rust_engine:
		push_warning("[DEV][OpenFootballAPI] rust_engine is null")
	else:
		if not rust_engine.has_method("start_simulation"):
			push_warning("[DEV][OpenFootballAPI] start_simulation() not found (async path)")
		if not rust_engine.has_method("poll_simulation"):
			push_warning("[DEV][OpenFootballAPI] poll_simulation() not found (ensure polling elsewhere)")
	print("[DEV][OpenFootballAPI] Preflight end")


## Public conversion entrypoint (exposed for PlayerData.gd)
func convert_player_to_core_format(player_data: Dictionary) -> Dictionary:
	if player_data.is_empty():
		push_warning("[OpenFootballAPI] convert_player_to_core_format called with empty data")
		return {}

	var normalized_source := player_data.duplicate(true)
	normalized_source["condition"] = _resolve_condition_score(player_data)
	normalized_source["stamina"] = _resolve_stamina_score(player_data)

	# Ensure legacy PlayerData keys align with _build_player_json expectations
	if not normalized_source.has("technical") and player_data.has("technical_stats"):
		normalized_source["technical"] = player_data.technical_stats
	if not normalized_source.has("mental") and player_data.has("mental_stats"):
		normalized_source["mental"] = player_data.mental_stats
	if not normalized_source.has("physical") and player_data.has("physical_stats"):
		normalized_source["physical"] = player_data.physical_stats
	if not normalized_source.has("goalkeeper") and player_data.has("goalkeeper_stats"):
		normalized_source["goalkeeper"] = player_data.goalkeeper_stats

	if not normalized_source.has("overall") and player_data.has("overall_rating"):
		normalized_source["overall"] = int(player_data.overall_rating)

	return _build_player_json(normalized_source)


func _resolve_condition_score(player_data: Dictionary) -> int:
	var raw_condition = player_data.get("condition", 50)
	if raw_condition is String:
		match raw_condition:
			"Excellent":
				return 100
			"Good":
				return 80
			"Normal":
				return 60
			"Poor":
				return 40
			"Terrible":
				return 20
	var cond_value = int(raw_condition)
	if cond_value <= 5:
		return clampi((cond_value - 1) * 25, 0, 100)
	return clampi(cond_value, 0, 100)


func _resolve_stamina_score(player_data: Dictionary) -> int:
	if player_data.has("stamina"):
		return clampi(int(player_data.get("stamina", 100)), 0, 100)
	if player_data.has("fatigue"):
		var fatigue_value = float(player_data.get("fatigue", 0.0))
		var stamina_from_fatigue = int(round(100.0 - clampf(fatigue_value, 0.0, 100.0)))
		return clampi(stamina_from_fatigue, 0, 100)
	# fallback: reuse condition score
	return _resolve_condition_score(player_data)


## Convert Godot PlayerData to CorePlayer payload dictionary (internal)
func _build_player_json(player_data: Dictionary) -> Dictionary:
	var player_id := _resolve_player_id(player_data, 0)
	var age_months := float(player_data.get("age_months", player_data.get("age", 16) * 12))
	var detailed_stats := _build_detailed_stats(player_data)

	var core_player = {
		"id": player_id,
		"name": player_data.get("name", "Unknown"),
		"ca": int(player_data.get("ca", 50)),
		"pa": int(player_data.get("pa", 70)),
		"position": _convert_position(player_data.get("position", "M")),
		"condition": int(player_data.get("condition", 50)),
		"stamina": int(player_data.get("stamina", 100)),
		"form": int(player_data.get("form", 50)),
		"morale": int(player_data.get("morale", 50)),
		"attributes": _build_attributes(player_data),
		"overall": int(player_data.get("overall", player_data.get("ca", 50))),
		"age_months": age_months,
		"detailed_stats": detailed_stats
	}

	return _normalize_core_player(core_player)


## Build detailed attributes from player data
func _build_attributes(player_data: Dictionary) -> Dictionary:
	# If player has detailed attributes, use them
	if player_data.has("attributes"):
		return _coerce_attribute_map(player_data.attributes)

	return _build_attribute_map_from_stats(player_data)


func _build_attribute_map_from_stats(player_data: Dictionary) -> Dictionary:
	# Handle both nested dictionaries and flat integer attributes
	var raw_technical = player_data.get("technical", player_data.get("technical_stats", {}))
	var raw_mental = player_data.get("mental", player_data.get("mental_stats", {}))
	var raw_physical = player_data.get("physical", player_data.get("physical_stats", {}))
	var raw_goalkeeper = player_data.get("goalkeeper", player_data.get("goalkeeper_stats", {}))

	# If attributes are flat integers (e.g., "technical": 60), convert to empty dict and use the int as base
	var technical: Dictionary = raw_technical if typeof(raw_technical) == TYPE_DICTIONARY else {}
	var mental: Dictionary = raw_mental if typeof(raw_mental) == TYPE_DICTIONARY else {}
	var physical: Dictionary = raw_physical if typeof(raw_physical) == TYPE_DICTIONARY else {}
	var goalkeeper: Dictionary = raw_goalkeeper if typeof(raw_goalkeeper) == TYPE_DICTIONARY else {}

	# Base values from flat attributes (if present)
	var base_technical: int = int(raw_technical) if typeof(raw_technical) == TYPE_INT else 15
	var base_mental: int = int(raw_mental) if typeof(raw_mental) == TYPE_INT else 15
	var base_physical: int = int(raw_physical) if typeof(raw_physical) == TYPE_INT else 15

	var attributes: Dictionary = {}

	# Physical - use base_physical as fallback when no nested dict
	attributes["pace"] = physical.get("pace", base_physical)
	attributes["acceleration"] = physical.get("acceleration", base_physical)
	attributes["stamina"] = physical.get("stamina", base_physical)
	attributes["strength"] = physical.get("strength", base_physical)
	attributes["agility"] = physical.get("agility", base_physical)
	attributes["balance"] = physical.get("balance", base_physical)
	attributes["jumping"] = physical.get("jumping", base_physical)
	attributes["natural_fitness"] = physical.get("natural_fitness", base_physical)

	# Technical - use base_technical as fallback when no nested dict
	attributes["corners"] = technical.get("corners", base_technical)
	attributes["crossing"] = technical.get("crossing", base_technical)
	attributes["dribbling"] = technical.get("dribbling", base_technical)
	attributes["finishing"] = technical.get("finishing", base_technical)
	attributes["first_touch"] = technical.get("first_touch", base_technical)
	attributes["free_kick_taking"] = technical.get("free_kicks", base_technical)
	attributes["heading"] = technical.get("heading", base_technical)
	attributes["long_shots"] = technical.get("long_shots", base_technical)
	attributes["long_throws"] = technical.get("long_throws", base_technical)
	attributes["marking"] = technical.get("marking", base_technical)
	attributes["passing"] = technical.get("passing", base_technical)
	attributes["penalty_taking"] = technical.get("penalty_kicks", base_technical)
	attributes["tackling"] = technical.get("tackling", base_technical)
	attributes["technique"] = technical.get("technique", base_technical)

	# Mental - use base_mental as fallback when no nested dict
	attributes["aggression"] = mental.get("aggression", base_mental)
	attributes["anticipation"] = mental.get("anticipation", base_mental)
	attributes["bravery"] = mental.get("bravery", base_mental)
	attributes["composure"] = mental.get("composure", base_mental)
	attributes["concentration"] = mental.get("concentration", base_mental)
	attributes["decisions"] = mental.get("decisions", base_mental)
	attributes["determination"] = mental.get("determination", base_mental)
	attributes["flair"] = mental.get("flair", base_mental)
	attributes["leadership"] = mental.get("leadership", base_mental)
	attributes["off_the_ball"] = mental.get("off_the_ball", base_mental)
	attributes["positioning"] = mental.get("positioning", base_mental)
	attributes["teamwork"] = mental.get("teamwork", base_mental)
	attributes["vision"] = mental.get("vision", base_mental)
	attributes["work_rate"] = mental.get("work_rate", base_mental)

	# Goalkeeper stats (map to existing fields where relevant)
	if not technical.has("handling"):
		attributes["handling"] = goalkeeper.get("handling", 15)
	else:
		attributes["handling"] = technical.get("handling", 15)
	attributes["aerial_reach"] = goalkeeper.get("aerial_reach", 15)
	attributes["command_of_area"] = goalkeeper.get("command_of_area", 15)
	attributes["communication"] = goalkeeper.get("communication", 15)
	attributes["eccentricity"] = goalkeeper.get("eccentricity", 15)
	attributes["kicking"] = goalkeeper.get("kicking", 15)

	return _coerce_attribute_map(attributes)


## Build training manager state payload
func _build_manager_json(player_data: Dictionary) -> Dictionary:
	var stamina_value := clampi(int(player_data.get("stamina", 100)), 0, 100)
	var condition_value := clampi(int(player_data.get("condition", 50)), 0, 100)

	return {
		"stamina_system": {"current": stamina_value, "maximum": 100, "recovery_rate": 30},
		"condition": _resolve_condition_enum(condition_value),
		"consecutive_training_days": int(player_data.get("consecutive_training_days", 0)),
		"consecutive_rest_days": int(player_data.get("consecutive_rest_days", 0)),
		"week_training_count": int(player_data.get("week_training_count", 0)),
		"training_history": [],
		"active_deck": null
	}


## Build team JSON for match simulation
func _prepare_team_payload(team_data: Dictionary) -> Dictionary:
	var payload := team_data.duplicate(true)
	if payload.has("players") and payload["players"] is Array:
		var cloned_players: Array = []
		for player_data in payload["players"]:
			if player_data is Dictionary:
				cloned_players.append((player_data as Dictionary).duplicate(true))
			else:
				cloned_players.append(player_data)
		payload["players"] = cloned_players
	else:
		payload["players"] = []

	if not payload.has("formation"):
		payload["formation"] = "4-4-2"

	return payload


func _extract_tactical_instructions(team_data: Dictionary) -> Dictionary:
	if not team_data.has("tactical_instructions"):
		return {}
	var payload = team_data.get("tactical_instructions", {})
	if not (payload is Dictionary):
		return {}
	var required := ["defensive_line", "pressing_intensity", "team_tempo", "team_width", "build_up_style"]
	for key in required:
		if not payload.has(key):
			return {}
	return (payload as Dictionary).duplicate(true)


## Normalize team payload to match the minimal schema expected by of_core JSON API
## Ensures: name:String, formation:String, players:Array[Dictionary{name, position, overall:u8}]
func _normalize_team_for_match(team_data: Dictionary) -> Dictionary:
	var team := _prepare_team_payload(team_data)
	team["name"] = str(team.get("name", "Academy Team"))
	team["formation"] = sanitize_formation(str(team.get("formation", "4-4-2")))
	var players: Array = []
	if team.has("players") and team["players"] is Array:
		for p in team["players"]:
			if typeof(p) == TYPE_DICTIONARY:
				var pd: Dictionary = p
				var player_name := str(pd.get("name", pd.get("player_id", pd.get("id", "Player"))))
				var pos := _convert_position(str(pd.get("position", POSITION_DEFAULT)))
				var ov_raw = pd.get("overall", pd.get("ca", 50))
				var ov := int(round(float(ov_raw)))
				ov = clampi(ov, 1, 200)
				var minimal := {
					"name": player_name,
					"position": pos,
					"overall": ov,
				}
				# Preserve useful identifiers
				if pd.has("id"):
					minimal["id"] = pd.get("id")
				elif pd.has("player_id"):
					minimal["id"] = pd.get("player_id")
				# Ensure CA/PA are present (engine expects these for some paths)
				# Fallback to overall when CA is missing; PA falls back to CA
				var ca_val: int = int(pd.get("ca", ov))
				var pa_val: int = int(pd.get("pa", ca_val))
				minimal["ca"] = ca_val
				minimal["pa"] = pa_val

				# Enrich with default runtime context for better simulation quality
				if not minimal.has("condition"):
					var cond_raw = pd.get("condition", 50)
					var cond_f := 0.5
					match typeof(cond_raw):
						TYPE_FLOAT, TYPE_INT:
							var v = float(cond_raw)
							cond_f = v if v <= 1.0 else v / 100.0
						_:
							cond_f = 0.5
					minimal["condition"] = clamp(cond_f, 0.0, 1.0)
				if not minimal.has("stamina"):
					minimal["stamina"] = int(pd.get("stamina", 100))
				if not minimal.has("form"):
					minimal["form"] = int(pd.get("form", 50))
				if not minimal.has("morale"):
					minimal["morale"] = int(pd.get("morale", 50))

				# Age in months (fallback from age years)
				if not minimal.has("age_months"):
					var age_months_val: int = 0
					if pd.has("age_months"):
						age_months_val = int(pd.get("age_months"))
					elif pd.has("age"):
						age_months_val = int(pd.get("age", 16)) * 12
					else:
						age_months_val = 16 * 12
					minimal["age_months"] = age_months_val

				# Always build a normalized attribute block using shared helper.
				# This guarantees that attributes include both OpenFootball keys
				# (e.g. penalty_taking) and Godot aliases (penalty_kicks) where needed.
				minimal["attributes"] = _build_attributes(pd)

				players.append(minimal)
	team["players"] = players
	return team


## Ensure exactly 18 players; if >18, truncate; if <18, pad with simple bench clones
func _ensure_exact_18_players(players: Array) -> Array:
	var out: Array = players.duplicate(true)
	# If empty, fabricate 18 placeholder players
	if out.size() == 0:
		for i in range(18):
			(
				out
				. append(
					{
						"name": "Player %02d" % (i + 1),
						"position": POSITION_DEFAULT,
						"overall": 50,
					}
				)
			)
		return out

	if out.size() > 18:
		return out.slice(0, 18)

	var idx: int = 0
	while out.size() < 18:
		var base: Dictionary = out[idx % out.size()] as Dictionary
		var clone: Dictionary = base.duplicate(true) as Dictionary
		clone["name"] = str(base.get("name", "Bench")) + " (Bench %02d)" % (out.size() + 1)
		# Slightly reduce overall for bench clones to avoid overpowering
		clone["overall"] = clampi(int(clone.get("overall", 50)) - 5, 1, 200)
		out.append(clone)
		idx += 1
	return out


## Ensure roster satisfies minimal formation requirements
func _ensure_formation_minimums(team: Dictionary, formation: String) -> Dictionary:
	var code := str(formation)
	var players: Array = team.get("players", [])
	if players.is_empty():
		return team

	# Parse formation generically: D-M-...-F (sum middle parts as MF)
	var parts: Array = code.split("-", false)
	var req_df := 4
	var req_fw := 2
	var req_mf := 4
	if parts.size() >= 2:
		var d := int(parts[0]) if String(parts[0]).is_valid_int() else 4
		var f := int(parts[parts.size() - 1]) if String(parts[parts.size() - 1]).is_valid_int() else 2
		var m: int = 0
		for i in range(1, parts.size() - 1):
			if String(parts[i]).is_valid_int():
				m += int(parts[i])
		req_df = max(1, d)
		req_fw = max(1, f)
		req_mf = max(1, m)

	# Count categories
	var cnt_gk := 0
	var cnt_df := 0
	var cnt_mf := 0
	var cnt_fw := 0
	for p in players:
		if typeof(p) != TYPE_DICTIONARY:
			continue
		var pos := str(p.get("position", POSITION_DEFAULT)).to_upper()
		var canon := _canonical_position(pos)
		match canon:
			"GK":
				cnt_gk += 1
			"ST", "CF", "FW":
				cnt_fw += 1
			"LB", "RB", "CB", "LWB", "RWB", "DF":
				cnt_df += 1
			"CDM", "CM", "CAM", "LM", "RM", "MF", "LW", "RW":
				cnt_mf += 1
			_:
				cnt_mf += 1

	# Ensure at least 2 GKs for 18-man squad (engine requirement)
	var need_gk: int = max(0, 2 - cnt_gk)
	if need_gk > 0 and players.size() > 0:
		var j: int = players.size() - 1
		while need_gk > 0 and j >= 0:
			var pdk: Dictionary = players[j] as Dictionary
			var ckg: String = _canonical_position(str(pdk.get("position", POSITION_DEFAULT)).to_upper())
			if ckg != "GK":
				pdk["position"] = "GK"
				need_gk -= 1
		j -= 1

	# Compute deficits
	var need_fw: int = max(0, req_fw - cnt_fw)
	var need_df: int = max(0, req_df - cnt_df)
	var need_mf: int = max(0, req_mf - cnt_mf)

	# Assign helpers from back of the list to minimize impact on starters
	var i: int = players.size() - 1
	while need_fw > 0 and i >= 0:
		var pd: Dictionary = players[i] as Dictionary
		var canon2: String = _canonical_position(str(pd.get("position", POSITION_DEFAULT)).to_upper())
		if canon2 != "GK" and not (canon2 in ["ST", "CF", "FW"]):
			pd["position"] = "ST"
			need_fw -= 1
		i -= 1

	# Re-scan after FW assignment
	cnt_df = 0
	cnt_mf = 0
	for p in players:
		if typeof(p) != TYPE_DICTIONARY:
			continue
		var c: String = _canonical_position(str((p as Dictionary).get("position", POSITION_DEFAULT)).to_upper())
		if c in ["LB", "RB", "CB", "LWB", "RWB", "DF"]:
			cnt_df += 1
		elif c in ["CDM", "CM", "CAM", "LM", "RM", "MF", "LW", "RW"]:
			cnt_mf += 1

	need_df = max(0, req_df - cnt_df)
	need_mf = max(0, req_mf - cnt_mf)

	# Fill defenders with CB
	i = players.size() - 1
	while need_df > 0 and i >= 0:
		var pd2: Dictionary = players[i] as Dictionary
		var c2: String = _canonical_position(str(pd2.get("position", POSITION_DEFAULT)).to_upper())
		if c2 != "GK" and not (c2 in ["LB", "RB", "CB", "LWB", "RWB", "DF"]):
			pd2["position"] = "CB"
			need_df -= 1
		i -= 1

	# Re-scan MF count and fill with CM
	cnt_mf = 0
	for p in players:
		if typeof(p) != TYPE_DICTIONARY:
			continue
		var c3: String = _canonical_position(str((p as Dictionary).get("position", POSITION_DEFAULT)).to_upper())
		if c3 in ["CDM", "CM", "CAM", "LM", "RM", "MF", "LW", "RW"]:
			cnt_mf += 1
	need_mf = max(0, req_mf - cnt_mf)

	i = players.size() - 1
	while need_mf > 0 and i >= 0:
		var pd3: Dictionary = players[i] as Dictionary
		var c4: String = _canonical_position(str(pd3.get("position", POSITION_DEFAULT)).to_upper())
		if c4 != "GK" and not (c4 in ["CDM", "CM", "CAM", "LM", "RM", "MF", "LW", "RW"]):
			pd3["position"] = "CM"
			need_mf -= 1
		i -= 1

	# Optional debug
	if OS.has_environment("DEV_VALIDATE"):
		var dbg_fw := 0
		var dbg_df := 0
		var dbg_mf := 0
		var dbg_gk := 0
		for p in players:
			if typeof(p) != TYPE_DICTIONARY:
				continue
			var cc := _canonical_position(str((p as Dictionary).get("position", POSITION_DEFAULT)).to_upper())
			match cc:
				"GK":
					dbg_gk += 1
				"ST":
					dbg_fw += 1
				"LB", "RB", "CB", "LWB", "RWB", "DF":
					dbg_df += 1
				_:
					dbg_mf += 1
		print("[OpenFootballAPI] VALIDATE %s => GK:%d DF:%d MF:%d FW:%d" % [code, dbg_gk, dbg_df, dbg_mf, dbg_fw])

	team["players"] = players
	return team


## Map a position string to a canonical code we use for formation checks
func _canonical_position(pos: String) -> String:
	var p := _convert_position(pos)
	# Map broad categories
	if p in ["ST", "CF", "FW"]:
		return "ST"
	if p in ["LB", "RB", "CB", "LWB", "RWB", "DF"]:
		return p
	if p in ["CDM", "CM", "CAM", "LM", "RM", "MF", "LW", "RW"]:
		return p
	return p


func _build_team_json(team_data: Dictionary) -> Dictionary:
	var payload := _prepare_team_payload(team_data)
	payload["formation"] = sanitize_formation(payload.get("formation", "4-4-2"))

	var players: Array = []
	if payload.has("players") and payload["players"] is Array:
		players = payload["players"]

	print(
		(
			"[OpenFootballAPI] ⚙️ _build_team_json delegating (%s players=%d)"
			% [payload.get("name", "Unnamed Team"), players.size()]
		)
	)

	var overall := int(payload.get("overall", 0))
	if overall <= 0:
		var total_ca := 0
		var counted_players := 0
		for player in players:
			if player is Dictionary:
				var player_dict: Dictionary = player
				total_ca += int(player_dict.get("ca", player_dict.get("overall", 0)))
				counted_players += 1
		if counted_players > 0:
			overall = int(round(float(total_ca) / float(counted_players)))
		elif payload.has("overall_rating") or payload.has("rating"):
			overall = int(payload.get("overall_rating", payload.get("rating", 0)))
	payload["overall"] = overall

	if not payload.has("overall_rating"):
		payload["overall_rating"] = int(payload.get("overall", 0))

	return payload


## ============================================================================
## RUNTIME SETTINGS HELPERS
## ============================================================================


func _resolve_budget_ms(hl_alias: String) -> int:
	var defaults := {"myplayer": 900, "simple": 800, "other": 500}
	var kind := hl_alias
	if kind != "myplayer" and kind != "simple":
		kind = "other"

	var env_key := ""
	match kind:
		"myplayer":
			env_key = "FOOTBALL_BUDGET_MYPLAYER_MS"
		"simple":
			env_key = "FOOTBALL_BUDGET_SIMPLE_MS"
		_:
			env_key = "FOOTBALL_BUDGET_OTHER_MS"

	if OS.has_environment(env_key):
		var vstr := String(OS.get_environment(env_key)).strip_edges()
		if vstr.is_valid_int():
			var vi := int(vstr)
			if vi > 0:
				return vi

	var ps_key := ""
	match kind:
		"myplayer":
			ps_key = "football/sim/budget_ms_myplayer"
		"simple":
			ps_key = "football/sim/budget_ms_simple"
		_:
			ps_key = "football/sim/budget_ms_other"

	if ProjectSettings.has_setting(ps_key):
		var psv = int(ProjectSettings.get_setting(ps_key))
		if psv > 0:
			return psv

	return int(defaults[kind])


func _should_include_stored_events() -> bool:
	var ps_override = _read_settings_bool(SETTINGS_FORCE_STORED_EVENTS)
	if ps_override != null:
		return ps_override
	var env_override = _read_env_bool(ENV_FORCE_STORED_EVENTS)
	if env_override != null:
		return env_override
	# Spec-kit 요구대로 기본값을 항상 true로 유지해 풍부한 stored_events를 수집한다.
	return true


func _should_log_match_payloads() -> bool:
	var ps_flag = _read_settings_bool(SETTINGS_VERBOSE_LOG)
	if ps_flag != null:
		return ps_flag
	var env_flag = _read_env_bool(ENV_VERBOSE_LOG)
	if env_flag != null:
		return env_flag
	return false


func _are_vendor_features_enabled() -> bool:
	var ps_flag = _read_settings_bool(SETTINGS_VENDOR_FLAG)
	if ps_flag != null:
		return ps_flag
	var env_flag = _read_env_bool(ENV_VENDOR_FLAG)
	if env_flag != null:
		return env_flag
	return false


func _read_env_bool(env_key: String) -> Variant:
	if OS.has_environment(env_key):
		return _coerce_bool(OS.get_environment(env_key))
	return null


func _read_settings_bool(setting_key: String) -> Variant:
	if ProjectSettings.has_setting(setting_key):
		return _coerce_bool(ProjectSettings.get_setting(setting_key))
	return null


func _coerce_bool(value: Variant) -> bool:
	match typeof(value):
		TYPE_BOOL:
			return value
		TYPE_INT, TYPE_FLOAT:
			return int(value) != 0
		TYPE_STRING:
			var normalized := String(value).strip_edges().to_lower()
			if normalized in ["1", "true", "yes", "on", "enable"]:
				return true
			if normalized in ["0", "false", "no", "off", "disable"]:
				return false
	return false


## Substitute zero/empty player_id in events using details.actor_id or myplayer id
## Returns a dictionary with counts: {"fixed": int, "remaining": int}
func _substitute_zero_ids_in_events(events: Array, user_player_id: String, key_kinds: Dictionary) -> Dictionary:
	var fixed_zero_ids := 0
	var remaining_zero_ids := 0
	for i in range(events.size()):
		var ev = events[i]
		if typeof(ev) != TYPE_DICTIONARY:
			continue
		var ed: Dictionary = ev
		var base_variant: Variant = ed.get("base", {})
		if base_variant is Dictionary:
			var base: Dictionary = base_variant
			var pid = base.get("player_id", null)
			var pid_is_zero := false
			match typeof(pid):
				TYPE_NIL:
					pid_is_zero = true
				TYPE_INT, TYPE_FLOAT:
					pid_is_zero = (int(pid) == 0)
				TYPE_STRING:
					var ps := String(pid)
					pid_is_zero = (ps == "" or ps == "0" or ps.to_lower() == "null")
				_:
					pid_is_zero = false
			if pid_is_zero:
				var kind_str := str(ed.get("kind", ed.get("etype", ed.get("type", "")))).to_lower()
				var substituted := false
				# 1) Prefer actor_id from details if present
				if ed.has("details") and ed.details is Dictionary:
					var det: Dictionary = ed.details
					for det_key in ["actor_id", "player_id", "uid", "id", "name"]:
						if det.has(det_key):
							var dv = det.get(det_key)
							var ds := str(dv)
							if ds != "" and ds != "0" and ds.to_lower() != "null":
								base["player_id"] = ds
								ed["base"] = base
								events[i] = ed
								substituted = true
								break
				# 2) Otherwise, if kind is key-like, substitute with myplayer id
				if not substituted and key_kinds.has(kind_str):
					base["player_id"] = user_player_id
					ed["base"] = base
					events[i] = ed
					substituted = true
				if substituted:
					fixed_zero_ids += 1
				else:
					remaining_zero_ids += 1
	return {"fixed": fixed_zero_ids, "remaining": remaining_zero_ids}


## ============================================================================
## RESPONSE PARSING METHODS
## ============================================================================


## Parse training result response
func _parse_training_result(response: Dictionary) -> Dictionary:
	var raw_improvements = response.get("improved_attributes", [])
	var improvements: Array = []
	var improvement_map: Dictionary = {}

	if raw_improvements is Array:
		for entry in raw_improvements:
			if entry is Dictionary:
				var attr_name := str(entry.get("attribute", ""))
				var growth_value := float(entry.get("growth", 0.0))
				improvements.append({"attribute": attr_name, "growth": growth_value})
				if attr_name != "":
					improvement_map[attr_name] = growth_value
	elif raw_improvements is Dictionary:
		for attr_name in raw_improvements.keys():
			var growth_value := float(raw_improvements[attr_name])
			var normalized_name := str(attr_name)
			improvements.append({"attribute": normalized_name, "growth": growth_value})
			improvement_map[normalized_name] = growth_value

	return {
		"ca_before": response.get("ca_before", 0),
		"ca_after": response.get("ca_after", 0),
		"ca_growth": response.get("ca_after", 0) - response.get("ca_before", 0),
		"stamina_before": response.get("stamina_before", 0),
		"stamina_after": response.get("stamina_after", 0),
		"stamina_change": response.get("stamina_after", 0) - response.get("stamina_before", 0),
		"condition": response.get("condition", 0),
		"improved_attributes": improvements,
		"attribute_changes": improvement_map,
		"injury_occurred": response.get("injury_occurred", false),
		"message": response.get("message", "")
	}


## Parse rest result response
func _parse_rest_result(response: Dictionary) -> Dictionary:
	return {
		"stamina_before": response.get("stamina_before", 0),
		"stamina_after": response.get("stamina_after", 0),
		"stamina_change": response.get("stamina_after", 0) - response.get("stamina_before", 0),
		"condition": response.get("condition", 0),
		"was_forced": response.get("was_forced", false)
	}


## Parse status result response
func _parse_status_result(response: Dictionary) -> Dictionary:
	return {
		"ca": response.get("ca", 0),
		"pa": response.get("pa", 0),
		"stamina": response.get("stamina", 0),
		"condition": response.get("condition", 0),
		"consecutive_training_days": response.get("consecutive_training_days", 0),
		"injury_risk": response.get("injury_risk", 0.0)
	}


## Parse match result response
func _parse_match_result(response: Dictionary) -> Dictionary:
	# Guard against unexpected payload types (e.g. legacy paths returning an
	# Array). Without this, calls to Dictionary-only methods such as `keys()`
	# will trigger runtime errors and break match flow.
	if not (response is Dictionary):
		push_warning("[OpenFootballAPI] _parse_match_result expected Dictionary but got %s" % typeof(response))
		return {
			"score_home": 0,
			"score_away": 0,
			"events": [],
			"rosters": {},
			"stats": {},
			"player_ratings": {},
			"goals": [],
			"assists": [],
			"yellow_cards": [],
			"red_cards": [],
			"timeline": [],
			"stored_events": [],
			"position_data": {}
		}

	# ========================================================================
	# Phase A: Extract events from correct location (timeline_doc.events)
	# ========================================================================
	# The Rust engine returns events nested in the timeline doc (legacy key also supported),
	# not directly in response["events"].
	var events: Array = []
	var rosters: Dictionary = {}  # ✅ Phase A+: Extract rosters data
	var timeline: Array = []
	var stored_events: Array = []
	var timeline_doc_local: Variant = null

	var legacy_doc_key := "re" + "play"
	if response.has("timeline_doc") or response.has(legacy_doc_key):
		var doc_variant: Variant = response.get("timeline_doc", response.get(legacy_doc_key, {}))
		if doc_variant is String:
			var parsed_doc = JSON.parse_string(doc_variant)
			if parsed_doc is Dictionary:
				doc_variant = parsed_doc
		var doc: Dictionary = doc_variant if doc_variant is Dictionary else {}
		if not doc.is_empty():
			timeline_doc_local = doc.duplicate(true)

		# Extract events (Phase A)
		if doc.has("events"):
			var raw_events = doc.get("events", [])
			if raw_events is Array:
				# ✅ Convert new format (etype, team, etc.) to kind/base format
				var converted_events: Array = []
				for raw_ev in raw_events:
					if typeof(raw_ev) != TYPE_DICTIONARY:
						continue
					var ed: Dictionary = raw_ev

					# Extract fields from new format
					var kind_str: String = str(ed.get("etype", ed.get("kind", "unknown"))).to_lower()
					var minute_val: float = float(ed.get("minute", 0.0))
					var t_val: float = float(ed.get("t", minute_val * 60.0))
					var team_str: String = str(ed.get("team", "HOME"))
					var team_id: int = 0 if team_str == "HOME" else 1
					# ✅ Fixed: Rust uses "player" key, not "player_id" - check both for compatibility
					var player_name: String = ""
					if ed.has("player") and typeof(ed["player"]) == TYPE_STRING:
						player_name = ed["player"]
					elif ed.has("player_id"):
						player_name = str(ed["player_id"])

					# Build base block
					var base_block := {"t": t_val, "minute": minute_val, "team_id": team_id, "player_id": player_name}

					# Add pos if available
					if ed.has("pos") and ed.pos is Dictionary:
						base_block["pos"] = ed.pos

					# Build converted event
					var out_event := {"kind": kind_str, "base": base_block}

					# Copy additional fields (from, to, target, receiver_id, etc.)
					for field in [
						"from", "to", "target", "receiver_id", "outcome", "xg", "on_target", "ball", "end_pos", "ground"
					]:
						if ed.has(field):
							out_event[field] = ed[field]

					converted_events.append(out_event)

				events = converted_events
				print("[OpenFootballAPI] ✅ Extracted %d events from timeline_doc.events" % events.size())

				# ✅ DEBUG: Print first 3 events to inspect structure
				print("\n=== 🔍 Event Data Structure Debug ===")
				for i in range(min(3, events.size())):
					print("Event %d: %s" % [i, JSON.stringify(events[i], "  ")])
				print("=== End Event Debug ===\n")
			else:
				print("[OpenFootballAPI] ⚠️ WARNING: timeline_doc.events is not an Array: %s" % typeof(raw_events))
		else:
			print("[OpenFootballAPI] ⚠️ WARNING: timeline_doc has no 'events' key")

		# ✅ Phase A+: Extract rosters (NEW - fixes player name display)
		if doc.has("rosters"):
			var raw_rosters = doc.get("rosters", {})
			if raw_rosters is Dictionary:
				rosters = raw_rosters
				var home_data = rosters.get("home", {})
				var away_data = rosters.get("away", {})
				var home_count = 0
				var away_count = 0
				if home_data is Dictionary:
					var home_players = home_data.get("players", [])
					if home_players is Array:
						home_count = home_players.size()
				if away_data is Dictionary:
					var away_players = away_data.get("players", [])
					if away_players is Array:
						away_count = away_players.size()
				print(
					(
						"[OpenFootballAPI] ✅ Extracted rosters from timeline_doc.rosters (home: %d, away: %d)"
						% [home_count, away_count]
					)
				)

				# ✅ DEBUG: Print roster structure
				print("\n=== 🔍 Roster Data Structure Debug ===")
				if rosters.has("home"):
					var home_roster = rosters.get("home", {})
					if home_roster is Dictionary:
						print("Home roster keys: %s" % str(home_roster.keys()))
						if home_roster.has("players") and home_roster.players is Array:
							var players = home_roster.players
							if players.size() > 0:
								print("Home player[0] sample: %s" % JSON.stringify(players[0], "  "))
					elif home_roster is Array:
						print("Home roster is Array with %d players" % home_roster.size())
				if rosters.has("away"):
					var away_roster = rosters.get("away", {})
					if away_roster is Dictionary:
						print("Away roster keys: %s" % str(away_roster.keys()))
						if away_roster.has("players") and away_roster.players is Array:
							var players = away_roster.players
							if players.size() > 0:
								print("Away player[0] sample: %s" % JSON.stringify(players[0], "  "))
					elif away_roster is Array:
						print("Away roster is Array with %d players" % away_roster.size())
				print("=== End Roster Debug ===\n")
			else:
				print(
					"[OpenFootballAPI] ⚠️ WARNING: timeline_doc.rosters is not a Dictionary: %s" % typeof(raw_rosters)
				)
		else:
			print("[OpenFootballAPI] ⚠️ WARNING: timeline_doc has no 'rosters' key")
			# Extract timeline data if present
		# Extract timeline data if present
		if doc.has("timeline"):
			var raw_timeline: Variant = doc.get("timeline", [])
			if raw_timeline is Array:
				timeline = raw_timeline.duplicate(true)
			else:
				print(
					(
						"[OpenFootballAPI] ⚠️ WARNING: timeline payload 'timeline' is not an Array: %s"
						% typeof(raw_timeline)
					)
				)
		else:
			print("[OpenFootballAPI] ⚠️ WARNING: timeline payload missing 'timeline' key")
	else:
		print("[OpenFootballAPI] ⚠️ WARNING: response missing legacy timeline payload key")
		# Fallback: use top-level 'events' if provided by engine JSON API, and synthesize a minimal timeline document
		if response.has("events") and response["events"] is Array:
			var raw_events2: Array = response["events"]
			events = raw_events2.duplicate(true)
			for i in range(events.size()):
				if typeof(events[i]) == TYPE_DICTIONARY:
					_sanitize_timeline_event(events[i])
			print("[OpenFootballAPI] ✅ Fallback: using %d events from response.events" % events.size())
			# Synthesize minimal timeline document to aid 3D overlay (adds 'kind' + base timing)
			var synthesized_events: Array = []
			for e in events:
				if typeof(e) != TYPE_DICTIONARY:
					continue
				var ed: Dictionary = e
				var kind_str: String = str(ed.get("etype", ed.get("type", ""))).to_lower()
				var minute_val: int = int(ed.get("minute", 0))
				var is_home: bool = bool(ed.get("is_home_team", true))
				var base_block := {
					"t": float(minute_val),  # minutes, not seconds, for EventAnimator3D
					"minute": minute_val,
					"team_id": 0 if is_home else 1,
					"player_id": int(ed.get("player_id", 0))
				}
				var out_event := {"kind": kind_str if kind_str != "" else "generic", "base": base_block}
				# Preserve any extra details for UI/debug
				if ed.has("details"):
					out_event["details"] = ed["details"]
				synthesized_events.append(out_event)
			# Replace events with synthesized ones for downstream consumers
			events = synthesized_events
			# Also attach a minimal timeline_doc so consumers can use a consistent shape
			rosters = (
				response.get("rosters", {}) if response.has("rosters") and response["rosters"] is Dictionary else {}
			)
			var synth_timeline_doc := {
				"version": 1,
				"pitch_m": {"width_m": 105.0, "height_m": 68.0},
				"events": events.duplicate(true),
				"rosters": (rosters as Dictionary).duplicate(true),
				"metadata": {"seed": int(response.get("seed", 0))}
			}
			timeline_doc_local = synth_timeline_doc
			# Expose synthesized doc to callers.
			# Note: The top-level return below adds events and leaves the doc in parsed_result
			# so consumers like the 3D overlay can read it.
			# Assign to local to carry out
			# (we will include in returned dictionary below via parsed_result["timeline_doc"]).
			# Store in a temporary variable accessible after return construction by the caller context.

	var stored_variant: Variant = response.get("stored_events", [])
	if stored_variant is Array:
		for entry in stored_variant:
			if entry is Dictionary:
				stored_events.append((entry as Dictionary).duplicate(true))
	else:
		stored_events = []

	var result := {
		"score_home": response.get("score_home", 0),
		"score_away": response.get("score_away", 0),
		"events": events,
		"rosters": rosters,  # ✅ Phase A+: Include rosters in result (NEW)
		"stats": response.get("stats", {}),
		"player_ratings": response.get("player_ratings", {}),
		"goals": _extract_goals(events, rosters),  # C7: Pass rosters for name resolution
		"assists": _extract_assists(events, rosters),  # C7: Pass rosters for name resolution
		"yellow_cards": _extract_cards(events, "yellow", rosters),  # C7: Pass rosters for name resolution
		"red_cards": _extract_cards(events, "red", rosters),  # C7: Pass rosters for name resolution
		"timeline": timeline,
		"stored_events": stored_events,
		"position_data": response.get("position_data", {})  # ✅ Pass position_data for minimap
	}
	var goal_heat_variant: Variant = response.get("goal_heat_samples", null)
	if goal_heat_variant is Array:
		result["goal_heat_samples"] = (goal_heat_variant as Array).duplicate(true)
	if timeline_doc_local is Dictionary:
		result["timeline_doc"] = (timeline_doc_local as Dictionary).duplicate(true)
	return result


func _sanitize_timeline_event(event: Dictionary) -> void:
	for key in ["at", "from", "to", "spot", "target", "position", "communication_target", "heading_target"]:
		_normalize_meter_point(event, key)

	if event.has("ball") and event.ball is Dictionary:
		var ball_dict: Dictionary = event.ball
		_normalize_meter_point(ball_dict, "from")
		_normalize_meter_point(ball_dict, "to")
		event.ball = ball_dict

	var base: Variant = event.get("base", {})
	var timestamp_ms := int(event.get("timestamp", 0))
	var time_seconds := float(event.get("time", event.get("minute", 0.0)))
	if base is Dictionary:
		if not event.has("team_id"):
			event["team_id"] = int(base.get("team_id", event.get("team_id", -1)))
		else:
			event["team_id"] = int(event.get("team_id"))
		time_seconds = float(base.get("t", base.get("minute", time_seconds)))
		if not event.has("minute"):
			event["minute"] = float(base.get("minute", time_seconds))
		if not event.has("player_id") and base.has("player_id"):
			event["player_id"] = base.get("player_id")
	if not event.has("minute"):
		event["minute"] = float(event.get("minute", time_seconds))
	if not event.has("time"):
		event["time"] = time_seconds
	if timestamp_ms == 0:
		timestamp_ms = int(round(float(event.get("time", 0.0)) * 1000.0))
	event["timestamp"] = timestamp_ms

	var kind := str(event.get("kind", event.get("type", ""))).to_lower()
	match kind:
		"pass", "through_ball":
			_augment_pass_event_fields(event)
		"shot":
			_augment_shot_event_fields(event)
		"run":
			_augment_run_event_fields(event)
		"dribble":
			_augment_dribble_event_fields(event)
		"pressure", "press":
			_augment_run_event_fields(event)
		"communication":
			_augment_communication_event_fields(event)
		"header":
			_augment_header_event_fields(event)
		"boundary":
			_augment_boundary_event_fields(event)


func _normalize_meter_point(container: Dictionary, key: String) -> void:
	if not container.has(key):
		return
	var point_variant: Variant = container.get(key)
	if typeof(point_variant) != TYPE_DICTIONARY:
		return
	var point: Dictionary = point_variant
	var x_raw: float = float(point.get("x", 0.0))
	var y_raw: float = float(point.get("y", 0.0))
	var x_val: float = clamp(x_raw, 0.0, FIELD_LENGTH_M)
	var y_val: float = clamp(y_raw, 0.0, FIELD_WIDTH_M)
	if (x_val != x_raw or y_val != y_raw) and not _fix02_coord_clamp_warned:
		_fix02_coord_clamp_warned = true
		push_warning(
			(
				"[FIX02][COORD] Clamped event point '%s': (%.3f, %.3f) -> (%.3f, %.3f). "
				+ "Upstream should provide meters within [0..105]x[0..68] or explicit out-of-play contract."
			)
			% [key, x_raw, y_raw, x_val, y_val]
		)
	point["x"] = x_val
	point["y"] = y_val
	container[key] = point


func _augment_pass_event_fields(event: Dictionary) -> void:
	var receiver_field: Variant = event.get("receiver_id", event.get("to_player_id", event.get("target_player", 0)))
	event["receiver_id"] = int(receiver_field)
	_ensure_point_field(event, "from", event.get("origin", null))
	_ensure_point_field(event, "to", event.get("target", null))
	var distance_variant: Variant = event.get("distance_m", event.get("pass_distance_m", event.get("distance", null)))
	if distance_variant != null:
		event["pass_distance_m"] = float(distance_variant)
	else:
		var from_vec: Variant = _vector_from_point(event.get("from", null))
		var to_vec: Variant = _vector_from_point(event.get("to", null))
		if (from_vec is Vector2) and (to_vec is Vector2):
			event["pass_distance_m"] = (from_vec as Vector2).distance_to(to_vec)
	var force_variant: Variant = event.get("pass_force", event.get("force", null))
	if force_variant != null:
		event["pass_force"] = float(force_variant)
	event["is_clearance"] = bool(event.get("is_clearance", false))


func _augment_shot_event_fields(event: Dictionary) -> void:
	var xg_variant: Variant = event.get("xg", event.get("xg_value", null))
	if xg_variant != null:
		event["xg"] = float(xg_variant)
	if not event.has("target"):
		var ball_dict: Variant = event.get("ball", null)
		if ball_dict is Dictionary and ball_dict.has("to"):
			event["target"] = ball_dict.get("to")
	var ball_state: Variant = event.get("ball", null)
	if ball_state is Dictionary:
		event["ball_speed"] = float(ball_state.get("speed_mps", ball_state.get("speed", 0.0)))
		if ball_state.has("curve"):
			event["ball_curve"] = str(ball_state.get("curve"))
	_ensure_point_field(event, "from")
	_ensure_point_field(event, "target")


func _augment_run_event_fields(event: Dictionary) -> void:
	var from_vec: Variant = _ensure_point_field(event, "from")
	var to_vec: Variant = _ensure_point_field(event, "to", event.get("target", null))
	var computed_distance := -1.0
	if (from_vec is Vector2) and (to_vec is Vector2):
		computed_distance = (from_vec as Vector2).distance_to(to_vec)

	var explicit_distance: Variant = event.get(
		"segment_distance_m", event.get("distance_m", event.get("distance", null))
	)
	if explicit_distance != null:
		var distance_val := float(explicit_distance)
		if distance_val > 0.0:
			computed_distance = distance_val
	if computed_distance > 0.0:
		event["segment_distance_m"] = computed_distance
		if not event.has("run_distance_m"):
			event["run_distance_m"] = computed_distance

	var speed_val := float(event.get("speed_mps", event.get("speed", -1.0)))
	if speed_val <= 0.0 and computed_distance > 0.0:
		var duration_val := _derive_duration_seconds(event)
		if duration_val > 0.0:
			speed_val = computed_distance / duration_val
	if speed_val > 0.0:
		event["speed_mps"] = speed_val

	if event.has("with_ball"):
		event["with_ball"] = bool(event.get("with_ball"))


func _augment_dribble_event_fields(event: Dictionary) -> void:
	_augment_run_event_fields(event)
	if event.has("segment_distance_m"):
		event["dribble_distance_m"] = float(event.get("segment_distance_m"))
	var touches_variant: Variant = event.get("touches", event.get("dribble_touches", event.get("touch_count", null)))
	if touches_variant != null:
		var touches := int(touches_variant)
		event["touches"] = touches
		event["dribble_touches"] = touches
	event["with_ball"] = bool(event.get("with_ball", true))


func _augment_communication_event_fields(event: Dictionary) -> void:
	var origin_vec: Variant = _ensure_point_field(event, "position", event.get("at", null))
	if origin_vec == null:
		origin_vec = _ensure_point_field(event, "at")
	var target_vec: Variant = _ensure_point_field(event, "communication_target", event.get("target", null))
	if target_vec == null:
		target_vec = _ensure_point_field(event, "target")
	if (origin_vec is Vector2) and (target_vec is Vector2):
		event["communication_distance_m"] = (origin_vec as Vector2).distance_to(target_vec)
		event["has_target"] = true
	var message_text := str(event.get("message", event.get("message_key", ""))).strip_edges()
	event["message_key"] = message_text
	event["message"] = message_text


func _augment_header_event_fields(event: Dictionary) -> void:
	var origin_vec: Variant = _ensure_point_field(event, "from", event.get("position", null))
	if origin_vec is Vector2:
		event["position"] = event.get("from")
	var direction_variant: Variant = event.get("direction", event.get("direction_vector", null))
	var direction_vec := Vector2.ZERO
	if direction_variant is Dictionary:
		var dict: Dictionary = direction_variant
		direction_vec = Vector2(
			float(dict.get("x", dict.get("0", 0.0))), float(dict.get("y", dict.get("1", dict.get("z", 0.0))))
		)
		event["direction_vector"] = {"x": direction_vec.x, "y": direction_vec.y}
	if (origin_vec is Vector2) and direction_vec != Vector2.ZERO:
		var normalized := direction_vec
		if normalized.length() > 0.001:
			normalized = normalized.normalized()
		var tip := (origin_vec as Vector2) + normalized * 6.0
		event["heading_target"] = {"x": tip.x, "y": tip.y}
		_normalize_meter_point(event, "heading_target")
		event["heading_angle_deg"] = rad_to_deg(direction_vec.angle())


func _augment_boundary_event_fields(event: Dictionary) -> void:
	var pos_vec: Variant = _ensure_point_field(event, "position", event.get("at", null))
	if pos_vec == null:
		pos_vec = _ensure_point_field(event, "at")
	if not event.has("player_id") and event.has("last_touch_player_id"):
		event["player_id"] = int(event.get("last_touch_player_id"))
	if not event.has("team_id") and event.has("last_touch_team_id"):
		event["team_id"] = int(event.get("last_touch_team_id"))


func _ensure_point_field(event: Dictionary, key: String, fallback: Variant = null) -> Variant:
	var existing: Variant = event.get(key, null)
	var vec: Variant = _vector_from_point(existing)
	if vec == null and fallback != null:
		vec = _vector_from_point(fallback)
	if vec is Vector2:
		event[key] = {"x": (vec as Vector2).x, "y": (vec as Vector2).y}
		_normalize_meter_point(event, key)
		return vec
	elif existing is Dictionary:
		_normalize_meter_point(event, key)
		return _vector_from_point(event[key])
	return vec


func _vector_from_point(value: Variant) -> Variant:
	if value is Dictionary:
		var dict: Dictionary = value
		var x_val := float(dict.get("x", dict.get("0", 0.0)))
		var y_val := float(dict.get("z", dict.get("y", dict.get("1", 0.0))))
		return Vector2(x_val, y_val)
	elif value is PackedFloat32Array:
		var arr32: PackedFloat32Array = value
		var x32 := float(arr32[0]) if arr32.size() > 0 else 0.0
		var y32 := float(arr32[2]) if arr32.size() > 2 else (float(arr32[1]) if arr32.size() > 1 else 0.0)
		return Vector2(x32, y32)
	elif value is PackedFloat64Array:
		var arr64: PackedFloat64Array = value
		var x64 := float(arr64[0]) if arr64.size() > 0 else 0.0
		var y64 := float(arr64[2]) if arr64.size() > 2 else (float(arr64[1]) if arr64.size() > 1 else 0.0)
		return Vector2(x64, y64)
	elif value is Array:
		var arr: Array = value
		var x_val2 := float(arr[0]) if arr.size() > 0 else 0.0
		var y_val2 := float(arr[2]) if arr.size() > 2 else (float(arr[1]) if arr.size() > 1 else 0.0)
		return Vector2(x_val2, y_val2)
	return null


func _derive_duration_seconds(event: Dictionary) -> float:
	var candidates := ["duration_s", "duration_sec", "duration_seconds", "duration"]
	for key in candidates:
		if event.has(key):
			var duration_val := float(event.get(key))
			if duration_val > 0.0:
				return duration_val
	if event.has("duration_ms"):
		var duration_ms := float(event.get("duration_ms"))
		if duration_ms > 0.0:
			return duration_ms / 1000.0
	return 0.0


func _dump_match_debug(result: Dictionary) -> void:
	_log_event_summary(result)
	_save_match_dump(result)


func _log_event_summary(result: Dictionary) -> void:
	var events_variant = result.get("events", [])
	if not (events_variant is Array):
		print("[OpenFootballAPI] ⚠️ Match result missing events array for logging")
		return
	var events: Array = events_variant
	if events.is_empty():
		print("[OpenFootballAPI] ⚠️ Match events array is empty")
		return
	var limit: int = min(events.size(), MAX_TEXT_EVENT_LOG)
	print("=== Match Event Summary (%d events, showing %d) ===" % [events.size(), limit])
	for i in range(limit):
		var ev = events[i]
		if typeof(ev) != TYPE_DICTIONARY:
			continue
		var event_dict: Dictionary = ev
		var base_variant: Variant = event_dict.get("base", {})
		var base: Dictionary = base_variant if typeof(base_variant) == TYPE_DICTIONARY else {}
		# Support both old (base.minute) and new (top-level minute) formats
		var minute := float(base.get("minute", event_dict.get("minute", 0.0)))
		var team_label := str(base.get("team_id", event_dict.get("team_id", "")))
		var kind := str(event_dict.get("kind", event_dict.get("type", "unknown")))
		var details: Array = []
		var has_player_id_detail := false
		for field in ["player_name", "player_id", "to", "from", "result", "target_player", "assist_player"]:
			if event_dict.has(field):
				details.append("%s=%s" % [field, str(event_dict[field])])
				if field == "player_id":
					has_player_id_detail = true
		if base.has("player_id") and not has_player_id_detail:
			details.append("player_id=%s" % str(base["player_id"]))
		var detail_str: String = ", ".join(PackedStringArray(details))
		print("  [%d] %0.1f' team=%s kind=%s %s" % [i + 1, minute, team_label, kind, detail_str])
	print("=== End Match Event Summary ===")


func _save_match_dump(result: Dictionary) -> void:
	var err := DirAccess.make_dir_recursive_absolute(MATCH_DEBUG_LOG_DIR)
	if err != OK and err != ERR_ALREADY_EXISTS:
		push_warning("[OpenFootballAPI] Failed to prepare log directory (%s)" % str(err))
		return
	var timestamp := Time.get_datetime_string_from_system(true, true).replace(":", "-")
	var seed_value := str(result.get("seed", "unknown"))
	var file_path := "%s/match_%s_seed_%s.json" % [MATCH_DEBUG_LOG_DIR, timestamp, seed_value]
	var payload := {
		"seed": result.get("seed", null),
		"summary": result.get("summary", {}),
		"timeline": result.get("timeline", []),
		"events": result.get("events", []),
		"rosters": result.get("rosters", {})
	}
	var file := FileAccess.open(file_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(payload, "\t"))
		file.close()
		print("[OpenFootballAPI] Match dump saved -> %s" % file_path)
	else:
		push_warning("[OpenFootballAPI] Failed to write match dump: %s" % file_path)


## C7: Resolve player name from track_id using rosters
func _resolve_player_name_from_track_id(track_id: int, rosters: Dictionary) -> String:
	if track_id == -1 or track_id > 21:
		return "Unknown"

	var is_home: bool = track_id <= 10
	var local_idx: int = track_id if is_home else (track_id - 11)
	var team_key: String = "home" if is_home else "away"

	if not rosters.has(team_key):
		return "Unknown"

	var team_data = rosters.get(team_key, {})
	if not team_data is Dictionary:
		return "Unknown"

	var players = team_data.get("players", [])
	if not players is Array or local_idx >= players.size():
		return "Unknown"

	var player = players[local_idx]
	if player is Dictionary:
		return player.get("name", "Unknown")

	return "Unknown"


## Extract goal scorers from match events
## Supports both "type" (legacy) and "kind" (new format) fields
func _extract_goals(events: Array, rosters: Dictionary = {}) -> Array:
	var goals = []
	for event in events:
		var event_kind: String = str(event.get("kind", event.get("type", ""))).to_lower()
		if event_kind == "goal":
			# C7: Resolve player name from track_id
			var track_id: int = event.get("player_track_id", -1)
			var player_name: String = _resolve_player_name_from_track_id(track_id, rosters)

			goals.append(
				{
					"player": player_name,
					"minute": event.get("minute"),
					"team": event.get("team"),
					"is_home_team": event.get("is_home_team", true)
				}
			)
	return goals


## Extract assists from match events
## Supports both "type" (legacy) and "kind" (new format) fields
func _extract_assists(events: Array, rosters: Dictionary = {}) -> Array:
	var assists = []
	for event in events:
		var event_kind: String = str(event.get("kind", event.get("type", ""))).to_lower()
		if event_kind == "goal":
			# C7: Assist is in target_track_id field
			var assist_track_id: int = event.get("target_track_id", -1)
			if assist_track_id != -1 and assist_track_id <= 21:
				var assist_player: String = _resolve_player_name_from_track_id(assist_track_id, rosters)
				if assist_player != "Unknown":
					assists.append(
						{
							"player": assist_player,
							"minute": event.get("minute"),
							"team": event.get("team"),
							"is_home_team": event.get("is_home_team", true)
						}
					)
	return assists


## Extract cards from match events
## Supports both "type" (legacy) and "kind" (new format) fields
func _extract_cards(events: Array, card_type: String, rosters: Dictionary = {}) -> Array:
	var cards = []
	for event in events:
		var event_kind: String = str(event.get("kind", event.get("type", ""))).to_lower()
		# Check for "yellow_card", "red_card" or legacy "card" with card_type
		var is_match := false
		if card_type == "yellow":
			is_match = (
				event_kind in ["yellow_card", "yellowcard"]
				or (event_kind == "card" and event.get("card_type") == "yellow")
			)
		elif card_type == "red":
			is_match = (
				event_kind in ["red_card", "redcard"] or (event_kind == "card" and event.get("card_type") == "red")
			)
		if is_match:
			# C7: Resolve player name from track_id
			var track_id: int = event.get("player_track_id", -1)
			var player_name: String = _resolve_player_name_from_track_id(track_id, rosters)

			cards.append(
				{
					"player": player_name,
					"minute": event.get("minute"),
					"team": event.get("team"),
					"is_home_team": event.get("is_home_team", true)
				}
			)
	return cards


## ============================================================================
## TYPE CONVERSION UTILITIES
## ============================================================================


## Convert Godot training type to OpenFootball target
func _convert_training_type(godot_type: String) -> String:
	var normalized = godot_type.to_lower().replace(" ", "_")
	return TRAINING_TYPE_MAP.get(normalized, "")


## Convert Godot intensity to OpenFootball intensity
func _convert_intensity(godot_intensity: String) -> String:
	var normalized = godot_intensity.to_lower().replace(" ", "_")
	return INTENSITY_MAP.get(normalized, DEFAULT_INTENSITY)


## Convert Godot position code to OpenFootball position
func _convert_position(godot_position: String) -> String:
	if godot_position == null:
		return POSITION_DEFAULT

	var normalized := str(godot_position).strip_edges().to_upper()
	if normalized == "":
		return POSITION_DEFAULT

	normalized = normalized.get_slice("/", 0)
	normalized = normalized.get_slice(" ", 0)
	normalized = normalized.get_slice("-", 0)
	normalized = normalized.get_slice(",", 0)

	if POSITION_ALIAS_MAP.has(normalized):
		return POSITION_ALIAS_MAP[normalized]

	if normalized in POSITION_CODES:
		return normalized

	var step_length := normalized.length()
	while step_length > 1:
		var prefix := normalized.substr(0, step_length)
		if POSITION_ALIAS_MAP.has(prefix):
			return POSITION_ALIAS_MAP[prefix]
		if prefix in POSITION_CODES:
			return prefix
		step_length -= 1

	return POSITION_DEFAULT  # Default to midfielder


func _normalize_core_player(core_player: Dictionary) -> Dictionary:
	var normalized := core_player.duplicate(true)
	normalized["name"] = str(normalized.get("name", "Player"))
	normalized["position"] = _convert_position(normalized.get("position", POSITION_DEFAULT))
	normalized["ca"] = int(normalized.get("ca", 0))
	normalized["pa"] = int(normalized.get("pa", normalized.get("ca", 0)))
	normalized["overall"] = int(normalized.get("overall", normalized.get("ca", 0)))
	normalized["condition"] = int(normalized.get("condition", 0))
	normalized["stamina"] = int(normalized.get("stamina", 0))
	normalized["form"] = int(normalized.get("form", 0))
	normalized["morale"] = int(normalized.get("morale", 0))
	if normalized.has("attributes"):
		normalized["attributes"] = _coerce_attribute_map(normalized.get("attributes"))
	if normalized.has("detailed_stats"):
		var stats: Dictionary = {}
		for key in normalized.detailed_stats.keys():
			stats[str(key)] = int(round(float(normalized.detailed_stats[key])))
		normalized["detailed_stats"] = stats
	return normalized


func _coerce_attribute_map(values: Dictionary) -> Dictionary:
	var result: Dictionary = {}
	for key in values.keys():
		result[key] = int(round(float(values[key])))
	return result


## Convert OpenFootball condition to Godot form
func convert_condition_to_form(condition: String) -> int:
	match condition:
		"Excellent":
			return 90
		"Good":
			return 70
		"Normal":
			return 50
		"Poor":
			return 30
		"Terrible":
			return 10
		_:
			return 50


## ============================================================================
## HELPER METHODS
## ============================================================================


## Get recommended training intensity based on stamina
func get_recommended_intensity(stamina: int) -> String:
	if stamina >= 80:
		return "high"
	elif stamina >= 60:
		return "moderate"
	elif stamina >= 40:
		return "light"
	else:
		return "very_light"


## Check if player can train
func can_train(player_data: Dictionary) -> Dictionary:
	var stamina = player_data.get("stamina", 100)
	var injury_risk = player_data.get("injury_risk", 0.0)

	if stamina < 20:
		return {"can_train": false, "reason": "Stamina too low (forced rest required)", "recommended_action": "rest"}

	if injury_risk > 0.8:
		return {"can_train": false, "reason": "Injury risk too high", "recommended_action": "rest"}

	if stamina < 40:
		return {"can_train": true, "reason": "Can train, but low stamina", "recommended_action": "light_training"}

	return {"can_train": true, "reason": "Good condition", "recommended_action": "normal_training"}


## Calculate form multiplier from form value
func calculate_form_multiplier(form: int) -> float:
	if form >= 80:
		return 1.15  # Excellent
	elif form >= 60:
		return 1.05  # Good
	elif form >= 40:
		return 1.00  # Average
	elif form >= 20:
		return 0.90  # Poor
	else:
		return 0.75  # Terrible


## Apply form effect to training result
func apply_form_to_training(training_result: Dictionary, form: int) -> Dictionary:
	var multiplier = calculate_form_multiplier(form)

	# Apply multiplier to CA growth
	if training_result.has("ca_growth"):
		training_result.ca_growth = int(training_result.ca_growth * multiplier)
		training_result.ca_after = training_result.ca_before + training_result.ca_growth

	# Adjust improved attributes
	if training_result.has("improved_attributes"):
		for attr in training_result.improved_attributes:
			attr.improvement *= multiplier

	return training_result


## ============================================================================
## ERROR HANDLING
## ============================================================================


## Ensure Rust engine is available
func _ensure_engine() -> bool:
	if rust_engine == null:
		rust_engine = get_node_or_null("/root/FootballRustEngine")

	if rust_engine == null:
		last_error = "FootballRustEngine not found"
		error_count += 1
		push_error("[OpenFootballAPI] %s" % last_error)
		return false

	return true


## Create error result dictionary
func _error_result(error_message: String) -> Dictionary:
	last_error = error_message
	error_count += 1
	push_error("[OpenFootballAPI] %s" % error_message)

	return {"success": false, "response": {}, "error": error_message}


## Generate random seed for API calls
func _generate_seed() -> int:
	return DEFAULT_SEED_BASE + randi() % 90000  # 10000-99999


## ============================================================================
## ABILITY SYSTEM API METHODS (Phase 2 Wrapper Layer Enforcement)
## ============================================================================


## Calculate training efficiency based on personality traits
## High-level wrapper for FootballRustEngine.calculate_training_efficiency()
func calculate_training_efficiency(personality: Dictionary) -> float:
	"""
	Calculate training efficiency multiplier based on personality traits

	Args:
		personality: Dictionary with traits (discipline, professionalism, determination, ambition)

	Returns:
		Float efficiency multiplier (0.0-1.0), or 0.5 as default on error
	"""
	if not _ensure_engine():
		push_error("[OpenFootballAPI] Engine not ready for training efficiency calculation")
		return 0.5  # Default efficiency

	# Validate personality dict
	if personality.is_empty():
		push_warning("[OpenFootballAPI] Empty personality dict, using default efficiency")
		return 0.5

	return rust_engine.calculate_training_efficiency(personality)


## Calculate effects of special abilities on player performance
## High-level wrapper for FootballRustEngine.calculate_ability_effects()
func calculate_ability_effects(special_abilities: Array) -> Dictionary:
	"""
	Calculate performance effects from player's special abilities

	Args:
		special_abilities: Array of ability dictionaries

	Returns:
		Dictionary with calculated ability effects, or empty dict on error
	"""
	if not _ensure_engine():
		push_error("[OpenFootballAPI] Engine not ready for ability calculation")
		return {}

	# Validate abilities array
	if special_abilities.is_empty():
		return {}  # No abilities = no effects

	return rust_engine.calculate_ability_effects(special_abilities)


## Process ability combinations and synergies
## High-level wrapper for FootballRustEngine.process_ability_combinations()
func process_ability_combinations(ability_collection: Array, context: Dictionary) -> Dictionary:
	"""
	Process ability combinations and detect synergies

	Args:
		ability_collection: Array of player abilities
		context: Dictionary with context info (training type, match situation, etc.)

	Returns:
		Dictionary with combinations detected and effects applied
	"""
	if not _ensure_engine():
		push_error("[OpenFootballAPI] Engine not ready for ability combinations")
		return {"combinations": []}

	# Validate inputs
	if ability_collection.is_empty():
		return {"combinations": []}  # No abilities = no combinations

	return rust_engine.process_ability_combinations(ability_collection, context)


## Check if player acquires new ability from training
## High-level wrapper for FootballRustEngine.check_ability_acquisition()
func check_ability_acquisition(training_type: String, quality: float, coach_specialty: String = "") -> Dictionary:
	"""
	Check if player acquires new special ability from training

	Args:
		training_type: Type of training executed (e.g., "technical_passing")
		quality: Training quality score (0.0-1.0)
		coach_specialty: Coach's specialty ability type (optional)

	Returns:
		Dictionary with acquisition result { acquired: bool, ability: Dictionary }
	"""
	if not _ensure_engine():
		push_error("[OpenFootballAPI] Engine not ready for ability acquisition check")
		return {"acquired": false}

	# Validate inputs
	if training_type == "":
		push_warning("[OpenFootballAPI] Empty training type for ability acquisition")
		return {"acquired": false}

	if quality < 0.0 or quality > 1.0:
		push_warning("[OpenFootballAPI] Invalid quality value: %f (clamping to 0.0-1.0)" % quality)
		quality = clamp(quality, 0.0, 1.0)

	return rust_engine.check_ability_acquisition(training_type, quality, coach_specialty)
