extends Control
class_name LineChart
## Phase 11: Line Chart for Trend Visualization
## Displays multiple data series as line graphs

@export var padding: float = 40.0
@export var show_grid: bool = true
@export var show_labels: bool = true
@export var show_points: bool = true
@export var line_width: float = 3.0

# Data series: Array of {"name": String, "points": PackedVector2Array, "color": Color}
var _series: Array = []
var _x_labels: Array = []
var _y_labels: Array = []

# Animation support (Phase 12)
var _animation_progress: float = 1.0  # 0.0 to 1.0, controls drawing progress

# Chart area
var _chart_rect: Rect2

# Colors for multiple series
var default_colors: Array[Color] = [
	Color(0.4, 0.7, 1.0),  # Blue
	Color(1.0, 0.5, 0.3),  # Orange
	Color(0.5, 1.0, 0.5),  # Green
	Color(1.0, 0.3, 0.5),  # Pink
	Color(0.7, 0.5, 1.0),  # Purple
	Color(1.0, 1.0, 0.3),  # Yellow
]


func _ready():
	custom_minimum_size = Vector2(400, 300)


func set_series(series: Array):
	"""
	Set data series to display
	series: Array of Dictionary with structure:
		{
			"name": "Series Name",
			"points": PackedVector2Array (normalized 0-1),
			"color": Color (optional)
		}
	"""
	_series = series

	# Assign colors if not provided
	for i in range(_series.size()):
		if not _series[i].has("color"):
			_series[i]["color"] = default_colors[i % default_colors.size()]

	queue_redraw()


func set_x_labels(labels: Array):
	"""Set X-axis labels"""
	_x_labels = labels
	queue_redraw()


func set_y_labels(labels: Array):
	"""Set Y-axis labels"""
	_y_labels = labels
	queue_redraw()


func _draw():
	if _series.is_empty():
		_draw_empty_state()
		return

	# Calculate chart area
	_chart_rect = Rect2(Vector2(padding, padding), size - Vector2(padding * 2, padding * 2))

	# Draw background
	_draw_background()

	# Draw grid
	if show_grid:
		_draw_grid()

	# Draw axes
	_draw_axes()

	# Draw series
	for series_data in _series:
		_draw_series(series_data)

	# Draw legend
	_draw_legend()


func _draw_background():
	"""Draw chart background"""
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.15, 0.15, 0.2, 0.8)
	style.set_corner_radius_all(8)
	draw_style_box(style, Rect2(Vector2.ZERO, size))


func _draw_grid():
	"""Draw grid lines"""
	var grid_color = Color(0.3, 0.3, 0.35, 0.5)

	# Horizontal grid lines (5 lines)
	for i in range(6):
		var y = _chart_rect.position.y + (_chart_rect.size.y * i / 5)
		draw_line(
			Vector2(_chart_rect.position.x, y), Vector2(_chart_rect.position.x + _chart_rect.size.x, y), grid_color, 1.0
		)

	# Vertical grid lines (5 lines)
	for i in range(6):
		var x = _chart_rect.position.x + (_chart_rect.size.x * i / 5)
		draw_line(
			Vector2(x, _chart_rect.position.y), Vector2(x, _chart_rect.position.y + _chart_rect.size.y), grid_color, 1.0
		)


func _draw_axes():
	"""Draw X and Y axes"""
	var axis_color = Color(0.7, 0.7, 0.7, 1.0)

	# Y-axis (left)
	draw_line(_chart_rect.position, _chart_rect.position + Vector2(0, _chart_rect.size.y), axis_color, 2.0)

	# X-axis (bottom)
	draw_line(
		_chart_rect.position + Vector2(0, _chart_rect.size.y), _chart_rect.position + _chart_rect.size, axis_color, 2.0
	)

	# Y-axis labels
	if show_labels and not _y_labels.is_empty():
		var font = get_theme_default_font()
		var font_size = 12

		for i in range(_y_labels.size()):
			var label = str(_y_labels[i])
			var y = (
				_chart_rect.position.y + _chart_rect.size.y - (_chart_rect.size.y * i / max(1, _y_labels.size() - 1))
			)
			var label_pos = Vector2(_chart_rect.position.x - 35, y + 5)

			draw_string(font, label_pos, label, HORIZONTAL_ALIGNMENT_RIGHT, -1, font_size, Color(0.7, 0.7, 0.7))

	# X-axis labels
	if show_labels and not _x_labels.is_empty():
		var font = get_theme_default_font()
		var font_size = 12

		for i in range(_x_labels.size()):
			var label = str(_x_labels[i])
			var x = _chart_rect.position.x + (_chart_rect.size.x * i / max(1, _x_labels.size() - 1))
			var label_pos = Vector2(x - 10, _chart_rect.position.y + _chart_rect.size.y + 20)

			draw_string(font, label_pos, label, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(0.7, 0.7, 0.7))


func _draw_series(series_data: Dictionary):
	"""Draw a single data series"""
	var points: PackedVector2Array = series_data.get("points", PackedVector2Array())
	var color: Color = series_data.get("color", Color.WHITE)

	if points.size() < 2:
		return

	# Convert normalized points to screen coordinates
	var screen_points = PackedVector2Array()
	for point in points:
		var x = _chart_rect.position.x + (point.x * _chart_rect.size.x)
		var y = _chart_rect.position.y + _chart_rect.size.y - (point.y * _chart_rect.size.y)
		screen_points.append(Vector2(x, y))

	# Calculate how many points to draw based on animation progress
	var points_to_draw = max(2, int(screen_points.size() * _animation_progress))

	# Draw line segments up to animation progress
	for i in range(min(points_to_draw - 1, screen_points.size() - 1)):
		var start = screen_points[i]
		var end = screen_points[i + 1]

		# Interpolate last segment if needed
		if i == points_to_draw - 2 and _animation_progress < 1.0:
			var segment_progress = (screen_points.size() * _animation_progress) - i - 1
			end = start.lerp(end, segment_progress)

		draw_line(start, end, color, line_width)

	# Draw points up to animation progress
	if show_points:
		for i in range(min(points_to_draw, screen_points.size())):
			var point = screen_points[i]
			draw_circle(point, 4.0, color)
			draw_circle(point, 2.5, Color.WHITE)


func _draw_legend():
	"""Draw series legend"""
	if _series.is_empty():
		return

	var font = get_theme_default_font()
	var font_size = 14
	var legend_x = _chart_rect.position.x + _chart_rect.size.x - 150
	var legend_y = _chart_rect.position.y + 10
	var line_height = 25

	for i in range(_series.size()):
		var series_data = _series[i]
		var series_name = series_data.get("name", "Series %d" % (i + 1))
		var color = series_data.get("color", Color.WHITE)

		var y = legend_y + (i * line_height)

		# Color indicator
		draw_circle(Vector2(legend_x, y + 8), 5.0, color)

		# Series name
		draw_string(
			font, Vector2(legend_x + 15, y + 12), series_name, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(0.9, 0.9, 0.9)
		)


func _draw_empty_state():
	"""Draw empty state message"""
	var font = get_theme_default_font()
	var font_size = 18
	var text = "No data to display"

	var text_size = font.get_string_size(text, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size)
	var text_pos = (size - text_size) / 2

	draw_string(font, text_pos, text, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size, Color(0.5, 0.5, 0.5))


func clear():
	"""Clear all data"""
	_series.clear()
	_x_labels.clear()
	_y_labels.clear()
	queue_redraw()


## Phase 12: Animation support


func set_animation_progress(progress: float):
	"""Set animation progress (0.0 to 1.0) for progressive drawing"""
	_animation_progress = clamp(progress, 0.0, 1.0)
	queue_redraw()


func reset_animation():
	"""Reset animation to start"""
	_animation_progress = 0.0
	queue_redraw()


func complete_animation():
	"""Complete animation instantly"""
	_animation_progress = 1.0
	queue_redraw()
