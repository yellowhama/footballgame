extends Control

# UI 요소들
@onready var back_button: Button = $BottomBar/HBox/BackButton
@onready var start_game_button: Button = $BottomBar/HBox/StartGameButton
@onready var strengths_label: Label = $MainContent/SelectionInfo/StrengthsLabel
@onready var weaknesses_label: Label = $MainContent/SelectionInfo/WeaknessesLabel

# 탭 컨테이너와 그리드들
@onready var tab_container: TabContainer = $MainContent/TabContainer
@onready var physical_grid: GridContainer = $MainContent/TabContainer/Physical/PhysicalGrid
@onready var technical_grid: GridContainer = $MainContent/TabContainer/Technical/TechnicalGrid
@onready var mental_grid: GridContainer = $MainContent/TabContainer/Mental/MentalGrid

# 선택된 능력치 표시 영역
@onready
var strengths_list: VBoxContainer = $MainContent/SelectedAbilities/SelectedContainer/StrengthsContainer/StrengthsList
@onready
var weaknesses_list: VBoxContainer = $MainContent/SelectedAbilities/SelectedContainer/WeaknessesContainer/WeaknessesList

# 선택된 능력치
var selected_strengths: Array = []
var selected_weaknesses: Array = []

# 능력치 목록 (32개 필드 플레이어 능력치)
var abilities: Dictionary = {
	"physical":
	[
		{"name": "stamina", "description": "지구력"},
		{"name": "strength", "description": "근력"},
		{"name": "pace", "description": "주력"},
		{"name": "acceleration", "description": "가속력"},
		{"name": "agility", "description": "민첩성"},
		{"name": "balance", "description": "균형감"},
		{"name": "natural_fitness", "description": "천부적 체력"},
		{"name": "jumping", "description": "점프력"}
	],
	"technical":
	[
		{"name": "technique", "description": "기술"},
		{"name": "passing", "description": "패스"},
		{"name": "finishing", "description": "골 결정력"},
		{"name": "dribbling", "description": "드리블"},
		{"name": "first_touch", "description": "첫 터치"},
		{"name": "crossing", "description": "크로스"},
		{"name": "free_kicks", "description": "프리킥"},
		{"name": "penalty_taking", "description": "페널티킥"},
		{"name": "corners", "description": "코너킥"},
		{"name": "long_shots", "description": "중거리 슈팅"},
		{"name": "tackling", "description": "태클"},
		{"name": "marking", "description": "마킹"},
		{"name": "heading", "description": "헤딩"}
	],
	"mental":
	[
		{"name": "concentration", "description": "집중력"},
		{"name": "decisions", "description": "판단력"},
		{"name": "vision", "description": "시야"},
		{"name": "anticipation", "description": "예측력"},
		{"name": "positioning", "description": "포지셔닝"},
		{"name": "off_the_ball", "description": "무볼 움직임"},
		{"name": "teamwork", "description": "팀워크"},
		{"name": "work_rate", "description": "활동량"},
		{"name": "determination", "description": "의지력"},
		{"name": "aggression", "description": "공격성"},
		{"name": "leadership", "description": "리더십"},
		{"name": "composure", "description": "침착함"}
	]
}

# 현재 선택 모드 (strength/weakness)
var current_selection_mode: String = "strength"


func _ready():
	print("[CharacterCreateAbilities] Initializing abilities selection...")

	# 버튼 연결
	_connect_buttons()

	# 능력치 버튼 생성
	_create_ability_buttons()

	# 초기 상태 설정
	_update_selection_info()

	print("[CharacterCreateAbilities] Ready complete")


func _connect_buttons():
	"""버튼들 연결"""
	print("[CharacterCreateAbilities] Connecting buttons...")

	if back_button:
		back_button.pressed.connect(_on_back_pressed)
		print("[CharacterCreateAbilities] Back button connected")
	else:
		print("[CharacterCreateAbilities] ERROR: Back button not found!")

	if start_game_button:
		start_game_button.pressed.connect(_on_start_game_pressed)
		print("[CharacterCreateAbilities] Start game button connected")
	else:
		print("[CharacterCreateAbilities] ERROR: Start game button not found!")


func _create_ability_buttons():
	"""능력치 버튼들 생성"""
	# Physical 탭
	for ability in abilities["physical"]:
		var button = _create_ability_button(ability)
		physical_grid.add_child(button)

	# Technical 탭
	for ability in abilities["technical"]:
		var button = _create_ability_button(ability)
		technical_grid.add_child(button)

	# Mental 탭
	for ability in abilities["mental"]:
		var button = _create_ability_button(ability)
		mental_grid.add_child(button)


func _create_ability_button(ability: Dictionary) -> Button:
	"""능력치 버튼 생성"""
	var button = Button.new()
	button.text = ability["name"] + "\n" + ability["description"]
	button.custom_minimum_size = Vector2(200, 80)
	button.add_theme_font_size_override("font_size", 16)
	button.pressed.connect(_on_ability_selected.bind(ability))
	return button


func _on_back_pressed():
	print("[CharacterCreateAbilities] Back button pressed")
	get_tree().change_scene_to_file("res://scenes/CharacterCreatePosition.tscn")


func _on_start_game_pressed():
	print("[CharacterCreateAbilities] Start game button pressed")
	if selected_strengths.size() == 5 and selected_weaknesses.size() == 5:
		# 능력치 데이터를 전역으로 저장
		GlobalCharacterData.set_abilities(selected_strengths, selected_weaknesses)

		# 최종 캐릭터 데이터 생성
		_create_final_character()

		# 게임 시작 - 홈 화면으로 이동
		get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")
	else:
		print("[CharacterCreateAbilities] ERROR: Need 5 strengths and 5 weaknesses!")


func _on_ability_selected(ability: Dictionary):
	"""능력치 선택"""
	var ability_name = ability["name"]
	print("[CharacterCreateAbilities] Ability selected: %s" % ability_name)

	# 이미 선택된 능력치인지 확인
	if ability_name in selected_strengths:
		# 장점에서 제거
		selected_strengths.erase(ability_name)
		_remove_from_selected_list(ability_name, "strength")
	elif ability_name in selected_weaknesses:
		# 단점에서 제거
		selected_weaknesses.erase(ability_name)
		_remove_from_selected_list(ability_name, "weakness")
	else:
		# 새로운 선택
		if current_selection_mode == "strength" and selected_strengths.size() < 5:
			selected_strengths.append(ability_name)
			_add_to_selected_list(ability_name, "strength")
		elif current_selection_mode == "weakness" and selected_weaknesses.size() < 5:
			selected_weaknesses.append(ability_name)
			_add_to_selected_list(ability_name, "weakness")
		else:
			print("[CharacterCreateAbilities] Maximum abilities selected for current mode!")
			return

	# UI 업데이트
	_update_selection_info()
	_update_ability_buttons()
	_update_selection_mode()


func _add_to_selected_list(ability_name: String, type: String):
	"""선택된 능력치 목록에 추가"""
	var label = Label.new()
	label.text = ability_name
	label.add_theme_font_size_override("font_size", 12)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.custom_minimum_size = Vector2(0, 20)

	if type == "strength":
		label.add_theme_color_override("font_color", Color(0.3, 1.0, 0.3, 1.0))
		strengths_list.add_child(label)
	else:
		label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3, 1.0))
		weaknesses_list.add_child(label)


func _remove_from_selected_list(ability_name: String, type: String):
	"""선택된 능력치 목록에서 제거"""
	var target_list = strengths_list if type == "strength" else weaknesses_list

	for child in target_list.get_children():
		if child.text == ability_name:
			child.queue_free()
			break


func _update_selection_info():
	"""선택 정보 업데이트"""
	strengths_label.text = "장점: %d/5" % selected_strengths.size()
	weaknesses_label.text = "단점: %d/5" % selected_weaknesses.size()

	# 게임 시작 버튼 활성화/비활성화
	start_game_button.disabled = !(selected_strengths.size() == 5 and selected_weaknesses.size() == 5)


func _update_ability_buttons():
	"""능력치 버튼 상태 업데이트"""
	# 모든 그리드의 버튼들 업데이트
	_update_grid_buttons(physical_grid)
	_update_grid_buttons(technical_grid)
	_update_grid_buttons(mental_grid)


func _update_grid_buttons(grid: GridContainer):
	"""그리드의 버튼들 상태 업데이트"""
	for i in range(grid.get_child_count()):
		var button = grid.get_child(i)
		var ability_name = button.text.split("\n")[0]  # 첫 번째 줄이 능력치 이름

		if ability_name in selected_strengths:
			button.modulate = Color(0.7, 1.0, 0.7, 1.0)  # 녹색
		elif ability_name in selected_weaknesses:
			button.modulate = Color(1.0, 0.7, 0.7, 1.0)  # 빨간색
		else:
			button.modulate = Color.WHITE


func _update_selection_mode():
	"""선택 모드 업데이트"""
	# 장점 5개를 모두 선택했으면 단점 선택 모드로 전환
	if selected_strengths.size() == 5 and current_selection_mode == "strength":
		current_selection_mode = "weakness"
		print("[CharacterCreateAbilities] Switching to weakness selection mode")

		# 모든 버튼을 빨간색으로 변경하여 단점 선택 모드임을 표시
		_update_all_buttons_color(Color(1.0, 0.8, 0.8, 1.0))
	elif selected_weaknesses.size() == 5 and current_selection_mode == "weakness":
		print("[CharacterCreateAbilities] All abilities selected!")


func _update_all_buttons_color(color: Color):
	"""모든 버튼의 색상 업데이트 (선택되지 않은 버튼만)"""
	_update_grid_color(physical_grid, color)
	_update_grid_color(technical_grid, color)
	_update_grid_color(mental_grid, color)


func _update_grid_color(grid: GridContainer, color: Color):
	"""그리드의 선택되지 않은 버튼들 색상 업데이트"""
	for i in range(grid.get_child_count()):
		var button = grid.get_child(i)
		var ability_name = button.text.split("\n")[0]

		# 이미 선택된 버튼은 색상 변경하지 않음
		if ability_name not in selected_strengths and ability_name not in selected_weaknesses:
			button.modulate = color


func _create_final_character():
	"""최종 캐릭터 데이터 생성"""
	var character_data = GlobalCharacterData.character_data.duplicate()

	# 기본 능력치 설정 (포지션별 차별화)
	var base_stats = _get_base_stats_for_position(character_data.get("position", "attacker"))

	# 장점 능력치 +10
	for strength in selected_strengths:
		if strength in base_stats:
			base_stats[strength] += 10

	# 단점 능력치 -10
	for weakness in selected_weaknesses:
		if weakness in base_stats:
			base_stats[weakness] -= 10

	# 최종 캐릭터 데이터 저장
	character_data["final_character"] = {
		"appearance": character_data.get("appearance", {}),
		"position": character_data.get("position", "attacker"),
		"strengths": selected_strengths,
		"weaknesses": selected_weaknesses,
		"stats": base_stats
	}

	GlobalCharacterData.set_character_data(character_data)
	print("[CharacterCreateAbilities] Final character created: %s" % character_data["final_character"])


func _get_base_stats_for_position(position: String) -> Dictionary:
	"""포지션별 기본 능력치 반환 (32개 능력치)"""
	var base_stats = {
		# Physical (8개)
		"stamina": 20,
		"strength": 20,
		"pace": 20,
		"acceleration": 20,
		"agility": 20,
		"balance": 20,
		"natural_fitness": 20,
		"jumping": 20,
		# Technical (13개)
		"technique": 20,
		"passing": 20,
		"finishing": 20,
		"dribbling": 20,
		"first_touch": 20,
		"crossing": 20,
		"free_kicks": 20,
		"penalty_taking": 20,
		"corners": 20,
		"long_shots": 20,
		"tackling": 20,
		"marking": 20,
		"heading": 20,
		# Mental (11개)
		"concentration": 20,
		"decisions": 20,
		"vision": 20,
		"anticipation": 20,
		"positioning": 20,
		"off_the_ball": 20,
		"teamwork": 20,
		"work_rate": 20,
		"determination": 20,
		"aggression": 20,
		"leadership": 20,
		"composure": 20
	}

	match position:
		"attacker":
			# 공격수 특화 능력치
			base_stats["finishing"] = 35  # 골 결정력
			base_stats["dribbling"] = 30  # 드리블
			base_stats["pace"] = 32  # 주력
			base_stats["acceleration"] = 30  # 가속력
			base_stats["off_the_ball"] = 28  # 무볼 움직임
			base_stats["long_shots"] = 25  # 중거리 슈팅
		"midfielder":
			# 미드필더 특화 능력치
			base_stats["passing"] = 32  # 패스
			base_stats["technique"] = 30  # 기술
			base_stats["vision"] = 28  # 시야
			base_stats["teamwork"] = 30  # 팀워크
			base_stats["work_rate"] = 28  # 활동량
			base_stats["tackling"] = 25  # 태클
		"defender":
			# 수비수 특화 능력치
			base_stats["tackling"] = 35  # 태클
			base_stats["marking"] = 32  # 마킹
			base_stats["positioning"] = 30  # 포지셔닝
			base_stats["stamina"] = 32  # 지구력
			base_stats["strength"] = 30  # 근력
			base_stats["heading"] = 28  # 헤딩

	return base_stats
