extends "res://scripts/match_viewer_3d/match_viewer_3d.gd"
class_name MatchViewer3DGLB

## Extended match viewer that supports GLB characters from the LIA pipeline.
## Can use either FBX (animated) or GLB (static/custom animated) player models.

const PLAYER_GLB_SCENE := preload("res://scenes/match_viewer_3d/AnimatedPlayerGLB.tscn")

# Player model mode
enum PlayerModelMode {
	FBX,  # Original FBX with Mixamo animations
	GLB,  # TRELLIS + RigAnything GLB characters
}

@export var player_model_mode: PlayerModelMode = PlayerModelMode.GLB

# GLB character assignment (player_index -> character path)
var _character_assignments: Dictionary = {}

# Character pools for random assignment
var _main_characters: Array[String] = []
var _male_characters: Array[String] = []
var _female_characters: Array[String] = []
var _npc_characters: Array[String] = []


func _ready() -> void:
	_load_character_pools()
	_spawn_players()
	_setup_camera_controller()
	_update_score_display()


## Load available character lists
func _load_character_pools() -> void:
	_main_characters = _scan_character_directory("main_characters/")
	_male_characters = _scan_character_directory("male_characters/") + _scan_character_directory("basic_male/")
	_female_characters = _scan_character_directory("basic_female/")

	# NPC characters are in individual folders (npc_*/mesh.glb)
	_npc_characters = _scan_npc_characters()


func _scan_character_directory(subdir: String) -> Array[String]:
	var characters: Array[String] = []
	var base_path := "res://assets/soccer_players/characters/" + subdir

	var dir := DirAccess.open(base_path)
	if not dir:
		return characters

	dir.list_dir_begin()
	var file_name := dir.get_next()
	while file_name != "":
		if file_name.ends_with(".glb"):
			characters.append(subdir + file_name)
		file_name = dir.get_next()
	dir.list_dir_end()

	return characters


func _scan_npc_characters() -> Array[String]:
	var characters: Array[String] = []
	var base_path := "res://assets/soccer_players/characters/"

	var dir := DirAccess.open(base_path)
	if not dir:
		return characters

	dir.list_dir_begin()
	var folder_name := dir.get_next()
	while folder_name != "":
		# Look for npc_* and plain_* folders
		if folder_name.begins_with("npc_") or folder_name.begins_with("plain_"):
			# Check for mesh.glb or *_rigged.glb inside
			var subdir := DirAccess.open(base_path + folder_name)
			if subdir:
				subdir.list_dir_begin()
				var file := subdir.get_next()
				while file != "":
					if file.ends_with(".glb"):
						characters.append(folder_name + "/" + file)
						break  # Take first GLB found
					file = subdir.get_next()
				subdir.list_dir_end()
		folder_name = dir.get_next()
	dir.list_dir_end()

	return characters


## Override spawn to support both FBX and GLB modes
func _spawn_players() -> void:
	_players.clear()

	if player_model_mode == PlayerModelMode.FBX:
		_spawn_fbx_players()
	else:
		_spawn_glb_players()


func _spawn_fbx_players() -> void:
	# Original FBX spawning (parent class behavior)
	# Home team (11 players)
	for i in range(11):
		var player := PLAYER_SCENE.instantiate() as Node3D
		_home_team.add_child(player)
		if player.has_method("setup"):
			player.setup(i, true)
		if player.has_method("set_team_colors"):
			player.set_team_colors(home_shirt_color, Color.WHITE)
		_players.append(player)

	# Away team (11 players)
	for i in range(11):
		var player := PLAYER_SCENE.instantiate() as Node3D
		_away_team.add_child(player)
		if player.has_method("setup"):
			player.setup(i + 11, false)
		if player.has_method("set_team_colors"):
			player.set_team_colors(away_shirt_color, Color.WHITE)
		_players.append(player)


func _spawn_glb_players() -> void:
	# Home team (11 players)
	for i in range(11):
		var player := PLAYER_GLB_SCENE.instantiate() as Node3D
		_home_team.add_child(player)
		if player.has_method("setup"):
			player.setup(i, true)

		# Load character model
		var char_path := _get_character_for_player(i, true)
		if player.has_method("load_character"):
			player.load_character(char_path)

		if player.has_method("set_team_colors"):
			player.set_team_colors(home_shirt_color, Color.WHITE)

		_players.append(player)

	# Away team (11 players)
	for i in range(11):
		var player := PLAYER_GLB_SCENE.instantiate() as Node3D
		_away_team.add_child(player)
		if player.has_method("setup"):
			player.setup(i + 11, false)

		# Load character model
		var char_path := _get_character_for_player(i + 11, false)
		if player.has_method("load_character"):
			player.load_character(char_path)

		if player.has_method("set_team_colors"):
			player.set_team_colors(away_shirt_color, Color.WHITE)

		_players.append(player)


## Get character path for player (with random fallback)
func _get_character_for_player(player_index: int, is_home: bool) -> String:
	# Check explicit assignment first
	if _character_assignments.has(player_index):
		return _character_assignments[player_index]

	# Random assignment from pools
	var pool: Array[String]
	if player_index == 0 or player_index == 11:
		# Goalkeepers - use main characters
		pool = _main_characters if not _main_characters.is_empty() else _male_characters
	elif is_home:
		# Home team - prefer main/male characters
		pool = _main_characters + _male_characters
	else:
		# Away team - prefer female/npc characters
		pool = _female_characters + _npc_characters

	if pool.is_empty():
		pool = _main_characters + _male_characters + _female_characters

	if pool.is_empty():
		return "main_characters/captain_rigged.glb"  # Default fallback

	return pool[randi() % pool.size()]


## Assign specific character to player
func assign_character(player_index: int, character_path: String) -> void:
	_character_assignments[player_index] = character_path

	# If player already spawned, reload model
	if player_index < _players.size() and _players[player_index]:
		if _players[player_index].has_method("load_character"):
			_players[player_index].load_character(character_path)


## Assign characters from roster data
func assign_characters_from_roster(roster: Array) -> void:
	for i in range(mini(roster.size(), 22)):
		var player_data: Dictionary = roster[i] if roster[i] is Dictionary else {}
		var char_path: String = player_data.get("character_model", "")
		if not char_path.is_empty():
			assign_character(i, char_path)


## Switch between FBX and GLB modes
func set_player_model_mode(mode: PlayerModelMode) -> void:
	if player_model_mode == mode:
		return

	player_model_mode = mode

	# Clear existing players
	for player in _players:
		if player:
			player.queue_free()
	_players.clear()

	# Re-spawn with new mode
	_spawn_players()


## Get available character list
func get_available_characters() -> Dictionary:
	return {
		"main": _main_characters,
		"male": _male_characters,
		"female": _female_characters,
		"npc": _npc_characters,
	}


## Get character count by category
func get_character_counts() -> Dictionary:
	return {
		"main": _main_characters.size(),
		"male": _male_characters.size(),
		"female": _female_characters.size(),
		"npc": _npc_characters.size(),
		"total": _main_characters.size() + _male_characters.size() + _female_characters.size() + _npc_characters.size(),
	}
