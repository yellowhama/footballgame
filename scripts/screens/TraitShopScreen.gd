extends Control
class_name TraitShopScreen

## Trait Shop UI (2025-12-03)
## - Purchase trait packs with coins
## - Daily rotating shop offers
## - Targeted trait purchase by category

# UI References
@onready var back_button = $VBoxContainer/Header/BackButton
@onready var coins_label = $VBoxContainer/Header/CoinsContainer/CoinsLabel
@onready var pack_grid = $VBoxContainer/PackSection/PackGrid
@onready var daily_deals_container = $VBoxContainer/DailyDeals/DealsGrid
@onready var refresh_timer_label = $VBoxContainer/DailyDeals/RefreshTimer

# State
var current_player_id: String = "player_0"
var current_coins: int = 5000  # Starting coins for testing

# Shop configuration
const PACK_PRICES = {
	"basic": 500,  # Bronze 80%, Silver 18%, Gold 2%
	"premium": 1500,  # Bronze 50%, Silver 40%, Gold 10%
	"elite": 3000,  # Bronze 20%, Silver 50%, Gold 30%
	"category": 1000,  # Specific category pack
}

const DAILY_DEAL_DISCOUNT = 0.3  # 30% off

# Daily shop refresh (based on UTC day)
var last_refresh_day: int = -1
var daily_deals: Array = []


func _ready():
	_setup_ui()
	_load_coins()
	_refresh_daily_deals_if_needed()
	_update_display()


func _setup_ui():
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	_create_pack_buttons()


func _create_pack_buttons():
	# Clear existing
	if pack_grid:
		for child in pack_grid.get_children():
			child.queue_free()

	# Basic Pack
	var basic_btn = _create_pack_button(
		"basic", "기본 팩", "🥉 Bronze 80%\n🥈 Silver 18%\n🥇 Gold 2%", PACK_PRICES.basic, Color(0.6, 0.4, 0.2)  # Bronze color
	)
	pack_grid.add_child(basic_btn)

	# Premium Pack
	var premium_btn = _create_pack_button(
		"premium", "프리미엄 팩", "🥉 Bronze 50%\n🥈 Silver 40%\n🥇 Gold 10%", PACK_PRICES.premium, Color(0.7, 0.7, 0.8)  # Silver color
	)
	pack_grid.add_child(premium_btn)

	# Elite Pack
	var elite_btn = _create_pack_button(
		"elite", "엘리트 팩", "🥉 Bronze 20%\n🥈 Silver 50%\n🥇 Gold 30%", PACK_PRICES.elite, Color(1.0, 0.84, 0.0)  # Gold color
	)
	pack_grid.add_child(elite_btn)

	# Category Packs
	var categories = ["슈팅", "패스", "드리블", "수비", "골키퍼"]
	var cat_icons = ["⚽", "📨", "🏃", "🛡️", "🧤"]

	for i in range(categories.size()):
		var cat_btn = _create_pack_button(
			"category_%d" % i,
			"%s %s 팩" % [cat_icons[i], categories[i]],
			"해당 카테고리 특성만 등장\n🥉70% 🥈25% 🥇5%",
			PACK_PRICES.category,
			Color(0.3, 0.5, 0.8)
		)
		pack_grid.add_child(cat_btn)


func _create_pack_button(pack_id: String, title: String, description: String, price: int, color: Color) -> Panel:
	var panel = Panel.new()
	panel.custom_minimum_size = Vector2(200, 180)

	# Style
	var style = StyleBoxFlat.new()
	style.bg_color = color.darkened(0.7)
	style.border_color = color
	style.border_width_left = 2
	style.border_width_right = 2
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.corner_radius_top_left = 10
	style.corner_radius_top_right = 10
	style.corner_radius_bottom_left = 10
	style.corner_radius_bottom_right = 10
	panel.add_theme_stylebox_override("panel", style)

	var vbox = VBoxContainer.new()
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 5)
	panel.add_child(vbox)

	# Margin container for padding
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 10)
	margin.add_theme_constant_override("margin_top", 10)
	margin.add_theme_constant_override("margin_right", 10)
	margin.add_theme_constant_override("margin_bottom", 10)
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	margin.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_child(margin)

	var inner_vbox = VBoxContainer.new()
	margin.add_child(inner_vbox)

	# Title
	var title_label = Label.new()
	title_label.text = title
	title_label.add_theme_font_size_override("font_size", 18)
	title_label.modulate = color
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	inner_vbox.add_child(title_label)

	# Description
	var desc_label = Label.new()
	desc_label.text = description
	desc_label.add_theme_font_size_override("font_size", 12)
	desc_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	inner_vbox.add_child(desc_label)

	# Spacer
	var spacer = Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	inner_vbox.add_child(spacer)

	# Price button
	var buy_btn = Button.new()
	buy_btn.text = "🪙 %d 코인" % price
	buy_btn.add_theme_font_size_override("font_size", 14)
	buy_btn.pressed.connect(_on_pack_purchase.bind(pack_id, price))
	inner_vbox.add_child(buy_btn)

	panel.set_meta("pack_id", pack_id)
	panel.set_meta("price", price)

	return panel


func _load_coins():
	# Load from PlayerState or other system
	if has_node("/root/PlayerState"):
		var ps = get_node("/root/PlayerState")
		if ps.has_method("get_coins"):
			current_coins = ps.get_coins()


func _save_coins():
	if has_node("/root/PlayerState"):
		var ps = get_node("/root/PlayerState")
		if ps.has_method("set_coins"):
			ps.set_coins(current_coins)


func _refresh_daily_deals_if_needed():
	var current_day = Time.get_unix_time_from_system() / 86400

	if current_day != last_refresh_day:
		last_refresh_day = current_day
		_generate_daily_deals()


func _generate_daily_deals():
	daily_deals.clear()

	# Generate 3 random daily deals
	var trait_types = TraitManager.TRAIT_DATA.keys()
	var used_types = []

	for i in range(3):
		# Pick random trait type not already used
		var trait_type = trait_types[randi() % trait_types.size()]
		while trait_type in used_types:
			trait_type = trait_types[randi() % trait_types.size()]
		used_types.append(trait_type)

		# Random tier (weighted toward lower)
		var tier = _roll_deal_tier()

		# Calculate discounted price
		var base_price = _get_trait_base_price(tier)
		var discount_price = int(base_price * (1.0 - DAILY_DEAL_DISCOUNT))

		daily_deals.append(
			{"type": trait_type, "tier": tier, "original_price": base_price, "discount_price": discount_price}
		)


func _roll_deal_tier() -> int:
	var roll = randf()
	if roll < 0.6:
		return TraitManager.TraitTier.BRONZE
	elif roll < 0.9:
		return TraitManager.TraitTier.SILVER
	else:
		return TraitManager.TraitTier.GOLD


func _get_trait_base_price(tier: int) -> int:
	match tier:
		TraitManager.TraitTier.BRONZE:
			return 300
		TraitManager.TraitTier.SILVER:
			return 800
		TraitManager.TraitTier.GOLD:
			return 2500
		_:
			return 500


func _update_display():
	_update_coins_display()
	_update_daily_deals_display()
	_update_refresh_timer()


func _update_coins_display():
	if coins_label:
		coins_label.text = "🪙 %d" % current_coins


func _update_daily_deals_display():
	if not daily_deals_container:
		return

	# Clear existing
	for child in daily_deals_container.get_children():
		child.queue_free()

	for deal in daily_deals:
		var panel = _create_deal_card(deal)
		daily_deals_container.add_child(panel)


func _create_deal_card(deal: Dictionary) -> Panel:
	var panel = Panel.new()
	panel.custom_minimum_size = Vector2(180, 140)

	var display = TraitManager.get_trait_display(deal.type, deal.tier)

	# Style with tier color
	var style = StyleBoxFlat.new()
	style.bg_color = display.tier_color.darkened(0.7)
	style.border_color = display.tier_color
	style.border_width_left = 2
	style.border_width_right = 2
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	panel.add_theme_stylebox_override("panel", style)

	var vbox = VBoxContainer.new()
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	panel.add_child(vbox)

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 8)
	margin.add_theme_constant_override("margin_top", 8)
	margin.add_theme_constant_override("margin_right", 8)
	margin.add_theme_constant_override("margin_bottom", 8)
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	margin.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_child(margin)

	var inner = VBoxContainer.new()
	margin.add_child(inner)

	# Sale badge
	var sale_label = Label.new()
	sale_label.text = "🔥 -30%"
	sale_label.add_theme_font_size_override("font_size", 12)
	sale_label.modulate = Color(1, 0.3, 0.3)
	inner.add_child(sale_label)

	# Trait icon and name
	var header = HBoxContainer.new()
	inner.add_child(header)

	var icon = Label.new()
	icon.text = "%s %s" % [display.tier_icon, display.icon]
	icon.add_theme_font_size_override("font_size", 20)
	header.add_child(icon)

	var name_label = Label.new()
	name_label.text = display.name_ko
	name_label.add_theme_font_size_override("font_size", 14)
	inner.add_child(name_label)

	# Price
	var price_container = HBoxContainer.new()
	inner.add_child(price_container)

	var orig_price = Label.new()
	orig_price.text = "%d" % deal.original_price
	orig_price.add_theme_font_size_override("font_size", 12)
	orig_price.modulate = Color(0.5, 0.5, 0.5)
	# Strikethrough effect would need BBCode
	price_container.add_child(orig_price)

	var arrow = Label.new()
	arrow.text = " → "
	arrow.add_theme_font_size_override("font_size", 12)
	price_container.add_child(arrow)

	var disc_price = Label.new()
	disc_price.text = "🪙 %d" % deal.discount_price
	disc_price.add_theme_font_size_override("font_size", 14)
	disc_price.modulate = Color(0.3, 1, 0.3)
	price_container.add_child(disc_price)

	# Buy button
	var buy_btn = Button.new()
	buy_btn.text = "구매"
	buy_btn.pressed.connect(_on_deal_purchase.bind(deal))
	inner.add_child(buy_btn)

	return panel


func _update_refresh_timer():
	if not refresh_timer_label:
		return

	# Calculate time until next UTC day
	var now = Time.get_unix_time_from_system()
	var next_day = (int(float(now) / 86400.0) + 1) * 86400
	var seconds_left = next_day - now

	var hours = int(float(seconds_left) / 3600.0)
	var minutes = int(fmod(float(seconds_left), 3600.0) / 60.0)

	refresh_timer_label.text = "새로고침: %02d:%02d" % [hours, minutes]


# ============================================================================
# Purchase Logic
# ============================================================================


func _on_pack_purchase(pack_id: String, price: int):
	if current_coins < price:
		_show_toast("코인이 부족합니다!")
		return

	# Deduct coins
	current_coins -= price
	_save_coins()

	# Roll trait
	var trait_result = _roll_pack(pack_id)

	# Add to inventory
	TraitManager.add_trait_to_inventory(current_player_id, trait_result.type, trait_result.tier)

	# Show result
	var display = TraitManager.get_trait_display(trait_result.type, trait_result.tier)
	_show_result_popup(display)

	_update_display()


func _roll_pack(pack_id: String) -> Dictionary:
	var roll = randf()
	var tier: int
	var trait_type: String

	# Determine tier based on pack type
	if pack_id == "basic":
		if roll < 0.80:
			tier = TraitManager.TraitTier.BRONZE
		elif roll < 0.98:
			tier = TraitManager.TraitTier.SILVER
		else:
			tier = TraitManager.TraitTier.GOLD
	elif pack_id == "premium":
		if roll < 0.50:
			tier = TraitManager.TraitTier.BRONZE
		elif roll < 0.90:
			tier = TraitManager.TraitTier.SILVER
		else:
			tier = TraitManager.TraitTier.GOLD
	elif pack_id == "elite":
		if roll < 0.20:
			tier = TraitManager.TraitTier.BRONZE
		elif roll < 0.70:
			tier = TraitManager.TraitTier.SILVER
		else:
			tier = TraitManager.TraitTier.GOLD
	elif pack_id.begins_with("category_"):
		var cat_idx = int(pack_id.split("_")[1])
		# Category-specific roll
		if roll < 0.70:
			tier = TraitManager.TraitTier.BRONZE
		elif roll < 0.95:
			tier = TraitManager.TraitTier.SILVER
		else:
			tier = TraitManager.TraitTier.GOLD

		# Get traits from specific category
		var cat_traits = TraitManager.get_traits_by_category(cat_idx)
		trait_type = cat_traits[randi() % cat_traits.size()]
		return {"type": trait_type, "tier": tier}
	else:
		tier = TraitManager.TraitTier.BRONZE

	# Random trait type (all categories)
	var all_traits = TraitManager.TRAIT_DATA.keys()
	trait_type = all_traits[randi() % all_traits.size()]

	return {"type": trait_type, "tier": tier}


func _on_deal_purchase(deal: Dictionary):
	if current_coins < deal.discount_price:
		_show_toast("코인이 부족합니다!")
		return

	# Deduct coins
	current_coins -= deal.discount_price
	_save_coins()

	# Add specific trait to inventory
	TraitManager.add_trait_to_inventory(current_player_id, deal.type, deal.tier)

	# Remove from daily deals (one-time purchase)
	daily_deals.erase(deal)

	# Show result
	var display = TraitManager.get_trait_display(deal.type, deal.tier)
	_show_result_popup(display)

	_update_display()


func _show_result_popup(display: Dictionary):
	var popup = AcceptDialog.new()
	popup.title = "특성 획득!"
	popup.dialog_text = "%s %s\n%s\n\nTier: %s" % [display.tier_icon, display.icon, display.name_ko, display.tier_name]
	add_child(popup)
	popup.popup_centered()
	popup.confirmed.connect(func(): popup.queue_free())


func _show_toast(message: String):
	if has_node("/root/UIService"):
		get_node("/root/UIService").show_toast(message, 2.0)
	else:
		print("[TraitShopScreen] %s" % message)


func _on_back_pressed():
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


# Update timer every minute
func _process(_delta):
	# Update refresh timer periodically
	if Engine.get_frames_drawn() % 3600 == 0:  # ~every minute at 60fps
		_update_refresh_timer()
