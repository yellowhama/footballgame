# PlayerService - Player profile/progression/currency hub
# Autoload as: Name=PlayerService, Path=res://autoload/services/player_service.gd, Singleton=On

extends Node

signal player_level_up(player_id: String, new_level: int)
signal currency_changed(player_id: String, delta: int, balance: int)
signal stat_changed(player_id: String, stat_name: String, old_value: int, new_value: int)

# In-memory model (replace with resource/DTO later)
var _profiles: Dictionary = {}


func _ready() -> void:
	print("[PlayerService] ready")


func ensure_profile(player_id: String) -> void:
	if not _profiles.has(player_id):
		_profiles[player_id] = {
			"level": 1,
			"xp": 0,
			"gold": 0,
			# Football-specific stats
			"ca": 50,  # Current Ability
			"pa": 80,  # Potential Ability
			"hexagon_stats": {"pace": 50, "power": 50, "technical": 50, "shooting": 50, "passing": 50, "defending": 50}
		}


func add_xp(player_id: String, amount: int) -> void:
	ensure_profile(player_id)
	_profiles[player_id].xp += amount
	while _profiles[player_id].xp >= _xp_to_next(_profiles[player_id].level):
		_profiles[player_id].xp -= _xp_to_next(_profiles[player_id].level)
		_profiles[player_id].level += 1
		player_level_up.emit(player_id, _profiles[player_id].level)


func add_gold(player_id: String, delta: int) -> int:
	ensure_profile(player_id)
	_profiles[player_id].gold += delta
	currency_changed.emit(player_id, delta, _profiles[player_id].gold)
	return _profiles[player_id].gold


func update_stat(player_id: String, stat_name: String, new_value: int) -> void:
	ensure_profile(player_id)
	var old_value = 0

	if stat_name == "ca":
		old_value = _profiles[player_id].ca
		_profiles[player_id].ca = new_value
	elif stat_name == "pa":
		old_value = _profiles[player_id].pa
		_profiles[player_id].pa = new_value
	elif _profiles[player_id].hexagon_stats.has(stat_name):
		old_value = _profiles[player_id].hexagon_stats[stat_name]
		_profiles[player_id].hexagon_stats[stat_name] = new_value

	stat_changed.emit(player_id, stat_name, old_value, new_value)


func get_profile(player_id: String) -> Dictionary:
	ensure_profile(player_id)
	return _profiles[player_id].duplicate(true)


func get_hexagon_stats(player_id: String) -> Dictionary:
	ensure_profile(player_id)
	return _profiles[player_id].hexagon_stats.duplicate(true)


func _xp_to_next(level: int) -> int:
	return 100 + (level - 1) * 25


# Save/Load integration (will be connected to GameCore later)
func save_player_data(player_id: String) -> Dictionary:
	ensure_profile(player_id)
	return _profiles[player_id].duplicate(true)


func load_player_data(player_id: String, data: Dictionary) -> bool:
	if data.has("level") and data.has("xp"):
		_profiles[player_id] = data.duplicate(true)
		return true
	return false
