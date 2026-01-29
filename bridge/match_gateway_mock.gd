# Deterministic mock fallback implementation
extends Node

const MatchGatewayPolicy = preload("res://bridge/match_gateway_policy.gd")

var _seed_rng: RandomNumberGenerator
var _healthy := true

signal gateway_error(error_message: String)


func _ready() -> void:
	_seed_rng = RandomNumberGenerator.new()
	_seed_rng.seed = 42  # Deterministic seed
	print("[MatchGatewayMock] Mock gateway initialized")


func is_healthy() -> bool:
	return _healthy


func simulate_json(plan: Dictionary, budget_ms: int = -1) -> Dictionary:
	# Simulate processing time
	var wall_ms = budget_ms if budget_ms > 0 else MatchGatewayPolicy.get_default_budget_ms()
	var process_time = minf(wall_ms, 20)  # Mock never takes more than 20ms

	if process_time > 10:
		await get_tree().create_timer(process_time / 1000.0).timeout

	# Extract team data from plan
	var home_team = plan.get("home_team", {"name": "Home", "rating": 75})
	var away_team = plan.get("away_team", {"name": "Away", "rating": 75})

	# Deterministic score calculation based on team ratings
	var home_rating = home_team.get("rating", 75)
	var away_rating = away_team.get("rating", 75)

	# Use team names as additional seed
	var team_hash = (home_team.get("name", "Home") + away_team.get("name", "Away")).hash()
	_seed_rng.seed = abs(team_hash) % 1000000

	# Calculate goal probability based on rating difference
	var rating_diff = home_rating - away_rating
	var home_prob = 0.5 + (rating_diff * 0.002)  # ±0.1 per 50 rating points
	home_prob = clampf(home_prob, 0.1, 0.9)

	# Generate deterministic scores
	var home_score = 0
	var away_score = 0
	var total_goals = _seed_rng.randi_range(0, 5)  # 0-5 goals per match

	# Distribute goals
	for i in total_goals:
		if _seed_rng.randf() < home_prob:
			home_score += 1
		else:
			away_score += 1

	# Generate mock events
	var events = _generate_mock_events(home_score, away_score, plan)

	var result = {
		"success": true,
		"home_score": home_score,
		"away_score": away_score,
		"events": events,
		"partial": false,
		"wall_time_used": int(process_time),
		"engine": "mock_deterministic"
	}

	print(
		"[MatchGatewayMock] Simulated: ",
		home_team.get("name", "Home"),
		" ",
		home_score,
		"-",
		away_score,
		" ",
		away_team.get("name", "Away")
	)
	return result


func _generate_mock_events(home_goals: int, away_goals: int, plan: Dictionary) -> Array:
	var events = []
	var minute = 1

	# Add kickoff
	events.append({"type": "kickoff", "minute": 0, "description": "Match begins"})

	# Add goal events
	var total_goals = home_goals + away_goals
	var goals_added = 0

	while goals_added < total_goals and minute <= 90:
		minute += _seed_rng.randi_range(5, 20)
		if minute > 90:
			minute = 90

		var is_home_goal = goals_added < home_goals
		if home_goals > 0 and away_goals > 0:
			# Mixed scoring - use probability
			is_home_goal = _seed_rng.randf() < (float(home_goals) / total_goals)

		var team = "home" if is_home_goal else "away"
		var player = "Player" + str(_seed_rng.randi_range(1, 11))

		events.append(
			{
				"type": "goal",
				"minute": minute,
				"team": team,
				"player": player,
				"description": team.capitalize() + " team scores! " + player
			}
		)

		goals_added += 1

	# Add final whistle
	events.append({"type": "final_whistle", "minute": 90, "description": "Match ends"})

	return events


# Mock error testing
func trigger_mock_error(error_type: String = "timeout") -> void:
	_healthy = false
	emit_signal("gateway_error", "Mock error: " + error_type)

	# Recover after delay
	await get_tree().create_timer(1.0).timeout
	_healthy = true


# Direct method access for testing compatibility
func call_rust_direct(method: String, args = null) -> Variant:
	print("[MatchGatewayMock] Mock call: ", method)

	match method:
		"test_connection":
			return "mock_connection_ok"
		"get_engine_version":
			return "mock_v1.0"
		_:
			return {"mock": true, "method": method, "args": args}
