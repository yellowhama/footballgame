extends PanelContainer
## Phase 12: Tooltip Component
## Displays helpful information on hover

@export var text: String = ""
@export var show_delay: float = 0.5
@export var fade_duration: float = 0.2

var _label: Label
var _show_timer: Timer
var _tween: Tween


func _ready():
	# Setup panel
	set_mouse_filter(MOUSE_FILTER_IGNORE)
	modulate.a = 0.0
	z_index = 1000

	# Create background style
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.1, 0.1, 0.15, 0.95)
	style.border_color = Color(0.4, 0.7, 1.0, 0.8)
	style.set_border_width_all(2)
	style.set_corner_radius_all(6)
	style.content_margin_left = 10
	style.content_margin_right = 10
	style.content_margin_top = 6
	style.content_margin_bottom = 6
	add_theme_stylebox_override("panel", style)

	# Create label
	_label = Label.new()
	_label.text = text
	_label.add_theme_font_size_override("font_size", 14)
	_label.add_theme_color_override("font_color", Color.WHITE)
	add_child(_label)

	# Create timer
	_show_timer = Timer.new()
	_show_timer.one_shot = true
	_show_timer.timeout.connect(_on_show_timer_timeout)
	add_child(_show_timer)

	hide()


func set_text(new_text: String):
	"""Update tooltip text"""
	text = new_text
	if _label:
		_label.text = new_text


func show_tooltip(target_position: Vector2):
	"""Show tooltip at position with delay"""
	_show_timer.start(show_delay)
	global_position = target_position + Vector2(10, -size.y - 10)


func hide_tooltip():
	"""Hide tooltip immediately"""
	_show_timer.stop()
	_fade_out()


func _on_show_timer_timeout():
	"""Show tooltip after delay"""
	_fade_in()


func _fade_in():
	"""Fade in animation"""
	if _tween and _tween.is_running():
		_tween.kill()

	show()
	_tween = create_tween()
	_tween.tween_property(self, "modulate:a", 1.0, fade_duration)


func _fade_out():
	"""Fade out animation"""
	if _tween and _tween.is_running():
		_tween.kill()

	_tween = create_tween()
	_tween.tween_property(self, "modulate:a", 0.0, fade_duration)
	_tween.tween_callback(hide)
