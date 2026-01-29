extends Control
## HexagonChart - 선수 속성 시각화를 위한 6축 레이더 차트
## FM 스타일 헥사곤 차트 컴포넌트
##
## 작성일: 2025-11-27

signal value_clicked(attribute_name: String)

# ============================================
# 설정
# ============================================

## 6개 축 이름 (순서대로 시계방향)
@export var attribute_names: PackedStringArray = ["기술", "피지컬", "멘탈", "수비", "공격", "스피드"]

## 각 축의 값 (0-100)
@export var attribute_values: PackedFloat32Array = [70.0, 65.0, 80.0, 55.0, 75.0, 85.0]

## 차트 반지름
@export var radius: float = 100.0

## 배경 그리드 선 색상
@export var grid_color: Color = Color(0.3, 0.3, 0.3, 0.5)

## 축 라벨 색상
@export var label_color: Color = Color(0.9, 0.9, 0.9, 1.0)

## 데이터 영역 색상
@export var fill_color: Color = Color(0.2, 0.6, 1.0, 0.3)

## 데이터 테두리 색상
@export var stroke_color: Color = Color(0.2, 0.6, 1.0, 1.0)

## 그리드 레벨 수 (동심원 개수)
@export var grid_levels: int = 5

## 폰트 크기
@export var font_size: int = 14

# ============================================
# 내부 변수
# ============================================

var _center: Vector2
var _angles: PackedFloat32Array = []

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_calculate_angles()
	_center = size / 2.0
	queue_redraw()


func _calculate_angles() -> void:
	_angles.clear()
	var angle_step = TAU / 6.0  # 360도 / 6축 = 60도

	# 12시 방향부터 시작 (위쪽)
	var start_angle = -PI / 2.0

	for i in range(6):
		_angles.append(start_angle + i * angle_step)


# ============================================
# 공개 API
# ============================================


func set_values(values: PackedFloat32Array) -> void:
	"""속성 값 설정 (0-100 범위)"""
	if values.size() != 6:
		push_error("[HexagonChart] values must have exactly 6 elements")
		return

	attribute_values = values
	queue_redraw()


func set_attribute_names(names: PackedStringArray) -> void:
	"""축 이름 설정"""
	if names.size() != 6:
		push_error("[HexagonChart] names must have exactly 6 elements")
		return

	attribute_names = names
	queue_redraw()


# ============================================
# 그리기
# ============================================


func _draw() -> void:
	_center = size / 2.0

	# 배경 그리드
	_draw_grid()

	# 데이터 폴리곤
	_draw_data_polygon()

	# 축 라인
	_draw_axes()

	# 라벨
	_draw_labels()


func _draw_grid() -> void:
	"""배경 동심 헥사곤 그리드"""
	for level in range(1, grid_levels + 1):
		var level_radius = radius * (level / float(grid_levels))
		var points: PackedVector2Array = []

		for i in range(6):
			var angle = _angles[i]
			var point = _center + Vector2(cos(angle) * level_radius, sin(angle) * level_radius)
			points.append(point)

		# 헥사곤 그리기
		for i in range(6):
			var next_i = (i + 1) % 6
			draw_line(points[i], points[next_i], grid_color, 1.0)


func _draw_axes() -> void:
	"""중심에서 각 꼭지점까지 축 라인"""
	for i in range(6):
		var angle = _angles[i]
		var end_point = _center + Vector2(cos(angle) * radius, sin(angle) * radius)
		draw_line(_center, end_point, grid_color, 1.0)


func _draw_data_polygon() -> void:
	"""실제 데이터 값을 나타내는 폴리곤"""
	if attribute_values.size() != 6:
		return

	var points: PackedVector2Array = []

	for i in range(6):
		var value = clamp(attribute_values[i], 0.0, 100.0)
		var normalized = value / 100.0
		var angle = _angles[i]

		var point = _center + Vector2(cos(angle) * radius * normalized, sin(angle) * radius * normalized)
		points.append(point)

	# 채우기
	draw_colored_polygon(points, fill_color)

	# 테두리
	for i in range(6):
		var next_i = (i + 1) % 6
		draw_line(points[i], points[next_i], stroke_color, 2.0)

	# 꼭지점 원
	for point in points:
		draw_circle(point, 4.0, stroke_color)


func _draw_labels() -> void:
	"""각 축의 이름과 값 표시"""
	if attribute_names.size() != 6 or attribute_values.size() != 6:
		return

	for i in range(6):
		var angle = _angles[i]
		var label_distance = radius + 25.0

		var label_pos = _center + Vector2(cos(angle) * label_distance, sin(angle) * label_distance)

		# 라벨 텍스트
		var label_text = "%s\n%.0f" % [attribute_names[i], attribute_values[i]]

		# 텍스트 정렬 조정
		var text_size = Vector2(60, 40)
		var adjusted_pos = label_pos - text_size / 2.0

		# 배경
		draw_rect(Rect2(adjusted_pos, text_size), Color(0.1, 0.1, 0.1, 0.7))

		# 텍스트 (간단한 방법 - 실제로는 Label 노드 사용 권장)
		var font = ThemeDB.fallback_font
		var font_color = label_color

		# 이름
		draw_string(
			font,
			adjusted_pos + Vector2(5, 15),
			attribute_names[i],
			HORIZONTAL_ALIGNMENT_LEFT,
			-1,
			font_size,
			font_color
		)

		# 값
		var value_str = "%.0f" % attribute_values[i]
		draw_string(
			font,
			adjusted_pos + Vector2(5, 32),
			value_str,
			HORIZONTAL_ALIGNMENT_LEFT,
			-1,
			font_size,
			Color(1.0, 0.8, 0.2, 1.0)
		)


# ============================================
# 입력 처리
# ============================================


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			_handle_click(event.position)


func _handle_click(pos: Vector2) -> void:
	"""클릭한 영역의 속성 확인"""
	var rel_pos = pos - _center
	var angle = atan2(rel_pos.y, rel_pos.x)

	# 각도를 0-TAU 범위로 정규화
	if angle < 0:
		angle += TAU

	# 어느 축에 가까운지 판단
	var closest_index = 0
	var min_diff = abs(_wrap_angle_diff(angle, _angles[0]))

	for i in range(1, 6):
		var diff = abs(_wrap_angle_diff(angle, _angles[i]))
		if diff < min_diff:
			min_diff = diff
			closest_index = i

	value_clicked.emit(attribute_names[closest_index])


func _wrap_angle_diff(a: float, b: float) -> float:
	"""두 각도 차이 계산 (-PI ~ PI)"""
	var diff = a - b
	while diff > PI:
		diff -= TAU
	while diff < -PI:
		diff += TAU
	return diff


# ============================================
# 리사이즈
# ============================================


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_center = size / 2.0
		queue_redraw()
