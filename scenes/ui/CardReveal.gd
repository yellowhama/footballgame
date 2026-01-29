extends Control

# Card Reveal Animation Scene
class_name CardReveal

signal cards_revealed_complete

# Animation states
enum RevealState { WAITING, SHOWING_BACK, FLIPPING, SHOWING_FRONT, COMPLETE }

var current_state = RevealState.WAITING
var cards_to_reveal: Array = []
var current_card_index = 0

@onready var card_container = $CardContainer
@onready var card_back = $CardContainer/CardBack
@onready var card_front = $CardContainer/CardFront
@onready var rarity_effects = $RarityEffects
@onready var skip_button = $SkipButton
@onready var continue_button = $ContinueButton

# Card reveal animation timing
const CARD_APPEAR_TIME = 0.5
const CARD_FLIP_TIME = 1.0
const EFFECT_DURATION = 2.0


func _ready():
	continue_button.visible = false
	skip_button.visible = true
	card_front.visible = false


func reveal_cards(cards: Array):
	cards_to_reveal = cards
	current_card_index = 0

	if cards.is_empty():
		cards_revealed_complete.emit()
		return

	start_reveal_sequence()


func start_reveal_sequence():
	current_state = RevealState.SHOWING_BACK
	show_card_back()


func show_card_back():
	card_back.visible = true
	card_front.visible = false

	# Animate card appearing
	var tween = create_tween()
	card_container.scale = Vector2.ZERO
	tween.tween_property(card_container, "scale", Vector2.ONE, CARD_APPEAR_TIME).set_trans(Tween.TRANS_BACK).set_ease(
		Tween.EASE_OUT
	)
	tween.tween_callback(wait_for_tap)


func wait_for_tap():
	current_state = RevealState.FLIPPING
	# Wait for user to tap or auto-flip after delay


func flip_card():
	if current_card_index >= cards_to_reveal.size():
		return

	var card = cards_to_reveal[current_card_index]
	setup_card_front(card)

	# Flip animation
	var tween = create_tween()
	tween.tween_property(card_container, "scale:x", 0, CARD_FLIP_TIME / 2).set_trans(Tween.TRANS_QUAD).set_ease(
		Tween.EASE_IN
	)
	tween.tween_callback(swap_to_front)
	tween.tween_property(card_container, "scale:x", 1, CARD_FLIP_TIME / 2).set_trans(Tween.TRANS_QUAD).set_ease(
		Tween.EASE_OUT
	)
	tween.tween_callback(show_rarity_effect.bind(card))


func swap_to_front():
	card_back.visible = false
	card_front.visible = true


func setup_card_front(card: Dictionary):
	# Set up card display based on card data
	if card_front.has_node("NameLabel"):
		card_front.get_node("NameLabel").text = card.get("name", "Unknown Card")

	if card_front.has_node("RarityStars"):
		var rarity = card.get("rarity", 1)
		update_rarity_display(rarity)

	if card_front.has_node("CardImage"):
		# Load card image
		pass

	if card_front.has_node("TypeLabel"):
		card_front.get_node("TypeLabel").text = card.get("card_type", "Coach")


func update_rarity_display(rarity: int):
	var stars_node = card_front.get_node_or_null("RarityStars")
	if stars_node:
		for i in range(5):
			var star = stars_node.get_child(i)
			star.visible = i < rarity


func show_rarity_effect(card: Dictionary):
	var rarity = card.get("rarity", 1)

	match rarity:
		5:
			play_legendary_effect()
		4:
			play_epic_effect()
		3:
			play_rare_effect()
		2:
			play_common_effect()
		_:
			play_basic_effect()

	# Wait before showing next card or complete
	await get_tree().create_timer(EFFECT_DURATION).timeout
	next_card()


func play_legendary_effect():
	# Rainbow effect with particles
	if rarity_effects.has_node("LegendaryParticles"):
		rarity_effects.get_node("LegendaryParticles").emitting = true

	# Screen flash
	var flash = create_tween()
	modulate = Color.WHITE * 2.0
	flash.tween_property(self, "modulate", Color.WHITE, 0.5)


func play_epic_effect():
	# Purple glow effect
	if rarity_effects.has_node("EpicGlow"):
		rarity_effects.get_node("EpicGlow").visible = true


func play_rare_effect():
	# Blue shine effect
	if rarity_effects.has_node("RareShine"):
		rarity_effects.get_node("RareShine").visible = true


func play_common_effect():
	# Simple glow
	pass


func play_basic_effect():
	# No special effect
	pass


func next_card():
	current_card_index += 1

	if current_card_index < cards_to_reveal.size():
		# Reset and show next card
		card_front.visible = false
		show_card_back()
	else:
		# All cards revealed
		complete_reveal()


func complete_reveal():
	current_state = RevealState.COMPLETE
	skip_button.visible = false
	continue_button.visible = true


func _on_skip_button_pressed():
	# Skip to showing all cards at once
	show_all_cards_summary()


func _on_continue_button_pressed():
	cards_revealed_complete.emit()


func show_all_cards_summary():
	# Show all cards in a grid
	current_state = RevealState.COMPLETE
	skip_button.visible = false
	continue_button.visible = true

	# Create summary display
	# ... implementation for showing all cards at once


func _input(event):
	if current_state == RevealState.FLIPPING:
		if event.is_action_pressed("ui_accept") or (event is InputEventMouseButton and event.pressed):
			flip_card()
