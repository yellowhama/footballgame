extends Camera2D
class_name MatchCamera
##
## MatchCamera - 가로형 경기 뷰어용 카메라
##
## 특징:
##   - Zoom Lock: 픽셀 퍼펙트를 위해 줌 고정
##   - Safe Box (Drag Margin): 공이 박스 안에 있으면 카메라 안 움직임
##   - Y축 고정: 가로 레이아웃이므로 상하 이동 최소화
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
##

#region Constants
const FIELD_PX_WIDTH: float = 1050.0  ## 105m × 10
const FIELD_PX_HEIGHT: float = 680.0  ## 68m × 10
const FIELD_MARGIN: float = 50.0  ## 경기장 밖 여유 공간

## Zoom Lock - 픽셀 퍼펙트 원칙
const DEFAULT_ZOOM := Vector2(2.0, 2.0)
#endregion

#region Export Variables
@export var target_node: Node2D = null  ## 추적 대상 (공)
@export var smoothing_speed: float = 5.0
#endregion

#region State
enum CameraMode {
	FULL_PITCH,  ## 전체 경기장 보기
	CAMERA_FOLLOW,  ## 공 따라가기
	TACTICAL_HALF,  ## 공격 진영 하프 코트
}

var current_mode: CameraMode = CameraMode.CAMERA_FOLLOW
var ball_position: Vector2 = Vector2(525, 340)  ## 센터 기본값
var follow_offset: Vector2 = Vector2.ZERO
var _shake_tween: Tween = null  ## Tween for camera shake effect
#endregion


func _ready() -> void:
	_setup_limits()
	_setup_smoothing()
	_setup_drag_margins()
	zoom = DEFAULT_ZOOM


func _process(_delta: float) -> void:
	match current_mode:
		CameraMode.CAMERA_FOLLOW:
			_update_follow_camera()
		CameraMode.FULL_PITCH:
			pass  ## 고정
		CameraMode.TACTICAL_HALF:
			pass  ## 고정


func _setup_limits() -> void:
	## 카메라가 경기장 밖을 비추지 않도록 제한
	limit_left = int(-FIELD_MARGIN)
	limit_top = int(-FIELD_MARGIN)
	limit_right = int(FIELD_PX_WIDTH + FIELD_MARGIN)
	limit_bottom = int(FIELD_PX_HEIGHT + FIELD_MARGIN)


func _setup_smoothing() -> void:
	position_smoothing_enabled = true
	position_smoothing_speed = smoothing_speed


func _setup_drag_margins() -> void:
	## Safe Box: 공이 이 박스 안에 있으면 카메라 안 움직임
	## 좌우 (Horizontal): 20% 마진 → 활발히 추적
	drag_horizontal_enabled = true
	drag_left_margin = 0.2
	drag_right_margin = 0.2

	## 상하 (Vertical): 비활성화 → Y축 고정
	drag_vertical_enabled = false


func _update_follow_camera() -> void:
	## Y축 강제 고정
	if not drag_vertical_enabled:
		global_position.y = FIELD_PX_HEIGHT / 2

	## X축: target_node 또는 ball_position 추적
	if target_node:
		global_position.x = target_node.global_position.x + follow_offset.x
	else:
		global_position.x = ball_position.x + follow_offset.x


## 공 위치 업데이트 (외부에서 호출)
func update_ball_position(new_pos: Vector2, velocity: Vector2 = Vector2.ZERO) -> void:
	ball_position = new_pos

	## 공이 움직이는 방향으로 카메라 약간 앞서기 (예측)
	if velocity.length() > 1.0:
		follow_offset = velocity.normalized() * 50.0  ## 50px 앞서기
	else:
		follow_offset = Vector2.ZERO


#region Camera Mode Methods
func set_full_pitch_view() -> void:
	current_mode = CameraMode.FULL_PITCH
	global_position = Vector2(FIELD_PX_WIDTH / 2, FIELD_PX_HEIGHT / 2)

	## 전체 경기장이 보이도록 줌 계산 (정수배 유지 시도)
	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	var zoom_x: float = viewport_size.x / (FIELD_PX_WIDTH + FIELD_MARGIN * 2)
	var zoom_y: float = viewport_size.y / (FIELD_PX_HEIGHT + FIELD_MARGIN * 2)
	var zoom_level: float = min(zoom_x, zoom_y)

	## 정수배에 가깝게 조정 (0.5, 1.0 등)
	zoom_level = _snap_to_integer_zoom(zoom_level)
	zoom = Vector2(zoom_level, zoom_level)


func set_follow_mode() -> void:
	current_mode = CameraMode.CAMERA_FOLLOW
	zoom = DEFAULT_ZOOM
	_setup_drag_margins()


func set_tactical_half_view(attacking_right: bool) -> void:
	current_mode = CameraMode.TACTICAL_HALF

	var half_center_x: float = FIELD_PX_WIDTH * 0.75 if attacking_right else FIELD_PX_WIDTH * 0.25
	global_position = Vector2(half_center_x, FIELD_PX_HEIGHT / 2)

	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	var half_width: float = FIELD_PX_WIDTH / 2 + FIELD_MARGIN
	var zoom_level: float = viewport_size.x / half_width
	zoom_level = _snap_to_integer_zoom(zoom_level)
	zoom = Vector2(zoom_level, zoom_level)


#endregion


#region Pixel Perfect Helpers
func _snap_to_integer_zoom(value: float) -> float:
	## 픽셀 퍼펙트를 위해 정수배 또는 0.5배에 스냅
	if value >= 1.75:
		return 2.0
	elif value >= 1.25:
		return 1.5
	elif value >= 0.75:
		return 1.0
	elif value >= 0.375:
		return 0.5
	else:
		return 0.25


#endregion

#region Effects (2025-12-11 Phase 9: 카메라 워크 강화)


## 카메라 셰이크 (임팩트 강조) - Tween-based for proper cleanup
func camera_shake(intensity: float = 5.0, duration: float = 0.3) -> void:
	# Kill any existing shake tween to prevent conflicts
	if _shake_tween and _shake_tween.is_valid():
		_shake_tween.kill()

	var original_offset := offset
	_shake_tween = create_tween()

	# Tween with decay (shake intensity decreases over time)
	_shake_tween.tween_method(
		func(t: float):
			if is_instance_valid(self):
				var decay = 1.0 - t
				offset = (
					original_offset
					+ Vector2(
						randf_range(-intensity * decay, intensity * decay),
						randf_range(-intensity * decay, intensity * decay)
					)
				),
		0.0,
		1.0,
		duration
	)

	# Restore original offset when complete
	_shake_tween.tween_callback(func(): offset = original_offset)


## 골 장면 슬로우 모션
func highlight_goal_moment(slow_duration: float = 2.0) -> void:
	Engine.time_scale = 0.3
	await get_tree().create_timer(slow_duration * 0.3).timeout  ## 실제 시간 기준
	Engine.time_scale = 1.0


## 슛/골 줌인 (2025-12-11)
## 임시로 줌인했다가 복원 (픽셀 퍼펙트 예외 허용)
var _is_zooming: bool = false


func zoom_in_for_shot(target_pos: Vector2, zoom_level: float = 2.5, duration: float = 0.8) -> void:
	if _is_zooming:
		return
	_is_zooming = true

	var original_zoom := zoom
	var original_pos := global_position
	var target_zoom := Vector2(zoom_level, zoom_level)

	## 부드러운 줌인 (Tween)
	var tween := create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "zoom", target_zoom, 0.2).set_ease(Tween.EASE_OUT)
	tween.tween_property(self, "global_position", target_pos, 0.2).set_ease(Tween.EASE_OUT)

	await tween.finished

	## 유지
	await get_tree().create_timer(duration * 0.5).timeout

	## 복원
	var restore_tween := create_tween()
	restore_tween.set_parallel(true)
	restore_tween.tween_property(self, "zoom", original_zoom, 0.3).set_ease(Tween.EASE_IN_OUT)
	restore_tween.tween_property(self, "global_position", original_pos, 0.3).set_ease(Tween.EASE_IN_OUT)

	await restore_tween.finished
	_is_zooming = false


## 골 줌인 + 슬로우모션 + 셰이크 (2025-12-11)
func zoom_in_for_goal(target_pos: Vector2) -> void:
	if _is_zooming:
		return
	_is_zooming = true

	var original_zoom := zoom
	var original_pos := global_position
	var goal_zoom := Vector2(3.0, 3.0)

	## 1. 빠른 줌인
	var tween := create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "zoom", goal_zoom, 0.15).set_ease(Tween.EASE_OUT)
	tween.tween_property(self, "global_position", target_pos, 0.15).set_ease(Tween.EASE_OUT)
	await tween.finished

	## 2. 슬로우 모션 + 셰이크
	Engine.time_scale = 0.3
	camera_shake(8.0, 0.15)  ## 충격 효과 (비동기)
	await get_tree().create_timer(0.6).timeout  ## 실제 0.18초

	## 3. 복원
	Engine.time_scale = 1.0
	var restore_tween := create_tween()
	restore_tween.set_parallel(true)
	restore_tween.tween_property(self, "zoom", original_zoom, 0.4).set_ease(Tween.EASE_IN_OUT)
	restore_tween.tween_property(self, "global_position", original_pos, 0.4).set_ease(Tween.EASE_IN_OUT)

	await restore_tween.finished
	_is_zooming = false


## 파울 줌인 (2025-12-11)
func zoom_in_for_foul(target_pos: Vector2) -> void:
	if _is_zooming:
		return
	_is_zooming = true

	var original_zoom := zoom
	var original_pos := global_position
	var foul_zoom := Vector2(2.8, 2.8)

	## 빠른 줌인
	var tween := create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "zoom", foul_zoom, 0.1).set_ease(Tween.EASE_OUT)
	tween.tween_property(self, "global_position", target_pos, 0.1).set_ease(Tween.EASE_OUT)
	await tween.finished

	## 짧은 셰이크
	camera_shake(4.0, 0.1)
	await get_tree().create_timer(0.3).timeout

	## 복원
	var restore_tween := create_tween()
	restore_tween.set_parallel(true)
	restore_tween.tween_property(self, "zoom", original_zoom, 0.3).set_ease(Tween.EASE_IN_OUT)
	restore_tween.tween_property(self, "global_position", original_pos, 0.3).set_ease(Tween.EASE_IN_OUT)

	await restore_tween.finished
	_is_zooming = false


#endregion


#region Input (줌 비활성화)
func _input(event: InputEvent) -> void:
	## 마우스 휠 줌 비활성화 (픽셀 퍼펙트 원칙)
	if event is InputEventMouseButton:
		if event.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			get_viewport().set_input_as_handled()
			return
#endregion
