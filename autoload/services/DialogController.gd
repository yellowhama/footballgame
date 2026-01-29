extends Node
## DialogController - 다이얼로그 제어 시스템
##
## 스토리 이벤트, 코치 조언, 선수 대화 등 모든 다이얼로그를 관리합니다.
## OpenFootball의 스토리 시스템과 연동됩니다.

signal dialog_started(dialog_id: String)
signal dialog_choice_made(choice_index: int)
signal dialog_ended(dialog_id: String, result: Dictionary)
signal story_event_triggered(event_id: String)
signal coach_advice_shown(advice_data: Dictionary)

const DIALOG_SCENE_PATH := "res://ui/dialogs/DialogBox.tscn"
const MAX_CHOICES := 4
const DEFAULT_TEXT_SPEED := 0.03

var current_dialog: Dictionary = {}
var dialog_queue: Array = []
var dialog_history: Array = []
var is_dialog_active: bool = false
var dialog_box: Control = null
var choices_disabled: bool = false
var story_flags: Dictionary = {}


func _ready() -> void:
	print("[DialogController] Initialized")
	set_process(false)


## 다이얼로그 표시
func show_dialog(dialog_data: Dictionary) -> void:
	if is_dialog_active:
		dialog_queue.append(dialog_data)
		return

	current_dialog = dialog_data
	is_dialog_active = true
	dialog_started.emit(dialog_data.get("id", "unknown"))

	# UI 생성 또는 표시
	_create_dialog_ui(dialog_data)


## 스토리 이벤트 다이얼로그
func show_story_event(event_id: String, event_data: Dictionary) -> void:
	var dialog_data = {
		"id": event_id,
		"type": "story_event",
		"title": event_data.get("title", ""),
		"text": event_data.get("description", ""),
		"choices": event_data.get("choices", []),
		"speaker": event_data.get("speaker", "System"),
		"portrait": event_data.get("portrait", null),
		"mood": event_data.get("mood", "neutral")
	}

	story_event_triggered.emit(event_id)
	show_dialog(dialog_data)


## 코치 조언 다이얼로그
func show_coach_advice(coach_name: String, advice_text: String, advice_type: String = "general") -> void:
	var advice_data = {"coach": coach_name, "text": advice_text, "type": advice_type}

	var dialog_data = {
		"id": "coach_advice_" + str(Time.get_unix_time_from_system()),
		"type": "coach_advice",
		"title": "코치의 조언",
		"text": advice_text,
		"speaker": coach_name,
		"portrait": _get_coach_portrait(coach_name),
		"mood": _get_advice_mood(advice_type),
		"choices": ["알겠습니다", "다시 한번 설명해주세요"]
	}

	coach_advice_shown.emit(advice_data)
	show_dialog(dialog_data)


## 선수 대화
func show_player_conversation(player_name: String, conversation_data: Dictionary) -> void:
	var dialog_data = {
		"id": "player_conv_" + str(Time.get_unix_time_from_system()),
		"type": "player_conversation",
		"title": player_name,
		"text": conversation_data.get("text", ""),
		"speaker": player_name,
		"portrait": conversation_data.get("portrait", null),
		"mood": conversation_data.get("mood", "neutral"),
		"choices": conversation_data.get("choices", [])
	}

	show_dialog(dialog_data)


## 시스템 메시지
func show_system_message(title: String, message: String, options: Array = ["확인"]) -> void:
	var dialog_data = {
		"id": "system_" + str(Time.get_unix_time_from_system()),
		"type": "system",
		"title": title,
		"text": message,
		"speaker": "System",
		"choices": options,
		"can_skip": false
	}

	show_dialog(dialog_data)


## 훈련 피드백 다이얼로그
func show_training_feedback(feedback_data: Dictionary) -> void:
	var dialog_data = {
		"id": "training_feedback_" + str(Time.get_unix_time_from_system()),
		"type": "training_feedback",
		"title": "훈련 결과",
		"text": _format_training_feedback(feedback_data),
		"speaker": "코치",
		"mood": _get_training_mood(feedback_data),
		"choices": ["확인"]
	}

	show_dialog(dialog_data)


## 경기 결과 다이얼로그
func show_match_result(match_data: Dictionary) -> void:
	var dialog_data = {
		"id": "match_result_" + str(Time.get_unix_time_from_system()),
		"type": "match_result",
		"title": "경기 결과",
		"text": _format_match_result(match_data),
		"speaker": "감독",
		"mood": _get_match_mood(match_data),
		"choices": ["다시보기", "다음으로"]
	}

	show_dialog(dialog_data)


## 선택지 선택
func make_choice(choice_index: int) -> void:
	if not is_dialog_active or choices_disabled:
		return

	dialog_choice_made.emit(choice_index)

	# 선택 결과 처리
	var result = {
		"dialog_id": current_dialog.get("id", ""),
		"choice_index": choice_index,
		"choice_text":
		(
			current_dialog.get("choices", [])[choice_index]
			if choice_index < current_dialog.get("choices", []).size()
			else ""
		)
	}

	# 스토리 이벤트인 경우 Rust 엔진으로 전달
	if current_dialog.get("type") == "story_event" and FootballRustEngine.simulator:
		var event_choice = {"event_id": current_dialog.get("id"), "choice_index": choice_index}
		FootballRustEngine.simulator.process_story_choice(JSON.stringify(event_choice))

	end_dialog(result)


## 다이얼로그 종료
func end_dialog(result: Dictionary = {}) -> void:
	if not is_dialog_active:
		return

	# 히스토리에 저장
	var history_entry = current_dialog.duplicate()
	history_entry["result"] = result
	history_entry["timestamp"] = Time.get_unix_time_from_system()
	dialog_history.append(history_entry)

	dialog_ended.emit(current_dialog.get("id", ""), result)

	# UI 제거
	if dialog_box:
		dialog_box.queue_free()
		dialog_box = null

	is_dialog_active = false
	current_dialog.clear()

	# 큐에 있는 다음 다이얼로그 표시
	if not dialog_queue.is_empty():
		var next_dialog = dialog_queue.pop_front()
		call_deferred("show_dialog", next_dialog)


## 다이얼로그 스킵
func skip_dialog() -> void:
	if not is_dialog_active or not current_dialog.get("can_skip", true):
		return

	end_dialog({"skipped": true})


## 모든 다이얼로그 클리어
func clear_all_dialogs() -> void:
	dialog_queue.clear()
	if is_dialog_active:
		end_dialog({"cleared": true})


## 스토리 플래그 설정
func set_story_flag(flag_name: String, value: Variant) -> void:
	story_flags[flag_name] = value


## 스토리 플래그 조회
func get_story_flag(flag_name: String, default_value: Variant = null) -> Variant:
	return story_flags.get(flag_name, default_value)


## 히스토리 조회
func get_dialog_history(type_filter: String = "") -> Array:
	if type_filter.is_empty():
		return dialog_history

	return dialog_history.filter(func(entry): return entry.get("type") == type_filter)


## === Private Helper Functions ===


func _create_dialog_ui(_dialog_data: Dictionary) -> void:
	# 실제 UI 생성 로직 (간략화)
	if not dialog_box:
		# dialog_box = load(DIALOG_SCENE_PATH).instantiate()
		# get_tree().current_scene.add_child(dialog_box)
		pass

	# UI 업데이트
	if dialog_box:
		# dialog_box.set_title(_dialog_data.get("title", ""))
		# dialog_box.set_text(_dialog_data.get("text", ""))
		# dialog_box.set_choices(_dialog_data.get("choices", []))
		pass


func _get_coach_portrait(_coach_name: String) -> Texture2D:
	# 코치 포트레이트 로드
	return null


func _get_advice_mood(advice_type: String) -> String:
	match advice_type:
		"warning":
			return "serious"
		"praise":
			return "happy"
		"criticism":
			return "angry"
		_:
			return "neutral"


func _format_training_feedback(feedback_data: Dictionary) -> String:
	var text = "훈련이 완료되었습니다.\n"

	if feedback_data.has("attribute_changes"):
		text += "\n성장한 능력치:\n"
		for attr in feedback_data.attribute_changes:
			text += "• %s: +%d\n" % [attr, feedback_data.attribute_changes[attr]]

	if feedback_data.has("condition_change"):
		text += "\n컨디션: %+d%%" % feedback_data.condition_change

	return text


func _format_match_result(match_data: Dictionary) -> String:
	var home_score = match_data.get("home_score", 0)
	var away_score = match_data.get("away_score", 0)
	var is_home = match_data.get("is_home", true)

	var result_text = ""
	if (is_home and home_score > away_score) or (not is_home and away_score > home_score):
		result_text = "승리! %d - %d" % [home_score, away_score]
	elif home_score == away_score:
		result_text = "무승부 %d - %d" % [home_score, away_score]
	else:
		result_text = "패배... %d - %d" % [home_score, away_score]

	return result_text


func _get_training_mood(feedback_data: Dictionary) -> String:
	var growth = feedback_data.get("total_growth", 0)
	if growth > 10:
		return "excited"
	elif growth > 5:
		return "happy"
	elif growth > 0:
		return "neutral"
	else:
		return "disappointed"


func _get_match_mood(match_data: Dictionary) -> String:
	var home_score = match_data.get("home_score", 0)
	var away_score = match_data.get("away_score", 0)
	var is_home = match_data.get("is_home", true)

	if (is_home and home_score > away_score) or (not is_home and away_score > home_score):
		return "happy"
	elif home_score == away_score:
		return "neutral"
	else:
		return "disappointed"


## 저장 데이터
func get_save_data() -> Dictionary:
	return {"dialog_history": dialog_history.duplicate(true), "story_flags": story_flags.duplicate(true)}


## 로드 데이터
func load_save_data(data: Dictionary) -> void:
	if data.has("dialog_history"):
		dialog_history = data.dialog_history.duplicate(true)
	if data.has("story_flags"):
		story_flags = data.story_flags.duplicate(true)
