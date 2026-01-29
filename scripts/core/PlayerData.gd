extends Node
# PlayerData - 플레이어 데이터 관리 시스템

signal stats_changed(changes: Dictionary)

const CA_VALIDATOR = preload("res://scripts/character_creation/CAValidator.gd")

# 플레이어 기본 정보
var player_name: String = "플레이어"
var age: int = 18
var position: String = "CM"  # 직접 선택된 포지션 (ST, LM, CM, RM, LB, CB, RB)
var hero_uid: int = 1  # MVP 육성 모드 주인공 UID (슬롯 로컬 ID)

# 42개 스탯 시스템 (CLAUDE2_IMPLEMENTATION.md 명세에 따름)
# Technical Skills (14개)
var technical_stats = {
	"corners": 15,
	"crossing": 15,
	"dribbling": 15,
	"finishing": 15,
	"first_touch": 15,
	"free_kicks": 15,
	"heading": 15,
	"long_shots": 15,
	"long_throws": 15,
	"marking": 15,
	"passing": 15,
	"penalty_kicks": 15,
	"tackling": 15,
	"technique": 15
}

# Mental Skills (14개)
var mental_stats = {
	"aggression": 15,
	"anticipation": 15,
	"bravery": 15,
	"composure": 15,
	"concentration": 15,
	"decisions": 15,
	"determination": 15,
	"flair": 15,
	"leadership": 15,
	"off_the_ball": 15,
	"positioning": 15,
	"teamwork": 15,
	"vision": 15,
	"work_rate": 15
}

# Physical Skills (8개)
var physical_stats = {
	"acceleration": 15,
	"agility": 15,
	"balance": 15,
	"jumping": 15,
	"natural_fitness": 15,
	"pace": 15,
	"stamina": 15,
	"strength": 15
}

# Goalkeeper Skills (6개)
var goalkeeper_stats = {
	"aerial_reach": 15, "command_of_area": 15, "communication": 15, "eccentricity": 15, "handling": 15, "kicking": 15
}

# detailed_attributes - 외부에서 접근 가능한 통합 능력치 딕셔너리
var detailed_attributes: Dictionary = {}
var current_ca: int = 50
var potential_ability: int = 199

# ========== Phase 4: Personality System (Pawapuro 스타일 8개 성격 능력치) ==========
# 0-100 범위, Uma Musume 스타일 trust와는 별개의 시스템
# 훈련 효율성에 영향 (0.7x ~ 1.3x multiplier)
var personality_adaptability: int = 50  # 적응력 (0-100)
var personality_ambition: int = 50  # 야망 (0-100)
var personality_determination: int = 50  # 결단력 (0-100)
var personality_discipline: int = 50  # 규율 (0-100)
var personality_loyalty: int = 50  # 충성도 (0-100)
var personality_pressure: int = 50  # 압박 처리 (0-100)
var personality_professionalism: int = 50  # 프로정신 (0-100)
var personality_temperament: int = 50  # 성격/기질 (0-100)
var personality_archetype: String = "Steady"  # 성격 유형 (Leader, Genius, Workhorse, Rebel, Steady)

# ========== Phase 3: Special Abilities (파워프로 스타일 특수능력) ==========
# Bronze → Silver → Gold → Diamond → Legend 조합 시스템
var special_abilities: Array = []  # Array[Dictionary]: {ability_type: String, tier: String}

# ========== Phase 2: Exclusive Trait System (루트 전용 특성) ==========
# 한 플레이당 1개만 획득 가능한 영구 패시브 버프
var exclusive_trait: String = ""  # "rival_awakening", "team_chemistry", "tactical_understanding", "leadership", "iron_defense"
var exclusive_trait_level: int = 0  # 1~3 (분기점마다 레벨업)

# 게임 상태
var fatigue: float = 0.0
var condition: int = 3  # 1-5 (Terrible to Perfect)
var current_week: int = 1
var current_year: int = 1
var last_match_rating: float = 0.0  # 최근 경기 평점(0.0~10.0)

# ========== Phase 5.5: Hero Growth XP Overflow ==========
# Hero Time 액션으로 누적된 XP 중 스탯으로 변환되지 못한 잔여분
# 다음 경기에서 이어서 누적됨
var xp_overflow: Dictionary = {}  # { "passing": 5.3, "vision": 2.1, ... }


func _ready():
	print("[PlayerData] Initializing...")

	# GlobalCharacterData에서 캐릭터 생성 데이터 가져오기
	if GlobalCharacterData and GlobalCharacterData.character_data.size() > 0:
		_load_from_character_creation()

	# Phase 4: 성격 초기화 (캐릭터 생성에서 설정되지 않은 경우 랜덤 생성)
	if personality_archetype == "Steady" and personality_discipline == 50:
		_initialize_personality_random()

	_recalculate_openfootball_ca()

	# Phase 22: Connect to MatchManager for Hero Time growth (deferred for safety)
	call_deferred("_connect_to_match_manager")

	print("[PlayerData] Initialized with name: ", player_name, " | Archetype: ", personality_archetype)


func _get_color_system():
	# ColorSystem 노드 가져오기
	var scene_tree = Engine.get_main_loop()
	if scene_tree:
		var color_system = scene_tree.root.get_node_or_null("ColorSystem")
		if color_system:
			return color_system
	return null


func get_overall_rating() -> int:
	# 전체 능력치 계산 (42개 능력치)
	var total = 0
	var count = 0

	for category in [technical_stats, mental_stats, physical_stats, goalkeeper_stats]:
		for stat in category.values():
			total += stat
			count += 1

	return int(total / count) if count > 0 else 0


func get_grade(value: float) -> String:
	# Power Pro 스타일 등급 계산 (F~S)
	if value >= 90:
		return "S"
	elif value >= 80:
		return "A"
	elif value >= 70:
		return "B"
	elif value >= 60:
		return "C"
	elif value >= 50:
		return "D"
	elif value >= 40:
		return "E"
	elif value >= 30:
		return "F"
	else:
		return "G"


func rest_player():
	# 휴식 처리
	fatigue = max(0, fatigue - 20)
	condition = min(5, condition + 1)

	print("[PlayerData] Player rested. Fatigue: ", fatigue, " Condition: ", condition)


func get_player_name() -> String:
	return player_name


func get_uid() -> String:
	# 현재 MVP 세션에서 사용하는 주인공 UID
	# MVP mode uses "grad:" prefix for graduated players
	return "grad:%d" % hero_uid


func get_hero_uid_raw() -> int:
	# Raw integer UID for legacy compatibility
	return hero_uid


func get_position() -> String:
	"""플레이어 포지션 반환"""
	return position


func reload_from_global() -> void:
	"""Reload player information from GlobalCharacterData after character creation"""
	_load_from_character_creation()
	_recalculate_openfootball_ca()


func _load_from_character_creation():
	"""GlobalCharacterData에서 데이터 로드"""
	var char_data = GlobalCharacterData.character_data

	# 기본 정보 로드
	if char_data.has("basic_info"):
		var basic = char_data.basic_info
		if basic.has("name"):
			player_name = basic.name
		if basic.has("birthday"):
			# 생년월일로부터 나이 계산 (간단히 18세로 고정)
			age = 18
		if basic.has("position"):
			# 직접 선택된 포지션 사용
			position = basic.position

	# 능력치 로드 (detailed_attributes가 최종 능력치를 포함)
	if char_data.has("detailed_attributes"):
		_load_detailed_attributes(char_data.detailed_attributes)

	# 캐릭터 생성에서 계산된 최종 CA/PA가 있으면 참고
	if char_data.has("final_ca"):
		current_ca = int(char_data.final_ca)
	potential_ability = 199

	print("[PlayerData] Loaded character data from creation")
	print("  Name: ", player_name)
	print("  Position: ", position)
	print("  Overall Rating: ", get_overall_rating())
	print("  OpenFootball CA: ", current_ca)


func _position_to_category(pos: String) -> String:
	"""포지션을 카테고리로 변환 (CA 계산용)"""
	match pos:
		"ST":
			return "공격수"
		"LM", "CM", "RM":
			return "미드필더"
		"LB", "CB", "RB":
			return "수비수"
		"GK":
			return "골키퍼"
		_:
			return "미드필더"


func _load_detailed_attributes(attributes: Dictionary):
	"""최종 능력치 직접 로드"""
	# detailed_attributes 프로퍼티에 저장 (평탄화 형태 유지)
	detailed_attributes = attributes.duplicate(true)

	var has_category_keys := (
		attributes.has("technical")
		or attributes.has("mental")
		or attributes.has("physical")
		or attributes.has("goalkeeper")
	)

	if has_category_keys:
		if attributes.has("technical"):
			for stat_name in attributes.technical:
				if stat_name in technical_stats:
					technical_stats[stat_name] = attributes.technical[stat_name]

		if attributes.has("mental"):
			for stat_name in attributes.mental:
				if stat_name in mental_stats:
					mental_stats[stat_name] = attributes.mental[stat_name]

		if attributes.has("physical"):
			for stat_name in attributes.physical:
				if stat_name in physical_stats:
					physical_stats[stat_name] = attributes.physical[stat_name]

		if attributes.has("goalkeeper"):
			for stat_name in attributes.goalkeeper:
				if stat_name in goalkeeper_stats:
					goalkeeper_stats[stat_name] = attributes.goalkeeper[stat_name]
	else:
		# Attributes are already flattened – map them into category dictionaries
		for stat_name in technical_stats.keys():
			if attributes.has(stat_name):
				technical_stats[stat_name] = int(attributes[stat_name])

		for stat_name in mental_stats.keys():
			if attributes.has(stat_name):
				mental_stats[stat_name] = int(attributes[stat_name])

		# physical stats may use "pace" instead of "speed" depending on source
		for stat_name in physical_stats.keys():
			if stat_name == "pace":
				if attributes.has("pace"):
					physical_stats[stat_name] = int(attributes["pace"])
				elif attributes.has("speed"):
					physical_stats[stat_name] = int(attributes["speed"])
			elif attributes.has(stat_name):
				physical_stats[stat_name] = int(attributes[stat_name])

		for stat_name in goalkeeper_stats.keys():
			if attributes.has(stat_name):
				goalkeeper_stats[stat_name] = int(attributes[stat_name])

	print("[PlayerData] Loaded detailed attributes from character creation")


func _set_stat_value(stat_name: String, value: int):
	"""스탯 값 설정 (모든 카테고리 검색)"""
	# 각 스탯 그룹에서 찾아서 설정
	if stat_name in technical_stats:
		technical_stats[stat_name] = value
	elif stat_name in mental_stats:
		mental_stats[stat_name] = value
	elif stat_name in physical_stats:
		physical_stats[stat_name] = value
	elif stat_name in goalkeeper_stats:
		goalkeeper_stats[stat_name] = value


func get_player_data() -> Dictionary:
	"""플레이어 전체 데이터 반환 (MatchSimulationManager용 + Phase 4: Personality 포함)"""
	_recalculate_openfootball_ca()

	return {
		"name": player_name,
		"age": age,
		"position": position,
		"position_category": _position_to_category(position),
		"technical": technical_stats,
		"mental": mental_stats,
		"physical": physical_stats,
		"goalkeeper": goalkeeper_stats,
		"fatigue": fatigue,
		"condition": condition,
		"overall": get_overall_rating(),
		"ca": current_ca,
		"pa": potential_ability,
		# Phase 4: Personality
		"personality": get_personality_dict(),
		"personality_archetype": personality_archetype,
		# Phase 3: Special Abilities
		"special_abilities": special_abilities.duplicate(true),
		# Phase 2: Exclusive Trait
		"exclusive_trait": exclusive_trait,
		"exclusive_trait_level": exclusive_trait_level,
		# Appearance
		"appearance": _get_appearance_from_global_character_data()
	}


func _get_appearance_from_global_character_data() -> Dictionary:
	"""Get appearance data from GlobalCharacterData or return defaults"""
	if GlobalCharacterData and GlobalCharacterData.character_data.has("appearance"):
		return GlobalCharacterData.character_data.appearance

	# Fallback defaults
	return {"hair_color": "#8B4513", "skin_color": "#FFDBAC", "eye_color": "#8B4513"}


func add_reputation(amount: float):
	"""평판 추가 (MandatoryTeamTrainingManager용)"""
	# 간단한 구현 - 추후 확장 가능
	print("[PlayerData] Reputation increased by %.1f" % amount)


# ========================================
# Phase 6C: OpenFootball CorePlayer Conversion
# ========================================


func to_core_player_format() -> Dictionary:
	"""
	Convert PlayerData to OpenFootball CorePlayer JSON format
	Uses OpenFootballAPI.convert_player_to_core_format() for accurate mapping

	CorePlayer format (crates/of_core/src/player/core_player.rs):
	{
		"uid": int,
		"name": String,
		"ca": int,           # Current Ability (calculated from attributes)
		"pa": int,           # Potential Ability (estimated)
		"position": String,  # Single letter: G/D/M/F
		"age": int,
		"stamina": int,      # 0-100
		"form": int,         # 0-100
		"morale": int,       # 0-100
		"condition": int,    # 0-100 (converted from 1-5 scale)
		"attributes": {      # 36 OpenFootball skills (0-20)
			"pace": int, "acceleration": int, "stamina": int, "strength": int,
			"agility": int, "balance": int, "jumping": int, "natural_fitness": int,
			"corners": int, "crossing": int, "dribbling": int, "finishing": int,
			"first_touch": int, "free_kick_taking": int, "heading": int, "long_shots": int,
			"long_throws": int, "marking": int, "passing": int, "penalty_taking": int,
			"tackling": int, "technique": int, "aggression": int, "anticipation": int,
			"bravery": int, "composure": int, "concentration": int, "decisions": int,
			"determination": int, "flair": int, "leadership": int, "off_the_ball": int,
			"positioning": int, "teamwork": int, "vision": int, "work_rate": int
		}
	}
	"""
	# Use OpenFootballAPI for conversion (handles 42→36 mapping and CA calculation)
	if has_node("/root/OpenFootballAPI"):
		var api = get_node("/root/OpenFootballAPI")
		var godot_player_data = get_player_data()
		return api.convert_player_to_core_format(godot_player_data)
	else:
		push_error("[PlayerData] OpenFootballAPI not available for CorePlayer conversion!")
		# Fallback: manual conversion (less accurate)
		return _manual_core_player_conversion()


func _manual_core_player_conversion() -> Dictionary:
	"""Fallback CorePlayer conversion when OpenFootballAPI unavailable"""
	# Convert position to single letter
	var of_position = "M"  # Default midfielder
	match position:
		"GK":
			of_position = "G"
		"CB", "LB", "RB", "LWB", "RWB", "SW":
			of_position = "D"
		"CM", "CDM", "CAM", "LM", "RM":
			of_position = "M"
		"ST", "CF", "LW", "RW":
			of_position = "F"

	# Convert condition (1-5) to stamina-like scale (0-100)
	var of_condition = (condition - 1) * 25  # 1→0, 2→25, 3→50, 4→75, 5→100

	# Simple CA estimation (average of all stats)
	var total = 0
	var count = 0
	for category in [technical_stats, mental_stats, physical_stats]:
		for value in category.values():
			total += value
			count += 1
	var estimated_ca = int(total / count) if count > 0 else 50
	if current_ca > 0:
		estimated_ca = current_ca

	# Build attributes (36 OpenFootball skills from 42 Godot stats)
	var attributes: Dictionary = _assemble_core_attributes_from_stats()

	return {
		"uid": hero_uid,  # Player UID (should be unique in real usage)
		"name": player_name,
		"ca": estimated_ca,
		"pa": potential_ability,
		"position": of_position,
		"age": age,
		"stamina": 100 - int(fatigue),  # Convert fatigue to stamina
		"form": 50,  # Default form (could be tracked separately)
		"morale": 50,  # Default morale (could be tracked separately)
		"condition": of_condition,
		"attributes": attributes
	}


# ========================================
# End Phase 6C CorePlayer Conversion
# ========================================


func get_current_week() -> int:
	return current_week


func get_condition_text() -> String:
	# 컨디션 텍스트 반환
	match condition:
		1:
			return "Terrible"
		2:
			return "Poor"
		3:
			return "Normal"
		4:
			return "Good"
		5:
			return "Perfect"
		_:
			return "Normal"


func get_condition_color() -> Color:
	# 컨디션 색상 반환
	var color_system = _get_color_system()
	if color_system:
		return color_system.get_condition_color(condition)
	else:
		# 기본 색상
		match condition:
			1:
				return Color(0.8, 0.2, 0.2)  # 빨강
			2:
				return Color(0.9, 0.5, 0.2)  # 주황
			3:
				return Color(0.9, 0.9, 0.2)  # 노랑
			4:
				return Color(0.2, 0.8, 0.2)  # 초록
			5:
				return Color(0.2, 0.6, 1.0)  # 파랑
			_:
				return Color(0.9, 0.9, 0.2)  # 기본 노랑


func get_fatigue_percentage() -> float:
	# 피로도 퍼센트 반환
	return fatigue / 100.0


func get_fatigue_color() -> Color:
	# 피로도 색상 반환
	var color_system = _get_color_system()
	if color_system:
		return color_system.get_fatigue_color(fatigue)
	else:
		# 기본 색상
		if fatigue >= 80:
			return Color(0.8, 0.2, 0.2)  # 빨강
		elif fatigue >= 60:
			return Color(0.9, 0.5, 0.2)  # 주황
		else:
			return Color(0.2, 0.8, 0.2)  # 초록


func add_fatigue(amount: float):
	# 피로도 추가
	fatigue = min(100.0, fatigue + amount)
	print("[PlayerData] Fatigue added: ", amount, " Total: ", fatigue)


func reduce_fatigue(amount: float):
	# 피로도 감소
	fatigue = max(0.0, fatigue - amount)
	print("[PlayerData] Fatigue reduced: ", amount, " Total: ", fatigue)


func set_stat(category: String, stat_name: String, value: int):
	# 스탯 설정 (0-100 범위)
	value = clamp(value, 0, 100)

	match category:
		"technical":
			if stat_name in technical_stats:
				technical_stats[stat_name] = value
		"mental":
			if stat_name in mental_stats:
				mental_stats[stat_name] = value
		"physical":
			if stat_name in physical_stats:
				physical_stats[stat_name] = value
		"goalkeeper":
			if stat_name in goalkeeper_stats:
				goalkeeper_stats[stat_name] = value


func get_stat(category: String, stat_name: String) -> int:
	# 스탯 조회
	match category:
		"technical":
			return technical_stats.get(stat_name, 0)
		"mental":
			return mental_stats.get(stat_name, 0)
		"physical":
			return physical_stats.get(stat_name, 0)
		"goalkeeper":
			return goalkeeper_stats.get(stat_name, 0)
		_:
			return 0


func get_all_stats() -> Dictionary:
	# 모든 42개 스탯 반환
	return {
		"technical": technical_stats, "mental": mental_stats, "physical": physical_stats, "goalkeeper": goalkeeper_stats
	}


func increase_attribute(attr_name: String, delta: int):
	"""
	Increases a specific player attribute by a given delta.
	Finds the attribute across all categories and updates its value, clamping between 0-100.
	"""
	var found = false
	var category = ""
	var stat_dict = null

	# Check technical stats
	if technical_stats.has(attr_name):
		stat_dict = technical_stats
		category = "technical"
		found = true
	# Check mental stats
	elif mental_stats.has(attr_name):
		stat_dict = mental_stats
		category = "mental"
		found = true
	# Check physical stats
	elif physical_stats.has(attr_name):
		stat_dict = physical_stats
		category = "physical"
		found = true
	# Check goalkeeper stats
	elif goalkeeper_stats.has(attr_name):
		stat_dict = goalkeeper_stats
		category = "goalkeeper"
		found = true

	if found:
		var current_value = stat_dict[attr_name]
		var new_value = clamp(current_value + delta, 0, 100)
		stat_dict[attr_name] = new_value
		print("[PlayerData] Increased %s in %s by %+d (new value: %d)" % [attr_name, category, delta, new_value])
		# Emit signal for UI and other systems to react
		stats_changed.emit({attr_name: new_value})
	else:
		push_warning("[PlayerData] Attempted to increase unknown attribute: %s" % attr_name)


func _recalculate_openfootball_ca() -> void:
	"""OpenFootball 공식으로 현재 CA 재계산"""
	var attribute_map = _build_ca_attribute_map()
	if attribute_map.is_empty():
		return

	var pos_category = _position_to_category(position)
	var calc_result = CA_VALIDATOR.calculate_ca_openfootball_accurate(attribute_map, pos_category)
	current_ca = int(calc_result.get("ca", get_overall_rating()))
	potential_ability = 199  # 유저 생성 선수 PA는 고정값


func _build_ca_attribute_map() -> Dictionary:
	"""CAValidator에 전달할 평탄화된 속성 맵 생성"""
	var attr_map: Dictionary = {}

	if not detailed_attributes.is_empty():
		attr_map = detailed_attributes.duplicate(true)

	for key in technical_stats.keys():
		attr_map[key] = int(technical_stats[key])
	for key in mental_stats.keys():
		attr_map[key] = int(mental_stats[key])
	for key in physical_stats.keys():
		attr_map[key] = int(physical_stats[key])
	for key in goalkeeper_stats.keys():
		attr_map[key] = int(goalkeeper_stats[key])

	if not attr_map.has("penalties") and attr_map.has("penalty_kicks"):
		attr_map["penalties"] = int(attr_map["penalty_kicks"])
	if not attr_map.has("throw_ins") and attr_map.has("long_throws"):
		attr_map["throw_ins"] = int(attr_map["long_throws"])
	if not attr_map.has("speed") and attr_map.has("pace"):
		attr_map["speed"] = int(attr_map["pace"])
	if not attr_map.has("ball_control") and attr_map.has("first_touch"):
		attr_map["ball_control"] = int(attr_map["first_touch"])
	if not attr_map.has("shooting") and attr_map.has("finishing"):
		attr_map["shooting"] = int(attr_map["finishing"])

	return attr_map


func apply_training_xp(training_type: String, xp: float):
	# 훈련 XP 적용 (TrainingManager에서 호출)
	var affected_stats = get_affected_stats_by_training(training_type)
	for stat_data in affected_stats:
		var category = stat_data.category
		var stat_name = stat_data.stat
		var current_value = get_stat(category, stat_name)
		var new_value = min(100, current_value + xp * 0.1)
		set_stat(category, stat_name, int(new_value))


func get_affected_stats_by_training(training_type: String) -> Array:
	# 21가지 훈련 타입별로 영향받는 스탯 반환 (CLAUDE2_IMPLEMENTATION.md 명세)
	match training_type:
		# Physical Training (5가지)
		"Physical_Endurance":
			return [{"category": "physical", "stat": "stamina"}, {"category": "physical", "stat": "natural_fitness"}]
		"Physical_Strength":
			return [{"category": "physical", "stat": "strength"}, {"category": "physical", "stat": "jumping"}]
		"Physical_Speed":
			return [{"category": "physical", "stat": "pace"}, {"category": "physical", "stat": "acceleration"}]
		"Physical_Agility":
			return [{"category": "physical", "stat": "agility"}, {"category": "physical", "stat": "balance"}]
		"Physical_Recovery":
			return []  # 회복 훈련은 피로도만 감소, 스탯 증가 없음

		# Technical Training (5가지)
		"Technical_BallControl":
			return [{"category": "technical", "stat": "dribbling"}, {"category": "technical", "stat": "first_touch"}]
		"Technical_Passing":
			return [{"category": "technical", "stat": "passing"}, {"category": "mental", "stat": "vision"}]
		"Technical_Shooting":
			return [{"category": "technical", "stat": "finishing"}, {"category": "technical", "stat": "long_shots"}]
		"Technical_Crossing":
			return [{"category": "technical", "stat": "crossing"}, {"category": "technical", "stat": "technique"}]
		"Technical_SetPieces":
			return [{"category": "technical", "stat": "free_kicks"}, {"category": "technical", "stat": "penalty_kicks"}]

		# Tactical Training (5가지)
		"Tactical_Positioning":
			return [{"category": "mental", "stat": "positioning"}, {"category": "mental", "stat": "anticipation"}]
		"Tactical_TeamShape":
			return [{"category": "mental", "stat": "teamwork"}, {"category": "mental", "stat": "decisions"}]
		"Tactical_PressingDrills":
			return [{"category": "technical", "stat": "tackling"}, {"category": "mental", "stat": "work_rate"}]
		"Tactical_TransitionPlay":
			return [{"category": "mental", "stat": "decisions"}, {"category": "physical", "stat": "pace"}]
		"Tactical_SetPiecesDefensive":
			return [{"category": "technical", "stat": "marking"}, {"category": "mental", "stat": "concentration"}]

		# Mental Training (3가지)
		"Mental_Concentration":
			return [{"category": "mental", "stat": "concentration"}, {"category": "mental", "stat": "composure"}]
		"Mental_DecisionMaking":
			return [{"category": "mental", "stat": "decisions"}, {"category": "mental", "stat": "anticipation"}]
		"Mental_Leadership":
			return [{"category": "mental", "stat": "leadership"}, {"category": "mental", "stat": "determination"}]

		# Match Preparation (3가지)
		"Match_Preparation":
			return [{"category": "mental", "stat": "composure"}, {"category": "mental", "stat": "concentration"}]
		"Match_VideoAnalysis":
			return [{"category": "mental", "stat": "vision"}, {"category": "mental", "stat": "anticipation"}]
		"Match_OpponentSpecific":
			return [{"category": "mental", "stat": "decisions"}, {"category": "technical", "stat": "marking"}]

		_:
			return []


## ========== Phase 4: Personality System Functions ==========


func _initialize_personality_random() -> void:
	"""랜덤 성격 생성 (GDScript implementation - Rust not yet available)"""
	# 랜덤 archetype 선택
	var archetypes = ["Leader", "Genius", "Workhorse", "Rebel", "Steady"]
	var random_archetype = archetypes[randi() % archetypes.size()]
	var random_seed = randi()

	# Use random_seed for deterministic generation
	seed(random_seed)

	# Generate personality based on archetype with variation
	var personality = _generate_personality_by_archetype(random_archetype)

	set_personality(personality)
	personality_archetype = random_archetype
	print("[PlayerData] Generated random personality: ", random_archetype)


func _generate_personality_by_archetype(archetype: String) -> Dictionary:
	"""Generate personality traits based on archetype with random variation"""
	var base_traits = {}

	match archetype:
		"Leader":
			base_traits = {
				"adaptability": 70,
				"ambition": 80,
				"determination": 85,
				"discipline": 75,
				"loyalty": 80,
				"pressure": 85,
				"professionalism": 80,
				"temperament": 70
			}
		"Genius":
			base_traits = {
				"adaptability": 85,
				"ambition": 75,
				"determination": 70,
				"discipline": 60,
				"loyalty": 65,
				"pressure": 70,
				"professionalism": 70,
				"temperament": 65
			}
		"Workhorse":
			base_traits = {
				"adaptability": 65,
				"ambition": 70,
				"determination": 85,
				"discipline": 90,
				"loyalty": 85,
				"pressure": 75,
				"professionalism": 85,
				"temperament": 80
			}
		"Rebel":
			base_traits = {
				"adaptability": 75,
				"ambition": 85,
				"determination": 80,
				"discipline": 50,
				"loyalty": 55,
				"pressure": 65,
				"professionalism": 60,
				"temperament": 50
			}
		"Steady":
			base_traits = {
				"adaptability": 60,
				"ambition": 60,
				"determination": 65,
				"discipline": 70,
				"loyalty": 75,
				"pressure": 65,
				"professionalism": 70,
				"temperament": 75
			}
		_:
			# Default to Steady if unknown archetype
			base_traits = {
				"adaptability": 50,
				"ambition": 50,
				"determination": 50,
				"discipline": 50,
				"loyalty": 50,
				"pressure": 50,
				"professionalism": 50,
				"temperament": 50
			}

	# Add random variation (±10) to each trait
	var varied_traits = {}
	for trait_name in base_traits:
		var base_value = base_traits[trait_name]
		var variation = randi_range(-10, 10)
		varied_traits[trait_name] = clamp(base_value + variation, 0, 100)

	return varied_traits


func get_personality_dict() -> Dictionary:
	"""성격 데이터를 Dictionary로 반환"""
	return {
		"adaptability": personality_adaptability,
		"ambition": personality_ambition,
		"determination": personality_determination,
		"discipline": personality_discipline,
		"loyalty": personality_loyalty,
		"pressure": personality_pressure,
		"professionalism": personality_professionalism,
		"temperament": personality_temperament
	}


func set_personality(personality: Dictionary) -> void:
	"""Dictionary로부터 성격 데이터 설정"""
	if personality.has("adaptability"):
		personality_adaptability = clamp(personality.adaptability, 0, 100)
	if personality.has("ambition"):
		personality_ambition = clamp(personality.ambition, 0, 100)
	if personality.has("determination"):
		personality_determination = clamp(personality.determination, 0, 100)
	if personality.has("discipline"):
		personality_discipline = clamp(personality.discipline, 0, 100)
	if personality.has("loyalty"):
		personality_loyalty = clamp(personality.loyalty, 0, 100)
	if personality.has("pressure"):
		personality_pressure = clamp(personality.pressure, 0, 100)
	if personality.has("professionalism"):
		personality_professionalism = clamp(personality.professionalism, 0, 100)
	if personality.has("temperament"):
		personality_temperament = clamp(personality.temperament, 0, 100)


func get_training_efficiency_multiplier() -> float:
	"""
	훈련 효율성 배수 계산 (0.7 ~ 1.3 범위)
	Formula: 0.7 + (base_efficiency × 0.6)
	base_efficiency = (discipline × 0.4 + professionalism × 0.3 + determination × 0.2 + ambition × 0.1) / 100
	"""
	return OpenFootballAPI.calculate_training_efficiency(get_personality_dict())


## ========== Phase 3: Special Abilities Functions ==========


func add_ability(ability: Dictionary) -> void:
	"""특수능력 추가 (중복 체크)"""
	# 이미 있는 능력인지 확인
	for existing_ability in special_abilities:
		if existing_ability.ability_type == ability.ability_type:
			# 티어만 업데이트
			existing_ability.tier = ability.tier
			print("[PlayerData] Updated ability: ", ability.ability_type, " → ", ability.tier)
			return

	# 새로운 능력 추가
	special_abilities.append(ability.duplicate())
	print("[PlayerData] Added ability: ", ability.ability_type, " (", ability.tier, ")")


func remove_ability(ability_type: String) -> void:
	"""특수능력 제거 (ability_type으로)"""
	for i in range(special_abilities.size() - 1, -1, -1):
		if special_abilities[i].ability_type == ability_type:
			special_abilities.remove_at(i)
			print("[PlayerData] Removed ability: ", ability_type)
			return


func get_abilities() -> Array:
	"""모든 특수능력 반환"""
	return special_abilities.duplicate()


func calculate_combined_ability_effects() -> Dictionary:
	"""
	모든 특수능력의 종합 효과 계산 (Rust engine 호출)
	@return: Dictionary with success, effects (36 OpenFootball skills)
	"""
	if special_abilities.is_empty():
		return {"success": true, "effects": {}}

	return OpenFootballAPI.calculate_ability_effects(special_abilities)


func process_ability_combinations() -> Dictionary:
	"""
	자동 능력 조합 실행 (Bronze 2개 → Silver 등)
	@return: Dictionary with success, combinations array
	"""
	# PlayerContext 생성
	var context = {
		"current_ability": get_overall_rating(),
		"potential_ability": get_overall_rating() + 20,  # 임시: 현재+20
		"games_played": 0,  # TODO: 게임 플레이 횟수 추적 필요
		"training_consistency": 0.8,  # TODO: 훈련 일관성 추적 필요
		"is_team_captain": false,  # TODO: 주장 여부
		"is_national_team_player": false,  # TODO: 국대 여부
		"major_titles": 0,  # TODO: 우승 횟수 추적 필요
		"perfect_games": 0,  # TODO: 완벽한 경기 횟수
		"perfect_season": false,  # TODO: 완벽한 시즌 여부
		"all_relationships_maxed": false  # TODO: 관계도 최대치 여부
	}

	# 능력 컬렉션 생성
	var collection = {"abilities": special_abilities, "combination_history": []}

	# Rust engine 호출
	var result_variant = OpenFootballAPI.process_ability_combinations(collection, context)
	var result: Dictionary = {}
	if result_variant is Dictionary:
		result = result_variant
	else:
		return {
			"success": false,
			"total_combinations": 0,
			"combinations": [],
			"error": "process_ability_combinations returned invalid payload"
		}

	# 조합이 발생한 경우 special_abilities 업데이트
	if result.get("success", false) and result.get("total_combinations", 0) > 0:
		# TODO: 실제로는 Rust에서 업데이트된 컬렉션을 받아와야 함
		# 지금은 간단히 메시지만 출력
		var combinations: Array = result.get("combinations", [])
		for combination in combinations:
			print("[PlayerData] Ability combination: ", combination.get("message", ""))

	return result


func _assemble_core_attributes_from_stats() -> Dictionary:
	var attrs: Dictionary = {}

	attrs["pace"] = physical_stats.get("pace", 15)
	attrs["acceleration"] = physical_stats.get("acceleration", 15)
	attrs["stamina"] = physical_stats.get("stamina", 15)
	attrs["strength"] = physical_stats.get("strength", 15)
	attrs["agility"] = physical_stats.get("agility", 15)
	attrs["balance"] = physical_stats.get("balance", 15)
	attrs["jumping"] = physical_stats.get("jumping", 15)
	attrs["natural_fitness"] = physical_stats.get("natural_fitness", 15)

	attrs["corners"] = technical_stats.get("corners", 15)
	attrs["crossing"] = technical_stats.get("crossing", 15)
	attrs["dribbling"] = technical_stats.get("dribbling", 15)
	attrs["finishing"] = technical_stats.get("finishing", 15)
	attrs["first_touch"] = technical_stats.get("first_touch", 15)
	attrs["free_kick_taking"] = technical_stats.get("free_kicks", 15)
	attrs["heading"] = technical_stats.get("heading", 15)
	attrs["long_shots"] = technical_stats.get("long_shots", 15)
	attrs["long_throws"] = technical_stats.get("long_throws", 15)
	attrs["marking"] = technical_stats.get("marking", 15)
	attrs["passing"] = technical_stats.get("passing", 15)
	attrs["penalty_taking"] = technical_stats.get("penalty_kicks", 15)
	attrs["tackling"] = technical_stats.get("tackling", 15)
	attrs["technique"] = technical_stats.get("technique", 15)

	attrs["aggression"] = mental_stats.get("aggression", 15)
	attrs["anticipation"] = mental_stats.get("anticipation", 15)
	attrs["bravery"] = mental_stats.get("bravery", 15)
	attrs["composure"] = mental_stats.get("composure", 15)
	attrs["concentration"] = mental_stats.get("concentration", 15)
	attrs["decisions"] = mental_stats.get("decisions", 15)
	attrs["determination"] = mental_stats.get("determination", 15)
	attrs["flair"] = mental_stats.get("flair", 15)
	attrs["leadership"] = mental_stats.get("leadership", 15)
	attrs["off_the_ball"] = mental_stats.get("off_the_ball", 15)
	attrs["positioning"] = mental_stats.get("positioning", 15)
	attrs["teamwork"] = mental_stats.get("teamwork", 15)
	attrs["vision"] = mental_stats.get("vision", 15)
	attrs["work_rate"] = mental_stats.get("work_rate", 15)

	attrs["handling"] = goalkeeper_stats.get("handling", 15)
	attrs["aerial_reach"] = goalkeeper_stats.get("aerial_reach", 15)
	attrs["command_of_area"] = goalkeeper_stats.get("command_of_area", 15)
	attrs["communication"] = goalkeeper_stats.get("communication", 15)
	attrs["eccentricity"] = goalkeeper_stats.get("eccentricity", 15)
	attrs["kicking"] = goalkeeper_stats.get("kicking", 15)

	return attrs


# ========== Phase 2: Exclusive Trait Functions ==========


func has_exclusive_trait(trait_name: String) -> bool:
	"""
	특정 Exclusive 특성 보유 여부 확인
	@param trait_name: 특성 이름 ("rival_awakening", "team_chemistry", 등)
	@return: 보유 여부
	"""
	return exclusive_trait == trait_name


func get_exclusive_trait_multiplier(stat_category: String) -> float:
	"""
	Exclusive 특성에 따른 능력치 배율 반환
	@param stat_category: "training", "deck", "tactics", "morale", "defensive", "clean_sheet"
	@return: 배율 (1.0 = 효과 없음)
	"""
	match exclusive_trait:
		"rival_awakening":
			# 라이벌 각성: 훈련 효율 +30%
			return 1.30 if stat_category == "training" else 1.0

		"team_chemistry":
			# 팀 케미: 덱 효과 +15%
			return 1.15 if stat_category == "deck" else 1.0

		"tactical_understanding":
			# 전술 이해도: 전술 효과 +25%
			return 1.25 if stat_category == "tactics" else 1.0

		"leadership":
			# 리더십: 팀 사기 +25%
			return 1.25 if stat_category == "morale" else 1.0

		"iron_defense":
			# 철벽 수비: 수비 능력 +20%, 무실점 보상 +50%
			if stat_category == "defensive":
				return 1.20
			elif stat_category == "clean_sheet":
				return 1.50
			else:
				return 1.0

		_:
			return 1.0


func set_exclusive_trait(trait_name: String, level: int = 1) -> void:
	"""
	Exclusive 특성 설정 (기존 특성 덮어쓰기)
	@param trait_name: 특성 이름
	@param level: 특성 레벨 (1-3)
	"""
	if exclusive_trait != "" and exclusive_trait != trait_name:
		push_warning("[PlayerData] Replacing existing trait '%s' with '%s'" % [exclusive_trait, trait_name])

	exclusive_trait = trait_name
	exclusive_trait_level = clampi(level, 1, 3)
	print("[PlayerData] Exclusive trait set: %s Lv.%d" % [trait_name, exclusive_trait_level])


func upgrade_exclusive_trait() -> bool:
	"""
	현재 Exclusive 특성 레벨업
	@return: 성공 여부 (최대 레벨 3)
	"""
	if exclusive_trait == "":
		push_warning("[PlayerData] No exclusive trait to upgrade")
		return false

	if exclusive_trait_level >= 3:
		push_warning("[PlayerData] Exclusive trait already at max level (3)")
		return false

	exclusive_trait_level += 1
	print("[PlayerData] Exclusive trait upgraded: %s Lv.%d" % [exclusive_trait, exclusive_trait_level])
	return true


func get_exclusive_trait_name() -> String:
	"""
	현재 Exclusive 특성의 한글 이름 반환
	@return: 한글 이름 (없으면 빈 문자열)
	"""
	var trait_names = {
		"rival_awakening": "라이벌 각성",
		"team_chemistry": "팀 케미",
		"tactical_understanding": "전술 이해도",
		"leadership": "리더십",
		"iron_defense": "철벽 수비"
	}
	return trait_names.get(exclusive_trait, "")


func get_exclusive_trait_description() -> String:
	"""
	현재 Exclusive 특성의 효과 설명 반환
	@return: 설명 문자열
	"""
	match exclusive_trait:
		"rival_awakening":
			return "훈련 효율 +%d%%, CA 성장 상한 +%d" % [30 * exclusive_trait_level / 1, 10]
		"team_chemistry":
			return "팀 전체 호감도 +%d%%, 덱 효과 +%d%%" % [20, 15 * exclusive_trait_level / 1]
		"tactical_understanding":
			return "Mental +%d%%, Vision +%d%%, 전술 효과 +%d%%" % [20, 15, 25 * exclusive_trait_level / 1]
		"leadership":
			return "팀 사기 +%d%%, 주장 권한 언락, 전체 호감도 +%d%%" % [25 * exclusive_trait_level / 1, 15]
		"iron_defense":
			return "Defensive +%d%%, 무실점 경기 보상 +%d%%, 실점률 -%d%%" % [20, 50 * exclusive_trait_level / 1, 30]
		_:
			return "없음"


# ========== Phase 5.5: Hero Growth System ==========


func apply_match_growth(growth: Dictionary) -> Dictionary:
	"""
	경기 종료 후 Hero Time XP에서 계산된 스탯 성장 적용
	Rust engine의 HeroMatchGrowth 구조체에서 변환된 Dictionary 형식:
	{
		"stat_gains": { "passing": 1, "dribbling": 2 },
		"xp_overflow": { "passing": 5.3, "vision": 2.1 },
		"total_xp_earned": 45.5,
		"highlight_gains": [["passing", 1], ["dribbling", 2]]
	}

	@param growth: HeroMatchGrowth Dictionary
	@return: 실제 적용된 성장 결과 { applied: {}, capped: {} }
	"""
	var result: Dictionary = {"applied": {}, "capped": {}}

	if growth.is_empty():
		return result

	# 1. 스탯 증가 적용
	var stat_gains: Dictionary = growth.get("stat_gains", {})
	for stat_name in stat_gains:
		var gain: int = int(stat_gains[stat_name])
		if gain <= 0:
			continue

		# 현재 스탯 값 찾기
		var current_value: int = _get_stat_by_name(stat_name)
		if current_value < 0:
			push_warning("[PlayerData] Unknown stat for growth: %s" % stat_name)
			continue

		# 새 값 계산 (최대 99)
		var new_value: int = mini(current_value + gain, 99)
		var actual_gain: int = new_value - current_value

		if actual_gain > 0:
			_set_stat_by_name(stat_name, new_value)
			result.applied[stat_name] = actual_gain
			print("[PlayerData] 📈 %s +%d (%d → %d)" % [stat_name, actual_gain, current_value, new_value])

		if actual_gain < gain:
			result.capped[stat_name] = gain - actual_gain

	# 2. XP 오버플로우 저장 (다음 경기로 이월)
	var new_overflow: Dictionary = growth.get("xp_overflow", {})
	for stat_name in new_overflow:
		var overflow_xp: float = float(new_overflow[stat_name])
		if overflow_xp > 0.0:
			# 기존 오버플로우에 추가
			xp_overflow[stat_name] = xp_overflow.get(stat_name, 0.0) + overflow_xp

	# 3. 시그널 발생 (UI 업데이트용)
	if not result.applied.is_empty():
		stats_changed.emit(result.applied)

		# CA 재계산
		_recalculate_openfootball_ca()

		print("[PlayerData] ⭐ Match growth applied: %d stats improved" % result.applied.size())
		print("  Total XP earned: %.1f" % growth.get("total_xp_earned", 0.0))

	return result


func _connect_to_match_manager():
	"""Connect to MatchManager.match_ended signal for Hero Time growth"""
	if has_node("/root/MatchManager"):
		var match_mgr = get_node("/root/MatchManager")
		if not match_mgr.match_ended.is_connected(_on_match_ended):
			match_mgr.match_ended.connect(_on_match_ended)
			print("[PlayerData] ✅ Connected to MatchManager.match_ended")
	else:
		print("[PlayerData] ⚠️ MatchManager not found (will be available when match starts)")


func _on_match_ended(result: Dictionary):
	"""Handle match end and apply Hero Time growth"""
	print("[PlayerData] Match ended - checking for growth data")

	# Check if result contains growth data
	if not result.has("hero_growth"):
		print("[PlayerData] ⚠️ No hero_growth data in match result")
		return

	var growth = result.hero_growth

	# Check if there's any actual growth
	if growth.get("stat_gains", {}).is_empty():
		print("[PlayerData] No stat gains this match (Hero Time: %d actions)" % growth.get("hero_time_actions", 0))
		return

	# Apply growth (CA is already recalculated inside apply_match_growth)
	var applied = apply_match_growth(growth)

	print("[PlayerData] ✅ Growth applied: %d stats increased" % applied.applied.size())
	if applied.capped.size() > 0:
		print("[PlayerData] ⚠️ %d stats capped by PA" % applied.capped.size())


func get_xp_overflow() -> Dictionary:
	"""
	다음 경기로 이월될 XP 오버플로우 반환
	@return: { "stat_name": xp_amount, ... }
	"""
	return xp_overflow.duplicate()


func clear_xp_overflow() -> void:
	"""XP 오버플로우 초기화 (새 시즌 시작 등)"""
	xp_overflow.clear()
	print("[PlayerData] XP overflow cleared")


func _get_stat_by_name(stat_name: String) -> int:
	"""스탯 이름으로 현재 값 조회 (모든 카테고리 검색)"""
	if technical_stats.has(stat_name):
		return int(technical_stats[stat_name])
	if mental_stats.has(stat_name):
		return int(mental_stats[stat_name])
	if physical_stats.has(stat_name):
		return int(physical_stats[stat_name])
	if goalkeeper_stats.has(stat_name):
		return int(goalkeeper_stats[stat_name])
	return -1  # Not found


func _set_stat_by_name(stat_name: String, value: int) -> bool:
	"""스탯 이름으로 값 설정 (모든 카테고리 검색)"""
	value = clampi(value, 0, 99)

	if technical_stats.has(stat_name):
		technical_stats[stat_name] = value
		return true
	if mental_stats.has(stat_name):
		mental_stats[stat_name] = value
		return true
	if physical_stats.has(stat_name):
		physical_stats[stat_name] = value
		return true
	if goalkeeper_stats.has(stat_name):
		goalkeeper_stats[stat_name] = value
		return true
	return false  # Not found
