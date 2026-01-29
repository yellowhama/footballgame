## RadialInputDetector - Touch/Drag Gesture Processor for Radial UI
##
## Responsibilities:
## - Process touch/drag events (mobile + desktop simulation)
## - Calculate polar coordinates (angle, distance) from center point
## - Detect sector entry/exit based on angle ranges
## - Emit high-level signals for parent RadialDecisionUI
##
## Pattern References:
## - HexagonChart.gd: Angle calculation with atan2()
## - SwipeNavigator.gd: Multi-touch tracking dictionary
## - RawGesture.gd: Touch/Drag event classes

class_name RadialInputDetector
extends Control

# ============================================================================
# Signals
# ============================================================================

## Emitted when touch/press starts at a position
signal press_started(position: Vector2)

## Emitted continuously during drag with angle and distance
signal drag_updated(position: Vector2, angle: float, distance: float)

## Emitted when finger enters a slot's sector range
signal sector_entered(sector_id: String, angle: float)

## Emitted when finger exits a slot's sector range
signal sector_exited(sector_id: String)

## Emitted when touch/press is released on a sector
signal release_confirmed(sector_id: String)

# ============================================================================
# Constants (from plan)
# ============================================================================

const ACTIVATION_MIN_PX := 72.0  # Activation range start
const ACTIVATION_MAX_PX := 128.0  # Activation range end
const INNER_RADIUS_PX := 24.0  # Dead zone (no detection)

# ============================================================================
# State
# ============================================================================

## Center point for radial calculation (updated by parent)
var _center: Vector2 = Vector2.ZERO

## Whether currently in press+drag state
var _is_pressing: bool = false

## Currently highlighted sector ID (empty if none)
var _current_sector: String = ""

## Position where press started
var _start_position: Vector2 = Vector2.ZERO

## Active slots for hit detection (populated by parent RadialDecisionUI)
## Format: {sector_id: {angle: float, min_angle: float, max_angle: float}}
var active_slots: Dictionary = {}

# ============================================================================
# Initialization
# ============================================================================


func _ready() -> void:
	# Don't block clicks on other UI elements
	mouse_filter = Control.MOUSE_FILTER_IGNORE


# ============================================================================
# Input Handling
# ============================================================================


func _gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		_handle_touch(event)
	elif event is InputEventScreenDrag:
		_handle_drag(event)
	elif event is InputEventMouseButton:
		# Desktop simulation
		_handle_mouse_button(event)
	elif event is InputEventMouseMotion:
		# Desktop drag simulation
		if _is_pressing:
			_handle_mouse_motion(event)


## Handle touch events (mobile)
func _handle_touch(event: InputEventScreenTouch) -> void:
	if event.pressed:
		_start_press(event.position)
	else:
		_end_press(event.position)


## Handle drag events (mobile)
func _handle_drag(event: InputEventScreenDrag) -> void:
	if not _is_pressing:
		return

	_update_position(event.position)


## Handle mouse button events (desktop simulation)
func _handle_mouse_button(event: InputEventMouseButton) -> void:
	if event.button_index != MOUSE_BUTTON_LEFT:
		return

	if event.pressed:
		_start_press(event.position)
	else:
		_end_press(event.position)


## Handle mouse motion events (desktop drag simulation)
func _handle_mouse_motion(event: InputEventMouseMotion) -> void:
	_update_position(event.position)


# ============================================================================
# Press/Drag State Machine
# ============================================================================


## Start press at position (sets center point)
func _start_press(pos: Vector2) -> void:
	_is_pressing = true
	_start_position = pos
	_center = pos  # Player-centered radial UI
	_current_sector = ""

	press_started.emit(pos)


## Update position during drag (calculate angle/distance, detect sectors)
func _update_position(pos: Vector2) -> void:
	if not _is_pressing:
		return

	# Calculate polar coordinates from center
	var delta := pos - _center
	var angle := atan2(delta.y, delta.x)  # Returns -PI to PI
	var distance := delta.length()

	# Emit drag update
	drag_updated.emit(pos, angle, distance)

	# Sector detection: Only in activation range (72-128px)
	if distance >= ACTIVATION_MIN_PX and distance <= ACTIVATION_MAX_PX:
		var new_sector := _find_sector_at_angle(angle)

		if new_sector != _current_sector:
			# Sector changed
			if _current_sector != "":
				sector_exited.emit(_current_sector)

			if new_sector != "":
				sector_entered.emit(new_sector, angle)
				# Haptic feedback on sector entry (pattern from SwipeNavigator)
				Input.vibrate_handheld(50)  # Light vibration (50ms)

			_current_sector = new_sector
	else:
		# Out of activation range - clear sector
		if _current_sector != "":
			sector_exited.emit(_current_sector)
			_current_sector = ""


## End press (release) - confirm selection if on sector
func _end_press(pos: Vector2) -> void:
	if not _is_pressing:
		return

	_is_pressing = false

	# If released on a sector, emit confirmation
	if _current_sector != "":
		release_confirmed.emit(_current_sector)

	# Reset state
	_current_sector = ""
	_center = Vector2.ZERO


# ============================================================================
# Sector Detection (Pattern from HexagonChart._check_hover)
# ============================================================================


## Find which sector (if any) contains the given angle
##
## Args:
##   angle: Angle in radians from center (-PI to PI from atan2)
##
## Returns:
##   sector_id: String - ID of sector, or "" if no match
func _find_sector_at_angle(angle: float) -> String:
	# Normalize angle to 0-TAU (0-2π) for consistent comparison
	var norm_angle := fposmod(angle, TAU)

	# Check each active slot
	for sector_id in active_slots:
		var slot: Dictionary = active_slots[sector_id]

		# Get min/max angles for this sector (also normalized)
		var min_a := fposmod(slot.min_angle, TAU)
		var max_a := fposmod(slot.max_angle, TAU)

		# Handle wrap-around at 0/TAU boundary
		# Case 1: Normal range (min < max)
		if min_a <= max_a:
			if norm_angle >= min_a and norm_angle <= max_a:
				return sector_id
		# Case 2: Wrap-around range (e.g., 350° to 10°)
		else:
			if norm_angle >= min_a or norm_angle <= max_a:
				return sector_id

	return ""


# ============================================================================
# Public API (called by parent RadialDecisionUI)
# ============================================================================


## Update the active slots for hit detection
##
## Args:
##   slots: Dictionary of {sector_id: {angle, min_angle, max_angle}}
func update_active_slots(slots: Dictionary) -> void:
	active_slots = slots


## Set the center point for radial calculation (player position)
func set_center(center_pos: Vector2) -> void:
	_center = center_pos


## Check if currently pressing
func is_pressing() -> bool:
	return _is_pressing


## Get current sector ID (empty if none highlighted)
func get_current_sector() -> String:
	return _current_sector


# ============================================================================
# Debug Helpers
# ============================================================================


func _to_string() -> String:
	return (
		"[RadialInputDetector] Pressing: %s | Sector: %s | Center: %s"
		% [str(_is_pressing), _current_sector if _current_sector != "" else "none", str(_center)]
	)
