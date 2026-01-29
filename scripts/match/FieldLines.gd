class_name FieldLines
extends Node2D

const FIELD_LENGTH := 105.0
const FIELD_WIDTH := 68.0

# Dependencies injected by parent
var project_iso_func: Callable


func _draw() -> void:
	if not project_iso_func.is_valid():
		return

	draw_touchlines()
	draw_goal_lines()
	draw_halfway_line()
	draw_center_circle()
	draw_penalty_boxes()


func draw_touchlines() -> void:
	_draw_field_line(Vector2(0, 0), Vector2(FIELD_LENGTH, 0))
	_draw_field_line(Vector2(0, FIELD_WIDTH), Vector2(FIELD_LENGTH, FIELD_WIDTH))


func draw_goal_lines() -> void:
	_draw_field_line(Vector2(0, 0), Vector2(0, FIELD_WIDTH))
	_draw_field_line(Vector2(FIELD_LENGTH, 0), Vector2(FIELD_LENGTH, FIELD_WIDTH))


func draw_halfway_line() -> void:
	_draw_field_line(Vector2(FIELD_LENGTH / 2.0, 0), Vector2(FIELD_LENGTH / 2.0, FIELD_WIDTH))


func draw_center_circle() -> void:
	# Center circle radius 9.15m
	# In isometric, a circle becomes an ellipse.
	# We can approximate with a polygon or use draw_arc if we can transform it.
	# Simple approach: draw a polygon approximating the ellipse.
	var center = Vector2(FIELD_LENGTH / 2.0, FIELD_WIDTH / 2.0)
	var radius = 9.15
	var points = []
	for i in range(33):
		var angle = i * TAU / 32.0
		var dx = cos(angle) * radius
		var dy = sin(angle) * radius
		points.append(project_iso_func.call(center.x + dx, center.y + dy))

	for i in range(points.size() - 1):
		draw_line(points[i], points[i + 1], Color.WHITE, 2.0)


func draw_penalty_boxes() -> void:
	# Left Box
	# Area: 16.5m from posts, 16.5m deep.
	# Goal posts at y=30.34 and 37.66 (width 7.32) -> center 34.
	# Box width = 16.5 + 7.32 + 16.5 = 40.32.
	# Top y = 34 - 20.16 = 13.84
	# Bottom y = 34 + 20.16 = 54.16
	# Depth x = 16.5

	var box_depth = 16.5
	var box_width_half = 20.16
	var center_y = FIELD_WIDTH / 2.0

	var y_top = center_y - box_width_half
	var y_bottom = center_y + box_width_half

	# Left
	_draw_field_line(Vector2(0, y_top), Vector2(box_depth, y_top))
	_draw_field_line(Vector2(0, y_bottom), Vector2(box_depth, y_bottom))
	_draw_field_line(Vector2(box_depth, y_top), Vector2(box_depth, y_bottom))

	# Right
	var x_right = FIELD_LENGTH
	var x_depth_right = FIELD_LENGTH - box_depth

	_draw_field_line(Vector2(x_right, y_top), Vector2(x_depth_right, y_top))
	_draw_field_line(Vector2(x_right, y_bottom), Vector2(x_depth_right, y_bottom))
	_draw_field_line(Vector2(x_depth_right, y_top), Vector2(x_depth_right, y_bottom))


func _draw_field_line(a: Vector2, b: Vector2, color := Color.WHITE, width := 2.0) -> void:
	var p1 = project_iso_func.call(a.x, a.y, 0.0)
	var p2 = project_iso_func.call(b.x, b.y, 0.0)
	draw_line(p1, p2, color, width)
