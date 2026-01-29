extends CanvasLayer
class_name GameHUD

## Simple global HUD overlay
## - Shows current week / stamina at the top
## - Provides bottom navigation buttons (hook up navigation later)
## - Listens to UIService.hud_update for future extensions

@onready var week_label: Label = $TopBar/WeekLabel
@onready var condition_bar: ProgressBar = $TopBar/ConditionBar
@onready var condition_label: Label = $TopBar/ConditionLabel

@onready var home_button: Button = $BottomBar/HomeButton
@onready var training_button: Button = $BottomBar/TrainingButton
@onready var match_button: Button = $BottomBar/MatchButton
@onready var tactics_button: Button = $BottomBar/TacticsButton
@onready var menu_button: Button = $BottomBar/MenuButton


func _ready() -> void:
	_sync_date_from_manager()
	_sync_condition_from_manager()
	_connect_buttons()

	# Listen for HUD updates from UIService (e.g., training_last_result)
	if Engine.is_editor_hint():
		return

	if UIService and not UIService.hud_update.is_connected(_on_hud_update):
		UIService.hud_update.connect(_on_hud_update)

	# Listen for stamina changes to keep the bar in sync
	if DateManager and not DateManager.stamina_changed.is_connected(_on_stamina_changed):
		DateManager.stamina_changed.connect(_on_stamina_changed)

	# Default: start hidden so it does not interfere with heavy UI flows
	# (character creation, complex screens). Screens that want a global
	# HUD should explicitly call HUD.show_hud().
	visible = false


func _sync_date_from_manager() -> void:
	if not week_label:
		return
	if DateManager:
		var year: int = DateManager.current_year
		var week: int = DateManager.current_week
		week_label.text = "Year %d · Week %d" % [year, week]


func _sync_condition_from_manager() -> void:
	if not condition_bar or not condition_label:
		return
	if DateManager:
		var stamina: int = DateManager.stamina
		condition_bar.value = float(stamina)
		condition_label.text = "%d%%" % stamina


func _on_stamina_changed(_old_value: int, new_value: int, _reason: String) -> void:
	condition_bar.value = float(new_value)
	condition_label.text = "%d%%" % new_value


func _on_hud_update(element: String, data: Dictionary) -> void:
	# For now we only log; specific elements can be visualised later.
	if element == "training_last_result":
		# In the future we might surface a compact summary here.
		print("[GameHUD] training_last_result updated: %s" % str(data))


func _connect_buttons() -> void:
	if home_button:
		home_button.pressed.connect(
			func() -> void:
				print("[GameHUD] Home button pressed")
				_navigate_to("res://scenes/HomeImproved.tscn")
		)
	if training_button:
		training_button.pressed.connect(
			func() -> void:
				print("[GameHUD] Training button pressed")
				# Use the improved responsive training screen when available.
				_navigate_to("res://scenes/TrainingScreenImproved_Responsive.tscn")
		)
	if match_button:
		match_button.pressed.connect(
			func() -> void:
				print("[GameHUD] Match button pressed")
				# Route to MVP WeekHub as the central match entrypoint.
				_navigate_to("res://scenes/mvp/WeekHub.tscn")
		)
	if tactics_button:
		tactics_button.pressed.connect(
			func() -> void:
				print("[GameHUD] Tactics button pressed")
				_navigate_to("res://scenes/screens/TacticsScreen.tscn")
		)
	if menu_button:
		menu_button.pressed.connect(
			func() -> void:
				print("[GameHUD] Menu button pressed")
				_navigate_to("res://scenes/TitleScreenImproved.tscn")
		)


func _navigate_to(scene_path: String) -> void:
	if scene_path == "":
		return

	# Prefer SceneLoader if available (handles loading screens, etc.)
	if has_node("/root/SceneLoader"):
		var loader := get_node("/root/SceneLoader")
		if loader and loader.has_method("load_scene"):
			loader.call("load_scene", scene_path)
			return

	if get_tree().has_method("change_scene_to_file"):
		get_tree().change_scene_to_file(scene_path)


func show_hud() -> void:
	visible = true


func hide_hud() -> void:
	visible = false
