extends PanelContainer
class_name TacticalPresetCard

## Tactical Preset Card UI Component
##
## Phase 3.3: UI 프리셋 선택 메뉴
##
## 개별 전술 프리셋을 표시하는 카드 UI 컴포넌트입니다.
## 선택 상태를 관리하고, 사용자 클릭에 반응합니다.
##
## 사용법:
##   var card = TacticalPresetCard.new()
##   card.set_preset("tiki_taka")
##   card.preset_selected.connect(_on_preset_selected)

## 시그널
signal preset_selected(preset_id: String)

## 프리셋 데이터
var _preset_id: String = ""
var _preset_data: Dictionary = {}
var _is_selected: bool = false

## 선택 상태 색상
const COLOR_NORMAL = Color(0.15, 0.15, 0.15, 0.9)
const COLOR_SELECTED = Color(0.3, 0.5, 0.8, 1.0)
const COLOR_HOVER = Color(0.2, 0.2, 0.2, 1.0)

## UI 노드 (동적 생성)
var title_container: HBoxContainer
var icon_label: Label
var title_label: Label
var description_label: Label
var stats_container: VBoxContainer
var vbox: VBoxContainer


func _ready():
	_create_ui()
	_setup_interactions()


## UI 생성
func _create_ui():
	# 최소 크기 설정
	custom_minimum_size = Vector2(200, 180)

	# VBoxContainer
	vbox = VBoxContainer.new()
	vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(vbox)

	# Title Container (아이콘 + 이름)
	title_container = HBoxContainer.new()
	vbox.add_child(title_container)

	icon_label = Label.new()
	icon_label.add_theme_font_size_override("font_size", 24)
	title_container.add_child(icon_label)

	title_label = Label.new()
	title_label.add_theme_font_size_override("font_size", 18)
	title_label.add_theme_color_override("font_color", Color(1, 1, 1))
	title_container.add_child(title_label)

	# Description Label
	description_label = Label.new()
	description_label.add_theme_font_size_override("font_size", 12)
	description_label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	description_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	description_label.custom_minimum_size = Vector2(0, 40)
	vbox.add_child(description_label)

	# Spacer
	var spacer = Control.new()
	spacer.custom_minimum_size = Vector2(0, 10)
	vbox.add_child(spacer)

	# Stats Container
	stats_container = VBoxContainer.new()
	stats_container.add_theme_constant_override("separation", 4)
	vbox.add_child(stats_container)


## 인터랙션 설정
func _setup_interactions():
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)
	gui_input.connect(_on_gui_input)


## 프리셋 데이터 설정
func set_preset(preset_id: String):
	"""
	프리셋 ID로 카드 데이터 설정

	Args:
		preset_id: 프리셋 ID (예: "tiki_taka")
	"""
	_preset_id = preset_id
	_preset_data = TacticalPresets.get_preset(preset_id)

	if _preset_data.is_empty():
		push_warning("[TacticalPresetCard] Unknown preset: %s" % preset_id)
		return

	_update_ui()


## UI 업데이트
func _update_ui():
	# 제목
	if icon_label:
		icon_label.text = _preset_data.get("icon", "")
	if title_label:
		title_label.text = _preset_data.get("name", "")

	# 설명
	if description_label:
		description_label.text = _preset_data.get("description", "")

	# 전술 값 미리보기
	if stats_container:
		_update_stats()

	# 색상
	_update_color()


## 전술 값 표시
func _update_stats():
	# Clear existing
	for child in stats_container.get_children():
		child.queue_free()

	var instructions = _preset_data.get("instructions", {})

	# Tempo
	_add_stat_label("Tempo:", instructions.get("tempo", "Medium"), _get_tempo_icon(instructions.get("tempo", "Medium")))

	# Pressing
	_add_stat_label(
		"Pressing:", instructions.get("pressing", "Medium"), _get_pressing_icon(instructions.get("pressing", "Medium"))
	)

	# Width
	_add_stat_label("Width:", instructions.get("width", "Medium"), _get_width_icon(instructions.get("width", "Medium")))


## 통계 라벨 추가
func _add_stat_label(label_name: String, value: String, icon: String):
	var hbox = HBoxContainer.new()

	var name_label = Label.new()
	name_label.text = label_name
	name_label.custom_minimum_size = Vector2(70, 0)
	name_label.add_theme_font_size_override("font_size", 11)
	name_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	hbox.add_child(name_label)

	var value_label = Label.new()
	value_label.text = value
	value_label.add_theme_font_size_override("font_size", 11)
	value_label.add_theme_color_override("font_color", Color(0.9, 0.9, 0.9))
	value_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_child(value_label)

	var icon_label = Label.new()
	icon_label.text = " " + icon
	icon_label.add_theme_font_size_override("font_size", 14)
	hbox.add_child(icon_label)

	stats_container.add_child(hbox)


## 아이콘 헬퍼
func _get_tempo_icon(tempo: String) -> String:
	"""
	Tempo 값에 해당하는 아이콘 반환

	Args:
		tempo: Tempo 값 (VerySlow/Slow/Medium/Fast/VeryFast)

	Returns:
		String: 아이콘 (이모지)
	"""
	match tempo:
		"VerySlow":
			return "🐌"
		"Slow":
			return "🚶"
		"Medium":
			return "🏃"
		"Fast":
			return "⚡"
		"VeryFast":
			return "🚀"
		_:
			return ""


func _get_pressing_icon(pressing: String) -> String:
	"""
	Pressing 값에 해당하는 아이콘 반환

	Args:
		pressing: Pressing 값 (VeryLow/Low/Medium/High/VeryHigh)

	Returns:
		String: 아이콘 (이모지)
	"""
	match pressing:
		"VeryLow":
			return "❄️"
		"Low":
			return "💧"
		"Medium":
			return "🔥"
		"High":
			return "🔥🔥"
		"VeryHigh":
			return "🔥🔥🔥"
		_:
			return ""


func _get_width_icon(width: String) -> String:
	"""
	Width 값에 해당하는 아이콘 반환

	Args:
		width: Width 값 (Narrow/Medium/Wide)

	Returns:
		String: 아이콘 (이모지)
	"""
	match width:
		"Narrow":
			return "↕️"
		"Medium":
			return "↔️"
		"Wide":
			return "↔️↔️"
		_:
			return ""


## 선택 상태 설정
func set_selected(selected: bool):
	"""
	카드 선택 상태 설정

	Args:
		selected: 선택 여부
	"""
	_is_selected = selected
	_update_color()


## 선택 상태 반환
func is_selected() -> bool:
	"""
	현재 선택 상태 반환

	Returns:
		bool: 선택 여부
	"""
	return _is_selected


## 프리셋 ID 반환
func get_preset_id() -> String:
	"""
	현재 프리셋 ID 반환

	Returns:
		String: 프리셋 ID
	"""
	return _preset_id


## 색상 업데이트
func _update_color():
	if _is_selected:
		self_modulate = COLOR_SELECTED
	else:
		self_modulate = COLOR_NORMAL


## 마우스 호버
func _on_mouse_entered():
	if not _is_selected:
		self_modulate = COLOR_HOVER


func _on_mouse_exited():
	_update_color()


## 클릭 이벤트
func _on_gui_input(event: InputEvent):
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			preset_selected.emit(_preset_id)
			print("[TacticalPresetCard] Preset selected: %s" % _preset_id)
