extends "res://scenes/academy/base/AdaptiveLayoutContainer.gd"
## Responsive Status Screen - Phase 7B Implementation
## Cross-platform UI with 3 layout variants (Mobile/Tablet/Desktop)

# Mobile layout node references
@onready var mobile_back_button = $MobilePortraitLayout/Header/HBox/BackButton
@onready var mobile_info_button = $MobilePortraitLayout/Header/HBox/InfoButton
@onready var mobile_ovr_value = $MobilePortraitLayout/TabContainer/Overview/VBox/OVRPanel/VBox/OVRValue
@onready
var mobile_condition_value = $MobilePortraitLayout/TabContainer/Overview/VBox/ConditionPanel/VBox/Grid/ConditionValue
@onready var mobile_fatigue_bar = $MobilePortraitLayout/TabContainer/Overview/VBox/ConditionPanel/VBox/Grid/FatigueBar

# Tablet layout node references
@onready var tablet_back_button = $TabletHybridLayout/Header/HBox/BackButton
@onready var tablet_info_button = $TabletHybridLayout/Header/HBox/InfoButton
@onready var tablet_ovr_value = $TabletHybridLayout/TabContainer/Overview/VBox/OVRPanel/VBox/HBox/OVRValue
@onready var tablet_potential = $TabletHybridLayout/TabContainer/Overview/VBox/OVRPanel/VBox/HBox/Details/Potential
@onready var tablet_growth = $TabletHybridLayout/TabContainer/Overview/VBox/OVRPanel/VBox/HBox/Details/Growth
@onready
var tablet_condition_value = $TabletHybridLayout/TabContainer/Overview/VBox/ConditionPanel/VBox/Grid/ConditionValue
@onready var tablet_fatigue_bar = $TabletHybridLayout/TabContainer/Overview/VBox/ConditionPanel/VBox/Grid/FatigueBar

# Desktop layout node references
@onready var desktop_back_button = $DesktopLandscapeLayout/Header/HBox/BackButton
@onready var desktop_info_button = $DesktopLandscapeLayout/Header/HBox/InfoButton
@onready
var desktop_ovr_value = $DesktopLandscapeLayout/TabContainer/Overview/HBox/RightPanel/OVRPanel/VBox/HBox/OVRValue
@onready
var desktop_potential = $DesktopLandscapeLayout/TabContainer/Overview/HBox/RightPanel/OVRPanel/VBox/HBox/Details/Potential
@onready
var desktop_growth = $DesktopLandscapeLayout/TabContainer/Overview/HBox/RightPanel/OVRPanel/VBox/HBox/Details/Growth
@onready
var desktop_condition_value = $DesktopLandscapeLayout/TabContainer/Overview/HBox/RightPanel/ConditionPanel/VBox/Grid/ConditionValue
@onready
var desktop_fatigue_bar = $DesktopLandscapeLayout/TabContainer/Overview/HBox/RightPanel/ConditionPanel/VBox/Grid/FatigueBar
@onready
var desktop_injury_value = $DesktopLandscapeLayout/TabContainer/Overview/HBox/RightPanel/ConditionPanel/VBox/Grid/InjuryValue

# Player status data (normally from PlayerCondition or GlobalCharacterData)
var player_ovr: int = 65
var player_potential: int = 85
var player_growth: int = 12
var player_condition: int = 5  # 1-5 scale
var player_fatigue: int = 30  # 0-100 scale
var player_injury: String = "정상"
var return_scene_path: String = "res://scenes/HomeImproved_Responsive.tscn"
var _close_callback: Callable = Callable()
var _training_summary_labels: Dictionary = {}
var _last_training_summary: Dictionary = {}
const TRAINING_INTENSITY_LABELS := {
	"light": "UI_TRAINING_INTENSITY_LIGHT",
	"normal": "UI_TRAINING_INTENSITY_NORMAL",
	"intense": "UI_TRAINING_INTENSITY_INTENSE"
}

# 42 attribute values (placeholder - normally from GlobalCharacterData)
var attributes = {
	"technical":
	{
		"Corners": 72,
		"Crossing": 68,
		"Dribbling": 75,
		"Finishing": 70,
		"FirstTouch": 73,
		"FreeKicks": 65,
		"Heading": 71,
		"LongShots": 67,
		"LongThrows": 62,
		"Marking": 64,
		"Passing": 76,
		"PenaltyTaking": 69,
		"Tackling": 66,
		"Technique": 74
	},
	"mental":
	{
		"Aggression": 72,
		"Anticipation": 68,
		"Bravery": 74,
		"Composure": 69,
		"Concentration": 65,
		"Decisions": 71,
		"Determination": 73,
		"Flair": 70,
		"Leadership": 67,
		"OffTheBall": 66,
		"Positioning": 72,
		"Teamwork": 74,
		"Vision": 71,
		"WorkRate": 68
	},
	"physical":
	{
		"Speed": 78,
		"Stamina": 72,
		"Strength": 69,
		"Agility": 75,
		"Balance": 71,
		"Jumping": 68,
		"NaturalFitness": 73,
		"Acceleration": 76
	},
	"goalkeeper":
	{"Reflexes": 82, "Handling": 79, "AerialAbility": 75, "CommandOfArea": 72, "Communication": 68, "Kicking": 74}
}


func _ready():
	super._ready()  # Call AdaptiveLayoutContainer._ready()

	print("[StatusScreen] Responsive scene initialized")

	# Apply ThemeManager styles
	_apply_theme_styles()

	# Connect layout activation signals
	layout_activated.connect(_on_layout_activated)


func _apply_theme_styles():
	"""ThemeManager 스타일 적용"""
	# 배경색 적용 (모든 레이아웃)
	for layout_name in ["MobilePortraitLayout", "TabletHybridLayout", "DesktopLandscapeLayout"]:
		var layout = get_node_or_null(layout_name)
		if not layout:
			continue

		# 배경
		var bg = layout.get_node_or_null("Background")
		if bg and bg is ColorRect:
			bg.color = ThemeManager.BG_PRIMARY

		# 헤더 스타일
		var header = layout.get_node_or_null("Header")
		if header and header is Panel:
			ThemeManager.apply_header_style(header)

		# Back/Info/Save 버튼 스타일
		var hbox = layout.get_node_or_null("Header/HBox")
		if hbox:
			for child in hbox.get_children():
				if child is Button:
					var style = ThemeManager.get_button_style("secondary")
					ThemeManager.apply_button_style(child, style)

		# TabContainer 스타일 (탭 버튼)
		var tab_container = layout.get_node_or_null("TabContainer")
		if tab_container:
			_apply_tab_container_style(tab_container)

	# Connect button signals for all layouts
	_connect_mobile_signals()
	_connect_tablet_signals()
	_connect_desktop_signals()

	# Phase 9.1: Add Save buttons to all layouts
	_add_save_buttons_to_all_layouts()

	# Connect GlobalCharacterData signals
	_connect_global_data_signals()

	# Wait for platform detection
	await get_tree().process_frame

	# Create attribute panels for all layouts
	_create_attribute_panels_for_all_layouts()

	_inject_training_summary_panels()
	_connect_ui_service_signals()
	_refresh_training_summary_from_hud()

	# Initial data population
	_populate_current_layout()

	# Validate UI standards
	_validate_ui_standards()


func set_return_scene(path: String) -> void:
	return_scene_path = path


func set_close_callback(callback: Callable) -> void:
	_close_callback = callback


func _connect_global_data_signals():
	"""Connect GlobalCharacterData signals for real-time updates"""
	if not GlobalCharacterData:
		push_warning("[StatusScreen] GlobalCharacterData not found")
		return

	GlobalCharacterData.data_changed.connect(_on_data_changed)
	print("[StatusScreen] GlobalCharacterData signals connected")


func _on_data_changed(category: String, attribute: String, new_value: int):
	"""Handle data change from GlobalCharacterData"""
	print("[StatusScreen] Data changed: %s.%s = %d" % [category, attribute, new_value])

	# Update attributes dictionary
	if attributes.has(category) and attributes[category].has(attribute):
		attributes[category][attribute] = new_value

		# Update UI for this specific attribute
		_update_attribute_ui(category, attribute, new_value)
	else:
		push_warning("[StatusScreen] Unknown attribute: %s.%s" % [category, attribute])


func _update_attribute_ui(category: String, attr_name: String, value: int):
	"""Update UI for a specific attribute across all layouts"""
	var layouts = ["MobilePortraitLayout", "TabletHybridLayout", "DesktopLandscapeLayout"]

	for layout_name in layouts:
		var layout_root = get_node_or_null(layout_name)
		if not layout_root:
			continue

		# Find the attribute panel
		var grid_name = _get_grid_name_for_category(category)
		var section_path = "TabContainer/Attributes/VBox/%sSection/%s" % [category.capitalize(), grid_name]
		var grid = layout_root.get_node_or_null(section_path)

		if not grid:
			continue

		# Find the specific attribute panel
		var panel = grid.get_node_or_null(attr_name)
		if not panel:
			continue

		# Update value label
		var value_label = panel.get_node_or_null("Value")
		if value_label:
			value_label.text = str(value)

		# Update progress bar
		var progress_bar = panel.get_node_or_null("Bar")
		if progress_bar:
			progress_bar.value = value

	print("[StatusScreen] Updated UI for %s.%s across all layouts" % [category, attr_name])


func _get_grid_name_for_category(category: String) -> String:
	"""Get grid name for attribute category"""
	match category:
		"technical":
			return "TechnicalGrid"
		"mental":
			return "MentalGrid"
		"physical":
			return "PhysicalGrid"
		"goalkeeper":
			return "GoalkeeperGrid"
		_:
			return ""


func _connect_mobile_signals():
	"""Connect Mobile layout button signals"""
	if mobile_back_button:
		mobile_back_button.pressed.connect(_on_back_pressed)
	if mobile_info_button:
		mobile_info_button.pressed.connect(_on_info_pressed)


func _connect_tablet_signals():
	"""Connect Tablet layout button signals"""
	if tablet_back_button:
		tablet_back_button.pressed.connect(_on_back_pressed)
	if tablet_info_button:
		tablet_info_button.pressed.connect(_on_info_pressed)


func _connect_desktop_signals():
	"""Connect Desktop layout button signals"""
	if desktop_back_button:
		desktop_back_button.pressed.connect(_on_back_pressed)
	if desktop_info_button:
		desktop_info_button.pressed.connect(_on_info_pressed)


func _create_attribute_panels_for_all_layouts():
	"""Create attribute panels dynamically for all layouts"""
	print("[StatusScreen] Creating attribute panels for all layouts...")

	# Mobile layout
	_create_attribute_panels_for_layout("MobilePortraitLayout", 1)

	# Tablet layout
	_create_attribute_panels_for_layout("TabletHybridLayout", 2)

	# Desktop layout
	_create_attribute_panels_for_layout("DesktopLandscapeLayout", 3)


func _inject_training_summary_panels() -> void:
	_add_training_summary_panel("MobilePortraitLayout", "MobilePortraitLayout/TabContainer/Overview/VBox")
	_add_training_summary_panel("TabletHybridLayout", "TabletHybridLayout/TabContainer/Overview/VBox")
	_add_training_summary_panel(
		"DesktopLandscapeLayout", "DesktopLandscapeLayout/TabContainer/Overview/HBox/RightPanel"
	)


func _add_training_summary_panel(layout_name: String, container_path: String) -> void:
	var container = get_node_or_null(container_path)
	if not container:
		return
	if container.has_node("TrainingSummaryPanel"):
		var detail_label = container.get_node("TrainingSummaryPanel/Detail")
		_training_summary_labels[layout_name] = detail_label
		return
	var panel := VBoxContainer.new()
	panel.name = "TrainingSummaryPanel"
	panel.add_theme_constant_override("separation", 4)
	var title := Label.new()
	title.text = tr("UI_STATUS_RECENT_TRAINING_TITLE")
	title.add_theme_font_size_override("font_size", 28)
	panel.add_child(title)
	var detail := Label.new()
	detail.name = "Detail"
	detail.text = tr("UI_STATUS_RECENT_TRAINING_EMPTY")
	panel.add_child(detail)
	container.add_child(panel)
	_training_summary_labels[layout_name] = detail


func _connect_ui_service_signals() -> void:
	if UIService and not UIService.hud_update.is_connected(_on_hud_update):
		UIService.hud_update.connect(_on_hud_update)


func _refresh_training_summary_from_hud() -> void:
	if UIService:
		var existing := UIService.get_hud_element("training_last_result")
		if existing and not existing.is_empty():
			_last_training_summary = existing
		else:
			_last_training_summary = {}
	_update_training_summary_labels()


func _on_hud_update(element: String, data: Dictionary) -> void:
	if element != "training_last_result":
		return
	_last_training_summary = data.duplicate(true)
	_update_training_summary_labels()


func _update_training_summary_labels() -> void:
	var summary_text := tr("UI_STATUS_RECENT_TRAINING_EMPTY")
	var tooltip := ""
	if not _last_training_summary.is_empty():
		summary_text = _format_training_summary_text(_last_training_summary)
		tooltip = _build_training_summary_tooltip(_last_training_summary)
	for layout_name in _training_summary_labels.keys():
		var label: Label = _training_summary_labels[layout_name]
		if not label:
			continue
		label.text = summary_text
		label.tooltip_text = tooltip


func _format_training_summary_text(data: Dictionary) -> String:
	var training_name := String(data.get("name", tr("UI_STATUS_RECENT_TRAINING_TITLE")))
	var mode_label := String(data.get("mode", ""))
	if mode_label.is_empty():
		mode_label = _describe_training_mode(String(data.get("mode_id", "personal")))
	var intensity_label := String(data.get("intensity", ""))
	if intensity_label.is_empty():
		intensity_label = _resolve_intensity_label(String(data.get("intensity_id", "")))
	var deck_bonus := int(data.get("deck_bonus", 0))
	var parts := ["%s (%s · %s)" % [training_name, mode_label, intensity_label]]
	if deck_bonus != 0:
		parts.append("%s %+d%%" % [tr("UI_TRAINING_RESULT_FIELD_DECK_BONUS"), deck_bonus])
	var coach_log_variant: Variant = data.get("coach_bonus_log", [])
	if coach_log_variant is Array and not (coach_log_variant as Array).is_empty():
		var coach_log: Array = coach_log_variant
		parts.append("%s %d" % [tr("UI_TRAINING_RESULT_FIELD_COACH_BONUS"), coach_log.size()])
	if bool(data.get("needs_rest_warning", false)):
		parts.append(tr("UI_TRAINING_SUMMARY_REST_WARNING"))
	var injury_risk := float(data.get("injury_risk", -1.0))
	if injury_risk >= 0.0:
		parts.append("%s %.0f%%" % [tr("UI_TRAINING_RESULT_FIELD_INJURY"), injury_risk * 100.0])
	return " | ".join(parts)


func _build_training_summary_tooltip(data: Dictionary) -> String:
	if data.is_empty():
		return ""
	var lines := PackedStringArray()
	var changes_variant: Variant = data.get("changes", {})
	var changes: Dictionary = changes_variant if changes_variant is Dictionary else {}
	if not changes.is_empty():
		var change_list: Array = []
		for attr in changes.keys():
			change_list.append("%s %+d" % [attr, int(changes[attr])])
		lines.append("%s: %s" % [tr("UI_TRAINING_RESULT_FIELD_CHANGES"), ", ".join(change_list)])
	var deck_bonus_data_variant: Variant = data.get("deck_bonus_data", {})
	var deck_bonus_data: Dictionary = deck_bonus_data_variant if deck_bonus_data_variant is Dictionary else {}
	if deck_bonus_data is Dictionary and not deck_bonus_data.is_empty():
		var total_pct := float(deck_bonus_data.get("total_bonus", deck_bonus_data.get("total", 0.0))) * 100.0
		lines.append("%s: %+0.1f%%" % [tr("UI_TRAINING_RESULT_FIELD_DECK_BONUS"), total_pct])
		var active_cards_variant: Variant = deck_bonus_data.get("active_characters", [])
		if active_cards_variant is Array and not (active_cards_variant as Array).is_empty():
			var active_cards: Array = active_cards_variant
			lines.append("%s: %s" % [tr("UI_STATUS_RECENT_TRAINING_DECK_CARDS"), ", ".join(active_cards)])
	var deck_snapshot_variant: Variant = data.get("deck_snapshot", [])
	if deck_snapshot_variant is Array and not (deck_snapshot_variant as Array).is_empty():
		var deck_snapshot: Array = deck_snapshot_variant
		var card_names: Array = []
		for card in deck_snapshot:
			if card is Dictionary:
				var label := String(card.get("name", card.get("character_name", card.get("id", ""))))
				if not label.is_empty():
					card_names.append(label)
		if card_names.size() > 0:
			lines.append("%s: %s" % [tr("UI_STATUS_RECENT_TRAINING_ACTIVE_DECK"), ", ".join(card_names)])
	var training_load_variant: Variant = data.get("training_load", {})
	var training_load: Dictionary = training_load_variant if training_load_variant is Dictionary else {}
	if training_load is Dictionary and not training_load.is_empty():
		if training_load.has("load_ratio"):
			lines.append("%s: %.2f" % [tr("UI_STATUS_RECENT_TRAINING_LOAD_RATIO"), float(training_load["load_ratio"])])
		var load_state_variant: Variant = training_load.get("state", training_load.get("load_state", ""))
		var load_state_str := String(load_state_variant)
		if not load_state_str.is_empty():
			lines.append("%s: %s" % [tr("UI_STATUS_RECENT_TRAINING_LOAD_STATE"), load_state_str])
	var coach_bonus_log_variant: Variant = data.get("coach_bonus_log", [])
	if coach_bonus_log_variant is Array and not (coach_bonus_log_variant as Array).is_empty():
		var coach_bonus_log: Array = coach_bonus_log_variant
		var coach_lines := PackedStringArray()
		for entry in coach_bonus_log:
			if entry is Dictionary:
				var coach_name := String(entry.get("card_name", entry.get("coach_id", entry.get("id", "Coach"))))
				if coach_name.is_empty():
					continue
				var bonus_value := float(entry.get("bonus", entry.get("value", 0.0)))
				if bonus_value != 0.0:
					coach_lines.append("%s %+0.1f%%" % [coach_name, bonus_value * 100.0])
				else:
					coach_lines.append(coach_name)
		if coach_lines.size() > 0:
			lines.append("%s: %s" % [tr("UI_STATUS_RECENT_TRAINING_COACH_CARDS"), ", ".join(coach_lines)])
	if data.has("timestamp"):
		lines.append(
			(
				"%s: %s"
				% [
					tr("UI_STATUS_RECENT_TRAINING_COMPLETED_AT"),
					Time.get_datetime_string_from_unix_time(int(data["timestamp"]))
				]
			)
		)
	return "\n".join(lines)


func _describe_training_mode(mode_id: String) -> String:
	match mode_id:
		"team":
			return tr("UI_TRAINING_MODE_TEAM")
		"special":
			return tr("UI_TRAINING_MODE_SPECIAL")
		_:
			return tr("UI_TRAINING_MODE_PERSONAL")


func _resolve_intensity_label(intensity_id: String) -> String:
	if intensity_id.is_empty():
		return tr("UI_TRAINING_INTENSITY_NORMAL")
	if TRAINING_INTENSITY_LABELS.has(intensity_id):
		return tr(TRAINING_INTENSITY_LABELS[intensity_id])
	return intensity_id.capitalize()


func _create_attribute_panels_for_layout(layout_name: String, grid_columns: int):
	"""Create attribute panels for a specific layout"""
	var layout_root = get_node_or_null(layout_name)
	if not layout_root:
		push_warning("[StatusScreen] Layout not found: %s" % layout_name)
		return

	# Find Attributes tab VBox
	var attributes_vbox = layout_root.get_node_or_null("TabContainer/Attributes/VBox")
	if not attributes_vbox:
		push_warning("[StatusScreen] Attributes VBox not found in %s" % layout_name)
		return

	# Create panels for each category
	_create_category_panels(attributes_vbox, "TechnicalSection", "TechnicalGrid", "technical", grid_columns)
	_create_category_panels(attributes_vbox, "MentalSection", "MentalGrid", "mental", grid_columns)
	_create_category_panels(attributes_vbox, "PhysicalSection", "PhysicalGrid", "physical", grid_columns)
	_create_category_panels(attributes_vbox, "GoalkeeperSection", "GoalkeeperGrid", "goalkeeper", grid_columns)


func _create_category_panels(
	parent_vbox: VBoxContainer, section_name: String, grid_name: String, category: String, columns: int
):
	"""Create attribute panels for a specific category"""
	var section = parent_vbox.get_node_or_null(section_name)

	# Create section if it doesn't exist
	if not section:
		section = VBoxContainer.new()
		section.name = section_name
		section.add_theme_constant_override("separation", 15)
		parent_vbox.add_child(section)

		# Create title label
		var title = Label.new()
		title.name = section_name.replace("Section", "Title")
		title.add_theme_color_override("font_color", _get_category_color(category))
		title.add_theme_font_size_override("font_size", 24)
		title.text = _get_category_title(category)
		section.add_child(title)

		# Create grid container
		var grid = GridContainer.new()
		grid.name = grid_name
		grid.add_theme_constant_override("h_separation", 20)
		grid.add_theme_constant_override("v_separation", 10)
		grid.columns = columns
		section.add_child(grid)

	var grid = section.get_node_or_null(grid_name)
	if not grid:
		push_warning("[StatusScreen] Grid not found: %s" % grid_name)
		return

	# Clear existing children (if any)
	for child in grid.get_children():
		child.queue_free()

	# Set grid columns
	grid.columns = columns

	# Create panel for each attribute in this category
	if not attributes.has(category):
		return

	for attr_name in attributes[category].keys():
		var panel = _create_attribute_panel(attr_name, attributes[category][attr_name])
		grid.add_child(panel)


func _create_attribute_panel(attr_name: String, value: int) -> Panel:
	"""Create a single attribute panel with label, value, and progress bar"""
	var panel = Panel.new()
	panel.custom_minimum_size = Vector2(500, 60)
	panel.name = attr_name

	# 패널 스타일 적용
	var panel_style = ThemeManager.create_card_style()
	panel_style.content_margin_left = ThemeManager.SPACE_SM
	panel_style.content_margin_right = ThemeManager.SPACE_SM
	panel_style.content_margin_top = ThemeManager.SPACE_XS
	panel_style.content_margin_bottom = ThemeManager.SPACE_XS
	panel.add_theme_stylebox_override("panel", panel_style)

	var hbox = HBoxContainer.new()
	hbox.anchor_right = 1.0
	hbox.anchor_bottom = 1.0
	hbox.add_theme_constant_override("separation", ThemeManager.SPACE_SM)
	panel.add_child(hbox)

	# Attribute label
	var label = Label.new()
	label.text = _get_attribute_display_name(attr_name)
	label.custom_minimum_size = Vector2(150, 0)
	label.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)
	label.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
	hbox.add_child(label)

	# Value label - 능력치 색상 적용
	var value_label = Label.new()
	value_label.text = str(value)
	value_label.custom_minimum_size = Vector2(50, 0)
	value_label.add_theme_font_size_override("font_size", ThemeManager.FONT_H3)
	value_label.add_theme_color_override("font_color", ThemeManager.get_stat_color(value))
	value_label.name = "Value"
	hbox.add_child(value_label)

	# Progress bar - ThemeManager 스타일
	var progress = ProgressBar.new()
	progress.min_value = 1
	progress.max_value = 99
	progress.value = value
	progress.custom_minimum_size = Vector2(250, 25)
	progress.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	progress.name = "Bar"

	# ProgressBar 스타일
	var fill_style = StyleBoxFlat.new()
	fill_style.bg_color = ThemeManager.get_stat_color(value)
	fill_style.set_corner_radius_all(ThemeManager.CORNER_RADIUS_SMALL)
	progress.add_theme_stylebox_override("fill", fill_style)

	var bg_style = StyleBoxFlat.new()
	bg_style.bg_color = ThemeManager.BG_TERTIARY
	bg_style.set_corner_radius_all(ThemeManager.CORNER_RADIUS_SMALL)
	progress.add_theme_stylebox_override("background", bg_style)

	hbox.add_child(progress)

	return panel


func _get_attribute_display_name(attr_name: String) -> String:
	"""Convert attribute name to Korean display name"""
	var display_names = {
		# Technical
		"Corners": "코너킥",
		"Crossing": "크로스",
		"Dribbling": "드리블",
		"Finishing": "결정력",
		"FirstTouch": "퍼스트터치",
		"FreeKicks": "프리킥",
		"Heading": "헤딩",
		"LongShots": "중거리슛",
		"LongThrows": "롱스로",
		"Marking": "마크",
		"Passing": "패스",
		"PenaltyTaking": "페널티킥",
		"Tackling": "태클",
		"Technique": "테크닉",
		# Mental
		"Aggression": "적극성",
		"Anticipation": "예측력",
		"Bravery": "용기",
		"Composure": "침착성",
		"Concentration": "집중력",
		"Decisions": "판단력",
		"Determination": "정신력",
		"Flair": "창의성",
		"Leadership": "리더십",
		"OffTheBall": "오프더볼",
		"Positioning": "위치선정",
		"Teamwork": "팀워크",
		"Vision": "시야",
		"WorkRate": "활동량",
		# Physical
		"Speed": "속도",
		"Stamina": "스태미나",
		"Strength": "힘",
		"Agility": "민첩성",
		"Balance": "밸런스",
		"Jumping": "점프력",
		"NaturalFitness": "타고난체력",
		"Acceleration": "가속력",
		# Goalkeeper
		"Reflexes": "반사신경",
		"Handling": "핸들링",
		"AerialAbility": "공중장악",
		"CommandOfArea": "진영장악",
		"Communication": "커뮤니케이션",
		"Kicking": "킥력"
	}
	return display_names.get(attr_name, attr_name)


func _get_category_title(category: String) -> String:
	"""Get category display title"""
	match category:
		"technical":
			return "🎯 기술 능력치 (14개)"
		"mental":
			return "🧠 정신 능력치 (14개)"
		"physical":
			return "💪 신체 능력치 (8개)"
		"goalkeeper":
			return "🧤 골키퍼 능력치 (6개)"
		_:
			return category


func _get_category_color(category: String) -> Color:
	"""Get category color using ThemeManager"""
	match category:
		"technical":
			return ThemeManager.INFO  # Blue
		"mental":
			return ThemeManager.ACCENT  # Light Blue
		"physical":
			return ThemeManager.WARNING  # Orange/Amber
		"goalkeeper":
			return ThemeManager.SUCCESS  # Green
		_:
			return ThemeManager.TEXT_PRIMARY


func _apply_tab_container_style(tab_container: TabContainer):
	"""TabContainer에 ThemeManager 스타일 적용"""
	# 탭 폰트 색상
	tab_container.add_theme_color_override("font_selected_color", ThemeManager.ACCENT)
	tab_container.add_theme_color_override("font_unselected_color", ThemeManager.TEXT_SECONDARY)
	tab_container.add_theme_color_override("font_hovered_color", ThemeManager.TEXT_PRIMARY)
	tab_container.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)

	# 탭 패널 배경
	var panel_style = StyleBoxFlat.new()
	panel_style.bg_color = ThemeManager.BG_SECONDARY
	panel_style.set_corner_radius_all(ThemeManager.CORNER_RADIUS_MEDIUM)
	tab_container.add_theme_stylebox_override("panel", panel_style)

	# 선택된 탭 스타일
	var tab_selected = StyleBoxFlat.new()
	tab_selected.bg_color = ThemeManager.BG_SECONDARY
	tab_selected.border_width_bottom = 2
	tab_selected.border_color = ThemeManager.ACCENT
	tab_container.add_theme_stylebox_override("tab_selected", tab_selected)

	# 미선택 탭 스타일
	var tab_unselected = StyleBoxFlat.new()
	tab_unselected.bg_color = ThemeManager.BG_TERTIARY
	tab_container.add_theme_stylebox_override("tab_unselected", tab_unselected)


func _on_layout_activated(layout_name: String):
	"""Handle layout activation"""
	print(
		(
			"[StatusScreen] Layout activated: %s (Platform: %s)"
			% [layout_name, PlatformManager.get_platform_name() if PlatformManager else "Unknown"]
		)
	)
	_populate_current_layout()


func _populate_current_layout():
	"""Populate data for currently active layout"""
	var active = get_active_layout()
	if not active:
		push_warning("[StatusScreen] No active layout found")
		return

	match get_active_layout_name():
		"mobile":
			_populate_mobile_layout()
		"tablet":
			_populate_tablet_layout()
		"desktop":
			_populate_desktop_layout()

	_update_training_summary_labels()


func _populate_mobile_layout():
	"""Populate mobile-specific layout with player data"""
	print("[StatusScreen] Populating mobile layout")

	# OVR and basic stats
	if mobile_ovr_value:
		mobile_ovr_value.text = str(player_ovr)

	# Condition (emoji + text)
	if mobile_condition_value:
		var condition_emoji = _get_condition_emoji(player_condition)
		var condition_text = _get_condition_text(player_condition)
		mobile_condition_value.text = "%s %s (%d/5)" % [condition_emoji, condition_text, player_condition]

	# Fatigue bar
	if mobile_fatigue_bar:
		mobile_fatigue_bar.value = player_fatigue

	# Note: Attribute values would be populated from GlobalCharacterData
	# For now, data is hardcoded in attributes dictionary


func _populate_tablet_layout():
	"""Populate tablet-specific layout with player data"""
	print("[StatusScreen] Populating tablet layout")

	# OVR and stats
	if tablet_ovr_value:
		tablet_ovr_value.text = str(player_ovr)
	if tablet_potential:
		tablet_potential.text = "잠재력: %d" % player_potential
	if tablet_growth:
		tablet_growth.text = "성장률: +%d (이번 학기)" % player_growth

	# Condition (emoji in separate node)
	if tablet_condition_value:
		var emoji_node = tablet_condition_value.get_node("Emoji")
		var text_node = tablet_condition_value.get_node("Text")
		if emoji_node and text_node:
			emoji_node.text = _get_condition_emoji(player_condition)
			text_node.text = " %s (%d/5)" % [_get_condition_text(player_condition), player_condition]

	# Fatigue bar
	if tablet_fatigue_bar:
		tablet_fatigue_bar.value = player_fatigue


func _populate_desktop_layout():
	"""Populate desktop-specific layout with player data"""
	print("[StatusScreen] Populating desktop layout")

	# OVR and stats (desktop shows more detail)
	if desktop_ovr_value:
		desktop_ovr_value.text = str(player_ovr)
	if desktop_potential:
		desktop_potential.text = "잠재력: %d" % player_potential
	if desktop_growth:
		desktop_growth.text = "성장률: +%d (이번 학기)" % player_growth

	# Condition (emoji in separate node)
	if desktop_condition_value:
		var emoji_node = desktop_condition_value.get_node("Emoji")
		var text_node = desktop_condition_value.get_node("Text")
		if emoji_node and text_node:
			emoji_node.text = _get_condition_emoji(player_condition)
			text_node.text = " %s (%d/5)" % [_get_condition_text(player_condition), player_condition]

	# Fatigue bar
	if desktop_fatigue_bar:
		desktop_fatigue_bar.value = player_fatigue

	# Injury status (desktop-only)
	if desktop_injury_value:
		desktop_injury_value.text = player_injury
		# Color based on injury state
		if player_injury == "없음":
			desktop_injury_value.add_theme_color_override("font_color", Color(0.3, 1, 0.3, 1))  # Green
		else:
			desktop_injury_value.add_theme_color_override("font_color", Color(1, 0.3, 0.3, 1))  # Red


func _get_condition_emoji(condition: int) -> String:
	"""Get emoji for condition level (1-5)"""
	match condition:
		5:
			return "😄"
		4:
			return "😊"
		3:
			return "😐"
		2:
			return "😟"
		1:
			return "😢"
		_:
			return "😐"


func _get_condition_text(condition: int) -> String:
	"""Get text description for condition level (1-5)"""
	match condition:
		5:
			return "최상"
		4:
			return "좋음"
		3:
			return "보통"
		2:
			return "나쁨"
		1:
			return "매우 나쁨"
		_:
			return "보통"


func _get_condition_color(condition: int) -> Color:
	"""Get color for condition level using ThemeManager"""
	return ThemeManager.get_condition_color(condition)


func _validate_ui_standards():
	"""Validate UI against UIStandards requirements"""
	validate_ui_standards_base()


## Button signal handlers


func _on_back_pressed():
	"""Navigate back to home screen"""
	print("[StatusScreen] Back button pressed")
	if _close_callback.is_valid():
		_close_callback.call()
	elif return_scene_path != "":
		get_tree().change_scene_to_file(return_scene_path)


func _on_info_pressed():
	"""Show info dialog"""
	print("[StatusScreen] Info button pressed")
	# TODO: Show explanation dialog for attributes
	# InfoDialog.show("선수 능력치는 훈련과 경기를 통해 성장합니다...")


## Phase 9.1: Save/Load functionality


func _add_save_buttons_to_all_layouts():
	"""Add Save buttons to all layout headers (Phase 9.1)"""
	print("[StatusScreen] Adding Save buttons to all layouts...")

	# Mobile layout
	_add_save_button_to_layout("MobilePortraitLayout/Header/HBox")

	# Tablet layout
	_add_save_button_to_layout("TabletHybridLayout/Header/HBox")

	# Desktop layout
	_add_save_button_to_layout("DesktopLandscapeLayout/Header/HBox")


func _add_save_button_to_layout(header_path: String):
	"""Add Save button to a specific layout header"""
	var header_hbox = get_node_or_null(header_path)
	if not header_hbox:
		push_warning("[StatusScreen] Header HBox not found: %s" % header_path)
		return

	# Create Save button
	var save_button = Button.new()
	save_button.name = "SaveButton"
	save_button.text = "💾 저장"
	save_button.custom_minimum_size = Vector2(120, 60)
	save_button.pressed.connect(_on_save_pressed)

	# Add button to header (before InfoButton if it exists)
	var info_button_index = -1
	for i in range(header_hbox.get_child_count()):
		var child = header_hbox.get_child(i)
		if child.name == "InfoButton":
			info_button_index = i
			break

	if info_button_index >= 0:
		header_hbox.add_child(save_button)
		header_hbox.move_child(save_button, info_button_index)
	else:
		header_hbox.add_child(save_button)

	print("[StatusScreen] Save button added to: %s" % header_path)


func _on_save_pressed():
	"""Handle Save button press - Show SaveLoadDialog in save mode"""
	print("[StatusScreen] Save button pressed")

	# Load SaveLoadDialog scene and set to save mode
	var save_load_scene = load("res://scenes/ui/SaveLoadDialog.tscn")
	if not save_load_scene:
		push_error("[StatusScreen] SaveLoadDialog scene not found")
		return

	var save_load_dialog = save_load_scene.instantiate()
	if not save_load_dialog:
		push_error("[StatusScreen] Failed to instantiate SaveLoadDialog")
		return

	# Set mode to "save"
	if save_load_dialog.has_method("set_mode"):
		save_load_dialog.set_mode("save")

	# Add to scene tree
	get_tree().root.add_child(save_load_dialog)
	print("[StatusScreen] SaveLoadDialog opened in save mode")


## Public API for data updates


func update_player_data(ovr: int, potential: int, growth: int):
	"""Update player OVR, potential, and growth (called by external manager)"""
	player_ovr = ovr
	player_potential = potential
	player_growth = growth
	_populate_current_layout()


func update_condition(condition: int, fatigue: int, injury: String = "없음"):
	"""Update player condition, fatigue, and injury status"""
	player_condition = clamp(condition, 1, 5)
	player_fatigue = clamp(fatigue, 0, 100)
	player_injury = injury
	_populate_current_layout()


func update_attributes(category: String, attr_name: String, value: int):
	"""Update a specific attribute value"""
	if attributes.has(category) and attributes[category].has(attr_name):
		attributes[category][attr_name] = clamp(value, 1, 99)
		# TODO: Update specific attribute UI nodes
		# For full implementation, would need to update ProgressBar values
		print("[StatusScreen] Updated %s.%s to %d" % [category, attr_name, value])


## Debug helpers


func print_layout_debug_info():
	"""Print detailed layout information for debugging"""
	print_layout_info()  # From AdaptiveLayoutContainer

	print("\n[StatusScreen] Player Data:")
	print("  OVR: %d (Potential: %d)" % [player_ovr, player_potential])
	print("  Condition: %s (%d/5)" % [_get_condition_text(player_condition), player_condition])
	print("  Fatigue: %d%%" % player_fatigue)
	print("  Injury: %s" % player_injury)

	if PlatformManager:
		print("\n[PlatformManager]:")
		print("  Platform: %s" % PlatformManager.get_platform_name())
		print("  Orientation: %s" % PlatformManager.get_orientation_name())
		print("  Viewport: %v" % PlatformManager.viewport_size)
