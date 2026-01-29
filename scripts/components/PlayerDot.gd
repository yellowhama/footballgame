extends Control

class_name PlayerDot

@onready var background = $Background
@onready var number_label = $NumberLabel

var normalized_pos: Vector2 = Vector2(0.5, 0.5)


func _ready():
	# Center the dot on its position
	pivot_offset = size / 2


func setup(number: String, color: Color, pos: Vector2):
	if number_label:
		number_label.text = number
	if background:
		var style = background.get_theme_stylebox("panel").duplicate()
		style.bg_color = color
		background.add_theme_stylebox_override("panel", style)

	normalized_pos = pos
	update_position_on_field()


func update_position_on_field():
	var parent = get_parent()
	if parent:
		var field_size = parent.size
		position = normalized_pos * field_size - (size / 2)


func _notification(what):
	if what == NOTIFICATION_RESIZED:
		pivot_offset = size / 2
