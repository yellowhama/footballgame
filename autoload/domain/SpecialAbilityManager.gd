extends Node
## SpecialAbilityManager - Phase 3 Special Abilities System
## Manages 12 abilities × 5 tiers (60 abilities) with combination system
## Part of domain/ autoload layer (Business logic)

signal ability_acquired(player_id: String, ability_type: String, tier: int)
signal ability_upgraded(player_id: String, ability_type: String, from_tier: int, to_tier: int)
signal combination_triggered(player_id: String, combinations: Array)
signal ability_activated(player_id: String, ability_type: String, effect_multiplier: float)

## Ability types (12 categories)
enum AbilityType {
	DRIBBLING_MASTER,  # 드리블의 달인
	FINISHING_SPECIALIST,  # 마무리의 달인
	PASSING_MAESTRO,  # 패스의 달인
	DEFENSIVE_WALL,  # 수비의 벽
	SPEED_DEMON,  # 질주하는 악마
	AERIAL_THREAT,  # 공중의 지배자
	PLAYMAKER,  # 플레이메이커
	CLINICAL_FINISHER,  # 결정력
	CLUTCH_PLAYER,  # 클러치
	LEADERSHIP,  # 리더십
	WORKHORSE,  # 워크호스
	TECHNICAL_GENIUS  # 테크니션
}

## Ability tiers (5 levels)
enum Tier { BRONZE = 1, SILVER = 2, GOLD = 3, DIAMOND = 4, LEGEND = 5 }  # 브론즈  # 실버  # 골드  # 다이아몬드  # 레전드

## Player ability collections
## Structure: { player_id: { abilities: [], combination_history: [] } }
var _player_abilities: Dictionary = {}

## Ability type name mapping (for JSON communication with Rust)
const ABILITY_TYPE_NAMES = {
	AbilityType.DRIBBLING_MASTER: "DribblingMaster",
	AbilityType.FINISHING_SPECIALIST: "FinishingSpecialist",
	AbilityType.PASSING_MAESTRO: "PassingMaestro",
	AbilityType.DEFENSIVE_WALL: "DefensiveWall",
	AbilityType.SPEED_DEMON: "SpeedDemon",
	AbilityType.AERIAL_THREAT: "AerialThreat",
	AbilityType.PLAYMAKER: "Playmaker",
	AbilityType.CLINICAL_FINISHER: "ClinicalFinisher",
	AbilityType.CLUTCH_PLAYER: "ClutchPlayer",
	AbilityType.LEADERSHIP: "Leadership",
	AbilityType.WORKHORSE: "Workhorse",
	AbilityType.TECHNICAL_GENIUS: "TechnicalGenius"
}

## Korean names for UI
const ABILITY_KOREAN_NAMES = {
	"DribblingMaster": "드리블의 달인",
	"FinishingSpecialist": "마무리의 달인",
	"PassingMaestro": "패스의 달인",
	"DefensiveWall": "수비의 벽",
	"SpeedDemon": "질주하는 악마",
	"AerialThreat": "공중의 지배자",
	"Playmaker": "플레이메이커",
	"ClinicalFinisher": "결정력",
	"ClutchPlayer": "클러치",
	"Leadership": "리더십",
	"Workhorse": "워크호스",
	"TechnicalGenius": "테크니션"
}

const TIER_KOREAN_NAMES = {1: "브론즈", 2: "실버", 3: "골드", 4: "다이아몬드", 5: "레전드"}


func _ready() -> void:
	print("[SpecialAbilityManager] Initialized")


## ========== Player Ability Collection Management ==========


## Initialize ability collection for a new player
func initialize_player(player_id: String) -> void:
	if _player_abilities.has(player_id):
		push_warning("[SpecialAbilityManager] Player %s already initialized" % player_id)
		return

	_player_abilities[player_id] = {"abilities": [], "combination_history": []}
	print("[SpecialAbilityManager] Initialized player: %s" % player_id)


## Get player's ability collection
func get_player_abilities(player_id: String) -> Dictionary:
	if not _player_abilities.has(player_id):
		push_warning("[SpecialAbilityManager] Player %s not found, initializing" % player_id)
		initialize_player(player_id)

	return _player_abilities[player_id]


## Add ability to player
func add_ability(player_id: String, ability_type: String, tier: int) -> bool:
	if not _player_abilities.has(player_id):
		initialize_player(player_id)

	# Validate tier
	if tier < Tier.BRONZE or tier > Tier.LEGEND:
		push_error("[SpecialAbilityManager] Invalid tier: %d" % tier)
		return false

	# Check if player already has this ability type
	var collection = _player_abilities[player_id]
	for ability in collection.abilities:
		if ability.ability_type == ability_type:
			push_warning("[SpecialAbilityManager] Player already has %s, use upgrade instead" % ability_type)
			return false

	# Add new ability
	var new_ability = {"ability_type": ability_type, "tier": tier, "acquired_at": Time.get_ticks_msec()}
	collection.abilities.append(new_ability)

	# Emit signal
	ability_acquired.emit(player_id, ability_type, tier)
	print("[SpecialAbilityManager] Added %s (%s) to player %s" % [ability_type, TIER_KOREAN_NAMES[tier], player_id])

	return true


## Upgrade ability tier
func upgrade_ability(player_id: String, ability_type: String, new_tier: int) -> bool:
	if not _player_abilities.has(player_id):
		push_error("[SpecialAbilityManager] Player %s not found" % player_id)
		return false

	var collection = _player_abilities[player_id]
	for ability in collection.abilities:
		if ability.ability_type == ability_type:
			var old_tier = ability.tier

			# Validate upgrade
			if new_tier <= old_tier:
				push_error("[SpecialAbilityManager] New tier must be higher than current tier")
				return false

			if new_tier > Tier.LEGEND:
				push_error("[SpecialAbilityManager] Tier cannot exceed LEGEND")
				return false

			# Upgrade
			ability.tier = new_tier
			ability.upgraded_at = Time.get_ticks_msec()

			# Emit signal
			ability_upgraded.emit(player_id, ability_type, old_tier, new_tier)
			print(
				(
					"[SpecialAbilityManager] Upgraded %s from %s to %s for player %s"
					% [ability_type, TIER_KOREAN_NAMES[old_tier], TIER_KOREAN_NAMES[new_tier], player_id]
				)
			)

			return true

	push_error("[SpecialAbilityManager] Player does not have %s ability" % ability_type)
	return false


## Remove ability from player
func remove_ability(player_id: String, ability_type: String) -> bool:
	if not _player_abilities.has(player_id):
		return false

	var collection = _player_abilities[player_id]
	for i in range(collection.abilities.size()):
		if collection.abilities[i].ability_type == ability_type:
			collection.abilities.remove_at(i)
			print("[SpecialAbilityManager] Removed %s from player %s" % [ability_type, player_id])
			return true

	return false


## ========== Rust Integration ==========


## Calculate combined effects of all player abilities (Rust)
func calculate_player_effects(player_id: String) -> Dictionary:
	var collection = get_player_abilities(player_id)

	if collection.abilities.is_empty():
		return {"success": true, "effects": {}, "total_abilities": 0}

	# Use OpenFootballAPI wrapper instead of direct Rust call
	return OpenFootballAPI.calculate_ability_effects(collection.abilities)


## Process automatic ability combinations (Rust)
## Bronze × 3 → Silver, Silver × 3 → Gold, etc.
func process_combinations(player_id: String, context: Dictionary = {}) -> Dictionary:
	var collection = get_player_abilities(player_id)

	# Default context
	if context.is_empty():
		context = {"current_turn": 1, "games_played": 0, "is_team_captain": false, "personality_archetype": "Steady"}

	# Use OpenFootballAPI wrapper instead of direct Rust call
	var result = OpenFootballAPI.process_ability_combinations(collection, context)

	if result.get("success", false) and result.get("combinations", []).size() > 0:
		# Update abilities based on combinations
		for combo in result.combinations:
			if combo.has("resulting_ability"):
				var resulting = combo.resulting_ability
				# Remove consumed abilities
				for consumed in combo.consumed_abilities:
					remove_ability(player_id, consumed.ability_type)
				# Add resulting ability
				add_ability(player_id, resulting.ability_type, resulting.tier)

		# Record combination history
		collection.combination_history.append({"timestamp": Time.get_ticks_msec(), "combinations": result.combinations})

		# Emit signal
		combination_triggered.emit(player_id, result.combinations)
		print(
			"[SpecialAbilityManager] Processed %d combinations for player %s" % [result.combinations.size(), player_id]
		)

	return result


## Check if ability should be acquired during training (Rust)
func check_acquisition_during_training(
	training_type: String, quality: float, coach_specialty: String = ""
) -> Dictionary:
	# Validate training type
	var valid_types = ["technical", "mental", "physical"]
	if not training_type in valid_types:
		return {
			"success": false, "error": "Invalid training type: %s (must be technical/mental/physical)" % training_type
		}

	# Clamp quality
	quality = clamp(quality, 0.0, 10.0)

	# Use OpenFootballAPI wrapper instead of direct Rust call
	return OpenFootballAPI.check_ability_acquisition(training_type, quality, coach_specialty)


## Check if ability activates during match (Rust)
## For situational abilities like ClutchPlayer
func check_activation_in_match(player_id: String, ability_type: String, match_context: Dictionary) -> Dictionary:
	var collection = get_player_abilities(player_id)

	# Find ability
	var ability_data = null
	for ability in collection.abilities:
		if ability.ability_type == ability_type:
			ability_data = ability
			break

	if not ability_data:
		return {"activated": false, "error": "Player does not have ability: %s" % ability_type}

	# Default match context
	if not match_context.has("match_minute"):
		match_context.match_minute = 45
	if not match_context.has("score_difference"):
		match_context.score_difference = 0
	if not match_context.has("pressure_level"):
		match_context.pressure_level = 0.5

	# Use OpenFootballAPI wrapper (placeholder - need to add to API)
	var result = FootballRustEngine.check_ability_activation(ability_data, match_context)

	if result.get("activated", false):
		# Emit signal
		var multiplier = result.get("effect_multiplier", 1.0)
		ability_activated.emit(player_id, ability_type, multiplier)
		print("[SpecialAbilityManager] %s activated for player %s (×%.2f)" % [ability_type, player_id, multiplier])

	return result


## ========== Utility Functions ==========


## Get ability type name for Rust communication
func get_ability_type_name(ability_type: int) -> String:
	if ABILITY_TYPE_NAMES.has(ability_type):
		return ABILITY_TYPE_NAMES[ability_type]
	return ""


## Get Korean name for UI display
func get_ability_korean_name(ability_type: String) -> String:
	if ABILITY_KOREAN_NAMES.has(ability_type):
		return ABILITY_KOREAN_NAMES[ability_type]
	return ability_type


## Get tier Korean name
func get_tier_korean_name(tier: int) -> String:
	if TIER_KOREAN_NAMES.has(tier):
		return TIER_KOREAN_NAMES[tier]
	return "Unknown"


## Count abilities by tier for a player
func count_abilities_by_tier(player_id: String) -> Dictionary:
	var collection = get_player_abilities(player_id)
	var counts = {Tier.BRONZE: 0, Tier.SILVER: 0, Tier.GOLD: 0, Tier.DIAMOND: 0, Tier.LEGEND: 0}

	for ability in collection.abilities:
		var tier = ability.tier
		if counts.has(tier):
			counts[tier] += 1

	return counts


## Get all abilities for a specific tier
func get_abilities_by_tier(player_id: String, tier: int) -> Array:
	var collection = get_player_abilities(player_id)
	var result = []

	for ability in collection.abilities:
		if ability.tier == tier:
			result.append(ability)

	return result


## Check if player has specific ability
func has_ability(player_id: String, ability_type: String) -> bool:
	var collection = get_player_abilities(player_id)

	for ability in collection.abilities:
		if ability.ability_type == ability_type:
			return true

	return false


## Get ability tier (returns 0 if not found)
func get_ability_tier(player_id: String, ability_type: String) -> int:
	var collection = get_player_abilities(player_id)

	for ability in collection.abilities:
		if ability.ability_type == ability_type:
			return ability.tier

	return 0


## ========== Save/Load ==========


## Export player abilities for saving
func export_player_data(player_id: String) -> Dictionary:
	if not _player_abilities.has(player_id):
		return {}

	return _player_abilities[player_id].duplicate(true)


## Import player abilities from save data
func import_player_data(player_id: String, data: Dictionary) -> void:
	_player_abilities[player_id] = data.duplicate(true)
	print(
		(
			"[SpecialAbilityManager] Imported data for player: %s (%d abilities)"
			% [player_id, data.abilities.size() if data.has("abilities") else 0]
		)
	)


## Clear all player data (for testing)
func clear_all_data() -> void:
	_player_abilities.clear()
	print("[SpecialAbilityManager] Cleared all player ability data")


## ========== Testing ==========


## Test special ability system (Rust)
func test_system() -> Dictionary:
	return FootballRustEngine.test_special_ability_system()


## Create test player with random abilities
func create_test_player(player_id: String, num_abilities: int = 3) -> void:
	initialize_player(player_id)

	var ability_types = ABILITY_TYPE_NAMES.values()
	var added = 0

	while added < num_abilities and added < ability_types.size():
		var random_type = ability_types[randi() % ability_types.size()]
		var random_tier = (randi() % Tier.LEGEND) + 1

		if add_ability(player_id, random_type, random_tier):
			added += 1

	print("[SpecialAbilityManager] Created test player %s with %d abilities" % [player_id, added])


## Print player abilities (for debugging)
func print_player_abilities(player_id: String) -> void:
	var collection = get_player_abilities(player_id)

	print("========== Player %s Abilities ==========" % player_id)
	print("Total: %d abilities" % collection.abilities.size())

	for ability in collection.abilities:
		var korean_name = get_ability_korean_name(ability.ability_type)
		var tier_name = get_tier_korean_name(ability.tier)
		print("  - %s (%s) [%s]" % [korean_name, ability.ability_type, tier_name])

	print("Combination History: %d entries" % collection.combination_history.size())
	print("==========================================")
