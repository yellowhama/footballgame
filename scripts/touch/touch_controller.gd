extends Node

# 터치 및 스와이프 관리
signal swipe_detected(direction: String)
signal long_press(position: Vector2)
signal pinch_zoom(scale: float)

var swipe_start: Vector2
var swipe_threshold: float = 50.0
var is_swiping: bool = false
var long_press_time: float = 0.5
var press_timer: float = 0.0
var press_position: Vector2

# 모바일 최적화 설정
const MIN_TOUCH_SIZE = 44  # 최소 터치 크기 (픽셀)
const SWIPE_SPEED_THRESHOLD = 0.1  # 스와이프 속도 임계값


func _ready():
	print("Touch Controller initialized")
	set_process_input(true)
	set_process(true)


func _input(event):
	# 터치/마우스 입력 처리
	if event is InputEventScreenTouch or event is InputEventMouseButton:
		_handle_touch(event)
	elif event is InputEventScreenDrag or event is InputEventMouseMotion:
		_handle_drag(event)


func _handle_touch(event):
	if event.pressed:
		# 터치 시작
		swipe_start = event.position
		press_position = event.position
		is_swiping = true
		press_timer = 0.0
	else:
		# 터치 종료
		if is_swiping:
			_check_swipe(event.position)
		is_swiping = false
		press_timer = 0.0


func _handle_drag(event):
	if is_swiping:
		# 드래그 중
		var current_distance = event.position.distance_to(swipe_start)
		if current_distance > swipe_threshold:
			# 스와이프로 판정
			press_timer = -1.0  # 롱 프레스 취소


func _check_swipe(end_pos: Vector2):
	var swipe_vector = end_pos - swipe_start

	if swipe_vector.length() < swipe_threshold:
		return

	# 스와이프 방향 판정
	var direction = ""
	if abs(swipe_vector.x) > abs(swipe_vector.y):
		if swipe_vector.x > 0:
			direction = "right"
		else:
			direction = "left"
	else:
		if swipe_vector.y > 0:
			direction = "down"
		else:
			direction = "up"

	emit_signal("swipe_detected", direction)
	print("Swipe detected: ", direction)


func _process(delta):
	# 롱 프레스 체크
	if is_swiping and press_timer >= 0:
		press_timer += delta
		if press_timer >= long_press_time:
			emit_signal("long_press", press_position)
			print("Long press detected at: ", press_position)
			press_timer = -1.0  # 한 번만 발생


func adapt_ui_for_mobile(control: Control):
	"""UI 요소를 모바일용으로 최적화"""
	if not control:
		return

	# 모든 버튼 크기 조정
	for child in control.get_children():
		if child is Button:
			child.custom_minimum_size = Vector2(
				max(child.custom_minimum_size.x, MIN_TOUCH_SIZE), max(child.custom_minimum_size.y, MIN_TOUCH_SIZE)
			)
		elif child is Control:
			adapt_ui_for_mobile(child)  # 재귀적으로 처리


func is_mobile() -> bool:
	"""모바일 기기인지 확인"""
	return OS.has_feature("mobile") or OS.has_feature("web_android") or OS.has_feature("web_ios")
