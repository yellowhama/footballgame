## TeamPreviewBackground.gd
## 여러 선수가 돌아다니는 배경 컴포넌트
## 마이팀 설정 화면에서 사용
##
## 사용법:
##   var bg = TeamPreviewBackground.new()
##   bg.set_team_colors(Color.RED, Color.WHITE, 2)  # 세로줄 패턴
##
## 참조: assets/ui/2025-12-08_CHARACTER_SPRITE_USAGE_FOR_UI.md

class_name TeamPreviewBackground
extends Control

## 표시할 선수 수
@export var player_count: int = 8

## 팀 컬러
@export var primary_color: Color = Color.RED:
	set(value):
		primary_color = value
		_update_all_players_color()

@export var secondary_color: Color = Color.WHITE:
	set(value):
		secondary_color = value
		_update_all_players_color()

## 패턴 타입 (0=단색, 1=가로줄, 2=세로줄, 3=체크)
@export_range(0, 4) var pattern_type: int = 0:
	set(value):
		pattern_type = value
		_update_all_players_color()

## 선수 스케일 (배경용이라 작게)
@export var player_scale: float = 2.0

## 내부 데이터
var _players: Array[Node2D] = []
var _ball: Sprite2D = null
var _tweens: Array[Tween] = []

const HAIR_FOLDERS := ["black", "blonde", "redhead", "other"]
const BALL_TEXTURE := "res://assets/socceralia/ball-idle.png"
const CHARACTER_SCENE := "res://scenes/ui/components/CharacterPreviewSprite.tscn"


func _ready() -> void:
	clip_contents = true
	_spawn_players()
	_spawn_ball()


func _exit_tree() -> void:
	# 모든 트윈 정리
	for tween in _tweens:
		if tween and tween.is_valid():
			tween.kill()
	_tweens.clear()


func _spawn_players() -> void:
	for i in range(player_count):
		var player := _create_player(i)
		add_child(player)
		_players.append(player)
		# 약간의 딜레이를 두고 움직임 시작 (자연스러움)
		await get_tree().create_timer(randf_range(0.0, 0.5)).timeout
		_start_random_movement(player)


func _create_player(index: int) -> Node2D:
	var player: Node2D

	# CharacterPreviewSprite 씬이 있으면 사용, 없으면 직접 생성
	if ResourceLoader.exists(CHARACTER_SCENE):
		player = load(CHARACTER_SCENE).instantiate()
	else:
		player = CharacterPreviewSprite.new()

	# 랜덤 헤어 폴더
	player.hair_folder = HAIR_FOLDERS[index % HAIR_FOLDERS.size()]

	# 팀 컬러
	player.primary_color = primary_color
	player.secondary_color = secondary_color
	player.pattern_type = pattern_type

	# 랜덤 시작 위치 (컨테이너 영역 내)
	var margin := 50.0
	player.position = Vector2(randf_range(margin, size.x - margin), randf_range(margin, size.y - margin))

	# 스케일 조정
	player.set_scale_factor(player_scale)

	# 기본 애니메이션
	player.play_animation(CharacterPreviewSprite.AnimState.IDLE)

	return player


func _spawn_ball() -> void:
	if not ResourceLoader.exists(BALL_TEXTURE):
		return

	_ball = Sprite2D.new()
	_ball.name = "Ball"
	_ball.texture = load(BALL_TEXTURE)
	_ball.scale = Vector2(2.0, 2.0)
	_ball.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	_ball.position = size / 2
	add_child(_ball)

	# 공도 천천히 움직임
	_start_ball_movement()


func _start_random_movement(player: Node2D) -> void:
	if not is_inside_tree():
		return
	_move_to_random_target(player)


func _move_to_random_target(player: Node2D) -> void:
	if not is_inside_tree() or not is_instance_valid(player):
		return

	var margin := 30.0
	var target := Vector2(randf_range(margin, size.x - margin), randf_range(margin, size.y - margin))

	var distance := player.position.distance_to(target)
	var speed := randf_range(30.0, 60.0)  # 픽셀/초
	var duration := distance / speed

	# 방향에 따라 flip
	player.flip_h(target.x < player.position.x)

	# 달리기 애니메이션
	player.play_animation(CharacterPreviewSprite.AnimState.RUN)

	var tween := create_tween()
	_tweens.append(tween)

	tween.tween_property(player, "position", target, duration)
	tween.tween_callback(
		func():
			if not is_instance_valid(player):
				return
			# 도착 후 잠시 대기
			player.play_animation(CharacterPreviewSprite.AnimState.IDLE)

			# 가끔 킥 동작
			if randf() < 0.2:
				player.play_animation(CharacterPreviewSprite.AnimState.KICK)
				await get_tree().create_timer(0.5).timeout

			# 대기 후 다시 이동
			await get_tree().create_timer(randf_range(1.0, 3.0)).timeout
			if is_inside_tree() and is_instance_valid(player):
				_move_to_random_target(player)
	)


func _start_ball_movement() -> void:
	if not _ball or not is_inside_tree():
		return
	_move_ball_to_random()


func _move_ball_to_random() -> void:
	if not is_inside_tree() or not is_instance_valid(_ball):
		return

	var margin := 50.0
	var target := Vector2(randf_range(margin, size.x - margin), randf_range(margin, size.y - margin))

	var duration := randf_range(3.0, 6.0)

	var tween := create_tween()
	_tweens.append(tween)

	tween.tween_property(_ball, "position", target, duration).set_ease(Tween.EASE_IN_OUT)
	tween.tween_callback(
		func():
			await get_tree().create_timer(randf_range(1.0, 2.0)).timeout
			if is_inside_tree() and is_instance_valid(_ball):
				_move_ball_to_random()
	)


func _update_all_players_color() -> void:
	for player in _players:
		if is_instance_valid(player) and player.has_method("set_team_colors"):
			player.set_team_colors(primary_color, secondary_color, pattern_type)


## 외부 API
func set_team_colors(primary: Color, secondary: Color, pattern: int = -1) -> void:
	primary_color = primary
	secondary_color = secondary
	if pattern >= 0:
		pattern_type = pattern


func set_pattern_type(pattern: int) -> void:
	pattern_type = pattern


func set_player_count(count: int) -> void:
	## 플레이어 수 변경 (기존 플레이어 제거 후 재생성)
	for player in _players:
		if is_instance_valid(player):
			player.queue_free()
	_players.clear()

	player_count = count
	_spawn_players()
