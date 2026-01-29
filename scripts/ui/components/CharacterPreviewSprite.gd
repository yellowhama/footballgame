## CharacterPreviewSprite.gd
## 단일 캐릭터를 애니메이션과 함께 표시하는 UI 컴포넌트
## Socceralia 스프라이트 사용
##
## 사용법:
##   var preview = CharacterPreviewSprite.new()
##   preview.hair_folder = "blonde"
##   preview.set_team_colors(Color.RED, Color.WHITE)
##   preview.play_animation(AnimState.RUN)
##
## 참조: assets/ui/2025-12-08_CHARACTER_SPRITE_USAGE_FOR_UI.md

class_name CharacterPreviewSprite
extends Node2D

signal appearance_changed

## 헤어/스프라이트 폴더 (black, blonde, redhead, other, gk)
@export var hair_folder: String = "black":
	set(value):
		if hair_folder != value:
			hair_folder = value
			if is_node_ready():
				_load_textures_for_hair_folder()
				appearance_changed.emit()

## 팀 컬러
@export var primary_color: Color = Color.RED:
	set(value):
		primary_color = value
		if is_node_ready():
			_apply_team_color()

@export var secondary_color: Color = Color.WHITE:
	set(value):
		secondary_color = value
		if is_node_ready():
			_apply_team_color()

## 패턴 타입 (0=단색, 1=가로줄, 2=세로줄, 3=체크, 4=대각선)
@export_range(0, 4) var pattern_type: int = 0:
	set(value):
		pattern_type = value
		if is_node_ready():
			_apply_team_color()

## 애니메이션 상태
enum AnimState { IDLE, RUN, KICK, CELEBRATE }
var current_anim: AnimState = AnimState.IDLE

## 내부 노드
var _sprite: AnimatedSprite2D
var _shadow: Sprite2D
var _sprite_frames: SpriteFrames

## 스케일 (UI용: 16px → 64px)
const UI_SCALE := Vector2(4.0, 4.0)

## 스프라이트 경로 상수
const SPRITE_BASE_PATH := "res://assets/sprites/socceralia/player/"
const SHADOW_TEXTURE := "res://assets/socceralia/ball_shadow.png"
const KIT_SHADER := "res://assets/shaders/KitPattern.gdshader"

## 프레임 매핑 (1-indexed from spec)
const FRAME_IDLE := [1]
const FRAME_RUN := [2, 3, 4, 5]
const FRAME_KICK := [10]
const FRAME_CELEBRATE := [17]


func _ready() -> void:
	_setup_nodes()
	_setup_sprite_frames()
	_load_textures_for_hair_folder()
	_apply_team_color()
	play_animation(AnimState.IDLE)


func _setup_nodes() -> void:
	# Shadow
	_shadow = Sprite2D.new()
	_shadow.name = "Shadow"
	if ResourceLoader.exists(SHADOW_TEXTURE):
		_shadow.texture = load(SHADOW_TEXTURE)
	_shadow.modulate = Color(0, 0, 0, 0.3)
	_shadow.position = Vector2(2, 4)
	_shadow.scale = UI_SCALE
	_shadow.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	add_child(_shadow)

	# Sprite
	_sprite = AnimatedSprite2D.new()
	_sprite.name = "AnimatedSprite2D"
	_sprite.scale = UI_SCALE
	_sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	add_child(_sprite)


func _setup_sprite_frames() -> void:
	_sprite_frames = SpriteFrames.new()

	# Idle animation
	_sprite_frames.add_animation("idle")
	_sprite_frames.set_animation_speed("idle", 1)
	_sprite_frames.set_animation_loop("idle", true)

	# Run animation
	_sprite_frames.add_animation("run")
	_sprite_frames.set_animation_speed("run", 8)
	_sprite_frames.set_animation_loop("run", true)

	# Kick animation
	_sprite_frames.add_animation("kick")
	_sprite_frames.set_animation_speed("kick", 6)
	_sprite_frames.set_animation_loop("kick", false)

	# Celebrate animation
	_sprite_frames.add_animation("celebrate")
	_sprite_frames.set_animation_speed("celebrate", 4)
	_sprite_frames.set_animation_loop("celebrate", true)

	_sprite.sprite_frames = _sprite_frames


func _load_textures_for_hair_folder() -> void:
	if not _sprite_frames:
		return

	var folder := hair_folder
	# GK 폴더는 파일명 형식이 다름 (player_gk-N.png vs player-black-N.png)
	var is_gk := folder == "gk"
	var file_prefix: String
	if is_gk:
		file_prefix = "%s%s/player_gk-" % [SPRITE_BASE_PATH, folder]
	else:
		file_prefix = "%s%s/player-%s-" % [SPRITE_BASE_PATH, folder, folder]

	# Idle
	_sprite_frames.clear("idle")
	for frame_num in FRAME_IDLE:
		var tex := _load_texture(file_prefix + "%d.png" % frame_num)
		if tex:
			_sprite_frames.add_frame("idle", tex)

	# Run
	_sprite_frames.clear("run")
	for frame_num in FRAME_RUN:
		var tex := _load_texture(file_prefix + "%d.png" % frame_num)
		if tex:
			_sprite_frames.add_frame("run", tex)

	# Kick
	_sprite_frames.clear("kick")
	for frame_num in FRAME_KICK:
		var tex := _load_texture(file_prefix + "%d.png" % frame_num)
		if tex:
			_sprite_frames.add_frame("kick", tex)

	# Celebrate
	_sprite_frames.clear("celebrate")
	for frame_num in FRAME_CELEBRATE:
		var tex := _load_texture(file_prefix + "%d.png" % frame_num)
		if tex:
			_sprite_frames.add_frame("celebrate", tex)

	# 현재 애니메이션 다시 재생
	play_animation(current_anim)


func _load_texture(path: String) -> Texture2D:
	if ResourceLoader.exists(path):
		return load(path)
	push_warning("[CharacterPreviewSprite] Texture not found: %s" % path)
	return null


func _apply_team_color() -> void:
	if not _sprite:
		return

	if not ResourceLoader.exists(KIT_SHADER):
		push_warning("[CharacterPreviewSprite] Shader not found: %s" % KIT_SHADER)
		return

	var mat := ShaderMaterial.new()
	mat.shader = load(KIT_SHADER)
	mat.set_shader_parameter("primary_color", primary_color)
	mat.set_shader_parameter("secondary_color", secondary_color)
	mat.set_shader_parameter("pattern_type", pattern_type)
	mat.set_shader_parameter("key_color", Color.WHITE)
	mat.set_shader_parameter("tolerance", 0.15)
	mat.set_shader_parameter("preserve_shading", true)
	_sprite.material = mat


func play_animation(state: AnimState) -> void:
	current_anim = state
	if not _sprite:
		return

	match state:
		AnimState.IDLE:
			_sprite.play("idle")
		AnimState.RUN:
			_sprite.play("run")
		AnimState.KICK:
			_sprite.play("kick")
		AnimState.CELEBRATE:
			_sprite.play("celebrate")


## 외부 API
func set_hair_folder(folder: String) -> void:
	hair_folder = folder


func set_team_colors(primary: Color, secondary: Color, pattern: int = -1) -> void:
	primary_color = primary
	secondary_color = secondary
	if pattern >= 0:
		pattern_type = pattern


func set_pattern_type(pattern: int) -> void:
	pattern_type = pattern


func set_scale_factor(factor: float) -> void:
	## UI 스케일 조정 (기본 4.0 = 64px)
	var new_scale := Vector2(factor, factor)
	if _sprite:
		_sprite.scale = new_scale
	if _shadow:
		_shadow.scale = new_scale


func flip_h(flipped: bool) -> void:
	if _sprite:
		_sprite.flip_h = flipped
