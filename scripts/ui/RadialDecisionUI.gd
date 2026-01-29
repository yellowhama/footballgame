## RadialDecisionUI v2.0 - Player-Centered Radial Gesture Decision System
##
## Mobile-first 2-tier radial UI for Career Player Mode:
## - Tier 1 (WHAT): Intent selection - PASS/SHOOT/KEEP/BREAK (4 slots at 90°)
## - Tier 3 (WHERE): Target selection (direct jump, no Tier 2)
##   - PASS: Magnet targeting with snap-to-player
##   - SHOOT: Goal dots selection
##   - KEEP: Immediate execution (no Tier 3)
##   - BREAK: Range ring + 8-direction selection
##
## v2.0 Changes:
## - Removed Tier 2 (technique selection)
## - Auto-inference: System picks technique based on context
## - 4 intent slots (PASS/SHOOT/KEEP/BREAK) at 0°/90°/-90°/180°
## - KEEP executes immediately without target selection
##
## Features:
## - Player-centered positioning (NOT fixed corner)
## - Press+Drag gesture input (NOT tap)
## - Thumb zone avoidance (6 o'clock empty)
## - MODE system: MANUAL/AUTO/FULL_AUTO
##
## Integration Points:
## - UnifiedFramePipeline: snapshot_ready signal (single source of snapshots)
## - MatchSimulationManager: send_user_command()
## - Match viewers: _field_to_screen() for player positioning
## - TargetSelectOverlay: Tier 3 target selection with magnet snap

class_name RadialDecisionUI
extends Control

# ============================================================================
# Signals
# ============================================================================

## Emitted when user selects action in Interactive Mode (Game OS Phase E integration)
signal action_selected(action: Dictionary)

# ============================================================================
# Constants (from plan)
# ============================================================================

const INNER_RADIUS_PX := 24.0  # Dead zone (no detection)
const OUTER_RADIUS_PX := 110.0  # Slot center distance from player (thumb-friendly)
const ACTIVATION_MIN_PX := 72.0  # Activation range start
const ACTIVATION_MAX_PX := 140.0  # Activation range end (adjusted for new radius)
const VISUAL_BUTTON_SIZE := 56.0  # Visual size
const HIT_AREA_SIZE := 80.0  # Touch hit area (mobile-optimized)

# Fixed Angles (radians) - Tier 1
const ANGLE_PASS := 0.0  # 3 o'clock (RIGHT)
const ANGLE_DRIBBLE := PI  # 9 o'clock (LEFT)
const ANGLE_SHOOT := -PI / 2.0  # 12 o'clock (UP)
# 6 o'clock (PI/2) intentionally EMPTY - thumb zone

# Visual Feedback
const HIGHLIGHT_SCALE := 1.2  # +20% scale on highlight
const ANIM_DURATION := 0.15  # Tween duration
const LINE_WIDTH := 5.0  # Connection line thickness (mobile visibility)

# Timeout
const SELECTION_TIMEOUT_MS := 120000  # 120 seconds per tier (long for testing)

# ============================================================================
# State Machine
# ============================================================================

enum State { STATE_HIDDEN, STATE_TIER1_ACTIVE, STATE_TIER3_ACTIVE, STATE_KEEP_ACTIVE, STATE_AUTO_MODE, STATE_FULL_AUTO }  # Off-ball or disabled  # Showing intent slots (PASS/SHOOT/KEEP/BREAK)  # TargetSelectOverlay active  # KEEP state - immediate execution  # AI taking over this turn  # AI mode until manually disabled

var _state: State = State.STATE_HIDDEN
var _selection_started_ms: int = 0

# ============================================================================
# Mode System
# ============================================================================

enum Mode { MANUAL, AUTO, FULL_AUTO }

var _current_mode: Mode = Mode.MANUAL
var _mode_before_auto: Mode = Mode.MANUAL
var _mode_indicator: Label = null  # Top-left mode display

# Long-press detection for FULL_AUTO toggle
const LONG_PRESS_THRESHOLD_MS := 1000  # 1 second hold
var _auto_button_press_time_ms: int = 0
var _auto_button_is_down: bool = false

# ============================================================================
# Operation Mode (Game OS Phase E integration)
# ============================================================================

enum OperationMode { NORMAL, INTERACTIVE }  # Ball gain/loss, direct send_user_command (existing behavior)  # Intervention mode, emit action_selected signal (Game OS Phase E)

var _operation_mode: OperationMode = OperationMode.NORMAL
var _current_intervention_context: Dictionary = {}

# ============================================================================
# Selection State
# ============================================================================

var _selected_intent: String = ""
var _selected_technique: String = ""
var _selected_target: Dictionary = {}

# ============================================================================
# Exports (PlayerHUD integration)
# ============================================================================

@export var controlled_track_id: int = 9
@export var controlled_side: String = "home"
@export var controller_id: int = -1
@export var match_id: String = "session_001"
@export var enable_sticky_buttons: bool = true

# ============================================================================
# Component State
# ============================================================================

## Match OS state (from FieldBoard)
var _current_pressure: float = 0.0  # 4m tactical pressure (FieldBoard)
var _immediate_pressure: float = 0.0  # 2m tackle threat (Match OS v1.2)
var _current_cell: Vector2i = Vector2i.ZERO
var _simulator: Node = null
var _tooltip_panel: Control = null

## Radial center position (player screen position)
var _radial_center: Vector2 = Vector2.ZERO

## Tier 1 slots (PASS/SHOOT/KEEP/BREAK)
var _tier1_slots: Array[RadialSlot] = []

## Connection line from center to highlighted slot
var _connection_line: Line2D

## Input locked until (prevents double-tap)
var _input_locked_until_ms: int = 0
const INPUT_LOCK_MS := 300

# Sticky actions (sprint/dribble/press)
var _sticky_actions: Dictionary = {"sprint": false, "dribble": false, "press": false}
var _controller_registered: bool = false
var _sticky_buttons: Dictionary = {}

# ============================================================================
# Node References (to be created in scenes or _ready)
# ============================================================================

var input_detector: RadialInputDetector
var tier1_layer: Control
var tier2_layer: Control
var auxiliary_panel: Control
var target_overlay: Control  # TargetSelectOverlay (will be added in Phase 4)
var _cached_match_viewer: Node = null  # Phase 1: 노드 탐색 결과 캐싱

# ============================================================================
# Tier Definitions
# ============================================================================

const TIER1_SLOTS := {
	"pass": {"angle": 0.0, "label": "PASS", "color": Color(0.2, 0.8, 0.3)},  # 0° (right)  # Green
	"shoot": {"angle": PI / 2.0, "label": "SHOOT", "color": Color(0.9, 0.5, 0.1)},  # 90° (up)  # Orange
	"dribble_keep": {"angle": -PI / 2.0, "label": "KEEP", "color": Color(0.9, 0.9, 0.1)},  # -90° (down)  # Yellow
	"dribble_break": {"angle": PI, "label": "BREAK", "color": Color(0.9, 0.8, 0.1)}  # 180° (left)  # Yellow (darker)
}

# TIER2 REMOVED in v2.0 - Direct Tier 1 → Tier 3 flow

# ============================================================================
# Initialization
# ============================================================================


func _ready() -> void:
	# Create component structure
	_setup_components()

	# Wire signals
	_wire_signals()

	# Subscribe to unified snapshots (single pipeline)
	var pipeline := get_node_or_null("/root/UnifiedFramePipeline")
	if pipeline and pipeline.has_signal("snapshot_ready"):
		pipeline.snapshot_ready.connect(_on_unified_snapshot)
		print("[RadialDecisionUI] Subscribed to UnifiedFramePipeline")
	else:
		print("[RadialDecisionUI] WARNING: UnifiedFramePipeline not found or missing signal")

	# Initialize state
	_reset_to_hidden()

	# Don't block clicks on other UI elements
	mouse_filter = Control.MOUSE_FILTER_IGNORE

	print("[RadialDecisionUI] Initialized")


func _setup_components() -> void:
	# Input detector layer (full rect, transparent)
	input_detector = RadialInputDetector.new()
	input_detector.name = "InputDetector"
	input_detector.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(input_detector)

	# Tier 1 layer (slots container)
	tier1_layer = Control.new()
	tier1_layer.name = "Tier1Layer"
	tier1_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	tier1_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(tier1_layer)

	# Tier 2 layer (techniques container)
	tier2_layer = Control.new()
	tier2_layer.name = "Tier2Layer"
	tier2_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	tier2_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(tier2_layer)

	# Auxiliary panel (Cancel/Auto/Info buttons) - Phase 2
	auxiliary_panel = _create_auxiliary_panel()
	add_child(auxiliary_panel)

	# Connection line (visual feedback)
	_connection_line = Line2D.new()
	_connection_line.name = "ConnectionLine"
	_connection_line.width = LINE_WIDTH
	_connection_line.default_color = Color(1.0, 1.0, 1.0, 0.7)
	_connection_line.visible = false
	add_child(_connection_line)

	# Target overlay (Tier 3) - Phase 4.3
	var overlay_script = load("res://scripts/ui/TargetSelectOverlay.gd")
	if overlay_script:
		target_overlay = TargetSelectOverlay.new()
		target_overlay.name = "TargetOverlay"
		target_overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
		target_overlay.visible = false
		target_overlay.controlled_track_id = controlled_track_id
		target_overlay.controlled_side = controlled_side
		add_child(target_overlay)
		print("[RadialDecisionUI] TargetSelectOverlay created")
	else:
		print("[RadialDecisionUI] ERROR: Could not load TargetSelectOverlay.gd")

	# Mode indicator (top-left) - Phase 5
	_mode_indicator = Label.new()
	_mode_indicator.name = "ModeIndicator"
	_mode_indicator.set_anchors_preset(Control.PRESET_TOP_LEFT)
	_mode_indicator.offset_left = 20
	_mode_indicator.offset_top = 20
	_mode_indicator.add_theme_font_size_override("font_size", 28)
	_mode_indicator.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0, 0.9))
	_mode_indicator.text = "MANUAL"
	_mode_indicator.visible = false  # Hidden by default
	add_child(_mode_indicator)

	# Create Tier 1 slots
	_create_tier1_slots()


func _wire_signals() -> void:
	# Input detector signals
	input_detector.press_started.connect(_on_press_started)
	input_detector.drag_updated.connect(_on_drag_updated)
	input_detector.sector_entered.connect(_on_sector_entered)
	input_detector.sector_exited.connect(_on_sector_exited)
	input_detector.release_confirmed.connect(_on_release_confirmed)

	# Target overlay signal (Phase 4.3)
	if target_overlay:
		target_overlay.target_selected.connect(_on_target_selected)

	# Snapshot wiring happens in _ready() via UnifiedFramePipeline.


# ============================================================================
# Tier 1 Slot Creation (Phase 1 & 2)
# ============================================================================


func _create_tier1_slots() -> void:
	_tier1_slots.clear()

	for slot_id in TIER1_SLOTS:
		var slot_data: Dictionary = TIER1_SLOTS[slot_id]

		# Create slot
		var slot := RadialSlot.new()
		slot.slot_id = slot_id
		slot.label_text = slot_data["label"]
		slot.slot_color = slot_data["color"]
		slot.visible = false  # Hidden initially

		# Make slots clickable (connect once during creation)
		slot.mouse_filter = Control.MOUSE_FILTER_STOP
		slot.gui_input.connect(_on_tier1_slot_clicked.bind(slot_id))

		# Add to layer
		tier1_layer.add_child(slot)
		_tier1_slots.append(slot)

	print("[RadialDecisionUI] Created %d Tier 1 slots" % _tier1_slots.size())


func _show_tier1_slots() -> void:
	# Update active slots for input detector
	var active_slots := {}

	for slot in _tier1_slots:
		var angle: float = TIER1_SLOTS[slot.slot_id]["angle"]

		# Each slot has ±30° hit range (PI/6 radians)
		var min_angle := angle - PI / 6.0
		var max_angle := angle + PI / 6.0

		active_slots[slot.slot_id] = {"angle": angle, "min_angle": min_angle, "max_angle": max_angle}

		# Position slot at angle
		slot.position_at_angle(_radial_center, angle, OUTER_RADIUS_PX)
		slot.visible = true

	# Update input detector with sector ranges
	input_detector.update_active_slots(active_slots)


func _hide_tier1_slots() -> void:
	for slot in _tier1_slots:
		slot.visible = false
		slot.set_highlighted(false)


# ============================================================================
# Tier 2 REMOVED in v2.0 - Direct Tier 1 → Tier 3 flow
# ============================================================================

# ============================================================================
# Auxiliary Panel (Cancel Button - Phase 2)
# ============================================================================


func _create_auxiliary_panel() -> Control:
	# Panel container (right side of screen)
	var panel := PanelContainer.new()
	panel.name = "AuxiliaryPanel"

	# Anchor to top-right corner
	panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	panel.offset_left = -80  # 80px from right edge
	panel.offset_top = 20
	panel.offset_right = -20
	panel.offset_bottom = 420  # Taller to fit sticky buttons

	# Margin container
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 8)
	margin.add_theme_constant_override("margin_top", 8)
	margin.add_theme_constant_override("margin_right", 8)
	margin.add_theme_constant_override("margin_bottom", 8)
	panel.add_child(margin)

	# VBox for buttons
	var vbox := VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 8)
	margin.add_child(vbox)

	# Cancel button
	var btn_cancel := Button.new()
	btn_cancel.name = "BtnCancel"
	btn_cancel.text = "Cancel"
	btn_cancel.custom_minimum_size = Vector2(64, 56)  # Mobile-friendly: 56px height minimum
	btn_cancel.pressed.connect(_on_cancel_pressed)
	vbox.add_child(btn_cancel)

	# Auto button (Phase 5 - Active)
	var btn_auto := Button.new()
	btn_auto.name = "BtnAuto"
	btn_auto.text = "Auto"
	btn_auto.custom_minimum_size = Vector2(64, 56)  # Mobile-friendly
	btn_auto.button_down.connect(_on_auto_button_down)
	btn_auto.button_up.connect(_on_auto_button_up)
	vbox.add_child(btn_auto)

	# Info button (Future - placeholder for decision log)
	var btn_info := Button.new()
	btn_info.name = "BtnInfo"
	btn_info.text = "Info"
	btn_info.custom_minimum_size = Vector2(64, 56)  # Mobile-friendly
	btn_info.disabled = false  # NEW: Enabled (Match OS v1.1)
	btn_info.pressed.connect(_on_info_pressed)
	vbox.add_child(btn_info)

	if enable_sticky_buttons:
		var sticky_label := Label.new()
		sticky_label.text = "Sticky"
		sticky_label.add_theme_font_size_override("font_size", 16)
		vbox.add_child(sticky_label)

		vbox.add_child(_create_sticky_button("Sprint", "sprint"))
		vbox.add_child(_create_sticky_button("Dribble", "dribble"))
		vbox.add_child(_create_sticky_button("Press", "press"))

	panel.visible = false  # Hidden until UI is active

	return panel


func _create_sticky_button(label: String, action: String) -> Button:
	var btn := Button.new()
	btn.name = "BtnSticky" + action.capitalize()
	btn.text = label
	btn.custom_minimum_size = Vector2(64, 56)
	btn.toggle_mode = true
	btn.button_pressed = bool(_sticky_actions.get(action, false))
	btn.toggled.connect(_on_sticky_toggled.bind(action))
	_sticky_buttons[action] = btn
	return btn


func _on_cancel_pressed() -> void:
	print("[RadialDecisionUI] Cancel pressed")
	Input.vibrate_handheld(50)  # Light haptic feedback
	_reset_to_hidden()


func _on_auto_button_down() -> void:
	_auto_button_is_down = true
	_auto_button_press_time_ms = Time.get_ticks_msec()


func _on_auto_button_up() -> void:
	if not _auto_button_is_down:
		return

	_auto_button_is_down = false
	var press_duration_ms := Time.get_ticks_msec() - _auto_button_press_time_ms

	if press_duration_ms >= LONG_PRESS_THRESHOLD_MS:
		# Long press: Toggle FULL_AUTO mode
		_toggle_full_auto_mode()
	else:
		# Short press: Send AUTO command for this turn
		_send_auto_command()


func _on_sticky_toggled(pressed: bool, action: String) -> void:
	_sticky_actions[action] = pressed
	_send_sticky_action(action, pressed)


func _send_sticky_action(action: String, enabled: bool) -> void:
	var manager = get_node_or_null("/root/MatchSimulationManager")
	if manager and manager.has_method("set_sticky_action"):
		manager.set_sticky_action(controlled_track_id, action, enabled)
		print("[RadialDecisionUI] Sticky action %s=%s (track_id=%d)" % [action, str(enabled), controlled_track_id])
	else:
		print("[RadialDecisionUI] ERROR: MatchSimulationManager not available for sticky actions")


func _toggle_full_auto_mode() -> void:
	if _current_mode == Mode.FULL_AUTO:
		_current_mode = Mode.MANUAL
		_mode_indicator.text = "MANUAL"
		_mode_indicator.visible = false
		print("[RadialDecisionUI] Mode: FULL_AUTO → MANUAL")
		Input.vibrate_handheld(100)  # Medium haptic
	else:
		_current_mode = Mode.FULL_AUTO
		_mode_indicator.text = "FULL AUTO"
		_mode_indicator.visible = true
		print("[RadialDecisionUI] Mode: MANUAL → FULL_AUTO")
		Input.vibrate_handheld(100)  # Medium haptic
		_reset_to_hidden()  # Hide UI in FULL_AUTO


func _send_auto_command() -> void:
	print("[RadialDecisionUI] AUTO mode: Sending AI command for this turn")
	Input.vibrate_handheld(50)  # Light haptic

	# Build AUTO command payload
	var payload := {"cmd": "on_ball_action", "auto_target": true}  # Tell backend to use AI decision

	var cmd := {
		"mode": "career_player",
		"match_id": match_id,
		"side": controlled_side,
		"controlled_track_id": controlled_track_id,
		"payload": payload
	}

	var manager := get_node_or_null("/root/MatchSimulationManager")
	if manager and manager.has_method("send_user_command"):
		manager.send_user_command(cmd)
		print("[RadialDecisionUI] AUTO command sent")
	else:
		print("[RadialDecisionUI] ERROR: MatchSimulationManager not found")

	# Auto-return to MANUAL after command sent
	_current_mode = Mode.MANUAL
	_reset_to_hidden()


# ============================================================================
# State Transitions
# ============================================================================


func _enter_state_tier1() -> void:
	_state = State.STATE_TIER1_ACTIVE
	_selection_started_ms = Time.get_ticks_msec()

	_selected_intent = ""
	_selected_technique = ""

	_show_tier1_slots()

	print("[RadialDecisionUI] → TIER1_ACTIVE")


func _enter_state_keep() -> void:
	_state = State.STATE_KEEP_ACTIVE
	print("[RadialDecisionUI] → KEEP_ACTIVE (shielding ball)")

	# Build command immediately
	var command := {
		"mode": "career_player",
		"match_id": match_id,
		"side": controlled_side,
		"controlled_track_id": controlled_track_id,
		"payload": {"cmd": "on_ball_action", "action": "hold", "auto_target": false}  # FIXED: Use "hold" action (not "dribble" with "keep" variant)
	}

	# Send to engine
	var manager := get_node_or_null("/root/MatchSimulationManager")
	if manager and manager.has_method("send_user_command"):
		manager.send_user_command(command)
		print("[RadialDecisionUI] KEEP command sent")
	else:
		push_error("[RadialDecisionUI] MatchSimulationManager not found")

	# Show state UI (TODO: shield icon, ring, text)
	# _show_keep_state_ui()

	# Auto-hide after 3 seconds
	await get_tree().create_timer(3.0).timeout
	if _state == State.STATE_KEEP_ACTIVE:
		_reset_to_hidden()


func _enter_state_tier3(intent_id: String) -> void:
	_state = State.STATE_TIER3_ACTIVE
	_selection_started_ms = Time.get_ticks_msec()

	# Hide Tier 1 slots
	_hide_tier1_slots()

	# Map intent to default technique (no Tier 2 in v2.0)
	_selected_technique = _map_intent_to_technique(intent_id)

	# Show auxiliary panel
	if auxiliary_panel:
		auxiliary_panel.visible = true

	# Show target overlay in radial mode (Phase 4.3)
	if target_overlay:
		target_overlay.show_targets_radial(_selected_intent, _selected_technique, _radial_center)
		target_overlay.visible = true
		print("[RadialDecisionUI] Target overlay shown in radial mode")
	else:
		print("[RadialDecisionUI] WARNING: target_overlay not found")

	print("[RadialDecisionUI] → TIER3_ACTIVE (intent: %s, technique: %s)" % [intent_id, _selected_technique])


func _map_intent_to_technique(intent_id: String) -> String:
	match intent_id:
		"pass":
			return "pass"
		"shoot":
			return "placed"
		"dribble_break":
			return "takeon"
		_:
			return intent_id


func _reset_to_hidden() -> void:
	_state = State.STATE_HIDDEN
	_selected_intent = ""
	_selected_technique = ""
	_selected_target = {}

	_hide_tier1_slots()
	_connection_line.visible = false

	# Hide auxiliary panel
	if auxiliary_panel:
		auxiliary_panel.visible = false

	# Hide target overlay (Phase 4.3)
	if target_overlay:
		target_overlay.visible = false

	print("[RadialDecisionUI] → HIDDEN")


# ============================================================================
# Input Callbacks
# ============================================================================


func _on_press_started(position: Vector2) -> void:
	if _state != State.STATE_HIDDEN:
		return  # Already active

	if _is_input_locked():
		return

	# Set radial center to press position (Phase 1)
	# TODO: Phase 4 - Use player screen position from _field_to_screen()
	_radial_center = position

	# Enter Tier 1
	_enter_state_tier1()


func _on_drag_updated(position: Vector2, angle: float, distance: float) -> void:
	# Update connection line
	if distance >= ACTIVATION_MIN_PX and distance <= ACTIVATION_MAX_PX:
		_connection_line.clear_points()
		_connection_line.add_point(_radial_center)
		_connection_line.add_point(position)
		_connection_line.visible = true
	else:
		_connection_line.visible = false


func _on_sector_entered(sector_id: String, angle: float) -> void:
	if _state == State.STATE_TIER1_ACTIVE:
		# Highlight Tier 1 slot
		for slot in _tier1_slots:
			slot.set_highlighted(slot.slot_id == sector_id)

		# NEW: Show tooltip for KEEP
		if sector_id == "dribble_keep":
			_show_keep_tooltip()
		else:
			_hide_tooltip()


func _on_sector_exited(sector_id: String) -> void:
	if _state == State.STATE_TIER1_ACTIVE:
		# Unhighlight Tier 1 slot
		for slot in _tier1_slots:
			if slot.slot_id == sector_id:
				slot.set_highlighted(false)

		# NEW: Hide tooltip
		if sector_id == "dribble_keep":
			_hide_tooltip()


func _on_release_confirmed(sector_id: String) -> void:
	if _state == State.STATE_TIER1_ACTIVE:
		# Intent selected → Direct to KEEP or Tier 3
		_selected_intent = sector_id

		match sector_id:
			"dribble_keep":
				_enter_state_keep()
			"pass", "shoot", "dribble_break":
				_enter_state_tier3(sector_id)


## Handle Tier 1 button click (v2.0: Direct to KEEP or Tier 3)
func _on_tier1_slot_clicked(event: InputEvent, intent_id: String) -> void:
	if _state != State.STATE_TIER1_ACTIVE:
		return

	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		print("[RadialDecisionUI] Tier 1 clicked: %s" % intent_id)
		_selected_intent = intent_id

		# Highlight clicked slot
		for slot in _tier1_slots:
			slot.set_highlighted(slot.slot_id == intent_id)

		# DIRECT JUMP based on intent
		match intent_id:
			"dribble_keep":
				_enter_state_keep()  # Immediate execution
			"pass", "shoot", "dribble_break":
				_enter_state_tier3(intent_id)  # Show Tier 3


## Handle target selection from TargetSelectOverlay (Phase 4.3)
func _on_target_selected(target_data: Dictionary) -> void:
	if _state != State.STATE_TIER3_ACTIVE:
		print("[RadialDecisionUI] WARNING: Target selected in wrong state: %s" % State.keys()[_state])
		return

	print("[RadialDecisionUI] Target selected: %s" % JSON.stringify(target_data))

	# Store target data
	_selected_target = target_data

	# Send command
	_send_user_command(_selected_intent, _selected_technique, _selected_target)

	# Hide UI and reset to HIDDEN
	_reset_to_hidden()

	# Haptic feedback for selection
	Input.vibrate_handheld(100)  # Medium vibration


# ============================================================================
# Game OS Interactive Mode Integration (Phase E)
# ============================================================================


## Show radial menu for Interactive Mode intervention
## Called by MatchSimulationScreen when InteractiveMatchController emits intervention_requested
func show_intervention(context: Dictionary) -> void:
	print("[RadialDecisionUI] Interactive Mode: show_intervention() called")
	print("[RadialDecisionUI] Context: %s" % JSON.stringify(context))

	# Switch to Interactive Mode
	_operation_mode = OperationMode.INTERACTIVE
	_current_intervention_context = context

	# Extract context data (for future probability display)
	var player_id = context.get("player_id", -1)
	var shoot_prob = context.get("shoot_prob", 0.0)
	var dribble_prob = context.get("dribble_prob", 0.0)
	var pass_targets = context.get("pass_targets", [])

	print(
		(
			"[RadialDecisionUI] Player: %d, Shoot: %.2f, Dribble: %.2f, Pass targets: %d"
			% [player_id, shoot_prob, dribble_prob, pass_targets.size()]
		)
	)

	# TODO: Update Tier 1 slot labels to show probabilities
	# For now, just show Tier 1 menu

	# Show Tier 1 (intent selection)
	_enter_state_tier1()


## Convert RadialDecisionUI action to InteractiveMatchController format
func _convert_to_interactive_action(intent: String, target_id: int) -> Dictionary:
	match intent:
		"shoot":
			return {"type": "shoot"}
		"carry":
			return {"type": "dribble"}
		"dribble":
			return {"type": "dribble"}
		"pass":
			return {"type": "pass_to", "target_id": target_id}
		"hold":
			# Hold = keep possession, map to dribble
			return {"type": "dribble"}
		_:
			# Fallback: dribble
			push_warning("[RadialDecisionUI] Unknown intent '%s', falling back to dribble" % intent)
			return {"type": "dribble"}


# ============================================================================
# Command Sending (Phase 4 integration)
# ============================================================================


func _send_user_command(intent: String, technique: String, target: Dictionary) -> void:
	_lock_input()

	# ========================================================================
	# Game OS Interactive Mode: Emit signal instead of direct call
	# ========================================================================
	if _operation_mode == OperationMode.INTERACTIVE:
		print("[RadialDecisionUI] Interactive Mode: Converting action and emitting signal")

		# Extract target_id from target dictionary
		var target_id: int = int(target.get("track_id", 0))

		# Convert to InteractiveMatchController format
		var action := _convert_to_interactive_action(intent, target_id)

		print("[RadialDecisionUI] Emitting action_selected: %s" % JSON.stringify(action))

		# Emit signal for MatchSimulationScreen to catch
		action_selected.emit(action)

		# Hide UI
		_reset_to_hidden()

		# Reset to normal mode
		_operation_mode = OperationMode.NORMAL
		_current_intervention_context = {}

		return  # Don't call MatchSimulationManager in Interactive Mode

	# ========================================================================
	# Normal Mode: Direct call to MatchSimulationManager (existing behavior)
	# ========================================================================

	# Map to backwards-compatible format (same as desktop PlayerHUD)
	var action := _map_intent_to_action(intent)
	var payload := {"cmd": "on_ball_action", "action": action}

	# Add variant if different from base action
	if technique != action:
		payload["variant"] = technique

	# Add target parameters
	if target.get("type") == "player":
		payload["target_track_id"] = target.get("track_id", -1)
	elif target.get("type") == "goal_point":
		payload["target_y_m"] = target.get("y_m", 34.0)
	elif target.get("type") == "direction":
		payload["direction_dx"] = target.get("dx", 0.0)
		payload["direction_dy"] = target.get("dy", 0.0)
		payload["direction_meters"] = target.get("meters", 6.0)
	elif target.get("type") == "auto":
		payload["auto_target"] = true

	# Full command
	var manager := get_node_or_null("/root/MatchSimulationManager")
	if controller_id >= 0:
		if not _ensure_controller_registered(manager):
			print("[RadialDecisionUI] ERROR: Controller registration failed")
			return
		var cmd := {"controller_id": controller_id, "payload": payload}
		if manager and manager.has_method("send_multi_agent_commands"):
			manager.send_multi_agent_commands([cmd])
			print("[RadialDecisionUI] Sent multi-agent: %s/%s" % [intent, technique])
		else:
			print("[RadialDecisionUI] ERROR: MatchSimulationManager not available for multi-agent")
		_reset_to_hidden()
		return

	var cmd := {
		"mode": "career_player",
		"match_id": match_id,
		"side": controlled_side,
		"controlled_track_id": controlled_track_id,
		"payload": payload
	}

	# Send to MatchSimulationManager
	if manager and manager.has_method("send_user_command"):
		manager.send_user_command(cmd)
		print("[RadialDecisionUI] Sent command: %s/%s" % [intent, technique])

	# Reset to hidden
	_reset_to_hidden()


func _ensure_controller_registered(manager: Node) -> bool:
	if controller_id < 0:
		return false
	if _controller_registered:
		return true
	if manager == null or not manager.has_method("register_controller_slot"):
		return false

	var slot := _resolve_controller_slot()
	if slot < 0:
		return false

	var result = manager.register_controller_slot(controller_id, controlled_side, slot)
	var parsed = JSON.parse_string(str(result))
	if parsed is Dictionary and parsed.get("success", false):
		_controller_registered = true
		return true
	return false


func _resolve_controller_slot() -> int:
	if controlled_track_id < 0:
		return -1
	if controlled_side == "home":
		return controlled_track_id
	if controlled_side == "away":
		return controlled_track_id - 11
	return -1


func _map_intent_to_action(intent: String) -> String:
	match intent:
		"pass":
			return "pass"
		"shoot":
			return "shoot"
		"dribble":
			return "carry"
	return "pass"


# ============================================================================
# Input Locking (prevent double-tap)
# ============================================================================


func _is_input_locked() -> bool:
	return Time.get_ticks_msec() < _input_locked_until_ms


func _lock_input() -> void:
	_input_locked_until_ms = Time.get_ticks_msec() + INPUT_LOCK_MS


# ============================================================================
# Snapshot Integration (Phase 7)
# ============================================================================

var _last_ball_owner_id: int = -1


func _on_unified_snapshot(_t_ms: int, snapshot: Dictionary) -> void:
	# Skip if not in session mode
	if match_id.is_empty():
		return

	# Extract ball owner
	var ball_owner_id := _extract_ball_owner_id(snapshot)

	# Check if controlled player has the ball
	var has_ball := ball_owner_id == controlled_track_id
	var had_ball := _last_ball_owner_id == controlled_track_id

	_last_ball_owner_id = ball_owner_id

	# Handle possession changes
	if has_ball and not had_ball:
		# Just got possession
		_on_possession_gained(snapshot)
	elif not has_ball and had_ball:
		# Just lost possession
		_on_possession_lost()


func _extract_ball_owner_id(snapshot: Dictionary) -> int:
	# Method 1: Check ball.owner_id
	if snapshot.has("ball") and snapshot["ball"] is Dictionary:
		var ball := snapshot["ball"] as Dictionary
		if ball.has("owner_id"):
			return int(ball["owner_id"])

	# Method 2: Check ball_owner_track_id (legacy)
	if snapshot.has("ball_owner_track_id"):
		return int(snapshot["ball_owner_track_id"])

	# Method 3: Check ball_owner_idx (legacy)
	if snapshot.has("ball_owner_idx"):
		return int(snapshot["ball_owner_idx"])

	# Method 4: Check players array for has_ball flag
	if snapshot.has("players") and snapshot["players"] is Array:
		for p in snapshot["players"]:
			if p is Dictionary and p.has("has_ball") and p["has_ball"]:
				if p.has("track_id"):
					return int(p["track_id"])

	return -1  # No owner (loose ball)


func _on_possession_gained(snapshot: Dictionary) -> void:
	print("[RadialDecisionUI] Possession gained")

	# Don't show UI in FULL_AUTO mode
	if _current_mode == Mode.FULL_AUTO:
		print("[RadialDecisionUI] FULL_AUTO mode active, UI stays hidden")
		return

	# Update player position
	_update_radial_center_from_snapshot(snapshot)

	# NEW: Fetch FieldBoard data
	_fetch_field_board_data(snapshot)

	# Enter Tier 1 if hidden
	if _state == State.STATE_HIDDEN:
		_enter_state_tier1()


func _on_possession_lost() -> void:
	print("[RadialDecisionUI] Possession lost")
	_reset_to_hidden()


func _update_radial_center_from_snapshot(snapshot: Dictionary) -> void:
	# Find a match viewer that can convert field meters to screen pixels
	var viewer := _find_match_viewer()
	if not viewer:
		print("[RadialDecisionUI] WARNING: Match viewer not found, using center position")
		_radial_center = get_viewport_rect().size / 2.0
		return

	# Get player position from snapshot
	var player_pos_m := _get_player_field_pos(snapshot, controlled_track_id)
	if player_pos_m == Vector2.ZERO:
		print("[RadialDecisionUI] WARNING: Player position not found in snapshot")
		_radial_center = get_viewport_rect().size / 2.0
		return

	# Convert field position to screen position
	var screen_pos: Vector2 = viewer._field_to_screen(player_pos_m)

	# Clamp to viewport bounds with margin
	var viewport_size := get_viewport_rect().size
	var margin := OUTER_RADIUS_PX + VISUAL_BUTTON_SIZE
	_radial_center = Vector2(
		clamp(screen_pos.x, margin, viewport_size.x - margin), clamp(screen_pos.y, margin, viewport_size.y - margin)
	)

	print("[RadialDecisionUI] Radial center updated: ", _radial_center)


func _find_match_viewer() -> Node:
	# Phase 1: 캐시된 결과 사용 (유효성 검사 포함)
	if _cached_match_viewer and is_instance_valid(_cached_match_viewer):
		return _cached_match_viewer
	var root := get_tree().root
	_cached_match_viewer = _find_first_node_with_method(root, "_field_to_screen")
	return _cached_match_viewer


func _find_first_node_with_method(node: Node, method_name: String) -> Node:
	if node.has_method(method_name):
		return node

	for child in node.get_children():
		if child is Node:
			var found := _find_first_node_with_method(child, method_name)
			if found:
				return found

	return null


func _get_player_field_pos(snapshot: Dictionary, track_id: int) -> Vector2:
	if not snapshot.has("players"):
		return Vector2.ZERO

	var players = snapshot["players"]  # Variant type, checked with 'is' below
	if players is Dictionary:
		# Dictionary format: {track_id: player_data}
		if players.has(str(track_id)):
			var p := players[str(track_id)] as Dictionary
			if p.has("pos") and p["pos"] is Vector2:
				return p["pos"]
			elif p.has("x_m") and p.has("y_m"):
				return Vector2(float(p["x_m"]), float(p["y_m"]))
	elif players is Array:
		# Array format: [{track_id: ..., pos: ...}]
		for p in players:
			if p is Dictionary and p.has("track_id") and int(p["track_id"]) == track_id:
				if p.has("pos") and p["pos"] is Vector2:
					return p["pos"]
				elif p.has("x_m") and p.has("y_m"):
					return Vector2(float(p["x_m"]), float(p["y_m"]))

	return Vector2.ZERO


# ============================================================================
# Timeout Handling (Phase 2)
# ============================================================================


func _process(_delta: float) -> void:
	if _state == State.STATE_HIDDEN or _state == State.STATE_FULL_AUTO:
		return

	# Timeout disabled for testing
	# var elapsed := Time.get_ticks_msec() - _selection_started_ms
	# if elapsed > SELECTION_TIMEOUT_MS:
	# 	_handle_timeout()


func _handle_timeout() -> void:
	match _state:
		State.STATE_TIER1_ACTIVE:
			# Default to PASS and jump to Tier 3
			_selected_intent = "pass"
			_enter_state_tier3("pass")

		State.STATE_TIER3_ACTIVE:
			# Auto target
			var auto_target := {"type": "auto"}
			_send_user_command(_selected_intent, _selected_technique, auto_target)


func _get_default_technique(intent: String) -> String:
	match intent:
		"pass":
			return "pass"
		"shoot":
			return "placed"
		"dribble":
			return "carry"
	return "pass"


# ============================================================================
# Match OS Integration (v1.1)
# ============================================================================


## Fetch FieldBoard data from simulator
func _fetch_field_board_data(snapshot: Dictionary) -> void:
	# Lazy init simulator reference
	if not _simulator:
		_simulator = get_node_or_null("/root/FootballMatchSimulator")

	if not _simulator or not _simulator.has_method("get_field_board_snapshot"):
		_current_pressure = 0.5  # Fallback
		_immediate_pressure = 0.5
		return

	var fb = _simulator.get_field_board_snapshot()
	if fb.is_empty() or fb.has("error"):
		_current_pressure = 0.5
		_immediate_pressure = 0.5
		return

	# Extract player position from snapshot
	var player_pos_m = _get_player_pos_from_snapshot(snapshot)
	if player_pos_m == Vector2.ZERO:
		_current_pressure = 0.5
		_immediate_pressure = 0.5
		return

	# Calculate cell
	var cols = fb.get("cols", 28)
	var rows = fb.get("rows", 18)
	var cell_col = int(clamp(player_pos_m.x / 105.0 * cols, 0, cols - 1))
	var cell_row = int(clamp(player_pos_m.y / 68.0 * rows, 0, rows - 1))
	_current_cell = Vector2i(cell_col, cell_row)

	# Extract pressure (respect team side)
	var pressure_key = "pressure_against_home" if controlled_side == "home" else "pressure_against_away"
	var pressure_data = fb.get(pressure_key, PackedFloat32Array())

	if pressure_data.size() == cols * rows:
		var idx = cell_row * cols + cell_col
		_current_pressure = pressure_data[idx] / 3.0  # Normalize (max pressure = 3.0)
		_current_pressure = clamp(_current_pressure, 0.0, 1.0)
		print("[RadialDecisionUI] Pressure at cell %s: %.2f" % [_current_cell, _current_pressure])
	else:
		_current_pressure = 0.5

	# Match OS v1.2: Calculate immediate pressure (2m tackle threat)
	_immediate_pressure = _calculate_immediate_pressure(snapshot, player_pos_m)


func _get_player_pos_from_snapshot(snapshot: Dictionary) -> Vector2:
	if not snapshot.has("players"):
		return Vector2.ZERO

	var players = snapshot["players"]
	if players is Dictionary:
		if players.has(str(controlled_track_id)):
			var p = players[str(controlled_track_id)]
			if p.has("pos") and p["pos"] is Vector2:
				return p["pos"]
			elif p.has("x_m") and p.has("y_m"):
				return Vector2(float(p["x_m"]), float(p["y_m"]))
	elif players is Array:
		for p in players:
			if p is Dictionary and p.has("track_id") and int(p["track_id"]) == controlled_track_id:
				if p.has("pos") and p["pos"] is Vector2:
					return p["pos"]
				elif p.has("x_m") and p.has("y_m"):
					return Vector2(float(p["x_m"]), float(p["y_m"]))

	return Vector2.ZERO


## Match OS v1.2: Calculate immediate pressure (2m radius tackle threat)
func _calculate_immediate_pressure(snapshot: Dictionary, player_pos_m: Vector2) -> float:
	const R_IMMEDIATE_M = 2.0  # 2m radius for immediate tackle threat

	if not snapshot.has("players"):
		return 0.0

	var pressure = 0.0
	var players = snapshot["players"]

	# Get opponent track IDs (home: 11-21, away: 0-10)
	var opponent_start = 11 if controlled_side == "home" else 0
	var opponent_end = 21 if controlled_side == "home" else 10

	# Check all opponents
	var players_array = []
	if players is Dictionary:
		for key in players.keys():
			players_array.append(players[key])
	elif players is Array:
		players_array = players

	for p in players_array:
		if not p is Dictionary or not p.has("track_id"):
			continue

		var track_id = int(p["track_id"])
		if track_id < opponent_start or track_id > opponent_end:
			continue  # Not an opponent

		# Get opponent position
		var opp_pos = Vector2.ZERO
		if p.has("pos") and p["pos"] is Vector2:
			opp_pos = p["pos"]
		elif p.has("x_m") and p.has("y_m"):
			opp_pos = Vector2(float(p["x_m"]), float(p["y_m"]))
		else:
			continue

		# Calculate distance
		var dist = player_pos_m.distance_to(opp_pos)

		if dist < R_IMMEDIATE_M:
			# Quadratic falloff: closer = more pressure
			var contrib = pow(1.0 - dist / R_IMMEDIATE_M, 2.0)

			# Weight by opponent stamina (if available)
			var opp_stamina = 1.0
			if p.has("stamina"):
				opp_stamina = float(p["stamina"])
			var w_defender = opp_stamina * 0.7 + 0.3  # 30% baseline even when exhausted

			pressure += contrib * w_defender

	return clamp(pressure, 0.0, 1.0)


func _show_keep_tooltip() -> void:
	_hide_tooltip()  # Remove old tooltip if any

	var tooltip = PanelContainer.new()
	tooltip.name = "KEEPTooltip"

	# Position above KEEP slot (angle = -PI/2, radius = OUTER_RADIUS + 80)
	var tooltip_offset = Vector2(0, -(OUTER_RADIUS_PX + 80))
	tooltip.position = _radial_center + tooltip_offset - Vector2(100, 0)  # Center horizontally
	tooltip.custom_minimum_size = Vector2(200, 100)

	# Panel style
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.1, 0.1, 0.1, 0.9)
	style.corner_radius_top_left = 8
	style.corner_radius_top_right = 8
	style.corner_radius_bottom_left = 8
	style.corner_radius_bottom_right = 8
	tooltip.add_theme_stylebox_override("panel", style)

	# Build content
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 12)
	margin.add_theme_constant_override("margin_top", 8)
	margin.add_theme_constant_override("margin_right", 12)
	margin.add_theme_constant_override("margin_bottom", 8)
	tooltip.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 4)
	margin.add_child(vbox)

	# Title
	var title = Label.new()
	title.text = "KEEP Recommended"
	title.add_theme_font_size_override("font_size", 18)
	title.add_theme_color_override("font_color", Color(0.9, 0.9, 0.1))
	vbox.add_child(title)

	# Separator
	var sep = HSeparator.new()
	vbox.add_child(sep)

	# Tactical Pressure (4m FieldBoard)
	var tactical_label = Label.new()
	tactical_label.text = "Tactical: %s (%.2f)" % [_get_pressure_bar(_current_pressure), _current_pressure]
	tactical_label.add_theme_font_size_override("font_size", 14)
	vbox.add_child(tactical_label)

	# Immediate Pressure (2m tackle threat) - Match OS v1.2
	var immediate_label = Label.new()
	immediate_label.text = "Immediate: %s (%.2f)" % [_get_pressure_bar(_immediate_pressure), _immediate_pressure]
	immediate_label.add_theme_font_size_override("font_size", 14)
	var color = Color(1.0, 0.3, 0.3) if _immediate_pressure > 0.7 else Color(0.9, 0.9, 0.9)
	immediate_label.add_theme_color_override("font_color", color)
	vbox.add_child(immediate_label)

	# Zone (based on immediate pressure for tackle threat)
	var zone_label = Label.new()
	zone_label.text = "Zone: %s" % _get_zone_text(_immediate_pressure)
	zone_label.add_theme_font_size_override("font_size", 14)
	vbox.add_child(zone_label)

	# Advice
	var advice_label = Label.new()
	advice_label.text = _get_advice_text(_current_pressure)
	advice_label.add_theme_font_size_override("font_size", 12)
	advice_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	advice_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(advice_label)

	add_child(tooltip)
	_tooltip_panel = tooltip

	# Fade in
	tooltip.modulate.a = 0.0
	var tween = create_tween()
	tween.tween_property(tooltip, "modulate:a", 1.0, 0.15)


func _hide_tooltip() -> void:
	if _tooltip_panel:
		_tooltip_panel.queue_free()
		_tooltip_panel = null


func _get_pressure_bar(pressure: float) -> String:
	var filled = int(pressure * 5)
	var empty = 5 - filled
	var bar = ""
	for i in range(filled):
		bar += "●"
	for i in range(empty):
		bar += "○"
	return bar


func _get_zone_text(pressure: float) -> String:
	if pressure < 0.3:
		return "Safe"
	elif pressure < 0.7:
		return "Medium Risk"
	else:
		return "High Risk"


func _get_advice_text(pressure: float) -> String:
	if pressure > 0.7:
		return "Hold ball, wait for space"
	elif pressure > 0.4:
		return "Look for passing options"
	else:
		return "Space available"


func _on_info_pressed() -> void:
	print("[RadialDecisionUI] Info button pressed")

	var panel = MatchOSInfoPanel.new()
	panel.update_data(
		{
			"local_pressure": _current_pressure,
			"immediate_pressure": _immediate_pressure,  # Match OS v1.2
			"cell": _current_cell,
			"track_id": controlled_track_id
		}
	)
	panel.panel_closed.connect(func(): print("[RadialDecisionUI] Info panel closed"))

	# Add to scene tree (find viewport root)
	var root = get_tree().root
	root.add_child(panel)


# ============================================================================
# Debug
# ============================================================================


func _to_string() -> String:
	return "[RadialDecisionUI] State: %s | Center: %s" % [State.keys()[_state], str(_radial_center)]
