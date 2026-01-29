extends Node
# class_name removed - this is an autoload singleton

# TrainingManager - 21가지 훈련 시스템 (CLAUDE2_IMPLEMENTATION.md 명세)

signal training_completed(result: Dictionary)
# Event Bus로 변경됨
# signal fatigue_changed(value: float)

# 21가지 훈련 타입 (Power Pro 스타일)
enum TrainingType {
	# Physical Training (5가지)
	PHYSICAL_ENDURANCE,
	PHYSICAL_STRENGTH,
	PHYSICAL_SPEED,
	PHYSICAL_AGILITY,
	PHYSICAL_RECOVERY,
	# Technical Training (5가지)
	TECHNICAL_BALL_CONTROL,
	TECHNICAL_PASSING,
	TECHNICAL_SHOOTING,
	TECHNICAL_CROSSING,
	TECHNICAL_SET_PIECES,
	# Tactical Training (5가지)
	TACTICAL_POSITIONING,
	TACTICAL_TEAM_SHAPE,
	TACTICAL_PRESSING_DRILLS,
	TACTICAL_TRANSITION_PLAY,
	TACTICAL_SET_PIECES_DEFENSIVE,
	# Mental Training (3가지)
	MENTAL_CONCENTRATION,
	MENTAL_DECISION_MAKING,
	MENTAL_LEADERSHIP,
	# Match Preparation (3가지)
	MATCH_PREPARATION,
	MATCH_VIDEO_ANALYSIS,
	MATCH_OPPONENT_SPECIFIC
}

# 5단계 훈련 강도
enum TrainingIntensity { VERY_LIGHT, LIGHT, MODERATE, HIGH, VERY_HIGH }  # 20-40%  # 40-60%  # 60-75%  # 75-90%  # 90-100%

# ========================================
# Phase 6C: OpenFootball API Type Conversion
# ========================================


## Convert Godot TrainingType (21) to OpenFootball target attribute (7)
## OpenFootball targets: pace, power, technical, passing, shooting, defending, mental
func convert_training_type_to_of(godot_type: TrainingType) -> String:
	"""Convert Godot TrainingType enum to OpenFootball target string"""
	match godot_type:
		# Physical → pace (2 types)
		TrainingType.PHYSICAL_SPEED, TrainingType.PHYSICAL_AGILITY:
			return "pace"

		# Physical → power (3 types)
		TrainingType.PHYSICAL_ENDURANCE, TrainingType.PHYSICAL_STRENGTH, TrainingType.PHYSICAL_RECOVERY:
			return "power"

		# Technical → technical (1 type)
		TrainingType.TECHNICAL_BALL_CONTROL:
			return "technical"

		# Technical → passing (2 types)
		TrainingType.TECHNICAL_PASSING, TrainingType.TECHNICAL_CROSSING:
			return "passing"

		# Technical → shooting (2 types)
		TrainingType.TECHNICAL_SHOOTING, TrainingType.TECHNICAL_SET_PIECES:
			return "shooting"

		# Tactical → pace (1 type - transition requires speed)
		TrainingType.TACTICAL_TRANSITION_PLAY:
			return "pace"

		# Tactical → defending (4 types)
		TrainingType.TACTICAL_POSITIONING, TrainingType.TACTICAL_TEAM_SHAPE, TrainingType.TACTICAL_PRESSING_DRILLS, TrainingType.TACTICAL_SET_PIECES_DEFENSIVE:
			return "defending"

		# Mental → mental (3 types)
		TrainingType.MENTAL_CONCENTRATION, TrainingType.MENTAL_DECISION_MAKING, TrainingType.MENTAL_LEADERSHIP:
			return "mental"

		# Match Preparation → technical with light intensity (3 types)
		TrainingType.MATCH_PREPARATION, TrainingType.MATCH_VIDEO_ANALYSIS, TrainingType.MATCH_OPPONENT_SPECIFIC:
			return "technical"

		_:
			push_error("[TrainingManager] Unknown training type: %d" % godot_type)
			return "technical"  # Safe fallback


## Convert Godot TrainingIntensity (5) to OpenFootball intensity (3)
## OpenFootball intensities: light, normal, intensive
func convert_intensity_to_of(godot_intensity: TrainingIntensity) -> String:
	"""Convert Godot TrainingIntensity enum to OpenFootball intensity string"""
	match godot_intensity:
		TrainingIntensity.VERY_LIGHT, TrainingIntensity.LIGHT:
			return "light"
		TrainingIntensity.MODERATE:
			return "normal"
		TrainingIntensity.HIGH, TrainingIntensity.VERY_HIGH:
			return "intensive"
		_:
			push_error("[TrainingManager] Unknown intensity: %d" % godot_intensity)
			return "normal"  # Safe fallback


# ========================================
# End Phase 6C Type Conversion
# ========================================

# 훈련 레벨 (1-9)
var training_levels: Dictionary = {}

# 코치 보너스
var coach_bonus: float = 1.0
var mentor_bonus: float = 0.0


func _ready():
	print("[TrainingManager] Initialized")
	_init_training_levels()


func _init_training_levels():
	# 모든 훈련의 레벨을 1로 초기화
	for training_type in TrainingType.values():
		training_levels[training_type] = 1


func execute_training(training_type: TrainingType, intensity: TrainingIntensity) -> Dictionary:
	"""
	Phase 6C: Execute training via OpenFootballAPI
	Replaces custom XP system with OpenFootball Rust engine training simulation
	"""
	# Convert Godot types to OpenFootball format
	var of_target = convert_training_type_to_of(training_type)
	var of_intensity = convert_intensity_to_of(intensity)

	# Get PlayerData in CorePlayer format
	if not PlayerData:
		push_error("[TrainingManager] PlayerData not available!")
		return {"success": false, "error": "PlayerData unavailable"}

	var core_player = PlayerData.to_core_player_format()

	# Execute training via OpenFootballAPI
	var api = get_node_or_null("/root/OpenFootballAPI")
	if not api:
		push_error("[TrainingManager] OpenFootballAPI not available! Falling back to legacy system.")
		return _execute_training_legacy(training_type, intensity)

	var api_result = api.execute_training(core_player, of_target, of_intensity)

	# Check for API errors
	if not api_result.success:
		push_error("[TrainingManager] Training API failed: %s" % api_result.get("error", "Unknown error"))
		return {"success": false, "error": api_result.get("error", "API failure")}

	var response = api_result.response

	# Apply training results to PlayerData
	_apply_training_results_to_player(response)

	# Build result dictionary for UI/signals
	var result = {
		"training_name": get_training_name(training_type),
		"intensity_name": get_intensity_name(intensity),
		"success": true,
		"ca_before": response.get("ca_before", 0),
		"ca_after": response.get("ca_after", 0),
		"ca_gained": response.get("ca_after", 0) - response.get("ca_before", 0),
		"stamina_before": response.get("stamina_before", 100),
		"stamina_after": response.get("stamina_after", 100),
		"stamina_cost": response.get("stamina_cost", 0),
		"target_attribute": of_target,
		"intensity": of_intensity,
		"attribute_changes": response.get("attribute_changes", {}),
		"condition_before": response.get("condition_before", "Normal"),
		"condition_after": response.get("condition_after", "Normal")
	}

	# Emit signals
	training_completed.emit(result)

	# EventBus notification (convert stamina cost to fatigue)
	var fatigue_equivalent = response.get("stamina_cost", 0)
	EventBus.emit("player_fatigue_changed", {"fatigue": fatigue_equivalent, "source": "training"})

	# 훈련 레벨 증가 (확률적 - legacy system)
	_try_level_up(training_type)

	print(
		(
			"[TrainingManager] ✅ Training via API: %s (%s) | CA: %d → %d (+%d) | Stamina: %d → %d (-%d)"
			% [
				result.training_name,
				result.intensity_name,
				result.ca_before,
				result.ca_after,
				result.ca_gained,
				result.stamina_before,
				result.stamina_after,
				result.stamina_cost
			]
		)
	)

	return result


func _apply_training_results_to_player(response: Dictionary):
	"""Apply OpenFootball training results to PlayerData"""
	if not PlayerData:
		return

	# Apply attribute changes
	var attr_changes = response.get("attribute_changes", {})
	for attr_name in attr_changes:
		var change = attr_changes[attr_name]
		_apply_attribute_change(attr_name, change)

	# Update stamina (convert to fatigue)
	var stamina_after = response.get("stamina_after", 100)
	var fatigue = 100 - stamina_after
	PlayerData.fatigue = clamp(fatigue, 0, 100)

	# Update condition (convert OpenFootball 5-tier to Godot 1-5)
	var condition_str = response.get("condition_after", "Normal")
	var condition_value = _convert_of_condition_to_godot(condition_str)
	PlayerData.condition = condition_value

	print(
		(
			"[TrainingManager] Applied results to PlayerData: Fatigue=%d, Condition=%d (%s)"
			% [PlayerData.fatigue, PlayerData.condition, condition_str]
		)
	)


func _apply_attribute_change(of_attr_name: String, change: int):
	"""Apply single OpenFootball attribute change to Godot PlayerData"""
	if change == 0:
		return  # No change

	# Map OpenFootball attr (36) → Godot attr (42)
	# Most have direct 1:1 mapping, some need conversion
	var godot_attr_name = of_attr_name
	var category = ""

	# Special name mappings (OF → Godot)
	match of_attr_name:
		"free_kick_taking":
			godot_attr_name = "free_kicks"
		"penalty_taking":
			godot_attr_name = "penalty_kicks"

	# Determine category
	if (
		of_attr_name
		in ["pace", "acceleration", "stamina", "strength", "agility", "balance", "jumping", "natural_fitness"]
	):
		category = "physical"
	elif (
		of_attr_name
		in [
			"aggression",
			"anticipation",
			"bravery",
			"composure",
			"concentration",
			"decisions",
			"determination",
			"flair",
			"leadership",
			"off_the_ball",
			"positioning",
			"teamwork",
			"vision",
			"work_rate"
		]
	):
		category = "mental"
	else:
		category = "technical"

	# Apply change
	var current_value = PlayerData.get_stat(category, godot_attr_name)
	var new_value = clamp(current_value + change, 0, 100)
	PlayerData.set_stat(category, godot_attr_name, new_value)

	if change > 0:
		print("[TrainingManager]   ⬆ %s: %d → %d (+%d)" % [godot_attr_name, current_value, new_value, change])
	elif change < 0:
		print("[TrainingManager]   ⬇ %s: %d → %d (%d)" % [godot_attr_name, current_value, new_value, change])


func _convert_of_condition_to_godot(of_condition: String) -> int:
	"""Convert OpenFootball condition string to Godot 1-5 scale"""
	match of_condition:
		"Terrible":
			return 1
		"Poor":
			return 2
		"Normal":
			return 3
		"Good":
			return 4
		"Excellent":
			return 5
		_:
			return 3  # Default Normal


func _execute_training_legacy(training_type: TrainingType, intensity: TrainingIntensity) -> Dictionary:
	"""
	Legacy training execution (fallback when OpenFootballAPI unavailable)
	Kept for backward compatibility and offline testing
	"""
	var result = {
		"training_name": get_training_name(training_type),
		"intensity_name": get_intensity_name(intensity),
		"xp_gained": 0.0,
		"fatigue": 0.0,
		"injury_risk": 0.0,
		"success": true
	}

	# 기본 XP 계산
	var base_xp = calculate_base_xp(training_type, intensity)

	# 훈련 레벨 보너스 적용
	var level = training_levels.get(training_type, 1)
	var level_bonus = 1.0 + (level - 1) * 0.05  # 레벨당 5% 보너스

	# 코치/멘토 보너스 적용
	var final_xp = base_xp * level_bonus * (coach_bonus + mentor_bonus)

	# 피로도 계산
	var fatigue_amount = calculate_fatigue(intensity)

	# 부상 위험 계산
	var injury_risk = calculate_injury_risk(intensity)

	result.xp_gained = final_xp
	result.fatigue = fatigue_amount
	result.injury_risk = injury_risk

	# PlayerData에 XP 적용
	if PlayerData:
		PlayerData.apply_training_xp(get_training_name(training_type), final_xp)
		PlayerData.add_fatigue(fatigue_amount)

	# 훈련 레벨 증가 (확률적)
	_try_level_up(training_type)

	training_completed.emit(result)
	# Event Bus로 피로도 변경 알림
	EventBus.emit("player_fatigue_changed", {"fatigue": fatigue_amount, "source": "training"})

	print("[TrainingManager] ⚠️ Legacy training: %s - XP: %.1f" % [result.training_name, result.xp_gained])
	return result


# ========================================
# Phase 6C: Rest API Integration
# ========================================


func execute_rest() -> Dictionary:
	"""
	Execute rest/recovery via OpenFootballAPI
	Replaces simple fatigue reduction with OpenFootball stamina recovery system
	"""
	# Get PlayerData in CorePlayer format
	if not PlayerData:
		push_error("[TrainingManager] PlayerData not available!")
		return {"success": false, "error": "PlayerData unavailable"}

	var core_player = PlayerData.to_core_player_format()

	# Execute rest via OpenFootballAPI
	var api = get_node_or_null("/root/OpenFootballAPI")
	if not api:
		push_error("[TrainingManager] OpenFootballAPI not available! Falling back to legacy rest.")
		return _execute_rest_legacy()

	var api_result = api.execute_rest(core_player)

	# Check for API errors
	if not api_result.success:
		push_error("[TrainingManager] Rest API failed: %s" % api_result.get("error", "Unknown error"))
		return {"success": false, "error": api_result.get("error", "API failure")}

	var response = api_result.response

	# Apply rest results to PlayerData
	var stamina_after = response.get("stamina_after", 100)
	var stamina_recovered = response.get("stamina_recovered", 0)
	var fatigue = 100 - stamina_after
	PlayerData.fatigue = clamp(fatigue, 0, 100)

	# Update condition
	var condition_str = response.get("condition_after", "Normal")
	var condition_value = _convert_of_condition_to_godot(condition_str)
	PlayerData.condition = condition_value

	# Build result dictionary
	var result = {
		"success": true,
		"stamina_before": response.get("stamina_before", 0),
		"stamina_after": stamina_after,
		"stamina_recovered": stamina_recovered,
		"fatigue_before": 100 - response.get("stamina_before", 100),
		"fatigue_after": fatigue,
		"condition_before": response.get("condition_before", "Normal"),
		"condition_after": condition_str
	}

	print(
		(
			"[TrainingManager] ✅ Rest via API: Stamina %d → %d (+%d) | Fatigue %d → %d | Condition: %s → %s"
			% [
				result.stamina_before,
				result.stamina_after,
				result.stamina_recovered,
				result.fatigue_before,
				result.fatigue_after,
				result.condition_before,
				result.condition_after
			]
		)
	)

	return result


func _execute_rest_legacy() -> Dictionary:
	"""Legacy rest execution (fallback when OpenFootballAPI unavailable)"""
	if not PlayerData:
		return {"success": false, "error": "PlayerData unavailable"}

	var fatigue_before = PlayerData.fatigue
	var condition_before = PlayerData.condition

	# Simple rest logic
	PlayerData.rest_player()  # Reduces fatigue by 20, increases condition by 1

	var result = {
		"success": true,
		"fatigue_before": fatigue_before,
		"fatigue_after": PlayerData.fatigue,
		"fatigue_recovered": fatigue_before - PlayerData.fatigue,
		"condition_before": condition_before,
		"condition_after": PlayerData.condition
	}

	print(
		(
			"[TrainingManager] ⚠️ Legacy rest: Fatigue %d → %d | Condition %d → %d"
			% [result.fatigue_before, result.fatigue_after, result.condition_before, result.condition_after]
		)
	)

	return result


# ========================================
# End Phase 6C Rest API Integration
# ========================================


func calculate_base_xp(training_type: TrainingType, intensity: TrainingIntensity) -> float:
	# 강도별 기본 XP
	var intensity_multiplier = get_intensity_multiplier(intensity)

	# 훈련 타입별 기본 XP (Balance)
	var base_xp = 100.0

	# 특정 훈련은 XP가 다를 수 있음
	match training_type:
		TrainingType.PHYSICAL_RECOVERY:
			base_xp = 50.0  # 회복 훈련은 XP가 적음
		TrainingType.MATCH_PREPARATION, TrainingType.MATCH_VIDEO_ANALYSIS, TrainingType.MATCH_OPPONENT_SPECIFIC:
			base_xp = 120.0  # 경기 준비는 XP가 높음
		_:
			base_xp = 100.0

	# Apply training efficiency from GameCache
	var efficiency_multiplier = _get_training_efficiency_from_cache(training_type)

	return base_xp * intensity_multiplier * efficiency_multiplier


func _get_training_efficiency_from_cache(training_type: TrainingType) -> float:
	"""Get training efficiency multiplier from GameCache based on training category."""
	# Get GameCache singleton
	if not has_node("/root/GameCache"):
		return 1.0  # No cache, use default multiplier

	var game_cache = get_node("/root/GameCache")
	if not game_cache.is_loaded:
		return 1.0  # Cache not loaded

	# Map TrainingType to training efficiency categories
	var category_key = ""
	if training_type >= TrainingType.PHYSICAL_ENDURANCE and training_type <= TrainingType.PHYSICAL_RECOVERY:
		category_key = "physical"
	elif training_type >= TrainingType.TECHNICAL_BALL_CONTROL and training_type <= TrainingType.TECHNICAL_SET_PIECES:
		category_key = "technical"
	elif (
		training_type >= TrainingType.TACTICAL_POSITIONING
		and training_type <= TrainingType.TACTICAL_SET_PIECES_DEFENSIVE
	):
		category_key = "tactical"
	elif training_type >= TrainingType.MENTAL_CONCENTRATION and training_type <= TrainingType.MENTAL_LEADERSHIP:
		category_key = "mental"
	else:
		category_key = "technical"  # Default fallback

	# Get efficiency data from cache
	var efficiency_data = game_cache.get_training_efficiency(category_key)
	if efficiency_data.is_empty():
		return 1.0  # No data, use default

	var base_improvement = efficiency_data.get("base_improvement", 0.05)
	var efficiency_multiplier = 1.0 + base_improvement

	print(
		(
			"[TrainingManager] 📊 Training efficiency for %s: %.1f%% (category: %s)"
			% [get_training_name(training_type), base_improvement * 100, category_key]
		)
	)

	return efficiency_multiplier


func get_intensity_multiplier(intensity: TrainingIntensity) -> float:
	match intensity:
		TrainingIntensity.VERY_LIGHT:
			return 0.3
		TrainingIntensity.LIGHT:
			return 0.5
		TrainingIntensity.MODERATE:
			return 0.7
		TrainingIntensity.HIGH:
			return 0.9
		TrainingIntensity.VERY_HIGH:
			return 1.0
		_:
			return 0.5


func calculate_fatigue(intensity: TrainingIntensity) -> float:
	match intensity:
		TrainingIntensity.VERY_LIGHT:
			return 5.0
		TrainingIntensity.LIGHT:
			return 10.0
		TrainingIntensity.MODERATE:
			return 20.0
		TrainingIntensity.HIGH:
			return 35.0
		TrainingIntensity.VERY_HIGH:
			return 50.0
		_:
			return 20.0


func calculate_injury_risk(intensity: TrainingIntensity) -> float:
	match intensity:
		TrainingIntensity.VERY_LIGHT:
			return 0.1
		TrainingIntensity.LIGHT:
			return 0.2
		TrainingIntensity.MODERATE:
			return 0.5
		TrainingIntensity.HIGH:
			return 1.0
		TrainingIntensity.VERY_HIGH:
			return 2.0
		_:
			return 0.5


func _try_level_up(training_type: TrainingType):
	# 훈련 레벨업 시도 (확률적)
	var current_level = training_levels.get(training_type, 1)

	if current_level >= 9:
		return  # 최대 레벨

	# 레벨이 높을수록 레벨업 확률 감소
	var level_up_chance = 0.1 - (current_level - 1) * 0.01  # 10%에서 시작, 레벨당 1% 감소

	if randf() < level_up_chance:
		training_levels[training_type] = current_level + 1
		print(
			"[TrainingManager] Level up! ",
			get_training_name(training_type),
			" -> Level ",
			training_levels[training_type]
		)


func get_training_name(training_type: TrainingType) -> String:
	match training_type:
		# Physical (5)
		TrainingType.PHYSICAL_ENDURANCE:
			return "Physical_Endurance"
		TrainingType.PHYSICAL_STRENGTH:
			return "Physical_Strength"
		TrainingType.PHYSICAL_SPEED:
			return "Physical_Speed"
		TrainingType.PHYSICAL_AGILITY:
			return "Physical_Agility"
		TrainingType.PHYSICAL_RECOVERY:
			return "Physical_Recovery"

		# Technical (5)
		TrainingType.TECHNICAL_BALL_CONTROL:
			return "Technical_BallControl"
		TrainingType.TECHNICAL_PASSING:
			return "Technical_Passing"
		TrainingType.TECHNICAL_SHOOTING:
			return "Technical_Shooting"
		TrainingType.TECHNICAL_CROSSING:
			return "Technical_Crossing"
		TrainingType.TECHNICAL_SET_PIECES:
			return "Technical_SetPieces"

		# Tactical (5)
		TrainingType.TACTICAL_POSITIONING:
			return "Tactical_Positioning"
		TrainingType.TACTICAL_TEAM_SHAPE:
			return "Tactical_TeamShape"
		TrainingType.TACTICAL_PRESSING_DRILLS:
			return "Tactical_PressingDrills"
		TrainingType.TACTICAL_TRANSITION_PLAY:
			return "Tactical_TransitionPlay"
		TrainingType.TACTICAL_SET_PIECES_DEFENSIVE:
			return "Tactical_SetPiecesDefensive"

		# Mental (3)
		TrainingType.MENTAL_CONCENTRATION:
			return "Mental_Concentration"
		TrainingType.MENTAL_DECISION_MAKING:
			return "Mental_DecisionMaking"
		TrainingType.MENTAL_LEADERSHIP:
			return "Mental_Leadership"

		# Match Preparation (3)
		TrainingType.MATCH_PREPARATION:
			return "Match_Preparation"
		TrainingType.MATCH_VIDEO_ANALYSIS:
			return "Match_VideoAnalysis"
		TrainingType.MATCH_OPPONENT_SPECIFIC:
			return "Match_OpponentSpecific"

		_:
			return "Unknown"


func get_intensity_name(intensity: TrainingIntensity) -> String:
	match intensity:
		TrainingIntensity.VERY_LIGHT:
			return "Very Light"
		TrainingIntensity.LIGHT:
			return "Light"
		TrainingIntensity.MODERATE:
			return "Moderate"
		TrainingIntensity.HIGH:
			return "High"
		TrainingIntensity.VERY_HIGH:
			return "Very High"
		_:
			return "Unknown"


func get_training_level(training_type: TrainingType) -> int:
	return training_levels.get(training_type, 1)


func set_coach_bonus(bonus: float):
	coach_bonus = bonus
	print("[TrainingManager] Coach bonus set to: ", coach_bonus)


func set_mentor_bonus(bonus: float):
	mentor_bonus = bonus
	print("[TrainingManager] Mentor bonus set to: ", mentor_bonus)


func get_all_training_levels() -> Dictionary:
	return training_levels.duplicate()


func get_training_efficiency(training_type: TrainingType) -> float:
	# 훈련 효율성 계산 (레벨 + 보너스)
	var level = training_levels.get(training_type, 1)
	var efficiency = 1.0 + (level - 1) * 0.05  # 레벨당 5% 증가
	return efficiency * (coach_bonus + mentor_bonus)


# ========================================
# Phase 6C: Training Recommendation System
# ========================================


func can_train() -> bool:
	"""
	Check if player can train today (via OpenFootball recommendation)
	Returns true if stamina/condition allows training
	"""
	if not PlayerData:
		return false

	var core_player = PlayerData.to_core_player_format()
	var api = get_node_or_null("/root/OpenFootballAPI")

	if not api:
		# Fallback: simple fatigue check
		return PlayerData.fatigue < 80.0

	var recommendation = api.get_training_recommendation(core_player)

	if not recommendation.success:
		return false  # If can't get recommendation, default to no training

	return recommendation.response.get("can_train", false)


func should_rest() -> bool:
	"""
	Check if player should rest today (via OpenFootball recommendation)
	Returns true if stamina/condition requires rest
	"""
	if not PlayerData:
		return true  # Safe default: rest if no data

	var core_player = PlayerData.to_core_player_format()
	var api = get_node_or_null("/root/OpenFootballAPI")

	if not api:
		# Fallback: simple fatigue check
		return PlayerData.fatigue >= 60.0

	var recommendation = api.get_training_recommendation(core_player)

	if not recommendation.success:
		return true  # Safe default: rest if can't get recommendation

	return recommendation.response.get("should_rest", false)


func get_recommended_intensity() -> String:
	"""
	Get recommended training intensity based on player condition
	Returns: "light", "normal", or "intensive"
	"""
	if not PlayerData:
		return "light"  # Safe default

	var core_player = PlayerData.to_core_player_format()
	var api = get_node_or_null("/root/OpenFootballAPI")

	if not api:
		# Fallback: intensity based on fatigue
		if PlayerData.fatigue >= 60.0:
			return "light"
		elif PlayerData.fatigue >= 30.0:
			return "normal"
		else:
			return "intensive"

	var recommendation = api.get_training_recommendation(core_player)

	if not recommendation.success:
		return "normal"  # Default to normal intensity

	return recommendation.response.get("recommended_intensity", "normal")


func get_training_recommendation() -> Dictionary:
	"""
	Get full training recommendation from OpenFootballAPI
	Returns complete recommendation including warnings and reasoning
	"""
	if not PlayerData:
		return {"success": false, "error": "PlayerData unavailable"}

	var core_player = PlayerData.to_core_player_format()
	var api = get_node_or_null("/root/OpenFootballAPI")

	if not api:
		# Fallback: basic recommendation
		return {
			"success": true,
			"can_train": PlayerData.fatigue < 80.0,
			"should_rest": PlayerData.fatigue >= 60.0,
			"recommended_intensity": get_recommended_intensity(),
			"reason": "Legacy system - simple fatigue check",
			"warnings": []
		}

	return api.get_training_recommendation(core_player)

# ========================================
# End Phase 6C Training Recommendation
# ========================================
