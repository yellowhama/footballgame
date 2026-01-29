extends Node
## StoryManager - 스토리 시스템 관리
## Rust StoryBridge 래퍼 (godot_extension/src/story_bridge.rs)
##
## 작성일: 2025-11-26
## 연결: StoryBridge (RefCounted class in GDExtension)

# ============================================
# 시그널
# ============================================

## 스토리 시스템 초기화 완료
signal story_initialized(initial_route: String)

## 주간 이벤트 발생
signal week_processed(week: int, events: Array)

## 이벤트 발생 (UI에서 표시 필요)
signal story_event_triggered(event: Dictionary)

## 선택 완료
signal choice_made(event_id: String, choice_index: int, result: Dictionary)

## 루트 변경
signal route_changed(new_route: String)

## 에러 발생
signal story_error(error: String, code: String)

# ============================================
# 상태 변수
# ============================================

var _story_bridge: Object = null
var _is_initialized: bool = false
var _current_route: String = ""
var _current_week: int = 0
var _pending_events: Array = []  # 처리 대기 중인 이벤트

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_initialize_bridge()
	_connect_game_signals()  # Phase 6


func _initialize_bridge() -> void:
	"""StoryBridge 인스턴스 생성"""
	if ClassDB.class_exists("StoryBridge"):
		_story_bridge = ClassDB.instantiate("StoryBridge")
		if _story_bridge:
			print("[StoryManager] StoryBridge initialized successfully")
		else:
			push_error("[StoryManager] Failed to instantiate StoryBridge")
	else:
		push_warning("[StoryManager] StoryBridge class not available - Story features disabled")
		push_warning("[StoryManager] Make sure GDExtension is built with story_bridge.rs")


func is_available() -> bool:
	"""스토리 시스템 사용 가능 여부"""
	return _story_bridge != null


func is_initialized() -> bool:
	"""스토리 시스템 초기화 여부"""
	return _is_initialized


# ============================================
# 스토리 초기화
# ============================================


## 스토리 시스템 초기화
## @param config: Dictionary
##   - player_name: String
##   - player_ca: int
##   - team_name: String (optional)
##   - personality: String (optional)
## @return: Dictionary with result
func init_story(config: Dictionary) -> Dictionary:
	if not _story_bridge:
		var error = {"success": false, "error": "StoryBridge not available", "error_code": "NOT_AVAILABLE"}
		story_error.emit(error.error, error.error_code)
		return error

	var json_config = JSON.stringify(config)
	var json_response = _story_bridge.story_init(json_config)
	var result = _parse_json(json_response)

	if result.get("success", false):
		_is_initialized = true
		_current_route = result.get("initial_route", "")
		_current_week = result.get("current_week", 1)
		print("[StoryManager] Story initialized - Route: %s" % _current_route)
		story_initialized.emit(_current_route)
	else:
		var error_msg = result.get("error", "Unknown error")
		var error_code = result.get("error_code", "UNKNOWN")
		push_error("[StoryManager] Init failed: %s" % error_msg)
		story_error.emit(error_msg, error_code)

	return result


# ============================================
# 주간 처리
# ============================================


## 주간 이벤트 처리
## @param week_data: Dictionary
##   - week: int - 현재 주차
##   - match_results: Array (optional) - 경기 결과
##     - goals: int
##     - assists: int
##   - training_done: bool (optional)
## @return: Dictionary with events
func process_week(week_data: Dictionary) -> Dictionary:
	if not _story_bridge:
		return {"success": false, "error": "StoryBridge not available", "error_code": "NOT_AVAILABLE"}

	if not _is_initialized:
		return {"success": false, "error": "Story not initialized", "error_code": "NOT_INITIALIZED"}

	var json_data = JSON.stringify(week_data)
	var json_response = _story_bridge.story_process_week(json_data)
	var result = _parse_json(json_response)

	if result.get("success", false):
		_current_week = result.get("week", _current_week)

		# 이벤트 처리
		var events = result.get("events", [])
		if events.size() > 0:
			_pending_events = events.duplicate()
			print("[StoryManager] Week %d: %d events triggered" % [_current_week, events.size()])

			# 첫 번째 이벤트 발생
			_trigger_next_event()

		# 루트 변경 확인
		var new_route = result.get("current_route", _current_route)
		if new_route != _current_route and result.get("route_changed", false):
			_current_route = new_route
			print("[StoryManager] Route changed to: %s" % _current_route)
			route_changed.emit(_current_route)

		week_processed.emit(_current_week, events)
	else:
		var error_msg = result.get("error", "Unknown error")
		story_error.emit(error_msg, result.get("error_code", "UNKNOWN"))

	return result


## 대기 중인 다음 이벤트 트리거
func _trigger_next_event() -> void:
	if _pending_events.size() > 0:
		var event = _pending_events.pop_front()
		story_event_triggered.emit(event)


## 대기 중인 이벤트 있는지 확인
func has_pending_events() -> bool:
	return _pending_events.size() > 0


## 대기 중인 이벤트 수
func get_pending_event_count() -> int:
	return _pending_events.size()


## 대기 중인 이벤트 목록 반환
func get_pending_events() -> Array:
	return _pending_events.duplicate()


# ============================================
# Dialogic Integration (Phase 3)
# ============================================

## Story Event ID → Dialogic Timeline Path 매핑
const STORY_TO_DIALOGIC := {
	"manager_first_meeting": "res://dialogic/timelines/manager_first_meeting.dtl",
	"first_day": "res://dialogic/timelines/first_day_at_academy.dtl",
	"route_choice_week10": "res://dialogic/timelines/route_decision.dtl",
	# TODO: Add more timeline mappings as needed
	# "coach_intro": "res://dialogic/timelines/coach_introduction.dtl",
	# "first_match_prep": "res://dialogic/timelines/first_match_preparation.dtl",
}


## 이벤트 ID에 대응하는 Dialogic 타임라인 경로 반환
## @param event_id: String - 스토리 이벤트 ID
## @return: String - 타임라인 경로 (없으면 빈 문자열)
func get_dialogic_timeline(event_id: String) -> String:
	return STORY_TO_DIALOGIC.get(event_id, "")


# ============================================
# 선택지 처리
# ============================================


## 이벤트 선택지 선택
## @param event_id: String - 이벤트 ID
## @param choice_index: int - 선택지 인덱스 (0-based)
## @return: Dictionary with result
func make_choice(event_id: String, choice_index: int) -> Dictionary:
	if not _story_bridge:
		return {"success": false, "error": "StoryBridge not available", "error_code": "NOT_AVAILABLE"}

	if not _is_initialized:
		return {"success": false, "error": "Story not initialized", "error_code": "NOT_INITIALIZED"}

	var choice_data = {"event_id": event_id, "choice_index": choice_index}

	var json_data = JSON.stringify(choice_data)
	var json_response = _story_bridge.story_make_choice(json_data)
	var result = _parse_json(json_response)

	if result.get("success", false):
		print("[StoryManager] Choice made: Event=%s, Choice=%d" % [event_id, choice_index])
		choice_made.emit(event_id, choice_index, result)

		# 다음 대기 이벤트 트리거
		if has_pending_events():
			# 약간의 딜레이 후 다음 이벤트
			await get_tree().create_timer(0.5).timeout
			_trigger_next_event()
	else:
		var error_msg = result.get("error", "Unknown error")
		story_error.emit(error_msg, result.get("error_code", "UNKNOWN"))

	return result


# ============================================
# 상태 조회
# ============================================


## 현재 스토리 상태 조회
## @return: Dictionary with state info
func get_story_state() -> Dictionary:
	if not _story_bridge:
		return {"success": false, "error": "StoryBridge not available"}

	if not _is_initialized:
		return {"success": false, "error": "Story not initialized"}

	var json_response = _story_bridge.get_story_state()
	return _parse_json(json_response)


## 현재 루트
func get_current_route() -> String:
	return _current_route


## 현재 주차
func get_current_week() -> int:
	return _current_week


# ============================================
# 저장/로드
# ============================================


## 스토리 상태 저장 (Base64 인코딩 문자열 반환)
## @return: Dictionary with save_data string
func save_story() -> Dictionary:
	if not _story_bridge:
		return {"success": false, "error": "StoryBridge not available"}

	if not _is_initialized:
		return {"success": false, "error": "Story not initialized"}

	var json_response = _story_bridge.save_story_state()
	var result = _parse_json(json_response)

	if result.get("success", false):
		print("[StoryManager] Story saved: %d bytes" % result.get("size_bytes", 0))

	return result


## 스토리 상태 로드
## @param save_data: String - Base64 인코딩된 저장 데이터
## @return: Dictionary with result
func load_story(save_data: String) -> Dictionary:
	if not _story_bridge:
		return {"success": false, "error": "StoryBridge not available"}

	var json_response = _story_bridge.load_story_state(save_data)
	var result = _parse_json(json_response)

	if result.get("success", false):
		_is_initialized = true
		_current_week = result.get("current_week", 1)
		print("[StoryManager] Story loaded: Week %d" % _current_week)

		# 현재 상태 새로고침
		var state = get_story_state()
		if state.get("success", false):
			_current_route = state.get("current_route", "")

	return result


# ============================================
# 유틸리티
# ============================================


func _parse_json(json_string: String) -> Dictionary:
	"""JSON 문자열 파싱"""
	if json_string.is_empty():
		return {"success": false, "error": "Empty response", "error_code": "EMPTY_RESPONSE"}

	var parser = JSON.new()
	var error = parser.parse(json_string)

	if error != OK:
		return {
			"success": false, "error": "JSON parse error: %s" % parser.get_error_message(), "error_code": "PARSE_ERROR"
		}

	var data = parser.data
	if data is Dictionary:
		return data
	else:
		return {"success": false, "error": "Invalid response format", "error_code": "INVALID_FORMAT"}


# ============================================
# 디버그
# ============================================


func _debug_print_state() -> void:
	"""현재 상태 출력 (디버그용)"""
	print("=== Story Manager State ===")
	print("Available: ", is_available())
	print("Initialized: ", _is_initialized)
	print("Current Route: ", _current_route)
	print("Current Week: ", _current_week)
	print("Pending Events: ", _pending_events.size())
	print("===========================")


# ============================================
# Game Event Integration (Phase 6)
# ============================================

# Match result storage
var _last_match_result: Dictionary = {}


func _connect_game_signals() -> void:
	"""Connect to DateManager and MatchManager signals"""
	# DateManager connection (weekly events + auto-init)
	if DateManager:
		if DateManager.has_signal("week_started"):
			# Force disconnect before connecting to prevent hot-reload duplicates
			if DateManager.week_started.is_connected(_on_week_started):
				DateManager.week_started.disconnect(_on_week_started)
			DateManager.week_started.connect(_on_week_started)
			print("[StoryManager] Connected to DateManager.week_started")
		else:
			push_warning("[StoryManager] DateManager.week_started signal not found")
	else:
		push_warning("[StoryManager] DateManager not available")

	# MatchManager connection (match results)
	if MatchManager:
		if MatchManager.has_signal("match_ended"):
			# Force disconnect before connecting to prevent hot-reload duplicates
			if MatchManager.match_ended.is_connected(_on_match_ended):
				MatchManager.match_ended.disconnect(_on_match_ended)
			MatchManager.match_ended.connect(_on_match_ended)
			print("[StoryManager] Connected to MatchManager.match_ended")
		else:
			push_warning("[StoryManager] MatchManager.match_ended signal not found")
	else:
		push_warning("[StoryManager] MatchManager not available")


func _on_week_started(week: int, _schedule) -> void:
	"""Handle week start - auto-init on week 1, trigger weekly events"""
	print("[StoryManager] Week %d started" % week)

	# Auto-initialize on first week
	if not _is_initialized and week == 1:
		print("[StoryManager] Auto-initializing story system on week 1")
		var player_name: String = PlayerData.player_name if PlayerData else "Player"
		var player_ca: int = 100  # Default
		if PlayerData and PlayerData.has_method("get_overall_rating"):
			player_ca = PlayerData.get_overall_rating()

		var config := {"player_name": player_name, "player_ca": player_ca, "team_name": "Youth Academy"}

		init_story(config)

	# Process weekly events if initialized
	if _is_initialized:
		var week_data := {"week": week, "match_results": _last_match_result, "training_done": true}  # Assume training was done

		var result := process_week(week_data)
		if not result.get("success", false):
			push_warning("[StoryManager] Failed to process week: %s" % result.get("error", "unknown"))

		# Clear last match result after processing
		_last_match_result = {}


func _on_match_ended(match_result: Dictionary) -> void:
	"""Handle match end - store result for next week processing"""
	if not match_result.is_empty():
		# Extract relevant data
		var goals: int = (
			match_result.get("home_goals", 0)
			if match_result.get("is_home", true)
			else match_result.get("away_goals", 0)
		)
		var assists: int = match_result.get("assists", 0)
		var won: bool = match_result.get("result", "") == "win"

		_last_match_result = {"goals": goals, "assists": assists, "won": won}

		print("[StoryManager] Match result stored: %d goals, %d assists, won: %s" % [goals, assists, won])


# ============================================
# Debug Commands (Phase 6)
# ============================================


## Debug: Force trigger a story event
func debug_trigger_event(event_id: String) -> void:
	"""Manually trigger a story event for testing"""
	var event := {
		"id": event_id,
		"title": "Debug Event: %s" % event_id,
		"description": "Manually triggered event for testing",
		"route": _current_route,
		"choices": [{"text": "Choice 1", "available": true}, {"text": "Choice 2", "available": true}]
	}

	_pending_events.append(event)
	story_event_triggered.emit(event)
	print("[StoryManager] Debug event triggered: %s" % event_id)


## Debug: Force route change
func debug_set_route(route: String) -> void:
	"""Manually change story route for testing"""
	if route in ["Elite", "Standard", "Underdog"]:
		_current_route = route
		route_changed.emit(route)
		print("[StoryManager] Debug route changed to: %s" % route)
	else:
		push_error("[StoryManager] Invalid route: %s (must be Elite, Standard, or Underdog)" % route)


## Debug: Print current status
func debug_status() -> void:
	"""Print detailed status information"""
	_debug_print_state()
