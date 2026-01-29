class_name EventParser
extends RefCounted
## ============================================================================
## EventParser - Match Event Parsing and Augmentation
## ============================================================================
##
## PURPOSE: Parse and augment raw match events from Rust engine
##
## EXTRACTED FROM: MatchSimulationManager.gd (ST-005 God Class refactoring)
##
## RESPONSIBILITIES:
## - Parse position/vector data from various formats
## - Augment pass, shot, run, dribble, header events with derived data
## - Format communication labels
## - Infer boundary labels from positions
##
## USAGE:
##   var parser := EventParser.new()
##   parser.augment_pass_like_event(event)
##   parser.augment_shot_event(event)
## ============================================================================

## Field dimensions (meters) - standard football pitch
const FIELD_LENGTH := 105.0
const FIELD_WIDTH := 68.0


# =============================================================================
# Public Event Parsing Methods
# =============================================================================

## Parse and augment a pass event
func parse_pass_event(event: Dictionary) -> void:
	augment_pass_like_event(event)


## Parse and augment a throughball event
func parse_throughball_event(event: Dictionary) -> void:
	augment_pass_like_event(event)


## Parse and augment a shot event
func parse_shot_event(event: Dictionary) -> void:
	augment_shot_event(event)


## Parse and augment a run event
func parse_run_event(event: Dictionary) -> void:
	augment_run_like_event(event, false)


## Parse and augment a dribble event
func parse_dribble_event(event: Dictionary) -> void:
	augment_run_like_event(event, true)


## Parse and augment a communication event
func parse_communication_event(event: Dictionary) -> void:
	var pos_variant: Variant = event.get("position", event.get("at", null))
	var base_vec: Variant = event_vector_from_point(pos_variant)
	if base_vec is Vector2:
		store_point_on_event(event, "position", base_vec)
	var target_variant: Variant = event.get("communication_target", event.get("target", null))
	var target_vec: Variant = event_vector_from_point(target_variant)
	if target_vec is Vector2:
		store_point_on_event(event, "communication_target", target_vec)
	if (base_vec is Vector2) and (target_vec is Vector2):
		event["communication_distance_m"] = (base_vec as Vector2).distance_to(target_vec)
		event["has_target"] = true
	var message_key := str(event.get("message", event.get("message_key", ""))).strip_edges()
	event["message_key"] = message_key
	if message_key != "":
		event["message_label"] = format_comm_label(message_key)


## Parse and augment a header event
func parse_header_event(event: Dictionary) -> void:
	var origin_variant: Variant = event.get("from", event.get("position", null))
	var origin_vec: Variant = event_vector_from_point(origin_variant)
	if origin_vec is Vector2:
		store_point_on_event(event, "from", origin_vec)
		store_point_on_event(event, "position", origin_vec)
	var direction_variant: Variant = event.get("direction", event.get("direction_vector", null))
	var direction_vec: Vector2 = Vector2.ZERO
	if direction_variant is Dictionary:
		var dict: Dictionary = direction_variant
		direction_vec.x = parse_float(dict.get("x", 0.0))
		direction_vec.y = parse_float(dict.get("y", 0.0))
	if direction_vec != Vector2.ZERO:
		event["direction_vector"] = {"x": direction_vec.x, "y": direction_vec.y}
		if origin_vec is Vector2:
			var adjusted_dir := direction_vec
			if adjusted_dir.length() > 0.001:
				adjusted_dir = adjusted_dir.normalized()
			var heading_target := (origin_vec as Vector2) + adjusted_dir * 6.0
			store_point_on_event(event, "heading_target", heading_target)


## Parse and augment a boundary event (out of play)
func parse_boundary_event(event: Dictionary) -> void:
	var position_variant: Variant = event.get("position", event.get("at", null))
	var pos_vec: Variant = event_vector_from_point(position_variant)
	if pos_vec is Vector2:
		store_point_on_event(event, "position", pos_vec)
	if not event.has("player_id") and event.has("last_touch_player_id"):
		event["player_id"] = int(event.get("last_touch_player_id"))
	if not event.has("team_id") and event.has("last_touch_team_id"):
		event["team_id"] = int(event.get("last_touch_team_id"))
	event["boundary_label"] = event.get("boundary_label", infer_boundary_label(pos_vec))


# =============================================================================
# Event Augmentation Helpers
# =============================================================================

## Augment pass-like events (pass, throughball, cross, etc.)
func augment_pass_like_event(event: Dictionary) -> void:
	var receiver_field = event.get("receiver_id", event.get("to_player_id", event.get("target_player", 0)))
	event["receiver_id"] = int(receiver_field)
	if not event.has("from") and event.has("origin"):
		event["from"] = event.get("origin")
	if not event.has("to") and event.has("target"):
		event["to"] = event.get("target")
	var distance_variant = event.get("distance_m", event.get("distance", null))
	if distance_variant != null:
		event["pass_distance_m"] = float(distance_variant)
	var force_variant = event.get("force", event.get("pass_force", null))
	if force_variant != null:
		event["pass_force"] = float(force_variant)
	event["is_clearance"] = bool(event.get("is_clearance", false))


## Augment shot events with xG and ball state
func augment_shot_event(event: Dictionary) -> void:
	var xg_variant = event.get("xg", event.get("xg_value", null))
	if xg_variant != null:
		event["xg"] = float(xg_variant)
	if not event.has("target"):
		var ball_dict: Variant = event.get("ball", null)
		if ball_dict is Dictionary and ball_dict.has("to"):
			event["target"] = ball_dict.get("to")
	var ball_state: Variant = event.get("ball", null)
	if ball_state is Dictionary:
		event["ball_speed"] = float(ball_state.get("speed_mps", ball_state.get("speed", 0.0)))
		if ball_state.has("curve"):
			event["ball_curve"] = str(ball_state.get("curve"))


## Augment run-like events (run, dribble, sprint)
func augment_run_like_event(event: Dictionary, is_dribble: bool) -> void:
	var from_vec: Variant = event_vector_from_point(event.get("from", event.get("origin", null)))
	var to_vec: Variant = event_vector_from_point(event.get("to", event.get("target", null)))
	if from_vec is Vector2 and not (event.get("from") is Dictionary):
		store_point_on_event(event, "from", from_vec as Vector2)
	if to_vec is Vector2 and not (event.get("to") is Dictionary):
		store_point_on_event(event, "to", to_vec as Vector2)

	var computed_distance := -1.0
	if (from_vec is Vector2) and (to_vec is Vector2):
		computed_distance = (from_vec as Vector2).distance_to(to_vec)
	var explicit_distance: Variant = event.get(
		"segment_distance_m", event.get("distance_m", event.get("distance", null))
	)
	if explicit_distance != null:
		var explicit_val := parse_float(explicit_distance)
		if explicit_val > 0.0:
			computed_distance = explicit_val
	if computed_distance > 0.0:
		event["segment_distance_m"] = computed_distance
		if is_dribble:
			event["dribble_distance_m"] = computed_distance
		else:
			event["run_distance_m"] = computed_distance

	var speed_val := parse_float(event.get("speed_mps", event.get("speed", null)))
	if speed_val <= 0.0 and computed_distance > 0.0:
		var duration_val := extract_duration_seconds(event)
		if duration_val > 0.0:
			speed_val = computed_distance / duration_val
	if speed_val > 0.0:
		event["speed_mps"] = speed_val

	if event.has("with_ball"):
		event["with_ball"] = bool(event.get("with_ball"))
	elif is_dribble:
		event["with_ball"] = true

	var touches_variant: Variant = event.get("touches", event.get("dribble_touches", event.get("touch_count", null)))
	if touches_variant != null:
		var touches_value := int(parse_float(touches_variant))
		event["touches"] = touches_value
		if is_dribble:
			event["dribble_touches"] = touches_value


# =============================================================================
# Utility Functions
# =============================================================================

## Extract duration in seconds from event (supports various key formats)
func extract_duration_seconds(event: Dictionary) -> float:
	var direct_keys := ["duration_s", "duration_sec", "duration_seconds", "duration"]
	for key in direct_keys:
		if event.has(key):
			var value := parse_float(event.get(key))
			if value > 0.0:
				return value
	if event.has("duration_ms"):
		var duration_ms := parse_float(event.get("duration_ms"))
		if duration_ms > 0.0:
			return duration_ms / 1000.0
	return 0.0


## Convert various point formats to Vector2
func event_vector_from_point(value: Variant) -> Variant:
	if value == null:
		return null
	if value is Vector2:
		return value
	if value is Dictionary:
		var dict: Dictionary = value
		var x_val := parse_float(dict.get("x", dict.get("0", 0.0)))
		var y_val := parse_float(dict.get("y", dict.get("1", dict.get("z", 0.0))))
		return Vector2(x_val, y_val)
	if value is PackedFloat32Array:
		var arr32: PackedFloat32Array = value
		if arr32.is_empty():
			return null
		var x32 := float(arr32[0])
		var y32 := 0.0
		if arr32.size() > 2:
			y32 = float(arr32[2])
		elif arr32.size() > 1:
			y32 = float(arr32[1])
		return Vector2(x32, y32)
	if value is PackedFloat64Array:
		var arr64: PackedFloat64Array = value
		if arr64.is_empty():
			return null
		var x64 := float(arr64[0])
		var y64 := 0.0
		if arr64.size() > 2:
			y64 = float(arr64[2])
		elif arr64.size() > 1:
			y64 = float(arr64[1])
		return Vector2(x64, y64)
	if value is Array:
		var arr: Array = value
		if arr.is_empty():
			return null
		var x_arr := float(arr[0])
		var y_arr := 0.0
		if arr.size() > 2:
			y_arr = float(arr[2])
		elif arr.size() > 1:
			y_arr = float(arr[1])
		return Vector2(x_arr, y_arr)
	return null


## Store a Vector2 point on event as dictionary format
func store_point_on_event(event: Dictionary, key: String, vec: Vector2) -> void:
	event[key] = {"x": vec.x, "y": vec.y}


## Format communication message key to display label
func format_comm_label(key: String) -> String:
	var cleaned := key.strip_edges()
	if cleaned == "":
		return "CALL"
	cleaned = cleaned.replace("_", " ")
	return cleaned.capitalize()


## Infer boundary label from position (corner, goal kick, throw-in)
func infer_boundary_label(pos_variant: Variant) -> String:
	if not (pos_variant is Vector2):
		return "OUT"
	var pos_vec: Vector2 = pos_variant
	var near_goal_line := pos_vec.x <= 2.0 or pos_vec.x >= (FIELD_LENGTH - 2.0)
	var near_touch_line := pos_vec.y <= 1.5 or pos_vec.y >= (FIELD_WIDTH - 1.5)
	if near_goal_line and near_touch_line:
		return "CORNER"
	if near_goal_line:
		return "GOAL KICK"
	if near_touch_line:
		return "THROW-IN"
	return "OUT"


## Parse various numeric formats to float
func parse_float(value: Variant) -> float:
	if value == null:
		return 0.0
	if value is float:
		return value
	if value is int:
		return float(value)
	if value is String:
		var stripped := (value as String).strip_edges()
		if stripped.is_valid_float():
			return stripped.to_float()
		if stripped.is_valid_int():
			return float(stripped.to_int())
	return 0.0
