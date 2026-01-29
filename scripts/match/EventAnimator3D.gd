extends Node3D
## OpenFootball 이벤트 애니메이터 (3D Isometric)
## Timeline 이벤트를 3D 아이소메트릭 뷰로 표현

class_name EventAnimator3D

@export var camera: Camera3D
@export var event_label: Label
@export var player_marker: Node3D  # 공 마커 (RigidBody3D)

# 캐릭터 풀 시스템
var player_pool: Array[Node3D] = []
const MAX_PLAYERS = 22
# LIA Pipeline Characters (RigAnything skeleton: Bone_0 ~ Bone_33)
const HOME_CHARACTER_MODEL := "res://assets/soccer_players/characters/main_characters/captain_rigged.glb"
const AWAY_CHARACTER_MODEL := "res://assets/soccer_players/characters/main_characters/shadow_rigged.glb"
const TEAM_CHARACTER_MODELS := {0: HOME_CHARACTER_MODEL, 1: AWAY_CHARACTER_MODEL}

# Character pool for variety (optional, for future use)
const CHARACTER_POOL_HOME := [
	"res://assets/soccer_players/characters/main_characters/captain_rigged.glb",
	"res://assets/soccer_players/characters/main_characters/ace_rigged.glb",
	"res://assets/soccer_players/characters/main_characters/wall_rigged.glb",
	"res://assets/soccer_players/characters/main_characters/princess_rigged.glb",
]
const CHARACTER_POOL_AWAY := [
	"res://assets/soccer_players/characters/main_characters/shadow_rigged.glb",
	"res://assets/soccer_players/characters/main_characters/fairy_f_rigged.glb",
	"res://assets/soccer_players/characters/main_characters/genki_f_rigged.glb",
	"res://assets/soccer_players/characters/main_characters/tomboy_rigged.glb",
]
var animation_cache: Dictionary = {}  # player -> AnimationPlayer

# ✅ NEW: Roster 데이터 (선수 ID → 선수 정보)
var player_roster: Dictionary = {}  # { team_id: { player_index: {name, position, ca, ...} } }
var player_index_by_id: Dictionary = {0: {}, 1: {}}  # Maps string id/name -> index per team

# ✅ NEW: PlaybackController 연결
var playback_controller: PlaybackController = null

# 애니메이션 상태
var current_events: Array = []
var current_event_index: int = 0
var is_playing: bool = false
var animation_speed: float = 1.0  # 재생 속도 배율

## P3.2: Seeded RNG for deterministic cosmetic animations
var cosmetic_rng: RandomNumberGenerator = null
var timeline_seed: int = 0

# 카메라 트래킹 시스템
var camera_tracking_enabled: bool = true
var camera_tracking_target: Node3D = null  # 추적 대상
var camera_tracking_targets: Array[Node3D] = []  # 다중 타겟
var camera_default_offset: Vector3 = Vector3(15, 25, 15)  # 기본 오프셋
var camera_tracking_speed: float = 3.0  # 추적 속도 (낮을수록 부드러움)

# 이벤트별 애니메이션 시간 (초) - 천천히!
const EVENT_DURATION = {"goal": 5.0, "shot": 3.0, "pass": 1.0, "kickoff": 2.0, "full_time": 3.0}  # 골: 5초간 보여주기  # 슛: 3초  # 패스: 1초  # 킥오프: 2초  # 종료: 3초

const PLAYERS_PER_TEAM: int = 11

# 카메라 줌 레벨 (orthogonal size) - 2.4배 스케일 필드에 맞춤
const SIZE_DEFAULT: float = 200.0
const SIZE_GOAL: float = 30.0
const SIZE_SHOT: float = 50.0

# FIFA 규격
const FieldDimensions = preload("res://scripts/core/FieldDimensions.gd")

const OF_FIELD_LENGTH: float = FieldDimensions.REAL_LENGTH  # OpenFootball x (105m)
const OF_FIELD_WIDTH: float = FieldDimensions.REAL_WIDTH  # OpenFootball y (68m)
const RUN_TRAIL_COLOR := Color(0.25, 0.9, 0.65, 0.85)
const DRIBBLE_TRAIL_COLOR := Color(1.0, 0.55, 0.35, 0.8)
const THROUGH_BALL_TRAIL_COLOR := Color(0.4, 0.65, 1.0, 0.85)
const PASS_TRAIL_COLOR := Color(0.95, 0.85, 0.3, 0.85)
const SHOT_TRAIL_COLOR := Color(1.0, 0.45, 0.2, 0.9)

# 필드 좌표 보정 (코너 노드 기반으로 자동 측정)
var field_min_x: float = 134.5
var field_min_z: float = 39.5
var field_size_x: float = 68.0
var field_size_z: float = 105.0

# 애니메이션 매핑 테이블
# Animation mapping for LIA characters
# NOTE: Animations are loaded from res://assets/soccer_players/animations_retargeted/
# RigAnything skeleton (Bone_0~Bone_33) uses retargeted Mixamo animations
const ANIMATION_MAP = {
	"Shot": {"shooter": ["soccer_kick"], "duration": 1.0},
	"Goal": {"scorer": ["soccer_kick", "running"], "duration": 3.0},
	"Pass": {"passer": ["soccer_pass"], "duration": 0.8},
	"Tackle": {"tackler": ["running"], "victim": ["offensive_idle"], "duration": 1.0},
	"Header": {"player": ["header_soccerball"], "duration": 0.8}
}

# Retargeted animation library paths (Mixamo -> RigAnything)
const ANIM_LIBRARY_PATH := "res://assets/soccer_players/animations_retargeted/"
const ANIM_LIBRARY_RESOURCE := "res://assets/soccer_players/lia_animations.tres"

# Shared AnimationLibrary for all LIA characters
var shared_animation_library: AnimationLibrary = null

const AVAILABLE_ANIMATIONS := {
	"idle": "lia_offensive_idle.fbx",
	"soccer_idle": "lia_soccer_idle.fbx",
	"running": "lia_running.fbx",
	"soccer_running": "lia_soccer_running.fbx",
	"jog_left": "lia_jog_strafe_left.fbx",
	"jog_right": "lia_jog_strafe_right.fbx",
	"soccer_kick": "lia_soccer_kick.fbx",
	"kick_retargeted": "lia_kick_retargeted.fbx",
	"soccer_pass": "lia_soccer_pass.fbx",
	"header_soccerball": "lia_header_soccerball.fbx",
	"receive_soccerball": "lia_receive_soccerball.fbx",
	"goalkeeper_idle": "lia_soccer_goalkeeper_idle.fbx",
	"goalkeeper_catch": "lia_soccer_goalkeeper_catch.fbx",
}

# 발 선호도 (나중에 선수 데이터와 연동)
enum Foot { LEFT, RIGHT, BOTH }
var player_preferences: Dictionary = {}  # player_name -> Foot
var _transient_nodes: Array[Node] = []

signal event_started(event: Dictionary)
signal event_finished(event: Dictionary)
signal all_events_finished


func _ready() -> void:
	# export 변수가 null이면 자동으로 자식 노드 찾기
	if not camera:
		camera = get_node_or_null("Camera3D")
	if not event_label:
		event_label = get_node_or_null("UI/EventLabel")
	if not player_marker:
		player_marker = get_node_or_null("PlayerMarker")

	print("🔧 EventAnimator3D _ready()")
	print("   camera: ", camera)
	print("   event_label: ", event_label)
	print("   player_marker: ", player_marker)

	# FBX 필드 모델의 실제 transform 및 크기 확인
	var field = get_node_or_null("FootballPitch")
	if field:
		# 불필요한 오브젝트 숨기기 (훈련 도구, 공 등)
		_hide_unnecessary_objects(field)
		print("   ⚽ FootballPitch found!")
		print("      position: ", field.position)
		print("      scale: ", field.scale)
		print("      rotation_degrees: ", field.rotation_degrees)

		# 실제 필드 범위 계산 (모든 MeshInstance3D의 전역 AABB 합산)
		var result = _calculate_total_aabb(field)

		if result.has("aabb"):
			var total_aabb = result.aabb
			print("   📐 Total Field AABB:")
			print("      position: ", total_aabb.position)
			print("      size: ", total_aabb.size)
			print("      center: ", total_aabb.get_center())

			# OpenFootball 좌표와 비교
			print("   🔍 Scale ratio:")
			print("      OpenFootball: 105m x 68m")
			print("      FBX X scale: %.3f (should be ~1.0 for 105)" % (total_aabb.size.x / 105.0))
			print("      FBX Z scale: %.3f (should be ~1.0 for 68)" % (total_aabb.size.z / 68.0))
		else:
			print("   ⚠️ No mesh instances found in field!")

		# 코너 플래그 위치 찾기
		print("   🚩 Corner Flags (Cones):")
		_find_corner_flags(field)

		# Line 메쉬 찾기 (경기장 흰 선)
		print("   📏 경기장 라인 찾기:")
		_find_field_lines(field)

		# 전체 구조 출력
		print("   🔍 FBX 전체 구조:")
		_print_children_recursive(field, 0)
	else:
		print("   ❌ FootballPitch not found!")

	_calibrate_field_bounds(field)

	# ✅ FIXED: Initialize camera and player pool ONCE after all setup
	if camera:
		camera.size = SIZE_DEFAULT
	else:
		push_warning("⚠️ Camera not found - animations will be limited")

	if player_marker:
		player_marker.visible = false

		# ✅ FIX: Scale ONLY the visual model (FootballModel), NOT the CollisionShape
		# CollisionShape radius 0.11m is already correct for 0.22m ball
		# Ball needs to be MUCH smaller (0.22m diameter = real soccer ball)
		var football_model = player_marker.get_node_or_null("FootballModel")
		if football_model:
			# Scale down the visual model to match CollisionShape size (22cm diameter)
			# Reduced from 0.022 to 0.015 (ball was still too big compared to goal)
			football_model.scale = Vector3(0.015, 0.015, 0.015)
			print("   ⚽ FootballModel scaled to: ", football_model.scale)
		else:
			push_warning("⚠️ FootballModel not found in PlayerMarker")

		print("   ℹ️ PlayerMarker = 슛/골 발생 위치 (공의 위치)")
	else:
		push_warning("⚠️ PlayerMarker not found - player positions won't be shown")

	if not event_label:
		push_warning("⚠️ EventLabel not found - event info won't be displayed")

	# 캐릭터 풀 초기화 (22명 선수 미리 생성)
	_initialize_player_pool()

	# ✅ NEW: PlaybackController 연결
	_setup_playback_controller()


## P3.2: Initialize timeline with seed for deterministic visuals
func initialize_timeline(rng_seed: int) -> void:
	timeline_seed = rng_seed
	cosmetic_rng = RandomNumberGenerator.new()
	cosmetic_rng.seed = rng_seed
	print("[EventAnimator3D] Seeded cosmetic RNG: %d" % rng_seed)


func _find_corner_flags(node: Node) -> void:
	var cones = []
	_find_cones_recursive(node, cones)

	print("   Total Cones found: %d" % cones.size())
	for cone in cones:
		var pos = cone.global_position
		print("      %s: (%.2f, %.2f, %.2f)" % [cone.name, pos.x, pos.y, pos.z])

	# 경기장 실제 코너 4개 찾기 (X, Z 극값 조합)
	if cones.size() >= 4:
		var _corners = []  # unused
		var min_x = 999999.0
		var max_x = -999999.0
		var min_z = 999999.0
		var max_z = -999999.0

		for cone in cones:
			var pos = cone.global_position
			min_x = min(min_x, pos.x)
			max_x = max(max_x, pos.x)
			min_z = min(min_z, pos.z)
			max_z = max(max_z, pos.z)

		print("   🎯 코너 범위: X[%.2f ~ %.2f], Z[%.2f ~ %.2f]" % [min_x, max_x, min_z, max_z])
		print("   📍 경기장 4개 코너 (추정):")
		print("      좌하: (%.2f, 1.63, %.2f)" % [min_x, min_z])
		print("      우하: (%.2f, 1.63, %.2f)" % [max_x, min_z])
		print("      좌상: (%.2f, 1.63, %.2f)" % [min_x, max_z])
		print("      우상: (%.2f, 1.63, %.2f)" % [max_x, max_z])


func _find_cones_recursive(node: Node, cones: Array) -> void:
	for child in node.get_children():
		if "Cone" in child.name:
			cones.append(child)
		if child.get_child_count() > 0:
			_find_cones_recursive(child, cones)


func _find_field_lines(node: Node) -> void:
	var lines = []
	_find_lines_recursive(node, lines)

	print("   Total Lines found: %d" % lines.size())
	for line in lines:
		if line is MeshInstance3D:
			var aabb = line.get_aabb()
			var line_transform: Transform3D = line.global_transform
			var global_aabb = line_transform * aabb
			var _pos = global_aabb.position
			var size = global_aabb.size
			var center = global_aabb.get_center()
			print(
				(
					"      %s: center=(%.2f, %.2f, %.2f) size=(%.2f, %.2f, %.2f)"
					% [line.name, center.x, center.y, center.z, size.x, size.y, size.z]
				)
			)


func _find_lines_recursive(node: Node, lines: Array) -> void:
	for child in node.get_children():
		if "Line" in child.name or "Boundary" in child.name or "Border" in child.name:
			lines.append(child)
		if child.get_child_count() > 0:
			_find_lines_recursive(child, lines)


func _calculate_total_aabb(node: Node) -> Dictionary:
	var result: Dictionary = {}
	var total_aabb: AABB
	var found_mesh: bool = false

	for child in node.get_children():
		if child is MeshInstance3D:
			var mesh_aabb: AABB = child.get_aabb()
			var child_transform: Transform3D = child.global_transform
			var global_aabb: AABB = child_transform * mesh_aabb

			if not found_mesh:
				total_aabb = global_aabb
				found_mesh = true
			else:
				total_aabb = total_aabb.merge(global_aabb)

		# 재귀적으로 자식 노드를 검사
		if child.get_child_count() > 0:
			var child_result: Dictionary = _calculate_total_aabb(child)
			if child_result.has("aabb"):
				if not found_mesh:
					total_aabb = child_result.aabb
					found_mesh = true
				else:
					total_aabb = total_aabb.merge(child_result.aabb)

	if found_mesh:
		result.aabb = total_aabb

	return result


func _hide_unnecessary_objects(field: Node) -> void:
	# 불필요한 오브젝트 이름 패턴 (훈련 도구, 공, Icosphere 등)
	var hide_patterns = ["Icosphere", "Sphere", "Ball", "Cube_02", "Cube_03", "Cylinder_00"]

	_hide_recursive(field, hide_patterns)
	print("   🧹 Unnecessary objects hidden")


func _hide_recursive(node: Node, patterns: Array) -> void:
	for child in node.get_children():
		for pattern in patterns:
			if pattern in child.name:
				if "visible" in child:
					child.visible = false
				if child is CollisionObject3D:
					child.collision_layer = 0
					child.collision_mask = 0
				for grandchild in child.get_children():
					if grandchild is CollisionObject3D:
						grandchild.collision_layer = 0
						grandchild.collision_mask = 0
				print("      Hidden: %s" % child.name)
				break

		if child.get_child_count() > 0:
			_hide_recursive(child, patterns)


func _print_children_recursive(node: Node, depth: int) -> void:
	var indent = "      " + "  ".repeat(depth)
	for child in node.get_children():
		var info = "%s- %s (%s)" % [indent, child.name, child.get_class()]

		# MeshInstance3D나 CSGShape3D면 크기 정보 출력
		if child is MeshInstance3D:
			var aabb = child.get_aabb()
			info += " size=(%.1f, %.1f, %.1f)" % [aabb.size.x, aabb.size.y, aabb.size.z]
		elif child is Node3D:
			info += " pos=%s" % str(child.position)

		print(info)

		if depth < 3:  # 3단계까지만
			_print_children_recursive(child, depth + 1)


func _process(delta: float) -> void:
	if camera_tracking_enabled and camera:
		_update_camera_tracking(delta)


## ✅ NEW: PlaybackController 설정
func _setup_playback_controller() -> void:
	# PlaybackController 인스턴스 생성 및 추가
	playback_controller = PlaybackController.new()
	add_child(playback_controller)

	# 시그널 연결
	playback_controller.time_scale_changed.connect(_on_time_scale_changed)
	playback_controller.pause_state_changed.connect(_on_pause_state_changed)
	playback_controller.skip_requested.connect(_on_skip_requested)
	playback_controller.restart_requested.connect(_on_restart_requested)

	print("🎮 PlaybackController connected")


## ✅ NEW: 재생 속도 변경 핸들러
func _on_time_scale_changed(new_scale: float) -> void:
	animation_speed = new_scale
	print("⏩ Animation speed updated: %.1fx" % animation_speed)


## ✅ NEW: 일시정지 상태 변경 핸들러
func _on_pause_state_changed(paused: bool) -> void:
	# Pause logic handled in event loop
	if paused:
		print("⏸️ EventAnimator3D paused")
	else:
		print("▶️ EventAnimator3D resumed")


## ✅ NEW: 이벤트 스킵 핸들러
func _on_skip_requested() -> void:
	if is_playing and current_event_index < current_events.size():
		print("⏭️ Skipping event %d" % (current_event_index + 1))
		# Skip은 현재 이벤트를 즉시 완료하고 다음으로 이동
		# 여기서는 플래그만 설정하고, 각 애니메이션에서 체크하도록 함


## ✅ NEW: 재시작 핸들러
func _on_restart_requested() -> void:
	print("🔄 Restarting timeline from beginning")
	stop()
	if not current_events.is_empty():
		play_events(current_events, {})


## 카메라 자동 추적 업데이트
func _update_camera_tracking(delta: float) -> void:
	if camera_tracking_targets.size() > 0:
		# 다중 타겟: 모든 타겟의 중심점 + AABB 계산
		var center = Vector3.ZERO
		var min_bounds = Vector3(999999, 999999, 999999)
		var max_bounds = Vector3(-999999, -999999, -999999)

		for target in camera_tracking_targets:
			if target and target.visible:
				var pos = target.global_position
				center += pos
				min_bounds.x = min(min_bounds.x, pos.x)
				min_bounds.y = min(min_bounds.y, pos.y)
				min_bounds.z = min(min_bounds.z, pos.z)
				max_bounds.x = max(max_bounds.x, pos.x)
				max_bounds.y = max(max_bounds.y, pos.y)
				max_bounds.z = max(max_bounds.z, pos.z)

		if camera_tracking_targets.size() > 0:
			center /= camera_tracking_targets.size()

			# AABB 크기 기반으로 줌 레벨 계산
			var bounds_size = max_bounds - min_bounds
			var max_dimension = max(bounds_size.x, bounds_size.z)
			var target_size = clamp(max_dimension * 1.5, 40.0, 120.0)

			# 카메라 위치: 중심점 + 오프셋
			var target_pos = center + camera_default_offset

			# 부드러운 이동 (lerp)
			camera.position = camera.position.lerp(target_pos, delta * camera_tracking_speed)
			camera.size = lerp(camera.size, target_size, delta * camera_tracking_speed)

	elif camera_tracking_target and camera_tracking_target.visible:
		# 단일 타겟: 타겟 위치 + 오프셋
		var target_pos = camera_tracking_target.global_position + camera_default_offset
		camera.position = camera.position.lerp(target_pos, delta * camera_tracking_speed)


## 카메라 트래킹 시작 (단일 타겟)
func start_camera_tracking(target: Node3D, zoom_size: float = 50.0, offset: Vector3 = Vector3(15, 25, 15)) -> void:
	camera_tracking_target = target
	camera_tracking_targets.clear()
	camera_default_offset = offset
	camera_tracking_enabled = true

	# 즉시 줌 설정
	if camera:
		var tween = create_tween()
		tween.tween_property(camera, "size", zoom_size, 0.3).set_trans(Tween.TRANS_QUAD)


## 카메라 트래킹 시작 (다중 타겟)
func start_camera_tracking_multi(targets: Array, offset: Vector3 = Vector3(15, 25, 15)) -> void:
	camera_tracking_target = null
	camera_tracking_targets.clear()
	for target in targets:
		if target is Node3D:
			camera_tracking_targets.append(target)
	camera_default_offset = offset
	camera_tracking_enabled = true


## 카메라 트래킹 중지
func stop_camera_tracking() -> void:
	camera_tracking_enabled = false
	camera_tracking_target = null
	camera_tracking_targets.clear()


## 캐릭터 풀 초기화 (22명 선수 미리 생성)
func _initialize_player_pool() -> void:
	print("🎭 Initializing player pool...")

	player_pool.clear()
	animation_cache.clear()

	# Load shared AnimationLibrary
	_load_animation_library()

	for team_id in range(2):
		var model_path: String = String(TEAM_CHARACTER_MODELS.get(team_id, HOME_CHARACTER_MODEL))
		var packed_scene: PackedScene = load(model_path) as PackedScene
		if packed_scene == null:
			push_error("❌ Failed to load character model: %s" % model_path)
			continue

		for slot in range(PLAYERS_PER_TEAM):
			var player: Node3D = packed_scene.instantiate() as Node3D
			if player == null:
				continue
			player.name = "Player_%d_%d" % [team_id, slot]
			player.visible = false
			player.scale = Vector3(4.2, 4.2, 4.2)
			add_child(player)
			player.global_position = Vector3(0, -0.85, -100)

			# Setup AnimationPlayer with AnimationLibrary for GLB characters
			_setup_player_animation(player)

			player_pool.append(player)

	print("   ✅ Created %d players in pool (scaled to ~1.7m)" % player_pool.size())


## Load the shared AnimationLibrary for LIA characters
func _load_animation_library() -> void:
	if shared_animation_library != null:
		return  # Already loaded

	if ResourceLoader.exists(ANIM_LIBRARY_RESOURCE):
		shared_animation_library = load(ANIM_LIBRARY_RESOURCE) as AnimationLibrary
		if shared_animation_library:
			print("   📚 Loaded AnimationLibrary: %s" % ANIM_LIBRARY_RESOURCE)
			print("      Animations: %s" % str(shared_animation_library.get_animation_list()))
		else:
			push_warning("   ⚠️ Failed to load AnimationLibrary: %s" % ANIM_LIBRARY_RESOURCE)
	else:
		push_warning("   ⚠️ AnimationLibrary not found: %s" % ANIM_LIBRARY_RESOURCE)
		push_warning("      Run tools/generate_animation_library.gd in Godot Editor to create it.")


## Setup AnimationPlayer for a GLB character
func _setup_player_animation(player: Node3D) -> void:
	# Check if AnimationPlayer already exists
	var existing_anim_player := _find_animation_player(player)
	if existing_anim_player and shared_animation_library:
		# Add AnimationLibrary to existing player
		if not existing_anim_player.has_animation_library("lia"):
			existing_anim_player.add_animation_library("lia", shared_animation_library)
		animation_cache[player] = existing_anim_player
		return

	# Find the Skeleton3D to bind animations
	var skeleton := _find_skeleton(player)
	if skeleton == null:
		# No skeleton means no animation support
		return

	# Create new AnimationPlayer
	var anim_player := AnimationPlayer.new()
	anim_player.name = "AnimationPlayer"
	player.add_child(anim_player)

	# Set the root node for animation paths
	anim_player.root_node = anim_player.get_path_to(player)

	# Add shared AnimationLibrary
	if shared_animation_library:
		anim_player.add_animation_library("lia", shared_animation_library)
		print("      🎬 AnimationPlayer added to %s with lia library" % player.name)

	animation_cache[player] = anim_player


## Find Skeleton3D in a node tree
func _find_skeleton(node: Node) -> Skeleton3D:
	if node is Skeleton3D:
		return node
	for child in node.get_children():
		var found := _find_skeleton(child)
		if found:
			return found
	return null


## 풀에서 선수 가져오기
func get_player_from_pool(index: int) -> Node3D:
	if index < 0 or index >= player_pool.size():
		push_warning("⚠️ Invalid player index: %d" % index)
		return null
	return player_pool[index]


func _get_team_pool_index(team_id: int, slot: int) -> int:
	var clamped_team: int = max(0, min(team_id, 1))
	var normalized_slot: int = slot
	if normalized_slot < 0:
		normalized_slot = 0
	if PLAYERS_PER_TEAM > 0:
		normalized_slot = normalized_slot % PLAYERS_PER_TEAM
	return clamped_team * PLAYERS_PER_TEAM + normalized_slot


func get_team_player(team_id: int, player_index: int, fallback_slot: int = 0) -> Node3D:
	var slot := player_index
	if slot < 0:
		slot = fallback_slot
	if PLAYERS_PER_TEAM > 0:
		slot = slot % PLAYERS_PER_TEAM
	return get_player_from_pool(_get_team_pool_index(team_id, slot))


func _hide_team_players(team_id: int) -> void:
	for slot in range(PLAYERS_PER_TEAM):
		var player := get_player_from_pool(_get_team_pool_index(team_id, slot))
		if player:
			player.visible = false


## AnimationPlayer 가져오기 (캐싱)
func get_animation_player(player: Node3D) -> AnimationPlayer:
	if animation_cache.has(player):
		return animation_cache[player]

	var anim_player = _find_animation_player(player)
	if anim_player:
		animation_cache[player] = anim_player
	return anim_player


func _find_animation_player(node: Node) -> AnimationPlayer:
	if node is AnimationPlayer:
		return node
	for child in node.get_children():
		var found = _find_animation_player(child)
		if found:
			return found
	return null


## 애니메이션 재생 헬퍼
func play_animation(player: Node3D, anim_name: String) -> void:
	var anim_player = get_animation_player(player)
	if not anim_player:
		push_warning("⚠️ No AnimationPlayer found for %s" % player.name)
		return

	# Try with AnimationLibrary prefix first (lia/anim_name)
	var lia_anim_name := "lia/%s" % anim_name
	if anim_player.has_animation(lia_anim_name):
		anim_player.play(lia_anim_name)
		print("   ▶️ Playing animation: %s on %s" % [lia_anim_name, player.name])
	elif anim_player.has_animation(anim_name):
		# Fallback to direct animation name (for backwards compatibility)
		anim_player.play(anim_name)
		print("   ▶️ Playing animation: %s on %s" % [anim_name, player.name])
	else:
		# List available animations for debugging
		var available := anim_player.get_animation_list()
		push_warning("⚠️ Animation not found: %s (tried lia/%s). Available: %s" % [anim_name, anim_name, str(available)])


## 발 선호도에 따른 킥 애니메이션 선택
func get_kick_animation(player_name: String = "") -> String:
	# LIA characters use unified soccer_kick animation (retargeted from Mixamo)
	# TODO: Add left/right foot variants when available
	return "soccer_kick"


## ✅ NEW: Roster 데이터 로드
func load_roster(rosters: Dictionary) -> void:
	player_roster.clear()
	player_index_by_id = {0: {}, 1: {}}

	if rosters.has("home") and rosters.home.has("players"):
		player_roster[0] = {}  # Home team = team_id 0
		for i in range(rosters.home.players.size()):
			var player = rosters.home.players[i]
			player_roster[0][i] = player
			# Build id -> index mapping (player_id, uid, name)
			if typeof(player) == TYPE_DICTIONARY:
				var pd: Dictionary = player
				if pd.has("player_id"):
					player_index_by_id[0][str(pd.get("player_id"))] = i
				if pd.has("uid"):
					player_index_by_id[0][str(pd.get("uid"))] = i
				if pd.has("name"):
					player_index_by_id[0][str(pd.get("name"))] = i
		print("📋 Loaded home roster: %d players" % rosters.home.players.size())

	if rosters.has("away") and rosters.away.has("players"):
		player_roster[1] = {}  # Away team = team_id 1
		for i in range(rosters.away.players.size()):
			var player = rosters.away.players[i]
			player_roster[1][i] = player
			# Build id -> index mapping (player_id, uid, name)
			if typeof(player) == TYPE_DICTIONARY:
				var pd: Dictionary = player
				if pd.has("player_id"):
					player_index_by_id[1][str(pd.get("player_id"))] = i
				if pd.has("uid"):
					player_index_by_id[1][str(pd.get("uid"))] = i
				if pd.has("name"):
					player_index_by_id[1][str(pd.get("name"))] = i
		print("📋 Loaded away roster: %d players" % rosters.away.players.size())


## ✅ NEW: Player ID로 이름 찾기
func get_player_name(team_id: int, player_index: int) -> String:
	if player_roster.has(team_id) and player_roster[team_id].has(player_index):
		return player_roster[team_id][player_index].name
	return "알 수 없음 #%d" % player_index


func get_player_index(team_id: int, id_value: Variant) -> int:
	if typeof(id_value) == TYPE_INT:
		var idx := int(id_value)
		if idx >= 0 and player_roster.has(team_id) and player_roster[team_id].has(idx):
			return idx
		return idx
	if typeof(id_value) == TYPE_STRING:
		var key := String(id_value)
		if player_index_by_id.has(team_id) and player_index_by_id[team_id].has(key):
			return int(player_index_by_id[team_id][key])
		if key.is_valid_int():
			var idx2 := int(key)
			if player_roster.has(team_id) and player_roster[team_id].has(idx2):
				return idx2
	return -1


func get_player_name_from_id(team_id: int, id_value: Variant) -> String:
	var idx := get_player_index(team_id, id_value)
	if idx >= 0:
		return get_player_name(team_id, idx)
	if typeof(id_value) == TYPE_STRING and String(id_value) != "":
		return String(id_value)
	return "알 수 없음"


func _event_time_seconds(event: Dictionary) -> float:
	return _base_time_seconds(event.get("base", {}))


func _base_time_seconds(base: Variant) -> float:
	if typeof(base) != TYPE_DICTIONARY:
		return 0.0
	var base_dict: Dictionary = base
	if base_dict.has("t") and base_dict["t"] is float or base_dict["t"] is int:
		return float(base_dict["t"])
	if base_dict.has("minute"):
		return float(base_dict.get("minute", 0.0)) * 60.0
	return 0.0


func _minute_from_event(event: Dictionary) -> int:
	return int(floor(_event_time_seconds(event) / 60.0))


func _minute_from_base(base: Dictionary) -> int:
	return int(floor(_base_time_seconds(base) / 60.0))


func _format_time_label(seconds: float) -> String:
	var total_seconds := int(max(seconds, 0.0))
	var minutes := total_seconds / 60
	var secs := total_seconds % 60
	return "%02d:%02d" % [minutes, secs]


func _time_label_from_event(event: Dictionary) -> String:
	return _format_time_label(_event_time_seconds(event))


func _time_label_from_base(base: Dictionary) -> String:
	return _format_time_label(_base_time_seconds(base))


func _resolve_event_player_name(team_id: int, id_value: Variant, event: Dictionary) -> String:
	var resolved := get_player_name_from_id(team_id, id_value)
	if not resolved.begins_with("알 수 없음"):
		return resolved
	if event.has("player_name"):
		resolved = String(event.get("player_name"))
	elif event.has("details") and event.details is Dictionary:
		var details: Dictionary = event.details
		for key in ["player_name", "name", "shooter_name", "passer_name", "tackler_name"]:
			if details.has(key):
				resolved = String(details.get(key))
				break
	elif event.has("base") and event.base is Dictionary:
		var base: Dictionary = event.base
		if base.has("player_name"):
			resolved = String(base.get("player_name"))
	if resolved == "" or resolved.begins_with("알 수 없음"):
		resolved = "알 수 없음"
	return resolved


## Normalize event structure to handle both nested and flattened formats
## Rust serializes with #[serde(tag = "etype", flatten)] producing:
##   {"etype": "pass", "t": 75.0, "player_id": "H5", ...}
## This function converts to the nested format expected by EventAnimator3D:
##   {"kind": "pass", "base": {"t": 75.0, "player_id": "H5", ...}, ...}
func _normalize_event_structure(event: Dictionary) -> Dictionary:
	# If already in nested format (has "kind" and "base"), return as-is
	if event.has("kind") and event.has("base"):
		return event

	# If in flattened format (has "etype"), convert to nested
	if event.has("etype"):
		var normalized: Dictionary = {"kind": event.get("etype"), "base": {}}

		# Base fields that should be nested
		var base_fields: Array = ["t", "minute", "team", "player_id", "pos"]
		for field in base_fields:
			if event.has(field):
				normalized["base"][field] = event.get(field)

		# Copy all other fields to top level (event-specific fields)
		for key in event.keys():
			if key != "etype" and key not in base_fields:
				normalized[key] = event.get(key)

		return normalized

	# If neither format is detected, return as-is
	return event


## Timeline 이벤트 로드 및 재생 시작
func play_events(events: Array, rosters: Dictionary = {}) -> void:
	if events.is_empty():
		push_warning("⚠️ EventAnimator3D: No events to play")
		return

	# ✅ NEW: Roster 데이터 먼저 로드
	if not rosters.is_empty():
		load_roster(rosters)

	# ✅ Issue #3: Test coordinate conversion
	_test_coordinate_conversion()

	current_events = events
	current_event_index = 0
	is_playing = true

	print("🎬 EventAnimator3D: Playing %d events" % events.size())
	_play_next_event()


## 재생 중지
func stop() -> void:
	is_playing = false
	current_event_index = 0
	_reset_camera()
	if player_marker:
		player_marker.visible = false


## 다음 이벤트 재생 (각 이벤트를 await로 순차 실행)
func _play_next_event() -> void:
	# ✅ NEW: 일시정지 상태 체크 (Signal-based wait to avoid busy loop)
	if playback_controller and playback_controller.is_paused:
		print("⏸️ EventAnimator3D: Paused, waiting for resume...")
		while playback_controller.is_paused and is_playing:
			await playback_controller.pause_state_changed
		print("▶️ EventAnimator3D: Resumed")

	if not is_playing or current_event_index >= current_events.size():
		is_playing = false
		all_events_finished.emit()
		print("✅ EventAnimator3D: All events finished")
		return

	var event = current_events[current_event_index]

	# ✅ Normalize event structure (support both nested and flattened formats)
	var normalized_event = _normalize_event_structure(event)

	# ✅ DEBUG: Print event structure before processing
	print("\n=== 🔍 Event %d/%d Debug ===" % [current_event_index + 1, current_events.size()])
	print("Event keys: %s" % str(normalized_event.keys()))
	print("Event.kind: %s" % normalized_event.get("kind", "unknown"))
	if normalized_event.has("base"):
		print("Event.base: %s" % JSON.stringify(normalized_event.get("base"), "  "))
	print("=== End Event Debug ===\n")

	event = normalized_event

	if player_marker and player_marker is RigidBody3D:
		player_marker.freeze = false
	event_started.emit(event)

	# ✅ FULL WRAPPING: Access event.kind instead of event.type
	var event_kind = event.get("kind", "unknown")
	var event_seconds := _event_time_seconds(event)
	var time_label := _format_time_label(event_seconds)
	print(
		(
			"🎬 Playing event %d/%d: %s at %s (%.1fs)"
			% [current_event_index + 1, current_events.size(), event_kind, time_label, event_seconds]
		)
	)

	# 이벤트 타입별 애니메이션 (AWAIT로 순차 실행!)
	match event_kind:
		"goal":
			await _animate_goal(event)
		"shot", "shot_on_target", "shot_off_target":
			await _animate_shot(event)
		"pass":
			await _animate_pass(event)
		"run":
			await _animate_run(event)
		"dribble":
			await _animate_dribble(event)
		"through_ball":
			await _animate_through_ball(event)
		"tackle":
			await _animate_tackle(event)
		"header":
			await _animate_header(event)
		"ball_move":
			await _animate_ball_move(event)
		"kickoff", "kick_off":
			await _animate_kickoff(event)
		"half_time":
			await _animate_half_time(event)
		"full_time":
			await _animate_full_time(event)
		"yellow_card":
			await _animate_yellow_card(event)
		"red_card":
			await _animate_red_card(event)
		"substitution":
			await _animate_substitution(event)
		"injury":
			await _animate_injury(event)
		_:
			# 기본 이벤트 표시
			await _animate_generic(event)


## OpenFootball 미터 좌표 → 3D 월드 좌표
## OpenFootball: (x, y) where x=0~105m(길이), y=0~68m(폭)
## 3D: X=폭(좁음), Z=길이(김) → X와 Z 반대 매핑!
func meter_to_3d(meter_pos: Vector2) -> Vector3:
	var effective_size_x: float = field_size_x if field_size_x != 0.0 else 68.0
	var effective_size_z: float = field_size_z if field_size_z != 0.0 else 105.0

	# OpenFootball 좌표 매핑: OF_x(길이) → 3D_Z(길이), OF_y(폭) → 3D_X(폭)
	var world_x: float = field_min_x + (meter_pos.y / OF_FIELD_WIDTH) * effective_size_x  # y → X (폭)
	var world_z: float = field_min_z + (meter_pos.x / OF_FIELD_LENGTH) * effective_size_z  # x → Z (길이)

	return Vector3(world_x, -0.5, world_z)  # y=-0.5는 필드 표면 높이


## ✅ Issue #3: Test coordinate conversion accuracy
func _test_coordinate_conversion() -> void:
	print("\n=== 🧪 Coordinate Conversion Test ===")
	print(
		(
			"Field bounds: min_x=%f, min_z=%f, size_x=%f, size_z=%f"
			% [field_min_x, field_min_z, field_size_x, field_size_z]
		)
	)
	print("Field constants: OF_FIELD_LENGTH=%f, OF_FIELD_WIDTH=%f" % [OF_FIELD_LENGTH, OF_FIELD_WIDTH])

	# Test 1: 필드 중앙 (OpenFootball 좌표계 중심)
	var center_of := Vector2(52.5, 34.0)  # OpenFootball 중앙 (105/2, 68/2)
	var center_3d := meter_to_3d(center_of)
	var expected_center_3d := Vector3(field_min_x + field_size_x / 2.0, -0.5, field_min_z + field_size_z / 2.0)
	print("Test 1 - Field Center:")
	print("  OF: %s → 3D: %s" % [center_of, center_3d])
	print("  Expected 3D center: %s" % expected_center_3d)
	print("  Distance from expected: %f" % center_3d.distance_to(expected_center_3d))

	# Test 2: 왼쪽 골라인 중앙 (x=0, y=34)
	var left_goal := Vector2(0, 34.0)
	var left_goal_3d := meter_to_3d(left_goal)
	print("Test 2 - Left Goal (x=0):")
	print("  OF: %s → 3D: %s" % [left_goal, left_goal_3d])

	# Test 3: 오른쪽 골라인 중앙 (x=105, y=34)
	var right_goal := Vector2(105, 34.0)
	var right_goal_3d := meter_to_3d(right_goal)
	print("Test 3 - Right Goal (x=105):")
	print("  OF: %s → 3D: %s" % [right_goal, right_goal_3d])

	# Test 4: 코너 (0, 0)
	var corner_1 := Vector2(0, 0)
	var corner_1_3d := meter_to_3d(corner_1)
	print("Test 4 - Corner (0,0):")
	print("  OF: %s → 3D: %s" % [corner_1, corner_1_3d])

	# Test 5: 반대쪽 코너 (105, 68)
	var corner_2 := Vector2(105, 68)
	var corner_2_3d := meter_to_3d(corner_2)
	print("Test 5 - Corner (105,68):")
	print("  OF: %s → 3D: %s" % [corner_2, corner_2_3d])

	# Test 6: 필드 대각선 길이 검증
	var diagonal_of := corner_1.distance_to(corner_2)
	var diagonal_3d := corner_1_3d.distance_to(corner_2_3d)
	print("Test 6 - Diagonal Distance:")
	print("  OF diagonal: %f meters" % diagonal_of)
	print("  3D diagonal: %f units" % diagonal_3d)
	print("  Ratio (should be close to 1.0): %f" % (diagonal_3d / diagonal_of if diagonal_of > 0 else 0))

	print("=== Coordinate Test Complete ===\n")


func _calibrate_field_bounds(field: Node3D = null) -> void:
	var corner_names: Array = [
		"OpenFootballCorner1", "OpenFootballCorner2", "OpenFootballCorner3", "OpenFootballCorner4"
	]
	var corner_positions: Array = []
	for corner_name in corner_names:
		var node := get_node_or_null(corner_name)
		if node is Node3D:
			corner_positions.append((node as Node3D).global_position)

	if corner_positions.size() >= 2:
		var min_x: float = corner_positions[0].x
		var max_x: float = corner_positions[0].x
		var min_z: float = corner_positions[0].z
		var max_z: float = corner_positions[0].z
		for pos in corner_positions:
			min_x = min(min_x, pos.x)
			max_x = max(max_x, pos.x)
			min_z = min(min_z, pos.z)
			max_z = max(max_z, pos.z)
		field_min_x = min_x
		field_min_z = min_z
		field_size_x = max_x - min_x
		field_size_z = max_z - min_z
		print(
			"[EventAnimator3D] Field bounds calibrated from corner markers:",
			field_min_x,
			field_min_z,
			field_size_x,
			field_size_z
		)
		return

	if field != null:
		var result: Dictionary = _calculate_total_aabb(field)
		if result.has("aabb"):
			var total_aabb: AABB = result.aabb
			var measured_x: float = total_aabb.size.x
			var measured_z: float = total_aabb.size.z
			if measured_x >= OF_FIELD_WIDTH:
				var margin_x: float = (measured_x - OF_FIELD_WIDTH) / 2.0
				field_min_x = total_aabb.position.x + margin_x
				field_size_x = OF_FIELD_WIDTH
			else:
				field_min_x = total_aabb.position.x
				field_size_x = measured_x
			if measured_z >= OF_FIELD_LENGTH:
				var margin_z: float = (measured_z - OF_FIELD_LENGTH) / 2.0
				field_min_z = total_aabb.position.z + margin_z
				field_size_z = OF_FIELD_LENGTH
			else:
				field_min_z = total_aabb.position.z
				field_size_z = measured_z
			print(
				"[EventAnimator3D] Field bounds calibrated from AABB:",
				field_min_x,
				field_min_z,
				field_size_x,
				field_size_z
			)


## 골 이벤트 애니메이션
func _animate_goal(event: Dictionary) -> void:
	# ✅ DEBUG: Print full event structure
	print("\n=== 🔍 GOAL Event Debug ===")
	print("Full event: %s" % JSON.stringify(event, "  "))
	print("Event keys: %s" % str(event.keys()))
	print("Has 'at': %s" % event.has("at"))
	print("Has 'position': %s" % event.has("position"))
	print("Has 'base': %s" % event.has("base"))
	print("=== End GOAL Event Debug ===\n")

	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)
	var player_index = int(player_id) if player_id != null else -1
	var player_name = _resolve_event_player_name(team_id, player_id, event)
	var scorer_node: Node3D = get_team_player(team_id, player_index, 0)
	var celebration_nodes: Array[Node3D] = []
	var world_pos := Vector3.ZERO
	var goal_world := Vector3.ZERO
	var distance := 0.0
	var has_position := false

	print("\n⚽ GOAL! %s - 선수: %s (Team: %d, ID: %s)" % [time_label, player_name, team_id, str(player_id)])

	if event.has("at") and event.at != null:
		var pos_dict = event.at
		var meter_pos = Vector2(pos_dict.x, pos_dict.y)
		world_pos = meter_to_3d(meter_pos)
		var goal_x = OF_FIELD_LENGTH if team_id == 0 else 0.0
		var goal_z = OF_FIELD_WIDTH / 2.0
		goal_world = meter_to_3d(Vector2(goal_x, goal_z))
		distance = meter_pos.distance_to(Vector2(goal_x, goal_z))
		has_position = true

		if scorer_node:
			scorer_node.global_position = world_pos
			scorer_node.visible = true
			scorer_node.look_at(goal_world, Vector3.UP)
			play_animation(scorer_node, get_kick_animation(player_name))
			await get_tree().create_timer(1.0).timeout
			play_animation(scorer_node, "jump")
			await get_tree().create_timer(0.8).timeout
			play_animation(scorer_node, "emote-yes")

		var num_teammates = 3
		for offset in range(1, num_teammates + 1):
			var teammate = get_team_player(team_id, player_index + offset, offset)
			if teammate:
				var angle = ((offset - 1) * TAU / num_teammates) + randf_range(-0.3, 0.3)
				var radius = randf_range(5.0, 8.0)
				var teammate_start = world_pos + Vector3(cos(angle) * radius, 0, sin(angle) * radius)
				teammate.global_position = teammate_start
				teammate.visible = true
				teammate.look_at(world_pos, Vector3.UP)
				play_animation(teammate, "sprint")
				celebration_nodes.append(teammate)

		if celebration_nodes.size() > 0:
			await get_tree().create_timer(1.0).timeout
			for i in range(celebration_nodes.size()):
				var node = celebration_nodes[i]
				if node:
					var celebrate_anim = "interact-right" if i % 2 == 0 else "jump"
					play_animation(node, celebrate_anim)

		var celebrators: Array = []
		if scorer_node:
			celebrators.append(scorer_node)
		celebrators += celebration_nodes
		if celebrators.size() > 0:
			start_camera_tracking_multi(celebrators, Vector3(15, 25, 15))

		if player_marker and player_marker is RigidBody3D:
			player_marker.freeze = false
			player_marker.global_position = world_pos
			player_marker.linear_velocity = Vector3.ZERO
			player_marker.angular_velocity = Vector3.ZERO
			player_marker.visible = true
			var direction = (goal_world - world_pos).normalized()
			var distance_3d = world_pos.distance_to(goal_world)
			var shot_power = min(distance_3d * 0.3, 15.0)
			var lift = clamp(distance_3d * 0.1, 3.0, 6.0)
			var impulse = Vector3(direction.x * shot_power, lift, direction.z * shot_power)
			player_marker.apply_central_impulse(impulse)
			await get_tree().create_timer(2.0).timeout
			var settle_direction := Vector3(direction.x, 0.0, direction.z).normalized()
			var settle_distance := 5.0
			var ball_final_pos := goal_world + (settle_direction * settle_distance)
			ball_final_pos.y = 0.0
			player_marker.global_position = ball_final_pos
			player_marker.linear_velocity = Vector3.ZERO
			player_marker.angular_velocity = Vector3.ZERO
			player_marker.freeze = true

	if event_label:
		if has_position:
			event_label.text = "⚽ GOAL! %s\n선수: %s\n거리: %.1fm" % [time_label, player_name, distance]
		else:
			event_label.text = "⚽ GOAL! %s\n선수: %s" % [time_label, player_name]

	await get_tree().create_timer(EVENT_DURATION["goal"] / animation_speed).timeout
	stop_camera_tracking()
	await _reset_camera()

	if scorer_node:
		scorer_node.visible = false
	for teammate in celebration_nodes:
		if teammate:
			teammate.visible = false

	print("   ✅ Goal animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 슛 이벤트 애니메이션
func _animate_shot(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)

	var player_index = int(player_id) if player_id != null else -1
	var player_name = _resolve_event_player_name(team_id, player_id, event)
	var on_target = event.get("on_target", false)
	var is_home = team_id == 0

	var default_from_x := randf_range(80.0, 95.0) if is_home else randf_range(10.0, 25.0)
	var default_from := Vector2(default_from_x, randf_range(20.0, 48.0))
	var meter_from := _extract_meter_vector(event.get("from"), default_from)
	var world_from := meter_to_3d(meter_from)
	var default_target_x := 105.0 if is_home else 0.0
	var default_target := Vector2(default_target_x, OF_FIELD_WIDTH * 0.5)
	var meter_target := _extract_meter_vector(event.get("target"), default_target)
	var world_target := meter_to_3d(meter_target)

	var distance_to_target := meter_from.distance_to(meter_target)
	var ball_speed := _calculate_shot_ball_speed(event, distance_to_target)
	var flight_duration: float = max(distance_to_target / max(ball_speed, 0.1), 0.45) / animation_speed

	print(
		(
			"🎯 Shot %s - %s (Player: %s, Dist: %.1fm, Speed: %.1fm/s)"
			% ["목표 안" if on_target else "목표 밖", time_label, player_name, distance_to_target, ball_speed]
		)
	)

	var shooter = get_team_player(team_id, player_index, 0)
	if shooter:
		shooter.global_position = world_from
		shooter.visible = true
		shooter.look_at(world_target, Vector3.UP)
		play_animation(shooter, get_kick_animation(player_name))
		start_camera_tracking(shooter, SIZE_SHOT, Vector3(15, 20, 15))

	_spawn_speed_trail(world_from, world_target, SHOT_TRAIL_COLOR, flight_duration + 0.4)
	_animate_ball_marker_path(world_from, world_target, flight_duration)
	await get_tree().create_timer(flight_duration).timeout

	var event_base_duration: Variant = EVENT_DURATION["shot"]
	var hold_duration: float = max(float(event_base_duration) / animation_speed - flight_duration, 0.25)
	if hold_duration > 0.0:
		await get_tree().create_timer(hold_duration).timeout

	stop_camera_tracking()
	await _reset_camera()

	var shooter_cleanup = get_team_player(team_id, player_index, 0)
	if shooter_cleanup:
		shooter_cleanup.visible = false

	if event_label:
		var info: Array[String] = []
		var xg_val := float(event.get("xg", event.get("xg_value", -1.0)))
		if xg_val >= 0.0:
			info.append("xG %.2f" % xg_val)
		info.append("속도 %.1fm/s" % ball_speed)
		event_label.text = (
			"🎯 슛 %s - %s\n%s\n위치: (%.0f, %.0f) → (%.0f, %.0f)"
			% [
				"목표 안" if on_target else "목표 밖",
				time_label,
				" · ".join(info),
				meter_from.x,
				meter_from.y,
				meter_target.x,
				meter_target.y
			]
		)

	print("   ✅ Shot animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 패스 이벤트 애니메이션
func _animate_pass(event: Dictionary) -> void:
	print("\n=== 📮 PASS Event Debug ===")
	print("Full event: %s" % JSON.stringify(event, "  "))
	print("Event keys: %s" % str(event.keys()))
	print("Has 'from': %s" % event.has("from"))
	print("Has 'to': %s" % event.has("to"))
	print("Has 'base': %s" % event.has("base"))
	print("=== End PASS Event Debug ===\n")

	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)

	var player_index = int(player_id) if player_id != null else -1
	var passer_name = _resolve_event_player_name(team_id, player_id, event)
	var receiver_team_id: int = int(event.get("target_team_id", team_id))
	var receiver_index: int = int(event.get("target_player_id", player_index + 1))

	var passer_node: Node3D = null
	var receiver_node: Node3D = null

	print("\n📨 Pass - %s (Player: %s)" % [time_label, passer_name])

	if event.has("from") and event.has("to") and event["from"] != null and event["to"] != null:
		var from_pos := _extract_meter_vector(event["from"], _default_run_start(team_id))
		var forward_offset := 8.0 if team_id == 0 else -8.0
		var to_pos := _extract_meter_vector(event["to"], from_pos + Vector2(forward_offset, 0.0))
		var from_world = meter_to_3d(from_pos)
		var to_world = meter_to_3d(to_pos)
		var distance_m := from_pos.distance_to(to_pos)
		var pass_speed := _calculate_pass_ball_speed(event, distance_m)
		var pass_duration: float = max(distance_m / max(pass_speed, 0.1), 0.35) / animation_speed

		print(
			(
				"   📐 From: (%.1f, %.1f) → To: (%.1f, %.1f) | Dist: %.1fm | Speed: %.1fm/s"
				% [from_pos.x, from_pos.y, to_pos.x, to_pos.y, distance_m, pass_speed]
			)
		)

		passer_node = get_team_player(team_id, player_index, 0)
		if passer_node:
			passer_node.global_position = from_world
			passer_node.visible = true
			passer_node.look_at(to_world, Vector3.UP)
			play_animation(passer_node, get_kick_animation(passer_name))

		receiver_node = get_team_player(receiver_team_id, receiver_index, player_index + 1)
		if receiver_node:
			receiver_node.global_position = to_world
			receiver_node.visible = true
			receiver_node.look_at(from_world, Vector3.UP)
			play_animation(receiver_node, "walk")

		if passer_node and receiver_node:
			start_camera_tracking_multi([passer_node, receiver_node], Vector3(15, 25, 15))

		_spawn_speed_trail(from_world, to_world, PASS_TRAIL_COLOR, pass_duration + 0.2)
		_animate_ball_marker_path(from_world, to_world, pass_duration)
		await get_tree().create_timer(pass_duration).timeout

		if receiver_node:
			play_animation(receiver_node, "idle")

		if event_label:
			var info := "거리 %.1fm · 속도 %.1fm/s" % [distance_m, pass_speed]
			if not bool(event.get("ground", event.get("is_ground", true))):
				info += " · Aerial"
			event_label.text = (
				"📨 패스 - %s\n%s\n(%.0f, %.0f) → (%.0f, %.0f)"
				% [time_label, info, from_pos.x, from_pos.y, to_pos.x, to_pos.y]
			)
	else:
		if event_label:
			event_label.text = "📨 패스 - %s" % time_label

	stop_camera_tracking()
	await _reset_camera()

	if passer_node:
		passer_node.visible = false
	if receiver_node:
		receiver_node.visible = false

	print("   ✅ Pass animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


func _animate_run(event: Dictionary) -> void:
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)
	var player_index = get_player_index(team_id, player_id)
	if player_index < 0 and player_id is int:
		player_index = int(player_id)
	var player_name = _resolve_event_player_name(team_id, player_id, event)

	var from_meter := _extract_meter_vector(event.get("from"), _default_run_start(team_id))
	var run_forward := 12.0 if team_id == 0 else -12.0
	var default_to := from_meter + Vector2(run_forward, 0.0)
	var to_meter := _extract_meter_vector(event.get("to"), default_to)
	var from_world := meter_to_3d(from_meter)
	var to_world := meter_to_3d(to_meter)

	var distance_m := from_meter.distance_to(to_meter)
	var speed_mps: float = max(float(event.get("speed_mps", 6.5)), 2.5)
	var duration: float = max(distance_m / speed_mps, 0.6) / animation_speed
	var with_ball := bool(event.get("with_ball", false))

	print("\n🏃 Run - %s (Player: %s, Dist: %.1fm, Speed: %.1fm/s)" % [time_label, player_name, distance_m, speed_mps])

	var runner := get_team_player(team_id, player_index, 0)
	if not runner:
		await get_tree().create_timer(duration).timeout
		event_finished.emit(event)
		current_event_index += 1
		_play_next_event()
		return

	runner.global_position = from_world
	runner.visible = true
	runner.look_at(to_world, Vector3.UP)
	play_animation(runner, "sprint")

	start_camera_tracking(runner, 45.0, Vector3(15, 22, 15))
	_spawn_speed_trail(from_world, to_world, RUN_TRAIL_COLOR, duration + 0.4)
	if with_ball:
		_animate_ball_marker_path(from_world, to_world, duration)

	var tween = create_tween()
	tween.tween_property(runner, "global_position", to_world, duration).set_trans(Tween.TRANS_SINE).set_ease(
		Tween.EASE_IN_OUT
	)
	await tween.finished

	if event_label:
		var ball_suffix := " · with ball" if with_ball else ""
		event_label.text = "🏃 Run - %s\n거리 %.1fm · 속도 %.1fm/s%s" % [time_label, distance_m, speed_mps, ball_suffix]

	await get_tree().create_timer(0.2).timeout
	stop_camera_tracking()
	await _reset_camera()
	runner.visible = false

	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


func _animate_dribble(event: Dictionary) -> void:
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)
	var player_index = get_player_index(team_id, player_id)
	if player_index < 0 and player_id is int:
		player_index = int(player_id)
	var player_name = _resolve_event_player_name(team_id, player_id, event)

	var from_meter := _extract_meter_vector(event.get("from"), _default_run_start(team_id))
	var dribble_forward := 8.0 if team_id == 0 else -8.0
	var default_to := from_meter + Vector2(dribble_forward, randf_range(-2.0, 2.0))
	var to_meter := _extract_meter_vector(event.get("to"), default_to)
	var from_world := meter_to_3d(from_meter)
	var to_world := meter_to_3d(to_meter)

	var distance_m := from_meter.distance_to(to_meter)
	var touches: int = int(event.get("touches", max(1, int(distance_m / 2.5))))
	var speed_mps: float = max(float(event.get("speed_mps", 5.0)), 2.0)
	var duration: float = max(distance_m / speed_mps, 0.6) / animation_speed

	print("\n🌀 Dribble - %s (Player: %s, Dist: %.1fm, Touches: %d)" % [time_label, player_name, distance_m, touches])

	var runner := get_team_player(team_id, player_index, 0)
	if not runner:
		await get_tree().create_timer(duration).timeout
		event_finished.emit(event)
		current_event_index += 1
		_play_next_event()
		return

	runner.global_position = from_world
	runner.visible = true
	runner.look_at(to_world, Vector3.UP)
	play_animation(runner, "sprint")
	start_camera_tracking(runner, 42.0, Vector3(12, 20, 12))
	_spawn_speed_trail(from_world, to_world, DRIBBLE_TRAIL_COLOR, duration + 0.3)
	_animate_ball_marker_path(from_world, to_world, duration)

	var tween = create_tween()
	tween.tween_property(runner, "global_position", to_world, duration).set_trans(Tween.TRANS_CUBIC).set_ease(
		Tween.EASE_OUT
	)
	await tween.finished

	if event_label:
		event_label.text = "🌀 Dribble - %s\n거리 %.1fm · 터치 %d회" % [time_label, distance_m, touches]

	await get_tree().create_timer(0.2).timeout
	stop_camera_tracking()
	await _reset_camera()
	runner.visible = false

	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


func _animate_through_ball(event: Dictionary) -> void:
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var team_id = base.get("team_id", 0)
	var passer_id = base.get("player_id", null)
	var passer_index = get_player_index(team_id, passer_id)
	if passer_index < 0 and passer_id is int:
		passer_index = int(passer_id)
	var passer_name = _resolve_event_player_name(team_id, passer_id, event)

	var from_meter := _extract_meter_vector(event.get("from"), _default_run_start(team_id))
	var through_forward := 18.0 if team_id == 0 else -18.0
	var default_to := from_meter + Vector2(through_forward, randf_range(-4.0, 4.0))
	var to_meter := _extract_meter_vector(event.get("to"), default_to)
	var from_world := meter_to_3d(from_meter)
	var to_world := meter_to_3d(to_meter)

	var receiver_id = event.get("receiver_id", null)
	var receiver_index = get_player_index(team_id, receiver_id)
	if receiver_index < 0:
		receiver_index = passer_index + 1

	var runner := get_team_player(team_id, receiver_index, 1)
	var runner_offset := -6.0 if team_id == 0 else 6.0
	var runner_start_meter := to_meter + Vector2(runner_offset, randf_range(-3.0, 3.0))
	var runner_start_world := meter_to_3d(runner_start_meter)
	var runner_distance := runner_start_meter.distance_to(to_meter)
	var runner_speed := float(event.get("runner_speed", event.get("speed_mps", 6.8)))
	if runner_speed <= 0.0:
		runner_speed = 6.8
	var runner_duration: float = max(runner_distance / runner_speed, 0.6) / animation_speed

	var ball_distance := from_meter.distance_to(to_meter)
	var ball_speed := _calculate_pass_ball_speed(event, ball_distance, 24.0)
	var pass_duration: float = max(ball_distance / max(ball_speed, 0.1), 0.5) / animation_speed

	print(
		(
			"🚀 Through Ball - %s (Passer: %s, Dist: %.1fm, Speed: %.1fm/s)"
			% [time_label, passer_name, ball_distance, ball_speed]
		)
	)

	var passer := get_team_player(team_id, passer_index, 0)
	if passer:
		passer.global_position = from_world
		passer.visible = true
		passer.look_at(to_world, Vector3.UP)
		play_animation(passer, get_kick_animation(passer_name))

	if runner:
		runner.global_position = runner_start_world
		runner.visible = true
		runner.look_at(to_world, Vector3.UP)
		play_animation(runner, "sprint")

	if passer and runner:
		start_camera_tracking_multi([passer, runner], Vector3(18, 24, 18))
	elif runner:
		start_camera_tracking(runner, 40.0, Vector3(15, 20, 15))

	_spawn_speed_trail(from_world, to_world, THROUGH_BALL_TRAIL_COLOR, pass_duration + 0.5)
	_animate_ball_marker_path(from_world, to_world, pass_duration)

	if runner:
		var runner_tween = create_tween()
		(
			runner_tween
			. tween_property(runner, "global_position", to_world, runner_duration)
			. set_trans(Tween.TRANS_SINE)
			. set_ease(Tween.EASE_OUT)
		)
		await runner_tween.finished
	else:
		await get_tree().create_timer(pass_duration).timeout

	if event_label:
		event_label.text = "🚀 스루패스 - %s\n거리 %.1fm · 공 속도 %.1fm/s" % [time_label, ball_distance, ball_speed]

	await get_tree().create_timer(0.2).timeout
	stop_camera_tracking()
	await _reset_camera()

	if passer:
		passer.visible = false
	if runner:
		runner.visible = false
	if player_marker:
		player_marker.visible = false

	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


func _animate_ball_move(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)

	print("\n⚡ Ball Move - %s" % time_label)

	# 볼 위치 표시
	if event.has("to") and event["to"] != null:
		var pos_dict = event["to"]
		var meter_pos = Vector2(pos_dict.x, pos_dict.y)
		var world_pos = meter_to_3d(meter_pos)

		if player_marker:
			player_marker.global_position = world_pos
			player_marker.visible = true

		if event_label:
			event_label.text = "⚡ Ball Move - %s\n위치: (%.0f, %.0f)" % [time_label, meter_pos.x, meter_pos.y]
	else:
		if event_label:
			event_label.text = "⚡ Ball Move - %s" % time_label

	# 짧은 대기
	await get_tree().create_timer(0.3).timeout

	# ✅ FIX: Don't hide the ball - let it stay visible
	# if player_marker:
	# 	player_marker.visible = false

	print("   ✅ Ball move animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


func _extract_meter_vector(value: Variant, fallback: Vector2 = Vector2(52.5, 34.0)) -> Vector2:
	if value == null:
		return fallback
	if value is Vector2:
		return value
	if value is Dictionary:
		var dict: Dictionary = value
		var x_val := float(dict.get("x", dict.get("0", fallback.x)))
		var y_val := float(dict.get("y", dict.get("1", dict.get("z", fallback.y))))
		return Vector2(x_val, y_val)
	if value is Array:
		var arr: Array = value
		if arr.size() >= 2:
			return Vector2(float(arr[0]), float(arr[1]))
	if value is PackedFloat32Array:
		var pack32: PackedFloat32Array = value
		if pack32.size() >= 2:
			return Vector2(float(pack32[0]), float(pack32[1]))
	if value is PackedFloat64Array:
		var pack64: PackedFloat64Array = value
		if pack64.size() >= 2:
			return Vector2(float(pack64[0]), float(pack64[1]))
	return fallback


func _calculate_pass_ball_speed(event: Dictionary, distance_m: float, default_speed: float = 18.0) -> float:
	var explicit := float(event.get("ball_speed", -1.0))
	if explicit > 0.0:
		return clamp(explicit, 6.0, 35.0)
	var pass_force := float(event.get("pass_force", -1.0))
	if pass_force > 0.0:
		return clamp(8.0 + pass_force * 5.0, 7.5, 32.0)
	if distance_m > 0.0:
		return clamp(8.0 + distance_m * 0.6, 7.0, 28.0)
	return default_speed


func _calculate_shot_ball_speed(event: Dictionary, distance_m: float) -> float:
	var explicit := float(event.get("ball_speed", -1.0))
	if explicit > 0.0:
		return clamp(explicit, 12.0, 40.0)
	if distance_m > 0.0:
		return clamp(18.0 + distance_m * 0.5, 14.0, 35.0)
	return 24.0


func _default_run_start(team_id: int) -> Vector2:
	if team_id == 0:
		return Vector2(45.0, randf_range(20.0, 48.0))
	return Vector2(60.0, randf_range(20.0, 48.0))


func _spawn_speed_trail(from_world: Vector3, to_world: Vector3, color: Color, lifespan: float) -> void:
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = color
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.metallic = 0.0
	material.roughness = 0.1

	var mesh := ImmediateMesh.new()
	mesh.surface_begin(Mesh.PRIMITIVE_LINES, material)
	mesh.surface_add_vertex(from_world)
	mesh.surface_add_vertex(to_world)
	mesh.surface_end()

	var line := MeshInstance3D.new()
	line.mesh = mesh
	add_child(line)
	_transient_nodes.append(line)
	var timer := get_tree().create_timer(max(lifespan, 0.2))
	timer.timeout.connect(
		func():
			if is_instance_valid(line):
				line.queue_free()
			_transient_nodes.erase(line)
	)


func _animate_ball_marker_path(start: Vector3, finish: Vector3, duration: float) -> void:
	if not player_marker:
		return
	if player_marker is RigidBody3D:
		player_marker.freeze = true
		player_marker.linear_velocity = Vector3.ZERO
		player_marker.angular_velocity = Vector3.ZERO
	player_marker.visible = true
	player_marker.global_position = start
	var tween = create_tween()
	tween.tween_property(player_marker, "global_position", finish, duration).set_trans(Tween.TRANS_SINE).set_ease(
		Tween.EASE_OUT
	)
	tween.finished.connect(
		func():
			if player_marker:
				player_marker.visible = false
	)


## 태클 이벤트 애니메이션
func _animate_tackle(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)
	var success = event.get("success", true)

	# ✅ NEW: Roster에서 실제 이름 찾기 (tackler)
	var player_index = int(player_id) if player_id != null else -1
	var tackler_name = _resolve_event_player_name(team_id, player_id, event)
	var opponent_team_id: int = 1 - int(team_id)

	var victim_index: int = -1
	var victim_id_variant = event.get("victim_id", null)
	if victim_id_variant is int:
		victim_index = victim_id_variant
	elif victim_id_variant is float:
		victim_index = int(victim_id_variant)
	elif victim_id_variant is String and victim_id_variant != "알 수 없음":
		victim_index = int(float(victim_id_variant))

	var victim_name = get_player_name(opponent_team_id, victim_index) if victim_index >= 0 else str(victim_id_variant)

	print("\n💥 Tackle - %s (Tackler: %s, Victim: %s, Success: %s)" % [time_label, tackler_name, victim_name, success])

	# from: 공을 가진 선수 위치, to: 태클 후 공 위치
	if event.has("from") and event.has("to") and event["from"] != null and event["to"] != null:
		var from_dict = event["from"]
		var to_dict = event["to"]
		var from_pos = Vector2(from_dict.x, from_dict.y)
		var to_pos = Vector2(to_dict.x, to_dict.y)
		var from_world = meter_to_3d(from_pos)
		var to_world = meter_to_3d(to_pos)

		print("   📍 Tackle at: (%.1f, %.1f)" % [from_pos.x, from_pos.y])

		# 1. 공을 가진 선수 (victim/ball carrier)
		var victim = get_team_player(opponent_team_id, victim_index, 0)
		if victim:
			victim.global_position = from_world
			victim.visible = true

			# 드리블 방향 (from → to)
			victim.look_at(to_world, Vector3.UP)

			# sprint 애니메이션 (공 가지고 달리는 중)
			play_animation(victim, "sprint")

			print("   🏃 Victim (ball carrier) at: ", from_world)

		# 2. 태클하는 선수 (tackler)
		var tackler = get_team_player(team_id, player_index, 0)
		if tackler:
			# 태클러는 약간 옆/뒤에서 접근
			var tackler_offset = Vector3(-2, 0, -2)
			tackler.global_position = from_world + tackler_offset
			tackler.visible = true

			# 공 가진 선수 바라보기
			tackler.look_at(from_world, Vector3.UP)

			# sprint 애니메이션 (달려들기)
			play_animation(tackler, "sprint")

			print("   💨 Tackler approaching from: ", tackler.global_position)

		# 접근 시간 (0.3초)
		await get_tree().create_timer(0.3).timeout

		# 3. 태클 동작
		if tackler:
			# 태클 애니메이션
			var rnd = cosmetic_rng.randf() if cosmetic_rng else randf()
			var tackle_anim = "attack-melee-right" if rnd > 0.5 else "attack-melee-left"
			play_animation(tackler, tackle_anim)

		if victim:
			if success:
				# 태클 성공: victim이 넘어지거나 공 빼앗김
				var rnd = cosmetic_rng.randf() if cosmetic_rng else randf()
				var reaction_anim = "die" if rnd > 0.5 else "fall"
				play_animation(victim, reaction_anim)
				print("   ✅ Tackle successful - victim loses ball")
			else:
				# 태클 실패: victim이 계속 달림
				play_animation(victim, "sprint")
				print("   ❌ Tackle failed - victim escapes")

		# 공 위치 이동
		if player_marker:
			player_marker.global_position = from_world
			player_marker.visible = true

		# 카메라 다중 타겟 추적 (tackler + victim)
		if tackler and victim:
			start_camera_tracking_multi([tackler, victim], Vector3(10, 15, 10))
			print("   🎥 Camera tracking tackle action")

		# 추적 초기화 대기
		await get_tree().create_timer(0.3).timeout

		# 이벤트 정보 표시
		if event_label:
			var result_text = "성공!" if success else "실패"
			event_label.text = "💥 Tackle %s - %s\n위치: (%.0f, %.0f)" % [result_text, time_label, from_pos.x, from_pos.y]

	# 태클 동작 완료 대기
	print("   ⏱️ Tackle holding for %.1f seconds..." % ANIMATION_MAP["Tackle"]["duration"])
	var duration = ANIMATION_MAP["Tackle"]["duration"] / animation_speed
	await get_tree().create_timer(duration).timeout

	# 카메라 트래킹 중지
	stop_camera_tracking()

	# 정리
	await _reset_camera()
	# ✅ FIX: Don't hide the ball - let it stay visible
	# if player_marker:
	# 	player_marker.visible = false

	# 캐릭터 숨기기
	var victim_cleanup = get_team_player(opponent_team_id, victim_index, 0)
	if victim_cleanup:
		victim_cleanup.visible = false
	var tackler_cleanup = get_team_player(team_id, player_index, 0)
	if tackler_cleanup:
		tackler_cleanup.visible = false

	print("   ✅ Tackle animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 헤딩 이벤트 애니메이션
func _animate_header(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)

	# ✅ NEW: Roster에서 실제 이름 찾기
	var player_index = int(player_id) if player_id != null else -1
	var player_name = _resolve_event_player_name(team_id, player_id, event)

	print("\n🦘 Header - %s (Player: %s)" % [time_label, player_name])

	# 헤딩 위치
	if event.has("position") and event["position"] != null:
		var pos_dict = event["position"]
		var meter_pos = Vector2(pos_dict.x, pos_dict.y)
		var world_pos = meter_to_3d(meter_pos)

		print("   📍 Header at: (%.1f, %.1f)" % [meter_pos.x, meter_pos.y])

		# 헤딩하는 선수
		var header_player = get_team_player(team_id, player_index, 0)
		if header_player:
			# 약간 뒤에서 달려오는 위치
			var run_start = world_pos - Vector3(0, 0, 5)
			header_player.global_position = run_start
			header_player.visible = true

			# 헤딩 위치 바라보기
			header_player.look_at(world_pos, Vector3.UP)

			# 1. Sprint 애니메이션 (달려들기)
			play_animation(header_player, "sprint")

			print("   🏃 Player running from: ", run_start)
			print("   🎯 Heading to: ", world_pos)

			# 카메라 자동 추적 시작
			start_camera_tracking(header_player, 50.0, Vector3(12, 18, 12))
			print("   🎥 Camera tracking header player")

			# 달리는 동안 위치 이동 (Tween)
			var tween = create_tween()
			tween.tween_property(header_player, "global_position", world_pos, 0.5)
			await tween.finished

			# 2. Jump 애니메이션 (점프 헤딩)
			play_animation(header_player, "jump")

			print("   🦘 Player jumping for header!")

		# 공 위치 표시
		if player_marker:
			player_marker.global_position = world_pos + Vector3(0, 2, 0)  # 공중에
			player_marker.visible = true

		# 점프 애니메이션 대기
		await get_tree().create_timer(0.3).timeout

		# 이벤트 정보 표시
		if event_label:
			event_label.text = (
				"🦘 Header - %s\n선수: %s\n위치: (%.0f, %.0f)" % [time_label, player_name, meter_pos.x, meter_pos.y]
			)

	# 헤딩 동작 완료 대기
	print("   ⏱️ Header holding for %.1f seconds..." % ANIMATION_MAP["Header"]["duration"])
	var duration = ANIMATION_MAP["Header"]["duration"] / animation_speed
	await get_tree().create_timer(duration).timeout

	# 카메라 트래킹 중지
	stop_camera_tracking()

	# 정리
	await _reset_camera()
	# ✅ FIX: Don't hide the ball - let it stay visible
	# if player_marker:
	# 	player_marker.visible = false

	# 캐릭터 숨기기
	var header_player = get_team_player(team_id, player_index, 0)
	if header_player:
		header_player.visible = false

	print("   ✅ Header animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 킥오프 이벤트
func _animate_kickoff(event: Dictionary) -> void:
	print("\n⚪ Kickoff")

	if event_label:
		event_label.text = "⚪ Kickoff"

	var center = Vector2(OF_FIELD_LENGTH / 2.0, OF_FIELD_WIDTH / 2.0)
	var visuals: Array = []

	var home_forward = get_team_player(0, 0, 0)
	if home_forward:
		var pos = meter_to_3d(Vector2(center.x - 1.5, center.y))
		home_forward.global_position = pos
		home_forward.visible = true
		home_forward.look_at(meter_to_3d(Vector2(center.x + 20.0, center.y)), Vector3.UP)
		play_animation(home_forward, "idle")
		visuals.append(home_forward)

	var home_support = get_team_player(0, 1, 1)
	if home_support:
		var pos_support = meter_to_3d(Vector2(center.x - 4.0, center.y + 3.0))
		home_support.global_position = pos_support
		home_support.visible = true
		home_support.look_at(meter_to_3d(Vector2(center.x + 20.0, center.y)), Vector3.UP)
		play_animation(home_support, "walk")
		visuals.append(home_support)

	var away_forward = get_team_player(1, 0, 0)
	if away_forward:
		var pos_away = meter_to_3d(Vector2(center.x + 1.5, center.y))
		away_forward.global_position = pos_away
		away_forward.visible = true
		away_forward.look_at(meter_to_3d(Vector2(center.x - 20.0, center.y)), Vector3.UP)
		play_animation(away_forward, "idle")
		visuals.append(away_forward)

	var away_support = get_team_player(1, 1, 1)
	if away_support:
		var pos_away_support = meter_to_3d(Vector2(center.x + 4.0, center.y - 3.0))
		away_support.global_position = pos_away_support
		away_support.visible = true
		away_support.look_at(meter_to_3d(Vector2(center.x - 20.0, center.y)), Vector3.UP)
		play_animation(away_support, "walk")
		visuals.append(away_support)

	if player_marker:
		player_marker.global_position = meter_to_3d(center)
		player_marker.linear_velocity = Vector3.ZERO
		player_marker.angular_velocity = Vector3.ZERO
		player_marker.visible = true

	if visuals.size() > 0:
		start_camera_tracking_multi(visuals, Vector3(20, 30, 20))

	var duration = EVENT_DURATION["kickoff"] / animation_speed
	await get_tree().create_timer(duration).timeout

	stop_camera_tracking()
	await _reset_camera()

	for actor in visuals:
		if actor:
			actor.visible = false

	print("   ✅ Kickoff complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 하프 타임 이벤트
func _animate_half_time(event: Dictionary) -> void:
	print("\n⏸️ Half Time")

	if event_label:
		event_label.text = "⏸️ Half Time"

	await _reset_camera()

	print("   ⏱️ Half time holding for 2.0 seconds...")
	var duration = 2.0 / animation_speed
	await get_tree().create_timer(duration).timeout

	print("   ✅ Half time complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 종료 이벤트
func _animate_full_time(event: Dictionary) -> void:
	print("\n🏁 Full Time")

	if event_label:
		event_label.text = "🏁 Full Time"

	await _reset_camera()

	print("   ⏱️ Full time holding for %.1f seconds..." % EVENT_DURATION["full_time"])
	var duration = EVENT_DURATION["full_time"] / animation_speed
	await get_tree().create_timer(duration).timeout

	print("   ✅ Full time complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 옐로우 카드 이벤트 애니메이션
func _animate_yellow_card(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)

	# ✅ NEW: Roster에서 실제 이름 찾기
	var player_index = int(player_id) if player_id != null else -1
	var player_name = _resolve_event_player_name(team_id, player_id, event)

	print("\n🟨 Yellow Card - %s (Player: %s, Team: %d)" % [time_label, player_name, team_id])

	# 경고 받은 선수 위치 (이벤트에 position 필드가 있으면 사용)
	if event.has("position") and event.position != null:
		var pos_dict = event.position
		var meter_pos = Vector2(pos_dict.x, pos_dict.y)
		var world_pos = meter_to_3d(meter_pos)

		# 경고 받은 선수
		var warned_player = get_team_player(team_id, player_index, 0)
		if warned_player:
			warned_player.global_position = world_pos
			warned_player.visible = true

			# idle 애니메이션 (주심을 바라보며 대기)
			play_animation(warned_player, "idle")

			print("   🟨 Warned player at: ", world_pos)

			# 카메라 추적
			start_camera_tracking(warned_player, 40.0, Vector3(12, 18, 12))
			print("   🎥 Camera tracking warned player")

		# 마커로 위치 표시
		if player_marker:
			player_marker.global_position = world_pos
			player_marker.visible = true

		# 이벤트 정보 표시
		if event_label:
			event_label.text = "🟨 Yellow Card - %s\n선수: %s" % [time_label, player_name]
	else:
		# 위치 정보 없으면 간단히 표시
		if event_label:
			event_label.text = "🟨 Yellow Card - %s\n선수: %s" % [time_label, player_name]

	# 카드 제시 장면 대기 (2초)
	print("   ⏱️ Yellow card showing for 2.0 seconds...")
	await get_tree().create_timer(2.0 / animation_speed).timeout

	# 카메라 추적 중지
	stop_camera_tracking()

	# 정리
	await _reset_camera()
	# ✅ FIX: Don't hide the ball - let it stay visible
	# if player_marker:
	# 	player_marker.visible = false

	var warned_player = get_team_player(team_id, player_index, 0)
	if warned_player:
		warned_player.visible = false

	print("   ✅ Yellow card animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 레드 카드 이벤트 애니메이션
func _animate_red_card(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)

	# ✅ NEW: Roster에서 실제 이름 찾기
	var player_index = int(player_id) if player_id != null else -1
	var player_name = _resolve_event_player_name(team_id, player_id, event)

	print("\n🟥 Red Card - %s (Player: %s, Team: %d)" % [time_label, player_name, team_id])

	# 퇴장 당한 선수 위치
	if event.has("position") and event.position != null:
		var pos_dict = event.position
		var meter_pos = Vector2(pos_dict.x, pos_dict.y)
		var world_pos = meter_to_3d(meter_pos)

		# 퇴장 당한 선수
		var expelled_player = get_team_player(team_id, player_index, 0)
		if expelled_player:
			expelled_player.global_position = world_pos
			expelled_player.visible = true

			# 1. idle 애니메이션 (주심 바라보기)
			play_animation(expelled_player, "idle")

			print("   🟥 Expelled player at: ", world_pos)

			# 카메라 추적
			start_camera_tracking(expelled_player, 40.0, Vector3(12, 18, 12))
			print("   🎥 Camera tracking expelled player")

			# 카드 제시 대기 (1.5초)
			await get_tree().create_timer(1.5).timeout

			# 2. 슬픔 애니메이션
			play_animation(expelled_player, "emote-no")
			print("   😢 Player reacting to red card")

		# 마커로 위치 표시
		if player_marker:
			player_marker.global_position = world_pos
			player_marker.visible = true

		# 이벤트 정보 표시
		if event_label:
			event_label.text = "🟥 Red Card - %s\n선수: %s\n퇴장!" % [time_label, player_name]
	else:
		if event_label:
			event_label.text = "🟥 Red Card - %s\n선수: %s" % [time_label, player_name]

	# 퇴장 장면 대기 (3초 - 레드카드는 더 극적으로)
	print("   ⏱️ Red card showing for 3.0 seconds...")
	await get_tree().create_timer(3.0 / animation_speed).timeout

	# 카메라 추적 중지
	stop_camera_tracking()

	# 정리
	await _reset_camera()
	# ✅ FIX: Don't hide the ball - let it stay visible
	# if player_marker:
	# 	player_marker.visible = false

	var expelled_player = get_team_player(team_id, player_index, 0)
	if expelled_player:
		expelled_player.visible = false

	print("   ✅ Red card animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 선수 교체 이벤트 애니메이션
func _animate_substitution(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var team_id = base.get("team_id", 0)

	# ✅ NEW: Roster에서 실제 이름 찾기 (player_out, player_in)
	var player_out_id = event.get("player_out", null)
	var player_in_id = event.get("player_in", null)

	var player_out_index = int(player_out_id) if player_out_id != null else -1
	var player_in_index = int(player_in_id) if player_in_id != null else -1

	var player_out_name = _resolve_event_player_name(team_id, player_out_id, event)
	var player_in_name = _resolve_event_player_name(team_id, player_in_id, event)

	print("\n🔄 Substitution - %s (Out: %s → In: %s, Team: %d)" % [time_label, player_out_name, player_in_name, team_id])

	# 교체 위치 (터치라인 근처)
	var sub_world = meter_to_3d(Vector2(52.5, 5.0))  # 중앙, 터치라인 근처

	# 1. 교체 아웃되는 선수
	var player_out = get_team_player(team_id, player_out_index, 0)
	if player_out:
		player_out.global_position = sub_world
		player_out.visible = true

		# walk 애니메이션 (벤치로 걸어가기)
		play_animation(player_out, "walk")

		print("   👋 Player out: %s at: " % player_out_id, sub_world)

	# 2. 교체 인되는 선수
	var player_in = get_team_player(team_id, player_in_index, 1)
	if player_in:
		# 약간 옆에서 들어오기
		var in_pos = sub_world + Vector3(3, 0, 0)
		player_in.global_position = in_pos
		player_in.visible = true

		# sprint 애니메이션 (경기장으로 뛰어들어가기)
		play_animation(player_in, "sprint")

		print("   👏 Player in: %s at: " % player_in_id, in_pos)

	# 카메라 다중 타겟 추적
	if player_out and player_in:
		start_camera_tracking_multi([player_out, player_in], Vector3(12, 18, 12))
		print("   🎥 Camera tracking substitution")

	# 이벤트 정보 표시
	if event_label:
		event_label.text = "🔄 Substitution - %s\nOut: %s\nIn: %s" % [time_label, player_out_id, player_in_id]

	# 교체 장면 대기 (2초)
	print("   ⏱️ Substitution showing for 2.0 seconds...")
	await get_tree().create_timer(2.0 / animation_speed).timeout

	# 카메라 추적 중지
	stop_camera_tracking()

	# 정리
	await _reset_camera()

	if player_out:
		player_out.visible = false
	if player_in:
		player_in.visible = false

	print("   ✅ Substitution animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 부상 이벤트 애니메이션
func _animate_injury(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access nested base structure
	var base = event.get("base", {})
	var time_label = _time_label_from_base(base)
	var player_id = base.get("player_id", null)
	var team_id = base.get("team_id", 0)
	var severity = event.get("severity", "minor")  # minor, serious

	# ✅ NEW: Roster에서 실제 이름 찾기
	var player_index = int(player_id) if player_id != null else -1
	var player_name = _resolve_event_player_name(team_id, player_id, event)

	print("\n🚑 Injury - %s (Player: %s, Severity: %s)" % [time_label, player_name, severity])

	# 부상 위치
	if event.has("position") and event.position != null:
		var pos_dict = event.position
		var meter_pos = Vector2(pos_dict.x, pos_dict.y)
		var world_pos = meter_to_3d(meter_pos)

		# 부상당한 선수
		var injured_player = get_team_player(team_id, player_index, 0)
		if injured_player:
			injured_player.global_position = world_pos
			injured_player.visible = true

			# 부상 애니메이션 (넘어짐)
			var injury_anim = "die" if severity == "serious" else "fall"
			play_animation(injured_player, injury_anim)

			print("   🤕 Injured player at: ", world_pos)
			print("   💥 Injury severity: %s" % severity)

			# 카메라 추적
			start_camera_tracking(injured_player, 35.0, Vector3(10, 15, 10))
			print("   🎥 Camera tracking injured player")

		# 마커로 위치 표시
		if player_marker:
			player_marker.global_position = world_pos
			player_marker.visible = true

		# 이벤트 정보 표시
		if event_label:
			var severity_text = "심각" if severity == "serious" else "경미"
			event_label.text = "🚑 Injury (%s) - %s\n선수: %s" % [severity_text, time_label, player_name]
	else:
		if event_label:
			event_label.text = "🚑 Injury - %s\n선수: %s" % [time_label, player_name]

	# 부상 장면 대기 (심각도에 따라 다르게)
	var hold_time = 3.0 if severity == "serious" else 2.0
	print("   ⏱️ Injury showing for %.1f seconds..." % hold_time)
	await get_tree().create_timer(hold_time / animation_speed).timeout

	# 카메라 추적 중지
	stop_camera_tracking()

	# 정리
	await _reset_camera()
	# ✅ FIX: Don't hide the ball - let it stay visible
	# if player_marker:
	# 	player_marker.visible = false

	var injured_player = get_team_player(team_id, player_index, 0)
	if injured_player:
		injured_player.visible = false

	print("   ✅ Injury animation complete!\n")
	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 기본 이벤트 애니메이션
func _animate_generic(event: Dictionary) -> void:
	# ✅ FULL WRAPPING: Access event.kind instead of event.type
	var event_kind = event.get("kind", "unknown")
	print("📋 Event: %s" % event_kind)

	if event_label:
		event_label.text = "📋 %s" % event_kind

	await get_tree().create_timer(0.5 / animation_speed).timeout

	event_finished.emit(event)
	current_event_index += 1
	_play_next_event()


## 카메라 리셋 (초기 아이소메트릭 뷰) - 2.4배 스케일 필드 중심
func _clear_transient_nodes() -> void:
	for node in _transient_nodes:
		if is_instance_valid(node):
			node.queue_free()
	_transient_nodes.clear()


func _reset_camera() -> void:
	_clear_transient_nodes()
	if camera:
		print("   🔙 Resetting camera to default position")
		var tween = create_tween()
		tween.set_parallel(true)
		# 필드 중심 (168.51, -0.5, 94.69) + 아이소메트릭 오프셋
		tween.tween_property(camera, "position", Vector3(220, 150, 147), 1.0).set_trans(Tween.TRANS_QUAD)
		tween.tween_property(camera, "size", SIZE_DEFAULT, 1.0).set_trans(Tween.TRANS_QUAD)

		# Tween 완료 대기
		await tween.finished
		print("   ✅ Camera reset complete!")


## 재생 속도 변경
func set_animation_speed(speed: float) -> void:
	animation_speed = clamp(speed, 0.5, 3.0)
	print("⏱️ Animation speed: %.1fx" % animation_speed)


## UI 버튼 시그널 핸들러
func _on_play_button_pressed() -> void:
	# 테스트용: 더미 이벤트 재생
	if current_events.is_empty():
		print("⚠️ No events loaded. Loading test events...")
		_load_test_events()
	else:
		play_events(current_events)


func _on_stop_button_pressed() -> void:
	stop()


## 테스트용 더미 이벤트 로드 (다양한 위치)
func _load_test_events() -> void:
	var test_events = [
		{"type": "kickoff", "time": 0},
		# Pass 애니메이션 테스트
		{"type": "pass", "time": 5, "player_id": "김민수", "from": {"x": 50.0, "y": 34.0}, "to": {"x": 70.0, "y": 40.0}},
		# Tackle 애니메이션 테스트 (성공)
		{
			"type": "tackle",
			"time": 12,
			"tackler_id": "박지성",
			"victim_id": "메시",
			"success": true,
			"from": {"x": 60.0, "y": 30.0},
			"to": {"x": 65.0, "y": 32.0}
		},
		# Goal 애니메이션 테스트
		{"type": "goal", "time": 18, "player_id": "손흥민", "position": {"x": 88.0, "y": 34.0}},
		# Yellow Card 애니메이션 테스트
		{"type": "yellow_card", "time": 22, "player_id": "메시", "team_id": 1, "position": {"x": 60.0, "y": 30.0}},
		# Shot 애니메이션 테스트
		{"type": "shot", "time": 25, "player_id": "이청용", "on_target": true, "from": {"x": 75.0, "y": 20.0}},
		# Header 애니메이션 테스트
		{"type": "header", "time": 35, "player_id": "김영권", "position": {"x": 95.0, "y": 40.0}},
		# Injury 애니메이션 테스트 (경미)
		{
			"type": "injury",
			"time": 40,
			"player_id": "호날두",
			"team_id": 1,
			"severity": "minor",
			"position": {"x": 70.0, "y": 50.0}
		},
		# Tackle 애니메이션 테스트 (실패)
		{
			"type": "tackle",
			"time": 45,
			"tackler_id": "백승호",
			"victim_id": "호날두",
			"success": false,
			"from": {"x": 55.0, "y": 34.0},
			"to": {"x": 60.0, "y": 34.0}
		},
		{"type": "half_time", "time": 45},
		# Substitution 애니메이션 테스트
		{"type": "substitution", "time": 55, "player_out": "김민수", "player_in": "이강인", "team_id": 0},
		# Red Card 애니메이션 테스트
		{"type": "red_card", "time": 65, "player_id": "박지성", "team_id": 0, "position": {"x": 50.0, "y": 34.0}},
		# Pass → Goal 연속 이벤트
		{"type": "pass", "time": 70, "player_id": "황희찬", "from": {"x": 80.0, "y": 30.0}, "to": {"x": 95.0, "y": 34.0}},
		{"type": "goal", "time": 71, "player_id": "황의조", "position": {"x": 95.0, "y": 34.0}},
		# Injury 애니메이션 테스트 (심각)
		{
			"type": "injury",
			"time": 80,
			"player_id": "이청용",
			"team_id": 0,
			"severity": "serious",
			"position": {"x": 90.0, "y": 20.0}
		},
		{"type": "full_time", "time": 90}
	]

	play_events(test_events)
