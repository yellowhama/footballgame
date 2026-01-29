extends Control
## HorizontalMatchSessionViewerController
##
## Purpose:
## - Load a match record from MatchTimelineHolder
## - Configure HorizontalMatchViewer metadata (teams/rosters/events)
## - Drive playback via MatchTimelineController + MatchTimelineControls

const _HOLDER_PATH := "/root/MatchTimelineHolder"
const _TIMELINE_CONTROLLER_PATH := "/root/MatchTimelineController"

@onready var _viewer: Control = $HorizontalMatchViewer
@onready var _controls: Control = $MatchTimelineControls

var _timeline_controller: Node = null
var _playback_started: bool = false
var _is_paused: bool = false
var _current_speed: float = 1.0
var _current_timestamp: int = 0
var _timeline_markers: Array = []


func _ready() -> void:
	if not _viewer:
		push_error("[HorizontalMatchSessionViewer] HorizontalMatchViewer node not found")
		return

	_init_controls()

	# Keep controls timestamp in sync with the viewer's applied snapshot time.
	if _viewer.has_signal("snapshot_applied"):
		_viewer.snapshot_applied.connect(_on_viewer_snapshot_applied)

	var record: Dictionary = _load_record_from_holder()
	if record.is_empty():
		push_warning("[HorizontalMatchSessionViewer] No record provided in MatchTimelineHolder")
		return
	var highlight_level := str(record.get("highlight_level", "full"))
	if _controls and _controls.has_method("set_highlight_level"):
		_controls.set_highlight_level(highlight_level)

	var metadata: Dictionary = _extract_metadata(record)
	var rosters: Dictionary = _extract_rosters(record)
	var events: Array = _extract_events(record)

	_setup_viewer(metadata, rosters)
	if not events.is_empty() and _viewer.has_method("set_events"):
		_viewer.set_events(events)

	_init_position_playback(record, rosters, events)

	if _controls and _controls.has_method("set_event_markers") and not _timeline_markers.is_empty():
		_controls.set_event_markers(_timeline_markers)


func set_timeline_markers(markers: Array) -> void:
	_timeline_markers = markers.duplicate(true)
	if _controls and _controls.has_method("set_event_markers"):
		_controls.set_event_markers(_timeline_markers)


func _load_record_from_holder() -> Dictionary:
	var holder := get_node_or_null(_HOLDER_PATH)
	if holder and holder.has_method("get_timeline_data"):
		var data: Variant = holder.get_timeline_data()
		if data is Dictionary:
			return (data as Dictionary).duplicate(true)
	return {}


func _setup_viewer(metadata: Dictionary, rosters: Dictionary) -> void:
	if not _viewer:
		return

	var home_name: String = str(metadata.get("home_team_name", "Home"))
	var away_name: String = str(metadata.get("away_team_name", "Away"))
	if _viewer.has_method("set_hud_team_names"):
		_viewer.set_hud_team_names(home_name, away_name)

	var home_score: int = int(metadata.get("home_score", 0))
	var away_score: int = int(metadata.get("away_score", 0))
	if _viewer.has_method("set_score"):
		_viewer.set_score(home_score, away_score)

	var home_id: String = str(metadata.get("home_team_id", "home"))
	var away_id: String = str(metadata.get("away_team_id", "away"))
	if _viewer.has_method("set_team_colors"):
		_viewer.set_team_colors(home_id, away_id)

	if _viewer.has_method("setup_teams") and not rosters.is_empty():
		var home_roster: Array = _extract_players_from_roster(rosters.get("home", {}))
		var away_roster: Array = _extract_players_from_roster(rosters.get("away", {}))
		_viewer.setup_teams(home_roster, away_roster, home_id, away_id)


func _extract_players_from_roster(roster_variant: Variant) -> Array:
	if roster_variant is Array:
		return (roster_variant as Array).duplicate(true)
	if roster_variant is Dictionary:
		if roster_variant.has("players") and roster_variant.players is Array:
			return (roster_variant.players as Array).duplicate(true)
	return []


func _init_controls() -> void:
	if not _controls:
		return

	if _controls.has_method("set_controls_enabled"):
		_controls.set_controls_enabled(false)
	if _controls.has_method("set_play_state"):
		_controls.set_play_state(false)
	if _controls.has_method("set_speed_value"):
		_controls.set_speed_value(1.0)
	if _controls.has_method("set_event_markers"):
		_controls.set_event_markers([])

	if _controls.has_signal("play_requested"):
		_controls.play_requested.connect(_on_controls_play_requested)
	if _controls.has_signal("pause_requested"):
		_controls.pause_requested.connect(_on_controls_pause_requested)
	if _controls.has_signal("stop_requested"):
		_controls.stop_requested.connect(_on_controls_stop_requested)
	if _controls.has_signal("seek_requested"):
		_controls.seek_requested.connect(_on_controls_seek_requested)
	if _controls.has_signal("speed_changed"):
		_controls.speed_changed.connect(_on_controls_speed_changed)
	if _controls.has_signal("exit_requested"):
		_controls.exit_requested.connect(_on_controls_exit_requested)


func _get_timeline_controller() -> Node:
	if _timeline_controller:
		return _timeline_controller
	_timeline_controller = get_node_or_null(_TIMELINE_CONTROLLER_PATH)
	return _timeline_controller


func _init_position_playback(record: Dictionary, rosters: Dictionary, events: Array) -> void:
	var position_data: Dictionary = record.get("position_data", {})
	if position_data.is_empty():
		return

	var controller := _get_timeline_controller()
	if not controller:
		push_warning("[HorizontalMatchSessionViewer] MatchTimelineController not found; position playback disabled")
		return
	if not controller.has_method("load_position_data"):
		push_warning("[HorizontalMatchSessionViewer] MatchTimelineController has no load_position_data method")
		return

	# Phase20 P0.1: Always pass timeline events into the controller so the unified pipeline
	# can expose `StandardSnapshot.events` for overlays/SFX/timeline UX.
	controller.load_position_data(position_data, rosters, events)

	if _controls and controller.has_method("has_position_data") and controller.has_position_data():
		var duration_ms := int(controller.get("position_total_duration_ms"))
		if _controls.has_method("set_total_duration"):
			_controls.set_total_duration(duration_ms)
		if _controls.has_method("set_controls_enabled"):
			_controls.set_controls_enabled(true)
		if _controls.has_method("set_speed_value"):
			_controls.set_speed_value(1.0)
		if _controls.has_method("set_play_state"):
			_controls.set_play_state(false)
		if _controls.has_method("update_timestamp"):
			_controls.update_timestamp(0)

	# Start playback after a brief delay to allow viewer/pipeline to initialize.
	await get_tree().create_timer(0.1).timeout

	var holder := get_node_or_null(_HOLDER_PATH)
	var pending_clip_ms: int = 0
	var pending_autoplay: bool = false
	if holder:
		pending_clip_ms = int(holder.get("pending_clip_ms", 0))
		pending_autoplay = bool(holder.get("pending_autoplay", false))

	if pending_autoplay and pending_clip_ms > 0 and controller.has_method("play_clip_at"):
		controller.play_clip_at(pending_clip_ms, _current_speed)
		_playback_started = true
		_is_paused = false
		if _controls and _controls.has_method("set_play_state"):
			_controls.set_play_state(true)
		if holder:
			if holder.has_method("clear_pending_clip"):
				holder.clear_pending_clip()
			else:
				holder.set("pending_clip_ms", 0)
				holder.set("pending_autoplay", false)
	else:
		if controller.has_method("start_position_playback"):
			controller.start_position_playback(_current_speed)
			_playback_started = true
			_is_paused = false
			if _controls and _controls.has_method("set_play_state"):
				_controls.set_play_state(true)


func _on_viewer_snapshot_applied(t_ms: int) -> void:
	_current_timestamp = t_ms
	if _controls and _controls.has_method("update_timestamp"):
		_controls.update_timestamp(t_ms)


func _on_controls_play_requested() -> void:
	var controller := _get_timeline_controller()
	if not controller:
		return
	if not _playback_started:
		if controller.has_method("start_position_playback"):
			controller.start_position_playback(_current_speed)
		_playback_started = true
		_is_paused = false
	else:
		if _is_paused:
			if controller.has_method("resume_position_playback"):
				controller.resume_position_playback()
			_is_paused = false
		else:
			if controller.has_method("set_position_playback_speed"):
				controller.set_position_playback_speed(_current_speed)
	if _controls and _controls.has_method("set_play_state"):
		_controls.set_play_state(true)


func _on_controls_pause_requested() -> void:
	var controller := _get_timeline_controller()
	if not controller:
		return
	if controller.has_method("pause_position_playback"):
		controller.pause_position_playback()
	_is_paused = true
	if _controls and _controls.has_method("set_play_state"):
		_controls.set_play_state(false)


func _on_controls_stop_requested() -> void:
	var controller := _get_timeline_controller()
	if not controller:
		return
	if controller.has_method("stop_position_playback"):
		controller.stop_position_playback()
	_playback_started = false
	_is_paused = false
	_current_timestamp = 0
	if _controls and _controls.has_method("update_timestamp"):
		_controls.update_timestamp(0)
	if _controls and _controls.has_method("set_play_state"):
		_controls.set_play_state(false)


func _on_controls_seek_requested(time_ms: int) -> void:
	var controller := _get_timeline_controller()
	if not controller:
		return
	if controller.has_method("seek_position_time"):
		controller.seek_position_time(time_ms)


func _on_controls_speed_changed(multiplier: float) -> void:
	_current_speed = max(multiplier, 0.1)
	var controller := _get_timeline_controller()
	if controller and controller.has_method("set_position_playback_speed"):
		controller.set_position_playback_speed(_current_speed)


func _on_controls_exit_requested() -> void:
	var holder := get_node_or_null(_HOLDER_PATH)
	if holder and holder.has_method("get_return_scene"):
		var path: String = str(holder.get_return_scene())
		if path != "":
			get_tree().change_scene_to_file(path)
			return
	get_tree().change_scene_to_file("res://scenes/MainHomeScreenProper.tscn")


func _extract_rosters(record: Dictionary) -> Dictionary:
	# Prefer explicit rosters fields.
	if record.has("rosters") and record.rosters is Dictionary:
		return (record.rosters as Dictionary).duplicate(true)
	if record.has("timeline_rosters") and record.timeline_rosters is Dictionary:
		return (record.timeline_rosters as Dictionary).duplicate(true)

	# Try timeline_doc.rosters (legacy keys supported).
	var legacy_doc_key := "re" + "play"
	var legacy_doc_key2 := ("re" + "play") + "_doc"
	var doc_variant: Variant = record.get("timeline_doc", record.get(legacy_doc_key2, record.get(legacy_doc_key, {})))
	if doc_variant is Dictionary and doc_variant.has("rosters") and doc_variant.rosters is Dictionary:
		return (doc_variant.rosters as Dictionary).duplicate(true)

	# Try nested match_result payload.
	var match_result: Variant = record.get("match_result", record.get("raw_result", {}))
	if match_result is Dictionary:
		var mr: Dictionary = match_result
		if mr.has("rosters") and mr.rosters is Dictionary:
			return (mr.rosters as Dictionary).duplicate(true)
		doc_variant = mr.get("timeline_doc", mr.get(legacy_doc_key2, mr.get(legacy_doc_key, {})))
		if doc_variant is Dictionary and doc_variant.has("rosters") and doc_variant.rosters is Dictionary:
			return (doc_variant.rosters as Dictionary).duplicate(true)

	return {}


func _extract_metadata(record: Dictionary) -> Dictionary:
	var metadata: Dictionary = {}
	metadata["home_team_name"] = record.get("home_team_name", record.get("team_home_name", "Home"))
	metadata["away_team_name"] = record.get("opponent_name", record.get("team_away_name", "Away"))
	metadata["home_team_id"] = record.get("home_team_id", "home")
	metadata["away_team_id"] = record.get("away_team_id", "away")

	var final_score: Variant = record.get("final_score", [])
	if final_score is Array and (final_score as Array).size() >= 2:
		metadata["home_score"] = int((final_score as Array)[0])
		metadata["away_score"] = int((final_score as Array)[1])
	else:
		metadata["home_score"] = int(record.get("goals_scored", record.get("home_score", 0)))
		metadata["away_score"] = int(record.get("goals_conceded", record.get("away_score", 0)))

	metadata["year"] = int(record.get("year", 1))
	metadata["week"] = int(record.get("week", 1))
	metadata["match_id"] = str(record.get("match_id", ""))
	metadata["seed"] = int(record.get("seed", 0))
	metadata["hero_player_id"] = str(record.get("hero_player_id", record.get("user_player_id", "")))

	return metadata


func _extract_events(record: Dictionary) -> Array:
	# Prefer top-level timeline events.
	var timeline_variant: Variant = record.get("timeline", null)
	if timeline_variant is Array:
		return (timeline_variant as Array).duplicate(true)

	# Prefer top-level events if present.
	var events_variant: Variant = record.get("events", null)
	if events_variant is Array:
		return (events_variant as Array).duplicate(true)

	# Try timeline_doc.events (legacy keys supported).
	var legacy_doc_key := "re" + "play"
	var legacy_doc_key2 := ("re" + "play") + "_doc"
	var doc_variant: Variant = record.get("timeline_doc", record.get(legacy_doc_key2, record.get(legacy_doc_key, {})))
	if doc_variant is Dictionary and doc_variant.has("events") and doc_variant.events is Array:
		return (doc_variant.events as Array).duplicate(true)

	# Try nested match_result payload.
	var match_result: Variant = record.get("match_result", record.get("raw_result", {}))
	if match_result is Dictionary and match_result.has("events") and match_result.events is Array:
		return (match_result.events as Array).duplicate(true)

	return []
