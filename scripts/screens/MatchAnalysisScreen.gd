extends Control
class_name MatchAnalysisScreen

## Match Analysis Screen
## Displays tactical patterns and insights from completed matches

@onready var continue_button: Button = $ContinueButton
@onready
var timeline_canvas: TimelineCanvas = $ScrollContainer/ContentContainer/TimelinePanel/PanelContent/TimelineCanvas
@onready var possession_chart: LineChart = $ScrollContainer/ContentContainer/PossessionPanel/PanelContent/PossessionChart
@onready var insights_list: VBoxContainer = $ScrollContainer/ContentContainer/PossessionPanel/PanelContent/InsightsList
@onready var danger_list: VBoxContainer = $ScrollContainer/ContentContainer/DangerPanel/PanelContent/DangerList
@onready
var zone_grid_canvas: ZoneGridCanvas = $ScrollContainer/ContentContainer/AttackZonesPanel/PanelContent/ContentHBox/ZoneGridCanvas
@onready
var zone_legend: VBoxContainer = $ScrollContainer/ContentContainer/AttackZonesPanel/PanelContent/ContentHBox/ZoneLegend
@onready var pressure_list: VBoxContainer = $ScrollContainer/ContentContainer/PressurePanel/PanelContent/PressureList

var analysis_report: Dictionary = {}
var bridge: Object = null


func _ready() -> void:
	# Initialize bridge
	if FootballRustEngine and FootballRustEngine._rust_simulator:
		bridge = FootballRustEngine._rust_simulator

	# Connect continue button
	if continue_button:
		continue_button.pressed.connect(_on_continue_pressed)

	# Load data from meta
	var root := get_tree().root
	if root.has_meta("match_analysis_data"):
		var data: Dictionary = root.get_meta("match_analysis_data")
		root.remove_meta("match_analysis_data")
		_load_analysis(data)


func _load_analysis(data: Dictionary) -> void:
	"""Load and process match analysis data"""
	if not bridge:
		push_error("[MatchAnalysisScreen] Bridge not available")
		return

	# Get match_result_json from data
	var match_result_json: String = data.get("match_result_json", "")
	if match_result_json == "":
		push_error("[MatchAnalysisScreen] No match_result_json provided")
		return

	# Call Rust analysis
	if bridge.has_method("get_match_analysis"):
		analysis_report = bridge.get_match_analysis(match_result_json)
	else:
		push_error("[MatchAnalysisScreen] get_match_analysis method not found in bridge")
		return

	# Check for errors
	if analysis_report.has("error"):
		push_error("[MatchAnalysisScreen] Analysis error: " + str(analysis_report.get("error")))
		return

	# Populate UI
	_populate_timeline()
	_populate_possession()
	_populate_danger_moments()
	_populate_attack_zones()
	_populate_pressure_patterns()


func _populate_timeline() -> void:
        """Populate timeline canvas with danger moments and possession shifts"""
        if not timeline_canvas:
                return

        var moments: Array = analysis_report.get("danger_timeline", [])
        var shifts: Array = analysis_report.get("possession_shifts", [])
        var duration: int = analysis_report.get("duration_minutes", 90)

        var dsa_series: Dictionary = {}
        # Prefer Rust authoritative DSA summary when present (stable for saved replays).
        var dsa_summary_variant: Variant = analysis_report.get("dsa_summary", null)
        if dsa_summary_variant is Dictionary:
                var dsa_summary: Dictionary = dsa_summary_variant
                var minute_series_variant: Variant = dsa_summary.get("minute_series", {})
                if minute_series_variant is Dictionary:
                        dsa_series = minute_series_variant

        # Fallback to live runtime DSA series (overlay-only; may be absent for saved loads).
        if dsa_series.is_empty():
                var dsa := get_node_or_null("/root/DistributedSensingManager")
                if dsa:
                        if dsa.has_method("get_minute_series"):
                                dsa_series = dsa.get_minute_series(duration)
                        elif dsa.has_method("get_minute_aggregates"):
                                dsa_series = _build_dsa_series_from_minute_aggs(dsa.get_minute_aggregates(), duration)

        timeline_canvas.set_data(moments, shifts, duration, dsa_series)


func _build_dsa_series_from_minute_aggs(minute_aggs: Dictionary, duration: int) -> Dictionary:
	var pressure_by_minute: Array = []
	var tempo_by_minute: Array = []
	var transitions_by_minute: Array = []

	pressure_by_minute.resize(duration + 1)
	tempo_by_minute.resize(duration + 1)
	transitions_by_minute.resize(duration + 1)
	for i in range(duration + 1):
		pressure_by_minute[i] = 0.0
		tempo_by_minute[i] = 0.0
		transitions_by_minute[i] = 0.0

	for minute_idx in range(duration + 1):
		var agg: Dictionary = minute_aggs.get(minute_idx, {})
		if agg.is_empty():
			continue

		var samples: int = int(agg.get("samples", 0))
		if samples > 0:
			pressure_by_minute[minute_idx] = float(agg.get("pressure_sum", 0.0)) / float(samples)
			tempo_by_minute[minute_idx] = float(agg.get("ball_speed_sum", 0.0)) / float(samples)
		transitions_by_minute[minute_idx] = float(agg.get("transitions", 0))

	return {
		"pressure": pressure_by_minute,
		"tempo": tempo_by_minute,
		"transitions": transitions_by_minute,
	}


func _populate_possession() -> void:
	"""Populate possession chart and insights list"""
	_populate_possession_chart()
	_populate_possession_insights()


func _populate_possession_chart() -> void:
	"""Generate possession trend line chart"""
	if not possession_chart:
		return

	var shifts: Array = analysis_report.get("possession_shifts", [])
	if shifts.is_empty():
		return

	# Generate data points for possession over time
	# We'll sample at 5-minute intervals (0, 5, 10, ..., 90)
	var home_points := PackedVector2Array()
	var away_points := PackedVector2Array()

	# Build possession timeline from shifts
	var possession_timeline: Array = _build_possession_timeline(shifts)

	# Sample points every 5 minutes
	var duration: int = analysis_report.get("duration_minutes", 90)
	for minute in range(0, duration + 1, 5):
		var poss_home := _get_possession_at_minute(minute, possession_timeline)
		var poss_away := 100.0 - poss_home

		# Normalize to 0-1 for LineChart
		var x := float(minute) / float(duration)
		var y_home := poss_home / 100.0
		var y_away := poss_away / 100.0

		home_points.append(Vector2(x, y_home))
		away_points.append(Vector2(x, y_away))

	# Set series
	var series := [
		{"name": "Home Possession", "points": home_points, "color": Color(0.2, 0.8, 1.0)},
		{"name": "Away Possession", "points": away_points, "color": Color(1.0, 0.3, 0.3)}
	]

	possession_chart.set_series(series)

		# Set X labels (time)
		var x_labels := ["0'", "15'", "30'", "45'", "60'", "75'", "%d'" % duration]
		possession_chart.set_x_labels(x_labels)

	# Set Y labels (percentage)
	var y_labels := ["0%", "25%", "50%", "75%", "100%"]
	possession_chart.set_y_labels(y_labels)


func _build_possession_timeline(shifts: Array) -> Array:
	"""Build timeline of possession values from shifts"""
	var timeline: Array = []
	var duration: int = analysis_report.get("duration_minutes", 90)

	if shifts.is_empty():
		# Default 50-50
		timeline.append({"minute": 0, "possession_home": 50.0})
		timeline.append({"minute": duration, "possession_home": 50.0})
		return timeline

	# Start with first shift's starting possession
	var first_shift: Dictionary = shifts[0]
	timeline.append({"minute": 0, "possession_home": first_shift.get("from_possession_home", 50.0)})

	# Add all shifts
	for shift in shifts:
		timeline.append(
			{
				"minute": shift.get("end_minute", duration),
				"possession_home": shift.get("to_possession_home", 50.0)
			}
		)

	return timeline


func _get_possession_at_minute(minute: int, timeline: Array) -> float:
	"""Get possession percentage at a given minute"""
	if timeline.is_empty():
		return 50.0

	# Find the last timeline point before or at this minute
	var last_poss := 50.0
	for point in timeline:
		var point_minute: int = point.get("minute", 0)
		if point_minute <= minute:
			last_poss = point.get("possession_home", 50.0)
		else:
			break

	return last_poss


func _populate_possession_insights() -> void:
	"""Populate possession shift insights list"""
	if not insights_list:
		return

	# Clear existing children
	for child in insights_list.get_children():
		child.queue_free()

	var shifts: Array = analysis_report.get("possession_shifts", [])
	if shifts.is_empty():
		var label := Label.new()
		label.text = "No significant possession shifts detected"
		label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
		insights_list.add_child(label)
		return

	# Add shift entries
	for shift in shifts:
		var entry := _create_possession_shift_entry(shift)
		insights_list.add_child(entry)


func _create_possession_shift_entry(shift: Dictionary) -> Control:
	"""Create a possession shift entry widget"""
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(0, 60)

	var hbox := HBoxContainer.new()
	panel.add_child(hbox)

	# Time range
	var time_label := Label.new()
	time_label.custom_minimum_size = Vector2(80, 0)
	var start: int = int(shift.get("start_minute", 0))
	var end: int = int(shift.get("end_minute", 90))
	time_label.text = "%d' - %d'" % [start, end]
	time_label.add_theme_font_size_override("font_size", 14)
	hbox.add_child(time_label)

	# Description
	var desc_label := Label.new()
	desc_label.text = shift.get("description", "Possession shift")
	desc_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	desc_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_child(desc_label)

	# Magnitude indicator
	# Magnitude is in percentage points (0..100) from Rust MatchAnalysisReport.
	var mag_pp: float = float(shift.get("magnitude", 0.0))
	var mag_label := Label.new()
	mag_label.text = "±%.1f%%" % mag_pp
	mag_label.add_theme_font_size_override("font_size", 14)
	var tier: String = str(shift.get("tier", "significant"))
	var mag_color: Color
	match tier:
		"extreme":
			mag_color = Color(1.0, 0.25, 0.25)
		"major":
			mag_color = Color(1.0, 0.5, 0.2)
		_:
			mag_color = Color(0.7, 0.7, 0.7)
	mag_label.add_theme_color_override("font_color", mag_color)
	hbox.add_child(mag_label)

	return panel


func _populate_danger_moments() -> void:
	"""Populate danger moments list"""
	if not danger_list:
		return

	# Clear existing children
	for child in danger_list.get_children():
		child.queue_free()

	var moments: Array = analysis_report.get("danger_timeline", [])
	if moments.is_empty():
		var label := Label.new()
		label.text = "No high-danger moments detected"
		label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
		danger_list.add_child(label)
		return

	# Add moment entries (sorted by xG descending)
	for moment in moments:
		var entry := _create_danger_moment_entry(moment)
		danger_list.add_child(entry)


func _create_danger_moment_entry(moment: Dictionary) -> Control:
	"""Create a danger moment entry widget"""
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(0, 70)

	var vbox := VBoxContainer.new()
	panel.add_child(vbox)

	# Top row: minute + xG + team
	var top_hbox := HBoxContainer.new()
	vbox.add_child(top_hbox)

	var minute_label := Label.new()
	minute_label.text = "%d'" % moment.get("minute", 0)
	minute_label.custom_minimum_size = Vector2(50, 0)
	minute_label.add_theme_font_size_override("font_size", 16)
	minute_label.add_theme_color_override("font_color", Color(1.0, 1.0, 0.5))
	top_hbox.add_child(minute_label)

	var xg_label := Label.new()
	xg_label.text = "xG: %.2f" % moment.get("xg_value", 0.0)
	xg_label.add_theme_font_size_override("font_size", 14)
	top_hbox.add_child(xg_label)

	top_hbox.add_child(Control.new())  # Spacer
	top_hbox.get_child(-1).size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var team_label := Label.new()
	var is_home: bool = moment.get("is_home", true)
	team_label.text = "Home" if is_home else "Away"
	var team_color := Color(0.2, 0.8, 1.0) if is_home else Color(1.0, 0.3, 0.3)
	team_label.add_theme_color_override("font_color", team_color)
	team_label.add_theme_font_size_override("font_size", 14)
	top_hbox.add_child(team_label)

	# Bottom row: player + event description
	var desc_label := Label.new()
	var player_name: String = str(moment.get("player_name", "Unknown"))
	var event_type: String = str(moment.get("event_type", "Shot"))
	desc_label.text = "%s - %s" % [player_name, event_type]
	desc_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(desc_label)

	return panel


func _populate_attack_zones() -> void:
	"""Populate attack zones visualization"""
	_populate_zone_grid()
	_populate_zone_legend()


func _populate_zone_grid() -> void:
	"""Populate zone grid canvas"""
	if not zone_grid_canvas:
		return

	var attack_zones_data: Dictionary = analysis_report.get("attack_zones", {})
	var zones: Array = attack_zones_data.get("zones", [])
	var total: int = attack_zones_data.get("total_attacks", 0)

	zone_grid_canvas.set_zones(zones, total)


func _populate_zone_legend() -> void:
	"""Populate zone legend list"""
	if not zone_legend:
		return

	# Clear existing children
	for child in zone_legend.get_children():
		child.queue_free()

	# Title
	var title := Label.new()
	title.text = "Attack Distribution"
	title.add_theme_font_size_override("font_size", 18)
	title.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
	zone_legend.add_child(title)

	var attack_zones_data: Dictionary = analysis_report.get("attack_zones", {})
	var zones: Array = attack_zones_data.get("zones", [])
	var dominant_zone: String = attack_zones_data.get("dominant_zone", "")
	var total_attacks: int = attack_zones_data.get("total_attacks", 0)

	if zones.is_empty():
		var label := Label.new()
		label.text = "No attack data available"
		label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
		zone_legend.add_child(label)
		return

	# Total attacks
	var total_label := Label.new()
	total_label.text = "Total Attacks: %d" % total_attacks
	total_label.add_theme_font_size_override("font_size", 14)
	zone_legend.add_child(total_label)

	# Dominant zone
	if dominant_zone != "":
		var dom_label := Label.new()
		dom_label.text = "Dominant: %s" % dominant_zone
		dom_label.add_theme_color_override("font_color", Color(1.0, 0.8, 0.3))
		zone_legend.add_child(dom_label)

	# Spacer
	zone_legend.add_child(Control.new())
	zone_legend.get_child(-1).custom_minimum_size = Vector2(0, 20)

	# Zone list (sorted by percentage descending)
	var sorted_zones := zones.duplicate()
	sorted_zones.sort_custom(func(a, b): return a.get("percentage", 0.0) > b.get("percentage", 0.0))

	for zone in sorted_zones:
		var entry := _create_zone_legend_entry(zone)
		zone_legend.add_child(entry)


func _create_zone_legend_entry(zone: Dictionary) -> Control:
	"""Create a zone legend entry"""
	var hbox := HBoxContainer.new()

	var name_label := Label.new()
	name_label.text = zone.get("zone_name", "Unknown")
	name_label.custom_minimum_size = Vector2(150, 0)
	hbox.add_child(name_label)

	var count_label := Label.new()
	count_label.text = "%d attacks" % zone.get("attack_count", 0)
	count_label.custom_minimum_size = Vector2(80, 0)
	hbox.add_child(count_label)

	var pct_label := Label.new()
	pct_label.text = "%.1f%%" % zone.get("percentage", 0.0)
	pct_label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	hbox.add_child(pct_label)

	return hbox


func _populate_pressure_patterns() -> void:
	"""Populate pressure patterns list"""
	if not pressure_list:
		return

	# Clear existing children
	for child in pressure_list.get_children():
		child.queue_free()

	var patterns: Array = analysis_report.get("pressure_patterns", [])
	if patterns.is_empty():
		var label := Label.new()
		label.text = "No significant pressure patterns detected"
		label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
		pressure_list.add_child(label)
		return

	# Add pattern entries
	for pattern in patterns:
		var entry := _create_pressure_pattern_entry(pattern)
		pressure_list.add_child(entry)


func _create_pressure_pattern_entry(pattern: Dictionary) -> Control:
	"""Create a pressure pattern entry widget"""
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(0, 70)

	var vbox := VBoxContainer.new()
	panel.add_child(vbox)

	# Top row: time range + intensity
	var top_hbox := HBoxContainer.new()
	vbox.add_child(top_hbox)

	var time_label := Label.new()
	var start: int = int(pattern.get("start_minute", 0))
	var end: int = int(pattern.get("end_minute", 90))
	time_label.text = "%d' - %d'" % [start, end]
	time_label.custom_minimum_size = Vector2(80, 0)
	time_label.add_theme_font_size_override("font_size", 14)
	top_hbox.add_child(time_label)

	var intensity: String = str(pattern.get("intensity", "Medium"))
	var intensity_label := Label.new()
	intensity_label.text = intensity + " Intensity"
	var intensity_color := Color(1.0, 0.3, 0.3) if intensity == "High" else Color(0.8, 0.8, 0.3)
	intensity_label.add_theme_color_override("font_color", intensity_color)
	top_hbox.add_child(intensity_label)

	# Middle row: pressing team + third
	var mid_hbox := HBoxContainer.new()
	vbox.add_child(mid_hbox)

	var team_label := Label.new()
	team_label.text = "%s pressing in %s" % [pattern.get("pressing_team", ""), pattern.get("field_third", "")]
	team_label.add_theme_font_size_override("font_size", 12)
	mid_hbox.add_child(team_label)

	# Bottom row: description
	var desc_label := Label.new()
	desc_label.text = pattern.get("description", "")
	desc_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	desc_label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	vbox.add_child(desc_label)

	return panel


func _on_continue_pressed() -> void:
	"""Handle continue button press"""
	# Return to Home screen
	var home_scene := "res://scenes/HomeScreen.tscn"
	if ResourceLoader.exists(home_scene):
		if has_node("/root/ScreenTransition"):
			get_node("/root/ScreenTransition").change_scene(home_scene, "slide_left")
		else:
			get_tree().change_scene_to_file(home_scene)
	else:
		# Fallback
		get_tree().change_scene_to_file("res://scenes/menus/main_menu.tscn")
