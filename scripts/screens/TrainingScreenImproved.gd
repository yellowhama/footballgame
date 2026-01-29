extends Control

# TrainingScreenImproved.gd
# Refactored to use TrainingCard component and Kairosoft-style UI

const TrainingCardScene = preload("res://scenes/components/TrainingCard.tscn")

@onready var tab_container = $MainVBox/TabContainer
@onready var back_button = $MainVBox/Header/HBox/BackButton
@onready var fatigue_bar = $MainVBox/Header/HBox/FatigueInfo/FatigueBar
@onready var cancel_button = $MainVBox/BottomBar/HBox/CancelButton
@onready var confirm_button = $MainVBox/BottomBar/HBox/ConfirmButton

# Data
var training_data = {}
var selected_training_data = null
var selected_card_instance = null

# OpenFootball Targets (Static Data for now, could be moved to a JSON/Resource)
var openfoot_targets = [
	{
		"target": "technical",
		"category": "Technical",
		"name": "Technical Drills",
		"icon": "⚡",
		"description": "Improves Technique, Control, Dribbling",
		"stamina_cost": 15
	},
	{
		"target": "shooting",
		"category": "Technical",
		"name": "Shooting Practice",
		"icon": "⚽",
		"description": "Improves Finishing, Long Shots",
		"stamina_cost": 15
	},
	{
		"target": "passing",
		"category": "Technical",
		"name": "Passing Drills",
		"icon": "🎯",
		"description": "Improves Passing, Vision, Crossing",
		"stamina_cost": 12
	},
	{
		"target": "pace",
		"category": "Physical",
		"name": "Sprints",
		"icon": "🏃",
		"description": "Improves Pace, Acceleration, Agility",
		"stamina_cost": 18
	},
	{
		"target": "power",
		"category": "Physical",
		"name": "Gym Work",
		"icon": "💪",
		"description": "Improves Strength, Jumping",
		"stamina_cost": 18
	},
	{
		"target": "physical",
		"category": "Physical",
		"name": "Endurance Run",
		"icon": "❤️",
		"description": "Improves Stamina, Work Rate",
		"stamina_cost": 15
	},
	{
		"target": "mental",
		"category": "Mental",
		"name": "Tactical Study",
		"icon": "🧠",
		"description": "Improves Concentration, Decisions",
		"stamina_cost": 10
	},
	{
		"target": "defending",
		"category": "Defensive",
		"name": "Defensive Drills",
		"icon": "🛡️",
		"description": "Improves Tackling, Marking",
		"stamina_cost": 15
	},
	{
		"target": "rest",
		"category": "Special",
		"name": "Rest",
		"icon": "😴",
		"description": "Recover Stamina and Condition",
		"stamina_cost": -15
	},
	{
		"target": "go_out",
		"category": "Special",
		"name": "Go Out",
		"icon": "🎮",
		"description": "Relax and improve Morale",
		"stamina_cost": -5
	}
]


func _ready():
	print("[TrainingScreen] Initializing...")
	_connect_signals()
	_build_training_data()
	_populate_ui()
	_update_fatigue_display()


func _connect_signals():
	if back_button:
		back_button.pressed.connect(_on_back_pressed)
	if cancel_button:
		cancel_button.pressed.connect(_on_cancel_pressed)
	if confirm_button:
		confirm_button.pressed.connect(_on_confirm_pressed)

	# Listen for global training finished signal if manager exists
	if has_node("/root/TrainingManager"):
		var tm = get_node("/root/TrainingManager")
		if not tm.training_finished.is_connected(_on_training_finished):
			tm.training_finished.connect(_on_training_finished)


func _build_training_data():
	# 1. OpenFootball Targets
	for target in openfoot_targets:
		var cat = target.category
		if not training_data.has(cat):
			training_data[cat] = []

		var item = target.duplicate()
		item["is_personal"] = false
		training_data[cat].append(item)

	# 2. Personal Training (from TrainingManager)
	if has_node("/root/TrainingManager"):
		var tm = get_node("/root/TrainingManager")
		var personal_programs = _get_personal_programs_list(tm)

		training_data["Personal"] = []
		for prog in personal_programs:
			prog["is_personal"] = true
			prog["category"] = "Personal"
			training_data["Personal"].append(prog)


func _get_personal_programs_list(tm) -> Array:
	# This mimics the logic from the old script to fetch programs by ID
	var list = []
	var ids = [
		"shooting_precision",
		"passing_accuracy",
		"dribbling_control",
		"stamina_boost",
		"pace_training",
		"strength_conditioning",
		"tackling_drills",
		"marking_practice",
		"positioning_work"
	]

	for id in ids:
		var data = tm.get_training_by_id(id)
		if not data.is_empty():
			data["target"] = id  # Ensure target ID is set
			list.append(data)

	return list


func _populate_ui():
	# Clear existing children first if needed (though usually empty on load)
	# Iterate through known tabs
	var tabs = ["Technical", "Physical", "Mental", "Defensive", "Special", "Personal"]

	for tab_name in tabs:
		if not training_data.has(tab_name):
			continue

		var grid_path = "MainVBox/TabContainer/%s/GridContainer" % tab_name
		if not has_node(grid_path):
			print("Grid not found for: " + tab_name)
			continue

		var grid = get_node(grid_path)
		for item in training_data[tab_name]:
			var card = TrainingCardScene.instantiate()
			grid.add_child(card)
			card.setup(item)
			card.selected.connect(_on_card_selected.bind(card))


func _on_card_selected(data: Dictionary, card_instance: Control):
	# Deselect previous
	if selected_card_instance and is_instance_valid(selected_card_instance):
		selected_card_instance.modulate = Color(1, 1, 1, 1)  # Reset visual

	selected_training_data = data
	selected_card_instance = card_instance

	# Visual feedback for selection
	card_instance.modulate = Color(1.2, 1.2, 1.2, 1)  # Brighten

	confirm_button.disabled = false
	print("Selected: ", data.get("name"))


func _on_confirm_pressed():
	if not selected_training_data:
		return

	print("Confirming training: ", selected_training_data.get("name"))
	confirm_button.disabled = true
	cancel_button.disabled = true

	var tm = get_node_or_null("/root/TrainingManager")
	if tm:
		var target_id = selected_training_data.get("target")
		var is_personal = selected_training_data.get("is_personal", false)
		tm.execute_training(target_id, is_personal)
	else:
		print("TrainingManager not found!")
		_return_to_home()


func _on_training_finished(result: Dictionary):
	print("Training finished: ", result)

	# Save result to GameManager for the ResultScreen to display
	var game_manager = get_node_or_null("/root/GameManager")
	if game_manager:
		if game_manager.has_method("set_last_result"):
			game_manager.set_last_result(result)
		else:
			game_manager.last_result = result

	# Transition to ResultScreen
	get_tree().change_scene_to_file("res://scenes/ResultScreenImproved.tscn")


func _on_back_pressed():
	_return_to_home()


func _on_cancel_pressed():
	_return_to_home()


func _return_to_home():
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _update_fatigue_display():
	if not fatigue_bar:
		return

	## 컨디션 시스템에서 현재 컨디션 퍼센트 가져오기
	var condition_pct: float = 100.0
	if has_node("/root/ConditionSystem"):
		var condition_sys = get_node("/root/ConditionSystem")
		if condition_sys.has_method("get_condition_percentage"):
			condition_pct = condition_sys.get_condition_percentage()

	fatigue_bar.value = condition_pct

	## 컨디션 바 색상 (녹색 → 노랑 → 빨강)
	if condition_pct >= 70:
		fatigue_bar.modulate = Color(0.3, 0.9, 0.3)  ## 녹색
	elif condition_pct >= 40:
		fatigue_bar.modulate = Color(0.9, 0.8, 0.2)  ## 노랑
	else:
		fatigue_bar.modulate = Color(0.9, 0.3, 0.3)  ## 빨강

	## 최소 컨디션 미달 시 경고 표시
	var tm = get_node_or_null("/root/TrainingManager")
	if tm and condition_pct < tm.MIN_CONDITION_TO_TRAIN:
		if confirm_button:
			confirm_button.disabled = true
			confirm_button.tooltip_text = "컨디션이 너무 낮습니다 (%.1f%% < %.1f%%)" % [condition_pct, tm.MIN_CONDITION_TO_TRAIN]
