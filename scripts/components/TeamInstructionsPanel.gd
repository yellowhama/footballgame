extends Control

## TeamInstructionsPanel - Phase 5 Team Instructions System
## Provides UI for editing team-wide tactical instructions with OpenFootball integration

signal instructions_applied(instructions: Dictionary)
signal preset_applied(preset_name: String)

# UI Components
var preset_buttons: Array = []
var instruction_selectors: Dictionary = {}
var modifier_labels: Dictionary = {}

# Current instructions
var current_instructions: Dictionary = {}
var available_options: Dictionary = {}
var tactical_presets: Array = []

# Offside trap toggle
var offside_trap_checkbox: CheckButton = null
var use_offside_trap: bool = false

# Rust Engine reference
var rust_engine: Node = null


func _ready():
	print("[TeamInstructionsPanel] Initializing Phase 5 team instructions panel")

	# Get Rust Engine
	rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[TeamInstructionsPanel] ERROR: FootballRustEngine not available!")
		_show_error_ui()
		return

	# Load data from API
	_load_instruction_options()
	_load_tactical_presets()

	# Build UI
	_build_ui()

	# Load current instructions (default to Balanced if none set)
	_apply_preset("Balanced")

	print("[TeamInstructionsPanel] Initialization complete")


func _build_ui():
	"""Build the team instructions UI"""
	# Main container
	var main_vbox = VBoxContainer.new()
	main_vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_vbox.anchor_right = 1.0
	main_vbox.anchor_bottom = 1.0
	main_vbox.add_theme_constant_override("separation", 15)
	add_child(main_vbox)

	# Add margin
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_top", 20)
	margin.add_theme_constant_override("margin_bottom", 20)
	main_vbox.add_child(margin)

	var content_vbox = VBoxContainer.new()
	content_vbox.add_theme_constant_override("separation", 20)
	content_vbox.size_flags_vertical = Control.SIZE_EXPAND_FILL
	margin.add_child(content_vbox)

	# Title
	var title = Label.new()
	title.text = "팀 지시사항 (Team Instructions)"
	title.add_theme_font_size_override("font_size", 28)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content_vbox.add_child(title)

	# Description
	var desc = Label.new()
	desc.text = "팀 전체에 적용되는 전술적 지시사항을 설정합니다"
	desc.add_theme_font_size_override("font_size", 16)
	desc.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	desc.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content_vbox.add_child(desc)

	# Separator
	var sep1 = HSeparator.new()
	content_vbox.add_child(sep1)

	# Preset buttons section
	var preset_section = _create_preset_section()
	content_vbox.add_child(preset_section)

	# Separator
	var sep2 = HSeparator.new()
	content_vbox.add_child(sep2)

	# Instructions section (with scroll)
	var scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content_vbox.add_child(scroll)

	var instructions_vbox = VBoxContainer.new()
	instructions_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	instructions_vbox.add_theme_constant_override("separation", 15)
	scroll.add_child(instructions_vbox)

	# Custom Instructions Header
	var custom_header = Label.new()
	custom_header.text = "커스텀 지시사항 설정"
	custom_header.add_theme_font_size_override("font_size", 20)
	instructions_vbox.add_child(custom_header)

	# Create instruction selectors for 5 categories
	var categories = ["defensive_line", "team_width", "team_tempo", "pressing_intensity", "build_up_style"]
	for category in categories:
		var category_container = _create_instruction_selector(category)
		instructions_vbox.add_child(category_container)

	# Offside trap toggle (only effective with High/VeryHigh defensive line)
	var offside_container = _create_offside_trap_toggle()
	instructions_vbox.add_child(offside_container)

	# Separator
	var sep3 = HSeparator.new()
	instructions_vbox.add_child(sep3)

	# Modifier preview section
	var preview_section = _create_modifier_preview_section()
	instructions_vbox.add_child(preview_section)

	# Apply button
	var apply_btn = Button.new()
	apply_btn.text = "✅ 변경사항 적용"
	apply_btn.custom_minimum_size = Vector2(200, 60)
	apply_btn.add_theme_font_size_override("font_size", 18)
	apply_btn.pressed.connect(_on_apply_custom_instructions)
	instructions_vbox.add_child(apply_btn)


func _create_preset_section() -> Control:
	"""Create tactical preset buttons section"""
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 10)

	# Section title
	var title = Label.new()
	title.text = "전술 프리셋 (Tactical Presets)"
	title.add_theme_font_size_override("font_size", 20)
	section.add_child(title)

	# Preset buttons grid
	var grid = GridContainer.new()
	grid.columns = 2  # 2 columns for mobile layout
	grid.add_theme_constant_override("h_separation", 10)
	grid.add_theme_constant_override("v_separation", 10)
	section.add_child(grid)

	# Create buttons for each preset
	for preset in tactical_presets:
		var btn = _create_preset_button(preset)
		preset_buttons.append(btn)
		grid.add_child(btn)

	return section


func _create_preset_button(preset_data: Dictionary) -> Button:
	"""Create a preset button"""
	var btn = Button.new()
	btn.custom_minimum_size = Vector2(400, 100)

	var preset_id = preset_data.get("id", "")
	var name_ko = preset_data.get("name_ko", preset_id)
	var desc_ko = preset_data.get("description_ko", "")

	btn.text = "%s\n%s" % [name_ko, desc_ko]
	btn.add_theme_font_size_override("font_size", 16)
	btn.tooltip_text = desc_ko

	# Set background color based on preset type
	var style = StyleBoxFlat.new()
	match preset_id:
		"HighPressing":
			style.bg_color = Color(1.0, 0.2, 0.2, 0.3)  # Red - Aggressive
		"Counterattack":
			style.bg_color = Color(1.0, 0.6, 0.0, 0.3)  # Orange - Fast
		"Possession":
			style.bg_color = Color(0.2, 0.6, 1.0, 0.3)  # Blue - Patient
		"Balanced":
			style.bg_color = Color(0.3, 0.8, 0.3, 0.3)  # Green - Balanced
		"Defensive":
			style.bg_color = Color(0.5, 0.5, 0.9, 0.3)  # Purple - Defensive

	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	btn.add_theme_stylebox_override("normal", style)

	btn.pressed.connect(_on_preset_button_pressed.bind(preset_id))

	return btn


func _create_offside_trap_toggle() -> Control:
	"""Create offside trap toggle"""
	var container = HBoxContainer.new()
	container.add_theme_constant_override("separation", 15)

	# Checkbox
	offside_trap_checkbox = CheckButton.new()
	offside_trap_checkbox.text = "오프사이드 트랩 사용"
	offside_trap_checkbox.add_theme_font_size_override("font_size", 18)
	offside_trap_checkbox.toggled.connect(_on_offside_trap_toggled)
	container.add_child(offside_trap_checkbox)

	# Info label
	var info_label = Label.new()
	info_label.text = "(수비 라인이 High/VeryHigh일 때 효과적)"
	info_label.add_theme_font_size_override("font_size", 14)
	info_label.add_theme_color_override("font_color", Color(0.6, 0.6, 0.6))
	container.add_child(info_label)

	return container


func _on_offside_trap_toggled(pressed: bool):
	"""Handle offside trap toggle"""
	use_offside_trap = pressed
	print("[TeamInstructionsPanel] Offside trap: %s" % ("ON" if pressed else "OFF"))

	# Check if defensive line is high enough
	if pressed and instruction_selectors.has("defensive_line"):
		var selector = instruction_selectors["defensive_line"]
		var selected_index = selector.selected
		if selected_index >= 0:
			var value = selector.get_item_metadata(selected_index)
			if value not in ["High", "VeryHigh"]:
				_show_message("⚠️ 오프사이드 트랩은 수비 라인이 High 또는 VeryHigh일 때 가장 효과적입니다.")


func _create_instruction_selector(category: String) -> Control:
	"""Create instruction selector for a category"""
	var container = VBoxContainer.new()
	container.add_theme_constant_override("separation", 8)

	# Get category info
	var korean_label = available_options.get("korean_labels", {}).get(category, category)
	var options = available_options.get("instruction_options", {}).get(category, [])
	var korean_values = available_options.get("korean_values", {}).get(category, {})

	# Category label
	var label = Label.new()
	label.text = korean_label
	label.add_theme_font_size_override("font_size", 18)
	container.add_child(label)

	# Selector
	var selector = OptionButton.new()
	selector.custom_minimum_size = Vector2(400, 50)
	selector.add_theme_font_size_override("font_size", 16)

	# Add options with Korean labels
	for option in options:
		var korean_value = korean_values.get(option, option)
		selector.add_item(korean_value)
		selector.set_item_metadata(selector.get_item_count() - 1, option)

	selector.item_selected.connect(_on_instruction_changed.bind(category))
	instruction_selectors[category] = selector
	container.add_child(selector)

	return container


func _create_modifier_preview_section() -> Control:
	"""Create modifier preview section"""
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 10)

	# Title
	var title = Label.new()
	title.text = "전술 효과 미리보기 (Modifier Preview)"
	title.add_theme_font_size_override("font_size", 20)
	section.add_child(title)

	# Preview grid
	var preview_grid = GridContainer.new()
	preview_grid.columns = 2
	preview_grid.add_theme_constant_override("h_separation", 30)
	preview_grid.add_theme_constant_override("v_separation", 8)
	section.add_child(preview_grid)

	# Create preview labels (will be updated when instructions change)
	var preview_items = [
		{"key": "defensive_line_numeric", "label": "수비 라인 높이:"},
		{"key": "defensive_line_positioning", "label": "포지셔닝 요구:"},
		{"key": "defensive_line_pace", "label": "속도 요구:"},
		{"key": "team_width_numeric", "label": "팀 폭:"},
		{"key": "team_width_crossing", "label": "크로스 보너스:"},
		{"key": "team_tempo_numeric", "label": "템포:"},
		{"key": "team_tempo_stamina", "label": "체력 소모:"},
		{"key": "pressing_numeric", "label": "압박 강도:"},
		{"key": "pressing_work_rate", "label": "활동량 요구:"},
		{"key": "pressing_stamina", "label": "압박 체력비용:"},
		{"key": "buildup_numeric", "label": "빌드업 스타일:"},
		{"key": "buildup_passing", "label": "패싱 보너스:"}
	]

	for item in preview_items:
		var label_text = Label.new()
		label_text.text = item.label
		label_text.add_theme_font_size_override("font_size", 14)
		preview_grid.add_child(label_text)

		var label_value = Label.new()
		label_value.text = "-"
		label_value.add_theme_font_size_override("font_size", 14)
		label_value.add_theme_color_override("font_color", Color(0.8, 0.9, 1.0))
		label_value.name = "Modifier_" + item.key
		modifier_labels[item.key] = label_value
		preview_grid.add_child(label_value)

	return section


func _show_error_ui():
	"""Show error message when Rust engine not available"""
	var error_label = Label.new()
	error_label.text = "❌ FootballRustEngine을 찾을 수 없습니다.\nRust GDExtension이 로드되었는지 확인해주세요."
	error_label.add_theme_font_size_override("font_size", 20)
	error_label.add_theme_color_override("font_color", Color(1, 0.3, 0.3))
	error_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	error_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	error_label.set_anchors_preset(Control.PRESET_FULL_RECT)
	error_label.anchor_right = 1.0
	error_label.anchor_bottom = 1.0
	add_child(error_label)


# ==============================================================================
# Data Loading Functions
# ==============================================================================


func _load_instruction_options():
	"""Load instruction options from Rust API"""
	if not rust_engine:
		return

	print("[TeamInstructionsPanel] Loading instruction options...")
	var result = rust_engine.get_team_instruction_options()
	if result.get("success", false):
		available_options = result.get("options", {})
		print("[TeamInstructionsPanel] Loaded instruction options: %s" % available_options.keys())
	else:
		print("[TeamInstructionsPanel] Failed to load instruction options: %s" % result.get("error", "Unknown"))
		available_options = {}


func _load_tactical_presets():
	"""Load tactical presets from Rust API"""
	if not rust_engine:
		return

	print("[TeamInstructionsPanel] Loading tactical presets...")
	var result = rust_engine.get_tactical_presets()
	if result.get("success", false):
		tactical_presets = result.get("presets", [])
		print("[TeamInstructionsPanel] Loaded %d tactical presets" % tactical_presets.size())
	else:
		print("[TeamInstructionsPanel] Failed to load tactical presets: %s" % result.get("error", "Unknown"))
		# Fallback presets
		tactical_presets = [
			{"id": "Balanced", "name_ko": "균형", "description_ko": "공격과 수비의 균형"},
			{"id": "HighPressing", "name_ko": "고압축", "description_ko": "적극적인 압박과 공격"},
			{"id": "Counterattack", "name_ko": "역습", "description_ko": "빠른 역습과 카운터"},
			{"id": "Possession", "name_ko": "점유", "description_ko": "볼 점유율 중심의 전술"},
			{"id": "Defensive", "name_ko": "수비", "description_ko": "안정적인 수비 중심"}
		]


# ==============================================================================
# Signal Handlers
# ==============================================================================


func _on_preset_button_pressed(preset_id: String):
	"""Handle preset button press"""
	print("[TeamInstructionsPanel] Preset selected: %s" % preset_id)
	_apply_preset(preset_id)


func _on_instruction_changed(category: String, index: int):
	"""Handle instruction selector change"""
	if not instruction_selectors.has(category):
		return

	var selector = instruction_selectors[category]
	var value = selector.get_item_metadata(index)
	print("[TeamInstructionsPanel] Instruction changed - %s: %s" % [category, value])

	# Update preview (don't apply yet - wait for Apply button)
	_update_modifier_preview()


func _on_apply_custom_instructions():
	"""Apply custom instructions from selectors"""
	if not rust_engine:
		_show_message("Rust 엔진을 찾을 수 없습니다.")
		return

	# Collect current instruction values
	var instructions = {}
	for category in instruction_selectors.keys():
		var selector = instruction_selectors[category]
		var selected_index = selector.selected
		if selected_index >= 0:
			var value = selector.get_item_metadata(selected_index)
			instructions[category] = value

	# Validate all categories are set
	var required = ["defensive_line", "team_width", "team_tempo", "pressing_intensity", "build_up_style"]
	for category in required:
		if not instructions.has(category):
			_show_message("모든 지시사항을 설정해주세요.")
			return

	# Add offside trap setting
	instructions["use_offside_trap"] = use_offside_trap

	print("[TeamInstructionsPanel] Applying custom instructions: %s" % instructions)

	# Call Rust API
	var result = rust_engine.set_team_instructions_custom(instructions)
	if not result.get("success", false):
		_show_message("지시사항 적용 실패: %s" % result.get("error", "Unknown"))
		return

	current_instructions = instructions
	_update_modifier_preview_from_result(result)
	_show_message("커스텀 지시사항이 적용되었습니다!")
	instructions_applied.emit(instructions)


func _apply_preset(preset_id: String):
	"""Apply a tactical preset"""
	if not rust_engine:
		return

	print("[TeamInstructionsPanel] Applying preset: %s" % preset_id)

	# Call Rust API
	var result = rust_engine.set_team_instructions_preset(preset_id)
	if not result.get("success", false):
		_show_message("프리셋 적용 실패: %s" % result.get("error", "Unknown"))
		return

	# Update UI from preset data
	var preset_info = result.get("preset", {})
	var instructions = result.get("instructions", {})

	# Ensure preset_info is a Dictionary, not a String
	if typeof(preset_info) != TYPE_DICTIONARY:
		print("[TeamInstructionsPanel] Warning: preset_info is not a Dictionary, type: ", typeof(preset_info))
		preset_info = {}

	# Ensure instructions is a Dictionary
	if typeof(instructions) != TYPE_DICTIONARY:
		print("[TeamInstructionsPanel] Warning: instructions is not a Dictionary, type: ", typeof(instructions))
		instructions = {}

	# Update selectors to match preset
	for category in instructions.keys():
		if instruction_selectors.has(category):
			var selector = instruction_selectors[category]
			var value = instructions[category]

			# Find matching item
			for i in range(selector.get_item_count()):
				if selector.get_item_metadata(i) == value:
					selector.select(i)
					break

	current_instructions = instructions
	_update_modifier_preview_from_result(result)

	var preset_name = preset_info.get("name_ko", preset_id)
	_show_message("'%s' 프리셋이 적용되었습니다!" % preset_name)
	preset_applied.emit(preset_id)


# ==============================================================================
# Helper Functions
# ==============================================================================


func _update_modifier_preview():
	"""Update modifier preview (without applying)"""
	# Collect current values for preview
	var instructions = {}
	for category in instruction_selectors.keys():
		var selector = instruction_selectors[category]
		var selected_index = selector.selected
		if selected_index >= 0:
			instructions[category] = selector.get_item_metadata(selected_index)

	# Call API to get modifiers (but don't apply)
	if rust_engine and instructions.size() == 5:
		var result = rust_engine.set_team_instructions_custom(instructions)
		if result.get("success", false):
			_update_modifier_preview_from_result(result)


func _update_modifier_preview_from_result(result: Dictionary):
	"""Update modifier preview labels from API result"""
	var modifiers = result.get("modifiers", {})

	# Defensive line modifiers
	var def_line = modifiers.get("defensive_line", {})
	_update_label("defensive_line_numeric", "%+d" % def_line.get("numeric", 0))
	_update_label("defensive_line_positioning", "%+d" % def_line.get("positioning_mod", 0))
	_update_label("defensive_line_pace", "%+d" % def_line.get("pace_requirement", 0))

	# Team width modifiers
	var width = modifiers.get("team_width", {})
	_update_label("team_width_numeric", "%+d" % width.get("numeric", 0))
	_update_label("team_width_crossing", "%+d" % width.get("crossing_mod", 0))

	# Team tempo modifiers
	var tempo = modifiers.get("team_tempo", {})
	_update_label("team_tempo_numeric", "%+d" % tempo.get("numeric", 0))
	_update_label("team_tempo_stamina", "%.1fx" % tempo.get("stamina_drain", 1.0))

	# Pressing modifiers
	var pressing = modifiers.get("pressing_intensity", {})
	_update_label("pressing_numeric", "%+d" % pressing.get("numeric", 0))
	_update_label("pressing_work_rate", "%+d" % pressing.get("work_rate_mod", 0))
	_update_label("pressing_stamina", "%.2fx" % pressing.get("stamina_cost", 1.0))

	# Build-up style modifiers
	var buildup = modifiers.get("build_up_style", {})
	_update_label("buildup_numeric", "%+d" % buildup.get("numeric", 0))
	_update_label("buildup_passing", "%+d" % buildup.get("passing_mod", 0))


func _update_label(key: String, value: String):
	"""Update a modifier label"""
	if modifier_labels.has(key):
		modifier_labels[key].text = value


func _show_message(text: String):
	"""Show a simple message dialog"""
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "팀 지시사항"
	add_child(popup)
	popup.popup_centered(Vector2(450, 200))
	popup.confirmed.connect(popup.queue_free)


# ==============================================================================
# Public API
# ==============================================================================


func get_current_instructions() -> Dictionary:
	"""Get current team instructions"""
	return current_instructions.duplicate(true)


func reload_data():
	"""Reload instruction options and presets from API"""
	_load_instruction_options()
	_load_tactical_presets()
