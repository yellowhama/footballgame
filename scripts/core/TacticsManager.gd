extends Node
# Removed class_name to avoid autoload conflict
## TacticsManager - Tactical System Integration Singleton
## Manages formations, tactical calculations, and integrates with Rust GDExtension

# Preload to avoid autoload order issues with class_name
const _TacticalPresetDB = preload("res://scripts/resources/TacticalPresetDB.gd")

# Rust GDExtension references (autoloads without class_name)
var formation_manager: Node
var tactical_engine: Node

# Note: Using preloaded const instead of class_name due to autoload order
var preset_db: _TacticalPresetDB
var tactical_modifier_provider: Node = null
const TACTICAL_DECK_CATEGORY = "tactical"
var current_home_preset: String = ""
var current_away_preset: String = ""
var preset_adjustments: Dictionary = {"home": {}, "away": {}}
var last_applied_parameters: Dictionary = {"home": {}, "away": {}}

# Formation cache
var formations_cache: Dictionary = {}  # formation_id -> Formation data
var current_home_formation: String = "4-4-2"
var current_away_formation: String = "4-3-3"

# Formation data paths
const FORMATIONS_DIR = "res://data/formations/"
const DEFAULT_FORMATIONS = [
	"4-4-2", "4-3-3", "4-2-3-1", "3-5-2", "5-3-2", "4-1-4-1", "3-4-3", "4-4-1-1", "5-4-1", "4-5-1"
]

# Events that TacticsManager emits via EventBus
const EVENT_FORMATION_CHANGED = "formation_changed"
const EVENT_TACTICAL_CALCULATED = "tactical_effectiveness_calculated"
const EVENT_FORMATION_LOADED = "formation_loaded"
const EVENT_TACTICS_READY = "tactics_manager_ready"

# Signals for direct connections
signal formation_changed(team: String, formation_id: String)
signal tactical_effectiveness_calculated(home_effectiveness: float, away_effectiveness: float)
signal formation_loaded(formation_id: String)
signal tactics_system_ready


func _initialize_preset_db():
	if preset_db:
		preset_db.reload()
		print("[TacticsManager] Reloaded %d tactical presets" % preset_db.presets.size())
		return
	preset_db = _TacticalPresetDB.new()
	if preset_db and not preset_db.presets.is_empty():
		print("[TacticsManager] Loaded %d tactical presets" % preset_db.presets.size())
	else:
		print("[TacticsManager] ⚠️ No tactical presets found")


func _ensure_preset_db():
	if not preset_db:
		_initialize_preset_db()


func register_tactical_modifier_provider(provider: Node) -> void:
	tactical_modifier_provider = provider


func _ready():
	print("[TacticsManager] Initializing tactical system...")
	_initialize_preset_db()

	# Initialize Rust components (continue even if it fails)
	var rust_ok = _initialize_rust_components()
	if not rust_ok:
		print("[TacticsManager] ⚠️ Rust components not available - using GDScript fallback")

	# Load default formations (even without Rust, we cache the data)
	_load_default_formations()

	# Setup EventBus listeners
	_setup_event_listeners()

	# Load saved formations
	_load_saved_formations()

	print("[TacticsManager] ✅ Tactical system initialized (Rust=%s)" % rust_ok)
	tactics_system_ready.emit()

	# Emit ready event via EventBus
	var event_bus = get_node_or_null("/root/EventBus")
	if event_bus:
		event_bus.emit(EVENT_TACTICS_READY, {})


func _initialize_rust_components() -> bool:
	"""Initialize tactical components (GDScript autoloads, not Rust classes)"""

	# FormationManager and TacticalEngine are GDScript autoloads, not Rust classes
	# Access them via the autoload system (no class_name, so no type casting)
	formation_manager = get_node_or_null("/root/FormationManager")
	tactical_engine = get_node_or_null("/root/TacticalEngine")

	if not formation_manager:
		print("[TacticsManager] FormationManager autoload not found")
		return false

	if not tactical_engine:
		print("[TacticsManager] TacticalEngine autoload not found")
		return false

	print("[TacticsManager] Tactical components initialized (autoloads)")
	return true


func _load_default_formations():
	"""Load default formation JSON files"""

	for formation_id in DEFAULT_FORMATIONS:
		var formation_data = _create_default_formation(formation_id)

		# Always cache formation data
		formations_cache[formation_id] = formation_data
		formation_loaded.emit(formation_id)

		# Emit via EventBus
		var event_bus = get_node_or_null("/root/EventBus")
		if event_bus:
			event_bus.emit(EVENT_FORMATION_LOADED, {"formation_id": formation_id})


func _create_default_formation(formation_id: String) -> Dictionary:
	"""Create default formation data structure"""

	var formation = {
		"id": formation_id,
		"name": formation_id + " Formation",
		"category": _get_formation_category(formation_id),
		"positions": [],  # Will be filled with actual position data
		"tactical_parameters":
		{
			"attacking_intensity": 0.5,
			"defensive_line_height": 0.5,
			"width": 0.5,
			"pressing_trigger": 0.5,
			"tempo": 0.5,
			"directness": 0.5
		},
		"role_requirements": {}
	}

	# Adjust parameters based on formation type
	match formation_id:
		"4-4-2":
			formation.tactical_parameters.attacking_intensity = 0.5
			formation.tactical_parameters.defensive_line_height = 0.5
		"4-3-3":
			formation.tactical_parameters.attacking_intensity = 0.7
			formation.tactical_parameters.directness = 0.6
		"5-3-2":
			formation.tactical_parameters.attacking_intensity = 0.3
			formation.tactical_parameters.defensive_line_height = 0.3
		"3-5-2":
			formation.tactical_parameters.width = 0.7
			formation.tactical_parameters.pressing_trigger = 0.6
		"4-2-3-1":
			formation.tactical_parameters.attacking_intensity = 0.6
			formation.tactical_parameters.tempo = 0.6

	return formation


func get_available_presets() -> Array:
	_ensure_preset_db()
	if not preset_db:
		return []
	return preset_db.list_presets()


func get_preset_details(preset_id: String) -> Dictionary:
	_ensure_preset_db()
	if not preset_db:
		return {}
	return preset_db.get_preset(preset_id)


func apply_preset(team: String, preset_id: String, adjustments: Dictionary = {}) -> Dictionary:
	_ensure_preset_db()
	var normalized_team: String = "away" if team == "away" else "home"
	var preset_data: Dictionary = get_preset_details(preset_id)
	if preset_data.is_empty():
		push_warning("[TacticsManager] Unknown preset requested: %s" % preset_id)
		return {}

	if normalized_team == "home":
		current_home_preset = preset_id
	else:
		current_away_preset = preset_id
	preset_adjustments[normalized_team] = adjustments.duplicate(true)

	var merged_adjustments: Dictionary = adjustments.duplicate(true)
	var instructions: Dictionary = _build_instruction_map(preset_data, normalized_team, merged_adjustments)
	instructions = _apply_coach_modifiers(instructions)
	last_applied_parameters[normalized_team] = instructions.duplicate(true)
	preset_adjustments[normalized_team] = merged_adjustments
	return instructions


func get_team_preset_snapshot(team: String) -> Dictionary:
	var normalized_team: String = "away" if team == "away" else "home"
	var preset_id: String = current_home_preset if normalized_team == "home" else current_away_preset
	var snapshot: Dictionary = {
		"team": normalized_team,
		"preset_id": preset_id,
		"adjustments": preset_adjustments.get(normalized_team, {}).duplicate(true),
		"instructions": last_applied_parameters.get(normalized_team, {}).duplicate(true)
	}
	if not preset_id.is_empty():
		snapshot["preset_data"] = get_preset_details(preset_id)
	return snapshot


func get_debug_snapshot() -> Dictionary:
	return {"home": get_team_preset_snapshot("home"), "away": get_team_preset_snapshot("away")}


func _build_instruction_map(preset_source, _team: String, adjustments: Dictionary = {}) -> Dictionary:
	_ensure_preset_db()
	var preset_data: Dictionary = {}
	if typeof(preset_source) == TYPE_STRING:
		preset_data = get_preset_details(String(preset_source))
	else:
		preset_data = preset_source if preset_source is Dictionary else {}
	if preset_data.is_empty():
		return {}

	var parameters: Dictionary = preset_data.get("tactical_parameters", {})
	var mapping: Dictionary = {
		"attacking_intensity": "attacking_intensity",
		"defensive_line": "defensive_line_height",
		"pressing": "pressing_trigger",
		"tempo": "tempo"
	}
	var instructions: Dictionary = {}
	for target in mapping.keys():
		var source: String = mapping[target]
		var base_value: float = float(parameters.get(source, 0.5)) * 100.0
		var delta: float = float(adjustments.get(target, 0.0))
		instructions[target] = clamp(base_value + delta, 0.0, 100.0)
	return instructions


func _apply_coach_modifiers(instructions: Dictionary) -> Dictionary:
	if not tactical_modifier_provider:
		return instructions
	if not tactical_modifier_provider.has_method("get_tactical_modifiers"):
		return instructions
	var modifiers: Variant = tactical_modifier_provider.get_tactical_modifiers()
	if modifiers is Dictionary and not modifiers.is_empty():
		for key in ["attacking_intensity", "defensive_line", "pressing", "tempo"]:
			if modifiers.has(key):
				instructions[key] = clampf(float(instructions.get(key, 50)) + float(modifiers[key]), 0.0, 100.0)
	return instructions


func _get_formation_category(formation_id: String) -> String:
	"""Determine formation category based on ID"""

	var parts = formation_id.split("-")
	if parts.size() < 3:
		return "Balanced"

	var defenders = int(parts[0])
	var attackers = int(parts[parts.size() - 1])

	if defenders >= 5:
		return "Defensive"
	elif attackers >= 3:
		return "Attacking"
	elif formation_id.find("3-5") != -1 or formation_id.find("5-3") != -1:
		return "Counter"
	else:
		return "Balanced"


func _setup_event_listeners():
	"""Setup EventBus event listeners"""

	var event_bus = get_node_or_null("/root/EventBus")
	if not event_bus:
		print("[TacticsManager] EventBus not found - skipping event setup")
		return

	# Subscribe to relevant events
	event_bus.subscribe("request_formation_change", _on_formation_change_requested)
	event_bus.subscribe("request_tactical_calculation", _on_tactical_calculation_requested)
	event_bus.subscribe("match_starting", _on_match_starting)
	event_bus.subscribe("halftime", _on_halftime)

	print("[TacticsManager] EventBus listeners configured")


func _load_saved_formations():
	"""Load formations from SaveManager"""

	if not SaveManager:
		print("[TacticsManager] SaveManager not found")
		return

	# Try to get saved formation selections from SaveManager
	var save_data = SaveManager.get_tactics_data() if SaveManager.has_method("get_tactics_data") else {}

	if save_data.has("home_formation"):
		current_home_formation = save_data.home_formation
		print("[TacticsManager] Loaded saved home formation: ", current_home_formation)

	if save_data.has("away_formation"):
		current_away_formation = save_data.away_formation
		print("[TacticsManager] Loaded saved away formation: ", current_away_formation)


## Public API Methods


func set_formation(team: String, formation_id: String) -> bool:
	"""Set formation for a team"""

	if not formations_cache.has(formation_id):
		print("[TacticsManager] Formation not found: ", formation_id)
		return false

	match team.to_lower():
		"home":
			current_home_formation = formation_id
		"away":
			current_away_formation = formation_id
		_:
			print("[TacticsManager] Invalid team: ", team)
			return false

	# Emit signals
	formation_changed.emit(team, formation_id)

	# Emit via EventBus
	var event_bus = get_node_or_null("/root/EventBus")
	if event_bus:
		event_bus.emit(EVENT_FORMATION_CHANGED, {"team": team, "formation_id": formation_id})

	# Save to SaveManager
	_save_formations()

	print("[TacticsManager] Set ", team, " formation to: ", formation_id)
	return true


func get_formation(team: String) -> String:
	"""Get current formation for a team"""

	match team.to_lower():
		"home":
			return current_home_formation
		"away":
			return current_away_formation
		_:
			return ""


func calculate_formation_effectiveness() -> Dictionary:
	"""Calculate tactical effectiveness between current formations"""

	if not formation_manager:
		print("[TacticsManager] Formation manager not available")
		return {"home_effectiveness": 0.5, "away_effectiveness": 0.5}

	# Use Rust calculation
	var home_vs_away = formation_manager.calculate_formation_effectiveness(
		current_home_formation, current_away_formation
	)

	var away_vs_home = formation_manager.calculate_formation_effectiveness(
		current_away_formation, current_home_formation
	)

	var result = {
		"home_effectiveness": home_vs_away,
		"away_effectiveness": away_vs_home,
		"home_formation": current_home_formation,
		"away_formation": current_away_formation
	}

	# Emit signals
	tactical_effectiveness_calculated.emit(home_vs_away, away_vs_home)

	# Emit via EventBus
	var event_bus = get_node_or_null("/root/EventBus")
	if event_bus:
		event_bus.emit(EVENT_TACTICAL_CALCULATED, result)

	print("[TacticsManager] Effectiveness - Home: %.2f, Away: %.2f" % [home_vs_away, away_vs_home])
	return result


func calculate_detailed_effectiveness(
	home_formation: String, away_formation: String, player_stats_json: String = "[]", match_context_json: String = "{}"
) -> Dictionary:
	"""Calculate detailed tactical effectiveness with context"""

	if not tactical_engine:
		print("[TacticsManager] Tactical engine not available")
		return {}

	# Use Rust tactical engine for detailed calculation
	var result = tactical_engine.calculate_formation_effectiveness(
		home_formation, away_formation, player_stats_json, match_context_json
	)

	return result


func get_available_formations() -> Array:
	"""Get list of available formations"""
	return formations_cache.keys()


func get_formation_data(formation_id: String) -> Dictionary:
	"""Get detailed formation data"""

	if not formations_cache.has(formation_id):
		return {}

	return formations_cache[formation_id]


func load_formation_from_file(path: String) -> bool:
	"""Load a formation from a JSON file"""

	if not FileAccess.file_exists(path):
		print("[TacticsManager] Formation file not found: ", path)
		return false

	var file = FileAccess.open(path, FileAccess.READ)
	if not file:
		return false

	var json_string = file.get_as_text()
	file.close()

	var json = JSON.new()
	var parse_result = json.parse(json_string)

	if parse_result != OK:
		print("[TacticsManager] Failed to parse formation JSON")
		return false

	var formation_data = json.data

	# Load into Rust component
	if formation_manager:
		if formation_manager.load_formation(json_string):
			formations_cache[formation_data.id] = formation_data
			formation_loaded.emit(formation_data.id)

			var event_bus = get_node_or_null("/root/EventBus")
			if event_bus:
				event_bus.emit(EVENT_FORMATION_LOADED, {"formation_id": formation_data.id})

			return true

	return false


## Event Handlers


func _on_formation_change_requested(data: Dictionary):
	"""Handle formation change request from EventBus"""

	var team = data.get("team", "home")
	var formation_id = data.get("formation_id", "")

	if formation_id:
		set_formation(team, formation_id)


func _on_tactical_calculation_requested(_data: Dictionary):
	"""Handle tactical calculation request from EventBus"""
	calculate_formation_effectiveness()


func _on_match_starting(data: Dictionary):
	"""Handle match start event"""

	print(
		"[TacticsManager] Match starting with formations - Home: ",
		current_home_formation,
		", Away: ",
		current_away_formation
	)

	# Calculate initial effectiveness
	var effectiveness = calculate_formation_effectiveness()

	# Store in match data
	data["tactical_effectiveness"] = effectiveness


func _on_halftime(data: Dictionary):
	"""Handle halftime tactical adjustments"""

	print("[TacticsManager] Halftime - checking for tactical adjustments")

	# Could implement AI-based formation changes here
	# For now, just recalculate effectiveness
	var effectiveness = calculate_formation_effectiveness()
	data["halftime_tactical_effectiveness"] = effectiveness


## Persistence


func _save_formations():
	"""Save current formations to SaveManager"""

	if not SaveManager:
		return

	var tactics_data = {"home_formation": current_home_formation, "away_formation": current_away_formation}

	# If SaveManager has a specific method for tactics data
	if SaveManager and SaveManager.has_method("set_tactics_data"):
		SaveManager.set_tactics_data(tactics_data)
	else:
		# Otherwise, we'll need to integrate with the general save system
		print("[TacticsManager] SaveManager integration pending")


func get_save_data() -> Dictionary:
	"""Get data for saving"""

	return {
		"home_formation": current_home_formation,
		"away_formation": current_away_formation,
		"formations_cache": formations_cache,
		"home_preset": current_home_preset,
		"away_preset": current_away_preset,
		"preset_adjustments": preset_adjustments.duplicate(true)
	}


func load_save_data(data: Dictionary):
	"""Load data from save"""

	_ensure_preset_db()

	if data.has("home_formation"):
		current_home_formation = data.home_formation

	if data.has("away_formation"):
		current_away_formation = data.away_formation

	if data.has("formations_cache"):
		formations_cache = data.formations_cache

		# Reload formations into Rust component
		if formation_manager:
			for formation_id in formations_cache:
				var formation_json = JSON.stringify(formations_cache[formation_id])
				formation_manager.load_formation(formation_json)

	if data.has("home_preset"):
		current_home_preset = String(data.get("home_preset", ""))
	if data.has("away_preset"):
		current_away_preset = String(data.get("away_preset", ""))
	if data.has("preset_adjustments"):
		preset_adjustments = data.get("preset_adjustments", preset_adjustments).duplicate(true)
	if not preset_adjustments.has("home"):
		preset_adjustments["home"] = {}
	if not preset_adjustments.has("away"):
		preset_adjustments["away"] = {}

	# Rebuild cached parameters
	if current_home_preset != "":
		last_applied_parameters["home"] = _build_instruction_map(current_home_preset, "home")
	if current_away_preset != "":
		last_applied_parameters["away"] = _build_instruction_map(current_away_preset, "away")


## Cleanup


func _exit_tree():
	"""Cleanup when exiting"""

	if formation_manager:
		formation_manager.queue_free()

	if tactical_engine:
		tactical_engine.queue_free()

	print("[TacticsManager] Cleaned up tactical system")
