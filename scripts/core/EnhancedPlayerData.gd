extends Node

# Enhanced Player Data Singleton
# Manages the 42-skill player state and provides UI access

signal stats_changed
signal fatigue_changed(new_value: float)
signal condition_changed(new_value: int)
signal injury_changed(days: int)

# Player basic info
var player_name: String = "Player"
var player_position: String = "ST"
var player_year: int = 1
var player_week: int = 1

# Growth curve system (Power Pro style)
enum GrowthCurve { EARLY, BALANCED, LATE, FLUCTUATING }  # 1-2학년 빠른 성장, 3학년 정체  # 1-3학년 균등한 성장  # 1-2학년 느린 성장, 3학년 폭발적 성장  # 랜덤하게 성장 속도 변화

var growth_curve: GrowthCurve = GrowthCurve.BALANCED

# 42-skill system
var skills: Dictionary = {}

# Player status
var fatigue: float = 30.0:
	set(value):
		fatigue = clampf(value, 0.0, 100.0)
		condition = _calculate_condition()
		fatigue_changed.emit(fatigue)
		# Update new condition system based on fatigue
		_sync_condition_system()

var condition: int = 4:
	set(value):
		condition = clampi(value, 1, 5)
		condition_changed.emit(condition)

var injury_days: int = 0:
	set(value):
		injury_days = maxi(0, value)
		injury_changed.emit(injury_days)

var experience: int = 0
var potential: int = 80

# Hexagon stats (6-hexagon display)
var pace_stat: float = 50.0
var power_stat: float = 50.0
var technical_stat: float = 50.0
var shooting_stat: float = 50.0
var passing_stat: float = 50.0
var defending_stat: float = 50.0


func _ready():
	# Initialize with default 42 skills
	_initialize_skills()

	# Connect to Bridge for state updates
	if has_node("/root/Bridge"):
		var bridge = get_node("/root/Bridge")
		bridge.state_changed.connect(_on_bridge_state_changed)

	# Initialize condition system integration
	_connect_condition_system()


func _initialize_skills():
	"""Initialize all 42 skills with default values"""

	# Technical skills (14)
	var technical_skills = [
		"corners",
		"crossing",
		"dribbling",
		"finishing",
		"first_touch",
		"free_kicks",
		"heading",
		"long_shots",
		"long_throws",
		"marking",
		"passing",
		"penalty_taking",
		"tackling",
		"technique"
	]

	for skill in technical_skills:
		skills[skill] = 50.0

	# Mental skills (14)
	var mental_skills = [
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

	for skill in mental_skills:
		skills[skill] = 50.0

	# Physical skills (8)
	var physical_skills = [
		"acceleration", "agility", "balance", "jumping", "natural_fitness", "pace", "stamina", "strength"
	]

	for skill in physical_skills:
		skills[skill] = 50.0

	# Goalkeeper skills (10)
	var gk_skills = [
		"aerial_reach",
		"command_of_area",
		"communication",
		"eccentricity",
		"handling",
		"kicking",
		"one_on_ones",
		"reflexes",
		"rushing_out",
		"throwing"
	]

	for skill in gk_skills:
		skills[skill] = 30.0  # Lower default for non-GK


func get_skill(skill_name: String) -> float:
	"""Get a specific skill value"""
	return skills.get(skill_name, 0.0)


func set_skill(skill_name: String, value: float) -> void:
	"""Set a specific skill value"""
	if skills.has(skill_name):
		skills[skill_name] = clampf(value, 0.0, 100.0)
		stats_changed.emit()


func add_skill_delta(skill_name: String, delta: float) -> void:
	"""Add delta to a skill"""
	if skills.has(skill_name):
		set_skill(skill_name, skills[skill_name] + delta)


func get_skill_category(skill_name: String) -> String:
	"""Get the category of a skill"""
	var categories = {
		"Technical":
		[
			"corners",
			"crossing",
			"dribbling",
			"finishing",
			"first_touch",
			"free_kicks",
			"heading",
			"long_shots",
			"long_throws",
			"marking",
			"passing",
			"penalty_taking",
			"tackling",
			"technique"
		],
		"Mental":
		[
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
		],
		"Physical": ["acceleration", "agility", "balance", "jumping", "natural_fitness", "pace", "stamina", "strength"],
		"Goalkeeper":
		[
			"aerial_reach",
			"command_of_area",
			"communication",
			"eccentricity",
			"handling",
			"kicking",
			"one_on_ones",
			"reflexes",
			"rushing_out",
			"throwing"
		]
	}

	for category in categories:
		if skill_name in categories[category]:
			return category

	return "Unknown"


func get_skills_by_category(category: String) -> Dictionary:
	"""Get all skills in a category"""
	var result = {}

	var category_skills = []
	match category:
		"Technical":
			category_skills = [
				"corners",
				"crossing",
				"dribbling",
				"finishing",
				"first_touch",
				"free_kicks",
				"heading",
				"long_shots",
				"long_throws",
				"marking",
				"passing",
				"penalty_taking",
				"tackling",
				"technique"
			]
		"Mental":
			category_skills = [
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
		"Physical":
			category_skills = [
				"acceleration", "agility", "balance", "jumping", "natural_fitness", "pace", "stamina", "strength"
			]
		"Goalkeeper":
			category_skills = [
				"aerial_reach",
				"command_of_area",
				"communication",
				"eccentricity",
				"handling",
				"kicking",
				"one_on_ones",
				"reflexes",
				"rushing_out",
				"throwing"
			]

	for skill in category_skills:
		result[skill] = skills.get(skill, 0.0)

	return result


func get_category_average(category: String) -> float:
	"""Calculate average value for a skill category"""
	var cat_skills = get_skills_by_category(category)
	if cat_skills.is_empty():
		return 0.0

	var total = 0.0
	for skill in cat_skills:
		total += cat_skills[skill]

	return total / cat_skills.size()


func get_overall_rating() -> int:
	"""Calculate overall player rating based on position"""
	var weights = {}

	# Position-specific weights
	match player_position:
		"ST":
			weights = {
				"finishing": 0.15,
				"heading": 0.1,
				"pace": 0.1,
				"acceleration": 0.08,
				"positioning": 0.1,
				"off_the_ball": 0.08,
				"composure": 0.07,
				"dribbling": 0.06,
				"first_touch": 0.06,
				"strength": 0.05
			}
		"CAM":
			weights = {
				"passing": 0.15,
				"vision": 0.12,
				"technique": 0.1,
				"dribbling": 0.1,
				"first_touch": 0.08,
				"decisions": 0.08,
				"flair": 0.07,
				"composure": 0.06,
				"agility": 0.06,
				"long_shots": 0.05
			}
		"CM":
			weights = {
				"passing": 0.12,
				"stamina": 0.1,
				"positioning": 0.1,
				"decisions": 0.08,
				"vision": 0.08,
				"work_rate": 0.08,
				"tackling": 0.07,
				"first_touch": 0.06,
				"composure": 0.06,
				"teamwork": 0.06
			}
		"CB":
			weights = {
				"marking": 0.15,
				"tackling": 0.12,
				"positioning": 0.1,
				"heading": 0.1,
				"strength": 0.08,
				"jumping": 0.08,
				"concentration": 0.07,
				"anticipation": 0.07,
				"bravery": 0.06,
				"composure": 0.05
			}
		"GK":
			weights = {
				"handling": 0.15,
				"reflexes": 0.15,
				"one_on_ones": 0.12,
				"positioning": 0.1,
				"command_of_area": 0.08,
				"aerial_reach": 0.08,
				"communication": 0.06,
				"kicking": 0.06,
				"concentration": 0.05,
				"anticipation": 0.05
			}
		_:  # Default balanced
			weights = {
				"passing": 0.1,
				"dribbling": 0.08,
				"finishing": 0.08,
				"stamina": 0.08,
				"pace": 0.08,
				"positioning": 0.08,
				"decisions": 0.08,
				"first_touch": 0.08,
				"tackling": 0.06,
				"strength": 0.06
			}

	var total = 0.0
	var weight_sum = 0.0

	for skill_name in weights:
		if skills.has(skill_name):
			total += skills[skill_name] * weights[skill_name]
			weight_sum += weights[skill_name]

	# Add remaining skills with lower weight
	var remaining_weight = (1.0 - weight_sum) / max(1, skills.size() - weights.size())
	for skill_name in skills:
		if not weights.has(skill_name):
			total += skills[skill_name] * remaining_weight

	return int(total)


func get_skill_grade(value: float) -> String:
	"""Convert skill value to letter grade (Power Pro style: F(1-20) ~ S(90-100))"""
	if value >= 90:
		return "S"
	elif value >= 80:
		return "A"
	elif value >= 70:
		return "B"
	elif value >= 60:
		return "C"
	elif value >= 50:
		return "D"
	elif value >= 30:
		return "E"
	elif value >= 20:
		return "F"
	else:
		return "F-"


func _calculate_condition() -> int:
	"""Calculate condition based on fatigue"""
	if fatigue < 20:
		return 5
	elif fatigue < 40:
		return 4
	elif fatigue < 60:
		return 3
	elif fatigue < 80:
		return 2
	else:
		return 1


func calculate_growth_modifier(_week: int) -> float:
	"""Calculate growth modifier based on growth curve and current week"""
	var total_weeks = player_year * 52 + player_week

	match growth_curve:
		GrowthCurve.EARLY:
			# 1-2학년 빠른 성장, 3학년 정체
			if total_weeks <= 104:  # 1-2학년
				return 1.5
			else:  # 3학년
				return 0.8
		GrowthCurve.BALANCED:
			# 1-3학년 균등한 성장
			return 1.0
		GrowthCurve.LATE:
			# 1-2학년 느린 성장, 3학년 폭발적 성장
			if total_weeks <= 104:  # 1-2학년
				return 0.7
			else:  # 3학년
				return 1.3
		GrowthCurve.FLUCTUATING:
			# 랜덤하게 성장 속도 변화
			return randf_range(0.8, 1.2)
		_:
			return 1.0


func get_growth_curve_name() -> String:
	"""Get human-readable growth curve name"""
	match growth_curve:
		GrowthCurve.EARLY:
			return "조기형"
		GrowthCurve.BALANCED:
			return "균형형"
		GrowthCurve.LATE:
			return "후기형"
		GrowthCurve.FLUCTUATING:
			return "변동형"
		_:
			return "알 수 없음"


func set_growth_curve(curve: GrowthCurve) -> void:
	"""Set growth curve"""
	growth_curve = curve
	stats_changed.emit()


func _on_bridge_state_changed():
	"""Handle state changes from Bridge"""
	var bridge = get_node("/root/Bridge")
	var player_data = bridge.get_player_data()

	if player_data.has("skills"):
		skills = player_data["skills"].duplicate()
		stats_changed.emit()

	if player_data.has("fatigue"):
		fatigue = player_data["fatigue"]

	if player_data.has("injury_days"):
		injury_days = player_data["injury_days"]

	if player_data.has("name"):
		player_name = player_data["name"]

	if player_data.has("position"):
		player_position = player_data["position"]


func apply_training_result(result: Dictionary):
	"""Apply training results to player"""
	# Apply skill deltas
	if result.has("deltas"):
		for skill_name in result.deltas:
			add_skill_delta(skill_name, result.deltas[skill_name])

	# Apply fatigue
	if result.has("fatigue_delta"):
		fatigue += result.fatigue_delta

	# Check for injury
	if result.has("injury") and result.injury:
		injury_days = result.get("injury_days", 7)


func save_to_dictionary() -> Dictionary:
	"""Export player data as dictionary"""
	return {
		"name": player_name,
		"position": player_position,
		"year": player_year,
		"week": player_week,
		"growth_curve": growth_curve,
		"skills": skills.duplicate(),
		"fatigue": fatigue,
		"condition": condition,
		"injury_days": injury_days,
		"experience": experience,
		"potential": potential
	}


func load_from_dictionary(data: Dictionary):
	"""Load player data from dictionary"""
	player_name = data.get("name", "Player")
	player_position = data.get("position", "ST")
	player_year = data.get("year", 1)
	player_week = data.get("week", 1)
	growth_curve = data.get("growth_curve", GrowthCurve.BALANCED)
	skills = data.get("skills", {}).duplicate()
	fatigue = data.get("fatigue", 30.0)
	injury_days = data.get("injury_days", 0)
	experience = data.get("experience", 0)
	potential = data.get("potential", 80)

	# Ensure all 42 skills exist
	if skills.size() < 42:
		_initialize_skills()
		# Overlay loaded skills
		for skill_name in data.get("skills", {}):
			skills[skill_name] = data["skills"][skill_name]

	stats_changed.emit()


# ========== Condition System Integration ==========


func _connect_condition_system() -> void:
	"""Connect to the new ConditionSystem"""
	# Wait for ConditionSystem to be ready
	await get_tree().process_frame

	var condition_system = get_node_or_null("/root/ConditionSystem")
	if condition_system:
		condition_system.condition_changed.connect(_on_condition_system_changed)
		_sync_condition_system()


func _sync_condition_system() -> void:
	"""Sync condition system with current fatigue"""
	var condition_system = get_node_or_null("/root/ConditionSystem")
	if not condition_system:
		return

	# Convert fatigue to condition percentage (inverted)
	var condition_percentage = 100.0 - fatigue
	condition_system.condition_percentage = condition_percentage


func _on_condition_system_changed(new_condition_level, percentage: float) -> void:
	"""Handle condition system changes"""
	# Update legacy condition variable for backward compatibility
	condition = int(new_condition_level)

	# Update fatigue based on condition (inverted)
	var new_fatigue = 100.0 - percentage
	if abs(fatigue - new_fatigue) > 1.0:  # Only update if significant change
		fatigue = new_fatigue


func get_condition_modifier() -> float:
	"""Get condition modifier for abilities"""
	var condition_system = get_node_or_null("/root/ConditionSystem")
	if condition_system:
		return condition_system.get_ability_modifier()
	else:
		# Fallback to legacy system
		return _legacy_condition_modifier()


func get_training_condition_modifier() -> float:
	"""Get condition modifier for training effectiveness"""
	var condition_system = get_node_or_null("/root/ConditionSystem")
	if condition_system:
		return condition_system.get_training_modifier()
	else:
		# Fallback to legacy system
		return _legacy_training_modifier()


func _legacy_condition_modifier() -> float:
	"""Legacy condition modifier for backward compatibility"""
	match condition:
		5:
			return 1.15
		4:
			return 1.08
		3:
			return 1.0
		2:
			return 0.9
		1:
			return 0.8
		_:
			return 1.0


func _legacy_training_modifier() -> float:
	"""Legacy training modifier for backward compatibility"""
	match condition:
		5:
			return 1.30
		4:
			return 1.15
		3:
			return 1.0
		2:
			return 0.8
		1:
			return 0.6
		_:
			return 1.0


func apply_condition_change(change_type: String, _reason: String = "") -> void:
	"""Apply condition changes through the new system"""
	var condition_system = get_node_or_null("/root/ConditionSystem")
	if not condition_system:
		return

	match change_type:
		"rest":
			condition_system.apply_rest_recovery()
		"light_training":
			condition_system.apply_light_training()
		"team_meal":
			condition_system.apply_team_meal()
		"victory":
			condition_system.apply_victory_bonus()
		"intense_training":
			condition_system.apply_intense_training()
		"consecutive_training":
			condition_system.apply_consecutive_training()
		"defeat":
			condition_system.apply_defeat_penalty()
		"injury":
			condition_system.apply_injury_penalty()
		"match_fatigue":
			condition_system.apply_match_fatigue()


func get_condition_status() -> Dictionary:
	"""Get detailed condition status"""
	var condition_system = get_node_or_null("/root/ConditionSystem")
	if condition_system:
		return {
			"level": condition_system.get_condition_level(),
			"percentage": condition_system.get_condition_percentage(),
			"name": condition_system.get_condition_name(),
			"color": condition_system.get_condition_color(),
			"description": condition_system.get_condition_description(),
			"ability_modifier": condition_system.get_ability_modifier(),
			"training_modifier": condition_system.get_training_modifier()
		}
	else:
		return {
			"level": condition,
			"percentage": 100.0 - fatigue,
			"name": "보통",
			"color": Color.WHITE,
			"description": "Legacy condition system",
			"ability_modifier": _legacy_condition_modifier(),
			"training_modifier": _legacy_training_modifier()
		}


func get_ca() -> int:
	"""Current Ability 계산 (0-200 스케일)"""
	return int(get_overall_rating() * 2.0)
