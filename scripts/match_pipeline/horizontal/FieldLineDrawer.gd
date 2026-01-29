extends Node2D
class_name FieldLineDrawer
##
## FieldLineDrawer - 필드 라인을 _draw()로 직접 그리는 컴포넌트
##
## **ARCHITECTURE UPDATE (2025-12-19)**:
## - Now uses FieldSpec as SSOT for field geometry (Game OS v1.1)
## - FieldSpec = MatchOS SSOT (field coordinates/dimensions)
## - Eliminates coordinate constant duplication across viewers
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
## 참조: docs/spec+@/spec_v4/reference/FOOTBALL_PITCH_DIMENSIONS.md
## FieldSpec: docs/specs/SSOT/FIELD_SPEC.md v1.3
##
## 좌표계:
## - 미터 좌표 (FieldSpec): 필드 105m × 68m, Home 골대 x=0, Away 골대 x=105
## - 픽셀 좌표 (여기): 필드 1050px × 680px (×10 스케일)
## - 변환: pixels = meters × FieldSpec.PIXELS_PER_METER
##

#region Constants - FieldSpec SSOT 참조
## Conversion factor (inherited from FieldSpec)
const METER_TO_PIXEL: float = FieldSpec.PIXELS_PER_METER  # 10.0

## Line rendering properties
const LINE_COLOR: Color = Color.WHITE
const LINE_WIDTH: float = 3.5

## Field dimensions (from FieldSpec SSOT)
const FIELD_LENGTH_M: float = FieldSpec.FIELD_LENGTH_M  # 105.0
const FIELD_WIDTH_M: float = FieldSpec.FIELD_WIDTH_M  # 68.0
const FIELD_LENGTH_PX: float = FIELD_LENGTH_M * METER_TO_PIXEL  # 1050
const FIELD_WIDTH_PX: float = FIELD_WIDTH_M * METER_TO_PIXEL  # 680

## Center circle (FIFA standard - from FieldSpec)
const CENTER_CIRCLE_RADIUS_M: float = FieldSpec.CENTER_CIRCLE_RADIUS_M  # 9.15
const CENTER_CIRCLE_RADIUS_PX: float = CENTER_CIRCLE_RADIUS_M * METER_TO_PIXEL

## Penalty area (FIFA standard - from FieldSpec)
const PENALTY_AREA_LENGTH_M: float = FieldSpec.PENALTY_AREA_LENGTH_M  # 16.5
const PENALTY_AREA_WIDTH_M: float = FieldSpec.PENALTY_AREA_WIDTH_M  # 40.32
const PENALTY_AREA_LENGTH_PX: float = PENALTY_AREA_LENGTH_M * METER_TO_PIXEL
const PENALTY_AREA_WIDTH_PX: float = PENALTY_AREA_WIDTH_M * METER_TO_PIXEL

## Goal area (6-yard box - from FieldSpec)
const GOAL_AREA_LENGTH_M: float = FieldSpec.GOAL_AREA_LENGTH_M  # 5.5
const GOAL_AREA_WIDTH_M: float = FieldSpec.GOAL_AREA_WIDTH_M  # 18.32
const GOAL_AREA_LENGTH_PX: float = GOAL_AREA_LENGTH_M * METER_TO_PIXEL
const GOAL_AREA_WIDTH_PX: float = GOAL_AREA_WIDTH_M * METER_TO_PIXEL

## Penalty mark (from FieldSpec)
const PENALTY_MARK_DISTANCE_M: float = FieldSpec.PENALTY_MARK_DISTANCE_M  # 11.0
const PENALTY_MARK_DISTANCE_PX: float = PENALTY_MARK_DISTANCE_M * METER_TO_PIXEL

## Corner arc (from FieldSpec)
const CORNER_ARC_RADIUS_M: float = FieldSpec.CORNER_ARC_RADIUS_M  # 1.0
const CORNER_ARC_RADIUS_PX: float = CORNER_ARC_RADIUS_M * METER_TO_PIXEL

## Goal dimensions (from FieldSpec)
const GOAL_WIDTH_M: float = FieldSpec.GOAL_WIDTH_M  # 7.32
const GOAL_HEIGHT_M: float = FieldSpec.GOAL_HEIGHT_M  # 2.44 (crossbar)
const GOAL_WIDTH_PX: float = GOAL_WIDTH_M * METER_TO_PIXEL
const GOAL_POST_TOP_Y_PX: float = FieldSpec.GOAL_POST_TOP_Y * METER_TO_PIXEL  # 303.4
const GOAL_POST_BOTTOM_Y_PX: float = FieldSpec.GOAL_POST_BOTTOM_Y * METER_TO_PIXEL  # 376.6
#endregion


func _draw() -> void:
	_draw_outer_lines()
	_draw_halfway_line()
	_draw_center_circle()
	_draw_penalty_areas()
	_draw_goal_areas()
	_draw_penalty_marks()
	_draw_penalty_arcs()
	_draw_corner_arcs()


func _draw_outer_lines() -> void:
	## 외곽선 (터치 라인 + 골 라인)
	var rect := Rect2(0, 0, FIELD_LENGTH_PX, FIELD_WIDTH_PX)
	draw_rect(rect, LINE_COLOR, false, LINE_WIDTH)


func _draw_halfway_line() -> void:
	## 중앙선
	var center_x := FIELD_LENGTH_PX / 2
	draw_line(Vector2(center_x, 0), Vector2(center_x, FIELD_WIDTH_PX), LINE_COLOR, LINE_WIDTH)


func _draw_center_circle() -> void:
	## 센터 서클
	var center := Vector2(FIELD_LENGTH_PX / 2, FIELD_WIDTH_PX / 2)
	draw_arc(center, CENTER_CIRCLE_RADIUS_PX, 0, TAU, 64, LINE_COLOR, LINE_WIDTH)

	## 센터 마크
	draw_circle(center, 3.0, LINE_COLOR)


func _draw_penalty_areas() -> void:
	## 왼쪽 (홈) 페널티 에어리어
	var left_pa_y := (FIELD_WIDTH_PX - PENALTY_AREA_WIDTH_PX) / 2
	var left_pa_rect := Rect2(0, left_pa_y, PENALTY_AREA_LENGTH_PX, PENALTY_AREA_WIDTH_PX)
	draw_rect(left_pa_rect, LINE_COLOR, false, LINE_WIDTH)

	## 오른쪽 (어웨이) 페널티 에어리어
	var right_pa_x := FIELD_LENGTH_PX - PENALTY_AREA_LENGTH_PX
	var right_pa_rect := Rect2(right_pa_x, left_pa_y, PENALTY_AREA_LENGTH_PX, PENALTY_AREA_WIDTH_PX)
	draw_rect(right_pa_rect, LINE_COLOR, false, LINE_WIDTH)


func _draw_goal_areas() -> void:
	## 왼쪽 (홈) 골 에어리어
	var left_ga_y := (FIELD_WIDTH_PX - GOAL_AREA_WIDTH_PX) / 2
	var left_ga_rect := Rect2(0, left_ga_y, GOAL_AREA_LENGTH_PX, GOAL_AREA_WIDTH_PX)
	draw_rect(left_ga_rect, LINE_COLOR, false, LINE_WIDTH)

	## 오른쪽 (어웨이) 골 에어리어
	var right_ga_x := FIELD_LENGTH_PX - GOAL_AREA_LENGTH_PX
	var right_ga_rect := Rect2(right_ga_x, left_ga_y, GOAL_AREA_LENGTH_PX, GOAL_AREA_WIDTH_PX)
	draw_rect(right_ga_rect, LINE_COLOR, false, LINE_WIDTH)


func _draw_penalty_marks() -> void:
	## 왼쪽 페널티 마크
	var left_mark := Vector2(PENALTY_MARK_DISTANCE_PX, FIELD_WIDTH_PX / 2)
	draw_circle(left_mark, 3.0, LINE_COLOR)

	## 오른쪽 페널티 마크
	var right_mark := Vector2(FIELD_LENGTH_PX - PENALTY_MARK_DISTANCE_PX, FIELD_WIDTH_PX / 2)
	draw_circle(right_mark, 3.0, LINE_COLOR)


func _draw_penalty_arcs() -> void:
	## 왼쪽 페널티 아크 (페널티 박스 밖 반원)
	var left_mark := Vector2(PENALTY_MARK_DISTANCE_PX, FIELD_WIDTH_PX / 2)
	var arc_start := -acos(
		PENALTY_AREA_LENGTH_PX / CENTER_CIRCLE_RADIUS_PX - PENALTY_MARK_DISTANCE_PX / CENTER_CIRCLE_RADIUS_PX
	)
	## 페널티 박스 바깥 부분만 그리기 위해 각도 계산
	## 아크 중심: 페널티 마크, 반지름: 9.15m
	## 페널티 박스 끝 x좌표: 16.5m
	## 아크가 박스 밖으로 나가는 각도 계산
	var box_edge_x := PENALTY_AREA_LENGTH_PX
	var arc_angle := acos((box_edge_x - PENALTY_MARK_DISTANCE_PX) / CENTER_CIRCLE_RADIUS_PX)
	draw_arc(left_mark, CENTER_CIRCLE_RADIUS_PX, -arc_angle, arc_angle, 32, LINE_COLOR, LINE_WIDTH)

	## 오른쪽 페널티 아크
	var right_mark := Vector2(FIELD_LENGTH_PX - PENALTY_MARK_DISTANCE_PX, FIELD_WIDTH_PX / 2)
	draw_arc(right_mark, CENTER_CIRCLE_RADIUS_PX, PI - arc_angle, PI + arc_angle, 32, LINE_COLOR, LINE_WIDTH)


func _draw_corner_arcs() -> void:
	## 4개 코너 아크
	## 좌상단 (0, 0)
	draw_arc(Vector2.ZERO, CORNER_ARC_RADIUS_PX, 0, PI / 2, 16, LINE_COLOR, LINE_WIDTH)

	## 좌하단 (0, 680)
	draw_arc(Vector2(0, FIELD_WIDTH_PX), CORNER_ARC_RADIUS_PX, -PI / 2, 0, 16, LINE_COLOR, LINE_WIDTH)

	## 우상단 (1050, 0)
	draw_arc(Vector2(FIELD_LENGTH_PX, 0), CORNER_ARC_RADIUS_PX, PI / 2, PI, 16, LINE_COLOR, LINE_WIDTH)

	## 우하단 (1050, 680)
	draw_arc(Vector2(FIELD_LENGTH_PX, FIELD_WIDTH_PX), CORNER_ARC_RADIUS_PX, PI, PI * 1.5, 16, LINE_COLOR, LINE_WIDTH)
