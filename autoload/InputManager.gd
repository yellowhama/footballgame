extends Node
## Input method detection and abstraction for cross-platform support
## Handles touch (mobile/tablet), mouse (desktop), and hybrid (touchscreen laptops)
## Phase 7A Foundation Component

enum InputMethod { TOUCH, MOUSE, KEYBOARD, HYBRID }

var current_method: InputMethod = InputMethod.MOUSE
var last_input_device: String = ""

signal input_method_changed(new_method: InputMethod)


func _ready():
	print("[InputManager] Initializing...")
	_detect_initial_method()
	set_process_input(true)
	print("[InputManager] Initialized - Method: %s" % InputMethod.keys()[current_method])


func _detect_initial_method():
	# Check if touchscreen is available
	if DisplayServer.is_touchscreen_available():
		if OS.has_feature("pc") or OS.has_feature("macos") or OS.has_feature("linux"):
			current_method = InputMethod.HYBRID  # Touchscreen laptop/desktop
			print("[InputManager] Hybrid device detected (touchscreen PC)")
		else:
			current_method = InputMethod.TOUCH  # Mobile/tablet
			print("[InputManager] Touch device detected (mobile/tablet)")
	else:
		# Desktop without touchscreen
		current_method = InputMethod.MOUSE
		print("[InputManager] Mouse device detected (desktop)")


func _input(event: InputEvent):
	# Track input device changes dynamically
	if event is InputEventScreenTouch or event is InputEventScreenDrag:
		_set_method(InputMethod.TOUCH)
		last_input_device = "touch"

	elif event is InputEventMouse:
		if current_method == InputMethod.HYBRID:
			# Don't switch away from HYBRID on hybrid devices
			last_input_device = "mouse"
		else:
			_set_method(InputMethod.MOUSE)
			last_input_device = "mouse"

	elif event is InputEventKey:
		_set_method(InputMethod.KEYBOARD)
		last_input_device = "keyboard"


func _set_method(method: InputMethod):
	if current_method != method:
		var old_method = current_method
		current_method = method
		input_method_changed.emit(method)
		print(
			(
				"[InputManager] Input method changed: %s → %s"
				% [InputMethod.keys()[old_method], InputMethod.keys()[method]]
			)
		)


## Public API for input method queries
func is_touch() -> bool:
	return current_method == InputMethod.TOUCH


func is_mouse() -> bool:
	return current_method == InputMethod.MOUSE or current_method == InputMethod.HYBRID


func is_keyboard() -> bool:
	return current_method == InputMethod.KEYBOARD


func is_hybrid() -> bool:
	return current_method == InputMethod.HYBRID


func supports_hover() -> bool:
	# Mouse and hybrid devices support hover states
	return current_method in [InputMethod.MOUSE, InputMethod.KEYBOARD, InputMethod.HYBRID]


func supports_touch() -> bool:
	return current_method in [InputMethod.TOUCH, InputMethod.HYBRID]


func get_method_name() -> String:
	return InputMethod.keys()[current_method]


## Helper function for UI components to decide interaction style
func should_show_hover_effects() -> bool:
	return supports_hover()


func should_use_touch_feedback() -> bool:
	return supports_touch()


func should_enable_keyboard_shortcuts() -> bool:
	# Keyboard shortcuts only on desktop (not hybrid, to avoid conflicts)
	return current_method in [InputMethod.MOUSE, InputMethod.KEYBOARD]
