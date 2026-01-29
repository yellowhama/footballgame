extends Control
class_name MatchTimelineScrubber
## MatchTimelineScrubber - 타임라인 컨트롤 UI
##
## 기존 MatchTimelineController(autoload/rust/MatchTimelineController.gd)와 연동하여
## 재생/일시정지/배속/탐색 기능을 제공합니다.
##
## 사용법:
##   1. 씬에 추가하고 controller_path를 MatchTimelineController 노드로 설정
##   2. 또는 autoload MatchTimelineController 자동 연결 (use_autoload = true)

#region Export Variables
@export_group("Controller")
@export var controller_path: NodePath
@export var use_autoload: bool = true  ## /root/MatchTimelineController 자동 연결

@export_group("Playback")
@export var speed_options: PackedFloat32Array = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0]
@export var default_speed_index: int = 2  ## 1.0x

@export_group("UI")
@export var show_time_label: bool = true
@export var show_speed_button: bool = true
#endregion

#region Node References
var _controller: Node = null

@onready var _slider: HSlider = $HBox/TimeSlider if has_node("HBox/TimeSlider") else null
@onready var _play_button: Button = $HBox/PlayPauseButton if has_node("HBox/PlayPauseButton") else null
@onready var _speed_button: Button = $HBox/SpeedButton if has_node("HBox/SpeedButton") else null
@onready var _time_label: Label = $HBox/TimeLabel if has_node("HBox/TimeLabel") else null
#endregion

#region State
var _speed_index: int = 2
var _ignore_slider_change: bool = false
var _is_initialized: bool = false
#endregion


func _ready() -> void:
	_speed_index = default_speed_index
	_connect_controller()
	_setup_ui()
	_is_initialized = true


func _connect_controller() -> void:
	## 1. 먼저 controller_path 시도
	if not controller_path.is_empty():
		_controller = get_node_or_null(controller_path)

	## 2. autoload 시도
	if _controller == null and use_autoload:
		if has_node("/root/MatchTimelineController"):
			_controller = get_node("/root/MatchTimelineController")

	if _controller == null:
		push_warning("[MatchTimelineScrubber] Controller not found - set controller_path or enable use_autoload")
		return

	## 시그널 연결
	if _controller.has_signal("position_playback_stopped"):
		_controller.position_playback_stopped.connect(_on_playback_stopped)
	if _controller.has_signal("position_playback_started"):
		_controller.position_playback_started.connect(_on_playback_started)

	print("[MatchTimelineScrubber] Connected to MatchTimelineController")


func _setup_ui() -> void:
	## Slider 설정
	if _slider:
		_slider.min_value = 0.0
		_slider.max_value = 1.0
		_slider.step = 0.001
		_slider.value = 0.0
		_slider.value_changed.connect(_on_slider_value_changed)

	## Play/Pause 버튼
	if _play_button:
		_play_button.text = "Play"
		_play_button.pressed.connect(_on_play_pause_pressed)

	## Speed 버튼
	if _speed_button:
		_speed_button.visible = show_speed_button
		_speed_button.pressed.connect(_on_speed_pressed)
		_update_speed_button()

	## Time 라벨
	if _time_label:
		_time_label.visible = show_time_label
		_update_time_label(0, 0)


func _process(_delta: float) -> void:
	if not _is_initialized or _controller == null:
		return

	## 현재 시간/전체 시간 가져오기
	var t_ms: int = 0
	var duration_ms: int = 1

	if _controller.has_method("get_current_time_ms"):
		t_ms = _controller.get_current_time_ms()
	elif "position_time_ms" in _controller:
		t_ms = _controller.position_time_ms

	if "position_total_duration_ms" in _controller:
		duration_ms = max(_controller.position_total_duration_ms, 1)

	## Slider 업데이트 (프로그래밍 변경 시 value_changed 트리거 방지)
	if _slider and duration_ms > 0:
		_ignore_slider_change = true
		_slider.value = float(t_ms) / float(duration_ms)
		_ignore_slider_change = false

		## duration이 0이면 Slider 비활성화
		_slider.editable = duration_ms > 1

	## Time 라벨 업데이트
	_update_time_label(t_ms, duration_ms)

	## Play/Pause 버튼 텍스트 업데이트
	if _play_button:
		var is_playing := false
		if _controller.has_method("is_playing"):
			is_playing = _controller.is_playing()
		elif "position_playing" in _controller:
			is_playing = _controller.position_playing and not _controller.position_paused
		_play_button.text = "Pause" if is_playing else "Play"


#region UI Event Handlers


func _on_slider_value_changed(value: float) -> void:
	if _ignore_slider_change:
		return
	if _controller == null:
		return

	## 재생 중이면 일시정지
	if _controller.has_method("pause_position_playback"):
		_controller.pause_position_playback()

	## Seek
	var duration_ms: int = 1
	if "position_total_duration_ms" in _controller:
		duration_ms = max(_controller.position_total_duration_ms, 1)

	var target_ms: int = int(value * duration_ms)

	if _controller.has_method("seek_position_time"):
		_controller.seek_position_time(target_ms)


func _on_play_pause_pressed() -> void:
	if _controller == null:
		return

	var is_playing := false
	if "position_playing" in _controller:
		is_playing = _controller.position_playing and not _controller.position_paused

	if is_playing:
		if _controller.has_method("pause_position_playback"):
			_controller.pause_position_playback()
	else:
		## 재생 시작 또는 재개
		if "position_paused" in _controller and _controller.position_paused:
			if _controller.has_method("resume_position_playback"):
				_controller.resume_position_playback()
		elif "position_playing" in _controller and not _controller.position_playing:
			if _controller.has_method("start_position_playback"):
				var speed := speed_options[_speed_index] if _speed_index < speed_options.size() else 1.0
				_controller.start_position_playback(speed)
		else:
			## 이미 재생 중이면 resume
			if _controller.has_method("resume_position_playback"):
				_controller.resume_position_playback()


func _on_speed_pressed() -> void:
	if _controller == null:
		return

	_speed_index = (_speed_index + 1) % speed_options.size()
	var speed: float = speed_options[_speed_index]

	if _controller.has_method("set_position_playback_speed"):
		_controller.set_position_playback_speed(speed)

	_update_speed_button()


#endregion

#region Signal Handlers


func _on_playback_started(_duration_ms: int) -> void:
	if _play_button:
		_play_button.text = "Pause"


func _on_playback_stopped() -> void:
	if _play_button:
		_play_button.text = "Play"


#endregion

#region UI Updates


func _update_speed_button() -> void:
	if _speed_button == null:
		return

	var speed: float = speed_options[_speed_index] if _speed_index < speed_options.size() else 1.0
	_speed_button.text = "%.2gx" % speed


func _update_time_label(t_ms: int, duration_ms: int) -> void:
	if _time_label == null:
		return

	var cur_s := int(t_ms / 1000)
	var dur_s := int(duration_ms / 1000)

	## MM:SS 포맷
	_time_label.text = "%02d:%02d / %02d:%02d" % [cur_s / 60, cur_s % 60, dur_s / 60, dur_s % 60]


#endregion

#region Public API


## 외부에서 Controller 설정
func set_controller(controller: Node) -> void:
	_controller = controller
	_connect_controller()


## 배속 직접 설정 (인덱스)
func set_speed_index(index: int) -> void:
	if index >= 0 and index < speed_options.size():
		_speed_index = index
		_update_speed_button()
		if _controller and _controller.has_method("set_position_playback_speed"):
			_controller.set_position_playback_speed(speed_options[index])


## 현재 배속 반환
func get_current_speed() -> float:
	return speed_options[_speed_index] if _speed_index < speed_options.size() else 1.0

#endregion
