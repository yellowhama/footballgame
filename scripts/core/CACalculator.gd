extends Node

# FM 스타일 CA/PA 계산 시스템
# 핵심 공식: 총 유닛 = 150 + (CA × 5)

signal pa_changed(new_pa: int)

const BASE_UNITS = 150
const UNITS_PER_CA = 5
const MAX_CA = 200
const MAX_PA = 200

# 능력치별 기본 비용 (FM 스타일)
const SKILL_COSTS = {
	# Physical - 2유닛
	"pace": 2.0,
	"acceleration": 2.0,
	"agility": 2.0,
	"balance": 1.5,
	"jumping": 1.5,
	"natural_fitness": 0.0,  # CA에 영향 없음
	"stamina": 2.0,
	"strength": 2.0,
	# Technical - 1유닛
	"corners": 1.0,
	"crossing": 1.0,
	"dribbling": 1.0,
	"finishing": 1.0,
	"first_touch": 1.0,
	"free_kicks": 1.0,
	"heading": 1.0,
	"long_shots": 1.0,
	"long_throws": 0.5,
	"marking": 1.0,
	"passing": 1.0,
	"penalty_taking": 0.8,
	"tackling": 1.0,
	"technique": 1.0,
	# Mental - 1유닛
	"aggression": 0.0,  # CA에 영향 없음
	"anticipation": 1.0,
	"bravery": 0.8,
	"composure": 1.0,
	"concentration": 1.0,
	"decisions": 1.0,
	"determination": 0.0,  # CA에 영향 없음
	"flair": 0.0,  # CA에 영향 없음
	"leadership": 0.5,
	"off_the_ball": 1.0,
	"positioning": 1.0,
	"teamwork": 0.8,
	"vision": 1.0,
	"work_rate": 0.8,
	# Goalkeeper - 1유닛 (GK만 적용)
	"aerial_reach": 1.0,
	"command_of_area": 1.0,
	"communication": 0.5,
	"eccentricity": 0.0,  # CA에 영향 없음
	"handling": 1.2,
	"kicking": 0.8,
	"one_on_ones": 1.0,
	"reflexes": 1.5,
	"rushing_out": 0.0,  # 성향이라 CA 영향 없음
	"throwing": 0.5
}

# 포지션별 가중치 (중요 능력치는 더 많은 CA 소모)
const POSITION_WEIGHTS = {
	"ST":
	{
		"finishing": 2.5,
		"heading": 1.8,
		"pace": 2.0,
		"acceleration": 2.0,
		"composure": 1.5,
		"off_the_ball": 1.8,
		"anticipation": 1.5,
		"decisions": 1.3
	},
	"MF":
	{
		"passing": 2.2,
		"vision": 2.0,
		"technique": 1.8,
		"decisions": 1.8,
		"stamina": 1.8,
		"positioning": 1.5,
		"first_touch": 1.5,
		"concentration": 1.3
	},
	"DF":
	{
		"marking": 2.5,
		"tackling": 2.2,
		"positioning": 2.0,
		"heading": 1.8,
		"strength": 1.8,
		"anticipation": 1.8,
		"concentration": 1.5,
		"decisions": 1.5
	},
	"GK":
	{
		"reflexes": 3.0,
		"handling": 2.5,
		"positioning": 2.0,
		"command_of_area": 2.0,
		"aerial_reach": 1.8,
		"concentration": 1.8,
		"decisions": 1.5,
		"communication": 1.3
	}
}


func calculate_ca_from_skills(skills: Dictionary, position: String = "MF") -> int:
	"""42개 능력치로부터 CA 계산"""
	var total_units = 0.0
	var position_weights = POSITION_WEIGHTS.get(position, {})

	for skill_name in skills:
		if not SKILL_COSTS.has(skill_name):
			continue

		var skill_value = skills[skill_name]
		var base_cost = SKILL_COSTS[skill_name]

		# 포지션별 가중치 적용
		var position_weight = position_weights.get(skill_name, 1.0)

		# 능력치 1당 유닛 계산 (0-20 스케일 가정)
		var units_for_skill = (skill_value / 20.0) * base_cost * position_weight
		total_units += units_for_skill

	# 유닛을 CA로 변환
	var ca = int((total_units - BASE_UNITS) / UNITS_PER_CA)
	return clamp(ca, 1, MAX_CA)


func get_available_units(ca: int) -> int:
	"""CA에서 사용 가능한 총 유닛 계산"""
	return BASE_UNITS + (ca * UNITS_PER_CA)


func calculate_skill_distribution(ca: int, position: String, growth_focus: String = "balanced") -> Dictionary:
	"""CA와 포지션에 따른 이상적인 능력치 분배 계산"""
	var available_units = get_available_units(ca)
	var skill_distribution = {}

	# 포지션별 핵심 능력치 우선 배분
	var position_priorities = _get_position_priorities(position)
	var growth_modifier = _get_growth_modifier(growth_focus)

	# 유닛 배분 (우선순위에 따라)
	var remaining_units = available_units

	for priority_group in position_priorities:
		var skills_in_group = priority_group["skills"]
		var allocation_ratio = priority_group["ratio"]
		var units_for_group = available_units * allocation_ratio

		for skill in skills_in_group:
			if remaining_units <= 0:
				break

			var base_cost = SKILL_COSTS.get(skill, 1.0)
			if base_cost == 0:
				continue  # CA에 영향 없는 능력치 스킵

			# 성장 초점에 따른 수정
			var modifier = growth_modifier.get(skill, 1.0)

			# 능력치 값 계산 (20점 만점 기준)
			var skill_value = min(20, (units_for_group / len(skills_in_group)) * modifier * 20 / base_cost)
			skill_distribution[skill] = int(skill_value)

			remaining_units -= skill_value * base_cost / 20

	return skill_distribution


func _get_position_priorities(position: String) -> Array:
	"""포지션별 능력치 우선순위 반환"""
	match position:
		"ST":
			return [
				{"skills": ["finishing", "heading", "composure"], "ratio": 0.35},
				{"skills": ["pace", "acceleration", "off_the_ball"], "ratio": 0.30},
				{"skills": ["anticipation", "decisions", "technique"], "ratio": 0.20},
				{"skills": ["strength", "jumping", "balance"], "ratio": 0.15}
			]
		"MF":
			return [
				{"skills": ["passing", "vision", "technique"], "ratio": 0.35},
				{"skills": ["decisions", "positioning", "first_touch"], "ratio": 0.25},
				{"skills": ["stamina", "work_rate", "concentration"], "ratio": 0.20},
				{"skills": ["pace", "agility", "balance"], "ratio": 0.20}
			]
		"DF":
			return [
				{"skills": ["marking", "tackling", "positioning"], "ratio": 0.35},
				{"skills": ["heading", "strength", "jumping"], "ratio": 0.25},
				{"skills": ["anticipation", "concentration", "decisions"], "ratio": 0.20},
				{"skills": ["pace", "acceleration", "agility"], "ratio": 0.20}
			]
		"GK":
			return [
				{"skills": ["reflexes", "handling", "positioning"], "ratio": 0.40},
				{"skills": ["command_of_area", "aerial_reach", "communication"], "ratio": 0.30},
				{"skills": ["concentration", "decisions", "anticipation"], "ratio": 0.20},
				{"skills": ["kicking", "throwing", "first_touch"], "ratio": 0.10}
			]
		_:
			# 기본값 (밸런스형)
			return [
				{"skills": ["technique", "passing", "first_touch"], "ratio": 0.25},
				{"skills": ["pace", "stamina", "strength"], "ratio": 0.25},
				{"skills": ["decisions", "positioning", "vision"], "ratio": 0.25},
				{"skills": ["marking", "tackling", "heading"], "ratio": 0.25}
			]


func _get_growth_modifier(focus: String) -> Dictionary:
	"""성장 초점에 따른 능력치 수정자"""
	match focus:
		"physical":
			return {"pace": 1.3, "acceleration": 1.3, "strength": 1.3, "stamina": 1.3, "jumping": 1.2, "balance": 1.2}
		"technical":
			return {
				"technique": 1.3,
				"dribbling": 1.3,
				"first_touch": 1.3,
				"passing": 1.2,
				"crossing": 1.2,
				"finishing": 1.2
			}
		"mental":
			return {
				"decisions": 1.3,
				"vision": 1.3,
				"anticipation": 1.3,
				"positioning": 1.2,
				"concentration": 1.2,
				"composure": 1.2
			}
		_:  # balanced
			return {}


func calculate_growth_potential(current_ca: int, pa: int, age: int, training_quality: float = 1.0) -> Dictionary:
	"""성장 잠재력 계산"""
	var remaining_potential = pa - current_ca

	# 나이에 따른 성장률
	var age_factor = 1.0
	if age < 18:
		age_factor = 1.5  # 청소년 보너스
	elif age < 21:
		age_factor = 1.2
	elif age < 24:
		age_factor = 1.0
	elif age < 27:
		age_factor = 0.8
	elif age < 30:
		age_factor = 0.5
	else:
		age_factor = 0.2  # 30세 이후 급격히 감소

	# 주간 성장 가능 CA
	var weekly_growth = min(2.0, remaining_potential * 0.02 * age_factor * training_quality)

	# 예상 피크 도달 시간 (주 단위)
	var weeks_to_peak = 0
	if weekly_growth > 0:
		weeks_to_peak = int(remaining_potential / weekly_growth)

	return {
		"remaining_potential": remaining_potential,
		"weekly_growth_rate": weekly_growth,
		"weeks_to_peak": weeks_to_peak,
		"age_factor": age_factor,
		"max_ca": pa
	}


func apply_training_to_ca(current_ca: int, training_type: String, training_intensity: float = 1.0) -> Dictionary:
	"""훈련에 따른 CA 증가 계산"""
	# 기본 성장률 (주당)
	var base_growth = {"team_training": 0.5, "individual_training": 0.3, "match_experience": 0.4, "rest": 0.0}

	var growth = base_growth.get(training_type, 0.0) * training_intensity
	var new_ca = min(MAX_CA, current_ca + growth)

	return {"old_ca": current_ca, "new_ca": new_ca, "growth": new_ca - current_ca, "training_type": training_type}


# 6각형 시스템과 연동
func calculate_hexagon_from_skills(skills: Dictionary) -> Dictionary:
	"""42개 능력치를 6각형 능력치로 변환"""
	return {
		"PACE":
		_weighted_average(
			skills,
			[
				{"skill": "pace", "weight": 0.4},
				{"skill": "acceleration", "weight": 0.3},
				{"skill": "agility", "weight": 0.2},
				{"skill": "balance", "weight": 0.1}
			]
		),
		"POWER":
		_weighted_average(
			skills,
			[
				{"skill": "strength", "weight": 0.4},
				{"skill": "stamina", "weight": 0.3},
				{"skill": "jumping", "weight": 0.2},
				{"skill": "natural_fitness", "weight": 0.1}
			]
		),
		"TECHNICAL":
		_weighted_average(
			skills,
			[
				{"skill": "technique", "weight": 0.3},
				{"skill": "dribbling", "weight": 0.3},
				{"skill": "first_touch", "weight": 0.2},
				{"skill": "passing", "weight": 0.2}
			]
		),
		"SHOOTING":
		_weighted_average(
			skills,
			[
				{"skill": "finishing", "weight": 0.4},
				{"skill": "long_shots", "weight": 0.3},
				{"skill": "composure", "weight": 0.2},
				{"skill": "penalty_taking", "weight": 0.1}
			]
		),
		"PASSING":
		_weighted_average(
			skills,
			[
				{"skill": "passing", "weight": 0.4},
				{"skill": "vision", "weight": 0.3},
				{"skill": "crossing", "weight": 0.2},
				{"skill": "decisions", "weight": 0.1}
			]
		),
		"DEFENDING":
		_weighted_average(
			skills,
			[
				{"skill": "marking", "weight": 0.3},
				{"skill": "tackling", "weight": 0.3},
				{"skill": "positioning", "weight": 0.2},
				{"skill": "anticipation", "weight": 0.2}
			]
		)
	}


func _weighted_average(skills: Dictionary, weights: Array) -> float:
	"""가중 평균 계산"""
	var total = 0.0
	var weight_sum = 0.0

	for item in weights:
		var skill_value = skills.get(item["skill"], 0.0)
		total += skill_value * item["weight"]
		weight_sum += item["weight"]

	if weight_sum > 0:
		return total / weight_sum
	return 0.0


# 테스트 함수
func test_ca_system():
	print("=== CA Calculator Test ===")

	# 테스트 1: 기본 CA 계산
	var test_skills = {}
	for skill in SKILL_COSTS:
		test_skills[skill] = 10  # 모든 능력치 10

	var ca = calculate_ca_from_skills(test_skills, "MF")
	print("All skills at 10: CA = ", ca)

	# 테스트 2: 사용 가능 유닛
	var units = get_available_units(100)
	print("CA 100 = ", units, " units")

	# 테스트 3: 6각형 변환
	var hexagon = calculate_hexagon_from_skills(test_skills)
	print("Hexagon stats: ", hexagon)

	# 테스트 4: 성장 잠재력
	var growth = calculate_growth_potential(60, 140, 16, 1.0)
	print("Growth potential (CA 60, PA 140, Age 16): ", growth)

	print("✅ CA system test completed")
