class_name PlaybackController
extends Node
## Playback control system for match timeline
##
## Provides interactive controls for timeline viewing including:
## - Time scale adjustment (0.5x, 1x, 2x, 4x)
## - Pause/resume functionality
## - Event skipping
## - Keyboard shortcuts
##
## Usage:
##   var controller = PlaybackController.new()
##   controller.set_time_scale(2.0)  # 2x speed
##   controller.pause()
##   controller.resume()

#region Signals
## Emitted when playback speed changes
signal time_scale_changed(new_scale: float)

## Emitted when pause state changes
signal pause_state_changed(is_paused: bool)

## Emitted when skip is requested
signal skip_requested

## Emitted when timeline restart is requested
signal restart_requested
#endregion

#region Properties
## Current playback speed multiplier
## Valid values: 0.5 (slow), 1.0 (normal), 2.0 (fast), 4.0 (very fast)
var time_scale: float = 1.0:
	set(value):
		var clamped = clampf(value, 0.1, 10.0)  # Allow wider range
		if time_scale != clamped:
			time_scale = clamped
			time_scale_changed.emit(time_scale)
			print("⏩ Playback speed: %.1fx" % time_scale)

## Current pause state
var is_paused: bool = false:
	set(value):
		if is_paused != value:
			is_paused = value
			pause_state_changed.emit(is_paused)
			if is_paused:
				print("⏸️ Playback paused")
			else:
				print("▶️ Playback resumed")

## Flag for event skip request
var skip_requested_flag: bool = false

## Enable/disable keyboard shortcuts
var keyboard_enabled: bool = true
#endregion


#region Lifecycle
func _ready() -> void:
	set_process_input(true)
	print("🎮 PlaybackController initialized (Speed: %.1fx)" % time_scale)


func _input(event: InputEvent) -> void:
	if not keyboard_enabled:
		return

	if event is InputEventKey and event.pressed and not event.echo:
		match event.keycode:
			KEY_SPACE:
				toggle_pause()
			KEY_RIGHT:
				skip_current_event()
			KEY_LEFT:
				# Optional: Previous event (not implemented yet)
				pass
			KEY_R:
				restart_timeline()
			KEY_1:
				set_time_scale(0.5)
			KEY_2:
				set_time_scale(1.0)
			KEY_3:
				set_time_scale(2.0)
			KEY_4:
				set_time_scale(4.0)
			KEY_EQUAL, KEY_KP_ADD:  # + key
				increase_speed()
			KEY_MINUS, KEY_KP_SUBTRACT:  # - key
				decrease_speed()


#endregion


#region Public API
## Set playback speed
## @param scale: Speed multiplier (0.1 to 10.0, recommended: 0.5, 1.0, 2.0, 4.0)
func set_time_scale(scale: float) -> void:
	time_scale = scale


## Pause timeline
func pause() -> void:
	is_paused = true


## Resume timeline
func resume() -> void:
	is_paused = false


## Toggle pause state
func toggle_pause() -> void:
	is_paused = not is_paused


## Skip current event
func skip_current_event() -> void:
	skip_requested_flag = true
	skip_requested.emit()
	print("⏭️ Skip event requested")


## Check if skip was requested and reset flag
func consume_skip_request() -> bool:
	var was_requested = skip_requested_flag
	skip_requested_flag = false
	return was_requested


## Restart timeline from beginning
func restart_timeline() -> void:
	restart_requested.emit()
	print("🔄 Timeline restart requested")


## Increase playback speed (double current speed)
func increase_speed() -> void:
	var new_scale = time_scale * 2.0
	set_time_scale(min(new_scale, 10.0))


## Decrease playback speed (half current speed)
func decrease_speed() -> void:
	var new_scale = time_scale / 2.0
	set_time_scale(max(new_scale, 0.1))


## Get adjusted delta time based on current state
## Returns 0 if paused, otherwise delta * time_scale
func get_adjusted_delta(delta: float) -> float:
	if is_paused:
		return 0.0
	return delta * time_scale


## Enable keyboard shortcuts
func enable_keyboard() -> void:
	keyboard_enabled = true


## Disable keyboard shortcuts
func disable_keyboard() -> void:
	keyboard_enabled = false


#endregion


#region Highlight Mode Support
## Filter events for highlight mode
## @param events: All timeline events
## @param filter_types: Array of event kinds to include (e.g., ["Shot", "Foul"])
## @return: Filtered array of events
func filter_highlights(events: Array, filter_types: Array[String]) -> Array:
	var highlights = []
	for event in events:
		if event is Dictionary and event.has("kind"):
			var kind = event["kind"]
			# Check if this event matches any filter type
			if kind in filter_types:
				highlights.append(event)
			# Special handling for Shot events (check outcome)
			elif kind == "Shot" and "Shot" in filter_types:
				if event.has("outcome") and event["outcome"] == "Goal":
					highlights.append(event)

	print("🎯 Filtered %d highlights from %d events" % [highlights.size(), events.size()])
	return highlights


## Get events with temporal context
## @param event_index: Index of highlight event
## @param all_events: Complete event list
## @param context_seconds: Seconds of context before highlight (default: 3.0)
## @return: Events including context window
func get_events_with_context(event_index: int, all_events: Array, context_seconds: float = 3.0) -> Array:
	if event_index < 0 or event_index >= all_events.size():
		return []

	var target_event = all_events[event_index]
	var target_time = target_event.get("base", {}).get("t", 0.0)
	var context_start_time = target_time - context_seconds

	var context_events = []
	for i in range(all_events.size()):
		var event = all_events[i]
		var event_time = event.get("base", {}).get("t", 0.0)

		if event_time >= context_start_time and i <= event_index:
			context_events.append(event)

	return context_events


#endregion


#region Status Display
## Get current status string for UI display
func get_status_string() -> String:
	var status = ""
	status += "Speed: %.1fx" % time_scale
	status += " | "
	status += "Paused" if is_paused else "Playing"
	return status


## Get keyboard shortcuts help text
func get_keyboard_help() -> String:
	return """
Keyboard Controls:
  SPACE - Pause/Resume
  RIGHT - Skip Event
  R - Restart
  1/2/3/4 - Speed (0.5x/1x/2x/4x)
  +/- - Increase/Decrease Speed
"""
#endregion
