extends Node

## CoachAffinitySystem.gd
##
## 코치 호감도 특훈 시스템 (コーチ好感度特訓) - Phase 2 of Gameplay Upgrade Plan
##
## Manages coach affinity (separate from trust) and special training triggers.
## Based on パワフルプロ野球 Success Mode affinity mechanics.
##
## Core Mechanics:
## - Track affinity (0-100) per coach through training interaction
## - Special training (特訓) triggers at 80+ affinity with 30% chance
## - Special training provides 3x training effect multiplier
## - Affinity increases based on training quality and coach specialty match

# ============================================================
# SIGNALS
# ============================================================

## Emitted when affinity with a coach changes
## @param coach_name: Name of the coach
## @param new_value: New affinity score (0-100)
## @param delta: Change amount
signal affinity_changed(coach_name: String, new_value: int, delta: int)

## Emitted when special training is triggered
## @param coach_name: Coach who triggered the special training
## @param training_category: Training category that will receive bonus
signal special_training_triggered(coach_name: String, training_category: String)

## Emitted when special training effect is applied
## @param multiplier: Training effect multiplier (typically 3.0)
signal special_training_applied(multiplier: float)

# ============================================================
# CONSTANTS
# ============================================================

## Affinity thresholds
const MIN_AFFINITY: int = 0
const MAX_AFFINITY: int = 100
const SPECIAL_TRAINING_THRESHOLD: int = 80

## Special training mechanics
const SPECIAL_TRAINING_CHANCE: float = 0.3  # 30% chance
const SPECIAL_TRAINING_MULTIPLIER: float = 3.0  # 3x effect

## Affinity gain rates
const BASE_AFFINITY_GAIN: int = 5  # Base gain for any training together
const SPECIALTY_MATCH_BONUS: int = 3  # Extra gain when coach specialty matches training
const HIGH_QUALITY_BONUS: int = 2  # Extra gain for high-quality training (>80% success)

# ============================================================
# STATE VARIABLES
# ============================================================

## Coach affinity dictionary: coach_name -> affinity_value (0-100)
var coach_affinities: Dictionary = {}

## Special training state
var special_training_active: bool = false
var special_training_coach: String = ""
var special_training_multiplier: float = 1.0

# ============================================================
# LIFECYCLE
# ============================================================


func _ready() -> void:
	# Connect to TrainingManager signal
	if TrainingManager:
		TrainingManager.training_completed.connect(_on_training_completed)
		print("[CoachAffinitySystem] Connected to TrainingManager.training_completed")
	else:
		push_error("[CoachAffinitySystem] TrainingManager not found!")

	print("[CoachAffinitySystem] Initialized")


# ============================================================
# PUBLIC API - AFFINITY MANAGEMENT
# ============================================================


## Get affinity value for a specific coach
## @param coach_name: Name of the coach
## @returns: Affinity value (0-100), defaults to 0 if coach not tracked
func get_affinity(coach_name: String) -> int:
	if coach_affinities.has(coach_name):
		return coach_affinities[coach_name]
	return 0


## Set affinity value directly (for initialization or special events)
## @param coach_name: Name of the coach
## @param value: New affinity value (will be clamped 0-100)
func set_affinity(coach_name: String, value: int) -> void:
	var old_value = get_affinity(coach_name)
	var new_value = clampi(value, MIN_AFFINITY, MAX_AFFINITY)

	if old_value != new_value:
		coach_affinities[coach_name] = new_value
		var delta = new_value - old_value
		affinity_changed.emit(coach_name, new_value, delta)

		print("[CoachAffinitySystem] %s affinity: %d → %d (%+d)" % [coach_name, old_value, new_value, delta])


## Increase affinity with a coach
## @param coach_name: Name of the coach
## @param amount: Amount to increase
func increase_affinity(coach_name: String, amount: int) -> void:
	var current = get_affinity(coach_name)
	set_affinity(coach_name, current + amount)


## Get all coaches with their affinity values
## @returns: Dictionary of coach_name -> affinity_value
func get_all_affinities() -> Dictionary:
	return coach_affinities.duplicate()


## Get coaches above affinity threshold
## @param threshold: Minimum affinity value
## @returns: Array of coach names
func get_coaches_above_threshold(threshold: int) -> Array[String]:
	var result: Array[String] = []
	for coach_name in coach_affinities:
		if coach_affinities[coach_name] >= threshold:
			result.append(coach_name)
	return result


# ============================================================
# PUBLIC API - SPECIAL TRAINING
# ============================================================


## Check if special training is currently active
func is_special_training_active() -> bool:
	return special_training_active


## Get current training multiplier (includes special training if active)
func get_training_multiplier() -> float:
	return special_training_multiplier if special_training_active else 1.0


## Manually trigger special training (for events or debugging)
## @param coach_name: Coach triggering the special training
func trigger_special_training(coach_name: String) -> void:
	special_training_active = true
	special_training_coach = coach_name
	special_training_multiplier = SPECIAL_TRAINING_MULTIPLIER

	print(
		(
			"[CoachAffinitySystem] 特訓発動! %s triggers special training (%.1fx multiplier)"
			% [coach_name, SPECIAL_TRAINING_MULTIPLIER]
		)
	)

	var training_category = _get_coach_specialty(coach_name)
	special_training_triggered.emit(coach_name, training_category)


## Clear special training state (called after training is applied)
func clear_special_training() -> void:
	if special_training_active:
		print("[CoachAffinitySystem] Special training effect cleared")
		special_training_active = false
		special_training_coach = ""
		special_training_multiplier = 1.0


# ============================================================
# TRAINING INTEGRATION
# ============================================================


## Called by TrainingManager signal when training completes
## @param training_id: ID of completed training
## @param results: Training results dictionary
func _on_training_completed(event: Dictionary) -> void:
	# Get active coaches from deck
	if not CoachCardSystem:
		return
	var training_id: String = event.get("training_id", "")
	var results: Dictionary = event.get("result", {})

	var active_coaches: Array = []
	if CoachCardSystem and CoachCardSystem.has_method("get_deck_coaches"):
		active_coaches = CoachCardSystem.call("get_deck_coaches")
	else:
		return
	if active_coaches.is_empty():
		return

	# Calculate training quality for affinity gain
	var training_quality = _calculate_training_quality(results)
	var training_category = _get_training_category(training_id)

	# Update affinity for each active coach
	for coach_card in active_coaches:
		var coach_name = coach_card.coach_name
		var affinity_gain = _calculate_affinity_gain(coach_card, training_category, training_quality)

		if affinity_gain > 0:
			increase_affinity(coach_name, affinity_gain)

			# Check for special training trigger
			_check_special_training_trigger(coach_name, training_category)

	# Apply special training multiplier if active
	if special_training_active:
		special_training_applied.emit(special_training_multiplier)
		# Note: TrainingManager should query get_training_multiplier() before applying results


## Calculate training quality score from results
## @param results: Training results dictionary
## @returns: Quality score (0.0 - 1.0)
func _calculate_training_quality(results: Dictionary) -> float:
	# Check if training was successful
	if not (results.get("success") if results.has("success") else false):
		return 0.0

	# Base quality on attribute changes
	var changes = results.get("changes") if results.has("changes") else {}
	var total_change = 0

	for attr in changes:
		if attr != "condition":  # Don't count condition in quality
			total_change += abs(changes[attr])

	# Normalize to 0.0-1.0 range (assume max realistic gain is 15 points total)
	var quality = clampf(float(total_change) / 15.0, 0.0, 1.0)

	return quality


## Calculate affinity gain for a coach based on training
## @param coach_card: Coach card data
## @param training_category: Training category (TECHNICAL, PHYSICAL, etc.)
## @param quality: Training quality (0.0-1.0)
## @returns: Affinity gain amount
func _calculate_affinity_gain(coach_card, training_category: String, quality: float) -> int:
	if quality <= 0.0:
		return 0

	var gain = BASE_AFFINITY_GAIN

	# Specialty match bonus
	var coach_specialty = coach_card.get("specialty") if coach_card.has("specialty") else ""
	if coach_specialty == training_category:
		gain += SPECIALTY_MATCH_BONUS

	# High quality bonus (>80% quality)
	if quality >= 0.8:
		gain += HIGH_QUALITY_BONUS

	return gain


## Check if special training should trigger for a coach
## @param coach_name: Coach to check
## @param training_category: Current training category
func _check_special_training_trigger(coach_name: String, _training_category: String) -> void:
	var affinity = get_affinity(coach_name)

	# Must be above threshold and not already active
	if affinity < SPECIAL_TRAINING_THRESHOLD or special_training_active:
		return

	# Random chance check
	if randf() < SPECIAL_TRAINING_CHANCE:
		trigger_special_training(coach_name)


# ============================================================
# HELPER FUNCTIONS
# ============================================================


## Get training category from training ID
## @param training_id: Training program ID
## @returns: Training category string
func _get_training_category(training_id: String) -> String:
	if not TrainingManager:
		return ""

	# Get training data from TrainingManager
	var training_data = (
		TrainingManager.training_programs.get(training_id) if TrainingManager.training_programs.has(training_id) else {}
	)
	return training_data.get("type") if training_data.has("type") else ""


## Get coach specialty category
## @param coach_name: Coach name
## @returns: Specialty category string
func _get_coach_specialty(coach_name: String) -> String:
	if not CoachCardSystem:
		return ""

	var active_coaches: Array = []
	if CoachCardSystem and CoachCardSystem.has_method("get_deck_coaches"):
		active_coaches = CoachCardSystem.call("get_deck_coaches")
	else:
		return ""
	for coach_card in active_coaches:
		if coach_card.coach_name == coach_name:
			if coach_card.has("specialty"):
				return coach_card.get("specialty")
			return ""

	return ""


# ============================================================
# UI HELPER FUNCTIONS
# ============================================================


## Get affinity display string with status
## Example: "コーチ田中: 85/100 (特訓可能!)"
func get_affinity_display(coach_name: String) -> String:
	var affinity = get_affinity(coach_name)
	var status = ""

	if affinity >= SPECIAL_TRAINING_THRESHOLD:
		status = " (特訓可能!)"
	elif affinity >= SPECIAL_TRAINING_THRESHOLD - 10:
		status = " (もうすぐ!)"

	return "%s: %d/100%s" % [coach_name, affinity, status]


## Get color for affinity bar display
func get_affinity_color(affinity: int) -> Color:
	if affinity >= SPECIAL_TRAINING_THRESHOLD:
		return Color(1.0, 0.84, 0.0)  # Gold - special training available
	elif affinity >= 60:
		return Color(0.2, 0.8, 0.2)  # Green - high affinity
	elif affinity >= 30:
		return Color(0.8, 0.6, 0.2)  # Yellow - medium affinity
	else:
		return Color(0.8, 0.2, 0.2)  # Red - low affinity


## Get affinity rank name
func get_affinity_rank(affinity: int) -> String:
	if affinity >= 90:
		return "親友"  # Best friend
	elif affinity >= SPECIAL_TRAINING_THRESHOLD:
		return "信頼"  # Trust
	elif affinity >= 60:
		return "友好"  # Friendly
	elif affinity >= 30:
		return "普通"  # Normal
	else:
		return "知人"  # Acquaintance


# ============================================================
# DEBUG FUNCTIONS
# ============================================================


## Set affinity for debugging
func debug_set_affinity(coach_name: String, value: int) -> void:
	if OS.is_debug_build():
		set_affinity(coach_name, value)
		print("[CoachAffinitySystem] DEBUG: Set %s affinity to %d" % [coach_name, value])


## Print all affinity states
func debug_print_affinities() -> void:
	if OS.is_debug_build():
		print("=== CoachAffinitySystem State ===")
		for coach_name in coach_affinities:
			var affinity = coach_affinities[coach_name]
			print("%s: %d/100 (%s)" % [coach_name, affinity, get_affinity_rank(affinity)])
		print("Special training active: %s" % special_training_active)
		if special_training_active:
			print("Special training coach: %s (%.1fx)" % [special_training_coach, special_training_multiplier])
		print("================================")
