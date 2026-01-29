extends PanelContainer
class_name PlayerRow

signal row_selected(player_id: String)

@onready var pos_rect: ColorRect = $HBox/PosRect
@onready var pos_label: Label = $HBox/PosRect/Label
@onready var name_label: Label = $HBox/NameLabel
@onready var role_label: Label = $HBox/RoleLabel
@onready var cond_bar: ProgressBar = $HBox/CondBar
@onready var ca_bar: ProgressBar = $HBox/CABar

var _player_id: String = ""

func _ready() -> void:
	gui_input.connect(_on_gui_input)

func setup(player: Dictionary) -> void:
	_player_id = str(player.get("id", ""))
	
	var pos = str(player.get("position", "??"))
	var p_name = str(player.get("name", "Unknown"))
	var cond = float(player.get("condition", 1.0)) * 100.0
	var ca = float(player.get("ca", 100))
	var role = str(player.get("role", ""))
	
	if pos_label: pos_label.text = pos
	if name_label: name_label.text = p_name
	if role_label: role_label.text = role
	if cond_bar: cond_bar.value = cond
	if ca_bar: ca_bar.value = ca
	
	if pos_rect:
		# Simple color coding based on position char
		var color = Color.GRAY
		if pos.begins_with("G"): color = Color(0.9, 0.7, 0.2) # GK - Gold/Yellow
		elif pos.begins_with("D"): color = Color(0.2, 0.6, 0.9) # Def - Blue
		elif pos.begins_with("M") or pos == "DM" or pos == "AM": color = Color(0.2, 0.8, 0.4) # Mid - Green
		elif pos.begins_with("F") or pos == "ST" or pos == "RW" or pos == "LW": color = Color(0.9, 0.3, 0.3) # Fwd - Red
		
		pos_rect.color = color

func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			row_selected.emit(_player_id)
