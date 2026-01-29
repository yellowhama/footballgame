extends Control
class_name DashboardGrid
## Grid container for dashboard widgets with drag-and-drop
## Phase 13: Extended Features - Dashboard System

signal widget_added(widget: DashboardWidget)
signal widget_removed(widget: DashboardWidget)
signal layout_changed

## Grid configuration
@export var grid_columns: int = 6
@export var grid_rows: int = 8
@export var cell_size: Vector2 = Vector2(180, 180)
@export var cell_spacing: int = 8

## Edit mode
var edit_mode: bool = false

## Widget management
var widgets: Array[DashboardWidget] = []
var grid_cells: Array = []  # 2D array of occupied cells

## Visual debug
var show_grid: bool = false


func _ready():
	custom_minimum_size = Vector2(
		grid_columns * cell_size.x + (grid_columns - 1) * cell_spacing,
		grid_rows * cell_size.y + (grid_rows - 1) * cell_spacing
	)

	_initialize_grid()


func _initialize_grid():
	"""Initialize grid cell occupation tracking"""
	grid_cells.clear()
	for y in range(grid_rows):
		var row = []
		for x in range(grid_columns):
			row.append(false)  # false = empty, true = occupied
		grid_cells.append(row)


func _draw():
	"""Draw grid lines for visual debugging"""
	if not show_grid:
		return

	# Draw grid lines
	for x in range(grid_columns + 1):
		var x_pos = x * (cell_size.x + cell_spacing)
		draw_line(
			Vector2(x_pos, 0), Vector2(x_pos, grid_rows * (cell_size.y + cell_spacing)), Color(0.3, 0.3, 0.3, 0.5), 1.0
		)

	for y in range(grid_rows + 1):
		var y_pos = y * (cell_size.y + cell_spacing)
		draw_line(
			Vector2(0, y_pos),
			Vector2(grid_columns * (cell_size.x + cell_spacing), y_pos),
			Color(0.3, 0.3, 0.3, 0.5),
			1.0
		)


func add_widget(widget: DashboardWidget) -> bool:
	"""Add widget to grid"""
	if not widget or not widget.config:
		return false

	# Check if position is valid and available
	if not _can_place_widget(widget.config.grid_position, widget.config.grid_size):
		# Try to find first available position
		var new_pos = _find_available_position(widget.config.grid_size)
		if new_pos == Vector2i(-1, -1):
			push_warning("[DashboardGrid] No space available for widget")
			return false
		widget.config.grid_position = new_pos

	# Mark cells as occupied
	_occupy_cells(widget.config.grid_position, widget.config.grid_size, true)

	# Add to children
	add_child(widget)
	widgets.append(widget)

	# Set widget properties
	widget.grid_cell_size = cell_size
	widget.set_edit_mode(edit_mode)

	# Position widget
	_position_widget(widget)

	# Connect signals
	widget.widget_moved.connect(_on_widget_moved)
	widget.widget_deleted.connect(_on_widget_deleted)

	# Animate appearance
	widget.animate_appearance()

	widget_added.emit(widget)
	layout_changed.emit()

	return true


func remove_widget(widget: DashboardWidget):
	"""Remove widget from grid"""
	if not widget in widgets:
		return

	# Free occupied cells
	_occupy_cells(widget.config.grid_position, widget.config.grid_size, false)

	# Remove from array
	widgets.erase(widget)

	widget_removed.emit(widget)
	layout_changed.emit()


func clear_widgets():
	"""Remove all widgets"""
	for widget in widgets.duplicate():
		widget.queue_free()

	widgets.clear()
	_initialize_grid()

	layout_changed.emit()


func get_widgets() -> Array[DashboardWidget]:
	"""Get all widgets"""
	return widgets.duplicate()


func get_widget_by_id(widget_id: String) -> DashboardWidget:
	"""Find widget by ID"""
	for widget in widgets:
		if widget.config.widget_id == widget_id:
			return widget
	return null


func set_edit_mode(enabled: bool):
	"""Toggle edit mode for all widgets"""
	edit_mode = enabled
	show_grid = enabled

	for widget in widgets:
		widget.set_edit_mode(enabled)

	queue_redraw()


func _can_place_widget(pos: Vector2i, size: Vector2i) -> bool:
	"""Check if widget can be placed at position"""
	# Check bounds
	if pos.x < 0 or pos.y < 0:
		return false
	if pos.x + size.x > grid_columns or pos.y + size.y > grid_rows:
		return false

	# Check occupation
	for y in range(pos.y, pos.y + size.y):
		for x in range(pos.x, pos.x + size.x):
			if grid_cells[y][x]:
				return false

	return true


func _find_available_position(size: Vector2i) -> Vector2i:
	"""Find first available position for widget of given size"""
	for y in range(grid_rows - size.y + 1):
		for x in range(grid_columns - size.x + 1):
			if _can_place_widget(Vector2i(x, y), size):
				return Vector2i(x, y)

	return Vector2i(-1, -1)


func _occupy_cells(pos: Vector2i, size: Vector2i, occupied: bool):
	"""Mark cells as occupied or free"""
	for y in range(pos.y, min(pos.y + size.y, grid_rows)):
		for x in range(pos.x, min(pos.x + size.x, grid_columns)):
			grid_cells[y][x] = occupied


func _position_widget(widget: DashboardWidget):
	"""Position widget based on grid coordinates"""
	if not widget or not widget.config:
		return

	var pos = widget.config.grid_position
	widget.position = Vector2(pos.x * (cell_size.x + cell_spacing), pos.y * (cell_size.y + cell_spacing))

	# Update size
	widget.custom_minimum_size = Vector2(
		widget.config.grid_size.x * cell_size.x + (widget.config.grid_size.x - 1) * cell_spacing,
		widget.config.grid_size.y * cell_size.y + (widget.config.grid_size.y - 1) * cell_spacing
	)


func _on_widget_moved(widget: DashboardWidget, new_grid_pos: Vector2i):
	"""Widget was moved"""
	var old_pos = widget.config.grid_position
	var size = widget.config.grid_size

	# Free old cells
	_occupy_cells(old_pos, size, false)

	# Check if new position is valid
	if not _can_place_widget(new_grid_pos, size):
		# Snap back to old position
		widget.config.grid_position = old_pos
		_position_widget(widget)
		_occupy_cells(old_pos, size, true)
		return

	# Occupy new cells
	_occupy_cells(new_grid_pos, size, true)

	# Update position
	widget.config.grid_position = new_grid_pos
	_position_widget(widget)

	layout_changed.emit()


func _on_widget_deleted(widget: DashboardWidget):
	"""Widget was deleted"""
	remove_widget(widget)


func save_layout() -> Array:
	"""Save current layout as array of configs"""
	var layout = []
	for widget in widgets:
		layout.append(widget.config.to_dict())
	return layout


func load_layout(layout_data: Array):
	"""Load layout from saved data"""
	# Clear existing widgets
	clear_widgets()

	# Create widgets from configs
	for widget_data in layout_data:
		var config = WidgetConfig.from_dict(widget_data)
		if not config.is_valid():
			push_warning("[DashboardGrid] Invalid widget config: %s" % config.widget_id)
			continue

		var widget: DashboardWidget = null

		match config.widget_type:
			"chart":
				widget = ChartWidget.new(config)

			"stats":
				widget = StatsWidget.new(config)

			"quick_action":
				widget = QuickActionWidget.new(config)

			_:
				push_warning("[DashboardGrid] Unknown widget type: %s" % config.widget_type)
				continue

		if widget:
			add_widget(widget)


func get_grid_info() -> Dictionary:
	"""Get grid information"""
	return {
		"columns": grid_columns,
		"rows": grid_rows,
		"cell_size": {"x": cell_size.x, "y": cell_size.y},
		"spacing": cell_spacing,
		"widget_count": widgets.size()
	}
