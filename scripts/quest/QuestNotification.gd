extends Control
## Quest Notification Popup
## Shows in top-right corner for quest updates/completions
## Phase 4 implementation

# Node references
@onready var icon_label: Label = $Panel/MarginContainer/VBox/Header/Icon
@onready var title_label: Label = $Panel/MarginContainer/VBox/Header/Title
@onready var message_label: Label = $Panel/MarginContainer/VBox/Message
@onready var animation_player: AnimationPlayer = $AnimationPlayer

# Constants
const DISPLAY_DURATION: float = 3.0  # Seconds to display before auto-dismiss
const SLIDE_IN_DURATION: float = 0.4
const SLIDE_OUT_DURATION: float = 0.3

# Notification types
enum NotificationType { UPDATED, COMPLETED, UNLOCKED, FAILED }  # Quest progress updated  # Quest completed  # New quest unlocked  # Quest failed


func _ready() -> void:
	# Start hidden off-screen
	position.x = get_viewport_rect().size.x

	# Create animations
	_setup_animations()


## Show quest update notification
## @param quest_title: String - Quest name
## @param progress_text: String - e.g., "First Training (1/3)"
func show_update(quest_title: String, progress_text: String) -> void:
	icon_label.text = "📋"
	title_label.text = "Quest Updated"
	message_label.text = "%s\n%s" % [quest_title, progress_text]
	title_label.add_theme_color_override("font_color", Color(0.8, 0.8, 1.0))

	_display()


## Show quest completion notification
## @param quest_title: String - Quest name
func show_completion(quest_title: String) -> void:
	icon_label.text = "✅"
	title_label.text = "Quest Completed!"
	message_label.text = quest_title
	title_label.add_theme_color_override("font_color", Color(0.2, 1.0, 0.2))

	_display()


## Show quest unlocked notification
## @param quest_title: String - Quest name
func show_unlocked(quest_title: String) -> void:
	icon_label.text = "🔓"
	title_label.text = "New Quest Available"
	message_label.text = quest_title
	title_label.add_theme_color_override("font_color", Color(1.0, 0.9, 0.3))

	_display()


## Show quest failed notification
## @param quest_title: String - Quest name
func show_failed(quest_title: String) -> void:
	icon_label.text = "❌"
	title_label.text = "Quest Failed"
	message_label.text = quest_title
	title_label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3))

	_display()


func _setup_animations() -> void:
	# Wait for tree entry if needed
	if not is_inside_tree():
		await ready

	# Get viewport width with fallback
	var viewport_width: float = get_viewport_rect().size.x
	if viewport_width <= 0:
		viewport_width = 1920  # Fallback default

	# Slide in animation
	var slide_in := Animation.new()
	slide_in.length = SLIDE_IN_DURATION
	var track_idx := slide_in.add_track(Animation.TYPE_VALUE)
	slide_in.track_set_path(track_idx, ".:position:x")
	slide_in.track_insert_key(track_idx, 0.0, viewport_width)
	slide_in.track_insert_key(track_idx, SLIDE_IN_DURATION, viewport_width - size.x - 20)
	slide_in.track_set_interpolation_type(track_idx, Animation.INTERPOLATION_CUBIC)
	slide_in.track_set_interpolation_loop_wrap(track_idx, false)

	# Slide out animation
	var slide_out := Animation.new()
	slide_out.length = SLIDE_OUT_DURATION
	var track_idx2 := slide_out.add_track(Animation.TYPE_VALUE)
	slide_out.track_set_path(track_idx2, ".:position:x")
	slide_out.track_insert_key(track_idx2, 0.0, viewport_width - size.x - 20)
	slide_out.track_insert_key(track_idx2, SLIDE_OUT_DURATION, viewport_width)
	slide_out.track_set_interpolation_type(track_idx2, Animation.INTERPOLATION_CUBIC)
	slide_out.track_set_interpolation_loop_wrap(track_idx2, false)

	# Add animations to player
	var library := AnimationLibrary.new()
	library.add_animation("slide_in", slide_in)
	library.add_animation("slide_out", slide_out)
	animation_player.add_animation_library("", library)


func _display() -> void:
	# Slide in
	animation_player.play("slide_in")
	await animation_player.animation_finished

	# Wait for display duration (with safety check)
	if not is_inside_tree() or not get_tree():
		queue_free()
		return
	await get_tree().create_timer(DISPLAY_DURATION).timeout

	# Slide out
	animation_player.play("slide_out")
	await animation_player.animation_finished

	# Clean up
	queue_free()


## Handle click to dismiss early
func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed:
		# Stop current animation and force slide out
		if animation_player.is_playing():
			animation_player.stop()
		# Force slide out immediately
		animation_player.play("slide_out")
		await animation_player.animation_finished
		queue_free()
