extends Control
## Phase 10 - History Viewing Screen
## Displays Training History, Match History, Progress, and Auto-save Status
## Phase 13 - Extended Features: Search/Filter, Export, Comparison, Dashboard

const _MatchTimeFormatter = preload("res://scripts/utils/MatchTimeFormatter.gd")

# UI References
@onready var title_label = $VBox/TitleLabel
@onready var tab_container = $VBox/TabContainer
@onready var back_button = $VBox/BackButton

# Phase 13: Filter Panel
var filter_panel: FilterPanel
var filter_button: Button
var filter_active: bool = false

# Phase 13: Export Dialog
var export_dialog: ExportDialog
var export_button: Button

# Phase 13: Comparison View
var comparison_view: ComparisonView
var comparison_button: Button
var selected_records: Array = []  # Currently selected records for comparison
var selection_mode: bool = false

# Training History Tab
@onready var training_list = $VBox/TabContainer/훈련기록/ScrollContainer/TrainingList
@onready var training_stats_label = $VBox/TabContainer/훈련기록/StatsPanel/StatsLabel

# Match History Tab
@onready var match_list = $VBox/TabContainer/경기기록/ScrollContainer/MatchList
@onready var match_stats_label = $VBox/TabContainer/경기기록/StatsPanel/StatsLabel

# Progress Tab
@onready var progress_bar = $VBox/TabContainer/진행상황/ProgressPanel/ProgressBar
@onready var progress_label = $VBox/TabContainer/진행상황/ProgressPanel/ProgressLabel
@onready var auto_save_status = $VBox/TabContainer/진행상황/AutoSavePanel/StatusLabel

# Analytics Tab (Phase 11)
@onready var training_trend_container = $VBox/TabContainer/분석/TrainingTrendPanel/TrainingTrendChart
@onready var match_performance_container = $VBox/TabContainer/분석/MatchPerformancePanel/MatchPerformanceChart
@onready var attribute_growth_container = $VBox/TabContainer/분석/AttributeGrowthPanel/AttributeGrowthChart
@onready var prediction_label = $VBox/TabContainer/분석/PredictionPanel/PredictionLabel

# Chart instances
var training_trend_chart: LineChart
var match_performance_chart: BarChart
var attribute_growth_chart: HexagonChart

# Phase 12: Animation support
var training_animator: ChartAnimator
var match_animator: ChartAnimator
var attribute_animator: ChartAnimator

# Phase 13: Filter data storage
var all_training_records: Array = []
var all_match_records: Array = []
var current_filter_criteria: FilterCriteria = null


func _ready():
	print("[HistoryScreen] Initializing Phase 10 History Screen...")

	# UI 초기화
	_setup_ui()

	# Phase 13: Setup filter panel, export dialog, and comparison view
	_setup_filter_panel()
	_setup_export_dialog()
	_setup_comparison_view()

	# 데이터 로드
	_load_training_history()
	_load_match_history()
	_load_progress_info()

	# Phase 11: Analytics 탭 로드
	_load_analytics()

	# 버튼 연결
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	print("[HistoryScreen] Ready!")


func _setup_ui():
	"""UI 초기 설정"""
	if title_label:
		title_label.text = "📊 기록 보기"
		title_label.add_theme_font_size_override("font_size", 32)


func _setup_filter_panel():
	"""Phase 13: Filter panel setup"""
	# Create filter button in title area
	if title_label and title_label.get_parent():
		var title_container = title_label.get_parent()

		# Check if title_container is HBoxContainer, if not, restructure
		if not title_container is HBoxContainer:
			# Create HBoxContainer for title + button
			var hbox = HBoxContainer.new()
			var title_parent = title_container.get_parent()
			var title_index = title_label.get_index()

			title_container.remove_child(title_label)
			title_parent.add_child(hbox)
			title_parent.move_child(hbox, title_index)
			hbox.add_child(title_label)

			title_container = hbox

		# Add spacer
		var spacer = Control.new()
		spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		title_container.add_child(spacer)

		# Create filter button
		filter_button = Button.new()
		filter_button.text = "🔍 필터"
		filter_button.custom_minimum_size = Vector2(100, 48)
		filter_button.pressed.connect(_on_filter_button_pressed)
		title_container.add_child(filter_button)

	# Create filter panel (initially hidden)
	filter_panel = FilterPanel.new()
	filter_panel.visible = false
	filter_panel.filter_changed.connect(_on_filter_changed)
	filter_panel.filter_reset.connect(_on_filter_reset)

	# Add to scene (above tab container)
	if tab_container and tab_container.get_parent():
		var vbox_parent = tab_container.get_parent()
		var tab_index = tab_container.get_index()
		vbox_parent.add_child(filter_panel)
		vbox_parent.move_child(filter_panel, tab_index)


func _setup_export_dialog():
	"""Phase 13: Export dialog setup"""
	# Create export button next to filter button
	if filter_button and filter_button.get_parent():
		var button_container = filter_button.get_parent()

		export_button = Button.new()
		export_button.text = "📤 내보내기"
		export_button.custom_minimum_size = Vector2(120, 48)
		export_button.pressed.connect(_on_export_button_pressed)
		button_container.add_child(export_button)

	# Create export dialog
	export_dialog = ExportDialog.new()
	export_dialog.export_requested.connect(_on_export_requested)
	add_child(export_dialog)


func _setup_comparison_view():
	"""Phase 13: Comparison view setup"""
	# Create comparison button next to export button
	if export_button and export_button.get_parent():
		var button_container = export_button.get_parent()

		comparison_button = Button.new()
		comparison_button.text = "🔀 비교"
		comparison_button.custom_minimum_size = Vector2(100, 48)
		comparison_button.pressed.connect(_on_comparison_button_pressed)
		button_container.add_child(comparison_button)

	# Create comparison view (initially hidden)
	comparison_view = ComparisonView.new()
	comparison_view.visible = false
	comparison_view.comparison_closed.connect(_on_comparison_closed)
	add_child(comparison_view)


func _load_training_history():
	"""훈련 기록 로드 및 표시"""
	if not TrainingManager:
		push_warning("[HistoryScreen] TrainingManager not found")
		return

	# Phase 13: Load ALL records for filtering
	all_training_records = TrainingManager.get_training_history(100)  # Load more records

	if all_training_records.size() == 0:
		_show_empty_message(training_list, "아직 훈련 기록이 없습니다")
		return

	# Apply filter if active
	var display_records = all_training_records
	if current_filter_criteria and filter_active:
		display_records = FilterService.filter_training_records(all_training_records, current_filter_criteria)

	# 기존 리스트 초기화
	for child in training_list.get_children():
		child.queue_free()

	# 훈련 기록 표시 (최대 20개)
	var max_display = min(20, display_records.size())
	for i in range(max_display):
		var item = _create_training_item(display_records[i])
		training_list.add_child(item)

	# 통계 표시
	_update_training_stats()

	print("[HistoryScreen] Loaded %d training records (showing %d)" % [all_training_records.size(), max_display])


func _create_training_item(record: Dictionary) -> Control:
	"""개별 훈련 기록 아이템 생성"""
	var container = PanelContainer.new()
	container.custom_minimum_size = Vector2(0, 80)

	# Store record data in metadata for selection
	container.set_meta("record_data", record)
	container.set_meta("record_type", "training")

	# 스타일
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.3, 0.4, 0.8)
	style.border_color = Color(0.4, 0.6, 0.8)
	style.set_border_width_all(2)
	style.set_corner_radius_all(8)
	container.add_theme_stylebox_override("panel", style)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 5)
	container.add_child(vbox)

	# 훈련 정보
	var title_label = Label.new()
	title_label.text = (
		"⚽ %s (Week %d-%d)" % [record.get("training_name", "Unknown"), record.get("year", 1), record.get("week", 1)]
	)
	title_label.add_theme_font_size_override("font_size", 18)
	vbox.add_child(title_label)

	# 능력치 변화
	var changes_text = "📈 변화: "
	var changes = record.get("attribute_changes", {})
	for attr in changes:
		changes_text += "%s +%d " % [attr, changes[attr]]

	var changes_label = Label.new()
	changes_label.text = changes_text
	changes_label.add_theme_color_override("font_color", Color(0.5, 1, 0.5))
	vbox.add_child(changes_label)

	# 컨디션 정보
	var condition_label = Label.new()
	condition_label.text = (
		"💪 컨디션: %.1f%% → %.1f%% (효과: %.0f%%)"
		% [
			record.get("condition_before", 100),
			record.get("condition_after", 90),
			record.get("effectiveness_modifier", 1.0) * 100
		]
	)
	condition_label.add_theme_font_size_override("font_size", 14)
	vbox.add_child(condition_label)

	# Phase 13: Add click handler for selection
	container.gui_input.connect(
		func(event):
			if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
				_on_record_selected(container)
	)

	return container


func _update_training_stats():
	"""훈련 통계 업데이트"""
	if not TrainingManager or not training_stats_label:
		return

	var stats = TrainingManager.get_training_stats()

	var stats_text = (
		"""📊 훈련 통계

총 훈련 횟수: %d회
평균 효과: %.1f%%
총 컨디션 소모: %.1f%%

훈련 타입별:
"""
		% [
			stats.get("total_sessions", 0),
			stats.get("average_effectiveness", 1.0) * 100,
			stats.get("total_condition_cost", 0)
		]
	)

	# 타입별 횟수
	var sessions_by_type = stats.get("sessions_by_type", {})
	for type in sessions_by_type:
		stats_text += "• %s: %d회\n" % [type, sessions_by_type[type]]

	training_stats_label.text = stats_text


func _load_match_history():
	"""경기 기록 로드 및 표시"""
	if not MatchManager:
		push_warning("[HistoryScreen] MatchManager not found")
		return

	# Phase 13: Load ALL records for filtering
	all_match_records = MatchManager.get_match_history(100)  # Load more records

	if all_match_records.size() == 0:
		_show_empty_message(match_list, "아직 경기 기록이 없습니다")
		return

	# Apply filter if active
	var display_records = all_match_records
	if current_filter_criteria and filter_active:
		display_records = FilterService.filter_match_records(all_match_records, current_filter_criteria)

	# 기존 리스트 초기화
	for child in match_list.get_children():
		child.queue_free()

	# 경기 기록 표시 (최대 20개)
	var max_display = min(20, display_records.size())
	for i in range(max_display):
		var item = _create_match_item(display_records[i])
		match_list.add_child(item)

	# 통계 표시
	_update_match_stats()

	print("[HistoryScreen] Loaded %d match records (showing %d)" % [all_match_records.size(), max_display])


func _create_match_item(record: Dictionary) -> Control:
	"""개별 경기 기록 아이템 생성"""
	var container = PanelContainer.new()
	container.custom_minimum_size = Vector2(0, 80)

	# Store record data in metadata for selection
	container.set_meta("record_data", record)
	container.set_meta("record_type", "match")

	# 결과에 따른 색상
	var result = record.get("result", "무승부")
	var bg_color = Color(0.3, 0.3, 0.3, 0.8)
	var border_color = Color(0.5, 0.5, 0.5)

	if result == "승리":
		bg_color = Color(0.2, 0.4, 0.2, 0.8)
		border_color = Color(0.4, 0.8, 0.4)
	elif result == "패배":
		bg_color = Color(0.4, 0.2, 0.2, 0.8)
		border_color = Color(0.8, 0.4, 0.4)

	var style = StyleBoxFlat.new()
	style.bg_color = bg_color
	style.border_color = border_color
	style.set_border_width_all(2)
	style.set_corner_radius_all(8)
	container.add_theme_stylebox_override("panel", style)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 5)
	container.add_child(vbox)

	# 경기 정보
	var title_label = Label.new()
	var result_icon = "✅" if result == "승리" else ("❌" if result == "패배" else "🟰")
	title_label.text = (
		"%s vs %s (Week %d-%d)"
		% [result_icon, record.get("opponent_name", "Unknown"), record.get("year", 1), record.get("week", 1)]
	)
	title_label.add_theme_font_size_override("font_size", 18)
	vbox.add_child(title_label)

        # 스코어
        var score_label = Label.new()
        var pen_suffix := _MatchTimeFormatter.format_penalty_shootout_suffix(record)
        score_label.text = "⚽ 스코어: %d - %d%s" % [
                record.get("goals_scored", 0),
                record.get("goals_conceded", 0),
                pen_suffix,
        ]
        score_label.add_theme_font_size_override("font_size", 16)
        vbox.add_child(score_label)

	# 내 선수 성과 (Phase F)
	var rating: float = float(record.get("player_rating", -1.0))
	var goals: int = int(record.get("player_goals", record.get("goals", 0)))
	var assists: int = int(record.get("player_assists", record.get("assists", 0)))

	if rating > 0.0 or goals > 0 or assists > 0:
		var perf_label := Label.new()
		var perf_text := ""

		# 평점 텍스트 + 아이콘
		if rating > 0.0:
			var rating_str := "%.1f" % rating
			if rating >= 8.5:
				perf_text += "★ 평점 %s" % rating_str
			else:
				perf_text += "평점 %s" % rating_str

		# 골/도움 텍스트
		if goals > 0 or assists > 0:
			if perf_text != "":
				perf_text += " | "
			perf_text += "%d골 %d도움" % [goals, assists]

		perf_label.text = perf_text

		# 색상 규칙
		var col := Color(0.9, 0.9, 0.9)
		if rating > 0.0:
			if rating >= 8.5:
				col = Color(1.0, 0.9, 0.3)  # 강조색 + ★
			elif rating >= 7.0:
				col = Color(0.8, 1.0, 0.8)  # 연한 강조색
			elif rating < 5.0:
				col = Color(0.7, 0.7, 0.7)  # 회색

		perf_label.add_theme_font_size_override("font_size", 14)
		perf_label.add_theme_color_override("font_color", col)
		vbox.add_child(perf_label)

	# 추가 정보
	var info_label = Label.new()
	info_label.text = (
		"📋 전술: %s | 상대 레이팅: %d" % [record.get("tactic_used", "Unknown"), record.get("opponent_rating", 50)]
	)
	info_label.add_theme_font_size_override("font_size", 14)
	vbox.add_child(info_label)

	# Phase 13: Add click handler for selection
	container.gui_input.connect(
		func(event):
			if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
				_on_record_selected(container)
	)

	return container


func _update_match_stats():
	"""경기 통계 업데이트"""
	if not MatchManager or not match_stats_label:
		return

	var stats = MatchManager.get_match_stats()

	var stats_text = (
		"""📊 경기 통계

총 경기 수: %d경기
전적: %d승 %d무 %d패
승률: %.1f%%

득실 기록:
• 득점: %d골 (평균 %.1f)
• 실점: %d골 (평균 %.1f)
• 득실차: %+d
"""
		% [
			stats.get("total_matches", 0),
			stats.get("wins", 0),
			stats.get("draws", 0),
			stats.get("losses", 0),
			stats.get("win_rate", 0),
			stats.get("goals_scored", 0),
			stats.get("average_goals_scored", 0),
			stats.get("goals_conceded", 0),
			stats.get("average_goals_conceded", 0),
			stats.get("goals_scored", 0) - stats.get("goals_conceded", 0)
		]
	)

	match_stats_label.text = stats_text


func _load_progress_info():
	"""진행 상황 로드"""
	if not SaveManager:
		push_warning("[HistoryScreen] SaveManager not found")
		return

	# 진행도 표시
	var progress = SaveManager.get_game_progress()

	if progress_bar:
		progress_bar.value = progress

	if progress_label:
		progress_label.text = "📈 게임 진행도: %.1f%%" % progress
		progress_label.add_theme_font_size_override("font_size", 24)

	# 자동저장 상태 표시
	var auto_save_config = SaveManager.get_auto_save_config()

	if auto_save_status:
		var status_text = (
			"""🔄 자동저장 상태

• 활성화: %s
• 주기: %d주마다
• 최근 자동저장: %s
"""
			% [
				"✅ ON" if auto_save_config.enabled else "❌ OFF",
				SaveManager.auto_save_frequency,
				"있음" if auto_save_config.has_auto_save else "없음"
			]
		)

		auto_save_status.text = status_text
		auto_save_status.add_theme_font_size_override("font_size", 16)

	print("[HistoryScreen] Progress: %.1f%%" % progress)


func _show_empty_message(parent: Control, message: String):
	"""빈 메시지 표시"""
	var label = Label.new()
	label.text = message
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.add_theme_font_size_override("font_size", 20)
	label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	parent.add_child(label)


func _on_back_pressed():
	"""뒤로가기 버튼 처리"""
	print("[HistoryScreen] Back button pressed")
	# Phase 12: Use transition system
	if ScreenTransition:
		ScreenTransition.change_scene("res://scenes/HomeImproved.tscn", "fade")
	else:
		get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


## Phase 13: Filter Callbacks


func _on_filter_button_pressed():
	"""필터 버튼 클릭 시 패널 토글"""
	if filter_panel:
		filter_panel.toggle_visibility()
		filter_active = filter_panel.visible


func _on_filter_changed(criteria: FilterCriteria):
	"""필터 적용 시"""
	current_filter_criteria = criteria
	filter_active = true

	# Reload data with filter
	_load_training_history()
	_load_match_history()

	print(
		(
			"[HistoryScreen] Filter applied: %d training, %d match records after filter"
			% [
				training_list.get_child_count() if training_list else 0,
				match_list.get_child_count() if match_list else 0
			]
		)
	)


func _on_filter_reset():
	"""필터 초기화 시"""
	current_filter_criteria = null
	filter_active = false

	# Reload data without filter
	_load_training_history()
	_load_match_history()

	print("[HistoryScreen] Filter reset")


## Phase 13: Export Callbacks


func _on_export_button_pressed():
	"""내보내기 버튼 클릭 시"""
	if export_dialog:
		export_dialog.show_dialog()


func _on_export_requested(export_type: String, format: String, file_path: String):
	"""내보내기 요청 시"""
	print("[HistoryScreen] Export requested: type=%s, format=%s, path=%s" % [export_type, format, file_path])

	var success = false

	match export_type:
		"match":
			if format == "csv":
				success = ExportService.export_match_records_csv(all_match_records, file_path)
			else:
				success = ExportService.export_match_records_json(all_match_records, file_path)

		"training":
			if format == "csv":
				success = ExportService.export_training_records_csv(all_training_records, file_path)
			else:
				success = ExportService.export_training_records_json(all_training_records, file_path)

		"all":
			if format == "json":
				success = ExportService.export_all_records_json(all_match_records, all_training_records, file_path)
			else:
				# CSV doesn't support combined export, do matches only
				success = ExportService.export_match_records_csv(all_match_records, file_path)

	if success:
		print("[HistoryScreen] Export successful: %s" % file_path)
	else:
		push_error("[HistoryScreen] Export failed")


## Phase 13: Comparison Callbacks


func _on_comparison_button_pressed():
	"""비교 버튼 클릭 시"""
	# Toggle selection mode
	selection_mode = not selection_mode

	if selection_mode:
		comparison_button.text = "취소"
		selected_records.clear()
		print("[HistoryScreen] Selection mode enabled - click 2 records to compare")
	else:
		comparison_button.text = "🔀 비교"
		selected_records.clear()
		print("[HistoryScreen] Selection mode disabled")


func _on_record_selected(container: Control):
	"""레코드 선택 시 (현재는 직접 구현 필요)"""
	if not selection_mode:
		return

	var record_data = container.get_meta("record_data")
	var record_type = container.get_meta("record_type")

	# Check if already selected
	var already_selected = false
	for i in range(selected_records.size()):
		if selected_records[i].container == container:
			already_selected = true
			selected_records.remove_at(i)
			break

	if not already_selected:
		selected_records.append({"container": container, "data": record_data, "type": record_type})

	# Visual feedback (highlight selected)
	_update_selection_visual()

	# If 2 records selected, show comparison
	if selected_records.size() == 2:
		var rec_a = selected_records[0]
		var rec_b = selected_records[1]

		# Check if same type
		if rec_a.type != rec_b.type:
			print("[HistoryScreen] Cannot compare different record types")
			selected_records.clear()
			_update_selection_visual()
			return

		# Show comparison view
		if rec_a.type == "match":
			comparison_view.set_match_comparison(rec_a.data, rec_b.data)
		else:
			comparison_view.set_training_comparison(rec_a.data, rec_b.data)

		comparison_view.visible = true

		# Reset selection
		selection_mode = false
		comparison_button.text = "🔀 비교"
		selected_records.clear()
		_update_selection_visual()


func _update_selection_visual():
	"""선택 상태 시각적 표시 업데이트"""
	# Update all training items
	if training_list:
		for child in training_list.get_children():
			var is_selected = false
			for sel in selected_records:
				if sel.container == child:
					is_selected = true
					break

			# Update visual (add border or highlight)
			var style: StyleBoxFlat = child.get_theme_stylebox("panel")
			if style:
				if is_selected:
					style.border_color = Color(1.0, 1.0, 0.0)
					style.set_border_width_all(4)
				else:
					style.border_color = Color(0.4, 0.6, 0.8)
					style.set_border_width_all(2)

	# Update all match items
	if match_list:
		for child in match_list.get_children():
			var is_selected = false
			for sel in selected_records:
				if sel.container == child:
					is_selected = true
					break

			# Update visual
			var style: StyleBoxFlat = child.get_theme_stylebox("panel")
			if style:
				if is_selected:
					style.border_color = Color(1.0, 1.0, 0.0)
					style.set_border_width_all(4)
				else:
					var result = child.get_meta("record_data").get("result", "무승부")
					if result == "승리":
						style.border_color = Color(0.4, 0.8, 0.4)
					elif result == "패배":
						style.border_color = Color(0.8, 0.4, 0.4)
					else:
						style.border_color = Color(0.5, 0.5, 0.5)
					style.set_border_width_all(2)


func _on_comparison_closed():
	"""비교 뷰 닫기"""
	if comparison_view:
		comparison_view.visible = false


## Phase 11: Analytics Functions


func _load_analytics():
	"""Analytics 탭 로드 및 차트 생성"""
	# Create charts
	_create_training_trend_chart()
	_create_match_performance_chart()
	_create_attribute_growth_chart()

	# Load analytics data
	_load_training_trends()
	_load_match_performance()
	_load_attribute_growth()
	_generate_predictions()


func _create_training_trend_chart():
	"""훈련 효과 트렌드 차트 생성"""
	if not training_trend_container:
		return

	training_trend_chart = LineChart.new()
	training_trend_chart.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	training_trend_chart.size_flags_vertical = Control.SIZE_EXPAND_FILL
	training_trend_container.add_child(training_trend_chart)

	# Phase 12: Add animator
	training_animator = ChartAnimator.new()
	training_animator.duration = 1.0
	training_trend_container.add_child(training_animator)


func _create_match_performance_chart():
	"""경기 성적 차트 생성"""
	if not match_performance_container:
		return

	match_performance_chart = BarChart.new()
	match_performance_chart.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	match_performance_chart.size_flags_vertical = Control.SIZE_EXPAND_FILL
	match_performance_container.add_child(match_performance_chart)

	# Phase 12: Add animator
	match_animator = ChartAnimator.new()
	match_animator.duration = 0.8
	match_performance_container.add_child(match_animator)


func _create_attribute_growth_chart():
	"""능력치 성장 차트 생성"""
	if not attribute_growth_container:
		return

	attribute_growth_chart = HexagonChart.new()
	attribute_growth_chart.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	attribute_growth_chart.size_flags_vertical = Control.SIZE_EXPAND_FILL
	attribute_growth_chart.chart_size = 100.0
	attribute_growth_container.add_child(attribute_growth_chart)

	# Phase 12: Add animator
	attribute_animator = ChartAnimator.new()
	attribute_animator.duration = 0.6
	attribute_growth_container.add_child(attribute_animator)


func _load_training_trends():
	"""훈련 효과 트렌드 데이터 로드"""
	if not TrainingManager or not training_trend_chart:
		return

	var history = TrainingManager.get_training_history(20)
	if history.size() < 2:
		print("[HistoryScreen] Not enough training history for trends")
		return

	# Prepare data: Effectiveness over time
	var effectiveness_data = PackedVector2Array()
	var weeks = []

	var min_week = 1e9
	var max_week = -1e9
	var min_eff = 1e9
	var max_eff = -1e9

	# Collect data
	for record in history:
		var week = record.get("week", 1)
		var year = record.get("year", 1)
		var total_week = (year - 1) * 52 + week
		var effectiveness = record.get("effectiveness_modifier", 1.0)

		min_week = min(min_week, total_week)
		max_week = max(max_week, total_week)
		min_eff = min(min_eff, effectiveness)
		max_eff = max(max_eff, effectiveness)

		weeks.append(total_week)
		effectiveness_data.append(Vector2(total_week, effectiveness))

	# Normalize data to 0-1
	var span_week = max(1.0, max_week - min_week)
	var span_eff = max(0.1, max_eff - min_eff)

	var normalized_points = PackedVector2Array()
	for point in effectiveness_data:
		var x = (point.x - min_week) / span_week
		var y = (point.y - min_eff) / span_eff
		normalized_points.append(Vector2(x, y))

	# Set series data
	var series = [{"name": "훈련 효과", "points": normalized_points, "color": Color(0.4, 0.8, 1.0)}]

	training_trend_chart.set_series(series)

	# Set labels
	var x_labels = []
	var step = max(1, history.size() // 5)
	for i in range(0, history.size(), step):
		var record = history[i]
		x_labels.append("W%d" % record.get("week", 1))

	var y_labels = []
	for i in range(6):
		var value = min_eff + (span_eff * float(i) / 5.0)
		y_labels.append("%.1f" % value)

	training_trend_chart.set_x_labels(x_labels)
	training_trend_chart.set_y_labels(y_labels)

	# Phase 12: Animate chart
	if training_animator:
		training_animator.animate_chart(training_trend_chart, "draw")

	print("[HistoryScreen] Training trend chart loaded with %d points" % normalized_points.size())


func _load_match_performance():
	"""경기 성적 데이터 로드"""
	if not MatchManager or not match_performance_chart:
		return

	var stats = MatchManager.get_match_stats()

	# Prepare bar chart data: Wins, Draws, Losses
	var data = [
		{"label": "승리", "value": stats.get("wins", 0), "color": Color(0.5, 0.9, 0.5)},
		{"label": "무승부", "value": stats.get("draws", 0), "color": Color(0.7, 0.7, 0.7)},
		{"label": "패배", "value": stats.get("losses", 0), "color": Color(0.9, 0.5, 0.5)}
	]

	match_performance_chart.set_data(data)

	# Phase 12: Animate chart
	if match_animator:
		match_animator.animate_chart(match_performance_chart, "draw")

	print("[HistoryScreen] Match performance chart loaded")


func _load_attribute_growth():
	"""능력치 성장 데이터 로드"""
	if not GlobalCharacterData or not attribute_growth_chart:
		return

	# Get current attributes from GlobalCharacterData
	var attributes = GlobalCharacterData.character_data.get("attributes", {})

	# Prepare hexagon data (normalized to 0-1)
	var hexagon_stats = {
		"PACE": attributes.get("Pace", 50) / 100.0,
		"POWER": attributes.get("Strength", 50) / 100.0,
		"TECHNICAL": attributes.get("Dribbling", 50) / 100.0,
		"SHOOTING": attributes.get("Finishing", 50) / 100.0,
		"PASSING": attributes.get("Passing", 50) / 100.0,
		"DEFENDING": attributes.get("Tackling", 50) / 100.0
	}

	attribute_growth_chart.set_stats(hexagon_stats, true)

	# Phase 12: Animate chart (fade in for hexagon)
	if attribute_animator:
		attribute_animator.animate_chart(attribute_growth_chart, "fade_in")

	print("[HistoryScreen] Attribute growth chart loaded")


func _generate_predictions():
	"""예측 분석 생성"""
	if not prediction_label:
		return

	var prediction_text = "🔮 예측 분석\n\n"

	# Training prediction
	if TrainingManager:
		var stats = TrainingManager.get_training_stats()
		var total_sessions = stats.get("total_sessions", 0)
		var avg_effectiveness = stats.get("average_effectiveness", 1.0)

		if total_sessions > 5:
			var predicted_growth = avg_effectiveness * 10  # Next 10 trainings
			prediction_text += "📊 훈련 예측:\n"
			prediction_text += "• 평균 효과: %.0f%%\n" % (avg_effectiveness * 100)
			prediction_text += "• 향후 10회 예상 성장: +%.0f\n\n" % predicted_growth
		else:
			prediction_text += "📊 훈련 예측:\n"
			prediction_text += "• 데이터 부족 (5회 이상 필요)\n\n"

	# Match prediction
	if MatchManager:
		var stats = MatchManager.get_match_stats()
		var total_matches = stats.get("total_matches", 0)
		var win_rate = stats.get("win_rate", 0)

		if total_matches > 3:
			var predicted_wins = round(win_rate / 100.0 * 10)  # Next 10 matches
			prediction_text += "⚽ 경기 예측:\n"
			prediction_text += "• 현재 승률: %.1f%%\n" % win_rate
			prediction_text += "• 향후 10경기 예상 승수: %d승\n\n" % predicted_wins
		else:
			prediction_text += "⚽ 경기 예측:\n"
			prediction_text += "• 데이터 부족 (3경기 이상 필요)\n\n"

	# Progress prediction
	if SaveManager:
		var progress = SaveManager.get_game_progress()
		var remaining = 100.0 - progress

		prediction_text += "📈 진행도 예측:\n"
		prediction_text += "• 현재 진행: %.1f%%\n" % progress
		prediction_text += "• 남은 진행: %.1f%%\n" % remaining

		if progress > 5.0 and DateManager:
			var current_day = DateManager.current_day
			var estimated_total_days = current_day / (progress / 100.0)
			var remaining_days = estimated_total_days - current_day
			prediction_text += "• 예상 소요 일수: %d일\n" % remaining_days

	prediction_label.text = prediction_text
	print("[HistoryScreen] Predictions generated")
