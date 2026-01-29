extends Node
## Phase 12: Keyboard Navigation System
## Provides keyboard and gamepad navigation for UI

signal focus_changed(focused_control: Control)

@export var enable_gamepad: bool = true
@export var wrap_navigation: bool = true

var _current_focus: Control
var _focus_indicator: Control
var _navigable_controls: Array[Control] = []


func _ready():
	# Create focus indicator
	_focus_indicator = preload("res://scripts/components/FocusIndicator.gd").new()
	_focus_indicator.z_index = 999
	get_tree().root.call_deferred("add_child", _focus_indicator)

	# Connect to tree changes
	get_tree().node_added.connect(_on_node_added)
	get_tree().node_removed.connect(_on_node_removed)


func _unhandled_input(event):
	"""Handle keyboard/gamepad navigation input"""
	# Tab navigation
	if event.is_action_pressed("ui_focus_next"):
		navigate_next()
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("ui_focus_prev"):
		navigate_previous()
		get_viewport().set_input_as_handled()

	# Arrow key navigation
	elif event.is_action_pressed("ui_up"):
		navigate_direction(Vector2.UP)
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("ui_down"):
		navigate_direction(Vector2.DOWN)
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("ui_left"):
		navigate_direction(Vector2.LEFT)
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("ui_right"):
		navigate_direction(Vector2.RIGHT)
		get_viewport().set_input_as_handled()

	# Accept action
	elif event.is_action_pressed("ui_accept"):
		activate_focused()
		get_viewport().set_input_as_handled()


func navigate_next():
	"""Move focus to next control"""
	if _navigable_controls.is_empty():
		return

	var current_index = _navigable_controls.find(_current_focus)
	var next_index = (current_index + 1) % _navigable_controls.size()

	if not wrap_navigation and next_index == 0 and current_index >= 0:
		return  # Don't wrap

	set_focus(_navigable_controls[next_index])


func navigate_previous():
	"""Move focus to previous control"""
	if _navigable_controls.is_empty():
		return

	var current_index = _navigable_controls.find(_current_focus)
	var prev_index = current_index - 1

	if prev_index < 0:
		prev_index = _navigable_controls.size() - 1 if wrap_navigation else 0

	set_focus(_navigable_controls[prev_index])


func navigate_direction(direction: Vector2):
	"""Navigate in a specific direction"""
	if not _current_focus or _navigable_controls.is_empty():
		set_focus(_navigable_controls[0] if not _navigable_controls.is_empty() else null)
		return

	var current_pos = _current_focus.global_position + _current_focus.size / 2
	var best_control: Control = null
	var best_score = 9999999.0

	for control in _navigable_controls:
		if control == _current_focus:
			continue

		var target_pos = control.global_position + control.size / 2
		var delta = target_pos - current_pos

		# Check if control is roughly in the target direction
		var dot = delta.normalized().dot(direction)
		if dot < 0.5:  # Must be at least 45 degrees in direction
			continue

		# Score by distance and alignment
		var distance = delta.length()
		var score = distance / (dot + 0.1)  # Favor aligned targets

		if score < best_score:
			best_score = score
			best_control = control

	if best_control:
		set_focus(best_control)


func set_focus(control: Control):
	"""Set focus to a specific control"""
	if _current_focus == control:
		return

	_current_focus = control

	if _current_focus and _focus_indicator:
		_focus_indicator.attach_to(_current_focus)
	elif _focus_indicator:
		_focus_indicator.detach()

	focus_changed.emit(_current_focus)


func activate_focused():
	"""Activate the currently focused control"""
	if not _current_focus:
		return

	if _current_focus is Button:
		_current_focus.emit_signal("pressed")
	elif _current_focus is BaseButton:
		_current_focus.emit_signal("pressed")


func register_control(control: Control):
	"""Register a control for keyboard navigation"""
	if control in _navigable_controls:
		return

	if control is Button or control is BaseButton:
		_navigable_controls.append(control)
		_sort_controls()


func unregister_control(control: Control):
	"""Unregister a control from keyboard navigation"""
	var index = _navigable_controls.find(control)
	if index >= 0:
		_navigable_controls.remove_at(index)

		if _current_focus == control:
			_current_focus = null
			if _focus_indicator:
				_focus_indicator.detach()


func _sort_controls():
	"""Sort controls by position (top-to-bottom, left-to-right)"""
	_navigable_controls.sort_custom(
		func(a, b):
			var pos_a = a.global_position
			var pos_b = b.global_position

			# Sort by Y first (top to bottom)
			if abs(pos_a.y - pos_b.y) > 50:  # Allow some vertical tolerance
				return pos_a.y < pos_b.y

			# Then by X (left to right)
			return pos_a.x < pos_b.x
	)


func _on_node_added(node: Node):
	"""Auto-register navigable nodes"""
	if node is Button or node is BaseButton:
		register_control(node)


func _on_node_removed(node: Node):
	"""Auto-unregister removed nodes"""
	if node is Control:
		unregister_control(node)


func enable():
	"""Enable keyboard navigation"""
	set_process_unhandled_input(true)


func disable():
	"""Disable keyboard navigation"""
	set_process_unhandled_input(false)
	if _focus_indicator:
		_focus_indicator.detach()
	_current_focus = null
