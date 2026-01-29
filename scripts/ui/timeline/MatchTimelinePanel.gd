extends Control
class_name MatchTimelinePanel

## Top-level timeline panel that connects controls to MatchTimelineController.

const _MatchTimelineControlsScript = preload("res://scripts/ui/timeline/MatchTimelineControls.gd")
const _TimelineField2DScript = preload("res://scripts/ui/timeline/TimelineField2D.gd")
const _MatchTimelineViewerScript = preload("res://scripts/match_pipeline/MatchTimelineViewer.gd")

const TIMELINE_CONTROLLER_PATH := "/root/MatchTimelineController"
const UNIFIED_FRAME_PIPELINE_PATH := "/root/UnifiedFramePipeline"
const MINIMAP_VIEWER_SCENE := preload("res://scenes/match_pipeline/MatchTimelineViewer.tscn")
const QA_LOG_SETTING := "debug/timeline/enable_minimap_phase_d_qa_logs"
const QA_SCREENSHOT_SETTING := "debug/timeline/capture_minimap_phase_d_screenshot"

# Legacy key support (never keep the legacy word as a literal in source).
const _LEGACY_PAYLOAD_KEY := "re" + "play"
const _LEGACY_DOC_KEY := _LEGACY_PAYLOAD_KEY + "_doc"
const _LEGACY_EVENTS_KEY := _LEGACY_PAYLOAD_KEY + "_events"
const _LEGACY_ROSTERS_KEY := _LEGACY_PAYLOAD_KEY + "_rosters"

const _MatchTimeFormatter = preload("res://scripts/utils/MatchTimeFormatter.gd")

@onready var _controls = %MatchTimelineControls  # Type: _MatchTimelineControlsScript
@onready var _status_label: Label = %StatusLabel
@onready var _title_label: Label = %TitleLabel
@onready var _score_label: Label = %ScoreLabel
@onready var _dev_toggle_button: Button = %DevToggleButton
@onready var _close_button: Button = %CloseButton
@onready var _field = %TimelineField  # Type: _TimelineField2DScript
@onready var _event_panel: PanelContainer = %EventPanel
@onready var _event_list: ItemList = %EventList
# EventTitle moved inside EventHeader HBox for Why button placement
@onready var _event_title_label: Label = get_node_or_null("%EventTitle")
@onready var _event_summary_label: Label = get_node_or_null("%EventSummary")
@onready var _dev_panel: PanelContainer = %DevPanel
@onready var _dev_text: RichTextLabel = %DevText
@onready var _dev_title_label: Label = %DevTitle
@onready var _minimap_holder: Control = %MiniMapHolder
# RuleBook "Why?" button UI (P1 Godot Integration)
@onready var _why_button: Button = get_node_or_null("%WhyButton")
@onready var _why_popup: PopupPanel = get_node_or_null("%WhyPopup")
@onready var _why_popup_text: RichTextLabel = get_node_or_null("%WhyPopupText")
@onready var _why_popup_close_btn: Button = get_node_or_null("WhyPopup/WhyPopupMargin/WhyPopupVBox/WhyPopupCloseBtn")

var _timeline_controller: Node = null
var _rust_simulator: Object = null  # FootballMatchSimulator for RuleBook UI Card API
var _frame_pipeline: Node = null
var _pending_payload: Dictionary = {}
var _pending_record: Dictionary = {}
var _has_payload: bool = false
var _playback_started: bool = false
var _is_paused: bool = false
var _current_speed: float = 1.0
var _current_timestamp: int = 0
var _roster_payload: Dictionary = {}
var _timeline_events_payload: Array = []
var _timeline_markers: Array = []
var _last_snapshot: Dictionary = {}
var _dev_panel_visible: bool = false
var _last_marker: Dictionary = {}
var _marker_index_map: Dictionary = {}
var _suppress_event_list_signal: bool = false
var _minimap = null  # Type: _MatchTimelineViewerScript
var _pending_screenshot: bool = false


func _ready() -> void:
	_controls.set_controls_enabled(false)
	_controls.play_requested.connect(_on_play_requested)
	_controls.pause_requested.connect(_on_pause_requested)
	_controls.stop_requested.connect(_on_stop_requested)
	_controls.seek_requested.connect(_on_seek_requested)
	_controls.speed_changed.connect(_on_speed_changed)
	_controls.marker_selected.connect(_on_marker_selected)
	if _event_list:
		_event_list.item_selected.connect(_on_event_list_selected)
		_event_list.item_activated.connect(_on_event_list_activated)
	if _event_title_label:
		_event_title_label.text = _translate_or_default("UI_TIMELINE_EVENTS_TITLE", "Events")
	if _dev_title_label:
		_dev_title_label.text = _translate_or_default("UI_TIMELINE_DEV_TITLE", "Dev Snapshot")
	if _dev_toggle_button:
		_dev_toggle_button.toggle_mode = true
		_dev_toggle_button.toggled.connect(_on_dev_toggle_toggled)
	if _dev_panel:
		_dev_panel.visible = false
	_dev_panel_visible = false
	if _dev_toggle_button:
		_dev_toggle_button.button_pressed = false
	_close_button.pressed.connect(_on_close_pressed)
	_spawn_minimap()
	_initialize_rulebook_ui()

	_attach_timeline_controller()

	if not _pending_record.is_empty():
		_apply_match_record(_pending_record)
		_pending_record.clear()
	if not _pending_payload.is_empty():
		_apply_position_payload(_pending_payload)
		_pending_payload.clear()


func set_match_record(record: Dictionary) -> void:
	_pending_record = record.duplicate(true)
	if is_inside_tree():
		_apply_match_record(_pending_record)
		_pending_record.clear()


func set_position_payload(payload: Dictionary) -> void:
	_pending_payload = payload.duplicate(true)
	if is_inside_tree():
		_apply_position_payload(_pending_payload)
		_pending_payload.clear()


func set_status_message(message: String) -> void:
	_status_label.text = message


func _attach_timeline_controller() -> void:
	_timeline_controller = get_node_or_null(TIMELINE_CONTROLLER_PATH)
	if not _timeline_controller:
		_status_label.text = _translate_or_default(
			"UI_TIMELINE_STATUS_NO_CONTROLLER", "Timeline controller unavailable"
		)
		return

	if not _timeline_controller.position_playback_started.is_connected(_on_position_playback_started):
		_timeline_controller.position_playback_started.connect(_on_position_playback_started)
	if not _timeline_controller.position_playback_stopped.is_connected(_on_position_playback_stopped):
		_timeline_controller.position_playback_stopped.connect(_on_position_playback_stopped)

	# Match OS unification: snapshots must be consumed from UnifiedFramePipeline only.
	_frame_pipeline = get_node_or_null(UNIFIED_FRAME_PIPELINE_PATH)
	if _frame_pipeline and _frame_pipeline.has_signal("snapshot_ready"):
		if not _frame_pipeline.snapshot_ready.is_connected(_on_unified_snapshot):
			_frame_pipeline.snapshot_ready.connect(_on_unified_snapshot)
	else:
		push_warning("[MatchTimelinePanel] UnifiedFramePipeline not available; minimap will not receive snapshots.")

	_status_label.text = _translate_or_default("UI_TIMELINE_STATUS_WAITING", "Load a timeline to begin")
	_hydrate_from_controller_state()


func _spawn_minimap() -> void:
	if not MINIMAP_VIEWER_SCENE or not _minimap_holder or _minimap:
		return
	_minimap = MINIMAP_VIEWER_SCENE.instantiate()
	if not _minimap:
		return

	# Apply session preset for real-time viewing (camera follow + minimap overlay)
	if _minimap.has_method("apply_preset_session_match"):
		_minimap.apply_preset_session_match()

	_minimap_holder.add_child(_minimap)
	if _minimap is Control:
		var ctrl := _minimap as Control
		ctrl.anchor_left = 0.0
		ctrl.anchor_top = 0.0
		ctrl.anchor_right = 1.0
		ctrl.anchor_bottom = 1.0
		ctrl.offset_left = 0.0
		ctrl.offset_top = 0.0
		ctrl.offset_right = 0.0
		ctrl.offset_bottom = 0.0


## RuleBook P1: Initialize "Why?" button and Rust simulator reference
func _initialize_rulebook_ui() -> void:
	# Get Rust simulator for RuleBook UI Card API
	var engine := get_node_or_null("/root/FootballRustEngine")
	if engine:
		var sim: Variant = engine.get("_rust_simulator")
		if sim != null and sim is Object:
			_rust_simulator = sim

	# Connect "Why?" button if it exists in the scene
	if _why_button:
		_why_button.pressed.connect(_on_why_button_pressed)
		_why_button.visible = false  # Hidden until an event is selected

	# Initialize popup close behavior
	if _why_popup:
		_why_popup.popup_hide.connect(_on_why_popup_closed)
	if _why_popup_close_btn:
		_why_popup_close_btn.pressed.connect(_on_why_popup_close_pressed)
	if _why_popup_text:
		_why_popup_text.bbcode_enabled = true
		if not _why_popup_text.meta_clicked.is_connected(_on_why_popup_meta_clicked):
			_why_popup_text.meta_clicked.connect(_on_why_popup_meta_clicked)


## RuleBook P1: Check if "Why?" button should be shown for the current event
func _update_why_button_visibility(marker: Dictionary) -> void:
	if not _why_button or not _rust_simulator:
		return

	var event_type := str(marker.get("event_type", "")).to_lower()
	if event_type == "":
		_why_button.visible = false
		return

	# Call Rust API to check if this event type has an explanation
	if _rust_simulator.has_method("should_show_why_button"):
		var should_show: bool = _rust_simulator.should_show_why_button(event_type)
		_why_button.visible = should_show
	else:
		_why_button.visible = false


## RuleBook P1: Handle "Why?" button press
func _on_why_button_pressed() -> void:
	if _last_marker.is_empty():
		return

	_show_event_explanation(_last_marker)


## RuleBook P1: Show rule explanation popup for an event
func _show_event_explanation(marker: Dictionary) -> void:
	if not _rust_simulator:
		push_warning("[MatchTimelinePanel] No Rust simulator available for RuleBook API")
		return

	var event_type := str(marker.get("event_type", "")).to_lower()
	if event_type == "":
		return

	# Determine language preference (Korean if system locale starts with "ko")
	var use_korean := OS.get_locale().begins_with("ko")

	var explanation: Dictionary = {}

	# Preferred: full event JSON (more accurate cards/ref links)
	var event_json := str(marker.get("_event_json", ""))
	if event_json != "" and _rust_simulator.has_method("get_event_explanation_from_json"):
		explanation = _rust_simulator.get_event_explanation_from_json(event_json, use_korean)

	# Fallback: event_type only (generic explanation)
	if explanation.is_empty() and _rust_simulator.has_method("get_event_explanation"):
		explanation = _rust_simulator.get_event_explanation(event_type, use_korean)

	if explanation.is_empty() or bool(explanation.get("error", false)):
		_show_why_popup_fallback(marker)
		return

	# Render the UI Card
	_render_why_popup(explanation, marker)


## RuleBook P1: Render explanation popup with UI Card data
func _render_why_popup(explanation: Dictionary, marker: Dictionary) -> void:
	if not _why_popup or not _why_popup_text:
		# Fallback: show in status label
		var title := str(explanation.get("schema_version", "Rule Explanation"))
		_status_label.text = title
		return

	var lines: PackedStringArray = []

	# Header: L1 title if present, else event type
	var cards: Array = explanation.get("cards", [])
	var title := ""
	if cards.size() > 0 and cards[0] is Dictionary:
		title = str((cards[0] as Dictionary).get("title", ""))
	if title == "":
		var ev: Dictionary = explanation.get("event", {})
		title = str(ev.get("event_type", "Rule Explanation"))

	lines.append("[b][font_size=18]%s[/font_size][/b]" % title)
	lines.append("")

	# Rule reference (optional)
	var rule: Variant = explanation.get("rule", null)
	if rule is Dictionary:
		var r: Dictionary = rule
		var law_number := int(r.get("law_number", 0))
		var law_name := str(r.get("law_name", ""))
		if law_number > 0 and law_name != "":
			lines.append("[color=gray]Law %d — %s[/color]" % [law_number, law_name])
		lines.append("")

	# Cards (L1, L2, L3)
	for block in cards:
		if not (block is Dictionary):
			continue
		var block_title := str(block.get("title", ""))
		var level := int(block.get("level", 1))

		# Format block title based on level
		if block_title != "":
			match level:
				1:
					lines.append("[b]%s[/b]" % block_title)
				2:
					lines.append("[b][color=#4a90d9]%s[/color][/b]" % block_title)
				3:
					lines.append("[i]%s[/i]" % block_title)

		# Render lines in block
		var block_lines: Array = block.get("lines", [])
		for line in block_lines:
			if not (line is Dictionary):
				continue
			var line_text := _format_card_line(line)
			if line_text != "":
				lines.append(line_text)

		lines.append("")  # Spacing between blocks

	_why_popup_text.text = "\n".join(lines)

	# Position and show popup
	var popup_size := Vector2i(400, 300)
	var screen_size := DisplayServer.window_get_size()
	var popup_pos := Vector2i(
		(screen_size.x - popup_size.x) / 2,
		(screen_size.y - popup_size.y) / 2
	)
	_why_popup.popup(Rect2i(popup_pos, popup_size))


## RuleBook P1: Format a single CardLine for BBCode display
func _format_card_line(line: Dictionary) -> String:
	var text := str(line.get("text", ""))
	var kind := str(line.get("kind", "plain"))

	if text == "":
		return ""

	var formatted := ""
	match kind:
		"bullet":
			formatted = "  • %s" % text
		"kv":
			formatted = "[color=#888888]%s[/color]" % text
		"warning":
			formatted = "[color=#e67e22]⚠ %s[/color]" % text
		"note":
			formatted = "[color=#7f8c8d][i]%s[/i][/color]" % text
		_:  # plain
			formatted = text

	# Optional replay/analytics ref (clickable)
	var ref: Variant = line.get("ref", null)
	if ref is Dictionary:
		var r: Dictionary = ref
		var ref_type := str(r.get("type", ""))
		var ref_id := str(r.get("id", ""))
		if ref_type != "" and ref_id != "":
			var meta := "ref:%s:%s" % [ref_type, ref_id]
			formatted = "[url=%s][color=#4a90d9][u]%s[/u][/color][/url]" % [meta, formatted]

	return formatted


func _on_why_popup_meta_clicked(meta: Variant) -> void:
	var meta_str := str(meta)
	if not meta_str.begins_with("ref:"):
		return
	var parts := meta_str.split(":", false)
	if parts.size() < 3:
		return
	var ref_type := parts[1]
	var ref_id := parts[2]

	if not _field:
		return

	if _field.has_method("clear_highlights"):
		_field.clear_highlights()

	match ref_type:
		"player_track_id":
			if ref_id.is_valid_int() and _field.has_method("highlight_player_track_id"):
				_field.highlight_player_track_id(int(ref_id))
		"pitch_x_m":
			if ref_id.is_valid_float() and _field.has_method("highlight_pitch_x_m"):
				_field.highlight_pitch_x_m(float(ref_id))
		"pitch_line":
			if _field.has_method("highlight_pitch_line"):
				_field.highlight_pitch_line(ref_id)
		"event_id":
			# vNext: jump to event_id
			push_warning("[MatchTimelinePanel] event_id jump not implemented: %s" % ref_id)


## RuleBook P1: Fallback when no explanation is available
func _show_why_popup_fallback(marker: Dictionary) -> void:
	var event_type := str(marker.get("event_type", "event"))
	var message := _translate_or_default(
		"UI_TIMELINE_NO_RULE_EXPLANATION",
		"No rule explanation available for: %s"
	) % event_type

	if _why_popup and _why_popup_text:
		_why_popup_text.text = message
		var popup_size := Vector2i(300, 100)
		var screen_size := DisplayServer.window_get_size()
		var popup_pos := Vector2i(
			(screen_size.x - popup_size.x) / 2,
			(screen_size.y - popup_size.y) / 2
		)
		_why_popup.popup(Rect2i(popup_pos, popup_size))
	else:
		_status_label.text = message


## RuleBook P1: Handle popup close
func _on_why_popup_closed() -> void:
	# Optional: any cleanup needed when popup closes
	pass


## RuleBook P1: Handle close button press
func _on_why_popup_close_pressed() -> void:
	if _why_popup:
		_why_popup.hide()


func _update_minimap_data(record: Dictionary) -> void:
	if not _minimap:
		return
	var metadata := _extract_minimap_metadata(record)
	var stored_events := _extract_stored_events(record)
	var events := _extract_timeline_events(record)
	# MatchTimelineViewer uses load_timeline_data()
	if _minimap.has_method("load_timeline_data"):
		_minimap.load_timeline_data(events, _roster_payload, metadata, stored_events)
	_restart_minimap()
	_minimap.set_speed(_current_speed)
	_sync_minimap_time(_current_timestamp, _playback_started and not _is_paused)


func _restart_minimap() -> void:
	if _minimap:
		if _minimap.has_method("restart"):
			_minimap.restart()
		if _minimap.has_method("pause"):
			_minimap.pause()


func _pause_minimap() -> void:
	if _minimap and _minimap.has_method("pause"):
		_minimap.pause()


func _resume_minimap() -> void:
	if _minimap:
		_sync_minimap_time(_current_timestamp, not _is_paused)


func _sync_minimap_time(target_ms: int, should_play: bool) -> void:
	if not _minimap:
		return
	var clamped_ms: int = max(target_ms, 0)
	var target_seconds := float(clamped_ms) / 1000.0

	if _minimap.has_method("set_speed"):
		_minimap.set_speed(_current_speed)

	if _minimap.has_method("jump_to"):
		_minimap.jump_to(target_seconds)

	if should_play:
		if _minimap.has_method("play"):
			_minimap.play()
	else:
		if _minimap.has_method("pause"):
			_minimap.pause()


func _hydrate_from_controller_state() -> void:
	if not _timeline_controller:
		return
	if _timeline_controller.has_method("has_position_data") and _timeline_controller.has_position_data():
		_has_payload = true
		var duration := int(_timeline_controller.position_total_duration_ms)
		_controls.set_total_duration(duration)
		_controls.set_controls_enabled(true)
		_controls.set_speed_value(1.0)
		_controls.set_play_state(false)
		_controls.update_timestamp(0)
		_status_label.text = _translate_or_default("UI_TIMELINE_STATUS_READY", "Ready")
		_update_field_snapshot(0)


func _apply_match_record(record: Dictionary) -> void:
	var opponent := str(record.get("opponent_name", record.get("opponent", "")))
	var home_team := str(record.get("home_team", "Home"))
	var score_home := int(record.get("home_score", record.get("goals_scored", 0)))
	var score_away := int(record.get("away_score", record.get("goals_conceded", 0)))
	var title := str(record.get("title", "Match Timeline"))
	var pen_suffix := _MatchTimeFormatter.format_penalty_shootout_suffix(record)

	if _title_label:
		_title_label.text = (
			title if title != "" else "%s vs %s" % [home_team, opponent if opponent != "" else "Opponent"]
		)
	if _score_label:
		_score_label.text = "%s %d : %d %s%s" % [
			home_team,
			score_home,
			score_away,
			opponent if opponent != "" else "?",
			pen_suffix,
		]
	_roster_payload = _extract_rosters(record)
	_timeline_events_payload = _extract_timeline_events(record)
	if _field and not _roster_payload.is_empty():
		_field.set_rosters(_roster_payload)
	elif _field:
		_field.set_rosters({})
	_timeline_markers = _extract_timeline_markers(record)
	_build_marker_index_map()
	_populate_event_list()
	if _controls:
		_controls.set_event_markers(_timeline_markers)
	_update_minimap_data(record)
	_export_timeline_qa_snapshot(record)
	if _qa_screenshot_enabled() and _timeline_markers.size() > 0:
		_apply_marker_selection(_timeline_markers[0], true)


func _apply_position_payload(payload: Dictionary) -> void:
	if not _timeline_controller:
		return
	if payload.is_empty():
		_controls.set_controls_enabled(false)
		_status_label.text = _translate_or_default("UI_TIMELINE_STATUS_NO_DATA", "No timeline data found")
		_has_payload = false
		_timeline_events_payload = []
		if _field:
			_field.clear_snapshot()
		_last_snapshot = {}
		_update_debug_panel()
		return

	# Ensure MatchTimelineController has rosters + timeline events for UnifiedFramePipeline overlays.
	_timeline_controller.load_position_data(payload.duplicate(true), _roster_payload, _timeline_events_payload)
	_controls.set_total_duration(_timeline_controller.position_total_duration_ms)
	_controls.update_timestamp(0)
	_controls.set_controls_enabled(true)
	_controls.set_play_state(false)
	_controls.set_speed_value(1.0)
	_current_speed = 1.0
	if _minimap:
		_minimap.set_speed(_current_speed)
	_current_timestamp = 0
	_has_payload = true
	_playback_started = false
	_is_paused = false
	_status_label.text = _translate_or_default("UI_TIMELINE_STATUS_READY", "Ready")
	_update_field_snapshot(0)
	if _controls:
		_controls.set_event_markers(_timeline_markers)
	_restart_minimap()
	_sync_minimap_time(_current_timestamp, false)


func _ensure_payload_ready() -> bool:
	if not _timeline_controller:
		_status_label.text = _translate_or_default(
			"UI_TIMELINE_STATUS_NO_CONTROLLER", "Timeline controller unavailable"
		)
		return false
	if not _has_payload:
		_status_label.text = _translate_or_default("UI_TIMELINE_STATUS_NO_DATA", "No timeline data found")
		return false
	return true


func _on_play_requested() -> void:
	if not _ensure_payload_ready():
		_controls.set_play_state(false)
		return
	if not _playback_started:
		_timeline_controller.start_position_playback(_current_speed)
	else:
		if _is_paused:
			_timeline_controller.resume_position_playback()
			_is_paused = false
		else:
			_timeline_controller.set_position_playback_speed(_current_speed)
	_resume_minimap()
	_controls.set_play_state(true)


func _on_pause_requested() -> void:
	if not _playback_started or not _timeline_controller:
		return
	_timeline_controller.pause_position_playback()
	_is_paused = true
	_controls.set_play_state(false)
	_pause_minimap()


func _on_stop_requested() -> void:
	if _timeline_controller:
		_timeline_controller.stop_position_playback()
		_timeline_controller.seek_position_time(0)
	_playback_started = false
	_is_paused = false
	_current_timestamp = 0
	_controls.update_timestamp(0)
	_controls.set_play_state(false)
	_restart_minimap()
	_sync_minimap_time(_current_timestamp, false)


func _on_seek_requested(time_ms: int) -> void:
	_current_timestamp = max(time_ms, 0)
	if _timeline_controller:
		_timeline_controller.seek_position_time(_current_timestamp)
	if not _playback_started:
		_controls.update_timestamp(_current_timestamp)
	var should_play := _playback_started and not _is_paused
	_sync_minimap_time(_current_timestamp, should_play)


func _on_speed_changed(multiplier: float) -> void:
	_current_speed = multiplier
	if _minimap:
		_minimap.set_speed(_current_speed)
	if _timeline_controller and _playback_started and not _is_paused:
		_timeline_controller.set_position_playback_speed(_current_speed)


func _on_position_playback_started(duration_ms: int) -> void:
	_playback_started = true
	_is_paused = false
	_controls.set_total_duration(duration_ms)
	_controls.set_play_state(true)
	_status_label.text = _translate_or_default("UI_TIMELINE_STATUS_PLAYING", "Playing")
	_update_debug_panel()


func _on_unified_snapshot(t_ms: int, snapshot: Dictionary) -> void:
	_current_timestamp = t_ms
	_controls.update_timestamp(t_ms)
	if _field:
		_field.set_snapshot(snapshot)
	if _minimap:
		_minimap.apply_position_snapshot(snapshot)
	_last_snapshot = snapshot.duplicate(true)
	_update_debug_panel(_last_snapshot)


func _on_position_playback_stopped() -> void:
	_playback_started = false
	_is_paused = false
	_controls.set_play_state(false)
	_controls.update_timestamp(0)
	_status_label.text = _translate_or_default("UI_TIMELINE_STATUS_STOPPED", "Playback stopped")
	_current_timestamp = 0
	_update_debug_panel()
	_restart_minimap()
	_sync_minimap_time(_current_timestamp, false)


func _on_marker_selected(marker: Dictionary) -> void:
	_apply_marker_selection(marker, true)
	# P0: Trigger key moment lock for important events
	if _controls:
		_controls.trigger_key_moment_from_marker(marker)


func _apply_marker_selection(marker: Dictionary, update_event_list: bool = true) -> void:
	if not (marker is Dictionary):
		return
	_last_marker = marker
	var label := str(marker.get("label", marker.get("event_type", "event")))
	var minute := float(marker.get("time_ms", 0)) / 60000.0
	var time_text := "%.1f'" % minute if minute > 0.0 else "%d ms" % int(marker.get("time_ms", 0))
	var team_id := int(marker.get("team_id", -1))
	var team_text := ""
	if team_id == 0:
		team_text = _translate_or_default("UI_TEAM_HOME", "HOME")
	elif team_id == 1:
		team_text = _translate_or_default("UI_TEAM_AWAY", "AWAY")
	var summary := "%s %s" % [time_text, label]
	if team_text != "":
		summary = "%s %s" % [team_text, summary]
	var detail_line := _format_marker_detail(marker)
	if detail_line != "":
		summary = "%s\n%s" % [summary, detail_line]
	var template := _translate_or_default("UI_TIMELINE_STATUS_EVENT", "Event: %s")
	if template.find("%s") != -1:
		_status_label.text = template % summary
	else:
		_status_label.text = "%s %s" % [template, summary]
	if update_event_list:
		_set_event_list_selection(marker)
	_update_debug_panel()
	_update_why_button_visibility(marker)  # RuleBook P1: Show/hide "Why?" button
	_apply_rulebook_p2_consumption(marker)
	var target_ms := int(marker.get("time_ms", _current_timestamp))
	_current_timestamp = target_ms
	_controls.update_timestamp(target_ms)
	if _timeline_controller:
		_timeline_controller.seek_position_time(target_ms)
	_sync_minimap_time(target_ms, false)
	if _qa_screenshot_enabled():
		_schedule_timeline_screenshot()


func _set_event_list_selection(marker: Dictionary) -> void:
	if not _event_list:
		return
	var marker_id := int(marker.get("_marker_id", -1))
	if not _marker_index_map.has(marker_id):
		return
	var index := int(_marker_index_map[marker_id])
	_suppress_event_list_signal = true
	_event_list.select(index)
	_event_list.ensure_current_is_visible()
	_suppress_event_list_signal = false


func _on_event_list_selected(index: int) -> void:
	if _suppress_event_list_signal:
		return
	if not _event_list:
		return
	var marker: Variant = _event_list.get_item_metadata(index)
	if marker is Dictionary:
		_apply_marker_selection(marker as Dictionary, false)
		_seek_to_marker(marker)


func _on_event_list_activated(index: int) -> void:
	if not _event_list:
		return
	var marker: Variant = _event_list.get_item_metadata(index)
	if marker is Dictionary:
		_seek_to_marker(marker)


func _seek_to_marker(marker: Dictionary) -> void:
	var time_ms := int(marker.get("time_ms", 0))
	_current_timestamp = time_ms
	if _timeline_controller:
		_timeline_controller.seek_position_time(time_ms)
	_controls.update_timestamp(time_ms)
	var should_play := _playback_started and not _is_paused
	_sync_minimap_time(time_ms, should_play)


func _on_close_pressed() -> void:
	queue_free()


## RuleBook P2: Consumption-only overlays + summary on marker selection
func _apply_rulebook_p2_consumption(marker: Dictionary) -> void:
	if _field and _field.has_method("clear_highlights"):
		_field.clear_highlights()

	_update_event_summary_label(marker)
	_apply_rulebook_p2_overlays(marker)


func _update_event_summary_label(marker: Dictionary) -> void:
	if not _event_summary_label:
		return
	var summary := _build_rule_summary_from_marker(marker).strip_edges()
	_event_summary_label.text = summary
	_event_summary_label.visible = summary != ""


func _build_rule_summary_from_marker(marker: Dictionary) -> String:
	var event_json := str(marker.get("_event_json", "")).strip_edges()
	if event_json == "":
		return ""
	var parsed: Variant = JSON.parse_string(event_json)
	if not (parsed is Dictionary):
		return ""
	var event: Dictionary = parsed
	var details: Dictionary = {}
	if event.get("details") is Dictionary:
		details = event.get("details") as Dictionary

	var event_type := str(marker.get("event_type", "")).to_lower()
	if event_type == "":
		event_type = str(event.get("event_type", "")).to_lower()

	# VAR summary (no law mapping required for v1)
	if details.get("var_review") is Dictionary:
		var var_review: Dictionary = details.get("var_review") as Dictionary
		var reviewed := str(var_review.get("reviewed_event_type", "")).strip_edges()
		var outcome := str(var_review.get("outcome", "")).strip_edges()
		var parts: Array[String] = []
		if reviewed != "":
			parts.append(reviewed)
		if outcome != "":
			parts.append(outcome)
		var tail := " · ".join(parts)
		if tail == "":
			return "VAR"
		return "VAR · %s" % tail

	# Rule-based summary (Law N + key values)
	var rule_id := str(details.get("rule_id", "")).strip_edges()
	if rule_id == "":
		return ""

	var law_n := _rule_id_to_law_number(rule_id)
	var pieces: Array[String] = []
	if law_n > 0:
		pieces.append("Law %d" % law_n)
	else:
		pieces.append(rule_id)

	if event_type == "offside" and details.get("offside_details") is Dictionary:
		var offside: Dictionary = details.get("offside_details") as Dictionary
		var margin = offside.get("margin_m", null)
		if margin != null:
			pieces.append("Margin %.2fm" % float(margin))
	elif (event_type == "foul" or event_type == "yellow_card" or event_type == "red_card") and details.get("foul_details") is Dictionary:
		var foul: Dictionary = details.get("foul_details") as Dictionary
		var severity := str(foul.get("severity", "")).strip_edges()
		if severity != "":
			pieces.append(severity.replace("_", " ").to_lower().capitalize())
		if bool(foul.get("is_dogso", false)):
			pieces.append("DOGSO")

	return " · ".join(pieces)


func _rule_id_to_law_number(rule_id: String) -> int:
	var rid := rule_id.to_upper()
	if rid == "DURATION":
		return 7
	if rid == "KICKOFF":
		return 8
	if rid == "BALL_IN_OUT":
		return 9
	if rid == "GOAL":
		return 10
	if rid.begins_with("OFFSIDE"):
		return 11
	if rid.begins_with("FOUL") or rid in ["DOGSO", "SERIOUS_FOUL_PLAY", "VIOLENT_CONDUCT", "HANDBALL", "SIMULATION"]:
		return 12
	if rid in ["DIRECT_FREE_KICK", "INDIRECT_FREE_KICK"]:
		return 13
	if rid == "PENALTY_KICK":
		return 14
	if rid == "THROW_IN":
		return 15
	if rid == "GOAL_KICK":
		return 16
	if rid == "CORNER_KICK":
		return 17
	return 0


func _apply_rulebook_p2_overlays(marker: Dictionary) -> void:
	if not _field:
		return

	var event_json := str(marker.get("_event_json", "")).strip_edges()
	if event_json == "":
		return
	var parsed: Variant = JSON.parse_string(event_json)
	if not (parsed is Dictionary):
		return
	var event: Dictionary = parsed
	var details: Dictionary = {}
	if event.get("details") is Dictionary:
		details = event.get("details") as Dictionary

	var event_type := str(marker.get("event_type", "")).to_lower()
	if event_type == "":
		event_type = str(event.get("event_type", "")).to_lower()

	if event_type == "offside" and details.get("offside_details") is Dictionary:
		var offside: Dictionary = details.get("offside_details") as Dictionary
		var x_m = offside.get("offside_line_m", null)
		if x_m != null and _field.has_method("highlight_pitch_x_m"):
			_field.highlight_pitch_x_m(float(x_m))

		var track_id: Variant = event.get("player_track_id", null)
		if track_id == null:
			track_id = offside.get("passer_track_id", null)
		if track_id != null and _field.has_method("highlight_player_track_id"):
			_field.highlight_player_track_id(int(track_id))

	elif (event_type == "foul" or event_type == "yellow_card" or event_type == "red_card") and details.get("foul_details") is Dictionary:
		var foul: Dictionary = details.get("foul_details") as Dictionary
		var track_id: Variant = event.get("player_track_id", null)
		if track_id == null:
			track_id = foul.get("victim_track_id", null)
		if track_id != null and _field.has_method("highlight_player_track_id"):
			_field.highlight_player_track_id(int(track_id))

func _translate_or_default(key: String, fallback: String) -> String:
	var localized := tr(key)
	return localized if localized != key else fallback


func _extract_rosters(record: Dictionary) -> Dictionary:
	var candidate_keys := ["timeline_rosters", "rosters", _LEGACY_ROSTERS_KEY]
	for key in candidate_keys:
		var variant: Variant = record.get(key, null)
		if variant is Dictionary and not (variant as Dictionary).is_empty():
			return (variant as Dictionary).duplicate(true)
	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary and match_result_variant.has("rosters"):
		var rosters_variant: Variant = match_result_variant.get("rosters")
		if rosters_variant is Dictionary:
			return (rosters_variant as Dictionary).duplicate(true)
	var doc_variant: Variant = record.get("timeline_doc", null)
	if doc_variant is Dictionary and doc_variant.has("rosters"):
		var doc_rosters: Variant = doc_variant.get("rosters")
		if doc_rosters is Dictionary:
			return (doc_rosters as Dictionary).duplicate(true)
	var legacy_doc_variant: Variant = record.get(_LEGACY_DOC_KEY, record.get(_LEGACY_PAYLOAD_KEY, {}))
	if legacy_doc_variant is Dictionary and legacy_doc_variant.has("rosters"):
		var legacy_rosters: Variant = legacy_doc_variant.get("rosters")
		if legacy_rosters is Dictionary:
			return (legacy_rosters as Dictionary).duplicate(true)
	var raw_result_variant: Variant = record.get("raw_result", {})
	if raw_result_variant is Dictionary and raw_result_variant.has("rosters"):
		var raw_rosters: Variant = raw_result_variant.get("rosters")
		if raw_rosters is Dictionary:
			return (raw_rosters as Dictionary).duplicate(true)
	return {}


func _extract_minimap_metadata(record: Dictionary) -> Dictionary:
	var metadata: Dictionary = {}
	var direct_metadata_variant: Variant = record.get("timeline_metadata", null)
	if direct_metadata_variant is Dictionary and not (direct_metadata_variant as Dictionary).is_empty():
		metadata = (direct_metadata_variant as Dictionary).duplicate(true)
	else:
		var doc_variant: Variant = record.get("timeline_doc", null)
		if doc_variant is Dictionary and (doc_variant as Dictionary).has("match_info"):
			var match_info_variant: Variant = (doc_variant as Dictionary).get("match_info")
			if match_info_variant is Dictionary:
				metadata = (match_info_variant as Dictionary).duplicate(true)
		if metadata.is_empty():
			var legacy_doc_variant: Variant = record.get(_LEGACY_DOC_KEY, record.get(_LEGACY_PAYLOAD_KEY, {}))
			if legacy_doc_variant is Dictionary and (legacy_doc_variant as Dictionary).has("match_info"):
				var legacy_match_info_variant: Variant = (legacy_doc_variant as Dictionary).get("match_info")
				if legacy_match_info_variant is Dictionary:
					metadata = (legacy_match_info_variant as Dictionary).duplicate(true)
	var home_id_variant: Variant = metadata.get(
		"home_team_id", record.get("home_team_engine_id", record.get("home_team_id", 1))
	)
	var away_id_variant: Variant = metadata.get(
		"away_team_id", record.get("away_team_engine_id", record.get("away_team_id", 2))
	)
	metadata["home_team_id"] = int(home_id_variant)
	metadata["away_team_id"] = int(away_id_variant)
	if not metadata.has("home_name"):
		metadata["home_name"] = str(record.get("home_team_name", record.get("home_team", "Home")))
	if not metadata.has("away_name"):
		metadata["away_name"] = str(record.get("opponent_name", record.get("opponent", "Away")))

	var heat_sources: Array = [
		record.get("goal_heat_samples", null),
		record.get("match_result", {}).get("goal_heat_samples"),
		record.get("raw_result", {}).get("goal_heat_samples")
	]
	for source in heat_sources:
		if source is Array and not (source as Array).is_empty():
			metadata["goal_heat_samples"] = (source as Array).duplicate(true)
			break
	return metadata


func _extract_stored_events(record: Dictionary) -> Array:
	var sources: Array = []
	sources.append(record.get("stored_events", null))
	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary and match_result_variant.has("stored_events"):
		sources.append(match_result_variant.get("stored_events"))
	var doc_variant: Variant = record.get("timeline_doc", null)
	if doc_variant is Dictionary and (doc_variant as Dictionary).has("stored_events"):
		sources.append((doc_variant as Dictionary).get("stored_events"))
	var legacy_doc_variant: Variant = record.get(_LEGACY_DOC_KEY, record.get(_LEGACY_PAYLOAD_KEY, {}))
	if legacy_doc_variant is Dictionary and (legacy_doc_variant as Dictionary).has("stored_events"):
		sources.append((legacy_doc_variant as Dictionary).get("stored_events"))
	var raw_result_variant: Variant = record.get("raw_result", {})
	if raw_result_variant is Dictionary and raw_result_variant.has("stored_events"):
		sources.append(raw_result_variant.get("stored_events"))
	for source in sources:
		if source is Array and not (source as Array).is_empty():
			return (source as Array).duplicate(true)
	return []


func _extract_timeline_events(record: Dictionary) -> Array:
	var direct_variant: Variant = record.get("timeline_events", null)
	if direct_variant is Array and not (direct_variant as Array).is_empty():
		return (direct_variant as Array).duplicate(true)
	if direct_variant is String:
		var parsed_variant: Variant = JSON.parse_string(String(direct_variant))
		if parsed_variant is Array and not (parsed_variant as Array).is_empty():
			return (parsed_variant as Array).duplicate(true)

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
			if events_variant is String:
				var parsed_events: Variant = JSON.parse_string(String(events_variant))
				if parsed_events is Array and not (parsed_events as Array).is_empty():
					return (parsed_events as Array).duplicate(true)

	var fallback_variant: Variant = record.get("events", record.get(_LEGACY_EVENTS_KEY, []))
	if fallback_variant is Array:
		return (fallback_variant as Array).duplicate(true)
	if fallback_variant is String:
		var parsed_variant: Variant = JSON.parse_string(String(fallback_variant))
		if parsed_variant is Array:
			return (parsed_variant as Array).duplicate(true)
	return []


func _extract_timeline_markers(record: Dictionary) -> Array:
	var markers: Array = []
	var sources := _collect_timeline_sources(record)
	for source in sources:
		for entry in source:
			if entry is Dictionary:
				var marker := _convert_timeline_entry(entry as Dictionary)
				if not marker.is_empty():
					var idx := markers.size()
					marker["_marker_id"] = idx
					markers.append(marker)
	if markers.is_empty():
		var events_variant: Variant = record.get("events", record.get(_LEGACY_EVENTS_KEY, []))
		if events_variant is Array:
			for entry in events_variant:
				if entry is Dictionary:
					var converted := _convert_event_entry(entry as Dictionary)
					if not converted.is_empty():
						var idx_fallback := markers.size()
						converted["_marker_id"] = idx_fallback
						markers.append(converted)

	# BestMoment 마커 추가 (Rust API에서 생성된 하이라이트)
	var best_moments := _extract_best_moments(record)
	for moment in best_moments:
		var idx := markers.size()
		moment["_marker_id"] = idx
		markers.append(moment)

	return markers


## BestMoment 하이라이트 추출 (Rust get_best_moments API 또는 match_result.best_moments)
func _extract_best_moments(record: Dictionary) -> Array:
	var moments: Array = []

	# 1. 직접 best_moments 필드 체크
	var direct_variant: Variant = record.get("best_moments", null)
	if direct_variant is Array and not (direct_variant as Array).is_empty():
		return _convert_best_moments_array(direct_variant as Array)

	# 2. match_result 내부 체크
	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary:
		var mr_moments: Variant = match_result_variant.get("best_moments", null)
		if mr_moments is Array and not (mr_moments as Array).is_empty():
			return _convert_best_moments_array(mr_moments as Array)

	# 3. raw_result 내부 체크
	var raw_result_variant: Variant = record.get("raw_result", {})
	if raw_result_variant is Dictionary:
		var rr_moments: Variant = raw_result_variant.get("best_moments", null)
		if rr_moments is Array and not (rr_moments as Array).is_empty():
			return _convert_best_moments_array(rr_moments as Array)

	return moments


## BestMoment 배열을 타임라인 마커 형식으로 변환
func _convert_best_moments_array(best_moments: Array) -> Array:
	var markers: Array = []
	for moment in best_moments:
		if not (moment is Dictionary):
			continue
		var marker := _convert_best_moment_to_marker(moment as Dictionary)
		if not marker.is_empty():
			markers.append(marker)
	return markers


## 단일 BestMoment를 타임라인 마커로 변환
func _convert_best_moment_to_marker(moment: Dictionary) -> Dictionary:
	var start_time_ms := int(moment.get("start_time_ms", 0))
	var moment_type := str(moment.get("moment_type", "")).to_lower()
	var priority := int(moment.get("priority", 50))
	var minute := int(moment.get("minute", start_time_ms / 60000))
	var description := str(moment.get("description", ""))
	var is_home := bool(moment.get("is_home_team", false))

	# 마커 라벨 생성
	var label := _moment_type_to_label(moment_type)
	if description != "":
		label = "%s: %s" % [label, description]

	return {
		"time_ms": start_time_ms,
		"moment_type": moment_type,  # TimelineMarkers._color_for_marker에서 사용
		"event_type": moment_type,  # 기존 호환성
		"label": label,
		"priority": priority,
		"team_id": 0 if is_home else 1,
		"is_best_moment": true  # BestMoment 식별용
	}


## MomentType을 사람이 읽을 수 있는 라벨로 변환
func _moment_type_to_label(moment_type: String) -> String:
	match moment_type:
		"goal":
			return _translate_or_default("UI_MOMENT_GOAL", "Goal")
		"penalty":
			return _translate_or_default("UI_MOMENT_PENALTY", "Penalty")
		"redcard", "red_card":
			return _translate_or_default("UI_MOMENT_RED_CARD", "Red Card")
		"save":
			return _translate_or_default("UI_MOMENT_SAVE", "Save")
		"shotontarget", "shot_on_target":
			return _translate_or_default("UI_MOMENT_SHOT_ON_TARGET", "Shot on Target")
		"posthit", "post_hit":
			return _translate_or_default("UI_MOMENT_POST_HIT", "Post Hit")
		"barhit", "bar_hit":
			return _translate_or_default("UI_MOMENT_BAR_HIT", "Crossbar Hit")
		"keychance", "key_chance":
			return _translate_or_default("UI_MOMENT_KEY_CHANCE", "Key Chance")
		_:
			return moment_type.capitalize()


func _collect_timeline_sources(record: Dictionary) -> Array:
	var sources: Array = []
	var direct_variant: Variant = record.get("timeline", null)
	if direct_variant is Array:
		var direct_array: Array = direct_variant
		sources.append(direct_array.duplicate(true))
	var raw_timeline_variant: Variant = record.get("raw_timeline", null)
	if raw_timeline_variant is Array:
		var raw_timeline_array: Array = raw_timeline_variant
		sources.append(raw_timeline_array.duplicate(true))

	var match_result_variant: Variant = record.get("match_result", {})
	if match_result_variant is Dictionary:
		var match_result: Dictionary = match_result_variant
		var match_timeline_variant: Variant = match_result.get("timeline", null)
		if match_timeline_variant is Array:
			var match_timeline_array: Array = match_timeline_variant
			sources.append(match_timeline_array.duplicate(true))
		var legacy_payload_variant: Variant = match_result.get(_LEGACY_PAYLOAD_KEY, null)
		if legacy_payload_variant is Dictionary and legacy_payload_variant.has("timeline"):
			var legacy_timeline_variant: Variant = (legacy_payload_variant as Dictionary).get("timeline")
			if legacy_timeline_variant is Array:
				var legacy_timeline_array: Array = legacy_timeline_variant
				sources.append(legacy_timeline_array.duplicate(true))

	var raw_result_variant: Variant = record.get("raw_result", {})
	if raw_result_variant is Dictionary:
		var raw_result: Dictionary = raw_result_variant
		var raw_result_timeline_variant: Variant = raw_result.get("timeline", null)
		if raw_result_timeline_variant is Array:
			var raw_result_timeline_array: Array = raw_result_timeline_variant
			sources.append(raw_result_timeline_array.duplicate(true))

	var doc_variant: Variant = record.get("timeline_doc", null)
	if doc_variant is Dictionary:
		var timeline_doc: Dictionary = doc_variant
		if timeline_doc.has("timeline"):
			var doc_timeline_variant: Variant = timeline_doc.get("timeline")
			if doc_timeline_variant is Array:
				var doc_timeline_array: Array = doc_timeline_variant
				sources.append(doc_timeline_array.duplicate(true))

	var legacy_doc_variant: Variant = record.get(_LEGACY_DOC_KEY, record.get(_LEGACY_PAYLOAD_KEY, {}))
	if legacy_doc_variant is Dictionary:
		var legacy_doc: Dictionary = legacy_doc_variant
		if legacy_doc.has("timeline"):
			var legacy_doc_timeline_variant: Variant = legacy_doc.get("timeline")
			if legacy_doc_timeline_variant is Array:
				var legacy_doc_timeline_array: Array = legacy_doc_timeline_variant
				sources.append(legacy_doc_timeline_array.duplicate(true))

	return sources


func _convert_timeline_entry(entry: Dictionary) -> Dictionary:
	var seconds := _extract_seconds(entry)
	if seconds < 0.0:
		return {}
	var event_type := _MatchTimeFormatter.normalize_event_kind(
		str(entry.get("event_type", entry.get("kind", entry.get("type", ""))))
	)
	var label := _extract_label(entry).strip_edges()
	if label == "" or label == "Event" or label.to_lower() == event_type:
		var kind_label := _MatchTimeFormatter.format_event_kind_display(event_type)
		if kind_label != "":
			label = kind_label
	return {
		"time_ms": int(round(seconds * 1000.0)),
		"label": label,
		"team_id": _normalize_team_identifier(entry.get("team_id", entry.get("team", entry.get("team_label", "")))),
		"event_type": event_type,
		"_event_json": JSON.stringify(entry)
	}


func _convert_event_entry(entry: Dictionary) -> Dictionary:
	var seconds := _extract_seconds(entry)
	if seconds < 0.0 and entry.has("minute"):
		seconds = float(entry.get("minute", 0)) * 60.0
	if seconds < 0.0:
		return {}
	var event_type := _MatchTimeFormatter.normalize_event_kind(
		str(entry.get("type", entry.get("event", "")))
	)
	var base_label := _extract_label(entry).strip_edges()
	var decorated_label := _decorate_event_label(event_type, entry, base_label)
	var marker := {
		"time_ms": int(round(seconds * 1000.0)),
		"label": decorated_label,
		"team_id": _normalize_team_identifier(entry.get("team_id", entry.get("team", entry.get("team_label", "")))),
		"event_type": event_type,
		"_event_json": JSON.stringify(entry)
	}
	var extra_fields := [
		"from",
		"to",
		"target",
		"position",
		"receiver_id",
		"receiver_label",
		"pass_distance_m",
		"pass_force",
		"distance",
		"force",
		"ground",
		"is_ground",
		"is_clearance",
		"xg",
		"xg_value",
		"outcome",
		"result",
		"ball_speed",
		"ball_curve",
		"segment_distance_m",
		"dribble_distance_m",
		"speed_mps",
		"with_ball",
		"touches",
		"dribble_touches",
		"message",
		"message_key",
		"message_label",
		"communication_distance_m",
		"has_target",
		"communication_target",
		"direction",
		"direction_vector",
		"heading_target",
		"heading_angle_deg",
		"boundary_label",
		"player_id",
		"player_name",
		"player_label",
		"last_touch_player_id",
		"last_touch_team_id"
	]
	for field in extra_fields:
		if entry.has(field):
			marker[field] = entry.get(field)
	if not marker.has("player_label"):
		var inline_player := _format_inline_player(
			entry.get("player_label", entry.get("player_name", entry.get("player_id", "")))
		)
		if inline_player != "":
			marker["player_label"] = inline_player
		elif entry.has("player_id"):
			marker["player_label"] = _format_inline_player(entry.get("player_id"))
	return marker


func _extract_seconds(entry: Dictionary) -> float:
	if entry.has("timestamp_ms"):
		return float(entry.get("timestamp_ms", 0)) / 1000.0
	if entry.has("t"):
		return float(entry.get("t", 0.0))
	if entry.has("time"):
		return float(entry.get("time", 0.0))
	if entry.has("minute"):
		var minute_val := float(entry.get("minute", 0.0))
		var second_val := float(entry.get("second", entry.get("seconds", 0.0)))
		return minute_val * 60.0 + second_val
	return -1.0


func _extract_label(entry: Dictionary) -> String:
	var candidates := ["label", "text", "summary", "event_type", "type"]
	for key in candidates:
		if entry.has(key):
			var value := str(entry.get(key))
			if value.strip_edges() != "":
				return value
	return "Event"


func _normalize_team_identifier(value: Variant) -> int:
	if value is int:
		return value
	if value is float:
		return int(value)
	var label := str(value).strip_edges().to_lower()
	if label.is_valid_int():
		return int(label.to_int())
	match label:
		"home", "ally", "my team", "우리팀":
			return 0
		"away", "opponent", "enemy", "상대":
			return 1
	return -1


func _build_marker_index_map() -> void:
	_marker_index_map.clear()
	for i in range(_timeline_markers.size()):
		var marker = _timeline_markers[i]
		if marker is Dictionary:
			var marker_id := int(marker.get("_marker_id", i))
			_marker_index_map[marker_id] = i


func _populate_event_list() -> void:
	if not _event_list:
		return
	_suppress_event_list_signal = true
	_event_list.clear()
	_suppress_event_list_signal = false
	var show_panel := not _timeline_markers.is_empty()
	if show_panel:
		for marker in _timeline_markers:
			if not (marker is Dictionary):
				continue
			var label := str(marker.get("label", marker.get("event_type", "event")))
			var minute := float(marker.get("time_ms", 0)) / 60000.0
			var time_text := "%.1f'" % minute if minute > 0.0 else "%d ms" % int(marker.get("time_ms", 0))
			var team_id := int(marker.get("team_id", -1))
			var team_text := ""
			if team_id == 0:
				team_text = _translate_or_default("UI_TEAM_HOME", "HOME")
			elif team_id == 1:
				team_text = _translate_or_default("UI_TEAM_AWAY", "AWAY")
			var item_text := "%s %s" % [time_text, label]
			if team_text != "":
				item_text = "%s | %s" % [team_text, item_text]
			var index := _event_list.add_item(item_text)
			_event_list.set_item_metadata(index, marker)
			_event_list.set_item_tooltip(index, item_text)
	if _event_list:
		_event_list.visible = show_panel
	if _event_panel:
		_event_panel.visible = show_panel


func _update_field_snapshot(timestamp_ms: int) -> void:
	if _timeline_controller and _timeline_controller.has_method("get_standard_snapshot") and _has_payload:
		var snapshot: Variant = _timeline_controller.get_standard_snapshot(timestamp_ms)
		if snapshot is Dictionary and not snapshot.is_empty():
			_last_snapshot = snapshot.duplicate(true)
			if _field:
				_field.set_snapshot(_last_snapshot)
			_update_debug_panel(_last_snapshot)
			return
	if _field:
		_field.clear_snapshot()
	_last_snapshot = {}
	_update_debug_panel()


func _on_dev_toggle_toggled(pressed: bool) -> void:
	_dev_panel_visible = pressed
	if _dev_panel:
		_dev_panel.visible = pressed
	if pressed:
		_update_debug_panel(_last_snapshot)


func _update_debug_panel(snapshot: Dictionary = {}) -> void:
	if not _dev_panel_visible or not _dev_text:
		return
	var lines: Array = []
	lines.append("[b]Playback[/b]")
	var duration_total_ms := 0
	if _timeline_controller:
		duration_total_ms = int(_timeline_controller.position_total_duration_ms)
	var duration_seconds := float(duration_total_ms) / 1000.0
	lines.append("- Time: %.2fs / %.2fs" % [float(_current_timestamp) / 1000.0, duration_seconds])
	lines.append("- Speed: %.2fx" % _current_speed)
	var state_label := "STOPPED"
	if _playback_started and not _is_paused:
		state_label = "PLAYING"
	elif _is_paused:
		state_label = "PAUSED"
	lines.append("- State: %s" % state_label)

	if snapshot.has("ball"):
		var ball_vec := _vector_from_variant(snapshot.get("ball"))
		lines.append("[b]Ball[/b]: (%.1f, %.1f)" % [ball_vec.x, ball_vec.y])
	if snapshot.has("players") and snapshot.players is Dictionary:
		var player_dict: Dictionary = snapshot.players
		lines.append("[b]Players[/b]: %d tracks" % player_dict.size())
		var preview_count := 0
		for pid in player_dict.keys():
			if preview_count >= 3:
				break
			var entry: Variant = player_dict[pid]
			if entry is Dictionary:
				var pos_vec := _vector_from_variant(entry.get("position", Vector2.ZERO))
				var entry_state_label := str(entry.get("state", ""))
				lines.append("- #%s (%.1f, %.1f) %s" % [str(pid), pos_vec.x, pos_vec.y, entry_state_label])
				preview_count += 1
	else:
		lines.append("[b]Players[/b]: n/a")

	if _timeline_markers.size() > 0:
		var next_marker := _next_marker_after(_current_timestamp)
		if not next_marker.is_empty():
			lines.append(
				(
					"[b]Next Event[/b]: %s @ %.2fs"
					% [next_marker.get("label", "event"), float(next_marker.get("time_ms", 0)) / 1000.0]
				)
			)

	if not _last_marker.is_empty():
		lines.append(
			(
				"[b]Last Marker[/b]: %s @ %.2fs"
				% [_last_marker.get("label", "event"), float(_last_marker.get("time_ms", 0)) / 1000.0]
			)
		)

	_dev_text.text = "\n".join(lines)


func _vector_from_variant(value: Variant) -> Vector2:
	if value is Vector2:
		return value
	if value is Vector3:
		return Vector2(value.x, value.y)
	if value is Array and value.size() >= 2:
		return Vector2(float(value[0]), float(value[1]))
	if value is Dictionary:
		return Vector2(float(value.get("x", 0.0)), float(value.get("y", value.get("z", 0.0))))
	return Vector2.ZERO


func _has_point_value(value: Variant) -> bool:
	if value == null:
		return false
	return value is Vector2 or value is Vector3 or (value is Array and value.size() >= 2) or (value is Dictionary)


func _estimate_distance(from_value: Variant, to_value: Variant) -> float:
	if not _has_point_value(from_value) or not _has_point_value(to_value):
		return -1.0
	var from_vec := _vector_from_variant(from_value)
	var to_vec := _vector_from_variant(to_value)
	return from_vec.distance_to(to_vec)


func _qa_logging_enabled() -> bool:
	if ProjectSettings.has_setting(QA_LOG_SETTING):
		return bool(ProjectSettings.get_setting(QA_LOG_SETTING))
	return true


func _export_timeline_qa_snapshot(record: Dictionary) -> void:
	if not _qa_logging_enabled():
		return
	var summary := _summarize_events_for_qa(record)
	if summary.is_empty():
		return
	var qa_dir := ProjectSettings.globalize_path("user://qa_logs")
	DirAccess.make_dir_recursive_absolute(qa_dir)
	var stamp := Time.get_datetime_string_from_system()
	var safe_stamp := stamp.replace(":", "-").replace(" ", "_")
	var file_path := "%s/minimap_phase_d_%s.json" % [qa_dir, safe_stamp]
	var file := FileAccess.open(file_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(summary, "\t"))
		print("[MatchTimelinePanel][QA] Saved QA snapshot -> %s" % file_path)


func _summarize_events_for_qa(record: Dictionary) -> Dictionary:
	var stored_events := _extract_stored_events(record)
	if stored_events.is_empty():
		return {}
	var counts: Dictionary = {}
	var samples: Dictionary = {"communication": [], "header": [], "boundary": []}
	for event_variant in stored_events:
		if not (event_variant is Dictionary):
			continue
		var event_dict: Dictionary = event_variant
		var kind := str(event_dict.get("type", event_dict.get("kind", ""))).to_lower()
		if kind == "":
			continue
		counts[kind] = counts.get(kind, 0) + 1
		if samples.has(kind) and samples[kind].size() < 3:
			samples[kind].append(_build_qa_sample(kind, event_dict))
	var marker_summary := _collect_marker_summary_for_qa()
	var title_text: String = _title_label.text if _title_label else str(record.get("title", "Unknown"))
	var score_text: String = _score_label.text if _score_label else "N/A"
	return {
		"generated_at": Time.get_datetime_string_from_system(),
		"title": title_text,
		"score": score_text,
		"seed": record.get("seed", record.get("match_seed", 0)),
		"event_counts": counts,
		"samples": samples,
		"markers": marker_summary
	}


func _build_qa_sample(kind: String, event_dict: Dictionary) -> Dictionary:
	var base: Variant = event_dict.get("base", {})
	var sample: Dictionary = {
		"timestamp": event_dict.get("timestamp", base.get("t", 0.0)),
		"team_id": event_dict.get("team_id", base.get("team_id", -1)),
		"player_id": event_dict.get("player_id", base.get("player_id", 0))
	}
	match kind:
		"communication":
			sample["message"] = event_dict.get("message", event_dict.get("message_key", ""))
			sample["has_target"] = event_dict.has("target")
		"header":
			sample["direction"] = event_dict.get("direction", event_dict.get("direction_vector", null))
		"boundary":
			sample["position"] = event_dict.get("position", event_dict.get("at", null))
			sample["last_touch_player_id"] = event_dict.get("last_touch_player_id", event_dict.get("player_id", 0))
	return sample


func _collect_marker_summary_for_qa() -> Dictionary:
	var summary: Dictionary = {"total": _timeline_markers.size(), "communication": 0, "header": 0, "boundary": 0}
	for marker in _timeline_markers:
		if not (marker is Dictionary):
			continue
		var event_type := str(marker.get("event_type", "")).to_lower()
		if summary.has(event_type):
			summary[event_type] = int(summary.get(event_type, 0)) + 1
	if not _last_marker.is_empty():
		summary["last_marker_label"] = _last_marker.get("label", "")
	return summary


func _qa_screenshot_enabled() -> bool:
	if ProjectSettings.has_setting(QA_SCREENSHOT_SETTING):
		return bool(ProjectSettings.get_setting(QA_SCREENSHOT_SETTING))
	return false


func _schedule_timeline_screenshot() -> void:
	if not _qa_screenshot_enabled():
		return
	if _pending_screenshot:
		return
	_pending_screenshot = true
	call_deferred("_capture_timeline_screenshot")


func _capture_timeline_screenshot() -> void:
	await get_tree().process_frame
	await get_tree().process_frame
	var viewport := get_viewport()
	if not viewport:
		_pending_screenshot = false
		return
	var tex := viewport.get_texture()
	if tex == null:
		_pending_screenshot = false
		return
	var image := tex.get_image()
	if image == null or image.is_empty():
		_pending_screenshot = false
		return
	var qa_dir := ProjectSettings.globalize_path("user://qa_logs")
	DirAccess.make_dir_recursive_absolute(qa_dir)
	var stamp := Time.get_datetime_string_from_system()
	var safe_stamp := stamp.replace(":", "-").replace(" ", "_")
	var file_path := "%s/minimap_phase_d_%s.png" % [qa_dir, safe_stamp]
	var err := image.save_png(file_path)
	if err == OK:
		print("[MatchTimelinePanel][QA] Saved screenshot -> %s" % file_path)
	else:
		push_warning("[MatchTimelinePanel] Failed to save QA screenshot (%s)" % file_path)
	_pending_screenshot = false


func _next_marker_after(timestamp_ms: int) -> Dictionary:
	var closest: Dictionary = {}
	var diff := INF
	for marker in _timeline_markers:
		if not (marker is Dictionary):
			continue
		var marker_time := int(marker.get("time_ms", 0))
		if marker_time >= timestamp_ms and marker_time - timestamp_ms < diff:
			diff = marker_time - timestamp_ms
			closest = marker
	return closest


func _decorate_event_label(event_type: String, entry: Dictionary, base_label: String) -> String:
	var label := base_label.strip_edges()
	if label == "" or label == "Event" or label.to_lower() == event_type:
		var kind_label := _MatchTimeFormatter.format_event_kind_display(event_type)
		if kind_label != "":
			label = kind_label
	match event_type:
		"pass":
			label = _decorate_pass_label(entry, label, false)
		"through_ball":
			label = _decorate_pass_label(entry, "Through Ball", true)
		"shot":
			label = _decorate_shot_label(entry, label)
		"run":
			label = _decorate_run_label(entry, label, false)
		"dribble":
			label = _decorate_run_label(entry, "Dribble", true)
		"communication":
			label = _decorate_communication_label(entry, label)
		"header":
			label = _decorate_header_label(entry, label)
		"boundary":
			label = _decorate_boundary_label(entry, label)
		_:
			pass
	return label


func _decorate_pass_label(entry: Dictionary, base_label: String, is_through: bool) -> String:
	var segments: PackedStringArray = []
	var receiver_raw: Variant = entry.get("receiver_label", entry.get("receiver_id", entry.get("target_player", "")))
	var receiver_label := _format_inline_player(receiver_raw)
	if receiver_label != "":
		segments.append("→ %s" % receiver_label)
	var distance_val := float(entry.get("pass_distance_m", entry.get("distance", -1.0)))
	if distance_val > 0.0:
		segments.append("%.1fm" % distance_val)
	var force_val := float(entry.get("pass_force", entry.get("force", -1.0)))
	if force_val > 0.0:
		segments.append("F%.1f" % force_val)
	var is_ground := bool(entry.get("ground", entry.get("is_ground", true)))
	if not is_ground:
		segments.append("Air")
	var outcome_text := str(entry.get("outcome", entry.get("pass_outcome", ""))).strip_edges()
	if outcome_text != "":
		segments.append(outcome_text.capitalize())
	if segments.is_empty():
		return base_label if not is_through else "Through Ball"
	var final_label := base_label if not is_through else "Through Ball"
	return "%s (%s)" % [final_label, ", ".join(segments)]


func _decorate_shot_label(entry: Dictionary, base_label: String) -> String:
	var segments: PackedStringArray = []
	var xg_val := float(entry.get("xg", entry.get("xg_value", -1.0)))
	if xg_val >= 0.0:
		segments.append("xG %.2f" % xg_val)
	var outcome_text := str(entry.get("outcome", entry.get("result", ""))).strip_edges()
	if outcome_text != "":
		segments.append(outcome_text.capitalize())
	var speed_val := float(entry.get("ball_speed", entry.get("speed", -1.0)))
	if speed_val > 0.0:
		segments.append("%.1f m/s" % speed_val)
	var curve_label := str(entry.get("ball_curve", "")).strip_edges()
	if curve_label != "":
		segments.append(curve_label.capitalize())
	if segments.is_empty():
		return base_label
	return "%s (%s)" % [base_label, ", ".join(segments)]


func _decorate_run_label(entry: Dictionary, base_label: String, is_dribble: bool) -> String:
	var segments: PackedStringArray = []
	var distance_val := float(entry.get("segment_distance_m", entry.get("distance", -1.0)))
	if distance_val <= 0.0:
		distance_val = float(entry.get("dribble_distance_m", -1.0))
	if distance_val > 0.0:
		segments.append("%.1fm" % distance_val)
	var speed_val := float(entry.get("speed_mps", entry.get("speed", -1.0)))
	if speed_val > 0.0:
		segments.append("%.1f m/s" % speed_val)
	if bool(entry.get("with_ball", false)):
		segments.append("with ball")
	var touches_val := int(entry.get("touches", entry.get("dribble_touches", 0)))
	if touches_val > 0:
		segments.append("%d touches" % touches_val)
	if segments.is_empty():
		return base_label if not is_dribble else "Dribble"
	var final_label := base_label if not is_dribble else "Dribble"
	return "%s (%s)" % [final_label, ", ".join(segments)]


func _decorate_communication_label(entry: Dictionary, base_label: String) -> String:
	var message_raw := str(entry.get("message_label", entry.get("message", entry.get("message_key", base_label))))
	var label := _format_comm_label(message_raw)
	if label == "":
		label = base_label.strip_edges()
	if label == "":
		label = "Communication"
	var segments: PackedStringArray = []
	var speaker := _format_inline_player(entry.get("player_label", entry.get("player_id", "")))
	if speaker != "":
		segments.append("%s" % speaker)
	var distance_val := float(entry.get("communication_distance_m", entry.get("distance", -1.0)))
	if distance_val > 0.0:
		segments.append("%.1fm" % distance_val)
	if bool(entry.get("has_target", entry.has("communication_target"))):
		segments.append(_translate_or_default("UI_TIMELINE_DETAIL_TARGETED", "Targeted"))
	if segments.is_empty():
		return label
	return "%s (%s)" % [label, ", ".join(segments)]


func _decorate_header_label(entry: Dictionary, base_label: String) -> String:
	var label := base_label.strip_edges()
	if label == "":
		label = "Header"
	var segments: PackedStringArray = []
	var player := _format_inline_player(entry.get("player_label", entry.get("player_id", "")))
	if player != "":
		segments.append(player)
	var angle_val := float(entry.get("heading_angle_deg", -999.0))
	if angle_val > -360.0:
		segments.append("%s°" % int(round(angle_val)))
	var distance_val := _estimate_distance(
		entry.get("from", null), entry.get("heading_target", entry.get("target", null))
	)
	if distance_val > 0.0:
		segments.append("%.1fm" % distance_val)
	if segments.is_empty():
		return label
	return "%s (%s)" % [label, ", ".join(segments)]


func _decorate_boundary_label(entry: Dictionary, base_label: String) -> String:
	var label := str(entry.get("boundary_label", base_label)).strip_edges()
	if label == "":
		label = "Boundary"
	var segments: PackedStringArray = []
	var player := _format_inline_player(
		entry.get("player_label", entry.get("player_id", entry.get("last_touch_player_id", "")))
	)
	if player != "":
		segments.append(player)
	if segments.is_empty():
		return label
	return "%s (%s)" % [label, ", ".join(segments)]


func _format_inline_player(value: Variant) -> String:
	if typeof(value) == TYPE_DICTIONARY:
		var dict: Dictionary = value
		if dict.has("name"):
			return str(dict.get("name"))
	var label := str(value).strip_edges()
	if label == "":
		return ""
	if label.is_valid_int():
		return "#%d" % label.to_int()
	return label


func _format_comm_label(raw_value: Variant) -> String:
	var text := str(raw_value).strip_edges()
	if text == "":
		return ""
	var normalized := text.replace("_", " ").replace("-", " ")
	var parts := normalized.split(" ", false)
	for i in range(parts.size()):
		var word := str(parts[i]).strip_edges()
		if word == "":
			continue
		parts[i] = _capitalize_word(word)
	return " ".join(parts)


func _capitalize_word(word: String) -> String:
	if word.length() == 0:
		return ""
	if word.length() == 1:
		return word.to_upper()
	return "%s%s" % [word.substr(0, 1).to_upper(), word.substr(1).to_lower()]


func _format_marker_detail(marker: Dictionary) -> String:
	var event_type := str(marker.get("event_type", "")).to_lower()
	var segments: PackedStringArray = []
	match event_type:
		"pass", "through_ball":
			var receiver := _format_inline_player(marker.get("receiver_label", marker.get("receiver_id", "")))
			if receiver != "":
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_RECEIVER", "Receiver: %s") % receiver)
			var distance_val := float(marker.get("pass_distance_m", marker.get("distance", -1.0)))
			if distance_val > 0.0:
				segments.append("%.1fm" % distance_val)
			var force_val := float(marker.get("pass_force", marker.get("force", -1.0)))
			if force_val > 0.0:
				segments.append("F%.1f" % force_val)
			var is_ground := bool(marker.get("ground", marker.get("is_ground", true)))
			if not is_ground:
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_AERIAL", "Aerial"))
			if bool(marker.get("is_clearance", false)):
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_CLEARANCE", "Clearance"))
			var outcome := str(marker.get("outcome", marker.get("pass_outcome", ""))).strip_edges()
			if outcome != "":
				segments.append(outcome.capitalize())
		"shot":
			var xg_val := float(marker.get("xg", marker.get("xg_value", -1.0)))
			if xg_val >= 0.0:
				segments.append("xG %.2f" % xg_val)
			var outcome := str(marker.get("outcome", marker.get("result", ""))).strip_edges()
			if outcome != "":
				segments.append(outcome.capitalize())
			var speed_val := float(marker.get("ball_speed", marker.get("speed", -1.0)))
			if speed_val > 0.0:
				segments.append("%.1f m/s" % speed_val)
			var curve := str(marker.get("ball_curve", "")).strip_edges()
			if curve != "":
				segments.append(curve.capitalize())
		"run", "dribble":
			var distance_val := float(
				marker.get("segment_distance_m", marker.get("distance", marker.get("dribble_distance_m", -1.0)))
			)
			if distance_val > 0.0:
				segments.append("%.1fm" % distance_val)
			var speed_val := float(marker.get("speed_mps", marker.get("speed", -1.0)))
			if speed_val > 0.0:
				segments.append("%.1f m/s" % speed_val)
			if bool(marker.get("with_ball", false)):
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_WITH_BALL", "With ball"))
			var touches := int(marker.get("touches", marker.get("dribble_touches", 0)))
			if touches > 0:
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_TOUCHES", "%d touches") % touches)
		"communication":
			var speaker := _format_inline_player(marker.get("player_label", marker.get("player_id", "")))
			if speaker != "":
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_SPEAKER", "Speaker: %s") % speaker)
			var message := _format_comm_label(
				marker.get("message_label", marker.get("message", marker.get("message_key", "")))
			)
			if message != "":
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_MESSAGE", "Message: %s") % message)
			var comm_distance := float(marker.get("communication_distance_m", marker.get("distance", -1.0)))
			if comm_distance > 0.0:
				segments.append("%.1fm" % comm_distance)
			if bool(marker.get("has_target", marker.has("communication_target"))):
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_TARGETED", "Targeted call"))
		"header":
			var header_player := _format_inline_player(marker.get("player_label", marker.get("player_id", "")))
			if header_player != "":
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_HEADER_BY", "Header: %s") % header_player)
			var angle := float(marker.get("heading_angle_deg", -999.0))
			if angle > -360.0:
				segments.append("%d°" % int(round(angle)))
			var header_distance := _estimate_distance(
				marker.get("from", null), marker.get("heading_target", marker.get("target", null))
			)
			if header_distance > 0.0:
				segments.append("%.1fm" % header_distance)
		"boundary":
			var restart := str(marker.get("boundary_label", "")).strip_edges()
			if restart != "":
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_RESTART", "Restart: %s") % restart)
			var toucher := _format_inline_player(
				marker.get("player_label", marker.get("player_id", marker.get("last_touch_player_id", "")))
			)
			if toucher != "":
				segments.append(_translate_or_default("UI_TIMELINE_DETAIL_LAST_TOUCH", "Last touch: %s") % toucher)
		_:
			pass
	if segments.is_empty():
		return ""
	return " • ".join(segments)
