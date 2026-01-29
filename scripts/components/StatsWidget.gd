extends DashboardWidget
class_name StatsWidget
## Stats display widget for dashboard
## Phase 13: Extended Features - Dashboard System

var stats_label: Label


func _init(widget_config: WidgetConfig = null):
	super._init(widget_config)


func _ready():
	super._ready()
	_populate_content()


func _populate_content():
	"""Create and populate stats display"""
	if not config or not content_container:
		return

	# Clear existing content
	for child in content_container.get_children():
		child.queue_free()

	# Create stats label
	stats_label = Label.new()
	stats_label.add_theme_font_size_override("font_size", 14)
	stats_label.autowrap_mode = TextServer.AUTOWRAP_WORD
	stats_label.vertical_alignment = VERTICAL_ALIGNMENT_TOP
	content_container.add_child(stats_label)

	# Load data
	_load_stats_data()


func _load_stats_data():
	"""Load stats based on config.stats_type"""
	if not config or not stats_label:
		return

	match config.stats_type:
		"training":
			_load_training_stats()

		"match":
			_load_match_stats()

		"progress":
			_load_progress_stats()

		_:
			stats_label.text = "알 수 없는 통계 유형"


func _load_training_stats():
	"""Load training statistics"""
	if not TrainingManager:
		stats_label.text = "훈련 데이터를 불러올 수 없습니다"
		return

	var stats = TrainingManager.get_training_stats()

	if config.display_mode == "compact":
		stats_label.text = (
			"""📊 훈련 요약
총 %d회 | 평균 효과 %.0f%%"""
			% [stats.get("total_sessions", 0), stats.get("average_effectiveness", 1.0) * 100]
		)
	else:
		# Detailed mode
		var text = "📊 훈련 통계\n\n"
		text += "총 훈련 횟수: %d회\n" % stats.get("total_sessions", 0)
		text += "평균 효과: %.1f%%\n" % (stats.get("average_effectiveness", 1.0) * 100)
		text += "총 컨디션 소모: %.1f%%\n\n" % stats.get("total_condition_cost", 0)

		var sessions_by_type = stats.get("sessions_by_type", {})
		if sessions_by_type.size() > 0:
			text += "타입별 횟수:\n"
			for type_name in sessions_by_type:
				text += "• %s: %d회\n" % [type_name, sessions_by_type[type_name]]

		stats_label.text = text


func _load_match_stats():
	"""Load match statistics"""
	if not MatchManager:
		stats_label.text = "경기 데이터를 불러올 수 없습니다"
		return

	var stats = MatchManager.get_match_stats()

	if config.display_mode == "compact":
		stats_label.text = (
			"""⚽ 경기 요약
%d전 %d승 %d무 %d패
승률 %.1f%%"""
			% [
				stats.get("total_matches", 0),
				stats.get("wins", 0),
				stats.get("draws", 0),
				stats.get("losses", 0),
				stats.get("win_rate", 0)
			]
		)
	else:
		# Detailed mode
		var text = "⚽ 경기 통계\n\n"
		text += "총 경기 수: %d경기\n" % stats.get("total_matches", 0)
		text += "전적: %d승 %d무 %d패\n" % [stats.get("wins", 0), stats.get("draws", 0), stats.get("losses", 0)]
		text += "승률: %.1f%%\n\n" % stats.get("win_rate", 0)

		text += "득실 기록:\n"
		text += "• 득점: %d골 (평균 %.1f)\n" % [stats.get("goals_scored", 0), stats.get("average_goals_scored", 0)]
		text += "• 실점: %d골 (평균 %.1f)\n" % [stats.get("goals_conceded", 0), stats.get("average_goals_conceded", 0)]
		text += "• 득실차: %+d" % (stats.get("goals_scored", 0) - stats.get("goals_conceded", 0))

		stats_label.text = text


func _load_progress_stats():
	"""Load game progress statistics"""
	if not SaveManager:
		stats_label.text = "진행 데이터를 불러올 수 없습니다"
		return

	var progress = SaveManager.get_game_progress()

	if config.display_mode == "compact":
		stats_label.text = (
			"""📈 진행 상황
%.1f%% 완료"""
			% progress
		)
	else:
		# Detailed mode
		var text = "📈 게임 진행 상황\n\n"
		text += "현재 진행도: %.1f%%\n" % progress
		text += "남은 진행: %.1f%%\n\n" % (100.0 - progress)

		# Auto-save info
		var auto_save_config = SaveManager.get_auto_save_config()
		text += "자동저장:\n"
		text += "• 활성화: %s\n" % ("ON" if auto_save_config.enabled else "OFF")
		text += "• 주기: %d주마다\n" % SaveManager.auto_save_frequency
		text += "• 최근 자동저장: %s" % ("있음" if auto_save_config.has_auto_save else "없음")

		stats_label.text = text


func refresh_data():
	"""Refresh stats data"""
	_load_stats_data()

	# Fade in animation
	if stats_label:
		stats_label.modulate.a = 0.5
		var tween = create_tween()
		tween.tween_property(stats_label, "modulate:a", 1.0, 0.3)
