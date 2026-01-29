extends Control
class_name MatchScreen

# UI Nodes - Setup
@onready var setup_panel: Control = $SetupPanel
@onready var formation_opt: OptionButton = $SetupPanel/VBox/Content/Left/Formation/OptionButton
@onready var team_instruction_sliders: VBoxContainer = $SetupPanel/VBox/Content/Right/Sliders
@onready var start_btn: Button = $SetupPanel/VBox/ActionArea/StartBtn

# UI Nodes - View
@onready var view_panel: Control = $ViewPanel
@onready var score_lbl: Label = $ViewPanel/Header/ScoreLabel
@onready var time_lbl: Label = $ViewPanel/Header/TimeLabel
@onready var game_view_rect: ColorRect = $ViewPanel/GameViewArea/PlaceholderRect
@onready var exit_btn: Button = $ViewPanel/Footer/ExitBtn

# Data
var _tactics_bridge: TacticsBridge
var _selected_formation: String = "4-4-2"
var _is_match_running: bool = false

func _ready() -> void:
	_init_bridges()
	_setup_ui()
	_connect_signals()
	
	setup_panel.visible = true
	view_panel.visible = false

func _init_bridges() -> void:
	# TacticsBridge instantiation (assuming RustSimulator is available via a global or we mock it)
	_tactics_bridge = TacticsBridge.new()
	# In a real scenario: _tactics_bridge.initialize(FootballRustEngine)
	# For this UI impl, we might need to mock if engine isn't ready, but let's assume autocompletion works.

func _setup_ui() -> void:
	# Populate Formations
	formation_opt.clear()
	# Hardcoded list from FormationManager.gd inspection (mocking the 'get_all_formations' for UI speed)
	var formations = ["4-4-2", "4-3-3", "4-2-3-1", "3-5-2", "5-3-2", "4-1-4-1"]
	
	for i in range(formations.size()):
		formation_opt.add_item(formations[i])
		if formations[i] == _selected_formation:
			formation_opt.selected = i

func _connect_signals() -> void:
	if start_btn: start_btn.pressed.connect(_on_start_match_pressed)
	if exit_btn: exit_btn.pressed.connect(_on_exit_match_pressed)
	if formation_opt: formation_opt.item_selected.connect(_on_formation_selected)
	
	# Connect to UnifiedFramePipeline for live updates
	if UnifiedFramePipeline:
		UnifiedFramePipeline.snapshot_ready.connect(_on_snapshot_ready)

func _on_formation_selected(index: int) -> void:
	_selected_formation = formation_opt.get_item_text(index)
	print("Formation selected: ", _selected_formation)
	# In real app: FormationManager.set_formation(_selected_formation)

func _on_start_match_pressed() -> void:
	setup_panel.visible = false
	view_panel.visible = true
	_is_match_running = true
	
	print("Starting Match with Formation: ", _selected_formation)
	
	# Mapping UI Sliders (0-100) to Rust Instructions (0-20)
	# Formula: round(value / 5)
	# This will be passed to TacticsBridge in the future
	# var tempo_val = round(tempo_slider.value / 5.0)
	# var width_val = round(width_slider.value / 5.0)
	
	# Start the pipeline
	if UnifiedFramePipeline:
		UnifiedFramePipeline.load_match_data({}) # Reset/Load
		UnifiedFramePipeline.set_replay_mode(false) # Live mode
		UnifiedFramePipeline.start()

func _on_exit_match_pressed() -> void:
	# Stop match
	if UnifiedFramePipeline:
		UnifiedFramePipeline.stop()
	
	_is_match_running = false
	setup_panel.visible = true
	view_panel.visible = false

func _on_snapshot_ready(t_ms: int, snapshot: Dictionary) -> void:
	if not _is_match_running or not visible: return
	
	# Update Time
	var seconds = t_ms / 1000
	var mins = seconds / 60
	var secs = seconds % 60
	time_lbl.text = "%02d:%02d" % [mins, secs]
	
	# Update Score (Mocking structure if snapshot doesn't have it yet)
	var home = snapshot.get("home_score", 0)
	var away = snapshot.get("away_score", 0)
	score_lbl.text = "%d - %d" % [home, away]
	
	# In a real 3D view, we would pass 'snapshot' to the renderer here.
