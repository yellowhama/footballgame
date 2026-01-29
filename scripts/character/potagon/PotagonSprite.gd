@tool
extends Node2D
class_name PotagonSprite

## PotagonSprite
##
## Handles the rendering of Potagon Pixel characters with layered sprites (Body, Shirt, Pant)
## and dynamic 8-directional texture switching.
##
## usage:
##  1. Instantiate this scene.
##  2. Set `skin_id` (1-15), `shirt_id` (1-10), `pant_id` (1-10).
##  3. Set `primary_color` (Shirt tint) and `secondary_color` (Pant tint).
##  4. Call `set_direction(velocity)` in _process to update sprite facing.
##  5. Call `play_animation("run")` etc.
##
## Keeper & Hand Physics Usage:
##  - Normal Player:
##      is_goalkeeper = false
##      enable_hand_physics = true (Hands dangle/trail behind)
##
##  - Goalkeeper:
##      is_goalkeeper = true (Hands become Big White Gloves)
##      enable_hand_physics = true (When moving)
##      enable_hand_physics = false (When Saving/Blocking -> Hands reach forward)
##

# --- Export Properties ---
@export_range(1, 15) var skin_id: int = 1:
	set(v):
		skin_id = v
		_request_texture_update()

@export_range(1, 10) var shirt_id: int = 1:
	set(v):
		shirt_id = v
		_request_texture_update()

@export_range(1, 10) var pant_id: int = 1:
	set(v):
		pant_id = v
		_request_texture_update()

@export var primary_color: Color = Color.WHITE:
	set(c):
		primary_color = c
		if _shirt_sprite:
			_shirt_sprite.self_modulate = c

@export var secondary_color: Color = Color.WHITE:
	set(c):
		secondary_color = c
		if _pant_sprite:
			_pant_sprite.self_modulate = c

# --- Nodes ---
@onready var _body_sprite = $BodySprite
@onready var _shirt_sprite = $ShirtSprite
@onready var _pant_sprite = $PantSprite
@onready var _anim_player = $AnimationPlayer
@onready var _shadow = $Shadow

# Physics Hands
@onready var _left_hand: Sprite2D = $LeftHand
@onready var _right_hand: Sprite2D = $RightHand
@onready var _left_shoulder: Marker2D = $BodySprite/LeftShoulder
@onready var _right_shoulder: Marker2D = $BodySprite/RightShoulder

# --- Physics Config ---
var nav_velocity: Vector2 = Vector2.ZERO
var follow_speed: float = 12.0
var dangle_amount: float = 0.08
var flutter_speed: float = 15.0
var flutter_range: float = 2.0

# If true, hands dangle (Run/Walk).
# If false, hands reach forward (Keeper Save/Push/Attack).
@export var enable_hand_physics: bool = true
@export var is_goalkeeper: bool = false:
	set(v):
		is_goalkeeper = v
		_request_texture_update()

# --- Action Lock System (SSOT) ---
var _action_locked: bool = false
var _action_lock_until: float = 0.0  # Time.get_ticks_msec()
var _current_animation: String = "idle"

# --- Constants ---
## Phase 11: Migrated to res:// paths for portability (2025-12-15)
const DEFAULT_BASE_PATH_CHAR = "res://assets/potagon/pixel/CharacterSheets/CharacterSheets"
const DEFAULT_BASE_PATH_SHIRT = "res://assets/potagon/pixel/ShirtSheets/ShirtSheets"
const DEFAULT_BASE_PATH_PANT = "res://assets/potagon/pixel/PantSheets/PantSheets"

var _base_path_char: String = DEFAULT_BASE_PATH_CHAR
var _base_path_shirt: String = DEFAULT_BASE_PATH_SHIRT
var _base_path_pant: String = DEFAULT_BASE_PATH_PANT

var _asset_manifest: Dictionary = {}

var _current_dir_name: String = "Down"

var _skin_colors = {
	1: Color.BISQUE,
	2: Color.TAN,
	3: Color.ROSY_BROWN,
	4: Color.PERU,
	5: Color.SADDLE_BROWN,
}


func _ready():
	# Scale up body sprites (keep hands at original size)
	if _body_sprite:
		_body_sprite.scale = Vector2(3.0, 3.0)
	if _shirt_sprite:
		_shirt_sprite.scale = Vector2(3.0, 3.0)
	if _pant_sprite:
		_pant_sprite.scale = Vector2(3.0, 3.0)
	if _shadow:
		_shadow.scale = Vector2(2.5, 2.5)  # Larger shadow

	_load_asset_manifest()
	_apply_manifest_animation_library()

	_update_textures()
	play_animation("idle")


func _process(delta):
	_update_hand_physics(delta)

	# Check lock expiration
	if _action_locked:
		var now_ms = Time.get_ticks_msec()
		if now_ms >= _action_lock_until:
			_action_locked = false


func _update_hand_physics(delta: float):
	if not _left_hand or not _left_shoulder:
		return

	var target_l = _left_shoulder.global_position
	var target_r = _right_shoulder.global_position

	# Idle gravity effect: hands drop slightly
	var gravity_offset = Vector2.ZERO
	if _current_animation == "idle":
		gravity_offset = Vector2(0, 4)  # 4px down for idle

	if enable_hand_physics:
		# --- MODE A: Dangle (Inertia + Flutter) ---

		# Dynamic Inertia (10m/s -> ~35px drag)
		var inertia = -nav_velocity * dangle_amount * 3.5

		var t = Time.get_ticks_msec() / 1000.0
		var flutter = Vector2(0, sin(t * flutter_speed) * flutter_range)

		target_l += inertia + flutter + gravity_offset
		target_r += inertia + (flutter * -1.0) + gravity_offset

		# Smoothly interpolate
		if _left_hand.top_level:
			_left_hand.global_position = _left_hand.global_position.lerp(target_l, follow_speed * delta)
			_right_hand.global_position = _right_hand.global_position.lerp(target_r, follow_speed * delta)

	else:
		# --- MODE B: Manual Reach (Keeper Save / Push) ---
		# Hands snap mainly to texturing direction (Forward)

		var forward_vec = Vector2.DOWN  # Default
		match _current_dir_name:
			"Up", "UpLeft", "UpRight":
				forward_vec = Vector2.UP
			"Down", "DownLeft", "DownRight":
				forward_vec = Vector2.DOWN
			"Left":
				forward_vec = Vector2.LEFT
			"RIght", "Right":
				forward_vec = Vector2.RIGHT

		# Reach out by 12 pixels
		var reach_offset = forward_vec * 12.0
		var spread_offset = Vector2.ZERO

		# Slight spread for hands width
		if forward_vec.y != 0:
			spread_offset = Vector2(4, 0)  # Vertical move -> Spread X
		else:
			spread_offset = Vector2(0, 4)  # Horizontal move -> Spread Y

		# Snap instantly (no lerp) for responsive action
		if _left_hand.top_level:
			_left_hand.global_position = target_l + reach_offset - spread_offset
			_right_hand.global_position = target_r + reach_offset + spread_offset


# --- Animation Control ---
func play_animation(anim_name: String, force: bool = false) -> bool:
	# Check lock
	var now_ms = Time.get_ticks_msec()
	if _action_locked and now_ms < _action_lock_until and not force:
		var remaining = (_action_lock_until - now_ms) / 1000.0
		print(
			(
				"[PotagonSprite] Animation '%s' REJECTED (locked: %.2fs remaining, current='%s')"
				% [anim_name, remaining, _current_animation]
			)
		)
		return false  # Animation rejected

	# Apply runtime loop enforcement from SSOT
	var should_loop = PotagonActionRules.get_loop(anim_name)
	if _anim_player and _anim_player.has_animation(anim_name):
		var anim = _anim_player.get_animation(anim_name)
		if anim:
			anim.loop_mode = Animation.LOOP_LINEAR if should_loop else Animation.LOOP_NONE

	# Play animation
	if _anim_player:
		_anim_player.play(anim_name)
		_current_animation = anim_name

		# Set lock based on hold time from SSOT
		var hold_time = PotagonActionRules.get_hold_time(anim_name)
		if hold_time > 0.0:
			_action_locked = true
			_action_lock_until = now_ms + (hold_time * 1000.0)
			print(
				"[PotagonSprite] Animation '%s' PLAYING (hold: %.2fs, loop: %s)" % [anim_name, hold_time, should_loop]
			)
		else:
			_action_locked = false
			print("[PotagonSprite] Animation '%s' PLAYING (no hold, loop: %s)" % [anim_name, should_loop])

		return true
	return false


func is_locked() -> bool:
	if not _action_locked:
		return false
	var now_ms = Time.get_ticks_msec()
	if now_ms >= _action_lock_until:
		_action_locked = false
		return false
	return true


func get_remaining_lock_time() -> float:
	if not is_locked():
		return 0.0
	var now_ms = Time.get_ticks_msec()
	return max(0.0, (_action_lock_until - now_ms) / 1000.0)


func set_direction(dir_vector: Vector2):
	nav_velocity = dir_vector  # Update for physics

	if dir_vector.length_squared() < 0.01:
		return

	var angel = dir_vector.angle()
	var new_dir_name = _get_dir_name_from_angle(angel)

	if new_dir_name != _current_dir_name:
		_current_dir_name = new_dir_name
		_update_textures()
		_update_hand_z_index(new_dir_name)


func _update_hand_z_index(dir: String):
	# Adjust Hand Z-Order based on facing
	if not _left_hand:
		return

	match dir:
		"Up", "UpLeft", "UpRight":
			_left_hand.z_index = -1
			_right_hand.z_index = -1
		"Down", "DownLeft", "DownRight":
			_left_hand.z_index = 0
			_right_hand.z_index = 0
		"Left":
			_left_hand.z_index = -1  # Right hand (screen left) is back
			_right_hand.z_index = 0
		"RIght", "Right":
			_left_hand.z_index = 0
			_right_hand.z_index = -1


func _get_dir_name_from_angle(rad: float) -> String:
	var deg = rad_to_deg(rad)
	var index = int(round(deg / 45.0))
	match index:
		0:
			return "RIght"
		1:
			return "DownRight"
		2:
			return "Down"
		3:
			return "DownLeft"
		4, -4:
			return "Left"
		-1:
			return "UpRight"
		-2:
			return "Up"
		-3:
			return "UpLeft"
	return "Down"


func _update_textures():
	if not is_inside_tree():
		return

	_body_sprite.texture = _load_external_texture(_base_path_char, "Color%d" % skin_id, _current_dir_name)
	_shirt_sprite.texture = _load_external_texture(_base_path_shirt, "Color%d" % shirt_id, _current_dir_name)
	_pant_sprite.texture = _load_external_texture(_base_path_pant, "Color%d" % pant_id, _current_dir_name)

	_shirt_sprite.self_modulate = primary_color
	_pant_sprite.self_modulate = secondary_color

	# Hand Logic (Gloves vs Skin)
	if is_goalkeeper:
		# Gloves: White (or light gray), Big Scale
		var glove_color = Color(0.95, 0.95, 0.95)
		if _left_hand:
			_left_hand.self_modulate = glove_color
			_left_hand.scale = Vector2(1.6, 1.6)  # Big Gloves
		if _right_hand:
			_right_hand.self_modulate = glove_color
			_right_hand.scale = Vector2(1.6, 1.6)
	else:
		# Skin: Tinted, Normal Scale
		var hand_color = _skin_colors.get(skin_id, Color.BISQUE)
		if _left_hand:
			_left_hand.self_modulate = hand_color
			_left_hand.scale = Vector2(1.0, 1.0)
		if _right_hand:
			_right_hand.self_modulate = hand_color
			_right_hand.scale = Vector2(1.0, 1.0)


var _texture_cache = {}


func _load_external_texture(base: String, color_folder: String, dir_name: String) -> Texture2D:
	var fname = dir_name + ".png"
	var path = "%s/%s/%s" % [base, color_folder, fname]

	# Check cache first
	if _texture_cache.has(path):
		return _texture_cache[path]

	## Phase 11: Use ResourceLoader for res:// paths (2025-12-15)
	# Godot's ResourceLoader handles both res:// and user:// paths
	if ResourceLoader.exists(path):
		var tex = load(path) as Texture2D
		if tex:
			_texture_cache[path] = tex
			return tex
		else:
			push_error("[PotagonSprite] Failed to load texture: %s" % path)
	else:
		push_error("[PotagonSprite] Texture not found: %s" % path)

	return null


func _request_texture_update():
	call_deferred("_update_textures")


func _load_asset_manifest() -> void:
	_asset_manifest = AssetManifestLoader.load_manifest("res://assets_manifest.json")
	if _asset_manifest.is_empty():
		_base_path_char = DEFAULT_BASE_PATH_CHAR
		_base_path_shirt = DEFAULT_BASE_PATH_SHIRT
		_base_path_pant = DEFAULT_BASE_PATH_PANT
		return

	var paths: Dictionary = _asset_manifest.get("paths", {})
	if paths is Dictionary:
		_base_path_char = str(paths.get("character_sheets_base", DEFAULT_BASE_PATH_CHAR))
		_base_path_shirt = str(paths.get("shirt_sheets_base", DEFAULT_BASE_PATH_SHIRT))
		_base_path_pant = str(paths.get("pant_sheets_base", DEFAULT_BASE_PATH_PANT))


func _apply_manifest_animation_library() -> void:
	if _anim_player == null:
		return

	if _asset_manifest.is_empty():
		return

	var track_paths := [NodePath("BodySprite:frame"), NodePath("ShirtSprite:frame"), NodePath("PantSprite:frame")]
	var lib := PotagonSpriteFramesBuilder.build_animation_library(_asset_manifest, track_paths)
	if lib == null:
		return

	var ok := PotagonSpriteFramesBuilder.apply_animation_library(_anim_player, lib, "")
	if ok:
		# Ensure sheet layout matches manifest (hframes/vframes) for correct indexing.
		var sheet: Dictionary = _asset_manifest.get("sheet", {})
		var cols := int(sheet.get("columns", 8))
		var rows := int(sheet.get("rows", 6))
		for sprite in [_body_sprite, _shirt_sprite, _pant_sprite]:
			if sprite:
				sprite.hframes = cols
				sprite.vframes = rows
