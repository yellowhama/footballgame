extends Control
class_name ReplayScreen

# Dependencies (Inputs)
# Assumes UnifiedFramePipeline is an Autoload or relevant singleton
# If not, it needs to be injected or accessed via a service locator.

@onready var timeline_canvas: TimelineCanvas = $ControlsLayer/PanelContainer/VBoxContainer/TimelineCanvas
@onready var btn_play_pause: Button = $ControlsLayer/PanelContainer/VBoxContainer/ButtonRow/PlayPauseBtn
@onready var btn_speed: Button = $ControlsLayer/PanelContainer/VBoxContainer/ButtonRow/SpeedToggle
@onready var btn_filter: Button = $ControlsLayer/PanelContainer/VBoxContainer/ButtonRow/FilterToggle

var _is_playing: bool = false
var _playback_speed_idx: int = 1 # Index for [0.5, 1, 2, 4]
const SPEED_MULTIPLIERS = [0.5, 1.0, 2.0, 4.0]

# Data Contracts
var match_events: Array = [] # List of MatchEvent (from MatchResult)

func _ready() -> void:
	_connect_signals()
	_update_ui_state()
	
	# Initialize Replay Mode
	if UnifiedFramePipeline:
		UnifiedFramePipeline.set_replay_mode(true)
		# Start paused by default?
		UnifiedFramePipeline.pause()
		_is_playing = false
		_update_ui_state()

func _connect_signals() -> void:
	# Timeline scrub
	if timeline_canvas:
		timeline_canvas.connect("timeline_scrubbed", _on_timeline_scrubbed)
	
	# Buttons
	if btn_play_pause:
		btn_play_pause.pressed.connect(_on_play_pause_toggled)
	if btn_speed:
		btn_speed.pressed.connect(_on_speed_toggled)
	if btn_filter:
		btn_filter.pressed.connect(_on_filter_toggled)

func _process(delta: float) -> void:
	if not visible:
		return
		
	# Synchronize playhead from Pipeline
	if UnifiedFramePipeline:
		# Read playhead directly from property
		var current_ms = UnifiedFramePipeline.playhead_ms
		var minutes = float(current_ms) / 60000.0
		
		if timeline_canvas:
			timeline_canvas.set_playhead(minutes)

# --- Public API for Data Injection ---
func setup(events: Array) -> void:
	match_events = events
	# In a real scenario, we might also push these events to the Pipeline 
	# if the Pipeline doesn't already have them. 
	# UnifiedFramePipeline.set_event_stream(events) 
	
	if timeline_canvas:
		timeline_canvas.set_events(match_events)

# --- Event Handlers ---

func _on_timeline_scrubbed(target_minute: float) -> void:
	var target_ms = int(target_minute * 60000.0)
	
	if UnifiedFramePipeline:
		UnifiedFramePipeline.set_playhead_ms(target_ms)
		UnifiedFramePipeline.force_update()
	
	if timeline_canvas:
		timeline_canvas.set_playhead(target_minute)

func _on_play_pause_toggled() -> void:
	_is_playing = not _is_playing
	
	if UnifiedFramePipeline:
		if _is_playing:
			UnifiedFramePipeline.resume()
		else:
			UnifiedFramePipeline.pause()
			
	_update_ui_state()

func _on_speed_toggled() -> void:
	_playback_speed_idx = (_playback_speed_idx + 1) % SPEED_MULTIPLIERS.size()
	var new_speed = SPEED_MULTIPLIERS[_playback_speed_idx]
	
	if UnifiedFramePipeline:
		UnifiedFramePipeline.set_playback_speed(new_speed)
	
	if btn_speed:
		btn_speed.text = str(new_speed) + "x"

var _filter_goals_only: bool = false

func _on_filter_toggled() -> void:
	_filter_goals_only = not _filter_goals_only
	
	if btn_filter:
		btn_filter.text = "Events: Goals" if _filter_goals_only else "Events: All"
	
	if timeline_canvas:
		if _filter_goals_only:
			var filtered = match_events.filter(func(e): return _is_important_event(e))
			timeline_canvas.set_events(filtered)
		else:
			timeline_canvas.set_events(match_events)

func _is_important_event(e: Variant) -> bool:
	var type = ""
	if e is Dictionary:
		type = str(e.get("event_type", "")).to_lower()
	elif e is Object and "event_type" in e:
		type = str(e.event_type).to_lower()
		
	return type in ["goal", "red_card", "penalty"]

func _update_ui_state() -> void:
	if btn_play_pause:
		btn_play_pause.text = "Pause" if _is_playing else "Play"
