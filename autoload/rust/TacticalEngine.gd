extends Node
## TacticalEngine - Core autoload for Rust tactical analysis
## Part of the 5-autoload architecture (Constitution v7.0)

signal tactical_analysis_complete(analysis: Dictionary)
signal counter_tactics_suggested(suggestions: Array)

var _football_engine: Node = null
var _rust_match_simulator: Object = null

const _DEFAULT_TEAM_INSTRUCTIONS := {
	"defensive_line": "Normal",
	"team_width": "Normal",
	"team_tempo": "Normal",
	"pressing_intensity": "Medium",
	"build_up_style": "Mixed",
	"use_offside_trap": false,
}


func _ready() -> void:
	# Use FootballRustEngine instead of creating our own instance
	if has_node("/root/FootballRustEngine"):
		_football_engine = get_node("/root/FootballRustEngine")
		if _football_engine.has_method("is_ready"):
			if _football_engine.is_ready():
				_initialize()
			else:
				_football_engine.engine_ready.connect(_initialize, CONNECT_ONE_SHOT)


func _initialize() -> void:
	# Session tactics should forward to the authoritative Rust match session (FootballMatchSimulator).
	# Avoid introducing a separate Rust TacticalEngine class (it doesn't exist in the current build).
	if not _football_engine:
		return
	var simulator: Variant = _football_engine.get("_rust_simulator")
	if simulator != null and simulator is Object:
		_rust_match_simulator = simulator
		print("[TacticalEngine] Using FootballMatchSimulator for session tactics")


## Update session tactical settings (Phase E stub - forwards to Rust JSON API)  
func update_session_tactics(payload: Dictionary) -> Dictionary:
	if not _football_engine or not _football_engine.has_method("is_ready") or not _football_engine.is_ready():
		return {"success": false, "error": "Engine not ready", "code": "ENGINE_NOT_READY"}

	if not _rust_match_simulator:
		var simulator: Variant = _football_engine.get("_rust_simulator")
		if simulator != null and simulator is Object:
			_rust_match_simulator = simulator

	if not _rust_match_simulator:
		return {"success": false, "error": "Rust simulator not initialized", "code": "ENGINE_NOT_READY"}

	var team := str(payload.get("team", "home")).strip_edges().to_lower()
	if team != "home" and team != "away":
		team = "home"

	var formation := str(payload.get("formation", "")).strip_edges()
	var instructions := _coerce_payload_to_team_instructions(payload)
	var instructions_json := JSON.stringify(instructions)
	if instructions_json == "":
		return {"success": false, "error": "Failed to encode TeamInstructions", "code": "SERIALIZATION_ERROR"}

	var result := {"success": true, "team": team, "applied": []}

	# Optional formation change (separate Rust API).
	if formation != "" and _rust_match_simulator.has_method("change_formation_live_match"):
		var formation_resp := _parse_json_dict(str(_rust_match_simulator.call("change_formation_live_match", team, formation)))
		result["formation_response"] = formation_resp
		if not bool(formation_resp.get("success", false)):
			result["success"] = false
			result["error"] = formation_resp.get("error", "FORMATION_FAILED")
			result["code"] = formation_resp.get("code", "FORMATION_FAILED")
			return result
		result["applied"].append("formation")

	# Tactical instructions (TeamInstructions).
	if not _rust_match_simulator.has_method("change_live_tactic"):
		return {"success": false, "error": "Rust method change_live_tactic not found", "code": "MISSING_METHOD"}

	var tactics_resp := _parse_json_dict(str(_rust_match_simulator.call("change_live_tactic", team, instructions_json)))
	result["tactics_response"] = tactics_resp
	if not bool(tactics_resp.get("success", false)):
		result["success"] = false
		result["error"] = tactics_resp.get("error", "TACTICS_FAILED")
		result["code"] = tactics_resp.get("code", "TACTICS_FAILED")
		return result

	result["applied"].append("tactics")
	return result


func _parse_json_dict(json_str: String) -> Dictionary:
	if json_str == "":
		return {"success": false, "error": "Empty response", "code": "EMPTY_RESPONSE"}
	var parser := JSON.new()
	if parser.parse(json_str) != OK:
		return {
			"success": false,
			"error": "Failed to parse JSON: %s" % parser.get_error_message(),
			"code": "PARSE_ERROR",
			"raw": json_str,
		}
	var data: Variant = parser.data
	if typeof(data) != TYPE_DICTIONARY:
		return {"success": false, "error": "Unexpected response format", "code": "INVALID_RESPONSE"}
	return data as Dictionary


func _coerce_payload_to_team_instructions(payload: Dictionary) -> Dictionary:
	# Ensure we always provide all required TeamInstructions fields.
	# Rust side does NOT default missing fields (except use_offside_trap), so we must.
	var out := _DEFAULT_TEAM_INSTRUCTIONS.duplicate(true)

	# 1) Preset mapping (Rust TacticalPreset names)
	if payload.has("preset"):
		var preset := str(payload.get("preset", "")).strip_edges()
		match preset:
			"HighPressing":
				out = {
					"defensive_line": "VeryHigh",
					"team_width": "Wide",
					"team_tempo": "VeryFast",
					"pressing_intensity": "VeryHigh",
					"build_up_style": "Short",
					"use_offside_trap": true,
				}
			"Counterattack":
				out = {
					"defensive_line": "Deep",
					"team_width": "Narrow",
					"team_tempo": "Fast",
					"pressing_intensity": "Low",
					"build_up_style": "Direct",
					"use_offside_trap": false,
				}
			"Possession":
				out = {
					"defensive_line": "High",
					"team_width": "Wide",
					"team_tempo": "Slow",
					"pressing_intensity": "Medium",
					"build_up_style": "Short",
					"use_offside_trap": true,
				}
			"Defensive":
				out = {
					"defensive_line": "VeryDeep",
					"team_width": "Narrow",
					"team_tempo": "Normal",
					"pressing_intensity": "Low",
					"build_up_style": "Direct",
					"use_offside_trap": false,
				}
			"Balanced", _:
				pass

	# 2) Explicit instructions dict (preferred)
	var instr: Dictionary = {}
	if payload.has("instructions") and payload.instructions is Dictionary:
		instr = payload.instructions

	# 3) Overlay fields (canonical + aliases + UI scalars)
	# IMPORTANT: Do not clobber preset/default values when the field is missing.
	var defensive_line_val: Variant = null
	if instr.has("defensive_line"):
		defensive_line_val = instr.get("defensive_line")
	elif payload.has("defensive_line"):
		defensive_line_val = payload.get("defensive_line")
	elif payload.has("defensive_line_height"):
		defensive_line_val = payload.get("defensive_line_height")
	if defensive_line_val != null:
		out["defensive_line"] = _map_defensive_line(defensive_line_val)

	var team_width_val: Variant = null
	if instr.has("team_width"):
		team_width_val = instr.get("team_width")
	elif instr.has("width"):
		team_width_val = instr.get("width")
	elif payload.has("team_width"):
		team_width_val = payload.get("team_width")
	elif payload.has("width"):
		team_width_val = payload.get("width")
	if team_width_val != null:
		out["team_width"] = _map_team_width(team_width_val)

	var team_tempo_val: Variant = null
	if instr.has("team_tempo"):
		team_tempo_val = instr.get("team_tempo")
	elif instr.has("tempo"):
		team_tempo_val = instr.get("tempo")
	elif payload.has("tempo"):
		team_tempo_val = payload.get("tempo")
	if team_tempo_val != null:
		out["team_tempo"] = _map_team_tempo(team_tempo_val)

	var pressing_val: Variant = null
	if instr.has("pressing_intensity"):
		pressing_val = instr.get("pressing_intensity")
	elif instr.has("pressing"):
		pressing_val = instr.get("pressing")
	elif payload.has("press_intensity"):
		pressing_val = payload.get("press_intensity")
	elif payload.has("pressing_intensity"):
		pressing_val = payload.get("pressing_intensity")
	elif payload.has("pressing"):
		pressing_val = payload.get("pressing")
	if pressing_val != null:
		out["pressing_intensity"] = _map_pressing_intensity(pressing_val)

	var build_up_val: Variant = null
	if instr.has("build_up_style"):
		build_up_val = instr.get("build_up_style")
	elif instr.has("build_up_play"):
		build_up_val = instr.get("build_up_play")
	elif payload.has("build_up_style"):
		build_up_val = payload.get("build_up_style")
	elif payload.has("build_up_play"):
		build_up_val = payload.get("build_up_play")
	if build_up_val != null:
		out["build_up_style"] = _map_build_up_style(build_up_val)

	# Optional: allow explicit override; else infer from defensive line.
	if payload.has("use_offside_trap"):
		out["use_offside_trap"] = bool(payload.get("use_offside_trap", false))
	elif instr.has("use_offside_trap"):
		out["use_offside_trap"] = bool(instr.get("use_offside_trap", false))
	else:
		var dl := str(out.get("defensive_line", "Normal"))
		out["use_offside_trap"] = dl == "High" or dl == "VeryHigh"

	return out


func _coerce_scalar01(value: Variant) -> float:
	match typeof(value):
		TYPE_FLOAT, TYPE_INT:
			var f := float(value)
			if f > 1.0:
				f = f / 100.0
			return clampf(f, 0.0, 1.0)
		TYPE_STRING:
			var s := str(value).strip_edges()
			if s.is_valid_float():
				var f := float(s)
				if f > 1.0:
					f = f / 100.0
				return clampf(f, 0.0, 1.0)
	return -1.0


func _normalize_enum_token(value: Variant) -> String:
	var s := str(value).strip_edges()
	if s == "":
		return ""
	var lower := s.to_lower()
	match lower:
		"veryfast":
			return "VeryFast"
		"fast":
			return "Fast"
		"normal", "medium":
			return "Normal"
		"slow":
			return "Slow"
		"veryslow":
			return "VerySlow"
		"veryhigh":
			return "VeryHigh"
		"high":
			return "High"
		"low":
			return "Low"
		"verylow":
			return "VeryLow"
		"verywide":
			return "VeryWide"
		"wide":
			return "Wide"
		"narrow":
			return "Narrow"
		"verynarrow":
			return "VeryNarrow"
		"verydeep":
			return "VeryDeep"
		"deep":
			return "Deep"
		"shortpassing":
			return "ShortPassing"
		"directpassing":
			return "DirectPassing"
		"short":
			return "Short"
		"mixed":
			return "Mixed"
		"direct":
			return "Direct"
		_:
			return s


func _map_team_tempo(value: Variant) -> String:
	var token := _normalize_enum_token(value)
	if token in ["VerySlow", "Slow", "Normal", "Fast", "VeryFast"]:
		return token
	var s := _coerce_scalar01(value)
	if s < 0.0:
		return "Normal"
	if s < 0.2:
		return "VerySlow"
	if s < 0.4:
		return "Slow"
	if s < 0.6:
		return "Normal"
	if s < 0.8:
		return "Fast"
	return "VeryFast"


func _map_pressing_intensity(value: Variant) -> String:
	var token := _normalize_enum_token(value)
	if token in ["VeryLow", "Low", "Medium", "High", "VeryHigh"]:
		return token
	var s := _coerce_scalar01(value)
	if s < 0.0:
		return "Medium"
	if s < 0.2:
		return "VeryLow"
	if s < 0.4:
		return "Low"
	if s < 0.6:
		return "Medium"
	if s < 0.8:
		return "High"
	return "VeryHigh"


func _map_team_width(value: Variant) -> String:
	var token := _normalize_enum_token(value)
	if token in ["VeryNarrow", "Narrow", "Normal", "Wide", "VeryWide", "Medium"]:
		return "Normal" if token == "Medium" else token
	var s := _coerce_scalar01(value)
	if s < 0.0:
		return "Normal"
	if s < 0.2:
		return "VeryNarrow"
	if s < 0.4:
		return "Narrow"
	if s < 0.6:
		return "Normal"
	if s < 0.8:
		return "Wide"
	return "VeryWide"


func _map_defensive_line(value: Variant) -> String:
	var token := _normalize_enum_token(value)
	match token:
		"VeryLow":
			return "VeryDeep"
		"Low":
			return "Deep"
		"Medium":
			return "Normal"
		"VeryDeep", "Deep", "Normal", "High", "VeryHigh":
			return token

	var s := _coerce_scalar01(value)
	if s < 0.0:
		return "Normal"
	if s < 0.2:
		return "VeryDeep"
	if s < 0.4:
		return "Deep"
	if s < 0.6:
		return "Normal"
	if s < 0.8:
		return "High"
	return "VeryHigh"


func _map_build_up_style(value: Variant) -> String:
	var token := _normalize_enum_token(value)
	match token:
		"ShortPassing", "Short":
			return "Short"
		"DirectPassing", "Direct":
			return "Direct"
		"Mixed":
			return "Mixed"
	return "Mixed"


## Analyze team tactics
## @param team_data: Dictionary containing team information
## @return: Dictionary with tactical analysis
func analyze_team_tactics(team_data: Dictionary) -> Dictionary:
	if not _football_engine:
		return {"error": true, "message": "Tactical engine not initialized"}

	# Basic tactical analysis
	var formation = team_data.get("formation", "4-4-2")
	var players = team_data.get("players", [])
	var instructions = team_data.get("instructions", {})

	var analysis = {
		"formation": formation,
		"style": _determine_playing_style(formation, players),
		"strengths": _identify_strengths(formation, players),
		"weaknesses": _identify_weaknesses(formation, players),
		"key_players": _identify_key_players(players),
		"tactical_flexibility": _calculate_flexibility(formation, players),
		"recommended_strategy": _recommend_strategy(formation, players),
		# Initialize empty modifiers
		"event_modifiers": [],
		"stamina_drain": 0,
		"modifier_summary": ""
	}

	# Get tactical modifiers from Rust if instructions are provided
	if not instructions.is_empty() and _football_engine.has_method("get_tactical_modifiers"):
		var modifiers_json = _football_engine.get_tactical_modifiers(JSON.stringify(instructions))

		var parser := JSON.new()
		if parser.parse(modifiers_json) == OK and parser.data.get("success", false):
			var modifiers_result = parser.data
			analysis["event_modifiers"] = modifiers_result.get("modifiers", [])
			analysis["stamina_drain"] = modifiers_result.get("stamina_drain_percent", 0)
			analysis["modifier_summary"] = modifiers_result.get("summary", "")

	tactical_analysis_complete.emit(analysis)
	return analysis


## Generate counter tactics for opponent
## @param opponent_tactics: Dictionary with opponent's tactical setup
## @return: Array of counter-tactic suggestions
func generate_counter_tactics(opponent_tactics: Dictionary) -> Array:
	# Try Rust API first
	if _football_engine and _football_engine.has_method("analyze_opponent_tactics"):
		var opponent_instructions = opponent_tactics.get("instructions", {})
		if not opponent_instructions.is_empty():
			var analysis_json = _football_engine.analyze_opponent_tactics(JSON.stringify(opponent_instructions))

			var parser := JSON.new()
			if parser.parse(analysis_json) == OK and parser.data.get("success", false):
				var analysis = parser.data
				var suggestions = _convert_analysis_to_suggestions(analysis)
				counter_tactics_suggested.emit(suggestions)
				return suggestions

	# Fallback to legacy logic
	return _generate_legacy_counter_tactics(opponent_tactics)


## Convert Rust analysis to suggestion format
func _convert_analysis_to_suggestions(analysis: Dictionary) -> Array:
	var suggestions = []

	# Add weakness-based suggestions
	for weakness in analysis.get("weaknesses", []):
		var suggestion = {
			"type": "weakness_exploit",
			"weakness": weakness.get("type", ""),
			"description": weakness.get("description_ko", ""),
			"method": weakness.get("exploit_method", ""),
			"effectiveness": 0.75
		}
		suggestions.append(suggestion)

	# Add counter tactic recommendation
	var counter = analysis.get("recommended_counter", {})
	if not counter.is_empty():
		suggestions.append(
			{
				"type": "counter_tactic",
				"style": counter.get("style", "Mixed"),
				"tempo": counter.get("tempo", "Normal"),
				"key_areas": counter.get("key_areas", []),
				"description": counter.get("description_ko", ""),
				"effectiveness": 0.80
			}
		)

	return suggestions


## Legacy counter tactics generation (fallback)
func _generate_legacy_counter_tactics(opponent_tactics: Dictionary) -> Array:
	var opponent_formation = opponent_tactics.get("formation", "4-4-2")
	var opponent_style = opponent_tactics.get("style", "balanced")

	var suggestions = []

	# Counter-formation suggestions
	match opponent_formation:
		"4-4-2":
			suggestions.append(
				{"formation": "4-3-3", "reasoning": "Overload midfield and exploit wide areas", "effectiveness": 0.75}
			)
			suggestions.append(
				{"formation": "3-5-2", "reasoning": "Dominate midfield with numbers", "effectiveness": 0.70}
			)
		"4-3-3":
			suggestions.append(
				{"formation": "4-5-1", "reasoning": "Compact midfield to deny space", "effectiveness": 0.72}
			)
			suggestions.append(
				{"formation": "5-3-2", "reasoning": "Defensive solidity against wide threats", "effectiveness": 0.68}
			)
		"3-5-2":
			suggestions.append(
				{"formation": "4-3-3", "reasoning": "Exploit space behind wing-backs", "effectiveness": 0.76}
			)
			suggestions.append(
				{
					"formation": "4-2-3-1",
					"reasoning": "Width in attack, double pivot for stability",
					"effectiveness": 0.74
				}
			)
		_:
			suggestions.append(
				{
					"formation": "4-4-2",
					"reasoning": "Balanced approach against unknown formation",
					"effectiveness": 0.60
				}
			)

	# Tactical adjustments
	match opponent_style:
		"attacking":
			suggestions.append(
				{
					"adjustment": "Drop defensive line",
					"reasoning": "Absorb pressure and counter-attack",
					"effectiveness": 0.70
				}
			)
		"defensive":
			suggestions.append(
				{"adjustment": "High pressing", "reasoning": "Force errors in opponent's half", "effectiveness": 0.72}
			)
		"possession":
			suggestions.append(
				{"adjustment": "Compact shape", "reasoning": "Deny space between lines", "effectiveness": 0.68}
			)
		"counter-attacking":
			suggestions.append(
				{
					"adjustment": "Control possession",
					"reasoning": "Deny counter-attacking opportunities",
					"effectiveness": 0.71
				}
			)

	counter_tactics_suggested.emit(suggestions)
	return suggestions


## Calculate match prediction based on tactics
## @param home_tactics: Dictionary
## @param away_tactics: Dictionary
## @return: Dictionary with match prediction
func predict_match_outcome(home_tactics: Dictionary, away_tactics: Dictionary) -> Dictionary:
	var home_strength = _calculate_team_strength(home_tactics)
	var away_strength = _calculate_team_strength(away_tactics)

	# Apply Rust counter bonus
	var home_bonus = 1.0
	var away_bonus = 1.0

	if _football_engine and _football_engine.has_method("calculate_counter_bonus"):
		var home_instructions = home_tactics.get("instructions", {})
		var away_instructions = away_tactics.get("instructions", {})

		if not home_instructions.is_empty() and not away_instructions.is_empty():
			# Home team counter bonus
			var home_bonus_json = _football_engine.calculate_counter_bonus(
				JSON.stringify(home_instructions), JSON.stringify(away_instructions)
			)
			var parser := JSON.new()
			if parser.parse(home_bonus_json) == OK and parser.data.get("success", false):
				home_bonus = parser.data.get("bonus", 1.0)

			# Away team counter bonus
			var away_bonus_json = _football_engine.calculate_counter_bonus(
				JSON.stringify(away_instructions), JSON.stringify(home_instructions)
			)
			if parser.parse(away_bonus_json) == OK and parser.data.get("success", false):
				away_bonus = parser.data.get("bonus", 1.0)

	home_strength *= home_bonus
	away_strength *= away_bonus

	# Home advantage
	var home_advantage = 0.1
	home_strength *= (1.0 + home_advantage)

	var total_strength = home_strength + away_strength
	var home_win_prob = home_strength / total_strength
	var away_win_prob = away_strength / total_strength
	var draw_prob = 0.25

	# Adjust probabilities for close matches
	if abs(home_strength - away_strength) < 0.1:
		draw_prob = 0.35
		home_win_prob = (1.0 - draw_prob) * home_win_prob
		away_win_prob = (1.0 - draw_prob) * away_win_prob

	return {
		"home_win_probability": home_win_prob,
		"draw_probability": draw_prob,
		"away_win_probability": away_win_prob,
		"predicted_score": {"home": int(home_strength * 3), "away": int(away_strength * 3)},
		"confidence": _calculate_confidence(home_tactics, away_tactics),
		"tactical_advantage": {"home_bonus": home_bonus, "away_bonus": away_bonus}
	}


## Get tactical instructions for a specific match situation
## @param situation: String describing the match situation
## @param _current_tactics: Dictionary with current setup (reserved for future context-aware tactics)
## @return: Array of tactical instructions
func get_situational_instructions(situation: String, _current_tactics: Dictionary) -> Array:
	var instructions = []

	match situation:
		"winning_late":
			instructions = [
				{"type": "mentality", "value": "defensive", "priority": "high"},
				{"type": "tempo", "value": "slow", "priority": "medium"},
				{"type": "width", "value": "narrow", "priority": "low"},
				{"type": "substitution", "value": "defensive midfielder for attacker", "priority": "high"}
			]
		"losing_late":
			instructions = [
				{"type": "mentality", "value": "ultra-attacking", "priority": "high"},
				{"type": "tempo", "value": "high", "priority": "high"},
				{"type": "width", "value": "wide", "priority": "medium"},
				{"type": "substitution", "value": "attacker for defensive midfielder", "priority": "high"}
			]
		"drawing_need_win":
			instructions = [
				{"type": "mentality", "value": "attacking", "priority": "high"},
				{"type": "risk", "value": "high", "priority": "medium"},
				{"type": "pressing", "value": "high", "priority": "medium"}
			]
		"red_card_against":
			instructions = [
				{"type": "formation_change", "value": "more attacking", "priority": "high"},
				{"type": "focus", "value": "exploit numerical advantage", "priority": "high"},
				{"type": "width", "value": "wide", "priority": "medium"}
			]
		"red_card_for":
			instructions = [
				{"type": "formation_change", "value": "more compact", "priority": "high"},
				{"type": "mentality", "value": "cautious", "priority": "medium"},
				{"type": "focus", "value": "maintain shape", "priority": "high"}
			]
		_:
			instructions = [{"type": "maintain", "value": "current tactics", "priority": "low"}]

	return instructions


# Helper functions


func _determine_playing_style(formation: String, _players: Array) -> String:
	# Analyze formation and player attributes to determine style
	# TODO: Use _players data for more sophisticated style determination
	match formation:
		"4-3-3", "3-4-3":
			return "attacking"
		"5-3-2", "5-4-1":
			return "defensive"
		"4-2-3-1", "4-1-4-1":
			return "possession"
		"4-4-2":
			return "balanced"
		"3-5-2", "3-4-1-2":
			return "wing-play"
		_:
			return "flexible"


func _identify_strengths(formation: String, players: Array) -> Array:
	var strengths = []

	# Formation-based strengths
	if "5" in formation.split("-")[0]:
		strengths.append("Strong defensive structure")
	if "5" in formation.split("-")[1]:
		strengths.append("Midfield dominance")
	if "3" in formation.split("-")[2]:
		strengths.append("Multiple attacking options")

	# Player-based strengths
	var high_rated_count = 0
	for player in players:
		if player.get("overall", 0) > 80:
			high_rated_count += 1

	if high_rated_count > 5:
		strengths.append("High quality squad")

	return strengths


func _identify_weaknesses(formation: String, players: Array) -> Array:
	var weaknesses = []

	# Formation-based weaknesses
	if "3" in formation.split("-")[0]:
		weaknesses.append("Vulnerable to wide attacks")
	if "2" in formation.split("-")[1]:
		weaknesses.append("Potential midfield overload")
	if "1" in formation.split("-")[2]:
		weaknesses.append("Isolated striker")

	# Player-based weaknesses
	var low_rated_count = 0
	for player in players:
		if player.get("overall", 0) < 65:
			low_rated_count += 1

	if low_rated_count > 3:
		weaknesses.append("Weak bench depth")

	return weaknesses


func _identify_key_players(players: Array) -> Array:
	var key_players = []

	# Sort by overall rating
	var sorted_players = players.duplicate()
	sorted_players.sort_custom(func(a, b): return a.get("overall", 0) > b.get("overall", 0))

	# Take top 3 players
	for i in min(3, sorted_players.size()):
		key_players.append(
			{
				"name": sorted_players[i].get("name", "Unknown"),
				"position": sorted_players[i].get("position", "?"),
				"overall": sorted_players[i].get("overall", 0),
				"importance": "high" if i == 0 else "medium"
			}
		)

	return key_players


func _calculate_flexibility(formation: String, players: Array) -> float:
	# Calculate how flexible the tactical setup is
	var flexibility = 0.5  # Base flexibility

	# Players who can play multiple positions increase flexibility
	var versatile_players = 0
	for player in players:
		if player.has("alternative_positions") and player.alternative_positions.size() > 1:
			versatile_players += 1

	flexibility += versatile_players * 0.05

	# Certain formations are more flexible
	if formation in ["4-3-3", "4-2-3-1", "3-5-2"]:
		flexibility += 0.1

	return clamp(flexibility, 0.0, 1.0)


func _recommend_strategy(_formation: String, players: Array) -> String:
	# TODO: Use _formation data for formation-specific strategy recommendations
	var avg_overall = 0.0
	for player in players:
		avg_overall += player.get("overall", 50)
	avg_overall /= float(players.size()) if players.size() > 0 else 1.0

	if avg_overall > 75:
		return "Dominate possession and control the game"
	elif avg_overall > 65:
		return "Balanced approach with tactical flexibility"
	else:
		return "Compact defense and quick counter-attacks"


func _calculate_team_strength(tactics: Dictionary) -> float:
	var base_strength = 0.5

	# Add player quality
	var players = tactics.get("players", [])
	var total_overall = 0
	for player in players:
		total_overall += player.get("overall", 50)

	if players.size() > 0:
		base_strength = float(total_overall) / float(players.size()) / 100.0

	# Tactical bonuses
	if tactics.has("style") and tactics.style in ["attacking", "possession"]:
		base_strength *= 1.05

	return clamp(base_strength, 0.1, 1.0)


func _calculate_confidence(home_tactics: Dictionary, away_tactics: Dictionary) -> float:
	# Calculate confidence in the prediction
	var confidence = 0.5

	# More data = more confidence
	if home_tactics.has("players") and home_tactics.players.size() >= 11:
		confidence += 0.1
	if away_tactics.has("players") and away_tactics.players.size() >= 11:
		confidence += 0.1

	# Clear strength difference = more confidence
	var home_strength = _calculate_team_strength(home_tactics)
	var away_strength = _calculate_team_strength(away_tactics)
	var strength_diff = abs(home_strength - away_strength)

	if strength_diff > 0.2:
		confidence += 0.2
	elif strength_diff > 0.1:
		confidence += 0.1

	return clamp(confidence, 0.0, 1.0)
