extends Control
class_name InstructionSelector

# Reusable component for selecting player instruction values
# Follows SkillWidget.gd pattern with signals and ThemeManager integration

signal instruction_changed(instruction_name: String, new_value: String)
signal instruction_hovered(instruction_name: String)

@export var instruction_name: String = ""
@export var instruction_label_ko: String = ""
@export var current_value: String = ""
@export var options: Array[String] = []

var _label: Label
var _option_button: OptionButton
var _description_label: Label
var _container: VBoxContainer


func _ready():
	_create_ui()
	_apply_styles()
	_connect_signals()
	_update_display()


func _create_ui():
	# Main container
	_container = VBoxContainer.new()
	_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_child(_container)

	# Header with label
	var header = HBoxContainer.new()
	header.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_container.add_child(header)

	_label = Label.new()
	_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_label.text = instruction_label_ko if instruction_label_ko else instruction_name
	header.add_child(_label)

	# Option button (dropdown)
	_option_button = OptionButton.new()
	_option_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_option_button.custom_minimum_size = Vector2(0, 80)  # Touch-friendly height
	_container.add_child(_option_button)

	# Description label (shows current selection info)
	_description_label = Label.new()
	_description_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_description_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_description_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_description_label.custom_minimum_size = Vector2(0, 60)
	_container.add_child(_description_label)

	# Populate options
	_populate_options()


func _populate_options():
	_option_button.clear()

	for i in range(options.size()):
		var option = options[i]
		var display_text = _get_korean_label(option)
		_option_button.add_item(display_text, i)

		# Set selected item if it matches current_value
		if option == current_value:
			_option_button.select(i)


func _get_korean_label(value: String) -> String:
	# Korean labels for instruction values (common across all types)
	var common_labels = {
		# Mentality
		"Conservative": "보수적",
		"Balanced": "균형",
		"Aggressive": "공격적",
		# Width
		"StayWide": "넓게 유지",
		"Normal": "보통",
		"CutInside": "안쪽으로",
		"Roam": "자유롭게",
		# Depth
		"StayBack": "뒤에 머무름",
		"Standard": "표준",
		"GetForward": "앞으로 전진",
		"PushUp": "밀어올림",
		# PassingStyle
		"Short": "짧게",
		"Mixed": "혼합",
		"Direct": "직접",
		"Long": "길게",
		# DribblingFrequency
		"Rarely": "거의 안함",
		"Occasionally": "가끔",
		"Frequently": "자주",
		"Always": "항상",
		# ShootingTendency
		"LowFrequency": "낮음",
		"HighFrequency": "높음",
		"ShootOnSight": "즉시 슛",
		# DefensiveWork
		"Minimal": "최소",
		"High": "높음",
		"Maximum": "최대",
		# PressingIntensity
		"Low": "낮음",
		"Medium": "중간",
		"VeryHigh": "매우 높음"
	}

	return common_labels.get(value, value)


func _get_description(value: String) -> String:
	# Descriptions organized by instruction type to handle overlapping enum values
	var descriptions_by_type = {
		"mentality":
		{"Conservative": "수비적으로 플레이, 리스크 최소화", "Balanced": "공수 균형잡힌 플레이", "Aggressive": "공격적으로 플레이, 높은 리스크"},
		"width": {"StayWide": "측면을 넓게 활용", "Normal": "일반적인 포지션 유지", "CutInside": "중앙으로 이동", "Roam": "자유롭게 움직임"},
		"depth": {"StayBack": "수비 라인 유지", "Standard": "표준 포지션", "GetForward": "공격 가담", "PushUp": "전방 압박"},
		"passing_style": {"Short": "짧은 패스 위주", "Mixed": "상황에 맞춰 패스", "Direct": "직선적인 패스", "Long": "긴 패스 위주"},
		"dribbling_frequency":
		{"Rarely": "드리블 거의 안함", "Occasionally": "필요시 드리블", "Frequently": "드리블 자주 시도", "Always": "드리블 우선"},
		"shooting_tendency":
		{"LowFrequency": "확실한 기회만 슛", "Normal": "적절한 타이밍에 슛", "HighFrequency": "자주 슛 시도", "ShootOnSight": "기회되면 즉시 슛"},
		"defensive_work": {"Minimal": "수비 최소한만", "Normal": "일반적인 수비", "High": "적극적인 수비", "Maximum": "최대한 수비 가담"},
		"pressing_intensity": {"Low": "느슨한 압박", "Medium": "적절한 압박", "High": "강한 압박", "VeryHigh": "매우 강한 압박"}
	}

	# Try to get description from the specific instruction type
	var type_key = instruction_name.to_lower()
	if descriptions_by_type.has(type_key):
		return descriptions_by_type[type_key].get(value, "")

	# Fallback: search all types for the value
	for type in descriptions_by_type.values():
		if type.has(value):
			return type[value]

	return ""


func _apply_styles():
	# ThemeManager integration for consistent styling
	# Background panel
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.15, 0.15, 0.2, 1.0)  # ThemeManager.BG_SURFACE equivalent
	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	style.content_margin_left = 16
	style.content_margin_right = 16
	style.content_margin_top = 12
	style.content_margin_bottom = 12

	# Apply to self
	add_theme_stylebox_override("panel", style)

	# Label styling
	if _label:
		_label.add_theme_font_size_override("font_size", 28)
		_label.add_theme_color_override("font_color", Color(0.9, 0.9, 0.9, 1.0))

	# Description styling
	if _description_label:
		_description_label.add_theme_font_size_override("font_size", 22)
		_description_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7, 1.0))

	# OptionButton styling
	if _option_button:
		_option_button.add_theme_font_size_override("font_size", 26)


func _connect_signals():
	if _option_button:
		_option_button.item_selected.connect(_on_option_selected)
		_option_button.mouse_entered.connect(_on_mouse_entered)


func _update_display():
	if _description_label and current_value:
		_description_label.text = _get_description(current_value)


func _on_option_selected(index: int):
	if index < 0 or index >= options.size():
		return

	var new_value = options[index]
	if new_value != current_value:
		current_value = new_value
		_update_display()
		instruction_changed.emit(instruction_name, new_value)


func _on_mouse_entered():
	instruction_hovered.emit(instruction_name)


# Public API
func set_value(value: String):
	"""Update the selected value programmatically"""
	if value in options:
		current_value = value
		# Find and select the matching option
		for i in range(options.size()):
			if options[i] == value:
				_option_button.select(i)
				break
		_update_display()


func get_value() -> String:
	"""Get the currently selected value"""
	return current_value


func set_options(new_options: Array[String]):
	"""Update available options"""
	options = new_options
	if _option_button:
		_populate_options()


func set_label(label_ko: String):
	"""Update the display label"""
	instruction_label_ko = label_ko
	if _label:
		_label.text = label_ko
