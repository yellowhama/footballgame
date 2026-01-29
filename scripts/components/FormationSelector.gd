extends Control

# Formation Selector UI Component - Interactive formation selection with instant feedback
# Implements FR-018: Immediate feedback on formation choice affects team projections
# Integrates with TacticalManager and FormationDisplay for complete user experience

signal formation_preview_started(formation_id: String)
signal formation_preview_ended
signal formation_selected(formation_id: String, success: bool, feedback_data: Dictionary)

# UI Components
@export var button_container: VBoxContainer  # Changed to VBoxContainer for vertical list
@export var feedback_panel: Panel
@export var formation_display: Control  # Reference to FormationDisplay component

# Formation button settings
@export var formation_button_style: StyleBox
@export var selected_button_color: Color = Color.GREEN
@export var preview_button_color: Color = Color.YELLOW
@export var disabled_button_color: Color = Color.GRAY

# Formation data (loaded from OpenFootball)
var available_formations: Array = []  # Will be populated from Rust API
var formation_buttons: Dictionary = {}
var formation_info: Dictionary = {}  # Will be populated from Rust API

# Integration with systems
var tactical_manager: Node = null
var coach_system: Node = null

# Current state
var current_formation: String = ""
var selected_formation: String = ""
var preview_formation: String = ""
var is_previewing: bool = false

# Feedback display components
var feedback_title: Label
var effectiveness_label: Label
var suitability_label: Label
var advantages_label: RichTextLabel
var disadvantages_label: RichTextLabel
var change_button: Button
var cancel_button: Button


func _ready():
	print("[FormationSelector] Initializing formation selector UI")

	# Connect to systems
	_connect_to_systems()

	# Initialize formation data
	_initialize_formation_data()

	# Setup UI components
	_setup_ui_layout()
	_setup_formation_buttons()
	_setup_feedback_panel()

	# Load current formation
	_update_current_formation()

	print("[FormationSelector] Formation selector ready")


func _connect_to_systems():
	"""Connect to TacticalManager and other systems"""
	tactical_manager = get_node_or_null("/root/TacticalManager")
	if not tactical_manager:
		tactical_manager = get_tree().get_first_node_in_group("tactical_manager")

	coach_system = get_node_or_null("/root/CoachSystem")
	if not coach_system:
		coach_system = get_tree().get_first_node_in_group("coach_system")

	# Connect to TacticalManager signals
	if tactical_manager:
		if tactical_manager.has_signal("formation_changed"):
			tactical_manager.formation_changed.connect(_on_formation_changed)
		if tactical_manager.has_signal("manager_authority_confirmed"):
			tactical_manager.manager_authority_confirmed.connect(_on_authority_confirmed)

	print(
		(
			"[FormationSelector] Connected to systems - Tactical: %s, Coach: %s"
			% [tactical_manager != null, coach_system != null]
		)
	)


func _initialize_formation_data():
	"""Initialize formation information data from OpenFootball/Rust backend"""
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine:
		print("[FormationSelector] Warning: FootballRustEngine not found")
		_load_fallback_formations()
		return

	if not rust_engine.is_ready():
		print("[FormationSelector] Warning: FootballRustEngine not ready")
		_load_fallback_formations()
		return

	# Get all formations from Rust API
	var result = rust_engine.get_all_formations()

	if not result.get("success", false):
		print("[FormationSelector] Error loading formations: %s" % result.get("error", "Unknown"))
		_load_fallback_formations()
		return

	var formations = result.get("formations", [])
	if formations.size() == 0:
		print("[FormationSelector] Warning: No formations returned from API")
		_load_fallback_formations()
		return

	# Convert formations to our format
	available_formations.clear()
	formation_info.clear()

	for formation in formations:
		var formation_id = formation.get("id", "")
		if formation_id == "":
			continue

		available_formations.append(formation_id)

		formation_info[formation_id] = {
			"name": formation.get("name_en", formation_id),
			"name_ko": formation.get("name_ko", formation_id),
			"description": formation.get("description_ko", "설명 없음"),
			"style": formation.get("tactical_style", "Unknown"),
			"advantages": formation.get("strengths", []),
			"disadvantages": formation.get("weaknesses", []),
			"tactical_complexity": 3  # Default
		}

	print("[FormationSelector] Loaded %d formations from OpenFootball" % available_formations.size())


func _load_fallback_formations():
	"""Load fallback formations when API fails"""
	available_formations = ["T442", "T433", "T352"]
	formation_info = {
		"T442":
		{
			"name": "4-4-2",
			"name_ko": "4-4-2",
			"description": "균형잡힌 클래식 포메이션",
			"style": "Balanced",
			"advantages": ["안정적인 수비", "균형잡힌 공수"],
			"disadvantages": ["측면 공격력 부족"],
			"tactical_complexity": 2
		},
		"T433":
		{
			"name": "4-3-3",
			"name_ko": "4-3-3",
			"description": "공격적인 포메이션",
			"style": "Attacking",
			"advantages": ["강력한 압박", "측면 공격"],
			"disadvantages": ["중원 열세 가능"],
			"tactical_complexity": 4
		},
		"T352":
		{
			"name": "3-5-2",
			"name_ko": "3-5-2",
			"description": "중원 장악 중심",
			"style": "Possession",
			"advantages": ["중원 숫자 우위", "측면 공간 활용"],
			"disadvantages": ["측면 수비 취약"],
			"tactical_complexity": 5
		}
	}


func _setup_ui_layout():
	"""Setup the main UI layout structure"""
	# Main container - fill parent
	var main_container = VBoxContainer.new()
	main_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_container.anchor_right = 1.0
	main_container.anchor_bottom = 1.0
	main_container.grow_horizontal = Control.GROW_DIRECTION_BOTH
	main_container.grow_vertical = Control.GROW_DIRECTION_BOTH
	add_child(main_container)

	# Scroll container for formations
	var scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	main_container.add_child(scroll)

	# Formation buttons container in a VBox (vertical layout is better for many formations)
	var vbox = VBoxContainer.new()
	vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(vbox)

	button_container = vbox

	# Feedback panel (hidden by default, shown when previewing/selecting)
	feedback_panel = Panel.new()
	feedback_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
	feedback_panel.custom_minimum_size = Vector2(0, 300)
	feedback_panel.visible = false
	main_container.add_child(feedback_panel)


func _setup_formation_buttons():
	"""Setup formation selection buttons"""
	formation_buttons.clear()

	for formation_id in available_formations:
		var button = Button.new()
		var info = formation_info.get(formation_id, {})
		# Show Korean name if available, fallback to English name
		var display_name = info.get("name_ko", info.get("name", formation_id))
		button.text = display_name
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.custom_minimum_size = Vector2(0, 60)  # Full width, 60px tall
		button.add_theme_font_size_override("font_size", 18)
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT  # Left-align text

		# Connect signals
		button.pressed.connect(_on_formation_button_pressed.bind(formation_id))
		button.mouse_entered.connect(_on_formation_button_hover.bind(formation_id))
		button.mouse_exited.connect(_on_formation_button_unhover.bind(formation_id))

		# Style the button
		_style_formation_button(button, formation_id)

		button_container.add_child(button)
		formation_buttons[formation_id] = button

	print("[FormationSelector] Created %d formation buttons" % formation_buttons.size())


func _style_formation_button(button: Button, formation_id: String):
	"""Apply styling to formation button"""
	# Check if formation is available
	var is_available = _is_formation_available(formation_id)
	var is_current = formation_id == current_formation

	if not is_available:
		button.disabled = true
		button.modulate = disabled_button_color
		button.tooltip_text = "이 포메이션을 사용하려면 코치의 전술 지식이 필요합니다."
	elif is_current:
		button.modulate = selected_button_color
		button.tooltip_text = "현재 포메이션"
	else:
		button.modulate = Color.WHITE
		button.tooltip_text = "클릭하여 포메이션 변경"


func _setup_feedback_panel():
	"""Setup the feedback display panel"""
	var margin = MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	margin.anchor_right = 1.0
	margin.anchor_bottom = 1.0
	margin.add_theme_constant_override("margin_left", 15)
	margin.add_theme_constant_override("margin_top", 15)
	margin.add_theme_constant_override("margin_right", 15)
	margin.add_theme_constant_override("margin_bottom", 15)
	feedback_panel.add_child(margin)

	var feedback_container = VBoxContainer.new()
	feedback_container.add_theme_constant_override("separation", 10)
	margin.add_child(feedback_container)

	# Feedback title
	feedback_title = Label.new()
	feedback_title.text = "포메이션 정보"
	feedback_title.add_theme_font_size_override("font_size", 18)
	feedback_container.add_child(feedback_title)

	# Effectiveness display
	effectiveness_label = Label.new()
	effectiveness_label.text = "효과성: 계산 중..."
	feedback_container.add_child(effectiveness_label)

	# Squad suitability
	suitability_label = Label.new()
	suitability_label.text = "스쿼드 적합도: 계산 중..."
	feedback_container.add_child(suitability_label)

	# Advantages section
	var advantages_title = Label.new()
	advantages_title.text = "장점:"
	advantages_title.add_theme_font_size_override("font_size", 14)
	feedback_container.add_child(advantages_title)

	advantages_label = RichTextLabel.new()
	advantages_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
	advantages_label.bbcode_enabled = true
	feedback_container.add_child(advantages_label)

	# Disadvantages section
	var disadvantages_title = Label.new()
	disadvantages_title.text = "단점:"
	disadvantages_title.add_theme_font_size_override("font_size", 14)
	feedback_container.add_child(disadvantages_title)

	disadvantages_label = RichTextLabel.new()
	disadvantages_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
	disadvantages_label.bbcode_enabled = true
	feedback_container.add_child(disadvantages_label)

	# Action buttons
	var button_row = HBoxContainer.new()
	feedback_container.add_child(button_row)

	change_button = Button.new()
	change_button.text = "포메이션 변경"
	change_button.pressed.connect(_on_change_button_pressed)
	button_row.add_child(change_button)

	cancel_button = Button.new()
	cancel_button.text = "취소"
	cancel_button.pressed.connect(_on_cancel_button_pressed)
	button_row.add_child(cancel_button)


func _update_current_formation():
	"""Update current formation from TacticalManager"""
	if tactical_manager and tactical_manager.has_method("get_current_formation"):
		current_formation = tactical_manager.get_current_formation()
		_update_button_styles()


func _update_button_styles():
	"""Update all button styles based on current state"""
	for formation_id in formation_buttons:
		var button = formation_buttons[formation_id]
		_style_formation_button(button, formation_id)


func _is_formation_available(formation_id: String) -> bool:
	"""Check if formation is available based on coach knowledge"""
	if not coach_system:
		return true  # No restrictions if no coach system

	if coach_system.has_method("get_tactical_knowledge"):
		var tactical_knowledge = coach_system.get_tactical_knowledge()
		var coach_formation_name = _convert_formation_id_to_coach_format(formation_id)
		return tactical_knowledge.get(coach_formation_name, 0) > 0

	return true


func _convert_formation_id_to_coach_format(formation_id: String) -> String:
	"""Convert formation ID to coach system format"""
	match formation_id:
		"442_standard":
			return "4-4-2"
		"433_standard":
			return "4-3-3"
		"352_standard":
			return "3-5-2"
		_:
			return "4-4-2"


# ==============================================================================
# FR-018: Immediate Formation Feedback Implementation
# ==============================================================================


func _show_formation_feedback(formation_id: String, is_preview: bool = true):
	"""Show immediate feedback for formation selection (FR-018)"""
	feedback_panel.visible = true

	var info = formation_info.get(formation_id, {})
	# Show Korean name with English fallback
	var display_name = info.get("name_ko", info.get("name", formation_id))
	feedback_title.text = "%s %s" % [display_name, "(미리보기)" if is_preview else ""]

	# Calculate and display effectiveness
	_update_effectiveness_feedback(formation_id)

	# Display formation advantages
	var advantages_text = ""
	for advantage in info.get("advantages", []):
		advantages_text += "• " + advantage + "\n"
	advantages_label.text = "[color=green]" + advantages_text + "[/color]"

	# Display formation disadvantages
	var disadvantages_text = ""
	for disadvantage in info.get("disadvantages", []):
		disadvantages_text += "• " + disadvantage + "\n"
	disadvantages_label.text = "[color=red]" + disadvantages_text + "[/color]"

	# Update action buttons
	change_button.disabled = (formation_id == current_formation) or not _is_formation_available(formation_id)
	change_button.text = "포메이션 변경" if formation_id != current_formation else "현재 포메이션"

	# Store preview formation
	if is_preview:
		preview_formation = formation_id
		formation_preview_started.emit(formation_id)

		# Update FormationDisplay for preview
		if formation_display and formation_display.has_method("set_formation"):
			formation_display.set_formation(formation_id)


func _update_effectiveness_feedback(formation_id: String):
	"""Update effectiveness and suitability feedback"""
	if not tactical_manager:
		effectiveness_label.text = "효과성: 계산 불가 (TacticalManager 없음)"
		suitability_label.text = "적합도: 계산 불가"
		return

	# Get formation suitability
	var suitability = 0.0
	if tactical_manager.has_method("get_formation_suitability"):
		suitability = tactical_manager.get_formation_suitability(formation_id)

	# Get current effectiveness (simplified for preview)
	var effectiveness = 0.5  # Default
	if tactical_manager.has_method("get_formation_effectiveness"):
		var effectiveness_data = tactical_manager.get_formation_effectiveness()
		effectiveness = effectiveness_data.get("overall_effectiveness", 0.5)

	# Display with color coding
	var effectiveness_color = _get_effectiveness_color(effectiveness)
	var suitability_color = _get_effectiveness_color(suitability)

	effectiveness_label.text = "효과성: %.1f%%" % (effectiveness * 100)
	effectiveness_label.add_theme_color_override("font_color", effectiveness_color)

	suitability_label.text = "스쿼드 적합도: %.1f%%" % (suitability * 100)
	suitability_label.add_theme_color_override("font_color", suitability_color)


func _get_effectiveness_color(value: float) -> Color:
	"""Get color based on effectiveness value"""
	if value < 0.4:
		return Color.RED
	elif value < 0.7:
		return Color.YELLOW
	else:
		return Color.GREEN


func _hide_formation_feedback():
	"""Hide formation feedback panel"""
	feedback_panel.visible = false
	preview_formation = ""
	formation_preview_ended.emit()

	# Restore FormationDisplay to current formation
	if formation_display and formation_display.has_method("set_formation"):
		formation_display.set_formation(current_formation)


# ==============================================================================
# Signal Handlers
# ==============================================================================


func _on_formation_button_pressed(formation_id: String):
	"""Handle formation button press"""
	print("[FormationSelector] Formation button pressed: %s" % formation_id)

	selected_formation = formation_id
	_show_formation_feedback(formation_id, false)

	# Emit formation_selected signal immediately for direct selection
	var feedback_data = {"formation_id": formation_id, "timestamp": Time.get_unix_time_from_system()}
	formation_selected.emit(formation_id, true, feedback_data)


func _on_formation_button_hover(formation_id: String):
	"""Handle formation button hover for preview"""
	if not is_previewing and formation_id != selected_formation:
		is_previewing = true
		_show_formation_feedback(formation_id, true)


func _on_formation_button_unhover(formation_id: String):
	"""Handle formation button unhover"""
	if is_previewing and preview_formation == formation_id:
		is_previewing = false
		if selected_formation.is_empty():
			_hide_formation_feedback()
		else:
			_show_formation_feedback(selected_formation, false)


func _on_change_button_pressed():
	"""Handle formation change confirmation"""
	if selected_formation.is_empty():
		return

	print("[FormationSelector] Requesting formation change: %s" % selected_formation)

	if tactical_manager and tactical_manager.has_method("request_formation_change"):
		var result = tactical_manager.request_formation_change(selected_formation, "manager")

		var feedback_data = {
			"formation_id": selected_formation, "result": result, "timestamp": Time.get_unix_time_from_system()
		}

		formation_selected.emit(selected_formation, result.get("success", false), feedback_data)

		if result.get("success", false):
			print("[FormationSelector] Formation change successful!")
			_hide_formation_feedback()
		else:
			print("[FormationSelector] Formation change failed: %s" % result.get("message", "Unknown error"))
			# Show error message in feedback
			effectiveness_label.text = "오류: " + result.get("message", "알 수 없는 오류")
			effectiveness_label.add_theme_color_override("font_color", Color.RED)


func _on_cancel_button_pressed():
	"""Handle formation change cancellation"""
	selected_formation = ""
	_hide_formation_feedback()


func _on_formation_changed(formation_id: String, effectiveness: float):
	"""Handle formation change from TacticalManager"""
	current_formation = formation_id
	selected_formation = ""
	_update_button_styles()
	_hide_formation_feedback()

	print("[FormationSelector] Formation changed to: %s (%.2f)" % [formation_id, effectiveness])


func _on_authority_confirmed(action: String, allowed: bool):
	"""Handle manager authority confirmation"""
	if action == "formation_change":
		if not allowed:
			effectiveness_label.text = "권한 없음: 매니저만 포메이션을 변경할 수 있습니다."
			effectiveness_label.add_theme_color_override("font_color", Color.RED)
			change_button.disabled = true


# ==============================================================================
# Public API
# ==============================================================================


func set_formation_display_reference(display_control: Control):
	"""Set reference to FormationDisplay component"""
	formation_display = display_control
	print("[FormationSelector] FormationDisplay reference set")


func get_available_formations() -> Array:
	"""Get list of available formations"""
	return available_formations


func get_formation_info(formation_id: String) -> Dictionary:
	"""Get formation information"""
	return formation_info.get(formation_id, {})


func refresh_formation_availability():
	"""Refresh formation availability based on current coach knowledge"""
	_update_button_styles()


# ==============================================================================
# Testing
# ==============================================================================


func test_formation_selector():
	"""Test the formation selector component"""
	print("=== FormationSelector Test ===")

	# Test button hover
	for formation_id in available_formations:
		print("Testing hover for: %s" % formation_id)
		_on_formation_button_hover(formation_id)
		await get_tree().create_timer(0.5).timeout
		_on_formation_button_unhover(formation_id)

	# Test formation selection
	print("Testing formation selection...")
	_on_formation_button_pressed("433_standard")

	print("✅ FormationSelector test completed")
