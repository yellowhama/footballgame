extends Node

# Training Level System
# Implements Power Pro style training level progression (1-9 levels)
# Each training type has its own level that affects XP gain and efficiency

# class_name removed - this is an autoload singleton

signal training_leveled_up(training_type: String, new_level: int)
signal training_mastered(training_type: String)  # Reached level 9

# Training level requirements and effects
const LEVEL_REQUIREMENTS = {
	1: {"uses": 0, "xp_bonus": 1.0, "description": "초보자"},
	2: {"uses": 5, "xp_bonus": 1.1, "description": "견습생"},
	3: {"uses": 15, "xp_bonus": 1.2, "description": "학습자"},
	4: {"uses": 30, "xp_bonus": 1.3, "description": "숙련자"},
	5: {"uses": 50, "xp_bonus": 1.4, "description": "전문가"},
	6: {"uses": 75, "xp_bonus": 1.5, "description": "달인"},
	7: {"uses": 105, "xp_bonus": 1.6, "description": "고수"},
	8: {"uses": 140, "xp_bonus": 1.7, "description": "대가"},
	9: {"uses": 180, "xp_bonus": 1.8, "description": "마스터"}
}

# Training usage tracking
var training_usage: Dictionary = {}  # training_type -> usage_count
var training_levels: Dictionary = {}  # training_type -> current_level
var training_mastery: Dictionary = {}  # training_type -> is_mastered


func _ready():
	# Initialize all training types
	_initialize_training_levels()


func _initialize_training_levels():
	"""Initialize all training types to level 1"""
	var all_trainings = [
		# Physical Training
		"Endurance",
		"Strength",
		"Speed",
		"Agility",
		"Balance",
		# Technical Training
		"Dribbling",
		"Passing",
		"Shooting",
		"Crossing",
		"FreeKicks",
		# Tactical Training
		"Positioning",
		"Tackling",
		"Heading",
		"Marking",
		"Intercepting",
		# Mental Training
		"Concentration",
		"DecisionMaking",
		"Leadership",
		# Match Preparation
		"MatchFitness",
		"Teamwork",
		"Formation"
	]

	for training_type in all_trainings:
		training_usage[training_type] = 0
		training_levels[training_type] = 1
		training_mastery[training_type] = false


func record_training_usage(training_type: String):
	"""Record that a training was used and check for level up"""
	if not training_usage.has(training_type):
		training_usage[training_type] = 0
		training_levels[training_type] = 1
		training_mastery[training_type] = false

	training_usage[training_type] += 1
	var current_usage = training_usage[training_type]
	var current_level = training_levels[training_type]

	# Check if we can level up
	var new_level = _calculate_level_from_usage(current_usage)
	if new_level > current_level:
		training_levels[training_type] = new_level
		training_leveled_up.emit(training_type, new_level)

		# Check for mastery (level 9)
		if new_level == 9 and not training_mastery[training_type]:
			training_mastery[training_type] = true
			training_mastered.emit(training_type)
			print("🎯 %s 훈련을 완전히 마스터했습니다!" % training_type)

		print("📈 %s 훈련 레벨 상승: %d → %d (사용 횟수: %d)" % [training_type, current_level, new_level, current_usage])


func _calculate_level_from_usage(usage_count: int) -> int:
	"""Calculate training level based on usage count"""
	for level in range(9, 0, -1):
		var requirements = LEVEL_REQUIREMENTS[level]
		if usage_count >= requirements.uses:
			return level
	return 1


func get_training_level(training_type: String) -> int:
	"""Get current training level for a training type"""
	return training_levels.get(training_type, 1)


func get_training_usage(training_type: String) -> int:
	"""Get usage count for a training type"""
	return training_usage.get(training_type, 0)


func get_level_progress(training_type: String) -> Dictionary:
	"""Get progress towards next level"""
	var current_level = get_training_level(training_type)
	var current_usage = get_training_usage(training_type)
	var current_requirements = LEVEL_REQUIREMENTS[current_level]

	var next_level = current_level + 1
	var progress_data = {
		"current_level": current_level,
		"current_usage": current_usage,
		"current_requirements": current_requirements,
		"is_max_level": current_level >= 9,
		"progress_percentage": 0.0
	}

	if current_level < 9:
		var next_requirements = LEVEL_REQUIREMENTS[next_level]
		var progress = float(current_usage - current_requirements.uses)
		var total_needed = next_requirements.uses - current_requirements.uses
		progress_data.progress_percentage = (progress / total_needed) * 100.0
		progress_data.next_level_requirements = next_requirements
	else:
		progress_data.progress_percentage = 100.0

	return progress_data


func get_xp_bonus(training_type: String) -> float:
	"""Get XP bonus multiplier for training level"""
	var level = get_training_level(training_type)
	var requirements = LEVEL_REQUIREMENTS.get(level, LEVEL_REQUIREMENTS[1])
	return requirements.xp_bonus


func get_level_description(training_type: String) -> String:
	"""Get level description for training type"""
	var level = get_training_level(training_type)
	var requirements = LEVEL_REQUIREMENTS.get(level, LEVEL_REQUIREMENTS[1])
	return requirements.description


func is_training_mastered(training_type: String) -> bool:
	"""Check if training is mastered (level 9)"""
	return training_mastery.get(training_type, false)


func get_mastered_trainings() -> Array:
	"""Get list of all mastered training types"""
	var mastered: Array = []
	for training_type in training_mastery:
		if training_mastery[training_type]:
			mastered.append(training_type)
	return mastered


func get_all_training_info() -> Dictionary:
	"""Get comprehensive training level information"""
	var info = {}
	for training_type in training_levels:
		info[training_type] = {
			"level": get_training_level(training_type),
			"usage": get_training_usage(training_type),
			"xp_bonus": get_xp_bonus(training_type),
			"description": get_level_description(training_type),
			"is_mastered": is_training_mastered(training_type),
			"progress": get_level_progress(training_type)
		}
	return info


func get_training_efficiency_summary() -> Dictionary:
	"""Get summary of training efficiency across all types"""
	var total_trainings = training_levels.size()
	var mastered_count = get_mastered_trainings().size()
	var average_level = 0.0
	var total_xp_bonus = 0.0

	for training_type in training_levels:
		var level = get_training_level(training_type)
		var xp_bonus = get_xp_bonus(training_type)
		average_level += level
		total_xp_bonus += xp_bonus

	average_level /= total_trainings
	total_xp_bonus /= total_trainings

	return {
		"total_trainings": total_trainings,
		"mastered_count": mastered_count,
		"mastery_percentage": (float(mastered_count) / total_trainings) * 100.0,
		"average_level": average_level,
		"average_xp_bonus": total_xp_bonus,
		"total_usage": _get_total_usage()
	}


func _get_total_usage() -> int:
	"""Get total training usage across all types"""
	var total = 0
	for usage in training_usage.values():
		total += usage
	return total


# Save/Load functions
func save_training_levels() -> Dictionary:
	"""Save training level data"""
	return {"training_usage": training_usage, "training_levels": training_levels, "training_mastery": training_mastery}


func load_training_levels(data: Dictionary):
	"""Load training level data"""
	if data.has("training_usage"):
		training_usage = data.training_usage
	if data.has("training_levels"):
		training_levels = data.training_levels
	if data.has("training_mastery"):
		training_mastery = data.training_mastery
