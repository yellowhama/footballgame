extends Control
class_name TraitScreen

const TraitTutorialScript = preload("res://scripts/screens/TraitTutorial.gd")

## Unified Trait System UI (2025-12-03)
## - 4 equipment slots (unlocks at Lv.1, 10, 20, 30)
## - Trait inventory with tier badges
## - Merge system (3 same tier → 1 higher tier)

# UI References
@onready var back_button = $VBoxContainer/Header/BackButton
@onready var help_button = $VBoxContainer/Header/HelpButton
@onready var player_info_label = $VBoxContainer/Header/PlayerInfo
@onready var slots_container = $VBoxContainer/SlotsSection/SlotsGrid
@onready var category_tabs = $VBoxContainer/InventorySection/CategoryTabs
@onready var merge_button = $VBoxContainer/InventorySection/MergeButton
@onready var merge_panel = $MergePanel

# Slot UI elements
var slot_buttons: Array[Button] = []

# State
var current_player_id: String = "player_0"
var selected_slot_index: int = -1
var selected_inventory_trait: Dictionary = {}


func _ready():
	_setup_ui()
	_load_player_data()
	_update_display()
	_show_tutorial_if_first_time()


func _show_tutorial_if_first_time():
	# Check if trait tutorial should be shown
	var should_show = true
	if has_node("/root/SaveManager"):
		var save_manager = get_node("/root/SaveManager")
		if save_manager.has_method("get_flag"):
			should_show = not save_manager.get_flag("trait_tutorial_completed", false)

	if should_show:
		# Load and show TraitTutorial
		if TraitTutorialScript:
			var tutorial = TraitTutorialScript.new()
			tutorial.tutorial_completed.connect(_on_tutorial_completed)
			tutorial.tutorial_skipped.connect(_on_tutorial_skipped)
			add_child(tutorial)


func _on_tutorial_completed():
	print("[TraitScreen] Tutorial completed")


func _on_tutorial_skipped():
	print("[TraitScreen] Tutorial skipped")


func _setup_ui():
	# Create slot buttons
	for i in range(TraitManager.MAX_SLOTS):
		var slot_btn = _create_slot_button(i)
		slots_container.add_child(slot_btn)
		slot_buttons.append(slot_btn)

	# Setup category tabs with trait grids
	_setup_category_tabs()

	# Connect signals
	if back_button:
		back_button.pressed.connect(_on_back_button_pressed)
	if help_button:
		help_button.pressed.connect(_on_help_button_pressed)
	if merge_button:
		merge_button.pressed.connect(_on_merge_button_pressed)

	# Connect to TraitManager signals
	TraitManager.trait_equipped.connect(_on_trait_equipped)
	TraitManager.trait_unequipped.connect(_on_trait_unequipped)
	TraitManager.trait_merged.connect(_on_trait_merged)
	TraitManager.slot_unlocked.connect(_on_slot_unlocked)


func _create_slot_button(index: int) -> Button:
	var btn = Button.new()
	btn.name = "Slot%d" % index
	btn.custom_minimum_size = Vector2(200, 120)
	btn.text = "Slot %d" % (index + 1)
	btn.pressed.connect(_on_slot_pressed.bind(index))
	return btn


func _setup_category_tabs():
	# Clear existing tabs
	for child in category_tabs.get_children():
		child.queue_free()

	# Create tabs for each category
	var categories = [
		TraitManager.TraitCategory.SHOOTING,
		TraitManager.TraitCategory.PASSING,
		TraitManager.TraitCategory.DRIBBLING,
		TraitManager.TraitCategory.DEFENSE,
		TraitManager.TraitCategory.GOALKEEPER
	]

	for category in categories:
		var scroll = ScrollContainer.new()
		scroll.name = TraitManager.CATEGORY_NAMES_KO[category]
		scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL

		var grid = GridContainer.new()
		grid.name = "TraitGrid"
		grid.columns = 2
		grid.add_theme_constant_override("h_separation", 10)
		grid.add_theme_constant_override("v_separation", 10)
		scroll.add_child(grid)

		category_tabs.add_child(scroll)


func _load_player_data():
	# Initialize current player if not exists
	TraitManager.initialize_player(current_player_id, 25)  # Default level 25 for testing


func _update_display():
	_update_player_info()
	_update_slot_display()
	_update_inventory_display()
	_update_merge_button()


func _update_player_info():
	var data = TraitManager.get_player_data(current_player_id)
	var unlocked = TraitManager.get_unlocked_slot_count(current_player_id)
	if player_info_label:
		player_info_label.text = "Lv.%d | Slots: %d/%d" % [data.level, unlocked, TraitManager.MAX_SLOTS]


func _update_slot_display():
	var data = TraitManager.get_player_data(current_player_id)

	for i in range(slot_buttons.size()):
		var btn = slot_buttons[i]
		var is_unlocked = TraitManager.is_slot_unlocked(current_player_id, i)
		var slot_data = data.slots[i]

		btn.disabled = not is_unlocked

		if not is_unlocked:
			btn.text = "🔒 Lv.%d" % TraitManager.SLOT_UNLOCK_LEVELS[i]
			btn.modulate = Color(0.5, 0.5, 0.5, 0.7)
		elif slot_data == null:
			btn.text = "[Empty]\nTap to Equip"
			btn.modulate = Color.WHITE
		else:
			var display = TraitManager.get_trait_display(slot_data.type, slot_data.tier)
			btn.text = "%s %s\n%s" % [display.tier_icon, display.icon, display.name_ko]
			btn.modulate = display.tier_color

		# Highlight selected slot
		if i == selected_slot_index:
			btn.add_theme_stylebox_override("normal", _create_selected_style())
		else:
			btn.remove_theme_stylebox_override("normal")


func _update_inventory_display():
	var data = TraitManager.get_player_data(current_player_id)

	# Get current tab index
	var current_tab = category_tabs.current_tab if category_tabs else 0

	# Update each category grid
	for tab_idx in range(category_tabs.get_tab_count()):
		var scroll = category_tabs.get_child(tab_idx)
		var grid = scroll.get_node_or_null("TraitGrid")
		if not grid:
			continue

		# Clear grid
		for child in grid.get_children():
			child.queue_free()

		# Get traits for this category
		var category = tab_idx  # Categories are indexed 0-4
		var trait_types = TraitManager.get_traits_by_category(category)

		# Group inventory by type
		var inventory_by_type = {}
		for item in data.inventory:
			if not inventory_by_type.has(item.type):
				inventory_by_type[item.type] = []
			inventory_by_type[item.type].append(item)

		# Create cards for each trait type in this category
		for trait_type in trait_types:
			var items = inventory_by_type.get(trait_type, [])
			if items.is_empty():
				continue

			for item in items:
				var card = _create_inventory_card(item)
				grid.add_child(card)


func _create_inventory_card(trait_item: Dictionary) -> Panel:
	var panel = Panel.new()
	panel.custom_minimum_size = Vector2(160, 100)

	var display = TraitManager.get_trait_display(trait_item.type, trait_item.tier)

	var vbox = VBoxContainer.new()
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("margin_left", 8)
	vbox.add_theme_constant_override("margin_top", 8)
	vbox.add_theme_constant_override("margin_right", -8)
	vbox.add_theme_constant_override("margin_bottom", -8)
	panel.add_child(vbox)

	# Header: tier icon + trait icon
	var header = HBoxContainer.new()
	vbox.add_child(header)

	var tier_label = Label.new()
	tier_label.text = display.tier_icon
	tier_label.add_theme_font_size_override("font_size", 24)
	header.add_child(tier_label)

	var icon_label = Label.new()
	icon_label.text = display.icon
	icon_label.add_theme_font_size_override("font_size", 24)
	header.add_child(icon_label)

	# Name
	var name_label = Label.new()
	name_label.text = display.name_ko
	name_label.add_theme_font_size_override("font_size", 16)
	vbox.add_child(name_label)

	# Tier name
	var tier_name = Label.new()
	tier_name.text = display.tier_name
	tier_name.add_theme_font_size_override("font_size", 12)
	tier_name.modulate = display.tier_color
	vbox.add_child(tier_name)

	# Make it clickable
	panel.gui_input.connect(_on_inventory_card_input.bind(trait_item))
	panel.set_meta("trait_item", trait_item)

	return panel


func _create_selected_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.5, 0.8, 0.3)
	style.border_color = Color(0.2, 0.5, 0.8, 1.0)
	style.border_width_left = 3
	style.border_width_right = 3
	style.border_width_top = 3
	style.border_width_bottom = 3
	style.corner_radius_top_left = 5
	style.corner_radius_top_right = 5
	style.corner_radius_bottom_left = 5
	style.corner_radius_bottom_right = 5
	return style


func _update_merge_button():
	var mergeable = TraitManager.get_mergeable_traits(current_player_id)
	if merge_button:
		merge_button.disabled = mergeable.is_empty()
		merge_button.text = "Merge (%d)" % mergeable.size() if not mergeable.is_empty() else "Merge"


# ============================================================================
# Event Handlers
# ============================================================================


func _on_slot_pressed(slot_index: int):
	if not TraitManager.is_slot_unlocked(current_player_id, slot_index):
		_show_toast("Slot unlocks at Lv.%d" % TraitManager.SLOT_UNLOCK_LEVELS[slot_index])
		return

	var data = TraitManager.get_player_data(current_player_id)
	var slot_data = data.slots[slot_index]

	if slot_data != null:
		# Slot has trait - unequip it
		TraitManager.unequip_trait(current_player_id, slot_index)
		selected_slot_index = -1
	else:
		# Empty slot - select it for equipping
		selected_slot_index = slot_index
		_show_toast("Select a trait from inventory")

	_update_display()


func _on_inventory_card_input(event: InputEvent, trait_item: Dictionary):
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		if selected_slot_index >= 0:
			# Equip to selected slot
			TraitManager.equip_trait(current_player_id, selected_slot_index, trait_item.type, trait_item.tier)
			selected_slot_index = -1
			_update_display()
		else:
			# Show trait details
			_show_trait_details(trait_item)


func _on_merge_button_pressed():
	_show_merge_panel()


func _on_back_button_pressed():
	get_tree().change_scene_to_file("res://scenes/StatusScreenImproved.tscn")


func _on_help_button_pressed():
	# Show tutorial (manual trigger)
	if TraitTutorialScript:
		var tutorial = TraitTutorialScript.new()
		tutorial.tutorial_completed.connect(_on_tutorial_completed)
		tutorial.tutorial_skipped.connect(_on_tutorial_skipped)
		add_child(tutorial)


func _on_trait_equipped(player_id: String, slot: int, trait_data: Dictionary):
	if player_id == current_player_id:
		_show_toast("Equipped %s!" % TraitManager.TRAIT_DATA[trait_data.type].name_ko)
		_update_display()


func _on_trait_unequipped(player_id: String, slot: int):
	if player_id == current_player_id:
		_update_display()


func _on_trait_merged(player_id: String, from_tier: int, to_tier: int, trait_type: String):
	if player_id == current_player_id:
		var tier_names = TraitManager.TIER_NAMES_KO
		_show_toast("Merged! %s → %s" % [tier_names[from_tier], tier_names[to_tier]])
		_update_display()


func _on_slot_unlocked(player_id: String, slot: int):
	if player_id == current_player_id:
		_show_toast("Slot %d unlocked!" % (slot + 1))
		_update_display()


# ============================================================================
# Merge Panel
# ============================================================================


func _show_merge_panel():
	var mergeable = TraitManager.get_mergeable_traits(current_player_id)
	if mergeable.is_empty():
		_show_toast("No traits available to merge")
		return

	# Create merge options popup
	var popup = PopupPanel.new()
	popup.size = Vector2(400, 300)
	popup.position = (get_viewport_rect().size - popup.size) / 2

	var vbox = VBoxContainer.new()
	popup.add_child(vbox)

	var title = Label.new()
	title.text = "Select Trait to Merge (3 → 1)"
	title.add_theme_font_size_override("font_size", 20)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(title)

	var scroll = ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(380, 200)
	vbox.add_child(scroll)

	var list = VBoxContainer.new()
	scroll.add_child(list)

	for merge_info in mergeable:
		var btn = Button.new()
		var display = TraitManager.get_trait_display(merge_info.type, merge_info.tier)
		var next_tier = merge_info.tier + 1
		var next_display = TraitManager.get_trait_display(merge_info.type, next_tier)

		btn.text = (
			"%s %s ×%d → %s %s"
			% [display.tier_icon, display.name_ko, merge_info.count, next_display.tier_icon, next_display.name_ko]
		)
		btn.pressed.connect(
			func():
				TraitManager.merge_traits(current_player_id, merge_info.type, merge_info.tier)
				popup.queue_free()
		)
		list.add_child(btn)

	var close_btn = Button.new()
	close_btn.text = "Cancel"
	close_btn.pressed.connect(func(): popup.queue_free())
	vbox.add_child(close_btn)

	add_child(popup)
	popup.popup_centered()


func _show_trait_details(trait_item: Dictionary):
	var display = TraitManager.get_trait_display(trait_item.type, trait_item.tier)

	var stats_text = ""
	for stat_name in display.scaled_stats:
		stats_text += "\n  %s: +%d" % [stat_name, display.scaled_stats[stat_name]]

	var message = (
		"%s %s %s\n\nTier: %s (%s)\nCategory: %s\n\nStat Bonuses:%s\n\nActive Multiplier: ×%.1f"
		% [
			display.tier_icon,
			display.icon,
			display.name_ko,
			display.tier_name,
			"×%.1f" % display.stat_multiplier,
			display.category_name,
			stats_text,
			display.active_multiplier
		]
	)

	_show_modal("Trait Details", message)


func _show_toast(message: String):
	if has_node("/root/UIService"):
		get_node("/root/UIService").show_toast(message, 2.0)
	else:
		print("[TraitScreen] Toast: %s" % message)


func _show_modal(title: String, message: String):
	if has_node("/root/UIService"):
		get_node("/root/UIService").open_modal(title, message, ["OK"])
	else:
		print("[TraitScreen] Modal - %s: %s" % [title, message])


# ============================================================================
# Testing
# ============================================================================


func _input(event):
	# Debug key for testing
	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_T and event.ctrl_pressed:
			TraitManager.create_test_player(current_player_id)
			_update_display()
			print("[TraitScreen] Test player created")
