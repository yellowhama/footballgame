extends Control
class_name BulkProgressDialog

## Phase 6.3: Bulk Progress Dialog UI
## 4주 고속 진행 다이얼로그

signal progress_started
signal progress_cancelled
signal progress_completed(results: Dictionary)

## UI References
@onready var current_week_label: Label = %CurrentWeekLabel
@onready var target_week_label: Label = %TargetWeekLabel
@onready var warning_label: Label = %WarningLabel
@onready var progress_bar: ProgressBar = %ProgressBar
@onready var progress_label: Label = %ProgressLabel
@onready var cancel_button: Button = %CancelButton
@onready var start_button: Button = %StartButton
@onready var results_container: VBoxContainer = %ResultsContainer
@onready var stat_changes_label: Label = %StatChangesLabel

## State
var _game_manager: Node = null
var _is_progressing: bool = false
var _weeks_to_advance: int = 4


func _ready() -> void:
	_game_manager = get_node_or_null("/root/GameManager")

	if cancel_button:
		cancel_button.pressed.connect(_on_cancel_pressed)
	if start_button:
		start_button.pressed.connect(_on_start_pressed)

	if _game_manager:
		_game_manager.bulk_progress_week_completed.connect(_on_week_completed)
		_game_manager.bulk_progress_completed.connect(_on_progress_completed)
		_game_manager.bulk_progress_interrupted.connect(_on_progress_interrupted)

	results_container.visible = false
	_update_preview()


func show_dialog(weeks: int = 4) -> void:
	"""다이얼로그 표시.

	Args:
		weeks: 진행할 주 수 (기본 4주)
	"""
	_weeks_to_advance = weeks
	_is_progressing = false

	start_button.disabled = false
	start_button.text = "진행"
	cancel_button.text = "취소"

	progress_bar.value = 0
	progress_label.text = ""
	results_container.visible = false

	_update_preview()
	show()


func _update_preview() -> void:
	"""미리보기 업데이트."""
	if not _game_manager:
		return

	var current_week: int = _game_manager.current_week
	var current_year: int = _game_manager.current_year

	current_week_label.text = "현재: %d학년 %d주" % [current_year, current_week]

	# 목표 주차 계산
	var target_week: int = current_week + _weeks_to_advance
	var target_year: int = current_year
	while target_week > 52:
		target_week -= 52
		target_year += 1

	target_week_label.text = "목표: %d학년 %d주" % [target_year, target_week]


func _on_start_pressed() -> void:
	"""진행 시작."""
	if not _game_manager or _is_progressing:
		return

	_is_progressing = true
	start_button.disabled = true
	start_button.text = "진행 중..."
	cancel_button.text = "중단"

	progress_started.emit()

	# 비동기로 진행 시작
	var results: Dictionary = await _game_manager.bulk_advance_weeks(_weeks_to_advance)

	# 완료 후 처리 (이미 시그널로 처리됨)


func _on_cancel_pressed() -> void:
	"""취소/중단."""
	if _is_progressing:
		if _game_manager:
			_game_manager.cancel_bulk_progress()
		_is_progressing = false
	else:
		progress_cancelled.emit()
		hide()


func _on_week_completed(week_num: int, _result: Dictionary) -> void:
	"""주 완료 콜백."""
	var progress: float = float(week_num) / float(_weeks_to_advance) * 100.0
	progress_bar.value = progress
	progress_label.text = "%d / %d 주 완료" % [week_num, _weeks_to_advance]


func _on_progress_completed(results: Dictionary) -> void:
	"""진행 완료 콜백."""
	_is_progressing = false

	progress_bar.value = 100
	progress_label.text = "완료!"

	start_button.text = "확인"
	start_button.disabled = false
	start_button.pressed.disconnect(_on_start_pressed)
	start_button.pressed.connect(_on_close_after_complete)

	cancel_button.visible = false

	_show_results(results)
	progress_completed.emit(results)


func _on_progress_interrupted(reason: String, results: Dictionary) -> void:
	"""진행 중단 콜백."""
	_is_progressing = false

	progress_label.text = "중단됨: %s" % reason

	start_button.text = "확인"
	start_button.disabled = false
	start_button.pressed.disconnect(_on_start_pressed)
	start_button.pressed.connect(_on_close_after_complete)

	cancel_button.visible = false

	_show_results(results)
	progress_completed.emit(results)


func _show_results(results: Dictionary) -> void:
	"""결과 표시."""
	results_container.visible = true

	var stat_text: String = ""
	var stat_changes: Dictionary = results.get("stat_changes", {})

	if stat_changes.is_empty():
		stat_text = "스탯 변화 없음"
	else:
		for stat_name in stat_changes:
			var change: int = stat_changes[stat_name]
			if change > 0:
				stat_text += "%s +%d\n" % [stat_name, change]
			elif change < 0:
				stat_text += "%s %d\n" % [stat_name, change]

	stat_changes_label.text = stat_text.strip_edges()

	# 추가 정보
	var weeks_advanced: int = results.get("weeks_advanced", 0)
	var matches_played: int = results.get("matches_played", 0)
	var events_count: int = results.get("events_triggered", []).size()

	var info_text: String = "%d주 진행 | %d경기 | %d이벤트" % [weeks_advanced, matches_played, events_count]

	if results.get("interrupted", false):
		info_text += "\n(중단됨: %s)" % results.get("interrupt_reason", "")

	# 결과 라벨이 있다면 업데이트
	var info_label = results_container.get_node_or_null("InfoLabel")
	if info_label:
		info_label.text = info_text


func _on_close_after_complete() -> void:
	"""완료 후 닫기."""
	# 연결 복원
	if start_button.pressed.is_connected(_on_close_after_complete):
		start_button.pressed.disconnect(_on_close_after_complete)
	if not start_button.pressed.is_connected(_on_start_pressed):
		start_button.pressed.connect(_on_start_pressed)

	cancel_button.visible = true
	hide()
