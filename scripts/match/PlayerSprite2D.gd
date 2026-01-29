class_name PlayerSprite2D
extends Node2D

@onready var body_sprite: Sprite2D = $Body
@onready var uniform_sprite: Sprite2D = $Body/Uniform
@onready var anim_player: AnimationPlayer = $AnimationPlayer

var _team_color: Color = Color.WHITE


func set_team_color(color: Color) -> void:
	_team_color = color
	if uniform_sprite:
		uniform_sprite.modulate = color


func set_world_position(screen_pos: Vector2, world_pos: Vector3) -> void:
	position = screen_pos
	# TODO: Determine facing direction and animation based on world_pos delta
	pass


func play_animation(anim_name: String) -> void:
	# TODO: Implement animation logic
	pass
