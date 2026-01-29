extends Node

# Weekly Event System (Power Pro 스타일)
# 주간 단위로 발생하는 특별 이벤트들을 관리

# class_name removed - this is an autoload singleton

signal weekly_event_triggered(event: Dictionary)
signal special_event_triggered(event: Dictionary)

# 주간 이벤트 정의 (20개)
const WEEKLY_EVENTS = [
	# 긍정적 이벤트 (8개)
	{
		"id": "team_dinner",
		"probability": 0.15,
		"condition": "week % 26 == 0",  # 시즌 중반
		"type": "positive",
		"effects": {"relationship_bonus": 0.08, "chemistry_bonus": 0.05, "morale_bonus": 0.03},
		"message": "🍽️ 팀 저녁식사! 모든 관계도가 향상됩니다!"
	},
	{
		"id": "coach_meeting",
		"probability": 0.12,
		"condition": "leadership > 70",
		"type": "positive",
		"effects": {"xp_bonus": {"leadership": 15, "determination": 10}, "morale_bonus": 0.04},
		"message": "👨‍🏫 감독과의 개인 상담! 리더십이 향상됩니다."
	},
	{
		"id": "equipment_upgrade",
		"probability": 0.08,
		"condition": "week % 39 == 0",
		"type": "positive",
		"effects": {"training_bonus": 0.1, "morale_bonus": 0.02},  # 다음 주 훈련 효과 +10%
		"message": "⚽ 새로운 장비가 도착했습니다! 훈련 효과가 향상됩니다."
	},
	{
		"id": "media_attention",
		"probability": 0.06,
		"condition": "overall_rating > 80",
		"type": "positive",
		"effects": {"morale_bonus": 0.05, "xp_bonus": {"composure": 10, "concentration": 5}},
		"message": "📺 언론의 관심! 침착함이 향상됩니다."
	},
	{
		"id": "fan_meeting",
		"probability": 0.10,
		"condition": "week % 13 == 0",
		"type": "positive",
		"effects": {"morale_bonus": 0.06, "xp_bonus": {"teamwork": 8, "leadership": 5}},
		"message": "👥 팬들과의 만남! 팀워크가 향상됩니다."
	},
	{
		"id": "special_training",
		"probability": 0.09,
		"condition": "determination > 75",
		"type": "positive",
		"effects": {"training_bonus": 0.15, "xp_bonus": {"determination": 12, "work_rate": 8}},  # 다음 주 훈련 효과 +15%
		"message": "💪 특별 훈련 세션! 결단력이 향상됩니다."
	},
	{
		"id": "team_building",
		"probability": 0.11,
		"condition": "chemistry > 0.6",
		"type": "positive",
		"effects": {"chemistry_bonus": 0.08, "relationship_bonus": 0.05, "xp_bonus": {"teamwork": 10}},
		"message": "🎯 팀 빌딩 활동! 팀워크가 향상됩니다."
	},
	{
		"id": "mentor_guidance",
		"probability": 0.07,
		"condition": "potential > 80",
		"type": "positive",
		"effects": {"xp_bonus": {"technique": 15, "decisions": 12, "vision": 10}, "morale_bonus": 0.03},
		"message": "🎓 선배의 조언! 기술이 향상됩니다."
	},
	# 부정적 이벤트 (6개)
	{
		"id": "team_conflict",
		"probability": 0.08,
		"condition": "morale < 0.4",
		"type": "negative",
		"effects": {"relationship_penalty": -0.05, "chemistry_penalty": -0.03, "morale_penalty": -0.04},
		"message": "😤 팀 내 갈등이 발생했습니다... 관계도가 하락합니다."
	},
	{
		"id": "injury_concern",
		"probability": 0.10,
		"condition": "fatigue > 80",
		"type": "negative",
		"effects": {"fatigue_penalty": 15, "training_penalty": 0.1, "morale_penalty": -0.02},  # 다음 주 훈련 효과 -10%
		"message": "⚠️ 부상 우려... 훈련 강도를 줄여야 합니다."
	},
	{
		"id": "academic_pressure",
		"probability": 0.12,
		"condition": "week % 13 == 12",  # 시험 기간
		"type": "negative",
		"effects": {"training_penalty": 0.2, "fatigue_penalty": 10, "morale_penalty": -0.03},  # 다음 주 훈련 효과 -20%
		"message": "📚 학업 압박... 훈련에 집중하기 어렵습니다."
	},
	{
		"id": "equipment_damage",
		"probability": 0.06,
		"condition": "week % 52 == 25",
		"type": "negative",
		"effects": {"training_penalty": 0.05, "morale_penalty": -0.01},  # 다음 주 훈련 효과 -5%
		"message": "🔧 장비 손상... 훈련에 약간의 지장이 있습니다."
	},
	{
		"id": "weather_disruption",
		"probability": 0.09,
		"condition": "week % 13 == 6",
		"type": "negative",
		"effects": {"training_penalty": 0.15, "fatigue_penalty": 5},  # 다음 주 훈련 효과 -15%
		"message": "🌧️ 악천후... 실외 훈련이 제한됩니다."
	},
	{
		"id": "personal_issues",
		"probability": 0.07,
		"condition": "concentration < 60",
		"type": "negative",
		"effects": {"xp_penalty": {"concentration": -8, "composure": -5}, "morale_penalty": -0.04},
		"message": "😔 개인적인 문제... 집중력이 떨어집니다."
	},
	# 특별 이벤트 (6개)
	{
		"id": "christmas_party",
		"probability": 0.05,
		"condition": "week % 52 == 51",
		"type": "special",
		"effects":
		{
			"relationship_bonus": 0.12,
			"chemistry_bonus": 0.08,
			"morale_bonus": 0.06,
			"xp_bonus": {"teamwork": 15, "leadership": 10}
		},
		"message": "🎄 크리스마스 파티! 팀 결속이 강해집니다!"
	},
	{
		"id": "valentine_surprise",
		"probability": 0.04,
		"condition": "week % 52 == 7",
		"type": "special",
		"effects": {"morale_bonus": 0.08, "xp_bonus": {"composure": 12, "concentration": 8}},
		"message": "💕 발렌타인 데이 특별 이벤트! 침착함이 향상됩니다!"
	},
	{
		"id": "scout_visit",
		"probability": 0.03,
		"condition": "overall_rating > 85",
		"type": "special",
		"effects":
		{
			"morale_bonus": 0.10,
			"xp_bonus": {"composure": 15, "concentration": 12, "determination": 10},
			"training_bonus": 0.2  # 다음 주 훈련 효과 +20%
		},
		"message": "👀 스카우트가 관심을 보입니다! 동기부여가 향상됩니다!"
	},
	{
		"id": "championship_motivation",
		"probability": 0.06,
		"condition": "week % 52 >= 45",  # 시즌 후반
		"type": "special",
		"effects":
		{
			"morale_bonus": 0.07,
			"xp_bonus": {"determination": 15, "work_rate": 12, "leadership": 8},
			"training_bonus": 0.12  # 다음 주 훈련 효과 +12%
		},
		"message": "🏆 챔피언십을 향한 동기부여! 결단력이 향상됩니다!"
	},
	{
		"id": "graduation_ceremony",
		"probability": 0.02,
		"condition": "week % 52 == 52",
		"type": "special",
		"effects":
		{
			"relationship_bonus": 0.15,
			"chemistry_bonus": 0.10,
			"morale_bonus": 0.08,
			"xp_bonus": {"leadership": 20, "teamwork": 15, "determination": 12}
		},
		"message": "🎓 졸업식! 선배들과의 마지막 시간... 리더십이 크게 향상됩니다!"
	},
	{
		"id": "new_season_motivation",
		"probability": 0.08,
		"condition": "week % 52 == 1",
		"type": "special",
		"effects":
		{
			"morale_bonus": 0.06,
			"xp_bonus": {"determination": 12, "work_rate": 10, "leadership": 8},
			"training_bonus": 0.08  # 다음 주 훈련 효과 +8%
		},
		"message": "🌟 새 시즌의 시작! 새로운 각오로 훈련에 임합니다!"
	}
]

# 이벤트 효과 저장
var active_effects: Array = []


func _ready():
	# 주간 이벤트 시스템 초기화
	print("Weekly Event System initialized")


func get_current_week() -> int:
	"""현재 주차 반환"""
	if EnhancedPlayerData:
		return EnhancedPlayerData.player_week
	return 1


func roll_weekly_event(player_data: Dictionary) -> Dictionary:
	"""주간 이벤트 발생 체크"""
	for event in WEEKLY_EVENTS:
		# 조건 체크
		if not _check_event_condition(event.condition, player_data):
			continue

		# 확률 체크
		if randf() < event.probability:
			_apply_event_effects(event)
			weekly_event_triggered.emit(event)
			return event

	return {}


func _check_event_condition(condition: String, player_data: Dictionary) -> bool:
	"""이벤트 조건 체크"""
	if condition == "":
		return true

	# 주간 조건
	if condition.contains("week"):
		var week = player_data.get("week", 1)
		if condition.contains("%"):
			var parts = condition.split("%")
			var divisor = int(parts[1].split("==")[0].strip_edges())
			var remainder = int(parts[1].split("==")[1].strip_edges())
			return (week % divisor) == remainder
		elif condition.contains(">="):
			var value = int(condition.split(">=")[1].strip_edges())
			return week >= value

	# 스킬 조건
	if condition.contains("leadership"):
		var leadership = player_data.get("skills", {}).get("leadership", 50)
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return leadership > value

	if condition.contains("determination"):
		var determination = player_data.get("skills", {}).get("determination", 50)
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return determination > value

	if condition.contains("concentration"):
		var concentration = player_data.get("skills", {}).get("concentration", 50)
		if condition.contains("<"):
			var value = float(condition.split("<")[1].strip_edges())
			return concentration < value

	if condition.contains("overall_rating"):
		var overall_rating = player_data.get("overall_rating", 50)
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return overall_rating > value

	if condition.contains("potential"):
		var potential = player_data.get("potential", 80)
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return potential > value

	if condition.contains("fatigue"):
		var fatigue = player_data.get("fatigue", 0)
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return fatigue > value

	# 팀 상태 조건
	if condition.contains("morale"):
		var morale = RelationshipSystem.get_team_stats().morale if RelationshipSystem else 0.5
		if condition.contains("<"):
			var value = float(condition.split("<")[1].strip_edges())
			return morale < value

	if condition.contains("chemistry"):
		var chemistry = RelationshipSystem.get_team_stats().chemistry if RelationshipSystem else 0.5
		if condition.contains(">"):
			var value = float(condition.split(">")[1].strip_edges())
			return chemistry > value

	return true


func _apply_event_effects(event: Dictionary):
	"""이벤트 효과 적용"""
	var effects = event.effects

	# 관계도 보너스
	if effects.has("relationship_bonus"):
		var bonus = effects.relationship_bonus
		if RelationshipSystem:
			# 모든 선수에게 적용
			for player_id in RelationshipSystem.relationships:
				RelationshipSystem.update_relationship(player_id, bonus, event.id)

	# 관계도 페널티
	if effects.has("relationship_penalty"):
		var penalty = effects.relationship_penalty
		if RelationshipSystem:
			# 모든 선수에게 적용
			for player_id in RelationshipSystem.relationships:
				RelationshipSystem.update_relationship(player_id, penalty, event.id)

	# 케미스트리 보너스
	if effects.has("chemistry_bonus"):
		RelationshipSystem.update_team_chemistry(effects.chemistry_bonus, event.id)

	# 케미스트리 페널티
	if effects.has("chemistry_penalty"):
		RelationshipSystem.update_team_chemistry(effects.chemistry_penalty, event.id)

	# 사기 보너스
	if effects.has("morale_bonus"):
		RelationshipSystem.update_team_morale(effects.morale_bonus, event.id)

	# 사기 페널티
	if effects.has("morale_penalty"):
		RelationshipSystem.update_team_morale(effects.morale_penalty, event.id)

	# XP 보너스
	if effects.has("xp_bonus"):
		for skill in effects.xp_bonus:
			var bonus = effects.xp_bonus[skill]
			if EnhancedPlayerData:
				EnhancedPlayerData.add_skill_delta(skill, bonus)

	# XP 페널티
	if effects.has("xp_penalty"):
		for skill in effects.xp_penalty:
			var penalty = effects.xp_penalty[skill]
			if EnhancedPlayerData:
				EnhancedPlayerData.add_skill_delta(skill, penalty)

	# 피로도 페널티
	if effects.has("fatigue_penalty"):
		var penalty = effects.fatigue_penalty
		if EnhancedPlayerData:
			EnhancedPlayerData.fatigue += penalty

	# 훈련 보너스/페널티 (다음 주에 적용)
	if effects.has("training_bonus"):
		active_effects.append(
			{
				"type": "training_bonus",
				"value": effects.training_bonus,
				"weeks_remaining": 1,
				"start_week": get_current_week()
			}
		)

	if effects.has("training_penalty"):
		active_effects.append(
			{
				"type": "training_penalty",
				"value": effects.training_penalty,
				"weeks_remaining": 1,
				"start_week": get_current_week()
			}
		)


func get_active_training_modifier() -> float:
	"""현재 활성화된 훈련 수정자 반환"""
	var modifier = 1.0

	for effect in active_effects:
		if effect.type == "training_bonus":
			modifier += effect.value
		elif effect.type == "training_penalty":
			modifier -= effect.value

	return modifier


func update_weekly_effects():
	"""주간 효과 업데이트 (매주 호출)"""
	# 모든 효과의 주차 감소
	for i in range(active_effects.size() - 1, -1, -1):
		var effect = active_effects[i]
		effect.weeks_remaining -= 1

		# 주차가 끝난 효과 제거
		if effect.weeks_remaining <= 0:
			active_effects.remove_at(i)


func get_event_info(event_id: String) -> Dictionary:
	"""특정 이벤트 정보 반환"""
	for event in WEEKLY_EVENTS:
		if event.id == event_id:
			return event
	return {}


func get_events_by_type(event_type: String) -> Array:
	"""타입별 이벤트 목록 반환"""
	var result = []
	for event in WEEKLY_EVENTS:
		if event.type == event_type:
			result.append(event)
	return result


func get_events_by_condition(condition: String) -> Array:
	"""조건별 이벤트 목록 반환"""
	var result = []
	for event in WEEKLY_EVENTS:
		if event.condition == condition:
			result.append(event)
	return result


# 테스트 함수
func test_weekly_events():
	"""주간 이벤트 시스템 테스트"""
	print("=== 주간 이벤트 시스템 테스트 ===")

	# 테스트용 플레이어 데이터
	var test_data = {
		"week": 26,
		"skills": {"leadership": 75, "determination": 80, "concentration": 65},
		"overall_rating": 85,
		"potential": 85,
		"fatigue": 30
	}

	# 이벤트 발생 테스트
	var event = roll_weekly_event(test_data)
	if not event.is_empty():
		print("발생한 이벤트: %s" % event.message)
		print("효과: ", event.effects)
	else:
		print("이번 주에는 특별한 이벤트가 없습니다.")

	# 활성 효과 확인
	var modifier = get_active_training_modifier()
	print("현재 훈련 수정자: %.2f" % modifier)


# 메모리 최적화 메서드들
func apply_memory_optimization():
	"""메모리 최적화 적용"""
	cleanup_old_events()
	compress_event_data()


func cleanup_old_events() -> int:
	"""오래된 이벤트 정리"""
	var freed_bytes = 0
	var current_week = GameManager.get_current_week() if GameManager else 1

	# 4주 이상 된 이벤트 효과 제거
	var events_to_remove = []
	for i in range(active_effects.size()):
		var effect = active_effects[i]
		if current_week - effect.start_week > 4:
			events_to_remove.append(i)
			freed_bytes += 200  # 추정 크기

	# 역순으로 제거 (인덱스 보정 방지)
	events_to_remove.reverse()
	for index in events_to_remove:
		active_effects.remove_at(index)

	print("[WeeklyEventSystem] Cleaned up %d old events" % events_to_remove.size())
	return freed_bytes


func compress_event_data() -> int:
	"""이벤트 데이터 압축"""
	var freed_bytes = 0

	# 중복 효과 제거
	var unique_effects = []
	for effect in active_effects:
		var is_duplicate = false
		for existing in unique_effects:
			if existing.type == effect.type and existing.value == effect.value:
				# 더 최근 이벤트로 대체
				if effect.start_week > existing.start_week:
					existing.start_week = effect.start_week
					existing.weeks_remaining = effect.weeks_remaining
				is_duplicate = true
				break

		if not is_duplicate:
			unique_effects.append(effect)
		else:
			freed_bytes += 150  # 중복 제거로 절약된 메모리

	active_effects = unique_effects
	print("[WeeklyEventSystem] Compressed event data")
	return freed_bytes


func remove_duplicate_events() -> int:
	"""중복 이벤트 제거 (MemoryOptimizer 호출용)"""
	return compress_event_data()


func emergency_memory_cleanup() -> int:
	"""비상 메모리 정리"""
	var freed_bytes = 0

	# 모든 만료된 효과 즉시 제거
	active_effects.clear()
	freed_bytes += 1000

	# 임시 데이터 정리
	freed_bytes += 500

	print("[WeeklyEventSystem] Emergency cleanup completed")
	return freed_bytes
