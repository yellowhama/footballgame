extends Control
## InventoryScreen - 카드 인벤토리 화면
## 보유 카드 목록, 필터링, 정렬, 상세 보기
##
## 작성일: 2025-11-26
## 참조: 03_tasks.md [2.1.3] 인벤토리 화면

signal back_requested
signal card_selected(card_data: Dictionary)

# ============================================
# UI 노드 참조
# ============================================

@onready var back_button: Button = %BackButton
@onready var title_label: Label = %TitleLabel
@onready var count_label: Label = %CountLabel
@onready var deck_builder_button: Button = %DeckBuilderButton

@onready var filter_container: HBoxContainer = %FilterContainer
@onready var sort_button: Button = %SortButton
@onready var cards_grid: GridContainer = %CardsGrid
@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var empty_label: Label = %EmptyLabel

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG_PRIMARY = Color("#0D1117")
const COLOR_BG_SECONDARY = Color("#161B22")
const COLOR_BG_ELEVATED = Color("#21262D")
const COLOR_BG_CARD = Color("#30363D")
const COLOR_ACCENT_PRIMARY = Color("#238636")
const COLOR_ACCENT_SECONDARY = Color("#1F6FEB")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")
const COLOR_BORDER = Color("#30363D")

# 레어도 색상
const RARITY_COLORS = {
	1: Color("#6E7681"), 2: Color("#3FB950"), 3: Color("#58A6FF"), 4: Color("#A371F7"), 5: Color("#FFD700")
}

# ============================================
# 필터 옵션
# ============================================

const FILTER_OPTIONS = {"all": "전체", "manager": "감독", "coach": "코치", "tactics": "전술"}

const SORT_OPTIONS = {"rarity_desc": "레어도 ↓", "rarity_asc": "레어도 ↑", "name_asc": "이름 ↑", "newest": "최신순"}

# ============================================
# 상태 변수
# ============================================

var _all_cards: Array = []
var _filtered_cards: Array = []
var _current_filter: String = "all"
var _current_sort: String = "rarity_desc"
var _filter_buttons: Dictionary = {}
var _card_items: Array[Control] = []

var _card_detail_popup: Control = null

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_connect_signals()
	_setup_ui()
	_create_filter_buttons()
	_load_inventory()
	print("[InventoryScreen] Initialized")


func _connect_signals() -> void:
	if back_button:
		back_button.pressed.connect(_on_back_pressed)
	if sort_button:
		sort_button.pressed.connect(_on_sort_pressed)
	if deck_builder_button:
		deck_builder_button.pressed.connect(_on_deck_builder_pressed)

	# GachaManager 시그널
	if GachaManager:
		GachaManager.inventory_updated.connect(_on_inventory_updated)


func _setup_ui() -> void:
	# 배경색
	if has_node("Background"):
		$Background.color = COLOR_BG_PRIMARY

	# 타이틀
	if title_label:
		title_label.text = "카드 인벤토리"
		title_label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)

	# 빈 상태 라벨
	if empty_label:
		empty_label.text = "보유한 카드가 없습니다\n가챠에서 카드를 뽑아보세요!"
		empty_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
		empty_label.visible = false

	# 그리드 설정
	if cards_grid:
		cards_grid.columns = 4


func _create_filter_buttons() -> void:
	if not filter_container:
		return

	# 기존 버튼 제거
	for child in filter_container.get_children():
		child.queue_free()

	# 필터 버튼 생성
	for key in FILTER_OPTIONS.keys():
		var btn = Button.new()
		btn.text = FILTER_OPTIONS[key]
		btn.custom_minimum_size = Vector2(70, 36)
		btn.pressed.connect(_on_filter_pressed.bind(key))
		btn.add_theme_font_size_override("font_size", 13)
		filter_container.add_child(btn)
		_filter_buttons[key] = btn

	_update_filter_button_styles()


# ============================================
# 데이터 로드
# ============================================


func _load_inventory() -> void:
	if not GachaManager:
		_show_mock_inventory()
		return

	var result = GachaManager.get_inventory()
	if result.get("success", false):
		_all_cards = result.get("cards", [])
	else:
		# 목업 데이터 사용
		_show_mock_inventory()
		return

	_apply_filter_and_sort()
	_update_count_display()


func _show_mock_inventory() -> void:
	"""테스트용 목업 인벤토리"""
	_all_cards = []

	# 다양한 카드 생성
	var types = ["manager", "coach", "tactics"]
	var names = {
		"manager": ["김 감독", "박 감독", "이 감독"],
		"coach": ["스피드 코치", "피지컬 코치", "테크닉 코치", "멘탈 코치"],
		"tactics": ["공격 전술", "수비 전술", "역습 전술", "점유율 전술"]
	}
	var specialties = ["speed", "power", "technical", "mental", "balanced"]
	var specialty_names = {"speed": "스피드", "power": "파워", "technical": "테크닉", "mental": "멘탈", "balanced": "밸런스"}

	for i in range(15):
		var card_type = types[i % types.size()]
		var type_names = names[card_type]
		var specialty = specialties[randi() % specialties.size()]
		var rarity = randi_range(1, 5)

		_all_cards.append(
			{
				"id": "card_%s_%d" % [card_type, i],
				"name": type_names[i % type_names.size()],
				"card_type": card_type,
				"rarity": rarity,
				"level": randi_range(1, 10),
				"experience": randi() % 100,
				"specialty": specialty,
				"specialty_name": specialty_names[specialty],
				"description": "테스트 카드 설명",
				"bonus_value": 1.0 + (rarity * 0.05)
			}
		)

	_apply_filter_and_sort()
	_update_count_display()


# ============================================
# 필터 & 정렬
# ============================================


func _apply_filter_and_sort() -> void:
  # 필터 적용
  if _current_filter == "all":
          _filtered_cards = _all_cards.duplicate()
  else:
          _filtered_cards = _all_cards.filter(
                  func(card):
                          return str(card.get("type", card.get("card_type", ""))).to_lower()
                          == _current_filter
          )

	# 정렬 적용
	match _current_sort:
		"rarity_desc":
			_filtered_cards.sort_custom(func(a, b): return a.get("rarity", 0) > b.get("rarity", 0))
		"rarity_asc":
			_filtered_cards.sort_custom(func(a, b): return a.get("rarity", 0) < b.get("rarity", 0))
		"name_asc":
			_filtered_cards.sort_custom(func(a, b): return a.get("name", "") < b.get("name", ""))
		"newest":
			# ID 기반 (최신 = 마지막)
			_filtered_cards.reverse()

	_rebuild_grid()


func _update_filter_button_styles() -> void:
	for key in _filter_buttons.keys():
		var btn = _filter_buttons[key] as Button
		if key == _current_filter:
			# 선택됨
			btn.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
			btn.add_theme_stylebox_override("normal", _create_selected_button_style())
		else:
			# 선택 안됨
			btn.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
			btn.remove_theme_stylebox_override("normal")


func _create_selected_button_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = COLOR_ACCENT_PRIMARY
	style.corner_radius_top_left = 4
	style.corner_radius_top_right = 4
	style.corner_radius_bottom_left = 4
	style.corner_radius_bottom_right = 4
	return style


# ============================================
# 그리드 렌더링
# ============================================


func _rebuild_grid() -> void:
	# 기존 아이템 제거
	for item in _card_items:
		item.queue_free()
	_card_items.clear()

	# 빈 상태 체크
	if empty_label:
		empty_label.visible = _filtered_cards.is_empty()

	if _filtered_cards.is_empty():
		return

	# 카드 아이템 생성
	for card in _filtered_cards:
		var card_item = _create_card_item(card)
		cards_grid.add_child(card_item)
		_card_items.append(card_item)


func _create_card_item(card_data: Dictionary) -> Control:
	"""카드 아이템 UI 생성"""
	var rarity = card_data.get("rarity", 1)
  var card_type = str(card_data.get("type", card_data.get("card_type", "coach"))).to_lower()

	# 메인 컨테이너
	var panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(140, 180)

	var style = StyleBoxFlat.new()
	style.bg_color = COLOR_BG_CARD
	style.border_color = RARITY_COLORS.get(rarity, COLOR_BORDER)
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.border_width_left = 2
	style.border_width_right = 2
	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	panel.add_theme_stylebox_override("panel", style)

	# 내부 레이아웃
	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 8)
	panel.add_child(vbox)

	# 마진
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 10)
	margin.add_theme_constant_override("margin_right", 10)
	margin.add_theme_constant_override("margin_top", 10)
	margin.add_theme_constant_override("margin_bottom", 10)
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	margin.size_flags_vertical = Control.SIZE_EXPAND_FILL
	panel.add_child(margin)

	var inner_vbox = VBoxContainer.new()
	inner_vbox.add_theme_constant_override("separation", 6)
	margin.add_child(inner_vbox)

	# 레어도 별
	var stars_label = Label.new()
	stars_label.text = "★".repeat(rarity)
	stars_label.add_theme_font_size_override("font_size", 14)
	stars_label.add_theme_color_override("font_color", RARITY_COLORS.get(rarity, COLOR_TEXT_PRIMARY))
	stars_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	inner_vbox.add_child(stars_label)

	# 카드 이름
	var name_label = Label.new()
	name_label.text = card_data.get("name", "Unknown")
	name_label.add_theme_font_size_override("font_size", 14)
	name_label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD
	inner_vbox.add_child(name_label)

	# 카드 타입
	var type_names = {"manager": "감독", "coach": "코치", "tactics": "전술"}
	var type_label = Label.new()
  type_label.text = type_names.get(card_type, "카드")
	type_label.add_theme_font_size_override("font_size", 12)
	type_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	type_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	inner_vbox.add_child(type_label)

	# 전문분야
	var specialty_label = Label.new()
	specialty_label.text = card_data.get("specialty_name", "")
	specialty_label.add_theme_font_size_override("font_size", 11)
	specialty_label.add_theme_color_override("font_color", COLOR_ACCENT_PRIMARY)
	specialty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	inner_vbox.add_child(specialty_label)

	# 레벨
	var level_label = Label.new()
	level_label.text = "Lv.%d" % card_data.get("level", 1)
	level_label.add_theme_font_size_override("font_size", 11)
	level_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	level_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	inner_vbox.add_child(level_label)

	# 클릭 이벤트
	panel.gui_input.connect(_on_card_item_clicked.bind(card_data))
	panel.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND

	return panel


func _update_count_display() -> void:
	if count_label:
		count_label.text = "%d / %d" % [_filtered_cards.size(), _all_cards.size()]
		count_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)


# ============================================
# 카드 상세 팝업
# ============================================


func _show_card_detail(card_data: Dictionary) -> void:
	"""카드 상세 팝업 표시"""
	if _card_detail_popup:
		_card_detail_popup.queue_free()

	_card_detail_popup = _create_card_detail_popup(card_data)
	add_child(_card_detail_popup)


func _create_card_detail_popup(card_data: Dictionary) -> Control:
	"""카드 상세 팝업 생성"""
	var rarity = card_data.get("rarity", 1)
  var card_type = str(card_data.get("type", card_data.get("card_type", "coach"))).to_lower()
	var type_names = {"manager": "감독", "coach": "코치", "tactics": "전술"}

	# 오버레이
	var overlay = ColorRect.new()
	overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	overlay.color = Color(0, 0, 0, 0.7)
	overlay.gui_input.connect(
		func(event):
			if event is InputEventMouseButton and event.pressed:
				_close_card_detail()
	)

	# 팝업 컨테이너
	var popup_container = CenterContainer.new()
	popup_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	overlay.add_child(popup_container)

	# 팝업 패널
	var panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(320, 420)
	popup_container.add_child(panel)

	var panel_style = StyleBoxFlat.new()
	panel_style.bg_color = COLOR_BG_SECONDARY
	panel_style.border_color = RARITY_COLORS.get(rarity, COLOR_BORDER)
	panel_style.border_width_top = 3
	panel_style.border_width_bottom = 3
	panel_style.border_width_left = 3
	panel_style.border_width_right = 3
	panel_style.corner_radius_top_left = 12
	panel_style.corner_radius_top_right = 12
	panel_style.corner_radius_bottom_left = 12
	panel_style.corner_radius_bottom_right = 12
	panel.add_theme_stylebox_override("panel", panel_style)

	# 내부 마진
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_top", 20)
	margin.add_theme_constant_override("margin_bottom", 20)
	panel.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 16)
	margin.add_child(vbox)

	# 닫기 버튼
	var close_hbox = HBoxContainer.new()
	var spacer = Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	close_hbox.add_child(spacer)

	var close_btn = Button.new()
	close_btn.text = "✕"
	close_btn.custom_minimum_size = Vector2(32, 32)
	close_btn.pressed.connect(_close_card_detail)
	close_hbox.add_child(close_btn)
	vbox.add_child(close_hbox)

	# 레어도 별
	var stars = Label.new()
	stars.text = "★".repeat(rarity)
	stars.add_theme_font_size_override("font_size", 28)
	stars.add_theme_color_override("font_color", RARITY_COLORS.get(rarity, COLOR_TEXT_PRIMARY))
	stars.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(stars)

	# 카드 이름
	var name_lbl = Label.new()
	name_lbl.text = card_data.get("name", "Unknown Card")
	name_lbl.add_theme_font_size_override("font_size", 22)
	name_lbl.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	name_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(name_lbl)

	# 구분선
	var sep = HSeparator.new()
	sep.add_theme_color_override("separation", COLOR_BORDER)
	vbox.add_child(sep)

	# 정보 그리드
	var info_grid = GridContainer.new()
	info_grid.columns = 2
	info_grid.add_theme_constant_override("h_separation", 20)
	info_grid.add_theme_constant_override("v_separation", 12)
	vbox.add_child(info_grid)

	# 정보 항목들
	var info_items = [
              ["종류", type_names.get(card_type, "카드")],
		["전문분야", card_data.get("specialty_name", "-")],
		["레벨", "Lv.%d" % card_data.get("level", 1)],
		["경험치", "%d%%" % card_data.get("experience", 0)],
		["보너스", "+%.0f%%" % ((card_data.get("bonus_value", 1.0) - 1.0) * 100)]
	]

	for item in info_items:
		var key_lbl = Label.new()
		key_lbl.text = item[0]
		key_lbl.add_theme_font_size_override("font_size", 14)
		key_lbl.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
		info_grid.add_child(key_lbl)

		var val_lbl = Label.new()
		val_lbl.text = str(item[1])
		val_lbl.add_theme_font_size_override("font_size", 14)
		val_lbl.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
		info_grid.add_child(val_lbl)

	# 설명
	var desc_lbl = Label.new()
	desc_lbl.text = card_data.get("description", "")
	desc_lbl.add_theme_font_size_override("font_size", 13)
	desc_lbl.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	desc_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	desc_lbl.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(desc_lbl)

	# ID (디버그용)
	var id_lbl = Label.new()
	id_lbl.text = "ID: %s" % card_data.get("id", "")
	id_lbl.add_theme_font_size_override("font_size", 10)
	id_lbl.add_theme_color_override("font_color", Color(COLOR_TEXT_SECONDARY, 0.5))
	id_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(id_lbl)

	return overlay


func _close_card_detail() -> void:
	if _card_detail_popup:
		_card_detail_popup.queue_free()
		_card_detail_popup = null


# ============================================
# 이벤트 핸들러
# ============================================


func _on_back_pressed() -> void:
	back_requested.emit()
	# 가챠 화면으로 돌아가기
	var gacha_path = "res://scenes/screens/GachaScreen.tscn"
	if ResourceLoader.exists(gacha_path):
		get_tree().change_scene_to_file(gacha_path)
	else:
		get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_deck_builder_pressed() -> void:
	var deck_builder_path = "res://scenes/screens/DeckBuilderScreen.tscn"
	if ResourceLoader.exists(deck_builder_path):
		get_tree().change_scene_to_file(deck_builder_path)
	else:
		push_warning("[InventoryScreen] Deck builder scene not found")


func _on_filter_pressed(filter_key: String) -> void:
	if _current_filter == filter_key:
		return

	_current_filter = filter_key
	_update_filter_button_styles()
	_apply_filter_and_sort()
	_update_count_display()
	print("[InventoryScreen] Filter: %s" % filter_key)


func _on_sort_pressed() -> void:
	# 정렬 옵션 순환
	var sort_keys = SORT_OPTIONS.keys()
	var current_idx = sort_keys.find(_current_sort)
	var next_idx = (current_idx + 1) % sort_keys.size()
	_current_sort = sort_keys[next_idx]

	if sort_button:
		sort_button.text = SORT_OPTIONS[_current_sort]

	_apply_filter_and_sort()
	print("[InventoryScreen] Sort: %s" % _current_sort)


func _on_card_item_clicked(event: InputEvent, card_data: Dictionary) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			_show_card_detail(card_data)
			card_selected.emit(card_data)


func _on_inventory_updated(cards: Array) -> void:
	_all_cards = cards
	_apply_filter_and_sort()
	_update_count_display()


# ============================================
# 외부 API
# ============================================


func refresh() -> void:
	_load_inventory()
