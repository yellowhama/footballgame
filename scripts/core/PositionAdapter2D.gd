class_name PositionAdapter2D
extends Node
## 2D Isometric Position Adapter
##
## OpenFootball Rust 엔진의 3D 좌표(Vector3)를
## Godot 2D 쿼터뷰 아이소메트릭 화면 좌표(Vector2)로 변환
##
## Field Dimensions (meters, SSOT: FieldSpec):
## - Length: 105m (x-axis in Rust)
## - Width: 68m (y-axis in Rust)
## - z-axis: Height (jumps, ball trajectory)

# 축구장 크기 (미터 기준, SSOT: FieldSpec)
const FIELD_LENGTH_M: float = FieldSpec.FIELD_LENGTH_M  # Rust x-axis (length)
const FIELD_WIDTH_M: float = FieldSpec.FIELD_WIDTH_M  # Rust y-axis (width)
const HALF_LENGTH_M: float = FIELD_LENGTH_M * 0.5
const HALF_WIDTH_M: float = FIELD_WIDTH_M * 0.5

# 화면 스케일 (픽셀/미터) - 뷰 전용 파라미터
const SCALE_FACTOR: float = 8.0  # 1미터 = 8픽셀

# 아이소메트릭 각도 (45도 쿼터뷰)
const ISO_ANGLE: float = PI / 4  # 0.785 radians

# 높이 투영 비율
const HEIGHT_RATIO: float = 0.5  # z축 높이가 화면에서 절반 크기로 표시


## Rust Vector3 → Godot Vector2 변환
##
## Rust 좌표계:
## - (0, 0, 0) = 필드 중심
## - x: -52.5 ~ 52.5 (폭 105m) → 화면 좌→우
## - y: -34 ~ 34 (높이 68m) → 화면 위→아래
## - z: 0 ~ n (공중 높이)
##
## Godot 좌표계:
## - (0, 0) = 화면 중심
## - Pocket League 스타일: 왼쪽 아래 → 오른쪽 위 대각선
## - 골대: 왼쪽 아래 모서리, 오른쪽 위 모서리
func rust_to_screen(rust_pos: Vector3) -> Vector2:
	# Pocket League 스타일 아이소메트릭 변환
	# 왼쪽 아래 → 오른쪽 위
	var iso_x: float = (rust_pos.x - rust_pos.y) * cos(ISO_ANGLE)
	var iso_y: float = -(rust_pos.x + rust_pos.y) * sin(ISO_ANGLE)

	# 높이(z) 보정 - 위로 올라갈수록 화면에서 위쪽으로 이동
	iso_y -= rust_pos.z * HEIGHT_RATIO

	# 화면 픽셀 좌표로 스케일
	return Vector2(iso_x, iso_y) * SCALE_FACTOR


## Godot Vector2 → Rust Vector3 역변환 (z=0 가정)
##
## 마우스 클릭 등 화면 좌표를 필드 좌표로 변환할 때 사용
## z축은 0으로 가정 (지면)
func screen_to_rust(screen_pos: Vector2, z: float = 0.0) -> Vector3:
	# 스케일 제거
	var normalized: Vector2 = screen_pos / SCALE_FACTOR

	# 높이 보정 복원
	normalized.y += z * HEIGHT_RATIO

	# Pocket League 스타일 역변환
	# Forward: iso_x = (x - y) * c, iso_y = -(x + y) * c (where c = cos(45°) = sin(45°))
	# Solving: x = (iso_x - iso_y) / (2c), y = (-iso_x - iso_y) / (2c)
	var c: float = cos(ISO_ANGLE)  # Same as sin(ISO_ANGLE) for 45°
	var rust_x: float = (normalized.x - normalized.y) / (2.0 * c)
	var rust_y: float = (-normalized.x - normalized.y) / (2.0 * c)

	return Vector3(rust_x, rust_y, z)


## 필드 경계 확인
##
## Rust 좌표가 유효한 필드 범위 내에 있는지 검증
func is_in_field(rust_pos: Vector3) -> bool:
	return abs(rust_pos.x) <= HALF_LENGTH_M and abs(rust_pos.y) <= HALF_WIDTH_M


## 화면 좌표가 필드 범위 내인지 확인
func is_screen_in_field(screen_pos: Vector2) -> bool:
	var rust_pos: Vector3 = screen_to_rust(screen_pos)
	return is_in_field(rust_pos)


## 거리 계산 (Rust 좌표계)
##
## 두 위치 사이의 실제 필드 거리 (미터)
func distance_rust(pos_a: Vector3, pos_b: Vector3) -> float:
	return pos_a.distance_to(pos_b)


## 화면 거리 계산
##
## 두 화면 좌표 사이의 픽셀 거리
func distance_screen(pos_a: Vector2, pos_b: Vector2) -> float:
	return pos_a.distance_to(pos_b)


## 필드 주요 위치 (Rust 좌표)
##
## 자주 사용하는 필드 위치들을 미리 정의
class FieldPositions:
	# 골대
	static var LEFT_GOAL: Vector3 = Vector3(-HALF_LENGTH_M, 0.0, 0.0)
	static var RIGHT_GOAL: Vector3 = Vector3(HALF_LENGTH_M, 0.0, 0.0)

	# 페널티 구역 (골대에서 16.5m)
	static var LEFT_PENALTY_SPOT: Vector3 = Vector3(-HALF_LENGTH_M + 11.0, 0.0, 0.0)
	static var RIGHT_PENALTY_SPOT: Vector3 = Vector3(HALF_LENGTH_M - 11.0, 0.0, 0.0)

	# 중앙
	static var CENTER: Vector3 = Vector3.ZERO

	# 코너
	static var TOP_LEFT_CORNER: Vector3 = Vector3(-HALF_LENGTH_M, HALF_WIDTH_M, 0.0)
	static var TOP_RIGHT_CORNER: Vector3 = Vector3(HALF_LENGTH_M, HALF_WIDTH_M, 0.0)
	static var BOTTOM_LEFT_CORNER: Vector3 = Vector3(-HALF_LENGTH_M, -HALF_WIDTH_M, 0.0)
	static var BOTTOM_RIGHT_CORNER: Vector3 = Vector3(HALF_LENGTH_M, -HALF_WIDTH_M, 0.0)


## 디버그 정보 출력
func print_conversion_info(rust_pos: Vector3) -> void:
	var screen_pos: Vector2 = rust_to_screen(rust_pos)
	var rust_restored: Vector3 = screen_to_rust(screen_pos, rust_pos.z)

	print("=== PositionAdapter2D Conversion ===")
	print("Rust (input):    ", rust_pos)
	print("Screen (output): ", screen_pos)
	print("Rust (restored): ", rust_restored)
	print("Error:           ", rust_pos.distance_to(rust_restored))
	print("In field:        ", is_in_field(rust_pos))
