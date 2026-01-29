extends Control
class_name TimelineMarkers

signal marker_clicked(time_ms: int, marker: Dictionary)

@export var home_color: Color = Color(0.9, 0.2, 0.2, 0.95)
@export var away_color: Color = Color(0.15, 0.35, 0.95, 0.95)
@export var neutral_color: Color = Color(0.8, 0.8, 0.8, 0.9)
@export var goal_color: Color = Color(1.0, 0.84, 0.2, 0.95)

# BestMoment 전용 색상 (0103_VIEWER_3D_INTEGRATION_PLAN.md 기준)
@export var penalty_color: Color = Color(1.0, 0.27, 0.27, 0.95)  # #FF4444
@export var red_card_color: Color = Color(0.55, 0.0, 0.0, 0.95)  # #8B0000
@export var save_color: Color = Color(0.0, 1.0, 0.0, 0.95)  # #00FF00
@export var shot_on_target_color: Color = Color(1.0, 0.65, 0.0, 0.95)  # #FFA500
@export var post_hit_color: Color = Color(1.0, 1.0, 0.0, 0.95)  # #FFFF00
@export var key_chance_color: Color = Color(0.6, 0.4, 1.0, 0.95)  # Purple

var _markers: Array = []
var _duration_ms: int = 0
var _marker_positions: Array = []


func _ready() -> void:
	mouse_filter = MOUSE_FILTER_PASS
	resized.connect(func(): queue_redraw())


func clear_markers() -> void:
	_markers.clear()
	_marker_positions.clear()
	queue_redraw()


func set_duration(duration_ms: int) -> void:
	_duration_ms = max(duration_ms, 0)
	queue_redraw()


func set_markers(markers: Array, duration_ms: int = -1) -> void:
	_markers.clear()
	_marker_positions.clear()
	if duration_ms >= 0:
		_duration_ms = max(duration_ms, 0)
	for marker in markers:
		if marker is Dictionary and marker.has("time_ms"):
			var normalized: Dictionary = marker.duplicate(true)
			normalized["time_ms"] = max(0, int(round(normalized.get("time_ms", 0))))
			_markers.append(normalized)
	queue_redraw()


func _draw() -> void:
	_marker_positions.clear()
	if _markers.is_empty() or _duration_ms <= 0:
		return

	var rect: Rect2 = Rect2(Vector2.ZERO, get_size())
	var base_y: float = rect.position.y + rect.size.y * 0.25
	var height: float = rect.size.y * 0.65

	for marker in _markers:
		var time_ms: float = float(marker.get("time_ms", 0))
		var ratio: float = clamp(time_ms / float(_duration_ms), 0.0, 1.0)
		var x: float = rect.position.x + ratio * rect.size.x
		var color: Color = _color_for_marker(marker)
		var top: Vector2 = Vector2(x, base_y)
		var bottom: Vector2 = Vector2(x, base_y + height)
		draw_line(top, bottom, color, 2.0)
		var triangle: PackedVector2Array = PackedVector2Array(
			[Vector2(x, base_y), Vector2(x - 4.5, base_y - 8.0), Vector2(x + 4.5, base_y - 8.0)]
		)
		draw_colored_polygon(triangle, color)
		_marker_positions.append({"x": x, "marker": marker})


func _gui_input(event: InputEvent) -> void:
	if not (event is InputEventMouseButton):
		return
	var mouse_event: InputEventMouseButton = event as InputEventMouseButton
	if mouse_event.button_index != MOUSE_BUTTON_LEFT or not mouse_event.pressed:
		return
	if _marker_positions.is_empty():
		return
	var entry: Variant = _find_marker_near(mouse_event.position.x)
	if entry:
		var marker: Dictionary = entry.get("marker", {})
		marker_clicked.emit(int(marker.get("time_ms", 0)), marker)
		accept_event()


func _get_tooltip(at_position: Vector2) -> String:
	if _marker_positions.is_empty():
		return ""
	var entry: Variant = _find_marker_near(at_position.x, 14.0)
	if not entry:
		return ""
	var marker: Dictionary = entry.get("marker", {})
	var label := str(marker.get("label", marker.get("event_type", "event")))
	var team_id := int(marker.get("team_id", -1))
	var minute: float = float(marker.get("time_ms", 0)) / 60000.0
	var team_label: String = ""
	match team_id:
		0:
			team_label = "HOME"
		1:
			team_label = "AWAY"
	var time_text := "%.1f'" % minute if minute > 0.0 else "%d ms" % int(marker.get("time_ms", 0))
	return "%s %s - %s" % [team_label, time_text, label] if team_label != "" else "%s - %s" % [time_text, label]


func _find_marker_near(local_x: float, tolerance: float = 12.0) -> Variant:
	var closest: Variant = null
	var closest_dist: float = tolerance
	for entry in _marker_positions:
		if not (entry is Dictionary):
			continue
		var entry_dict: Dictionary = entry
		var dist: float = abs(float(entry_dict.get("x", 0.0)) - local_x)
		if dist <= closest_dist:
			closest = entry_dict
			closest_dist = dist
	return closest


func _color_for_marker(marker: Dictionary) -> Color:
	# BestMoment moment_type 우선 체크 (Rust API에서 전달)
	var moment_type := str(marker.get("moment_type", "")).to_lower()
	if moment_type != "":
		match moment_type:
			"goal":
				return goal_color
			"penalty":
				return penalty_color
			"redcard", "red_card":
				return red_card_color
			"save":
				return save_color
			"shotontarget", "shot_on_target":
				return shot_on_target_color
			"posthit", "post_hit", "barhit", "bar_hit":
				return post_hit_color
			"keychance", "key_chance":
				return key_chance_color

	# 기존 event_type 폴백
	var event_type := str(marker.get("event_type", "")).to_lower()
	if event_type == "goal" or event_type == "scored":
		return goal_color
	if event_type == "save":
		return save_color
	if event_type == "shot" or event_type == "shot_on_target":
		return shot_on_target_color
	if event_type == "penalty":
		return penalty_color
	if event_type == "red_card":
		return red_card_color

	var team_id := int(marker.get("team_id", -1))
	match team_id:
		0:
			return home_color
		1:
			return away_color
		_:
			return neutral_color
