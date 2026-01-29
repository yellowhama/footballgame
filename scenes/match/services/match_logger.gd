# Scene-local logger capturing events for timeline/telemetry.

extends Node
class_name MatchLogger

var events: Array = []
var started: bool = false

signal logged(event: Dictionary)


func start() -> void:
	started = true
	events.clear()
	print("[MatchLogger] start")


func stop() -> void:
	started = false
	print("[MatchLogger] stop; total=", events.size())


func log(event: Dictionary) -> void:
	if not started:
		return
	events.append(event)
	emit_signal("logged", event)


func save_snapshot(path: String = "user://last_match_events.json") -> void:
	var f := FileAccess.open(path, FileAccess.WRITE)
	f.store_string(JSON.stringify({"events": events}, "\t"))
	f.close()
	print("[MatchLogger] snapshot written:", path)
