## UIManager.gd - UI 상태 및 전환 관리
## 글로벌 팝업 관리 포함
extends Node

signal screen_changed(screen_name: String)
signal ui_state_changed(state: String)

var current_screen: String = ""
var ui_state: String = "normal"
var screen_stack: Array[String] = []

# 글로벌 팝업 관리
var _story_popup: Control = null
var _popup_layer: CanvasLayer = null

const StoryEventPopupScene = preload("res://scenes/ui/StoryEventPopup.tscn")


func _ready() -> void:
	print("[UIManager] initialized")
	_setup_popup_layer()
	_connect_story_signals()


func _setup_popup_layer() -> void:
	"""팝업용 CanvasLayer 생성 (최상위 레이어)"""
	_popup_layer = CanvasLayer.new()
	_popup_layer.name = "PopupLayer"
	_popup_layer.layer = 100  # 최상위 레이어
	add_child(_popup_layer)


func _connect_story_signals() -> void:
	"""StoryManager 시그널 연결"""
	# 약간의 딜레이 후 연결 (autoload 로딩 순서 문제 방지)
	await get_tree().process_frame
	if StoryManager:
		StoryManager.story_event_triggered.connect(_on_story_event_triggered)
		print("[UIManager] Connected to StoryManager signals")


func change_screen(screen_name: String) -> void:
	var previous_screen: String = current_screen
	current_screen = screen_name
	screen_changed.emit(screen_name)
	print("Screen changed from %s to %s" % [previous_screen, screen_name])


func push_screen(screen_name: String) -> void:
	if current_screen != "":
		screen_stack.push_back(current_screen)
	change_screen(screen_name)


func pop_screen() -> void:
	if screen_stack.size() > 0:
		var previous_screen: String = screen_stack.pop_back()
		change_screen(previous_screen)


func set_ui_state(new_state: String) -> void:
	ui_state = new_state
	ui_state_changed.emit(new_state)


func get_current_screen() -> String:
	return current_screen


func show_loading(show: bool = true) -> void:
	# Stub implementation
	print("Loading screen: %s" % ("shown" if show else "hidden"))


func show_popup(popup_name: String, data: Dictionary = {}) -> void:
	match popup_name:
		"story_event":
			show_story_event(data)
		_:
			print("[UIManager] Unknown popup: %s with data: %s" % [popup_name, str(data)])


# ============================================
# Story Event Popup 관리
# ============================================


func _on_story_event_triggered(event: Dictionary) -> void:
	"""StoryManager에서 이벤트 발생 시 호출"""
	show_story_event(event)


func show_story_event(event: Dictionary) -> void:
	"""스토리 이벤트 팝업 표시"""
	if not _popup_layer:
		push_warning("[UIManager] Popup layer not initialized")
		return

	# 팝업 인스턴스 생성 (없으면)
	if not _story_popup:
		_story_popup = StoryEventPopupScene.instantiate()
		_popup_layer.add_child(_story_popup)
		_story_popup.popup_closed.connect(_on_story_popup_closed)
		print("[UIManager] StoryEventPopup instantiated")

	# 이벤트 표시
	_story_popup.show_event(event)
	set_ui_state("popup")


func hide_story_popup() -> void:
	"""스토리 이벤트 팝업 숨기기"""
	if _story_popup:
		_story_popup.hide_popup()


func _on_story_popup_closed() -> void:
	"""팝업 닫힘 콜백"""
	set_ui_state("normal")
	print("[UIManager] Story popup closed")
