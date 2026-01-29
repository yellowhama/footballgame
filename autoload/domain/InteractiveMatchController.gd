class_name InteractiveMatchController
extends RefCounted
## Phase E: Interactive Match Controller
## Orchestrates interactive match flow with bullet-time interventions.
##
## Usage:
##   var controller = InteractiveMatchController.new()
##   controller.intervention_requested.connect(_on_intervention)
##   controller.match_finished.connect(_on_match_finished)
##   controller.start_interactive_match(match_request)

## Emitted when the simulation pauses for user intervention.
## context contains: player_id, time_seconds, position, shoot_prob, dribble_prob, pass_targets
signal intervention_requested(context: Dictionary)

## Emitted when the match finishes (no more interventions possible).
## result contains the final match result data.
signal match_finished(result: Dictionary)

## Emitted when simulation state changes (for UI updates).
signal state_changed(state: String)

## Emitted on errors.
signal error_occurred(message: String)

enum MatchState {
	IDLE,
	RUNNING,
	PAUSED,
	FINISHED,
}

var _current_state: MatchState = MatchState.IDLE
var _last_context: Dictionary = {}
var _match_request: Dictionary = {}


## Start an interactive match. Returns true if started successfully.
func start_interactive_match(match_request: Dictionary) -> bool:
	if _current_state != MatchState.IDLE:
		push_warning("[InteractiveMatchController] Cannot start: match already in progress")
		return false

	_match_request = match_request
	_current_state = MatchState.RUNNING
	state_changed.emit("starting")

	var state_bytes := FootballRustEngine.start_interactive_match_binary(match_request)
	if state_bytes.is_empty():
		_current_state = MatchState.IDLE
		error_occurred.emit("Failed to start interactive match")
		return false

	_handle_state_bytes(state_bytes)
	return true


## Resume the match with the given action.
## action = { "type": "shoot" | "dribble" | "pass_to", "target_id": int (for pass_to) }
func resume_with_action(action: Dictionary) -> void:
	if _current_state != MatchState.PAUSED:
		push_warning("[InteractiveMatchController] Cannot resume: not in PAUSED state")
		return

	_current_state = MatchState.RUNNING
	state_changed.emit("resuming")

	var state_bytes := FootballRustEngine.resume_interactive_match_binary(action)
	if state_bytes.is_empty():
		error_occurred.emit("Failed to resume interactive match")
		return

	_handle_state_bytes(state_bytes)


## Auto-continue without user input (AI decides based on probabilities).
func auto_continue() -> void:
	if _current_state != MatchState.PAUSED:
		return

	var action := _auto_select_action(_last_context)
	resume_with_action(action)


## Get the current match state.
func get_state() -> MatchState:
	return _current_state


## Get the last intervention context (only valid when state is PAUSED).
func get_last_context() -> Dictionary:
	return _last_context


## Check if the match is in progress.
func is_running() -> bool:
	return _current_state == MatchState.RUNNING or _current_state == MatchState.PAUSED


## Reset the controller to IDLE state.
func reset() -> void:
	_current_state = MatchState.IDLE
	_last_context = {}
	_match_request = {}
	state_changed.emit("idle")


func _handle_state_bytes(state_bytes: PackedByteArray) -> void:
	var state := FootballRustEngine.decode_interactive_state(state_bytes)

	match state.get("state", "invalid"):
		"running":
			_current_state = MatchState.RUNNING
			state_changed.emit("running")
			# Auto-continue simulation (call resume with no action)
			# In a real implementation, this would be called in a loop or timer
			# For now, we emit a signal and let the caller decide

		"paused":
			_current_state = MatchState.PAUSED
			_last_context = state
			state_changed.emit("paused")
			intervention_requested.emit(state)

		"finished":
			_current_state = MatchState.FINISHED
			state_changed.emit("finished")
			# Convert to match result format expected by existing systems
			var result := _build_match_result_from_state(state)
			match_finished.emit(result)

		_:
			error_occurred.emit("Invalid state received: %s" % str(state))


func _build_match_result_from_state(state: Dictionary) -> Dictionary:
	# Phase E: Finished payload must come from the engine (SSOT).
	# state is expected to include:
	#   - result_json (MatchResult JSON)
	#   - replay_json (ReplayDoc JSON, optional)
	var result_json := str(state.get("result_json", "")).strip_edges()
	if result_json == "":
		return {
			"error": true,
			"message": "Interactive finished state missing result_json",
			"interactive_state": state,
			"match_request": _match_request.duplicate(true),
		}

	var parser := JSON.new()
	if parser.parse(result_json) != OK:
		return {
			"error": true,
			"message": "Failed to parse interactive result_json: %s" % parser.get_error_message(),
			"interactive_state": state,
			"match_request": _match_request.duplicate(true),
		}

	if typeof(parser.data) != TYPE_DICTIONARY:
		return {
			"error": true,
			"message": "Unexpected interactive result_json format (not a Dictionary)",
			"interactive_state": state,
			"match_request": _match_request.duplicate(true),
		}

	var result: Dictionary = parser.data as Dictionary
	# Attach replay JSON for consumers that want to persist/view it.
	if state.has("replay_json"):
		result["replay_json"] = state.get("replay_json", "")
	result["interactive"] = true
	return result


func _auto_select_action(context: Dictionary) -> Dictionary:
	# Simple AI: choose action based on highest probability
	var shoot_prob: float = context.get("shoot_prob", 0.0)
	var dribble_prob: float = context.get("dribble_prob", 0.0)

	var best_pass_prob: float = 0.0
	var best_pass_id: int = 0
	var pass_targets: Array = context.get("pass_targets", [])

	for target in pass_targets:
		var prob: float = float(target.get("success_prob", 0.0))
		if prob > best_pass_prob:
			best_pass_prob = prob
			best_pass_id = int(target.get("id", 0))

	# Choose the action with highest success probability
	if shoot_prob >= dribble_prob and shoot_prob >= best_pass_prob:
		return {"type": "shoot"}
	elif dribble_prob >= best_pass_prob:
		return {"type": "dribble"}
	else:
		return {"type": "pass_to", "target_id": best_pass_id}
