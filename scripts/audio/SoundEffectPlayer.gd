extends Node
class_name SoundEffectPlayer

## Sound Effect Player
## Centralized sound effect management with placeholder support
## Created: 2025-10-25

# ============================================
# Sound Effect Types
# ============================================

enum SFX {
	# UI Sounds
	BUTTON_CLICK,
	BUTTON_HOVER,
	DIALOGUE_OPEN,
	DIALOGUE_CLOSE,
	CHOICE_SELECT,
	# Event Sounds
	EVENT_START,
	EVENT_END,
	AFFECTION_INCREASE,
	AFFECTION_DECREASE,
	# Typing
	TEXT_TYPE,
	TEXT_COMPLETE,
	# Notifications
	NOTIFICATION,
	WARNING,
	SUCCESS,
}

# ============================================
# Audio Players Pool
# ============================================

var audio_players: Array[AudioStreamPlayer] = []
var player_pool_size: int = 8
var current_player_index: int = 0

# ============================================
# Sound Effect Paths (Placeholders)
# ============================================

var sfx_paths = {
	SFX.BUTTON_CLICK: "res://assets/audio/sfx/button_click.ogg",
	SFX.BUTTON_HOVER: "res://assets/audio/sfx/button_hover.ogg",
	SFX.DIALOGUE_OPEN: "res://assets/audio/sfx/dialogue_open.ogg",
	SFX.DIALOGUE_CLOSE: "res://assets/audio/sfx/dialogue_close.ogg",
	SFX.CHOICE_SELECT: "res://assets/audio/sfx/choice_select.ogg",
	SFX.EVENT_START: "res://assets/audio/sfx/event_start.ogg",
	SFX.EVENT_END: "res://assets/audio/sfx/event_end.ogg",
	SFX.AFFECTION_INCREASE: "res://assets/audio/sfx/affection_up.ogg",
	SFX.AFFECTION_DECREASE: "res://assets/audio/sfx/affection_down.ogg",
	SFX.TEXT_TYPE: "res://assets/audio/sfx/text_type.ogg",
	SFX.TEXT_COMPLETE: "res://assets/audio/sfx/text_complete.ogg",
	SFX.NOTIFICATION: "res://assets/audio/sfx/notification.ogg",
	SFX.WARNING: "res://assets/audio/sfx/warning.ogg",
	SFX.SUCCESS: "res://assets/audio/sfx/success.ogg",
}

# ============================================
# Settings
# ============================================

var sfx_volume: float = 0.8  # 0.0 to 1.0
var sfx_enabled: bool = true

# ============================================
# Initialization
# ============================================


func _ready() -> void:
	_create_audio_player_pool()
	print("[SoundEffectPlayer] Initialized with %d audio players" % player_pool_size)


func _create_audio_player_pool() -> void:
	"""Create a pool of AudioStreamPlayer nodes for simultaneous sounds"""
	for i in range(player_pool_size):
		var player = AudioStreamPlayer.new()
		player.bus = "SFX"  # Assumes SFX audio bus exists
		add_child(player)
		audio_players.append(player)


# ============================================
# Play Functions
# ============================================


func play(sfx_type: SFX, volume_override: float = -1.0) -> void:
	"""Play a sound effect"""
	if not sfx_enabled:
		return

	var sfx_path = sfx_paths.get(sfx_type, "")

	# Check if audio file exists
	if not ResourceLoader.exists(sfx_path):
		# Placeholder: Use beep or print
		print("[SoundEffectPlayer] ⚠️ Missing SFX: %s (placeholder)" % sfx_path)
		_play_placeholder_beep(sfx_type)
		return

	# Load and play the sound
	var audio_stream = load(sfx_path)
	if not audio_stream:
		print("[SoundEffectPlayer] ❌ Failed to load: %s" % sfx_path)
		return

	# Get next available player from pool
	var player = audio_players[current_player_index]
	current_player_index = (current_player_index + 1) % player_pool_size

	# Set volume
	var volume = volume_override if volume_override >= 0.0 else sfx_volume
	player.volume_db = linear_to_db(volume)

	# Play
	player.stream = audio_stream
	player.play()


func _play_placeholder_beep(sfx_type: SFX) -> void:
	"""Play a simple beep as placeholder when audio file doesn't exist"""
	# Generate a simple sine wave beep
	var frequency = 440.0  # A4 note

	# Map different SFX types to different pitches
	match sfx_type:
		SFX.BUTTON_CLICK:
			frequency = 523.25  # C5
		SFX.BUTTON_HOVER:
			frequency = 659.25  # E5
		SFX.AFFECTION_INCREASE:
			frequency = 783.99  # G5
		SFX.AFFECTION_DECREASE:
			frequency = 329.63  # E4
		SFX.SUCCESS:
			frequency = 880.0  # A5
		SFX.WARNING:
			frequency = 293.66  # D4
		_:
			frequency = 440.0  # A4

	# Note: Godot doesn't easily support runtime audio generation
	# This is just a conceptual placeholder
	# In practice, we'd just skip the sound or use a generic beep.ogg


# ============================================
# Volume Control
# ============================================


func set_volume(volume: float) -> void:
	"""Set SFX volume (0.0 to 1.0)"""
	sfx_volume = clamp(volume, 0.0, 1.0)


func set_enabled(enabled: bool) -> void:
	"""Enable or disable sound effects"""
	sfx_enabled = enabled


func stop_all() -> void:
	"""Stop all currently playing sounds"""
	for player in audio_players:
		if player.playing:
			player.stop()
