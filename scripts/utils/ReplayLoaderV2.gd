extends RefCounted
class_name ReplayLoaderV2

## ReplayV2 JSON 로더 및 유틸리티
##
## FIX_2512 Phase 3 - TASK_08
## 2025-12-25


## ReplayV2 JSON 파일 로드
##
## Returns: ReplayV2 Dictionary
## {
##   "version": 2,
##   "meta": {...},
##   "save_frames": [{t_ms, entities}, ...],
##   "events": [{t_ms, kind, a, b, x10, y10, aux}, ...]
## }
static func load_json(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		push_error("[ReplayLoaderV2] File not found: " + path)
		return {}

	var file = FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_error("[ReplayLoaderV2] Failed to open file: " + path)
		return {}

	var json_text = file.get_as_text()
	file.close()

	var json = JSON.new()
	var parse_result = json.parse(json_text)
	if parse_result != OK:
		push_error("[ReplayLoaderV2] Failed to parse JSON: error code " + str(parse_result))
		return {}

	var replay = json.get_data()

	# 버전 체크
	if not replay.has("version") or replay["version"] != 2:
		push_error("[ReplayLoaderV2] Invalid replay version: expected 2, got " + str(replay.get("version", "?")))
		return {}

	# 구조 검증
	if not replay.has("meta") or not replay.has("save_frames"):
		push_error("[ReplayLoaderV2] Invalid replay structure: missing meta or save_frames")
		return {}

	return replay


## 특정 시간의 프레임 가져오기 (이진 탐색)
##
## Arguments:
##   replay: ReplayV2 Dictionary
##   t_ms: 타임스탬프 (밀리초)
##
## Returns: SaveFrameV2 Dictionary or {} if not found
static func get_frame_at(replay: Dictionary, t_ms: int) -> Dictionary:
	if not replay.has("save_frames"):
		return {}

	var frames: Array = replay["save_frames"]

	if frames.is_empty():
		return {}

	# 단일 프레임만 있으면 그대로 반환
	if frames.size() == 1:
		return frames[0]

	# 이진 탐색
	var left := 0
	var right := frames.size() - 1

	while left <= right:
		var mid := (left + right) / 2
		var frame_t: int = int(frames[mid].get("t_ms", 0))

		if frame_t == t_ms:
			return frames[mid]
		elif frame_t < t_ms:
			left = mid + 1
		else:
			right = mid - 1

	# 정확한 매치 없음 - 가장 가까운 프레임 반환
	if right < 0:
		return frames[0]
	if left >= frames.size():
		return frames[-1]

	# left와 right 중 더 가까운 것 선택
	var left_t: int = int(frames[left].get("t_ms", 0))
	var right_t: int = int(frames[right].get("t_ms", 0))
	var left_diff := abs(left_t - t_ms)
	var right_diff := abs(right_t - t_ms)

	return frames[left] if left_diff < right_diff else frames[right]


## 전체 프레임 반복자
##
## Returns: Array of SaveFrameV2
static func iter_frames(replay: Dictionary) -> Array:
	if not replay.has("save_frames"):
		return []
	return replay.get("save_frames", [])


## 이벤트 필터링 (kind별)
##
## Arguments:
##   replay: ReplayV2 Dictionary
##   kind: 이벤트 타입 (0=goal, 1=pass, 2=shot, etc.)
##
## Returns: Array of ReplayEventV2
static func get_events_by_type(replay: Dictionary, kind: int) -> Array:
	if not replay.has("events"):
		return []

	var filtered := []
	var events: Array = replay.get("events", [])

	for event in events:
		if not (event is Dictionary):
			continue
		if int(event.get("kind", -1)) == kind:
			filtered.append(event)

	return filtered


## 시간 범위 내 이벤트 가져오기
##
## Arguments:
##   replay: ReplayV2 Dictionary
##   t_start_ms: 시작 시간 (밀리초)
##   t_end_ms: 종료 시간 (밀리초)
##
## Returns: Array of ReplayEventV2
static func get_events_in_range(replay: Dictionary, t_start_ms: int, t_end_ms: int) -> Array:
	if not replay.has("events"):
		return []

	var filtered := []
	var events: Array = replay.get("events", [])

	for event in events:
		if not (event is Dictionary):
			continue
		var event_t: int = int(event.get("t_ms", 0))
		if event_t >= t_start_ms and event_t <= t_end_ms:
			filtered.append(event)

	return filtered


## 메타 정보 출력
static func print_meta(replay: Dictionary):
	if not replay.has("meta"):
		print("[ReplayLoaderV2] No meta information")
		return

	var meta: Dictionary = replay["meta"]
	print("=== ReplayV2 Meta ===")
	print("- Coord unit: ", meta.get("coord_unit_mm", "?"), "mm")
	print("- Sim tick: ", meta.get("sim_tick_ms", "?"), "ms")
	print("- View tick: ", meta.get("view_tick_ms", "?"), "ms")
	print("- Save tick: ", meta.get("save_tick_ms", "?"), "ms")
	print("- Field: ", float(meta.get("field_x_max", 0)) / 10.0, "m x ", float(meta.get("field_y_max", 0)) / 10.0, "m")
	print("- Tracks: ", meta.get("track_count", "?"))

	if meta.has("match_info"):
		var match_info: Dictionary = meta["match_info"]
		print("- Seed: ", match_info.get("seed", "?"))
		print("- Score: ", match_info.get("score_home", "?"), "-", match_info.get("score_away", "?"))
		print("- Duration: ", match_info.get("duration_minutes", "?"), " minutes")


## 리플레이 유효성 검증
##
## Returns: true if valid
static func validate(replay: Dictionary) -> bool:
	if replay.is_empty():
		push_error("[ReplayLoaderV2] Empty replay")
		return false

	# 버전 체크
	if not replay.has("version") or replay["version"] != 2:
		push_error("[ReplayLoaderV2] Invalid version")
		return false

	# 메타 체크
	if not replay.has("meta"):
		push_error("[ReplayLoaderV2] Missing meta")
		return false

	var meta: Dictionary = replay["meta"]
	if meta.get("coord_unit_mm", 0) != 100:
		push_error("[ReplayLoaderV2] Invalid coord_unit_mm: expected 100")
		return false

	if meta.get("track_count", 0) != 23:
		push_error("[ReplayLoaderV2] Invalid track_count: expected 23")
		return false

	# 프레임 체크
	if not replay.has("save_frames"):
		push_error("[ReplayLoaderV2] Missing save_frames")
		return false

	var frames: Array = replay["save_frames"]
	if frames.is_empty():
		push_warning("[ReplayLoaderV2] Empty save_frames")

	# 첫 프레임 엔티티 수 검증
	if not frames.is_empty():
		var first_frame: Dictionary = frames[0]
		if not first_frame.has("entities"):
			push_error("[ReplayLoaderV2] Frame missing entities")
			return false
		var entities: Array = first_frame["entities"]
		if entities.size() != 23:
			push_error("[ReplayLoaderV2] Invalid entity count: expected 23, got %d" % entities.size())
			return false

	return true


## 리플레이 통계 정보
##
## Returns: Dictionary with stats
static func get_stats(replay: Dictionary) -> Dictionary:
	if replay.is_empty():
		return {}

	var frames: Array = replay.get("save_frames", [])
	var events: Array = replay.get("events", [])

	var stats := {
		"frame_count": frames.size(),
		"event_count": events.size(),
		"duration_ms": 0,
		"goals": 0,
		"passes": 0,
		"shots": 0
	}

	# 재생 시간 계산
	if not frames.is_empty():
		var last_frame: Dictionary = frames[-1]
		stats["duration_ms"] = int(last_frame.get("t_ms", 0))

	# 이벤트 타입별 집계
	for event in events:
		if not (event is Dictionary):
			continue
		var kind: int = int(event.get("kind", -1))
		match kind:
			0:  # goal
				stats["goals"] += 1
			1:  # pass
				stats["passes"] += 1
			2:  # shot
				stats["shots"] += 1

	return stats


## 이벤트 종류 상수 (참고용)
const EVENT_GOAL := 0
const EVENT_PASS := 1
const EVENT_SHOT := 2
const EVENT_TACKLE := 3
const EVENT_FOUL := 4
const EVENT_OFFSIDE := 5
const EVENT_CORNER := 6
const EVENT_FREEKICK := 7
const EVENT_PENALTY := 8
const EVENT_CARD_YELLOW := 9
const EVENT_CARD_RED := 10
const EVENT_SUBSTITUTION := 11
