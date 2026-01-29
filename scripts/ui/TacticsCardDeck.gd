extends Control

## TacticsCardDeck - Tactical card collection and management UI
## Displays owned tactical cards with level and experience

signal card_selected(card_data: Dictionary)
signal card_equipped(card_id: String)

# Card data
var owned_cards: Array = []
var equipped_cards: Array = []  # Max 2 for combos

# UI References
var card_grid: GridContainer = null
var combo_display: VBoxContainer = null
var detail_panel: Control = null

# Rust Engine
var rust_engine: Node = null

const MAX_EQUIPPED = 2
const CARD_STYLES = {
	"Defensive": {"color": Color(0.3, 0.3, 0.8), "icon": "🛡️"},
	"Balanced": {"color": Color(0.3, 0.7, 0.3), "icon": "⚖️"},
	"Attacking": {"color": Color(0.8, 0.3, 0.3), "icon": "⚔️"},
	"CounterAttack": {"color": Color(0.8, 0.5, 0.2), "icon": "⚡"},
	"Possession": {"color": Color(0.2, 0.6, 0.8), "icon": "🔄"},
	"Pressing": {"color": Color(0.9, 0.2, 0.2), "icon": "🔥"},
	"DirectPlay": {"color": Color(0.6, 0.4, 0.8), "icon": "➡️"},
	"WingPlay": {"color": Color(0.4, 0.8, 0.4), "icon": "↗️"}
}

const TACTICAL_COMBOS = {
	"Total Football": {"cards": ["Possession", "Pressing"], "bonus": "+0.25 팀 조직력", "icon": "🌟"},
	"Park the Bus": {"cards": ["Defensive", "CounterAttack"], "bonus": "+0.3 안정성", "icon": "🚌"},
	"Gegenpress": {"cards": ["Pressing", "DirectPlay"], "bonus": "+0.25 전환 속도", "icon": "💨"},
	"Tiki-Taka": {"cards": ["Possession", "Attacking"], "bonus": "+0.3 패스 네트워크", "icon": "🎯"},
	"Wing Overload": {"cards": ["WingPlay", "Attacking"], "bonus": "+0.25 측면 지배력", "icon": "🚀"}
}


func _ready():
	print("[TacticsCardDeck] Initializing tactical card deck")

	rust_engine = get_node_or_null("/root/FootballRustEngine")

	_build_ui()
	_load_cards()


func _build_ui():
	# Main container
	var main_hbox = HBoxContainer.new()
	main_hbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_hbox.add_theme_constant_override("separation", 20)
	add_child(main_hbox)

	# Left panel - Card grid
	var left_panel = _create_card_grid_panel()
	left_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	main_hbox.add_child(left_panel)

	# Right panel - Equipped cards and combos
	var right_panel = _create_equipped_panel()
	right_panel.custom_minimum_size.x = 350
	main_hbox.add_child(right_panel)


func _create_card_grid_panel() -> Control:
	var panel = PanelContainer.new()

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_top", 20)
	margin.add_theme_constant_override("margin_bottom", 20)
	panel.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 15)
	margin.add_child(vbox)

	# Title
	var title = Label.new()
	title.text = "🃏 전술 카드 덱"
	title.add_theme_font_size_override("font_size", 28)
	vbox.add_child(title)

	# Description
	var desc = Label.new()
	desc.text = "카드를 선택하여 장착하세요. 2장 조합 시 보너스!"
	desc.add_theme_font_size_override("font_size", 14)
	desc.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	vbox.add_child(desc)

	# Scroll container for cards
	var scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_child(scroll)

	# Card grid
	card_grid = GridContainer.new()
	card_grid.columns = 4
	card_grid.add_theme_constant_override("h_separation", 15)
	card_grid.add_theme_constant_override("v_separation", 15)
	scroll.add_child(card_grid)

	return panel


func _create_equipped_panel() -> Control:
	var panel = PanelContainer.new()

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 15)
	margin.add_theme_constant_override("margin_right", 15)
	margin.add_theme_constant_override("margin_top", 15)
	margin.add_theme_constant_override("margin_bottom", 15)
	panel.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 20)
	margin.add_child(vbox)

	# Title
	var title = Label.new()
	title.text = "⚡ 장착된 카드"
	title.add_theme_font_size_override("font_size", 22)
	vbox.add_child(title)

	# Equipped slots
	var equipped_container = VBoxContainer.new()
	equipped_container.name = "EquippedContainer"
	equipped_container.add_theme_constant_override("separation", 10)
	vbox.add_child(equipped_container)

	# Create 2 empty slots
	for i in range(MAX_EQUIPPED):
		var slot = _create_empty_slot(i)
		equipped_container.add_child(slot)

	# Separator
	var sep = HSeparator.new()
	vbox.add_child(sep)

	# Combo display
	var combo_title = Label.new()
	combo_title.text = "🌟 전술 콤보"
	combo_title.add_theme_font_size_override("font_size", 20)
	vbox.add_child(combo_title)

	combo_display = VBoxContainer.new()
	combo_display.name = "ComboDisplay"
	combo_display.add_theme_constant_override("separation", 8)
	vbox.add_child(combo_display)

	var no_combo = Label.new()
	no_combo.name = "NoComboLabel"
	no_combo.text = "카드 2장 장착 시 콤보 발동"
	no_combo.add_theme_font_size_override("font_size", 14)
	no_combo.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
	combo_display.add_child(no_combo)

	return panel


func _create_empty_slot(index: int) -> Control:
	var slot = PanelContainer.new()
	slot.name = "Slot_%d" % index
	slot.custom_minimum_size = Vector2(0, 80)

	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.2, 0.2, 0.5)
	style.border_color = Color(0.4, 0.4, 0.4)
	style.set_border_width_all(2)
	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	slot.add_theme_stylebox_override("panel", style)

	var center = CenterContainer.new()
	slot.add_child(center)

	var label = Label.new()
	label.name = "SlotLabel"
	label.text = "빈 슬롯 %d" % (index + 1)
	label.add_theme_font_size_override("font_size", 16)
	label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
	center.add_child(label)

	return slot


func _create_card_ui(card_data: Dictionary) -> Control:
	var card = Button.new()
	card.custom_minimum_size = Vector2(150, 200)
	card.toggle_mode = true

	var style_id = card_data.get("tactical_style", "Balanced")
	var card_style = CARD_STYLES.get(style_id, CARD_STYLES["Balanced"])

	# Card styling
	var style = StyleBoxFlat.new()
	style.bg_color = card_style.color.darkened(0.3)
	style.border_color = card_style.color
	style.set_border_width_all(3)
	style.corner_radius_top_left = 10
	style.corner_radius_top_right = 10
	style.corner_radius_bottom_left = 10
	style.corner_radius_bottom_right = 10
	card.add_theme_stylebox_override("normal", style)

	var pressed_style = style.duplicate()
	pressed_style.border_color = Color.WHITE
	pressed_style.set_border_width_all(4)
	card.add_theme_stylebox_override("pressed", pressed_style)

	# Card content
	var vbox = VBoxContainer.new()
	vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 5)
	card.add_child(vbox)

	# Icon
	var icon = Label.new()
	icon.text = card_style.icon
	icon.add_theme_font_size_override("font_size", 36)
	icon.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(icon)

	# Name
	var name_label = Label.new()
	name_label.text = _get_korean_style_name(style_id)
	name_label.add_theme_font_size_override("font_size", 16)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(name_label)

	# Level
	var level = card_data.get("level", 1)
	var level_label = Label.new()
	level_label.text = "Lv.%d" % level
	level_label.add_theme_font_size_override("font_size", 20)
	level_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(level_label)

	# Experience bar
	var exp = card_data.get("experience", 0)
	var max_exp = level * 100  # Simple formula
	var exp_bar = ProgressBar.new()
	exp_bar.min_value = 0
	exp_bar.max_value = max_exp
	exp_bar.value = exp
	exp_bar.show_percentage = false
	exp_bar.custom_minimum_size.y = 10
	vbox.add_child(exp_bar)

	# EXP text
	var exp_label = Label.new()
	exp_label.text = "%d/%d" % [exp, max_exp]
	exp_label.add_theme_font_size_override("font_size", 12)
	exp_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(exp_label)

	# Connect signal
	card.pressed.connect(_on_card_pressed.bind(card_data))

	return card


func _get_korean_style_name(style_id: String) -> String:
	match style_id:
		"Defensive":
			return "수비적"
		"Balanced":
			return "균형"
		"Attacking":
			return "공격적"
		"CounterAttack":
			return "역습"
		"Possession":
			return "점유"
		"Pressing":
			return "압박"
		"DirectPlay":
			return "직접 플레이"
		"WingPlay":
			return "측면 플레이"
		_:
			return style_id


func _load_cards():
	# Load cards from Rust or create defaults
	if rust_engine and rust_engine.has_method("get_tactics_cards"):
		var result = rust_engine.get_tactics_cards()
		if result.get("success", false):
			owned_cards = result.get("cards", [])

	# Default cards if none loaded
	if owned_cards.is_empty():
		owned_cards = [
			{"id": "balanced_1", "tactical_style": "Balanced", "level": 1, "experience": 50},
			{"id": "defensive_1", "tactical_style": "Defensive", "level": 1, "experience": 30},
			{"id": "attacking_1", "tactical_style": "Attacking", "level": 2, "experience": 80},
			{"id": "possession_1", "tactical_style": "Possession", "level": 1, "experience": 20},
			{"id": "pressing_1", "tactical_style": "Pressing", "level": 3, "experience": 150},
			{"id": "counter_1", "tactical_style": "CounterAttack", "level": 1, "experience": 0},
			{"id": "direct_1", "tactical_style": "DirectPlay", "level": 2, "experience": 100},
			{"id": "wing_1", "tactical_style": "WingPlay", "level": 1, "experience": 45}
		]

	_refresh_card_grid()


func _refresh_card_grid():
	# Clear existing
	for child in card_grid.get_children():
		child.queue_free()

	# Add cards
	for card_data in owned_cards:
		var card_ui = _create_card_ui(card_data)
		card_grid.add_child(card_ui)


func _on_card_pressed(card_data: Dictionary):
	var card_id = card_data.get("id", "")
	var style = card_data.get("tactical_style", "")

	print("[TacticsCardDeck] Card pressed: %s (%s)" % [card_id, style])

	# Check if already equipped
	var equipped_index = -1
	for i in range(equipped_cards.size()):
		if equipped_cards[i].get("id") == card_id:
			equipped_index = i
			break

	if equipped_index >= 0:
		# Unequip
		equipped_cards.remove_at(equipped_index)
		print("[TacticsCardDeck] Card unequipped")
	else:
		# Equip if slots available
		if equipped_cards.size() < MAX_EQUIPPED:
			equipped_cards.append(card_data)
			print("[TacticsCardDeck] Card equipped")
			card_equipped.emit(card_id)
		else:
			_show_message("슬롯이 가득 찼습니다. 먼저 카드를 해제하세요.")
			return

	_update_equipped_display()
	_check_combos()
	card_selected.emit(card_data)


func _update_equipped_display():
	var equipped_container = get_node_or_null("PanelContainer/MarginContainer/VBoxContainer/EquippedContainer")
	if not equipped_container:
		return

	# Update slots
	for i in range(MAX_EQUIPPED):
		var slot = equipped_container.get_node_or_null("Slot_%d" % i)
		if not slot:
			continue

		var label = slot.get_node_or_null("CenterContainer/SlotLabel")
		if not label:
			continue

		if i < equipped_cards.size():
			var card = equipped_cards[i]
			var style_id = card.get("tactical_style", "")
			var style_info = CARD_STYLES.get(style_id, CARD_STYLES["Balanced"])
			label.text = "%s %s (Lv.%d)" % [style_info.icon, _get_korean_style_name(style_id), card.get("level", 1)]
			label.add_theme_color_override("font_color", style_info.color)
		else:
			label.text = "빈 슬롯 %d" % (i + 1)
			label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))


func _check_combos():
	if not combo_display:
		return

	# Clear combo display
	for child in combo_display.get_children():
		child.queue_free()

	if equipped_cards.size() < 2:
		var no_combo = Label.new()
		no_combo.text = "카드 2장 장착 시 콤보 발동"
		no_combo.add_theme_font_size_override("font_size", 14)
		no_combo.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
		combo_display.add_child(no_combo)
		return

	# Check for matching combos
	var styles = []
	for card in equipped_cards:
		styles.append(card.get("tactical_style", ""))

	var found_combo = false
	for combo_name in TACTICAL_COMBOS:
		var combo = TACTICAL_COMBOS[combo_name]
		var required = combo.cards

		if (required[0] in styles and required[1] in styles) or (required[1] in styles and required[0] in styles):
			# Combo found!
			found_combo = true
			var combo_label = Label.new()
			combo_label.text = "%s %s\n%s" % [combo.icon, combo_name, combo.bonus]
			combo_label.add_theme_font_size_override("font_size", 16)
			combo_label.add_theme_color_override("font_color", Color(1, 0.8, 0.2))
			combo_display.add_child(combo_label)
			break

	if not found_combo:
		var no_match = Label.new()
		no_match.text = "일치하는 콤보 없음"
		no_match.add_theme_font_size_override("font_size", 14)
		no_match.add_theme_color_override("font_color", Color(0.6, 0.6, 0.6))
		combo_display.add_child(no_match)


func _show_message(text: String):
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "전술 카드"
	add_child(popup)
	popup.popup_centered(Vector2(350, 150))
	popup.confirmed.connect(popup.queue_free)


func get_equipped_cards() -> Array:
	return equipped_cards.duplicate()


func get_active_combo() -> String:
	if equipped_cards.size() < 2:
		return ""

	var styles = []
	for card in equipped_cards:
		styles.append(card.get("tactical_style", ""))

	for combo_name in TACTICAL_COMBOS:
		var combo = TACTICAL_COMBOS[combo_name]
		var required = combo.cards
		if required[0] in styles and required[1] in styles:
			return combo_name

	return ""
