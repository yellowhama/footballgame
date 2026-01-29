extends Node

# Relationship System (Princess Maker 스타일)
# 선수간 관계, 팀워크, 사기, 케미스트리 관리

# class_name removed - this is an autoload singleton

signal relationship_changed(player_id: int, new_value: float)
signal team_morale_changed(new_value: float)
signal team_chemistry_changed(new_value: float)
signal relationship_event_triggered(event: Dictionary)

# 개별 선수와의 관계 (0.0-1.0)
var relationships: Dictionary = {}  # player_id -> relationship_value

# 팀 전체 상태
var team_morale: float = 0.5  # 팀 사기 (0.0-1.0)
var team_chemistry: float = 0.5  # 팀 케미스트리 (0.0-1.0)
var team_cohesion: float = 0.5  # 팀 화합도 (0.0-1.0)

# 감독 관계도 (-100 ~ 100)
var manager_relationship: float = 0.0

# 관계 이벤트 정의
const RELATIONSHIP_EVENTS = [
	{
		"id": "teammate_help",
		"probability": 0.1,
		"condition": "morale > 0.6",
		"type": "positive",
		"relationship_bonus": 0.05,
		"message": "🤝 동료가 도움을 주었습니다! (+5 관계도)"
	},
	{
		"id": "conflict",
		"probability": 0.05,
		"condition": "morale < 0.4",
		"type": "negative",
		"relationship_penalty": -0.03,
		"morale_penalty": -0.02,
		"message": "😤 동료와 갈등이 생겼습니다... (-3 관계도)"
	},
	{
		"id": "team_bonding",
		"probability": 0.08,
		"condition": "chemistry > 0.7",
		"type": "positive",
		"chemistry_bonus": 0.03,
		"morale_bonus": 0.02,
		"message": "🎉 팀 결속이 강해졌습니다! (+3 케미스트리)"
	},
	{
		"id": "leadership_moment",
		"probability": 0.06,
		"condition": "leadership > 80",
		"type": "positive",
		"morale_bonus": 0.04,
		"relationship_bonus": 0.02,
		"message": "👑 리더십을 발휘했습니다! (+4 사기)"
	},
	{
		"id": "team_dinner",
		"probability": 0.03,
		"condition": "week % 52 == 26",  # 시즌 중반
		"type": "special",
		"relationship_bonus": 0.08,
		"chemistry_bonus": 0.05,
		"message": "🍽️ 팀 저녁식사! 모든 관계도가 향상됩니다!"
	}
]


func _ready():
	# 기존 관계 데이터 로드
	load_relationship_data()


func update_relationship(player_id: int, delta: float, reason: String = ""):
	"""개별 선수와의 관계도 업데이트"""
	var current = relationships.get(player_id, 0.5)
	var new_value = clampf(current + delta, 0.0, 1.0)
	relationships[player_id] = new_value

	relationship_changed.emit(player_id, new_value)

	if reason != "":
		print("관계도 변화: Player %d %s (%.2f -> %.2f)" % [player_id, reason, current, new_value])

	# 팀 상태 업데이트
	_update_team_stats()


func update_team_morale(delta: float, reason: String = ""):
	"""팀 사기 업데이트"""
	var old_morale = team_morale
	team_morale = clampf(team_morale + delta, 0.0, 1.0)

	team_morale_changed.emit(team_morale)

	if reason != "":
		print("팀 사기 변화: %s (%.2f -> %.2f)" % [reason, old_morale, team_morale])

	# 팀 상태 업데이트
	_update_team_stats()


func update_team_chemistry(delta: float, reason: String = ""):
	"""팀 케미스트리 업데이트"""
	var old_chemistry = team_chemistry
	team_chemistry = clampf(team_chemistry + delta, 0.0, 1.0)

	team_chemistry_changed.emit(team_chemistry)

	if reason != "":
		print("팀 케미스트리 변화: %s (%.2f -> %.2f)" % [reason, old_chemistry, team_chemistry])

	# 팀 상태 업데이트
	_update_team_stats()


func _update_team_stats():
	"""팀 전체 상태 계산"""
	# 팀 화합도 = 평균 관계도
	if relationships.size() > 0:
		var total_relationship = 0.0
		for rel in relationships.values():
			total_relationship += rel
		team_cohesion = total_relationship / relationships.size()
	else:
		team_cohesion = 0.5

	# 케미스트리 = 화합도 + 사기의 평균
	team_chemistry = (team_cohesion + team_morale) / 2.0


func get_relationship(player_id: int) -> float:
	"""특정 선수와의 관계도 반환"""
	return relationships.get(player_id, 0.5)


func get_team_stats() -> Dictionary:
	"""팀 전체 상태 반환"""
	return {
		"morale": team_morale,
		"chemistry": team_chemistry,
		"cohesion": team_cohesion,
		"relationship_count": relationships.size()
	}


func get_relationship_level(player_id: int) -> String:
	"""관계도 레벨 반환"""
	var rel = get_relationship(player_id)

	if rel >= 0.9:
		return "최고의 친구"
	elif rel >= 0.8:
		return "친한 친구"
	elif rel >= 0.7:
		return "좋은 친구"
	elif rel >= 0.6:
		return "친구"
	elif rel >= 0.5:
		return "동료"
	elif rel >= 0.4:
		return "알고 지내는 사이"
	elif rel >= 0.3:
		return "낯선 사이"
	elif rel >= 0.2:
		return "어색한 사이"
	else:
		return "적대적"


func roll_relationship_event(player_data: Dictionary) -> Dictionary:
	"""관계 이벤트 발생 체크"""
	for event in RELATIONSHIP_EVENTS:
		# 조건 체크
		if not _check_event_condition(event.condition, player_data):
			continue

		# 확률 체크
		if randf() < event.probability:
			_apply_event_effects(event)
			relationship_event_triggered.emit(event)
			return event

	return {}


func _check_event_condition(condition: String, player_data: Dictionary) -> bool:
	"""이벤트 조건 체크"""
	if condition == "":
		return true

	# 간단한 조건 파서
	if condition.contains("morale"):
		var morale = team_morale
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return morale > value
		elif condition.contains("<"):
			var value = float(condition.split("<")[1].strip_edges())
			return morale < value

	if condition.contains("chemistry"):
		var chemistry = team_chemistry
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return chemistry > value
		elif condition.contains("<"):
			var value = float(condition.split("<")[1].strip_edges())
			return chemistry < value

	if condition.contains("leadership"):
		var leadership = player_data.get("leadership", 0.0)
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return leadership > value

	if condition.contains("week"):
		var week = player_data.get("week", 1)
		if condition.contains("%"):
			var parts = condition.split("%")
			var divisor = int(parts[1].split("==")[0].strip_edges())
			var remainder = int(parts[1].split("==")[1].strip_edges())
			return (week % divisor) == remainder

	return true


func _apply_event_effects(event: Dictionary):
	"""이벤트 효과 적용"""
	# 관계도 보너스
	if event.has("relationship_bonus"):
		var bonus = event.relationship_bonus
		# 모든 선수에게 적용
		for player_id in relationships:
			update_relationship(player_id, bonus, event.id)

	# 관계도 페널티
	if event.has("relationship_penalty"):
		var penalty = event.relationship_penalty
		# 모든 선수에게 적용
		for player_id in relationships:
			update_relationship(player_id, penalty, event.id)

	# 사기 보너스
	if event.has("morale_bonus"):
		update_team_morale(event.morale_bonus, event.id)

	# 사기 페널티
	if event.has("morale_penalty"):
		update_team_morale(event.morale_penalty, event.id)

	# 케미스트리 보너스
	if event.has("chemistry_bonus"):
		update_team_chemistry(event.chemistry_bonus, event.id)


func get_relationship_modifier() -> float:
	"""관계에 따른 훈련 효과 수정자 반환"""
	# 팀워크가 좋을수록 훈련 효과 향상
	var base_modifier = 1.0
	var chemistry_bonus = (team_chemistry - 0.5) * 0.2  # -0.1 ~ +0.1
	var morale_bonus = (team_morale - 0.5) * 0.1  # -0.05 ~ +0.05

	return base_modifier + chemistry_bonus + morale_bonus


func add_teammate(player_id: int, initial_relationship: float = 0.5):
	"""새로운 팀원 추가"""
	relationships[player_id] = clampf(initial_relationship, 0.0, 1.0)
	_update_team_stats()
	print("새 팀원 추가: Player %d (관계도: %.2f)" % [player_id, initial_relationship])


func remove_teammate(player_id: int):
	"""팀원 제거"""
	if relationships.has(player_id):
		relationships.erase(player_id)
		_update_team_stats()
		print("팀원 제거: Player %d" % player_id)


func save_relationship_data():
	"""관계 데이터 저장"""
	var save_data = {
		"relationships": relationships,
		"team_morale": team_morale,
		"team_chemistry": team_chemistry,
		"team_cohesion": team_cohesion,
		"manager_relationship": manager_relationship
	}

	var file = FileAccess.open("user://relationships.save", FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(save_data))
		file.close()


func load_relationship_data():
	"""관계 데이터 로드"""
	var file = FileAccess.open("user://relationships.save", FileAccess.READ)
	if file:
		var json_string = file.get_as_text()
		file.close()

		var json = JSON.new()
		var parse_result = json.parse(json_string)

		if parse_result == OK:
			var data = json.data
			relationships = data.get("relationships", {})
			team_morale = data.get("team_morale", 0.5)
			team_chemistry = data.get("team_chemistry", 0.5)
			team_cohesion = data.get("team_cohesion", 0.5)
			manager_relationship = data.get("manager_relationship", 0.0)
		else:
			print("관계 데이터 로드 실패")


func reset_relationships():
	"""관계 초기화 (테스트용)"""
	relationships.clear()
	team_morale = 0.5
	team_chemistry = 0.5
	team_cohesion = 0.5
	manager_relationship = 0.0
	save_relationship_data()
	print("관계가 초기화되었습니다.")


# MandatoryTeamTrainingManager를 위한 감독 관계 함수들
func improve_manager_relationship(amount: float):
	"""감독과의 관계 개선"""
	var old_value = manager_relationship
	manager_relationship = clampf(manager_relationship + abs(amount), -100.0, 100.0)

	print("[RelationshipSystem] 감독 관계 개선: %.1f -> %.1f (+%.1f)" % [old_value, manager_relationship, abs(amount)])

	# 감독 관계가 팀 사기에 영향
	var morale_change = abs(amount) * 0.01  # 1% per 1 point
	update_team_morale(morale_change, "감독 관계 개선")


func worsen_manager_relationship(amount: float):
	"""감독과의 관계 악화"""
	var old_value = manager_relationship
	manager_relationship = clampf(manager_relationship - abs(amount), -100.0, 100.0)

	print("[RelationshipSystem] 감독 관계 악화: %.1f -> %.1f (-%.1f)" % [old_value, manager_relationship, abs(amount)])

	# 감독 관계가 팀 사기에 영향
	var morale_change = -abs(amount) * 0.01  # -1% per 1 point
	update_team_morale(morale_change, "감독 관계 악화")


func get_manager_relationship() -> float:
	"""현재 감독 관계도 반환 (-100 ~ 100)"""
	return manager_relationship


func improve_team_chemistry(amount: float):
	"""팀 케미스트리 개선 (0 ~ 100 스케일로 변환)"""
	# 0-100 스케일을 0-1 스케일로 변환
	var delta = abs(amount) / 100.0
	update_team_chemistry(delta, "팀 케미스트리 개선")


func worsen_team_chemistry(amount: float):
	"""팀 케미스트리 악화 (0 ~ 100 스케일로 변환)"""
	# 0-100 스케일을 0-1 스케일로 변환
	var delta = -abs(amount) / 100.0
	update_team_chemistry(delta, "팀 케미스트리 악화")


# Alias 함수들 (coach = manager)
func improve_coach_relationship(amount: float):
	"""감독과의 관계 개선 (improve_manager_relationship의 별칭)"""
	improve_manager_relationship(amount)


func get_coach_relationship() -> float:
	"""현재 감독 관계도 반환 (get_manager_relationship의 별칭)"""
	return get_manager_relationship()


# 테스트 함수
func test_relationship_system():
	"""관계 시스템 테스트"""
	print("=== 관계 시스템 테스트 ===")

	# 테스트용 팀원 추가
	add_teammate(1, 0.6)
	add_teammate(2, 0.8)
	add_teammate(3, 0.4)

	# 관계도 업데이트
	update_relationship(1, 0.1, "훈련 도움")
	update_team_morale(0.05, "경기 승리")

	# 팀 상태 출력
	var stats = get_team_stats()
	print("팀 상태: ", stats)

	# 관계 이벤트 테스트
	var player_data = {"leadership": 85, "week": 26}
	var event = roll_relationship_event(player_data)
	if not event.is_empty():
		print("이벤트 발생: ", event.message)
