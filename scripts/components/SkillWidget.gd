extends Control
class_name SkillWidget
# SkillWidget - 개별 스킬 표시용 재사용 컴포넌트
# Enhanced42StatsScreen에서 42개 스킬 동적 생성용

signal skill_clicked(skill_name: String, value: float)
signal skill_hovered(skill_name: String, value: float)

@export var skill_name: String = ""
@export var skill_display_name: String = ""
@export var skill_value: float = 50.0
@export var show_bar: bool = true
@export var show_delta: bool = false
@export var delta_value: float = 0.0

# UI 컴포넌트들
var skill_label: Label
var value_label: Label
var skill_bar: ProgressBar
var delta_label: Label
var background_panel: Panel

# 테마 참조 (autoload에서 가져옴)
# CustomStyles is an autoload, access via global


func _ready():
	_create_ui()
	_apply_styles()
	_connect_signals()
	_update_display()


func _create_ui():
	# 메인 배경 패널
	background_panel = Panel.new()
	add_child(background_panel)
	background_panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)

	# VBox 컨테이너
	var vbox = VBoxContainer.new()
	add_child(vbox)
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 8)

	# 상단 HBox (이름 + 값)
	var top_hbox = HBoxContainer.new()
	vbox.add_child(top_hbox)

	# 스킬 이름 라벨
	skill_label = Label.new()
	skill_label.text = skill_display_name if skill_display_name != "" else skill_name
	skill_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	skill_label.add_theme_font_size_override("font_size", 14)
	top_hbox.add_child(skill_label)

	# 값 라벨
	value_label = Label.new()
	value_label.text = str(int(skill_value))
	value_label.size_flags_horizontal = Control.SIZE_SHRINK_END
	value_label.add_theme_font_size_override("font_size", 16)
	top_hbox.add_child(value_label)

	# 프로그레스 바 (선택적)
	if show_bar:
		skill_bar = ProgressBar.new()
		skill_bar.min_value = 0
		skill_bar.max_value = 100
		skill_bar.value = skill_value
		skill_bar.show_percentage = false
		skill_bar.custom_minimum_size = Vector2(0, 12)
		vbox.add_child(skill_bar)

	# 델타 라벨 (변화량 표시, 선택적)
	if show_delta and delta_value != 0.0:
		delta_label = Label.new()
		delta_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		delta_label.add_theme_font_size_override("font_size", 12)
		vbox.add_child(delta_label)


func _apply_styles():
	# 배경 패널 스타일
	if background_panel:
		var style = StyleBoxFlat.new()
		style.bg_color = ThemeManager.BG_SURFACE
		style.corner_radius_top_left = ThemeManager.CORNER_RADIUS_MEDIUM
		style.corner_radius_top_right = ThemeManager.CORNER_RADIUS_MEDIUM
		style.corner_radius_bottom_left = ThemeManager.CORNER_RADIUS_MEDIUM
		style.corner_radius_bottom_right = ThemeManager.CORNER_RADIUS_MEDIUM
		style.content_margin_left = 12
		style.content_margin_right = 12
		style.content_margin_top = 10
		style.content_margin_bottom = 10
		background_panel.add_theme_stylebox_override("panel", style)

	# 스킬 이름 라벨 스타일
	if skill_label:
		skill_label.modulate = ThemeManager.TEXT_PRIMARY

	# 값 라벨 스타일 (스킬 값에 따른 색상)
	if value_label:
		value_label.modulate = ThemeManager.get_stat_color(skill_value, 100.0)

	# 프로그레스 바 스타일
	if skill_bar:
		var fill_style = StyleBoxFlat.new()
		fill_style.bg_color = ThemeManager.get_stat_color(skill_value, 100.0)
		fill_style.corner_radius_top_left = 6
		fill_style.corner_radius_top_right = 6
		fill_style.corner_radius_bottom_left = 6
		fill_style.corner_radius_bottom_right = 6
		skill_bar.add_theme_stylebox_override("fill", fill_style)

		var bg_style = StyleBoxFlat.new()
		bg_style.bg_color = ThemeManager.BG_SURFACE_VARIANT
		bg_style.corner_radius_top_left = 6
		bg_style.corner_radius_top_right = 6
		bg_style.corner_radius_bottom_left = 6
		bg_style.corner_radius_bottom_right = 6
		skill_bar.add_theme_stylebox_override("background", bg_style)


func _connect_signals():
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)
	gui_input.connect(_on_gui_input)


func _on_mouse_entered():
	# 호버 효과
	if background_panel:
		var style = background_panel.get_theme_stylebox("panel").duplicate() as StyleBoxFlat
		style.bg_color = ThemeManager.BG_SURFACE_VARIANT
		background_panel.add_theme_stylebox_override("panel", style)

	skill_hovered.emit(skill_name, skill_value)


func _on_mouse_exited():
	# 호버 해제
	_apply_styles()


func _on_gui_input(event: InputEvent):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			skill_clicked.emit(skill_name, skill_value)


func set_skill_data(name: String, display_name: String, value: float):
	"""스킬 데이터 설정"""
	skill_name = name
	skill_display_name = display_name
	skill_value = value
	_update_display()


func set_skill_value(new_value: float, animate: bool = false):
	"""스킬 값 업데이트 (애니메이션 선택적)"""
	var old_value = skill_value
	skill_value = new_value

	if animate and is_inside_tree():
		_animate_value_change(old_value, new_value)
	else:
		_update_display()


func set_delta_value(delta: float):
	"""변화량 설정 및 표시"""
	delta_value = delta
	show_delta = (delta != 0.0)
	_update_delta_display()


func _update_display():
	"""UI 요소들 업데이트"""
	if skill_label:
		skill_label.text = skill_display_name if skill_display_name != "" else skill_name

	if value_label:
		value_label.text = str(int(skill_value))
		value_label.modulate = ThemeManager.get_stat_color(skill_value, 100.0)

	if skill_bar:
		skill_bar.value = skill_value
		# 바 색상 업데이트
		var fill_style = StyleBoxFlat.new()
		fill_style.bg_color = ThemeManager.get_stat_color(skill_value, 100.0)
		fill_style.corner_radius_top_left = 6
		fill_style.corner_radius_top_right = 6
		fill_style.corner_radius_bottom_left = 6
		fill_style.corner_radius_bottom_right = 6
		skill_bar.add_theme_stylebox_override("fill", fill_style)


func _update_delta_display():
	"""델타 표시 업데이트"""
	if not show_delta or not delta_label:
		return

	if delta_value > 0:
		delta_label.text = "+%.1f" % delta_value
		delta_label.modulate = ThemeManager.SUCCESS
	elif delta_value < 0:
		delta_label.text = "%.1f" % delta_value
		delta_label.modulate = ThemeManager.DANGER
	else:
		delta_label.text = "±0.0"
		delta_label.modulate = ThemeManager.TEXT_SECONDARY


func _animate_value_change(old_val: float, new_val: float):
	"""값 변화 애니메이션"""
	var tween = create_tween()
	tween.set_parallel(true)

	# 값 라벨 애니메이션
	tween.tween_method(
		func(val: float):
			if value_label:
				value_label.text = str(int(val))
				value_label.modulate = ThemeManager.get_stat_color(val, 100.0),
		old_val,
		new_val,
		0.3
	)

	# 프로그레스 바 애니메이션
	if skill_bar:
		tween.tween_property(skill_bar, "value", new_val, 0.3)

	# 스케일 효과 (변화 강조)
	if abs(new_val - old_val) > 1.0:
		tween.tween_property(self, "scale", Vector2(1.05, 1.05), 0.1)
		tween.tween_property(self, "scale", Vector2(1.0, 1.0), 0.2).set_delay(0.1)


# 카테고리별 색상 테마 설정
func set_category_theme(category: String):
	"""카테고리별 테마 색상 적용"""
	var accent_color: Color

	match category:
		"Technical":
			accent_color = ThemeManager.PASTEL_BLUE
		"Mental":
			accent_color = ThemeManager.PASTEL_PURPLE
		"Physical":
			accent_color = ThemeManager.PASTEL_GREEN
		"Goalkeeper":
			accent_color = ThemeManager.PASTEL_YELLOW
		_:
			accent_color = ThemeManager.ACCENT

	# 배경에 카테고리 색상 힌트 추가
	if background_panel:
		var style = background_panel.get_theme_stylebox("panel").duplicate() as StyleBoxFlat
		style.border_color = accent_color
		style.border_width_left = 2
		style.border_width_top = 2
		style.border_width_right = 2
		style.border_width_bottom = 2
		background_panel.add_theme_stylebox_override("panel", style)


# 스킬 등급 계산 (S/A/B/C/D)
func get_skill_grade() -> String:
	"""스킬 값을 등급으로 변환"""
	if skill_value >= 90:
		return "S"
	elif skill_value >= 80:
		return "A"
	elif skill_value >= 70:
		return "B"
	elif skill_value >= 60:
		return "C"
	else:
		return "D"


# 스킬 설명 툴팁 (future feature)
func get_skill_description() -> String:
	"""스킬 상세 설명 반환"""
	# TODO: 스킬별 상세 설명 데이터 추가
	return "%s: %.1f/100" % [skill_display_name, skill_value]
