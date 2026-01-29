extends Control
# Save/Load Screen with Multiple Slots - Phase 9.1 SaveManager Integration

@onready var title_label = $VBox/TitleLabel
@onready var slots_container = $VBox/SlotsContainer
@onready var back_button = $VBox/ButtonsContainer/BackButton
@onready var mode_label = $VBox/ModeLabel

var current_mode = "save"  # "save" or "load"
var slot_buttons = []


func _ready():
	print("[SaveLoadScreen] Initializing save/load screen (Phase 9.1)...")

	# Connect to SaveManager signals
	if SaveManager:
		SaveManager.save_completed.connect(_on_save_completed)
		SaveManager.load_completed.connect(_on_load_completed)
		SaveManager.save_error.connect(_on_save_error)

	# UI 초기화
	_setup_ui()
	_connect_signals()
	_update_slot_info()

	print("[SaveLoadScreen] Ready!")


func _setup_ui():
	"""UI 초기 설정"""
	# 타이틀 설정
	if title_label:
		title_label.text = "게임 저장/로드"
		title_label.add_theme_font_size_override("font_size", 32)

	# 모드 라벨 설정
	if mode_label:
		mode_label.text = "저장할 슬롯을 선택하세요" if current_mode == "save" else "로드할 슬롯을 선택하세요"

	# 슬롯 버튼들 생성 (3개 슬롯 + 자동저장)
	_create_slot_buttons()


func _create_slot_buttons():
	"""슬롯 버튼들 생성"""
	if not slots_container:
		return

	# 기존 버튼들 제거
	for child in slots_container.get_children():
		child.queue_free()

	slot_buttons.clear()

	# 일반 슬롯 3개
	for i in range(3):
		var slot_button = _create_slot_button(i)
		slots_container.add_child(slot_button)
		slot_buttons.append(slot_button)

	# 구분선
	var separator = HSeparator.new()
	separator.custom_minimum_size.y = 20
	slots_container.add_child(separator)

	# 자동저장 슬롯
	var auto_save_button = _create_auto_save_button()
	slots_container.add_child(auto_save_button)


func _create_slot_button(slot_index: int) -> Control:
	"""개별 슬롯 버튼 생성"""
	var container = VBoxContainer.new()

	# 슬롯 버튼
	var button = Button.new()
	button.custom_minimum_size = Vector2(600, 80)
	button.name = "SlotButton%d" % slot_index

	# 슬롯 정보 표시
	var slot_info = _get_slot_info(slot_index)
	_update_slot_button_display(button, slot_index, slot_info)

	# 시그널 연결
	button.pressed.connect(_on_slot_selected.bind(slot_index))

	container.add_child(button)
	return container


func _create_auto_save_button() -> Control:
	"""자동저장 버튼 생성"""
	var container = VBoxContainer.new()

	var button = Button.new()
	button.custom_minimum_size = Vector2(600, 60)
	button.name = "AutoSaveButton"

	# 자동저장 정보
	var auto_save_exists = _check_auto_save_exists()
	if auto_save_exists:
		button.text = "🔄 자동저장 (사용 가능)"
		if current_mode == "save":
			button.disabled = true  # 자동저장은 수동으로 덮어쓸 수 없음
	else:
		button.text = "🔄 자동저장 (없음)"
		if current_mode == "load":
			button.disabled = true

	# 시그널 연결
	if not button.disabled:
		button.pressed.connect(_on_auto_save_selected)

	container.add_child(button)
	return container


func _get_slot_info(slot_index: int) -> Dictionary:
	"""슬롯 정보 가져오기 (Phase 9.1 SaveManager 사용)"""
	if not SaveManager:
		return {"exists": false}

	var slot_id = "slot_%d" % slot_index
	var save_slots = SaveManager.get_save_slots()

	if save_slots.has(slot_id) and save_slots[slot_id].get("exists", false):
		var slot_data = save_slots[slot_id]
		return {
			"exists": true,
			"player_name": slot_data.get("player_name", "Unknown"),
			"progress": slot_data.get("progress", 0.0),
			"timestamp": slot_data.get("timestamp", 0)
		}
	else:
		return {"exists": false}


func _check_auto_save_exists() -> bool:
	"""자동저장 존재 확인 (Phase 9.2)"""
	if not SaveManager:
		return false
	return SaveManager.has_auto_save()


func _update_slot_button_display(button: Button, slot_index: int, slot_info: Dictionary):
	"""슬롯 버튼 표시 업데이트 (Phase 9.1)"""
	if slot_info.exists:
		var timestamp = Time.get_datetime_dict_from_unix_time(slot_info.timestamp)
		button.text = (
			"""💾 슬롯 %d
플레이어: %s
진행도: %.1f%%
저장일시: %04d-%02d-%02d %02d:%02d"""
			% [
				slot_index + 1,
				slot_info.player_name,
				slot_info.progress,
				timestamp.year,
				timestamp.month,
				timestamp.day,
				timestamp.hour,
				timestamp.minute
			]
		)

		# 저장된 슬롯 스타일
		var style = StyleBoxFlat.new()
		style.bg_color = Color(0.2, 0.4, 0.2, 0.8)  # 녹색 계열
		style.border_color = Color(0.4, 0.8, 0.4)
		style.border_width_left = 2
		style.border_width_right = 2
		style.border_width_top = 2
		style.border_width_bottom = 2
		style.corner_radius_top_left = 8
		style.corner_radius_top_right = 8
		style.corner_radius_bottom_left = 8
		style.corner_radius_bottom_right = 8
		button.add_theme_stylebox_override("normal", style)
	else:
		button.text = (
			"""📂 슬롯 %d
(비어있음)

새 게임을 저장하려면 클릭하세요"""
			% (slot_index + 1)
		)

		# 빈 슬롯 스타일
		var style = StyleBoxFlat.new()
		style.bg_color = Color(0.3, 0.3, 0.3, 0.5)  # 회색 계열
		style.border_color = Color(0.5, 0.5, 0.5)
		style.border_width_left = 2
		style.border_width_right = 2
		style.border_width_top = 2
		style.border_width_bottom = 2
		style.corner_radius_top_left = 8
		style.corner_radius_top_right = 8
		style.corner_radius_bottom_left = 8
		style.corner_radius_bottom_right = 8
		button.add_theme_stylebox_override("normal", style)

	# 로드 모드에서 빈 슬롯은 비활성화
	if current_mode == "load" and not slot_info.exists:
		button.disabled = true


func _update_slot_info():
	"""모든 슬롯 정보 업데이트"""
	for i in range(slot_buttons.size()):
		var slot_button = slot_buttons[i].get_child(0) as Button
		if slot_button:
			var slot_info = _get_slot_info(i)
			_update_slot_button_display(slot_button, i, slot_info)


func _connect_signals():
	"""시그널 연결"""
	if back_button:
		back_button.pressed.connect(_on_back_pressed)


func set_mode(mode: String):
	"""모드 설정 (save/load)"""
	current_mode = mode
	if mode_label:
		mode_label.text = "저장할 슬롯을 선택하세요" if mode == "save" else "로드할 슬롯을 선택하세요"
	_update_slot_info()


func _on_slot_selected(slot_index: int):
	"""슬롯 선택 처리"""
	print("[SaveLoadScreen] Slot %d selected for %s" % [slot_index, current_mode])

	var success = false
	if current_mode == "save":
		success = _perform_save(slot_index)
	else:
		success = _perform_load(slot_index)

	if success:
		_show_result_popup("성공!", "%s 작업이 완료되었습니다." % ("저장" if current_mode == "save" else "로드"))
		# 슬롯 정보 업데이트
		_update_slot_info()
	else:
		_show_result_popup("실패!", "%s 작업이 실패했습니다." % ("저장" if current_mode == "save" else "로드"))


func _on_auto_save_selected():
	"""자동저장 선택 처리"""
	print("[SaveLoadScreen] Auto save selected for %s" % current_mode)

	var success = false
	if current_mode == "load":
		success = _perform_auto_load()

	if success:
		_show_result_popup("성공!", "자동저장 로드가 완료되었습니다.")
	else:
		_show_result_popup("실패!", "자동저장 로드가 실패했습니다.")


func _perform_save(slot_index: int) -> bool:
	"""실제 저장 수행 (Phase 9.1 SaveManager 사용)"""
	if not SaveManager:
		push_error("[SaveLoadScreen] SaveManager not found")
		return false

	var slot_id = "slot_%d" % slot_index

	# 현재 게임 데이터 수집 (player_name, progress 등)
	var game_data = {}
	if GlobalCharacterData and GlobalCharacterData.character_data.has("player_name"):
		game_data["player_name"] = GlobalCharacterData.character_data["player_name"]
	else:
		game_data["player_name"] = "Player %d" % (slot_index + 1)

	# Phase 9.2: 실제 게임 진행도 계산 (GameManager 또는 DateManager 기반)
	game_data["progress"] = SaveManager.get_game_progress()

	# SaveManager를 통해 저장
	SaveManager.save_game(slot_id, game_data)

	# Signal로 결과 확인하므로 여기서는 true 반환
	return true


func _perform_load(slot_index: int) -> bool:
	"""실제 로드 수행 (Phase 9.1 SaveManager 사용)"""
	if not SaveManager:
		push_error("[SaveLoadScreen] SaveManager not found")
		return false

	var slot_id = "slot_%d" % slot_index

	# SaveManager를 통해 로드 (자동으로 모든 매니저에 데이터 복원)
	var loaded_data = SaveManager.load_game(slot_id)

	# 데이터가 반환되면 성공
	return loaded_data.size() > 0


func _perform_auto_load() -> bool:
	"""자동저장 로드 수행 (Phase 9.2)"""
	if not SaveManager:
		push_error("[SaveLoadScreen] SaveManager not found")
		return false

	if not SaveManager.has_auto_save():
		push_warning("[SaveLoadScreen] No auto-save found")
		return false

	# Load from auto-save slot
	var loaded_data = SaveManager.load_auto_save()

	# 데이터가 반환되면 성공
	return loaded_data.size() > 0


func _show_result_popup(title: String, message: String):
	"""결과 팝업 표시"""
	var popup = AcceptDialog.new()
	popup.title = title
	popup.dialog_text = message
	get_tree().root.add_child(popup)
	popup.popup_centered()

	# 3초 후 자동 닫기
	await get_tree().create_timer(3.0).timeout
	if popup and is_instance_valid(popup):
		popup.queue_free()


func _on_back_pressed():
	"""뒤로가기 버튼 처리"""
	get_tree().change_scene_to_file("res://scenes/menus/main_menu.tscn")


# Phase 9.1: SaveManager signal handlers
func _on_save_completed(slot_id: String):
	"""저장 완료 시그널 핸들러"""
	print("[SaveLoadScreen] Save completed: %s" % slot_id)
	_update_slot_info()


func _on_load_completed(slot_id: String):
	"""로드 완료 시그널 핸들러"""
	print("[SaveLoadScreen] Load completed: %s" % slot_id)


func _on_save_error(error_message: String):
	"""저장 에러 시그널 핸들러"""
	push_error("[SaveLoadScreen] Save error: %s" % error_message)
	_show_result_popup("오류!", "저장 중 오류가 발생했습니다: %s" % error_message)
