## InteractiveMatchSetup - Game OS wrapper for Interactive Mode (Bullet-Time)
## Part of Game OS Phase E
##
## DESIGN: Wraps MatchSetup with interactive-specific metadata
## - base_match_setup: Reuses Game OS validation/normalization
## - user_player_track_id: Which player user controls (0-21)
## - intervention_config: Auto-play and frequency settings
##
## KEY METHOD: to_interactive_request() - Converts MatchSetup → InteractiveMatchRequest
## This bridges Game OS (UID-based) to Interactive API (inline player data)
##
## Priority: P0 (Phase E.1)
class_name InteractiveMatchSetup
extends RefCounted

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const _TeamSetup = preload("res://scripts/core/TeamSetup.gd")
const _PlayerLibrary = preload("res://scripts/core/PlayerLibrary.gd")
const _GameConstantsCore = preload("res://scripts/core/GameConstants.gd")

## Base MatchSetup (created via MatchSetupBuilder)
var base_match_setup: _MatchSetup = null

## User player control
var user_player_track_id: int = -1  # 0-21 (home 0-10, away 11-21), -1 = no user control
var user_player_team: String = "home"  # "home" or "away"

## Highlight configuration
var highlight_level: String = "my_player"  # "skip", "simple", "my_player", "full"

## Intervention configuration (Phase E.3)
var intervention_config: Dictionary = {
	"frequency": "medium",  # "high" (every touch), "medium" (key moments), "low" (shots/key passes)
	"auto_play": false,  # Auto-select best action based on probabilities
}


## Validate interactive-specific requirements
func validate_interactive() -> Dictionary:
	var errors: Array = []

	# Check base setup exists
	if base_match_setup == null:
		errors.append("Base MatchSetup is null")
		return {"valid": false, "errors": errors}

	# Validate base setup
	var base_validation = base_match_setup.validate()
	if not base_validation.valid:
		errors.append("Base MatchSetup validation failed: %s" % str(base_validation.errors))

	# Check user player track_id validity
	if user_player_track_id != -1:  # -1 = no user control (AI only)
		if user_player_track_id < 0 or user_player_track_id >= _GameConstantsCore.TOTAL_PLAYER_SLOTS:
			errors.append("Invalid user_player_track_id: %d (must be 0-21 or -1 for no control)" % user_player_track_id)

		# Check user player exists
		var user_slot = base_match_setup.get_slot(user_player_track_id)
		if user_slot == null or user_slot.player == null:
			errors.append("User player not found at track_id %d" % user_player_track_id)

		# Verify team consistency
		var expected_team = base_match_setup.get_team_side(user_player_track_id)
		if expected_team != user_player_team:
			errors.append(
				(
					"User player team mismatch: track_id %d is %s, but user_player_team is %s"
					% [user_player_track_id, expected_team, user_player_team]
				)
			)

	# Validate highlight level
	var valid_levels = ["skip", "simple", "my_player", "full"]
	if not highlight_level in valid_levels:
		errors.append("Invalid highlight_level: '%s' (must be one of %s)" % [highlight_level, str(valid_levels)])

	# Validate intervention frequency
	var valid_frequencies = ["high", "medium", "low"]
	var frequency = intervention_config.get("frequency", "medium")
	if not frequency in valid_frequencies:
		errors.append("Invalid intervention frequency: '%s' (must be one of %s)" % [frequency, str(valid_frequencies)])

	return {"valid": errors.is_empty(), "errors": errors}


## Convert to InteractiveMatchRequest format (for Rust engine)
## This method resolves MatchSetup UIDs → inline player data for the engine
func to_interactive_request(rng_seed: int, player_library: _PlayerLibrary) -> Dictionary:
	if base_match_setup == null:
		push_error("[InteractiveMatchSetup] Cannot convert to request: base_match_setup is null")
		return {}

	# Validate first
	var validation = validate_interactive()
	if not validation.valid:
		push_error("[InteractiveMatchSetup] Validation failed: %s" % str(validation.errors))
		return {}

	# Build home team inline data
	var home_team_data = _build_team_data_for_request(
		base_match_setup.home_team, base_match_setup, "home", player_library
	)

	# Build away team inline data
	var away_team_data = _build_team_data_for_request(
		base_match_setup.away_team, base_match_setup, "away", player_library
	)

	# Build user_player config (if user control enabled)
	var user_player_data = null
	if user_player_track_id != -1:
		var user_slot = base_match_setup.get_slot(user_player_track_id)
		if user_slot and user_slot.player:
			user_player_data = {
				"team": user_player_team, "player_name": user_slot.player.name, "highlight_level": highlight_level
			}

	# Build request
	var request = {
		"schema_version": 1,
		"seed": rng_seed,
		"home_team": home_team_data,
		"away_team": away_team_data,
		"user_player": user_player_data
	}

	# Add tactical instructions if present
	if base_match_setup.home_team.tactics and not base_match_setup.home_team.tactics.is_empty():
		request["home_instructions"] = base_match_setup.home_team.tactics.duplicate()

	if base_match_setup.away_team.tactics and not base_match_setup.away_team.tactics.is_empty():
		request["away_instructions"] = base_match_setup.away_team.tactics.duplicate()

	return request


## Build team data in InteractiveMatchRequest format
## Converts TeamSetup + MatchSetup player slots → inline player array (18 players)
static func _build_team_data_for_request(
	team_setup: _TeamSetup, _match_setup: _MatchSetup, _side: String, player_library: _PlayerLibrary
) -> Dictionary:
	var players_data = []

	# Get roster UIDs (11 starters + 7 subs)
	var roster_uids = team_setup.get_roster_uids()  # Returns Array of 18 UIDs

	# Resolve each UID → player data
	for uid in roster_uids:
		var player_data = player_library.get_player_data(uid)

		if player_data == null:
			# Fallback: Create minimal player data
			push_warning("[InteractiveMatchSetup] Failed to resolve UID '%s', using fallback" % uid)
			player_data = {"name": "Player_%s" % uid.substr(4, 3), "position": "MF", "overall": 65}  # e.g., "csv:123" → "Player_123"

		# Convert to InteractivePlayer format
		players_data.append(
			{
				"name": player_data.get("name", "Unknown"),
				"position": player_data.get("position", "MF"),
				"overall": int(player_data.get("overall", 65))
			}
		)

	return {"name": team_setup.name, "formation": team_setup.formation, "players": players_data}  # Exactly 18 players


## Get summary string for debugging
func get_summary() -> String:
	if base_match_setup == null:
		return "InteractiveMatchSetup (no base setup)"

	var user_info = "no user control"
	if user_player_track_id != -1:
		var player_name = base_match_setup.get_player_name(user_player_track_id)
		user_info = "user: %s (track_id %d, %s team)" % [player_name, user_player_track_id, user_player_team]

	return (
		"InteractiveMatchSetup: %s (%s, highlight: %s)" % [base_match_setup.get_summary(), user_info, highlight_level]
	)


## Export to dictionary (for persistence)
func to_dict() -> Dictionary:
	return {
		"base_match_setup": base_match_setup.to_dict() if base_match_setup else null,
		"user_player_track_id": user_player_track_id,
		"user_player_team": user_player_team,
		"highlight_level": highlight_level,
		"intervention_config": intervention_config.duplicate()
	}
