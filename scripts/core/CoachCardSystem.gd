extends Node

## Phase 2: CoachCard System Singleton
## Uma Musume-style 6-card deck management with trust and rainbow training
## Based on UNIFIED_ACADEMY_SPECIFICATION_V3.md Section 3

## Preload CoachCard class
const CoachCardClass = preload("res://scripts/model/CoachCard.gd")

## Signals

signal trust_changed(coachClass, old_trust: int, new_trust: int)
signal rainbow_training_triggered(coachClass, category: int)
signal deck_changed
signal coach_added_to_deck(coachClass)
signal coach_removed_from_deck(coachClass)
## Phase 6.1: Coach Merge System signals
signal coach_merged(from_cards: Array, result_card: Resource)
signal coach_upgraded(card: Resource, new_rarity: int)
signal inventory_changed

## Constants

const MAX_DECK_SIZE: int = 6
const WEEKLY_TRUST_DECAY: int = 2  # Trust decreases by 2 per week
const TRUST_GAIN_SAME_CATEGORY: int = 5  # Trust gain when training matching category
const TRUST_GAIN_DIFFERENT_CATEGORY: int = 2  # Trust gain when training different category
const RAINBOW_TRAINING_BONUS: float = 0.50  # 50% bonus when rainbow training triggers

## Phase 6.1: Coach Merge Constants
## 합성 규칙: 동일 카드 N장 → 상위 등급 (실패 없음)
const MERGE_REQUIREMENTS: Dictionary = {
	CoachCardClass.Rarity.COMMON: {"count": 3, "result": CoachCardClass.Rarity.UNCOMMON},
	CoachCardClass.Rarity.UNCOMMON: {"count": 3, "result": CoachCardClass.Rarity.RARE},
	CoachCardClass.Rarity.RARE: {"count": 2, "result": CoachCardClass.Rarity.EPIC},
	CoachCardClass.Rarity.EPIC: {"count": 2, "result": CoachCardClass.Rarity.LEGENDARY},
	CoachCardClass.Rarity.LEGENDARY: {"count": 0, "result": -1}  # 최고 등급
}

## 등급별 보너스
const RARITY_BONUSES: Dictionary = {
	CoachCardClass.Rarity.COMMON: {"training_bonus": 1.0, "special_rate": 0.0},
	CoachCardClass.Rarity.UNCOMMON: {"training_bonus": 1.1, "special_rate": 0.05},
	CoachCardClass.Rarity.RARE: {"training_bonus": 1.2, "special_rate": 0.10},
	CoachCardClass.Rarity.EPIC: {"training_bonus": 1.3, "special_rate": 0.20},
	CoachCardClass.Rarity.LEGENDARY: {"training_bonus": 1.5, "special_rate": 0.40}
}

## State

var active_deck: Array = []
var available_coaches: Array = []  # 획득 가능한 코치 타입 (템플릿)
var owned_coaches: Array = []  # Phase 6.1: 실제 보유한 코치 카드 인스턴스
var rainbow_triggered_this_week: bool = false

## Initialization


func _ready() -> void:
	_initialize_coaches()
	print("[CoachCardSystem] ✅ Initialized with %d coaches" % available_coaches.size())


func _initialize_coaches() -> void:
	"""Load coach cards from GameCache (data warehouse pipeline)."""
	available_coaches.clear()

	# Get GameCache singleton
	if not has_node("/root/GameCache"):
		push_error("[CoachCardSystem] ❌ GameCache not found! Using fallback data")
		_initialize_coaches_fallback()
		return

	var game_cache = get_node("/root/GameCache")

	# Check if cache is loaded
	if not game_cache.is_loaded:
		push_warning("[CoachCardSystem] ⚠️ GameCache not loaded yet, using fallback data")
		_initialize_coaches_fallback()
		return

	# Load coaches from cache
	var coach_data = game_cache.coach_cards
	if coach_data.is_empty():
		push_warning("[CoachCardSystem] ⚠️ No coach data in cache, using fallback")
		_initialize_coaches_fallback()
		return

	# Create coach cards from cached data
	for coach_id in coach_data.keys():
		var data = coach_data[coach_id]
		available_coaches.append(_create_coach_from_cache(coach_id, data))

	print("[CoachCardSystem] ✅ Loaded %d coaches from GameCache" % available_coaches.size())


func _create_coach_from_cache(_coach_id: String, data: Dictionary) -> Resource:
	"""Create CoachCard from cached data.

	@param _coach_id: Coach unique identifier (reserved for future coach lookup/tracking)
	"""
	var coach = CoachCardClass.new()
	coach.coach_name = data.get("name", "Unknown Coach")
	coach.rarity = data.get("rarity", CoachCardClass.Rarity.COMMON)
	coach.category = data.get("category", CoachCardClass.Category.TECHNICAL)
	coach.training_effect_bonus = data.get("training_effect_bonus", 0.10)
	coach.specialty_ability = data.get("specialty_ability", "")
	coach.specialty_ability_type = data.get("specialty_ability_type", "")
	return coach


func _initialize_coaches_fallback() -> void:
	"""Fallback: Create hardcoded coach cards if cache unavailable."""
	# 1. Technical Coaches
	available_coaches.append(
		_create_coach(
			"박지성",
			CoachCardClass.Rarity.LEGENDARY,
			CoachCardClass.Category.TECHNICAL,
			0.40,
			"창의적인 드리블과 패스 능력을 극대화",
			"DribblingMaster"
		)
	)

	# 2. Physical Coaches
	available_coaches.append(
		_create_coach(
			"차범근",
			CoachCardClass.Rarity.LEGENDARY,
			CoachCardClass.Category.PHYSICAL,
			0.40,
			"폭발적인 스피드와 파워 훈련",
			"SpeedDemon"
		)
	)

	# 3. Mental Coaches
	available_coaches.append(
		_create_coach(
			"히딩크",
			CoachCardClass.Rarity.LEGENDARY,
			CoachCardClass.Category.MENTAL,
			0.40,
			"절대 포기하지 않는 정신력 각인",
			"ClutchPlayer"
		)
	)

	# 4. Tactical Coaches
	available_coaches.append(
		_create_coach(
			"김호곤",
			CoachCardClass.Rarity.EPIC,
			CoachCardClass.Category.TACTICAL,
			0.30,
			"맞춤형 전술 이해와 포지셔닝",
			"TacticalGenius"
		)
	)

	print("[CoachCardSystem] Created %d fallback coach cards" % available_coaches.size())


func _create_coach(
	coach_name: String, rarity: int, category: int, bonus: float, ability: String, ability_type: String = ""
) -> Resource:
	"""Helper to create coach card instances."""
	var coach = CoachCardClass.new()
	coach.coach_name = coach_name
	coach.rarity = rarity
	coach.category = category
	coach.training_effect_bonus = bonus
	coach.specialty_ability = ability
	coach.specialty_ability_type = ability_type  # Phase 3: SpecialAbilityType enum name
	return coach


## Deck Management


func add_to_deck(coach) -> bool:
	"""
	Add coach to active deck if space available.
	Returns true if successful, false if deck is full or coach already in deck.
	"""
	if active_deck.size() >= MAX_DECK_SIZE:
		push_warning("[CoachCardSystem] Deck is full (%d/%d)" % [active_deck.size(), MAX_DECK_SIZE])
		return false

	if coach in active_deck:
		push_warning("[CoachCardSystem] Coach %s already in deck" % coach.coach_name)
		return false

	active_deck.append(coach)
	deck_changed.emit()
	coach_added_to_deck.emit(coach)
	print("[CoachCardSystem] ✅ Added %s to deck (%d/%d)" % [coach.coach_name, active_deck.size(), MAX_DECK_SIZE])
	return true


func remove_from_deck(coach) -> bool:
	"""
	Remove coach from active deck.
	Returns true if successful, false if coach not in deck.
	"""
	var index = active_deck.find(coach)
	if index == -1:
		push_warning("[CoachCardSystem] Coach %s not in deck" % coach.coach_name)
		return false

	active_deck.remove_at(index)
	deck_changed.emit()
	coach_removed_from_deck.emit(coach)
	print("[CoachCardSystem] Removed %s from deck (%d/%d)" % [coach.coach_name, active_deck.size(), MAX_DECK_SIZE])
	return true


func clear_deck() -> void:
	"""Remove all coaches from deck."""
	var count = active_deck.size()
	active_deck.clear()
	deck_changed.emit()
	print("[CoachCardSystem] Cleared deck (%d coaches removed)" % count)


func get_deck_size() -> int:
	"""Get current deck size."""
	return active_deck.size()


func is_deck_full() -> bool:
	"""Check if deck is full."""
	return active_deck.size() >= MAX_DECK_SIZE


func get_deck_coaches() -> Array:
	"""Get copy of active deck."""
	return active_deck.duplicate()


func get_available_coaches() -> Array:
	"""Get all available coaches."""
	return available_coaches.duplicate()


func find_coach_by_name(coach_name: String) -> Variant:
	"""Find coach by name in available coaches."""
	for coach in available_coaches:
		if coach.coach_name == coach_name:
			return coach
	return null


## Trust System


func increase_trust(coach, training_category: int, trained_with_coach: bool = true) -> int:
	"""
	Increase trust for coach based on training.
	- Same category: +5 trust
	- Different category: +2 trust
	- Only if coach is in active deck and training together

	Returns actual trust gained.
	"""
	if not trained_with_coach:
		return 0

	if coach not in active_deck:
		push_warning("[CoachCardSystem] Coach %s not in deck, no trust gain" % coach.coach_name)
		return 0

	# Calculate trust gain based on category match
	var trust_gain: int = TRUST_GAIN_DIFFERENT_CATEGORY
	if coach.category == training_category:
		trust_gain = TRUST_GAIN_SAME_CATEGORY

	var old_trust: int = coach.current_trust
	var actual_gain = coach.increase_trust(trust_gain, "Training together")

	if actual_gain > 0:
		trust_changed.emit(coach, old_trust, coach.current_trust)
		print(
			(
				"[CoachCardSystem] 📈 %s trust: %d → %d (+%d)"
				% [coach.coach_name, old_trust, coach.current_trust, actual_gain]
			)
		)

	return actual_gain


func increase_trust_all(amount: int) -> void:
	"""Increase trust for all coaches in active deck by specified amount

	Called by MatchSimulationManager after each match to update coach trust
	based on match result:
	- Win: +5 trust
	- Draw: +1 trust
	- Loss: -3 trust

	Args:
		amount: Trust change amount (can be positive or negative)
	"""
	if active_deck.size() == 0:
		print("[CoachCardSystem] No coaches in deck, no trust changes")
		return

	var coaches_affected = 0

	for coach in active_deck:
		var old_trust: int = coach.current_trust
		var actual_change = 0

		if amount > 0:
			# Positive trust gain
			actual_change = coach.increase_trust(amount, "Match result")
		elif amount < 0:
			# Negative trust change (loss)
			actual_change = -coach.decrease_trust(abs(amount), "Match result")

		if actual_change != 0:
			trust_changed.emit(coach, old_trust, coach.current_trust)
			print(
				(
					"[CoachCardSystem] %s %s trust: %d → %d (%+d)"
					% [
						"📈" if actual_change > 0 else "📉",
						coach.coach_name,
						old_trust,
						coach.current_trust,
						actual_change
					]
				)
			)
			coaches_affected += 1

	print(
		(
			"[CoachCardSystem] 👔 Match trust update: %d/%d coaches affected (%+d)"
			% [coaches_affected, active_deck.size(), amount]
		)
	)


func decay_trust_weekly() -> void:
	"""
	Decay trust for all coaches in deck by WEEKLY_TRUST_DECAY.
	Called by GameManager at end of each week.
	"""
	for coach in active_deck:
		var old_trust: int = coach.current_trust
		var decay_amount = coach.decrease_trust(WEEKLY_TRUST_DECAY, "Weekly decay")

		if decay_amount > 0:
			trust_changed.emit(coach, old_trust, coach.current_trust)
			print(
				(
					"[CoachCardSystem] 📉 %s trust decay: %d → %d (-%d)"
					% [coach.coach_name, old_trust, coach.current_trust, decay_amount]
				)
			)

	# Reset rainbow training flag for new week
	rainbow_triggered_this_week = false


## Rainbow Training System


func check_rainbow_training(training_category: int, deck: Array = []) -> Variant:
	"""
	Check if rainbow training can trigger for given training category.

	Conditions:
	1. At least one coach in deck with matching category
	2. Coach trust >= 80
	3. Rainbow not already triggered this week

	Returns the coach that triggered rainbow, or null if no trigger.
	"""
	if rainbow_triggered_this_week:
		return null

	var target_deck = deck if deck.size() > 0 else active_deck

	# Check coaches in deck with matching category and high trust
	for coach in target_deck:
		if coach.category == training_category and coach.can_trigger_rainbow_training():
			rainbow_triggered_this_week = true
			rainbow_training_triggered.emit(coach, training_category)
			print(
				"[CoachCardSystem] 🌈 RAINBOW TRAINING! Coach: %s (Trust: %d)" % [coach.coach_name, coach.current_trust]
			)
			return coach

	return null


func can_trigger_rainbow_this_week() -> bool:
	"""Check if rainbow training is still available this week."""
	return not rainbow_triggered_this_week


## ========== Phase 3: Special Ability Teaching System ==========


func check_ability_teaching(coach, training_type: String, quality: float) -> Dictionary:
	"""
	Check if coach can teach their specialty ability to player.

	Conditions:
	1. Coach must have specialty_ability_type defined
	2. Coach trust must be >= 80 (same threshold as rainbow training)
	3. Training quality affects acquisition chance

	Args:
		coach: The coach who might teach ability
		training_type: "technical", "physical", "mental", or "tactical"
		quality: Training quality (0-10)

	Returns:
		Dictionary with:
			- success: bool (true if engine call succeeded)
			- acquired: bool (true if ability was acquired)
			- ability_type: String (SpecialAbilityType name)
			- tier: String (ability tier, usually "Bronze" for new acquisition)
			- message: String (acquisition message if acquired)
			- error: String (if success is false)
	"""
	# Check if coach has specialty defined
	if coach.specialty_ability_type.is_empty():
		return {"success": false, "acquired": false, "error": "Coach has no specialty ability type defined"}

	# Check trust level (must be >= 80, same as rainbow training)
	if not coach.can_trigger_rainbow_training():
		return {
			"success": false,
			"acquired": false,
			"error": "Coach trust level too low (%d/100, need 80+)" % coach.current_trust
		}

	# Call Rust engine to check acquisition
	# Note: coach specialty_ability_type is used as coach_specialty parameter
	var result = OpenFootballAPI.check_ability_acquisition(training_type, quality, coach.specialty_ability_type)

	# Add coach information to result
	if result.success and result.acquired:
		print(
			(
				"[CoachCardSystem] 🎓 ABILITY TAUGHT! %s taught %s (%s tier)"
				% [coach.coach_name, result.ability_type, result.tier]
			)
		)

	return result


## Training Bonus Calculation


func calculate_training_bonus(training_category: int, deck: Array = []) -> float:
	"""
	Calculate total training bonus from active deck.

	Process:
	1. Check for rainbow training trigger
	2. Sum effective bonuses from matching category coaches
	3. Add rainbow bonus if triggered

	Returns total bonus multiplier (e.g., 0.25 = 25% bonus).
	"""
	var target_deck = deck if deck.size() > 0 else active_deck
	if target_deck.size() == 0:
		return 0.0

	var total_bonus: float = 0.0

	# Check rainbow training first
	var rainbow_coach = check_rainbow_training(training_category, target_deck)
	var rainbow_active: bool = rainbow_coach != null

	# Sum bonuses from coaches matching training category
	for coach in target_deck:
		if coach.category == training_category:
			var effective_bonus = coach.get_effective_bonus()
			total_bonus += effective_bonus
			print(
				(
					"[CoachCardSystem]   %s: +%.1f%% (trust %d)"
					% [coach.coach_name, effective_bonus * 100, coach.current_trust]
				)
			)

	# Add rainbow bonus if triggered
	if rainbow_active:
		total_bonus += RAINBOW_TRAINING_BONUS
		print("[CoachCardSystem]   🌈 Rainbow bonus: +%.1f%%" % (RAINBOW_TRAINING_BONUS * 100))

	print("[CoachCardSystem] Total training bonus: %.1f%%" % (total_bonus * 100))
	return total_bonus


## Query Methods


func get_coaches_by_category(category: int) -> Array:
	"""Get all coaches of specific category from available coaches."""
	var result: Array = []
	for coach in available_coaches:
		if coach.category == category:
			result.append(coach)
	return result


func get_deck_coaches_by_category(category: int) -> Array:
	"""Get coaches of specific category currently in deck."""
	var result: Array = []
	for coach in active_deck:
		if coach.category == category:
			result.append(coach)
	return result


func get_highest_trust_coach() -> Variant:
	"""Get coach with highest trust in active deck."""
	if active_deck.size() == 0:
		return null

	var highest_coach = active_deck[0]
	for coach in active_deck:
		if coach.current_trust > highest_coach.current_trust:
			highest_coach = coach
	return highest_coach


func get_rainbow_ready_coaches() -> Array:
	"""Get all coaches in deck that can trigger rainbow training."""
	var result: Array = []
	for coach in active_deck:
		if coach.can_trigger_rainbow_training():
			result.append(coach)
	return result


## Data Serialization


func save_state() -> Dictionary:
	"""Serialize system state for saving."""
	var deck_data: Array = []
	for coach in active_deck:
		# Save reference by name (coaches are persistent)
		deck_data.append(coach.coach_name)

	return {"active_deck_names": deck_data, "rainbow_triggered_this_week": rainbow_triggered_this_week}


func load_state(data: Dictionary) -> void:
	"""Deserialize system state from save."""
	active_deck.clear()

	var deck_names: Array = data.get("active_deck_names", [])
	for coach_name in deck_names:
		var coach = find_coach_by_name(coach_name)
		if coach:
			active_deck.append(coach)

	rainbow_triggered_this_week = data.get("rainbow_triggered_this_week", false)
	deck_changed.emit()
	print("[CoachCardSystem] Loaded state: %d coaches in deck" % active_deck.size())


## Debug Methods


func debug_print_deck() -> void:
	"""Print current deck state."""
	print("\n" + "=".repeat(60))
	print("📋 COACH DECK (%d/%d)" % [active_deck.size(), MAX_DECK_SIZE])
	print("=".repeat(60))

	if active_deck.size() == 0:
		print("(Empty)")
	else:
		for i in range(active_deck.size()):
			var coach = active_deck[i]
			print("%d. %s" % [i + 1, coach.get_debug_info()])

	print("=".repeat(60) + "\n")


func debug_print_all_coaches() -> void:
	"""Print all available coaches."""
	print("\n" + "=".repeat(60))
	print("👥 ALL COACHES (%d total)" % available_coaches.size())
	print("=".repeat(60))

	for category in [
		CoachCardClass.Category.TECHNICAL,
		CoachCardClass.Category.PHYSICAL,
		CoachCardClass.Category.MENTAL,
		CoachCardClass.Category.TACTICAL
	]:
		var coaches = get_coaches_by_category(category)
		if coaches.size() > 0:
			var cat_name = coaches[0].get_category_name()
			print("\n%s 코치 (%d):" % [cat_name, coaches.size()])
			for coach in coaches:
				print("  - %s" % coach.get_debug_info())

	print("\n" + "=".repeat(60) + "\n")


## ========== Phase 6.1: Coach Merge/Upgrade System ==========


func add_coach_to_inventory(coach: Resource) -> void:
	"""보유 인벤토리에 코치 카드 추가."""
	owned_coaches.append(coach)
	inventory_changed.emit()
	print("[CoachCardSystem] ✅ Added %s (%s) to inventory" % [coach.coach_name, coach.get_rarity_name()])


func remove_coach_from_inventory(coach: Resource) -> bool:
	"""보유 인벤토리에서 코치 카드 제거."""
	var index = owned_coaches.find(coach)
	if index == -1:
		push_warning("[CoachCardSystem] Coach %s not in inventory" % coach.coach_name)
		return false

	# 덱에서도 제거
	if coach in active_deck:
		remove_from_deck(coach)

	owned_coaches.remove_at(index)
	inventory_changed.emit()
	return true


func get_owned_coaches() -> Array:
	"""보유한 모든 코치 카드 반환."""
	return owned_coaches.duplicate()


func get_owned_coaches_by_id(coach_id: String) -> Array:
	"""동일 ID(이름)의 보유 코치 카드 목록 반환."""
	var result: Array = []
	for coach in owned_coaches:
		if coach.coach_name == coach_id:
			result.append(coach)
	return result


func get_owned_coaches_by_rarity(rarity: int) -> Array:
	"""특정 등급의 보유 코치 카드 목록 반환."""
	var result: Array = []
	for coach in owned_coaches:
		if coach.rarity == rarity:
			result.append(coach)
	return result


func can_merge(coach_id: String) -> Dictionary:
	"""
	합성 가능 여부 확인.

	Returns:
		Dictionary with:
		- can_merge: bool
		- owned_count: int
		- required_count: int
		- result_rarity: int (-1 if already max)
		- current_rarity: int
	"""
	var owned = get_owned_coaches_by_id(coach_id)
	if owned.is_empty():
		return {
			"can_merge": false,
			"owned_count": 0,
			"required_count": 0,
			"result_rarity": -1,
			"current_rarity": -1,
			"error": "No coaches with this ID owned"
		}

	var current_rarity: int = owned[0].rarity

	if not MERGE_REQUIREMENTS.has(current_rarity):
		return {
			"can_merge": false,
			"owned_count": owned.size(),
			"required_count": 0,
			"result_rarity": -1,
			"current_rarity": current_rarity,
			"error": "Unknown rarity"
		}

	var requirement: Dictionary = MERGE_REQUIREMENTS[current_rarity]
	var required_count: int = requirement.count
	var result_rarity: int = requirement.result

	# 최고 등급은 합성 불가
	if required_count == 0 or result_rarity == -1:
		return {
			"can_merge": false,
			"owned_count": owned.size(),
			"required_count": 0,
			"result_rarity": -1,
			"current_rarity": current_rarity,
			"error": "Already at maximum rarity"
		}

	return {
		"can_merge": owned.size() >= required_count,
		"owned_count": owned.size(),
		"required_count": required_count,
		"result_rarity": result_rarity,
		"current_rarity": current_rarity
	}


func merge_coaches(coach_id: String) -> Resource:
	"""
	코치 카드 합성 실행.
	동일 코치 카드 N장을 소모하여 상위 등급 1장 생성.

	Returns:
		합성 결과 코치 카드, 실패 시 null
	"""
	var check = can_merge(coach_id)
	if not check.can_merge:
		push_warning("[CoachCardSystem] Cannot merge %s: %s" % [coach_id, check.get("error", "Requirements not met")])
		return null

	var cards_to_consume: Array = get_owned_coaches_by_id(coach_id).slice(0, check.required_count)
	var base_card: Resource = cards_to_consume[0]
	var result_card: Resource = _create_upgraded_card(base_card, check.result_rarity)

	# 소모된 카드 제거
	for card in cards_to_consume:
		remove_coach_from_inventory(card)

	# 결과 카드 추가
	add_coach_to_inventory(result_card)

	coach_merged.emit(cards_to_consume, result_card)
	print(
		(
			"[CoachCardSystem] 🔮 MERGED! %d x %s (%s) → %s (%s)"
			% [
				cards_to_consume.size(),
				base_card.coach_name,
				base_card.get_rarity_name(),
				result_card.coach_name,
				result_card.get_rarity_name()
			]
		)
	)

	return result_card


func _create_upgraded_card(base_card: Resource, new_rarity: int) -> Resource:
	"""기존 카드 기반으로 상위 등급 카드 생성."""
	var upgraded = CoachCardClass.new()
	upgraded.coach_name = base_card.coach_name
	upgraded.rarity = new_rarity
	upgraded.category = base_card.category
	upgraded.portrait = base_card.portrait
	upgraded.specialty_ability = base_card.specialty_ability
	upgraded.specialty_ability_type = base_card.specialty_ability_type

	# 등급별 보너스 적용
	var rarity_bonus: Dictionary = RARITY_BONUSES.get(new_rarity, {})
	var base_training_bonus: float = base_card.training_effect_bonus
	upgraded.training_effect_bonus = base_training_bonus * rarity_bonus.get("training_bonus", 1.0)

	# Trust 초기화 (합성 시 신뢰도 유지하지 않음)
	upgraded.current_trust = 0

	coach_upgraded.emit(upgraded, new_rarity)
	return upgraded


func get_rarity_bonus(rarity: int) -> Dictionary:
	"""등급별 보너스 정보 반환."""
	return RARITY_BONUSES.get(rarity, {"training_bonus": 1.0, "special_rate": 0.0})


func get_merge_info(coach_id: String) -> Dictionary:
	"""합성 정보 UI용 데이터 반환."""
	var check = can_merge(coach_id)
	if check.current_rarity == -1:
		return {"valid": false}

	var current_rarity_name: String = _get_rarity_name(check.current_rarity)
	var result_rarity_name: String = _get_rarity_name(check.result_rarity) if check.result_rarity != -1 else "MAX"
	var current_bonus: Dictionary = get_rarity_bonus(check.current_rarity)
	var result_bonus: Dictionary = get_rarity_bonus(check.result_rarity) if check.result_rarity != -1 else {}

	return {
		"valid": true,
		"can_merge": check.can_merge,
		"coach_id": coach_id,
		"owned_count": check.owned_count,
		"required_count": check.required_count,
		"current_rarity": current_rarity_name,
		"result_rarity": result_rarity_name,
		"current_training_bonus": current_bonus.get("training_bonus", 1.0),
		"result_training_bonus": result_bonus.get("training_bonus", 1.0),
		"current_special_rate": current_bonus.get("special_rate", 0.0),
		"result_special_rate": result_bonus.get("special_rate", 0.0)
	}


func _get_rarity_name(rarity: int) -> String:
	"""등급 숫자를 이름으로 변환."""
	match rarity:
		CoachCardClass.Rarity.COMMON:
			return "일반"
		CoachCardClass.Rarity.UNCOMMON:
			return "고급"
		CoachCardClass.Rarity.RARE:
			return "희귀"
		CoachCardClass.Rarity.EPIC:
			return "영웅"
		CoachCardClass.Rarity.LEGENDARY:
			return "전설"
		_:
			return "알 수 없음"


func get_mergeable_coaches() -> Array:
	"""현재 합성 가능한 코치 ID 목록 반환."""
	var mergeable: Array = []
	var checked_ids: Dictionary = {}

	for coach in owned_coaches:
		var coach_id: String = coach.coach_name
		if checked_ids.has(coach_id):
			continue
		checked_ids[coach_id] = true

		var check = can_merge(coach_id)
		if check.can_merge:
			mergeable.append(
				{
					"coach_id": coach_id,
					"owned_count": check.owned_count,
					"required_count": check.required_count,
					"current_rarity": check.current_rarity,
					"result_rarity": check.result_rarity
				}
			)

	return mergeable


## Phase 6.1: Inventory Save/Load Extension


func save_inventory_state() -> Dictionary:
	"""인벤토리 상태 저장용 직렬화."""
	var inventory_data: Array = []
	for coach in owned_coaches:
		inventory_data.append(coach.to_dict())

	return {"owned_coaches": inventory_data}


func load_inventory_state(data: Dictionary) -> void:
	"""인벤토리 상태 로드."""
	owned_coaches.clear()

	var inventory_data: Array = data.get("owned_coaches", [])
	for coach_data in inventory_data:
		var coach = CoachCardClass.new()
		coach.from_dict(coach_data)
		owned_coaches.append(coach)

	inventory_changed.emit()
	print("[CoachCardSystem] Loaded inventory: %d coaches" % owned_coaches.size())
