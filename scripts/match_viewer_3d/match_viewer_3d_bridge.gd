extends RefCounted
class_name MatchViewer3DBridge

## Bridge between existing 2D timeline system and 3D MatchViewer.
## Converts StandardSnapshot format to MatchViewer3D format.

# Field dimensions (meters) - matches MatchViewer3D
const FIELD_LENGTH := 105.0
const FIELD_WIDTH := 68.0


## Convert StandardSnapshot to MatchViewer3D replay format
## Input: StandardSnapshot from PositionSnapshotAdapter
## Output: Dictionary compatible with MatchViewer3D.load_replay_data()
static func convert_timeline_to_3d(
	position_data: Dictionary, rosters: Dictionary, match_info: Dictionary, best_moments: Array = []
) -> Dictionary:
	var result := {
		"frames": [],
		"duration_ms": 0,
		"home_team": str(match_info.get("home_name", match_info.get("home_team", "Home"))),
		"away_team": str(match_info.get("away_name", match_info.get("away_team", "Away"))),
		"home_score": int(match_info.get("home_score", 0)),
		"away_score": int(match_info.get("away_score", 0)),
		"best_moments": best_moments,
		"rosters": rosters
	}

	# Check if position_data has the expected structure
	if not position_data.has("ball") or not position_data.has("players"):
		push_warning("[MatchViewer3DBridge] Invalid position_data structure")
		return result

	# Get ball and player tracks
	var ball_track: Array = position_data.get("ball", [])
	var player_tracks: Dictionary = position_data.get("players", {})

	if ball_track.is_empty():
		push_warning("[MatchViewer3DBridge] No ball data in position_data")
		return result

	# Find duration from the last ball sample
	var max_time_ms := 0
	for sample in ball_track:
		if sample is Dictionary:
			var t := float(sample.get("t", sample.get("timestamp", 0.0)))
			var t_ms := int(t * 1000.0) if t < 200 else int(t)  # seconds vs ms detection
			max_time_ms = max(max_time_ms, t_ms)
	result["duration_ms"] = max_time_ms

	# Generate frames at regular intervals (50ms = 20fps, matching engine tick rate)
	const FRAME_INTERVAL_MS := 50
	var current_time_ms := 0

	while current_time_ms <= max_time_ms:
		var frame := _create_frame_at_time(current_time_ms, ball_track, player_tracks)
		result["frames"].append(frame)
		current_time_ms += FRAME_INTERVAL_MS

	return result


## Create a single frame at a specific time
static func _create_frame_at_time(time_ms: int, ball_track: Array, player_tracks: Dictionary) -> Dictionary:
	var frame := {"time_ms": time_ms, "t": float(time_ms) / 1000.0, "ball": Vector2(52.5, 34.0), "players": []}  # Center of field as default

	# Interpolate ball position
	var ball_sample := _interpolate_sample(ball_track, time_ms)
	if not ball_sample.is_empty():
		frame["ball"] = Vector2(float(ball_sample.get("x", 52.5)), float(ball_sample.get("y", 34.0)))

	# Get ball height for header detection
	var ball_height := float(ball_sample.get("z", 0.0))

	# Interpolate player positions (22 players: 0-10 home, 11-21 away)
	var players: Array = []
	for i in range(22):
		var track_id := str(i)
		var player_data := {"position": Vector2(52.5, 34.0), "pose": PoseBuilder.PoseType.IDLE}

		if player_tracks.has(track_id):
			var track: Variant = player_tracks[track_id]
			if track is Array:
				var sample := _interpolate_sample(track, time_ms)
				if not sample.is_empty():
					player_data["position"] = Vector2(float(sample.get("x", 52.5)), float(sample.get("y", 34.0)))
					# Derive pose from state, velocity, and ball height
					player_data["pose"] = _get_pose_from_sample(sample, ball_height)

		players.append(player_data)

	frame["players"] = players
	return frame


## Interpolate a sample from a track at a specific time
static func _interpolate_sample(track: Array, time_ms: int) -> Dictionary:
	if track.is_empty():
		return {}

	var time_sec: float = float(time_ms) / 1000.0

	# Find before and after samples
	var before: Dictionary = {}
	var after: Dictionary = {}
	var before_t: float = -INF
	var after_t: float = INF

	for sample in track:
		if not (sample is Dictionary):
			continue
		var t: float = float(sample.get("t", sample.get("timestamp", 0.0)))

		if t <= time_sec and t > before_t:
			before_t = t
			before = sample

		if t >= time_sec and t < after_t:
			after_t = t
			after = sample

	# If no samples found, return empty
	if before.is_empty() and after.is_empty():
		return track[0] if not track.is_empty() else {}

	# If only one side found, use that
	if before.is_empty():
		return after
	if after.is_empty():
		return before

	# If same sample, no interpolation needed
	if before_t == after_t:
		return before

	# Linear interpolation
	var alpha: float = (time_sec - before_t) / (after_t - before_t)
	alpha = clamp(alpha, 0.0, 1.0)

	var result := {
		"t": time_sec,
		"x": lerp(float(before.get("x", 0.0)), float(after.get("x", 0.0)), alpha),
		"y": lerp(float(before.get("y", 0.0)), float(after.get("y", 0.0)), alpha)
	}

	# Interpolate z if present (ball height) - FIX_2601: Also check "height" (Rust serialization)
	if before.has("z") or after.has("z") or before.has("height") or after.has("height"):
		var z_before = float(before.get("z", before.get("height", 0.0)))
		var z_after = float(after.get("z", after.get("height", 0.0)))
		result["z"] = lerp(z_before, z_after, alpha)

	# Interpolate velocity if present
	if before.has("vx") or after.has("vx"):
		result["vx"] = lerp(float(before.get("vx", 0.0)), float(after.get("vx", 0.0)), alpha)
	if before.has("vy") or after.has("vy"):
		result["vy"] = lerp(float(before.get("vy", 0.0)), float(after.get("vy", 0.0)), alpha)

	# State from before sample (can't interpolate discrete states)
	if before.has("state"):
		result["state"] = before.get("state")

	return result


## Derive pose type from sample data using PoseBuilder
## Returns PoseBuilder.PoseType enum value (int)
static func _get_pose_from_sample(sample: Dictionary, ball_height: float = 0.0) -> int:
	return PoseBuilder.calculate_pose_from_sample(sample, ball_height)


## Legacy: Convert integer state to pose string (backwards compatibility)
## Prefer using _get_pose_from_sample() with PoseBuilder instead
static func _state_int_to_pose(state: int, sample: Dictionary) -> String:
	var vx := float(sample.get("vx", 0.0))
	var vy := float(sample.get("vy", 0.0))
	var speed := sqrt(vx * vx + vy * vy)

	match state:
		0:  # Idle
			return "idle" if speed < 0.1 else "walk"
		1:  # Moving
			if speed < 0.1:
				return "idle"
			elif speed < 3.0:
				return "walk"
			elif speed < 6.0:
				return "run"
			else:
				return "sprint"
		2:  # Ball possession
			return "dribble" if speed > 1.0 else "control"
		3:  # Kick preparation
			return "pass"
		_:
			return "idle"


## Convert a single StandardSnapshot to MatchViewer3D frame format
## Used for real-time playback from MatchTimelineController
static func convert_snapshot_to_frame(snapshot: Dictionary) -> Dictionary:
	if snapshot.is_empty():
		return {}

	var frame := {
		"time_ms": int(snapshot.get("t_ms", 0)),
		"t": float(snapshot.get("t_ms", 0)) / 1000.0,
		"ball": Vector2(52.5, 34.0),
		"players": []
	}

	# Ball position
	if snapshot.has("ball") and snapshot.ball is Dictionary:
		var ball_data: Dictionary = snapshot.ball
		if ball_data.has("pos"):
			var pos = ball_data.get("pos")
			if pos is Vector2:
				frame["ball"] = pos
			elif pos is Dictionary:
				frame["ball"] = Vector2(float(pos.get("x", 52.5)), float(pos.get("y", 34.0)))

	# Get ball height for header detection
	var ball_height := 0.0
	if snapshot.has("ball") and snapshot.ball is Dictionary:
		ball_height = float(snapshot.ball.get("z", snapshot.ball.get("height", 0.0)))

	# Players (StandardSnapshot uses string keys "0" to "21")
	var players: Array = []
	for i in range(22):
		var track_id := str(i)
		var player_data := {"position": Vector2(52.5, 34.0), "pose": PoseBuilder.PoseType.IDLE}

		if snapshot.has("players") and snapshot.players is Dictionary:
			if snapshot.players.has(track_id):
				var p: Dictionary = snapshot.players[track_id]
				if p.has("pos"):
					var pos = p.get("pos")
					if pos is Vector2:
						player_data["position"] = pos
					elif pos is Dictionary:
						player_data["position"] = Vector2(float(pos.get("x", 52.5)), float(pos.get("y", 34.0)))

				# Derive pose using PoseBuilder
				player_data["pose"] = _get_pose_from_sample(p, ball_height)

		players.append(player_data)

	frame["players"] = players
	return frame


## Get team colors from rosters (if available)
static func extract_team_colors(rosters: Dictionary) -> Dictionary:
	var colors := {
		"home_shirt": Color(0.9, 0.1, 0.1),  # Red
		"home_shorts": Color.WHITE,
		"away_shirt": Color(0.1, 0.1, 0.9),  # Blue
		"away_shorts": Color.WHITE
	}

	# Try to extract from roster data
	if rosters.has("home") and rosters.home is Dictionary:
		var home: Dictionary = rosters.home
		if home.has("colors") and home.colors is Dictionary:
			var c: Dictionary = home.colors
			if c.has("shirt"):
				colors["home_shirt"] = _parse_color(c.get("shirt"))
			if c.has("shorts"):
				colors["home_shorts"] = _parse_color(c.get("shorts"))

	if rosters.has("away") and rosters.away is Dictionary:
		var away: Dictionary = rosters.away
		if away.has("colors") and away.colors is Dictionary:
			var c: Dictionary = away.colors
			if c.has("shirt"):
				colors["away_shirt"] = _parse_color(c.get("shirt"))
			if c.has("shorts"):
				colors["away_shorts"] = _parse_color(c.get("shorts"))

	return colors


## Parse color from various formats
static func _parse_color(value: Variant) -> Color:
	if value is Color:
		return value
	if value is String:
		# Try hex format
		if value.begins_with("#"):
			return Color.html(value)
		# Try named color
		return Color(value)
	if value is Dictionary:
		return Color(
			float(value.get("r", 1.0)),
			float(value.get("g", 1.0)),
			float(value.get("b", 1.0)),
			float(value.get("a", 1.0))
		)
	return Color.WHITE
