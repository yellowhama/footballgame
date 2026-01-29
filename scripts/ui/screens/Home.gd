extends Control
class_name HomeScreen

signal action_requested(action_id: String)

@onready var next_action_card: PanelContainer = $VBox/Content/NextActionCard
@onready var action_title: Label = $VBox/Content/NextActionCard/VBox/Title
@onready var action_desc: Label = $VBox/Content/NextActionCard/VBox/Description
@onready var action_btn: Button = $VBox/Content/NextActionCard/VBox/ActionBtn

# Mock State for prototype
var _next_match_opponent: String = "Arsenal"
var _days_to_match: int = 0 # 0 = Today

func _ready() -> void:
	_update_dashboard()
	action_btn.pressed.connect(_on_action_pressed)

func _update_dashboard() -> void:
	# Priority Logic (7+/-2 Rule)
	# 1. Match Day? -> Go to Match
	# 2. Training Needed? -> Go to Training
	# 3. Save Needed? -> Save
	
	if _days_to_match == 0:
		_setup_action_card("Match Day!", "vs " + _next_match_opponent, "Go to Match", "Match")
	elif _days_to_match < 0:
		_setup_action_card("Training Required", "Prepare for next match", "Training Plan", "Training")
	else:
		_setup_action_card("Weekly Hub", "Check team status", "View Squad", "Status")

func _setup_action_card(title: String, desc: String, btn_text: String, target_screen: String) -> void:
	action_title.text = title
	action_desc.text = desc
	action_btn.text = btn_text
	action_btn.set_meta("target", target_screen)

func _on_action_pressed() -> void:
	var target = action_btn.get_meta("target", "Home")
	if UIManager:
		UIManager.change_screen(target)
