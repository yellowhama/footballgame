extends RefCounted
class_name PositionSnapshotAdapter

# Preload to avoid class_name resolution order issues
const _MatchEventKeys = preload("res://scripts/constants/MatchEventKeys.gd")

## 표준 스냅샷 포맷 (StandardSnapshot)
## {
##   "t_ms": int,
##   "score": { "home": int, "away": int },  # 2025-12-07 추가
##   "ball": { "pos": Vector2, "z": float, "vel": Vector2?, "owner_id": String },
##   "players": {
##     track_id: {
##       "pos": Vector2,
##       "team_id": int,
##       "role": String,
##       "name": String,
##       "number": int,
##       "action": String,
##       "stamina": float
##     }
##   },
##   "events": Array
## }


static func _extract_vec2(value: Variant) -> Vector2:
	if value is Vector2:
		return value
	if value is Array and value.size() >= 2:
		return Vector2(float(value[0]), float(value[1]))
	if value is Dictionary:
		return Vector2(float(value.get("x", 0.0)), float(value.get("y", 0.0)))
	return Vector2.ZERO

static func _copy_optional_keys(src: Dictionary, dst: Dictionary, keys: Array) -> void:
	for key in keys:
		if src.has(key):
			dst[key] = src[key]


## ✅ P1-A (2025-12-22 FIX_2512): Convert roster_id to track_id
## roster_id formats:
##   - "H0" to "H10" (home players) → track_id 0-10
##   - "A0" to "A10" (away players) → track_id 11-21
##   - Integer string "5" → interpret based on team_id
##   - Empty "" → return -1
static func _resolve_track_id_from_roster_id(roster_id: String, team_id: int) -> int:
	if roster_id.is_empty():
		return -1

	# If it's already a pure integer, interpret based on team_id
	if roster_id.is_valid_int():
		var index := int(roster_id)
		if index >= 0 and index <= 10:
			return index if team_id == 0 else (index + 11)
		# If index > 10, it might already be a track_id
		if index >= 0 and index <= 21:
			return index
		return -1

	# Parse roster format "H5" or "A3"
	if roster_id.length() < 2:
		return -1

	var prefix := roster_id[0].to_upper()
	var index_str := roster_id.substr(1)

	if not index_str.is_valid_int():
		return -1

	var index := int(index_str)

	# Home player: H0-H10 → track_id 0-10
	if prefix == "H":
		if index >= 0 and index <= 10:
			return index
		return -1

	# Away player: A0-A10 → track_id 11-21
	if prefix == "A":
		if index >= 0 and index <= 10:
			return index + 11
		return -1

	return -1


## 기존 배치용 어댑터 (호환 유지) - 표준 스냅샷 이전 레거시 포맷
static func from_batch_base(base: Dictionary) -> Dictionary:
	var result: Dictionary = {}

	# Ball (with z=0 default for batch mode)
	if base.has("ball"):
		var ball_vec: Vector2 = _extract_vec2(base.get("ball"))
		result["ball"] = {
			"x": ball_vec.x,
			"y": ball_vec.y,
			"z": 0.0,
		}

	# Players
	if base.has("players") and base.players is Dictionary:
		var src_players: Dictionary = base.players
		var out_players: Dictionary = {}
		for key in src_players.keys():
			var entry: Variant = src_players[key]
			var pos_vec := Vector2.ZERO
			var state: String = ""

			if entry is Dictionary:
				var entry_dict: Dictionary = entry
				pos_vec = _extract_vec2(entry_dict.get("position", Vector2.ZERO))
				if entry_dict.has("state"):
					state = str(entry_dict.get("state"))
			else:
				pos_vec = _extract_vec2(entry)

			var player_out := {
				"position":
				{
					"x": pos_vec.x,
					"y": pos_vec.y,
				}
			}
			if state != "":
				player_out["state"] = state
			out_players[str(key)] = player_out

		if not out_players.is_empty():
			result["players"] = out_players

	return result


## 기존 세션용 어댑터 (호환 유지) - 표준 스냅샷 이전 레거시 포맷
static func from_step_raw(raw: Dictionary) -> Dictionary:
	if raw.is_empty():
		return {}

	var adapted: Dictionary = {}

	if raw.has("ball"):
		adapted["ball"] = raw.get("ball")

	var players_variant: Variant = raw.get("players", {})
	if players_variant is Dictionary:
		var src_players: Dictionary = players_variant
		var adapted_players: Dictionary = {}
		for key in src_players.keys():
			var src_entry: Variant = src_players[key]
			if not (src_entry is Dictionary):
				continue
			var src_dict: Dictionary = src_entry
			var pos_dict := {
				"x": float(src_dict.get("x", 0.0)),
				"y": float(src_dict.get("y", 0.0)),
			}
			adapted_players[key] = {"position": pos_dict}
		if not adapted_players.is_empty():
			adapted["players"] = adapted_players

	return adapted


# ============================================================================
# 신규: 표준 스냅샷 생성기 (Batch / Session 공통)
# ============================================================================

## 디버그 로깅 (처음 한 번만)
static var _debug_logged: bool = false
static var _fix02_predict_clamp_warned: bool = false


static func from_batch_data(position_data: Dictionary, rosters: Dictionary, time_ms: int) -> Dictionary:
	# position_data: { ball:[{timestamp,position(z?)}], players:{id:[{timestamp,position}]} }
	if position_data.is_empty():
		return {}

	## 디버그: 첫 호출 시 데이터 구조 로깅
	if not _debug_logged:
		_debug_logged = true
		var player_ids := []
		if position_data.has("players") and position_data.players is Dictionary:
			player_ids = position_data.players.keys()
		print("[PositionSnapshotAdapter] First call - position_data player KEYS: %s" % str(player_ids))

		## 로스터 구조 로깅
		if rosters.is_empty():
			print("[PositionSnapshotAdapter] WARNING: rosters is EMPTY!")
		else:
			for team_key in ["home", "away"]:
				if rosters.has(team_key):
					var team_data = rosters[team_key]
					var roster_ids := []
					var players: Array = []
					if team_data is Dictionary:
						players = team_data.get("players", team_data.get("roster", []))
					elif team_data is Array:
						players = team_data
					for p in players:
						if p is Dictionary:
							roster_ids.append(str(p.get("id", p.get("player_id", "?"))))
					print(
						(
							"[PositionSnapshotAdapter] %s roster IDs (first 11): %s"
							% [team_key, str(roster_ids.slice(0, 11))]
						)
					)

	var snapshot: Dictionary = {"t_ms": time_ms, "ball": {"pos": Vector2.ZERO, "z": 0.0}, "players": {}, "events": []}

	# Ball
	if position_data.has("ball") and position_data.ball is Array:
		var samples: Array = position_data.ball
		var frame := _find_frame(samples, time_ms)
		if not frame.is_empty():
			## frame 구조: { "t": float, "x": float, "y": float, "z": float, "vx": float, "vy": float }
			## 2025-12-11 P2: ball velocity 추출 추가
			## _extract_vec2는 {"x": x, "y": y} Dictionary를 처리할 수 있음
			var pos = _extract_vec2(frame)
			var z = float(frame.get("z", frame.get("height", 0.0)))  # FIX_2601: Rust serializes as "height"
			var ball_vx: float = float(frame.get("vx", 0.0))
			var ball_vy: float = float(frame.get("vy", 0.0))
			var ball_vel := Vector2(ball_vx, ball_vy)
			snapshot["ball"] = {
				"pos": pos,
				"z": z,
				"vel": ball_vel,
				# SSOT: track_id (-1 or 0..21). Timeline batches may not include owner_id -> stays -1.
				"owner_id": int(frame.get("owner_id", -1))
			}

	# Players
	if position_data.has("players") and position_data.players is Dictionary:
		# ✅ SSOT: key = track_id (0..21)
		for track_id in position_data.players.keys():
			var track: Variant = position_data.players[track_id]
			if not (track is Array):
				continue
			var frame := _find_frame(track, time_ms)
			if frame.is_empty():
				continue
			## frame 구조: { "t": float, "x": float, "y": float, "vx": float, "vy": float }
			## 2025-12-11: velocity 추출 추가
			var pos = _extract_vec2(frame)
			var vx: float = float(frame.get("vx", 0.0))
			var vy: float = float(frame.get("vy", 0.0))
			var vel := Vector2(vx, vy)
			var meta = _get_player_meta(track_id, rosters)
			meta["pos"] = pos
			meta["velocity"] = vel
			meta["stamina"] = float(frame.get("stamina", 1.0))  # ✅ Gap #5 Fix: Preserve from tick_position_data
			if frame.has("state"):
				meta["action"] = str(frame.get("state", "idle"))
			elif meta.get("action", "") == "":
				var speed := vel.length()
				if speed < 0.1:
					meta["action"] = "idle"
				elif speed < 3.0:
					meta["action"] = "walk"
				elif speed < 6.0:
					meta["action"] = "run"
				else:
					meta["action"] = "sprint"
			snapshot["players"][str(track_id)] = meta

	return snapshot


static func from_step(step_result: Dictionary, current_rosters: Dictionary) -> Dictionary:
	if step_result.is_empty():
		return {}

	var t_ms := int(step_result.get("timestamp_ms", step_result.get("t_ms", 0)))
	var snap_raw: Dictionary = step_result.get("snapshot", step_result.get("snap", {}))

	var snapshot: Dictionary = {
		"t_ms": t_ms,
		"score":
		{
			"home": int(snap_raw.get("score_home", step_result.get("score_home", 0))),
			"away": int(snap_raw.get("score_away", step_result.get("score_away", 0)))
		},
		"ball": {"pos": Vector2.ZERO, "z": 0.0},
		"players": {},
		"events": step_result.get("events", [])
	}

	if step_result.has("team_view_simple"):
		snapshot["team_view_simple"] = step_result.get("team_view_simple")
	if step_result.has("team_view_minimap"):
		snapshot["team_view_minimap"] = step_result.get("team_view_minimap")

	_copy_optional_keys(
		snap_raw,
		snapshot,
		[
			"decision_intents",
			"pressure_against_home",
			"pressure_against_away",
			"occupancy_total",
			"xgzone",
			"offside_lines"
		]
	)

	# Ball
	if snap_raw.has("ball"):
		var b = snap_raw["ball"]
		if b is Dictionary:
			var pos = _extract_vec2(b)
			var z = float(b.get("z", 0.0))
			var owner_id = int(b.get("owner_id", -1))  # -1 = loose ball
			snapshot["ball"] = {"pos": pos, "z": z, "owner_id": owner_id}
		else:
			var pos = _extract_vec2(b)
			snapshot["ball"] = {"pos": pos, "z": 0.0, "owner_id": -1}

	# Players
	if snap_raw.has("players") and snap_raw.players is Dictionary:
		# ✅ SSOT: key = track_id (0..21)
		for track_id in snap_raw.players.keys():
			var p = snap_raw.players[track_id]
			if not (p is Dictionary):
				continue
			var pos = Vector2(float(p.get("x", 0.0)), float(p.get("y", 0.0)))
			var vx: float = float(p.get("vx", 0.0))
			var vy: float = float(p.get("vy", 0.0))
			var vel := Vector2(vx, vy)
			var stamina = float(p.get("stamina", 1.0))
			var state = str(p.get("state", ""))
			var meta = _get_player_meta(track_id, current_rosters)
			meta["pos"] = pos
			meta["velocity"] = vel  # Phase 2B: Session velocity integration
			meta["stamina"] = stamina
			meta["action"] = state  # P2: player action/state for Hero Time UX
			snapshot["players"][str(track_id)] = meta

	return snapshot


# === 내부 유틸 ===


# 가장 가까운 timestamp 프레임을 선택 + 선형 보간 적용
# 2025-12-17: 부드러운 움직임을 위해 보간 로직 추가
# 2026-01-13: Dead Reckoning 추가 (Uber Realtime Architecture)
static func _find_frame(track: Array, time_ms: int) -> Dictionary:
	if track.is_empty():
		return {}

	var time_sec: float = time_ms / 1000.0

	# 단일 프레임만 있으면 보간 불가
	if track.size() == 1:
		return track[0]

	# 두 프레임 찾기: before (time_sec 이하), after (time_sec 이상)
	var before: Dictionary = {}
	var after: Dictionary = {}
	var before_t: float = -INF
	var after_t: float = INF

	for entry in track:
		if not (entry is Dictionary):
			continue
		var t: float = float(entry.get("t", entry.get("timestamp", 0.0)))

		# time_sec 이하 중 가장 큰 값 (before)
		if t <= time_sec and t > before_t:
			before_t = t
			before = entry

		# time_sec 이상 중 가장 작은 값 (after)
		if t >= time_sec and t < after_t:
			after_t = t
			after = entry

	# Dead Reckoning: time_sec이 모든 프레임보다 미래인 경우
	# 마지막 프레임의 속도를 기반으로 위치 예측
	if after.is_empty() and not before.is_empty():
		var dt_sec = time_sec - before_t
		# 최대 500ms까지만 예측 (너무 긴 예측은 부정확)
		if dt_sec > 0.0 and dt_sec < 0.5:
			return _predict_position(before, dt_sec)
		# 500ms 초과면 그냥 마지막 프레임 반환
		return before

	# 보간 불가능한 경우: 가장 가까운 프레임 반환
	if before.is_empty() or after.is_empty():
		if not before.is_empty():
			return before
		if not after.is_empty():
			return after
		return track[0]

	# 같은 프레임이면 보간 불필요
	if before_t == after_t:
		return before

	# 선형 보간 (Linear Interpolation)
	var alpha: float = (time_sec - before_t) / (after_t - before_t)
	alpha = clamp(alpha, 0.0, 1.0)

	var interpolated := {}

	# 시간 정보
	interpolated["t"] = time_sec

	# 위치 보간 (x, y)
	var before_x := float(before.get("x", 0.0))
	var before_y := float(before.get("y", 0.0))
	var after_x := float(after.get("x", 0.0))
	var after_y := float(after.get("y", 0.0))

	interpolated["x"] = lerp(before_x, after_x, alpha)
	interpolated["y"] = lerp(before_y, after_y, alpha)

	# z 좌표 보간 (볼 높이)
	if before.has("z") or after.has("z"):
		var before_z := float(before.get("z", 0.0))
		var after_z := float(after.get("z", 0.0))
		interpolated["z"] = lerp(before_z, after_z, alpha)

	# 속도 보간 (vx, vy) - 선택적
	if before.has("vx") or after.has("vx"):
		var before_vx := float(before.get("vx", 0.0))
		var after_vx := float(after.get("vx", 0.0))
		interpolated["vx"] = lerp(before_vx, after_vx, alpha)

	if before.has("vy") or after.has("vy"):
		var before_vy := float(before.get("vy", 0.0))
		var after_vy := float(after.get("vy", 0.0))
		interpolated["vy"] = lerp(before_vy, after_vy, alpha)

	# 상태는 보간 불가 - before 프레임 사용
	if before.has("state"):
		interpolated["state"] = before.get("state")

	return interpolated


## Dead Reckoning: 속도 기반 위치 예측 (Uber Realtime Architecture)
## 마지막 프레임의 속도를 사용하여 미래 위치를 예측
## Reference: docs/specs/fix_2601/0113/UBER_REALTIME_ARCHITECTURE.md
static func _predict_position(last_frame: Dictionary, dt_sec: float) -> Dictionary:
	var predicted := last_frame.duplicate()

	var vx := float(last_frame.get("vx", 0.0))
	var vy := float(last_frame.get("vy", 0.0))

	var new_x := float(last_frame.get("x", 0.0)) + vx * dt_sec
	var new_y := float(last_frame.get("y", 0.0)) + vy * dt_sec

	# FIX02: 기본 계약은 meters(0..105, 0..68).
	# Dead Reckoning은 예측 오차가 발생할 수 있으므로 clamp는 유지하되, "조용한 clamp"는 금지 → 경고 1회 남김.
	var x_clamped: float = clampf(new_x, 0.0, float(FieldSpec.FIELD_LENGTH_M))
	var y_clamped: float = clampf(new_y, 0.0, float(FieldSpec.FIELD_WIDTH_M))
	if (x_clamped != new_x or y_clamped != new_y) and not _fix02_predict_clamp_warned:
		_fix02_predict_clamp_warned = true
		push_warning(
			(
				"[FIX02][COORD] Dead reckoning predicted out-of-bounds; clamping once. "
				+ "pred=(%.3f, %.3f) -> (%.3f, %.3f)"
			)
			% [new_x, new_y, x_clamped, y_clamped]
		)
	predicted["x"] = x_clamped
	predicted["y"] = y_clamped

	# 시간 업데이트
	var base_t := float(last_frame.get("t", 0.0))
	predicted["t"] = base_t + dt_sec

	# 속도는 유지 (등속 운동 가정)
	predicted["vx"] = vx
	predicted["vy"] = vy

	# z 좌표 예측 (공의 경우)
	if last_frame.has("z"):
		var z := float(last_frame.get("z", 0.0))
		# 단순 감속 모델 (중력 효과는 생략 - 짧은 예측이므로)
		if z > 0.0:
			predicted["z"] = max(0.0, z - 9.8 * dt_sec * dt_sec * 0.5)
		else:
			predicted["z"] = 0.0

	return predicted


static func _get_player_meta(track_id: Variant, rosters: Dictionary) -> Dictionary:
	## Phase 4: Improved GK metadata mapping (2025-12-22 FIX_2512)
	## ✅ SSOT: track_id (0-21) = roster index
	##   - 0-10: home team (0 = home GK)
	##   - 11-21: away team (11 = away GK)

	var track_id_int := int(track_id) if track_id is int or track_id is float else -1
	if track_id_int < 0 or track_id_int > 21:
		# Invalid track_id, return empty meta
		return {
			"team_id": -1, "role": "", "name": "", "number": 0, "action": "", "stamina": 1.0, "track_id": track_id_int
		}

	var team = "home" if track_id_int < 11 else "away"
	var local_idx = track_id_int if track_id_int < 11 else track_id_int - 11

	# Try to get roster data
	var team_data = rosters.get(team, [])
	var roster = []

	if team_data is Dictionary:
		roster = team_data.get("players", team_data.get("roster", []))
	elif team_data is Array:
		roster = team_data

	# Get player metadata from roster
	if local_idx < roster.size():
		var player = roster[local_idx]
		if player is Dictionary:
			return {
				"team_id": 0 if team == "home" else 1,
				"role": str(player.get("position", player.get("role", ""))),
				"name": str(player.get("name", player.get("pseudo_name", player.get("player_name", "")))),
				"number":
				int(player.get("jersey_number", player.get("kit_number", player.get("number", local_idx + 1)))),
				"track_id": track_id_int,
				"action": "",
				"stamina": 1.0
			}

	# Fallback with GK detection
	return {
		"team_id": 0 if team == "home" else 1,
		"role": "GK" if local_idx == 0 else "FW",  # track_id 0, 11 = GK
		"name": "Player %d" % track_id_int,
		"number": local_idx + 1,
		"track_id": track_id_int,
		"action": "",
		"stamina": 1.0
	}


# ============================================================================
# Step 6: 이벤트 → 카메라/SFX 훅 헬퍼
# ============================================================================


## 이벤트 타입별 카메라/SFX 트리거 정보를 반환
## 뷰어에서 snapshot.events를 순회하며 이 헬퍼로 연출 결정
static func get_event_hooks(event: Dictionary) -> Dictionary:
	var event_type := str(event.get("type", event.get("kind", ""))).to_lower()
	var hooks := {
		"camera_action": "", "camera_target": null, "sfx": "", "overlay": "", "highlight": false, "pause_ms": 0  # zoom_in, follow_ball, pan_to_goal, etc.  # Vector2 or player_id  # goal_cheer, whistle, card_shown, etc.  # goal, halftime, fulltime, offside, card  # 하이라이트 강조 여부  # 이벤트 후 일시정지 시간
	}

	match event_type:
		"goal":
			hooks["camera_action"] = "zoom_goal"
			hooks["camera_target"] = _extract_vec2(event.get("pos", Vector2.ZERO))
			hooks["sfx"] = "goal_cheer"
			hooks["overlay"] = "goal"
			hooks["highlight"] = true
			hooks["pause_ms"] = 2000
		"shot", "shot_on_target":
			hooks["camera_action"] = "follow_ball"
			hooks["sfx"] = "kick"
		"save":
			hooks["camera_action"] = "zoom_keeper"
			hooks["sfx"] = "save"
			hooks["highlight"] = true
		"foul":
			hooks["camera_action"] = "pan_to_foul"
			hooks["camera_target"] = _extract_vec2(event.get("pos", Vector2.ZERO))
			hooks["sfx"] = "whistle"
		"yellow_card", "card_yellow":
			hooks["sfx"] = "card_shown"
			hooks["overlay"] = "yellow_card"
			hooks["pause_ms"] = 1000
		"red_card", "card_red":
			hooks["sfx"] = "card_shown"
			hooks["overlay"] = "red_card"
			hooks["pause_ms"] = 1500
		"offside":
			hooks["sfx"] = "whistle"
			hooks["overlay"] = "offside"
		"halftime":
			hooks["overlay"] = "halftime"
			hooks["sfx"] = "whistle_long"
			hooks["pause_ms"] = 3000
		"fulltime":
			hooks["overlay"] = "fulltime"
			hooks["sfx"] = "whistle_long"
			hooks["pause_ms"] = 3000
		"substitution":
			hooks["sfx"] = "sub_board"
			hooks["pause_ms"] = 500
		"corner", "corner_kick":
			hooks["camera_action"] = "pan_to_corner"
			hooks["sfx"] = "whistle"
		"penalty":
			hooks["camera_action"] = "zoom_penalty"
			hooks["sfx"] = "whistle"
			hooks["highlight"] = true
		"freekick", "free_kick":
			hooks["camera_action"] = "pan_to_foul"
			hooks["sfx"] = "whistle"

	return hooks


## 이벤트 배열에서 특정 타입만 필터링
static func filter_events_by_type(events: Array, types: Array) -> Array:
	var result := []
	for ev in events:
		if not (ev is Dictionary):
			continue
		var ev_type := str(ev.get("type", ev.get("kind", ""))).to_lower()
		if ev_type in types:
			result.append(ev)
	return result


## 이벤트가 하이라이트 대상인지 확인
static func is_highlight_event(event: Dictionary) -> bool:
	var hooks := get_event_hooks(event)
	return hooks.get("highlight", false)


# ============================================================================
# Step 7: 통합 이벤트 스키마 및 정규화
# ============================================================================

## 정규화된 이벤트 포맷 (NormalizedEvent):
## {
##   "type": String,         # 소문자 snake_case (goal, shot, foul, etc.)
##   "t_ms": int,            # 밀리초 타임스탬프
##   "minute": int,          # 분 (0-90+)
##   "team_id": int,         # 0=home, 1=away, -1=unknown
##   "player": String,       # 선수 이름 또는 ID
##   "pos": Vector2,         # 이벤트 발생 위치
##   "details": Dictionary   # 이벤트별 추가 정보 (assist_by, xg_value, etc.)
## }


## 단일 이벤트 정규화 (Session/Batch 공통)
static func normalize_event(raw_event: Dictionary, _rosters: Dictionary = {}) -> Dictionary:
	if raw_event.is_empty():
		return {}

	# === 타입 추출 ===
	var event_type: String = str(raw_event.get("type", raw_event.get("kind", raw_event.get("etype", "")))).to_lower()

	# === 타임스탬프 추출 ===
	var t_ms: int = 0
	var minute: int = 0

	# Session format: timestamp_ms, t_ms
	if raw_event.has("timestamp_ms"):
		t_ms = int(raw_event.get("timestamp_ms", 0))
		minute = int(float(t_ms) / 60000.0)
	elif raw_event.has("t_ms"):
		t_ms = int(raw_event.get("t_ms", 0))
		minute = int(float(t_ms) / 60000.0)
	# Batch format: base.t (seconds), t (seconds/minutes), time, minute
	elif raw_event.has("base") and raw_event.base is Dictionary:
		var base: Dictionary = raw_event.base
		var t_sec: float = float(base.get("t", 0.0))
		t_ms = int(t_sec * 1000.0)
		minute = int(t_sec / 60.0)
	elif raw_event.has("t"):
		var t_val = raw_event.get("t")
		if t_val is float or (t_val is int and t_val < 200):
			# t는 분 단위일 가능성 (< 200이면 분으로 간주)
			minute = int(t_val)
			t_ms = minute * 60000
		else:
			# 초 단위
			t_ms = int(float(t_val) * 1000.0)
			minute = int(float(t_ms) / 60000.0)
	elif raw_event.has("time"):
		var time_sec: float = float(raw_event.get("time", 0.0))
		t_ms = int(time_sec * 1000.0)
		minute = int(time_sec / 60.0)
	elif raw_event.has("minute"):
		minute = int(raw_event.get("minute", 0))
		t_ms = minute * 60000

	# === 팀 ID 추출 ===
	var team_id: int = -1

	# Session format: is_home_team, team
	if raw_event.has("is_home_team"):
		team_id = 0 if raw_event.get("is_home_team", false) else 1
	elif raw_event.has("team"):
		var team_str := str(raw_event.get("team")).to_lower()
		if team_str == "home":
			team_id = 0
		elif team_str == "away":
			team_id = 1
	# Batch format: base.team_id, team_id
	elif raw_event.has("base") and raw_event.base is Dictionary:
		team_id = int(raw_event.base.get("team_id", -1))
	elif raw_event.has("team_id"):
		team_id = int(raw_event.get("team_id", -1))

	# === 선수 이름 추출 ===
	var player_name: String = ""
	if raw_event.has("player"):
		player_name = str(raw_event.get("player", ""))
	elif raw_event.has("player_name"):
		player_name = str(raw_event.get("player_name", ""))
	elif raw_event.has("base") and raw_event.base is Dictionary:
		player_name = str(raw_event.base.get("player", ""))

	# === Event SSOT: track_id 추출 (값 없으면 -1) ===
	var player_track_id: int = -1

	# Try 1: Direct player_track_id (SSOT)
	if raw_event.has("player_track_id"):
		player_track_id = int(raw_event.get("player_track_id", -1))
	elif raw_event.has("base") and raw_event.base is Dictionary:
		if raw_event.base.has("player_track_id"):
			player_track_id = int(raw_event.base.get("player_track_id", -1))

	# ✅ P1-A (2025-12-22 FIX_2512): Try 2 - Fallback resolution from player_id + team_id
	if player_track_id < 0:
		var pid: String = ""
		if raw_event.has("base") and raw_event.base is Dictionary:
			pid = str(raw_event.base.get("player_id", ""))
		elif raw_event.has("player_id"):
			pid = str(raw_event.get("player_id", ""))

		if pid != "" and team_id >= 0:
			player_track_id = _resolve_track_id_from_roster_id(pid, team_id)

	var target_track_id: int = -1
	if raw_event.has("target_track_id"):
		target_track_id = int(raw_event.get("target_track_id", -1))
	elif raw_event.has("base") and raw_event.base is Dictionary:
		if raw_event.base.has("target_track_id"):
			target_track_id = int(raw_event.base.get("target_track_id", -1))

	# === 위치 추출 ===
	var pos := Vector2.ZERO
	if raw_event.has("pos"):
		pos = _extract_vec2(raw_event.get("pos"))
	elif raw_event.has("ball_position"):
		pos = _extract_vec2(raw_event.get("ball_position"))
	elif raw_event.has("details") and raw_event.details is Dictionary:
		if raw_event.details.has("ball_position"):
			pos = _extract_vec2(raw_event.details.get("ball_position"))
	elif raw_event.has("base") and raw_event.base is Dictionary:
		if raw_event.base.has("pos"):
			pos = _extract_vec2(raw_event.base.get("pos"))

	# === 상세 정보 추출 ===
	var details: Dictionary = {}
	if raw_event.has("details") and raw_event.details is Dictionary:
		details = raw_event.details.duplicate()

	# Session format 추가 필드
	if raw_event.has("assist_by"):
		details["assist_by"] = str(raw_event.get("assist_by"))
	if raw_event.has("xg_value"):
		details["xg_value"] = float(raw_event.get("xg_value", 0.0))
	if raw_event.has("replaced_player"):
		details["replaced_player"] = str(raw_event.get("replaced_player"))

	return {
		_MatchEventKeys.TYPE: event_type,
		_MatchEventKeys.T_MS: t_ms,
		_MatchEventKeys.PLAYER_TRACK_ID: player_track_id,
		_MatchEventKeys.TARGET_TRACK_ID: target_track_id,
		"minute": minute,
		"team_id": team_id,
		_MatchEventKeys.PLAYER_NAME: player_name,
		"pos": pos,
		"details": details
	}


## 이벤트 배열 정규화
static func normalize_events(raw_events: Array, rosters: Dictionary = {}) -> Array:
	var normalized: Array = []
	for raw in raw_events:
		if not (raw is Dictionary):
			continue
		var norm := normalize_event(raw, rosters)
		if not norm.is_empty() and norm.get("type", "") != "":
			normalized.append(norm)
	return normalized


## from_batch_data에서 events 정규화 포함 버전
static func from_batch_data_with_events(
	position_data: Dictionary, rosters: Dictionary, raw_events: Array, time_ms: int
) -> Dictionary:
	var snapshot := from_batch_data(position_data, rosters, time_ms)
	if snapshot.is_empty():
		return snapshot

	# 현재 시간 ±500ms 범위의 이벤트만 포함
	var events_in_range: Array = []
	for raw_ev in raw_events:
		var norm := normalize_event(raw_ev, rosters)
		if norm.is_empty():
			continue
		var ev_t_ms: int = norm.get("t_ms", 0)
		if abs(ev_t_ms - time_ms) <= 500:
			events_in_range.append(norm)

	snapshot["events"] = events_in_range
	return snapshot


## from_step에서 events 정규화 적용
static func from_step_normalized(step_result: Dictionary, current_rosters: Dictionary) -> Dictionary:
	var snapshot := from_step(step_result, current_rosters)
	if snapshot.is_empty():
		return snapshot

	# events 정규화
	var raw_events: Array = step_result.get("events", [])
	snapshot["events"] = normalize_events(raw_events, current_rosters)
	return snapshot


# ============================================================================
# Packed Format Support (Performance Optimization)
# ============================================================================


## PackedFloat32Array에서 표준 스냅샷으로 변환
## step_session_packed() 결과를 from_step()과 동일한 StandardSnapshot으로 변환
##
## Packed format:
##   snapshot.players_packed: PackedFloat32Array [x0,y0,x1,y1,...,x21,y21]
##   player_index i: x = players_packed[i*2], y = players_packed[i*2+1]
##   0-10 = home, 11-21 = away
static func from_step_packed(step_result: Dictionary, current_rosters: Dictionary) -> Dictionary:
	if step_result.is_empty():
		return {}

	var t_ms := int(step_result.get("t_ms", 0))
	var snap_raw: Dictionary = step_result.get("snapshot", {})

	var snapshot: Dictionary = {
		"t_ms": t_ms,
		"score": {"home": int(snap_raw.get("score_home", 0)), "away": int(snap_raw.get("score_away", 0))},
		"ball": {"pos": Vector2.ZERO, "z": 0.0},
		"players": {},
		"events": step_result.get("events", [])
	}

	if step_result.has("team_view_simple"):
		snapshot["team_view_simple"] = step_result.get("team_view_simple")
	if step_result.has("team_view_minimap"):
		snapshot["team_view_minimap"] = step_result.get("team_view_minimap")

	_copy_optional_keys(
		snap_raw,
		snapshot,
		[
			"decision_intents",
			"pressure_against_home",
			"pressure_against_away",
			"occupancy_total",
			"xgzone",
			"offside_lines"
		]
	)

	# Ball
	if snap_raw.has("ball"):
		var b = snap_raw["ball"]
		if b is Dictionary:
			var pos = _extract_vec2(b)
			var z = float(b.get("z", 0.0))
			var owner_id = int(b.get("owner_id", -1))  # -1 = loose ball
			snapshot["ball"] = {"pos": pos, "z": z, "owner_id": owner_id}

	# Players from PackedFloat32Array
	if snap_raw.has("players_packed"):
		var packed: PackedFloat32Array = snap_raw["players_packed"]
		var stamina_arr: PackedFloat32Array = snap_raw.get("stamina_packed", PackedFloat32Array())

		# Format detection: 88 floats = new format (x,y,vx,vy), 44 floats = old format (x,y)
		var has_velocity := packed.size() >= 88
		var stride := 4 if has_velocity else 2
		var min_size := 88 if has_velocity else 44

		if packed.size() >= min_size:
			for i in range(min(22, packed.size() / stride)):
				var base := i * stride
				var pos := Vector2(packed[base], packed[base + 1])
				# Skip zero positions (uninitialized players)
				if pos.x == 0.0 and pos.y == 0.0:
					continue

				var vel := Vector2.ZERO
				if has_velocity:
					vel = Vector2(packed[base + 2], packed[base + 3])

				var stamina := 1.0
				if stamina_arr.size() > i:
					stamina = stamina_arr[i]
				var meta := _get_player_meta(i, current_rosters)
				meta["pos"] = pos
				meta["velocity"] = vel  # Phase 2B: Session velocity integration
				meta["stamina"] = stamina
				# Extract action from states
				meta["action"] = ""
				if snap_raw.has("states") and snap_raw["states"] is Dictionary:
					meta["action"] = str(snap_raw["states"].get(str(i), ""))
				snapshot["players"][str(i)] = meta

	return snapshot


## Packed format + events 정규화
static func from_step_packed_normalized(step_result: Dictionary, current_rosters: Dictionary) -> Dictionary:
	var snapshot := from_step_packed(step_result, current_rosters)
	if snapshot.is_empty():
		return snapshot

	var raw_events: Array = step_result.get("events", [])
	snapshot["events"] = normalize_events(raw_events, current_rosters)
	return snapshot


# ============================================================================
# FIX_2512 Phase 3: ReplayV2 Support (2025-12-25)
# ============================================================================


## ReplayV2 SaveFrameV2 → StandardSnapshot 변환
##
## Input (SaveFrameV2 from JSON):
## {
##   "t_ms": 5000,
##   "entities": [
##     {"x10": 525, "y10": 340, "vx10": 72, "vy10": 0, "state": 1, "flags": 0, "wx10": 0, "wy10": 0},
##     ... (23개 - entities[0] = ball, entities[1..22] = players)
##   ]
## }
##
## Output (StandardSnapshot):
## {
##   "t_ms": 5000,
##   "ball": {
##     "pos": Vector2(52.5, 34.0),
##     "vel": Vector2(7.2, 0.0),
##     "z": 0.0,
##     "owner_id": -1,  # flags에서 추출
##     "state": 1
##   },
##   "players": {
##     "0": {"pos": Vector2(...), "vel": Vector2(...), "action": "...", ...},
##     ...
##     "21": {"pos": Vector2(...), ...}
##   },
##   "events": []
## }
static func from_replay_v2_frame(frame: Dictionary, rosters: Dictionary = {}) -> Dictionary:
	if frame.is_empty() or not frame.has("entities"):
		return {}

	var t_ms: int = int(frame.get("t_ms", 0))
	var entities: Array = frame.get("entities", [])

	if entities.size() != 23:
		push_error("[PositionSnapshotAdapter] Invalid ReplayV2 frame: expected 23 entities, got %d" % entities.size())
		return {}

	var snapshot: Dictionary = {"t_ms": t_ms, "ball": {"pos": Vector2.ZERO, "z": 0.0}, "players": {}, "events": []}

	# entities[0] = ball
	var ball_entity = entities[0]
	snapshot["ball"] = _parse_entity_v2(ball_entity, true)

	# entities[1..22] = players (track_id 0-21)
	for i in range(1, 23):
		var track_id := i - 1  # 0-21
		var player_entity = entities[i]
		var player_dict = _parse_entity_v2(player_entity, false)

		# 메타데이터 추가 (roster 정보)
		var meta = _get_player_meta(track_id, rosters)
		meta["pos"] = player_dict["pos"]
		meta["velocity"] = player_dict["vel"]
		meta["stamina"] = 1.0  # ReplayV2에서는 stamina 없음, 기본값

		# state → action 변환
		var state_value = player_dict.get("state", 0)
		meta["action"] = _state_to_action(state_value, player_dict["vel"])

		snapshot["players"][str(track_id)] = meta

	return snapshot


## EntitySnapV2 → Entity Dictionary
## EntitySnapV2: {x10, y10, vx10, vy10, state, flags, wx10, wy10}
## Output: {pos, vel, state, flags, waypoint}
static func _parse_entity_v2(entity: Dictionary, is_ball: bool) -> Dictionary:
	if entity.is_empty():
		return {"pos": Vector2.ZERO, "vel": Vector2.ZERO, "state": 0, "flags": 0}

	# i16 → meters (0.1m units)
	var pos := Vector2(float(entity.get("x10", 0)) * 0.1, float(entity.get("y10", 0)) * 0.1)

	# i16 → m/s (0.1m/s units)
	var vel := Vector2(float(entity.get("vx10", 0)) * 0.1, float(entity.get("vy10", 0)) * 0.1)

	var state := int(entity.get("state", 0))
	var flags := int(entity.get("flags", 0))

	var result := {"pos": pos, "vel": vel, "state": state, "flags": flags}

	# Waypoint (디버그용, 옵션)
	if entity.has("wx10") and entity.has("wy10"):
		var wx := float(entity.get("wx10", 0)) * 0.1
		var wy := float(entity.get("wy10", 0)) * 0.1
		if wx != 0.0 or wy != 0.0:
			result["waypoint"] = Vector2(wx, wy)

	# Ball-specific: z = 0.0, owner_id from flags
	if is_ball:
		result["z"] = 0.0
		result["owner_id"] = -1 if flags == 0 else (flags - 1)  # flags: 0=loose, 1=track_id 0, 2=track_id 1, etc.

	return result


## State value → action string 변환
## state 값은 MatchEngine FSM 상태를 나타냄 (임시 매핑)
static func _state_to_action(state_value: int, vel: Vector2) -> String:
	# 속도 기반 기본 action 결정
	var speed := vel.length()

	# state 값에 따른 명시적 매핑 (추후 확장 가능)
	match state_value:
		0:
			# idle 상태
			return "idle" if speed < 0.1 else "walk"
		1:
			# 이동 상태
			if speed < 0.1:
				return "idle"
			elif speed < 3.0:
				return "walk"
			elif speed < 6.0:
				return "run"
			else:
				return "sprint"
		2:
			# 볼 소유 상태
			return "dribble" if speed > 1.0 else "control"
		3:
			# 패스/슛 준비
			return "kick"
		_:
			# 기본 속도 기반 판단
			if speed < 0.1:
				return "idle"
			elif speed < 3.0:
				return "walk"
			elif speed < 6.0:
				return "run"
			else:
				return "sprint"
