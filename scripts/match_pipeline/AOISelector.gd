## AOISelector - Area of Interest based Level of Detail
## Part of Uber Realtime Architecture implementation
##
## Concept: Uber H3 - prioritize updates for entities near the ball
## Reference: docs/specs/fix_2601/0113/UBER_REALTIME_ARCHITECTURE.md
##
## LOD Tiers:
##   Tier 0: Ball proximity < 10m - Every frame update
##   Tier 1: Ball proximity < 20m - Every frame update
##   Tier 2: Ball proximity < 30m - Every 2nd frame
##   Tier 3: Ball proximity > 30m - Every 4th frame
##
## Usage:
##   var aoi = AOISelector.new()
##   var active = aoi.get_active_track_ids(ball_pos, player_positions)
##   # active.high_priority = players to update this frame
##   # active.low_priority = players to skip this frame
##
class_name AOISelector
extends RefCounted

enum Mode {
	BALL_CENTRIC,    # Priority based on distance from ball
	CAMERA_CENTRIC,  # Priority based on distance from camera center (future)
	FULL             # Update all players every frame (no LOD)
}

## Current mode
var mode: Mode = Mode.BALL_CENTRIC

## Tier distance thresholds (in meters)
var tier_0_radius: float = 10.0  # High priority: every frame
var tier_1_radius: float = 20.0  # High priority: every frame
var tier_2_radius: float = 30.0  # Medium priority: every 2nd frame
# Beyond tier_2 = Low priority: every 4th frame

## Frame counter for LOD cycling
var _frame_counter: int = 0

## Statistics
var high_priority_count: int = 0
var low_priority_count: int = 0


## Get which track_ids should be updated this frame
## Returns: { "high_priority": [track_ids], "low_priority": [track_ids] }
func get_active_track_ids(ball_pos: Vector2, player_positions: Dictionary) -> Dictionary:
	_frame_counter += 1

	var result = {
		"high_priority": [],   # Update this frame
		"low_priority": []     # Skip this frame (use last known position)
	}

	# FULL mode: everyone is high priority
	if mode == Mode.FULL:
		for track_id in player_positions:
			result["high_priority"].append(track_id)
		high_priority_count += player_positions.size()
		return result

	# BALL_CENTRIC mode: priority based on ball distance
	for track_id in player_positions:
		var pos = _extract_position(player_positions[track_id])
		var distance = ball_pos.distance_to(pos)
		var tier = _get_tier(distance)

		if _should_update_this_frame(tier):
			result["high_priority"].append(track_id)
			high_priority_count += 1
		else:
			result["low_priority"].append(track_id)
			low_priority_count += 1

	return result


## Get tier based on distance from ball
func _get_tier(distance: float) -> int:
	if distance < tier_0_radius:
		return 0
	elif distance < tier_1_radius:
		return 1
	elif distance < tier_2_radius:
		return 2
	else:
		return 3


## Check if this tier should be updated this frame
func _should_update_this_frame(tier: int) -> bool:
	match tier:
		0, 1:
			return true  # Every frame
		2:
			return _frame_counter % 2 == 0  # Every 2nd frame
		3:
			return _frame_counter % 4 == 0  # Every 4th frame
	return true


## Extract Vector2 position from various data formats
func _extract_position(data) -> Vector2:
	if data is Vector2:
		return data

	if data is Dictionary:
		# Format 1: {"pos": Vector2}
		if data.has("pos"):
			var pos = data.get("pos")
			if pos is Vector2:
				return pos

		# Format 2: {"x": float, "y": float}
		if data.has("x") and data.has("y"):
			return Vector2(float(data.get("x", 0)), float(data.get("y", 0)))

		# Format 3: {"position": Vector2}
		if data.has("position"):
			var pos = data.get("position")
			if pos is Vector2:
				return pos

	return Vector2.ZERO


## Reset frame counter and statistics
func reset() -> void:
	_frame_counter = 0
	high_priority_count = 0
	low_priority_count = 0


## Get statistics
func get_stats() -> Dictionary:
	var total = high_priority_count + low_priority_count
	return {
		"frame_count": _frame_counter,
		"high_priority_count": high_priority_count,
		"low_priority_count": low_priority_count,
		"high_priority_ratio": float(high_priority_count) / float(max(1, total))
	}


## Configure thresholds
func configure(config: Dictionary) -> void:
	if config.has("mode"):
		var mode_str = str(config.get("mode")).to_upper()
		match mode_str:
			"BALL_CENTRIC":
				mode = Mode.BALL_CENTRIC
			"CAMERA_CENTRIC":
				mode = Mode.CAMERA_CENTRIC
			"FULL":
				mode = Mode.FULL

	if config.has("tier_0_radius"):
		tier_0_radius = float(config.get("tier_0_radius"))
	if config.has("tier_1_radius"):
		tier_1_radius = float(config.get("tier_1_radius"))
	if config.has("tier_2_radius"):
		tier_2_radius = float(config.get("tier_2_radius"))
	if config.has("radius"):
		# Shorthand: set all tiers proportionally
		var r = float(config.get("radius"))
		tier_0_radius = r * 0.5
		tier_1_radius = r
		tier_2_radius = r * 1.5


## Apply AOI filtering to a snapshot
## Returns filtered snapshot with only high-priority players fully updated
func filter_snapshot(snapshot: Dictionary, ball_pos: Vector2) -> Dictionary:
	var players = snapshot.get("players", {})
	if players.is_empty():
		return snapshot

	var active = get_active_track_ids(ball_pos, players)

	# In current implementation, we don't actually remove low-priority players
	# Instead, we mark them for potential interpolation/prediction by viewers
	# This allows viewers to decide how to handle low-priority updates

	if not snapshot.has("aoi_metadata"):
		snapshot["aoi_metadata"] = {}

	snapshot["aoi_metadata"]["high_priority"] = active["high_priority"]
	snapshot["aoi_metadata"]["low_priority"] = active["low_priority"]
	snapshot["aoi_metadata"]["frame"] = _frame_counter

	return snapshot
