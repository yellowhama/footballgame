extends Node

signal view_mode_changed(mode: String)
signal substitution_applied(payload: Dictionary)
signal tactics_applied(payload: Dictionary)
signal performance_mode_changed(enabled: bool)

const MAX_SUBSTITUTIONS := 3
const VIEW_MODES := ["2d", "3d"]

var _state := {"view_mode": "3d", "remaining_substitutions": MAX_SUBSTITUTIONS, "performance_mode": false}
var _substitution_log: Array = []
var _current_tactics: Dictionary = {}


func _ready() -> void:
	# Ensure state emits initial values for listeners that may subscribe after autoload init.
	call_deferred("_emit_initial_state")


func _emit_initial_state() -> void:
	view_mode_changed.emit(_state.view_mode)
	performance_mode_changed.emit(_state.performance_mode)


func reset(default_mode: String = "3d") -> void:
	var mode := default_mode if default_mode in VIEW_MODES else "3d"
	_state.view_mode = mode
	_state.remaining_substitutions = MAX_SUBSTITUTIONS
	_state.performance_mode = false
	_substitution_log.clear()
	_current_tactics = {}
	view_mode_changed.emit(_state.view_mode)
	performance_mode_changed.emit(false)


func update_view_mode(mode: String, force: bool = false) -> void:
	if mode not in VIEW_MODES:
		push_warning("[MatchControlState] Ignoring invalid view mode: %s" % mode)
		return
	if _state.performance_mode and not force and mode == "3d":
		# When performance mode is active, block switching back to 3d unless forced.
		return
	if _state.view_mode == mode:
		return
	_state.view_mode = mode
	view_mode_changed.emit(mode)


func get_view_mode() -> String:
	return _state.view_mode


func enable_performance_mode(enabled: bool) -> void:
	if _state.performance_mode == enabled:
		return
	_state.performance_mode = enabled
	if enabled:
		update_view_mode("2d", true)
	performance_mode_changed.emit(enabled)


func is_performance_mode() -> bool:
	return _state.performance_mode


func consume_substitution(out_player_id: String, in_player_id: String, minute: int) -> bool:
	if _state.remaining_substitutions <= 0:
		return false

	for entry in _substitution_log:
		if str(entry.get("out_player_id", "")) == out_player_id:
			push_warning("[MatchControlState] Player %s already substituted out" % out_player_id)
			return false

	_state.remaining_substitutions -= 1

	var payload := {
		"out": out_player_id, "in": in_player_id, "minute": minute, "remaining": _state.remaining_substitutions
	}
	var record := {"out_player_id": out_player_id, "in_player_id": in_player_id, "minute": minute, "team": "home"}
	_substitution_log.append(record)
	payload["team"] = record.team
	substitution_applied.emit(payload)
	return true


func get_remaining_substitutions() -> int:
	return _state.remaining_substitutions


func apply_tactics(payload: Dictionary) -> void:
	_current_tactics = payload.duplicate(true)
	if not _current_tactics.has("team"):
		_current_tactics["team"] = "home"
	tactics_applied.emit(_current_tactics)


func get_substitution_log() -> Array:
	return _substitution_log.duplicate()


func get_current_tactics() -> Dictionary:
	return _current_tactics.duplicate(true)
