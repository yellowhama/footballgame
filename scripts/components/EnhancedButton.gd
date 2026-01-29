class_name EnhancedButton
extends Button
## Phase 12: Enhanced Button
## Button with hover effects, press animations, and tooltips

@export var hover_scale: float = 1.05
@export var press_scale: float = 0.95
@export var animation_duration: float = 0.15

var _original_scale: Vector2
var _tween: Tween


func _ready():
	_original_scale = scale

	# Connect signals
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)
	button_down.connect(_on_button_down)
	button_up.connect(_on_button_up)


func _exit_tree():
	# Nothing to clean up currently
	pass


func _on_mouse_entered():
	"""Handle mouse hover"""
	_animate_scale(_original_scale * hover_scale)


func _on_mouse_exited():
	"""Handle mouse exit"""
	_animate_scale(_original_scale)


func _on_button_down():
	"""Handle button press"""
	_animate_scale(_original_scale * press_scale)


func _on_button_up():
	"""Handle button release"""
	if is_hovered():
		_animate_scale(_original_scale * hover_scale)
	else:
		_animate_scale(_original_scale)


func _animate_scale(target_scale: Vector2):
	"""Animate scale change"""
	if _tween and _tween.is_running():
		_tween.kill()

	_tween = create_tween()
	_tween.set_ease(Tween.EASE_OUT)
	_tween.set_trans(Tween.TRANS_BACK)
	_tween.tween_property(self, "scale", target_scale, animation_duration)
