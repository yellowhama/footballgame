extends Control
class_name TimelineCanvas

## TimelineCanvas
## Renders the match timeline, scrub bar, and event markers.
## Pure rendering component. No data fetching.

# Constants
const MATCH_DURATION_MINUTES: int = 90
const MARGIN_X: float = 40.0
const MARGIN_Y: float = 20.0
const MARKER_RADIUS: float = 6.0
const PLAYHEAD_WIDTH: float = 2.0

# Colors
const COLOR_BG: Color = Color(0.15, 0.15, 0.20, 0.8)
const COLOR_AXIS: Color = Color(0.7, 0.7, 0.7)
const COLOR_TEXT: Color = Color(0.9, 0.9, 0.9)
const COLOR_PLAYHEAD: Color = Color(1.0, 1.0, 0.0, 0.8)
# Event colors
const COLOR_GOAL: Color = Color(1.0, 0.84, 0.0) # Gold
const COLOR_CARD: Color = Color(1.0, 0.3, 0.3) # Red/Warning
const COLOR_DEFAULT: Color = Color(0.2, 0.8, 1.0) # Blue

# State
var _events: Array = [] # Array of MatchEvent
var _current_minute: float = 0.0

# Signals
signal timeline_scrubbed(target_minute: float)

func _ready() -> void:
	custom_minimum_size = Vector2(0, 80) # Minimum height for touch target
	mouse_filter = MouseFilter.MOUSE_FILTER_STOP

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			_handle_input(event.position)
	elif event is InputEventMouseMotion:
		if event.button_mask & MOUSE_BUTTON_LEFT:
			_handle_input(event.position)

func _handle_input(local_pos: Vector2) -> void:
	var rect = _get_timeline_rect()
	var x_clamped = clamp(local_pos.x, rect.position.x, rect.position.x + rect.size.x)
	var ratio = (x_clamped - rect.position.x) / rect.size.x
	var target_minute = ratio * MATCH_DURATION_MINUTES
	
	emit_signal("timeline_scrubbed", target_minute)

# Public API for data injection
func set_events(events: Array) -> void:
	_events = events
	queue_redraw()

func set_playhead(minute: float) -> void:
	_current_minute = clamp(minute, 0.0, float(MATCH_DURATION_MINUTES))
	queue_redraw()

func _draw() -> void:
	var rect = _get_timeline_rect()
	
	_draw_background(rect)
	_draw_axis(rect)
	_draw_events(rect)
	_draw_playhead(rect)

func _get_timeline_rect() -> Rect2:
	var w = size.x - (MARGIN_X * 2)
	var h = size.y - (MARGIN_Y * 2)
	return Rect2(MARGIN_X, MARGIN_Y, w, h)

func _draw_background(rect: Rect2) -> void:
	draw_rect(rect, COLOR_BG)

func _draw_axis(rect: Rect2) -> void:
	var axis_y = rect.position.y + rect.size.y
	
	# Main line
	draw_line(Vector2(rect.position.x, axis_y), Vector2(rect.end.x, axis_y), COLOR_AXIS, 2.0)
	
	# Ticks (0, 15, 30, 45, 60, 75, 90)
	for m in range(0, 91, 15):
		var ratio = float(m) / float(MATCH_DURATION_MINUTES)
		var x = rect.position.x + (rect.size.x * ratio)
		var tick_h = 8.0 if m % 45 == 0 else 5.0
		
		# Tick
		draw_line(Vector2(x, axis_y), Vector2(x, axis_y + tick_h), COLOR_AXIS, 2.0)
		
		# Label
		var label_pos = Vector2(x - 10, axis_y + tick_h + 15)
		draw_string(ThemeDB.fallback_font, label_pos, str(m) + "'", HORIZONTAL_ALIGNMENT_CENTER, -1, 12, COLOR_TEXT)

func _draw_events(rect: Rect2) -> void:
	var y = rect.position.y + (rect.size.y * 0.5)
	
	for evt in _events:
		# Assuming standard MatchEvent structure or Dictionary wrapper
		# Safe access: use get() or property access if strongly typed
		var m = 0.0
		var type = ""
		
		if evt is Dictionary:
			m = float(evt.get("minute", 0))
			type = evt.get("event_type", "generic")
		elif evt is Object and "minute" in evt: # match_result.rs structs
			m = float(evt.minute)
			if "event_type" in evt:
				type = str(evt.event_type)
		else:
			continue
			
		var ratio = m / float(MATCH_DURATION_MINUTES)
		var x = rect.position.x + (rect.size.x * ratio)
		
		var color = COLOR_DEFAULT
		match type.to_lower():
			"goal": color = COLOR_GOAL
			"card", "foul": color = COLOR_CARD
		
		draw_circle(Vector2(x, y), MARKER_RADIUS, color)

func _draw_playhead(rect: Rect2) -> void:
	var ratio = _current_minute / float(MATCH_DURATION_MINUTES)
	var x = rect.position.x + (rect.size.x * ratio)
	
	draw_line(Vector2(x, rect.position.y), Vector2(x, rect.end.y + 10), COLOR_PLAYHEAD, PLAYHEAD_WIDTH)
