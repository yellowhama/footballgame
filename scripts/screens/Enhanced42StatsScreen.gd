extends Control
class_name Enhanced42StatsScreen
# Enhanced42StatsScreen - 42개 능력치 동적 표시 시스템
# open-football 42-skill system integration

signal skill_detail_requested(skill_name: String, skill_data: Dictionary)
signal back_requested

# 화면 크기 분류
enum ScreenSize { SMALL, MEDIUM, LARGE }

# UI 컴포넌트 참조
@onready var tab_container: TabContainer = $VBox/TabContainer
@onready var header_label: Label = $VBox/Header/HBox/Title
@onready var back_button: Button = $VBox/Header/HBox/BackButton
@onready var overall_label: Label = $VBox/Header/HBox/Overall
@onready var search_box: LineEdit = $VBox/SearchBar/HBox/SearchBox
@onready var filter_combo: OptionButton = $VBox/SearchBar/HBox/FilterCombo

# 스킬 위젯 컨테이너들
var skill_widgets: Dictionary = {}  # skill_name -> SkillWidget
var category_containers: Dictionary = {}  # category -> Container
var skill_data_cache: Dictionary = {}  # 42개 스킬 데이터 캐시

# 42개 스킬 정의 (OpenFootball PlayerAttributes 정확 매핑)
const SKILL_CATEGORIES = {
	"Technical":
	{
		"skills":
		[
			"corners",
			"crossing",
			"dribbling",
			"finishing",
			"first_touch",
			"free_kicks",
			"heading",
			"long_shots",
			"long_throws",
			"marking",
			"passing",
			"penalty_taking",
			"tackling",
			"technique",
			"ball_control",
			"shooting"
		],
		"display_names":
		{
			"corners": "코너킥",
			"crossing": "크로스",
			"dribbling": "드리블링",
			"finishing": "골결정력",
			"first_touch": "퍼스트터치",
			"free_kicks": "프리킥",
			"heading": "헤딩",
			"long_shots": "중거리슛",
			"long_throws": "롱스로우",
			"marking": "마킹",
			"passing": "패스",
			"penalty_taking": "페널티킥",
			"tackling": "태클링",
			"technique": "기술",
			"ball_control": "볼컨트롤",
			"shooting": "슈팅"
		}
	},
	"Mental":
	{
		"skills":
		[
			"aggression",
			"anticipation",
			"bravery",
			"composure",
			"concentration",
			"decisions",
			"determination",
			"flair",
			"leadership",
			"off_the_ball",
			"positioning",
			"teamwork",
			"vision",
			"work_rate"
		],
		"display_names":
		{
			"aggression": "공격성",
			"anticipation": "예측력",
			"bravery": "용기",
			"composure": "침착함",
			"concentration": "집중력",
			"decisions": "판단력",
			"determination": "결단력",
			"flair": "창의성",
			"leadership": "리더십",
			"off_the_ball": "오프더볼",
			"positioning": "포지셔닝",
			"teamwork": "팀워크",
			"vision": "시야",
			"work_rate": "활동량"
		}
	},
	"Physical":
	{
		"skills":
		["acceleration", "agility", "balance", "jumping", "natural_fitness", "pace", "stamina", "strength", "speed"],
		"display_names":
		{
			"acceleration": "가속력",
			"agility": "민첩성",
			"balance": "균형감",
			"jumping": "점프력",
			"natural_fitness": "타고난체력",
			"pace": "스피드",
			"stamina": "스태미나",
			"strength": "힘",
			"speed": "순간속도"
		}
	},
	"Goalkeeper":
	{
		"skills": ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"],
		"display_names":
		{
			"reflexes": "반사신경",
			"handling": "핸들링",
			"aerial_ability": "공중볼처리",
			"command_of_area": "박스장악력",
			"communication": "의사소통",
			"kicking": "킥력"
		}
	}
}

# 검색 및 필터링
var current_filter: String = "All"
var search_query: String = ""


func _ready():
	print("[Enhanced42StatsScreen] Initializing 42-skill system...")
	_setup_ui()
	_create_skill_tabs()
	_connect_signals()
	_load_player_skills()
	setup_mobile_adapters()
	print("[Enhanced42StatsScreen] Ready!")


func _setup_ui():
	"""기본 UI 설정"""
	# 헤더 설정
	if header_label:
		header_label.text = "선수 능력치 (42개 스킬)"
		header_label.add_theme_font_size_override("font_size", 24)

	# 검색 박스 설정
	if search_box:
		search_box.placeholder_text = "스킬 검색..."

	# 필터 콤보 설정
	if filter_combo:
		filter_combo.add_item("전체")
		filter_combo.add_item("Technical")
		filter_combo.add_item("Mental")
		filter_combo.add_item("Physical")
		filter_combo.add_item("Goalkeeper")

	# 스타일 적용
	_apply_custom_styles()


func _apply_custom_styles():
	"""커스텀 스타일 적용"""
	# 메인 배경
	var bg_panel = Panel.new()
	add_child(bg_panel)
	move_child(bg_panel, 0)  # 맨 뒤로
	bg_panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	bg_panel.add_theme_stylebox_override("panel", CustomStyles.create_main_background())

	# 헤더 스타일
	var header = $VBox/Header
	if header:
		header.add_theme_stylebox_override("panel", CustomStyles.create_header_panel())

	# 탭 컨테이너 스타일
	if tab_container:
		tab_container.add_theme_stylebox_override("panel", CustomStyles.create_card_panel())


func _create_skill_tabs():
	"""4개 카테고리 탭 생성"""
	for category in SKILL_CATEGORIES.keys():
		var tab = _create_category_tab(category)
		tab_container.add_child(tab)
		tab.name = category


func _create_category_tab(category: String) -> Control:
	"""개별 카테고리 탭 생성"""
	var tab = ScrollContainer.new()
	tab.name = category

	# Grid Container for skills
	var grid = GridContainer.new()
	grid.columns = 2  # 2열 레이아웃 (모바일 최적화)
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	tab.add_child(grid)

	# 카테고리별 스킬 위젯 생성
	var skills = SKILL_CATEGORIES[category]["skills"]
	var display_names = SKILL_CATEGORIES[category]["display_names"]

	for skill_name in skills:
		var skill_widget = _create_skill_widget(skill_name, display_names[skill_name], category)
		grid.add_child(skill_widget)
		skill_widgets[skill_name] = skill_widget

	category_containers[category] = grid
	return tab


func _create_skill_widget(skill_name: String, display_name: String, category: String) -> SkillWidget:
	"""개별 스킬 위젯 생성"""
	var widget = preload("res://scripts/components/SkillWidget.gd").new()

	# 기본 설정
	widget.custom_minimum_size = Vector2(200, 80)  # 모바일 친화적 크기
	widget.skill_name = skill_name
	widget.skill_display_name = display_name
	widget.show_bar = true
	widget.show_delta = false

	# 카테고리 테마 적용
	widget.set_category_theme(category)

	# 시그널 연결
	widget.skill_clicked.connect(_on_skill_clicked)
	widget.skill_hovered.connect(_on_skill_hovered)

	return widget


func _connect_signals():
	"""시그널 연결"""
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	if search_box:
		search_box.text_changed.connect(_on_search_text_changed)

	if filter_combo:
		filter_combo.item_selected.connect(_on_filter_changed)

	if tab_container:
		tab_container.tab_changed.connect(_on_tab_changed)


func _load_player_skills():
	"""플레이어 스킬 데이터 로드"""
	if not PlayerData:
		print("[Enhanced42StatsScreen] PlayerData not available")
		return

	print("[Enhanced42StatsScreen] Loading player skill data...")

	# 42개 스킬 데이터 로드
	for category in SKILL_CATEGORIES.keys():
		var skills = SKILL_CATEGORIES[category]["skills"]
		for skill_name in skills:
			var skill_value = _get_player_skill_value(skill_name)
			skill_data_cache[skill_name] = skill_value

			# 위젯 업데이트
			if skill_widgets.has(skill_name):
				skill_widgets[skill_name].set_skill_value(skill_value)

	# 전체 능력치 계산 및 표시
	_update_overall_rating()


func _get_player_skill_value(skill_name: String) -> float:
	"""플레이어 스킬 값 가져오기 (CoreMapper 통합)"""
	# PlayerData에서 스킬 값 가져오기
	# 기존 12개 스킬과 새로운 42개 스킬 매핑 처리

	# 기존 스킬 매핑
	var legacy_mapping = {
		"first_touch": "first_touch",
		"dribbling": "dribbling",
		"passing": "passing",
		"pace": "pace",
		"acceleration": "acceleration",
		"stamina": "stamina",
		"finishing": "finishing",
		"vision": "vision",
		"positioning": "positioning",
		"composure": "composure",
		"tackling": "tackling"
	}

	if legacy_mapping.has(skill_name) and PlayerData.has_method("get_stat"):
		return PlayerData.get_stat("skills", legacy_mapping[skill_name])  # 카테고리와 스킬명 전달

	# 새로운 스킬은 기본값 사용 (임시)
	return _generate_default_skill_value(skill_name)


func _generate_default_skill_value(skill_name: String) -> float:
	"""기본 스킬 값 생성 (임시 구현)"""
	# 포지션별 기본값 설정
	var position = PlayerData.position if "position" in PlayerData else 0
	var base_value = 50.0

	# 포지션별 보정
	match position:
		0:  # ST (스트라이커)
			if skill_name in ["finishing", "positioning", "off_the_ball"]:
				base_value += 15.0
			elif skill_name in ["tackling", "marking", "heading"]:
				base_value -= 10.0
		1:  # CAM (공격형 미드필더)
			if skill_name in ["vision", "passing", "creativity", "technique"]:
				base_value += 15.0
			elif skill_name in ["strength", "jumping"]:
				base_value -= 10.0
		2:  # CM (중앙 미드필더)
			if skill_name in ["passing", "vision", "teamwork", "work_rate"]:
				base_value += 10.0
		3:  # CB (중앙 수비수)
			if skill_name in ["tackling", "marking", "heading", "strength"]:
				base_value += 15.0
			elif skill_name in ["dribbling", "pace"]:
				base_value -= 10.0

	# 랜덤 변동 추가 (±10)
	base_value += randf_range(-10.0, 10.0)

	return clamp(base_value, 1.0, 100.0)


func _update_overall_rating():
	"""전체 능력치 계산"""
	if not overall_label:
		return

	var total_value = 0.0
	var skill_count = 0

	for skill_name in skill_data_cache.keys():
		total_value += skill_data_cache[skill_name]
		skill_count += 1

	if skill_count > 0:
		var overall = total_value / skill_count
		overall_label.text = "전체: %d" % int(overall)
		overall_label.modulate = ThemeManager.get_stat_color(overall, 100.0)


func update_skill_values(new_skill_data: Dictionary):
	"""스킬 값 일괄 업데이트 (훈련 결과 등)"""
	for skill_name in new_skill_data.keys():
		var new_value = new_skill_data[skill_name]
		var old_value = skill_data_cache.get(skill_name, 50.0)

		# 캐시 업데이트
		skill_data_cache[skill_name] = new_value

		# 위젯 업데이트 (애니메이션 포함)
		if skill_widgets.has(skill_name):
			skill_widgets[skill_name].set_skill_value(new_value, true)

			# 변화량 표시
			var delta = new_value - old_value
			if abs(delta) > 0.1:
				skill_widgets[skill_name].set_delta_value(delta)

	# 전체 능력치 재계산
	_update_overall_rating()


func highlight_changed_skills(changed_skills: Array):
	"""변화된 스킬들 하이라이트"""
	for skill_name in changed_skills:
		if skill_widgets.has(skill_name):
			var widget = skill_widgets[skill_name]
			# 반짝임 효과
			var tween = create_tween()
			tween.tween_property(widget, "modulate", Color(1.2, 1.2, 1.0), 0.3)
			tween.tween_property(widget, "modulate", Color.WHITE, 0.3)


func _on_search_text_changed(new_text: String):
	"""검색 텍스트 변경 처리"""
	_apply_touch_feedback()
	search_query = new_text.to_lower()
	print("[Enhanced42StatsScreen] Search query: %s" % search_query)
	_apply_filters()


func _on_filter_changed(index: int):
	"""필터 변경 처리"""
	_apply_touch_feedback()
	var filters = ["All", "Technical", "Mental", "Physical", "Goalkeeper"]
	current_filter = filters[index]
	print("[Enhanced42StatsScreen] Filter changed to: %s" % current_filter)
	_apply_filters()


func _apply_filters():
	"""검색 및 필터 적용"""
	for skill_name in skill_widgets.keys():
		var widget = skill_widgets[skill_name]
		var should_show = true

		# 카테고리 필터
		if current_filter != "All":
			var skill_category = _get_skill_category(skill_name)
			if skill_category != current_filter:
				should_show = false

		# 검색 필터
		if search_query != "":
			var display_name = widget.skill_display_name.to_lower()
			if not display_name.contains(search_query):
				should_show = false

		widget.visible = should_show


func _get_skill_category(skill_name: String) -> String:
	"""스킬이 속한 카테고리 반환"""
	for category in SKILL_CATEGORIES.keys():
		if skill_name in SKILL_CATEGORIES[category]["skills"]:
			return category
	return ""


func _on_skill_clicked(skill_name: String, value: float):
	"""스킬 클릭 처리"""
	print("[Enhanced42StatsScreen] Skill clicked: %s (%.1f)" % [skill_name, value])

	var skill_data = {
		"name": skill_name,
		"value": value,
		"category": _get_skill_category(skill_name),
		"grade": _get_skill_grade(value),
		"description": _get_skill_description(skill_name)
	}

	skill_detail_requested.emit(skill_name, skill_data)


func _on_skill_hovered(skill_name: String, value: float):
	"""스킬 호버 처리"""
	# 툴팁 표시 등
	pass


func _on_tab_changed(tab: int):
	"""탭 변경 처리"""
	var categories = SKILL_CATEGORIES.keys()
	if tab < categories.size():
		print("[Enhanced42StatsScreen] Tab changed to: %s" % categories[tab])


func _on_back_pressed():
	"""뒤로가기 버튼 처리"""
	_apply_touch_feedback()
	back_requested.emit()


func _get_skill_grade(value: float) -> String:
	"""스킬 등급 계산"""
	if value >= 90:
		return "S"
	elif value >= 80:
		return "A"
	elif value >= 70:
		return "B"
	elif value >= 60:
		return "C"
	else:
		return "D"


func _get_skill_description(skill_name: String) -> String:
	"""스킬 설명 반환"""
	# TODO: 상세 스킬 설명 데이터베이스 구축
	return "스킬에 대한 상세 설명이 여기에 표시됩니다."


# 플레이어 데이터와 동기화
func sync_with_player_data():
	"""PlayerData와 동기화"""
	_load_player_skills()


# 애니메이션 효과들
func play_skill_up_animation(skill_name: String):
	"""스킬 상승 애니메이션"""
	if skill_widgets.has(skill_name):
		var widget = skill_widgets[skill_name]
		var tween = create_tween()

		# 스케일 + 색상 효과
		tween.set_parallel(true)
		tween.tween_property(widget, "scale", Vector2(1.1, 1.1), 0.2)
		tween.tween_property(widget, "modulate", ThemeManager.SUCCESS, 0.2)

		tween.tween_property(widget, "scale", Vector2(1.0, 1.0), 0.3).set_delay(0.2)
		tween.tween_property(widget, "modulate", Color.WHITE, 0.3).set_delay(0.2)


# 데이터 내보내기 (개발용)
func export_skill_data() -> Dictionary:
	"""현재 스킬 데이터 내보내기"""
	return skill_data_cache.duplicate()


# 통계 정보 제공
func get_skill_statistics() -> Dictionary:
	"""스킬 통계 정보"""
	var stats = {
		"highest": {"name": "", "value": 0.0},
		"lowest": {"name": "", "value": 100.0},
		"average": 0.0,
		"category_averages": {}
	}

	var total = 0.0
	var count = 0

	for skill_name in skill_data_cache.keys():
		var value = skill_data_cache[skill_name]
		total += value
		count += 1

		if value > stats.highest.value:
			stats.highest = {"name": skill_name, "value": value}
		if value < stats.lowest.value:
			stats.lowest = {"name": skill_name, "value": value}

	if count > 0:
		stats.average = total / count

	# 카테고리별 평균
	for category in SKILL_CATEGORIES.keys():
		var cat_total = 0.0
		var cat_count = 0
		for skill_name in SKILL_CATEGORIES[category]["skills"]:
			if skill_data_cache.has(skill_name):
				cat_total += skill_data_cache[skill_name]
				cat_count += 1
		if cat_count > 0:
			stats.category_averages[category] = cat_total / cat_count

	return stats


func setup_mobile_adapters():
	"""모바일 적응 유틸리티 설정"""
	print("[Enhanced42StatsScreen] Setting up mobile adapters...")

	# MobileLayoutAdapter 통합
	var mobile_adapter = MobileLayoutAdapter.new()
	add_child(mobile_adapter)
	mobile_adapter.setup_responsive_layout(self)

	# SwipeNavigator 통합
	var swipe_nav = SwipeNavigator.new()
	add_child(swipe_nav)
	swipe_nav.connect("swipe_left", _on_swipe_left)
	swipe_nav.connect("swipe_right", _on_swipe_right)

	# AccessibilityManager 통합
	# AccessibilityManager.connect("accessibility_changed", _on_accessibility_changed)  # 비활성화: AccessibilityManager 없음

	print("[Enhanced42StatsScreen] Mobile adapters setup complete!")

	# 화면 크기별 레이아웃 조정
	adapt_layout_to_screen_size()


func _on_swipe_left():
	"""왼쪽 스와이프 - 이전 탭으로 이동"""
	print("[Enhanced42StatsScreen] Swipe left detected")
	if tab_container:
		var current_tab = tab_container.current_tab
		if current_tab > 0:
			tab_container.current_tab = current_tab - 1


func _on_swipe_right():
	"""오른쪽 스와이프 - 다음 탭으로 이동"""
	print("[Enhanced42StatsScreen] Swipe right detected")
	if tab_container:
		var current_tab = tab_container.current_tab
		if current_tab < tab_container.get_tab_count() - 1:
			tab_container.current_tab = current_tab + 1


func _on_accessibility_changed():
	"""접근성 설정 변경 시 호출"""
	print("[Enhanced42StatsScreen] Accessibility settings changed")
	# if AccessibilityManager.high_contrast_mode:
	#	apply_high_contrast_theme()
	# if AccessibilityManager.large_text_mode:
	#	apply_large_text_scaling()


func apply_high_contrast_theme():
	"""고대비 테마 적용"""
	print("[Enhanced42StatsScreen] Applying high contrast theme")

	# 고대비 색상 팔레트
	var high_contrast_colors = {
		"background": Color(0.0, 0.0, 0.0),  # 검은 배경
		"foreground": Color(1.0, 1.0, 1.0),  # 흰 텍스트
		"accent": Color(1.0, 1.0, 0.0),  # 노란 강조
		"success": Color(0.0, 1.0, 0.0),  # 녹색 성공
		"warning": Color(1.0, 1.0, 0.0),  # 노란 경고
		"danger": Color(1.0, 0.0, 0.0)  # 빨간 위험
	}

	# 배경색 적용
	var background = get_node_or_null("VBox")
	if background:
		background.modulate = high_contrast_colors.background

	# 텍스트 색상 적용
	_apply_text_colors(high_contrast_colors.foreground)

	# 버튼 색상 적용
	_apply_button_colors(high_contrast_colors)

	# 스킬 위젯 색상 적용
	_apply_skill_widget_colors(high_contrast_colors)


func apply_large_text_scaling():
	"""큰 텍스트 스케일링 적용"""
	print("[Enhanced42StatsScreen] Applying large text scaling")

	# 텍스트 스케일링 팩터
	var text_scale_factor = 1.5

	# 모든 라벨과 버튼의 폰트 크기 증가
	var all_labels = get_tree().get_nodes_in_group("ui_labels")
	for label in all_labels:
		if label is Label:
			var current_size = label.get_theme_font_size("font_size")
			label.add_theme_font_size_override("font_size", current_size * text_scale_factor)

	var all_buttons = get_tree().get_nodes_in_group("ui_buttons")
	for button in all_buttons:
		if button is BaseButton:
			var current_size = button.get_theme_font_size("font_size")
			button.add_theme_font_size_override("font_size", current_size * text_scale_factor)


func _apply_text_colors(color: Color):
	"""텍스트 색상 적용"""
	var labels = get_tree().get_nodes_in_group("ui_labels")
	for label in labels:
		if label is Label:
			label.add_theme_color_override("font_color", color)


func _apply_button_colors(colors: Dictionary):
	"""버튼 색상 적용"""
	var buttons = get_tree().get_nodes_in_group("ui_buttons")
	for button in buttons:
		if button is BaseButton:
			button.add_theme_color_override("font_color", colors.foreground)
			button.add_theme_color_override("font_color_pressed", colors.accent)


func _apply_skill_widget_colors(colors: Dictionary):
	"""스킬 위젯 색상 적용"""
	var skill_widgets = get_tree().get_nodes_in_group("skill_widgets")
	for widget in skill_widgets:
		if widget is Control:
			widget.modulate = colors.foreground


func adapt_layout_to_screen_size():
	"""화면 크기에 따른 레이아웃 조정"""
	print("[Enhanced42StatsScreen] Adapting layout to screen size...")

	var screen_size = get_viewport().get_visible_rect().size
	var screen_width = screen_size.x
	var screen_height = screen_size.y

	# 화면 크기 분류
	var current_screen_size: ScreenSize

	if screen_width < 720:  # 모바일
		current_screen_size = ScreenSize.SMALL
	elif screen_width < 1280:  # 태블릿
		current_screen_size = ScreenSize.MEDIUM
	else:  # 데스크톱
		current_screen_size = ScreenSize.LARGE

	print(
		(
			"[Enhanced42StatsScreen] Screen size: %s (%dx%d)"
			% [["SMALL", "MEDIUM", "LARGE"][current_screen_size], screen_width, screen_height]
		)
	)

	# 스킬 그리드 컬럼 수 조정
	adapt_skill_grid_layout(current_screen_size)

	# 기타 레이아웃 조정
	adapt_other_layouts(current_screen_size)


func adapt_skill_grid_layout(screen_size: ScreenSize):
	"""스킬 그리드 레이아웃 조정"""
	var skill_grids = get_tree().get_nodes_in_group("skill_grids")
	if skill_grids.is_empty():
		# TabContainer 내의 모든 GridContainer 찾기
		skill_grids = _find_all_grid_containers()

	for grid in skill_grids:
		if grid is GridContainer:
			match screen_size:
				ScreenSize.SMALL:  # 모바일: 1열
					grid.columns = 1
					print("[Enhanced42StatsScreen] Skill grid: 1 column (mobile)")
				ScreenSize.MEDIUM:  # 태블릿: 2열
					grid.columns = 2
					print("[Enhanced42StatsScreen] Skill grid: 2 columns (tablet)")
				ScreenSize.LARGE:  # 데스크톱: 3열
					grid.columns = 3
					print("[Enhanced42StatsScreen] Skill grid: 3 columns (desktop)")


func _find_all_grid_containers() -> Array:
	"""모든 GridContainer 찾기"""
	var grids = []
	var tab_container = get_node_or_null("VBox/TabContainer")
	if tab_container:
		for i in range(tab_container.get_tab_count()):
			var tab = tab_container.get_tab_control(i)
			_find_grids_recursive(tab, grids)
	return grids


func _find_grids_recursive(node: Node, grids: Array):
	"""재귀적으로 GridContainer 찾기"""
	if node is GridContainer:
		grids.append(node)
	for child in node.get_children():
		_find_grids_recursive(child, grids)


func adapt_other_layouts(screen_size: ScreenSize):
	"""기타 레이아웃 조정"""
	# 헤더 높이 조정
	var header = get_node_or_null("VBox/Header")
	if header:
		match screen_size:
			ScreenSize.SMALL:
				header.custom_minimum_size = Vector2(0, 60)
			ScreenSize.MEDIUM:
				header.custom_minimum_size = Vector2(0, 70)
			ScreenSize.LARGE:
				header.custom_minimum_size = Vector2(0, 80)

	# 검색바 높이 조정
	var search_bar = get_node_or_null("VBox/SearchBar")
	if search_bar:
		match screen_size:
			ScreenSize.SMALL:
				search_bar.custom_minimum_size = Vector2(0, 50)
			ScreenSize.MEDIUM:
				search_bar.custom_minimum_size = Vector2(0, 60)
			ScreenSize.LARGE:
				search_bar.custom_minimum_size = Vector2(0, 70)


func _apply_touch_feedback():
	"""터치 피드백 적용"""
	# 햅틱 피드백
	if OS.has_feature("mobile"):
		Input.vibrate_handheld(50)  # 50ms 진동

	# 시각적 피드백 (버튼 스케일 애니메이션)
	var button = get_viewport().gui_get_focus_owner()
	if button and button is BaseButton:
		var tween = create_tween()
		tween.set_parallel(true)
		tween.tween_property(button, "scale", Vector2(0.95, 0.95), 0.1)
		tween.tween_property(button, "scale", Vector2(1.0, 1.0), 0.1).set_delay(0.1)
