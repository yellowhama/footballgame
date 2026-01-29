extends Control
class_name ShotMapCanvas
# Note: FieldDimensions has class_name, so it's globally available
const PERF_TAG := "ShotMapCanvas"
const MAX_REDRAW_HZ := 10
const MAX_SHOTS := 300

const BASE_FIELD_WIDTH := 1000.0
const BASE_FIELD_HEIGHT := 600.0
const REAL_FIELD_WIDTH := FieldSpec.FIELD_LENGTH_M
const REAL_FIELD_HEIGHT := FieldSpec.FIELD_WIDTH_M

const RedrawThrottle = preload("res://scripts/ui/_perf/RedrawThrottle.gd")

var shots: Array = []
var _fix02_warned_legacy_units: bool = false
var _redraw_throttle = RedrawThrottle.new()


func _ready() -> void:
	custom_minimum_size = Vector2(BASE_FIELD_WIDTH, BASE_FIELD_HEIGHT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func set_shots(new_shots: Array) -> void:
	shots = _cap_shots(new_shots)
	_request_redraw()


func clear_shots() -> void:
	shots.clear()
	_request_redraw()


func _cap_shots(input: Variant) -> Array:
	if not (input is Array):
		return []
	var arr: Array = input
	if arr.size() <= MAX_SHOTS:
		return arr

	# Keep the most recent shots deterministically.
	return arr.slice(arr.size() - MAX_SHOTS, arr.size())


func _request_redraw() -> void:
	_redraw_throttle.request_redraw(self, MAX_REDRAW_HZ, func():
		queue_redraw()
	)


func _draw() -> void:
	_draw_field()
	_draw_shots()


func _draw_field() -> void:
	var field_rect := _get_field_rect()
	draw_rect(field_rect, Color(0.13, 0.55, 0.13))
	var line_color := Color(0.9, 0.9, 0.9)
	var thickness := 3.0
	# Center line
	var center_x := field_rect.position.x + field_rect.size.x / 2.0
	draw_line(
		Vector2(center_x, field_rect.position.y),
		Vector2(center_x, field_rect.position.y + field_rect.size.y),
		line_color,
		thickness
	)
	# Penalty boxes
	var scale_x := field_rect.size.x / BASE_FIELD_WIDTH
	var scale_y := field_rect.size.y / BASE_FIELD_HEIGHT
	var box_width := 165.0 * scale_x
	var box_height := 403.0 * scale_y
	var top_offset := (field_rect.size.y - box_height) / 2.0
	var left_box := Rect2(field_rect.position + Vector2(0, top_offset), Vector2(box_width, box_height))
	var right_box := Rect2(
		field_rect.position + Vector2(field_rect.size.x - box_width, top_offset), Vector2(box_width, box_height)
	)
	draw_rect(left_box, line_color, false, thickness)
	draw_rect(right_box, line_color, false, thickness)
	# Outer border
	draw_rect(field_rect, line_color, false, thickness)


func _draw_shots() -> void:
	var field_rect := _get_field_rect()
	if shots.is_empty():
		return
	for shot in shots:
		if not (shot is Dictionary):
			continue
		var raw_pos := Vector2(float(shot.get("x", 0.0)), float(shot.get("y", 0.0)))
		var pos_m := raw_pos

		# FIX02 (좌표 SSOT): 기본 계약은 meters(0..105, 0..68).
		# Legacy 데이터(840x545 engine units)가 섞여오면 변환하되 경고를 남긴다.
		if pos_m.x > REAL_FIELD_WIDTH + 1.0 or pos_m.y > REAL_FIELD_HEIGHT + 1.0:
			if not _fix02_warned_legacy_units:
				_fix02_warned_legacy_units = true
				push_warning(
					"[FIX02][COORD] ShotMap received non-meter coordinates; assuming legacy engine units and converting to meters."
				)
			pos_m = Vector2(FieldDimensions.to_real_x(pos_m.x), FieldDimensions.to_real_y(pos_m.y))

		var position := _convert_to_canvas(pos_m, field_rect)
		var color := Color(0.2, 0.8, 1.0) if shot.get("team", "home") == "home" else Color(1.0, 0.3, 0.3)
		var radius := 10.0
		draw_circle(position, radius, color)
		if shot.get("result", "") == "goal":
			var inner := radius * 0.5
			draw_circle(position, inner, Color(1, 1, 0))


func _get_field_rect() -> Rect2:
	var scale: float = min(size.x / BASE_FIELD_WIDTH, size.y / BASE_FIELD_HEIGHT)
	var field_size: Vector2 = Vector2(BASE_FIELD_WIDTH, BASE_FIELD_HEIGHT) * scale
	var offset: Vector2 = (size - field_size) / 2.0
	return Rect2(offset, field_size)


func _convert_to_canvas(real_position: Vector2, field_rect: Rect2) -> Vector2:
	var x := real_position.x
	var y := real_position.y

	var half_width := REAL_FIELD_WIDTH / 2.0
	var half_height := REAL_FIELD_HEIGHT / 2.0

	var normalized_x: float
	var normalized_y: float

	if abs(x) <= half_width + 0.01:
		normalized_x = (x + half_width) / REAL_FIELD_WIDTH
	else:
		normalized_x = x / REAL_FIELD_WIDTH

	if abs(y) <= half_height + 0.01:
		normalized_y = (y + half_height) / REAL_FIELD_HEIGHT
	else:
		normalized_y = y / REAL_FIELD_HEIGHT

	normalized_x = clamp(normalized_x, 0.0, 1.0)
	normalized_y = clamp(normalized_y, 0.0, 1.0)

	return field_rect.position + Vector2(normalized_x * field_rect.size.x, normalized_y * field_rect.size.y)
