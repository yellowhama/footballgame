extends Node3D
class_name MatchViewer3D

## Main controller for 3D match visualization.
## Manages 22 players, ball, camera, and replay playback.

signal playback_started
signal playback_stopped
signal time_changed(time_ms: int)
signal best_moment_reached(moment: Dictionary)

const PLAYER_SCENE := preload("res://scenes/match_viewer_3d/AnimatedPlayer.tscn")
const CameraController3DScript := preload("res://scripts/match_viewer_3d/camera_controller_3d.gd")

# Field dimensions (meters)
const FIELD_LENGTH := 105.0
const FIELD_WIDTH := 68.0

# Tick rate (engine simulation)
const TICK_INTERVAL_MS := 50  # 50ms = 20 ticks/second

# Node references
@onready var _ball: Node3D = %Ball
@onready var _home_team: Node3D = %HomeTeam
@onready var _away_team: Node3D = %AwayTeam
@onready var _camera: Camera3D = %Camera3D
@onready var _score_label: Label = %ScoreLabel
@onready var _time_label: Label = %TimeLabel

# Camera controller
var _camera_controller: Node = null

# Players array (0-10 = home, 11-21 = away)
var _players: Array[Node3D] = []

# Playback state
var _is_playing: bool = false
var _playback_speed: float = 1.0
var _current_time_ms: int = 0
var _total_duration_ms: int = 0

# Replay data
var _position_data: Array = []  # Array of frame dictionaries
var _best_moments: Array = []  # BestMoment markers
var _current_frame_index: int = 0

# Team info
var _home_team_name: String = "Home"
var _away_team_name: String = "Away"
var _home_score: int = 0
var _away_score: int = 0

# Team colors
@export var home_shirt_color: Color = Color(0.9, 0.1, 0.1)
@export var away_shirt_color: Color = Color(0.1, 0.1, 0.9)


func _ready() -> void:
	_spawn_players()
	_setup_camera_controller()
	_update_score_display()


func _setup_camera_controller() -> void:
	# Create camera controller
	_camera_controller = CameraController3DScript.new()
	_camera_controller.name = "CameraController3D"
	add_child(_camera_controller)

	# Configure camera controller
	_camera_controller.set_camera(_camera)
	_camera_controller.set_ball(_ball)
	_camera_controller.set_players(_players)
	_camera_controller.enable_auto_camera(true)

	# Connect best moment signal
	best_moment_reached.connect(_camera_controller.on_best_moment_start)


func _process(delta: float) -> void:
	if _is_playing:
		_advance_playback(delta)


## Spawn 22 player instances
func _spawn_players() -> void:
	_players.clear()

	# Home team (11 players)
	for i in range(11):
		var player := PLAYER_SCENE.instantiate() as Node3D
		_home_team.add_child(player)
		if player.has_method("setup"):
			player.setup(i, true)
		if player.has_method("set_team_colors"):
			player.set_team_colors(home_shirt_color, Color.WHITE)
		_players.append(player)

	# Away team (11 players)
	for i in range(11):
		var player := PLAYER_SCENE.instantiate() as Node3D
		_away_team.add_child(player)
		if player.has_method("setup"):
			player.setup(i + 11, false)
		if player.has_method("set_team_colors"):
			player.set_team_colors(away_shirt_color, Color.WHITE)
		_players.append(player)


## Load replay data
func load_replay_data(data: Dictionary) -> void:
	# Extract position frames
	_position_data = data.get("frames", data.get("positions", []))
	_total_duration_ms = data.get("duration_ms", 0)

	# Extract best moments
	_best_moments = data.get("best_moments", [])

	# Team info
	_home_team_name = data.get("home_team", "Home")
	_away_team_name = data.get("away_team", "Away")
	_home_score = data.get("home_score", 0)
	_away_score = data.get("away_score", 0)

	# Reset playback
	_current_time_ms = 0
	_current_frame_index = 0
	_is_playing = false

	_update_score_display()
	_apply_frame(0)


## Load best moments for timeline markers
func load_best_moments(moments: Array) -> void:
	_best_moments = moments


## Start playback
func play() -> void:
	if _position_data.is_empty():
		return
	_is_playing = true
	playback_started.emit()


## Pause playback
func pause() -> void:
	_is_playing = false


## Stop and reset
func stop() -> void:
	_is_playing = false
	_current_time_ms = 0
	_current_frame_index = 0
	_apply_frame(0)
	playback_stopped.emit()


## Seek to specific time
func seek(time_ms: int) -> void:
	_current_time_ms = clamp(time_ms, 0, _total_duration_ms)
	_current_frame_index = _find_frame_index(_current_time_ms)
	_apply_frame(_current_frame_index)
	time_changed.emit(_current_time_ms)
	_check_best_moments()


## Set playback speed
func set_speed(multiplier: float) -> void:
	_playback_speed = clamp(multiplier, 0.1, 4.0)


## Get current playback time
func get_current_time_ms() -> int:
	return _current_time_ms


## Get total duration
func get_total_duration_ms() -> int:
	return _total_duration_ms


## Check if playing
func is_playing() -> bool:
	return _is_playing


func _advance_playback(delta: float) -> void:
	var advance_ms := int(delta * 1000.0 * _playback_speed)
	_current_time_ms += advance_ms

	if _current_time_ms >= _total_duration_ms:
		_current_time_ms = _total_duration_ms
		_is_playing = false
		playback_stopped.emit()
		return

	var new_frame_index := _find_frame_index(_current_time_ms)
	if new_frame_index != _current_frame_index:
		_current_frame_index = new_frame_index
		_apply_frame(_current_frame_index)

	time_changed.emit(_current_time_ms)
	_update_time_display()
	_check_best_moments()


func _find_frame_index(time_ms: int) -> int:
	if _position_data.is_empty():
		return 0

	# Binary search for frame
	var low := 0
	var high := _position_data.size() - 1

	while low < high:
		var mid := (low + high + 1) / 2
		var frame: Dictionary = _position_data[mid]
		var frame_time := int(frame.get("time_ms", frame.get("t", 0) * 1000))
		if frame_time <= time_ms:
			low = mid
		else:
			high = mid - 1

	return low


func _apply_frame(frame_index: int) -> void:
	if frame_index < 0 or frame_index >= _position_data.size():
		return

	var frame: Dictionary = _position_data[frame_index]

	# Apply ball position
	var ball_pos: Variant = frame.get("ball", frame.get("ball_position", null))
	if ball_pos:
		var ball_vec := _parse_position(ball_pos)
		_ball.global_position = _field_to_world(ball_vec)

	# Apply player positions
	var players_data: Variant = frame.get("players", frame.get("positions", null))
	if players_data is Array:
		for i in range(mini(players_data.size(), _players.size())):
			var player_data: Variant = players_data[i]
			if player_data is Dictionary:
				var pos := _parse_position(player_data.get("position", player_data.get("pos", Vector2.ZERO)))
				var world_pos := _field_to_world(pos)
				if _players[i].has_method("set_target_position"):
					_players[i].set_target_position(world_pos)

				# Apply pose/animation (accepts PoseBuilder.PoseType int or string)
				var pose: Variant = player_data.get("pose", player_data.get("state", "idle"))
				if _players[i].has_method("play_pose"):
					_players[i].play_pose(pose)
	elif players_data is Dictionary:
		# Dictionary format with player IDs
		for key in players_data.keys():
			var idx := int(key) if str(key).is_valid_int() else -1
			if idx >= 0 and idx < _players.size():
				var player_data: Dictionary = players_data[key]
				var pos := _parse_position(player_data.get("position", player_data.get("pos", Vector2.ZERO)))
				var world_pos := _field_to_world(pos)
				if _players[idx].has_method("set_target_position"):
					_players[idx].set_target_position(world_pos)


func _parse_position(value: Variant) -> Vector2:
	if value is Vector2:
		return value
	if value is Vector3:
		return Vector2(value.x, value.z)
	if value is Array and value.size() >= 2:
		return Vector2(float(value[0]), float(value[1]))
	if value is Dictionary:
		return Vector2(float(value.get("x", 0)), float(value.get("y", value.get("z", 0))))
	return Vector2.ZERO


## Convert field coordinates (0-105, 0-68) to world position
func _field_to_world(field_pos: Vector2) -> Vector3:
	# Center the field at origin
	var x := field_pos.x - FIELD_LENGTH / 2.0
	var z := field_pos.y - FIELD_WIDTH / 2.0
	return Vector3(x, 0.0, z)


func _check_best_moments() -> void:
	for moment in _best_moments:
		var start_ms := int(moment.get("start_time_ms", 0))
		var end_ms := int(moment.get("end_time_ms", 0))
		if _current_time_ms >= start_ms and _current_time_ms <= end_ms:
			if not moment.get("_triggered", false):
				moment["_triggered"] = true
				best_moment_reached.emit(moment)


func _update_score_display() -> void:
	if _score_label:
		_score_label.text = "%s %d - %d %s" % [_home_team_name, _home_score, _away_score, _away_team_name]


func _update_time_display() -> void:
	if _time_label:
		var minutes := _current_time_ms / 60000
		var seconds := (_current_time_ms / 1000) % 60
		_time_label.text = "%02d:%02d" % [minutes, seconds]


## Set team colors
func set_home_team_colors(shirt: Color, shorts: Color = Color.WHITE) -> void:
	home_shirt_color = shirt
	for i in range(11):
		if _players[i].has_method("set_team_colors"):
			_players[i].set_team_colors(shirt, shorts)


func set_away_team_colors(shirt: Color, shorts: Color = Color.WHITE) -> void:
	away_shirt_color = shirt
	for i in range(11, 22):
		if _players[i].has_method("set_team_colors"):
			_players[i].set_team_colors(shirt, shorts)


## ============================================================================
## Camera Controller API
## ============================================================================


func get_camera_controller() -> Node:
	return _camera_controller


func set_camera_mode(mode: int) -> void:
	if _camera_controller and _camera_controller.has_method("set_mode"):
		_camera_controller.set_mode(mode)


func get_camera_mode() -> int:
	if _camera_controller and _camera_controller.has_method("get_mode"):
		return _camera_controller.get_mode()
	return 0


func set_camera_broadcast() -> void:
	if _camera_controller and _camera_controller.has_method("set_broadcast"):
		_camera_controller.set_broadcast()


func set_camera_follow_ball() -> void:
	if _camera_controller and _camera_controller.has_method("follow_ball"):
		_camera_controller.follow_ball()


func set_camera_tactical() -> void:
	if _camera_controller and _camera_controller.has_method("set_tactical"):
		_camera_controller.set_tactical()


func set_camera_free() -> void:
	if _camera_controller and _camera_controller.has_method("set_free"):
		_camera_controller.set_free()


func camera_follow_player(player_index: int) -> void:
	if _camera_controller and _camera_controller.has_method("follow_player"):
		_camera_controller.follow_player(player_index)


func enable_auto_camera(enabled: bool) -> void:
	if _camera_controller and _camera_controller.has_method("enable_auto_camera"):
		_camera_controller.enable_auto_camera(enabled)


func cycle_camera_mode() -> void:
	if _camera_controller and _camera_controller.has_method("cycle_mode"):
		_camera_controller.cycle_mode()
