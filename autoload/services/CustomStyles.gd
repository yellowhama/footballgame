extends Node
# 커스텀 스타일 헬퍼 클래스


# 안전한 ThemeManager 접근을 위한 헬퍼 함수들
func _get_theme_color(color_name: String, default_color: Color) -> Color:
	# ThemeManager가 autoload로 등록되어 있으므로 직접 접근
	if ThemeManager and ThemeManager.has_method("get"):
		var color_value = ThemeManager.get(color_name)
		if color_value != null:
			return color_value
	return default_color


func _get_theme_constant(constant_name: String, default_value: int) -> int:
	# ThemeManager가 autoload로 등록되어 있으므로 직접 접근
	if ThemeManager and ThemeManager.has_method("get"):
		var constant_value = ThemeManager.get(constant_name)
		if constant_value != null:
			return constant_value
	return default_value


## 버튼 스타일 프리셋
func create_primary_button() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("PRIMARY", Color(0.2, 0.6, 1.0))
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 12
	style.content_margin_bottom = 12
	style.shadow_size = 4
	style.shadow_offset = Vector2(2, 2)
	style.shadow_color = Color(0, 0, 0, 0.3)
	return style


func create_secondary_button() -> StyleBoxFlat:
	var style = create_primary_button()
	style.bg_color = _get_theme_color("SECONDARY", Color(0.8, 0.8, 0.8))
	return style


func create_danger_button() -> StyleBoxFlat:
	var style = create_primary_button()
	style.bg_color = _get_theme_color("DANGER", Color(0.8, 0.2, 0.2))
	return style


func create_success_button() -> StyleBoxFlat:
	var style = create_primary_button()
	style.bg_color = _get_theme_color("SUCCESS", Color(0.2, 0.8, 0.2))
	return style


func create_ghost_button() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0, 0, 0, 0)
	style.border_width_left = 2
	style.border_width_right = 2
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.border_color = _get_theme_color("PRIMARY", Color(0.2, 0.6, 1.0))
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 12
	style.content_margin_bottom = 12
	return style


## 패널 스타일 프리셋
func create_card_panel() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BG_SURFACE", Color(0.15, 0.15, 0.2))
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.shadow_size = 6
	style.shadow_offset = Vector2(0, 4)
	style.shadow_color = Color(0, 0, 0, 0.2)
	style.content_margin_left = 16
	style.content_margin_right = 16
	style.content_margin_top = 16
	style.content_margin_bottom = 16
	return style


func create_header_panel() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BG_MEDIUM", Color(0.2, 0.2, 0.25))
	style.corner_radius_top_left = 0
	style.corner_radius_top_right = 0
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.shadow_size = 4
	style.shadow_offset = Vector2(0, 2)
	style.shadow_color = Color(0, 0, 0, 0.3)
	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 16
	style.content_margin_bottom = 16
	return style


func create_highlight_panel(color: Color = Color(0.9, 0.4, 0.9)) -> StyleBoxFlat:
	var style = create_card_panel()
	style.border_width_left = 3
	style.border_width_right = 3
	style.border_width_top = 3
	style.border_width_bottom = 3
	style.border_color = color
	return style


## 프로그레스바 스타일
func create_fatigue_bar_bg() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.2, 0.2, 0.3)
	style.corner_radius_top_left = 10
	style.corner_radius_top_right = 10
	style.corner_radius_bottom_left = 10
	style.corner_radius_bottom_right = 10
	return style


func create_fatigue_bar_fill(fatigue: float) -> StyleBoxFlat:
	var style = StyleBoxFlat.new()

	# 피로도에 따른 색상 변경
	if fatigue >= 80:
		style.bg_color = _get_theme_color("DANGER", Color(0.8, 0.2, 0.2))
	elif fatigue >= 60:
		style.bg_color = _get_theme_color("WARNING", Color(1.0, 0.6, 0.0))
	elif fatigue >= 40:
		style.bg_color = _get_theme_color("PASTEL_YELLOW", Color(1.0, 1.0, 0.6))
	else:
		style.bg_color = _get_theme_color("SUCCESS", Color(0.2, 0.8, 0.2))

	style.corner_radius_top_left = 10
	style.corner_radius_top_right = 10
	style.corner_radius_bottom_left = 10
	style.corner_radius_bottom_right = 10
	return style


## 훈련 카드 스타일
func create_training_card_normal() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BG_SURFACE", Color(0.15, 0.15, 0.2))
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.border_width_left = 2
	style.border_width_right = 2
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.border_color = Color(1, 1, 1, 0.1)
	style.content_margin_left = 16
	style.content_margin_right = 16
	style.content_margin_top = 16
	style.content_margin_bottom = 16
	return style


func create_training_card_hover() -> StyleBoxFlat:
	var style = create_training_card_normal()
	style.border_color = _get_theme_color("PRIMARY", Color(0.2, 0.6, 1.0))
	style.shadow_size = 8
	style.shadow_offset = Vector2(0, 4)
	style.shadow_color = Color(0, 0, 0, 0.3)
	return style


func create_training_card_selected() -> StyleBoxFlat:
	var style = create_training_card_normal()
	style.bg_color = _get_theme_color("PRIMARY", Color(0.2, 0.6, 1.0)).darkened(0.7)
	style.border_color = _get_theme_color("PRIMARY", Color(0.2, 0.6, 1.0))
	style.border_width_left = 3
	style.border_width_right = 3
	style.border_width_top = 3
	style.border_width_bottom = 3
	return style


## 탭 스타일
func create_tab_button_normal() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BG_MEDIUM", Color(0.2, 0.2, 0.25))
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_left = 0
	style.corner_radius_bottom_right = 0
	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 10
	style.content_margin_bottom = 10
	return style


func create_tab_button_selected() -> StyleBoxFlat:
	var style = create_tab_button_normal()
	style.bg_color = _get_theme_color("PRIMARY", Color(0.2, 0.6, 1.0))
	return style


func create_tab_panel() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BG_SURFACE", Color(0.15, 0.15, 0.2))
	style.corner_radius_top_left = 0
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.content_margin_left = 16
	style.content_margin_right = 16
	style.content_margin_top = 16
	style.content_margin_bottom = 16
	return style


## 세이브 슬롯 카드 (카이로 스타일)
func create_save_slot_empty() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BG_SURFACE", Color(0.15, 0.15, 0.2)).darkened(0.3)
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.border_width_left = 2
	style.border_width_right = 2
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.border_color = Color(1, 1, 1, 0.2)
	style.border_blend = true
	style.draw_center = true
	# 점선 효과를 위한 설정
	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 20
	style.content_margin_bottom = 20
	return style


func create_save_slot_filled() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BG_SURFACE", Color(0.15, 0.15, 0.2))
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_LARGE", 12)
	style.shadow_size = 6
	style.shadow_offset = Vector2(0, 3)
	style.shadow_color = Color(0, 0, 0, 0.3)
	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 20
	style.content_margin_bottom = 20
	return style


## 조건부 스타일 생성
func get_condition_panel(condition: int) -> StyleBoxFlat:
	var style = create_card_panel()
	var color = _get_theme_color("SUCCESS", Color(0.2, 0.8, 0.2))

	match condition:
		1:
			color = _get_theme_color("DANGER", Color(0.8, 0.2, 0.2))
		2:
			color = _get_theme_color("WARNING", Color(1.0, 0.6, 0.0))
		3:
			color = _get_theme_color("PASTEL_YELLOW", Color(1.0, 1.0, 0.6))
		4:
			color = _get_theme_color("PASTEL_GREEN", Color(0.6, 1.0, 0.6))
		5:
			color = _get_theme_color("SUCCESS", Color(0.2, 0.8, 0.2))

	style.border_width_left = 4
	style.border_color = color
	return style


## 버튼 스타일 생성 (색상 지정 가능)
func create_button_style(color: Color) -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = color
	style.corner_radius_top_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_top_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_left = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.corner_radius_bottom_right = _get_theme_constant("CORNER_RADIUS_MEDIUM", 8)
	style.content_margin_left = 16
	style.content_margin_right = 16
	style.content_margin_top = 8
	style.content_margin_bottom = 8
	return style


## 메인 배경 스타일
func create_main_background() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = _get_theme_color("BACKGROUND", Color(0.1, 0.1, 0.15))
	style.corner_radius_top_left = 0
	style.corner_radius_top_right = 0
	style.corner_radius_bottom_left = 0
	style.corner_radius_bottom_right = 0
	return style
