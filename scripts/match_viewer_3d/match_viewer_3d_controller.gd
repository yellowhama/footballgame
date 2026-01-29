extends Node
class_name MatchViewer3DController

## Controller that connects MatchTimelineController/UnifiedFramePipeline to MatchViewer3D.
## Handles snapshot updates and converts them to 3D viewer format.

signal viewer_ready
signal playback_started(duration_ms: int)
signal playback_stopped
signal time_updated(time_ms: int)

const MatchViewer3DBridge := preload("res://scripts/match_viewer_3d/match_viewer_3d_bridge.gd")

## Reference to MatchViewer3D node (set via Inspector or find at runtime)
@export var viewer_path: NodePath = ^""
@export var auto_find_viewer: bool = true

## Reference to timeline controller (autoload)
@export var use_timeline_controller: bool = true

var _viewer: MatchViewer3D = null
var _timeline_controller: Node = null
var _unified_pipeline: Node = null
var _rosters: Dictionary = {}
var _match_info: Dictionary = {}
var _best_moments: Array = []
var _is_initialized: bool = false


func _ready() -> void:
	call_deferred("_initialize")


func _initialize() -> void:
	# Find viewer
	if not viewer_path.is_empty():
		_viewer = get_node_or_null(viewer_path) as MatchViewer3D
	elif auto_find_viewer:
		_viewer = _find_viewer_in_scene()

	if not _viewer:
		push_warning("[MatchViewer3DController] MatchViewer3D not found")
		return

	# Find timeline controller
	if use_timeline_controller:
		_timeline_controller = get_node_or_null("/root/MatchTimelineController")
		if _timeline_controller:
			_connect_timeline_controller()

	# Find unified pipeline
	_unified_pipeline = get_node_or_null("/root/UnifiedFramePipeline")
	if _unified_pipeline:
		_connect_unified_pipeline()

	_is_initialized = true
	viewer_ready.emit()
	print("[MatchViewer3DController] Initialized with viewer: %s" % _viewer.name)


func _find_viewer_in_scene() -> MatchViewer3D:
	# Search for MatchViewer3D in common locations
	var search_paths: Array[String] = [
		"/root/MatchViewer3D",
		"/root/Main/MatchViewer3D",
		"/root/GameScene/MatchViewer3D",
	]
	for path in search_paths:
		var node := get_node_or_null(path) as MatchViewer3D
		if node:
			return node

	# Search in current scene tree
	var root := get_tree().current_scene
	if root:
		return _find_node_of_type(root, "MatchViewer3D") as MatchViewer3D
	return null


func _find_node_of_type(node: Node, type_name: String) -> Node:
	if node.get_class() == type_name or (node.get_script() and node.get_script().get_global_name() == type_name):
		return node
	for child in node.get_children():
		var result := _find_node_of_type(child, type_name)
		if result:
			return result
	return null


func _connect_timeline_controller() -> void:
	if not _timeline_controller:
		return

	if _timeline_controller.has_signal("position_playback_started"):
		_timeline_controller.connect("position_playback_started", _on_playback_started)
	if _timeline_controller.has_signal("position_playback_stopped"):
		_timeline_controller.connect("position_playback_stopped", _on_playback_stopped)


func _connect_unified_pipeline() -> void:
	if not _unified_pipeline:
		return

	if _unified_pipeline.has_signal("snapshot_ready"):
		_unified_pipeline.connect("snapshot_ready", _on_snapshot_ready)


func _on_playback_started(duration_ms: int) -> void:
	playback_started.emit(duration_ms)


func _on_playback_stopped() -> void:
	playback_stopped.emit()


func _on_snapshot_ready(snapshot: Dictionary) -> void:
	if not _viewer:
		return

	# Convert StandardSnapshot to MatchViewer3D frame format
	var frame := MatchViewer3DBridge.convert_snapshot_to_frame(snapshot)
	if frame.is_empty():
		return

	# Apply frame to viewer (update current positions)
	_apply_frame_to_viewer(frame)

	# Emit time update
	var time_ms: int = frame.get("time_ms", 0)
	time_updated.emit(time_ms)


func _apply_frame_to_viewer(frame: Dictionary) -> void:
	if not _viewer:
		return

	# Update ball position
	var ball_pos: Vector2 = frame.get("ball", Vector2(52.5, 34.0))
	if _viewer.has_node("%Ball"):
		var ball := _viewer.get_node("%Ball")
		ball.global_position = _field_to_world(ball_pos)

	# Update player positions
	var players: Array = frame.get("players", [])
	for i in range(mini(players.size(), 22)):
		var player_data: Dictionary = players[i]
		var pos: Vector2 = player_data.get("position", Vector2(52.5, 34.0))
		var pose: Variant = player_data.get("pose", "idle")  # PoseBuilder.PoseType int or string

		if i < _viewer._players.size():
			var player: Node3D = _viewer._players[i]
			if player.has_method("set_target_position"):
				player.set_target_position(_field_to_world(pos))
			if player.has_method("play_pose"):
				player.play_pose(pose)


func _field_to_world(field_pos: Vector2) -> Vector3:
	# Center the field at origin (matches MatchViewer3D._field_to_world)
	var x := field_pos.x - 52.5  # FIELD_LENGTH / 2
	var z := field_pos.y - 34.0  # FIELD_WIDTH / 2
	return Vector3(x, 0.0, z)


## Load full replay data for the 3D viewer
func load_replay_data(
	position_data: Dictionary, rosters: Dictionary, match_info: Dictionary, best_moments: Array = []
) -> void:
	_rosters = rosters.duplicate(true) if rosters is Dictionary else {}
	_match_info = match_info.duplicate(true) if match_info is Dictionary else {}
	_best_moments = best_moments.duplicate(true) if best_moments is Array else []

	if not _viewer:
		push_warning("[MatchViewer3DController] No viewer to load data into")
		return

	# Convert to 3D viewer format
	var replay_data := MatchViewer3DBridge.convert_timeline_to_3d(position_data, _rosters, _match_info, _best_moments)

	# Apply team colors
	var colors := MatchViewer3DBridge.extract_team_colors(_rosters)
	_viewer.set_home_team_colors(colors.get("home_shirt", Color.RED), colors.get("home_shorts", Color.WHITE))
	_viewer.set_away_team_colors(colors.get("away_shirt", Color.BLUE), colors.get("away_shorts", Color.WHITE))

	# Load into viewer
	_viewer.load_replay_data(replay_data)

	print(
		(
			"[MatchViewer3DController] Loaded replay: %d frames, %d best moments"
			% [replay_data.get("frames", []).size(), _best_moments.size()]
		)
	)


## Set viewer reference manually
func set_viewer(viewer: MatchViewer3D) -> void:
	_viewer = viewer
	if _viewer:
		print("[MatchViewer3DController] Viewer set: %s" % _viewer.name)


## Get current viewer
func get_viewer() -> MatchViewer3D:
	return _viewer


## Playback controls (delegate to viewer)
func play() -> void:
	if _viewer:
		_viewer.play()


func pause() -> void:
	if _viewer:
		_viewer.pause()


func stop() -> void:
	if _viewer:
		_viewer.stop()


func seek(time_ms: int) -> void:
	if _viewer:
		_viewer.seek(time_ms)


func set_speed(multiplier: float) -> void:
	if _viewer:
		_viewer.set_speed(multiplier)
