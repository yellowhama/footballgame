extends Control
class_name HeatMapCanvas

const PERF_TAG := "HeatMapCanvas"
const MAX_REDRAW_HZ := 5
const MAX_GRID_SIZE := Vector2i(64, 48)
const MAX_CELLS := 64 * 48

const FIELD_SIZE := Vector2(105.0, 68.0)
const BACKGROUND_COLOR := Color(0.08, 0.12, 0.18, 1.0)
const PITCH_COLOR := Color(0.09, 0.4, 0.17, 1.0)
const LINE_COLOR := Color(0.9, 0.9, 0.9, 0.85)
const MIN_CELL_ALPHA := 0.35
const MAX_CELL_ALPHA := 0.9

const RedrawThrottle = preload("res://scripts/ui/_perf/RedrawThrottle.gd")

var _grid: PackedFloat32Array = PackedFloat32Array()
var _smoothed_grid: PackedFloat32Array = PackedFloat32Array()
var _grid_size: Vector2i = Vector2i.ZERO
var _max_intensity: float = 0.0001
var _smoothed_max: float = 0.0001
var _player_name: String = ""
var _redraw_throttle = RedrawThrottle.new()
var _warned_downsample: bool = false


func set_heat_data(data: Dictionary) -> void:
	_grid = data.get("grid", PackedFloat32Array())
	_grid_size = data.get("grid_size", Vector2i.ZERO)
	_max_intensity = max(float(data.get("max_intensity", 0.0)), 0.0001)
	_player_name = str(data.get("player_name", ""))

	if _grid.is_empty() or _grid_size == Vector2i.ZERO:
		_smoothed_grid = PackedFloat32Array()
		_smoothed_max = 0.0001
	else:
		_apply_grid_caps()
		_smoothed_grid = _smooth_grid(_grid, _grid_size)
		_smoothed_max = max(_max_intensity, _max_value(_smoothed_grid))

	_request_redraw()


func clear() -> void:
	_grid = PackedFloat32Array()
	_smoothed_grid = PackedFloat32Array()
	_grid_size = Vector2i.ZERO
	_max_intensity = 0.0001
	_smoothed_max = 0.0001
	_player_name = ""
	_request_redraw()


func _apply_grid_caps() -> void:
	if _grid_size.x <= 0 or _grid_size.y <= 0:
		return

	# Ensure grid size matches the provided data length (avoid out-of-bounds in smoothing).
	var expected_cells := _grid_size.x * _grid_size.y
	if expected_cells > _grid.size():
		var new_y := int(floor(float(_grid.size()) / float(max(_grid_size.x, 1))))
		if new_y <= 0:
			_grid = PackedFloat32Array()
			_grid_size = Vector2i.ZERO
			return
		_grid_size = Vector2i(_grid_size.x, min(_grid_size.y, new_y))

	# Enforce worst-case cap by downsampling (nearest-neighbor).
	var cells := _grid_size.x * _grid_size.y
	if cells <= MAX_CELLS and _grid_size.x <= MAX_GRID_SIZE.x and _grid_size.y <= MAX_GRID_SIZE.y:
		return

	var target := Vector2i(min(_grid_size.x, MAX_GRID_SIZE.x), min(_grid_size.y, MAX_GRID_SIZE.y))
	if target.x <= 0 or target.y <= 0:
		_grid = PackedFloat32Array()
		_grid_size = Vector2i.ZERO
		return

	if not _warned_downsample:
		_warned_downsample = true
		push_warning(
			"[FIX_2601][PERF][%s] HeatMap grid too large (%dx%d); downsampling to %dx%d (MAX_CELLS=%d)."
			% [PERF_TAG, _grid_size.x, _grid_size.y, target.x, target.y, MAX_CELLS]
		)

	_grid = _downsample_grid_nearest(_grid, _grid_size, target)
	_grid_size = target


func _downsample_grid_nearest(source: PackedFloat32Array, src_size: Vector2i, dst_size: Vector2i) -> PackedFloat32Array:
	var dest := PackedFloat32Array()
	var dst_cells := dst_size.x * dst_size.y
	dest.resize(max(dst_cells, 0))

	if source.is_empty() or src_size.x <= 0 or src_size.y <= 0 or dst_size.x <= 0 or dst_size.y <= 0:
		return dest

	var src_w := float(src_size.x)
	var src_h := float(src_size.y)
	var dst_w := float(dst_size.x)
	var dst_h := float(dst_size.y)

	for y in dst_size.y:
		var src_y := int(round(((float(y) + 0.5) * src_h / dst_h) - 0.5))
		src_y = clamp(src_y, 0, src_size.y - 1)
		for x in dst_size.x:
			var src_x := int(round(((float(x) + 0.5) * src_w / dst_w) - 0.5))
			src_x = clamp(src_x, 0, src_size.x - 1)
			var src_idx := src_y * src_size.x + src_x
			var dst_idx := y * dst_size.x + x
			dest[dst_idx] = source[src_idx] if src_idx >= 0 and src_idx < source.size() else 0.0

	return dest


func _request_redraw() -> void:
	_redraw_throttle.request_redraw(self, MAX_REDRAW_HZ, func():
		queue_redraw()
	)


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, size), BACKGROUND_COLOR, true)
	var field_rect := _compute_field_rect()
	_draw_pitch(field_rect)

	var grid: PackedFloat32Array = _grid if _smoothed_grid.is_empty() else _smoothed_grid
	var reference_max: float = max(_smoothed_max, 0.0001)
	if grid.is_empty() or _grid_size == Vector2i.ZERO:
		_draw_empty_placeholder(field_rect)
		return

	var cell_size := field_rect.size / Vector2(_grid_size)
	for y in _grid_size.y:
		for x in _grid_size.x:
			var idx := y * _grid_size.x + x
			if idx >= grid.size():
				continue
			var value := grid[idx]
			if value <= 0.0001:
				continue

			var intensity: float = clamp(value / reference_max, 0.0, 1.0)
			if intensity <= 0.01:
				continue

			var cell_pos := field_rect.position + Vector2(x, y) * cell_size
			var cell_rect := Rect2(cell_pos, cell_size).grow_individual(-1.0, -1.0, -1.0, -1.0)
			var color := _heat_color(intensity)
			color.a = lerp(MIN_CELL_ALPHA, MAX_CELL_ALPHA, intensity)
			draw_rect(cell_rect, color, true)

	_draw_pitch_overlay(field_rect)
	_draw_player_label(field_rect)


func _compute_field_rect() -> Rect2:
	var aspect: float = FIELD_SIZE.x / FIELD_SIZE.y
	var canvas_aspect: float = size.x / max(size.y, 0.001)
	var field_size := Vector2.ZERO
	if canvas_aspect > aspect:
		field_size.y = size.y * 0.9
		field_size.x = field_size.y * aspect
	else:
		field_size.x = size.x * 0.9
		field_size.y = field_size.x / aspect
	var offset := (size - field_size) / 2.0
	return Rect2(offset, field_size)


func _draw_pitch(rect: Rect2) -> void:
	draw_rect(rect, PITCH_COLOR, true)
	draw_rect(rect, LINE_COLOR, false, 2.0)
	var mid_x := rect.position.x + rect.size.x / 2.0
	draw_line(Vector2(mid_x, rect.position.y), Vector2(mid_x, rect.position.y + rect.size.y), LINE_COLOR, 1.5)
	var center_radius := rect.size.x * 0.12
	draw_arc(rect.position + rect.size / 2.0, center_radius, 0, TAU, 48, LINE_COLOR, 1.2)


func _draw_pitch_overlay(rect: Rect2) -> void:
	var box_width := rect.size.x * 0.18
	var box_height := rect.size.y * 0.32
	var penalty_width := rect.size.x * 0.08
	var penalty_height := rect.size.y * 0.12

	var top_box := Rect2(rect.position.x + (rect.size.x - box_width) / 2.0, rect.position.y, box_width, penalty_height)
	var bottom_box := Rect2(
		rect.position.x + (rect.size.x - box_width) / 2.0,
		rect.position.y + rect.size.y - penalty_height,
		box_width,
		penalty_height
	)
	draw_rect(top_box, LINE_COLOR, false, 1.0)
	draw_rect(bottom_box, LINE_COLOR, false, 1.0)

	var top_area := Rect2(
		rect.position.x + (rect.size.x - penalty_width) / 2.0, rect.position.y, penalty_width, box_height
	)
	var bottom_area := Rect2(
		rect.position.x + (rect.size.x - penalty_width) / 2.0,
		rect.position.y + rect.size.y - box_height,
		penalty_width,
		box_height
	)
	draw_rect(top_area, LINE_COLOR, false, 1.0)
	draw_rect(bottom_area, LINE_COLOR, false, 1.0)
	draw_circle(rect.position + Vector2(rect.size.x / 2.0, rect.size.y * 0.12), rect.size.x * 0.01, LINE_COLOR)
	draw_circle(rect.position + Vector2(rect.size.x / 2.0, rect.size.y * 0.88), rect.size.x * 0.01, LINE_COLOR)


func _draw_player_label(rect: Rect2) -> void:
	if _player_name == "":
		return
	var font := get_theme_default_font()
	if font == null:
		return
	var text := "[%s]" % _player_name
	var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14)
	var label_pos := Vector2(rect.position.x, rect.position.y - text_size.y - 6)
	if label_pos.y < 8:
		label_pos.y = rect.position.y + rect.size.y + text_size.y + 6
	draw_string(font, label_pos, text, HORIZONTAL_ALIGNMENT_LEFT, rect.size.x, 14, LINE_COLOR)


func _draw_empty_placeholder(rect: Rect2) -> void:
	var font := get_theme_default_font()
	if font == null:
		return
	var message := "데이터 없음"
	var text_size := font.get_string_size(message, HORIZONTAL_ALIGNMENT_LEFT, -1, 16)
	var position := rect.position + rect.size / 2.0 - text_size / 2.0
	draw_string(font, position, message, HORIZONTAL_ALIGNMENT_LEFT, -1, 16, Color(1, 1, 1, 0.6))


func _smooth_grid(source: PackedFloat32Array, size: Vector2i) -> PackedFloat32Array:
	if source.is_empty() or size.x <= 0 or size.y <= 0:
		return PackedFloat32Array()

	var dest := PackedFloat32Array()
	dest.resize(source.size())

	var kernel := [[1.0, 2.0, 1.0], [2.0, 4.0, 2.0], [1.0, 2.0, 1.0]]

	for y in size.y:
		for x in size.x:
			var accum := 0.0
			var weight_sum := 0.0
			for ky in range(-1, 2):
				var ny := y + ky
				if ny < 0 or ny >= size.y:
					continue
				for kx in range(-1, 2):
					var nx := x + kx
					if nx < 0 or nx >= size.x:
						continue
					var weight: float = kernel[ky + 1][kx + 1]
					var src_idx := ny * size.x + nx
					accum += source[src_idx] * weight
					weight_sum += weight
			var dst_idx := y * size.x + x
			if weight_sum > 0.0:
				dest[dst_idx] = accum / weight_sum
			else:
				dest[dst_idx] = source[dst_idx]

	return dest


func _max_value(values: PackedFloat32Array) -> float:
	var result := 0.0
	for value in values:
		result = max(result, value)
	return result


static func heat_color_from_intensity(intensity: float) -> Color:
	intensity = clamp(intensity, 0.0, 1.0)
	var cold := Color(0.1, 0.25, 0.7, 0.65)
	var warm := Color(0.95, 0.2, 0.1, 0.9)
	return cold.lerp(warm, intensity)


func _heat_color(intensity: float) -> Color:
	return heat_color_from_intensity(intensity)
