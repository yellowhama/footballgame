extends Node

signal training_state_changed(state)

var last_result: Dictionary = {}


func set_last_result(result: Dictionary) -> void:
	last_result = result.duplicate(true)
	emit_signal("training_state_changed", last_result)


func get_last_result() -> Dictionary:
	return last_result.duplicate(true)


func has_last_result() -> bool:
	return not last_result.is_empty()


func clear() -> void:
	last_result = {}
	emit_signal("training_state_changed", last_result)
