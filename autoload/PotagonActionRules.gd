extends Node
# PotagonActionRules - SSOT singleton for potagon animation rules
#
# Loads potagon_action_rules.json and provides access to:
# - Hold times for animations
# - Loop rules (6 loop=true, 6 loop=false)
# - Football action → Potagon animation mapping
# - Sequence overrides (tackle_slide, header)
# - Priority order

var rules: Dictionary = {}
var _loaded: bool = false


func _ready():
	_load_rules()


func _load_rules():
	var file = FileAccess.open("res://potagon_action_rules.json", FileAccess.READ)
	if file:
		var json = JSON.new()
		var parse_result = json.parse(file.get_as_text())
		if parse_result == OK:
			rules = json.data
			_loaded = true
			print("[PotagonActionRules] Loaded successfully (version: ", rules.get("version", "unknown"), ")")
		else:
			push_error("[PotagonActionRules] JSON parse error: " + str(json.get_error_message()))
		file.close()
	else:
		push_error("[PotagonActionRules] Failed to open potagon_action_rules.json")


func is_loaded() -> bool:
	return _loaded


func get_hold_time(anim: String) -> float:
	return rules.get("presentation", {}).get("hold_s", {}).get(anim, 0.0)


func get_loop(anim: String) -> bool:
	var loop_true = rules.get("presentation", {}).get("loop_rules", {}).get("loop_true", [])
	return anim in loop_true


func map_action(football_action: String) -> String:
	return rules.get("football_action_mapping", {}).get(football_action, "idle")


func get_sequence(sequence_id: String) -> Array:
	var seq = rules.get("sequence_overrides", {}).get(sequence_id, {})
	return seq.get("steps", [])


func get_priority_order() -> Array:
	return rules.get("presentation", {}).get("priority_order", [])


# Debug helper
func print_all_rules():
	print("[PotagonActionRules] === ALL RULES ===")
	print("Loaded: ", _loaded)
	print("Version: ", rules.get("version", "unknown"))
	print("\nLoop Rules:")
	print("  loop_true: ", rules.get("presentation", {}).get("loop_rules", {}).get("loop_true", []))
	print("  loop_false: ", rules.get("presentation", {}).get("loop_rules", {}).get("loop_false", []))
	print("\nHold Times:")
	for anim in rules.get("presentation", {}).get("hold_s", {}).keys():
		print("  ", anim, ": ", get_hold_time(anim), "s")
	print("\nPriority Order:")
	print("  ", get_priority_order())
	print("\nSequences:")
	for seq_id in rules.get("sequence_overrides", {}).keys():
		print("  ", seq_id, ": ", get_sequence(seq_id).size(), " steps")
	print("========================")
