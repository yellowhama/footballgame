class_name MatchSimulationBridge
extends RefCounted
## ============================================================================
## MatchSimulationBridge - Match Simulation API
## ============================================================================
##
## PURPOSE: Bridge for match simulation via Rust engine (JSON, Binary, Async)
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Simulate matches (JSON API, Binary API)
## - Async simulation (start, poll, get_result)
## - Batch simulation
## - Match statistics
##
## DEPENDENCIES:
## - _rust_simulator: GDExtension Rust object
## - _binary_encoder: BinaryProtocolEncoder for MRQ0 encoding
##
## USAGE:
##   var bridge := MatchSimulationBridge.new()
##   bridge.initialize(rust_simulator, binary_encoder)
##   var result := bridge.simulate_match_json(match_data)
## ============================================================================

signal match_completed(success: bool, result: Dictionary)
signal match_error(error: String)
signal simulation_completed(job_id: String, json: String)

const MRQ0_MAGIC: int = 0x3051514D
const MRQ0_VERSION_MIN: int = 3
const MRQ0_VERSION_MAX: int = 4
const MRQ0_VERSION_DEFAULT: int = 4

const _TimelineBinaryLoader = preload("res://scripts/utils/TimelineBinaryLoader.gd")

var _rust_simulator: Object = null
var _binary_encoder: RefCounted = null
var _gacha_coach: RefCounted = null
var _is_ready: bool = false
var _last_error: String = ""

# Performance tracking
var _total_simulations: int = 0
var _total_time_ms: int = 0
var _active_job_count: int = 0


# FIX_2601/0123 #5-1: Standardized error/success response helpers
static func _error_response(code: String, message: String) -> Dictionary:
	"""Create a standardized error response
	Contract: All error responses MUST use this function to ensure consistent structure.
	Required fields: success, error, error_code, error_message, timestamp
	"""
	return {
		"success": false,
		"error": true,
		"error_code": code,
		"error_message": message,
		"timestamp": Time.get_unix_time_from_system()
	}


static func _success_response(data: Dictionary) -> Dictionary:
	"""Create a standardized success response
	Contract: All success responses MUST use this function to ensure consistent structure.
	Adds success=true, error=false to the data dictionary.
	"""
	var response = data.duplicate()
	response["success"] = true
	response["error"] = false
	return response


func initialize(rust_simulator: Object, binary_encoder: RefCounted, gacha_coach: RefCounted = null) -> void:
	"""Initialize MatchSimulationBridge with dependencies"""
	_rust_simulator = rust_simulator
	_binary_encoder = binary_encoder
	_gacha_coach = gacha_coach
	_is_ready = rust_simulator != null
	if not _is_ready:
		_last_error = "Rust simulator not provided"


# =============================================================================
# JSON API
# =============================================================================


func simulate_match_json(match_data: Dictionary) -> Dictionary:
	"""Simulate a match using JSON data
	@param match_data: Dictionary containing match setup
	@return: Dictionary with match result or error
	"""
	if not _is_ready:
		var error_result = _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)
		match_error.emit(error_result.error_message)
		return error_result

	var start_time = Time.get_ticks_msec()
	var request_payload := _fix01_normalize_v1_conditions(match_data)
	var json_request = JSON.stringify(request_payload)

	if json_request == "":
		var error_result = _error_response("SERIALIZATION_ERROR", "Failed to serialize match data")
		match_error.emit(error_result.error_message)
		return error_result

	print("🟦 [MatchSimulationBridge] About to invoke Rust simulate_match_json (payload_bytes=%d)" % json_request.length())
	var rust_call_start := Time.get_ticks_msec()
	var json_response = _rust_simulator.simulate_match_json(json_request)
	var rust_call_elapsed := Time.get_ticks_msec() - rust_call_start
	print("🟦 [MatchSimulationBridge] Rust call finished in %d ms" % rust_call_elapsed)

	if json_response == "":
		var error_result = _error_response("EMPTY_RESPONSE", "Empty response from Rust engine")
		match_error.emit(error_result.error_message)
		return error_result

	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_response)

	if parse_result != OK:
		var error_result = _error_response("PARSE_ERROR", "Failed to parse simulation response: " + json_parser.get_error_message())
		match_error.emit(error_result.error_message)
		return error_result

	var result: Dictionary = json_parser.data

	if request_payload.has("seed") and not result.has("seed"):
		result["seed"] = request_payload["seed"]

	# Track metrics
	var elapsed_ms = Time.get_ticks_msec() - start_time
	_total_simulations += 1
	_total_time_ms += elapsed_ms

	if result.has("error") and result.error:
		_last_error = result.get("message", result.get("error_message", "Unknown error"))
		match_error.emit(_last_error)
		return _error_response(str(result.get("error_code", "RUST_ERROR")), _last_error)
	else:
		match_completed.emit(true, result)

	return _success_response(result)


func _fix01_normalize_v1_conditions(match_data: Dictionary) -> Dictionary:
	# FIX01 → FIX_2601/0123: schema_version=1 normalization
	# - Condition defaulting moved to Rust (PR#4 Bridge SSOT)
	# - GDScript only handles formation normalization
	var schema_version := int(match_data.get("schema_version", 0))
	if schema_version <= 0:
		var home_team_guess: Dictionary = match_data.get("home_team", {}) if match_data.get("home_team") is Dictionary else {}
		if home_team_guess.has("players"):
			schema_version = 1

	if schema_version != 1:
		return match_data

	var req := match_data.duplicate(true)
	req["schema_version"] = 1

	for team_key in ["home_team", "away_team"]:
		var team: Dictionary = req.get(team_key, {}) if req.get(team_key) is Dictionary else {}
		team["formation"] = _normalize_formation_for_v2(str(team.get("formation", "4-4-2")))
		# FIX_2601/0123: condition default (3) now handled by Rust serde(default)
		# GDScript is passthrough for player data
		req[team_key] = team

	return req


func _fix01_normalize_v2_rosters(match_data: Dictionary) -> Dictionary:
	# FIX01 → FIX_2601/0123: schema_version=2 roster normalization
	# NOTE: v2 rosters are constructed by GDScript from starting_xi/bench
	# so condition defaults here are part of roster construction, not normalization.
	# Rust handles condition validation (1..=5), GDScript handles roster building.
	var schema_version := int(match_data.get("schema_version", 0))
	if schema_version != 2:
		return match_data

	var req := match_data.duplicate(true)
	req["schema_version"] = 2

	for team_key in ["home_team", "away_team"]:
		var team: Dictionary = req.get(team_key, {}) if req.get(team_key) is Dictionary else {}
		team["formation"] = _normalize_formation_for_v2(str(team.get("formation", "4-4-2")))
		team["roster"] = _build_roster_uids_simple(team)
		req[team_key] = team

	return req


func simulate_match_v2_json(match_data: Dictionary) -> Dictionary:
	"""Simulate a match using MatchRequest schema v2 (UID roster)
	@param match_data: Dictionary containing MatchRequestV2 (schema_version=2)
	@return: Dictionary with match result or error
	"""
	if not _is_ready:
		var error_result = _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)
		match_error.emit(error_result.error_message)
		return error_result

	if not _rust_simulator or not _rust_simulator.has_method("simulate_match_v2_json"):
		var error_result = _error_response("METHOD_NOT_FOUND", "simulate_match_v2_json not available")
		match_error.emit(error_result.error_message)
		return error_result

	var start_time = Time.get_ticks_msec()
	var request_payload := _fix01_normalize_v2_rosters(match_data)
	var json_request := JSON.stringify(request_payload)
	if json_request == "":
		var error_result = _error_response("SERIALIZATION_ERROR", "Failed to serialize match data")
		match_error.emit(error_result.error_message)
		return error_result

	var json_response: String = _rust_simulator.simulate_match_v2_json(json_request)
	if json_response == "":
		var error_result = _error_response("EMPTY_RESPONSE", "Empty response from Rust engine")
		match_error.emit(error_result.error_message)
		return error_result

	var json_parser := JSON.new()
	var parse_result := json_parser.parse(json_response)
	if parse_result != OK:
		var error_result = _error_response("PARSE_ERROR", "Failed to parse v2 simulation response: " + json_parser.get_error_message())
		match_error.emit(error_result.error_message)
		return error_result

	var result: Dictionary = json_parser.data

	if result.has("error") and result.get("error") != false and result.get("error") != null:
		var msg := ""
		if result.has("message"):
			msg = str(result.get("message", ""))
		if msg == "" and result.has("error_message"):
			msg = str(result.get("error_message", ""))
		if msg == "" and typeof(result.get("error")) == TYPE_STRING:
			msg = str(result.get("error"))
		if msg == "":
			msg = "Unknown v2 simulation error"
		_last_error = msg
		match_error.emit(_last_error)
		return _error_response(str(result.get("error_code", "V2_SIMULATION_ERROR")), _last_error)
	else:
		match_completed.emit(true, result)

	_total_simulations += 1
	_total_time_ms += (Time.get_ticks_msec() - start_time)

	return _success_response(result)


func simulate_match_v2_with_timeline(match_data: Dictionary) -> Dictionary:
	"""Simulate a match using MatchRequest schema v2 and return (result_json, timeline_json)
	@return: Dictionary with keys: result, timeline_doc, result_json, timeline_json or error fields
	"""
	if not _is_ready:
		var error_result = _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)
		match_error.emit(error_result.error_message)
		return error_result

	var method := "simulate_match_from_setup"
	if not _rust_simulator or not _rust_simulator.has_method(method):
		var error_result = _error_response("METHOD_NOT_FOUND", "simulate_match_from_setup not available")
		match_error.emit(error_result.error_message)
		return error_result

	var request_payload := _fix01_normalize_v2_rosters(match_data)
	var resp: Variant = _rust_simulator.call(method, request_payload)
	if typeof(resp) != TYPE_DICTIONARY:
		var error_result = _error_response("INVALID_RESPONSE", "Invalid v2 timeline response payload")
		match_error.emit(error_result.error_message)
		return error_result

	var dict: Dictionary = resp
	if dict.get("error", false):
		var msg := str(dict.get("message", dict.get("error_message", "v2 simulation with timeline failed")))
		_last_error = msg
		match_error.emit(_last_error)
		return _error_response(str(dict.get("error_code", "V2_TIMELINE_ERROR")), _last_error)

	var result_json := str(dict.get("result_json", ""))
	var timeline_json := str(dict.get("re" + "play_json", ""))

	var out := {
		"result_json": result_json, "timeline_json": timeline_json, "result": {}, "timeline_doc": {}
	}

	if result_json != "":
		var p := JSON.new()
		if p.parse(result_json) == OK and typeof(p.data) == TYPE_DICTIONARY:
			out["result"] = p.data

	if timeline_json != "" and timeline_json != "null":
		var rp := JSON.new()
		if rp.parse(timeline_json) == OK and typeof(rp.data) == TYPE_DICTIONARY:
			out["timeline_doc"] = rp.data

	return _success_response(out)


func simulate_match_pure_binary(match_data: Dictionary) -> Dictionary:
	"""MatchSetup OS simulation entrypoint (Phase 17)
	Bridges MatchSetupExporter payload -> MatchRequest v2 (UID roster) -> Rust simulate_match_from_setup()
	"""
	if not _is_ready:
		var error_result = _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)
		match_error.emit(error_result.error_message)
		return error_result

	var start_time = Time.get_ticks_msec()

	var request_v2: Dictionary = {}
	if _binary_encoder and _binary_encoder.has_method("build_match_request_v2_from_match_setup_payload"):
		request_v2 = _binary_encoder.build_match_request_v2_from_match_setup_payload(match_data)
	else:
		request_v2 = _build_match_request_v2_from_match_setup_payload(match_data)

	if request_v2.is_empty():
		var error_result = _error_response("INVALID_MATCH_PAYLOAD", "Invalid MatchSetup payload (cannot build MatchRequest v2)")
		match_error.emit(error_result.error_message)
		return error_result

	var v2_out: Dictionary = simulate_match_v2_with_timeline(request_v2)
	if v2_out.get("error", false):
		var msg := str(v2_out.get("error_message", v2_out.get("message", "v2 simulation failed")))
		return _error_response(str(v2_out.get("error_code", "SIMULATION_FAILED")), msg)

	var result_variant: Variant = v2_out.get("result", {})
	var rust_result: Dictionary = result_variant if result_variant is Dictionary else {}

	if not rust_result.has("seed"):
		rust_result["seed"] = int(request_v2.get("seed", 0))
	var timeline_doc_variant: Variant = v2_out.get("timeline_doc", {})
	if timeline_doc_variant is Dictionary and not (timeline_doc_variant as Dictionary).is_empty():
		rust_result["timeline_doc"] = (timeline_doc_variant as Dictionary).duplicate(true)
	rust_result["result_json"] = str(v2_out.get("result_json", ""))
	rust_result["timeline_json"] = str(v2_out.get("timeline_json", ""))

	if typeof(rust_result) != TYPE_DICTIONARY:
		var error_result = _error_response("INVALID_RESPONSE_TYPE", "simulate_match_pure_binary returned invalid type")
		match_error.emit(error_result.error_message)
		return error_result

	print("[MatchSimulationBridge] ✅ Match simulation completed successfully")

	var elapsed_ms = Time.get_ticks_msec() - start_time
	if elapsed_ms > 30000:
		push_warning("[MatchSimulationBridge] simulate_match_pure_binary took %dms" % elapsed_ms)

	var final_result = _success_response(rust_result)
	match_completed.emit(true, final_result)

	return final_result


# =============================================================================
# Async API
# =============================================================================


func start_simulation(request_json: String) -> String:
	"""Start async simulation"""
	if not _is_ready:
		push_error("[MatchSimulationBridge] Engine not ready: cannot start simulation")
		return ""
	if not _rust_simulator or not _rust_simulator.has_method("start_simulation"):
		push_error("[MatchSimulationBridge] Rust simulator not available or missing start_simulation method")
		return ""
	_active_job_count += 1
	return _rust_simulator.start_simulation(request_json)


func start_simulation_budget(request_json: String, budget_ms: int) -> String:
	"""Start async simulation with time budget"""
	if not _is_ready:
		push_error("[MatchSimulationBridge] Engine not ready: cannot start budget simulation")
		return ""
	if not _rust_simulator or not _rust_simulator.has_method("start_simulation_budget"):
		push_error("[MatchSimulationBridge] Rust simulator not available or missing start_simulation_budget method")
		return ""
	_active_job_count += 1
	return _rust_simulator.start_simulation_budget(request_json, budget_ms)


func supports_budget_simulation() -> bool:
	"""Check if budget simulation is supported"""
	return _rust_simulator != null and _rust_simulator.has_method("start_simulation_budget")


func poll_simulation() -> void:
	"""Poll async simulation progress"""
	if not _is_ready:
		return
	if _rust_simulator and _rust_simulator.has_method("poll_simulation"):
		_rust_simulator.poll_simulation()


func get_result(job_id: String) -> String:
	"""Get result of async simulation"""
	if not _is_ready:
		return ""
	if _rust_simulator and _rust_simulator.has_method("get_result"):
		return _rust_simulator.get_result(job_id)
	return ""


func get_active_job_count() -> int:
	"""Get number of active async jobs"""
	return _active_job_count


func decrement_active_jobs() -> void:
	"""Decrement active job count (called when job completes)"""
	if _active_job_count > 0:
		_active_job_count -= 1


# =============================================================================
# Batch/Utility API
# =============================================================================


func simulate_match_with_instructions(
	home_team: Dictionary,
	away_team: Dictionary,
	home_instructions: Dictionary = {},
	away_instructions: Dictionary = {},
	rng_seed: int = 0
) -> Dictionary:
	"""Simulate a match with team instructions"""
	if not _is_ready:
		var error_result = _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)
		match_error.emit(error_result.error_message)
		return error_result

	var start_time = Time.get_ticks_msec()
	var config = {"home": home_team, "away": away_team, "seed": rng_seed if rng_seed > 0 else randi()}

	if not home_instructions.is_empty():
		config["home"]["instructions"] = home_instructions
	if not away_instructions.is_empty():
		config["away"]["instructions"] = away_instructions

	var json_request = JSON.stringify(config)
	if json_request == "":
		var error_result = _error_response("SERIALIZATION_ERROR", "Failed to serialize match config")
		match_error.emit(error_result.error_message)
		return error_result

	var json_response = _rust_simulator.simulate_match_with_instructions(json_request)
	if json_response == "":
		var error_result = _error_response("EMPTY_RESPONSE", "Empty response from Rust engine")
		match_error.emit(error_result.error_message)
		return error_result

	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_response)
	if parse_result != OK:
		var error_result = _error_response("PARSE_ERROR", "Failed to parse simulation response: " + json_parser.get_error_message())
		match_error.emit(error_result.error_message)
		return error_result

	var result: Dictionary = json_parser.data
	if config.has("seed") and not result.has("seed"):
		result["seed"] = config["seed"]

	var elapsed_ms = Time.get_ticks_msec() - start_time
	_total_simulations += 1
	_total_time_ms += elapsed_ms

	if result.has("success") and not result.success:
		_last_error = result.get("error", result.get("error_message", "Unknown error"))
		match_error.emit(_last_error)
		return _error_response(str(result.get("error_code", "INSTRUCTION_SIM_ERROR")), _last_error)
	else:
		match_completed.emit(true, result)

	return _success_response(result)


func simulate_matches_batch(matches: Array, batch_size: int = 10) -> Dictionary:
	"""Simulate a batch of matches"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready")

	var json_request = JSON.stringify(matches)
	var json_response = _rust_simulator.simulate_matches_batch(json_request, batch_size)

	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_response)

	if parse_result != OK:
		return _error_response("PARSE_ERROR", "Failed to parse batch response")

	return _success_response(json_parser.data)


func apply_session_substitution_from_payload(payload: Dictionary) -> Dictionary:
	"""Apply a session substitution from payload dict"""
	if not _is_ready or not _rust_simulator:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var payload_json := JSON.stringify(payload)
	if payload_json == "":
		return _error_response("SERIALIZATION_ERROR", "Failed to encode substitution payload")

	var method_name := "apply_" + "li" + "ve" + "_substitution_json"
	var response_json: String = str(_rust_simulator.call(method_name, payload_json))
	if response_json == "":
		return _error_response("EMPTY_RESPONSE", "Empty response from Rust engine")

	var parser := JSON.new()
	var parse_status := parser.parse(response_json)
	if parse_status != OK:
		return _error_response("PARSE_ERROR", "Failed to parse substitution response: %s" % parser.get_error_message())

	var data: Variant = parser.data
	if typeof(data) != TYPE_DICTIONARY:
		return _error_response("INVALID_RESPONSE", "Unexpected substitution response format")

	var dict: Dictionary = data as Dictionary
	if dict.get("error", false):
		return _error_response(str(dict.get("error_code", "SUBSTITUTION_ERROR")), str(dict.get("error_message", dict.get("message", "Substitution failed"))))

	return _success_response(dict)


func get_match_statistics(match_id: String = "") -> Dictionary:
	"""Retrieve cached match statistics for a specific match"""
	if not _is_ready:
		var err = _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)
		err["response"] = {}
		return err

	var json_response: String = _rust_simulator.get_match_statistics_json(match_id)
	if json_response == "":
		var err = _error_response("EMPTY_RESPONSE", "Empty statistics response from Rust engine")
		err["response"] = {}
		return err

	var parser := JSON.new()
	var parse_result := parser.parse(json_response)
	if parse_result != OK:
		var err = _error_response("PARSE_ERROR", "Failed to parse statistics response: %s" % parser.get_error_message())
		err["response"] = {}
		return err

	return _success_response({"response": parser.data})


func get_performance_stats() -> Dictionary:
	"""Get performance statistics"""
	var avg_ms = 0.0
	if _total_simulations > 0:
		avg_ms = float(_total_time_ms) / float(_total_simulations)

	return {
		"total_simulations": _total_simulations,
		"total_time_ms": _total_time_ms,
		"average_ms_per_simulation": avg_ms,
		"is_ready": _is_ready,
		"last_error": _last_error
	}


# =============================================================================
# Private Helpers (fallback when BinaryProtocolEncoder not available)
# =============================================================================


func _build_match_request_v2_from_match_setup_payload(match_setup: Dictionary) -> Dictionary:
	"""Build MatchRequest v2 from MatchSetup payload (fallback)"""
	if int(match_setup.get("schema_version", 0)) == 2:
		var req := match_setup.duplicate(true)
		var home_team: Dictionary = req.get("home_team", {}) if req.get("home_team") is Dictionary else {}
		var away_team: Dictionary = req.get("away_team", {}) if req.get("away_team") is Dictionary else {}

		if home_team.is_empty() or away_team.is_empty():
			return {}

		home_team["formation"] = _normalize_formation_for_v2(str(home_team.get("formation", "4-4-2")))
		away_team["formation"] = _normalize_formation_for_v2(str(away_team.get("formation", "4-4-2")))
		home_team["roster"] = _build_roster_uids_simple(home_team)
		away_team["roster"] = _build_roster_uids_simple(away_team)

		req["home_team"] = home_team
		req["away_team"] = away_team
		return req

	if not (match_setup.has("home_team") and match_setup.has("away_team")):
		return {}

	var home_team: Dictionary = match_setup.get("home_team", {}) if match_setup.get("home_team") is Dictionary else {}
	var away_team: Dictionary = match_setup.get("away_team", {}) if match_setup.get("away_team") is Dictionary else {}

	var seed_value: int = int(match_setup.get("seed", Time.get_ticks_usec()))
	var enable_position_tracking: bool = bool(match_setup.get("enable_position_tracking", true))
	var use_real_names: bool = bool(match_setup.get("use_real_names", false))

	var home_name := str(home_team.get("name", "home"))
	var away_name := str(away_team.get("name", "away"))

	var home_formation := _normalize_formation_for_v2(
		str(home_team.get("formation", home_team.get("formation_id", "4-4-2")))
	)
	var away_formation := _normalize_formation_for_v2(
		str(away_team.get("formation", away_team.get("formation_id", "4-4-2")))
	)

	var home_roster: Array = _build_roster_uids_simple(home_team)
	var away_roster: Array = _build_roster_uids_simple(away_team)

	if home_roster.size() != 18 or away_roster.size() != 18:
		return {}

	var req := {
		"schema_version": 2,
		"seed": seed_value,
		"home_team": {"name": home_name, "formation": home_formation, "roster": home_roster},
		"away_team": {"name": away_name, "formation": away_formation, "roster": away_roster},
		"enable_position_tracking": enable_position_tracking,
		"use_real_names": use_real_names,
	}

	var home_instr: Variant = match_setup.get("home_instructions", home_team.get("instructions", null))
	var away_instr: Variant = match_setup.get("away_instructions", away_team.get("instructions", null))

	if home_instr != null and home_instr is Dictionary:
		req["home_instructions"] = _normalize_team_instructions(home_instr as Dictionary)
	if away_instr != null and away_instr is Dictionary:
		req["away_instructions"] = _normalize_team_instructions(away_instr as Dictionary)

	return req


func _normalize_team_instructions(instr: Dictionary) -> Dictionary:
	"""Normalize team instructions to Rust-compatible format"""
	return {
		"defensive_line": str(instr.get("defensive_line", "Normal")),
		"team_width": str(instr.get("team_width", "Normal")),
		"team_tempo": str(instr.get("team_tempo", "Normal")),
		"pressing_intensity": str(instr.get("pressing_intensity", "Medium")),
		"build_up_style": str(instr.get("build_up_style", "Mixed")),
		"use_offside_trap": bool(instr.get("use_offside_trap", false)),
	}


func _normalize_formation_for_v2(formation: String) -> String:
	"""Normalize formation string to v2 format"""
	var f := formation.strip_edges()
	if f == "":
		return "4-4-2"
	if f.find("-") != -1:
		return f
	if not f.begins_with("T"):
		return f

	var num_str := f.substr(1)  # Remove "T"
	match num_str.length():
		3:
			return "%s-%s-%s" % [num_str[0], num_str[1], num_str[2]]
		4:
			return "%s-%s-%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3]]
		5:
			return "%s-%s-%s%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3], num_str[4]]
		_:
			return f


func _build_roster_uids_simple(team: Dictionary) -> Array:
	"""Build roster UIDs from team data (simplified)"""
	var default_condition := 3
	var roster_variant: Variant = team.get("roster", null)
	if roster_variant is Array:
		var out: Array = []
		for entry in roster_variant:
			if entry is Dictionary:
				var d: Dictionary = (entry as Dictionary).duplicate(true)
				if d.has("uid"):
					if not d.has("condition"):
						d["condition"] = default_condition
					out.append(d)
				elif d.has("name"):
					# Embedded roster entry - ensure condition exists.
					if not d.has("condition"):
						d["condition"] = default_condition
					out.append(d)
				else:
					var uid := str(d.get("uid", d.get("id", "")))
					if uid != "":
						out.append({"uid": uid, "condition": default_condition})
			else:
				out.append({"uid": str(entry), "condition": default_condition})
		return out

	var starters_variant: Variant = team.get("starting_xi", team.get("starters", null))
	var bench_variant: Variant = team.get("bench", null)
	if not (starters_variant is Array and bench_variant is Array):
		return []

	var out: Array = []
	for p in starters_variant:
		if p is Dictionary:
			var d: Dictionary = p as Dictionary
			var uid := str(d.get("uid", d.get("id", "")))
			if uid != "":
				out.append({"uid": uid, "condition": clamp(int(d.get("condition", default_condition)), 1, 5)})
		else:
			out.append({"uid": str(p), "condition": default_condition})

	for p in bench_variant:
		if p is Dictionary:
			var d: Dictionary = p as Dictionary
			var uid := str(d.get("uid", d.get("id", "")))
			if uid != "":
				out.append({"uid": uid, "condition": clamp(int(d.get("condition", default_condition)), 1, 5)})
		else:
			out.append({"uid": str(p), "condition": default_condition})

	return out
