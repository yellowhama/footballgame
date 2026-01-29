extends Control

# Shop Screen - Gacha and Card Management
class_name ShopScreen

signal gacha_draw_requested(draw_type: String)
signal deck_management_requested
signal inventory_requested

# UI References
@onready var shop_container = $ShopContainer
@onready var banner_display = $ShopContainer/BannerDisplay
@onready var currency_display = $ShopContainer/CurrencyPanel
@onready var draw_buttons = $ShopContainer/DrawButtons

# Current banner info
var current_banner = {"name": "Standard Banner", "pickup_cards": [], "end_date": "2025-10-15"}

# Player currency (will be loaded from OpenFootball)
var player_gems = 3000
var player_tickets = 10

# OpenFootball Integration
var football_simulator = null


func _ready():
	_initialize_openfootball()
	setup_ui()
	connect_signals()
	update_display()
	_load_player_currency()


func _initialize_openfootball():
	"""Initialize OpenFootball FootballSimulator instance"""
	if ClassDB.class_exists("FootballSimulator"):
		football_simulator = ClassDB.instantiate("FootballSimulator")
		print("[ShopScreen] FootballSimulator initialized")
	else:
		print("[ShopScreen] FootballSimulator class not found - using mock data")


func _load_player_currency():
	"""Load player currency from game data"""
	# TODO: Load from actual player data
	# For now using default values
	player_gems = 3000
	player_tickets = 10
	update_display()


func setup_ui():
	# Set up the shop layout
	pass


func connect_signals():
	# Connect button signals
	pass


func update_display():
	# Update currency display
	if currency_display:
		currency_display.get_node("GemsLabel").text = str(player_gems)
		currency_display.get_node("TicketsLabel").text = str(player_tickets)


func _on_single_draw_pressed():
	if player_gems >= 150:
		player_gems -= 150
		update_display()
		_perform_gacha_draw("single")
	else:
		show_insufficient_funds()


func _on_multi_draw_pressed():
	if player_gems >= 1500:
		player_gems -= 1500
		update_display()
		_perform_gacha_draw("multi")
	else:
		show_insufficient_funds()


func _on_ticket_draw_pressed():
	if player_tickets >= 1:
		player_tickets -= 1
		update_display()
		_perform_gacha_draw("ticket")
	else:
		show_insufficient_tickets()


func show_insufficient_funds():
	# Show popup or notification
	print("Not enough gems!")


func show_insufficient_tickets():
	# Show popup or notification
	print("Not enough tickets!")


func _on_back_button_pressed():
	"""Return to main home screen"""
	print("[ShopScreen] Back button pressed")
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_deck_button_pressed():
	"""Open deck management screen"""
	print("[ShopScreen] Deck management requested")
	# TODO: Implement deck management screen
	# For now, show current deck info
	_show_current_deck_info()


func _on_inventory_button_pressed():
	"""Open card inventory screen"""
	print("[ShopScreen] Card inventory requested")
	# TODO: Implement full inventory screen
	# For now, show inventory summary
	_show_inventory_summary()


func _show_current_deck_info():
	"""Show current deck configuration"""
	if not football_simulator:
		print("[ShopScreen] No FootballSimulator - showing mock deck")
		var dialog = AcceptDialog.new()
		dialog.dialog_text = "현재 덱:\n• 매니저: 테스트 매니저\n• 코치 1: 테스트 코치 A\n• 코치 2: 테스트 코치 B\n• 코치 3: 테스트 코치 C"
		dialog.title = "덱 정보"
		add_child(dialog)
		dialog.popup_centered()
		dialog.confirmed.connect(func(): dialog.queue_free())
		return

	var deck_json = football_simulator.call("load_deck_json", "active_deck")
	if deck_json.is_empty():
		print("[ShopScreen] Failed to load deck")
		return

	var json = JSON.new()
	var parse_result = json.parse(deck_json)
	if parse_result != OK:
		print("[ShopScreen] Failed to parse deck response")
		return

	var response = json.data
	if response.has("success") and response.success:
		var deck = response.deck
		var deck_text = "현재 덱:\n\n"

		if deck.has("manager_card") and deck.manager_card:
			deck_text += "매니저: %s\n" % deck.manager_card.name
		else:
			deck_text += "매니저: 없음\n"

		deck_text += "\n코치 카드:\n"
		for i in range(3):
			if i < deck.coach_cards.size() and deck.coach_cards[i]:
				deck_text += "• %s\n" % deck.coach_cards[i].name
			else:
				deck_text += "• 빈 슬롯\n"

		var dialog = AcceptDialog.new()
		dialog.dialog_text = deck_text
		dialog.title = "덱 정보"
		add_child(dialog)
		dialog.popup_centered()
		dialog.confirmed.connect(func(): dialog.queue_free())


func _show_inventory_summary():
	"""Show card inventory summary"""
	if not football_simulator:
		print("[ShopScreen] No FootballSimulator - showing mock inventory")
		var dialog = AcceptDialog.new()
		dialog.dialog_text = "카드 보관함:\n• 매니저 카드: 5개\n• 코치 카드: 12개\n• 전술 카드: 3개\n\n총 20개 카드 보유"
		dialog.title = "카드 보관함"
		add_child(dialog)
		dialog.popup_centered()
		dialog.confirmed.connect(func(): dialog.queue_free())
		return

	var request_data = {}
	var request_json = JSON.stringify(request_data)
	var inventory_json = football_simulator.call("get_card_inventory_json", request_json)

	if inventory_json.is_empty():
		print("[ShopScreen] Failed to get inventory")
		return

	var json = JSON.new()
	var parse_result = json.parse(inventory_json)
	if parse_result != OK:
		print("[ShopScreen] Failed to parse inventory response")
		return

	var response = json.data
	if response.has("success") and response.success:
		var inventory_text = "카드 보관함:\n\n"
		inventory_text += "총 %d / %d개 카드\n\n" % [response.total_count, response.max_capacity]

		var manager_count = 0
		var coach_count = 0

		for card in response.cards:
			if card.card_type == "Manager":
				manager_count += 1
			elif card.card_type == "Coach":
				coach_count += 1

		inventory_text += "• 매니저 카드: %d개\n" % manager_count
		inventory_text += "• 코치 카드: %d개\n" % coach_count

		var dialog = AcceptDialog.new()
		dialog.dialog_text = inventory_text
		dialog.title = "카드 보관함"
		add_child(dialog)
		dialog.popup_centered()
		dialog.confirmed.connect(func(): dialog.queue_free())


func _perform_gacha_draw(draw_type: String):
	"""Perform actual gacha draw using OpenFootball Coach API"""
	if not football_simulator:
		print("[ShopScreen] No FootballSimulator - showing mock cards")
		_show_mock_card_result(draw_type)
		return

	var request_data = {"pool_type": "regular", "seed": null}
	var request_json = JSON.stringify(request_data)

	var response_json = ""
	match draw_type:
		"single", "ticket":
			response_json = football_simulator.call("gacha_draw_single_json", request_json)
		"multi":
			response_json = football_simulator.call("gacha_draw_10x_json", request_json)

	if response_json.is_empty():
		print("[ShopScreen] Failed to get gacha response")
		_show_mock_card_result(draw_type)
		return

	var json = JSON.new()
	var parse_result = json.parse(response_json)

	if parse_result != OK:
		print("[ShopScreen] Failed to parse gacha response")
		_show_mock_card_result(draw_type)
		return

	var response = json.data
	if response.has("success") and response.success:
		_show_gacha_results(response.cards, draw_type)
	else:
		print("[ShopScreen] Gacha draw failed: ", response.get("error", "Unknown error"))
		_show_mock_card_result(draw_type)


func _show_gacha_results(cards: Array, draw_type: String):
	"""Display gacha results to player"""
	print("[ShopScreen] Gacha Results (%s):" % draw_type)
	for i in range(cards.size()):
		var card = cards[i]
		if card.has("Coach"):
			var coach_card = card.Coach
			print("  Card %d: %s (%s) - Level %d" % [i + 1, coach_card.name, coach_card.rarity, coach_card.level])
		elif card.has("Tactics"):
			var tactics_card = card.Tactics
			print("  Card %d: %s (Tactics)" % [i + 1, tactics_card.name])

	# For now, just show a simple popup
	_show_simple_card_popup(cards, draw_type)


func _show_mock_card_result(draw_type: String):
	"""Show mock card results for testing"""
	var card_count = 1 if draw_type != "multi" else 10
	var mock_cards = []

	for i in card_count:
		mock_cards.append(
			{"Coach": {"name": "테스트 코치 %d" % (i + 1), "rarity": "Common", "level": 1, "card_type": "Coach"}}
		)

	_show_gacha_results(mock_cards, draw_type)


func _show_simple_card_popup(cards: Array, draw_type: String):
	"""Simple popup to show gacha results"""
	var popup_text = "가챠 결과 (%s):\n\n" % draw_type

	for i in range(min(cards.size(), 5)):  # Show first 5 cards
		var card = cards[i]
		if card.has("Coach"):
			var coach_card = card.Coach
			popup_text += "• %s (%s)\n" % [coach_card.name, coach_card.rarity]
		elif card.has("Tactics"):
			var tactics_card = card.Tactics
			popup_text += "• %s (전술카드)\n" % tactics_card.name

	if cards.size() > 5:
		popup_text += "... 그리고 %d개 더" % (cards.size() - 5)

	# Create simple dialog
	var dialog = AcceptDialog.new()
	dialog.dialog_text = popup_text
	dialog.title = "가챠 결과"
	add_child(dialog)
	dialog.popup_centered()

	# Clean up after dialog is closed
	dialog.confirmed.connect(func(): dialog.queue_free())
