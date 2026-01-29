extends Control

# UI 요소들
@onready var back_button: Button = $BottomBar/HBox/BackButton
@onready var next_button: Button = $BottomBar/HBox/NextButton
@onready var info_text: RichTextLabel = $MainContent/PositionInfo/VBox/InfoText

# 포지션 버튼들
@onready var attacker_button: Button = $MainContent/PositionContainer/AttackerButton
@onready var midfielder_button: Button = $MainContent/PositionContainer/MidfielderButton
@onready var defender_button: Button = $MainContent/PositionContainer/DefenderButton

# 선택된 포지션
var selected_position: String = ""

# 포지션 정보
var position_info: Dictionary = {
	"attacker":
	{
		"name": "공격수 (Forward)",
		"description": "팀의 득점을 담당하는 포지션입니다.\n\n특징:\n• 높은 공격력과 스피드\n• 골 결정력이 중요\n• 상대 수비를 압박\n\n주요 능력치: 공격력, 스피드, 골 결정력",
		"color": Color(1.0, 0.3, 0.3, 1.0)
	},
	"midfielder":
	{
		"name": "미드필더 (Midfielder)",
		"description":
		"공격과 수비를 연결하는 중앙 포지션입니다.\n\n특징:\n• 균형잡힌 공격/수비 능력\n• 패스와 볼 컨트롤이 중요\n• 경기 흐름을 조절\n\n주요 능력치: 패스, 볼 컨트롤, 지구력",
		"color": Color(0.3, 1.0, 0.3, 1.0)
	},
	"defender":
	{
		"name": "수비수 (Defender)",
		"description": "팀의 수비를 담당하는 포지션입니다.\n\n특징:\n• 강한 수비력과 체력\n• 태클과 헤딩이 중요\n• 상대 공격을 차단\n\n주요 능력치: 수비력, 체력, 태클",
		"color": Color(0.3, 0.3, 1.0, 1.0)
	}
}


func _ready():
	print("[CharacterCreatePosition] Initializing position selection...")

	# 버튼 연결
	_connect_buttons()

	# 초기 상태 설정
	_update_button_states()

	print("[CharacterCreatePosition] Ready complete")


func _connect_buttons():
	"""버튼들 연결"""
	print("[CharacterCreatePosition] Connecting buttons...")

	if back_button:
		back_button.pressed.connect(_on_back_pressed)
		print("[CharacterCreatePosition] Back button connected")
	else:
		print("[CharacterCreatePosition] ERROR: Back button not found!")

	if next_button:
		next_button.pressed.connect(_on_next_pressed)
		print("[CharacterCreatePosition] Next button connected")
	else:
		print("[CharacterCreatePosition] ERROR: Next button not found!")

	# 포지션 버튼들
	if attacker_button:
		attacker_button.pressed.connect(_on_position_selected.bind("attacker"))
		print("[CharacterCreatePosition] Attacker button connected")
	else:
		print("[CharacterCreatePosition] ERROR: Attacker button not found!")

	if midfielder_button:
		midfielder_button.pressed.connect(_on_position_selected.bind("midfielder"))
		print("[CharacterCreatePosition] Midfielder button connected")
	else:
		print("[CharacterCreatePosition] ERROR: Midfielder button not found!")

	if defender_button:
		defender_button.pressed.connect(_on_position_selected.bind("defender"))
		print("[CharacterCreatePosition] Defender button connected")
	else:
		print("[CharacterCreatePosition] ERROR: Defender button not found!")


func _on_back_pressed():
	print("[CharacterCreatePosition] Back button pressed")
	get_tree().change_scene_to_file("res://scenes/CharacterCreateAppearance.tscn")


func _on_next_pressed():
	print("[CharacterCreatePosition] Next button pressed - Moving to abilities selection")
	if selected_position != "":
		# 포지션 데이터를 전역으로 저장
		GlobalCharacterData.set_position(selected_position)

		# 능력치 선택 씬으로 이동
		get_tree().change_scene_to_file("res://scenes/CharacterCreateAbilities.tscn")
	else:
		print("[CharacterCreatePosition] ERROR: No position selected!")


func _on_position_selected(position: String):
	"""포지션 선택"""
	print("[CharacterCreatePosition] Position selected: %s" % position)
	selected_position = position

	# 버튼 상태 업데이트
	_update_button_states()

	# 포지션 정보 표시
	_show_position_info(position)


func _update_button_states():
	"""버튼 상태 업데이트"""
	# 모든 포지션 버튼 초기화
	_reset_position_buttons()

	# 선택된 포지션 버튼 하이라이트
	if selected_position != "":
		_highlight_selected_position()
		next_button.disabled = false
	else:
		next_button.disabled = true


func _reset_position_buttons():
	"""모든 포지션 버튼 초기화"""
	attacker_button.modulate = Color.WHITE
	midfielder_button.modulate = Color.WHITE
	defender_button.modulate = Color.WHITE


func _highlight_selected_position():
	"""선택된 포지션 버튼 하이라이트"""
	match selected_position:
		"attacker":
			attacker_button.modulate = Color(0.7, 1.0, 0.7, 1.0)
		"midfielder":
			midfielder_button.modulate = Color(0.7, 1.0, 0.7, 1.0)
		"defender":
			defender_button.modulate = Color(0.7, 0.7, 1.0, 1.0)


func _show_position_info(position: String):
	"""포지션 정보 표시"""
	if position in position_info:
		var info = position_info[position]
		info_text.text = "[color=%s]%s[/color]\n\n%s" % [info["color"].to_html(), info["name"], info["description"]]
		print("[CharacterCreatePosition] Showing info for: %s" % info["name"])
