class_name MVPCareerHelper
extends RefCounted
## ============================================================================
## MVPCareerHelper - MVP Mode Match Scheduling
## ============================================================================
##
## PURPOSE: Manage MVP (single career) mode match schedule and recommendations
##
## EXTRACTED FROM: MatchSimulationManager.gd (ST-005 God Class refactoring)
##
## RESPONSIBILITIES:
## - Track upcoming MVP matches
## - Provide next match information
## - Recommend team training based on opponent
## - Mark matches as played
##
## USAGE:
##   var helper := MVPCareerHelper.new()
##   var next_match := helper.get_mvp_next_match()
##   var training := helper.get_recommended_team_training(next_match)
## ============================================================================

## MVP match schedule (copy of MatchSimulationManager.MVP_MATCH_SCHEDULE)
const MVP_MATCH_SCHEDULE = [
	{
		"week": 1,
		"type": "friendly",
		"opponent": "Academy Unity",
		"importance": 4,
		"overall_rating": 62,
		"seed": 744120001
	},
	{"week": 2, "type": "league", "opponent": "Rising Stars", "importance": 5, "overall_rating": 64, "seed": 744120002},
	{"week": 3, "type": "league", "opponent": "North Ridge", "importance": 5, "overall_rating": 65, "seed": 744120003},
	{
		"week": 4,
		"type": "league",
		"opponent": "Metro Juniors",
		"importance": 6,
		"overall_rating": 66,
		"seed": 744120004
	},
	{"week": 5, "type": "cup", "opponent": "Cup Challengers", "importance": 6, "overall_rating": 67, "seed": 744120005},
	{"week": 6, "type": "league", "opponent": "East Academy", "importance": 6, "overall_rating": 68, "seed": 744120006},
	{"week": 7, "type": "league", "opponent": "South City", "importance": 6, "overall_rating": 69, "seed": 744120007},
	{"week": 8, "type": "league", "opponent": "Westfield", "importance": 7, "overall_rating": 70, "seed": 744120008},
	{"week": 9, "type": "league", "opponent": "Capstone FC", "importance": 7, "overall_rating": 72, "seed": 744120009},
	{"week": 10, "type": "cup", "opponent": "District Elite", "importance": 7, "overall_rating": 74, "seed": 744120010},
	{
		"week": 11,
		"type": "league",
		"opponent": "Highland Youth",
		"importance": 8,
		"overall_rating": 76,
		"seed": 744120011
	},
	{
		"week": 12,
		"type": "league",
		"opponent": "Elite Juniors",
		"importance": 8,
		"overall_rating": 78,
		"seed": 744120012
	}
]

## Training recommendations by match type
const TEAM_TRAINING_BY_MATCH_TYPE = {
	"friendly": "physical",
	"cup": "tactical",
	"league": "tactical",
	"national": "defending",
	"national_qualifier": "defending"
}

## State: upcoming matches (mutable)
var upcoming_matches: Array = []


func _init() -> void:
	reset_schedule()


## Get the next scheduled MVP match
func get_mvp_next_match() -> Dictionary:
	if upcoming_matches.is_empty():
		return {}
	var next_match = upcoming_matches[0].duplicate(true)
	next_match["year"] = 1
	return next_match


## Get MVP match for a specific week
func get_mvp_match_for_week(week: int) -> Dictionary:
	for match_info in upcoming_matches:
		if match_info.get("week", 0) == week:
			var match_copy = match_info.duplicate(true)
			match_copy["year"] = 1
			return match_copy
	return {}


## Recommend team training based on upcoming match
func get_recommended_team_training(match_data: Dictionary, training_manager: Node = null) -> String:
	var default_training := "tactical"
	if match_data.is_empty():
		return default_training

	var match_type := String(match_data.get("type", "league")).to_lower()
	var importance := int(match_data.get("importance", 5))
	var opponent_rating := int(match_data.get("overall_rating", match_data.get("opponent_rating", 65)))
	var recommendation: String = String(TEAM_TRAINING_BY_MATCH_TYPE.get(match_type, default_training))

	if importance >= 8 or opponent_rating >= 75:
		recommendation = "defending"
	elif importance <= 4 and match_type == "friendly":
		recommendation = "physical"
	elif opponent_rating <= 65 and match_type == "league":
		recommendation = "shooting"

	# Validate with TrainingManager if available
	if training_manager and training_manager.has_method("get_training_by_id"):
		var program = training_manager.get_training_by_id(recommendation)
		if program.is_empty():
			return default_training

	return recommendation


## Mark a match as played (remove from schedule)
func mark_mvp_match_played(week: int) -> void:
	for i in range(upcoming_matches.size()):
		var scheduled_match: Dictionary = upcoming_matches[i]
		if scheduled_match.get("week", 0) == week:
			upcoming_matches.remove_at(i)
			return


## Reset schedule to initial state
func reset_schedule() -> void:
	upcoming_matches = MVP_MATCH_SCHEDULE.duplicate(true)


## Get count of remaining matches
func get_remaining_match_count() -> int:
	return upcoming_matches.size()


## Check if all matches are completed
func is_schedule_complete() -> bool:
	return upcoming_matches.is_empty()
