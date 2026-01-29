extends Node
# 씬에 ColorSystem을 일괄 적용하는 유틸리티 (수정 버전)
# Static 호출 문제를 해결하여 실제로 작동하도록 개선


static func apply_color_system_to_scene(node: Node):
	"""씬에 ColorSystem 색상을 일괄 적용 (Static 문제 해결)"""
	_apply_colors_recursive(node)


static func _apply_colors_recursive(node: Node):
	"""재귀적으로 모든 노드에 색상 적용"""

	# ColorSystem의 상수를 직접 사용 (노드 참조 대신)
	if node is ColorRect:
		_apply_colorrect_colors(node)
	elif node is Label:
		_apply_label_colors(node)
	elif node is ProgressBar:
		_apply_progressbar_colors(node)
	elif node is Button:
		_apply_button_colors(node)
	elif node is Panel:
		_apply_panel_colors(node)

	# 자식 노드들도 재귀적으로 처리
	for child in node.get_children():
		_apply_colors_recursive(child)


static func _apply_colorrect_colors(rect: ColorRect):
	"""ColorRect에 배경색 적용"""
	var node_name = rect.name.to_lower()

	if "background" in node_name:
		# 카이로소프트 스타일 밝은 배경
		rect.color = Color(0.95, 0.95, 0.98, 1.0)
	elif "card" in node_name:
		# 카드 배경 (파스텔)
		rect.color = Color(1.0, 1.0, 1.0, 0.95)
	elif "header" in node_name:
		# 헤더 (조금 진한 파스텔)
		rect.color = Color("#a8e6cf")  # KAIRO_GREEN
	elif "panel" in node_name:
		rect.color = Color(1.0, 1.0, 1.0, 0.9)


static func _apply_label_colors(label: Label):
	"""Label에 텍스트 색상 적용"""
	var node_name = label.name.to_lower()

	if "title" in node_name or "header" in node_name:
		# 제목 텍스트 (진한 색)
		label.add_theme_color_override("font_color", Color(0.1, 0.1, 0.2, 1.0))
		label.add_theme_font_size_override("font_size", 24)
	elif "subtitle" in node_name:
		# 부제목
		label.add_theme_color_override("font_color", Color(0.3, 0.3, 0.4, 1.0))
		label.add_theme_font_size_override("font_size", 18)
	elif "caption" in node_name or "info" in node_name:
		# 캡션/정보
		label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.6, 1.0))
		label.add_theme_font_size_override("font_size", 14)
	else:
		# 일반 텍스트
		label.add_theme_color_override("font_color", Color(0.2, 0.2, 0.3, 1.0))
		# 최소 16px (모바일 기준)
		if label.get_theme_font_size("font_size") < 16:
			label.add_theme_font_size_override("font_size", 16)


static func _apply_progressbar_colors(progress: ProgressBar):
	"""ProgressBar에 색상 적용"""
	var style_bg = StyleBoxFlat.new()
	var style_fill = StyleBoxFlat.new()

	# 배경 스타일
	style_bg.bg_color = Color(0.9, 0.9, 0.95, 1.0)
	style_bg.corner_radius_top_left = 4
	style_bg.corner_radius_top_right = 4
	style_bg.corner_radius_bottom_left = 4
	style_bg.corner_radius_bottom_right = 4

	# 채우기 스타일 (카이로소프트 파스텔)
	style_fill.bg_color = Color("#7fcdff")  # KAIRO_BLUE
	style_fill.corner_radius_top_left = 4
	style_fill.corner_radius_top_right = 4
	style_fill.corner_radius_bottom_left = 4
	style_fill.corner_radius_bottom_right = 4

	progress.add_theme_stylebox_override("background", style_bg)
	progress.add_theme_stylebox_override("fill", style_fill)

	# 모바일 최적화: 최소 높이 44px
	progress.custom_minimum_size.y = max(progress.custom_minimum_size.y, 44)


static func _apply_button_colors(button: Button):
	"""Button에 색상 및 스타일 적용"""
	# 텍스트 색상
	button.add_theme_color_override("font_color", Color(0.2, 0.2, 0.3, 1.0))
	button.add_theme_color_override("font_hover_color", Color(0.1, 0.1, 0.2, 1.0))
	button.add_theme_color_override("font_pressed_color", Color(0.0, 0.0, 0.1, 1.0))
	button.add_theme_color_override("font_disabled_color", Color(0.6, 0.6, 0.6, 0.5))

	# 폰트 크기 (최소 16px)
	if button.get_theme_font_size("font_size") < 16:
		button.add_theme_font_size_override("font_size", 16)

	# 버튼 스타일
	var style_normal = StyleBoxFlat.new()
	var style_hover = StyleBoxFlat.new()
	var style_pressed = StyleBoxFlat.new()

	# 일반 상태 (카이로소프트 파스텔)
	style_normal.bg_color = Color("#ffd3b6")  # KAIRO_YELLOW
	style_normal.border_color = Color("#ffb88c")
	style_normal.border_width_top = 2
	style_normal.border_width_bottom = 2
	style_normal.border_width_left = 2
	style_normal.border_width_right = 2
	style_normal.corner_radius_top_left = 8
	style_normal.corner_radius_top_right = 8
	style_normal.corner_radius_bottom_left = 8
	style_normal.corner_radius_bottom_right = 8
	style_normal.content_margin_top = 12
	style_normal.content_margin_bottom = 12
	style_normal.content_margin_left = 16
	style_normal.content_margin_right = 16

	# 호버 상태
	style_hover.bg_color = Color("#ffaaa5")  # KAIRO_PINK
	style_hover.border_color = Color("#ff8a85")
	style_hover.border_width_top = 2
	style_hover.border_width_bottom = 2
	style_hover.border_width_left = 2
	style_hover.border_width_right = 2
	style_hover.corner_radius_top_left = 8
	style_hover.corner_radius_top_right = 8
	style_hover.corner_radius_bottom_left = 8
	style_hover.corner_radius_bottom_right = 8
	style_hover.content_margin_top = 12
	style_hover.content_margin_bottom = 12
	style_hover.content_margin_left = 16
	style_hover.content_margin_right = 16

	# 눌림 상태
	style_pressed.bg_color = Color("#a8e6cf")  # KAIRO_GREEN
	style_pressed.border_color = Color("#88c6af")
	style_pressed.border_width_top = 2
	style_pressed.border_width_bottom = 2
	style_pressed.border_width_left = 2
	style_pressed.border_width_right = 2
	style_pressed.corner_radius_top_left = 8
	style_pressed.corner_radius_top_right = 8
	style_pressed.corner_radius_bottom_left = 8
	style_pressed.corner_radius_bottom_right = 8
	style_pressed.content_margin_top = 12
	style_pressed.content_margin_bottom = 12
	style_pressed.content_margin_left = 16
	style_pressed.content_margin_right = 16

	button.add_theme_stylebox_override("normal", style_normal)
	button.add_theme_stylebox_override("hover", style_hover)
	button.add_theme_stylebox_override("pressed", style_pressed)

	# 모바일 최적화: 최소 크기 44x44px
	button.custom_minimum_size.x = max(button.custom_minimum_size.x, 44)
	button.custom_minimum_size.y = max(button.custom_minimum_size.y, 44)


static func _apply_panel_colors(panel: Panel):
	"""Panel에 스타일 적용"""
	var style = StyleBoxFlat.new()

	# 카이로소프트 스타일 밝은 패널
	style.bg_color = Color(1.0, 1.0, 1.0, 0.95)
	style.border_color = Color(0.9, 0.9, 0.95, 1.0)
	style.border_width_top = 1
	style.border_width_bottom = 1
	style.border_width_left = 1
	style.border_width_right = 1
	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	style.content_margin_top = 8
	style.content_margin_bottom = 8
	style.content_margin_left = 8
	style.content_margin_right = 8

	panel.add_theme_stylebox_override("panel", style)


# 컨디션별 색상 가져오기 (Static)
static func get_condition_color(condition: int) -> Color:
	"""컨디션 레벨에 따른 색상 반환"""
	match condition:
		5:
			return Color("#2E7D32")  # 절호조
		4:
			return Color("#689F38")  # 호조
		3:
			return Color("#F57F17")  # 보통
		2:
			return Color("#E64A19")  # 부진
		1:
			return Color("#C62828")  # 절부진
		_:
			return Color("#F57F17")  # 기본 (보통)


# 포지션별 색상 가져오기 (Static)
static func get_position_color(position: String) -> Color:
	"""포지션에 따른 색상 반환"""
	match position:
		"ST", "CF", "LW", "RW":
			return Color("#ff6b6b")  # 공격수 (빨강)
		"CAM", "LM", "RM", "AM":
			return Color("#ffa500")  # 공격형 MF (주황)
		"CM", "CDM", "DM":
			return Color("#4ecdc4")  # 중앙 MF (청록)
		"CB", "LB", "RB", "SW":
			return Color("#45b7d1")  # 수비수 (파랑)
		"GK":
			return Color("#9c27b0")  # 골키퍼 (보라)
		_:
			return Color("#666666")  # 기본 (회색)
