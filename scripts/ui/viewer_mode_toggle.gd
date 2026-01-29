extends HBoxContainer
class_name ViewerModeToggle

## Toggle control for switching between 2D and 3D match viewers.
## Emits signals when the user changes the view mode.

signal mode_changed(mode: ViewerMode)

enum ViewerMode {
	VIEWER_2D,
	VIEWER_3D,
}

@export var default_mode: ViewerMode = ViewerMode.VIEWER_2D
@export var button_group: ButtonGroup = null

var _current_mode: ViewerMode = ViewerMode.VIEWER_2D

@onready var _btn_2d: Button = $Button2D
@onready var _btn_3d: Button = $Button3D


func _ready() -> void:
	_setup_buttons()
	set_mode(default_mode, false)


func _setup_buttons() -> void:
	# Create button group if not provided
	if not button_group:
		button_group = ButtonGroup.new()

	# Configure 2D button
	if _btn_2d:
		_btn_2d.toggle_mode = true
		_btn_2d.button_group = button_group
		_btn_2d.text = "2D"
		_btn_2d.tooltip_text = "2D Field View (Top-down)"
		_btn_2d.pressed.connect(_on_2d_pressed)

	# Configure 3D button
	if _btn_3d:
		_btn_3d.toggle_mode = true
		_btn_3d.button_group = button_group
		_btn_3d.text = "3D"
		_btn_3d.tooltip_text = "3D Stadium View"
		_btn_3d.pressed.connect(_on_3d_pressed)


func get_mode() -> ViewerMode:
	return _current_mode


func set_mode(mode: ViewerMode, emit_signal: bool = true) -> void:
	_current_mode = mode

	match mode:
		ViewerMode.VIEWER_2D:
			if _btn_2d:
				_btn_2d.button_pressed = true
		ViewerMode.VIEWER_3D:
			if _btn_3d:
				_btn_3d.button_pressed = true

	if emit_signal:
		mode_changed.emit(mode)


func _on_2d_pressed() -> void:
	if _current_mode != ViewerMode.VIEWER_2D:
		_current_mode = ViewerMode.VIEWER_2D
		mode_changed.emit(ViewerMode.VIEWER_2D)
		print("[ViewerModeToggle] Switched to 2D view")


func _on_3d_pressed() -> void:
	if _current_mode != ViewerMode.VIEWER_3D:
		_current_mode = ViewerMode.VIEWER_3D
		mode_changed.emit(ViewerMode.VIEWER_3D)
		print("[ViewerModeToggle] Switched to 3D view")


func is_3d_mode() -> bool:
	return _current_mode == ViewerMode.VIEWER_3D


func is_2d_mode() -> bool:
	return _current_mode == ViewerMode.VIEWER_2D
