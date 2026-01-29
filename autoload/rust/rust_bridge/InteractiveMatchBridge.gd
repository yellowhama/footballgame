class_name InteractiveMatchBridge
extends RefCounted
## ============================================================================
## InteractiveMatchBridge - Interactive Match Binary API
## ============================================================================
##
## PURPOSE: Bridge for interactive match control via Rust engine (binary state)
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Start interactive match and get initial SimState
## - Resume interactive match with player actions
## - Decode binary SimState into Dictionary for UI
##
## DEPENDENCIES:
## - _rust_simulator: GDExtension Rust object
##
## USAGE:
##   var bridge := InteractiveMatchBridge.new()
##   bridge.initialize(rust_simulator)
##   var state_bytes := bridge.start_interactive_match_binary(match_data)
##   var state := bridge.decode_interactive_state(state_bytes)
## ============================================================================

var _rust_simulator: Object = null
var _is_ready: bool = false
var _last_error: String = ""


func initialize(rust_simulator: Object) -> void:
	"""Initialize InteractiveMatchBridge with Rust simulator reference"""
	_rust_simulator = rust_simulator
	_is_ready = rust_simulator != null
	if not _is_ready:
		_last_error = "Rust simulator not provided"


# =============================================================================
# Public API
# =============================================================================


func start_interactive_match_binary(match_data: Dictionary) -> PackedByteArray:
	"""Start interactive match and get initial SimState as binary payload
	@param match_data: Dictionary with match setup data
	@return: PackedByteArray containing binary SimState
	"""
	if not _is_ready:
		push_error("[InteractiveMatchBridge] start_interactive_match_binary: engine not ready")
		return PackedByteArray()

	if not _rust_simulator.has_method("start_interactive_match_binary"):
		push_error("[InteractiveMatchBridge] Rust method start_interactive_match_binary not found")
		return PackedByteArray()

	var json_request := JSON.stringify(match_data)
	if json_request == "":
		push_error("[InteractiveMatchBridge] start_interactive_match_binary: failed to serialize request")
		return PackedByteArray()

	return _rust_simulator.start_interactive_match_binary(json_request)


func resume_interactive_match_binary(action: Dictionary) -> PackedByteArray:
	"""Resume interactive match with a high-level action dictionary
	@param action: Dictionary with action type and optional target
		{ "type": "shoot" | "dribble" | "pass_to", "target_id": int }
	@return: PackedByteArray containing updated binary SimState
	"""
	if not _is_ready:
		push_error("[InteractiveMatchBridge] resume_interactive_match_binary: engine not ready")
		return PackedByteArray()

	if not _rust_simulator.has_method("resume_interactive_match_binary"):
		push_error("[InteractiveMatchBridge] Rust method resume_interactive_match_binary not found")
		return PackedByteArray()

	var buffer := StreamPeerBuffer.new()
	var kind := String(action.get("type", "shoot")).to_lower()

	match kind:
		"shoot":
			buffer.put_u8(0)
		"dribble":
			buffer.put_u8(1)
		"pass_to":
			buffer.put_u8(2)
			var target_id: int = int(action.get("target_id", 0))
			buffer.put_u32(target_id)
		_:
			push_error("[InteractiveMatchBridge] resume_interactive_match_binary: invalid action type: %s" % kind)
			return PackedByteArray()

	return _rust_simulator.resume_interactive_match_binary(buffer.data_array)


func decode_interactive_state(state_bytes: PackedByteArray) -> Dictionary:
	"""Decode interactive SimState binary payload into a Dictionary for UI
	@param state_bytes: PackedByteArray containing binary SimState
	@return: Dictionary with one of:
		{ "state": "running" }
		{ "state": "finished", "result_json": String, "replay_json": String }
		{ "state": "paused", "player_id": int, "time_seconds": float,
		  "position": Vector2, "shoot_prob": float, "dribble_prob": float,
		  "pass_targets": [ { "id": int, "success_prob": float, "is_key_pass": bool }, ... ] }
	"""
	var result: Dictionary = {}
	if state_bytes.is_empty():
		result.state = "invalid"
		return result

	var stream := StreamPeerBuffer.new()
	stream.data_array = state_bytes
	stream.big_endian = false

	var tag := stream.get_u8()
	match tag:
		0:
			result.state = "running"
		2:
			result.state = "finished"
			# v1.1+: Finished payload may include result/replay JSON blobs.
			# Back-compat: legacy payload may be tag-only (no extra bytes).
			result.result_json = ""
			result.replay_json = ""

			var remaining := state_bytes.size() - stream.get_position()
			if remaining >= 4:
				var result_len := int(stream.get_u32())
				remaining = state_bytes.size() - stream.get_position()
				if result_len > 0 and remaining >= result_len:
					var read_result := stream.get_data(result_len)
					if read_result[0] == OK and read_result[1] is PackedByteArray:
						result.result_json = (read_result[1] as PackedByteArray).get_string_from_utf8()

			remaining = state_bytes.size() - stream.get_position()
			if remaining >= 4:
				var replay_len := int(stream.get_u32())
				remaining = state_bytes.size() - stream.get_position()
				if replay_len > 0 and remaining >= replay_len:
					var read_replay := stream.get_data(replay_len)
					if read_replay[0] == OK and read_replay[1] is PackedByteArray:
						result.replay_json = (read_replay[1] as PackedByteArray).get_string_from_utf8()
		1:
			result.state = "paused"
			result.player_id = int(stream.get_u32())
			result.time_seconds = stream.get_float()
			var px := stream.get_float()
			var py := stream.get_float()
			result.position = Vector2(px, py)
			result.shoot_prob = stream.get_float()
			result.dribble_prob = stream.get_float()

			var pass_count := int(stream.get_u16())
			var passes: Array = []
			for i in pass_count:
				var tid := int(stream.get_u32())
				var s_prob := stream.get_float()
				var is_key := stream.get_u8() == 1
				passes.append({"id": tid, "success_prob": s_prob, "is_key_pass": is_key})
			result.pass_targets = passes
		_:
			result.state = "unknown"

	return result
