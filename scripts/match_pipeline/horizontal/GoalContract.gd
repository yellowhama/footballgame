extends RefCounted
class_name GoalContract
##
## GoalContract - P0 Goal Contract 좌표 상수 (Rust 엔진과 동기화)
##
## 이 파일은 Rust의 engine/goal.rs와 동일한 값을 유지해야 합니다.
## 변경 시 양쪽 모두 업데이트 필요!
##
## 참조: docs/spec+@/spec_v5/fix/P0_GOAL_CONTRACT.md
##

#region Field Dimensions (미터)
## 필드 크기 (FIFA 표준)
const FIELD_LENGTH_M: float = 105.0  ## 골라인 간 거리
const FIELD_WIDTH_M: float = 68.0  ## 터치라인 간 거리
const FIELD_CENTER_X: float = 52.5  ## 필드 중앙 X
const FIELD_CENTER_Y: float = 34.0  ## 필드 중앙 Y (= 골대 중앙 Y)
#endregion

#region Goal Dimensions (미터)
## 골대 크기 (FIFA 표준)
const GOAL_WIDTH_M: float = 7.32  ## 골대 폭 (포스트 간 거리)
const GOAL_HEIGHT_M: float = 2.44  ## 골대 높이 (크로스바)
const GOAL_HALF_WIDTH: float = 3.66  ## 골대 반폭
#endregion

#region Goal Positions (미터)
## Home 골대 (x=0, 왼쪽) - Home팀이 수비
const HOME_GOAL_X: float = 0.0
const HOME_GOAL_CENTER_Y: float = 34.0
const HOME_GOAL_POST_TOP_Y: float = 30.34  ## 34.0 - 3.66
const HOME_GOAL_POST_BOTTOM_Y: float = 37.66  ## 34.0 + 3.66

## Away 골대 (x=105, 오른쪽) - Away팀이 수비
const AWAY_GOAL_X: float = 105.0
const AWAY_GOAL_CENTER_Y: float = 34.0
const AWAY_GOAL_POST_TOP_Y: float = 30.34
const AWAY_GOAL_POST_BOTTOM_Y: float = 37.66
#endregion

#region Coordinate Conversion
## 미터 → 픽셀 변환 (Godot 뷰어용)
const PIXELS_PER_METER: float = 10.0

## 픽셀 좌표 (미터 × 10)
const FIELD_LENGTH_PX: float = FIELD_LENGTH_M * PIXELS_PER_METER  ## 1050
const FIELD_WIDTH_PX: float = FIELD_WIDTH_M * PIXELS_PER_METER  ## 680
const GOAL_WIDTH_PX: float = GOAL_WIDTH_M * PIXELS_PER_METER  ## 73.2
const GOAL_HEIGHT_PX: float = GOAL_HEIGHT_M * PIXELS_PER_METER  ## 24.4
#endregion

#region Goal Post Positions (픽셀)
## Home 골대 픽셀 좌표
const HOME_GOAL_X_PX: float = 0.0
const HOME_GOAL_POST_TOP_Y_PX: float = 303.4  ## 30.34 * 10
const HOME_GOAL_POST_BOTTOM_Y_PX: float = 376.6  ## 37.66 * 10

## Away 골대 픽셀 좌표
const AWAY_GOAL_X_PX: float = 1050.0
const AWAY_GOAL_POST_TOP_Y_PX: float = 303.4
const AWAY_GOAL_POST_BOTTOM_Y_PX: float = 376.6
#endregion

#region 3D Height Constants
## 크로스바 높이 (골 판정용)
const CROSSBAR_HEIGHT_M: float = 2.44
const CROSSBAR_HEIGHT_PX: float = 24.4

## 2.5D 높이 표현용 상수
const HEIGHT_TO_Y_OFFSET_RATIO: float = 1.0  ## z 1미터 = y -10픽셀 (위로)
const MAX_DISPLAY_HEIGHT_M: float = 10.0  ## 표시할 최대 높이
#endregion

#region Static Methods


## 미터 좌표를 픽셀 좌표로 변환
static func meters_to_pixels(pos_m: Vector2) -> Vector2:
	return pos_m * PIXELS_PER_METER


## 픽셀 좌표를 미터 좌표로 변환
static func pixels_to_meters(pos_px: Vector2) -> Vector2:
	return pos_px / PIXELS_PER_METER


## 정규화 좌표(0~1)를 미터 좌표로 변환
static func normalized_to_meters(pos_norm: Vector2) -> Vector2:
	return Vector2(pos_norm.x * FIELD_LENGTH_M, pos_norm.y * FIELD_WIDTH_M)


## 미터 좌표를 정규화 좌표로 변환
static func meters_to_normalized(pos_m: Vector2) -> Vector2:
	return Vector2(pos_m.x / FIELD_LENGTH_M, pos_m.y / FIELD_WIDTH_M)


## 3D 위치를 2.5D 화면 위치로 변환
## pos_m: (x, y) 미터 좌표, height_m: z 높이(미터)
## 반환: (screen_x, screen_y) 픽셀 좌표 + shadow_y (그림자 Y 위치)
static func pos_3d_to_screen(pos_m: Vector2, height_m: float) -> Dictionary:
	var screen_pos := meters_to_pixels(pos_m)
	var height_offset := height_m * PIXELS_PER_METER * HEIGHT_TO_Y_OFFSET_RATIO

	return {
		"ball_x": screen_pos.x,
		"ball_y": screen_pos.y - height_offset,  ## 위로 띄움 (y 감소)
		"shadow_x": screen_pos.x,
		"shadow_y": screen_pos.y,  ## 그림자는 바닥 고정
		"height_m": height_m
	}


## 공이 골대 Y 범위 안에 있는지 확인
static func is_in_goal_y_range(y_m: float) -> bool:
	return y_m >= HOME_GOAL_POST_TOP_Y and y_m <= HOME_GOAL_POST_BOTTOM_Y


## 공이 크로스바 아래인지 확인 (골 유효성)
static func is_under_crossbar(height_m: float) -> bool:
	return height_m >= 0.0 and height_m <= CROSSBAR_HEIGHT_M


## 골 판정 (3D)
## 반환: "home" (Away 득점), "away" (Home 득점), "" (골 아님)
static func check_goal_3d(ball_x: float, ball_y: float, ball_height: float) -> String:
	## 크로스바 체크
	if not is_under_crossbar(ball_height):
		return ""

	## 골포스트 체크
	if not is_in_goal_y_range(ball_y):
		return ""

	## 골라인 통과 체크
	if ball_x <= HOME_GOAL_X:
		return "away"  ## Home 골대 통과 = Away 득점
	elif ball_x >= AWAY_GOAL_X:
		return "home"  ## Away 골대 통과 = Home 득점

	return ""


## Home팀의 공격 골대 X좌표 반환
static func attacking_goal_x(is_home: bool) -> float:
	return AWAY_GOAL_X if is_home else HOME_GOAL_X


## Home팀의 수비 골대 X좌표 반환
static func defending_goal_x(is_home: bool) -> float:
	return HOME_GOAL_X if is_home else AWAY_GOAL_X

#endregion
