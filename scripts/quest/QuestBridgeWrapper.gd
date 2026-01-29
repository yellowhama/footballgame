extends RefCounted
## GDScript wrapper for QuestBridge GDExtension

var _bridge: RefCounted = null


func _init() -> void:
	# Create the Rust QuestBridge instance
	_bridge = ClassDB.instantiate("QuestBridge")
	if _bridge:
		print("[QuestBridgeWrapper] QuestBridge instantiated")
	else:
		push_warning("[QuestBridgeWrapper] QuestBridge not available - using mock")


func quest_init(config_json: String) -> String:
	if _bridge and _bridge.has_method("quest_init"):
		return _bridge.quest_init(config_json)
	return _mock_init_response()


func get_all_quests() -> String:
	if _bridge and _bridge.has_method("get_all_quests"):
		return _bridge.get_all_quests()
	return _mock_quests_response()


func get_quests_by_status(status: String) -> String:
	if _bridge and _bridge.has_method("get_quests_by_status"):
		return _bridge.get_quests_by_status(status)
	return JSON.stringify({"success": true, "quests": []})


func get_active_quests() -> String:
	if _bridge and _bridge.has_method("get_active_quests"):
		return _bridge.get_active_quests()
	return JSON.stringify({"success": true, "quests": []})


func get_quest(quest_id: String) -> String:
	if _bridge and _bridge.has_method("get_quest"):
		return _bridge.get_quest(quest_id)
	return JSON.stringify({"success": false, "error": "Mock mode"})


func activate_quest(quest_id: String) -> String:
	if _bridge and _bridge.has_method("activate_quest"):
		return _bridge.activate_quest(quest_id)
	return JSON.stringify({"success": false, "error": "Mock mode"})


func update_objective(update_json: String) -> String:
	if _bridge and _bridge.has_method("update_objective"):
		return _bridge.update_objective(update_json)
	return JSON.stringify({"success": false, "error": "Mock mode"})


func update_all_by_type(objective_type: String, value: int) -> String:
	if _bridge and _bridge.has_method("update_all_by_type"):
		return _bridge.update_all_by_type(objective_type, value)
	return JSON.stringify({"success": true, "completed_quests": []})


func auto_unlock(player_level: int, squad_level: String) -> String:
	if _bridge and _bridge.has_method("auto_unlock"):
		return _bridge.auto_unlock(player_level, squad_level)
	return JSON.stringify({"success": true, "unlocked_quests": []})


func check_time_limits(current_time: int) -> String:
	if _bridge and _bridge.has_method("check_time_limits"):
		return _bridge.check_time_limits(current_time)
	return JSON.stringify({"success": true, "failed_quests": []})


func get_statistics() -> String:
	if _bridge and _bridge.has_method("get_statistics"):
		return _bridge.get_statistics()
	return _mock_statistics_response()


func save_quest_state() -> String:
	if _bridge and _bridge.has_method("save_quest_state"):
		return _bridge.save_quest_state()
	return JSON.stringify({"success": false, "error": "Mock mode"})


func load_quest_state(save_data: String, current_time: int) -> String:
	if _bridge and _bridge.has_method("load_quest_state"):
		return _bridge.load_quest_state(save_data, current_time)
	return JSON.stringify({"success": false, "error": "Mock mode"})


func add_quest(quest_json: String) -> String:
	if _bridge and _bridge.has_method("add_quest"):
		return _bridge.add_quest(quest_json)
	return JSON.stringify({"success": false, "error": "Mock mode"})


# Mock responses for testing without Rust backend


func _mock_init_response() -> String:
	return JSON.stringify({"success": true, "message": "Quest System initialized (mock)"})


func _mock_quests_response() -> String:
	return JSON.stringify(
		{
			"success": true,
			"quests":
			[
				{
					"id": "main_first_steps",
					"title": "First Steps",
					"description": "Begin your journey as a professional footballer.",
					"quest_type": "Main",
					"status": "Active",
					"objectives":
					[
						{
							"description": "Complete your first training session",
							"target_value": 1,
							"current_value": 0,
							"objective_type": "Train",
							"is_complete": false,
							"progress_percentage": 0.0
						}
					],
					"rewards": {"xp": 100, "items": []},
					"is_complete": false,
					"progress_percentage": 0.0
				},
				{
					"id": "main_rising_star",
					"title": "Rising Star",
					"description": "Prove yourself on the pitch.",
					"quest_type": "Main",
					"status": "Locked",
					"objectives":
					[
						{
							"description": "Win 3 matches",
							"target_value": 3,
							"current_value": 0,
							"objective_type": "Win",
							"is_complete": false,
							"progress_percentage": 0.0
						}
					],
					"rewards": {"xp": 250, "items": []},
					"is_complete": false,
					"progress_percentage": 0.0
				},
				{
					"id": "side_training_dedication",
					"title": "Training Dedication",
					"description": "Show your commitment to improvement.",
					"quest_type": "Side",
					"status": "Active",
					"objectives":
					[
						{
							"description": "Complete 5 training sessions",
							"target_value": 5,
							"current_value": 0,
							"objective_type": "Train",
							"is_complete": false,
							"progress_percentage": 0.0
						}
					],
					"rewards": {"xp": 150, "items": []},
					"is_complete": false,
					"progress_percentage": 0.0
				},
				{
					"id": "daily_training",
					"title": "Daily Training",
					"description": "Complete today's training.",
					"quest_type": "Daily",
					"status": "Active",
					"objectives":
					[
						{
							"description": "Complete 1 training session",
							"target_value": 1,
							"current_value": 0,
							"objective_type": "Train",
							"is_complete": false,
							"progress_percentage": 0.0
						}
					],
					"rewards": {"xp": 25, "items": []},
					"is_complete": false,
					"progress_percentage": 0.0
				}
			]
		}
	)


func _mock_statistics_response() -> String:
	return JSON.stringify(
		{"success": true, "statistics": {"total": 4, "completed": 0, "active": 3, "failed": 0, "locked": 1}}
	)
