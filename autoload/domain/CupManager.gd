extends Node
## CupManager - Cup Tournament State Management
## Manages 4-stage knockout cup tournament (R16 → QF → SF → Final)
## Phase 22: Cup Tournament Implementation

signal cup_match_scheduled(stage: String, opponent: Dictionary)
signal cup_eliminated(stage: String, result: Dictionary)
signal cup_won(result: Dictionary)

enum CupStage { R16, QUARTER_FINAL, SEMI_FINAL, FINAL }  # Round of 16  # Quarter-Final  # Semi-Final  # Final

# ============================================================================
# CUP STATE (per season)
# ============================================================================

var current_season: int = 1
var current_stage: CupStage = CupStage.R16
var is_cup_active: bool = true
var cup_results: Array = []  # Match results history

# ============================================================================
# CONSTANTS
# ============================================================================

# Cup schedule (week numbers for each stage)
const CUP_SCHEDULE = {
	1: {CupStage.R16: 16, CupStage.QUARTER_FINAL: 24, CupStage.SEMI_FINAL: 32, CupStage.FINAL: 40},  # Year 1: Regional Cup
	2: {CupStage.R16: 16, CupStage.QUARTER_FINAL: 24, CupStage.SEMI_FINAL: 32, CupStage.FINAL: 40},  # Year 2: National Qualifier
	3: {CupStage.R16: 16, CupStage.QUARTER_FINAL: 24, CupStage.SEMI_FINAL: 32, CupStage.FINAL: 40}  # Year 3: National Championship
}

# Stage display names
const STAGE_NAMES = {
	CupStage.R16: "Cup Round of 16",
	CupStage.QUARTER_FINAL: "Cup Quarter-Final",
	CupStage.SEMI_FINAL: "Cup Semi-Final",
	CupStage.FINAL: "Cup Final"
}

# Cup type names per year
const CUP_TYPE_NAMES = {1: "Regional Cup", 2: "National Qualifier", 3: "National Championship"}

# ============================================================================
# INITIALIZATION
# ============================================================================


func _ready():
	print("[CupManager] Cup tournament system initialized")
	reset_cup(1)


# ============================================================================
# CUP STATE MANAGEMENT
# ============================================================================


func reset_cup(season: int):
	"""Reset cup state for new season"""
	current_season = season
	current_stage = CupStage.R16
	is_cup_active = true
	cup_results.clear()

	var cup_name = CUP_TYPE_NAMES.get(season, "Unknown Cup")
	print("[CupManager] 🏆 Cup reset for season %d (%s)" % [season, cup_name])


func get_next_cup_match_week() -> int:
	"""Get week number for next cup match (or -1 if eliminated/finished)"""
	if not is_cup_active:
		return -1

	var schedule = CUP_SCHEDULE.get(current_season, {})
	return schedule.get(current_stage, -1)


func is_cup_match_week(week: int) -> bool:
	"""Check if this week has a cup match"""
	if not is_cup_active:
		return false

	return week == get_next_cup_match_week()


# ============================================================================
# OPPONENT GENERATION
# ============================================================================


func get_cup_opponent(stage: CupStage) -> Dictionary:
	"""Generate opponent for current cup stage"""
	var base_ca = 100

	# Get player CA if available
	if has_node("/root/PlayerData"):
		var player_data = get_node("/root/PlayerData")
		if player_data.has_method("get_ca"):
			base_ca = player_data.get_ca()
		elif "current_ca" in player_data:
			base_ca = player_data.current_ca

	# Cup opponents get stronger each round
	var ca_boost_by_stage = {CupStage.R16: 0, CupStage.QUARTER_FINAL: 10, CupStage.SEMI_FINAL: 20, CupStage.FINAL: 30}

	var opponent_ca = base_ca + ca_boost_by_stage.get(stage, 0)

	return {
		"name": _generate_cup_opponent_name(stage),
		"ca_min": opponent_ca - 5,
		"ca_max": opponent_ca + 5,
		"importance": 7 + int(stage)  # R16=7, QF=8, SF=9, Final=10
	}


func _generate_cup_opponent_name(stage: CupStage) -> String:
	"""Generate opponent name based on stage and season"""
	var stage_prefixes = {
		CupStage.R16: ["District", "Regional", "Local", "Academy"],
		CupStage.QUARTER_FINAL: ["Elite", "Top", "Premier", "Rising"],
		CupStage.SEMI_FINAL: ["National", "Championship", "Elite", "Select"],
		CupStage.FINAL: ["Champion", "Defending", "Elite", "Premier"]
	}

	var prefixes = stage_prefixes.get(stage, ["Unknown"])
	var suffix_options = ["HS", "Academy", "Team", "United", "FC"]

	var prefix = prefixes[randi() % prefixes.size()]
	var suffix = suffix_options[randi() % suffix_options.size()]

	return "%s %s" % [prefix, suffix]


# ============================================================================
# RESULT PROCESSING
# ============================================================================


func process_cup_result(result: Dictionary):
	"""Process cup match result and update cup state"""
	cup_results.append(result)

	var score_home = result.get("score_home", 0)
	var score_away = result.get("score_away", 0)
	var is_win = score_home > score_away

	print("[CupManager] Cup result: %d-%d (Win: %s)" % [score_home, score_away, is_win])

	if is_win:
		# Advance to next stage
		if current_stage == CupStage.FINAL:
			# Won the cup!
			is_cup_active = false
			cup_won.emit(result)
			print("[CupManager] 🏆 CUP WON! Final result: %d-%d" % [score_home, score_away])
		else:
			var next_stage: int = int(current_stage) + 1
			if next_stage > int(CupStage.FINAL):
				next_stage = int(CupStage.FINAL)
			current_stage = next_stage
			var next_week = get_next_cup_match_week()
			var opponent = get_cup_opponent(current_stage)
			cup_match_scheduled.emit(STAGE_NAMES[current_stage], opponent)
			print("[CupManager] ✅ Advanced to %s (Week %d)" % [STAGE_NAMES[current_stage], next_week])
	else:
		# Eliminated
		is_cup_active = false
		cup_eliminated.emit(STAGE_NAMES[current_stage], result)
		print("[CupManager] ❌ ELIMINATED at %s. Result: %d-%d" % [STAGE_NAMES[current_stage], score_home, score_away])


# ============================================================================
# SUMMARY & STATS
# ============================================================================


func get_cup_summary() -> Dictionary:
	"""Get cup run summary for season end"""
	if cup_results.is_empty():
		return {"reached_stage": "None", "matches": 0, "won_cup": false}

	var last_result = cup_results[-1]
	var won_final = cup_results.size() == 4 and last_result.get("score_home", 0) > last_result.get("score_away", 0)

	# Determine reached stage
	var reached_stage_name = "None"
	if not is_cup_active:
		# Eliminated or won
		if won_final:
			reached_stage_name = "Champion"
		else:
			var eliminated_at_stage = cup_results.size() - 1  # 0-indexed
			if eliminated_at_stage >= 0 and eliminated_at_stage < 4:
				reached_stage_name = STAGE_NAMES.get(eliminated_at_stage, "Unknown")
	else:
		# Still active (shouldn't happen at season end)
		reached_stage_name = STAGE_NAMES.get(current_stage, "Unknown")

	return {
		"reached_stage": reached_stage_name,
		"matches": cup_results.size(),
		"results": cup_results,
		"won_cup": won_final,
		"cup_type": CUP_TYPE_NAMES.get(current_season, "Cup")
	}


func get_current_stage_name() -> String:
	"""Get current cup stage name"""
	return STAGE_NAMES.get(current_stage, "Unknown")


func get_cup_type_name() -> String:
	"""Get cup type name for current season"""
	return CUP_TYPE_NAMES.get(current_season, "Cup")


# ============================================================================
# SAVE/LOAD SUPPORT
# ============================================================================


func get_save_data() -> Dictionary:
	"""Get cup state for saving"""
	return {"season": current_season, "stage": int(current_stage), "is_active": is_cup_active, "results": cup_results}


func load_save_data(data: Dictionary):
	"""Load cup state from save"""
	current_season = data.get("season", 1)
	var stage_value: int = int(data.get("stage", int(CupStage.R16)))
	if stage_value < int(CupStage.R16) or stage_value > int(CupStage.FINAL):
		stage_value = int(CupStage.R16)
	current_stage = stage_value
	is_cup_active = data.get("is_active", true)
	cup_results = data.get("results", [])

	print(
		(
			"[CupManager] Loaded cup state: Season %d, Stage %s, Active: %s"
			% [current_season, STAGE_NAMES[current_stage], is_cup_active]
		)
	)
