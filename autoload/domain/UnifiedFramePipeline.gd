extends Node
## UnifiedFramePipeline - Single snapshot emitter for Session and Timeline
## Phase20: Full unification of Session/Timeline data flows
## See: docs/specs/spec_v5/fix/phase20/PHASE20_UNIFIED_FRAME_PIPELINE_SPEC.md
##
## Uber Realtime Architecture (2026-01-13):
## - DeltaFilter: Drop redundant snapshots (30% reduction target)
## - AOISelector: LOD based on ball proximity
## See: docs/specs/fix_2601/0113/UBER_REALTIME_ARCHITECTURE.md

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _DeltaFilter = preload("res://scripts/match_pipeline/DeltaFilter.gd")
const _AOISelector = preload("res://scripts/match_pipeline/AOISelector.gd")
const _ReplaySmoother = preload("res://scripts/match_pipeline/ReplaySmoother.gd")
const _PositionSnapshotAdapter = preload("res://scripts/match_pipeline/PositionSnapshotAdapter.gd")
const MatchEventKeys = preload("res://scripts/constants/MatchEventKeys.gd")

signal snapshot_ready(t_ms: int, snapshot: Dictionary)

var rosters: Dictionary = {}

# Unified storage
var position_data: Dictionary = {"ball": [], "players": {}, "field_board_snapshot": []}
var events: Array = []
var event_index: int = 0
var playhead_ms: int = 0

# Phase23: TransitionSystem debug signal from engine (ms; -1 = inactive)
var transition_remaining_ms: int = -1
var team_view_simple: Dictionary = {}
var team_view_minimap: Dictionary = {}
var team_view_tick_ms: int = -1
var last_decision_intents: Array = []
var last_offside_lines: Dictionary = {}
var last_field_board_snapshot: Dictionary = {}

# Event tracking (Phase20 Step E: Event unification)
# Phase20 Bug Fix #12: Use Dictionary for O(1) dedup lookup (was Array with O(n) scan)
var last_emitted_events: Dictionary = {}  # event_id -> true

# Output timer (50ms = 20Hz)
var timer: Timer = null
const OUTPUT_TICK_MS: int = 50
var is_running: bool = false  # Phase20 Bug Fix #3: Lifecycle guard
var is_paused: bool = false   # Replay support

# Phase E: Variable speed playback
var playback_speed: float = 1.0  # 1.0 = normal, 0.25 = slow, 4.0 = fast
var is_replay_mode: bool = false  # True for replay, false for live session

# FieldBoard (for enrichment)
var field_board = null  # Reference to FieldBoard autoload

# FIX_2512 (B Layer): visual-only smoothing for playback snapshots
var enable_smoothing: bool = true
var smoothing_timeline_only: bool = true
var replay_smoother: _ReplaySmoother = null

# Uber Realtime Architecture: DeltaFilter
var delta_filter: _DeltaFilter = null
var enable_delta_filter: bool = true
var delta_filter_session_only: bool = true  # Only apply in live session (not replay)

# Uber Realtime Architecture: AOISelector
var aoi_selector: _AOISelector = null
var enable_aoi_selector: bool = true
var aoi_selector_session_only: bool = true  # Only apply in live session (not replay)


func _ready() -> void:
	timer = Timer.new()
	timer.wait_time = OUTPUT_TICK_MS / 1000.0
	timer.one_shot = false
	timer.timeout.connect(_on_timer_tick)
	add_child(timer)

	# Get FieldBoard reference
	if has_node("/root/FieldBoard"):
		field_board = get_node("/root/FieldBoard")

	if OS.is_debug_build():
		print("[UnifiedFramePipeline] Initialized (Phase20 unification)")

	# Init smoother (visual-only; does not mutate SSOT position_data)
	replay_smoother = _ReplaySmoother.new()

	# Init DeltaFilter (Uber Realtime Architecture)
	delta_filter = _DeltaFilter.new()

	# Init AOISelector (Uber Realtime Architecture)
	aoi_selector = _AOISelector.new()


func _exit_tree() -> void:
	# Phase20 Bug Fix #6: Explicit timer cleanup
	if timer:
		timer.stop()
		# Timer is a child node, so it will be automatically freed
		# But explicit stop is good practice


## API: Set mode (deprecated - no-op)
func set_mode(_new_mode: String) -> void:
	pass  # Mode system removed - unified pipeline


## API: Set rosters (required for both modes)
func set_rosters(new_rosters: Dictionary) -> void:
	rosters = new_rosters

	if OS.is_debug_build():
		print("[UnifiedFramePipeline] Rosters set: %d teams" % rosters.size())


## API: Push Session tick
func push_tick(step_result: Dictionary) -> void:
	# Phase20 Bug Fix #9: Validate input before processing
	if step_result.is_empty():
		if OS.is_debug_build():
			push_warning("[UnifiedFramePipeline] Empty step_result in push_tick")
		return

	# Convert and append to position_data (ring buffer: keep last 4 ticks)
	_convert_session_tick_to_position_data(step_result)

	# Update playhead (real-time progression)
	playhead_ms = step_result.get("t_ms", 0)

	# Phase23: TransitionSystem debug value from Rust step dict
	transition_remaining_ms = int(step_result.get("transition_remaining_ms", -1))

	# Update events (overwrite with latest tick's events)
	var tick_events = step_result.get("events", [])
	events = tick_events
	event_index = 0

	if step_result.has("team_view_simple"):
		team_view_simple = step_result.get("team_view_simple", {})
		team_view_tick_ms = int(step_result.get("t_ms", 0))
	if step_result.has("team_view_minimap"):
		team_view_minimap = step_result.get("team_view_minimap", {})
		team_view_tick_ms = int(step_result.get("t_ms", 0))

	var snap = step_result.get("snapshot", {})
	if snap is Dictionary:
		if snap.has("decision_intents"):
			last_decision_intents = snap.get("decision_intents", [])
		else:
			last_decision_intents = []

		if snap.has("offside_lines"):
			last_offside_lines = snap.get("offside_lines", {})
		else:
			last_offside_lines = {}

		last_field_board_snapshot = {}
		for key in ["occupancy_total", "pressure_against_home", "pressure_against_away", "xgzone"]:
			if snap.has(key):
				last_field_board_snapshot[key] = snap[key]


## API: Set position_data
func set_position_data(new_position_data: Dictionary) -> void:
	# Phase20 P0: SSOT probe gate (track_id 0..21)
	if OS.is_debug_build():
		var players_dict: Dictionary = {}
		if new_position_data.has("players") and new_position_data["players"] is Dictionary:
			players_dict = new_position_data["players"]
		_ssot_dbg_keys("UnifiedFramePipeline:set_position_data", players_dict)
		_ssot_assert_track_id_keys("UnifiedFramePipeline:set_position_data", players_dict)

	position_data = new_position_data
	team_view_simple = {}
	team_view_minimap = {}
	team_view_tick_ms = -1
	if replay_smoother != null:
		replay_smoother.reset()

	if OS.is_debug_build():
		var ball_frames = new_position_data.get("ball", []).size()
		print("[UnifiedFramePipeline] Position data set: %d ball frames" % ball_frames)


## API: Set events
func set_event_stream(new_events: Array) -> void:
	events = new_events
	event_index = 0

	if OS.is_debug_build():
		print("[UnifiedFramePipeline] Events set: %d events" % new_events.size())


## API: Set playhead
func set_playhead_ms(t_ms: int) -> void:
	playhead_ms = t_ms

	# Reset event index when seeking backwards
	# Phase20 Bug Fix #8: Increase threshold to 5s (allow showing recent goals after seeking)
	if events.size() > 0 and event_index > 0:
		var prev_raw: Variant = events[max(0, event_index - 1)]
		if prev_raw is Dictionary:
			var prev_norm := _PositionSnapshotAdapter.normalize_event((prev_raw as Dictionary).duplicate(), rosters)
			var prev_event_t_ms := int(prev_norm.get(MatchEventKeys.T_MS, 0))
			if t_ms < prev_event_t_ms - 5000:  # Seeking backwards by more than 5 seconds
				event_index = 0
				if OS.is_debug_build():
					print("[UnifiedFramePipeline] Seek backwards detected, reset event index")
		else:
			event_index = 0
			if OS.is_debug_build():
				print("[UnifiedFramePipeline] Seek backwards detected, reset event index")


## API: Start pipeline
func start() -> void:
	# Phase20 Bug Fix #3: Prevent duplicate start
	if is_running and not is_paused:
		if OS.is_debug_build():
			push_warning("[UnifiedFramePipeline] Pipeline already running, ignoring start()")
		return

	timer.start()
	is_running = true
	is_paused = false

	if OS.is_debug_build():
		print("[UnifiedFramePipeline] Pipeline started (output=%dms)" % OUTPUT_TICK_MS)


## API: Stop pipeline
func stop() -> void:
	# Phase20 Bug Fix #3: Prevent duplicate stop
	if not is_running:
		return

	timer.stop()
	timer.stop()
	is_running = false
	is_paused = false
	position_data = {"ball": [], "players": {}, "field_board_snapshot": []}
	events.clear()
	event_index = 0
	last_emitted_events.clear()
	playhead_ms = 0
	transition_remaining_ms = -1
	team_view_simple = {}
	team_view_minimap = {}
	team_view_tick_ms = -1
	last_decision_intents = []
	last_offside_lines = {}
	last_field_board_snapshot = {}
	if replay_smoother != null:
		replay_smoother.reset()
	if delta_filter != null:
		if OS.is_debug_build() and delta_filter.emit_count + delta_filter.drop_count > 0:
			print("[UnifiedFramePipeline] DeltaFilter stats: emit_ratio=%.2f%%" % (delta_filter.get_emit_ratio() * 100))
		delta_filter.reset()
	if aoi_selector != null:
		if OS.is_debug_build() and aoi_selector.high_priority_count + aoi_selector.low_priority_count > 0:
			var stats = aoi_selector.get_stats()
			print("[UnifiedFramePipeline] AOISelector stats: high_priority_ratio=%.2f%%" % (stats.get("high_priority_ratio", 1.0) * 100))
		aoi_selector.reset()

	if OS.is_debug_build():
		print("[UnifiedFramePipeline] Pipeline stopped")


## Internal: Timer tick (50ms)
func _on_timer_tick() -> void:
	# Phase E: Advance playhead in replay mode
	if is_replay_mode:
		playhead_ms += int(OUTPUT_TICK_MS * playback_speed)

	# Unified: Always use playhead_ms with interpolation delay
	var render_t_ms = playhead_ms - 100
	if render_t_ms < 0:
		return  # Not enough data yet

	var snapshot = _generate_snapshot(render_t_ms)
	if snapshot == null or snapshot.is_empty():
		return

	# Uber Realtime Architecture: DeltaFilter
	# Skip redundant snapshots to reduce processing (session-only by default)
	if enable_delta_filter and delta_filter:
		var should_filter = not is_replay_mode if delta_filter_session_only else true
		if should_filter and not delta_filter.should_emit(snapshot):
			return  # Drop redundant snapshot

	snapshot_ready.emit(render_t_ms, snapshot)


## Internal: Generate snapshot for given time
# Phase E: Variable speed playback controls
func set_playback_speed(speed: float) -> void:
	playback_speed = clamp(speed, 0.25, 4.0)
	print("[UnifiedFramePipeline] Playback speed set to %.2fx" % playback_speed)

func set_replay_mode(enabled: bool) -> void:
	is_replay_mode = enabled
	if enabled:
		print("[UnifiedFramePipeline] Replay mode enabled")
	else:
		print("[UnifiedFramePipeline] Live session mode enabled")

func get_playback_speed() -> float:
	return playback_speed

## API: Pause pipeline (maintains state)
func pause() -> void:
	if not is_running: return
	timer.stop()
	is_paused = true
	if OS.is_debug_build():
		print("[UnifiedFramePipeline] Pipeline paused")

## API: Resume pipeline
func resume() -> void:
	if not is_running: 
		start()
		return
	if is_paused:
		timer.start()
		is_paused = false
		if OS.is_debug_build():
			print("[UnifiedFramePipeline] Pipeline resumed")

## API: Force immediate snapshot emit (for seeking/scrubbing)
func force_update() -> void:
	if rosters.is_empty(): return
	
	# Use current playhead without advancing
	var render_t_ms = playhead_ms # No interpolation delay for static seek? 
	# Original uses playhead_ms - 100. Let's consistency with _on_timer_tick
	# But when paused/scrubbing, we usually want "what is at this exact time".
	# Let's keep the -100 offset if it's the standard for the pipeline
	render_t_ms = playhead_ms - 100
	if render_t_ms < 0: render_t_ms = 0
	
	var snapshot = _generate_snapshot(render_t_ms)
	if snapshot != null and not snapshot.is_empty():
		snapshot_ready.emit(render_t_ms, snapshot)


func _generate_snapshot(t_ms: int) -> Dictionary:
	if rosters.is_empty():
		return {}

	# No mode check - always use position_data
	# Phase20 Bug Fix #2: Validate position_data structure before access
	if not position_data.has("ball") or position_data["ball"].is_empty():
		return {}  # Not enough data yet

	var snapshot = _PositionSnapshotAdapter.from_batch_data(position_data, rosters, t_ms)

	# Phase20 Bug Fix #11: Validate adapter return value
	if snapshot == null or not snapshot is Dictionary:
		if OS.is_debug_build():
			push_warning("[UnifiedFramePipeline] Adapter returned invalid snapshot")
		return {}

	# Enrich with FieldBoard
	_enrich_field_board(snapshot, t_ms)

	if not last_field_board_snapshot.is_empty():
		for key in last_field_board_snapshot.keys():
			snapshot[key] = last_field_board_snapshot[key]
	if not last_decision_intents.is_empty():
		snapshot["decision_intents"] = last_decision_intents
	if not last_offside_lines.is_empty():
		snapshot["offside_lines"] = last_offside_lines

	# Phase20 Step E: Add events to snapshot
	snapshot["events"] = _collect_events_for_time(t_ms)

	# ✅ P0-2 (2025-12-22 FIX_2512): Filter out invalid events (prevents UI/camera pollution)
	if snapshot.has("events"):
		var valid_events = []
		for event in snapshot.get("events", []):
			if _is_valid_event(event):
				valid_events.append(event)
			elif OS.is_debug_build():
				print(
					(
						"[UnifiedFramePipeline] Filtered invalid event: type=%s, track_id=%s"
						% [
							str(event.get("kind", event.get("type", ""))),
							str(event.get(MatchEventKeys.PLAYER_TRACK_ID, -1))
						]
					)
				)
		snapshot["events"] = valid_events

	# Phase23: TransitionSystem debug field (optional; -1 = inactive)
	snapshot["transition_remaining_ms"] = transition_remaining_ms

	if not team_view_simple.is_empty():
		snapshot["team_view_simple"] = team_view_simple
	if not team_view_minimap.is_empty():
		snapshot["team_view_minimap"] = team_view_minimap

	# Uber Realtime Architecture: AOISelector
	# Add AOI metadata to snapshot (high/low priority player lists)
	if enable_aoi_selector and aoi_selector:
		var should_apply = not is_replay_mode if aoi_selector_session_only else true
		if should_apply:
			var ball_data = snapshot.get("ball", {})
			var ball_pos = ball_data.get("pos", Vector2.ZERO)
			var players = snapshot.get("players", {})
			if not players.is_empty() and ball_pos != Vector2.ZERO:
				snapshot = aoi_selector.filter_snapshot(snapshot, ball_pos)

	# FIX_2512 (B Layer): apply visual smoothing (timeline-only by default)
	if enable_smoothing and replay_smoother != null:
		var is_timeline_data: bool = false
		if position_data.has("ball") and position_data["ball"] is Array:
			is_timeline_data = (position_data["ball"] as Array).size() > 20
		if (not smoothing_timeline_only) or is_timeline_data:
			snapshot = replay_smoother.apply(snapshot)

	return snapshot


## Internal: Convert Session tick to position_data format
func _convert_session_tick_to_position_data(step_result: Dictionary) -> void:
	var t_ms = step_result.get("t_ms", 0)
	var t_sec = float(t_ms) / 1000.0  # CRITICAL: adapter expects seconds

	# ✅ Gap #2 Fix: Read from snapshot (nested structure)
	var snap = step_result.get("snapshot", {})

	# ✅ Fix #10: Validate snapshot
	if snap.is_empty():
		push_warning("[UnifiedFramePipeline] Empty snapshot in step_result")
		return

	var ball = snap.get("ball", {})

	# ✅ Fix #10: Validate ball data
	if ball.is_empty():
		push_warning("[UnifiedFramePipeline] No ball data in snapshot")
		return

	var ball_x = ball.get("x", 0.0)
	var ball_y = ball.get("y", 0.0)
	var ball_z = ball.get("z", 0.0)

	# ✅ NEW: Extract owner_id (Session-only field)
	var ball_owner_id = ball.get("owner_id", -1)

	# Compute ball velocity from previous sample
	var ball_vx = 0.0
	var ball_vy = 0.0
	if position_data["ball"].size() > 0:
		var prev = position_data["ball"][-1]
		var dt_sec = t_sec - prev["t"]
		if dt_sec > 0.001:  # Avoid division by zero
			ball_vx = (ball_x - prev["x"]) / dt_sec
			ball_vy = (ball_y - prev["y"]) / dt_sec

	position_data["ball"].append(
		{"t": t_sec, "x": ball_x, "y": ball_y, "z": ball_z, "vx": ball_vx, "vy": ball_vy, "owner_id": ball_owner_id}  # ✅ NEW: Preserve for adapter
	)

	# ✅ Gap #4 Fix: Check for packed vs unpacked format
	if snap.has("players_packed"):
		# Packed format (88 floats)
		_parse_players_packed(snap, t_sec)
	elif snap.has("players"):
		# Existing unpacked format
		_parse_players_unpacked(snap, t_sec)
	else:
		push_warning("[UnifiedFramePipeline] No player data in step_result")

	if snap.has("occupancy_total"):
		if not position_data.has("field_board_snapshot"):
			position_data["field_board_snapshot"] = []

		var fb_entry: Dictionary = {"t": t_sec, "occupancy_total": snap.get("occupancy_total", PackedFloat32Array())}
		if snap.has("pressure_against_home"):
			fb_entry["pressure_against_home"] = snap.get("pressure_against_home")
		if snap.has("pressure_against_away"):
			fb_entry["pressure_against_away"] = snap.get("pressure_against_away")
		if snap.has("xgzone"):
			fb_entry["xgzone"] = snap.get("xgzone")

		position_data["field_board_snapshot"].append(fb_entry)
		if position_data["field_board_snapshot"].size() > 4:
			position_data["field_board_snapshot"].pop_front()

	# Trim to 4 ticks (ring buffer)
	# Phase20 Bug Fix #10: Trimming is safe - Godot doesn't interrupt _process() mid-execution
	# Ball and player buffers are trimmed separately but within the same frame
	if position_data["ball"].size() > 4:
		# Phase20 Bug Fix #13: Debug logging for ring buffer trimming
		if OS.is_debug_build():
			var dropped = position_data["ball"][0]
			print("[UnifiedFramePipeline] Trimmed ball sample: t=%.3f" % dropped.get("t", 0.0))
		position_data["ball"].pop_front()

	for track_id_str in position_data["players"].keys():
		var track = position_data["players"][track_id_str]
		if track.size() > 4:
			track.pop_front()


## Gap #4 Helper: Parse unpacked player format (Dictionary)
func _parse_players_unpacked(snap: Dictionary, t_sec: float) -> void:
	var players = snap.get("players", {})
	for track_id_str in players.keys():
		var p = players[track_id_str]

		if not (p is Dictionary):
			continue

		if track_id_str not in position_data["players"]:
			position_data["players"][track_id_str] = []

		var vx = float(p.get("vx", 0.0))
		var vy = float(p.get("vy", 0.0))

		position_data["players"][track_id_str].append(
			{
				"t": t_sec,
				"x": p.get("x", 0.0),
				"y": p.get("y", 0.0),
				"vx": vx,
				"vy": vy,
				"state": p.get("state", ""),
				"stamina": p.get("stamina", 1.0)
			}
		)


## Gap #4 Helper: Parse packed player format (PackedFloat32Array)
func _parse_players_packed(snap: Dictionary, t_sec: float) -> void:
	var packed: PackedFloat32Array = snap.get("players_packed", PackedFloat32Array())
	var stamina_packed: PackedFloat32Array = snap.get("stamina_packed", PackedFloat32Array())
	var states: Dictionary = snap.get("states", {})

	# 88 floats = 22 players × 4 values (x, y, vx, vy)
	var num_players: int = min(22, int(packed.size() / 4))

	for i in range(num_players):
		var base_idx = i * 4
		if base_idx + 3 >= packed.size():
			break

		var track_id_str = str(i)

		if track_id_str not in position_data["players"]:
			position_data["players"][track_id_str] = []

		var x = packed[base_idx]
		var y = packed[base_idx + 1]
		var vx = packed[base_idx + 2]
		var vy = packed[base_idx + 3]
		var stamina = stamina_packed[i] if i < stamina_packed.size() else 1.0
		var state = states.get(track_id_str, "")

		position_data["players"][track_id_str].append(
			{"t": t_sec, "x": x, "y": y, "vx": vx, "vy": vy, "state": state, "stamina": stamina}
		)


## Internal: Enrich snapshot with FieldBoard data
func _enrich_field_board(snapshot: Dictionary, t_ms: int) -> void:
	if not field_board:
		return

	# No mode check - always try to get field_board_snapshot
	var field_board_snapshots = position_data.get("field_board_snapshot", [])
	if field_board_snapshots.is_empty():
		return

	var t_sec = t_ms / 1000.0
	var best_idx = -1
	var best_diff = INF

	for i in range(field_board_snapshots.size()):
		var fb_t = field_board_snapshots[i].get("t", 0.0)
		var diff = abs(fb_t - t_sec)
		if diff < best_diff:
			best_diff = diff
			best_idx = i

	# Phase20 Bug Fix #7: Validate FieldBoard methods before calling
	if best_idx >= 0 and best_diff < 1.0:  # Within 1 second
		if field_board.has_method("load_from_snapshot") and field_board.has_method("to_overlay_snapshot"):
			field_board.load_from_snapshot(field_board_snapshots[best_idx])
			snapshot["field_board"] = field_board.to_overlay_snapshot()
		else:
			if OS.is_debug_build():
				push_warning("[UnifiedFramePipeline] FieldBoard missing required methods")


## Phase20 Step E: Collect events for given time (UNIFIED)
func _collect_events_for_time(t_ms: int) -> Array:
	var events_to_emit: Array = []

	# Phase20 Bug Fix #6: Clamp event_index to prevent out-of-bounds access
	event_index = min(event_index, events.size())

	# Time window filtering (±500ms)
	while event_index < events.size():
		var event = events[event_index]

		# Peek at time to check window (normalize-only; keeps raw event untouched)
		var event_t_ms: int = 0
		if event is Dictionary:
			var peek_norm := _PositionSnapshotAdapter.normalize_event(event.duplicate(), rosters)
			event_t_ms = int(peek_norm.get(MatchEventKeys.T_MS, 0))

		if event_t_ms > t_ms + 500:
			break  # Too far in future
		if event_t_ms >= t_ms - 500:
			# Duplicate to avoid side effects
			var event_copy = event.duplicate()

			# Normalize (minute → t_ms auto-conversion)
			var normalized = _PositionSnapshotAdapter.normalize_event(event_copy, rosters)

			# FIX_2512: Skip null/invalid events before dedupe (prevents camera/UI pollution)
			var event_type_norm: String = str(normalized.get(MatchEventKeys.TYPE, "")).strip_edges().to_lower()
			if event_type_norm == "" or event_type_norm == "unknown" or event_type_norm == "n/a":
				event_index += 1
				continue

			# Dedup check
			# Event SSOT: prefer (type, t_ms, player_track_id, target_track_id)
			var event_type = event_type_norm
			var norm_t_ms = int(normalized.get(MatchEventKeys.T_MS, 0))
			var player_tid = int(normalized.get(MatchEventKeys.PLAYER_TRACK_ID, -1))
			var target_tid = int(normalized.get(MatchEventKeys.TARGET_TRACK_ID, -1))

			# If it's an action-like event but has no actor AND no meaningful position, drop it.
			# normalize_event always provides "pos" (Vector2), but missing source becomes Vector2.ZERO.
			var pos_v: Vector2 = normalized.get("pos", Vector2.ZERO)
			var has_pos: bool = pos_v != Vector2.ZERO
			var action_like: bool = (
				event_type in ["pass", "dribble", "shot", "shot_on_target", "shot_off_target", "tackle", "save", "goal"]
			)
			if action_like and player_tid < 0 and target_tid < 0 and not has_pos:
				event_index += 1
				continue

			# C7: Legacy name-based fallback removed. Event identity is always track_id-based.
			var event_id = "%s_%d_%d_%d" % [event_type, norm_t_ms, player_tid, target_tid]

			# Phase20 Bug Fix #12: O(1) Dictionary lookup instead of O(n) Array scan
			if not last_emitted_events.has(event_id):
				events_to_emit.append(normalized)
				last_emitted_events[event_id] = true

		event_index += 1

	# Trim dedup buffer
	# Phase20 Bug Fix #5/#12: Trim Dictionary keys more aggressively
	if last_emitted_events.size() > 50:
		var keys = last_emitted_events.keys()
		last_emitted_events.clear()
		# Keep last 25 keys
		for i in range(max(0, keys.size() - 25), keys.size()):
			last_emitted_events[keys[i]] = true

	return events_to_emit


## ✅ P0-2 (2025-12-22 FIX_2512): Validate event before emission
## Returns true if event has valid type/kind AND actor (track_id or player_id)
func _is_valid_event(event: Dictionary) -> bool:
	# Must have type/kind
	var action_type = str(event.get("kind", event.get("type", "")))
	if action_type == "" or action_type == "null":
		return false

	# Must have actor (track_id or player_id)
	var track_id = event.get(MatchEventKeys.PLAYER_TRACK_ID, -1)
	var player_id = event.get("player_id", event.get("base", {}).get("player_id", null))

	if track_id < 0 and (player_id == null or str(player_id) == ""):
		return false

	return true


# ============================================================================
# Phase20 P0: Track-ID SSOT probe helpers (debug-only usage recommended)
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


static func _ssot_assert_track_id_keys(tag: String, players_dict: Dictionary) -> void:
	if players_dict.is_empty():
		return

	if players_dict.size() != 22:
		push_error("[SSOT] %s players.size != 22: %d" % [tag, players_dict.size()])

	for k in players_dict.keys():
		var s := str(k)
		if not s.is_valid_int():
			push_error("[SSOT] %s non-int key: %s" % [tag, str(k)])
			continue
		var i := int(s)
		if i < 0 or i > 21:
			push_error("[SSOT] %s key out of range (expect 0..21): %d" % [tag, i])
