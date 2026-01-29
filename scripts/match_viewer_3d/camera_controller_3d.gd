extends Node3D
class_name CameraController3D

## Camera controller for 3D match viewer.
## Supports multiple camera modes and automatic camera for BestMoments.

signal mode_changed(mode: CameraMode)
signal target_changed(target: Node3D)

enum CameraMode {
	BROADCAST,  ## Side view (TV broadcast style)
	FOLLOW_BALL,  ## Track the ball
	FOLLOW_PLAYER,  ## Track a specific player
	FREE,  ## Free camera movement
	TACTICAL,  ## Top-down tactical view
}

## Field dimensions
const FIELD_LENGTH := 105.0
const FIELD_WIDTH := 68.0
const FIELD_CENTER := Vector3(0.0, 0.0, 0.0)

## Camera settings per mode
const MODE_SETTINGS := {
	CameraMode.BROADCAST:
	{
		"position": Vector3(0.0, 25.0, 55.0),
		"rotation": Vector3(-20.0, 0.0, 0.0),
		"fov": 50.0,
		"follow_speed": 3.0,
	},
	CameraMode.FOLLOW_BALL:
	{
		"distance": 20.0,
		"height": 12.0,
		"angle": -30.0,
		"fov": 60.0,
		"follow_speed": 5.0,
	},
	CameraMode.FOLLOW_PLAYER:
	{
		"distance": 8.0,
		"height": 4.0,
		"angle": -15.0,
		"fov": 70.0,
		"follow_speed": 6.0,
	},
	CameraMode.FREE:
	{
		"move_speed": 20.0,
		"rotate_speed": 2.0,
		"fov": 60.0,
	},
	CameraMode.TACTICAL:
	{
		"position": Vector3(0.0, 60.0, 0.0),
		"rotation": Vector3(-90.0, 0.0, 0.0),
		"fov": 45.0,
		"follow_speed": 2.0,
	},
}

## References
@export var camera_path: NodePath = ^""
@export var initial_mode: CameraMode = CameraMode.BROADCAST

var _camera: Camera3D = null
var _current_mode: CameraMode = CameraMode.BROADCAST
var _target: Node3D = null
var _ball: Node3D = null
var _players: Array[Node3D] = []
var _follow_player_index: int = -1

## Free camera state
var _free_velocity: Vector3 = Vector3.ZERO
var _free_rotation: Vector2 = Vector2.ZERO

## Smooth transition
var _target_position: Vector3 = Vector3.ZERO
var _target_rotation: Vector3 = Vector3.ZERO
var _transition_speed: float = 3.0
var _is_transitioning: bool = false

## Auto camera state
var _auto_camera_enabled: bool = false
var _auto_camera_timer: float = 0.0
var _current_moment: Dictionary = {}


func _ready() -> void:
	if not camera_path.is_empty():
		_camera = get_node_or_null(camera_path) as Camera3D
	else:
		_camera = _find_camera()

	if _camera:
		set_mode(initial_mode)
		print("[CameraController3D] Initialized with mode: %s" % CameraMode.keys()[initial_mode])


func _process(delta: float) -> void:
	if not _camera:
		return

	match _current_mode:
		CameraMode.BROADCAST:
			_update_broadcast_camera(delta)
		CameraMode.FOLLOW_BALL:
			_update_follow_ball_camera(delta)
		CameraMode.FOLLOW_PLAYER:
			_update_follow_player_camera(delta)
		CameraMode.FREE:
			_update_free_camera(delta)
		CameraMode.TACTICAL:
			_update_tactical_camera(delta)

	# Handle auto camera timer
	if _auto_camera_enabled and not _current_moment.is_empty():
		_auto_camera_timer -= delta
		if _auto_camera_timer <= 0.0:
			_end_auto_camera()


func _find_camera() -> Camera3D:
	# Look for camera in parent or siblings
	var parent := get_parent()
	if parent:
		for child in parent.get_children():
			if child is Camera3D:
				return child
		# Check if parent has unique node
		if parent.has_node("%Camera3D"):
			return parent.get_node("%Camera3D") as Camera3D
	return null


## ============================================================================
## Public API
## ============================================================================


func set_mode(mode: CameraMode) -> void:
	if mode == _current_mode:
		return

	_current_mode = mode
	_apply_mode_settings()
	mode_changed.emit(mode)

	print("[CameraController3D] Mode changed to: %s" % CameraMode.keys()[mode])


func get_mode() -> CameraMode:
	return _current_mode


func set_camera(camera: Camera3D) -> void:
	_camera = camera


func set_ball(ball: Node3D) -> void:
	_ball = ball


func set_players(players: Array) -> void:
	_players.clear()
	for p in players:
		if p is Node3D:
			_players.append(p)


func follow_player(player_index: int) -> void:
	if player_index < 0 or player_index >= _players.size():
		push_warning("[CameraController3D] Invalid player index: %d" % player_index)
		return

	_follow_player_index = player_index
	_target = _players[player_index]
	set_mode(CameraMode.FOLLOW_PLAYER)
	target_changed.emit(_target)


func follow_ball() -> void:
	_target = _ball
	set_mode(CameraMode.FOLLOW_BALL)
	target_changed.emit(_target)


func set_broadcast() -> void:
	set_mode(CameraMode.BROADCAST)


func set_tactical() -> void:
	set_mode(CameraMode.TACTICAL)


func set_free() -> void:
	set_mode(CameraMode.FREE)


func enable_auto_camera(enabled: bool) -> void:
	_auto_camera_enabled = enabled
	if not enabled:
		_current_moment = {}
		_auto_camera_timer = 0.0


func is_auto_camera_enabled() -> bool:
	return _auto_camera_enabled


## ============================================================================
## BestMoment Auto Camera
## ============================================================================


func on_best_moment_start(moment: Dictionary) -> void:
	if not _auto_camera_enabled:
		return

	_current_moment = moment
	var moment_type := str(moment.get("moment_type", "")).to_lower()
	var duration_ms := int(moment.get("end_time_ms", 0)) - int(moment.get("start_time_ms", 0))
	_auto_camera_timer = float(duration_ms) / 1000.0

	# Choose camera based on moment type
	match moment_type:
		"goal":
			_auto_camera_goal(moment)
		"save":
			_auto_camera_save(moment)
		"penalty":
			_auto_camera_penalty(moment)
		"shot_on_target", "shotontarget":
			_auto_camera_shot(moment)
		_:
			_auto_camera_default(moment)


func _auto_camera_goal(moment: Dictionary) -> void:
	# Zoom to goal area
	var team_id := int(moment.get("team_id", 0))
	var goal_x := FIELD_LENGTH / 2.0 if team_id == 0 else -FIELD_LENGTH / 2.0
	_target_position = Vector3(goal_x * 0.7, 15.0, 30.0)
	_target_rotation = Vector3(-25.0, 0.0, 0.0)
	_transition_to_position()


func _auto_camera_save(moment: Dictionary) -> void:
	# Follow goalkeeper
	var team_id := int(moment.get("team_id", 0))
	# Goalkeeper is usually index 0 (home) or 11 (away)
	var gk_index := 0 if team_id == 0 else 11
	if gk_index < _players.size():
		follow_player(gk_index)


func _auto_camera_penalty(moment: Dictionary) -> void:
	# Side view of penalty area
	var team_id := int(moment.get("team_id", 0))
	var goal_x := FIELD_LENGTH / 2.0 if team_id == 0 else -FIELD_LENGTH / 2.0
	_target_position = Vector3(goal_x * 0.6, 10.0, 35.0)
	_target_rotation = Vector3(-15.0, 0.0, 0.0)
	_transition_to_position()


func _auto_camera_shot(moment: Dictionary) -> void:
	# Follow ball during shot
	follow_ball()


func _auto_camera_default(_moment: Dictionary) -> void:
	# Default: follow ball
	if _ball:
		follow_ball()
	else:
		set_broadcast()


func _end_auto_camera() -> void:
	_current_moment = {}
	# Return to broadcast mode after moment ends
	set_broadcast()


func _transition_to_position() -> void:
	_is_transitioning = true
	# The actual transition happens in _process via lerp


## ============================================================================
## Camera Mode Updates
## ============================================================================


func _apply_mode_settings() -> void:
	if not _camera:
		return

	var settings: Dictionary = MODE_SETTINGS.get(_current_mode, {})

	if settings.has("fov"):
		_camera.fov = settings.fov

	if settings.has("position"):
		_target_position = settings.position
	if settings.has("rotation"):
		_target_rotation = settings.rotation


func _update_broadcast_camera(delta: float) -> void:
	var settings: Dictionary = MODE_SETTINGS[CameraMode.BROADCAST]
	var follow_speed: float = settings.follow_speed

	# Track ball horizontally with limits
	var target_x := 0.0
	if _ball:
		target_x = clamp(_ball.global_position.x, -FIELD_LENGTH * 0.4, FIELD_LENGTH * 0.4)

	var base_pos: Vector3 = settings.position
	var desired_pos := Vector3(target_x, base_pos.y, base_pos.z)

	_camera.global_position = _camera.global_position.lerp(desired_pos, follow_speed * delta)
	_camera.rotation_degrees = Vector3(settings.rotation.x, 0.0, 0.0)


func _update_follow_ball_camera(delta: float) -> void:
	if not _ball:
		_update_broadcast_camera(delta)
		return

	var settings: Dictionary = MODE_SETTINGS[CameraMode.FOLLOW_BALL]
	var follow_speed: float = settings.follow_speed
	var distance: float = settings.distance
	var height: float = settings.height

	var ball_pos := _ball.global_position
	# Camera behind and above the ball, looking at it
	var desired_pos := ball_pos + Vector3(0.0, height, distance)

	_camera.global_position = _camera.global_position.lerp(desired_pos, follow_speed * delta)
	_camera.look_at(ball_pos, Vector3.UP)


func _update_follow_player_camera(delta: float) -> void:
	if not _target or _follow_player_index < 0:
		_update_broadcast_camera(delta)
		return

	var settings: Dictionary = MODE_SETTINGS[CameraMode.FOLLOW_PLAYER]
	var follow_speed: float = settings.follow_speed
	var distance: float = settings.distance
	var height: float = settings.height

	var player_pos := _target.global_position
	# Camera behind player
	var player_forward := -_target.global_transform.basis.z
	var desired_pos := player_pos - player_forward * distance + Vector3(0.0, height, 0.0)

	_camera.global_position = _camera.global_position.lerp(desired_pos, follow_speed * delta)
	_camera.look_at(player_pos + Vector3(0.0, 1.0, 0.0), Vector3.UP)


func _update_free_camera(delta: float) -> void:
	var settings: Dictionary = MODE_SETTINGS[CameraMode.FREE]
	var move_speed: float = settings.move_speed
	var rotate_speed: float = settings.rotate_speed

	# Get input for free camera
	var input_dir := Vector3.ZERO
	if Input.is_action_pressed("ui_up"):
		input_dir.z -= 1.0
	if Input.is_action_pressed("ui_down"):
		input_dir.z += 1.0
	if Input.is_action_pressed("ui_left"):
		input_dir.x -= 1.0
	if Input.is_action_pressed("ui_right"):
		input_dir.x += 1.0
	if Input.is_action_pressed("ui_page_up"):
		input_dir.y += 1.0
	if Input.is_action_pressed("ui_page_down"):
		input_dir.y -= 1.0

	# Transform input to camera space
	var camera_basis := _camera.global_transform.basis
	var movement := camera_basis * input_dir.normalized() * move_speed * delta
	_camera.global_position += movement

	# Mouse look (if capturing mouse)
	if Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
		var mouse_delta := Input.get_last_mouse_velocity() * 0.001 * rotate_speed
		_camera.rotate_y(-mouse_delta.x)
		_camera.rotate_object_local(Vector3.RIGHT, -mouse_delta.y)


func _update_tactical_camera(delta: float) -> void:
	var settings: Dictionary = MODE_SETTINGS[CameraMode.TACTICAL]
	var follow_speed: float = settings.follow_speed

	# Track ball from above
	var target_pos := FIELD_CENTER
	if _ball:
		target_pos = Vector3(_ball.global_position.x, 0.0, _ball.global_position.z)
		target_pos.x = clamp(target_pos.x, -FIELD_LENGTH * 0.3, FIELD_LENGTH * 0.3)
		target_pos.z = clamp(target_pos.z, -FIELD_WIDTH * 0.3, FIELD_WIDTH * 0.3)

	var base_pos: Vector3 = settings.position
	var desired_pos := Vector3(target_pos.x, base_pos.y, target_pos.z)

	_camera.global_position = _camera.global_position.lerp(desired_pos, follow_speed * delta)
	_camera.rotation_degrees = Vector3(settings.rotation.x, 0.0, 0.0)


## ============================================================================
## Input Handling
## ============================================================================


func _unhandled_input(event: InputEvent) -> void:
	if not _camera:
		return

	# Number keys to switch modes
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_1:
				set_broadcast()
			KEY_2:
				follow_ball()
			KEY_3:
				set_tactical()
			KEY_4:
				set_free()
			KEY_ESCAPE:
				if _current_mode == CameraMode.FREE:
					Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)

	# Mouse capture for free camera
	if _current_mode == CameraMode.FREE:
		if event is InputEventMouseButton and event.pressed:
			if event.button_index == MOUSE_BUTTON_RIGHT:
				Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


## ============================================================================
## Utility
## ============================================================================


func get_mode_name() -> String:
	return CameraMode.keys()[_current_mode]


func cycle_mode() -> void:
	var next_mode := (_current_mode + 1) % CameraMode.size()
	set_mode(next_mode as CameraMode)
