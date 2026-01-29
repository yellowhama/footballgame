extends RefCounted
class_name DebugSettings

const CONFIG_PATH := "user://debug_settings.cfg"
const SECTION := "debug"


static func get_team_view_always_on(default_value: bool = false) -> bool:
	var config := ConfigFile.new()
	var err := config.load(CONFIG_PATH)
	if err != OK:
		return default_value
	return bool(config.get_value(SECTION, "team_view_always_on", default_value))


static func set_team_view_always_on(value: bool) -> void:
	var config := ConfigFile.new()
	var err := config.load(CONFIG_PATH)
	if err != OK:
		config.clear()
	config.set_value(SECTION, "team_view_always_on", value)
	config.save(CONFIG_PATH)


# ==============================================================================
# MULTI-AGENT DEBUG SETTINGS (FIX_2601/0107)
# ==============================================================================


## Enable multi-agent mode with 2 controllers (home/away strikers)
static func get_multi_agent_enabled(default_value: bool = false) -> bool:
	var config := ConfigFile.new()
	var err := config.load(CONFIG_PATH)
	if err != OK:
		return default_value
	return bool(config.get_value(SECTION, "multi_agent_enabled", default_value))


static func set_multi_agent_enabled(value: bool) -> void:
	var config := ConfigFile.new()
	var err := config.load(CONFIG_PATH)
	if err != OK:
		config.clear()
	config.set_value(SECTION, "multi_agent_enabled", value)
	config.save(CONFIG_PATH)


## Get controller slot configuration (JSON array)
## Format: [{"controller_id": 0, "team_side": "home", "player_slot": 9}, ...]
static func get_multi_agent_slots(default_value: String = "[]") -> String:
	var config := ConfigFile.new()
	var err := config.load(CONFIG_PATH)
	if err != OK:
		return default_value
	return String(config.get_value(SECTION, "multi_agent_slots", default_value))


static func set_multi_agent_slots(value: String) -> void:
	var config := ConfigFile.new()
	var err := config.load(CONFIG_PATH)
	if err != OK:
		config.clear()
	config.set_value(SECTION, "multi_agent_slots", value)
	config.save(CONFIG_PATH)


## Parse multi-agent slots from JSON string
static func parse_multi_agent_slots() -> Array:
	var json_str := get_multi_agent_slots()
	var parsed = JSON.parse_string(json_str)
	if parsed is Array:
		return parsed
	return []


## Default 2-player configuration: home striker vs away striker
static func get_default_pvp_slots() -> Array:
	return [
		{"controller_id": 0, "team_side": "home", "player_slot": 9},
		{"controller_id": 1, "team_side": "away", "player_slot": 9}
	]


## Default co-op configuration: both on home team (striker + midfielder)
static func get_default_coop_slots() -> Array:
	return [
		{"controller_id": 0, "team_side": "home", "player_slot": 9},
		{"controller_id": 1, "team_side": "home", "player_slot": 7}
	]


## Set up default PvP configuration
static func setup_default_pvp() -> void:
	set_multi_agent_enabled(true)
	set_multi_agent_slots(JSON.stringify(get_default_pvp_slots()))


## Set up default Co-op configuration
static func setup_default_coop() -> void:
	set_multi_agent_enabled(true)
	set_multi_agent_slots(JSON.stringify(get_default_coop_slots()))
