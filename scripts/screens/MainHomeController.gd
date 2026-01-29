extends Control
class_name MainHomeController
# MainHomeController - 메인 게임 허브 화면
# Uma Musume + Football Manager UI 패턴 융합

# ============================================================================
# UI 요소 참조 (@onready 변수들은 Scene 생성 후 실제 경로에 맞춰 수정 필요)
# ============================================================================

# Header Bar 요소들
@onready var player_name_label: Label = get_node_or_null("HeaderBar/HeaderContent/PlayerSection/PlayerInfo/PlayerName")
@onready var player_level_label: Label = get_node_or_null("HeaderBar/HeaderContent/PlayerSection/PlayerInfo/PlayerLevel")
@onready var currency_label: Label = get_node_or_null("HeaderBar/HeaderContent/CurrencySection/GoldPanel/Gold/Amount")
@onready var gem_label: Label = get_node_or_null("HeaderBar/HeaderContent/CurrencySection/GemPanel/Gem/Amount")
@onready var notification_badge: Control = get_node_or_null("HeaderBar/HeaderContent/CurrencySection/NotificationBadge")

# Character Display 영역
@onready var character_display: Control = get_node_or_null("MainContainer/CharacterDisplay")
@onready var interaction_area: Control = get_node_or_null(
	"MainContainer/CharacterDisplay/CharacterContainer/ViewportContainer/InteractionArea"
)
@onready var character_display_label: Label = get_node_or_null(
	"MainContainer/CharacterDisplay/CharacterContainer/ViewportContainer/InteractionArea/CharacterLabel"
)
@onready var speech_bubble: PanelContainer = get_node_or_null("MainContainer/CharacterDisplay/SpeechBubble")
@onready var speech_text: RichTextLabel = get_node_or_null("MainContainer/CharacterDisplay/SpeechBubble/SpeechText")

# Game Mode 버튼들 (2x2 Grid)
@onready var career_button: Button = get_node_or_null("MainContainer/GameModeSection/GameModeGrid/CareerModeCard")
@onready var team_button: Button = get_node_or_null("MainContainer/GameModeSection/GameModeGrid/TeamModeCard")
@onready var shop_button: Button = get_node_or_null("MainContainer/GameModeSection/GameModeGrid/ShopModeCard")
@onready
var multiplayer_button: Button = get_node_or_null("MainContainer/GameModeSection/GameModeGrid/MultiplayerModeCard")

# Quick Info Cards
@onready var next_training_card: PanelContainer = get_node_or_null(
	"MainContainer/QuickInfoSection/MarginContainer/QuickInfoCards/NextTrainingCard"
)
@onready var recent_match_card: PanelContainer = get_node_or_null(
	"MainContainer/QuickInfoSection/MarginContainer/QuickInfoCards/RecentMatchCard"
)
@onready var events_card: PanelContainer = get_node_or_null(
	"MainContainer/QuickInfoSection/MarginContainer/QuickInfoCards/EventsCard"
)

# Bottom Navigation
@onready var home_button: Button = get_node_or_null("BottomNavigation/NavigationButtons/HomeButton")
@onready var profile_button: Button = get_node_or_null("BottomNavigation/NavigationButtons/ProfileButton")
@onready var events_button: Button = get_node_or_null("BottomNavigation/NavigationButtons/EventsButton")
@onready var more_button: Button = get_node_or_null("BottomNavigation/NavigationButtons/MoreButton")

# QuickBar support
var quickbar: QuickBar

# ============================================================================
# 게임 모드 Scene 경로들
# ============================================================================

const CAREER_SCENE = "res://scenes/CareerIntroScreen.tscn"  # Career mode slot selection
const TEAM_SCENE = "res://scenes/MyTeamScreen.tscn"
const SHOP_SCENE = "res://scenes/ShopScreenImproved.tscn"
const MULTIPLAYER_SCENE = "res://scenes/MultiplayerScreen.tscn"  # 추후 생성

# ============================================================================
# 색상 및 스타일 상수 (Uma Musume 스타일)
# ============================================================================

const PRIMARY_GREEN = Color(0.2, 0.7, 0.3, 1)  # 축구장 그린
const ACCENT_GOLD = Color(1.0, 0.8, 0.0, 1)  # 우마무스메 골드
const BACKGROUND_WHITE = Color(0.95, 0.95, 0.95, 1)  # 깔끔한 배경
const BUTTON_NORMAL = Color(1, 1, 1, 0.95)
const BUTTON_HOVER = Color(1, 0.95, 0.8, 1)
const BUTTON_PRESSED = Color(0.9, 0.9, 0.9, 1)

# ============================================================================
# 초기화 및 설정
# ============================================================================


func _ready():
	print("==============================================")
	print("[MainHomeController] LOADING MAIN HOME SCREEN!")
	print("==============================================")
	print("[DEBUG] Scene name: ", get_tree().current_scene.name if get_tree().current_scene else "NO SCENE")
	print("[DEBUG] Self visible: ", visible)
	print("[DEBUG] Self modulate: ", modulate)

	# UI 연결 및 초기화
	_connect_buttons()
	_setup_ui_styles()
	_update_ui()

	# 캐릭터 상호작용 설정
	_setup_character_interaction()

	# QuickBar 초기화
	_initialize_quickbar()

	# 진입 애니메이션
	_play_entrance_animation()

	print("[MainHomeController] Main home screen ready!")
	print("[DEBUG] Final visible state: ", visible)
	print("==============================================")


func _connect_buttons():
	"""모든 버튼 이벤트 연결"""
	print("[MainHomeController] Connecting buttons...")

	var connected_count = 0

	# Game Mode 버튼들
	if career_button:
		career_button.pressed.connect(_on_career_mode_pressed)
		career_button.mouse_entered.connect(func(): _on_button_hover(career_button, true))
		career_button.mouse_exited.connect(func(): _on_button_hover(career_button, false))
		connected_count += 1
		print("✅ Career button connected")
	else:
		print("❌ Career button not found")

	if team_button:
		team_button.pressed.connect(_on_team_mode_pressed)
		team_button.mouse_entered.connect(func(): _on_button_hover(team_button, true))
		team_button.mouse_exited.connect(func(): _on_button_hover(team_button, false))
		connected_count += 1
		print("✅ Team button connected")
	else:
		print("❌ Team button not found")

	if shop_button:
		shop_button.pressed.connect(_on_shop_mode_pressed)
		shop_button.mouse_entered.connect(func(): _on_button_hover(shop_button, true))
		shop_button.mouse_exited.connect(func(): _on_button_hover(shop_button, false))
		connected_count += 1
		print("✅ Shop button connected")
	else:
		print("❌ Shop button not found")

	if multiplayer_button:
		multiplayer_button.pressed.connect(_on_multiplayer_mode_pressed)
		multiplayer_button.mouse_entered.connect(func(): _on_button_hover(multiplayer_button, true))
		multiplayer_button.mouse_exited.connect(func(): _on_button_hover(multiplayer_button, false))
		connected_count += 1
		print("✅ Multiplayer button connected")
	else:
		print("❌ Multiplayer button not found")

	# Character Display 터치
	if interaction_area:
		if interaction_area.has_signal("gui_input"):
			interaction_area.gui_input.connect(_on_character_touched)
			print("✅ Character interaction connected")

	# Bottom Navigation (추후 구현)
	if profile_button:
		profile_button.pressed.connect(_on_profile_pressed)
		connected_count += 1

	print("[MainHomeController] Total buttons connected: ", connected_count)


func _setup_ui_styles():
	"""Uma Musume 스타일 UI 적용"""
	print("[MainHomeController] Setting up UI styles...")

	# Game Mode 버튼들 스타일링
	var mode_buttons = [career_button, team_button, shop_button, multiplayer_button]
	for button in mode_buttons:
		if button:
			_setup_game_mode_button_style(button)

	# Multiplayer 버튼은 Coming Soon 상태로 설정
	if multiplayer_button:
		multiplayer_button.disabled = true
		multiplayer_button.modulate.a = 0.6
		multiplayer_button.tooltip_text = "Coming Soon! 🌐"

	print("[MainHomeController] UI styles applied")


func _setup_game_mode_button_style(button: Button):
	"""게임 모드 버튼에 Uma Musume 스타일 적용"""
	if not button:
		return
	# CustomStyles API를 사용한 Uma Musume 스타일 적용
	var base_style: StyleBoxFlat = null

	if CustomStyles:
		base_style = CustomStyles.create_primary_button()
		# Football Manager 스타일에 맞게 커스터마이징
		base_style.bg_color = BUTTON_NORMAL
		base_style.border_color = PRIMARY_GREEN
		base_style.border_width_left = 3
		base_style.border_width_right = 3
		base_style.border_width_top = 3
		base_style.border_width_bottom = 3
		base_style.corner_radius_top_left = 20
		base_style.corner_radius_top_right = 20
		base_style.corner_radius_bottom_left = 20
		base_style.corner_radius_bottom_right = 20
		print("[MainHomeController] CustomStyles applied to button: ", button.name)
	else:
		# Fallback: 기본 스타일 생성
		base_style = StyleBoxFlat.new()
		base_style.bg_color = BUTTON_NORMAL
		base_style.corner_radius_top_left = 20
		base_style.corner_radius_top_right = 20
		base_style.corner_radius_bottom_left = 20
		base_style.corner_radius_bottom_right = 20
		base_style.border_width_left = 3
		base_style.border_width_right = 3
		base_style.border_width_top = 3
		base_style.border_width_bottom = 3
		base_style.border_color = PRIMARY_GREEN
		print("[MainHomeController] Warning: CustomStyles not available, using fallback")

	# 기본 스타일 적용
	button.add_theme_stylebox_override("normal", base_style)

	# 호버 스타일
	var hover_style = base_style.duplicate()
	hover_style.bg_color = BUTTON_HOVER
	hover_style.border_color = ACCENT_GOLD
	hover_style.shadow_size = 12
	button.add_theme_stylebox_override("hover", hover_style)

	# 눌림 스타일
	var pressed_style = base_style.duplicate()
	pressed_style.bg_color = BUTTON_PRESSED
	pressed_style.shadow_size = 4
	pressed_style.shadow_offset = Vector2(0, 2)
	button.add_theme_stylebox_override("pressed", pressed_style)


func _setup_character_interaction():
	"""캐릭터 상호작용 영역 설정"""
	if interaction_area:
		# 터치 영역 활성화
		interaction_area.mouse_filter = Control.MOUSE_FILTER_STOP
		print("[MainHomeController] Character interaction area setup complete")


func _initialize_quickbar():
	"""QuickBar 초기화 및 신호 연결"""
	if has_node("QuickBar"):
		quickbar = %QuickBar
		if quickbar:
			print("[MainHomeController] QuickBar found, connecting signals...")
			# 신호 연결 - MainHome에서는 LOG만 사용
			quickbar.open_log.connect(_on_quickbar_log_opened)

			# MainHome 전용 설정
			var quickbar_vm = {"visible": true, "position": "top-right"}
			quickbar.apply_view_model(quickbar_vm)
			print("[MainHomeController] QuickBar initialized for MainHome")
		else:
			print("[MainHomeController] QuickBar node found but not valid")
	else:
		print("[MainHomeController] QuickBar node not found in scene")


func _on_quickbar_log_opened():
	"""QuickBar Log 버튼 처리"""
	print("[MainHomeController] QuickBar Log opened")
	# 간단한 로그 팝업 표시
	_show_coming_soon_popup("게임 로그", "게임 진행 로그 시스템\n추후 업데이트 예정!")


# ============================================================================
# UI 업데이트 및 정보 표시
# ============================================================================


func _update_ui():
	"""모든 UI 요소 업데이트"""
	_update_player_info()
	_update_character_display()
	_update_quick_cards()
	_update_notifications()


func _update_player_info():
	"""Header의 플레이어 정보 업데이트"""
	if player_name_label:
		if PlayerData:
			player_name_label.text = PlayerData.player_name
		else:
			player_name_label.text = "플레이어"  # Fallback

	if player_level_label:
		if PlayerData:
			var overall = PlayerData.get_overall_rating()
			player_level_label.text = "Lv." + str(overall)
		else:
			player_level_label.text = "Lv.80"  # Fallback

	if currency_label:
		# 임시로 고정값, 추후 실제 재화 시스템 연동
		currency_label.text = "2,500"

	print(
		"[MainHomeController] Player info updated: ",
		player_name_label.text if player_name_label else "N/A",
		" ",
		player_level_label.text if player_level_label else "N/A"
	)


func _update_character_display():
	"""2D 캐릭터 표시 업데이트"""
	if not character_display_label:
		print("[MainHomeController] Character display label not found")
		return

	# Default display
	var face_emoji = "😀"
	var body_emoji = "👕"

	# GlobalCharacterData에서 appearance 데이터 로드
	if GlobalCharacterData and GlobalCharacterData.character_data.has("appearance"):
		var appearance = GlobalCharacterData.character_data.appearance

		# Face preset (0-5)
		var faces = ["😀", "😄", "😎", "🤩", "😐", "🤔"]
		var face_preset = appearance.get("face_preset", 0)
		if face_preset >= 0 and face_preset < faces.size():
			face_emoji = faces[face_preset]

		# Body type (0-2)
		var body_type = appearance.get("body_type", 1)
		body_emoji = _get_body_emoji(body_type)

		print("[MainHomeController] Character display updated from GlobalCharacterData")
		print("  Face: ", face_emoji, " Body: ", body_emoji)
	else:
		print("[MainHomeController] Using default character appearance")

	# 캐릭터 표시
	character_display_label.text = face_emoji + "\n" + body_emoji

	# 간단한 호흡 애니메이션
	_play_character_idle_animation()


func _get_body_emoji(body_type: int) -> String:
	"""체형에 따른 바디 이모지 반환"""
	match body_type:
		0:  # 마른 체형
			return "🎽"
		1:  # 보통 체형
			return "👕"
		2:  # 건장한 체형
			return "💪"
		_:
			return "👕"  # 기본값


func _play_character_idle_animation():
	"""캐릭터 idle 애니메이션 (호흡 효과)"""
	if not character_display_label:
		return

	# 부드러운 스케일 애니메이션
	var tween = get_tree().create_tween()
	tween.set_loops(1)
	tween.tween_property(character_display_label, "scale", Vector2(1.05, 1.05), 2.0)
	tween.tween_property(character_display_label, "scale", Vector2(1.0, 1.0), 2.0)


func _update_quick_cards():
	"""Quick Info Cards 실제 데이터 업데이트"""
	_update_training_card()
	_update_match_card()
	_update_events_card()
	print("[MainHomeController] Quick cards updated with real data")


func _update_training_card():
	"""Next Training Card 업데이트"""
	if not next_training_card:
		return

	var training_label = next_training_card.get_node_or_null("VBox/Title")
	var training_info = next_training_card.get_node_or_null("VBox/Info")

	if training_label:
		training_label.text = "Next Training"

	if training_info:
		# GameManager 또는 PlayerData에서 실제 데이터 가져오기
		var info_text = "Schedule: "

		if GameManager and GameManager.has_method("get_current_week"):
			var current_year = GameManager.current_year if "current_year" in GameManager else 1
			var current_week = GameManager.current_week if "current_week" in GameManager else 1
			var next_week = current_week + 1
			info_text += "Year %d Week %d" % [current_year, next_week]
		elif PlayerData:
			var current_year = PlayerData.current_year if "current_year" in PlayerData else 1
			var current_week = PlayerData.current_week if "current_week" in PlayerData else 1
			var next_week = current_week + 1
			info_text += "Year %d Week %d" % [current_year, next_week]
		else:
			info_text += "Year 1 Week 2"  # Fallback

		training_info.text = info_text


func _update_match_card():
	"""Recent Match Card 업데이트"""
	if not recent_match_card:
		return

	var match_label = recent_match_card.get_node_or_null("VBox/Title")
	var match_info = recent_match_card.get_node_or_null("VBox/Info")

	if match_label:
		match_label.text = "Recent Match"

	if match_info:
		# 추후 MatchHistory에서 실제 데이터 가져오기
		# 현재는 PlayerData 기반 간단한 정보 표시
		if PlayerData and PlayerData.has_method("get_overall_rating"):
			var overall = PlayerData.get_overall_rating()
			if overall >= 70:
				match_info.text = "Victory 2-1 ⚽"
			elif overall >= 50:
				match_info.text = "Draw 1-1 ⚖️"
			else:
				match_info.text = "Defeat 0-2 😔"
		else:
			match_info.text = "No matches yet"


func _update_events_card():
	"""Events Card 업데이트"""
	if not events_card:
		return

	var event_label = events_card.get_node_or_null("VBox/Title")
	var event_info = events_card.get_node_or_null("VBox/Info")

	if event_label:
		event_label.text = "Events"

	if event_info:
		# MyTeamData에서 선수 수 확인해서 이벤트 알림 표시
		var info_text = ""

		if MyTeamData:
			var saved_players = MyTeamData.saved_players if MyTeamData.saved_players != null else []
			var saved_players_count = saved_players.size()
			if saved_players_count > 0:
				info_text = "🎉 %d players in My Team!" % saved_players_count
			else:
				info_text = "Complete Career Mode to unlock!"
		else:
			info_text = "No special events"

		event_info.text = info_text


func _update_notifications():
	"""알림 배지 업데이트"""
	if notification_badge:
		# 임시로 숨김, 추후 실제 알림 시스템 연동
		notification_badge.visible = false


# ============================================================================
# 네비게이션 및 Scene 전환
# ============================================================================


func _on_career_mode_pressed():
	"""육성 모드 버튼 처리"""
	print("[MainHomeController] Career mode selected")
	_transition_to_scene(CAREER_SCENE, "🏃‍♂️ 육성 모드")


func _on_team_mode_pressed():
	"""팀 관리 버튼 처리"""
	print("[MainHomeController] Team mode selected")
	_transition_to_scene(TEAM_SCENE, "⚽ 팀 관리")


func _on_shop_mode_pressed():
	"""상점 버튼 처리"""
	print("[MainHomeController] Shop mode selected")
	_transition_to_scene(SHOP_SCENE, "🛒 상점")


func _on_multiplayer_mode_pressed():
	"""멀티플레이어 버튼 처리 (Coming Soon)"""
	print("[MainHomeController] Multiplayer mode selected")
	_show_coming_soon_popup(
		"멀티플레이어",
		"""🌐 멀티플레이어 모드

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
	)


func _transition_to_scene(scene_path: String, mode_name: String):
	"""Scene 전환 처리 (애니메이션 포함)"""
	# Scene 파일 존재 확인
	if not ResourceLoader.exists(scene_path):
		print("[MainHomeController] ERROR: Scene file not found: ", scene_path)
		_show_error_popup("Scene 파일을 찾을 수 없습니다", scene_path)
		return

	print("[MainHomeController] Transitioning to %s..." % mode_name)

	# 페이드 아웃 애니메이션
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate:a", 0.0, 0.3)
	tween.tween_callback(
		func():
			var result = get_tree().change_scene_to_file(scene_path)
			if result != OK:
				print("[MainHomeController] ERROR: Scene change failed with code: ", result)
	)


func _show_coming_soon_popup(title: String, content: String):
	"""Coming Soon 팝업 표시 (Back 버튼 포함)"""
	var popup = AcceptDialog.new()
	popup.title = title + " - Coming Soon!"
	popup.dialog_text = content

	# Back 버튼 텍스트 커스터마이징
	popup.get_ok_button().text = "돌아가기"
	popup.get_ok_button().add_theme_font_size_override("font_size", 16)

	# 팝업 스타일링
	popup.add_theme_font_size_override("title_font_size", 20)
	popup.min_size = Vector2(450, 400)

	add_child(popup)
	popup.popup_centered()

	# 팝업이 닫히면 자동 제거
	popup.confirmed.connect(popup.queue_free)
	popup.canceled.connect(popup.queue_free)

	print("[MainHomeController] Showing Coming Soon popup for: %s" % title)


func _show_error_popup(title: String, details: String):
	"""에러 팝업 표시"""
	var popup = AcceptDialog.new()
	popup.title = "⚠️ " + title
	popup.dialog_text = "오류가 발생했습니다:\n" + details
	add_child(popup)
	popup.popup_centered(Vector2(400, 200))
	popup.confirmed.connect(popup.queue_free)


# ============================================================================
# 캐릭터 상호작용 (Uma Musume 스타일)
# ============================================================================


func _on_character_touched(event: InputEvent):
	"""캐릭터 터치 시 상호작용 (음성, 애니메이션)"""
	if event is InputEventMouseButton and event.pressed:
		print("[MainHomeController] Character touched!")
		_play_character_interaction()


func _play_character_interaction():
	"""캐릭터 상호작용 실행"""
	# 바운스 애니메이션
	if character_display:
		var original_scale = character_display.scale
		var tween = get_tree().create_tween()
		tween.set_parallel(true)
		tween.tween_property(character_display, "scale", original_scale * 1.1, 0.1)
		tween.tween_property(character_display, "scale", original_scale, 0.2).set_delay(0.1)

	# 축구공 이펙트 (홈이니프로브드에서 가져온 패턴)
	_spawn_ball_effect(character_display.global_position + character_display.size / 2)

	# 간단한 음성 메시지 (텍스트로 표시)
	_play_character_voice()


func _spawn_ball_effect(pos: Vector2):
	"""축구공 이펙트 생성"""
	var ball = Label.new()
	ball.text = "⚽"
	ball.add_theme_font_size_override("font_size", 32)
	ball.position = pos
	add_child(ball)

	# 위로 떠오르며 사라지는 애니메이션
	var tween = get_tree().create_tween()
	tween.set_parallel(true)
	tween.tween_property(ball, "position:y", pos.y - 100, 0.5)
	tween.tween_property(ball, "modulate:a", 0.0, 0.5)
	tween.set_parallel(false)
	tween.tween_callback(ball.queue_free)


func _play_character_voice():
	"""캐릭터 음성 (텍스트 버전)"""
	var voices = ["훈련 열심히 할게요! ⚽", "오늘도 좋은 하루네요! 😊", "다음 경기가 기대돼요! 🔥", "항상 응원해주셔서 감사해요! 💪", "더 강해지고 싶어요! ⭐"]

	var random_voice = voices[randi() % voices.size()]
	_show_voice_bubble(random_voice)


func _show_voice_bubble(text: String):
	"""말풍선 효과로 음성 표시"""
	# 새로운 씬 구조의 speech bubble 사용
	if speech_bubble and speech_text:
		speech_text.text = text
		speech_bubble.visible = true
		speech_bubble.modulate = Color.WHITE

		# 3초 후 사라지기
		var tween = get_tree().create_tween()
		tween.tween_interval(2.0)
		tween.tween_property(speech_bubble, "modulate:a", 0.0, 1.0)
		tween.tween_callback(
			func():
				speech_bubble.visible = false
				speech_bubble.modulate.a = 1.0
		)
	else:
		# Fallback to old system
		var bubble = Label.new()
		bubble.text = text
		bubble.add_theme_font_size_override("font_size", 18)
		bubble.position = Vector2(get_viewport().size.x / 2 - 100, 200)
		bubble.modulate = ACCENT_GOLD
		add_child(bubble)

		var tween = get_tree().create_tween()
		tween.tween_interval(2.0)
		tween.tween_property(bubble, "modulate:a", 0.0, 1.0)
		tween.tween_callback(bubble.queue_free)


# ============================================================================
# 애니메이션 및 효과
# ============================================================================


func _on_button_hover(button: Button, is_hovering: bool):
	"""버튼 호버 효과 (Uma Musume 스타일)"""
	if not button:
		return

	if is_hovering:
		# 확대 애니메이션
		var tween = get_tree().create_tween()
		tween.tween_property(button, "scale", Vector2(1.05, 1.05), 0.1).set_ease(Tween.EASE_OUT).set_trans(
			Tween.TRANS_ELASTIC
		)

		# 축구공 이펙트
		_spawn_ball_effect(button.global_position + button.size / 2)
	else:
		# 원래 크기로
		var tween = get_tree().create_tween()
		tween.tween_property(button, "scale", Vector2(1.0, 1.0), 0.2)


func _play_entrance_animation():
	"""화면 진입 애니메이션"""
	# 전체 화면 페이드 인
	modulate.a = 0.0
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate:a", 1.0, 0.5)

	# 버튼들 순차 진입 (HomeImproved 패턴)
	var mode_buttons = [career_button, team_button, shop_button, multiplayer_button]
	var delay = 0.0

	for button in mode_buttons:
		if button:
			button.modulate.a = 0.0
			button.position.x = button.position.x - 50

			var button_tween = get_tree().create_tween()
			button_tween.tween_interval(delay)
			button_tween.set_parallel(true)
			button_tween.tween_property(button, "modulate:a", 1.0, 0.3)
			button_tween.tween_property(button, "position:x", button.position.x + 50, 0.3).set_ease(Tween.EASE_OUT)

			delay += 0.1


# ============================================================================
# Bottom Navigation (추후 확장)
# ============================================================================


func _on_profile_pressed():
	"""프로필 버튼 처리 (추후 구현)"""
	print("[MainHomeController] Profile pressed")
	_show_coming_soon_popup("프로필", "플레이어 프로필 화면\n추후 업데이트 예정!")


# ============================================================================
# 디버그 및 유틸리티
# ============================================================================


func _on_back_to_title():
	"""타이틀로 돌아가기 (ESC 키 등)"""
	print("[MainHomeController] Returning to title screen...")
	get_tree().change_scene_to_file("res://scenes/TitleScreenImproved.tscn")


func _input(event):
	"""키보드 입력 처리"""
	if event.is_action_pressed("ui_cancel"):  # ESC 키
		_on_back_to_title()
