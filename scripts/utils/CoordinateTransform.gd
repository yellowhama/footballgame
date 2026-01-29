extends Node
class_name CoordinateTransform

# UI-only coordinate conversions (meters <-> normalized).


static func _clamp01(value: float) -> float:
	return clamp(value, 0.0, 1.0)


static func meters_to_normalized(pos_m: Vector2) -> Vector2:
	return Vector2(_clamp01(pos_m.x / FieldSpec.FIELD_LENGTH_M), _clamp01(pos_m.y / FieldSpec.FIELD_WIDTH_M))


static func normalized_to_meters(pos_norm: Vector2) -> Vector2:
	return Vector2(_clamp01(pos_norm.x) * FieldSpec.FIELD_LENGTH_M, _clamp01(pos_norm.y) * FieldSpec.FIELD_WIDTH_M)
