extends Control
class_name ChoiceLogScreen

## Choice Log Screen
## Displays history of all player choices with immediate and long-term impacts

@onready var choice_list: VBoxContainer = $Panel/MarginContainer/VBox/ScrollContainer/ChoiceList
@onready var close_button: Button = $Panel/MarginContainer/VBox/TopBar/CloseButton
@onready var no_choices_label: Label = $Panel/MarginContainer/VBox/NoChoicesLabel
@onready var filter_container: HBoxContainer = $Panel/MarginContainer/VBox/FilterBar

var choice_entry_scene = preload("res://scenes/ui/ChoiceLogEntry.tscn")

# Filter state
var current_character_filter: String = "all"  # "all", "rival", "friendship", "mentor", "captain", "guardian"
var current_type_filter: String = "all"  # "all", "low_point", "critical_choice", etc.


func _ready():
	close_button.pressed.connect(_on_close_button_pressed)
	_setup_filters()
	_load_choice_history()


func _setup_filters() -> void:
	"""Setup filter buttons"""
	if not filter_container:
		# Create filter container if it doesn't exist in scene
		filter_container = HBoxContainer.new()
		var vbox = $Panel/MarginContainer/VBox
		vbox.add_child(filter_container)
		vbox.move_child(filter_container, 1)  # Place after TopBar

	# Character filters
	var char_label = Label.new()
	char_label.text = "캐릭터: "
	filter_container.add_child(char_label)

	var char_filters = ["all", "rival", "friendship", "mentor", "captain", "guardian"]
	var char_names = {
		"all": "전체", "rival": "강태양", "friendship": "박민준", "mentor": "김철수", "captain": "이서준", "guardian": "최지훈"
	}

	for filter_id in char_filters:
		var btn = Button.new()
		btn.text = char_names[filter_id]
		btn.toggle_mode = true
		btn.pressed.connect(_on_character_filter_pressed.bind(filter_id))
		filter_container.add_child(btn)

		if filter_id == "all":
			btn.button_pressed = true  # Default selection

	# Spacer
	var spacer = Control.new()
	spacer.custom_minimum_size = Vector2(20, 0)
	filter_container.add_child(spacer)

	# Type filters
	var type_label = Label.new()
	type_label.text = "타입: "
	filter_container.add_child(type_label)

	var type_filters = ["all", "low_point", "critical_choice", "turning_point"]
	var type_names = {"all": "전체", "low_point": "위기", "critical_choice": "선택", "turning_point": "전환점"}

	for filter_id in type_filters:
		var btn = Button.new()
		btn.text = type_names.get(filter_id, filter_id)
		btn.toggle_mode = true
		btn.pressed.connect(_on_type_filter_pressed.bind(filter_id))
		filter_container.add_child(btn)

		if filter_id == "all":
			btn.button_pressed = true  # Default selection


func _load_choice_history() -> void:
	"""Load and display choice history from EventManager"""
	# Clear existing entries
	for child in choice_list.get_children():
		child.queue_free()

	# Get history from EventManager
	if not EventManager:
		_show_no_choices()
		return

	var history = EventManager.get_choice_history()

	if history.is_empty():
		_show_no_choices()
		return

	# Hide no choices label
	no_choices_label.visible = false

	# Create entry for each choice (with filtering)
	var displayed_count = 0
	for choice_record in history:
		# Apply filters
		if not _passes_filters(choice_record):
			continue

		var entry = choice_entry_scene.instantiate()
		choice_list.add_child(entry)
		entry.set_choice_data(choice_record)
		displayed_count += 1

	if displayed_count == 0:
		_show_no_choices()

	print("[ChoiceLogScreen] Loaded %d/%d choices (filtered)" % [displayed_count, history.size()])


func _show_no_choices() -> void:
	"""Show message when no choices have been made"""
	no_choices_label.visible = true
	no_choices_label.text = "아직 선택한 분기점이 없습니다."


func _on_close_button_pressed() -> void:
	"""Close the choice log screen"""
	queue_free()


# ============================================
# Filter Functions
# ============================================


func _passes_filters(choice_record: Dictionary) -> bool:
	"""Check if choice passes current filters"""
	var event_data = choice_record.get("event_data", {})

	# Character filter
	if current_character_filter != "all":
		var route = event_data.get("route_id", "")
		if route != current_character_filter:
			return false

	# Type filter
	if current_type_filter != "all":
		var event_type = event_data.get("type", "")
		if event_type != current_type_filter:
			return false

	return true


func _on_character_filter_pressed(filter_id: String) -> void:
	"""Handle character filter button press"""
	current_character_filter = filter_id

	# Uncheck other buttons in same group
	for child in filter_container.get_children():
		if child is Button and child.toggle_mode:
			if child.text in ["전체", "강태양", "박민준", "김철수", "이서준", "최지훈"]:
				child.button_pressed = false

	# Reload with new filter
	_load_choice_history()


func _on_type_filter_pressed(filter_id: String) -> void:
	"""Handle type filter button press"""
	current_type_filter = filter_id

	# Uncheck other buttons in same group
	for child in filter_container.get_children():
		if child is Button and child.toggle_mode:
			if child.text in ["전체", "위기", "선택", "전환점"]:
				child.button_pressed = false

	# Reload with new filter
	_load_choice_history()
