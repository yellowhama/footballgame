@tool
extends Camera3D
class_name QuarterViewSmartCineCamera

# ===============================
# ⚙️ 기본 설정
# ===============================
@export var field_node_path: NodePath
@export var players_group_path: NodePath
@export var ball_node_path: NodePath

@export var tilt_angle_deg: float = 45.0
@export var margin_ratio: float = 1.25
@export var min_height: float = 10.0
@export var max_height: float = 80.0
@export var look_direction: Vector3 = Vector3(0, 0, -1)

# ===============================
# 🎬 시네마틱 옵션
# ===============================
@export var enable_orbit: bool = false
@export var orbit_speed_deg_per_sec: float = 10.0

@export var enable_tracking: bool = false
@export var tracking_smoothness: float = 0.1  # 0.0(즉시) ~ 1.0(매우 부드럽게)

# 컷 시퀀스(자동 전환용)
@export var cut_sequence_angles: Array[float] = [30.0, 60.0, 90.0, 120.0, 150.0, 180.0]
@export var cut_sequence_interval: float = 3.0
var _cut_sequence_timer := 0.0
var _cut_sequence_index := 0
@export var enable_cut_sequence: bool = false

# ===============================
# 내부 계산 상태
# ===============================
var scene_center: Vector3
var scene_size: Vector3
var ball_last_pos: Vector3
var orbit_angle_deg: float = 0.0


# ===============================
# 🚀 초기화
# ===============================
func _ready():
	call_deferred("_setup_camera")


func _setup_camera():
	if not is_inside_tree():
		return
	var merged_aabb = AABB()
	merged_aabb = _merge_all_targets()

	if merged_aabb.size == Vector3.ZERO:
		push_warning("[SmartCineCamera] ⚠️ AABB 크기가 0입니다. 노드 경로 확인 필요.")
		return

	scene_center = merged_aabb.position + merged_aabb.size * 0.5
	scene_size = merged_aabb.size
	ball_last_pos = scene_center
	_recalculate_camera_transform()

	_log_scene_info(merged_aabb)


# ===============================
# 🧭 AABB 수집
# ===============================
func _merge_all_targets() -> AABB:
	var merged_aabb = AABB()
	var field = get_node_or_null(field_node_path)
	if field:
		merged_aabb = merged_aabb.merge(_get_aabb_recursive(field))

	var players_root = get_node_or_null(players_group_path)
	if players_root:
		for p in players_root.get_children():
			if p is Node3D:
				merged_aabb = merged_aabb.merge(_get_aabb_recursive(p))

	var ball = get_node_or_null(ball_node_path)
	if ball:
		merged_aabb = merged_aabb.merge(_get_aabb_recursive(ball))

	return merged_aabb


func _get_aabb_recursive(node: Node3D) -> AABB:
	var result := AABB()
	if node is MeshInstance3D:
		result = node.get_aabb()
		result.position += node.global_position
	for child in node.get_children():
		if child is Node3D:
			result = result.merge(_get_aabb_recursive(child))
	return result


# ===============================
# 📏 카메라 위치 계산
# ===============================
func _recalculate_camera_transform():
	var avg_size = (scene_size.x + scene_size.z) * 0.5
	var distance = avg_size * 0.9 * margin_ratio
	var height = clamp(distance * 0.75, min_height, max_height)

	var offset = look_direction.normalized() * -distance
	offset.y += height
	if not is_inside_tree():
		return
	global_position = scene_center + offset
	look_at(scene_center, Vector3.UP)


# ===============================
# 🎥 실시간 업데이트
# ===============================
func _process(delta):
	if enable_tracking:
		_update_tracking(delta)
	if enable_orbit:
		_update_orbit(delta)
	if enable_cut_sequence:
		_update_cut_sequence(delta)


# -------------------------------
# 🎯 트래킹 (공 따라가기)
# -------------------------------
func _update_tracking(delta):
	var ball = get_node_or_null(ball_node_path)
	if not ball:
		return
	if not is_inside_tree():
		return
	var target_pos = ball.global_position
	ball_last_pos = lerp(ball_last_pos, target_pos, tracking_smoothness)
	look_at(ball_last_pos, Vector3.UP)


# -------------------------------
# 🔄 오비트 (자동 회전)
# -------------------------------
func _update_orbit(delta):
	orbit_angle_deg += orbit_speed_deg_per_sec * delta
	var orbit_rad = deg_to_rad(orbit_angle_deg)
	var distance = (scene_size.x + scene_size.z) * 0.5 * margin_ratio
	var height = clamp(distance * 0.75, min_height, max_height)
	var offset = Vector3(sin(orbit_rad) * distance, height, cos(orbit_rad) * distance)
	if not is_inside_tree():
		return
	global_position = scene_center + offset
	look_at(scene_center, Vector3.UP)


# -------------------------------
# 🎬 컷 시퀀스
# -------------------------------
func _update_cut_sequence(delta):
	_cut_sequence_timer += delta
	if _cut_sequence_timer >= cut_sequence_interval:
		_cut_sequence_timer = 0.0
		_cut_sequence_index = (_cut_sequence_index + 1) % cut_sequence_angles.size()
		tilt_angle_deg = cut_sequence_angles[_cut_sequence_index]
		_recalculate_camera_transform()


# -------------------------------
# 🧾 로그 출력
# -------------------------------
func _log_scene_info(aabb: AABB):
	print("[SmartCineCamera] Scene Info")
	print("  • Center:", aabb.position + aabb.size * 0.5)
	print("  • Size:", aabb.size)
	print("  • Field:", field_node_path)
	print("  • Players:", players_group_path)
	print("  • Ball:", ball_node_path)
	print("  • Orbit:", enable_orbit, " Speed:", orbit_speed_deg_per_sec)
	print("  • Tracking:", enable_tracking, " Smoothness:", tracking_smoothness)
