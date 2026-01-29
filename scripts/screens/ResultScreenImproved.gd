extends Control
# ResultScreenImproved - 개선된 결과 화면

# UI 요소들
@onready var result_title: Label = $Header/VBox/Title
@onready var result_message: Label = $Header/VBox/Message
@onready var continue_button: Button = $BottomBar/ContinueButton

# 결과 데이터
var result_data: Dictionary = {}


func _ready():
	print("[ResultScreenImproved] Initializing...")

	# ColorSystem 적용
	SceneColorUpdater.apply_color_system_to_scene(self)

	# 반응형 레이아웃 수정
	ResponsiveLayoutFixer.fix_scene_layout(self)

	# 터치 피드백 적용
	# TouchFeedback class doesn't exist - TODO: implement if needed

	# 버튼 연결
	_connect_buttons()

	# 결과 데이터 로드
	_load_result_data()


func _connect_buttons():
	if continue_button:
		continue_button.pressed.connect(_on_continue_pressed)
		print("[ResultScreenImproved] Continue button connected")
	else:
		print("[ResultScreenImproved] Continue button not found!")


func _load_result_data():
	# GameManager에서 결과 정보 가져오기
	if GameManager:
		result_data = GameManager.get_last_result()

	# 기본 결과 설정
	if result_data.is_empty():
		result_data = {"type": "training", "title": "훈련 결과", "description": "훈련을 완료했습니다.", "stats_changed": {}}

	# UI 업데이트
	_update_result_display()


func _update_result_display():
	if result_title:
		result_title.text = result_data.get("title", "결과")

	if result_message:
		var description = result_data.get("description", "작업이 완료되었습니다.")
		result_message.text = description

	# TODO: 추후 MainContent의 BeforeAfterPanel을 활용해서 시각적으로 표시


func _format_team_training_result() -> String:
	"""팀훈련 결과를 포맷팅"""
	var text = ""

	# 훈련 타입
	var training_type = result_data.get("training_type", "알 수 없음")
	text += "[center][font_size=24][b]%s 훈련[/b][/font_size][/center]\n\n" % training_type

	# 메시지
	var description = result_data.get("description", "")
	if description != "":
		text += "[center]%s[/center]\n\n" % description

	text += "[color=yellow]━━━━━━━━━━━━━━━━━━━━[/color]\n\n"

	# 능력치 변화
	var stats_changed = result_data.get("stats_changed", {})
	if not stats_changed.is_empty():
		text += "[font_size=20][b]📈 능력치 상승[/b][/font_size]\n"
		for stat_name in stats_changed:
			var delta = stats_changed[stat_name]
			if delta > 0:
				text += "  • %s: [color=green]+%.1f[/color]\n" % [_translate_stat_name(stat_name), delta]
		text += "\n"

	# 체력 소모
	var fatigue_cost = result_data.get("fatigue_cost", 0)
	if fatigue_cost > 0:
		text += "[font_size=20][b]😴 체력 소모[/b][/font_size]\n"
		text += "  • 피로도: [color=orange]+%d[/color]\n\n" % fatigue_cost

	# 관계도 변화
	var coach_rel = result_data.get("coach_relationship", 0)
	var team_chem = result_data.get("team_chemistry", 0)

	if coach_rel != 0 or team_chem != 0:
		text += "[font_size=20][b]🤝 관계도 변화[/b][/font_size]\n"
		if coach_rel > 0:
			text += "  • 감독 관계: [color=green]+%.1f[/color]\n" % coach_rel
		if team_chem > 0:
			text += "  • 팀 케미스트리: [color=green]+%.1f[/color]\n" % team_chem
		text += "\n"

	text += "[color=yellow]━━━━━━━━━━━━━━━━━━━━[/color]\n\n"
	text += "[center][color=gray]계속하려면 버튼을 누르세요[/color][/center]"

	return text


func _translate_stat_name(stat: String) -> String:
	"""능력치 이름 한글 번역"""
	var translations = {
		"finishing": "결정력",
		"long_shots": "중거리슛",
		"passing": "패스",
		"crossing": "크로스",
		"dribbling": "드리블",
		"ball_control": "볼컨트롤",
		"tackling": "태클",
		"marking": "마킹",
		"heading": "헤딩",
		"positioning": "포지셔닝",
		"stamina": "스태미나",
		"strength": "힘",
		"pace": "스피드",
		"acceleration": "가속도",
		"agility": "민첩성",
		"balance": "밸런스",
		"jumping": "점프력",
		"vision": "시야",
		"composure": "침착성",
		"aggression": "적극성",
		"teamwork": "팀워크",
		"work_rate": "활동량",
		"leadership": "리더십",
		"technique": "기술"
	}

	return translations.get(stat, stat)


func _on_continue_pressed():
	print("[ResultScreenImproved] Continue button pressed")

	# 결과 타입에 따라 다른 화면으로 이동
	var result_type = result_data.get("type", "training")

	match result_type:
		"training", "team_training":
			get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")
		"match":
			get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")
		_:
			get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")
