# Rust GDExtension wrapper with health checks and budget system
extends Node

const MatchGatewayPolicy = preload("res://bridge/match_gateway_policy.gd")

var _rust: Object
var _healthy := false
var _last_health_check := 0

signal gateway_error(error_message: String)


func _ready() -> void:
	_init_rust_connection()


func _init_rust_connection() -> void:
	if not ClassDB.class_exists("FootballMatchSimulator"):
		push_warning("[MatchGateway] FootballMatchSimulator class not found")
		_healthy = false
		return

	_rust = ClassDB.instantiate("FootballMatchSimulator")
	if _rust:
		print(
			"[MatchGateway][DIAG] Gateway instantiated FootballMatchSimulator instance_id=",
			str(_rust.get_instance_id())
		)
	if not _rust:
		push_warning("[MatchGateway] Failed to instantiate FootballMatchSimulator")
		_healthy = false
		return

	# Test connection
	_test_connection()


func _test_connection() -> void:
	if not _rust:
		_healthy = false
		return

	var test_result = _rust.call("test_connection")
	_healthy = bool(test_result) or str(test_result).length() > 0
	_last_health_check = Time.get_ticks_msec()

	print("[MatchGateway] Rust connection test: ", "✅ healthy" if _healthy else "❌ unhealthy")


func is_healthy() -> bool:
	# Periodic health check
	if MatchGatewayPolicy.should_health_check(_last_health_check):
		_test_connection()

	return _healthy


func _inject_budget(plan: Dictionary, wall_ms: int) -> Dictionary:
	var req := plan.duplicate(true)
	req["budget"] = {"wall_time_ms": MatchGatewayPolicy.clamp_wall(wall_ms)}
	return req


func simulate_json(plan: Dictionary, budget_ms: int = -1) -> Dictionary:
	if not is_healthy():
		emit_signal("gateway_error", "Rust gateway unhealthy")
		return _error_response("Gateway unhealthy")

	var wall_ms = budget_ms if budget_ms > 0 else MatchGatewayPolicy.get_default_budget_ms()
	var req = _inject_budget(plan, wall_ms)
	var attempt = 0
	var max_attempts = MatchGatewayPolicy.get_retry_count() + 1

	while attempt < max_attempts:
		attempt += 1

		# Call Rust simulation
		var res_json = _rust.call("simulate_match_json_budget", JSON.stringify(req))

		if res_json == null or res_json == "":
			if attempt < max_attempts:
				print("[MatchGateway] Attempt ", attempt, " failed, retrying...")
				await get_tree().create_timer(MatchGatewayPolicy.get_retry_backoff_ms() / 1000.0).timeout
				continue
			else:
				emit_signal("gateway_error", "Rust call returned null after retries")
				return _error_response("Rust call failed")

		# Parse response
		var parsed = JSON.parse_string(str(res_json))
		if parsed == null:
			if attempt < max_attempts:
				print("[MatchGateway] JSON parse failed, retrying...")
				await get_tree().create_timer(MatchGatewayPolicy.get_retry_backoff_ms() / 1000.0).timeout
				continue
			else:
				emit_signal("gateway_error", "JSON parse failed after retries")
				return _error_response("JSON parse failed")

		# Success
		print("[MatchGateway] Simulation successful (attempt ", attempt, ")")
		return parsed

	return _error_response("Max retries exceeded")


func _error_response(message: String) -> Dictionary:
	return {"success": false, "error": message, "home_score": 0, "away_score": 0, "events": [], "partial": true}


# Direct method access for testing
func call_rust_direct(method: String, args = null) -> Variant:
	if not is_healthy():
		return null

	if args != null:
		return _rust.call(method, args)
	else:
		return _rust.call(method)
