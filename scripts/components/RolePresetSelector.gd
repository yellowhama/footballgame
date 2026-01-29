extends Control

## RolePresetSelector - Role-Based Instruction Preset Selector
## Provides quick preset buttons for common tactical roles

signal preset_selected(preset_id: String)

var player_position: String = ""
var current_slot: int = -1
var preset_buttons: Array = []


func _ready():
	pass


func setup(position: String, slot: int):
	"""Initialize with player position and slot"""
	player_position = position
	current_slot = slot
	_build_ui()


func _build_ui():
	"""Build preset selector UI"""
	# Clear existing children
	for child in get_children():
		child.queue_free()

	# Main container
	var main_vbox = VBoxContainer.new()
	main_vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(main_vbox)

	# Title
	var title = Label.new()
	title.text = "역할 프리셋"
	title.add_theme_font_size_override("font_size", 20)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	main_vbox.add_child(title)

	# Spacer
	var spacer1 = Control.new()
	spacer1.custom_minimum_size = Vector2(0, 10)
	main_vbox.add_child(spacer1)

	# Get available presets from MyTeamData
	if not has_node("/root/MyTeamData"):
		_show_error("MyTeamData not found")
		return

	var my_team_data = get_node("/root/MyTeamData")
	var presets = my_team_data.get_role_instruction_presets_by_position(player_position)

	if presets.size() == 0:
		_show_error("No presets available for position: %s" % player_position)
		return

	# Preset buttons grid
	var scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	main_vbox.add_child(scroll)

	var grid = VBoxContainer.new()
	grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(grid)

	# Create button for each preset
	preset_buttons.clear()
	for preset in presets:
		var btn_container = _create_preset_button(preset)
		grid.add_child(btn_container)
		preset_buttons.append(btn_container)


func _create_preset_button(preset: Dictionary) -> Control:
	"""Create a preset button with name and description"""
	var container = VBoxContainer.new()
	container.custom_minimum_size = Vector2(0, 100)

	# Button
	var btn = Button.new()
	btn.text = preset.name_ko
	btn.custom_minimum_size = Vector2(0, 60)
	btn.pressed.connect(_on_preset_button_pressed.bind(preset.id))
	container.add_child(btn)

	# Style button
	btn.add_theme_font_size_override("font_size", 24)

	# Description
	var desc_label = Label.new()
	desc_label.text = preset.description
	desc_label.add_theme_font_size_override("font_size", 18)
	desc_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	desc_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	desc_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	container.add_child(desc_label)

	# Spacer
	var spacer = Control.new()
	spacer.custom_minimum_size = Vector2(0, 10)
	container.add_child(spacer)

	return container


func _show_error(message: String):
	"""Show error message"""
	var label = Label.new()
	label.text = message
	label.add_theme_font_size_override("font_size", 18)
	label.add_theme_color_override("font_color", Color(1, 0.3, 0.3))
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	add_child(label)


func _on_preset_button_pressed(preset_id: String):
	"""Handle preset button press"""
	print("[RolePresetSelector] Preset selected: %s for slot %d" % [preset_id, current_slot])

	if not has_node("/root/MyTeamData"):
		print("[RolePresetSelector] Error: MyTeamData not found")
		return

	var my_team_data = get_node("/root/MyTeamData")

	# Apply preset
	var success = my_team_data.apply_role_instruction_preset(current_slot, preset_id)

	if success:
		print("[RolePresetSelector] ✅ Preset applied successfully")
		preset_selected.emit(preset_id)
	else:
		print("[RolePresetSelector] ❌ Failed to apply preset")
