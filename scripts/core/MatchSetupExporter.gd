## MatchSetupExporter - Convert MatchSetup → Engine JSON (v2 format)
## Part of Game OS (MatchSetup Phase 17)
##
## CRITICAL: Single point of UID normalization
## - Lenient UID parsing: "csv:1234", "csv_1234", "1234" → 1234
## - Converts to MatchRequest v2 format
## - Handles CSV players (reference) vs Graduated players (full data)
##
## Risk Mitigation:
## - Risk 2: UID canonicalization (single parsing point)
## - Condition 3: Engine calls use this exporter ONLY
##
## Priority: P0 (File 7 of 7)
##
## ------------------------------------------------------------
## ENGINE UID SSOT (HARD RULE - P0.5)
## Engine input UID must be numeric string only: "117"
## Domain/UI may use namespaced UID: "csv:117", "grad:1"
## Boundary conversion is ONLY allowed here (MatchSetupExporter).
## Validation is MANDATORY at engine entry (OpenFootballAPI).
## See: docs/specs/FIX_2512/1223/PLAYER_CACHE_P0_5_STABILIZATION.md
## ------------------------------------------------------------
class_name MatchSetupExporter
extends RefCounted

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const _TeamSetup = preload("res://scripts/core/TeamSetup.gd")
const _MatchPlayer = preload("res://scripts/core/MatchPlayer.gd")

## Schema version for engine compatibility
const SCHEMA_VERSION = 2

## Log throttling: per match_id warn-once registry
## Key: match_id -> { warning_key: true }
static var _WARN_ONCE: Dictionary = {}


## Warn once per match_id + key combination (spam guard)
static func _warn_once(match_id: Variant, key: String, message: String) -> void:
	var mid := str(match_id)
	if mid == "":
		mid = "unknown_match"
	if not _WARN_ONCE.has(mid):
		_WARN_ONCE[mid] = {}
	var bucket: Dictionary = _WARN_ONCE[mid]
	if bucket.has(key):
		return  # Already warned for this match_id + key
	bucket[key] = true
	_WARN_ONCE[mid] = bucket
	push_warning(message)


## Export MatchSetup to engine JSON (Rust MatchSetup structure)
static func to_json(match_setup: _MatchSetup) -> Dictionary:
	if match_setup == null:
		push_error("[MatchSetupExporter] MatchSetup is null")
		return {}

	# Use match_id as track_id (required by Rust MatchSetup)
	var track_id = match_setup.match_id if match_setup.match_id != "" else _generate_uuid()
	var match_id = match_setup.match_id  # For warn-once throttling

	var result = {
		"track_id": track_id,
		"match_type": "Friendly",  # Default match type
		"home_team": _export_team(match_setup.home_team, match_id),
		"away_team": _export_team(match_setup.away_team, match_id),
		"environment":
		{"weather": "Clear", "pitch_condition": "Good", "temperature": 20, "humidity": 50, "attendance": 0},
		"seed": match_setup.rng_seed,
		"created_at": _get_rfc3339_timestamp(),  # RFC 3339 format for Rust DateTime<Utc>
		"metadata": {}
	}

	var home_instructions := _build_team_instructions_from_tactics(
		match_setup.home_team.tactics if match_setup.home_team else {}
	)
	if not home_instructions.is_empty():
		result["home_instructions"] = home_instructions
		if OS.is_debug_build():
			print("[MatchSetupExporter] Home instructions:", home_instructions)

	var away_instructions := _build_team_instructions_from_tactics(
		match_setup.away_team.tactics if match_setup.away_team else {}
	)
	if not away_instructions.is_empty():
		result["away_instructions"] = away_instructions
		if OS.is_debug_build():
			print("[MatchSetupExporter] Away instructions:", away_instructions)

	return result


## Export TeamSetup to engine format (Rust TeamPreset structure)
static func _export_team(team: _TeamSetup, match_id: Variant) -> Dictionary:
	if team == null:
		push_error("[MatchSetupExporter] Team is null")
		return {}

	# Generate UUID v4 for team (required by Rust TeamPreset)
	var team_id = _generate_uuid()

	# Convert starters to PlayerId format (numeric strings only)
	# ✅ ENGINE UID SSOT (2025-12-23): Rust PLAYER_LIBRARY uses integer keys (2) not "csv:2"
	var starting_xi = []
	for player in team.starters:
		var pid := _to_engine_player_id(player.uid)
		if pid != "":
			starting_xi.append(pid)
		else:
			# grad:* must NOT reach engine. Pick a nearby valid uid from engine cache.
			var fallback := _pick_valid_engine_uid_near(_guess_center_uid(team), 200)
			if fallback == "":
				fallback = "1"
			_warn_once(
				match_id,
				"uid_fallback:" + str(player.uid),
				(
					"[MatchSetupExporter] Non-engine UID %s - using fallback %s (match_id=%s, warn once)"
					% [str(player.uid), fallback, str(match_id)]
				)
			)
			starting_xi.append(fallback)

	# Convert substitutes to PlayerId format (numeric strings only)
	var bench = []
	for uid in team.substitutes:
		var pid := _to_engine_player_id(uid)
		if pid != "":
			bench.append(pid)
		else:
			var fallback := _pick_valid_engine_uid_near(_guess_center_uid(team), 200)
			if fallback == "":
				fallback = "1"
			_warn_once(
				match_id,
				"uid_fallback:" + str(uid),
				(
					"[MatchSetupExporter] Non-engine bench UID %s - using fallback %s (match_id=%s, warn once)"
					% [str(uid), fallback, str(match_id)]
				)
			)
			bench.append(fallback)  # Fallback instead of placeholder

	# Ensure we have exactly 11 starters
	while starting_xi.size() < 11:
		var fallback := _pick_valid_engine_uid_near(_guess_center_uid(team), 200)
		starting_xi.append(fallback if fallback != "" else "1")

	# Ensure we have exactly 7 bench players
	while bench.size() < 7:
		var fallback := _pick_valid_engine_uid_near(_guess_center_uid(team), 200)
		bench.append(fallback if fallback != "" else "1")

	return {
		"id": team_id,
		"name": team.name,
		"description": null,
		"starting_xi": starting_xi,
		"bench": bench,
		"formation_id": team.formation,
		"tactical_preset": "Balanced",  # Default tactical preset
		"created_at": _get_rfc3339_timestamp(),  # RFC 3339 format for Rust DateTime<Utc>
		"metadata": {}
	}


static func _build_team_instructions_from_tactics(tactics: Dictionary) -> Dictionary:
	if tactics.is_empty():
		return {}
	return {
		"defensive_line": _map_defensive_line_value(tactics.get("defensive_line", 50.0)),
		"pressing_intensity": _map_pressing_value(tactics.get("pressing", 50.0)),
		"team_tempo": _map_tempo_value(tactics.get("tempo", 50.0)),
		"team_width": _map_width_value(tactics.get("width", 50.0)),
		"build_up_style": _map_build_up_value(
			tactics.get("passing_style", tactics.get("directness", 50.0))
		),
		"use_offside_trap": bool(tactics.get("use_offside_trap", false))
	}


static func _map_defensive_line_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryHigh"
	if value >= 65.0:
		return "High"
	if value >= 45.0:
		return "Normal"
	if value >= 25.0:
		return "Deep"
	return "VeryDeep"


static func _map_pressing_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryHigh"
	if value >= 65.0:
		return "High"
	if value >= 45.0:
		return "Medium"
	if value >= 25.0:
		return "Low"
	return "VeryLow"


static func _map_tempo_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryFast"
	if value >= 65.0:
		return "Fast"
	if value >= 45.0:
		return "Normal"
	if value >= 25.0:
		return "Slow"
	return "VerySlow"


static func _map_width_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryWide"
	if value >= 65.0:
		return "Wide"
	if value >= 45.0:
		return "Normal"
	if value >= 25.0:
		return "Narrow"
	return "VeryNarrow"


static func _map_build_up_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 60.0:
		return "Direct"
	if value <= 40.0:
		return "Short"
	return "Mixed"


## Export player to v2 format (CSV reference vs Full data)
static func _export_player_v2(player: _MatchPlayer) -> Dictionary:
	var csv_id = _parse_csv_id(player.uid)

	if csv_id != null:
		# CSV player: Reference only (engine loads from CSV)
		return {
			"source": "csv",
			"csv_id": csv_id,
			"name": player.name,
			"position": player.position,
			"overall": player.overall,
			"jersey_number": player.jersey_number
		}
	else:
		# Graduated/Custom player: Full data (engine uses inline)
		return {
			"source": "dynamic",
			"name": player.name,
			"position": player.position,
			"preferred_position": player.preferred_position,
			"overall": player.overall,
			"technical": player.technical,
			"mental": player.mental,
			"physical": player.physical,
			"pace": player.pace,
			"shooting": player.shooting,
			"passing": player.passing,
			"dribbling": player.dribbling,
			"defending": player.defending,
			"goalkeeping": player.goalkeeping,
			"condition": player.condition,
			"morale": player.morale,
			"jersey_number": player.jersey_number,
			"traits": player.traits.duplicate() if not player.traits.is_empty() else []
		}


## Parse CSV UID to integer ID (LENIENT)
## CRITICAL: Single point of UID normalization (Risk 2 mitigation)
##
## Accepts:
## - "csv:1234" → 1234
## - "csv_1234" → 1234
## - "1234" (int) → 1234
## - "1234" (string) → 1234
##
## Returns null for non-CSV UIDs (graduated players)
static func _parse_csv_id(uid: Variant) -> Variant:
	var uid_str = str(uid)

	# Format: "csv:1234"
	if uid_str.begins_with("csv:"):
		var id_str = uid_str.substr(4)
		if id_str.is_valid_int():
			return int(id_str)
		else:
			# Invalid format - return null silently (caller will warn once if needed)
			return null

	# Format: "csv_1234"
	if uid_str.begins_with("csv_"):
		var id_str = uid_str.substr(4)
		if id_str.is_valid_int():
			return int(id_str)
		else:
			# Invalid format - return null silently (caller will warn once if needed)
			return null

	# Format: integer or numeric string
	if uid_str.is_valid_int():
		return int(uid_str)

	# Not a CSV player (graduated or unknown format)
	return null


## Convert any accepted UID into engine PlayerId (csv: prefix format)
## Accepts: "123", 123, "csv:123", "csv_123"
## Returns: "csv:123" or "" if not convertible
## ✅ ENGINE UID SSOT (2025-12-23): Rust v2 API requires "csv:<u32>" format
static func _to_engine_player_id(uid: Variant) -> String:
	var csv_id = _parse_csv_id(uid)
	if csv_id == null:
		return ""
	return "csv:%d" % int(csv_id)


## Validate UID is in "csv:<u32>" format (for validation checks)
## Returns true if string matches "csv:123" pattern
static func _is_valid_csv_uid(uid_str: String) -> bool:
	if not uid_str.begins_with("csv:"):
		return false
	var num_part = uid_str.substr(4)
	return num_part.is_valid_int()


## Guess a reasonable center uid for fallback search.
## - If starters contain any numeric uid, use median-ish value
## - else default to 1
static func _guess_center_uid(team: _TeamSetup) -> int:
	var nums: Array = []
	for p in team.starters:
		if p == null:
			continue
		var s := str(p.uid).strip_edges()
		# accept "123" or "csv:123"
		if s.begins_with("csv:"):
			s = s.substr(4, s.length() - 4)
		if s.is_valid_int():
			nums.append(int(s))
	if nums.size() == 0:
		return 1
	nums.sort()
	return int(nums[int(nums.size() / 2)])


## Pick a valid engine uid near the given center using FootballRustEngine.has_player_uid.
static func _pick_valid_engine_uid_near(center: int, max_radius: int) -> String:
	var c := int(center)
	var rmax := int(max_radius)
	for r in range(rmax + 1):
		var a: int = clampi(c - r, 1, 1000000)
		var b: int = clampi(c + r, 1, 1000000)
		var ua := str(a)
		var ub := str(b)
		if _engine_has_uid(ua):
			return ua
		if _engine_has_uid(ub):
			return ub
	return ""


## Check if uid exists in engine cache via FootballRustEngine.has_player_uid()
static func _engine_has_uid(uid: String) -> bool:
	# NOTE:
	# - Avoid referencing the autoload singleton identifier directly (typed GDScript may treat it as undefined at compile time).
	# - Resolve via SceneTree/root to keep headless CI self-contained and deterministic.
	var main_loop: MainLoop = Engine.get_main_loop()
	if main_loop == null or not (main_loop is SceneTree):
		return false

	var root = (main_loop as SceneTree).get_root()
	if root == null:
		return false

	var n = root.get_node_or_null("/root/FootballRustEngine")
	if n and n.has_method("has_player_uid"):
		return bool(n.call("has_player_uid", uid))
	return false


## Validate engine payload before sending (Rust MatchSetup structure)
static func validate_payload(payload: Dictionary) -> Dictionary:
	var errors = []

	# Check Rust MatchSetup required fields
	if not payload.has("track_id"):
		errors.append("Missing track_id")

	if not payload.has("match_type"):
		errors.append("Missing match_type")

	if not payload.has("environment"):
		errors.append("Missing environment")

	# Check teams
	if not payload.has("home_team"):
		errors.append("Missing home_team")
	if not payload.has("away_team"):
		errors.append("Missing away_team")

	# Check TeamPreset structure (Rust format)
	if payload.has("home_team"):
		var home_team = payload.home_team
		if not home_team.has("id"):
			errors.append("Home team missing id")
		if not home_team.has("starting_xi"):
			errors.append("Home team missing starting_xi")
		else:
			var starting_xi = home_team.starting_xi
			if starting_xi.size() != 11:
				errors.append("Home team must have 11 starting_xi (got %d)" % starting_xi.size())
			else:
				# ✅ P0.5 ENGINE UID SSOT: Reject namespaced UIDs at engine boundary
				for pid in starting_xi:
					var s := str(pid)
					# ENFORCE: Reject non-engine UIDs (grad:*) and unsupported formats.
					# Accept: csv:<u32>, csv_<u32>, <u32>
					if s.begins_with("grad:"):
						errors.append("Home starting_xi contains non-engine uid at engine boundary: %s" % s)
					elif _parse_csv_id(s) == null:
						errors.append("Home starting_xi contains unsupported player uid: %s" % s)
		if not home_team.has("bench"):
			errors.append("Home team missing bench")
		else:
			var bench = home_team.bench
			if bench.size() != 7:
				errors.append("Home team must have 7 bench players (got %d)" % bench.size())
			else:
				# ✅ P0.5 ENGINE UID SSOT: Reject namespaced UIDs at engine boundary
				for pid in bench:
					var s := str(pid)
					# ENFORCE: Reject non-engine UIDs (grad:*) and unsupported formats.
					# Accept: csv:<u32>, csv_<u32>, <u32>
					if s.begins_with("grad:"):
						errors.append("Home bench contains non-engine uid at engine boundary: %s" % s)
					elif _parse_csv_id(s) == null:
						errors.append("Home bench contains unsupported player uid: %s" % s)

	if payload.has("away_team"):
		var away_team = payload.away_team
		if not away_team.has("id"):
			errors.append("Away team missing id")
		if not away_team.has("starting_xi"):
			errors.append("Away team missing starting_xi")
		else:
			var starting_xi = away_team.starting_xi
			if starting_xi.size() != 11:
				errors.append("Away team must have 11 starting_xi (got %d)" % starting_xi.size())
			else:
				# ✅ P0.5 ENGINE UID SSOT: Reject namespaced UIDs at engine boundary
				for pid in starting_xi:
					var s := str(pid)
					# ENFORCE: Reject non-engine UIDs (grad:*) and unsupported formats.
					# Accept: csv:<u32>, csv_<u32>, <u32>
					if s.begins_with("grad:"):
						errors.append("Away starting_xi contains non-engine uid at engine boundary: %s" % s)
					elif _parse_csv_id(s) == null:
						errors.append("Away starting_xi contains unsupported player uid: %s" % s)
		if not away_team.has("bench"):
			errors.append("Away team missing bench")
		else:
			var bench = away_team.bench
			if bench.size() != 7:
				errors.append("Away team must have 7 bench players (got %d)" % bench.size())
			else:
				# ✅ P0.5 ENGINE UID SSOT: Reject namespaced UIDs at engine boundary
				for pid in bench:
					var s := str(pid)
					# ENFORCE: Reject non-engine UIDs (grad:*) and unsupported formats.
					# Accept: csv:<u32>, csv_<u32>, <u32>
					if s.begins_with("grad:"):
						errors.append("Away bench contains non-engine uid at engine boundary: %s" % s)
					elif _parse_csv_id(s) == null:
						errors.append("Away bench contains unsupported player uid: %s" % s)

	return {"valid": errors.is_empty(), "errors": errors}


## Debug: Print UID parsing examples
static func debug_print_uid_parsing() -> void:
	print("[MatchSetupExporter] UID Parsing Examples:")
	print("  'csv:1234' → %s" % str(_parse_csv_id("csv:1234")))
	print("  'csv_1234' → %s" % str(_parse_csv_id("csv_1234")))
	print("  '1234' → %s" % str(_parse_csv_id("1234")))
	print("  1234 → %s" % str(_parse_csv_id(1234)))
	print("  'grad:xxx' → %s" % str(_parse_csv_id("grad:xxx")))


## Generate simple UUID v4 (simplified for Godot)
static func _generate_uuid() -> String:
	var timestamp = Time.get_ticks_msec()
	var random = randi()
	return "match_%d_%d" % [timestamp, random]


## Get RFC 3339 timestamp for Rust DateTime<Utc> compatibility
## Returns: "2025-12-23T10:30:45.123Z"
static func _get_rfc3339_timestamp() -> String:
	# Get current time as datetime dictionary
	var dt = Time.get_datetime_dict_from_system(true)  # UTC

	# Format as RFC 3339: YYYY-MM-DDTHH:MM:SS.sssZ
	return "%04d-%02d-%02dT%02d:%02d:%02d.000Z" % [dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second]
