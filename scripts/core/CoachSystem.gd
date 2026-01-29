extends Node

# Coach Management System - 코치/감독 시스템 (Week 10 고급 기능)
# 감독과의 상호작용, 조언, 특별 훈련, 전술 지도 등을 제공

signal coach_advice_given(advice: Dictionary)
signal special_training_unlocked(training_type: String)
signal coach_relationship_changed(relationship_level: float)
signal tactical_knowledge_gained(knowledge: Dictionary)

# 코치 타입
enum CoachType { HEAD_COACH, TECHNICAL_COACH, FITNESS_COACH, MENTAL_COACH, GOALKEEPING_COACH }  # 감독 (전체적인 조언)  # 기술 코치 (기술 훈련 특화)  # 피트니스 코치 (체력 훈련 특화)  # 멘탈 코치 (정신력 훈련 특화)  # 골키퍼 코치 (골키퍼 특화)

# 코치 정보 데이터
var coaches = {
	CoachType.HEAD_COACH:
	{
		"name": "김 감독",
		"specialty": "overall_development",
		"relationship": 0.5,  # 0.0 - 1.0
		"experience": 85,  # 경험치
		"personality": "balanced",  # balanced, strict, supportive
		"advice_cooldown": 0,
		"available_tactics": ["4-4-2", "4-3-3", "3-5-2"],
		"special_abilities": ["team_motivation", "tactical_insight"]
	},
	CoachType.TECHNICAL_COACH:
	{
		"name": "박 기술코치",
		"specialty": "technical_skills",
		"relationship": 0.6,
		"experience": 78,
		"personality": "supportive",
		"advice_cooldown": 0,
		"training_bonus": {"technical": 1.2},
		"special_abilities": ["skill_development", "technique_analysis"]
	},
	CoachType.FITNESS_COACH:
	{
		"name": "이 체력코치",
		"specialty": "physical_conditioning",
		"relationship": 0.4,
		"experience": 80,
		"personality": "strict",
		"advice_cooldown": 0,
		"training_bonus": {"physical": 1.3},
		"special_abilities": ["endurance_boost", "injury_prevention"]
	},
	CoachType.MENTAL_COACH:
	{
		"name": "정 멘탈코치",
		"specialty": "mental_strength",
		"relationship": 0.7,
		"experience": 75,
		"personality": "supportive",
		"advice_cooldown": 0,
		"training_bonus": {"mental": 1.25},
		"special_abilities": ["confidence_boost", "pressure_handling"]
	}
}

# 현재 선택된 전술
var current_tactics = "4-4-2"
var tactical_knowledge = {}

# 코치 상호작용 기록
var interaction_history: Array = []


func _ready():
	print("[CoachSystem] Initializing coach management system")

	# 주간 이벤트 연결
	if GameManager:
		GameManager.week_advanced.connect(_on_week_advanced)

	# 초기 전술 지식 설정
	_initialize_tactical_knowledge()
	_register_with_tactics_manager()


func _register_with_tactics_manager():
	if has_node("/root/TacticsManager"):
		var tactics_manager = get_node("/root/TacticsManager")
		if tactics_manager and tactics_manager.has_method("register_tactical_modifier_provider"):
			tactics_manager.register_tactical_modifier_provider(self)


func _on_week_advanced(_week: int, _year: int):
	"""주간 진행 시 코치 시스템 업데이트"""
	# 조언 쿨다운 감소
	for coach_type in coaches:
		if coaches[coach_type].advice_cooldown > 0:
			coaches[coach_type].advice_cooldown -= 1

	# 주간 코치 이벤트 확률적 발생
	_check_weekly_coach_events()


func request_advice(coach_type: CoachType) -> Dictionary:
	"""코치에게 조언 요청"""
	if not coaches.has(coach_type):
		return {"success": false, "message": "존재하지 않는 코치입니다."}

	var coach = coaches[coach_type]

	# 쿨다운 체크
	if coach.advice_cooldown > 0:
		return {"success": false, "message": "%s님은 %d주 후에 다시 조언을 구할 수 있습니다." % [coach.name, coach.advice_cooldown]}

	# 관계도에 따른 조언 품질
	var advice_quality = _calculate_advice_quality(coach)
	var advice = _generate_advice(coach_type, advice_quality)

	# 쿨다운 설정 (관계도가 높을수록 짧음)
	coach.advice_cooldown = max(1, int(4 - coach.relationship * 2))

	# 관계도 향상 (소폭)
	_improve_relationship(coach_type, 0.05)

	# 상호작용 기록
	interaction_history.append(
		{
			"week": GameManager.get_current_week() if GameManager else 1,
			"coach_type": coach_type,
			"advice": advice,
			"relationship": coach.relationship
		}
	)

	coach_advice_given.emit(advice)
	return {"success": true, "advice": advice}


func _calculate_advice_quality(coach: Dictionary) -> float:
	"""조언 품질 계산 (관계도 + 경험치 기반)"""
	var relationship_factor = coach.relationship  # 0.0 - 1.0
	var experience_factor = coach.experience / 100.0  # 0.0 - 1.0
	var personality_factor = 1.0

	# 성격에 따른 조정
	match coach.personality:
		"supportive":
			personality_factor = 1.1  # 더 도움이 되는 조언
		"strict":
			personality_factor = 0.9  # 엄격하지만 효과적
		_:
			personality_factor = 1.0  # 균형잡힌 조언

	return (relationship_factor + experience_factor) * personality_factor / 2.0


func _generate_advice(coach_type: CoachType, quality: float) -> Dictionary:
	"""코치 타입에 따른 조언 생성"""
	var advice = {
		"coach_type": coach_type,
		"coach_name": coaches[coach_type].name,
		"quality": quality,
		"message": "",
		"effects": {},
		"training_recommendation": ""
	}

	match coach_type:
		CoachType.HEAD_COACH:
			advice = _generate_head_coach_advice(quality)
		CoachType.TECHNICAL_COACH:
			advice = _generate_technical_coach_advice(quality)
		CoachType.FITNESS_COACH:
			advice = _generate_fitness_coach_advice(quality)
		CoachType.MENTAL_COACH:
			advice = _generate_mental_coach_advice(quality)

	return advice


func _generate_head_coach_advice(quality: float) -> Dictionary:
	"""감독 조언 생성"""
	var messages = [
		"전체적인 밸런스가 중요합니다. 한 분야에만 치우치지 마세요.",
		"경기에서의 판단력을 기르려면 더 많은 실전 경험이 필요합니다.",
		"팀워크는 개인 기술만큼 중요합니다. 동료들과의 호흡을 맞춰보세요.",
		"당신의 포지션에서 요구되는 역할을 명확히 이해하세요.",
		"승부욕은 좋지만, 냉정한 판단력도 함께 길러야 합니다."
	]

	var training_recommendations = [
		"균형잡힌 훈련으로 전체 능력치를 고르게 발전시키세요.",
		"실전과 유사한 상황 훈련을 늘려보세요.",
		"팀 훈련 참여를 늘려 팀워크를 향상시키세요.",
		"포지션별 전문 훈련에 집중해보세요.",
		"정신력 훈련을 통해 경기 중 집중력을 높이세요."
	]

	var effects = {}
	if quality > 0.7:  # 고품질 조언
		effects = {"training_bonus": 0.1, "motivation": 15, "tactical_insight": 1}  # 다음 훈련 10% 보너스  # 동기 증가  # 전술 이해도 +1
	elif quality > 0.4:  # 중간 품질 조언
		effects = {"training_bonus": 0.05, "motivation": 10}

	return {
		"coach_type": CoachType.HEAD_COACH,
		"coach_name": coaches[CoachType.HEAD_COACH].name,
		"quality": quality,
		"message": messages[randi() % messages.size()],
		"effects": effects,
		"training_recommendation": training_recommendations[randi() % training_recommendations.size()]
	}


func _generate_technical_coach_advice(quality: float) -> Dictionary:
	"""기술 코치 조언 생성"""
	var messages = [
		"볼 터치의 정확성을 높이는 것이 가장 중요합니다.",
		"패스의 타이밍과 정확도를 더욱 개선해야 합니다.",
		"드리블 시 몸의 균형을 유지하는 연습을 더 하세요.",
		"슛 연습 시 다양한 각도에서 시도해보세요.",
		"기본기가 탄탄해야 응용 기술도 늘어납니다."
	]

	var effects = {}
	if quality > 0.7:
		effects = {"technical_bonus": 0.15, "skill_unlock": "advanced_technique"}  # 기술 훈련 15% 보너스
	elif quality > 0.4:
		effects = {"technical_bonus": 0.1}

	return {
		"coach_type": CoachType.TECHNICAL_COACH,
		"coach_name": coaches[CoachType.TECHNICAL_COACH].name,
		"quality": quality,
		"message": messages[randi() % messages.size()],
		"effects": effects,
		"training_recommendation": "기술 훈련(볼 컨트롤, 패스, 드리블) 집중 실시"
	}


func _generate_fitness_coach_advice(quality: float) -> Dictionary:
	"""피트니스 코치 조언 생성"""
	var messages = [
		"지구력을 더 기르면 후반 체력 관리에 도움이 됩니다.",
		"근력 훈련도 중요하지만 부상 방지가 우선입니다.",
		"스피드와 민첩성 향상을 위한 전문 훈련이 필요합니다.",
		"회복 훈련을 소홀히 하면 오버트레이닝이 될 수 있습니다.",
		"체력은 모든 기술의 기반입니다."
	]

	var effects = {}
	if quality > 0.7:
		effects = {"physical_bonus": 0.2, "injury_resistance": 10, "stamina_bonus": 5}  # 체력 훈련 20% 보너스  # 부상 저항력 증가  # 스태미나 직접 증가
	elif quality > 0.4:
		effects = {"physical_bonus": 0.1, "injury_resistance": 5}

	return {
		"coach_type": CoachType.FITNESS_COACH,
		"coach_name": coaches[CoachType.FITNESS_COACH].name,
		"quality": quality,
		"message": messages[randi() % messages.size()],
		"effects": effects,
		"training_recommendation": "체력 훈련(지구력, 근력, 스피드) 강화 필요"
	}


func _generate_mental_coach_advice(quality: float) -> Dictionary:
	"""멘탈 코치 조언 생성"""
	var messages = [
		"자신감을 갖되, 겸손함을 잃지 마세요.",
		"압박 상황에서도 침착함을 유지하는 연습을 하세요.",
		"팀원들과의 소통 능력도 중요한 실력입니다.",
		"실패를 두려워하지 말고 도전하세요.",
		"정신력이 강해야 기술도 빛을 발합니다."
	]

	var effects = {}
	if quality > 0.7:
		effects = {"mental_bonus": 0.15, "confidence": 20, "pressure_resistance": 10, "composure": 3}  # 정신력 훈련 15% 보너스  # 자신감 증가  # 압박 저항력  # 침착함 직접 증가
	elif quality > 0.4:
		effects = {"mental_bonus": 0.1, "confidence": 10}

	return {
		"coach_type": CoachType.MENTAL_COACH,
		"coach_name": coaches[CoachType.MENTAL_COACH].name,
		"quality": quality,
		"message": messages[randi() % messages.size()],
		"effects": effects,
		"training_recommendation": "정신력 훈련(집중력, 판단력, 리더십) 중점 실시"
	}


func _improve_relationship(coach_type: CoachType, amount: float):
	"""코치와의 관계도 향상"""
	if coaches.has(coach_type):
		coaches[coach_type].relationship = min(1.0, coaches[coach_type].relationship + amount)
		coach_relationship_changed.emit(coaches[coach_type].relationship)


func apply_advice_effects(advice: Dictionary):
	"""���� ȿ�� ����"""
	var effects_variant: Variant = advice.get("effects", {})
	var effects: Dictionary = effects_variant if effects_variant is Dictionary else {}

	# TrainingManager ���ʽ� ����
	if TrainingManager:
		if effects.has("training_bonus"):
			TrainingManager.set_temporary_bonus(effects.training_bonus)
		if effects.has("technical_bonus"):
			TrainingManager.set_category_bonus("technical", effects.technical_bonus)
		if effects.has("physical_bonus"):
			TrainingManager.set_category_bonus("physical", effects.physical_bonus)
		if effects.has("mental_bonus"):
			TrainingManager.set_category_bonus("mental", effects.mental_bonus)

	# PlayerManager ���� �ɷ�ġ ����
	var player_manager = get_node_or_null("/root/PlayerManager")
	if player_manager:
		if effects.has("stamina_bonus"):
			player_manager.add_experience("stamina", effects.stamina_bonus)
		if effects.has("composure"):
			player_manager.add_experience("composure", effects.composure)

	# Ư�� �ɷ� �ر�
	if effects.has("skill_unlock"):
		special_training_unlocked.emit(effects.skill_unlock)

	print("[CoachSystem] Applied advice effects: %s" % str(effects))


func issue_rest_warning(info: Dictionary) -> void:
	if not coaches.has(CoachType.FITNESS_COACH):
		return
	var coach = coaches[CoachType.FITNESS_COACH]
	var load_ratio: float = float(info.get("training_load", {}).get("load_ratio", 0.0))
	var message := "%s: �ֱ� �Ʒ� ���ϰ� �����ϴ� (Load %.2f). �̹� �ִ� �޽��� �����մϴ�." % [coach.name, load_ratio]
	if info.has("injury_risk") and info.injury_risk >= 0:
		message += " (���� �λ� ���� %.0f%%)" % clamp(info.injury_risk * 100.0, 0.0, 100.0)

	var advice := {
		"coach_type": CoachType.FITNESS_COACH,
		"coach_name": coach.name,
		"quality": 1.0,
		"message": message,
		"effects": {"training_bonus": 0.0},
		"training_recommendation": "rest"
	}
	coach_advice_given.emit(advice)


func change_tactics(new_tactics: String) -> bool:
	"""전술 변경"""
	var head_coach = coaches.get(CoachType.HEAD_COACH, {})
	var available_tactics = head_coach.get("available_tactics", ["4-4-2"])

	if new_tactics in available_tactics:
		current_tactics = new_tactics
		print("[CoachSystem] Tactics changed to: %s" % new_tactics)

		# 전술 지식 증가
		if not tactical_knowledge.has(new_tactics):
			tactical_knowledge[new_tactics] = 0
		tactical_knowledge[new_tactics] += 1

		tactical_knowledge_gained.emit({"tactics": new_tactics, "level": tactical_knowledge[new_tactics]})
		return true
	else:
		print("[CoachSystem] Tactics not available: %s" % new_tactics)
		return false


func _initialize_tactical_knowledge():
	"""전술 지식 초기화"""
	tactical_knowledge = {"4-4-2": 1, "4-3-3": 0, "3-5-2": 0, "4-5-1": 0, "3-4-3": 0}  # 기본 전술은 이미 알고 있음


func _check_weekly_coach_events():
	"""주간 코치 이벤트 확인"""
	# 10% 확률로 코치 이벤트 발생
	if randf() < 0.1:
		var event_type = randi() % 3
		match event_type:
			0:
				_trigger_special_training_offer()
			1:
				_trigger_tactical_lesson()
			2:
				_trigger_motivational_speech()


func _trigger_special_training_offer():
	"""특별 훈련 제안 이벤트"""
	var coach_types = [CoachType.TECHNICAL_COACH, CoachType.FITNESS_COACH, CoachType.MENTAL_COACH]
	var selected_coach = coach_types[randi() % coach_types.size()]
	var coach = coaches[selected_coach]

	print("[CoachSystem] %s님이 특별 훈련을 제안했습니다!" % coach.name)

	# TODO: DialogController 대신 다른 알림 시스템 사용 (추후 구현)
	# 현재는 콘솔 출력으로 대체
	print("[CoachSystem] %s: 특별 훈련 기회가 있습니다!" % coach.name)

	special_training_unlocked.emit("special_coaching_session")


func _trigger_tactical_lesson():
	"""전술 수업 이벤트"""
	var head_coach = coaches[CoachType.HEAD_COACH]
	print("[CoachSystem] %s님의 전술 수업이 시작됩니다!" % head_coach.name)

	# 새로운 전술 해금 기회
	var locked_tactics = []
	for tactics in ["4-3-3", "3-5-2", "4-5-1", "3-4-3"]:
		if tactical_knowledge.get(tactics, 0) == 0:
			locked_tactics.append(tactics)

	if locked_tactics.size() > 0:
		var new_tactics = locked_tactics[randi() % locked_tactics.size()]
		tactical_knowledge[new_tactics] = 1
		print("[CoachSystem] 새로운 전술 '%s'을(를) 배웠습니다!" % new_tactics)


func _trigger_motivational_speech():
	"""동기부여 연설 이벤트"""
	var head_coach = coaches[CoachType.HEAD_COACH]
	print("[CoachSystem] %s님의 격려 연설!" % head_coach.name)

	# 모든 코치와의 관계도 소폭 향상
	for coach_type in coaches:
		_improve_relationship(coach_type, 0.02)


# 공개 API 메서드
func get_coach_info(coach_type: CoachType) -> Dictionary:
	"""코치 정보 반환"""
	return coaches.get(coach_type, {}).duplicate()


func get_all_coaches() -> Dictionary:
	"""모든 코치 정보 반환"""
	return coaches.duplicate()


func get_current_tactics() -> String:
	"""현재 전술 반환"""
	return current_tactics


func get_tactical_knowledge() -> Dictionary:
	"""전술 지식 반환"""
	return tactical_knowledge.duplicate()


func get_interaction_history(limit: int = 10) -> Array:
	"""상호작용 기록 반환"""
	var start_index: int = max(0, interaction_history.size() - limit)
	return interaction_history.slice(start_index)


func get_tactical_modifiers() -> Dictionary:
	var head_coach: Dictionary = coaches.get(CoachType.HEAD_COACH, {})
	if head_coach.is_empty():
		return {}

	var relationship: float = float(head_coach.get("relationship", 0.5))
	var experience: float = float(head_coach.get("experience", 70))
	var leadership_bonus: float = (relationship - 0.5) * 20.0
	var experience_bonus: float = (experience - 70.0) * 0.1
	var knowledge_level: float = float(tactical_knowledge.get(current_tactics, 0))
	var knowledge_bonus: float = min(10.0, knowledge_level * 2.5)

	var modifiers: Dictionary = {
		"attacking_intensity": leadership_bonus + experience_bonus,
		"pressing": leadership_bonus * 0.8,
		"tempo": knowledge_bonus,
		"defensive_line": -knowledge_bonus * 0.5
	}

	if DeckManager and DeckManager.has_method("get_deck_quality_score"):
		var deck_quality: float = float(DeckManager.get_deck_quality_score())
		modifiers["pressing"] += deck_quality * 10.0
		modifiers["attacking_intensity"] += deck_quality * 5.0

	return modifiers


func is_advice_available(coach_type: CoachType) -> bool:
	"""조언 가능 여부 확인"""
	if not coaches.has(coach_type):
		return false
	return coaches[coach_type].advice_cooldown == 0


# 테스트 함수
func test_coach_system():
	"""코치 시스템 테스트"""
	print("=== Coach System Test ===")

	# 각 코치에게 조언 요청
	for coach_type in coaches:
		var result = request_advice(coach_type)
		print("Advice from %s: %s" % [coaches[coach_type].name, result])

	# 전술 변경 테스트
	print("Current tactics: %s" % get_current_tactics())
	change_tactics("4-3-3")
	print("New tactics: %s" % get_current_tactics())

	# 전술 지식 출력
	print("Tactical knowledge: %s" % str(get_tactical_knowledge()))

	print("✅ Coach system test completed")
