extends Node
## TraitAcquisitionSystem - Trait 획득 로직
## 훈련/경기 결과에 따른 trait drop 시스템

signal trait_dropped(player_id: String, trait_type: String, tier: int, source: String)

## Drop rates by source
const BASE_DROP_RATES = {
	"training": 0.08,  # 8% base chance per training
	"match_win": 0.12,  # 12% on win
	"match_draw": 0.06,  # 6% on draw
	"match_loss": 0.03,  # 3% on loss
	"match_goal": 0.05,  # 5% per goal scored
	"match_assist": 0.04,  # 4% per assist
	"match_motm": 0.15,  # 15% if Man of the Match
}

## Tier drop weights (higher = more common)
const TIER_WEIGHTS = {
	TraitManager.TraitTier.BRONZE: 70,  # 70%
	TraitManager.TraitTier.SILVER: 25,  # 25%
	TraitManager.TraitTier.GOLD: 5,  # 5%
}

## Training type → Trait category mapping
const TRAINING_TRAIT_MAP = {
	"shooting": TraitManager.TraitCategory.SHOOTING,
	"shooting_precision": TraitManager.TraitCategory.SHOOTING,
	"passing": TraitManager.TraitCategory.PASSING,
	"passing_accuracy": TraitManager.TraitCategory.PASSING,
	"dribbling": TraitManager.TraitCategory.DRIBBLING,
	"dribbling_control": TraitManager.TraitCategory.DRIBBLING,
	"defending": TraitManager.TraitCategory.DEFENSE,
	"physical": TraitManager.TraitCategory.DEFENSE,
	"tactical": TraitManager.TraitCategory.PASSING,  # Vision/positioning → passing traits
}

## Position → Preferred trait categories
const POSITION_TRAIT_AFFINITY = {
	"GK": [TraitManager.TraitCategory.GOALKEEPER],
	"CB": [TraitManager.TraitCategory.DEFENSE],
	"LB": [TraitManager.TraitCategory.DEFENSE, TraitManager.TraitCategory.DRIBBLING],
	"RB": [TraitManager.TraitCategory.DEFENSE, TraitManager.TraitCategory.DRIBBLING],
	"CDM": [TraitManager.TraitCategory.DEFENSE, TraitManager.TraitCategory.PASSING],
	"CM": [TraitManager.TraitCategory.PASSING, TraitManager.TraitCategory.DRIBBLING],
	"CAM": [TraitManager.TraitCategory.PASSING, TraitManager.TraitCategory.SHOOTING],
	"LM": [TraitManager.TraitCategory.DRIBBLING, TraitManager.TraitCategory.PASSING],
	"RM": [TraitManager.TraitCategory.DRIBBLING, TraitManager.TraitCategory.PASSING],
	"LW": [TraitManager.TraitCategory.DRIBBLING, TraitManager.TraitCategory.SHOOTING],
	"RW": [TraitManager.TraitCategory.DRIBBLING, TraitManager.TraitCategory.SHOOTING],
	"CF": [TraitManager.TraitCategory.SHOOTING, TraitManager.TraitCategory.DRIBBLING],
	"ST": [TraitManager.TraitCategory.SHOOTING],
}


func _ready():
	print("[TraitAcquisitionSystem] Initialized")
	_connect_signals()


func _connect_signals():
	# Connect to TrainingManager
	if has_node("/root/TrainingManager"):
		var tm = get_node("/root/TrainingManager")
		if tm.has_signal("training_completed"):
			tm.training_completed.connect(_on_training_completed)
			print("[TraitAcquisitionSystem] Connected to TrainingManager")

	# Connect to MatchManager
	if has_node("/root/MatchManager"):
		var mm = get_node("/root/MatchManager")
		if mm.has_signal("match_ended"):
			mm.match_ended.connect(_on_match_ended)
			print("[TraitAcquisitionSystem] Connected to MatchManager")


# ============================================================================
# Training-based Acquisition
# ============================================================================


func _on_training_completed(event: Dictionary):
	var player_id = event.get("player_id", "player_0")
	var training_id = event.get("training_id", "")
	var quality = event.get("quality", 1.0)  # Training quality 0.0-2.0

	# Calculate drop chance
	var base_chance = BASE_DROP_RATES["training"]
	var modified_chance = base_chance * quality  # Higher quality = better chance

	# Roll for drop
	if randf() < modified_chance:
		var category = _get_training_category(training_id)
		var trait_type = _pick_random_trait_from_category(category)
		var tier = _roll_tier()

		# Add to inventory
		if TraitManager.add_trait_to_inventory(player_id, trait_type, tier):
			trait_dropped.emit(player_id, trait_type, tier, "training")
			_show_drop_notification(trait_type, tier, "훈련")
			print(
				(
					"[TraitAcquisitionSystem] Training drop: %s (%s) for %s"
					% [trait_type, TraitManager.TIER_NAMES_KO[tier], player_id]
				)
			)


func _get_training_category(training_id: String) -> int:
	if TRAINING_TRAIT_MAP.has(training_id):
		return TRAINING_TRAIT_MAP[training_id]
	# Default to random category (excluding GK)
	var categories = [
		TraitManager.TraitCategory.SHOOTING,
		TraitManager.TraitCategory.PASSING,
		TraitManager.TraitCategory.DRIBBLING,
		TraitManager.TraitCategory.DEFENSE
	]
	return categories[randi() % categories.size()]


# ============================================================================
# Match-based Acquisition
# ============================================================================


func _on_match_ended(result: Dictionary):
	var player_id = "player_0"  # Current player
	var player_stats = result.get("player_stats", {})
	var match_result = result.get("result", "draw")  # "win", "draw", "loss"

	# Calculate total drop chance from match
	var total_chance = 0.0

	# Base chance from result
	match match_result:
		"win":
			total_chance += BASE_DROP_RATES["match_win"]
		"draw":
			total_chance += BASE_DROP_RATES["match_draw"]
		"loss":
			total_chance += BASE_DROP_RATES["match_loss"]

	# Bonus from performance
	var goals = player_stats.get("goals", 0)
	var assists = player_stats.get("assists", 0)
	var is_motm = player_stats.get("man_of_the_match", false)

	total_chance += goals * BASE_DROP_RATES["match_goal"]
	total_chance += assists * BASE_DROP_RATES["match_assist"]
	if is_motm:
		total_chance += BASE_DROP_RATES["match_motm"]

	# Cap at 50%
	total_chance = min(total_chance, 0.5)

	# Roll for drop
	if randf() < total_chance:
		var position = player_stats.get("position", "CM")
		var category = _get_position_category(position)
		var trait_type = _pick_random_trait_from_category(category)
		var tier = _roll_tier_with_performance_bonus(goals, assists, is_motm)

		# Add to inventory
		if TraitManager.add_trait_to_inventory(player_id, trait_type, tier):
			trait_dropped.emit(player_id, trait_type, tier, "match")
			_show_drop_notification(trait_type, tier, "경기")
			print(
				(
					"[TraitAcquisitionSystem] Match drop: %s (%s) for %s"
					% [trait_type, TraitManager.TIER_NAMES_KO[tier], player_id]
				)
			)


func _get_position_category(position: String) -> int:
	if POSITION_TRAIT_AFFINITY.has(position):
		var categories = POSITION_TRAIT_AFFINITY[position]
		return categories[randi() % categories.size()]
	# Default
	return TraitManager.TraitCategory.DRIBBLING


# ============================================================================
# Tier Rolling
# ============================================================================


func _roll_tier() -> int:
	var total_weight = 0
	for tier in TIER_WEIGHTS:
		total_weight += TIER_WEIGHTS[tier]

	var roll = randi() % total_weight
	var cumulative = 0

	for tier in TIER_WEIGHTS:
		cumulative += TIER_WEIGHTS[tier]
		if roll < cumulative:
			return tier

	return TraitManager.TraitTier.BRONZE


func _roll_tier_with_performance_bonus(goals: int, assists: int, is_motm: bool) -> int:
	# Better performance = higher chance of better tiers
	var bronze_weight = TIER_WEIGHTS[TraitManager.TraitTier.BRONZE]
	var silver_weight = TIER_WEIGHTS[TraitManager.TraitTier.SILVER]
	var gold_weight = TIER_WEIGHTS[TraitManager.TraitTier.GOLD]

	# Shift weights based on performance
	var performance_bonus = goals * 5 + assists * 3 + (10 if is_motm else 0)

	# Move weight from bronze to silver/gold
	var bronze_reduction = min(bronze_weight * 0.5, performance_bonus)
	bronze_weight -= bronze_reduction
	silver_weight += bronze_reduction * 0.7
	gold_weight += bronze_reduction * 0.3

	var total_weight = bronze_weight + silver_weight + gold_weight
	var roll = randf() * total_weight
	var cumulative = 0.0

	cumulative += bronze_weight
	if roll < cumulative:
		return TraitManager.TraitTier.BRONZE

	cumulative += silver_weight
	if roll < cumulative:
		return TraitManager.TraitTier.SILVER

	return TraitManager.TraitTier.GOLD


# ============================================================================
# Trait Selection
# ============================================================================


func _pick_random_trait_from_category(category: int) -> String:
	var traits = TraitManager.get_traits_by_category(category)
	if traits.is_empty():
		# Fallback to any trait
		var all_traits = TraitManager.TRAIT_DATA.keys()
		return all_traits[randi() % all_traits.size()]
	return traits[randi() % traits.size()]


# ============================================================================
# Notifications
# ============================================================================


func _show_drop_notification(trait_type: String, tier: int, source: String):
	var display = TraitManager.get_trait_display(trait_type, tier)
	var message = "%s %s %s 획득! (%s)" % [display.tier_icon, display.icon, display.name_ko, source]

	if has_node("/root/UIService"):
		get_node("/root/UIService").show_success_toast(message)
	else:
		print("[TraitAcquisitionSystem] %s" % message)


# ============================================================================
# Manual Trait Granting (for shop, events, etc.)
# ============================================================================


## Grant a specific trait to player
func grant_trait(player_id: String, trait_type: String, tier: int, source: String = "reward") -> bool:
	if TraitManager.add_trait_to_inventory(player_id, trait_type, tier):
		trait_dropped.emit(player_id, trait_type, tier, source)
		_show_drop_notification(trait_type, tier, source)
		return true
	return false


## Grant a random trait from category
func grant_random_trait(player_id: String, category: int, tier: int = -1, source: String = "reward") -> bool:
	var trait_type = _pick_random_trait_from_category(category)
	var actual_tier = tier if tier > 0 else _roll_tier()
	return grant_trait(player_id, trait_type, actual_tier, source)


## Grant random trait (any category)
func grant_random_any_trait(player_id: String, source: String = "reward") -> bool:
	var categories = [
		TraitManager.TraitCategory.SHOOTING,
		TraitManager.TraitCategory.PASSING,
		TraitManager.TraitCategory.DRIBBLING,
		TraitManager.TraitCategory.DEFENSE
	]
	var category = categories[randi() % categories.size()]
	return grant_random_trait(player_id, category, -1, source)


# ============================================================================
# Testing
# ============================================================================


func test_training_drop(player_id: String = "player_0"):
	var event = {"player_id": player_id, "training_id": "shooting", "quality": 1.5}
	_on_training_completed(event)


func test_match_drop(_player_id: String = "player_0"):
	var result = {
		"result": "win", "player_stats": {"position": "ST", "goals": 2, "assists": 1, "man_of_the_match": true}
	}
	_on_match_ended(result)
