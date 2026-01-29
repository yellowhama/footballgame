extends RefCounted

class_name FieldDimensions

## LEGACY: 840×545 "engine units" scaling helper.
##
## - Use `FieldSpec` for meters SSOT (105×68).
## - Keep this only for backwards-compat UI conversions.
## - FIX02 contract forbids treating ENGINE_* as meters.

const ENGINE_LENGTH := 840.0
const ENGINE_WIDTH := 545.0

const REAL_LENGTH := 105.0
const REAL_WIDTH := 68.0

const LENGTH_SCALE := ENGINE_LENGTH / REAL_LENGTH
const WIDTH_SCALE := ENGINE_WIDTH / REAL_WIDTH


static func to_engine_x(meters: float) -> float:
	return meters * LENGTH_SCALE


static func to_engine_y(meters: float) -> float:
	return meters * WIDTH_SCALE


static func to_real_x(engine_units: float) -> float:
	return engine_units / LENGTH_SCALE


static func to_real_y(engine_units: float) -> float:
	return engine_units / WIDTH_SCALE


static func clamp_engine_x(value: float) -> float:
	return clamp(value, 0.0, ENGINE_LENGTH)


static func clamp_engine_y(value: float) -> float:
	return clamp(value, 0.0, ENGINE_WIDTH)


static func normalize_x(value: float) -> float:
	return clamp(value / ENGINE_LENGTH, 0.0, 1.0)


static func normalize_y(value: float) -> float:
	return clamp(value / ENGINE_WIDTH, 0.0, 1.0)
