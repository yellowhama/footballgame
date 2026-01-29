extends Node2D
class_name SoccerPlayer

# Potagon Integration
const POTAGON_SCENE = preload("res://scenes/character/potagon/PotagonSprite.tscn")
var potagon: PotagonSprite = null

# Legacy Nodes (Hidden)
@onready var legacy_sprite: Sprite2D = $Sprite2D
@onready var legacy_shadow: Sprite2D = $Shadow
@onready var number_label: Label = $NumberLabel


# Snapshot buffer for Hermite interpolation
class Snap:
	var t_ms: float  # Timestamp (milliseconds)
	var pos: Vector2  # Position (screen pixels)
	var vel: Vector2  # Velocity (screen px/s)

	func _init(_t_ms: float, _pos: Vector2, _vel: Vector2):
		t_ms = _t_ms
		pos = _pos
		vel = _vel


var _snap_buf: Array[Snap] = []  # 최소 2개 필요 (s0, s1)

# State
var velocity: Vector2 = Vector2.ZERO
var target_position: Vector2 = Vector2.ZERO
var action: String = "idle"
var jersey_number: int = 0
var team_id: int = 0
var player_id: String = ""  # Used by TeamColorManager
var track_id: int = -1  # Engine tracking ID
var side: String = ""  # "home" or "away"

# Sequence Player (SSOT)
var _sequence_active: bool = false
var _sequence_steps: Array = []
var _sequence_step_index: int = 0
var _sequence_step_until: float = 0.0

# Smoothing
var _smooth_enabled: bool = true
var _smooth_speed: float = 12.0

# Visual state (separated from SSOT snapshot, for Dead Reckoning)
var _visual_pos: Vector2 = Vector2.ZERO
var _visual_vel: Vector2 = Vector2.ZERO
var _last_render_ms: float = 0.0
var _last_snapshot_ms: float = 0.0

## FIX02 M5: UI/Replay time SSOT
## - Snap timestamps must come from snapshot.t_ms (tick-based), not OS time.
## - UnifiedFramePipeline already emits snapshots with a fixed delay (e.g., playhead_ms - 100).
const INTERP_DELAY_MS := 0.0


func _ready() -> void:
	_setup_nodes()
	_setup_number_label()


func _setup_nodes():
	# Hide Legacy
	if legacy_sprite:
		legacy_sprite.visible = false
	if legacy_shadow:
		legacy_shadow.visible = false

	# Add Potagon
	if POTAGON_SCENE:
		potagon = POTAGON_SCENE.instantiate()
		add_child(potagon)
		move_child(potagon, 0)  # Behind label


func _setup_number_label():
	# Setup jersey number label styling
	if number_label:
		number_label.text = ""
		number_label.z_index = 1


func _process(delta: float) -> void:
	# Dead Reckoning + Hermite + Error Correction
	if _snap_buf.size() < 2:
		# Fallback: 스냅샷 부족 시 기존 lerp 사용
		if _smooth_enabled and target_position != Vector2.ZERO:
			position = position.lerp(target_position, _smooth_speed * delta)
	else:
		var render_t_ms := _last_snapshot_ms - INTERP_DELAY_MS

		# Find [s0, s1] snapshots where render_t_ms is between
		var i := 0
		while i < _snap_buf.size() - 2 and _snap_buf[i + 1].t_ms < render_t_ms:
			i += 1

		var s0: Snap = _snap_buf[i]
		var s1: Snap = _snap_buf[i + 1]

		# Interpolation parameters
		var seg_ms := maxf(s1.t_ms - s0.t_ms, 1.0)
		var dt_seg := seg_ms / 1000.0
		var t01 := (render_t_ms - s0.t_ms) / seg_ms
		t01 = clampf(t01, 0.0, 1.2)  # 약간의 overrun 허용 (예측)

		# 1. Hermite interpolation → target position
		var target_pos := RealInterp.hermite(s0.pos, s0.vel, s1.pos, s1.vel, t01, dt_seg)

		# 2. Velocity lerp → target velocity
		var target_vel := s0.vel.lerp(s1.vel, clampf(t01, 0.0, 1.0))

		# 3. Turn-rate limiting
		var vel_turn := RealInterp.turn_limited(_visual_vel, target_vel, RealInterp.MAX_TURN_RAD_PER_SEC, delta)

		# 4. Acceleration limiting
		var vel_new := RealInterp.accel_limited(_visual_vel, vel_turn, RealInterp.MAX_ACCEL_MPS2, delta)

		# 5. Dead Reckoning (prediction)
		var pred_pos := _visual_pos + vel_new * delta

		# 6. Error correction
		var err := target_pos - pred_pos
		var err_len := err.length()

		if err_len >= RealInterp.ERROR_HARD_M:
			# Hard teleport for large errors
			_visual_pos = target_pos
			_visual_vel = target_vel
		else:
			# Soft blending for small errors
			var blend := RealInterp.error_blend_factor(RealInterp.ERROR_HALF_LIFE_SEC, delta)
			var k := blend * clampf(err_len / RealInterp.ERROR_SOFT_M, 0.15, 1.0)
			_visual_pos = pred_pos + err * k
			_visual_vel = vel_new

		_last_render_ms = render_t_ms

		# Apply visual state to node
		position = _visual_pos
		velocity = _visual_vel  # For sprite direction

	# Pass velocity for direction update
	if potagon:
		potagon.set_direction(velocity)

	# Sequence player
	if _sequence_active:
		var now_ms := _last_snapshot_ms
		if now_ms >= _sequence_step_until:
			_sequence_step_index += 1
			_play_sequence_step()


func update_from_snapshot(t_ms: int, engine_pos: Vector2, new_action: String = "", new_velocity: Vector2 = Vector2.ZERO) -> void:
	var new_screen_pos := _engine_to_screen(engine_pos)
	var new_screen_vel := new_velocity  # Already transformed by HorizontalMatchViewer

	# Add snapshot to buffer
	var snap_t_ms := float(t_ms)
	_last_snapshot_ms = snap_t_ms
	_snap_buf.append(Snap.new(snap_t_ms, new_screen_pos, new_screen_vel))

	# Keep buffer size <= 3 (avoid memory leak)
	while _snap_buf.size() > 3:
		_snap_buf.pop_front()

	# Initialize visual state if first snapshot
	if _visual_pos == Vector2.ZERO and _snap_buf.size() > 0:
		_visual_pos = new_screen_pos
		_visual_vel = new_screen_vel
		_last_render_ms = snap_t_ms

	# Legacy target_position (for fallback)
	target_position = new_screen_pos
	velocity = new_screen_vel

	# Update Action (discrete state, immediate transition)
	if new_action != "" and new_action != action:
		action = new_action
		_update_potagon_animation(action)


func set_role(r: String):
	if potagon:
		potagon.is_goalkeeper = (r == "GK")


func _update_potagon_animation(act: String):
	if not potagon:
		return

	# Check for sequence override (tackle_slide, header)
	var sequence_id = _get_sequence_id(act)
	if sequence_id != "":
		_start_sequence(sequence_id)
		return

	# Map football action to potagon animation using SSOT
	var anim_name = PotagonActionRules.map_action(act)

	# Attempt to play (respects action lock)
	var played = potagon.play_animation(anim_name)

	# Handle GK hand physics override
	if (
		act
		in [
			MatchActions.GK_DIVE_L,
			MatchActions.GK_DIVE_R,
			MatchActions.GK_CATCH_HIGH,
			MatchActions.GK_CATCH_LOW,
			MatchActions.GK_PUNCH,
			MatchActions.BLOCK
		]
	):
		potagon.enable_hand_physics = false
	elif played:
		potagon.enable_hand_physics = true


func set_position_immediate(engine_pos: Vector2) -> void:
	position = _engine_to_screen(engine_pos)
	target_position = position


func _engine_to_screen(engine_pos: Vector2) -> Vector2:
	# Keep original logic
	const ENGINE_FIELD_LENGTH: float = 68.0
	const ENGINE_FIELD_WIDTH: float = 105.0
	const SCREEN_FIELD_LENGTH: float = 1050.0
	const SCREEN_FIELD_WIDTH: float = 680.0

	return Vector2(
		(engine_pos.y / ENGINE_FIELD_LENGTH) * SCREEN_FIELD_LENGTH,
		(engine_pos.x / ENGINE_FIELD_WIDTH) * SCREEN_FIELD_WIDTH
	)


# --- Sequence Player (SSOT) ---


func _get_sequence_id(act: String) -> String:
	## Phase 4: Basic sequences
	if act in [MatchActions.TACKLE_SLIDE, "tackle_slide"]:
		return "tackle_slide"
	if act in [MatchActions.HEADER, "header"]:
		return "header"

	## Phase 12: Advanced sequences (2025-12-15)
	if act in ["shoot_volley", "volley_shot"]:
		return "shoot_volley"
	if act in ["bicycle_kick", "overhead_kick"]:
		return "bicycle_kick"
	if act in ["long_throw", "throw_in_long"]:
		return "long_throw"
	if act in ["penalty_kick", "penalty"]:
		return "penalty_kick"
	if act in ["corner_kick", "corner"]:
		return "corner_kick"
	if act in ["free_kick", "direct_free_kick"]:
		return "free_kick"

	return ""


func _start_sequence(sequence_id: String):
	_sequence_steps = PotagonActionRules.get_sequence(sequence_id)
	if _sequence_steps.is_empty():
		return

	_sequence_active = true
	_sequence_step_index = 0
	_play_sequence_step()


func _play_sequence_step():
	if _sequence_step_index >= _sequence_steps.size():
		print("[SoccerPlayer] Sequence COMPLETE")
		_sequence_active = false
		return

	var step = _sequence_steps[_sequence_step_index]
	var anim = step.get("anim", "idle")
	var hold_s = step.get("hold_s", 0.3)

	print(
		(
			"[SoccerPlayer] Sequence step %d/%d: '%s' (%.2fs)"
			% [_sequence_step_index + 1, _sequence_steps.size(), anim, hold_s]
		)
	)

	# Force play (override lock)
	potagon.play_animation(anim, true)

	# Set step timer
	var now_ms := _last_snapshot_ms
	_sequence_step_until = now_ms + (hold_s * 1000.0)


# --- Appearance Config ---


# Simple mapping for Potagon
func setup_from_roster(data: Dictionary):
	if not potagon:
		return

	# Extract kit colors
	var pri = Color.RED
	var sec = Color.WHITE

	# Logic from original script to extract colors
	if data.has("kit_primary"):  # Rust
		var kp = data["kit_primary"]
		pri = Color(kp[0] / 255.0, kp[1] / 255.0, kp[2] / 255.0)
		var ks = data.get("kit_secondary", [255, 255, 255])
		sec = Color(ks[0] / 255.0, ks[1] / 255.0, ks[2] / 255.0)
	elif data.has("uniform"):  # Socceralia
		pri = Color(data.uniform.get("primary_color", "#ff0000"))
		sec = Color(data.uniform.get("secondary_color", "#ffffff"))

	# Apply Kit
	potagon.primary_color = pri
	potagon.secondary_color = sec

	# ID Mapping (Hash from name/id to stable random 1-15, 1-10)
	var seed_val = 0
	if data.has("player_id"):
		seed_val = data.player_id.hash()
	elif data.has("name"):
		seed_val = data.name.hash()
	else:
		seed_val = randi()

	potagon.skin_id = (abs(seed_val) % 15) + 1
	potagon.shirt_id = (abs(seed_val >> 4) % 10) + 1
	potagon.pant_id = (abs(seed_val >> 8) % 10) + 1


func set_jersey_number(number: int):
	jersey_number = number
	if number_label:
		number_label.text = str(number) if number > 0 else ""


func set_highlighted(val: bool):
	if potagon:
		potagon.modulate = Color(1.3, 1.3, 1.1) if val else Color.WHITE
