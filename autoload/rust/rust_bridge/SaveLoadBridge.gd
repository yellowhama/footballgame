class_name SaveLoadBridge
extends RefCounted
## ============================================================================
## SaveLoadBridge - Save/Load Binary API
## ============================================================================
##
## PURPOSE: Bridge for game save/load via Rust engine (MessagePack + LZ4 + SHA256)
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Save game data to binary format
## - Load game data from binary format
## - Handle payload encoding/decoding
##
## DEPENDENCIES:
## - _rust_simulator: GDExtension Rust object
##
## USAGE:
##   var bridge := SaveLoadBridge.new()
##   bridge.initialize(rust_simulator)
##   var binary := bridge.save_game_binary(save_data)
##   var data := bridge.load_game_binary(binary)
## ============================================================================

var _rust_simulator: Object = null
var _is_ready: bool = false


func initialize(rust_simulator: Object) -> void:
	"""Initialize SaveLoadBridge with Rust simulator reference"""
	_rust_simulator = rust_simulator
	_is_ready = rust_simulator != null


# =============================================================================
# Public API
# =============================================================================

func save_game_binary(save_data: Dictionary) -> PackedByteArray:
	"""Save game data to binary format (MessagePack + LZ4 + SHA256)
	@param save_data: Dictionary containing all game data
	@return: PackedByteArray with compressed binary data, or empty array on error
	"""
	if not _is_ready:
		push_error("[SaveLoadBridge] Cannot save: Engine not ready")
		return PackedByteArray()

	# Convert dictionary to JSON string first
	var json_str = JSON.stringify(save_data)
	if json_str == "":
		push_error("[SaveLoadBridge] Failed to serialize save data")
		return PackedByteArray()

	# Call Rust binary save (MessagePack + LZ4 + SHA256)
	var rust_payload = _rust_simulator.save_game_binary(json_str)
	var binary_data := _decode_rust_binary_payload(rust_payload)

	if binary_data.size() == 0:
		push_error("[SaveLoadBridge] Rust returned empty binary data")
		return PackedByteArray()

	print("[SaveLoadBridge] Binary save: %d bytes (compressed)" % binary_data.size())
	return binary_data


func load_game_binary(payload: Variant) -> Dictionary:
	"""Load game data from binary format (MessagePack + LZ4 + SHA256 verification)
	@param payload: PackedByteArray OR base64(String) containing compressed binary data
	@return: Dictionary with game data, or empty dict on error
	"""
	if not _is_ready:
		push_error("[SaveLoadBridge] Cannot load: Engine not ready")
		return {}

	# Accept legacy base64 string payloads
	var binary_data: PackedByteArray = _decode_rust_binary_payload(payload)
	if binary_data.size() == 0:
		push_error("[SaveLoadBridge] Binary data is empty")
		return {}

	# Call Rust binary load (decompress LZ4, verify SHA256, deserialize MessagePack)
	var json_str = _rust_simulator.load_game_binary(binary_data)

	if json_str == "":
		push_error("[SaveLoadBridge] Rust returned empty JSON string")
		return {}

	# Parse JSON
	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_str)

	if parse_result != OK:
		push_error("[SaveLoadBridge] Failed to parse loaded JSON: %s" % json_parser.get_error_message())
		return {}

	print("[SaveLoadBridge] Binary load: %d bytes decompressed" % binary_data.size())
	return json_parser.data


# =============================================================================
# Private Helpers
# =============================================================================

func _decode_rust_binary_payload(payload: Variant) -> PackedByteArray:
	"""Decode payload from Rust (handles both PackedByteArray and base64 string)"""
	var binary := PackedByteArray()
	match typeof(payload):
		TYPE_PACKED_BYTE_ARRAY:
			var bytes: PackedByteArray = payload
			# OpenFootball Save magic: "OFSV"
			if bytes.size() >= 4 and bytes[0] == 79 and bytes[1] == 70 and bytes[2] == 83 and bytes[3] == 86:
				return bytes

			# Backward compatibility: some older saves stored binary as base64 text
			var encoded := bytes.get_string_from_utf8().strip_edges()
			if encoded.begins_with("T0ZTVg"):  # base64("OFSV") prefix
				var decoded := Marshalls.base64_to_raw(encoded)
				if (
					decoded.size() >= 4
					and decoded[0] == 79
					and decoded[1] == 70
					and decoded[2] == 83
					and decoded[3] == 86
				):
					return decoded
			return bytes
		TYPE_STRING, TYPE_STRING_NAME:
			var encoded := str(payload)
			if encoded == "":
				return binary
			encoded = encoded.strip_edges()
			var decoded := Marshalls.base64_to_raw(encoded)
			# Validate OpenFootball Save magic: "OFSV"
			if decoded.size() >= 4 and decoded[0] == 79 and decoded[1] == 70 and decoded[2] == 83 and decoded[3] == 86:
				return decoded
			return binary
		_:
			push_error("[SaveLoadBridge] Unexpected payload type: %s" % typeof(payload))
	return binary


func encode_binary_payload(data: PackedByteArray) -> String:
	"""Encode binary data to base64 string"""
	if data.size() == 0:
		return ""
	return Marshalls.raw_to_base64(data)
