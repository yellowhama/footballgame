extends Control
## TacticsScreen - 전술 설정 메인 화면
## TacticalEngine, FormationManager 연동
##
## 작성일: 2025-11-26
## 참조: 04_ui_design_system.md, working_apis.md

signal back_requested
signal tactics_saved(tactics: Dictionary)

# ============================================
# UI 노드 참조
# ============================================

@onready var back_button: Button = $Header/BackButton
@onready var title_label: Label = $Header/TitleLabel
@onready var save_button: Button = $Header/SaveButton

@onready var formation_container: Control = $Content/LeftPanel/FormationContainer
@onready var formation_dropdown: OptionButton = $Content/LeftPanel/FormationDropdown
@onready var formation_visualizer: Control = $Content/LeftPanel/FormationVisualizer
@onready var formation_label: Label = $Content/LeftPanel/FormationLabel

@onready var tactics_panel: Control = $Content/RightPanel/TacticsPanel
@onready var style_container: HBoxContainer = $Content/RightPanel/StyleContainer
@onready var instructions_container: VBoxContainer = $Content/RightPanel/InstructionsContainer

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG_PRIMARY = Color("#0D1117")
const COLOR_BG_SECONDARY = Color("#161B22")
const COLOR_BG_ELEVATED = Color("#30363D")
const COLOR_ACCENT_PRIMARY = Color("#238636")
const COLOR_ACCENT_SECONDARY = Color("#1F6FEB")
const COLOR_ACCENT_WARNING = Color("#D29922")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")

# 포지션 색상
const POSITION_COLORS = {
	"GK": Color("#FFC107"), "DEF": Color("#4CAF50"), "MID": Color("#2196F3"), "FWD": Color("#F44336")
}

# ============================================
# 상태 변수
# ============================================

var _current_formation: String = "4-4-2"
var _current_style: String = "balanced"
var _current_instructions: Dictionary = {}
var _formations: Array = []
var _position_markers: Array = []
var _style_buttons: Dictionary = {}
var _instruction_sliders: Dictionary = {}

# 전술 스타일
const TACTICAL_STYLES = {
	"attacking": {"label": "공격적", "icon": "⚔️", "description": "적극적인 전진과 공격 위주"},
	"defensive": {"label": "수비적", "icon": "🛡️", "description": "견고한 수비와 역습 기회"},
	"balanced": {"label": "균형", "icon": "⚖️", "description": "상황에 따른 유동적 대응"},
	"counter": {"label": "역습", "icon": "↩️", "description": "빠른 전환 플레이"},
	"possession": {"label": "점유", "icon": "⚽", "description": "볼 점유율 중시 플레이"}
}

# 전술 인스트럭션
const TACTICAL_INSTRUCTIONS = {
	"defensive_line_height":
	{"label": "수비 라인", "min": 0.0, "max": 1.0, "default": 0.5, "low_text": "낮음", "high_text": "높음"},
	"pressing_intensity":
	{"label": "압박 강도", "min": 0.0, "max": 1.0, "default": 0.5, "low_text": "낮음", "high_text": "높음"},
	"tempo": {"label": "템포", "min": 0.0, "max": 1.0, "default": 0.5, "low_text": "느림", "high_text": "빠름"},
	"width": {"label": "팀 폭", "min": 0.0, "max": 1.0, "default": 0.5, "low_text": "좁음", "high_text": "넓음"}
}

# 네비바 씬
const MainNavBarScene = preload("res://scenes/components/MainNavBar.tscn")

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_connect_signals()
	_setup_ui()
	_load_formations()
	_build_style_buttons()
	_build_instruction_sliders()
	_load_current_tactics()
	_add_navigation_bar()
	print("[TacticsScreen] Initialized")


func _add_navigation_bar() -> void:
	if MainNavBarScene:
		var navbar = MainNavBarScene.instantiate()
		add_child(navbar)
		navbar.set_active_tab("tactics")


func _connect_signals() -> void:
	if back_button:
		back_button.pressed.connect(_on_back_pressed)
	if save_button:
		save_button.pressed.connect(_on_save_pressed)
	if formation_dropdown:
		formation_dropdown.item_selected.connect(_on_formation_selected)


func _setup_ui() -> void:
	if has_node("Background"):
		$Background.color = COLOR_BG_PRIMARY


# ============================================
# 포메이션 관리
# ============================================


func _load_formations() -> void:
	"""포메이션 목록 로드"""
	if FormationManager:
		_formations = FormationManager.get_recommended_formations([])
	else:
		# 기본 포메이션 목록
		_formations = [
			{"formation": "4-4-2", "style": "균형"},
			{"formation": "4-3-3", "style": "공격"},
			{"formation": "3-5-2", "style": "측면"},
			{"formation": "4-2-3-1", "style": "현대"},
			{"formation": "5-3-2", "style": "수비"},
			{"formation": "4-1-4-1", "style": "중앙"},
			{"formation": "3-4-3", "style": "공격"}
		]

	_populate_formation_dropdown()
	_update_formation_visualizer()


func _populate_formation_dropdown() -> void:
	if not formation_dropdown:
		return

	formation_dropdown.clear()
	for i in range(_formations.size()):
		var formation = _formations[i]
		var label = "%s (%s)" % [formation.get("formation", ""), formation.get("style", "")]
		formation_dropdown.add_item(label, i)

	# 현재 포메이션 선택
	for i in range(_formations.size()):
		if _formations[i].get("formation", "") == _current_formation:
			formation_dropdown.select(i)
			break


func _update_formation_visualizer() -> void:
	"""포메이션 시각화 업데이트"""
	if not formation_visualizer:
		return

	# 기존 마커 제거
	for marker in _position_markers:
		marker.queue_free()
	_position_markers.clear()

	# 포지션 요구사항 가져오기
	var requirements: Dictionary
	if FormationManager:
		requirements = FormationManager.get_position_requirements(_current_formation)
	else:
		requirements = _get_default_positions(_current_formation)

	# 필드 크기 계산
	var field_size = formation_visualizer.size
	var field_width = field_size.x
	var field_height = field_size.y

	# 골키퍼
	_create_position_marker(Vector2(field_width / 2, field_height - 30), "GK", "GK")

	# 수비수
	var defenders = requirements.get("defenders", [])
	_create_position_line(defenders, field_height * 0.75, field_width, "DEF")

	# 미드필더
	var midfielders = requirements.get("midfielders", [])
	_create_position_line(midfielders, field_height * 0.45, field_width, "MID")

	# 공격수
	var forwards = requirements.get("forwards", [])
	_create_position_line(forwards, field_height * 0.15, field_width, "FWD")

	# 포메이션 라벨 업데이트
	if formation_label:
		formation_label.text = _current_formation


func _create_position_line(positions: Array, y_pos: float, width: float, pos_type: String) -> void:
	"""한 줄의 포지션 마커 생성"""
	var count = positions.size()
	if count == 0:
		return

	var spacing = width / (count + 1)
	for i in range(count):
		var x_pos = spacing * (i + 1)
		var pos_name = positions[i] if i < positions.size() else pos_type
		_create_position_marker(Vector2(x_pos, y_pos), pos_name, pos_type)


func _create_position_marker(pos: Vector2, label: String, pos_type: String) -> void:
	"""포지션 마커 생성"""
	var marker = ColorRect.new()
	marker.size = Vector2(36, 36)
	marker.position = pos - marker.size / 2
	marker.color = POSITION_COLORS.get(pos_type, Color.WHITE)

	# 둥근 모서리 효과 (간단히)
	var pos_label = Label.new()
	pos_label.text = label
	pos_label.add_theme_font_size_override("font_size", 10)
	pos_label.add_theme_color_override("font_color", Color.WHITE)
	pos_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	pos_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	pos_label.anchors_preset = Control.PRESET_FULL_RECT
	marker.add_child(pos_label)

	formation_visualizer.add_child(marker)
	_position_markers.append(marker)


func _get_default_positions(formation: String) -> Dictionary:
	"""기본 포지션 요구사항"""
	match formation:
		"4-4-2":
			return {
				"defenders": ["LB", "CB", "CB", "RB"], "midfielders": ["LM", "CM", "CM", "RM"], "forwards": ["ST", "ST"]
			}
		"4-3-3":
			return {
				"defenders": ["LB", "CB", "CB", "RB"], "midfielders": ["CM", "CM", "CM"], "forwards": ["LW", "CF", "RW"]
			}
		"3-5-2":
			return {
				"defenders": ["CB", "CB", "CB"], "midfielders": ["LM", "CM", "DM", "CM", "RM"], "forwards": ["ST", "ST"]
			}
		"4-2-3-1":
			return {
				"defenders": ["LB", "CB", "CB", "RB"], "midfielders": ["DM", "DM", "LM", "AM", "RM"], "forwards": ["ST"]
			}
		"5-3-2":
			return {
				"defenders": ["LB", "CB", "CB", "CB", "RB"], "midfielders": ["CM", "DM", "CM"], "forwards": ["ST", "ST"]
			}
		_:
			return {
				"defenders": ["LB", "CB", "CB", "RB"], "midfielders": ["LM", "CM", "CM", "RM"], "forwards": ["ST", "ST"]
			}


# ============================================
# 전술 스타일
# ============================================


func _build_style_buttons() -> void:
	"""전술 스타일 버튼 생성"""
	if not style_container:
		return

	# 기존 버튼 제거
	for child in style_container.get_children():
		child.queue_free()

	for style_id in TACTICAL_STYLES:
		var style_data = TACTICAL_STYLES[style_id]
		var btn = Button.new()
		btn.text = "%s %s" % [style_data.get("icon", ""), style_data.get("label", "")]
		btn.tooltip_text = style_data.get("description", "")
		btn.custom_minimum_size = Vector2(80, 50)
		btn.pressed.connect(_on_style_selected.bind(style_id))
		style_container.add_child(btn)
		_style_buttons[style_id] = btn

	_update_style_buttons()


func _update_style_buttons() -> void:
	"""스타일 버튼 상태 업데이트"""
	for style_id in _style_buttons:
		var btn = _style_buttons[style_id]
		if style_id == _current_style:
			btn.add_theme_color_override("font_color", COLOR_ACCENT_SECONDARY)
		else:
			btn.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)


# ============================================
# 전술 인스트럭션
# ============================================


func _build_instruction_sliders() -> void:
	"""전술 인스트럭션 슬라이더 생성"""
	if not instructions_container:
		return

	# 기존 슬라이더 제거
	for child in instructions_container.get_children():
		child.queue_free()

	for param_id in TACTICAL_INSTRUCTIONS:
		var param_data = TACTICAL_INSTRUCTIONS[param_id]
		var slider_row = _create_instruction_slider(param_id, param_data)
		instructions_container.add_child(slider_row)


func _create_instruction_slider(param_id: String, param_data: Dictionary) -> Control:
	"""인스트럭션 슬라이더 행 생성"""
	var row = VBoxContainer.new()
	row.custom_minimum_size = Vector2(0, 60)

	# 라벨 행
	var label_row = HBoxContainer.new()
	row.add_child(label_row)

	var label = Label.new()
	label.text = param_data.get("label", param_id)
	label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label_row.add_child(label)

	var value_label = Label.new()
	value_label.name = "ValueLabel"
	value_label.text = "50%"
	value_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	label_row.add_child(value_label)

	# 슬라이더 행
	var slider_row = HBoxContainer.new()
	row.add_child(slider_row)

	var low_label = Label.new()
	low_label.text = param_data.get("low_text", "낮음")
	low_label.add_theme_font_size_override("font_size", 12)
	low_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	slider_row.add_child(low_label)

	var slider = HSlider.new()
	slider.min_value = param_data.get("min", 0.0)
	slider.max_value = param_data.get("max", 1.0)
	slider.value = param_data.get("default", 0.5)
	slider.step = 0.05
	slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	slider.value_changed.connect(_on_instruction_changed.bind(param_id, value_label))
	slider_row.add_child(slider)

	var high_label = Label.new()
	high_label.text = param_data.get("high_text", "높음")
	high_label.add_theme_font_size_override("font_size", 12)
	high_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	slider_row.add_child(high_label)

	_instruction_sliders[param_id] = slider
	_current_instructions[param_id] = slider.value

	return row


# ============================================
# 전술 로드/저장
# ============================================


func _load_current_tactics() -> void:
	"""현재 전술 설정 로드"""
	if TacticalEngine:
		# TacticalEngine에서 현재 설정 가져오기
		# (저장된 설정이 있다면)
		pass

	# 기본값 적용
	_current_formation = "4-4-2"
	_current_style = "balanced"

	for param_id in TACTICAL_INSTRUCTIONS:
		var default_val = TACTICAL_INSTRUCTIONS[param_id].get("default", 0.5)
		_current_instructions[param_id] = default_val
		if _instruction_sliders.has(param_id):
			_instruction_sliders[param_id].value = default_val


func _save_current_tactics() -> Dictionary:
	"""현재 전술 설정 저장"""
	var tactics = {
		"formation": _current_formation, "style": _current_style, "instructions": _current_instructions.duplicate()
	}

	# TacticalEngine으로 전송
	if TacticalEngine:
		var result = TacticalEngine.update_session_tactics(tactics)
		if not result.get("success", false):
			push_warning("[TacticsScreen] Failed to save tactics: %s" % result.get("error", "Unknown"))

	tactics_saved.emit(tactics)
	return tactics


# ============================================
# 이벤트 핸들러
# ============================================


func _on_back_pressed() -> void:
	back_requested.emit()
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_save_pressed() -> void:
	var saved = _save_current_tactics()
	print("[TacticsScreen] Tactics saved: %s" % saved)
	_show_notification("전술 설정이 저장되었습니다")


func _on_formation_selected(index: int) -> void:
	if index < 0 or index >= _formations.size():
		return

	_current_formation = _formations[index].get("formation", "4-4-2")
	_update_formation_visualizer()
	print("[TacticsScreen] Formation changed: %s" % _current_formation)


func _on_style_selected(style_id: String) -> void:
	_current_style = style_id
	_update_style_buttons()
	print("[TacticsScreen] Style changed: %s" % style_id)


func _on_instruction_changed(value: float, param_id: String, value_label: Label) -> void:
	_current_instructions[param_id] = value
	value_label.text = "%d%%" % int(value * 100)


# ============================================
# UI 피드백
# ============================================


func _show_notification(message: String) -> void:
	print("[TacticsScreen] %s" % message)
	# TODO: 토스트 알림 표시


# ============================================
# 외부 API
# ============================================


func refresh() -> void:
	"""화면 새로고침"""
	_load_current_tactics()
	_update_formation_visualizer()
	_update_style_buttons()


func get_current_tactics() -> Dictionary:
	"""현재 전술 설정 반환"""
	return {"formation": _current_formation, "style": _current_style, "instructions": _current_instructions.duplicate()}
