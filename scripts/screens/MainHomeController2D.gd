extends Control
class_name MainHomeController2D

# ============================================================================
# 간단한 2D 메인 홈 화면 - 복잡한 3D 없이 실용적으로
# ============================================================================

# UI 참조 - 간단한 구조
@onready var player_name: Label = $HeaderBar/HeaderContent/PlayerSection/PlayerInfo/PlayerName
@onready var player_level: Label = $HeaderBar/HeaderContent/PlayerSection/PlayerInfo/PlayerLevel
@onready var gold_amount: Label = $HeaderBar/HeaderContent/CurrencySection/GoldPanel/GoldAmount
@onready var gem_amount: Label = $HeaderBar/HeaderContent/CurrencySection/GemPanel/GemAmount

@onready var character_display: Control = $CharacterDisplay
@onready var character_sprite: TextureRect = $CharacterDisplay/CharacterSprite
@onready var character_placeholder: Label = $CharacterDisplay/CharacterSprite/CharacterPlaceholder
@onready var interaction_area: Control = $CharacterDisplay/InteractionArea
@onready var speech_bubble: PanelContainer = $CharacterDisplay/SpeechBubble
@onready var speech_text: Label = $CharacterDisplay/SpeechBubble/SpeechText

@onready var career_button: Button = $GameModeButtons/CareerButton
@onready var team_button: Button = $GameModeButtons/TeamButton
@onready var shop_button: Button = $GameModeButtons/ShopButton
@onready var quest_button: Button = $GameModeButtons/QuestButton
@onready var multiplayer_button: Button = $GameModeButtons/MultiplayerButton

@onready var myteam_info: Label = $GameInfoCards/MyTeamCard/HBox/VBox/Info
@onready var notice_info: Label = $GameInfoCards/NoticeCard/HBox/VBox/Info
@onready var achievement_info: Label = $GameInfoCards/AchievementCard/HBox/VBox/Info

# Scene 경로들
const CAREER_SCENE = "res://scenes/CareerIntroScreen.tscn"
const TEAM_SCENE = "res://scenes/MyTeamScreen.tscn"
const SHOP_SCENE = "res://scenes/ShopScreenImproved.tscn"
const QUEST_SCENE = "res://scenes/ui/QuestLogScreen.tscn"

# ============================================================================
# 초기화
# ============================================================================


func _ready():
	print("\n[MainHomeController2D] ✅ NEW 2D VERSION LOADING!")

	_apply_theme_styles()
	_setup_buttons()
	_update_player_info()
	_update_character()
	_update_game_info()
	_setup_interaction()


func _apply_theme_styles():
	"""ThemeManager 스타일 일괄 적용"""
	# 배경색 적용
	var bg = $Background
	if bg:
		bg.color = ThemeManager.BG_PRIMARY

	# 헤더바 스타일
	var header = $HeaderBar
	if header:
		var header_style = ThemeManager.create_header_style()
		header.add_theme_stylebox_override("panel", header_style)

	# 헤더 텍스트 색상
	if player_name:
		player_name.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
		player_name.add_theme_font_size_override("font_size", ThemeManager.FONT_H3)
	if player_level:
		player_level.add_theme_color_override("font_color", ThemeManager.ACCENT)
		player_level.add_theme_font_size_override("font_size", ThemeManager.FONT_CAPTION)

	# 게임 모드 버튼들에 ThemeManager 스타일 적용
	_apply_game_mode_button_styles()

	# Info 카드들 스타일 적용
	_apply_info_card_styles()


func _apply_game_mode_button_styles():
	"""게임 모드 버튼 스타일 적용"""
	var mode_buttons = [career_button, team_button, shop_button, quest_button]
	for button in mode_buttons:
		if button:
			var style = ThemeManager.get_button_style("secondary")
			ThemeManager.apply_button_style(button, style)

			# 버튼 내부 라벨 색상 조정
			var title = button.get_node_or_null("HBox/VBox/Title")
			var desc = button.get_node_or_null("HBox/VBox/Desc")
			if title:
				title.add_theme_color_override("font_color", ThemeManager.ACCENT)
				title.add_theme_font_size_override("font_size", ThemeManager.FONT_H3)
			if desc:
				desc.add_theme_color_override("font_color", ThemeManager.TEXT_SECONDARY)
				desc.add_theme_font_size_override("font_size", ThemeManager.FONT_MICRO)

	# 멀티플레이어 버튼은 비활성 스타일
	if multiplayer_button:
		var disabled_style = ThemeManager.create_button_stylebox(ThemeManager.BG_TERTIARY)
		multiplayer_button.add_theme_stylebox_override("normal", disabled_style)
		multiplayer_button.add_theme_stylebox_override("disabled", disabled_style)


func _apply_info_card_styles():
	"""Info 카드 스타일 적용"""
	var cards = $GameInfoCards
	if not cards:
		return

	for card in cards.get_children():
		if card is PanelContainer:
			var style = ThemeManager.create_card_style()
			card.add_theme_stylebox_override("panel", style)

			# 카드 내부 라벨 스타일
			var title = card.get_node_or_null("HBox/VBox/Title")
			var info = card.get_node_or_null("HBox/VBox/Info")
			if title:
				title.add_theme_color_override("font_color", ThemeManager.ACCENT)
				title.add_theme_font_size_override("font_size", ThemeManager.FONT_CAPTION)
			if info:
				info.add_theme_color_override("font_color", ThemeManager.TEXT_SECONDARY)
				info.add_theme_font_size_override("font_size", ThemeManager.FONT_MICRO)

	# 말풍선 스타일
	if speech_bubble:
		var bubble_style = ThemeManager.create_card_style()
		bubble_style.bg_color = ThemeManager.BG_SECONDARY
		speech_bubble.add_theme_stylebox_override("panel", bubble_style)
	if speech_text:
		speech_text.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
		speech_text.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)

	# 재화 섹션 스타일
	if gold_amount:
		gold_amount.add_theme_color_override("font_color", ThemeManager.TEXT_HIGHLIGHT)
		gold_amount.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)
	if gem_amount:
		gem_amount.add_theme_color_override("font_color", ThemeManager.ACCENT)
		gem_amount.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)

	# 캐릭터 플레이스홀더
	if character_placeholder:
		character_placeholder.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)


func _setup_buttons():
	"""버튼 연결"""
	career_button.pressed.connect(_on_career_pressed)
	team_button.pressed.connect(_on_team_pressed)
	shop_button.pressed.connect(_on_shop_pressed)
	quest_button.pressed.connect(_on_quest_pressed)
	multiplayer_button.pressed.connect(_on_multiplayer_pressed)

	# 버튼 호버 효과
	for button in [career_button, team_button, shop_button, quest_button]:
		button.mouse_entered.connect(func(): _on_button_hover(button, true))
		button.mouse_exited.connect(func(): _on_button_hover(button, false))


func _setup_interaction():
	"""캐릭터 상호작용"""
	interaction_area.gui_input.connect(_on_character_touched)


# ============================================================================
# UI 업데이트
# ============================================================================


func _update_player_info():
	"""플레이어 정보 업데이트"""
	if PlayerData:
		player_name.text = PlayerData.player_name
		if PlayerData.has_method("get_overall_rating"):
			var overall = PlayerData.get_overall_rating()
			player_level.text = "Lv.%d" % overall
	else:
		player_name.text = "플레이어"
		player_level.text = "Lv.80"

	# 재화 (임시 값)
	gold_amount.text = "2,500"
	gem_amount.text = "150"


func _update_character():
	"""2D 캐릭터 표시"""
	# 나중에 실제 스프라이트로 교체
	# character_sprite.texture = load("res://sprites/player.png")

	# 현재는 이모지 플레이스홀더
	character_placeholder.text = "⚽\n선수"

	# 간단한 idle 애니메이션
	var tween = get_tree().create_tween()
	tween.set_loops(1)
	tween.tween_property(character_placeholder, "scale", Vector2(1.05, 1.05), 2.0)
	tween.tween_property(character_placeholder, "scale", Vector2(1.0, 1.0), 2.0)


func _update_game_info():
	"""게임 전체 정보 카드 업데이트"""
	# MyTeam 정보 - 육성완료된 선수 수
	if MyTeamData and "saved_players" in MyTeamData:
		var count = MyTeamData.saved_players.size()
		if count > 0:
			myteam_info.text = "%d명 보유" % count
		else:
			myteam_info.text = "선수 없음"
	else:
		myteam_info.text = "0명 보유"

	# 공지사항 - 환영 메시지나 게임 팁
	var notices = ["환영합니다!", "육성부터 시작하세요", "코치 카드를 모아보세요", "팀을 구성해보세요"]
	notice_info.text = notices[randi() % notices.size()]

	# 업적 - 플레이어 진행도
	if PlayerData:
		if PlayerData.has_method("get_overall_rating"):
			var overall = PlayerData.get_overall_rating()
			if overall >= 80:
				achievement_info.text = "엘리트 선수!"
			elif overall >= 60:
				achievement_info.text = "실력자!"
			else:
				achievement_info.text = "성장 중!"
		else:
			achievement_info.text = "새로 시작!"
	else:
		achievement_info.text = "새로 시작!"


# ============================================================================
# 캐릭터 상호작용
# ============================================================================


func _on_character_touched(event: InputEvent):
	"""캐릭터 터치 반응"""
	if event is InputEventMouseButton and event.pressed:
		_play_character_reaction()


func _play_character_reaction():
	"""캐릭터 반응"""
	# 바운스 애니메이션
	var tween = get_tree().create_tween()
	tween.tween_property(character_display, "scale", Vector2(1.1, 1.1), 0.1)
	tween.tween_property(character_display, "scale", Vector2(1.0, 1.0), 0.1)

	# 말풍선 표시
	var messages = ["열심히 훈련할게요!", "화이팅! ⚽", "오늘도 좋은 하루!", "감독님 믿고 있어요!", "최고가 되고 싶어요!"]

	speech_text.text = messages[randi() % messages.size()]
	speech_bubble.visible = true

	# 2초 후 사라짐
	await get_tree().create_timer(2.0).timeout
	speech_bubble.visible = false


# ============================================================================
# 버튼 이벤트
# ============================================================================


func _on_career_pressed():
	"""육성 모드"""
	print("[MainHome2D] 육성 모드 시작")
	get_tree().change_scene_to_file(CAREER_SCENE)


func _on_team_pressed():
	"""팀 관리"""
	print("[MainHome2D] 팀 관리 시작")
	get_tree().change_scene_to_file(TEAM_SCENE)


func _on_shop_pressed():
	"""상점"""
	print("[MainHome2D] 상점 열기")
	get_tree().change_scene_to_file(SHOP_SCENE)


func _on_quest_pressed():
	"""퀘스트"""
	print("[MainHome2D] 퀘스트 화면으로 이동")
	get_tree().change_scene_to_file(QUEST_SCENE)


func _on_multiplayer_pressed():
	"""멀티플레이어 (Coming Soon)"""
	var dialog = AcceptDialog.new()
	dialog.title = "멀티플레이어"
	dialog.dialog_text = "🌐 Coming Soon!\n\n온라인 대전 모드는\n곧 업데이트 예정입니다!"
	dialog.min_size = Vector2(300, 150)
	add_child(dialog)
	dialog.popup_centered()
	dialog.confirmed.connect(dialog.queue_free)


func _on_button_hover(button: Button, hovering: bool):
	"""버튼 호버 효과"""
	var tween = get_tree().create_tween()
	if hovering:
		tween.tween_property(button, "scale", Vector2(1.05, 1.05), 0.1)
	else:
		tween.tween_property(button, "scale", Vector2(1.0, 1.0), 0.1)


# ============================================================================
# ESC 키로 타이틀로
# ============================================================================


func _input(event):
	if event.is_action_pressed("ui_cancel"):
		get_tree().change_scene_to_file("res://scenes/TitleScreenImproved.tscn")
