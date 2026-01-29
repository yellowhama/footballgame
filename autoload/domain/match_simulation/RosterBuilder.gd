class_name RosterBuilder
extends RefCounted
## ============================================================================
## RosterBuilder - Match Roster UID Generation
## ============================================================================
##
## PURPOSE: Build roster UIDs for match simulation
##
## EXTRACTED FROM: MatchSimulationManager.gd (ST-005 God Class refactoring)
##
## RESPONSIBILITIES:
## - Build 18 UIDs for MVP mode player team
## - Build 18 UIDs for opponent team based on CA
## - Build 18 UIDs for Interactive mode
## - Build CSV-only rosters for session v2
## - Resolve engine UIDs (grad:* → csv:*)
## - Validate UIDs against Rust engine cache
##
## DEPENDENCIES:
## - FootballRustEngine (autoload): For UID validation
## - PlayerData (autoload): For main player UID
##
## USAGE:
##   var builder := RosterBuilder.new()
##   builder.initialize(rust_engine_node, player_data_node)
##   var home_uids := builder.build_roster_uids_for_mvp({}, 12345)
##   var away_uids := builder.build_opponent_roster_uids(70, 12345)
## ============================================================================

## Reference to external nodes
var _rust_engine: Node = null
var _player_data: Node = null
var _scene_tree: SceneTree = null


func initialize(rust_engine: Node = null, player_data: Node = null, scene_tree: SceneTree = null) -> void:
	"""Initialize RosterBuilder with external dependencies"""
	_rust_engine = rust_engine
	_player_data = player_data
	_scene_tree = scene_tree


# =============================================================================
# Public API
# =============================================================================

func build_roster_uids_for_mvp(_player_data_dict: Dictionary, rng_seed: int) -> Array:
	"""Build 18 UIDs (1 player + 17 teammates) for MVP mode

	Engine UID SSOT (2025-12-23):
	- Rust PLAYER_LIBRARY uses integer keys (2) not string UIDs ("csv:2")
	- Never pass "grad:*" to engine - use proxy UID from cache
	- Output format: ["117", "2", "45", ...] (numeric strings only)
	"""
	var roster_uids = []

	# Main player (from PlayerData)
	if _player_data and _player_data.has_method("get_uid"):
		var raw_uid = _player_data.get_uid()
		var engine_uid := resolve_engine_uid_for_main_player(raw_uid)
		if engine_uid == "":
			engine_uid = pick_valid_engine_uid_near(100)
		if engine_uid == "":
			engine_uid = "1"
		roster_uids.append(engine_uid)
		print("[RosterBuilder] MVP player (engine uid): %s (raw=%s)" % [engine_uid, str(raw_uid)])
	else:
		roster_uids.append("1")
		push_warning("[RosterBuilder] PlayerData.get_uid() not available, using uid 1")

	# Generate 17 teammates (deterministic from seed)
	var rng = RandomNumberGenerator.new()
	rng.seed = rng_seed + 1000

	for i in range(17):
		var csv_id = rng.randi_range(1, 1000)
		var uid := pick_valid_engine_uid_near(csv_id)
		if uid == "":
			uid = str(int(csv_id))
		roster_uids.append(uid)

	print("[RosterBuilder] Built MVP roster: %d players (engine UIDs)" % roster_uids.size())
	return roster_uids


func build_opponent_roster_uids(opponent_ca: int, rng_seed: int) -> Array:
	"""Build 18 UIDs for opponent team

	Engine UID SSOT (2025-12-23):
	- Output format: ["2", "45", "117", ...] (numeric strings only)
	"""
	var roster_uids = []
	var rng = RandomNumberGenerator.new()
	rng.seed = rng_seed + 2000

	var used := {}
	var attempts := 0
	while roster_uids.size() < 18 and attempts < 5000:
		attempts += 1
		var csv_id = opponent_ca + rng.randi_range(-20, 20)
		csv_id = clamp(csv_id, 1, 1000)
		var uid := pick_valid_engine_uid_near(csv_id)
		if uid == "":
			uid = str(int(csv_id))
		if used.has(uid):
			continue
		used[uid] = true
		roster_uids.append(uid)

	# Fallback padding
	var fallback_id := 1
	while roster_uids.size() < 18:
		var uid := str(int(fallback_id))
		fallback_id += 1
		if used.has(uid):
			continue
		used[uid] = true
		roster_uids.append(uid)

	print("[RosterBuilder] Built opponent roster: %d players (engine UIDs), CA ~%d" % [roster_uids.size(), opponent_ca])
	return roster_uids


func build_roster_uids_for_interactive(_player_data_dict: Dictionary, _match_data: Dictionary, rng_seed: int) -> Array:
	"""Build 18 UIDs for Interactive Mode player team

	Phase E.2: Game OS Migration (Interactive Mode)
	Handles Hall of Fame snapshot or generates from PlayerData like MVP mode.
	"""
	var roster_uids = []

	# BM Two-Track: Check for Hall of Fame snapshot from StageSelectScreen
	var stage_player_team: Dictionary = {}
	if _scene_tree and _scene_tree.root:
		stage_player_team = _scene_tree.root.get_meta("stage_player_team", {})

	if not stage_player_team.is_empty():
		print("[RosterBuilder] Hall of Fame snapshot detected for Interactive Mode")
		var squad: Array = stage_player_team.get("squad", [])

		var rng = RandomNumberGenerator.new()
		rng.seed = rng_seed + 3000

		for i in range(min(squad.size(), 18)):
			var p = squad[i]
			var ca = int(p.get("ca", p.get("overall", 60)))
			var csv_id = clamp(ca + rng.randi_range(-10, 10), 1, 1000)
			roster_uids.append("csv:%d" % csv_id)

		while roster_uids.size() < 18:
			roster_uids.append("csv:%d" % rng.randi_range(50, 70))

		print("[RosterBuilder] Built Interactive roster from Hall of Fame: %d players" % roster_uids.size())
		return roster_uids

	# Default: Build from PlayerData (same as MVP mode)
	if _player_data and _player_data.has_method("get_uid"):
		var player_uid = _player_data.get_uid()
		roster_uids.append(player_uid)
		print("[RosterBuilder] Interactive player UID: %s" % player_uid)
	else:
		roster_uids.append("csv:1")
		push_warning("[RosterBuilder] PlayerData.get_uid() not available, using csv:1")

	var rng = RandomNumberGenerator.new()
	rng.seed = rng_seed + 1000

	for i in range(17):
		var csv_id = rng.randi_range(1, 1000)
		roster_uids.append("csv:%d" % csv_id)

	print("[RosterBuilder] Built Interactive roster from PlayerData: %d players (Game OS mode)" % roster_uids.size())
	return roster_uids


func build_csv_only_roster_uids_for_session_v2(player_data_dict: Dictionary, rng_seed: int) -> Array:
	"""Build 18 unique CSV UIDs for session mode (MatchRequest v2).

	Lock (v2): roster must be CSV UID-only and unique; graduated PlayerLibrary integration deferred to v3.
	"""
	var roster_uids: Array = []

	var base_ca := int(player_data_dict.get("overall", 60))
	base_ca = clamp(base_ca, 1, 1000)

	var rng := RandomNumberGenerator.new()
	rng.seed = rng_seed + 1100

	var used := {}
	var attempts := 0
	while roster_uids.size() < 18 and attempts < 5000:
		attempts += 1
		var csv_id := base_ca + rng.randi_range(-20, 20)
		csv_id = clamp(csv_id, 1, 1000)
		var uid := "csv:%d" % csv_id
		if used.has(uid):
			continue
		used[uid] = true
		roster_uids.append(uid)

	var fallback_id := 1
	while roster_uids.size() < 18:
		var uid := "csv:%d" % fallback_id
		fallback_id += 1
		if used.has(uid):
			continue
		used[uid] = true
		roster_uids.append(uid)

	if OS.is_debug_build():
		print("[RosterBuilder] Built session v2 roster (csv-only): %d players (base_ca=%d)" % [roster_uids.size(), base_ca])

	return roster_uids


# =============================================================================
# UID Resolution Utilities
# =============================================================================

func resolve_engine_uid_for_main_player(raw_uid: Variant) -> String:
	"""Resolve main player UID to engine-compatible format

	Engine UID SSOT (2025-12-23):
	- grad:* is not resolvable by Rust PLAYER_LIBRARY
	- Must proxy to a real engine cache UID ("csv:<n>" format)
	- Rust v2 API requires "csv:" prefix
	"""
	var s := str(raw_uid).strip_edges()

	# Already numeric → add "csv:" prefix
	if s.is_valid_int():
		return "csv:%s" % s

	# Already "csv:<n>" → keep as is
	if s.begins_with("csv:"):
		return s

	# grad:* → pick a proxy uid that actually exists in engine cache
	if s.begins_with("grad:"):
		return pick_valid_engine_uid_near(100)

	return ""


func pick_valid_engine_uid_near(center: int) -> String:
	"""Find a UID that exists in engine cache (uses FootballRustEngine.has_player_uid)
	Searches outward from center to avoid "Player not found" failures

	Engine UID SSOT (2025-12-23):
	- Returns "csv:<n>" format ("csv:2", "csv:117", etc.)
	- Guarantees the UID exists in Rust PLAYER_LIBRARY
	- Rust v2 API requires "csv:" prefix
	"""
	var c := int(center)
	var max_radius := 200

	for r in range(max_radius + 1):
		var a: int = clamp(c - r, 1, 1000000)
		var b: int = clamp(c + r, 1, 1000000)
		var ua := str(a)
		var ub := str(b)

		if _rust_engine and _rust_engine.has_method("has_player_uid"):
			if bool(_rust_engine.call("has_player_uid", ua)):
				return "csv:%s" % ua
			if bool(_rust_engine.call("has_player_uid", ub)):
				return "csv:%s" % ub
		else:
			# If engine cache check isn't available, just return "csv:" prefixed
			return "csv:%s" % ua

	return ""
