extends Control
## DeckBuilderScreen - 가챠 카드 덱 편성 화면
## 감독 1 + 코치 3 + 전술 3 = 7슬롯 덱 시스템
##
## 작성일: 2025-11-26
## 참조: 03_tasks.md [2.2] 덱 빌더

signal back_requested
signal deck_saved(deck_id: String)

# ============================================
# UI 노드 참조
# ============================================

@onready var back_button: Button = %BackButton
@onready var title_label: Label = %TitleLabel
@onready var deck_name_edit: LineEdit = %DeckNameEdit
@onready var save_button: Button = %SaveButton

@onready var slots_container: VBoxContainer = %SlotsContainer
@onready var inventory_grid: GridContainer = %InventoryGrid
@onready var filter_container: HBoxContainer = %FilterContainer

@onready var bonus_label: Label = %BonusLabel
@onready var synergy_label: Label = %SynergyLabel

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG_PRIMARY = Color("#0D1117")
const COLOR_BG_SECONDARY = Color("#161B22")
const COLOR_BG_ELEVATED = Color("#21262D")
const COLOR_BG_SLOT = Color("#30363D")
const COLOR_BG_SLOT_EMPTY = Color("#21262D")
const COLOR_ACCENT_PRIMARY = Color("#238636")
const COLOR_ACCENT_SECONDARY = Color("#1F6FEB")
const COLOR_ACCENT_WARNING = Color("#D29922")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")
const COLOR_BORDER = Color("#30363D")

const RARITY_COLORS = {
	1: Color("#6E7681"), 2: Color("#3FB950"), 3: Color("#58A6FF"), 4: Color("#A371F7"), 5: Color("#FFD700")
}

# ============================================
# 덱 구조
# ============================================

const DECK_STRUCTURE = {
	"manager": {"count": 1, "label": "감독"}, "coach": {"count": 3, "label": "코치"}, "tactics": {"count": 3, "label": "전술"}
}

# ============================================
# 상태 변수
# ============================================

var _deck_slots: Dictionary = {"manager": [null], "coach": [null, null, null], "tactics": [null, null, null]}  # 1 slot  # 3 slots  # 3 slots

var _inventory_cards: Array = []
var _filter_type: String = "all"
var _slot_ui_refs: Dictionary = {}  # {type: [slot_panel, ...]}
var _inventory_item_refs: Array = []

var _deck_id: String = ""
var _deck_name: String = "새 덱"

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_connect_signals()
	_setup_ui()
	_create_deck_slots()
	_load_inventory()
	_load_current_deck()
	print("[DeckBuilderScreen] Initialized")


func _connect_signals() -> void:
	if back_button:
		back_button.pressed.connect(_on_back_pressed)
	if save_button:
		save_button.pressed.connect(_on_save_pressed)
	if deck_name_edit:
		deck_name_edit.text_changed.connect(_on_deck_name_changed)


func _setup_ui() -> void:
	if has_node("Background"):
		$Background.color = COLOR_BG_PRIMARY

	if title_label:
		title_label.text = "덱 편성"
		title_label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)

	if deck_name_edit:
		deck_name_edit.text = _deck_name
		deck_name_edit.placeholder_text = "덱 이름 입력..."

	_create_filter_buttons()


# ============================================
# 덱 슬롯 생성
# ============================================


func _create_deck_slots() -> void:
	if not slots_container:
		return

	# 기존 슬롯 제거
	for child in slots_container.get_children():
		child.queue_free()

	_slot_ui_refs.clear()

	# 각 타입별 슬롯 그룹 생성
	for type in ["manager", "coach", "tactics"]:
		var type_info = DECK_STRUCTURE[type]
		var slot_count = type_info["count"]
		var type_label = type_info["label"]

		# 그룹 컨테이너
		var group = VBoxContainer.new()
		group.add_theme_constant_override("separation", 8)
		slots_container.add_child(group)

		# 타입 라벨
		var label = Label.new()
		label.text = "%s (%d)" % [type_label, slot_count]
		label.add_theme_font_size_override("font_size", 14)
		label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
		group.add_child(label)

		# 슬롯 행
		var slot_row = HBoxContainer.new()
		slot_row.add_theme_constant_override("separation", 8)
		group.add_child(slot_row)

		_slot_ui_refs[type] = []

		for i in range(slot_count):
			var slot_panel = _create_slot_panel(type, i)
			slot_row.add_child(slot_panel)
			_slot_ui_refs[type].append(slot_panel)


func _create_slot_panel(slot_type: String, slot_index: int) -> PanelContainer:
	"""개별 슬롯 패널 생성"""
	var panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(100, 120)

	var style = StyleBoxFlat.new()
	style.bg_color = COLOR_BG_SLOT_EMPTY
	style.border_color = COLOR_BORDER
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.border_width_left = 2
	style.border_width_right = 2
	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	panel.add_theme_stylebox_override("panel", style)

	# 내부 컨텐츠
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 8)
	margin.add_theme_constant_override("margin_right", 8)
	margin.add_theme_constant_override("margin_top", 8)
	margin.add_theme_constant_override("margin_bottom", 8)
	panel.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 4)
	vbox.alignment = BoxContainer.ALIGNMENT_CENTER
	margin.add_child(vbox)

	# 빈 슬롯 아이콘
	var empty_icon = Label.new()
	empty_icon.name = "EmptyIcon"
	empty_icon.text = "+"
	empty_icon.add_theme_font_size_override("font_size", 32)
	empty_icon.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	empty_icon.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(empty_icon)

	# 카드 정보 (숨김)
	var card_info = VBoxContainer.new()
	card_info.name = "CardInfo"
	card_info.visible = false
	card_info.add_theme_constant_override("separation", 2)
	vbox.add_child(card_info)

	var rarity_label = Label.new()
	rarity_label.name = "RarityLabel"
	rarity_label.add_theme_font_size_override("font_size", 12)
	rarity_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	card_info.add_child(rarity_label)

	var name_label = Label.new()
	name_label.name = "NameLabel"
	name_label.add_theme_font_size_override("font_size", 11)
	name_label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD
	card_info.add_child(name_label)

	var specialty_label = Label.new()
	specialty_label.name = "SpecialtyLabel"
	specialty_label.add_theme_font_size_override("font_size", 10)
	specialty_label.add_theme_color_override("font_color", COLOR_ACCENT_PRIMARY)
	specialty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	card_info.add_child(specialty_label)

	# 메타데이터 저장
	panel.set_meta("slot_type", slot_type)
	panel.set_meta("slot_index", slot_index)

	# 클릭 이벤트
	panel.gui_input.connect(_on_slot_clicked.bind(slot_type, slot_index))
	panel.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND

	return panel


func _update_slot_display(slot_type: String, slot_index: int) -> void:
	"""슬롯 UI 업데이트"""
	if not _slot_ui_refs.has(slot_type):
		return
	if slot_index >= _slot_ui_refs[slot_type].size():
		return

	var panel = _slot_ui_refs[slot_type][slot_index]
	var card = _deck_slots[slot_type][slot_index]

	var empty_icon = panel.find_child("EmptyIcon", true, false)
	var card_info = panel.find_child("CardInfo", true, false)

	if card == null:
		# 빈 슬롯
		if empty_icon:
			empty_icon.visible = true
		if card_info:
			card_info.visible = false

		var style = panel.get_theme_stylebox("panel") as StyleBoxFlat
		if style:
			style.bg_color = COLOR_BG_SLOT_EMPTY
			style.border_color = COLOR_BORDER
	else:
		# 카드가 있는 슬롯
		if empty_icon:
			empty_icon.visible = false
		if card_info:
			card_info.visible = true

			var rarity = card.get("rarity", 1)
			var rarity_label = card_info.find_child("RarityLabel", true, false)
			if rarity_label:
				rarity_label.text = "★".repeat(rarity)
				rarity_label.add_theme_color_override("font_color", RARITY_COLORS.get(rarity, COLOR_TEXT_PRIMARY))

			var name_label = card_info.find_child("NameLabel", true, false)
			if name_label:
				name_label.text = card.get("name", "Unknown")

			var specialty_label = card_info.find_child("SpecialtyLabel", true, false)
			if specialty_label:
				specialty_label.text = card.get("specialty_name", "")

		var style = panel.get_theme_stylebox("panel") as StyleBoxFlat
		if style:
			style.bg_color = COLOR_BG_SLOT
			style.border_color = RARITY_COLORS.get(card.get("rarity", 1), COLOR_BORDER)


# ============================================
# 인벤토리 로드
# ============================================


func _create_filter_buttons() -> void:
	if not filter_container:
		return

	for child in filter_container.get_children():
		child.queue_free()

	var filters = {"all": "전체", "manager": "감독", "coach": "코치", "tactics": "전술"}

	for key in filters.keys():
		var btn = Button.new()
		btn.text = filters[key]
		btn.custom_minimum_size = Vector2(60, 32)
		btn.pressed.connect(_on_filter_pressed.bind(key))
		btn.add_theme_font_size_override("font_size", 12)
		filter_container.add_child(btn)


func _load_inventory() -> void:
	if not GachaManager:
		_create_mock_inventory()
		return

	var result = GachaManager.get_inventory()
	if result.get("success", false):
		_inventory_cards = result.get("cards", [])
	else:
		_create_mock_inventory()

	_rebuild_inventory_grid()


func _create_mock_inventory() -> void:
	"""테스트용 목업 인벤토리"""
	_inventory_cards = []

	var types = ["manager", "coach", "tactics"]
	var names = {
		"manager": ["김 감독", "박 감독"],
		"coach": ["스피드 코치", "피지컬 코치", "테크닉 코치", "멘탈 코치"],
		"tactics": ["공격 전술", "수비 전술", "역습 전술"]
	}
	var specialties = ["speed", "power", "technical", "mental", "balanced"]
	var specialty_names = {"speed": "스피드", "power": "파워", "technical": "테크닉", "mental": "멘탈", "balanced": "밸런스"}

	for type in types:
		var type_names = names[type]
		for i in range(type_names.size()):
			var specialty = specialties[randi() % specialties.size()]
			var rarity = randi_range(2, 5)

			_inventory_cards.append(
				{
					"id": "card_%s_%d" % [type, i],
					"name": type_names[i],
					"card_type": type,
					"rarity": rarity,
					"level": randi_range(1, 5),
					"specialty": specialty,
					"specialty_name": specialty_names[specialty],
					"bonus_value": 1.0 + (rarity * 0.05)
				}
			)


func _rebuild_inventory_grid() -> void:
	if not inventory_grid:
		return

	# 기존 아이템 제거
	for item in _inventory_item_refs:
		item.queue_free()
	_inventory_item_refs.clear()

  # 필터 적용
  var filtered = _inventory_cards
  if _filter_type != "all":
          filtered = _inventory_cards.filter(
                  func(card):
                          return str(card.get("type", card.get("card_type", ""))).to_lower()
                          == _filter_type
          )

	# 이미 덱에 있는 카드 제외 (동일 ID)
	var deck_ids = _get_all_deck_card_ids()
	filtered = filtered.filter(func(card): return card.get("id", "") not in deck_ids)

	if filtered.is_empty():
		var empty_label = Label.new()
		empty_label.text = "사용 가능한 카드가 없습니다"
		empty_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
		inventory_grid.add_child(empty_label)
		_inventory_item_refs.append(empty_label)
		return

	# 레어도 순 정렬
	filtered.sort_custom(func(a, b): return a.get("rarity", 0) > b.get("rarity", 0))

	# 카드 아이템 생성
	for card in filtered:
		var item = _create_inventory_item(card)
		inventory_grid.add_child(item)
		_inventory_item_refs.append(item)


func _create_inventory_item(card_data: Dictionary) -> Control:
	"""인벤토리 카드 아이템 생성"""
	var rarity = card_data.get("rarity", 1)
  var card_type = str(card_data.get("type", card_data.get("card_type", "coach"))).to_lower()

	var panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(90, 100)

	var style = StyleBoxFlat.new()
	style.bg_color = COLOR_BG_ELEVATED
	style.border_color = RARITY_COLORS.get(rarity, COLOR_BORDER)
	style.border_width_top = 1
	style.border_width_bottom = 1
	style.border_width_left = 1
	style.border_width_right = 1
	style.corner_radius_top_left = 6
	style.corner_radius_top_right = 6
	style.corner_radius_bottom_left = 6
	style.corner_radius_bottom_right = 6
	panel.add_theme_stylebox_override("panel", style)

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 6)
	margin.add_theme_constant_override("margin_right", 6)
	margin.add_theme_constant_override("margin_top", 6)
	margin.add_theme_constant_override("margin_bottom", 6)
	panel.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 2)
	margin.add_child(vbox)

	# 별
	var stars = Label.new()
	stars.text = "★".repeat(rarity)
	stars.add_theme_font_size_override("font_size", 10)
	stars.add_theme_color_override("font_color", RARITY_COLORS.get(rarity, COLOR_TEXT_PRIMARY))
	stars.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(stars)

	# 이름
	var name_lbl = Label.new()
	name_lbl.text = card_data.get("name", "")
	name_lbl.add_theme_font_size_override("font_size", 10)
	name_lbl.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	name_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_lbl.autowrap_mode = TextServer.AUTOWRAP_WORD
	vbox.add_child(name_lbl)

	# 타입
	var type_names = {"manager": "감독", "coach": "코치", "tactics": "전술"}
	var type_lbl = Label.new()
	type_lbl.text = type_names.get(card_type, "")
	type_lbl.add_theme_font_size_override("font_size", 9)
	type_lbl.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	type_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(type_lbl)

	# 클릭 이벤트
	panel.gui_input.connect(_on_inventory_item_clicked.bind(card_data))
	panel.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND

	return panel


func _get_all_deck_card_ids() -> Array:
	var ids = []
	for type in _deck_slots.keys():
		for card in _deck_slots[type]:
			if card != null:
				ids.append(card.get("id", ""))
	return ids


# ============================================
# 덱 로드/저장
# ============================================


func _load_current_deck() -> void:
	"""현재 저장된 덱 로드"""
	if not GachaManager:
		return

	# 기본 덱 ID 사용
	var result = GachaManager.load_deck("default")
	if result.get("success", false):
		var deck_data = result.get("deck", {})
		_deck_id = deck_data.get("deck_id", "default")
		_deck_name = deck_data.get("deck_name", "새 덱")

		if deck_name_edit:
			deck_name_edit.text = _deck_name

		# 슬롯 데이터 로드
		var slots_data = deck_data.get("slots", {})
		for type in _deck_slots.keys():
			var type_cards = slots_data.get(type, [])
			for i in range(_deck_slots[type].size()):
				if i < type_cards.size():
					_deck_slots[type][i] = type_cards[i]
				else:
					_deck_slots[type][i] = null

		_update_all_slots()
		_rebuild_inventory_grid()


func _save_deck() -> void:
	"""현재 덱 저장"""
	var deck_data = {"deck_id": _deck_id if _deck_id else "default", "deck_name": _deck_name, "slots": {}}

 for type in _deck_slots.keys():
          deck_data["slots"][type] = []
          for card in _deck_slots[type]:
                  # Preserve slot positions (keep nulls) for round-trip fidelity.
                  deck_data["slots"][type].append(card)

	if GachaManager:
		var result = GachaManager.save_deck(deck_data)
		if result.get("success", false):
			print("[DeckBuilderScreen] Deck saved: %s" % _deck_name)
			deck_saved.emit(deck_data.get("deck_id", ""))
			_show_notification("덱이 저장되었습니다", COLOR_ACCENT_PRIMARY)
		else:
			_show_notification("저장 실패", COLOR_ACCENT_WARNING)
	else:
		print("[DeckBuilderScreen] (Mock) Deck saved: %s" % _deck_name)
		_show_notification("덱이 저장되었습니다", COLOR_ACCENT_PRIMARY)


func _update_all_slots() -> void:
	for type in _deck_slots.keys():
		for i in range(_deck_slots[type].size()):
			_update_slot_display(type, i)

	_update_bonus_display()


# ============================================
# 보너스 계산
# ============================================


func _update_bonus_display() -> void:
	var total_bonus = 0.0
	var card_count = 0

	for type in _deck_slots.keys():
		for card in _deck_slots[type]:
			if card != null:
				total_bonus += card.get("bonus_value", 1.0) - 1.0
				card_count += 1

	if bonus_label:
		bonus_label.text = "총 보너스: +%.0f%%" % (total_bonus * 100)
		bonus_label.add_theme_color_override(
			"font_color", COLOR_TEXT_PRIMARY if total_bonus > 0 else COLOR_TEXT_SECONDARY
		)

	# 시너지 체크
	_check_synergy()


func _check_synergy() -> void:
	"""시너지 효과 체크"""
	var specialty_count = {}

	for type in _deck_slots.keys():
		for card in _deck_slots[type]:
			if card != null:
				var specialty = card.get("specialty", "")
				if specialty:
					specialty_count[specialty] = specialty_count.get(specialty, 0) + 1

	var synergies = []
	for specialty in specialty_count.keys():
		if specialty_count[specialty] >= 3:
			var specialty_names = {"speed": "스피드", "power": "파워", "technical": "테크닉", "mental": "멘탈", "balanced": "밸런스"}
			synergies.append("%s 시너지 (x%d)" % [specialty_names.get(specialty, specialty), specialty_count[specialty]])

	if synergy_label:
		if synergies.is_empty():
			synergy_label.text = "시너지: 없음"
			synergy_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
		else:
			synergy_label.text = "시너지: " + ", ".join(synergies)
			synergy_label.add_theme_color_override("font_color", COLOR_ACCENT_PRIMARY)


# ============================================
# 이벤트 핸들러
# ============================================


func _on_back_pressed() -> void:
	back_requested.emit()
	var inventory_path = "res://scenes/screens/InventoryScreen.tscn"
	if ResourceLoader.exists(inventory_path):
		get_tree().change_scene_to_file(inventory_path)
	else:
		get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_save_pressed() -> void:
	_save_deck()


func _on_deck_name_changed(new_name: String) -> void:
	_deck_name = new_name


func _on_filter_pressed(filter_key: String) -> void:
	_filter_type = filter_key
	_rebuild_inventory_grid()


func _on_slot_clicked(event: InputEvent, slot_type: String, slot_index: int) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			var current_card = _deck_slots[slot_type][slot_index]
			if current_card != null:
				# 슬롯에서 카드 제거
				_deck_slots[slot_type][slot_index] = null
				_update_slot_display(slot_type, slot_index)
				_rebuild_inventory_grid()
				_update_bonus_display()
				print("[DeckBuilderScreen] Card removed from %s[%d]" % [slot_type, slot_index])


func _on_inventory_item_clicked(event: InputEvent, card_data: Dictionary) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			_try_add_card_to_deck(card_data)


func _try_add_card_to_deck(card_data: Dictionary) -> void:
 """카드를 덱에 추가 시도"""
 var card_type = str(card_data.get("type", card_data.get("card_type", ""))).to_lower()

	if not _deck_slots.has(card_type):
		_show_notification("알 수 없는 카드 타입입니다", COLOR_ACCENT_WARNING)
		return

	# 빈 슬롯 찾기
	for i in range(_deck_slots[card_type].size()):
		if _deck_slots[card_type][i] == null:
			_deck_slots[card_type][i] = card_data
			_update_slot_display(card_type, i)
			_rebuild_inventory_grid()
			_update_bonus_display()
			print("[DeckBuilderScreen] Card added to %s[%d]: %s" % [card_type, i, card_data.get("name", "")])
			return

	# 슬롯이 가득 참
	var type_labels = {"manager": "감독", "coach": "코치", "tactics": "전술"}
	_show_notification("%s 슬롯이 가득 찼습니다" % type_labels.get(card_type, card_type), COLOR_ACCENT_WARNING)


# ============================================
# 알림
# ============================================


func _show_notification(text: String, color: Color = Color.WHITE) -> void:
	var notif = Label.new()
	notif.text = text
	notif.add_theme_font_size_override("font_size", 16)
	notif.add_theme_color_override("font_color", color)
	notif.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	notif.set_anchors_preset(Control.PRESET_CENTER_TOP)
	notif.position.y = 80

	add_child(notif)

	var tween = create_tween()
	tween.tween_interval(1.5)
	tween.tween_property(notif, "modulate:a", 0.0, 0.5)
	tween.tween_callback(notif.queue_free)
