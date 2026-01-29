extends Control
class_name AIMatchSetupController

## AI Match Setup Controller
##
## Phase 3.2: AI Tactical Manager UI Integration
## Phase 3.3: UI 프리셋 선택 메뉴 통합
##
## 경기 시작 전 AI 난이도와 전술 프리셋을 선택할 수 있는 화면 컨트롤러
##
## 사용법:
##   1. Scene에 AIMatchSetupController를 추가
##   2. home_ai_selector, away_ai_selector를 연결
##   3. home_preset_selector, away_preset_selector를 연결 (Phase 3.3)
##   4. start_match_pressed 시그널을 MatchSessionController에 연결
##
## Rust 연동:
##   - match_plan에 home_ai_difficulty, away_ai_difficulty 추가
##   - match_plan에 home_instructions, away_instructions 추가 (Phase 3.3)

## 시그널
signal match_ready(match_plan: Dictionary)

## UI 노드 참조
@onready var home_ai_selector: AIDifficultySelector = $VBox/HomeAIContainer/HomeAIDifficulty
@onready var away_ai_selector: AIDifficultySelector = $VBox/AwayAIContainer/AwayAIDifficulty
@onready var home_preset_selector: TacticalPresetSelector = $VBox/HomePresetContainer/HomePresetSelector
@onready var away_preset_selector: TacticalPresetSelector = $VBox/AwayPresetContainer/AwayPresetSelector
@onready var difficulty_description: RichTextLabel = $VBox/DescriptionPanel/DescriptionText
@onready var start_button: Button = $VBox/ButtonsContainer/StartButton
@onready var back_button: Button = $VBox/ButtonsContainer/BackButton

## 경기 설정 데이터
var _home_team_data: Dictionary = {}
var _away_team_data: Dictionary = {}
var _formation_home: String = "4-4-2"
var _formation_away: String = "4-4-2"


func _ready():
	_connect_signals()
	_update_description()


## 시그널 연결
func _connect_signals():
	# AI 난이도 선택 (Phase 3.2)
	if home_ai_selector:
		home_ai_selector.difficulty_changed.connect(_on_home_difficulty_changed)

	if away_ai_selector:
		away_ai_selector.difficulty_changed.connect(_on_away_difficulty_changed)

	# 전술 프리셋 선택 (Phase 3.3)
	if home_preset_selector:
		home_preset_selector.preset_changed.connect(_on_home_preset_changed)

	if away_preset_selector:
		away_preset_selector.preset_changed.connect(_on_away_preset_changed)

	# 버튼
	if start_button:
		start_button.pressed.connect(_on_start_button_pressed)

	if back_button:
		back_button.pressed.connect(_on_back_button_pressed)


## 홈팀 AI 난이도 변경 핸들러
func _on_home_difficulty_changed(difficulty: int):
	_update_description()
	print("[AIMatchSetup] Home AI difficulty changed to: %d" % difficulty)


## 원정팀 AI 난이도 변경 핸들러
func _on_away_difficulty_changed(difficulty: int):
	_update_description()
	print("[AIMatchSetup] Away AI difficulty changed to: %d" % difficulty)


## 홈팀 전술 프리셋 변경 핸들러 (Phase 3.3)
func _on_home_preset_changed(preset_id: String):
	_update_description()
	print("[AIMatchSetup] Home preset changed to: %s" % preset_id)


## 원정팀 전술 프리셋 변경 핸들러 (Phase 3.3)
func _on_away_preset_changed(preset_id: String):
	_update_description()
	print("[AIMatchSetup] Away preset changed to: %s" % preset_id)


## 난이도 및 프리셋 설명 업데이트 (Phase 3.2 + 3.3)
func _update_description():
	if not difficulty_description:
		return

	var home_ai_meta = home_ai_selector.get_selected_metadata() if home_ai_selector else {}
	var away_ai_meta = away_ai_selector.get_selected_metadata() if away_ai_selector else {}

	var home_preset_id = home_preset_selector.get_selected_preset() if home_preset_selector else ""
	var away_preset_id = away_preset_selector.get_selected_preset() if away_preset_selector else ""

	var home_preset = TacticalPresets.get_preset(home_preset_id)
	var away_preset = TacticalPresets.get_preset(away_preset_id)

	var description_text = ""

	# 홈팀 정보
	description_text += "[b]홈팀:[/b]\n"

	# AI 난이도
	if not home_ai_meta.is_empty():
		description_text += (
			"  AI: %s %s (%d%% 업데이트)\n"
			% [home_ai_meta.get("icon", ""), home_ai_meta.get("name_ko", ""), home_ai_meta.get("update_chance", 0)]
		)

	# 전술 프리셋
	if not home_preset.is_empty():
		description_text += "  전술: %s %s\n" % [home_preset.get("icon", ""), home_preset.get("name", "")]

	description_text += "\n"

	# 원정팀 정보
	description_text += "[b]원정팀:[/b]\n"

	# AI 난이도
	if not away_ai_meta.is_empty():
		description_text += (
			"  AI: %s %s (%d%% 업데이트)\n"
			% [away_ai_meta.get("icon", ""), away_ai_meta.get("name_ko", ""), away_ai_meta.get("update_chance", 0)]
		)

	# 전술 프리셋
	if not away_preset.is_empty():
		description_text += "  전술: %s %s" % [away_preset.get("icon", ""), away_preset.get("name", "")]

	difficulty_description.text = description_text


## 경기 시작 버튼 핸들러
func _on_start_button_pressed():
	var match_plan = create_match_plan()

	# Phase 3.2 + 3.3: AI 난이도 + 전술 프리셋 출력
	print("[AIMatchSetup] Starting match with:")
	print(
		(
			"  Home AI: %d (%s)"
			% [
				match_plan.home_ai_difficulty,
				home_ai_selector.get_selected_metadata().get("name_en", "Unknown") if home_ai_selector else "None"
			]
		)
	)
	print("  Home Preset: %s" % [home_preset_selector.get_selected_preset() if home_preset_selector else "None"])
	print(
		(
			"  Away AI: %d (%s)"
			% [
				match_plan.away_ai_difficulty,
				away_ai_selector.get_selected_metadata().get("name_en", "Unknown") if away_ai_selector else "None"
			]
		)
	)
	print("  Away Preset: %s" % [away_preset_selector.get_selected_preset() if away_preset_selector else "None"])

	# Phase 4: Rust Engine 호출
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine:
		push_error("[AIMatchSetup] FootballRustEngine not found!")
		return

	print("[AIMatchSetup] Calling FootballRustEngine.simulate_match_json()...")
	var result = rust_engine.simulate_match_json(match_plan)

	if result.get("error", false):
		push_error("[AIMatchSetup] Match simulation failed: %s" % result.get("message", "Unknown error"))
		return

	print("[AIMatchSetup] Match completed!")
	print("  Score: %d - %d" % [result.get("score_home", 0), result.get("score_away", 0)])
	print("  Events: %d" % result.get("events", []).size())

	# 기존 시그널 유지 (하위 호환성)
	match_ready.emit(match_plan)


## 뒤로 버튼 핸들러
func _on_back_button_pressed():
	# TODO: 메인 메뉴로 돌아가기
	get_tree().change_scene_to_file("res://scenes/ui/main_menu.tscn")


## MatchPlan 생성 (Rust MatchPlan 구조체와 호환)
func create_match_plan() -> Dictionary:
	"""
	Rust MatchPlan 구조체로 전달할 경기 설정 생성

	Phase 3.2: AI 난이도 추가
	Phase 3.3: 전술 프리셋 (home_instructions, away_instructions) 추가

	Returns:
		Dictionary: {
			home_team: {...},
			away_team: {...},
			home_ai_difficulty: int,  # 0=Easy, 1=Medium, 2=Hard, 3=Expert
			away_ai_difficulty: int,
			home_instructions: {...},  # TeamInstructions (Phase 3.3)
			away_instructions: {...},
			seed: int
		}
	"""
	# AI 난이도 (Phase 3.2)
	var home_difficulty = home_ai_selector.get_selected_difficulty() if home_ai_selector else 1
	var away_difficulty = away_ai_selector.get_selected_difficulty() if away_ai_selector else 1

	# 전술 프리셋 (Phase 3.3)
	var home_instructions = home_preset_selector.get_instructions() if home_preset_selector else null
	var away_instructions = away_preset_selector.get_instructions() if away_preset_selector else null

	var match_plan = {
		"home_team": _create_team_data("Home Team", _formation_home),
		"away_team": _create_team_data("Away Team", _formation_away),
		"home_ai_difficulty": home_difficulty,
		"away_ai_difficulty": away_difficulty,
		"home_instructions": home_instructions,  # Phase 3.3: 초기 전술 프리셋
		"away_instructions": away_instructions,  # Phase 3.3: 초기 전술 프리셋
		"seed": randi(),  # 랜덤 시드
		"user_player": null,  # 유저 플레이어 없음 (AI vs AI)
		"home_formation": _formation_home,
		"away_formation": _formation_away,
		"home_player_instructions": null,
		"away_player_instructions": null
	}

	return match_plan


## 팀 데이터 생성 (스텁)
func _create_team_data(team_name: String, formation: String) -> Dictionary:
	"""
	테스트용 팀 데이터 생성

	실제 프로젝트에서는 PlayerLibrary나 TeamData에서 가져와야 함
	"""
	var players = []

	# 11명의 선수 생성 (테스트용)
	for i in range(11):
		players.append(
			{
				"name": "Player %d" % (i + 1),
				"position": "MF",
				"overall": 75,
				"attributes": null,
				"equipped_skills": [],
				"traits": {},
				"personality": "Balanced"
			}
		)

	return {"name": team_name, "formation": formation, "players": players}


## 팀 데이터 설정 (외부에서 호출)
func set_team_data(home_team: Dictionary, away_team: Dictionary):
	"""
	외부에서 팀 데이터를 설정

	Args:
		home_team: 홈팀 데이터
		away_team: 원정팀 데이터
	"""
	_home_team_data = home_team
	_away_team_data = away_team


## 포메이션 설정 (외부에서 호출)
func set_formations(home: String, away: String):
	"""
	외부에서 포메이션 설정

	Args:
		home: 홈팀 포메이션 (예: "4-4-2")
		away: 원정팀 포메이션 (예: "4-3-3")
	"""
	_formation_home = home
	_formation_away = away


## AI 난이도 프로그래밍 방식 설정
func set_ai_difficulties(home: int, away: int):
	"""
	프로그래밍 방식으로 AI 난이도 설정

	Args:
		home: 홈팀 AI 난이도 (0=Easy, 1=Medium, 2=Hard, 3=Expert)
		away: 원정팀 AI 난이도
	"""
	if home_ai_selector:
		home_ai_selector.set_difficulty(home)

	if away_ai_selector:
		away_ai_selector.set_difficulty(away)


## 현재 AI 난이도 반환
func get_ai_difficulties() -> Dictionary:
	"""
	현재 선택된 AI 난이도 반환

	Returns:
		Dictionary: { home: int, away: int }
	"""
	return {
		"home": home_ai_selector.get_selected_difficulty() if home_ai_selector else 1,
		"away": away_ai_selector.get_selected_difficulty() if away_ai_selector else 1
	}


## 전술 프리셋 프로그래밍 방식 설정 (Phase 3.3)
func set_presets(home_preset_id: String, away_preset_id: String):
	"""
	프로그래밍 방식으로 전술 프리셋 설정

	Args:
		home_preset_id: 홈팀 프리셋 ID (예: "tiki_taka")
		away_preset_id: 원정팀 프리셋 ID
	"""
	if home_preset_selector:
		home_preset_selector.set_preset(home_preset_id)

	if away_preset_selector:
		away_preset_selector.set_preset(away_preset_id)


## 현재 전술 프리셋 반환 (Phase 3.3)
func get_presets() -> Dictionary:
	"""
	현재 선택된 전술 프리셋 ID 반환

	Returns:
		Dictionary: { home: String, away: String }
	"""
	return {
		"home": home_preset_selector.get_selected_preset() if home_preset_selector else "tiki_taka",
		"away": away_preset_selector.get_selected_preset() if away_preset_selector else "tiki_taka"
	}


## 현재 전술 지시 반환 (Phase 3.3)
func get_instructions() -> Dictionary:
	"""
	현재 선택된 전술 지시 반환

	Returns:
		Dictionary: {
			home: { tempo, pressing, width, build_up_play, defensive_line },
			away: { tempo, pressing, width, build_up_play, defensive_line }
		}
	"""
	return {
		"home": home_preset_selector.get_instructions() if home_preset_selector else {},
		"away": away_preset_selector.get_instructions() if away_preset_selector else {}
	}
