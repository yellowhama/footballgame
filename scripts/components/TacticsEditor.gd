extends Control

## TacticsEditor - Team Tactics Configuration Component
## Provides UI for editing team tactical parameters with presets and custom sliders

signal tactics_changed(parameters: Dictionary)
signal preset_applied(preset_name: String)

# UI Components
var preset_buttons: Dictionary = {}
var parameter_sliders: Dictionary = {}
var parameter_labels: Dictionary = {}

# Current tactics
var current_tactics: Dictionary = {}

# Parameter info (Korean labels and descriptions)
const PARAMETER_INFO = {
	"attacking_intensity": {"label": "공격 강도", "description": "공격 시 얼마나 적극적으로 전진하는가", "low": "보수적", "high": "공격적"},
	"defensive_line_height": {"label": "수비 라인", "description": "수비 라인의 위치 (높을수록 전방)", "low": "낮은 라인", "high": "높은 라인"},
	"width": {"label": "팀 폭", "description": "선수들이 얼마나 넓게 퍼지는가", "low": "좁게", "high": "넓게"},
	"pressing_trigger": {"label": "압박 시점", "description": "언제 압박을 시작하는가", "low": "낮은 압박", "high": "높은 압박"},
	"tempo": {"label": "템포", "description": "플레이 속도", "low": "느린 템포", "high": "빠른 템포"},
	"directness": {"label": "직접성", "description": "패싱 스타일 (직접적 vs 빌드업)", "low": "빌드업", "high": "직접적"}
}


func _ready():
	print("[TacticsEditor] Initializing tactics editor")
	_build_ui()
	_load_current_tactics()


func _build_ui():
	"""Build the tactics editor UI"""
	# Main container
	var main_vbox = VBoxContainer.new()
	main_vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_vbox.anchor_right = 1.0
	main_vbox.anchor_bottom = 1.0
	add_child(main_vbox)

	# Title
	var title = Label.new()
	title.text = "팀 전술 설정"
	title.add_theme_font_size_override("font_size", 24)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	main_vbox.add_child(title)

	# Spacer
	var spacer1 = Control.new()
	spacer1.custom_minimum_size = Vector2(0, 20)
	main_vbox.add_child(spacer1)

	# Preset buttons section
	var preset_section = _create_preset_section()
	main_vbox.add_child(preset_section)

	# Spacer
	var spacer2 = Control.new()
	spacer2.custom_minimum_size = Vector2(0, 30)
	main_vbox.add_child(spacer2)

	# Parameter sliders section
	var sliders_scroll = ScrollContainer.new()
	sliders_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	main_vbox.add_child(sliders_scroll)

	var sliders_vbox = VBoxContainer.new()
	sliders_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	sliders_scroll.add_child(sliders_vbox)

	# Create sliders for each parameter
	var param_order = [
		"attacking_intensity", "defensive_line_height", "width", "pressing_trigger", "tempo", "directness"
	]

	for param_name in param_order:
		var slider_container = _create_parameter_slider(param_name)
		sliders_vbox.add_child(slider_container)

	# Tactics preview section
	var preview_section = _create_preview_section()
	main_vbox.add_child(preview_section)


func _create_preset_section() -> Control:
	"""Create preset buttons section"""
	var section = VBoxContainer.new()

	# Built-in presets section
	var builtin_label = Label.new()
	builtin_label.text = "빠른 설정 프리셋"
	builtin_label.add_theme_font_size_override("font_size", 18)
	section.add_child(builtin_label)

	var spacer1 = Control.new()
	spacer1.custom_minimum_size = Vector2(0, 5)
	section.add_child(spacer1)

	var builtin_grid = GridContainer.new()
	builtin_grid.name = "BuiltinPresetsGrid"
	builtin_grid.columns = 3
	builtin_grid.add_theme_constant_override("h_separation", 10)
	builtin_grid.add_theme_constant_override("v_separation", 10)
	section.add_child(builtin_grid)

	# Get presets from MyTeamData
	if has_node("/root/MyTeamData"):
		var my_team_data = get_node("/root/MyTeamData")
		var presets = my_team_data.get_available_presets()

		# Add only built-in presets
		for preset_info in presets:
			if not preset_info.is_custom:
				var btn = Button.new()
				btn.text = preset_info.name_ko
				btn.custom_minimum_size = Vector2(150, 50)
				btn.tooltip_text = preset_info.description
				btn.pressed.connect(_on_preset_button_pressed.bind(preset_info.id))
				preset_buttons[preset_info.id] = btn
				builtin_grid.add_child(btn)

	# Custom slots section
	var spacer2 = Control.new()
	spacer2.custom_minimum_size = Vector2(0, 20)
	section.add_child(spacer2)

	var custom_header = HBoxContainer.new()
	section.add_child(custom_header)

	var custom_label = Label.new()
	custom_label.text = "커스텀 전술 슬롯"
	custom_label.add_theme_font_size_override("font_size", 18)
	custom_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	custom_header.add_child(custom_label)

	# Save button
	var save_custom_btn = Button.new()
	save_custom_btn.text = "💾 현재 전술 저장"
	save_custom_btn.custom_minimum_size = Vector2(140, 35)
	save_custom_btn.add_theme_font_size_override("font_size", 14)
	save_custom_btn.pressed.connect(_on_save_custom_preset)
	custom_header.add_child(save_custom_btn)

	var spacer3 = Control.new()
	spacer3.custom_minimum_size = Vector2(0, 10)
	section.add_child(spacer3)

	# Custom slots grid
	var custom_grid = GridContainer.new()
	custom_grid.name = "CustomSlotsGrid"
	custom_grid.columns = 2
	custom_grid.add_theme_constant_override("h_separation", 10)
	custom_grid.add_theme_constant_override("v_separation", 10)
	section.add_child(custom_grid)

	# Create 5 custom slots
	for i in range(5):
		var slot_id = "custom_slot_%d" % (i + 1)
		_create_custom_slot(custom_grid, slot_id, i + 1)

	return section


func _create_custom_slot(parent: GridContainer, slot_id: String, slot_number: int):
	"""Create a custom tactic slot"""
	var my_team_data = get_node_or_null("/root/MyTeamData")
	if not my_team_data:
		return

	# Check if slot has saved data
	var presets = my_team_data.get_available_presets()
	var slot_preset = null
	for preset in presets:
		if preset.is_custom and preset.id == slot_id:
			slot_preset = preset
			break

	var slot_container = VBoxContainer.new()
	slot_container.add_theme_constant_override("separation", 4)

	# Main slot button
	var slot_btn = Button.new()
	slot_btn.custom_minimum_size = Vector2(200, 50)

	if slot_preset:
		slot_btn.text = "⭐ %s" % slot_preset.get("name_ko", "슬롯 %d" % slot_number)
		slot_btn.tooltip_text = slot_preset.get("description", "")
		slot_btn.pressed.connect(_on_preset_button_pressed.bind(slot_id))
	else:
		slot_btn.text = "[ 비어있음 - 슬롯 %d ]" % slot_number
		slot_btn.disabled = true
		slot_btn.modulate = Color(0.6, 0.6, 0.6)

	preset_buttons[slot_id] = slot_btn
	slot_container.add_child(slot_btn)

	# Action buttons (rename/clear)
	if slot_preset:
		var action_hbox = HBoxContainer.new()
		action_hbox.add_theme_constant_override("separation", 6)

		var rename_btn = Button.new()
		rename_btn.text = "✏️ 이름변경"
		rename_btn.custom_minimum_size = Vector2(90, 28)
		rename_btn.add_theme_font_size_override("font_size", 12)
		rename_btn.pressed.connect(_on_rename_slot.bind(slot_id))
		action_hbox.add_child(rename_btn)

		var clear_btn = Button.new()
		clear_btn.text = "🗑️ 비우기"
		clear_btn.custom_minimum_size = Vector2(90, 28)
		clear_btn.add_theme_font_size_override("font_size", 12)
		clear_btn.pressed.connect(_on_clear_slot.bind(slot_id))
		action_hbox.add_child(clear_btn)

		slot_container.add_child(action_hbox)

	parent.add_child(slot_container)


func _create_parameter_slider(param_name: String) -> Control:
	"""Create a slider for a tactical parameter"""
	var container = VBoxContainer.new()
	container.custom_minimum_size = Vector2(0, 80)

	var info = PARAMETER_INFO[param_name]

	# Parameter name label
	var name_label = Label.new()
	name_label.text = info.label
	name_label.add_theme_font_size_override("font_size", 16)
	container.add_child(name_label)

	# Slider with value label
	var slider_hbox = HBoxContainer.new()
	container.add_child(slider_hbox)

	var slider = HSlider.new()
	slider.min_value = 0.0
	slider.max_value = 1.0
	slider.step = 0.05
	slider.value = 0.5
	slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	slider.value_changed.connect(_on_slider_changed.bind(param_name))
	slider_hbox.add_child(slider)

	var value_label = Label.new()
	value_label.text = "0.50"
	value_label.custom_minimum_size = Vector2(50, 0)
	slider_hbox.add_child(value_label)

	parameter_sliders[param_name] = slider
	parameter_labels[param_name] = value_label

	# Description label
	var desc_label = Label.new()
	desc_label.text = "%s  ←  %s  →  %s" % [info.low, info.description, info.high]
	desc_label.add_theme_font_size_override("font_size", 12)
	desc_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	container.add_child(desc_label)

	return container


func _create_preview_section() -> Control:
	"""Create tactics preview section"""
	var section = VBoxContainer.new()
	section.custom_minimum_size = Vector2(0, 100)

	var label = Label.new()
	label.text = "전술 효과 미리보기"
	label.add_theme_font_size_override("font_size", 18)
	section.add_child(label)

	var preview_grid = GridContainer.new()
	preview_grid.columns = 2
	preview_grid.add_theme_constant_override("h_separation", 20)
	section.add_child(preview_grid)

	# Attack bias
	var attack_label = Label.new()
	attack_label.text = "공격 편향: 50%"
	attack_label.name = "AttackBiasLabel"
	preview_grid.add_child(attack_label)

	# Defense bias
	var defense_label = Label.new()
	defense_label.text = "수비 편향: 50%"
	defense_label.name = "DefenseBiasLabel"
	preview_grid.add_child(defense_label)

	return section


func _load_current_tactics():
	"""Load current tactics from MyTeamData"""
	if not has_node("/root/MyTeamData"):
		return

	var my_team_data = get_node("/root/MyTeamData")
	current_tactics = my_team_data.get_team_tactics()

	# Update sliders
	var parameters = current_tactics.get("parameters", {})
	for param_name in parameters.keys():
		if parameter_sliders.has(param_name):
			var slider = parameter_sliders[param_name]
			slider.value = parameters[param_name]
			_update_slider_label(param_name, parameters[param_name])

	# Highlight current preset
	var current_preset = current_tactics.get("preset", "")
	for preset_name in preset_buttons.keys():
		var btn = preset_buttons[preset_name]
		if preset_name == current_preset:
			btn.add_theme_color_override("font_color", Color(0, 1, 0))
		else:
			btn.remove_theme_color_override("font_color")

	_update_preview()


func _update_slider_label(param_name: String, value: float):
	"""Update slider value label"""
	if parameter_labels.has(param_name):
		parameter_labels[param_name].text = "%.2f" % value


func _update_preview():
	"""Update tactics preview"""
	var parameters = {}
	for param_name in parameter_sliders.keys():
		parameters[param_name] = parameter_sliders[param_name].value

	# Calculate attack and defense bias
	var attacking = parameters.get("attacking_intensity", 0.5)
	var defensive_line = parameters.get("defensive_line_height", 0.5)
	var tempo = parameters.get("tempo", 0.5)

	var attack_bias = (attacking + defensive_line + tempo) / 3.0
	var defense_bias = 1.0 - attack_bias

	# Update preview labels
	var attack_label = get_node_or_null("AttackBiasLabel")
	if attack_label:
		attack_label.text = "공격 편향: %.0f%%" % (attack_bias * 100)
		if attack_bias > 0.6:
			attack_label.add_theme_color_override("font_color", Color(1, 0.3, 0.3))
		else:
			attack_label.remove_theme_color_override("font_color")

	var defense_label = get_node_or_null("DefenseBiasLabel")
	if defense_label:
		defense_label.text = "수비 편향: %.0f%%" % (defense_bias * 100)
		if defense_bias > 0.6:
			defense_label.add_theme_color_override("font_color", Color(0.3, 0.3, 1))
		else:
			defense_label.remove_theme_color_override("font_color")


# ==============================================================================
# Signal Handlers
# ==============================================================================


func _on_preset_button_pressed(preset_name: String):
	"""Handle preset button press"""
	print("[TacticsEditor] Preset button pressed: %s" % preset_name)

	if not has_node("/root/MyTeamData"):
		return

	var my_team_data = get_node("/root/MyTeamData")
	if my_team_data.set_team_tactics_preset(preset_name):
		_load_current_tactics()
		preset_applied.emit(preset_name)


func _on_slider_changed(value: float, param_name: String):
	"""Handle slider value change"""
	_update_slider_label(param_name, value)
	_update_preview()

	# Collect all parameters
	var parameters = {}
	for pname in parameter_sliders.keys():
		parameters[pname] = parameter_sliders[pname].value

	# Emit signal
	tactics_changed.emit(parameters)

	# Apply to MyTeamData
	if has_node("/root/MyTeamData"):
		var my_team_data = get_node("/root/MyTeamData")
		my_team_data.set_team_tactics_custom(parameters)


func _on_save_custom_preset():
	"""Save current slider values to a chosen slot"""
	print("[TacticsEditor] Save to custom slot requested")

	# Get current parameters from sliders
	var parameters = {}
	for param_name in parameter_sliders.keys():
		parameters[param_name] = parameter_sliders[param_name].value

	# Show slot selection dialog
	var dialog = AcceptDialog.new()
	dialog.title = "커스텀 전술 저장"
	dialog.dialog_text = "저장할 슬롯을 선택하세요:"
	dialog.min_size = Vector2(400, 300)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 8)
	dialog.add_child(vbox)

	# Slot selection buttons
	var my_team_data = get_node_or_null("/root/MyTeamData")
	var selected_slot = ""

	for i in range(5):
		var slot_id = "custom_slot_%d" % (i + 1)
		var slot_btn = Button.new()
		slot_btn.custom_minimum_size = Vector2(350, 40)
		slot_btn.add_theme_font_size_override("font_size", 14)

		# Check if slot is occupied
		var presets = my_team_data.get_available_presets() if my_team_data else []
		var slot_preset = null
		for preset in presets:
			if preset.is_custom and preset.id == slot_id:
				slot_preset = preset
				break

		if slot_preset:
			slot_btn.text = "슬롯 %d: %s (덮어쓰기)" % [i + 1, slot_preset.get("name_ko", "")]
		else:
			slot_btn.text = "슬롯 %d: 비어있음" % (i + 1)

		slot_btn.pressed.connect(
			func():
				selected_slot = slot_id
				dialog.hide()
				_show_save_details_dialog(slot_id, parameters, slot_preset != null)
		)

		vbox.add_child(slot_btn)

	add_child(dialog)
	dialog.popup_centered()
	dialog.canceled.connect(dialog.queue_free)


func _show_save_details_dialog(slot_id: String, parameters: Dictionary, is_overwrite: bool):
	"""Show dialog to enter name and description for the slot"""
	var dialog = AcceptDialog.new()
	dialog.title = "전술 이름 입력"
	dialog.dialog_text = "전술 이름과 설명을 입력하세요:"
	dialog.min_size = Vector2(400, 250)

	# Name input
	var name_label = Label.new()
	name_label.text = "이름:"
	dialog.add_child(name_label)

	var line_edit = LineEdit.new()
	line_edit.placeholder_text = "예: 나만의 공격 전술"
	line_edit.custom_minimum_size = Vector2(350, 40)
	dialog.add_child(line_edit)

	# Description input
	var desc_label = Label.new()
	desc_label.text = "설명 (선택사항):"
	dialog.add_child(desc_label)

	var desc_edit = TextEdit.new()
	desc_edit.placeholder_text = "전술에 대한 간단한 설명"
	desc_edit.custom_minimum_size = Vector2(350, 60)
	dialog.add_child(desc_edit)

	add_child(dialog)
	dialog.popup_centered()

	dialog.confirmed.connect(
		func():
			var preset_name = line_edit.text.strip_edges()
			var description = desc_edit.text.strip_edges()

			if preset_name == "":
				_show_message("전술 이름을 입력해주세요.")
				dialog.queue_free()
				return

			if has_node("/root/MyTeamData"):
				var my_team_data = get_node("/root/MyTeamData")
				# Use slot_id as the preset ID for fixed slots
				if my_team_data.save_custom_preset(slot_id, parameters, description, preset_name):
					if is_overwrite:
						_show_message("슬롯이 '%s'(으)로 덮어씌워졌습니다!" % preset_name)
					else:
						_show_message("슬롯에 '%s'이(가) 저장되었습니다!" % preset_name)
					_reload_presets()
				else:
					_show_message("전술 저장에 실패했습니다.")

			dialog.queue_free()
	)

	dialog.canceled.connect(dialog.queue_free)


func _on_rename_slot(slot_id: String):
	"""Rename a custom slot"""
	print("[TacticsEditor] Rename slot: %s" % slot_id)

	# Get current slot data
	var my_team_data = get_node_or_null("/root/MyTeamData")
	if not my_team_data:
		return

	var presets = my_team_data.get_available_presets()
	var slot_preset = null
	for preset in presets:
		if preset.is_custom and preset.id == slot_id:
			slot_preset = preset
			break

	if not slot_preset:
		_show_message("슬롯 데이터를 찾을 수 없습니다.")
		return

	var dialog = AcceptDialog.new()
	dialog.title = "슬롯 이름 변경"
	dialog.dialog_text = "새로운 이름을 입력하세요:"
	dialog.min_size = Vector2(400, 150)

	var line_edit = LineEdit.new()
	line_edit.text = slot_preset.get("name_ko", "")
	line_edit.custom_minimum_size = Vector2(350, 40)
	dialog.add_child(line_edit)

	add_child(dialog)
	dialog.popup_centered()

	dialog.confirmed.connect(
		func():
			var new_name = line_edit.text.strip_edges()
			if new_name == "":
				_show_message("이름을 입력해주세요.")
				dialog.queue_free()
				return

			if my_team_data.rename_custom_preset(slot_id, new_name):
				_show_message("슬롯 이름이 '%s'(으)로 변경되었습니다!" % new_name)
				_reload_presets()
			else:
				_show_message("이름 변경에 실패했습니다.")

			dialog.queue_free()
	)

	dialog.canceled.connect(dialog.queue_free)


func _on_clear_slot(slot_id: String):
	"""Clear a custom slot"""
	print("[TacticsEditor] Clear slot: %s" % slot_id)

	var dialog = ConfirmationDialog.new()
	dialog.title = "슬롯 비우기"
	dialog.dialog_text = "정말 이 슬롯을 비우시겠습니까?\n저장된 전술이 삭제됩니다."
	dialog.min_size = Vector2(350, 120)

	add_child(dialog)
	dialog.popup_centered()

	dialog.confirmed.connect(
		func():
			if has_node("/root/MyTeamData"):
				var my_team_data = get_node("/root/MyTeamData")
				if my_team_data.delete_custom_preset(slot_id):
					_show_message("슬롯이 비워졌습니다.")
					_reload_presets()
				else:
					_show_message("슬롯 비우기에 실패했습니다.")

			dialog.queue_free()
	)

	dialog.canceled.connect(dialog.queue_free)


func _reload_presets():
	"""Reload preset buttons"""
	# Find and remove old preset grid
	var old_grid = get_node_or_null("PresetsGrid")
	if old_grid and old_grid.get_parent():
		var parent = old_grid.get_parent()
		old_grid.queue_free()

		# Recreate preset section
		preset_buttons.clear()
		var new_section = _create_preset_section()

		# Replace old section with new one
		# This is a bit hacky, but works for now
		# Ideally we'd rebuild the whole UI, but this is simpler
		queue_free()
		get_parent()._add_team_tactics_tab()


func _show_message(text: String):
	"""Show a simple message dialog"""
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "팀 전술"
	add_child(popup)
	popup.popup_centered(Vector2(350, 150))
	popup.confirmed.connect(popup.queue_free)


# ==============================================================================
# Public API
# ==============================================================================


func get_current_tactics() -> Dictionary:
	"""Get current tactics from sliders"""
	var parameters = {}
	for param_name in parameter_sliders.keys():
		parameters[param_name] = parameter_sliders[param_name].value
	return {"preset": current_tactics.get("preset", "Custom"), "parameters": parameters}


func set_tactics(tactics: Dictionary):
	"""Set tactics (updates sliders)"""
	current_tactics = tactics.duplicate(true)
	_load_current_tactics()
