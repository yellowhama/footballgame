extends Node
## FormationManager - Core autoload for Rust formation calculations
## Part of the 5-autoload architecture (Constitution v7.0)

signal formation_validated(is_valid: bool, errors: Array)
signal formation_optimized(formation: Dictionary)

var _football_engine: Node = null


func _ready() -> void:
	# Use FootballRustEngine instead of creating our own instance
	if has_node("/root/FootballRustEngine"):
		_football_engine = get_node("/root/FootballRustEngine")
		if _football_engine.has_method("is_ready"):
			if _football_engine.is_ready():
				_initialize()
			else:
				_football_engine.engine_ready.connect(_initialize, CONNECT_ONE_SHOT)


func _initialize() -> void:
	print("[FormationManager] Initialized with Rust engine")


## Validate a formation
## @param formation: String like "4-4-2" or "4-3-3"
## @param _players: Array of player dictionaries (reserved for future Rust implementation)
## @return: Dictionary with validation result
func validate_formation(formation: String, _players: Array) -> Dictionary:
	if not _football_engine or not _football_engine.is_ready():
		return {"valid": false, "errors": ["Formation engine not initialized"]}

	# Basic validation in GDScript for now
	# TODO: Move to Rust when formation validation is implemented in of_core
	var parts = formation.split("-")
	if parts.size() != 3:
		return {"valid": false, "errors": ["Invalid formation format. Expected format: X-X-X"]}

	var total = 0
	for part in parts:
		if not part.is_valid_int():
			return {"valid": false, "errors": ["Invalid number in formation: " + part]}
		total += int(part)

	if total != 10:
		return {"valid": false, "errors": ["Formation must have exactly 10 outfield players (got " + str(total) + ")"]}

	var result = {
		"valid": true,
		"errors": [],
		"formation": formation,
		"defenders": int(parts[0]),
		"midfielders": int(parts[1]),
		"forwards": int(parts[2])
	}

	formation_validated.emit(true, [])
	return result


## Get recommended formations for a set of players
## @param _players: Array of player dictionaries (reserved for future Rust implementation)
## @return: Array of recommended formations
func get_recommended_formations(_players: Array) -> Array:
	# Basic recommendations
	# TODO: Implement in Rust for sophisticated analysis
	var formations = [
		{"formation": "4-4-2", "style": "Balanced", "score": 0.8},
		{"formation": "4-3-3", "style": "Attacking", "score": 0.75},
		{"formation": "3-5-2", "style": "Wide play", "score": 0.7},
		{"formation": "4-2-3-1", "style": "Modern", "score": 0.85},
		{"formation": "5-3-2", "style": "Defensive", "score": 0.65}
	]

	return formations


## Calculate formation effectiveness against opponent
## @param our_formation: String
## @param opponent_formation: String
## @return: Dictionary with effectiveness analysis
func calculate_effectiveness(our_formation: String, opponent_formation: String) -> Dictionary:
	var effectiveness = {
		"overall_score": 0.5,
		"defensive_score": 0.5,
		"midfield_score": 0.5,
		"attacking_score": 0.5,
		"advantages": [],
		"disadvantages": []
	}

	# Parse formations
	var our_parts = _parse_formation(our_formation)
	var opp_parts = _parse_formation(opponent_formation)

	# Defensive analysis
	if our_parts.defenders >= 5:
		effectiveness.defensive_score = 0.75
		effectiveness.advantages.append("Solid 5-back defense")
	elif our_parts.defenders >= 4:
		effectiveness.defensive_score = 0.6

	# Midfield control
	if our_parts.midfielders > opp_parts.midfielders:
		effectiveness.midfield_score = 0.65
		effectiveness.advantages.append("Midfield numerical advantage")
	elif our_parts.midfielders < opp_parts.midfielders:
		effectiveness.midfield_score = 0.4
		effectiveness.disadvantages.append("Outnumbered in midfield")

	# Attacking threat
	if our_parts.forwards >= 3:
		effectiveness.attacking_score = 0.7
		effectiveness.advantages.append("Strong attacking presence")
	elif our_parts.forwards == 1:
		effectiveness.attacking_score = 0.4
		effectiveness.disadvantages.append("Limited attacking options")

	# Specific matchup bonuses
	var matchup = our_formation + "_vs_" + opponent_formation
	match matchup:
		"4-4-2_vs_4-3-3":
			effectiveness.midfield_score += 0.1
			effectiveness.advantages.append("Extra midfielder vs 3-man midfield")
		"4-3-3_vs_3-5-2":
			effectiveness.attacking_score += 0.15
			effectiveness.advantages.append("Wide forwards exploit wing-backs")
		"5-3-2_vs_4-3-3":
			effectiveness.defensive_score += 0.15
			effectiveness.advantages.append("5 defenders nullify wide attacks")
		"3-5-2_vs_4-4-2":
			effectiveness.midfield_score += 0.15
			effectiveness.advantages.append("Midfield dominance")
		"4-2-3-1_vs_4-4-2":
			effectiveness.attacking_score += 0.1
			effectiveness.advantages.append("CAM exploits gap between lines")
		"4-1-4-1_vs_4-3-3":
			effectiveness.defensive_score += 0.1
			effectiveness.advantages.append("CDM provides cover")
		"3-4-3_vs_5-3-2":
			effectiveness.attacking_score += 0.2
			effectiveness.advantages.append("3 forwards vs 3 CBs")
			effectiveness.disadvantages.append("Wing-backs exposed")

	# Clamp scores
	effectiveness.defensive_score = clamp(effectiveness.defensive_score, 0.0, 1.0)
	effectiveness.midfield_score = clamp(effectiveness.midfield_score, 0.0, 1.0)
	effectiveness.attacking_score = clamp(effectiveness.attacking_score, 0.0, 1.0)

	# Calculate overall score
	effectiveness.overall_score = (
		effectiveness.defensive_score * 0.3 + effectiveness.midfield_score * 0.4 + effectiveness.attacking_score * 0.3
	)

	return effectiveness


func _parse_formation(formation: String) -> Dictionary:
	"""Parse formation string into component counts"""
	var parts = formation.split("-")
	var result = {"defenders": 0, "midfielders": 0, "forwards": 0}

	if parts.size() >= 3:
		result.defenders = int(parts[0])
		result.forwards = int(parts[parts.size() - 1])
		for i in range(1, parts.size() - 1):
			result.midfielders += int(parts[i])

	return result


## Get position requirements for a formation
## @param formation: String
## @return: Dictionary with position requirements
func get_position_requirements(formation: String) -> Dictionary:
	var requirements = {"GK": 1, "defenders": [], "midfielders": [], "forwards": []}

	match formation:
		# 4-back formations
		"4-4-2":
			requirements.defenders = ["LB", "CB", "CB", "RB"]
			requirements.midfielders = ["LM", "CM", "CM", "RM"]
			requirements.forwards = ["ST", "ST"]
		"4-3-3":
			requirements.defenders = ["LB", "CB", "CB", "RB"]
			requirements.midfielders = ["CDM", "CM", "CM"]
			requirements.forwards = ["LW", "CF", "RW"]
		"4-2-3-1":
			requirements.defenders = ["LB", "CB", "CB", "RB"]
			requirements.midfielders = ["CDM", "CDM", "LM", "CAM", "RM"]
			requirements.forwards = ["ST"]
		"4-1-4-1":
			requirements.defenders = ["LB", "CB", "CB", "RB"]
			requirements.midfielders = ["CDM", "LM", "CM", "CM", "RM"]
			requirements.forwards = ["ST"]
		"4-3-2-1":
			requirements.defenders = ["LB", "CB", "CB", "RB"]
			requirements.midfielders = ["CM", "CM", "CM", "LW", "RW"]
			requirements.forwards = ["ST"]
		"4-4-1-1":
			requirements.defenders = ["LB", "CB", "CB", "RB"]
			requirements.midfielders = ["LM", "CM", "CM", "RM"]
			requirements.forwards = ["CAM", "ST"]
		"4-5-1":
			requirements.defenders = ["LB", "CB", "CB", "RB"]
			requirements.midfielders = ["LM", "CM", "CDM", "CM", "RM"]
			requirements.forwards = ["ST"]
		# 3-back formations
		"3-5-2":
			requirements.defenders = ["CB", "CB", "CB"]
			requirements.midfielders = ["LWB", "CM", "CDM", "CM", "RWB"]
			requirements.forwards = ["ST", "ST"]
		"3-4-3":
			requirements.defenders = ["CB", "CB", "CB"]
			requirements.midfielders = ["LWB", "CM", "CM", "RWB"]
			requirements.forwards = ["LW", "ST", "RW"]
		"3-4-2-1":
			requirements.defenders = ["CB", "CB", "CB"]
			requirements.midfielders = ["LWB", "CM", "CM", "RWB"]
			requirements.forwards = ["CAM", "CAM", "ST"]
		"3-4-1-2":
			requirements.defenders = ["CB", "CB", "CB"]
			requirements.midfielders = ["LWB", "CM", "CM", "RWB"]
			requirements.forwards = ["CAM", "ST", "ST"]
		# 5-back formations
		"5-3-2":
			requirements.defenders = ["LWB", "CB", "CB", "CB", "RWB"]
			requirements.midfielders = ["CM", "CDM", "CM"]
			requirements.forwards = ["ST", "ST"]
		"5-4-1":
			requirements.defenders = ["LWB", "CB", "CB", "CB", "RWB"]
			requirements.midfielders = ["LM", "CM", "CM", "RM"]
			requirements.forwards = ["ST"]
		"5-2-3":
			requirements.defenders = ["LWB", "CB", "CB", "CB", "RWB"]
			requirements.midfielders = ["CDM", "CDM"]
			requirements.forwards = ["LW", "ST", "RW"]
		_:
			# Generic formation
			var parts = formation.split("-")
			if parts.size() == 3:
				for i in int(parts[0]):
					requirements.defenders.append("DF")
				for i in int(parts[1]):
					requirements.midfielders.append("MF")
				for i in int(parts[2]):
					requirements.forwards.append("FW")

	return requirements


## Optimize player positions for a formation
## @param formation: String
## @param players: Array of player dictionaries
## @return: Dictionary with optimized lineup
func optimize_lineup(formation: String, players: Array) -> Dictionary:
	var requirements = get_position_requirements(formation)
	var lineup = {"formation": formation, "players": [], "score": 0.0}

	# Simple assignment based on position and overall rating
	# TODO: Implement sophisticated optimization in Rust
	var available_players = players.duplicate()

	# Assign GK
	for player in available_players:
		if player.get("position", "") == "GK":
			lineup.players.append(player)
			available_players.erase(player)
			break

	# Assign defenders
	for pos in requirements.defenders:
		var best_player = _find_best_player_for_position(available_players, pos)
		if best_player:
			lineup.players.append(best_player)
			available_players.erase(best_player)

	# Assign midfielders
	for pos in requirements.midfielders:
		var best_player = _find_best_player_for_position(available_players, pos)
		if best_player:
			lineup.players.append(best_player)
			available_players.erase(best_player)

	# Assign forwards
	for pos in requirements.forwards:
		var best_player = _find_best_player_for_position(available_players, pos)
		if best_player:
			lineup.players.append(best_player)
			available_players.erase(best_player)

	# Calculate lineup score
	var total_overall = 0
	for player in lineup.players:
		total_overall += player.get("overall", 50)
	lineup.score = float(total_overall) / float(lineup.players.size()) if lineup.players.size() > 0 else 0.0

	formation_optimized.emit(lineup)
	return lineup


func _find_best_player_for_position(players: Array, position: String) -> Dictionary:
	var best_player = null
	var best_score = -1

	for player in players:
		var player_pos = player.get("position", "")
		var overall = player.get("overall", 50)

		# Exact match
		if player_pos == position:
			if overall > best_score:
				best_player = player
				best_score = overall
		# Generic match (DF for any defender, etc)
		elif _is_compatible_position(player_pos, position):
			var adjusted_score = overall * 0.9  # 10% penalty for non-exact match
			if adjusted_score > best_score:
				best_player = player
				best_score = adjusted_score

	return best_player if best_player else {}


func _is_compatible_position(player_pos: String, required_pos: String) -> bool:
	# Map generic positions
	var defender_positions = ["CB", "LB", "RB", "LWB", "RWB", "DF"]
	var midfielder_positions = ["CDM", "CM", "CAM", "LM", "RM", "MF"]
	var forward_positions = ["ST", "CF", "LW", "RW", "FW"]

	if required_pos in ["DF", "defender"] and player_pos in defender_positions:
		return true
	elif required_pos in ["MF", "midfielder"] and player_pos in midfielder_positions:
		return true
	elif required_pos in ["FW", "forward"] and player_pos in forward_positions:
		return true

	return false
