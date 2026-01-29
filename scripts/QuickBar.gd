## QuickBar.gd (Godot 4.x)
extends Control
# Removed class_name to avoid global class conflict

signal skip
signal toggle_auto(state: bool)
signal change_speed(speed: int)
signal change_highlight(level: int)
signal open_log

@export var bar_position: String = "bottom-right"  # Renamed from 'position' to avoid conflict
@export var sticky: bool = true
@export var opacity: float = 0.95
@export var safe_area_inset: bool = true
@export var show_when: String = "interactive"
@export var buttons: Array[String] = ["SKIP", "AUTO", "FAST", "HL", "LOG"]
@export var speed_steps: PackedInt32Array = PackedInt32Array([1, 2, 4, 8])
@export var highlight_levels: PackedInt32Array = PackedInt32Array([1, 2, 3, 4])

var _auto_enabled: bool = false
var _current_speed: int = 1
var _highlight_level: int = 1

var _bar: PanelContainer
var _hbox: HBoxContainer
var _btn_skip: Button
var _btn_auto: Button
var _btn_fast: Button
var _btn_hl: Button
var _btn_log: Button

const ICONS := "res://ui/icons/"


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_PASS
	_build_ui()
	_apply_layout()


func _build_ui() -> void:
	_bar = PanelContainer.new()
	_bar.name = "QuickBar"
	_bar.modulate.a = opacity
	_bar.mouse_filter = Control.MOUSE_FILTER_PASS
	add_child(_bar)

	_hbox = HBoxContainer.new()
	_hbox.name = "Buttons"
	_hbox.add_theme_constant_override("separation", 8)
	_bar.add_child(_hbox)

	if "SKIP" in buttons:
		_btn_skip = _make_button("")
		_btn_skip.icon = _icon("fast.svg")
		_btn_skip.pressed.connect(func(): emit_signal("skip"))
	if "AUTO" in buttons:
		_btn_auto = _make_button("")
		_btn_auto.icon = _icon("auto.svg")
		_btn_auto.toggle_mode = true
		_btn_auto.pressed.connect(_on_auto_pressed)
	if "FAST" in buttons:
		_btn_fast = _make_button("")
		_apply_speed_icon()
		_btn_fast.pressed.connect(_on_fast_pressed)
	if "HL" in buttons:
		_btn_hl = _make_button("")
		_apply_hl_icon()
		_btn_hl.pressed.connect(_on_hl_pressed)
	if "LOG" in buttons:
		_btn_log = _make_button("")
		_btn_log.icon = _icon("log.svg")
		_btn_log.pressed.connect(func(): emit_signal("open_log"))


func _make_button(text: String) -> Button:
	var b := Button.new()
	b.text = text
	b.custom_minimum_size = Vector2(92, 56)
	_hbox.add_child(b)
	return b


func _apply_layout() -> void:
	var margin := 16.0
	_bar.anchor_left = 1.0
	_bar.anchor_top = 1.0
	_bar.anchor_right = 1.0
	_bar.anchor_bottom = 1.0
	_bar.offset_right = -margin
	_bar.offset_bottom = -margin
	_bar.offset_left = -1
	_bar.offset_top = -1

	if bar_position == "bottom-left":
		_bar.anchor_left = 0.0
		_bar.anchor_right = 0.0
		_bar.offset_left = margin
		_bar.offset_right = 0.0


func apply_view_model(vm_quickbar: Dictionary) -> void:
	if vm_quickbar.has("autoEnabled"):
		_auto_enabled = bool(vm_quickbar["autoEnabled"])
		if is_instance_valid(_btn_auto):
			_btn_auto.button_pressed = _auto_enabled
		emit_signal("toggle_auto", _auto_enabled)

	if vm_quickbar.has("currentSpeed"):
		_current_speed = int(vm_quickbar["currentSpeed"])
		_apply_speed_icon()
		emit_signal("change_speed", _current_speed)

	if vm_quickbar.has("highlightLevel"):
		_highlight_level = int(vm_quickbar["highlightLevel"])
		_apply_hl_icon()
		emit_signal("change_highlight", _highlight_level)

	if vm_quickbar.has("visible"):
		visible = bool(vm_quickbar["visible"])


func _on_auto_pressed() -> void:
	_auto_enabled = not _auto_enabled
	emit_signal("toggle_auto", _auto_enabled)


func _on_fast_pressed() -> void:
	var idx := speed_steps.find(_current_speed)
	idx = (idx + 1) % speed_steps.size()
	_current_speed = speed_steps[idx]
	_apply_speed_icon()
	emit_signal("change_speed", _current_speed)


func _on_hl_pressed() -> void:
	var idx := highlight_levels.find(_highlight_level)
	idx = (idx + 1) % highlight_levels.size()
	_highlight_level = highlight_levels[idx]
	_apply_hl_icon()
	emit_signal("change_highlight", _highlight_level)


func _apply_speed_icon() -> void:
	if not is_instance_valid(_btn_fast):
		return
	var icon_name := "speed%d.svg" % _current_speed
	_btn_fast.icon = _icon(icon_name)


func _apply_hl_icon() -> void:
	if not is_instance_valid(_btn_hl):
		return
	var icon_name := "hl%d.svg" % _highlight_level
	_btn_hl.icon = _icon(icon_name)


func _icon(name: String) -> Texture2D:
	var p := ICONS + name
	if ResourceLoader.exists(p):
		return load(p)
	return null
