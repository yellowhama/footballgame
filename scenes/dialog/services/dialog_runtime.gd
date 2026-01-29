# Minimal dialog runtime (scene-local). Integrate Dialogic later if needed.

extends Node
class_name DialogRuntime

signal line(text: String)

var _lines: Array[String] = []
var _idx := -1


func load_script(lines: Array[String]) -> void:
	_lines = lines
	_idx = -1


func next() -> void:
	_idx += 1
	if _idx < _lines.size():
		emit_signal("line", _lines[_idx])
	else:
		emit_signal("line", "<end>")
