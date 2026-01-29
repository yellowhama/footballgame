## PlayerLibrary - Unified player data source
## Part of Game OS v1.1 - P1.3 Binary Cache Integration
## Created: 2025-12-19
##
## PURPOSE:
## Unified player data access layer that wraps GameCache binary player system.
## Provides UID-based lookup and search capabilities for both CSV and graduated players.
##
## DATA FLOW:
## UID string ("csv:1234") → integer ID (1234) → GameCache.get_player(1234) → Player Dictionary
##
## UID Format:
## - CSV players: "csv:1234" (maps to GameCache UID 1-8345)
## - Graduated players: "grad:1234567890_0001" (timestamp_sequence, Phase 2)
##
## INTEGRATION:
## - Uses GameCache.gd autoload (loads cache_players.v3.msgpack.lz4 at boot)
## - DataCacheStore (Rust GDExtension) for binary cache access
## - No CSV parsing needed (binary cache pre-loaded)
##
## REFERENCES:
## - Plan: /home/hugh51/.claude/plans/quirky-questing-mountain.md (P1.3)
## - Binary cache: data/exports/cache_players.v3.msgpack.lz4 (8,345 players)
##
## Priority: P1 (Game OS v1.1)
class_name PlayerLibrary
extends RefCounted

## ============================================================================
## CONSTANTS
## ============================================================================

## Attributes schema version (P0.75-2: Full 36-attr injection)
const ATTRS_VERSION: int = 1

## GameCache binary cache size (8,345 players from FM 2023 CSV)
const PLAYER_CACHE_SIZE: int = 8345

## Default attribute ranges for missing data
const DEFAULT_MIN_ATTRIBUTE: int = 30
const DEFAULT_MAX_ATTRIBUTE: int = 90
const DEFAULT_JERSEY_NUMBER: int = 1

## ============================================================================
## STATE
## ============================================================================

## Reference to GameCache autoload (lazy loaded)
var _game_cache: Node = null

## Cache loaded flag
var _cache_ready: bool = false

## ============================================================================
## INITIALIZATION
## ============================================================================


## Initialize GameCache reference (lazy load)
func _ensure_cache_ready() -> bool:
	if _cache_ready:
		return true

	# Get GameCache autoload
	if _game_cache == null:
		if not Engine.has_singleton("GameCache"):
			var root = Engine.get_main_loop().root if Engine.get_main_loop() else null
			if root and root.has_node("/root/GameCache"):
				_game_cache = root.get_node("/root/GameCache")
			else:
				push_error("[PlayerLibrary] GameCache autoload not found")
				return false
		else:
			_game_cache = Engine.get_singleton("GameCache")

	# Check if player cache is ready
	if _game_cache and _game_cache.has_method("is_player_cache_ready"):
		# Best-effort: ask GameCache to load the player cache if it isn't ready yet.
		# This avoids match startup failures when GameCache boot order is late.
		if not _cache_ready and _game_cache.has_method("ensure_player_cache_loaded"):
			_game_cache.ensure_player_cache_loaded()

		_cache_ready = _game_cache.is_player_cache_ready()
		if not _cache_ready:
			push_warning("[PlayerLibrary] GameCache player cache not loaded yet")
			return false
	else:
		push_error("[PlayerLibrary] GameCache.is_player_cache_ready() not available")
		return false

	return true


## ============================================================================
## PUBLIC API
## ============================================================================


## Get player data by UID
func get_player_data(uid: String) -> Dictionary:
	if uid.begins_with("csv:"):
		return _get_csv_player(uid)
	elif uid.begins_with("grad:"):
		return _get_graduated_player(uid)
	else:
		push_warning("[PlayerLibrary] Unknown UID format: %s" % uid)
		return {}


## ============================================================================
## PRIVATE HELPERS
## ============================================================================


## Get CSV player from binary cache
func _get_csv_player(uid: String) -> Dictionary:
	# Ensure cache is loaded
	if not _ensure_cache_ready():
		push_error("[PlayerLibrary] Cannot load player - GameCache not ready: %s" % uid)
		return {}

	# Parse UID to integer
	var csv_id = _parse_csv_id(uid)
	if csv_id == null:
		push_error("[PlayerLibrary] Invalid CSV UID format: %s" % uid)
		return {}

	# Validate UID range (1-8345)
	if csv_id < 1 or csv_id > PLAYER_CACHE_SIZE:
		push_error("[PlayerLibrary] CSV UID out of range (1-%d): %d" % [PLAYER_CACHE_SIZE, csv_id])
		return {}

	# Get player from GameCache
	var player_data = _game_cache.get_player(csv_id)
	if player_data.is_empty():
		push_warning("[PlayerLibrary] Player not found in cache: %s (ID: %d)" % [uid, csv_id])
		return {}

	# Convert GameCache format to expected format
	return _convert_cache_to_player_data(uid, player_data)


## Graduated players (Phase 2 - not yet implemented)
## Falls back to binary cache for now
func _get_graduated_player(uid: String) -> Dictionary:
	push_warning("[PlayerLibrary] Graduated players not implemented (Phase 2) - falling back to CSV")
	# TODO: Load from graduated_players.json

	# Tier 1: Try to extract numeric ID from graduated UID
	var fallback_csv_id = _extract_numeric_from_grad_uid(uid)
	if fallback_csv_id > 0:
		print("[PlayerLibrary] Fallback: grad UID '%s' → csv:%d" % [uid, fallback_csv_id])
		return _get_csv_player("csv:%d" % fallback_csv_id)

	# Tier 2: Use hash-based CSV ID from binary cache
	var hash_id = (abs(uid.hash()) % PLAYER_CACHE_SIZE) + 1
	push_warning("[PlayerLibrary] Fallback: grad UID '%s' → hash-based csv:%d" % [uid, hash_id])
	return _get_csv_player("csv:%d" % hash_id)


## Convert GameCache format to expected player data format
##
## GameCache format: {uid, name, nationality, team, position, ca, pa, age}
## Expected format: {uid, name, position, preferred_position, overall, technical, mental, physical, jersey_number, condition, morale}
func _convert_cache_to_player_data(uid_string: String, cache_data: Dictionary) -> Dictionary:
	# Extract fields from GameCache
	var name = cache_data.get("name", "Unknown Player")
	var position = cache_data.get("position", "MF")
	var ca = cache_data.get("ca", 100)  # Current Ability (0-200 FM scale)
	var pa = cache_data.get("pa", 100)  # Potential Ability (0-200 FM scale)
	var age = cache_data.get("age", 25)
	var nationality = cache_data.get("nationality", "Unknown")
	var team = cache_data.get("team", "Free Agent")

	# Convert FM 2023 CA (0-200) to game overall rating (0-100)
	# CA 0-200 → Overall 0-100 (divide by 2)
	var overall = int(ca / 2.0)
	overall = clamp(overall, 0, 100)

	# -------------------------------------------------------------------------
	# ✅ P0.75-2 Contract: ALWAYS inject full PlayerAttributes (36) for Rust
	# - Deterministic: same uid => same attributes
	# - Position-aware: role bias
	# - Backward compatible: keep legacy technical/mental/physical summary scores
	# -------------------------------------------------------------------------
	var attrs: Dictionary = _derive_player_attributes(uid_string, position, ca, pa)
	var tech_attrs: Dictionary = attrs["technical"]
	var ment_attrs: Dictionary = attrs["mental"]
	var phys_attrs: Dictionary = attrs["physical"]

	# Legacy summary (keep UI expectations intact)
	var technical: int = int(_avg_dict(tech_attrs))
	var mental: int = int(_avg_dict(ment_attrs))
	var physical: int = int(_avg_dict(phys_attrs))

	# Generate jersey number (1-99)
	var csv_id = _parse_csv_id(uid_string)
	var jersey_number = (csv_id % 99) + 1 if csv_id != null else DEFAULT_JERSEY_NUMBER

	# Build player data dictionary
	return {
		"uid": uid_string,  # Keep original UID string ("csv:1234")
		"name": name,
		"position": position,
		"preferred_position": position,  # Use same position
		"overall": overall,
		# Legacy summary scores (UI/backward compatibility)
		"technical": technical,
		"mental": mental,
		"physical": physical,
		"jersey_number": jersey_number,
		"condition": 100,  # Default condition
		"morale": 50,  # Default morale
		# Extra metadata from binary cache
		"nationality": nationality,
		"team": team,
		"age": age,
		"ca": ca,  # FM 2023 Current Ability (0-200)
		"pa": pa,  # FM 2023 Potential Ability (0-200)
		# ✅ P0.75-2: Contract payload for Rust (36 attrs)
		# Rust expects Player.attributes = Some(PlayerAttributes) as FLAT dictionary
		# Flatten technical/mental/physical into single dict
		"attributes": _flatten_attributes(tech_attrs, ment_attrs, phys_attrs),
		"attributes_version": ATTRS_VERSION,
	}


## Extract numeric portion from graduated UID for fallback
##
## Examples:
##   - "grad:1734634567890_0001" → (1734634567890 % 8345) + 1 = 6891
##   - "grad:player_123" → 123 % 8345 = 123
func _extract_numeric_from_grad_uid(uid: String) -> int:
	if not uid.begins_with("grad:"):
		return 0

	var id_part = uid.substr(5)  # Remove "grad:"

	# Try to find numeric portion using RegEx
	var regex = RegEx.new()
	regex.compile("\\d+")
	var result = regex.search(id_part)

	if result:
		var num = int(result.get_string())
		# Map to binary cache range (1-8345)
		return (num % PLAYER_CACHE_SIZE) + 1

	return 0


## Parse CSV UID to integer ID
func _parse_csv_id(uid: String) -> Variant:
	if uid.begins_with("csv:"):
		var id_str = uid.substr(4)
		if id_str.is_valid_int():
			return int(id_str)
	return null


## Search players by name using GameCache
##
## PARAMETERS:
##   query: String - Search query (partial match, case-insensitive)
##   use_real_names: bool - Use real names instead of pseudonyms (default false)
##   max_results: int - Maximum number of results (0 = all, default 50)
##
## RETURNS:
##   Array of player Dictionaries (converted to expected format)
func search_players_by_name(query: String, use_real_names: bool = false, max_results: int = 50) -> Array:
	if not _ensure_cache_ready():
		push_error("[PlayerLibrary] Cannot search - GameCache not ready")
		return []

	# Use GameCache search_players method
	var results = _game_cache.search_players(query, use_real_names, max_results)

	# Convert each result to expected format
	var converted_results = []
	for player_data in results:
		# Extract UID from GameCache result
		var uid_int = player_data.get("uid", 0)
		var uid_string = "csv:%d" % uid_int

		# Convert to expected format
		var converted = _convert_cache_to_player_data(uid_string, player_data)
		converted_results.append(converted)

	return converted_results


## Search players by position rating (e.g., find best strikers)
##
## PARAMETERS:
##   position: String - Position code ("GK", "DC", "MC", "ST", etc.)
##   min_rating: int - Minimum position rating (0-20, default 15)
##   max_results: int - Maximum number of results (default 50)
##
## RETURNS:
##   Array of player Dictionaries with position_rating field
func find_best_players_for_position(position: String, min_rating: int = 15, max_results: int = 50) -> Array:
	if not _ensure_cache_ready():
		push_error("[PlayerLibrary] Cannot search - GameCache not ready")
		return []

	# Use GameCache find_best_players_for_position method
	var results = _game_cache.find_best_players_for_position(position, min_rating, max_results)

	# Convert each result to expected format
	var converted_results = []
	for player_data in results:
		var uid_int = player_data.get("uid", 0)
		var uid_string = "csv:%d" % uid_int

		# Convert and add position_rating field
		var converted = _convert_cache_to_player_data(uid_string, player_data)
		converted["position_rating"] = player_data.get("position_rating", 0)
		converted_results.append(converted)

	return converted_results


## Get players by team (exact match, case-sensitive)
##
## PARAMETERS:
##   team_name: String - Team name to filter by
##   use_real_names: bool - Use real team names instead of pseudonyms (default false)
##
## RETURNS:
##   Array of player Dictionaries
func get_players_by_team(team_name: String, use_real_names: bool = false) -> Array:
	if not _ensure_cache_ready():
		push_error("[PlayerLibrary] Cannot search - GameCache not ready")
		return []

	# Use GameCache get_players_by_team method
	var results = _game_cache.get_players_by_team(team_name, use_real_names)

	# Convert each result to expected format
	var converted_results = []
	for player_data in results:
		var uid_int = player_data.get("uid", 0)
		var uid_string = "csv:%d" % uid_int
		var converted = _convert_cache_to_player_data(uid_string, player_data)
		converted_results.append(converted)

	return converted_results


## Get balanced roster (position-aware selection)
##
## PARAMETERS:
##   min_ca: int - Minimum current ability (0-200, FM scale)
##   max_players: int - Roster size (default 18)
##   use_real_names: bool - Use real names instead of pseudonyms (default false)
##
## RETURNS:
##   Array of player Dictionaries (2 GK, 6 DF, 6 MF, 4 FW)
func get_balanced_roster(min_ca: int = 0, max_players: int = 18, use_real_names: bool = false) -> Array:
	if not _ensure_cache_ready():
		push_error("[PlayerLibrary] Cannot generate roster - GameCache not ready")
		return []

	# Use GameCache get_balanced_roster method
	var results = _game_cache.get_balanced_roster(min_ca, max_players, use_real_names)

	# Convert each result to expected format
	var converted_results = []
	for player_data in results:
		var uid_int = player_data.get("uid", 0)
		var uid_string = "csv:%d" % uid_int

		# Convert and preserve position_group field
		var converted = _convert_cache_to_player_data(uid_string, player_data)
		converted["position_group"] = player_data.get("position_group", "MF")
		converted["primary_position"] = player_data.get("primary_position", "MF")
		converted_results.append(converted)

	return converted_results


## List all graduated players (Phase 2 - not implemented)
func list_graduated_players() -> Array:
	push_warning("[PlayerLibrary] Graduated players not implemented (Phase 2)")
	# TODO: Load from graduated_players.json
	return []


## Generate random roster UIDs for testing
##
## PARAMETERS:
##   count: int - Number of players (default 18)
##   min_id: int - Minimum UID (default 1)
##   max_id: int - Maximum UID (default 8345)
##
## RETURNS:
##   Array of UID strings ["csv:1234", "csv:5678", ...]
func generate_random_roster(count: int = 18, min_id: int = 1, max_id: int = PLAYER_CACHE_SIZE) -> Array:
	var roster = []
	for i in range(count):
		var csv_id = randi_range(min_id, max_id)
		roster.append("csv:%d" % csv_id)
	return roster


## ============================================================================
## P0.75-2 Contract Helpers: deterministic 36-attr injection
## ============================================================================


## Derive full PlayerAttributes (36 fields) from FM CA/PA and position
##
## RETURNS:
##   Dictionary with keys: { "technical": {...14}, "mental": {...14}, "physical": {...8} }
func _derive_player_attributes(uid_string: String, position: String, ca: int, _pa: int) -> Dictionary:
	# Deterministic seed from uid string
	var uid_seed: int = _uid_to_seed(uid_string)
	var rng := RandomNumberGenerator.new()
	rng.seed = uid_seed

	# Normalize CA (FM 0..200) -> (1..99)
	var base: float = clamp(float(ca) / 2.0, 1.0, 99.0)

	# Position bucket (simple)
	var pos := position.strip_edges().to_upper()
	var profile := _position_profile(pos)

	# Category baselines from CA + profile bias
	var base_tech: float = clamp(base * profile["tech_mul"], 1.0, 99.0)
	var base_ment: float = clamp(base * profile["ment_mul"], 1.0, 99.0)
	var base_phys: float = clamp(base * profile["phys_mul"], 1.0, 99.0)

	# Spread controls (tightness)
	var spread_tech: float = 10.0
	var spread_ment: float = 10.0
	var spread_phys: float = 10.0

	# Build dicts with exact Rust keys
	var technical := {
		"corners": _roll_attr(rng, base_tech, spread_tech),
		"crossing": _roll_attr(rng, base_tech, spread_tech),
		"dribbling": _roll_attr(rng, base_tech, spread_tech),
		"finishing": _roll_attr(rng, base_tech, spread_tech),
		"first_touch": _roll_attr(rng, base_tech, spread_tech),
		"free_kicks": _roll_attr(rng, base_tech, spread_tech),
		"heading": _roll_attr(rng, base_tech, spread_tech),
		"long_shots": _roll_attr(rng, base_tech, spread_tech),
		"long_throws": _roll_attr(rng, base_tech, spread_tech),
		"marking": _roll_attr(rng, base_tech, spread_tech),
		"passing": _roll_attr(rng, base_tech, spread_tech),
		"penalty_taking": _roll_attr(rng, base_tech, spread_tech),
		"tackling": _roll_attr(rng, base_tech, spread_tech),
		"technique": _roll_attr(rng, base_tech, spread_tech),
	}

	var mental := {
		"aggression": _roll_attr(rng, base_ment, spread_ment),
		"anticipation": _roll_attr(rng, base_ment, spread_ment),
		"bravery": _roll_attr(rng, base_ment, spread_ment),
		"composure": _roll_attr(rng, base_ment, spread_ment),
		"concentration": _roll_attr(rng, base_ment, spread_ment),
		"decisions": _roll_attr(rng, base_ment, spread_ment),
		"determination": _roll_attr(rng, base_ment, spread_ment),
		"flair": _roll_attr(rng, base_ment, spread_ment),
		"leadership": _roll_attr(rng, base_ment, spread_ment),
		"off_the_ball": _roll_attr(rng, base_ment, spread_ment),
		"positioning": _roll_attr(rng, base_ment, spread_ment),
		"teamwork": _roll_attr(rng, base_ment, spread_ment),
		"vision": _roll_attr(rng, base_ment, spread_ment),
		"work_rate": _roll_attr(rng, base_ment, spread_ment),
	}

	var physical := {
		"acceleration": _roll_attr(rng, base_phys, spread_phys),
		"agility": _roll_attr(rng, base_phys, spread_phys),
		"balance": _roll_attr(rng, base_phys, spread_phys),
		"jumping": _roll_attr(rng, base_phys, spread_phys),
		"natural_fitness": _roll_attr(rng, base_phys, spread_phys),
		"pace": _roll_attr(rng, base_phys, spread_phys),
		"stamina": _roll_attr(rng, base_phys, spread_phys),
		"strength": _roll_attr(rng, base_phys, spread_phys),
	}

	# Apply role emphasis (adds/subtracts small deterministic deltas)
	_apply_emphasis(rng, technical, mental, physical, profile)

	return {
		"technical": technical,
		"mental": mental,
		"physical": physical,
	}


## Convert UID string to deterministic integer seed
## Accepts: "csv:123", "csv_123", "123"
func _uid_to_seed(uid_string: String) -> int:
	var s := uid_string.strip_edges()
	if s.begins_with("csv:"):
		s = s.substr(4)
	elif s.begins_with("csv_"):
		s = s.substr(4)
	s = s.strip_edges()
	if s.is_valid_int():
		return int(s)
	# fallback stable hash
	return abs(s.hash())


## Roll single attribute with deterministic variance
## Uses pseudo-normal distribution (sum of 3 uniforms)
func _roll_attr(rng: RandomNumberGenerator, base_val: float, spread: float) -> int:
	# deterministic pseudo-normal using sum of uniforms
	var u := (rng.randf() + rng.randf() + rng.randf()) / 3.0  # ~0..1
	var delta := (u - 0.5) * 2.0 * spread
	return int(clamp(base_val + delta, 1.0, 99.0))


## Calculate average value from dictionary
func _avg_dict(d: Dictionary) -> float:
	if d.is_empty():
		return 50.0
	var sum := 0.0
	for k in d.keys():
		sum += float(d[k])
	return sum / float(d.size())


## Flatten technical/mental/physical dicts into single flat dict for Rust
## Rust PlayerAttributes expects all 36 fields at top level
func _flatten_attributes(tech: Dictionary, ment: Dictionary, phys: Dictionary) -> Dictionary:
	var flat := {}
	# Merge all three categories
	for k in tech.keys():
		flat[k] = tech[k]
	for k in ment.keys():
		flat[k] = ment[k]
	for k in phys.keys():
		flat[k] = phys[k]
	return flat


## Get position profile (category multipliers + emphasis map)
##
## RETURNS:
##   Dictionary with keys: { "tech_mul": float, "ment_mul": float, "phys_mul": float, "emphasis": {...} }
func _position_profile(pos: String) -> Dictionary:
	# Minimal buckets (tune later)
	# Keys: tech_mul, ment_mul, phys_mul, emphasis (Dictionary attr->delta)
	var p := pos

	# GK - Goalkeeper
	if p.find("GK") != -1:
		return {
			"tech_mul": 0.85,
			"ment_mul": 1.00,
			"phys_mul": 0.95,
			"emphasis":
			{
				"technical": {"passing": -3, "finishing": -6, "tackling": -2},
				"mental": {"composure": 4, "decisions": 3, "positioning": 3},
				"physical": {"agility": 2, "balance": 2}
			}
		}

	# CB/DC - Center Back
	if p.find("CB") != -1 or p.find("DC") != -1:
		return {
			"tech_mul": 0.95,
			"ment_mul": 1.00,
			"phys_mul": 1.05,
			"emphasis":
			{
				"technical": {"marking": 5, "tackling": 5, "heading": 4},
				"mental": {"positioning": 4, "bravery": 3, "anticipation": 2},
				"physical": {"strength": 3, "jumping": 3}
			}
		}

	# DM - Defensive Midfielder
	if p.find("DM") != -1:
		return {
			"tech_mul": 1.00,
			"ment_mul": 1.05,
			"phys_mul": 1.00,
			"emphasis":
			{
				"technical": {"tackling": 3, "passing": 3, "marking": 3},
				"mental": {"decisions": 3, "positioning": 3, "work_rate": 2},
				"physical": {"stamina": 2}
			}
		}

	# CM - Central Midfielder
	if p.find("CM") != -1 or p.find("MC") != -1:
		return {
			"tech_mul": 1.05,
			"ment_mul": 1.05,
			"phys_mul": 1.00,
			"emphasis":
			{
				"technical": {"passing": 5, "first_touch": 2},
				"mental": {"vision": 4, "decisions": 3, "teamwork": 2},
				"physical": {"stamina": 1}
			}
		}

	# AM/CAM - Attacking Midfielder
	if p.find("AM") != -1 or p.find("CAM") != -1:
		return {
			"tech_mul": 1.08,
			"ment_mul": 1.05,
			"phys_mul": 0.98,
			"emphasis":
			{
				"technical": {"dribbling": 4, "technique": 3, "passing": 3},
				"mental": {"flair": 4, "vision": 3, "off_the_ball": 2},
				"physical": {"acceleration": 1}
			}
		}

	# LW/RW/LM/RM - Wingers
	if p.find("LW") != -1 or p.find("RW") != -1 or p.find("LM") != -1 or p.find("RM") != -1:
		return {
			"tech_mul": 1.05,
			"ment_mul": 1.00,
			"phys_mul": 1.05,
			"emphasis":
			{
				"technical": {"crossing": 4, "dribbling": 3, "first_touch": 2},
				"mental": {"flair": 2, "off_the_ball": 2},
				"physical": {"pace": 3, "acceleration": 2}
			}
		}

	# ST/CF - Striker
	if p.find("ST") != -1 or p.find("CF") != -1 or p.find("FW") != -1:
		return {
			"tech_mul": 1.08,
			"ment_mul": 1.00,
			"phys_mul": 1.02,
			"emphasis":
			{
				"technical": {"finishing": 6, "first_touch": 3, "technique": 2},
				"mental": {"composure": 4, "off_the_ball": 3},
				"physical": {"acceleration": 2, "pace": 2}
			}
		}

	# fallback (MF default)
	return {"tech_mul": 1.0, "ment_mul": 1.0, "phys_mul": 1.0, "emphasis": {}}


## Apply position-specific emphasis to attributes (mutates dictionaries)
func _apply_emphasis(
	_rng: RandomNumberGenerator, technical: Dictionary, mental: Dictionary, physical: Dictionary, profile: Dictionary
) -> void:
	var em: Dictionary = profile.get("emphasis", {})

	if em.has("technical"):
		for k in em["technical"].keys():
			if technical.has(k):
				technical[k] = int(clamp(float(technical[k]) + float(em["technical"][k]), 1.0, 99.0))

	if em.has("mental"):
		for k in em["mental"].keys():
			if mental.has(k):
				mental[k] = int(clamp(float(mental[k]) + float(em["mental"][k]), 1.0, 99.0))

	if em.has("physical"):
		for k in em["physical"].keys():
			if physical.has(k):
				physical[k] = int(clamp(float(physical[k]) + float(em["physical"][k]), 1.0, 99.0))
