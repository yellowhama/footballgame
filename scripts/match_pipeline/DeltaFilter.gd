## DeltaFilter - Only emit snapshots with meaningful changes
## Part of Uber Realtime Architecture implementation
##
## Concept: Uber Fireball - drop redundant updates to reduce bandwidth/processing
## Reference: docs/specs/fix_2601/0113/UBER_REALTIME_ARCHITECTURE.md
##
## Usage:
##   var filter = DeltaFilter.new()
##   if filter.should_emit(snapshot):
##       emit_snapshot(snapshot)
##
class_name DeltaFilter
extends RefCounted

## Thresholds (in meters)
var ball_threshold: float = 0.5  # Ball must move 0.5m to trigger emit
var player_threshold: float = 0.5  # Player must move 0.5m to count as "changed"
var min_changed_players: int = 2  # Need at least 2 players changed to emit (if no ball change)

## Last emitted state
var _last_ball_pos: Vector2 = Vector2.ZERO
var _last_player_positions: Dictionary = {}  # track_id -> Vector2

## Statistics
var emit_count: int = 0
var drop_count: int = 0


## Check if snapshot should be emitted
## Returns true if snapshot has meaningful changes
func should_emit(snapshot: Dictionary) -> bool:
	# Rule 1: Always emit if there are events (goals, fouls, cards, etc.)
	var events = snapshot.get("events", [])
	if events.size() > 0:
		_update_last_state(snapshot)
		emit_count += 1
		return true

	# Rule 2: Emit if ball moved significantly
	var ball = snapshot.get("ball", {})
	var ball_pos = _extract_position(ball)
	if _last_ball_pos.distance_to(ball_pos) > ball_threshold:
		_update_last_state(snapshot)
		emit_count += 1
		return true

	# Rule 3: Emit if multiple players moved significantly
	var changed_count = _count_changed_players(snapshot)
	if changed_count >= min_changed_players:
		_update_last_state(snapshot)
		emit_count += 1
		return true

	# Drop this snapshot
	drop_count += 1
	return false


## Count how many players have moved beyond threshold
func _count_changed_players(snapshot: Dictionary) -> int:
	var count = 0
	var players = snapshot.get("players", {})

	for track_id in players:
		var player_data = players[track_id]
		var pos = _extract_position(player_data)
		var last_pos = _last_player_positions.get(track_id, Vector2.ZERO)

		if last_pos.distance_to(pos) > player_threshold:
			count += 1

	return count


## Extract Vector2 position from various data formats
func _extract_position(data: Dictionary) -> Vector2:
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


## Update last state after emit
func _update_last_state(snapshot: Dictionary) -> void:
	# Update ball position
	var ball = snapshot.get("ball", {})
	_last_ball_pos = _extract_position(ball)

	# Update player positions
	var players = snapshot.get("players", {})
	for track_id in players:
		var player_data = players[track_id]
		_last_player_positions[track_id] = _extract_position(player_data)


## Reset filter state (call at match start)
func reset() -> void:
	_last_ball_pos = Vector2.ZERO
	_last_player_positions.clear()
	emit_count = 0
	drop_count = 0


## Get emit ratio (for debugging/stats)
func get_emit_ratio() -> float:
	var total = emit_count + drop_count
	if total == 0:
		return 1.0
	return float(emit_count) / float(total)


## Get statistics dictionary
func get_stats() -> Dictionary:
	return {
		"emit_count": emit_count,
		"drop_count": drop_count,
		"emit_ratio": get_emit_ratio(),
		"tracked_players": _last_player_positions.size()
	}


## Configure thresholds
func configure(config: Dictionary) -> void:
	if config.has("ball_threshold"):
		ball_threshold = float(config.get("ball_threshold"))
	if config.has("player_threshold"):
		player_threshold = float(config.get("player_threshold"))
	if config.has("min_changed_players"):
		min_changed_players = int(config.get("min_changed_players"))
