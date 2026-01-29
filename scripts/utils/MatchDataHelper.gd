## MatchDataHelper - Dictionary-based match data utilities
## Part of TASK_12: API Migration to DECISION_00 compliance
## Autoload singleton (must extend Node for autoload compatibility)
extends Node

## Schema version for compatibility tracking
## v1 = players array with inline data, v2 = roster with UID strings
const SCHEMA_VERSION: int = 1


## Build team Dictionary from game data
static func build_team_dict(team_data) -> Dictionary:
	"""
	Convert game team data to engine Dictionary format

	Args:
		team_data: TeamSetup or similar object with roster data

	Returns:
		Dictionary with team configuration
	"""
	if team_data == null:
		push_error("[MatchDataHelper] Team data is null")
		return {}

	var players = []

	# Handle TeamSetup.starters array
	if team_data.has("starters") and team_data.starters is Array:
		for starter in team_data.starters:
			players.append(player_to_dict(starter))
	elif team_data.has("players") and team_data.players is Array:
		# Handle Dictionary format
		for player in team_data.players:
			players.append(player_to_dict(player))

	return {
		"name": team_data.get("name", "Team"),
		"players": players,
		"formation": team_data.get("formation", "4-4-2"),
		"tactics": team_data.get("tactics", {}),
		"instructions": team_data.get("player_instructions", {})
	}


## Convert player object to Dictionary
static func player_to_dict(player) -> Dictionary:
	"""
	Convert player object (MatchPlayer, etc.) to Dictionary

	Args:
		player: Player object or Dictionary

	Returns:
		Dictionary with player attributes
	"""
	if player == null:
		return {}

	# If already a Dictionary, pass through
	if player is Dictionary:
		var d: Dictionary = (player as Dictionary).duplicate(true)
		if not d.has("condition") or d.get("condition") == null:
			d["condition"] = 3
		return d

	# Extract from MatchPlayer or similar
	return {
		"name": player.get("name", "Unknown"),
		"uid": str(player.get("uid", "")),
		"position": player.get("position", "MF"),
		"jersey_number": int(player.get("jersey_number", 1)),
		"overall": int(player.get("overall", 50)),
		"pace": int(player.get("pace", 50)),
		"shooting": int(player.get("shooting", 50)),
		"passing": int(player.get("passing", 50)),
		"dribbling": int(player.get("dribbling", 50)),
		"defending": int(player.get("defending", 50)),
		"physical": int(player.get("physical", 50)),
		"condition": int(player.get("condition", 3))
	}


## Build complete match Dictionary
static func build_match_dict(home_team, away_team, rng_seed: int = -1) -> Dictionary:
	"""
	Build complete match setup Dictionary

	Args:
		home_team: Home team data (TeamSetup or Dictionary)
		away_team: Away team data
		rng_seed: Random seed (generated if -1)

	Returns:
		Match setup Dictionary ready for engine
	"""
	var match_seed = rng_seed if rng_seed > 0 else Time.get_ticks_usec()

	return {
		"schema_version": SCHEMA_VERSION,
		"home_team": build_team_dict(home_team),
		"away_team": build_team_dict(away_team),
		"seed": match_seed,
		"weather": "clear",
		"stadium": "neutral"
	}


## Parse match result Dictionary
static func parse_match_result(result: Dictionary) -> Dictionary:
	"""
	Safe parsing of match result with defaults

	Args:
		result: Raw result from engine

	Returns:
		Normalized result Dictionary
	"""
	return {
		"success": result.get("success", false),
		"home_score": int(result.get("home_score", 0)),
		"away_score": int(result.get("away_score", 0)),
		"events": result.get("events", []),
		"timeline_data": result.get("timeline_data", {}),
		"stats": result.get("stats", {}),
		"possession": float(result.get("possession", 50.0)),
		"shots": result.get("shots", {})
	}


## Validate match Dictionary structure
static func validate_match_dict(match_data: Dictionary) -> Dictionary:
	"""
	Validate match Dictionary has required keys

	Args:
		match_data: Match setup Dictionary

	Returns:
		{valid: bool, errors: Array[String]}
	"""
	var errors = []

	if not match_data.has("home_team"):
		errors.append("Missing home_team")
	if not match_data.has("away_team"):
		errors.append("Missing away_team")
	if not match_data.has("seed"):
		errors.append("Missing seed")

	# Validate team structure (Rust engine requires exactly 18 players)
	for team_key in ["home_team", "away_team"]:
		if match_data.has(team_key):
			var team = match_data[team_key]
			if not team.has("players"):
				errors.append("%s missing players" % team_key)
			elif team.players.size() != 18:
				errors.append(
					"%s requires 18 players (11 starters + 7 bench), found %d" % [team_key, team.players.size()]
				)

	return {"valid": errors.is_empty(), "errors": errors}
