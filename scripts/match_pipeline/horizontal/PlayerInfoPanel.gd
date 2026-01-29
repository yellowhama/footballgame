## PlayerInfoPanel.gd
## P2.2: Session Match Player Info Panel
##
## Displays selected player information during match playback:
## - Basic info: name, position, overall rating
## - Current state: action, stamina bar
##
## Style: Glassmorphism (dark bg, blue border, 85% opacity)
## Position: Top-right corner, 200x120px

extends PanelContainer

## Cached rosters for overall rating lookup
var _rosters: Dictionary = {}

## UI References
var _name_label: Label
var _position_label: Label
var _overall_label: Label
var _action_label: Label
var _stamina_bar: ProgressBar

## Current player being displayed
var _current_track_id: int = -1

## Constants
const PANEL_WIDTH := 200
const PANEL_HEIGHT := 120
const MARGIN := 10

## Colors
const BG_COLOR := Color(0.1, 0.1, 0.1, 0.85)
const BORDER_COLOR := Color(0.4, 0.7, 1.0, 0.6)
const STAMINA_HIGH := Color(0.2, 0.8, 0.2)  # Green >70%
const STAMINA_MID := Color(0.9, 0.9, 0.2)  # Yellow 30-70%
const STAMINA_LOW := Color(0.9, 0.2, 0.2)  # Red <30%


func _init() -> void:
	name = "PlayerInfoPanel"


func _ready() -> void:
	_setup_panel_style()
	_create_ui_elements()
	visible = false


## Setup glassmorphism panel style
func _setup_panel_style() -> void:
	# Size and position
	custom_minimum_size = Vector2(PANEL_WIDTH, PANEL_HEIGHT)
	size = Vector2(PANEL_WIDTH, PANEL_HEIGHT)

	# Anchors: top-right corner
	anchor_left = 1.0
	anchor_top = 0.0
	anchor_right = 1.0
	anchor_bottom = 0.0

	offset_left = -PANEL_WIDTH - MARGIN
	offset_top = MARGIN
	offset_right = -MARGIN
	offset_bottom = MARGIN + PANEL_HEIGHT

	# Glassmorphism style
	var style_box := StyleBoxFlat.new()
	style_box.bg_color = BG_COLOR
	style_box.border_color = BORDER_COLOR
	style_box.set_border_width_all(2)
	style_box.corner_radius_top_left = 8
	style_box.corner_radius_top_right = 8
	style_box.corner_radius_bottom_left = 8
	style_box.corner_radius_bottom_right = 8

	add_theme_stylebox_override("panel", style_box)


## Create UI elements
func _create_ui_elements() -> void:
	var vbox := VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 4)
	add_child(vbox)

	# Name (16px, bold)
	_name_label = Label.new()
	_name_label.text = "Player Name"
	_name_label.add_theme_font_size_override("font_size", 16)
	_name_label.add_theme_color_override("font_color", Color.WHITE)
	vbox.add_child(_name_label)

	# Position + Overall (horizontal)
	var pos_ovr_hbox := HBoxContainer.new()
	pos_ovr_hbox.add_theme_constant_override("separation", 8)
	vbox.add_child(pos_ovr_hbox)

	_position_label = Label.new()
	_position_label.text = "CM"
	_position_label.add_theme_font_size_override("font_size", 12)
	_position_label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	pos_ovr_hbox.add_child(_position_label)

	_overall_label = Label.new()
	_overall_label.text = "OVR: --"
	_overall_label.add_theme_font_size_override("font_size", 12)
	_overall_label.add_theme_color_override("font_color", Color(1.0, 0.9, 0.3))
	pos_ovr_hbox.add_child(_overall_label)

	# Action
	_action_label = Label.new()
	_action_label.text = "Action: Idle"
	_action_label.add_theme_font_size_override("font_size", 11)
	_action_label.add_theme_color_override("font_color", Color(0.7, 0.9, 1.0))
	vbox.add_child(_action_label)

	# Stamina bar
	var stamina_container := VBoxContainer.new()
	stamina_container.add_theme_constant_override("separation", 2)
	vbox.add_child(stamina_container)

	var stamina_title := Label.new()
	stamina_title.text = "Stamina"
	stamina_title.add_theme_font_size_override("font_size", 10)
	stamina_title.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	stamina_container.add_child(stamina_title)

	_stamina_bar = ProgressBar.new()
	_stamina_bar.min_value = 0.0
	_stamina_bar.max_value = 100.0
	_stamina_bar.value = 100.0
	_stamina_bar.show_percentage = false
	_stamina_bar.custom_minimum_size = Vector2(180, 16)
	_update_stamina_bar_color(100.0)
	stamina_container.add_child(_stamina_bar)


## Set rosters for overall rating lookup
func set_rosters(rosters: Dictionary) -> void:
	_rosters = rosters


## Update panel from snapshot player data
func update_player_from_snapshot(player_data: Dictionary) -> void:
	if player_data.is_empty():
		clear()
		return

	var track_id: int = int(player_data.get("track_id", -1))
	var name: String = str(player_data.get("name", "Unknown"))
	var position: String = str(player_data.get("role", "CM"))
	var action: String = str(player_data.get("action", "Idle"))
	var stamina: float = float(player_data.get("stamina", 1.0)) * 100.0
	var jersey_number: int = int(player_data.get("number", 0))

	# Update labels
	if _name_label:
		_name_label.text = name

	if _position_label:
		_position_label.text = position

	if _action_label:
		_action_label.text = "Action: %s" % _format_action(action)

	# Update stamina bar
	if _stamina_bar:
		_stamina_bar.value = stamina
		_update_stamina_bar_color(stamina)

	# Lookup overall rating from rosters
	if track_id != _current_track_id:
		_current_track_id = track_id
		var overall: int = _lookup_overall_by_track_id(track_id, jersey_number)
		if _overall_label:
			if overall > 0:
				_overall_label.text = "OVR: %d" % overall
			else:
				_overall_label.text = "OVR: --"

	visible = true


## Clear panel and hide
func clear() -> void:
	_current_track_id = -1
	visible = false


## Lookup overall rating from rosters
func _lookup_overall_by_track_id(track_id: int, jersey_number: int) -> int:
	var team: String = "home" if track_id < 11 else "away"
	var roster: Array = _rosters.get(team, [])

	# First try track_id match
	for entry in roster:
		if not (entry is Dictionary):
			continue
		var entry_id = entry.get("id", -1)
		if entry_id == track_id:
			return int(entry.get("overall", entry.get("ca", 0)))

	# Fallback: jersey number match
	if jersey_number > 0:
		for entry in roster:
			if not (entry is Dictionary):
				continue
			var entry_number: int = int(entry.get("kit_number", 0))
			if entry_number == jersey_number:
				return int(entry.get("overall", entry.get("ca", 0)))

	return 0


## Format action string for display
func _format_action(action: String) -> String:
	match action.to_lower():
		"idle":
			return "Idle"
		"run", "running":
			return "Running"
		"pass", "passing":
			return "Passing"
		"shoot", "shooting":
			return "Shooting"
		"tackle", "tackling":
			return "Tackling"
		"dribble", "dribbling":
			return "Dribbling"
		"header", "heading":
			return "Heading"
		"intercept", "intercepting":
			return "Intercepting"
		_:
			return action.capitalize()


## Update stamina bar color based on percentage
func _update_stamina_bar_color(stamina_percent: float) -> void:
	if not _stamina_bar:
		return

	var style_box := StyleBoxFlat.new()

	if stamina_percent > 70.0:
		style_box.bg_color = STAMINA_HIGH
	elif stamina_percent > 30.0:
		style_box.bg_color = STAMINA_MID
	else:
		style_box.bg_color = STAMINA_LOW

	style_box.corner_radius_top_left = 4
	style_box.corner_radius_top_right = 4
	style_box.corner_radius_bottom_left = 4
	style_box.corner_radius_bottom_right = 4

	_stamina_bar.add_theme_stylebox_override("fill", style_box)
