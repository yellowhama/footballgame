extends Control
# EndingScreenImproved - 개선된 엔딩 화면

# UI 요소들
@onready var title_label: Label = $VBox/TitleLabel
@onready var ending_text: RichTextLabel = $VBox/EndingText
@onready var main_menu_button: Button = $VBox/Buttons/MainMenuButton
@onready var new_game_button: Button = $VBox/Buttons/NewGameButton
@onready var save_to_myteam_button: Button = $VBox/Buttons/SaveToMyTeamButton

# 엔딩 데이터
var ending_data: Dictionary = {}
var player_saved_to_myteam: bool = false


func _ready():
	print("[EndingScreenImproved] Initializing...")

	# 버튼 연결
	_connect_buttons()

	# 엔딩 데이터 로드
	_load_ending_data()


func _connect_buttons():
	if main_menu_button:
		main_menu_button.pressed.connect(_on_main_menu_pressed)

	if new_game_button:
		new_game_button.pressed.connect(_on_new_game_pressed)

	if save_to_myteam_button:
		save_to_myteam_button.pressed.connect(_on_save_to_myteam_pressed)


func _load_ending_data():
	# GameManager에서 엔딩 정보 가져오기
	if GameManager:
		ending_data = GameManager.get_ending_result()

	# 기본 엔딩 설정
	if ending_data.is_empty():
		ending_data = {
			"title": "축구 선수로서의 여정",
			"description": "3년간의 고등학교 축구 생활을 마치며, 당신은 훌륭한 선수로 성장했습니다.",
			"final_stats": {"overall_rating": 75}
		}

	# UI 업데이트
	_update_ending_display()


func _update_ending_display():
	if title_label:
		title_label.text = ending_data.get("title", "엔딩")

	if ending_text:
		var description = ending_data.get("description", "게임이 끝났습니다.")
		ending_text.text = description


func _on_main_menu_pressed():
	print("[EndingScreenImproved] Main menu button pressed")
	get_tree().change_scene_to_file("res://scenes/TitleScreenImproved.tscn")


func _on_new_game_pressed():
	print("[EndingScreenImproved] New game button pressed")
	get_tree().change_scene_to_file("res://scenes/CharacterCreateAppearance.tscn")


func _on_save_to_myteam_pressed():
	"""선수를 MyTeam에 저장"""
	if player_saved_to_myteam:
		print("[EndingScreenImproved] Player already saved to MyTeam")
		return

	print("[EndingScreenImproved] Saving player to MyTeam...")

	# PlayerData와 GlobalCharacterData에서 정보 수집
	var player_data = {}

	if PlayerData:
		player_data = {
			"name": PlayerData.player_name,
			"age": PlayerData.age,
			"position": PlayerData.position,
			"overall": PlayerData.get_overall_rating(),
			"technical": PlayerData.technical_stats.duplicate(),
			"mental": PlayerData.mental_stats.duplicate(),
			"physical": PlayerData.physical_stats.duplicate(),
			"goalkeeper": PlayerData.goalkeeper_stats.duplicate()
		}

	# GlobalCharacterData에서 외형 및 특성 정보
	if GlobalCharacterData:
		var char_data = GlobalCharacterData.get_final_character()
		if char_data.size() > 0:
			player_data["appearance"] = char_data.get("appearance", {})
			player_data["strengths"] = char_data.get("strengths", [])
			player_data["weaknesses"] = char_data.get("weaknesses", [])

	# 엔딩 정보 추가
	player_data["ending_type"] = ending_data.get("type", "normal")
	player_data["seasons_played"] = ending_data.get("seasons", 3)

	# 경기 통계 추가 (기본값)
	player_data["total_goals"] = ending_data.get("total_goals", 0)
	player_data["total_assists"] = ending_data.get("total_assists", 0)
	player_data["total_matches"] = ending_data.get("total_matches", 0)

	# MyTeamData에 저장
	if MyTeamData:
		if MyTeamData.save_player_from_career(player_data):
			player_saved_to_myteam = true
			print("[EndingScreenImproved] Player saved successfully!")

			# 버튼 텍스트 변경
			if save_to_myteam_button:
				save_to_myteam_button.text = "저장 완료!"
				save_to_myteam_button.disabled = true

			# 성공 메시지 표시 (간단한 구현)
			_show_save_success_message()
		else:
			print("[EndingScreenImproved] Failed to save player (team might be full)")


func _show_save_success_message():
	"""저장 성공 메시지 표시"""
	# 간단한 메시지 표시 (실제로는 팝업이나 토스트 메시지 사용)
	if ending_text:
		ending_text.text += "\n\n[color=green]✓ 선수가 MyTeam에 저장되었습니다![/color]"
