extends Node

# Manager System - 감독 시스템 (팀훈련 결정, 징계권, 전술 지시)
# Coach와 완전 분리: Manager는 권위자, Coach는 조언자

signal manager_decision_made(decision_type: String, details: Dictionary)
signal disciplinary_action_taken(action: String, severity: int)
signal tactical_change_announced(new_tactics: String)
signal manager_relationship_changed(relationship: float)

enum ManagerType { AUTHORITARIAN, DEMOCRATIC, TACTICAL, MOTIVATIONAL, DEVELOPMENTAL }  # 권위주의 - 엄격한 규율, 강한 처벌  # 민주적 - 선수 의견 수렴, 대화 중시  # 전술가 - 전술과 시스템 중시  # 동기부여형 - 선수 격려와 정신력 중시  # 육성형 - 젊은 선수 발전에 집중

enum TrainingPhilosophy { PHYSICAL_FOCUSED, TECHNICAL_FOCUSED, DEFENSIVE_FOCUSED, ATTACKING_FOCUSED, BALANCED_APPROACH }  # 체력파 - PACE/POWER 중심  # 기술파 - TECHNICAL/PASSING 중심  # 수비파 - DEFENDING 중심  # 공격파 - SHOOTING 중심  # 균형파 - 모든 훈련 균등

# 감독 정보
var manager_data = {
	"name": "김철수 감독",
	"type": ManagerType.TACTICAL,
	"experience": 85,
	"relationship": 50.0,  # 0-100 스케일
	"philosophy": TrainingPhilosophy.TECHNICAL_FOCUSED,
	"strictness": 0.7,  # 0.0(관대) ~ 1.0(엄격)
	"reputation": 75,  # 감독으로서의 평판
	"contract_years": 3,  # 계약 기간
	"tactics": "4-3-3",
	"preferred_style": "possession_based"
}

# 감독별 팀훈련 가중치
const MANAGER_TRAINING_WEIGHTS = {
	TrainingPhilosophy.PHYSICAL_FOCUSED:
	{"PACE": 0.30, "POWER": 0.35, "DEFENDING": 0.20, "TECHNICAL": 0.05, "SHOOTING": 0.05, "PASSING": 0.05},
	TrainingPhilosophy.TECHNICAL_FOCUSED:
	{"TECHNICAL": 0.40, "PASSING": 0.30, "SHOOTING": 0.15, "PACE": 0.05, "POWER": 0.05, "DEFENDING": 0.05},
	TrainingPhilosophy.DEFENSIVE_FOCUSED:
	{"DEFENDING": 0.50, "POWER": 0.25, "PACE": 0.15, "TECHNICAL": 0.05, "SHOOTING": 0.03, "PASSING": 0.02},
	TrainingPhilosophy.ATTACKING_FOCUSED:
	{"SHOOTING": 0.35, "TECHNICAL": 0.25, "PASSING": 0.20, "PACE": 0.10, "POWER": 0.05, "DEFENDING": 0.05},
	TrainingPhilosophy.BALANCED_APPROACH:
	{"PACE": 0.17, "POWER": 0.17, "TECHNICAL": 0.17, "SHOOTING": 0.17, "PASSING": 0.16, "DEFENDING": 0.16}
}

# 감독 타입별 특성
const MANAGER_TYPE_TRAITS = {
	ManagerType.AUTHORITARIAN:  # 불참 시 페널티 2배  # 매우 경직됨  # 칭찬 빈도 낮음  # 비판 강도 높음
	{"skip_penalty_multiplier": 2.0, "flexibility": 0.1, "praise_frequency": 0.2, "criticism_severity": 1.5},
	ManagerType.DEMOCRATIC:  # 유연함  # 부드러운 비판
	{"skip_penalty_multiplier": 1.0, "flexibility": 0.8, "praise_frequency": 0.7, "criticism_severity": 0.8},
	ManagerType.TACTICAL:
	{
		"skip_penalty_multiplier": 1.3,
		"flexibility": 0.4,
		"praise_frequency": 0.4,
		"criticism_severity": 1.1,
		"tactical_bonus": 1.2  # 전술 이해도 보너스
	},
	ManagerType.MOTIVATIONAL:
	{
		"skip_penalty_multiplier": 0.8,
		"flexibility": 0.6,
		"praise_frequency": 0.9,  # 격려 많이 함
		"criticism_severity": 0.6,
		"motivation_bonus": 1.3  # 동기부여 보너스
	},
	ManagerType.DEVELOPMENTAL:
	{
		"skip_penalty_multiplier": 0.7,  # 젊은 선수에게 관대
		"flexibility": 0.9,  # 매우 유연
		"praise_frequency": 0.8,
		"criticism_severity": 0.5,  # 매우 부드러운 비판
		"growth_bonus": 1.2  # 성장 보너스
	}
}

# 징계 기록
var disciplinary_record = {"warnings": 0, "bench_count": 0, "suspension_count": 0, "last_incident_week": 0}


func _ready():
	print("[ManagerSystem] Initializing manager system")

	# 주간 이벤트 연결
	if GameManager:
		GameManager.week_advanced.connect(_on_week_advanced)


func _on_week_advanced(_week: int, _year: int):
	"""주간 진행 시 감독 시스템 업데이트"""
	# 주간 감독 이벤트
	_check_weekly_manager_events()

	# 관계도 자연 변화 (시간이 지나면서 서서히 중립으로)
	_apply_natural_relationship_decay()


func decide_weekly_training() -> int:
	"""감독이 이번 주 팀훈련을 100% 결정"""
	var philosophy = manager_data.philosophy
	var weights = MANAGER_TRAINING_WEIGHTS[philosophy]

	# 감독 타입에 따른 추가 조정
	weights = _apply_manager_type_adjustment(weights)

	# 최근 경기 결과에 따른 조정
	weights = _apply_performance_based_adjustment(weights)

	var selected_training = _weighted_random_selection(weights)

	# 감독 결정 발표
	_announce_training_decision(selected_training)

	return selected_training


func _weighted_random_selection(weights: Dictionary) -> int:
	"""가중치 기반 랜덤 선택"""
	var training_indices = {"PACE": 0, "POWER": 1, "TECHNICAL": 2, "SHOOTING": 3, "PASSING": 4, "DEFENDING": 5}

	var total_weight = 0.0
	for weight in weights.values():
		total_weight += weight

	var random_value = randf() * total_weight
	var accumulated_weight = 0.0

	for training_name in weights:
		accumulated_weight += weights[training_name]
		if random_value <= accumulated_weight:
			return training_indices[training_name]

	return 0  # 기본값: PACE


func _apply_manager_type_adjustment(base_weights: Dictionary) -> Dictionary:
	"""감독 타입에 따른 훈련 가중치 조정"""
	var adjusted_weights = base_weights.duplicate()
	var manager_type = manager_data.type

	match manager_type:
		ManagerType.AUTHORITARIAN:
			# 권위주의는 수비와 체력 훈련 선호
			adjusted_weights["DEFENDING"] *= 1.3
			adjusted_weights["POWER"] *= 1.2

		ManagerType.TACTICAL:
			# 전술가는 기술과 패싱 훈련 선호
			adjusted_weights["TECHNICAL"] *= 1.2
			adjusted_weights["PASSING"] *= 1.3

		ManagerType.DEVELOPMENTAL:
			# 육성형은 모든 훈련을 고르게 (가중치 평준화)
			var avg_weight = 0.0
			for weight in adjusted_weights.values():
				avg_weight += weight
			avg_weight /= adjusted_weights.size()

			for training in adjusted_weights:
				adjusted_weights[training] = lerp(adjusted_weights[training], avg_weight, 0.3)

	return adjusted_weights


func _apply_performance_based_adjustment(base_weights: Dictionary) -> Dictionary:
	"""최근 경기 성과에 따른 훈련 조정"""
	var adjusted_weights = base_weights.duplicate()

	# 최근 실점이 많았다면 수비 훈련 증가
	if _get_recent_goals_conceded() > 2:
		adjusted_weights["DEFENDING"] *= 1.4
		adjusted_weights["POWER"] *= 1.2
		print("[ManagerSystem] 감독: '수비가 너무 약하다! 수비 훈련을 늘리겠다.'")

	# 최근 득점이 적었다면 공격 훈련 증가
	if _get_recent_goals_scored() < 1:
		adjusted_weights["SHOOTING"] *= 1.3
		adjusted_weights["TECHNICAL"] *= 1.2
		print("[ManagerSystem] 감독: '골이 부족하다. 공격 훈련에 집중하자.'")

	return adjusted_weights


func _announce_training_decision(training_type: int):
	"""감독의 훈련 결정 발표"""
	var training_names = {
		0: "⚡ PACE Training",
		1: "💪 POWER Training",
		2: "⚽ TECHNICAL Training",
		3: "🎯 SHOOTING Training",
		4: "🔄 PASSING Training",
		5: "🛡️ DEFENDING Training"
	}

	var philosophy_messages = {
		TrainingPhilosophy.PHYSICAL_FOCUSED: "강한 몸이 모든 것의 기초다!",
		TrainingPhilosophy.TECHNICAL_FOCUSED: "기술이 승부를 결정한다.",
		TrainingPhilosophy.DEFENSIVE_FOCUSED: "수비는 모든 것의 기본이다!",
		TrainingPhilosophy.ATTACKING_FOCUSED: "공격이 최고의 수비다!",
		TrainingPhilosophy.BALANCED_APPROACH: "모든 것이 조화를 이루어야 한다."
	}

	var training_name = training_names[training_type]
	var philosophy_msg = philosophy_messages[manager_data.philosophy]
	var manager_name = manager_data.name

	print("[ManagerSystem] %s: '이번 주는 %s을 하겠다. %s'" % [manager_name, training_name, philosophy_msg])

	# 시그널 발송
	manager_decision_made.emit(
		"weekly_training",
		{"training_type": training_type, "training_name": training_name, "manager_message": philosophy_msg}
	)


func handle_training_absence() -> Dictionary:
	"""팀훈련 불참에 대한 감독의 징계 처분"""
	disciplinary_record.warnings += 1
	disciplinary_record.last_incident_week = GameManager.get_current_week() if GameManager else 1

	var manager_type = manager_data.type
	var traits = MANAGER_TYPE_TRAITS[manager_type]
	var base_penalty = -15.0 * traits.skip_penalty_multiplier

	# 관계도 악화
	var relationship_penalty = base_penalty * (1.0 + manager_data.strictness)
	_change_relationship(relationship_penalty)

	var action_result = {
		"severity": 1,
		"relationship_penalty": relationship_penalty,
		"warning_count": disciplinary_record.warnings,
		"message": "",
		"consequences": []
	}

	# 경고 횟수에 따른 처벌 강화
	match disciplinary_record.warnings:
		1:
			action_result.message = _get_first_warning_message()
			action_result.consequences = ["경고_1회"]

		2:
			action_result.message = _get_second_warning_message()
			action_result.consequences = ["경고_2회", "추가_훈련"]
			action_result.severity = 2

		3:
			action_result.message = _get_bench_warning_message()
			action_result.consequences = ["경고_3회", "벤치_경고"]
			action_result.severity = 3

		4:
			action_result.message = _get_bench_punishment_message()
			action_result.consequences = ["벤치_처분"]
			action_result.severity = 4
			disciplinary_record.bench_count += 1

		_:  # 5회 이상
			action_result.message = _get_expulsion_message()
			action_result.consequences = ["퇴부_처분", "게임오버_위험"]
			action_result.severity = 5

	print("[ManagerSystem] 징계 처분: %s (관계도: %.1f)" % [action_result.message, manager_data.relationship])

	disciplinary_action_taken.emit("training_absence", action_result.severity)
	return action_result


func _get_first_warning_message() -> String:
	match manager_data.type:
		ManagerType.AUTHORITARIAN:
			return "감독: '규율을 어기면 용서하지 않는다. 다음에는 없다.'"
		ManagerType.DEMOCRATIC:
			return "감독: '무슨 일이 있었나? 다음엔 미리 이야기하자.'"
		ManagerType.MOTIVATIONAL:
			return "감독: '너에게 실망했다. 팀을 생각해서라도 참석하자.'"
		_:
			return "감독: '팀훈련은 필수다. 다음에는 꼭 참석하거라.'"


func _get_second_warning_message() -> String:
	match manager_data.type:
		ManagerType.AUTHORITARIAN:
			return "감독: '두 번째 경고다. 벤치에서 경기를 보고 싶지 않으면 정신 차려라.'"
		ManagerType.DEMOCRATIC:
			return "감독: '이해할 수 없다. 팀에 대한 책임감을 보여달라.'"
		_:
			return "감독: '이런 식으로 하면 벤치다. 마지막 경고다.'"


func _get_bench_warning_message() -> String:
	return "감독: '세 번째다. 한 번 더 빠지면 벤치에서 경기를 봐야 할 것이다.'"


func _get_bench_punishment_message() -> String:
	return "감독: '벤치에 앉아서 반성하며 경기를 지켜봐라. 팀워크가 뭔지 깨달을 때까지.'"


func _get_expulsion_message() -> String:
	return "감독: '팀에 헌신할 의지가 없는 선수는 필요 없다. 퇴부 처분을 검토하겠다.'"


func _change_relationship(amount: float):
	"""감독과의 관계도 변경"""
	manager_data.relationship = clamp(manager_data.relationship + amount, 0.0, 100.0)
	manager_relationship_changed.emit(manager_data.relationship)


func praise_player(reason: String) -> Dictionary:
	"""감독의 선수 칭찬"""
	var manager_type = manager_data.type
	var traits = MANAGER_TYPE_TRAITS[manager_type]

	# 칭찬 빈도에 따른 관계도 향상
	var relationship_bonus = 8.0 * traits.praise_frequency
	_change_relationship(relationship_bonus)

	var praise_messages = {
		ManagerType.AUTHORITARIAN: "감독: '이번만은 잘했다. 계속 이렇게 해라.'",
		ManagerType.DEMOCRATIC: "감독: '훌륭했다! 팀을 위한 네 노력을 높이 산다.'",
		ManagerType.MOTIVATIONAL: "감독: '대단하다! 너라면 더 큰 일도 할 수 있어!'",
		ManagerType.DEVELOPMENTAL: "감독: '많이 늘었구나! 이 조자로 계속 성장하자.'",
		ManagerType.TACTICAL: "감독: '잘했다. 이런 모습을 계속 보여달라.'"
	}

	var result = {
		"relationship_bonus": relationship_bonus,
		"message": praise_messages.get(manager_type, "감독: '잘했다. 이런 모습을 계속 보여달라.'"),
		"reason": reason
	}

	print("[ManagerSystem] 감독 칭찬: %s (+%.1f 관계도)" % [result.message, relationship_bonus])
	return result


func criticize_player(reason: String) -> Dictionary:
	"""감독의 선수 비판"""
	var manager_type = manager_data.type
	var traits = MANAGER_TYPE_TRAITS[manager_type]

	# 비판 강도에 따른 관계도 하락
	var relationship_penalty = -5.0 * traits.criticism_severity
	_change_relationship(relationship_penalty)

	var criticism_messages = {
		ManagerType.AUTHORITARIAN: "감독: '이런 식으로 하면 팀에서 나갈 수밖에 없다.'",
		ManagerType.DEMOCRATIC: "감독: '실망스럽다. 우리 함께 해결책을 찾아보자.'",
		ManagerType.MOTIVATIONAL: "감독: '너라면 더 잘할 수 있다고 믿었는데...'",
		ManagerType.DEVELOPMENTAL: "감독: '실수는 성장의 기회다. 다음엔 더 잘하자.'",
		ManagerType.TACTICAL: "감독: '기대에 못 미친다. 더 노력해야겠다.'"
	}

	var result = {
		"relationship_penalty": relationship_penalty,
		"message": criticism_messages.get(manager_type, "감독: '기대에 못 미친다. 더 노력해야겠다.'"),
		"reason": reason
	}

	print("[ManagerSystem] 감독 비판: %s (%.1f 관계도)" % [result.message, relationship_penalty])
	return result


func change_tactics(new_tactics: String) -> bool:
	"""감독의 전술 변경"""
	var available_tactics = ["4-4-2", "4-3-3", "3-5-2", "4-2-3-1", "3-4-3"]

	if new_tactics in available_tactics:
		manager_data.tactics = new_tactics
		print("[ManagerSystem] %s: '전술을 %s로 변경한다.'" % [manager_data.name, new_tactics])

		tactical_change_announced.emit(new_tactics)
		return true
	else:
		print("[ManagerSystem] 사용할 수 없는 전술: %s" % new_tactics)
		return false


func _check_weekly_manager_events():
	"""주간 감독 이벤트 확인"""
	# 15% 확률로 감독 이벤트 발생
	if randf() < 0.15:
		var event_type = randi() % 4
		match event_type:
			0:
				_trigger_tactical_meeting()
			1:
				_trigger_individual_meeting()
			2:
				_trigger_team_speech()
			3:
				_trigger_performance_review()


func _trigger_tactical_meeting():
	"""전술 회의 이벤트"""
	print("[ManagerSystem] %s님이 전술 회의를 소집했습니다." % manager_data.name)

	if manager_data.type == ManagerType.TACTICAL:
		# 전술가 감독의 특별 보너스
		var _bonus = {"tactical_understanding": 5, "positioning": 3}
		print("[ManagerSystem] 전술 이해도가 향상되었습니다!")


func _trigger_individual_meeting():
	"""개인 면담 이벤트"""
	if manager_data.relationship < 30:
		var _criticism = criticize_player("최근 경기력")
	elif manager_data.relationship > 70:
		var _praise = praise_player("꾸준한 노력")
	else:
		print("[ManagerSystem] 감독: '현재 상태를 유지하면서 더 발전해 나가자.'")


func _trigger_team_speech():
	"""팀 연설 이벤트"""
	if manager_data.type == ManagerType.MOTIVATIONAL:
		# 동기부여형 감독의 특별 효과
		_change_relationship(5.0)
		print("[ManagerSystem] 감독의 격려 연설로 동기부여가 크게 향상되었습니다!")


func _trigger_performance_review():
	"""성과 평가 이벤트"""
	# 최근 성과에 따른 감독 평가
	var recent_performance = _evaluate_recent_performance()
	if recent_performance > 75:
		praise_player("우수한 성과")
	elif recent_performance < 40:
		criticize_player("부진한 성과")


func _apply_natural_relationship_decay():
	"""시간 경과에 따른 자연스러운 관계 변화"""
	var target_relationship = 50.0  # 중립 지점
	var current_relationship = manager_data.relationship

	# 중립점으로 서서히 수렴 (매우 천천히)
	var decay_rate = 0.5
	var difference = target_relationship - current_relationship
	var adjustment = difference * decay_rate * 0.01  # 1% 조정

	if abs(adjustment) > 0.1:  # 미미한 변화는 무시
		_change_relationship(adjustment)


# 헬퍼 함수들
func _get_recent_goals_scored() -> int:
	# TODO: 실제 경기 시스템과 연동
	return randi() % 3  # 임시


func _get_recent_goals_conceded() -> int:
	# TODO: 실제 경기 시스템과 연동
	return randi() % 4  # 임시


func _evaluate_recent_performance() -> int:
	# TODO: 실제 성과 시스템과 연동
	return randi() % 100  # 임시


# 공개 API
func get_manager_info() -> Dictionary:
	return manager_data.duplicate()


func get_disciplinary_record() -> Dictionary:
	return disciplinary_record.duplicate()


func get_relationship() -> float:
	return manager_data.relationship


func is_player_benched() -> bool:
	return disciplinary_record.warnings >= 4


func is_expulsion_risk() -> bool:
	return disciplinary_record.warnings >= 5


func reset_season():
	"""새 시즌 시작 시 초기화"""
	disciplinary_record.warnings = 0
	disciplinary_record.bench_count = 0
	disciplinary_record.last_incident_week = 0
	print("[ManagerSystem] 새 시즌 시작 - 징계 기록 초기화")


# 테스트 함수
func test_manager_system():
	"""감독 시스템 테스트"""
	print("=== Manager System Test ===")

	# 여러 주간 훈련 결정 테스트
	print("\n1. 주간 훈련 결정 테스트:")
	for i in range(5):
		var training = decide_weekly_training()
		var names = ["⚡PACE", "💪POWER", "⚽TECHNICAL", "🎯SHOOTING", "🔄PASSING", "🛡️DEFENDING"]
		print("Week %d: %s" % [i + 1, names[training]])

	# 징계 시스템 테스트
	print("\n2. 징계 시스템 테스트:")
	for i in range(6):
		var result = handle_training_absence()
		print("Warning %d: %s (Severity: %d)" % [i + 1, result.message, result.severity])

	# 관계 시스템 테스트
	print("\n3. 관계 시스템 테스트:")
	praise_player("훌륭한 골")
	criticize_player("실수 반복")

	print("✅ Manager system test completed")
