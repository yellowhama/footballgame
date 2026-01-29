extends PanelContainer
## Broadcast Timeline UI Panel
## Provides timeline scrubbing, playback controls, and clip information display

## References to FootballCineManager
@export var cine_manager: FootballCineManager

## UI Controls
@onready var progress_bar: ProgressBar = %ProgressBar
@onready var timeline_slider: HSlider = %TimelineSlider
@onready var clip_markers: Control = %ClipMarkers

@onready var stop_button: Button = %StopButton
@onready var prev_clip_button: Button = %PrevClipButton
@onready var frame_back_button: Button = %FrameBackButton
@onready var play_pause_button: Button = %PlayPauseButton
@onready var frame_forward_button: Button = %FrameForwardButton
@onready var next_clip_button: Button = %NextClipButton

@onready var speed_slider: HSlider = %SpeedSlider
@onready var speed_value: Label = %SpeedValue

@onready var clip_counter: Label = $MarginContainer/VBoxContainer/TimelineInfo/ClipCounter
@onready var time_display: Label = $MarginContainer/VBoxContainer/TimelineInfo/TimeDisplay
@onready var current_clip_label: Label = %CurrentClipLabel
@onready var camera_label: Label = %CameraLabel
@onready var effect_label: Label = %EffectLabel

## State
var is_dragging: bool = false
var clip_marker_colors: Dictionary = {
	"Goal": Color.GOLD,
	"Shot": Color.LIGHT_BLUE,
	"Foul": Color.ORANGE_RED,
	"Card": Color.RED,
	"Corner": Color.LIGHT_GREEN,
	"Pass": Color.LIGHT_GRAY,
	"highlight": Color.YELLOW,
	"re" + "play": Color.CYAN
}


func _ready() -> void:
	if not cine_manager:
		push_error("[BroadcastUIPanel] ❌ CineManager reference not set!")
		return

	# Connect UI controls
	_connect_controls()

	# Connect to CineManager signals
	_connect_cine_manager_signals()

	# Connect clip markers draw
	clip_markers.draw.connect(_on_clip_markers_draw)

	# Initial UI update
	_update_ui()


func _connect_controls() -> void:
	# Timeline scrubbing
	timeline_slider.value_changed.connect(_on_timeline_slider_changed)
	timeline_slider.drag_started.connect(_on_timeline_drag_started)
	timeline_slider.drag_ended.connect(_on_timeline_drag_ended)

	# Playback buttons
	stop_button.pressed.connect(_on_stop_pressed)
	prev_clip_button.pressed.connect(_on_prev_clip_pressed)
	frame_back_button.pressed.connect(_on_frame_back_pressed)
	play_pause_button.pressed.connect(_on_play_pause_pressed)
	frame_forward_button.pressed.connect(_on_frame_forward_pressed)
	next_clip_button.pressed.connect(_on_next_clip_pressed)

	# Speed control
	speed_slider.value_changed.connect(_on_speed_changed)


func _connect_cine_manager_signals() -> void:
	cine_manager.timeline_loaded.connect(_on_timeline_loaded)
	cine_manager.playback_started.connect(_on_playback_started)
	cine_manager.playback_paused.connect(_on_playback_paused)
	cine_manager.playback_stopped.connect(_on_playback_stopped)
	cine_manager.clip_started.connect(_on_clip_started)
	cine_manager.timeline_finished.connect(_on_timeline_finished)
	cine_manager.camera_switched.connect(_on_camera_switched)


func _process(_delta: float) -> void:
	if not is_dragging and cine_manager.is_playing:
		_update_ui()


# ========================================
# UI Update Functions
# ========================================


func _update_ui() -> void:
	var status = cine_manager.get_status()

	# Update progress bar and slider
	if status.loaded and status.timeline_duration > 0:
		var progress = status.playback_time / status.timeline_duration
		progress_bar.value = progress
		if not is_dragging:
			timeline_slider.value = progress
	else:
		progress_bar.value = 0
		timeline_slider.value = 0

	# Update time display
	time_display.text = "%s / %s" % [_format_time(status.playback_time), _format_time(status.timeline_duration)]

	# Update clip counter
	if status.loaded and status.total_clips > 0:
		clip_counter.text = "Clip %d/%d" % [status.current_clip_index + 1, status.total_clips]
	else:
		clip_counter.text = "Clip 0/0"

	# Update clip info
	if status.current_clip:
		var clip = status.current_clip
		current_clip_label.text = "🎬 %s (%.1fs)" % [clip.clip_type, clip.end - clip.start]
		camera_label.text = "📷 Camera: %s" % clip.camera

		if clip.has("effect") and clip.effect:
			effect_label.text = "⚡ Effect: %s" % clip.effect
		else:
			effect_label.text = "⚡ Effect: None"
	else:
		current_clip_label.text = "No clip active"
		camera_label.text = "📷 Camera: None"
		effect_label.text = "⚡ Effect: None"

	# Update play/pause button
	if status.playing:
		play_pause_button.text = "⏸️ Pause"
	else:
		play_pause_button.text = "▶️ Play"

	# Update speed display
	speed_value.text = "%.2fx" % status.playback_speed
	if not speed_slider.has_focus():
		speed_slider.value = status.playback_speed

	# Enable/disable controls
	var controls_enabled = status.loaded
	timeline_slider.editable = controls_enabled
	stop_button.disabled = not controls_enabled
	prev_clip_button.disabled = not controls_enabled
	frame_back_button.disabled = not controls_enabled
	play_pause_button.disabled = not controls_enabled
	frame_forward_button.disabled = not controls_enabled
	next_clip_button.disabled = not controls_enabled
	speed_slider.editable = controls_enabled


func _format_time(seconds: float) -> String:
	var minutes = int(seconds / 60.0)
	var secs = int(seconds) % 60
	var millis = int((seconds - int(seconds)) * 100)
	return "%d:%02d.%02d" % [minutes, secs, millis]


func _draw_clip_markers() -> void:
	if not cine_manager or not cine_manager.current_timeline:
		return

	clip_markers.queue_redraw()


func _on_clip_markers_draw() -> void:
	if not cine_manager or not cine_manager.current_timeline:
		return

	var timeline = cine_manager.current_timeline
	var duration = timeline.metadata.total_duration
	if duration <= 0:
		return

	var width = clip_markers.size.x
	var height = clip_markers.size.y

	# Draw clip boundaries
	for clip in timeline.clips:
		var start_x = (clip.start / duration) * width
		var end_x = (clip.end / duration) * width

		# Get color based on clip type
		var color = clip_marker_colors.get(clip.clip_type, Color.WHITE)
		color.a = 0.6

		# Draw clip region
		clip_markers.draw_rect(Rect2(start_x, 0, end_x - start_x, height), color)

		# Draw clip boundary lines
		clip_markers.draw_line(Vector2(start_x, 0), Vector2(start_x, height), Color.WHITE, 2.0)


# ========================================
# Control Handlers
# ========================================


func _on_timeline_slider_changed(value: float) -> void:
	if is_dragging:
		var duration = cine_manager.get_status().timeline_duration
		var target_time = value * duration
		cine_manager.seek_to_time(target_time)


func _on_timeline_drag_started() -> void:
	is_dragging = true


func _on_timeline_drag_ended(_value_changed: bool) -> void:
	is_dragging = false


func _on_stop_pressed() -> void:
	cine_manager.stop()


func _on_prev_clip_pressed() -> void:
	cine_manager.previous_clip()


func _on_frame_back_pressed() -> void:
	cine_manager.step_frame(-1)


func _on_play_pause_pressed() -> void:
	if cine_manager.is_playing:
		cine_manager.pause()
	else:
		cine_manager.play()


func _on_frame_forward_pressed() -> void:
	cine_manager.step_frame(1)


func _on_next_clip_pressed() -> void:
	cine_manager.next_clip()


func _on_speed_changed(value: float) -> void:
	cine_manager.set_playback_speed(value)


# ========================================
# CineManager Signal Handlers
# ========================================


func _on_timeline_loaded(_timeline: Dictionary) -> void:
	_draw_clip_markers()
	_update_ui()
	print("[BroadcastUIPanel] 📥 Timeline loaded")


func _on_playback_started() -> void:
	_update_ui()
	print("[BroadcastUIPanel] ▶️ Playback started")


func _on_playback_paused() -> void:
	_update_ui()
	print("[BroadcastUIPanel] ⏸️ Playback paused")


func _on_playback_stopped() -> void:
	_update_ui()
	print("[BroadcastUIPanel] ⏹️ Playback stopped")


func _on_clip_started(clip: Dictionary, index: int) -> void:
	_update_ui()
	print("[BroadcastUIPanel] 🎬 Clip %d started: %s" % [index + 1, clip.clip_type])


func _on_timeline_finished() -> void:
	_update_ui()
	print("[BroadcastUIPanel] 🏁 Timeline finished")


func _on_camera_switched(camera_name: String, _camera: Camera3D) -> void:
	_update_ui()
	print("[BroadcastUIPanel] 📷 Camera switched: %s" % camera_name)


# ========================================
# Public API
# ========================================


## Jump to specific clip by index
func jump_to_clip(clip_index: int) -> void:
	if not cine_manager:
		return

	cine_manager.seek_to_clip(clip_index)
	_update_ui()


## Set custom clip marker color
func set_clip_type_color(clip_type: String, color: Color) -> void:
	clip_marker_colors[clip_type] = color
	_draw_clip_markers()
