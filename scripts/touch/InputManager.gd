extends Node
# InputManager - 간단한 터치 입력 관리 (Godot 4.4 호환)

signal touch_started(position: Vector2)
signal touch_ended(position: Vector2)
signal touch_moved(position: Vector2, relative: Vector2)
signal tap(position: Vector2)
signal swipe(direction: Vector2, velocity: Vector2)

var touch_active: bool = false
var touch_start_position: Vector2
var touch_last_position: Vector2


func _ready():
	# 터치 입력 활성화
	set_process_unhandled_input(true)


func _unhandled_input(event: InputEvent):
	if event is InputEventScreenTouch:
		_handle_screen_touch(event)
	elif event is InputEventScreenDrag:
		_handle_screen_drag(event)


func _handle_screen_touch(event: InputEventScreenTouch):
	if event.pressed:
		# 터치 시작
		touch_active = true
		touch_start_position = event.position
		touch_last_position = event.position
		touch_started.emit(event.position)
	else:
		# 터치 종료
		if touch_active:
			touch_active = false
			touch_ended.emit(event.position)

			# 탭 감지 (짧은 거리)
			var distance = touch_start_position.distance_to(event.position)
			if distance < 50:  # 50픽셀 이내면 탭
				tap.emit(event.position)
			else:
				# 스와이프 감지
				var direction = (event.position - touch_start_position).normalized()
				var velocity = (event.position - touch_last_position) / get_process_delta_time()
				swipe.emit(direction, velocity)


func _handle_screen_drag(event: InputEventScreenDrag):
	if touch_active:
		touch_moved.emit(event.position, event.relative)
		touch_last_position = event.position


func is_touch_active() -> bool:
	return touch_active


func get_touch_position() -> Vector2:
	return touch_last_position if touch_active else Vector2.ZERO
