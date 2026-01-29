extends Node
## Match timeline viewer data handoff (temporary holder)
## Autoload: MatchTimelineHolder

var timeline_record: Dictionary = {}
var return_scene_path: String = ""

## Optional pending clip request (set by PostMatch before switching into viewer).
## The viewer consumes and clears these values to avoid stale replays.
var pending_clip_ms: int = 0
var pending_autoplay: bool = false


func set_timeline_data(record: Dictionary, return_path: String = "") -> void:
	timeline_record = record.duplicate(true)
	return_scene_path = return_path


func get_timeline_data() -> Dictionary:
	return timeline_record


func get_return_scene() -> String:
	return return_scene_path


func clear() -> void:
	timeline_record = {}
	return_scene_path = ""
	clear_pending_clip()

func clear_pending_clip() -> void:
	pending_clip_ms = 0
	pending_autoplay = false
