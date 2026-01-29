extends Node

## Phase 23: Division System Manager
## Performance-based U18 league progression with 3-tier divisions
## Author: Claude Sonnet 4.5
## Date: 2025-12-18

# =============================================================================
# SIGNALS
# =============================================================================

signal league_table_updated(table: Array)
signal player_promoted(old_div: int, new_div: int)
signal player_relegated(old_div: int, new_div: int)
signal player_stayed(division: int, position: int)
signal season_summary_ready(summary: Dictionary)

# =============================================================================
# ENUMS & CONSTANTS
# =============================================================================

enum Division { DIV_3 = 3, DIV_2 = 2, DIV_1 = 1 }  # Local League (Starting point)  # Regional League  # Top League

const DIVISION_CONFIG = {
	Division.DIV_3:
	{
		"name": "U18 Division 3 (Local)",
		"teams": 6,
		"opponent_ca_range": [55, 70],
		"team_names": ["Local HS A", "Local HS B", "Local HS C", "Local HS D", "Local HS E"]
	},
	Division.DIV_2:
	{
		"name": "U18 Division 2 (Regional)",
		"teams": 6,
		"opponent_ca_range": [70, 85],
		"team_names": ["Regional HS A", "Regional HS B", "Regional HS C", "Regional HS D", "Regional HS E"]
	},
	Division.DIV_1:
	{
		"name": "U18 Division 1 (Top)",
		"teams": 6,
		"opponent_ca_range": [80, 95],
		"team_names": ["Top HS A", "Top HS B", "Top HS C", "Top HS D", "Top HS E"]
	}
}

# League match weeks (6 total: 5 round-robin + 1 playoff)
const LEAGUE_WEEKS = [8, 12, 20, 28, 36, 44]

# Promotion/Relegation zones
const PROMOTION_ZONE = 1  # Top 1 team promotes
const RELEGATION_ZONE = 6  # Bottom 1 team relegates

# =============================================================================
# STATE VARIABLES
# =============================================================================

# Division state
var current_division: Division = Division.DIV_3
var current_season: int = 1  # 1-3 (3 years total)

# League table (6 teams: Player + 5 AI opponents)
var league_table: Array = []  # Array of team stats

# Player stats (tracked separately for clarity)
var player_stats: Dictionary = {
	"team_name": "My Team",
	"played": 0,
	"won": 0,
	"drawn": 0,
	"lost": 0,
	"goals_for": 0,
	"goals_against": 0,
	"goal_difference": 0,
	"points": 0,
	"position": 0  # 1-6
}

# AI opponent stats (5 teams, simulated)
var ai_teams: Array = []  # 5 AI teams with stats

# Match tracking
var matches_played: int = 0  # 0-6
var current_match_week_index: int = 0  # Index into LEAGUE_WEEKS

# =============================================================================
# INITIALIZATION
# =============================================================================


func _ready():
	print("[DivisionManager] Initializing Division System...")
	_initialize_season(Division.DIV_3, 1)


func _initialize_season(division: Division, season: int):
	"""Initialize a new season in the specified division"""
	print("[DivisionManager] Starting Season %d in Division %d" % [season, division])

	current_division = division
	current_season = season
	matches_played = 0
	current_match_week_index = 0

	_reset_player_stats()
	_generate_ai_opponents(division)
	_simulate_ai_matches()
	_sort_league_table()

	league_table_updated.emit(_build_league_table())

	print("[DivisionManager] Season initialized: %s, 6 teams" % DIVISION_CONFIG[division]["name"])


func _reset_player_stats():
	"""Reset player statistics for new season"""
	player_stats = {
		"team_name": "My Team",
		"played": 0,
		"won": 0,
		"drawn": 0,
		"lost": 0,
		"goals_for": 0,
		"goals_against": 0,
		"goal_difference": 0,
		"points": 0,
		"position": 0
	}


# =============================================================================
# AI OPPONENT GENERATION & SIMULATION
# =============================================================================


func _generate_ai_opponents(division: Division):
	"""Generate 5 AI teams for the current division"""
	ai_teams.clear()
	var config = DIVISION_CONFIG[division]
	var team_names = config["team_names"]

	for i in range(5):  # 5 AI teams (Player + 5 = 6 total)
		var team_ca = randf_range(config["opponent_ca_range"][0], config["opponent_ca_range"][1])
		ai_teams.append(
			{
				"team_name": team_names[i],
				"ca": team_ca,
				"played": 0,
				"won": 0,
				"drawn": 0,
				"lost": 0,
				"goals_for": 0,
				"goals_against": 0,
				"goal_difference": 0,
				"points": 0,
				"position": 0
			}
		)

	print(
		(
			"[DivisionManager] Generated 5 AI opponents (CA: %d-%d)"
			% [config["opponent_ca_range"][0], config["opponent_ca_range"][1]]
		)
	)


func _simulate_ai_matches():
	"""Simulate matches between AI teams (probabilistic)"""
	# Each AI team plays each other once (10 AI-vs-AI matches total)
	var matches_simulated = 0

	for i in range(ai_teams.size()):
		for j in range(i + 1, ai_teams.size()):
			_simulate_single_ai_match(ai_teams[i], ai_teams[j])
			matches_simulated += 1

	print("[DivisionManager] Simulated %d AI vs AI matches" % matches_simulated)


func _simulate_single_ai_match(team_a: Dictionary, team_b: Dictionary):
	"""Simulate one AI vs AI match (CA-based probability)"""
	var ca_diff = team_a["ca"] - team_b["ca"]
	var win_prob = 0.5 + (ca_diff / 100.0) * 0.3  # ±30% swing

	var roll = randf()
	var goals_a = 0
	var goals_b = 0

	if roll < win_prob:
		# Team A wins
		goals_a = randi_range(1, 3)
		goals_b = randi_range(0, max(0, goals_a - 1))
		team_a["won"] += 1
		team_a["points"] += 3
		team_b["lost"] += 1
	elif roll < win_prob + 0.25:
		# Draw
		goals_a = randi_range(0, 2)
		goals_b = goals_a
		team_a["drawn"] += 1
		team_b["drawn"] += 1
		team_a["points"] += 1
		team_b["points"] += 1
	else:
		# Team B wins
		goals_b = randi_range(1, 3)
		goals_a = randi_range(0, max(0, goals_b - 1))
		team_b["won"] += 1
		team_b["points"] += 3
		team_a["lost"] += 1

	# Update stats
	team_a["goals_for"] += goals_a
	team_a["goals_against"] += goals_b
	team_a["goal_difference"] = team_a["goals_for"] - team_a["goals_against"]
	team_b["goals_for"] += goals_b
	team_b["goals_against"] += goals_a
	team_b["goal_difference"] = team_b["goals_for"] - team_b["goals_against"]

	team_a["played"] += 1
	team_b["played"] += 1


# =============================================================================
# MATCH RESULT PROCESSING
# =============================================================================


func process_league_result(result: Dictionary):
	"""
	Update player stats from match result

	Args:
		result: {
			"win": bool,
			"draw": bool,
			"gf": int (goals for),
			"ga": int (goals against)
		}
	"""
	# Update match count
	player_stats["played"] += 1
	matches_played += 1
	current_match_week_index += 1

	# Update goals
	player_stats["goals_for"] += result["gf"]
	player_stats["goals_against"] += result["ga"]
	player_stats["goal_difference"] = player_stats["goals_for"] - player_stats["goals_against"]

	# Update points and W/D/L
	if result["win"]:
		player_stats["won"] += 1
		player_stats["points"] += 3
	elif result["draw"]:
		player_stats["drawn"] += 1
		player_stats["points"] += 1
	else:
		player_stats["lost"] += 1

	# Recalculate standings
	_sort_league_table()
	league_table_updated.emit(_build_league_table())

	print(
		(
			"[DivisionManager] Match %d/6 processed: %dW %dD %dL, Position %d/6, Points %d"
			% [
				matches_played,
				player_stats["won"],
				player_stats["drawn"],
				player_stats["lost"],
				player_stats["position"],
				player_stats["points"]
			]
		)
	)


# =============================================================================
# LEAGUE TABLE CALCULATION
# =============================================================================


func _sort_league_table():
	"""Sort league table by Points → GD → GF (FIFA rules)"""
	var combined_table = [player_stats] + ai_teams

	combined_table.sort_custom(
		func(a, b):
			# 1. Points (higher is better)
			if a["points"] != b["points"]:
				return a["points"] > b["points"]

			# 2. Goal Difference (higher is better)
			if a["goal_difference"] != b["goal_difference"]:
				return a["goal_difference"] > b["goal_difference"]

			# 3. Goals For (higher is better)
			return a["goals_for"] > b["goals_for"]
	)

	# Assign positions (1-6)
	for i in range(combined_table.size()):
		combined_table[i]["position"] = i + 1

	league_table = combined_table


func _build_league_table() -> Array:
	"""Return current league table (sorted)"""
	return league_table.duplicate(true)


func get_player_position() -> int:
	"""Get player's current position in league table (1-6)"""
	_sort_league_table()
	return player_stats["position"]


# =============================================================================
# PROMOTION/RELEGATION
# =============================================================================


func check_promotion_relegation() -> Dictionary:
	"""
	Check promotion/relegation based on final position

	Returns:
		{
			"outcome": "promoted" | "relegated" | "stayed",
			"old_division": int,
			"new_division": int,
			"final_position": int,
			"points": int
		}
	"""
	var final_position = get_player_position()  # 1-6
	var outcome = "stayed"
	var new_division = current_division

	# Promotion (1st place only, cannot promote from Div 1)
	if final_position == PROMOTION_ZONE and current_division > Division.DIV_1:
		new_division = int(current_division) - 1
		outcome = "promoted"

	# Relegation (6th place only, cannot relegate from Div 3)
	elif final_position == RELEGATION_ZONE and current_division < Division.DIV_3:
		new_division = int(current_division) + 1
		outcome = "relegated"

	print(
		(
			"[DivisionManager] Season End: Position %d/6, %s (Div %d → Div %d)"
			% [final_position, outcome.to_upper(), current_division, new_division]
		)
	)

	return {
		"outcome": outcome,
		"old_division": int(current_division),
		"new_division": int(new_division),
		"final_position": final_position,
		"points": player_stats["points"]
	}


# =============================================================================
# SEASON MANAGEMENT
# =============================================================================


func get_season_summary() -> Dictionary:
	"""Get season statistics for season end screen"""
	return {
		"year": current_season,
		"division": int(current_division),
		"division_name": DIVISION_CONFIG[current_division]["name"],
		"played": player_stats["played"],
		"won": player_stats["won"],
		"drawn": player_stats["drawn"],
		"lost": player_stats["lost"],
		"points": player_stats["points"],
		"gf": player_stats["goals_for"],
		"ga": player_stats["goals_against"],
		"gd": player_stats["goal_difference"],
		"position": player_stats["position"]
	}


func start_new_season(season: int):
	"""Start a new season in the new division (after promotion/relegation)"""
	var outcome = check_promotion_relegation()
	var new_div = outcome["new_division"]

	_initialize_season(new_div, season)

	print("[DivisionManager] New season %d started in Division %d" % [season, new_div])


# =============================================================================
# MATCH SCHEDULING
# =============================================================================


func is_league_match_week(week: int) -> bool:
	"""Check if the given week has a league match"""
	return week in LEAGUE_WEEKS


func get_opponent_for_week(week: int) -> Dictionary:
	"""
	Get opponent for the current league match week

	Args:
		week: Current week number

	Returns:
		{
			"name": String,
			"ca_range": [int, int]
		}
	"""
	if not is_league_match_week(week):
		return {}

	# Find which match number this is (0-5)
	var match_index = LEAGUE_WEEKS.find(week)
	if match_index < 0 or match_index >= ai_teams.size():
		push_error("[DivisionManager] Invalid match index: %d" % match_index)
		return {}

	# Return corresponding AI team
	var opponent = ai_teams[match_index]
	var config = DIVISION_CONFIG[current_division]

	return {"name": opponent["team_name"], "ca_range": config["opponent_ca_range"]}


# =============================================================================
# SAVE/LOAD
# =============================================================================


func save_to_dict() -> Dictionary:
	"""Save division state for persistence"""
	return {
		"current_division": int(current_division),
		"current_season": current_season,
		"matches_played": matches_played,
		"current_match_week_index": current_match_week_index,
		"player_stats": player_stats.duplicate(true),
		"ai_teams": ai_teams.duplicate(true),
		"league_table": league_table.duplicate(true)
	}


func load_from_dict(data: Dictionary):
	"""Load division state from save file"""
	current_division = data.get("current_division", 3)
	current_season = data.get("current_season", 1)
	matches_played = data.get("matches_played", 0)
	current_match_week_index = data.get("current_match_week_index", 0)
	player_stats = data.get("player_stats", {})
	ai_teams = data.get("ai_teams", [])
	league_table = data.get("league_table", [])

	print(
		(
			"[DivisionManager] Loaded: Division %d, Season %d, Position %d/6, Matches %d/6"
			% [current_division, current_season, player_stats.get("position", 0), matches_played]
		)
	)

	league_table_updated.emit(league_table)
