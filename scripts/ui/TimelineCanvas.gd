extends Control
class_name TimelineCanvas

## Match Timeline Visualization
## Displays 0-90 minute timeline with danger moments and possession shifts

const PERF_TAG := "TimelineCanvas"
const MAX_REDRAW_HZ := 5
const MAX_MATCH_DURATION_MINUTES := 90
const MAX_MOMENTS := 60
const MAX_SHIFTS := 30
const MAX_SERIES := 91 # 90 minutes + kickoff tick

const TIMELINE_HEIGHT := 120.0
const TIMELINE_MIN_WIDTH := 1000.0
const MARGIN_X := 40.0
const MARGIN_Y := 20.0

const RedrawThrottle = preload("res://scripts/ui/_perf/RedrawThrottle.gd")

var danger_moments: Array = []
var possession_shifts: Array = []
var match_duration := 90
var _redraw_throttle = RedrawThrottle.new()

# Optional additive overlay data (derived telemetry).
# This MUST be computed elsewhere (e.g., DistributedSensingManager) to keep render loop clean.
var dsa_series: Dictionary = {}
var _dsa_pressure_norm: Array = []
var _dsa_tempo_norm: Array = []
var _dsa_transitions_norm: Array = []


func _ready() -> void:
	custom_minimum_size = Vector2(TIMELINE_MIN_WIDTH, TIMELINE_HEIGHT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func set_data(moments: Array, shifts: Array, duration: int = 90, dsa: Dictionary = {}) -> void:
	"""
	Set timeline data
 	@param moments: Array of DangerMoment dictionaries
 	@param shifts: Array of PossessionShift dictionaries
 	@param duration: Match duration in minutes (default 90)
	@param dsa: Optional derived series (e.g. {"pressure":[...], "tempo":[...], "transitions":[...]})
 	"""
	danger_moments = _cap_items(moments if moments else [], MAX_MOMENTS)
	possession_shifts = _cap_items(shifts if shifts else [], MAX_SHIFTS)
	match_duration = clamp(duration, 0, MAX_MATCH_DURATION_MINUTES)
	_set_dsa_series(dsa)
	_request_redraw()


func clear_data() -> void:
	danger_moments.clear()
	possession_shifts.clear()
	dsa_series.clear()
	_dsa_pressure_norm.clear()
	_dsa_tempo_norm.clear()
	_dsa_transitions_norm.clear()
	_request_redraw()


func _cap_items(items: Variant, max_items: int) -> Array:
	if not (items is Array):
		return []

	var arr: Array = items
	if max_items <= 0 or arr.size() <= max_items:
		return arr

	# Deterministic downsample: keep distribution across the full timeline.
	var out: Array = []
	out.resize(max_items)
	if max_items == 1:
		out[0] = arr[0]
		return out

	var step: float = float(arr.size() - 1) / float(max_items - 1)
	for i in range(max_items):
		var idx := int(round(step * float(i)))
		idx = clamp(idx, 0, arr.size() - 1)
		out[i] = arr[idx]
	return out


func _draw() -> void:
	var timeline_rect := _get_timeline_rect()
	_draw_background(timeline_rect)
	_draw_possession_shifts(timeline_rect)
	_draw_dsa_overlay(timeline_rect)
	_draw_timeline_axis(timeline_rect)
	_draw_danger_moments(timeline_rect)


func _get_timeline_rect() -> Rect2:
	"""Calculate the drawable timeline area"""
	var width := size.x - (MARGIN_X * 2.0)
	var height := size.y - (MARGIN_Y * 2.0)
	return Rect2(MARGIN_X, MARGIN_Y, width, height)


func _draw_background(rect: Rect2) -> void:
	"""Draw timeline background"""
	var bg_color := Color(0.15, 0.15, 0.20, 0.8)
	draw_rect(rect, bg_color)


func _draw_possession_shifts(rect: Rect2) -> void:
	"""Draw possession shift regions as subtle background highlights"""
	if possession_shifts.is_empty():
		return

	for shift in possession_shifts:
		if not (shift is Dictionary):
			continue

		var start_min: int = shift.get("start_minute", 0)
		var end_min: int = shift.get("end_minute", 90)
		# Magnitude is in percentage points (0..100) from Rust MatchAnalysisReport.
		var magnitude_pp: float = shift.get("magnitude", 0.0)

		# Convert minutes to x position
		var start_x := rect.position.x + (float(start_min) / float(match_duration)) * rect.size.x
		var end_x := rect.position.x + (float(end_min) / float(match_duration)) * rect.size.x
		var width := end_x - start_x

		# NOTE: Visualization scale only; semantic thresholding must be done in Rust.
		var alpha: float = float(clamp(magnitude_pp / 50.0, 0.15, 0.5))
		var color := Color(1.0, 0.8, 0.2, alpha)

		# Draw shift region
		var shift_rect := Rect2(start_x, rect.position.y, width, rect.size.y)
		draw_rect(shift_rect, color)


func _draw_timeline_axis(rect: Rect2) -> void:
	"""Draw timeline axis with minute markers"""
	var line_color := Color(0.7, 0.7, 0.7)
	var text_color := Color(0.9, 0.9, 0.9)
	var axis_y := rect.position.y + rect.size.y

	# Main axis line
	draw_line(Vector2(rect.position.x, axis_y), Vector2(rect.position.x + rect.size.x, axis_y), line_color, 2.0)

	# Minute markers (every 15 minutes)
	for minute in [0, 15, 30, 45, 60, 75, 90]:
		if minute > match_duration:
			continue

		var x := rect.position.x + (float(minute) / float(match_duration)) * rect.size.x

		# Tick mark
		var tick_height := 8.0 if minute % 45 == 0 else 5.0
		draw_line(Vector2(x, axis_y), Vector2(x, axis_y + tick_height), line_color, 2.0)

		# Minute label
		var label := str(minute) + "'"
		var font_size := 14
		draw_string(
			ThemeDB.fallback_font,
			Vector2(x - 10, axis_y + tick_height + 15),
			label,
			HORIZONTAL_ALIGNMENT_CENTER,
			-1,
			font_size,
			text_color
		)

	# Half-time marker (special)
	var half_time_x := rect.position.x + (45.0 / float(match_duration)) * rect.size.x
	var half_time_color := Color(1.0, 1.0, 0.0, 0.6)
	draw_line(Vector2(half_time_x, rect.position.y), Vector2(half_time_x, axis_y), half_time_color, 2.0)

	# "HT" label
	draw_string(
		ThemeDB.fallback_font,
		Vector2(half_time_x - 10, rect.position.y - 5),
		"HT",
		HORIZONTAL_ALIGNMENT_CENTER,
		-1,
		12,
		half_time_color
	)


func _draw_danger_moments(rect: Rect2) -> void:
	"""Draw danger moment markers sized by xG value (tiered in Rust)."""
	if danger_moments.is_empty():
		return

	var center_y := rect.position.y + rect.size.y * 0.4

	for moment in danger_moments:
		if not (moment is Dictionary):
			continue

		var minute: int = moment.get("minute", 0)
		if minute > match_duration:
			continue

		var xg: float = moment.get("xg_value", 0.0)
		var tier: String = str(moment.get("tier", "normal"))
		var is_home: bool = moment.get("is_home", true)

		# Convert minute to x position
		var x := rect.position.x + (float(minute) / float(match_duration)) * rect.size.x

		# Size based on xG (min 6px, max 20px)
		var base_radius := 6.0
		var max_radius := 20.0
		var radius := base_radius + (xg * (max_radius - base_radius))
		radius = clamp(radius, base_radius, max_radius)

		# Color based on team
		var color := Color(0.2, 0.8, 1.0) if is_home else Color(1.0, 0.3, 0.3)

		# Draw circle
		draw_circle(Vector2(x, center_y), radius, color)

		# Draw inner highlight for high-tier moments (tier computed in Rust)
		if tier == "high":
			var inner_radius := radius * 0.5
			draw_circle(Vector2(x, center_y), inner_radius, Color(1.0, 1.0, 0.0, 0.8))


func _set_dsa_series(dsa: Dictionary) -> void:
	dsa_series = dsa if dsa else {}
	_dsa_pressure_norm.clear()
	_dsa_tempo_norm.clear()
	_dsa_transitions_norm.clear()

	if dsa_series.is_empty():
		return

	var pressure_raw: Array = _fit_series_to_duration(dsa_series.get("pressure", []), match_duration)
	var tempo_raw: Array = _fit_series_to_duration(dsa_series.get("tempo", []), match_duration)
	var transitions_raw: Array = _fit_series_to_duration(dsa_series.get("transitions", []), match_duration)

	_dsa_pressure_norm = _clamp01_series(pressure_raw)
	_dsa_tempo_norm = _normalize_series(tempo_raw)
	_dsa_transitions_norm = _normalize_series(transitions_raw)


func _fit_series_to_duration(values: Variant, duration: int) -> Array:
	var out: Array = []
	var n: int = clamp(duration, 0, MAX_MATCH_DURATION_MINUTES) + 1
	out.resize(n)
	for i in range(n):
		out[i] = 0.0

	if values is Array:
		var src_arr: Array = values
		var m: int = min(src_arr.size(), n)
		for i in range(m):
			out[i] = float(src_arr[i])
	elif values is PackedFloat32Array:
		var src_f: PackedFloat32Array = values
		var m: int = min(src_f.size(), n)
		for i in range(m):
			out[i] = float(src_f[i])
	elif values is PackedInt32Array:
		var src_i: PackedInt32Array = values
		var m: int = min(src_i.size(), n)
		for i in range(m):
			out[i] = float(src_i[i])
	return out


func _request_redraw() -> void:
	_redraw_throttle.request_redraw(self, MAX_REDRAW_HZ, func():
		queue_redraw()
	)


func _clamp01_series(values: Array) -> Array:
	var out: Array = []
	out.resize(values.size())
	for i in range(values.size()):
		out[i] = clamp(float(values[i]), 0.0, 1.0)
	return out


func _normalize_series(values: Array) -> Array:
	var max_v: float = 0.0
	for v in values:
		max_v = max(max_v, float(v))
	if max_v <= 0.0001:
		max_v = 1.0

	var out: Array = []
	out.resize(values.size())
	for i in range(values.size()):
		out[i] = clamp(float(values[i]) / max_v, 0.0, 1.0)
	return out


func _draw_dsa_overlay(rect: Rect2) -> void:
	if dsa_series.is_empty():
		return
	if _dsa_pressure_norm.is_empty() and _dsa_tempo_norm.is_empty() and _dsa_transitions_norm.is_empty():
		return

	var y_top := rect.position.y + rect.size.y * 0.10
	var y_bottom := rect.position.y + rect.size.y * 0.88

	# Pressure / tempo lines (thin, semi-transparent)
	if not _dsa_pressure_norm.is_empty():
		_draw_series_line(rect, _dsa_pressure_norm, y_top, y_bottom, Color(1.0, 0.35, 0.25, 0.75))
	if not _dsa_tempo_norm.is_empty():
		_draw_series_line(rect, _dsa_tempo_norm, y_top, y_bottom, Color(0.25, 1.0, 0.65, 0.65))

	# Transitions as small ticks near the bottom
	if not _dsa_transitions_norm.is_empty():
		_draw_series_ticks(rect, _dsa_transitions_norm, y_bottom, Color(1.0, 1.0, 1.0, 0.25))

	# Minimal label
	draw_string(
		ThemeDB.fallback_font,
		rect.position + Vector2(6, 14),
		"DSA",
		HORIZONTAL_ALIGNMENT_LEFT,
		-1,
		12,
		Color(0.9, 0.9, 0.9, 0.55)
	)


func _draw_series_line(rect: Rect2, series01: Array, y_top: float, y_bottom: float, color: Color) -> void:
	var points: PackedVector2Array = PackedVector2Array()
	points.resize(series01.size())

	var denom: float = max(1.0, float(match_duration))
	for i in range(series01.size()):
		var x: float = rect.position.x + (float(i) / denom) * rect.size.x
		var v: float = clampf(float(series01[i]), 0.0, 1.0)
		var y: float = lerpf(y_bottom, y_top, v)
		points[i] = Vector2(x, y)

	draw_polyline(points, color, 2.0, true)


func _draw_series_ticks(rect: Rect2, series01: Array, y: float, color: Color) -> void:
	var denom: float = max(1.0, float(match_duration))
	for i in range(series01.size()):
		var v: float = clampf(float(series01[i]), 0.0, 1.0)
		if v <= 0.001:
			continue

		var x: float = rect.position.x + (float(i) / denom) * rect.size.x
		var h: float = lerpf(2.0, 12.0, v)
		draw_line(Vector2(x, y), Vector2(x, y - h), color, 2.0)
