extends Control
## TrainingResultPopup - 훈련 결과 표시 팝업
## 디자인: 04_ui_design_system.md 기반
##
## P0: 대성공 연출 시스템 (Kairosoft/Blue Lock PWC inspired)
## - 일반: "+2 Speed" (작은 텍스트)
## - 성공: "+4 Speed!" (중간 + 색상 강조)
## - 대성공: "★ +6 Speed ★" (전체화면 플래시 + 파티클)
##
## 작성일: 2025-11-26
## 업데이트: 2025-12-03 (P0 대성공 연출 추가)

signal closed

# ============================================
# 훈련 결과 등급 (P0: Kairosoft style)
# ============================================
enum TrainingGrade { NORMAL, SUCCESS, GREAT_SUCCESS }

# ============================================
# UI 노드 참조
# ============================================

@onready var panel: PanelContainer = $CenterContainer/Panel
@onready var title_label: Label = $CenterContainer/Panel/VBox/TitleLabel
@onready var training_name_label: Label = $CenterContainer/Panel/VBox/TrainingNameLabel
@onready var changes_container: VBoxContainer = $CenterContainer/Panel/VBox/ChangesContainer
@onready var total_label: Label = $CenterContainer/Panel/VBox/TotalLabel
@onready var close_button: Button = $CenterContainer/Panel/VBox/CloseButton
@onready var background_dim: ColorRect = $BackgroundDim

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_POSITIVE = Color("#238636")  # accent-primary (성공, 성장)
const COLOR_NEGATIVE = Color("#DA3633")  # accent-danger (감소)
const COLOR_NEUTRAL = Color("#8B949E")  # text-secondary
const COLOR_TEXT = Color("#E6EDF3")  # text-primary
const COLOR_BG = Color("#161B22")  # bg-secondary

## P0: Grade-specific colors (Kairosoft/Blue Lock inspired)
const COLOR_NORMAL = Color("#8B949E")  # Gray - normal result
const COLOR_SUCCESS = Color("#4CAF50")  # Green - good result
const COLOR_GREAT_SUCCESS = Color("#FFD700")  # Gold - great success!
const COLOR_GREAT_SUCCESS_GLOW = Color("#FF8C00")  # Orange glow

# ============================================
# 상태
# ============================================

var _result_data: Dictionary = {}
var _animation_tweens: Array[Tween] = []
var _current_grade: TrainingGrade = TrainingGrade.NORMAL
var _particle_nodes: Array[Node] = []

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	visible = false

	if close_button:
		close_button.pressed.connect(_on_close_pressed)

	if background_dim:
		background_dim.gui_input.connect(_on_background_clicked)

	# 초기 투명 상태
	modulate.a = 0


func _input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		_on_close_pressed()
		get_viewport().set_input_as_handled()


# ============================================
# 공개 API
# ============================================


## 훈련 결과 표시
## @param result: Dictionary from TrainingManager.execute_training()
##   - success: bool
##   - training_id: String
##   - training_name: String
##   - changes: Dictionary {attribute_name: delta_value}
##   - condition_cost: float
##   - message: String (optional)
func show_result(result: Dictionary) -> void:
	_result_data = result

	# UI 업데이트
	_update_title(result)
	_update_training_name(result)
	_clear_changes()
	_populate_changes(result.get("changes", {}))
	_update_total(result)

	# 표시 및 애니메이션
	visible = true
	_animate_show()


## 팝업 닫기
func hide_popup() -> void:
	_animate_hide()


# ============================================
# UI 업데이트
# ============================================


func _update_title(result: Dictionary) -> void:
	if not title_label:
		return

	if not result.get("success", false):
		title_label.text = "훈련 실패"
		title_label.add_theme_color_override("font_color", COLOR_NEGATIVE)
		_current_grade = TrainingGrade.NORMAL
		return

	# P0: Determine training grade based on result
	_current_grade = _determine_grade(result)

	match _current_grade:
		TrainingGrade.GREAT_SUCCESS:
			title_label.text = "★ 대성공! ★"
			title_label.add_theme_color_override("font_color", COLOR_GREAT_SUCCESS)
			title_label.add_theme_font_size_override("font_size", 32)
		TrainingGrade.SUCCESS:
			title_label.text = "성공!"
			title_label.add_theme_color_override("font_color", COLOR_SUCCESS)
			title_label.add_theme_font_size_override("font_size", 26)
		_:
			title_label.text = "훈련 완료"
			title_label.add_theme_color_override("font_color", COLOR_POSITIVE)
			title_label.add_theme_font_size_override("font_size", 22)


## P0: Determine training grade from result data
func _determine_grade(result: Dictionary) -> TrainingGrade:
	# Check explicit grade field first
	var grade_str := str(result.get("grade", "")).to_lower()
	if grade_str == "great_success" or grade_str == "super":
		return TrainingGrade.GREAT_SUCCESS
	if grade_str == "success" or grade_str == "good":
		return TrainingGrade.SUCCESS

	# Fallback: calculate from total stat changes
	var changes: Dictionary = result.get("changes", {})
	var total_gain := 0
	for attr in changes:
		var delta: int = int(changes[attr])
		if delta > 0:
			total_gain += delta

	# Thresholds: Great Success >= 6, Success >= 3
	if total_gain >= 6:
		return TrainingGrade.GREAT_SUCCESS
	elif total_gain >= 3:
		return TrainingGrade.SUCCESS
	else:
		return TrainingGrade.NORMAL


func _update_training_name(result: Dictionary) -> void:
	if not training_name_label:
		return

	var name = result.get("training_name", result.get("training_id", "훈련"))
	training_name_label.text = name


func _clear_changes() -> void:
	if not changes_container:
		return

	for child in changes_container.get_children():
		child.queue_free()


func _populate_changes(changes: Dictionary) -> void:
	if not changes_container:
		return

	if changes.is_empty():
		var no_change_label = Label.new()
		no_change_label.text = "변화 없음"
		no_change_label.add_theme_color_override("font_color", COLOR_NEUTRAL)
		no_change_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		changes_container.add_child(no_change_label)
		return

	# 변화량 순으로 정렬 (큰 것부터)
	var sorted_changes = []
	for attr in changes:
		sorted_changes.append({"name": attr, "value": changes[attr]})
	sorted_changes.sort_custom(func(a, b): return abs(a.value) > abs(b.value))

	# 각 변화 항목 생성
	for i in range(sorted_changes.size()):
		var change = sorted_changes[i]
		var row = _create_change_row(change.name, change.value, i)
		changes_container.add_child(row)


func _create_change_row(attr_name: String, delta: int, index: int) -> HBoxContainer:
	var row = HBoxContainer.new()
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 속성 이름
	var name_label = Label.new()
	name_label.text = _translate_attribute(attr_name)
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_label.add_theme_color_override("font_color", COLOR_TEXT)
	name_label.add_theme_font_size_override("font_size", 18)
	row.add_child(name_label)

	# 변화량
	var value_label = Label.new()
	value_label.name = "ValueLabel"
	value_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	value_label.add_theme_font_size_override("font_size", 20)

	if delta > 0:
		value_label.text = "+%d" % delta
		value_label.add_theme_color_override("font_color", COLOR_POSITIVE)
	elif delta < 0:
		value_label.text = "%d" % delta
		value_label.add_theme_color_override("font_color", COLOR_NEGATIVE)
	else:
		value_label.text = "0"
		value_label.add_theme_color_override("font_color", COLOR_NEUTRAL)

	row.add_child(value_label)

	# 초기 투명 (애니메이션용)
	row.modulate.a = 0

	# 순차 페이드인 애니메이션
	var tween = create_tween()
	tween.tween_property(row, "modulate:a", 1.0, 0.2).set_delay(0.1 + index * 0.08)
	_animation_tweens.append(tween)

	return row


func _update_total(result: Dictionary) -> void:
	if not total_label:
		return

	var changes = result.get("changes", {})
	var total_positive = 0
	var total_negative = 0

	for attr in changes:
		var delta = changes[attr]
		if delta > 0:
			total_positive += delta
		else:
			total_negative += delta

	var condition_cost = result.get("condition_cost", 0)

	if total_positive > 0 or total_negative < 0:
		var parts = []
		if total_positive > 0:
			parts.append("[color=#238636]+%d[/color]" % total_positive)
		if total_negative < 0:
			parts.append("[color=#DA3633]%d[/color]" % total_negative)

		total_label.text = "총 변화: " + " / ".join(parts)
		if condition_cost > 0:
			total_label.text += " | 컨디션 -%d%%" % int(condition_cost)
	else:
		total_label.text = ""


# ============================================
# 애니메이션
# ============================================


func _animate_show() -> void:
	# 기존 애니메이션 정리
	for tween in _animation_tweens:
		if tween and tween.is_valid():
			tween.kill()
	_animation_tweens.clear()
	_cleanup_particles()

	# 배경 페이드인
	var bg_tween = create_tween()
	bg_tween.tween_property(self, "modulate:a", 1.0, 0.2)
	_animation_tweens.append(bg_tween)

	# 패널 스케일 애니메이션
	if panel:
		panel.scale = Vector2(0.8, 0.8)
		panel.pivot_offset = panel.size / 2

		var panel_tween = create_tween()
		panel_tween.set_ease(Tween.EASE_OUT)
		panel_tween.set_trans(Tween.TRANS_BACK)
		panel_tween.tween_property(panel, "scale", Vector2(1.0, 1.0), 0.3)
		_animation_tweens.append(panel_tween)

	# P0: Grade-specific special effects
	match _current_grade:
		TrainingGrade.GREAT_SUCCESS:
			_animate_great_success()
		TrainingGrade.SUCCESS:
			_animate_success()


## P0: Great Success special animation (Kairosoft "Super Parameter Up" style)
func _animate_great_success() -> void:
	# 1. Screen flash effect
	var flash := ColorRect.new()
	flash.color = Color(1.0, 0.9, 0.0, 0.0)  # Gold flash
	flash.set_anchors_preset(Control.PRESET_FULL_RECT)
	flash.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(flash)
	move_child(flash, 0)  # Behind panel
	_particle_nodes.append(flash)

	var flash_tween := create_tween()
	flash_tween.tween_property(flash, "color:a", 0.6, 0.1)
	flash_tween.tween_property(flash, "color:a", 0.0, 0.4)
	_animation_tweens.append(flash_tween)

	# 2. Title pulse animation
	if title_label:
		var pulse_tween := create_tween().set_loops(3)
		pulse_tween.tween_property(title_label, "scale", Vector2(1.15, 1.15), 0.15)
		pulse_tween.tween_property(title_label, "scale", Vector2(1.0, 1.0), 0.15)
		title_label.pivot_offset = title_label.size / 2
		_animation_tweens.append(pulse_tween)

	# 3. Spawn sparkle particles
	_spawn_sparkles(8, COLOR_GREAT_SUCCESS)

	# 4. Panel glow effect
	if panel:
		var glow_tween := create_tween().set_loops(2)
		glow_tween.tween_property(panel, "modulate", Color(1.2, 1.1, 0.9), 0.2)
		glow_tween.tween_property(panel, "modulate", Color.WHITE, 0.2)
		_animation_tweens.append(glow_tween)


## P0: Success animation (milder version)
func _animate_success() -> void:
	# Title subtle pulse
	if title_label:
		var pulse_tween := create_tween().set_loops(2)
		pulse_tween.tween_property(title_label, "scale", Vector2(1.08, 1.08), 0.12)
		pulse_tween.tween_property(title_label, "scale", Vector2(1.0, 1.0), 0.12)
		title_label.pivot_offset = title_label.size / 2
		_animation_tweens.append(pulse_tween)

	# Spawn fewer sparkles
	_spawn_sparkles(4, COLOR_SUCCESS)


## P0: Spawn sparkle particle effects
func _spawn_sparkles(count: int, color: Color) -> void:
	var viewport_size := get_viewport_rect().size
	var center := viewport_size / 2

	for i in range(count):
		var sparkle := Label.new()
		sparkle.text = "✦"
		sparkle.add_theme_font_size_override("font_size", randi_range(16, 28))
		sparkle.add_theme_color_override("font_color", color)
		sparkle.modulate.a = 0.0

		# Random position around center
		var angle := randf() * TAU
		var dist := randf_range(80, 200)
		var start_pos := center + Vector2(cos(angle), sin(angle)) * dist
		sparkle.position = start_pos
		sparkle.pivot_offset = sparkle.size / 2

		add_child(sparkle)
		_particle_nodes.append(sparkle)

		# Animate: fade in, float up, fade out
		var tween := create_tween()
		tween.set_parallel(true)
		tween.tween_property(sparkle, "modulate:a", 1.0, 0.2).set_delay(i * 0.05)
		tween.tween_property(sparkle, "position:y", start_pos.y - randf_range(40, 80), 0.8).set_delay(i * 0.05)
		tween.tween_property(sparkle, "rotation", randf_range(-0.5, 0.5), 0.8).set_delay(i * 0.05)
		tween.set_parallel(false)
		tween.tween_property(sparkle, "modulate:a", 0.0, 0.3)
		_animation_tweens.append(tween)


## P0: Cleanup particle nodes
func _cleanup_particles() -> void:
	for node in _particle_nodes:
		if is_instance_valid(node):
			node.queue_free()
	_particle_nodes.clear()


func _animate_hide() -> void:
	_cleanup_particles()
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.tween_property(self, "modulate:a", 0.0, 0.15)
	tween.tween_callback(
		func():
			visible = false
			closed.emit()
	)


# ============================================
# 이벤트 핸들러
# ============================================


func _on_close_pressed() -> void:
	hide_popup()


func _on_background_clicked(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		_on_close_pressed()


# ============================================
# 유틸리티
# ============================================


func _translate_attribute(attr: String) -> String:
	"""속성 이름 한글화"""
	const TRANSLATIONS = {
		"Finishing": "골 결정력",
		"LongShots": "중거리 슛",
		"Passing": "패스",
		"Crossing": "크로스",
		"Dribbling": "드리블",
		"Agility": "민첩성",
		"Stamina": "지구력",
		"Strength": "근력",
		"Vision": "시야",
		"Positioning": "포지셔닝",
		"Tackling": "태클",
		"Marking": "마킹",
		"Pace": "속도",
		"Acceleration": "가속력",
		"Composure": "침착성",
		"Balance": "밸런스",
		"Heading": "헤딩",
		"Interception": "인터셉트",
		"Mentality": "정신력",
	}
	return TRANSLATIONS.get(attr, attr)
