extends PanelContainer
class_name TrainingCard

signal selected(card_data)

var _data: Dictionary = {}

@onready var icon_label = $VBox/Header/Icon
@onready var title_label = $VBox/Header/Title
@onready var effects_list = $VBox/EffectsList
@onready var cost_label = $VBox/CostLabel
@onready var select_button = $VBox/SelectButton


func _ready():
	select_button.pressed.connect(_on_select_pressed)

	# Hover effects
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)


func setup(data: Dictionary):
	_data = data

	if icon_label:
		icon_label.text = data.get("icon", "📋")

	if title_label:
		title_label.text = data.get("name", "Unknown Training")

	if cost_label:
		var cost = data.get("fatigue_cost", 0)
		cost_label.text = "⚡ -%d" % cost

	_setup_effects(data)


func _setup_effects(data: Dictionary):
	if not effects_list:
		return

	# Clear existing effects (except the placeholder if we want to keep it for preview, but better to clear)
	for child in effects_list.get_children():
		child.queue_free()

	# Add attributes if present
	var attributes = data.get("attributes", {})
	if attributes.size() > 0:
		for attr in attributes:
			_add_effect_label("✨ %s +%d" % [attr, attributes[attr]], Color(0.2, 0.8, 0.2))
	elif data.get("description"):
		# Fallback to description if no specific attributes
		_add_effect_label(data.get("description"), Color(0.7, 0.7, 0.7))


func _add_effect_label(text: String, color: Color):
	var label = Label.new()
	label.text = text
	label.add_theme_color_override("font_color", color)
	label.add_theme_font_size_override("font_size", 14)
	effects_list.add_child(label)


func _on_select_pressed():
	selected.emit(_data)


func _on_mouse_entered():
	# Simple hover effect: scale up slightly
	var tween = create_tween()
	tween.tween_property(self, "scale", Vector2(1.05, 1.05), 0.1)


func _on_mouse_exited():
	var tween = create_tween()
	tween.tween_property(self, "scale", Vector2(1.0, 1.0), 0.1)
