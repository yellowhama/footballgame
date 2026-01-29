extends Node

## PositionManager.gd
## 포지션별 적합도 계산 시스템
## 기반: OpenFootball Position Rating Analysis
## 작성일: 2025-10-24
## 버전: 1.0

# ============================================
# 포지션별 핵심 능력치 정의
# ============================================

const POSITION_KEY_ATTRIBUTES = {
	"ST": ["Finishing", "Off The Ball", "Positioning", "Pace"],
	"FC": ["Finishing", "Off The Ball", "Positioning", "Pace"],
	"AMC": ["Finishing", "Passing", "Vision", "Off The Ball"],
	"MC": ["Passing", "Vision", "Positioning", "Stamina"],
	"DM": ["Tackling", "Positioning", "Marking", "Anticipation"],
	"DC": ["Tackling", "Positioning", "Heading", "Marking"],
	"DL": ["Tackling", "Positioning", "Marking", "Pace"],
	"DR": ["Tackling", "Positioning", "Marking", "Pace"],
	"WL": ["Crossing", "Pace", "Dribbling", "Off The Ball"],
	"WR": ["Crossing", "Pace", "Dribbling", "Off The Ball"],
	"GK": ["Jumping", "Agility", "Positioning", "Concentration"]
}

## 포지션 그룹 매핑
const POSITION_GROUPS = {
	"FW": ["ST", "FC"], "MF": ["AMC", "MC", "DM", "WL", "WR"], "DF": ["DC", "DL", "DR"], "GK": ["GK"]
}

# ============================================
# Position Level 계산
# ============================================


func calculate_position_level(player_attributes: Dictionary, position: String) -> int:
	"""
	포지션 레벨 계산 (0-20 스케일)

	@param player_attributes: 선수 능력치 Dictionary
	@param position: 포지션 (예: "ST", "MC", "GK")
	@return: Position Level (0-20)
	"""

	var key_attrs = POSITION_KEY_ATTRIBUTES.get(position, [])

	if key_attrs.is_empty():
		print("[PositionManager] Unknown position: ", position)
		return 0

	var total = 0.0

	for attr_name in key_attrs:
		var value = _get_attribute_value(player_attributes, attr_name)
		total += value

	var avg = total / key_attrs.size()
	return int(avg)


func _get_attribute_value(attributes: Dictionary, attr_name: String) -> float:
	"""능력치 값 조회 (Technical/Mental/Physical에서 찾기)"""

	# Technical 능력치
	if attributes.has("technical"):
		if attributes["technical"].has(attr_name):
			return attributes["technical"][attr_name]

	# Mental 능력치
	if attributes.has("mental"):
		if attributes["mental"].has(attr_name):
			return attributes["mental"][attr_name]

	# Physical 능력치
	if attributes.has("physical"):
		if attributes["physical"].has(attr_name):
			return attributes["physical"][attr_name]

	# GK 능력치
	if attributes.has("goalkeeper"):
		if attributes["goalkeeper"].has(attr_name):
			return attributes["goalkeeper"][attr_name]

	# 찾지 못한 경우 10 (평균값)
	return 10.0


# ============================================
# Position Rating 계산
# ============================================


func calculate_position_rating(player: Dictionary, position: String, ca: int = -1, condition: int = -1) -> float:
	"""
	포지션 적합도 종합 평가 (OpenFootball 공식 기반)

	@param player: 선수 데이터 Dictionary
	@param position: 포지션
	@param ca: Current Ability (기본값: PlayerData.ca)
	@param condition: 컨디션 (기본값: PlayerData.condition)
	@return: Position Rating (보통 8-20 범위)
	"""

	var rating = 0.0

	# CA 기본값 처리
	if ca < 0:
		ca = player.get("ca", PlayerData.ca)

	# Condition 기본값 처리
	if condition < 0:
		condition = player.get("condition", PlayerData.condition)

	# 1. Position Level (40% weight)
	var pos_level = calculate_position_level(player, position)
	rating += pos_level * 0.4

	# 2. CA (20% weight)
	rating += (ca / 200.0) * 20.0 * 0.2

	# 3. Condition (25% weight)
	rating += (condition / 100.0) * 20.0 * 0.25

	# 4. Tactical Fit (10% weight) - 기본값 10
	var tactical_fit = calculate_tactical_fit(player, position)
	rating += tactical_fit * 0.1

	# 5. Reputation (5% weight)
	var reputation = player.get("reputation", 0)
	rating += (reputation / 100.0) * 0.05

	return rating


func calculate_tactical_fit(_player: Dictionary, _position: String) -> float:
	"""
	전술 적합도 계산 (기본 10점 + 보너스)

	@param _player: 선수 데이터 (TODO: implement)
	@param _position: 포지션 (TODO: implement)
	@return: 적합도 점수 (10-15 범위)
	"""

	var fit_score = 10.0

	# TODO: TacticalStyle 시스템 구현 후 추가
	# 현재는 기본 점수만 반환

	return fit_score


# ============================================
# 승격 평가
# ============================================


func evaluate_for_promotion(player: Dictionary, target_tier: String, position: String) -> Dictionary:
	"""
	승격 조건 평가

	@param player: 선수 데이터
	@param target_tier: "B-Team" 또는 "A-Team"
	@param position: 포지션
	@return: 평가 결과 Dictionary
	"""

	var result = {
		"eligible": false,
		"ca_check": false,
		"rating_check": false,
		"match_avg_check": false,
		"current_ca": 0,
		"required_ca": 0,
		"current_rating": 0.0,
		"required_rating": 0.0,
		"current_match_avg": 0.0,
		"required_match_avg": 0.0,
		"missing_requirements": []
	}

	var ca = player.get("ca", PlayerData.ca)
	var rating = calculate_position_rating(player, position, ca)
	var match_avg = _get_recent_match_average()

	result["current_ca"] = ca
	result["current_rating"] = rating
	result["current_match_avg"] = match_avg

	# 목표 티어별 기준 설정
	match target_tier:
		"B-Team":
			result["required_ca"] = 110
			result["required_rating"] = 12.0
			result["required_match_avg"] = 6.5

		"A-Team":
			result["required_ca"] = 140
			result["required_rating"] = 16.0
			result["required_match_avg"] = 7.5

		_:
			print("[PositionManager] Unknown tier: ", target_tier)
			return result

	# CA 체크
	result["ca_check"] = ca >= result["required_ca"]
	if not result["ca_check"]:
		result["missing_requirements"].append("CA 부족: " + str(ca) + "/" + str(result["required_ca"]))

	# Rating 체크
	result["rating_check"] = rating >= result["required_rating"]
	if not result["rating_check"]:
		result["missing_requirements"].append("Position Rating 부족: %.1f/%.1f" % [rating, result["required_rating"]])

	# Match Average 체크
	result["match_avg_check"] = match_avg >= result["required_match_avg"]
	if not result["match_avg_check"]:
		result["missing_requirements"].append("경기 평점 부족: %.1f/%.1f" % [match_avg, result["required_match_avg"]])

	# 전체 통과 여부
	result["eligible"] = (result["ca_check"] and result["rating_check"] and result["match_avg_check"])

	return result


func _get_recent_match_average() -> float:
	"""최근 5경기 평균 평점 조회"""
	# TODO: MatchManager에서 실제 경기 평점 데이터 가져오기
	# 현재는 임시값 7.0 반환
	return 7.0


# ============================================
# 포지션별 핵심 능력치 조회
# ============================================


func get_key_attributes(position: String) -> Array[String]:
	"""포지션별 핵심 능력치 목록 반환"""
	return POSITION_KEY_ATTRIBUTES.get(position, [])


func get_weakest_attributes(player: Dictionary, position: String, count: int = 3) -> Array[Dictionary]:
	"""
	포지션 핵심 능력치 중 가장 낮은 능력치 반환

	@param player: 선수 데이터
	@param position: 포지션
	@param count: 반환할 개수
	@return: [{name: String, value: float}] 배열
	"""

	var key_attrs = get_key_attributes(position)
	var attr_values = []

	for attr_name in key_attrs:
		var value = _get_attribute_value(player, attr_name)
		attr_values.append({"name": attr_name, "value": value})

	# 낮은 순으로 정렬
	attr_values.sort_custom(func(a, b): return a["value"] < b["value"])

	# 상위 count개 반환
	return attr_values.slice(0, count)


# ============================================
# 유틸리티
# ============================================


func get_position_group(position: String) -> String:
	"""포지션 그룹 반환 (FW/MF/DF/GK)"""
	for group in POSITION_GROUPS:
		if position in POSITION_GROUPS[group]:
			return group
	return "FW"


func is_valid_position(position: String) -> bool:
	"""유효한 포지션인지 확인"""
	return POSITION_KEY_ATTRIBUTES.has(position)


# ============================================
# 디버그
# ============================================


func debug_print_position_analysis(player: Dictionary, position: String) -> void:
	"""포지션 분석 결과 출력 (디버그용)"""
	print("=== Position Analysis ===")
	print("Position: ", position)

	var pos_level = calculate_position_level(player, position)
	print("Position Level: ", pos_level, "/20")

	var rating = calculate_position_rating(player, position)
	print("Position Rating: %.2f" % rating)

	print("\nKey Attributes:")
	var key_attrs = get_key_attributes(position)
	for attr_name in key_attrs:
		var value = _get_attribute_value(player, attr_name)
		print("  ", attr_name, ": ", value)

	print("\nWeakest Attributes:")
	var weakest = get_weakest_attributes(player, position, 3)
	for attr in weakest:
		print("  ", attr["name"], ": ", attr["value"])

	print("=========================")
