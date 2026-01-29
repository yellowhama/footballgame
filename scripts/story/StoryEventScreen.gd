extends Control
## Full-screen Story Event viewer with character portraits, backgrounds, and choices
## Phase 2 implementation of Quest/Story UI system
## Phase 3: Dialogic integration with fallback to default UI

# Signals
signal choice_selected(event_id: String, choice_index: int)
signal screen_closed

# Node references
@onready var background: ColorRect = $Background
@onready var left_character: TextureRect = $CharacterLayer/LeftCharacter
@onready var center_character: TextureRect = $CharacterLayer/CenterCharacter
@onready var right_character: TextureRect = $CharacterLayer/RightCharacter
@onready var event_title: Label = $TextPanel/MarginContainer/VBox/Header/EventTitle
@onready var route_indicator: Label = $TextPanel/MarginContainer/VBox/Header/RouteIndicator
@onready var description: RichTextLabel = $TextPanel/MarginContainer/VBox/Description
@onready var choices_container: VBoxContainer = $TextPanel/MarginContainer/VBox/ChoicesContainer
@onready var animation_player: AnimationPlayer = $AnimationPlayer

# Animation constants
const FADE_IN_DURATION: float = 0.5
const FADE_OUT_DURATION: float = 0.3
const TYPEWRITER_SPEED: float = 0.05  # Seconds per character (optional)

# Character portrait constants
const CHARACTER_FADE_DURATION: float = 0.3

# Properties
var _current_event: Dictionary = {}
var _close_callback: Callable = Callable()
var _choice_buttons: Array[Button] = []

# Dialogic integration
var _dialogic_active: bool = false
var _dialogic_node: Node = null


func _ready() -> void:
	# Hide all character portraits by default
	left_character.modulate.a = 0.0
	center_character.modulate.a = 0.0
	right_character.modulate.a = 0.0

	# Fade in when ready
	_fade_in()


func _fade_in() -> void:
	var tween := create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "modulate:a", 1.0, FADE_IN_DURATION).from(0.0)


func _fade_out() -> void:
	var tween := create_tween()
	tween.tween_property(self, "modulate:a", 0.0, FADE_OUT_DURATION)
	await tween.finished


## Main entry point - Display a story event
func show_event(event: Dictionary) -> void:
	if event.is_empty():
		push_error("StoryEventScreen: Cannot show empty event")
		return

	_current_event = event

	# Check if Dialogic timeline exists for this event
	var event_id: String = event.get("id", "")
	var timeline_path: String = ""

	if StoryManager and StoryManager.has_method("get_dialogic_timeline"):
		timeline_path = StoryManager.get_dialogic_timeline(event_id)

	# If Dialogic is available and timeline exists, use Dialogic
	if not timeline_path.is_empty() and _is_dialogic_available():
		_start_dialogic_timeline(timeline_path)
	else:
		# Fall back to default UI
		_show_default_ui(event)


## Check if Dialogic addon is available
func _is_dialogic_available() -> bool:
	# Check if Dialogic singleton exists
	# NOTE: Dialogic addon must be installed and enabled in project settings
	return Engine.has_singleton("Dialogic") or ClassDB.class_exists("Dialogic")


## Start a Dialogic timeline
func _start_dialogic_timeline(timeline_path: String) -> void:
	print("[StoryEventScreen] Starting Dialogic timeline: %s" % timeline_path)

	# Hide default UI when using Dialogic
	$TextPanel.visible = false
	$CloseButton.visible = false

	_dialogic_active = true

	# NOTE: This code will work once Dialogic addon is installed
	# For now, it's ready but will fall back to default UI
	if Engine.has_singleton("Dialogic"):
		# Dialogic 2.x API
		var dialogic = Engine.get_singleton("Dialogic")
		if dialogic and dialogic.has_method("start"):
			dialogic.start(timeline_path)
			# Connect to timeline_ended signal (disconnect first to prevent duplicates)
			if dialogic.timeline_ended.is_connected(_on_dialogic_timeline_ended):
				dialogic.timeline_ended.disconnect(_on_dialogic_timeline_ended)
			dialogic.timeline_ended.connect(_on_dialogic_timeline_ended)
		else:
			push_warning("[StoryEventScreen] Dialogic singleton found but start() method missing")
			_show_default_ui(_current_event)
	else:
		push_warning("[StoryEventScreen] Dialogic not available, falling back to default UI")
		_show_default_ui(_current_event)


## Show default UI (non-Dialogic)
func _show_default_ui(event: Dictionary) -> void:
	_dialogic_active = false
	$TextPanel.visible = true
	$CloseButton.visible = true

	# Update all UI elements
	_update_background(event)
	_update_characters(event)
	_update_text(event)
	_create_choice_buttons(event.get("choices", []))


## Handle Dialogic timeline ended
func _on_dialogic_timeline_ended() -> void:
	print("[StoryEventScreen] Dialogic timeline ended")
	_dialogic_active = false

	# Show default UI for choices if event has choices
	if _current_event.has("choices") and _current_event["choices"].size() > 0:
		_show_default_ui(_current_event)
	else:
		# No choices, close the screen
		_close_screen()


## Set callback to invoke when screen closes
func set_close_callback(callback: Callable) -> void:
	_close_callback = callback


## Update background based on event or route
func _update_background(event: Dictionary) -> void:
	# Get background color/image from event or use route default
	var route: String = event.get("route", "Standard")

	# Default colors for each route
	match route:
		"Elite":
			background.color = Color(0.1, 0.05, 0.15, 1)  # Dark purple tint
		"Underdog":
			background.color = Color(0.08, 0.1, 0.05, 1)  # Dark green tint
		_:  # Standard
			background.color = Color(0.05, 0.05, 0.08, 1)  # Neutral dark

	# TODO: Load background texture if specified in event
	# if event.has("background_image"):
	#     var texture := load(event["background_image"]) as Texture2D
	#     if texture:
	#         background.texture = texture


## Update character portraits based on event data
func _update_characters(event: Dictionary) -> void:
	# Hide all characters first
	_hide_character(left_character)
	_hide_character(center_character)
	_hide_character(right_character)

	# Show characters specified in event
	var characters: Array = event.get("characters", [])
	for char_data in characters:
		var position: String = char_data.get("position", "center")
		var portrait_path: String = char_data.get("portrait", "")

		match position:
			"left":
				_show_character(left_character, portrait_path)
			"center":
				_show_character(center_character, portrait_path)
			"right":
				_show_character(right_character, portrait_path)


func _show_character(character_node: TextureRect, portrait_path: String) -> void:
	if portrait_path.is_empty():
		return

	# TODO: Load character portrait texture
	# var texture := load(portrait_path) as Texture2D
	# if texture:
	#     character_node.texture = texture

	# Fade in character
	var tween := create_tween()
	tween.tween_property(character_node, "modulate:a", 1.0, CHARACTER_FADE_DURATION)


func _hide_character(character_node: TextureRect) -> void:
	var tween := create_tween()
	tween.tween_property(character_node, "modulate:a", 0.0, CHARACTER_FADE_DURATION)


## Update title, route indicator, and description text
func _update_text(event: Dictionary) -> void:
	# Set event title
	var title: String = event.get("title", "Untitled Event")
	event_title.text = title

	# Set route indicator
	var route: String = event.get("route", "Standard")
	route_indicator.text = "[%s]" % route

	# Set route color
	match route:
		"Elite":
			route_indicator.add_theme_color_override("font_color", Color(0.9, 0.7, 1.0))
		"Underdog":
			route_indicator.add_theme_color_override("font_color", Color(0.7, 1.0, 0.7))
		_:
			route_indicator.add_theme_color_override("font_color", Color(0.9, 0.9, 0.9))

	# Set description
	var desc: String = event.get("description", "")
	description.text = desc

	# TODO: Implement typewriter effect (optional)
	# _start_typewriter_effect(desc)


## Create choice buttons from event choices array
func _create_choice_buttons(choices: Array) -> void:
	# Clear existing buttons
	for button in _choice_buttons:
		button.queue_free()
	_choice_buttons.clear()

	# Create new buttons for each choice
	for i in range(choices.size()):
		var choice_data: Dictionary = choices[i]
		var button := Button.new()

		# Button styling
		button.custom_minimum_size = Vector2(0, 60)
		button.add_theme_font_size_override("font_size", 18)

		# Button text
		var choice_text: String = choice_data.get("text", "Choice %d" % (i + 1))
		var available: bool = choice_data.get("available", true)

		if available:
			button.text = choice_text
			button.disabled = false
		else:
			var requirement: String = choice_data.get("requirement", "Locked")
			button.text = "%s (요구: %s)" % [choice_text, requirement]
			button.disabled = true

		# Connect button signal (use direct binding to avoid closure issues)
		button.pressed.connect(_on_choice_pressed.bind(i))

		# Add to container
		choices_container.add_child(button)
		_choice_buttons.append(button)

		# Add hover animation
		_setup_button_hover_effect(button)


## Setup button hover effect (scale up on hover)
func _setup_button_hover_effect(button: Button) -> void:
	button.mouse_entered.connect(
		func():
			var tween := create_tween()
			tween.tween_property(button, "scale", Vector2(1.05, 1.05), 0.1)
	)
	button.mouse_exited.connect(
		func():
			var tween := create_tween()
			tween.tween_property(button, "scale", Vector2(1.0, 1.0), 0.1)
	)


## Handle choice button press
func _on_choice_pressed(choice_index: int) -> void:
	if _current_event.is_empty():
		push_error("StoryEventScreen: No event loaded")
		return

	var event_id: String = _current_event.get("id", "")

	# Emit signal
	choice_selected.emit(event_id, choice_index)

	# Close screen after choice
	_close_screen()


## Handle close button press
func _on_close_button_pressed() -> void:
	_close_screen()


## Close the screen with fade out animation
func _close_screen() -> void:
	await _fade_out()

	# Emit screen_closed signal
	screen_closed.emit()

	# Call close callback if set
	if _close_callback.is_valid():
		_close_callback.call()


## Focus handling - renamed to avoid overriding Control.grab_focus()
func focus_first_choice() -> void:
	if _choice_buttons.size() > 0:
		_choice_buttons[0].grab_focus()


## Keyboard shortcut (ESC to close)
func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_ESCAPE:
			_close_screen()
			get_viewport().set_input_as_handled()
