extends "res://scenes/academy/base/AdaptiveLayoutContainer.gd"
## Responsive Training Screen - Phase 7B Implementation
## Cross-platform UI with 3 layout variants (Mobile/Tablet/Desktop)

# Signals
signal training_executed(training_id: String, result: Dictionary)

const DECK_EDITOR_DIALOG_PATH := "res://scripts/ui/DeckEditorDialog.gd"
const DECK_BUILD_SCREEN_PATH := "res://scenes/screens/DeckBuilderScreen.tscn"
const TrainingEventPayloadClass := preload("res://scripts/utils/TrainingEventPayload.gd")

const TRAINING_MODE_CONFIG := [
	{"id": "team", "label_key": "UI_TRAINING_MODE_TEAM"},
	{"id": "personal", "label_key": "UI_TRAINING_MODE_PERSONAL"},
	{"id": "special", "label_key": "UI_TRAINING_MODE_SPECIAL"}
]
const TEAM_TRAINING_IDS := ["shooting", "passing", "dribbling", "physical", "tactical", "defending"]
const SPECIAL_PLACEHOLDER_PROGRAMS := [
	{
		"id": "__special_event_placeholder",
		"name": "Special Event (Coming Soon)",
		"type": "special",
		"duration": 90,
		"attributes": {},
		"condition_cost": 0,
		"description": "Special events and camp content are coming soon.",
		"__placeholder": true
	}
]
const TRAINING_INTENSITY_STEPS := ["light", "normal", "intense"]
const TRAINING_INTENSITY_UI_META := {
	"light": {"label_key": "UI_TRAINING_INTENSITY_LIGHT", "hint_key": "UI_TRAINING_INTENSITY_HINT_LIGHT"},
	"normal": {"label_key": "UI_TRAINING_INTENSITY_NORMAL", "hint_key": "UI_TRAINING_INTENSITY_HINT_NORMAL"},
	"intense": {"label_key": "UI_TRAINING_INTENSITY_INTENSE", "hint_key": "UI_TRAINING_INTENSITY_HINT_INTENSE"}
}

# Mobile layout node references
# Mobile layout node references
@onready var mobile_back_button = $MobilePortraitLayout/Header/HBox/BackButton
@onready var mobile_fatigue_bar = $MobilePortraitLayout/Header/HBox/FatigueInfo/FatigueBar
@onready var mobile_cancel_button = $MobilePortraitLayout/BottomBar/HBox/CancelButton
@onready var mobile_confirm_button = $MobilePortraitLayout/BottomBar/HBox/ConfirmButton
@onready var mobile_tab_container = $MobilePortraitLayout/TabContainer

# Tablet layout node references
@onready var tablet_back_button = $TabletHybridLayout/Header/HBox/BackButton
@onready var tablet_fatigue_bar = $TabletHybridLayout/Header/HBox/FatigueInfo/FatigueBar
@onready var tablet_cancel_button = $TabletHybridLayout/BottomBar/HBox/CancelButton
@onready var tablet_confirm_button = $TabletHybridLayout/BottomBar/HBox/ConfirmButton
@onready var tablet_tab_container = $TabletHybridLayout/TabContainer
@onready var tablet_bottom_bar = $TabletHybridLayout/BottomBar/HBox

# Desktop layout node references
@onready var desktop_back_button = $DesktopLandscapeLayout/Header/HBox/BackButton
@onready var desktop_fatigue_bar = $DesktopLandscapeLayout/Header/HBox/FatigueInfo/FatigueBar
@onready var desktop_cancel_button = $DesktopLandscapeLayout/BottomBar/HBox/CancelButton
@onready var desktop_confirm_button = $DesktopLandscapeLayout/BottomBar/HBox/ConfirmButton
@onready var desktop_tab_container = $DesktopLandscapeLayout/TabContainer
@onready var desktop_bottom_bar = $DesktopLandscapeLayout/BottomBar/HBox
@onready var mobile_bottom_bar = $MobilePortraitLayout/BottomBar/HBox

# Selected training slot
var selected_training: String = ""
var selected_training_mode: String = "personal"
var selected_intensity_id: String = "normal"

# Training data caches
var training_programs: Array = []
var training_cards: Dictionary = {}  # training_id -> Array[Dictionary]
var training_program_lookup: Dictionary = {}
var layout_grids: Dictionary = {}
var close_callback: Callable = Callable()
var mode_buttons_by_layout: Dictionary = {}
var intensity_sliders_by_layout: Dictionary = {}
var intensity_labels_by_layout: Dictionary = {}
var deck_summary_labels: Dictionary = {}
var _deck_editor_dialog: ConfirmationDialog = null
var _deck_builder_screen: Control = null
var _deck_editor_script: Script = null
var _deck_build_scene: PackedScene = null


func _load_deck_resources() -> void:
	if ResourceLoader.exists(DECK_EDITOR_DIALOG_PATH):
		var script_variant := load(DECK_EDITOR_DIALOG_PATH)
		if script_variant is Script:
			_deck_editor_script = script_variant
	if ResourceLoader.exists(DECK_BUILD_SCREEN_PATH):
		var scene_variant := load(DECK_BUILD_SCREEN_PATH)
		if scene_variant is PackedScene:
			_deck_build_scene = scene_variant


func _get_condition_percentage() -> float:
	# Check if ConditionSystem autoload exists
	if Engine.has_singleton("ConditionSystem"):
		var condition_system = Engine.get_singleton("ConditionSystem")
		if condition_system and condition_system.has_method("get_condition_percentage"):
			return clampf(condition_system.get_condition_percentage(), 0.0, 100.0)
	# Fallback to PlayerCondition
	if PlayerCondition and PlayerCondition.has_method("get_stamina_percentage"):
		return clampf(PlayerCondition.get_stamina_percentage() * 100.0, 0.0, 100.0)
	return 30.0


func _update_fatigue_widgets(percentage: float) -> void:
	if mobile_fatigue_bar:
		mobile_fatigue_bar.value = percentage
	if tablet_fatigue_bar:
		tablet_fatigue_bar.value = percentage
	if desktop_fatigue_bar:
		desktop_fatigue_bar.value = percentage


func _set_confirm_button_state(button: Button, enabled: bool, reason: String) -> void:
	if not button:
		print("[TrainingScreen] WARNING: Confirm button is null!")
		return
	button.disabled = not enabled
	if OS.is_debug_build():
		print("[TrainingScreen] Confirm button state: enabled=%s, reason=%s" % [enabled, reason])
	if reason.is_empty():
		button.tooltip_text = ""
	else:
		button.tooltip_text = reason


func _update_confirm_buttons_state() -> void:
	var has_selection = not selected_training.is_empty()
	var can_execute = has_selection
	var reason = ""

	if has_selection:
		if not TrainingManager:
			can_execute = false
			reason = "훈련 시스템을 초기화할 수 없습니다"
		else:
			var selected_program = _get_program_for_id(selected_training)
			if selected_program.get("__placeholder", false):
				can_execute = false
				reason = "특별 이벤트가 아직 열리지 않았습니다."
			else:
				var check = TrainingManager.can_execute_training(selected_training, selected_training_mode)
				if not check.get("can_execute", false):
					can_execute = false
					reason = check.get("reason", "")

	_set_confirm_button_state(mobile_confirm_button, can_execute, reason)
	_set_confirm_button_state(tablet_confirm_button, can_execute, reason)
	_set_confirm_button_state(desktop_confirm_button, can_execute, reason)


func _connect_condition_signals() -> void:
	# Check if ConditionSystem autoload exists
	if not Engine.has_singleton("ConditionSystem"):
		return
	var condition_system = Engine.get_singleton("ConditionSystem")
	if not condition_system:
		return
	if (
		condition_system.has_signal("condition_changed")
		and not condition_system.condition_changed.is_connected(_on_condition_changed)
	):
		condition_system.condition_changed.connect(_on_condition_changed)


func _connect_date_signals() -> void:
	if not DateManager:
		return
	if not DateManager.week_started.is_connected(_on_week_started):
		DateManager.week_started.connect(_on_week_started)


func _apply_selected_preferences() -> void:
	if TrainingManager:
		if TrainingManager.has_method("set_frontend_training_mode"):
			TrainingManager.set_frontend_training_mode(selected_training_mode)
		if TrainingManager.has_method("set_training_intensity"):
			TrainingManager.set_training_intensity(selected_intensity_id)
	_connect_deck_signals()


func _connect_deck_signals() -> void:
	if not DeckManager:
		return
	if not DeckManager.deck_changed.is_connected(_on_deck_changed):
		DeckManager.deck_changed.connect(_on_deck_changed)
	if not DeckManager.card_added.is_connected(_on_deck_card_event):
		DeckManager.card_added.connect(_on_deck_card_event)
	if not DeckManager.card_removed.is_connected(_on_deck_card_event):
		DeckManager.card_removed.connect(_on_deck_card_event)


func _install_layout_controls() -> void:
	var bars = {"mobile": mobile_bottom_bar, "tablet": tablet_bottom_bar, "desktop": desktop_bottom_bar}
	for layout_name in bars.keys():
		var bar: HBoxContainer = bars[layout_name]
		if not bar:
			continue
		_build_training_controls_for_layout(layout_name, bar)


func _build_training_controls_for_layout(layout_name: String, bar: HBoxContainer) -> void:
	var wrapper_name = "%sTrainingControls" % layout_name.capitalize()
	var existing := bar.get_node_or_null(wrapper_name)
	if existing:
		existing.queue_free()
	var wrapper := VBoxContainer.new()
	wrapper.name = wrapper_name
	wrapper.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	wrapper.custom_minimum_size = Vector2(280, 0)
	bar.add_child(wrapper)
	bar.move_child(wrapper, 0)

	var mode_label := Label.new()
	mode_label.text = tr("UI_TRAINING_CONTROLS_MODE_LABEL")
	mode_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	wrapper.add_child(mode_label)

	var mode_container := HBoxContainer.new()
	mode_container.alignment = BoxContainer.ALIGNMENT_CENTER
	mode_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	wrapper.add_child(mode_container)
	var group := ButtonGroup.new()
	mode_buttons_by_layout[layout_name] = {}
	for config in TRAINING_MODE_CONFIG:
		var mode_button := Button.new()
		mode_button.toggle_mode = true
		mode_button.button_group = group
		var mode_id: String = String(config.get("id", "personal"))
		mode_button.text = _get_mode_label(mode_id)
		mode_button.focus_mode = Control.FOCUS_NONE
		mode_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		mode_button.tooltip_text = tr("UI_TRAINING_CONTROLS_MODE_TOOLTIP") % _get_mode_label(mode_id)
		mode_button.toggled.connect(_on_mode_button_toggled.bind(mode_id), Object.CONNECT_REFERENCE_COUNTED)
		mode_container.add_child(mode_button)
		mode_buttons_by_layout[layout_name][mode_id] = mode_button

	var intensity_box := VBoxContainer.new()
	intensity_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	wrapper.add_child(intensity_box)
	var intensity_label := Label.new()
	intensity_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	intensity_box.add_child(intensity_label)
	intensity_labels_by_layout[layout_name] = intensity_label
	var slider := HSlider.new()
	slider.min_value = 0
	slider.max_value = float(TRAINING_INTENSITY_STEPS.size() - 1)
	slider.step = 1.0
	slider.tick_count = TRAINING_INTENSITY_STEPS.size()
	slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	slider.value_changed.connect(_on_intensity_slider_changed.bind(layout_name), Object.CONNECT_REFERENCE_COUNTED)
	intensity_box.add_child(slider)
	intensity_sliders_by_layout[layout_name] = slider

	var deck_box := HBoxContainer.new()
	deck_box.alignment = BoxContainer.ALIGNMENT_CENTER
	deck_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	wrapper.add_child(deck_box)
	var deck_button := Button.new()
	deck_button.text = tr("UI_TRAINING_DECK_EDIT")
	deck_button.focus_mode = Control.FOCUS_NONE
	deck_button.pressed.connect(_on_deck_button_pressed, Object.CONNECT_REFERENCE_COUNTED)
	deck_box.add_child(deck_button)
	var deck_label: Label = Label.new()
	deck_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	deck_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	deck_label.text = tr("UI_TRAINING_DECK_SUMMARY_PLACEHOLDER")
	deck_box.add_child(deck_label)
	deck_summary_labels[layout_name] = deck_label


func _sync_mode_buttons() -> void:
	for layout_name in mode_buttons_by_layout.keys():
		for mode_id in mode_buttons_by_layout[layout_name].keys():
			var button: Button = mode_buttons_by_layout[layout_name][mode_id]
			if not button:
				continue
			button.set_pressed_no_signal(mode_id == selected_training_mode)
			button.disabled = false
			button.tooltip_text = tr("UI_TRAINING_CONTROLS_MODE_TOOLTIP") % _get_mode_label(mode_id)


func _sync_intensity_sliders() -> void:
	var target_index: int = TRAINING_INTENSITY_STEPS.find(selected_intensity_id)
	if target_index == -1:
		target_index = TRAINING_INTENSITY_STEPS.find("normal")
	if target_index == -1:
		target_index = 1
	for layout_name in intensity_sliders_by_layout.keys():
		var slider: HSlider = intensity_sliders_by_layout[layout_name]
		if slider and int(slider.value) != target_index:
			slider.set_value_no_signal(float(target_index))
		var label: Label = intensity_labels_by_layout.get(layout_name, null)
		if label:
			var meta: Dictionary = TRAINING_INTENSITY_UI_META.get(selected_intensity_id, {})
			var title := selected_intensity_id.capitalize()
			var hint := ""
			if meta:
				if meta.has("label_key"):
					title = tr(meta.get("label_key", "UI_TRAINING_INTENSITY_NORMAL"))
				if meta.has("hint_key"):
					hint = tr(meta.get("hint_key", ""))
			var hint_suffix := " (%s)" % hint if not hint.is_empty() else ""
			label.text = "%s: %s%s" % [tr("UI_TRAINING_CONTROLS_INTENSITY_LABEL"), title, hint_suffix]


func _update_deck_summary_labels() -> void:
	var deck_count := 0
	var deck_capacity := 6
	var tooltip := tr("UI_TRAINING_DECK_TOOLTIP_UNAVAILABLE")
	var bonus_text := ""
	if DeckManager:
		deck_count = DeckManager.current_deck.size()
		if DeckManager.has_method("get_deck_summary"):
			tooltip = DeckManager.get_deck_summary()
		if DeckManager.has_method("calculate_training_bonus"):
			var category := _get_current_deck_category_hint()
			if not category.is_empty():
				var bonus_info = DeckManager.calculate_training_bonus(category)
				var percent := float(bonus_info.get("total_bonus", 0.0)) * 100.0
				if percent > 0.0:
					bonus_text = " | %s +%.0f%%" % [tr("UI_TRAINING_RESULT_FIELD_DECK_BONUS"), percent]
		deck_capacity = int(DeckManager.MAX_DECK_SIZE)
	for layout_name in deck_summary_labels.keys():
		var label: Label = deck_summary_labels[layout_name]
		if not label:
			continue
		label.text = tr("UI_TRAINING_DECK_SUMMARY") % [deck_count, deck_capacity, bonus_text]
		label.tooltip_text = tooltip


func _on_deck_changed(_deck = null) -> void:
	_update_deck_summary_labels()


func _on_deck_card_event(_card) -> void:
	_update_deck_summary_labels()


func _on_deck_button_pressed() -> void:
	if _open_deck_build_screen():
		return
	_open_legacy_deck_dialog()


func _open_legacy_deck_dialog() -> void:
	if not DeckManager:
		_show_training_error("DeckManager unavailable")
		return
	if not _deck_editor_script:
		_show_training_error("Deck editor dialog unavailable")
		return
	if not _deck_editor_dialog or not is_instance_valid(_deck_editor_dialog):
		_deck_editor_dialog = _deck_editor_script.new()
		_deck_editor_dialog.deck_updated.connect(_update_deck_summary_labels)
		add_child(_deck_editor_dialog)
	_deck_editor_dialog.popup_centered()


func _open_deck_build_screen() -> bool:
	if _deck_build_scene == null:
		return false
	if _deck_builder_screen and is_instance_valid(_deck_builder_screen):
		_deck_builder_screen.grab_focus()
		return true
	var root := get_tree().root
	if root == null:
		return false
	var deck_screen = _deck_build_scene.instantiate()
	if deck_screen == null:
		return false
	if deck_screen is Control:
		_deck_builder_screen = deck_screen
		if deck_screen.has_method("set_close_callback"):
			deck_screen.set_close_callback(Callable(self, "_on_deck_builder_closed"))
		root.add_child(deck_screen)
		(deck_screen as Control).set_anchors_preset(Control.PRESET_FULL_RECT)
		deck_screen.z_index = max(deck_screen.z_index, 2048)
		deck_screen.grab_focus()
		return true
	deck_screen.queue_free()
	return false


func _on_deck_builder_closed() -> void:
	_cleanup_deck_build_screen()
	_update_deck_summary_labels()


func _cleanup_deck_build_screen() -> void:
	if _deck_builder_screen and is_instance_valid(_deck_builder_screen):
		var overlay := _deck_builder_screen
		_deck_builder_screen = null
		overlay.queue_free()
	else:
		_deck_builder_screen = null


func _on_mode_button_toggled(pressed: bool, mode_id: String) -> void:
	if not pressed or mode_id == selected_training_mode:
		return
	if not _is_mode_enabled(mode_id):
		_show_training_error("선택한 모드는 아직 사용할 수 없습니다.")
		_sync_mode_buttons()
		return
	selected_training_mode = mode_id
	_apply_selected_preferences()
	_sync_mode_buttons()
	_setup_training_cards()
	_update_confirm_buttons_state()


func _on_intensity_slider_changed(value: float, _layout_name: String) -> void:
	var index: int = clampi(roundi(value), 0, TRAINING_INTENSITY_STEPS.size() - 1)
	var intensity_id: String = String(TRAINING_INTENSITY_STEPS[index])
	if selected_intensity_id == intensity_id:
		return
	selected_intensity_id = intensity_id
	if TrainingManager:
		TrainingManager.set_training_intensity(selected_intensity_id)
	_sync_intensity_sliders()


func _is_mode_enabled(_mode_id: String) -> bool:
	return true


func _get_mode_label(mode_id: String) -> String:
	for config in TRAINING_MODE_CONFIG:
		if config.get("id", "") == mode_id:
			var label_key: String = String(config.get("label_key", ""))
			if not label_key.is_empty():
				return tr(label_key)
			return config.get("label", mode_id.capitalize())
	return tr("UI_TRAINING_MODE_PERSONAL") if mode_id == "personal" else mode_id.capitalize()


func _get_programs_for_mode(mode: String) -> Array:
	var result: Array = []
	if mode == "team":
		for program in training_programs:
			if TEAM_TRAINING_IDS.has(String(program.get("id", ""))):
				result.append(program.duplicate(true))
	elif mode == "special":
		for program in training_programs:
			if String(program.get("type", "")) == "special":
				result.append(program.duplicate(true))
		if result.is_empty():
			for placeholder in SPECIAL_PLACEHOLDER_PROGRAMS:
				result.append(placeholder.duplicate(true))
	else:
		for program in training_programs:
			var program_id := String(program.get("id", ""))
			if TEAM_TRAINING_IDS.has(program_id):
				continue
			if String(program.get("type", "")) == "special":
				continue
			result.append(program.duplicate(true))
	return result


func _get_program_for_id(training_id: String) -> Dictionary:
	if training_program_lookup.has(training_id):
		return training_program_lookup[training_id]
	for placeholder in SPECIAL_PLACEHOLDER_PROGRAMS:
		if placeholder.get("id", "") == training_id:
			return placeholder
	return {}


func _get_current_training_type_hint() -> String:
	if not selected_training.is_empty():
		return String(_get_program_for_id(selected_training).get("type", ""))
	for program in _get_programs_for_mode(selected_training_mode):
		if program.get("__placeholder", false):
			continue
		return String(program.get("type", ""))
	return ""


func _get_current_deck_category_hint() -> String:
	var training_type := _get_current_training_type_hint()
	return _map_training_type_to_category(training_type)


func _map_training_type_to_category(training_type: String) -> String:
	match training_type:
		"technical", "tactical":
			return "technical"
		"physical":
			return "physical"
		"mental", "defensive":
			return "mental"
		_:
			return ""


func _get_empty_state_message() -> String:
	if selected_training_mode == "team":
		return "No team training is scheduled this week."
	if selected_training_mode == "special":
		return "Special training content is still locked."
	return "No available training sessions."


func _ready():
	super._ready()  # Call AdaptiveLayoutContainer._ready()

	print("[TrainingScreen] Responsive scene initialized")
	_load_deck_resources()

	# Connect layout activation signals
	layout_activated.connect(_on_layout_activated)

	# Connect button signals for all layouts
	_connect_mobile_signals()
	_connect_tablet_signals()
	_connect_desktop_signals()
	_cache_layout_grids()
	_connect_condition_signals()
	_connect_date_signals()
	_install_layout_controls()
	_sync_mode_buttons()
	_sync_intensity_sliders()
	_update_deck_summary_labels()
	_apply_selected_preferences()
	_update_fatigue_widgets(_get_condition_percentage())
	_update_confirm_buttons_state()

	# Wait for platform detection
	await get_tree().process_frame

	# Load training data and populate UI
	_setup_training_cards()
	_populate_current_layout()

	# Validate UI standards
	_validate_ui_standards()


func _connect_mobile_signals():
	"""Connect Mobile layout button signals"""
	if mobile_back_button:
		mobile_back_button.pressed.connect(_on_back_pressed)
	if mobile_cancel_button:
		mobile_cancel_button.pressed.connect(_on_cancel_pressed)
	if mobile_confirm_button:
		mobile_confirm_button.pressed.connect(_on_confirm_pressed)


func _connect_tablet_signals():
	"""Connect Tablet layout button signals"""
	if tablet_back_button:
		tablet_back_button.pressed.connect(_on_back_pressed)
	if tablet_cancel_button:
		tablet_cancel_button.pressed.connect(_on_cancel_pressed)
	if tablet_confirm_button:
		tablet_confirm_button.pressed.connect(_on_confirm_pressed)


func _connect_desktop_signals():
	"""Connect Desktop layout button signals"""
	if desktop_back_button:
		desktop_back_button.pressed.connect(_on_back_pressed)
	if desktop_cancel_button:
		desktop_cancel_button.pressed.connect(_on_cancel_pressed)
	if desktop_confirm_button:
		desktop_confirm_button.pressed.connect(_on_confirm_pressed)


func _on_layout_activated(layout_name: String):
	"""Handle layout activation"""
	print(
		(
			"[TrainingScreen] Layout activated: %s (Platform: %s)"
			% [layout_name, PlatformManager.get_platform_name() if PlatformManager else "Unknown"]
		)
	)
	_populate_current_layout()


func _populate_current_layout():
	"""Populate data for currently active layout"""
	var active = get_active_layout()
	if not active:
		push_warning("[TrainingScreen] No active layout found")
		return

	_update_fatigue_widgets(_get_condition_percentage())

	match get_active_layout_name():
		"mobile":
			_populate_mobile_layout()
		"tablet":
			_populate_tablet_layout()
		"desktop":
			_populate_desktop_layout()

	_sync_mode_buttons()
	_sync_intensity_sliders()
	_update_deck_summary_labels()
	_update_confirm_buttons_state()


func _populate_mobile_layout():
	"""Populate mobile-specific layout with training data"""
	print("[TrainingScreen] Populating mobile layout")


func _populate_tablet_layout():
	"""Populate tablet-specific layout with training data"""
	print("[TrainingScreen] Populating tablet layout")


func _populate_desktop_layout():
	"""Populate desktop-specific layout with training data"""
	print("[TrainingScreen] Populating desktop layout")


func _validate_ui_standards():
	"""Validate UI against UIStandards requirements"""
	validate_ui_standards_base()


func _cache_layout_grids() -> void:
	layout_grids.clear()
	layout_grids["mobile"] = {
		"grid": $MobilePortraitLayout/TabContainer/기술/GridContainer,
		"fatigue_bar": mobile_fatigue_bar,
		"confirm": mobile_confirm_button
	}
	layout_grids["tablet"] = {
		"grid": $TabletHybridLayout/TabContainer/기술/GridContainer,
		"fatigue_bar": tablet_fatigue_bar,
		"confirm": tablet_confirm_button
	}
	layout_grids["desktop"] = {
		"grid": $DesktopLandscapeLayout/TabContainer/기술/GridContainer,
		"fatigue_bar": desktop_fatigue_bar,
		"confirm": desktop_confirm_button
	}


func _setup_training_cards() -> void:
	training_cards.clear()
	training_program_lookup.clear()
	if TrainingManager:
		training_programs = TrainingManager.get_available_trainings()
	else:
		training_programs = []
	for program in training_programs:
		var program_id := String(program.get("id", ""))
		if not program_id.is_empty():
			training_program_lookup[program_id] = program

	var first_selectable_id := ""
	for layout in layout_grids.keys():
		var info = layout_grids[layout]
		var grid: GridContainer = info["grid"]
		if not grid:
			continue

		var template: Control = null
		if grid.get_child_count() > 0:
			template = grid.get_child(0)
			grid.remove_child(template)

		for child in grid.get_children():
			child.queue_free()

		var mode_programs = _get_programs_for_mode(selected_training_mode)
		if mode_programs.is_empty():
			var empty_label := Label.new()
			empty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			empty_label.text = _get_empty_state_message()
			empty_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			grid.add_child(empty_label)
		else:
			for program in mode_programs:
				var card: Control = null
				if template:
					card = template.duplicate(true)
				else:
					card = Panel.new()
				card.name = "%s_%s" % [layout, program.get("id", "training")]
				grid.add_child(card)
				_configure_training_card(card, program)
				if first_selectable_id.is_empty() and not program.get("__placeholder", false):
					first_selectable_id = String(program.get("id", ""))

		if template:
			template.queue_free()

	if not training_cards.has(selected_training):
		selected_training = ""
	if selected_training.is_empty():
		selected_training = first_selectable_id
	_update_selection_states()


func _configure_training_card(card: Control, program: Dictionary) -> void:
	var title: Label = card.get_node("VBox/Header/Title") if card.has_node("VBox/Header/Title") else null
	if title:
		title.text = program.get("name", "훈련")

	var effects_container: VBoxContainer = card.get_node("VBox/Effects") if card.has_node("VBox/Effects") else null
	if effects_container:
		for child in effects_container.get_children():
			child.queue_free()
		var attributes: Dictionary = program.get("attributes", {})
		if attributes.is_empty():
			var desc_label = Label.new()
			var fallback_desc := tr("UI_TRAINING_CARD_DESC_PLACEHOLDER")
			desc_label.text = program.get("description", fallback_desc)
			desc_label.add_theme_font_size_override("font_size", 18)
			desc_label.add_theme_color_override("font_color", Color.DARK_GRAY)
			effects_container.add_child(desc_label)
		else:
			for attr in attributes.keys():
				var delta: int = attributes[attr]
				var effect_label = Label.new()
				effect_label.text = "- %s %+d" % [attr, delta]
				effect_label.add_theme_font_size_override("font_size", 20)
				effect_label.add_theme_color_override("font_color", Color(0.3, 1, 0.3))
				effects_container.add_child(effect_label)
		var cost_label = Label.new()
		cost_label.text = (
			tr("UI_TRAINING_CARD_COST_TEMPLATE")
			% [int(program.get("duration", 0)), int(program.get("condition_cost", 0))]
		)
		cost_label.add_theme_font_size_override("font_size", 18)
		cost_label.add_theme_color_override("font_color", Color(1, 0.5, 0.5))
		effects_container.add_child(cost_label)
		var note_text := String(program.get("ui_note", program.get("description", "")))
		if not note_text.is_empty():
			var note_label := Label.new()
			note_label.text = note_text
			note_label.add_theme_font_size_override("font_size", 16)
			note_label.add_theme_color_override("font_color", Color(0.6, 0.6, 0.6))
			note_label.autowrap_mode = TextServer.AUTOWRAP_WORD
			effects_container.add_child(note_label)

	var button: Button = card.get_node("VBox/SelectButton") if card.has_node("VBox/SelectButton") else null
	if button:
		var program_id := String(program.get("id", ""))
		var is_placeholder := bool(program.get("__placeholder", false))
		button.text = tr("UI_TRAINING_CARD_SELECT") if not is_placeholder else tr("UI_TRAINING_CARD_LOCKED")
		button.disabled = is_placeholder or program_id.is_empty()
		button.set_meta("training_id", program_id)
		button.set_meta("placeholder", is_placeholder)
		var tooltip_note := String(program.get("ui_note", program.get("description", "")))
		if not button.disabled and not tooltip_note.is_empty():
			button.tooltip_text = tooltip_note
		else:
			button.tooltip_text = (
				tr("UI_TRAINING_CARD_TOOLTIP_SELECT")
				if not button.disabled
				else tr("UI_TRAINING_CARD_TOOLTIP_PLACEHOLDER")
			)
		if not button.disabled:
			button.pressed.connect(_on_training_card_button_pressed.bind(program_id), Object.CONNECT_REFERENCE_COUNTED)
	_register_training_card(program.get("id", ""), card, button)


func _register_training_card(training_id: String, card: Control, button: Button) -> void:
	if training_id.is_empty():
		return
	if not training_cards.has(training_id):
		training_cards[training_id] = []
	training_cards[training_id].append({"card": card, "button": button})


func set_close_callback(callback: Callable) -> void:
	close_callback = callback


func _request_close() -> void:
	_cleanup_deck_build_screen()
	if close_callback.is_valid():
		close_callback.call()
	else:
		get_tree().change_scene_to_file("res://scenes/HomeImproved_Responsive.tscn")


func _on_training_card_button_pressed(training_id: String) -> void:
	selected_training = training_id
	_update_selection_states()
	_populate_current_layout()


func _update_selection_states() -> void:
	if not selected_training.is_empty() and not training_cards.has(selected_training):
		selected_training = ""
	for training_id in training_cards.keys():
		for entry in training_cards[training_id]:
			var card: Control = entry.get("card")
			var button: Button = entry.get("button")
			var is_placeholder := button and bool(button.get_meta("placeholder", false))
			if training_id == selected_training and not is_placeholder:
				if card:
					card.self_modulate = Color(0.9, 1.0, 0.9)
				if button:
					button.text = tr("UI_TRAINING_CARD_SELECTED")
					button.disabled = true
			else:
				if card:
					card.self_modulate = Color(1, 1, 1)
				if button:
					button.text = tr("UI_TRAINING_CARD_SELECT") if not is_placeholder else tr("UI_TRAINING_CARD_LOCKED")
					button.disabled = is_placeholder
	_update_confirm_buttons_state()


## Button signal handlers


func _on_back_pressed():
	"""Navigate back to home screen"""
	print("[TrainingScreen] Back button pressed")
	_request_close()


func _on_cancel_pressed():
	"""Cancel training selection"""
	print("[TrainingScreen] Cancel button pressed")
	selected_training = ""
	_populate_current_layout()  # Refresh UI
	_request_close()


func _on_confirm_pressed():
	"""Confirm and execute selected training"""
	print("[TrainingScreen] Confirm button pressed - Training: %s" % selected_training)

	if selected_training.is_empty():
		push_warning("[TrainingScreen] No training selected")
		return

	var training_id := selected_training

	# Execute training via TrainingManager
	if TrainingManager:
		var program = _get_program_for_id(training_id)
		if program.get("__placeholder", false):
			_show_training_error("특별 이벤트가 아직 준비 중입니다.")
			return
		var eligibility = TrainingManager.can_execute_training(selected_training, selected_training_mode)
		if not eligibility.get("can_execute", false):
			var reason = eligibility.get("reason", "이 훈련은 지금 실행할 수 없습니다.")
			_show_training_error(reason)
			_update_confirm_buttons_state()
			return

		var is_personal := selected_training_mode != "team"
		var result = TrainingManager.execute_training(selected_training, is_personal)

		if result.success:
			_show_training_result(result)
			selected_training = ""
			_update_selection_states()
			_populate_current_layout()
			training_executed.emit(training_id, result)
			_push_training_to_weekly_plan(training_id, result)
			_request_close()
		else:
			_show_training_error(result.message)
			_update_confirm_buttons_state()
	else:
		push_error("[TrainingScreen] TrainingManager not found")
	_update_confirm_buttons_state()


func _on_training_selected(training_id: String):
	"""Handle training card selection"""
	print("[TrainingScreen] Training selected: %s" % training_id)
	selected_training = training_id
	_update_selection_states()
	_update_confirm_buttons_state()  # Update button state immediately
	_populate_current_layout()  # Refresh UI (enables confirm button)


func _on_condition_changed(_level, percentage: float) -> void:
	_update_fatigue_widgets(percentage)
	_update_confirm_buttons_state()


func _on_week_started(_week_number: int, _week_schedule) -> void:
	_update_fatigue_widgets(_get_condition_percentage())
	_update_confirm_buttons_state()


## Public API for manager integration


func set_available_trainings(trainings: Array):
	"""
	Set list of available training programs (called by TrainingManager)

	Args:
		trainings: Array of training dictionaries with structure:
			{ "id": String, "name": String, "type": String, "duration": int, ... }
	"""
	print("[TrainingScreen] Setting available trainings: %d programs" % trainings.size())
	training_programs = trainings.duplicate(true)
	_setup_training_cards()
	_populate_current_layout()


func select_training(training_id: String):
	"""
	Programmatically select a training (called by TrainingManager or UI)

	Args:
		training_id: ID of the training program to select
	"""
	if training_id.is_empty():
		push_warning("[TrainingScreen] Cannot select empty training ID")
		return

	selected_training = training_id
	_populate_current_layout()
	print("[TrainingScreen] Training selected programmatically: %s" % training_id)


func get_selected_training() -> String:
	"""
	Get currently selected training ID

	Returns:
		String: ID of selected training, or empty string if none selected
	"""
	return selected_training


func _push_training_to_weekly_plan(training_id: String, result: Dictionary) -> void:
	if not DateManager or not DateManager.has_method("record_training_activity"):
		return
	var payload := {
		"training_id": training_id,
		"training_name": result.get("training_name", training_id),
		"mode": result.get("mode", selected_training_mode),
		"intensity": result.get("intensity", selected_intensity_id),
		"result": result.duplicate(true),
		"timestamp": Time.get_unix_time_from_system()
	}
	if result.has("deck_bonus"):
		payload["deck_bonus"] = result.get("deck_bonus")
	if result.has("deck_snapshot"):
		payload["deck_snapshot"] = result.get("deck_snapshot")
	DateManager.record_training_activity(payload)


## Debug helpers


func print_layout_debug_info():
	"""Print detailed layout information for debugging"""
	print_layout_info()  # From AdaptiveLayoutContainer

	print("\n[TrainingScreen] Data State:")
	print("  PlayerCondition: %s" % ("✓" if PlayerCondition else "✗"))
	print("  Selected Training: %s" % selected_training)

	if PlatformManager:
		print("\n[PlatformManager]:")
		print("  Platform: %s" % PlatformManager.get_platform_name())
		print("  Orientation: %s" % PlatformManager.get_orientation_name())
		print("  Viewport: %v" % PlatformManager.viewport_size)
		print("  DPI: %d" % PlatformManager.dpi)


func _show_training_result(result: Dictionary) -> void:
	var payload: Dictionary = TrainingEventPayloadClass.normalize(result)
	if payload.is_empty():
		return

	var dialog = AcceptDialog.new()
	dialog.title = tr("UI_TRAINING_RESULT_TITLE")

	var lines := PackedStringArray()
	lines.append(
		(
			"%s: %s (%s · %s)"
			% [
				tr("UI_TRAINING_RESULT_FIELD_SUMMARY"),
				String(payload.get("training_name", "")),
				String(payload.get("mode_label", "")),
				String(payload.get("intensity_label", ""))
			]
		)
	)

	var deck_text := tr("UI_TRAINING_RESULT_FIELD_NONE")
	var deck_bonus_pct := int(payload.get("deck_bonus_pct", 0))
	if deck_bonus_pct != 0:
		deck_text = "+%d%%" % deck_bonus_pct
	lines.append("%s: %s" % [tr("UI_TRAINING_RESULT_FIELD_DECK_BONUS"), deck_text])

	var coach_logs_variant: Variant = payload.get("coach_bonus_log", [])
	var coach_logs: Array = coach_logs_variant if coach_logs_variant is Array else []
	var coach_text := tr("UI_TRAINING_RESULT_FIELD_NONE")
	if coach_logs is Array and coach_logs.size() > 0:
		coach_text = str(coach_logs.size())
	lines.append("%s: %s" % [tr("UI_TRAINING_RESULT_FIELD_COACH_BONUS"), coach_text])

	var training_load_line := tr("UI_TRAINING_RESULT_FIELD_NONE")
	var training_load_variant: Variant = payload.get("training_load", {})
	if training_load_variant is Dictionary and not (training_load_variant as Dictionary).is_empty():
		var load_parts := PackedStringArray()
		var training_load: Dictionary = training_load_variant
		if training_load.has("load_ratio"):
			load_parts.append("x%.2f" % float(training_load["load_ratio"]))
		if training_load.has("fatigue_cost"):
			load_parts.append("%s +%d" % [tr("UI_TRAINING_RESULT_FIELD_FATIGUE"), int(training_load["fatigue_cost"])])
		if training_load.has("needs_rest") and bool(training_load["needs_rest"]):
			load_parts.append(tr("UI_TRAINING_SUMMARY_REST_WARNING"))
		if load_parts.is_empty():
			load_parts.append(str(training_load))
		training_load_line = ", ".join(load_parts)
	lines.append("%s: %s" % [tr("UI_TRAINING_RESULT_FIELD_TRAINING_LOAD"), training_load_line])

	var changes_variant: Variant = payload.get("changes", {})
	var changes_dict: Dictionary = changes_variant if changes_variant is Dictionary else {}
	var changes_line := tr("UI_TRAINING_RESULT_FIELD_NONE")
	if not changes_dict.is_empty():
		var change_parts := PackedStringArray()
		for attr in changes_dict.keys():
			change_parts.append("%s %+d" % [attr, int(changes_dict[attr])])
		changes_line = ", ".join(change_parts)
	lines.append("%s: %s" % [tr("UI_TRAINING_RESULT_FIELD_CHANGES"), changes_line])

	var fatigue_cost := int(roundi(float(payload.get("condition_cost", 0.0))))
	lines.append("%s: +%d" % [tr("UI_TRAINING_RESULT_FIELD_FATIGUE"), fatigue_cost])

	var deck_snapshot_variant: Variant = payload.get("deck_snapshot", [])
	var deck_snapshot: Array = deck_snapshot_variant if deck_snapshot_variant is Array else []
	if deck_snapshot.size() > 0:
		var card_names := PackedStringArray()
		for card in deck_snapshot:
			if card is Dictionary:
				var label := String(card.get("name", card.get("character_name", card.get("id", ""))))
				if not label.is_empty():
					card_names.append(label)
		if card_names.size() > 0:
			lines.append("%s: %s" % [tr("UI_TRAINING_RESULT_FIELD_ACTIVE_CARDS"), ", ".join(card_names)])

	var note_text := String(payload.get("ui_note", ""))
	if note_text.is_empty():
		note_text = String(payload.get("description", ""))
	if not note_text.is_empty():
		lines.append("%s: %s" % [tr("UI_TRAINING_RESULT_FIELD_NOTE"), note_text])

	dialog.dialog_text = "\n".join(lines)
	add_child(dialog)
	dialog.popup_centered()
	dialog.confirmed.connect(Callable(self, "_queue_free_dialog").bind(dialog), Object.CONNECT_REFERENCE_COUNTED)
	# Godot 4.x: AcceptDialog doesn't have 'closed' signal, only 'canceled'
	if dialog.has_signal("canceled"):
		dialog.canceled.connect(Callable(self, "_queue_free_dialog").bind(dialog), Object.CONNECT_REFERENCE_COUNTED)


func _show_training_error(message: String) -> void:
	var dialog = AcceptDialog.new()
	dialog.title = tr("UI_TRAINING_ERROR_TITLE")
	dialog.dialog_text = message
	add_child(dialog)
	dialog.popup_centered()
	dialog.confirmed.connect(Callable(self, "_queue_free_dialog").bind(dialog), Object.CONNECT_REFERENCE_COUNTED)
	# Godot 4.x: AcceptDialog doesn't have 'closed' signal, only 'canceled'
	if dialog.has_signal("canceled"):
		dialog.canceled.connect(Callable(self, "_queue_free_dialog").bind(dialog), Object.CONNECT_REFERENCE_COUNTED)


func _queue_free_dialog(dialog: Window) -> void:
	if dialog and dialog.is_inside_tree():
		dialog.queue_free()
