extends Node

const VIEWER_SCENE := preload("res://scenes/ui/MatchTimelinePanel.tscn")
const RECORD_PATH := "res://data/qa/minimap_phase_d_record.json"


func _ready() -> void:
	# Only run if RUN_PHASE_D_QA environment variable is set
	if OS.get_environment("RUN_PHASE_D_QA") != "1":
		return
	call_deferred("_run_qa_export")


func _run_qa_export() -> void:
	print("[PhaseD][QA] runner start")
	ProjectSettings.set_setting("debug/timeline/enable_minimap_phase_d_qa_logs", true)
	ProjectSettings.set_setting("debug/timeline/capture_minimap_phase_d_screenshot", true)
	var record: Dictionary = _load_record()
	if record.is_empty():
		push_error("[PhaseD][QA] Failed to parse match record: %s" % RECORD_PATH)
		get_tree().quit()
		return
	var viewer_scene: PackedScene = VIEWER_SCENE
	if not viewer_scene:
		push_error("[PhaseD][QA] Timeline panel scene missing")
		get_tree().quit()
		return
	var viewer: Node = viewer_scene.instantiate()
	get_tree().root.add_child(viewer)
	await get_tree().process_frame
	print("[PhaseD][QA] viewer ready")
	if viewer.has_method("set_status_message"):
		viewer.call("set_status_message", "Phase D QA automation")
	if viewer.has_method("set_match_record"):
		viewer.call("set_match_record", record)
	if viewer.has_method("set_position_payload"):
		viewer.call("set_position_payload", {})
	print("[PhaseD][QA] record applied, waiting for screenshot")
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().create_timer(1.0).timeout
	print("[PhaseD][QA] Export complete – check user://qa_logs for outputs")
	get_tree().quit()


func _load_record() -> Dictionary:
	var file: FileAccess = FileAccess.open(RECORD_PATH, FileAccess.READ)
	if file == null:
		push_error("[PhaseD][QA] Cannot open record: %s" % RECORD_PATH)
		return {}
	var text: String = file.get_as_text()
	file.close()
	var parsed: Variant = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		push_warning("[PhaseD][QA] Replay fixture is not a dictionary")
		return {}
	return parsed
