## InteractiveMatchSetupBuilder - Factory for creating InteractiveMatchSetup
## Part of Game OS Phase E
##
## CRITICAL: Single point of InteractiveMatchSetup creation
## - Wraps MatchSetupBuilder (reuses validation/normalization)
## - Finds user player track_id from UID
## - Validates interactive-specific requirements
##
## Priority: P0 (Phase E.1)
class_name InteractiveMatchSetupBuilder
extends RefCounted

# Preload scripts to avoid Godot 4.4 class_name resolution issues
const _PlayerLibrary = preload("res://scripts/core/PlayerLibrary.gd")
const _MatchSetupBuilder = preload("res://scripts/core/MatchSetupBuilder.gd")
const _InteractiveMatchSetup = preload("res://scripts/core/InteractiveMatchSetup.gd")
const _MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const _GameConstantsCore = preload("res://scripts/core/GameConstants.gd")


## Build InteractiveMatchSetup from roster UIDs
## @param home_roster_uids: Array of 18 UIDs (11 starters + 7 subs)
## @param away_roster_uids: Array of 18 UIDs (11 starters + 7 subs)
## @param home_formation: Formation string (e.g., "4-4-2")
## @param away_formation: Formation string (e.g., "4-4-2")
## @param player_library: PlayerLibrary instance for UID lookup
## @param match_config: Match config (seed, match_type, etc.)
## @param user_player_config: User player config {team, player_uid, highlight_level, intervention_config}
static func build(
	home_roster_uids: Array,
	away_roster_uids: Array,
	home_formation: String,
	away_formation: String,
	player_library: _PlayerLibrary,
	match_config: Dictionary,
	user_player_config: Dictionary
) -> _InteractiveMatchSetup:
	# Step 1: Build base MatchSetup using MatchSetupBuilder
	print("[InteractiveMatchSetupBuilder] Building base MatchSetup...")
	var base_setup = _MatchSetupBuilder.build(
		home_roster_uids, away_roster_uids, home_formation, away_formation, player_library, match_config
	)

	if base_setup == null:
		push_error("[InteractiveMatchSetupBuilder] Failed to build base MatchSetup")
		return null

	# Step 2: Find user player track_id (if user control enabled)
	var user_track_id = -1  # -1 = no user control (AI only)
	var user_team = user_player_config.get("team", "home")

	if user_player_config.has("player_uid"):
		var player_uid = str(user_player_config.get("player_uid"))
		print("[InteractiveMatchSetupBuilder] Finding user player: %s (team: %s)" % [player_uid, user_team])

		user_track_id = _find_user_player_track_id(base_setup, user_team, player_uid)

		if user_track_id == -1:
			push_error(
				"[InteractiveMatchSetupBuilder] User player not found: UID '%s' in team '%s'" % [player_uid, user_team]
			)
			# Don't fail, just disable user control
			push_warning("[InteractiveMatchSetupBuilder] Continuing without user control (AI only)")

	# Step 3: Create InteractiveMatchSetup
	var interactive_setup = _InteractiveMatchSetup.new()
	interactive_setup.base_match_setup = base_setup
	interactive_setup.user_player_track_id = user_track_id
	interactive_setup.user_player_team = user_team
	interactive_setup.highlight_level = str(user_player_config.get("highlight_level", "my_player"))

	# Apply intervention config (if provided)
	if user_player_config.has("intervention_config"):
		var intervention = user_player_config.get("intervention_config")
		interactive_setup.intervention_config = intervention.duplicate()

	# Step 4: Validate interactive-specific requirements
	var validation = interactive_setup.validate_interactive()
	if not validation.valid:
		push_error("[InteractiveMatchSetupBuilder] Validation failed: %s" % str(validation.errors))
		return null

	print("[InteractiveMatchSetupBuilder] ✅ Created InteractiveMatchSetup: %s" % interactive_setup.get_summary())
	return interactive_setup


## Find user player track_id by UID
## Searches through MatchSetup.player_slots to find matching UID
## @return track_id (0-21) or -1 if not found
static func _find_user_player_track_id(match_setup: _MatchSetup, team: String, player_uid: String) -> int:
	# Determine search range based on team
	var start_id = 0
	var end_id = _GameConstantsCore.STARTERS_PER_TEAM  # 11

	if team == "away":
		start_id = _GameConstantsCore.AWAY_TRACK_ID_START  # 11
		end_id = _GameConstantsCore.TOTAL_PLAYER_SLOTS  # 22

	# Search starters for matching UID
	for track_id in range(start_id, end_id):
		var slot = match_setup.get_slot(track_id)
		if slot and slot.player:
			if slot.player.uid == player_uid:
				print(
					"[InteractiveMatchSetupBuilder] Found user player: %s at track_id %d" % [slot.player.name, track_id]
				)
				return track_id

	# Not found
	push_warning(
		(
			"[InteractiveMatchSetupBuilder] Player UID '%s' not found in %s starters (searched track_id %d-%d)"
			% [player_uid, team, start_id, end_id - 1]
		)
	)
	return -1


## Build with auto-selected user player (first forward)
## Helper method for quick testing
static func build_with_auto_user_player(
	home_roster_uids: Array,
	away_roster_uids: Array,
	home_formation: String,
	away_formation: String,
	player_library: _PlayerLibrary,
	match_config: Dictionary
) -> _InteractiveMatchSetup:
	# Auto-select first player (usually striker)
	var user_player_config = {
		"team": "home",
		"player_uid": home_roster_uids[0] if home_roster_uids.size() > 0 else "csv:1",
		"highlight_level": "my_player"
	}

	return build(
		home_roster_uids,
		away_roster_uids,
		home_formation,
		away_formation,
		player_library,
		match_config,
		user_player_config
	)


## Build without user control (AI only, for testing)
static func build_ai_only(
	home_roster_uids: Array,
	away_roster_uids: Array,
	home_formation: String,
	away_formation: String,
	player_library: _PlayerLibrary,
	match_config: Dictionary
) -> _InteractiveMatchSetup:
	# No user control
	var user_player_config = {"team": "home", "highlight_level": "skip"}  # No highlights

	return build(
		home_roster_uids,
		away_roster_uids,
		home_formation,
		away_formation,
		player_library,
		match_config,
		user_player_config
	)
