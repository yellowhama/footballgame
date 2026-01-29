extends Control
# StatusScreen 개선 - 탭 시스템과 HexagonChart 통합

# Autoload references (safe access)
var PlayerData = null
var ThemeManager = null
var SceneColorUpdater = null
var ResponsiveLayoutFixer = null

# Using unique name syntax (%) for nodes marked with unique_name_in_owner in the scene
@onready var tab_container: TabContainer = %TabContainer
@onready var hexagon_chart = %HexagonChart
@onready var back_button: Button = %BackButton
@onready var info_button: Button = %InfoButton
@onready var detail_view_button: Button = %DetailViewButton
# Note: SpecialAbilityButton does not exist in scene, using get_node_or_null for safety
@onready var special_ability_button = get_node_or_null("TabContainer/Overview/VBox/SpecialAbilityButton")

# Style sections (using unique name syntax)
@onready var header: Panel = %Header
@onready var ovr_section: Panel = %OVRSection
@onready var condition_section: Panel = %ConditionSection

# Overview 탭
@onready var ovr_value: Label = %OVRValue
@onready var potential_label: Label = %Potential
@onready var growth_label: Label = %Growth
@onready var condition_emoji: Label = %Emoji
@onready var condition_text: Label = %ConditionText
@onready var fatigue_bar: ProgressBar = %FatigueBar
@onready var injury_value: Label = %InjuryValue

# 스탯 표시 컨테이너들
var stat_panels = {}
# 42개 전체 능력치 패널들 (새로운 Attributes 탭용)
var attributes_panels = {}

# QuickBar support
var quickbar: QuickBar

# Fallback colors when ThemeManager is not available
const FALLBACK_SUCCESS = Color(0.2, 1.0, 0.2)
const FALLBACK_WARNING = Color(1.0, 1.0, 0.2)
const FALLBACK_DANGER = Color(1.0, 0.2, 0.2)
const FALLBACK_BG_SURFACE = Color(0.1, 0.1, 0.1)
const FALLBACK_PASTEL_GREEN = Color(0.6, 1.0, 0.6)
const FALLBACK_PASTEL_YELLOW = Color(1.0, 1.0, 0.6)
const FALLBACK_CORNER_RADIUS_MEDIUM = 8

# 네비바 씬
const MainNavBarScene = preload("res://scenes/components/MainNavBar.tscn")


func _get_stat_color(value: float, max_value: float) -> Color:
	if ThemeManager:
		return ThemeManager.get_stat_color(value, max_value)
	else:
		# Simple fallback color logic
		var ratio = value / max_value
		if ratio >= 0.8:
			return FALLBACK_SUCCESS
		elif ratio >= 0.6:
			return FALLBACK_PASTEL_GREEN
		elif ratio >= 0.4:
			return FALLBACK_PASTEL_YELLOW
		else:
			return FALLBACK_DANGER


func _ready():
	print("[StatusScreenImproved] Initializing...")

	# Get autoload references
	PlayerData = get_node_or_null("/root/PlayerData")
	ThemeManager = get_node_or_null("/root/ThemeManager")
	SceneColorUpdater = get_node_or_null("/root/SceneColorUpdater")
	ResponsiveLayoutFixer = get_node_or_null("/root/ResponsiveLayoutFixer")

	# ColorSystem 적용
	if SceneColorUpdater:
		SceneColorUpdater.apply_color_system_to_scene(self)

	# 반응형 레이아웃 수정
	if ResponsiveLayoutFixer:
		ResponsiveLayoutFixer.fix_scene_layout(self)

	# 터치 피드백 적용
	# TouchFeedback class doesn't exist - TODO: implement if needed

	# 스타일 적용
	_apply_custom_styles()

	# 시그널 연결
	_connect_signals()

	# 스탯 패널 수집
	_collect_stat_panels()

	# 42개 능력치 패널 수집
	_collect_attributes_panels()

	# QuickBar 초기화
	_initialize_quickbar()

	# 초기 데이터 로드
	_load_player_data()

	# 네비게이션 바 추가
	_add_navigation_bar()

	print("[StatusScreenImproved] Ready!")


func _add_navigation_bar() -> void:
	if MainNavBarScene:
		var navbar = MainNavBarScene.instantiate()
		add_child(navbar)
		navbar.set_active_tab("player")


func _apply_custom_styles():
	# 헤더 스타일 (using @onready header)
	if header:
		# TODO: CustomStyles not implemented yet
		# header.add_theme_stylebox_override("panel", CustomStyles.create_header_panel())
		pass

	# 버튼 스타일
	if back_button:
		# TODO: CustomStyles not implemented yet
		# back_button.add_theme_stylebox_override("normal", CustomStyles.create_ghost_button())
		pass

	# Overview 섹션 스타일 (using @onready ovr_section)
	if ovr_section:
		# TODO: CustomStyles not implemented yet
		# ovr_section.add_theme_stylebox_override("panel", CustomStyles.create_card_panel())
		pass

	# Condition 섹션 스타일 (using @onready condition_section)
	if condition_section:
		# TODO: CustomStyles not implemented yet
		# condition_section.add_theme_stylebox_override("panel", CustomStyles.create_card_panel())
		pass

	# 스탯 패널 스타일 적용
	_apply_stat_panel_styles()


func _apply_stat_panel_styles():
	# Technical 탭 스탯 패널들
	var technical_stats = ["FirstTouch", "Dribbling", "Passing", "Vision"]
	for stat_name in technical_stats:
		var panel = get_node_or_null("TabContainer/Technical/VBox/StatsContainer/" + stat_name)
		if panel:
			panel.add_theme_stylebox_override("panel", _create_stat_panel_style())

	# Physical 탭 스탯 패널들
	var physical_stats = ["Pace", "Acceleration", "Stamina"]
	for stat_name in physical_stats:
		var panel = get_node_or_null("TabContainer/Physical/VBox/StatsContainer/" + stat_name)
		if panel:
			panel.add_theme_stylebox_override("panel", _create_stat_panel_style())

	# Mental 탭 스탯 패널들
	var mental_stats = ["Composure"]
	for stat_name in mental_stats:
		var panel = get_node_or_null("TabContainer/Mental/VBox/StatsContainer/" + stat_name)
		if panel:
			panel.add_theme_stylebox_override("panel", _create_stat_panel_style())


func _create_stat_panel_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = FALLBACK_BG_SURFACE
	style.corner_radius_top_left = FALLBACK_CORNER_RADIUS_MEDIUM
	style.corner_radius_top_right = FALLBACK_CORNER_RADIUS_MEDIUM
	style.corner_radius_bottom_left = FALLBACK_CORNER_RADIUS_MEDIUM
	style.corner_radius_bottom_right = FALLBACK_CORNER_RADIUS_MEDIUM
	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 15
	style.content_margin_bottom = 15
	return style


func _connect_signals():
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	if info_button:
		info_button.pressed.connect(_on_info_pressed)

	if detail_view_button:
		detail_view_button.pressed.connect(_on_detail_view_pressed)

	if special_ability_button:
		special_ability_button.pressed.connect(_on_special_ability_pressed)

	if hexagon_chart:
		hexagon_chart.stat_hovered.connect(_on_stat_hovered)
		hexagon_chart.stat_clicked.connect(_on_stat_clicked)

	if tab_container:
		tab_container.tab_changed.connect(_on_tab_changed)


func _initialize_quickbar():
	"""QuickBar 초기화 및 신호 연결"""
	if has_node("QuickBar"):
		quickbar = %QuickBar
		if quickbar:
			print("[StatusScreenImproved] QuickBar found, connecting signals...")
			# 신호 연결
			quickbar.skip.connect(_on_quickbar_skip)
			quickbar.toggle_auto.connect(_on_quickbar_auto_toggled)
			quickbar.change_speed.connect(_on_quickbar_speed_changed)
			quickbar.change_highlight.connect(_on_quickbar_highlight_changed)
			quickbar.open_log.connect(_on_quickbar_log_opened)

			# 기본 설정 적용 (StatusScreen에 적합한 설정)
			var quickbar_vm = {"autoEnabled": false, "currentSpeed": 1, "highlightLevel": 3, "visible": true}  # StatusScreen에서는 더 자세한 하이라이트
			quickbar.apply_view_model(quickbar_vm)
		else:
			print("[StatusScreenImproved] QuickBar node found but not valid")
	else:
		print("[StatusScreenImproved] QuickBar node not found in scene")


# QuickBar 신호 핸들러들
func _on_quickbar_skip():
	"""QuickBar Skip 버튼 처리"""
	print("[StatusScreenImproved] QuickBar Skip pressed")
	# StatusScreen에서는 뒤로가기 처리
	_on_back_pressed()


func _on_quickbar_auto_toggled(enabled: bool):
	"""QuickBar Auto 토글 처리"""
	print("[StatusScreenImproved] QuickBar Auto toggled: ", enabled)
	# Auto 모드에서는 스탯 업데이트 자동화
	if enabled:
		# 주기적으로 스탯 업데이트
		var timer = Timer.new()
		timer.wait_time = 2.0
		timer.timeout.connect(_load_player_data)
		add_child(timer)
		timer.start()


func _on_quickbar_speed_changed(speed: int):
	"""QuickBar 속도 변경 처리"""
	print("[StatusScreenImproved] QuickBar Speed changed to: ", speed)
	# 애니메이션 속도 조절
	# HexagonChart 업데이트 속도 조절 가능


func _on_quickbar_highlight_changed(level: int):
	"""QuickBar 하이라이트 레벨 변경 처리"""
	print("[StatusScreenImproved] QuickBar Highlight level changed to: ", level)
	# 하이라이트 레벨에 따른 정보 표시 조절
	# Level 1: 기본 스탯만
	# Level 2: + 6각형 차트
	# Level 3: + 상세 스탯
	# Level 4: + 모든 정보


func _on_quickbar_log_opened():
	"""QuickBar Log 버튼 처리"""
	print("[StatusScreenImproved] QuickBar Log opened")
	# 스탯 변화 로그 표시
	# 추후 스탯 변화 히스토리 구현


func _collect_stat_panels():
	# Technical 스탯들
	stat_panels["first_touch"] = {
		"value": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/FirstTouch/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/FirstTouch/HBox/Bar")
	}
	stat_panels["dribbling"] = {
		"value": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/Dribbling/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/Dribbling/HBox/Bar")
	}
	stat_panels["passing"] = {
		"value": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/Passing/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/Passing/HBox/Bar")
	}
	stat_panels["vision"] = {
		"value": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/Vision/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Technical/VBox/StatsContainer/Vision/HBox/Bar")
	}

	# Physical 스탯들
	stat_panels["pace"] = {
		"value": get_node_or_null("TabContainer/Physical/VBox/StatsContainer/Pace/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Physical/VBox/StatsContainer/Pace/HBox/Bar")
	}
	stat_panels["acceleration"] = {
		"value": get_node_or_null("TabContainer/Physical/VBox/StatsContainer/Acceleration/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Physical/VBox/StatsContainer/Acceleration/HBox/Bar")
	}
	stat_panels["stamina"] = {
		"value": get_node_or_null("TabContainer/Physical/VBox/StatsContainer/Stamina/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Physical/VBox/StatsContainer/Stamina/HBox/Bar")
	}

	# Mental 스탯들
	stat_panels["composure"] = {
		"value": get_node_or_null("TabContainer/Mental/VBox/StatsContainer/Composure/HBox/Value"),
		"bar": get_node_or_null("TabContainer/Mental/VBox/StatsContainer/Composure/HBox/Bar")
	}


func _collect_attributes_panels():
	"""새로운 Attributes 탭의 42개 능력치 패널들을 수집"""
	# Technical 능력치 (14개)
	var technical_attrs = [
		"Corners",
		"Crossing",
		"Dribbling",
		"Finishing",
		"FirstTouch",
		"FreeKicks",
		"Heading",
		"LongShots",
		"LongThrows",
		"Marking",
		"Passing",
		"PenaltyTaking",
		"Tackling",
		"Technique"
	]

	for attr in technical_attrs:
		attributes_panels[attr.to_lower()] = {
			"value":
			get_node_or_null("TabContainer/Attributes/VBox/TechnicalSection/TechnicalGrid/" + attr + "/HBox/Value"),
			"bar": get_node_or_null("TabContainer/Attributes/VBox/TechnicalSection/TechnicalGrid/" + attr + "/HBox/Bar")
		}

	# Mental 능력치 (14개)
	var mental_attrs = [
		"Aggression",
		"Anticipation",
		"Bravery",
		"Composure",
		"Concentration",
		"Decisions",
		"Determination",
		"Flair",
		"Leadership",
		"OffTheBall",
		"Positioning",
		"Teamwork",
		"Vision",
		"WorkRate"
	]

	for attr in mental_attrs:
		var key = attr.to_lower()
		if key == "offtheball":
			key = "off_the_ball"
		elif key == "workrate":
			key = "work_rate"

		attributes_panels[key] = {
			"value": get_node_or_null("TabContainer/Attributes/VBox/MentalSection/MentalGrid/" + attr + "/HBox/Value"),
			"bar": get_node_or_null("TabContainer/Attributes/VBox/MentalSection/MentalGrid/" + attr + "/HBox/Bar")
		}

	# Physical 능력치 (8개)
	var physical_attrs = [
		"Speed", "Stamina", "Strength", "Agility", "Balance", "Jumping", "NaturalFitness", "Acceleration"
	]

	for attr in physical_attrs:
		var key = attr.to_lower()
		if key == "naturalfitness":
			key = "natural_fitness"

		attributes_panels[key] = {
			"value":
			get_node_or_null("TabContainer/Attributes/VBox/PhysicalSection/PhysicalGrid/" + attr + "/HBox/Value"),
			"bar": get_node_or_null("TabContainer/Attributes/VBox/PhysicalSection/PhysicalGrid/" + attr + "/HBox/Bar")
		}

	# Goalkeeper 능력치 (6개)
	var gk_attrs = ["Reflexes", "Handling", "AerialAbility", "CommandOfArea", "Communication", "Kicking"]

	for attr in gk_attrs:
		var key = attr.to_lower()
		if key == "aerialability":
			key = "aerial_ability"
		elif key == "commandofarea":
			key = "command_of_area"

		attributes_panels[key] = {
			"value":
			get_node_or_null("TabContainer/Attributes/VBox/GoalkeeperSection/GoalkeeperGrid/" + attr + "/HBox/Value"),
			"bar":
			get_node_or_null("TabContainer/Attributes/VBox/GoalkeeperSection/GoalkeeperGrid/" + attr + "/HBox/Bar")
		}

	print("[StatusScreenImproved] Collected %d attribute panels" % attributes_panels.size())


func _load_player_data():
	if not PlayerData:
		print("[StatusScreenImproved] PlayerData not available")
		return

	# Overview 업데이트
	_update_overview()

	# HexagonChart 업데이트
	_update_hexagon_chart()

	# 각 탭의 스탯 업데이트
	_update_all_stats()

	# 42개 능력치 업데이트
	_update_attributes_display()


func _update_overview():
	if not PlayerData:
		return
	# OVR 계산
	if ovr_value:
		var ovr = PlayerData.get_overall_rating()
		ovr_value.text = str(int(ovr))
		ovr_value.modulate = _get_stat_color(ovr, 100.0)

	# 잠재력 (임시 값)
	if potential_label:
		potential_label.text = "잠재력: 85"

	# 성장률 계산 (임시)
	if growth_label:
		growth_label.text = "성장률: +12 (이번 학기)"

	# 컨디션 업데이트
	_update_condition_display()

	# 피로도 업데이트
	_update_fatigue_display()

	# 부상 상태
	_update_injury_display()


func _update_condition_display():
	if not PlayerData:
		return
	var condition = PlayerData.condition if "condition" in PlayerData else 5

	if condition_emoji:
		match condition:
			5:
				condition_emoji.text = "😄"
			4:
				condition_emoji.text = "🙂"
			3:
				condition_emoji.text = "😐"
			2:
				condition_emoji.text = "😟"
			1:
				condition_emoji.text = "😫"
			_:
				condition_emoji.text = "😴"

	if condition_text:
		var text = ""
		var color = Color.WHITE
		match condition:
			5:
				text = " 최상 (5/5)"
				color = FALLBACK_SUCCESS
			4:
				text = " 좋음 (4/5)"
				color = FALLBACK_PASTEL_GREEN
			3:
				text = " 보통 (3/5)"
				color = FALLBACK_PASTEL_YELLOW
			2:
				text = " 나쁨 (2/5)"
				color = FALLBACK_WARNING
			1:
				text = " 최악 (1/5)"
				color = FALLBACK_DANGER
		condition_text.text = text
		condition_text.modulate = color


func _update_fatigue_display():
	if not PlayerData:
		return
	if not fatigue_bar:
		return

	var fatigue = PlayerData.fatigue if "fatigue" in PlayerData else 0.0
	fatigue_bar.value = fatigue

	# 피로도에 따른 바 색상 변경
	# TODO: CustomStyles not implemented yet
	# var fill_style = CustomStyles.create_fatigue_bar_fill(fatigue)
	var fill_style = null  # Temporary fix
	# fatigue_bar.add_theme_stylebox_override("fill", fill_style)  # Disabled until CustomStyles is implemented


func _update_injury_display():
	if not PlayerData:
		return
	if not injury_value:
		return

	var injury_days = PlayerData.injury_days if "injury_days" in PlayerData else 0

	if injury_days > 0:
		injury_value.text = "부상 (%d일 남음)" % injury_days
		injury_value.modulate = FALLBACK_DANGER
	else:
		injury_value.text = "없음"
		injury_value.modulate = FALLBACK_SUCCESS


func _update_hexagon_chart():
	if not hexagon_chart:
		return
	if not PlayerData:
		print("[StatusScreenImproved] PlayerData not available for hexagon chart")
		return

	var all_stats = PlayerData.get_all_stats()
	var technical = all_stats.technical
	var physical = all_stats.physical
	var mental = all_stats.mental  # Although mental isn't directly used for these 6, it might be for others

	var chart_stats = {
		"PACE": physical.get("pace", 50),
		"POWER": physical.get("strength", 50),
		"TECHNICAL": technical.get("technique", 50),
		"SHOOTING": technical.get("finishing", 50),
		"PASSING": technical.get("passing", 50),
		"DEFENDING": technical.get("tackling", 50)
	}

	hexagon_chart.stats = chart_stats
	hexagon_chart.queue_redraw()
	print("[StatusScreenImproved] Hexagon chart updated with PlayerData")


func _update_all_stats():
	if not PlayerData:
		return
	# Technical 스탯
	_update_stat_display("first_touch", PlayerData.get_stat("technical", "first_touch"))
	_update_stat_display("dribbling", PlayerData.get_stat("technical", "dribbling"))
	_update_stat_display("passing", PlayerData.get_stat("technical", "passing"))
	_update_stat_display("vision", PlayerData.get_stat("mental", "vision"))

	# Physical 스탯
	_update_stat_display("pace", PlayerData.get_stat("physical", "pace"))
	_update_stat_display("acceleration", PlayerData.get_stat("physical", "acceleration"))
	_update_stat_display("stamina", PlayerData.get_stat("physical", "stamina"))

	# Mental 스탯
	_update_stat_display("composure", PlayerData.get_stat("mental", "composure"))


func _update_attributes_display():
	"""OpenFootball PlayerAttributes 42개 능력치를 UI에 표시 (Refactored)"""
	if not PlayerData:
		print("[StatusScreenImproved] PlayerData not available for attributes display")
		return

	var all_stats = PlayerData.get_all_stats()
	var technical_stats = all_stats.technical
	var mental_stats = all_stats.mental
	var physical_stats = all_stats.physical
	var goalkeeper_stats = all_stats.goalkeeper

	# Technical attributes (14)
	_update_single_attribute("corners", technical_stats.get("corners", 0))
	_update_single_attribute("crossing", technical_stats.get("crossing", 0))
	_update_single_attribute("dribbling", technical_stats.get("dribbling", 0))
	_update_single_attribute("finishing", technical_stats.get("finishing", 0))
	_update_single_attribute("first_touch", technical_stats.get("first_touch", 0))
	_update_single_attribute("free_kicks", technical_stats.get("free_kicks", 0))
	_update_single_attribute("heading", technical_stats.get("heading", 0))
	_update_single_attribute("long_shots", technical_stats.get("long_shots", 0))
	_update_single_attribute("long_throws", technical_stats.get("long_throws", 0))
	_update_single_attribute("marking", technical_stats.get("marking", 0))
	_update_single_attribute("passing", technical_stats.get("passing", 0))
	_update_single_attribute("penalty_taking", technical_stats.get("penalty_kicks", 0))  # Mapped from penalty_kicks
	_update_single_attribute("tackling", technical_stats.get("tackling", 0))
	_update_single_attribute("technique", technical_stats.get("technique", 0))

	# Mental attributes (14)
	_update_single_attribute("aggression", mental_stats.get("aggression", 0))
	_update_single_attribute("anticipation", mental_stats.get("anticipation", 0))
	_update_single_attribute("bravery", mental_stats.get("bravery", 0))
	_update_single_attribute("composure", mental_stats.get("composure", 0))
	_update_single_attribute("concentration", mental_stats.get("concentration", 0))
	_update_single_attribute("decisions", mental_stats.get("decisions", 0))
	_update_single_attribute("determination", mental_stats.get("determination", 0))
	_update_single_attribute("flair", mental_stats.get("flair", 0))
	_update_single_attribute("leadership", mental_stats.get("leadership", 0))
	_update_single_attribute("off_the_ball", mental_stats.get("off_the_ball", 0))
	_update_single_attribute("positioning", mental_stats.get("positioning", 0))
	_update_single_attribute("teamwork", mental_stats.get("teamwork", 0))
	_update_single_attribute("vision", mental_stats.get("vision", 0))
	_update_single_attribute("work_rate", mental_stats.get("work_rate", 0))

	# Physical attributes (8)
	_update_single_attribute("pace", physical_stats.get("pace", 0))
	_update_single_attribute("acceleration", physical_stats.get("acceleration", 0))
	_update_single_attribute("stamina", physical_stats.get("stamina", 0))
	_update_single_attribute("strength", physical_stats.get("strength", 0))
	_update_single_attribute("agility", physical_stats.get("agility", 0))
	_update_single_attribute("balance", physical_stats.get("balance", 0))
	_update_single_attribute("jumping", physical_stats.get("jumping", 0))
	_update_single_attribute("natural_fitness", physical_stats.get("natural_fitness", 0))

	# Goalkeeper attributes (6) - Note: PlayerData.gd uses different GK stat names
	_update_single_attribute("reflexes", goalkeeper_stats.get("reflexes", 0))  # Reflexes is not in PlayerData.gd, using 0
	_update_single_attribute("handling", goalkeeper_stats.get("handling", 0))
	_update_single_attribute("aerial_ability", goalkeeper_stats.get("aerial_reach", 0))  # Mapped from aerial_reach
	_update_single_attribute("command_of_area", goalkeeper_stats.get("command_of_area", 0))
	_update_single_attribute("communication", goalkeeper_stats.get("communication", 0))
	_update_single_attribute("kicking", goalkeeper_stats.get("kicking", 0))

	print("[StatusScreenImproved] Updated all 42 attributes display with PlayerData")


func _update_single_attribute(attr_key: String, value):
	"""단일 능력치를 UI에 표시"""
	if not attributes_panels.has(attr_key):
		return

	var panel = attributes_panels[attr_key]
	var display_value = int(value) if value != null else 50

	if panel["value"]:
		panel["value"].text = str(display_value)
		panel["value"].modulate = _get_stat_color(display_value, 100.0)

	if panel["bar"]:
		panel["bar"].value = display_value

		# 바 색상 설정
		var fill_style = StyleBoxFlat.new()
		fill_style.bg_color = _get_stat_color(display_value, 100.0)
		fill_style.corner_radius_top_left = 8
		fill_style.corner_radius_top_right = 8
		fill_style.corner_radius_bottom_left = 8
		fill_style.corner_radius_bottom_right = 8
		panel["bar"].add_theme_stylebox_override("fill", fill_style)


func _update_stat_display(stat_name: String, value: float):
	if not stat_panels.has(stat_name):
		return

	var panel = stat_panels[stat_name]

	if panel["value"]:
		panel["value"].text = str(int(value))
		panel["value"].modulate = _get_stat_color(value, 100.0)

	if panel["bar"]:
		panel["bar"].value = value

		# 바 색상 설정
		var fill_style = StyleBoxFlat.new()
		fill_style.bg_color = _get_stat_color(value, 100.0)
		fill_style.corner_radius_top_left = 8
		fill_style.corner_radius_top_right = 8
		fill_style.corner_radius_bottom_left = 8
		fill_style.corner_radius_bottom_right = 8
		panel["bar"].add_theme_stylebox_override("fill", fill_style)


func _on_back_pressed():
	print("[StatusScreenImproved] Back button pressed")
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_info_pressed():
	print("[StatusScreenImproved] Info button pressed")
	# 도움말 팝업 표시


func _on_detail_view_pressed():
	print("[StatusScreenImproved] Detail view button pressed")
	# TODO: 42개 상세 속성 UI 구현 필요 (현재 미구현)
	# get_tree().change_scene_to_file("res://scenes/Enhanced42StatsScreen.tscn")
	print("[StatusScreenImproved] Detail view - Coming Soon")


func _on_special_ability_pressed():
	print("[StatusScreenImproved] Special Ability button pressed")
	# Navigate to Special Ability Screen (Power Pro style)
	get_tree().change_scene_to_file("res://scenes/SpecialAbilityScreen.tscn")


func _on_stat_hovered(stat_name: String, value: float):
	print("[StatusScreenImproved] Stat hovered: %s = %.1f" % [stat_name, value])


func _on_stat_clicked(stat_name: String, value: float):
	print("[StatusScreenImproved] Stat clicked: %s = %.1f" % [stat_name, value])
	# 스탯 상세 정보 팝업


func _on_tab_changed(tab_index: int):
	var tab_names = ["Overview", "Attributes", "Technical", "Physical", "Mental"]
	if tab_index < tab_names.size():
		print("[StatusScreenImproved] Tab changed to: %s" % tab_names[tab_index])
	else:
		print("[StatusScreenImproved] Tab changed to index: %d" % tab_index)

	# 탭 전환 애니메이션
	var tween = get_tree().create_tween()
	tween.tween_property(tab_container, "modulate:a", 0.0, 0.1)
	tween.tween_property(tab_container, "modulate:a", 1.0, 0.2)
