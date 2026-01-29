extends Node
class_name ChartAnimator
## Phase 12: Chart Animation System
## Provides smooth animation for chart rendering

signal animation_completed

@export var duration: float = 0.8
@export var easing_type: Tween.EaseType = Tween.EASE_OUT
@export var transition_type: Tween.TransitionType = Tween.TRANS_CUBIC

var _tween: Tween
var _target_chart: Control
var _animation_progress: float = 0.0


func _ready():
	set_process(false)


func animate_chart(chart: Control, animation_type: String = "draw"):
	"""
	Animate a chart component
	animation_type: "draw", "fade_in", "scale_in"
	"""
	if _tween and _tween.is_running():
		_tween.kill()

	_target_chart = chart
	_animation_progress = 0.0

	match animation_type:
		"draw":
			_animate_draw()
		"fade_in":
			_animate_fade_in()
		"scale_in":
			_animate_scale_in()
		_:
			push_warning("Unknown animation type: %s" % animation_type)


func _animate_draw():
	"""Animate progressive drawing of chart"""
	if not _target_chart:
		return

	# Store original modulate
	var original_modulate = _target_chart.modulate
	_target_chart.modulate = Color(original_modulate.r, original_modulate.g, original_modulate.b, 0.0)

	# Create tween
	_tween = create_tween()
	_tween.set_ease(easing_type)
	_tween.set_trans(transition_type)

	# Animate progress and opacity together
	_tween.parallel().tween_property(self, "_animation_progress", 1.0, duration)
	_tween.parallel().tween_property(_target_chart, "modulate:a", 1.0, duration * 0.3)

	_tween.tween_callback(_on_animation_complete)

	# Update chart on each frame
	set_process(true)


func _animate_fade_in():
	"""Fade in the chart"""
	if not _target_chart:
		return

	var original_modulate = _target_chart.modulate
	_target_chart.modulate = Color(original_modulate.r, original_modulate.g, original_modulate.b, 0.0)

	_tween = create_tween()
	_tween.set_ease(easing_type)
	_tween.set_trans(transition_type)
	_tween.tween_property(_target_chart, "modulate:a", 1.0, duration)
	_tween.tween_callback(_on_animation_complete)


func _animate_scale_in():
	"""Scale in the chart from center"""
	if not _target_chart:
		return

	# Store original values
	var original_scale = _target_chart.scale
	var original_pivot = _target_chart.pivot_offset

	# Set pivot to center
	_target_chart.pivot_offset = _target_chart.size / 2.0
	_target_chart.scale = Vector2.ZERO

	_tween = create_tween()
	_tween.set_ease(easing_type)
	_tween.set_trans(transition_type)
	_tween.tween_property(_target_chart, "scale", original_scale, duration)
	_tween.tween_callback(_on_animation_complete)


func _process(_delta):
	"""Update chart drawing progress"""
	if _target_chart and _target_chart.has_method("set_animation_progress"):
		_target_chart.set_animation_progress(_animation_progress)


func _on_animation_complete():
	set_process(false)
	animation_completed.emit()


func stop():
	"""Stop current animation"""
	if _tween and _tween.is_running():
		_tween.kill()
	set_process(false)
	_animation_progress = 1.0


func get_progress() -> float:
	"""Get current animation progress (0.0 to 1.0)"""
	return _animation_progress
