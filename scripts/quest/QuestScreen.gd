extends Control
## Quest Screen - displays quests from Rust QuestBridge

signal quest_activated(quest_id: String)
signal quest_completed(quest_id: String)

const QUEST_ITEM_HEIGHT := 80
const QUEST_ITEM_SPACING := 8
const QUEST_NOTIFICATION_SCENE := preload("res://scenes/quest/QuestNotification.tscn")

@onready var tab_container: TabContainer = $MainPanel/VBox/TabContainer
@onready var main_quests_list: VBoxContainer = $MainPanel/VBox/TabContainer/Main/ScrollContainer/QuestList
@onready var side_quests_list: VBoxContainer = $MainPanel/VBox/TabContainer/Side/ScrollContainer/QuestList
@onready var daily_quests_list: VBoxContainer = $MainPanel/VBox/TabContainer/Daily/ScrollContainer/QuestList
@onready var stats_label: Label = $MainPanel/VBox/Header/StatsLabel
@onready var close_button: Button = $MainPanel/VBox/Header/CloseButton
@onready var refresh_button: Button = $MainPanel/VBox/Footer/RefreshButton

var _quest_bridge: RefCounted = null
var _close_callback: Callable = Callable()


func _ready() -> void:
	_init_quest_bridge()
	_connect_signals()
	_refresh_quests()


func _init_quest_bridge() -> void:
	_quest_bridge = RefCounted.new()
	_quest_bridge.set_script(load("res://scripts/quest/QuestBridgeWrapper.gd"))

	# Initialize quest system
	var init_result := _call_bridge("quest_init", "{}")
	if init_result.get("success", false):
		print("[QuestScreen] Quest system initialized")
	else:
		push_error("[QuestScreen] Failed to init quest system: %s" % init_result.get("error", "unknown"))


func _connect_signals() -> void:
	if close_button:
		close_button.pressed.connect(_on_close_pressed)
	if refresh_button:
		refresh_button.pressed.connect(_refresh_quests)


func set_close_callback(callback: Callable) -> void:
	_close_callback = callback


func _on_close_pressed() -> void:
	if _close_callback.is_valid():
		_close_callback.call()
	else:
		queue_free()


func _refresh_quests() -> void:
	_update_stats()
	_populate_quest_list(main_quests_list, "Main")
	_populate_quest_list(side_quests_list, "Side")
	_populate_quest_list(daily_quests_list, "Daily")


func _update_stats() -> void:
	var stats := _call_bridge("get_statistics", "")
	if stats.get("success", false):
		var stat_data: Dictionary = stats.get("statistics", {})
		if stats_label:
			stats_label.text = (
				"Total: %d | Active: %d | Completed: %d"
				% [stat_data.get("total", 0), stat_data.get("active", 0), stat_data.get("completed", 0)]
			)


func _populate_quest_list(container: VBoxContainer, quest_type: String) -> void:
	if not container:
		return

	# Clear existing items
	for child in container.get_children():
		child.queue_free()

	# Get quests of this type
	var all_quests := _call_bridge("get_all_quests", "")
	if not all_quests.get("success", false):
		_add_empty_label(container, "Failed to load quests")
		return

	var quests: Array = all_quests.get("quests", [])
	var filtered: Array = []

	for quest in quests:
		if quest.get("quest_type", "") == quest_type:
			filtered.append(quest)

	if filtered.is_empty():
		_add_empty_label(container, "No %s quests available" % quest_type.to_lower())
		return

	# Sort: Active first, then Locked, then Completed
	filtered.sort_custom(
		func(a, b):
			var status_order := {"Active": 0, "Locked": 1, "Completed": 2, "Failed": 3}
			var a_order: int = status_order.get(a.get("status", "Locked"), 99)
			var b_order: int = status_order.get(b.get("status", "Locked"), 99)
			return a_order < b_order
	)

	for quest in filtered:
		var item := _create_quest_item(quest)
		container.add_child(item)


func _add_empty_label(container: VBoxContainer, text: String) -> void:
	var label := Label.new()
	label.text = text
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.add_theme_color_override("font_color", Color(0.6, 0.6, 0.6))
	container.add_child(label)


func _create_quest_item(quest: Dictionary) -> Control:
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(0, QUEST_ITEM_HEIGHT)

	var hbox := HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 12)
	panel.add_child(hbox)

	# Status icon
	var status_icon := Label.new()
	status_icon.custom_minimum_size = Vector2(40, 0)
	status_icon.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	status_icon.vertical_alignment = VERTICAL_ALIGNMENT_CENTER

	var status: String = quest.get("status", "Locked")
	match status:
		"Active":
			status_icon.text = "[!]"
			status_icon.add_theme_color_override("font_color", Color(0.2, 0.8, 0.2))
		"Completed":
			status_icon.text = "[v]"
			status_icon.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
		"Failed":
			status_icon.text = "[x]"
			status_icon.add_theme_color_override("font_color", Color(0.8, 0.3, 0.3))
		_:
			status_icon.text = "[?]"
			status_icon.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	hbox.add_child(status_icon)

	# Quest info
	var info_vbox := VBoxContainer.new()
	info_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_child(info_vbox)

	# Title
	var title_label := Label.new()
	title_label.text = quest.get("title", "Unknown Quest")
	title_label.add_theme_font_size_override("font_size", 16)
	if status == "Completed":
		title_label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
	info_vbox.add_child(title_label)

	# Description
	var desc_label := Label.new()
	desc_label.text = quest.get("description", "")
	desc_label.add_theme_font_size_override("font_size", 12)
	desc_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	desc_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	info_vbox.add_child(desc_label)

	# Progress
	var progress_pct: float = quest.get("progress_percentage", 0.0)
	if status == "Active" and progress_pct > 0:
		var progress_bar := ProgressBar.new()
		progress_bar.value = progress_pct
		progress_bar.custom_minimum_size = Vector2(0, 8)
		progress_bar.show_percentage = false
		info_vbox.add_child(progress_bar)

	# Rewards Display (Enhanced - Phase 4)
	var rewards: Dictionary = quest.get("rewards", {})
	var reward_container := VBoxContainer.new()
	reward_container.custom_minimum_size = Vector2(120, 0)

	# XP Reward
	var xp: int = rewards.get("xp", 0)
	if xp > 0:
		var xp_label := Label.new()
		xp_label.text = "⭐ +%d XP" % xp
		xp_label.add_theme_font_size_override("font_size", 14)
		xp_label.add_theme_color_override("font_color", Color(1.0, 0.84, 0.0))
		xp_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		reward_container.add_child(xp_label)

	# Attribute Rewards
	var attributes: Dictionary = rewards.get("attributes", {})
	if not attributes.is_empty():
		for attr_name in attributes.keys():
			var attr_value: int = attributes[attr_name]
			if attr_value != 0:
				var attr_label := Label.new()
				var sign: String = "+" if attr_value > 0 else ""
				attr_label.text = "🎯 %s%d %s" % [sign, attr_value, attr_name.capitalize()]
				attr_label.add_theme_font_size_override("font_size", 13)
				attr_label.add_theme_color_override("font_color", Color(0.5, 0.9, 1.0))
				attr_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
				reward_container.add_child(attr_label)

	# Item Rewards
	var items: Array = rewards.get("items", [])
	if not items.is_empty():
		for item in items:
			var item_label := Label.new()
			var item_name: String = item if item is String else item.get("name", "Item")
			item_label.text = "🎴 %s" % item_name
			item_label.add_theme_font_size_override("font_size", 13)
			item_label.add_theme_color_override("font_color", Color(0.9, 0.7, 1.0))
			item_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
			reward_container.add_child(item_label)

	# Currency Reward (if exists)
	var currency: int = rewards.get("currency", 0)
	if currency > 0:
		var currency_label := Label.new()
		currency_label.text = "💰 +%d" % currency
		currency_label.add_theme_font_size_override("font_size", 13)
		currency_label.add_theme_color_override("font_color", Color(0.2, 1.0, 0.2))
		currency_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		reward_container.add_child(currency_label)

	# Add reward container to hbox if any rewards exist
	if reward_container.get_child_count() > 0:
		hbox.add_child(reward_container)

	# Store quest_id in metadata
	panel.set_meta("quest_id", quest.get("id", ""))
	panel.set_meta("quest_status", status)

	# Make clickable if active (to show details)
	panel.gui_input.connect(_on_quest_item_input.bind(panel))

	return panel


func _on_quest_item_input(event: InputEvent, panel: Control) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var quest_id: String = panel.get_meta("quest_id", "")
		var status: String = panel.get_meta("quest_status", "")

		if quest_id.is_empty():
			return

		if status == "Active":
			# Show quest details or activate
			_show_quest_details(quest_id)
		elif status == "Locked":
			# Try to unlock
			_try_unlock_quest(quest_id)


func _show_quest_details(quest_id: String) -> void:
	var result := _call_bridge("get_quest", quest_id)
	if not result.get("success", false):
		print("[QuestScreen] Failed to get quest details: %s" % quest_id)
		return

	var quest: Dictionary = result.get("quest", {})
	var objectives: Array = quest.get("objectives", [])

	# Simple dialog showing objectives
	var dialog := AcceptDialog.new()
	dialog.title = quest.get("title", "Quest")

	var text: String = str(quest.get("description", "")) + "\n\nObjectives:\n"
	for obj in objectives:
		var status_mark: String = "[v]" if obj.get("is_complete", false) else "[ ]"
		text += (
			"%s %s (%d/%d)\n"
			% [
				status_mark,
				str(obj.get("description", "")),
				int(obj.get("current_value", 0)),
				int(obj.get("target_value", 1))
			]
		)

	dialog.dialog_text = text
	add_child(dialog)
	dialog.popup_centered()
	dialog.confirmed.connect(func(): dialog.queue_free())


func _try_unlock_quest(quest_id: String) -> void:
	# For now, just show that it's locked
	var dialog := AcceptDialog.new()
	dialog.title = "Quest Locked"
	dialog.dialog_text = "This quest is not yet available.\nComplete prerequisites to unlock."
	add_child(dialog)
	dialog.popup_centered()
	dialog.confirmed.connect(func(): dialog.queue_free())


func _call_bridge(method: String, arg: String) -> Dictionary:
	if not _quest_bridge:
		return {"success": false, "error": "No bridge"}

	var result_str: String = ""
	match method:
		"quest_init":
			result_str = _quest_bridge.quest_init(arg)
		"get_all_quests":
			result_str = _quest_bridge.get_all_quests()
		"get_statistics":
			result_str = _quest_bridge.get_statistics()
		"get_quest":
			result_str = _quest_bridge.get_quest(arg)
		"activate_quest":
			result_str = _quest_bridge.activate_quest(arg)
		"auto_unlock":
			result_str = _quest_bridge.auto_unlock(1, "")
		_:
			return {"success": false, "error": "Unknown method"}

	var json := JSON.new()
	var err := json.parse(result_str)
	if err != OK:
		return {"success": false, "error": "JSON parse error"}
	return json.get_data()


## Called externally when a training is completed
func on_training_completed() -> void:
	var result := _call_bridge("update_all_by_type", "train,1")
	if result.get("success", false):
		var completed: Array = result.get("completed_quests", [])
		for quest_id in completed:
			quest_completed.emit(quest_id)
			show_quest_completion_notification(quest_id)  # Phase 4
		_refresh_quests()


## Called externally when a match is won
func on_match_won() -> void:
	var result := _call_bridge("update_all_by_type", "win,1")
	if result.get("success", false):
		var completed: Array = result.get("completed_quests", [])
		for quest_id in completed:
			quest_completed.emit(quest_id)
			show_quest_completion_notification(quest_id)  # Phase 4
		_refresh_quests()


## Called externally when a match is lost (Phase 4)
func on_match_lost() -> void:
	var result := _call_bridge("update_all_by_type", "loss,1")
	if result.get("success", false):
		var completed: Array = result.get("completed_quests", [])
		for quest_id in completed:
			quest_completed.emit(quest_id)
			show_quest_completion_notification(quest_id)  # Phase 4
		_refresh_quests()


## Called externally when a week advances (Phase 4)
func on_week_advanced(week: int) -> void:
	var result := _call_bridge("update_all_by_type", "week,%d" % week)
	if result.get("success", false):
		var completed: Array = result.get("completed_quests", [])
		for quest_id in completed:
			quest_completed.emit(quest_id)
			show_quest_completion_notification(quest_id)  # Phase 4
		_refresh_quests()


## Called externally when player attribute reaches milestone (Phase 4)
## @param attribute: String - Attribute name (e.g., "passing", "speed")
## @param value: int - New attribute value
func on_attribute_milestone(attribute: String, value: int) -> void:
	var result := _call_bridge("update_all_by_type", "attribute_%s,%d" % [attribute, value])
	if result.get("success", false):
		var completed: Array = result.get("completed_quests", [])
		for quest_id in completed:
			quest_completed.emit(quest_id)
			show_quest_completion_notification(quest_id)  # Phase 4
		_refresh_quests()


# ============================================
# Quest Notification System (Phase 4)
# ============================================


## Show quest update notification
## @param quest_id: String - Quest ID
## @param progress_text: String - Progress description
func show_quest_update_notification(quest_id: String, progress_text: String) -> void:
	# Get quest details
	var quest_result := _call_bridge("get_quest", quest_id)
	if not quest_result.get("success", false):
		return

	var quest: Dictionary = quest_result.get("quest", {})
	var quest_title: String = quest.get("title", quest_id)

	# Create and show notification with safety checks
	if not is_inside_tree():
		push_warning("[QuestScreen] Cannot show notification - not in tree")
		return

	var notification := QUEST_NOTIFICATION_SCENE.instantiate()
	var root_canvas = get_tree().root.get_node_or_null("CanvasLayer")
	if not root_canvas:
		root_canvas = get_tree().root

	root_canvas.add_child(notification)
	notification.z_index = 2000  # Ensure visibility above other UI
	notification.show_update(quest_title, progress_text)


## Show quest completion notification
## @param quest_id: String - Quest ID
func show_quest_completion_notification(quest_id: String) -> void:
	# Get quest details
	var quest_result := _call_bridge("get_quest", quest_id)
	if not quest_result.get("success", false):
		return

	var quest: Dictionary = quest_result.get("quest", {})
	var quest_title: String = quest.get("title", quest_id)

	# Create and show notification with safety checks
	if not is_inside_tree():
		push_warning("[QuestScreen] Cannot show notification - not in tree")
		return

	var notification := QUEST_NOTIFICATION_SCENE.instantiate()
	var root_canvas = get_tree().root.get_node_or_null("CanvasLayer")
	if not root_canvas:
		root_canvas = get_tree().root

	root_canvas.add_child(notification)
	notification.z_index = 2000  # Ensure visibility above other UI
	notification.show_completion(quest_title)


## Automatically show notification when quest completes (connected to signal)
func _on_quest_auto_notification(quest_id: String) -> void:
	show_quest_completion_notification(quest_id)
