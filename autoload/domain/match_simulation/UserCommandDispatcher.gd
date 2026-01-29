class_name UserCommandDispatcher
extends RefCounted
## ============================================================================
## UserCommandDispatcher - User Command System for Career Player Mode
## ============================================================================
##
## PURPOSE: Send user commands to Rust engine during match simulation
##
## EXTRACTED FROM: MatchSimulationManager.gd (ST-005 God Class refactoring)
##
## RESPONSIBILITIES:
## - Send user commands to Rust engine (Career Player Mode)
## - Manage multi-agent controller slots
## - Handle sticky actions (sprint/dribble/press)
## - Rate limit command spam
##
## DEPENDENCIES:
## - FootballRustEngine (autoload): For Rust engine communication
##
## USAGE:
##   var dispatcher := UserCommandDispatcher.new()
##   dispatcher.initialize(rust_engine_node)
##   dispatcher.send_user_command({"action": "pass", "target_player_id": 5})
## ============================================================================

signal user_command_sent(cmd: Dictionary)
signal user_command_acked(seq: int, ok: bool, reason: String)

## Rate limiting
const USER_CMD_COOLDOWN_MS := 150  ## Client spam prevention

## State
var _user_cmd_seq: int = 0
var _last_sent_ms: int = 0
var _multi_agent_seq: Dictionary = {}

## External dependency
var _rust_engine: Node = null


func initialize(rust_engine: Node = null) -> void:
	"""Initialize UserCommandDispatcher with Rust engine reference"""
	_rust_engine = rust_engine


func _get_rust_simulator() -> Object:
	if not _rust_engine:
		return null
	var sim: Variant = _rust_engine.get("_rust_simulator")
	if sim == null:
		return null
	if sim is Object:
		return sim
	return null


# =============================================================================
# Single User Command
# =============================================================================

func send_user_command(cmd: Dictionary) -> void:
	"""Send user command to Rust engine (Career Player Mode)"""
	var now := Time.get_ticks_msec()
	if now - _last_sent_ms < USER_CMD_COOLDOWN_MS:
		print("[UserCommandDispatcher] Command spam detected, ignoring")
		return
	_last_sent_ms = now

	_user_cmd_seq += 1

	# Flatten payload to top level for Rust compatibility
	if cmd.has("payload"):
		var payload = cmd["payload"]
		for key in payload.keys():
			cmd[key] = payload[key]
		cmd.erase("payload")

	cmd["type"] = "user_command"
	cmd["t_client_ms"] = now
	cmd["seq"] = _user_cmd_seq

	user_command_sent.emit(cmd)

	# Send to Rust
	var json := JSON.stringify(cmd)
	var simulator := _get_rust_simulator()
	if simulator:
		if simulator.has_method("submit_user_command"):
			var result = simulator.submit_user_command(json)
			print("[UserCommandDispatcher] Sent user command seq=%d: %s" % [_user_cmd_seq, result])
		else:
			print("[UserCommandDispatcher] ERROR: submit_user_command method not found")
	else:
		print("[UserCommandDispatcher] ERROR: Rust engine not available")


# =============================================================================
# Multi-Agent Controller Slots
# =============================================================================

func register_controller_slot(controller_id: int, team_side: String, player_slot: int) -> void:
	"""Register a controller slot for multi-agent control"""
	var simulator := _get_rust_simulator()
	if not simulator:
		push_warning("[UserCommandDispatcher] Rust engine not available for register_controller_slot")
		return

	if simulator.has_method("register_controller_slot"):
		var result = simulator.register_controller_slot(controller_id, team_side, player_slot)
		if OS.is_debug_build():
			print("[UserCommandDispatcher] register_controller_slot %d/%s/%d: %s" % [controller_id, team_side, player_slot, str(result)])


func unregister_controller_slot(controller_id: int) -> void:
	"""Unregister a controller slot for multi-agent control"""
	var simulator := _get_rust_simulator()
	if not simulator:
		push_warning("[UserCommandDispatcher] Rust engine not available for unregister_controller_slot")
		return

	if simulator.has_method("unregister_controller_slot"):
		var result = simulator.unregister_controller_slot(controller_id)
		if OS.is_debug_build():
			print("[UserCommandDispatcher] unregister_controller_slot %d: %s" % [controller_id, str(result)])


func clear_controller_slots() -> void:
	"""Clear all controller slots for multi-agent control"""
	_multi_agent_seq.clear()

	var simulator := _get_rust_simulator()
	if not simulator:
		push_warning("[UserCommandDispatcher] Rust engine not available for clear_controller_slots")
		return

	if simulator.has_method("clear_controller_slots"):
		var result = simulator.clear_controller_slots()
		if OS.is_debug_build():
			print("[UserCommandDispatcher] clear_controller_slots: %s" % str(result))


# =============================================================================
# Multi-Agent Commands
# =============================================================================

func send_multi_agent_commands(commands: Array) -> void:
	"""Send multi-agent commands to Rust engine.
	Each command must include controller_id. If seq missing, it is auto-assigned per controller.
	"""
	if commands.is_empty():
		return

	var now := Time.get_ticks_msec()
	var payloads: Array = []

	for raw_cmd in commands:
		if not (raw_cmd is Dictionary):
			continue
		var cmd: Dictionary = raw_cmd.duplicate(true)
		if not cmd.has("controller_id"):
			continue

		# Flatten payload to top level (Rust expects flat structure)
		if cmd.has("payload"):
			var payload = cmd["payload"]
			for key in payload.keys():
				cmd[key] = payload[key]
			cmd.erase("payload")

		var controller_id: int = int(cmd["controller_id"])
		if not cmd.has("seq"):
			var last_seq := 0
			if _multi_agent_seq.has(controller_id):
				last_seq = int(_multi_agent_seq[controller_id])
			last_seq += 1
			_multi_agent_seq[controller_id] = last_seq
			cmd["seq"] = last_seq

		cmd["t_client_ms"] = now
		payloads.append(cmd)

	if payloads.is_empty():
		return

	var json := JSON.stringify(payloads)
	var simulator := _get_rust_simulator()
	if not simulator:
		push_warning("[UserCommandDispatcher] Rust engine not available for submit_multi_agent_commands")
		return

	if simulator.has_method("submit_multi_agent_commands"):
		var result = simulator.submit_multi_agent_commands(json)
		if OS.is_debug_build():
			print("[UserCommandDispatcher] Sent multi-agent commands (%d): %s" % [payloads.size(), str(result)])


# =============================================================================
# Sticky Actions
# =============================================================================

func set_sticky_action(track_id: int, action: String, enabled: bool) -> void:
	"""Toggle sticky actions (sprint/dribble/press) for a player"""
	if track_id < 0:
		push_warning("[UserCommandDispatcher] Sticky action ignored: invalid track_id")
		return

	var simulator := _get_rust_simulator()
	if not simulator:
		push_warning("[UserCommandDispatcher] Rust engine not available for sticky action")
		return

	if simulator.has_method("set_sticky_action"):
		var result = simulator.set_sticky_action(track_id, action, enabled)
		if OS.is_debug_build():
			print("[UserCommandDispatcher] Sticky action %s=%s (track_id=%d): %s" % [action, str(enabled), track_id, str(result)])


# =============================================================================
# Getters
# =============================================================================

func get_current_seq() -> int:
	return _user_cmd_seq


func get_multi_agent_seq(controller_id: int) -> int:
	return int(_multi_agent_seq.get(controller_id, 0))
