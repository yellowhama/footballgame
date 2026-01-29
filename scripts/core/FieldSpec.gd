extends Node
class_name FieldSpec
## Field Coordinate System Specification (FieldSpec)
## Part of Game OS v1.1 - Coordinate System SSOT
## Created: 2025-12-19
##
## PURPOSE:
## Single source of truth for all field geometry and coordinates.
## Prevents coordinate system bugs across UI/minimap/tactical board/replay.
##
## ARCHITECTURE:
## - FieldSpec = MatchOS SSOT (field geometry/lines/coordinates)
## - Engine = Event SSOT (goal/out/offside/foul judgements)
## - OS = Validator + Overlay (verification + visualization)
##
## REFERENCES:
## - Spec: docs/specs/SSOT/FIELD_SPEC.md v1.3
## - GoalContract: scripts/match_pipeline/horizontal/GoalContract.gd (verified constants)
## - FieldLineDrawer: scripts/match_pipeline/horizontal/FieldLineDrawer.gd (implementation)

## ============================================================================
## FIELD DIMENSIONS (FIFA Standard)
## ============================================================================

## Field length in meters (FIFA standard: 105m)
const FIELD_LENGTH_M: float = 105.0

## Field width in meters (FIFA standard: 68m)
const FIELD_WIDTH_M: float = 68.0

## Field center X coordinate
const FIELD_CENTER_X: float = 52.5  # 105 / 2

## Field center Y coordinate
const FIELD_CENTER_Y: float = 34.0  # 68 / 2

## ============================================================================
## GOAL DIMENSIONS (FIFA Standard)
## ============================================================================

## Goal width in meters (FIFA standard: 7.32m)
const GOAL_WIDTH_M: float = 7.32

## Goal height in meters (FIFA standard: 2.44m)
const GOAL_HEIGHT_M: float = 2.44

## Goal half-width (3.66m)
const GOAL_HALF_WIDTH: float = 3.66  # 7.32 / 2

## Home goal X position (left side)
const HOME_GOAL_X: float = 0.0

## Away goal X position (right side)
const AWAY_GOAL_X: float = 105.0

## Goal post top Y (center - half-width)
const GOAL_POST_TOP_Y: float = 30.34  # 34.0 - 3.66

## Goal post bottom Y (center + half-width)
const GOAL_POST_BOTTOM_Y: float = 37.66  # 34.0 + 3.66

## ============================================================================
## PENALTY AREA (FIFA Standard)
## ============================================================================

## Penalty area length from goal line (FIFA standard: 16.5m)
const PENALTY_AREA_LENGTH_M: float = 16.5

## Penalty area width (FIFA standard: 40.32m)
const PENALTY_AREA_WIDTH_M: float = 40.32

## Penalty area Y offset from touchline
const PENALTY_AREA_Y_OFFSET: float = 13.84  # (68 - 40.32) / 2

## ============================================================================
## GOAL AREA (6-yard box)
## ============================================================================

## Goal area length from goal line (FIFA standard: 5.5m)
const GOAL_AREA_LENGTH_M: float = 5.5

## Goal area width (FIFA standard: 18.32m)
const GOAL_AREA_WIDTH_M: float = 18.32

## Goal area Y offset from touchline
const GOAL_AREA_Y_OFFSET: float = 24.84  # (68 - 18.32) / 2

## ============================================================================
## CENTER CIRCLE & MARKS
## ============================================================================

## Center circle radius (FIFA standard: 9.15m)
const CENTER_CIRCLE_RADIUS_M: float = 9.15

## Penalty mark distance from goal line (FIFA standard: 11.0m)
const PENALTY_MARK_DISTANCE_M: float = 11.0

## Corner arc radius (FIFA standard: 1.0m)
const CORNER_ARC_RADIUS_M: float = 1.0

## ============================================================================
## 2.5D Z-AXIS CONSTANTS (Phase Z)
## ============================================================================

## Ground level (z = 0)
const GROUND_Z: float = 0.0

## Ball radius in meters (FIFA standard: 11cm)
const BALL_RADIUS_M: float = 0.11

## Goal crossbar height (FIFA standard: 2.44m)
const GOAL_CROSSBAR_Z_M: float = 2.44

## Average player height for visualization (1.75m)
const PLAYER_HEIGHT_M: float = 1.75

## Player jump max height for visualization (0.60m)
const PLAYER_JUMP_Z_M: float = 0.60

## Header jump max height (higher than regular jump, 0.75m)
const HEADER_JUMP_Z_M: float = 0.75

## ============================================================================
## COORDINATE CONVERSION
## ============================================================================

## Pixels per meter (1m = 10px)
const PIXELS_PER_METER: float = 10.0

## Z-index sort factor (for 2.5D draw order)
const Z_SORT_FACTOR: float = 10.0  # Same as PIXELS_PER_METER

## ============================================================================
## STATIC HELPER METHODS
## ============================================================================


## Get field center point (meters)
static func center() -> Vector2:
	return Vector2(FIELD_CENTER_X, FIELD_CENTER_Y)


## Get field bounds rectangle (meters)
static func bounds() -> Rect2:
	return Rect2(0, 0, FIELD_LENGTH_M, FIELD_WIDTH_M)


## Get home goal position (meters)
static func home_goal_position() -> Vector2:
	return Vector2(HOME_GOAL_X, FIELD_CENTER_Y)


## Get away goal position (meters)
static func away_goal_position() -> Vector2:
	return Vector2(AWAY_GOAL_X, FIELD_CENTER_Y)


## Get home goal post Y range (meters)
static func home_goal_post_y_range() -> Vector2:
	return Vector2(GOAL_POST_TOP_Y, GOAL_POST_BOTTOM_Y)


## Get away goal post Y range (meters)
static func away_goal_post_y_range() -> Vector2:
	return Vector2(GOAL_POST_TOP_Y, GOAL_POST_BOTTOM_Y)


## Get home penalty area bounds (meters)
static func home_penalty_area() -> Rect2:
	return Rect2(0, PENALTY_AREA_Y_OFFSET, PENALTY_AREA_LENGTH_M, PENALTY_AREA_WIDTH_M)


## Get away penalty area bounds (meters)
static func away_penalty_area() -> Rect2:
	return Rect2(
		FIELD_LENGTH_M - PENALTY_AREA_LENGTH_M, PENALTY_AREA_Y_OFFSET, PENALTY_AREA_LENGTH_M, PENALTY_AREA_WIDTH_M
	)


## Get home goal area bounds (meters)
static func home_goal_area() -> Rect2:
	return Rect2(0, GOAL_AREA_Y_OFFSET, GOAL_AREA_LENGTH_M, GOAL_AREA_WIDTH_M)


## Get away goal area bounds (meters)
static func away_goal_area() -> Rect2:
	return Rect2(FIELD_LENGTH_M - GOAL_AREA_LENGTH_M, GOAL_AREA_Y_OFFSET, GOAL_AREA_LENGTH_M, GOAL_AREA_WIDTH_M)


## Get home penalty mark position (meters)
static func home_penalty_mark() -> Vector2:
	return Vector2(PENALTY_MARK_DISTANCE_M, FIELD_CENTER_Y)


## Get away penalty mark position (meters)
static func away_penalty_mark() -> Vector2:
	return Vector2(FIELD_LENGTH_M - PENALTY_MARK_DISTANCE_M, FIELD_CENTER_Y)


## Convert meters to pixels
static func meters_to_pixels(pos_m: Vector2) -> Vector2:
	return pos_m * PIXELS_PER_METER


## Convert pixels to meters
static func pixels_to_meters(pos_px: Vector2) -> Vector2:
	return pos_px / PIXELS_PER_METER


## Calculate z-index for 2.5D draw order (screen_y + z * Z_SORT_FACTOR)
static func calculate_z_index(screen_y: float, z_m: float) -> int:
	return int(screen_y + (z_m * Z_SORT_FACTOR))


## Check if position is within field bounds (meters)
static func is_in_bounds(pos_m: Vector2) -> bool:
	return pos_m.x >= 0.0 and pos_m.x <= FIELD_LENGTH_M and pos_m.y >= 0.0 and pos_m.y <= FIELD_WIDTH_M


## Check if position is in home penalty area (meters)
static func is_in_home_penalty_area(pos_m: Vector2) -> bool:
	return home_penalty_area().has_point(pos_m)


## Check if position is in away penalty area (meters)
static func is_in_away_penalty_area(pos_m: Vector2) -> bool:
	return away_penalty_area().has_point(pos_m)
