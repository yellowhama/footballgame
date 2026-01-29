extends Control

## MyTeamSetupScreen: Team customization screen
## Allows user to set team name, nickname, emblem composition, and colors
## Now includes Socceralia team preview background with animated players
##
## 참조: assets/ui/2025-12-08_CHARACTER_SPRITE_USAGE_FOR_UI.md

signal closed

## Socceralia 팀 미리보기 사용 여부
@export var use_team_preview: bool = true

# UI References
@onready var team_name_edit = $ScrollContainer/VBoxContainer/TeamNameEdit
@onready var nickname_edit = $ScrollContainer/VBoxContainer/NicknameEdit
@onready var primary_color_picker = $ScrollContainer/VBoxContainer/ColorsContainer/PrimaryContainer/PrimaryColorPicker
@onready
var secondary_color_picker = $ScrollContainer/VBoxContainer/ColorsContainer/SecondaryContainer/SecondaryColorPicker
@onready var emblem_preview = $ScrollContainer/VBoxContainer/PreviewPanel/CenterContainer/EmblemPreview
@onready var preview_panel = $ScrollContainer/VBoxContainer/PreviewPanel

# Background buttons
@onready var background_grid = $ScrollContainer/VBoxContainer/BackgroundGrid

# Icon buttons
@onready var icon_grid = $ScrollContainer/VBoxContainer/IconGrid

# Action buttons
@onready var confirm_button = $ScrollContainer/VBoxContainer/ButtonContainer/ConfirmButton
@onready var cancel_button = $ScrollContainer/VBoxContainer/ButtonContainer/CancelButton

# Team Preview (Socceralia)
var team_preview_container: Control = null
var team_preview_background: Control = null
var pattern_buttons: Dictionary = {}

# Current selection
var current_team_name: String = "My Academy"
var current_nickname: String = "Youngsters"
var current_icon: int = 0
var current_background: int = 0
var current_primary: Color = Color.RED
var current_secondary: Color = Color.WHITE
var current_pattern: int = 0

# MyTeamData reference
var my_team_data: Node = null

# Pattern types
const PATTERN_TYPES = {
	0: {"name": "단색", "icon": "━"},
	1: {"name": "가로줄", "icon": "═"},
	2: {"name": "세로줄", "icon": "║"},
	3: {"name": "체크", "icon": "▦"}
}


func _ready():
	# Get MyTeamData singleton
	my_team_data = get_node_or_null("/root/MyTeamData")
	if not my_team_data:
		print("[MyTeamSetupScreen] ⚠️ MyTeamData not found!")
		return

	# Load current settings
	_load_current_settings()

	# Setup team preview (Socceralia)
	if use_team_preview:
		_setup_team_preview()

	# Update UI
	_update_ui()

	# Wire up UI signals
	_connect_ui_signals()

	# Initial preview update
	_update_preview()


func _setup_team_preview():
	"""Setup Socceralia team preview background with animated players"""
	if not preview_panel:
		return

	# PreviewContainer 씬 로드
	var preview_container_scene = load("res://scenes/ui/components/PreviewContainer.tscn")
	if not preview_container_scene:
		push_warning("[MyTeamSetupScreen] PreviewContainer scene not found")
		return

	# 기존 PreviewPanel에 팀 미리보기 추가 (엠블럼 위에)
	team_preview_container = preview_container_scene.instantiate()
	team_preview_container.name = "TeamPreviewContainer"
	team_preview_container.custom_minimum_size = Vector2(0, 300)

	# PreviewPanel 상단에 삽입
	var scroll_vbox = $ScrollContainer/VBoxContainer
	if scroll_vbox:
		# PreviewPanel 앞에 삽입
		var preview_idx = preview_panel.get_index()
		scroll_vbox.add_child(team_preview_container)
		scroll_vbox.move_child(team_preview_container, preview_idx)

	# 팀 미리보기 (8명의 선수)
	await get_tree().process_frame  # 노드가 준비될 때까지 대기
	team_preview_background = team_preview_container.add_team_preview(8)

	if team_preview_background:
		team_preview_background.set_team_colors(current_primary, current_secondary, current_pattern)
		print("[MyTeamSetupScreen] Team preview created with 8 players")

	# 패턴 선택 UI 추가
	_setup_pattern_selector()


func _setup_pattern_selector():
	"""패턴 선택 버튼 추가"""
	var colors_container = $ScrollContainer/VBoxContainer/ColorsContainer
	if not colors_container:
		return

	# 패턴 컨테이너
	var pattern_container = VBoxContainer.new()
	pattern_container.name = "PatternContainer"

	var pattern_label = Label.new()
	pattern_label.text = "유니폼 패턴"
	pattern_label.add_theme_font_size_override("font_size", 18)
	pattern_container.add_child(pattern_label)

	var pattern_grid = GridContainer.new()
	pattern_grid.name = "PatternGrid"
	pattern_grid.columns = 4
	pattern_container.add_child(pattern_grid)

	for pattern_id in PATTERN_TYPES:
		var data = PATTERN_TYPES[pattern_id]
		var btn = Button.new()
		btn.text = "%s %s" % [data.icon, data.name]
		btn.custom_minimum_size = Vector2(120, 40)
		btn.toggle_mode = true
		btn.button_pressed = (pattern_id == current_pattern)
		btn.pressed.connect(_on_pattern_selected.bind(pattern_id))
		pattern_grid.add_child(btn)
		pattern_buttons[pattern_id] = btn

	# ColorsContainer 다음에 추가
	var parent = colors_container.get_parent()
	var idx = colors_container.get_index()
	parent.add_child(pattern_container)
	parent.move_child(pattern_container, idx + 1)


func _connect_ui_signals():
	"""Connect all UI element signals"""
	# Text edits
	if team_name_edit:
		team_name_edit.text_changed.connect(_on_team_name_changed)

	if nickname_edit:
		nickname_edit.text_changed.connect(_on_nickname_changed)

	# Background buttons
	if background_grid:
		for i in range(background_grid.get_child_count()):
			var button = background_grid.get_child(i)
			if button is Button:
				button.pressed.connect(_on_background_selected.bind(i))

	# Icon buttons
	if icon_grid:
		for i in range(icon_grid.get_child_count()):
			var button = icon_grid.get_child(i)
			if button is Button:
				button.pressed.connect(_on_icon_selected.bind(i))

	# Color pickers
	if primary_color_picker:
		primary_color_picker.color_changed.connect(_on_primary_color_changed)

	if secondary_color_picker:
		secondary_color_picker.color_changed.connect(_on_secondary_color_changed)

	# Action buttons
	if confirm_button:
		confirm_button.pressed.connect(_on_confirm_pressed)

	if cancel_button:
		cancel_button.pressed.connect(_on_cancel_pressed)


func _load_current_settings():
	"""Load current academy settings from MyTeamData"""
	if not my_team_data:
		return

	var settings = my_team_data.academy_settings

	current_team_name = settings.get("team_name", "My Academy")
	current_nickname = settings.get("nickname", "Youngsters")
	current_icon = settings.get("emblem_icon", 0)
	current_background = settings.get("emblem_background", 0)

	# Parse color strings
	var primary_str = settings.get("primary_color", "#FF0000")
	var secondary_str = settings.get("secondary_color", "#FFFFFF")

	current_primary = Color(primary_str)
	current_secondary = Color(secondary_str)

	# Load pattern if available
	var uniform = settings.get("uniform", {})
	if uniform.has("home"):
		current_pattern = uniform.home.get("pattern_type", 0)


func _update_ui():
	"""Update UI elements with current settings"""
	team_name_edit.text = current_team_name
	nickname_edit.text = current_nickname
	primary_color_picker.color = current_primary
	secondary_color_picker.color = current_secondary

	# Highlight selected background button
	_highlight_background_button(current_background)

	# Highlight selected icon button
	_highlight_icon_button(current_icon)

	# Update pattern buttons
	_update_pattern_buttons()


func _highlight_background_button(bg_id: int):
	"""Highlight the selected background button"""
	for i in range(background_grid.get_child_count()):
		var button = background_grid.get_child(i)
		if button is Button:
			if i == bg_id:
				button.modulate = Color(1.5, 1.5, 1.0)  # Highlight
			else:
				button.modulate = Color(1, 1, 1)  # Normal


func _highlight_icon_button(icon_id: int):
	"""Highlight the selected icon button"""
	for i in range(icon_grid.get_child_count()):
		var button = icon_grid.get_child(i)
		if button is Button:
			if i == icon_id:
				button.modulate = Color(1.5, 1.5, 1.0)  # Highlight
			else:
				button.modulate = Color(1, 1, 1)  # Normal


func _update_pattern_buttons():
	"""Update pattern button states"""
	for pattern_id in pattern_buttons:
		pattern_buttons[pattern_id].button_pressed = (pattern_id == current_pattern)


func _update_preview():
	"""Update emblem preview with current composition"""
	if emblem_preview:
		emblem_preview.set_emblem(current_icon, current_background, current_primary, current_secondary)

	# Update team preview background
	_update_team_preview()


func _update_team_preview():
	"""Update Socceralia team preview with current colors"""
	if team_preview_background and team_preview_background.has_method("set_team_colors"):
		team_preview_background.set_team_colors(current_primary, current_secondary, current_pattern)


# Signal Handlers


func _on_team_name_changed(new_text: String):
	"""Handle team name text change"""
	current_team_name = new_text
	print("[MyTeamSetupScreen] Team name: %s" % new_text)


func _on_nickname_changed(new_text: String):
	"""Handle nickname text change"""
	current_nickname = new_text
	print("[MyTeamSetupScreen] Nickname: %s" % new_text)


func _on_background_selected(bg_id: int):
	"""Handle background shape selection"""
	current_background = bg_id
	_highlight_background_button(bg_id)
	_update_preview()
	print("[MyTeamSetupScreen] Background selected: %d" % bg_id)


func _on_icon_selected(icon_id: int):
	"""Handle icon symbol selection"""
	current_icon = icon_id
	_highlight_icon_button(icon_id)
	_update_preview()
	print("[MyTeamSetupScreen] Icon selected: %d" % icon_id)


func _on_primary_color_changed(color: Color):
	"""Handle primary color change"""
	current_primary = color
	_update_preview()
	print("[MyTeamSetupScreen] Primary color: %s" % color.to_html())


func _on_secondary_color_changed(color: Color):
	"""Handle secondary color change"""
	current_secondary = color
	_update_preview()
	print("[MyTeamSetupScreen] Secondary color: %s" % color.to_html())


func _on_pattern_selected(pattern_id: int):
	"""Handle pattern selection"""
	current_pattern = pattern_id
	_update_pattern_buttons()
	_update_team_preview()
	print("[MyTeamSetupScreen] Pattern selected: %d (%s)" % [pattern_id, PATTERN_TYPES[pattern_id].name])


func _on_confirm_pressed():
	"""Validate and save settings"""
	if not my_team_data:
		print("[MyTeamSetupScreen] ⚠️ Cannot save: MyTeamData not found")
		return

	# Build settings dictionary
	var new_settings = {
		"team_name": current_team_name,
		"nickname": current_nickname,
		"emblem_icon": current_icon,
		"emblem_background": current_background,
		"primary_color": current_primary.to_html(),
		"secondary_color": current_secondary.to_html(),
		# Add uniform with pattern
		"uniform":
		{
			"home":
			{
				"primary": current_primary.to_html(),
				"secondary": current_secondary.to_html(),
				"pattern_type": current_pattern
			},
			"away": {"primary": current_secondary.to_html(), "secondary": current_primary.to_html(), "pattern_type": 0}
		}
	}

	# Validate using MyTeamData
	var validated = my_team_data.validate_academy_settings(new_settings)

	# Apply validated settings
	my_team_data.academy_settings = validated
	my_team_data.save_to_file()

	print("[MyTeamSetupScreen] ✅ Settings saved successfully!")
	print("   - Team: %s" % validated["team_name"])
	print("   - Nickname: %s" % validated["nickname"])
	print("   - Emblem: Icon %d, Background %d" % [validated["emblem_icon"], validated["emblem_background"]])
	print("   - Colors: %s / %s" % [validated["primary_color"], validated["secondary_color"]])
	print("   - Pattern: %d" % current_pattern)

	# Emit closed signal before removing
	closed.emit()
	queue_free()


func _on_cancel_pressed():
	"""Cancel and close without saving"""
	print("[MyTeamSetupScreen] ❌ Cancelled")
	# Emit closed signal before removing
	closed.emit()
	queue_free()
