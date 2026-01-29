extends PanelContainer
class_name ChoiceImpactPopup

## Choice Impact Popup
## Displays immediate effects of a player choice with sequential animations
## Auto-closes after 2-3 seconds

@onready var title_label: Label = $MarginContainer/VBox/TitleBar/Title
@onready var impact_list: VBoxContainer = $MarginContainer/VBox/ImpactList
@onready var auto_close_timer: Timer = $AutoCloseTimer

var impact_item_scene = preload("res://scenes/ui/ImpactItem.tscn")

## Animation settings
var fade_in_duration: float = 0.3
var item_delay: float = 0.2
var auto_close_delay: float = 2.0


func _ready():
	# Start hidden
	modulate.a = 0.0

	# Setup auto close timer
	auto_close_timer.one_shot = true
	auto_close_timer.timeout.connect(_on_auto_close_timer_timeout)


func show_impacts(impacts: Array[Dictionary]) -> void:
	"""
	Display choice impacts with sequential animations

	@param impacts: Array of impact dictionaries with structure:
		[
			{"type": "affection", "character": "kang_taeyang", "value": 20, "from": 60, "to": 80},
			{"type": "trait", "trait_name": "rival_awakening", "level": 1},
			{"type": "card", "card_id": "kang_taeyang_ssr_rival"},
			{"type": "ending", "ending_id": "rival_true_ending", "progress": 1, "total": 3},
			{"type": "stat", "stat_name": "Teamwork", "value": 15}
		]
	"""
	print("[ChoiceImpactPopup] Showing %d impacts" % impacts.size())

	# Clear existing items
	for child in impact_list.get_children():
		child.queue_free()

	# Fade in popup
	var fade_tween = create_tween()
	fade_tween.tween_property(self, "modulate:a", 1.0, fade_in_duration)
	await fade_tween.finished

	# Add impact items with sequential animation
	for i in range(impacts.size()):
		var impact = impacts[i]
		var item = impact_item_scene.instantiate()
		impact_list.add_child(item)

		# Set data
		item.set_impact_data(impact)

		# Start hidden
		item.modulate.a = 0.0

		# Wait for delay
		if i > 0:
			await get_tree().create_timer(item_delay).timeout

		# Fade in
		var item_tween = create_tween()
		item_tween.tween_property(item, "modulate:a", 1.0, fade_in_duration)

	# Calculate auto-close time (base + time per item)
	var total_time = auto_close_delay + (impacts.size() * item_delay)
	auto_close_timer.start(total_time)

	print("[ChoiceImpactPopup] Auto-close in %.1f seconds" % total_time)


func _on_auto_close_timer_timeout() -> void:
	"""Fade out and close popup"""
	print("[ChoiceImpactPopup] Auto-closing")

	# Fade out
	var fade_tween = create_tween()
	fade_tween.tween_property(self, "modulate:a", 0.0, 0.5)
	await fade_tween.finished

	# Remove from tree
	queue_free()


func close_immediately() -> void:
	"""Close popup without animation (for manual close)"""
	if auto_close_timer.time_left > 0:
		auto_close_timer.stop()
	queue_free()
