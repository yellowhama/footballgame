extends Control
class_name PlayerHUD

# ========== Career Player Mode: Phase 3 - 3-Stage Selection UI ==========
# State machine for Intent → Technique → Target selection
# Connects to MatchSimulationManager.send_user_command()

# ========== State Machine ==========
enum State { STATE_IDLE, STATE_PICK_INTENT, STATE_PICK_TECHNIQUE, STATE_PICK_TARGET }  # Off-ball or waiting  # Show 3 intent buttons  # Show TechniqueBox  # Show TargetSelectOverlay + Connector

var _state: State = State.STATE_IDLE
var _selected_intent: String = ""
var _selected_technique: String = ""
var _selection_started_ms: int = 0
const SELECTION_TIMEOUT_MS := 1500

# ========== Exports ==========
@export var controlled_track_id: int = 9
@export var controlled_side: String = "home"
@export var controller_id: int = -1
@export var match_id: String = "session_001"
@export var enable_sticky_hotkeys: bool = true

const STICKY_KEY_SPRINT := KEY_Q
const STICKY_KEY_DRIBBLE := KEY_E
const STICKY_KEY_PRESS := KEY_R
const STICKY_ICON_ACTIVE := Color(0.2, 1.0, 0.2, 0.95)
const STICKY_ICON_INACTIVE := Color(0.6, 0.6, 0.6, 0.7)

# ========== State ==========
var _is_on_ball: bool = false
var _ball_owner_track_id: int = -1
var _input_locked_until_ms: int = 0
const INPUT_LOCK_MS := 300
var _sticky_actions: Dictionary = {"sprint": false, "dribble": false, "press": false}
var _controller_registered: bool = false

# ========== Node References ==========
@onready var intent_buttons: HBoxContainer = $IntentButtons
@onready var btn_pass: Button = $IntentButtons/BtnPass
@onready var btn_shoot: Button = $IntentButtons/BtnShoot
@onready var btn_dribble: Button = $IntentButtons/BtnDribble

@onready var technique_box: Control = $TechniqueBox
@onready var target_overlay: Control = $TargetSelectOverlay
@onready var connector: Control = $SelectionConnector

@onready var label_debug: Label = $LabelDebug
@onready var sticky_icons: HBoxContainer = $StickyIcons
@onready var sticky_icon_sprint: Label = $StickyIcons/StickySprint
@onready var sticky_icon_dribble: Label = $StickyIcons/StickyDribble
@onready var sticky_icon_press: Label = $StickyIcons/StickyPress


# ========== Initialization ==========
func _ready() -> void:
	add_to_group("player_hud")
	var pipeline = get_node_or_null("/root/UnifiedFramePipeline")
	if pipeline and pipeline.has_signal("snapshot_ready"):
		pipeline.snapshot_ready.connect(_on_unified_snapshot)
		print("[PlayerHUD] Connected to UnifiedFramePipeline")
	else:
		print("[PlayerHUD] WARNING: UnifiedFramePipeline not available")

	set_process_unhandled_input(true)
	_wire_intent_buttons()
	_wire_component_signals()
	_reset_to_idle()
	_update_sticky_icons()


func _wire_intent_buttons() -> void:
	btn_pass.pressed.connect(func(): _on_intent_clicked("pass"))
	btn_shoot.pressed.connect(func(): _on_intent_clicked("shoot"))
	btn_dribble.pressed.connect(func(): _on_intent_clicked("dribble"))


func _wire_component_signals() -> void:
	# TechniqueBox signal
	if technique_box and technique_box.has_signal("technique_selected"):
		technique_box.technique_selected.connect(_on_technique_clicked)
		print("[PlayerHUD] Connected to TechniqueBox.technique_selected")

	# TargetSelectOverlay signal
	if target_overlay and target_overlay.has_signal("target_selected"):
		target_overlay.target_selected.connect(_on_target_clicked)
		print("[PlayerHUD] Connected to TargetSelectOverlay.target_selected")


# ========== Snapshot Handler ==========
func _on_unified_snapshot(_t_ms: int, snapshot: Dictionary) -> void:
	_ball_owner_track_id = _extract_ball_owner_track_id(snapshot)
	_is_on_ball = (_ball_owner_track_id == controlled_track_id)

	# On-ball transition: IDLE → PICK_INTENT
	if _is_on_ball and _state == State.STATE_IDLE:
		if not _is_input_locked():
			_enter_state_pick_intent()
	# Off-ball transition: ANY → IDLE
	elif not _is_on_ball and _state != State.STATE_IDLE:
		_reset_to_idle()

	_update_debug_label()
	_update_sticky_icons()


func _extract_ball_owner_track_id(snapshot: Dictionary) -> int:
	if snapshot.has("ball_owner_track_id"):
		return int(snapshot["ball_owner_track_id"])
	elif snapshot.has("ball_owner_idx"):
		return int(snapshot["ball_owner_idx"])

	# Fallback: check players array
	if snapshot.has("players") and snapshot["players"] is Array:
		for p in snapshot["players"]:
			if p is Dictionary and p.has("has_ball") and p["has_ball"]:
				if p.has("track_id"):
					return int(p["track_id"])

	return -1


# ========== State Transitions ==========
func _enter_state_pick_intent() -> void:
	print("[PlayerHUD] State: IDLE → PICK_INTENT")
	_state = State.STATE_PICK_INTENT
	_selection_started_ms = Time.get_ticks_msec()
	intent_buttons.visible = true
	technique_box.visible = false
	target_overlay.visible = false
	connector.visible = false


func _on_intent_clicked(intent: String) -> void:
	if _state != State.STATE_PICK_INTENT:
		print("[PlayerHUD] Ignored intent click (wrong state)")
		return

	print("[PlayerHUD] Intent selected: %s" % intent)
	_selected_intent = intent
	_enter_state_pick_technique()


func _enter_state_pick_technique() -> void:
	print("[PlayerHUD] State: PICK_INTENT → PICK_TECHNIQUE")
	_state = State.STATE_PICK_TECHNIQUE
	_selection_started_ms = Time.get_ticks_msec()
	intent_buttons.visible = false

	if technique_box and technique_box.has_method("show_techniques"):
		technique_box.show_techniques(_selected_intent)
		technique_box.visible = true
	else:
		print("[PlayerHUD] ERROR: TechniqueBox missing show_techniques()")
		# Fallback: auto-select default technique
		_selected_technique = _get_default_technique(_selected_intent)
		_enter_state_pick_target()


func _on_technique_clicked(technique: String) -> void:
	if _state != State.STATE_PICK_TECHNIQUE:
		print("[PlayerHUD] Ignored technique click (wrong state)")
		return

	print("[PlayerHUD] Technique selected: %s" % technique)
	_selected_technique = technique
	_enter_state_pick_target()


func _enter_state_pick_target() -> void:
	print("[PlayerHUD] State: PICK_TECHNIQUE → PICK_TARGET")
	_state = State.STATE_PICK_TARGET
	_selection_started_ms = Time.get_ticks_msec()
	technique_box.visible = false

	if target_overlay and target_overlay.has_method("show_targets"):
		target_overlay.show_targets(_selected_intent, _selected_technique)
		target_overlay.visible = true
	else:
		print("[PlayerHUD] ERROR: TargetSelectOverlay missing show_targets()")

	if connector and connector.has_method("update_connection"):
		connector.update_connection()
		connector.visible = true
	else:
		print("[PlayerHUD] WARNING: SelectionConnector not available")


func _on_target_clicked(target_data: Dictionary) -> void:
	if _state != State.STATE_PICK_TARGET:
		print("[PlayerHUD] Ignored target click (wrong state)")
		return

	print("[PlayerHUD] Target selected: %s" % JSON.stringify(target_data))
	_send_user_command(_selected_intent, _selected_technique, target_data)
	_reset_to_idle()


func _reset_to_idle() -> void:
	if _state != State.STATE_IDLE:
		print("[PlayerHUD] State: %s → IDLE" % State.keys()[_state])

	_state = State.STATE_IDLE
	_selected_intent = ""
	_selected_technique = ""
	intent_buttons.visible = false
	technique_box.visible = false
	target_overlay.visible = false
	connector.visible = false


# ========== Timeout Handler ==========
func _process(_delta: float) -> void:
	if _state == State.STATE_IDLE:
		return

	var elapsed = Time.get_ticks_msec() - _selection_started_ms
	if elapsed > SELECTION_TIMEOUT_MS:
		_handle_timeout()


func _unhandled_input(event: InputEvent) -> void:
	if not enable_sticky_hotkeys:
		return
	if not (event is InputEventKey):
		return
	if not event.pressed or event.echo:
		return

	match event.keycode:
		STICKY_KEY_SPRINT:
			_toggle_sticky_action("sprint")
		STICKY_KEY_DRIBBLE:
			_toggle_sticky_action("dribble")
		STICKY_KEY_PRESS:
			_toggle_sticky_action("press")


func _handle_timeout() -> void:
	print("[PlayerHUD] Timeout at state: %s" % State.keys()[_state])

	match _state:
		State.STATE_PICK_INTENT:
			_selected_intent = "pass"  # Default intent
			print("[PlayerHUD] Auto-selected intent: pass")
			_enter_state_pick_technique()

		State.STATE_PICK_TECHNIQUE:
			_selected_technique = _get_default_technique(_selected_intent)
			print("[PlayerHUD] Auto-selected technique: %s" % _selected_technique)
			_enter_state_pick_target()

		State.STATE_PICK_TARGET:
			var auto_target = {"type": "auto"}
			print("[PlayerHUD] Auto-selected target: AUTO")
			_send_user_command(_selected_intent, _selected_technique, auto_target)
			_reset_to_idle()


func _get_default_technique(intent: String) -> String:
	match intent:
		"pass":
			return "pass"
		"shoot":
			return "placed"
		"dribble":
			return "carry"
	return "pass"


func _toggle_sticky_action(action: String) -> void:
	var next_state = not bool(_sticky_actions.get(action, false))
	_sticky_actions[action] = next_state

	var manager = get_node_or_null("/root/MatchSimulationManager")
	if manager and manager.has_method("set_sticky_action"):
		manager.set_sticky_action(controlled_track_id, action, next_state)
		print("[PlayerHUD] Sticky action %s=%s (track_id=%d)" % [action, str(next_state), controlled_track_id])
	else:
		print("[PlayerHUD] ERROR: MatchSimulationManager not available for sticky actions")
	_update_sticky_icons()


# ========== Command Sending ==========
func _send_user_command(intent: String, technique: String, target: Dictionary) -> void:
	_lock_input()

	# Map to backwards-compatible format (Phase 2 payload structure)
	var action = _map_intent_to_action(intent)
	var payload := {"cmd": "on_ball_action", "action": action}

	# Add variant if technique differs from base action
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

	# Build full command
	var manager = get_node_or_null("/root/MatchSimulationManager")
	if controller_id >= 0:
		if not _ensure_controller_registered(manager):
			print("[PlayerHUD] ERROR: Controller registration failed")
			return
		var cmd := {"controller_id": controller_id, "payload": payload}
		if manager and manager.has_method("send_multi_agent_commands"):
			manager.send_multi_agent_commands([cmd])
			print("[PlayerHUD] Sent multi-agent: %s/%s → %s" % [intent, technique, target.get("type", "unknown")])
		else:
			print("[PlayerHUD] ERROR: MatchSimulationManager not available for multi-agent")
		return

	# Full command (single controller)
	var cmd := {
		"mode": "career_player",
		"match_id": match_id,
		"side": controlled_side,
		"controlled_track_id": controlled_track_id,
		"payload": payload
	}

	# Send via MatchSimulationManager
	if manager and manager.has_method("send_user_command"):
		manager.send_user_command(cmd)
		print("[PlayerHUD] Sent: %s/%s → %s" % [intent, technique, target.get("type", "unknown")])
	else:
		print("[PlayerHUD] ERROR: MatchSimulationManager not available")


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


# ========== Input Lock Helpers ==========
func _is_input_locked() -> bool:
	return Time.get_ticks_msec() < _input_locked_until_ms


func _lock_input() -> void:
	_input_locked_until_ms = Time.get_ticks_msec() + INPUT_LOCK_MS
	print("[PlayerHUD] Input locked for %dms" % INPUT_LOCK_MS)


func get_remaining_lock_time() -> float:
	if not _is_input_locked():
		return 0.0
	var now_ms = Time.get_ticks_msec()
	return max(0.0, (_input_locked_until_ms - now_ms) / 1000.0)


# ========== Debug ==========
func _update_debug_label() -> void:
	var state_name = State.keys()[_state]
	var lock_status = ""
	if _is_input_locked():
		var remaining = get_remaining_lock_time()
		lock_status = " | LOCKED (%.2fs)" % remaining

	var sticky_status = ""
	if enable_sticky_hotkeys:
		sticky_status = (
			" | Sticky S:%s D:%s P:%s"
			% [
				str(_sticky_actions.get("sprint", false)),
				str(_sticky_actions.get("dribble", false)),
				str(_sticky_actions.get("press", false)),
			]
		)

	label_debug.text = (
		"State: %s | Owner: %d | On-ball: %s%s%s"
		% [
			state_name,
			_ball_owner_track_id,
			str(_is_on_ball),
			lock_status,
			sticky_status,
		]
	)


func _update_sticky_icons() -> void:
	if sticky_icons == null:
		return
	_set_sticky_icon(sticky_icon_sprint, bool(_sticky_actions.get("sprint", false)))
	_set_sticky_icon(sticky_icon_dribble, bool(_sticky_actions.get("dribble", false)))
	_set_sticky_icon(sticky_icon_press, bool(_sticky_actions.get("press", false)))


func _set_sticky_icon(label: Label, active: bool) -> void:
	if label == null:
		return
	label.modulate = STICKY_ICON_ACTIVE if active else STICKY_ICON_INACTIVE
