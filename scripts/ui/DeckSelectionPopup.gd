extends PopupPanel
# 훈련용 덱 선택 팝업

signal deck_selected(deck_id: String)
signal deck_cleared

@onready var deck_list = $VBox/ScrollContainer/DeckList
@onready var confirm_button = $VBox/ButtonContainer/ConfirmButton
@onready var cancel_button = $VBox/ButtonContainer/CancelButton
@onready var clear_button = $VBox/ButtonContainer/ClearButton
@onready var deck_preview = $VBox/DeckPreview

var available_decks = []
var selected_deck_id = null
var selected_deck_data = null


func _ready():
	print("[DeckSelectionPopup] Ready")

	# 스타일 설정
	_setup_styles()

	# 시그널 연결
	_connect_signals()

	# 덱 목록 로드
	_load_available_decks()


func _setup_styles():
	"""팝업 스타일 설정"""
	var panel_style = StyleBoxFlat.new()
	panel_style.bg_color = Color(0.1, 0.1, 0.15, 0.95)
	panel_style.border_color = Color(0.3, 0.3, 0.4)
	panel_style.border_width_left = 2
	panel_style.border_width_right = 2
	panel_style.border_width_top = 2
	panel_style.border_width_bottom = 2
	panel_style.corner_radius_top_left = 12
	panel_style.corner_radius_top_right = 12
	panel_style.corner_radius_bottom_left = 12
	panel_style.corner_radius_bottom_right = 12
	add_theme_stylebox_override("panel", panel_style)

	# 버튼 스타일
	if confirm_button:
		var btn_style = StyleBoxFlat.new()
		btn_style.bg_color = Color(0.2, 0.6, 0.2)
		btn_style.corner_radius_top_left = 4
		btn_style.corner_radius_top_right = 4
		btn_style.corner_radius_bottom_left = 4
		btn_style.corner_radius_bottom_right = 4
		confirm_button.add_theme_stylebox_override("normal", btn_style)
		confirm_button.add_theme_font_size_override("font_size", 16)


func _connect_signals():
	"""시그널 연결"""
	if confirm_button:
		confirm_button.pressed.connect(_on_confirm_pressed)

	if cancel_button:
		cancel_button.pressed.connect(_on_cancel_pressed)

	if clear_button:
		clear_button.pressed.connect(_on_clear_pressed)


func _load_available_decks():
	"""저장된 덱 목록 로드"""
	# 임시 데이터 (실제로는 save 파일이나 API에서 로드)
	available_decks = [
		{
			"id": "deck_speed",
			"name": "스피드 특화 덱",
			"manager": {"name": "스피드 감독", "rarity": 4, "specialty": "Speed"},
			"coaches":
			[
				{"name": "민첩성 코치", "rarity": 3, "specialty": "Speed"},
				{"name": "가속력 코치", "rarity": 3, "specialty": "Speed"},
				{"name": "균형 코치", "rarity": 2, "specialty": "Balanced"}
			],
			"total_bonus": 1.45,
			"synergy": "스피드 시너지 Lv2"
		},
		{
			"id": "deck_technical",
			"name": "기술 특화 덱",
			"manager": {"name": "기술 감독", "rarity": 5, "specialty": "Technical"},
			"coaches":
			[
				{"name": "드리블 코치", "rarity": 3, "specialty": "Technical"},
				{"name": "볼컨트롤 코치", "rarity": 3, "specialty": "Technical"},
				{"name": "패스 코치", "rarity": 3, "specialty": "Technical"}
			],
			"total_bonus": 1.60,
			"synergy": "기술 시너지 Lv3"
		},
		{
			"id": "deck_balanced",
			"name": "밸런스 덱",
			"manager": {"name": "올라운드 감독", "rarity": 3, "specialty": "Balanced"},
			"coaches":
			[
				{"name": "피지컬 코치", "rarity": 2, "specialty": "Power"},
				{"name": "멘탈 코치", "rarity": 2, "specialty": "Mental"},
				{"name": "기술 코치", "rarity": 2, "specialty": "Technical"}
			],
			"total_bonus": 1.25,
			"synergy": "밸런스 보너스"
		}
	]

	# 실제 API 호출 (FootballSimulator 사용 시)
	# if FootballSimulator:
	#     var response = FootballSimulator.get_saved_decks()
	#     available_decks = JSON.parse(response).result.decks

	_display_deck_list()


func _display_deck_list():
	"""덱 목록 표시"""
	# 기존 항목 제거
	for child in deck_list.get_children():
		child.queue_free()

	# 덱별로 항목 생성
	for deck in available_decks:
		var deck_item = _create_deck_item(deck)
		deck_list.add_child(deck_item)


func _create_deck_item(deck: Dictionary) -> Control:
	"""덱 아이템 UI 생성"""
	var item = Panel.new()
	item.custom_minimum_size.y = 100

	# 아이템 스타일
	var item_style = StyleBoxFlat.new()
	item_style.bg_color = Color(0.15, 0.15, 0.2, 0.8)
	item_style.border_color = Color(0.3, 0.3, 0.4)
	item_style.border_width_left = 1
	item_style.border_width_right = 1
	item_style.border_width_top = 1
	item_style.border_width_bottom = 1
	item_style.corner_radius_top_left = 4
	item_style.corner_radius_top_right = 4
	item_style.corner_radius_bottom_left = 4
	item_style.corner_radius_bottom_right = 4
	item.add_theme_stylebox_override("panel", item_style)

	var vbox = VBoxContainer.new()
	item.add_child(vbox)

	# 덱 이름
	var name_label = Label.new()
	name_label.text = deck.name
	name_label.add_theme_font_size_override("font_size", 18)
	name_label.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
	vbox.add_child(name_label)

	# 보너스 표시
	var bonus_label = Label.new()
	var bonus_percent = (deck.total_bonus - 1.0) * 100
	bonus_label.text = "총 보너스: +%.0f%%" % bonus_percent
	bonus_label.add_theme_font_size_override("font_size", 16)

	# 보너스 크기에 따라 색상
	if bonus_percent >= 50:
		bonus_label.add_theme_color_override("font_color", Color(1.0, 0.843, 0.0))  # Gold
	elif bonus_percent >= 30:
		bonus_label.add_theme_color_override("font_color", Color(0.678, 0.847, 0.902))  # Light Blue
	else:
		bonus_label.add_theme_color_override("font_color", Color(0.565, 0.933, 0.565))  # Light Green
	vbox.add_child(bonus_label)

	# 시너지 표시
	if deck.has("synergy"):
		var synergy_label = Label.new()
		synergy_label.text = "시너지: " + deck.synergy
		synergy_label.add_theme_font_size_override("font_size", 14)
		synergy_label.add_theme_color_override("font_color", Color(0.7, 0.7, 1.0))
		vbox.add_child(synergy_label)

	# 카드 구성 표시
	var cards_container = HBoxContainer.new()
	vbox.add_child(cards_container)

	# 매니저 카드
	var manager_icon = _create_card_mini_icon(deck.manager, true)
	cards_container.add_child(manager_icon)

	# 코치 카드들
	for coach in deck.coaches:
		var coach_icon = _create_card_mini_icon(coach, false)
		cards_container.add_child(coach_icon)

	# 클릭 이벤트
	var button = Button.new()
	button.flat = true
	button.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	button.pressed.connect(_on_deck_item_clicked.bind(deck.id, deck))
	item.add_child(button)

	return item


func _create_card_mini_icon(card: Dictionary, is_manager: bool) -> Control:
	"""카드 미니 아이콘 생성"""
	var icon = ColorRect.new()
	icon.custom_minimum_size = Vector2(30, 40)
	icon.color = _get_rarity_color(card.rarity)

	var label = Label.new()
	label.text = "M" if is_manager else "C"
	label.add_theme_font_size_override("font_size", 14)
	label.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	icon.add_child(label)

	# 툴팁
	var stars = ""
	for i in range(card.rarity):
		stars += "⭐"
	icon.tooltip_text = "%s %s\n%s" % [card.name, stars, card.specialty]

	return icon


func _get_rarity_color(rarity: int) -> Color:
	"""레어도별 색상"""
	match rarity:
		1:
			return Color(0.6, 0.6, 0.6)  # Gray
		2:
			return Color(0.4, 0.8, 0.4)  # Green
		3:
			return Color(0.4, 0.4, 0.9)  # Blue
		4:
			return Color(0.7, 0.4, 0.9)  # Purple
		5:
			return Color(0.9, 0.7, 0.1)  # Gold
		_:
			return Color.WHITE


func _on_deck_item_clicked(deck_id: String, deck_data: Dictionary):
	"""덱 아이템 클릭 처리"""
	selected_deck_id = deck_id
	selected_deck_data = deck_data

	# 선택된 덱 하이라이트
	for child in deck_list.get_children():
		var style = child.get_theme_stylebox("panel")
		if style:
			style.border_color = Color(0.3, 0.3, 0.4)
			style.border_width_left = 1
			style.border_width_right = 1
			style.border_width_top = 1
			style.border_width_bottom = 1

	# 선택된 아이템 강조
	# TODO: 선택된 아이템 찾아서 강조

	# 덱 미리보기 업데이트
	_update_deck_preview()

	# 확인 버튼 활성화
	if confirm_button:
		confirm_button.disabled = false


func _update_deck_preview():
	"""덱 미리보기 업데이트"""
	if not deck_preview or not selected_deck_data:
		return

	# 미리보기 내용 업데이트
	deck_preview.text = (
		"""
	선택된 덱: %s
	총 보너스: +%.0f%%
	시너지: %s

	특화 분야: %s
	"""
		% [
			selected_deck_data.name,
			(selected_deck_data.total_bonus - 1.0) * 100,
			selected_deck_data.get("synergy", "없음"),
			selected_deck_data.manager.specialty
		]
	)


func _on_confirm_pressed():
	"""확인 버튼 클릭"""
	if selected_deck_id:
		# OpenFootball API 호출하여 활성 덱 설정
		if ClassDB.class_exists("FootballMatchSimulator"):
			var simulator = ClassDB.instantiate("FootballMatchSimulator")
			var request = {"deck_id": selected_deck_id, "action": "set_active_for_training"}
			simulator.set_training_deck(JSON.stringify(request))

		emit_signal("deck_selected", selected_deck_id)
		print("[DeckSelectionPopup] Deck selected: ", selected_deck_id)
		hide()


func _on_cancel_pressed():
	"""취소 버튼 클릭"""
	hide()


func _on_clear_pressed():
	"""덱 해제 버튼 클릭"""
	# 활성 덱 해제
	if ClassDB.class_exists("FootballMatchSimulator"):
		var simulator = ClassDB.instantiate("FootballMatchSimulator")
		var request = {"action": "clear_active_deck"}
		simulator.clear_training_deck(JSON.stringify(request))

	emit_signal("deck_cleared")
	print("[DeckSelectionPopup] Active deck cleared")
	hide()


func show_popup():
	"""팝업 표시"""
	popup_centered(Vector2(600, 500))
	_load_available_decks()  # 최신 정보 리로드
