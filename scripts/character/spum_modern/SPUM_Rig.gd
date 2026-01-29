@tool
extends Node2D

class_name SPUM_Rig

# Skeleton References
@onready var skeleton: Skeleton2D = $Skeleton2D
@onready var anim_player: AnimationPlayer = $AnimationPlayer

# Part Sprites (Attached to Bones)
@onready var _body_sprite = $Skeleton2D/Hip/BodySprite
@onready var _pant_sprite = $Skeleton2D/Hip/PantSprite
@onready var _cloth_sprite = $Skeleton2D/Hip/Spine/ClothSprite
@onready var _head_sprite = $Skeleton2D/Hip/Spine/Head/HeadSprite
@onready var _face_hair_sprite = $Skeleton2D/Hip/Spine/Head/FaceHair
@onready var _hair_sprite = $Skeleton2D/Hip/Spine/Head/Hair
@onready var _arm_l_sprite = $Skeleton2D/Hip/Spine/ArmL/ArmLSprite
@onready var _arm_r_sprite = $Skeleton2D/Hip/Spine/ArmR/ArmRSprite
@onready var _foot_l_sprite = $Skeleton2D/Hip/LegL/FootLSprite
@onready var _foot_r_sprite = $Skeleton2D/Hip/LegR/FootRSprite
# Add Weapon/Back/Helmet references as needed (omitted for brevity)

# Atlas Constants (from asset.meta cleanup)
const RECT_HEAD = Rect2(0, 16, 17, 15)
const RECT_BODY = Rect2(2, 6, 12, 10)
const RECT_ARM_R = Rect2(15, 8, 6, 7)
const RECT_ARM_L = Rect2(15, 0, 6, 7)
const RECT_FOOT_R = Rect2(3, 0, 4, 7)
const RECT_FOOT_L = Rect2(9, 0, 4, 7)

# Offsets are now handled by Bone Rest positions in the Rig,
# so sprites should generally be centered (0,0) or have small local tweaks.
# BUT, the AtlasTexture slice doesn't reset origin.
# We will use simple offsets to align the sliced texture center to the bone.


func _ready():
	pass


func set_appearance_data(data: Dictionary):
	# Body Split
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

	if data.has("kit_colors"):
		_apply_kit_colors(data.kit_colors)


func _load_body_parts(folder_path: String, file_name: String):
	var full_path = "res://assets/sprites/spum_modern/%s/%s.png" % [folder_path, file_name]
	if not ResourceLoader.exists(full_path):
		return
	var tex = load(full_path)

	_set_atlas_part(_body_sprite, tex, RECT_BODY)
	_set_atlas_part(_head_sprite, tex, RECT_HEAD)
	_set_atlas_part(_arm_l_sprite, tex, RECT_ARM_L)
	_set_atlas_part(_arm_r_sprite, tex, RECT_ARM_R)
	# Swap Feet: Foot_R (x=3, Left side of texture) -> FootL (Screen Left)
	_set_atlas_part(_foot_l_sprite, tex, RECT_FOOT_R)
	_set_atlas_part(_foot_r_sprite, tex, RECT_FOOT_L)


func _set_atlas_part(sprite: Sprite2D, atlas_tex: Texture2D, region: Rect2):
	if not sprite:
		return
	var at = AtlasTexture.new()
	at.atlas = atlas_tex
	at.region = region
	at.filter_clip = true
	sprite.texture = at
	# Center the sprite on the bone
	# sprite.offset should be used if needed?
	# For now assume default.


func _load_part(sprite: Sprite2D, folder_path: String, file_name: String):
	if not sprite:
		return
	if file_name == "" or file_name == "None":
		sprite.texture = null
		return
	var full_path = "res://assets/sprites/spum_modern/%s/%s.png" % [folder_path, file_name]
	if ResourceLoader.exists(full_path):
		sprite.texture = load(full_path)


func _apply_kit_colors(colors: Dictionary):
	if _cloth_sprite:
		_cloth_sprite.self_modulate = Color(colors.get("primary", "#ffffff"))
	if _pant_sprite:
		_pant_sprite.self_modulate = Color(colors.get("secondary", "#ffffff"))


func play_anim(anim_name: String):
	if anim_player.has_animation(anim_name):
		anim_player.play(anim_name)
