class_name CoachCard
extends Resource

## Phase 2: CoachCard Resource Definition
## Uma Musume-style coach card with trust system and rainbow training
## Based on UNIFIED_ACADEMY_SPECIFICATION_V3.md Section 3.2

## Enums

enum Rarity { COMMON, UNCOMMON, RARE, EPIC, LEGENDARY }  # 기본 (10% 보너스)  # 희귀 (15% 보너스)  # 레어 (20% 보너스)  # 에픽 (30% 보너스)  # 전설 (40% 보너스)

enum Category { TECHNICAL, PHYSICAL, MENTAL, TACTICAL }  # 기술 훈련  # 신체 훈련  # 정신 훈련  # 전술 훈련

## Core Properties

@export var coach_name: String = ""
@export var rarity: Rarity = Rarity.COMMON
@export var category: Category = Category.TECHNICAL
@export var portrait: Texture2D = null
@export var training_effect_bonus: float = 0.10  # Base bonus (10%-40% based on rarity)
@export var specialty_ability: String = ""  # Special coaching ability description
@export var specialty_ability_type: String = ""  # Phase 3: SpecialAbilityType enum name (e.g., "DribblingMaster")

## Trust System

var current_trust: int = 0  # Trust gauge: 0-100
var trust_gain_history: Array[Dictionary] = []  # Track trust gain events

## Constants

const MAX_TRUST: int = 100
const MIN_TRUST: int = 0
const RAINBOW_TRAINING_THRESHOLD: int = 80  # Trust level needed for rainbow training

## Core Methods


func get_effective_bonus() -> float:
	"""
	Calculate training bonus based on base bonus and current trust level.
	Formula: base_bonus * (1.0 + trust_percentage * 0.5)
	At 100 trust, bonus is 1.5x base (e.g., 10% → 15%)
	"""
	var base_bonus: float = training_effect_bonus
	var trust_multiplier: float = 1.0 + (float(current_trust) / 100.0) * 0.5
	return base_bonus * trust_multiplier


func can_trigger_rainbow_training() -> bool:
	"""
	Check if coach can trigger rainbow training (Uma Musume mechanic).
	Requires trust >= 80 and specific conditions during training.
	"""
	return current_trust >= RAINBOW_TRAINING_THRESHOLD


## Trust Management Methods


func increase_trust(amount: int, reason: String = "") -> int:
	"""
	Increase trust level, clamped to MAX_TRUST.
	Returns actual amount gained (may be less if capped).
	"""
	var old_trust: int = current_trust
	current_trust = clampi(current_trust + amount, MIN_TRUST, MAX_TRUST)
	var actual_gain: int = current_trust - old_trust

	# Record trust gain event
	if actual_gain > 0:
		trust_gain_history.append(
			{"amount": actual_gain, "reason": reason, "new_trust": current_trust, "timestamp": Time.get_ticks_msec()}
		)

	return actual_gain


func decrease_trust(amount: int, reason: String = "") -> int:
	"""
	Decrease trust level (weekly decay), clamped to MIN_TRUST.
	Returns actual amount lost.
	"""
	var old_trust: int = current_trust
	current_trust = clampi(current_trust - amount, MIN_TRUST, MAX_TRUST)
	var actual_loss: int = old_trust - current_trust

	# Record trust loss event
	if actual_loss > 0:
		trust_gain_history.append(
			{"amount": -actual_loss, "reason": reason, "new_trust": current_trust, "timestamp": Time.get_ticks_msec()}
		)

	return actual_loss


func set_trust(value: int) -> void:
	"""Directly set trust level (for testing or special events)."""
	current_trust = clampi(value, MIN_TRUST, MAX_TRUST)


func get_trust_percentage() -> float:
	"""Get trust as percentage (0.0 - 1.0)."""
	return float(current_trust) / float(MAX_TRUST)


## Query Methods


func get_rarity_name() -> String:
	"""Get human-readable rarity name."""
	match rarity:
		Rarity.COMMON:
			return "일반"
		Rarity.UNCOMMON:
			return "고급"
		Rarity.RARE:
			return "희귀"
		Rarity.EPIC:
			return "영웅"
		Rarity.LEGENDARY:
			return "전설"
		_:
			return "알 수 없음"


func get_category_name() -> String:
	"""Get human-readable category name."""
	match category:
		Category.TECHNICAL:
			return "기술"
		Category.PHYSICAL:
			return "신체"
		Category.MENTAL:
			return "정신"
		Category.TACTICAL:
			return "전술"
		_:
			return "알 수 없음"


func get_trust_status() -> String:
	"""Get descriptive trust status."""
	if current_trust >= 80:
		return "최고 신뢰"
	elif current_trust >= 60:
		return "높은 신뢰"
	elif current_trust >= 40:
		return "보통"
	elif current_trust >= 20:
		return "낮은 신뢰"
	else:
		return "신뢰 부족"


func is_rainbow_ready() -> bool:
	"""Alias for can_trigger_rainbow_training()."""
	return can_trigger_rainbow_training()


## Data Serialization


func to_dict() -> Dictionary:
	"""Serialize coach card data for saving."""
	return {
		"coach_name": coach_name,
		"rarity": rarity,
		"category": category,
		"training_effect_bonus": training_effect_bonus,
		"specialty_ability": specialty_ability,
		"current_trust": current_trust,
		"trust_history_size": trust_gain_history.size()
	}


func from_dict(data: Dictionary) -> void:
	"""Deserialize coach card data from save."""
	coach_name = data.get("coach_name", "")
	rarity = data.get("rarity", Rarity.COMMON)
	category = data.get("category", Category.TECHNICAL)
	training_effect_bonus = data.get("training_effect_bonus", 0.10)
	specialty_ability = data.get("specialty_ability", "")
	current_trust = data.get("current_trust", 0)
	# Note: trust_history not restored from save (would be too large)


## Debug Methods


func get_debug_info() -> String:
	"""Get formatted debug information."""
	return (
		"[CoachCard] %s (%s/%s) | Trust: %d/100 (%s) | Bonus: %.1f%% (Effective: %.1f%%)"
		% [
			coach_name,
			get_rarity_name(),
			get_category_name(),
			current_trust,
			get_trust_status(),
			training_effect_bonus * 100,
			get_effective_bonus() * 100
		]
	)


func _to_string() -> String:
	"""String representation for print()."""
	return "%s [%s]" % [coach_name, get_rarity_name()]
