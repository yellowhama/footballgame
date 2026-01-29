extends Node
class_name MatchSessionController
##
## MatchSessionController
##
## 실시간 Match Viewer를 위한 Godot 측 컨트롤러 스켈레톤.
## - Rust GDExtension(FootballMatchSimulator)의 Session step API를 호출해
##   매 tick마다 snapshot + events를 방송하는 역할을 한다.
## - 현재는 Rust 쪽 step API가 아직 없으므로, 안전한 스텁 형태로만 구현되어 있다.
##
## 참고 스펙:
## - docs/specs/spec_v5/fix/phase20/PHASE20_UNIFIED_FRAME_PIPELINE_SPEC.md
## - docs/specs/spec_v5/fix/phase20/PHASE20_VIEWER_ENGINE_DATA_CONTRACT.md
##
## Phase20: UnifiedFramePipeline integration (2025-12-18)
## - Emits tick_raw signal with raw step_result to UnifiedFramePipeline
## - Pipeline converts 250ms ticks to 50ms smooth output with ball interpolation
## - See: docs/specs/spec_v5/fix/phase20/PHASE20_UNIFIED_FRAME_PIPELINE_SPEC.md
##

# Preload to avoid class_name resolution issues
const PositionSnapshotAdapter := preload("res://scripts/match_pipeline/PositionSnapshotAdapter.gd")
const _METHOD_START_SESSION := "start_match_session"
const _METHOD_STEP_SESSION := "step_match_session"
const _METHOD_STEP_SESSION_PACKED := _METHOD_STEP_SESSION + "_packed"
const _METHOD_FINISH_SESSION := "finish_match_session"
signal tick(t_ms: int, snapshot: Dictionary, events: Array)
signal halftime(t_ms: int, snapshot: Dictionary, events: Array)
signal finished(result: Dictionary)
## Phase 4.6: Hero Time pause signal
## Emitted when the user's player has the ball and needs to make a decision
signal paused(t_ms: int, decision_context: Dictionary)
## Phase20: Pipeline integration - emit raw ticks to UnifiedFramePipeline
signal tick_raw(step_result: Dictionary)

var _simulator: Object = null
var _running: bool = false
var _max_dt_ms: int = 250
var _accumulated_ms: float = 0.0
var _current_rosters: Dictionary = {}

var _warned_missing_api: bool = false
var _halftime_notified: bool = false
const MAX_STEPS_PER_FRAME := 4

## Use packed format for better performance (reduces Dictionary allocations by 88%)
## Set to false to use legacy Dictionary-based format for compatibility
@export var use_packed_format: bool = true

## Current simulation time in milliseconds (source of truth for Session mode)
## Use this instead of MatchTimelineController.position_time_ms for substitution/tactic timing
var current_t_ms: int = 0

## Current simulation minute (derived from current_t_ms for convenience)
var current_minute: float:
	get:
		return float(current_t_ms) / 60000.0


func _ready() -> void:
	# 실시간 step 루프는 start_session() 가 성공했을 때만 활성화된다.
	set_process(false)
	_accumulated_ms = 0.0


func _exit_tree() -> void:
	# Clean up Rust session when node exits the tree
	stop_session()


func start_session(match_request: Dictionary) -> bool:
	## Session 세션 시작 요청.
	## 아직 Rust side Session API가 없기 때문에, 존재 여부를 점검하고
	## 없으면 경고만 출력한 뒤 아무 것도 하지 않는다.
	var engine := get_node_or_null("/root/FootballRustEngine")
	if not engine:
		push_warning("[MatchSessionController] FootballRustEngine not found; Session mode disabled.")
		return false

	if engine.has_method("is_ready") and not engine.is_ready():
		push_warning("[MatchSessionController] FootballRustEngine is not ready; Session mode disabled.")
		return false

	# 내부 _rust_simulator와 Session API 존재 여부 확인
	_simulator = engine.get("_rust_simulator")
	if _simulator == null:
		_warn_session_api_missing("engine._rust_simulator not available")
		return false

	if not _simulator.has_method(_METHOD_START_SESSION):
		_warn_session_api_missing("FootballMatchSimulator.start_match_session() not available")
		_simulator = null
		return false

	# Rust 쪽 API가 준비되면 여기서 실제로 세션을 시작한다.
	var json_request := JSON.stringify(match_request)
	var start_res: Variant = _simulator.call(_METHOD_START_SESSION, json_request)
	if not (start_res is Dictionary):
		push_warning(
			"[MatchSessionController] start_match_session() returned non-dictionary value; Session mode aborted."
		)
		_simulator = null
		return false
	var start_dict: Dictionary = start_res
	var ok: bool = bool(start_dict.get("success", false))
	var start_err: String = str(start_dict.get("error", ""))
	if (not ok) or start_err != "":
		push_warning("[MatchSessionController] start_match_session failed: %s" % start_err)
		_simulator = null
		return false

	# Store rosters for viewer metadata mapping (track_id → name/role/number)
	if match_request.has("rosters"):
		_current_rosters = match_request.get("rosters", {})
	else:
		_current_rosters = {}

	_running = true
	_accumulated_ms = 0.0
	_halftime_notified = false
	current_t_ms = 0
	set_process(true)
	print("[MatchSessionController] Session match session started.")
	return true


func stop_session() -> void:
	## Session 세션 강제 종료. Rust 측 finish_session() 이 있으면 호출한다.
	if not _running:
		return

	_running = false
	set_process(false)
	_accumulated_ms = 0.0

	if _simulator and _simulator.has_method(_METHOD_FINISH_SESSION):
		var result: Variant = _simulator.call(_METHOD_FINISH_SESSION)
		if result is Dictionary:
			finished.emit(result)
	else:
		finished.emit({})

	_simulator = null
	print("[MatchSessionController] Session match session stopped.")


func set_speed(multiplier: float) -> void:
	## 간단한 speed 조절 – tick 크기를 변경하는 방식으로만 처리.
	if multiplier <= 0.0:
		multiplier = 1.0
	_max_dt_ms = int(250.0 * multiplier)


func pause() -> void:
	_running = false


func resume() -> void:
	if _simulator != null:
		_running = true
		set_process(true)


func resume_second_half() -> void:
	## Resume the simulation after halftime.
	## Calls the Rust resume_second_half() API if available, then resumes processing.
	if _simulator == null:
		push_warning("[MatchSessionController] Cannot resume - no active simulator")
		return

	if _simulator.has_method("resume_second_half"):
		var result: Variant = _simulator.resume_second_half()
		if result is Dictionary:
			var res_dict: Dictionary = result
			if res_dict.has("error") and str(res_dict.get("error")) != "":
				push_warning("[MatchSessionController] resume_second_half error: %s" % res_dict.get("error"))
				return

	_running = true
	set_process(true)
	print("[MatchSessionController] Second half resumed.")


func _process(_delta: float) -> void:
	if not _running or _simulator == null:
		return

	if not _simulator.has_method(_METHOD_STEP_SESSION):
		_warn_session_api_missing("FootballMatchSimulator.step_session() not available")
		_running = false
		set_process(false)
		return

	# delta를 누적해서 일정 간격(_max_dt_ms)마다 step을 호출한다.
	_accumulated_ms += _delta * 1000.0
	if _accumulated_ms < float(_max_dt_ms):
		return

	# 프레임 드랍 등으로 인해 시간이 많이 누적된 경우에도
	# 한 프레임에 여러 번 step을 호출해 따라잡되,
	# MAX_STEPS_PER_FRAME 상한을 두어 한 프레임에 과도한 연산이 몰리지 않도록 한다.
	var steps_this_frame := 0
	while (
		_accumulated_ms >= float(_max_dt_ms)
		and _running
		and _simulator != null
		and steps_this_frame < MAX_STEPS_PER_FRAME
	):
		_accumulated_ms -= float(_max_dt_ms)
		_perform_step()
		steps_this_frame += 1


func _perform_step() -> void:
	# Defensive null check (prevents crash if simulator was cleared mid-frame)
	if _simulator == null:
		_running = false
		return

	# Use packed or legacy format based on configuration
	var res: Variant
	if use_packed_format and _simulator.has_method(_METHOD_STEP_SESSION_PACKED):
		res = _simulator.call(_METHOD_STEP_SESSION_PACKED, _max_dt_ms)
	else:
		res = _simulator.call(_METHOD_STEP_SESSION, _max_dt_ms)

	if not (res is Dictionary):
		push_warning("[MatchSessionController] step_session() returned non-dictionary value.")
		return

	var result: Dictionary = res
	# Keep viewer metadata mapping SSOT-correct: track_id is a pitch slot, and substitutions swap the occupant.
	# Apply roster swap BEFORE snapshot normalization so the same tick uses the updated mapping.
	_apply_substitution_events_to_rosters(result.get("events", []))
	# 정규화된 버전 사용 (통합 이벤트 스키마 적용)
	var snapshot: Dictionary
	if use_packed_format and result.get("snapshot", {}).has("players_packed"):
		snapshot = PositionSnapshotAdapter.from_step_packed_normalized(result, _current_rosters)
	else:
		snapshot = PositionSnapshotAdapter.from_step_normalized(result, _current_rosters)
	var events: Array = snapshot.get("events", [])
	var t_ms: int = int(result.get("t_ms", 0))

	# Update the source of truth for current simulation time
	current_t_ms = t_ms

	# Rust step API가 에러 정보를 돌려주는 경우를 대비한 방어 로직.
	# "error" 키가 존재하고 비어있지 않다면 라이브 세션을 종료하고
	# finished 신호로 상위 레이어에 그대로 전달한다.
	var error_msg := ""
	if result.has("error"):
		error_msg = str(result.get("error"))
	if error_msg != "":
		push_warning("[MatchSessionController] step_session error: %s" % error_msg)
		_running = false
		set_process(false)
		_accumulated_ms = 0.0
		finished.emit(result)
		_simulator = null
		return

	# Phase20: Emit raw tick to pipeline (before normalized snapshot)
	tick_raw.emit(result)

	# Still emit tick for MatchSimulationManager (state tracking)
	tick.emit(t_ms, snapshot, events)

	# Phase 4.6: Hero Time pause detection
	# When the Rust engine returns "paused": true with "user_decision" context,
	# pause the simulation and emit paused signal for UI to handle
	if result.get("paused", false):
		var decision_ctx: Dictionary = result.get("user_decision", {})
		if not decision_ctx.is_empty():
			_running = false
			set_process(false)
			print(
				(
					"[MatchSessionController] Hero Time pause at t_ms=%d, player=%s"
					% [t_ms, str(decision_ctx.get("player_track_id", -1))]
				)
			)
			paused.emit(t_ms, decision_ctx)
			return

	# Halftime detection: pause the simulation and emit halftime signal
	if bool(result.get("halftime", false)) and not _halftime_notified:
		_halftime_notified = true
		_running = false
		set_process(false)
		print("[MatchSessionController] Halftime reached at t_ms=%d" % t_ms)
		halftime.emit(t_ms, snapshot, events)
		return

	if bool(result.get("finished", false)):
		_running = false
		set_process(false)
		_accumulated_ms = 0.0

		var final_result: Dictionary = {}
		if _simulator != null and _simulator.has_method(_METHOD_FINISH_SESSION):
			var fr: Variant = _simulator.call(_METHOD_FINISH_SESSION)
			if fr is Dictionary:
				final_result = fr

		finished.emit(final_result)
		_simulator = null
		print("[MatchSessionController] Session match session finished.")


func _apply_substitution_events_to_rosters(raw_events: Array) -> void:
	if _current_rosters.is_empty() or raw_events.is_empty():
		return

	for ev in raw_events:
		if not (ev is Dictionary):
			continue
		var event_dict: Dictionary = ev
		var etype := str(event_dict.get("type", event_dict.get("etype", ""))).to_lower()
		if etype != "substitution":
			continue

		var is_home := true
		if event_dict.has("is_home_team"):
			is_home = bool(event_dict.get("is_home_team", true))
		elif event_dict.has("team"):
			var team_str := str(event_dict.get("team", "home")).to_lower()
			is_home = team_str != "away"
		else:
			# Fallback: infer from track_id split (0..10 home, 11..21 away)
			var tid_guess := int(event_dict.get("player_track_id", -1))
			is_home = tid_guess >= 0 and tid_guess <= 10

		var out_track_id := int(event_dict.get("player_track_id", -1))
		if out_track_id < 0:
			continue

		var details: Variant = event_dict.get("details", null)
		var bench_slot := -1
		if details is Dictionary:
			var sub: Variant = (details as Dictionary).get("substitution", null)
			if sub is Dictionary:
				bench_slot = int((sub as Dictionary).get("bench_slot", -1))

		if bench_slot < 0 or bench_slot > 6:
			continue

		var team_key := "home" if is_home else "away"
		var local_idx := out_track_id if is_home else (out_track_id - 11)
		var bench_idx := 11 + bench_slot

		var team_data: Variant = _current_rosters.get(team_key, null)
		if team_data == null:
			continue

		if team_data is Dictionary:
			var team_dict: Dictionary = team_data
			var roster_key := "players" if team_dict.has("players") else "roster"
			var roster: Variant = team_dict.get(roster_key, [])
			if not (roster is Array):
				continue
			var arr: Array = roster
			if local_idx >= 0 and local_idx < arr.size() and bench_idx >= 0 and bench_idx < arr.size():
				var tmp = arr[local_idx]
				arr[local_idx] = arr[bench_idx]
				arr[bench_idx] = tmp
				team_dict[roster_key] = arr
				_current_rosters[team_key] = team_dict
		elif team_data is Array:
			var arr: Array = team_data
			if local_idx >= 0 and local_idx < arr.size() and bench_idx >= 0 and bench_idx < arr.size():
				var tmp = arr[local_idx]
				arr[local_idx] = arr[bench_idx]
				arr[bench_idx] = tmp
				_current_rosters[team_key] = arr


func _warn_session_api_missing(detail: String) -> void:
	if _warned_missing_api:
		return
	_warned_missing_api = true
	push_warning("[MatchSessionController] Rust Session step API not available yet: %s" % detail)


func _adapt_snapshot_for_viewer(raw: Dictionary) -> Dictionary:
	## Session step API에서 넘어오는 snapshot을 MatchTimelineViewer가 기대하는 형태로 변환한다.
	## 입력:  { ball: {x, y, z}, players: { \"0\": {x, y}, ... } }
	## 출력: { ball, players: { \"0\": { position: {x, y} }, ... } }
	return PositionSnapshotAdapter.from_step_raw(raw)


## Phase 4.6: Hero Time user action submission
func submit_user_action(action: Dictionary) -> bool:
	## Submit user's action choice after Hero Time pause.
	## action format: { "action": "shoot" | "pass" | "dribble", "target_id": optional int }
	## Returns true if action was submitted successfully.
	if _simulator == null:
		push_warning("[MatchSessionController] Cannot submit action - no active simulator")
		return false

	if not _simulator.has_method("submit_user_action"):
		push_warning("[MatchSessionController] submit_user_action() not available in simulator")
		# Fallback: just resume without action
		resume()
		return false

	var json_action := JSON.stringify(action)
	var result: Variant = _simulator.submit_user_action(json_action)

	if result is Dictionary:
		var res_dict: Dictionary = result
		if res_dict.has("error") and str(res_dict.get("error")) != "":
			push_warning("[MatchSessionController] submit_user_action error: %s" % res_dict.get("error"))
			return false

	# Resume simulation after action is submitted
	resume()
	print("[MatchSessionController] User action submitted: %s" % str(action))
	return true


func resume_from_hero_time(action: Dictionary) -> void:
	## Convenience method: submit action and resume in one call.
	## Called by PlayerCommandOverlay when user selects an action.
	if submit_user_action(action):
		print("[MatchSessionController] Resumed from Hero Time with action: %s" % action.get("action", "unknown"))


func is_running() -> bool:
	return _running
