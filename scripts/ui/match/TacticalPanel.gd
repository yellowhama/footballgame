extends Control
class_name TacticalPanel

signal tactical_applied(payload: Dictionary)
signal cancelled

const DEFAULT_FORMATIONS := ["4-4-2", "4-3-3", "3-5-2", "5-3-2"]

@onready var _backdrop: ColorRect = $Backdrop
@onready var _panel: Panel = $Panel
@onready var _formation_selector: OptionButton = $Panel/VBox/FormSection/FormationOption
@onready var _attack_slider: HSlider = $Panel/VBox/SliderSection/AttackRow/AttackSlider
@onready var _attack_value: Label = $Panel/VBox/SliderSection/AttackRow/ValueLabel
@onready var _press_slider: HSlider = $Panel/VBox/SliderSection/PressRow/PressSlider
@onready var _press_value: Label = $Panel/VBox/SliderSection/PressRow/ValueLabel
@onready var _tempo_selector: OptionButton = $Panel/VBox/TempoSection/TempoOption
@onready var _apply_button: Button = $Panel/VBox/ButtonRow/ApplyButton
@onready var _cancel_button: Button = $Panel/VBox/ButtonRow/CancelButton

# Additional tactical controls (added dynamically)
var _width_slider: HSlider = null
var _width_value: Label = null
var _buildup_selector: OptionButton = null

var _current_payload: Dictionary = {}


func _ready() -> void:
	visible = false
	mouse_filter = Control.MOUSE_FILTER_STOP
	if _backdrop:
		_backdrop.mouse_filter = Control.MOUSE_FILTER_STOP

	_attack_slider.value_changed.connect(_on_attack_changed)
	_press_slider.value_changed.connect(_on_press_changed)
	_apply_button.pressed.connect(_on_apply_pressed)
	_cancel_button.pressed.connect(_on_cancel_pressed)

	_setup_tempo_selector()
	_setup_additional_controls()
	_populate_formations(DEFAULT_FORMATIONS)
	_refresh_slider_labels()


func _setup_additional_controls() -> void:
	# Find SliderSection to add new controls
	var slider_section = $Panel/VBox/SliderSection
	if not slider_section:
		return

	# Add Width slider row
	var width_row = HBoxContainer.new()
	width_row.name = "WidthRow"

	var width_label = Label.new()
	width_label.text = "팀 폭:"
	width_label.custom_minimum_size.x = 100
	width_row.add_child(width_label)

	_width_slider = HSlider.new()
	_width_slider.min_value = 0
	_width_slider.max_value = 100
	_width_slider.value = 50
	_width_slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_width_slider.value_changed.connect(_on_width_changed)
	width_row.add_child(_width_slider)

	_width_value = Label.new()
	_width_value.text = "50"
	_width_value.custom_minimum_size.x = 40
	_width_value.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	width_row.add_child(_width_value)

	slider_section.add_child(width_row)

	# Add Build-up style selector after tempo
	var tempo_section = $Panel/VBox/TempoSection
	if tempo_section:
		var buildup_row = HBoxContainer.new()
		buildup_row.name = "BuildupRow"

		var buildup_label = Label.new()
		buildup_label.text = "빌드업:"
		buildup_label.custom_minimum_size.x = 100
		buildup_row.add_child(buildup_label)

		_buildup_selector = OptionButton.new()
		_buildup_selector.add_item("숏 패스", 0)
		_buildup_selector.add_item("혼합", 1)
		_buildup_selector.add_item("직접 플레이", 2)
		_buildup_selector.selected = 1
		_buildup_selector.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		buildup_row.add_child(_buildup_selector)

		tempo_section.get_parent().add_child(buildup_row)
		tempo_section.get_parent().move_child(buildup_row, tempo_section.get_index() + 1)


func _on_width_changed(_value: float) -> void:
	if _width_value:
		_width_value.text = "%d" % int(round(_width_slider.value))


func show_with_state(formations: Array, payload: Dictionary) -> void:
	if formations and formations.size() > 0:
		_populate_formations(formations)
	else:
		_populate_formations(DEFAULT_FORMATIONS)

	_current_payload = payload.duplicate(true)
	_set_from_payload(_current_payload)
	visible = true
	_panel.grab_focus()


func hide_panel() -> void:
	visible = false


func _setup_tempo_selector() -> void:
	_tempo_selector.clear()
	_tempo_selector.add_item("느림", 0)
	_tempo_selector.add_item("보통", 1)
	_tempo_selector.add_item("빠름", 2)
	_tempo_selector.selected = 1


func _populate_formations(formations: Array) -> void:
	_formation_selector.clear()
	for formation in formations:
		_formation_selector.add_item(str(formation))
	if _formation_selector.item_count > 0:
		_formation_selector.selected = 0


func _set_from_payload(payload: Dictionary) -> void:
	var formation := str(payload.get("formation", _formation_selector.get_item_text(_formation_selector.selected)))
	for i in _formation_selector.item_count:
		if _formation_selector.get_item_text(i) == formation:
			_formation_selector.selected = i
			break

	_attack_slider.value = clampf(float(payload.get("attack_bias", 0.5)) * 100.0, 0.0, 100.0)
	_press_slider.value = clampf(float(payload.get("press_intensity", 0.5)) * 100.0, 0.0, 100.0)

	# Set width slider
	if _width_slider:
		_width_slider.value = clampf(float(payload.get("team_width", 0.5)) * 100.0, 0.0, 100.0)

	_refresh_slider_labels()

	var tempo := str(payload.get("tempo", "normal")).to_lower()
	match tempo:
		"slow":
			_tempo_selector.selected = 0
		"fast":
			_tempo_selector.selected = 2
		_:
			_tempo_selector.selected = 1

	# Set build-up style
	if _buildup_selector:
		var buildup := str(payload.get("build_up_style", "Mixed")).to_lower()
		match buildup:
			"short":
				_buildup_selector.selected = 0
			"direct":
				_buildup_selector.selected = 2
			_:
				_buildup_selector.selected = 1


func _refresh_slider_labels() -> void:
	_attack_value.text = "%d" % int(round(_attack_slider.value))
	_press_value.text = "%d" % int(round(_press_slider.value))


func _on_attack_changed(_value: float) -> void:
	_refresh_slider_labels()


func _on_press_changed(_value: float) -> void:
	_refresh_slider_labels()


func _on_apply_pressed() -> void:
	var payload := _build_payload()
	tactical_applied.emit(payload)
	hide_panel()


func _on_cancel_pressed() -> void:
	cancelled.emit()
	hide_panel()


func _build_payload() -> Dictionary:
	var formation := _formation_selector.get_item_text(_formation_selector.selected)
	var tempo_index := _tempo_selector.selected
	var tempo := "normal"
	match tempo_index:
		0:
			tempo = "slow"
		2:
			tempo = "fast"

	# Get build-up style
	var buildup := "Mixed"
	if _buildup_selector:
		match _buildup_selector.selected:
			0:
				buildup = "Short"
			2:
				buildup = "Direct"

	# Get team width
	var team_width := 0.5
	if _width_slider:
		team_width = clampf(_width_slider.value / 100.0, 0.0, 1.0)

	return {
		"formation": formation,
		"attack_bias": clampf(_attack_slider.value / 100.0, 0.0, 1.0),
		"press_intensity": clampf(_press_slider.value / 100.0, 0.0, 1.0),
		"tempo": tempo,
		"team_width": team_width,
		"build_up_style": buildup
	}


func _unhandled_input(event: InputEvent) -> void:
	if not visible:
		return
	if event.is_action_pressed("ui_cancel"):
		_on_cancel_pressed()
