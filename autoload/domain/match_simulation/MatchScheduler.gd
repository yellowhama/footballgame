class_name MatchScheduler
extends RefCounted
## ============================================================================
## MatchScheduler - Match Scheduling Logic
## ============================================================================
##
## PURPOSE: Handle match scheduling for career mode (league and cup matches)
##
## EXTRACTED FROM: MatchSimulationManager.gd (ST-005 God Class refactoring)
##
## RESPONSIBILITIES:
## - Initialize match schedule for each season
## - Respond to week advancement
## - Determine if current week has a match (cup or league)
## - Calculate match importance
## - Track and consume scheduled matches
##
## DEPENDENCIES:
## - GameManager: Current year
## - CupManager: Cup match detection
## - DivisionManager: Legacy league fallback
## - StageManager: League SSOT (stage_league)
## - TacticsManager: Formation normalization (optional)
##
## USAGE:
##   var scheduler := MatchScheduler.new()
##   scheduler.initialize(tactics_manager, mvp_reset_callback)
##   var match_data := scheduler.get_match_for_week(week, year)
## ============================================================================

# ============================================================================
# SIGNALS
# ============================================================================

signal match_scheduled(match_data: Dictionary)

# ============================================================================
# CONSTANTS
# ============================================================================

## League match weeks: 8, 12, 20, 28, 36, 44 (6 total per season)
const CAREER_LEAGUE_MATCH_WEEKS := [8, 12, 20, 28, 36, 44]
const CAREER_LEAGUE_OPPONENT_SOURCE := "stage_league"

# ============================================================================
# STATE
# ============================================================================

var upcoming_matches: Array = []

# ============================================================================
# DEPENDENCIES
# ============================================================================

var _tactics_manager: RefCounted = null
var _mvp_reset_callback: Callable = Callable()
var _is_ready: bool = false

# ============================================================================
# INITIALIZATION
# ============================================================================


func initialize(tactics_manager: RefCounted = null, mvp_reset_callback: Callable = Callable()) -> void:
	"""Initialize MatchScheduler with optional dependencies"""
	_tactics_manager = tactics_manager
	_mvp_reset_callback = mvp_reset_callback
	_is_ready = true


# ============================================================================
# PUBLIC API
# ============================================================================


func initialize_match_schedule() -> void:
	"""Initialize upcoming matches for current year"""
	upcoming_matches.clear()

	# Get current year from GameManager or default to 1
	var current_year := 1
	var game_manager := _get_game_manager()
	if game_manager:
		current_year = game_manager.current_year

	# Phase 23 (Career): league opponents resolved via StageManager (league_config.json SSOT).
	# We keep the schedule lightweight; opponent selection is performed when the week advances.
	upcoming_matches = []
	print("[MatchScheduler] Match schedule initialized for Year %d (dynamic league opponents)" % current_year)

	# Reset MVP schedule via callback
	if _mvp_reset_callback.is_valid():
		_mvp_reset_callback.call()


func on_week_advanced(week: int, year: int) -> Dictionary:
	"""Check if there's a match this week
	@return: match_data Dictionary if match found, empty Dictionary otherwise
	"""
	var match_data: Dictionary = get_match_for_week(week, year)

	if not match_data.is_empty():
		print("[MatchScheduler] Match scheduled: Week %d, Year %d" % [week, year])
		match_scheduled.emit(match_data)

	return match_data


func on_season_completed(year: int) -> void:
	"""Update match schedule for new year"""
	print("[MatchScheduler] Season %d completed, loading Year %d matches" % [year, year + 1])
	initialize_match_schedule()

	# Reset cup state for new season (Phase 22)
	var cup_manager := _get_cup_manager()
	if cup_manager:
		cup_manager.reset_cup(year + 1)
		print("[MatchScheduler] Cup reset for season %d" % (year + 1))


func get_match_for_week(week: int, year: int) -> Dictionary:
	"""Find if there's a match scheduled for this week (cup or league)"""

	# 1. Check if this is a cup match week (priority)
	var cup_manager := _get_cup_manager()
	if cup_manager and cup_manager.is_cup_match_week(week):
		var opponent = cup_manager.get_cup_opponent(cup_manager.current_stage)
		var stage_name = cup_manager.get_current_stage_name()

		print("[MatchScheduler] Cup match found: %s (Week %d)" % [stage_name, week])

		return {
			"week": week,
			"year": year,
			"type": "cup",
			"stage": stage_name,
			"opponent": opponent.name,
			"importance": opponent.importance,
			"ca_min": opponent.ca_min,
			"ca_max": opponent.ca_max,
			"is_cup_match": true
		}

	# 2. Check if this is a league match week (Career SSOT: StageManager league_config.json)
	if is_career_league_match_week(week):
		var importance := calculate_match_importance(week, year)
		var match_payload := {
			"week": week, "year": year, "type": "league", "importance": importance, "is_cup_match": false
		}
		match_payload = _apply_stage_league_ssot(match_payload)
		if String(match_payload.get("opponent_source", "")) == CAREER_LEAGUE_OPPONENT_SOURCE:
			print(
				(
					"[MatchScheduler] League match (Stage SSOT): %s (Week %d, League %s)"
					% [match_payload.get("opponent", "Unknown"), week, str(match_payload.get("league_id", "?"))]
				)
			)
			return match_payload

		# Legacy fallback: keep DivisionManager path if StageManager is missing/unavailable.
		var division_manager := _get_division_manager()
		if division_manager and division_manager.is_league_match_week(week):
			var opponent = division_manager.get_opponent_for_week(week)

			if opponent.is_empty():
				push_error("[MatchScheduler] DivisionManager returned empty opponent for week %d" % week)
				return {}

			print(
				(
					"[MatchScheduler] League match (legacy DivisionManager): %s (Week %d, Division %d)"
					% [opponent.name, week, division_manager.current_division]
				)
			)

			return {
				"week": week,
				"year": year,
				"type": "league",
				"opponent": opponent.name,
				"importance": importance,
				"ca_min": opponent.ca_range[0],
				"ca_max": opponent.ca_range[1],
				"division": division_manager.current_division,
				"is_cup_match": false
			}

	return {}


func calculate_match_importance(week: int, year: int) -> int:
	"""Calculate match importance based on week and year (Phase 23)"""
	# Week 44 (final match) = highest importance
	if week == 44:
		return 8 if year == 3 else 7
	# Mid-season matches (week 28+)
	elif week >= 28:
		return 6
	# Early season
	else:
		return 5


func consume_scheduled_match(match_data: Dictionary) -> void:
	"""Remove a played match from the upcoming schedule"""
	var target_week: int = int(match_data.get("week", -1))
	var target_type: String = String(match_data.get("type", ""))
	var target_opponent: String = String(match_data.get("opponent", ""))

	for i in range(upcoming_matches.size()):
		var scheduled: Dictionary = upcoming_matches[i]
		if (
			int(scheduled.get("week", -1)) == target_week
			and String(scheduled.get("type", "")) == target_type
			and String(scheduled.get("opponent", "")) == target_opponent
		):
			upcoming_matches.remove_at(i)
			break


func is_career_league_match_week(week: int) -> bool:
	"""Check if the given week is a career league match week"""
	return CAREER_LEAGUE_MATCH_WEEKS.has(week)


# ============================================================================
# PRIVATE HELPERS
# ============================================================================


func _apply_stage_league_ssot(match_data: Dictionary) -> Dictionary:
	"""Inject StageManager league_config SSOT fields into a league match_data (opponent, ca range, formation)."""
	var match_type := String(match_data.get("type", "")).to_lower()
	if match_type != "league":
		return match_data

	# Avoid double-advancing the StageManager round if already hydrated.
	if String(match_data.get("opponent_source", "")) == CAREER_LEAGUE_OPPONENT_SOURCE:
		return match_data
	if match_data.has("opponent_team_id") or match_data.has("opponent_avg_ca"):
		return match_data

	var stage_manager := _get_stage_manager()
	if stage_manager == null:
		return match_data
	if not stage_manager.has_method("get_next_opponent"):
		return match_data

	var opponent: Dictionary = stage_manager.get_next_opponent()
	if opponent.is_empty():
		return match_data

	var league_id := -1
	if stage_manager.has_method("get_active_league"):
		league_id = int(stage_manager.get_active_league())

	var opponent_name := String(opponent.get("club_name", match_data.get("opponent", "Opponent")))
	var opponent_team_id := int(opponent.get("team_id", -1))
	var opponent_avg_ca := float(opponent.get("avg_ca", match_data.get("overall_rating", 60)))
	var opponent_formation_id := String(opponent.get("formation", ""))
	var away_formation := _normalize_stage_formation_to_display(opponent_formation_id)

	match_data["opponent_source"] = CAREER_LEAGUE_OPPONENT_SOURCE
	match_data["league_id"] = league_id
	match_data["opponent"] = opponent_name
	match_data["opponent_team_id"] = opponent_team_id
	match_data["opponent_avg_ca"] = opponent_avg_ca
	match_data["opponent_formation_id"] = opponent_formation_id
	match_data["away_formation"] = away_formation

	var ca_range := _get_stage_league_ca_range(stage_manager, league_id)
	if ca_range.size() == 2 and ca_range[0] > 0 and ca_range[1] > 0:
		match_data["ca_min"] = ca_range[0]
		match_data["ca_max"] = ca_range[1]

	return match_data


func _get_stage_league_ca_range(stage_manager: Node, league_id: int) -> Array:
	"""Get CA range for a league from StageManager"""
	if stage_manager == null or not stage_manager.has_method("get_league_list"):
		return []
	var leagues: Array = stage_manager.get_league_list()
	for entry in leagues:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var d: Dictionary = entry
		if int(d.get("id", -1)) == league_id:
			return [int(d.get("min_ca", 0)), int(d.get("max_ca", 0))]
	return []


func _normalize_stage_formation_to_display(formation_id: String) -> String:
	"""Normalize engine formation ID to display format (e.g., T442 -> 4-4-2)"""
	if _tactics_manager and _tactics_manager.has_method("normalize_stage_formation_to_display"):
		return _tactics_manager.normalize_stage_formation_to_display(formation_id)
	# Fallback logic
	var raw := formation_id.strip_edges()
	if raw == "":
		return "4-4-2"
	if raw.find("-") != -1:
		return raw
	# Simple conversion: remove 'T' prefix and add dashes
	if raw.begins_with("T"):
		raw = raw.substr(1)
	if raw.length() == 3:
		return "%s-%s-%s" % [raw[0], raw[1], raw[2]]
	return raw


# ============================================================================
# NODE REFERENCES (SceneTree access)
# ============================================================================


func _get_game_manager() -> Node:
	var tree := Engine.get_main_loop()
	if tree and tree is SceneTree:
		return (tree as SceneTree).root.get_node_or_null("/root/GameManager")
	return null


func _get_cup_manager() -> Node:
	var tree := Engine.get_main_loop()
	if tree and tree is SceneTree:
		return (tree as SceneTree).root.get_node_or_null("/root/CupManager")
	return null


func _get_division_manager() -> Node:
	var tree := Engine.get_main_loop()
	if tree and tree is SceneTree:
		return (tree as SceneTree).root.get_node_or_null("/root/DivisionManager")
	return null


func _get_stage_manager() -> Node:
	var tree := Engine.get_main_loop()
	if tree and tree is SceneTree:
		return (tree as SceneTree).root.get_node_or_null("/root/StageManager")
	return null
