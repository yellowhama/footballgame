extends Control

# UI 요소들
@onready var back_button: Button = $BottomBar/HBox/BackButton
@onready var next_button: Button = $BottomBar/HBox/NextButton
@onready var random_button: Button = $MainContent/CharacterDisplay/VBox/RandomButton

# 미리보기 영역 버튼들
@onready var head_preview_button: Button = $MainContent/CharacterDisplay/VBox/CharacterViewport/CharacterSprite/Head
@onready var body_preview_button: Button = $MainContent/CharacterDisplay/VBox/CharacterViewport/CharacterSprite/Body
@onready var legs_preview_button: Button = $MainContent/CharacterDisplay/VBox/CharacterViewport/CharacterSprite/Legs

# 디자인 카드 시스템
var design_card_container: VBoxContainer = null
var current_design_category: String = ""

# 캐릭터 외형 데이터
var appearance_data: Dictionary = {}


func _ready():
	print("[CharacterCreateAppearance] Initializing appearance customization...")

	# 버튼 연결
	_connect_buttons()

	# 초기 외형 데이터 설정
	_setup_initial_appearance()

	# 미리보기 업데이트
	_update_appearance_display()

	print("[CharacterCreateAppearance] Ready complete - All buttons connected")


func _connect_buttons():
	"""버튼들 연결"""
	print("[CharacterCreateAppearance] Connecting buttons...")

	if back_button:
		back_button.pressed.connect(_on_back_pressed)
		print("[CharacterCreateAppearance] Back button connected")
	else:
		print("[CharacterCreateAppearance] ERROR: Back button not found!")

	if next_button:
		next_button.pressed.connect(_on_next_pressed)
		print("[CharacterCreateAppearance] Next button connected")
	else:
		print("[CharacterCreateAppearance] ERROR: Next button not found!")

	if random_button:
		random_button.pressed.connect(_on_random_pressed)
		print("[CharacterCreateAppearance] Random button connected")
	else:
		print("[CharacterCreateAppearance] ERROR: Random button not found!")

	# 미리보기 버튼들
	if head_preview_button:
		head_preview_button.pressed.connect(_on_head_preview_pressed)
		print("[CharacterCreateAppearance] Head preview button connected")
	else:
		print("[CharacterCreateAppearance] ERROR: Head preview button not found!")

	if body_preview_button:
		body_preview_button.pressed.connect(_on_body_preview_pressed)
		print("[CharacterCreateAppearance] Body preview button connected")
	else:
		print("[CharacterCreateAppearance] ERROR: Body preview button not found!")

	if legs_preview_button:
		legs_preview_button.pressed.connect(_on_legs_preview_pressed)
		print("[CharacterCreateAppearance] Legs preview button connected")
	else:
		print("[CharacterCreateAppearance] ERROR: Legs preview button not found!")


func _setup_initial_appearance():
	"""초기 외형 데이터 설정"""
	appearance_data = {
		"hair_style": 0,  # 8가지 헤어스타일
		"hair_color": 0,  # 10가지 헤어컬러
		"face_type": 0,  # 6가지 얼굴형
		"uniform": 0,  # 12가지 유니폼
		"uniform_color": 0,  # 8가지 유니폼 색상
		"jersey_number": 9,
		"body_type": 0,  # 5가지 체형
		"shorts": 0,  # 6가지 반바지
		"socks": 0,  # 8가지 양말
		"shoes": 0,  # 10가지 신발
		"shoe_color": 0  # 8가지 신발 색상
	}


func _on_back_pressed():
	print("[CharacterCreateAppearance] Back button pressed")
	get_tree().change_scene_to_file("res://scenes/TitleScreenImproved.tscn")


func _on_next_pressed():
	print("[CharacterCreateAppearance] Next button pressed - Moving to position selection")
	# 외형 데이터를 전역으로 저장
	GlobalCharacterData.set_appearance(appearance_data)

	# 포지션 선택 씬으로 이동
	get_tree().change_scene_to_file("res://scenes/CharacterCreatePosition.tscn")


func _on_random_pressed():
	print("[CharacterCreateAppearance] Random button pressed")
	_generate_random_appearance()


func _generate_random_appearance():
	"""랜덤 외형 생성"""
	appearance_data["hair_style"] = randi() % 8
	appearance_data["hair_color"] = randi() % 10
	appearance_data["face_type"] = randi() % 6
	appearance_data["uniform"] = randi() % 12
	appearance_data["uniform_color"] = randi() % 8
	appearance_data["jersey_number"] = randi() % 99 + 1
	appearance_data["body_type"] = randi() % 5
	appearance_data["shorts"] = randi() % 6
	appearance_data["socks"] = randi() % 8
	appearance_data["shoes"] = randi() % 10
	appearance_data["shoe_color"] = randi() % 8

	_update_appearance_display()
	print("[CharacterCreateAppearance] Random appearance generated: %s" % appearance_data)


# 미리보기 버튼 핸들러들
func _on_head_preview_pressed():
	print("[CharacterCreateAppearance] Head preview button pressed")
	_show_design_cards("head")


func _on_body_preview_pressed():
	print("[CharacterCreateAppearance] Body preview button pressed")
	_show_design_cards("body")


func _on_legs_preview_pressed():
	print("[CharacterCreateAppearance] Legs preview button pressed")
	_show_design_cards("legs")


func _show_design_cards(category: String):
	"""디자인 카드 그리드 표시"""
	print("[CharacterCreateAppearance] Showing design cards for category: %s" % category)

	# 기존 카드 컨테이너 제거
	if design_card_container:
		design_card_container.queue_free()
		design_card_container = null

	# 새 카드 컨테이너 생성
	design_card_container = VBoxContainer.new()
	design_card_container.name = "DesignCardContainer"
	design_card_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	design_card_container.size_flags_vertical = Control.SIZE_EXPAND_FILL

	# 제목 추가
	var title_label = Label.new()
	title_label.text = _get_category_title(category)
	title_label.add_theme_font_size_override("font_size", 24)
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	design_card_container.add_child(title_label)

	# 닫기 버튼 추가
	var close_button = Button.new()
	close_button.text = "닫기"
	close_button.custom_minimum_size = Vector2(100, 40)
	close_button.pressed.connect(_hide_design_cards)
	design_card_container.add_child(close_button)

	# 카드 그리드 생성
	var card_grid = GridContainer.new()
	card_grid.columns = 4
	card_grid.add_theme_constant_override("h_separation", 10)
	card_grid.add_theme_constant_override("v_separation", 10)

	# 카테고리별 디자인 옵션 생성
	var designs = _get_design_options(category)
	for i in range(designs.size()):
		var card = _create_design_card(category, i, designs[i])
		card_grid.add_child(card)

	design_card_container.add_child(card_grid)

	# MainContent에 추가
	var main_content = $MainContent
	main_content.add_child(design_card_container)
	current_design_category = category


func _get_category_title(category: String) -> String:
	match category:
		"head":
			return "머리 디자인 선택"
		"body":
			return "상체 디자인 선택"
		"legs":
			return "하체 디자인 선택"
		_:
			return "디자인 선택"


func _get_design_options(category: String) -> Array:
	match category:
		"head":
			return [
				{"name": "짧은 머리", "icon": "💇", "style": 0},
				{"name": "긴 머리", "icon": "💇‍♀️", "style": 1},
				{"name": "볼드", "icon": "👨‍🦲", "style": 2},
				{"name": "컬리", "icon": "👨‍🦱", "style": 3},
				{"name": "스트레이트", "icon": "👨", "style": 4},
				{"name": "펑키", "icon": "🤘", "style": 5},
				{"name": "클래식", "icon": "👨‍💼", "style": 6},
				{"name": "모히칸", "icon": "💪", "style": 7}
			]
		"body":
			return [
				{"name": "기본 유니폼", "icon": "👕", "style": 0},
				{"name": "클래식 유니폼", "icon": "👔", "style": 1},
				{"name": "모던 유니폼", "icon": "👕", "style": 2},
				{"name": "레트로 유니폼", "icon": "👕", "style": 3},
				{"name": "프리미엄 유니폼", "icon": "👕", "style": 4},
				{"name": "스포츠 유니폼", "icon": "👕", "style": 5},
				{"name": "캐주얼 유니폼", "icon": "👕", "style": 6},
				{"name": "포멀 유니폼", "icon": "👕", "style": 7},
				{"name": "스트리트 유니폼", "icon": "👕", "style": 8},
				{"name": "빈티지 유니폼", "icon": "👕", "style": 9},
				{"name": "미래형 유니폼", "icon": "👕", "style": 10},
				{"name": "레인보우 유니폼", "icon": "👕", "style": 11}
			]
		"legs":
			return [
				{"name": "기본 반바지", "icon": "🩳", "style": 0},
				{"name": "짧은 반바지", "icon": "🩳", "style": 1},
				{"name": "긴 반바지", "icon": "👖", "style": 2},
				{"name": "스포츠 반바지", "icon": "🩳", "style": 3},
				{"name": "캐주얼 반바지", "icon": "🩳", "style": 4},
				{"name": "클래식 반바지", "icon": "🩳", "style": 5}
			]
		_:
			return []


func _create_design_card(category: String, index: int, design_data: Dictionary) -> Button:
	"""디자인 카드 생성"""
	var card = Button.new()
	card.custom_minimum_size = Vector2(120, 120)
	card.text = design_data["icon"] + "\n" + design_data["name"]
	card.add_theme_font_size_override("font_size", 16)
	card.pressed.connect(_on_design_card_selected.bind(category, design_data["style"]))

	# 현재 선택된 스타일인지 확인
	var current_style = 0
	match category:
		"head":
			current_style = appearance_data["hair_style"]
		"body":
			current_style = appearance_data["uniform"]
		"legs":
			current_style = appearance_data["shorts"]

	if design_data["style"] == current_style:
		card.modulate = Color(0.5, 1.0, 0.5, 1.0)  # 녹색으로 표시

	return card


func _on_design_card_selected(category: String, style: int):
	"""디자인 카드 선택"""
	print("[CharacterCreateAppearance] Design card selected - Category: %s, Style: %d" % [category, style])

	# 외형 데이터 업데이트
	match category:
		"head":
			appearance_data["hair_style"] = style
		"body":
			appearance_data["uniform"] = style
		"legs":
			appearance_data["shorts"] = style

	# 미리보기 업데이트
	_update_appearance_display()

	# 카드 컨테이너 숨기기
	_hide_design_cards()


func _hide_design_cards():
	"""디자인 카드 컨테이너 숨기기"""
	if design_card_container:
		design_card_container.queue_free()
		design_card_container = null
		current_design_category = ""


func _update_appearance_display():
	"""외형 변경 시 화면 업데이트"""
	print("[CharacterCreateAppearance] Updating appearance display: %s" % appearance_data)
	# 여기서 실제 캐릭터 모델을 업데이트하는 코드를 추가할 수 있습니다
