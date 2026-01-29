extends PanelContainer
class_name DashboardWidget
## Base class for dashboard widgets
## Phase 13: Extended Features - Dashboard System

signal widget_selected(widget: DashboardWidget)
signal widget_deselected(widget: DashboardWidget)
signal widget_moved(widget: DashboardWidget, new_position: Vector2i)
signal widget_resized(widget: DashboardWidget, new_size: Vector2i)
signal widget_deleted(widget: DashboardWidget)

## Widget configuration
var config: WidgetConfig

## Drag state
var is_dragging: bool = false
var drag_offset: Vector2 = Vector2.ZERO
var is_selected: bool = false

## Edit mode
var edit_mode: bool = false

## UI references
var title_label: Label
var content_container: MarginContainer
var delete_button: Button
var resize_handle: Control

## Grid cell size (set by DashboardGrid)
var grid_cell_size: Vector2 = Vector2(200, 200)


func _init(widget_config: WidgetConfig = null):
	if widget_config:
		config = widget_config
	else:
		config = WidgetConfig.new()
		config.widget_id = WidgetConfig.generate_id()

	_build_ui()


func _ready():
	_apply_config()
	_update_visuals()


func _build_ui():
	"""Build base widget UI structure"""
	# Main container
	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 8)
	add_child(vbox)

	# Title bar
	var title_bar = HBoxContainer.new()
	vbox.add_child(title_bar)

	title_label = Label.new()
	title_label.text = config.title if config else "Widget"
	title_label.add_theme_font_size_override("font_size", 16)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_bar.add_child(title_label)

	# Delete button (only visible in edit mode)
	delete_button = Button.new()
	delete_button.text = "✖"
	delete_button.custom_minimum_size = Vector2(32, 32)
	delete_button.visible = false
	delete_button.pressed.connect(_on_delete_pressed)
	title_bar.add_child(delete_button)

	# Content container
	content_container = MarginContainer.new()
	content_container.add_theme_constant_override("margin_left", 8)
	content_container.add_theme_constant_override("margin_right", 8)
	content_container.add_theme_constant_override("margin_top", 4)
	content_container.add_theme_constant_override("margin_bottom", 8)
	content_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_child(content_container)

	# Resize handle (only visible in edit mode)
	resize_handle = Control.new()
	resize_handle.custom_minimum_size = Vector2(16, 16)
	resize_handle.visible = false
	resize_handle.position = Vector2(size.x - 16, size.y - 16)
	add_child(resize_handle)


func _apply_config():
	"""Apply configuration to widget"""
	if not config:
		return

	# Update title
	if title_label:
		title_label.text = config.title
		title_label.visible = config.show_title

	# Update style
	var style = StyleBoxFlat.new()
	style.bg_color = config.background_color
	style.border_color = config.border_color
	style.set_border_width_all(2)
	style.set_corner_radius_all(8)
	add_theme_stylebox_override("panel", style)

	# Update size based on grid
	custom_minimum_size = Vector2(config.grid_size.x * grid_cell_size.x, config.grid_size.y * grid_cell_size.y)


func _update_visuals():
	"""Update visual state (selection, edit mode)"""
	var style: StyleBoxFlat = get_theme_stylebox("panel")
	if not style:
		return

	if is_selected:
		style.border_color = Color(1.0, 1.0, 0.0)
		style.set_border_width_all(4)
	elif edit_mode:
		style.border_color = Color(0.7, 0.7, 0.7)
		style.set_border_width_all(3)
	else:
		style.border_color = config.border_color if config else Color(0.4, 0.6, 0.8)
		style.set_border_width_all(2)

	# Update edit mode controls
	if delete_button:
		delete_button.visible = edit_mode
	if resize_handle:
		resize_handle.visible = edit_mode


func _gui_input(event: InputEvent):
	"""Handle input for dragging and selection"""
	if not edit_mode:
		return

	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				# Start dragging
				is_dragging = true
				drag_offset = get_global_mouse_position() - global_position

				# Select this widget
				if not is_selected:
					is_selected = true
					_update_visuals()
					widget_selected.emit(self)
			else:
				# Stop dragging
				if is_dragging:
					is_dragging = false

					# Calculate new grid position
					var new_grid_pos = _screen_to_grid(global_position)
					if new_grid_pos != config.grid_position:
						config.grid_position = new_grid_pos
						config.last_modified = Time.get_datetime_string_from_system()
						widget_moved.emit(self, new_grid_pos)

	elif event is InputEventMouseMotion:
		if is_dragging:
			# Update position during drag
			global_position = get_global_mouse_position() - drag_offset


func _screen_to_grid(screen_pos: Vector2) -> Vector2i:
	"""Convert screen position to grid coordinates"""
	var parent_global = get_parent().global_position if get_parent() else Vector2.ZERO
	var relative_pos = screen_pos - parent_global

	return Vector2i(int(relative_pos.x / grid_cell_size.x), int(relative_pos.y / grid_cell_size.y))


func _grid_to_screen(grid_pos: Vector2i) -> Vector2:
	"""Convert grid coordinates to screen position"""
	return Vector2(grid_pos.x * grid_cell_size.x, grid_pos.y * grid_cell_size.y)


func set_edit_mode(enabled: bool):
	"""Toggle edit mode"""
	edit_mode = enabled
	_update_visuals()


func set_selected(selected: bool):
	"""Set selection state"""
	is_selected = selected
	_update_visuals()

	if selected:
		widget_selected.emit(self)
	else:
		widget_deselected.emit(self)


func get_config() -> WidgetConfig:
	"""Get current widget configuration"""
	return config


func update_config(new_config: WidgetConfig):
	"""Update widget configuration"""
	config = new_config
	_apply_config()
	_update_visuals()


func animate_appearance():
	"""Animate widget appearance (Phase 12 integration)"""
	if not config or not config.animate_on_load:
		return

	match config.animation_type:
		"fade_in":
			modulate.a = 0.0
			var tween = create_tween()
			tween.set_ease(Tween.EASE_OUT)
			tween.set_trans(Tween.TRANS_CUBIC)
			tween.tween_property(self, "modulate:a", 1.0, 0.4)

		"scale_in":
			scale = Vector2(0.5, 0.5)
			modulate.a = 0.0
			var tween = create_tween()
			tween.set_ease(Tween.EASE_OUT)
			tween.set_trans(Tween.TRANS_BACK)
			tween.parallel().tween_property(self, "scale", Vector2(1.0, 1.0), 0.5)
			tween.parallel().tween_property(self, "modulate:a", 1.0, 0.5)

		"slide_in":
			position.y -= 50
			modulate.a = 0.0
			var tween = create_tween()
			tween.set_ease(Tween.EASE_OUT)
			tween.set_trans(Tween.TRANS_CUBIC)
			tween.parallel().tween_property(self, "position:y", position.y + 50, 0.4)
			tween.parallel().tween_property(self, "modulate:a", 1.0, 0.4)


## Virtual methods for subclasses to override


func _populate_content():
	"""Override this to populate widget content"""
	pass


func refresh_data():
	"""Override this to refresh widget data"""
	pass


## Signal handlers


func _on_delete_pressed():
	"""Delete button pressed"""
	widget_deleted.emit(self)
	queue_free()
