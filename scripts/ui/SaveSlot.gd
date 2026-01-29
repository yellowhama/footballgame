extends PanelContainer

# This script manages a single save/load slot in a list.
# It's responsible for displaying its state (filled or empty)
# and handling the load action.

@onready var player_name_label: Label = %PlayerNameLabel
@onready var progress_label: Label = %ProgressLabel
@onready var play_time_label: Label = %PlayTimeLabel
@onready var empty_label: Label = %EmptyLabel
@onready var info_container: HBoxContainer = %InfoContainer

signal slot_loaded(slot_number)

var _slot_number: int = 0


func update_info(info: Dictionary, slot_num: int):
	"""Populates the slot's UI based on the provided save data."""
	_slot_number = slot_num

	if info.get("empty", true):
		# --- Show as EMPTY slot ---
		info_container.visible = false
		empty_label.visible = true
		empty_label.text = "➕ 빈 슬롯 %d" % _slot_number

		# Remove any existing load button
		var existing_button = find_child("LoadButton", false, false)
		if existing_button:
			existing_button.queue_free()

		add_theme_stylebox_override("panel", CustomStyles.create_save_slot_empty())
	else:
		# --- Show as FILLED slot ---
		info_container.visible = true
		empty_label.visible = false

		player_name_label.text = info.get("player_name", "Unknown")
		progress_label.text = "%d학년 %d주차" % [info.get("year", 1), info.get("week", 1)]

		# Format timestamp
		var timestamp = info.get("timestamp", 0)
		if timestamp > 0:
			var dt = Time.get_datetime_dict_from_unix_time(timestamp)
			play_time_label.text = "%02d월 %02d일 %02d:%02d" % [dt.month, dt.day, dt.hour, dt.minute]
		else:
			play_time_label.text = ""

		_add_load_button()
		add_theme_stylebox_override("panel", CustomStyles.create_save_slot_filled())


func _add_load_button():
	"""Adds a load button to the slot."""
	# Remove if it exists to prevent duplicates
	var existing_button = find_child("LoadButton", false, false)
	if existing_button:
		existing_button.queue_free()

	var load_btn = Button.new()
	load_btn.name = "LoadButton"
	load_btn.text = "불러오기"
	load_btn.custom_minimum_size = Vector2(100, 40)
	load_btn.position = Vector2(size.x - 120, size.y / 2 - 20)

	# Basic styling
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.5, 0.8, 1)
	style.corner_radius_top_left = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	style.corner_radius_top_right = 8
	load_btn.add_theme_stylebox_override("normal", style)

	load_btn.pressed.connect(_on_load_pressed)
	add_child(load_btn)


func _on_load_pressed():
	"""Emits a signal that this slot was chosen to be loaded."""
	print("[SaveSlot] Load pressed for slot %d" % _slot_number)
	slot_loaded.emit(_slot_number)
