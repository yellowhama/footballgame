extends Node

# 접근성 관리 시스템 (Autoload로 사용)
# 고대비 모드, 큰 텍스트 모드, 색맹 친화적 UI 등 지원

signal accessibility_changed
signal high_contrast_changed(enabled: bool)
signal large_text_changed(enabled: bool)
signal colorblind_mode_changed(mode: ColorblindMode)

enum ColorblindMode { NORMAL, PROTANOPIA, DEUTERANOPIA, TRITANOPIA }  # 적색맹  # 녹색맹  # 청색맹

# 접근성 설정
var high_contrast_mode: bool = false
var large_text_mode: bool = false

# 설정 파일
var config: ConfigFile
var colorblind_mode: ColorblindMode = ColorblindMode.NORMAL
var font_scale_factor: float = 1.0
var contrast_scale_factor: float = 1.0

# 테마 리소스들
var normal_theme: Theme
var high_contrast_theme: Theme
var large_text_theme: Theme

# 색상 팔레트들
var normal_colors: Dictionary
var colorblind_safe_colors: Dictionary
var high_contrast_colors: Dictionary


func _ready():
	load_accessibility_settings()
	setup_color_palettes()
	check_system_accessibility()


func load_accessibility_settings():
	# 사용자 설정 파일에서 접근성 설정 로드
	var config = ConfigFile.new()
	var err = config.load("user://accessibility_settings.cfg")

	if err == OK:
		high_contrast_mode = config.get_value("accessibility", "high_contrast", false)
		large_text_mode = config.get_value("accessibility", "large_text", false)
		colorblind_mode = config.get_value("accessibility", "colorblind_mode", ColorblindMode.NORMAL)
		font_scale_factor = config.get_value("accessibility", "font_scale", 1.0)
		contrast_scale_factor = config.get_value("accessibility", "contrast_scale", 1.0)


func save_accessibility_settings():
	# 접근성 설정을 파일에 저장
	var config = ConfigFile.new()
	config.set_value("accessibility", "high_contrast", high_contrast_mode)
	config.set_value("accessibility", "large_text", large_text_mode)
	config.set_value("accessibility", "colorblind_mode", colorblind_mode)
	config.set_value("accessibility", "font_scale", font_scale_factor)
	config.set_value("accessibility", "contrast_scale", contrast_scale_factor)
	config.save("user://accessibility_settings.cfg")


func check_system_accessibility():
	# 시스템 접근성 설정 확인 (OS.is_feature 미지원)
	# 대신 사용자 설정을 확인
	if config and config.get_value("accessibility", "high_contrast", false):
		enable_high_contrast()

	if config and config.get_value("accessibility", "large_text", false):
		enable_large_text()


func setup_color_palettes():
	# 일반 색상 팔레트
	normal_colors = {
		"success": Color(0.18, 0.8, 0.44),  # 초록색
		"warning": Color(1.0, 0.84, 0.0),  # 노란색
		"danger": Color(0.86, 0.21, 0.27),  # 빨간색
		"info": Color(0.09, 0.63, 0.82),  # 파란색
		"neutral": Color(0.6, 0.6, 0.6),  # 회색
		"background": Color(0.1, 0.1, 0.15),  # 어두운 배경
		"surface": Color(0.16, 0.16, 0.22),  # 표면색
		"text": Color(0.9, 0.9, 0.9)  # 텍스트
	}

	# 색맹 친화적 색상 팔레트
	colorblind_safe_colors = {
		"success": Color(0.2, 0.6, 0.8),  # 파란색 (성공)
		"warning": Color(0.9, 0.8, 0.2),  # 노란색 (경고)
		"danger": Color(0.8, 0.3, 0.8),  # 보라색 (위험)
		"info": Color(0.4, 0.7, 0.9),  # 밝은 파란색 (정보)
		"neutral": Color(0.6, 0.6, 0.6),  # 회색 (중립)
		"background": Color(0.1, 0.1, 0.15),  # 어두운 배경
		"surface": Color(0.16, 0.16, 0.22),  # 표면색
		"text": Color(0.9, 0.9, 0.9)  # 텍스트
	}

	# 고대비 색상 팔레트
	high_contrast_colors = {
		"success": Color(0.0, 1.0, 0.0),  # 순녹색
		"warning": Color(1.0, 1.0, 0.0),  # 순노랑
		"danger": Color(1.0, 0.0, 0.0),  # 순빨강
		"info": Color(0.0, 0.0, 1.0),  # 순파랑
		"neutral": Color(0.8, 0.8, 0.8),  # 밝은 회색
		"background": Color(0.0, 0.0, 0.0),  # 순검정
		"surface": Color(0.1, 0.1, 0.1),  # 어두운 회색
		"text": Color(1.0, 1.0, 1.0)  # 순흰색
	}


# 고대비 모드 토글
func toggle_high_contrast():
	set_high_contrast_mode(not high_contrast_mode)


func set_high_contrast_mode(enabled: bool):
	if high_contrast_mode == enabled:
		return

	high_contrast_mode = enabled
	emit_signal("high_contrast_changed", enabled)
	emit_signal("accessibility_changed")
	apply_accessibility_changes()
	save_accessibility_settings()


func enable_high_contrast():
	set_high_contrast_mode(true)


func disable_high_contrast():
	set_high_contrast_mode(false)


# 큰 텍스트 모드 토글
func toggle_large_text():
	set_large_text_mode(not large_text_mode)


func set_large_text_mode(enabled: bool):
	if large_text_mode == enabled:
		return

	large_text_mode = enabled
	font_scale_factor = 1.5 if enabled else 1.0
	emit_signal("large_text_changed", enabled)
	emit_signal("accessibility_changed")
	apply_accessibility_changes()
	save_accessibility_settings()


func enable_large_text():
	set_large_text_mode(true)


func disable_large_text():
	set_large_text_mode(false)


# 색맹 모드 설정
func set_colorblind_mode(mode: ColorblindMode):
	if colorblind_mode == mode:
		return

	colorblind_mode = mode
	emit_signal("colorblind_mode_changed", mode)
	emit_signal("accessibility_changed")
	apply_accessibility_changes()
	save_accessibility_settings()


# 폰트 크기 스케일 설정
func set_font_scale(scale: float):
	font_scale_factor = clamp(scale, 0.8, 2.0)
	if large_text_mode and font_scale_factor < 1.5:
		font_scale_factor = 1.5
	apply_accessibility_changes()
	save_accessibility_settings()


# 대비 스케일 설정
func set_contrast_scale(scale: float):
	contrast_scale_factor = clamp(scale, 1.0, 2.0)
	apply_accessibility_changes()
	save_accessibility_settings()


# 접근성 변경사항 적용
func apply_accessibility_changes():
	var all_scenes = get_tree().get_nodes_in_group("ui_scenes")
	for scene in all_scenes:
		apply_to_scene(scene)


func apply_to_scene(scene: Node):
	# 씬에 접근성 설정 적용
	apply_font_scaling_to_scene(scene)
	apply_color_scheme_to_scene(scene)
	apply_contrast_to_scene(scene)


func apply_font_scaling_to_scene(scene: Node):
	var text_nodes = []
	find_text_nodes_recursive(scene, text_nodes)

	for node in text_nodes:
		apply_font_scaling_to_node(node)


func apply_font_scaling_to_node(node: Node):
	if not node.has_method("add_theme_font_size_override"):
		return

	var base_size = 16  # 기본 폰트 크기

	# 노드 타입별 기본 크기 조정
	if node is Button:
		base_size = 16
	elif node is Label:
		base_size = 14
		# 제목인지 확인
		if "title" in node.name.to_lower() or "header" in node.name.to_lower():
			base_size = 24

	var scaled_size = int(base_size * font_scale_factor)
	node.add_theme_font_size_override("font_size", scaled_size)


func apply_color_scheme_to_scene(scene: Node):
	var current_palette = get_current_color_palette()

	# 모든 UI 요소에 색상 적용
	var ui_nodes = []
	find_ui_nodes_recursive(scene, ui_nodes)

	for node in ui_nodes:
		apply_colors_to_node(node, current_palette)


func apply_colors_to_node(node: Node, palette: Dictionary):
	# 노드 타입별 색상 적용
	if node is Button:
		if high_contrast_mode:
			node.add_theme_color_override("font_color", palette.text)
			node.add_theme_color_override("font_hover_color", palette.warning)
	elif node is Label:
		node.add_theme_color_override("font_color", palette.text)
	elif node is Panel or node is PanelContainer:
		# 배경색 적용 (필요시 커스텀 스타일박스 생성)
		pass


func apply_contrast_to_scene(scene: Node):
	if high_contrast_mode:
		scene.modulate = Color(contrast_scale_factor, contrast_scale_factor, contrast_scale_factor)
	else:
		scene.modulate = Color(1.0, 1.0, 1.0)


func get_current_color_palette() -> Dictionary:
	if high_contrast_mode:
		return high_contrast_colors
	elif colorblind_mode != ColorblindMode.NORMAL:
		return colorblind_safe_colors
	else:
		return normal_colors


func find_text_nodes_recursive(node: Node, text_nodes: Array):
	if node is Label or node is Button or node is LineEdit or node is RichTextLabel:
		text_nodes.append(node)

	for child in node.get_children():
		find_text_nodes_recursive(child, text_nodes)


func find_ui_nodes_recursive(node: Node, ui_nodes: Array):
	if node is Control:
		ui_nodes.append(node)

	for child in node.get_children():
		find_ui_nodes_recursive(child, ui_nodes)


# 색맹별 색상 변환
func convert_color_for_colorblind(color: Color) -> Color:
	match colorblind_mode:
		ColorblindMode.PROTANOPIA:
			return convert_protanopia(color)
		ColorblindMode.DEUTERANOPIA:
			return convert_deuteranopia(color)
		ColorblindMode.TRITANOPIA:
			return convert_tritanopia(color)
		_:
			return color


func convert_protanopia(color: Color) -> Color:
	# 적색맹용 색상 변환 (간단한 근사)
	var r = 0.567 * color.r + 0.433 * color.g
	var g = 0.558 * color.r + 0.442 * color.g
	var b = 0.242 * color.g + 0.758 * color.b
	return Color(r, g, b, color.a)


func convert_deuteranopia(color: Color) -> Color:
	# 녹색맹용 색상 변환
	var r = 0.625 * color.r + 0.375 * color.g
	var g = 0.7 * color.r + 0.3 * color.g
	var b = 0.3 * color.g + 0.7 * color.b
	return Color(r, g, b, color.a)


func convert_tritanopia(color: Color) -> Color:
	# 청색맹용 색상 변환
	var r = 0.95 * color.r + 0.05 * color.g
	var g = 0.433 * color.g + 0.567 * color.b
	var b = 0.475 * color.g + 0.525 * color.b
	return Color(r, g, b, color.a)


# 접근성 정보 제공
func get_accessibility_info() -> Dictionary:
	return {
		"high_contrast": high_contrast_mode,
		"large_text": large_text_mode,
		"colorblind_mode": colorblind_mode,
		"font_scale": font_scale_factor,
		"contrast_scale": contrast_scale_factor
	}


# 접근성 설정 UI를 위한 헬퍼 메서드들
func get_colorblind_mode_name(mode: ColorblindMode) -> String:
	match mode:
		ColorblindMode.NORMAL:
			return "Normal"
		ColorblindMode.PROTANOPIA:
			return "Protanopia (Red-blind)"
		ColorblindMode.DEUTERANOPIA:
			return "Deuteranopia (Green-blind)"
		ColorblindMode.TRITANOPIA:
			return "Tritanopia (Blue-blind)"
		_:
			return "Unknown"
