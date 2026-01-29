extends Node
## GameManager - 게임 상태 관리 총괄 시스템
##
## Phase 1 Integration: TurnManager (48-turn system) overlays the 156-week calendar.
## GameManager handles weekly progression; TurnManager provides turn-based decision points.

signal week_advanced(week: int, year: int)
signal game_state_changed(state: String)
signal season_completed(year: int)
signal team_training_executed(result: Dictionary)  # For UI to react to team training
signal turn_decision_required(turn_info: Dictionary, actions: Array)  # NEW: Turn-based decision
signal events_check_completed(events: Array)  # Emitted when all events have been triggered

var current_week: int = 1
var current_year: int = 1
var game_state: String = "playing"  # playing, paused, ended
var last_result: Dictionary = {}
var _friday_sim_in_progress: bool = false
var _week_flow_running: bool = false

# 3년간 진행 (156주)
var total_weeks: int = 156  # 3년 × 52주
var completed_weeks: int = 0

# 주간 진행 통계
var weekly_stats: Array = []

# 주간 활동 상태 추적 (사용자 요청 기능)
var weekly_personal_activity_done: bool = false
var weekly_activity_type: String = ""  # "training", "rest", "go_out"

# Phase 6B: DateManager Integration (replaces TurnManager)
var date_manager: Node = null
var waiting_for_turn_decision: bool = false  # TRUE when awaiting player's turn choice

# Phase 2: CoachCardSystem Integration
var coach_card_system: Node = null

# 시즌 시스템 (calendar.yaml 기반)
enum Period { SEASON, CAMP, VACATION }  # 시즌 중 - 팀훈련 O  # 합숙 - 팀훈련 O  # 방학 - 팀훈련 X

# 합숙 주차 정의 (year -> weeks[])
const CAMP_WEEKS = {
	1: [16, 17, 18, 19, 40, 41, 42, 43, 44, 45], 2: [16, 17, 18, 19, 20, 40, 41, 42, 43, 44, 45], 3: [40, 41, 42]  # 여름 합숙 + 겨울 전지훈련  # 엘리트 여름 합숙 + U19 선발 합숙  # 졸업 전 마지막 합숙
}

# 방학 주차 정의 (단, 합숙 주간은 제외)
# 간단한 정의: 여름 방학은 주차 27-39 (단, 합숙 제외), 겨울 방학은 48-52, 1-2
const VACATION_WEEKS = {
	1: [1, 2, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 48, 49, 50, 51, 52],
	2: [1, 2, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 48, 49, 50, 51, 52],
	3: [1, 2, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 48, 49, 50, 51, 52]
}


func _ready():
	print("[GameManager] Initialized - Football Player Development Game")
	print("[GameManager] Total game duration: 3 years (156 weeks)")

	# Phase 6B: Initialize DateManager
	_initialize_date_manager()

	# Phase 2: Initialize CoachCardSystem
	_initialize_coach_card_system()

	# TrainingManager 신호 연결 (핵심!)
	if TrainingManager:
		if not TrainingManager.training_completed.is_connected(_on_training_completed):
			TrainingManager.training_completed.connect(_on_training_completed)
		if not TrainingManager.training_started.is_connected(_on_training_started):
			TrainingManager.training_started.connect(_on_training_started)
		if not TrainingManager.training_failed.is_connected(_on_training_failed):
			TrainingManager.training_failed.connect(_on_training_failed)
		print("[GameManager] ✅ Connected to TrainingManager training signals")
	else:
		print("[GameManager] ❌ TrainingManager not found!")

	# Event Bus 구독 - TrainingCard 이벤트 처리
	EventBus.subscribe("training_selected", _on_training_card_selected)
	EventBus.subscribe("training_hovered", _on_training_card_hovered)

	# Event Bus 구독 - Bridge 이벤트 처리
	EventBus.subscribe("bridge_training_completed", _on_bridge_training_completed)
	EventBus.subscribe("bridge_week_completed", _on_bridge_week_completed)
	EventBus.subscribe("bridge_match_completed", _on_bridge_match_completed)
	EventBus.subscribe("bridge_state_changed", _on_bridge_state_changed)

	# Event Bus 구독 - 추가 핵심 이벤트들
	EventBus.subscribe("player_fatigue_changed", _on_player_fatigue_changed)
	EventBus.subscribe("player_performance_updated", _on_player_performance_updated)

	print("[GameManager] ✅ Event Bus 구독 완료 - 모든 핵심 이벤트들")

	# MatchSimulationManager 신호 연결 (안전하게 확인)
	await get_tree().process_frame  # 다른 autoload들이 초기화될 때까지 대기

	var match_sim_manager = get_node_or_null("/root/MatchSimulationManager")
	if match_sim_manager:
		match_sim_manager.match_completed.connect(_on_match_completed)
		print("[GameManager] ✅ Connected to MatchSimulationManager.match_completed")
	else:
		print("[GameManager] ❌ MatchSimulationManager not found!")

	# GraduationManager 신호 연결 (3년차 완료 처리)
	var graduation_manager = get_node_or_null("/root/GraduationManager")
	if graduation_manager:
		graduation_manager.graduation_completed.connect(_on_graduation_completed)
		print("[GameManager] ✅ Connected to GraduationManager.graduation_completed")
	else:
		print("[GameManager] ❌ GraduationManager not found!")


func _initialize_date_manager():
	"""Phase 6B: Initialize and connect DateManager"""
	# Get DateManager singleton (autoload)
	date_manager = get_node_or_null("/root/DateManager")
	if not date_manager:
		push_error("[GameManager] Failed to get DateManager autoload!")
		return

	print("[GameManager] ✅ DateManager connected (autoload)")

	# Connect DateManager signals
	date_manager.day_started.connect(_on_day_started)
	date_manager.week_started.connect(_on_week_started)
	date_manager.turn_decision_required.connect(_on_turn_decision_required)
	date_manager.turn_completed.connect(_on_turn_completed)
	date_manager.season_ended.connect(_on_season_ended)
	date_manager.academy_completed.connect(_on_academy_completed)

	print("[GameManager] ✅ DateManager signals connected")

	# Sync initial state from DateManager
	current_week = date_manager.current_week
	current_year = date_manager.current_year
	print("[GameManager] ✅ Synced initial state: Year %d Week %d" % [current_year, current_week])


func _initialize_coach_card_system():
	"""Phase 2: Initialize CoachCardSystem"""
	# Get CoachCardSystem singleton
	coach_card_system = get_node_or_null("/root/CoachCardSystem")
	if not coach_card_system:
		push_warning("[GameManager] CoachCardSystem not found as autoload (Phase 2 not active yet)")
		return

	print("[GameManager] ✅ CoachCardSystem connected")

	# Connect CoachCardSystem signals (optional for feedback)
	coach_card_system.trust_changed.connect(_on_coach_trust_changed)
	coach_card_system.rainbow_training_triggered.connect(_on_rainbow_training_triggered)

	print("[GameManager] ✅ CoachCardSystem signals connected")


## Phase 6B: DateManager Signal Handlers


func _on_day_started(day: int, day_of_week: int):
	"""Handle day start from DateManager"""
	var day_name = ["월", "화", "수", "목", "금", "토", "일"][day_of_week]
	print("[GameManager] 🌅 Day %d started (%s) - Year %d Week %d" % [day, day_name, current_year, current_week])


func _on_week_started(week: int, _week_schedule) -> void:
	"""Handle week start from DateManager"""
	var year = date_manager.current_year if date_manager else current_year
	print("[GameManager] 📅 Week %d Year %d started" % [week, year])

	# Sync local state
	current_week = week
	current_year = year

	# Process weekly flow (existing GameManager logic)
	await _process_weekly_flow()

	# Emit existing week_advanced signal for backward compatibility
	week_advanced.emit(current_week, current_year)


func _on_turn_completed(turn_type: String, turn_data: Dictionary):
	"""Handle turn completion from DateManager"""
	print("[GameManager] ✅ Turn completed: %s" % turn_type)
	waiting_for_turn_decision = false

	# Store turn results
	set_last_result(turn_data)


func _on_turn_decision_required(turn_type: String, turn_data: Dictionary):
	"""Handle turn decision required - emit signal to UI for player choice"""
	print("[GameManager] 🤔 Turn decision required: %s (Day %d)" % [turn_type, date_manager.current_day])

	waiting_for_turn_decision = true

	# Build available actions based on turn type
	var available_actions = _get_available_actions_for_turn(turn_type, turn_data)

	# Emit decision signal with turn info
	turn_decision_required.emit(turn_data, available_actions)


func _on_season_ended(year: int):
	"""Handle season end from DateManager"""
	print("[GameManager] 🎉 Year %d completed!" % year)

	# Emit existing season_completed signal for backward compatibility
	season_completed.emit(year)


func _on_academy_completed():
	"""Handle academy completion from DateManager"""
	print("[GameManager] 🏆 Academy journey completed! (798 days)")
	end_game()


func _get_available_actions_for_turn(turn_type: String, _turn_data: Dictionary) -> Array:
	"""Get available actions based on turn type"""
	var actions = []

	match turn_type:
		"Training":
			actions = [
				{"id": "training_technical", "label": "기술 훈련", "category": 0},
				{"id": "training_physical", "label": "체력 훈련", "category": 1},
				{"id": "training_mental", "label": "멘탈 훈련", "category": 2},
				{"id": "rest", "label": "휴식", "category": -1}
			]
		"Match_Prep":
			actions = [
				{"id": "prep_tactical", "label": "전술 훈련", "category": 3},
				{"id": "prep_light", "label": "가벼운 훈련", "category": 0},
				{"id": "rest", "label": "휴식", "category": -1}
			]
		"Recovery":
			actions = [
				{"id": "recovery_active", "label": "적극적 회복", "category": -1},
				{"id": "rest", "label": "완전 휴식", "category": -1}
			]
		_:
			# Default actions for unknown turn types
			actions = [{"id": "continue", "label": "계속", "category": -1}]

	return actions


## Phase 2: CoachCardSystem Signal Handlers


func _on_coach_trust_changed(coach: Resource, old_trust: int, new_trust: int):
	"""Handle coach trust changes"""
	print("[GameManager] 💖 Coach %s trust: %d → %d" % [coach.coach_name, old_trust, new_trust])


func _on_rainbow_training_triggered(coach: Resource, category: int):
	"""Handle rainbow training activation"""
	print("[GameManager] 🌈 RAINBOW TRAINING! Coach: %s, Category: %d" % [coach.coach_name, category])


## Phase 6B: Turn Decision Handling (DateManager)


func make_turn_decision(action_id: String, data: Dictionary = {}):
	"""Player makes a decision for the current turn"""
	if not waiting_for_turn_decision:
		push_error("[GameManager] No active turn decision!")
		return

	if not date_manager:
		push_error("[GameManager] DateManager not initialized!")
		return

	print("[GameManager] 📝 Turn decision: %s (Day %d)" % [action_id, date_manager.current_day])

	# Process the decision
	var results = _process_turn_action(action_id, data)

	# Complete the turn through DateManager
	date_manager.complete_current_turn(results)


func _process_turn_action(action_id: String, _data: Dictionary) -> Dictionary:
	"""Process a turn action and return results"""
	var results = {
		"action_id": action_id,
		"success": true,
		"turn_type": date_manager.current_turn_type if date_manager else "",
		"effects": {}
	}

	# Process based on action type
	var training_category: int = -1  # Track training category for coach trust
	match action_id:
		"training_technical":
			results.effects = {"technical_focus": true, "ca_gain": 2}
			training_category = 0  # CoachCard.Category.TECHNICAL
			print("[GameManager] Technical training selected")

		"training_physical":
			results.effects = {"physical_focus": true, "ca_gain": 2}
			training_category = 1  # CoachCard.Category.PHYSICAL
			print("[GameManager] Physical training selected")

		"training_mental":
			results.effects = {"mental_focus": true, "ca_gain": 2}
			training_category = 2  # CoachCard.Category.MENTAL
			print("[GameManager] Mental training selected")

		"rest":
			results.effects = {"stamina_recovery": 30}
			print("[GameManager] Rest selected")

		"camp_intensive":
			results.effects = {"intensive_training": true, "ca_gain": 5, "stamina_cost": 40}
			training_category = 0  # Default to TECHNICAL for camp
			print("[GameManager] Intensive camp training")

		"camp_balanced":
			results.effects = {"balanced_training": true, "ca_gain": 3, "stamina_cost": 20}
			training_category = 0  # Default to TECHNICAL for camp
			print("[GameManager] Balanced camp training")

		_:
			print("[GameManager] Unknown action: ", action_id)
			results.success = false

	# Phase 2: Increase coach trust for training actions
	if coach_card_system and training_category >= 0:
		var deck = coach_card_system.get_deck_coaches()
		for coach in deck:
			coach_card_system.increase_trust(coach, training_category, true)

	# Note: DateManager handles day/week progression automatically
	# No need to manually advance weeks here

	return results


# NOTE: Week advancement now handled by DateManager via _on_week_started() signal
# _advance_week_internal() removed - DateManager manages time progression


func _on_training_started(event: Dictionary) -> void:
	if EventBus:
		EventBus.emit("training_started", event)


func _on_training_failed(event: Dictionary) -> void:
	if EventBus:
		EventBus.emit("training_failed", event)


func _on_training_completed(event: Dictionary):
	"""�Ʒ� �Ϸ� �� ���� ����"""
	print("[GameManager] Training completed, advancing week...")
	var result: Dictionary = event.get("result", {})
	set_last_result(result)  # �Ʒ� ��� ��ȿ (�Ƿε�, ����ġ ��)
	if EventBus:
		EventBus.emit("training_completed", event)


func _check_ability_acquisition_from_training(training_result: Dictionary):
	"""
	Phase 3: Check if player acquires special ability from coach teaching.
	Called after training completion but before week advancement.
	"""
	if not coach_card_system:
		return

	# 1. Determine training type and category
	var training_type: String = ""  # "technical", "physical", "mental", "tactical"
	var training_category: int = -1  # CoachCard.Category enum

	if training_result.has("effects"):
		var effects = training_result.effects
		if effects.has("physical_focus"):
			training_type = "physical"
			training_category = 1  # CoachCard.Category.PHYSICAL
		elif effects.has("mental_focus"):
			training_type = "mental"
			training_category = 2  # CoachCard.Category.MENTAL
		elif effects.has("technical_focus"):
			training_type = "technical"
			training_category = 0  # CoachCard.Category.TECHNICAL
		elif effects.has("tactical_focus"):
			training_type = "tactical"
			training_category = 3  # CoachCard.Category.TACTICAL

	if training_type.is_empty():
		# Default to technical if no specific focus
		training_type = "technical"
		training_category = 0

	print("[GameManager] Phase 3: Checking ability acquisition for %s training" % training_type)

	# 2. Calculate training quality (reuse logic from _calculate_weekly_growth)
	var coach_bonus: float = coach_card_system.calculate_training_bonus(training_category)
	var personality_multiplier: float = 1.0
	var player_data_node = get_node_or_null("/root/PlayerData")
	if player_data_node and player_data_node.has_method("get_training_efficiency_multiplier"):
		personality_multiplier = player_data_node.get_training_efficiency_multiplier()

	var training_quality: float = (1.0 + coach_bonus) * personality_multiplier
	# Scale to 0-10 range for Rust engine (1.0 = 5.0, 2.0 = 10.0)
	var quality_score: float = training_quality * 5.0

	print("[GameManager] Training quality: %.2f (score: %.1f/10)" % [training_quality, quality_score])

	# 3. Get coaches from active deck matching training category
	var deck_coaches = coach_card_system.get_deck_coaches_by_category(training_category)
	if deck_coaches.size() == 0:
		print("[GameManager] No coaches in deck for this training category")
		return

	# 4. Check each coach for ability teaching
	for coach in deck_coaches:
		var acquisition_result = coach_card_system.check_ability_teaching(coach, training_type, quality_score)

		# 5. If ability acquired, add to PlayerData
		if acquisition_result.success and acquisition_result.acquired:
			if player_data_node and player_data_node.has_method("add_ability"):
				var ability = {"ability_type": acquisition_result.ability_type, "tier": acquisition_result.tier}
				player_data_node.add_ability(ability)

				print(
					(
						"[GameManager] 🎓 NEW ABILITY! %s taught you %s (%s)"
						% [coach.coach_name, acquisition_result.ability_type, acquisition_result.tier]
					)
				)

				if acquisition_result.has("message"):
					print("[GameManager]   Message: %s" % acquisition_result.message)

				# Process automatic ability combinations
				if player_data_node.has_method("process_ability_combinations"):
					var combination_result = player_data_node.process_ability_combinations()
					if combination_result.success and combination_result.total_combinations > 0:
						print(
							(
								"[GameManager] 🌟 Ability combinations triggered! (%d)"
								% combination_result.total_combinations
							)
						)


func _on_match_completed(success_or_result, maybe_result = null):
	"""경기 완료 시 주차 진행 및 승격 체크"""
	var success := true
	var result: Dictionary = {}

	if maybe_result == null:
		if success_or_result is Dictionary:
			result = success_or_result
		else:
			print("[GameManager] Match completed callback received unexpected payload, skipping")
			return
	else:
		success = bool(success_or_result)
		result = maybe_result

	if not success:
		print("[GameManager] Match completed signal received with failure state. Skipping week advance.")
		return

	print("[GameManager] Match completed, advancing week...")
	set_last_result(result)  # 경기 결과 저장 (평점, 스코어 등)

	# M1.4: Check for squad promotion after match
	_check_squad_promotion()

	if _week_flow_running:
		print("[GameManager] ⏸️ Weekly flow already running; match completion handled by current cycle.")
		return

	advance_week()


func _check_squad_promotion():
	"""Check if player is eligible for squad promotion (M1.4)"""
	var my_team_data = get_node_or_null("/root/MyTeamData")
	if not my_team_data:
		return

	# Check promotion eligibility
	var eligibility = my_team_data.check_promotion_eligibility()
	if not eligibility or not eligibility.get("eligible", false):
		return

	# Auto-promote
	var promoted = my_team_data.promote_to_next_squad()
	if promoted:
		print("[GameManager] 🎉 Player auto-promoted to next squad level!")
	else:
		print("[GameManager] ⚠️ Promotion eligibility detected but promotion failed")


func _on_graduation_completed(success: bool, player_name: String):
	"""졸업 완료 시 처리 (3년차 종료)"""
	print("[GameManager] 🎓 GRADUATION! Success: %s, Player: %s" % [success, player_name])

	if not success:
		push_error("[GameManager] Graduation failed!")
		return

	# Save final game state
	if has_node("/root/SaveManager"):
		var save_manager = get_node("/root/SaveManager")
		if save_manager.current_save_slot:
			save_manager.save_game(save_manager.current_save_slot)
			print("[GameManager] Final game state saved")

	# Transition to graduation screen
	print("[GameManager] Transitioning to graduation ceremony...")
	get_tree().change_scene_to_file("res://scenes/GraduationScreen.tscn")


func handle_training(training_data: Dictionary) -> Dictionary:
	"""훈련 처리 및 주차 진행"""
	print("[GameManager] Handling training: ", training_data.get("name", "Unknown"))

	var result = {
		"success": true,
		"type": "training",
		"training_type": training_data.get("type", ""),
		"training_name": training_data.get("name", ""),
		"effects": training_data.get("effects", {}),
		"fatigue_added": training_data.get("fatigue_cost", 10),
		"week": current_week,
		"year": current_year
	}

	# PlayerData에 훈련 효과 적용
	if has_node("/root/PlayerData"):
		var player_data = get_node("/root/PlayerData")
		var effects = training_data.get("effects", {})
		for stat in effects:
			if stat in player_data:
				player_data[stat] += effects[stat]
				print("[GameManager] Applied training effect: %s +%d" % [stat, effects[stat]])

		# 피로도 증가
		if "fatigue" in player_data:
			player_data.fatigue += training_data.get("fatigue_cost", 10)
			print("[GameManager] Fatigue increased to: ", player_data.fatigue)

	# 결과 저장 및 주차 진행
	set_last_result(result)
	advance_week()

	return result


## ========= WeekHub / UI Bridges =========


func reset_weekly_training_state() -> void:
	"""외부 UI가 요청할 때 TrainingManager의 주간 상태를 초기화"""
	if TrainingManager and TrainingManager.has_method("reset_weekly_state"):
		TrainingManager.reset_weekly_state()


func apply_personal_rest() -> bool:
	"""ConditionSystem에 휴식 요청"""
	var condition = get_node_or_null("/root/PlayerCondition")
	if condition and condition.has_method("apply_rest_recovery"):
		condition.apply_rest_recovery()
		return true
	return false


func get_mvp_match_for_week(week: int) -> Dictionary:
	var match_sim = get_node_or_null("/root/MatchSimulationManager")
	if match_sim and match_sim.has_method("get_mvp_match_for_week"):
		return match_sim.call("get_mvp_match_for_week", week)
	return {}


func mark_mvp_match_played(week: int) -> void:
	var match_sim = get_node_or_null("/root/MatchSimulationManager")
	if match_sim and match_sim.has_method("mark_mvp_match_played"):
		match_sim.call("mark_mvp_match_played", week)


func get_recommended_team_training(match_data: Dictionary = {}) -> String:
	var match_sim = get_node_or_null("/root/MatchSimulationManager")
	if match_sim and match_sim.has_method("get_recommended_team_training"):
		return match_sim.call("get_recommended_team_training", match_data)
	return "tactical"


func execute_team_training(training_id: String, _match_data: Dictionary = {}) -> Dictionary:
	var response := {"success": false, "training_id": training_id, "message": "", "changes": {}}

	if not TrainingManager:
		response.message = "TrainingManager unavailable"
		return response

	var eligible := true
	if TrainingManager.has_method("can_execute_training"):
		var eligibility = TrainingManager.can_execute_training(training_id, "team")
		eligible = eligibility.get("can_execute", true)
		if not eligible:
			response.message = eligibility.get("reason", "Not eligible")
			return response

	var result = TrainingManager.execute_training(training_id, false)
	if result.get("success", false):
		response.success = true
		response.changes = result.get("changes", {})
	else:
		response.message = result.get("message", "Unknown failure")
	return response


func start_mvp_match(match_data: Dictionary) -> bool:
	if match_data.is_empty():
		return false
	var match_manager = get_node_or_null("/root/MatchManager")
	if match_manager and match_manager.has_method("start_match"):
		match_manager.call_deferred("start_match", match_data)
		return true
	return false


func advance_day_mvp() -> Dictionary:
	if DateManager and DateManager.has_method("advance_day_mvp"):
		return DateManager.advance_day_mvp()
	return {}


func advance_week() -> void:
	"""
	Advances the game by one week, handling all automatic processes.
	This is the primary entry point for weekly progression.
	Refactored to be async and include mandatory team training logic.
	"""
	if _week_flow_running:
		print(
			"[GameManager] ⚠️ advance_week() requested while weekly flow is already running. Ignoring duplicate call."
		)
		return

	_week_flow_running = true

	# --- Mandatory Team Training ---
	var _team_training_executed_flag = false
	if is_team_training_available():
		var mttm = get_node_or_null("/root/MandatoryTeamTrainingManager")
		if mttm and mttm.can_attend_training():
			print("[GameManager] 🏃 Executing mandatory team training...")
			await get_tree().create_timer(1.5).timeout

			var training_result = mttm.attend_team_training()
			_team_training_executed_flag = true
			print("[GameManager] ✅ Team training completed: %s" % training_result.get("message", ""))

			var result_payload = {
				"type": "team_training",
				"title": "⚽ 팀훈련 결과",
				"training_type": training_result.get("training_type", "알 수 없음"),
				"description": training_result.get("message", "팀훈련을 완료했습니다."),
				"stats_changed": training_result.get("stats_gained", {}),
				"fatigue_cost": training_result.get("fatigue_consumed", 0),
				"coach_relationship": training_result.get("coach_relationship_change", 0),
				"team_chemistry": training_result.get("team_chemistry_change", 0)
			}
			set_last_result(result_payload)
			team_training_executed.emit(result_payload)
		else:
			print("[GameManager] ⚠️ Cannot attend team training (e.g., low stamina).")
	else:
		print("[GameManager] ⏸️ No team training this week (Vacation).")

	# --- Week Progression (ALWAYS RUNS) ---
	current_week += 1
	completed_weeks += 1

	if current_week > 52:
		current_week = 1
		current_year += 1
		season_completed.emit(current_year - 1)
		print("[GameManager] New year: ", current_year)

	print("[GameManager] ===== Week %d Year %d =====" % [current_week, current_year])

	weekly_personal_activity_done = false
	weekly_activity_type = ""
	print("[GameManager] 🔄 Weekly activity reset.")

	if is_game_completed():
		end_game()
		_week_flow_running = false
		return

	await _process_weekly_flow()

	if coach_card_system:
		coach_card_system.decay_trust_weekly()

	week_advanced.emit(current_week, current_year)

	_week_flow_running = false


## ========== Phase 6.3: 4-Week Bulk Progress ==========

signal bulk_progress_started(weeks: int)
signal bulk_progress_week_completed(week_num: int, result: Dictionary)
signal bulk_progress_completed(results: Dictionary)
signal bulk_progress_interrupted(reason: String, results: Dictionary)

var _bulk_progress_running: bool = false


func bulk_advance_weeks(weeks: int = 4) -> Dictionary:
	"""
	4주 일괄 진행 (고속 육성).
	숙련 유저를 위한 훈련/이벤트 자동 처리.

	Args:
		weeks: 진행할 주 수 (기본 4주)

	Returns:
		결과 Dictionary:
		- weeks_advanced: 실제 진행된 주 수
		- training_results: 각 주 훈련 결과 배열
		- events_triggered: 발생한 이벤트 배열
		- stat_changes: 총 스탯 변화
		- interrupted: 중단 여부
		- interrupt_reason: 중단 이유
	"""
	if _bulk_progress_running:
		return {"error": "Bulk progress already running"}

	if _week_flow_running:
		return {"error": "Weekly flow already running"}

	_bulk_progress_running = true
	bulk_progress_started.emit(weeks)

	var results: Dictionary = {
		"weeks_advanced": 0,
		"training_results": [],
		"events_triggered": [],
		"stat_changes": {},
		"matches_played": 0,
		"interrupted": false,
		"interrupt_reason": ""
	}

	print("[GameManager] 🚀 Bulk progress started: %d weeks" % weeks)

	for i in range(weeks):
		# 게임 완료 체크
		if is_game_completed():
			results.interrupted = true
			results.interrupt_reason = "Game completed"
			break

		# 1주 자동 시뮬레이션
		var week_result: Dictionary = await _simulate_week_auto()

		results.weeks_advanced += 1
		results.training_results.append(week_result.get("training", {}))

		# 이벤트 수집
		var events: Array = week_result.get("events", [])
		results.events_triggered.append_array(events)

		# 스탯 변화 누적
		var stat_gains: Dictionary = week_result.get("stat_gains", {})
		for stat_name in stat_gains:
			if not results.stat_changes.has(stat_name):
				results.stat_changes[stat_name] = 0
			results.stat_changes[stat_name] += stat_gains[stat_name]

		# 경기 카운트
		if week_result.get("match_played", false):
			results.matches_played += 1

		bulk_progress_week_completed.emit(i + 1, week_result)

		# 중요 이벤트 발생 시 중단
		if week_result.get("has_blocking_event", false):
			results.interrupted = true
			results.interrupt_reason = (
				"Blocking event: %s" % week_result.get("blocking_event", {}).get("title", "Unknown")
			)
			print("[GameManager] ⚠️ Bulk progress interrupted: %s" % results.interrupt_reason)
			break

		# 짧은 대기 (UI 업데이트용)
		await get_tree().create_timer(0.1).timeout

	_bulk_progress_running = false

	if results.interrupted:
		bulk_progress_interrupted.emit(results.interrupt_reason, results)
	else:
		bulk_progress_completed.emit(results)

	print("[GameManager] ✅ Bulk progress completed: %d/%d weeks" % [results.weeks_advanced, weeks])
	return results


func _simulate_week_auto() -> Dictionary:
	"""
	1주 자동 시뮬레이션.
	기본 훈련 선택, 이벤트 체크, 경기 처리.
	"""
	var result: Dictionary = {
		"training": {},
		"events": [],
		"stat_gains": {},
		"match_played": false,
		"has_blocking_event": false,
		"blocking_event": {}
	}

	# 1. 자동 훈련 선택 및 실행
	var training_choice: String = _select_auto_training()
	if TrainingManager and TrainingManager.has_method("execute_training"):
		var training_result = TrainingManager.execute_training(training_choice, false)
		result.training = training_result
		if training_result.get("success", false):
			result.stat_gains = training_result.get("changes", {})

	# 2. 이벤트 체크
	var events: Array = _check_blocking_events()
	result.events = events

	for event in events:
		if _is_blocking_event(event):
			result.has_blocking_event = true
			result.blocking_event = event
			break

	# 3. 경기 처리 (있는 경우)
	if has_match_this_week() and not result.has_blocking_event:
		result.match_played = true
		# 경기는 간단히 스킵 처리 (결과만 생성)
		var match_result = _generate_auto_match_result()
		result["match_result"] = match_result

	# 4. 주차 진행 (블로킹 이벤트 없는 경우)
	if not result.has_blocking_event:
		current_week += 1
		completed_weeks += 1

		if current_week > 52:
			current_week = 1
			current_year += 1

		weekly_personal_activity_done = false
		weekly_activity_type = ""

		week_advanced.emit(current_week, current_year)

	return result


func _select_auto_training() -> String:
	"""자동 훈련 선택 (밸런스드)."""
	# 플레이어 상태에 따른 자동 선택
	var condition = get_node_or_null("/root/PlayerCondition")

	if condition:
		# 체력 낮으면 휴식
		if condition.has_method("should_rest") and condition.should_rest():
			return "rest"

		# 컨디션에 따른 훈련 강도 조절
		if condition.has_method("get_condition_level"):
			var level = condition.get_condition_level()
			if level < 30:
				return "rest"

	# 기본: 밸런스드 훈련 (순환)
	var training_options: Array = ["technical", "physical", "mental", "tactical"]
	var week_index: int = current_week % training_options.size()
	return training_options[week_index]


func _check_blocking_events() -> Array:
	"""블로킹 이벤트 체크."""
	var events: Array = []

	if not EventManager:
		return events

	# 주간 이벤트 체크
	var weekly_event = EventManager.get_event_by_week(current_week)
	if not weekly_event.is_empty():
		events.append(weekly_event)

	# 큐잉된 이벤트 체크
	var queued = EventManager.get_queued_events()
	events.append_array(queued)

	return events


func _is_blocking_event(event: Dictionary) -> bool:
	"""이벤트가 블로킹인지 확인."""
	# 다음 유형의 이벤트는 블로킹:
	# - 선택이 필요한 이벤트
	# - 스토리 이벤트
	# - 보스 이벤트
	# - 특별 매치 이벤트

	var event_type: String = event.get("type", "")
	var requires_choice: bool = event.get("requires_choice", false)
	var is_story: bool = event.get("is_story", false)
	var is_boss: bool = event.get("is_boss", false)

	if requires_choice or is_story or is_boss:
		return true

	# 특정 이벤트 타입 블로킹
	var blocking_types: Array = ["story", "boss", "promotion", "graduation", "special_match"]
	if event_type in blocking_types:
		return true

	return false


func _generate_auto_match_result() -> Dictionary:
	"""자동 경기 결과 생성 (간략화)."""
	# 간단한 결과 생성 (실제로는 MatchSimulationManager 사용)
	var player_rating: float = randf_range(5.5, 8.0)
	var home_score: int = randi_range(0, 3)
	var away_score: int = randi_range(0, 3)

	return {
		"success": true,
		"home_score": home_score,
		"away_score": away_score,
		"player_rating": player_rating,
		"auto_generated": true
	}


func is_bulk_progress_running() -> bool:
	"""벌크 진행 중 여부."""
	return _bulk_progress_running


func cancel_bulk_progress() -> void:
	"""벌크 진행 취소."""
	if _bulk_progress_running:
		_bulk_progress_running = false
		print("[GameManager] Bulk progress cancelled")


func check_and_trigger_events() -> Array:
	"""
	Checks for events at the current week and triggers them via EventManager.
	Returns an array of events that were triggered (for UI to handle dialogue await).
	Business logic moved from HomeImproved._check_for_events().
	"""
	var triggered_events: Array = []

	if not EventManager:
		print("[GameManager] ⚠️ EventManager not available for event checking.")
		events_check_completed.emit(triggered_events)
		return triggered_events

	# 1. Week별 이벤트 체크
	var weekly_event = EventManager.get_event_by_week(current_week)

	if not weekly_event.is_empty():
		print("[GameManager] 🎭 Event triggered for week %d: %s" % [current_week, weekly_event.get("title", "")])
		EventManager.trigger_event(weekly_event)
		triggered_events.append(weekly_event)

	# 2. 큐잉된 이벤트 체크
	var queued_events = EventManager.get_queued_events()

	for queued_event in queued_events:
		print("[GameManager] 🎭 Queued event triggered: %s" % queued_event.get("title", ""))
		EventManager.trigger_event(queued_event)
		triggered_events.append(queued_event)

	events_check_completed.emit(triggered_events)
	return triggered_events


func _process_weekly_systems():
	"""주간 시스템 통합 처리"""
	var week_summary = {
		"week": current_week,
		"year": current_year,
		"events": [],
		"relationship_changes": [],
		"condition_changes": {},
		"training_modifiers": {}
	}

	# 1. 플레이어 데이터 수집
	var player_data = _get_player_data_snapshot()

	# 2. 주간 이벤트 처리 (WeeklyEventSystem)
	if has_node("/root/WeeklyEventSystem"):
		var event_system = get_node("/root/WeeklyEventSystem")
		var weekly_event = event_system.roll_weekly_event(player_data)
		if not weekly_event.is_empty():
			week_summary.events.append(weekly_event)
			print("[GameManager] Weekly Event: ", weekly_event.message)

	# 3. 관계 이벤트 처리 (RelationshipSystem)
	if has_node("/root/RelationshipSystem"):
		var rel_system = get_node("/root/RelationshipSystem")
		var relationship_event = rel_system.roll_relationship_event(player_data)
		if not relationship_event.is_empty():
			week_summary.events.append(relationship_event)
			print("[GameManager] Relationship Event: ", relationship_event.message)

	# 4. 컨디션 업데이트 (PlayerCondition)
	if has_node("/root/PlayerCondition"):
		var condition_system = get_node("/root/PlayerCondition")
		var training_intensity = last_result.get("average_intensity", 0.5)
		var consecutive_days = _calculate_consecutive_training_days()
		condition_system.update_daily_condition(training_intensity, consecutive_days)
		week_summary.condition_changes = {
			"level": condition_system.get_condition_text(), "modifier": condition_system.get_training_modifier()
		}

	# 5. 훈련 효과 업데이트 (WeeklyEventSystem)
	if has_node("/root/WeeklyEventSystem"):
		var event_system = get_node("/root/WeeklyEventSystem")
		event_system.update_weekly_effects()
		week_summary.training_modifiers["event_modifier"] = event_system.get_active_training_modifier()

	# 6. 관계 훈련 수정자 (RelationshipSystem)
	if has_node("/root/RelationshipSystem"):
		var rel_system = get_node("/root/RelationshipSystem")
		week_summary.training_modifiers["relationship_modifier"] = rel_system.get_relationship_modifier()

	# 7. 플레이어 데이터 업데이트
	if has_node("/root/PlayerData"):
		var player_data_node = get_node("/root/PlayerData")
		player_data_node.current_week = current_week
		player_data_node.current_year = current_year

	# 8. 주간 통계 저장
	weekly_stats.append(week_summary)

	# 9. 마지막 결과 초기화 (새로운 주 시작)
	clear_last_result()

	print("[GameManager] Weekly systems processed for Week ", current_week)


func _get_player_data_snapshot() -> Dictionary:
	"""현재 플레이어 데이터 스냅샷 생성"""
	var snapshot = {
		"week": current_week, "year": current_year, "overall_rating": 50, "potential": 80, "fatigue": 0, "skills": {}
	}

	# PlayerData에서 정보 수집
	if has_node("/root/PlayerData"):
		var player_data = get_node("/root/PlayerData")
		snapshot.overall_rating = player_data.get_overall_rating()
		snapshot.fatigue = player_data.fatigue

		# 개별 스킬 정보
		var all_stats = player_data.get_all_stats()
		for category in all_stats:
			for skill_name in all_stats[category]:
				var skill_value = all_stats[category][skill_name]
				snapshot.skills[skill_name] = skill_value

	# PlayerManager에서 추가 정보
	if has_node("/root/PlayerManager"):
		var player_manager = get_node("/root/PlayerManager")
		# 특수능력 정보 수집
		if player_manager.has_method("get_special_abilities"):
			snapshot["special_abilities"] = player_manager.get_special_abilities()

		# 성장 곡선 정보 수집
		if player_manager.has_method("get_growth_curve"):
			snapshot["growth_curve"] = player_manager.get_growth_curve()

		# 현재 CA/PA 정보
		if player_manager.has_method("get_ability_info"):
			var ability_info = player_manager.get_ability_info()
			snapshot["current_ability"] = ability_info.get("ca", 50)
			snapshot["potential_ability"] = ability_info.get("pa", 80)

	return snapshot


func _calculate_consecutive_training_days() -> int:
	"""연속 훈련 일수 계산"""
	# 간단한 구현: 마지막 결과의 강도를 기반으로 추정
	var intensity = last_result.get("average_intensity", 0.5)
	if intensity > 0.7:
		return 5  # 고강도 훈련 추정
	elif intensity > 0.3:
		return 3  # 중강도 훈련 추정
	else:
		return 1  # 저강도 또는 휴식


func get_current_week() -> int:
	return current_week


func get_current_year() -> int:
	return current_year


func get_completed_weeks() -> int:
	return completed_weeks


func get_remaining_weeks() -> int:
	return total_weeks - completed_weeks


func get_progress_percentage() -> float:
	return float(completed_weeks) / float(total_weeks) * 100.0


func get_week_display_text() -> String:
	"""주차 표시 텍스트"""
	return "%d학년 %d주차" % [current_year, current_week]


func get_game_summary() -> Dictionary:
	"""게임 요약 정보"""
	return {
		"current_week": current_week,
		"current_year": current_year,
		"completed_weeks": completed_weeks,
		"remaining_weeks": get_remaining_weeks(),
		"progress_percentage": get_progress_percentage(),
		"total_events": weekly_stats.size(),
		"game_state": game_state
	}


func pause_game():
	"""게임 일시정지"""
	game_state = "paused"
	game_state_changed.emit(game_state)
	print("[GameManager] Game paused")


func resume_game():
	"""게임 재개"""
	game_state = "playing"
	game_state_changed.emit(game_state)
	print("[GameManager] Game resumed")


func end_game():
	"""게임 종료"""
	game_state = "ended"
	game_state_changed.emit(game_state)
	print("[GameManager] Game ended - Final Stats:")
	print("  - Total weeks completed: ", completed_weeks)
	print("  - Years completed: ", current_year)
	print("  - Total events experienced: ", weekly_stats.size())


func is_playing() -> bool:
	"""게임 진행 중인지 확인"""
	return game_state == "playing"


func is_game_completed() -> bool:
	"""게임 완료 여부"""
	return completed_weeks >= total_weeks


func reset_game():
	"""게임 초기화"""
	current_week = 1
	current_year = 1
	completed_weeks = 0
	game_state = "playing"
	last_result = {}
	weekly_stats.clear()
	waiting_for_turn_decision = false

	# Phase 6B: Reset DateManager
	if date_manager and date_manager.has_method("reset_for_testing"):
		date_manager.reset_for_testing()
		print("[GameManager] DateManager reset")

	game_state_changed.emit(game_state)
	print("[GameManager] Game reset")


func set_last_result(result: Dictionary):
	"""마지막 결과 저장"""
	last_result = result
	print("[GameManager] Last result saved: ", result.get("type", "unknown"))


func get_last_result() -> Dictionary:
	"""마지막 결과 반환"""
	return last_result


func clear_last_result():
	"""마지막 결과 초기화"""
	last_result = {}


func get_weekly_stats() -> Array:
	"""주간 통계 배열 반환"""
	return weekly_stats


func get_recent_events(count: int = 5) -> Array:
	"""최근 이벤트 반환"""
	var recent_events = []
	var start_index = max(0, weekly_stats.size() - count)

	for i in range(start_index, weekly_stats.size()):
		var week_data = weekly_stats[i]
		for event in week_data.events:
			recent_events.append({"week": week_data.week, "year": week_data.year, "event": event})

	return recent_events


func _process_weekly_flow():
	"""우마무스메 스타일 주간 진행 플로우"""
	if _week_flow_running:
		print("[GameManager] ⚠️ Weekly flow already running; skipping duplicate call")
		return
	_week_flow_running = true
	print("[GameManager] 📅 주간 플로우 시작")

	# 1. 월요일: 체력 리셋
	_monday_stamina_reset()

	# 2. 화요일: 의무 팀훈련 (감독 결정)
	_tuesday_team_training()

	# 3. 수-목: 개인훈련 선택 (플레이어 선택)
	_wednesday_thursday_individual_training()

	# 4. 금요일: 경기 (있는 경우)
	if _friday_sim_in_progress:
		print_debug("[GameManager] Skipping duplicate Friday match trigger")
	else:
		_friday_sim_in_progress = true
		await _friday_match()
		_friday_sim_in_progress = false

	# 5. 주말: 이벤트 & 휴식
	_weekend_events()

	# 6. 주간 성장 계산
	_calculate_weekly_growth()

	# 7. 기존 시스템 처리
	_process_weekly_systems()
	_week_flow_running = false


func _monday_stamina_reset():
	"""월요일: 체력 리셋 (우마무스메 스타일)"""
	print("[GameManager] 🔄 월요일: 체력 리셋")

	if has_node("/root/PlayerCondition"):
		var condition = get_node("/root/PlayerCondition")
		condition.on_week_start()


func _tuesday_team_training():
	"""화요일: 의무 팀훈련"""
	print("[GameManager] 💪 화요일: 팀훈련")

	# 학기 체크 - 방학 중에는 팀훈련 없음
	if not is_team_training_available():
		print("[GameManager] ⏸️  방학 중 - 팀훈련 없음 (현재: %s)" % get_period_name())
		return

	print("[GameManager] ✅ 팀훈련 가능 (현재: %s)" % get_period_name())

	# 감독이 훈련 종류 결정
	if has_node("/root/ManagerSystem"):
		var manager = get_node("/root/ManagerSystem")
		var training_type = manager.decide_weekly_training()
		print("[GameManager] 감독 결정: %s 훈련" % training_type)

		# 팀훈련 실행
		if has_node("/root/MandatoryTeamTrainingManager"):
			var team_training = get_node("/root/MandatoryTeamTrainingManager")
			team_training.attend_team_training()


func _wednesday_thursday_individual_training():
	"""수-목: 개인훈련 기회"""
	print("[GameManager] 🏃 수-목: 개인훈련 기회")

	# UI에서 선택하도록 신호 발송 (실제 게임에서)
	# 여기서는 자동 처리를 위한 로직
	if has_node("/root/PlayerCondition"):
		var condition = get_node("/root/PlayerCondition")

		# 체력에 따른 자동 선택 (AI 모드)
		if condition.should_rest():
			print("[GameManager] 체력 부족으로 휴식 선택")
			condition.consume_stamina("rest")
		else:
			print("[GameManager] 개인훈련 실행 가능")
			# TrainingManager로 개인훈련 처리


func _friday_match():
	"""금요일: 경기"""
	if has_match_this_week():
		print("[GameManager] ⚽ 금요일: 경기")

		if has_node("/root/MatchSimulationManager"):
			var match_sim = get_node("/root/MatchSimulationManager")

			# 경기 시뮬레이션 실행 (타임아웃 가드 추가)
			var match_result: Dictionary = {}
			var timeout_seconds := 30.0
			var timed_out := false

			# 타임아웃 타이머 시작 (백그라운드)
			var timeout_timer := get_tree().create_timer(timeout_seconds)
			timeout_timer.timeout.connect(func():
				if match_result.is_empty():
					timed_out = true
					push_warning("[GameManager] ⚠️ 경기 시뮬레이션이 %.0fs를 초과했습니다" % timeout_seconds)
			)

			# 경기 시뮬레이션 실행
			match_result = await match_sim.start_league_match()

			# 타임아웃 체크
			if timed_out:
				push_warning("[GameManager] 경기가 타임아웃 후 완료됨 - 결과 무시")
				match_result = {"success": false, "error": "timeout"}

			# 경기 결과 저장
			if match_result.has("success") and match_result.success:
				set_last_result(match_result)
				print(
					(
						"[GameManager] 경기 결과: %d - %d"
						% [match_result.get("home_score", 0), match_result.get("away_score", 0)]
					)
				)

				# 플레이어 평점 적용
				if match_result.has("player_rating") and PlayerData:
					PlayerData.last_match_rating = match_result.player_rating
			else:
				print("[GameManager] 경기 시뮬레이션 실패: %s" % str(match_result))

		# 체력 소모
		if has_node("/root/PlayerCondition"):
			var condition = get_node("/root/PlayerCondition")
			condition.consume_stamina("match")


func _weekend_events():
	"""주말: 이벤트 & 휴식"""
	print("[GameManager] 🎉 주말: 이벤트 & 휴식")

	# 주간 이벤트 시스템 처리
	if has_node("/root/WeeklyEventSystem"):
		var event_system = get_node("/root/WeeklyEventSystem")
		# 이벤트는 이미 _process_weekly_systems(94번줄)에서 처리됨
		# 추가적인 주말 특별 이벤트만 처리
		if event_system.has_method("trigger_weekend_event"):
			var weekend_event = event_system.trigger_weekend_event()
			if not weekend_event.is_empty():
				print("[GameManager] 주말 이벤트 발생: %s" % weekend_event.get("message", ""))


func _calculate_weekly_growth():
	"""주간 CA 성장 계산"""
	print("[GameManager] 📈 CA 성장 계산")

	if not has_node("/root/EnhancedPlayerData"):
		return

	var player_data = get_node("/root/EnhancedPlayerData")
	if not has_node("/root/CACalculator"):
		return

	var ca_calculator = get_node("/root/CACalculator")

	# Phase 2: Calculate coach training bonuses (if available)
	var coach_bonus: float = 0.0
	if coach_card_system:
		# Determine training category from last result (default TECHNICAL if unknown)
		var training_category: int = 0  # Default to TECHNICAL
		if last_result.has("effects"):
			var effects = last_result.effects
			if effects.has("physical_focus"):
				training_category = 1  # PHYSICAL
			elif effects.has("mental_focus"):
				training_category = 2  # MENTAL
			elif effects.has("technical_focus"):
				training_category = 0  # TECHNICAL

		coach_bonus = coach_card_system.calculate_training_bonus(training_category)
		print("[GameManager] Coach bonus: +%.1f%%" % (coach_bonus * 100))

	# Phase 4: Calculate personality training efficiency multiplier (0.7-1.3x)
	var personality_multiplier: float = 1.0
	var player_data_node = get_node_or_null("/root/PlayerData")
	if player_data_node and player_data_node.has_method("get_training_efficiency_multiplier"):
		personality_multiplier = player_data_node.get_training_efficiency_multiplier()
		print("[GameManager] Personality efficiency: %.2fx" % personality_multiplier)

	# 나이와 PA를 고려한 성장 계산 (코치 보너스 + 성격 효율 적용)
	var training_quality: float = (1.0 + coach_bonus) * personality_multiplier
	var growth_data = ca_calculator.calculate_growth_potential(
		player_data.current_ability, player_data.potential_ability, player_data.player_age, training_quality  # 훈련 품질에 코치 보너스 반영
	)

	# 주간 CA 성장 적용
	if growth_data.weekly_growth_rate > 0:
		var old_ca = player_data.current_ability
		player_data.current_ability = min(
			player_data.potential_ability, player_data.current_ability + int(growth_data.weekly_growth_rate)
		)

		if old_ca != player_data.current_ability:
			print(
				(
					"[GameManager] CA 성장: %d → %d (+%d)"
					% [old_ca, player_data.current_ability, player_data.current_ability - old_ca]
				)
			)


func has_match_this_week() -> bool:
	"""이번 주 경기 여부 확인"""
	# 간단한 로직: 4주에 3번 경기
	return (current_week % 4) != 0


func get_current_period() -> Period:
	"""현재 학기 상태 반환 (학기/합숙/방학)"""
	# 1. 합숙 체크 (최우선)
	if CAMP_WEEKS.has(current_year):
		if current_week in CAMP_WEEKS[current_year]:
			return Period.CAMP

	# 2. 방학 체크 (단, 합숙 주간은 이미 위에서 처리됨)
	if VACATION_WEEKS.has(current_year):
		if current_week in VACATION_WEEKS[current_year]:
			return Period.VACATION

	# 3. 그 외는 시즌 중
	return Period.SEASON


func get_period_name() -> String:
	"""현재 기간 이름 반환"""
	var period = get_current_period()
	match period:
		Period.SEASON:
			return "시즌중"
		Period.CAMP:
			return "합숙"
		Period.VACATION:
			return "방학"
		_:
			return "알 수 없음"


func is_team_training_available() -> bool:
	"""팀훈련 가능 여부 체크"""
	var period = get_current_period()
	# 시즌중 또는 합숙 중에만 팀훈련 가능
	return period == Period.SEASON or period == Period.CAMP


# 테스트 함수
func test_weekly_progression():
	"""주간 진행 시스템 테스트"""
	print("=== 주간 진행 시스템 테스트 ===")
	print("현재 상태: ", get_week_display_text())
	print("진행률: %.1f%%" % get_progress_percentage())

	# 몇 주 진행 테스트
	for i in range(3):
		advance_week()
		await get_tree().process_frame

	print("테스트 완료 - 현재: ", get_week_display_text())
	print("최근 이벤트: ", get_recent_events(3))


# Event Bus 이벤트 핸들러들
func _on_training_card_selected(data: Dictionary):
	"""TrainingCard에서 훈련이 선택되었을 때"""
	print("[GameManager] 🎯 훈련 선택됨: ", data["training_type"])
	print("  ├─ 강도: ", data["intensity"])
	print("  ├─ 피로도: ", data["fatigue_cost"])
	print("  └─ 카테고리: ", data["category"])

	# 여기서 실제 훈련 시스템과 연동 가능
	# TrainingManager.execute_training(data)


func _on_training_card_hovered(data: Dictionary):
	"""TrainingCard에 마우스가 호버되었을 때"""
	print("[GameManager] 👆 훈련 호버: ", data["display_name"])
	print("  └─ 설명: ", data["description"])

	# 여기서 UI 툴팁이나 정보 패널 업데이트 가능


# Bridge Event Bus 이벤트 핸들러들
func _on_bridge_training_completed(data: Dictionary):
	"""Bridge에서 훈련이 완료되었을 때"""
	print("[GameManager] 🏃‍♂️ Bridge 훈련 완료: ", data)

	# 여기서 훈련 결과를 PlayerData에 반영하고 UI 업데이트
	# 또는 QuestSystem에 진행도 업데이트 가능


func _on_bridge_week_completed(data: Dictionary):
	"""Bridge에서 주간 처리가 완료되었을 때"""
	print("[GameManager] 📅 Bridge 주간 완료: ", data)

	# 새로운 주차로 진행하고 이벤트 트리거
	advance_week()


func _on_bridge_match_completed(data: Dictionary):
	"""Bridge에서 경기가 완료되었을 때"""
	print("[GameManager] ⚽ Bridge 경기 완료: ", data)

	# 경기 결과를 분석하고 시즌 기록에 추가
	# 성과에 따른 스토리 이벤트 트리거 가능


func _on_bridge_state_changed(_data: Dictionary):
	"""Bridge에서 게임 상태가 변경되었을 때"""
	print("[GameManager] 🔄 Bridge 상태 변경: 플레이어 데이터 업데이트됨")

	# 게임 상태 변경을 다른 시스템들에 알림
	# UI 시스템들이 최신 데이터로 업데이트하도록


func _on_player_fatigue_changed(data: Dictionary):
	"""플레이어 피로도가 변경되었을 때"""
	print("[GameManager] 😴 피로도 변경: ", data["fatigue"], " (소스: ", data["source"], ")")

	# 피로도에 따른 추가 이벤트나 경고 시스템
	if data["fatigue"] > 80:
		print("  ⚠️ 피로도 높음 - 휴식 권장")
		# 여기서 Dialogic으로 피로 경고 이벤트 트리거 가능


func _on_player_performance_updated(data: Dictionary):
	"""경기에서 선수 성과가 업데이트되었을 때"""
	print("[GameManager] ⭐ 선수 성과: ", data["player_name"], " 평점:", data["rating"])

	# 좋은 성과시 스토리 이벤트 트리거
	if data["rating"] >= 8.0:
		print("  🏆 훌륭한 경기! 스토리 이벤트 트리거 가능")
		# EventBus.emit("trigger_story", {"type": "excellent_performance"})

	# QuestSystem에 경기 성과 진행도 업데이트
	# QuestSystem.update_progress("match_performance", "rating", data["rating"])


## ===== 주간 활동 제한 시스템 (사용자 요청 기능) =====


func mark_personal_activity(activity_type: String):
	"""개인 활동 완료 표시 (training/rest/go_out)"""
	weekly_personal_activity_done = true
	weekly_activity_type = activity_type
	print("[GameManager] ✅ Personal activity marked: %s" % activity_type)


func can_do_personal_activity() -> bool:
	"""개인 활동 가능 여부 확인 (주간 1회 제한)"""
	return not weekly_personal_activity_done


func can_progress_week() -> bool:
	"""주차 진행 가능 여부 (개인 활동 1회 필수)"""
	return weekly_personal_activity_done


func get_weekly_activity_status() -> Dictionary:
	"""주간 활동 상태 조회"""
	return {
		"activity_done": weekly_personal_activity_done,
		"activity_type": weekly_activity_type,
		"can_do_activity": can_do_personal_activity(),
		"can_progress": can_progress_week()
	}


## Phase 6B: DateManager Query Methods


func get_current_turn_info() -> Dictionary:
	"""Get current turn information from DateManager"""
	if not date_manager:
		return {}

	return {
		"current_day": date_manager.current_day,
		"current_week": date_manager.current_week,
		"current_year": date_manager.current_year,
		"day_of_week": date_manager.current_day_of_week,
		"turn_type": date_manager.current_turn_type,
		"turn_state": date_manager.current_turn_state,
		"stamina": date_manager.stamina,
		"form": date_manager.form,
		"morale": date_manager.morale
	}


func get_turn_progress() -> Dictionary:
	"""Get overall turn progress statistics from DateManager"""
	if not date_manager:
		return {}

	return {
		"total_days": 798,
		"completed_days": date_manager.current_day,
		"current_week": date_manager.current_week,
		"current_year": date_manager.current_year,
		"total_training_turns": date_manager.total_training_turns,
		"total_match_turns": date_manager.total_match_turns,
		"total_active_turns": date_manager.active_turn_count,
		"progress_percentage": (float(date_manager.current_day) / 798.0) * 100.0
	}


func is_waiting_for_decision() -> bool:
	"""Check if currently waiting for player's turn decision"""
	return waiting_for_turn_decision


func debug_print_turn_schedule():
	"""Debug: Print DateManager's fixture schedule"""
	if date_manager:
		print("[GameManager] ====== DateManager Schedule ======")
		print("Current State:")
		print("  - Day: %d / 798" % date_manager.current_day)
		print("  - Week: %d Year: %d" % [date_manager.current_week, date_manager.current_year])
		print("  - Turn Type: %s" % date_manager.current_turn_type)
		print("  - Stamina: %d, Form: %d, Morale: %d" % [date_manager.stamina, date_manager.form, date_manager.morale])
		print("")
		print("Upcoming Fixtures (Year %d):" % date_manager.current_year)
		var fixtures = date_manager.season_fixtures
		for fixture in fixtures.slice(0, min(5, fixtures.size())):
			print(
				(
					"  - Week %d Day %d: %s vs %s"
					% [fixture.week, fixture.day_of_week, fixture.home_team, fixture.away_team]
				)
			)
	else:
		print("[GameManager] DateManager not initialized!")
