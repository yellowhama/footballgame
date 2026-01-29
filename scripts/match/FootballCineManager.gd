extends Node
class_name FootballCineManager
## Broadcast Timeline Manager - Controls cinematic cameras and playback
##
## Manages highlight timeline playback with automatic camera switching,
## slow motion effects, and broadcast-style highlight presentation.
##
## Week 2 Implementation (T2.1-T2.7)

# ========================================
# 📷 Camera References
# ========================================
@export var cine_main: Camera3D  ## Main broadcast camera
@export var cine_ball: Camera3D  ## Ball-tracking camera
@export var cine_side: Camera3D  ## Side angle camera
@export var cine_top: Camera3D  ## Top/bird's eye camera

# ========================================
# 🎬 Playback Settings
# ========================================
@export var auto_play_on_load: bool = false
@export var loop_timeline: bool = false
@export var default_playback_speed: float = 1.0

## Camera transition settings (T3.3)
@export var enable_camera_transitions: bool = true
@export var transition_duration: float = 0.3  ## Fade duration in seconds
@export var transition_overlay: ColorRect = null  ## Optional ColorRect for fade effect

# ========================================
# 🎯 Internal State
# ========================================
var current_timeline: Dictionary = {}
var current_clip_index: int = -1
var playback_time: float = 0.0
var is_playing: bool = false
var playback_speed: float = 1.0
var active_camera: Camera3D = null

# Camera lookup table
var cameras: Dictionary = {}

# Camera transition state (T3.3)
var transition_active: bool = false
var transition_time: float = 0.0
var transition_from_camera: Camera3D = null
var transition_to_camera: Camera3D = null

# ========================================
# 📊 Signals
# ========================================
signal timeline_loaded(timeline: Dictionary)
signal playback_started
signal playback_paused
signal playback_stopped
signal clip_started(clip: Dictionary, index: int)
signal clip_finished(clip: Dictionary, index: int)
signal timeline_finished
signal camera_switched(camera_name: String, camera: Camera3D)


# ========================================
# 🚀 Initialization (T2.1)
# ========================================
func _ready() -> void:
	_initialize_cameras()
	playback_speed = default_playback_speed
	_setup_eventbus_connections()
	_setup_transition_overlay()
	print("[FootballCineManager] ✅ Initialized")


## Initialize camera lookup table
func _initialize_cameras() -> void:
	cameras = {"Cine_Main": cine_main, "Cine_Ball": cine_ball, "Cine_Side": cine_side, "Cine_Top": cine_top}

	# Validate camera references
	var missing_cameras = []
	for camera_name in cameras:
		if cameras[camera_name] == null:
			missing_cameras.append(camera_name)
			push_warning("[FootballCineManager] ⚠️ Missing camera: %s" % camera_name)

	if missing_cameras.is_empty():
		print("[FootballCineManager] ✅ All 4 cameras initialized:")
		for camera_name in cameras:
			print("  • %s: %s" % [camera_name, cameras[camera_name].name if cameras[camera_name] else "NULL"])
	else:
		push_error(
			"[FootballCineManager] ❌ Missing %d cameras: %s" % [missing_cameras.size(), ", ".join(missing_cameras)]
		)

	# Set default active camera
	if cine_main:
		_activate_camera("Cine_Main")


# ========================================
# 📥 Timeline Loading (T2.2)
# ========================================


## Load timeline from JSON string
func load_timeline_json(timeline_json: String) -> bool:
	var timeline = JSON.parse_string(timeline_json)

	if not timeline:
		push_error("[FootballCineManager] ❌ Failed to parse timeline JSON")
		return false

	return load_timeline(timeline)


## Load timeline from dictionary
func load_timeline(timeline: Dictionary) -> bool:
	# Validate timeline structure
	if not _validate_timeline(timeline):
		return false

	# Stop current playback
	if is_playing:
		stop()

	# Load new timeline
	current_timeline = timeline
	current_clip_index = -1
	playback_time = 0.0

	print("[FootballCineManager] ✅ Timeline loaded:")
	print("  • Version: %s" % timeline.get("version", "unknown"))
	print("  • Match ID: %s" % timeline.get("match_id", "unknown"))
	print("  • Clips: %d" % timeline.clips.size())
	print("  • Duration: %.1fs" % timeline.metadata.total_duration)

	timeline_loaded.emit(timeline)

	if auto_play_on_load:
		play()

	return true


## Validate timeline structure (T2.2)
func _validate_timeline(timeline: Dictionary) -> bool:
	# Check required fields
	if not timeline.has("version"):
		push_error("[FootballCineManager] ❌ Timeline missing 'version' field")
		return false

	if not timeline.has("clips"):
		push_error("[FootballCineManager] ❌ Timeline missing 'clips' field")
		return false

	if not timeline.has("metadata"):
		push_error("[FootballCineManager] ❌ Timeline missing 'metadata' field")
		return false

	# Validate clips array
	if not timeline.clips is Array:
		push_error("[FootballCineManager] ❌ 'clips' must be an array")
		return false

	if timeline.clips.is_empty():
		push_warning("[FootballCineManager] ⚠️ Timeline has no clips")
		# Empty timeline is valid but not useful

	# Validate metadata
	var metadata = timeline.metadata
	if not metadata.has("total_duration"):
		push_error("[FootballCineManager] ❌ Metadata missing 'total_duration'")
		return false

	if not metadata.has("clip_count"):
		push_error("[FootballCineManager] ❌ Metadata missing 'clip_count'")
		return false

	# Validate each clip
	for i in range(timeline.clips.size()):
		var clip = timeline.clips[i]
		if not _validate_clip(clip, i):
			return false

	print("[FootballCineManager] ✅ Timeline validation passed")
	return true


## Validate individual clip structure
func _validate_clip(clip: Dictionary, index: int) -> bool:
	var required_fields = ["id", "clip_type", "start", "end", "camera", "importance", "event_id"]

	for field in required_fields:
		if not clip.has(field):
			push_error("[FootballCineManager] ❌ Clip %d missing field: %s" % [index, field])
			return false

	# Validate timing
	if clip.start < 0:
		push_error("[FootballCineManager] ❌ Clip %d has negative start time: %.1f" % [index, clip.start])
		return false

	if clip.end <= clip.start:
		push_error("[FootballCineManager] ❌ Clip %d has invalid timing: %.1f-%.1fs" % [index, clip.start, clip.end])
		return false

	# Validate camera
	if not cameras.has(clip.camera):
		push_warning("[FootballCineManager] ⚠️ Clip %d uses unknown camera: %s" % [index, clip.camera])
		# Not fatal - will use default camera

	return true


# ========================================
# ⏯️ Playback Control (T2.3)
# ========================================


## Start playback
func play() -> void:
	if current_timeline.is_empty():
		push_warning("[FootballCineManager] ⚠️ Cannot play - no timeline loaded")
		return

	if current_timeline.clips.is_empty():
		push_warning("[FootballCineManager] ⚠️ Cannot play - timeline has no clips")
		return

	is_playing = true

	# Start from beginning if finished or not started
	if current_clip_index == -1 or current_clip_index >= current_timeline.clips.size():
		current_clip_index = 0
		playback_time = current_timeline.clips[0].start

	print(
		(
			"[FootballCineManager] ▶️ Playback started (clip %d/%d)"
			% [current_clip_index + 1, current_timeline.clips.size()]
		)
	)

	playback_started.emit()


## Pause playback
func pause() -> void:
	if not is_playing:
		return

	is_playing = false
	print("[FootballCineManager] ⏸️ Playback paused at %.1fs" % playback_time)
	playback_paused.emit()


## Stop playback and reset
func stop() -> void:
	var was_playing := is_playing
	is_playing = false
	current_clip_index = -1
	playback_time = 0.0

	if was_playing:
		print("[FootballCineManager] ⏹️ Playback stopped")
		playback_stopped.emit()


## Skip to next clip
func next_clip() -> void:
	if current_timeline.clips.is_empty():
		return

	if current_clip_index < current_timeline.clips.size() - 1:
		current_clip_index += 1
		var clip = current_timeline.clips[current_clip_index]
		playback_time = clip.start
		_switch_camera(clip.camera)
		print("[FootballCineManager] ⏭️ Next clip: %s (%.1fs)" % [clip.clip_type, clip.start])


## Skip to previous clip
func previous_clip() -> void:
	if current_timeline.clips.is_empty():
		return

	if current_clip_index > 0:
		current_clip_index -= 1
		var clip = current_timeline.clips[current_clip_index]
		playback_time = clip.start
		_switch_camera(clip.camera)
		print("[FootballCineManager] ⏮️ Previous clip: %s (%.1fs)" % [clip.clip_type, clip.start])


## Set playback speed
func set_playback_speed(speed: float) -> void:
	playback_speed = clamp(speed, 0.1, 4.0)
	print("[FootballCineManager] ⚡ Playback speed: %.2fx" % playback_speed)


## Seek to specific time
func seek(time: float) -> void:
	seek_to_time(time)


## Seek to specific time (T3.1 - Timeline scrubbing)
func seek_to_time(time: float) -> void:
	if current_timeline.is_empty():
		return

	var total_duration = current_timeline.metadata.total_duration
	playback_time = clamp(time, 0.0, total_duration)

	# Find clip at this time
	for i in range(current_timeline.clips.size()):
		var clip = current_timeline.clips[i]
		if playback_time >= clip.start and playback_time < clip.end:
			if current_clip_index != i:
				current_clip_index = i
				_switch_camera(clip.camera)
				_apply_clip_effects(clip)
				clip_started.emit(clip, current_clip_index)
			break

	print("[FootballCineManager] 🔎 Seeked to %.1fs (clip %d)" % [playback_time, current_clip_index + 1])


## Seek to specific clip (T3.1)
func seek_to_clip(clip_index: int) -> void:
	if current_timeline.is_empty() or current_timeline.clips.is_empty():
		return

	# Clamp index to valid range
	clip_index = clamp(clip_index, 0, current_timeline.clips.size() - 1)

	var clip = current_timeline.clips[clip_index]
	current_clip_index = clip_index
	playback_time = clip.start

	_switch_camera(clip.camera)
	_apply_clip_effects(clip)
	clip_started.emit(clip, current_clip_index)

	print("[FootballCineManager] 🔎 Jumped to clip %d: %s" % [clip_index + 1, clip.clip_type])


## Frame-by-frame control (T3.4)
## @param direction: 1 for forward, -1 for backward
func step_frame(direction: int) -> void:
	if current_timeline.is_empty():
		return

	# Step by 1/30th of a second (30fps)
	var frame_time = 1.0 / 30.0
	seek_to_time(playback_time + (frame_time * direction))

	print("[FootballCineManager] 🎞️ Frame step %s: %.3fs" % ["forward" if direction > 0 else "backward", playback_time])


# ========================================
# 🎬 Playback Update (T2.3)
# ========================================


func _process(delta: float) -> void:
	# Always update camera transitions
	_update_camera_transition(delta)

	if not is_playing or current_timeline.is_empty():
		return

	# Update playback time
	playback_time += delta * playback_speed

	# Check if we need to switch clips
	_update_current_clip()


## Update current clip and handle transitions
func _update_current_clip() -> void:
	if current_clip_index < 0 or current_clip_index >= current_timeline.clips.size():
		return

	var clip = current_timeline.clips[current_clip_index]

	# Check if current clip finished
	if playback_time >= clip.end:
		clip_finished.emit(clip, current_clip_index)

		# Move to next clip
		if current_clip_index < current_timeline.clips.size() - 1:
			current_clip_index += 1
			var next_clip = current_timeline.clips[current_clip_index]
			playback_time = next_clip.start

			# Switch camera for new clip (T2.4)
			_switch_camera(next_clip.camera)

			# Apply effects (T2.5)
			_apply_clip_effects(next_clip)

			clip_started.emit(next_clip, current_clip_index)

			print(
				(
					"[FootballCineManager] 🎬 Clip %d/%d: %s (%.1f-%.1fs) [%s]"
					% [
						current_clip_index + 1,
						current_timeline.clips.size(),
						next_clip.clip_type,
						next_clip.start,
						next_clip.end,
						next_clip.camera
					]
				)
			)
		else:
			# Timeline finished
			_on_timeline_finished()


## Handle timeline completion
func _on_timeline_finished() -> void:
	print("[FootballCineManager] 🏁 Timeline finished")
	timeline_finished.emit()

	if loop_timeline:
		print("[FootballCineManager] 🔄 Looping timeline")
		current_clip_index = 0
		if not current_timeline.clips.is_empty():
			playback_time = current_timeline.clips[0].start
	else:
		stop()


# ========================================
# 📷 Camera Switching (T2.4)
# ========================================


## Switch to specified camera
func _switch_camera(camera_name: String) -> void:
	if not cameras.has(camera_name):
		push_warning("[FootballCineManager] ⚠️ Unknown camera: %s, using default" % camera_name)
		camera_name = "Cine_Main"

	var camera = cameras[camera_name]
	if camera == null:
		push_error("[FootballCineManager] ❌ Camera %s is null!" % camera_name)
		return

	_activate_camera(camera_name)


## Activate camera (make it current) with optional transition
func _activate_camera(camera_name: String) -> void:
	var camera = cameras.get(camera_name)
	if camera == null:
		return

	# Use transition if enabled
	if enable_camera_transitions and active_camera and active_camera != camera:
		_start_camera_transition(active_camera, camera)
	else:
		# Instant switch
		_complete_camera_switch(camera)

	camera_switched.emit(camera_name, camera)


## Get active camera name
func get_active_camera_name() -> String:
	for camera_name in cameras:
		if cameras[camera_name] == active_camera:
			return camera_name
	return "Unknown"


# ========================================
# ✨ Effects (T2.5)
# ========================================


## Apply clip effects (slow motion, etc.)
func _apply_clip_effects(clip: Dictionary) -> void:
	if not clip.has("effect"):
		# Reset to normal speed
		set_playback_speed(default_playback_speed)
		return

	var effect = clip.effect
	if effect == null:
		set_playback_speed(default_playback_speed)
		return

	match effect:
		"slowmo":
			set_playback_speed(0.5)
			print("[FootballCineManager] 🐌 Slow motion effect applied")
		"fastmo":
			set_playback_speed(2.0)
			print("[FootballCineManager] ⚡ Fast motion effect applied")
		_:
			push_warning("[FootballCineManager] ⚠️ Unknown effect: %s" % effect)
			set_playback_speed(default_playback_speed)


# ========================================
# 📊 Status & Debug (T2.7)
# ========================================


## Get current playback status
func get_status() -> Dictionary:
	if current_timeline.is_empty():
		return {"loaded": false, "playing": false}

	var current_clip = null
	if current_clip_index >= 0 and current_clip_index < current_timeline.clips.size():
		current_clip = current_timeline.clips[current_clip_index]

	return {
		"loaded": true,
		"playing": is_playing,
		"playback_time": playback_time,
		"playback_speed": playback_speed,
		"current_clip_index": current_clip_index,
		"total_clips": current_timeline.clips.size(),
		"current_clip": current_clip,
		"active_camera": get_active_camera_name(),
		"timeline_duration": current_timeline.metadata.total_duration
	}


## Print debug info
func debug_print_status() -> void:
	var status = get_status()
	print("\n[FootballCineManager] 📊 Status:")
	for key in status:
		if key != "current_clip":  # Skip detailed clip info
			print("  • %s: %s" % [key, str(status[key])])


# ========================================
# ✂️ Timeline Editing (T3.5)
# ========================================


## Trim clip start time
func trim_clip_start(clip_index: int, new_start: float) -> bool:
	if current_timeline.is_empty() or clip_index < 0 or clip_index >= current_timeline.clips.size():
		push_error("[FootballCineManager] ❌ Invalid clip index for trim: %d" % clip_index)
		return false

	var clip = current_timeline.clips[clip_index]

	# Validate new start time
	if new_start >= clip.end:
		push_error("[FootballCineManager] ❌ New start time must be before end time")
		return false

	if new_start < 0:
		push_error("[FootballCineManager] ❌ Start time cannot be negative")
		return false

	clip.start = new_start
	print("[FootballCineManager] ✂️ Trimmed clip %d start to %.1fs" % [clip_index, new_start])
	return true


## Trim clip end time
func trim_clip_end(clip_index: int, new_end: float) -> bool:
	if current_timeline.is_empty() or clip_index < 0 or clip_index >= current_timeline.clips.size():
		push_error("[FootballCineManager] ❌ Invalid clip index for trim: %d" % clip_index)
		return false

	var clip = current_timeline.clips[clip_index]

	# Validate new end time
	if new_end <= clip.start:
		push_error("[FootballCineManager] ❌ New end time must be after start time")
		return false

	clip.end = new_end
	print("[FootballCineManager] ✂️ Trimmed clip %d end to %.1fs" % [clip_index, new_end])
	return true


## Delete clip from timeline
func delete_clip(clip_index: int) -> bool:
	if current_timeline.is_empty() or clip_index < 0 or clip_index >= current_timeline.clips.size():
		push_error("[FootballCineManager] ❌ Invalid clip index for delete: %d" % clip_index)
		return false

	var clip = current_timeline.clips[clip_index]
	current_timeline.clips.remove_at(clip_index)

	# Update metadata
	current_timeline.metadata.clip_count = current_timeline.clips.size()

	# Adjust current clip index if needed
	if current_clip_index >= current_timeline.clips.size():
		current_clip_index = current_timeline.clips.size() - 1

	print("[FootballCineManager] 🗑️ Deleted clip %d: %s" % [clip_index, clip.clip_type])
	return true


## Move clip to new position in timeline
func move_clip(from_index: int, to_index: int) -> bool:
	if current_timeline.is_empty():
		return false

	if from_index < 0 or from_index >= current_timeline.clips.size():
		push_error("[FootballCineManager] ❌ Invalid from_index: %d" % from_index)
		return false

	if to_index < 0 or to_index >= current_timeline.clips.size():
		push_error("[FootballCineManager] ❌ Invalid to_index: %d" % to_index)
		return false

	var clip = current_timeline.clips[from_index]
	current_timeline.clips.remove_at(from_index)
	current_timeline.clips.insert(to_index, clip)

	# Update current clip index if needed
	if current_clip_index == from_index:
		current_clip_index = to_index
	elif current_clip_index > from_index and current_clip_index <= to_index:
		current_clip_index -= 1
	elif current_clip_index < from_index and current_clip_index >= to_index:
		current_clip_index += 1

	print("[FootballCineManager] 🔄 Moved clip from %d to %d" % [from_index, to_index])
	return true


## Update clip camera
func set_clip_camera(clip_index: int, camera_name: String) -> bool:
	if current_timeline.is_empty() or clip_index < 0 or clip_index >= current_timeline.clips.size():
		push_error("[FootballCineManager] ❌ Invalid clip index: %d" % clip_index)
		return false

	if not cameras.has(camera_name):
		push_error("[FootballCineManager] ❌ Unknown camera: %s" % camera_name)
		return false

	current_timeline.clips[clip_index].camera = camera_name
	print("[FootballCineManager] 📷 Set clip %d camera to %s" % [clip_index, camera_name])
	return true


## Update clip effect
func set_clip_effect(clip_index: int, effect: String) -> bool:
	if current_timeline.is_empty() or clip_index < 0 or clip_index >= current_timeline.clips.size():
		push_error("[FootballCineManager] ❌ Invalid clip index: %d" % clip_index)
		return false

	current_timeline.clips[clip_index].effect = effect
	print("[FootballCineManager] ⚡ Set clip %d effect to %s" % [clip_index, effect])
	return true


## Save modified timeline to JSON
func export_timeline_json() -> String:
	if current_timeline.is_empty():
		push_warning("[FootballCineManager] ⚠️ No timeline to export")
		return "{}"

	# Recalculate total duration
	var total_duration = 0.0
	for clip in current_timeline.clips:
		total_duration = max(total_duration, clip.end)

	current_timeline.metadata.total_duration = total_duration
	current_timeline.metadata.clip_count = current_timeline.clips.size()

	var json_string = JSON.stringify(current_timeline, "\t")
	print("[FootballCineManager] 💾 Exported timeline JSON (%d clips)" % current_timeline.clips.size())
	return json_string


# ========================================
# 🎞️ Camera Transitions (T3.3)
# ========================================


## Setup transition overlay
func _setup_transition_overlay() -> void:
	if not enable_camera_transitions:
		return

	if transition_overlay:
		# Hide overlay initially
		transition_overlay.color = Color(0, 0, 0, 0)
		transition_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
		print("[FootballCineManager] ✅ Transition overlay configured")
	else:
		push_warning("[FootballCineManager] ⚠️ Transition overlay not set - transitions disabled")
		enable_camera_transitions = false


## Start camera transition with fade effect
func _start_camera_transition(from_camera: Camera3D, to_camera: Camera3D) -> void:
	if not enable_camera_transitions or not transition_overlay:
		# Instant switch
		_complete_camera_switch(to_camera)
		return

	transition_active = true
	transition_time = 0.0
	transition_from_camera = from_camera
	transition_to_camera = to_camera

	print("[FootballCineManager] 🎬 Starting camera transition (%.1fms)" % (transition_duration * 1000.0))


## Update transition state
func _update_camera_transition(delta: float) -> void:
	if not transition_active:
		return

	transition_time += delta

	# Calculate fade alpha (0 → 1 → 0)
	var progress = transition_time / transition_duration
	var alpha: float

	if progress < 0.5:
		# Fade to black (first half)
		alpha = progress * 2.0  # 0 → 1
	else:
		# Fade from black (second half)
		alpha = (1.0 - progress) * 2.0  # 1 → 0

		# Switch camera at midpoint
		if transition_from_camera and transition_to_camera:
			_complete_camera_switch(transition_to_camera)
			transition_from_camera = null  # Only switch once

	# Apply fade
	transition_overlay.color = Color(0, 0, 0, clamp(alpha, 0.0, 1.0))

	# Complete transition
	if transition_time >= transition_duration:
		transition_active = false
		transition_overlay.color = Color(0, 0, 0, 0)
		print("[FootballCineManager] ✅ Camera transition complete")


## Complete camera switch (called during transition or instantly)
func _complete_camera_switch(camera: Camera3D) -> void:
	if camera == null:
		return

	# Activate target camera (automatically deactivates others)
	camera.make_current()
	active_camera = camera


# ========================================
# 🔌 EventBus Integration (T2.6)
# ========================================


## Setup EventBus connections
func _setup_eventbus_connections() -> void:
	if not EventBus:
		push_warning("[FootballCineManager] ⚠️ EventBus not available")
		return

	# Connect to EventBus signals
	timeline_loaded.connect(_on_timeline_loaded_eventbus)
	playback_started.connect(_on_playback_started_eventbus)
	playback_paused.connect(_on_playback_paused_eventbus)
	playback_stopped.connect(_on_playback_stopped_eventbus)
	clip_started.connect(_on_clip_started_eventbus)
	clip_finished.connect(_on_clip_finished_eventbus)
	timeline_finished.connect(_on_timeline_finished_eventbus)
	camera_switched.connect(_on_camera_switched_eventbus)

	print("[FootballCineManager] ✅ EventBus connections established")


## EventBus: Timeline loaded
func _on_timeline_loaded_eventbus(timeline: Dictionary) -> void:
	EventBus.emit(
		"broadcast_timeline_loaded",
		{
			"match_id": timeline.get("match_id", ""),
			"clip_count": timeline.metadata.clip_count,
			"duration": timeline.metadata.total_duration
		}
	)


## EventBus: Playback started
func _on_playback_started_eventbus() -> void:
	EventBus.emit(
		"broadcast_playback_started", {"clip_index": current_clip_index, "total_clips": current_timeline.clips.size()}
	)


## EventBus: Playback paused
func _on_playback_paused_eventbus() -> void:
	EventBus.emit("broadcast_playback_paused", {"time": playback_time})


## EventBus: Playback stopped
func _on_playback_stopped_eventbus() -> void:
	EventBus.emit("broadcast_playback_stopped", {})


## EventBus: Clip started
func _on_clip_started_eventbus(clip: Dictionary, index: int) -> void:
	EventBus.emit("broadcast_clip_started", {"clip": clip, "index": index, "total": current_timeline.clips.size()})


## EventBus: Clip finished
func _on_clip_finished_eventbus(clip: Dictionary, index: int) -> void:
	EventBus.emit("broadcast_clip_finished", {"clip": clip, "index": index})


## EventBus: Timeline finished
func _on_timeline_finished_eventbus() -> void:
	EventBus.emit(
		"broadcast_timeline_finished",
		{"total_clips": current_timeline.clips.size(), "duration": current_timeline.metadata.total_duration}
	)


## EventBus: Camera switched
func _on_camera_switched_eventbus(camera_name: String, camera: Camera3D) -> void:
	EventBus.emit("broadcast_camera_switched", {"camera": camera_name})
