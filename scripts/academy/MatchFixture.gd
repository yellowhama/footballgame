## MatchFixture.gd
## Match fixture data structure with type-based multipliers
## Represents a scheduled match in Academy Mode
extends Resource
class_name MatchFixture

## ============================================================================
## ENUMS
## ============================================================================

## Match types with different characteristics
enum MatchType { LEAGUE, CUP, FRIENDLY, UEFA_YOUTH_LEAGUE }  ## Regular league matches (most common)  ## Cup competitions (high importance)  ## Friendly matches (low pressure)  ## UEFA Youth League (highest importance)

## ============================================================================
## CORE PROPERTIES
## ============================================================================

## Match identification
@export var match_id: int = 0  ## Unique match ID (auto-generated)
@export var fixture_index: int = 0  ## Index in season fixtures (0-based)

## Scheduling information
@export var scheduled_year: int = 1  ## Year 1-3 (U16/U17/U18)
@export var scheduled_week: int = 1  ## Week 1-38 within year
@export var scheduled_day_of_week: int = 5  ## 0-6 (Monday=0, Saturday=5, Wednesday=2)
@export var absolute_day: int = 0  ## Absolute day number (1-798)

## Match type and characteristics
@export var match_type: MatchType = MatchType.LEAGUE
@export var is_home_match: bool = true  ## Home vs Away

## Opponent information
@export var opponent_name: String = ""  ## e.g., "Arsenal U16", "Chelsea U17"
@export var opponent_strength: int = 50  ## 0-100 (used for match simulation)

## Importance and multipliers
@export var importance: float = 1.0  ## Base importance (0.5-2.0)
@export var fatigue_multiplier: float = 1.0  ## Stamina cost multiplier
@export var ca_growth_multiplier: float = 1.0  ## CA growth reward multiplier
@export var morale_impact_multiplier: float = 1.0  ## Morale change multiplier

## Match result (filled after match completion)
@export var is_completed: bool = false
@export var match_result: Dictionary = {}  ## From OpenFootballAPI.simulate_match()

## ============================================================================
## CONSTANTS
## ============================================================================

## Match type configurations
const MATCH_TYPE_CONFIG = {
	MatchType.LEAGUE:
	{
		"name": "League",
		"importance": 1.0,
		"fatigue_multiplier": 1.0,
		"ca_growth_multiplier": 1.0,
		"morale_impact_multiplier": 1.0,
		"stamina_cost_base": 20
	},
	MatchType.CUP:
	{
		"name": "Cup",
		"importance": 1.5,
		"fatigue_multiplier": 1.2,
		"ca_growth_multiplier": 1.3,
		"morale_impact_multiplier": 1.5,
		"stamina_cost_base": 25
	},
	MatchType.FRIENDLY:
	{
		"name": "Friendly",
		"importance": 0.5,
		"fatigue_multiplier": 0.7,
		"ca_growth_multiplier": 0.8,
		"morale_impact_multiplier": 0.5,
		"stamina_cost_base": 15
	},
	MatchType.UEFA_YOUTH_LEAGUE:
	{
		"name": "UEFA Youth League",
		"importance": 2.0,
		"fatigue_multiplier": 1.5,
		"ca_growth_multiplier": 1.5,
		"morale_impact_multiplier": 2.0,
		"stamina_cost_base": 30
	}
}

## ============================================================================
## INITIALIZATION
## ============================================================================


func _init(
	p_match_type: MatchType = MatchType.LEAGUE, p_year: int = 1, p_week: int = 1, p_day_of_week: int = 5
) -> void:
	match_type = p_match_type
	scheduled_year = p_year
	scheduled_week = p_week
	scheduled_day_of_week = p_day_of_week

	# Apply default multipliers based on match type
	_apply_match_type_config()


## ============================================================================
## CONFIGURATION METHODS
## ============================================================================


## Apply match type configuration to multipliers
func _apply_match_type_config() -> void:
	var config = MATCH_TYPE_CONFIG.get(match_type, MATCH_TYPE_CONFIG[MatchType.LEAGUE])

	importance = config.importance
	fatigue_multiplier = config.fatigue_multiplier
	ca_growth_multiplier = config.ca_growth_multiplier
	morale_impact_multiplier = config.morale_impact_multiplier


## Set match type and update multipliers
func set_match_type(new_type: MatchType) -> void:
	match_type = new_type
	_apply_match_type_config()


## Set custom multipliers (overrides match type defaults)
func set_custom_multipliers(
	p_importance: float = -1.0, p_fatigue: float = -1.0, p_ca_growth: float = -1.0, p_morale_impact: float = -1.0
) -> void:
	if p_importance >= 0.0:
		importance = p_importance
	if p_fatigue >= 0.0:
		fatigue_multiplier = p_fatigue
	if p_ca_growth >= 0.0:
		ca_growth_multiplier = p_ca_growth
	if p_morale_impact >= 0.0:
		morale_impact_multiplier = p_morale_impact


## ============================================================================
## STAMINA CALCULATION
## ============================================================================


## Calculate stamina cost for this match
func calculate_stamina_cost(base_stamina_cost: int = 0) -> int:
	# Use match type base cost if not provided
	if base_stamina_cost == 0:
		var config = MATCH_TYPE_CONFIG.get(match_type, MATCH_TYPE_CONFIG[MatchType.LEAGUE])
		base_stamina_cost = config.stamina_cost_base

	var total_cost = int(base_stamina_cost * fatigue_multiplier)

	# Home advantage: -10% stamina cost
	if is_home_match:
		total_cost = int(total_cost * 0.9)

	return total_cost


## ============================================================================
## CA GROWTH CALCULATION
## ============================================================================


## Apply CA growth multiplier to match result
func apply_ca_growth_multiplier(base_ca_growth: int) -> int:
	return int(base_ca_growth * ca_growth_multiplier)


## ============================================================================
## MORALE CALCULATION
## ============================================================================


## Calculate morale change from match result
func calculate_morale_change(match_result_dict: Dictionary) -> int:
	var base_morale = 0

	# Result-based morale
	if match_result_dict.get("is_win", false):
		base_morale = 10
	elif match_result_dict.get("is_draw", false):
		base_morale = 2
	else:
		base_morale = -8

	# Performance adjustment (rating-based)
	var rating = match_result_dict.get("player_rating", 6.0)
	if rating >= 8.0:
		base_morale += 5
	elif rating >= 7.0:
		base_morale += 2
	elif rating < 6.0:
		base_morale -= 3

	# Apply morale impact multiplier
	var final_morale = int(base_morale * morale_impact_multiplier)

	return final_morale


## ============================================================================
## MATCH COMPLETION
## ============================================================================


## Mark match as completed with results
func complete_match(result: Dictionary) -> void:
	is_completed = true
	match_result = result


## Check if match is completed
func is_match_completed() -> bool:
	return is_completed


## ============================================================================
## HELPER METHODS
## ============================================================================


## Get match type name as string
func get_match_type_name() -> String:
	var config = MATCH_TYPE_CONFIG.get(match_type, MATCH_TYPE_CONFIG[MatchType.LEAGUE])
	return config.name


## Get full match description
func get_match_description() -> String:
	var home_away = "vs" if is_home_match else "@"
	var type_name = get_match_type_name()
	return "%s %s %s (Week %d)" % [type_name, home_away, opponent_name, scheduled_week]


## Get importance level as string
func get_importance_level() -> String:
	if importance >= 2.0:
		return "Critical"
	elif importance >= 1.5:
		return "Very High"
	elif importance >= 1.0:
		return "High"
	elif importance >= 0.7:
		return "Medium"
	else:
		return "Low"


## ============================================================================
## SERIALIZATION
## ============================================================================


## Convert to dictionary for save system
func to_dict() -> Dictionary:
	return {
		"match_id": match_id,
		"fixture_index": fixture_index,
		"scheduled_year": scheduled_year,
		"scheduled_week": scheduled_week,
		"scheduled_day_of_week": scheduled_day_of_week,
		"absolute_day": absolute_day,
		"match_type": match_type,
		"is_home_match": is_home_match,
		"opponent_name": opponent_name,
		"opponent_strength": opponent_strength,
		"importance": importance,
		"fatigue_multiplier": fatigue_multiplier,
		"ca_growth_multiplier": ca_growth_multiplier,
		"morale_impact_multiplier": morale_impact_multiplier,
		"is_completed": is_completed,
		"match_result": match_result
	}


## Create from dictionary (for load system)
static func from_dict(data: Dictionary):  # Returns MatchFixture
	# Static functions cannot use class_name for instantiation - use Resource.new() and set script
	var fixture = load("res://scripts/academy/MatchFixture.gd").new()

	fixture.match_id = data.get("match_id", 0)
	fixture.fixture_index = data.get("fixture_index", 0)
	fixture.scheduled_year = data.get("scheduled_year", 1)
	fixture.scheduled_week = data.get("scheduled_week", 1)
	fixture.scheduled_day_of_week = data.get("scheduled_day_of_week", 5)
	fixture.absolute_day = data.get("absolute_day", 0)
	fixture.match_type = data.get("match_type", MatchType.LEAGUE)
	fixture.is_home_match = data.get("is_home_match", true)
	fixture.opponent_name = data.get("opponent_name", "")
	fixture.opponent_strength = data.get("opponent_strength", 50)
	fixture.importance = data.get("importance", 1.0)
	fixture.fatigue_multiplier = data.get("fatigue_multiplier", 1.0)
	fixture.ca_growth_multiplier = data.get("ca_growth_multiplier", 1.0)
	fixture.morale_impact_multiplier = data.get("morale_impact_multiplier", 1.0)
	fixture.is_completed = data.get("is_completed", false)
	fixture.match_result = data.get("match_result", {})

	return fixture
