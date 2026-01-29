extends Node3D
class_name PlayerController3D

## Controls a single 3D player model for replay visualization.
## Handles position interpolation, rotation, animation, and appearance customization.

signal animation_started(anim_name: String)
signal animation_finished(anim_name: String)

const PlayerAppearanceClass := preload("res://scripts/match_viewer_3d/player_appearance.gd")

# Team colors
@export var home_shirt_color: Color = Color(0.9, 0.1, 0.1)  # Red
@export var away_shirt_color: Color = Color(0.1, 0.1, 0.9)  # Blue
@export var home_shorts_color: Color = Color.WHITE
@export var away_shorts_color: Color = Color.WHITE

# Goalkeeper colors
@export var gk_shirt_color: Color = Color(0.2, 0.8, 0.2)  # Green
@export var gk_shorts_color: Color = Color.BLACK

# Player state
var player_index: int = -1
var is_home_team: bool = true
var is_goalkeeper: bool = false
var jersey_number: int = 0
var jersey_name: String = ""
var current_position: Vector3 = Vector3.ZERO
var target_position: Vector3 = Vector3.ZERO
var current_rotation: float = 0.0  # Y-axis rotation in radians
var current_pose: String = "idle"

# Appearance data
var _appearance: RefCounted = null  # PlayerAppearance

# Animation mapping (PoseBuilder.PoseType -> FBX animation names)
# FBX animations from Studio Ochi Soccer Players pack:
#   - "Metarig Woman.013|Soccer Idle"
#   - "Metarig Woman.013|Soccer Running"
#   - "Metarig Woman.013|Soccer Pass"
#   - "Metarig Woman.013|Soccer Kick"
#   - "Metarig Woman.013|Soccer Goalkeeper Idle"
#   - "Metarig Woman.013|Soccer Goalkeeper Catch"
const POSE_TO_ANIM: Dictionary = {
	# PoseBuilder.PoseType enum values -> FBX animation names
	0: "Metarig Woman.013|Soccer Idle",  # IDLE
	1: "Metarig Woman.013|Soccer Running",  # WALK (use running)
	2: "Metarig Woman.013|Soccer Running",  # RUN
	3: "Metarig Woman.013|Soccer Running",  # SPRINT (use running)
	4: "Metarig Woman.013|Soccer Pass",  # PASS
	5: "Metarig Woman.013|Soccer Kick",  # SHOT
	6: "Metarig Woman.013|Soccer Kick",  # HEAD (use kick)
	7: "Metarig Woman.013|Soccer Kick",  # TACKLE (use kick)
	8: "Metarig Woman.013|Soccer Running",  # DRIBBLE (use running)
	9: "Metarig Woman.013|Soccer Goalkeeper Catch",  # SAVE
	10: "Metarig Woman.013|Soccer Pass",  # THROW_IN (use pass)
}

# String-based fallback mapping (for backwards compatibility)
const POSE_STRING_TO_ANIM: Dictionary = {
	"idle": "Metarig Woman.013|Soccer Idle",
	"walk": "Metarig Woman.013|Soccer Running",
	"run": "Metarig Woman.013|Soccer Running",
	"sprint": "Metarig Woman.013|Soccer Running",
	"pass": "Metarig Woman.013|Soccer Pass",
	"shot": "Metarig Woman.013|Soccer Kick",
	"head": "Metarig Woman.013|Soccer Kick",
	"tackle": "Metarig Woman.013|Soccer Kick",
	"dribble": "Metarig Woman.013|Soccer Running",
	"save": "Metarig Woman.013|Soccer Goalkeeper Catch",
	"throw_in": "Metarig Woman.013|Soccer Pass",
	"celebrate": "Metarig Woman.013|Soccer Idle",
}

# Default fallback animation (when requested anim not found)
const FALLBACK_ANIMATION := "Metarig Woman.013|Soccer Idle"

# Interpolation settings
@export var position_lerp_speed: float = 10.0
@export var rotation_lerp_speed: float = 8.0

# Node references
var _animation_player: AnimationPlayer = null
var _skeleton: Skeleton3D = null
var _mesh_instances: Array[MeshInstance3D] = []

# Materials for team colors
var _shirt_material: StandardMaterial3D = null
var _shorts_material: StandardMaterial3D = null

# Jersey number label
var _number_label: Label3D = null


func _ready() -> void:
	_find_child_nodes()
	_setup_materials()
	_setup_number_label()


func _process(delta: float) -> void:
	_interpolate_position(delta)
	_interpolate_rotation(delta)


## Initialize player with index and team
func setup(idx: int, home_team: bool) -> void:
	player_index = idx
	is_home_team = home_team
	apply_team_colors()


## Set target position for interpolation (from replay data)
func set_target_position(pos: Vector3) -> void:
	target_position = pos


## Set target position immediately without interpolation
func set_position_immediate(pos: Vector3) -> void:
	target_position = pos
	current_position = pos
	global_position = pos


## Set target rotation (radians, Y-axis)
func set_target_rotation(rot: float) -> void:
	current_rotation = rot


# Animation blending settings
const ANIMATION_FADE_TIME := 0.2  # Cross-fade duration in seconds


## Play animation by pose type (accepts int PoseType or string)
func play_pose(pose_type: Variant) -> void:
	var anim_name: String
	var pose_key: String

	# Handle both int (PoseBuilder.PoseType enum) and string
	if pose_type is int:
		anim_name = POSE_TO_ANIM.get(pose_type, "idle")
		pose_key = str(pose_type)
	else:
		var pose_str := str(pose_type).to_lower()
		anim_name = POSE_STRING_TO_ANIM.get(pose_str, "idle")
		pose_key = pose_str

	# Skip if same pose
	if current_pose == pose_key:
		return
	current_pose = pose_key

	if not _animation_player:
		return

	# Try to find the requested animation
	if not _animation_player.has_animation(anim_name):
		# Fallback to idle
		anim_name = "idle"
		if not _animation_player.has_animation(anim_name):
			# Final fallback to whatever animation exists
			anim_name = FALLBACK_ANIMATION
			if not _animation_player.has_animation(anim_name):
				return

	# Cross-fade to new animation
	if _animation_player.current_animation != "" and _animation_player.current_animation != anim_name:
		_animation_player.play(anim_name, ANIMATION_FADE_TIME)
	else:
		_animation_player.play(anim_name)

	animation_started.emit(anim_name)


func _find_child_nodes() -> void:
	# Find AnimationPlayer
	_animation_player = _find_node_of_type(self, "AnimationPlayer") as AnimationPlayer

	# Find Skeleton3D
	_skeleton = _find_node_of_type(self, "Skeleton3D") as Skeleton3D

	# Find all MeshInstance3D nodes
	_mesh_instances = []
	_collect_mesh_instances(self)


func _collect_mesh_instances(node: Node) -> void:
	if node is MeshInstance3D:
		_mesh_instances.append(node as MeshInstance3D)
	for child in node.get_children():
		_collect_mesh_instances(child)


func _find_node_of_type(node: Node, type_name: String) -> Node:
	if node.get_class() == type_name:
		return node
	for child in node.get_children():
		var found := _find_node_of_type(child, type_name)
		if found:
			return found
	return null


func _setup_materials() -> void:
	# Create override materials for shirt and shorts
	_shirt_material = StandardMaterial3D.new()
	_shirt_material.albedo_color = home_shirt_color if is_home_team else away_shirt_color

	_shorts_material = StandardMaterial3D.new()
	_shorts_material.albedo_color = home_shorts_color if is_home_team else away_shorts_color

	# Apply to mesh instances (assumes specific naming convention)
	for mesh in _mesh_instances:
		var mesh_name := mesh.name.to_lower()
		if "shirt" in mesh_name or "jersey" in mesh_name or "top" in mesh_name:
			mesh.material_override = _shirt_material
		elif "shorts" in mesh_name or "pants" in mesh_name:
			mesh.material_override = _shorts_material


func _interpolate_position(delta: float) -> void:
	if current_position.distance_to(target_position) > 0.01:
		current_position = current_position.lerp(target_position, position_lerp_speed * delta)
		global_position = current_position


func _interpolate_rotation(delta: float) -> void:
	var target_rot := Quaternion(Vector3.UP, current_rotation)
	var current_rot := quaternion
	quaternion = current_rot.slerp(target_rot, rotation_lerp_speed * delta)


## Get animation list from AnimationPlayer
func get_available_animations() -> PackedStringArray:
	if _animation_player:
		return _animation_player.get_animation_list()
	return PackedStringArray()


## Check if currently playing animation
func is_playing() -> bool:
	return _animation_player and _animation_player.is_playing()


## Stop current animation
func stop_animation() -> void:
	if _animation_player:
		_animation_player.stop()


## ============================================================================
## Appearance Customization
## ============================================================================


func _setup_number_label() -> void:
	# Create 3D label for jersey number (back of player)
	_number_label = Label3D.new()
	_number_label.name = "JerseyNumber"
	_number_label.text = ""
	_number_label.font_size = 64
	_number_label.pixel_size = 0.01
	_number_label.billboard = BaseMaterial3D.BILLBOARD_DISABLED
	_number_label.no_depth_test = false
	_number_label.modulate = Color.WHITE
	_number_label.outline_modulate = Color.BLACK
	_number_label.outline_size = 8

	# Position on back of player
	_number_label.position = Vector3(0.0, 1.4, -0.15)
	_number_label.rotation_degrees = Vector3(0.0, 180.0, 0.0)

	add_child(_number_label)
	_number_label.visible = false  # Hidden until number is set


func set_jersey_number(number: int) -> void:
	jersey_number = number
	if _number_label:
		if number > 0:
			_number_label.text = str(number)
			_number_label.visible = true
		else:
			_number_label.visible = false


func set_jersey_name(pname: String) -> void:
	jersey_name = pname
	# Could add name label above number in future


func set_goalkeeper(is_gk: bool) -> void:
	is_goalkeeper = is_gk
	apply_team_colors()


func apply_appearance(appearance: RefCounted) -> void:
	if not appearance:
		return

	_appearance = appearance

	# Extract data from PlayerAppearance
	jersey_number = int(appearance.get("jersey_number")) if appearance.get("jersey_number") else 0
	jersey_name = str(appearance.get("jersey_name")) if appearance.get("jersey_name") else ""
	is_goalkeeper = appearance.get("role") == 1 if appearance.get("role") != null else false

	# Apply body scale
	var body_scale: Variant = appearance.get("body_scale")
	if body_scale is Vector3:
		scale = body_scale
	elif body_scale is Dictionary:
		scale = Vector3(
			float(body_scale.get("x", 1.0)), float(body_scale.get("y", 1.0)), float(body_scale.get("z", 1.0))
		)

	# Apply colors
	var shirt_color: Variant = appearance.get("shirt_color")
	var shorts_color: Variant = appearance.get("shorts_color")
	if appearance.get("role") == 1:  # Goalkeeper
		shirt_color = appearance.get("gk_shirt_color")
		shorts_color = appearance.get("gk_shorts_color")

	if shirt_color is Color:
		if is_home_team:
			home_shirt_color = shirt_color
		else:
			away_shirt_color = shirt_color

	if shorts_color is Color:
		if is_home_team:
			home_shorts_color = shorts_color
		else:
			away_shorts_color = shorts_color

	# Apply visuals
	set_jersey_number(jersey_number)
	apply_team_colors()


func apply_team_colors() -> void:
	var shirt_color: Color
	var shorts_color: Color

	if is_goalkeeper:
		shirt_color = gk_shirt_color
		shorts_color = gk_shorts_color
	elif is_home_team:
		shirt_color = home_shirt_color
		shorts_color = home_shorts_color
	else:
		shirt_color = away_shirt_color
		shorts_color = away_shorts_color

	if _shirt_material:
		_shirt_material.albedo_color = shirt_color
	if _shorts_material:
		_shorts_material.albedo_color = shorts_color


func set_team_colors(shirt: Color, shorts: Color) -> void:
	if is_home_team:
		home_shirt_color = shirt
		home_shorts_color = shorts
	else:
		away_shirt_color = shirt
		away_shorts_color = shorts
	apply_team_colors()


func set_goalkeeper_colors(shirt: Color, shorts: Color) -> void:
	gk_shirt_color = shirt
	gk_shorts_color = shorts
	if is_goalkeeper:
		apply_team_colors()
