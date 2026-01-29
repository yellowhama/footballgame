extends Control
## GachaScreen - 가챠 메인 화면
## GachaManager 연동 및 카드 뽑기 UI
##
## 작성일: 2025-11-26
## 참조: 04_ui_design_system.md, 10_gacha_inventory_flow.md

signal back_requested

# ============================================
# UI 노드 참조
# ============================================

@onready var back_button: Button = $Header/BackButton
@onready var title_label: Label = $Header/TitleLabel
@onready var pity_label: Label = $Header/PityLabel

@onready var banner_container: Control = $Content/BannerContainer
@onready var banner_name: Label = $Content/BannerContainer/BannerName
@onready var banner_description: Label = $Content/BannerContainer/BannerDescription

@onready var cards_display: Control = $Content/CardsDisplay
@onready var cards_grid: GridContainer = $Content/CardsDisplay/CardsGrid

@onready var draw_single_button: Button = $Footer/DrawButtons/DrawSingleButton
@onready var draw_10x_button: Button = $Footer/DrawButtons/Draw10xButton
@onready var inventory_button: Button = $Footer/ActionButtons/InventoryButton
@onready var deck_builder_button: Button = $Footer/ActionButtons/DeckBuilderButton
@onready var skip_button: Button = $Footer/ActionButtons/SkipButton

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG_PRIMARY = Color("#0D1117")
const COLOR_BG_SECONDARY = Color("#161B22")
const COLOR_BG_ELEVATED = Color("#30363D")
const COLOR_ACCENT_PRIMARY = Color("#238636")
const COLOR_ACCENT_SECONDARY = Color("#1F6FEB")
const COLOR_ACCENT_WARNING = Color("#D29922")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")

# ============================================
# 상태 변수
# ============================================

var _is_drawing: bool = false
var _current_cards: Array = []
var _card_displays: Array[Control] = []
var _flip_index: int = 0

const GachaCardDisplayScene = preload("res://scenes/components/GachaCardDisplay.tscn")
const MainNavBarScene = preload("res://scenes/components/MainNavBar.tscn")

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_connect_signals()
	_setup_ui()
	_update_pity_display()
	_add_navigation_bar()
	print("[GachaScreen] Initialized")


func _add_navigation_bar() -> void:
	if MainNavBarScene:
		var navbar = MainNavBarScene.instantiate()
		add_child(navbar)
		navbar.set_active_tab("gacha")


func _connect_signals() -> void:
	if back_button:
		back_button.pressed.connect(_on_back_pressed)
	if draw_single_button:
		draw_single_button.pressed.connect(_on_draw_single_pressed)
	if draw_10x_button:
		draw_10x_button.pressed.connect(_on_draw_10x_pressed)
	if inventory_button:
		inventory_button.pressed.connect(_on_inventory_pressed)
	if deck_builder_button:
		deck_builder_button.pressed.connect(_on_deck_builder_pressed)
	if skip_button:
		skip_button.pressed.connect(_on_skip_pressed)
		skip_button.visible = false

	# GachaManager 시그널
	if GachaManager:
		GachaManager.gacha_draw_completed.connect(_on_gacha_draw_completed)


func _setup_ui() -> void:
	# 배경색 설정
	if has_node("Background"):
		$Background.color = COLOR_BG_PRIMARY

	# 초기 상태
	_clear_cards_display()


func _input(event: InputEvent) -> void:
	if _is_drawing and event.is_action_pressed("ui_accept"):
		_flip_next_card()


# ============================================
# 상태 업데이트
# ============================================


func _update_pity_display() -> void:
	if not GachaManager:
		return

	var pity_remaining = GachaManager.get_pity_remaining()
	var pity_counter = GachaManager.get_pity_counter()

	if pity_label:
		pity_label.text = "천장까지: %d회" % pity_remaining
		if pity_remaining <= 20:
			pity_label.add_theme_color_override("font_color", COLOR_ACCENT_WARNING)
		else:
			pity_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)


func _update_banner_display() -> void:
	if banner_name:
		banner_name.text = "일반 가챠"
	if banner_description:
		banner_description.text = "감독, 코치, 전술 카드를 뽑을 수 있습니다"


func _set_buttons_enabled(enabled: bool) -> void:
	if draw_single_button:
		draw_single_button.disabled = not enabled
	if draw_10x_button:
		draw_10x_button.disabled = not enabled
	if skip_button:
		skip_button.visible = not enabled


# ============================================
# 가챠 실행
# ============================================


func _on_draw_single_pressed() -> void:
	if _is_drawing:
		return

	print("[GachaScreen] Drawing single card...")
	_is_drawing = true
	_set_buttons_enabled(false)
	_clear_cards_display()

	if GachaManager:
		var result = GachaManager.draw_single()
		if result.get("success", false):
			_display_single_card(result.get("card", {}))
		else:
			_show_error(result.get("error", "뽑기 실패"))
			_is_drawing = false
			_set_buttons_enabled(true)


func _on_draw_10x_pressed() -> void:
	if _is_drawing:
		return

	print("[GachaScreen] Drawing 10x cards...")
	_is_drawing = true
	_set_buttons_enabled(false)
	_clear_cards_display()

	if GachaManager:
		var result = GachaManager.draw_10x()
		if result.get("success", false):
			_display_multiple_cards(result.get("cards", []))
		else:
			_show_error(result.get("error", "뽑기 실패"))
			_is_drawing = false
			_set_buttons_enabled(true)


# ============================================
# 카드 표시
# ============================================


func _clear_cards_display() -> void:
	for card_display in _card_displays:
		card_display.queue_free()
	_card_displays.clear()
	_current_cards.clear()
	_flip_index = 0


func _display_single_card(card: Dictionary) -> void:
	_current_cards = [card]

	var card_display = GachaCardDisplayScene.instantiate()
	cards_grid.add_child(card_display)
	_card_displays.append(card_display)

	card_display.setup(card)
	card_display.flip_completed.connect(_on_card_flip_completed)

	# 중앙 배치를 위한 설정
	cards_grid.columns = 1

	# 자동 뒤집기
	await get_tree().create_timer(0.5).timeout
	card_display.flip_card()


func _display_multiple_cards(cards: Array) -> void:
	_current_cards = cards
	_flip_index = 0

	# 5열 그리드 (10장 → 2행)
	cards_grid.columns = 5

	for i in range(cards.size()):
		var card = cards[i]
		var card_display = GachaCardDisplayScene.instantiate()
		cards_grid.add_child(card_display)
		_card_displays.append(card_display)

		card_display.setup(card)
		card_display.flip_completed.connect(_on_card_flip_completed)

	# 순차 뒤집기 시작
	await get_tree().create_timer(0.3).timeout
	_flip_next_card()


func _flip_next_card() -> void:
	if _flip_index >= _card_displays.size():
		_finish_drawing()
		return

	var card_display = _card_displays[_flip_index]
	card_display.flip_card()
	_flip_index += 1


func _flip_all_remaining() -> void:
	"""남은 카드 모두 즉시 뒤집기"""
	for i in range(_flip_index, _card_displays.size()):
		var card_data = _current_cards[i]
		_card_displays[i].show_card_instant(card_data)

	_flip_index = _card_displays.size()
	_finish_drawing()


func _finish_drawing() -> void:
	_is_drawing = false
	_set_buttons_enabled(true)
	_update_pity_display()
	print("[GachaScreen] Drawing finished")


# ============================================
# 이벤트 핸들러
# ============================================


func _on_back_pressed() -> void:
	if _is_drawing:
		# 뽑기 중에는 확인 후 나가기
		_flip_all_remaining()

	back_requested.emit()
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_inventory_pressed() -> void:
	print("[GachaScreen] Open inventory")
	var inventory_path = "res://scenes/screens/InventoryScreen.tscn"
	if ResourceLoader.exists(inventory_path):
		get_tree().change_scene_to_file(inventory_path)
	else:
		push_warning("[GachaScreen] Inventory scene not found: %s" % inventory_path)


func _on_deck_builder_pressed() -> void:
	print("[GachaScreen] Open deck builder")
	var deck_builder_path = "res://scenes/screens/DeckBuilderScreen.tscn"
	if ResourceLoader.exists(deck_builder_path):
		get_tree().change_scene_to_file(deck_builder_path)
	else:
		push_warning("[GachaScreen] Deck builder scene not found: %s" % deck_builder_path)


func _on_skip_pressed() -> void:
	"""남은 카드 스킵 (즉시 표시)"""
	if _is_drawing:
		_flip_all_remaining()


func _on_card_flip_completed() -> void:
	"""카드 뒤집기 완료 시 다음 카드 뒤집기"""
	if _is_drawing and _flip_index < _card_displays.size():
		await get_tree().create_timer(0.15).timeout
		_flip_next_card()
	elif _flip_index >= _card_displays.size():
		_finish_drawing()


func _on_gacha_draw_completed(result: Dictionary) -> void:
	"""GachaManager에서 뽑기 완료 시그널"""
	_update_pity_display()

	# 결과 요약 출력
	if result.has("card"):
		var card = result.get("card", {})
		print(
			(
				"[GachaScreen] Drew: %s (%s)"
				% [card.get("name", "Unknown"), GachaManager.get_rarity_stars(card.get("rarity", 1))]
			)
		)
	elif result.has("cards"):
		var cards = result.get("cards", [])
		var highest = 1
		for card in cards:
			if card.get("rarity", 1) > highest:
				highest = card.get("rarity", 1)
		print("[GachaScreen] Drew %d cards, highest: %s" % [cards.size(), GachaManager.get_rarity_stars(highest)])


# ============================================
# UI 피드백
# ============================================


func _show_error(message: String) -> void:
	push_warning("[GachaScreen] Error: %s" % message)
	# TODO: 에러 토스트 표시


func _show_result_summary() -> void:
	"""뽑기 결과 요약 팝업"""
	if _current_cards.is_empty():
		return

	var rarity_count = {}
	var new_count = 0

	for card in _current_cards:
		var rarity = card.get("rarity", 1)
		rarity_count[rarity] = rarity_count.get(rarity, 0) + 1
		if card.get("is_new", false):
			new_count += 1

	print("[GachaScreen] Result summary:")
	for rarity in range(5, 0, -1):
		if rarity_count.has(rarity):
			print("  %s: %d장" % [GachaManager.get_rarity_stars(rarity), rarity_count[rarity]])
	print("  NEW: %d장" % new_count)


# ============================================
# 외부 API
# ============================================


func refresh() -> void:
	"""화면 새로고침"""
	_update_pity_display()
	_update_banner_display()
