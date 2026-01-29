extends Node
## Global simulation lock to prevent concurrent match simulations
## Shared by MatchManager and MatchSimulationManager
##
## P0.5 STABILIZATION - Complete structural seal against duplicate simulation calls
## See: docs/specs/FIX_2512/1223/PLAYER_CACHE_P0_5_STABILIZATION.md

signal lock_acquired(token: String)
signal lock_released(token: String)
signal lock_timeout(token: String)

const LOCK_TIMEOUT_SECONDS: float = 120.0

var _locked: bool = false
var _lock_token: String = ""
var _lock_acquired_at: float = 0.0
var _lock_timeout_timer: Timer = null


func _ready() -> void:
	# Create timeout timer
	_lock_timeout_timer = Timer.new()
	_lock_timeout_timer.one_shot = true
	_lock_timeout_timer.timeout.connect(_on_lock_timeout)
	add_child(_lock_timeout_timer)

	print("[SimulationLock] Initialized")


func _process(_delta: float) -> void:
	# Safety check: timeout old locks
	if _locked and Time.get_ticks_msec() / 1000.0 - _lock_acquired_at > LOCK_TIMEOUT_SECONDS:
		push_warning(
			"[SimulationLock] TIMEOUT: Force-releasing lock '%s' after %.1fs" % [_lock_token, LOCK_TIMEOUT_SECONDS]
		)
		force_release()


## Attempt to acquire lock
## Returns: {success: bool, token: String, reason: String}
func try_acquire(requester: String) -> Dictionary:
	if _locked:
		return {
			"success": false,
			"token": "",
			"reason":
			"Lock held by '%s' (age: %.1fs)" % [_lock_token, Time.get_ticks_msec() / 1000.0 - _lock_acquired_at]
		}

	# Generate unique token
	var token := "%s_%d" % [requester, Time.get_ticks_msec()]

	# Acquire lock
	_locked = true
	_lock_token = token
	_lock_acquired_at = Time.get_ticks_msec() / 1000.0

	# Start timeout timer
	_lock_timeout_timer.start(LOCK_TIMEOUT_SECONDS)

	print("[SimulationLock] 🔒 LOCK acquired: %s" % token)
	lock_acquired.emit(token)

	return {"success": true, "token": token, "reason": ""}


## Release lock by token (must match current lock)
func release(token: String) -> bool:
	if not _locked:
		push_warning("[SimulationLock] Release called but not locked (token=%s)" % token)
		return false

	if _lock_token != token:
		push_error("[SimulationLock] Token mismatch! Expected '%s', got '%s'" % [_lock_token, token])
		return false

	_do_release()
	return true


## Force release (timeout/emergency)
func force_release() -> void:
	if _locked:
		push_warning("[SimulationLock] FORCE release: %s" % _lock_token)
		lock_timeout.emit(_lock_token)
		_do_release()


func _do_release() -> void:
	var old_token := _lock_token

	_locked = false
	_lock_token = ""
	_lock_acquired_at = 0.0
	_lock_timeout_timer.stop()

	print("[SimulationLock] 🔓 UNLOCK: %s" % old_token)
	lock_released.emit(old_token)


func _on_lock_timeout() -> void:
	if _locked:
		force_release()


## Query lock state (for debugging)
func is_locked() -> bool:
	return _locked


func get_lock_info() -> Dictionary:
	return {
		"locked": _locked,
		"token": _lock_token,
		"age_seconds": Time.get_ticks_msec() / 1000.0 - _lock_acquired_at if _locked else 0.0
	}
