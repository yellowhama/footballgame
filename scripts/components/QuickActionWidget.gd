extends DashboardWidget
class_name QuickActionWidget
## Quick action button widget for dashboard
## Phase 13: Extended Features - Dashboard System

signal action_triggered(action_type: String)

var action_button: Button


func _init(widget_config: WidgetConfig = null):
	super._init(widget_config)


func _ready():
	super._ready()
	_populate_content()


func _populate_content():
	"""Create action button"""
	if not config or not content_container:
		return

	# Clear existing content
	for child in content_container.get_children():
		child.queue_free()

	# Create action button
	action_button = Button.new()
	action_button.text = config.button_text if config.button_text != "" else "실행"
	action_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	action_button.size_flags_vertical = Control.SIZE_EXPAND_FILL
	action_button.add_theme_font_size_override("font_size", 18)
	action_button.pressed.connect(_on_action_pressed)
	content_container.add_child(action_button)

	# Set button style based on action type
	_apply_action_style()


func _apply_action_style():
	"""Apply visual style based on action type"""
	if not action_button or not config:
		return

	var style = StyleBoxFlat.new()
	style.set_corner_radius_all(8)

	match config.action_type:
		"filter":
			style.bg_color = Color(0.3, 0.5, 0.8, 0.9)
			action_button.text = "🔍 " + action_button.text

		"export":
			style.bg_color = Color(0.5, 0.7, 0.3, 0.9)
			action_button.text = "📤 " + action_button.text

		"compare":
			style.bg_color = Color(0.8, 0.5, 0.3, 0.9)
			action_button.text = "🔀 " + action_button.text

		"refresh":
			style.bg_color = Color(0.5, 0.3, 0.8, 0.9)
			action_button.text = "🔄 " + action_button.text

		_:
			style.bg_color = Color(0.4, 0.4, 0.4, 0.9)

	action_button.add_theme_stylebox_override("normal", style)

	# Hover style
	var hover_style = style.duplicate()
	hover_style.bg_color = hover_style.bg_color.lightened(0.2)
	action_button.add_theme_stylebox_override("hover", hover_style)

	# Pressed style
	var pressed_style = style.duplicate()
	pressed_style.bg_color = pressed_style.bg_color.darkened(0.2)
	action_button.add_theme_stylebox_override("pressed", pressed_style)


func _on_action_pressed():
	"""Action button pressed"""
	if not config:
		return

	action_triggered.emit(config.action_type)

	# Visual feedback
	var tween = create_tween()
	tween.tween_property(action_button, "scale", Vector2(0.95, 0.95), 0.1)
	tween.tween_property(action_button, "scale", Vector2(1.0, 1.0), 0.1)


func refresh_data():
	"""Quick action widgets don't need data refresh"""
	pass
