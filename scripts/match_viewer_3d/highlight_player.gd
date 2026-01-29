extends Node
class_name HighlightPlayer

## Automatic highlight player that sequences through BestMoments.
## Shows key match moments with transitions and camera effects.

signal highlight_started
signal highlight_ended
signal moment_started(moment: Dictionary, index: int, total: int)
signal moment_ended(moment: Dictionary)
signal all_highlights_complete

enum State {
	IDLE,
	PLAYING,
	TRANSITIONING,
	PAUSED,
}

## Configuration
@export var transition_duration: float = 1.5  # Seconds between highlights
@export var fade_color: Color = Color.BLACK
@export var auto_skip_low_priority: bool = false
@export var min_priority_to_show: int = 40  # Skip moments below this priority

## References
var _viewer: MatchViewer3D = null
var _camera_controller: Node = null

## State
var _state: State = State.IDLE
var _moments: Array = []
var _current_index: int = -1
var _moment_timer: float = 0.0
var _transition_timer: float = 0.0
var _is_fading: bool = false

## UI overlay for transitions
var _fade_overlay: ColorRect = null


func _ready() -> void:
	_create_fade_overlay()


func _process(delta: float) -> void:
	match _state:
		State.PLAYING:
			_update_playing(delta)
		State.TRANSITIONING:
			_update_transition(delta)


func _create_fade_overlay() -> void:
	_fade_overlay = ColorRect.new()
	_fade_overlay.name = "FadeOverlay"
	_fade_overlay.color = Color(fade_color.r, fade_color.g, fade_color.b, 0.0)
	_fade_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_fade_overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	_fade_overlay.visible = false


func setup(viewer: MatchViewer3D) -> void:
	_viewer = viewer
	if _viewer:
		_camera_controller = _viewer.get_camera_controller()

		# Add fade overlay to viewer's UI layer
		var ui_layer := _viewer.get_node_or_null("UI") as CanvasLayer
		if ui_layer and _fade_overlay.get_parent() == null:
			ui_layer.add_child(_fade_overlay)


func load_moments(moments: Array) -> void:
	_moments.clear()

	# Filter and sort by priority
	for moment in moments:
		if moment is Dictionary:
			var priority := int(moment.get("priority", 50))
			if priority >= min_priority_to_show or not auto_skip_low_priority:
				_moments.append(moment)

	# Sort by start time
	_moments.sort_custom(_sort_by_time)

	print("[HighlightPlayer] Loaded %d highlights" % _moments.size())


func _sort_by_time(a: Dictionary, b: Dictionary) -> bool:
	return int(a.get("start_time_ms", 0)) < int(b.get("start_time_ms", 0))


## ============================================================================
## Playback Control
## ============================================================================


func play() -> void:
	if _moments.is_empty():
		push_warning("[HighlightPlayer] No moments to play")
		return

	if _state == State.PAUSED:
		_state = State.PLAYING
		if _viewer:
			_viewer.play()
		return

	_current_index = -1
	_state = State.TRANSITIONING
	_transition_timer = 0.0
	highlight_started.emit()
	_start_next_moment()


func pause() -> void:
	if _state == State.PLAYING:
		_state = State.PAUSED
		if _viewer:
			_viewer.pause()


func resume() -> void:
	if _state == State.PAUSED:
		_state = State.PLAYING
		if _viewer:
			_viewer.play()


func stop() -> void:
	_state = State.IDLE
	_current_index = -1
	if _viewer:
		_viewer.stop()
	_hide_fade()
	highlight_ended.emit()


func skip_to_next() -> void:
	if _state == State.PLAYING or _state == State.PAUSED:
		_end_current_moment()
		_start_next_moment()


func skip_to_previous() -> void:
	if _current_index > 0:
		_current_index -= 2  # Will be incremented in _start_next_moment
		_end_current_moment()
		_start_next_moment()


func get_current_moment() -> Dictionary:
	if _current_index >= 0 and _current_index < _moments.size():
		return _moments[_current_index]
	return {}


func get_progress() -> Dictionary:
	return {"current": _current_index + 1, "total": _moments.size(), "state": State.keys()[_state]}


func is_playing() -> bool:
	return _state == State.PLAYING or _state == State.TRANSITIONING


## ============================================================================
## Internal Updates
## ============================================================================


func _update_playing(delta: float) -> void:
	if not _viewer or _current_index < 0:
		return

	var moment := _moments[_current_index] as Dictionary
	var end_time_ms := int(moment.get("end_time_ms", 0))
	var current_time_ms := _viewer.get_current_time_ms()

	# Check if moment ended
	if current_time_ms >= end_time_ms:
		_end_current_moment()
		_start_transition()


func _update_transition(delta: float) -> void:
	_transition_timer += delta

	# Fade effect
	var half_duration := transition_duration / 2.0
	if _transition_timer < half_duration:
		# Fade out
		var alpha := _transition_timer / half_duration
		_set_fade_alpha(alpha)
	elif _transition_timer < transition_duration:
		# Fade in
		var alpha := 1.0 - ((_transition_timer - half_duration) / half_duration)
		_set_fade_alpha(alpha)

		# Start next moment at midpoint
		if not _is_fading:
			_is_fading = true
			_start_next_moment()
	else:
		# Transition complete
		_hide_fade()
		_is_fading = false
		_state = State.PLAYING
		if _viewer:
			_viewer.play()


func _start_transition() -> void:
	_state = State.TRANSITIONING
	_transition_timer = 0.0
	_is_fading = false
	if _viewer:
		_viewer.pause()


func _start_next_moment() -> void:
	_current_index += 1

	if _current_index >= _moments.size():
		_complete_all_highlights()
		return

	var moment := _moments[_current_index] as Dictionary
	var start_time_ms := int(moment.get("start_time_ms", 0))

	# Seek to moment start
	if _viewer:
		_viewer.seek(start_time_ms)

	# Notify camera controller
	if _camera_controller and _camera_controller.has_method("on_best_moment_start"):
		_camera_controller.on_best_moment_start(moment)

	moment_started.emit(moment, _current_index, _moments.size())

	var moment_type := str(moment.get("moment_type", ""))
	var minute := int(moment.get("minute", 0))
	print(
		"[HighlightPlayer] Playing moment %d/%d: %s at %d'" % [_current_index + 1, _moments.size(), moment_type, minute]
	)


func _end_current_moment() -> void:
	if _current_index >= 0 and _current_index < _moments.size():
		moment_ended.emit(_moments[_current_index])


func _complete_all_highlights() -> void:
	_state = State.IDLE
	_hide_fade()
	if _viewer:
		_viewer.stop()
	all_highlights_complete.emit()
	print("[HighlightPlayer] All highlights complete")


## ============================================================================
## Fade Effect
## ============================================================================


func _set_fade_alpha(alpha: float) -> void:
	if _fade_overlay:
		_fade_overlay.visible = true
		_fade_overlay.color.a = clamp(alpha, 0.0, 1.0)


func _hide_fade() -> void:
	if _fade_overlay:
		_fade_overlay.visible = false
		_fade_overlay.color.a = 0.0


## ============================================================================
## Utility
## ============================================================================


func get_moment_label(moment: Dictionary) -> String:
	var moment_type := str(moment.get("moment_type", "")).to_lower()
	var minute := int(moment.get("minute", 0))

	var type_label: String
	match moment_type:
		"goal":
			type_label = "GOAL"
		"save":
			type_label = "SAVE"
		"shot_on_target", "shotontarget":
			type_label = "SHOT"
		"penalty":
			type_label = "PENALTY"
		"red_card", "redcard":
			type_label = "RED CARD"
		"post_hit", "posthit":
			type_label = "POST"
		"bar_hit", "barhit":
			type_label = "CROSSBAR"
		"key_chance", "keychance":
			type_label = "CHANCE"
		_:
			type_label = moment_type.to_upper()

	return "%s %d'" % [type_label, minute]
