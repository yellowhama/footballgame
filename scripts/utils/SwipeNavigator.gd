extends Control
class_name SwipeNavigator

# 터치 제스처 네비게이션 시스템
# 스와이프, 핀치, 롱프레스 등 모바일 제스처 지원

signal swipe_left
signal swipe_right
signal swipe_up
signal swipe_down
signal pinch_zoom(scale_factor: float)
signal long_press(position: Vector2)
signal double_tap(position: Vector2)

# 설정 가능한 임계값들
@export var swipe_threshold: float = 100.0  # 최소 스와이프 거리
@export var swipe_velocity_threshold: float = 500.0  # 최소 스와이프 속도
@export var long_press_duration: float = 0.8  # 롱프레스 시간 (초)
@export var double_tap_time: float = 0.3  # 더블탭 인식 시간 (초)
@export var pinch_threshold: float = 50.0  # 핀치 인식 최소 거리

# 내부 상태 변수들
var touch_start_pos: Vector2
var touch_start_time: float
var is_touching: bool = false
var is_swiping: bool = false
var long_press_timer: Timer
var last_tap_time: float = 0.0
var last_tap_pos: Vector2

# 멀티터치 지원
var touches: Dictionary = {}
var initial_pinch_distance: float = 0.0
var is_pinching: bool = false


func _ready():
	# 롱프레스 타이머 설정
	long_press_timer = Timer.new()
	long_press_timer.wait_time = long_press_duration
	long_press_timer.one_shot = true
	long_press_timer.timeout.connect(_on_long_press_timeout)
	add_child(long_press_timer)

	# 모든 입력 이벤트를 받도록 설정
	set_process_unhandled_input(true)


func _unhandled_input(event):
	if event is InputEventScreenTouch:
		handle_touch_event(event)
	elif event is InputEventScreenDrag:
		handle_drag_event(event)


func handle_touch_event(event: InputEventScreenTouch):
	var touch_index = event.index

	if event.pressed:
		# 터치 시작
		touches[touch_index] = {
			"start_pos": event.position, "current_pos": event.position, "start_time": Time.get_time_dict_from_system()
		}

		if touch_index == 0:  # 첫 번째 터치
			touch_start_pos = event.position
			touch_start_time = (
				Time.get_time_dict_from_system()["second"] + Time.get_time_dict_from_system()["minute"] * 60
			)
			is_touching = true
			is_swiping = false

			# 롱프레스 타이머 시작
			long_press_timer.start()

			# 더블탭 검사
			check_double_tap(event.position)

		elif touch_index == 1 and touches.has(0):  # 두 번째 터치 (핀치 시작)
			start_pinch()

	else:
		# 터치 종료
		if touch_index == 0 and is_touching:
			is_touching = false
			long_press_timer.stop()

			if is_swiping:
				process_swipe(event.position)
				is_swiping = false

		if touches.has(touch_index):
			touches.erase(touch_index)

		if is_pinching and touches.size() < 2:
			end_pinch()


func handle_drag_event(event: InputEventScreenDrag):
	var touch_index = event.index

	if touches.has(touch_index):
		touches[touch_index]["current_pos"] = event.position

	if touch_index == 0 and is_touching:
		var drag_distance = event.position.distance_to(touch_start_pos)

		if drag_distance > swipe_threshold:
			is_swiping = true
			long_press_timer.stop()  # 드래그 중에는 롱프레스 취소

	# 핀치 제스처 처리
	if is_pinching and touches.size() >= 2:
		process_pinch()


func process_swipe(end_pos: Vector2):
	var swipe_vector = end_pos - touch_start_pos
	var swipe_distance = swipe_vector.length()

	if swipe_distance < swipe_threshold:
		return

	# 속도 계산
	var current_time = Time.get_time_dict_from_system()["second"] + Time.get_time_dict_from_system()["minute"] * 60
	var swipe_time = current_time - touch_start_time
	var swipe_velocity = swipe_distance / max(swipe_time, 0.01)

	if swipe_velocity < swipe_velocity_threshold:
		return

	var swipe_direction = swipe_vector.normalized()

	# 방향 결정 (가로/세로 중 더 큰 성분)
	if abs(swipe_direction.x) > abs(swipe_direction.y):
		# 가로 스와이프
		if swipe_direction.x > 0:
			emit_signal("swipe_right")
			print("Swipe right detected")
		else:
			emit_signal("swipe_left")
			print("Swipe left detected")
	else:
		# 세로 스와이프
		if swipe_direction.y > 0:
			emit_signal("swipe_down")
			print("Swipe down detected")
		else:
			emit_signal("swipe_up")
			print("Swipe up detected")


func check_double_tap(pos: Vector2):
	var current_time = Time.get_time_dict_from_system()["second"] + Time.get_time_dict_from_system()["minute"] * 60
	var time_diff = current_time - last_tap_time
	var pos_diff = pos.distance_to(last_tap_pos)

	if time_diff < double_tap_time and pos_diff < 50:  # 50픽셀 이내
		emit_signal("double_tap", pos)
		print("Double tap detected at: ", pos)
		last_tap_time = 0  # 더블탭 후 리셋
	else:
		last_tap_time = current_time
		last_tap_pos = pos


func start_pinch():
	if touches.size() >= 2:
		var touch_keys = touches.keys()
		var pos1 = touches[touch_keys[0]]["current_pos"]
		var pos2 = touches[touch_keys[1]]["current_pos"]
		initial_pinch_distance = pos1.distance_to(pos2)
		is_pinching = true
		print("Pinch started, initial distance: ", initial_pinch_distance)


func process_pinch():
	if touches.size() >= 2:
		var touch_keys = touches.keys()
		var pos1 = touches[touch_keys[0]]["current_pos"]
		var pos2 = touches[touch_keys[1]]["current_pos"]
		var current_distance = pos1.distance_to(pos2)

		if initial_pinch_distance > 0:
			var scale_factor = current_distance / initial_pinch_distance
			emit_signal("pinch_zoom", scale_factor)


func end_pinch():
	is_pinching = false
	initial_pinch_distance = 0.0
	print("Pinch ended")


func _on_long_press_timeout():
	if is_touching and not is_swiping:
		emit_signal("long_press", touch_start_pos)
		print("Long press detected at: ", touch_start_pos)


# 햅틱 피드백 제공
func provide_haptic_feedback(intensity: int = 50):
	if OS.has_feature("mobile"):
		Input.vibrate_handheld(intensity)


# 제스처 설정 메서드들
func set_swipe_threshold(threshold: float):
	swipe_threshold = threshold


func set_long_press_duration(duration: float):
	long_press_duration = duration
	if long_press_timer:
		long_press_timer.wait_time = duration


func set_double_tap_time(time: float):
	double_tap_time = time


# 특정 씬에 맞는 제스처 바인딩
func setup_training_screen_gestures():
	# 훈련 화면용 제스처 설정
	connect("swipe_left", _on_training_swipe_left)
	connect("swipe_right", _on_training_swipe_right)
	connect("double_tap", _on_training_double_tap)


func setup_stats_screen_gestures():
	# 스탯 화면용 제스처 설정
	connect("swipe_left", _on_stats_swipe_left)
	connect("swipe_right", _on_stats_swipe_right)
	connect("pinch_zoom", _on_stats_pinch_zoom)


# 제스처 핸들러 예시들
func _on_training_swipe_left():
	# 이전 훈련 카테고리로
	pass


func _on_training_swipe_right():
	# 다음 훈련 카테고리로
	pass


func _on_training_double_tap(pos: Vector2):
	# 빠른 훈련 선택
	provide_haptic_feedback(30)


func _on_stats_swipe_left():
	# 이전 스탯 탭으로
	pass


func _on_stats_swipe_right():
	# 다음 스탯 탭으로
	pass


func _on_stats_pinch_zoom(scale: float):
	# 스탯 차트 확대/축소
	pass
