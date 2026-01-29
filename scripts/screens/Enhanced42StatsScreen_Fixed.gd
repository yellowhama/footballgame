extends Control

# Enhanced 42-Skill Stats Screen (Fixed Version)
# Properly integrated with existing UI system

signal back_pressed

# UI References
@onready var back_button = $MainVBox/Header/BackButton
@onready var title_label = $MainVBox/Header/Title
@onready var search_input = $MainVBox/Header/SearchContainer/SearchInput
@onready var tab_container = $MainVBox/TabContainer
@onready var stats_panel = $MainVBox/StatsPanel
@onready var stats_text = $MainVBox/StatsPanel/StatsContent/StatsText

# Skill widgets storage
var skill_widgets = {}
var all_skills = []
var filtered_skills = []

# Skill categories with display names
const SKILL_CATEGORIES = {
	"Technical":
	{
		"skills":
		[
			"corners",
			"crossing",
			"dribbling",
			"finishing",
			"first_touch",
			"free_kicks",
			"heading",
			"long_shots",
			"long_throws",
			"marking",
			"passing",
			"penalty_taking",
			"tackling",
			"technique"
		],
		"display_names":
		{
			"corners": "Corners",
			"crossing": "Crossing",
			"dribbling": "Dribbling",
			"finishing": "Finishing",
			"first_touch": "First Touch",
			"free_kicks": "Free Kicks",
			"heading": "Heading",
			"long_shots": "Long Shots",
			"long_throws": "Long Throws",
			"marking": "Marking",
			"passing": "Passing",
			"penalty_taking": "Penalties",
			"tackling": "Tackling",
			"technique": "Technique"
		},
		"color": ThemeManager.PASTEL_BLUE
	},
	"Mental":
	{
		"skills":
		[
			"aggression",
			"anticipation",
			"bravery",
			"composure",
			"concentration",
			"decisions",
			"determination",
			"flair",
			"leadership",
			"off_the_ball",
			"positioning",
			"teamwork",
			"vision",
			"work_rate"
		],
		"display_names":
		{
			"aggression": "Aggression",
			"anticipation": "Anticipation",
			"bravery": "Bravery",
			"composure": "Composure",
			"concentration": "Concentration",
			"decisions": "Decisions",
			"determination": "Determination",
			"flair": "Flair",
			"leadership": "Leadership",
			"off_the_ball": "Off The Ball",
			"positioning": "Positioning",
			"teamwork": "Teamwork",
			"vision": "Vision",
			"work_rate": "Work Rate"
		},
		"color": ThemeManager.PASTEL_PURPLE
	},
	"Physical":
	{
		"skills": ["acceleration", "agility", "balance", "jumping", "natural_fitness", "pace", "stamina", "strength"],
		"display_names":
		{
			"acceleration": "Acceleration",
			"agility": "Agility",
			"balance": "Balance",
			"jumping": "Jumping",
			"natural_fitness": "Natural Fitness",
			"pace": "Pace",
			"stamina": "Stamina",
			"strength": "Strength"
		},
		"color": ThemeManager.PASTEL_GREEN
	},
	"Goalkeeper":
	{
		"skills":
		[
			"aerial_reach",
			"command_of_area",
			"communication",
			"eccentricity",
			"handling",
			"kicking",
			"one_on_ones",
			"reflexes",
			"rushing_out",
			"throwing"
		],
		"display_names":
		{
			"aerial_reach": "Aerial Reach",
			"command_of_area": "Command of Area",
			"communication": "Communication",
			"eccentricity": "Eccentricity",
			"handling": "Handling",
			"kicking": "Kicking",
			"one_on_ones": "One on Ones",
			"reflexes": "Reflexes",
			"rushing_out": "Rushing Out",
			"throwing": "Throwing"
		},
		"color": ThemeManager.PASTEL_YELLOW
	}
}


func _ready():
	print("[Enhanced42StatsScreen] Initializing with proper integration...")

	# Apply styles
	_apply_styles()

	# Setup UI
	_setup_ui()

	# Connect signals
	_connect_signals()

	# Load player data
	_load_player_data()

	# Update statistics
	_update_statistics()

	print("[Enhanced42StatsScreen] Ready!")


func _apply_styles():
	"""Apply CustomStyles and ThemeManager styles to all elements"""

	# Main container background
	var bg = ColorRect.new()
	bg.color = ThemeManager.BG_PRIMARY
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.z_index = -1
	add_child(bg)

	# Header styling
	var header = $MainVBox/Header
	if header:
		var header_style = StyleBoxFlat.new()
		header_style.bg_color = ThemeManager.BG_SURFACE
		header_style.corner_radius_bottom_left = ThemeManager.CORNER_RADIUS_LARGE
		header_style.corner_radius_bottom_right = ThemeManager.CORNER_RADIUS_LARGE
		header_style.shadow_size = 4
		header_style.shadow_offset = Vector2(0, 2)
		header_style.shadow_color = ThemeManager.SHADOW_COLOR
		header_style.content_margin_left = ThemeManager.MOBILE_MARGIN
		header_style.content_margin_right = ThemeManager.MOBILE_MARGIN
		header_style.content_margin_top = ThemeManager.MOBILE_PADDING
		header_style.content_margin_bottom = ThemeManager.MOBILE_PADDING
		header.add_theme_stylebox_override("panel", header_style)

	# Back button styling
	if back_button:
		back_button.custom_minimum_size = Vector2(ThemeManager.MOBILE_TOUCH_SIZE * 2, ThemeManager.MOBILE_TOUCH_SIZE)
		back_button.add_theme_stylebox_override("normal", CustomStyles.create_ghost_button())
		back_button.add_theme_stylebox_override("hover", CustomStyles.create_ghost_button())
		back_button.add_theme_stylebox_override("pressed", CustomStyles.create_primary_button())
		back_button.add_theme_font_size_override("font_size", ThemeManager.FONT_SIZE_MEDIUM)

	# Search input styling
	if search_input:
		search_input.custom_minimum_size = Vector2(400, ThemeManager.MOBILE_TOUCH_SIZE)
		var search_style = StyleBoxFlat.new()
		search_style.bg_color = ThemeManager.BG_SURFACE_VARIANT
		search_style.corner_radius_top_left = ThemeManager.CORNER_RADIUS_MEDIUM
		search_style.corner_radius_top_right = ThemeManager.CORNER_RADIUS_MEDIUM
		search_style.corner_radius_bottom_left = ThemeManager.CORNER_RADIUS_MEDIUM
		search_style.corner_radius_bottom_right = ThemeManager.CORNER_RADIUS_MEDIUM
		search_style.content_margin_left = 12
		search_style.content_margin_right = 12
		search_input.add_theme_stylebox_override("normal", search_style)
		search_input.add_theme_font_size_override("font_size", ThemeManager.FONT_SIZE_MEDIUM)

	# Tab container styling
	if tab_container:
		tab_container.add_theme_font_size_override("font_size", ThemeManager.FONT_SIZE_MEDIUM)
		# Style each tab
		for i in range(tab_container.get_tab_count()):
			var tab = tab_container.get_child(i)
			if tab:
				_style_tab_content(tab)

	# Stats panel styling
	if stats_panel:
		stats_panel.custom_minimum_size = Vector2(0, 150)
		stats_panel.add_theme_stylebox_override("panel", CustomStyles.create_card_panel())


func _style_tab_content(tab: Control):
	"""Style individual tab content"""
	var scroll = tab.get_child(0) if tab.get_child_count() > 0 else null
	if scroll and scroll is ScrollContainer:
		# Make scroll container touch-friendly
		scroll.custom_minimum_size = Vector2(0, 600)

		var grid = scroll.get_child(0) if scroll.get_child_count() > 0 else null
		if grid and grid is GridContainer:
			grid.add_theme_constant_override("h_separation", 20)
			grid.add_theme_constant_override("v_separation", 20)


func _setup_ui():
	"""Setup UI components"""

	# Create tabs for each category
	for category in SKILL_CATEGORIES:
		var existing_tab = tab_container.get_node_or_null(category + "Tab")
		if not existing_tab:
			_create_category_tab(category)

	# Initialize all skills list
	for category in SKILL_CATEGORIES:
		for skill in SKILL_CATEGORIES[category]["skills"]:
			all_skills.append(
				{
					"name": skill,
					"display_name": SKILL_CATEGORIES[category]["display_names"][skill],
					"category": category
				}
			)


func _create_category_tab(category: String):
	"""Create a tab for a skill category"""
	var scroll = ScrollContainer.new()
	scroll.name = category + "Tab"
	tab_container.add_child(scroll)
	tab_container.set_tab_title(tab_container.get_tab_count() - 1, category)

	var grid = GridContainer.new()
	grid.name = category + "Grid"
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 20)
	grid.add_theme_constant_override("v_separation", 15)
	scroll.add_child(grid)

	# Add margin container for padding
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", ThemeManager.MOBILE_MARGIN)
	margin.add_theme_constant_override("margin_right", ThemeManager.MOBILE_MARGIN)
	margin.add_theme_constant_override("margin_top", ThemeManager.MOBILE_PADDING)
	margin.add_theme_constant_override("margin_bottom", ThemeManager.MOBILE_PADDING)
	scroll.add_child(margin)
	margin.add_child(grid)


func _connect_signals():
	"""Connect UI signals"""

	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	if search_input:
		search_input.text_changed.connect(_on_search_changed)

	# Connect to player data changes
	if EnhancedPlayerData:
		EnhancedPlayerData.stats_changed.connect(_on_stats_changed)


func _load_player_data():
	"""Load and display player skill data"""

	print("[Enhanced42StatsScreen] Loading player data...")

	for category in SKILL_CATEGORIES:
		var grid = tab_container.get_node(category + "Tab/" + category + "Grid")
		if not grid:
			grid = tab_container.get_node(category + "Tab/MarginContainer/" + category + "Grid")

		if grid:
			# Clear existing widgets
			for child in grid.get_children():
				child.queue_free()

			# Create skill widgets
			for skill_name in SKILL_CATEGORIES[category]["skills"]:
				var widget = _create_skill_widget(skill_name, category)
				grid.add_child(widget)
				skill_widgets[skill_name] = widget


func _create_skill_widget(skill_name: String, category: String) -> Control:
	"""Create a properly styled skill widget"""

	var widget = PanelContainer.new()
	widget.custom_minimum_size = Vector2(400, 80)
	widget.add_theme_stylebox_override("panel", _create_skill_panel_style(category))

	var hbox = HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 15)
	widget.add_child(hbox)

	# Skill name
	var name_label = Label.new()
	name_label.text = SKILL_CATEGORIES[category]["display_names"][skill_name]
	name_label.custom_minimum_size = Vector2(150, 0)
	name_label.add_theme_font_size_override("font_size", ThemeManager.FONT_SIZE_MEDIUM)
	name_label.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
	hbox.add_child(name_label)

	# Value and progress bar
	var bar_container = VBoxContainer.new()
	bar_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_child(bar_container)

	# Get skill value from EnhancedPlayerData
	var skill_value = EnhancedPlayerData.get_skill(skill_name) if EnhancedPlayerData else 50.0

	# Progress bar
	var progress_bar = ProgressBar.new()
	progress_bar.value = skill_value
	progress_bar.max_value = 100
	progress_bar.custom_minimum_size = Vector2(0, 25)
	progress_bar.show_percentage = false

	# Style progress bar based on value
	var bar_style = StyleBoxFlat.new()
	bar_style.bg_color = ThemeManager.get_stat_color(skill_value)
	bar_style.corner_radius_top_left = 4
	bar_style.corner_radius_top_right = 4
	bar_style.corner_radius_bottom_left = 4
	bar_style.corner_radius_bottom_right = 4
	progress_bar.add_theme_stylebox_override("fill", bar_style)

	bar_container.add_child(progress_bar)

	# Value label
	var value_label = Label.new()
	value_label.text = "%d / 100" % int(skill_value)
	value_label.add_theme_font_size_override("font_size", ThemeManager.FONT_SIZE_SMALL)
	value_label.add_theme_color_override("font_color", ThemeManager.TEXT_SECONDARY)
	bar_container.add_child(value_label)

	# Grade badge
	var grade_container = CenterContainer.new()
	grade_container.custom_minimum_size = Vector2(60, 60)
	hbox.add_child(grade_container)

	var grade_panel = Panel.new()
	grade_panel.custom_minimum_size = Vector2(50, 50)

	var grade = EnhancedPlayerData.get_skill_grade(skill_value) if EnhancedPlayerData else "C"
	var grade_style = StyleBoxFlat.new()
	grade_style.bg_color = ThemeManager.get_skill_grade_background(grade)
	grade_style.corner_radius_top_left = 25
	grade_style.corner_radius_top_right = 25
	grade_style.corner_radius_bottom_left = 25
	grade_style.corner_radius_bottom_right = 25
	grade_panel.add_theme_stylebox_override("panel", grade_style)
	grade_container.add_child(grade_panel)

	var grade_label = Label.new()
	grade_label.text = grade
	grade_label.add_theme_font_size_override("font_size", ThemeManager.FONT_SIZE_TITLE)
	grade_label.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
	grade_label.set_anchors_preset(Control.PRESET_CENTER)
	grade_panel.add_child(grade_label)

	# Store references
	widget.set_meta("skill_name", skill_name)
	widget.set_meta("progress_bar", progress_bar)
	widget.set_meta("value_label", value_label)
	widget.set_meta("grade_label", grade_label)
	widget.set_meta("grade_panel", grade_panel)

	# Add animation on hover (mobile touch feedback)
	widget.gui_input.connect(_on_widget_input.bind(widget))

	return widget


func _create_skill_panel_style(category: String) -> StyleBoxFlat:
	"""Create styled panel for skill widget"""
	var style = StyleBoxFlat.new()
	style.bg_color = ThemeManager.BG_SURFACE
	style.corner_radius_top_left = ThemeManager.CORNER_RADIUS_MEDIUM
	style.corner_radius_top_right = ThemeManager.CORNER_RADIUS_MEDIUM
	style.corner_radius_bottom_left = ThemeManager.CORNER_RADIUS_MEDIUM
	style.corner_radius_bottom_right = ThemeManager.CORNER_RADIUS_MEDIUM

	# Add category-specific border color
	style.border_width_left = 3
	style.border_color = SKILL_CATEGORIES[category]["color"]

	style.content_margin_left = 20
	style.content_margin_right = 20
	style.content_margin_top = 15
	style.content_margin_bottom = 15

	# Shadow
	style.shadow_size = 2
	style.shadow_offset = Vector2(1, 2)
	style.shadow_color = ThemeManager.SHADOW_COLOR

	return style


func _on_widget_input(event: InputEvent, widget: Control):
	"""Handle touch/click on skill widget"""
	if event is InputEventScreenTouch or event is InputEventMouseButton:
		if event.pressed:
			# Visual feedback
			var tween = create_tween()
			tween.set_ease(Tween.EASE_OUT)
			tween.set_trans(Tween.TRANS_CUBIC)
			tween.tween_property(widget, "scale", Vector2(0.95, 0.95), 0.1)
			tween.tween_property(widget, "scale", Vector2(1.0, 1.0), 0.1)


func _update_statistics():
	"""Update statistics panel"""
	if not stats_text:
		return

	stats_text.clear()
	stats_text.push_color(ThemeManager.TEXT_PRIMARY)
	stats_text.append_text("Player Statistics\n\n")
	stats_text.pop()

	# Calculate category averages
	for category in SKILL_CATEGORIES:
		var avg = EnhancedPlayerData.get_category_average(category) if EnhancedPlayerData else 50.0
		var color = SKILL_CATEGORIES[category]["color"]

		stats_text.push_color(color)
		stats_text.append_text("● %s: " % category)
		stats_text.pop()
		stats_text.append_text("%.1f\n" % avg)

	# Overall rating
	var overall = EnhancedPlayerData.get_overall_rating() if EnhancedPlayerData else 50
	stats_text.append_text("\n")
	stats_text.push_bold()
	stats_text.append_text("Overall Rating: %d" % overall)
	stats_text.pop()


func _on_search_changed(text: String):
	"""Handle search input"""
	var search_term = text.to_lower()

	if search_term == "":
		# Show all skills
		for widget in skill_widgets.values():
			widget.visible = true
	else:
		# Filter skills
		for skill_name in skill_widgets:
			var widget = skill_widgets[skill_name]
			var display_name = ""

			# Find display name
			for category in SKILL_CATEGORIES:
				if skill_name in SKILL_CATEGORIES[category]["skills"]:
					display_name = SKILL_CATEGORIES[category]["display_names"][skill_name]
					break

			# Show/hide based on search
			widget.visible = display_name.to_lower().contains(search_term)


func _on_stats_changed():
	"""Handle player stats change"""
	# Refresh all skill widgets
	for skill_name in skill_widgets:
		var widget = skill_widgets[skill_name]
		_update_skill_widget(widget, skill_name)

	# Update statistics
	_update_statistics()


func _update_skill_widget(widget: Control, skill_name: String):
	"""Update a skill widget with current data"""
	var skill_value = EnhancedPlayerData.get_skill(skill_name) if EnhancedPlayerData else 50.0
	var grade = EnhancedPlayerData.get_skill_grade(skill_value) if EnhancedPlayerData else "C"

	# Update progress bar
	var progress_bar = widget.get_meta("progress_bar")
	if progress_bar:
		progress_bar.value = skill_value

		# Update bar color
		var bar_style = StyleBoxFlat.new()
		bar_style.bg_color = ThemeManager.get_stat_color(skill_value)
		bar_style.corner_radius_top_left = 4
		bar_style.corner_radius_top_right = 4
		bar_style.corner_radius_bottom_left = 4
		bar_style.corner_radius_bottom_right = 4
		progress_bar.add_theme_stylebox_override("fill", bar_style)

	# Update value label
	var value_label = widget.get_meta("value_label")
	if value_label:
		value_label.text = "%d / 100" % int(skill_value)

	# Update grade
	var grade_label = widget.get_meta("grade_label")
	if grade_label:
		grade_label.text = grade

	var grade_panel = widget.get_meta("grade_panel")
	if grade_panel:
		var grade_style = StyleBoxFlat.new()
		grade_style.bg_color = ThemeManager.get_skill_grade_background(grade)
		grade_style.corner_radius_top_left = 25
		grade_style.corner_radius_top_right = 25
		grade_style.corner_radius_bottom_left = 25
		grade_style.corner_radius_bottom_right = 25
		grade_panel.add_theme_stylebox_override("panel", grade_style)


func _on_back_pressed():
	"""Handle back button press"""
	print("[Enhanced42StatsScreen] Back pressed")

	# Use UIManager to go back
	if UIManager:
		UIManager.pop()
	else:
		back_pressed.emit()


func show_skills_animated():
	"""Show skills with animation (for result screen)"""
	var delay = 0.0

	for widget in skill_widgets.values():
		widget.modulate.a = 0.0
		widget.scale = Vector2(0.8, 0.8)

		var tween = create_tween()
		tween.set_ease(Tween.EASE_OUT)
		tween.set_trans(Tween.TRANS_CUBIC)
		tween.set_parallel()

		tween.tween_property(widget, "modulate:a", 1.0, 0.3).set_delay(delay)
		tween.tween_property(widget, "scale", Vector2(1.0, 1.0), 0.3).set_delay(delay)

		delay += 0.05  # Stagger animation


# Touch gesture support
var touch_start_pos = Vector2.ZERO
var is_swiping = false


func _input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			touch_start_pos = event.position
			is_swiping = true
		else:
			is_swiping = false

	elif event is InputEventScreenDrag and is_swiping:
		var swipe_distance = event.position.x - touch_start_pos.x

		# Swipe to change tabs
		if abs(swipe_distance) > 100:
			if swipe_distance > 0:
				# Swipe right - previous tab
				if tab_container.current_tab > 0:
					tab_container.current_tab -= 1
			else:
				# Swipe left - next tab
				if tab_container.current_tab < tab_container.get_tab_count() - 1:
					tab_container.current_tab += 1

			is_swiping = false
