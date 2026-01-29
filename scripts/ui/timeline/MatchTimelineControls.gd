extends Control
class_name MatchTimelineControls

## UI component for controlling MatchTimelineController position playback.
## Emits high level signals (play/pause/seek/speed) for parent screens.
##
## P0: Unskippable Key Moments - Nintendo Pocket FC inspired
## - Goal, Save, Foul events cannot be skipped immediately
## - Skip button has delay during key moments (1-2 seconds)

const _TimelineMarkersScript := preload("res://scripts/ui/timeline/TimelineMarkers.gd")

signal play_requested
signal pause_requested
signal stop_requested
signal seek_requested(time_ms: int)
signal speed_changed(multiplier: float)
signal marker_selected(marker: Dictionary)
signal key_moment_started(event_type: String)
signal key_moment_ended
signal highlight_filter_changed(filter_level: String)
signal exit_requested

const SPEED_OPTIONS := [
	{"label": "0.5x", "value": 0.5},
	{"label": "1x", "value": 1.0},
	{"label": "2x", "value": 2.0},
	{"label": "4x", "value": 4.0}
]

## P6: Highlight filter levels (2025-12-09)
const HIGHLIGHT_OPTIONS := [
	{"label": "Full", "value": "full"},  ## All events
	{"label": "Key", "value": "simple"},  ## Goals + major incidents only
	{"label": "None", "value": "none"},  ## No timeline markers
	{"label": "My Player", "value": "my_player"}  ## Player-specific events
]

## P0: Key moment event types that cannot be skipped immediately
const KEY_MOMENT_EVENTS := ["goal", "save", "penalty", "red_card", "injury"]
## P0: Duration in seconds before skip becomes available
const KEY_MOMENT_LOCK_DURATION := 1.5

## Node references - assigned at runtime to avoid @onready errors when nodes are missing
var _play_button: Button = null
var _stop_button: Button = null
var _exit_button: Button = null
var _speed_selector: OptionButton = null
var _current_time_label: Label = null
var _duration_label: Label = null
var _time_slider: HSlider = null
var _marker_layer = null  # Type: _TimelineMarkersScript
var _highlight_selector: OptionButton = null  ## P6: Highlight filter

var _total_duration_ms: int = 0
var _suppress_slider_signal: bool = false
var _current_speed: float = 1.0
var _marker_data: Array = []
var _current_highlight: String = "full"  ## P6: Current highlight level
## P0: Key moment lock state
var _key_moment_active: bool = false
var _key_moment_timer: float = 0.0
var _current_key_event: String = ""


func _ready() -> void:
	# Assign nodes at runtime using find_child to avoid errors when nodes are missing
	_play_button = find_child("PlayPauseButton", true, false) as Button
	_stop_button = find_child("StopButton", true, false) as Button
	_exit_button = find_child("ExitButton", true, false) as Button
	_speed_selector = find_child("SpeedSelector", true, false) as OptionButton
	_current_time_label = find_child("CurrentTimeLabel", true, false) as Label
	_duration_label = find_child("DurationLabel", true, false) as Label
	_time_slider = find_child("TimeSlider", true, false) as HSlider
	_marker_layer = find_child("TimelineMarkers", true, false)
	_highlight_selector = find_child("HighlightSelector", true, false) as OptionButton

	# Guard against missing nodes (e.g., when used in a session viewer without full UI)
	if not _play_button or not _time_slider:
		push_warning("[MatchTimelineControls] Required nodes missing, controls disabled")
		return
	_configure_buttons()
	_configure_speed_selector()
	_configure_highlight_selector()
	_time_slider.value_changed.connect(_on_slider_changed)
	if _marker_layer:
		_marker_layer.marker_clicked.connect(_on_marker_clicked)
	_update_time_labels(0)


func set_total_duration(duration_ms: int) -> void:
	_total_duration_ms = max(duration_ms, 0)
	if not _time_slider:
		return
	_time_slider.max_value = float(_total_duration_ms)
	_time_slider.editable = _total_duration_ms > 0
	if _duration_label:
		_duration_label.text = _format_time(_total_duration_ms) if _total_duration_ms > 0 else "--:--"
	if _marker_layer:
		_marker_layer.set_duration(_total_duration_ms)
		_marker_layer.set_markers(_marker_data, _total_duration_ms)
	if _time_slider.value > _total_duration_ms:
		update_timestamp(0)


func update_timestamp(timestamp_ms: int) -> void:
	if not _time_slider:
		return
	var clamped: int = clamp(timestamp_ms, 0, _total_duration_ms if _total_duration_ms > 0 else timestamp_ms)
	_suppress_slider_signal = true
	_time_slider.value = float(clamped)
	_suppress_slider_signal = false
	_update_time_labels(clamped)


func set_controls_enabled(enabled: bool) -> void:
	if _play_button:
		_play_button.disabled = not enabled
	if _stop_button:
		_stop_button.disabled = not enabled
	if _speed_selector:
		_speed_selector.disabled = not enabled
	if _time_slider:
		_time_slider.editable = enabled and _total_duration_ms > 0


func set_play_state(is_playing: bool) -> void:
	if not _play_button:
		return
	_play_button.set_pressed_no_signal(is_playing)
	_play_button.text = _play_label(is_playing)


func set_speed_value(multiplier: float) -> void:
	if not _speed_selector:
		return
	for idx in range(_speed_selector.item_count):
		var value: float = float(_speed_selector.get_item_metadata(idx))
		if is_equal_approx(value, multiplier):
			_speed_selector.select(idx)
			_current_speed = multiplier
			return


func set_event_markers(markers: Array) -> void:
	_marker_data.clear()
	for marker in markers:
		if marker is Dictionary and marker.has("time_ms"):
			_marker_data.append(marker.duplicate(true))
	_apply_marker_filter()


## P6: Apply highlight filter to markers and update timeline (2025-12-09)
func _apply_marker_filter() -> void:
	if not _marker_layer:
		return
	var filtered_markers := filter_events(_marker_data)
	_marker_layer.set_markers(filtered_markers, _total_duration_ms)


func _configure_buttons() -> void:
	if not _play_button or not _stop_button:
		return
	_play_button.toggle_mode = true
	_play_button.toggled.connect(_on_play_toggled)
	_play_button.text = _play_label(false)
	_stop_button.pressed.connect(_on_stop_pressed)
	if _exit_button:
		_exit_button.pressed.connect(_on_exit_pressed)


func _configure_speed_selector() -> void:
	if not _speed_selector:
		return
	_speed_selector.clear()
	var default_index: int = 0
	for idx in range(SPEED_OPTIONS.size()):
		var option: Dictionary = SPEED_OPTIONS[idx]
		_speed_selector.add_item(option["label"], idx)
		_speed_selector.set_item_metadata(idx, option["value"])
		if is_equal_approx(option["value"], 1.0):
			default_index = idx
	_speed_selector.select(default_index)
	_current_speed = float(SPEED_OPTIONS[default_index]["value"])
	_speed_selector.item_selected.connect(_on_speed_selected)


## P6: Configure highlight filter selector (2025-12-09)
func _configure_highlight_selector() -> void:
	if not _highlight_selector:
		return
	_highlight_selector.clear()
	var default_index: int = 0
	for idx in range(HIGHLIGHT_OPTIONS.size()):
		var option: Dictionary = HIGHLIGHT_OPTIONS[idx]
		_highlight_selector.add_item(option["label"], idx)
		_highlight_selector.set_item_metadata(idx, option["value"])
		if option["value"] == "full":
			default_index = idx
	_highlight_selector.select(default_index)
	_current_highlight = str(HIGHLIGHT_OPTIONS[default_index]["value"])
	_highlight_selector.item_selected.connect(_on_highlight_selected)


func _on_play_toggled(button_pressed: bool) -> void:
	if _play_button:
		_play_button.text = _play_label(button_pressed)
	if button_pressed:
		play_requested.emit()
	else:
		pause_requested.emit()


func _on_stop_pressed() -> void:
	if _play_button:
		_play_button.set_pressed_no_signal(false)
		_play_button.text = _play_label(false)
	stop_requested.emit()


func _on_exit_pressed() -> void:
	exit_requested.emit()


func _on_speed_selected(index: int) -> void:
	if not _speed_selector:
		return
	if index < 0 or index >= _speed_selector.item_count:
		return
	_current_speed = float(_speed_selector.get_item_metadata(index))
	speed_changed.emit(_current_speed)


## P6: Highlight filter selection handler (2025-12-09)
func _on_highlight_selected(index: int) -> void:
	if not _highlight_selector:
		return
	if index < 0 or index >= _highlight_selector.item_count:
		return
	_current_highlight = str(_highlight_selector.get_item_metadata(index))
	_apply_marker_filter()  # 필터 변경 시 마커 업데이트
	highlight_filter_changed.emit(_current_highlight)


func _on_slider_changed(value: float) -> void:
	if _suppress_slider_signal:
		return
	var time_ms: int = int(round(value))
	_update_time_labels(time_ms)
	seek_requested.emit(time_ms)


func _on_marker_clicked(time_ms: int, marker: Dictionary) -> void:
	update_timestamp(time_ms)
	seek_requested.emit(time_ms)
	marker_selected.emit(marker)


func _update_time_labels(timestamp_ms: int) -> void:
	if _current_time_label:
		_current_time_label.text = _format_time(timestamp_ms)
	if _duration_label and _total_duration_ms == 0:
		_duration_label.text = "--:--"


func _play_label(is_playing: bool) -> String:
	var key := "UI_TIMELINE_ACTION_PAUSE" if is_playing else "UI_TIMELINE_ACTION_PLAY"
	var fallback := "Pause" if is_playing else "Play"
	return _translate_or_default(key, fallback)


func _format_time(timestamp_ms: int) -> String:
	var total_seconds := int(floor(float(timestamp_ms) / 1000.0))
	var minutes := total_seconds / 60
	var seconds := total_seconds % 60
	return "%02d:%02d" % [minutes, seconds]


func _translate_or_default(key: String, fallback: String) -> String:
	var localized := tr(key)
	return localized if localized != key else fallback


## ============================================================================
## P0: Key Moment System - Unskippable Events (Nintendo Pocket FC inspired)
## ============================================================================


func _process(delta: float) -> void:
	if _key_moment_active:
		_key_moment_timer -= delta
		if _key_moment_timer <= 0.0:
			_end_key_moment()


## Check if event type is a key moment that should lock controls
func is_key_moment_event(event_type: String) -> bool:
	return event_type.to_lower() in KEY_MOMENT_EVENTS


## Start a key moment lock - disables skip/seek temporarily
func start_key_moment(event_type: String) -> void:
	if not is_key_moment_event(event_type):
		return
	_key_moment_active = true
	_key_moment_timer = KEY_MOMENT_LOCK_DURATION
	_current_key_event = event_type
	_update_controls_for_key_moment(true)
	key_moment_started.emit(event_type)


## End key moment lock
func _end_key_moment() -> void:
	_key_moment_active = false
	_key_moment_timer = 0.0
	var _ended_event := _current_key_event
	_current_key_event = ""
	_update_controls_for_key_moment(false)
	key_moment_ended.emit()


## Update UI controls based on key moment state
func _update_controls_for_key_moment(locked: bool) -> void:
	# During key moment: disable slider, speed change, and modify button appearance
	if _time_slider:
		_time_slider.editable = not locked and _total_duration_ms > 0
		_time_slider.modulate = Color(0.5, 0.5, 0.5, 0.8) if locked else Color.WHITE
	if _speed_selector:
		_speed_selector.disabled = locked
		_speed_selector.modulate = Color(0.5, 0.5, 0.5, 0.8) if locked else Color.WHITE


## Check if currently in key moment lock
func is_key_moment_locked() -> bool:
	return _key_moment_active


## Get remaining lock time
func get_key_moment_remaining() -> float:
	return max(_key_moment_timer, 0.0)


## Get current key event type
func get_current_key_event() -> String:
	return _current_key_event


## Trigger key moment from marker data
func trigger_key_moment_from_marker(marker: Dictionary) -> void:
	var event_type := str(marker.get("event_type", "")).to_lower()
	if is_key_moment_event(event_type):
		start_key_moment(event_type)


## ============================================================================
## P6: Highlight Filter System (2025-12-09)
## ============================================================================


## Get current highlight filter level
func get_current_highlight_level() -> String:
	return _current_highlight


## Set highlight filter level programmatically
func set_highlight_level(level: String) -> void:
	if not _highlight_selector:
		return
	for idx in range(_highlight_selector.item_count):
		var value: String = str(_highlight_selector.get_item_metadata(idx))
		if value == level:
			_highlight_selector.select(idx)
			_current_highlight = level
			return


## Check if event should be shown based on current filter
## event_data should have "importance" field: "major", "important", or "all"
func should_show_event(event_data: Dictionary) -> bool:
	var importance := str(event_data.get("importance", "all"))
	match _current_highlight:
		"none":
			return false
		"simple":
			# Only show major events (goals, red cards, penalties)
			return importance == "major"
		"my_player":
			# Show major + important events
			return importance in ["major", "important"]
		"full", _:
			# Show all events
			return true


## Filter events array based on current highlight level
func filter_events(events: Array) -> Array:
	if _current_highlight == "none":
		return []
	if _current_highlight == "full":
		return events
	var filtered: Array = []
	for event in events:
		if event is Dictionary and should_show_event(event):
			filtered.append(event)
	return filtered
