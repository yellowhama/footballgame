## PlayerStatsProcessor - P0.1 Match Statistics System Integration
## Phase: Game OS v1.1 (Post v1.0)
## Created: 2025-12-19
##
## PURPOSE:
## Extract per-player statistics from MatchResult events and calculate player ratings.
## This is the first component of the Match Statistics pipeline that connects
## match simulation data to player progression (Career Mode).
##
## DATA FLOW:
## MatchResult (Rust) → PlayerStatsProcessor.extract_player_stats() → {uid: PlayerMatchStats}
##                    → PlayerStatsProcessor.calculate_player_rating() → rating (0-10)
##
## DEPENDENCIES:
## - MatchSetup (Game OS v1.0) - For track_id → UID mapping
## - MatchResult from Rust - events array with EventType enum
## - PlayerData - For loading player info (position for rating calculation)
##
## REFERENCES:
## - Plan: /home/hugh51/.claude/plans/quirky-questing-mountain.md (P0.1-P0.2)
## - Spec: docs/specs/progress/game_os/99_GAME_OS_V1_DECLARATION.md (Future Work P0)

class_name PlayerStatsProcessor
extends RefCounted

## ============================================================================
## INNER CLASSES
## ============================================================================


## Per-player match statistics
class PlayerMatchStats:
	extends RefCounted
	var uid: String = ""
	var track_id: int = -1
	var player_name: String = ""
	var position: String = ""
	var team_side: String = ""  # "home" or "away"

	# Offensive stats
	var goals: int = 0
	var assists: int = 0
	var shots: int = 0
	var shots_on_target: int = 0
	var shots_off_target: int = 0
	var shots_blocked: int = 0

	# Passing stats
	var passes: int = 0
	var passes_completed: int = 0
	var key_passes: int = 0

	# Defensive stats
	var tackles: int = 0
	var interceptions: int = 0
	var fouls: int = 0

	# Disciplinary
	var yellow_cards: int = 0
	var red_cards: int = 0

	# Other
	var dribbles: int = 0
	var saves: int = 0  # For goalkeepers
	var corners: int = 0
	var offsides: int = 0

	# Match info
	var minutes_played: int = 90
	var rating: float = 0.0  # 0.0-10.0 scale (calculated by calculate_player_rating)

	func _init(
		p_uid: String = "", p_track_id: int = -1, p_name: String = "", p_position: String = "", p_team: String = ""
	):
		uid = p_uid
		track_id = p_track_id
		player_name = p_name
		position = p_position
		team_side = p_team

	func to_dict() -> Dictionary:
		return {
			"uid": uid,
			"track_id": track_id,
			"player_name": player_name,
			"position": position,
			"team_side": team_side,
			"goals": goals,
			"assists": assists,
			"shots": shots,
			"shots_on_target": shots_on_target,
			"shots_off_target": shots_off_target,
			"shots_blocked": shots_blocked,
			"passes": passes,
			"passes_completed": passes_completed,
			"key_passes": key_passes,
			"tackles": tackles,
			"interceptions": interceptions,
			"fouls": fouls,
			"yellow_cards": yellow_cards,
			"red_cards": red_cards,
			"dribbles": dribbles,
			"saves": saves,
			"corners": corners,
			"offsides": offsides,
			"minutes_played": minutes_played,
			"rating": rating
		}


## ============================================================================
## PUBLIC API
## ============================================================================


## Extract per-player statistics from MatchResult
##
## PARAMETERS:
##   match_result: Dictionary - Rust MatchResult structure with:
##     - events: Array[Dictionary] - MatchEvent array from engine
##     - match_setup: Dictionary - MatchSetupExport (optional, for name→track_id mapping)
##   match_setup: MatchSetup - Game OS MatchSetup instance for track_id→UID mapping
##
## RETURNS:
##   Dictionary {uid: PlayerMatchStats} - Per-player statistics keyed by UID
##
## ALGORITHM:
##   1. Initialize PlayerMatchStats for all 22 players (track_id 0-21)
##   2. Build name→track_id lookup map from match_setup
##   3. Process each MatchEvent and map to player stats
##   4. Return dictionary keyed by UID
static func extract_player_stats(match_result: Dictionary, match_setup) -> Dictionary:
	if match_setup == null:
		push_error("[PlayerStatsProcessor] MatchSetup is null, cannot extract stats")
		return {}

	var events: Array = match_result.get("events", [])
	if events.is_empty():
		push_warning("[PlayerStatsProcessor] No events in match result")

	# Step 1: Initialize stats for all 22 players
	var player_stats: Dictionary = {}  # {uid: PlayerMatchStats}

	# GameConstantsCore.TOTAL_PLAYER_SLOTS should be 22 (0-21)
	const TOTAL_SLOTS = 22
	for track_id in range(TOTAL_SLOTS):
		var slot = match_setup.get_slot(track_id)
		if slot == null:
			continue

		var player = slot.player
		if player == null:
			continue

		var uid = slot.uid
		var player_name = player.name
		var position = player.position
		var team_side = match_setup.get_team_side(track_id)

		# Create PlayerMatchStats instance
		var stats = PlayerMatchStats.new(uid, track_id, player_name, position, team_side)
		player_stats[uid] = stats

	# Step 2: Process events (C7: Use player_track_id directly, no name lookup needed)
	for event in events:
		if not event is Dictionary:
			continue

		var event_type: String = event.get("type", "")
		var track_id: int = event.get("player_track_id", -1)  # C7: Direct track_id access
		var is_home_team: bool = event.get("is_home_team", false)
		var details: Dictionary = event.get("details", {})

		# Validate track_id range (0-21)
		if track_id == -1 or track_id > 21:
			continue  # Neutral event or invalid track_id

		# Get UID from track_id via MatchSetup
		var slot = match_setup.get_slot(track_id)
		if slot == null or slot.player == null:
			continue
		var uid = slot.player.uid

		# Get stats object
		var stats: PlayerMatchStats = player_stats.get(uid)
		if stats == null:
			continue

		# Map event to stats (using EventType enum from Rust)
		match event_type:
			"goal":
				stats.goals += 1
				stats.shots_on_target += 1  # Goals count as shots on target
				stats.shots += 1

				# C7: Check for assist via target_track_id
				var target_id: int = event.get("target_track_id", -1)
				if target_id != -1 and target_id <= 21:
					var assist_slot = match_setup.get_slot(target_id)
					if assist_slot and assist_slot.player:
						var assist_uid = assist_slot.player.uid
						var assist_stats = player_stats.get(assist_uid)
						if assist_stats:
							assist_stats.assists += 1
							assist_stats.key_passes += 1

			"own_goal":
				# C7: Own goal - player_track_id is the player who scored it
				# Penalize with negative goal (will affect rating)
				stats.goals -= 1

			"shot", "shot_on_target":
				stats.shots += 1
				stats.shots_on_target += 1

			"shot_off_target":
				stats.shots += 1
				stats.shots_off_target += 1

			"shot_blocked":
				stats.shots += 1
				stats.shots_blocked += 1

			"pass":
				stats.passes += 1
				# Assume successful pass (Rust doesn't track failed passes separately)
				stats.passes_completed += 1

			"key_chance":
				stats.key_passes += 1

			"tackle":
				stats.tackles += 1

			"dribble":
				stats.dribbles += 1

			"foul":
				stats.fouls += 1

			"yellow_card":
				stats.yellow_cards += 1

			"red_card":
				stats.red_cards += 1

			"save":
				stats.saves += 1

			"corner":
				stats.corners += 1

			"offside":
				stats.offsides += 1

			"substitution":
				# C7: Handle substitution for minutes_played tracking
				# player_track_id = player coming ON
				# target_track_id = player going OFF
				var player_off_id: int = event.get("target_track_id", -1)
				if player_off_id != -1 and player_off_id <= 21:
					var player_off_slot = match_setup.get_slot(player_off_id)
					if player_off_slot and player_off_slot.player:
						var player_off_uid = player_off_slot.player.uid
						var player_off_stats = player_stats.get(player_off_uid)
						if player_off_stats:
							var sub_minute: int = event.get("minute", 90)
							player_off_stats.minutes_played = sub_minute

			_:
				# Unhandled event type (e.g., kick_off, half_time, full_time)
				pass

	return player_stats


## Calculate player rating (0.0-10.0 scale) using FM 2023 style formula
##
## PARAMETERS:
##   stats: PlayerMatchStats - Player statistics from extract_player_stats()
##   match_result: String - Match result from player's team perspective ("win", "draw", "loss")
##
## RETURNS:
##   float - Rating between 0.0 and 10.0 (clamped)
##
## FORMULA:
##   Base rating: 6.0 (average performance)
##   + Match result bonus: win +0.5, draw +0.0, loss -0.3
##   + Goals: +1.5 per goal (position-weighted)
##   + Assists: +1.0 per assist (position-weighted)
##   + Shots on target: +0.1 per shot
##   + Passes completed: +0.01 per pass (position-weighted)
##   + Tackles: +0.2 per tackle (position-weighted)
##   - Fouls: -0.1 per foul
##   - Yellow cards: -0.5 per card
##   - Red cards: -2.0 per card
##   - Own goals: -3.0 per own goal
##
## POSITION WEIGHTS:
##   Forwards (ST, CF, LW, RW): Goals 2.0x, Assists 1.5x, Tackles 0.5x
##   Midfielders (CM, CAM, CDM, LM, RM): Goals 1.5x, Assists 1.5x, Passes 1.5x
##   Defenders (CB, LB, RB, LWB, RWB): Goals 1.0x, Tackles 2.0x, Passes 1.0x
##   Goalkeeper (GK): Special formula (saves, goals conceded)
static func calculate_player_rating(stats: PlayerMatchStats, match_result: String = "draw") -> float:
	var base_rating: float = 6.0

	# Match result bonus
	var result_bonus: float = 0.0
	match match_result.to_lower():
		"win":
			result_bonus = 0.5
		"draw":
			result_bonus = 0.0
		"loss":
			result_bonus = -0.3

	# Get position multipliers
	var position_mult = _get_position_multiplier(stats.position)

	# Special formula for goalkeepers
	if stats.position == "GK":
		return _calculate_goalkeeper_rating(stats, match_result)

	# Positive contributions
	var goals_bonus: float = stats.goals * 1.5 * position_mult.goals
	var assists_bonus: float = stats.assists * 1.0 * position_mult.assists
	var shots_bonus: float = stats.shots_on_target * 0.1
	var passes_bonus: float = stats.passes_completed * 0.01 * position_mult.passes
	var tackles_bonus: float = stats.tackles * 0.2 * position_mult.tackles

	# Negative contributions
	var fouls_penalty: float = stats.fouls * 0.1
	var cards_penalty: float = stats.yellow_cards * 0.5 + stats.red_cards * 2.0
	var own_goal_penalty: float = max(0, -stats.goals) * 3.0  # Negative goals are own goals

	# Calculate final rating
	var final_rating: float = base_rating + result_bonus
	final_rating += goals_bonus + assists_bonus + shots_bonus
	final_rating += passes_bonus + tackles_bonus
	final_rating -= fouls_penalty + cards_penalty + own_goal_penalty

	# Clamp to 0.0-10.0 range
	return clampf(final_rating, 0.0, 10.0)


## ============================================================================
## PRIVATE HELPERS
## ============================================================================


## Get position-specific multipliers for rating calculation
static func _get_position_multiplier(position: String) -> Dictionary:
	var pos_upper = position.to_upper()

	# Forward positions
	if pos_upper in ["ST", "CF", "LW", "RW", "SS"]:
		return {"goals": 2.0, "assists": 1.5, "passes": 0.8, "tackles": 0.5}

	# Midfielder positions
	elif pos_upper in ["CM", "CAM", "CDM", "LM", "RM", "AM"]:
		return {"goals": 1.5, "assists": 1.5, "passes": 1.5, "tackles": 1.0}

	# Defender positions
	elif pos_upper in ["CB", "LB", "RB", "LWB", "RWB", "DC", "DL", "DR"]:
		return {"goals": 1.0, "assists": 0.8, "passes": 1.0, "tackles": 2.0}

	# Default (neutral)
	else:
		return {"goals": 1.0, "assists": 1.0, "passes": 1.0, "tackles": 1.0}


## Calculate goalkeeper-specific rating
##
## GK rating formula:
##   Base: 6.0
##   + Saves: +0.3 per save
##   + Clean sheet: +1.0
##   - Goals conceded: -0.5 per goal
##   + Match result: win +0.5, draw +0.0, loss -0.3
static func _calculate_goalkeeper_rating(stats: PlayerMatchStats, match_result: String) -> float:
	var base_rating: float = 6.0

	# Match result bonus
	var result_bonus: float = 0.0
	match match_result.to_lower():
		"win":
			result_bonus = 0.5
		"draw":
			result_bonus = 0.0
		"loss":
			result_bonus = -0.3

	# Saves bonus
	var saves_bonus: float = stats.saves * 0.3

	# Note: Goals conceded info not available in PlayerMatchStats
	# Would need team statistics to calculate. For now, use match result as proxy.

	# Clean sheet bonus (estimated from match result and team)
	var clean_sheet_bonus: float = 0.0
	# TODO: Check if team conceded 0 goals (requires team stats)

	var final_rating: float = base_rating + result_bonus + saves_bonus + clean_sheet_bonus

	return clampf(final_rating, 0.0, 10.0)


## ============================================================================
## TESTING / DEBUG
## ============================================================================


## Print player stats for debugging
static func debug_print_stats(player_stats: Dictionary) -> void:
	print("[PlayerStatsProcessor] === Player Statistics ===")
	for uid in player_stats:
		var stats: PlayerMatchStats = player_stats[uid]
		print("  %s (%s) [%s]:" % [stats.player_name, stats.position, stats.team_side])
		print(
			(
				"    Goals: %d | Assists: %d | Shots: %d (%d on target)"
				% [stats.goals, stats.assists, stats.shots, stats.shots_on_target]
			)
		)
		print(
			(
				"    Passes: %d/%d | Tackles: %d | Fouls: %d"
				% [stats.passes_completed, stats.passes, stats.tackles, stats.fouls]
			)
		)
		print("    Cards: Y%d R%d | Rating: %.1f" % [stats.yellow_cards, stats.red_cards, stats.rating])
