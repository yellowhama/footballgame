extends Control

# Formation Display Component - Visual representation of football formations
# Implements FR-017: Visual feedback for formation selection
# Integrates with TacticalManager for real-time updates

signal player_position_clicked(player_index: int, position_data: Dictionary)
signal formation_feedback_requested(formation_id: String)

# Formation display settings
@export var field_color: Color = Color(0.2, 0.8, 0.2, 1.0)  # Football field green
@export var line_color: Color = Color.WHITE
@export var player_size: float = 30.0
@export var show_player_names: bool = false
@export var show_effectiveness_colors: bool = true

# Player position nodes
var player_positions: Array[Control] = []
var field_background: ColorRect
var effectiveness_display: Label

# Integration with TacticalManager
var tactical_manager: Node = null

# Current formation data
var current_formation_id: String = ""
var current_positions: Array = []
var current_effectiveness: Dictionary = {}

# Player position display data
var position_effectiveness: Array[float] = []


func _ready():
	print("[FormationDisplay] Initializing formation display component")

	# Connect to TacticalManager
	_connect_to_tactical_manager()

	# Setup UI components
	_setup_field_display()
	_setup_player_positions()
	_setup_effectiveness_display()

	# Load initial formation (fallback if no formation set externally)
	if current_formation_id == "":
		set_formation("T442")  # Default formation

	print("[FormationDisplay] Formation display ready")


func _connect_to_tactical_manager():
	"""Connect to TacticalManager for real-time updates"""
	tactical_manager = get_node_or_null("/root/TacticalManager")
	if not tactical_manager:
		tactical_manager = get_tree().get_first_node_in_group("tactical_manager")

	if tactical_manager:
		# Connect to signals
		if tactical_manager.has_signal("formation_changed"):
			tactical_manager.formation_changed.connect(_on_formation_changed)
		if tactical_manager.has_signal("tactical_effectiveness_updated"):
			tactical_manager.tactical_effectiveness_updated.connect(_on_effectiveness_updated)

		print("[FormationDisplay] Connected to TacticalManager")
	else:
		print("[FormationDisplay] Warning: TacticalManager not found")


func _setup_field_display():
	"""Setup the football field background"""
	field_background = ColorRect.new()
	field_background.color = field_color
	field_background.set_anchors_preset(Control.PRESET_FULL_RECT)
	field_background.anchor_right = 1.0
	field_background.anchor_bottom = 1.0
	field_background.grow_horizontal = Control.GROW_DIRECTION_BOTH
	field_background.grow_vertical = Control.GROW_DIRECTION_BOTH
	add_child(field_background)

	# Connect to resized signal to update player positions
	resized.connect(_on_resized)

	# Add field lines overlay
	_draw_field_lines()


func _draw_field_lines():
	"""Draw football field lines - simplified version"""
	# Note: Drawing is disabled for now to avoid complexity
	# Field lines can be added later using a dedicated CanvasItem node
	pass


func _setup_player_positions():
	"""Setup player position indicators"""
	player_positions.clear()

	for i in range(11):  # 11 players
		var player_node = _create_player_position_node(i)
		add_child(player_node)
		player_positions.append(player_node)


func _create_player_position_node(player_index: int) -> Control:
	"""Create a single player position display node"""
	var player_container = Control.new()
	player_container.size = Vector2(player_size, player_size)

	# Player circle background
	var player_circle = ColorRect.new()
	player_circle.size = Vector2(player_size, player_size)
	player_circle.color = Color.BLUE
	player_circle.position = Vector2.ZERO
	player_container.add_child(player_circle)

	# Player number/role label
	var player_label = Label.new()
	player_label.text = str(player_index + 1)
	player_label.size = Vector2(player_size, player_size)
	player_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	player_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	player_label.add_theme_color_override("font_color", Color.WHITE)
	player_container.add_child(player_label)

	# Make clickable
	var button = Button.new()
	button.size = Vector2(player_size, player_size)
	button.flat = true
	button.modulate = Color(1, 1, 1, 0)  # Transparent but clickable
	button.pressed.connect(_on_player_position_clicked.bind(player_index))
	player_container.add_child(button)

	return player_container


func _setup_effectiveness_display():
	"""Setup effectiveness display at the top"""
	effectiveness_display = Label.new()
	effectiveness_display.text = "포메이션 효과성: 계산 중..."
	effectiveness_display.position = Vector2(10, 10)
	effectiveness_display.add_theme_color_override("font_color", Color.WHITE)
	effectiveness_display.add_theme_color_override("font_shadow_color", Color.BLACK)
	effectiveness_display.add_theme_constant_override("shadow_offset_x", 1)
	effectiveness_display.add_theme_constant_override("shadow_offset_y", 1)
	add_child(effectiveness_display)


func _update_formation_display():
	"""Update the formation display with current data"""
	if not tactical_manager:
		return

	# Get current formation
	var formation_id = ""
	if tactical_manager.has_method("get_current_formation"):
		formation_id = tactical_manager.get_current_formation()

	if formation_id == current_formation_id:
		return  # No change needed

	current_formation_id = formation_id
	print("[FormationDisplay] Updating formation display: %s" % formation_id)

	# Get formation positions from TacticalManager
	_load_formation_positions(formation_id)

	# Update player positions
	_update_player_positions()

	# Update effectiveness display
	_update_effectiveness_display()


func _load_formation_positions(formation_id: String):
	"""Load formation positions from OpenFootball/Rust backend"""
	# Get formation details from Rust engine via FootballRustEngine
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine:
		print("[FormationDisplay] Warning: FootballRustEngine not found, using fallback")
		current_positions = _get_fallback_positions()
		return

	if not rust_engine.is_ready():
		print("[FormationDisplay] Warning: FootballRustEngine not ready, using fallback")
		current_positions = _get_fallback_positions()
		return

	# Call Formation API
	var result = rust_engine.get_formation_details(formation_id)

	if not result.get("success", false):
		print("[FormationDisplay] Error loading formation: %s" % result.get("error", "Unknown"))
		current_positions = _get_fallback_positions()
		return

	# Extract positions from formation data
	var formation = result.get("formation", {})
	var positions = formation.get("positions", [])

	if positions.size() != 11:
		print("[FormationDisplay] Warning: Expected 11 positions, got %d" % positions.size())
		current_positions = _get_fallback_positions()
		return

	# Convert OpenFootball positions to our format
	current_positions.clear()
	for pos in positions:
		current_positions.append(
			{
				"x": pos.get("x", 0.5),
				"y": pos.get("y", 0.5),
				"role": pos.get("position_type", ""),
				"slot": pos.get("slot", 0),
				"name_ko": pos.get("position_name_ko", "")
			}
		)

	print("[FormationDisplay] Loaded %d positions for %s" % [current_positions.size(), formation_id])


func _get_fallback_positions() -> Array:
	"""Get fallback 4-4-2 formation positions when API fails"""
	return [
		{"x": 0.5, "y": 0.1, "role": "GK", "slot": 0},
		{"x": 0.2, "y": 0.25, "role": "LB", "slot": 1},
		{"x": 0.35, "y": 0.2, "role": "CB", "slot": 2},
		{"x": 0.65, "y": 0.2, "role": "CB", "slot": 3},
		{"x": 0.8, "y": 0.25, "role": "RB", "slot": 4},
		{"x": 0.2, "y": 0.55, "role": "LM", "slot": 5},
		{"x": 0.35, "y": 0.5, "role": "CM", "slot": 6},
		{"x": 0.65, "y": 0.5, "role": "CM", "slot": 7},
		{"x": 0.8, "y": 0.55, "role": "RM", "slot": 8},
		{"x": 0.4, "y": 0.8, "role": "ST", "slot": 9},
		{"x": 0.6, "y": 0.8, "role": "ST", "slot": 10}
	]


func _on_resized():
	"""Handle Control resize - update player positions"""
	_update_player_positions()


func _update_player_positions():
	"""Update visual positions of all players"""
	# Wait for layout to be calculated
	if size.x == 0 or size.y == 0:
		return

	for i in range(min(player_positions.size(), current_positions.size())):
		var player_node = player_positions[i]
		var position_data = current_positions[i]

		# Calculate screen position
		# Flip Y: GK (y=0.1) → screen bottom, ST (y=0.8) → screen top
		var screen_x = position_data.x * size.x - player_size * 0.5
		var screen_y = (1.0 - position_data.y) * size.y - player_size * 0.5

		player_node.position = Vector2(screen_x, screen_y)

		# Update player label with role (prefer Korean name)
		var label = player_node.get_child(1) as Label
		if show_player_names:
			if position_data.has("name_ko") and position_data.name_ko != "":
				label.text = position_data.name_ko
			elif position_data.has("role"):
				label.text = position_data.role
			else:
				label.text = str(i + 1)
		else:
			label.text = str(i + 1)

		# Update effectiveness color if enabled
		if show_effectiveness_colors and i < position_effectiveness.size():
			_update_player_effectiveness_color(player_node, position_effectiveness[i])


func _update_player_effectiveness_color(player_node: Control, effectiveness: float):
	"""Update player circle color based on effectiveness"""
	var circle = player_node.get_child(0) as ColorRect

	# Color based on effectiveness (0.0 = red, 0.5 = yellow, 1.0 = green)
	var color: Color
	if effectiveness < 0.5:
		# Red to Yellow
		color = Color.RED.lerp(Color.YELLOW, effectiveness * 2.0)
	else:
		# Yellow to Green
		color = Color.YELLOW.lerp(Color.GREEN, (effectiveness - 0.5) * 2.0)

	circle.color = color


func _update_effectiveness_display():
	"""Update the effectiveness text display"""
	if not tactical_manager:
		effectiveness_display.text = "포메이션 효과성: TacticalManager 없음"
		return

	# Get current effectiveness
	var effectiveness_data = {}
	if tactical_manager.has_method("get_formation_effectiveness"):
		effectiveness_data = tactical_manager.get_formation_effectiveness()

	var overall = effectiveness_data.get("overall_effectiveness", 0.0)
	var attacking = effectiveness_data.get("attacking_effectiveness", 0.0)
	var defensive = effectiveness_data.get("defensive_effectiveness", 0.0)

	effectiveness_display.text = (
		"포메이션 효과성: %.1f%% | 공격: %.1f%% | 수비: %.1f%%" % [overall * 100, attacking * 100, defensive * 100]
	)

	# Color based on overall effectiveness
	var color: Color
	if overall < 0.4:
		color = Color.RED
	elif overall < 0.7:
		color = Color.YELLOW
	else:
		color = Color.GREEN

	effectiveness_display.add_theme_color_override("font_color", color)


# ==============================================================================
# Signal Handlers
# ==============================================================================


func _on_formation_changed(formation_id: String, effectiveness: float):
	"""Handle formation change from TacticalManager"""
	print("[FormationDisplay] Formation changed: %s (%.2f)" % [formation_id, effectiveness])
	_update_formation_display()


func _on_effectiveness_updated(effectiveness_data: Dictionary):
	"""Handle effectiveness update from TacticalManager"""
	current_effectiveness = effectiveness_data

	# Update player adaptation rates if available
	var adaptations = effectiveness_data.get("player_adaptations", {})
	position_effectiveness.clear()

	for i in range(11):
		var player_id = "P%d" % i
		var adaptation = adaptations.get(player_id, 0.5)
		position_effectiveness.append(adaptation)

	_update_effectiveness_display()

	if show_effectiveness_colors:
		_update_player_positions()  # Refresh colors


func _on_player_position_clicked(player_index: int):
	"""Handle player position click"""
	if player_index < current_positions.size():
		var position_data = current_positions[player_index]
		var effectiveness = 0.5
		if player_index < position_effectiveness.size():
			effectiveness = position_effectiveness[player_index]

		position_data["effectiveness"] = effectiveness
		position_data["player_index"] = player_index

		player_position_clicked.emit(player_index, position_data)
		print(
			"[FormationDisplay] Player position clicked: %d (%s)" % [player_index, position_data.get("role", "Unknown")]
		)


# ==============================================================================
# Public API
# ==============================================================================


func set_formation(formation_id: String):
	"""Manually set formation to display"""
	if formation_id == current_formation_id:
		return

	current_formation_id = formation_id
	print("[FormationDisplay] Setting formation: %s" % formation_id)

	# Load formation positions directly
	_load_formation_positions(formation_id)

	# Update player positions
	_update_player_positions()

	# Update effectiveness display (will show N/A if no TacticalManager)
	_update_effectiveness_display()


func get_current_formation() -> String:
	"""Get currently displayed formation"""
	return current_formation_id


func set_show_player_names(show: bool):
	"""Toggle player name display"""
	show_player_names = show
	_update_player_positions()


func set_show_effectiveness_colors(show: bool):
	"""Toggle effectiveness color display"""
	show_effectiveness_colors = show
	_update_player_positions()


func get_formation_effectiveness() -> Dictionary:
	"""Get current formation effectiveness data"""
	return current_effectiveness


func request_formation_feedback():
	"""Request immediate formation feedback update"""
	formation_feedback_requested.emit(current_formation_id)
	_update_formation_display()


# ==============================================================================
# Testing
# ==============================================================================


func test_formation_display():
	"""Test the formation display component"""
	print("=== FormationDisplay Test ===")

	# Test different formations
	var test_formations = ["442_standard", "433_standard", "352_standard"]

	for formation in test_formations:
		print("Testing formation: %s" % formation)
		set_formation(formation)
		await get_tree().create_timer(1.0).timeout

	# Test effectiveness colors
	print("Testing effectiveness colors...")
	set_show_effectiveness_colors(true)

	# Test player names
	print("Testing player names...")
	set_show_player_names(true)

	print("✅ FormationDisplay test completed")
