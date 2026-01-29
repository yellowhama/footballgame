extends PanelContainer
class_name TrainingCard

signal selected(card_data: Dictionary)

@onready var icon_lbl: Label = $VBox/Header/Icon
@onready var title_lbl: Label = $VBox/Header/Title
@onready var desc_lbl: Label = $VBox/Description
@onready var bg_panel: StyleBoxFlat = get_theme_stylebox("panel").duplicate()

var _data: Dictionary = {}
var _is_selected: bool = false

func _ready() -> void:
	add_theme_stylebox_override("panel", bg_panel)
	gui_input.connect(_on_gui_input)

func setup(data: Dictionary) -> void:
	_data = data
	if icon_lbl: icon_lbl.text = data.get("icon", "")
	if title_lbl: title_lbl.text = data.get("name", "")
	if desc_lbl: desc_lbl.text = data.get("desc", "")

func get_data() -> Dictionary:
	return _data

func set_selected(val: bool) -> void:
	_is_selected = val
	if _is_selected:
		modulate = Color(1.0, 1.0, 1.0) # Full brightness
		bg_panel.bg_color = Color(0.2, 0.3, 0.5, 0.8) # Highlight color
		bg_panel.border_width_left = 2
		bg_panel.border_width_top = 2
		bg_panel.border_width_right = 2
		bg_panel.border_width_bottom = 2
		bg_panel.border_color = Color(0.4, 0.8, 1.0)
	else:
		modulate = Color(0.9, 0.9, 0.9) # Dimmed
		bg_panel.bg_color = Color(0.15, 0.15, 0.18, 0.8) # Default color
		bg_panel.border_width_left = 0
		bg_panel.border_width_top = 0
		bg_panel.border_width_right = 0
		bg_panel.border_width_bottom = 0

func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			selected.emit(_data)
