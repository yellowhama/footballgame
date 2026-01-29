class_name TacticalPresetDB
extends Resource

@export var preset_files: Array[String] = [
	"res://data/tactics/presets/high_press_433.tactic.json", "res://data/tactics/presets/low_block_532.tactic.json"
]

var presets: Dictionary = {}  # preset_id -> data


func _init():
	load_presets()


func load_presets() -> void:
	presets.clear()
	for path in preset_files:
		var data = _load_preset(path)
		if data.is_empty():
			continue
		var preset_id := String(data.get("id", ""))
		if preset_id.is_empty():
			continue
		presets[preset_id] = data


func reload() -> void:
	load_presets()


func list_presets() -> Array:
	return presets.keys()


func get_preset(preset_id: String) -> Dictionary:
	return presets.get(preset_id, {}).duplicate(true)


func _load_preset(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		push_warning("[TacticalPresetDB] Missing preset file: %s" % path)
		return {}
	var file := FileAccess.open(path, FileAccess.READ)
	if not file:
		push_warning("[TacticalPresetDB] Cannot open preset: %s" % path)
		return {}
	var json_text := file.get_as_text()
	file.close()
	var json := JSON.new()
	var parse_result := json.parse(json_text)
	if parse_result != OK:
		push_warning("[TacticalPresetDB] JSON parse error in %s: %s" % [path, json.get_error_message()])
		return {}
	return json.data
