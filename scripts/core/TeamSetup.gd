## TeamSetup - Team roster (11 starters + 7 substitutes)
## Part of Game OS (MatchSetup Phase 17)
##
## CRITICAL: Substitutes stored as UID array (NO track_id)
## - starters: Array[_MatchPlayer] (11 players with track_id 0-21)
## - substitutes: Array (7 UIDs as String, NO track_id)
##
## track_id is allocated only when bench player is substituted onto field
##
## Priority: P0 (File 3 of 7)
class_name TeamSetup
extends RefCounted

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _MatchPlayer = preload("res://scripts/core/MatchPlayer.gd")
const _GameConstantsCore = preload("res://scripts/core/GameConstants.gd")

## Team identification
var name: String = "Team"
var side: String = "home"  # "home" or "away"

## Formation (e.g., "4-4-2", "4-3-3")
var formation: String = "4-4-2"

## Tactical settings
var tactics: Dictionary = {
	"attacking_intensity": 50, "defensive_line": 50, "pressing": 50, "tempo": 50, "width": 50, "passing_style": 50  # 0-100  # 0-100 (low = deep, high = high line)  # 0-100  # 0-100 (slow build-up vs fast counter)  # 0-100 (narrow vs wide)  # 0-100 (direct vs possession)
}

## CRITICAL: Starters have track_id assigned (0-10 home, 11-21 away)
var starters: Array[_MatchPlayer] = []  # Size 11

## CRITICAL: Substitutes stored as UID ONLY (NO track_id)
## track_id allocated only when substituted onto field
var substitutes: Array = []  # Size 7, stores UID strings


## Validate team roster
func validate() -> Dictionary:
	var errors: Array = []

	# Check starters count (FIFA regulation: 11)
	if starters.size() != _GameConstantsCore.STARTERS_PER_TEAM:
		errors.append("Starters must be exactly %d (got %d)" % [_GameConstantsCore.STARTERS_PER_TEAM, starters.size()])

	# Check substitutes count (FIFA regulation: max 7)
	if substitutes.size() != _GameConstantsCore.SUBSTITUTES_PER_TEAM:
		errors.append(
			"Substitutes must be exactly %d (got %d)" % [_GameConstantsCore.SUBSTITUTES_PER_TEAM, substitutes.size()]
		)

	# Check formation validity
	if not _is_valid_formation(formation):
		errors.append("Invalid formation: %s" % formation)

	# Check for null players in starters
	for i in range(starters.size()):
		if starters[i] == null:
			errors.append("Starter slot %d is null" % i)

	# Check for null/empty UIDs in substitutes
	for i in range(substitutes.size()):
		if substitutes[i] == null or str(substitutes[i]).is_empty():
			errors.append("Substitute slot %d is null or empty" % i)

	return {"valid": errors.is_empty(), "errors": errors}


## Check if formation is valid
func _is_valid_formation(fmt: String) -> bool:
	# Common formations
	var valid_formations = ["4-4-2", "4-3-3", "4-2-3-1", "3-5-2", "5-3-2", "4-5-1", "3-4-3", "4-1-4-1", "4-3-2-1"]
	return fmt in valid_formations


## Get total roster size (starters + subs)
func get_roster_size() -> int:
	return starters.size() + substitutes.size()


## Get starter at formation position (0-10)
func get_starter(index: int) -> _MatchPlayer:
	if index < 0 or index >= starters.size():
		push_error("[TeamSetup] Invalid starter index: %d" % index)
		return null
	return starters[index]


## Get substitute UID at bench position (0-6)
func get_substitute_uid(index: int) -> String:
	if index < 0 or index >= substitutes.size():
		push_error("[TeamSetup] Invalid substitute index: %d" % index)
		return ""
	return str(substitutes[index])


## Export to dictionary
func to_dict() -> Dictionary:
	var starter_dicts = []
	for player in starters:
		starter_dicts.append(player.to_dict() if player else null)

	return {
		"name": name,
		"side": side,
		"formation": formation,
		"tactics": tactics.duplicate(),
		"starters": starter_dicts,
		"substitutes": substitutes.duplicate()  # UIDs only
	}


## Get formation breakdown (e.g., "4-4-2" → [4, 4, 2])
func get_formation_array() -> Array:
	var parts = formation.split("-")
	var result = []
	for part in parts:
		result.append(int(part))
	return result


## Get all roster UIDs (11 starters + 7 subs = 18 total)
## Used by InteractiveMatchSetup for UID resolution
func get_roster_uids() -> Array:
	var uids = []

	# Add starter UIDs (11 players)
	for player in starters:
		if player:
			uids.append(player.uid)
		else:
			push_warning("[TeamSetup] Null player in starters")
			uids.append("csv:0")  # Fallback UID

	# Add substitute UIDs (7 players, already stored as UIDs)
	for sub_uid in substitutes:
		uids.append(str(sub_uid))

	if uids.size() != _GameConstantsCore.ROSTER_SIZE_PER_TEAM:
		push_error(
			(
				"[TeamSetup] get_roster_uids() returned %d UIDs (expected %d)"
				% [uids.size(), _GameConstantsCore.ROSTER_SIZE_PER_TEAM]
			)
		)

	return uids


## Apply default tactics based on formation
func apply_default_tactics() -> void:
	# Attacking formations: Higher tempo, intensity
	if formation in ["4-3-3", "4-2-3-1"]:
		tactics["attacking_intensity"] = 65
		tactics["tempo"] = 60
	# Defensive formations: Lower line, more pressing
	elif formation in ["5-3-2", "4-5-1"]:
		tactics["defensive_line"] = 40
		tactics["pressing"] = 60
	# Balanced formations
	else:
		tactics["attacking_intensity"] = 50
		tactics["defensive_line"] = 50
		tactics["pressing"] = 50
		tactics["tempo"] = 50
