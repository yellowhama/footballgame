extends ConfirmationDialog
## 간단한 덱 편집 팝업 (DeckManager 연동)

signal deck_updated
# Note: CharacterCard has class_name, so it's globally available

var _deck_manager: Node = null
var _available_list: ItemList
var _deck_list: ItemList
var _summary_label: Label
var _add_button: Button
var _remove_button: Button
var _clear_button: Button


func _ready() -> void:
	title = "덱 편집"
	initial_position = Window.WINDOW_INITIAL_POSITION_CENTER_PRIMARY_SCREEN
	size = Vector2i(720, 420)
	get_ok_button().text = "닫기"
	_deck_manager = _resolve_deck_manager()
	_build_ui()
	_connect_signals()
	_refresh_lists()


func _exit_tree() -> void:
	if _deck_manager:
		if _deck_manager.deck_changed.is_connected(_on_deck_changed):
			_deck_manager.deck_changed.disconnect(_on_deck_changed)


func _notification(what: int) -> void:
	if what == NOTIFICATION_VISIBILITY_CHANGED and visible:
		_refresh_lists()


func _resolve_deck_manager() -> Node:
	if typeof(DeckManager) == TYPE_NIL:
		return null
	return DeckManager


func _build_ui() -> void:
	# In Godot 4.x, add children directly to the dialog
	var root := VBoxContainer.new()
	root.custom_minimum_size = Vector2(680, 360)
	root.size_flags_vertical = Control.SIZE_EXPAND_FILL
	root.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	root.add_theme_constant_override("separation", 12)
	add_child(root)

	var info_label := Label.new()
	info_label.text = "덱에 배치할 카드를 선택하세요. 최대 %d장까지 배치할 수 있습니다." % (DeckManager.MAX_DECK_SIZE if _deck_manager else 6)
	root.add_child(info_label)

	var body := HBoxContainer.new()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 16)
	root.add_child(body)

	var available_box := VBoxContainer.new()
	available_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	available_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_child(available_box)

	var available_title := Label.new()
	available_title.text = "보유 카드"
	available_box.add_child(available_title)

	_available_list = ItemList.new()
	_available_list.allow_reselect = true
	_available_list.select_mode = ItemList.SELECT_SINGLE
	_available_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_available_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	available_box.add_child(_available_list)

	var button_box := VBoxContainer.new()
	button_box.alignment = BoxContainer.ALIGNMENT_CENTER
	body.add_child(button_box)

	_add_button = Button.new()
	_add_button.text = "추가 →"
	_add_button.disabled = true
	button_box.add_child(_add_button)

	_remove_button = Button.new()
	_remove_button.text = "← 제거"
	_remove_button.disabled = true
	button_box.add_child(_remove_button)

	_clear_button = Button.new()
	_clear_button.text = "덱 비우기"
	button_box.add_child(_clear_button)

	var deck_box := VBoxContainer.new()
	deck_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	deck_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_child(deck_box)

	var deck_title := Label.new()
	deck_title.text = "현재 덱"
	deck_box.add_child(deck_title)

	_deck_list = ItemList.new()
	_deck_list.allow_reselect = true
	_deck_list.select_mode = ItemList.SELECT_SINGLE
	_deck_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_deck_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	deck_box.add_child(_deck_list)

	_summary_label = Label.new()
	_summary_label.text = "덱 정보 없음"
	root.add_child(_summary_label)


func _connect_signals() -> void:
	if _deck_manager and not _deck_manager.deck_changed.is_connected(_on_deck_changed):
		_deck_manager.deck_changed.connect(_on_deck_changed)
	_available_list.item_selected.connect(_on_available_selected)
	_deck_list.item_selected.connect(_on_deck_selected)
	_add_button.pressed.connect(_on_add_pressed)
	_remove_button.pressed.connect(_on_remove_pressed)
	_clear_button.pressed.connect(_on_clear_pressed)


func _refresh_lists() -> void:
	_populate_available_list()
	_populate_deck_list()
	_update_summary()
	_update_button_states()


func _populate_available_list() -> void:
	_available_list.clear()
	if not _deck_manager or not _deck_manager.has_method("get_available_cards"):
		_available_list.add_item("DeckManager가 활성화되지 않았습니다.")
		_available_list.disabled = true
		return
	_available_list.disabled = false
	var cards: Array = _deck_manager.get_available_cards()
	for card in cards:
		if card == null:
			continue
		var label := "%s (%s)" % [card.character_name, card.rarity]
		var idx := _available_list.add_item(label)
		_available_list.set_item_metadata(idx, card)


func _populate_deck_list() -> void:
	_deck_list.clear()
	if not _deck_manager:
		return
	for card in _deck_manager.current_deck:
		var label := "%s · %s" % [card.character_name, card.character_type.capitalize()]
		var idx := _deck_list.add_item(label)
		_deck_list.set_item_metadata(idx, card)


func _update_summary() -> void:
	if _deck_manager and _deck_manager.has_method("get_deck_summary"):
		_summary_label.text = _deck_manager.get_deck_summary()
	else:
		_summary_label.text = "덱 정보 없음"


func _update_button_states() -> void:
	var have_selection := not _available_list.get_selected_items().is_empty()
	_add_button.disabled = not have_selection or not _deck_manager or _deck_manager.is_deck_full()
	var deck_selected := not _deck_list.get_selected_items().is_empty()
	_remove_button.disabled = not deck_selected or not _deck_manager


func _on_available_selected(_index: int) -> void:
	_update_button_states()


func _on_deck_selected(_index: int) -> void:
	_update_button_states()


func _on_add_pressed() -> void:
	if not _deck_manager:
		return
	var selected := _available_list.get_selected_items()
	if selected.is_empty():
		return
	var meta = _available_list.get_item_metadata(selected[0])
	if meta:
		if _deck_manager.add_card_to_deck(meta):
			deck_updated.emit()
	_refresh_lists()


func _on_remove_pressed() -> void:
	if not _deck_manager:
		return
	var selected := _deck_list.get_selected_items()
	if selected.is_empty():
		return
	var meta = _deck_list.get_item_metadata(selected[0])
	if meta and meta is CharacterCard:
		if _deck_manager.remove_card_from_deck(meta.character_id):
			deck_updated.emit()
	_refresh_lists()


func _on_clear_pressed() -> void:
	if not _deck_manager:
		return
	_deck_manager.clear_deck()
	deck_updated.emit()
	_refresh_lists()


func _on_deck_changed(_deck) -> void:
	_refresh_lists()
