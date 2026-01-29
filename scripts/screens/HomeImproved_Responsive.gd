extends "res://scenes/academy/base/AdaptiveLayoutContainer.gd"
## Responsive Academy Mode Home Screen
## Phase 7B Implementation - Cross-platform UI with 3 layout variants

# Mobile layout node references
@onready var mobile_week_label = $MobilePortraitLayout/Control/WeeklyBar/Content/WeekInfo/WeekLabel
@onready var mobile_week_current = $MobilePortraitLayout/Control/WeeklyBar/Content/WeekInfo/WeekProgress/Current
@onready
var mobile_player_name = $MobilePortraitLayout/Control/ScrollContainer/MainContent/TopSection/PlayerCard/VBox/PlayerHeader/PlayerInfo/Name
@onready
var mobile_condition_text = $MobilePortraitLayout/Control/ScrollContainer/MainContent/TopSection/PlayerCard/VBox/ConditionSection/ConditionBar/ConditionInfo/ConditionText
@onready
var mobile_ovr_value = $MobilePortraitLayout/Control/ScrollContainer/MainContent/TopSection/PlayerCard/VBox/StatsOverview/OVR/Value
@onready var mobile_advance_button = $MobilePortraitLayout/Control/BottomBar/ButtonContainer/AdvanceButton

# Tablet layout node references
@onready var tablet_week_label = $TabletHybridLayout/Control/WeeklyBar/Content/WeekInfo/WeekLabel
@onready var tablet_week_current = $TabletHybridLayout/Control/WeeklyBar/Content/WeekInfo/WeekProgress/Current
@onready
var tablet_player_name = $TabletHybridLayout/Control/MainContainer/LeftColumn/PlayerCard/VBox/PlayerHeader/PlayerInfo/Name
@onready
var tablet_condition_text = $TabletHybridLayout/Control/MainContainer/LeftColumn/PlayerCard/VBox/StatsGrid/ConditionSection/ConditionBar/ConditionInfo/ConditionText
@onready
var tablet_ovr_value = $TabletHybridLayout/Control/MainContainer/LeftColumn/PlayerCard/VBox/StatsGrid/OVRSection/OVR/Value
@onready var tablet_advance_button = $TabletHybridLayout/Control/BottomBar/ButtonContainer/AdvanceButton

# Desktop layout node references
@onready var desktop_week_label = $DesktopLandscapeLayout/Control/WeeklyBar/Content/WeekInfo/WeekLabel
@onready var desktop_week_current = $DesktopLandscapeLayout/Control/WeeklyBar/Content/WeekInfo/WeekProgress/Current
@onready
var desktop_player_name = $DesktopLandscapeLayout/Control/MainContainer/LeftColumn/PlayerCard/VBox/PlayerHeader/PlayerInfo/Name
@onready
var desktop_condition_text = $DesktopLandscapeLayout/Control/MainContainer/LeftColumn/PlayerCard/VBox/StatsGrid/ConditionSection/ConditionBar/ConditionInfo/ConditionText
@onready
var desktop_ovr_value = $DesktopLandscapeLayout/Control/MainContainer/LeftColumn/PlayerCard/VBox/StatsGrid/OVRSection/OVR/Value
@onready var desktop_advance_button = $DesktopLandscapeLayout/Control/BottomBar/ButtonContainer/AdvanceButton


func _ready():
	super._ready()  # Call AdaptiveLayoutContainer._ready()

	print("[HomeImproved] Responsive scene initialized")

	# Connect layout activation signals
	layout_activated.connect(_on_layout_activated)

	# Connect button signals for all layouts
	if mobile_advance_button:
		mobile_advance_button.pressed.connect(_on_advance_pressed)
	if tablet_advance_button:
		tablet_advance_button.pressed.connect(_on_advance_pressed)
	if desktop_advance_button:
		desktop_advance_button.pressed.connect(_on_advance_pressed)

	# Wait for platform detection
	await get_tree().process_frame

	# Initial data population
	_populate_current_layout()

	if DateManager and DateManager.mvp_mode_enabled:
		call_deferred("_go_to_weekhub")

	# Validate UI standards
	_validate_ui_standards()


func _go_to_weekhub() -> void:
	var target_scene := "res://scenes/mvp/WeekHub.tscn"
	if is_inside_tree() and get_tree() != null:
		var result := get_tree().change_scene_to_file(target_scene)
		if result != OK:
			print("[HomeImprovedResponsive] ❌ Failed to redirect to WeekHub (error %d)" % result)
	else:
		print("[HomeImprovedResponsive] ❌ Cannot redirect to WeekHub – tree unavailable")


func _on_layout_activated(layout_name: String):
	print(
		(
			"[HomeImproved] Layout activated: %s (Platform: %s)"
			% [layout_name, PlatformManager.get_platform_name() if PlatformManager else "Unknown"]
		)
	)
	_populate_current_layout()


func _populate_current_layout():
	"""Populate data for currently active layout"""
	var active = get_active_layout()
	if not active:
		push_warning("[HomeImproved] No active layout found")
		return

	match get_active_layout_name():
		"mobile":
			_populate_mobile_layout()
		"tablet":
			_populate_tablet_layout()
		"desktop":
			_populate_desktop_layout()


func _populate_mobile_layout():
	"""Populate mobile-specific layout with player data"""
	print("[HomeImproved] Populating mobile layout")

	# Week info
	if DateManager:
		if mobile_week_label:
			mobile_week_label.text = _get_term_label()
		if mobile_week_current:
			mobile_week_current.text = str(DateManager.current_week)

	# Player info
	if PlayerData:
		if mobile_player_name:
			mobile_player_name.text = PlayerData.player_name
		if mobile_ovr_value:
			# Assuming OVR calculation exists in PlayerData
			mobile_ovr_value.text = str(_calculate_ovr())

	# Condition
	if PlayerCondition:
		if mobile_condition_text:
			var condition = (
				PlayerCondition.get_condition_level() if PlayerCondition.has_method("get_condition_level") else 5
			)
			mobile_condition_text.text = _get_condition_text(condition)


func _populate_tablet_layout():
	"""Populate tablet-specific layout with player data"""
	print("[HomeImproved] Populating tablet layout")

	# Week info
	if DateManager:
		if tablet_week_label:
			tablet_week_label.text = _get_term_label()
		if tablet_week_current:
			tablet_week_current.text = str(DateManager.current_week)

	# Player info
	if PlayerData:
		if tablet_player_name:
			tablet_player_name.text = PlayerData.player_name
		if tablet_ovr_value:
			tablet_ovr_value.text = str(_calculate_ovr())

	# Condition
	if PlayerCondition:
		if tablet_condition_text:
			var condition = (
				PlayerCondition.get_condition_level() if PlayerCondition.has_method("get_condition_level") else 5
			)
			tablet_condition_text.text = _get_condition_text(condition)


func _populate_desktop_layout():
	"""Populate desktop-specific layout with player data"""
	print("[HomeImproved] Populating desktop layout")

	# Week info
	if DateManager:
		if desktop_week_label:
			desktop_week_label.text = _get_term_label()
		if desktop_week_current:
			desktop_week_current.text = str(DateManager.current_week)

	# Player info
	if PlayerData:
		if desktop_player_name:
			desktop_player_name.text = PlayerData.player_name
		if desktop_ovr_value:
			desktop_ovr_value.text = str(_calculate_ovr())

	# Condition
	if PlayerCondition:
		if desktop_condition_text:
			var condition = (
				PlayerCondition.get_condition_level() if PlayerCondition.has_method("get_condition_level") else 5
			)
			desktop_condition_text.text = _get_condition_text(condition)


func _get_term_label() -> String:
	"""Get current term label (e.g., '1학년 봄학기')"""
	if not DateManager:
		return "1학년 봄학기"

	var year = DateManager.current_year
	# Simple season detection based on week (weeks 1-19 = spring, 20-38 = fall)
	var season = "봄" if DateManager.current_week <= 19 else "가을"

	match year:
		1:
			return "1학년 %s학기" % season
		2:
			return "2학년 %s학기" % season
		3:
			return "3학년 %s학기" % season
		_:
			return "%d학년 %s학기" % [year, season]


func _get_condition_text(condition: int) -> String:
	"""Get condition text from condition level (1-5)"""
	match condition:
		5:
			return "최상"
		4:
			return "좋음"
		3:
			return "보통"
		2:
			return "피곤"
		1:
			return "탈진"
		_:
			return "보통"


func _calculate_ovr() -> int:
	"""Calculate overall rating (placeholder until real calculation is available)"""
	# PlayerData might have OVR calculation method, or we return placeholder
	if PlayerData and PlayerData.has_method("get_overall_rating"):
		return PlayerData.get_overall_rating()
	else:
		# Placeholder OVR until actual calculation is available
		return 65


func _validate_ui_standards():
	"""Validate UI against UIStandards requirements"""
	if not UIStandards or not UIStandards.validation_enabled:
		return

	print("[HomeImproved] Running UIStandards validation...")

	var violations = UIStandards.scan_scene_for_violations(self)

	if violations.touch_targets.size() > 0:
		print("[HomeImproved] Touch target violations found: %d" % violations.touch_targets.size())
		for node in violations.touch_targets:
			print("  - %s (size: %v)" % [node.name, node.size])
			# Auto-fix if enabled
			if UIStandards.auto_fix_touch_target(node):
				print("    ✓ Auto-fixed to: %v" % node.custom_minimum_size)

	if violations.font_sizes.size() > 0:
		print("[HomeImproved] Font size violations found: %d" % violations.font_sizes.size())
		for node in violations.font_sizes:
			print("  - %s" % node.name)

	if violations.touch_targets.size() == 0 and violations.font_sizes.size() == 0:
		print("[HomeImproved] ✅ UIStandards validation passed")


## Button signal handlers


func _on_advance_pressed():
	"""Handle advance button press (progress to next turn)"""
	print("[HomeImproved] Advance button pressed")

	if DateManager and DateManager.has_method("advance_turn"):
		DateManager.advance_turn()
		_populate_current_layout()  # Refresh UI
	else:
		push_warning("[HomeImproved] DateManager.advance_turn() not available")


func _on_status_pressed():
	"""Navigate to status screen"""
	print("[HomeImproved] Status button pressed")
	# TODO: Scene transition to StatusScreenImproved


func _on_save_pressed():
	"""Save game"""
	print("[HomeImproved] Save button pressed")
	# TODO: Implement save functionality when SaveManager API is known
	if SaveManager:
		push_warning("[HomeImproved] SaveManager exists but save_game() signature unknown")


func _on_training_pressed():
	"""Navigate to training screen"""
	print("[HomeImproved] Training button pressed")
	# TODO: Scene transition to TrainingScreenImproved


func _on_rest_pressed():
	"""Execute rest action"""
	print("[HomeImproved] Rest button pressed")
	# TODO: Execute rest action


func _on_go_out_pressed():
	"""Execute go out action"""
	print("[HomeImproved] Go out button pressed")
	# TODO: Execute go out action


## Debug helpers


func print_layout_debug_info():
	"""Print detailed layout information for debugging"""
	print_layout_info()  # From AdaptiveLayoutContainer

	print("\n[HomeImproved] Data State:")
	print("  DateManager: %s" % ("✓" if DateManager else "✗"))
	print("  PlayerData: %s" % ("✓" if PlayerData else "✗"))
	print("  PlayerCondition: %s" % ("✓" if PlayerCondition else "✗"))
	print("  SaveManager: %s" % ("✓" if SaveManager else "✗"))

	if PlatformManager:
		print("\n[PlatformManager]:")
		print("  Platform: %s" % PlatformManager.get_platform_name())
		print("  Orientation: %s" % PlatformManager.get_orientation_name())
		print("  Viewport: %v" % PlatformManager.viewport_size)
		print("  DPI: %d" % PlatformManager.dpi)
		print("  Layout Variant: %s" % PlatformManager.get_layout_variant())
