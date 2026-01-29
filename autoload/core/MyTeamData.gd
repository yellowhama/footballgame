extends Node
# Preload to avoid autoload order issues with class_name
const _PlayerAppearanceBridge = preload("res://scripts/character/player_appearance_bridge.gd")

signal player_saved(player_data: Dictionary)
signal player_removed(player_id: String)
signal team_updated

const SAVE_PATH: String = "user://myteam/"
const MAX_PLAYERS: int = 50  # 최대 보유 선수
const TEAM_SIZE: int = 18  # 팀당 선수 수 (11 주전 + 7 교체)

var saved_players: Array = []
var current_team: Dictionary = {"formation": "T442", "players": []}  # 18명 선수 배열

const TECHNICAL_KEYS := [
	"corners",
	"crossing",
	"dribbling",
	"finishing",
	"first_touch",
	"free_kicks",
	"heading",
	"long_shots",
	"long_throws",
	"marking",
	"passing",
	"penalty_taking",
	"tackling",
	"technique"
]
const MENTAL_KEYS := [
	"aggression",
	"anticipation",
	"bravery",
	"composure",
	"concentration",
	"decisions",
	"determination",
	"flair",
	"leadership",
	"off_the_ball",
	"positioning",
	"teamwork",
	"vision",
	"work_rate"
]
const PHYSICAL_KEYS := [
	"acceleration", "agility", "balance", "jumping", "natural_fitness", "pace", "stamina", "strength"
]
const GOALKEEPER_KEYS := [
	"handling", "reflexes", "aerial_reach", "command_of_area", "communication", "eccentricity", "kicking", "rushing_out"
]

# Team tactics (TacticalParameters from OpenFootball)
var team_tactics: Dictionary = {
	"preset": "Balanced",  # Current preset name
	"parameters":
	{
		"attacking_intensity": 0.5,
		"defensive_line_height": 0.5,
		"width": 0.7,
		"pressing_trigger": 0.5,
		"tempo": 0.6,
		"directness": 0.5
	}
}

# User custom tactics presets
var custom_presets: Dictionary = {}

# Phase 22: UID generation state (collision-resistant)
static var _uid_sequence_counter: int = 0
static var _last_uid_timestamp: int = 0

# === Youth Academy Mode - M1.1 ===
enum SquadLevel { YOUTH, BTEAM, ATEAM }

var academy_settings: Dictionary = {
	"team_name": "My Academy",
	"short_name": "MYA",  # 3글자 약어 (스코어보드용)
	"nickname": "Youngsters",
	"emblem_id": 0,  # 엠블럼 ID (0-42 프리셋, 100+ 템플릿)
	"foundation_year": 2025,  # 창단년도
	# 유니폼 설정 (홈/원정) - Socceralia 스프라이트 지원 (2025-12-08)
	"uniform":
	{
		"home": {"primary": "#FF0000", "secondary": "#FFFFFF", "pattern_type": 0},  # HEX 컬러  # 0=단색, 1=가로줄, 2=세로줄, 3=체크
		"away": {"primary": "#FFFFFF", "secondary": "#FF0000", "pattern_type": 0}
	},
	# 레거시 호환용 (deprecated - uniform으로 대체)
	"emblem_icon": 0,
	"emblem_background": 0,
	"primary_color": "#FF0000",
	"secondary_color": "#FFFFFF"
}

var squad_level: int = SquadLevel.YOUTH


## 팀 생성 완료 여부 확인
func is_team_created() -> bool:
	return academy_settings.get("team_name", "") != "My Academy" or academy_settings.get("foundation_year", 0) > 0


## 팀 유니폼 가져오기 (홈/원정)
## 반환: { primary: "#HEX", secondary: "#HEX", pattern_type: int }
func get_team_uniform(is_home: bool = true) -> Dictionary:
	var uniform = academy_settings.get("uniform", {})
	var default_home = {"primary": "#FF0000", "secondary": "#FFFFFF", "pattern_type": 0}
	var default_away = {"primary": "#FFFFFF", "secondary": "#FF0000", "pattern_type": 0}

	var result: Dictionary
	if is_home:
		result = uniform.get("home", default_home)
	else:
		result = uniform.get("away", default_away)

	# 레거시 호환: pattern_type이 없으면 0
	if not result.has("pattern_type"):
		result["pattern_type"] = 0

	# 레거시 호환: 색상이 HEX가 아니면 변환
	if result.get("primary", "").find("#") == -1:
		result["primary"] = _PlayerAppearanceBridge._color_id_to_hex(result.get("primary", "red"))
	if result.get("secondary", "").find("#") == -1:
		result["secondary"] = _PlayerAppearanceBridge._color_id_to_hex(result.get("secondary", "white"))

	return result


## 팀 설정 저장
func save_academy_settings(settings: Dictionary) -> void:
	for key in settings:
		academy_settings[key] = settings[key]
	# 창단년도 자동 설정
	if not academy_settings.has("foundation_year") or academy_settings["foundation_year"] == 0:
		academy_settings["foundation_year"] = Time.get_datetime_dict_from_system()["year"]
	save_to_file()
	team_updated.emit()
	print("[MyTeamData] Academy settings saved: %s" % academy_settings.get("team_name", "Unknown"))


## 팀 생성 유효성 검사 (스펙 11.7)
func validate_team_creation() -> Dictionary:
	var errors: Array = []

	var team_name = academy_settings.get("team_name", "")
	var short_name = academy_settings.get("short_name", "")
	var uniform = academy_settings.get("uniform", {})

	# 팀 이름 검사
	if team_name.length() < 2:
		errors.append("팀 이름은 2자 이상이어야 합니다")
	if team_name.length() > 20:
		errors.append("팀 이름은 20자 이하여야 합니다")

	# 약어 검사
	if short_name.length() != 3:
		errors.append("약어는 정확히 3자여야 합니다")

	# 유니폼 충돌 검사 (홈 ≠ 원정 주색상)
	var home_primary = uniform.get("home", {}).get("primary", "")
	var away_primary = uniform.get("away", {}).get("primary", "")
	if home_primary == away_primary and home_primary != "":
		errors.append("홈과 원정 유니폼 주색상은 달라야 합니다")

	return {"valid": errors.is_empty(), "errors": errors}


func _ready():
	if not DirAccess.dir_exists_absolute(SAVE_PATH):
		DirAccess.open("user://").make_dir_recursive("myteam")

	load_all_players()
	print("[MyTeamData] Initialized with %d saved players" % saved_players.size())

	# First-time setup: Generate starter squad if no players exist
	if saved_players.size() == 0:
		print("[MyTeamData] First-time setup detected - Generating starter squad...")
		generate_starter_squad()
		print("[MyTeamData] ✅ Starter squad generated: %d players" % saved_players.size())
	elif saved_players.size() < 22:
		# Upgrade existing save to 22 players (2 per position)
		print("[MyTeamData] Upgrading squad to 22 players (2 per position)...")
		clear_all_players()
		generate_starter_squad()
		print("[MyTeamData] ✅ Squad upgraded: %d players" % saved_players.size())


func save_player_from_career(player_data: Dictionary) -> bool:
	"""
	Phase 22: ENHANCED - 82 fields 완전 저장
	SSOT: FM2023_PLAYER_DATA_SPEC.md
	Saves complete graduated player data including ending, personality, special abilities
	"""
	if saved_players.size() >= MAX_PLAYERS:
		print("[MyTeamData] Cannot save - reached maximum players limit")
		return false

	# Phase 22: Use new SSOT-compliant UID
	var player_id = generate_graduated_player_uid()

	# 82 fields complete data structure
	var save_data = {
		# ===== Basic Info (7 fields) =====
		"id": player_id,
		"name": player_data.get("name", "Unknown Player"),
		"pseudo_name": player_data.get("name", "Unknown Player"),
		"nationality": player_data.get("nationality", "KOR"),
		"age": player_data.get("age", 21),
		"position": player_data.get("position", "ST"),
		"positions": player_data.get("positions", [player_data.get("position", "ST")]),
		# ===== Abilities (3 fields) =====
		"overall": player_data.get("overall", 60),
		"ca": player_data.get("ca", player_data.get("overall", 60)),
		"pa": player_data.get("pa", 199),
		# ===== FM2023 Attributes (42 fields) =====
		"technical": player_data.get("technical", {}).duplicate(true),
		"mental": player_data.get("mental", {}).duplicate(true),
		"physical": player_data.get("physical", {}).duplicate(true),
		"goalkeeper": player_data.get("goalkeeper", {}).duplicate(true),
		# ===== Phase 4: Personality (9 fields) =====
		"personality": player_data.get("personality", {}).duplicate(true),
		"personality_archetype": player_data.get("personality_archetype", "Steady"),
		# ===== Phase 3: Special Abilities =====
		"special_abilities": player_data.get("special_abilities", []).duplicate(true),
		# ===== Phase 2: Exclusive Trait =====
		"exclusive_trait": player_data.get("exclusive_trait", null),
		# ===== Appearance (3 fields) =====
		"appearance": player_data.get("appearance", {}).duplicate(true),
		# ===== Career Stats (7+ fields) =====
		"career_stats": player_data.get("career_stats", {}).duplicate(true),
		# ===== Phase 24: Ending Data (6 fields) - CRITICAL! =====
		"ending": player_data.get("ending", {}).duplicate(true),
		# ===== Phase 24: Graduation Metadata (6 fields) - CRITICAL! =====
		"graduation_info": player_data.get("graduation_info", {}).duplicate(true),
		# ===== Internal (2 fields) =====
		"created_date": Time.get_datetime_string_from_system(),
		"is_default": false
	}

	# Phase 22: Validate data before saving
	if not _validate_graduated_player_data(save_data):
		push_error("[MyTeamData] Validation failed! Cannot save player.")
		return false

	# 메모리에 추가
	saved_players.append(save_data)

	# 파일로 저장
	save_to_file()

	player_saved.emit(save_data)

	# Enhanced logging with ending info
	print("[MyTeamData] ✅ Saved: %s (ID: %s, CA: %d)" % [save_data.name, player_id, save_data.ca])
	if save_data.ending.has("korean_title"):
		print("   Ending: %s (%s)" % [save_data.ending.get("korean_title", "?"), save_data.ending.get("rarity", "?")])

	return true


func _validate_graduated_player_data(data: Dictionary) -> bool:
	"""
	Phase 22: Validate 82 fields structure
	Returns false if critical fields are missing
	"""
	var required_keys = ["id", "name", "age", "ca", "technical", "mental", "physical", "ending", "graduation_info"]

	for key in required_keys:
		if not data.has(key):
			push_error("[MyTeamData] Missing required field: %s" % key)
			return false

	# Validate UID format
	if not data.id.begins_with("grad_"):
		push_error("[MyTeamData] Invalid UID format: %s (must start with 'grad_')" % data.id)
		return false

	# Validate ending structure
	if data.ending.is_empty():
		push_warning("[MyTeamData] Warning: Empty ending data for %s" % data.name)

	# Validate graduation_info structure
	if data.graduation_info.is_empty():
		push_warning("[MyTeamData] Warning: Empty graduation_info for %s" % data.name)

	return true


func generate_player_id() -> String:
	"""고유 플레이어 ID 생성 (LEGACY - keep for compatibility)"""
	var timestamp = Time.get_ticks_msec()
	var random_suffix = randi() % 10000
	return "player_%d_%04d" % [timestamp, random_suffix]


func generate_graduated_player_uid() -> String:
	"""
	Phase 22: SSOT-compliant UID generation for graduated players
	Format: grad_{timestamp_ms}_{seq}
	Collision-resistant: uses Unix timestamp + sequence counter
	"""
	var timestamp_ms = int(Time.get_unix_time_from_system() * 1000)

	if timestamp_ms == _last_uid_timestamp:
		_uid_sequence_counter += 1
	else:
		_uid_sequence_counter = 0
		_last_uid_timestamp = timestamp_ms

	return "grad_%d_%03d" % [timestamp_ms, _uid_sequence_counter]


func remove_player(player_id: String) -> bool:
	"""선수 방출"""
	for i in range(saved_players.size()):
		if saved_players[i].get("id", "") == player_id:
			saved_players.remove_at(i)
			save_to_file()
			player_removed.emit(player_id)

			# 현재 팀에서도 제거
			remove_from_current_team(player_id)
			return true
	return false


func clear_all_players():
	"""모든 선수 데이터 삭제 (리셋용)"""
	saved_players.clear()
	current_team.players = []
	save_to_file()
	print("[MyTeamData] All players cleared")


func get_player_by_id(player_id: String) -> Dictionary:
	"""ID로 선수 정보 조회"""
	for player in saved_players:
		if player.get("id", "") == player_id:
			return player
	return {}


func get_players_by_position(position: String) -> Array:
	"""포지션별 선수 목록"""
	var result = []
	for player in saved_players:
		if player.get("position", "") == position:
			result.append(player)
	return result


func get_top_players(count: int = 11) -> Array:
	"""Overall 상위 선수 목록"""
	var sorted = saved_players.duplicate()
	sorted.sort_custom(func(a, b): return a.get("overall", 0) > b.get("overall", 0))
	return sorted.slice(0, min(count, sorted.size()))


func set_team_formation(formation: String):
	"""팀 포메이션 설정"""
	current_team.formation = formation
	team_updated.emit()
	save_to_file()


func add_to_current_team(player_id: String, position_index: int) -> bool:
	"""현재 팀에 선수 추가 (0-17 위치)"""
	if position_index < 0 or position_index >= TEAM_SIZE:
		return false

	# 이미 팀에 있는지 확인
	for i in range(current_team.players.size()):
		if current_team.players[i] == player_id:
			# 위치 변경
			current_team.players[i] = ""
			break

	# 팀 배열 크기 보장
	while current_team.players.size() <= position_index:
		current_team.players.append("")

	current_team.players[position_index] = player_id
	team_updated.emit()
	save_to_file()
	return true


func swap_players(index_a: int, index_b: int) -> bool:
	"""Swap two players between slots (drag & drop support)"""
	if index_a < 0 or index_a >= TEAM_SIZE or index_b < 0 or index_b >= TEAM_SIZE:
		return false

	# Ensure array size
	while current_team.players.size() < TEAM_SIZE:
		current_team.players.append("")

	# Swap the player IDs
	var temp = current_team.players[index_a]
	current_team.players[index_a] = current_team.players[index_b]
	current_team.players[index_b] = temp

	team_updated.emit()
	save_to_file()
	print("[MyTeamData] Swapped players: slot %d <-> slot %d" % [index_a, index_b])
	return true


func remove_from_current_team(player_id: String):
	"""현재 팀에서 선수 제거"""
	for i in range(current_team.players.size()):
		if current_team.players[i] == player_id:
			current_team.players[i] = ""
			break
	team_updated.emit()
	save_to_file()


func get_current_team_players() -> Array:
	"""현재 팀의 선수 데이터 배열 반환"""
	var team_data = []
	for player_id in current_team.players:
		if player_id != "":
			var player = get_player_by_id(player_id)
			if player.size() > 0:
				team_data.append(player)
	return team_data


func get_team_for_match() -> Dictionary:
	"""매치 시뮬레이션용 팀 데이터 생성 (with instructions)"""
	var players_array = []

	# 현재 팀의 선수들을 OpenFootball CorePlayer 형식으로 변환
	for player_id in current_team.players:
		if player_id != "":
			var player = get_player_by_id(player_id)
			if player.size() > 0:
				# Use _convert_player_to_coreplayer to include instructions
				var core_player = _convert_player_to_coreplayer(player)
				players_array.append(core_player)

	# 부족한 인원 채우기 (최소 18명 필요) - CorePlayer format
	while players_array.size() < 18:
		var reserve_position = _get_default_position(players_array.size())
		var reserve_player = {
			"id": "reserve_%d" % (players_array.size() + 1),
			"name": "Reserve %d" % (players_array.size() + 1),
			"position": reserve_position,
			"age_months": 204.0,  # 17 years
			"ca": 60,
			"pa": 80,
			"detailed_stats": _get_default_stats(reserve_position),
			"hexagon_stats": {"pace": 10, "power": 10, "technical": 10, "shooting": 10, "passing": 10, "defending": 10},
			"personality":
			{
				"adaptability": 10,
				"ambition": 10,
				"determination": 10,
				"discipline": 10,
				"loyalty": 10,
				"pressure": 10,
				"professionalism": 10,
				"temperament": 10
			},
			"growth_profile":
			{
				"specialization": [],
				"training_response":
				{"technical_multiplier": 1.0, "physical_multiplier": 1.0, "mental_multiplier": 1.0},
				"growth_rate": 1.0,
				"injury_prone": 0.1
			},
			"special_abilities": {"abilities": [], "combination_history": []},
			"created_at": "2025-01-01T00:00:00Z",
			"updated_at": "2025-01-01T00:00:00Z",
			"instructions": null
		}
		players_array.append(reserve_player)

	return {"name": "My Team FC", "formation": current_team.formation, "players": players_array}


func _import_players_from_csv(max_players: int = 22) -> int:
	"""Import pseudo-named players from CSV dataset"""
	var csv_path = "res://data/players_with_pseudonym.csv"
	if not FileAccess.file_exists(csv_path):
		return 0

	var file = FileAccess.open(csv_path, FileAccess.READ)
	if not file:
		return 0

	# Skip header
	if not file.eof_reached():
		file.get_csv_line()

	var grouped_players := {}

	while not file.eof_reached():
		var row = file.get_csv_line()
		if row.is_empty() or row.size() < 9:
			continue

		var pseudo_team = row[8].strip_edges()
		if pseudo_team == "":
			continue

		if not grouped_players.has(pseudo_team):
			grouped_players[pseudo_team] = []
		grouped_players[pseudo_team].append(row)

	if grouped_players.is_empty():
		return 0

	# Pick the team with the most entries
	var target_team := ""
	var max_count := 0
	for team_name in grouped_players.keys():
		var size = grouped_players[team_name].size()
		if size > max_count:
			max_count = size
			target_team = team_name

	if target_team == "":
		return 0

	var selected_rows: Array = grouped_players[target_team]
	if selected_rows.is_empty():
		return 0

	clear_all_players()
	academy_settings.team_name = target_team
	current_team.players = []

	var imported_count := 0
	for row in selected_rows:
		if imported_count >= max_players:
			break

		var player_data = _create_player_from_csv_row(row)
		if player_data.is_empty():
			continue

		if save_player_from_career(player_data):
			var new_id = saved_players[-1].get("id", "")
			if imported_count < TEAM_SIZE:
				add_to_current_team(new_id, imported_count)
			imported_count += 1

	if imported_count >= TEAM_SIZE:
		return imported_count

	# Not enough players imported, reset and fall back
	clear_all_players()
	current_team.formation = "T442"
	return 0


func _create_player_from_csv_row(row: Array) -> Dictionary:
	if row.size() < 9:
		return {}

	var raw_name: String = row[7].strip_edges()
	if raw_name == "":
		raw_name = row[0].strip_edges()
	var mapped_position = _map_position(row[3])

	var ca := 60
	var pa := 75
	var age := 18

	var raw_ca = row[4].strip_edges()
	var raw_pa = row[5].strip_edges()
	var raw_age = row[6].strip_edges()

	if raw_ca != "":
		ca = max(40, int(raw_ca.to_float()))
	if raw_pa != "":
		pa = max(ca, int(raw_pa.to_float()))
	if raw_age != "":
		age = max(15, int(raw_age.to_float()))

	var stats = _generate_stats_from_ca(mapped_position, ca)

	return {
		"name": raw_name,
		"age": age,
		"position": mapped_position,
		"overall": ca,
		"technical": stats.get("technical", {}),
		"mental": stats.get("mental", {}),
		"physical": stats.get("physical", {}),
		"goalkeeper": stats.get("goalkeeper", {}),
		"appearance": _generate_default_appearance(),
		"strengths": [],
		"weaknesses": [],
		"hexagon_stats": stats.get("hexagon", {}),
		"personality": stats.get("personality", _default_personality_stats(1.0)),
		"growth_profile": stats.get("growth", _default_growth_profile()),
		"special_abilities": stats.get("special", _default_special_abilities()),
		"career_stats":
		{"seasons_played": 0, "total_goals": 0, "total_assists": 0, "total_matches": 0, "ending_type": "starter"}
	}


func _map_position(raw_position: String) -> String:
	if raw_position == null:
		return "CM"
	var upper = raw_position.to_upper()
	var tokens = upper.split(",", false)
	if tokens.size() > 0:
		upper = tokens[0]
	upper = upper.replace("(", " ").replace(")", " ")
	var words = upper.split(" ", false)
	var preferred_positions = [
		"GK", "CB", "LB", "RB", "LCB", "RCB", "LWB", "RWB", "CDM", "CM", "CAM", "LM", "RM", "LW", "RW", "CF", "ST", "FW"
	]
	for word in words:
		var trimmed = word.strip_edges()
		if trimmed == "":
			continue
		if trimmed in ["DC", "CB"]:
			return "CB"
		if trimmed in ["DL", "LB"]:
			return "LB"
		if trimmed in ["DR", "RB"]:
			return "RB"
		if trimmed == "DM":
			return "CDM"
		if trimmed == "AM":
			return "CAM"
		if trimmed == "CF":
			return "ST"
		if trimmed in preferred_positions:
			return trimmed
	if upper.contains("GK"):
		return "GK"
	if upper.contains("CB"):
		return "CB"
	if upper.contains("LB"):
		return "LB"
	if upper.contains("RB"):
		return "RB"
	if upper.contains("CDM"):
		return "CDM"
	if upper.contains("CAM"):
		return "CAM"
	if upper.contains("CM"):
		return "CM"
	if upper.contains("LM"):
		return "LM"
	if upper.contains("RM"):
		return "RM"
	if upper.contains("LW"):
		return "LW"
	if upper.contains("RW"):
		return "RW"
	if upper.contains("ST") or upper.contains("FW"):
		return "ST"
	return "CM"


func _generate_stats_from_ca(position: String, ca: int) -> Dictionary:
	var factor = max(0.5, float(ca) / 60.0)
	var base_stats = _get_default_stats(position)

	var technical := {}
	for key in TECHNICAL_KEYS:
		var base_value = base_stats.get(key, 6)
		technical[key] = _scale_attribute(base_value, factor)

	var mental := {}
	for key in MENTAL_KEYS:
		var base_value = base_stats.get(key, 6)
		mental[key] = _scale_attribute(base_value, factor)

	var physical := {}
	for key in PHYSICAL_KEYS:
		var base_value = base_stats.get(key, 6)
		physical[key] = _scale_attribute(base_value, factor)

	var goalkeeper_stats := {}
	if position == "GK":
		for key in GOALKEEPER_KEYS:
			goalkeeper_stats[key] = _scale_attribute(12, factor)
	else:
		for key in GOALKEEPER_KEYS:
			goalkeeper_stats[key] = 5

	var hexagon = _compute_hexagon_stats_from_ca(position, ca, technical, mental, physical)
	var personality = _default_personality_stats(factor)
	var growth = _default_growth_profile()
	var special = _default_special_abilities()

	return {
		"technical": technical,
		"mental": mental,
		"physical": physical,
		"goalkeeper": goalkeeper_stats,
		"hexagon": hexagon,
		"personality": personality,
		"growth": growth,
		"special": special
	}


func _scale_attribute(value: float, factor: float) -> int:
	return int(clamp(round(value * factor), 1, 20))


func _compute_hexagon_stats_from_ca(
	position: String, ca: int, technical: Dictionary, mental: Dictionary, physical: Dictionary
) -> Dictionary:
	var pace = _scale_attribute(10, float(ca) / 60.0)
	var shooting = _scale_attribute(
		(technical.get("finishing", 10) + technical.get("long_shots", 10)) / 2.0, float(ca) / 65.0
	)
	var passing = _scale_attribute((technical.get("passing", 10) + mental.get("vision", 10)) / 2.0, float(ca) / 65.0)
	var defending = _scale_attribute(
		(technical.get("tackling", 10) + technical.get("marking", 10)) / 2.0, float(ca) / 70.0
	)
	var technical_score = _scale_attribute(
		(technical.get("dribbling", 10) + technical.get("technique", 10)) / 2.0, float(ca) / 65.0
	)
	var power = _scale_attribute((physical.get("strength", 10) + physical.get("stamina", 10)) / 2.0, float(ca) / 60.0)

	if position == "GK":
		shooting = 5
		passing = _scale_attribute(
			(technical.get("passing", 10) + physical.get("kicking", 10) if physical.has("kicking") else 10) / 2.0,
			float(ca) / 65.0
		)
		defending = _scale_attribute(
			(technical.get("tackling", 10) + physical.get("balance", 10)) / 2.0, float(ca) / 70.0
		)

	return {
		"pace": pace,
		"shooting": shooting,
		"passing": passing,
		"defending": defending,
		"technical": technical_score,
		"power": power
	}


func _default_personality_stats(factor: float) -> Dictionary:
	return {
		"adaptability": _scale_attribute(10, factor),
		"ambition": _scale_attribute(11, factor),
		"determination": _scale_attribute(12, factor),
		"discipline": _scale_attribute(10, factor),
		"loyalty": _scale_attribute(11, factor),
		"pressure": _scale_attribute(10, factor),
		"professionalism": _scale_attribute(11, factor),
		"temperament": _scale_attribute(10, factor)
	}


func _default_growth_profile() -> Dictionary:
	return {
		"specialization": [],
		"training_response": {"technical_multiplier": 1.0, "physical_multiplier": 1.0, "mental_multiplier": 1.0},
		"growth_rate": 1.0,
		"injury_prone": 0.1
	}


func _default_special_abilities() -> Dictionary:
	return {"abilities": [], "combination_history": []}


func _get_default_position(index: int) -> String:
	"""기본 포지션 할당"""
	var positions = [
		"GK", "LB", "CB", "CB", "RB", "LM", "CM", "CM", "RM", "ST", "ST", "GK", "DF", "DF", "MF", "MF", "FW", "FW"
	]
	return positions[min(index, positions.size() - 1)]


func _get_default_stats(position: String) -> Dictionary:
	"""Generate position-appropriate default stats for reserve players"""
	var ca = 60  # Base CA for reserves
	var base = ca / 10.0  # Base attribute value (6 for CA 60)

	# All positions get basic attributes
	var attrs = {
		# Technical (14 fields)
		"corners": base - 2,
		"crossing": base - 1,
		"dribbling": base,
		"finishing": base,
		"first_touch": base,
		"free_kicks": base - 2,
		"heading": base - 1,
		"long_shots": base - 1,
		"long_throws": base - 4,
		"marking": base - 3,
		"passing": base,
		"penalty_taking": base - 2,
		"tackling": base - 3,
		"technique": base,
		# Mental (14 fields)
		"aggression": base - 2,
		"anticipation": base,
		"bravery": base - 1,
		"composure": base,
		"concentration": base,
		"decisions": base,
		"determination": base,
		"flair": base - 1,
		"leadership": base - 3,
		"off_the_ball": base,
		"positioning": base,
		"teamwork": base - 1,
		"vision": base,
		"work_rate": base,
		# Physical (8 fields)
		"acceleration": base,
		"agility": base - 1,
		"balance": base - 1,
		"jumping": base - 1,
		"natural_fitness": base,
		"pace": base,
		"stamina": base,
		"strength": base
	}

	# Position-specific boosts
	if position == "ST":
		attrs.finishing += 3
		attrs.off_the_ball += 2
		attrs.composure += 2
		attrs.heading += 2
	elif position == "CM":
		attrs.passing += 2
		attrs.vision += 2
		attrs.work_rate += 2
		attrs.stamina += 1
	elif position == "CB":
		attrs.tackling += 3
		attrs.marking += 3
		attrs.heading += 2
		attrs.positioning += 2
	elif position == "GK":
		attrs.first_touch += 2
		attrs.technique += 1
		attrs.positioning += 2
		attrs.concentration += 2

	return attrs


func save_to_file():
	"""데이터를 파일로 저장"""
	var save_file = FileAccess.open(SAVE_PATH + "myteam_data.save", FileAccess.WRITE)
	if save_file:
		var save_data = {
			"saved_players": saved_players,
			"current_team": current_team,
			"team_tactics": team_tactics,
			"custom_presets": custom_presets,
			"academy_settings": academy_settings,
			"squad_level": squad_level,
			"version": "1.2"
		}
		save_file.store_string(JSON.stringify(save_data))
		save_file.close()
		print("[MyTeamData] Data saved to file")


func load_all_players():
	"""저장된 데이터 불러오기"""
	var save_file_path = SAVE_PATH + "myteam_data.save"
	if not FileAccess.file_exists(save_file_path):
		print("[MyTeamData] No save file found")
		return

	var save_file = FileAccess.open(save_file_path, FileAccess.READ)
	if save_file:
		var json_text = save_file.get_as_text()
		save_file.close()

		var json = JSON.new()
		var parse_result = json.parse(json_text)

		if parse_result == OK:
			var data = json.data
			var save_version = data.get("version", "1.0")

			# Check if migration is needed
			if save_version == "1.0" or save_version == "1.1":
				print("[MyTeamData] 📦 Detected v%s save file - Migration required" % save_version)
				_create_backup_file(save_file_path)
				data = _migrate_to_v1_2(data)

			saved_players = data.get("saved_players", [])

			# ✅ Phase 22: Migrate old player format (52 fields → 82 fields)
			for i in range(saved_players.size()):
				saved_players[i] = _migrate_old_player_format(saved_players[i])

			current_team = data.get("current_team", {"formation": "T442", "players": []})

			# Migrate old formation IDs to new format
			current_team.formation = _migrate_formation_id(current_team.formation)

			# Load team tactics if available
			team_tactics = data.get("team_tactics", team_tactics)

			# Load custom presets
			custom_presets = data.get("custom_presets", {})

			# Load Youth Academy Mode settings (M1.1)
			academy_settings = data.get("academy_settings", academy_settings)
			squad_level = data.get("squad_level", SquadLevel.YOUTH)

			# Ensure academy_settings has required fields (M1.2 emblem system)
			if not academy_settings.has("emblem_icon"):
				academy_settings["emblem_icon"] = 0
				print("[MyTeamData] Added missing emblem_icon field")
			if not academy_settings.has("emblem_background"):
				academy_settings["emblem_background"] = 0
				print("[MyTeamData] Added missing emblem_background field")

			# Remove old emblem_preset field if it exists (v1.2 cleanup)
			if academy_settings.has("emblem_preset"):
				academy_settings.erase("emblem_preset")
				print("[MyTeamData] Removed obsolete emblem_preset field")

			print("[MyTeamData] Loaded %d players from file" % saved_players.size())
			if custom_presets.size() > 0:
				print("[MyTeamData] Loaded %d custom tactics presets" % custom_presets.size())
			if current_team.formation != "T442":
				print("[MyTeamData] Using formation: %s" % current_team.formation)
			if team_tactics.preset != "Balanced":
				print("[MyTeamData] Using tactics preset: %s" % team_tactics.preset)


func _migrate_formation_id(old_id: String) -> String:
	"""Migrate old formation IDs (4-4-2) to new format (T442)"""
	var migration_map = {
		"4-4-2": "T442",
		"4-3-3": "T433",
		"4-2-3-1": "T4231",
		"3-5-2": "T352",
		"5-3-2": "T532",
		"3-4-3": "T343",
		"4-1-2-1-2": "T41212",
		"4-1-4-1": "T4141",
		"4-3-1-2": "T4312",
		"5-4-1": "T541",
		"4-4-1-1": "T4411",
		"3-4-1-2": "T3412",
		"4-5-1": "T451",
		"3-4-2-1": "T3421"
	}

	if old_id in migration_map:
		print("[MyTeamData] Migrating formation ID: %s -> %s" % [old_id, migration_map[old_id]])
		return migration_map[old_id]

	return old_id


# ===== M1.5: Save File Migration System =====


func _create_backup_file(save_file_path: String) -> bool:
	"""Create backup of save file before migration (.bak)"""
	var backup_path = save_file_path + ".bak"

	# If backup already exists, add timestamp
	if FileAccess.file_exists(backup_path):
		var timestamp = Time.get_datetime_string_from_system().replace(":", "-")
		backup_path = save_file_path + "_" + timestamp + ".bak"

	var source_file = FileAccess.open(save_file_path, FileAccess.READ)
	if not source_file:
		print("[MyTeamData] ❌ Failed to open source file for backup")
		return false

	var content = source_file.get_as_text()
	source_file.close()

	var backup_file = FileAccess.open(backup_path, FileAccess.WRITE)
	if not backup_file:
		print("[MyTeamData] ❌ Failed to create backup file")
		return false

	backup_file.store_string(content)
	backup_file.close()

	print("[MyTeamData] ✅ Backup created: %s" % backup_path)
	return true


func _migrate_to_v1_2(old_data: Dictionary) -> Dictionary:
	"""Migrate v1.0/v1.1 save data to v1.2 format
	Changes in v1.2:
	- Added academy_settings (team customization)
	- Added squad_level (3-tier system)
	- Terminology: academy terminology established
	- Emblem system: emblem_preset → emblem_icon + emblem_background
	"""
	var migrated_data = old_data.duplicate(true)
	var old_version = old_data.get("version", "1.0")

	print("[MyTeamData] 🔄 Migrating from v%s to v1.2..." % old_version)

	# 1. Add academy_settings if missing or convert old emblem_preset
	if not migrated_data.has("academy_settings"):
		migrated_data["academy_settings"] = {
			"team_name": "My Academy",
			"nickname": "Youngsters",
			"emblem_icon": 0,
			"emblem_background": 0,
			"primary_color": "#FF0000",
			"secondary_color": "#FFFFFF"
		}
		print("[MyTeamData]   ✅ Added academy_settings")
	else:
		# Convert old emblem_preset to new emblem_icon + emblem_background
		var academy = migrated_data["academy_settings"]
		if academy.has("emblem_preset"):
			var preset = academy.get("emblem_preset", 0)
			# Map old preset (0-11) to new system (20 icons, 6 backgrounds)
			# icon = (preset * 2) % 20 (distribute across 20 icons)
			# background = preset / 2 (distribute across 6 backgrounds)
			academy["emblem_icon"] = (preset * 2) % 20
			academy["emblem_background"] = mini(int(float(preset) / 2.0), 5)
			academy.erase("emblem_preset")
			print(
				(
					"[MyTeamData]   ✅ Converted emblem_preset %d → icon %d, bg %d"
					% [preset, academy["emblem_icon"], academy["emblem_background"]]
				)
			)

	# 2. Add squad_level if missing (default to YOUTH)
	if not migrated_data.has("squad_level"):
		migrated_data["squad_level"] = SquadLevel.YOUTH
		print("[MyTeamData]   ✅ Added squad_level (YOUTH)")

	# 3. Terminology migration in player data (academy terminology)
	var migrated_players = migrated_data.get("saved_players", [])
	var players_migrated = 0

	for player in migrated_players:
		var modified = false

		# Migrate career_stats field names if they exist
		if player.has("career_stats"):
			var career_stats = player.get("career_stats", {})

			# Check for old terminology fields
			if career_stats.has("school_seasons"):
				career_stats["academy_seasons"] = career_stats.get("school_seasons", 0)
				career_stats.erase("school_seasons")
				modified = true

			if career_stats.has("school_goals"):
				career_stats["academy_goals"] = career_stats.get("school_goals", 0)
				career_stats.erase("school_goals")
				modified = true

			if career_stats.has("graduation_type"):
				career_stats["contract_type"] = career_stats.get("graduation_type", "normal")
				career_stats.erase("graduation_type")
				modified = true

		if modified:
			players_migrated += 1

	if players_migrated > 0:
		print("[MyTeamData]   ✅ Migrated %d player(s) terminology" % players_migrated)

	# 4. Update version to 1.2
	migrated_data["version"] = "1.2"

	# 5. Save migrated data immediately
	_save_migrated_data(migrated_data)

	print("[MyTeamData] ✅ Migration complete: v%s → v1.2" % old_version)
	return migrated_data


func _save_migrated_data(migrated_data: Dictionary):
	"""Save migrated data to file"""
	var save_file_path = SAVE_PATH + "myteam_data.save"
	var save_file = FileAccess.open(save_file_path, FileAccess.WRITE)
	if save_file:
		save_file.store_string(JSON.stringify(migrated_data))
		save_file.close()
		print("[MyTeamData] 💾 Migrated data saved to file")


func _migrate_old_player_format(player: Dictionary) -> Dictionary:
	"""
	Phase 22: Migrate old format (52 fields) → new format (82 fields)
	Backward compatibility for legacy saves
	"""
	# Already new format (has Phase 24 ending data)
	if player.has("ending"):
		return player

	print("[MyTeamData] 🔄 Migrating old player: %s" % player.get("name", "Unknown"))

	var migrated = player.duplicate(true)

	# Add missing personality (Phase 4)
	if not migrated.has("personality"):
		migrated["personality"] = {
			"adaptability": 50,
			"ambition": 50,
			"determination": 50,
			"discipline": 50,
			"loyalty": 50,
			"pressure": 50,
			"professionalism": 50,
			"temperament": 50
		}
		migrated["personality_archetype"] = "Steady"

	# Add missing special abilities (Phase 3)
	if not migrated.has("special_abilities"):
		migrated["special_abilities"] = []

	# Add missing exclusive trait (Phase 2)
	if not migrated.has("exclusive_trait"):
		migrated["exclusive_trait"] = null

	# Add fallback ending (Phase 24)
	if not migrated.has("ending"):
		migrated["ending"] = {
			"type": "LEGACY",
			"korean_title": "졸업생",
			"english_title": "Graduate",
			"rarity": "B",
			"description": "Legacy save (migrated from old format)",
			"narrative": "Career details not available for this player.",
			"highlights": [],
			"stats_summary": {}
		}

	# Add graduation info
	if not migrated.has("graduation_info"):
		migrated["graduation_info"] = {
			"graduation_year": 3,
			"graduation_week": 52,
			"graduation_date": migrated.get("created_date", "Unknown"),
			"final_condition": 3,
			"final_fatigue": 0.0,
			"final_ca": migrated.get("overall", 60),
			"coach_evaluation": 50
		}

	# Add missing basic fields
	if not migrated.has("pseudo_name"):
		migrated["pseudo_name"] = migrated.get("name", "Unknown")

	if not migrated.has("nationality"):
		migrated["nationality"] = "KOR"

	if not migrated.has("positions"):
		migrated["positions"] = [migrated.get("position", "ST")]

	if not migrated.has("ca"):
		migrated["ca"] = migrated.get("overall", 60)

	if not migrated.has("pa"):
		migrated["pa"] = 199

	print("[MyTeamData] ✅ Migration complete: %d fields → 82 fields" % player.size())
	return migrated


func get_statistics() -> Dictionary:
	"""마이팀 통계"""
	var stats = {"total_players": saved_players.size(), "avg_overall": 0, "by_position": {}, "by_ending": {}}

	if saved_players.size() > 0:
		var total_overall = 0
		for player in saved_players:
			total_overall += player.get("overall", 60)

			# 포지션별 집계
			var pos = player.get("position", "")
			if not stats.by_position.has(pos):
				stats.by_position[pos] = 0
			stats.by_position[pos] += 1

			# 엔딩별 집계
			var ending = player.get("career_stats", {}).get("ending_type", "normal")
			if not stats.by_ending.has(ending):
				stats.by_ending[ending] = 0
			stats.by_ending[ending] += 1

		stats.avg_overall = float(total_overall) / saved_players.size()

	return stats


# ===== Player Instructions Integration =====


func get_player_at_slot(slot: int) -> Dictionary:
	"""Get player data at formation slot (0-17)"""
	if slot < 0 or slot >= current_team.players.size():
		return {}

	var player_id = current_team.players[slot]
	if player_id == "":
		return {}

	return get_player_by_id(player_id)


func update_player_at_slot(slot: int, player_data: Dictionary) -> bool:
	"""Update player data at formation slot"""
	if slot < 0 or slot >= current_team.players.size():
		return false

	var player_id = current_team.players[slot]
	if player_id == "":
		return false

	# Find and update player in saved_players
	for i in range(saved_players.size()):
		if saved_players[i].get("id", "") == player_id:
			saved_players[i] = player_data
			save_to_file()
			return true

	return false


func _convert_player_to_coreplayer(player_data: Dictionary) -> Dictionary:
	"""Convert MyTeamData player format to CorePlayer format for Rust API"""
	# Extract dictionary attributes and flatten them
	var technical = player_data.get("technical", {})
	var mental = player_data.get("mental", {})
	var physical = player_data.get("physical", {})

	# Build detailed_stats with all 36 fields expected by OpenFootball CorePlayer
	# All values must be integers (u8 in Rust)
	# Use .get() with default values to ensure robustness
	# NOTE: GDScript uses "penalty_kicks" but Rust expects "penalty_taking"
	var detailed_stats = {
		# Technical (14 fields) - exact match with OpenFootball
		"corners": int(technical.get("corners", 50)),
		"crossing": int(technical.get("crossing", 50)),
		"dribbling": int(technical.get("dribbling", 50)),
		"finishing": int(technical.get("finishing", 50)),
		"first_touch": int(technical.get("first_touch", 50)),
		"free_kicks": int(technical.get("free_kicks", 50)),
		"heading": int(technical.get("heading", 50)),
		"long_shots": int(technical.get("long_shots", 50)),
		"long_throws": int(technical.get("long_throws", 50)),
		"marking": int(technical.get("marking", 50)),
		"passing": int(technical.get("passing", 50)),
		"penalty_taking": int(technical.get("penalty_kicks", 50)),  # GDScript uses "penalty_kicks"
		"tackling": int(technical.get("tackling", 50)),
		"technique": int(technical.get("technique", 50)),
		# Mental (14 fields) - exact match with OpenFootball
		"aggression": int(mental.get("aggression", 50)),
		"anticipation": int(mental.get("anticipation", 50)),
		"bravery": int(mental.get("bravery", 50)),
		"composure": int(mental.get("composure", 50)),
		"concentration": int(mental.get("concentration", 50)),
		"decisions": int(mental.get("decisions", 50)),
		"determination": int(mental.get("determination", 50)),
		"flair": int(mental.get("flair", 50)),
		"leadership": int(mental.get("leadership", 50)),
		"off_the_ball": int(mental.get("off_the_ball", 50)),
		"positioning": int(mental.get("positioning", 50)),
		"teamwork": int(mental.get("teamwork", 50)),
		"vision": int(mental.get("vision", 50)),
		"work_rate": int(mental.get("work_rate", 50)),
		# Physical (8 fields) - exact match with OpenFootball
		"acceleration": int(physical.get("acceleration", 50)),
		"agility": int(physical.get("agility", 50)),
		"balance": int(physical.get("balance", 50)),
		"jumping": int(physical.get("jumping", 50)),
		"natural_fitness": int(physical.get("natural_fitness", 50)),
		"pace": int(physical.get("pace", physical.get("speed", 50))),  # Try "pace" first, fallback to "speed"
		"stamina": int(physical.get("stamina", 50)),
		"strength": int(physical.get("strength", 50))
	}

	# Build CorePlayer format
	var core_player = {
		"id": player_data.get("id", ""),
		"name": player_data.get("name", "Unknown"),
		"position": player_data.get("position", "ST"),
		"age_months": float(player_data.get("age", 18)),
		"ca": int(player_data.get("overall", 60)),
		"pa": int(player_data.get("overall", 60)) + 20,  # Rough estimate: PA = CA + 20
		"detailed_stats": detailed_stats,
		"hexagon_stats":
		{
			"pace": int(50),
			"power": int(50),
			"technical": int(50),
			"shooting": int(50),
			"passing": int(50),
			"defending": int(50)
		},
		"personality":
		{
			"adaptability": int(10),
			"ambition": int(10),
			"determination": int(10),
			"discipline": int(10),
			"loyalty": int(10),
			"pressure": int(10),
			"professionalism": int(10),
			"temperament": int(10)
		},
		"growth_profile":
		{
			"specialization": [],
			"training_response": {"technical_multiplier": 1.0, "physical_multiplier": 1.0, "mental_multiplier": 1.0},
			"growth_rate": 1.0,
			"injury_prone": 0.1
		},
		"special_abilities": {"abilities": [], "combination_history": []},
		"created_at": "2025-01-01T00:00:00Z",
		"updated_at": "2025-01-01T00:00:00Z",
		"instructions": player_data.get("instructions", null)  # Read saved instructions from player data
	}

	return core_player


func set_player_role(slot: int, role_name: String) -> bool:
	"""Set role for player at formation slot using Rust engine"""
	var player_data = get_player_at_slot(slot)
	if player_data.size() == 0:
		return false

	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamData] Error: FootballRustEngine not available")
		return false

	# Convert player data to CorePlayer format (Dictionary)
	var core_player = _convert_player_to_coreplayer(player_data)

	# Call Rust engine with Dictionary
	var result = rust_engine.set_player_role(core_player, role_name)

	if result.get("success", false):
		# Update player data with role information
		player_data["role"] = role_name
		return update_player_at_slot(slot, player_data)
	else:
		print("[MyTeamData] Error setting role: %s" % result.get("error", "Unknown"))
		return false


func set_player_instructions(slot: int, instructions: Dictionary) -> bool:
	"""Set instructions for player at formation slot using Rust engine"""
	var player_data = get_player_at_slot(slot)
	if player_data.size() == 0:
		return false

	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamData] Error: FootballRustEngine not available")
		return false

	# Convert player data to CorePlayer format (Dictionary)
	var core_player = _convert_player_to_coreplayer(player_data)

	# Call Rust engine with Dictionaries
	var result = rust_engine.set_player_instructions(core_player, instructions)

	if result.get("success", false):
		# Update player data with instructions
		player_data["instructions"] = instructions
		return update_player_at_slot(slot, player_data)
	else:
		print("[MyTeamData] Error setting instructions: %s" % result.get("error", "Unknown"))
		return false


func generate_starter_squad():
	"""Generate starter squad with 2 players per position (22 total)"""
	var imported = _import_players_from_csv(22)
	if imported > 0:
		print("[MyTeamData] ✅ Imported %d players from players_with_pseudonym.csv" % imported)
		return

	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamData] Error: FootballRustEngine not available for starter squad generation")
		return

	# Create PlayerGenerator instance
	var player_generator = rust_engine.create_player_generator()
	if not player_generator:
		print("[MyTeamData] Error: Failed to create PlayerGenerator")
		return

	# Define all major positions (2 players each)
	var all_positions = ["GK", "CB", "CB", "LB", "RB", "CM", "CM", "LM", "RM", "ST", "ST"]

	# Generate 2 players for each position in the list (22 total)
	var starter_positions = []
	for pos in all_positions:
		starter_positions.append(pos)
		starter_positions.append(pos)  # Add twice for 2 players per position

	print("[MyTeamData] Generating %d starter players (2 per position)..." % starter_positions.size())

	# Generate starter players
	var position_counters = {}  # Track how many of each position we've created
	for i in range(starter_positions.size()):
		var position = starter_positions[i]

		# Count positions for unique naming
		if not position_counters.has(position):
			position_counters[position] = 1
		else:
			position_counters[position] += 1

		var _position_number = position_counters[position]

		# Generate player with base overall 60-65
		var base_overall = 60 + (randi() % 6)
		var player_age = 18  # Starting age

		# Get attributes from Rust generator
		var attributes = player_generator.generate_starter_player(position, player_age, base_overall)

		# Create Korean person name (e.g., "김민수", "박철수", etc.)
		var player_name = _generate_korean_name()

		# Create player data
		var player_data = {
			"name": player_name,
			"age": player_age,
			"position": position,
			"overall": base_overall,
			"technical": attributes.get("technical", {}),
			"mental": attributes.get("mental", {}),
			"physical": attributes.get("physical", {}),
			"goalkeeper": attributes.get("goalkeeper", {}),
			"appearance": _generate_default_appearance(),
			"strengths": [],
			"weaknesses": [],
			"career_stats":
			{"seasons_played": 0, "total_goals": 0, "total_assists": 0, "total_matches": 0, "ending_type": "starter"}
		}

		# Save player
		if save_player_from_career(player_data):
			# Add to current team (first 11 are starters, 0-10)
			add_to_current_team(saved_players[-1].get("id", ""), i)
			print("[MyTeamData] Generated %s (%s, Overall: %d)" % [player_name, position, base_overall])
		else:
			print("[MyTeamData] Error: Failed to save starter player %s" % player_name)

	print("[MyTeamData] ✅ Starter squad generation complete: %d players" % saved_players.size())


func _generate_default_appearance() -> Dictionary:
	"""Generate basic default appearance"""
	return {"skin_tone": randi() % 6, "hair_style": randi() % 10, "hair_color": randi() % 8, "body_type": "average"}


func _generate_korean_name() -> String:
	"""Generate random Korean person name"""
	var surnames = [
		"김",
		"이",
		"박",
		"최",
		"정",
		"강",
		"조",
		"윤",
		"장",
		"임",
		"한",
		"오",
		"서",
		"신",
		"권",
		"황",
		"안",
		"송",
		"전",
		"홍",
		"문",
		"양",
		"손",
		"배",
		"조",
		"백",
		"허",
		"유",
		"남",
		"심"
	]

	var given_names = [
		"민수",
		"지훈",
		"준호",
		"서준",
		"도윤",
		"시우",
		"하준",
		"주원",
		"민준",
		"건우",
		"유준",
		"현우",
		"예준",
		"지호",
		"승현",
		"승우",
		"지환",
		"승민",
		"시현",
		"우진",
		"선우",
		"연우",
		"정우",
		"민재",
		"현준",
		"동현",
		"태양",
		"은우",
		"수호",
		"준서",
		"태민",
		"재윤",
		"인성",
		"지우",
		"준영",
		"재현",
		"태현",
		"성민",
		"민성",
		"진우",
		"철수",
		"영수",
		"명수",
		"성수",
		"동수",
		"진수",
		"상우",
		"경민"
	]

	var surname = surnames[randi() % surnames.size()]
	var given_name = given_names[randi() % given_names.size()]

	return surname + given_name


# ===== Team Tactics Management =====

const TACTICS_PRESETS = {
	"Balanced":
	{
		"name_ko": "균형잡힌",
		"description": "공수 균형이 잘 잡힌 표준 전술",
		"parameters":
		{
			"attacking_intensity": 0.5,
			"defensive_line_height": 0.5,
			"width": 0.7,
			"pressing_trigger": 0.5,
			"tempo": 0.6,
			"directness": 0.5
		}
	},
	"TikiTaka":
	{
		"name_ko": "티키타카",
		"description": "점유 중심, 짧은 패싱, 높은 압박",
		"parameters":
		{
			"attacking_intensity": 0.6,
			"defensive_line_height": 0.7,
			"width": 0.9,
			"pressing_trigger": 0.8,
			"tempo": 0.8,
			"directness": 0.3
		}
	},
	"Counter":
	{
		"name_ko": "역습",
		"description": "수비 후 빠른 역습, 직접적 플레이",
		"parameters":
		{
			"attacking_intensity": 0.7,
			"defensive_line_height": 0.3,
			"width": 0.6,
			"pressing_trigger": 0.3,
			"tempo": 0.9,
			"directness": 0.9
		}
	},
	"Direct":
	{
		"name_ko": "직접적",
		"description": "롱볼 위주, 직접적인 플레이 스타일",
		"parameters":
		{
			"attacking_intensity": 0.8,
			"defensive_line_height": 0.5,
			"width": 0.7,
			"pressing_trigger": 0.5,
			"tempo": 0.7,
			"directness": 0.9
		}
	},
	"HighPress":
	{
		"name_ko": "전방 압박",
		"description": "높은 압박, 공격적 수비",
		"parameters":
		{
			"attacking_intensity": 0.9,
			"defensive_line_height": 0.8,
			"width": 0.7,
			"pressing_trigger": 0.9,
			"tempo": 0.8,
			"directness": 0.6
		}
	},
	"ParkTheBus":
	{
		"name_ko": "극단적 수비",
		"description": "페널티 박스 밀집 수비, 역습 대기",
		"parameters":
		{
			"attacking_intensity": 0.2,
			"defensive_line_height": 0.2,
			"width": 0.4,
			"pressing_trigger": 0.1,
			"tempo": 0.4,
			"directness": 0.8
		}
	},
	"GegenPress":
	{
		"name_ko": "게겐프레싱",
		"description": "볼 손실 즉시 압박, 빠른 볼 회복",
		"parameters":
		{
			"attacking_intensity": 0.8,
			"defensive_line_height": 0.7,
			"width": 0.7,
			"pressing_trigger": 0.95,
			"tempo": 0.9,
			"directness": 0.5
		}
	},
	"WingPlay":
	{
		"name_ko": "측면 공격",
		"description": "양 측면 활용, 크로스 중심",
		"parameters":
		{
			"attacking_intensity": 0.7,
			"defensive_line_height": 0.5,
			"width": 0.95,
			"pressing_trigger": 0.5,
			"tempo": 0.6,
			"directness": 0.6
		}
	},
	"Possession":
	{
		"name_ko": "점유율",
		"description": "천천히 빌드업, 볼 점유 유지",
		"parameters":
		{
			"attacking_intensity": 0.4,
			"defensive_line_height": 0.6,
			"width": 0.8,
			"pressing_trigger": 0.7,
			"tempo": 0.4,
			"directness": 0.2
		}
	}
}


func set_team_tactics_preset(preset_name: String) -> bool:
	"""Apply a tactical preset (built-in or custom)"""
	var preset_data = null

	# Check built-in presets first
	if TACTICS_PRESETS.has(preset_name):
		preset_data = TACTICS_PRESETS[preset_name]
	# Check custom presets
	elif custom_presets.has(preset_name):
		preset_data = custom_presets[preset_name]
	else:
		print("[MyTeamData] Error: Unknown tactics preset '%s'" % preset_name)
		return false

	team_tactics.preset = preset_name
	team_tactics.parameters = preset_data.parameters.duplicate(true)

	save_to_file()
	team_updated.emit()

	print("[MyTeamData] Applied tactics preset: %s (%s)" % [preset_name, preset_data.name_ko])
	return true


func set_team_tactics_custom(parameters: Dictionary) -> bool:
	"""Set custom tactical parameters"""
	# Validate all parameters are present and in range [0.0, 1.0]
	var required_params = [
		"attacking_intensity", "defensive_line_height", "width", "pressing_trigger", "tempo", "directness"
	]

	for param in required_params:
		if not parameters.has(param):
			print("[MyTeamData] Error: Missing parameter '%s'" % param)
			return false

		var value = parameters[param]
		if typeof(value) != TYPE_FLOAT and typeof(value) != TYPE_INT:
			print("[MyTeamData] Error: Parameter '%s' must be a number" % param)
			return false

		if value < 0.0 or value > 1.0:
			print("[MyTeamData] Error: Parameter '%s' must be between 0.0 and 1.0" % param)
			return false

	# Apply custom tactics
	team_tactics.preset = "Custom"
	team_tactics.parameters = parameters.duplicate(true)

	save_to_file()
	team_updated.emit()

	print("[MyTeamData] Applied custom tactics")
	return true


func get_team_tactics() -> Dictionary:
	"""Get current team tactics"""
	return team_tactics.duplicate(true)


func get_available_presets() -> Array:
	"""Get list of available tactics presets with info (built-in + custom)"""
	var presets = []

	# Add built-in presets
	for preset_name in TACTICS_PRESETS.keys():
		var preset = TACTICS_PRESETS[preset_name]
		presets.append(
			{
				"id": preset_name,
				"name_ko": preset.name_ko,
				"description": preset.description,
				"is_current": team_tactics.preset == preset_name,
				"is_custom": false
			}
		)

	# Add custom presets
	for preset_name in custom_presets.keys():
		var preset = custom_presets[preset_name]
		presets.append(
			{
				"id": preset_name,
				"name_ko": preset.name_ko,
				"description": preset.description,
				"is_current": team_tactics.preset == preset_name,
				"is_custom": true
			}
		)

	return presets


func save_custom_preset(
	preset_id: String, parameters: Dictionary, description: String = "", display_name: String = ""
) -> bool:
	"""Save a new custom tactics preset (with optional separate display name)"""
	# Validate preset ID
	if preset_id.strip_edges() == "":
		print("[MyTeamData] Error: Preset ID cannot be empty")
		return false

	# Use preset_id as display name if not provided (backward compatibility)
	var name_ko = display_name if display_name != "" else preset_id

	# Check if ID conflicts with built-in presets (only if using ID as name)
	if display_name == "" and TACTICS_PRESETS.has(preset_id):
		print("[MyTeamData] Error: Cannot use built-in preset name '%s'" % preset_id)
		return false

	# Validate parameters
	var required_params = [
		"attacking_intensity", "defensive_line_height", "width", "pressing_trigger", "tempo", "directness"
	]
	for param in required_params:
		if not parameters.has(param):
			print("[MyTeamData] Error: Missing parameter '%s'" % param)
			return false

		var value = parameters[param]
		if typeof(value) != TYPE_FLOAT and typeof(value) != TYPE_INT:
			print("[MyTeamData] Error: Parameter '%s' must be a number" % param)
			return false

		if value < 0.0 or value > 1.0:
			print("[MyTeamData] Error: Parameter '%s' must be between 0.0 and 1.0" % param)
			return false

	# Save custom preset (use preset_id as key, name_ko as display name)
	custom_presets[preset_id] = {
		"name_ko": name_ko,
		"description": description if description != "" else "사용자 커스텀 전술",
		"parameters": parameters.duplicate(true)
	}

	save_to_file()
	team_updated.emit()

	print("[MyTeamData] Saved custom tactics preset: %s (ID: %s)" % [name_ko, preset_id])
	return true


func rename_custom_preset(old_name: String, new_name: String) -> bool:
	"""Rename an existing custom preset"""
	if not custom_presets.has(old_name):
		print("[MyTeamData] Error: Custom preset '%s' not found" % old_name)
		return false

	if new_name.strip_edges() == "":
		print("[MyTeamData] Error: New name cannot be empty")
		return false

	if TACTICS_PRESETS.has(new_name):
		print("[MyTeamData] Error: Cannot use built-in preset name '%s'" % new_name)
		return false

	if custom_presets.has(new_name) and old_name != new_name:
		print("[MyTeamData] Error: Custom preset '%s' already exists" % new_name)
		return false

	# Copy preset with new name
	var preset_data = custom_presets[old_name].duplicate(true)
	preset_data.name_ko = new_name
	custom_presets[new_name] = preset_data

	# Delete old preset
	custom_presets.erase(old_name)

	# Update current tactics if needed
	if team_tactics.preset == old_name:
		team_tactics.preset = new_name

	save_to_file()
	team_updated.emit()

	print("[MyTeamData] Renamed custom preset: %s -> %s" % [old_name, new_name])
	return true


func delete_custom_preset(preset_name: String) -> bool:
	"""Delete a custom tactics preset"""
	if not custom_presets.has(preset_name):
		print("[MyTeamData] Error: Custom preset '%s' not found" % preset_name)
		return false

	# Remove preset
	custom_presets.erase(preset_name)

	# Reset to Balanced if current preset was deleted
	if team_tactics.preset == preset_name:
		set_team_tactics_preset("Balanced")

	save_to_file()
	team_updated.emit()

	print("[MyTeamData] Deleted custom preset: %s" % preset_name)
	return true


func get_formation_with_instructions() -> Dictionary:
	"""Get current formation with all player instructions"""
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamData] Error: FootballRustEngine not available")
		return {}

	var formation_id = current_team.formation
	var formation_result = rust_engine.get_formation_details(formation_id)

	if not formation_result.get("success", false):
		print("[MyTeamData] Error getting formation details: %s" % formation_result.get("error", "Unknown"))
		return {}

	var formation = formation_result.get("formation", {})
	var positions = formation.get("positions", [])

	var result = {
		"formation_id": formation_id,
		"formation_name_ko": formation.get("name_ko", formation_id),
		"formation_name_en": formation.get("name_en", formation_id),
		"tactical_style": formation.get("tactical_style", "Unknown"),
		"positions": []
	}

	# Build position data with player info
	for i in range(11):
		var position = positions[i] if i < positions.size() else {}
		var player_data = get_player_at_slot(i)

		var position_info = {
			"slot": i,
			"position_type": position.get("position_type", ""),
			"position_name_ko": position.get("position_name_ko", ""),
			"position_name_en": position.get("position_name_en", ""),
			"x": position.get("x", 0.5),
			"y": position.get("y", 0.5),
			"player_id": current_team.players[i] if i < current_team.players.size() else "",
			"player_name": player_data.get("name", "") if player_data.size() > 0 else "",
			"player_overall": player_data.get("overall", 0) if player_data.size() > 0 else 0,
			"role": player_data.get("role", null) if player_data.size() > 0 else null,
			"instructions": player_data.get("instructions", {}) if player_data.size() > 0 else {}
		}

		result.positions.append(position_info)

	return result


# ===== Role-Based Instruction Presets =====

const ROLE_INSTRUCTION_PRESETS = {
	# Goalkeeper
	"Goalkeeper":
	{
		"name_ko": "골키퍼",
		"description": "골문 수비 중심",
		"instructions": {"mentality": "Conservative", "depth": "StayBack", "defensive_work": "Maximum"}
	},
	# Defenders
	"CenterBack":
	{
		"name_ko": "중앙 수비수 (표준)",
		"description": "중앙 수비, 안정적",
		"instructions":
		{
			"mentality": "Conservative",
			"depth": "StayBack",
			"passing_style": "Short",
			"defensive_work": "Maximum",
			"pressing_intensity": "Medium"
		}
	},
	"CenterBack_BallPlaying":
	{
		"name_ko": "중앙 수비수 (빌드업)",
		"description": "빌드업 참여, 패스 중심",
		"instructions":
		{
			"mentality": "Balanced",
			"depth": "Standard",
			"passing_style": "Mixed",
			"dribbling_frequency": "Occasionally",
			"defensive_work": "High",
			"pressing_intensity": "Medium"
		}
	},
	"LeftBack":
	{
		"name_ko": "좌측 풀백 (수비형)",
		"description": "측면 수비 중심",
		"instructions":
		{
			"mentality": "Conservative",
			"width": "StayWide",
			"depth": "StayBack",
			"passing_style": "Short",
			"defensive_work": "Maximum",
			"pressing_intensity": "Medium"
		}
	},
	"LeftBack_Attacking":
	{
		"name_ko": "좌측 풀백 (공격형)",
		"description": "측면 오버래핑, 공격 가담",
		"instructions":
		{
			"mentality": "Balanced",
			"width": "StayWide",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Occasionally",
			"defensive_work": "High",
			"pressing_intensity": "High"
		}
	},
	"RightBack":
	{
		"name_ko": "우측 풀백 (수비형)",
		"description": "측면 수비 중심",
		"instructions":
		{
			"mentality": "Conservative",
			"width": "StayWide",
			"depth": "StayBack",
			"passing_style": "Short",
			"defensive_work": "Maximum",
			"pressing_intensity": "Medium"
		}
	},
	"RightBack_Attacking":
	{
		"name_ko": "우측 풀백 (공격형)",
		"description": "측면 오버래핑, 공격 가담",
		"instructions":
		{
			"mentality": "Balanced",
			"width": "StayWide",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Occasionally",
			"defensive_work": "High",
			"pressing_intensity": "High"
		}
	},
	"WingBack":
	{
		"name_ko": "윙백",
		"description": "측면 전체 커버, 공수 겸비",
		"instructions":
		{
			"mentality": "Aggressive",
			"width": "StayWide",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Frequently",
			"defensive_work": "High",
			"pressing_intensity": "High"
		}
	},
	# Midfielders
	"DefensiveMidfield":
	{
		"name_ko": "수비형 미드필더",
		"description": "중원 수비, 볼 차단",
		"instructions":
		{
			"mentality": "Conservative",
			"depth": "StayBack",
			"passing_style": "Short",
			"dribbling_frequency": "Rarely",
			"defensive_work": "Maximum",
			"pressing_intensity": "High"
		}
	},
	"DefensiveMidfield_Playmaker":
	{
		"name_ko": "수비형 미드필더 (플레이메이커)",
		"description": "깊은 위치에서 빌드업",
		"instructions":
		{
			"mentality": "Balanced",
			"depth": "StayBack",
			"passing_style": "Mixed",
			"dribbling_frequency": "Occasionally",
			"defensive_work": "High",
			"pressing_intensity": "Medium"
		}
	},
	"CentralMidfield":
	{
		"name_ko": "중앙 미드필더 (표준)",
		"description": "공수 균형, 박스 투 박스",
		"instructions":
		{
			"mentality": "Balanced",
			"depth": "Standard",
			"passing_style": "Mixed",
			"dribbling_frequency": "Occasionally",
			"shooting_tendency": "Normal",
			"defensive_work": "High",
			"pressing_intensity": "Medium"
		}
	},
	"CentralMidfield_Attacking":
	{
		"name_ko": "중앙 미드필더 (공격형)",
		"description": "전방 전개, 슈팅 적극",
		"instructions":
		{
			"mentality": "Aggressive",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Frequently",
			"shooting_tendency": "HighFrequency",
			"defensive_work": "Normal",
			"pressing_intensity": "High"
		}
	},
	"AttackingMidfield":
	{
		"name_ko": "공격형 미드필더",
		"description": "전방 플레이메이커, 창조적",
		"instructions":
		{
			"mentality": "Aggressive",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Frequently",
			"shooting_tendency": "HighFrequency",
			"defensive_work": "Minimal",
			"pressing_intensity": "Medium"
		}
	},
	"LeftWing":
	{
		"name_ko": "좌측 윙어 (컷인형)",
		"description": "중앙으로 컷인, 슈팅",
		"instructions":
		{
			"mentality": "Aggressive",
			"width": "CutInside",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Frequently",
			"shooting_tendency": "HighFrequency",
			"defensive_work": "Minimal",
			"pressing_intensity": "Medium"
		}
	},
	"LeftWing_Wide":
	{
		"name_ko": "좌측 윙어 (측면형)",
		"description": "측면 돌파, 크로스",
		"instructions":
		{
			"mentality": "Aggressive",
			"width": "StayWide",
			"depth": "GetForward",
			"passing_style": "Direct",
			"dribbling_frequency": "Always",
			"shooting_tendency": "Normal",
			"defensive_work": "Minimal",
			"pressing_intensity": "Low"
		}
	},
	"RightWing":
	{
		"name_ko": "우측 윙어 (컷인형)",
		"description": "중앙으로 컷인, 슈팅",
		"instructions":
		{
			"mentality": "Aggressive",
			"width": "CutInside",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Frequently",
			"shooting_tendency": "HighFrequency",
			"defensive_work": "Minimal",
			"pressing_intensity": "Medium"
		}
	},
	"RightWing_Wide":
	{
		"name_ko": "우측 윙어 (측면형)",
		"description": "측면 돌파, 크로스",
		"instructions":
		{
			"mentality": "Aggressive",
			"width": "StayWide",
			"depth": "GetForward",
			"passing_style": "Direct",
			"dribbling_frequency": "Always",
			"shooting_tendency": "Normal",
			"defensive_work": "Minimal",
			"pressing_intensity": "Low"
		}
	},
	# Forwards
	"Striker":
	{
		"name_ko": "스트라이커 (표준)",
		"description": "골 결정력 중심",
		"instructions":
		{
			"mentality": "Aggressive",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Occasionally",
			"shooting_tendency": "HighFrequency",
			"defensive_work": "Minimal",
			"pressing_intensity": "Low"
		}
	},
	"Striker_Poacher":
	{
		"name_ko": "스트라이커 (포처)",
		"description": "페널티 박스 내 대기",
		"instructions":
		{
			"mentality": "Aggressive",
			"depth": "PushUp",
			"passing_style": "Short",
			"dribbling_frequency": "Rarely",
			"shooting_tendency": "ShootOnSight",
			"defensive_work": "Minimal",
			"pressing_intensity": "Low"
		}
	},
	"Striker_TargetMan":
	{
		"name_ko": "스트라이커 (타겟맨)",
		"description": "공중볼 장악, 포스트 플레이",
		"instructions":
		{
			"mentality": "Balanced",
			"depth": "Standard",
			"passing_style": "Short",
			"dribbling_frequency": "Rarely",
			"shooting_tendency": "Normal",
			"defensive_work": "Minimal",
			"pressing_intensity": "Medium"
		}
	},
	"CenterForward":
	{
		"name_ko": "중앙 공격수",
		"description": "전방 중앙, 골 결정력",
		"instructions":
		{
			"mentality": "Aggressive",
			"depth": "GetForward",
			"passing_style": "Mixed",
			"dribbling_frequency": "Frequently",
			"shooting_tendency": "HighFrequency",
			"defensive_work": "Minimal",
			"pressing_intensity": "Medium"
		}
	},
	"FalseNine":
	{
		"name_ko": "거짓 9번",
		"description": "중원 하강, 플레이메이킹",
		"instructions":
		{
			"mentality": "Balanced",
			"width": "Roam",
			"depth": "Standard",
			"passing_style": "Mixed",
			"dribbling_frequency": "Frequently",
			"shooting_tendency": "Normal",
			"defensive_work": "Normal",
			"pressing_intensity": "High"
		}
	}
}


func get_role_instruction_presets_by_position(position: String) -> Array:
	"""Get available instruction presets for a position"""
	var presets = []

	# Map positions to role preset groups
	var position_role_map = {
		"GK": ["Goalkeeper"],
		"CB": ["CenterBack", "CenterBack_BallPlaying"],
		"LB": ["LeftBack", "LeftBack_Attacking"],
		"RB": ["RightBack", "RightBack_Attacking"],
		"LWB": ["WingBack"],
		"RWB": ["WingBack"],
		"DM": ["DefensiveMidfield", "DefensiveMidfield_Playmaker"],
		"CM": ["CentralMidfield", "CentralMidfield_Attacking"],
		"LM": ["LeftWing", "LeftWing_Wide", "CentralMidfield"],
		"RM": ["RightWing", "RightWing_Wide", "CentralMidfield"],
		"CAM": ["AttackingMidfield"],
		"LW": ["LeftWing", "LeftWing_Wide"],
		"RW": ["RightWing", "RightWing_Wide"],
		"ST": ["Striker", "Striker_Poacher", "Striker_TargetMan", "CenterForward"],
		"CF": ["CenterForward", "FalseNine"]
	}

	var role_ids = position_role_map.get(position, [])
	for role_id in role_ids:
		if ROLE_INSTRUCTION_PRESETS.has(role_id):
			var preset = ROLE_INSTRUCTION_PRESETS[role_id]
			presets.append(
				{
					"id": role_id,
					"name_ko": preset.name_ko,
					"description": preset.description,
					"instructions": preset.instructions.duplicate(true)
				}
			)

	return presets


func apply_role_instruction_preset(slot: int, preset_id: String) -> bool:
	"""Apply a role-based instruction preset to a player"""
	if not ROLE_INSTRUCTION_PRESETS.has(preset_id):
		print("[MyTeamData] Error: Unknown role preset '%s'" % preset_id)
		return false

	var preset = ROLE_INSTRUCTION_PRESETS[preset_id]
	var instructions = preset.instructions.duplicate(true)

	# Apply instructions to player
	var success = set_player_instructions(slot, instructions)
	if success:
		print("[MyTeamData] Applied role preset '%s' (%s) to slot %d" % [preset_id, preset.name_ko, slot])

	return success


# ===== Youth Academy Mode Functions (M1.1) =====


func validate_academy_settings(settings: Dictionary) -> Dictionary:
	"""Validate and sanitize academy settings"""
	var validated = settings.duplicate()

	# Team name: 2-30 characters
	var team_name = str(validated.get("team_name", "My Academy"))
	if team_name.length() < 2 or team_name.length() > 30:
		validated["team_name"] = "My Academy"
		print("[MyTeamData] ⚠️ Team name length invalid, using default")
	else:
		validated["team_name"] = team_name

	# Nickname: 2-15 characters
	var nickname = str(validated.get("nickname", "Youngsters"))
	if nickname.length() < 2 or nickname.length() > 15:
		validated["nickname"] = "Youngsters"
		print("[MyTeamData] ⚠️ Nickname length invalid, using default")
	else:
		validated["nickname"] = nickname

	# Emblem icon: 0-19 (20 icons)
	var emblem_icon = validated.get("emblem_icon", 0)
	if typeof(emblem_icon) != TYPE_INT or emblem_icon < 0 or emblem_icon > 19:
		validated["emblem_icon"] = 0
		print("[MyTeamData] ⚠️ Emblem icon out of range, using default (0)")
	else:
		validated["emblem_icon"] = emblem_icon

	# Emblem background: 0-5 (6 shapes)
	var emblem_background = validated.get("emblem_background", 0)
	if typeof(emblem_background) != TYPE_INT or emblem_background < 0 or emblem_background > 5:
		validated["emblem_background"] = 0
		print("[MyTeamData] ⚠️ Emblem background out of range, using default (0)")
	else:
		validated["emblem_background"] = emblem_background

	# Primary color: Hex format #RRGGBB
	var primary_color = str(validated.get("primary_color", "#FF0000"))
	if not primary_color.is_valid_html_color():
		validated["primary_color"] = "#FF0000"
		print("[MyTeamData] ⚠️ Primary color invalid, using default (red)")
	else:
		validated["primary_color"] = primary_color

	# Secondary color: Hex format #RRGGBB
	var secondary_color = str(validated.get("secondary_color", "#FFFFFF"))
	if not secondary_color.is_valid_html_color():
		validated["secondary_color"] = "#FFFFFF"
		print("[MyTeamData] ⚠️ Secondary color invalid, using default (white)")
	else:
		validated["secondary_color"] = secondary_color

	return validated


# ===== M1.4: 3-Tier Squad System - Promotion Logic =====


func check_promotion_eligibility() -> Dictionary:
	"""Check if squad is eligible for promotion to next level
	Returns: {eligible: bool, next_level: int, current_ca: int, reasons: Array, requirements: Dictionary}
	"""
	var result = {"eligible": false, "next_level": squad_level, "current_ca": 0, "reasons": [], "requirements": {}}

	# Need at least one player to check
	if saved_players.size() == 0:
		result.requirements = {"error": "No players in squad"}
		return result

	# Get main player (assume first player or highest CA player)
	var main_player = _get_main_player()
	if main_player.size() == 0:
		result.requirements = {"error": "No valid player data"}
		return result

	var current_ca = main_player.get("overall", 60)  # Using 'overall' as CA
	result.current_ca = current_ca

	# Check hard gate based on current squad level
	var hard_gate_ca = 0
	var next_level = squad_level
	var level_name = ""

	match squad_level:
		SquadLevel.YOUTH:
			hard_gate_ca = 100
			next_level = SquadLevel.BTEAM
			level_name = "B-Team"
		SquadLevel.BTEAM:
			hard_gate_ca = 115
			next_level = SquadLevel.ATEAM
			level_name = "A-Team"
		SquadLevel.ATEAM:
			# Already at max level
			result.requirements = {
				"ca": {"current": current_ca, "required": hard_gate_ca}, "status": "Already at maximum level (A-Team)"
			}
			return result

	# Hard gate check
	if current_ca < hard_gate_ca:
		result.requirements = {
			"ca": {"current": current_ca, "required": hard_gate_ca}, "deficit": hard_gate_ca - current_ca
		}
		return result

	# Hard gate passed, check soft gates
	var soft_gates_result = calculate_promotion_soft_gates(main_player)
	var soft_gates_met = soft_gates_result.met
	var soft_gate_reasons = soft_gates_result.reasons

	# Need at least 1 soft gate for promotion
	if soft_gates_met >= 1:
		result.eligible = true
		result.next_level = next_level
		result.reasons = soft_gate_reasons
		result.requirements = {
			"ca": {"current": current_ca, "required": hard_gate_ca},
			"soft_gates": {"current": soft_gates_met, "required": 1}
		}
		print(
			(
				"[MyTeamData] ✅ Promotion eligible: %s → %s (CA: %d, Soft gates: %d)"
				% [_get_squad_level_name(squad_level), level_name, current_ca, soft_gates_met]
			)
		)
	else:
		result.requirements = {
			"ca": {"current": current_ca, "required": hard_gate_ca},
			"soft_gates": {"current": 0, "required": 1},
			"hint": "Play more matches or train consistently"
		}

	return result


func calculate_promotion_soft_gates(player: Dictionary) -> Dictionary:
	"""Calculate how many soft gates are met for promotion
	Soft gates:
	1. Recent match performance (avg rating >= 7.2)
	2. Core stat threshold (2+ position-specific stats >= 70)
	3. Recent CA growth (last 8 weeks +6 or more)
	4. Manager development focus (auto-pass for now)
	"""
	var met = 0
	var reasons = []

	# Soft gate 1: Recent match performance
	# Note: We don't have match history yet, so this will be added in future
	# For now, auto-pass this gate to enable testing
	met += 1
	reasons.append("✅ Consistent training effort")

	# Soft gate 2: Core stat threshold
	var position = player.get("position", "ST")
	var core_stats_met = _check_core_stats(player, position)
	if core_stats_met >= 2:
		met += 1
		reasons.append("✅ Core abilities developed (%d stats above threshold)" % core_stats_met)

	# Soft gate 3: Recent CA growth
	# Note: Growth tracking will be added in future with training system
	# For now, check if player has good potential (PA > CA + 20)
	var current_ca = player.get("overall", 60)
	var potential = player.get("potential", current_ca + 20)
	if potential >= current_ca + 20:
		met += 1
		reasons.append("✅ High growth potential (PA: %d)" % potential)

	return {"met": met, "reasons": reasons}


func _check_core_stats(player: Dictionary, position: String) -> int:
	"""Check how many core stats for position are >= 70"""
	var technical = player.get("technical", {})
	var mental = player.get("mental", {})
	var physical = player.get("physical", {})

	var core_stats_above_threshold = 0

	# Define core stats by position (2-3 most important stats)
	var core_stat_names = []
	match position:
		"ST", "CF":
			core_stat_names = ["finishing", "off_the_ball", "composure"]
		"CM":
			core_stat_names = ["passing", "vision", "work_rate"]
		"CB":
			core_stat_names = ["tackling", "marking", "positioning"]
		"GK":
			core_stat_names = ["positioning", "concentration"]
		"LW", "RW":
			core_stat_names = ["dribbling", "pace", "crossing"]
		"CAM":
			core_stat_names = ["passing", "vision", "technique"]
		"CDM", "DM":
			core_stat_names = ["tackling", "positioning", "work_rate"]
		_:
			# Default: check passing and work_rate
			core_stat_names = ["passing", "work_rate"]

	# Check each core stat
	for stat_name in core_stat_names:
		var stat_value = 0
		if technical.has(stat_name):
			stat_value = technical.get(stat_name, 0)
		elif mental.has(stat_name):
			stat_value = mental.get(stat_name, 0)
		elif physical.has(stat_name):
			stat_value = physical.get(stat_name, 0)

		if stat_value >= 70:
			core_stats_above_threshold += 1

	return core_stats_above_threshold


func promote_to_next_squad() -> bool:
	"""Execute promotion to next squad level
	Returns: true if promotion successful, false otherwise
	"""
	var promo_check = check_promotion_eligibility()

	if not promo_check.eligible:
		print("[MyTeamData] ❌ Promotion not eligible")
		return false

	var previous_level = squad_level
	var previous_ca = promo_check.current_ca
	squad_level = promo_check.next_level

	# Save state
	save_to_file()

	# Emit promotion event via EventBus
	var event_bus = get_node_or_null("/root/EventBus")
	if event_bus and event_bus.has_method("pub"):
		event_bus.pub(
			"squad_promoted",
			{
				"previous_level": previous_level,
				"new_level": squad_level,
				"previous_ca": previous_ca,
				"current_ca": promo_check.current_ca,
				"reasons": promo_check.reasons
			}
		)

	print(
		(
			"[MyTeamData] 🎉 PROMOTED: %s → %s!"
			% [_get_squad_level_name(previous_level), _get_squad_level_name(squad_level)]
		)
	)

	return true


func _get_main_player() -> Dictionary:
	"""Get the main player (highest CA player from saved_players)"""
	if saved_players.size() == 0:
		return {}

	# Find player with highest overall
	var main_player = saved_players[0]
	var highest_ca = main_player.get("overall", 0)

	for player in saved_players:
		var player_ca = player.get("overall", 0)
		if player_ca > highest_ca:
			highest_ca = player_ca
			main_player = player

	return main_player


func _get_squad_level_name(level: int) -> String:
	"""Get display name for squad level"""
	match level:
		SquadLevel.YOUTH:
			return "U18 Youth"
		SquadLevel.BTEAM:
			return "B-Team"
		SquadLevel.ATEAM:
			return "A-Team"
	return "Unknown"
