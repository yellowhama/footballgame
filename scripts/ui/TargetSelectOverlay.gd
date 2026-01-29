extends Control
class_name TargetSelectOverlay

# ========== TargetSelectOverlay: Phase 3 Career Player Mode ==========
# Displays dots at world positions for target selection
# 3 modes: PASS_TARGET, SHOOT_TARGET, DRIBBLE_DIR
#
# Phase 4.3: Mobile Radial UI Support
# - Radial mode with thumb zone avoidance (6 o'clock ±45°)
# - Increased hit areas (32px → 80px for mobile)
# - Player-centered positioning

signal target_selected(target_data: Dictionary)

enum Mode { PASS_TARGET, SHOOT_TARGET, DRIBBLE_DIR }

# ============================================================================
# Constants (Mobile Radial UI)
# ============================================================================

const HIT_AREA_DESKTOP := 32.0  # Original desktop hit area
const HIT_AREA_MOBILE := 80.0  # Mobile-optimized hit area
const THUMB_ZONE_ANGLE := PI / 2.0  # 6 o'clock in radians (90°)
const THUMB_ZONE_RANGE := PI / 4.0  # ±45° range

# ============================================================================
# State
# ============================================================================

var _mode: Mode
var _dots: Array[Control] = []
var _radial_mode: bool = false  # True when used with radial UI
var _radial_center: Vector2 = Vector2.ZERO  # Player position for thumb zone check

# Tier 3 Redesign (Phase 4.3+)
var _is_desktop_mode: bool = true  # Detected from input type (mouse vs touch)
var _selector_box: Control = null  # Selector box that follows cursor (desktop) or stays centered (mobile)
var _confirm_dialog: ConfirmationDialog = null  # YES/NO dialog (desktop only)
var _ok_button: Button = null  # OK button (mobile only)
var _drag_offset: Vector2 = Vector2.ZERO  # Accumulated drag offset (mobile only)
var _nearest_dot: Control = null  # Currently highlighted nearest dot
var _connection_dots: Array[TextureRect] = []  # Connection dots (morph between small and large)
var _line_offset: float = 0.0  # Animation offset for wave effect
var _is_frozen: bool = false  # True when user clicked (freeze animation)

## Magnet Targeting (Phase 2 - v2.0)
var _pass_targets: Array = []  # Player characters (not dots)
var _current_snap_target: int = -1  # Snapped player track_id
var _in_sticky_release: bool = false
var _sticky_timer: float = 0.0

const SNAP_IN_RADIUS := 70.0
const SNAP_OUT_RADIUS := 95.0
const SNAP_SPRING_K := 0.28
const STICKY_DURATION := 0.12
const STICKY_FRICTION := 0.6

## v2.0: Career Player Mode context
var controlled_track_id: int = 0  # Controlled player track_id
var controlled_side: String = "home"  # Controlled player side

## Test override for HorizontalMatchViewer reference
var _viewer_override: Node = null

@export var dot_texture: Texture2D


func _ready() -> void:
	dot_texture = load("res://assets/ui/sunnyside/select_dots_large.png")
	if not dot_texture:
		print("[TargetSelectOverlay] ERROR: Failed to load select_dots_large.png")
	else:
		print("[TargetSelectOverlay] Initialized")

	# Allow input events but pass them through
	mouse_filter = Control.MOUSE_FILTER_PASS
	set_process_input(true)


func show_targets(intent: String, technique: String) -> void:
	print("[TargetSelectOverlay] Showing targets for: %s/%s" % [intent, technique])
	_radial_mode = false  # Desktop mode
	_clear_dots()

	match intent:
		"pass":
			_mode = Mode.PASS_TARGET
			_show_pass_targets()
		"shoot":
			_mode = Mode.SHOOT_TARGET
			_show_shoot_targets()
		"dribble":
			_mode = Mode.DRIBBLE_DIR
			_show_dribble_dirs()
		_:
			print("[TargetSelectOverlay] WARNING: Unknown intent: %s" % intent)


# ============================================================================
# Radial Mode (Mobile) - Phase 4.3
# ============================================================================


## Show targets in radial mode with thumb zone avoidance
##
## Args:
##   intent: String - "pass", "shoot", or "dribble"
##   technique: String - Technique variant (e.g., "through", "power")
##   radial_center: Vector2 - Player screen position (for thumb zone calculation)
func show_targets_radial(intent: String, technique: String, radial_center: Vector2) -> void:
	print("[TargetSelectOverlay] Showing RADIAL targets for: %s/%s at %s" % [intent, technique, radial_center])
	_radial_mode = true
	_radial_center = radial_center
	_drag_offset = Vector2.ZERO  # Reset drag offset (mobile)
	_line_offset = 0.0  # Reset wave animation
	_is_frozen = false  # Reset freeze state
	_clear_dots()

	visible = true  # Show overlay

	match intent:
		"pass":
			_mode = Mode.PASS_TARGET
			_show_pass_targets()  # Show dots AND enable magnet snap in _process()
		"shoot":
			_mode = Mode.SHOOT_TARGET
			_show_shoot_targets()
		"dribble":
			_mode = Mode.DRIBBLE_DIR
			_show_dribble_dirs()
		"dribble_break":
			_mode = Mode.DRIBBLE_DIR
			_show_dribble_break()  # v2.0: Range ring + 8-direction
		_:
			print("[TargetSelectOverlay] WARNING: Unknown intent: %s" % intent)

	# Create UI components (Step 7: Tier 3 Redesign)
	_create_selector_box()

	if _is_desktop_mode:
		_create_confirm_dialog()
	else:
		_create_ok_button()

	# Store original positions for mobile drag
	_store_original_positions()


func _show_pass_targets() -> void:
	print("[TargetSelectOverlay] Mode: PASS_TARGET (v2.0 - use HorizontalMatchViewer sprites)")

	var viewer = _get_viewer_node()
	if not viewer:
		print("[TargetSelectOverlay] WARNING: HorizontalMatchViewer not found")
		_show_auto_button()
		return

	# Get player sprites from HorizontalMatchViewer
	var players = viewer._players if "_players" in viewer else []
	if players.is_empty():
		print("[TargetSelectOverlay] WARNING: No players found in HorizontalMatchViewer")
		_show_auto_button()
		return

	# v2.0: Store player screen positions for magnet snap (no dots)
	_pass_targets.clear()
	for player in players:
		# Skip controlled player
		if player.track_id == controlled_track_id:
			continue

		# Skip opponent team
		if player.side != controlled_side:
			continue

		# Get screen position from sprite
		var screen_pos = player.global_position

		# Skip if in thumb zone
		if _is_in_thumb_zone(screen_pos):
			continue

		_pass_targets.append(
			{"type": "player", "track_id": player.track_id, "screen_pos": screen_pos, "player_node": player}  # Store reference for visual feedback
		)

	print("[TargetSelectOverlay] Found %d pass targets from HorizontalMatchViewer" % _pass_targets.size())

	# Create selector box for magnet snap
	_create_selector_box()

	# Desktop: confirm dialog, Mobile: OK button
	if _is_desktop_mode:
		_create_confirm_dialog()
	else:
		_create_ok_button()

	# Always add AUTO button
	_show_auto_button()


func _show_shoot_targets() -> void:
	print("[TargetSelectOverlay] Mode: SHOOT_TARGET")

	var viewer = _get_viewer_node()
	if not viewer:
		print("[TargetSelectOverlay] WARNING: Viewer node not found")
		return

	# TODO: Determine attacking goal direction from snapshot
	var goal_x = 105.0  # Attacking goal (right side)
	var goal_y = 34.0  # Center

	var targets = [
		{"label": "Near", "y_m": goal_y - 3.66},  # 7.32m goal / 2
		{"label": "Center", "y_m": goal_y},
		{"label": "Far", "y_m": goal_y + 3.66}
	]

	for t in targets:
		var field_pos = Vector2(goal_x, t["y_m"])
		var screen_pos = viewer._field_to_screen(field_pos)

		# Skip if in thumb zone (radial mode only)
		if _is_in_thumb_zone(screen_pos):
			print("[TargetSelectOverlay] Skipping goal target in thumb zone: %s" % t["label"])
			continue

		var dot = _create_dot(screen_pos, {"type": "goal_point", "y_m": t["y_m"], "label": t["label"]})
		add_child(dot)
		_dots.append(dot)

	print("[TargetSelectOverlay] Created %d goal target dots" % _dots.size())


func _show_dribble_dirs() -> void:
	print("[TargetSelectOverlay] Mode: DRIBBLE_DIR")

	var viewer = _get_viewer_node()
	if not viewer:
		print("[TargetSelectOverlay] WARNING: Viewer node not found")
		return

	# TODO: Get player position from snapshot
	var player_pos = Vector2(52.5, 34.0)  # Center field default
	var meters = 6.0

	var directions = [
		{"label": "N", "dx": 0.0, "dy": -1.0},
		{"label": "NE", "dx": 0.707, "dy": -0.707},
		{"label": "E", "dx": 1.0, "dy": 0.0},
		{"label": "SE", "dx": 0.707, "dy": 0.707},
		{"label": "S", "dx": 0.0, "dy": 1.0},
		{"label": "SW", "dx": -0.707, "dy": 0.707},
		{"label": "W", "dx": -1.0, "dy": 0.0},
		{"label": "NW", "dx": -0.707, "dy": -0.707}
	]

	for d in directions:
		var target_pos = player_pos + Vector2(d["dx"], d["dy"]) * meters
		var screen_pos = viewer._field_to_screen(target_pos)

		# Skip if in thumb zone (radial mode only)
		if _is_in_thumb_zone(screen_pos):
			print("[TargetSelectOverlay] Skipping direction in thumb zone: %s" % d["label"])
			continue

		var dot = _create_dot(
			screen_pos, {"type": "direction", "dx": d["dx"], "dy": d["dy"], "meters": meters, "label": d["label"]}
		)
		add_child(dot)
		_dots.append(dot)

	print("[TargetSelectOverlay] Created %d direction dots" % _dots.size())


## v2.0: DRIBBLE-BREAK with range ring and 8 directions
func _show_dribble_break() -> void:
	print("[TargetSelectOverlay] Mode: DRIBBLE-BREAK - 8-way + range ring")

	var viewer = _get_viewer_node()
	if not viewer:
		print("[TargetSelectOverlay] WARNING: Viewer node not found")
		return

	# Create range ring (6m max) - centered at radial_center
	_create_range_ring(_radial_center, 6.0)

	# Get player world position from radial_center (need to convert screen → world)
	# TODO: This is approximate - ideally get from snapshot
	var player_world_pos = Vector2(52.5, 34.0)  # Default center field
	var meters = 6.0

	# 8 directions (E, NE, N, NW, W, SW, S, SE)
	var directions = [
		{"label": "E", "dx": 1.0, "dy": 0.0},
		{"label": "NE", "dx": 0.707, "dy": -0.707},
		{"label": "N", "dx": 0.0, "dy": -1.0},
		{"label": "NW", "dx": -0.707, "dy": -0.707},
		{"label": "W", "dx": -1.0, "dy": 0.0},
		{"label": "SW", "dx": -0.707, "dy": 0.707},
		{"label": "S", "dx": 0.0, "dy": 1.0},
		{"label": "SE", "dx": 0.707, "dy": 0.707}
	]

	for d in directions:
		var target_world_pos = player_world_pos + Vector2(d["dx"], d["dy"]) * meters
		var screen_pos = viewer._field_to_screen(target_world_pos)

		# Skip if in thumb zone
		if _is_in_thumb_zone(screen_pos):
			print("[TargetSelectOverlay] Skipping direction in thumb zone: %s" % d["label"])
			continue

		var dot = _create_dot(
			screen_pos, {"type": "direction", "dx": d["dx"], "dy": d["dy"], "meters": meters, "label": d["label"]}
		)
		add_child(dot)
		_dots.append(dot)

	print("[TargetSelectOverlay] Created %d direction dots (BREAK mode)" % _dots.size())


## Create a visual range ring (circle outline)
func _create_range_ring(center: Vector2, max_radius_m: float) -> void:
	var ring := Line2D.new()
	ring.name = "RangeRing"
	ring.width = 3.0
	ring.default_color = Color(0.9, 0.9, 0.1, 0.6)  # Yellow translucent

	# Convert meters to pixels (approximate - need proper conversion)
	# TODO: Get proper meter→pixel conversion from viewer
	var radius_px := max_radius_m * 10.0  # Rough estimate: 10 px/meter

	# Draw circle (65 points for smooth curve)
	var points := []
	for i in range(65):
		var angle := (i / 64.0) * TAU
		var point := center + Vector2(cos(angle), sin(angle)) * radius_px
		points.append(point)

	ring.points = points
	add_child(ring)
	ring.z_index = 1  # Behind dots but in front of background

	print("[TargetSelectOverlay] Range ring created at %s (radius: %.1f px)" % [center, radius_px])


func _create_dot(screen_pos: Vector2, data: Dictionary) -> Control:
	# Use larger hit area for mobile radial mode
	var hit_area := HIT_AREA_MOBILE if _radial_mode else HIT_AREA_DESKTOP
	var visual_size := 32.0  # Visual always 32px (but hit area is larger)
	var half_hit := hit_area / 2.0
	var half_visual := visual_size / 2.0

	var container = Control.new()
	container.custom_minimum_size = Vector2(hit_area, hit_area)
	container.position = screen_pos - Vector2(half_hit, half_hit)

	# IMPORTANT: Store data in metadata (used by selector for selection)
	container.set_meta("target_data", data)

	# Texture for visual (centered in hit area)
	var texture_rect = TextureRect.new()
	texture_rect.texture = dot_texture
	texture_rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	texture_rect.custom_minimum_size = Vector2(visual_size, visual_size)
	texture_rect.position = Vector2(half_hit - half_visual, half_hit - half_visual)
	texture_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	container.add_child(texture_rect)

	# NO BUTTON - Tier 3 redesign uses selector + confirm dialog (desktop) or OK button (mobile)
	# Click detection is handled by _input() → _on_selector_clicked()

	return container


func _show_auto_button() -> void:
	# AUTO button stays clickable (not affected by selector)
	var auto_button = Button.new()
	auto_button.text = "AUTO"
	auto_button.custom_minimum_size = Vector2(100, 50)
	auto_button.position = Vector2(get_viewport_rect().size.x - 120, get_viewport_rect().size.y - 70)
	auto_button.pressed.connect(
		func():
			target_selected.emit({"type": "auto"})
			visible = false
	)
	auto_button.set_meta("target_data", {"type": "auto"})
	add_child(auto_button)
	_dots.append(auto_button)
	print("[TargetSelectOverlay] Added AUTO button")


# NOTE: _on_dot_clicked() removed - Tier 3 redesign uses selector + confirm (desktop) or OK button (mobile)


func _clear_dots() -> void:
	for dot in _dots:
		dot.queue_free()
	_dots.clear()

	# Cleanup UI components
	if _selector_box:
		_selector_box.queue_free()
		_selector_box = null

	for dot in _connection_dots:
		dot.queue_free()
	_connection_dots.clear()

	if _confirm_dialog:
		_confirm_dialog.queue_free()
		_confirm_dialog = null
	if _ok_button:
		_ok_button.queue_free()
		_ok_button = null

	_nearest_dot = null


func _get_viewer_node() -> Node:
	# Test override (set by test harness)
	if _viewer_override:
		return _viewer_override

	var root = get_tree().root
	for child in root.get_children():
		if child.has_method("_field_to_screen"):
			return child
	return null


func _get_latest_snapshot() -> Dictionary:
	# TODO: Get from MatchSimulationManager or PlayerHUD (Phase 6)
	# For now, return empty dict
	return {}


func _get_teammates(snapshot: Dictionary) -> Array:
	# TODO: Filter players by side (Phase 6)
	# For now, return empty array
	var teammates := []
	return teammates


# ============================================================================
# Thumb Zone Avoidance (Radial Mode) - Phase 4.3
# ============================================================================


## Check if screen position is in thumb zone (6 o'clock ±45°)
##
## Args:
##   screen_pos: Vector2 - Screen position to check
##
## Returns:
##   bool - True if in thumb zone (should be filtered out)
func _is_in_thumb_zone(screen_pos: Vector2) -> bool:
	if not _radial_mode or _radial_center == Vector2.ZERO:
		return false  # Not in radial mode or no center set

	var delta := screen_pos - _radial_center
	var angle := atan2(delta.y, delta.x)  # Returns -PI to PI
	var norm_angle := fposmod(angle, TAU)  # Normalize to 0-2π

	# 6 o'clock is PI/2 in screen coords (Y-down)
	# Check if angle is within ±45° of 6 o'clock
	var min_angle := THUMB_ZONE_ANGLE - THUMB_ZONE_RANGE
	var max_angle := THUMB_ZONE_ANGLE + THUMB_ZONE_RANGE

	return norm_angle >= min_angle and norm_angle <= max_angle


# ============================================================================
# Tier 3 Redesign: Platform Detection & Input Handling
# ============================================================================


## Detect platform (Desktop vs Mobile) from input event type
func _detect_platform_from_input(event: InputEvent) -> void:
	if event is InputEventMouse or event is InputEventMouseButton or event is InputEventMouseMotion:
		_is_desktop_mode = true
	elif event is InputEventScreenTouch or event is InputEventScreenDrag:
		_is_desktop_mode = false


## Handle input events (click for desktop, drag for mobile)
func _input(event: InputEvent) -> void:
	if not visible:
		return

	# Detect platform from first input
	if event is InputEventMouse or event is InputEventScreenTouch:
		_detect_platform_from_input(event)

	# Desktop: Click to show confirm dialog
	if _is_desktop_mode and event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			print("[TargetSelectOverlay] Mouse click detected at: %s" % event.position)
			_on_selector_clicked()

	# Mobile: Drag to move dots
	elif not _is_desktop_mode and event is InputEventScreenDrag:
		_handle_mobile_drag(event)


## Update selector position (desktop mode) and connection line
func _process(_delta: float) -> void:
	if not visible or _is_frozen:
		return

	if _is_desktop_mode and _selector_box:
		# Follow mouse cursor
		var mouse_pos = get_viewport().get_mouse_position()
		_selector_box.position = mouse_pos - _selector_box.custom_minimum_size / 2.0

		# Magnet snap for PASS mode (v2.0)
		if _mode == Mode.PASS_TARGET and not _pass_targets.is_empty():
			_check_magnet_snap()
		elif not _dots.is_empty():
			_check_snap_to_nearest()

	# Sticky release timer
	if _in_sticky_release:
		_sticky_timer += _delta
		if _sticky_timer >= STICKY_DURATION:
			_current_snap_target = -1
			_in_sticky_release = false
			_on_snap_exit()

	# Update connection dots from player to selector box
	if _selector_box and _radial_center != Vector2.ZERO:
		var selector_center = _selector_box.global_position + _selector_box.custom_minimum_size / 2.0

		# Calculate direction and distance
		var direction = (selector_center - _radial_center).normalized()
		var distance = _radial_center.distance_to(selector_center)
		var max_distance = max(0, distance - 35.0)  # Stop 35px before selector box

		# Wave animation: moves through dots sequentially (player → selector)
		_line_offset += _delta * 3.0  # 3 dots per second
		var num_dots = int(max_distance / 30.0) + 1
		if _line_offset >= float(num_dots) + 1.0:  # Reset after wave completes
			_line_offset = 0.0

		# Debug print (first frame only)
		if _connection_dots.is_empty() and num_dots > 0:
			print(
				(
					"[TargetSelectOverlay] Creating %d connection dots from %s to %s (distance: %.1f)"
					% [num_dots, _radial_center, selector_center, distance]
				)
			)

		# Adjust array size (30px spacing)
		while _connection_dots.size() < num_dots:
			var dot = TextureRect.new()
			dot.texture = load("res://assets/ui/sunnyside/select_dots.png")  # Start with small
			dot.custom_minimum_size = Vector2(10, 10)
			dot.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			dot.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			add_child(dot)
			dot.z_index = 5
			_connection_dots.append(dot)

		while _connection_dots.size() > num_dots:
			var dot = _connection_dots.pop_back()
			dot.queue_free()

		# Position dots - all large dots visible, one highlighted by wave
		# Use already loaded dot_texture

		for i in range(_connection_dots.size()):
			var dot = _connection_dots[i]
			var offset_distance = i * 30.0

			if offset_distance <= max_distance:
				# Fixed position for this dot
				var pos = _radial_center + direction * offset_distance

				# All dots are large and visible
				dot.texture = dot_texture
				dot.custom_minimum_size = Vector2(30, 30)
				dot.position = pos - Vector2(15, 15)

				# Calculate wave phase for this dot
				var dot_index = float(i)
				var wave_pos = _line_offset

				# Pattern: 3 bright → 3 dark → 3 bright → 3 dark (repeating)
				# Calculate relative position in the wave pattern
				var relative_pos = int(dot_index - wave_pos) % 6
				if relative_pos < 0:
					relative_pos += 6

				# First 3 positions (0,1,2) are bright, next 3 (3,4,5) are dark
				if relative_pos < 3:
					# Bright dot
					dot.modulate = Color(1.5, 1.5, 1.5, 1.0)
				else:
					# Dark dot
					dot.modulate = Color(1.0, 1.0, 1.0, 0.6)

				dot.visible = true
			else:
				dot.visible = false


# ============================================================================
# Step 2: Selector Box Component
# ============================================================================


func _create_selector_box() -> void:
	_selector_box = Control.new()
	_selector_box.name = "SelectorBox"

	# Size: 60x60px (Sunnyside selectbox size)
	_selector_box.custom_minimum_size = Vector2(60, 60)

	# Make clickable
	_selector_box.mouse_filter = Control.MOUSE_FILTER_STOP

	# Load selectbox corner assets
	var tl = load("res://assets/ui/sunnyside/selectbox_tl.png")
	var tr = load("res://assets/ui/sunnyside/selectbox_tr.png")
	var bl = load("res://assets/ui/sunnyside/selectbox_bl.png")
	var br = load("res://assets/ui/sunnyside/selectbox_br.png")

	# Create 4 corner TextureRects
	var corner_size = 16.0  # Approximate corner size

	# Top-left
	var tl_rect = TextureRect.new()
	tl_rect.texture = tl
	tl_rect.position = Vector2(0, 0)
	tl_rect.custom_minimum_size = Vector2(corner_size, corner_size)
	tl_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_selector_box.add_child(tl_rect)

	# Top-right
	var tr_rect = TextureRect.new()
	tr_rect.texture = tr
	tr_rect.position = Vector2(60 - corner_size, 0)
	tr_rect.custom_minimum_size = Vector2(corner_size, corner_size)
	tr_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_selector_box.add_child(tr_rect)

	# Bottom-left
	var bl_rect = TextureRect.new()
	bl_rect.texture = bl
	bl_rect.position = Vector2(0, 60 - corner_size)
	bl_rect.custom_minimum_size = Vector2(corner_size, corner_size)
	bl_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_selector_box.add_child(bl_rect)

	# Bottom-right
	var br_rect = TextureRect.new()
	br_rect.texture = br
	br_rect.position = Vector2(60 - corner_size, 60 - corner_size)
	br_rect.custom_minimum_size = Vector2(corner_size, corner_size)
	br_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_selector_box.add_child(br_rect)

	# Connect click handler
	_selector_box.gui_input.connect(_on_selector_box_input)

	# Desktop: Position at mouse
	# Mobile: Position at screen center
	if _is_desktop_mode:
		_selector_box.position = get_viewport().get_mouse_position() - Vector2(30, 30)
	else:
		var center = get_viewport_rect().size / 2.0
		_selector_box.position = center - Vector2(30, 30)

	add_child(_selector_box)
	_selector_box.z_index = 10  # Above dots


# ============================================================================
# Step 3: Snap-to-Nearest Logic
# ============================================================================


## v2.0: Magnet snap with hysteresis for PASS mode
func _check_magnet_snap() -> void:
	if _pass_targets.is_empty():
		return

	var selector_center = _selector_box.global_position + _selector_box.custom_minimum_size / 2.0

	# Find nearest player
	var nearest_dist := INF
	var nearest_target: Dictionary = {}
	var nearest_track_id := -1

	for target in _pass_targets:
		var player_pos = target["screen_pos"]
		var dist = selector_center.distance_to(player_pos)
		var track_id = target["track_id"]

		# Snap In: No current target, within 70px
		if _current_snap_target == -1 and dist < SNAP_IN_RADIUS and dist < nearest_dist:
			nearest_dist = dist
			nearest_target = target
			nearest_track_id = track_id

		# Snap Hold: Current target, within 95px
		elif _current_snap_target == track_id:
			if dist < SNAP_OUT_RADIUS:
				# Stay snapped, spring interpolation
				selector_center = selector_center.lerp(player_pos, SNAP_SPRING_K)
				_selector_box.position = selector_center - _selector_box.custom_minimum_size / 2.0

				# Visual feedback on snapped player
				if target.has("player_node"):
					_highlight_player(target["player_node"])
				return
			else:
				# Entered sticky release zone
				if not _in_sticky_release:
					_in_sticky_release = true
					_sticky_timer = 0.0
				return

	# Snap to new target
	if nearest_track_id != -1 and nearest_track_id != _current_snap_target:
		_current_snap_target = nearest_track_id
		_in_sticky_release = false
		_on_snap_enter(nearest_target)

	# Visual feedback for nearest player
	if not nearest_target.is_empty() and nearest_target.has("player_node"):
		_highlight_player(nearest_target["player_node"])


func _check_snap_to_nearest() -> void:
	if _dots.is_empty():
		return

	var selector_center = _selector_box.global_position + _selector_box.custom_minimum_size / 2.0

	var nearest: Control = _dots[0]
	var min_dist = selector_center.distance_to(_get_dot_center(nearest))

	for dot in _dots:
		# Skip AUTO button
		if dot is Button and dot.text == "AUTO":
			continue

		var dot_center = _get_dot_center(dot)
		var dist = selector_center.distance_to(dot_center)

		if dist < min_dist:
			min_dist = dist
			nearest = dot

	# Update highlight
	if _nearest_dot != nearest:
		_unhighlight_all_dots()
		_highlight_dot(nearest)
		_nearest_dot = nearest


func _get_dot_center(dot: Control) -> Vector2:
	return dot.global_position + dot.custom_minimum_size / 2.0


func _highlight_dot(dot: Control) -> void:
	# Scale up + glow effect
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(dot, "scale", Vector2.ONE * 1.3, 0.15)

	# Add glow modulate
	tween.parallel().tween_property(dot, "modulate", Color(1.5, 1.5, 1.0), 0.15)


func _unhighlight_all_dots() -> void:
	for dot in _dots:
		dot.scale = Vector2.ONE
		dot.modulate = Color.WHITE


## v2.0: Visual feedback when snapping to a player
func _on_snap_enter(target: Dictionary) -> void:
	if not _selector_box:
		return

	# Scale up selector box
	var tween = create_tween()
	tween.tween_property(_selector_box, "scale", Vector2.ONE * 1.15, 0.1)

	# Highlight player sprite
	if target.has("player_node"):
		_highlight_player(target["player_node"])

	# Haptic tick
	if Input.has_method("vibrate_handheld"):
		Input.vibrate_handheld(20)

	print("[TargetSelectOverlay] Snapped to player #%d" % target.get("track_id", -1))


func _on_snap_exit() -> void:
	if not _selector_box:
		return

	# Scale down
	var tween = create_tween()
	tween.tween_property(_selector_box, "scale", Vector2.ONE, 0.12)

	_selector_box.modulate.a = 1.0

	print("[TargetSelectOverlay] Snap released")


## v2.0: Highlight player sprite when snapped
func _highlight_player(player_node: Node) -> void:
	if not player_node:
		return

	# Scale up slightly
	var tween = create_tween()
	tween.tween_property(player_node, "scale", Vector2.ONE * 1.1, 0.1)

	# Add glow effect (modulate to brighter color)
	tween.parallel().tween_property(player_node, "modulate", Color(1.3, 1.3, 1.3, 1.0), 0.1)


## v2.0: Selector box click handler
func _on_selector_box_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		_on_selector_clicked()


# ============================================================================
# Step 4: Desktop Confirm Dialog
# ============================================================================


func _create_confirm_dialog() -> void:
	_confirm_dialog = ConfirmationDialog.new()
	_confirm_dialog.title = "Confirm Target"
	_confirm_dialog.dialog_text = "Select this target?"
	_confirm_dialog.dialog_autowrap = true
	_confirm_dialog.size = Vector2i(300, 150)
	_confirm_dialog.ok_button_text = "YES"
	_confirm_dialog.cancel_button_text = "NO"

	# Signals
	_confirm_dialog.confirmed.connect(_on_confirm_yes)
	_confirm_dialog.canceled.connect(_on_confirm_no)

	add_child(_confirm_dialog)
	_confirm_dialog.visible = false  # Hidden initially


func _on_selector_clicked() -> void:
	print("[TargetSelectOverlay] _on_selector_clicked called")

	# v2.0: PASS mode handling
	if _mode == Mode.PASS_TARGET:
		if _current_snap_target != -1:
			# Snapped to player - pass to player
			var target_data: Dictionary = {}
			for target in _pass_targets:
				if target["track_id"] == _current_snap_target:
					target_data = target
					break

			if target_data.is_empty():
				print("[TargetSelectOverlay] No target data for snapped player")
				return

			print("[TargetSelectOverlay] Freezing animation and showing dialog")

			# Freeze animation and hide connection dots
			_is_frozen = true
			for dot in _connection_dots:
				dot.visible = false

			# Show confirm dialog
			var player_name = "Player #%d" % target_data.get("track_id", -1)
			_confirm_dialog.dialog_text = "Pass to %s?" % player_name
			_confirm_dialog.popup_centered()
			print("[TargetSelectOverlay] Dialog shown")
		else:
			# No snap - space pass
			print("[TargetSelectOverlay] Space pass (no player snapped)")

			# Freeze animation and hide connection dots
			_is_frozen = true
			for dot in _connection_dots:
				dot.visible = false

			# Show confirm dialog for space pass
			_confirm_dialog.dialog_text = "Pass to space?"
			_confirm_dialog.popup_centered()
			print("[TargetSelectOverlay] Dialog shown")
	else:
		# Fallback to nearest dot (for SHOOT/DRIBBLE modes)
		if not _nearest_dot:
			print("[TargetSelectOverlay] No nearest dot, returning")
			return

		print("[TargetSelectOverlay] Freezing animation and showing dialog")

		# Freeze animation and hide connection dots
		_is_frozen = true
		for dot in _connection_dots:
			dot.visible = false

		# Show confirm dialog
		_confirm_dialog.dialog_text = "Select this target?\n%s" % _get_dot_label(_nearest_dot)
		_confirm_dialog.popup_centered()
		print("[TargetSelectOverlay] Dialog shown")


func _get_dot_label(dot: Control) -> String:
	# Extract label from dot data (stored in metadata)
	var data = dot.get_meta("target_data", {})
	if data.has("name"):
		return data["name"]
	elif data.has("label"):
		return data["label"]
	elif data.has("type"):
		return data["type"].capitalize()
	return "Unknown Target"


func _on_confirm_yes() -> void:
	# v2.0: PASS mode handling
	if _mode == Mode.PASS_TARGET:
		if _current_snap_target != -1:
			# Player pass
			var target_data: Dictionary = {}
			for target in _pass_targets:
				if target["track_id"] == _current_snap_target:
					target_data = {"type": "player", "track_id": target["track_id"]}
					break

			if target_data.is_empty():
				print("[TargetSelectOverlay] No target data for confirmed player")
				return

			print("[TargetSelectOverlay] Target confirmed: %s" % JSON.stringify(target_data))
			target_selected.emit(target_data)
			visible = false
		else:
			# Space pass - use selector position
			var selector_center = _selector_box.global_position + _selector_box.custom_minimum_size / 2.0

			# Convert screen position to field position
			var viewer = _get_viewer_node()
			if not viewer or not viewer.has_method("_screen_to_field"):
				print("[TargetSelectOverlay] Cannot convert screen to field position")
				return

			var field_pos = viewer._screen_to_field(selector_center)

			var target_data = {"type": "space", "position": field_pos}

			print("[TargetSelectOverlay] Space pass confirmed: %s" % JSON.stringify(target_data))
			target_selected.emit(target_data)
			visible = false
	else:
		# Fallback to nearest dot (for SHOOT/DRIBBLE modes)
		if not _nearest_dot:
			return

		var data = _nearest_dot.get_meta("target_data", {})
		print("[TargetSelectOverlay] Target confirmed: %s" % JSON.stringify(data))
		target_selected.emit(data)
		visible = false


func _on_confirm_no() -> void:
	# Unfreeze animation and show dots again
	_is_frozen = false
	print("[TargetSelectOverlay] Selection canceled")


# ============================================================================
# Step 5: Mobile Drag-to-Move
# ============================================================================


func _handle_mobile_drag(event: InputEventScreenDrag) -> void:
	# Accumulate drag offset
	_drag_offset += event.relative

	# Update all dot positions
	_update_dot_positions()

	# Check snap-to-nearest (selector is fixed, dots move)
	_check_snap_to_nearest()


func _update_dot_positions() -> void:
	for dot in _dots:
		# Skip OK button (stays at bottom)
		if dot == _ok_button:
			continue

		# Get original position (stored in metadata)
		var original_pos = dot.get_meta("original_position", Vector2.ZERO)

		# Apply drag offset
		dot.position = original_pos + _drag_offset


func _store_original_positions() -> void:
	# Call this after creating all dots
	for dot in _dots:
		dot.set_meta("original_position", dot.position)


# ============================================================================
# Step 6: Mobile OK Button
# ============================================================================


func _create_ok_button() -> void:
	_ok_button = Button.new()
	_ok_button.text = "OK"
	_ok_button.custom_minimum_size = Vector2(120, 60)

	# Position: Bottom center
	var viewport_size = get_viewport_rect().size
	_ok_button.position = Vector2(viewport_size.x / 2.0 - 60, viewport_size.y - 80)  # Center horizontally  # 80px from bottom

	# Style
	_ok_button.add_theme_font_size_override("font_size", 24)
	_ok_button.add_theme_color_override("font_color", Color.WHITE)

	# Signal
	_ok_button.pressed.connect(_on_ok_pressed)

	add_child(_ok_button)
	_ok_button.z_index = 20  # Above everything


func _on_ok_pressed() -> void:
	if not _nearest_dot:
		print("[TargetSelectOverlay] No target selected")
		return

	var data = _nearest_dot.get_meta("target_data", {})
	print("[TargetSelectOverlay] OK pressed, target: %s" % JSON.stringify(data))

	# Haptic feedback
	Input.vibrate_handheld(100)

	target_selected.emit(data)
	visible = false
