extends Control
class_name TimelineField2D

## Simple 2D pitch visualization that renders engine position_data snapshots.

@export var field_length_m: float = 105.0
@export var field_width_m: float = 68.0
@export var padding: float = 24.0
@export var home_color: Color = Color(0.9, 0.2, 0.2, 0.95)
@export var away_color: Color = Color(0.15, 0.35, 0.95, 0.95)
@export var neutral_color: Color = Color(0.95, 0.95, 0.95, 0.85)
@export var ball_color: Color = Color(1, 1, 1, 1)
@export var pitch_color: Color = Color(0.06, 0.38, 0.18)
@export var border_color: Color = Color(0.8, 0.95, 0.78, 0.5)
@export var line_color: Color = Color(1, 1, 1, 0.35)
@export var text_color: Color = Color(1, 1, 1, 0.9)

const PLAYER_RADIUS := 10.0
const BALL_RADIUS := 5.0

var _ball_position: Vector2 = Vector2(field_length_m * 0.5, field_width_m * 0.5)
var _players: Dictionary = {}
var _player_team: Dictionary = {}
var _player_labels: Dictionary = {}
var _player_states: Dictionary = {}
var _has_snapshot: bool = false
var _highlight_player_track_id: int = -1
var _highlight_pitch_x_m: float = -1.0
var _highlight_pitch_line: String = ""


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	focus_mode = Control.FOCUS_NONE
	_has_snapshot = false
	queue_redraw()


func set_rosters(rosters: Dictionary) -> void:
	_player_team.clear()
	_player_labels.clear()
	if rosters is Dictionary and not rosters.is_empty():
		_ingest_roster(rosters.get("home", []), 0)
		_ingest_roster(rosters.get("away", []), 1)
	else:
		# Support direct arrays (legacy)
		if rosters.has("players") and rosters.players is Array:
			_ingest_roster(rosters.players, 0)
	queue_redraw()


func set_snapshot(snapshot: Dictionary) -> void:
	## 두 가지 포맷 지원:
	## 레거시: { ball: {x, y}, players: { id: {position: {x, y}} } }
	## 표준 (StandardSnapshot): { ball: {pos: Vector2}, players: { id: {pos: Vector2} } }
	if not (snapshot is Dictionary):
		return

	# Ball
	if snapshot.has("ball"):
		var ball_data: Variant = snapshot.get("ball")
		if ball_data is Dictionary:
			# StandardSnapshot: ball.pos 또는 레거시: ball.x/y
			if ball_data.has("pos"):
				_ball_position = _extract_vec2(ball_data.get("pos"))
			else:
				_ball_position = _extract_vec2(ball_data)
		else:
			_ball_position = _extract_vec2(ball_data)

	_players.clear()
	_player_states.clear()
	if snapshot.has("players") and snapshot.players is Dictionary:
		for key in snapshot.players:
			var track: Variant = snapshot.players[key]
			if not (track is Dictionary):
				continue
			var player_id := int(key)

			# 위치 추출: 표준(pos) 또는 레거시(position)
			var position_vec: Vector2
			if track.has("pos"):
				position_vec = _extract_vec2(track.get("pos"))
			elif track.has("position"):
				position_vec = _extract_vec2(track.get("position"))
			else:
				position_vec = Vector2(field_length_m * 0.5, field_width_m * 0.5)

			# 상태: action (표준) 또는 state (레거시)
			var state_label: String = str(track.get("action", track.get("state", "")))
			_players[player_id] = position_vec
			_player_states[player_id] = state_label

			# 로스터 정보가 없을 때 스냅샷에서 팀/이름 추출
			if not _player_team.has(player_id) and track.has("team_id"):
				_player_team[player_id] = int(track.get("team_id", 0))
			if not _player_labels.has(player_id):
				var label: String = str(track.get("name", ""))
				if label.is_empty() and track.has("number"):
					label = "#%d" % int(track.get("number", 0))
				if not label.is_empty():
					_player_labels[player_id] = label
	_has_snapshot = true
	queue_redraw()


func clear_snapshot() -> void:
	_players.clear()
	_player_states.clear()
	_ball_position = Vector2(field_length_m * 0.5, field_width_m * 0.5)
	_has_snapshot = false
	clear_highlights()
	queue_redraw()

func clear_highlights() -> void:
	_highlight_player_track_id = -1
	_highlight_pitch_x_m = -1.0
	_highlight_pitch_line = ""
	queue_redraw()


func highlight_player_track_id(track_id: int) -> void:
	_highlight_player_track_id = track_id
	queue_redraw()


func highlight_pitch_x_m(x_m: float) -> void:
	_highlight_pitch_x_m = x_m
	queue_redraw()


func highlight_pitch_line(line_id: String) -> void:
	_highlight_pitch_line = line_id
	queue_redraw()


func set_field_dimensions(length_m: float, width_m: float) -> void:
	field_length_m = max(length_m, 1.0)
	field_width_m = max(width_m, 1.0)
	queue_redraw()


func _ingest_roster(roster_entries: Variant, team_id: int) -> void:
	if not (roster_entries is Array):
		return
	for entry in roster_entries:
		if not (entry is Dictionary):
			continue
		var player_id_variant: Variant = _extract_player_id(entry)
		if player_id_variant == null:
			continue
		var pid := int(player_id_variant)
		_player_team[pid] = team_id
		_player_labels[pid] = _extract_label(entry, team_id, pid)


func _extract_player_id(entry: Dictionary) -> Variant:
	var candidate_keys := ["player_id", "playerId", "id", "engine_id", "engineId", "player_index", "index", "idx"]
	for key in candidate_keys:
		if entry.has(key):
			var value: Variant = entry.get(key)
			if value is int:
				return value
			var as_string := str(value)
			if as_string.is_valid_int():
				return int(as_string)
	return null


func _extract_label(entry: Dictionary, team_id: int, fallback_id: int) -> String:
	var candidate_keys := ["display_name", "short_name", "nickname", "name_ko", "name", "full_name"]
	for key in candidate_keys:
		if entry.has(key):
			var label := str(entry.get(key))
			if label.strip_edges() != "":
				return label
	var number_keys := ["number", "kit_number", "jersey", "shirt_number"]
	for key in number_keys:
		if entry.has(key):
			var number_value := str(entry.get(key))
			if number_value.strip_edges() != "":
				return "#%s" % number_value
	return ("H%02d" if team_id == 0 else "A%02d") % fallback_id


func _extract_vec2(value: Variant) -> Vector2:
	if value is Vector2:
		return value
	if value is Vector3:
		return Vector2(value.x, value.y)
	if value is Array and value.size() >= 2:
		return Vector2(float(value[0]), float(value[1]))
	if value is Dictionary:
		return Vector2(float(value.get("x", 0.0)), float(value.get("y", 0.0)))
	return Vector2(field_length_m * 0.5, field_width_m * 0.5)


func _field_rect() -> Rect2:
	var size: Vector2 = get_size()
	var rect_pos: Vector2 = Vector2(padding, padding)
	var rect_size: Vector2 = size - Vector2(padding, padding) * 2.0
	if rect_size.x <= 0 or rect_size.y <= 0:
		rect_size = Vector2(max(size.x, 1.0), max(size.y, 1.0))
	return Rect2(rect_pos, rect_size)


func _map_to_canvas(field_pos: Vector2) -> Vector2:
	var rect: Rect2 = _field_rect()
	var px: float = clamp(field_pos.x / field_length_m, 0.0, 1.0)
	var py: float = clamp(field_pos.y / field_width_m, 0.0, 1.0)
	return Vector2(rect.position.x + rect.size.x * px, rect.position.y + rect.size.y * py)


func _team_color(team_id: int) -> Color:
	match team_id:
		0:
			return home_color
		1:
			return away_color
		_:
			return neutral_color


func _draw() -> void:
	var rect: Rect2 = _field_rect()
	draw_rect(rect, pitch_color, true)
	draw_rect(rect, border_color, false, 2.0)

	var mid_x: float = rect.position.x + rect.size.x * 0.5
	draw_line(Vector2(mid_x, rect.position.y), Vector2(mid_x, rect.position.y + rect.size.y), line_color, 1.5)
	var center: Vector2 = rect.position + rect.size * 0.5
	var circle_radius: float = min(rect.size.x, rect.size.y) * 0.12
	draw_arc(center, circle_radius, 0.0, TAU, 48, line_color, 1.5)
	_draw_penalty_boxes(rect)

	_draw_players()
	_draw_ball()
	_draw_highlights(rect)

	if not _has_snapshot:
		var font: Font = get_theme_default_font()
		var font_size: int = get_theme_default_font_size()
		if font:
			var message: String = tr("UI_TIMELINE_FIELD_WAITING")
			if message == "UI_TIMELINE_FIELD_WAITING":
				message = "Timeline data not loaded"
			var text_width: float = font.get_string_size(message, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
			var pos: Vector2 = center - Vector2(text_width * 0.5, -font_size * 0.5)
			draw_string(
				font, pos, message, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, _color_with_alpha(text_color, 0.75)
			)


func _draw_penalty_boxes(rect: Rect2) -> void:
	var penalty_length_ratio: float = 16.5 / field_length_m
	var penalty_width_ratio: float = 40.3 / field_width_m
	var penalty_width: float = rect.size.x * penalty_length_ratio
	var penalty_height: float = rect.size.y * penalty_width_ratio
	var offset_y: float = (rect.size.y - penalty_height) * 0.5

	var left_rect: Rect2 = Rect2(rect.position + Vector2(0, offset_y), Vector2(penalty_width, penalty_height))
	var right_rect: Rect2 = Rect2(
		Vector2(rect.position.x + rect.size.x - penalty_width, rect.position.y + offset_y),
		Vector2(penalty_width, penalty_height)
	)
	draw_rect(left_rect, line_color, false, 1.0)
	draw_rect(right_rect, line_color, false, 1.0)


func _draw_players() -> void:
	var font: Font = get_theme_default_font()
	var font_size: int = get_theme_default_font_size()
	for pid in _players.keys():
		var field_pos: Vector2 = _players[pid]
		var canvas_pos: Vector2 = _map_to_canvas(field_pos)
		var color: Color = _team_color(int(_player_team.get(pid, -1)))
		draw_circle(canvas_pos, PLAYER_RADIUS, color)
		draw_circle(canvas_pos, PLAYER_RADIUS, Color(0, 0, 0, 0.45), false, 1.0)

		if font:
			var label_text: String = String(_player_labels.get(pid, "P%d" % pid))
			var label_pos: Vector2 = canvas_pos + Vector2(-PLAYER_RADIUS, PLAYER_RADIUS + float(font_size))
			draw_string(
				font, label_pos, label_text, HORIZONTAL_ALIGNMENT_LEFT, PLAYER_RADIUS * 2.0, font_size, text_color
			)
			var state_text: String = String(_player_states.get(pid, ""))
			if state_text.strip_edges() != "":
				var state_pos: Vector2 = canvas_pos + Vector2(-PLAYER_RADIUS, -PLAYER_RADIUS * 0.5)
				draw_string(
					font,
					state_pos,
					state_text,
					HORIZONTAL_ALIGNMENT_LEFT,
					PLAYER_RADIUS * 2.0,
					max(font_size - 4, 10),
					_color_with_alpha(text_color, 0.7)
				)


func _draw_ball() -> void:
	var canvas_pos: Vector2 = _map_to_canvas(_ball_position)
	draw_circle(canvas_pos, BALL_RADIUS, ball_color)
	draw_circle(canvas_pos, BALL_RADIUS, Color(0, 0, 0, 0.4), false, 1.0)

func _draw_highlights(rect: Rect2) -> void:
	var highlight_color := Color(1.0, 0.92, 0.2, 0.95)

	# Vertical reference line (e.g., offside line)
	if _highlight_pitch_x_m >= 0.0:
		var x_m: float = clampf(_highlight_pitch_x_m, 0.0, field_length_m)
		var top := _map_to_canvas(Vector2(x_m, 0.0))
		var bottom := _map_to_canvas(Vector2(x_m, field_width_m))
		draw_line(Vector2(top.x, rect.position.y), Vector2(bottom.x, rect.position.y + rect.size.y), highlight_color, 2.0)

	# Named pitch lines (best-effort)
	if _highlight_pitch_line != "":
		var id := _highlight_pitch_line.to_lower()
		if id.find("goal") != -1 and (id.find("home") != -1 or id.ends_with("_0") or id.ends_with(":0")):
			var top := _map_to_canvas(Vector2(0.0, 0.0))
			draw_line(Vector2(top.x, rect.position.y), Vector2(top.x, rect.position.y + rect.size.y), highlight_color, 2.0)
		elif id.find("goal") != -1 and (id.find("away") != -1 or id.ends_with("_105") or id.ends_with(":105")):
			var top := _map_to_canvas(Vector2(field_length_m, 0.0))
			draw_line(Vector2(top.x, rect.position.y), Vector2(top.x, rect.position.y + rect.size.y), highlight_color, 2.0)
		elif id.find("touch") != -1 and (id.find("top") != -1 or id.find("upper") != -1):
			var left := _map_to_canvas(Vector2(0.0, 0.0))
			draw_line(Vector2(rect.position.x, left.y), Vector2(rect.position.x + rect.size.x, left.y), highlight_color, 2.0)
		elif id.find("touch") != -1 and (id.find("bottom") != -1 or id.find("lower") != -1):
			var left := _map_to_canvas(Vector2(0.0, field_width_m))
			draw_line(Vector2(rect.position.x, left.y), Vector2(rect.position.x + rect.size.x, left.y), highlight_color, 2.0)

	# Player highlight ring
	if _highlight_player_track_id >= 0 and _players.has(_highlight_player_track_id):
		var field_pos: Vector2 = _players[_highlight_player_track_id]
		var canvas_pos: Vector2 = _map_to_canvas(field_pos)
		draw_arc(canvas_pos, PLAYER_RADIUS + 6.0, 0.0, TAU, 48, highlight_color, 3.0)


func _color_with_alpha(color: Color, alpha: float) -> Color:
	var out := color
	out.a = clamp(alpha, 0.0, 1.0)
	return out
