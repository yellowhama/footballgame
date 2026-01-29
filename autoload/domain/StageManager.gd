extends Node

## Stage System Manager
##
## Manages the 350-stage progression system where players face increasingly difficult teams.
## Loads team data from stage_teams_safe.json (pseudonymized names) and tracks player progress.

# Signals
signal stage_started(stage_id: int, team_data: Dictionary)
signal stage_completed(stage_id: int, result: Dictionary)
signal stage_failed(stage_id: int, result: Dictionary)
signal progress_updated(unlocked_stage: int)

# Stage data
var stage_teams: Array = []  # 350 teams from JSON
var team_lookup: Dictionary = {}  # team_id -> team data
var total_stages: int = 0

# Progress tracking
var current_stage: int = 1
var unlocked_stage: int = 1
var stage_history: Dictionary = {}  # stage_id -> {result, rewards, timestamp}

# League configuration (12부제 리그 시스템)
var league_config: Array = []  # Loaded from JSON
var league_map: Dictionary = {}  # league_id -> {config, teams:Array, current_round:int}
var active_league_id: int = 12  # 12부 리그에서 시작
var player_team_id: int = -1  # 플레이어 소속 팀 ID
var current_player_team: Dictionary = {}  # Cached player team data

# Manager data (감독 데이터)
var managers_data: Array = []  # Loaded from dummy_managers.json

# BM Two-Track: 명예의 전당 스냅샷 연동
var selected_team_snapshot: Dictionary = {}  # Hall of Fame에서 선택된 팀 스냅샷
signal team_snapshot_changed(snapshot: Dictionary)

# File paths
const STAGE_DATA_PATH = "res://data/stage_teams_enhanced.json"  # Enhanced with tactics & manager data
const STAGE_DATA_FALLBACK = "res://data/stage_teams_safe.json"  # Fallback if enhanced not found
const LEAGUE_CONFIG_PATH = "res://data/league_config.json"  # 12부제 리그 설정
const SAVE_KEY = "stage_progress"


# Safe accessor for FootballMatchSimulator (GDExtension may not be loaded)
func _get_match_simulator() -> Object:
	if ClassDB.class_exists("FootballMatchSimulator"):
		return ClassDB.instantiate("FootballMatchSimulator")
	push_warning("[StageManager] FootballMatchSimulator not available - GDExtension not loaded")
	return null


# Ready
func _ready():
	print("[StageManager] Initializing...")
	load_stage_teams()
	load_league_config()
	load_progress()
	_find_player_team()
	print("[StageManager] Ready! Total stages: %d, Unlocked: %d" % [total_stages, unlocked_stage])
	print("[StageManager] Active League: %d부, Player Team ID: %d" % [active_league_id, player_team_id])


func load_league_config() -> void:
	"""Load league configuration from JSON file"""
	print("[StageManager] Loading league config from: %s" % LEAGUE_CONFIG_PATH)

	if not FileAccess.file_exists(LEAGUE_CONFIG_PATH):
		push_error("[StageManager] League config file not found: %s" % LEAGUE_CONFIG_PATH)
		return

	var file = FileAccess.open(LEAGUE_CONFIG_PATH, FileAccess.READ)
	if not file:
		push_error("[StageManager] Failed to open league config file")
		return

	var json_text = file.get_as_text()
	file.close()

	var json = JSON.new()
	var parse_result = json.parse(json_text)

	if parse_result != OK:
		push_error("[StageManager] Failed to parse league config: %s" % json.get_error_message())
		return

	var data = json.get_data()
	league_config = data.get("leagues", [])

	# Build league_map from config
	league_map.clear()
	for league in league_config:
		var league_id = int(league.get("league_id", 0))
		league_map[league_id] = {
			"config": league,
			"teams": league.get("teams", []),
			"current_round": 0,
		}

	print("[StageManager] Loaded %d leagues" % league_config.size())
	for league in league_config:
		var lid = league.get("league_id", 0)
		var teams = league.get("teams", [])
		print(
			(
				"[StageManager]   %d부: %d teams (CA %d-%d)"
				% [lid, teams.size(), league.get("ca_range", [0, 0])[0], league.get("ca_range", [0, 0])[1]]
			)
		)


func _find_player_team() -> void:
	"""Find the player's team in stage_teams"""
	current_player_team = {}
	for team in stage_teams:
		if team.get("is_player_team", false):
			player_team_id = int(team.get("team_id", -1))
			current_player_team = team.duplicate(true)
			# Find which league the player is in
			for league_id in league_map:
				var league_data = league_map[league_id]
				for league_team in league_data["teams"]:
					if int(league_team.get("team_id", -1)) == player_team_id:
						active_league_id = league_id
						print(
							(
								"[StageManager] Found player team: %s (ID %d) in %d부"
								% [team.get("club_name", "Unknown"), player_team_id, active_league_id]
							)
						)
						return
			break

	if player_team_id == -1:
		push_warning("[StageManager] Player team not found, defaulting to league 12")
		current_player_team = {}


# ============================================================================
# Data Loading
# ============================================================================


func load_stage_teams() -> void:
	"""Load stage teams from JSON file (uses enhanced if available)"""
	var data_path := STAGE_DATA_PATH

	# Try enhanced file first, fallback to original if not found
	if not FileAccess.file_exists(STAGE_DATA_PATH):
		print("[StageManager] Enhanced data not found, using fallback: %s" % STAGE_DATA_FALLBACK)
		data_path = STAGE_DATA_FALLBACK

	print("[StageManager] Loading stage teams from: %s" % data_path)

	if not FileAccess.file_exists(data_path):
		push_error("[StageManager] Stage data file not found: %s" % data_path)
		return

	var file = FileAccess.open(data_path, FileAccess.READ)
	if not file:
		push_error("[StageManager] Failed to open stage data file")
		return

	var json_text = file.get_as_text()
	file.close()

	var json = JSON.new()
	var parse_result = json.parse(json_text)

	if parse_result != OK:
		push_error("[StageManager] Failed to parse JSON: %s" % json.get_error_message())
		return

	stage_teams = json.get_data()
	total_stages = stage_teams.size()
	_build_team_lookup()

	print("[StageManager] Loaded %d stage teams successfully" % total_stages)
	print("[StageManager]   - Easiest: %s (CA %.1f)" % [stage_teams[0]["club_name"], stage_teams[0]["avg_ca"]])
	print("[StageManager]   - Hardest: %s (CA %.1f)" % [stage_teams[-1]["club_name"], stage_teams[-1]["avg_ca"]])


func _build_team_lookup() -> void:
	team_lookup.clear()
	for team in stage_teams:
		var team_id = int(team.get("team_id", -1))
		if team_id > 0:
			team_lookup[team_id] = team


func _get_team_data_by_id(team_id: int) -> Dictionary:
	if team_id <= 0:
		return {}
	if team_lookup.has(team_id):
		return (team_lookup[team_id] as Dictionary).duplicate(true)
	return {}


func _resolve_league_team(team_entry: Dictionary) -> Dictionary:
	if team_entry.is_empty():
		return {}
	var team_id = int(team_entry.get("team_id", -1))
	var resolved = _get_team_data_by_id(team_id)
	if resolved.is_empty():
		print("[StageManager] ⚠️ Team %d not found in stage_teams, using league config entry only" % team_id)
		return team_entry.duplicate(true)
	var squad_size = resolved.get("squad", []).size()
	print(
		(
			"[StageManager] ✅ Resolved team %d: %s (squad: %d players)"
			% [team_id, resolved.get("club_name", "Unknown"), squad_size]
		)
	)
	return resolved


func _ensure_player_team_cached() -> void:
	if not current_player_team.is_empty():
		return
	if player_team_id == -1:
		_find_player_team()
		return
	var cached = _get_team_data_by_id(player_team_id)
	if cached.is_empty():
		_find_player_team()
	else:
		current_player_team = cached


# ============================================================================
# League Management
# ============================================================================


func get_current_league_teams() -> Array:
	"""Get teams in the current active league"""
	if not league_map.has(active_league_id):
		return []
	return league_map[active_league_id].get("teams", [])


func get_league_opponents() -> Array:
	"""Get opponent teams (exclude player's team)"""
	var teams = get_current_league_teams()
	return teams.filter(func(t): return int(t.get("team_id", -1)) != player_team_id)


func promote_league() -> bool:
	"""Promote player to higher league (lower number)"""
	if active_league_id <= 1:
		print("[StageManager] Already at top league!")
		return false

	active_league_id -= 1
	print("[StageManager] PROMOTED to %d부 리그!" % active_league_id)
	save_progress()
	return true


func relegate_league() -> bool:
	"""Relegate player to lower league (higher number)"""
	if active_league_id >= 12:
		print("[StageManager] Already at bottom league!")
		return false

	active_league_id += 1
	print("[StageManager] Relegated to %d부 리그" % active_league_id)
	save_progress()
	return true


func get_next_opponent() -> Dictionary:
	"""Get next opponent from current league"""
	var opponents = get_league_opponents()
	if opponents.is_empty():
		return {}

	var league_data = league_map.get(active_league_id, {})
	var current_round = league_data.get("current_round", 0)

	if current_round >= opponents.size():
		# All opponents faced, reset round
		current_round = 0

	var opponent_summary = opponents[current_round]
	league_data["current_round"] = current_round + 1
	league_map[active_league_id] = league_data
	var opponent = _resolve_league_team(opponent_summary)
	var log_team = opponent if not opponent.is_empty() else opponent_summary

	print(
		(
			"[StageManager] Next opponent: %s (CA %.1f) - Round %d/%d"
			% [
				log_team.get("club_name", "Unknown"),
				float(log_team.get("avg_ca", 0.0)),
				current_round + 1,
				opponents.size()
			]
		)
	)

	return opponent


# ============================================================================
# Stage Management
# ============================================================================


func start_stage(stage_id: int) -> bool:
	"""
	Start a stage match
	Returns true if stage can be started, false otherwise
	"""
	# Validation
	if stage_id < 1 or stage_id > total_stages:
		push_error("[StageManager] Invalid stage ID: %d" % stage_id)
		return false

	if stage_id > unlocked_stage:
		print("[StageManager] Stage %d is locked! (Unlocked: %d)" % [stage_id, unlocked_stage])
		return false

	# Get team data
	var team_data = get_stage_team(stage_id)
	if team_data.is_empty():
		push_error("[StageManager] Failed to get team data for stage %d" % stage_id)
		return false

	# Update current stage
	current_stage = stage_id

	print("[StageManager] Starting Stage %d: %s (CA %.1f)" % [stage_id, team_data["club_name"], team_data["avg_ca"]])

	# Emit signal
	stage_started.emit(stage_id, team_data)

	return true


func complete_stage(stage_id: int, result: Dictionary) -> void:
	"""
	Handle stage completion
	result should contain: {winner: "home"/"away", score_home: int, score_away: int}
	"""
	if stage_id < 1 or stage_id > total_stages:
		push_error("[StageManager] Invalid stage ID: %d" % stage_id)
		return

	var winner_value: String = str(result.get("winner", ""))
	if winner_value == "":
		var outcome: String = str(result.get("result", "draw"))
		match outcome:
			"win":
				winner_value = "home"
			"loss":
				winner_value = "away"
	var victory = winner_value == "home"

	if victory:
		print(
			(
				"[StageManager] Stage %d COMPLETED! Result: %d-%d"
				% [stage_id, result.get("score_home", 0), result.get("score_away", 0)]
			)
		)

		# Unlock next stage
		if stage_id == unlocked_stage and unlocked_stage < total_stages:
			unlocked_stage += 1
			print("[StageManager] Stage %d UNLOCKED!" % unlocked_stage)
			progress_updated.emit(unlocked_stage)

		# Award rewards
		var rewards = calculate_rewards(stage_id)
		award_rewards(rewards)

		# Record history
		stage_history[stage_id] = {"result": result, "rewards": rewards, "timestamp": Time.get_unix_time_from_system()}

		# Save progress
		save_progress()

		# Emit signal
		stage_completed.emit(stage_id, result)

	else:
		print(
			(
				"[StageManager] Stage %d FAILED. Result: %d-%d"
				% [stage_id, result.get("score_home", 0), result.get("score_away", 0)]
			)
		)

		# Emit signal
		stage_failed.emit(stage_id, result)


func calculate_rewards(stage_id: int) -> Dictionary:
	"""
	Calculate rewards based on stage difficulty
	Higher stages give more rewards
	"""
	var base_gold = 50
	var base_xp = 100

	# Scale rewards based on stage
	var stage_multiplier = 1.0 + (stage_id / 100.0)  # Increases every 100 stages

	var gold = int(base_gold * stage_multiplier)
	var xp = int(base_xp * stage_multiplier)

	# Bonus for completing milestone stages
	if stage_id % 50 == 0:
		gold *= 2
		xp *= 2
		print("[StageManager] MILESTONE BONUS! Stage %d" % stage_id)

	return {"gold": gold, "xp": xp, "stage_id": stage_id}


func award_rewards(rewards: Dictionary) -> void:
	"""Award rewards to player (integrate with your reward system)"""
	print("[StageManager] Rewards awarded: +%d gold, +%d XP" % [rewards.get("gold", 0), rewards.get("xp", 0)])

	# TODO: Integrate with your game's reward system
	# Example:
	# PlayerData.add_gold(rewards["gold"])
	# PlayerData.add_xp(rewards["xp"])


# ============================================================================
# Data Access
# ============================================================================


func get_stage_team(stage_id: int) -> Dictionary:
	"""Get team data for a specific stage (1-indexed)"""
	if stage_id < 1 or stage_id > total_stages:
		push_error("[StageManager] Invalid stage ID: %d" % stage_id)
		return {}

	# Find team with matching stage_id
	for team in stage_teams:
		if team.get("stage_id", -1) == stage_id:
			return team

	push_error("[StageManager] Team not found for stage ID: %d" % stage_id)
	return {}


func get_stage_info(stage_id: int) -> Dictionary:
	"""Get stage information (without full squad data)"""
	var team = get_stage_team(stage_id)
	if team.is_empty():
		return {}

	return {
		"stage_id": stage_id,
		"club_name": team.get("club_name", "Unknown"),
		"division": team.get("division", "Unknown"),
		"avg_ca": team.get("avg_ca", 0),
		"tier": team.get("tier", "first_team"),
		"league_id": team.get("league_id", ""),
		"formation": team.get("formation", "T442"),
		"tactical_style": team.get("tactical_style", "Balanced"),
		"is_unlocked": stage_id <= unlocked_stage,
		"is_completed": stage_history.has(stage_id)
	}


func get_team_tactics(team_data: Dictionary) -> Dictionary:
	"""Get tactical parameters for a team (from enhanced data or defaults)"""
	if team_data.has("tactics"):
		return team_data.get("tactics", {})

	# Default tactics based on tactical style
	var style := team_data.get("tactical_style", "Balanced") as String
	match style:
		"Attacking":
			return {
				"attacking_intensity": 0.8,
				"defensive_line_height": 0.7,
				"width": 0.75,
				"pressing_trigger": 0.7,
				"tempo": 0.8,
				"directness": 0.6
			}
		"Defensive":
			return {
				"attacking_intensity": 0.3,
				"defensive_line_height": 0.3,
				"width": 0.5,
				"pressing_trigger": 0.3,
				"tempo": 0.4,
				"directness": 0.4
			}
		"Possession":
			return {
				"attacking_intensity": 0.5,
				"defensive_line_height": 0.6,
				"width": 0.7,
				"pressing_trigger": 0.6,
				"tempo": 0.3,
				"directness": 0.3
			}
		"Counter":
			return {
				"attacking_intensity": 0.7,
				"defensive_line_height": 0.35,
				"width": 0.5,
				"pressing_trigger": 0.4,
				"tempo": 0.9,
				"directness": 0.85
			}
		"Pressing":
			return {
				"attacking_intensity": 0.7,
				"defensive_line_height": 0.75,
				"width": 0.65,
				"pressing_trigger": 0.9,
				"tempo": 0.7,
				"directness": 0.6
			}
		_:
			return {
				"attacking_intensity": 0.5,
				"defensive_line_height": 0.5,
				"width": 0.6,
				"pressing_trigger": 0.5,
				"tempo": 0.5,
				"directness": 0.5
			}


func get_team_formation(team_data: Dictionary) -> String:
	"""Get formation for a team"""
	return team_data.get("formation", "T442") as String


func get_all_stage_info() -> Array:
	"""Get information for all stages (for UI display)"""
	var info_list = []
	for i in range(1, total_stages + 1):
		info_list.append(get_stage_info(i))
	return info_list


func is_stage_unlocked(stage_id: int) -> bool:
	"""Check if stage is unlocked"""
	return stage_id <= unlocked_stage and stage_id >= 1 and stage_id <= total_stages


func is_stage_completed(stage_id: int) -> bool:
	"""Check if stage has been completed"""
	return stage_history.has(stage_id)


func get_league_list() -> Array:
	var list: Array = []
	for config in league_config:
		var league_id: int = int(config.get("league_id", 0))
		var league: Dictionary = league_map.get(league_id, {})
		var ca_range: Array = config.get("ca_range", [0, 0])
		(
			list
			. append(
				{
					"id": league_id,
					"name": config.get("name", league_id),
					"min_ca": ca_range[0] if ca_range.size() >= 1 else 0,
					"max_ca": ca_range[1] if ca_range.size() >= 2 else 0,
					"team_count": (league.get("teams", []) as Array).size(),
					"current_round": int(league.get("current_round", 0)),
					"is_active": league_id == active_league_id,
				}
			)
		)
	return list


func get_league_snapshot(league_id: int) -> Dictionary:
	if not league_map.has(league_id):
		return {}
	var entry: Dictionary = league_map[league_id]
	var config: Dictionary = entry.get("config", {})
	return {
		"id": league_id,
		"name": config.get("name", league_id),
		"teams": entry.get("teams", []),
		"current_round": entry.get("current_round", 0),
	}


func get_active_league() -> int:
	return active_league_id


func set_active_league(league_id: int) -> void:
	if league_id <= 0 or not league_map.has(league_id):
		return
	active_league_id = league_id


func set_active_league_by_ca(avg_ca: float) -> void:
	var league_id: int = _determine_league(avg_ca)
	set_active_league(league_id)


func get_next_league_team(league_id: int = -1) -> Dictionary:
	var target_id := league_id if league_id > 0 else active_league_id
	if target_id <= 0 or not league_map.has(target_id):
		return {}
	var entry: Dictionary = league_map[target_id]
	var teams: Array = entry.get("teams", [])
	if teams.is_empty():
		return {}
	var round_num: int = int(entry.get("current_round", 0))
	var team: Dictionary = (teams[round_num % teams.size()] as Dictionary).duplicate(true)
	entry["current_round"] = (round_num + 1) % teams.size()
	league_map[target_id] = entry
	var resolved := _resolve_league_team(team)
	if not resolved.is_empty():
		resolved["league_id"] = target_id
	return resolved


func _determine_league(avg_ca: float) -> int:
	for config in league_config:
		var ca_range: Array = config.get("ca_range", [])
		if ca_range.size() >= 2:
			var min_ca = float(ca_range[0])
			var max_ca = float(ca_range[1])
			if avg_ca >= min_ca and avg_ca < max_ca:
				return int(config.get("league_id", active_league_id))
	if league_config.is_empty():
		return active_league_id
	return int(league_config[league_config.size() - 1].get("league_id", active_league_id))


func get_completion_percentage() -> float:
	"""Get overall completion percentage"""
	if total_stages == 0:
		return 0.0
	return (float(stage_history.size()) / float(total_stages)) * 100.0


# ============================================================================
# Save/Load Progress
# ============================================================================


func save_progress() -> void:
	"""Save stage progress to SaveManager"""
	var _save_data = {"unlocked_stage": unlocked_stage, "current_stage": current_stage, "stage_history": stage_history}

	# TODO: Integrate with your save system
	# Example:
	# SaveManager.set_data(SAVE_KEY, _save_data)
	# SaveManager.save_game()

	print("[StageManager] Progress saved (unlocked: %d, completed: %d)" % [unlocked_stage, stage_history.size()])


func load_progress() -> void:
	"""Load stage progress from SaveManager"""
	# TODO: Integrate with your save system
	# Example:
	# var save_data = SaveManager.get_data(SAVE_KEY, {})

	var save_data = {}  # Placeholder

	if save_data.is_empty():
		print("[StageManager] No saved progress found, starting fresh")
		unlocked_stage = 1
		current_stage = 1
		stage_history = {}
		return

	unlocked_stage = save_data.get("unlocked_stage", 1)
	current_stage = save_data.get("current_stage", 1)
	stage_history = save_data.get("stage_history", {})

	print("[StageManager] Progress loaded (unlocked: %d, completed: %d)" % [unlocked_stage, stage_history.size()])


func reset_progress() -> void:
	"""Reset all progress (for testing or new game)"""
	unlocked_stage = 1
	current_stage = 1
	stage_history = {}
	save_progress()
	print("[StageManager] Progress reset")


# ============================================================================
# Utility
# ============================================================================


func get_stats() -> Dictionary:
	"""Get overall statistics"""
	return {
		"total_stages": total_stages,
		"unlocked_stage": unlocked_stage,
		"current_stage": current_stage,
		"completed_stages": stage_history.size(),
		"completion_percentage": get_completion_percentage()
	}


func print_stats() -> void:
	"""Print statistics to console"""
	var stats = get_stats()
	print("\n[StageManager] ======================")
	print("[StageManager] STAGE SYSTEM STATS")
	print("[StageManager] ======================")
	print("[StageManager] Total stages: %d" % stats["total_stages"])
	print("[StageManager] Unlocked: %d" % stats["unlocked_stage"])
	print("[StageManager] Completed: %d" % stats["completed_stages"])
	print("[StageManager] Completion: %.1f%%" % stats["completion_percentage"])
	print("[StageManager] ======================\n")


# ============================================================================
# Squad Selection
# ============================================================================


func select_squad_for_team(team_data: Dictionary, formation: String = "") -> Dictionary:
	"""
	Select optimal squad for a team using the Rust SquadSelector

	Args:
		team_data: Team dictionary from stage_teams (must include 'squad' array)
		formation: Optional formation string (e.g., "T442"). Uses team default if empty.

	Returns:
		Dictionary with:
		- success: bool
		- starters: Array of selected players with positions and scores
		- substitutes: Array of bench players
		- formation: The formation used
	"""
	var simulator = _get_match_simulator()
	if simulator == null:
		return {"success": false, "error": "Simulator not available"}

	# Convert team_data to JSON string
	var team_json = JSON.stringify(team_data)

	# Call Rust squad selector
	var result_json = simulator.select_squad(team_json, formation)
	var result = JSON.parse_string(result_json)

	if result == null:
		push_error("[StageManager] Failed to parse squad selection result")
		return {"success": false, "error": "JSON parse failed"}

	if result.get("success", false):
		print(
			(
				"[StageManager] Squad selected for %s (%s)"
				% [result.get("team_name", "Unknown"), result.get("formation", "T442")]
			)
		)
		print(
			(
				"[StageManager]   Starters: %d, Subs: %d"
				% [result.get("starters_count", 0), result.get("substitutes_count", 0)]
			)
		)
	else:
		push_error("[StageManager] Squad selection failed: %s" % result.get("error", "Unknown"))

	return result


func get_player_team_squad(formation: String = "") -> Dictionary:
	"""
	Get optimal squad for the player's team

	Returns squad selection result or empty dict if player team not found
	"""
	_ensure_player_team_cached()
	if current_player_team.is_empty():
		push_error("[StageManager] Player team not set")
		return {"success": false, "error": "Player team not found"}
	return select_squad_for_team(current_player_team, formation)


func get_opponent_squad(opponent_data: Dictionary, formation: String = "") -> Dictionary:
	"""
	Get optimal squad for an opponent team

	Args:
		opponent_data: Opponent team dictionary
		formation: Optional formation override

	Returns squad selection result
	"""
	return select_squad_for_team(opponent_data, formation)


func get_formation_positions(formation: String) -> Array:
	"""
	Get positions required for a formation

	Args:
		formation: Formation string (e.g., "T442", "T433")

	Returns array of position strings
	"""
	var simulator = _get_match_simulator()
	if simulator == null:
		return []
	var result_json = simulator.get_formation_positions(formation)
	var result = JSON.parse_string(result_json)

	if result and result.get("success", false):
		return result.get("positions", [])

	return []


func prepare_match_squads(formation: String = "") -> Dictionary:
	"""
	Prepare both player and opponent squads for a match

	Returns:
		Dictionary with:
		- player_squad: Squad selection for player's team
		- opponent_squad: Squad selection for next opponent
		- opponent_team: The opponent team data
	"""
	var opponent = get_next_opponent()
	if opponent.is_empty():
		return {"success": false, "error": "No opponent available"}

	var player_squad = get_player_team_squad(formation)
	var opponent_squad = get_opponent_squad(opponent, formation)

	return {
		"success": player_squad.get("success", false) and opponent_squad.get("success", false),
		"player_squad": player_squad,
		"opponent_squad": opponent_squad,
		"opponent_team": opponent
	}


# Phase 2: 상황 인식 전술 선택
func select_contextual_tactics_for_team(
	team_data: Dictionary, morale: float = -1.0, recent_results: Array = []
) -> Dictionary:
	"""
	Select tactics for a team based on context (morale, recent results)

	Args:
		team_data: Team dictionary from stage_teams
		morale: Team morale (0.0 ~ 1.0), -1 for default
		recent_results: Array of recent match results ["W", "L", "D"]

	Returns:
		Dictionary with tactics information
	"""
	var simulator = _get_match_simulator()
	if simulator == null:
		return {"success": false, "error": "Simulator not available"}
	var team_json = JSON.stringify(team_data)
	var results_json = JSON.stringify(recent_results)

	var result_json = simulator.select_contextual_tactics(team_json, morale, results_json)
	var result = JSON.parse_string(result_json)

	if result == null:
		return {"success": false, "error": "Failed to parse tactics result"}

	return result


func get_player_team_tactics(morale: float = -1.0, recent_results: Array = []) -> Dictionary:
	"""
	Get contextual tactics for player's team

	Args:
		morale: Team morale (0.0 ~ 1.0), -1 for default
		recent_results: Array of recent match results ["W", "L", "D"]

	Returns:
		Dictionary with tactics information
	"""
	_ensure_player_team_cached()
	if current_player_team.is_empty():
		return {"success": false, "error": "No player team set"}

	return select_contextual_tactics_for_team(current_player_team, morale, recent_results)


func get_opponent_tactics(opponent_data: Dictionary, morale: float = -1.0, recent_results: Array = []) -> Dictionary:
	"""
	Get contextual tactics for opponent team

	Args:
		opponent_data: Opponent team dictionary
		morale: Team morale (0.0 ~ 1.0), -1 for default
		recent_results: Array of recent match results ["W", "L", "D"]

	Returns:
		Dictionary with tactics information
	"""
	return select_contextual_tactics_for_team(opponent_data, morale, recent_results)


func prepare_match_tactics(
	player_morale: float = -1.0, player_results: Array = [], opponent_morale: float = -1.0, opponent_results: Array = []
) -> Dictionary:
	"""
	Prepare contextual tactics for both teams before a match

	Args:
		player_morale: Player team morale
		player_results: Player team recent results
		opponent_morale: Opponent team morale
		opponent_results: Opponent team recent results

	Returns:
		Dictionary with both teams' tactics
	"""
	var opponent = get_next_opponent()
	if opponent.is_empty():
		return {"success": false, "error": "No opponent available"}

	var player_tactics = get_player_team_tactics(player_morale, player_results)
	var opponent_tactics = get_opponent_tactics(opponent, opponent_morale, opponent_results)

	return {
		"success": player_tactics.get("success", false) and opponent_tactics.get("success", false),
		"player_tactics": player_tactics,
		"opponent_tactics": opponent_tactics,
		"opponent_team": opponent
	}


# Phase 3: 전술 분석 및 추천
func analyze_team(team_data: Dictionary) -> Dictionary:
	"""
	Analyze team setup and get recommendations

	Args:
		team_data: Team dictionary from stage_teams

	Returns:
		Dictionary with tactical analysis including warnings and recommendations
	"""
	var simulator = _get_match_simulator()
	if simulator == null:
		return {"success": false, "error": "Simulator not available"}
	var team_json = JSON.stringify(team_data)

	var result_json = simulator.analyze_team_setup(team_json)
	var result = JSON.parse_string(result_json)

	if result == null:
		return {"success": false, "error": "Failed to parse analysis result"}

	return result


func analyze_player_team() -> Dictionary:
	"""
	Analyze player's team setup and get recommendations

	Returns:
		Dictionary with tactical analysis
	"""
	_ensure_player_team_cached()
	if current_player_team.is_empty():
		return {"success": false, "error": "No player team set"}

	return analyze_team(current_player_team)


func analyze_opponent(opponent_data: Dictionary) -> Dictionary:
	"""
	Analyze opponent team setup

	Args:
		opponent_data: Opponent team dictionary

	Returns:
		Dictionary with tactical analysis
	"""
	return analyze_team(opponent_data)


func get_prematch_analysis() -> Dictionary:
	"""
	Get complete prematch analysis for both teams

	Returns:
		Dictionary with analysis for player team and opponent
	"""
	var opponent = get_next_opponent()
	if opponent.is_empty():
		return {"success": false, "error": "No opponent available"}

	var player_analysis = analyze_player_team()
	var opponent_analysis = analyze_opponent(opponent)

	return {
		"success": player_analysis.get("success", false) and opponent_analysis.get("success", false),
		"player_analysis": player_analysis,
		"opponent_analysis": opponent_analysis,
		"opponent_team": opponent
	}


# Phase 5: 카운터 전술 분석
func get_counter_tactic(team_data: Dictionary, opponent_formation: String) -> Dictionary:
	"""
	Analyze counter tactic against opponent formation

	Args:
		team_data: Team dictionary from stage_teams
		opponent_formation: Opponent's formation string

	Returns:
		Dictionary with counter tactic information
	"""
	var simulator = _get_match_simulator()
	if simulator == null:
		return {"success": false, "error": "Simulator not available"}
	var team_json = JSON.stringify(team_data)

	var result_json = simulator.analyze_counter_tactic(team_json, opponent_formation)
	var result = JSON.parse_string(result_json)

	if result == null:
		return {"success": false, "error": "Failed to parse counter tactic result"}

	return result


func get_player_team_counter_tactic(opponent_formation: String) -> Dictionary:
	"""
	Get counter tactic for player's team against opponent formation

	Args:
		opponent_formation: Opponent's formation string

	Returns:
		Dictionary with counter tactic information
	"""
	_ensure_player_team_cached()
	if current_player_team.is_empty():
		return {"success": false, "error": "No player team set"}

	return get_counter_tactic(current_player_team, opponent_formation)


func get_recommended_counter_tactic() -> Dictionary:
	"""
	Get recommended counter tactic against next opponent

	Returns:
		Dictionary with counter tactic info based on opponent's formation
	"""
	var opponent = get_next_opponent()
	if opponent.is_empty():
		return {"success": false, "error": "No opponent available"}

	var opponent_formation = opponent.get("formation", "T442")
	return get_player_team_counter_tactic(opponent_formation)


# Phase 4: 감독 영향 기반 전술
func get_tactics_with_coach(team_data: Dictionary, manager_data: Dictionary) -> Dictionary:
	"""
	Get tactics with coach influence applied

	Args:
		team_data: Team dictionary from stage_teams
		manager_data: Manager dictionary from dummy_managers

	Returns:
		Dictionary with tactics and coach adjustments
	"""
	var simulator = _get_match_simulator()
	if simulator == null:
		return {"success": false, "error": "Simulator not available"}
	var team_json = JSON.stringify(team_data)
	var manager_json = JSON.stringify(manager_data)

	var result_json = simulator.select_tactics_with_coach(team_json, manager_json)
	var result = JSON.parse_string(result_json)

	if result == null:
		return {"success": false, "error": "Failed to parse coach tactics result"}

	return result


func get_player_team_tactics_with_coach() -> Dictionary:
	"""
	Get tactics for player's team with their manager's influence

	Returns:
		Dictionary with tactics and coach adjustments
	"""
	_ensure_player_team_cached()
	if current_player_team.is_empty():
		return {"success": false, "error": "No player team set"}

	# Find manager for this team
	var manager_id = current_player_team.get("manager_id", 6)  # Default manager
	var manager = _find_manager_by_id(manager_id)

	if manager.is_empty():
		return {"success": false, "error": "Manager not found"}

	return get_tactics_with_coach(current_player_team, manager)


func get_player_manager_data() -> Dictionary:
	"""Expose the current player team's manager profile for match payloads"""
	_ensure_player_team_cached()
	if current_player_team.is_empty():
		return {}
	return get_manager_data_for_team(current_player_team)


func get_manager_data_for_team(team_data: Dictionary) -> Dictionary:
	"""Return the manager dictionary for a given team dictionary (if defined)"""
	if team_data.is_empty():
		return {}

	var manager_id = int(team_data.get("manager_id", -1))
	if manager_id <= 0:
		return {}

	var manager = _find_manager_by_id(manager_id)
	if manager.is_empty():
		push_warning(
			(
				"[StageManager] ⚠️ Manager ID %d not found for %s"
				% [manager_id, team_data.get("club_name", team_data.get("name", "Unknown Team"))]
			)
		)
		return {}

	return manager.duplicate(true)


func _find_manager_by_id(manager_id: int) -> Dictionary:
	"""Find manager by ID from loaded managers"""
	if managers_data.is_empty():
		_load_managers_data()

	for manager in managers_data:
		if manager.get("id", -1) == manager_id:
			return manager

	return {}


func _load_managers_data():
	"""Load managers data from JSON file"""
	var file = FileAccess.open("res://data/dummy_managers.json", FileAccess.READ)
	if not file:
		push_warning("[StageManager] Unable to open dummy_managers.json")
		managers_data = []
		return

	var json_str = file.get_as_text()
	file.close()

	var parsed = JSON.parse_string(json_str)
	if parsed == null:
		push_warning("[StageManager] Failed to parse dummy_managers.json")
		managers_data = []
		return

	if parsed is Dictionary and parsed.has("managers"):
		managers_data = parsed.get("managers", [])
	elif parsed is Array:
		managers_data = parsed
	else:
		push_warning("[StageManager] Unexpected manager data format")
		managers_data = []


# ============================================================================
# BM Two-Track: Hall of Fame Snapshot Integration
# ============================================================================


func set_team_snapshot(snapshot: Dictionary) -> void:
	"""Set the active team snapshot from Hall of Fame for stage battles"""
	selected_team_snapshot = snapshot.duplicate(true)
	team_snapshot_changed.emit(selected_team_snapshot)
	if snapshot.is_empty():
		print("[StageManager] Team snapshot cleared")
	else:
		print(
			(
				"[StageManager] Team snapshot set: %s (avg: %.1f)"
				% [snapshot.get("team_name", "Unknown"), float(snapshot.get("avg_overall", 0))]
			)
		)


func get_selected_team_snapshot() -> Dictionary:
	"""Get the currently selected team snapshot"""
	return selected_team_snapshot


func has_team_snapshot() -> bool:
	"""Check if a team snapshot is selected"""
	return not selected_team_snapshot.is_empty()


func clear_team_snapshot() -> void:
	"""Clear the selected team snapshot"""
	selected_team_snapshot = {}
	team_snapshot_changed.emit({})
	print("[StageManager] Team snapshot cleared")


func convert_snapshot_to_match_team(snapshot: Dictionary = {}) -> Dictionary:
	"""
	Convert a Hall of Fame snapshot to match-ready team format

	Args:
		snapshot: Team snapshot dictionary (uses selected if empty)

	Returns:
		Dictionary ready for match simulation
	"""
	var source := snapshot if not snapshot.is_empty() else selected_team_snapshot
	if source.is_empty():
		return {}

	# Build squad array from players
	var squad := []
	var players: Array = source.get("players", [])
	for player in players:
		(
			squad
			. append(
				{
					"player_id": player.get("id", 0),
					"name": player.get("name", "Unknown"),
					"position": player.get("position", "CM"),
					"ca": player.get("overall", 50),
					"pa": player.get("potential", 60),
					"stats": player.get("stats", {}),
					"dual_ending": player.get("dual_ending", ""),
				}
			)
		)

	# Get tactics from snapshot or defaults
	var tactics: Dictionary = source.get("tactics", {})
	if tactics.is_empty():
		tactics = {
			"attacking_intensity": 0.5,
			"defensive_line_height": 0.5,
			"width": 0.6,
			"pressing_trigger": 0.5,
			"tempo": 0.5,
			"directness": 0.5
		}

	return {
		"team_id": -1,  # Player team
		"club_name": source.get("team_name", "My Team"),
		"is_player_team": true,
		"avg_ca": source.get("avg_overall", 50),
		"squad": squad,
		"formation": source.get("tactics", {}).get("formation", "T442"),
		"tactics": tactics,
		"tactical_style": "Player",
		"academy_settings": source.get("academy_settings", {}),
	}


func get_match_ready_player_team() -> Dictionary:
	"""
	Get match-ready team data (snapshot if selected, else current_player_team)

	Returns:
		Dictionary ready for match simulation
	"""
	if has_team_snapshot():
		return convert_snapshot_to_match_team()

	_ensure_player_team_cached()
	return current_player_team
