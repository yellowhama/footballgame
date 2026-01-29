extends PanelContainer
class_name FilterPanel
## Filter panel UI component
## Phase 13: Extended Features - Search/Filter System

signal filter_changed(criteria: FilterCriteria)
signal filter_reset

## Current filter criteria
var current_criteria: FilterCriteria = FilterCriteria.new()

## UI References
var search_input: LineEdit
var type_checkboxes: Dictionary = {}
var result_checkboxes: Dictionary = {}
var date_from_input: LineEdit
var date_to_input: LineEdit
var min_gain_spinbox: SpinBox
var max_gain_spinbox: SpinBox
var sort_field_option: OptionButton
var sort_order_option: OptionButton
var apply_button: EnhancedButton
var reset_button: EnhancedButton
var filter_count_label: Label

## Panel visibility toggle
var _panel_visible: bool = false


func _ready():
	_build_ui()
	_connect_signals()
	_update_filter_count_label()


## Build the complete filter panel UI
func _build_ui():
	# Main container
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 16)
	margin.add_theme_constant_override("margin_right", 16)
	margin.add_theme_constant_override("margin_top", 12)
	margin.add_theme_constant_override("margin_bottom", 12)
	add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 12)
	margin.add_child(vbox)

	# Title with filter count
	var title_hbox = HBoxContainer.new()
	vbox.add_child(title_hbox)

	var title = Label.new()
	title.text = "필터"
	title.add_theme_font_size_override("font_size", 20)
	title_hbox.add_child(title)

	title_hbox.add_child(Control.new())  # Spacer
	title_hbox.get_child(-1).size_flags_horizontal = Control.SIZE_EXPAND_FILL

	filter_count_label = Label.new()
	filter_count_label.text = ""
	filter_count_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	title_hbox.add_child(filter_count_label)

	# Search input
	var search_section = _create_section("검색", vbox)
	search_input = LineEdit.new()
	search_input.placeholder_text = "기록 검색..."
	search_input.custom_minimum_size = Vector2(0, 44)
	search_section.add_child(search_input)

	# Match type checkboxes
	var type_section = _create_section("경기 유형", vbox)
	var type_grid = GridContainer.new()
	type_grid.columns = 2
	type_grid.add_theme_constant_override("h_separation", 16)
	type_grid.add_theme_constant_override("v_separation", 8)
	type_section.add_child(type_grid)

	type_checkboxes["friendly"] = _create_checkbox("친선경기", true, type_grid)
	type_checkboxes["league"] = _create_checkbox("리그전", true, type_grid)
	type_checkboxes["cup"] = _create_checkbox("컵대회", true, type_grid)
	type_checkboxes["training"] = _create_checkbox("훈련", true, type_grid)

	# Result checkboxes
	var result_section = _create_section("결과", vbox)
	var result_grid = GridContainer.new()
	result_grid.columns = 3
	result_grid.add_theme_constant_override("h_separation", 16)
	result_section.add_child(result_grid)

	result_checkboxes["win"] = _create_checkbox("승", true, result_grid)
	result_checkboxes["draw"] = _create_checkbox("무", true, result_grid)
	result_checkboxes["loss"] = _create_checkbox("패", true, result_grid)

	# Date range
	var date_section = _create_section("날짜 범위", vbox)
	var date_grid = GridContainer.new()
	date_grid.columns = 2
	date_grid.add_theme_constant_override("h_separation", 8)
	date_section.add_child(date_grid)

	var from_label = Label.new()
	from_label.text = "시작:"
	date_grid.add_child(from_label)

	date_from_input = LineEdit.new()
	date_from_input.placeholder_text = "YYYY-MM-DD"
	date_from_input.custom_minimum_size = Vector2(0, 40)
	date_grid.add_child(date_from_input)

	var to_label = Label.new()
	to_label.text = "종료:"
	date_grid.add_child(to_label)

	date_to_input = LineEdit.new()
	date_to_input.placeholder_text = "YYYY-MM-DD"
	date_to_input.custom_minimum_size = Vector2(0, 40)
	date_grid.add_child(date_to_input)

	# Attribute gain range (for training records)
	var gain_section = _create_section("능력치 변화 범위", vbox)
	var gain_grid = GridContainer.new()
	gain_grid.columns = 2
	gain_grid.add_theme_constant_override("h_separation", 8)
	gain_section.add_child(gain_grid)

	var min_label = Label.new()
	min_label.text = "최소:"
	gain_grid.add_child(min_label)

	min_gain_spinbox = SpinBox.new()
	min_gain_spinbox.min_value = 0.0
	min_gain_spinbox.max_value = 100.0
	min_gain_spinbox.step = 0.1
	min_gain_spinbox.value = 0.0
	min_gain_spinbox.custom_minimum_size = Vector2(0, 40)
	gain_grid.add_child(min_gain_spinbox)

	var max_label = Label.new()
	max_label.text = "최대:"
	gain_grid.add_child(max_label)

	max_gain_spinbox = SpinBox.new()
	max_gain_spinbox.min_value = 0.0
	max_gain_spinbox.max_value = 100.0
	max_gain_spinbox.step = 0.1
	max_gain_spinbox.value = 100.0
	max_gain_spinbox.custom_minimum_size = Vector2(0, 40)
	gain_grid.add_child(max_gain_spinbox)

	# Sort options
	var sort_section = _create_section("정렬", vbox)
	var sort_grid = GridContainer.new()
	sort_grid.columns = 2
	sort_grid.add_theme_constant_override("h_separation", 8)
	sort_section.add_child(sort_grid)

	var field_label = Label.new()
	field_label.text = "기준:"
	sort_grid.add_child(field_label)

	sort_field_option = OptionButton.new()
	sort_field_option.add_item("날짜", FilterCriteria.SortField.DATE)
	sort_field_option.add_item("결과", FilterCriteria.SortField.RESULT)
	sort_field_option.add_item("득실차", FilterCriteria.SortField.SCORE_DIFF)
	sort_field_option.add_item("능력치 변화", FilterCriteria.SortField.ATTRIBUTE_GAIN)
	sort_field_option.selected = FilterCriteria.SortField.DATE
	sort_field_option.custom_minimum_size = Vector2(0, 40)
	sort_grid.add_child(sort_field_option)

	var order_label = Label.new()
	order_label.text = "순서:"
	sort_grid.add_child(order_label)

	sort_order_option = OptionButton.new()
	sort_order_option.add_item("오름차순", FilterCriteria.SortOrder.ASCENDING)
	sort_order_option.add_item("내림차순", FilterCriteria.SortOrder.DESCENDING)
	sort_order_option.selected = FilterCriteria.SortOrder.DESCENDING
	sort_order_option.custom_minimum_size = Vector2(0, 40)
	sort_grid.add_child(sort_order_option)

	# Action buttons
	var button_hbox = HBoxContainer.new()
	button_hbox.add_theme_constant_override("separation", 8)
	vbox.add_child(button_hbox)

	reset_button = EnhancedButton.new()
	reset_button.text = "초기화"
	reset_button.custom_minimum_size = Vector2(0, 48)
	reset_button.tooltip_text = "입력값을 초기 상태로 되돌립니다"
	button_hbox.add_child(reset_button)

	apply_button = EnhancedButton.new()
	apply_button.text = "적용"
	apply_button.custom_minimum_size = Vector2(0, 48)
	apply_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	apply_button.tooltip_text = "필터를 적용하여 리스트를 업데이트합니다"
	button_hbox.add_child(apply_button)


## Create a section with title
func _create_section(title: String, parent: VBoxContainer) -> VBoxContainer:
	var section_vbox = VBoxContainer.new()
	section_vbox.add_theme_constant_override("separation", 8)
	parent.add_child(section_vbox)

	var section_title = Label.new()
	section_title.text = title
	section_title.add_theme_font_size_override("font_size", 16)
	section_title.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	section_vbox.add_child(section_title)

	return section_vbox


## Create a checkbox with label
func _create_checkbox(label_text: String, checked: bool, parent: GridContainer) -> CheckBox:
	var checkbox = CheckBox.new()
	checkbox.text = label_text
	checkbox.button_pressed = checked
	checkbox.custom_minimum_size = Vector2(0, 40)
	parent.add_child(checkbox)
	return checkbox


## Connect all UI signals
func _connect_signals():
	if search_input:
		search_input.text_changed.connect(_on_search_text_changed)

	if apply_button:
		apply_button.pressed.connect(_on_apply_pressed)

	if reset_button:
		reset_button.pressed.connect(_on_reset_pressed)


## Update filter count label
func _update_filter_count_label():
	if not filter_count_label:
		return

	if current_criteria.has_active_filters():
		var count = _count_active_filters()
		filter_count_label.text = "(%d개 활성)" % count
		filter_count_label.add_theme_color_override("font_color", Color(0.3, 0.7, 1.0))
	else:
		filter_count_label.text = ""


## Count number of active filters
func _count_active_filters() -> int:
	var count = 0
	if current_criteria.search_text != "":
		count += 1
	if not (
		current_criteria.include_friendly
		and current_criteria.include_league
		and current_criteria.include_cup
		and current_criteria.include_training
	):
		count += 1
	if not (current_criteria.include_win and current_criteria.include_draw and current_criteria.include_loss):
		count += 1
	if current_criteria.date_from != "" or current_criteria.date_to != "":
		count += 1
	if current_criteria.min_attribute_gain > 0.0 or current_criteria.max_attribute_gain < 100.0:
		count += 1
	return count


## Apply button pressed
func _on_apply_pressed():
	_update_criteria_from_ui()
	_update_filter_count_label()
	filter_changed.emit(current_criteria)


## Reset button pressed
func _on_reset_pressed():
	current_criteria.reset()
	_update_ui_from_criteria()
	_update_filter_count_label()
	filter_reset.emit()


## Search text changed (instant update)
func _on_search_text_changed(new_text: String):
	current_criteria.search_text = new_text
	_update_filter_count_label()


## Update criteria from UI values
func _update_criteria_from_ui():
	current_criteria.search_text = search_input.text if search_input else ""
	current_criteria.include_friendly = (
		type_checkboxes.get("friendly", null).button_pressed if type_checkboxes.has("friendly") else true
	)
	current_criteria.include_league = (
		type_checkboxes.get("league", null).button_pressed if type_checkboxes.has("league") else true
	)
	current_criteria.include_cup = (
		type_checkboxes.get("cup", null).button_pressed if type_checkboxes.has("cup") else true
	)
	current_criteria.include_training = (
		type_checkboxes.get("training", null).button_pressed if type_checkboxes.has("training") else true
	)
	current_criteria.include_win = (
		result_checkboxes.get("win", null).button_pressed if result_checkboxes.has("win") else true
	)
	current_criteria.include_draw = (
		result_checkboxes.get("draw", null).button_pressed if result_checkboxes.has("draw") else true
	)
	current_criteria.include_loss = (
		result_checkboxes.get("loss", null).button_pressed if result_checkboxes.has("loss") else true
	)
	current_criteria.date_from = date_from_input.text if date_from_input else ""
	current_criteria.date_to = date_to_input.text if date_to_input else ""
	current_criteria.min_attribute_gain = min_gain_spinbox.value if min_gain_spinbox else 0.0
	current_criteria.max_attribute_gain = max_gain_spinbox.value if max_gain_spinbox else 100.0
	current_criteria.sort_field = (
		sort_field_option.get_selected_id() if sort_field_option else FilterCriteria.SortField.DATE
	)
	current_criteria.sort_order = (
		sort_order_option.get_selected_id() if sort_order_option else FilterCriteria.SortOrder.DESCENDING
	)


## Update UI from criteria values
func _update_ui_from_criteria():
	if search_input:
		search_input.text = current_criteria.search_text

	if type_checkboxes.has("friendly"):
		type_checkboxes["friendly"].button_pressed = current_criteria.include_friendly
	if type_checkboxes.has("league"):
		type_checkboxes["league"].button_pressed = current_criteria.include_league
	if type_checkboxes.has("cup"):
		type_checkboxes["cup"].button_pressed = current_criteria.include_cup
	if type_checkboxes.has("training"):
		type_checkboxes["training"].button_pressed = current_criteria.include_training

	if result_checkboxes.has("win"):
		result_checkboxes["win"].button_pressed = current_criteria.include_win
	if result_checkboxes.has("draw"):
		result_checkboxes["draw"].button_pressed = current_criteria.include_draw
	if result_checkboxes.has("loss"):
		result_checkboxes["loss"].button_pressed = current_criteria.include_loss

	if date_from_input:
		date_from_input.text = current_criteria.date_from
	if date_to_input:
		date_to_input.text = current_criteria.date_to

	if min_gain_spinbox:
		min_gain_spinbox.value = current_criteria.min_attribute_gain
	if max_gain_spinbox:
		max_gain_spinbox.value = current_criteria.max_attribute_gain

	if sort_field_option:
		sort_field_option.selected = current_criteria.sort_field
	if sort_order_option:
		sort_order_option.selected = current_criteria.sort_order


## Get current filter criteria
func get_criteria() -> FilterCriteria:
	_update_criteria_from_ui()
	return current_criteria.duplicate_criteria()


## Set filter criteria (programmatically)
func set_criteria(criteria: FilterCriteria):
	current_criteria = criteria.duplicate_criteria()
	_update_ui_from_criteria()
	_update_filter_count_label()


## Toggle panel visibility with animation (Phase 12 integration)
func toggle_visibility():
	_panel_visible = not _panel_visible

	if _panel_visible:
		show()
		modulate.a = 0.0
		var tween = create_tween()
		tween.set_ease(Tween.EASE_OUT)
		tween.set_trans(Tween.TRANS_CUBIC)
		tween.tween_property(self, "modulate:a", 1.0, 0.3)
	else:
		var tween = create_tween()
		tween.set_ease(Tween.EASE_OUT)
		tween.set_trans(Tween.TRANS_CUBIC)
		tween.tween_property(self, "modulate:a", 0.0, 0.3)
		tween.tween_callback(hide)
