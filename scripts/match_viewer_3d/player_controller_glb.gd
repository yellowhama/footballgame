extends Node3D
class_name PlayerControllerGLB

## Controls a GLB player model for match visualization.
## Supports dynamically loaded TRELLIS+RigAnything characters.
## Note: GLB models have skeleton but no animations - uses static poses or shared animation library.

signal animation_started(anim_name: String)
signal animation_finished(anim_name: String)
signal model_loaded(character_name: String)

# Character library paths
const CHARACTER_BASE_PATH := "res://assets/soccer_players/characters/"
const DEFAULT_CHARACTER := "main_characters/captain_rigged.glb"

# Character categories
const CHARACTER_CATEGORIES := {
	"main": "main_characters/",
	"male": "male_characters/",
	"female": "basic_female/",
	"npc": "npc_characters/",
	"plain_male": "plain_male/",
	"plain_female": "plain_female/",
}

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
var character_name: String = ""
var current_position: Vector3 = Vector3.ZERO
var target_position: Vector3 = Vector3.ZERO
var current_rotation: float = 0.0  # Y-axis rotation in radians
var current_pose: String = "idle"

# GLB model state
var _model_loaded: bool = false
var _model_node: Node3D = null
var _model_root: Node3D = null

# Skeleton info (RigAnything uses Bone_0 ~ Bone_33)
var _skeleton: Skeleton3D = null
var _mesh_instances: Array[MeshInstance3D] = []

# Animation (optional - for future animation library support)
var _animation_player: AnimationPlayer = null

# Interpolation settings
@export var position_lerp_speed: float = 10.0
@export var rotation_lerp_speed: float = 8.0

# Jersey number label
var _number_label: Label3D = null

# TRELLIS mesh rendering settings
const RENDER_DOUBLE_SIDED := true
const ALPHA_SCISSOR_THRESHOLD := 0.5


func _ready() -> void:
	_model_root = get_node_or_null("ModelRoot")
	if not _model_root:
		_model_root = Node3D.new()
		_model_root.name = "ModelRoot"
		add_child(_model_root)

	_setup_number_label()


func _process(delta: float) -> void:
	_interpolate_position(delta)
	_interpolate_rotation(delta)


## Initialize player with index and team
func setup(idx: int, home_team: bool) -> void:
	player_index = idx
	is_home_team = home_team


## Load GLB character model
func load_character(char_path: String) -> bool:
	# Clear existing model
	if _model_node:
		_model_node.queue_free()
		_model_node = null
		_model_loaded = false

	# Build full path
	var full_path := char_path
	if not char_path.begins_with("res://"):
		full_path = CHARACTER_BASE_PATH + char_path

	# Check if file exists
	if not ResourceLoader.exists(full_path):
		push_warning("GLB character not found: " + full_path)
		return false

	# Load GLB as PackedScene
	var glb_scene: PackedScene = load(full_path)
	if not glb_scene:
		push_error("Failed to load GLB: " + full_path)
		return false

	# Instantiate and add to scene
	_model_node = glb_scene.instantiate()
	_model_node.name = "PlayerModel"
	_model_root.add_child(_model_node)

	# Extract character name from path
	character_name = full_path.get_file().get_basename()

	# Find skeleton and mesh instances
	_find_child_nodes()

	# Configure TRELLIS mesh rendering
	_configure_trellis_materials()

	_model_loaded = true
	model_loaded.emit(character_name)

	return true


## Load random character from category
func load_random_character(category: String = "main") -> bool:
	var category_path := CHARACTER_CATEGORIES.get(category, "main_characters/")
	var dir_path := CHARACTER_BASE_PATH + category_path

	# Get list of GLB files
	var dir := DirAccess.open(dir_path)
	if not dir:
		push_warning("Cannot open character directory: " + dir_path)
		return load_character(DEFAULT_CHARACTER)

	var glb_files: Array[String] = []
	dir.list_dir_begin()
	var file_name := dir.get_next()
	while file_name != "":
		if file_name.ends_with(".glb"):
			glb_files.append(category_path + file_name)
		file_name = dir.get_next()
	dir.list_dir_end()

	if glb_files.is_empty():
		return load_character(DEFAULT_CHARACTER)

	# Pick random
	var random_idx := randi() % glb_files.size()
	return load_character(glb_files[random_idx])


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


## Play pose (static for GLB without animations)
func play_pose(pose_type: Variant) -> void:
	var pose_key: String

	if pose_type is int:
		pose_key = str(pose_type)
	else:
		pose_key = str(pose_type).to_lower()

	# Skip if same pose
	if current_pose == pose_key:
		return
	current_pose = pose_key

	# If we have an animation player with animations, use it
	if _animation_player and _animation_player.get_animation_list().size() > 0:
		_play_animation_for_pose(pose_key)
	else:
		# Static pose - no animation
		animation_started.emit(pose_key)


func _play_animation_for_pose(pose_key: String) -> void:
	# Animation library mapping (for future use)
	var anim_map := {
		"idle": "idle",
		"0": "idle",
		"run": "run",
		"2": "run",
		"walk": "walk",
		"1": "walk",
		"pass": "pass",
		"4": "pass",
		"shot": "kick",
		"5": "kick",
	}

	var anim_name: String = anim_map.get(pose_key, "idle")

	if _animation_player.has_animation(anim_name):
		_animation_player.play(anim_name)
		animation_started.emit(anim_name)


func _find_child_nodes() -> void:
	if not _model_node:
		return

	# Find AnimationPlayer (if any)
	_animation_player = _find_node_of_type(_model_node, "AnimationPlayer") as AnimationPlayer

	# Find Skeleton3D
	_skeleton = _find_node_of_type(_model_node, "Skeleton3D") as Skeleton3D

	# Find all MeshInstance3D nodes
	_mesh_instances = []
	_collect_mesh_instances(_model_node)


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


## Configure TRELLIS mesh materials for proper rendering
func _configure_trellis_materials() -> void:
	for mesh_inst in _mesh_instances:
		# Process each surface material
		var mesh: Mesh = mesh_inst.mesh
		if not mesh:
			continue

		for i in range(mesh.get_surface_count()):
			var mat: Material = mesh_inst.get_active_material(i)
			if mat is StandardMaterial3D:
				var std_mat := mat as StandardMaterial3D

				# Create override material to not modify original
				var override_mat := std_mat.duplicate() as StandardMaterial3D

				# TRELLIS mesh fixes:
				# 1. Force opaque blend mode (TRELLIS exports with HASHED)
				override_mat.transparency = BaseMaterial3D.TRANSPARENCY_DISABLED
				override_mat.blend_mode = BaseMaterial3D.BLEND_MODE_MIX

				# 2. Disable backface culling (TRELLIS sparse mesh needs double-sided)
				override_mat.cull_mode = BaseMaterial3D.CULL_DISABLED

				# 3. Ensure shadows work
				override_mat.shadow_to_opacity = false

				# Apply override
				mesh_inst.set_surface_override_material(i, override_mat)

			elif mat is BaseMaterial3D:
				# Generic material handling
				var base_mat := mat as BaseMaterial3D
				var override_mat := base_mat.duplicate() as BaseMaterial3D
				override_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
				mesh_inst.set_surface_override_material(i, override_mat)


func _interpolate_position(delta: float) -> void:
	if current_position.distance_to(target_position) > 0.01:
		current_position = current_position.lerp(target_position, position_lerp_speed * delta)
		global_position = current_position


func _interpolate_rotation(delta: float) -> void:
	var target_rot := Quaternion(Vector3.UP, current_rotation)
	var current_rot := quaternion
	quaternion = current_rot.slerp(target_rot, rotation_lerp_speed * delta)


## Get skeleton bone count (RigAnything uses 34 bones: Bone_0 ~ Bone_33)
func get_bone_count() -> int:
	if _skeleton:
		return _skeleton.get_bone_count()
	return 0


## Get skeleton bone names
func get_bone_names() -> PackedStringArray:
	var names: PackedStringArray = []
	if _skeleton:
		for i in range(_skeleton.get_bone_count()):
			names.append(_skeleton.get_bone_name(i))
	return names


## Check if model is loaded
func is_model_loaded() -> bool:
	return _model_loaded


## Get available animations (if any)
func get_available_animations() -> PackedStringArray:
	if _animation_player:
		return _animation_player.get_animation_list()
	return PackedStringArray()


func is_playing() -> bool:
	return _animation_player and _animation_player.is_playing()


func stop_animation() -> void:
	if _animation_player:
		_animation_player.stop()


# ============================================================================
# Appearance Customization
# ============================================================================


func _setup_number_label() -> void:
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
	_number_label.visible = false


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


func set_goalkeeper(is_gk: bool) -> void:
	is_goalkeeper = is_gk


func apply_team_colors() -> void:
	# TRELLIS models have baked textures - color override is limited
	# For now, just update jersey number label color based on team
	var label_color: Color = home_shirt_color if is_home_team else away_shirt_color
	if is_goalkeeper:
		label_color = gk_shirt_color

	if _number_label:
		# Use contrasting color for label
		var brightness := label_color.r * 0.299 + label_color.g * 0.587 + label_color.b * 0.114
		_number_label.modulate = Color.WHITE if brightness < 0.5 else Color.BLACK
		_number_label.outline_modulate = Color.BLACK if brightness < 0.5 else Color.WHITE


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


## Apply model scale (for body variants)
func set_model_scale(scale_vec: Vector3) -> void:
	if _model_root:
		_model_root.scale = scale_vec
