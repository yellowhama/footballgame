extends Control
class_name MenuScreen

signal closed

@onready var close_btn: Button = $Panel/VBox/TitleBar/CloseBtn
@onready var relay_btn: Button = $Panel/VBox/Content/ReplayBtn
@onready var settings_btn: Button = $Panel/VBox/Content/SettingsBtn
@onready var quit_btn: Button = $Panel/VBox/Content/QuitBtn

func _ready() -> void:
	if close_btn: close_btn.pressed.connect(_on_close_pressed)
	if relay_btn: relay_btn.pressed.connect(_on_replay_pressed)
	if quit_btn: quit_btn.pressed.connect(_on_quit_pressed)

func _on_close_pressed() -> void:
	# Just hide or pop
	if visible:
		hide()
	closed.emit()

func _on_replay_pressed() -> void:
	if UIManager:
		UIManager.change_screen("Replay")
	_on_close_pressed()

func _on_quit_pressed() -> void:
	get_tree().quit()
