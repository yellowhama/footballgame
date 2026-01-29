extends Control
# CharacterCreateImproved - 개선된 캐릭터 생성 화면 (4단계 시스템)

# UI 요소들
@onready var back_button: Button = $BottomBar/HBox/BackButton
@onready var next_button: Button = $BottomBar/HBox/NextButton
@onready var random_button: Button = $MainContent/CharacterDisplay/VBox/RandomButton

# 단계 표시 버튼들
@onready var step1_button: Button = $Header/StepIndicator/Step1
@onready var step2_button: Button = $Header/StepIndicator/Step2
@onready var step3_button: Button = $Header/StepIndicator/Step3
@onready var step4_button: Button = $Header/StepIndicator/Step4

# 메인 컨텐츠 영역들
@onready var main_content: HBoxContainer = $MainContent
@onready var character_display: Panel = $MainContent/CharacterDisplay
@onready var customization_panel: ScrollContainer = $MainContent/CustomizationPanel

# 추가 버튼들 (중복 제거됨)

# 커스터마이징 화살표 버튼들
@onready var hair_style_next: Button = $MainContent/CustomizationPanel/VBox/HeadSection/VBox/HairStyle/NextButton
@onready var hair_color_next: Button = $MainContent/CustomizationPanel/VBox/HeadSection/VBox/HairColor/NextButton
@onready var face_type_next: Button = $MainContent/CustomizationPanel/VBox/HeadSection/VBox/FaceType/NextButton
@onready var uniform_next: Button = $MainContent/CustomizationPanel/VBox/BodySection/VBox/Uniform/NextButton
@onready var uniform_color_next: Button = $MainContent/CustomizationPanel/VBox/BodySection/VBox/UniformColor/NextButton
@onready var body_type_next: Button = $MainContent/CustomizationPanel/VBox/BodySection/VBox/BodyType/NextButton
@onready var shorts_next: Button = $MainContent/CustomizationPanel/VBox/LegSection/VBox/Shorts/NextButton
@onready var socks_next: Button = $MainContent/CustomizationPanel/VBox/LegSection/VBox/Socks/NextButton
@onready var shoes_next: Button = $MainContent/CustomizationPanel/VBox/LegSection/VBox/Shoes/NextButton
@onready var shoe_color_next: Button = $MainContent/CustomizationPanel/VBox/LegSection/VBox/ShoeColor/NextButton

# 미리보기 영역 버튼들
@onready var head_preview_button: Button = $MainContent/CharacterDisplay/VBox/CharacterViewport/CharacterSprite/Head
@onready var body_preview_button: Button = $MainContent/CharacterDisplay/VBox/CharacterViewport/CharacterSprite/Body
@onready var legs_preview_button: Button = $MainContent/CharacterDisplay/VBox/CharacterViewport/CharacterSprite/Legs

# 단계별 UI 컨테이너들 (동적 생성)
var step_containers: Dictionary = {}

# 디자인 카드 시스템
var design_card_container: VBoxContainer = null
var current_design_category: String = ""
var design_cards: Dictionary = {}

# 캐릭터 생성 데이터
var character_data: Dictionary = {}
var current_step: int = 1
var max_steps: int = 4

# 단계별 완료 상태
var step_completed: Dictionary = {1: false, 2: false, 3: false, 4: false}  # 외형  # 정보  # 능력치  # 확인


func _ready():
	print("[CharacterCreateImproved] Initializing 4-step character creation...")

	# ColorSystem 적용
	SceneColorUpdater.apply_color_system_to_scene(self)

	# 반응형 레이아웃 수정
	ResponsiveLayoutFixer.fix_scene_layout(self)

	# 터치 피드백 적용 - TouchFeedback class doesn't exist
	# TODO: Implement touch feedback if needed

	# UI 요소들 찾기
	_find_ui_elements()

	# 버튼 연결
	_connect_buttons()

	# 초기 설정
	_setup_initial_state()

	# 첫 번째 단계 표시
	_show_step(1)
	_update_step_buttons()


func _find_ui_elements():
	# UI 요소들은 @onready로 이미 초기화됨
	pass


func _connect_buttons():
	print("[CharacterCreateImproved] Connecting buttons...")

	# 뒤로가기 버튼
	if back_button:
		print("[CharacterCreateImproved] Connecting back button")
		back_button.pressed.connect(_on_back_pressed)
	else:
		print("[CharacterCreateImproved] ERROR: back_button is null!")

	# 다음 단계 버튼
	if next_button:
		print("[CharacterCreateImproved] Connecting next button")
		next_button.pressed.connect(_on_next_pressed)
	else:
		print("[CharacterCreateImproved] ERROR: next_button is null!")

	# 랜덤 생성 버튼
	if random_button:
		print("[CharacterCreateImproved] Connecting random button")
		random_button.pressed.connect(_on_random_pressed)
	else:
		print("[CharacterCreateImproved] ERROR: random_button is null!")

	# 단계별 버튼들
	if step1_button:
		print("[CharacterCreateImproved] Connecting step1 button")
		step1_button.pressed.connect(_on_step1_pressed)
	else:
		print("[CharacterCreateImproved] ERROR: step1_button is null!")

	if step2_button:
		print("[CharacterCreateImproved] Connecting step2 button")
		step2_button.pressed.connect(_on_step2_pressed)
	else:
		print("[CharacterCreateImproved] ERROR: step2_button is null!")

	if step3_button:
		print("[CharacterCreateImproved] Connecting step3 button")
		step3_button.pressed.connect(_on_step3_pressed)
	else:
		print("[CharacterCreateImproved] ERROR: step3_button is null!")

	if step4_button:
		print("[CharacterCreateImproved] Connecting step4 button")
		step4_button.pressed.connect(_on_step4_pressed)
	else:
		print("[CharacterCreateImproved] ERROR: step4_button is null!")

	# 커스터마이징 화살표 버튼들 연결
	_connect_customization_buttons()

	# 미리보기 버튼들 연결
	_connect_preview_buttons()

	print("[CharacterCreateImproved] Button connection completed")


func _connect_customization_buttons():
	"""커스터마이징 화살표 버튼들 연결"""
	print("[CharacterCreateImproved] Connecting customization buttons...")

	# 헤어 관련
	if hair_style_next:
		hair_style_next.pressed.connect(_on_hair_style_next)
	if hair_color_next:
		hair_color_next.pressed.connect(_on_hair_color_next)
	if face_type_next:
		face_type_next.pressed.connect(_on_face_type_next)

	# 유니폼 관련
	if uniform_next:
		uniform_next.pressed.connect(_on_uniform_next)
	if uniform_color_next:
		uniform_color_next.pressed.connect(_on_uniform_color_next)
	if body_type_next:
		body_type_next.pressed.connect(_on_body_type_next)

	# 하체 관련
	if shorts_next:
		shorts_next.pressed.connect(_on_shorts_next)
	if socks_next:
		socks_next.pressed.connect(_on_socks_next)
	if shoes_next:
		shoes_next.pressed.connect(_on_shoes_next)
	if shoe_color_next:
		shoe_color_next.pressed.connect(_on_shoe_color_next)

	print("[CharacterCreateImproved] Customization buttons connected")


func _connect_preview_buttons():
	"""미리보기 영역 버튼들 연결"""
	print("[CharacterCreateImproved] Connecting preview buttons...")

	if head_preview_button:
		head_preview_button.pressed.connect(_on_head_preview_pressed)
		print("[CharacterCreateImproved] Head preview button connected")
	else:
		print("[CharacterCreateImproved] ERROR: head_preview_button is null!")

	if body_preview_button:
		body_preview_button.pressed.connect(_on_body_preview_pressed)
		print("[CharacterCreateImproved] Body preview button connected")
	else:
		print("[CharacterCreateImproved] ERROR: body_preview_button is null!")

	if legs_preview_button:
		legs_preview_button.pressed.connect(_on_legs_preview_pressed)
		print("[CharacterCreateImproved] Legs preview button connected")
	else:
		print("[CharacterCreateImproved] ERROR: legs_preview_button is null!")

	print("[CharacterCreateImproved] Preview buttons connected")


func _setup_initial_state():
	# 기본 캐릭터 데이터 설정 (확장된 옵션)
	character_data = {
		"name": "김민수",
		"position": "ST",
		"appearance":
		{
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
	}

	# 단계별 완료 상태 초기화
	step_completed = {1: false, 2: false, 3: false, 4: false}  # 외형  # 정보  # 능력치  # 확인


func _on_back_pressed():
	print("[CharacterCreateImproved] Back button pressed")
	get_tree().change_scene_to_file("res://scenes/TitleScreenImproved.tscn")


func _on_next_pressed():
	print("[CharacterCreateImproved] ===== NEXT BUTTON PRESSED =====")
	print("[CharacterCreateImproved] Current step: %d" % current_step)
	print("[CharacterCreateImproved] Max steps: %d" % max_steps)

	if current_step < max_steps:
		print("[CharacterCreateImproved] Moving to next step...")
		# 다음 단계로 진행
		_next_step()
	else:
		print("[CharacterCreateImproved] Starting game...")
		# 마지막 단계에서 게임 시작
		_start_game()


func _on_random_pressed():
	print("[CharacterCreateImproved] Random button pressed")

	# 랜덤 캐릭터 생성
	_generate_random_character()


func _on_step1_pressed():
	print("[CharacterCreateImproved] Step 1 button pressed")
	_go_to_step(1)


func _on_step2_pressed():
	print("[CharacterCreateImproved] Step 2 button pressed")
	if step_completed[1]:  # 1단계가 완료된 경우에만
		_go_to_step(2)


func _on_step3_pressed():
	print("[CharacterCreateImproved] Step 3 button pressed")
	if step_completed[1] and step_completed[2]:  # 1,2단계가 완료된 경우에만
		_go_to_step(3)


func _on_step4_pressed():
	print("[CharacterCreateImproved] Step 4 button pressed")
	if step_completed[1] and step_completed[2] and step_completed[3]:  # 1,2,3단계가 완료된 경우에만
		_go_to_step(4)


func _collect_character_data():
	# 기본 스탯
	character_data["stats"] = {"technical": 50, "mental": 50, "physical": 50, "goalkeeper": 10}

	# 기본 정보
	character_data["level"] = 1
	character_data["experience"] = 0
	character_data["week"] = 1
	character_data["year"] = 1

	print("[CharacterCreateImproved] Character data collected: %s" % character_data)


func _generate_random_character():
	# 랜덤 외형 생성 (옵션 수 증가)
	character_data["appearance"]["hair_style"] = randi() % 8  # 8가지 헤어스타일
	character_data["appearance"]["hair_color"] = randi() % 10  # 10가지 헤어컬러
	character_data["appearance"]["face_type"] = randi() % 6  # 6가지 얼굴형
	character_data["appearance"]["uniform"] = randi() % 12  # 12가지 유니폼
	character_data["appearance"]["uniform_color"] = randi() % 8  # 8가지 유니폼 색상
	character_data["appearance"]["jersey_number"] = randi() % 99 + 1
	character_data["appearance"]["body_type"] = randi() % 5  # 5가지 체형
	character_data["appearance"]["shorts"] = randi() % 6  # 6가지 반바지
	character_data["appearance"]["socks"] = randi() % 8  # 8가지 양말
	character_data["appearance"]["shoes"] = randi() % 10  # 10가지 신발
	character_data["appearance"]["shoe_color"] = randi() % 8  # 8가지 신발 색상

	# 랜덤 이름 생성 (확장된 이름 목록)
	var korean_names = [
		# 김씨
		"김민수",
		"김지훈",
		"김준호",
		"김성민",
		"김현우",
		"김태현",
		"김동현",
		"김재민",
		"김승우",
		"김민호",
		"김준영",
		"김민석",
		"김동욱",
		"김지훈",
		"김성호",
		"김민재",
		"김태현",
		"김승우",
		"김지호",
		"김준혁",
		# 이씨
		"이지훈",
		"이준호",
		"이성민",
		"이현우",
		"이태현",
		"이동현",
		"이재민",
		"이승우",
		"이민호",
		"이준영",
		"이민석",
		"이동욱",
		"이지훈",
		"이성호",
		"이민재",
		"이태현",
		"이승우",
		"이지호",
		"이준혁",
		"이민수",
		# 박씨
		"박준호",
		"박성민",
		"박현우",
		"박태현",
		"박동현",
		"박재민",
		"박승우",
		"박민호",
		"박준영",
		"박민석",
		"박동욱",
		"박지훈",
		"박성호",
		"박민재",
		"박태현",
		"박승우",
		"박지호",
		"박준혁",
		"박민수",
		"박지훈",
		# 최씨
		"최성민",
		"최현우",
		"최태현",
		"최동현",
		"최재민",
		"최승우",
		"최민호",
		"최준영",
		"최민석",
		"최동욱",
		"최지훈",
		"최성호",
		"최민재",
		"최태현",
		"최승우",
		"최지호",
		"최준혁",
		"최민수",
		"최지훈",
		"최준호",
		# 정씨
		"정현우",
		"정태현",
		"정동현",
		"정재민",
		"정승우",
		"정민호",
		"정준영",
		"정민석",
		"정동욱",
		"정지훈",
		"정성호",
		"정민재",
		"정태현",
		"정승우",
		"정지호",
		"정준혁",
		"정민수",
		"정지훈",
		"정준호",
		"정성민",
		# 기타 성씨
		"강태현",
		"윤동현",
		"임재민",
		"한승우",
		"조민호",
		"서준영",
		"오현석",
		"신동욱",
		"권민수",
		"홍지훈",
		"안준호",
		"유태현",
		"노승우",
		"문지호",
		"배준혁",
		"송민재",
		"허동현",
		"전성민",
		"고현우",
		"양지훈"
	]
	character_data["name"] = korean_names[randi() % korean_names.size()]

	print("[CharacterCreateImproved] Random character generated: %s" % character_data)


func _next_step():
	"""다음 단계로 진행"""
	print("[CharacterCreateImproved] Moving to next step from %d" % current_step)

	# 현재 단계 완료 처리
	step_completed[current_step] = true
	print("[CharacterCreateImproved] Step %d completed" % current_step)

	current_step += 1
	print("[CharacterCreateImproved] Current step is now %d" % current_step)
	_show_step(current_step)
	_update_step_indicators()
	_update_step_buttons()
	print("[CharacterCreateImproved] Step %d UI should be visible now" % current_step)


func _go_to_step(step: int):
	"""특정 단계로 이동"""
	print("[CharacterCreateImproved] Going to step %d" % step)
	current_step = step
	_show_step(current_step)
	_update_step_indicators()
	_update_step_buttons()


func _show_step(step: int):
	"""특정 단계의 UI 표시"""
	print("[CharacterCreateImproved] Showing step %d" % step)

	match step:
		1:
			_show_appearance_step()
		2:
			_show_info_step()
		3:
			_show_abilities_step()
		4:
			_show_confirm_step()


func _update_step_indicators():
	"""단계 표시기 업데이트"""
	var step_buttons = [step1_button, step2_button, step3_button, step4_button]
	var step_names = ["외형", "정보", "능력치", "확인"]

	for i in range(step_buttons.size()):
		var button = step_buttons[i]
		if button and is_instance_valid(button):
			if i + 1 == current_step:
				# 현재 단계
				button.text = "● " + step_names[i]
				button.theme_override_colors.font_color = Color(1, 0.84, 0, 1)  # 노란색
			elif step_completed[i + 1]:
				# 완료된 단계
				button.text = "✓ " + step_names[i]
				button.theme_override_colors.font_color = Color(0, 1, 0, 1)  # 녹색
			else:
				# 미완료 단계
				button.text = "○ " + step_names[i]
				button.theme_override_colors.font_color = Color(0.5, 0.5, 0.5, 1)  # 회색
		else:
			print("[CharacterCreateImproved] Warning: step_button[" + str(i) + "] is null")


func _update_step_buttons():
	"""단계별 버튼 활성화/비활성화 업데이트"""
	# 1단계는 항상 활성화
	step1_button.disabled = false

	# 2단계는 1단계 완료 시 활성화
	step2_button.disabled = not step_completed[1]

	# 3단계는 1,2단계 완료 시 활성화
	step3_button.disabled = not (step_completed[1] and step_completed[2])

	# 4단계는 1,2,3단계 완료 시 활성화
	step4_button.disabled = not (step_completed[1] and step_completed[2] and step_completed[3])


func _show_appearance_step():
	"""1단계: 외형 커스터마이징"""
	# 기존 외형 커스터마이징 UI 표시
	character_display.visible = true
	customization_panel.visible = true
	random_button.visible = true

	# 다른 단계 UI 숨기기
	_hide_other_step_uis(1)


func _show_info_step():
	"""2단계: 정보 입력 (이름, 포지션)"""
	print("[CharacterCreateImproved] Showing info step (step 2)")
	character_display.visible = true
	customization_panel.visible = false
	random_button.visible = false

	# 간단한 테스트용 UI 생성
	var test_label = Label.new()
	test_label.text = "2단계: 정보 입력\n이름과 포지션을 선택하세요"
	test_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	test_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	test_label.add_theme_font_size_override("font_size", 24)
	test_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	test_label.size_flags_vertical = Control.SIZE_EXPAND_FILL

	# 기존 UI 제거
	if step_containers.has(2):
		step_containers[2].queue_free()

	# 새 UI 추가
	main_content.add_child(test_label)
	step_containers[2] = test_label

	print("[CharacterCreateImproved] Info step UI should be visible now")


func _show_abilities_step():
	"""3단계: 능력치 분배"""
	print("[CharacterCreateImproved] Showing abilities step (step 3)")
	character_display.visible = true
	customization_panel.visible = false
	random_button.visible = false

	# 간단한 테스트용 UI 생성
	var test_label = Label.new()
	test_label.text = "3단계: 능력치 분배\n초기 능력치를 설정하세요"
	test_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	test_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	test_label.add_theme_font_size_override("font_size", 24)
	test_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	test_label.size_flags_vertical = Control.SIZE_EXPAND_FILL

	# 기존 UI 제거
	if step_containers.has(3):
		step_containers[3].queue_free()

	# 새 UI 추가
	main_content.add_child(test_label)
	step_containers[3] = test_label


func _show_confirm_step():
	"""4단계: 최종 확인"""
	print("[CharacterCreateImproved] Showing confirm step (step 4)")
	character_display.visible = true
	customization_panel.visible = false
	random_button.visible = false

	# 간단한 테스트용 UI 생성
	var test_label = Label.new()
	test_label.text = "4단계: 최종 확인\n캐릭터 생성이 완료되었습니다!"
	test_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	test_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	test_label.add_theme_font_size_override("font_size", 24)
	test_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	test_label.size_flags_vertical = Control.SIZE_EXPAND_FILL

	# 기존 UI 제거
	if step_containers.has(4):
		step_containers[4].queue_free()

	# 새 UI 추가
	main_content.add_child(test_label)
	step_containers[4] = test_label


func _hide_other_step_uis(current_step: int):
	"""다른 단계 UI들 숨기기"""
	print("[CharacterCreateImproved] Hiding other step UIs, current step: %d" % current_step)
	for step in range(1, max_steps + 1):
		if step != current_step and step_containers.has(step):
			print("[CharacterCreateImproved] Hiding step %d UI" % step)
			step_containers[step].visible = false


func _create_info_step_ui():
	"""2단계: 정보 입력 UI 생성"""
	print("[CharacterCreateImproved] Creating info step UI...")
	if step_containers.has(2):
		print("[CharacterCreateImproved] Info step UI already exists, making visible")
		step_containers[2].visible = true
		return

	# 정보 입력 컨테이너 생성
	var info_container = VBoxContainer.new()
	info_container.name = "InfoStepContainer"
	info_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 제목
	var title = Label.new()
	title.text = "선수 정보 입력"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 24)
	info_container.add_child(title)

	# 이름 입력
	var name_section = VBoxContainer.new()
	var name_label = Label.new()
	name_label.text = "선수 이름:"
	name_label.add_theme_font_size_override("font_size", 18)
	name_section.add_child(name_label)

	var name_input = LineEdit.new()
	name_input.placeholder_text = "선수 이름을 입력하세요"
	name_input.text = character_data.get("name", "김민수")
	name_input.add_theme_font_size_override("font_size", 16)
	name_section.add_child(name_input)
	info_container.add_child(name_section)

	# 포지션 선택
	var position_section = VBoxContainer.new()
	var position_label = Label.new()
	position_label.text = "포지션 선택:"
	position_label.add_theme_font_size_override("font_size", 18)
	position_section.add_child(position_label)

	var position_grid = GridContainer.new()
	position_grid.columns = 3
	position_grid.add_theme_constant_override("h_separation", 10)
	position_grid.add_theme_constant_override("v_separation", 10)

	var positions = [
		"ST",
		"CF",
		"LW",
		"RW",
		"LWF",
		"RWF",  # 공격수
		"CAM",
		"CM",
		"CDM",
		"LAM",
		"RAM",
		"LCM",
		"RCM",  # 미드필더
		"LB",
		"CB",
		"RB",
		"LCB",
		"RCB",
		"LWB",
		"RWB",  # 수비수
		"GK",
		"SW"  # 골키퍼, 스위퍼
	]
	for pos in positions:
		var pos_button = Button.new()
		pos_button.text = pos
		pos_button.custom_minimum_size = Vector2(80, 40)
		pos_button.pressed.connect(_on_position_selected.bind(pos))
		position_grid.add_child(pos_button)

	position_section.add_child(position_grid)
	info_container.add_child(position_section)

	# MainContent에 추가
	print("[CharacterCreateImproved] Adding info container to main_content")
	main_content.add_child(info_container)
	step_containers[2] = info_container
	print("[CharacterCreateImproved] Info step UI created and added successfully")


func _create_abilities_step_ui():
	"""3단계: 능력치 분배 UI 생성"""
	if step_containers.has(3):
		step_containers[3].visible = true
		return

	# 능력치 분배 컨테이너 생성
	var abilities_container = VBoxContainer.new()
	abilities_container.name = "AbilitiesStepContainer"
	abilities_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 제목
	var title = Label.new()
	title.text = "초기 능력치 설정"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 24)
	abilities_container.add_child(title)

	# 설명
	var desc = Label.new()
	desc.text = "42개 능력치 중 10개를 선택하여 초기값을 설정합니다."
	desc.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	desc.add_theme_font_size_override("font_size", 16)
	abilities_container.add_child(desc)

	# 능력치 선택 버튼들
	var abilities_grid = GridContainer.new()
	abilities_grid.columns = 3
	abilities_grid.add_theme_constant_override("h_separation", 10)
	abilities_grid.add_theme_constant_override("v_separation", 10)

	var abilities = [
		"드리블",
		"패스",
		"슈팅",
		"헤딩",
		"크로스",
		"프리킥",
		"페널티",  # 기본 기술
		"골키퍼",
		"리액션",
		"위치선정",
		"스피드",
		"스태미나",
		"파워",  # 신체/기본 능력
		"볼컨트롤",
		"롱패스",
		"숏패스",
		"슬라이딩",
		"태클",
		"인터셉트",  # 고급 기술
		"크로스",
		"코너킥",
		"스로인",
		"킥오프",
		"세이브",
		"펀칭",  # 특수 상황
		"리더십",
		"멘탈",
		"집중력",
		"판단력",
		"창의성",
		"팀워크"  # 정신적 능력
	]
	for ability in abilities:
		var ability_button = Button.new()
		ability_button.text = ability
		ability_button.custom_minimum_size = Vector2(120, 50)
		ability_button.pressed.connect(_on_ability_selected.bind(ability))
		abilities_grid.add_child(ability_button)

	abilities_container.add_child(abilities_grid)

	# MainContent에 추가
	main_content.add_child(abilities_container)
	step_containers[3] = abilities_container


func _create_confirm_step_ui():
	"""4단계: 최종 확인 UI 생성"""
	if step_containers.has(4):
		step_containers[4].visible = true
		return

	# 최종 확인 컨테이너 생성
	var confirm_container = VBoxContainer.new()
	confirm_container.name = "ConfirmStepContainer"
	confirm_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 제목
	var title = Label.new()
	title.text = "캐릭터 생성 완료"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 24)
	confirm_container.add_child(title)

	# 캐릭터 정보 요약
	var info_summary = VBoxContainer.new()
	info_summary.add_theme_constant_override("separation", 10)

	var name_label = Label.new()
	name_label.text = "이름: " + character_data.get("name", "김민수")
	name_label.add_theme_font_size_override("font_size", 18)
	info_summary.add_child(name_label)

	var position_label = Label.new()
	position_label.text = "포지션: " + character_data.get("position", "ST")
	position_label.add_theme_font_size_override("font_size", 18)
	info_summary.add_child(position_label)

	var appearance_label = Label.new()
	appearance_label.text = "외형: 커스터마이징 완료"
	appearance_label.add_theme_font_size_override("font_size", 18)
	info_summary.add_child(appearance_label)

	confirm_container.add_child(info_summary)

	# MainContent에 추가
	main_content.add_child(confirm_container)
	step_containers[4] = confirm_container


func _on_position_selected(position: String):
	"""포지션 선택"""
	character_data["position"] = position
	print("[CharacterCreateImproved] Position selected: %s" % position)


func _on_ability_selected(ability: String):
	"""능력치 선택"""
	if not character_data.has("selected_abilities"):
		character_data["selected_abilities"] = []

	if ability in character_data["selected_abilities"]:
		character_data["selected_abilities"].erase(ability)
		print("[CharacterCreateImproved] Ability deselected: %s" % ability)
	else:
		character_data["selected_abilities"].append(ability)
		print("[CharacterCreateImproved] Ability selected: %s" % ability)


func _start_game():
	"""게임 시작"""
	print("[CharacterCreateImproved] Starting game with character data: %s" % character_data)

	# 캐릭터 데이터 수집
	_collect_character_data()

	# PlayerData에 캐릭터 정보 저장
	if PlayerData:
		PlayerData.initialize_player(character_data)

	# 게임 매니저 초기화
	var game_manager = get_node_or_null("/root/GameManager")
	if game_manager:
		game_manager.start_new_game()
	else:
		print("[CharacterCreateImproved] GameManager not found - skipping")

	# 홈 화면으로 이동
	# HomeImproved (육성 홈)로 이동 - 캐릭터 생성 후 바로 육성 시작
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


# 커스터마이징 화살표 버튼 핸들러들
func _on_hair_style_next():
	print("[CharacterCreateImproved] Hair style next pressed")
	character_data["appearance"]["hair_style"] = (character_data["appearance"]["hair_style"] + 1) % 8
	_update_appearance_display()


func _on_hair_color_next():
	print("[CharacterCreateImproved] Hair color next pressed")
	character_data["appearance"]["hair_color"] = (character_data["appearance"]["hair_color"] + 1) % 10
	_update_appearance_display()


func _on_face_type_next():
	print("[CharacterCreateImproved] Face type next pressed")
	character_data["appearance"]["face_type"] = (character_data["appearance"]["face_type"] + 1) % 6
	_update_appearance_display()


func _on_uniform_next():
	print("[CharacterCreateImproved] Uniform next pressed")
	character_data["appearance"]["uniform"] = (character_data["appearance"]["uniform"] + 1) % 12
	_update_appearance_display()


func _on_uniform_color_next():
	print("[CharacterCreateImproved] Uniform color next pressed")
	character_data["appearance"]["uniform_color"] = (character_data["appearance"]["uniform_color"] + 1) % 8
	_update_appearance_display()


func _on_body_type_next():
	print("[CharacterCreateImproved] Body type next pressed")
	character_data["appearance"]["body_type"] = (character_data["appearance"]["body_type"] + 1) % 5
	_update_appearance_display()


func _on_shorts_next():
	print("[CharacterCreateImproved] Shorts next pressed")
	character_data["appearance"]["shorts"] = (character_data["appearance"]["shorts"] + 1) % 6
	_update_appearance_display()


func _on_socks_next():
	print("[CharacterCreateImproved] Socks next pressed")
	character_data["appearance"]["socks"] = (character_data["appearance"]["socks"] + 1) % 8
	_update_appearance_display()


func _on_shoes_next():
	print("[CharacterCreateImproved] Shoes next pressed")
	character_data["appearance"]["shoes"] = (character_data["appearance"]["shoes"] + 1) % 10
	_update_appearance_display()


func _on_shoe_color_next():
	print("[CharacterCreateImproved] Shoe color next pressed")
	character_data["appearance"]["shoe_color"] = (character_data["appearance"]["shoe_color"] + 1) % 8
	_update_appearance_display()


func _update_appearance_display():
	"""외형 변경 시 화면 업데이트"""
	print("[CharacterCreateImproved] Updating appearance display: %s" % character_data["appearance"])
	# 여기서 실제 캐릭터 모델을 업데이트하는 코드를 추가할 수 있습니다


# 미리보기 버튼 핸들러들
func _on_head_preview_pressed():
	print("[CharacterCreateImproved] Head preview button pressed")
	_show_design_cards("head")


func _on_body_preview_pressed():
	print("[CharacterCreateImproved] Body preview button pressed")
	_show_design_cards("body")


func _on_legs_preview_pressed():
	print("[CharacterCreateImproved] Legs preview button pressed")
	_show_design_cards("legs")


func _show_design_cards(category: String):
	"""디자인 카드 그리드 표시"""
	print("[CharacterCreateImproved] Showing design cards for category: %s" % category)

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

	# 오른쪽 패널에 추가
	customization_panel.add_child(design_card_container)
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
			current_style = character_data["appearance"]["hair_style"]
		"body":
			current_style = character_data["appearance"]["uniform"]
		"legs":
			current_style = character_data["appearance"]["shorts"]

	if design_data["style"] == current_style:
		card.modulate = Color(0.5, 1.0, 0.5, 1.0)  # 녹색으로 표시

	return card


func _on_design_card_selected(category: String, style: int):
	"""디자인 카드 선택"""
	print("[CharacterCreateImproved] Design card selected - Category: %s, Style: %d" % [category, style])

	# 캐릭터 데이터 업데이트
	match category:
		"head":
			character_data["appearance"]["hair_style"] = style
		"body":
			character_data["appearance"]["uniform"] = style
		"legs":
			character_data["appearance"]["shorts"] = style

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
