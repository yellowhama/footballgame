class_name TacticsBridge
extends RefCounted
## ============================================================================
## TacticsBridge - Tactics, Formation, and Instructions API
## ============================================================================
##
## PURPOSE: Bridge for tactical systems via Rust engine
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Player Instructions API (roles, custom instructions)
## - Formation API (list, details, recommendations)
## - Team Instructions API (presets, custom tactics)
##
## DEPENDENCIES:
## - _rust_simulator: GDExtension Rust object
##
## USAGE:
##   var bridge := TacticsBridge.new()
##   bridge.initialize(rust_simulator)
##   var formations := bridge.get_all_formations()
## ============================================================================

var _rust_simulator: Object = null
var _is_ready: bool = false
var _last_error: String = ""


func initialize(rust_simulator: Object) -> void:
	"""Initialize TacticsBridge with Rust simulator reference"""
	_rust_simulator = rust_simulator
	_is_ready = rust_simulator != null
	if not _is_ready:
		_last_error = "Rust simulator not provided"


# =============================================================================
# Player Instructions API
# =============================================================================

func get_available_roles(position: String = "") -> Dictionary:
	"""Get available player roles (optionally filtered by position)
	@param position: String - Optional position filter (e.g., "ST", "CM", "CB"). Empty for all roles.
	@return: Dictionary with success, roles array, total_roles
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var result_json = _rust_simulator.get_available_roles(position)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse roles response"}


func get_instruction_options() -> Dictionary:
	"""Get instruction option values for all categories
	@return: Dictionary with success, instruction_options, korean_labels
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready"}

	var result_json = _rust_simulator.get_instruction_options()
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse instruction options"}


func set_player_role(player_data: Dictionary, role_name: String) -> Dictionary:
	"""Set player role (applies preset instructions)
	@param player_data: Dictionary - Player data
	@param role_name: String - Role name (e.g., "TargetMan", "Playmaker")
	@return: Dictionary with success, player (updated), role_applied
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready"}

	var player_json = JSON.stringify(player_data)
	var result_json = _rust_simulator.set_player_role(player_json, role_name)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse role response"}


func set_player_instructions(player_data: Dictionary, instructions: Dictionary) -> Dictionary:
	"""Set custom player instructions
	@param player_data: Dictionary - Player data
	@param instructions: Dictionary - Custom instructions with 8 categories
	@return: Dictionary with success, player (updated)
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready"}

	var player_json = JSON.stringify(player_data)
	var instructions_json = JSON.stringify(instructions)
	var result_json = _rust_simulator.set_player_instructions(player_json, instructions_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse instructions response"}


func get_player_modified_attributes(player_data: Dictionary) -> Dictionary:
	"""Get player's modified attributes (with instructions applied)
	@param player_data: Dictionary - Player data
	@return: Dictionary with success, has_instructions, base_attributes, modified_attributes
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready"}

	var player_json = JSON.stringify(player_data)
	var result_json = _rust_simulator.get_player_modified_attributes(player_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse attributes response"}


func clear_player_instructions(player_data: Dictionary) -> Dictionary:
	"""Clear player instructions (revert to no instructions)
	@param player_data: Dictionary - Player data
	@return: Dictionary with success, player (updated with instructions cleared)
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready"}

	var player_json = JSON.stringify(player_data)
	var result_json = _rust_simulator.clear_player_instructions(player_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse clear response"}


# =============================================================================
# Formation API
# =============================================================================

func get_all_formations() -> Dictionary:
	"""Get all available formations (14 formations from OpenFootball)
	@return: Dictionary with success, total_formations, formations array
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("get_all_formations"):
		push_error("[TacticsBridge] Formation API method not found in Rust extension")
		return {"success": false, "error": "Formation API method not available"}

	var result_json = _rust_simulator.get_all_formations()
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse formations response"}


func get_formation_details(formation_id: String) -> Dictionary:
	"""Get detailed information about a specific formation
	@param formation_id: String - Formation ID (e.g., "T442", "T433", "T4231")
	@return: Dictionary with success, formation details (positions, coordinates, strengths, weaknesses)
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("get_formation_details"):
		push_error("[TacticsBridge] Formation details method not found")
		return {"success": false, "error": "Formation details method not available"}

	var result_json = _rust_simulator.get_formation_details(formation_id)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse formation details response"}


func recommend_formations(players: Array) -> Dictionary:
	"""Recommend formations based on player squad composition
	@param players: Array of player dictionaries with position and CA data
	@return: Dictionary with success, recommendations array, squad_analysis
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("recommend_formations"):
		push_error("[TacticsBridge] Formation recommendation method not found")
		return {"success": false, "error": "Formation recommendation method not available"}

	var players_json = JSON.stringify(players)
	var result_json = _rust_simulator.recommend_formations(players_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse recommendations response"}


func calculate_formation_fitness(formation_id: String, players: Array) -> Dictionary:
	"""Calculate how well a formation fits your squad
	@param formation_id: String - Formation ID (e.g., "T442")
	@param players: Array of player dictionaries
	@return: Dictionary with success, fitness_score (0.0-1.0), details
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("calculate_formation_fitness"):
		push_error("[TacticsBridge] Formation fitness method not found")
		return {"success": false, "error": "Formation fitness method not available"}

	var players_json = JSON.stringify(players)
	var result_json = _rust_simulator.calculate_formation_fitness(formation_id, players_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse fitness response"}


func suggest_counter_formation(opponent_formation_id: String, our_players: Array) -> Dictionary:
	"""Suggest counter formations based on opponent's formation
	@param opponent_formation_id: String - Opponent's formation ID
	@param our_players: Array of our player dictionaries
	@return: Dictionary with success, opponent_formation, counter_formations array
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("suggest_counter_formation"):
		push_error("[TacticsBridge] Counter formation method not found")
		return {"success": false, "error": "Counter formation method not available"}

	var players_json = JSON.stringify(our_players)
	var result_json = _rust_simulator.suggest_counter_formation(opponent_formation_id, players_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse counter formation response"}


func suggest_situational_formation(current_formation_id: String, match_state: Dictionary) -> Dictionary:
	"""Suggest situational formation changes during a match
	@param current_formation_id: String - Current formation ID
	@param match_state: Dictionary with minute, score_diff, possession, team_stamina
	@return: Dictionary with success, current_formation, match_situation, suggestions array
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("suggest_situational_formation"):
		push_error("[TacticsBridge] Situational formation method not found")
		return {"success": false, "error": "Situational formation method not available"}

	var match_state_json = JSON.stringify(match_state)
	var result_json = _rust_simulator.suggest_situational_formation(current_formation_id, match_state_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse situational formation response"}


# =============================================================================
# Team Instructions API
# =============================================================================

func get_team_instruction_options() -> Dictionary:
	"""Get team instruction option values for all categories
	@return: Dictionary with success, instruction_options, korean_labels, korean_values
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("get_team_instruction_options"):
		push_error("[TacticsBridge] Team instruction options method not found")
		return {"success": false, "error": "Team instruction options method not available"}

	var result_json = _rust_simulator.get_team_instruction_options()
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse team instruction options response"}


func get_tactical_presets() -> Dictionary:
	"""Get all tactical presets with their configurations
	@return: Dictionary with success, presets array (5 presets), total_presets
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("get_tactical_presets"):
		push_error("[TacticsBridge] Tactical presets method not found")
		return {"success": false, "error": "Tactical presets method not available"}

	var result_json = _rust_simulator.get_tactical_presets()
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse tactical presets response"}


func set_team_instructions_custom(instructions: Dictionary) -> Dictionary:
	"""Set custom team instructions
	@param instructions: Dictionary with defensive_line, team_width, team_tempo, pressing_intensity, build_up_style
	@return: Dictionary with success, instructions, modifiers, description_ko
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("set_team_instructions_custom"):
		push_error("[TacticsBridge] Custom team instructions method not found")
		return {"success": false, "error": "Custom team instructions method not available"}

	var instructions_json = JSON.stringify(instructions)
	var result_json = _rust_simulator.set_team_instructions_custom(instructions_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse team instructions response"}


func set_team_instructions_preset(preset_name: String) -> Dictionary:
	"""Set team instructions using a tactical preset
	@param preset_name: String - One of: "HighPressing", "Counterattack", "Possession", "Balanced", "Defensive"
	@return: Dictionary with success, preset, preset_name_ko, preset_description_ko, instructions, modifiers, description_ko
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	if not _rust_simulator.has_method("set_team_instructions_preset"):
		push_error("[TacticsBridge] Preset team instructions method not found")
		return {"success": false, "error": "Preset team instructions method not available"}

	var result_json = _rust_simulator.set_team_instructions_preset(preset_name)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse tactical preset response"}
