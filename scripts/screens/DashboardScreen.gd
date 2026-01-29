extends Control
## DashboardScreen - 게임 상태 대시보드
## 다음 경기, 선수 상태, 팀 상태를 한눈에 표시
##
## 작성일: 2025-11-26
## 참조: 04_ui_design_system.md

signal screen_changed(screen_name: String)

# ============================================
# UI 노드 참조
# ============================================

@onready var title_label: Label = $Header/TitleLabel
@onready var week_label: Label = $Header/WeekLabel

# 위젯 컨테이너
@onready var widgets_grid: GridContainer = $Content/WidgetsGrid
@onready var next_match_widget: PanelContainer = $Content/WidgetsGrid/NextMatchWidget
@onready var player_status_widget: PanelContainer = $Content/WidgetsGrid/PlayerStatusWidget
@onready var team_stats_widget: PanelContainer = $Content/WidgetsGrid/TeamStatsWidget
@onready var quick_actions_widget: PanelContainer = $Content/WidgetsGrid/QuickActionsWidget

# 빠른 액션 버튼
@onready var training_button: Button = $Footer/QuickButtons/TrainingButton
@onready var match_button: Button = $Footer/QuickButtons/MatchButton
@onready var gacha_button: Button = $Footer/QuickButtons/GachaButton
@onready var tactics_button: Button = $Footer/QuickButtons/TacticsButton

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG_PRIMARY = Color("#0D1117")
const COLOR_BG_SECONDARY = Color("#161B22")
const COLOR_BG_ELEVATED = Color("#30363D")
const COLOR_ACCENT_PRIMARY = Color("#238636")
const COLOR_ACCENT_SECONDARY = Color("#1F6FEB")
const COLOR_ACCENT_WARNING = Color("#D29922")
const COLOR_ACCENT_DANGER = Color("#DA3633")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")

# 컨디션 색상
const CONDITION_COLORS = {
	"절호조": Color("#FF4444"),  # 빨강
	"호조": Color("#FFD700"),  # 노랑
	"보통": Color("#FFFFFF"),  # 흰색
	"부진": Color("#6699FF"),  # 파랑
	"절부진": Color("#9966FF")  # 보라
}

# 네비바 씬
const MainNavBarScene = preload("res://scenes/components/MainNavBar.tscn")

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_connect_signals()
	_setup_ui()
	_update_all_widgets()
	_add_navigation_bar()
	print("[DashboardScreen] Initialized")


func _connect_signals() -> void:
	# 빠른 액션 버튼
	if training_button:
		training_button.pressed.connect(_on_training_pressed)
	if match_button:
		match_button.pressed.connect(_on_match_pressed)
	if gacha_button:
		gacha_button.pressed.connect(_on_gacha_pressed)
	if tactics_button:
		tactics_button.pressed.connect(_on_tactics_pressed)

	# 매니저 시그널 연결
	if ConditionSystem:
		ConditionSystem.condition_changed.connect(_on_condition_changed)
	if DateManager:
		DateManager.day_started.connect(_on_day_started)
	if MatchManager:
		MatchManager.match_ended.connect(_on_match_ended)


func _setup_ui() -> void:
	if has_node("Background"):
		$Background.color = COLOR_BG_PRIMARY


# ============================================
# 위젯 업데이트
# ============================================


func _update_all_widgets() -> void:
	_update_header()
	_update_next_match_widget()
	_update_player_status_widget()
	_update_team_stats_widget()
	_update_quick_actions_widget()


func _update_header() -> void:
	if title_label:
		title_label.text = "대시보드"

	if week_label and DateManager:
		var week = DateManager.current_week if DateManager.has_method("get") else 1
		var year = DateManager.current_year if DateManager.has_method("get") else 1
		week_label.text = "%d년차 %d주차" % [year, week]


func _update_next_match_widget() -> void:
	"""다음 경기 위젯 업데이트"""
	if not next_match_widget:
		return

	var content = _get_or_create_widget_content(next_match_widget, "다음 경기")

	# 경기 정보 가져오기
	var match_info = _get_next_match_info()

	# 내용 생성
	var info_vbox = VBoxContainer.new()
	info_vbox.add_theme_constant_override("separation", 8)
	content.add_child(info_vbox)

	if match_info.is_empty():
		var no_match = Label.new()
		no_match.text = "예정된 경기 없음"
		no_match.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
		info_vbox.add_child(no_match)
	else:
		# 상대팀
		var opponent_label = Label.new()
		opponent_label.text = "vs %s" % match_info.get("opponent", "Unknown")
		opponent_label.add_theme_font_size_override("font_size", 20)
		opponent_label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
		info_vbox.add_child(opponent_label)

		# 경기 유형
		var type_label = Label.new()
		type_label.text = match_info.get("match_type", "리그전")
		type_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
		info_vbox.add_child(type_label)

		# 홈/원정
		var venue_label = Label.new()
		venue_label.text = "🏠 홈" if match_info.get("is_home", true) else "✈️ 원정"
		venue_label.add_theme_color_override("font_color", COLOR_ACCENT_SECONDARY)
		info_vbox.add_child(venue_label)


func _update_player_status_widget() -> void:
	"""선수 상태 위젯 업데이트"""
	if not player_status_widget:
		return

	var content = _get_or_create_widget_content(player_status_widget, "선수 상태")

	var info_vbox = VBoxContainer.new()
	info_vbox.add_theme_constant_override("separation", 12)
	content.add_child(info_vbox)

	# 컨디션
	var condition_row = _create_status_row("컨디션", _get_condition_info())
	info_vbox.add_child(condition_row)

	# 체력/스태미나
	var stamina_row = _create_status_row("체력", _get_stamina_info())
	info_vbox.add_child(stamina_row)

	# 동기부여
	var motivation_row = _create_status_row("동기", _get_motivation_info())
	info_vbox.add_child(motivation_row)


func _update_team_stats_widget() -> void:
	"""팀 통계 위젯 업데이트"""
	if not team_stats_widget:
		return

	var content = _get_or_create_widget_content(team_stats_widget, "팀 통계")

	var stats = _get_team_stats()

	var info_vbox = VBoxContainer.new()
	info_vbox.add_theme_constant_override("separation", 8)
	content.add_child(info_vbox)

	# 팀명
	var team_label = Label.new()
	team_label.text = stats.get("team_name", "My Team")
	team_label.add_theme_font_size_override("font_size", 18)
	team_label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	info_vbox.add_child(team_label)

	# 전적
	var record_label = Label.new()
	record_label.text = "%d승 %d무 %d패" % [stats.get("wins", 0), stats.get("draws", 0), stats.get("losses", 0)]
	record_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	info_vbox.add_child(record_label)

	# 승률
	var total = stats.get("total_matches", 0)
	var win_rate = 0.0
	if total > 0:
		win_rate = float(stats.get("wins", 0)) / total * 100

	var winrate_label = Label.new()
	winrate_label.text = "승률: %.1f%%" % win_rate
	winrate_label.add_theme_color_override(
		"font_color", COLOR_ACCENT_PRIMARY if win_rate >= 50 else COLOR_TEXT_SECONDARY
	)
	info_vbox.add_child(winrate_label)

	# 로스터 정보
	var roster_label = Label.new()
	roster_label.text = "로스터: %d명" % stats.get("roster_size", 0)
	roster_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	info_vbox.add_child(roster_label)


func _update_quick_actions_widget() -> void:
	"""빠른 액션 위젯 업데이트"""
	if not quick_actions_widget:
		return

	var content = _get_or_create_widget_content(quick_actions_widget, "빠른 액션")

	var buttons_vbox = VBoxContainer.new()
	buttons_vbox.add_theme_constant_override("separation", 8)
	content.add_child(buttons_vbox)

	# 훈련 실행 가능 여부
	var can_train = true
	if TrainingManager:
		var stats = TrainingManager.get_training_stats()
		can_train = stats.get("personal_trainings_completed", 0) < 3

	# 훈련 버튼
	var train_btn = Button.new()
	train_btn.text = "💪 훈련하기" if can_train else "💪 훈련 (한도 도달)"
	train_btn.custom_minimum_size = Vector2(0, 40)
	train_btn.disabled = not can_train
	train_btn.pressed.connect(_on_training_pressed)
	buttons_vbox.add_child(train_btn)

	# 휴식 버튼
	var rest_btn = Button.new()
	rest_btn.text = "😴 휴식하기"
	rest_btn.custom_minimum_size = Vector2(0, 40)
	rest_btn.pressed.connect(_on_rest_pressed)
	buttons_vbox.add_child(rest_btn)

	# 진행 버튼
	var advance_btn = Button.new()
	advance_btn.text = "⏭️ 다음으로"
	advance_btn.custom_minimum_size = Vector2(0, 40)
	advance_btn.pressed.connect(_on_advance_pressed)
	buttons_vbox.add_child(advance_btn)


# ============================================
# 위젯 헬퍼
# ============================================


func _get_or_create_widget_content(widget: PanelContainer, title: String) -> Control:
	"""위젯 내용 컨테이너 가져오기/생성"""
	# 기존 내용 제거
	for child in widget.get_children():
		child.queue_free()

	# 배경색 설정
	var bg = ColorRect.new()
	bg.color = COLOR_BG_SECONDARY
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	widget.add_child(bg)

	# 메인 컨테이너
	var vbox = VBoxContainer.new()
	vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 12)
	widget.add_child(vbox)

	# 마진
	var margin_top = Control.new()
	margin_top.custom_minimum_size = Vector2(0, 12)
	vbox.add_child(margin_top)

	# 제목
	var title_label = Label.new()
	title_label.text = title
	title_label.add_theme_font_size_override("font_size", 16)
	title_label.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(title_label)

	# 구분선
	var separator = HSeparator.new()
	vbox.add_child(separator)

	# 내용 컨테이너
	var content = MarginContainer.new()
	content.add_theme_constant_override("margin_left", 16)
	content.add_theme_constant_override("margin_right", 16)
	content.add_theme_constant_override("margin_bottom", 16)
	content.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_child(content)

	return content


func _create_status_row(label_text: String, value_info: Dictionary) -> HBoxContainer:
	"""상태 행 생성"""
	var row = HBoxContainer.new()

	var label = Label.new()
	label.text = label_text
	label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(label)

	var value = Label.new()
	value.text = value_info.get("text", "-")
	value.add_theme_color_override("font_color", value_info.get("color", COLOR_TEXT_PRIMARY))
	row.add_child(value)

	return row


# ============================================
# 데이터 소스
# ============================================


func _get_next_match_info() -> Dictionary:
	"""다음 경기 정보 가져오기"""
	# DateManager에서 주간 계획 확인
	if DateManager and DateManager.has_method("get_current_weekly_plan"):
		var plan = DateManager.get_current_weekly_plan()
		if plan and plan.has_method("get_match_day"):
			var match_day = plan.get_match_day()
			if match_day:
				return {
					"opponent": match_day.get("opponent", "상대팀"),
					"match_type": match_day.get("match_type", "리그전"),
					"is_home": match_day.get("is_home", true)
				}

	# 기본값 (목업)
	return {"opponent": "라이벌 FC", "match_type": "리그전", "is_home": true}


func _get_condition_info() -> Dictionary:
	"""컨디션 정보 가져오기"""
	if ConditionSystem:
		var name = ConditionSystem.get_condition_name()
		var percentage = ConditionSystem.get_condition_percentage()
		return {"text": "%s (%.0f%%)" % [name, percentage], "color": CONDITION_COLORS.get(name, COLOR_TEXT_PRIMARY)}

	return {"text": "보통 (75%)", "color": COLOR_TEXT_PRIMARY}


func _get_stamina_info() -> Dictionary:
	"""체력 정보 가져오기"""
	if DateManager and DateManager.has_method("get_stamina_status"):
		var status = DateManager.get_stamina_status()
		return {"text": status, "color": COLOR_ACCENT_PRIMARY if "높음" in status else COLOR_TEXT_PRIMARY}

	return {"text": "양호", "color": COLOR_TEXT_PRIMARY}


func _get_motivation_info() -> Dictionary:
	"""동기부여 정보 가져오기"""
	if ConditionSystem:
		var percentage = ConditionSystem.motivation_percentage
		var level = "높음" if percentage >= 70 else "보통" if percentage >= 40 else "낮음"
		return {
			"text": "%s (%.0f%%)" % [level, percentage],
			"color": COLOR_ACCENT_PRIMARY if percentage >= 70 else COLOR_TEXT_PRIMARY
		}

	return {"text": "보통 (60%)", "color": COLOR_TEXT_PRIMARY}


func _get_team_stats() -> Dictionary:
	"""팀 통계 가져오기"""
	if MyTeamManager:
		return {
			"team_name": MyTeamManager.team_name,
			"wins": MyTeamManager.total_wins,
			"draws": MyTeamManager.total_draws,
			"losses": MyTeamManager.total_losses,
			"total_matches": MyTeamManager.total_matches_played,
			"roster_size": MyTeamManager.first_team.size() + MyTeamManager.reserves.size()
		}

	return {"team_name": "My Team", "wins": 0, "draws": 0, "losses": 0, "total_matches": 0, "roster_size": 11}


# ============================================
# 이벤트 핸들러
# ============================================


func _on_training_pressed() -> void:
	get_tree().change_scene_to_file("res://scenes/screens/TrainingScreen.tscn")


func _on_match_pressed() -> void:
	print("[DashboardScreen] Match button pressed")
	# TODO: 경기 화면으로 이동


func _on_gacha_pressed() -> void:
	get_tree().change_scene_to_file("res://scenes/screens/GachaScreen.tscn")


func _on_tactics_pressed() -> void:
	get_tree().change_scene_to_file("res://scenes/screens/TacticsScreen.tscn")


func _on_rest_pressed() -> void:
	if TrainingManager:
		var result = await TrainingManager.perform_rest_activity()
		if result.get("success", false):
			_update_player_status_widget()
			print("[DashboardScreen] Rest completed")


func _on_advance_pressed() -> void:
	print("[DashboardScreen] Advance button pressed")
	# TODO: 다음 턴으로 진행


func _on_condition_changed(_level, _percentage) -> void:
	_update_player_status_widget()


func _on_day_started(_day_info: Dictionary) -> void:
	_update_all_widgets()


func _on_match_ended(_result: Dictionary) -> void:
	_update_team_stats_widget()
	_update_next_match_widget()


# ============================================
# 외부 API
# ============================================


func refresh() -> void:
	"""대시보드 새로고침"""
	_update_all_widgets()


func _add_navigation_bar() -> void:
	"""하단 네비게이션 바 추가"""
	if MainNavBarScene:
		var navbar = MainNavBarScene.instantiate()
		add_child(navbar)
		navbar.set_active_tab("dashboard")
