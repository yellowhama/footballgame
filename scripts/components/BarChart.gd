extends Control
class_name BarChart
## Phase 11: Bar Chart for Statistics Visualization
## Displays categorical data as vertical bars

@export var padding: float = 40.0
@export var show_grid: bool = true
@export var show_labels: bool = true
@export var show_values: bool = true
@export var bar_spacing: float = 10.0

# Data: Array of {"label": String, "value": float, "color": Color (optional)}
var _data: Array = []
var _max_value: float = 100.0

# Animation support (Phase 12)
var _animation_progress: float = 1.0  # 0.0 to 1.0, controls bar height animation

# Default colors
var default_color: Color = Color(0.4, 0.7, 1.0)
var default_colors: Array[Color] = [
	Color(0.5, 0.8, 0.5),  # Green (Win)
	Color(0.7, 0.7, 0.7),  # Gray (Draw)
	Color(0.8, 0.5, 0.5),  # Red (Loss)
]

# Chart area
var _chart_rect: Rect2


func _ready():
	custom_minimum_size = Vector2(400, 300)


func set_data(data: Array, max_value: float = 0.0):
	"""
	Set data to display
	data: Array of Dictionary with structure:
		{
			"label": "Category Name",
			"value": float,
			"color": Color (optional)
		}
	max_value: Maximum value for Y-axis (auto-calculate if 0)
	"""
	_data = data

	# Auto-calculate max value if not provided
	if max_value <= 0.0:
		_max_value = 0.0
		for item in _data:
			_max_value = max(_max_value, item.get("value", 0.0))
		# Round up to nice number
		_max_value = ceil(_max_value / 10.0) * 10.0
		if _max_value == 0.0:
			_max_value = 100.0
	else:
		_max_value = max_value

	# Assign default colors if not provided
	for i in range(_data.size()):
		if not _data[i].has("color"):
			if i < default_colors.size():
				_data[i]["color"] = default_colors[i]
			else:
				_data[i]["color"] = default_color

	queue_redraw()


func _draw():
	if _data.is_empty():
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

	# Draw bars
	_draw_bars()


func _draw_background():
	"""Draw chart background"""
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.15, 0.15, 0.2, 0.8)
	style.set_corner_radius_all(8)
	draw_style_box(style, Rect2(Vector2.ZERO, size))


func _draw_grid():
	"""Draw horizontal grid lines"""
	var grid_color = Color(0.3, 0.3, 0.35, 0.5)

	# Horizontal grid lines (5 lines)
	for i in range(6):
		var y = _chart_rect.position.y + (_chart_rect.size.y * float(i) / 5.0)
		draw_line(
			Vector2(_chart_rect.position.x, y), Vector2(_chart_rect.position.x + _chart_rect.size.x, y), grid_color, 1.0
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

	# Y-axis labels (values)
	if show_labels:
		var font = get_theme_default_font()
		var font_size = 12

		for i in range(6):
			var value = _max_value * float(5 - i) / 5.0
			var label = "%.0f" % value
			var y = _chart_rect.position.y + (_chart_rect.size.y * float(i) / 5.0)
			var label_pos = Vector2(_chart_rect.position.x - 35, y + 5)

			draw_string(font, label_pos, label, HORIZONTAL_ALIGNMENT_RIGHT, -1, font_size, Color(0.7, 0.7, 0.7))


func _draw_bars():
	"""Draw bars and labels"""
	if _data.is_empty():
		return

	var bar_count = _data.size()
	var total_spacing = bar_spacing * (bar_count + 1)
	var available_width = _chart_rect.size.x - total_spacing
	var bar_width = available_width / bar_count

	var font = get_theme_default_font()
	var font_size = 12

	for i in range(bar_count):
		var item = _data[i]
		var value = item.get("value", 0.0)
		var label = item.get("label", "")
		var color = item.get("color", default_color)

		# Calculate bar position and height
		var x = _chart_rect.position.x + bar_spacing + (i * (bar_width + bar_spacing))
		var normalized_value = clamp(value / _max_value, 0.0, 1.0)
		var target_bar_height = _chart_rect.size.y * normalized_value

		# Apply animation to bar height (Phase 12)
		var bar_height = target_bar_height * _animation_progress
		var y = _chart_rect.position.y + _chart_rect.size.y - bar_height

		# Draw bar
		var bar_rect = Rect2(x, y, bar_width, bar_height)
		draw_rect(bar_rect, color)

		# Draw bar border
		draw_rect(bar_rect, color.lightened(0.2), false, 2.0)

		# Draw value on top of bar (if enabled)
		if show_values and bar_height > 20:
			var value_text = "%.0f" % value
			var text_size = font.get_string_size(value_text, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size)
			var text_pos = Vector2(x + bar_width / 2 - text_size.x / 2, y - 5)

			draw_string(font, text_pos, value_text, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size, Color(1, 1, 1))

		# Draw label below X-axis
		if show_labels and label != "":
			var text_size = font.get_string_size(label, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size)
			var text_pos = Vector2(
				x + bar_width / 2 - text_size.x / 2, _chart_rect.position.y + _chart_rect.size.y + 20
			)

			draw_string(font, text_pos, label, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size, Color(0.7, 0.7, 0.7))


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
	_data.clear()
	queue_redraw()


## Phase 12: Animation support


func set_animation_progress(progress: float):
	"""Set animation progress (0.0 to 1.0) for bar height animation"""
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
