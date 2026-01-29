extends DashboardWidget
class_name ChartWidget
## Chart widget for dashboard
## Phase 13: Extended Features - Dashboard System

var chart_instance: Control  # LineChart, BarChart, or HexagonChart
var chart_animator: ChartAnimator


func _init(widget_config: WidgetConfig = null):
	super._init(widget_config)


func _ready():
	super._ready()
	_populate_content()


func _populate_content():
	"""Create and populate chart based on config"""
	if not config or not content_container:
		return

	# Clear existing content
	for child in content_container.get_children():
		child.queue_free()

	# Create chart based on type
	match config.chart_type:
		"line":
			chart_instance = LineChart.new()
			_load_line_chart_data()

		"bar":
			chart_instance = BarChart.new()
			_load_bar_chart_data()

		"hexagon":
			chart_instance = HexagonChart.new()
			_load_hexagon_chart_data()

		_:
			push_warning("[ChartWidget] Unknown chart type: %s" % config.chart_type)
			return

	if chart_instance:
		chart_instance.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		chart_instance.size_flags_vertical = Control.SIZE_EXPAND_FILL
		content_container.add_child(chart_instance)

		# Add animator (Phase 12)
		chart_animator = ChartAnimator.new()
		chart_animator.duration = 0.8
		content_container.add_child(chart_animator)


func _load_line_chart_data():
	"""Load data for line chart based on data_source"""
	if not chart_instance or not config:
		return

	var line_chart: LineChart = chart_instance as LineChart

	match config.data_source:
		"training_trends":
			_load_training_trends(line_chart)

		"match_performance_trend":
			_load_match_performance_trend(line_chart)

		_:
			# Default: empty chart
			line_chart.set_series([])


func _load_bar_chart_data():
	"""Load data for bar chart based on data_source"""
	if not chart_instance or not config:
		return

	var bar_chart: BarChart = chart_instance as BarChart

	match config.data_source:
		"match_performance":
			_load_match_performance_bars(bar_chart)

		"training_by_type":
			_load_training_by_type(bar_chart)

		_:
			# Default: empty chart
			bar_chart.set_data([])


func _load_hexagon_chart_data():
	"""Load data for hexagon chart based on data_source"""
	if not chart_instance or not config:
		return

	var hexagon_chart: HexagonChart = chart_instance as HexagonChart

	match config.data_source:
		"attribute_growth":
			_load_attribute_growth(hexagon_chart)

		_:
			# Default: empty chart
			hexagon_chart.set_stats({}, false)


func _load_training_trends(line_chart: LineChart):
	"""Load training effectiveness trends"""
	if not TrainingManager:
		return

	var history = TrainingManager.get_training_history(20)
	if history.size() < 2:
		return

	var effectiveness_data = PackedVector2Array()
	var min_week = 1e9
	var max_week = -1e9
	var min_eff = 1e9
	var max_eff = -1e9

	for record in history:
		var week = record.get("week", 1)
		var year = record.get("year", 1)
		var total_week = (year - 1) * 52 + week
		var effectiveness = record.get("effectiveness_modifier", 1.0)

		min_week = min(min_week, total_week)
		max_week = max(max_week, total_week)
		min_eff = min(min_eff, effectiveness)
		max_eff = max(max_eff, effectiveness)

		effectiveness_data.append(Vector2(total_week, effectiveness))

	# Normalize
	var span_week = max(1.0, max_week - min_week)
	var span_eff = max(0.1, max_eff - min_eff)

	var normalized_points = PackedVector2Array()
	for point in effectiveness_data:
		var x = (point.x - min_week) / span_week
		var y = (point.y - min_eff) / span_eff
		normalized_points.append(Vector2(x, y))

	line_chart.set_series([{"name": "훈련 효과", "points": normalized_points, "color": Color(0.4, 0.8, 1.0)}])


func _load_match_performance_trend(line_chart: LineChart):
	"""Load match performance trend (win rate over time)"""
	if not MatchManager:
		return

	var history = MatchManager.get_match_history(20)
	if history.size() < 2:
		return

	# Calculate rolling win rate
	var win_rate_data = PackedVector2Array()
	var wins = 0

	for i in range(history.size()):
		var record = history[i]
		if record.get("result", "draw") == "승리":
			wins += 1

		var win_rate = float(wins) / float(i + 1)
		win_rate_data.append(Vector2(float(i) / float(history.size() - 1), win_rate))

	line_chart.set_series([{"name": "승률", "points": win_rate_data, "color": Color(0.5, 1.0, 0.5)}])


func _load_match_performance_bars(bar_chart: BarChart):
	"""Load match performance bars (wins/draws/losses)"""
	if not MatchManager:
		return

	var stats = MatchManager.get_match_stats()

	bar_chart.set_data(
		[
			{"label": "승", "value": stats.get("wins", 0), "color": Color(0.5, 0.9, 0.5)},
			{"label": "무", "value": stats.get("draws", 0), "color": Color(0.7, 0.7, 0.7)},
			{"label": "패", "value": stats.get("losses", 0), "color": Color(0.9, 0.5, 0.5)}
		]
	)


func _load_training_by_type(bar_chart: BarChart):
	"""Load training sessions by type"""
	if not TrainingManager:
		return

	var stats = TrainingManager.get_training_stats()
	var sessions_by_type = stats.get("sessions_by_type", {})

	var data = []
	for type_name in sessions_by_type:
		data.append({"label": type_name, "value": sessions_by_type[type_name], "color": Color(0.4, 0.7, 1.0)})

	bar_chart.set_data(data)


func _load_attribute_growth(hexagon_chart: HexagonChart):
	"""Load current player attributes"""
	if not GlobalCharacterData:
		return

	var attributes = GlobalCharacterData.character_data.get("attributes", {})

	var hexagon_stats = {
		"PACE": attributes.get("Pace", 50) / 100.0,
		"POWER": attributes.get("Strength", 50) / 100.0,
		"TECHNICAL": attributes.get("Dribbling", 50) / 100.0,
		"SHOOTING": attributes.get("Finishing", 50) / 100.0,
		"PASSING": attributes.get("Passing", 50) / 100.0,
		"DEFENDING": attributes.get("Tackling", 50) / 100.0
	}

	hexagon_chart.set_stats(hexagon_stats, true)


func refresh_data():
	"""Refresh chart data"""
	_populate_content()

	# Animate refresh
	if chart_animator and chart_instance:
		chart_animator.animate_chart(chart_instance, "draw")
