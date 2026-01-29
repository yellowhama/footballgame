extends CanvasLayer
class_name NESTextOverlay
##
## NESTextOverlay - NES 스타일 텍스트 오버레이
##
## 기능:
##   - NES 8x8 픽셀 폰트 스타일의 게임 이벤트 텍스트
##   - "HALF TIME!", "FULL TIME!", "OFFSIDE!", "GOAL!" 등
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
##

#region Constants
const NES_SPRITESHEET_PATH := "res://assets/sprites/nes/Football Full Sprite Sheet.png"
const NES_SCALE := 4.0  ## 4배 확대 (가독성)
const DISPLAY_DURATION := 2.5

## 스프라이트 시트 내 텍스트 영역 (Football Full Sprite Sheet.png 1024x1024 기준)
## 텍스트는 스프라이트 하단 좌측에 위치 (y=976~1016)
const TEXT_REGIONS := {
	"half_time": Rect2(720, 976, 80, 8),  ## "HALF TIME!"
	"full_time": Rect2(720, 984, 80, 8),  ## "FULL TIME!"
	"offside": Rect2(720, 992, 64, 8),  ## "OFFSIDE!"
	"goal": Rect2(720, 1000, 40, 8),  ## "GOAL!"
}
#endregion

#region Node References
var _overlay_container: CenterContainer = null
var _text_sprite: Sprite2D = null
var _background: ColorRect = null
#endregion

#region NES Assets
var _nes_spritesheet: Texture2D = null
var _text_textures: Dictionary = {}  ## String -> AtlasTexture
#endregion

#region State
var _is_showing: bool = false
var _current_tween: Tween = null
#endregion

#region Signals
signal overlay_finished
signal text_shown(text_type: String)
#endregion


func _ready() -> void:
	layer = 100  # 최상위 레이어
	_load_textures()
	_create_ui_elements()
	_hide_all()


func _load_textures() -> void:
	if not ResourceLoader.exists(NES_SPRITESHEET_PATH):
		push_warning("[NESTextOverlay] NES spritesheet not found: %s" % NES_SPRITESHEET_PATH)
		return

	_nes_spritesheet = load(NES_SPRITESHEET_PATH)

	## AtlasTexture 생성
	for text_type in TEXT_REGIONS:
		var atlas := AtlasTexture.new()
		atlas.atlas = _nes_spritesheet
		atlas.region = TEXT_REGIONS[text_type]
		_text_textures[text_type] = atlas


func _create_ui_elements() -> void:
	## 배경 (반투명 검정)
	_background = ColorRect.new()
	_background.name = "Background"
	_background.color = Color(0, 0, 0, 0.6)
	_background.set_anchors_preset(Control.PRESET_FULL_RECT)
	_background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_background)

	## 중앙 정렬 컨테이너
	_overlay_container = CenterContainer.new()
	_overlay_container.name = "OverlayContainer"
	_overlay_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	_overlay_container.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_overlay_container)

	## 텍스트 스프라이트
	_text_sprite = Sprite2D.new()
	_text_sprite.name = "TextSprite"
	_text_sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	_text_sprite.scale = Vector2(NES_SCALE, NES_SCALE)
	_overlay_container.add_child(_text_sprite)


func _hide_all() -> void:
	_background.visible = false
	_text_sprite.visible = false
	_is_showing = false
	if _current_tween:
		_current_tween.kill()
		_current_tween = null


#region Public API


## 하프타임 오버레이
func show_half_time() -> void:
	await _show_text_overlay("half_time", DISPLAY_DURATION + 0.5)


## 풀타임 오버레이
func show_full_time() -> void:
	await _show_text_overlay("full_time", DISPLAY_DURATION + 0.5)


## 오프사이드 오버레이
func show_offside() -> void:
	await _show_text_overlay("offside", DISPLAY_DURATION)


## 골 오버레이
func show_goal() -> void:
	await _show_text_overlay("goal", DISPLAY_DURATION)


## 숨기기
func hide_overlay() -> void:
	_hide_all()


## 현재 표시 중인지
func is_showing() -> bool:
	return _is_showing


#endregion

#region Internal


func _show_text_overlay(text_type: String, duration: float) -> void:
	if _is_showing:
		return

	if not _text_textures.has(text_type):
		push_warning("[NESTextOverlay] Unknown text type: %s" % text_type)
		return

	_is_showing = true
	_background.visible = true
	_text_sprite.visible = true
	_text_sprite.texture = _text_textures[text_type]

	## 깜빡임 효과 (NES 스타일)
	_text_sprite.modulate = Color.WHITE
	_current_tween = create_tween()
	_current_tween.set_loops(3)
	_current_tween.tween_property(_text_sprite, "modulate", Color(1, 1, 0), 0.15)  # 노란색
	_current_tween.tween_property(_text_sprite, "modulate", Color.WHITE, 0.15)

	text_shown.emit(text_type)

	await get_tree().create_timer(duration).timeout
	_hide_all()
	overlay_finished.emit()

#endregion
