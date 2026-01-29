extends Node

# 의무 팀훈련 시스템 - 매주 필수 참석
# 불참 시 감독과 관계 악화, 3회 불참 시 벤치/퇴부

signal team_training_completed(result: Dictionary)
signal warning_issued(warning_count: int, message: String)
signal forced_bench_triggered(reason: String)

enum TeamTrainingType { PACE, POWER, TECHNICAL, SHOOTING, PASSING, DEFENDING }  # ⚡ 스피드 훈련  # 💪 파워 훈련  # ⚽ 기술 훈련  # 🎯 슈팅 훈련  # 🔄 패싱 훈련  # 🛡️ 수비 훈련

enum ParticipationStatus { ATTENDED, SKIPPED, FAILED_DUE_TO_STAMINA }

# 팀훈련별 체력 소모량
const TEAM_TRAINING_STAMINA_COST = {
	TeamTrainingType.PACE: 30,
	TeamTrainingType.POWER: 30,
	TeamTrainingType.TECHNICAL: 25,
	TeamTrainingType.SHOOTING: 25,
	TeamTrainingType.PASSING: 20,
	TeamTrainingType.DEFENDING: 25
}

# 팀훈련 불참 시 페널티
const SKIP_PENALTY = {"coach_relationship": -15, "team_chemistry": -5, "reputation": -3, "warning_count": 1}

# 참석 시 보너스
const ATTENDANCE_BONUS = {"coach_relationship": 5, "team_chemistry": 2, "reputation": 1}

# 경고 메시지
const WARNING_MESSAGES = {
	1: '감독: "다음에는 꼭 참석하거라. 팀이 우선이다."',
	2: '감독: "이런 식으로 하면 벤치에 앉히겠다!"',
	3: '감독: "벤치에서 반성하며 지켜봐라!"',
	4: '감독: "한 번 더 빠지면 퇴부시킨다!"',
	5: '감독: "팀에 헌신하지 못하는 선수는 필요 없다!"'
}

var current_week_training: TeamTrainingType
var player_warnings: int = 0
var is_benched: bool = false
var consecutive_skips: int = 0


func _ready():
	# 매주 랜덤한 팀훈련 타입 선택
	randomize_weekly_training()


func randomize_weekly_training():
	"""감독이 팀훈련 결정 (Coach가 아님)"""
	if ManagerSystem and ManagerSystem.has_method("decide_weekly_training"):
		# Enum 타입 캐스팅 수정
		var training_type = ManagerSystem.decide_weekly_training()
		current_week_training = training_type as TeamTrainingType
	else:
		# 기본값: 랜덤 (ManagerSystem 미구현 시)
		# Enum 타입 캐스팅 수정
		current_week_training = randi() % TeamTrainingType.size() as TeamTrainingType


func get_current_training_info() -> Dictionary:
	"""현재 주 팀훈련 정보 반환"""
	var training_names = {
		TeamTrainingType.PACE: "⚡ PACE Training",
		TeamTrainingType.POWER: "💪 POWER Training",
		TeamTrainingType.TECHNICAL: "⚽ TECHNICAL Training",
		TeamTrainingType.SHOOTING: "🎯 SHOOTING Training",
		TeamTrainingType.PASSING: "🔄 PASSING Training",
		TeamTrainingType.DEFENDING: "🛡️ DEFENDING Training"
	}

	return {
		"type": current_week_training,
		"name": training_names[current_week_training],
		"stamina_cost": TEAM_TRAINING_STAMINA_COST[current_week_training],
		"is_mandatory": true,
		"skip_penalty": SKIP_PENALTY,
		"attendance_bonus": ATTENDANCE_BONUS
	}


func can_attend_training() -> bool:
	"""팀훈련 참석 가능 여부 확인"""
	var current_stamina = PlayerCondition.get_stamina()
	var required_stamina = TEAM_TRAINING_STAMINA_COST[current_week_training]

	return current_stamina >= (required_stamina * 0.3)  # 최소 30% 체력 필요


func calculate_success_rate() -> float:
	"""현재 체력 상태에 따른 팀훈련 성공률"""
	var stamina = PlayerCondition.get_stamina()

	if stamina >= 80:
		return 1.0  # 100% 성공
	elif stamina >= 60:
		return 0.9  # 90% 성공
	elif stamina >= 40:
		return 0.7  # 70% 성공
	elif stamina >= 20:
		return 0.4  # 40% 성공 (위험!)
	else:
		return 0.1  # 10% 성공 (매우 위험!)


func attend_team_training(force_attend: bool = false) -> Dictionary:
	"""팀훈련 참석 처리"""
	var result = {
		"success": false, "status": ParticipationStatus.SKIPPED, "message": "", "effects": {}, "stamina_cost": 0
	}

	# 체력 확인
	if not can_attend_training() and not force_attend:
		result.status = ParticipationStatus.FAILED_DUE_TO_STAMINA
		result.message = "체력이 너무 부족하여 팀훈련에 참석할 수 없습니다!"
		_apply_skip_penalty()
		return result

	# 성공률 계산 및 판정
	var success_rate = calculate_success_rate()
	var is_success = randf() <= success_rate

	var stamina_cost = TEAM_TRAINING_STAMINA_COST[current_week_training]

	if is_success:
		result.success = true
		result.status = ParticipationStatus.ATTENDED
		result.message = "팀훈련을 성공적으로 완료했습니다!"
		result.effects = _apply_attendance_bonus()
		result.stamina_cost = stamina_cost

		# 연속 불참 기록 초기화
		consecutive_skips = 0

		# 능력치 향상 적용
		var stat_gains = _calculate_team_training_gains()
		result.effects["stat_gains"] = stat_gains

		# 체력 소모
		PlayerCondition.consume_stamina(stamina_cost)

	else:
		result.status = ParticipationStatus.FAILED_DUE_TO_STAMINA
		result.message = "체력 부족으로 팀훈련에서 좋지 않은 퍼포먼스를 보였습니다."
		result.effects = {"coach_relationship": -5, "stamina_cost": stamina_cost * 0.7}

		# 여전히 체력은 소모됨 (70%)
		PlayerCondition.consume_stamina(stamina_cost * 0.7)

	team_training_completed.emit(result)
	return result


func skip_team_training() -> Dictionary:
	"""팀훈련 불참 처리"""
	var result = {
		"success": false,
		"status": ParticipationStatus.SKIPPED,
		"message": "팀훈련에 불참했습니다.",
		"effects": _apply_skip_penalty(),
		"stamina_cost": 0
	}

	team_training_completed.emit(result)
	return result


func _apply_attendance_bonus() -> Dictionary:
	"""팀훈련 참석 보너스 적용"""
	var effects = ATTENDANCE_BONUS.duplicate()

	# 관계 시스템이 있다면 적용
	if RelationshipSystem:
		RelationshipSystem.improve_coach_relationship(effects.coach_relationship)
		RelationshipSystem.improve_team_chemistry(effects.team_chemistry)

	# 평판 시스템이 있다면 적용
	if PlayerData.has_method("add_reputation"):
		PlayerData.add_reputation(effects.reputation)

	return effects


func _apply_skip_penalty() -> Dictionary:
	"""팀훈련 불참 페널티 적용 - 감독이 직접 징계"""
	var effects = {}

	# ManagerSystem을 통한 징계 처분
	if ManagerSystem and ManagerSystem.has_method("handle_training_absence"):
		var manager_action = ManagerSystem.handle_training_absence()
		effects = {
			"manager_relationship": manager_action.relationship_penalty,
			"warning_count": manager_action.warning_count,
			"severity": manager_action.severity,
			"consequences": manager_action.consequences,
			"manager_message": manager_action.message
		}

		# 경고 메시지는 감독 메시지로 대체
		warning_issued.emit(manager_action.warning_count, manager_action.message)

		# 벤치/퇴부 처리
		if manager_action.severity >= 4:
			is_benched = true
			forced_bench_triggered.emit(manager_action.message)

		player_warnings = manager_action.warning_count

	else:
		# 기본 페널티 (ManagerSystem 없을 시)
		effects = SKIP_PENALTY.duplicate()
		player_warnings += effects.warning_count
		consecutive_skips += 1

		# 기존 관계 시스템 적용
		if RelationshipSystem:
			RelationshipSystem.worsen_coach_relationship(abs(effects.coach_relationship))
			RelationshipSystem.worsen_team_chemistry(abs(effects.team_chemistry))

		if player_warnings <= WARNING_MESSAGES.size():
			warning_issued.emit(player_warnings, WARNING_MESSAGES[player_warnings])

		_check_bench_condition()

	return effects


func _check_bench_condition():
	"""벤치/퇴부 조건 확인"""
	if player_warnings >= 3 and not is_benched:
		is_benched = true
		forced_bench_triggered.emit("팀훈련 불참으로 인한 벤치 처분")

	elif player_warnings >= 5:
		# 게임 오버 조건
		forced_bench_triggered.emit("반복적인 팀훈련 불참으로 인한 강제 퇴부")


func _calculate_team_training_gains() -> Dictionary:
	"""팀훈련 타입별 능력치 향상 계산 (포지션 특화 적용)"""
	var base_gains = {}
	var player_position = PlayerData.get_position() if PlayerData else "MF"

	# 6각형 능력치 기반 향상
	match current_week_training:
		TeamTrainingType.PACE:
			base_gains = {
				"pace": 10,
				"acceleration": 9,
				"agility": 7,
				"balance": 5,
				"hexagon_pace": 12,
				"hexagon_power": 3,
				"hexagon_technical": 2
			}
		TeamTrainingType.POWER:
			base_gains = {
				"strength": 10,
				"stamina": 8,
				"jumping": 7,
				"heading": 6,
				"hexagon_power": 12,
				"hexagon_defending": 4,
				"hexagon_shooting": 3
			}
		TeamTrainingType.TECHNICAL:
			base_gains = {
				"technique": 10,
				"dribbling": 8,
				"first_touch": 7,
				"flair": 5,
				"hexagon_technical": 12,
				"hexagon_passing": 4,
				"hexagon_shooting": 2
			}
		TeamTrainingType.SHOOTING:
			base_gains = {
				"finishing": 10,
				"long_shots": 8,
				"penalty_taking": 6,
				"composure": 6,
				"hexagon_shooting": 12,
				"hexagon_technical": 3,
				"hexagon_power": 3
			}
		TeamTrainingType.PASSING:
			base_gains = {
				"passing": 10,
				"crossing": 8,
				"vision": 7,
				"decisions": 6,
				"hexagon_passing": 12,
				"hexagon_technical": 4,
				"hexagon_defending": 2
			}
		TeamTrainingType.DEFENDING:
			base_gains = {
				"marking": 10,
				"tackling": 9,
				"positioning": 8,
				"anticipation": 6,
				"hexagon_defending": 12,
				"hexagon_power": 4,
				"hexagon_pace": 2
			}

	# 포지션별 특화 보정 적용
	base_gains = _apply_position_specialization(base_gains, player_position)

	# 컨디션 및 감독 관계 보정
	var condition_multiplier = PlayerCondition.get_condition_multiplier()
	var coach_relationship_bonus = 1.0

	if RelationshipSystem:
		var coach_rel = RelationshipSystem.get_coach_relationship()
		coach_relationship_bonus = 1.0 + (coach_rel / 200.0)  # -50~50 관계를 0.75~1.25 배수로

	# 최종 능력치 적용
	var final_gains = {}
	for skill in base_gains:
		var gain = base_gains[skill] * condition_multiplier * coach_relationship_bonus
		final_gains[skill] = max(1, round(gain))  # 최소 1 보장

		# EnhancedPlayerData에 능력치 적용
		if EnhancedPlayerData and EnhancedPlayerData.has_method("add_skill"):
			EnhancedPlayerData.add_skill(skill, final_gains[skill])

	return final_gains


func _apply_position_specialization(base_gains: Dictionary, position: String) -> Dictionary:
	"""포지션별 훈련 효과 특화 적용"""
	var position_multipliers = {
		"FW":
		{
			"PACE": {"pace": 1.2, "acceleration": 1.3},
			"SHOOTING": {"finishing": 1.4, "composure": 1.2},
			"DEFENDING": {"all": 0.6}  # 공격수는 수비훈련 효과 낮음
		},
		"MF": {"PASSING": {"passing": 1.3, "vision": 1.2}, "TECHNICAL": {"technique": 1.2, "dribbling": 1.1}},
		"DF":
		{
			"DEFENDING": {"marking": 1.4, "tackling": 1.3},
			"POWER": {"strength": 1.2, "heading": 1.3},
			"SHOOTING": {"all": 0.4}  # 수비수는 슈팅훈련 효과 낮음
		},
		"GK":  # 골키퍼 전용 능력 포함  # 핸들링 등  # 골키퍼는 슈팅 거의 안함  # 골키퍼는 스피드 덜 중요
		{"DEFENDING": {"all": 1.5}, "TECHNICAL": {"technique": 1.3}, "SHOOTING": {"all": 0.1}, "PACE": {"all": 0.3}}
	}

	var training_name = TeamTrainingType.keys()[current_week_training]
	var position_data = position_multipliers.get(position, {})
	var training_multipliers = position_data.get(training_name, {})

	var specialized_gains = base_gains.duplicate()
	for skill in specialized_gains:
		var multiplier = training_multipliers.get(skill, training_multipliers.get("all", 1.0))
		specialized_gains[skill] = base_gains[skill] * multiplier

	return specialized_gains


func get_warning_status() -> Dictionary:
	"""현재 경고 상태 반환"""
	return {
		"warning_count": player_warnings,
		"is_benched": is_benched,
		"consecutive_skips": consecutive_skips,
		"next_penalty": WARNING_MESSAGES.get(player_warnings + 1, "퇴부 위험!")
	}


func reset_warnings():
	"""경고 초기화 (새 학년 시작 시)"""
	player_warnings = 0
	consecutive_skips = 0
	is_benched = false


func get_weekly_training_schedule() -> Array:
	"""주간 팀훈련 스케줄 반환 (향후 확장용)"""
	var schedule = []
	for i in range(7):  # 7주 미리보기
		var training_type = (current_week_training + i) % TeamTrainingType.size() as TeamTrainingType
		schedule.append(
			{"week": GameManager.current_week + i, "type": training_type, "name": get_training_name(training_type)}
		)
	return schedule


func get_training_name(type: TeamTrainingType) -> String:
	"""팀훈련 타입별 이름 반환"""
	var names = {
		TeamTrainingType.PACE: "⚡ PACE Training",
		TeamTrainingType.POWER: "💪 POWER Training",
		TeamTrainingType.TECHNICAL: "⚽ TECHNICAL Training",
		TeamTrainingType.SHOOTING: "🎯 SHOOTING Training",
		TeamTrainingType.PASSING: "🔄 PASSING Training",
		TeamTrainingType.DEFENDING: "🛡️ DEFENDING Training"
	}
	return names[type]


# 매주 호출되는 함수 (GameManager에서 호출)
func on_week_start():
	"""새로운 주 시작 시 호출"""
	randomize_weekly_training()

	# 감독의 훈련 계획 공지
	if CoachSystem and CoachSystem.has_method("announce_weekly_training"):
		CoachSystem.announce_weekly_training(current_week_training)


func on_week_end():
	"""주 종료 시 호출"""
	# 팀훈련 미참석 체크 및 자동 페널티 적용
	pass
