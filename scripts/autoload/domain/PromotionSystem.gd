extends Node

## PromotionSystem.gd
## 승격 평가 시스템 (U18 → B-Team → A-Team)
## 작성일: 2025-10-24
## 버전: 1.0

# ============================================
# 시그널
# ============================================

## 승격 평가 시작 시 발생
signal promotion_evaluation_started(tier: String)

## 승격 평가 완료 시 발생
signal promotion_evaluation_completed(tier: String, result: Dictionary)

## 승격 성공 시 발생
signal promotion_succeeded(tier: String)

## 승격 실패 시 발생
signal promotion_failed(tier: String, missing_requirements: Array)

# ============================================
# 상태 변수
# ============================================

## 현재 소속 (U18/B-Team/A-Team)
var current_tier: String = "U18"

## 승격 평가 이력
var evaluation_history: Array[Dictionary] = []

## 다음 평가 주차
var next_evaluation_week: int = 85

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	print("[PromotionSystem] Initialized")
	print("[PromotionSystem] Current Tier: ", current_tier)

	# DateManager 시그널 연결
	_connect_to_date_manager()


func _connect_to_date_manager() -> void:
	"""DateManager 시그널에 연결"""
	if not DateManager:
		print("[PromotionSystem] WARNING: DateManager not found")
		return

	# week_started: 주차 변경 시 자동 평가 체크
	if not DateManager.week_started.is_connected(_on_week_started):
		DateManager.week_started.connect(_on_week_started)

	print("[PromotionSystem] Connected to DateManager signals")


func _on_week_started(week_number: int, _week_schedule) -> void:
	"""
	DateManager week_started 시그널 핸들러
	매주 시작 시 자동 평가 체크
	"""
	print("[PromotionSystem] Week ", week_number, " started - checking auto evaluation")
	check_auto_evaluation()


# ============================================
# 승격 평가
# ============================================


func evaluate_promotion(target_tier: String) -> Dictionary:
	"""
	승격 평가 실행

	@param target_tier: "B-Team" 또는 "A-Team"
	@return: 평가 결과 Dictionary
	"""

	promotion_evaluation_started.emit(target_tier)

	print("[PromotionSystem] Evaluating promotion to ", target_tier)

	# PositionManager를 통한 평가
	var player_data = _get_player_data()
	var position = PlayerData.position

	var evaluation = PositionManager.evaluate_for_promotion(player_data, target_tier, position)

	# 평가 결과 저장
	var result = {
		"week": GameManager.current_week,
		"target_tier": target_tier,
		"success": evaluation["eligible"],
		"ca": evaluation["current_ca"],
		"position_rating": evaluation["current_rating"],
		"match_average": evaluation["current_match_avg"],
		"missing_requirements": evaluation["missing_requirements"]
	}

	evaluation_history.append(result)

	# 성공/실패 시그널 발생
	if result["success"]:
		_on_promotion_succeeded(target_tier)
	else:
		_on_promotion_failed(target_tier, evaluation["missing_requirements"])

	promotion_evaluation_completed.emit(target_tier, result)

	return result


func _get_player_data() -> Dictionary:
	"""현재 선수 데이터 조회"""
	return {
		"ca": PlayerData.ca,
		"pa": PlayerData.pa,
		"condition": PlayerData.condition,
		"position": PlayerData.position,
		"technical": PlayerData.technical_stats,
		"mental": PlayerData.mental_stats,
		"physical": PlayerData.physical_stats,
		"goalkeeper": PlayerData.goalkeeper_stats,
		"reputation": 0  # TODO: Reputation 시스템 구현 후 추가
	}


func _on_promotion_succeeded(tier: String) -> void:
	"""승격 성공 처리"""
	current_tier = tier
	promotion_succeeded.emit(tier)

	print("[PromotionSystem] Promotion succeeded! New tier: ", tier)

	# 승격 축하 이벤트 트리거
	_trigger_promotion_event(tier)

	# 다음 평가 주차 설정
	_set_next_evaluation_week(tier)


func _on_promotion_failed(tier: String, missing: Array) -> void:
	"""승격 실패 처리"""
	promotion_failed.emit(tier, missing)

	print("[PromotionSystem] Promotion failed. Missing:")
	for req in missing:
		print("  - ", req)

	# 재평가 일정 설정 (10주 후)
	next_evaluation_week = GameManager.current_week + 10


func _trigger_promotion_event(_tier: String) -> void:
	"""승격 성공 이벤트 트리거"""
	# TODO: EventManager와 연동하여 승격 이벤트 발동
	# Week 2에 구현 예정
	pass


func _set_next_evaluation_week(tier: String) -> void:
	"""다음 평가 주차 설정"""
	match tier:
		"B-Team":
			# B팀 승격 성공 → A팀 평가는 Week 135
			next_evaluation_week = 135

		"A-Team":
			# A팀 승격 성공 → 더 이상 평가 없음
			next_evaluation_week = -1

		_:
			next_evaluation_week = -1


# ============================================
# 자동 평가 스케줄
# ============================================


func check_auto_evaluation() -> void:
	"""
	자동 평가 체크 (DateManager에서 주차 변경 시 호출)
	"""

	var current_week = GameManager.current_week

	# 다음 평가 주차가 아니면 스킵
	if current_week != next_evaluation_week:
		return

	# 현재 티어에 따라 목표 티어 결정
	var target_tier = ""

	match current_tier:
		"U18":
			target_tier = "B-Team"

		"B-Team":
			target_tier = "A-Team"

		"A-Team":
			# 이미 최상위 티어
			return

	# 자동 평가 실행
	print("[PromotionSystem] Auto evaluation triggered at Week ", current_week)
	evaluate_promotion(target_tier)


# ============================================
# 수동 평가 요청
# ============================================


func request_early_evaluation(target_tier: String) -> Dictionary:
	"""
	조기 평가 요청 (플레이어가 직접 요청)

	@param target_tier: "B-Team" 또는 "A-Team"
	@return: 평가 결과
	"""

	# 현재 티어 체크
	if not _can_request_evaluation(target_tier):
		return {"success": false, "error": "Cannot request evaluation for " + target_tier}

	return evaluate_promotion(target_tier)


func _can_request_evaluation(target_tier: String) -> bool:
	"""평가 요청 가능 여부 체크"""
	match current_tier:
		"U18":
			return target_tier == "B-Team"

		"B-Team":
			return target_tier == "A-Team"

		"A-Team":
			return false

		_:
			return false


# ============================================
# 조회 메서드
# ============================================


func get_current_tier() -> String:
	"""현재 소속 티어 반환"""
	return current_tier


func get_next_evaluation_week() -> int:
	"""다음 평가 예정 주차 반환"""
	return next_evaluation_week


func get_evaluation_history() -> Array[Dictionary]:
	"""평가 이력 반환"""
	return evaluation_history


func get_last_evaluation() -> Dictionary:
	"""마지막 평가 결과 반환"""
	if evaluation_history.is_empty():
		return {}

	return evaluation_history[-1]


func get_promotion_progress(target_tier: String) -> Dictionary:
	"""
	승격 진행 상황 반환

	@param target_tier: "B-Team" 또는 "A-Team"
	@return: 진행 상황 Dictionary
	"""

	var player_data = _get_player_data()
	var position = PlayerData.position

	var evaluation = PositionManager.evaluate_for_promotion(player_data, target_tier, position)

	return {
		"ca_progress": float(evaluation["current_ca"]) / float(evaluation["required_ca"]),
		"rating_progress": evaluation["current_rating"] / evaluation["required_rating"],
		"match_avg_progress": evaluation["current_match_avg"] / evaluation["required_match_avg"],
		"overall_progress": _calculate_overall_progress(evaluation),
		"ready": evaluation["eligible"]
	}


func _calculate_overall_progress(evaluation: Dictionary) -> float:
	"""전체 진행률 계산 (0.0 ~ 1.0)"""
	var ca_prog = clamp(float(evaluation["current_ca"]) / float(evaluation["required_ca"]), 0.0, 1.0)
	var rating_prog = clamp(evaluation["current_rating"] / evaluation["required_rating"], 0.0, 1.0)
	var match_prog = clamp(evaluation["current_match_avg"] / evaluation["required_match_avg"], 0.0, 1.0)

	return (ca_prog + rating_prog + match_prog) / 3.0


# ============================================
# 세이브/로드
# ============================================


func save_promotion_data() -> Dictionary:
	"""승격 시스템 데이터 저장"""
	return {
		"current_tier": current_tier,
		"next_evaluation_week": next_evaluation_week,
		"evaluation_history": evaluation_history
	}


func load_promotion_data(data: Dictionary) -> void:
	"""승격 시스템 데이터 로드"""
	current_tier = data.get("current_tier", "U18")
	next_evaluation_week = data.get("next_evaluation_week", 85)
	evaluation_history = data.get("evaluation_history", [])

	print("[PromotionSystem] Data loaded. Tier: ", current_tier)


# ============================================
# 디버그
# ============================================


func debug_print_status() -> void:
	"""현재 상태 출력 (디버그용)"""
	print("=== Promotion System Status ===")
	print("Current Tier: ", current_tier)
	print("Next Evaluation: Week ", next_evaluation_week)
	print("Evaluation History: ", evaluation_history.size(), " records")

	if not evaluation_history.is_empty():
		var last = evaluation_history[-1]
		print("\nLast Evaluation:")
		print("  Week: ", last["week"])
		print("  Target: ", last["target_tier"])
		print("  Success: ", last["success"])
		print("  CA: ", last["ca"])
		print("  Rating: %.2f" % last["position_rating"])

	print("===============================")


func debug_force_promotion(tier: String) -> void:
	"""강제 승격 (디버그용)"""
	current_tier = tier
	print("[PromotionSystem] DEBUG: Forced promotion to ", tier)
