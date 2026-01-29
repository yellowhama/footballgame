extends Node
## ============================================================================
## CareerStatsManager - ALL PLAYERS DETAILED STATISTICS (P0.4)
## ============================================================================
##
## PURPOSE: Track detailed match statistics for ALL players in the league
##
## SCOPE: All players (protagonist + AI teammates + opponents)
##
## DATA TRACKED (per player):
## - Appearances, minutes played
## - Goals, assists (with per-match averages)
## - Shots, shots on target
## - Passes, passes completed
## - Tackles, fouls
## - Yellow/red cards
## - Clean sheets (goalkeepers)
## - Best performances, match history (last 100 matches)
##
## CONSUMERS:
## - MatchSimulationManager: Updates stats after each match
## - LeaderboardScreen: Displays top scorers, assisters, etc.
## - PlayerProfileScreen: Shows individual player statistics
##
## STORAGE:
## - File: user://career_stats.json
## - Schema version: 1
##
## RELATED BUT DISTINCT:
## - CareerStatisticsManager: Tracks PROTAGONIST's story arc only
##   (CA timeline, decision patterns, key moments) for endings.
##   CareerStatisticsManager is the SSOT for protagonist's narrative.
##   CareerStatsManager is the SSOT for league-wide detailed statistics.
##
## NOTE: Both managers track goals/assists/matches for the protagonist,
## but serve different purposes:
## - This manager: DETAILED STATS for analytics/leaderboards
## - CareerStatisticsManager: NARRATIVE DATA for story/endings
## ============================================================================

## ============================================================================
## SIGNALS
## ============================================================================

signal career_stats_updated(player_uid: String, stats: Dictionary)
signal milestone_achieved(player_uid: String, milestone_type: String, value: int)

## ============================================================================
## CONSTANTS
## ============================================================================

const CAREER_STATS_FILE = "user://career_stats.json"
const SCHEMA_VERSION = 1

## Milestone thresholds
const MILESTONES = {
	"goals": [1, 5, 10, 25, 50, 100],
	"assists": [1, 5, 10, 25, 50, 100],
	"appearances": [1, 10, 25, 50, 100, 200],
	"clean_sheets": [1, 5, 10, 25, 50],  # For goalkeepers
}

## ============================================================================
## DATA STRUCTURES
## ============================================================================


## Per-player career statistics
class CareerStats:
	extends RefCounted
	var uid: String = ""
	var player_name: String = ""
	var position: String = ""

	# Aggregate stats (lifetime totals)
	var total_goals: int = 0
	var total_assists: int = 0
	var total_appearances: int = 0
	var total_minutes: int = 0
	var total_shots: int = 0
	var total_shots_on_target: int = 0
	var total_passes: int = 0
	var total_passes_completed: int = 0
	var total_tackles: int = 0
	var total_fouls: int = 0
	var total_yellow_cards: int = 0
	var total_red_cards: int = 0
	var total_clean_sheets: int = 0  # For goalkeepers

	# Averages
	var average_rating: float = 0.0
	var goals_per_match: float = 0.0
	var assists_per_match: float = 0.0

	# Best performances
	var best_rating: float = 0.0
	var best_match_id: String = ""
	var best_match_date: String = ""
	var most_goals_in_match: int = 0
	var most_assists_in_match: int = 0

	# Match history (limited to last 100 matches for performance)
	var matches_history: Array[Dictionary] = []  # Array of MatchPerformance

	func _init(p_uid: String = "", p_name: String = "", p_position: String = ""):
		uid = p_uid
		player_name = p_name
		position = p_position

	func to_dict() -> Dictionary:
		return {
			"uid": uid,
			"player_name": player_name,
			"position": position,
			"total_goals": total_goals,
			"total_assists": total_assists,
			"total_appearances": total_appearances,
			"total_minutes": total_minutes,
			"total_shots": total_shots,
			"total_shots_on_target": total_shots_on_target,
			"total_passes": total_passes,
			"total_passes_completed": total_passes_completed,
			"total_tackles": total_tackles,
			"total_fouls": total_fouls,
			"total_yellow_cards": total_yellow_cards,
			"total_red_cards": total_red_cards,
			"total_clean_sheets": total_clean_sheets,
			"average_rating": average_rating,
			"goals_per_match": goals_per_match,
			"assists_per_match": assists_per_match,
			"best_rating": best_rating,
			"best_match_id": best_match_id,
			"best_match_date": best_match_date,
			"most_goals_in_match": most_goals_in_match,
			"most_assists_in_match": most_assists_in_match,
			"matches_history": matches_history
		}

	static func from_dict(data: Dictionary) -> CareerStats:
		var stats = CareerStats.new()
		stats.uid = data.get("uid", "")
		stats.player_name = data.get("player_name", "")
		stats.position = data.get("position", "")
		stats.total_goals = data.get("total_goals", 0)
		stats.total_assists = data.get("total_assists", 0)
		stats.total_appearances = data.get("total_appearances", 0)
		stats.total_minutes = data.get("total_minutes", 0)
		stats.total_shots = data.get("total_shots", 0)
		stats.total_shots_on_target = data.get("total_shots_on_target", 0)
		stats.total_passes = data.get("total_passes", 0)
		stats.total_passes_completed = data.get("total_passes_completed", 0)
		stats.total_tackles = data.get("total_tackles", 0)
		stats.total_fouls = data.get("total_fouls", 0)
		stats.total_yellow_cards = data.get("total_yellow_cards", 0)
		stats.total_red_cards = data.get("total_red_cards", 0)
		stats.total_clean_sheets = data.get("total_clean_sheets", 0)
		stats.average_rating = data.get("average_rating", 0.0)
		stats.goals_per_match = data.get("goals_per_match", 0.0)
		stats.assists_per_match = data.get("assists_per_match", 0.0)
		stats.best_rating = data.get("best_rating", 0.0)
		stats.best_match_id = data.get("best_match_id", "")
		stats.best_match_date = data.get("best_match_date", "")
		stats.most_goals_in_match = data.get("most_goals_in_match", 0)
		stats.most_assists_in_match = data.get("most_assists_in_match", 0)

		# Load match history (limited to 100 matches)
		var history = data.get("matches_history", [])
		for match in history:
			stats.matches_history.append(match)

		return stats


## Single match performance snapshot
class MatchPerformance:
	var match_id: String = ""
	var match_date: String = ""
	var opponent: String = ""
	var home_away: String = ""  # "home" or "away"
	var goals: int = 0
	var assists: int = 0
	var rating: float = 0.0
	var minutes_played: int = 90
	var result: String = ""  # "win", "draw", "loss"

	func to_dict() -> Dictionary:
		return {
			"match_id": match_id,
			"match_date": match_date,
			"opponent": opponent,
			"home_away": home_away,
			"goals": goals,
			"assists": assists,
			"rating": rating,
			"minutes_played": minutes_played,
			"result": result
		}


## ============================================================================
## STATE
## ============================================================================

var _career_stats: Dictionary = {}  # {uid: CareerStats}
var _loaded: bool = false

## ============================================================================
## LIFECYCLE
## ============================================================================


func _ready():
	load_from_database()
	print("[CareerStatsManager] Initialized with %d player records" % _career_stats.size())


## ============================================================================
## PUBLIC API
## ============================================================================


## Update career stats for a player after a match
##
## PARAMETERS:
##   player_uid: String - Player UID
##   match_stats: PlayerMatchStats - Match statistics (from PlayerStatsProcessor)
##   match_info: Dictionary (optional) - Additional match info:
##     - match_id: String
##     - opponent: String
##     - home_away: String
##     - result: String ("win", "draw", "loss")
##     - goals_conceded: int (for goalkeeper clean sheets)
func update_career_stats(player_uid: String, match_stats, match_info: Dictionary = {}) -> void:
	if not _loaded:
		load_from_database()

	# Get or create career stats
	var stats: CareerStats = _career_stats.get(player_uid)
	if stats == null:
		stats = CareerStats.new(
			player_uid,
			match_stats.player_name if match_stats.has("player_name") else "",
			match_stats.position if match_stats.has("position") else ""
		)
		_career_stats[player_uid] = stats

	# Update aggregate stats
	var previous_goals = stats.total_goals
	var previous_assists = stats.total_assists
	var previous_appearances = stats.total_appearances

	stats.total_goals += match_stats.goals if match_stats.has("goals") else 0
	stats.total_assists += match_stats.assists if match_stats.has("assists") else 0
	stats.total_appearances += 1
	stats.total_minutes += match_stats.minutes_played if match_stats.has("minutes_played") else 90
	stats.total_shots += match_stats.shots if match_stats.has("shots") else 0
	stats.total_shots_on_target += match_stats.shots_on_target if match_stats.has("shots_on_target") else 0
	stats.total_passes += match_stats.passes if match_stats.has("passes") else 0
	stats.total_passes_completed += match_stats.passes_completed if match_stats.has("passes_completed") else 0
	stats.total_tackles += match_stats.tackles if match_stats.has("tackles") else 0
	stats.total_fouls += match_stats.fouls if match_stats.has("fouls") else 0
	stats.total_yellow_cards += match_stats.yellow_cards if match_stats.has("yellow_cards") else 0
	stats.total_red_cards += match_stats.red_cards if match_stats.has("red_cards") else 0

	# Goalkeeper clean sheet
	if match_stats.position == "GK" and match_info.has("goals_conceded"):
		if match_info.goals_conceded == 0:
			stats.total_clean_sheets += 1

	# Update averages
	if stats.total_appearances > 0:
		var total_rating_sum = stats.average_rating * (stats.total_appearances - 1)
		total_rating_sum += match_stats.rating if match_stats.has("rating") else 6.0
		stats.average_rating = total_rating_sum / stats.total_appearances

		stats.goals_per_match = float(stats.total_goals) / stats.total_appearances
		stats.assists_per_match = float(stats.total_assists) / stats.total_appearances

	# Update best performances
	var match_rating = match_stats.rating if match_stats.has("rating") else 0.0
	if match_rating > stats.best_rating:
		stats.best_rating = match_rating
		stats.best_match_id = match_info.get("match_id", "")
		stats.best_match_date = Time.get_datetime_string_from_system()

	var match_goals = match_stats.goals if match_stats.has("goals") else 0
	if match_goals > stats.most_goals_in_match:
		stats.most_goals_in_match = match_goals

	var match_assists = match_stats.assists if match_stats.has("assists") else 0
	if match_assists > stats.most_assists_in_match:
		stats.most_assists_in_match = match_assists

	# Add to match history (keep last 100 matches)
	var performance = MatchPerformance.new()
	performance.match_id = match_info.get("match_id", "")
	performance.match_date = Time.get_datetime_string_from_system()
	performance.opponent = match_info.get("opponent", "")
	performance.home_away = match_info.get("home_away", "")
	performance.goals = match_goals
	performance.assists = match_assists
	performance.rating = match_rating
	performance.minutes_played = match_stats.minutes_played if match_stats.has("minutes_played") else 90
	performance.result = match_info.get("result", "")

	stats.matches_history.append(performance.to_dict())

	# Keep only last 100 matches
	if stats.matches_history.size() > 100:
		stats.matches_history = stats.matches_history.slice(-100)

	# Check for milestones
	_check_milestones(player_uid, stats, previous_goals, previous_assists, previous_appearances)

	# Emit signal
	career_stats_updated.emit(player_uid, stats.to_dict())


## Get career stats for a player
func get_career_stats(player_uid: String) -> Dictionary:
	if not _loaded:
		load_from_database()

	var stats: CareerStats = _career_stats.get(player_uid)
	if stats == null:
		return {}

	return stats.to_dict()


## Get career stats for all players
func get_all_career_stats() -> Dictionary:
	if not _loaded:
		load_from_database()

	var result: Dictionary = {}
	for uid in _career_stats:
		result[uid] = _career_stats[uid].to_dict()
	return result


## Get top scorers (sorted by total_goals)
func get_top_scorers(limit: int = 10) -> Array:
	if not _loaded:
		load_from_database()

	var all_stats: Array = []
	for uid in _career_stats:
		all_stats.append(_career_stats[uid])

	all_stats.sort_custom(func(a, b): return a.total_goals > b.total_goals)

	var result: Array = []
	for i in range(mini(limit, all_stats.size())):
		result.append(all_stats[i].to_dict())
	return result


## Get top assisters (sorted by total_assists)
func get_top_assisters(limit: int = 10) -> Array:
	if not _loaded:
		load_from_database()

	var all_stats: Array = []
	for uid in _career_stats:
		all_stats.append(_career_stats[uid])

	all_stats.sort_custom(func(a, b): return a.total_assists > b.total_assists)

	var result: Array = []
	for i in range(mini(limit, all_stats.size())):
		result.append(all_stats[i].to_dict())
	return result


## Save career stats to database
func save_to_database() -> bool:
	var data = {"version": SCHEMA_VERSION, "last_updated": Time.get_datetime_string_from_system(), "players": {}}

	for uid in _career_stats:
		data.players[uid] = _career_stats[uid].to_dict()

	var file = FileAccess.open(CAREER_STATS_FILE, FileAccess.WRITE)
	if file == null:
		push_error("[CareerStatsManager] Failed to open file for writing: %s" % CAREER_STATS_FILE)
		return false

	file.store_string(JSON.stringify(data, "\t"))
	file.close()

	print("[CareerStatsManager] Saved career stats for %d players" % _career_stats.size())
	return true


## Load career stats from database
func load_from_database() -> bool:
	if _loaded:
		return true

	if not FileAccess.file_exists(CAREER_STATS_FILE):
		print("[CareerStatsManager] No career stats file found, starting fresh")
		_loaded = true
		return true

	var file = FileAccess.open(CAREER_STATS_FILE, FileAccess.READ)
	if file == null:
		push_error("[CareerStatsManager] Failed to open file for reading: %s" % CAREER_STATS_FILE)
		_loaded = true
		return false

	var json_string = file.get_as_text()
	file.close()

	var json = JSON.new()
	var error = json.parse(json_string)
	if error != OK:
		push_error("[CareerStatsManager] JSON parse error: %s" % json.get_error_message())
		_loaded = true
		return false

	var data = json.data
	if not data is Dictionary:
		push_error("[CareerStatsManager] Invalid data format")
		_loaded = true
		return false

	# Load player stats
	var players = data.get("players", {})
	for uid in players:
		_career_stats[uid] = CareerStats.from_dict(players[uid])

	print("[CareerStatsManager] Loaded career stats for %d players" % _career_stats.size())
	_loaded = true
	return true


## Clear all career stats (for testing)
func clear_all_stats() -> void:
	_career_stats.clear()
	save_to_database()
	print("[CareerStatsManager] All career stats cleared")


## ============================================================================
## PRIVATE HELPERS
## ============================================================================


## Check for milestone achievements
func _check_milestones(
	player_uid: String, stats: CareerStats, previous_goals: int, previous_assists: int, previous_appearances: int
) -> void:
	# Goals milestone
	for threshold in MILESTONES.goals:
		if previous_goals < threshold and stats.total_goals >= threshold:
			milestone_achieved.emit(player_uid, "goals", threshold)
			print("[CareerStatsManager] Milestone: %s reached %d career goals!" % [stats.player_name, threshold])

	# Assists milestone
	for threshold in MILESTONES.assists:
		if previous_assists < threshold and stats.total_assists >= threshold:
			milestone_achieved.emit(player_uid, "assists", threshold)
			print("[CareerStatsManager] Milestone: %s reached %d career assists!" % [stats.player_name, threshold])

	# Appearances milestone
	for threshold in MILESTONES.appearances:
		if previous_appearances < threshold and stats.total_appearances >= threshold:
			milestone_achieved.emit(player_uid, "appearances", threshold)
			print("[CareerStatsManager] Milestone: %s reached %d appearances!" % [stats.player_name, threshold])

	# Clean sheets milestone (goalkeepers)
	if stats.position == "GK":
		for threshold in MILESTONES.clean_sheets:
			if stats.total_clean_sheets == threshold:
				milestone_achieved.emit(player_uid, "clean_sheets", threshold)
				print("[CareerStatsManager] Milestone: %s reached %d clean sheets!" % [stats.player_name, threshold])
