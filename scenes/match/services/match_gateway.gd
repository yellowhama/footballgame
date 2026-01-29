# Gateway that encapsulates all Rust calls; scene-local to avoid global init.

extends Node
class_name MatchGateway

# NOTE: replace with actual GDExtension binding class name
var _rust: Object = null


func _ready() -> void:
	# Lazy-resolve rust extension only when match scene is active
	if ClassDB.class_exists("FootballMatchSimulator"):
		_rust = ClassDB.instantiate("FootballMatchSimulator")
		print("[MatchGateway] Rust bound")
	else:
		push_warning("[MatchGateway] Rust extension not found; using mock")


func simulate_json(plan: Dictionary, budget_ms: int = 50) -> Dictionary:
	if _rust != null and _rust.has_method("simulate_match_json_budget"):
		var req := plan.duplicate(true)
		req["budget"] = {"wall_time_ms": budget_ms}
		var res_json: String = _rust.simulate_match_json_budget(JSON.stringify(req))
		return JSON.parse_string(res_json)
	# fallback mock
	return {"partial": true, "reason": "mock", "events": [], "home_score": 0, "away_score": 0}
