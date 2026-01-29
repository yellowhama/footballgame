extends Control
class_name MatchViewer3DUIIntegration

## Integration layer between MatchTimelinePanel and MatchViewer3D.
## Handles switching between 2D minimap and 3D viewer, synchronizing playback state.

signal viewer_mode_changed(is_3d: bool)
signal playback_started
signal playback_stopped

const MatchViewer3DScene := preload("res://scenes/match_viewer_3d/MatchViewer3D.tscn")
const MatchViewer3DBridgeScript := preload("res://scripts/match_viewer_3d/match_viewer_3d_bridge.gd")
const HighlightPlayerScript := preload("res://scripts/match_viewer_3d/highlight_player.gd")
const ViewerModeToggleScript := preload("res://scripts/ui/viewer_mode_toggle.gd")

const TIMELINE_CONTROLLER_PATH := "/root/MatchTimelineController"
const UNIFIED_FRAME_PIPELINE_PATH := "/root/UnifiedFramePipeline"

## Configuration
@export var enable_highlight_mode: bool = true
@export var auto_switch_to_3d_for_highlights: bool = false

## Node references (set in scene or via code)
var _viewer_2d_container: Control = null
var _viewer_3d_container: SubViewportContainer = null
var _mode_toggle: Control = null  # ViewerModeToggle

## Internal state
var _match_viewer_3d: Node3D = null
var _highlight_player: Node = null
var _timeline_controller: Node = null
var _frame_pipeline: Node = null
var _is_3d_mode: bool = false
var _match_data_loaded: bool = false
var _current_record: Dictionary = {}
var _current_rosters: Dictionary = {}
var _best_moments: Array = []


func _ready() -> void:
	_attach_controllers()


func _attach_controllers() -> void:
	_timeline_controller = get_node_or_null(TIMELINE_CONTROLLER_PATH)
	_frame_pipeline = get_node_or_null(UNIFIED_FRAME_PIPELINE_PATH)

	if _frame_pipeline and _frame_pipeline.has_signal("snapshot_ready"):
		if not _frame_pipeline.snapshot_ready.is_connected(_on_unified_snapshot):
			_frame_pipeline.snapshot_ready.connect(_on_unified_snapshot)


## ============================================================================
## Setup API
## ============================================================================


func setup_containers(viewer_2d: Control, viewer_3d: SubViewportContainer, toggle: Control) -> void:
	_viewer_2d_container = viewer_2d
	_viewer_3d_container = viewer_3d
	_mode_toggle = toggle

	if _mode_toggle and _mode_toggle.has_signal("mode_changed"):
		_mode_toggle.mode_changed.connect(_on_mode_toggle_changed)

	# Initialize 3D viewer
	_spawn_3d_viewer()

	# Start in 2D mode
	_set_viewer_visibility(false)


func _spawn_3d_viewer() -> void:
	if not _viewer_3d_container or _match_viewer_3d:
		return

	# Create SubViewport for 3D rendering
	var viewport := SubViewport.new()
	viewport.name = "Viewer3DViewport"
	viewport.size = Vector2i(1280, 720)
	viewport.render_target_update_mode = SubViewport.UPDATE_WHEN_VISIBLE
	viewport.handle_input_locally = true
	_viewer_3d_container.add_child(viewport)

	# Instantiate 3D viewer
	if MatchViewer3DScene:
		_match_viewer_3d = MatchViewer3DScene.instantiate()
		viewport.add_child(_match_viewer_3d)

		# Connect signals
		if _match_viewer_3d.has_signal("playback_started"):
			_match_viewer_3d.playback_started.connect(_on_3d_playback_started)
		if _match_viewer_3d.has_signal("playback_stopped"):
			_match_viewer_3d.playback_stopped.connect(_on_3d_playback_stopped)
		if _match_viewer_3d.has_signal("best_moment_reached"):
			_match_viewer_3d.best_moment_reached.connect(_on_best_moment_reached)

	# Create highlight player
	if enable_highlight_mode:
		_highlight_player = HighlightPlayerScript.new()
		_highlight_player.name = "HighlightPlayer"
		add_child(_highlight_player)
		if _match_viewer_3d:
			_highlight_player.setup(_match_viewer_3d)


func _set_viewer_visibility(show_3d: bool) -> void:
	_is_3d_mode = show_3d

	if _viewer_2d_container:
		_viewer_2d_container.visible = not show_3d
	if _viewer_3d_container:
		_viewer_3d_container.visible = show_3d

	viewer_mode_changed.emit(show_3d)


## ============================================================================
## Data Loading API
## ============================================================================


func load_match_data(record: Dictionary) -> void:
	_current_record = record.duplicate(true)
	_current_rosters = _extract_rosters(record)
	_best_moments = _extract_best_moments(record)
	_match_data_loaded = true

	# Convert and load into 3D viewer
	if _match_viewer_3d:
		var match_info := _extract_match_info(record)
		var position_data := _extract_position_data(record)

		var viewer_data := MatchViewer3DBridgeScript.convert_timeline_to_3d(
			position_data, _current_rosters, match_info, _best_moments
		)

		_match_viewer_3d.load_replay_data(viewer_data)

		# Load team colors
		var colors := MatchViewer3DBridgeScript.extract_team_colors(_current_rosters)
		if colors.has("home_shirt"):
			_match_viewer_3d.set_home_team_colors(colors.home_shirt, colors.get("home_shorts", Color.WHITE))
		if colors.has("away_shirt"):
			_match_viewer_3d.set_away_team_colors(colors.away_shirt, colors.get("away_shorts", Color.WHITE))

	# Load highlights
	if _highlight_player and not _best_moments.is_empty():
		_highlight_player.load_moments(_best_moments)

	print("[MatchViewer3DUIIntegration] Match data loaded (%d best moments)" % _best_moments.size())


func _extract_rosters(record: Dictionary) -> Dictionary:
	var candidate_keys := ["timeline_rosters", "rosters", "replay_rosters"]
	for key in candidate_keys:
		var variant: Variant = record.get(key, null)
		if variant is Dictionary and not (variant as Dictionary).is_empty():
			return (variant as Dictionary).duplicate(true)

	var match_result: Variant = record.get("match_result", {})
	if match_result is Dictionary and match_result.has("rosters"):
		var rosters: Variant = match_result.get("rosters")
		if rosters is Dictionary:
			return (rosters as Dictionary).duplicate(true)

	return {}


func _extract_best_moments(record: Dictionary) -> Array:
	# Direct field
	var direct: Variant = record.get("best_moments", null)
	if direct is Array and not (direct as Array).is_empty():
		return (direct as Array).duplicate(true)

	# From match_result
	var match_result: Variant = record.get("match_result", {})
	if match_result is Dictionary:
		var moments: Variant = match_result.get("best_moments", null)
		if moments is Array and not (moments as Array).is_empty():
			return (moments as Array).duplicate(true)

	# From raw_result
	var raw_result: Variant = record.get("raw_result", {})
	if raw_result is Dictionary:
		var moments: Variant = raw_result.get("best_moments", null)
		if moments is Array and not (moments as Array).is_empty():
			return (moments as Array).duplicate(true)

	return []


func _extract_match_info(record: Dictionary) -> Dictionary:
	return {
		"home_team": str(record.get("home_team_name", record.get("home_team", "Home"))),
		"away_team": str(record.get("opponent_name", record.get("away_team", "Away"))),
		"home_score": int(record.get("home_score", record.get("goals_scored", 0))),
		"away_score": int(record.get("away_score", record.get("goals_conceded", 0))),
	}


func _extract_position_data(record: Dictionary) -> Dictionary:
	var candidate_keys := ["position_data", "timeline_positions", "replay"]
	for key in candidate_keys:
		var variant: Variant = record.get(key, null)
		if variant is Dictionary and not (variant as Dictionary).is_empty():
			return (variant as Dictionary).duplicate(true)

	var timeline_doc: Variant = record.get("timeline_doc", {})
	if timeline_doc is Dictionary and timeline_doc.has("position_data"):
		return (timeline_doc.get("position_data") as Dictionary).duplicate(true)

	return {}


## ============================================================================
## Playback Control API
## ============================================================================


func play() -> void:
	if _is_3d_mode and _match_viewer_3d:
		_match_viewer_3d.play()
	# 2D playback is handled by MatchTimelineController


func pause() -> void:
	if _is_3d_mode and _match_viewer_3d:
		_match_viewer_3d.pause()


func stop() -> void:
	if _is_3d_mode and _match_viewer_3d:
		_match_viewer_3d.stop()


func seek(time_ms: int) -> void:
	if _is_3d_mode and _match_viewer_3d:
		_match_viewer_3d.seek(time_ms)


func set_speed(multiplier: float) -> void:
	if _match_viewer_3d:
		_match_viewer_3d.set_speed(multiplier)


func play_highlights() -> void:
	if not _highlight_player:
		return

	# Switch to 3D mode for highlights
	if auto_switch_to_3d_for_highlights and not _is_3d_mode:
		switch_to_3d()

	_highlight_player.play()


func stop_highlights() -> void:
	if _highlight_player:
		_highlight_player.stop()


## ============================================================================
## Mode Switching
## ============================================================================


func switch_to_2d() -> void:
	_set_viewer_visibility(false)
	if _mode_toggle and _mode_toggle.has_method("set_mode"):
		_mode_toggle.set_mode(0, false)  # ViewerMode.VIEWER_2D


func switch_to_3d() -> void:
	_set_viewer_visibility(true)
	if _mode_toggle and _mode_toggle.has_method("set_mode"):
		_mode_toggle.set_mode(1, false)  # ViewerMode.VIEWER_3D

	# Sync current time from timeline controller
	if _timeline_controller and _match_viewer_3d:
		var current_time: int = 0
		if _timeline_controller.has_method("get_current_time_ms"):
			current_time = _timeline_controller.get_current_time_ms()
		elif _timeline_controller.get("position_current_time_ms") != null:
			current_time = int(_timeline_controller.position_current_time_ms)
		_match_viewer_3d.seek(current_time)


func toggle_viewer_mode() -> void:
	if _is_3d_mode:
		switch_to_2d()
	else:
		switch_to_3d()


func is_3d_mode() -> bool:
	return _is_3d_mode


## ============================================================================
## Camera Control API (3D only)
## ============================================================================


func set_camera_mode(mode: int) -> void:
	if _match_viewer_3d:
		_match_viewer_3d.set_camera_mode(mode)


func cycle_camera_mode() -> void:
	if _match_viewer_3d:
		_match_viewer_3d.cycle_camera_mode()


func set_camera_broadcast() -> void:
	if _match_viewer_3d:
		_match_viewer_3d.set_camera_broadcast()


func set_camera_tactical() -> void:
	if _match_viewer_3d:
		_match_viewer_3d.set_camera_tactical()


func camera_follow_ball() -> void:
	if _match_viewer_3d:
		_match_viewer_3d.set_camera_follow_ball()


func camera_follow_player(player_index: int) -> void:
	if _match_viewer_3d:
		_match_viewer_3d.camera_follow_player(player_index)


## ============================================================================
## Signal Handlers
## ============================================================================


func _on_mode_toggle_changed(mode: int) -> void:
	# mode 0 = 2D, mode 1 = 3D
	if mode == 1:
		switch_to_3d()
	else:
		switch_to_2d()


func _on_unified_snapshot(t_ms: int, snapshot: Dictionary) -> void:
	# Sync 3D viewer with 2D timeline when in 3D mode
	if _is_3d_mode and _match_viewer_3d:
		# Apply snapshot directly to 3D viewer for real-time sync
		var frame := MatchViewer3DBridgeScript.convert_snapshot_to_frame(snapshot)
		if _match_viewer_3d.has_method("_apply_frame"):
			_match_viewer_3d._apply_frame(0)  # Current frame
		# Alternative: just seek to time
		# _match_viewer_3d.seek(t_ms)


func _on_3d_playback_started() -> void:
	playback_started.emit()


func _on_3d_playback_stopped() -> void:
	playback_stopped.emit()


func _on_best_moment_reached(moment: Dictionary) -> void:
	var moment_type := str(moment.get("moment_type", ""))
	var minute := int(moment.get("minute", 0))
	print("[MatchViewer3DUIIntegration] Best moment: %s at %d'" % [moment_type, minute])


## ============================================================================
## Input Handling (3D camera controls)
## ============================================================================


func _input(event: InputEvent) -> void:
	if not _is_3d_mode:
		return

	# Camera mode shortcuts (only in 3D mode)
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_1:
				set_camera_broadcast()
				get_viewport().set_input_as_handled()
			KEY_2:
				camera_follow_ball()
				get_viewport().set_input_as_handled()
			KEY_3:
				set_camera_tactical()
				get_viewport().set_input_as_handled()
			KEY_C:
				cycle_camera_mode()
				get_viewport().set_input_as_handled()
			KEY_H:
				if enable_highlight_mode:
					play_highlights()
					get_viewport().set_input_as_handled()
