extends Control
## TacticalAnalysisViewer Controller
## Reads a match record from MatchTimelineHolder and feeds it into MatchTimelineViewer.

const _LEGACY_PAYLOAD_KEY := "re" + "play"
const _LEGACY_DOC_KEY := _LEGACY_PAYLOAD_KEY + "_doc"
const _LEGACY_EVENTS_KEY := _LEGACY_PAYLOAD_KEY + "_events"
const _LEGACY_ROSTERS_KEY := _LEGACY_PAYLOAD_KEY + "_rosters"

@onready var _viewer: MatchTimelineViewer = $MatchTimelineViewer


func _ready() -> void:
	if not _viewer:
		push_error("[TacticalAnalysisViewerController] Viewer node not found!")
		return

	var record: Dictionary = {}
	if MatchTimelineHolder:
		record = MatchTimelineHolder.get_timeline_data()

	if record.is_empty():
		push_warning("[TacticalAnalysisViewerController] No match record available")
		return

	var events: Array = _extract_events(record)
	var rosters: Dictionary = _extract_rosters(record)
	var metadata: Dictionary = _extract_metadata(record)
	var stored_events: Array = _extract_stored_events(record)

	_viewer.load_timeline_data(events, rosters, metadata, stored_events)
	_viewer.apply_preset_tactical_analysis()
	await _init_position_playback(record)

	print("[TacticalAnalysisViewerController] Tactical view ready")


func _init_position_playback(record: Dictionary) -> void:
	var position_data: Dictionary = record.get("position_data", {})
	if position_data.is_empty():
		var match_result_raw = record.get("match_result", record.get("raw_result", {}))
		if match_result_raw is Dictionary:
			var match_result: Dictionary = match_result_raw
			position_data = match_result.get("position_data", match_result.get("position_payload", {}))

	if position_data.is_empty():
		return

	var controller := get_node_or_null("/root/MatchTimelineController")
	if not controller:
		push_warning("[TacticalAnalysisViewerController] Timeline controller not found - position playback disabled")
		return

	if controller.has_method("load_position_data"):
		var rosters: Dictionary = _extract_rosters(record)
		var events: Array = _extract_events(record)
		controller.load_position_data(position_data, rosters, events)

	await get_tree().create_timer(0.1).timeout
	if controller.has_method("start_position_playback"):
		controller.start_position_playback(1.0)


func _extract_events(record: Dictionary) -> Array:
	var direct_variant: Variant = record.get("timeline_events", null)
	if direct_variant is Array and not (direct_variant as Array).is_empty():
		return (direct_variant as Array).duplicate(true)

	var sources: Array = []
	var doc_variant: Variant = record.get("timeline_doc", null)
	if doc_variant is Dictionary:
		sources.append(doc_variant)
	var legacy_doc_variant: Variant = record.get(_LEGACY_DOC_KEY, record.get(_LEGACY_PAYLOAD_KEY, null))
	if legacy_doc_variant is Dictionary:
		sources.append(legacy_doc_variant)
	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary:
		sources.append(match_result_variant)
	var raw_result_variant: Variant = record.get("raw_result", {})
	if raw_result_variant is Dictionary:
		sources.append(raw_result_variant)

	for source in sources:
		if source is Dictionary:
			var events_variant: Variant = (source as Dictionary).get("events", null)
			if events_variant is Array and not (events_variant as Array).is_empty():
				return (events_variant as Array).duplicate(true)

	var fallback_variant: Variant = record.get("events", record.get(_LEGACY_EVENTS_KEY, []))
	if fallback_variant is Array:
		return (fallback_variant as Array).duplicate(true)
	return []


func _extract_rosters(record: Dictionary) -> Dictionary:
	var candidate_keys := ["timeline_rosters", "rosters", _LEGACY_ROSTERS_KEY]
	for key in candidate_keys:
		var variant: Variant = record.get(key, null)
		if variant is Dictionary and not (variant as Dictionary).is_empty():
			return (variant as Dictionary).duplicate(true)

	var doc_variant: Variant = record.get("timeline_doc", null)
	if doc_variant is Dictionary and (doc_variant as Dictionary).has("rosters"):
		var rosters_variant: Variant = (doc_variant as Dictionary).get("rosters")
		if rosters_variant is Dictionary:
			return (rosters_variant as Dictionary).duplicate(true)

	var legacy_doc_variant: Variant = record.get(_LEGACY_DOC_KEY, record.get(_LEGACY_PAYLOAD_KEY, {}))
	if legacy_doc_variant is Dictionary and (legacy_doc_variant as Dictionary).has("rosters"):
		var legacy_rosters_variant: Variant = (legacy_doc_variant as Dictionary).get("rosters")
		if legacy_rosters_variant is Dictionary:
			return (legacy_rosters_variant as Dictionary).duplicate(true)

	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary and (match_result_variant as Dictionary).has("rosters"):
		var mr_rosters: Variant = (match_result_variant as Dictionary).get("rosters")
		if mr_rosters is Dictionary:
			return (mr_rosters as Dictionary).duplicate(true)

	return {}


func _extract_metadata(record: Dictionary) -> Dictionary:
	var metadata: Dictionary = {}
	var direct_metadata_variant: Variant = record.get("timeline_metadata", null)
	if direct_metadata_variant is Dictionary and not (direct_metadata_variant as Dictionary).is_empty():
		metadata = (direct_metadata_variant as Dictionary).duplicate(true)

	if not metadata.has("home_team_name"):
		metadata["home_team_name"] = record.get("home_team_name", record.get("team_home_name", "Home"))
	if not metadata.has("away_team_name"):
		metadata["away_team_name"] = record.get("opponent_name", record.get("team_away_name", "Away"))

	metadata["seed"] = record.get("seed", 0)
	metadata["hero_player_id"] = record.get("hero_player_id", record.get("user_player_id", ""))
	return metadata


func _extract_stored_events(record: Dictionary) -> Array:
	var sources: Array = []
	sources.append(record.get("stored_events", null))
	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary and (match_result_variant as Dictionary).has("stored_events"):
		sources.append((match_result_variant as Dictionary).get("stored_events"))
	var doc_variant: Variant = record.get("timeline_doc", null)
	if doc_variant is Dictionary and (doc_variant as Dictionary).has("stored_events"):
		sources.append((doc_variant as Dictionary).get("stored_events"))
	var raw_result_variant: Variant = record.get("raw_result", {})
	if raw_result_variant is Dictionary and (raw_result_variant as Dictionary).has("stored_events"):
		sources.append((raw_result_variant as Dictionary).get("stored_events"))
	for source in sources:
		if source is Array and not (source as Array).is_empty():
			return (source as Array).duplicate(true)
	return []
