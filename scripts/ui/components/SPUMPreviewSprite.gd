@tool
extends Node2D

class_name SPUMPreviewSprite

# --- References ---
@onready var _body_sprite = $Body
@onready var _head_sprite = $Head
@onready var _arm_l_sprite = $ArmL
@onready var _arm_r_sprite = $ArmR
@onready var _foot_l_sprite = $FootL
@onready var _foot_r_sprite = $FootR

@onready var _eyes_sprite = $Eyes
@onready var _hair_sprite = $Hair
@onready var _face_hair_sprite = $FaceHair
@onready var _cloth_sprite = $Cloth
@onready var _pant_sprite = $Pant
@onready var _back_sprite = $Back
@onready var _helmet_sprite = $Helmet
@onready var _armor_sprite = $Armor
@onready var _weapon_l_sprite = $WeaponL
@onready var _weapon_r_sprite = $WeaponR

# --- Animation Hack ---
var _current_action: String = MatchActions.IDLE
var _anim_timer: float = 0.0
# For "Kick Hack": if action is SHOOT/PASS, we might freeze/tween the container.
# Since we don't have actual animation frames yet, we will just use static poses.


func _ready():
	# Default setup
	pass


func set_action(action_id: String):
	_current_action = action_id
	# TODO: Implement animation mapping when frames are available.
	# For now, just reset to idle pose (0_idle usually)
	# or log warning.
	print("SPUMPreview: Action set to ", action_id)

	# Temporary "Kick Hack" visualization (just a wobble for now)
	if action_id == MatchActions.SHOOT or action_id == MatchActions.PASS_SHORT:
		var tween = create_tween()
		tween.tween_property(self, "rotation", deg_to_rad(15), 0.1)
		tween.tween_property(self, "rotation", deg_to_rad(-15), 0.2)
		tween.tween_property(self, "rotation", 0, 0.1)


func set_appearance_data(data: Dictionary):
	# Body Split Logic
	if data.has("body"):
		_load_body_parts("bases/BodySource", data.body)

	if data.has("hair"):
		_load_part(_hair_sprite, "items/hair/0_Hair", data.hair)
	if data.has("face_hair"):
		_load_part(_face_hair_sprite, "items/hair/1_FaceHair", data.face_hair)
	if data.has("cloth"):
		_load_part(_cloth_sprite, "items/cloth/2_Cloth", data.cloth)
	if data.has("pant"):
		_load_part(_pant_sprite, "items/pant/3_Pant", data.pant)
	if data.has("helmet"):
		_load_part(_helmet_sprite, "items/helmet/4_Helmet", data.helmet)

	# Apply Kit Colors if needed
	if data.has("kit_colors"):
		_apply_kit_colors(data.kit_colors)


# Atlas Rects (from asset.meta)
const RECT_HEAD = Rect2(0, 16, 17, 15)  # Pivot (0.5,0.5)
const RECT_BODY = Rect2(2, 6, 12, 10)
const RECT_ARM_R = Rect2(15, 8, 6, 7)
const RECT_ARM_L = Rect2(15, 0, 6, 7)
const RECT_FOOT_R = Rect2(3, 0, 4, 7)
const RECT_FOOT_L = Rect2(9, 0, 4, 7)

# Visual Assembly Offsets (Trial & Error / Standard Humanoid)
# Assuming Center (0,0) is Body Center
const OFFSET_HEAD = Vector2(0, -6)
const OFFSET_ARM_L = Vector2(-7, -2)
const OFFSET_ARM_R = Vector2(7, -2)
const OFFSET_FOOT_L = Vector2(-3, 8)
const OFFSET_FOOT_R = Vector2(3, 8)


func _load_body_parts(folder_path: String, file_name: String):
	if file_name == "" or file_name == "None":
		_body_sprite.texture = null
		_head_sprite.texture = null
		return

	var full_path = "res://assets/sprites/spum_modern/%s/%s.png" % [folder_path, file_name]
	if not ResourceLoader.exists(full_path):
		print("SPUMPreview: Failed to load body ", full_path)
		return

	var tex = load(full_path)

	_set_atlas_part(_body_sprite, tex, RECT_BODY, Vector2(0, 0))
	_set_atlas_part(_head_sprite, tex, RECT_HEAD, OFFSET_HEAD)
	_set_atlas_part(_arm_l_sprite, tex, RECT_ARM_L, OFFSET_ARM_L)
	_set_atlas_part(_arm_r_sprite, tex, RECT_ARM_R, OFFSET_ARM_R)
	_set_atlas_part(_foot_l_sprite, tex, RECT_FOOT_L, OFFSET_FOOT_L)
	_set_atlas_part(_foot_r_sprite, tex, RECT_FOOT_R, OFFSET_FOOT_R)


func _set_atlas_part(sprite: Sprite2D, atlas_tex: Texture2D, region: Rect2, offset: Vector2):
	if not sprite:
		return
	var at = AtlasTexture.new()
	at.atlas = atlas_tex
	at.region = region
	at.filter_clip = true
	sprite.texture = at
	sprite.position = offset  # Simple local offset


func _load_part(sprite: Sprite2D, folder_path: String, file_name: String):
	if not sprite:
		return
	if file_name == "" or file_name == "None":
		sprite.texture = null
		return

	var full_path = "res://assets/sprites/spum_modern/%s/%s.png" % [folder_path, file_name]
	if ResourceLoader.exists(full_path):
		sprite.texture = load(full_path)
	else:
		# Try searching recursively? Or just fail silently.
		# Trying flatter structure if failed
		full_path = "res://assets/sprites/spum_modern/%s/%s.png" % [folder_path.split("/")[-1], file_name]  # Fallback
		if ResourceLoader.exists(full_path):
			sprite.texture = load(full_path)
		else:
			print("SPUMPreview: Failed to load ", full_path)


func _apply_kit_colors(colors: Dictionary):
	# Assuming KitPattern.gdshader is attached to Cloth/Pant
	# For now, just self_modulate as fallback if shader missing
	if _cloth_sprite.material:
		_cloth_sprite.material.set_shader_parameter("primary_color", Color(colors.get("primary", "#ffffff")))
		_cloth_sprite.material.set_shader_parameter("secondary_color", Color(colors.get("secondary", "#ffffff")))
	else:
		_cloth_sprite.self_modulate = Color(colors.get("primary", "#ffffff"))
