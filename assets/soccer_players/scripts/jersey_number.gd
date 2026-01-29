extends Node3D
class_name JerseyNumber

## Jersey number display using Label3D (front and back)

@export var number: int = 10:
	set(value):
		number = clampi(value, 0, 99)
		_update_number()

@export var number_color: Color = Color.WHITE:
	set(value):
		number_color = value
		_update_color()

@export var outline_color: Color = Color.BLACK:
	set(value):
		outline_color = value
		_update_color()

@export var font_size: int = 128:
	set(value):
		font_size = value
		_update_number()

# Position offsets
@export var back_offset: Vector3 = Vector3(0, 1.1, -0.12)
@export var front_offset: Vector3 = Vector3(0, 0.9, 0.12)

var _back_label: Label3D
var _front_label: Label3D

func _ready() -> void:
	_create_labels()
	_update_number()
	_update_color()

func _create_labels() -> void:
	# Back number (larger)
	_back_label = Label3D.new()
	_back_label.name = "BackNumber"
	_back_label.pixel_size = 0.002
	_back_label.font_size = font_size
	_back_label.outline_size = 8
	_back_label.position = back_offset
	_back_label.rotation_degrees = Vector3(0, 180, 0)  # Face backward
	_back_label.billboard = BaseMaterial3D.BILLBOARD_DISABLED
	_back_label.no_depth_test = false
	_back_label.double_sided = true
	add_child(_back_label)

	# Front number (smaller)
	_front_label = Label3D.new()
	_front_label.name = "FrontNumber"
	_front_label.pixel_size = 0.0015
	_front_label.font_size = int(font_size * 0.6)
	_front_label.outline_size = 6
	_front_label.position = front_offset
	_front_label.billboard = BaseMaterial3D.BILLBOARD_DISABLED
	_front_label.no_depth_test = false
	_front_label.double_sided = true
	add_child(_front_label)

func _update_number() -> void:
	var num_str = str(number)
	if _back_label:
		_back_label.text = num_str
		_back_label.font_size = font_size
	if _front_label:
		_front_label.text = num_str
		_front_label.font_size = int(font_size * 0.6)

func _update_color() -> void:
	if _back_label:
		_back_label.modulate = number_color
		_back_label.outline_modulate = outline_color
	if _front_label:
		_front_label.modulate = number_color
		_front_label.outline_modulate = outline_color

## Quick setup for common styles
func set_style_white_black() -> void:
	number_color = Color.WHITE
	outline_color = Color.BLACK

func set_style_black_white() -> void:
	number_color = Color.BLACK
	outline_color = Color.WHITE

func set_style_gold() -> void:
	number_color = Color(1.0, 0.84, 0.0)
	outline_color = Color(0.3, 0.2, 0.0)

func set_style_team_color(team_color: Color) -> void:
	# Contrasting number color
	var brightness = (team_color.r + team_color.g + team_color.b) / 3.0
	if brightness > 0.5:
		number_color = Color.BLACK
		outline_color = Color.WHITE
	else:
		number_color = Color.WHITE
		outline_color = Color.BLACK
