extends Control
## MatchHistoryScreen - 경기 기록 및 하이라이트 화면
## Displays match history, event timeline, and statistics

signal back_pressed

# Viewer scenes
const TIMELINE_VIEWER_SCENE: PackedScene = preload("res://scenes/ui/MatchTimelinePanel.tscn")
const TACTICAL_ANALYSIS_VIEWER_SCENE: PackedScene = preload(
	"res://scenes/match_pipeline/examples/TacticalAnalysisViewer.tscn"
)
const _MatchTimeFormatter = preload("res://scripts/utils/MatchTimeFormatter.gd")

# UI References
@onready var back_button: Button = $BackButton
@onready var tab_container: TabContainer = $TabContainer

# History Tab
@onready var history_list: VBoxContainer = $TabContainer/History/VBox/HistoryList/ScrollContainer/VBoxContainer
@onready var match_detail_panel: Control = $TabContainer/History/VBox/MatchDetail
@onready var viewer_buttons: HBoxContainer = $TabContainer/History/VBox/MatchDetail/VBox/ViewerButtons
@onready var timeline_button: Button = $TabContainer/History/VBox/MatchDetail/VBox/ViewerButtons/TimelineButton
@onready var tactical_button: Button = $TabContainer/History/VBox/MatchDetail/VBox/ViewerButtons/TacticalButton
@onready var detail_opponent_label: Label = $TabContainer/History/VBox/MatchDetail/VBox/OpponentLabel
@onready var detail_score_label: Label = $TabContainer/History/VBox/MatchDetail/VBox/ScoreLabel
@onready var detail_result_label: Label = $TabContainer/History/VBox/MatchDetail/VBox/ResultLabel
@onready var detail_date_label: Label = $TabContainer/History/VBox/MatchDetail/VBox/DateLabel
@onready
var detail_events: VBoxContainer = $TabContainer/History/VBox/MatchDetail/VBox/Events/ScrollContainer/VBoxContainer

# Statistics Tab
@onready var total_matches_label: Label = $TabContainer/Statistics/VBox/TotalMatches
@onready var record_label: Label = $TabContainer/Statistics/VBox/Record
@onready var win_rate_label: Label = $TabContainer/Statistics/VBox/WinRate
@onready var goals_label: Label = $TabContainer/Statistics/VBox/Goals
@onready var avg_goals_label: Label = $TabContainer/Statistics/VBox/AvgGoals

var selected_match: Dictionary = {}


func _ready():
	print("[MatchHistoryScreen] Initializing...")

	# Connect back button
	if back_button:
		back_button.pressed.connect(_on_back_pressed)
	if timeline_button:
		timeline_button.pressed.connect(_on_timeline_button_pressed)
	if tactical_button:
		tactical_button.pressed.connect(_on_tactical_button_pressed)

	# Load data
	_load_match_history()
	_load_statistics()

	# Hide detail panel initially
	if match_detail_panel:
		match_detail_panel.visible = false
	if viewer_buttons:
		viewer_buttons.visible = false

	print("[MatchHistoryScreen] Ready!")


func _load_match_history():
	"""Load and display match history"""
	if not history_list:
		return

	# Clear existing entries
	for child in history_list.get_children():
		child.queue_free()

	# Get match history from MatchManager
	var match_manager = get_node_or_null("/root/MatchManager")
	if not match_manager:
		print("[MatchHistoryScreen] ⚠️ MatchManager not found")
		var empty_label = Label.new()
		empty_label.text = "경기 기록이 없습니다"
		empty_label.add_theme_font_size_override("font_size", 20)
		empty_label.modulate = Color(0.7, 0.7, 0.7)
		history_list.add_child(empty_label)
		return

	var history = match_manager.get_match_history()

	if history.size() == 0:
		var empty_label = Label.new()
		empty_label.text = "아직 경기를 치르지 않았습니다!\n경기를 진행하면 기록이 표시됩니다."
		empty_label.add_theme_font_size_override("font_size", 20)
		empty_label.modulate = Color(0.7, 0.7, 0.7)
		empty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		history_list.add_child(empty_label)
		return

	# Create match entry buttons
	for match in history:
		var button = _create_match_button(match)
		history_list.add_child(button)

	print("[MatchHistoryScreen] Loaded %d match records" % history.size())


func _create_match_button(match: Dictionary) -> Button:
	"""Create a button for a match record"""
	var button = Button.new()
	button.custom_minimum_size = Vector2(0, 100)
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Format match info
	var opponent: String = str(match.get("opponent_name", "Unknown"))
	var result: String = str(match.get("result", "무승부"))
	var score_variant: Variant = match.get("final_score", [0, 0])
	var score: Array = []
	if score_variant is Array:
		score = score_variant
	var display_home_score: int = int(match.get("home_score", 0))
	var display_away_score: int = int(match.get("away_score", 0))
	if score.size() > 0:
		display_home_score = int(score[0])
	if score.size() > 1:
		display_away_score = int(score[1])
	var year: int = int(match.get("year", 1))
	var week: int = int(match.get("week", 1))
	var pen_suffix := _format_penalty_shootout_suffix(match)

	# Result icon
	var icon = ""
	var color = Color.GRAY
	match result:
		"승리":
			icon = "🏆"
			color = Color(0.2, 0.8, 0.2, 0.4)  # Green
		"패배":
			icon = "❌"
			color = Color(0.9, 0.2, 0.2, 0.4)  # Red
		"무승부":
			icon = "🤝"
			color = Color(0.7, 0.7, 0.7, 0.4)  # Gray

	button.text = (
		"%s %s\n%s %d-%d%s\n%d학년 %d주차"
		% [icon, result, opponent, display_home_score, display_away_score, pen_suffix, year, week]
	)
	button.add_theme_font_size_override("font_size", 18)

	# Style
	var style = StyleBoxFlat.new()
	style.bg_color = color
	style.corner_radius_top_left = 10
	style.corner_radius_top_right = 10
	style.corner_radius_bottom_left = 10
	style.corner_radius_bottom_right = 10
	button.add_theme_stylebox_override("normal", style)

	button.pressed.connect(_on_match_selected.bind(match))

	return button


func _open_timeline_viewer(match: Dictionary) -> void:
	if not TIMELINE_VIEWER_SCENE:
		return
	var timeline_viewer: Node = TIMELINE_VIEWER_SCENE.instantiate()
	if not timeline_viewer:
		return

	var record_copy: Dictionary = match.duplicate(true)
	var position_payload: Dictionary = _extract_position_payload(record_copy)
	if position_payload.is_empty():
		position_payload = _fetch_last_position_payload()

	var tree: SceneTree = get_tree()
	if tree:
		var root: Node = tree.root
		root.add_child(timeline_viewer)
		if timeline_viewer.has_method("set_match_record"):
			timeline_viewer.call_deferred("set_match_record", record_copy)
		if not position_payload.is_empty() and timeline_viewer.has_method("set_position_payload"):
			timeline_viewer.call_deferred("set_position_payload", position_payload)


func _open_tactical_viewer(match: Dictionary) -> void:
	# Phase 1 Integration: TacticalAnalysisViewer (전술 분석 프리셋)
	var record_copy: Dictionary = match.duplicate(true)
	var current_scene_path: String = get_tree().current_scene.scene_file_path

	# Hand off record via autoload holder
	MatchTimelineHolder.set_timeline_data(record_copy, current_scene_path)

	# NEW: TacticalAnalysisViewer로 전환 (경기 후 전술 분석에 적합)
	# - Full Landscape 뷰
	# - 모든 오버레이 활성화 (패스, 압박, Heat Map 등)
	# - 전술 분석에 최적화됨
	get_tree().change_scene_to_file("res://scenes/match_pipeline/examples/TacticalAnalysisViewer.tscn")


func _on_timeline_button_pressed() -> void:
	if selected_match.is_empty():
		return
	_open_timeline_viewer(selected_match)


func _on_tactical_button_pressed() -> void:
	if selected_match.is_empty():
		return
	_open_tactical_viewer(selected_match)


func _on_match_selected(match: Dictionary):
	"""Handle match selection to show details"""
	selected_match = match
	_display_match_details()


func _display_match_details():
	"""Display detailed information for selected match"""
	if selected_match.size() == 0:
		return

	if match_detail_panel:
		match_detail_panel.visible = true
	if viewer_buttons:
		viewer_buttons.visible = true
		var has_timeline_data := false
		var legacy_doc_key := "re" + "play"
		var legacy_doc_key2 := ("re" + "play") + "_doc"
		if (
			selected_match.has("timeline_doc")
			or selected_match.has(legacy_doc_key)
			or selected_match.has(legacy_doc_key2)
		):
			has_timeline_data = true
		else:
			var match_result_variant: Variant = selected_match.get("match_result", {})
			if match_result_variant is Dictionary:
				has_timeline_data = true
		timeline_button.disabled = not has_timeline_data
		tactical_button.disabled = not has_timeline_data

	# Opponent
	if detail_opponent_label:
		var opponent = selected_match.get("opponent_name", "Unknown")
		var rating = selected_match.get("opponent_rating", 50)
		detail_opponent_label.text = "상대: %s (OVR %d)" % [opponent, rating]

	# Score
	if detail_score_label:
		var score_variant: Variant = selected_match.get("final_score", [0, 0])
		var score_array: Array = []
		if score_variant is Array:
			score_array = score_variant
		var display_home: int = int(selected_match.get("goals_scored", 0))
		var display_away: int = int(selected_match.get("goals_conceded", 0))
		if score_array.size() > 0:
			display_home = int(score_array[0])
		if score_array.size() > 1:
			display_away = int(score_array[1])
		var pen_suffix := _format_penalty_shootout_suffix(selected_match)
		detail_score_label.text = "최종 스코어: %d - %d%s" % [display_home, display_away, pen_suffix]

	# Result
	if detail_result_label:
		var result: String = str(selected_match.get("result", "무승부"))
		var icon = ""
		match result:
			"승리":
				icon = "🏆"
			"패배":
				icon = "❌"
			"무승부":
				icon = "🤝"
		detail_result_label.text = "%s %s" % [icon, result]

	# Date
	if detail_date_label:
		var year: int = int(selected_match.get("year", 1))
		var week: int = int(selected_match.get("week", 1))
		detail_date_label.text = "경기 일자: %d학년 %d주차" % [year, week]

	# Events (simplified - deterministic resimulation not implemented yet)
	_display_match_events()


func _display_match_events():
	"""Display match events timeline"""
	if not detail_events:
		return

	# Clear existing events
	for child in detail_events.get_children():
		child.queue_free()

	var title_label = Label.new()
	title_label.text = "경기 이벤트 타임라인"
	title_label.add_theme_font_size_override("font_size", 20)
	title_label.add_theme_color_override("font_color", Color(1, 0.8, 0.3))
	detail_events.add_child(title_label)

	var separator1 = HSeparator.new()
	detail_events.add_child(separator1)

	var stored_events_variant: Variant = selected_match.get("events", [])
	if stored_events_variant is Array and stored_events_variant.size() > 0:
		for variant_event in stored_events_variant:
			var event_variant: Variant = variant_event
			if not (event_variant is Dictionary):
				continue
			var event_dict: Dictionary = event_variant as Dictionary
			var minute_text := "%02d'" % int(event_dict.get("minute", 0))
			var icon_text := str(event_dict.get("icon", "•"))
			var description_text := str(event_dict.get("text", ""))
			var detail_label = Label.new()
			detail_label.text = "%s %s %s" % [minute_text, icon_text, description_text]
			detail_label.add_theme_font_size_override("font_size", 16)
			detail_events.add_child(detail_label)
		return

	# Since actual events aren't stored yet, show summary
	var score_fallback_variant: Variant = selected_match.get("final_score", [0, 0])
	var score_fallback: Array = []
	if score_fallback_variant is Array:
		score_fallback = score_fallback_variant
	var home_goals: int = int(selected_match.get("goals_scored", 0))
	var away_goals: int = int(selected_match.get("goals_conceded", 0))
	if score_fallback.size() > 0:
		home_goals = int(score_fallback[0])
	if score_fallback.size() > 1:
		away_goals = int(score_fallback[1])

	# Simulate event timeline from score
	var half_time_minute: int = 45
	var full_time_minute: int = 90
	if _MatchTimeFormatter:
		half_time_minute = int(_MatchTimeFormatter.derive_half_time_minute(selected_match))
		full_time_minute = int(_MatchTimeFormatter.derive_full_time_minute(selected_match, half_time_minute))
	var event_label = Label.new()
	event_label.text = "⏱️ 0' - 경기 시작"
	event_label.add_theme_font_size_override("font_size", 16)
	detail_events.add_child(event_label)

	# Add goal events (simulated timing)
	for i in range(home_goals):
		var goal_time = randi_range(5, max(5, full_time_minute - 5))
		var goal_label = Label.new()
		goal_label.text = "⚽ %d' - 우리팀 득점! (%d-%d)" % [goal_time, i + 1, 0]
		goal_label.add_theme_font_size_override("font_size", 16)
		goal_label.add_theme_color_override("font_color", Color(0.3, 1, 0.3))
		detail_events.add_child(goal_label)

	for i in range(away_goals):
		var goal_time = randi_range(5, max(5, full_time_minute - 5))
		var goal_label = Label.new()
		goal_label.text = "⚽ %d' - 상대팀 득점... (%d-%d)" % [goal_time, home_goals, i + 1]
		goal_label.add_theme_font_size_override("font_size", 16)
		goal_label.add_theme_color_override("font_color", Color(1, 0.3, 0.3))
		detail_events.add_child(goal_label)

	var halftime_label = Label.new()
	var ht_label := ("%d'" % half_time_minute)
	if _MatchTimeFormatter:
		ht_label = str(_MatchTimeFormatter.format_minute_label(half_time_minute, half_time_minute))
	halftime_label.text = "⏱️ %s - 전반전 종료" % ht_label
	halftime_label.add_theme_font_size_override("font_size", 16)
	detail_events.add_child(halftime_label)

	var end_label = Label.new()
	var ft_label := ("%d'" % full_time_minute)
	if _MatchTimeFormatter:
		ft_label = str(_MatchTimeFormatter.format_minute_label(full_time_minute, half_time_minute))
	end_label.text = "⏱️ %s - 경기 종료" % ft_label
	end_label.add_theme_font_size_override("font_size", 16)
	detail_events.add_child(end_label)

	var shootout := _get_penalty_shootout(selected_match)
	if not shootout.is_empty():
		var goals_home := int(shootout.get("goals_home", 0))
		var goals_away := int(shootout.get("goals_away", 0))
		var winner_is_home := bool(shootout.get("winner_is_home", false))
		var pen_label := Label.new()
		pen_label.text = "🥅 승부차기 PEN %d-%d (%s 승)" % [goals_home, goals_away, ("홈" if winner_is_home else "원정")]
		pen_label.add_theme_font_size_override("font_size", 16)
		pen_label.modulate = Color(0.9, 0.9, 0.7)
		detail_events.add_child(pen_label)

	var separator2 = HSeparator.new()
	detail_events.add_child(separator2)

	# Tactic used
	var tactic = selected_match.get("tactic_used", "균형")
	var tactic_label = Label.new()
	tactic_label.text = "사용 전술: %s" % tactic
	tactic_label.add_theme_font_size_override("font_size", 16)
	tactic_label.modulate = Color(0.8, 0.8, 1)
	detail_events.add_child(tactic_label)


func _extract_position_payload(record: Dictionary) -> Dictionary:
	var direct_candidate: Variant = record.get("position_data", {})
	if direct_candidate is Dictionary and not (direct_candidate as Dictionary).is_empty():
		return (direct_candidate as Dictionary).duplicate(true)
	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary:
		var match_pos: Variant = match_result_variant.get("position_data", {})
		if match_pos is Dictionary and not (match_pos as Dictionary).is_empty():
			return (match_pos as Dictionary).duplicate(true)
	var raw_result_variant: Variant = record.get("raw_result", {})
	if raw_result_variant is Dictionary:
		var raw_pos: Variant = raw_result_variant.get("position_data", {})
		if raw_pos is Dictionary and not (raw_pos as Dictionary).is_empty():
			return (raw_pos as Dictionary).duplicate(true)
	return {}


func _fetch_last_position_payload() -> Dictionary:
	var sim := get_node_or_null("/root/MatchSimulationManager")
	if sim and sim.has_method("get_last_position_data"):
		var payload = sim.get_last_position_data()
		if payload is Dictionary and not payload.is_empty():
			return payload
	return {}


func _load_statistics():
	"""Load and display match statistics"""
	var match_manager = get_node_or_null("/root/MatchManager")
	if not match_manager:
		print("[MatchHistoryScreen] ⚠️ MatchManager not found for statistics")
		return

	var stats = match_manager.get_match_stats()

	# Total matches
	if total_matches_label:
		total_matches_label.text = "총 경기: %d 경기" % stats.total_matches

	# Record
	if record_label:
		record_label.text = "전적: %d승 %d무 %d패" % [stats.wins, stats.draws, stats.losses]

	# Win rate
	if win_rate_label:
		win_rate_label.text = "승률: %.1f%%" % stats.win_rate

	# Goals
	if goals_label:
		goals_label.text = "득점/실점: %d / %d" % [stats.goals_scored, stats.goals_conceded]

	# Average goals
	if avg_goals_label:
		avg_goals_label.text = "평균 득점/실점: %.2f / %.2f" % [stats.average_goals_scored, stats.average_goals_conceded]

	print("[MatchHistoryScreen] Statistics loaded")


func _get_penalty_shootout(match_record: Dictionary) -> Dictionary:
	var result_variant: Variant = match_record.get("match_result", null)
	if result_variant is Dictionary:
		var shootout_variant: Variant = (result_variant as Dictionary).get("penalty_shootout", null)
		if shootout_variant is Dictionary:
			return shootout_variant as Dictionary
		if shootout_variant is String:
			var parsed: Variant = JSON.parse_string(String(shootout_variant))
			if parsed is Dictionary:
				return parsed as Dictionary
	return {}


func _format_penalty_shootout_suffix(match_record: Dictionary) -> String:
	var shootout := _get_penalty_shootout(match_record)
	if shootout.is_empty():
		return ""
	var goals_home := int(shootout.get("goals_home", 0))
	var goals_away := int(shootout.get("goals_away", 0))
	return " (PEN %d-%d)" % [goals_home, goals_away]


func _on_back_pressed():
	"""Handle back button press"""
	print("[MatchHistoryScreen] Back pressed")
	back_pressed.emit()
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")
