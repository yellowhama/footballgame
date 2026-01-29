extends Control

class_name FieldView

@export var player_dot_scene: PackedScene

@onready var home_players_container = $HomePlayers
@onready var away_players_container = $AwayPlayers

# Standard formations (Normalized coordinates: x=0-1 (width), y=0-1 (height))
# Assuming Home attacks Right (Left side of screen) and Away attacks Left (Right side)
# But usually in 2D view, Home is Left, Away is Right.
# Coordinates: (0,0) is Top-Left.
# Home Goal is at x=0, Away Goal at x=1.

const FORMATIONS = {
	"4-4-2":
	{
		"home":
		[
			Vector2(0.05, 0.5),  # GK
			Vector2(0.2, 0.2),
			Vector2(0.2, 0.4),
			Vector2(0.2, 0.6),
			Vector2(0.2, 0.8),  # DEF
			Vector2(0.4, 0.2),
			Vector2(0.4, 0.4),
			Vector2(0.4, 0.6),
			Vector2(0.4, 0.8),  # MID
			Vector2(0.6, 0.4),
			Vector2(0.6, 0.6)  # FWD
		],
		"away":
		[
			Vector2(0.95, 0.5),  # GK
			Vector2(0.8, 0.2),
			Vector2(0.8, 0.4),
			Vector2(0.8, 0.6),
			Vector2(0.8, 0.8),  # DEF
			Vector2(0.6, 0.2),
			Vector2(0.6, 0.4),
			Vector2(0.6, 0.6),
			Vector2(0.6, 0.8),  # MID
			Vector2(0.4, 0.4),
			Vector2(0.4, 0.6)  # FWD
		]
	},
	"4-3-3":
	{
		"home":
		[
			Vector2(0.05, 0.5),  # GK
			Vector2(0.2, 0.2),
			Vector2(0.2, 0.4),
			Vector2(0.2, 0.6),
			Vector2(0.2, 0.8),  # DEF
			Vector2(0.35, 0.5),
			Vector2(0.4, 0.3),
			Vector2(0.4, 0.7),  # MID
			Vector2(0.6, 0.5),
			Vector2(0.6, 0.2),
			Vector2(0.6, 0.8)  # FWD
		],
		"away":
		[
			Vector2(0.95, 0.5),  # GK
			Vector2(0.8, 0.2),
			Vector2(0.8, 0.4),
			Vector2(0.8, 0.6),
			Vector2(0.8, 0.8),  # DEF
			Vector2(0.65, 0.5),
			Vector2(0.6, 0.3),
			Vector2(0.6, 0.7),  # MID
			Vector2(0.4, 0.5),
			Vector2(0.4, 0.2),
			Vector2(0.4, 0.8)  # FWD
		]
	}
}


func _ready():
	# Clear existing placeholders if any
	if home_players_container:
		for child in home_players_container.get_children():
			child.queue_free()
	if away_players_container:
		for child in away_players_container.get_children():
			child.queue_free()

	# Default setup for testing
	setup_formation("4-4-2", "4-4-2")


func setup_formation(home_fmt: String, away_fmt: String):
	if not player_dot_scene:
		print("PlayerDot scene not assigned!")
		return

	_spawn_team(home_fmt, "home", home_players_container, Color(0.2, 0.2, 0.8))
	_spawn_team(away_fmt, "away", away_players_container, Color(0.8, 0.2, 0.2))


func _spawn_team(fmt_name: String, side: String, container: Control, color: Color):
	if not container:
		return

	# Clear existing
	for child in container.get_children():
		child.queue_free()

	var positions = []
	if FORMATIONS.has(fmt_name):
		positions = FORMATIONS[fmt_name].get(side, [])
	else:
		positions = FORMATIONS["4-4-2"].get(side, [])  # Fallback

	for i in range(positions.size()):
		var pos = positions[i]
		var dot = player_dot_scene.instantiate()
		container.add_child(dot)
		dot.setup(str(i + 1), color, pos)


func _notification(what):
	if what == NOTIFICATION_RESIZED:
		_update_all_positions()


func _update_all_positions():
	if home_players_container:
		for child in home_players_container.get_children():
			if child.has_method("update_position_on_field"):
				child.update_position_on_field()
	if away_players_container:
		for child in away_players_container.get_children():
			if child.has_method("update_position_on_field"):
				child.update_position_on_field()
