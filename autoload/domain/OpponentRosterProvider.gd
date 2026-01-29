extends Node

# ============================================================================
# OpponentRosterProvider
# ============================================================================
# Phase B: Dedicated autoload for opponent roster generation
#
# Responsibilities:
# - Manage static CSV cache (parse once, use forever)
# - Provide balanced roster based on CA requirements
# - Fall back to CSV when DLL is unavailable
# - NO instance caching (stateless per request)
#
# Related Issues: MASTER_FIX_PLAN.md Phase B
# ============================================================================

const OPPONENT_CSV_PATH := "res://data/players_with_pseudonym.csv"
const USE_SMALL_DATA_ENV := "OPPONENT_TOP20_ONLY"  # set to 1/true to use small curated set

# Small curated set of top teams (name, base rating)
const TOP20_TEAMS := [
	{"name": "Man Town", "rating": 85},
	{"name": "Real Royal", "rating": 84},
	{"name": "Blue Bridge", "rating": 83},
	{"name": "Mun Reds", "rating": 83},
	{"name": "Paris Stars", "rating": 82},
	{"name": "Turin White", "rating": 82},
	{"name": "Catalan FC", "rating": 82},
	{"name": "Mun Blue", "rating": 81},
	{"name": "North Reds", "rating": 81},
	{"name": "Milan Black", "rating": 80},
	{"name": "Milan Red", "rating": 80},
	{"name": "London Gunners", "rating": 80},
	{"name": "London Spurs", "rating": 79},
	{"name": "Amsterdam Ajax", "rating": 79},
	{"name": "Lisbon Eagles", "rating": 78},
	{"name": "Dort Blue", "rating": 78},
	{"name": "Leipzig RB", "rating": 78},
	{"name": "Porto Dragons", "rating": 77},
	{"name": "Sevilla Red", "rating": 77},
	{"name": "Napoli", "rating": 77},
]

const STAGE_POSITION_TEMPLATE := [
	"GK", "RB", "LB", "CB", "CB", "DM", "CM", "LM", "RM", "ST", "ST", "GK", "CB", "CB", "FB", "AM", "CM", "ST"
]
const STAGE_POSITION_SYNONYMS := {
	"GOALKEEPER": "GK",
	"KEEPER": "GK",
	"GOALIE": "GK",
	"DEFENDER": "CB",
	"FULLBACK": "FB",
	"LEFTBACK": "LB",
	"RIGHTBACK": "RB",
	"MIDFIELDER": "CM",
	"FORWARD": "ST",
	"STRIKER": "ST"
}
const STAGE_VALID_POSITIONS := ["GK", "CB", "RB", "LB", "FB", "DM", "CM", "AM", "LM", "RM", "RW", "LW", "ST", "CF"]

var _last_stage_team: Dictionary = {}
var _last_stage_manager: Dictionary = {}

# ============================================================================
# STATIC CSV CACHE
# ============================================================================
# Parse CSV once at first match, reuse for all future matches
static var _csv_parsed_teams: Dictionary = {}  # team_name → Array[player_dict]
static var _csv_cache_initialized: bool = false

# ============================================================================
# PUBLIC API
# ============================================================================


## Get a balanced opponent roster based on CA requirements
## @param min_ca: Minimum current ability for team selection
## @param max_players: Maximum number of players to return (default 18)
## @param use_real_names: Whether to use real player/team names
## @return Array of player dictionaries (empty if failed)
func get_opponent_roster(min_ca: int = 50, max_players: int = 18, use_real_names: bool = false) -> Array:
	print(
		(
			"[OpponentRosterProvider] 🔍 Requesting roster: min_ca=%d, max_players=%d, use_real_names=%s"
			% [min_ca, max_players, use_real_names]
		)
	)

	# Strategy 0: Use StageManager teams in order (easiest to hardest)
	var stage_roster := _get_roster_from_stage_manager(max_players)
	if not stage_roster.is_empty():
		return stage_roster

	# Optional small-data mode for fast, schema-safe rosters (default ON in debug builds)
	if _use_small_set_default():
		return _get_roster_from_small_set(min_ca, max_players)

	# ========================================================================
	# Strategy 1: Try DLL-based roster generation (if available)
	# ========================================================================
	if GameCache.player_cache_loaded:
		var dll_roster := _get_roster_from_dll(min_ca, max_players, use_real_names)
		if not dll_roster.is_empty():
			print("[OpponentRosterProvider] ✅ Generated roster from DLL: %d players" % dll_roster.size())
			return dll_roster

	# ========================================================================
	# Strategy 2: Use CSV cache (fallback)
	# ========================================================================
	print("[OpponentRosterProvider] ⚠️ DLL unavailable, falling back to CSV")
	return _get_roster_from_csv(use_real_names, max_players)


func _get_roster_from_stage_manager(max_players: int) -> Array:
	# Get StageManager autoload
	var stage_manager = get_node_or_null("/root/StageManager")
	if stage_manager == null:
		print("[OpponentRosterProvider] ⚠️ StageManager not found")
		_last_stage_team = {}
		_last_stage_manager = {}
		return []

	# Check if stage teams are loaded
	if stage_manager.stage_teams.is_empty():
		print("[OpponentRosterProvider] ⚠️ StageManager has no teams loaded")
		_last_stage_team = {}
		_last_stage_manager = {}
		return []

	var team_data: Dictionary = {}
	var stage_index: int = -1
	if stage_manager.has_method("get_next_league_team"):
		var preferred_league: int = -1
		if stage_manager.has_method("get_active_league"):
			preferred_league = int(stage_manager.get_active_league())
		team_data = stage_manager.get_next_league_team(preferred_league)

	if team_data.is_empty():
		stage_index = stage_manager.current_stage - 1
		if stage_index < 0 or stage_index >= stage_manager.stage_teams.size():
			stage_index = 0
		team_data = stage_manager.stage_teams[stage_index]

	_last_stage_team = team_data.duplicate(true)
	_last_stage_manager = {}
	if stage_manager.has_method("get_manager_data_for_team"):
		var stage_manager_payload: Dictionary = stage_manager.get_manager_data_for_team(team_data)
		if stage_manager_payload is Dictionary and not stage_manager_payload.is_empty():
			_last_stage_manager = stage_manager_payload

	var squad: Array = team_data.get("squad", [])

	if squad.is_empty():
		print("[OpponentRosterProvider] ⚠️ Team %s has no squad" % team_data.get("club_name", "Unknown"))
		return []
	var league_id := str(team_data.get("league_id", "stage"))
	print(
		(
			"[OpponentRosterProvider] 🏆 Stage team selected: %s (League: %s, CA: %.1f)"
			% [team_data.get("club_name", "Unknown"), league_id, float(team_data.get("avg_ca", 0.0))]
		)
	)

	# Convert squad to expected format (preserve nested structure for _ensure_player_attribute_map)
	var position_usage := {}
	for member in squad:
		var canonical := _canonicalize_stage_position(String(member.get("position", "")))
		if canonical != "":
			position_usage[canonical] = true
	var has_goalkeeper := position_usage.has("GK")
	var has_forward := (
		position_usage.has("ST")
		or position_usage.has("CF")
		or position_usage.has("FW")
		or position_usage.has("LW")
		or position_usage.has("RW")
	)
	var has_defender := (
		position_usage.has("CB") or position_usage.has("RB") or position_usage.has("LB") or position_usage.has("FB")
	)
	var use_template_positions := position_usage.size() < 6 or not has_goalkeeper or not has_forward or not has_defender
	if use_template_positions:
		print(
			(
				"[OpponentRosterProvider] ⚠️ Stage roster missing variety (GK:%s DEF:%s FWD:%s unique:%d) → applying template positions"
				% [has_goalkeeper, has_defender, has_forward, position_usage.size()]
			)
		)

	var roster: Array = []
	for i in range(min(max_players, squad.size())):
		var player: Dictionary = squad[i]
		var tech_src: Dictionary = player.get("technical", {})
		var mental_src: Dictionary = player.get("mental", {})
		var phys_src: Dictionary = player.get("physical", {})

		var ca_value := int(player.get("ca", 50))
		var default_attr := clampi(int(round(float(ca_value))), 5, 120)
		var assigned_position := _resolve_stage_position(
			String(player.get("position", "CM")), i, use_template_positions
		)

		var short_pass := int(tech_src.get("short_passing", tech_src.get("long_passing", default_attr)))
		var long_pass := int(tech_src.get("long_passing", short_pass))
		var pass_avg := int(round((short_pass + long_pass) / 2.0))
		var crossing := int(tech_src.get("crossing", pass_avg))
		var dribbling := int(tech_src.get("dribbling", pass_avg))
		var finishing := int(tech_src.get("finishing", pass_avg))
		var heading := int(tech_src.get("heading_accuracy", pass_avg))
		var control := int(tech_src.get("ball_control", dribbling))
		var curve := int(tech_src.get("curve", crossing))
		var fk_acc := int(tech_src.get("free_kick_accuracy", curve))
		var volleys := int(tech_src.get("volleys", finishing))
		var long_shots := int(tech_src.get("long_shots", volleys))
		var marking_base := int(mental_src.get("positioning", default_attr))
		var tackling_base := int(mental_src.get("aggression", marking_base))

		var mental_default := int(mental_src.get("vision", default_attr))
		var aggression := int(mental_src.get("aggression", mental_default))
		var anticipation := int(mental_src.get("anticipation", mental_default))
		var determination := int(mental_src.get("determination", mental_default))
		var teamwork := int(mental_src.get("teamwork", mental_default))
		var positioning := int(mental_src.get("positioning", mental_default))
		var work_rate := int(mental_src.get("work_rate", mental_default))
		var vision := int(mental_src.get("vision", mental_default))
		var concentration := int(mental_src.get("concentration", mental_default))
		var decisions := int(mental_src.get("decisions", mental_default))
		var composure := int(mental_src.get("composure", mental_default))

		var phys_default := int(phys_src.get("pace", default_attr))
		var acceleration := int(phys_src.get("acceleration", phys_default))
		var agility := int(phys_src.get("agility", phys_default))
		var balance := int(phys_src.get("balance", phys_default))
		var jumping := int(phys_src.get("jumping", phys_default))
		var natural_fit := int(phys_src.get("natural_fitness", phys_default))
		var pace := int(phys_src.get("pace", phys_default))
		var stamina := int(phys_src.get("stamina", phys_default))
		var strength := int(phys_src.get("strength", phys_default))

		var technical := {
			"corners": curve,
			"crossing": crossing,
			"dribbling": dribbling,
			"finishing": finishing,
			"first_touch": control,
			"free_kicks": fk_acc,
			"heading": heading,
			"long_shots": long_shots,
			"long_throws": long_pass,
			"marking": marking_base,
			"passing": pass_avg,
			"penalty_taking": max(finishing, fk_acc),
			"tackling": tackling_base,
			"technique": control,
		}

		var mental := {
			"aggression": aggression,
			"anticipation": anticipation,
			"bravery": max(determination, aggression),
			"composure": composure,
			"concentration": concentration,
			"decisions": decisions,
			"determination": determination,
			"flair": vision,
			"leadership": max(teamwork, determination),
			"off_the_ball": int(round((vision + work_rate) / 2.0)),
			"positioning": positioning,
			"teamwork": teamwork,
			"vision": vision,
			"work_rate": work_rate,
		}

		var physical := {
			"acceleration": acceleration,
			"agility": agility,
			"balance": balance,
			"jumping": jumping,
			"natural_fitness": natural_fit,
			"pace": pace,
			"stamina": stamina,
			"strength": strength,
		}

		var goalkeeper_block := {
			"aerial_ability": jumping,
			"command_of_area": positioning,
			"communication": teamwork,
			"handling": control,
			"kicking": pass_avg,
			"reflexes": agility,
		}

		var converted := {
			"name": String(player.get("name", "Player %d" % (i + 1))),
			"position": assigned_position,
			"overall": ca_value,
			"ca": ca_value,
			"pa": int(player.get("pa", ca_value)),
			"age": int(player.get("age", 20)),
			"technical": technical,
			"mental": mental,
			"physical": physical,
			"goalkeeper": goalkeeper_block,
		}

		roster.append(converted)

	var team_name: String = String(team_data.get("club_name", "Unknown"))
	var avg_ca: float = float(team_data.get("avg_ca", 50.0))
	var stage_label: int = stage_index + 1 if stage_index >= 0 else stage_manager.current_stage
	print(
		(
			"[OpponentRosterProvider] ✅ Generated roster from StageManager: %d players (Stage %d: %s, CA %.1f)"
			% [roster.size(), stage_label, team_name, avg_ca]
		)
	)
	var position_summary: Array = []
	for entry in roster:
		var pos_label := String(entry.get("position", ""))
		if pos_label not in position_summary:
			position_summary.append(pos_label)
	print(
		(
			"[OpponentRosterProvider] Position spread: %s (template_used=%s)"
			% [", ".join(position_summary), str(use_template_positions)]
		)
	)

	return roster


func get_last_stage_team_payload() -> Dictionary:
	"""Expose the latest StageManager team payload used to build a roster"""
	return _last_stage_team.duplicate(true)


func get_last_stage_manager_data() -> Dictionary:
	"""Expose the StageManager manager payload for the last roster selection"""
	return _last_stage_manager.duplicate(true)


func _get_roster_from_small_set(min_ca: int, max_players: int) -> Array:
	# Pick a team at or above the requested CA, or highest below if none
	var candidates: Array = []
	for t in TOP20_TEAMS:
		if int(t.rating) >= min_ca:
			candidates.append(t)
	if candidates.is_empty():
		candidates = TOP20_TEAMS.duplicate()
	var team: Dictionary = candidates[randi() % candidates.size()] as Dictionary
	var rating: int = int(team.rating)

	# Build a 4-4-2 roster: 11 starters + bench up to max_players
	var starters := ["GK", "CB", "CB", "RB", "LB", "CM", "CM", "RM", "LM", "ST", "ST"]
	var bench := ["GK", "CB", "FB", "DM", "CM", "AM", "ST"]
	var positions: Array = starters + bench
	var roster: Array = []
	for i in range(min(max_players, positions.size())):
		var pos: String = String(positions[i])
		var ov: int = clampi(rating + (randi() % 5) - 2, 60, 95)
		(
			roster
			. append(
				{
					"name": "%s %s %02d" % [String(team.name), pos, i + 1],
					"position": pos,
					"overall": ov,
				}
			)
		)

	print(
		(
			"[OpponentRosterProvider] ✅ Generated roster from SMALL SET: %d players (team: %s, rating: %d)"
			% [roster.size(), team.name, rating]
		)
	)
	return roster


func _canonicalize_stage_position(raw_position: String) -> String:
	var normalized := raw_position.strip_edges().to_upper()
	if normalized == "":
		return ""
	if STAGE_POSITION_SYNONYMS.has(normalized):
		return STAGE_POSITION_SYNONYMS[normalized]
	return normalized


func _resolve_stage_position(raw_position: String, slot_index: int, use_template: bool) -> String:
	var canonical := _canonicalize_stage_position(raw_position)
	if canonical == "GK":
		return "GK"
	if not use_template and STAGE_VALID_POSITIONS.has(canonical):
		return canonical
	if use_template:
		return STAGE_POSITION_TEMPLATE[slot_index % STAGE_POSITION_TEMPLATE.size()]
	if STAGE_VALID_POSITIONS.has(canonical):
		return canonical
	return STAGE_POSITION_TEMPLATE[slot_index % STAGE_POSITION_TEMPLATE.size()]


func _use_small_set_default() -> bool:
	# Priority: env var (highest) → ProjectSettings → default (debug builds)
	if OS.has_environment(USE_SMALL_DATA_ENV):
		var val := OS.get_environment(USE_SMALL_DATA_ENV).strip_edges().to_lower()
		if val in ["1", "true", "yes"]:
			return true
		if val in ["0", "false", "no"]:
			return false
	# ProjectSettings toggle (app/dev/use_small_opponents)
	if ProjectSettings.has_setting("app/dev/use_small_opponents"):
		var ps_val := str(ProjectSettings.get_setting("app/dev/use_small_opponents")).strip_edges().to_lower()
		if ps_val in ["1", "true", "yes"]:
			return true
		if ps_val in ["0", "false", "no"]:
			return false
	# Default behavior: enable in debug/editor builds for stability & speed
	return OS.is_debug_build()


## Clear the static CSV cache (useful for testing or reloading)
func clear_cache() -> void:
	_csv_parsed_teams.clear()
	_csv_cache_initialized = false
	print("[OpponentRosterProvider] 🗑️ Cache cleared")


## Get cache statistics for debugging
func get_cache_stats() -> Dictionary:
	return {
		"initialized": _csv_cache_initialized,
		"teams_count": _csv_parsed_teams.size(),
		"dll_available": GameCache.player_cache_loaded
	}


# ============================================================================
# PRIVATE: DLL-based roster generation
# ============================================================================


func _get_roster_from_dll(min_ca: int, max_players: int, use_real_names: bool) -> Array:
	"""Try to generate roster using DLL functions"""

	# Check if get_balanced_roster exists (Phase 3: after Rust build)
	if GameCache.data_cache_store != null and GameCache.data_cache_store.has_method("get_balanced_roster"):
		print("[OpponentRosterProvider] 📦 Using DLL get_balanced_roster()")
		return GameCache.data_cache_store.get_balanced_roster(min_ca, max_players, use_real_names)

	# Fallback: Use wonderkids/world_class functions (limited CA ranges)
	return _get_roster_from_dll_fallback(min_ca, max_players)


func _get_roster_from_dll_fallback(min_ca: int, max_players: int) -> Array:
	"""Use wonderkids/world_class as poor man's roster generator"""

	if GameCache.data_cache_store == null:
		return []

	# High CA: use world_class_players
	if min_ca >= 170:
		if GameCache.data_cache_store.has_method("get_world_class_players"):
			print("[OpponentRosterProvider] 🌟 Using world_class_players for high CA")
			var players = GameCache.data_cache_store.get_world_class_players(max_players)
			return players if players is Array else []

	# Low CA: use wonderkids
	elif min_ca <= 60:
		if GameCache.data_cache_store.has_method("get_wonderkids"):
			print("[OpponentRosterProvider] 👶 Using wonderkids for low CA")
			var players = GameCache.data_cache_store.get_wonderkids(max_players)
			return players if players is Array else []

	# Mid CA: no good option in current DLL
	print("[OpponentRosterProvider] ⚠️ Mid-range CA (%d) not supported by DLL fallback" % min_ca)
	return []


# ============================================================================
# PRIVATE: CSV-based roster generation
# ============================================================================


func _get_roster_from_csv(use_real_names: bool, max_players: int) -> Array:
	"""Get roster from CSV (with caching) — formation-aware 4-4-2 selection"""

	# Initialize cache if needed
	if not _csv_cache_initialized:
		_initialize_csv_cache(use_real_names)

	# Check if cache is ready
	if _csv_parsed_teams.is_empty():
		push_error("[OpponentRosterProvider] ❌ CSV cache failed to initialize")
		return []

	# Pick a random team from cache
	var team_name: String = _select_team_from_cache()
	if team_name.is_empty():
		push_error("[OpponentRosterProvider] ❌ No valid team found in cache")
		return []

	# Get players from selected team
	var team_players: Array = _csv_parsed_teams[team_name]

	# Partition by coarse categories
	var gks: Array = []
	var dfs: Array = []
	var mfs: Array = []
	var fws: Array = []
	for p in team_players:
		if typeof(p) != TYPE_DICTIONARY:
			continue
		var pd: Dictionary = p
		var pos_raw: String = String(pd.get("position", "MF"))
		var cat: String = _csv_position_to_category(pos_raw)
		match cat:
			"GK":
				gks.append(pd)
			"DF":
				dfs.append(pd)
			"MF":
				mfs.append(pd)
			"FW":
				fws.append(pd)
			_:
				mfs.append(pd)

	# Sort each bucket by overall descending (fallback overall=50)
	gks.sort_custom(Callable(self, "_cmp_overall_desc"))
	dfs.sort_custom(Callable(self, "_cmp_overall_desc"))
	mfs.sort_custom(Callable(self, "_cmp_overall_desc"))
	fws.sort_custom(Callable(self, "_cmp_overall_desc"))

	# Quotas for 4-4-2 starters
	var need_gk: int = 2
	var need_df: int = 4
	var need_mf: int = 4
	var need_fw: int = 2

	var roster: Array = []

	# Take per quota using helper
	_take_from_list(roster, gks, need_gk)
	_take_from_list(roster, dfs, need_df)
	_take_from_list(roster, mfs, need_mf)
	_take_from_list(roster, fws, need_fw)

	# Ensure 11 starters; fill deficits from nearest categories (GK handled later by normalizer if still short)
	while roster.size() < 11:
		if fws.size() > 0:
			var it = fws[0]
			fws.remove_at(0)
			roster.append((it as Dictionary).duplicate(true))
		elif mfs.size() > 0:
			var it2 = mfs[0]
			mfs.remove_at(0)
			roster.append((it2 as Dictionary).duplicate(true))
		elif dfs.size() > 0:
			var it3 = dfs[0]
			dfs.remove_at(0)
			roster.append((it3 as Dictionary).duplicate(true))
		elif gks.size() > 0:
			var it4 = gks[0]
			gks.remove_at(0)
			roster.append((it4 as Dictionary).duplicate(true))
		else:
			break

	# Fill bench up to max_players with best remaining (prefer outfielders first)
	var pool_remain: Array = []
	for x in dfs:
		pool_remain.append(x)
	for x in mfs:
		pool_remain.append(x)
	for x in fws:
		pool_remain.append(x)
	for x in gks:
		pool_remain.append(x)
	pool_remain.sort_custom(Callable(self, "_cmp_overall_desc"))

	while roster.size() < max_players and pool_remain.size() > 0:
		var itp = pool_remain[0]
		pool_remain.remove_at(0)
		roster.append((itp as Dictionary).duplicate(true))

	if roster.size() > max_players:
		roster = roster.slice(0, max_players)

	print("[OpponentRosterProvider] ✅ Generated roster from CSV: %d players (team: %s)" % [roster.size(), team_name])
	return roster


func _take_from_list(roster: Array, from_list: Array, n: int) -> void:
	var k: int = min(n, from_list.size())
	for i in range(k):
		roster.append((from_list[i] as Dictionary).duplicate(true))
	for _i in range(k):
		from_list.remove_at(0)


func _cmp_overall_desc(a, b) -> bool:
	var oa: int = 50
	var ob: int = 50
	if typeof(a) == TYPE_DICTIONARY:
		oa = int((a as Dictionary).get("overall", 50))
	if typeof(b) == TYPE_DICTIONARY:
		ob = int((b as Dictionary).get("overall", 50))
	# Return true if a should come before b (descending)
	return oa > ob


func _csv_position_to_category(pos_str: String) -> String:
	# Robust classifier for exotic position strings (e.g., "D/WB/M (R)", "F C", "AM (RLC)")
	var p := String(pos_str).to_upper()
	# Normalize separators
	p = p.replace("/", " ")
	p = p.replace("(", " ")
	p = p.replace(")", " ")
	p = p.replace(".", " ")
	p = p.replace(",", " ")
	while p.find("  ") != -1:
		p = p.replace("  ", " ")
	p = p.strip_edges()

	# GK
	if p.find("GK") != -1 or p.begins_with("GK"):
		return "GK"

	# Forwards
	if (
		p.find("ST") != -1
		or p.find("CF") != -1
		or p.find(" FW") != -1
		or p.ends_with(" F")
		or p.begins_with("F")
		or p.find(" FOR") != -1
	):
		return "FW"

	# Defenders (includes wing-backs)
	if (
		p.find("CB") != -1
		or p.find("LB") != -1
		or p.find("RB") != -1
		or p.find("LWB") != -1
		or p.find("RWB") != -1
		or p.find("WB") != -1
		or p.find(" D") != -1
		or p.find("DEF") != -1
		or p.begins_with("D")
	):
		return "DF"

	# Midfielders (default catch-all for wide/attacking/defensive mids and wingers)
	if (
		p.find("AM") != -1
		or p.find("DM") != -1
		or p.find(" CM") != -1
		or p.find(" RM") != -1
		or p.find(" LM") != -1
		or p.find(" RW") != -1
		or p.find(" LW") != -1
		or p.find("W ") != -1
		or p.find(" MID") != -1
		or p.begins_with("M")
	):
		return "MF"

	return "MF"


func _initialize_csv_cache(use_real_names: bool) -> void:
	"""Parse CSV once and populate static cache"""

	print("[OpponentRosterProvider] 📂 Initializing CSV cache from: %s" % OPPONENT_CSV_PATH)

	var file := FileAccess.open(OPPONENT_CSV_PATH, FileAccess.READ)
	if file == null:
		push_error("[OpponentRosterProvider] ❌ Failed to open CSV: %s" % OPPONENT_CSV_PATH)
		return

	print("[OpponentRosterProvider] ✅ CSV file opened successfully")

	# Parse CSV header
	var header_line := file.get_line()
	var headers := header_line.split(",")
	if headers.is_empty():
		push_error("[OpponentRosterProvider] ❌ CSV has no headers")
		file.close()
		return

	# Parse all rows
	var all_rows: Array = []
	while not file.eof_reached():
		var line := file.get_line().strip_edges()
		if line.is_empty():
			continue

		var values := line.split(",")
		var row: Dictionary = {}
		for i in range(min(headers.size(), values.size())):
			row[headers[i].strip_edges()] = values[i].strip_edges()

		all_rows.append(row)

	file.close()
	print("[OpponentRosterProvider] ✅ Parsed %d rows from CSV" % all_rows.size())

	# Group by team
	var team_map: Dictionary = {}
	for row in all_rows:
		var team_key := String(row.get("PseudoTeam", "")).strip_edges()
		if team_key.is_empty():
			continue

		if not team_map.has(team_key):
			team_map[team_key] = []

		team_map[team_key].append(row)

	print("[OpponentRosterProvider] ✅ Grouped into %d teams" % team_map.size())

	# Filter teams with sufficient players and populate cache
	_csv_parsed_teams.clear()
	var valid_teams := 0

	for team_name in team_map.keys():
		var rows: Array = team_map[team_name]

		# Require at least 11 players (minimum for a match)
		if rows.size() < 11:
			continue

		# Create player dictionaries
		var team_players: Array = []
		for i in range(min(22, rows.size())):  # Store up to 22 players per team
			var csv_row: Dictionary = rows[i]
			var player: Dictionary = _create_player_from_csv_row(csv_row, i, use_real_names)
			team_players.append(player)

		# Store with pseudo name as key
		var pseudo_team: String = String(team_name).strip_edges()
		_csv_parsed_teams[pseudo_team] = team_players
		valid_teams += 1

		# Also store by real team name if different
		if not rows.is_empty() and use_real_names:
			var real_team: String = String(rows[0].get("Team", "")).strip_edges()
			if not real_team.is_empty() and real_team != pseudo_team:
				_csv_parsed_teams[real_team] = team_players

	_csv_cache_initialized = true
	print("[OpponentRosterProvider] ✅ CSV cache initialized: %d valid teams (with 11+ players)" % valid_teams)


func _create_player_from_csv_row(csv_row: Dictionary, index: int, use_real_names: bool) -> Dictionary:
	"""Convert CSV row to player dictionary"""

	var player_name := String(csv_row.get("PseudoName", "Player %d" % (index + 1))).strip_edges()
	if use_real_names:
		var real_name := String(csv_row.get("Name", "")).strip_edges()
		if not real_name.is_empty():
			player_name = real_name

	return {
		"index": index,
		"name": player_name,
		"position": String(csv_row.get("Position", "SUB")).strip_edges(),
		"ca": int(csv_row.get("CA", 50)),
		"pa": int(csv_row.get("PA", 50)),
		"age": int(csv_row.get("Age", 20)),
		"nationality": String(csv_row.get("Nationality", "Unknown")).strip_edges(),
		# Technical attributes
		"corners": int(csv_row.get("Corners", 10)),
		"crossing": int(csv_row.get("Crossing", 10)),
		"dribbling": int(csv_row.get("Dribbling", 10)),
		"finishing": int(csv_row.get("Finishing", 10)),
		"first_touch": int(csv_row.get("First Touch", 10)),
		"free_kick_taking": int(csv_row.get("Free Kick Taking", 10)),
		"heading": int(csv_row.get("Heading", 10)),
		"long_shots": int(csv_row.get("Long Shots", 10)),
		"long_throws": int(csv_row.get("Long Throws", 10)),
		"marking": int(csv_row.get("Marking", 10)),
		"passing": int(csv_row.get("Passing", 10)),
		"penalty_taking": int(csv_row.get("Penalty Taking", 10)),
		"tackling": int(csv_row.get("Tackling", 10)),
		"technique": int(csv_row.get("Technique", 10)),
		# Mental attributes
		"aggression": int(csv_row.get("Aggression", 10)),
		"anticipation": int(csv_row.get("Anticipation", 10)),
		"bravery": int(csv_row.get("Bravery", 10)),
		"composure": int(csv_row.get("Composure", 10)),
		"concentration": int(csv_row.get("Concentration", 10)),
		"decisions": int(csv_row.get("Decisions", 10)),
		"determination": int(csv_row.get("Determination", 10)),
		"flair": int(csv_row.get("Flair", 10)),
		"leadership": int(csv_row.get("Leadership", 10)),
		"off_the_ball": int(csv_row.get("Off the Ball", 10)),
		"positioning": int(csv_row.get("Positioning", 10)),
		"teamwork": int(csv_row.get("Teamwork", 10)),
		"vision": int(csv_row.get("Vision", 10)),
		"work_rate": int(csv_row.get("Work Rate", 10)),
		# Physical attributes
		"acceleration": int(csv_row.get("Acceleration", 10)),
		"agility": int(csv_row.get("Agility", 10)),
		"balance": int(csv_row.get("Balance", 10)),
		"jumping_reach": int(csv_row.get("Jumping Reach", 10)),
		"natural_fitness": int(csv_row.get("Natural Fitness", 10)),
		"pace": int(csv_row.get("Pace", 10)),
		"stamina": int(csv_row.get("Stamina", 10)),
		"strength": int(csv_row.get("Strength", 10)),
		# Goalkeeper attributes (if applicable)
		"aerial_reach": int(csv_row.get("Aerial Reach", 10)),
		"command_of_area": int(csv_row.get("Command of Area", 10)),
		"communication": int(csv_row.get("Communication", 10)),
		"eccentricity": int(csv_row.get("Eccentricity", 10)),
		"handling": int(csv_row.get("Handling", 10)),
		"kicking": int(csv_row.get("Kicking", 10)),
		"one_on_ones": int(csv_row.get("One on Ones", 10)),
		"reflexes": int(csv_row.get("Reflexes", 10)),
		"rushing_out": int(csv_row.get("Rushing Out (Tendency)", 10)),
		"punching": int(csv_row.get("Punching (Tendency)", 10)),
		"throwing": int(csv_row.get("Throwing", 10)),
		# Additional info
		"team": String(csv_row.get("Team" if use_real_names else "PseudoTeam", "Unknown")).strip_edges(),
		"preferred_foot": String(csv_row.get("Preferred Foot", "Right")).strip_edges(),
	}


func _select_team_from_cache() -> String:
	"""Select a team from the cache"""

	if _csv_parsed_teams.is_empty():
		return ""

	# For now: random selection
	# Phase D TODO: Implement CA-based smart selection
	var team_names: Array = _csv_parsed_teams.keys()
	var random_index := randi() % team_names.size()
	return team_names[random_index]
