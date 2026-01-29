## MatchViewScreen - Main UI for 3-Mode Viewing System
##
## Part of the 3-Mode Viewing System (Full Match / Highlight / Key Moment)
##
## Responsibilities:
## - Mode switching (Full/Highlight/Key Moment)
## - Clip navigation (Prev/Next)
## - Autopause integration with Career Player Mode
## - Visual feedback (notifications, transitions)
##
## Usage:
## ```gdscript
## # Switch to Highlight mode
## match_view_screen._on_mode_selected(ViewMode.HIGHLIGHT)
##
## # Enable Career mode autopause
## match_view_screen.enable_career_mode(controlled_track_id)
## ```

extends Control

# ============================================================================
# View Mode Enum
# ============================================================================

## View modes for match playback
## NOTE: Clip selection/filtering is SSOT in Rust (ClipReducer) via ClipProvider.
## UI only selects the mode ID and renders returned clips.
enum ViewMode { FULL, HIGHLIGHT, KEY_MOMENT }  ## Full playback / Highlights / Key moments

# ============================================================================
# State
# ============================================================================

## Current view mode
var current_mode: ViewMode = ViewMode.FULL

## Clip navigation state
var _current_clip_index: int = 0
var _clips: Array = []

## Career mode integration
var _career_mode_enabled: bool = false
var _controlled_track_id: int = -1
var _autopause_on_controlled: bool = false
var _is_paused: bool = false

## Current match ID
var _current_match_id: String = ""

# ============================================================================
# UI Nodes (will be set in scene)
# ============================================================================

## Mode toggle buttons
@onready var full_mode_btn: Button = $HUD/ModeToggle/FullButton
@onready var highlight_mode_btn: Button = $HUD/ModeToggle/HighlightButton
@onready var key_moment_mode_btn: Button = $HUD/ModeToggle/KeyMomentButton

## Clip navigation panel
@onready var clip_nav_panel: Control = $HUD/ClipNav
@onready var prev_clip_btn: Button = $HUD/ClipNav/PrevButton
@onready var next_clip_btn: Button = $HUD/ClipNav/NextButton
@onready var clip_info_label: Label = $HUD/ClipNav/InfoLabel

## Autopause toggle
@onready var autopause_toggle: CheckBox = $HUD/Settings/AutopauseCheckbox

## Autopause notification
@onready var autopause_notification: PanelContainer = $HUD/AutopauseNotification

# ============================================================================
# Lifecycle
# ============================================================================


func _ready() -> void:
	print("[MatchViewScreen] Initializing...")

	# Connect mode buttons
	if full_mode_btn:
		full_mode_btn.pressed.connect(_on_mode_selected.bind(ViewMode.FULL))
	if highlight_mode_btn:
		highlight_mode_btn.pressed.connect(_on_mode_selected.bind(ViewMode.HIGHLIGHT))
	if key_moment_mode_btn:
		key_moment_mode_btn.pressed.connect(_on_mode_selected.bind(ViewMode.KEY_MOMENT))

	# Connect clip navigation
	if prev_clip_btn:
		prev_clip_btn.pressed.connect(_on_prev_clip)
	if next_clip_btn:
		next_clip_btn.pressed.connect(_on_next_clip)

	# Connect autopause toggle
	if autopause_toggle:
		autopause_toggle.toggled.connect(_on_autopause_toggled)

	# Connect to ClipProvider signals
	ClipProvider.clips_loaded.connect(_on_clips_loaded)
	ClipProvider.clips_load_failed.connect(_on_clips_load_failed)

	# Connect to UnifiedFramePipeline for autopause snapshots (if available)
	var pipeline := get_node_or_null("/root/UnifiedFramePipeline")
	if pipeline and pipeline.has_signal("snapshot_ready"):
		pipeline.snapshot_ready.connect(_on_unified_snapshot)

	# Hide autopause notification initially
	if autopause_notification:
		autopause_notification.visible = false

	# Initial UI state
	_update_mode_ui()
	_update_clip_nav_ui()

	print("[MatchViewScreen] Initialized")


func _input(event: InputEvent) -> void:
	# Keyboard shortcuts for clip navigation
	if current_mode != ViewMode.FULL and _clips.size() > 0:
		if event.is_action_pressed("ui_left"):
			_on_prev_clip()
			get_viewport().set_input_as_handled()
		elif event.is_action_pressed("ui_right"):
			_on_next_clip()
			get_viewport().set_input_as_handled()


# ============================================================================
# Public API
# ============================================================================


## Set match ID for clip loading
func set_match_id(match_id: String) -> void:
	_current_match_id = match_id


## Enable Career Player Mode integration
func enable_career_mode(controlled_track_id: int) -> void:
	_career_mode_enabled = true
	_controlled_track_id = controlled_track_id
	print("[MatchViewScreen] Career mode enabled (player: %d)" % controlled_track_id)
	_update_mode_ui()


## Disable Career Player Mode integration
func disable_career_mode() -> void:
	_career_mode_enabled = false
	_controlled_track_id = -1
	_autopause_on_controlled = false
	print("[MatchViewScreen] Career mode disabled")
	_update_mode_ui()


# ============================================================================
# Mode Selection
# ============================================================================


func _on_mode_selected(mode: ViewMode) -> void:
        print("[MatchViewScreen] Mode selected: %s" % _mode_to_string(mode))    
        current_mode = mode

        # Load clips from ClipProvider
        var mode_str := _mode_to_api_string(mode)

        var match_id = _current_match_id
        if match_id.is_empty():
                match_id = "current_match"  # Fallback

        # v1.1: ClipProvider requires match_result_json (get_match_clips_from_result SSOT).
        # Best-effort seed from MatchTimelineHolder record when available.
        var holder := get_node_or_null("/root/MatchTimelineHolder")
        if holder and holder.has_method("get_timeline_data") and ClipProvider.has_method("set_match_result"):
                var record_variant: Variant = holder.get_timeline_data()
                if record_variant is Dictionary:
                        var record: Dictionary = record_variant
                        var mr_variant: Variant = record.get("match_result", {})
                        if mr_variant is Dictionary and not (mr_variant as Dictionary).is_empty():
                                ClipProvider.set_match_result(mr_variant)

        ClipProvider.load_clips_for_match(match_id, mode_str)

        # Update UI immediately
        _update_mode_ui()


func _mode_to_string(mode: ViewMode) -> String:
	match mode:
		ViewMode.FULL:
			return "Full Match"
		ViewMode.HIGHLIGHT:
			return "Highlight"
		ViewMode.KEY_MOMENT:
			return "Key Moment"
		_:
			return "Unknown"


func _mode_to_api_string(mode: ViewMode) -> String:
	match mode:
		ViewMode.FULL:
			return "full"
		ViewMode.HIGHLIGHT:
			return "highlight"
		ViewMode.KEY_MOMENT:
			return "key_moment"
		_:
			return "full"


# ============================================================================
# Clip Loading
# ============================================================================


func _on_clips_loaded(count: int) -> void:
	print("[MatchViewScreen] Clips loaded: %d" % count)

	_clips = ClipProvider.get_clips()
	_current_clip_index = 0

	if count > 0 and current_mode != ViewMode.FULL:
		_play_clip(_current_clip_index)

	_update_clip_nav_ui()


func _on_clips_load_failed(error: String) -> void:
	push_error("[MatchViewScreen] Failed to load clips: %s" % error)
	_clips.clear()
	_update_clip_nav_ui()


# ============================================================================
# Clip Navigation
# ============================================================================


func _on_prev_clip() -> void:
	if _current_clip_index > 0:
		_current_clip_index -= 1
		_play_clip(_current_clip_index)
		print("[MatchViewScreen] Previous clip: %d/%d" % [_current_clip_index + 1, _clips.size()])


func _on_next_clip() -> void:
	if _current_clip_index < _clips.size() - 1:
		_current_clip_index += 1
		_play_clip(_current_clip_index)
		print("[MatchViewScreen] Next clip: %d/%d" % [_current_clip_index + 1, _clips.size()])


func _play_clip(index: int) -> void:
	if index < 0 or index >= _clips.size():
		return

	var clip: Dictionary = _clips[index]
	var start_ms: int = clip.get("start_ms", 0)

	print(
		(
			"[MatchViewScreen] Playing clip %d: %s (%.2f)"
			% [index + 1, clip.get("description", "Unknown"), clip.get("chance_score", 0.0)]
		)
	)

        # Seek timeline to clip start
        if has_node("/root/MatchTimelineController"):
                var timeline_controller = get_node("/root/MatchTimelineController")
                if timeline_controller.has_method("play_clip_at"):
                        timeline_controller.play_clip_at(start_ms, 1.0)
                else:
                        if timeline_controller.has_method("seek_position_time"):
                                timeline_controller.seek_position_time(start_ms)
                        if timeline_controller.has_method("start_position_playback"):
                                timeline_controller.start_position_playback(1.0)  # 1.0x speed
        else:
                push_warning("[MatchViewScreen] MatchTimelineController not found")

	_update_clip_nav_ui()


# ============================================================================
# Autopause (Career Mode Integration)
# ============================================================================


func _on_autopause_toggled(enabled: bool) -> void:
	_autopause_on_controlled = enabled
	print("[MatchViewScreen] Autopause: %s" % ("enabled" if enabled else "disabled"))


func _on_unified_snapshot(_t_ms: int, snapshot: Dictionary) -> void:
	# Only autopause in Highlight/Key Moment modes with Career mode enabled
	if not _autopause_on_controlled or not _career_mode_enabled:
		return

	if current_mode == ViewMode.FULL:
		return

	# Check if controlled player has ball
	var ball_owner: int = snapshot.get("ball_owner_track_id", -1)

	if ball_owner == _controlled_track_id and not _is_paused:
		_pause_playback()
		_show_autopause_notification()


func _pause_playback() -> void:
	_is_paused = true

	if has_node("/root/MatchTimelineController"):
		var timeline_controller = get_node("/root/MatchTimelineController")
		if timeline_controller.has_method("pause_playback"):
			timeline_controller.pause_playback()

	print("[MatchViewScreen] Playback paused (autopause)")


func _resume_playback() -> void:
	_is_paused = false

	if has_node("/root/MatchTimelineController"):
		var timeline_controller = get_node("/root/MatchTimelineController")
		if timeline_controller.has_method("resume_playback"):
			timeline_controller.resume_playback()

	_hide_autopause_notification()
	print("[MatchViewScreen] Playback resumed")


func _show_autopause_notification() -> void:
	if autopause_notification:
		autopause_notification.visible = true

		# Auto-hide after 3 seconds
		await get_tree().create_timer(3.0).timeout
		if autopause_notification:
			autopause_notification.visible = false


func _hide_autopause_notification() -> void:
	if autopause_notification:
		autopause_notification.visible = false


# ============================================================================
# UI Updates
# ============================================================================


func _update_mode_ui() -> void:
	# Update button states (toggle appearance)
	if full_mode_btn:
		full_mode_btn.button_pressed = (current_mode == ViewMode.FULL)
	if highlight_mode_btn:
		highlight_mode_btn.button_pressed = (current_mode == ViewMode.HIGHLIGHT)
	if key_moment_mode_btn:
		key_moment_mode_btn.button_pressed = (current_mode == ViewMode.KEY_MOMENT)

	# Show/hide clip navigation panel
	if clip_nav_panel:
		clip_nav_panel.visible = (current_mode != ViewMode.FULL)

	# Show/hide autopause toggle (only in clip modes + Career mode)
	if autopause_toggle:
		autopause_toggle.visible = (current_mode != ViewMode.FULL and _career_mode_enabled)


func _update_clip_nav_ui() -> void:
	if current_mode == ViewMode.FULL:
		return

	var clip_count := _clips.size()

	# Update button states
	if prev_clip_btn:
		prev_clip_btn.disabled = (_current_clip_index == 0)
	if next_clip_btn:
		next_clip_btn.disabled = (_current_clip_index >= clip_count - 1)

	# Update clip info label
	if clip_info_label:
		if clip_count > 0:
			var clip: Dictionary = _clips[_current_clip_index]
			var desc: String = clip.get("description", "Unknown")
			var score: float = clip.get("chance_score", 0.0)
			clip_info_label.text = "Clip %d/%d: %s (%.2f)" % [_current_clip_index + 1, clip_count, desc, score]
		else:
			clip_info_label.text = "No clips"


# ============================================================================
# Debug Helpers
# ============================================================================


func debug_print_state() -> void:
	print("=== MatchViewScreen Debug ===")
	print("Current Mode: %s" % _mode_to_string(current_mode))
	print("Match ID: %s" % _current_match_id)
	print("Clip Count: %d" % _clips.size())
	print("Current Clip: %d" % (_current_clip_index + 1))
	print("Career Mode: %s" % ("enabled" if _career_mode_enabled else "disabled"))
	print("Autopause: %s" % ("enabled" if _autopause_on_controlled else "disabled"))
	print("============================")
