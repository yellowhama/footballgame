extends Node
# CreateEnhancedScenes - 모든 향상된 씬들을 자동 생성하는 도구
# Godot에서 실행하여 모든 씬 파일들을 생성


func _ready():
	print("=== CreateEnhancedScenes - 모든 향상된 씬 생성 시작 ===")

	# 1. Enhanced42StatsScreen.tscn 생성
	_create_enhanced_42_stats_screen()

	# 2. Enhanced21TrainingScreen.tscn 생성
	_create_enhanced_21_training_screen()

	# 3. Enhanced42ResultScreen.tscn 생성
	_create_enhanced_42_result_screen()

	# 4. StatusScreenImproved.tscn 생성
	_create_status_screen_improved()

	# 5. TitleScreenImproved.tscn 생성
	_create_title_screen_improved()

	# 6. TrainingScreenImproved.tscn 생성
	_create_training_screen_improved()

	print("=== 모든 향상된 씬 생성 완료 ===")


func _create_enhanced_42_stats_screen():
	print("[SceneBuilder] Creating Enhanced42StatsScreen.tscn...")

	# 메인 루트 노드
	var root = Control.new()
	root.name = "Enhanced42StatsScreen"
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.set_script(load("res://scripts/screens/Enhanced42StatsScreen.gd"))

	# VBox 컨테이너
	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 8)
	root.add_child(vbox)
	vbox.owner = root

	# 헤더 섹션
	var header = _create_header_section("선수 능력치", "전체: 75")
	vbox.add_child(header)
	header.owner = root

	# 검색바 섹션
	var search_bar = _create_search_bar_section()
	vbox.add_child(search_bar)
	search_bar.owner = root

	# 탭 컨테이너
	var tab_container = TabContainer.new()
	tab_container.name = "TabContainer"
	tab_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_child(tab_container)
	tab_container.owner = root

	# Technical Skills 탭
	var technical_tab = _create_skills_tab("Technical", "기술적 능력")
	tab_container.add_child(technical_tab)
	technical_tab.owner = root

	# Mental Skills 탭
	var mental_tab = _create_skills_tab("Mental", "정신적 능력")
	tab_container.add_child(mental_tab)
	mental_tab.owner = root

	# Physical Skills 탭
	var physical_tab = _create_skills_tab("Physical", "신체적 능력")
	tab_container.add_child(physical_tab)
	physical_tab.owner = root

	# Goalkeeper Skills 탭
	var gk_tab = _create_skills_tab("Goalkeeper", "골키퍼 능력")
	tab_container.add_child(gk_tab)
	gk_tab.owner = root

	# 씬 저장
	_save_scene(root, "res://scenes/Enhanced42StatsScreen.tscn")


func _create_enhanced_21_training_screen():
	print("[SceneBuilder] Creating Enhanced21TrainingScreen.tscn...")

	var root = Control.new()
	root.name = "Enhanced21TrainingScreen"
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.set_script(load("res://scripts/screens/Enhanced21TrainingScreen.gd"))

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 8)
	root.add_child(vbox)
	vbox.owner = root

	# 헤더
	var header = _create_header_section("훈련 선택", "피로도: 30/100")
	vbox.add_child(header)
	header.owner = root

	# 훈련 그리드
	var training_grid = _create_training_grid()
	vbox.add_child(training_grid)
	training_grid.owner = root

	# 하단 버튼
	var bottom_buttons = _create_bottom_buttons()
	vbox.add_child(bottom_buttons)
	bottom_buttons.owner = root

	_save_scene(root, "res://scenes/Enhanced21TrainingScreen.tscn")


func _create_enhanced_42_result_screen():
	print("[SceneBuilder] Creating Enhanced42ResultScreen.tscn...")

	var root = Control.new()
	root.name = "Enhanced42ResultScreen"
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.set_script(load("res://scripts/screens/Enhanced42ResultScreen.gd"))

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 8)
	root.add_child(vbox)
	vbox.owner = root

	# 헤더
	var header = _create_header_section("훈련 결과", "XP: +150")
	vbox.add_child(header)
	header.owner = root

	# 결과 표시
	var result_display = _create_result_display()
	vbox.add_child(result_display)
	result_display.owner = root

	# 하단 버튼
	var bottom_buttons = _create_bottom_buttons()
	vbox.add_child(bottom_buttons)
	bottom_buttons.owner = root

	_save_scene(root, "res://scenes/Enhanced42ResultScreen.tscn")


func _create_status_screen_improved():
	print("[SceneBuilder] Creating StatusScreenImproved.tscn...")

	var root = Control.new()
	root.name = "StatusScreenImproved"
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.set_script(load("res://scripts/screens/StatusScreenImproved.gd"))

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 8)
	root.add_child(vbox)
	vbox.owner = root

	# 헤더
	var header = _create_header_section("선수 상태", "1학년 1주차")
	vbox.add_child(header)
	header.owner = root

	# 상태 정보
	var status_info = _create_status_info()
	vbox.add_child(status_info)
	status_info.owner = root

	# 하단 버튼
	var bottom_buttons = _create_bottom_buttons()
	vbox.add_child(bottom_buttons)
	bottom_buttons.owner = root

	_save_scene(root, "res://scenes/StatusScreenImproved.tscn")


func _create_title_screen_improved():
	print("[SceneBuilder] Creating TitleScreenImproved.tscn...")

	var root = Control.new()
	root.name = "TitleScreenImproved"
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.set_script(load("res://scripts/screens/TitleScreenImproved.gd"))

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 20)
	vbox.add_theme_constant_override("separation", 20)
	root.add_child(vbox)
	vbox.owner = root

	# 제목
	var title = Label.new()
	title.name = "Title"
	title.text = "⚽ Football Player Game v3.0"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 48)
	vbox.add_child(title)
	title.owner = root

	# 부제목
	var subtitle = Label.new()
	subtitle.name = "Subtitle"
	subtitle.text = "Power Pro Style Football Development Simulator"
	subtitle.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	subtitle.add_theme_font_size_override("font_size", 24)
	vbox.add_child(subtitle)
	subtitle.owner = root

	# 버튼들
	var button_container = VBoxContainer.new()
	button_container.name = "ButtonContainer"
	button_container.add_theme_constant_override("separation", 10)
	vbox.add_child(button_container)
	button_container.owner = root

	var new_game_btn = Button.new()
	new_game_btn.name = "NewGameButton"
	new_game_btn.text = "새 게임 시작"
	new_game_btn.custom_minimum_size = Vector2(200, 50)
	button_container.add_child(new_game_btn)
	new_game_btn.owner = root

	var load_game_btn = Button.new()
	load_game_btn.name = "LoadGameButton"
	load_game_btn.text = "게임 불러오기"
	load_game_btn.custom_minimum_size = Vector2(200, 50)
	button_container.add_child(load_game_btn)
	load_game_btn.owner = root

	var settings_btn = Button.new()
	settings_btn.name = "SettingsButton"
	settings_btn.text = "설정"
	settings_btn.custom_minimum_size = Vector2(200, 50)
	button_container.add_child(settings_btn)
	settings_btn.owner = root

	var exit_btn = Button.new()
	exit_btn.name = "ExitButton"
	exit_btn.text = "종료"
	exit_btn.custom_minimum_size = Vector2(200, 50)
	button_container.add_child(exit_btn)
	exit_btn.owner = root

	_save_scene(root, "res://scenes/TitleScreenImproved.tscn")


func _create_training_screen_improved():
	print("[SceneBuilder] Creating TrainingScreenImproved.tscn...")

	var root = Control.new()
	root.name = "TrainingScreenImproved"
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.set_script(load("res://scripts/screens/TrainingScreenImproved.gd"))

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 8)
	root.add_child(vbox)
	vbox.owner = root

	# 헤더
	var header = _create_header_section("훈련", "주간 계획")
	vbox.add_child(header)
	header.owner = root

	# 훈련 선택
	var training_selection = _create_training_selection()
	vbox.add_child(training_selection)
	training_selection.owner = root

	# 하단 버튼
	var bottom_buttons = _create_bottom_buttons()
	vbox.add_child(bottom_buttons)
	bottom_buttons.owner = root

	_save_scene(root, "res://scenes/TrainingScreenImproved.tscn")


# 헬퍼 함수들
func _create_header_section(title: String, subtitle: String) -> Control:
	var header = Panel.new()
	header.name = "Header"
	header.custom_minimum_size = Vector2(0, 80)

	var hbox = HBoxContainer.new()
	hbox.name = "HBox"
	hbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	hbox.add_theme_constant_override("separation", 16)
	header.add_child(hbox)

	# 뒤로가기 버튼
	var back_button = Button.new()
	back_button.name = "BackButton"
	back_button.text = "← 뒤로"
	back_button.custom_minimum_size = Vector2(80, 40)
	hbox.add_child(back_button)

	# 제목
	var title_label = Label.new()
	title_label.name = "Title"
	title_label.text = title
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title_label.add_theme_font_size_override("font_size", 24)
	hbox.add_child(title_label)

	# 부제목
	var subtitle_label = Label.new()
	subtitle_label.name = "Subtitle"
	subtitle_label.text = subtitle
	subtitle_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	subtitle_label.custom_minimum_size = Vector2(120, 40)
	hbox.add_child(subtitle_label)

	return header


func _create_search_bar_section() -> Control:
	var search_bar = Panel.new()
	search_bar.name = "SearchBar"
	search_bar.custom_minimum_size = Vector2(0, 60)

	var hbox = HBoxContainer.new()
	hbox.name = "HBox"
	hbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	hbox.add_theme_constant_override("separation", 12)
	search_bar.add_child(hbox)

	# 검색 박스
	var search_box = LineEdit.new()
	search_box.name = "SearchBox"
	search_box.placeholder_text = "스킬 검색..."
	search_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_child(search_box)

	# 필터 콤보
	var filter_combo = OptionButton.new()
	filter_combo.name = "FilterCombo"
	filter_combo.custom_minimum_size = Vector2(120, 40)
	filter_combo.add_item("전체")
	filter_combo.add_item("Technical")
	filter_combo.add_item("Mental")
	filter_combo.add_item("Physical")
	filter_combo.add_item("Goalkeeper")
	hbox.add_child(filter_combo)

	return search_bar


func _create_skills_tab(category: String, display_name: String) -> Control:
	var tab = ScrollContainer.new()
	tab.name = category + "Tab"

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 4)
	tab.add_child(vbox)

	# 스킬 그리드
	var grid = GridContainer.new()
	grid.name = "SkillGrid"
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 4)
	vbox.add_child(grid)

	# 예시 스킬들 (실제로는 EnhancedPlayerData에서 가져와야 함)
	var sample_skills = _get_sample_skills(category)
	for skill_name in sample_skills:
		var skill_widget = _create_skill_widget(skill_name, 75)
		grid.add_child(skill_widget)

	return tab


func _get_sample_skills(category: String) -> Array[String]:
	match category:
		"Technical":
			return ["Dribbling", "Passing", "Shooting", "Crossing", "SetPieces", "Technique"]
		"Mental":
			return ["Vision", "Decision Making", "Leadership", "Concentration", "Teamwork", "Work Rate"]
		"Physical":
			return ["Speed", "Stamina", "Strength", "Agility", "Balance", "Jumping"]
		"Goalkeeper":
			return ["Handling", "Reflexes", "Aerial", "Distribution", "Communication", "Positioning"]
		_:
			return []


func _create_skill_widget(skill_name: String, value: int) -> Control:
	var container = HBoxContainer.new()
	container.name = skill_name + "Widget"
	container.add_theme_constant_override("separation", 8)

	var label = Label.new()
	label.name = "SkillLabel"
	label.text = skill_name
	label.custom_minimum_size = Vector2(120, 30)
	container.add_child(label)

	var progress = ProgressBar.new()
	progress.name = "SkillProgress"
	progress.value = value
	progress.max_value = 100
	progress.custom_minimum_size = Vector2(100, 30)
	container.add_child(progress)

	var value_label = Label.new()
	value_label.name = "ValueLabel"
	value_label.text = str(value)
	value_label.custom_minimum_size = Vector2(40, 30)
	container.add_child(value_label)

	return container


func _create_training_grid() -> Control:
	var grid = GridContainer.new()
	grid.name = "TrainingGrid"
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 8)

	# 21가지 훈련 카드 생성
	var training_types = [
		"Endurance",
		"Strength",
		"Speed",
		"Agility",
		"Recovery",
		"BallControl",
		"Passing",
		"Shooting",
		"Crossing",
		"SetPieces",
		"Positioning",
		"TeamShape",
		"PressingDrills",
		"TransitionPlay",
		"SetPiecesDefensive",
		"Concentration",
		"DecisionMaking",
		"Leadership",
		"MatchPreparation",
		"VideoAnalysis",
		"OpponentSpecific"
	]

	for training_type in training_types:
		var card = _create_training_card(training_type)
		grid.add_child(card)

	return grid


func _create_training_card(training_type: String) -> Control:
	var card = Panel.new()
	card.name = training_type + "Card"
	card.custom_minimum_size = Vector2(200, 120)

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 4)
	card.add_child(vbox)

	var title = Label.new()
	title.name = "Title"
	title.text = training_type
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 16)
	vbox.add_child(title)

	var description = Label.new()
	description.name = "Description"
	description.text = "훈련 설명..."
	description.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	description.add_theme_font_size_override("font_size", 12)
	vbox.add_child(description)

	var intensity = Label.new()
	intensity.name = "Intensity"
	intensity.text = "강도: 1.0"
	intensity.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	intensity.add_theme_font_size_override("font_size", 10)
	vbox.add_child(intensity)

	return card


func _create_result_display() -> Control:
	var container = VBoxContainer.new()
	container.name = "ResultDisplay"
	container.add_theme_constant_override("separation", 8)

	var title = Label.new()
	title.name = "Title"
	title.text = "훈련 결과"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 20)
	container.add_child(title)

	var xp_gained = Label.new()
	xp_gained.name = "XPGained"
	xp_gained.text = "XP 획득: +150"
	xp_gained.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	container.add_child(xp_gained)

	var skills_improved = Label.new()
	skills_improved.name = "SkillsImproved"
	skills_improved.text = "향상된 스킬: Stamina +25, Work Rate +10"
	skills_improved.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	container.add_child(skills_improved)

	return container


func _create_status_info() -> Control:
	var container = VBoxContainer.new()
	container.name = "StatusInfo"
	container.add_theme_constant_override("separation", 8)

	var name_label = Label.new()
	name_label.name = "NameLabel"
	name_label.text = "선수명: Player"
	name_label.add_theme_font_size_override("font_size", 18)
	container.add_child(name_label)

	var year_week = Label.new()
	year_week.name = "YearWeek"
	year_week.text = "학년/주차: 1학년 1주차"
	container.add_child(year_week)

	var fatigue = Label.new()
	fatigue.name = "Fatigue"
	fatigue.text = "피로도: 30/100"
	container.add_child(fatigue)

	var condition = Label.new()
	condition.name = "Condition"
	condition.text = "컨디션: 좋음"
	container.add_child(condition)

	return container


func _create_training_selection() -> Control:
	var container = VBoxContainer.new()
	container.name = "TrainingSelection"
	container.add_theme_constant_override("separation", 8)

	var title = Label.new()
	title.name = "Title"
	title.text = "훈련 선택"
	title.add_theme_font_size_override("font_size", 18)
	container.add_child(title)

	var training_list = ItemList.new()
	training_list.name = "TrainingList"
	training_list.custom_minimum_size = Vector2(0, 200)
	container.add_child(training_list)

	return container


func _create_bottom_buttons() -> Control:
	var hbox = HBoxContainer.new()
	hbox.name = "BottomButtons"
	hbox.add_theme_constant_override("separation", 8)

	var back_btn = Button.new()
	back_btn.name = "BackButton"
	back_btn.text = "뒤로"
	back_btn.custom_minimum_size = Vector2(100, 40)
	hbox.add_child(back_btn)

	var confirm_btn = Button.new()
	confirm_btn.name = "ConfirmButton"
	confirm_btn.text = "확인"
	confirm_btn.custom_minimum_size = Vector2(100, 40)
	hbox.add_child(confirm_btn)

	return hbox


func _save_scene(root: Node, path: String):
	var packed_scene = PackedScene.new()
	packed_scene.pack(root)

	var result = ResourceSaver.save(packed_scene, path)

	if result == OK:
		print("[SceneBuilder] Scene created successfully: ", path)
	else:
		print("[SceneBuilder] Failed to create scene: ", path, " Error: ", result)

	# 메모리 정리
	root.queue_free()


# 사용법 안내
func print_usage():
	print("=== CreateEnhancedScenes - 모든 향상된 씬 생성 도구 ===")
	print("1. Godot Editor에서 이 스크립트를 열기")
	print("2. Editor 메뉴에서 File > Run Script 실행")
	print("3. 다음 씬 파일들이 scenes/ 폴더에 생성됨:")
	print("   - Enhanced42StatsScreen.tscn")
	print("   - Enhanced21TrainingScreen.tscn")
	print("   - Enhanced42ResultScreen.tscn")
	print("   - StatusScreenImproved.tscn")
	print("   - TitleScreenImproved.tscn")
	print("   - TrainingScreenImproved.tscn")
	print("4. 생성된 씬 파일들을 열어서 세부 조정")
	print("=====================================================")
