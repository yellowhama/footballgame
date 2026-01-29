## PlayerSlot - track_id ↔ player mapping (22 slots ONLY)
## Part of Game OS (MatchSetup Phase 17)
##
## CRITICAL: track_id MUST be 0-21 (22 starters only)
## - Home starters: 0-10 (11 players)
## - Away starters: 11-21 (11 players)
## - Substitutes: Stored in TeamSetup.substitutes (NO track_id)
##
## Risk Mitigation: Enforces track_id range in constructor
##
## Priority: P0 (File 2 of 7)
class_name PlayerSlot
extends RefCounted

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _MatchPlayer = preload("res://scripts/core/MatchPlayer.gd")

enum SlotStatus { ACTIVE, SUBSTITUTED, INJURED, RED_CARDED }  ## Currently playing on field  ## Substituted off (replaced by bench player)  ## Injured during match (cannot return)  ## Sent off (cannot be replaced)

## CRITICAL: track_id MUST be 0-21
var track_id: int = 0

## Player in this slot
var player: _MatchPlayer = null

## Current status during match
var status: SlotStatus = SlotStatus.ACTIVE

## Minutes when status changed (for simulation tracking)
var status_changed_minute: int = 0


## Constructor with track_id validation
func _init(tid: int, p: _MatchPlayer):
	# CRITICAL: Enforce 0-21 range (Risk 1 mitigation)
	if tid < 0 or tid >= 22:
		push_error("[PlayerSlot] Invalid track_id: %d (must be 0-21)" % tid)
		track_id = 0  # Fallback to prevent crash
	else:
		track_id = tid

	player = p


## Check if player is on field (active)
func is_on_field() -> bool:
	return status == SlotStatus.ACTIVE


## Check if player can be substituted
func can_be_substituted() -> bool:
	return status == SlotStatus.ACTIVE


## Substitute this player off (called when replaced)
func substitute_off(minute: int) -> void:
	if status != SlotStatus.ACTIVE:
		push_warning("[PlayerSlot] Cannot substitute player not active (track_id=%d)" % track_id)
		return

	status = SlotStatus.SUBSTITUTED
	status_changed_minute = minute


## Mark player as injured
func mark_injured(minute: int) -> void:
	status = SlotStatus.INJURED
	status_changed_minute = minute


## Mark player as sent off (red card)
func mark_sent_off(minute: int) -> void:
	status = SlotStatus.RED_CARDED
	status_changed_minute = minute


## Get team side (home or away)
func get_team_side() -> String:
	return "home" if track_id < 11 else "away"


## Export to dictionary
func to_dict() -> Dictionary:
	return {
		"track_id": track_id,
		"player": player.to_dict() if player else null,
		"status": SlotStatus.keys()[status],
		"status_changed_minute": status_changed_minute
	}
