extends Node
class_name AttributeTemplates
# CA 80 기준 포지션별 기본 능력치 템플릿

# 명세서 CHARACTER_CREATION_PROCESS_V3.md에 따른 정확한 능력치 분배
# CA 80 = total_units 2600 필요 (OpenFootball 공식 적용)
# Technical(14) + Mental(14) + Physical(8×2) + GK(6) = 42개 능력치


static func get_position_template(position_category: String) -> Dictionary:
	match position_category:
		"공격수":
			return _get_forward_template()
		"미드필더":
			return _get_midfielder_template()
		"수비수":
			return _get_defender_template()
		_:
			push_error("Unknown position category: " + position_category)
			return _get_midfielder_template()  # 기본값


static func _get_forward_template() -> Dictionary:
	# 공격수: 골 결정력과 개인기 중심
	# CA 80 달성을 위한 정확한 분배
	return {
		"technical":
		{
			"corners": 35,
			"crossing": 42,
			"dribbling": 52,  # 핵심 능력
			"finishing": 58,  # 핵심 능력
			"first_touch": 50,
			"free_kicks": 38,
			"heading": 48,
			"long_shots": 45,
			"long_throws": 30,
			"marking": 30,
			"passing": 45,
			"penalty_kicks": 45,
			"tackling": 25,
			"technique": 50
		},
		"mental":
		{
			"aggression": 45,
			"anticipation": 48,
			"bravery": 45,
			"composure": 52,  # 핵심 능력
			"concentration": 45,
			"decisions": 48,
			"determination": 50,
			"flair": 50,
			"leadership": 40,
			"off_the_ball": 55,  # 핵심 능력
			"positioning": 50,
			"teamwork": 45,
			"vision": 45,
			"work_rate": 45
		},
		"physical":
		{
			"acceleration": 52,  # 핵심 능력
			"agility": 50,
			"balance": 50,
			"jumping": 45,
			"natural_fitness": 48,
			"pace": 52,  # 핵심 능력
			"stamina": 45,
			"strength": 45
		},
		"goalkeeper":
		{
			"aerial_reach": 25,
			"command_of_area": 25,
			"communication": 25,
			"eccentricity": 25,
			"handling": 25,
			"kicking": 25
		}
	}


static func _get_midfielder_template() -> Dictionary:
	# 미드필더: 패스와 시야, 활동량 중심
	# CA 80 달성을 위한 정확한 분배
	return {
		"technical":
		{
			"corners": 45,
			"crossing": 48,
			"dribbling": 45,
			"finishing": 35,
			"first_touch": 50,
			"free_kicks": 42,
			"heading": 40,
			"long_shots": 42,
			"long_throws": 35,
			"marking": 45,
			"passing": 55,  # 핵심 능력
			"penalty_kicks": 38,
			"tackling": 45,
			"technique": 50
		},
		"mental":
		{
			"aggression": 40,
			"anticipation": 48,
			"bravery": 45,
			"composure": 48,
			"concentration": 52,
			"decisions": 52,  # 핵심 능력
			"determination": 50,
			"flair": 42,
			"leadership": 48,
			"off_the_ball": 45,
			"positioning": 50,
			"teamwork": 55,  # 핵심 능력
			"vision": 55,  # 핵심 능력
			"work_rate": 55  # 핵심 능력
		},
		"physical":
		{
			"acceleration": 45,
			"agility": 48,
			"balance": 48,
			"jumping": 40,
			"natural_fitness": 52,
			"pace": 45,
			"stamina": 55,  # 핵심 능력
			"strength": 45
		},
		"goalkeeper":
		{
			"aerial_reach": 25,
			"command_of_area": 25,
			"communication": 25,
			"eccentricity": 25,
			"handling": 25,
			"kicking": 25
		}
	}


static func _get_defender_template() -> Dictionary:
	# 수비수: 수비 위치선정과 피지컬 중심
	# CA 80 달성을 위한 정확한 분배
	return {
		"technical":
		{
			"corners": 28,
			"crossing": 35,
			"dribbling": 35,
			"finishing": 22,
			"first_touch": 40,
			"free_kicks": 30,
			"heading": 55,  # 핵심 능력
			"long_shots": 25,
			"long_throws": 42,
			"marking": 55,  # 핵심 능력
			"passing": 48,
			"penalty_kicks": 30,
			"tackling": 55,  # 핵심 능력
			"technique": 38
		},
		"mental":
		{
			"aggression": 50,
			"anticipation": 55,  # 핵심 능력
			"bravery": 55,  # 핵심 능력
			"composure": 45,
			"concentration": 55,  # 핵심 능력
			"decisions": 50,
			"determination": 52,
			"flair": 25,
			"leadership": 50,
			"off_the_ball": 35,
			"positioning": 55,  # 핵심 능력
			"teamwork": 50,
			"vision": 40,
			"work_rate": 50
		},
		"physical":
		{
			"acceleration": 45,
			"agility": 45,
			"balance": 48,
			"jumping": 55,  # 핵심 능력
			"natural_fitness": 50,
			"pace": 45,
			"stamina": 50,
			"strength": 55  # 핵심 능력
		},
		"goalkeeper":
		{
			"aerial_reach": 25,
			"command_of_area": 25,
			"communication": 25,
			"eccentricity": 25,
			"handling": 25,
			"kicking": 25
		}
	}


# Step 3: 카테고리 보너스 적용 (모든 능력치 +3)
static func apply_category_bonus(base_attributes: Dictionary, category: String) -> Dictionary:
	var result = base_attributes.duplicate(true)

	var category_map = {"Technical": "technical", "Mental": "mental", "Physical": "physical"}

	if category in category_map:
		var attr_key = category_map[category]
		if attr_key in result:
			for stat in result[attr_key]:
				result[attr_key][stat] += 3

	return result


# Step 4: 특장점 보너스 적용 (선택한 3개 각각 +3)
static func apply_specialty_bonuses(base_attributes: Dictionary, specialties: Array) -> Dictionary:
	var result = base_attributes.duplicate(true)

	# 특장점이 어느 카테고리에 속하는지 매핑
	var specialty_mapping = {
		# Technical
		"Dribbling": {"category": "technical", "stat": "dribbling"},
		"Passing": {"category": "technical", "stat": "passing"},
		"Shooting": {"category": "technical", "stat": "finishing"},
		"Crossing": {"category": "technical", "stat": "crossing"},
		"First Touch": {"category": "technical", "stat": "first_touch"},
		"Ball Control": {"category": "technical", "stat": "technique"},
		"Technique": {"category": "technical", "stat": "technique"},
		"Heading": {"category": "technical", "stat": "heading"},
		"Finishing": {"category": "technical", "stat": "finishing"},
		"Long Shots": {"category": "technical", "stat": "long_shots"},
		"Free Kicks": {"category": "technical", "stat": "free_kicks"},
		"Penalties": {"category": "technical", "stat": "penalty_kicks"},
		"Corners": {"category": "technical", "stat": "corners"},
		"Throw-ins": {"category": "technical", "stat": "long_throws"},
		# Mental
		"Decisions": {"category": "mental", "stat": "decisions"},
		"Concentration": {"category": "mental", "stat": "concentration"},
		"Leadership": {"category": "mental", "stat": "leadership"},
		"Vision": {"category": "mental", "stat": "vision"},
		"Teamwork": {"category": "mental", "stat": "teamwork"},
		"Work Rate": {"category": "mental", "stat": "work_rate"},
		"Positioning": {"category": "mental", "stat": "positioning"},
		"Anticipation": {"category": "mental", "stat": "anticipation"},
		"Composure": {"category": "mental", "stat": "composure"},
		"Bravery": {"category": "mental", "stat": "bravery"},
		"Determination": {"category": "mental", "stat": "determination"},
		"Flair": {"category": "mental", "stat": "flair"},
		"Off the Ball": {"category": "mental", "stat": "off_the_ball"},
		"Aggression": {"category": "mental", "stat": "aggression"},
		# Physical
		"Speed": {"category": "physical", "stat": "pace"},
		"Stamina": {"category": "physical", "stat": "stamina"},
		"Strength": {"category": "physical", "stat": "strength"},
		"Agility": {"category": "physical", "stat": "agility"},
		"Balance": {"category": "physical", "stat": "balance"},
		"Jumping": {"category": "physical", "stat": "jumping"},
		"Natural Fitness": {"category": "physical", "stat": "natural_fitness"},
		"Acceleration": {"category": "physical", "stat": "acceleration"}
	}

	# 각 특장점에 대해 +3 보너스 적용
	for specialty in specialties:
		if specialty in specialty_mapping:
			var mapping = specialty_mapping[specialty]
			var category = mapping.category
			var stat = mapping.stat

			if category in result and stat in result[category]:
				result[category][stat] += 3
				print("Applied +3 bonus to %s/%s (now %d)" % [category, stat, result[category][stat]])

	return result


# CA 계산 (OpenFootball 공식)
static func calculate_ca(attributes: Dictionary) -> int:
	var technical_sum = 0
	var mental_sum = 0
	var physical_sum = 0
	var gk_sum = 0

	# Technical 합계
	if "technical" in attributes:
		for stat in attributes.technical.values():
			technical_sum += stat

	# Mental 합계
	if "mental" in attributes:
		for stat in attributes.mental.values():
			mental_sum += stat

	# Physical 합계 (2배 가중치)
	if "physical" in attributes:
		for stat in attributes.physical.values():
			physical_sum += stat

	# GK 합계
	if "goalkeeper" in attributes:
		for stat in attributes.goalkeeper.values():
			gk_sum += stat

	# OpenFootball 공식
	var total_units = technical_sum + mental_sum + (physical_sum * 2) + gk_sum
	var base_ca = 0

	if total_units >= 1000:
		base_ca = (total_units - 1000) // 20
	else:
		base_ca = total_units // 40

	# 포지션 modifier는 일단 1.0으로 (추후 구현)
	return int(base_ca)


# 전체 플로우 테스트용 함수
static func get_final_attributes(position_category: String, category_choice: String, specialties: Array) -> Dictionary:
	# Step 2: 기본 템플릿
	var base = get_position_template(position_category)

	# Step 3: 카테고리 보너스
	var with_category = apply_category_bonus(base, category_choice)

	# Step 4: 특장점 보너스
	var final = apply_specialty_bonuses(with_category, specialties)

	return final
