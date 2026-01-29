extends RefCounted
class_name EnhancedCoreMapper
# EnhancedCoreMapper - 42-skill & 21-training 시스템 지원
# v2.0 - Enhanced system with backward compatibility


func _init():
	print("[EnhancedCoreMapper] Initializing...")


# =================== Enhanced 21-Training 변환 ===================


static func enhanced_training_plan_to_json(training_type: String, intensity: float, duration: int = 1) -> Dictionary:
	"""21가지 훈련 타입을 Rust JSON으로 변환"""
	print("[EnhancedCoreMapper] enhanced_training_plan_to_json called with: ", training_type, intensity, duration)

	var result = {"training_type": training_type, "intensity": intensity, "duration": duration}

	print("[EnhancedCoreMapper] Result: ", result)
	return result


static func enhanced_training_result_from_json(json: Dictionary) -> Dictionary:
	"""Rust Enhanced21TrainingResult를 Godot Dictionary로 변환"""
	return {
		"training_type": json.get("training_type", "Recovery"),
		"intensity": json.get("intensity", 1.0),
		"applied": json.get("applied", false),
		"skill_changes": json.get("skill_changes", {}),
		"fatigue_delta": json.get("fatigue_delta", 0.0),
		"morale_delta": json.get("morale_delta", 0.0),
		"injury_occurred": json.get("injury_occurred", false),
		"injury_days_added": json.get("injury_days_added", 0),
		"condition_after": json.get("condition_after", 3),
		"message": json.get("message", "")
	}


static func weekly_training_queue_to_json(training_queue: Array) -> Dictionary:
	"""주간 훈련 큐를 Rust JSON으로 변환"""
	var json_queue = []

	for plan in training_queue:
		if plan == null:
			print("Warning: null plan in training_queue")
			continue

		var plan_json = enhanced_training_plan_to_json(plan.get("type", "Recovery"), plan.get("intensity", 1.0), 1)  # 각 훈련은 1일
		json_queue.append(plan_json)

	return {"weekly_plans": json_queue, "total_days": json_queue.size()}


# =================== Enhanced 42-Skill Player 변환 ===================


static func enhanced_player_to_json(player_data: Dictionary) -> Dictionary:
	"""42-skill 플레이어 데이터를 Rust JSON으로 변환"""
	return {
		"name": player_data.get("player_name", "Player"),
		"position": position_to_enhanced_string(player_data.get("position", 0)),
		"year": player_data.get("year", 1),
		"growth_curve": growth_curve_to_enhanced_string(player_data.get("growth_curve", 1)),
		"skills": enhanced_skills_to_json(player_data.get("stats", {})),
		"fatigue": player_data.get("fatigue", 0.0),
		"morale": player_data.get("morale", 50.0),
		"injury_days": player_data.get("injury_days", 0),
		"condition_level": player_data.get("condition", 3),
		"experience": player_data.get("experience", 0),
		"level": player_data.get("level", 1),
		"potential": player_data.get("potential", 80),
		"save_version": "2.0",
		"created_at": Time.get_unix_time_from_system()
	}


static func enhanced_player_from_json(json: Dictionary) -> Dictionary:
	"""Rust Enhanced Player를 Godot PlayerData로 변환"""
	var player_data = {}

	# 기본 정보
	player_data["player_name"] = json.get("name", "Player")
	player_data["position"] = enhanced_string_to_position(json.get("position", "ST"))
	player_data["year"] = json.get("year", 1)
	player_data["growth_curve"] = enhanced_string_to_growth_curve(json.get("growth_curve", "Balanced"))

	# 42-skill 시스템
	player_data["stats"] = enhanced_skills_from_json(json.get("skills", {}))

	# 상태 정보
	player_data["fatigue"] = json.get("fatigue", 0.0)
	player_data["morale"] = json.get("morale", 50.0)
	player_data["injury_days"] = json.get("injury_days", 0)
	player_data["condition"] = json.get("condition_level", 3)
	player_data["experience"] = json.get("experience", 0)
	player_data["level"] = json.get("level", 1)
	player_data["potential"] = json.get("potential", 80)

	return player_data


static func enhanced_skills_to_json(godot_stats: Dictionary) -> Dictionary:
	"""Godot 스탯을 Rust 42-skill JSON으로 변환"""
	var rust_skills = {"technical": {}, "mental": {}, "physical": {}, "goalkeeper": {}}

	# Technical skills (14개)
	var technical_mapping = {
		"corners": "corners",
		"crossing": "crossing",
		"dribbling": "dribbling",
		"finishing": "finishing",
		"first_touch": "first_touch",
		"free_kicks": "free_kicks",
		"heading": "heading",
		"long_shots": "long_shots",
		"long_throws": "long_throws",
		"marking": "marking",
		"passing": "passing",
		"penalty_taking": "penalty_taking",
		"tackling": "tackling",
		"technique": "technique"
	}

	for rust_name in technical_mapping.keys():
		var godot_name = technical_mapping[rust_name]
		rust_skills.technical[rust_name] = godot_stats.get(godot_name, 50.0)

	# Mental skills (14개)
	var mental_mapping = {
		"aggression": "aggression",
		"anticipation": "anticipation",
		"bravery": "bravery",
		"composure": "composure",
		"concentration": "concentration",
		"decisions": "decisions",
		"determination": "determination",
		"flair": "flair",
		"leadership": "leadership",
		"off_the_ball": "off_the_ball",
		"positioning": "positioning",
		"teamwork": "teamwork",
		"vision": "vision",
		"work_rate": "work_rate"
	}

	for rust_name in mental_mapping.keys():
		var godot_name = mental_mapping[rust_name]
		rust_skills.mental[rust_name] = godot_stats.get(godot_name, 50.0)

	# Physical skills (8개)
	var physical_mapping = {
		"acceleration": "acceleration",
		"agility": "agility",
		"balance": "balance",
		"jumping": "jumping",
		"natural_fitness": "natural_fitness",
		"pace": "pace",
		"stamina": "stamina",
		"strength": "strength"
	}

	for rust_name in physical_mapping.keys():
		var godot_name = physical_mapping[rust_name]
		rust_skills.physical[rust_name] = godot_stats.get(godot_name, 50.0)

	# Goalkeeper skills (10개) - 기본값 30
	var goalkeeper_mapping = {
		"aerial_reach": "aerial_reach",
		"command_of_area": "command_of_area",
		"communication": "communication",
		"eccentricity": "eccentricity",
		"handling": "handling",
		"kicking": "kicking",
		"one_on_ones": "one_on_ones",
		"reflexes": "reflexes",
		"rushing_out": "rushing_out",
		"throwing": "throwing"
	}

	for rust_name in goalkeeper_mapping.keys():
		var godot_name = goalkeeper_mapping[rust_name]
		rust_skills.goalkeeper[rust_name] = godot_stats.get(godot_name, 30.0)

	return rust_skills


static func enhanced_skills_from_json(rust_skills: Dictionary) -> Dictionary:
	"""Rust 42-skill JSON을 Godot 스탯으로 변환"""
	var godot_stats = {}

	# Technical skills 변환
	var technical = rust_skills.get("technical", {})
	for skill_name in technical.keys():
		godot_stats[skill_name] = technical[skill_name]

	# Mental skills 변환
	var mental = rust_skills.get("mental", {})
	for skill_name in mental.keys():
		godot_stats[skill_name] = mental[skill_name]

	# Physical skills 변환
	var physical = rust_skills.get("physical", {})
	for skill_name in physical.keys():
		godot_stats[skill_name] = physical[skill_name]

	# Goalkeeper skills 변환
	var goalkeeper = rust_skills.get("goalkeeper", {})
	for skill_name in goalkeeper.keys():
		godot_stats[skill_name] = goalkeeper[skill_name]

	# 누락된 스킬들을 기본값으로 채우기
	_fill_missing_skills(godot_stats)

	return godot_stats


static func _fill_missing_skills(stats: Dictionary):
	"""누락된 42개 스킬을 기본값으로 채우기"""
	var all_skills = [
		# Technical (14)
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
		"technique",
		# Mental (14)
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
		"work_rate",
		# Physical (8)
		"acceleration",
		"agility",
		"balance",
		"jumping",
		"natural_fitness",
		"pace",
		"stamina",
		"strength",
		# Goalkeeper (10)
		"aerial_reach",
		"command_of_area",
		"communication",
		"eccentricity",
		"handling",
		"kicking",
		"one_on_ones",
		"reflexes",
		"rushing_out",
		"throwing"
	]

	for skill in all_skills:
		if not stats.has(skill):
			# Goalkeeper 스킬은 낮은 기본값
			if (
				skill
				in [
					"aerial_reach",
					"command_of_area",
					"communication",
					"eccentricity",
					"handling",
					"kicking",
					"one_on_ones",
					"reflexes",
					"rushing_out",
					"throwing"
				]
			):
				stats[skill] = 30.0
			else:
				stats[skill] = 50.0


# =================== Enhanced GameState 변환 ===================


static func enhanced_gamestate_to_json(game_data: Dictionary) -> Dictionary:
	"""Enhanced GameState를 Rust JSON으로 변환"""
	return {
		"week": game_data.get("week", 1),
		"player": enhanced_player_to_json(game_data.get("player", {})),
		"rng_seed": game_data.get("rng_seed", 0),
		"balance": enhanced_balance_to_json(game_data.get("balance", {}))
	}


static func enhanced_gamestate_from_json(json: Dictionary) -> Dictionary:
	"""Rust Enhanced GameState를 Godot로 변환"""
	return {
		"week": json.get("week", 1),
		"player": enhanced_player_from_json(json.get("player", {})),
		"rng_seed": json.get("rng_seed", 0),
		"balance": enhanced_balance_from_json(json.get("balance", {}))
	}


static func enhanced_balance_to_json(balance_data: Dictionary) -> Dictionary:
	"""Enhanced Balance를 Rust JSON으로 변환"""
	return {
		"injury_fatigue_threshold": balance_data.get("injury_fatigue_threshold", 85.0),
		"injury_prob_high": balance_data.get("injury_prob_high", 0.10),
		"injury_prob_critical": balance_data.get("injury_prob_critical", 0.30),
		"training_intensity_multipliers": balance_data.get("training_intensity_multipliers", {}),
		"position_training_bonuses": balance_data.get("position_training_bonuses", {}),
		"growth_curve_multipliers": balance_data.get("growth_curve_multipliers", {})
	}


static func enhanced_balance_from_json(json: Dictionary) -> Dictionary:
	"""Rust Enhanced Balance를 Godot로 변환"""
	return {
		"injury_fatigue_threshold": json.get("injury_fatigue_threshold", 85.0),
		"injury_prob_high": json.get("injury_prob_high", 0.10),
		"injury_prob_critical": json.get("injury_prob_critical", 0.30),
		"training_intensity_multipliers": json.get("training_intensity_multipliers", {}),
		"position_training_bonuses": json.get("position_training_bonuses", {}),
		"growth_curve_multipliers": json.get("growth_curve_multipliers", {})
	}


# =================== 21-Training Type 매핑 ===================


static func training_type_to_enhanced_string(godot_type: String) -> String:
	"""Godot 훈련 타입을 Rust Enhanced21TrainingType으로 변환"""
	var mapping = {
		# Legacy 7 types -> Enhanced 21 types 매핑
		"Physical": "Endurance",  # Physical -> Endurance 기본 매핑
		"Technique": "BallControl",  # Technique -> BallControl
		"Shooting": "Shooting",  # 동일
		"Defense": "SetPiecesDefensive",  # Defense -> SetPiecesDefensive
		"Rest": "Recovery",  # Rest -> Recovery
		"GoOut": "Recovery",  # GoOut -> Recovery (휴식 카테고리)
		"Hospital": "Recovery",  # Hospital -> Recovery
		# Enhanced 21 types (직접 매핑)
		"Endurance": "Endurance",
		"Strength": "Strength",
		"Speed": "Speed",
		"Agility": "Agility",
		"Recovery": "Recovery",
		"BallControl": "BallControl",
		"Passing": "Passing",
		"Crossing": "Crossing",
		"SetPieces": "SetPieces",
		"Positioning": "Positioning",
		"TeamShape": "TeamShape",
		"PressingDrills": "PressingDrills",
		"TransitionPlay": "TransitionPlay",
		"SetPiecesDefensive": "SetPiecesDefensive",
		"Concentration": "Concentration",
		"DecisionMaking": "DecisionMaking",
		"Leadership": "Leadership",
		"MatchPreparation": "MatchPreparation",
		"VideoAnalysis": "VideoAnalysis",
		"OpponentSpecific": "OpponentSpecific"
	}

	return mapping.get(godot_type, "Recovery")


static func enhanced_string_to_training_type(rust_type: String) -> String:
	"""Rust Enhanced21TrainingType을 Godot 타입으로 변환"""
	# 21가지 훈련 타입 모두 그대로 반환 (Godot에서도 동일한 이름 사용)
	return rust_type


# =================== Position & Growth Curve 매핑 ===================


static func position_to_enhanced_string(pos: int) -> String:
	"""Godot 포지션을 Rust Enhanced Position으로 변환"""
	match pos:
		0:
			return "ST"
		1:
			return "CAM"
		2:
			return "CM"
		3:
			return "CB"
		4:
			return "GK"  # 새로 추가된 골키퍼
		_:
			return "ST"


static func enhanced_string_to_position(pos: String) -> int:
	"""Rust Enhanced Position을 Godot으로 변환"""
	match pos:
		"ST":
			return 0
		"CAM":
			return 1
		"CM":
			return 2
		"CB":
			return 3
		"GK":
			return 4  # 새로 추가된 골키퍼
		_:
			return 0


static func growth_curve_to_enhanced_string(curve: int) -> String:
	"""Godot 성장 곡선을 Rust Enhanced GrowthCurve로 변환"""
	match curve:
		0:
			return "Early"
		1:
			return "Balanced"
		2:
			return "Late"
		_:
			return "Balanced"


static func enhanced_string_to_growth_curve(curve: String) -> int:
	"""Rust Enhanced GrowthCurve를 Godot으로 변환"""
	match curve:
		"Early":
			return 0
		"Balanced":
			return 1
		"Late":
			return 2
		_:
			return 1


# =================== 세이브 마이그레이션 (12→42 스킬) ===================


func migrate_legacy_to_enhanced(legacy_data: Dictionary) -> Dictionary:
	"""기존 12-skill 데이터를 42-skill로 마이그레이션"""
	print("[EnhancedCoreMapper] Migrating legacy save to 42-skill system")

	var enhanced_data = legacy_data.duplicate(true)

	# 플레이어 데이터 마이그레이션
	if enhanced_data.has("player"):
		enhanced_data["player"] = migrate_legacy_player(enhanced_data["player"])

	# 게임 시스템 데이터 마이그레이션
	enhanced_data["save_version"] = "2.0"
	enhanced_data["migration_timestamp"] = Time.get_unix_time_from_system()
	enhanced_data["migrated_from"] = legacy_data.get("save_version", "1.0")

	print("[EnhancedCoreMapper] Migration completed successfully")
	return enhanced_data


static func migrate_legacy_player(legacy_player: Dictionary) -> Dictionary:
	"""기존 플레이어 데이터를 42-skill로 마이그레이션"""
	var enhanced_player = legacy_player.duplicate(true)

	# 기존 12개 스킬 추출
	var legacy_stats = legacy_player.get("stats", {})
	var enhanced_stats = {}

	# 1:1 매핑되는 스킬들
	var direct_mappings = {
		"first_touch": "first_touch",
		"dribbling": "dribbling",
		"passing": "passing",
		"pace": "pace",
		"acceleration": "acceleration",  # accel -> acceleration
		"stamina": "stamina",
		"finishing": "finishing",
		"vision": "vision",
		"positioning": "positioning",
		"composure": "concentration",  # concentration -> composure
		"tackling": "tackling"
	}

	# 직접 매핑 적용
	for enhanced_name in direct_mappings.keys():
		var legacy_name = direct_mappings[enhanced_name]
		if legacy_name == "acceleration":
			enhanced_stats[enhanced_name] = legacy_stats.get("accel", 50.0)
		elif legacy_name == "composure":
			enhanced_stats[enhanced_name] = legacy_stats.get("concentration", 50.0)
		else:
			enhanced_stats[enhanced_name] = legacy_stats.get(legacy_name, 50.0)

	# 특별 매핑 (shooting -> long_shots)
	enhanced_stats["long_shots"] = legacy_stats.get("shooting", 50.0)

	# 기존 스킬들의 평균치 계산
	var existing_values = []
	for value in enhanced_stats.values():
		if value > 0:
			existing_values.append(value)

	var average_skill = 50.0
	if existing_values.size() > 0:
		average_skill = 0.0
		for val in existing_values:
			average_skill += val
		average_skill /= existing_values.size()

	# 나머지 30개 스킬을 평균값 기준으로 생성
	var missing_skills = [
		# Technical (나머지)
		"corners",
		"crossing",
		"free_kicks",
		"heading",
		"long_throws",
		"marking",
		"penalty_taking",
		"technique",
		# Mental (나머지)
		"aggression",
		"anticipation",
		"bravery",
		"decisions",
		"determination",
		"flair",
		"leadership",
		"off_the_ball",
		"teamwork",
		"work_rate",
		# Physical (나머지)
		"agility",
		"balance",
		"jumping",
		"natural_fitness",
		"strength",
		# Goalkeeper (전체 - 낮은 값)
		"aerial_reach",
		"command_of_area",
		"communication",
		"eccentricity",
		"handling",
		"kicking",
		"one_on_ones",
		"reflexes",
		"rushing_out",
		"throwing"
	]

	# 랜덤 시드를 플레이어 이름 기반으로 생성 (일관성 유지)
	var player_name = legacy_player.get("player_name", "Player")
	var hash_seed = player_name.hash()
	var rng = RandomNumberGenerator.new()
	rng.seed = hash_seed

	for skill_name in missing_skills:
		if (
			skill_name.begins_with("aerial_reach")
			or skill_name.begins_with("command_of_area")
			or skill_name.begins_with("communication")
			or skill_name.begins_with("eccentricity")
			or skill_name.begins_with("handling")
			or skill_name.begins_with("kicking")
			or skill_name.begins_with("one_on_ones")
			or skill_name.begins_with("reflexes")
			or skill_name.begins_with("rushing_out")
			or skill_name.begins_with("throwing")
		):
			# Goalkeeper 스킬은 낮은 값 (20-40)
			enhanced_stats[skill_name] = rng.randf_range(20.0, 40.0)
		else:
			# 일반 스킬은 평균값 ± 15
			enhanced_stats[skill_name] = clamp(average_skill + rng.randf_range(-15.0, 15.0), 1.0, 100.0)

	enhanced_player["stats"] = enhanced_stats
	return enhanced_player


# =================== 세이브 검증 시스템 ===================


static func validate_enhanced_save_data(data: Dictionary) -> Dictionary:
	"""42-skill 세이브 데이터 유효성 검증"""
	var errors = []

	# 필수 필드 체크
	var required_fields = ["save_version", "player"]
	for field in required_fields:
		if not data.has(field):
			errors.append("Missing required field: " + field)

	# 플레이어 데이터 검증
	if data.has("player"):
		var player = data["player"]
		var player_errors = validate_enhanced_player_data(player)
		errors.append_array(player_errors)

	return {"valid": errors.is_empty(), "errors": errors, "skill_count": _count_player_skills(data.get("player", {}))}


static func validate_enhanced_player_data(player: Dictionary) -> Array:
	"""42-skill 플레이어 데이터 검증"""
	var errors = []

	# 필수 플레이어 필드 체크
	var required_player_fields = ["player_name", "stats"]
	for field in required_player_fields:
		if not player.has(field):
			errors.append("Missing player field: " + field)

	# 42개 스킬 유효성 체크
	if player.has("stats"):
		var stats = player["stats"]
		var skill_errors = validate_42_skills(stats)
		errors.append_array(skill_errors)

	return errors


static func validate_42_skills(stats: Dictionary) -> Array:
	"""42개 스킬 유효성 검증"""
	var errors = []

	# 최소 스킬 수 체크 (42개 중 최소 12개는 있어야 함)
	if stats.size() < 12:
		errors.append("Insufficient skills: found %d, minimum 12 required" % stats.size())

	# 스킬 값 범위 체크
	for skill_name in stats.keys():
		var value = stats[skill_name]
		if not (value is float or value is int):
			errors.append("Invalid skill type for %s: %s" % [skill_name, typeof(value)])
		elif value < 1.0 or value > 100.0:
			errors.append("Invalid skill value for %s: %.1f (must be 1-100)" % [skill_name, value])

	return errors


static func _count_player_skills(player: Dictionary) -> int:
	"""플레이어 스킬 개수 계산"""
	var stats = player.get("stats", {})
	return stats.size()


# =================== 디버그 및 유틸리티 ===================


static func debug_print_skill_mapping():
	"""스킬 매핑 디버그 정보 출력"""
	print("\n=== Enhanced 42-Skill Mapping Debug ===")

	var test_legacy = {
		"first_touch": 65.0,
		"dribbling": 70.0,
		"passing": 75.0,
		"pace": 60.0,
		"accel": 65.0,
		"stamina": 80.0,
		"finishing": 70.0,
		"vision": 75.0,
		"positioning": 65.0,
		"concentration": 60.0,
		"shooting": 65.0,
		"tackling": 55.0
	}

	print("Legacy skills (12): ", test_legacy.keys())

	var enhanced = {}
	enhanced["stats"] = test_legacy
	var migrated = migrate_legacy_player(enhanced)

	print("Enhanced skills (42): ", migrated["stats"].keys().size())
	print("Migration successful: ", migrated["stats"].size() >= 42)


static func get_skill_categories() -> Dictionary:
	"""42개 스킬을 카테고리별로 분류하여 반환"""
	return {
		"Technical":
		[
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
		],
		"Mental":
		[
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
		],
		"Physical": ["acceleration", "agility", "balance", "jumping", "natural_fitness", "pace", "stamina", "strength"],
		"Goalkeeper":
		[
			"aerial_reach",
			"command_of_area",
			"communication",
			"eccentricity",
			"handling",
			"kicking",
			"one_on_ones",
			"reflexes",
			"rushing_out",
			"throwing"
		]
	}


static func get_training_categories() -> Dictionary:
	"""21개 훈련을 카테고리별로 분류하여 반환"""
	return {
		"Physical": ["Endurance", "Strength", "Speed", "Agility", "Recovery"],
		"Technical": ["BallControl", "Passing", "Shooting", "Crossing", "SetPieces"],
		"Tactical": ["Positioning", "TeamShape", "PressingDrills", "TransitionPlay", "SetPiecesDefensive"],
		"Mental": ["Concentration", "DecisionMaking", "Leadership"],
		"Match": ["MatchPreparation", "VideoAnalysis", "OpponentSpecific"]
	}


# =================== 통계 및 분석 ===================


static func calculate_enhanced_overall_rating(stats: Dictionary, position: int) -> float:
	"""포지션별 가중치를 적용한 전체 능력치 계산"""
	match position:
		0:  # ST (스트라이커)
			return (
				stats.get("finishing", 50.0) * 0.25
				+ stats.get("dribbling", 50.0) * 0.15
				+ stats.get("pace", 50.0) * 0.15
				+ stats.get("positioning", 50.0) * 0.15
				+ stats.get("off_the_ball", 50.0) * 0.10
				+ stats.get("first_touch", 50.0) * 0.10
				+ stats.get("acceleration", 50.0) * 0.10
			)
		1:  # CAM (공격형 미드필더)
			return (
				stats.get("passing", 50.0) * 0.25
				+ stats.get("vision", 50.0) * 0.20
				+ stats.get("technique", 50.0) * 0.15
				+ stats.get("decisions", 50.0) * 0.15
				+ stats.get("dribbling", 50.0) * 0.10
				+ stats.get("flair", 50.0) * 0.10
				+ stats.get("off_the_ball", 50.0) * 0.05
			)
		2:  # CM (중앙 미드필더)
			return (
				stats.get("passing", 50.0) * 0.20
				+ stats.get("vision", 50.0) * 0.15
				+ stats.get("teamwork", 50.0) * 0.15
				+ stats.get("stamina", 50.0) * 0.15
				+ stats.get("work_rate", 50.0) * 0.15
				+ stats.get("decisions", 50.0) * 0.10
				+ stats.get("technique", 50.0) * 0.10
			)
		3:  # CB (중앙 수비수)
			return (
				stats.get("tackling", 50.0) * 0.25
				+ stats.get("marking", 50.0) * 0.20
				+ stats.get("heading", 50.0) * 0.15
				+ stats.get("positioning", 50.0) * 0.15
				+ stats.get("strength", 50.0) * 0.10
				+ stats.get("bravery", 50.0) * 0.10
				+ stats.get("concentration", 50.0) * 0.05
			)
		4:  # GK (골키퍼)
			return (
				stats.get("reflexes", 30.0) * 0.25
				+ stats.get("handling", 30.0) * 0.20
				+ stats.get("one_on_ones", 30.0) * 0.15
				+ stats.get("command_of_area", 30.0) * 0.15
				+ stats.get("kicking", 30.0) * 0.10
				+ stats.get("aerial_reach", 30.0) * 0.10
				+ stats.get("communication", 30.0) * 0.05
			)
		_:
			# 기본값 (전체 평균)
			var total = 0.0
			var count = 0
			for value in stats.values():
				total += value
				count += 1
			return total / max(count, 1)


static func analyze_skill_distribution(stats: Dictionary) -> Dictionary:
	"""스킬 분포 분석"""
	var categories = get_skill_categories()
	var analysis = {
		"category_averages": {},
		"highest_skills": [],
		"lowest_skills": [],
		"total_skills": stats.size(),
		"average_skill": 0.0
	}

	# 카테고리별 평균 계산
	for category in categories.keys():
		var cat_total = 0.0
		var cat_count = 0
		for skill_name in categories[category]:
			if stats.has(skill_name):
				cat_total += stats[skill_name]
				cat_count += 1
		if cat_count > 0:
			analysis.category_averages[category] = cat_total / cat_count

	# 전체 평균
	var total = 0.0
	for value in stats.values():
		total += value
	analysis.average_skill = total / max(stats.size(), 1)

	# 최고/최저 스킬 (상위/하위 5개)
	var skill_array = []
	for skill_name in stats.keys():
		skill_array.append({"name": skill_name, "value": stats[skill_name]})

	skill_array.sort_custom(func(a, b): return a.value > b.value)

	for i in range(min(5, skill_array.size())):
		analysis.highest_skills.append(skill_array[i])

	skill_array.reverse()
	for i in range(min(5, skill_array.size())):
		analysis.lowest_skills.append(skill_array[i])

	return analysis


# =================== Bridge.gd에서 호출하는 함수들 ===================


func get_enhanced_player_data(current_state: Dictionary) -> Dictionary:
	"""현재 플레이어 데이터를 enhanced 형식으로 반환"""
	var player_data = current_state.get("player", {})

	# 기본 enhanced 형식으로 변환
	if not player_data.has("skills"):
		player_data["skills"] = _get_default_skills()

	return player_data


func player_to_json(player_data: Dictionary) -> Dictionary:
	"""플레이어 데이터를 JSON 형식으로 변환"""
	return player_data.duplicate()


func create_new_enhanced_game(player_name: String, position: String) -> Dictionary:
	"""새로운 enhanced 게임 상태 생성"""
	return {
		"player":
		{"name": player_name, "position": position, "skills": _get_default_skills(), "level": 1, "experience": 0},
		"game_state": "active",
		"week": 1,
		"year": 1
	}


func apply_skill_deltas(player_data: Dictionary, skill_changes: Dictionary) -> Dictionary:
	"""스킬 변화를 플레이어 데이터에 적용"""
	var result = player_data.duplicate()

	if not result.has("skills"):
		result["skills"] = _get_default_skills()

	for skill_name in skill_changes:
		if result["skills"].has(skill_name):
			result["skills"][skill_name] += skill_changes[skill_name]
			# 스킬 값은 0-100 범위로 제한
			result["skills"][skill_name] = max(0, min(100, result["skills"][skill_name]))

	return result


func _get_default_skills() -> Dictionary:
	"""기본 스킬 딕셔너리 반환"""
	return {
		"shooting": 50,
		"passing": 50,
		"dribbling": 50,
		"defending": 50,
		"physical": 50,
		"mental": 50,
		"technique": 50,
		"speed": 50,
		"stamina": 50,
		"goalkeeping": 50
	}
