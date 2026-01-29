extends Node

## ============================================================================
## CareerStatisticsManager - PROTAGONIST STORY DATA (Phase 24)
## ============================================================================
##
## PURPOSE: Track protagonist's career journey for story-based endings
##
## SCOPE: Single player (protagonist) only
##
## DATA TRACKED:
## - CA timeline (weekly snapshots over 3 years / 156 weeks)
## - Match performance summary (goals, assists, ratings, W/D/L)
## - Division history (season-by-season progression)
## - Decision patterns (from DecisionTracker)
## - Key moments (significant career events)
##
## CONSUMERS:
## - EndingGenerator: Creates formula-based career endings
## - GraduationManager: Determines graduation outcome
## - GraduationScreen: Displays career summary
## - SaveManager: Persists protagonist's story data
##
## RELATED BUT DISTINCT:
## - CareerStatsManager: Tracks ALL players' detailed match statistics
##   (shots, passes, tackles, cards, etc.) for leaderboards/analytics.
##   CareerStatsManager is the SSOT for league-wide player statistics.
##   CareerStatisticsManager is the SSOT for protagonist's story arc.
##
## NOTE: Some data overlap exists (goals, assists, matches) but serves
## different purposes: this manager focuses on NARRATIVE, while
## CareerStatsManager focuses on DETAILED STATISTICS.
## ============================================================================

## CA timeline: Weekly snapshots of player CA over career
## [{week: 1, ca: 50, year: 1}, {week: 2, ca: 51, year: 1}, ...]
var ca_timeline: Array = []

## Match statistics (cumulative over entire career)
var total_goals: int = 0
var total_assists: int = 0
var total_matches: int = 0
var total_ratings: float = 0.0
var total_wins: int = 0
var total_draws: int = 0
var total_losses: int = 0

## Division history (season-by-season progression)
## [{year: 1, division: 3, position: 2, promoted: false, relegated: false}, ...]
var division_history: Array = []

## Signal emitted when CA is tracked
signal ca_tracked(week: int, ca: int)

## Signal emitted when match stats are recorded
signal match_stats_recorded(goals: int, assists: int, rating: float)

## Signal emitted when season division is recorded
signal season_division_recorded(year: int, division: int, position: int)

# ============================================================================
# PHASE 24 IMPROVEMENTS: MATCHMANAGER INTEGRATION (P3-2)
# ============================================================================


func _ready():
	"""Initialize and connect to MatchManager signals"""
	print("[CareerStatisticsManager] Initializing...")

	# Connect to MatchManager for automatic stats tracking
	if has_node("/root/MatchManager"):
		var match_manager = get_node("/root/MatchManager")
		if match_manager.has_signal("match_ended"):
			match_manager.match_ended.connect(_on_match_ended)
			print("[CareerStatisticsManager] Connected to MatchManager.match_ended signal")
		else:
			push_warning("[CareerStatisticsManager] MatchManager exists but has no 'match_ended' signal")
	else:
		push_warning("[CareerStatisticsManager] MatchManager not found - match stats won't auto-track")

	print("[CareerStatisticsManager] Initialized")


func _on_match_ended(match_result: Dictionary):
	"""
	Signal handler for MatchManager.match_ended
	Automatically extracts player stats and records them

	Args:
		match_result: Dictionary containing match data with player_stats
	"""
	if not match_result.has("player_stats"):
		if OS.is_debug_build():
			push_warning("[CareerStatisticsManager] match_result missing player_stats, skipping")
		return

	var player_stats = match_result.player_stats

	# Extract stats with safe fallbacks
	var goals = player_stats.get("goals", 0)
	var assists = player_stats.get("assists", 0)
	var rating = player_stats.get("rating", 0.0)

	# Extract result if available
	var result = ""
	if match_result.has("result"):
		result = match_result.result
	elif match_result.has("home_score") and match_result.has("away_score"):
		var is_home = match_result.get("is_player_home", true)
		var player_score = match_result.home_score if is_home else match_result.away_score
		var opponent_score = match_result.away_score if is_home else match_result.home_score

		if player_score > opponent_score:
			result = "win"
		elif player_score < opponent_score:
			result = "loss"
		else:
			result = "draw"

	# Record stats (with validation)
	record_match_stats(goals, assists, rating, result)

	if OS.is_debug_build():
		print("[CareerStatisticsManager] Auto-recorded match stats from MatchManager")


## Track weekly CA progression
##
## Called by DateManager._on_week_advanced() to snapshot CA each week
##
## @param week: Current week (1-156)
## @param ca: Player's current CA value
func track_weekly_ca(week: int, ca: int) -> void:
	var year = _calculate_year_from_week(week)

	var entry = {"week": week, "ca": ca, "year": year}

	ca_timeline.append(entry)
	ca_tracked.emit(week, ca)

	if OS.is_debug_build():
		print("[CareerStatisticsManager] CA tracked: Week %d, CA %d (Year %d)" % [week, ca, year])


## Record match statistics
##
## Called by MatchManager after match simulation
##
## @param goals: Goals scored by player in this match
## @param assists: Assists by player in this match
## @param rating: Player's match rating (0.0-10.0)
## @param result: Match result ("win", "draw", "loss")
func record_match_stats(goals: int, assists: int, rating: float, result: String = "") -> void:
	total_goals += goals
	total_assists += assists
	total_matches += 1
	total_ratings += rating

	# Record result
	match result:
		"win":
			total_wins += 1
		"draw":
			total_draws += 1
		"loss":
			total_losses += 1

	match_stats_recorded.emit(goals, assists, rating)

	if OS.is_debug_build():
		print(
			(
				"[CareerStatisticsManager] Match stats recorded: %d goals, %d assists, %.1f rating (%s)"
				% [goals, assists, rating, result]
			)
		)
		print(
			(
				"  → Career totals: %d goals, %d assists, %d matches, avg rating %.1f"
				% [total_goals, total_assists, total_matches, get_average_rating()]
			)
		)


## Record season division and position
##
## Called by DivisionManager at end of each season
##
## @param year: Season year (1, 2, 3)
## @param division: Final division (1, 2, 3)
## @param position: Final position in division (1-6)
## @param promoted: Whether player was promoted
## @param relegated: Whether player was relegated
func record_season_division(
	year: int, division: int, position: int, promoted: bool = false, relegated: bool = false
) -> void:
	var entry = {"year": year, "division": division, "position": position, "promoted": promoted, "relegated": relegated}

	division_history.append(entry)
	season_division_recorded.emit(year, division, position)

	if OS.is_debug_build():
		var status = ""
		if promoted:
			status = " 🔥 PROMOTED"
		elif relegated:
			status = " 😞 RELEGATED"
		print(
			(
				"[CareerStatisticsManager] Season %d recorded: Division %d, Position %d%s"
				% [year, division, position, status]
			)
		)


## Analyze player role based on goals/assists ratio
##
## Returns: "Striker", "Playmaker", "Balanced", "Defender", "GK", "Developing"
func analyze_role() -> String:
	if total_goals + total_assists == 0:
		return "Developing"

	# Check if player is GK or defender (based on PlayerData position)
	if PlayerData:
		var position = PlayerData.position if "position" in PlayerData else ""
		if position == "GK":
			return "GK"
		if position in ["CB", "LB", "RB", "LWB", "RWB"]:
			return "Defender"

	# Classify based on goals/assists ratio
	var goal_ratio = float(total_goals) / (total_goals + total_assists)

	if goal_ratio > 0.7:
		return "Striker"  # Goals >> Assists
	elif goal_ratio > 0.4:
		return "Balanced"  # Mix of goals and assists
	else:
		return "Playmaker"  # Assists >> Goals


## Get average match rating
##
## @return: Average rating (0.0-10.0), or 0.0 if no matches
func get_average_rating() -> float:
	if total_matches == 0:
		return 0.0
	return float(total_ratings) / total_matches


## Get win rate percentage
##
## @return: Win rate (0-100), or 0.0 if no matches
func get_win_rate() -> float:
	if total_matches == 0:
		return 0.0
	return (float(total_wins) / total_matches) * 100.0


## Get comprehensive career summary (for EndingGenerator)
##
## Returns: Dictionary with:
## - league_outcome: Division progression and final standing
## - player_role: Role classification and statistics
## - decision_patterns: Analysis from DecisionTracker
## - key_moments: Top 8 significant events
## - ca_timeline: CA progression over career
## - division_history: Season-by-season division progression
func get_career_summary() -> Dictionary:
	return {
		"league_outcome": _get_league_outcome(),
		"player_role": _get_player_role_summary(),
		"decision_patterns": _analyze_decision_patterns(),
		"key_moments": _identify_key_moments(),
		"ca_timeline": ca_timeline.duplicate(true),
		"division_history": division_history.duplicate(true),
		"match_statistics": _get_match_statistics()
	}


## Clear all statistics (for testing/reset)
func clear_statistics() -> void:
	ca_timeline.clear()
	total_goals = 0
	total_assists = 0
	total_matches = 0
	total_ratings = 0.0
	total_wins = 0
	total_draws = 0
	total_losses = 0
	division_history.clear()

	if OS.is_debug_build():
		print("[CareerStatisticsManager] Statistics cleared")


## Save statistics to dictionary (for SaveManager)
##
## @return: Dictionary with all career statistics
func save_to_dict() -> Dictionary:
	return {
		"ca_timeline": ca_timeline.duplicate(true),
		"total_goals": total_goals,
		"total_assists": total_assists,
		"total_matches": total_matches,
		"total_ratings": total_ratings,
		"total_wins": total_wins,
		"total_draws": total_draws,
		"total_losses": total_losses,
		"division_history": division_history.duplicate(true),
		"version": 1
	}


## Load statistics from dictionary (for SaveManager)
##
## @param data: Dictionary from save file
func load_from_dict(data: Dictionary) -> void:
	ca_timeline = data.get("ca_timeline", []).duplicate(true)
	total_goals = data.get("total_goals", 0)
	total_assists = data.get("total_assists", 0)
	total_matches = data.get("total_matches", 0)
	total_ratings = data.get("total_ratings", 0.0)
	total_wins = data.get("total_wins", 0)
	total_draws = data.get("total_draws", 0)
	total_losses = data.get("total_losses", 0)
	division_history = data.get("division_history", []).duplicate(true)

	if OS.is_debug_build():
		print(
			(
				"[CareerStatisticsManager] Loaded statistics: %d matches, %d goals, %d assists, %d CA snapshots"
				% [total_matches, total_goals, total_assists, ca_timeline.size()]
			)
		)


# ============================================================================
# PRIVATE METHODS
# ============================================================================


## Calculate year from week number (52 weeks per year)
func _calculate_year_from_week(week: int) -> int:
	return ceili(float(week) / 52.0)


## Get league outcome summary (for ending formula)
func _get_league_outcome() -> Dictionary:
	var final_division = 3
	var final_position = 6

	# Get final division from DivisionManager (Phase 23 integration)
	if DivisionManager:
		final_division = DivisionManager.current_division
		if DivisionManager.player_stats.has("position"):
			final_position = DivisionManager.player_stats.position

	# Fallback: Get from division_history if available
	if division_history.size() > 0:
		var last_season = division_history[division_history.size() - 1]
		final_division = last_season.division
		final_position = last_season.position

	return {
		"final_division": final_division, "final_position": final_position, "seasons": division_history.duplicate(true)
	}


## Get player role summary
func _get_player_role_summary() -> Dictionary:
	var position = "Unknown"
	if PlayerData and "position" in PlayerData:
		position = PlayerData.position

	return {"position": position, "goals": total_goals, "assists": total_assists, "role_type": analyze_role()}


## Analyze decision patterns from DecisionTracker
func _analyze_decision_patterns() -> Dictionary:
	if not DecisionTracker:
		return _get_default_decision_patterns()

	# Get pattern analysis from DecisionTracker
	var patterns = DecisionTracker.analyze_decision_patterns()

	return patterns


## Identify key moments from decision log and career events
func _identify_key_moments(top_n: int = 8) -> Array:
	var moments = []

	# Get key moments from DecisionTracker
	if DecisionTracker:
		var decision_moments = DecisionTracker.get_key_moments(top_n)
		for moment in decision_moments:
			moments.append(
				{
					"week": moment.week,
					"description": moment.narrative,
					"type": "decision",
					"significance": moment.get("significance", 0.5)
				}
			)

	# Add division milestones (promotions/relegations)
	for season in division_history:
		if season.get("promoted", false):
			moments.append(
				{
					"week": season.year * 52,
					"description": "Year %d 종료: Division %d 승격! 🔥" % [season.year, season.division - 1],
					"type": "achievement",
					"significance": 0.9
				}
			)
		elif season.get("relegated", false):
			moments.append(
				{
					"week": season.year * 52,
					"description": "Year %d 종료: Division %d 강등... 😞" % [season.year, season.division + 1],
					"type": "setback",
					"significance": 0.7
				}
			)
		elif season.division == 1 and season.position == 1:
			moments.append(
				{
					"week": season.year * 52,
					"description": "Year %d 종료: Division 1 우승! 🏆" % season.year,
					"type": "achievement",
					"significance": 1.0
				}
			)

	# Add goal/assist milestones
	if total_goals >= 30:
		moments.append(
			{"week": 156, "description": "총 %d골 기록 (득점왕 수준)!" % total_goals, "type": "achievement", "significance": 0.8}  # End of career
		)
	elif total_goals >= 20:
		moments.append(
			{"week": 156, "description": "총 %d골 기록" % total_goals, "type": "achievement", "significance": 0.6}
		)

	if total_assists >= 20:
		moments.append(
			{
				"week": 156,
				"description": "총 %d도움 기록 (플레이메이커 수준)!" % total_assists,
				"type": "achievement",
				"significance": 0.8
			}
		)

	# Sort by significance (highest first)
	moments.sort_custom(func(a, b): return a.significance > b.significance)

	# Return top N
	return moments.slice(0, min(top_n, moments.size()))


## Get match statistics summary
func _get_match_statistics() -> Dictionary:
	return {
		"total_matches": total_matches,
		"total_goals": total_goals,
		"total_assists": total_assists,
		"average_rating": get_average_rating(),
		"total_wins": total_wins,
		"total_draws": total_draws,
		"total_losses": total_losses,
		"win_rate": get_win_rate()
	}


## Get default decision patterns (when DecisionTracker not available)
func _get_default_decision_patterns() -> Dictionary:
	return {
		"training_tendency": "Balanced",
		"intensity_ratio": 0.5,
		"risk_taking": "Balanced",
		"risk_ratio": 0.0,
		"focus_areas": [],
		"balance_score": 0.5,
		"total_decisions": 0,
		"total_training": 0
	}
