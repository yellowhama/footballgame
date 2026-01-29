extends Node

# Quest Integration Manager
# Connects Event Bus quest triggers with QuestSystem addon

signal quest_progress_updated(quest_id: int, progress: Dictionary)

# Quest instances storage
var active_quests: Dictionary = {}
var quest_templates: Dictionary = {}

# QuestSystem reference (may be null if addon not installed)
var _quest_system: Node = null


func _ready():
	print("[QuestIntegrationManager] Initializing quest integration system")

	# Wait for other autoloads
	await get_tree().process_frame

	# Try to get QuestSystem from /root
	_quest_system = get_node_or_null("/root/QuestSystem")

	# Subscribe to Event Bus quest events
	EventBus.subscribe("quest_trigger", _on_quest_trigger)
	EventBus.subscribe("story_completed", _on_story_completed)
	EventBus.subscribe("quest_objective_completed", _on_quest_objective_completed)

	# Connect to QuestSystem signals if available
	if _quest_system:
		_quest_system.quest_accepted.connect(_on_quest_accepted)
		_quest_system.quest_completed.connect(_on_quest_completed)
		_quest_system.new_available_quest.connect(_on_new_available_quest)
		print("[QuestIntegrationManager] Connected to QuestSystem signals")
	else:
		print("[QuestIntegrationManager] QuestSystem not available (addon not installed)")

	# Initialize quest templates
	_initialize_quest_templates()


func _initialize_quest_templates():
	"""Initialize quest template registry"""
	print("[QuestIntegrationManager] Initializing quest templates...")

	# Register the FirstTrainingQuest
	quest_templates["complete_first_training"] = preload("res://scripts/quests/FirstTrainingQuest.gd")

	# TODO: Add more quest templates as they're created
	# quest_templates["weekly_training_routine"] = preload("res://scripts/quests/WeeklyTrainingQuest.gd")
	# quest_templates["first_match"] = preload("res://scripts/quests/FirstMatchQuest.gd")

	print("[QuestIntegrationManager] ✅ Quest templates registered: ", quest_templates.keys())


func _on_quest_trigger(data: Dictionary):
	"""Handle quest trigger events from Event Bus"""
	var quest_id = data.get("quest_id", "")
	var source = data.get("source", "unknown")
	var quest_data = data.get("data", {})

	print("[QuestIntegrationManager] Quest trigger received: ", quest_id, " from ", source)

	if quest_templates.has(quest_id):
		_create_and_start_quest(quest_id, quest_data)
	else:
		print("[QuestIntegrationManager] ❌ Unknown quest ID: ", quest_id)


func _create_and_start_quest(quest_id: String, quest_data: Dictionary):
	"""Create and start a new quest instance"""
	if active_quests.has(quest_id):
		print("[QuestIntegrationManager] ⚠️ Quest already active: ", quest_id)
		return

	# Create quest instance
	var quest_class = quest_templates[quest_id]
	var quest_instance = quest_class.new()

	# Store reference
	active_quests[quest_id] = quest_instance

	# Add to QuestSystem
	if _quest_system:
		_quest_system.available.add_quest(quest_instance)
		_quest_system.active.add_quest(quest_instance)

		# Start the quest
		quest_instance.start()

		print("[QuestIntegrationManager] ✅ Quest created and started: ", quest_id)

		# Emit Event Bus notification
		EventBus.emit(
			"quest_activated", {"quest_id": quest_id, "quest_name": quest_instance.quest_name, "quest_data": quest_data}
		)
	else:
		print("[QuestIntegrationManager] ❌ Cannot start quest - QuestSystem not available")


func _on_story_completed(data: Dictionary):
	"""Handle story completion events that trigger quests"""
	var timeline = data.get("timeline", "")
	var trigger_quest = data.get("trigger_quest", "")

	print("[QuestIntegrationManager] Story completed: ", timeline, " -> trigger: ", trigger_quest)

	if trigger_quest:
		# Trigger the associated quest
		_on_quest_trigger(
			{
				"quest_id": trigger_quest,
				"source": "story_completion",
				"data": {"timeline": timeline, "story_data": data}
			}
		)


func _on_quest_objective_completed(data: Dictionary):
	"""Handle quest objective completion events"""
	var quest_id = data.get("quest_id", 0)
	print("[QuestIntegrationManager] Quest objective completed: ", quest_id)

	quest_progress_updated.emit(quest_id, data)


func _on_quest_accepted(quest: Quest):
	"""Handle quest acceptance from QuestSystem"""
	print("[QuestIntegrationManager] Quest accepted: ", quest.quest_name)

	EventBus.emit("quest_accepted", {"quest_id": quest.id, "quest_name": quest.quest_name})


func _on_quest_completed(quest: Quest):
	"""Handle quest completion from QuestSystem"""
	print("[QuestIntegrationManager] Quest completed: ", quest.quest_name)

	# Remove from active quests
	for quest_key in active_quests:
		if active_quests[quest_key] == quest:
			active_quests.erase(quest_key)
			break

	EventBus.emit("quest_completed", {"quest_id": quest.id, "quest_name": quest.quest_name})


func _on_new_available_quest(quest: Quest):
	"""Handle new available quest from QuestSystem"""
	print("[QuestIntegrationManager] New quest available: ", quest.quest_name)

	EventBus.emit("new_quest_available", {"quest_id": quest.id, "quest_name": quest.quest_name})


# Public API


func get_active_quests() -> Array:
	"""Get list of currently active quests"""
	return active_quests.values()


func get_quest_by_id(quest_id: String) -> Quest:
	"""Get active quest by ID"""
	return active_quests.get(quest_id, null)


func is_quest_active(quest_id: String) -> bool:
	"""Check if a quest is currently active"""
	return active_quests.has(quest_id)


func get_quest_progress(quest_id: String) -> Dictionary:
	"""Get progress information for a quest"""
	var quest = get_quest_by_id(quest_id)
	if quest and quest.has_method("get_detailed_progress"):
		return quest.get_detailed_progress()
	return {}


func trigger_tutorial_sequence():
	"""Start the tutorial quest sequence"""
	print("[QuestIntegrationManager] Starting tutorial sequence...")

	# This would be called after the manager meeting
	_on_quest_trigger(
		{
			"quest_id": "complete_first_training",
			"source": "tutorial_start",
			"data": {"tutorial": true, "sequence": "introduction"}
		}
	)


# Debug/Test functions


func test_quest_system():
	"""Test the quest integration"""
	print("=== Quest Integration Test ===")

	# Test quest creation
	trigger_tutorial_sequence()

	# Wait a frame
	await get_tree().process_frame

	# Check active quests
	var active = get_active_quests()
	print("Active quests: ", active.size())

	for quest in active:
		print("- ", quest.quest_name, " (ID: ", quest.id, ")")

	print("=== Quest Integration Test Complete ===")


func debug_quest_status():
	"""Debug print current quest status"""
	print("\n=== Current Quest Status ===")
	print("Active quests: ", active_quests.size())

	for quest_id in active_quests:
		var quest = active_quests[quest_id]
		print("- %s: %s" % [quest_id, quest.quest_name])
		print("  Objective completed: ", quest.objective_completed)
		if quest.has_method("get_progress_text"):
			print("  Progress: ", quest.get_progress_text())

	print("=== End Quest Status ===\n")
