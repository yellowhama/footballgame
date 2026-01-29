extends Control
## StoryEventPopup - 스토리 이벤트 팝업
## StoryManager.story_event_triggered 시그널과 연동
##
## 작성일: 2025-11-26
## 참조: 04_ui_design_system.md §6.4

signal choice_selected(event_id: String, choice_index: int)
signal popup_closed

# ============================================
# UI 노드 참조
# ============================================

@onready var background_dim: ColorRect = $BackgroundDim
@onready var panel: PanelContainer = $CenterContainer/Panel
@onready var title_label: Label = $CenterContainer/Panel/VBox/TitleLabel
@onready var description_label: RichTextLabel = $CenterContainer/Panel/VBox/DescriptionLabel
@onready var choices_container: VBoxContainer = $CenterContainer/Panel/VBox/ChoicesContainer
@onready var skip_button: Button = $CenterContainer/Panel/VBox/SkipButton

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG_ELEVATED = Color("#30363D")
const COLOR_BG_TERTIARY = Color("#21262D")
const COLOR_ACCENT_SECONDARY = Color("#1F6FEB")
const COLOR_ACCENT_WARNING = Color("#D29922")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")
const COLOR_TEXT_MUTED = Color("#6E7681")

# ============================================
# 상태 변수
# ============================================

var _current_event: Dictionary = {}
var _choice_buttons: Array[Button] = []

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	visible = false
	modulate.a = 0

	if skip_button:
		skip_button.pressed.connect(_on_skip_pressed)
		skip_button.visible = false  # 기본적으로 숨김

	if background_dim:
		background_dim.gui_input.connect(_on_background_clicked)

	# StoryManager 시그널 연결
	if StoryManager:
		StoryManager.story_event_triggered.connect(_on_story_event_triggered)
		StoryManager.choice_made.connect(_on_choice_made)


func _input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		# ESC로 닫기 (선택지 없는 이벤트만)
		if _current_event.get("choices", []).is_empty():
			_close_popup()
			get_viewport().set_input_as_handled()


# ============================================
# 공개 API
# ============================================


## 이벤트 표시
## @param event: Dictionary from StoryManager
##   - id: String
##   - event_type: String
##   - title: String
##   - description: String
##   - choices: Array of {id, text, available, requirement_text}
##   - priority: String
##   - tags: Array
func show_event(event: Dictionary) -> void:
	_current_event = event

	# UI 업데이트
	_update_title(event)
	_update_description(event)
	_create_choice_buttons(event.get("choices", []))
	_update_skip_button(event)

	# 표시 및 애니메이션
	visible = true
	_animate_show()


## 팝업 닫기
func hide_popup() -> void:
	_close_popup()


# ============================================
# UI 업데이트
# ============================================


func _update_title(event: Dictionary) -> void:
	if not title_label:
		return

	var title = event.get("title", "이벤트")
	title_label.text = title

	# 이벤트 타입에 따른 아이콘 추가
	var event_type = event.get("event_type", "")
	var icon = _get_event_icon(event_type)
	if not icon.is_empty():
		title_label.text = icon + " " + title


func _update_description(event: Dictionary) -> void:
	if not description_label:
		return

	var description = event.get("description", "")
	description_label.text = description


func _create_choice_buttons(choices: Array) -> void:
	if not choices_container:
		return

	# 기존 버튼 제거
	for btn in _choice_buttons:
		btn.queue_free()
	_choice_buttons.clear()

	# 선택지 없으면 종료
	if choices.is_empty():
		return

	# 선택지 버튼 생성
	for i in range(choices.size()):
		var choice = choices[i]
		var btn = _create_choice_button(choice, i)
		choices_container.add_child(btn)
		_choice_buttons.append(btn)

		# 순차 페이드인
		btn.modulate.a = 0
		var tween = create_tween()
		tween.tween_property(btn, "modulate:a", 1.0, 0.2).set_delay(0.1 + i * 0.1)


func _create_choice_button(choice: Dictionary, index: int) -> Button:
	var btn = Button.new()
	btn.custom_minimum_size = Vector2(0, 60)
	btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 텍스트
	var text = choice.get("text", "선택 %d" % (index + 1))
	var available = choice.get("available", true)
	var requirement = choice.get("requirement_text", "")

	if available:
		btn.text = text
	else:
		btn.text = text + "\n[조건: %s]" % requirement if requirement else text
		btn.disabled = true

	# 스타일
	btn.add_theme_font_size_override("font_size", 16)

	if available:
		btn.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)
	else:
		btn.add_theme_color_override("font_color", COLOR_TEXT_MUTED)

	# 클릭 이벤트
	btn.pressed.connect(_on_choice_pressed.bind(index))

	# 호버 효과
	btn.mouse_entered.connect(func(): _on_choice_hover(btn, true))
	btn.mouse_exited.connect(func(): _on_choice_hover(btn, false))

	return btn


func _update_skip_button(event: Dictionary) -> void:
	if not skip_button:
		return

	# 선택지 없는 이벤트만 스킵 가능
	var choices = event.get("choices", [])
	skip_button.visible = choices.is_empty()


# ============================================
# 애니메이션
# ============================================


func _animate_show() -> void:
	# 배경 페이드인
	var bg_tween = create_tween()
	bg_tween.tween_property(self, "modulate:a", 1.0, 0.2)

	# 패널 슬라이드 + 스케일
	if panel:
		panel.scale = Vector2(0.9, 0.9)
		panel.pivot_offset = panel.size / 2

		var panel_tween = create_tween()
		panel_tween.set_ease(Tween.EASE_OUT)
		panel_tween.set_trans(Tween.TRANS_BACK)
		panel_tween.tween_property(panel, "scale", Vector2(1.0, 1.0), 0.3)


func _animate_hide(callback: Callable = Callable()) -> void:
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.tween_property(self, "modulate:a", 0.0, 0.15)
	tween.tween_callback(
		func():
			visible = false
			if callback.is_valid():
				callback.call()
	)


func _close_popup() -> void:
	_animate_hide(
		func():
			popup_closed.emit()
			_current_event = {}
	)


# ============================================
# 이벤트 핸들러
# ============================================


func _on_story_event_triggered(event: Dictionary) -> void:
	show_event(event)


func _on_choice_made(_event_id: String, _choice_index: int, result: Dictionary) -> void:
	# 선택 결과 처리
	if result.get("success", false):
		# 효과 표시 (필요시)
		var effects = result.get("effects_applied", [])
		if not effects.is_empty():
			print("[StoryEventPopup] Effects: %s" % effects)

		# 다음 이벤트 확인
		var next_event = result.get("next_event", {})
		if not next_event.is_empty():
			# 잠시 대기 후 다음 이벤트 표시
			await get_tree().create_timer(0.3).timeout
			show_event(next_event)
		else:
			_close_popup()


func _on_choice_pressed(index: int) -> void:
	var event_id = _current_event.get("id", "")
	if event_id.is_empty():
		return

	print("[StoryEventPopup] Choice selected: %d for event %s" % [index, event_id])
	choice_selected.emit(event_id, index)

	# StoryManager로 선택 전달
	if StoryManager:
		var _result = await StoryManager.make_choice(event_id, index)
		# 결과는 choice_made 시그널로 처리됨


func _on_choice_hover(btn: Button, is_hover: bool) -> void:
	if btn.disabled:
		return

	if is_hover:
		var tween = create_tween()
		tween.tween_property(btn, "modulate", Color(1.2, 1.2, 1.2), 0.1)
	else:
		var tween = create_tween()
		tween.tween_property(btn, "modulate", Color.WHITE, 0.1)


func _on_skip_pressed() -> void:
	_close_popup()


func _on_background_clicked(event: InputEvent) -> void:
	# 선택지 없는 이벤트만 배경 클릭으로 닫기
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		if _current_event.get("choices", []).is_empty():
			_close_popup()


# ============================================
# 유틸리티
# ============================================


func _get_event_icon(event_type: String) -> String:
	match event_type:
		"story":
			return "📖"
		"training":
			return "💪"
		"match":
			return "⚽"
		"relationship":
			return "🤝"
		"career":
			return "📈"
		"random":
			return "🎲"
		"milestone":
			return "🏆"
		_:
			return ""
