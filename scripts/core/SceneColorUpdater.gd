extends Node
class_name SceneColorUpdater
## 씬에 ThemeManager 색상을 일괄 적용하는 유틸리티
## Phase 1: ThemeManager 통합 (2025-12-03)


static func apply_color_system_to_scene(node: Node):
	# 씬에 ThemeManager 색상을 일괄 적용
	_apply_colors_recursive(node)


static func _apply_colors_recursive(node: Node):
	# 재귀적으로 모든 노드에 색상 적용
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
	# ColorRect에 배경색 적용
	var name_lower = rect.name.to_lower()
	if "background" in name_lower:
		rect.color = ThemeManager.BG_PRIMARY
	elif "card" in name_lower:
		rect.color = ThemeManager.BG_SECONDARY
	elif "header" in name_lower:
		rect.color = ThemeManager.BG_TERTIARY


static func _apply_label_colors(label: Label):
	# Label에 텍스트 색상 및 타이포그래피 적용
	var name_lower = label.name.to_lower()

	# 색상 적용
	if "title" in name_lower or "header" in name_lower:
		label.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
	elif "subtitle" in name_lower or "secondary" in name_lower:
		label.add_theme_color_override("font_color", ThemeManager.TEXT_SECONDARY)
	elif "disabled" in name_lower:
		label.add_theme_color_override("font_color", ThemeManager.TEXT_DISABLED)
	else:
		label.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)

	# 타이포그래피 적용 (스펙 3.1 폰트 스케일)
	if "h1" in name_lower or "screentitle" in name_lower:
		label.add_theme_font_size_override("font_size", ThemeManager.FONT_H1)
	elif "h2" in name_lower or "sectiontitle" in name_lower:
		label.add_theme_font_size_override("font_size", ThemeManager.FONT_H2)
	elif "h3" in name_lower or "cardtitle" in name_lower:
		label.add_theme_font_size_override("font_size", ThemeManager.FONT_H3)
	elif "body" in name_lower:
		label.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)
	elif "caption" in name_lower or "description" in name_lower:
		label.add_theme_font_size_override("font_size", ThemeManager.FONT_CAPTION)
	elif "micro" in name_lower or "label" in name_lower:
		label.add_theme_font_size_override("font_size", ThemeManager.FONT_MICRO)


static func _apply_progressbar_colors(bar: ProgressBar):
	# ProgressBar에 색상 적용
	var fill_style = StyleBoxFlat.new()
	fill_style.bg_color = ThemeManager.SUCCESS
	fill_style.corner_radius_top_left = 4
	fill_style.corner_radius_top_right = 4
	fill_style.corner_radius_bottom_left = 4
	fill_style.corner_radius_bottom_right = 4
	bar.add_theme_stylebox_override("fill", fill_style)

	var bg_style = StyleBoxFlat.new()
	bg_style.bg_color = ThemeManager.BG_TERTIARY
	bg_style.corner_radius_top_left = 4
	bg_style.corner_radius_top_right = 4
	bg_style.corner_radius_bottom_left = 4
	bg_style.corner_radius_bottom_right = 4
	bar.add_theme_stylebox_override("background", bg_style)


static func _apply_button_colors(button: Button):
	# Button에 색상 적용 (스펙 기반 - 버튼 이름으로 variant 결정)
	var name_lower = button.name.to_lower()
	var variant = "secondary"  # 기본값

	# 버튼 이름에서 variant 추론
	if "primary" in name_lower or "confirm" in name_lower or "ok" in name_lower or "submit" in name_lower:
		variant = "primary"
	elif "success" in name_lower or "save" in name_lower or "complete" in name_lower:
		variant = "success"
	elif "danger" in name_lower or "delete" in name_lower or "remove" in name_lower or "cancel" in name_lower:
		variant = "danger"
	elif "warning" in name_lower or "warn" in name_lower:
		variant = "warning"

	# ThemeManager의 버튼 스타일 적용
	var style = ThemeManager.get_button_style(variant)
	ThemeManager.apply_button_style(button, style)


static func _apply_panel_colors(panel: Panel):
	# Panel에 색상 적용 (이름 기반 스타일 결정)
	var name_lower = panel.name.to_lower()

	# 네비게이션 바 (하단)
	if "nav" in name_lower or "bottomnav" in name_lower or "toolbar" in name_lower:
		ThemeManager.apply_navbar_style(panel)
		return

	# 헤더 바 (상단)
	if "header" in name_lower or "topbar" in name_lower:
		ThemeManager.apply_header_style(panel)
		return

	# 카드 스타일
	if "card" in name_lower:
		var style = ThemeManager.create_card_style()
		panel.add_theme_stylebox_override("panel", style)
		return

	# 기본 패널 스타일
	var style = StyleBoxFlat.new()
	style.bg_color = ThemeManager.BG_PRIMARY
	style.set_corner_radius_all(ThemeManager.CORNER_RADIUS_MEDIUM)
	panel.add_theme_stylebox_override("panel", style)
