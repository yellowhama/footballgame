extends Node
# PlayerCondition - 플레이어 컨디션 & 체력 관리 시스템

signal stamina_changed(new_stamina: int)
@warning_ignore("unused_signal")
signal condition_changed(new_condition: int)  # TODO: Implement condition change emission
signal fatigue_warning(stamina: int)

enum ConditionLevel { PERFECT = 5, GOOD = 4, NORMAL = 3, POOR = 2, TERRIBLE = 1 }

# 컨디션 시스템
var current_condition: ConditionLevel = ConditionLevel.NORMAL
var condition_trend: int = 0  # -1: 하락, 0: 유지, 1: 상승

# 체력 시스템 (우마무스메 스타일)
var current_stamina: int = 100
var max_stamina: int = 100
var weekly_stamina_usage: int = 0  # 주간 체력 사용량 추적

# 훈련별 체력 소모량
const TRAINING_STAMINA_COST = {
	# 팀훈련 (의무)
	"team_training_pace": 30,
	"team_training_power": 30,
	"team_training_technical": 25,
	"team_training_shooting": 25,
	"team_training_passing": 25,
	"team_training_defending": 30,
	# 개인훈련 (선택)
	"individual_physical": 15,
	"individual_technical": 10,
	"individual_tactical": 10,
	"individual_mental": 5,
	"individual_match_prep": 10,
	# 특별 활동
	"match": 40,  # 경기
	"rest": -20,  # 휴식 (회복)
	"light_recovery": -10  # 가벼운 회복훈련
}


func _ready():
	print("[PlayerCondition] Initialized")


func _get_condition_level_from_int(condition_int: int) -> ConditionLevel:
	# 정수 값을 ConditionLevel enum으로 변환
	match condition_int:
		1:
			return ConditionLevel.TERRIBLE
		2:
			return ConditionLevel.POOR
		3:
			return ConditionLevel.NORMAL
		4:
			return ConditionLevel.GOOD
		5:
			return ConditionLevel.PERFECT
		_:
			return ConditionLevel.NORMAL


func get_condition_level() -> ConditionLevel:
	# 현재 컨디션 레벨 반환
	return current_condition


func get_condition_text() -> String:
	# 컨디션 텍스트 반환
	match current_condition:
		ConditionLevel.PERFECT:
			return "Perfect"
		ConditionLevel.GOOD:
			return "Good"
		ConditionLevel.NORMAL:
			return "Normal"
		ConditionLevel.POOR:
			return "Poor"
		ConditionLevel.TERRIBLE:
			return "Terrible"
		_:
			return "Normal"


func get_condition_color() -> Color:
	# 컨디션 색상 반환
	match current_condition:
		ConditionLevel.PERFECT:
			return Color(0.2, 0.6, 1.0)  # 파랑
		ConditionLevel.GOOD:
			return Color(0.2, 0.8, 0.2)  # 초록
		ConditionLevel.NORMAL:
			return Color(0.9, 0.9, 0.2)  # 노랑
		ConditionLevel.POOR:
			return Color(0.9, 0.5, 0.2)  # 주황
		ConditionLevel.TERRIBLE:
			return Color(0.8, 0.2, 0.2)  # 빨강
		_:
			return Color(0.9, 0.9, 0.2)  # 기본 노랑


func set_condition(level: ConditionLevel):
	# 컨디션 설정
	current_condition = level
	print("[PlayerCondition] Condition set to: ", get_condition_text())


func improve_condition():
	# 컨디션 개선
	if current_condition < ConditionLevel.PERFECT:
		current_condition = _get_condition_level_from_int(current_condition + 1)
		condition_trend = 1
		print("[PlayerCondition] Condition improved to: ", get_condition_text())


func worsen_condition():
	# 컨디션 악화
	if current_condition > ConditionLevel.TERRIBLE:
		current_condition = _get_condition_level_from_int(current_condition - 1)
		condition_trend = -1
		print("[PlayerCondition] Condition worsened to: ", get_condition_text())


func get_training_modifier() -> float:
	# 훈련 효과 수정자 반환
	match current_condition:
		ConditionLevel.PERFECT:
			return 1.2
		ConditionLevel.GOOD:
			return 1.1
		ConditionLevel.NORMAL:
			return 1.0
		ConditionLevel.POOR:
			return 0.8
		ConditionLevel.TERRIBLE:
			return 0.6
		_:
			return 1.0


func update_daily_condition(training_intensity: float, consecutive_days: int):
	# 일일 컨디션 업데이트
	# 훈련 강도에 따른 컨디션 변화
	if training_intensity > 0.8:  # 고강도 훈련
		if consecutive_days > 3:
			worsen_condition()
		else:
			# 컨디션 유지
			condition_trend = 0
	elif training_intensity < 0.3:  # 저강도 훈련 또는 휴식
		if consecutive_days > 2:
			improve_condition()
		else:
			# 컨디션 유지
			condition_trend = 0
	else:  # 중강도 훈련
		# 컨디션 유지
		condition_trend = 0

	print("[PlayerCondition] Daily condition updated. Current: ", get_condition_text())


# ==================== 체력 시스템 ====================


func on_week_start():
	"""매주 월요일 체력 리셋 (우마무스메 스타일)"""
	current_stamina = max_stamina
	weekly_stamina_usage = 0
	stamina_changed.emit(current_stamina)
	print("[PlayerCondition] 주간 체력 리셋: ", current_stamina)


# consume_stamina 함수는 아래에 오버로드 버전으로 대체됨


func restore_stamina(amount: int):
	"""체력 회복"""
	var old_stamina = current_stamina
	current_stamina = min(max_stamina, current_stamina + amount)
	stamina_changed.emit(current_stamina)
	print("[PlayerCondition] 체력 회복: %d → %d" % [old_stamina, current_stamina])


func get_stamina_percentage() -> float:
	"""체력 퍼센트 반환"""
	return float(current_stamina) / float(max_stamina)


func get_stamina_status() -> String:
	"""체력 상태 텍스트"""
	var percentage = get_stamina_percentage()
	if percentage >= 0.8:
		return "활력충만"
	elif percentage >= 0.6:
		return "양호"
	elif percentage >= 0.4:
		return "보통"
	elif percentage >= 0.2:
		return "피곤"
	else:
		return "탈진"


func get_stamina_color() -> Color:
	"""체력 상태별 색상"""
	var percentage = get_stamina_percentage()
	if percentage >= 0.8:
		return Color(0.2, 0.8, 0.2)  # 초록
	elif percentage >= 0.6:
		return Color(0.5, 0.8, 0.2)  # 연두
	elif percentage >= 0.4:
		return Color(0.9, 0.9, 0.2)  # 노랑
	elif percentage >= 0.2:
		return Color(0.9, 0.5, 0.2)  # 주황
	else:
		return Color(0.8, 0.2, 0.2)  # 빨강


func _update_condition_by_stamina():
	"""체력에 따른 컨디션 자동 조정"""
	var percentage = get_stamina_percentage()

	# 체력이 너무 낮으면 컨디션 하락
	if percentage < 0.2 and current_condition > ConditionLevel.POOR:
		worsen_condition()
	# 체력이 충분하면 컨디션 개선 기회
	elif percentage > 0.8 and current_condition < ConditionLevel.GOOD:
		if randf() > 0.7:  # 30% 확률로 컨디션 개선
			improve_condition()


func get_training_efficiency() -> float:
	"""체력과 컨디션을 고려한 훈련 효율"""
	var stamina_modifier = 1.0
	var percentage = get_stamina_percentage()

	if percentage < 0.2:
		stamina_modifier = 0.5  # 체력 부족시 효율 50%
	elif percentage < 0.4:
		stamina_modifier = 0.7
	elif percentage < 0.6:
		stamina_modifier = 0.85

	var condition_modifier = get_training_modifier()

	return stamina_modifier * condition_modifier


func should_rest() -> bool:
	"""휴식이 필요한지 판단"""
	return current_stamina < 40 or current_condition <= ConditionLevel.POOR


func get_recommended_activity() -> String:
	"""체력 상태에 따른 추천 활동"""
	if current_stamina >= 70:
		return "고강도 훈련 가능"
	elif current_stamina >= 40:
		return "일반 훈련 권장"
	elif current_stamina >= 20:
		return "가벼운 훈련 또는 휴식 권장"
	else:
		return "휴식 필수"


func get_stamina() -> int:
	"""현재 체력 반환 (MandatoryTeamTrainingManager용)"""
	return current_stamina


func get_condition_multiplier() -> float:
	"""컨디션에 따른 훈련 효과 배수 반환 (MandatoryTeamTrainingManager용)"""
	return get_training_modifier()


func consume_stamina(amount) -> bool:
	"""체력 소모 - 오버로드 (int 또는 String 받음)"""
	if amount is int:
		# int로 직접 체력 소모
		if current_stamina < amount:
			print("[PlayerCondition] 체력 부족! 필요: %d, 현재: %d" % [amount, current_stamina])
			fatigue_warning.emit(current_stamina)
			return false

		current_stamina -= amount
		weekly_stamina_usage += amount
		stamina_changed.emit(current_stamina)

		if current_stamina < 30:
			fatigue_warning.emit(current_stamina)
			print("[PlayerCondition] ⚠️ 체력 경고: %d/100" % current_stamina)

		_update_condition_by_stamina()
		return true
	elif amount is String:
		# 기존 String 기반 처리
		var cost = TRAINING_STAMINA_COST.get(amount, 0)

		if cost < 0:
			restore_stamina(-cost)
			return true

		if current_stamina < cost:
			print("[PlayerCondition] 체력 부족! 필요: %d, 현재: %d" % [cost, current_stamina])
			fatigue_warning.emit(current_stamina)
			return false

		current_stamina -= cost
		weekly_stamina_usage += cost
		stamina_changed.emit(current_stamina)

		if current_stamina < 30:
			fatigue_warning.emit(current_stamina)
			print("[PlayerCondition] ⚠️ 체력 경고: %d/100" % current_stamina)

		_update_condition_by_stamina()
		return true
	else:
		print("[PlayerCondition] ERROR: Invalid consume_stamina parameter type")
		return false
