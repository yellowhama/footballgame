extends Node
# class_name removed - this is an autoload singleton

# PlayerManager - 플레이어 데이터 관리 총괄 시스템 (CLAUDE2_IMPLEMENTATION.md 명세)

signal stats_changed(stats: Dictionary)
@warning_ignore("unused_signal")
signal condition_changed(condition: String)  # TODO: Implement condition change emission
signal level_up(stat_name: String, new_value: int)

# 플레이어 데이터 참조
var player_data: Node

# 성장 시스템 (Power Pro 스타일)
enum GrowthCurve { EARLY, BALANCED, LATE, FLUCTUATING }  # 조기형: 1-2학년 빠른 성장, 3학년 정체  # 균형형: 1-3학년 균등한 성장  # 후기형: 1-2학년 느린 성장, 3학년 폭발적 성장  # 변동형: 랜덤하게 성장 속도 변화

# 특수능력 시스템 (12가지)
enum SpecialAbility {
	# Technical (4가지)
	DRIBBLING_MASTER,  # 드리블 마스터
	PASSING_GENIUS,  # 패스 천재
	SHOOTING_STAR,  # 슈팅 스타
	SET_PIECE_SPECIALIST,  # 세트피스 전문가
	# Mental (4가지)
	CAPTAIN_MATERIAL,  # 주장 재질
	CLUTCH_PLAYER,  # 클러치 플레이어
	TEAM_PLAYER,  # 팀 플레이어
	PRESSURE_HANDLER,  # 압박 처리
	# Physical (4가지)
	SPEED_DEMON,  # 스피드 악마
	ENDURANCE_KING,  # 지구력 왕
	POWER_HOUSE,  # 파워하우스
	AGILITY_MASTER  # 민첩성 마스터
}

# 현재 성장 곡선
var growth_curve: GrowthCurve = GrowthCurve.BALANCED

# 보유 특수능력
var special_abilities: Array = []

# 성장 수정자
var growth_modifier: float = 1.0


func _ready():
	print("[PlayerManager] Initialized")

	# PlayerData 찾기 또는 생성
	if has_node("/root/PlayerData"):
		player_data = get_node("/root/PlayerData")
	else:
		# PlayerData가 없으면 로컬에서 찾기
		player_data = get_node_or_null("../PlayerData")

	if not player_data:
		push_error("[PlayerManager] PlayerData not found!")
		return

	# Connect to TrainingManager to apply attribute changes
	if TrainingManager:
		TrainingManager.player_attributes_changed.connect(_on_player_attributes_changed)
		print("[PlayerManager] ✅ Connected to TrainingManager for attribute updates.")
	else:
		push_warning("[PlayerManager] ⚠️ TrainingManager not found. Attribute updates will not be applied.")

	# 초기 성장 곡선 설정
	_set_random_growth_curve()

	print("[PlayerManager] PlayerData connected: ", player_data.name)


func _on_player_attributes_changed(changes: Dictionary):
	"""Applies attribute changes received from the TrainingManager."""
	if not player_data:
		push_error("[PlayerManager] Cannot apply attribute changes: PlayerData is not available.")
		return

	if not player_data.has_method("increase_attribute"):
		push_error("[PlayerManager] PlayerData node is missing the 'increase_attribute' method. Cannot apply changes.")
		return

	print("[PlayerManager] Applying %d attribute changes from training." % changes.size())
	for attr in changes:
		var delta = changes[attr]
		player_data.increase_attribute(attr, delta)
		print("  - Applied change: %s %+d" % [attr, delta])

	# Emit stats_changed signal to notify UI and other systems
	stats_changed.emit(player_data.get_all_stats())


func create_player(data: Dictionary):
	# 새 플레이어 생성 (포지션별 초기 능력치)
	if not player_data:
		return

	var position = data.get("position", "ST")
	player_data.position = position

	# 포지션별 초기 능력치 설정
	_set_position_stats(position)

	# 성장 곡선 설정
	if data.has("growth_curve"):
		growth_curve = data.growth_curve
	else:
		_set_random_growth_curve()

	stats_changed.emit(player_data.get_all_stats())
	print("[PlayerManager] Player created: ", position, " Growth: ", _get_growth_curve_name())


func _set_position_stats(position: String):
	# 포지션별 초기 능력치 보너스
	match position:
		"ST":  # 스트라이커
			player_data.set_stat("technical", "finishing", 25)
			player_data.set_stat("technical", "heading", 20)
			player_data.set_stat("physical", "pace", 20)
		"MF":  # 미드필더
			player_data.set_stat("technical", "passing", 25)
			player_data.set_stat("mental", "vision", 20)
			player_data.set_stat("mental", "teamwork", 20)
		"DF":  # 수비수
			player_data.set_stat("technical", "marking", 25)
			player_data.set_stat("technical", "tackling", 20)
			player_data.set_stat("mental", "positioning", 20)
		"GK":  # 골키퍼
			player_data.set_stat("goalkeeper", "handling", 25)
			player_data.set_stat("goalkeeper", "command_of_area", 20)
			player_data.set_stat("mental", "concentration", 20)


func _set_random_growth_curve():
	# 랜덤 성장 곡선 설정
	var curves = [GrowthCurve.EARLY, GrowthCurve.BALANCED, GrowthCurve.LATE, GrowthCurve.FLUCTUATING]
	growth_curve = curves[randi() % curves.size()]


func _get_growth_curve_name() -> String:
	match growth_curve:
		GrowthCurve.EARLY:
			return "Early"
		GrowthCurve.BALANCED:
			return "Balanced"
		GrowthCurve.LATE:
			return "Late"
		GrowthCurve.FLUCTUATING:
			return "Fluctuating"
		_:
			return "Unknown"


func apply_training_xp(training_type: String, base_xp: float):
	# 훈련 XP 적용 (성장 곡선 고려)
	if not player_data:
		return

	# 성장 곡선에 따른 XP 수정자
	var curve_modifier = _get_growth_modifier()

	# 최종 XP 계산
	var final_xp = base_xp * curve_modifier * growth_modifier

	# PlayerData에 적용
	var old_stats = player_data.get_all_stats().duplicate(true)
	player_data.apply_training_xp(training_type, final_xp)
	var new_stats = player_data.get_all_stats()

	# 특수능력 체크
	_check_special_abilities()

	# 변화된 스탯 체크
	_check_stat_changes(old_stats, new_stats)

	stats_changed.emit(new_stats)


func _get_growth_modifier() -> float:
	# 성장 곡선에 따른 수정자 (현재 학년 고려)
	var current_year = player_data.current_year

	match growth_curve:
		GrowthCurve.EARLY:
			match current_year:
				1:
					return 1.5  # 1학년 50% 보너스
				2:
					return 1.2  # 2학년 20% 보너스
				3:
					return 0.8  # 3학년 20% 감소
		GrowthCurve.BALANCED:
			return 1.0  # 균등한 성장
		GrowthCurve.LATE:
			match current_year:
				1:
					return 0.7  # 1학년 30% 감소
				2:
					return 0.9  # 2학년 10% 감소
				3:
					return 1.5  # 3학년 50% 보너스
		GrowthCurve.FLUCTUATING:
			return randf_range(0.5, 1.5)  # 랜덤 변동

	return 1.0


func _check_special_abilities():
	# 특수능력 해금 조건 체크
	if not player_data:
		return

	# 드리블 마스터 (드리블링 80+, 민첩성 70+)
	if not SpecialAbility.DRIBBLING_MASTER in special_abilities:
		if player_data.get_stat("technical", "dribbling") >= 80 and player_data.get_stat("physical", "agility") >= 70:
			_unlock_special_ability(SpecialAbility.DRIBBLING_MASTER)

	# 패스 천재 (패스 85+, 시야 80+)
	if not SpecialAbility.PASSING_GENIUS in special_abilities:
		if player_data.get_stat("technical", "passing") >= 85 and player_data.get_stat("mental", "vision") >= 80:
			_unlock_special_ability(SpecialAbility.PASSING_GENIUS)

	# 슈팅 스타 (골 결정력 85+, 중거리 슛 75+)
	if not SpecialAbility.SHOOTING_STAR in special_abilities:
		if (
			player_data.get_stat("technical", "finishing") >= 85
			and player_data.get_stat("technical", "long_shots") >= 75
		):
			_unlock_special_ability(SpecialAbility.SHOOTING_STAR)

	# 주장 재질 (리더십 80+, 팀워크 75+, 결단력 70+)
	if not SpecialAbility.CAPTAIN_MATERIAL in special_abilities:
		if (
			player_data.get_stat("mental", "leadership") >= 80
			and player_data.get_stat("mental", "teamwork") >= 75
			and player_data.get_stat("mental", "determination") >= 70
		):
			_unlock_special_ability(SpecialAbility.CAPTAIN_MATERIAL)

	# 스피드 악마 (스피드 90+, 가속력 85+)
	if not SpecialAbility.SPEED_DEMON in special_abilities:
		if player_data.get_stat("physical", "pace") >= 90 and player_data.get_stat("physical", "acceleration") >= 85:
			_unlock_special_ability(SpecialAbility.SPEED_DEMON)


func _unlock_special_ability(ability: SpecialAbility):
	# 특수능력 해금
	special_abilities.append(ability)
	var ability_name = _get_special_ability_name(ability)
	print("[PlayerManager] Special Ability Unlocked: ", ability_name)

	# 특수능력 효과 적용
	_apply_special_ability_effect(ability)


func _get_special_ability_name(ability: SpecialAbility) -> String:
	match ability:
		SpecialAbility.DRIBBLING_MASTER:
			return "Dribbling Master"
		SpecialAbility.PASSING_GENIUS:
			return "Passing Genius"
		SpecialAbility.SHOOTING_STAR:
			return "Shooting Star"
		SpecialAbility.SET_PIECE_SPECIALIST:
			return "Set Piece Specialist"
		SpecialAbility.CAPTAIN_MATERIAL:
			return "Captain Material"
		SpecialAbility.CLUTCH_PLAYER:
			return "Clutch Player"
		SpecialAbility.TEAM_PLAYER:
			return "Team Player"
		SpecialAbility.PRESSURE_HANDLER:
			return "Pressure Handler"
		SpecialAbility.SPEED_DEMON:
			return "Speed Demon"
		SpecialAbility.ENDURANCE_KING:
			return "Endurance King"
		SpecialAbility.POWER_HOUSE:
			return "Power House"
		SpecialAbility.AGILITY_MASTER:
			return "Agility Master"
		_:
			return "Unknown"


func _apply_special_ability_effect(ability: SpecialAbility):
	# 특수능력 효과 적용 (성장 수정자 증가 등)
	match ability:
		SpecialAbility.DRIBBLING_MASTER:
			growth_modifier += 0.1  # 10% 성장 보너스
		SpecialAbility.CAPTAIN_MATERIAL:
			growth_modifier += 0.15  # 15% 성장 보너스
		SpecialAbility.SPEED_DEMON:
			growth_modifier += 0.1
		_:
			growth_modifier += 0.05  # 기본 5% 보너스


func _check_stat_changes(old_stats: Dictionary, new_stats: Dictionary):
	# 스탯 변화 체크 및 레벨업 알림
	for category in new_stats:
		var old_category = old_stats.get(category, {})
		var new_category = new_stats[category]

		for stat_name in new_category:
			var old_value = old_category.get(stat_name, 0)
			var new_value = new_category[stat_name]

			if new_value > old_value:
				# 10단위로 레벨업 시 알림
				if int(float(new_value) / 10.0) > int(float(old_value) / 10.0):
					level_up.emit(stat_name, new_value)


func get_overall_rating() -> int:
	if player_data:
		return player_data.get_overall_rating()
	return 0


func get_grade(value: float) -> String:
	if player_data:
		return player_data.get_grade(value)
	return "G"


func get_special_abilities() -> Array:
	return special_abilities


func get_growth_curve() -> GrowthCurve:
	return growth_curve


func set_growth_modifier(modifier: float):
	growth_modifier = modifier


# ===============================================================================
# Phase 20 Phase 2: Position별 Attribute Priority (CA → Attributes 분배용)
# ===============================================================================

const POSITION_PRIORITIES = {
	"ST":
	[
		{"name": "finishing", "weight": 0.35},
		{"name": "positioning", "weight": 0.25},
		{"name": "pace", "weight": 0.20},
		{"name": "composure", "weight": 0.10},
		{"name": "off_the_ball", "weight": 0.10}
	],
	"CF":
	[
		{"name": "finishing", "weight": 0.30},
		{"name": "first_touch", "weight": 0.20},
		{"name": "dribbling", "weight": 0.15},
		{"name": "passing", "weight": 0.15},
		{"name": "positioning", "weight": 0.20}
	],
	"CM":
	[
		{"name": "passing", "weight": 0.30},
		{"name": "vision", "weight": 0.25},
		{"name": "stamina", "weight": 0.15},
		{"name": "work_rate", "weight": 0.15},
		{"name": "teamwork", "weight": 0.15}
	],
	"CB":
	[
		{"name": "tackling", "weight": 0.30},
		{"name": "marking", "weight": 0.25},
		{"name": "positioning", "weight": 0.20},
		{"name": "strength", "weight": 0.15},
		{"name": "heading", "weight": 0.10}
	],
	"LB":
	[
		{"name": "tackling", "weight": 0.25},
		{"name": "marking", "weight": 0.20},
		{"name": "pace", "weight": 0.20},
		{"name": "stamina", "weight": 0.20},
		{"name": "crossing", "weight": 0.15}
	],
	"RB":
	[
		{"name": "tackling", "weight": 0.25},
		{"name": "marking", "weight": 0.20},
		{"name": "pace", "weight": 0.20},
		{"name": "stamina", "weight": 0.20},
		{"name": "crossing", "weight": 0.15}
	],
	"LM":
	[
		{"name": "crossing", "weight": 0.30},
		{"name": "dribbling", "weight": 0.25},
		{"name": "pace", "weight": 0.20},
		{"name": "stamina", "weight": 0.15},
		{"name": "vision", "weight": 0.10}
	],
	"RM":
	[
		{"name": "crossing", "weight": 0.30},
		{"name": "dribbling", "weight": 0.25},
		{"name": "pace", "weight": 0.20},
		{"name": "stamina", "weight": 0.15},
		{"name": "vision", "weight": 0.10}
	],
	"GK":
	[
		{"name": "first_touch", "weight": 0.30},  # Catching proxy
		{"name": "concentration", "weight": 0.25},
		{"name": "anticipation", "weight": 0.20},
		{"name": "positioning", "weight": 0.15},
		{"name": "agility", "weight": 0.10}
	]
}


func apply_ca_growth(amount: float) -> void:
	"""Apply CA (Current Ability) growth from match experience

	Called by MatchSimulationManager after each match to apply CA growth
	based on performance and match result.

	Args:
		amount: CA growth amount (typically 0.5-3.0 per match)
			- Base growth: 0.7
			- Performance multiplier: rating/5.0 (0.0-2.0x)
			- Result bonus: Win +0.3, Draw +0.1, Loss 0

	Growth is constrained by:
		- PA (Potential Ability) upper limit
		- Current year growth curve modifier
	"""
	if not player_data:
		push_warning("[PlayerManager] Cannot apply CA growth: PlayerData not found")
		return

	# Get current CA
	var current_ca = 0
	if player_data.has_method("get_ca"):
		current_ca = player_data.get_ca()
	else:
		# Fallback: calculate from overall rating
		current_ca = get_overall_rating() * 2

	# Get PA limit
	var pa_limit = 0
	if "potential" in player_data:
		pa_limit = player_data.potential
	else:
		pa_limit = 200  # Default high potential

	# Apply growth curve modifier
	var curve_modifier = _get_growth_modifier()
	var final_growth = amount * curve_modifier * growth_modifier

	# Calculate new CA (capped at PA)
	var new_ca = min(current_ca + final_growth, pa_limit)
	var actual_growth = new_ca - current_ca

	# ✅ NEW: Distribute CA growth to attributes (Phase 20 Phase 2)
	var player_position = player_data.position if "position" in player_data else "CM"
	var attr_distribution = _distribute_ca_growth_to_attributes(actual_growth, player_position)

	for attr_name in attr_distribution:
		var growth_amount = attr_distribution[attr_name]
		if growth_amount > 0:
			player_data.increase_attribute(attr_name, growth_amount)

	# Update CA (will be recalculated from attributes if method exists)
	if player_data.has_method("_recalculate_openfootball_ca"):
		player_data._recalculate_openfootball_ca()
	elif player_data.has_method("set_ca"):
		player_data.set_ca(new_ca)
	elif "current_ability" in player_data:
		player_data.current_ability = new_ca
	elif "current_ca" in player_data:
		player_data.current_ca = new_ca
	else:
		push_warning("[PlayerManager] PlayerData does not have CA setter")

	# Log growth
	print("[PlayerManager] 📊 CA Growth Applied:")
	print("  Base amount: +%.2f" % amount)
	print("  Curve modifier: %.2fx (%s)" % [curve_modifier, _get_growth_curve_name()])
	print("  Growth modifier: %.2fx" % growth_modifier)
	print("  Final growth: +%.2f (%.0f → %.0f)" % [actual_growth, current_ca, new_ca])
	print("  Attributes changed: %s" % attr_distribution)

	if new_ca >= pa_limit:
		print("  ⚠️ PA limit reached (%.0f)" % pa_limit)

	# Emit stats changed signal
	stats_changed.emit(player_data.get_all_stats())


func _distribute_ca_growth_to_attributes(ca_growth: float, position: String) -> Dictionary:
	"""Distribute CA growth to position-relevant attributes (Phase 20 Phase 2)

	Args:
		ca_growth: CA growth amount (0.5-3.0)
		position: Player position (ST, CM, CB, etc.)

	Returns:
		Dictionary of {attribute_name: growth_amount}
		E.g., {"finishing": 1, "positioning": 1, "pace": 0}
	"""
	var distribution = {}
	var priorities = POSITION_PRIORITIES.get(position, POSITION_PRIORITIES.get("CM", []))

	# Distribute growth proportionally
	for priority in priorities:
		var attr_name = priority["name"]
		var weight = priority["weight"]
		var growth_points = floor(ca_growth * weight)

		if growth_points > 0:
			distribution[attr_name] = int(growth_points)

	return distribution


func get_ability_info() -> Dictionary:
	"""CA/PA 정보 반환 (GameManager용)"""
	var info = {}

	if player_data and player_data.has_method("get_ca"):
		info["ca"] = player_data.get_ca()
	else:
		# 폴백: overall rating 기반 계산
		info["ca"] = get_overall_rating() * 2

	# PA는 potential 속성 사용
	if player_data and "potential" in player_data:
		info["pa"] = player_data.potential
	else:
		# 기본 potential 값
		info["pa"] = 80

	return info
