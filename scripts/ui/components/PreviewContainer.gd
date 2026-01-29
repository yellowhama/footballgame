## PreviewContainer.gd
## 잔디 배경 + 캐릭터 레이어 컨테이너
## 캐릭터 생성, 마이팀 설정 화면에서 공통으로 사용
##
## 사용법:
##   - 단일 캐릭터: add_single_character("blonde")
##   - 팀 미리보기: add_team_preview(8)
##
## 참조: assets/ui/2025-12-08_CHARACTER_SPRITE_USAGE_FOR_UI.md

class_name PreviewContainer
extends Control

## 비네트 효과 사용 여부
@export var vignette_enabled: bool = true

## 비네트 강도
@export_range(0.0, 1.0) var vignette_intensity: float = 0.4

## 잔디 스케일 (타일 크기)
@export var grass_scale: float = 2.0

## 내부 노드
var _grass_bg: TextureRect
var _characters_layer: Node2D
var _vignette: ColorRect
var _single_character: Node2D = null
var _team_preview: Control = null

const GRASS_TEXTURE := "res://assets/socceralia/grass_tiles/grass8.png"
const VIGNETTE_SHADER := "res://assets/shaders/vignette.gdshader"
const CHARACTER_SCENE := "res://scenes/ui/components/CharacterPreviewSprite.tscn"
const TEAM_PREVIEW_SCENE := "res://scenes/ui/components/TeamPreviewBackground.tscn"


func _ready() -> void:
	clip_contents = true
	_setup_grass_background()
	_setup_characters_layer()
	if vignette_enabled:
		_setup_vignette()


func _setup_grass_background() -> void:
	_grass_bg = TextureRect.new()
	_grass_bg.name = "GrassBackground"

	if ResourceLoader.exists(GRASS_TEXTURE):
		_grass_bg.texture = load(GRASS_TEXTURE)
	else:
		push_warning("[PreviewContainer] Grass texture not found: %s" % GRASS_TEXTURE)

	_grass_bg.stretch_mode = TextureRect.STRETCH_TILE
	_grass_bg.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	_grass_bg.texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED

	# Full rect anchors
	_grass_bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	_grass_bg.offset_left = 0
	_grass_bg.offset_top = 0
	_grass_bg.offset_right = 0
	_grass_bg.offset_bottom = 0

	add_child(_grass_bg)


func _setup_characters_layer() -> void:
	_characters_layer = Node2D.new()
	_characters_layer.name = "CharactersLayer"
	add_child(_characters_layer)


func _setup_vignette() -> void:
	_vignette = ColorRect.new()
	_vignette.name = "Vignette"

	# Full rect anchors
	_vignette.set_anchors_preset(Control.PRESET_FULL_RECT)
	_vignette.offset_left = 0
	_vignette.offset_top = 0
	_vignette.offset_right = 0
	_vignette.offset_bottom = 0

	if ResourceLoader.exists(VIGNETTE_SHADER):
		var mat := ShaderMaterial.new()
		mat.shader = load(VIGNETTE_SHADER)
		mat.set_shader_parameter("intensity", vignette_intensity)
		mat.set_shader_parameter("softness", 0.5)
		_vignette.material = mat
	else:
		# 셰이더 없으면 간단한 반투명 검정
		_vignette.color = Color(0, 0, 0, 0.1)

	add_child(_vignette)


## === 외부 API ===


func add_single_character(hair_folder: String = "black") -> Node2D:
	## 단일 캐릭터 추가 (캐릭터 생성 화면용)
	_clear_characters()

	if ResourceLoader.exists(CHARACTER_SCENE):
		_single_character = load(CHARACTER_SCENE).instantiate()
	else:
		_single_character = CharacterPreviewSprite.new()

	_single_character.hair_folder = hair_folder

	# 중앙 배치
	_single_character.position = size / 2

	_characters_layer.add_child(_single_character)
	return _single_character


func add_team_preview(player_count: int = 8) -> Control:
	## 팀 미리보기 추가 (마이팀 설정 화면용)
	_clear_characters()

	if ResourceLoader.exists(TEAM_PREVIEW_SCENE):
		_team_preview = load(TEAM_PREVIEW_SCENE).instantiate()
	else:
		_team_preview = TeamPreviewBackground.new()

	_team_preview.player_count = player_count

	# Full rect
	_team_preview.set_anchors_preset(Control.PRESET_FULL_RECT)
	_team_preview.size = size

	_characters_layer.add_child(_team_preview)
	return _team_preview


func get_single_character() -> Node2D:
	return _single_character


func get_team_preview() -> Control:
	return _team_preview


func set_team_colors(primary: Color, secondary: Color, pattern: int = 0) -> void:
	if _single_character and _single_character.has_method("set_team_colors"):
		_single_character.set_team_colors(primary, secondary, pattern)
	if _team_preview and _team_preview.has_method("set_team_colors"):
		_team_preview.set_team_colors(primary, secondary, pattern)


func set_hair_folder(folder: String) -> void:
	if _single_character and _single_character.has_method("set_hair_folder"):
		_single_character.set_hair_folder(folder)


func play_animation(anim: int) -> void:
	## CharacterPreviewSprite.AnimState 값을 받아서 재생
	if _single_character and _single_character.has_method("play_animation"):
		_single_character.play_animation(anim)


func _clear_characters() -> void:
	if _single_character:
		_single_character.queue_free()
		_single_character = null
	if _team_preview:
		_team_preview.queue_free()
		_team_preview = null


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		# 크기 변경 시 캐릭터 위치 조정
		if _single_character:
			_single_character.position = size / 2
		if _team_preview:
			_team_preview.size = size
