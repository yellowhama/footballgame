extends Control
class_name PlayerInstructionsScreen

# Player Instructions configuration screen
# - Role preset selection (filtered by position)
# - Custom instruction tuning (8 categories)
# - Attribute comparison (base vs modified)

signal instructions_applied(player_data: Dictionary, instructions: Dictionary)
signal screen_closed

# UI References
var _title_label: Label
var _player_info_label: Label
var _role_container: HBoxContainer
var _scroll_container: ScrollContainer
var _instructions_container: VBoxContainer
var _attributes_container: VBoxContainer
var _action_buttons: HBoxContainer

# Instruction selectors
var _instruction_selectors: Dictionary = {}

# Data
var _current_player: Dictionary = {}
var _available_roles: Array = []
var _current_instructions: Dictionary = {}
var _engine: Node = null

# Constants
const SCREEN_WIDTH = 1080
const SCREEN_HEIGHT = 1920
const SAFE_MARGIN = 60
const ROLE_BUTTON_SIZE = Vector2(300, 120)
const INSTRUCTION_HEIGHT = 160
const BUTTON_SIZE = Vector2(300, 100)


func _ready():
	# Get engine reference
	if has_node("/root/FootballRustEngine"):
		_engine = get_node("/root/FootballRustEngine")
	else:
		push_error("FootballRustEngine not found!")
		return

	_create_ui()
	_apply_styles()
	_connect_signals()


func _create_ui():
	# Main HSplit layout - Left: Player Info, Right: Settings
	var main_hsplit = HSplitContainer.new()
	main_hsplit.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_hsplit.split_offset = 400  # Left panel width
	add_child(main_hsplit)

	# === Left Panel: Player Info + Action Buttons ===
	var left_panel = VBoxContainer.new()
	left_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	left_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
	main_hsplit.add_child(left_panel)

	# Header in left panel
	var header = _create_header()
	left_panel.add_child(header)

	# Attributes in left panel
	var attributes_section = _create_attributes_section()
	attributes_section.size_flags_vertical = Control.SIZE_EXPAND_FILL
	left_panel.add_child(attributes_section)

	# Action buttons at bottom of left panel
	var actions_section = _create_actions_section()
	left_panel.add_child(actions_section)

	# === Right Panel: All Settings (Scrollable) ===
	var right_scroll = ScrollContainer.new()
	right_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	right_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	main_hsplit.add_child(right_scroll)

	var right_panel = VBoxContainer.new()
	right_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	right_scroll.add_child(right_panel)

	# Role Preset Section
	var role_section = _create_role_section()
	right_panel.add_child(role_section)

	# Role Instruction Preset Section
	var role_instruction_section = _create_role_instruction_preset_section()
	right_panel.add_child(role_instruction_section)

	# Instructions Section
	var instructions_section = _create_instructions_section()
	right_panel.add_child(instructions_section)


func _create_header() -> VBoxContainer:
	var header = VBoxContainer.new()
	header.custom_minimum_size = Vector2(SCREEN_WIDTH, 150)

	# Title
	_title_label = Label.new()
	_title_label.text = "⚙️ 선수 인스트럭션 설정"
	_title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_title_label.add_theme_font_size_override("font_size", 40)
	header.add_child(_title_label)

	# Player info
	_player_info_label = Label.new()
	_player_info_label.text = "선수를 선택하세요"
	_player_info_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_player_info_label.add_theme_font_size_override("font_size", 28)
	header.add_child(_player_info_label)

	return header


func _create_role_section() -> VBoxContainer:
	var section = VBoxContainer.new()
	section.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Section title
	var title = Label.new()
	title.text = "📋 롤 프리셋"
	title.add_theme_font_size_override("font_size", 24)
	section.add_child(title)

	# Scrollable horizontal container for role buttons
	var scroll = ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(0, 140)
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	section.add_child(scroll)

	_role_container = HBoxContainer.new()
	_role_container.add_theme_constant_override("separation", 20)
	scroll.add_child(_role_container)

	return section


func _create_role_instruction_preset_section() -> VBoxContainer:
	var section = VBoxContainer.new()
	section.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Section title
	var title = Label.new()
	title.text = "⚡ 빠른 전술 프리셋"
	title.add_theme_font_size_override("font_size", 24)
	section.add_child(title)

	# Container for preset buttons (no scroll since parent already scrolls)
	var preset_container = VBoxContainer.new()
	preset_container.name = "RoleInstructionPresetContainer"
	preset_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	section.add_child(preset_container)

	return section


func _create_instructions_section() -> VBoxContainer:
	var section = VBoxContainer.new()
	section.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Section title
	var title = Label.new()
	title.text = "🎯 세부 인스트럭션 조정"
	title.add_theme_font_size_override("font_size", 32)
	section.add_child(title)

	# Direct container for instructions (no internal scroll)
	_instructions_container = VBoxContainer.new()
	_instructions_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_instructions_container.add_theme_constant_override("separation", 20)
	section.add_child(_instructions_container)

	return section


func _create_attributes_section() -> VBoxContainer:
	var section = VBoxContainer.new()
	section.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	section.size_flags_vertical = Control.SIZE_EXPAND_FILL

	# Section title
	var title = Label.new()
	title.text = "📊 속성 변화"
	title.add_theme_font_size_override("font_size", 24)
	section.add_child(title)

	# Direct attributes container (no internal scroll)
	_attributes_container = VBoxContainer.new()
	_attributes_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	section.add_child(_attributes_container)

	return section


func _create_actions_section() -> VBoxContainer:
	var section = VBoxContainer.new()
	section.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	section.add_theme_constant_override("separation", 10)

	# Apply button
	var apply_btn = Button.new()
	apply_btn.text = "✅ 적용"
	apply_btn.custom_minimum_size = Vector2(0, 60)
	apply_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	apply_btn.pressed.connect(_on_apply_pressed)
	section.add_child(apply_btn)

	# Reset button
	var reset_btn = Button.new()
	reset_btn.text = "🔄 초기화"
	reset_btn.custom_minimum_size = Vector2(0, 60)
	reset_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	reset_btn.pressed.connect(_on_reset_pressed)
	section.add_child(reset_btn)

	# Cancel button
	var cancel_btn = Button.new()
	cancel_btn.text = "❌ 취소"
	cancel_btn.custom_minimum_size = Vector2(0, 60)
	cancel_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	cancel_btn.pressed.connect(_on_cancel_pressed)
	section.add_child(cancel_btn)

	return section


func _apply_styles():
	# Background
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.1, 0.1, 0.15, 1.0)
	add_theme_stylebox_override("panel", style)


func _connect_signals():
	pass


# ===== Public API =====


func load_player(player_data: Dictionary):
	"""Load player data and populate UI"""
	if not _engine:
		push_error("Engine not available")
		return

	_current_player = player_data

	# Update player info
	var player_name = player_data.get("name", "Unknown")
	var position = player_data.get("position", "")
	var ca = player_data.get("ca", 0)
	_player_info_label.text = "%s (%s) - CA %d" % [player_name, position, ca]

	# Load available roles for position
	_load_available_roles(position)

	# Load role instruction presets for position
	_load_role_instruction_presets(position)

	# Load instruction options and create selectors
	_create_instruction_selectors()

	# Load current instructions if any
	if player_data.has("instructions") and player_data.instructions != null:
		_current_instructions = player_data.instructions
		_apply_instructions_to_ui(_current_instructions)
	else:
		# Default instructions
		_current_instructions = _get_default_instructions()
		_apply_instructions_to_ui(_current_instructions)

	# Update attributes comparison
	_update_attributes_comparison()


func _load_available_roles(position: String):
	"""Load available roles filtered by position"""
	if not _engine or not _engine.has_method("get_available_roles"):
		push_error("get_available_roles not available")
		return

	# FootballRustEngine returns Dictionary, not String
	var result = _engine.get_available_roles(position)

	if result and result.has("success") and result.success:
		_available_roles = result.get("roles", [])
		_populate_role_buttons()
	else:
		push_error("Failed to load roles: " + str(result.get("error", "Unknown")))


func _populate_role_buttons():
	"""Create role preset buttons"""
	# Clear existing buttons
	for child in _role_container.get_children():
		child.queue_free()

	# Create button for each role
	for role in _available_roles:
		var btn = Button.new()
		btn.text = role.get("name_ko", "Unknown")
		btn.custom_minimum_size = ROLE_BUTTON_SIZE
		btn.tooltip_text = role.get("description_ko", "")
		btn.pressed.connect(_on_role_selected.bind(role))
		_role_container.add_child(btn)

		# Style
		btn.add_theme_font_size_override("font_size", 28)


func _load_role_instruction_presets(position: String):
	"""Load role instruction presets from MyTeamData"""
	if not has_node("/root/MyTeamData"):
		print("[PlayerInstructionsScreen] MyTeamData not found")
		return

	var my_team_data = get_node("/root/MyTeamData")
	var presets = my_team_data.get_role_instruction_presets_by_position(position)

	_populate_role_instruction_preset_buttons(presets)


func _populate_role_instruction_preset_buttons(presets: Array):
	"""Create role instruction preset buttons"""
	# Find container - updated path for new HSplit layout
	var container_node = get_node_or_null(
		"HSplitContainer/ScrollContainer/VBoxContainer/VBoxContainer2/RoleInstructionPresetContainer"
	)
	if not container_node:
		print("[PlayerInstructionsScreen] RoleInstructionPresetContainer not found, trying alternative path")
		# Try to find by name recursively
		container_node = _find_node_by_name(self, "RoleInstructionPresetContainer")
		if not container_node:
			print("[PlayerInstructionsScreen] RoleInstructionPresetContainer not found anywhere")
			return

	# Clear existing buttons
	for child in container_node.get_children():
		child.queue_free()

	if presets.size() == 0:
		var no_presets_label = Label.new()
		no_presets_label.text = "사용 가능한 프리셋 없음"
		no_presets_label.add_theme_font_size_override("font_size", 24)
		no_presets_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		container_node.add_child(no_presets_label)
		return

	# Create grid for preset buttons
	var grid = GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 20)
	grid.add_theme_constant_override("v_separation", 15)
	container_node.add_child(grid)

	# Create button for each preset
	for preset in presets:
		var btn_vbox = VBoxContainer.new()

		var btn = Button.new()
		btn.text = preset.name_ko
		btn.custom_minimum_size = Vector2(480, 70)
		btn.pressed.connect(_on_role_instruction_preset_selected.bind(preset))
		btn.add_theme_font_size_override("font_size", 26)
		btn_vbox.add_child(btn)

		# Description label
		var desc = Label.new()
		desc.text = preset.description
		desc.add_theme_font_size_override("font_size", 18)
		desc.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
		desc.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		btn_vbox.add_child(desc)

		grid.add_child(btn_vbox)


func _create_instruction_selectors():
	"""Create 8 instruction selector widgets"""
	if not _engine or not _engine.has_method("get_instruction_options"):
		push_error("get_instruction_options not available")
		return

	# FootballRustEngine returns Dictionary, not String
	var result = _engine.get_instruction_options()

	if not result or not result.has("success") or not result.success:
		push_error("Failed to load instruction options: " + str(result.get("error", "Unknown")))
		return

	var options = result.get("instruction_options", {})
	var labels = result.get("korean_labels", {})

	# Clear existing selectors
	for child in _instructions_container.get_children():
		child.queue_free()
	_instruction_selectors.clear()

	# Create selector for each instruction category
	var categories = ["mentality", "width", "depth", "passing", "dribbling", "shooting", "defensive_work", "pressing"]

	for category in categories:
		if options.has(category):
			var selector = InstructionSelector.new()
			selector.instruction_name = category
			selector.instruction_label_ko = labels.get(category, category)
			# Convert to typed array
			var typed_options: Array[String] = []
			for opt in options[category]:
				typed_options.append(opt)
			selector.options = typed_options
			selector.custom_minimum_size = Vector2(SCREEN_WIDTH - 40, INSTRUCTION_HEIGHT)
			selector.instruction_changed.connect(_on_instruction_changed)

			_instructions_container.add_child(selector)
			_instruction_selectors[category] = selector


func _apply_instructions_to_ui(instructions: Dictionary):
	"""Apply instruction values to UI selectors"""
	for category in instructions.keys():
		if _instruction_selectors.has(category):
			var selector = _instruction_selectors[category]
			selector.set_value(instructions[category])


func _get_default_instructions() -> Dictionary:
	"""Get default instruction values"""
	return {
		"mentality": "Balanced",
		"width": "Normal",
		"depth": "Balanced",
		"passing": "Mixed",
		"dribbling": "Normal",
		"shooting": "Normal",
		"defensive_work": "Normal",
		"pressing": "Medium"
	}


func _update_attributes_comparison():
	"""Update attributes comparison section"""
	if not _engine or not _engine.has_method("get_player_modified_attributes"):
		return

	# Clear existing
	for child in _attributes_container.get_children():
		child.queue_free()

	# Get modified attributes - pass Dictionary directly
	# FootballRustEngine expects Dictionary, not String
	var result = _engine.get_player_modified_attributes(_current_player)

	if not result or not result.has("success") or not result.success:
		var error_label = Label.new()
		error_label.text = "속성 비교 불가: " + str(result.get("error", "Unknown"))
		_attributes_container.add_child(error_label)
		return

	var has_instructions = result.get("has_instructions", false)
	var base_attrs = result.get("base_attributes", {})
	var modified_attrs = result.get("modified_attributes", {})

	if not has_instructions:
		var info_label = Label.new()
		info_label.text = "인스트럭션이 설정되지 않음"
		info_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		_attributes_container.add_child(info_label)
		return

	# Show changed attributes
	var changes_found = false
	for attr_name in base_attrs.keys():
		var base_val = base_attrs[attr_name]
		var modified_val = modified_attrs.get(attr_name, base_val)

		if base_val != modified_val:
			changes_found = true
			var change_row = HBoxContainer.new()

			var name_label = Label.new()
			name_label.text = _get_korean_attribute_name(attr_name)
			name_label.custom_minimum_size = Vector2(300, 0)
			change_row.add_child(name_label)

			var base_label = Label.new()
			base_label.text = str(base_val)
			base_label.custom_minimum_size = Vector2(100, 0)
			base_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			change_row.add_child(base_label)

			var arrow_label = Label.new()
			arrow_label.text = "→"
			arrow_label.custom_minimum_size = Vector2(80, 0)
			arrow_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			change_row.add_child(arrow_label)

			var modified_label = Label.new()
			modified_label.text = str(modified_val)
			modified_label.custom_minimum_size = Vector2(100, 0)
			modified_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			# Color based on change
			if modified_val > base_val:
				modified_label.add_theme_color_override("font_color", Color(0.2, 1.0, 0.2))
			else:
				modified_label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3))
			change_row.add_child(modified_label)

			var diff_label = Label.new()
			var diff = modified_val - base_val
			diff_label.text = "%+d" % diff
			diff_label.custom_minimum_size = Vector2(100, 0)
			diff_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			if diff > 0:
				diff_label.add_theme_color_override("font_color", Color(0.2, 1.0, 0.2))
			else:
				diff_label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3))
			change_row.add_child(diff_label)

			_attributes_container.add_child(change_row)

	if not changes_found:
		var no_change_label = Label.new()
		no_change_label.text = "속성 변화 없음"
		no_change_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		_attributes_container.add_child(no_change_label)


func _get_korean_attribute_name(attr_name: String) -> String:
	"""Get Korean name for attribute"""
	var names = {
		"off_the_ball": "오프 더 볼",
		"positioning": "포지셔닝",
		"work_rate": "활동량",
		"concentration": "집중력",
		"crossing": "크로스",
		"dribbling": "드리블",
		"long_shots": "중거리 슛",
		"anticipation": "예측력",
		"first_touch": "퍼스트 터치",
		"passing": "패싱",
		"vision": "시야",
		"tackling": "태클",
		"marking": "마킹",
		"finishing": "마무리",
		"flair": "재능",
		"technique": "기술",
		"composure": "침착성",
		"decisions": "판단력",
		"aggression": "공격성",
		"stamina": "스태미나"
	}
	return names.get(attr_name, attr_name)


# ===== Signal Handlers =====


func _on_role_selected(role: Dictionary):
	"""Handle role preset selection"""
	print("Role selected: ", role.get("name_ko", ""))

	# Extract preset instructions
	var preset = role.get("preset_instructions", {})
	_current_instructions = preset.duplicate()

	# Update UI
	_apply_instructions_to_ui(_current_instructions)

	# Update player with role
	if _engine and _engine.has_method("set_player_role"):
		var role_id = role.get("role_id", "")
		# FootballRustEngine expects Dictionary directly
		var result = _engine.set_player_role(_current_player, role_id)

		if result and result.has("success") and result.success:
			_current_player = result.get("player", {})
			_update_attributes_comparison()
		else:
			push_error("Failed to set role: " + str(result.get("error", "")))


func _on_role_instruction_preset_selected(preset: Dictionary):
	"""Handle role instruction preset selection"""
	print("[PlayerInstructionsScreen] Role instruction preset selected: ", preset.name_ko)

	# Extract instructions from preset
	var instructions = preset.instructions.duplicate()
	_current_instructions = instructions

	# Update UI
	_apply_instructions_to_ui(_current_instructions)

	# Update player with instructions via engine
	if _engine and _engine.has_method("set_player_instructions"):
		# FootballRustEngine expects Dictionaries directly
		var result = _engine.set_player_instructions(_current_player, _current_instructions)

		if result and result.has("success") and result.success:
			_current_player = result.get("player", {})
			_update_attributes_comparison()
			print("[PlayerInstructionsScreen] ✅ Role instruction preset applied successfully")
		else:
			push_error("Failed to set instructions: " + str(result.get("error", "")))


func _on_instruction_changed(instruction_name: String, new_value: String):
	"""Handle individual instruction change"""
	_current_instructions[instruction_name] = new_value

	# Update player with new instructions
	if _engine and _engine.has_method("set_player_instructions"):
		# FootballRustEngine expects Dictionaries directly
		var result = _engine.set_player_instructions(_current_player, _current_instructions)

		if result and result.has("success") and result.success:
			_current_player = result.get("player", {})
			_update_attributes_comparison()
		else:
			push_error("Failed to set instructions: " + str(result.get("error", "")))


func _on_apply_pressed():
	"""Apply instructions and close screen"""
	print("Apply instructions")
	instructions_applied.emit(_current_player, _current_instructions)
	screen_closed.emit()


func _on_reset_pressed():
	"""Reset to default instructions"""
	print("Reset instructions")

	if _engine and _engine.has_method("clear_player_instructions"):
		# FootballRustEngine expects Dictionary directly
		var result = _engine.clear_player_instructions(_current_player)

		if result and result.has("success") and result.success:
			_current_player = result.get("player", {})
			_current_instructions = _get_default_instructions()
			_apply_instructions_to_ui(_current_instructions)
			_update_attributes_comparison()
		else:
			push_error("Failed to clear instructions: " + str(result.get("error", "")))


func _on_cancel_pressed():
	"""Cancel and close screen"""
	print("Cancel")
	screen_closed.emit()


# ==============================================================================
# Helper Functions
# ==============================================================================


func _find_node_by_name(root: Node, node_name: String) -> Node:
	"""Recursively find a node by name"""
	if root.name == node_name:
		return root

	for child in root.get_children():
		var result = _find_node_by_name(child, node_name)
		if result:
			return result

	return null
