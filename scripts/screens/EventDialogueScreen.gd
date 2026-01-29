extends Control
class_name EventDialogueScreen

## EventDialogueScreen.gd
## Displays story events with dialogue and choice buttons
## Created: 2025-10-25
## Version: 1.0

# ============================================
# Signals
# ============================================

## Emitted when dialogue screen is closed
signal dialogue_closed

## Emitted when a choice is made
signal choice_selected(choice_id: String)

# ============================================
# UI References
# ============================================

@onready var background_overlay: ColorRect = $BackgroundOverlay
@onready var dialogue_panel: PanelContainer = $DialoguePanel
@onready var character_portrait: TextureRect = $DialoguePanel/VBox/PortraitContainer/Portrait
@onready var character_name_label: Label = $DialoguePanel/VBox/CharacterName
@onready var dialogue_text: RichTextLabel = $DialoguePanel/VBox/ScrollContainer/DialogueText
@onready var choices_container: VBoxContainer = $DialoguePanel/VBox/ChoicesContainer
@onready var continue_button: Button = $DialoguePanel/VBox/ContinueButton

# ============================================
# State
# ============================================

var current_event: Dictionary = {}
var current_dialogue_index: int = 0
var dialogue_lines: Array = []
var typing_speed: float = 0.03  # Seconds per character
var is_typing: bool = false
var typing_tween: Tween = null

# ============================================
# Initialization
# ============================================


func _ready() -> void:
	# Connect to EventManager
	if EventManager:
		EventManager.event_started.connect(_on_event_started)

	# Hide by default
	hide()

	# Setup buttons
	if continue_button:
		continue_button.pressed.connect(_on_continue_pressed)


# ============================================
# Event Display
# ============================================


func _on_event_started(event_data: Dictionary) -> void:
	"""EventManager에서 이벤트 시작 시 호출"""
	show_event(event_data)


func show_event(event_data: Dictionary) -> void:
	"""이벤트 표시"""
	current_event = event_data
	current_dialogue_index = 0
	dialogue_lines = event_data.get("dialogue", [])

	# Validate dialogue
	if dialogue_lines.is_empty():
		print("[EventDialogueScreen] ⚠️ No dialogue in event: ", event_data.get("event_id", ""))
		close_dialogue()
		return

	# Show screen with fade-in and scale animation
	show()
	modulate.a = 0.0

	# Also animate dialogue panel scale for pop-in effect
	if dialogue_panel:
		dialogue_panel.scale = Vector2(0.9, 0.9)

	# Parallel animations: fade in + scale up
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_CUBIC)
	tween.set_parallel(true)
	tween.tween_property(self, "modulate:a", 1.0, 0.3)

	if dialogue_panel:
		tween.tween_property(dialogue_panel, "scale", Vector2(1.0, 1.0), 0.3)

	# Display first dialogue line after animation
	await tween.finished
	_display_dialogue_line()


func _display_dialogue_line() -> void:
	"""현재 대화 라인 표시"""
	if current_dialogue_index >= dialogue_lines.size():
		# All dialogue shown, show choices or close
		_show_choices_or_close()
		return

	var line = dialogue_lines[current_dialogue_index]

	# Update character info
	var speaker = line.get("speaker", "")
	if character_name_label:
		character_name_label.text = _get_speaker_display_name(speaker)

	# Update portrait with entrance animation
	_update_portrait_with_animation(speaker)

	# Update dialogue text with typing effect
	var text = line.get("text", "")
	if dialogue_text:
		dialogue_text.text = text
		dialogue_text.visible_characters = 0
		dialogue_text.modulate.a = 1.0

		# Start typing animation
		_start_typing_effect(text)

	# Show continue button, hide choices
	if continue_button:
		continue_button.visible = true
	if choices_container:
		choices_container.visible = false


func _on_continue_pressed() -> void:
	"""Continue to next dialogue line"""
	current_dialogue_index += 1
	_display_dialogue_line()


func _show_choices_or_close() -> void:
	"""Show choices if available, otherwise close"""
	var choices = current_event.get("choices", [])

	if choices.is_empty():
		# No choices, just close
		_apply_automatic_effects()
		close_dialogue()
	else:
		# Show choice buttons
		_display_choices(choices)


func _apply_automatic_effects() -> void:
	"""Apply event effects that have no choices"""
	var effects = current_event.get("effects", {})
	if not effects.is_empty():
		_apply_effects(effects)


func _display_choices(choices: Array) -> void:
	"""Display choice buttons with animation"""
	if not choices_container:
		return

	# Hide continue button
	if continue_button:
		continue_button.visible = false

	# Clear existing choices
	for child in choices_container.get_children():
		child.queue_free()

	# Create choice buttons
	var button_delay = 0.0
	for choice in choices:
		var choice_id = choice.get("choice_id", "")
		var choice_text = choice.get("text", "")
		var effects = choice.get("effects", {})

		var btn = Button.new()
		btn.text = choice_text
		btn.custom_minimum_size = Vector2(0, 60)
		btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		btn.pressed.connect(_on_choice_pressed.bind(choice_id, choice_text, choice))

		# Add hover preview for affection changes
		btn.mouse_entered.connect(_on_choice_hover.bind(btn, effects))
		btn.mouse_exited.connect(_on_choice_unhover.bind(btn))

		# Set tooltip with effect preview
		btn.tooltip_text = _create_choice_preview(effects)

		# Start invisible
		btn.modulate.a = 0.0
		choices_container.add_child(btn)

		# Animate button appearance with staggered delay
		var btn_tween = create_tween()
		btn_tween.set_ease(Tween.EASE_OUT)
		btn_tween.set_trans(Tween.TRANS_BACK)
		btn_tween.tween_interval(button_delay)
		btn_tween.tween_property(btn, "modulate:a", 1.0, 0.3)

		button_delay += 0.1  # Stagger each button by 0.1 seconds

	choices_container.visible = true


func _on_choice_pressed(choice_id: String, choice_text: String, choice_data: Dictionary) -> void:
	"""Handle choice selection"""
	print("[EventDialogueScreen] Choice selected: %s (%s)" % [choice_id, choice_text])

	# Apply choice effects
	var effects = choice_data.get("effects", {})
	_apply_effects(effects)

	# Notify EventManager
	var _route = current_event.get("route", "")
	var _event_id = current_event.get("event_id", "")
	var _week = current_event.get("week", 0)

	# Process choice in EventManager
	if EventManager:
		EventManager.process_event_choice(choice_id)

	# Emit signal
	choice_selected.emit(choice_id)

	# Close dialogue
	close_dialogue()


func _apply_effects(effects: Dictionary) -> void:
	"""Apply event effects to game state"""
	# Affection changes
	var affection_change = effects.get("affection_change", 0)
	if affection_change != 0:
		var character_id = _get_character_id_from_route(current_event.get("route", ""))
		if EventManager and character_id:
			var current_affection = EventManager.character_affection.get(character_id, 50)
			EventManager.character_affection[character_id] = clamp(current_affection + affection_change, 0, 100)
			print(
				(
					"[EventDialogueScreen] Affection changed: %s %+d (%d → %d)"
					% [
						character_id,
						affection_change,
						current_affection,
						EventManager.character_affection[character_id]
					]
				)
			)

	# Set flags
	var flags = effects.get("flags", [])
	for flag in flags:
		if EventManager:
			EventManager.set_flag(flag, true)
			print("[EventDialogueScreen] Flag set: ", flag)

	# Stat changes
	var stat_changes = effects.get("stat_changes", {})
	if not stat_changes.is_empty() and PlayerData:
		_apply_stat_changes(stat_changes)


func _apply_stat_changes(stat_changes: Dictionary) -> void:
	"""Apply stat changes to PlayerData

	Supported formats:
	- "technical.Dribbling": 5
	- "physical.Pace": -3
	- "mental.Determination": 10
	- "goalkeeper.Reflexes": 2
	"""
	for stat_key in stat_changes:
		var change = stat_changes[stat_key]

		# Parse stat_key format: "category.StatName"
		var parts = stat_key.split(".")
		if parts.size() != 2:
			print("[EventDialogueScreen] ⚠️ Invalid stat key format: %s (expected 'category.StatName')" % stat_key)
			continue

		var category = parts[0]
		var stat_name = parts[1]

		# Get current value
		var current_value = PlayerData.get_stat(category, stat_name)
		if current_value == -1:
			print("[EventDialogueScreen] ⚠️ Stat not found: %s.%s" % [category, stat_name])
			continue

		# Calculate new value
		var new_value = current_value + change

		# Apply change
		PlayerData.set_stat(category, stat_name, new_value)

		print(
			(
				"[EventDialogueScreen] 📊 Stat changed: %s.%s %+d (%d → %d)"
				% [category, stat_name, change, current_value, new_value]
			)
		)


func close_dialogue() -> void:
	"""Close dialogue screen with fade-out animation"""
	# Fade out animation
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_CUBIC)
	tween.tween_property(self, "modulate:a", 0.0, 0.2)

	await tween.finished

	hide()
	dialogue_closed.emit()

	# Clear state
	current_event = {}
	current_dialogue_index = 0
	dialogue_lines = []

	# Reset modulate for next use
	modulate.a = 1.0


# ============================================
# Helper Functions
# ============================================


func _get_speaker_display_name(speaker: String) -> String:
	"""Convert speaker ID to display name"""
	match speaker:
		"rival_taeyoung", "kang_taeyang":
			return "강태양"
		"friend_minjun", "park_minjun":
			return "박민준"
		"coach_cheolsu", "kim_cheolsu":
			return "김철수 코치"
		"captain_seojun", "lee_seojun":
			return "이서준"
		"gk_jihun", "choi_jihun":
			return "최지훈"
		"player", "me":
			return PlayerData.player_name if PlayerData else "나"
		_:
			return speaker


func _get_character_id_from_route(route: String) -> String:
	"""Get character ID from route"""
	match route:
		"rival":
			return "rival_taeyoung"
		"friendship":
			return "friend_minjun"
		"mentor":
			return "coach_cheolsu"
		"captain":
			return "captain_seojun"
		"guardian":
			return "gk_jihun"
		_:
			return ""


func _update_portrait(speaker: String) -> void:
	"""Update character portrait"""
	if not character_portrait:
		return

	# Try to load portrait image
	var portrait_path = "res://assets/portraits/%s.png" % speaker

	if ResourceLoader.exists(portrait_path):
		# Load actual portrait texture
		var texture = load(portrait_path)
		if texture:
			character_portrait.texture = texture
			character_portrait.visible = true
			print("[EventDialogueScreen] Portrait loaded: %s" % speaker)
			return

	# Fallback: Use color-coded placeholder
	_create_placeholder_portrait(speaker)


func _update_portrait_with_animation(speaker: String) -> void:
	"""Update character portrait with entrance animation"""
	if not character_portrait:
		return

	# Start from small and fade
	character_portrait.scale = Vector2(0.7, 0.7)
	character_portrait.modulate.a = 0.0

	# Update portrait (loads image or creates placeholder)
	_update_portrait(speaker)

	# Animate entrance: scale up + fade in
	var portrait_tween = create_tween()
	portrait_tween.set_ease(Tween.EASE_OUT)
	portrait_tween.set_trans(Tween.TRANS_BACK)
	portrait_tween.set_parallel(true)
	portrait_tween.tween_property(character_portrait, "scale", Vector2(1.0, 1.0), 0.4)
	portrait_tween.tween_property(character_portrait, "modulate:a", 1.0, 0.3)


func _create_placeholder_portrait(speaker: String) -> void:
	"""Create color-coded placeholder portrait when image not available"""
	# Clear existing texture
	character_portrait.texture = null

	# Get character-specific color
	var portrait_color = _get_character_color(speaker)

	# Create a simple colored rectangle as placeholder
	# This requires creating a ColorRect child or using modulate
	character_portrait.modulate = portrait_color
	character_portrait.visible = true

	print("[EventDialogueScreen] Using placeholder portrait for: %s" % speaker)


func _get_character_color(speaker: String) -> Color:
	"""Get character-specific color for placeholder"""
	match speaker:
		"rival_taeyoung", "kang_taeyang":
			return Color(1.0, 0.3, 0.3)  # Red - Rival energy
		"friend_minjun", "park_minjun":
			return Color(0.3, 0.7, 1.0)  # Blue - Friendly
		"coach_cheolsu", "kim_cheolsu":
			return Color(0.9, 0.6, 0.2)  # Orange - Mentor warmth
		"captain_seojun", "lee_seojun":
			return Color(0.5, 0.3, 0.9)  # Purple - Leadership
		"gk_jihun", "choi_jihun":
			return Color(0.3, 0.9, 0.5)  # Green - Guardian/GK
		"player", "me":
			return Color(1.0, 1.0, 1.0)  # White - Player
		_:
			return Color(0.7, 0.7, 0.7)  # Gray - Unknown


# ============================================
# Typing Effect
# ============================================


func _start_typing_effect(text: String) -> void:
	"""Start typing animation for dialogue text"""
	if not dialogue_text:
		return

	# Cancel previous typing if any
	if typing_tween:
		typing_tween.kill()

	is_typing = true
	var total_chars = text.length()
	var duration = total_chars * typing_speed

	# Animate visible_characters from 0 to total
	typing_tween = create_tween()
	typing_tween.set_trans(Tween.TRANS_LINEAR)
	typing_tween.tween_property(dialogue_text, "visible_characters", total_chars, duration)

	await typing_tween.finished
	is_typing = false


func _skip_typing() -> void:
	"""Skip typing animation and show full text immediately"""
	if is_typing and typing_tween:
		typing_tween.kill()
		if dialogue_text:
			dialogue_text.visible_characters = -1  # Show all characters
		is_typing = false


func _input(event: InputEvent) -> void:
	"""Handle input for skipping typing"""
	if not visible or not is_typing:
		return

	# Skip typing on mouse click or space/enter key
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			_skip_typing()
	elif event is InputEventKey:
		if event.pressed and (event.keycode == KEY_SPACE or event.keycode == KEY_ENTER):
			_skip_typing()


# ============================================
# Choice Preview & Hover Effects
# ============================================


func _create_choice_preview(effects: Dictionary) -> String:
	"""Create preview text for choice effects"""
	var preview_lines = []

	# Affection change
	var affection = effects.get("affection_change", 0)
	if affection != 0:
		var sign = "+" if affection > 0 else ""
		var color = "green" if affection > 0 else "red"
		preview_lines.append("[color=%s]호감도 %s%d[/color]" % [color, sign, affection])

	# Flags
	var flags = effects.get("flags", [])
	if not flags.is_empty():
		preview_lines.append("플래그: %s" % ", ".join(flags))

	# Stats
	var stats = effects.get("stats", {})
	if not stats.is_empty():
		var stat_previews = []
		for stat_key in stats:
			var value = stats[stat_key]
			var sign = "+" if value > 0 else ""
			stat_previews.append("%s %s%d" % [stat_key, sign, value])
		preview_lines.append("스탯: %s" % ", ".join(stat_previews))

	if preview_lines.is_empty():
		return "효과 없음"

	return "\n".join(preview_lines)


func _on_choice_hover(btn: Button, effects: Dictionary) -> void:
	"""Handle mouse hover on choice button"""
	# Scale up slightly on hover
	var hover_tween = create_tween()
	hover_tween.set_ease(Tween.EASE_OUT)
	hover_tween.set_trans(Tween.TRANS_CUBIC)
	hover_tween.tween_property(btn, "scale", Vector2(1.05, 1.05), 0.15)

	# Highlight based on affection change
	var affection = effects.get("affection_change", 0)
	if affection > 0:
		btn.modulate = Color(0.8, 1.0, 0.8)  # Green tint
	elif affection < 0:
		btn.modulate = Color(1.0, 0.8, 0.8)  # Red tint
	else:
		btn.modulate = Color(0.9, 0.9, 1.0)  # Blue tint


func _on_choice_unhover(btn: Button) -> void:
	"""Handle mouse exit from choice button"""
	# Scale back to normal
	var unhover_tween = create_tween()
	unhover_tween.set_ease(Tween.EASE_OUT)
	unhover_tween.set_trans(Tween.TRANS_CUBIC)
	unhover_tween.tween_property(btn, "scale", Vector2(1.0, 1.0), 0.15)

	# Reset color
	unhover_tween.tween_property(btn, "modulate", Color(1.0, 1.0, 1.0), 0.15)
