extends RefCounted
class_name CoreMapper
# CoreMapper - Rust JSON ↔ Godot Resource 변환 중앙화
# v1.1 - 코드 리뷰 반영

# =================== Training 변환 ===================


static func training_plan_to_json(plan: TrainingPlan) -> Dictionary:
	return {"kind": plan.kind, "intensity": plan.intensity}


static func training_result_from_json(json: Dictionary) -> TrainingResult:
	var result = TrainingResult.new()
	result.message = json.get("message", "")
	result.deltas = json.get("deltas", {})
	result.fatigue_delta = json.get("fatigue_delta", 0.0)
	result.injury = json.get("injury", false)
	result.injury_days = json.get("injury_days", 0)
	return result


# =================== Match 변환 ===================


static func match_plan_to_json(plan: MatchPlan) -> Dictionary:
	return {
		"opponent_name": plan.opponent_name,
		"opponent_level": plan.opponent_level,
		"venue": plan.venue,
		"aggression": plan.aggression,
		"tempo": plan.tempo
	}


static func match_result_from_json(json: Dictionary) -> MatchResult:
	var result = preload("res://scripts/model/MatchResult.gd").new()
	result.home_score = json.get("home_score", 0)
	result.away_score = json.get("away_score", 0)
	result.is_player_home = json.get("is_player_home", true)
	result.events = json.get("events", [])
	result.fatigue_delta = json.get("fatigue_delta", 0.0)
	result.morale_delta = json.get("morale_delta", 0.0)
	result.message = json.get("message", "")
	return result


# =================== Player 변환 ===================


static func player_to_json(data: PlayerData) -> Dictionary:
	return {
		"name": data.player_name,
		"pos": position_to_string(data.position),
		"year": data.year,
		"curve": growth_curve_to_string(data.growth_curve),
		"stats": stats_to_json(data.stats),
		"fatigue": data.fatigue,
		"morale": data.morale,
		"injury_days": data.injury_days
	}


static func player_from_json(json: Dictionary) -> void:
	# PlayerData autoload 직접 업데이트
	PlayerData.player_name = json.get("name", "Player")
	PlayerData.position = json.get("pos", "ST")
	PlayerData.year = json.get("year", 1)
	PlayerData.growth_curve = string_to_growth_curve(json.get("curve", "Balanced"))
	PlayerData.stats = stats_from_json(json.get("stats", {}))
	PlayerData.fatigue = json.get("fatigue", 0.0)
	PlayerData.morale = json.get("morale", 50.0)
	PlayerData.injury_days = json.get("injury_days", 0)


# =================== GameState 변환 ===================


static func gamestate_from_json(json: Dictionary) -> void:
	# 플레이어 데이터
	if json.has("player"):
		player_from_json(json["player"])

	# 게임 진행 상태
	if json.has("week"):
		var week_data = json["week"]
		# AcademyCalendar 대신 GameManager 사용 (미구현)
		# AcademyCalendar.year = week_data.get("year", 1)
		# AcademyCalendar.term = week_data.get("term", AcademyCalendar.Term.SPRING)
		# AcademyCalendar.week_in_term = week_data.get("week_in_term", 1)
		# AcademyCalendar.global_week = week_data.get("global_week", 1)

	# 리그 데이터 (LeagueScheduler 미사용)
	if json.has("league"):
		var league = json["league"]
		# LeagueScheduler.current_standings = league.get("standings", {})


static func gamestate_to_json() -> Dictionary:
	return {
		"player": player_to_json(PlayerData),
		"week": {"year": 1, "term": 0, "week_in_term": 1, "global_week": 1},  # AcademyCalendar 대신 기본값  # AcademyCalendar 대신 기본값  # AcademyCalendar 대신 기본값  # AcademyCalendar 대신 기본값
		"league": {"standings": {}},  # LeagueScheduler 대신 빈 딕셔너리
		"save_version": "1.1"
	}


# =================== 헬퍼 함수 ===================


static func position_to_string(pos: String) -> String:
	# 이미 String이므로 그대로 반환
	return pos


static func string_to_position(pos: String) -> int:
	match pos:
		"ST":
			return 0
		"CAM":
			return 1
		"CM":
			return 2
		"CB":
			return 3
		_:
			return 0


static func growth_curve_to_string(curve: int) -> String:
	match curve:
		0:
			return "Early"
		1:
			return "Balanced"
		2:
			return "Late"
		_:
			return "Balanced"


static func string_to_growth_curve(curve: String) -> int:
	match curve:
		"Early":
			return 0
		"Balanced":
			return 1
		"Late":
			return 2
		_:
			return 1


static func stats_to_json(stats: Dictionary) -> Dictionary:
	# Godot → Rust 스탯 매핑
	var rust_stats = {}

	# 직접 매핑
	var direct_maps = [
		"first_touch", "pace", "stamina", "finishing", "vision", "positioning", "decision", "composure", "tackling"
	]
	for stat in direct_maps:
		if stats.has(stat):
			rust_stats[stat] = stats[stat]

	# 이름 변환 필요
	if stats.has("dribbling"):
		rust_stats["dribble"] = stats["dribbling"]
	if stats.has("passing"):
		rust_stats["pass"] = stats["passing"]
	if stats.has("acceleration"):
		rust_stats["accel"] = stats["acceleration"]

	return rust_stats


static func stats_from_json(rust_stats: Dictionary) -> Dictionary:
	# Rust → Godot 스탯 매핑
	var godot_stats = {}

	# 직접 매핑
	var direct_maps = [
		"first_touch", "pace", "stamina", "finishing", "vision", "positioning", "decision", "composure", "tackling"
	]
	for stat in direct_maps:
		if rust_stats.has(stat):
			godot_stats[stat] = rust_stats[stat]

	# 이름 변환 필요
	if rust_stats.has("dribble"):
		godot_stats["dribbling"] = rust_stats["dribble"]
	if rust_stats.has("pass"):
		godot_stats["passing"] = rust_stats["pass"]
	if rust_stats.has("accel"):
		godot_stats["acceleration"] = rust_stats["accel"]

	# 기본값 채우기
	var all_stats = [
		"first_touch",
		"dribbling",
		"passing",
		"pace",
		"acceleration",
		"stamina",
		"finishing",
		"vision",
		"positioning",
		"decision",
		"composure",
		"tackling"
	]
	for stat in all_stats:
		if not godot_stats.has(stat):
			godot_stats[stat] = 50.0

	return godot_stats


# =================== 에러 포맷 통일 ===================


static func create_error_response(message: String, code: String = "UNKNOWN") -> Dictionary:
	return {"ok": false, "error": message, "code": code}


static func create_success_response(data: Dictionary = {}) -> Dictionary:
	var response = {"ok": true}
	for key in data:
		response[key] = data[key]
	return response


# =================== 세이브 마이그레이션 ===================


static func migrate_save_data(data: Dictionary) -> Dictionary:
	"""
	세이브 데이터 마이그레이션 중앙 관리
	버전별 호환성 보장 및 데이터 구조 업데이트
	"""
	var version = data.get("save_version", "0.0")
	var original_version = version

	print("[CoreMapper] Migrating save data from version: ", version)

	match version:
		"0.0":
			# v0 → v1.0 마이그레이션
			data = _migrate_v0_to_v1(data)
			version = "1.0"
		"1.0":
			# v1.0 → v1.1 마이그레이션
			data = _migrate_v1_to_v1_1(data)
			version = "1.1"

	# 마이그레이션 로그
	if version != original_version:
		var log_message = "Save migration completed: %s → %s" % [original_version, version]
		print("[CoreMapper] " + log_message)

		# 파일 로그 기록
		_append_migration_log(log_message + " OK")

		# 세이브 데이터에도 로그 추가
		data["migration_log"] = data.get("migration_log", [])
		data["migration_log"].append(
			{"from": original_version, "to": version, "timestamp": Time.get_unix_time_from_system()}
		)

	data["save_version"] = version
	return data


static func _migrate_v0_to_v1(data: Dictionary) -> Dictionary:
	print("[CoreMapper] Applying v0 → v1.0 migration")

	# day → week 용어 변경
	if data.has("day"):
		print("  - Converting 'day' to 'week'")
		data["week"] = data["day"]
		data.erase("day")

	# 52주 시스템 추가
	if not data.has("calendar"):
		print("  - Adding 52-week calendar system")
		data["calendar"] = {"year": 1, "term": "spring", "week_in_term": 1, "global_week": 1}

	# 레거시 스탯 매핑 변경
	if data.has("player") and data["player"].has("stats"):
		var stats = data["player"]["stats"]
		# dribbling → dribble 등의 변환
		if stats.has("dribbling"):
			stats["dribble"] = stats["dribbling"]
			stats.erase("dribbling")
		if stats.has("passing"):
			stats["pass"] = stats["passing"]
			stats.erase("passing")
		if stats.has("acceleration"):
			stats["accel"] = stats["acceleration"]
			stats.erase("acceleration")
		print("  - Updated stat mappings")

	return data


static func _migrate_v1_to_v1_1(data: Dictionary) -> Dictionary:
	print("[CoreMapper] Applying v1.0 → v1.1 migration")

	# Balance 설정 추가
	if not data.has("balance"):
		print("  - Adding Balance configuration")
		data["balance"] = {
			"injury_fatigue_threshold": 85.0,
			"injury_prob_high": 0.10,
			"injury_prob_critical": 0.30,
			"training_fatigue_base": 8.0,
			"rest_recovery": 15.0,
			"match_fatigue_base": 15.0,
			"match_morale_win": 10.0,
			"match_morale_lose": -5.0,
			"condition_threshold_5": 20.0,
			"condition_threshold_4": 40.0,
			"condition_threshold_3": 60.0,
			"condition_threshold_2": 80.0
		}

	# 코치 시스템 호환성
	if not data.has("coach"):
		print("  - Adding default coach system")
		data["coach"] = {"name": "기본 감독", "specialty": "Balanced", "morale_bonus": 1.0}

	# 리그 스케줄러 데이터 구조 추가
	if not data.has("league_schedule"):
		print("  - Adding league schedule structure")
		data["league_schedule"] = {"teams": ["High FC"], "current_season": 1, "matches_played": 0}

	return data


# =================== 세이브 검증 ===================


static func validate_save_data(data: Dictionary) -> Dictionary:
	"""
	세이브 데이터 유효성 검증
	필수 필드 존재 여부 및 데이터 타입 체크
	"""
	var errors = []

	# 필수 필드 체크
	var required_fields = ["save_version", "player"]
	for field in required_fields:
		if not data.has(field):
			errors.append("Missing required field: " + field)

	# 플레이어 데이터 검증
	if data.has("player"):
		var player = data["player"]
		var required_player_fields = ["name", "stats", "fatigue", "morale"]
		for field in required_player_fields:
			if not player.has(field):
				errors.append("Missing player field: " + field)

	# 스탯 범위 검증
	if data.has("player") and data["player"].has("stats"):
		var stats = data["player"]["stats"]
		for stat_name in stats:
			var value = stats[stat_name]
			if not (value is float or value is int) or value < 0 or value > 100:
				errors.append("Invalid stat value for %s: %s" % [stat_name, value])

	return {"valid": errors.is_empty(), "errors": errors}


# =================== 백업 시스템 ===================


static func create_backup_before_migration(data: Dictionary, slot: int = 1) -> bool:
	"""
	마이그레이션 전 백업 생성
	실패 시 원본 복구 가능
	"""
	var backup_path = "user://save_%d_backup_%d.json" % [slot, Time.get_unix_time_from_system()]
	var file = FileAccess.open(backup_path, FileAccess.WRITE)
	if file == null:
		print("[CoreMapper] Failed to create backup at: ", backup_path)
		_append_migration_log("Backup creation FAIL (slot %d)" % slot)
		return false

	file.store_string(JSON.stringify(data))
	file.close()
	print("[CoreMapper] Backup created: ", backup_path)
	_append_migration_log("Backup created for slot %d OK" % slot)
	return true


# =================== 마이그레이션 로그 시스템 ===================


static func _append_migration_log(message: String):
	"""
	마이그레이션 로그를 파일에 기록
	DevTools에서 확인 가능
	"""
	var log_dir = "user://logs"
	var log_path = log_dir + "/migration.log"

	# 로그 디렉터리 생성
	if not DirAccess.dir_exists_absolute(log_dir):
		DirAccess.open("user://").make_dir("logs")

	var file = FileAccess.open(log_path, FileAccess.WRITE)
	if not file:
		# 파일이 없으면 새로 생성
		file = FileAccess.open(log_path, FileAccess.WRITE)

	if file:
		# 기존 내용 읽기
		var existing_content = ""
		file.close()

		var read_file = FileAccess.open(log_path, FileAccess.READ)
		if read_file:
			existing_content = read_file.get_as_text()
			read_file.close()

		# 새 내용 추가
		var timestamp = Time.get_datetime_string_from_system()
		var log_line = "[%s] %s\n" % [timestamp, message]

		var write_file = FileAccess.open(log_path, FileAccess.WRITE)
		if write_file:
			write_file.store_string(existing_content + log_line)
			write_file.close()


static func get_migration_log_path() -> String:
	"""DevTools에서 사용할 로그 경로 반환"""
	return "user://logs/migration.log"
