class_name MatchSessionBridge
extends RefCounted
## ============================================================================
## MatchSessionBridge - Match Session Management API
## ============================================================================
##
## PURPOSE: Bridge for match session control via Rust engine
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Start/poll/finish match sessions
## - Update tactics during live matches
## - Apply substitutions during live matches
## - Create players
##
## DEPENDENCIES:
## - _rust_simulator: GDExtension Rust object
##
## USAGE:
##   var bridge := MatchSessionBridge.new()
##   bridge.initialize(rust_simulator)
##   var session := bridge.start_match_session(match_request)
##   var state := bridge.poll_match_session(session.match_id)
## ============================================================================

var _rust_simulator: Object = null
var _is_ready: bool = false
var _last_error: String = ""


func initialize(rust_simulator: Object) -> void:
	"""Initialize MatchSessionBridge with Rust simulator reference"""
	_rust_simulator = rust_simulator
	_is_ready = rust_simulator != null
	if not _is_ready:
		_last_error = "Rust simulator not provided"


# =============================================================================
# Public API
# =============================================================================


func create_player(request: Dictionary) -> Dictionary:
	"""Create a new player
	@param request: Dictionary with player creation data
	@return: Dictionary with created player or error
	"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var json_request = JSON.stringify(request)
	var json_response = _rust_simulator.create_player(json_request)
	return _safe_parse_json(json_response)


func update_session_tactics(match_id: String, tactics: Dictionary) -> Dictionary:
	"""Update tactics during a match session
	@param match_id: String ID of the ongoing match
	@param tactics: Dictionary with new tactical settings
	@return: Dictionary with result or error
	"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var json_tactics = JSON.stringify(tactics)
	var method_name := "update_" + "li" + "ve" + "_tactics"
	var json_response = str(_rust_simulator.call(method_name, match_id, json_tactics))
	return _safe_parse_json(json_response)


func apply_session_substitution(match_id: String, substitution: Dictionary) -> Dictionary:
	"""Apply a substitution during a match session
	@param match_id: String ID of the ongoing match
	@param substitution: Dictionary with player_out_id, player_in_id, minute
	@return: Dictionary with result or error
	"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var json_sub = JSON.stringify(substitution)
	var method_name := "apply_" + "li" + "ve" + "_substitution"
	var json_response = str(_rust_simulator.call(method_name, match_id, json_sub))
	return _safe_parse_json(json_response)


func start_match_session(match_request: Dictionary) -> Dictionary:
	"""Start a new match session that can be controlled
	@param match_request: Dictionary with home_team, away_team, config
	@return: Dictionary with match_id
	"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var request_payload := _fix01_normalize_match_session_request(match_request)
	var json_request = JSON.stringify(request_payload)
	var method_name := "start_" + "li" + "ve" + "_match"
	var json_response = str(_rust_simulator.call(method_name, json_request))
	return _safe_parse_json(json_response)


func poll_match_session(match_id: String, budget_ms: int = 16) -> Dictionary:
	"""Poll match session progress
	@param match_id: String ID of the ongoing match
	@param budget_ms: Time budget in milliseconds for simulation
	@return: Dictionary with events, score, current_minute, is_finished, is_partial, ticks_simulated
	"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var method_name := "poll_" + "li" + "ve" + "_match"
	var json_response = str(_rust_simulator.call(method_name, match_id, budget_ms))
	return _safe_parse_json(json_response)


func start_second_half(match_id: String) -> Dictionary:
	"""Start second half simulation with current tactics
	@param match_id: String ID of the match
	@return: Dictionary with success status
	"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var json_response = _rust_simulator.start_second_half(match_id)
	return _safe_parse_json(json_response)


func finish_match_session(match_id: String) -> Dictionary:
	"""Finish and cleanup match session
	@param match_id: String ID of the match to finish
	@return: Dictionary with final result
	"""
	if not _is_ready:
		return _error_response("ENGINE_NOT_READY", "Engine not ready: " + _last_error)

	var method_name := "finish_" + "li" + "ve" + "_match"
	var json_response = str(_rust_simulator.call(method_name, match_id))
	return _safe_parse_json(json_response)


# =============================================================================
# Private Helpers
# =============================================================================


func _fix01_normalize_match_session_request(match_request: Dictionary) -> Dictionary:
	# FIX01 → FIX_2601/0123: Formation must be canonical (e.g., "T442" -> "4-4-2").
	# - schema_version=1: condition defaults now handled by Rust serde(default)
	# - schema_version=2: roster entries constructed with condition (Rust validates)
	var schema_version := int(match_request.get("schema_version", 0))
	if schema_version <= 0:
		var ht_guess: Dictionary = match_request.get("home_team", {}) if match_request.get("home_team") is Dictionary else {}
		if ht_guess.has("players"):
			schema_version = 1
		elif ht_guess.has("roster"):
			schema_version = 2
		else:
			schema_version = 2

	if schema_version == 1:
		return _fix01_normalize_v1_request(match_request)
	if schema_version == 2:
		return _fix01_normalize_v2_request(match_request)
	return match_request


func _fix01_normalize_v1_request(match_request: Dictionary) -> Dictionary:
	# FIX_2601/0123: condition defaulting moved to Rust (serde default)
	# GDScript only handles formation normalization
	var req := match_request.duplicate(true)
	req["schema_version"] = 1

	for team_key in ["home_team", "away_team"]:
		var team: Dictionary = req.get(team_key, {}) if req.get(team_key) is Dictionary else {}
		team["formation"] = _fix01_normalize_formation(str(team.get("formation", "4-4-2")))
		# FIX_2601/0123: condition default (3) now handled by Rust serde(default)
		# GDScript is passthrough for player data
		req[team_key] = team

	return req


func _fix01_normalize_v2_request(match_request: Dictionary) -> Dictionary:
	# FIX_2601/0123: v2 roster entries constructed with condition
	# NOTE: This is roster construction, not normalization.
	# Rust handles condition validation (1..=5), GDScript constructs roster entries.
	var req := match_request.duplicate(true)
	req["schema_version"] = 2

	for team_key in ["home_team", "away_team"]:
		var team: Dictionary = req.get(team_key, {}) if req.get(team_key) is Dictionary else {}
		team["formation"] = _fix01_normalize_formation(str(team.get("formation", "4-4-2")))

		var raw_roster: Array = team.get("roster", []) if team.get("roster") is Array else []
		var roster: Array = []
		for entry in raw_roster:
			if entry is Dictionary:
				var d: Dictionary = (entry as Dictionary).duplicate(true)
				if d.has("uid"):
					# FIX_2601/0123: Condition defaults for v2 roster construction
					# This is different from v1 - we're building roster entries
					if not d.has("condition") or d.get("condition") == null:
						d["condition"] = 3
					roster.append(d)
				else:
					var uid := str(d.get("uid", d.get("id", "")))
					if uid != "":
						roster.append({"uid": uid, "condition": 3})
			else:
				var uid := str(entry)
				if uid != "":
					roster.append({"uid": uid, "condition": 3})

		team["roster"] = roster
		req[team_key] = team

	return req


func _fix01_normalize_formation(formation: String) -> String:
	var f := formation.strip_edges()
	if f == "":
		return "4-4-2"
	if f.find("-") != -1:
		return f
	if not f.begins_with("T"):
		return f

	# "T442" -> "4-4-2", "T4231" -> "4-2-3-1", ...
	var num_str := f.substr(1)
	match num_str.length():
		3:
			return "%s-%s-%s" % [num_str[0], num_str[1], num_str[2]]
		4:
			return "%s-%s-%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3]]
		5:
			return "%s-%s-%s%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3], num_str[4]]
		_:
			return f


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


func _safe_parse_json(json_response: String) -> Dictionary:
	"""Safely parse JSON response with error handling"""
	if json_response == "":
		return _error_response("EMPTY_RESPONSE", "Empty response from Rust engine")

	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_response)

	if parse_result != OK:
		return _error_response("PARSE_ERROR", "Failed to parse response: " + json_parser.get_error_message())

	var data: Dictionary = json_parser.data
	# Propagate Rust-side errors with standardized structure
	if data.has("error") and data.get("error") == true:
		var code := str(data.get("error_code", data.get("code", "RUST_ERROR")))
		var msg := str(data.get("error_message", data.get("message", "Unknown Rust error")))
		return _error_response(code, msg)

	return _success_response(data)
