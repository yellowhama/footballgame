extends RefCounted
## NOTE: This is a helper RefCounted used by MatchSimulationManager.
## Do NOT declare `class_name TacticsManager` here because the project also has an autoload singleton named `TacticsManager`.
## ============================================================================
## TacticsManager - Formation and Tactical Instructions Management
## ============================================================================
##
## PURPOSE: Manage player formations and tactical instructions for matches
##
## EXTRACTED FROM: MatchSimulationManager.gd (ST-005 God Class refactoring)
##
## RESPONSIBILITIES:
## - Load and cache formations from Rust engine
## - Validate and set player formations
## - Manage tactical instructions (attacking intensity, pressing, tempo, etc.)
## - Provide AI recommendations based on opponent strength
## - Convert between display format (4-4-2) and engine format (T442)
##
## DEPENDENCIES:
## - FootballRustEngine (autoload): For loading formations
## - MyTeamData (autoload): For team tactics integration
##
## USAGE:
##   var tactics := TacticsManager.new()
##   tactics.initialize(rust_engine_node)
##   tactics.set_formation("4-3-3")
##   tactics.set_instructions({"pressing": 70, "tempo": 60})
## ============================================================================

signal formation_changed(formation: String)
signal instructions_changed(instructions: Dictionary)

## Default tactical instructions
const DEFAULT_INSTRUCTIONS = {
	"attacking_intensity": 50,
	"defensive_line": 50,
	"pressing": 50,
	"tempo": 50,
	"width": 50,
	"passing_style": 50
}

## Valid instruction parameter names
const VALID_PARAMS = [
	"attacking_intensity",
	"defensive_line",
	"pressing",
	"tempo",
	"width",
	"passing_style"
]

# =============================================================================
# State
# =============================================================================

var player_formation: String = "4-4-2"
var player_instructions: Dictionary = {}
var _player_instructions_source: String = ""

## Engine formation cache
var _engine_formations: Array = []
var _formation_map: Dictionary = {}  ## "4-4-2" ↔ "T442" bidirectional mapping
var _engine_available: bool = false

## Reference to external nodes (set via initialize)
var _rust_engine: Node = null
var _my_team_data: Node = null


# =============================================================================
# Initialization
# =============================================================================

func initialize(rust_engine: Node = null, my_team_data: Node = null) -> void:
	"""Initialize TacticsManager with external dependencies"""
	_rust_engine = rust_engine
	_my_team_data = my_team_data
	_load_engine_formations()
	player_instructions = DEFAULT_INSTRUCTIONS.duplicate()


func _load_engine_formations() -> void:
	"""Load formations from OpenFootball engine"""
	if not _rust_engine:
		push_warning("[TacticsManager] Engine not available, using fallback formations")
		_engine_available = false
		return

	var formations_response: Dictionary = _rust_engine.get_all_formations()

	if formations_response.has("formations"):
		_engine_formations = formations_response.formations
	else:
		_engine_formations = []

	if _engine_formations.size() == 0:
		push_warning("[TacticsManager] Engine returned no formations, using fallback")
		_engine_available = false
		return

	# Build bidirectional mapping
	for formation in _engine_formations:
		var display_name: String = str(formation.get("display_name", ""))
		var engine_id: String = str(formation.get("id", ""))
		if display_name and engine_id:
			_formation_map[display_name] = engine_id
			_formation_map[engine_id] = display_name

	_engine_available = true
	print("[TacticsManager] Loaded %d formations from engine" % _engine_formations.size())


# =============================================================================
# Public API - Formation
# =============================================================================

func set_formation(formation: String) -> bool:
	"""Set player formation for match

	Args:
		formation: Formation string (e.g., "4-4-2", "4-3-3")

	Returns:
		bool: True if valid formation set, False otherwise
	"""
	var available = get_available_formations()

	if not formation in available:
		push_error("[TacticsManager] Invalid formation: %s" % formation)
		push_error("  Available: %s" % str(available))
		return false

	player_formation = formation

	var info = get_formation_info(formation)
	if info.size() > 0:
		print("[TacticsManager] Formation set: %s" % formation)
		print(
			"  %s (Attack: %d, Defense: %d)"
			% [info.get("description", ""), info.get("attacking", 50), info.get("defensive", 50)]
		)
	else:
		print("[TacticsManager] Formation set: %s" % formation)

	formation_changed.emit(formation)
	return true


func get_available_formations() -> Array:
	"""Get list of available formations from OpenFootball engine

	Returns:
		Array of formation strings (display format: "4-4-2", "4-3-3", etc.)
	"""
	if not _engine_available:
		push_error("[TacticsManager] Engine not available - formations require Rust GDExtension")
		return []
	if _engine_formations.size() == 0:
		push_error("[TacticsManager] No formations loaded from engine")
		return []

	var formations = []
	for formation in _engine_formations:
		var display_name = formation.get("display_name", "")
		if display_name:
			formations.append(display_name)
	return formations


func get_formation_info(formation: String) -> Dictionary:
	"""Get detailed information about a formation from engine

	Args:
		formation: Formation string (e.g., "4-4-2")

	Returns:
		Dictionary with formation characteristics from engine
	"""
	if not _engine_available:
		push_error("[TacticsManager] Engine not available - formation info requires Rust GDExtension")
		return {}
	if _engine_formations.size() == 0:
		push_error("[TacticsManager] No formations loaded from engine")
		return {}

	for f in _engine_formations:
		if f.get("display_name", "") == formation:
			return {
				"name": formation,
				"attacking": f.get("attacking", 50),
				"defensive": f.get("defensive", 50),
				"description": f.get("description", ""),
				"roles": f.get("roles", [])
			}
	push_error("[TacticsManager] Unknown formation: %s" % formation)
	return {}


func get_recommended_formation(opponent_ca: int, player_ca: int) -> String:
	"""Get AI-recommended formation based on opponent strength

	Args:
		opponent_ca: Opponent current ability
		player_ca: Player current ability

	Returns:
		Recommended formation string
	"""
	var ca_diff = player_ca - opponent_ca

	if ca_diff > 20:
		return "4-3-3"  # Much stronger -> Attacking
	elif ca_diff > 10:
		return "4-2-3-1"  # Stronger -> Balanced attacking
	elif ca_diff > -10:
		return "4-4-2"  # Even -> Balanced
	elif ca_diff > -20:
		return "4-5-1"  # Weaker -> Defensive balanced
	else:
		return "5-3-2"  # Much weaker -> Defensive


# =============================================================================
# Public API - Instructions
# =============================================================================

func set_instructions(instructions: Dictionary) -> bool:
	"""Set tactical instructions for match

	Args:
		instructions: Dictionary with tactical parameters (0-100)
			- attacking_intensity, defensive_line, pressing
			- tempo, width, passing_style

	Returns:
		bool: True if valid instructions set, False otherwise
	"""
	for param in instructions:
		if not param in VALID_PARAMS:
			push_warning("[TacticsManager] Unknown tactical parameter: %s" % param)
			continue

		var value = instructions[param]
		if typeof(value) != TYPE_INT and typeof(value) != TYPE_FLOAT:
			push_error("[TacticsManager] Invalid value type for %s: %s" % [param, typeof(value)])
			return false

		if value < 0 or value > 100:
			push_error("[TacticsManager] Parameter %s out of range (0-100): %d" % [param, value])
			return false

	# Merge with defaults
	player_instructions = DEFAULT_INSTRUCTIONS.duplicate()
	for param in instructions:
		if param in VALID_PARAMS:
			player_instructions[param] = instructions[param]
	_player_instructions_source = "manual"

	print("[TacticsManager] Tactical instructions updated:")
	print(
		"  Attacking: %d | Defensive Line: %d"
		% [player_instructions.attacking_intensity, player_instructions.defensive_line]
	)
	print("  Pressing: %d | Tempo: %d" % [player_instructions.pressing, player_instructions.tempo])

	instructions_changed.emit(player_instructions)
	return true


func get_recommended_instructions(formation: String, opponent_ca: int, player_ca: int) -> Dictionary:
	"""Get AI-recommended tactical instructions

	Args:
		formation: Current formation
		opponent_ca: Opponent current ability
		player_ca: Player current ability

	Returns:
		Dictionary with recommended tactical parameters
	"""
	var ca_diff = player_ca - opponent_ca
	var instructions = DEFAULT_INSTRUCTIONS.duplicate()

	# Base instructions on formation characteristics
	var formation_info = get_formation_info(formation)
	if formation_info.size() > 0:
		instructions.attacking_intensity = formation_info.get("attacking", 50)
		instructions.defensive_line = 100 - formation_info.get("defensive", 50)

	# Adjust based on relative strength
	if ca_diff > 20:
		instructions.pressing = 70
		instructions.tempo = 70
	elif ca_diff > 10:
		instructions.pressing = 60
		instructions.tempo = 60
	elif ca_diff > -10:
		instructions.pressing = 50
		instructions.tempo = 50
	elif ca_diff > -20:
		instructions.pressing = 40
		instructions.tempo = 45
	else:
		instructions.pressing = 30
		instructions.tempo = 40

	return instructions


func reset_to_default() -> void:
	"""Reset formation and instructions to defaults"""
	player_formation = "4-4-2"
	player_instructions = DEFAULT_INSTRUCTIONS.duplicate()
	_player_instructions_source = "default"
	print("[TacticsManager] Tactics reset to default (4-4-2)")
	formation_changed.emit(player_formation)
	instructions_changed.emit(player_instructions)


# =============================================================================
# Team Tactics Integration
# =============================================================================

func ensure_instructions_from_team_tactics() -> void:
	"""Apply team tactics as default instructions if not manually set"""
	if not player_instructions.is_empty() and _player_instructions_source != "team_tactics":
		return
	var derived := _build_instructions_from_team_tactics()
	if derived.is_empty():
		return
	player_instructions = derived
	_player_instructions_source = "team_tactics"
	print("[TacticsManager] Applied team tactics as default instructions")


func _build_instructions_from_team_tactics() -> Dictionary:
	if not _my_team_data or not _my_team_data.has_method("get_team_tactics"):
		return {}
	var tactics: Variant = _my_team_data.get_team_tactics()
	if tactics is Dictionary:
		var params: Variant = (tactics as Dictionary).get("parameters", {})
		if params is Dictionary and not (params as Dictionary).is_empty():
			var p := params as Dictionary
			var instructions := DEFAULT_INSTRUCTIONS.duplicate()
			instructions["attacking_intensity"] = int(
				round(clampf(float(p.get("attacking_intensity", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["defensive_line"] = int(
				round(clampf(float(p.get("defensive_line_height", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["pressing"] = int(
				round(clampf(float(p.get("pressing_trigger", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["tempo"] = int(
				round(clampf(float(p.get("tempo", 0.6)) * 100.0, 0.0, 100.0))
			)
			instructions["width"] = int(
				round(clampf(float(p.get("width", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["passing_style"] = int(
				round(clampf(float(p.get("directness", 0.5)) * 100.0, 0.0, 100.0))
			)
			return instructions
	return {}


# =============================================================================
# Format Conversion Utilities
# =============================================================================

func convert_to_engine_format(formation: String) -> String:
	"""Convert display format to engine format: '4-4-2' → 'T442'"""
	return "T" + formation.replace("-", "")


func convert_from_engine_format(formation_id: String) -> String:
	"""Convert engine format to display format: 'T442' → '4-4-2'"""
	if not formation_id.begins_with("T"):
		return formation_id

	var num_str = formation_id.substr(1)  # Remove "T"
	match num_str.length():
		3:
			return num_str[0] + "-" + num_str[1] + "-" + num_str[2]
		4:
			return num_str[0] + "-" + num_str[1] + "-" + num_str[2] + "-" + num_str[3]
		5:
			return num_str[0] + "-" + num_str[1] + "-" + num_str[2] + num_str[3] + "-" + num_str[4]
		_:
			return formation_id


func normalize_stage_formation_to_display(formation_id: String) -> String:
	"""Normalize StageManager formation IDs (e.g. 'T451') into display strings (e.g. '4-5-1')"""
	var raw := formation_id.strip_edges()
	if raw == "":
		return "4-4-2"
	if raw.find("-") != -1:
		return raw
	return convert_from_engine_format(raw)


# =============================================================================
# Getters for external access
# =============================================================================

func is_engine_available() -> bool:
	return _engine_available


func get_formation_map() -> Dictionary:
	return _formation_map.duplicate()


func get_instructions_source() -> String:
	return _player_instructions_source
