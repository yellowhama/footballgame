extends Control
class_name CareerIntroScreen

# ============================================================================
# 육성 게임 인트로 화면 - 슬롯 선택 시스템
# 우마무스메 스타일 세이브 슬롯 관리
# ============================================================================

# 씬 경로
const TEAM_CREATION_SCENE = "res://scenes/TeamCreation.tscn"
const CHARACTER_CREATION_SCENE = "res://scenes/CharacterCreation.tscn"
const HOME_IMPROVED_SCENE = "res://scenes/HomeImproved.tscn"
const MAIN_HOME_SCENE = "res://scenes/HomeImproved.tscn"

# 슬롯 데이터 구조
var slot_data = {"slot1": null, "slot2": null}  # SaveData 또는 null  # SaveData 또는 null

# UI 요소들 (%UniqueNodeName 패턴 사용)
@onready var slot1_panel: Panel = %Slot1Panel
@onready var slot2_panel: Panel = %Slot2Panel

@onready var slot1_button: Button = %Slot1LoadButton
@onready var slot2_button: Button = %Slot2LoadButton

@onready var slot1_delete: Button = %Slot1DeleteButton
@onready var slot2_delete: Button = %Slot2DeleteButton

@onready var slot1_info: VBoxContainer = %Slot1InfoContainer
@onready var slot2_info: VBoxContainer = %Slot2InfoContainer

@onready var back_button: Button = %BackButton

# 색상 테마
const SLOT_EMPTY_COLOR = Color(0.2, 0.2, 0.2, 0.8)
const SLOT_FILLED_COLOR = Color(0.1, 0.3, 0.1, 0.9)
const SLOT_HOVER_COLOR = Color(0.2, 0.4, 0.2, 1.0)
const DELETE_COLOR = Color(0.8, 0.2, 0.2, 1.0)

# ============================================================================
# 초기화
# ============================================================================


func _ready():
	print("[CareerIntroScreen] Initializing career intro screen...")

	# 슬롯 데이터 로드
	_load_slot_data()

	# UI 초기화
	_setup_ui()

	# 버튼 연결
	_connect_buttons()

	# 진입 애니메이션
	_play_entrance_animation()


func _load_slot_data():
	"""Load slot data from SaveManager"""
	# SaveManager를 사용
	if has_node("/root/SaveManager"):
		var save_manager = get_node("/root/SaveManager")
		var slots = save_manager.get_save_slots()

		# Slot 0과 1만 사용 (2개 슬롯)
		if slots.has("slot_0") and slots["slot_0"].get("exists", false):
			slot_data["slot1"] = slots["slot_0"]
		else:
			slot_data["slot1"] = null

		if slots.has("slot_1") and slots["slot_1"].get("exists", false):
			slot_data["slot2"] = slots["slot_1"]
		else:
			slot_data["slot2"] = null

		print("[CareerIntroScreen] Loaded save slots: ", slot_data)
	else:
		print("[CareerIntroScreen] SaveManager not found, creating dummy data")


func _setup_ui():
	"""UI 요소 초기화 및 표시"""
	_update_slot_display(1)
	_update_slot_display(2)


func _update_slot_display(slot_number: int):
	"""슬롯 표시 업데이트"""
	var panel = slot1_panel if slot_number == 1 else slot2_panel
	var button = slot1_button if slot_number == 1 else slot2_button
	var delete_btn = slot1_delete if slot_number == 1 else slot2_delete
	var info = slot1_info if slot_number == 1 else slot2_info
	var data = slot_data["slot" + str(slot_number)]

	if not panel or not button:
		print("[CareerIntroScreen] Warning: Slot UI elements not found for slot ", slot_number)
		return

	if data == null:
		# 빈 슬롯
		button.text = "🆕 NEW GAME"
		button.modulate = Color.WHITE
		if delete_btn:
			delete_btn.visible = false
		if info:
			info.visible = false

		# 패널 스타일
		var style = StyleBoxFlat.new()
		style.bg_color = SLOT_EMPTY_COLOR
		style.set_border_width_all(2)
		style.border_color = Color(0.3, 0.3, 0.3, 1.0)
		style.set_corner_radius_all(10)
		panel.add_theme_stylebox_override("panel", style)
	else:
		# 데이터가 있는 슬롯
		button.text = "▶️ CONTINUE"
		button.modulate = Color(0.8, 1.0, 0.8, 1.0)
		if delete_btn:
			delete_btn.visible = true

		# 슬롯 정보 표시
		if info:
			info.visible = true
			_display_save_info(info, data)

		# 패널 스타일
		var style = StyleBoxFlat.new()
		style.bg_color = SLOT_FILLED_COLOR
		style.set_border_width_all(2)
		style.border_color = Color(0.2, 0.5, 0.2, 1.0)
		style.set_corner_radius_all(10)
		panel.add_theme_stylebox_override("panel", style)


func _display_save_info(info_container: VBoxContainer, save_data: Dictionary):
	"""세이브 데이터 정보 표시"""
	# 기존 자식 제거
	for child in info_container.get_children():
		child.queue_free()

	# Player name
	var name_label = Label.new()
	name_label.text = "⚽ " + str(save_data.get("player_name", "Unknown Player"))
	name_label.add_theme_font_size_override("font_size", 18)
	info_container.add_child(name_label)

	# Progress
	var progress_label = Label.new()
	var year = save_data.get("current_year", 1)
	var week = save_data.get("current_week", 1)
	progress_label.text = "📅 Year %d Week %d" % [year, week]
	progress_label.add_theme_font_size_override("font_size", 14)
	progress_label.modulate = Color(0.8, 0.8, 0.8, 1.0)
	info_container.add_child(progress_label)

	# Stats
	var ca_label = Label.new()
	var ca = save_data.get("current_ability", 50)
	var pa = save_data.get("potential_ability", 80)
	ca_label.text = "💪 CA: %d / PA: %d" % [ca, pa]
	ca_label.add_theme_font_size_override("font_size", 14)
	ca_label.modulate = Color(0.8, 0.8, 0.8, 1.0)
	info_container.add_child(ca_label)

	# Save time
	var time_label = Label.new()
	var save_time = save_data.get("save_time", "Unknown")
	time_label.text = "💾 " + save_time
	time_label.add_theme_font_size_override("font_size", 12)
	time_label.modulate = Color(0.6, 0.6, 0.6, 1.0)
	info_container.add_child(time_label)


func _connect_buttons():
	"""버튼 이벤트 연결"""
	if slot1_button:
		slot1_button.pressed.connect(func(): _on_slot_pressed(1))
		slot1_button.mouse_entered.connect(func(): _on_slot_hover(1, true))
		slot1_button.mouse_exited.connect(func(): _on_slot_hover(1, false))

	if slot2_button:
		slot2_button.pressed.connect(func(): _on_slot_pressed(2))
		slot2_button.mouse_entered.connect(func(): _on_slot_hover(2, true))
		slot2_button.mouse_exited.connect(func(): _on_slot_hover(2, false))

	if slot1_delete:
		slot1_delete.pressed.connect(func(): _on_delete_pressed(1))

	if slot2_delete:
		slot2_delete.pressed.connect(func(): _on_delete_pressed(2))

	if back_button:
		back_button.pressed.connect(_on_back_pressed)


# ============================================================================
# 버튼 핸들러
# ============================================================================


func _on_slot_pressed(slot_number: int):
	"""슬롯 선택 처리"""
	print("[CareerIntroScreen] ========================================")
	print("[CareerIntroScreen] _on_slot_pressed called! slot_number = ", slot_number)
	var data = slot_data["slot" + str(slot_number)]
	print("[CareerIntroScreen] slot_data = ", data)

	if data == null:
		# 새 게임 시작 - Initial Setup Flow (스펙 11.1)
		# 1. 팀 창단 → 2. 캐릭터 생성 순서
		print("[CareerIntroScreen] 🎮 Starting new game in slot ", slot_number)

		# Save current slot number
		if has_node("/root/SaveManager"):
			var save_manager = get_node("/root/SaveManager")
			save_manager.current_save_slot = "slot_%d" % (slot_number - 1)
			print("[CareerIntroScreen] ✅ Saved to SaveManager")
		else:
			print("[CareerIntroScreen] ⚠️ SaveManager not found")

		# 팀 창단 화면으로 이동 (Initial Setup Flow)
		print("[CareerIntroScreen] 🏟️ Going to Team Creation first...")
		print("[CareerIntroScreen] TEAM_CREATION_SCENE = ", TEAM_CREATION_SCENE)
		_transition_to_scene(TEAM_CREATION_SCENE)
	else:
		# 기존 게임 로드
		print("[CareerIntroScreen] 📂 Loading game from slot ", slot_number)

		# Load save data
		if has_node("/root/SaveManager"):
			var save_manager = get_node("/root/SaveManager")
			var slot_id = "slot_%d" % (slot_number - 1)  # slot_0 or slot_1

			# ✅ 현재 슬롯 설정 (저장 시 필요)
			save_manager.current_save_slot = slot_id
			print("[CareerIntroScreen] ✅ Set current_save_slot to: ", slot_id)

			var loaded_data = save_manager.load_game(slot_id)

			if loaded_data and loaded_data.has("character_data"):
				# ✅ MyTeamManager에 로드된 캐릭터 데이터 적용
				if has_node("/root/MyTeamManager"):
					var team_manager = get_node("/root/MyTeamManager")
					team_manager.main_character = loaded_data["character_data"]
					print("[CareerIntroScreen] ✅ Loaded character to MyTeamManager")

				# Apply other loaded data to PlayerData if available
				if has_node("/root/PlayerData"):
					var player_data = get_node("/root/PlayerData")
					for key in loaded_data:
						if key in player_data:
							player_data.set(key, loaded_data[key])
					print("[CareerIntroScreen] ✅ Loaded data to PlayerData")
			else:
				print("[CareerIntroScreen] ⚠️ No character_data found in save file")

		# 육성 홈으로 이동
		_transition_to_scene(HOME_IMPROVED_SCENE)


func _on_delete_pressed(slot_number: int):
	"""슬롯 삭제 처리"""
	var data = slot_data["slot" + str(slot_number)]
	if data == null:
		return  # 빈 슬롯은 삭제할 수 없음

	# 확인 팝업 표시
	_show_delete_confirmation(slot_number)


func _show_delete_confirmation(slot_number: int):
	"""삭제 확인 팝업"""
	var dialog = ConfirmationDialog.new()
	dialog.title = "Delete Save"
	dialog.dialog_text = ("Delete save data in Slot %d?\n\n⚠️ This action cannot be undone!" % slot_number)

	# 팝업 스타일링
	dialog.add_theme_font_size_override("title_font_size", 20)

	add_child(dialog)
	dialog.popup_centered(Vector2(400, 200))

	# 확인 시 삭제 실행
	dialog.confirmed.connect(
		func():
			_delete_slot(slot_number)
			dialog.queue_free()
	)

	# 취소 시 그냥 닫기
	dialog.canceled.connect(func(): dialog.queue_free())


func _delete_slot(slot_number: int):
	"""슬롯 데이터 삭제"""
	print("[CareerIntroScreen] Deleting slot ", slot_number)

	# Delete through SaveManager
	if has_node("/root/SaveManager"):
		var save_manager = get_node("/root/SaveManager")
		var slot_id = "slot_%d" % (slot_number - 1)  # slot_0 or slot_1
		save_manager.delete_save(slot_id)

	# 로컬 데이터 업데이트
	slot_data["slot" + str(slot_number)] = null

	# UI 업데이트
	_update_slot_display(slot_number)

	# 삭제 완료 효과
	_play_delete_effect(slot_number)


func _on_slot_hover(slot_number: int, is_hovering: bool):
	"""슬롯 호버 효과"""
	var panel = slot1_panel if slot_number == 1 else slot2_panel
	if not panel:
		return

	var style = panel.get_theme_stylebox("panel")
	if style and style is StyleBoxFlat:
		if is_hovering:
			style.bg_color = style.bg_color.lerp(SLOT_HOVER_COLOR, 0.3)
		else:
			# 원래 색상으로 복구
			var data = slot_data["slot" + str(slot_number)]
			style.bg_color = SLOT_FILLED_COLOR if data else SLOT_EMPTY_COLOR


func _on_back_pressed():
	"""뒤로가기 - 메인 홈으로"""
	print("[CareerIntroScreen] Returning to main home...")
	_transition_to_scene(MAIN_HOME_SCENE)


# ============================================================================
# 애니메이션 및 효과
# ============================================================================


func _play_entrance_animation():
	"""진입 애니메이션"""
	modulate.a = 0
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate:a", 1.0, 0.3)


func _play_delete_effect(slot_number: int):
	"""삭제 효과 애니메이션"""
	var panel = slot1_panel if slot_number == 1 else slot2_panel
	if not panel:
		return

	var tween = get_tree().create_tween()
	tween.tween_property(panel, "modulate", Color(1, 0.3, 0.3, 1), 0.1)
	tween.tween_property(panel, "modulate", Color.WHITE, 0.2)

	# 삭제 완료 메시지
	_show_toast("Slot %d deleted" % slot_number)


func _show_toast(message: String):
	"""간단한 토스트 메시지"""
	var toast = Label.new()
	toast.text = message
	toast.add_theme_font_size_override("font_size", 16)
	toast.modulate = Color(1, 1, 1, 0)

	# 중앙 상단에 위치
	toast.set_anchors_preset(Control.PRESET_CENTER_TOP)
	toast.position.y = 50

	add_child(toast)

	# 페이드 인/아웃
	var tween = get_tree().create_tween()
	tween.tween_property(toast, "modulate:a", 1.0, 0.2)
	tween.tween_property(toast, "modulate:a", 1.0, 1.0)  # 유지
	tween.tween_property(toast, "modulate:a", 0.0, 0.3)
	tween.tween_callback(toast.queue_free)


func _transition_to_scene(scene_path: String):
	"""씬 전환"""
	print("[CareerIntroScreen] _transition_to_scene called with: ", scene_path)
	print("[CareerIntroScreen] Creating tween for fade out...")
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate:a", 0.0, 0.3)
	tween.tween_callback(
		func():
			print("[CareerIntroScreen] Tween finished, changing scene...")
			var result = get_tree().change_scene_to_file(scene_path)
			print("[CareerIntroScreen] Scene change result: ", result)
			if result != OK:
				print("[CareerIntroScreen] ❌ ERROR: Scene change failed! Error code: ", result)
	)
