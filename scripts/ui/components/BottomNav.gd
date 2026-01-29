extends PanelContainer
class_name BottomNav

signal nav_requested(screen_id: String)

@onready var btn_home: Button = $HBoxContainer/NavBtn_Home
@onready var btn_training: Button = $HBoxContainer/NavBtn_Training
@onready var btn_match: Button = $HBoxContainer/NavBtn_Match
@onready var btn_status: Button = $HBoxContainer/NavBtn_Status
@onready var btn_menu: Button = $HBoxContainer/NavBtn_Menu

func _ready() -> void:
	_connect_signals()

func _connect_signals() -> void:
	btn_home.pressed.connect(func(): nav_requested.emit("Home"))
	btn_training.pressed.connect(func(): nav_requested.emit("Training"))
	btn_match.pressed.connect(func(): nav_requested.emit("Match"))
	btn_status.pressed.connect(func(): nav_requested.emit("Status"))
	btn_menu.pressed.connect(func(): nav_requested.emit("Menu"))

func set_active_tab(screen_id: String) -> void:
	# Reset all buttons to default state (e.g. unpressed look)
	# Highlighting logic can be added here
	pass
