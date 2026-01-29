extends Control
class_name SaveLoadDialog

## SaveLoadDialog: Modal dialog for saving/loading game progress
## Used by StatusScreenImproved_Responsive.gd for save/load functionality
## Interfaces with SaveManager autoload for slot management

signal save_requested(slot_id: String)
signal load_requested(slot_id: String)
signal dialog_closed

var current_mode: String = "save"  # "save" or "load"
var selected_slot: int = -1

@onready var background: ColorRect = $Background
@onready var panel: Panel = $Panel
@onready var title: Label = $Panel/Title
@onready var save_slots: ItemList = $Panel/SaveSlots
@onready var save_button: Button = $Panel/SaveButton
@onready var load_button: Button = $Panel/LoadButton
@onready var delete_button: Button = $Panel/DeleteButton
@onready var close_button: Button = $Panel/CloseButton


func _ready() -> void:
	visible = true
	mouse_filter = Control.MOUSE_FILTER_STOP
	if background:
		background.mouse_filter = Control.MOUSE_FILTER_STOP

	_connect_signals()
	_populate_save_slots()
	_update_ui_for_mode()


## Public API: Set dialog mode to "save" or "load"
## Called by StatusScreenImproved_Responsive.gd after instantiation
func set_mode(mode: String) -> void:
	current_mode = mode
	_update_ui_for_mode()


## Connect all button signals and SaveManager signals
func _connect_signals() -> void:
	if save_slots:
		save_slots.item_selected.connect(_on_slot_selected)

	if save_button:
		save_button.pressed.connect(_on_save_pressed)

	if load_button:
		load_button.pressed.connect(_on_load_pressed)

	if delete_button:
		delete_button.pressed.connect(_on_delete_pressed)

	if close_button:
		close_button.pressed.connect(_on_close_pressed)

	# Connect SaveManager signals if available
	if SaveManager:
		if not SaveManager.save_completed.is_connected(_on_save_completed):
			SaveManager.save_completed.connect(_on_save_completed)
		if not SaveManager.load_completed.is_connected(_on_load_completed):
			SaveManager.load_completed.connect(_on_load_completed)
		if not SaveManager.save_error.is_connected(_on_save_error):
			SaveManager.save_error.connect(_on_save_error)


## Populate ItemList with all save slots from SaveManager
func _populate_save_slots() -> void:
	if not save_slots:
		return

	save_slots.clear()

	if not SaveManager:
		push_error("[SaveLoadDialog] SaveManager not found")
		return

	for i in range(SaveManager.MAX_SAVE_SLOTS):
		var slot_info = SaveManager.get_slot_info(i)
		var display_text = _format_slot_display(i, slot_info)
		save_slots.add_item(display_text)
		save_slots.set_item_metadata(i, slot_info)


## Format slot display text based on slot info
## @param slot_index: Slot index (0-based)
## @param slot_info: Dictionary from SaveManager.get_slot_info()
## @return Formatted string for ItemList display
func _format_slot_display(slot_index: int, slot_info: Dictionary) -> String:
	if not slot_info.get("exists", false):
		return "슬롯 %d: (비어있음)" % (slot_index + 1)

	var player_name = str(slot_info.get("player_name", "Unknown"))
	var progress = float(slot_info.get("progress", 0.0))
	return "슬롯 %d: %s (진행도: %.1f%%)" % [slot_index + 1, player_name, progress]


## Save button pressed - save game to selected slot
func _on_save_pressed() -> void:
	if selected_slot < 0:
		return

	if not SaveManager:
		push_error("[SaveLoadDialog] SaveManager not found")
		return

	var slot_id = "slot_%d" % selected_slot
	print("[SaveLoadDialog] Saving to slot: ", slot_id)
	SaveManager.save_game(slot_id, {})  # Empty dict, SaveManager gathers data


## Load button pressed - load game from selected slot
func _on_load_pressed() -> void:
	if selected_slot < 0:
		return

	if not save_slots:
		return

	var slot_info = save_slots.get_item_metadata(selected_slot)
	if not slot_info.get("exists", false):
		return  # Can't load empty slot

	if not SaveManager:
		push_error("[SaveLoadDialog] SaveManager not found")
		return

	var slot_id = "slot_%d" % selected_slot
	print("[SaveLoadDialog] Loading from slot: ", slot_id)
	SaveManager.load_game(slot_id)


## Delete button pressed - delete selected slot
func _on_delete_pressed() -> void:
	if selected_slot < 0:
		return

	if not SaveManager:
		push_error("[SaveLoadDialog] SaveManager not found")
		return

	print("[SaveLoadDialog] Deleting slot: ", selected_slot)
	if SaveManager.delete_slot(selected_slot):
		_populate_save_slots()
		selected_slot = -1
		_update_button_states()


## Close button pressed - emit signal and free dialog
func _on_close_pressed() -> void:
	dialog_closed.emit()
	queue_free()


## Slot selected in ItemList - update button states
func _on_slot_selected(index: int) -> void:
	selected_slot = index
	_update_button_states()


## SaveManager signal: Save completed successfully
func _on_save_completed(slot_id: String) -> void:
	print("[SaveLoadDialog] Save completed: ", slot_id)
	_populate_save_slots()
	# Auto-close after successful save
	_on_close_pressed()


## SaveManager signal: Load completed successfully
func _on_load_completed(slot_id: String) -> void:
	print("[SaveLoadDialog] Load completed: ", slot_id)
	_on_close_pressed()


## SaveManager signal: Save/load error occurred
func _on_save_error(error_message: String) -> void:
	push_error("[SaveLoadDialog] Save error: ", error_message)
	# TODO: Show error popup to user


## Update UI elements based on current mode ("save" or "load")
func _update_ui_for_mode() -> void:
	if not title or not save_button or not load_button or not delete_button:
		return

	if current_mode == "save":
		title.text = "게임 저장"
		save_button.visible = true
		load_button.visible = false
		delete_button.visible = true
	else:  # "load"
		title.text = "게임 불러오기"
		save_button.visible = false
		load_button.visible = true
		delete_button.visible = false

	_update_button_states()


## Update button enabled/disabled states based on selection and mode
func _update_button_states() -> void:
	if not save_slots:
		return

	var slot_selected = selected_slot >= 0

	if current_mode == "save":
		if save_button:
			save_button.disabled = not slot_selected
		if delete_button:
			delete_button.disabled = not slot_selected
	else:  # "load"
		if slot_selected:
			var slot_info = save_slots.get_item_metadata(selected_slot)
			if load_button:
				load_button.disabled = not slot_info.get("exists", false)
		else:
			if load_button:
				load_button.disabled = true
