extends Control
# TitleScreen 개선 - 카이로소프트 스타일

# UI 요소 (using unique name syntax)
@onready var main_title: Label = %MainTitle
@onready var sub_title: Label = %SubTitle

# 버튼들
@onready var career_btn: Button = %NewGameButton  # 육성게임 버튼 (세이브 슬롯 선택으로 이동)
@onready var myteam_btn: Button = %MyTeamButton
@onready var personality_demo_btn: Button = %PersonalityDemoButton
@onready var options_btn: Button = %OptionsButton
@onready var exit_btn: Button = %ExitButton

# 미니 캐릭터들 (not marked as unique - using path)
@onready var mini_characters = [
	$MiniCharacters/Player1,
	$MiniCharacters/Player2,
	$MiniCharacters/Player3,
	$MiniCharacters/Player4,
	$MiniCharacters/Player5
]

# 구름들 (not marked as unique - using path)
@onready var clouds = [$CloudLayer/Cloud1, $CloudLayer/Cloud2, $CloudLayer/Cloud3]

# 세이브 슬롯 팝업
@onready var save_slots_popup: Panel = %SaveSlotsPopup

# 사운드 토글
@onready var music_toggle: Button = %MusicToggle
@onready var sound_toggle: Button = %SoundToggle

# 언어 선택
@onready var language_button: Button = %LanguageButton
@onready var language_popup: AcceptDialog = %LanguagePopup
@onready var korean_button: Button = %KoreanButton
@onready var english_button: Button = %EnglishButton
@onready var japanese_button: Button = %JapaneseButton
@onready var chinese_button: Button = %ChineseButton
@onready var select_button: Button = %SelectButton

# 현재 선택된 언어
var selected_language = "English"

# 애니메이션 변수
var time_passed = 0.0
var character_speeds = []
var cloud_speeds = []
var button_hover_states = {}


func _ready():
	print("[TitleScreenImproved] Initializing with menu buttons...")

	# 타이틀 애니메이션
	_animate_title_entrance()

	# 헤드리스 모드나 --check-only 옵션이면 바로 종료
	var cmdline_args = OS.get_cmdline_args()
	var is_check_only = "--check-only" in cmdline_args
	var is_contract_tests = "--contract-tests" in cmdline_args

	if Engine.is_editor_hint():
		print("[TitleScreenImproved] Editor mode")
	elif is_check_only:
		print("[TitleScreenImproved] Check-only mode, exiting...")
		get_tree().quit(0)
		return
	elif OS.has_feature("headless"):
		if is_contract_tests:
			print("[TitleScreenImproved] Headless contract-tests mode, skipping UI init...")
		else:
			print("[TitleScreenImproved] Headless mode, exiting...")
			get_tree().quit(0)
			return
	else:
		# 버튼 스타일 및 연결
		_apply_custom_styles()
		_connect_buttons()
		_animate_menu_entrance()

	print("[TitleScreenImproved] Ready!")


func _hide_all_buttons():
	"""모든 버튼과 메뉴 요소 숨기기"""
	if $MenuContainer:
		$MenuContainer.visible = false
	if $FooterInfo:
		$FooterInfo.visible = false
	if language_button:
		language_button.visible = false
	if save_slots_popup:
		save_slots_popup.visible = false


func _start_auto_transition():
	"""자동 전환 시작 (3초 후 메인 홈으로)"""
	# 로딩 인디케이터 표시 (선택사항)
	_show_loading_indicator()

	# 3초 후 자동으로 메인 홈으로 이동
	await get_tree().create_timer(3.0).timeout
	_auto_start_game()


func _show_loading_indicator():
	"""간단한 로딩 인디케이터 표시"""
	var loading_label = Label.new()
	loading_label.text = "Loading..."
	loading_label.add_theme_font_size_override("font_size", 24)
	loading_label.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	loading_label.position.y += 200  # 타이틀 아래로
	add_child(loading_label)

	# 점들 애니메이션 (무한 루프 제거, 3번만 반복)
	var tween = get_tree().create_tween().set_loops(3)
	tween.tween_method(_update_loading_text.bind(loading_label), 0.0, 3.0, 1.0)


func _update_loading_text(dots: int, label: Label):
	"""로딩 텍스트 애니메이션"""
	var dot_string = ""
	for i in range(int(dots)):
		dot_string += "."
	label.text = "Loading" + dot_string


func _auto_start_game():
	"""자동으로 게임 시작 (기존 new game 로직 사용)"""
	print("[TitleScreenImproved] Auto-starting game...")
	# 화면 전환 애니메이션
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate:a", 0.0, 0.3)
	tween.tween_callback(
		func():
			# HomeImproved (육성 게임 메인)로 이동
			get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")
	)


func _apply_custom_styles():
	# 배경 필드 스타일
	var stadium_bg = $StadiumBackground
	if stadium_bg:
		var style = StyleBoxFlat.new()
		style.bg_color = Color(0.2, 0.7, 0.3, 1)
		style.corner_radius_top_left = 0
		style.corner_radius_top_right = 0
		style.shadow_size = 10
		style.shadow_offset = Vector2(0, -5)
		style.shadow_color = Color(0, 0, 0, 0.3)
		stadium_bg.add_theme_stylebox_override("panel", style)

	# 버튼 스타일
	var buttons = [career_btn, myteam_btn, personality_demo_btn, options_btn, exit_btn]
	for button in buttons:
		if button:
			_setup_button_style(button)

	# 세이브 슬롯 팝업 스타일
	if save_slots_popup:
		save_slots_popup.add_theme_stylebox_override("panel", CustomStyles.create_card_panel())

	# 슬롯 카드 스타일
	var slots = [
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot1,
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot2,
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot3
	]
	for slot in slots:
		if slot:
			if slot.has_node("EmptyLabel"):
				slot.add_theme_stylebox_override("panel", CustomStyles.create_save_slot_empty())
			else:
				slot.add_theme_stylebox_override("panel", CustomStyles.create_save_slot_filled())


func _setup_button_style(button: Button):
	var style = StyleBoxFlat.new()
	style.bg_color = Color(1, 1, 1, 0.95)
	style.corner_radius_top_left = 20
	style.corner_radius_top_right = 20
	style.corner_radius_bottom_left = 20
	style.corner_radius_bottom_right = 20
	style.shadow_size = 8
	style.shadow_offset = Vector2(0, 4)
	style.shadow_color = Color(0, 0, 0, 0.3)
	style.border_width_left = 3
	style.border_width_right = 3
	style.border_width_top = 3
	style.border_width_bottom = 3
	style.border_color = Color(0.9, 0.9, 0.9, 1)

	button.add_theme_stylebox_override("normal", style)

	# 호버 스타일
	var hover_style = style.duplicate()
	hover_style.bg_color = Color(1, 0.95, 0.8, 1)
	hover_style.border_color = ThemeManager.ACCENT
	hover_style.shadow_size = 12
	button.add_theme_stylebox_override("hover", hover_style)

	# 누름 스타일
	var pressed_style = style.duplicate()
	pressed_style.bg_color = Color(0.9, 0.9, 0.9, 1)
	pressed_style.shadow_size = 2
	pressed_style.shadow_offset = Vector2(0, 1)
	button.add_theme_stylebox_override("pressed", pressed_style)

	button.add_theme_color_override("font_color", Color(0.2, 0.2, 0.3, 1))
	button.add_theme_color_override("font_hover_color", Color(0.1, 0.1, 0.2, 1))


func _connect_buttons():
	if career_btn:
		career_btn.pressed.connect(_on_career_pressed)
		career_btn.mouse_entered.connect(func(): _on_button_hover(career_btn, true))
		career_btn.mouse_exited.connect(func(): _on_button_hover(career_btn, false))

	if myteam_btn:
		myteam_btn.pressed.connect(_on_myteam_pressed)
		myteam_btn.mouse_entered.connect(func(): _on_button_hover(myteam_btn, true))
		myteam_btn.mouse_exited.connect(func(): _on_button_hover(myteam_btn, false))

	if personality_demo_btn:
		personality_demo_btn.pressed.connect(_on_personality_demo_pressed)
		personality_demo_btn.mouse_entered.connect(func(): _on_button_hover(personality_demo_btn, true))
		personality_demo_btn.mouse_exited.connect(func(): _on_button_hover(personality_demo_btn, false))

	if options_btn:
		options_btn.pressed.connect(_on_options_pressed)

	if exit_btn:
		exit_btn.pressed.connect(_on_exit_pressed)

	# 세이브 슬롯 팝업
	var close_btn = $SaveSlotsPopup/VBox/CloseButton
	if close_btn:
		close_btn.pressed.connect(_close_save_slots)

	var slots = [
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot1,
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot2,
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot3
	]
	for slot in slots:
		if slot and slot.has_signal("slot_loaded"):
			slot.slot_loaded.connect(_load_save_slot)

	# 사운드 토글
	if music_toggle:
		music_toggle.pressed.connect(_toggle_music)
	if sound_toggle:
		sound_toggle.pressed.connect(_toggle_sound)

	# 언어 선택 버튼들
	if language_button:
		language_button.pressed.connect(_on_language_button_pressed)

	if korean_button:
		korean_button.pressed.connect(func(): _select_language("Korean"))
	if english_button:
		english_button.pressed.connect(func(): _select_language("English"))
	if japanese_button:
		japanese_button.pressed.connect(func(): _select_language("Japanese"))
	if chinese_button:
		chinese_button.pressed.connect(func(): _select_language("Chinese"))

	if select_button:
		select_button.pressed.connect(_on_select_language_pressed)


func _initialize_animations():
	# 미니 캐릭터 속도 설정
	for i in range(mini_characters.size()):
		character_speeds.append(randf_range(50, 150))

	# 구름 속도 설정
	for i in range(clouds.size()):
		cloud_speeds.append(randf_range(20, 60))


func _animate_title_entrance():
	# 타이틀 진입 애니메이션
	if main_title:
		main_title.modulate.a = 0
		main_title.position.y = -50

		var tween = get_tree().create_tween()
		tween.set_parallel(true)
		tween.tween_property(main_title, "modulate:a", 1.0, 0.5)
		tween.tween_property(main_title, "position:y", 0, 0.5).set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BOUNCE)

	if sub_title:
		sub_title.modulate.a = 0
		var tween = get_tree().create_tween()
		tween.tween_interval(0.3)
		tween.tween_property(sub_title, "modulate:a", 1.0, 0.5)


func _animate_menu_entrance():
	# 메뉴 버튼 순차적 진입
	var buttons = [career_btn, myteam_btn, options_btn, exit_btn]
	var delay = 0.0

	for button in buttons:
		if button:
			button.modulate.a = 0
			button.position.x = -100

			var tween = get_tree().create_tween()
			tween.tween_interval(delay)
			tween.set_parallel(true)
			tween.tween_property(button, "modulate:a", 1.0, 0.3)
			tween.tween_property(button, "position:x", 0, 0.3).set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_CUBIC)

			delay += 0.1


func _process(delta):
	time_passed += delta

	# 미니 캐릭터 움직임
	_animate_mini_characters(delta)

	# 구름 움직임
	_animate_clouds(delta)

	# 타이틀 플로팅 효과
	_animate_title_floating()


func _animate_mini_characters(delta):
	# 배열이 제대로 초기화되지 않았거나 비어있으면 건너뛰기
	if mini_characters.is_empty() or character_speeds.is_empty():
		return

	for i in range(mini_characters.size()):
		if i >= character_speeds.size() or not mini_characters[i]:
			continue

		var character = mini_characters[i]
		var speed = character_speeds[i]

		# 좌우 이동
		character.position.x += speed * delta

		# 화면 밖으로 나가면 반대편에서 나타남
		if character.position.x > get_viewport_rect().size.x + 100:
			character.position.x = -100
			# 랜덤하게 Y 위치 변경
			character.position.y = randf_range(1000, 1400)
			# 속도도 랜덤 변경
			character_speeds[i] = randf_range(50, 150)

		# 위아래 바운싱
		character.position.y += sin(time_passed * 3 + i) * 2


func _animate_clouds(delta):
	# 배열이 제대로 초기화되지 않았거나 비어있으면 건너뛰기
	if clouds.is_empty() or cloud_speeds.is_empty():
		return

	for i in range(clouds.size()):
		if i >= cloud_speeds.size() or not clouds[i]:
			continue

		var cloud = clouds[i]
		var speed = cloud_speeds[i]

		# 좌측으로 이동
		cloud.position.x -= speed * delta

		# 화면 밖으로 나가면 오른쪽에서 나타남
		if cloud.position.x < -200:
			cloud.position.x = get_viewport_rect().size.x + 100
			cloud.position.y = randf_range(50, 300)
			cloud_speeds[i] = randf_range(20, 60)


func _animate_title_floating():
	if main_title:
		main_title.position.y = sin(time_passed * 2) * 10


func _on_button_hover(button: Button, is_hovering: bool):
	if not button:
		return

	if is_hovering:
		# 바운스 애니메이션
		var tween = get_tree().create_tween()
		tween.tween_property(button, "scale", Vector2(1.1, 1.1), 0.1).set_ease(Tween.EASE_OUT).set_trans(
			Tween.TRANS_ELASTIC
		)

		# 축구공 이모지 추가 애니메이션
		_spawn_ball_effect(button.global_position + button.size / 2)
	else:
		# 원래 크기로
		var tween = get_tree().create_tween()
		tween.tween_property(button, "scale", Vector2(1.0, 1.0), 0.2)


func _spawn_ball_effect(pos: Vector2):
	var ball = Label.new()
	ball.text = "⚽"
	ball.add_theme_font_size_override("font_size", 32)
	ball.position = pos
	add_child(ball)

	# 위로 떠오르며 사라지는 애니메이션
	var tween = get_tree().create_tween()
	tween.set_parallel(true)
	tween.tween_property(ball, "position:y", pos.y - 100, 0.5)
	tween.tween_property(ball, "modulate:a", 0.0, 0.5)
	tween.set_parallel(false)
	tween.tween_callback(ball.queue_free)


func _on_career_pressed():
	print("[TitleScreenImproved] 육성게임 pressed - going to save slot selection")

	# 화면 전환 애니메이션
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate:a", 0.0, 0.3)
	tween.tween_callback(
		func():
			# CareerIntroScreen (세이브 슬롯 선택)으로 이동
			get_tree().change_scene_to_file("res://scenes/CareerIntroScreen.tscn")
	)


func _on_myteam_pressed():
	print("[TitleScreenImproved] My team pressed")
	if UIManager:
		UIManager.push("res://scenes/MyTeamScreen.tscn")


func _on_personality_demo_pressed():
	print("[TitleScreenImproved] Personality demo pressed")
	# PersonAttributes 테스트 화면으로 이동
	# DEPRECATED: PersonalityDemo.tscn has been archived
	# get_tree().change_scene_to_file("res://scenes/PersonalityDemo.tscn")


func _on_options_pressed():
	print("[TitleScreenImproved] Options pressed")
	# 옵션 화면 표시


func _on_exit_pressed():
	print("[TitleScreenImproved] Exit pressed")
	get_tree().quit()


func _show_save_slots():
	if save_slots_popup:
		save_slots_popup.visible = true
		save_slots_popup.modulate.a = 0
		save_slots_popup.scale = Vector2(0.8, 0.8)

		var tween = get_tree().create_tween()
		tween.set_parallel(true)
		tween.tween_property(save_slots_popup, "modulate:a", 1.0, 0.3)
		tween.tween_property(save_slots_popup, "scale", Vector2(1.0, 1.0), 0.3).set_ease(Tween.EASE_OUT).set_trans(
			Tween.TRANS_BACK
		)

		# 세이브 슬롯 정보 업데이트
		_update_save_slots_info()


func _close_save_slots():
	if save_slots_popup:
		var tween = get_tree().create_tween()
		tween.set_parallel(true)
		tween.tween_property(save_slots_popup, "modulate:a", 0.0, 0.2)
		tween.tween_property(save_slots_popup, "scale", Vector2(0.8, 0.8), 0.2)
		tween.set_parallel(false)
		tween.tween_callback(func(): save_slots_popup.visible = false)


func _toggle_music():
	print("[TitleScreenImproved] Music toggled")
	if music_toggle:
		music_toggle.text = "🔇" if music_toggle.text == "🎵" else "🎵"


func _toggle_sound():
	print("[TitleScreenImproved] Sound toggled")
	if sound_toggle:
		sound_toggle.text = "🔈" if sound_toggle.text == "🔊" else "🔊"


# 언어 선택 관련 함수들
func _on_language_button_pressed():
	print("[TitleScreenImproved] Language button pressed")
	if language_popup:
		language_popup.popup_centered()
		_update_language_button_styles()


func _select_language(language: String):
	print("[TitleScreenImproved] Language selected: %s" % language)
	selected_language = language
	_update_language_button_styles()


func _on_select_language_pressed():
	print("[TitleScreenImproved] Language confirmed: %s" % selected_language)
	if language_popup:
		language_popup.hide()

	# 언어 설정 저장 및 적용
	_apply_language_settings(selected_language)

	# 언어 버튼 텍스트 업데이트
	_update_language_button_text()


func _update_language_button_styles():
	# 모든 언어 버튼 스타일 초기화
	var language_buttons = [korean_button, english_button, japanese_button, chinese_button]
	for button in language_buttons:
		if button:
			button.modulate = Color.WHITE

	# 선택된 언어 버튼 하이라이트
	var selected_button = null
	match selected_language:
		"Korean":
			selected_button = korean_button
		"English":
			selected_button = english_button
		"Japanese":
			selected_button = japanese_button
		"Chinese":
			selected_button = chinese_button

	if selected_button:
		selected_button.modulate = Color.YELLOW


func _update_language_button_text():
	if language_button:
		var language_text = "🌐 "
		match selected_language:
			"Korean":
				language_text += "한국어"
			"English":
				language_text += "English"
			"Japanese":
				language_text += "日本語"
			"Chinese":
				language_text += "中文"
		language_button.text = language_text


func _apply_language_settings(language: String):
	# 언어 설정을 저장하고 게임 전체에 적용
	print("[TitleScreenImproved] Applying language: %s" % language)

	# 여기에 실제 언어 설정 로직 추가
	# 예: TranslationServer.set_locale(), 설정 파일 저장 등


func _update_save_slots_info():
	"""세이브 슬롯 정보 업데이트 (Refactored)"""
	print("[TitleScreenImproved] Updating save slots info...")

	var slots = [
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot1,
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot2,
		$SaveSlotsPopup/VBox/ScrollContainer/SlotsContainer/Slot3
	]

	for i in range(slots.size()):
		var slot = slots[i]
		var slot_num = i + 1

		if slot and SaveManager:
			var info = SaveManager.get_slot_info(slot_num)
			# The new SaveSlot.gd script is attached to the slot, so we can just call update_info
			if slot.has_method("update_info"):
				slot.update_info(info, slot_num)
			else:
				push_error("Slot node is missing the SaveSlot.gd script or its update_info method.")


func _load_save_slot(slot_number: int):
	"""세이브 슬롯 로드"""
	print("[TitleScreenImproved] Loading save slot %d..." % slot_number)

	if SaveManager:
		if SaveManager.load_from_slot(slot_number):
			print("[TitleScreenImproved] Save slot %d loaded successfully!" % slot_number)

			# HomeImproved (육성 게임 메인)로 전환
			var tween = get_tree().create_tween()
			tween.tween_property(self, "modulate:a", 0.0, 0.3)
			tween.tween_callback(func(): get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn"))
		else:
			print("[TitleScreenImproved] Failed to load save slot %d!" % slot_number)
			# 에러 메시지 표시 (나중에 구현)
