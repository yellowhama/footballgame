extends Control
## Phase 12: Focus Indicator
## Visual indicator for keyboard-focused controls

@export var border_width: float = 3.0
@export var border_color: Color = Color(0.4, 0.7, 1.0, 1.0)
@export var corner_radius: float = 8.0
@export var animate: bool = true
@export var pulse_speed: float = 2.0

var _target: Control
var _tween: Tween
var _pulse_phase: float = 0.0


func _ready():
	set_mouse_filter(MOUSE_FILTER_IGNORE)
	set_process(animate)


func _process(delta):
	if not animate or not visible:
		return

	# Pulse effect
	_pulse_phase += delta * pulse_speed
	var alpha = 0.7 + sin(_pulse_phase) * 0.3
	modulate.a = alpha
	queue_redraw()


func _draw():
	if not _target:
		return

	# Get target rect
	var rect = Rect2(Vector2.ZERO, size)

	# Draw border
	var style = StyleBoxFlat.new()
	style.draw_center = false
	style.border_color = border_color
	style.set_border_width_all(int(border_width))
	style.set_corner_radius_all(int(corner_radius))

	draw_style_box(style, rect)


func attach_to(control: Control):
	"""Attach focus indicator to a control"""
	_target = control

	if not _target:
		hide()
		return

	# Match target's rect
	global_position = _target.global_position
	size = _target.size

	# Show with animation
	if animate:
		modulate.a = 0.0
		show()

		if _tween and _tween.is_running():
			_tween.kill()

		_tween = create_tween()
		_tween.tween_property(self, "modulate:a", 1.0, 0.2)
	else:
		modulate.a = 1.0
		show()

	queue_redraw()


func detach():
	"""Detach from current target"""
	if animate:
		if _tween and _tween.is_running():
			_tween.kill()

		_tween = create_tween()
		_tween.tween_property(self, "modulate:a", 0.0, 0.15)
		_tween.tween_callback(hide)
	else:
		hide()

	_target = null
