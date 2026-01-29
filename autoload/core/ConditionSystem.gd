extends Node

# 5-Level Condition System for Football Player Game v4.0
# Based on CLAUDE2.md specifications: 절호조→호조→보통→부진→절부진

signal condition_changed(new_condition: ConditionLevel, percentage: float)
signal condition_effect_applied(bonuses: Dictionary)
signal motivation_changed(new_motivation: MotivationLevel, percentage: float)

enum ConditionLevel { TERRIBLE = 1, POOR = 2, AVERAGE = 3, GOOD = 4, EXCELLENT = 5 }  # 절부진 (0-39%)  # 부진 (40-59%)  # 보통 (60-79%)  # 호조 (80-94%)  # 절호조 (95-100%)

enum MotivationLevel { DESPAIR = 1, LOW = 2, NORMAL = 3, HIGH = 4, PEAK = 5 }  # 절망 (0-19%)  # 낮음 (20-39%)  # 보통 (40-69%)  # 높음 (70-89%)  # 최고 (90-100%)

# Condition percentage (0-100)
var condition_percentage: float = 75.0:
	set(value):
		condition_percentage = clampf(value, 0.0, 100.0)
		var old_level = current_condition_level
		current_condition_level = _calculate_condition_level()

		if old_level != current_condition_level:
			condition_changed.emit(current_condition_level, condition_percentage)
			_apply_condition_effects()

var current_condition_level: ConditionLevel = ConditionLevel.AVERAGE

# Motivation percentage (0-100) - パワプロ やる気システム
var motivation_percentage: float = 60.0:
	set(value):
		motivation_percentage = clampf(value, 0.0, 100.0)
		var old_level = current_motivation_level
		current_motivation_level = _calculate_motivation_level()

		if old_level != current_motivation_level:
			motivation_changed.emit(current_motivation_level, motivation_percentage)
			_apply_condition_effects()

var current_motivation_level: MotivationLevel = MotivationLevel.NORMAL

# Daily condition changes tracking
var daily_condition_factors: Dictionary = {}
var daily_motivation_factors: Dictionary = {}
var last_condition_update: Dictionary = {}

# Condition effects on abilities and training
var condition_bonuses: Dictionary = {}


func _ready():
	current_condition_level = _calculate_condition_level()
	current_motivation_level = _calculate_motivation_level()
	_apply_condition_effects()


func get_condition_level() -> ConditionLevel:
	"""Get current condition level"""
	return current_condition_level


func get_condition_percentage() -> float:
	"""Get current condition percentage"""
	return condition_percentage


func get_condition_name() -> String:
	"""Get localized condition name"""
	match current_condition_level:
		ConditionLevel.EXCELLENT:
			return "절호조"
		ConditionLevel.GOOD:
			return "호조"
		ConditionLevel.AVERAGE:
			return "보통"
		ConditionLevel.POOR:
			return "부진"
		ConditionLevel.TERRIBLE:
			return "절부진"
		_:
			return "보통"


func get_condition_color() -> Color:
	"""Get condition display color"""
	match current_condition_level:
		ConditionLevel.EXCELLENT:
			return Color.RED  # 🔴 절호조
		ConditionLevel.GOOD:
			return Color.YELLOW  # 🟡 호조
		ConditionLevel.AVERAGE:
			return Color.WHITE  # ⚪ 보통
		ConditionLevel.POOR:
			return Color.BLUE  # 🔵 부진
		ConditionLevel.TERRIBLE:
			return Color.PURPLE  # 🟣 절부진
		_:
			return Color.WHITE


func get_ability_modifier() -> float:
	"""Get ability bonus/penalty based on condition"""
	match current_condition_level:
		ConditionLevel.EXCELLENT:
			return 1.15  # +15%
		ConditionLevel.GOOD:
			return 1.08  # +8%
		ConditionLevel.AVERAGE:
			return 1.0  # +0%
		ConditionLevel.POOR:
			return 0.9  # -10%
		ConditionLevel.TERRIBLE:
			return 0.8  # -20%
		_:
			return 1.0


func get_training_modifier() -> float:
	"""Get training effectiveness bonus/penalty based on condition"""
	match current_condition_level:
		ConditionLevel.EXCELLENT:
			return 1.30  # +30%
		ConditionLevel.GOOD:
			return 1.15  # +15%
		ConditionLevel.AVERAGE:
			return 1.0  # +0%
		ConditionLevel.POOR:
			return 0.8  # -20%
		ConditionLevel.TERRIBLE:
			return 0.6  # -40%
		_:
			return 1.0


func get_motivation_modifier() -> float:
	"""Get training effectiveness bonus/penalty based on motivation (やる気)"""
	match current_motivation_level:
		MotivationLevel.PEAK:
			return 1.5  # +50%
		MotivationLevel.HIGH:
			return 1.25  # +25%
		MotivationLevel.NORMAL:
			return 1.0  # +0%
		MotivationLevel.LOW:
			return 0.75  # -25%
		MotivationLevel.DESPAIR:
			return 0.5  # -50%
		_:
			return 1.0


func get_combined_training_modifier() -> float:
	"""Get combined training effectiveness (Condition × Motivation)"""
	var condition_mod = get_training_modifier()
	var motivation_mod = get_motivation_modifier()
	return condition_mod * motivation_mod


func apply_daily_change(factor_type: String, change_value: float, reason: String = "") -> void:
	"""Apply daily condition change based on various factors"""

	# Record the change
	if not daily_condition_factors.has(factor_type):
		daily_condition_factors[factor_type] = []

	daily_condition_factors[factor_type].append(
		{"value": change_value, "reason": reason, "timestamp": Time.get_unix_time_from_system()}
	)

	# Apply the change
	condition_percentage += change_value

	print(
		"[ConditionSystem] Applied %s: %+.1f (%s) -> %.1f%%" % [factor_type, change_value, reason, condition_percentage]
	)


# Condition change factors from CLAUDE2.md spec
func apply_rest_recovery(quality: String = "normal") -> void:
	"""휴식으로 인한 컨디션 회복 (+8)"""
	var recovery = 8.0
	match quality:
		"excellent":
			recovery = 12.0
		"good":
			recovery = 10.0
		"poor":
			recovery = 5.0

	apply_daily_change("rest", recovery, "휴식 회복")


func apply_light_training() -> void:
	"""가벼운 훈련으로 인한 컨디션 상승 (+3)"""
	apply_daily_change("light_training", 3.0, "가벼운 훈련")


func apply_team_meal() -> void:
	"""팀 식사로 인한 컨디션 상승 (+5)"""
	apply_daily_change("team_meal", 5.0, "팀 식사")


func apply_victory_bonus() -> void:
	"""승리로 인한 컨디션 상승 (+10)"""
	apply_daily_change("victory", 10.0, "경기 승리")


func apply_intense_training() -> void:
	"""고강도 훈련으로 인한 컨디션 하락 (-8)"""
	apply_daily_change("intense_training", -8.0, "고강도 훈련")


func apply_consecutive_training() -> void:
	"""연속 훈련으로 인한 컨디션 하락 (-5)"""
	apply_daily_change("consecutive_training", -5.0, "연속 훈련")


func apply_defeat_penalty() -> void:
	"""패배로 인한 컨디션 하락 (-10)"""
	apply_daily_change("defeat", -10.0, "경기 패배")


func apply_injury_penalty() -> void:
	"""부상으로 인한 컨디션 하락 (-15)"""
	apply_daily_change("injury", -15.0, "부상")


func apply_match_fatigue() -> void:
	"""경기 피로로 인한 컨디션 하락 (-10)"""
	apply_daily_change("match_fatigue", -10.0, "경기 피로")


# ===== MOTIVATION CHANGE FUNCTIONS (やる気) =====


func apply_motivation_change(factor_type: String, change_value: float, reason: String = "") -> void:
	"""Apply motivation change based on various factors"""
	if not daily_motivation_factors.has(factor_type):
		daily_motivation_factors[factor_type] = []

	daily_motivation_factors[factor_type].append(
		{"value": change_value, "reason": reason, "timestamp": Time.get_unix_time_from_system()}
	)

	motivation_percentage += change_value
	print(
		(
			"[ConditionSystem] Motivation %s: %+.1f (%s) -> %.1f%%"
			% [factor_type, change_value, reason, motivation_percentage]
		)
	)


func apply_victory_motivation_boost() -> void:
	"""승리로 인한 동기 상승 (+15)"""
	apply_motivation_change("victory", 15.0, "경기 승리")


func apply_defeat_motivation_drop() -> void:
	"""패배로 인한 동기 하락 (-10)"""
	apply_motivation_change("defeat", -10.0, "경기 패배")


func apply_rest_motivation_recovery() -> void:
	"""휴식으로 동기 회복 (+5)"""
	apply_motivation_change("rest", 5.0, "휴식 회복")


func apply_consecutive_training_motivation_drop() -> void:
	"""연속 훈련으로 동기 하락 (-3)"""
	apply_motivation_change("consecutive_training", -3.0, "연속 훈련")


func apply_friend_event_motivation_boost() -> void:
	"""친구 이벤트로 동기 상승 (+10)"""
	apply_motivation_change("friend_event", 10.0, "친구와 함께")


func apply_coach_praise_motivation_boost() -> void:
	"""감독 칭찬으로 동기 상승 (+8)"""
	apply_motivation_change("coach_praise", 8.0, "감독 칭찬")


func apply_injury_motivation_drop() -> void:
	"""부상으로 동기 하락 (-12)"""
	apply_motivation_change("injury", -12.0, "부상")


func reset_daily_factors() -> void:
	"""Reset daily condition factors (call at start of new day)"""
	daily_condition_factors.clear()
	daily_motivation_factors.clear()


func get_daily_summary() -> Dictionary:
	"""Get summary of today's condition and motivation changes"""
	var summary = {
		"condition":
		{
			"total_change": 0.0,
			"factors": [],
			"final_percentage": condition_percentage,
			"final_level": get_condition_name()
		},
		"motivation":
		{
			"total_change": 0.0,
			"factors": [],
			"final_percentage": motivation_percentage,
			"final_level": get_motivation_name()
		}
	}

	# Condition summary
	for factor_type in daily_condition_factors:
		for change in daily_condition_factors[factor_type]:
			summary.condition.total_change += change.value
			summary.condition.factors.append({"type": factor_type, "value": change.value, "reason": change.reason})

	# Motivation summary
	for factor_type in daily_motivation_factors:
		for change in daily_motivation_factors[factor_type]:
			summary.motivation.total_change += change.value
			summary.motivation.factors.append({"type": factor_type, "value": change.value, "reason": change.reason})

	return summary


func _calculate_condition_level() -> ConditionLevel:
	"""Calculate condition level based on percentage"""
	if condition_percentage >= 95.0:
		return ConditionLevel.EXCELLENT  # 절호조 (95-100%)
	elif condition_percentage >= 80.0:
		return ConditionLevel.GOOD  # 호조 (80-94%)
	elif condition_percentage >= 60.0:
		return ConditionLevel.AVERAGE  # 보통 (60-79%)
	elif condition_percentage >= 40.0:
		return ConditionLevel.POOR  # 부진 (40-59%)
	else:
		return ConditionLevel.TERRIBLE  # 절부진 (0-39%)


func _calculate_motivation_level() -> MotivationLevel:
	"""Calculate motivation level based on percentage"""
	if motivation_percentage >= 90.0:
		return MotivationLevel.PEAK  # 최고 (90-100%)
	elif motivation_percentage >= 70.0:
		return MotivationLevel.HIGH  # 높음 (70-89%)
	elif motivation_percentage >= 40.0:
		return MotivationLevel.NORMAL  # 보통 (40-69%)
	elif motivation_percentage >= 20.0:
		return MotivationLevel.LOW  # 낮음 (20-39%)
	else:
		return MotivationLevel.DESPAIR  # 절망 (0-19%)


func _apply_condition_effects() -> void:
	"""Apply condition and motivation effects to bonuses"""
	condition_bonuses = {
		"ability_modifier": get_ability_modifier(),
		"training_modifier": get_training_modifier(),
		"motivation_modifier": get_motivation_modifier(),
		"combined_training_modifier": get_combined_training_modifier(),
		"condition_name": get_condition_name(),
		"condition_color": get_condition_color(),
		"motivation_name": get_motivation_name(),
		"motivation_color": get_motivation_color()
	}

	condition_effect_applied.emit(condition_bonuses)


# Integration with existing player data system
func sync_with_player_data() -> void:
	"""Sync with EnhancedPlayerData condition system"""
	if not EnhancedPlayerData:
		return

	# Update EnhancedPlayerData's condition based on our percentage
	var legacy_condition = int(current_condition_level)
	if EnhancedPlayerData.has_method("set_condition"):
		EnhancedPlayerData.set_condition(legacy_condition)


func get_motivation_name() -> String:
	"""Get localized motivation name"""
	match current_motivation_level:
		MotivationLevel.PEAK:
			return "최고"
		MotivationLevel.HIGH:
			return "높음"
		MotivationLevel.NORMAL:
			return "보통"
		MotivationLevel.LOW:
			return "낮음"
		MotivationLevel.DESPAIR:
			return "절망"
		_:
			return "보통"


func get_motivation_color() -> Color:
	"""Get motivation display color"""
	match current_motivation_level:
		MotivationLevel.PEAK:
			return Color.GOLD  # 🟡 최고
		MotivationLevel.HIGH:
			return Color.LIGHT_GREEN  # 🟢 높음
		MotivationLevel.NORMAL:
			return Color.WHITE  # ⚪ 보통
		MotivationLevel.LOW:
			return Color.ORANGE  # 🟠 낮음
		MotivationLevel.DESPAIR:
			return Color.DARK_RED  # 🔴 절망
		_:
			return Color.WHITE


func get_condition_description() -> String:
	"""Get detailed condition description for UI"""
	var desc = "%s (%.1f%%)" % [get_condition_name(), condition_percentage]

	match current_condition_level:
		ConditionLevel.EXCELLENT:
			desc += "\n모든 능력치 +15%, 훈련 효과 +30%"
		ConditionLevel.GOOD:
			desc += "\n모든 능력치 +8%, 훈련 효과 +15%"
		ConditionLevel.AVERAGE:
			desc += "\n기본 능력치, 훈련 효과 +0%"
		ConditionLevel.POOR:
			desc += "\n모든 능력치 -10%, 훈련 효과 -20%"
		ConditionLevel.TERRIBLE:
			desc += "\n모든 능력치 -20%, 훈련 효과 -40%"

	return desc


func get_motivation_description() -> String:
	"""Get detailed motivation description for UI"""
	var desc = "%s (%.1f%%)" % [get_motivation_name(), motivation_percentage]

	match current_motivation_level:
		MotivationLevel.PEAK:
			desc += "\n훈련 효과 +50%, 최고 상태!"
		MotivationLevel.HIGH:
			desc += "\n훈련 효과 +25%, 의욕 높음"
		MotivationLevel.NORMAL:
			desc += "\n훈련 효과 +0%, 보통 상태"
		MotivationLevel.LOW:
			desc += "\n훈련 효과 -25%, 의욕 낮음"
		MotivationLevel.DESPAIR:
			desc += "\n훈련 효과 -50%, 절망 상태..."

	return desc


func get_combined_description() -> String:
	"""Get combined condition + motivation description"""
	var combined_mod = get_combined_training_modifier()
	var percentage_change = (combined_mod - 1.0) * 100.0

	return "컨디션: %s | 동기: %s\n훈련 효율: %.0f%%" % [get_condition_name(), get_motivation_name(), 100.0 + percentage_change]


# Simulation helpers for testing
func simulate_week_condition_changes() -> void:
	"""Simulate a week of condition changes for testing"""
	for day in range(7):
		var day_change = randf_range(-5.0, 8.0)  # Random daily change
		apply_daily_change("simulation", day_change, "시뮬레이션 day %d" % day)


func set_condition_for_testing(percentage: float) -> void:
	"""Set condition for testing purposes"""
	condition_percentage = percentage


func set_motivation_for_testing(percentage: float) -> void:
	"""Set motivation for testing purposes"""
	motivation_percentage = percentage


func get_motivation_level() -> MotivationLevel:
	"""Get current motivation level"""
	return current_motivation_level


func get_motivation_percentage() -> float:
	"""Get current motivation percentage"""
	return motivation_percentage
