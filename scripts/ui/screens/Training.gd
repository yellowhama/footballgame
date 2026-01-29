extends Control
class_name TrainingScreen

@onready var grid_container: GridContainer = $VBox/Content/QuickView/VBox/TargetList
@onready var intensity_slider: HSlider = $VBox/Content/QuickView/VBox/ActionArea/IntensitySlider
@onready var execute_btn: Button = $VBox/Content/QuickView/VBox/ActionArea/ExecuteBtn
@onready var intensity_label: Label = $VBox/Content/QuickView/VBox/ActionArea/IntensityLabel

const TrainingCardScene = preload("res://scenes/ui/components/TrainingCard.tscn")

# Mock Training Targets (Canonical set from Rust types)
# PACE, POWER, TECHNICAL, SHOOTING, PASSING, DEFENDING, MENTAL, ENDURANCE
const TARGETS = [
	{"id": "pace", "name": "Pace", "icon": "🏃", "desc": "Speed & Acceleration"},
	{"id": "power", "name": "Power", "icon": "💪", "desc": "Strength & Jumping"},
	{"id": "technical", "name": "Technical", "icon": "⚡", "desc": "Technique & Dribbling"},
	{"id": "shooting", "name": "Shooting", "icon": "⚽", "desc": "Finishing & Long Shots"},
	{"id": "passing", "name": "Passing", "icon": "🎯", "desc": "Passing & Vision"},
	{"id": "defending", "name": "Defending", "icon": "🛡️", "desc": "Tackling & Marking"},
	{"id": "mental", "name": "Mental", "icon": "🧠", "desc": "Concentration & Composure"},
	{"id": "endurance", "name": "Endurance", "icon": "❤️", "desc": "Stamina & Work Rate"},
]

var _selected_target_id: String = ""

func _ready() -> void:
	_populate_cards()
	_update_intensity_label()
	
	if intensity_slider:
		intensity_slider.value_changed.connect(func(v): _update_intensity_label())
	if execute_btn:
		execute_btn.pressed.connect(_on_execute_pressed)

func _populate_cards() -> void:
	for child in grid_container.get_children():
		child.queue_free()
		
	for t in TARGETS:
		var card = TrainingCardScene.instantiate()
		grid_container.add_child(card)
		card.setup(t)
		card.selected.connect(_on_card_selected)

func _on_card_selected(card_data: Dictionary) -> void:
	_selected_target_id = card_data.get("id", "")
	# Highlight selected card visually (simple outline or color change)
	for child in grid_container.get_children():
		if child is TrainingCard:
			child.set_selected(child.get_data().get("id") == _selected_target_id)
	
	print("Selected training: ", _selected_target_id)

func _update_intensity_label() -> void:
	if not intensity_label or not intensity_slider: return
	
	var val = intensity_slider.value
	var text = "Normal"
	match int(val):
		0: text = "Very Light" # Rehab
		1: text = "Light"
		2: text = "Normal"
		3: text = "High"
		4: text = "Very High" # Overload
	
	intensity_label.text = "Intensity: " + text

func _on_execute_pressed() -> void:
	if _selected_target_id == "":
		_show_toast("Please select a training target first.", "warning")
		return
		
	var intensity_str = _get_intensity_string(intensity_slider.value)
	
	# Request Payload
	var request = {
		"training_type": "individual", # Defaulting to individual for this screen context
		"target": _selected_target_id,
		"intensity": intensity_str
	}
	
	# Mock Player/Manager data (In real implementation, get from Roster/GameState)
	var player_mock = {"id": "player_current", "name": "Current Player", "ca": 80, "condition": 0.9}
	var manager_mock = {"stamina_system": {"current": 90}}
	
	var bridge = TrainingBridge.new()
	# bridge.initialize(RustSimulator) # We don't have the simulator ref here easily in this disconnected context
	# So we might mock the response if bridge isn't ready, OR if we had a Autoload wrapper.
	
	# For prototype/UI verification, we simulate success
	_mock_success_response(request)

func _get_intensity_string(val: float) -> String:
	match int(val):
		0: return "Rest"
		1: return "Light"
		2: return "Normal"
		3: return "High"
		4: return "Intense"
	return "Normal"

func _mock_success_response(req: Dictionary) -> void:
	print("Executing Training: ", req)
	
	# Simulate processing delay
	await get_tree().create_timer(0.5).timeout
	
	_show_toast("Training Completed! " + req["target"] + " improved.", "success")
	# In real app, we would show a Result Popup (TrainingResult)

func _show_toast(msg: String, type: String = "info") -> void:
	# connect to GameUI toast system if available, or just print
	print("[TOAST] ", type.to_upper(), ": ", msg)
	# If we have a Toast Autoload or Signal
	# UIManager.show_toast(msg, type)
