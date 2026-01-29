class_name TimelineBinaryLoader
extends RefCounted

# Binary Format Constants
const MAGIC_HEADER = "OFRP"  # OpenFootball timeline (legacy header)
const VERSION_1 = 1  # Phase C: f32 coordinates
const VERSION_2 = 2  # Phase D: u16 compressed coordinates


# Struct Definitions (matching Rust)
class TimelineData:
	var metadata: MatchMetadata
	var ball_track: Array = []  # BallFrame objects
	# ✅ SSOT: key = track_id (0..21)
	var players: Dictionary = {}  # track_id -> PlayerSequence
	# Reverse lookup (metadata only): engine_player_id -> track_id
	var engine_to_track: Dictionary = {}
	var score_home: int = 0
	var score_away: int = 0
	var events: Array = []
	var timeline: Array = []
	var match_setup: Dictionary = {}  # P17: MatchSetup for player info (track_id -> name/position/overall)


class MatchMetadata:
	var duration: float
	var home_team_id: int
	var away_team_id: int
	var home_team_name: String
	var away_team_name: String


class BallFrame:
	var t: float
	var x: float
	var y: float
	var z: float
	var vx: float = 0.0  # 2025-12-11 P2: Ball velocity X (m/s)
	var vy: float = 0.0  # 2025-12-11 P2: Ball velocity Y (m/s)


class PlayerSequence:
	# Canonical identity for viewer/pipeline
	var track_id: int
	# Engine identity (metadata only, MUST NOT be used as key)
	var engine_player_id: int
	var team_id: int
	var kit_number: int
	var frames: Array = []  # PlayerFrame objects


class PlayerFrame:
	var t: float
	var x: float
	var y: float
	var vx: float = 0.0  # 2025-12-11: Player velocity X (m/s)
	var vy: float = 0.0  # 2025-12-11: Player velocity Y (m/s)


# Main loading function
static func load_from_buffer(buffer: PackedByteArray) -> TimelineData:
	print("[TimelineBinaryLoader] load_from_buffer called with %d bytes" % buffer.size())
	var stream = StreamPeerBuffer.new()
	stream.data_array = buffer
	stream.big_endian = false

	# Peek magic as u32 (LE). MRB0 = 0x3042524D, OFRP = ASCII "OFRP" = 0x4f465250
	var magic_num = stream.get_u32()
	stream.seek(0)
	print("[TimelineBinaryLoader] magic_num = 0x%08X" % magic_num)

	if magic_num == 0x3042524D:  # "MRB0" v3 hybrid
		print("[TimelineBinaryLoader] Detected MRB0 v3 hybrid format")
		return _load_v3_hybrid(stream)

	# Legacy OFRP path (v1/v2)
	print("[TimelineBinaryLoader] Using legacy OFRP path")
	return _load_legacy(stream)


# -------------------------
# Legacy v1/v2 (OFRP)
static func _load_legacy(stream: StreamPeerBuffer) -> TimelineData:
	var magic = stream.get_string(4)
	if magic != MAGIC_HEADER:
		push_error("Invalid Timeline Magic: " + magic)
		return null

	var version = stream.get_u32()
	if version < VERSION_1 or version > VERSION_2:
		push_error("Unsupported Timeline Version: " + str(version))
		return null

	var timeline_data = TimelineData.new()
	timeline_data.metadata = MatchMetadata.new()
	timeline_data.metadata.duration = stream.get_float()
	timeline_data.metadata.home_team_id = stream.get_u32()
	timeline_data.metadata.away_team_id = stream.get_u32()
	timeline_data.metadata.home_team_name = _read_string(stream)
	timeline_data.metadata.away_team_name = _read_string(stream)

	var ball_count = stream.get_u64()
	timeline_data.ball_track = []
	for i in range(ball_count):
		var frame = BallFrame.new()
		frame.t = stream.get_float()
		if version >= VERSION_2:
			frame.x = _decompress_x(stream.get_u16())
			frame.y = _decompress_y(stream.get_u16())
			frame.z = _decompress_z(stream.get_u16())
		else:
			frame.x = stream.get_float()
			frame.y = stream.get_float()
			frame.z = stream.get_float()
		timeline_data.ball_track.append(frame)

	var player_count = stream.get_u64()
	timeline_data.players = {}
	timeline_data.engine_to_track = {}
	for i in range(player_count):
		var seq = PlayerSequence.new()
		var engine_id := stream.get_u32()
		seq.track_id = i
		seq.engine_player_id = engine_id
		seq.team_id = stream.get_u32()
		seq.kit_number = stream.get_u32()

		var frame_count = stream.get_u64()
		seq.frames = []
		for j in range(frame_count):
			var frame = PlayerFrame.new()
			frame.t = stream.get_float()
			if version >= VERSION_2:
				frame.x = _decompress_x(stream.get_u16())
				frame.y = _decompress_y(stream.get_u16())
			else:
				frame.x = stream.get_float()
				frame.y = stream.get_float()
			seq.frames.append(frame)
		timeline_data.players[seq.track_id] = seq
		timeline_data.engine_to_track[seq.engine_player_id] = seq.track_id

	return timeline_data


# -------------------------
# Hybrid v3 (MRB0): [u32 magic][u8 version][u32 header_len][header JSON][body]
static func _load_v3_hybrid(stream: StreamPeerBuffer) -> TimelineData:
	print("[TimelineBinaryLoader] _load_v3_hybrid: starting... (buffer size: %d)" % stream.data_array.size())
	var timeline_data = TimelineData.new()

	# Magic + version + header
	var magic = stream.get_u32()
	var version = stream.get_u8()
	print("[TimelineBinaryLoader] _load_v3_hybrid: magic=0x%08X, version=%d" % [magic, version])
	if version != 3:
		push_error("Unsupported MRB version: " + str(version))
		return null

	var header_len = stream.get_u32()
	print("[TimelineBinaryLoader] _load_v3_hybrid: header_len=%d" % header_len)
	# ✅ Use get_data + get_string_from_utf8 to properly handle Korean/Unicode characters
	var header_result = stream.get_data(header_len)
	var header_json = ""
	if header_result[0] == OK and header_result[1] is PackedByteArray:
		header_json = header_result[1].get_string_from_utf8()
	else:
		push_error("[TimelineBinaryLoader] Failed to read header bytes")
		return null
	var header = {}
	if header_json != "":
		header = JSON.parse_string(header_json)
	timeline_data.metadata = MatchMetadata.new()
	timeline_data.metadata.duration = 0.0
	timeline_data.metadata.home_team_id = 0
	timeline_data.metadata.away_team_id = 1
	timeline_data.metadata.home_team_name = (
		String(header.teams.home.name)
		if (header.has("teams") and header.teams.has("home") and header.teams.home.has("name"))
		else "home"
	)
	timeline_data.metadata.away_team_name = (
		String(header.teams.away.name)
		if (header.has("teams") and header.teams.has("away") and header.teams.away.has("name"))
		else "away"
	)
	if header.has("score"):
		timeline_data.score_home = int(header.score.get("home", 0))
		timeline_data.score_away = int(header.score.get("away", 0))
	if header.has("events") and header.events is Array:
		timeline_data.events = header.events
	if header.has("timeline") and header.timeline is Array:
		timeline_data.timeline = header.timeline
	# P17: Parse match_setup for player info
	if header.has("match_setup") and header.match_setup is Dictionary:
		timeline_data.match_setup = header.match_setup.duplicate(true)

	# Body format v3
	var format_version = stream.get_u8()
	print("[TimelineBinaryLoader] _load_v3_hybrid: body format_version=%d" % format_version)
	if format_version != 3:
		push_warning("Unexpected body format_version: " + str(format_version) + ", continuing")

	timeline_data.metadata.duration = stream.get_float()
	timeline_data.score_home = stream.get_u8()
	timeline_data.score_away = stream.get_u8()
	print(
		(
			"[TimelineBinaryLoader] _load_v3_hybrid: duration=%.2f, score=%d-%d"
			% [timeline_data.metadata.duration, timeline_data.score_home, timeline_data.score_away]
		)
	)

	var event_count = stream.get_u32()
	print("[TimelineBinaryLoader] _load_v3_hybrid: event_count=%d" % event_count)
	if event_count > 10000:  # Sanity check
		push_error("[TimelineBinaryLoader] Suspicious event_count: %d, aborting" % event_count)
		return null
	for i in range(event_count):
		# minute, event_type, is_home, player_len, assist_len
		stream.get_u8()
		stream.get_u8()
		stream.get_u8()
		var p_len = stream.get_u16()
		if p_len > 0:
			stream.get_string(p_len)
		var a_len = stream.get_u16()
		if a_len > 0:
			stream.get_string(a_len)

	var ball_count = stream.get_u64()
	print("[TimelineBinaryLoader] _load_v3_hybrid: ball_count=%d (stream pos=%d)" % [ball_count, stream.get_position()])
	if ball_count > 1000000:  # Sanity check: ~24 bytes per frame = 24MB max reasonable
		push_error("[TimelineBinaryLoader] Suspicious ball_count: %d, aborting" % ball_count)
		return null
	timeline_data.ball_track = []
	for i in range(ball_count):
		var frame = BallFrame.new()
		frame.t = stream.get_float()
		frame.x = stream.get_float()
		frame.y = stream.get_float()
		frame.z = stream.get_float()
		frame.vx = stream.get_float()  # 2025-12-11 P2: Read ball velocity X
		frame.vy = stream.get_float()  # 2025-12-11 P2: Read ball velocity Y

		# HOTFIX (2025-12-22 FIX_2512 P0-A): Detect and normalize simulation units
		const MAX_FIELD_X = 105.0
		const MAX_FIELD_Y = 68.0
		const THRESHOLD = 200.0  # If > 200, it's simulation units

		if abs(frame.x) > THRESHOLD or abs(frame.y) > THRESHOLD:
			# Estimate scale factor (assuming ball is somewhere near field center)
			var scale_x = abs(frame.x) / (MAX_FIELD_X / 2.0) if abs(frame.x) > THRESHOLD else 1.0
			var scale_y = abs(frame.y) / (MAX_FIELD_Y / 2.0) if abs(frame.y) > THRESHOLD else 1.0
			var scale = max(scale_x, scale_y)  # Use larger scale to be safe

			# Normalize
			frame.x = frame.x / scale
			frame.y = frame.y / scale

			# Clamp to field boundaries
			frame.x = clamp(frame.x, 0, MAX_FIELD_X)
			frame.y = clamp(frame.y, 0, MAX_FIELD_Y)

			if i < 5:  # Log first few normalized frames
				push_warning("[TimelineBinaryLoader] Ball frame %d normalized: scale=%.2f" % [i, scale])

		timeline_data.ball_track.append(frame)

	# Phase 1.1: Ball coordinate validation (2025-12-22 FIX_2512)
	var min_x = INF
	var max_x = -INF
	var min_y = INF
	var max_y = -INF
	var corner_stuck_count = 0

	for frame in timeline_data.ball_track:
		min_x = min(min_x, frame.x)
		max_x = max(max_x, frame.x)
		min_y = min(min_y, frame.y)
		max_y = max(max_y, frame.y)

		# Corner detection: within 1m of edge
		if (frame.x < 1.0 or frame.x > 104.0) or (frame.y < 1.0 or frame.y > 67.0):
			corner_stuck_count += 1

	print("[TimelineBinaryLoader] _load_v3_hybrid: parsed %d ball frames" % timeline_data.ball_track.size())
	print("  Ball X range: %.2f ~ %.2f (expect: 0 ~ 105)" % [min_x, max_x])
	print("  Ball Y range: %.2f ~ %.2f (expect: 0 ~ 68)" % [min_y, max_y])
	print(
		(
			"  Corner stuck: %d / %d (%.1f%%)"
			% [
				corner_stuck_count,
				timeline_data.ball_track.size(),
				(
					100.0 * corner_stuck_count / timeline_data.ball_track.size()
					if timeline_data.ball_track.size() > 0
					else 0.0
				)
			]
		)
	)

	var player_count = stream.get_u64()
	print(
		(
			"[TimelineBinaryLoader] _load_v3_hybrid: player_count=%d (stream pos=%d)"
			% [player_count, stream.get_position()]
		)
	)
	if player_count > 100:  # Max 22 players * 2 + subs = ~50 max
		push_error("[TimelineBinaryLoader] Suspicious player_count: %d, aborting" % player_count)
		return null
	timeline_data.players = {}
	timeline_data.engine_to_track = {}
	for i in range(player_count):
		var seq = PlayerSequence.new()
		var engine_id := stream.get_u32()
		seq.track_id = i
		seq.engine_player_id = engine_id
		seq.team_id = 0
		seq.kit_number = 0
		var frame_count = stream.get_u64()
		if frame_count > 1000000:
			push_error(
				(
					"[TimelineBinaryLoader] Suspicious frame_count for player engine_id=%d(track_id=%d): %d, aborting"
					% [seq.engine_player_id, seq.track_id, frame_count]
				)
			)
			return null
		seq.frames = []
		for j in range(frame_count):
			var frame = PlayerFrame.new()
			frame.t = stream.get_float()
			frame.x = stream.get_float()
			frame.y = stream.get_float()
			frame.vx = stream.get_float()  # 2025-12-11: Read velocity X
			frame.vy = stream.get_float()  # 2025-12-11: Read velocity Y
			seq.frames.append(frame)
		timeline_data.players[seq.track_id] = seq
		timeline_data.engine_to_track[seq.engine_player_id] = seq.track_id
		if i < 5:  # Log first 5 players
			print(
				(
					"[TimelineBinaryLoader] Player %d: track_id=%d, engine_id=%d, frames=%d"
					% [i, seq.track_id, seq.engine_player_id, seq.frames.size()]
				)
			)

	# Phase 1.2: Player coordinate validation (2025-12-22 FIX_2512)
	if timeline_data.players.size() > 0:
		print("[TimelineBinaryLoader] Player validation:")
		for track_id in range(min(3, timeline_data.players.size())):
			var seq = timeline_data.players.get(track_id, null)
			if seq == null or seq.frames.size() == 0:
				continue
			var p_min_x = INF
			var p_max_x = -INF
			var p_min_y = INF
			var p_max_y = -INF
			for frame in seq.frames:
				p_min_x = min(p_min_x, frame.x)
				p_max_x = max(p_max_x, frame.x)
				p_min_y = min(p_min_y, frame.y)
				p_max_y = max(p_max_y, frame.y)
			print("  Player[%d]: X=%.1f~%.1f, Y=%.1f~%.1f" % [track_id, p_min_x, p_max_x, p_min_y, p_max_y])

	# ✅ P1 (2025-12-22 FIX_2512): Cluster density diagnostics
	if timeline_data.players.size() >= 5:
		var cluster_density = _calculate_cluster_density(timeline_data.players)
		print("  Cluster density: %.2f (expect < 0.3 for normal spacing)" % cluster_density)

	print("[TimelineBinaryLoader] _load_v3_hybrid: COMPLETE - %d players parsed" % timeline_data.players.size())
	return timeline_data


## ✅ P1 (2025-12-22 FIX_2512): Calculate average inter-player distance
## Returns average distance between first 5 players at their first frame
## - < 0.3: Normal tactical spacing
## - > 0.5: Clustering issue (players too close together)
static func _calculate_cluster_density(player_sequences: Dictionary) -> float:
	var total_distance = 0.0
	var count = 0

	# Sample first 5 players at their first frame
	var sample_size = min(5, player_sequences.size())
	var positions: Array = []

	for i in range(sample_size):
		var seq = player_sequences.get(i, null)
		if seq == null or seq.frames.size() == 0:
			continue
		var frame = seq.frames[0]
		positions.append(Vector2(frame.x, frame.y))

	# Calculate pairwise distances
	for i in range(positions.size()):
		for j in range(i + 1, positions.size()):
			var dist = positions[i].distance_to(positions[j])
			total_distance += dist
			count += 1

	return total_distance / count if count > 0 else 0.0


static func _read_string(stream: StreamPeerBuffer) -> String:
	var str_len = stream.get_u64()  # Rust String len is u64
	if str_len == 0:
		return ""
	# ✅ Use get_data + get_string_from_utf8 for proper Unicode support
	var result = stream.get_data(str_len)
	if result[0] == OK and result[1] is PackedByteArray:
		return result[1].get_string_from_utf8()
	return ""


# Phase D: Decompression functions
static func _decompress_x(x_u16: int) -> float:
	return (x_u16 / 65535.0) * 105.0


static func _decompress_y(y_u16: int) -> float:
	return (y_u16 / 65535.0) * 68.0


static func _decompress_z(z_u16: int) -> float:
	# Shift range from [0, 65535] back to [-5.0, +5.0]
	return (z_u16 / 65535.0) * 10.0 - 5.0


# =========================================================
# Pure-binary timeline loader (TimelineDataV3, Phase E helper)
# =========================================================

# Event type constants (matching Rust EventType enum)
const EVENT_TYPE_NAMES := {
	0: "goal",
	1: "shot",
	2: "shot_on_target",
	3: "shot_off_target",
	4: "shot_blocked",
	5: "save",
	6: "yellow_card",
	7: "red_card",
	8: "substitution",
	9: "injury",
	10: "corner",
	11: "freekick",
	12: "penalty",
	13: "offside",
	14: "foul",
	15: "key_chance",
	16: "pass",
	17: "tackle",
	18: "dribble",
}


static func load_pure_binary(buffer: PackedByteArray) -> Dictionary:
	if buffer.is_empty():
		return {}

	var stream := StreamPeerBuffer.new()
	stream.data_array = buffer
	stream.big_endian = false

	# Check format version (v2 starts with u8 version = 2)
	# v1 format starts with f32 duration, so first byte is typically 0x00-0x47 for durations 0-5400
	var first_byte := stream.get_u8()
	var format_version := 1
	var duration := 0.0
	var score_home := 0
	var score_away := 0
	var events: Array = []

	if first_byte == 2:
		# Format v2: has score and events
		format_version = 2
		duration = stream.get_float()
		score_home = stream.get_u8()
		score_away = stream.get_u8()

		# Read events
		var event_count := int(stream.get_u32())
		for _i in range(event_count):
			var minute := stream.get_u8()
			var event_type := stream.get_u8()
			var is_home_team := stream.get_u8() == 1

			# Player name
			var player_len := int(stream.get_u16())
			var player_name := ""
			if player_len > 0:
				var player_bytes := stream.get_data(player_len)
				if player_bytes[0] == OK:
					player_name = player_bytes[1].get_string_from_utf8()

			# Assist name
			var assist_len := int(stream.get_u16())
			var assist_name := ""
			if assist_len > 0:
				var assist_bytes := stream.get_data(assist_len)
				if assist_bytes[0] == OK:
					assist_name = assist_bytes[1].get_string_from_utf8()

			(
				events
				. append(
					{
						"minute": minute,
						"type": EVENT_TYPE_NAMES.get(event_type, "unknown"),
						"is_home_team": is_home_team,
						"player": player_name,
						"assist_by": assist_name if assist_name != "" else null,
					}
				)
			)
	else:
		# Format v1: first byte is part of duration float, need to reconstruct
		stream.seek(0)  # Reset to beginning
		duration = stream.get_float()

	# Ball frames
	var ball_count := int(stream.get_64())
	var ball_t := PackedFloat32Array()
	var ball_pos := PackedVector3Array()
	ball_t.resize(ball_count)
	ball_pos.resize(ball_count)

	for i in range(ball_count):
		var t := stream.get_float()
		var x := stream.get_float()
		var y := stream.get_float()
		var z := stream.get_float()
		ball_t[i] = t
		ball_pos[i] = Vector3(x, y, z)

	# Player frames (flat: id + t,x,y)
	var player_count := int(stream.get_64())
	var players := {}

	for i in range(player_count):
		var pid := int(stream.get_32())
		var t := stream.get_float()
		var px := stream.get_float()
		var py := stream.get_float()

		if not players.has(pid):
			players[pid] = {"t": [], "pos": []}
		players[pid]["t"].append(t)
		players[pid]["pos"].append(Vector2(px, py))

	# Convert arrays to Packed*Array for performance
	var players_packed := {}
	for pid in players.keys():
		var src: Dictionary = players[pid]
		var times: Array = src["t"]
		var poses: Array = src["pos"]

		var t_arr := PackedFloat32Array()
		t_arr.resize(times.size())
		for i_t in range(times.size()):
			t_arr[i_t] = float(times[i_t])

		var p_arr := PackedVector2Array()
		p_arr.resize(poses.size())
		for i_p in range(poses.size()):
			p_arr[i_p] = poses[i_p]

		players_packed[pid] = {"t": t_arr, "pos": p_arr}

	var result := {
		"format_version": format_version,
		"duration": duration,
		"ball":
		{
			"t": ball_t,
			"pos": ball_pos,
		},
		"players": players_packed,
	}

	# Add v2 fields if present
	if format_version == 2:
		result["score_home"] = score_home
		result["score_away"] = score_away
		result["events"] = events

	return result


# =========================================================
# Async Loading Support (Phase 4.2)
# =========================================================


## Async loader class for large timeline binary buffers
## Usage:
##   var loader = TimelineBinaryLoaderAsync.new()
##   loader.loading_completed.connect(_on_timeline_loaded)
##   loader.load_async(buffer)
##
class TimelineBinaryLoaderAsync:
	extends RefCounted
	signal loading_completed(timeline_data: TimelineData)
	signal loading_failed(error: String)

	var _task_id: int = -1
	var _buffer: PackedByteArray
	var _result: TimelineData = null

	## Start async loading of timeline data
	func load_async(buffer: PackedByteArray) -> void:
		_buffer = buffer
		# Use WorkerThreadPool for background parsing
		_task_id = WorkerThreadPool.add_task(_parse_in_background)

	## Check if loading is complete (call in _process if not using signals)
	func is_complete() -> bool:
		if _task_id < 0:
			return false
		return WorkerThreadPool.is_task_completed(_task_id)

	## Get result after completion
	func get_result() -> TimelineData:
		if _task_id >= 0 and WorkerThreadPool.is_task_completed(_task_id):
			WorkerThreadPool.wait_for_task_completion(_task_id)
			_task_id = -1
		return _result

	## Poll for completion (call each frame)
	func poll() -> bool:
		if _task_id < 0:
			return false
		if WorkerThreadPool.is_task_completed(_task_id):
			WorkerThreadPool.wait_for_task_completion(_task_id)
			_task_id = -1
			if _result != null:
				loading_completed.emit(_result)
			else:
				loading_failed.emit("Failed to parse timeline data")
			return true
		return false

	func _parse_in_background() -> void:
		# Use the current script reference to avoid class_name lookup issues in headless runs.
		_result = get_script().load_from_buffer(_buffer)


## Convenience function: Load timeline async and return via callback
## Returns the loader instance for polling/cancellation
static func load_async(buffer: PackedByteArray, callback: Callable) -> TimelineBinaryLoaderAsync:
	var loader = TimelineBinaryLoaderAsync.new()
	loader.loading_completed.connect(callback)
	loader.load_async(buffer)
	return loader


## Estimate loading time based on buffer size (for progress UI)
static func estimate_load_time_ms(buffer_size: int) -> int:
	# Rough estimate: ~1ms per 100KB on modern hardware
	return max(1, buffer_size / 102400)
