extends Control
class_name GameUI

@onready var screen_container: Control = $ScreenLayer/ScreenContainer
@onready var bottom_nav: BottomNav = $HUDLayer/BottomNav
@onready var top_bar: TopBar = $HUDLayer/TopBar

func _ready() -> void:
	_connect_signals()
	
	# Initial screen
	if UIManager and UIManager.current_screen == "":
		UIManager.change_screen("Home")


func _connect_signals() -> void:
	if bottom_nav:
		bottom_nav.nav_requested.connect(_on_nav_requested)
	
	if UIManager:
		UIManager.screen_changed.connect(_on_screen_changed)

func _on_nav_requested(screen_id: String) -> void:
	if UIManager:
		UIManager.change_screen(screen_id)

func _on_screen_changed(screen_id: String) -> void:
	# Load/Switch screen instance in ScreenContainer
	_load_screen(screen_id)
	
	if bottom_nav:
		bottom_nav.set_active_tab(screen_id)

func _load_screen(screen_id: String) -> void:
	# Clear existing children? Or just hide them if we want to caching?
	# Spec implies "Navigation Stack" or specific slots.
	# For v1, let's just clear and instantiate for simplicity, or toggle visibility if we pre-instantiate.
	
	# For Phase 2, we just support the structure.
	# We need to map screen_id to scene paths.
	
	var scene_path = ""
	match screen_id:
		"Home": scene_path = "res://scenes/ui/screens/Home.tscn"
		"Training": scene_path = "res://scenes/ui/screens/Training.tscn"
		"Match": scene_path = "res://scenes/ui/screens/Match.tscn"
		"Status": scene_path = "res://scenes/ui/screens/Status.tscn"
		"Replay": scene_path = "res://scenes/ui/screens/Replay.tscn" 
		"Menu": scene_path = "res://scenes/ui/screens/Menu.tscn"
		_:
			push_warning("Unknown screen_id: " + screen_id)
			return
	
	# Check if we already have it instantiated?
	var existing = screen_container.get_node_or_null(screen_id)
	if existing:
		_show_screen_node(existing)
	else:
		if ResourceLoader.exists(scene_path):
			var scene = load(scene_path)
			var instance = scene.instantiate()
			instance.name = screen_id
			screen_container.add_child(instance)
			_show_screen_node(instance)
		else:
			push_warning("Scene path not found: " + scene_path)

func _show_screen_node(target_node: Node) -> void:
	for child in screen_container.get_children():
		child.visible = (child == target_node)
