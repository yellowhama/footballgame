extends Control
# HomeImproved - 개선된 홈 화면

# UI 요소들 - 실제 경로 사용
# 플레이어 정보 표시
@onready
var player_name_label: Label = $ScrollContainer/MainContent/TopSection/PlayerCard/VBox/PlayerHeader/PlayerInfo/Name
@onready var week_info_label: Label = $WeeklyBar/Content/WeekInfo/WeekLabel

# Buttons - 실제 경로
@onready var training_button: Button = $"ScrollContainer/MainContent/QuickActions/VBox/Buttons/TrainingButton"
@onready var rest_button: Button = $"ScrollContainer/MainContent/QuickActions/VBox/Buttons/RestButton"
@onready var go_out_button: Button = $"ScrollContainer/MainContent/QuickActions/VBox/Buttons/GoOutButton"
@onready var status_button: Button = $"BottomBar/ButtonContainer/StatusButton"
@onready var advance_button: Button = $"BottomBar/ButtonContainer/AdvanceButton"
@onready var save_button: Button = $"BottomBar/ButtonContainer/SaveButton"
@onready var match_history_button: Button = $"BottomBar/ButtonContainer/MatchHistoryButton"
@onready var bottom_bar_container: Container = $"BottomBar/ButtonContainer"
@onready var quick_actions_container: Container = $"ScrollContainer/MainContent/QuickActions/VBox/Buttons"

# QuickBar support
var quickbar: QuickBar

# Event system support
var event_dialogue_screen: EventDialogueScreen = null

# Activity visual feedback
var activity_feedback: ActivityVisualFeedback = null


func _ready():
	print("==================================================")
	print("🏠🏠🏠 HOME SCREEN LOADED 🏠🏠🏠")
	print("==================================================")
	print("[HomeImproved] Initializing...")

	# @onready 변수들 확인
	print("Player name label: ", player_name_label)
	print("Week info label: ", week_info_label)

	# ColorSystem 적용 (안전하게)
	# 현재는 주석처리 (static 함수 호출 방식 수정 필요)
	# SceneColorUpdater.apply_color_system_to_scene(self)
	print("SceneColorUpdater skipped - static call needed")

	# 반응형 레이아웃 수정 (안전하게)
	# 현재는 주석처리 (static 함수 호출 방식 수정 필요)
	# ResponsiveLayoutFixer.fix_scene_layout(self)
	print("ResponsiveLayoutFixer skipped - static call needed")

	# 터치 피드백 적용 (안전하게)
	# 현재는 주석처리 (static 함수 호출 방식 수정 필요)
	# TouchFeedback.apply_to_all_buttons(self)
	print("TouchFeedback skipped - static call needed")

	# 버튼 연결
	_connect_buttons()

	# 매니저 신호 연결
	if TrainingManager:
		TrainingManager.rest_activity_completed.connect(_on_rest_activity_completed)
		TrainingManager.go_out_activity_completed.connect(_on_go_out_activity_completed)
	if GameManager:
		GameManager.team_training_executed.connect(_on_team_training_executed)

	# QuickBar 초기화
	_initialize_quickbar()

	# Event dialogue screen 초기화
	_initialize_event_dialogue()

	# Activity feedback 초기화
	_initialize_activity_feedback()

	# UI 업데이트
	_update_ui()

	# Redirect MVP flows to WeekHub for the new loop
	if DateManager and DateManager.mvp_mode_enabled:
		call_deferred("_go_to_weekhub")


func _go_to_weekhub() -> void:
	var target_scene := "res://scenes/mvp/WeekHub.tscn"
	if is_inside_tree() and get_tree() != null:
		var result := get_tree().change_scene_to_file(target_scene)
		if result != OK:
			print("[HomeImproved] ❌ Failed to redirect to WeekHub (error %d)" % result)
	else:
		print("[HomeImproved] ❌ Cannot redirect to WeekHub – tree unavailable")

	print("[HomeImproved] Initialization complete")


func _connect_buttons():
	print("[HomeImproved] Connecting buttons...")

	# MyTeam과 멀티플레이어 버튼 추가 (동적 생성)
	_add_new_buttons()

	# 대시보드 버튼 추가
	_add_dashboard_button()

	# Phase 10: History 버튼 추가
	_add_history_button()

	# 가챠 버튼 추가
	_add_gacha_button()

	# 전술 버튼 추가
	_add_tactics_button()

	# 설정 버튼 추가
	_add_settings_button()

	# 명예의 전당 버튼 추가
	_add_hall_of_fame_button()

	var connected_count = 0

	# 버튼 연결
	if training_button:
		training_button.pressed.connect(_on_training_pressed)
		connected_count += 1

	if rest_button:
		rest_button.pressed.connect(_on_rest_pressed)
		connected_count += 1

	if go_out_button:
		go_out_button.pressed.connect(_on_go_out_pressed)
		connected_count += 1

	if status_button:
		status_button.pressed.connect(_on_status_pressed)
		connected_count += 1

	if advance_button:
		advance_button.pressed.connect(_on_advance_pressed)
		connected_count += 1

	if save_button:
		save_button.pressed.connect(_on_save_pressed)
		connected_count += 1

	if match_history_button:
		match_history_button.pressed.connect(_on_match_history_pressed)
		connected_count += 1

	print("Total buttons connected: ", connected_count)


func _add_dashboard_button():
	"""대시보드 버튼 추가"""
	if not bottom_bar_container:
		print("[HomeImproved] ❌ BottomBar not found for dashboard button")
		return

	var dashboard_btn = Button.new()
	dashboard_btn.text = "🏠 대시보드"
	dashboard_btn.custom_minimum_size = Vector2(100, 60)
	dashboard_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	dashboard_btn.pressed.connect(_on_dashboard_pressed)

	# 맨 앞에 추가
	bottom_bar_container.call_deferred("add_child", dashboard_btn)
	bottom_bar_container.call_deferred("move_child", dashboard_btn, 0)
	print("✅ Dashboard button added")


func _on_dashboard_pressed():
	"""대시보드 화면으로 이동"""
	print("[HomeImproved] Dashboard button pressed")
	var scene_path = "res://scenes/screens/DashboardScreen.tscn"
	if ResourceLoader.exists(scene_path):
		get_tree().change_scene_to_file(scene_path)
	else:
		_show_notification("대시보드 화면 준비 중", Color(1, 0.8, 0.3, 1))


func _add_history_button():
	"""Phase 10: History 버튼 추가"""
	if not bottom_bar_container:
		print("[HomeImproved] ❌ BottomBar not found for history button")
		return

	var history_btn = Button.new()
	history_btn.text = "📊 기록"
	history_btn.custom_minimum_size = Vector2(100, 60)
	history_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	history_btn.pressed.connect(_on_history_pressed)

	# Status 버튼 다음에 추가
	var status_btn_index = -1
	for i in range(bottom_bar_container.get_child_count()):
		var child = bottom_bar_container.get_child(i)
		if "StatusButton" in child.name:
			status_btn_index = i
			break

	if status_btn_index >= 0:
		bottom_bar_container.call_deferred("add_child", history_btn)
		bottom_bar_container.call_deferred("move_child", history_btn, status_btn_index + 1)
	else:
		bottom_bar_container.call_deferred("add_child", history_btn)

	print("✅ History button added")


func _add_gacha_button():
	"""가챠 버튼 추가"""
	if not bottom_bar_container:
		print("[HomeImproved] ❌ BottomBar not found for gacha button")
		return

	var gacha_btn = Button.new()
	gacha_btn.text = "🎲 가챠"
	gacha_btn.custom_minimum_size = Vector2(100, 60)
	gacha_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	gacha_btn.pressed.connect(_on_gacha_pressed)

	bottom_bar_container.call_deferred("add_child", gacha_btn)
	print("✅ Gacha button added")


func _on_gacha_pressed():
	"""가챠 화면으로 이동"""
	print("[HomeImproved] Gacha button pressed")
	var scene_path = "res://scenes/screens/GachaScreen.tscn"
	if ResourceLoader.exists(scene_path):
		get_tree().change_scene_to_file(scene_path)
	else:
		_show_notification("가챠 화면 준비 중", Color(1, 0.8, 0.3, 1))


func _add_tactics_button():
	"""전술 버튼 추가"""
	if not bottom_bar_container:
		print("[HomeImproved] ❌ BottomBar not found for tactics button")
		return

	var tactics_btn = Button.new()
	tactics_btn.text = "⚽ 전술"
	tactics_btn.custom_minimum_size = Vector2(100, 60)
	tactics_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	tactics_btn.pressed.connect(_on_tactics_pressed)

	bottom_bar_container.call_deferred("add_child", tactics_btn)
	print("✅ Tactics button added")


func _on_tactics_pressed():
	"""전술 화면으로 이동"""
	print("[HomeImproved] Tactics button pressed")
	var scene_path = "res://scenes/screens/TacticsScreen.tscn"
	if ResourceLoader.exists(scene_path):
		get_tree().change_scene_to_file(scene_path)
	else:
		_show_notification("전술 화면 준비 중", Color(1, 0.8, 0.3, 1))


func _add_settings_button():
	"""설정 버튼 추가 (왼쪽 하단)"""
	if not bottom_bar_container:
		print("[HomeImproved] ❌ BottomBar not found for settings button")
		return

	var settings_btn = Button.new()
	settings_btn.text = "⚙️ 설정"
	settings_btn.custom_minimum_size = Vector2(100, 60)
	settings_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	settings_btn.pressed.connect(_on_settings_pressed)

	# 맨 앞에 추가
	bottom_bar_container.call_deferred("add_child", settings_btn)
	bottom_bar_container.call_deferred("move_child", settings_btn, 0)

	print("✅ Settings button added")


func _on_settings_pressed():
	"""설정 버튼 처리 - 팝업 메뉴 표시"""
	print("[HomeImproved] Settings button pressed")

	# 설정 팝업 메뉴
	var popup = PopupMenu.new()
	popup.add_item("💾 저장", 0)
	popup.add_item("🔊 볼륨 설정", 1)
	popup.add_separator()
	popup.add_item("❌ 닫기", 2)

	# 신호 연결
	popup.id_pressed.connect(_on_settings_menu_selected)

	# 화면에 추가 및 표시
	add_child(popup)
	popup.popup_centered(Vector2(300, 200))


func _on_settings_menu_selected(id: int):
	"""설정 메뉴 항목 선택 처리"""
	match id:
		0:  # 저장
			_on_save_pressed()
		1:  # 볼륨 설정
			_show_notification("🔊 볼륨 설정 (준비 중)", Color(0.7, 0.7, 1, 1))
		2:  # 닫기
			pass


func _add_hall_of_fame_button():
	"""명예의 전당 버튼 추가"""
	if not bottom_bar_container:
		print("[HomeImproved] ❌ BottomBar not found for hall of fame button")
		return

	var hof_btn = Button.new()
	hof_btn.text = "🏅 명예의 전당"
	hof_btn.custom_minimum_size = Vector2(120, 60)
	hof_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hof_btn.pressed.connect(_on_hall_of_fame_pressed)

	bottom_bar_container.call_deferred("add_child", hof_btn)
	print("✅ Hall of Fame button added")


func _on_hall_of_fame_pressed():
	"""명예의 전당 화면으로 이동"""
	print("[HomeImproved] Hall of Fame button pressed")
	var scene_path = "res://scenes/ui/HallOfFameScreen.tscn"
	if ResourceLoader.exists(scene_path):
		get_tree().change_scene_to_file(scene_path)
	else:
		_show_notification("명예의 전당 화면 준비 중", Color(1, 0.8, 0.3, 1))


func _add_new_buttons():
	"""MyTeam과 멀티플레이어 버튼 동적 추가"""
	print("[HomeImproved] Adding MyTeam and Multiplayer buttons...")

	# BottomBar/ButtonContainer를 찾기
	var button_container = bottom_bar_container
	if not button_container:
		# 대안: ScrollContainer 내부에 추가
		button_container = quick_actions_container

	if button_container:
		# MyTeam 버튼 생성
		var myteam_btn = Button.new()
		myteam_btn.text = "🏆 My Team"
		myteam_btn.custom_minimum_size = Vector2(100, 60)
		myteam_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		myteam_btn.pressed.connect(_on_myteam_pressed)
		button_container.call_deferred("add_child", myteam_btn)
		print("✅ MyTeam button added")

		# 멀티플레이어 버튼 생성 (비활성화)
		var multi_btn = Button.new()
		multi_btn.text = "🌐 Multiplayer"
		multi_btn.custom_minimum_size = Vector2(100, 60)
		multi_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		multi_btn.disabled = true
		multi_btn.modulate.a = 0.5
		multi_btn.tooltip_text = "Coming Soon!"
		multi_btn.pressed.connect(_on_multiplayer_pressed)
		button_container.call_deferred("add_child", multi_btn)
		print("✅ Multiplayer button added (disabled)")
	else:
		print("❌ Could not find button container for new buttons")


func _update_ui():
	# 플레이어 정보 업데이트
	if player_name_label:
		if PlayerData:
			player_name_label.text = PlayerData.get_player_name()
		else:
			player_name_label.text = "플레이어 이름"
		print("Player name updated")
	else:
		print("Warning: player_name_label is null")

	if week_info_label:
		if GameManager:
			week_info_label.text = "%d학년 %d주차" % [GameManager.current_year, GameManager.current_week]
		else:
			week_info_label.text = "1학년 1주차"
		print("Week info updated")
	else:
		print("Warning: week_info_label is null")

	# 개인활동 버튼 가시성 제어
	_update_personal_activity_buttons()

	# 진행 버튼 상태 제어
	_update_progress_button()


func _update_personal_activity_buttons():
	"""개인활동 버튼 표시/숨김 제어"""
	var can_do_activity = GameManager and GameManager.can_do_personal_activity()

	# 개인활동 버튼들 찾기 (이미 @onready로 선언됨)
	# var training_btn = ...
	# var rest_btn = ...
	# var go_out_btn = ...

	# 가시성 설정
	if training_button:
		training_button.visible = can_do_activity
	if rest_button:
		rest_button.visible = can_do_activity
	if go_out_button:
		go_out_button.visible = can_do_activity

	print("[HomeImproved] Personal activity buttons visible: ", can_do_activity)


func _update_progress_button():
	"""진행 버튼 활성화/비활성화 제어"""
	if not advance_button:
		return

	var can_progress = GameManager and GameManager.can_progress_week()

	advance_button.disabled = not can_progress

	if can_progress:
		advance_button.text = "▶ 진행"
		advance_button.modulate = Color(1, 1, 1, 1)
	else:
		advance_button.text = "🔒 개인활동 필요"
		advance_button.modulate = Color(0.6, 0.6, 0.6, 1)

	print("[HomeImproved] Progress button enabled: ", can_progress)


func _initialize_quickbar():
	"""QuickBar 초기화 및 신호 연결"""
	if has_node("QuickBar"):
		quickbar = %QuickBar
		if quickbar:
			print("[HomeImproved] QuickBar found, connecting signals...")
			# 신호 연결
			quickbar.skip.connect(_on_quickbar_skip)
			quickbar.toggle_auto.connect(_on_quickbar_auto_toggled)
			quickbar.change_speed.connect(_on_quickbar_speed_changed)
			quickbar.change_highlight.connect(_on_quickbar_highlight_changed)
			quickbar.open_log.connect(_on_quickbar_log_opened)

			# 기본 설정 적용
			var quickbar_vm = {"autoEnabled": false, "currentSpeed": 1, "highlightLevel": 2, "visible": true}
			quickbar.apply_view_model(quickbar_vm)
		else:
			pass  # QuickBar node found but not valid
	# else:
	# QuickBar 없음 (정상 - 일부 씬에서는 필요 없음)


func _initialize_event_dialogue():
	"""EventDialogueScreen 초기화"""
	# Load event dialogue scene
	var event_scene_path = "res://scenes/screens/EventDialogueScreen.tscn"
	if ResourceLoader.exists(event_scene_path):
		var event_scene = load(event_scene_path)
		if event_scene:
			event_dialogue_screen = event_scene.instantiate()
			add_child(event_dialogue_screen)
			event_dialogue_screen.dialogue_closed.connect(_on_event_dialogue_closed)
			print("[HomeImproved] ✅ EventDialogueScreen initialized")
		else:
			print("[HomeImproved] ⚠️ Failed to load EventDialogueScreen")
	else:
		print("[HomeImproved] ⚠️ EventDialogueScreen scene not found")


func _on_event_dialogue_closed():
	"""이벤트 다이얼로그 종료 시 호출"""
	print("[HomeImproved] Event dialogue closed")
	# UI 업데이트
	_update_ui()


func _initialize_activity_feedback():
	"""ActivityVisualFeedback 초기화"""
	var feedback_scene_path = "res://scenes/ui/ActivityVisualFeedback.tscn"
	if ResourceLoader.exists(feedback_scene_path):
		var feedback_scene = load(feedback_scene_path)
		if feedback_scene:
			activity_feedback = feedback_scene.instantiate()
			add_child(activity_feedback)
			activity_feedback.feedback_completed.connect(_on_activity_feedback_completed)
			print("[HomeImproved] ✅ ActivityVisualFeedback initialized")
		else:
			print("[HomeImproved] ⚠️ Failed to load ActivityVisualFeedback")
	else:
		print("[HomeImproved] ⚠️ ActivityVisualFeedback scene not found")


func _on_activity_feedback_completed():
	"""활동 피드백 완료 시 호출"""
	print("[HomeImproved] Activity feedback completed")
	# UI 업데이트
	_update_ui()


# QuickBar 신호 핸들러들
func _on_quickbar_skip():
	"""QuickBar Skip 버튼 처리"""
	print("[HomeImproved] QuickBar Skip pressed")
	# 빠른 진행 (주차 진행)
	_on_advance_pressed()


func _on_quickbar_auto_toggled(enabled: bool):
	"""QuickBar Auto 토글 처리"""
	print("[HomeImproved] QuickBar Auto toggled: ", enabled)
	# Auto 모드 활성화/비활성화 처리
	# 추후 자동 진행 로직 구현


func _on_quickbar_speed_changed(speed: int):
	"""QuickBar 속도 변경 처리"""
	print("[HomeImproved] QuickBar Speed changed to: ", speed)
	# 게임 속도 조절 처리
	# 추후 애니메이션 속도 조절 구현


func _on_quickbar_highlight_changed(level: int):
	"""QuickBar 하이라이트 레벨 변경 처리"""
	print("[HomeImproved] QuickBar Highlight level changed to: ", level)
	# 하이라이트 레벨 처리
	# 추후 이벤트 표시 수준 조절 구현


func _on_quickbar_log_opened():
	"""QuickBar Log 버튼 처리"""
	print("[HomeImproved] QuickBar Log opened")
	# 로그 창 열기
	# 추후 이벤트 로그 시스템 구현


# 팀훈련 버튼 제거됨 - 진행 버튼에서 자동 실행됨
# func _on_team_training_pressed():
# 	(사용자 요청에 따라 제거됨)


func _on_training_pressed():
	print("[HomeImproved] Training button pressed")

	# 주간 활동 가능 체크
	if GameManager and not GameManager.can_do_personal_activity():
		_show_notification("⚠️ 이번 주 개인활동은 이미 완료했습니다", Color(1, 0.5, 0, 1))
		return

	# 씬 파일 존재 확인 - TrainingManager 연동 새 화면
	var scene_path = "res://scenes/screens/TrainingScreen.tscn"
	if ResourceLoader.exists(scene_path):
		print("Scene file exists: ", scene_path)
		# 개인훈련 활동 표시
		if GameManager:
			GameManager.mark_personal_activity("training")
		var result = get_tree().change_scene_to_file(scene_path)
		if result == OK:
			print("Scene change successful")
		else:
			print("ERROR: Scene change failed with code: ", result)
	else:
		print("ERROR: Scene file not found: ", scene_path)


func _on_go_out_pressed():
	print("[HomeImproved] Go out button pressed (Refactored)")
	# The UI's only responsibility is to request the action from the manager.
	# The manager will emit go_out_activity_completed signal on completion.
	TrainingManager.perform_go_out_activity()


func _on_go_out_activity_completed(result: Dictionary):
	"""Handles the result of the go out activity from the TrainingManager."""
	print("[HomeImproved] Go out activity completed signal received.")

	if not result.get("success", false):
		# If the manager determined the action couldn't be performed, show its message.
		_show_notification("⚠️ %s" % result.get("message", "외출을 할 수 없습니다."), Color(1, 0.5, 0, 1))
		_update_ui()
		return

	# Format the success message from the result payload
	var message = "%s\n\n" % result.get("message", "외출 완료!")
	message += "피로도: %.1f → %.1f (%.1f 회복)\n" % [result.fatigue_before, result.fatigue_after, result.fatigue_recovered]
	message += "컨디션: %d → %d" % [result.condition_before, result.condition_after]

	# Use the dedicated visual feedback system to show the result.
	if activity_feedback:
		activity_feedback.show_activity_feedback("go_out", "🎮 외출", message)
	else:
		# Fallback if the visual feedback system isn't available
		_show_notification(message, Color(1.0, 0.7, 0.8))
		_update_ui()

	print("[HomeImproved] Player went out, activity completed via TrainingManager")


func _show_progress_popup(title: String, message: String, icon: String = "⚽"):
	"""진행 중 팝업 표시 (나중에 일러스트/애니메이션 추가 가능)"""
	var popup = AcceptDialog.new()
	popup.title = title
	popup.dialog_text = "\n\n" + icon + "\n\n" + message + "\n\n"

	# 넉넉한 크기 설정 (나중에 일러스트/애니메이션 공간 확보)
	popup.min_size = Vector2(500, 400)

	# 팝업을 화면에 추가
	add_child(popup)

	# 중앙에 표시
	popup.popup_centered()

	# 1.5초 후 자동으로 닫기
	await get_tree().create_timer(1.5).timeout
	popup.queue_free()

	print("[HomeImproved] Progress popup shown: %s" % title)


func _on_advance_pressed():
	print("[HomeImproved] Advance button pressed (Refactored)")
	# The GameManager now handles the core logic for advancing the week,
	# including team training. We await its completion.
	await GameManager.advance_week()

	# After the week is advanced, check for events.
	# This should eventually be moved into the GameManager's flow as well.
	await _check_for_events()

	# If no scene transition was triggered by signals (e.g., from team training),
	# ensure the UI is updated for the new week.
	if get_tree().current_scene == self:
		_update_ui()


func _on_team_training_executed(result: Dictionary):
	"""Handles the result of team training and transitions to the result screen."""
	print("[HomeImproved] Team training executed, transitioning to result screen.")

	var scene_path = "res://scenes/ResultScreenImproved.tscn"
	if ResourceLoader.exists(scene_path):
		get_tree().change_scene_to_file(scene_path)
	else:
		print("[HomeImproved] ⚠️ Result screen not found - UI will just update.")
		_update_ui()


func _on_rest_pressed():
	print("[HomeImproved] Rest button pressed (Refactored)")
	# The UI's only responsibility is to request the action from the manager.
	# The manager is now async and will emit a signal on completion.
	# A visual feedback will be shown by the completion handler.
	TrainingManager.perform_rest_activity()


func _on_rest_activity_completed(result: Dictionary):
	"""Handles the result of the rest activity from the TrainingManager."""
	print("[HomeImproved] Rest activity completed signal received.")

	if not result.get("success", false):
		# If the manager determined the action couldn't be performed, show its message.
		# This is a fallback, as the button should ideally be disabled.
		_show_notification("⚠️ %s" % result.get("message", "휴식을 취할 수 없습니다."), Color(1, 0.5, 0, 1))
		_update_ui()  # Ensure UI is refreshed even on failure
		return

	# Format the success message from the result payload
	var message = "%s\n\n" % result.get("message", "휴식 완료!")
	message += "피로도: %.1f → %.1f (%.1f 회복)\n" % [result.fatigue_before, result.fatigue_after, result.fatigue_recovered]
	message += "컨디션: %d → %d" % [result.condition_before, result.condition_after]

	# Use the dedicated visual feedback system to show the result.
	# This system will emit its own 'feedback_completed' signal, which triggers a UI update.
	if activity_feedback:
		activity_feedback.show_activity_feedback("rest", "😴 휴식", message)
	else:
		# Fallback if the visual feedback system isn't available
		_show_notification(message, Color(0.5, 0.7, 1.0))
		_update_ui()


func _on_status_pressed():
	print("[HomeImproved] Status button pressed")

	# 씬 파일 존재 확인
	var scene_path = "res://scenes/StatusScreenImproved.tscn"
	if ResourceLoader.exists(scene_path):
		print("Scene file exists: ", scene_path)
		var result = get_tree().change_scene_to_file(scene_path)
		if result == OK:
			print("Scene change successful")
		else:
			print("ERROR: Scene change failed with code: ", result)
	else:
		print("ERROR: Scene file not found: ", scene_path)


func _on_save_pressed():
	print("[HomeImproved] Save button pressed")

	# 저장 기능 (현재 슬롯에 저장)
	if has_node("/root/SaveManager"):
		var save_manager = get_node("/root/SaveManager")
		var current_slot = save_manager.current_save_slot
		print("[HomeImproved] Saving to slot: ", current_slot)

		# 저장할 데이터 준비
		var save_data = {}

		# MyTeamManager에서 캐릭터 데이터 가져오기
		if has_node("/root/MyTeamManager"):
			var team_manager = get_node("/root/MyTeamManager")
			save_data["character_data"] = team_manager.main_character
			save_data["player_name"] = team_manager.main_character.get("basic_info", {}).get("name", "Unknown")

		# TODO: 추가 데이터 (주차, 년도 등)
		# save_data["current_year"] = ...
		# save_data["current_week"] = ...
		save_data["save_time"] = Time.get_datetime_string_from_system()

		# 슬롯에 저장
		save_manager.save_game(current_slot, save_data)
		print("[HomeImproved] ✅ Game saved to slot: ", current_slot)

		# 저장 완료 메시지 표시
		_show_save_notification()
	else:
		print("[HomeImproved] ⚠️ SaveManager not found")


func _show_save_notification():
	"""저장 완료 알림 표시"""
	_show_notification("✅ 저장 완료!", Color(0, 1, 0, 1))


func _check_for_events() -> void:
	"""
	이벤트 체크 및 표시.
	비즈니스 로직(이벤트 체크/트리거)은 GameManager.check_and_trigger_events()로 이관됨.
	UI는 다이얼로그 대기만 처리.
	"""
	if not GameManager:
		return

	# GameManager가 이벤트를 체크하고 트리거
	var triggered_events = GameManager.check_and_trigger_events()

	# UI: 각 트리거된 이벤트에 대해 다이얼로그가 닫힐 때까지 대기
	for event in triggered_events:
		if is_instance_valid(event_dialogue_screen) and event_dialogue_screen.has_signal("dialogue_closed"):
			await event_dialogue_screen.dialogue_closed


func _show_notification(text: String, color: Color = Color(1, 1, 1, 1)):
	"""범용 알림 표시"""
	var notif_label = Label.new()
	notif_label.text = text
	notif_label.add_theme_font_size_override("font_size", 24)
	notif_label.position = Vector2(get_viewport().size.x / 2 - 100, 100)
	notif_label.modulate = color
	add_child(notif_label)

	# 2초 후 사라지기
	var tween = get_tree().create_tween()
	tween.tween_interval(1.0)
	tween.tween_property(notif_label, "modulate:a", 0.0, 1.0)
	tween.tween_callback(notif_label.queue_free)


# Load 기능은 현재 SaveManager가 없으므로 제거


func _on_myteam_pressed():
	"""MyTeam 버튼 처리"""
	print("[HomeImproved] MyTeam button pressed")

	# MyTeamManager 상태 확인
	if has_node("/root/MyTeamManager"):
		var myteam = get_node("/root/MyTeamManager")
		myteam.debug_print_team_status()

		# 팝업으로 팀 정보 표시
		var popup = AcceptDialog.new()
		var team_info = "🏆 %s\n\n" % myteam.team_name
		team_info += "📊 현황:\n"
		team_info += "• 1군: %d/%d명\n" % [myteam.first_team.size(), myteam.MAX_FIRST_TEAM]
		team_info += "• 리저브: %d/%d명\n" % [myteam.reserves.size(), myteam.MAX_RESERVES]
		team_info += "\n💼 코칭 스태프:\n"
		team_info += "• 감독: %s\n" % (myteam.current_deck.manager.name if myteam.current_deck.manager else "없음")
		team_info += "• 덱 보너스: x%.2f\n" % myteam.get_deck_bonus()
		team_info += "\n📈 전적:\n"
		team_info += (
			"%d전 %d승 %d무 %d패"
			% [myteam.total_matches_played, myteam.total_wins, myteam.total_draws, myteam.total_losses]
		)

		popup.dialog_text = team_info
		popup.title = "My Team"
		add_child(popup)
		popup.popup_centered(Vector2(400, 500))
	else:
		print("❌ MyTeamManager not found!")
		_show_notification("⚠️ MyTeam 시스템 초기화 중...", Color(1, 0.5, 0, 1))


func _on_history_pressed():
	"""Phase 10: History 버튼 처리"""
	print("[HomeImproved] History button pressed")

	# HistoryScreen으로 이동
	var scene_path = "res://scenes/ui/HistoryScreen.tscn"
	if ResourceLoader.exists(scene_path):
		print("Scene file exists: ", scene_path)
		var result = get_tree().change_scene_to_file(scene_path)
		if result == OK:
			print("Scene change successful")
		else:
			print("ERROR: Scene change failed with code: ", result)
	else:
		print("ERROR: Scene file not found: ", scene_path)
		_show_notification("⚠️ 기록 화면을 찾을 수 없습니다", Color(1, 0.5, 0, 1))


func _on_match_history_pressed():
	"""Match History 버튼 처리"""
	print("[HomeImproved] Match History button pressed")

	# MatchHistoryScreen으로 이동
	var scene_path = "res://scenes/MatchHistoryScreen.tscn"
	if ResourceLoader.exists(scene_path):
		print("Scene file exists: ", scene_path)
		var result = get_tree().change_scene_to_file(scene_path)
		if result == OK:
			print("Scene change successful")
		else:
			print("ERROR: Scene change failed with code: ", result)
	else:
		print("ERROR: Scene file not found: ", scene_path)
		_show_notification("⚠️ 경기 기록 화면을 찾을 수 없습니다", Color(1, 0.5, 0, 1))


func _on_multiplayer_pressed():
	"""멀티플레이어 버튼 처리 (비활성화 상태)"""
	print("[HomeImproved] Multiplayer button pressed")

	# Coming Soon 팝업
	var popup = AcceptDialog.new()
	popup.dialog_text = """🌐 멀티플레이어 모드

공개 예정!

📅 예정된 기능:
• MyTeam 리그 - PvP 비동기 대전
• 싱글 리그 - NPC 팀과 경쟁
• 1대1 모드 - 하프코트 실시간

🏆 시즌제 운영
• 월간 시즌 보상
• 랭킹 시스템
• 특별 이벤트

조금만 기다려 주세요!"""

	popup.title = "Multiplayer - Coming Soon!"
	add_child(popup)
	popup.popup_centered(Vector2(400, 400))
