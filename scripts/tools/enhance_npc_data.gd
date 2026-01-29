@tool
extends EditorScript
##
## NPC Data Enhancement Script
##
## Adds manager_id, formation, and tactics to stage_teams_safe.json
## Run from: Project -> Tools -> Execute "enhance_npc_data.gd"
##

const INPUT_PATH = "res://data/stage_teams_safe.json"
const OUTPUT_PATH = "res://data/stage_teams_enhanced.json"
const MANAGERS_PATH = "res://data/dummy_managers.json"

# Tactical presets based on team style
const TACTICAL_PRESETS := {
	"Attacking":
	{
		"attacking_intensity": 0.8,
		"defensive_line_height": 0.7,
		"width": 0.75,
		"pressing_trigger": 0.7,
		"tempo": 0.8,
		"directness": 0.6
	},
	"Defensive":
	{
		"attacking_intensity": 0.3,
		"defensive_line_height": 0.3,
		"width": 0.5,
		"pressing_trigger": 0.3,
		"tempo": 0.4,
		"directness": 0.4
	},
	"Balanced":
	{
		"attacking_intensity": 0.5,
		"defensive_line_height": 0.5,
		"width": 0.6,
		"pressing_trigger": 0.5,
		"tempo": 0.5,
		"directness": 0.5
	},
	"Possession":
	{
		"attacking_intensity": 0.5,
		"defensive_line_height": 0.6,
		"width": 0.7,
		"pressing_trigger": 0.6,
		"tempo": 0.3,
		"directness": 0.3
	},
	"Counter":
	{
		"attacking_intensity": 0.7,
		"defensive_line_height": 0.35,
		"width": 0.5,
		"pressing_trigger": 0.4,
		"tempo": 0.9,
		"directness": 0.85
	},
	"Pressing":
	{
		"attacking_intensity": 0.7,
		"defensive_line_height": 0.75,
		"width": 0.65,
		"pressing_trigger": 0.9,
		"tempo": 0.7,
		"directness": 0.6
	},
}

# Formation options based on tactical style
const STYLE_FORMATIONS := {
	"Attacking": ["T433", "T4231", "T352"],
	"Defensive": ["T541", "T532", "T4141"],
	"Balanced": ["T442", "T4231", "T433"],
	"Possession": ["T433", "T4231", "T4312"],
	"Counter": ["T442", "T4141", "T352"],
	"Pressing": ["T4231", "T433", "T442"],
}


func _run():
	print("[NPC Enhancer] Starting NPC data enhancement...")

	# Load managers
	var managers := _load_managers()
	if managers.is_empty():
		push_error("[NPC Enhancer] Failed to load managers")
		return
	print("[NPC Enhancer] Loaded %d managers" % managers.size())

	# Load stage teams
	var teams := _load_teams()
	if teams.is_empty():
		push_error("[NPC Enhancer] Failed to load stage teams")
		return
	print("[NPC Enhancer] Loaded %d teams" % teams.size())

	# Enhance each team
	var enhanced := []
	for i in range(teams.size()):
		var team: Dictionary = teams[i]
		var enhanced_team := _enhance_team(team, managers, i)
		enhanced.append(enhanced_team)

	# Save enhanced data
	_save_teams(enhanced)

	print("[NPC Enhancer] Enhancement complete! Output: %s" % OUTPUT_PATH)
	print("[NPC Enhancer] Teams enhanced: %d" % enhanced.size())


func _load_managers() -> Array:
	var file := FileAccess.open(MANAGERS_PATH, FileAccess.READ)
	if not file:
		return []

	var json := JSON.new()
	var error := json.parse(file.get_as_text())
	file.close()

	if error != OK:
		return []

	var data = json.data
	if data is Dictionary:
		return data.get("managers", [])
	return []


func _load_teams() -> Array:
	var file := FileAccess.open(INPUT_PATH, FileAccess.READ)
	if not file:
		return []

	var json := JSON.new()
	var error := json.parse(file.get_as_text())
	file.close()

	if error != OK:
		return []

	return json.data if json.data is Array else []


func _enhance_team(team: Dictionary, managers: Array, index: int) -> Dictionary:
	var enhanced := team.duplicate(true)
	var avg_ca: float = float(team.get("avg_ca", 50.0))

	# Skip if already has enhancement data
	if team.has("manager_id") and team.has("tactics") and team.has("formation"):
		return enhanced

	# Assign manager based on team strength tier
	if not team.has("manager_id"):
		var manager_id := _select_manager_for_ca(avg_ca, managers, index)
		enhanced["manager_id"] = manager_id

	# Determine tactical style
	var style := _determine_tactical_style(avg_ca, index)

	# Assign formation
	if not team.has("formation"):
		var formations: Array = STYLE_FORMATIONS.get(style, ["T442"])
		enhanced["formation"] = formations[index % formations.size()]

	# Assign tactics
	if not team.has("tactics"):
		var base_tactics: Dictionary = TACTICAL_PRESETS.get(style, TACTICAL_PRESETS["Balanced"])
		enhanced["tactics"] = _add_variation(base_tactics, index)

	# Add tactical style tag
	enhanced["tactical_style"] = style

	return enhanced


func _select_manager_for_ca(avg_ca: float, managers: Array, index: int) -> int:
	"""Select appropriate manager based on team CA"""
	if managers.is_empty():
		return 1

	# Higher CA teams get better managers (higher IDs in this dataset)
	var tier := 0
	if avg_ca >= 140:
		tier = 4  # Elite
	elif avg_ca >= 120:
		tier = 3  # Top
	elif avg_ca >= 100:
		tier = 2  # Good
	elif avg_ca >= 80:
		tier = 1  # Average
	else:
		tier = 0  # Basic

	# Select manager from appropriate tier with some variation
	var manager_count: int = managers.size()
	var tier_size: int = max(1, manager_count / 5)
	var tier_start: int = tier * tier_size
	var tier_end: int = min(tier_start + tier_size, manager_count)

	var manager_index: int = tier_start + (index % (tier_end - tier_start))
	return int(managers[manager_index].get("id", 1))


func _determine_tactical_style(avg_ca: float, index: int) -> String:
	"""Determine tactical style based on CA and index for variety"""
	var styles := TACTICAL_PRESETS.keys()

	# Weight distribution based on CA
	var weights: Array
	if avg_ca >= 140:
		weights = [0.3, 0.05, 0.2, 0.25, 0.1, 0.1]  # Attacking, Defensive, Balanced, Possession, Counter, Pressing
	elif avg_ca >= 120:
		weights = [0.25, 0.1, 0.25, 0.2, 0.1, 0.1]
	elif avg_ca >= 100:
		weights = [0.15, 0.15, 0.35, 0.15, 0.1, 0.1]
	elif avg_ca >= 80:
		weights = [0.1, 0.2, 0.35, 0.1, 0.15, 0.1]
	else:
		weights = [0.05, 0.35, 0.35, 0.05, 0.15, 0.05]  # Lower CA = more defensive/balanced

	# Use index as seed for deterministic selection
	var rand_value := fmod(float(index * 7919) / 10000.0, 1.0)  # Prime number for distribution

	var cumulative := 0.0
	for i in range(styles.size()):
		cumulative += weights[i]
		if rand_value <= cumulative:
			return styles[i]

	return "Balanced"


func _add_variation(base_tactics: Dictionary, index: int) -> Dictionary:
	"""Add small random variation to tactics for uniqueness"""
	var varied := {}

	for key in base_tactics:
		var base_value: float = base_tactics[key]
		# Add variation of +/- 0.1 based on index
		var variation := (fmod(float(index * 31), 20.0) - 10.0) / 100.0
		varied[key] = clampf(base_value + variation, 0.0, 1.0)

	return varied


func _save_teams(teams: Array) -> void:
	var file := FileAccess.open(OUTPUT_PATH, FileAccess.WRITE)
	if not file:
		push_error("[NPC Enhancer] Failed to open output file")
		return

	file.store_string(JSON.stringify(teams, "\t"))
	file.close()
