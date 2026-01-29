class_name BinaryProtocolEncoder
extends RefCounted
## ============================================================================
## BinaryProtocolEncoder - MRQ0 Binary Protocol Encoding
## ============================================================================
##
## PURPOSE: Encode match data to MRQ0 binary protocol for Rust engine
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Build match request v2 from match setup payload
## - Encode teams, players, and instructions to binary format
## - Normalize formations and team instructions
## - Build rosters from match data/timeline
##
## USAGE:
##   var encoder := BinaryProtocolEncoder.new()
##   var req := encoder.build_match_request_v2_from_match_setup_payload(setup)
##   encoder.encode_team_for_binary(buf, team)
## ============================================================================

## TimelineData type reference (optional, for rosters_from_timeline)
var _TimelineBinaryLoader: GDScript = null


func initialize(timeline_loader: GDScript = null) -> void:
	"""Initialize with optional TimelineBinaryLoader reference"""
	_TimelineBinaryLoader = timeline_loader


# =============================================================================
# Match Request v2 Builder
# =============================================================================

func build_match_request_v2_from_match_setup_payload(match_setup: Dictionary) -> Dictionary:
	"""Build MRQ0 v2 match request from match setup payload"""
	# Pass-through if caller already provided a schema v2 payload
	if int(match_setup.get("schema_version", 0)) == 2:
		return match_setup

	if not (match_setup.has("home_team") and match_setup.has("away_team")):
		return {}

	var home_team: Dictionary = match_setup.get("home_team", {}) if match_setup.get("home_team") is Dictionary else {}
	var away_team: Dictionary = match_setup.get("away_team", {}) if match_setup.get("away_team") is Dictionary else {}

	var seed_value: int = int(match_setup.get("seed", Time.get_ticks_usec()))
	var enable_position_tracking: bool = bool(match_setup.get("enable_position_tracking", true))
	var use_real_names: bool = bool(match_setup.get("use_real_names", false))

	var home_name := str(home_team.get("name", "home"))
	var away_name := str(away_team.get("name", "away"))

	var home_formation := normalize_formation_for_v2(
		str(home_team.get("formation", home_team.get("formation_id", "4-4-2")))
	)
	var away_formation := normalize_formation_for_v2(
		str(away_team.get("formation", away_team.get("formation_id", "4-4-2")))
	)

	var home_roster: Array = build_roster_uids_from_match_setup_team(home_team)
	var away_roster: Array = build_roster_uids_from_match_setup_team(away_team)

	if home_roster.size() != 18 or away_roster.size() != 18:
		return {}

	var req := {
		"schema_version": 2,
		"seed": seed_value,
		"home_team":
		{
			"name": home_name,
			"formation": home_formation,
			"roster": home_roster,
		},
		"away_team":
		{
			"name": away_name,
			"formation": away_formation,
			"roster": away_roster,
		},
		"enable_position_tracking": enable_position_tracking,
		"use_real_names": use_real_names,
	}

	# Pass through team instructions: check match_setup level first, then team level
	var home_instr: Variant = match_setup.get("home_instructions", home_team.get("instructions", null))
	var away_instr: Variant = match_setup.get("away_instructions", away_team.get("instructions", null))

	if home_instr != null and home_instr is Dictionary:
		req["home_instructions"] = normalize_team_instructions(home_instr as Dictionary)
	if away_instr != null and away_instr is Dictionary:
		req["away_instructions"] = normalize_team_instructions(away_instr as Dictionary)

	# Pass through AI difficulty settings
	var home_ai_diff: Variant = match_setup.get("home_ai_difficulty", home_team.get("ai_difficulty", null))
	var away_ai_diff: Variant = match_setup.get("away_ai_difficulty", away_team.get("ai_difficulty", null))

	if home_ai_diff != null:
		req["home_ai_difficulty"] = str(home_ai_diff)
	if away_ai_diff != null:
		req["away_ai_difficulty"] = str(away_ai_diff)

	return req


# =============================================================================
# Normalization Functions
# =============================================================================

func normalize_team_instructions(instr: Dictionary) -> Dictionary:
	"""Normalize team instructions to Rust-compatible format"""
	return {
		"defensive_line": str(instr.get("defensive_line", "Normal")),
		"team_width": str(instr.get("team_width", "Normal")),
		"team_tempo": str(instr.get("team_tempo", "Normal")),
		"pressing_intensity": str(instr.get("pressing_intensity", "Medium")),
		"build_up_style": str(instr.get("build_up_style", "Mixed")),
		"use_offside_trap": bool(instr.get("use_offside_trap", false)),
	}


func normalize_formation_for_v2(formation: String) -> String:
	"""Normalize formation string to v2 format (e.g., 'T442' -> '4-4-2')"""
	var f := formation.strip_edges()
	if f == "":
		return "4-4-2"
	if f.find("-") != -1:
		return f
	if not f.begins_with("T"):
		return f

	var num_str := f.substr(1)  # Remove "T"
	match num_str.length():
		3:
			return "%s-%s-%s" % [num_str[0], num_str[1], num_str[2]]
		4:
			return "%s-%s-%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3]]
		5:
			return "%s-%s-%s%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3], num_str[4]]
		_:
			return f


# =============================================================================
# Roster Building
# =============================================================================

func build_roster_uids_from_match_setup_team(team: Dictionary) -> Array:
	"""Build roster UIDs from match setup team dictionary"""
	# If already a v2 team payload, use it directly.
	var default_condition := 3
	var roster_variant: Variant = team.get("roster", null)
	if roster_variant is Array:
		var out: Array = []
		for entry in roster_variant:
			if entry is Dictionary:
				var d: Dictionary = (entry as Dictionary).duplicate(true)
				if d.has("uid"):
					if not d.has("condition"):
						d["condition"] = default_condition
					out.append(d)
				elif d.has("name"):
					# Embedded entry: ensure condition exists.
					if not d.has("condition"):
						d["condition"] = default_condition
					out.append(d)
				else:
					var uid := str(d.get("uid", d.get("id", "")))
					if uid != "":
						out.append({"uid": uid, "condition": default_condition})
			else:
				out.append({"uid": str(entry), "condition": default_condition})
		return out

	# NEW: If team has "players" array with full data, use embedded format (MRQ0 v3)
	var players_variant: Variant = team.get("players", null)
	if players_variant is Array:
		return build_roster_embedded_from_players(players_variant as Array)

	# MatchSetupExporter shape: starting_xi(11) + bench(7)
	var starters_variant: Variant = team.get("starting_xi", team.get("starters", null))
	var bench_variant: Variant = team.get("bench", null)
	if not (starters_variant is Array and bench_variant is Array):
		return []

	var starters: Array = starters_variant
	var bench: Array = bench_variant
	var out: Array = []
	for uid in starters:
		out.append({"uid": str(uid), "condition": default_condition})
	for uid in bench:
		out.append({"uid": str(uid), "condition": default_condition})
	return out


func build_roster_embedded_from_players(players: Array) -> Array:
	"""Build embedded player roster from players array (MRQ0 v3)
	Returns Array of Dictionaries with {name, position, overall, attributes?, track_id?}
	"""
	var out: Array = []
	for i in range(players.size()):
		var player: Variant = players[i]
		if player is String:
			# UID string - FIX01 requires {uid, condition}
			out.append({"uid": str(player), "condition": 3})
			continue
		if not player is Dictionary:
			continue

		var p: Dictionary = player as Dictionary
		var cond: int = clampi(int(p.get("condition", 3)), 1, 5)
		var entry: Dictionary = {
			"name": str(p.get("name", "Player_%d" % i)),
			"position": str(p.get("position_code", p.get("position", "MF"))),
			"overall": int(p.get("overall", 50)),
			"condition": cond,
		}

		# Include track_id if present
		if p.has("track_id"):
			entry["track_id"] = int(p.get("track_id"))

		# Include attributes if present (36 fields)
		var attrs_variant: Variant = p.get("attributes", null)
		if attrs_variant is Dictionary:
			var attrs: Dictionary = attrs_variant as Dictionary
			entry["attributes"] = convert_attributes_for_rust(attrs)

		# Include personality if present (Leader/Genius/Workhorse/Rebel/Steady)
		if p.has("personality") and p.personality != null:
			entry["personality"] = str(p.personality)

		# Include traits if present (max 4 slots)
		var traits_variant: Variant = p.get("traits", null)
		if traits_variant is Array:
			entry["traits"] = convert_traits_for_rust(traits_variant as Array)

		out.append(entry)
	return out


func build_rosters_from_match_data(home_team: Dictionary, away_team: Dictionary) -> Dictionary:
	"""Build rosters dictionary from match_data for binary timeline compatibility"""
	var rosters: Dictionary = {"home": [], "away": []}

	# Process home team players
	# NOTE: Rust engine uses track indices 0-10 for home, 11-21 for away
	# Roster order should match the order players were passed to the engine
	var home_players: Array = get_team_players_for_binary(home_team)
	for i in range(home_players.size()):
		var p: Dictionary = home_players[i]
		rosters["home"].append(
			{
				"id": i,  # Track index for home: 0-10
				"name": p.get("name", "Player %d" % i),
				"position": p.get("position", "CM"),
				"overall": p.get("overall", 60),
				"kit_number": p.get("jersey_number", p.get("kit_number", i + 1))
			}
		)

	# Process away team players
	var away_players: Array = get_team_players_for_binary(away_team)
	var home_count: int = home_players.size()
	for i in range(away_players.size()):
		var p: Dictionary = away_players[i]
		rosters["away"].append(
			{
				"id": home_count + i,  # Track index for away: 11-21
				"name": p.get("name", "Player %d" % i),
				"position": p.get("position", "CM"),
				"overall": p.get("overall", 60),
				"kit_number": p.get("jersey_number", p.get("kit_number", i + 1))
			}
		)

	return rosters


func build_rosters_from_timeline(
	timeline_data: Variant, home_team: Dictionary, away_team: Dictionary
) -> Dictionary:
	"""P17: Build rosters from timeline_data.match_setup (SSOT) or fallback to input teams"""
	# Check if match_setup exists in timeline data
	if timeline_data == null or not timeline_data.has("match_setup"):
		print("[BinaryProtocolEncoder] No match_setup in timeline, using input teams")
		return build_rosters_from_match_data(home_team, away_team)

	var match_setup: Dictionary = timeline_data.match_setup
	if match_setup.is_empty():
		print("[BinaryProtocolEncoder] Empty match_setup in timeline, using input teams")
		return build_rosters_from_match_data(home_team, away_team)

	if not match_setup.has("player_slots") or not (match_setup.player_slots is Array):
		print("[BinaryProtocolEncoder] Invalid match_setup structure, using input teams")
		return build_rosters_from_match_data(home_team, away_team)

	print("[BinaryProtocolEncoder] Using match_setup from timeline (SSOT)")
	var rosters: Dictionary = {"home": [], "away": []}

	var player_slots: Array = match_setup.player_slots
	for slot in player_slots:
		if not (slot is Dictionary):
			continue

		var track_id: int = int(slot.get("track_id", -1))
		var team: String = str(slot.get("team", ""))
		var name: String = str(slot.get("name", "Unknown"))
		var position: String = str(slot.get("position", "CM"))
		var overall: int = int(slot.get("overall", 60))
		var slot_num: int = int(slot.get("slot", 0))

		var roster_entry: Dictionary = {
			"id": track_id, "name": name, "position": position, "overall": overall, "kit_number": slot_num + 1  # slot is 0-based, kit_number is 1-based
		}

		if team == "home":
			rosters["home"].append(roster_entry)
		elif team == "away":
			rosters["away"].append(roster_entry)

	print(
		(
			"[BinaryProtocolEncoder] Built rosters from match_setup: home=%d, away=%d"
			% [rosters["home"].size(), rosters["away"].size()]
		)
	)
	return rosters


# =============================================================================
# Attribute/Trait Conversion
# =============================================================================

func convert_traits_for_rust(traits: Array) -> Array:
	"""Convert traits array to Rust-compatible format"""
	var out: Array = []
	for t in traits:
		if not t is Dictionary:
			continue
		var trait_dict: Dictionary = t as Dictionary
		out.append(
			{
				"id": str(trait_dict.get("id", "")),
				"tier": str(trait_dict.get("tier", "Bronze")),
			}
		)
	return out


func convert_attributes_for_rust(attrs: Dictionary) -> Dictionary:
	"""Convert player attributes dictionary to Rust-compatible format"""
	return {
		# Technical (14)
		"corners": int(attrs.get("corners", 50)),
		"crossing": int(attrs.get("crossing", 50)),
		"dribbling": int(attrs.get("dribbling", 50)),
		"finishing": int(attrs.get("finishing", 50)),
		"first_touch": int(attrs.get("first_touch", 50)),
		"free_kick_taking": int(attrs.get("free_kick_taking", attrs.get("free_kicks", 50))),
		"heading": int(attrs.get("heading", 50)),
		"long_shots": int(attrs.get("long_shots", 50)),
		"long_throws": int(attrs.get("long_throws", 50)),
		"marking": int(attrs.get("marking", 50)),
		"passing": int(attrs.get("passing", 50)),
		"penalty_taking": int(attrs.get("penalty_taking", 50)),
		"tackling": int(attrs.get("tackling", 50)),
		"technique": int(attrs.get("technique", 50)),
		# Mental (14)
		"aggression": int(attrs.get("aggression", 50)),
		"anticipation": int(attrs.get("anticipation", 50)),
		"bravery": int(attrs.get("bravery", 50)),
		"composure": int(attrs.get("composure", 50)),
		"concentration": int(attrs.get("concentration", 50)),
		"decisions": int(attrs.get("decisions", 50)),
		"determination": int(attrs.get("determination", 50)),
		"flair": int(attrs.get("flair", 50)),
		"leadership": int(attrs.get("leadership", 50)),
		"off_the_ball": int(attrs.get("off_the_ball", 50)),
		"positioning": int(attrs.get("positioning", 50)),
		"teamwork": int(attrs.get("teamwork", 50)),
		"vision": int(attrs.get("vision", 50)),
		"work_rate": int(attrs.get("work_rate", 50)),
		# Physical (8)
		"acceleration": int(attrs.get("acceleration", 50)),
		"agility": int(attrs.get("agility", 50)),
		"balance": int(attrs.get("balance", 50)),
		"jumping_reach": int(attrs.get("jumping_reach", attrs.get("jumping", 50))),
		"natural_fitness": int(attrs.get("natural_fitness", 50)),
		"pace": int(attrs.get("pace", 50)),
		"stamina": int(attrs.get("stamina", 50)),
		"strength": int(attrs.get("strength", 50)),
	}


# =============================================================================
# Binary Encoding Functions
# =============================================================================

func put_string_binary(buf: StreamPeerBuffer, s: String) -> void:
	"""Write a length-prefixed UTF-8 string to buffer"""
	var bytes := s.to_utf8_buffer()
	buf.put_u16(bytes.size())
	if bytes.size() > 0:
		buf.put_data(bytes)


func encode_position_binary(pos: String) -> int:
	"""Encode position string to binary enum value"""
	match pos.to_upper():
		"GK":
			return 0
		"LB":
			return 1
		"CB":
			return 2
		"RB":
			return 3
		"LWB":
			return 4
		"RWB":
			return 5
		"CDM":
			return 6
		"CM":
			return 7
		"CAM":
			return 8
		"LM":
			return 9
		"RM":
			return 10
		"LW":
			return 11
		"RW":
			return 12
		"CF":
			return 13
		"ST":
			return 14
		"DF":
			return 15
		"MF":
			return 16
		"FW":
			return 17
		_:
			return 7  # CM default


func encode_team_for_binary(buf: StreamPeerBuffer, team: Dictionary) -> void:
	"""Encode team data to binary format"""
	put_string_binary(buf, String(team.get("name", "Unknown")))
	put_string_binary(buf, String(team.get("formation", "4-4-2")))

	var players: Array = get_team_players_for_binary(team)
	var count: int = min(players.size(), 22)
	buf.put_u8(count)

	for i in range(count):
		var p: Dictionary = players[i]
		put_string_binary(buf, String(p.get("name", "Player %d" % i)))
		buf.put_u8(encode_position_binary(String(p.get("position", "CM"))))
		buf.put_u8(int(p.get("overall", 60)))


func get_team_players_for_binary(team: Dictionary) -> Array:
	"""Extract players array from team dictionary (supports multiple shapes)"""
	# Support both legacy shapes:
	# - `{ players: [...] }`
	# - MatchRequest v2 export: `{ starters: [...] }`
	var players_variant: Variant = team.get("players", null)
	if players_variant is Array:
		return players_variant
	var starters_variant: Variant = team.get("starters", null)
	if starters_variant is Array:
		return starters_variant
	return []


# =============================================================================
# Match Modifiers Encoding
# =============================================================================

func normalize_mrq0_match_modifiers(mods_variant: Variant) -> Array:
	"""Normalize MRQ0 match modifiers into a stable, deduped list.
	Input format: Array[Dictionary{ mod_id:int, value:float }]
	"""
	var by_id: Dictionary = {}
	if mods_variant is Array:
		for m in mods_variant:
			if typeof(m) != TYPE_DICTIONARY:
				continue
			var mod_id: int = int(m.get("mod_id", 0))
			if mod_id <= 0 or mod_id > 255:
				continue
			var value: float = float(m.get("value", 1.0))
			by_id[mod_id] = value

	var keys: Array = by_id.keys()
	keys.sort()

	var out: Array = []
	for k in keys:
		out.append({"mod_id": int(k), "value": float(by_id[k])})
	return out


func encode_mrq0_match_modifiers_list(buf: StreamPeerBuffer, mods: Array) -> void:
	"""Encode a match modifier list as: u8 count + repeated (u8 mod_id, f32 value)"""
	var normalized: Array = normalize_mrq0_match_modifiers(mods)
	buf.put_u8(min(normalized.size(), 255))
	for m in normalized:
		if typeof(m) != TYPE_DICTIONARY:
			continue
		var mod_id: int = int(m.get("mod_id", 0))
		if mod_id <= 0 or mod_id > 255:
			continue
		var value: float = float(m.get("value", 1.0))
		buf.put_u8(mod_id)
		buf.put_float(value)


# =============================================================================
# Instructions Encoding
# =============================================================================

func encode_instructions_binary(buf: StreamPeerBuffer, instr: Dictionary) -> void:
	"""Encode team instructions to binary format"""
	buf.put_u8(encode_defensive_line(str(instr.get("defensive_line", "Normal"))))
	buf.put_u8(encode_width(str(instr.get("team_width", "Normal"))))
	buf.put_u8(encode_tempo(str(instr.get("team_tempo", "Normal"))))
	buf.put_u8(encode_pressing(str(instr.get("pressing_intensity", "Medium"))))
	buf.put_u8(encode_build_up(str(instr.get("build_up_style", "Mixed"))))
	buf.put_u8(1 if bool(instr.get("use_offside_trap", false)) else 0)


func encode_defensive_line(val: String) -> int:
	"""Encode defensive line enum to binary"""
	match val:
		"VeryHigh":
			return 0
		"High":
			return 1
		"Normal":
			return 2
		"Deep", "Low":
			return 3
		"VeryDeep", "VeryLow":
			return 4
		_:
			return 2


func encode_width(val: String) -> int:
	"""Encode team width enum to binary"""
	match val:
		"VeryWide":
			return 0
		"Wide":
			return 1
		"Normal":
			return 2
		"Narrow":
			return 3
		"VeryNarrow":
			return 4
		_:
			return 2


func encode_tempo(val: String) -> int:
	"""Encode team tempo enum to binary"""
	match val:
		"VeryFast":
			return 0
		"Fast":
			return 1
		"Normal":
			return 2
		"Slow":
			return 3
		"VerySlow":
			return 4
		_:
			return 2


func encode_pressing(val: String) -> int:
	"""Encode pressing intensity enum to binary"""
	match val:
		"VeryHigh":
			return 0
		"High":
			return 1
		"Medium":
			return 2
		"Low":
			return 3
		"VeryLow":
			return 4
		_:
			return 2


func encode_build_up(val: String) -> int:
	"""Encode build-up style enum to binary"""
	match val:
		"Short", "ShortPassing":
			return 0
		"Mixed":
			return 1
		"Direct", "DirectPassing":
			return 2
		_:
			return 1
