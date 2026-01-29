extends EditorScript
# Enhanced42StatsSceneBuilder - Enhanced42StatsScreen.tscn 자동 생성 도구
# Godot Editor에서 실행하여 씬 파일 생성


func _run():
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
	var header = _create_header_section()
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

	# 씬 저장
	var packed_scene = PackedScene.new()
	packed_scene.pack(root)

	var save_path = "res://scenes/Enhanced42StatsScreen.tscn"
	var result = ResourceSaver.save(packed_scene, save_path)

	if result == OK:
		print("[SceneBuilder] Scene created successfully: ", save_path)
	else:
		print("[SceneBuilder] Failed to create scene: ", result)

	# 메모리 정리
	root.queue_free()


func _create_header_section() -> Control:
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
	var title = Label.new()
	title.name = "Title"
	title.text = "선수 능력치"
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 24)
	hbox.add_child(title)

	# 전체 능력치
	var overall = Label.new()
	overall.name = "Overall"
	overall.text = "전체: 75"
	overall.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	overall.custom_minimum_size = Vector2(80, 40)
	hbox.add_child(overall)

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
	hbox.add_child(filter_combo)

	return search_bar


# 사용법 안내
func print_usage():
	print("=== Enhanced42StatsScreen Scene Builder ===")
	print("1. Godot Editor에서 이 스크립트를 열기")
	print("2. Editor 메뉴에서 File > Run Script 실행")
	print("3. Enhanced42StatsScreen.tscn 파일이 scenes/ 폴더에 생성됨")
	print("4. 생성된 씬 파일을 열어서 세부 조정")
	print("=========================================")
