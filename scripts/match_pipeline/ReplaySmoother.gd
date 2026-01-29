extends RefCounted
class_name ReplaySmoother

# Visual-only smoothing for replay snapshots.
# - Does NOT mutate SSOT position_data.
# - Only mutates the emitted snapshot dictionary (display pose).
#
# Expected snapshot shape (partial):
# {
#   "t_ms": int,
#   "players": { "0": {"pos": Vector2, ...}, ... "21": {...} },
#   "ball": { "pos": Vector2, "owner_id": int, ... },
# }

var _prev_t_ms: int = -1
var _smooth_players: Dictionary = {}  # track_id(int) -> Vector2
var _smooth_ball: Vector2 = Vector2.ZERO
var _has_ball: bool = false

# Tunables (engine meters)
var player_alpha: float = 0.35
var ball_alpha: float = 0.45
var min_separation_m: float = 0.9
var separation_push: float = 0.25
var separation_passes: int = 1


func reset() -> void:
	_prev_t_ms = -1
	_smooth_players.clear()
	_smooth_ball = Vector2.ZERO
	_has_ball = false


func apply(snapshot: Dictionary) -> Dictionary:
	if snapshot.is_empty():
		return snapshot

	var t_ms: int = int(snapshot.get("t_ms", 0))
	var dt_sec: float = 0.05
	if _prev_t_ms >= 0:
		dt_sec = max(0.001, float(t_ms - _prev_t_ms) / 1000.0)
	_prev_t_ms = t_ms

	var players: Variant = snapshot.get("players", {})
	if players is Dictionary and not (players as Dictionary).is_empty():
		_apply_player_smoothing(players as Dictionary, dt_sec)
		for _p: int in range(max(0, separation_passes)):
			_apply_separation(players as Dictionary)
		snapshot["players"] = players

	var ball: Variant = snapshot.get("ball", {})
	if ball is Dictionary and (ball as Dictionary).has("pos"):
		_apply_ball_smoothing(snapshot, ball as Dictionary, dt_sec)
		snapshot["ball"] = ball

	return snapshot


func _apply_player_smoothing(players: Dictionary, dt_sec: float) -> void:
	for key: Variant in players.keys():
		var tid: int = _to_tid(key)
		if tid < 0 or tid > 21:
			continue

		var pdata: Variant = players[key]
		if not (pdata is Dictionary):
			continue

		var pdata_dict: Dictionary = pdata as Dictionary
		var raw_pos: Vector2 = pdata_dict.get("pos", Vector2.ZERO)
		var prev_pos: Vector2 = _smooth_players.get(tid, raw_pos)
		var sm_pos: Vector2 = prev_pos.lerp(raw_pos, player_alpha)

		_smooth_players[tid] = sm_pos
		pdata_dict["pos"] = sm_pos
		pdata_dict["velocity"] = (sm_pos - prev_pos) / dt_sec
		players[key] = pdata_dict


func _apply_ball_smoothing(snapshot: Dictionary, ball: Dictionary, dt_sec: float) -> void:
	var players: Dictionary = snapshot.get("players", {})
	var raw_ball: Vector2 = ball.get("pos", Vector2.ZERO)
	var target_ball: Vector2 = raw_ball

	var owner_id: int = int(ball.get("owner_id", -1))  # SSOT: track_id(-1 or 0..21)
	if owner_id >= 0 and owner_id <= 21 and players.has(str(owner_id)):
		var owner_p: Variant = players[str(owner_id)]
		if owner_p is Dictionary:
			target_ball = (owner_p as Dictionary).get("pos", raw_ball)

	var prev_ball: Vector2 = raw_ball
	if _has_ball:
		prev_ball = _smooth_ball
	else:
		_has_ball = true

	var sm_ball: Vector2 = prev_ball.lerp(target_ball, ball_alpha)
	_smooth_ball = sm_ball
	ball["pos"] = sm_ball
	ball["vel"] = (sm_ball - prev_ball) / dt_sec


func _apply_separation(players: Dictionary) -> void:
	var tids: Array[int] = []
	for key: Variant in players.keys():
		var tid: int = _to_tid(key)
		if tid >= 0 and tid <= 21:
			tids.append(tid)

	for i: int in range(tids.size()):
		for j: int in range(i + 1, tids.size()):
			var ti: int = tids[i]
			var tj: int = tids[j]
			var ki: String = str(ti)
			var kj: String = str(tj)
			if not players.has(ki) or not players.has(kj):
				continue

			var pi_v: Variant = players[ki]
			var pj_v: Variant = players[kj]
			if not (pi_v is Dictionary) or not (pj_v is Dictionary):
				continue

			var pi: Dictionary = pi_v as Dictionary
			var pj: Dictionary = pj_v as Dictionary
			var ai: Vector2 = pi.get("pos", Vector2.ZERO)
			var aj: Vector2 = pj.get("pos", Vector2.ZERO)

			var d: float = ai.distance_to(aj)
			if d <= 0.0001:
				continue
			if d < min_separation_m:
				var push_dir: Vector2 = (ai - aj).normalized()
				var push_amt: float = (min_separation_m - d) * separation_push
				ai += push_dir * push_amt
				aj -= push_dir * push_amt
				pi["pos"] = ai
				pj["pos"] = aj
				players[ki] = pi
				players[kj] = pj
				_smooth_players[ti] = ai
				_smooth_players[tj] = aj


static func _to_tid(key: Variant) -> int:
	if key is int:
		return int(key)
	var s := str(key)
	if s.is_valid_int():
		return int(s)
	return -1
