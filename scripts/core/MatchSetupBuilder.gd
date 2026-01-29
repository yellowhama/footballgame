## MatchSetupBuilder - Official Factory for MatchSetup Creation (Game OS v1.0)
##
## ╔═══════════════════════════════════════════════════════════════════════════╗
## ║  CRITICAL: This is the SINGLE AUTHORIZED entry point for creating        ║
## ║  MatchSetup instances. All game modes route through this builder.        ║
## ║  Direct MatchSetup instantiation is PROHIBITED.                          ║
## ╚═══════════════════════════════════════════════════════════════════════════╝
##
## RESPONSIBILITIES:
## - Validates roster requirements (18 players per team: 11 starters + 7 subs)
## - Assigns track_ids (0-21 for starters only, substitutes stored separately)
## - Enforces formation validation
## - Creates immutable MatchSetup configuration
##
## STANDARD USAGE PATTERN:
## ```gdscript
## # 1. Create/reuse PlayerLibrary
## if not _player_library:
##     _player_library = PlayerLibrary.new()
##
## # 2. Build roster UIDs (18 per team)
## var home_roster_uids = ["csv:1", "csv:2", ..., "csv:18"]
## var away_roster_uids = ["csv:501", "csv:502", ..., "csv:518"]
##
## # 3. Configure match
## var match_config = {
##     "seed": 12345,
##     "match_id": "career_001",
##     "match_type": "league",
##     "venue": "home",
##     "home_formation": "4-4-2",
##     "away_formation": "4-4-2"
## }
##
## # 4. Build MatchSetup via FACTORY
## var match_setup = MatchSetupBuilder.build(
##     home_roster_uids,
##     away_roster_uids,
##     match_config["home_formation"],
##     match_config["away_formation"],
##     _player_library,
##     match_config
## )
##
## # 5. Error handling
## if not match_setup:
##     push_error("MatchSetup creation failed - check roster size and UIDs")
##     return null
## ```
##
## FACTORY GUARANTEE:
## If build() succeeds (non-null), MatchSetup is guaranteed to be valid:
## - track_id range enforcement (0-21)
## - Team integrity verified
## - Formation validity checked
## - All 22 player slots populated
##
## HELPER FACTORIES:
## - build_test_match() - Quick testing utility (random rosters)
## - build_with_player_data() - Career Mode helper (player + 17 teammates)
##
## DERIVED FACTORIES:
## - InteractiveMatchSetupBuilder.build() - Wraps this factory for bullet-time mode
##
## ERROR HANDLING:
## - Returns null on failure (invalid roster size, formation, UID errors)
## - Prints warnings to console with specific error details
##
## ARCHITECTURE:
## Part of Game OS (MatchSetup Phase 17) - Single Source of Truth
##
## @see OpenFootballAPI.simulate_match_with_setup() - Consumes MatchSetup
## @see MatchSetupExporter.to_json() - Converts to Rust engine format
## @see docs/guides/GAME_OS_MIGRATION_GUIDE.md - Migration patterns
##
## Priority: P0 CRITICAL - All match simulations depend on this factory
class_name MatchSetupBuilder
extends RefCounted

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _PlayerLibrary = preload("res://scripts/core/PlayerLibrary.gd")
const _MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const _TeamSetup = preload("res://scripts/core/TeamSetup.gd")
const _MatchPlayer = preload("res://scripts/core/MatchPlayer.gd")
const _PlayerSlot = preload("res://scripts/core/PlayerSlot.gd")
const _GameConstantsCore = preload("res://scripts/core/GameConstants.gd")


## Build MatchSetup from roster UIDs
## @param home_roster_uids: Array of 18 UIDs (11 starters + 7 subs)
## @param away_roster_uids: Array of 18 UIDs (11 starters + 7 subs)
## @param home_formation: Formation string (e.g., "4-4-2")
## @param away_formation: Formation string (e.g., "4-4-2")
## @param player_library: PlayerLibrary instance for UID lookup
## @param config: Optional match config (seed, match_type, etc.)
static func build(
	home_roster_uids: Array,
	away_roster_uids: Array,
	home_formation: String,
	away_formation: String,
	player_library: _PlayerLibrary,
	config: Dictionary = {}
) -> _MatchSetup:
	# Validate roster sizes (FIFA regulation: 11 starters + 7 subs)
	if home_roster_uids.size() != _GameConstantsCore.ROSTER_SIZE_PER_TEAM:
		push_error(
			(
				"[MatchSetupBuilder] Home roster must have %d players (got %d)"
				% [_GameConstantsCore.ROSTER_SIZE_PER_TEAM, home_roster_uids.size()]
			)
		)
		return null

	if away_roster_uids.size() != _GameConstantsCore.ROSTER_SIZE_PER_TEAM:
		push_error(
			(
				"[MatchSetupBuilder] Away roster must have %d players (got %d)"
				% [_GameConstantsCore.ROSTER_SIZE_PER_TEAM, away_roster_uids.size()]
			)
		)
		return null

	# Create MatchSetup
	var match_setup = _MatchSetup.new()

	# Apply config
	match_setup.match_id = str(config.get("match_id", "match_%d" % Time.get_ticks_usec()))
	match_setup.rng_seed = int(config.get("seed", 0))
	match_setup.match_type = str(config.get("match_type", "friendly"))
	match_setup.venue = str(config.get("venue", "neutral"))
	match_setup.weather = str(config.get("weather", "clear"))

	# Build teams
	match_setup.home_team = _build_team(
		"Home", "home", home_roster_uids, home_formation, config.get("home_tactics", {}), player_library
	)

	match_setup.away_team = _build_team(
		"Away", "away", away_roster_uids, away_formation, config.get("away_tactics", {}), player_library
	)

	if match_setup.home_team == null or match_setup.away_team == null:
		push_error("[MatchSetupBuilder] Failed to build teams")
		return null

	# Assign track_ids to starters (0-21)
	_assign_track_ids(match_setup)

	# Validate
	var validation = match_setup.validate()
	if not validation.valid:
		push_error("[MatchSetupBuilder] Validation failed: %s" % str(validation.errors))
		return null

	print("[MatchSetupBuilder] ✅ Created MatchSetup: %s" % match_setup.get_summary())
	return match_setup


## Build TeamSetup from roster UIDs
static func _build_team(
	name: String,
	side: String,
	roster_uids: Array,
	formation: String,
	tactics: Dictionary,
	player_library: _PlayerLibrary
) -> _TeamSetup:
	var team = _TeamSetup.new()
	team.name = name
	team.side = side
	team.formation = formation

	# Apply tactics (or use defaults)
	if not tactics.is_empty():
		team.tactics = tactics.duplicate()
	else:
		team.apply_default_tactics()

	# Check for duplicate UIDs (warning only, non-blocking)
	var uid_set: Dictionary = {}
	for raw_uid in roster_uids:
		var uid_str := _normalize_roster_uid_for_lookup(raw_uid)
		if uid_set.has(uid_str):
			push_warning(
				"[MatchSetupBuilder] Duplicate UID in %s roster: %s (count: %d)" % [side, uid_str, uid_set[uid_str] + 1]
			)
			uid_set[uid_str] += 1
		else:
			uid_set[uid_str] = 1

	# Build starters (first 11 UIDs)
	for i in range(_GameConstantsCore.STARTERS_PER_TEAM):
		var lookup_uid := _normalize_roster_uid_for_lookup(roster_uids[i])
		var engine_uid := _normalize_roster_uid_for_engine(roster_uids[i])
		var player_data = player_library.get_player_data(lookup_uid)

		if player_data.is_empty():
			push_error("[MatchSetupBuilder] Failed to load player: %s" % lookup_uid)
			return null

		# ✅ CRITICAL: Duplicate dict to avoid modifying PlayerLibrary cache
		player_data = player_data.duplicate()

		# ✅ CRITICAL: Force engine UID format for MatchPlayer (SSOT 2025-12-23)
		# Rust PLAYER_LIBRARY uses integer keys (2) not string UIDs ("csv:2")
		player_data["uid"] = engine_uid
		player_data["player_id"] = engine_uid

		# Set jersey number if not present
		if not player_data.has("jersey_number") or player_data.jersey_number == 0:
			player_data["jersey_number"] = i + 1

		var player = _MatchPlayer.from_player_data(player_data)
		team.starters.append(player)

	# Store substitutes (last 7 UIDs, NO track_id)
	for i in range(_GameConstantsCore.STARTERS_PER_TEAM, _GameConstantsCore.ROSTER_SIZE_PER_TEAM):
		var engine_uid := _normalize_roster_uid_for_engine(roster_uids[i])
		team.substitutes.append(engine_uid)

	return team


## Normalize roster UID for PlayerLibrary lookup (Godot-side)
## Accepts legacy inputs like `1` or `"49"` and upgrades them to `"csv:<id>"`.
## NOTE: This is for LOOKUP only - engine UIDs use different format (numeric string)
static func _normalize_roster_uid_for_lookup(raw_uid: Variant) -> String:
	var uid_str := str(raw_uid).strip_edges()
	if uid_str.begins_with("csv:") or uid_str.begins_with("grad:"):
		return uid_str
	# Legacy numeric roster → treat as CSV UID
	if uid_str.is_valid_int():
		return "csv:%s" % uid_str
	return uid_str


## Normalize roster UID for Rust engine (csv: prefix format)
## ✅ Engine UID SSOT (2025-12-23):
## - Rust v2 API requires "csv:<u32>" format (resolve_person_by_player_uid)
## - grad:* is NOT resolvable by engine (caller must proxy before calling build())
## - Output: "csv:2", "csv:117", etc. ("csv:" prefix required)
static func _normalize_roster_uid_for_engine(raw_uid: Variant) -> String:
	var uid_str := str(raw_uid).strip_edges()

	# csv:<n> → keep as-is (already correct format)
	if uid_str.begins_with("csv:"):
		return uid_str

	# Pure numeric → "csv:<n>"
	if uid_str.is_valid_int():
		return "csv:%s" % uid_str

	# grad:* should NOT reach here - caller must proxy to valid engine UID
	# If it does, return as-is (will fail in engine, but logged for debugging)
	if uid_str.begins_with("grad:"):
		push_warning("[MatchSetupBuilder] grad:* UID reached engine normalization: %s (should be proxied)" % uid_str)

	return uid_str


## Assign track_ids to starters (CRITICAL: 0-21 only)
## Home starters: 0-10
## Away starters: 11-21
## Substitutes: NO track_id (stored in TeamSetup)
## ✅ P0-1 (2025-12-22 FIX_2512): Dual-keying for player_map (Dictionary)
static func _assign_track_ids(match_setup: _MatchSetup) -> void:
	# Ensure backward-compatible metadata map exists (MatchSetup.get_player uses player_map).
	# NOTE: MatchSetup.player_map is deprecated, but still required by several consumers.
	if match_setup.player_map == null:
		match_setup.player_map = {}

	# Home starters: track_id 0-10
	for i in range(_GameConstantsCore.STARTERS_PER_TEAM):
		var player = match_setup.home_team.starters[i]
		var slot = _PlayerSlot.new(i, player)
		var ks := str(i)  # ✅ P0-1: String key for dual storage (player_map only)

		# ✅ player_slots is Array[PlayerSlot] - int index only
		match_setup.player_slots[i] = slot

		var meta := {
			"track_id": i,
			"team": "home",
			"team_id": 0,
			"slot": i,
			"player_id": player.uid,
			"uid": player.uid,
			"name": player.name,
			"number": player.jersey_number,
			"position": player.position,
			"ca": player.overall,
		}

		# ✅ P0-1: Dual-key storage (Dictionary supports both int and string)
		match_setup.player_map[i] = meta
		match_setup.player_map[ks] = meta

	# Away starters: track_id 11-21
	for i in range(_GameConstantsCore.STARTERS_PER_TEAM):
		var player = match_setup.away_team.starters[i]
		var track_id := i + _GameConstantsCore.AWAY_TRACK_ID_START
		var ks := str(track_id)  # ✅ P0-1: String key for dual storage (player_map only)
		var slot = _PlayerSlot.new(track_id, player)

		# ✅ player_slots is Array[PlayerSlot] - int index only
		match_setup.player_slots[track_id] = slot

		var meta := {
			"track_id": track_id,
			"team": "away",
			"team_id": 1,
			"slot": i,
			"player_id": player.uid,
			"uid": player.uid,
			"name": player.name,
			"number": player.jersey_number,
			"position": player.position,
			"ca": player.overall,
		}

		# ✅ P0-1: Dual-key storage (Dictionary supports both int and string)
		match_setup.player_map[track_id] = meta
		match_setup.player_map[ks] = meta

	print(
		(
			"[MatchSetupBuilder] ✅ Assigned track_ids: Home %d-%d, Away %d-%d"
			% [
				_GameConstantsCore.HOME_TRACK_ID_START,
				_GameConstantsCore.HOME_TRACK_ID_END,
				_GameConstantsCore.AWAY_TRACK_ID_START,
				_GameConstantsCore.AWAY_TRACK_ID_END
			]
		)
	)


## Quick build for testing (generates random rosters)
static func build_test_match(
	player_library: _PlayerLibrary, rng_seed: int = 0, home_formation: String = "4-4-2", away_formation: String = "4-4-2"
) -> _MatchSetup:
	var home_roster = player_library.generate_random_roster(18, 1, 500)
	var away_roster = player_library.generate_random_roster(18, 501, 1000)

	var config = {
		"seed": rng_seed if rng_seed > 0 else Time.get_ticks_usec(),
		"match_type": "test",
		"match_id": "test_match_%d" % Time.get_ticks_usec()
	}

	return build(home_roster, away_roster, home_formation, away_formation, player_library, config)


## Build from existing PlayerData (Career Mode helper)
static func build_with_player_data(
	player_data_uid: String,
	opponent_roster_uids: Array,
	formation: String,
	opponent_formation: String,
	player_library: _PlayerLibrary,
	config: Dictionary = {}
) -> _MatchSetup:
	# Build home roster: player + 17 teammates
	var home_roster = [player_data_uid]

	# Generate 17 teammates
	for i in range(17):
		var teammate_roster = player_library.generate_random_roster(1, 1, 1000)
		home_roster.append(teammate_roster[0])

	return build(home_roster, opponent_roster_uids, formation, opponent_formation, player_library, config)


## Migration helper for legacy from_rosters() pattern
## Converts legacy roster format (Dictionary with "players" array) to modern MatchSetup
## Used by MatchTimelineController during SSOT cleanup (2025-12-22 FIX_2512)
## @param home_roster: Legacy roster format (Dictionary with "players" key or Array)
## @param away_roster: Legacy roster format (Dictionary with "players" key or Array)
## @returns MatchSetup with player_slots populated (Game OS SSOT format)
static func from_rosters_legacy(home_roster: Variant, away_roster: Variant) -> _MatchSetup:
	var match_setup := _MatchSetup.new()

	# Initialize player_map (Dictionary for backward compatibility)
	# NOTE: player_slots is already initialized as Array[PlayerSlot] in MatchSetup._init()
	if match_setup.player_map == null:
		match_setup.player_map = {}

	# Extract players array from legacy format
	var home_players: Array = []
	if home_roster is Dictionary and home_roster.has("players"):
		home_players = home_roster["players"]
	elif home_roster is Array:
		home_players = home_roster
	else:
		push_error("[MatchSetupBuilder] Invalid home_roster format")
		return match_setup

	var away_players: Array = []
	if away_roster is Dictionary and away_roster.has("players"):
		away_players = away_roster["players"]
	elif away_roster is Array:
		away_players = away_roster
	else:
		push_error("[MatchSetupBuilder] Invalid away_roster format")
		return match_setup

	# Populate home team (track_id 0-10)
	for i in range(min(11, home_players.size())):
		var player_data: Dictionary = home_players[i]
		var track_id := i

		var match_player := _MatchPlayer.new()
		match_player.uid = str(player_data.get("uid", player_data.get("id", i)))
		match_player.name = str(player_data.get("name", "Player %d" % i))
		match_player.jersey_number = int(player_data.get("number", i + 1))
		match_player.position = str(player_data.get("position", "MF"))
		match_player.overall = int(player_data.get("ca", 100))
		match_player.technical = int(player_data.get("ca", 100))
		match_player.mental = int(player_data.get("ca", 100))
		match_player.physical = int(player_data.get("ca", 100))

		var slot := _PlayerSlot.new(track_id, match_player)
		var ks := str(track_id)

		# ✅ player_slots is Array[PlayerSlot] - int index only
		match_setup.player_slots[track_id] = slot

		# Legacy player_map for backward compatibility (Dictionary - dual-key)
		var meta := {
			"track_id": track_id,
			"team": "home",
			"team_id": 0,
			"slot": i,
			"player_id": player_data.get("id", i),
			"uid": player_data.get("uid", player_data.get("id", i)),
			"name": player_data.get("name", "Player %d" % i),
			"number": player_data.get("number", i + 1),
			"position": player_data.get("position", ""),
			"ca": player_data.get("ca", 100)
		}

		match_setup.player_map[track_id] = meta
		match_setup.player_map[ks] = meta

	# Populate away team (track_id 11-21)
	for i in range(min(11, away_players.size())):
		var player_data: Dictionary = away_players[i]
		var track_id := i + 11

		var match_player := _MatchPlayer.new()
		match_player.uid = str(player_data.get("uid", player_data.get("id", i + 11)))
		match_player.name = str(player_data.get("name", "Player %d" % (i + 11)))
		match_player.jersey_number = int(player_data.get("number", i + 1))
		match_player.position = str(player_data.get("position", "MF"))
		match_player.overall = int(player_data.get("ca", 100))
		match_player.technical = int(player_data.get("ca", 100))
		match_player.mental = int(player_data.get("ca", 100))
		match_player.physical = int(player_data.get("ca", 100))

		var slot := _PlayerSlot.new(track_id, match_player)
		var ks := str(track_id)

		# ✅ player_slots is Array[PlayerSlot] - int index only
		match_setup.player_slots[track_id] = slot

		# Legacy player_map for backward compatibility (Dictionary - dual-key)
		var meta := {
			"track_id": track_id,
			"team": "away",
			"team_id": 1,
			"slot": i,
			"player_id": player_data.get("id", i + 11),
			"uid": player_data.get("uid", player_data.get("id", i + 11)),
			"name": player_data.get("name", "Player %d" % (i + 11)),
			"number": player_data.get("number", i + 1),
			"position": player_data.get("position", ""),
			"ca": player_data.get("ca", 100)
		}

		match_setup.player_map[track_id] = meta
		match_setup.player_map[ks] = meta

	print("[MatchSetupBuilder] ✅ Legacy migration: 22 players mapped")
	return match_setup
