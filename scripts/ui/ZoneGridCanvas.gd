extends Control
class_name ZoneGridCanvas

## Attack Zone Distribution Visualization
## Displays 3×3 grid with attack zone markers

const PERF_TAG := "ZoneGridCanvas"
const MAX_REDRAW_HZ := 5
const MAX_ZONES := 9

const CANVAS_WIDTH := 600.0
const CANVAS_HEIGHT := 400.0
const FIELD_ASPECT_RATIO := 105.0 / 68.0  # Length / Width
const MARGIN := 20.0

const RedrawThrottle = preload("res://scripts/ui/_perf/RedrawThrottle.gd")

var attack_zones: Array = []
var total_attacks: int = 0
var _redraw_throttle = RedrawThrottle.new()


func _ready() -> void:
	custom_minimum_size = Vector2(CANVAS_WIDTH, CANVAS_HEIGHT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func set_zones(zones: Array, total: int = 0) -> void:
	"""
	Set attack zone data
	@param zones: Array of AttackZone dictionaries
	@param total: Total attack count
	"""
	attack_zones = _cap_zones(zones) if zones else []
	total_attacks = total
	_request_redraw()


func clear_zones() -> void:
	attack_zones.clear()
	total_attacks = 0
	_request_redraw()


func _cap_zones(input: Variant) -> Array:
	if not (input is Array):
		return []
	var arr: Array = input
	if arr.size() <= MAX_ZONES:
		return arr
	return arr.slice(0, MAX_ZONES)


func _request_redraw() -> void:
	_redraw_throttle.request_redraw(self, MAX_REDRAW_HZ, func():
		queue_redraw()
	)


func _draw() -> void:
	var field_rect := _get_field_rect()
	_draw_field(field_rect)
	_draw_grid(field_rect)
	_draw_attack_markers(field_rect)


func _get_field_rect() -> Rect2:
	"""Calculate field rect maintaining aspect ratio"""
	var available_width := size.x - (MARGIN * 2.0)
	var available_height := size.y - (MARGIN * 2.0)

	var field_width: float
	var field_height: float

	# Maintain field aspect ratio (105:68)
	if available_width / available_height > FIELD_ASPECT_RATIO:
		# Height-constrained
		field_height = available_height
		field_width = field_height * FIELD_ASPECT_RATIO
	else:
		# Width-constrained
		field_width = available_width
		field_height = field_width / FIELD_ASPECT_RATIO

	# Center the field
	var offset_x := (size.x - field_width) / 2.0
	var offset_y := (size.y - field_height) / 2.0

	return Rect2(offset_x, offset_y, field_width, field_height)


func _draw_field(rect: Rect2) -> void:
	"""Draw field background (green)"""
	var field_color := Color(0.13, 0.55, 0.13)  # Match ShotMapCanvas
	draw_rect(rect, field_color)

	# Outer border
	var border_color := Color(0.9, 0.9, 0.9)
	draw_rect(rect, border_color, false, 3.0)


func _draw_grid(rect: Rect2) -> void:
	"""Draw 3×3 grid lines (lanes and thirds)"""
	var grid_color := Color(0.9, 0.9, 0.9, 0.5)
	var line_width := 2.0

	# Vertical lines (lanes): at x = 1/3 and 2/3
	for i in [1, 2]:
		var x := rect.position.x + (float(i) / 3.0) * rect.size.x
		draw_line(Vector2(x, rect.position.y), Vector2(x, rect.position.y + rect.size.y), grid_color, line_width)

	# Horizontal lines (thirds): at y = 1/3 and 2/3
	for i in [1, 2]:
		var y := rect.position.y + (float(i) / 3.0) * rect.size.y
		draw_line(Vector2(rect.position.x, y), Vector2(rect.position.x + rect.size.x, y), grid_color, line_width)


func _draw_attack_markers(rect: Rect2) -> void:
	"""Draw attack zone markers sized by attack count"""
	if attack_zones.is_empty():
		return

	for zone in attack_zones:
		if not (zone is Dictionary):
			continue

		var count: int = zone.get("attack_count", 0)
		if count == 0:
			continue

		var percentage: float = zone.get("percentage", 0.0)
		var center_pos: Array = zone.get("center_position", [0.5, 0.5])

		# Convert normalized position to canvas position
		var pos_x: float = center_pos[0] if center_pos.size() > 0 else 0.5
		var pos_y: float = center_pos[1] if center_pos.size() > 1 else 0.5

		var canvas_x := rect.position.x + pos_x * rect.size.x
		var canvas_y := rect.position.y + pos_y * rect.size.y

		# Size based on percentage (min 15px, max 60px)
		var min_radius := 15.0
		var max_radius := 60.0
		var radius := min_radius + (percentage / 100.0) * (max_radius - min_radius)
		radius = clamp(radius, min_radius, max_radius)

		# Color: gradient from blue (low) to red (high)
		var color: Color
		if percentage < 10.0:
			color = Color(0.3, 0.5, 1.0, 0.7)  # Blue
		else:
			if percentage < 15.0:
				color = Color(0.5, 0.7, 1.0, 0.7)  # Light blue
			else:
				if percentage < 20.0:
					color = Color(1.0, 0.7, 0.3, 0.7)  # Orange
				else:
					color = Color(1.0, 0.3, 0.3, 0.7)  # Red

		# Draw circle
		draw_circle(Vector2(canvas_x, canvas_y), radius, color)

		# Draw count text
		var text_color := Color(1.0, 1.0, 1.0)
		var font_size := 16
		var count_text := str(count)

		# Center text on circle
		var text_offset := Vector2(-8, 5)  # Approximate centering
		draw_string(
			ThemeDB.fallback_font,
			Vector2(canvas_x, canvas_y) + text_offset,
			count_text,
			HORIZONTAL_ALIGNMENT_CENTER,
			-1,
			font_size,
			text_color
		)

		# Draw percentage below (if significant)
		if percentage >= 5.0:
			var pct_text := "%.1f%%" % percentage
			var pct_offset := Vector2(-12, radius + 15)
			draw_string(
				ThemeDB.fallback_font,
				Vector2(canvas_x, canvas_y) + pct_offset,
				pct_text,
				HORIZONTAL_ALIGNMENT_CENTER,
				-1,
				12,
				Color(0.9, 0.9, 0.9)
			)
