extends CanvasLayer
## Phase 12: Screen Transition System
## Provides smooth transitions between scenes with fade and slide effects

signal transition_started
signal transition_midpoint
signal transition_completed

@export var default_duration: float = 0.5
@export var default_type: String = "fade"  # fade, slide_left, slide_right, slide_up, slide_down

var _transition_in_progress: bool = false
var _next_scene_path: String = ""
var _tween: Tween

# Overlay for transition effects
var _overlay: ColorRect


func _ready():
	layer = 100  # Ensure transitions are on top
	_create_overlay()


func _create_overlay():
	"""Create black overlay for transitions"""
	_overlay = ColorRect.new()
	_overlay.color = Color.BLACK
	_overlay.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_overlay.modulate.a = 0.0
	_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_overlay)


func change_scene(scene_path: String, transition_type: String = "", duration: float = -1):
	"""
	Change scene with transition effect
	scene_path: Path to the new scene
	transition_type: Type of transition (fade, slide_left, etc.)
	duration: Transition duration in seconds (-1 = use default)
	"""
	if _transition_in_progress:
		push_warning("[ScreenTransition] Transition already in progress, ignoring")
		return

	_transition_in_progress = true
	_next_scene_path = scene_path

	# Use defaults if not specified
	var trans_type = transition_type if transition_type != "" else default_type
	var trans_duration = duration if duration > 0 else default_duration

	match trans_type:
		"fade":
			_transition_fade(trans_duration)
		"slide_left":
			_transition_slide(trans_duration, Vector2(-1, 0))
		"slide_right":
			_transition_slide(trans_duration, Vector2(1, 0))
		"slide_up":
			_transition_slide(trans_duration, Vector2(0, -1))
		"slide_down":
			_transition_slide(trans_duration, Vector2(0, 1))
		_:
			push_warning("[ScreenTransition] Unknown transition type: %s, using fade" % trans_type)
			_transition_fade(trans_duration)


func _transition_fade(duration: float):
	"""Fade transition effect"""
	transition_started.emit()

	if _tween and _tween.is_running():
		_tween.kill()

	_tween = create_tween()
	_tween.set_ease(Tween.EASE_IN_OUT)
	_tween.set_trans(Tween.TRANS_CUBIC)

	# Fade to black
	_tween.tween_property(_overlay, "modulate:a", 1.0, duration / 2.0)
	_tween.tween_callback(_on_transition_midpoint)

	# Change scene
	_tween.tween_callback(_perform_scene_change)

	# Fade from black
	_tween.tween_property(_overlay, "modulate:a", 0.0, duration / 2.0)
	_tween.tween_callback(_on_transition_complete)


func _transition_slide(duration: float, direction: Vector2):
	"""Slide transition effect"""
	transition_started.emit()

	if _tween and _tween.is_running():
		_tween.kill()

	_tween = create_tween()
	_tween.set_ease(Tween.EASE_IN_OUT)
	_tween.set_trans(Tween.TRANS_CUBIC)

	# Get screen size
	var screen_size = get_viewport().get_visible_rect().size

	# Move overlay in from direction
	_overlay.position = direction * screen_size
	_overlay.modulate.a = 1.0

	_tween.tween_property(_overlay, "position", Vector2.ZERO, duration / 2.0)
	_tween.tween_callback(_on_transition_midpoint)

	# Change scene
	_tween.tween_callback(_perform_scene_change)

	# Move overlay out to opposite direction
	_tween.tween_property(_overlay, "position", -direction * screen_size, duration / 2.0)
	_tween.tween_callback(_on_transition_complete)


func _perform_scene_change():
	"""Actually change the scene"""
	if _next_scene_path != "":
		get_tree().change_scene_to_file(_next_scene_path)
	else:
		push_error("[ScreenTransition] No next scene path set!")


func _on_transition_midpoint():
	"""Called at the midpoint of transition"""
	transition_midpoint.emit()


func _on_transition_complete():
	"""Called when transition is complete"""
	_transition_in_progress = false
	_next_scene_path = ""
	_overlay.position = Vector2.ZERO
	_overlay.modulate.a = 0.0
	transition_completed.emit()


func is_transitioning() -> bool:
	"""Check if a transition is currently in progress"""
	return _transition_in_progress


func skip_transition():
	"""Skip to end of current transition immediately"""
	if _tween and _tween.is_running():
		_tween.custom_step(999.0)  # Jump to end
