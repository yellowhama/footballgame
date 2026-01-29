## ClipProvider - Singleton for managing match clips from Rust ClipReducer
##
## Part of the 3-Mode Viewing System (Full Match / Highlight / Key Moment)
##
## Responsibilities:
## - Fetch clips from Rust engine via get_match_clips_from_result(match_result_json, mode)
## - Cache clips per match_id and mode
## - Provide clip access interface for MatchViewScreen
##
## Usage:
## ```gdscript
## # Load clips for current match
## ClipProvider.load_clips_for_match("match_001", "highlight")
##
## # Wait for loading
## await ClipProvider.clips_loaded
##
## # Access clips
## var clips = ClipProvider.get_clips()
## for clip in clips:
##     print("Clip: %s - %d to %d (score: %.2f)" % [
##         clip.description,
##         clip.start_ms,
##         clip.end_ms,
##         clip.chance_score
##     ])
## ```

extends Node

## Emitted when clips are loaded (count = number of clips loaded)
signal clips_loaded(count: int)

## Emitted when clip loading fails
signal clips_load_failed(error: String)

# ============================================================================
# State
# ============================================================================

## Current match ID
var _match_id: String = ""

## Current clip mode ("full", "highlight", "key_moment")
var _current_mode: String = "full"

## Cached clips (Array of Dictionary)
var _clips: Array = []

## Loading state
var _is_loading: bool = false

## Optional cached match_result JSON for clip generation (authoritative source).
## This avoids depending on deprecated `get_match_clips(match_id, mode)` stub.
var _match_result_json: String = ""

# ============================================================================
# Public API
# ============================================================================


## Load clips for a specific match and mode
##
## Modes:
## - "full": No filtering (entire match playback)
## - "highlight": Precomputed highlights (SSOT in Rust ClipReducer)
## - "key_moment": Precomputed key moments (SSOT in Rust ClipReducer)
##
## Returns: void (listen to clips_loaded signal)
func load_clips_for_match(match_id: String, mode: String) -> void:
	if _is_loading:
		push_warning("[ClipProvider] Already loading clips, ignoring request")
		return

	_is_loading = true
	_match_id = match_id
	_current_mode = mode
	_clips.clear()

	# Full match mode: no clips needed
	if mode == "full":
		_is_loading = false
		clips_loaded.emit(0)
		return

	# Validate mode
	if mode not in ["highlight", "key_moment"]:
		push_error("[ClipProvider] Invalid mode: %s" % mode)
		_is_loading = false
		clips_load_failed.emit("Invalid mode: %s" % mode)
		return

	# Fetch from Rust engine
	_fetch_clips_from_rust(match_id, mode)


## Provide match_result JSON explicitly (recommended).
func set_match_result_json(match_result_json: String) -> void:
	_match_result_json = match_result_json


## Provide match_result Dictionary explicitly (recommended).
func set_match_result(match_result: Dictionary) -> void:
	if match_result.is_empty():
		_match_result_json = ""
		return
	_match_result_json = JSON.stringify(match_result)


## Get all cached clips
##
## Returns: Array of Dictionary with keys:
## - id: String (unique clip ID)
## - mode: String ("highlight" or "key_moment")
## - start_ms: int (clip start time in milliseconds)
## - end_ms: int (clip end time in milliseconds)
## - chance_score: float (ChanceScore value 0.0-1.0)
## - description: String (human-readable description)
func get_clips() -> Array:
	return _clips.duplicate()


## Get clip count
func get_clip_count() -> int:
	return _clips.size()


## Get clip at specific index
##
## Returns: Dictionary or empty dict if index out of bounds
func get_clip_at(index: int) -> Dictionary:
	if index < 0 or index >= _clips.size():
		return {}
	return _clips[index]


## Find clip that contains given time
##
## Args:
##   t_ms: Time in milliseconds from match start
##
## Returns: Dictionary of clip or empty dict if no clip found
func find_clip_at_time(t_ms: int) -> Dictionary:
	for clip in _clips:
		var start_ms: int = clip.get("start_ms", 0)
		var end_ms: int = clip.get("end_ms", 0)

		if start_ms <= t_ms and t_ms <= end_ms:
			return clip

	return {}


## Get current match ID
func get_match_id() -> String:
	return _match_id


## Get current mode
func get_current_mode() -> String:
	return _current_mode


## Check if currently loading
func is_loading() -> bool:
	return _is_loading


## Clear cached clips
func clear_clips() -> void:
	_clips.clear()
	_match_id = ""
	_current_mode = "full"
	_is_loading = false


# ============================================================================
# Internal Implementation
# ============================================================================


## Fetch clips from Rust engine
func _fetch_clips_from_rust(match_id: String, mode: String) -> void:
	# Check if Rust engine singleton exists
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine._rust_simulator:
		push_error("[ClipProvider] FootballRustEngine not available")
		_is_loading = false
		clips_load_failed.emit("Rust engine not available")
		return

	var rust = rust_engine._rust_simulator

	var match_result_json := _resolve_match_result_json()
	if match_result_json.is_empty():
		push_error("[ClipProvider] Missing match_result for clips. Provide it via set_match_result() or via MatchTimelineHolder record.")
		_is_loading = false
		clips_load_failed.emit("Missing match_result for clips")
		return

	# Prefer the authoritative API; legacy get_match_clips is deprecated and fail-fast.
	if not rust.has_method("get_match_clips_from_result"):
		push_error("[ClipProvider] get_match_clips_from_result() method not found on Rust engine")
		_is_loading = false
		clips_load_failed.emit("Rust method not available")
		return

	# Call Rust method
	var json_result: String = rust.get_match_clips_from_result(match_result_json, mode)

	# Parse JSON result
	var json = JSON.new()
	var parse_result = json.parse(json_result)

	if parse_result != OK:
		push_error("[ClipProvider] Failed to parse JSON from Rust: %s" % json_result)
		_is_loading = false
		clips_load_failed.emit("JSON parse error")
		return

	var data = json.data

	# Check for error response
	if data is Dictionary and data.has("error"):
		var error_msg: String = data.get("message", "Unknown error")
		push_error("[ClipProvider] Rust returned error: %s" % error_msg)
		_is_loading = false
		clips_load_failed.emit(error_msg)
		return

	# Expect array of clips
	if not data is Array:
		push_error("[ClipProvider] Expected array of clips, got: %s" % typeof(data))
		_is_loading = false
		clips_load_failed.emit("Invalid response format")
		return

	# Store clips
	_clips = data
	_is_loading = false

	print("[ClipProvider] Loaded %d clips for match %s (mode: %s)" % [_clips.size(), match_id, mode])

	clips_loaded.emit(_clips.size())


func _resolve_match_result_json() -> String:
	if _match_result_json != "":
		return _match_result_json

	# Fallback: if the caller is inside the viewer pipeline, the holder may have the record payload.
	var holder := get_node_or_null("/root/MatchTimelineHolder")
	if holder and holder.has_method("get_timeline_data"):
		var record_variant: Variant = holder.get_timeline_data()
		if record_variant is Dictionary:
			var record: Dictionary = record_variant
			var mr_variant: Variant = record.get("match_result", {})
			if mr_variant is Dictionary and not (mr_variant as Dictionary).is_empty():
				return JSON.stringify(mr_variant)
	return ""


# ============================================================================
# Debug Helpers
# ============================================================================


## Print all clips to console (for debugging)
func debug_print_clips() -> void:
	print("=== ClipProvider Debug ===")
	print("Match ID: %s" % _match_id)
	print("Mode: %s" % _current_mode)
	print("Clip Count: %d" % _clips.size())
	print()

	for i in range(_clips.size()):
		var clip = _clips[i]
		print("Clip %d:" % (i + 1))
		print("  ID: %s" % clip.get("id", "N/A"))
		print("  Mode: %s" % clip.get("mode", "N/A"))
		print(
			(
				"  Time: %d ms - %d ms (%.1f sec)"
				% [
					clip.get("start_ms", 0),
					clip.get("end_ms", 0),
					(clip.get("end_ms", 0) - clip.get("start_ms", 0)) / 1000.0
				]
			)
		)
		print("  ChanceScore: %.3f" % clip.get("chance_score", 0.0))
		print("  Description: %s" % clip.get("description", "N/A"))
		print()

	print("========================")


## Create test clips (for UI testing without Rust engine)
func create_test_clips() -> void:
	_clips = [
		{
			"id": "clip_test_1",
			"mode": "highlight",
			"start_ms": 10000,
			"end_ms": 15000,
			"chance_score": 0.15,
			"description": "Shot on target"
		},
		{
			"id": "clip_test_2",
			"mode": "highlight",
			"start_ms": 30000,
			"end_ms": 38000,
			"chance_score": 0.22,
			"description": "Dangerous attack"
		},
		{
			"id": "clip_test_3",
			"mode": "key_moment",
			"start_ms": 45000,
			"end_ms": 52000,
			"chance_score": 1.0,
			"description": "Goal"
		},
		{
			"id": "clip_test_4",
			"mode": "highlight",
			"start_ms": 65000,
			"end_ms": 72000,
			"chance_score": 0.18,
			"description": "Corner kick sequence"
		},
		{
			"id": "clip_test_5",
			"mode": "key_moment",
			"start_ms": 80000,
			"end_ms": 87000,
			"chance_score": 0.35,
			"description": "Close range shot"
		}
	]

	_match_id = "test_match"
	_current_mode = "highlight"

	print("[ClipProvider] Created %d test clips" % _clips.size())
	clips_loaded.emit(_clips.size())


# ============================================================================
# Lifecycle
# ============================================================================


func _ready() -> void:
	print("[ClipProvider] Initialized")

	# Set node name for easy access
	name = "ClipProvider"
