## MatchSetup - GAME OS (Single Source of Truth for all matches)
## Part of Game OS (MatchSetup Phase 17)
##
## CRITICAL: This is THE Game OS
## - All modes build MatchSetup BEFORE simulation
## - All UI/Viewer/Stats consume MatchSetup
## - player_slots size is 22 (NOT 36)
##   - Home starters: track_id 0-10
##   - Away starters: track_id 11-21
##   - Substitutes: Stored in TeamSetup (NO track_id)
##
## Risk Mitigation:
## - Risk 1: player_slots.resize(22) enforces 0-21 range
## - Risk 3: MatchSetup persisted in match result
##
## Priority: P0 (File 4 of 7)
extends RefCounted
class_name MatchSetup  # ✅ SSOT Cleanup (2025-12-22): Game OS SSOT - Single Source of Truth

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _TeamSetup = preload("res://scripts/core/TeamSetup.gd")
const _PlayerSlot = preload("res://scripts/core/PlayerSlot.gd")
const _MatchPlayer = preload("res://scripts/core/MatchPlayer.gd")
const _GameConstantsCore = preload("res://scripts/core/GameConstants.gd")

# Legacy match_pipeline/MatchSetup.gd has been deleted (SSOT normalization)

## Match identification
var match_id: String = ""
var rng_seed: int = 0

## Teams
var home_team: _TeamSetup = null
var away_team: _TeamSetup = null

## CRITICAL: player_slots size is 22 (starters ONLY)
## Home starters: 0-10, Away starters: 11-21
## Substitutes stored in TeamSetup.substitutes (UID only)
var player_slots: Array[_PlayerSlot] = []

## Legacy metadata map (DEPRECATED - use player_slots instead)
## Still required for backward compatibility with some consumers
## Maps track_id -> player metadata Dictionary
var player_map: Dictionary = {}

## Match metadata
var match_type: String = "friendly"  # friendly, league, cup, etc.
var venue: String = "neutral"
var weather: String = "clear"
var timestamp: int = 0


## Constructor - CRITICAL: Resize to 22 slots
func _init():
	player_slots.resize(_GameConstantsCore.TOTAL_PLAYER_SLOTS)  # ⚠️ CRITICAL: 22 slots for starters only (NOT 36)
	timestamp = Time.get_unix_time_from_system()


## Get player by track_id (0-21)
func get_player(track_id: int) -> _MatchPlayer:
	if track_id < 0 or track_id >= _GameConstantsCore.TOTAL_PLAYER_SLOTS:
		push_error(
			"[MatchSetup] Invalid track_id: %d (must be 0-%d)" % [track_id, _GameConstantsCore.TOTAL_PLAYER_SLOTS - 1]
		)
		return null

	var slot = player_slots[track_id]
	if slot == null:
		push_warning("[MatchSetup] No player in slot %d" % track_id)
		return null

	return slot.player


## Get player name by track_id
func get_player_name(track_id: int) -> String:
	var player = get_player(track_id)
	return player.name if player else "Unknown"


## Get player slot by track_id
func get_slot(track_id: int) -> _PlayerSlot:
	if track_id < 0 or track_id >= _GameConstantsCore.TOTAL_PLAYER_SLOTS:
		push_error(
			"[MatchSetup] Invalid track_id: %d (must be 0-%d)" % [track_id, _GameConstantsCore.TOTAL_PLAYER_SLOTS - 1]
		)
		return null
	return player_slots[track_id]


## Check if player is active on field
func is_player_active(track_id: int) -> bool:
	var slot = get_slot(track_id)
	if slot == null:
		return false
	return slot.is_on_field()


## Get team side for track_id
func get_team_side(track_id: int) -> String:
	if track_id < 0 or track_id >= _GameConstantsCore.TOTAL_PLAYER_SLOTS:
		return "unknown"
	return "home" if track_id < _GameConstantsCore.AWAY_TRACK_ID_START else "away"


## Get team roster by side
func get_team_roster(side: String) -> Array[_MatchPlayer]:
	var roster: Array[_MatchPlayer] = []

	if side == "home":
		for i in range(_GameConstantsCore.STARTERS_PER_TEAM):
			var player = get_player(i)
			if player:
				roster.append(player)
	elif side == "away":
		for i in range(_GameConstantsCore.AWAY_TRACK_ID_START, _GameConstantsCore.TOTAL_PLAYER_SLOTS):
			var player = get_player(i)
			if player:
				roster.append(player)

	return roster


## Validate match setup
func validate() -> Dictionary:
	var errors: Array = []

	# Check teams exist
	if home_team == null:
		errors.append("Home team is null")
	if away_team == null:
		errors.append("Away team is null")

	# Validate teams
	if home_team:
		var home_validation = home_team.validate()
		if not home_validation.valid:
			errors.append("Home team validation failed: %s" % str(home_validation.errors))

	if away_team:
		var away_validation = away_team.validate()
		if not away_validation.valid:
			errors.append("Away team validation failed: %s" % str(away_validation.errors))

	# Check player slots filled
	var empty_slots = 0
	for i in range(_GameConstantsCore.TOTAL_PLAYER_SLOTS):
		if player_slots[i] == null:
			empty_slots += 1
			errors.append("Player slot %d is empty" % i)

	# Check track_id consistency
	for i in range(_GameConstantsCore.TOTAL_PLAYER_SLOTS):
		var slot = player_slots[i]
		if slot and slot.track_id != i:
			errors.append("Slot %d has mismatched track_id %d" % [i, slot.track_id])

	return {"valid": errors.is_empty(), "errors": errors, "warnings": {"empty_slots": empty_slots}}


## Export to dictionary (for persistence and engine)
func to_dict() -> Dictionary:
	var player_slot_dicts = []
	for slot in player_slots:
		player_slot_dicts.append(slot.to_dict() if slot else null)

	return {
		"match_id": match_id,
		"seed": rng_seed,
		"home_team": home_team.to_dict() if home_team else null,
		"away_team": away_team.to_dict() if away_team else null,
		"player_slots": player_slot_dicts,
		"player_map": player_map,  # Legacy metadata map (for backward compatibility)
		"match_type": match_type,
		"venue": venue,
		"weather": weather,
		"timestamp": timestamp
	}


## Create from dictionary (for loading from save/result)
static func from_dict(data: Dictionary):  # Returns MatchSetup (self-reference workaround)
	var _Self = preload("res://scripts/core/MatchSetup.gd")
	var match_setup = _Self.new()

	match_setup.match_id = str(data.get("match_id", ""))
	match_setup.rng_seed = int(data.get("seed", 0))
	match_setup.match_type = str(data.get("match_type", "friendly"))
	match_setup.venue = str(data.get("venue", "neutral"))
	match_setup.weather = str(data.get("weather", "clear"))
	match_setup.timestamp = int(data.get("timestamp", 0))

	# Restore player_map (legacy metadata)
	if data.has("player_map"):
		match_setup.player_map = data.get("player_map", {})

	# TODO: Reconstruct TeamSetup and PlayerSlots from dict
	# This requires MatchPlayer.from_dict() and TeamSetup.from_dict()

	return match_setup


## Get match summary string
func get_summary() -> String:
	var home_name = home_team.name if home_team else "Unknown"
	var away_name = away_team.name if away_team else "Unknown"
	return "%s vs %s (%s)" % [home_name, away_name, match_type]


## Count active players on field
func count_active_players() -> Dictionary:
	var home_count = 0
	var away_count = 0

	for i in range(_GameConstantsCore.STARTERS_PER_TEAM):
		if is_player_active(i):
			home_count += 1

	for i in range(_GameConstantsCore.AWAY_TRACK_ID_START, _GameConstantsCore.TOTAL_PLAYER_SLOTS):
		if is_player_active(i):
			away_count += 1

	return {"home": home_count, "away": away_count}
