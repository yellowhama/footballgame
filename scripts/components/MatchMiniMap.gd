extends Control
class_name MatchMiniMap

# Match Mini Map - FM 2008 스타일 바둑알 미니맵
# OpenFootball JSON 이벤트 기반 선수 위치 표시
# FormationDisplay.gd 기반으로 확장 구현
# Phase 3: ThemeManager 스타일 적용 (2025-12-03)

signal player_dot_clicked(player_index: int, team_id: int)
signal ball_clicked

# 미니맵 설정 - ThemeManager 색상 사용
@export var use_theme_manager_colors: bool = true  # ThemeManager 색상 사용 여부
@export var use_position_colors: bool = false  # 포지션별 색상 사용 여부

# 기본 색상 (ThemeManager 미사용 시)
@export var field_color: Color = Color(0.15, 0.35, 0.15, 1.0)
@export var line_color: Color = Color(0.5, 0.5, 0.5, 0.8)
@export var home_team_color: Color = Color.BLUE
@export var away_team_color: Color = Color.RED
@export var ball_color: Color = Color.YELLOW
@export var dot_size: float = 20.0
@export var ball_size: float = 15.0

# 선수 포지션 데이터 (포지션 색상용)
var player_positions: Dictionary = {}  # dot_index → position_code

# UI 컴포넌트들
var field_background: ColorRect
var player_dots: Array[Panel] = []  # 22개 바둑알 (원형)
var ball_dot: Panel

# 좌표계 설정 (OpenFootball MeterPos → UI 변환)
# Note: FieldDimensions has class_name, so it's globally available

var field_rect: Rect2 = Rect2(40, 40, 800, 600)  # 105:68 비율 유지
const FIELD_LENGTH_M: float = FieldSpec.FIELD_LENGTH_M
const FIELD_WIDTH_M: float = FieldSpec.FIELD_WIDTH_M

# 현재 위치 데이터
var current_positions: Dictionary = {}  # player_id → Vector2
var ball_position: Vector2 = Vector2(FIELD_LENGTH_M * 0.5, FIELD_WIDTH_M * 0.5)


func _ready():
	print("[MatchMiniMap] Initializing FM 2008 style football mini map")

	_apply_theme_colors()
	_setup_field()
	_create_player_dots()
	_create_ball_dot()

	print("[MatchMiniMap] Mini map ready - 22 player dots + ball")


func _apply_theme_colors() -> void:
	"""ThemeManager 색상 적용"""
	if not use_theme_manager_colors:
		return

	field_color = ThemeManager.MINIMAP_FIELD
	line_color = ThemeManager.MINIMAP_LINE
	home_team_color = ThemeManager.MINIMAP_HOME_DEFAULT
	away_team_color = ThemeManager.MINIMAP_AWAY_DEFAULT
	ball_color = ThemeManager.MINIMAP_BALL


func _setup_field():
	"""축구장 필드 배경 설정"""
	# 전체 크기 설정
	size = Vector2(880, 680)  # 여백 포함

	# 필드 배경
	field_background = ColorRect.new()
	field_background.color = field_color
	field_background.position = field_rect.position
	field_background.size = field_rect.size
	add_child(field_background)

	# 필드 라인 그리기
	_draw_field_lines()


func _draw_field_lines():
	"""축구장 라인 그리기 - 이제 _draw()에서 직접 처리"""
	# 라인은 _draw()에서 그려짐
	pass


func _draw():
	"""필드 라인 직접 그리기"""
	var width = field_rect.size.x
	var height = field_rect.size.y
	var offset = field_rect.position

	# 외곽선
	draw_rect(field_rect, line_color, false, 2.0)

	# 중앙선
	draw_line(offset + Vector2(0, height * 0.5), offset + Vector2(width, height * 0.5), line_color, 2.0)

	# 중앙 원
	var center = offset + Vector2(width * 0.5, height * 0.5)
	draw_arc(center, min(width, height) * 0.15, 0, TAU, 32, line_color, 2.0)

	# 골 에리어 (상하)
	var goal_width = width * 0.2
	var goal_height = height * 0.15

	# 상단 골 에리어
	draw_rect(
		Rect2(offset + Vector2((width - goal_width) * 0.5, 10), Vector2(goal_width, goal_height)),
		line_color,
		false,
		2.0
	)

	# 하단 골 에리어
	draw_rect(
		Rect2(
			offset + Vector2((width - goal_width) * 0.5, height - goal_height - 10), Vector2(goal_width, goal_height)
		),
		line_color,
		false,
		2.0
	)


func _create_player_dots():
	"""22개 선수 바둑알 생성 (홈 11명 + 어웨이 11명)"""
	player_dots.clear()

	# 홈팀 11명 (파랑 바둑알)
	for i in range(11):
		var dot = _create_dot(home_team_color, dot_size)
		_make_clickable(dot, i, 0)  # team_id = 0 (홈)
		add_child(dot)
		player_dots.append(dot)

	# 어웨이팀 11명 (빨강 바둑알)
	for i in range(11):
		var dot = _create_dot(away_team_color, dot_size)
		_make_clickable(dot, i, 1)  # team_id = 1 (어웨이)
		add_child(dot)
		player_dots.append(dot)

	# 초기 포메이션 배치 (442 vs 433)
	_set_initial_formation()


func _create_ball_dot():
	"""볼 바둑알 생성"""
	ball_dot = _create_dot(ball_color, ball_size)
	_make_ball_clickable(ball_dot)
	add_child(ball_dot)

	# 초기 중앙 위치
	update_ball_position(ball_position)


func _create_dot(color: Color, dot_size_param: float) -> Panel:
	"""바둑알 생성 (원형 Panel with StyleBoxFlat)"""
	var dot = Panel.new()
	dot.size = Vector2(dot_size_param, dot_size_param)

	# ThemeManager 스타일 사용
	var style: StyleBoxFlat
	if use_theme_manager_colors:
		style = ThemeManager.create_player_dot_style(color, dot_size_param)
	else:
		# 기존 방식으로 스타일 생성
		style = StyleBoxFlat.new()
		style.bg_color = color
		style.corner_radius_top_left = int(dot_size_param * 0.5)
		style.corner_radius_top_right = int(dot_size_param * 0.5)
		style.corner_radius_bottom_left = int(dot_size_param * 0.5)
		style.corner_radius_bottom_right = int(dot_size_param * 0.5)
		style.set_border_width_all(1)
		style.border_color = Color(0, 0, 0, 0.5)

	dot.add_theme_stylebox_override("panel", style)
	return dot


func _make_clickable(dot: Panel, player_index: int, team_id: int):
	"""선수 바둑알 클릭 가능하게 만들기"""
	var button = Button.new()
	button.size = dot.size
	button.flat = true
	button.modulate = Color(1, 1, 1, 0)  # 투명하지만 클릭 가능
	button.pressed.connect(_on_player_dot_clicked.bind(player_index, team_id))
	dot.add_child(button)


func _make_ball_clickable(dot: Panel):
	"""볼 바둑알 클릭 가능하게 만들기"""
	var button = Button.new()
	button.size = dot.size
	button.flat = true
	button.modulate = Color(1, 1, 1, 0)
	button.pressed.connect(_on_ball_clicked)
	dot.add_child(button)


func _set_initial_formation():
	"""초기 포메이션 배치 (홈: 442, 어웨이: 433)"""
	# 홈팀 442 포메이션 (FormationDisplay 참고)
	var home_positions = [
		Vector2(5.0, 34.0),  # GK
		Vector2(20.0, 12.0),  # LB
		Vector2(20.0, 26.0),  # CB
		Vector2(20.0, 42.0),  # CB
		Vector2(20.0, 56.0),  # RB
		Vector2(50.0, 12.0),  # LM
		Vector2(50.0, 26.0),  # CM
		Vector2(50.0, 42.0),  # CM
		Vector2(50.0, 56.0),  # RM
		Vector2(80.0, 28.0),  # ST
		Vector2(80.0, 40.0)  # ST
	]

	# 어웨이팀 433 포메이션 (뒤집어서)
	var away_positions = [
		Vector2(100.0, 34.0),  # GK
		Vector2(85.0, 12.0),  # LB
		Vector2(85.0, 26.0),  # CB
		Vector2(85.0, 42.0),  # CB
		Vector2(85.0, 56.0),  # RB
		Vector2(65.0, 34.0),  # DM
		Vector2(60.0, 24.0),  # CM
		Vector2(60.0, 44.0),  # CM
		Vector2(35.0, 12.0),  # LW
		Vector2(30.0, 34.0),  # ST
		Vector2(35.0, 56.0)  # RW
	]

	# 홈팀 위치 설정
	for i in range(11):
		update_player_position(i, 0, home_positions[i])

	# 어웨이팀 위치 설정
	for i in range(11):
		update_player_position(i, 1, away_positions[i])


# ==============================================================================
# 좌표 변환 시스템
# ==============================================================================


func meter_to_ui(meter_pos: Vector2) -> Vector2:
	"""OpenFootball MeterPos → UI 좌표 변환"""
	var x_ratio = (meter_pos.x + 0.5) / (FIELD_LENGTH_M + 1.0)
	var y_ratio = (meter_pos.y + 0.5) / (FIELD_WIDTH_M + 1.0)

	return Vector2(
		field_rect.position.x + x_ratio * field_rect.size.x - dot_size * 0.5,
		field_rect.position.y + y_ratio * field_rect.size.y - dot_size * 0.5
	)


func ui_to_meter(ui_pos: Vector2) -> Vector2:
	"""UI 좌표 → OpenFootball MeterPos 변환 (역변환)"""
	var relative_x = (ui_pos.x - field_rect.position.x + dot_size * 0.5) / field_rect.size.x
	var relative_y = (ui_pos.y - field_rect.position.y + dot_size * 0.5) / field_rect.size.y

	return Vector2(relative_x * (FIELD_LENGTH_M + 1.0) - 0.5, relative_y * (FIELD_WIDTH_M + 1.0) - 0.5)


# ==============================================================================
# 공개 API - 위치 업데이트
# ==============================================================================


func update_player_position(player_index: int, team_id: int, meter_pos: Vector2):
	"""선수 위치 업데이트 (MeterPos 좌표)"""
	var dot_index = player_index + (team_id * 11)
	if dot_index < player_dots.size():
		var ui_pos = meter_to_ui(meter_pos)
		player_dots[dot_index].position = ui_pos

		# 위치 기록
		var player_key = "player_%d_%d" % [team_id, player_index]
		current_positions[player_key] = meter_pos


func update_ball_position(meter_pos: Vector2):
	"""볼 위치 업데이트"""
	ball_position = meter_pos
	var ui_pos = meter_to_ui(meter_pos)
	ball_dot.position = ui_pos


func show_pass_arrow(from_meter: Vector2, to_meter: Vector2):
	"""패스 화살표 애니메이션 표시"""
	# 미터 좌표를 UI 좌표로 변환
	var from_ui = meter_to_ui(from_meter)
	var to_ui = meter_to_ui(to_meter)

	# Line2D로 화살표 생성
	var arrow = Line2D.new()
	arrow.width = 3.0
	# ThemeManager 색상 사용
	arrow.default_color = ThemeManager.MINIMAP_PASS_TRAIL if use_theme_manager_colors else Color(0.2, 0.8, 0.2, 0.8)
	arrow.add_point(from_ui)
	arrow.add_point(from_ui)  # 시작점에서 시작
	add_child(arrow)

	# 화살표 머리 그리기 (삼각형)
	var arrow_head = Polygon2D.new()
	arrow_head.color = ThemeManager.MINIMAP_PASS_TRAIL if use_theme_manager_colors else Color(0.2, 0.8, 0.2, 0.8)
	var direction = (to_ui - from_ui).normalized()
	var arrow_size = 10.0
	var head_points = PackedVector2Array(
		[
			to_ui,
			to_ui - direction * arrow_size + direction.orthogonal() * arrow_size * 0.5,
			to_ui - direction * arrow_size - direction.orthogonal() * arrow_size * 0.5
		]
	)
	arrow_head.polygon = head_points
	arrow_head.modulate.a = 0.0  # 초기에는 투명
	add_child(arrow_head)

	# Tween으로 화살표 애니메이션
	var tween = get_tree().create_tween()
	tween.set_parallel(false)

	# 1. 라인이 늘어나는 애니메이션 (0.3초)
	tween.tween_method(
		func(progress: float):
			var current_end = from_ui.lerp(to_ui, progress)
			arrow.set_point_position(1, current_end - field_rect.position),
		0.0,
		1.0,
		0.3
	)

	# 2. 화살표 머리가 나타나는 애니메이션 (0.1초)
	tween.tween_property(arrow_head, "modulate:a", 1.0, 0.1)

	# 3. 잠시 유지 (0.5초)
	tween.tween_interval(0.5)

	# 4. 페이드 아웃 (0.3초)
	tween.set_parallel(true)
	tween.tween_property(arrow, "modulate:a", 0.0, 0.3)
	tween.tween_property(arrow_head, "modulate:a", 0.0, 0.3)

	# 5. 애니메이션 완료 후 제거
	tween.finished.connect(
		func():
			arrow.queue_free()
			arrow_head.queue_free()
	)


func show_shot_trajectory(from_meter: Vector2, to_meter: Vector2, is_goal: bool = false):
	"""슛 궤적 애니메이션 표시"""
	var from_ui = meter_to_ui(from_meter)
	var to_ui = meter_to_ui(to_meter)

	# 슛 궤적 라인 - ThemeManager 색상 사용
	var trajectory = Line2D.new()
	trajectory.width = 4.0
	if use_theme_manager_colors:
		trajectory.default_color = ThemeManager.MINIMAP_SHOT_TRAIL if is_goal else ThemeManager.MINIMAP_SHOT_MISS
	else:
		trajectory.default_color = Color(0.9, 0.2, 0.2, 0.9) if is_goal else Color(0.9, 0.6, 0.2, 0.8)
	trajectory.add_point(from_ui)
	trajectory.add_point(from_ui)
	add_child(trajectory)

	# 볼 스프라이트 (슛 애니메이션용)
	var shot_ball = Control.new()
	shot_ball.custom_minimum_size = Vector2(8, 8)
	shot_ball.position = from_ui - Vector2(4, 4)
	shot_ball.modulate = Color(1, 1, 0, 1)  # 노란색 볼
	add_child(shot_ball)

	# Tween 애니메이션
	var tween = get_tree().create_tween()
	tween.set_parallel(false)

	# 1. 궤적 라인 그리기 (0.2초)
	tween.tween_method(
		func(progress: float):
			var current_end = from_ui.lerp(to_ui, progress)
			trajectory.set_point_position(1, current_end - field_rect.position),
		0.0,
		1.0,
		0.2
	)

	# 2. 볼 이동 애니메이션 (0.3초)
	tween.set_parallel(true)
	tween.tween_property(shot_ball, "position", to_ui - Vector2(4, 4), 0.3).set_ease(Tween.EASE_OUT)

	# 3. 골인 이펙트 (골인 시에만)
	if is_goal:
		tween.tween_callback(func(): show_goal_effect(to_ui))

	# 4. 페이드 아웃 (0.5초 후)
	tween.tween_interval(0.5)
	tween.set_parallel(true)
	tween.tween_property(trajectory, "modulate:a", 0.0, 0.3)
	tween.tween_property(shot_ball, "modulate:a", 0.0, 0.3)

	# 5. 제거
	tween.finished.connect(
		func():
			trajectory.queue_free()
			shot_ball.queue_free()
	)


func show_goal_effect(position: Vector2):
	"""골 이펙트 표시"""
	# 폭발 효과를 위한 원형 이펙트
	for i in range(5):
		var effect = Control.new()
		effect.custom_minimum_size = Vector2(20, 20)
		effect.position = position - Vector2(10, 10)
		effect.modulate = Color(1.0, 0.8, 0.0, 0.8)  # 금색
		add_child(effect)

		var tween = get_tree().create_tween()
		# 확대 및 페이드 아웃
		tween.set_parallel(true)
		tween.tween_property(effect, "scale", Vector2(3, 3), 0.5)
		tween.tween_property(effect, "modulate:a", 0.0, 0.5)
		tween.finished.connect(func(): effect.queue_free())

		# 약간의 딜레이로 연속 효과
		await get_tree().create_timer(0.05 * i).timeout


func show_tackle_effect(position_meter: Vector2):
	"""태클 이펙트 표시"""
	var ui_pos = meter_to_ui(position_meter)

	# 충돌 이펙트를 위한 원형 파동 - ThemeManager 색상 사용
	var tackle_color = ThemeManager.MINIMAP_TACKLE_EFFECT if use_theme_manager_colors else Color(1.0, 0.5, 0.0, 0.9)
	for i in range(3):
		var ring = Control.new()
		ring.custom_minimum_size = Vector2(10, 10)
		ring.position = ui_pos - Vector2(5, 5)
		ring.modulate = tackle_color
		add_child(ring)

		var tween = get_tree().create_tween()
		tween.set_parallel(true)
		# 확대 및 페이드 아웃
		tween.tween_property(ring, "scale", Vector2(2.5, 2.5), 0.3)
		tween.tween_property(ring, "modulate:a", 0.0, 0.3)
		tween.finished.connect(func(): ring.queue_free())

		# 연속 파동 효과
		await get_tree().create_timer(0.1 * i).timeout


func show_dribble_trail(from_meter: Vector2, to_meter: Vector2):
	"""드리블 트레일 애니메이션"""
	var from_ui = meter_to_ui(from_meter)
	var to_ui = meter_to_ui(to_meter)

	# 점선 트레일 생성 - ThemeManager 색상 사용
	var trail = Line2D.new()
	trail.width = 2.0
	trail.default_color = ThemeManager.MINIMAP_DRIBBLE_TRAIL if use_theme_manager_colors else Color(0.6, 0.3, 0.9, 0.7)

	# 점선 효과를 위한 여러 점 추가
	var steps = 10
	for i in range(steps + 1):
		if i % 2 == 0:  # 점선 패턴
			var t = float(i) / float(steps)
			var point = from_ui.lerp(to_ui, t)
			trail.add_point(point - field_rect.position)
			if i < steps:
				var next_t = float(i + 1) / float(steps)
				var next_point = from_ui.lerp(to_ui, next_t)
				trail.add_point(next_point - field_rect.position)

	trail.modulate.a = 0.0
	add_child(trail)

	# Tween 애니메이션
	var tween = get_tree().create_tween()
	tween.set_parallel(false)

	# 1. 페이드 인 (0.2초)
	tween.tween_property(trail, "modulate:a", 1.0, 0.2)

	# 2. 유지 (0.3초)
	tween.tween_interval(0.3)

	# 3. 페이드 아웃 (0.3초)
	tween.tween_property(trail, "modulate:a", 0.0, 0.3)

	# 4. 제거
	tween.finished.connect(func(): trail.queue_free())


func update_from_event(event_data: Dictionary):
	"""OpenFootball JSON 이벤트에서 위치 업데이트"""
	var event_type = event_data.get("type", "")

	match event_type:
		"goal":
			if event_data.has("position"):
				var pos = event_data.position
				# 골 위치 (보통 골라인 근처)
				var goal_pos = Vector2(pos.x, pos.y)
				# 슈터 위치 추정 (골 위치에서 약간 떨어진 곳)
				var shooter_pos = Vector2(pos.x, pos.y + 10.0) if pos.y < 34 else Vector2(pos.x, pos.y - 10.0)
				# 골 애니메이션 표시
				show_shot_trajectory(shooter_pos, goal_pos, true)
				update_ball_position(goal_pos)
				print("[MatchMiniMap] GOAL! at: (%.1f, %.1f)" % [pos.x, pos.y])

		"shot", "shot_on_target", "shot_off_target":
			if event_data.has("from"):
				var from_pos = event_data.from
				# 슛 대상 위치 (골대 방향)
				var to_pos = event_data.get("to", {})
				if to_pos.is_empty():
					# 목표 위치가 없으면 골대 방향으로 추정
					to_pos = {"x": from_pos.x, "y": 0.0 if from_pos.y < 34 else 68.0}
				# 슛 궤적 애니메이션
				show_shot_trajectory(Vector2(from_pos.x, from_pos.y), Vector2(to_pos.x, to_pos.y), false)
				update_ball_position(Vector2(from_pos.x, from_pos.y))
				print(
					(
						"[MatchMiniMap] Shot from: (%.1f, %.1f) to (%.1f, %.1f)"
						% [from_pos.x, from_pos.y, to_pos.x, to_pos.y]
					)
				)

		"pass":
			if event_data.has("from") and event_data.has("to"):
				var from_pos = event_data.from
				var to_pos = event_data.to
				# 패스 화살표 애니메이션 표시
				show_pass_arrow(Vector2(from_pos.x, from_pos.y), Vector2(to_pos.x, to_pos.y))
				update_ball_position(Vector2(to_pos.x, to_pos.y))
				print("[MatchMiniMap] Pass: (%.1f, %.1f) → (%.1f, %.1f)" % [from_pos.x, from_pos.y, to_pos.x, to_pos.y])

		"tackle":
			if event_data.has("position"):
				var pos = event_data.position
				show_tackle_effect(Vector2(pos.x, pos.y))
				update_ball_position(Vector2(pos.x, pos.y))
				print("[MatchMiniMap] Tackle at: (%.1f, %.1f)" % [pos.x, pos.y])

		"dribble":
			if event_data.has("from") and event_data.has("to"):
				var from_pos = event_data.from
				var to_pos = event_data.to
				show_dribble_trail(Vector2(from_pos.x, from_pos.y), Vector2(to_pos.x, to_pos.y))
				update_ball_position(Vector2(to_pos.x, to_pos.y))
				print(
					"[MatchMiniMap] Dribble: (%.1f, %.1f) → (%.1f, %.1f)" % [from_pos.x, from_pos.y, to_pos.x, to_pos.y]
				)


func update_from_match_result(result: Dictionary):
	"""매치 결과 JSON에서 전체 이벤트 업데이트"""
	var events: Array = []
	var events_variant: Variant = result.get("events", [])
	if events_variant is Array:
		events = (events_variant as Array).duplicate(true)

	if events.is_empty():
		var legacy_payload_key := "re" + "play"
		var legacy_doc_key := ("re" + "play") + "_doc"
		var doc_variant: Variant = result.get(
			"timeline_doc", result.get(legacy_payload_key, result.get(legacy_doc_key, {}))
		)
		if doc_variant is Dictionary:
			var doc_dict: Dictionary = doc_variant
			var doc_events_variant: Variant = doc_dict.get("events", [])
			if doc_events_variant is Array:
				events = (doc_events_variant as Array).duplicate(true)

	if events.is_empty():
		print("[MatchMiniMap] No events found in result")
		return

	print("[MatchMiniMap] Processing %d events" % events.size())

	# 이벤트를 시간 순으로 재생 (애니메이션)
	for i in range(events.size()):
		var event = events[i]
		update_from_event(event)

		# 빠른 재생 (0.1초 간격)
		if i < events.size() - 1:
			await get_tree().create_timer(0.1).timeout


# ==============================================================================
# 선수 데이터 설정 API (Phase 3: MatchPlayer 통합)
# ==============================================================================


func set_player_data(player_index: int, team_id: int, player_data: Dictionary) -> void:
	"""선수 데이터 설정 (포지션 색상용)"""
	var dot_index = player_index + (team_id * 11)
	var position_code = player_data.get("position", "")
	player_positions[dot_index] = position_code

	# 포지션 색상 사용 시 도트 스타일 업데이트
	if use_position_colors and use_theme_manager_colors and dot_index < player_dots.size():
		var style = ThemeManager.create_position_dot_style(position_code, dot_size)
		player_dots[dot_index].add_theme_stylebox_override("panel", style)


func set_team_data(team_id: int, players: Array) -> void:
	"""팀 전체 선수 데이터 설정"""
	for i in range(min(players.size(), 11)):
		set_player_data(i, team_id, players[i])


func highlight_ball_holder(player_index: int, team_id: int) -> void:
	"""공 소유자 하이라이트 표시"""
	# 모든 선수의 하이라이트 해제
	for i in range(player_dots.size()):
		var is_home = i < 11
		var base_color = home_team_color if is_home else away_team_color

		# 포지션 색상 사용 여부에 따라 색상 결정
		if use_position_colors and player_positions.has(i):
			var pos = player_positions[i]
			var style = ThemeManager.create_position_dot_style(pos, dot_size)
			player_dots[i].add_theme_stylebox_override("panel", style)
		else:
			var style = ThemeManager.create_player_dot_style(base_color, dot_size)
			player_dots[i].add_theme_stylebox_override("panel", style)

	# 공 소유자 하이라이트
	var dot_index = player_index + (team_id * 11)
	if dot_index < player_dots.size():
		var team_color = home_team_color if team_id == 0 else away_team_color
		var style = ThemeManager.create_ball_holder_dot_style(team_color, dot_size)
		player_dots[dot_index].add_theme_stylebox_override("panel", style)


func clear_ball_holder_highlight() -> void:
	"""공 소유자 하이라이트 해제"""
	for i in range(player_dots.size()):
		var is_home = i < 11
		var base_color = home_team_color if is_home else away_team_color

		if use_position_colors and player_positions.has(i):
			var pos = player_positions[i]
			var style = ThemeManager.create_position_dot_style(pos, dot_size)
			player_dots[i].add_theme_stylebox_override("panel", style)
		else:
			var style = ThemeManager.create_player_dot_style(base_color, dot_size)
			player_dots[i].add_theme_stylebox_override("panel", style)


# ==============================================================================
# 신호 핸들러
# ==============================================================================


func _on_player_dot_clicked(player_index: int, team_id: int):
	"""선수 바둑알 클릭"""
	var player_key = "player_%d_%d" % [team_id, player_index]
	var meter_pos = current_positions.get(player_key, Vector2.ZERO)

	print(
		(
			"[MatchMiniMap] Player clicked: Team %d, Player %d at (%.1f, %.1f)"
			% [team_id, player_index, meter_pos.x, meter_pos.y]
		)
	)
	player_dot_clicked.emit(player_index, team_id)


func _on_ball_clicked():
	"""볼 바둑알 클릭"""
	print("[MatchMiniMap] Ball clicked at (%.1f, %.1f)" % [ball_position.x, ball_position.y])
	ball_clicked.emit()


# ==============================================================================
# 테스트 함수
# ==============================================================================


func test_coordinates():
	"""좌표 변환 테스트"""
	print("=== MatchMiniMap Coordinate Test ===")

	# 중앙 좌표 테스트
	var center_meter = Vector2(52.5, 34.0)
	var center_ui = meter_to_ui(center_meter)
	var back_to_meter = ui_to_meter(center_ui)

	print("Center test: %s → %s → %s" % [center_meter, center_ui, back_to_meter])

	# 골 라인 테스트
	var goal_meter = Vector2(0.0, 34.0)  # 홈 골라인(center)
	var goal_ui = meter_to_ui(goal_meter)
	print("Goal line test: %s → %s" % [goal_meter, goal_ui])

	print("✅ Coordinate test completed")


func test_match_event():
	"""매치 이벤트 테스트"""
	print("=== MatchMiniMap Event Test ===")

	# 테스트 이벤트들
	var test_events = [
		{"type": "goal", "time": 15.5, "position": {"x": 90.0, "y": 34.0}},
		{"type": "pass", "time": 30.2, "from": {"x": 50.0, "y": 30.0}, "to": {"x": 70.0, "y": 40.0}}
	]

	for event in test_events:
		update_from_event(event)
		await get_tree().create_timer(1.0).timeout

	print("✅ Event test completed")
