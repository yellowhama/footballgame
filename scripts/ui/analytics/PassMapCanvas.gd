extends Control
class_name PassMapCanvas

const PERF_TAG := "PassMapCanvas"
const MAX_REDRAW_HZ := 10
const MAX_NODES := 200
const MAX_EDGES := 500

const FIELD_SIZE := Vector2(105.0, 68.0)
const BACKGROUND_COLOR := Color(0.08, 0.12, 0.18, 1.0)
const PITCH_COLOR := Color(0.09, 0.4, 0.17, 1.0)
const LINE_COLOR := Color(0.9, 0.9, 0.9, 0.85)

const RedrawThrottle = preload("res://scripts/ui/_perf/RedrawThrottle.gd")

var _team_color: Color = Color(0.25, 0.55, 0.95, 0.9)
var _nodes: Dictionary = {}
var _edges: Array = []
var _redraw_throttle = RedrawThrottle.new()


func clear() -> void:
	_team_color = Color(0.25, 0.55, 0.95, 0.9)
	_nodes = {}
	_edges = []
	_request_redraw()


func set_pass_data(data: Dictionary) -> void:
	if not (data is Dictionary):
		clear()
		return

	_team_color = data.get("team_color", _team_color)
	_nodes = _cap_nodes(data.get("nodes", {}))
	_edges = _cap_edges(data.get("edges", []))
	_request_redraw()


func _cap_nodes(nodes: Variant) -> Dictionary:
	if not (nodes is Dictionary):
		return {}

	var dict: Dictionary = nodes
	if dict.size() <= MAX_NODES:
		return dict

	var keys: Array = dict.keys()
	keys.sort()
	var out: Dictionary = {}
	for i in range(min(keys.size(), MAX_NODES)):
		var key: Variant = keys[i]
		out[key] = dict.get(key)

	return out


func _cap_edges(edges: Variant) -> Array:
	if not (edges is Array):
		return []

	var arr: Array = edges
	if arr.size() <= MAX_EDGES:
		return arr

	return arr.slice(0, MAX_EDGES)


func _request_redraw() -> void:
	_redraw_throttle.request_redraw(self, MAX_REDRAW_HZ, func():
		queue_redraw()
	)


func _draw() -> void:
	var rect := Rect2(Vector2.ZERO, size)

	# Background + pitch base
	draw_rect(rect, BACKGROUND_COLOR)
	draw_rect(rect, PITCH_COLOR)

	_draw_pitch_lines(rect)

	if _nodes.is_empty():
		return

	# Draw edges first (under nodes)
	for edge in _edges:
		if not (edge is Dictionary):
			continue
		_draw_edge(rect, edge)

	# Draw nodes on top
	for node_id in _nodes.keys():
		var node: Dictionary = _nodes.get(node_id, {})
		_draw_node(rect, node)


func _draw_pitch_lines(rect: Rect2) -> void:
	# Outer boundaries + halfway line (minimal)
	draw_rect(rect, LINE_COLOR, false, 2.0)
	draw_line(
		Vector2(rect.position.x + rect.size.x * 0.5, rect.position.y),
		Vector2(rect.position.x + rect.size.x * 0.5, rect.position.y + rect.size.y),
		LINE_COLOR,
		2.0
	)


func _draw_edge(rect: Rect2, edge: Dictionary) -> void:
	var from_id: String = str(edge.get("from", ""))
	var to_id: String = str(edge.get("to", ""))
	if from_id == "" or to_id == "":
		return

	var from_pos_pitch: Variant = null
	var to_pos_pitch: Variant = null

	if _nodes.has(from_id):
		from_pos_pitch = (_nodes[from_id] as Dictionary).get("avg", null)
	if _nodes.has(to_id):
		to_pos_pitch = (_nodes[to_id] as Dictionary).get("avg", null)

	if from_pos_pitch == null:
		from_pos_pitch = edge.get("avg_start", null)
	if to_pos_pitch == null:
		to_pos_pitch = edge.get("avg_end", null)

	if not (from_pos_pitch is Vector2) or not (to_pos_pitch is Vector2):
		return

	var from_pos: Vector2 = _to_canvas(rect, from_pos_pitch)
	var to_pos: Vector2 = _to_canvas(rect, to_pos_pitch)

	var count: int = int(edge.get("count", 0))
	var success: int = int(edge.get("success", 0))
	var failure: int = int(edge.get("failure", 0))
	var total: int = max(success + failure, 1)
	var success_rate: float = float(success) / float(total)

	var width: float = clamp(1.0 + float(count) * 0.15, 1.0, 6.0)
	var alpha: float = clamp(0.15 + success_rate * 0.55, 0.15, 0.7)
	var color := Color(_team_color.r, _team_color.g, _team_color.b, alpha)

	draw_line(from_pos, to_pos, color, width)


func _draw_node(rect: Rect2, node: Dictionary) -> void:
	var pos_pitch: Variant = node.get("avg", null)
	if not (pos_pitch is Vector2):
		return
	var touches: int = int(node.get("touches", 0))

	var pos: Vector2 = _to_canvas(rect, pos_pitch)
	var radius: float = clamp(4.0 + sqrt(float(touches)) * 0.75, 4.0, 14.0)

	var fill := Color(_team_color.r, _team_color.g, _team_color.b, 0.85)
	draw_circle(pos, radius, fill)
	draw_arc(pos, radius, 0.0, TAU, 48, LINE_COLOR, 1.5, true)


func _to_canvas(rect: Rect2, pitch_pos: Vector2) -> Vector2:
	var x := rect.position.x + (pitch_pos.x / FIELD_SIZE.x) * rect.size.x
	var y := rect.position.y + (pitch_pos.y / FIELD_SIZE.y) * rect.size.y
	return Vector2(x, y)
