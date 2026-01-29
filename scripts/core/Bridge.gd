extends Node

# Bridge between Enhanced UI and Core systems
# Handles both legacy (12-skill) and enhanced (42-skill) systems

# Event Bus로 변경됨 - 기존 신호 제거
# signal training_completed(result: Dictionary)
# signal week_completed(results: Array)
# signal match_completed(result: Dictionary)
# signal state_changed()

var core = null  # Will be set to Rust GDExtension if available
var is_ready := false
var use_enhanced_system := true  # Toggle between legacy and enhanced

# Enhanced Core Mapper instance
var enhanced_mapper = EnhancedCoreMapper.new()

# Current game state
var current_state := {}


func _ready():
	# Try to load Rust core if available
	if FileAccess.file_exists("res://target/release/libfootball_game.so"):
		core = load("res://target/release/libfootball_game.so")
		if core:
			print("[Bridge] Rust core loaded successfully")
			is_ready = true
	else:
		print("[Bridge] Rust core not found, using mock data")
		_setup_mock_core()


func get_player_data() -> Dictionary:
	"""Get current player data in enhanced format"""
	if use_enhanced_system:
		return enhanced_mapper.get_enhanced_player_data(current_state)
	else:
		return current_state.get("player", {})


func set_player_data(data: Dictionary) -> void:
	"""Set player data (handles both legacy and enhanced)"""
	if use_enhanced_system and data.has("skills"):
		current_state["player"] = enhanced_mapper.player_to_json(data)
	else:
		current_state["player"] = data
	# Event Bus로 상태 변경 알림
	EventBus.emit("bridge_state_changed", current_state)
	print("[Bridge] 상태 변경됨 - Event Bus로 알림")


func train_async(training_type: String, intensity: float = 1.0) -> void:
	"""Execute training asynchronously"""
	print("[Bridge] Training async: %s at %fx intensity" % [training_type, intensity])

	# Simulate training in a thread
	var task_id = WorkerThreadPool.add_task(
		func():
			if core and is_ready:
				return _call_rust_training(training_type, intensity)
			else:
				return _mock_training(training_type, intensity)
	)

	var result = await WorkerThreadPool.wait_for_task_completion(task_id)

	# Apply result to state
	if result is Dictionary and result.has("ok") and result["ok"]:
		_apply_training_result(result["data"])
		# Event Bus로 훈련 완료 알림
		EventBus.emit("bridge_training_completed", result["data"])
		print("[Bridge] 훈련 완료됨 - Event Bus로 알림: ", result["data"])
	else:
		push_error(
			(
				"[Bridge] Training failed: "
				+ str(result.get("error", "Unknown error") if result is Dictionary else str(result))
			)
		)


func run_week_async(weekly_plans: Array) -> void:
	"""Run a full week of training"""
	print("[Bridge] Running week with %d training plans" % weekly_plans.size())

	var task_id = WorkerThreadPool.add_task(
		func():
			if core and is_ready:
				return _call_rust_week(weekly_plans)
			else:
				return _mock_week(weekly_plans)
	)

	var results = await WorkerThreadPool.wait_for_task_completion(task_id)

	if results is Dictionary and results.has("ok") and results["ok"]:
		# Event Bus로 주간 완료 알림
		EventBus.emit("bridge_week_completed", results["data"])
		print("[Bridge] 주간 완료됨 - Event Bus로 알림")
	else:
		push_error("[Bridge] Week execution failed")


func simulate_match_async(opponent: Dictionary) -> void:
	"""Simulate a match asynchronously"""
	print("[Bridge] Simulating match against: " + opponent.get("name", "Unknown"))

	var task_id = WorkerThreadPool.add_task(
		func():
			if core and is_ready:
				return _call_rust_match(opponent)
			else:
				return _mock_match(opponent)
	)

	var result = await WorkerThreadPool.wait_for_task_completion(task_id)

	if result is Dictionary and result.has("ok") and result["ok"]:
		# Event Bus로 경기 완료 알림
		EventBus.emit("bridge_match_completed", result["data"])
		print("[Bridge] 경기 완료됨 - Event Bus로 알림: ", result["data"])
	else:
		push_error("[Bridge] Match simulation failed")


func save_game(slot: int = 1) -> Dictionary:
	"""Save current game state"""
	var save_data = {
		"save_version": "2.0",  # Enhanced version
		"timestamp": Time.get_unix_time_from_system(),
		"state": current_state,
		"enhanced": use_enhanced_system
	}

	var save_path = "user://save_slot_%d.json" % slot
	var file = FileAccess.open(save_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(save_data))
		file.close()
		print("[Bridge] Game saved to slot %d" % slot)
		return {"ok": true}
	else:
		return {"ok": false, "error": "Failed to save"}


func load_game(slot: int = 1) -> Dictionary:
	"""Load game state from save"""
	var save_path = "user://save_slot_%d.json" % slot

	if not FileAccess.file_exists(save_path):
		return {"ok": false, "error": "Save file not found"}

	var file = FileAccess.open(save_path, FileAccess.READ)
	var json_string = file.get_as_text()
	file.close()

	var json = JSON.new()
	var parse_result = json.parse(json_string)

	if parse_result != OK:
		return {"ok": false, "error": "Invalid save file"}

	var save_data = json.data

	# Check for migration needs
	var version = save_data.get("save_version", "1.0")
	if version == "1.0" or version == "1.2":
		print("[Bridge] Migrating save from version %s to 2.0" % version)
		save_data = enhanced_mapper.migrate_legacy_to_enhanced(save_data)

	current_state = save_data.get("state", {})
	use_enhanced_system = save_data.get("enhanced", true)

	# Event Bus로 상태 변경 알림
	EventBus.emit("bridge_state_changed", current_state)
	print("[Bridge] 상태 변경됨 - Event Bus로 알림")
	print("[Bridge] Game loaded from slot %d" % slot)

	return {"ok": true, "data": current_state}


func new_game(player_name: String, position: String) -> Dictionary:
	"""Start a new game with enhanced system"""
	print("[Bridge] Starting new game: %s (%s)" % [player_name, position])

	if use_enhanced_system:
		current_state = enhanced_mapper.create_new_enhanced_game(player_name, position)
	else:
		current_state = _create_legacy_game(player_name, position)

	# Event Bus로 상태 변경 알림
	EventBus.emit("bridge_state_changed", current_state)
	print("[Bridge] 상태 변경됨 - Event Bus로 알림")
	return {"ok": true}


# Private helper functions


func _setup_mock_core():
	"""Setup mock data for testing without Rust core"""
	is_ready = true
	current_state = enhanced_mapper.create_new_enhanced_game("Test Player", "ST")


func _call_rust_training(training_type: String, intensity: float) -> Dictionary:
	"""Call Rust core for training (placeholder)"""
	# This would call the actual Rust function
	return _mock_training(training_type, intensity)


func _call_rust_week(weekly_plans: Array) -> Dictionary:
	"""Call Rust core for week simulation (placeholder)"""
	return _mock_week(weekly_plans)


func _call_rust_match(opponent: Dictionary) -> Dictionary:
	"""Call Rust core for match simulation (placeholder)"""
	return _mock_match(opponent)


func _mock_training(training_type: String, intensity: float) -> Dictionary:
	"""Mock training result for testing"""
	var result = {
		"ok": true,
		"data":
		{
			"training_type": training_type,
			"intensity": intensity,
			"deltas": {},
			"fatigue_delta": 8.0 * intensity,
			"message": "Training completed: " + training_type
		}
	}

	# Add some skill improvements based on training type
	match training_type:
		"Physical":
			result.data.deltas["pace"] = 0.3 * intensity
			result.data.deltas["stamina"] = 0.3 * intensity
		"Technical":
			result.data.deltas["dribbling"] = 0.3 * intensity
			result.data.deltas["passing"] = 0.3 * intensity
		"Shooting":
			result.data.deltas["finishing"] = 0.4 * intensity
			result.data.deltas["long_shots"] = 0.2 * intensity
		"BallControl":
			result.data.deltas["first_touch"] = 0.4 * intensity
			result.data.deltas["technique"] = 0.3 * intensity
		_:
			result.data.deltas["composure"] = 0.2 * intensity

	return result


func _mock_week(weekly_plans: Array) -> Dictionary:
	"""Mock week simulation for testing"""
	var results = []

	for plan in weekly_plans:
		var day_result = _mock_training(plan.get("type", "Rest"), plan.get("intensity", 1.0))
		results.append(day_result.data)

	return {"ok": true, "data": results}


func _mock_match(opponent: Dictionary) -> Dictionary:
	"""Mock match result for testing"""
	return {
		"ok": true,
		"data":
		{
			"home_score": randi() % 4,
			"away_score": randi() % 4,
			"events": ["Kickoff", "Goal!", "Half Time", "Full Time"],
			"fatigue_delta": 15.0,
			"experience_gained": 100
		}
	}


func _apply_training_result(result: Dictionary) -> void:
	"""Apply training results to current state"""
	if not current_state.has("player"):
		return

	var player = current_state["player"]

	# Apply skill changes
	if result.has("deltas"):
		if use_enhanced_system:
			enhanced_mapper.apply_skill_deltas(player, result.deltas)
		else:
			for skill in result.deltas:
				if player.has(skill):
					player[skill] = clampf(player[skill] + result.deltas[skill], 0.0, 100.0)

	# Apply fatigue
	if result.has("fatigue_delta"):
		player["fatigue"] = clampf(player.get("fatigue", 0.0) + result.fatigue_delta, 0.0, 100.0)


func _create_legacy_game(player_name: String, position: String) -> Dictionary:
	"""Create a legacy 12-skill game state"""
	return {
		"player":
		{
			"name": player_name,
			"position": position,
			"first_touch": 50.0,
			"dribbling": 50.0,
			"passing": 50.0,
			"pace": 50.0,
			"acceleration": 50.0,
			"stamina": 50.0,
			"finishing": 50.0,
			"vision": 50.0,
			"positioning": 50.0,
			"decision": 50.0,
			"composure": 50.0,
			"tackling": 50.0,
			"fatigue": 30.0,
			"condition": 4,
			"injury_days": 0
		},
		"week": 1,
		"year": 1
	}
