extends Node

signal swipe_detected(direction: String)
signal tap_detected(position: Vector2)
signal long_press_detected(position: Vector2)

var touch_start_position: Vector2
var touch_start_time: int
var long_press_threshold: int = 500  # 500ms
var swipe_threshold: float = 50.0  # 50 pixels


func _ready():
	print("TouchController initialized")


func _input(event: InputEvent):
	if event is InputEventScreenTouch:
		if event.pressed:
			_touch_start(event.position)
		else:
			_touch_end(event.position)

	elif event is InputEventScreenDrag:
		_touch_drag(event.position)


func _touch_start(position: Vector2):
	touch_start_position = position
	touch_start_time = Time.get_ticks_msec()


func _touch_end(position: Vector2):
	var duration = Time.get_ticks_msec() - touch_start_time
	var distance = position.distance_to(touch_start_position)

	# 롱 프레스 체크
	if duration >= long_press_threshold:
		long_press_detected.emit(position)
		return

	# 스와이프 체크
	if distance >= swipe_threshold:
		var direction = _get_swipe_direction(touch_start_position, position)
		swipe_detected.emit(direction)
		return

	# 일반 탭
	tap_detected.emit(position)


func _touch_drag(position: Vector2):
	# 드래그 중에는 별도 처리 없음
	pass


func _get_swipe_direction(start: Vector2, end: Vector2) -> String:
	var delta = end - start
	var abs_x = abs(delta.x)
	var abs_y = abs(delta.y)

	if abs_x > abs_y:
		return "left" if delta.x < 0 else "right"
	else:
		return "up" if delta.y < 0 else "down"


func is_mobile() -> bool:
	"""모바일 환경인지 확인"""
	return OS.has_feature("mobile") or OS.has_feature("android") or OS.has_feature("ios")


func get_screen_size() -> Vector2:
	"""화면 크기 반환"""
	return get_viewport().size
