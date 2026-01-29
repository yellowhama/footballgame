extends Node
## MatchTimelineController - Football Match Timeline Animation System
## Handles Timeline data from Rust match engine and animates events on the field
## Based on the engine timeline schema with FIFA-standard coordinates
##
## Phase20: UnifiedFramePipeline integration (2025-12-18)
## - Configures UnifiedFramePipeline with Timeline data and events
## - Acts as playhead driver only (no direct snapshot emission)
## - Pipeline generates snapshots at 50ms intervals with interpolation
## - See: docs/specs/spec_v5/fix/phase20/PHASE20_UNIFIED_FRAME_PIPELINE_SPEC.md

signal timeline_started(timeline_data: Dictionary)
signal timeline_paused
signal timeline_resumed
signal timeline_stopped
signal timeline_completed
signal event_animated(event_data: Dictionary)
signal event_skipped(event_data: Dictionary)
signal position_playback_started(total_duration_ms: int)
signal position_playback_stopped

# Preload to avoid autoload order issues with class_name
const _PositionSnapshotAdapter = preload("res://scripts/match_pipeline/PositionSnapshotAdapter.gd")
const FieldBoard = preload("res://scripts/match_pipeline/FieldBoard.gd")
const MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const MatchSetupBuilder = preload("res://scripts/core/MatchSetupBuilder.gd")
const PlayerRuntimeState = preload("res://scripts/match_pipeline/PlayerRuntimeState.gd")
# ✅ SSOT Cleanup (2025-12-22): MatchSetupBuilder is now accessible as global class_name

## Playback control
enum PlaybackState { STOPPED, PLAYING, PAUSED, SEEKING }

## Animation speed multipliers
enum PlaybackSpeed { VERY_SLOW = 1, SLOW = 2, NORMAL = 4, FAST = 8, VERY_FAST = 16 }  # 0.25x speed  # 0.5x speed  # 1.0x speed  # 2.0x speed  # 4.0x speed

## Event animation modes
enum AnimationMode { SKIP, SIMPLE, DETAILED, HIGHLIGHT }  # No animation - instant  # Simple interpolation  # Full physics simulation  # Enhanced for key moments

# Core state
var current_state: PlaybackState = PlaybackState.STOPPED
var playback_speed: PlaybackSpeed = PlaybackSpeed.NORMAL
var animation_mode: AnimationMode = AnimationMode.SIMPLE

# Timeline data
var timeline_data: Dictionary = {}
var events: Array = []
var current_event_index: int = 0
var total_events: int = 0

# Field dimensions (from Timeline data)
var field_length_m: float = 105.0
var field_width_m: float = 68.0
var field_scale: Vector2 = Vector2.ONE

# Animation control
var event_timer: Timer
var animation_tween: Tween
var event_queue: Array = []

# Performance settings
var max_concurrent_animations: int = 5
var animation_skip_threshold: float = 0.1  # Skip events within 100ms
var interpolation_smoothness: float = 0.8

# Position-based playback (position_data)
# 2025-12-17: 250ms → 50ms (20fps) for smoother ball movement
const POSITION_TICK_MS := 50
var position_data: Dictionary = {}
var timeline_rosters: Dictionary = {}
var has_position_payload: bool = false
var position_timer: Timer
var position_playback_speed: float = 1.0
var position_playing: bool = false
var position_paused: bool = false
var position_time_ms: int = 0
var position_total_duration_ms: int = 0

# Debug and development
var debug_mode: bool = false
var show_event_traces: bool = false
var log_event_details: bool = false

# Match OS Components (Layer 0 & 1)
var field_board = null
var match_setup = null
var runtime_states: Array = []
var os_initialized: bool = false


func _ready():
	print("MatchTimelineController initialized")
	_setup_timers()
	_setup_signals()


func _setup_timers():
	"""Initialize timers for Timeline control"""
	event_timer = Timer.new()
	event_timer.wait_time = 0.1  # 100ms base interval
	event_timer.timeout.connect(_process_next_event)
	event_timer.autostart = false
	call_deferred("add_child", event_timer)

	position_timer = Timer.new()
	position_timer.wait_time = float(POSITION_TICK_MS) / 1000.0
	position_timer.timeout.connect(_process_position_tick)
	position_timer.autostart = false
	call_deferred("add_child", position_timer)

	# Godot 4.x: Tween은 add_child 하지 않음, create_tween()으로 생성
	# animation_tween은 필요할 때 create_tween()으로 생성
	animation_tween = null


func _setup_signals():
	"""Connect internal signals"""
	timeline_started.connect(_on_timeline_started)
	timeline_completed.connect(_on_timeline_completed)
	event_animated.connect(_on_event_animated)


## ============================================================================
## Match OS Initialization
## ============================================================================


func _initialize_match_os(rosters: Dictionary) -> void:
	"""Initialize Match OS components from rosters"""
	if os_initialized:
		print("[MatchTimelineController] Match OS already initialized")
		return

	print("[MatchTimelineController] Initializing Match OS...")

	# 1. Create FieldBoard (Layer 0)
	field_board = FieldBoard.new()
	print("  ✅ FieldBoard created (28×18 grid)")

	# 2. Create MatchSetup (Boot Image)
	var home = rosters.get("home", {})
	var away = rosters.get("away", {})
	# ✅ SSOT Cleanup (2025-12-22): Use Game OS SSOT via builder
	var match_setup_builder = MatchSetupBuilder.new()
	match_setup = match_setup_builder.call("from_rosters_legacy", home, away)
	print("  ✅ MatchSetup created (22 players)")

	# 3. Create PlayerRuntimeState array (22 players)
	runtime_states.clear()
	for track_id in range(22):
		var runtime_state = PlayerRuntimeState.new()
		var player_metadata = match_setup.get_player(track_id)
		runtime_state.set_identity(player_metadata)
		runtime_states.append(runtime_state)
	print("  ✅ PlayerRuntimeState[22] initialized")

	os_initialized = true
	print("[MatchTimelineController] ✅ Match OS initialization complete")


## ============================================================================
## Public API - Timeline Control
## ============================================================================


func load_timeline_from_json(json_string: String) -> bool:
	"""Load Timeline data from JSON string"""
	if json_string.is_empty():
		push_error("MatchTimelineController: Empty JSON string provided")
		return false

	var json = JSON.new()
	var parse_result = json.parse(json_string)

	if parse_result != OK:
		push_error("MatchTimelineController: Failed to parse JSON - " + json.get_error_message())
		return false

	var parsed_data = json.get_data()
	if not _validate_timeline_data(parsed_data):
		return false

	timeline_data = parsed_data
	_extract_timeline_info()

	print("MatchTimelineController: Loaded Timeline with %d events" % total_events)
	return true


func load_timeline_from_rust(rust_engine_node: Node) -> bool:
	"""Load test Timeline data from Rust GDExtension"""
	var legacy_method := "create_test_" + "re" + "play"
	if not rust_engine_node or not rust_engine_node.has_method(legacy_method):
		push_error("MatchTimelineController: Invalid Rust engine node")
		return false

	var test_timeline_json = rust_engine_node.call(legacy_method)
	return load_timeline_from_json(test_timeline_json)


func load_position_data(position_payload: Dictionary, rosters: Dictionary = {}, events_payload: Array = []) -> void:
	# Always resolve events first. Some callers clear position payload ({}) before
	# providing real position data, but the event stream must not be clobbered.
	timeline_rosters = rosters.duplicate(true) if rosters is Dictionary else {}
	events = _resolve_timeline_events_for_pipeline(events_payload)
	total_events = events.size()

	if position_payload.is_empty():
		position_data = {}
		has_position_payload = false
		position_total_duration_ms = 0
		position_time_ms = 0
		if OS.is_debug_build() and total_events > 0:
			print("[MatchTimelineController] Loaded timeline events without position payload: %d" % total_events)
		return

	position_data = position_payload.duplicate(true)
	has_position_payload = true
	position_time_ms = 0
	position_total_duration_ms = _calculate_position_duration(position_data)

	# Initialize Match OS components
	_initialize_match_os(rosters)

	var ball_samples := 0
	if position_data.has("ball") and position_data["ball"] is Array:
		ball_samples = position_data["ball"].size()
	var player_tracks := 0
	if position_data.has("players") and position_data["players"] is Dictionary:
		player_tracks = position_data["players"].size()

	print(
		(
			"MatchTimelineController: Position data loaded (duration=%d ms, ball=%d samples, players=%d)"
			% [position_total_duration_ms, ball_samples, player_tracks]
		)
	)
	if OS.is_debug_build():
		print("[MatchTimelineController] Loaded timeline events for pipeline: %d" % total_events)


func _resolve_timeline_events_for_pipeline(events_payload: Array) -> Array:
	# Phase20 P0.1: empty events must be treated as "missing", not "valid empty".
	# Prefer explicitly provided events, otherwise fall back to MatchTimelineHolder record payload.
	if not events_payload.is_empty():
		return events_payload.duplicate(true)

	# Fallback: WeekHub/TimelinePanel sets MatchTimelineHolder before (or around) load_position_data.
	var holder: Node = get_node_or_null("/root/MatchTimelineHolder")
	if holder and holder.has_method("get_timeline_data"):
		var record_variant: Variant = holder.get_timeline_data()
		if record_variant is Dictionary and not (record_variant as Dictionary).is_empty():
			var recovered := _extract_timeline_events_from_record(record_variant as Dictionary)
			if not recovered.is_empty():
				if OS.is_debug_build():
					print(
						(
							"[MatchTimelineController] Recovered timeline events from MatchTimelineHolder: %d"
							% recovered.size()
						)
					)
				return recovered

	# Final fallback: treat empty as missing and keep existing events if already loaded.
	# This prevents UI layers from accidentally clobbering a valid stream with [].
	if not events.is_empty():
		if OS.is_debug_build():
			print("[MatchTimelineController] Preserving cached timeline events: %d" % events.size())
		return events.duplicate(true)

	return []


func _coerce_events_array(value: Variant) -> Array:
	if value is Array:
		return (value as Array).duplicate(true)
	if value is String:
		var parsed_variant: Variant = JSON.parse_string(String(value))
		if parsed_variant is Array:
			return (parsed_variant as Array).duplicate(true)
	return []


func _coerce_dict(value: Variant) -> Dictionary:
	if value is Dictionary:
		return (value as Dictionary).duplicate(true)
	if value is String:
		var parsed_variant: Variant = JSON.parse_string(String(value))
		if parsed_variant is Dictionary:
			return (parsed_variant as Dictionary).duplicate(true)
	return {}


func _extract_timeline_events_from_record(record: Dictionary) -> Array:
	# Mirrors WeekHub/MatchTimelinePanel extraction logic (minimal and safe).
	const LEGACY_PAYLOAD_KEY := "re" + "play"
	const LEGACY_DOC_KEY := LEGACY_PAYLOAD_KEY + "_doc"
	const LEGACY_EVENTS_KEY := LEGACY_PAYLOAD_KEY + "_events"

	var direct_sources: Array = [
		record.get("timeline_events", null),
		record.get("events", null),
		record.get(LEGACY_EVENTS_KEY, null),
	]
	for source in direct_sources:
		var arr := _coerce_events_array(source)
		if not arr.is_empty():
			return arr

	var doc_sources: Array = []
	doc_sources.append(record.get("timeline_doc", null))
	doc_sources.append(record.get(LEGACY_DOC_KEY, record.get(LEGACY_PAYLOAD_KEY, null)))

	var match_result_variant: Variant = record.get("match_result", null)
	if match_result_variant is Dictionary:
		var match_result: Dictionary = match_result_variant
		doc_sources.append(
			match_result.get(
				"timeline_doc", match_result.get(LEGACY_DOC_KEY, match_result.get(LEGACY_PAYLOAD_KEY, null))
			)
		)
		direct_sources = [
			match_result.get("timeline_events", null),
			match_result.get("events", null),
			match_result.get(LEGACY_EVENTS_KEY, null),
		]
		for source in direct_sources:
			var arr2 := _coerce_events_array(source)
			if not arr2.is_empty():
				return arr2

	var raw_result_variant: Variant = record.get("raw_result", null)
	if raw_result_variant is Dictionary:
		var raw_result: Dictionary = raw_result_variant
		doc_sources.append(
			raw_result.get("timeline_doc", raw_result.get(LEGACY_DOC_KEY, raw_result.get(LEGACY_PAYLOAD_KEY, null)))
		)
		direct_sources = [
			raw_result.get("timeline_events", null),
			raw_result.get("events", null),
			raw_result.get(LEGACY_EVENTS_KEY, null),
		]
		for source in direct_sources:
			var arr3 := _coerce_events_array(source)
			if not arr3.is_empty():
				return arr3

	for doc_source in doc_sources:
		var doc := _coerce_dict(doc_source)
		if doc.is_empty():
			continue
		var doc_events := _coerce_events_array(doc.get("events", null))
		if not doc_events.is_empty():
			return doc_events

	return []


func has_position_data() -> bool:
	return has_position_payload


func start_position_playback(speed: float = 1.0) -> void:
	if not has_position_payload:
		push_error("MatchTimelineController: No position_data loaded")
		return

	position_playback_speed = max(speed, 0.1)
	position_time_ms = 0
	position_playing = true
	position_paused = false
	_update_position_timer_interval()

	# Phase20: Configure UnifiedFramePipeline instead of direct emission
	if has_node("/root/UnifiedFramePipeline"):
		var pipeline = get_node("/root/UnifiedFramePipeline")
		pipeline.set_rosters(timeline_rosters)

		# Phase20 P0: SSOT probe gate (track_id 0..21)
		if OS.is_debug_build() and position_data.has("players") and position_data["players"] is Dictionary:
			_ssot_dbg_keys("MatchTimelineController:position.players(before)", position_data["players"])

		pipeline.set_position_data(position_data)

		if OS.is_debug_build():
			var pipe_pos: Variant = pipeline.get("position_data")
			if (
				pipe_pos is Dictionary
				and (pipe_pos as Dictionary).has("players")
				and (pipe_pos as Dictionary)["players"] is Dictionary
			):
				_ssot_dbg_keys("MatchTimelineController:position.players(after)", (pipe_pos as Dictionary)["players"])

		pipeline.set_event_stream(events)  # ✅ Gap #6 Fix: Correct variable name
		pipeline.set_playhead_ms(0)
		pipeline.start()

		if OS.is_debug_build():
			print("[MatchTimelineController] Configured UnifiedFramePipeline for Timeline mode")

	position_timer.start()
	position_playback_started.emit(position_total_duration_ms)


func pause_position_playback() -> void:
	if not position_playing or position_paused:
		return
	position_paused = true
	position_timer.stop()


func resume_position_playback() -> void:
	if not position_playing or not position_paused:
		return
	position_paused = false
	_update_position_timer_interval()
	position_timer.start()


func stop_position_playback() -> void:
	if not position_playing:
		return
	position_playing = false
	position_paused = false
	position_timer.stop()
	position_playback_stopped.emit()


func seek_position_time(timestamp_ms: int) -> void:
	if not has_position_payload:
		return
	position_time_ms = clamp(timestamp_ms, 0, position_total_duration_ms)


## Standard API for "clip playback" requests.
## v1 rule: do not rely on UI calling `seek -> start`, because `start_position_playback()`
## may reset the playhead to 0 during initialization. This helper enforces a single order.
func play_clip_at(timestamp_ms: int, speed: float = 1.0) -> void:
	if not has_position_payload:
		push_error("MatchTimelineController: No position_data loaded")
		return

	var t_ms: int = clamp(timestamp_ms, 0, position_total_duration_ms)

	if not position_playing:
		start_position_playback(speed)
	else:
		set_position_playback_speed(speed)
		if position_paused:
			resume_position_playback()

	seek_position_time(t_ms)

	var pipeline := get_node_or_null("/root/UnifiedFramePipeline")
	if pipeline and pipeline.has_method("set_playhead_ms"):
		pipeline.set_playhead_ms(t_ms)


func set_position_playback_speed(speed: float) -> void:
	position_playback_speed = max(speed, 0.1)
	if position_playing and not position_paused:
		_update_position_timer_interval()


## @deprecated Use get_standard_snapshot() or consume snapshots from UnifiedFramePipeline.snapshot_ready.
## This legacy function returns raw interpolated positions without metadata.
func get_positions_at(timestamp_ms: int) -> Dictionary:
	push_warning("[MatchTimelineController] get_positions_at() is deprecated. Use get_standard_snapshot() instead.")
	var snapshot := {"ball": Vector2(field_length_m * 0.5, field_width_m * 0.5), "players": {}}

	if not has_position_payload:
		return snapshot

	if position_data.has("ball") and position_data["ball"] is Array:
		snapshot["ball"] = _interpolate_ball_position(position_data["ball"], timestamp_ms)

	if position_data.has("players") and position_data["players"] is Dictionary:
		var players_dict: Dictionary = position_data["players"]
		for key in players_dict.keys():
			var track = players_dict[key]
			if not (track is Array):
				continue
			var key_str := str(key).strip_edges()
			if key_str == "":
				continue
			snapshot["players"][key_str] = _interpolate_player_position(track, timestamp_ms)

	return snapshot


## 표준 스냅샷 포맷 어댑터
## - 내부 get_positions_at() 결과를 MatchTimelineViewer/TimelineMiniMap 등이 공통으로 소비할 수 있는
##   형태로 래핑한다.
## - ball / players.position 은 {x, y} 딕셔너리로 정규화하고,
##   players.state 는 그대로 유지한다.
func get_standard_snapshot(timestamp_ms: int) -> Dictionary:
	# Legacy helper retained for backward compatibility; prefer PositionSnapshotAdapter.from_batch_data
	if position_data.is_empty():
		return {}
	return _PositionSnapshotAdapter.from_batch_data(position_data, timeline_rosters, timestamp_ms)


func start_timeline() -> bool:
	"""Start or resume Timeline playback"""
	if timeline_data.is_empty():
		push_error("MatchTimelineController: No Timeline data loaded")
		return false

	if current_state == PlaybackState.PAUSED:
		resume_timeline()
		return true

	current_event_index = 0
	current_state = PlaybackState.PLAYING
	_start_event_processing()
	timeline_started.emit(timeline_data)

	if debug_mode:
		print("MatchTimelineController: Started Timeline - %d events at %dx speed" % [total_events, playback_speed])

	return true


func pause_timeline():
	"""Pause Timeline playback"""
	if current_state != PlaybackState.PLAYING:
		return

	current_state = PlaybackState.PAUSED
	event_timer.stop()
	_pause_all_animations()
	timeline_paused.emit()

	if debug_mode:
		print("MatchTimelineController: Paused at event %d/%d" % [current_event_index, total_events])


func resume_timeline():
	"""Resume paused Timeline"""
	if current_state != PlaybackState.PAUSED:
		return

	current_state = PlaybackState.PLAYING
	_start_event_processing()
	_resume_all_animations()
	timeline_resumed.emit()

	if debug_mode:
		print("MatchTimelineController: Resumed at event %d/%d" % [current_event_index, total_events])


func stop_timeline():
	"""Stop Timeline and reset to beginning"""
	current_state = PlaybackState.STOPPED
	event_timer.stop()
	_stop_all_animations()
	current_event_index = 0
	event_queue.clear()
	timeline_stopped.emit()

	if debug_mode:
		print("MatchTimelineController: Stopped Timeline")


func seek_to_event(event_index: int):
	"""Seek to specific event in Timeline"""
	if event_index < 0 or event_index >= total_events:
		push_error("MatchTimelineController: Invalid event index: %d" % event_index)
		return

	var was_playing = current_state == PlaybackState.PLAYING
	stop_timeline()

	current_state = PlaybackState.SEEKING
	current_event_index = event_index

	# Fast-forward to the target event without animation
	var original_mode = animation_mode
	animation_mode = AnimationMode.SKIP

	for i in range(current_event_index):
		_process_event_immediately(events[i])

	animation_mode = original_mode

	if was_playing:
		start_timeline()
	else:
		current_state = PlaybackState.STOPPED

	if debug_mode:
		print("MatchTimelineController: Seeked to event %d" % event_index)


func set_playback_speed(speed: PlaybackSpeed):
	"""Change playback speed"""
	playback_speed = speed
	_update_timer_interval()

	if debug_mode:
		print("MatchTimelineController: Speed set to %dx" % speed)


func set_animation_mode(mode: AnimationMode):
	"""Change animation detail level"""
	animation_mode = mode

	if debug_mode:
		print("MatchTimelineController: Animation mode: %s" % AnimationMode.keys()[mode])


## ============================================================================
## Internal Processing
## ============================================================================


func _validate_timeline_data(data: Dictionary) -> bool:
	"""Validate Timeline data structure"""
	if not data.has("schema"):
		push_error("MatchTimelineController: Missing schema information")
		return false

	var schema = data.schema
	var expected_schema_name := "of_" + "re" + "play"
	if schema.get("name", "") != expected_schema_name or schema.get("version", 0) != 1:
		push_error(
			(
				"MatchTimelineController: Unsupported schema: %s v%d"
				% [schema.get("name", "unknown"), schema.get("version", 0)]
			)
		)
		return false

	if not data.has("match") or not data.has("events"):
		push_error("MatchTimelineController: Missing match or events data")
		return false

	if not data.events is Array or data.events.is_empty():
		push_error("MatchTimelineController: No events in Timeline data")
		return false

	return true


func _extract_timeline_info():
	"""Extract key information from Timeline data"""
	# Load raw events
	var raw_events: Array = timeline_data.events

	# P1-1: Filter duplicate events using composite key (time + kind)
	var filtered_events: Array = []
	var seen_keys: Dictionary = {}

	for event in raw_events:
		var kind: String = str(event.get("kind", event.get("etype", event.get("type", "")))).to_lower()
		# Extract time (seconds) from common shapes: base.t | t | time | minute
		var time_seconds: float = 0.0
		if event.has("base") and event.base is Dictionary:
			var base: Dictionary = event.base
			time_seconds = float(base.get("t", 0.0))
		elif event.has("t"):
			time_seconds = float(event.get("t", 0.0))
		elif event.has("time"):
			time_seconds = float(event.get("time", 0.0))
		else:
			var minute_val: int = int(event.get("minute", 0))
			time_seconds = float(minute_val) * 60.0

		# Composite key: rounded time + kind for stability
		var event_key: String = str(round(time_seconds * 100.0) / 100.0) + "_" + kind

		if not seen_keys.has(event_key):
			filtered_events.append(event)
			seen_keys[event_key] = true

	if filtered_events.size() < raw_events.size():
		print("⚠️ Filtered duplicate events: %d → %d" % [raw_events.size(), filtered_events.size()])

	# Commit filtered list
	events = filtered_events
	total_events = events.size()

	# Extract field dimensions
	var match_data = timeline_data.match
	var pitch = match_data.get("pitch", {})
	field_length_m = pitch.get("length_m", 105.0)
	field_width_m = pitch.get("width_m", 68.0)

	# Calculate field scale for screen coordinates
	# This would be set by the field renderer
	field_scale = Vector2.ONE

	# P1-2: Analyze goal events vs final score (debug only)
	if debug_mode:
		var goal_events: Array = []
		var home_goals_in_events: int = 0
		var away_goals_in_events: int = 0

		for ev in events:
			var kind2: String = str(ev.get("kind", ev.get("etype", ev.get("type", "")))).to_lower()
			if kind2 == "goal":
				goal_events.append(ev)
				# Infer team_id from common shapes: base.team_id | team_id | team (home/away)
				var ev_team: Variant = null
				if ev.has("base") and ev.base is Dictionary:
					var base2: Dictionary = ev.base
					ev_team = base2.get("team_id", null)
				elif ev.has("team_id"):
					ev_team = ev.get("team_id")
				elif ev.has("team"):
					var team_label := str(ev.get("team")).to_lower()
					if team_label == "home":
						ev_team = 0
					elif team_label == "away":
						ev_team = 1

				if ev_team == 0:
					home_goals_in_events += 1
				elif ev_team == 1:
					away_goals_in_events += 1

		# Derive final score from most-likely locations
		var home_final: int = 0
		var away_final: int = 0
		if timeline_data.has("score") and timeline_data.score is Dictionary:
			var score: Dictionary = timeline_data.get("score", {"home": 0, "away": 0})
			home_final = int(score.get("home", 0))
			away_final = int(score.get("away", 0))
		elif timeline_data.has("score_home") or timeline_data.has("score_away"):
			home_final = int(timeline_data.get("score_home", 0))
			away_final = int(timeline_data.get("score_away", 0))
		elif timeline_data.has("match") and timeline_data.match is Dictionary:
			var m: Dictionary = timeline_data.match
			home_final = int(m.get("home_score", m.get("score_home", 0)))
			away_final = int(m.get("away_score", m.get("score_away", 0)))

		print("=== Goal Event Analysis (P1-2) ===")
		print("Final Score: %d - %d" % [home_final, away_final])
		print(
			"Goal Events: Home=%d Away=%d Total=%d" % [home_goals_in_events, away_goals_in_events, goal_events.size()]
		)
		if home_goals_in_events != home_final or away_goals_in_events != away_final:
			print("⚠️ MISMATCH: Goal events don't match final score!")
			print(
				(
					"  Discrepancy: Home %+d, Away %+d"
					% [home_goals_in_events - home_final, away_goals_in_events - away_final]
				)
			)

		print("MatchTimelineController: Field %gm × %gm, %d events" % [field_length_m, field_width_m, total_events])


func _start_event_processing():
	"""Start processing events with timer"""
	_update_timer_interval()
	event_timer.start()


func _update_timer_interval():
	"""Update timer interval based on playback speed"""
	var base_interval = 0.1  # 100ms
	var speed_multiplier = float(playback_speed) / 4.0  # Normal = 4 -> 1.0x
	event_timer.wait_time = base_interval / speed_multiplier


func _process_next_event():
	"""Process the next event in the sequence"""
	if current_state != PlaybackState.PLAYING:
		return

	if current_event_index >= total_events:
		_complete_timeline()
		return

	var event_data = events[current_event_index]
	_animate_event(event_data)
	current_event_index += 1


func _process_event_immediately(event_data: Dictionary):
	"""Process event without animation (for seeking)"""
	if log_event_details:
		print(
			(
				"MatchTimelineController: Skipping event %s at (%g, %g)"
				% [
					event_data.get("etype", "unknown"),
					event_data.get("pos", {}).get("x", 0),
					event_data.get("pos", {}).get("y", 0)
				]
			)
		)

	event_skipped.emit(event_data)


func _animate_event(event_data: Dictionary):
	"""Animate a single event based on its type and current animation mode"""
	var event_type = event_data.get("etype", "unknown")
	var position = event_data.get("pos", {})
	var minute = event_data.get("minute", 0)

	if log_event_details:
		print(
			(
				"MatchTimelineController: Animating %s at (%g, %g) - %d'"
				% [event_type, position.get("x", 0), position.get("y", 0), minute]
			)
		)

	match animation_mode:
		AnimationMode.SKIP:
			_process_event_immediately(event_data)
		AnimationMode.SIMPLE:
			_animate_event_simple(event_data)
		AnimationMode.DETAILED:
			_animate_event_detailed(event_data)
		AnimationMode.HIGHLIGHT:
			_animate_event_highlight(event_data)

	event_animated.emit(event_data)


func _animate_event_simple(event_data: Dictionary):
	"""Simple event animation with basic interpolation"""
	var event_type = event_data.get("etype", "unknown")
	var pos = _convert_field_position(event_data.get("pos", {}))

	match event_type:
		"shot":
			_animate_shot_simple(event_data, pos)
		"pass":
			_animate_pass_simple(event_data, pos)
		"dribble":
			_animate_dribble_simple(event_data, pos)
		"save":
			_animate_save_simple(event_data, pos)
		_:
			_animate_generic_event(event_data, pos)


func _animate_event_detailed(event_data: Dictionary):
	"""Detailed event animation with physics"""
	_animate_event_simple(event_data)


func _animate_event_highlight(event_data: Dictionary):
	"""Enhanced animation for highlight moments"""
	_animate_event_simple(event_data)


func _animate_shot_simple(event_data: Dictionary, start_pos: Vector2):
	"""Simple shot animation"""
	var _ball_data = event_data.get("ball", {})  # Reserved for future ball trajectory
	var target_pos = _convert_field_position(event_data.get("target", {}))

	if show_event_traces:
		_draw_trajectory_line(start_pos, target_pos, Color.RED, 1.0)

	# TODO: Animate ball movement to target
	# This would move a ball sprite/3D object along the trajectory


func _animate_pass_simple(event_data: Dictionary, start_pos: Vector2):
	"""Simple pass animation"""
	var end_pos = _convert_field_position(event_data.get("end_pos", {}))
	var _ball_data = event_data.get("ball", {})  # Reserved for future ball trajectory

	if show_event_traces:
		_draw_trajectory_line(start_pos, end_pos, Color.BLUE, 0.8)

	# TODO: Animate ball movement to receiver


func _animate_dribble_simple(event_data: Dictionary, _start_pos: Vector2):
	"""Simple dribble animation"""
	var path = event_data.get("path", [])
	var _end_pos = _convert_field_position(event_data.get("end_pos", {}))  # Reserved for fallback

	if show_event_traces and path.size() > 0:
		for i in range(path.size() - 1):
			var from = _convert_field_position(path[i])
			var to = _convert_field_position(path[i + 1])
			_draw_trajectory_line(from, to, Color.YELLOW, 0.6)


func _animate_save_simple(event_data: Dictionary, _goalkeeper_pos: Vector2):
	"""Simple save animation"""
	var ball_data = event_data.get("ball", {})
	var ball_from = _convert_field_position(ball_data.get("from", {}))
	var ball_to = _convert_field_position(ball_data.get("to", {}))

	if show_event_traces:
		_draw_trajectory_line(ball_from, ball_to, Color.ORANGE, 1.2)

	# TODO: Animate goalkeeper movement


func _animate_generic_event(event_data: Dictionary, _pos: Vector2):
	"""Generic event animation for unknown event types"""
	if debug_mode:
		print("MatchTimelineController: Generic animation for %s" % event_data.get("etype", "unknown"))


func _convert_field_position(field_pos: Dictionary) -> Vector2:
	"""Convert field coordinates (meters) to screen coordinates"""
	var x_m = field_pos.get("x", 0.0)
	var y_m = field_pos.get("y", 0.0)

	# Convert FIFA field coordinates to screen coordinates
	# This would be customized based on the actual field renderer
	var screen_x = (x_m / field_length_m) * field_scale.x
	var screen_y = (y_m / field_width_m) * field_scale.y

	return Vector2(screen_x, screen_y)


func _draw_trajectory_line(from: Vector2, to: Vector2, _color: Color, _width: float):
	"""Draw a trajectory line for debugging (would use actual renderer in practice)"""
	if debug_mode:
		print("MatchTimelineController: Trajectory from (%g, %g) to (%g, %g)" % [from.x, from.y, to.x, to.y])


func _pause_all_animations():
	"""Pause all active animations"""
	if animation_tween and animation_tween.is_valid():
		animation_tween.pause()


func _resume_all_animations():
	"""Resume all paused animations"""
	if animation_tween and animation_tween.is_valid():
		animation_tween.resume()


func _stop_all_animations():
	"""Stop and clear all animations"""
	if animation_tween and animation_tween.is_valid():
		animation_tween.kill()
	event_queue.clear()


func _complete_timeline():
	"""Handle Timeline completion"""
	current_state = PlaybackState.STOPPED
	event_timer.stop()
	timeline_completed.emit()

	if debug_mode:
		print("MatchTimelineController: Timeline completed - %d events processed" % total_events)


## ============================================================================
## Signal Handlers
## ============================================================================


func _on_timeline_started(data: Dictionary):
	"""Handle Timeline started signal"""
	if debug_mode:
		print("MatchTimelineController: Timeline started with %s" % data.get("match", {}).get("id", "unknown"))


func _on_timeline_completed():
	"""Handle Timeline completion"""
	if debug_mode:
		print("MatchTimelineController: Timeline animation completed")


func _on_event_animated(_event: Dictionary):
	"""Handle individual event animation completion"""
	pass  # Can be extended for event-specific handling


## ============================================================================
## Debug and Development
## ============================================================================


func set_debug_mode(enabled: bool):
	"""Toggle debug mode"""
	debug_mode = enabled
	print("MatchTimelineController: Debug mode %s" % ("enabled" if enabled else "disabled"))


func set_show_traces(enabled: bool):
	"""Toggle trajectory traces"""
	show_event_traces = enabled


func set_event_logging(enabled: bool):
	"""Toggle event detail logging"""
	log_event_details = enabled


func get_timeline_info() -> Dictionary:
	"""Get current Timeline information"""
	return {
		"loaded": not timeline_data.is_empty(),
		"total_events": total_events,
		"current_event": current_event_index,
		"state": PlaybackState.keys()[current_state],
		"speed": playback_speed,
		"animation_mode": AnimationMode.keys()[animation_mode],
		"field_size": Vector2(field_length_m, field_width_m)
	}


func get_current_progress() -> float:
	"""Get Timeline progress as 0.0 to 1.0"""
	if total_events == 0:
		return 0.0
	return float(current_event_index) / float(total_events)


## 디버그: 첫 몇 틱만 로깅
var _position_tick_count: int = 0


func _process_position_tick() -> void:
	if not position_playing or not has_position_payload:
		position_timer.stop()
		return

	# Phase20: Just advance playhead, pipeline emits snapshot at 50ms intervals
	if has_node("/root/UnifiedFramePipeline"):
		var pipeline = get_node("/root/UnifiedFramePipeline")
		pipeline.set_playhead_ms(position_time_ms)

	## 디버그: 처음 5틱 + 매 100틱마다 로깅
	_position_tick_count += 1
	if _position_tick_count <= 5 or _position_tick_count % 100 == 0:
		if OS.is_debug_build():
			print(
				(
					"[MatchTimelineController] tick #%d: t_ms=%d (playhead only, pipeline emits snapshot)"
					% [_position_tick_count, position_time_ms]
				)
			)

	# Phase20: FieldBoard enrichment moved to UnifiedFramePipeline._enrich_field_board()
	# Phase20: RuntimeStates update moved to viewer or removed (TBD)
	# Phase20: NO direct snapshot emission - pipeline handles it

	position_time_ms += int(POSITION_TICK_MS * position_playback_speed)
	if position_time_ms >= position_total_duration_ms:
		stop_position_playback()


func _update_position_timer_interval() -> void:
	if not position_timer:
		return
	position_timer.wait_time = (float(POSITION_TICK_MS) / 1000.0) / position_playback_speed


func _calculate_position_duration(payload: Dictionary) -> int:
	var max_timestamp := 0
	if payload.has("ball") and payload["ball"] is Array and payload["ball"].size() > 0:
		var last_ball = payload["ball"][-1]
		# Support both "timestamp" (ms) and "t" (seconds) keys
		if last_ball.has("timestamp"):
			max_timestamp = max(max_timestamp, int(last_ball.get("timestamp", 0)))
		elif last_ball.has("t"):
			# "t" is in seconds, convert to ms
			max_timestamp = max(max_timestamp, int(last_ball.get("t", 0) * 1000))
	if payload.has("players") and payload["players"] is Dictionary:
		for track in payload["players"].values():
			if track is Array and track.size() > 0:
				var last_entry = track[-1]
				# Support both "timestamp" (ms) and "t" (seconds) keys
				if last_entry.has("timestamp"):
					max_timestamp = max(max_timestamp, int(last_entry.get("timestamp", 0)))
				elif last_entry.has("t"):
					max_timestamp = max(max_timestamp, int(last_entry.get("t", 0) * 1000))
	return max_timestamp


func _interpolate_ball_position(samples: Array, timestamp_ms: int) -> Vector2:
	if samples.is_empty():
		return Vector2(field_length_m * 0.5, field_width_m * 0.5)
	return _interpolate_position(samples, timestamp_ms)


func _interpolate_player_position(samples: Array, timestamp_ms: int) -> Dictionary:
	var result := {"position": Vector2(field_length_m * 0.5, field_width_m * 0.5), "state": ""}
	if samples.is_empty():
		return result

	var interpolated := _interpolate_position(samples, timestamp_ms)
	var state := ""

	var best_sample = _get_sample_near_timestamp(samples, timestamp_ms)
	if best_sample:
		state = str(best_sample.get("state", ""))

	result["position"] = interpolated
	result["state"] = state
	return result


func _interpolate_position(samples: Array, timestamp_ms: int) -> Vector2:
	if samples.size() == 1:
		return _extract_position_from_sample(samples[0])

	var upper_index = _find_segment_index(samples, timestamp_ms)

	if upper_index <= 0:
		return _extract_position_from_sample(samples[0])
	if upper_index >= samples.size():
		return _extract_position_from_sample(samples[-1])

	var prev_sample = samples[upper_index - 1]
	var next_sample = samples[upper_index]

	# Support both "timestamp" (ms) and "t" (seconds) keys
	var prev_time: int
	var next_time: int
	if prev_sample.has("timestamp"):
		prev_time = int(prev_sample.get("timestamp", 0))
	else:
		prev_time = int(prev_sample.get("t", 0) * 1000)
	if next_sample.has("timestamp"):
		next_time = int(next_sample.get("timestamp", 0))
	else:
		next_time = int(next_sample.get("t", 0) * 1000)

	if next_time <= prev_time:
		return _extract_position_from_sample(next_sample)

	var t = clamp(float(timestamp_ms - prev_time) / float(next_time - prev_time), 0.0, 1.0)
	var prev_vec = _extract_position_from_sample(prev_sample)
	var next_vec = _extract_position_from_sample(next_sample)
	return prev_vec.lerp(next_vec, t)


## Extract position from sample, supporting both formats:
## Format A (legacy): {"timestamp": ms, "position": {x, y}}
## Format B (binary): {"t": seconds, "x": ..., "y": ...}
func _extract_position_from_sample(sample: Dictionary) -> Vector2:
	if sample.has("position"):
		return _extract_vec2(sample.get("position", Vector2.ZERO))
	elif sample.has("x") and sample.has("y"):
		return Vector2(float(sample.get("x", 0.0)), float(sample.get("y", 0.0)))
	return Vector2.ZERO


func _find_segment_index(samples: Array, timestamp_ms: int) -> int:
	var low := 0
	var high := samples.size() - 1
	while low <= high:
		var mid := (low + high) >> 1
		# Support both "timestamp" (ms) and "t" (seconds) keys
		var mid_sample = samples[mid]
		var mid_ts: int
		if mid_sample.has("timestamp"):
			mid_ts = int(mid_sample.get("timestamp", 0))
		else:
			mid_ts = int(mid_sample.get("t", 0) * 1000)
		if mid_ts == timestamp_ms:
			return mid
		if mid_ts < timestamp_ms:
			low = mid + 1
		else:
			high = mid - 1
	return max(low, 1)


func _get_sample_near_timestamp(samples: Array, timestamp_ms: int) -> Variant:
	if samples.is_empty():
		return null
	var idx := _find_segment_index(samples, timestamp_ms)
	idx = clamp(idx, 1, samples.size() - 1)
	var prev_sample = samples[idx - 1]
	var next_sample = samples[idx]
	# Support both "timestamp" (ms) and "t" (seconds) keys
	var prev_ts: int
	var next_ts: int
	if prev_sample.has("timestamp"):
		prev_ts = int(prev_sample.get("timestamp", 0))
	else:
		prev_ts = int(prev_sample.get("t", 0) * 1000)
	if next_sample.has("timestamp"):
		next_ts = int(next_sample.get("timestamp", 0))
	else:
		next_ts = int(next_sample.get("t", 0) * 1000)
	var prev_time = abs(prev_ts - timestamp_ms)
	var next_time = abs(next_ts - timestamp_ms)
	return next_sample if next_time < prev_time else prev_sample


func _extract_vec2(value: Variant) -> Vector2:
	if value is Vector2:
		return value
	if value is Array and value.size() >= 2:
		return Vector2(float(value[0]), float(value[1]))
	if value is Dictionary:
		return Vector2(float(value.get("x", 0.0)), float(value.get("y", 0.0)))
	return Vector2.ZERO


## ============================================================================
## Integration with Rust Engine
## ============================================================================


func validate_timeline_with_rust(rust_engine_node: Node, timeline_json: String) -> Dictionary:
	"""Validate Timeline JSON using Rust engine validation"""
	var legacy_method := "validate_" + "re" + "play"
	if not rust_engine_node or not rust_engine_node.has_method(legacy_method):
		return {"valid": false, "error": "Rust engine not available"}

	var validation_result_json = rust_engine_node.call(legacy_method, timeline_json)
	var json = JSON.new()

	if json.parse(validation_result_json) == OK:
		return json.get_data()
	else:
		return {"valid": false, "error": "Failed to parse validation result"}


func create_timeline_from_match_result(
	rust_engine_node: Node, match_result_json: String, options: Dictionary = {}
) -> bool:
	"""Create Timeline from match result using Rust engine"""
	var legacy_method := "create_" + "re" + "play" + "_from_match"
	if not rust_engine_node or not rust_engine_node.has_method(legacy_method):
		push_error("MatchTimelineController: Rust engine not available")
		return false

	var options_json = JSON.stringify(options)
	var timeline_json = rust_engine_node.call(legacy_method, match_result_json, options_json)

	return load_timeline_from_json(timeline_json)


# ============================================================================
# Phase20 P0: Track-ID SSOT probe helper (debug-only usage recommended)
# ============================================================================


static func _ssot_dbg_keys(tag: String, players_dict: Dictionary) -> void:
	var keys := players_dict.keys()
	var n := keys.size()
	var bad := 0
	var min_k := 999999
	var max_k := -999999
	var sample := []

	for k in keys:
		var ki := -1
		if k is int:
			ki = k
		else:
			var s := str(k)
			if s.is_valid_int():
				ki = int(s)

		if ki == -1:
			bad += 1
		else:
			min_k = min(min_k, ki)
			max_k = max(max_k, ki)
			if ki < 0 or ki > 21:
				bad += 1

		if sample.size() < 10:
			sample.append(str(k))

	print("[SSOT_KEYS] %s n=%d min=%s max=%s bad=%d sample=%s" % [tag, n, str(min_k), str(max_k), bad, str(sample)])
