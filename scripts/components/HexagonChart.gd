extends Control
class_name HexagonChart

## 육각형 차트 컴포넌트
## 6개 스탯을 시각적으로 표현하는 레이더 차트

signal stat_hovered(stat_name: String, value: float)
signal stat_clicked(stat_name: String, value: float)

@export_group("Chart Settings")
@export var chart_size: float = 200.0
@export var chart_color: Color = ThemeManager.PRIMARY
@export var background_color: Color = Color(0.2, 0.2, 0.2, 0.3)
@export var border_color: Color = Color(1, 1, 1, 0.5)
@export var show_labels: bool = true
@export var show_values: bool = true
@export var animate_changes: bool = true

@export_group("Stats")
@export var max_value: float = 100.0
@export var stats: Dictionary = {
	"PACE": 50.0, "POWER": 50.0, "TECHNICAL": 50.0, "SHOOTING": 50.0, "PASSING": 50.0, "DEFENDING": 50.0  # ⚡ 스피드/가속력/민첩성/밸런스  # 💪 근력/점프력/스태미나/헤딩  # ⚽ 기술/드리블링/퍼스트터치/볼컨트롤  # 🎯 마무리/중거리슛/페널티킥/침착함  # 🔄 패스/크로스/시야/스루패스  # 🛡️ 마킹/태클/포지셔닝/예측력
}

# 내부 변수
var _current_stats: Dictionary = {}
var _target_stats: Dictionary = {}
var _animation_progress: float = 1.0
var _hover_stat: String = ""
var _center: Vector2
var _vertices: Array[Vector2] = []
var _stat_positions: Dictionary = {}

# 스탯 레이블 한글화 (FM/우마무스메 스타일)
var stat_labels: Dictionary = {
	"PACE": "페이스", "POWER": "파워", "TECHNICAL": "테크닉", "SHOOTING": "슈팅", "PASSING": "패스", "DEFENDING": "수비"  # ⚡ 속도 능력  # 💪 신체 능력  # ⚽ 기술 능력  # 🎯 결정력  # 🔄 플레이메이킹  # 🛡️ 수비 능력
}

# Alias map to resolve incoming stat keys to our canonical keys
var STAT_KEY_MAP: Dictionary = {
	# canonical 6
	"pace": "PACE",
	"shooting": "SHOOTING",
	"passing": "PASSING",
	"dribbling": "TECHNICAL",
	"technical": "TECHNICAL",
	"defending": "DEFENDING",
	"defense": "DEFENDING",
	"physical": "POWER",
	"power": "POWER",
	# tolerate uppercase/lowercase inputs as well
	"PACE": "PACE",
	"SHOOTING": "SHOOTING",
	"PASSING": "PASSING",
	"TECHNICAL": "TECHNICAL",
	"DEFENDING": "DEFENDING",
	"POWER": "POWER"
}

var STAT_SYNONYMS: Dictionary = {
	"pace": ["speed", "SPEED"],
	"power": ["physical", "PHYSICAL"],
	"technical": ["technique", "TECHNIQUE"],
	"defending": ["defense", "DEFENSE"]
}


func _ready():
	_current_stats = stats.duplicate()
	_target_stats = stats.duplicate()
	custom_minimum_size = Vector2(chart_size * 2, chart_size * 2)
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)


func _draw():
	_center = size / 2

	# 배경 육각형 그리기
	_draw_background_hexagon()

	# 그리드 라인 그리기
	_draw_grid_lines()

	# 스탯 육각형 그리기
	_draw_stat_hexagon()

	# 라벨 그리기
	if show_labels:
		_draw_labels()

	# 값 표시
	if show_values:
		_draw_values()

	# 호버 효과
	if _hover_stat != "":
		_draw_hover_effect()


func _draw_background_hexagon():
	var points: PackedVector2Array = []
	for i in range(6):
		var angle = PI / 2 + (i * PI / 3)
		var point = _center + Vector2(cos(angle), sin(angle)) * chart_size
		points.append(point)
		_vertices.append(point)

	# 배경 채우기
	draw_colored_polygon(points, background_color)

	# 테두리 그리기
	for i in range(6):
		var next = (i + 1) % 6
		draw_line(points[i], points[next], border_color, 2.0)


func _draw_grid_lines():
	# 중심에서 각 꼭지점으로 선 그리기
	for vertex in _vertices:
		draw_line(_center, vertex, Color(1, 1, 1, 0.1), 1.0)

	# 동심원 그리기 (25%, 50%, 75%)
	for level in [0.25, 0.5, 0.75]:
		var grid_points: PackedVector2Array = []
		for i in range(6):
			var angle = PI / 2 + (i * PI / 3)
			var point = _center + Vector2(cos(angle), sin(angle)) * (chart_size * level)
			grid_points.append(point)

		for i in range(6):
			var next = (i + 1) % 6
			draw_line(grid_points[i], grid_points[next], Color(1, 1, 1, 0.05), 1.0)


func _draw_stat_hexagon():
	var stat_points: PackedVector2Array = []
	var stat_names = ["pace", "shooting", "passing", "dribbling", "defending", "physical"]

	for i in range(6):
		var stat_name = stat_names[i]
		var value = _get_stat_value(stat_name)
		var normalized = clamp(value / max_value, 0.0, 1.0)

		var angle = PI / 2 + (i * PI / 3)
		var point = _center + Vector2(cos(angle), sin(angle)) * (chart_size * normalized)
		stat_points.append(point)
		_stat_positions[stat_name] = point

	# 스탯 영역 채우기 (반투명)
	var fill_color = chart_color
	fill_color.a = 0.4
	draw_colored_polygon(stat_points, fill_color)

	# 스탯 테두리 그리기
	for i in range(6):
		var next = (i + 1) % 6
		draw_line(stat_points[i], stat_points[next], chart_color, 3.0)

	# 스탯 포인트 그리기
	for point in stat_points:
		draw_circle(point, 5.0, chart_color)
		draw_circle(point, 3.0, Color.WHITE)


func _draw_labels():
	var stat_names = ["PACE", "SHOOTING", "PASSING", "TECHNICAL", "DEFENDING", "POWER"]
	var font = ThemeDB.fallback_font
	var font_size = 16

	for i in range(6):
		var stat_name = stat_names[i]
		var label = stat_labels.get(stat_name, stat_name)

		var angle = PI / 2 + (i * PI / 3)
		var label_pos = _center + Vector2(cos(angle), sin(angle)) * (chart_size + 30)

		# 텍스트 중앙 정렬을 위한 크기 계산
		var text_size = font.get_string_size(label, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size)
		label_pos -= text_size / 2

		# 그림자 효과
		draw_string(
			font, label_pos + Vector2(1, 1), label, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size, Color(0, 0, 0, 0.5)
		)

		# 실제 텍스트
		var text_color = ThemeManager.TEXT_PRIMARY
		if stat_name == _hover_stat:
			text_color = ThemeManager.ACCENT

		draw_string(font, label_pos, label, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size, text_color)


func _draw_values():
	if _hover_stat == "":
		return

	var font = ThemeDB.fallback_font
	var font_size = 14

	# 호버된 스탯의 값 표시
	var value = _get_stat_value(_hover_stat)
	var display_value = int(value)
	var text = str(display_value) + "/" + str(int(max_value))

	var text_pos = get_local_mouse_position() + Vector2(20, -20)

	# 배경 박스
	var text_size = font.get_string_size(text, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size)
	var padding = 8
	var bg_rect = Rect2(
		text_pos - Vector2(padding, text_size.y + padding), text_size + Vector2(padding * 2, padding * 2)
	)

	# 'width' has no effect when filled=true; use default outline
	draw_rect(bg_rect, Color(0, 0, 0, 0.8), true)

	# 값 텍스트
	draw_string(font, text_pos, text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, ThemeManager.TEXT_HIGHLIGHT)


func _draw_hover_effect():
	if _hover_stat == "" or not _stat_positions.has(_hover_stat):
		return

	var pos = _stat_positions[_hover_stat]

	# 호버 포인트 강조
	draw_circle(pos, 8.0, ThemeManager.ACCENT)
	draw_circle(pos, 6.0, Color.WHITE)

	# 중심에서 호버 포인트까지 선 강조
	draw_line(_center, pos, ThemeManager.ACCENT, 2.0)


func _canonical_key(key: String) -> String:
	# Direct mapping
	if STAT_KEY_MAP.has(key):
		return String(STAT_KEY_MAP[key])
	# Try case-normalization
	var upper = key.to_upper()
	if STAT_KEY_MAP.has(upper):
		return String(STAT_KEY_MAP[upper])
	# Try synonyms
	var lower = key.to_lower()
	if STAT_SYNONYMS.has(lower):
		for alt in STAT_SYNONYMS[lower]:
			if STAT_KEY_MAP.has(alt):
				return String(STAT_KEY_MAP[alt])
	# Fallback to provided upper key (may already be canonical)
	return upper


func _normalize_stats_dict(src: Dictionary) -> Dictionary:
	var out: Dictionary = {}
	for k in src.keys():
		var canon = _canonical_key(String(k))
		# Only accept the six canonical keys
		if canon in ["PACE", "SHOOTING", "PASSING", "TECHNICAL", "DEFENDING", "POWER"]:
			out[canon] = float(src[k])
	return out


func set_stats(new_stats: Dictionary, animate: bool = true):
	# Normalize incoming keys to canonical set to avoid 0s from alias keys
	_target_stats = _normalize_stats_dict(new_stats)

	if not animate or not animate_changes:
		_current_stats = _target_stats.duplicate()
		_animation_progress = 1.0
		queue_redraw()
	else:
		_animation_progress = 0.0
		var tween = get_tree().create_tween()
		tween.tween_property(self, "_animation_progress", 1.0, 0.5).set_ease(Tween.EASE_OUT).set_trans(
			Tween.TRANS_CUBIC
		)
		tween.tween_callback(queue_redraw)


func update_stat(stat_name: String, new_value: float, animate: bool = true):
	var canon = _canonical_key(stat_name)
	# Initialize target store if missing
	if not _target_stats.has(canon) and stats.has(canon):
		_target_stats[canon] = float(stats.get(canon, 0.0))

	_target_stats[canon] = clamp(new_value, 0.0, max_value)

	if not animate or not animate_changes:
		_current_stats[canon] = _target_stats[canon]
		queue_redraw()
	else:
		var tween = get_tree().create_tween()
		var start_value = float(_current_stats.get(canon, 0.0))
		var end_value = float(_target_stats.get(canon, 0.0))
		(
			tween
			. tween_method(
				func(interpolated_value):
					_current_stats[canon] = interpolated_value
					queue_redraw(),
				start_value,
				end_value,
				0.3
			)
			. set_ease(Tween.EASE_OUT)
			. set_trans(Tween.TRANS_CUBIC)
		)


func _process(_delta):
	if _animation_progress < 1.0:
		# 애니메이션 중 보간
		for stat_name in _current_stats:
			if _target_stats.has(stat_name):
				var current = float(_current_stats[stat_name])
				var target = float(_target_stats[stat_name])
				_current_stats[stat_name] = lerp(current, target, _animation_progress)
		queue_redraw()


func _gui_input(event):
	if event is InputEventMouseMotion:
		_check_hover(event.position)
	elif event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			_check_click(event.position)


func _check_hover(mouse_pos: Vector2):
	var closest_stat = ""
	var min_distance = 30.0  # 최소 거리 임계값

	for stat_name in _stat_positions:
		var stat_pos = _stat_positions[stat_name]
		var distance = mouse_pos.distance_to(stat_pos)

		if distance < min_distance:
			min_distance = distance
			closest_stat = stat_name

	if closest_stat != _hover_stat:
		_hover_stat = closest_stat
		queue_redraw()

		if _hover_stat != "":
			stat_hovered.emit(_hover_stat, _get_stat_value(_hover_stat))


func _check_click(mouse_pos: Vector2):
	for stat_name in _stat_positions:
		var stat_pos = _stat_positions[stat_name]
		var distance = mouse_pos.distance_to(stat_pos)

		if distance < 30.0:
			stat_clicked.emit(stat_name, _get_stat_value(stat_name))
			break


func _on_mouse_entered():
	pass


func _on_mouse_exited():
	_hover_stat = ""
	queue_redraw()


func get_stat_color(stat_name: String) -> Color:
	var value = _get_stat_value(stat_name)
	return ThemeManager.get_stat_color(value, max_value)


func get_overall_rating() -> float:
	var total = 0.0
	var count = 0

	for stat_name in _current_stats:
		total += _current_stats[stat_name]
		count += 1

	return total / count if count > 0 else 0.0


func sync_with_player_data():
	"""EnhancedPlayerData에서 6각형 스탯 동기화"""
	if not is_inside_tree():
		return

	var player_data = get_node_or_null("/root/EnhancedPlayerData")
	if player_data:
		# 먼저 6각형 스탯 업데이트 요청
		player_data.update_hexagon_stats()

		# 업데이트된 스탯 가져오기
		var new_stats = {
			"PACE": player_data.pace_stat,
			"POWER": player_data.power_stat,
			"TECHNICAL": player_data.technical_stat,
			"SHOOTING": player_data.shooting_stat,
			"PASSING": player_data.passing_stat,
			"DEFENDING": player_data.defending_stat
		}
		set_stats(new_stats, true)


func get_stat_order() -> Array:
	"""육각형 순서대로 스탯 이름 반환"""
	return ["PACE", "SHOOTING", "PASSING", "TECHNICAL", "DEFENDING", "POWER"]


func _get_stat_value(stat_name: String) -> float:
	# Prefer direct key as-is
	if _current_stats.has(stat_name):
		return float(_current_stats.get(stat_name, 0.0))
	# Map lowercase display name to canonical stored key
	var canon: String = ""
	if STAT_KEY_MAP.has(stat_name):
		canon = String(STAT_KEY_MAP[stat_name])
	else:
		canon = stat_name.to_upper()
	if canon != "" and _current_stats.has(canon):
		return float(_current_stats.get(canon, 0.0))
	# Try synonyms (e.g., pace → speed)
	if STAT_SYNONYMS.has(stat_name):
		for alt in STAT_SYNONYMS[stat_name]:
			if _current_stats.has(alt):
				return float(_current_stats.get(alt, 0.0))
	return 0.0
