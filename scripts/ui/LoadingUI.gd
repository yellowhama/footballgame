extends CanvasLayer

@onready var backdrop: ColorRect = $Backdrop
@onready var label: Label = $Center/VBox/Label
@onready var progress: ProgressBar = $Center/VBox/ProgressBar

var _shown_time: float = 0.0
var _timeout_sec: float = 30.0
var _initial_text: String = "경기 시뮬레이션 중…"
var _indeterminate: bool = true
var _progress_max: float = 100.0
var _progress_speed: float = 35.0


func _ready() -> void:
	visible = false
	set_process(false)
	if progress:
		progress.min_value = 0.0
		progress.max_value = _progress_max
		progress.visible = true
		progress.value = 0.0


func _process(delta: float) -> void:
	_shown_time += delta
	if _shown_time > _timeout_sec and label:
		label.text = "시간이 조금 걸리고 있어요… 잠시만 기다려 주세요"
	if progress and _indeterminate:
		progress.value = fmod(progress.value + _progress_speed * delta, _progress_max)


func show_loading(text: String = "경기 시뮬레이션 중…") -> void:
	_initial_text = text
	_shown_time = 0.0
	_indeterminate = true
	if label:
		label.text = text
	visible = true
	set_process(true)
	if progress:
		progress.value = 0.0
		progress.visible = true


func hide_loading() -> void:
	visible = false
	set_process(false)
	_shown_time = 0.0
	if label:
		label.text = _initial_text
	_indeterminate = true


func set_progress(p: float, max_value: float = 100.0) -> void:
	_indeterminate = false
	_progress_max = max_value
	if progress:
		progress.max_value = max_value
		progress.value = clampf(p, 0.0, max_value)
		progress.visible = true
