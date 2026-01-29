extends Control
##
## HorizontalMatchViewer 테스트 컨트롤러
##
## 기능: 선수 랜덤 이동, 카메라 모드 전환, 이펙트 테스트
##

@onready var viewer: HorizontalMatchViewer = $HorizontalMatchViewer
@onready var camera_mode_label: Label = $UI/CameraModeLabel
@onready var team_label: Label = $UI/TeamLabel

var _test_timer: float = 0.0
var _auto_move: bool = false

## 패턴 테스트용 팀 프리셋
const TEAM_PRESETS := [
	["home", "away"],  ## 기본
	["ac_milan", "inter_milan"],  ## Serie A Derby
	["barcelona", "argentina"],  ## 세로줄
	["celtic", "qpr"],  ## 가로줄
	["croatia", "brazil"],  ## 체크
	["korea", "japan"],  ## 아시아
]
var _current_preset_index: int = 0


func _ready() -> void:
	$UI/FollowButton.pressed.connect(_on_follow_pressed)
	$UI/FullPitchButton.pressed.connect(_on_full_pitch_pressed)
	$UI/TacticalButton.pressed.connect(_on_tactical_pressed)
	$UI/ShakeButton.pressed.connect(_on_shake_pressed)
	$UI/GoalEffectButton.pressed.connect(_on_goal_effect_pressed)
	$UI/RandomMoveButton.pressed.connect(_on_random_move_pressed)
	$UI/ChangeTeamButton.pressed.connect(_on_change_team_pressed)

	## 초기 테스트 데이터
	await get_tree().process_frame
	_apply_initial_positions()
	_update_team_label()


func _process(delta: float) -> void:
	if _auto_move:
		_test_timer += delta
		if _test_timer >= 0.1:  ## 100ms마다 업데이트
			_test_timer = 0.0
			_apply_random_snapshot()


func _apply_initial_positions() -> void:
	## 4-4-2 포메이션 초기 배치
	var snapshot := _create_formation_snapshot()
	viewer.apply_position_snapshot(snapshot)


func _create_formation_snapshot() -> Dictionary:
	var players := {}

	## 홈 팀 (왼쪽) 4-4-2
	var home_positions := [
		Vector2(5, 34),  ## GK
		Vector2(20, 10),  ## LB
		Vector2(20, 25),  ## CB
		Vector2(20, 43),  ## CB
		Vector2(20, 58),  ## RB
		Vector2(40, 15),  ## LM
		Vector2(40, 30),  ## CM
		Vector2(40, 38),  ## CM
		Vector2(40, 53),  ## RM
		Vector2(60, 25),  ## ST
		Vector2(60, 43),  ## ST
	]

	## 어웨이 팀 (오른쪽) 4-4-2 (미러)
	var away_positions := [
		Vector2(100, 34),  ## GK
		Vector2(85, 58),  ## LB
		Vector2(85, 43),  ## CB
		Vector2(85, 25),  ## CB
		Vector2(85, 10),  ## RB
		Vector2(65, 53),  ## LM
		Vector2(65, 38),  ## CM
		Vector2(65, 30),  ## CM
		Vector2(65, 15),  ## RM
		Vector2(45, 43),  ## ST
		Vector2(45, 25),  ## ST
	]

	for i in range(11):
		players["home_%d" % i] = {"pos": home_positions[i], "action": "idle"}
		players["away_%d" % i] = {"pos": away_positions[i], "action": "idle"}

	return {"t_ms": 0, "ball": {"pos": Vector2(52.5, 34), "z": 0.0}, "players": players}


func _apply_random_snapshot() -> void:
	var snapshot := _create_formation_snapshot()

	## 약간의 랜덤 오프셋 추가
	for player_id in snapshot["players"]:
		var data: Dictionary = snapshot["players"][player_id]
		data["pos"] += Vector2(randf_range(-2, 2), randf_range(-2, 2))
		data["action"] = ["idle", "running", "dribbling"][randi() % 3]

	## 공도 랜덤 이동
	var ball_pos: Vector2 = snapshot["ball"]["pos"]
	ball_pos += Vector2(randf_range(-5, 5), randf_range(-3, 3))
	ball_pos.x = clamp(ball_pos.x, 5, 100)
	ball_pos.y = clamp(ball_pos.y, 5, 63)
	snapshot["ball"]["pos"] = ball_pos

	viewer.apply_position_snapshot(snapshot)


func _on_follow_pressed() -> void:
	viewer.set_camera_mode_follow()
	camera_mode_label.text = "Camera: Follow"


func _on_full_pitch_pressed() -> void:
	viewer.set_camera_mode_full()
	camera_mode_label.text = "Camera: Full Pitch"


func _on_tactical_pressed() -> void:
	viewer.set_camera_mode_tactical(true)
	camera_mode_label.text = "Camera: Tactical"


func _on_shake_pressed() -> void:
	if viewer.camera:
		viewer.camera.camera_shake(10.0, 0.5)


func _on_goal_effect_pressed() -> void:
	viewer.trigger_goal_effect()


func _on_random_move_pressed() -> void:
	_auto_move = not _auto_move
	$UI/RandomMoveButton.text = "Stop Move" if _auto_move else "Random Move"


func _on_change_team_pressed() -> void:
	_current_preset_index = (_current_preset_index + 1) % TEAM_PRESETS.size()
	var preset: Array = TEAM_PRESETS[_current_preset_index]
	viewer.set_team_colors(preset[0], preset[1])
	_update_team_label()


func _update_team_label() -> void:
	if team_label:
		var preset: Array = TEAM_PRESETS[_current_preset_index]
		team_label.text = "Home: %s / Away: %s" % [preset[0], preset[1]]
