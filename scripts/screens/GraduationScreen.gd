extends Control
## GraduationScreen - 졸업식 화면 (3년차 완료 시)
## Phase 24: Enhanced with decision highlights, division timeline, real stats

# Existing UI elements
@onready var title_label = $VBox/Title
@onready var message_label = $VBox/Message
@onready var stats_container = $VBox/StatsContainer
@onready var player_name_label = $VBox/PlayerName
@onready var final_ca_label = $VBox/StatsContainer/FinalCA
@onready var matches_played_label = $VBox/StatsContainer/MatchesPlayed
@onready var training_count_label = $VBox/StatsContainer/TrainingCount
@onready var return_button = $VBox/ReturnButton

# Phase 24: New UI elements (will be created dynamically)
var ending_card_container: VBoxContainer
var rarity_badge_label: Label
var narrative_label: RichTextLabel
var highlights_panel: VBoxContainer
var division_panel: HBoxContainer
var enhanced_stats_panel: GridContainer

var graduation_data: Dictionary = {}


func _ready():
	print("[GraduationScreen] Graduation ceremony started")

	# Setup button
	if return_button:
		return_button.pressed.connect(_on_return_pressed)

	# Load graduation data
	_load_graduation_data()

	# Display graduation info
	_display_graduation_info()


func _load_graduation_data():
	"""Load graduation data from GraduationManager"""
	var graduation_manager = get_node_or_null("/root/GraduationManager")
	if not graduation_manager:
		push_error("[GraduationScreen] GraduationManager not found!")
		return

	if graduation_manager.has_method("get_graduation_data"):
		graduation_data = graduation_manager.get_graduation_data()
		print("[GraduationScreen] Graduation data loaded: %s" % graduation_data.get("name", "Unknown"))


# ============================================================================
# PHASE 24 IMPROVEMENTS: UI HELPER FUNCTIONS (P3-1)
# ============================================================================


func _insert_node_after(new_node: Node, anchor_node: Node):
	"""
	Insert new_node immediately after anchor_node in the parent container

	Args:
		new_node: The node to insert
		anchor_node: The reference node to insert after
	"""
	if not anchor_node:
		push_error("[GraduationScreen] anchor_node is null, cannot insert")
		return

	var parent = anchor_node.get_parent()
	if not parent:
		push_error("[GraduationScreen] Anchor node has no parent")
		return

	var index = anchor_node.get_index()
	parent.add_child(new_node)
	parent.move_child(new_node, index + 1)


func _insert_node_before(new_node: Node, anchor_node: Node):
	"""
	Insert new_node immediately before anchor_node in the parent container

	Args:
		new_node: The node to insert
		anchor_node: The reference node to insert before
	"""
	if not anchor_node:
		push_error("[GraduationScreen] anchor_node is null, cannot insert")
		return

	var parent = anchor_node.get_parent()
	if not parent:
		push_error("[GraduationScreen] Anchor node has no parent")
		return

	var index = anchor_node.get_index()
	parent.add_child(new_node)
	parent.move_child(new_node, index)


func _create_panel_with_title(
	title_text: String, title_font_size: int, title_color: Color, anchor_node: Control
) -> Dictionary:
	"""
	Create a title Label + VBoxContainer panel and insert after anchor_node

	Args:
		title_text: Text for the section title
		title_font_size: Font size for title
		title_color: Color for title text
		anchor_node: Node to insert the panel after

	Returns:
		Dictionary with "title": Label and "panel": VBoxContainer
	"""
	# Create title
	var title = Label.new()
	title.text = title_text
	title.add_theme_font_size_override("font_size", title_font_size)
	title.add_theme_color_override("font_color", title_color)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	# Create panel
	var panel = VBoxContainer.new()
	panel.custom_minimum_size = Vector2(0, 100)

	# Insert title first
	_insert_node_after(title, anchor_node)

	# Insert panel after title
	_insert_node_after(panel, title)

	return {"title": title, "panel": panel}


func _display_graduation_info():
	"""Display graduation ceremony information - Phase 24 Enhanced"""
	print("[GraduationScreen] Displaying Phase 24 enhanced graduation info...")

	# Get Phase 24 data
	var ending = graduation_data.get("ending", {})
	var career_stats = graduation_data.get("career_stats", {})
	var stats_summary = ending.get("stats_summary", {})

	# Player info
	var player_name = graduation_data.get("name", "선수")
	var final_ca = graduation_data.get("graduation_info", {}).get("final_ca", graduation_data.get("overall", 0))

	# === 1. Enhanced Title with Rarity ===
	_display_enhanced_title(ending, player_name)

	# === 2. Career Narrative ===
	_display_narrative(ending)

	# === 3. Decision Highlights (Phase 24 핵심) ===
	_display_decision_highlights(ending)

	# === 4. Division Progress Timeline ===
	_display_division_timeline(career_stats, stats_summary)

	# === 5. Enhanced Stats with Real Data ===
	_display_enhanced_stats(career_stats, stats_summary, final_ca)

	print("[GraduationScreen] Phase 24 graduation info displayed")


func _on_return_pressed():
	"""Return to main menu"""
	print("[GraduationScreen] Returning to main menu...")

	# Reset game state
	if has_node("/root/GameManager"):
		var game_manager = get_node("/root/GameManager")
		if game_manager.has_method("reset_game"):
			game_manager.reset_game()

	# Return to main menu
	get_tree().change_scene_to_file("res://scenes/menus/main_menu.tscn")


# ============================================================================
# PHASE 24: ENHANCED DISPLAY FUNCTIONS
# ============================================================================


func _display_enhanced_title(ending: Dictionary, player_name: String):
	"""Display title with rarity badge and ending"""
	# Get ending info
	var korean_title = ending.get("korean_title", "🎓 졸업생")
	var rarity = ending.get("rarity", "B")

	# Update title with rarity color
	if title_label:
		title_label.text = "🎓 졸업을 축하합니다!"

	# Update player name with rarity badge
	if player_name_label:
		var rarity_color = _get_rarity_color(rarity)
		player_name_label.text = "%s 선수 [%s]" % [player_name, rarity]
		player_name_label.add_theme_color_override("font_color", rarity_color)

	# Update message with ending title
	if message_label:
		message_label.text = korean_title
		var rarity_color = _get_rarity_color(rarity)
		message_label.add_theme_color_override("font_color", rarity_color)

	print("[GraduationScreen] Title displayed: %s (%s)" % [korean_title, rarity])


func _display_narrative(ending: Dictionary):
	"""Display career narrative story"""
	var narrative = ending.get("narrative", ending.get("description", ""))

	if narrative.is_empty():
		return

	# Create narrative label dynamically if needed
	if not narrative_label:
		narrative_label = RichTextLabel.new()
		narrative_label.custom_minimum_size = Vector2(0, 100)
		narrative_label.fit_content = true
		narrative_label.bbcode_enabled = true

		# Insert after message_label using helper (P3-1 refactor)
		if message_label:
			_insert_node_after(narrative_label, message_label)

	narrative_label.text = narrative
	print("[GraduationScreen] Narrative displayed (%d chars)" % narrative.length())


func _display_decision_highlights(ending: Dictionary):
	"""Display top 5-8 key moments - Phase 24 Core Feature"""
	var highlights = ending.get("highlights", [])

	if highlights.is_empty():
		print("[GraduationScreen] No decision highlights to display")
		return

	# Create highlights panel dynamically
	if not highlights_panel:
		highlights_panel = VBoxContainer.new()
		highlights_panel.custom_minimum_size = Vector2(0, 200)

		# Section title
		var section_title = Label.new()
		section_title.text = "━━━ 주요 순간들 (Key Moments) ━━━"
		section_title.add_theme_font_size_override("font_size", 32)
		section_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		section_title.add_theme_color_override("font_color", Color(1, 0.8, 0.3))
		highlights_panel.add_child(section_title)

		# Add spacer
		var spacer = Control.new()
		spacer.custom_minimum_size = Vector2(0, 20)
		highlights_panel.add_child(spacer)

		# Insert before stats_container using helper (P3-1 refactor)
		if stats_container:
			_insert_node_before(highlights_panel, stats_container)

	# Display top 5-8 highlights
	var display_count = min(highlights.size(), 8)
	for i in range(display_count):
		var highlight = highlights[i]

		var item = HBoxContainer.new()
		item.custom_minimum_size = Vector2(0, 40)

		# Week label
		var week_label = Label.new()
		week_label.text = "Week %d" % highlight.get("week", 0)
		week_label.custom_minimum_size = Vector2(100, 0)
		week_label.add_theme_font_size_override("font_size", 20)
		week_label.add_theme_color_override("font_color", Color.GRAY)
		item.add_child(week_label)

		# Description
		var desc_label = Label.new()
		desc_label.text = highlight.get("description", "Unknown event")
		desc_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		desc_label.add_theme_font_size_override("font_size", 22)
		desc_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		item.add_child(desc_label)

		# Type icon
		var type_icon = Label.new()
		var highlight_type = highlight.get("type", "decision")
		match highlight_type:
			"achievement":
				type_icon.text = "🏆"
			"decision":
				type_icon.text = "🔥"
			"setback":
				type_icon.text = "⚠️"
			"growth":
				type_icon.text = "📈"
			_:
				type_icon.text = "📌"
		type_icon.custom_minimum_size = Vector2(50, 0)
		type_icon.add_theme_font_size_override("font_size", 28)
		item.add_child(type_icon)

		highlights_panel.add_child(item)

	print("[GraduationScreen] Displayed %d decision highlights" % display_count)


func _display_division_timeline(career_stats: Dictionary, stats_summary: Dictionary):
	"""Display season-by-season division progression"""
	# Try to get division history from career_stats
	var division_history = []

	# Priority 1: career_stats (from CareerStatisticsManager)
	if career_stats.has("division_history"):
		division_history = career_stats.division_history
	# Priority 2: stats_summary fallback
	elif stats_summary.has("final_division"):
		division_history = [
			{
				"year": 3,
				"division": stats_summary.get("final_division", 3),
				"position": stats_summary.get("final_position", 6)
			}
		]

	if division_history.is_empty():
		print("[GraduationScreen] No division history to display")
		return

	# Create division panel dynamically
	if not division_panel:
		division_panel = HBoxContainer.new()
		division_panel.custom_minimum_size = Vector2(0, 150)
		division_panel.alignment = BoxContainer.ALIGNMENT_CENTER

		# Create wrapper with title
		var wrapper = VBoxContainer.new()

		# Section title
		var section_title = Label.new()
		section_title.text = "━━━ 디비전 진행 (Division Progress) ━━━"
		section_title.add_theme_font_size_override("font_size", 32)
		section_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		section_title.add_theme_color_override("font_color", Color(0.3, 0.8, 1))
		wrapper.add_child(section_title)

		# Add spacer
		var spacer = Control.new()
		spacer.custom_minimum_size = Vector2(0, 20)
		wrapper.add_child(spacer)

		wrapper.add_child(division_panel)

		# Insert before stats_container using helper (P3-1 refactor)
		if stats_container:
			_insert_node_before(wrapper, stats_container)

	# Display each season
	for i in range(division_history.size()):
		var season = division_history[i]

		# Season card
		var card = VBoxContainer.new()
		card.custom_minimum_size = Vector2(200, 150)

		# Year
		var year_label = Label.new()
		year_label.text = "Year %d" % season.get("year", i + 1)
		year_label.add_theme_font_size_override("font_size", 24)
		year_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		card.add_child(year_label)

		# Division
		var div_label = Label.new()
		div_label.text = "Division %d" % season.get("division", 3)
		div_label.add_theme_font_size_override("font_size", 32)
		div_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		div_label.add_theme_color_override("font_color", Color(1, 0.8, 0.3))
		card.add_child(div_label)

		# Position
		var pos_label = Label.new()
		pos_label.text = "%d위 / 6" % season.get("position", 6)
		pos_label.add_theme_font_size_override("font_size", 22)
		pos_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		card.add_child(pos_label)

		# Badge
		if season.get("promoted", false):
			var badge = Label.new()
			badge.text = "🔥 승격"
			badge.add_theme_color_override("font_color", Color.ORANGE)
			badge.add_theme_font_size_override("font_size", 20)
			badge.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			card.add_child(badge)
		elif season.get("relegated", false):
			var badge = Label.new()
			badge.text = "😞 강등"
			badge.add_theme_color_override("font_color", Color.DARK_RED)
			badge.add_theme_font_size_override("font_size", 20)
			badge.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			card.add_child(badge)

		division_panel.add_child(card)

		# Arrow (except last)
		if i < division_history.size() - 1:
			var arrow = Label.new()
			arrow.text = "→"
			arrow.add_theme_font_size_override("font_size", 48)
			arrow.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
			division_panel.add_child(arrow)

	print("[GraduationScreen] Displayed %d seasons in division timeline" % division_history.size())


func _display_enhanced_stats(career_stats: Dictionary, stats_summary: Dictionary, final_ca: int):
	"""Display enhanced stats with REAL data from CareerStatisticsManager"""
	# Get real statistics (Phase 24 - NO MORE HARDCODED 0s!)
	var total_goals = career_stats.get("total_goals", stats_summary.get("total_goals", 0))
	var total_assists = career_stats.get("total_assists", stats_summary.get("total_assists", 0))
	var total_matches = career_stats.get("total_matches", stats_summary.get("total_matches", 0))
	var average_rating = career_stats.get("average_rating", stats_summary.get("average_rating", 0.0))
	var win_rate = career_stats.get("win_rate", stats_summary.get("win_rate", 0.0))
	var ca_growth = stats_summary.get("ca_growth", 0)

	# Update existing stats labels with REAL DATA
	if final_ca_label:
		if ca_growth > 0:
			final_ca_label.text = "최종 능력치: %d (+%d)" % [final_ca, ca_growth]
		else:
			final_ca_label.text = "최종 능력치: %d" % final_ca

	if matches_played_label:
		matches_played_label.text = "총 경기: %d회" % total_matches

	if training_count_label:
		# Hide old training count, replace with goals/assists
		training_count_label.visible = false

	# Create enhanced stats panel dynamically
	if not enhanced_stats_panel:
		enhanced_stats_panel = GridContainer.new()
		enhanced_stats_panel.columns = 2
		enhanced_stats_panel.add_theme_constant_override("h_separation", 40)
		enhanced_stats_panel.add_theme_constant_override("v_separation", 15)

		# Insert after matches_played_label
		if matches_played_label:
			var parent = matches_played_label.get_parent()
			var index = matches_played_label.get_index()
			parent.add_child(enhanced_stats_panel)
			parent.move_child(enhanced_stats_panel, index + 1)

	# Clear existing stats
	for child in enhanced_stats_panel.get_children():
		child.queue_free()

	# Add real statistics (Phase 24!)
	_add_stat_row("총 득점", "%d골" % total_goals)
	_add_stat_row("총 도움", "%d개" % total_assists)

	if average_rating > 0:
		_add_stat_row("평균 평점", "%.1f" % average_rating)

	if win_rate > 0:
		_add_stat_row("승률", "%.0f%%" % win_rate)

	print("[GraduationScreen] Enhanced stats displayed: %d goals, %d assists" % [total_goals, total_assists])


func _add_stat_row(label_text: String, value_text: String, color: Color = Color.WHITE):
	"""Add a stat row to enhanced_stats_panel"""
	if not enhanced_stats_panel:
		return

	# Label
	var label_node = Label.new()
	label_node.text = label_text + ":"
	label_node.add_theme_font_size_override("font_size", 24)
	enhanced_stats_panel.add_child(label_node)

	# Value
	var value_node = Label.new()
	value_node.text = value_text
	value_node.add_theme_font_size_override("font_size", 24)
	value_node.add_theme_color_override("font_color", color)
	value_node.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	enhanced_stats_panel.add_child(value_node)


func _get_rarity_color(rarity: String) -> Color:
	"""Get color for rarity badge"""
	match rarity:
		"SS":
			return Color.GOLD
		"S":
			return Color.ORANGE_RED
		"A":
			return Color.ROYAL_BLUE
		"B":
			return Color.LIME_GREEN
		"C":
			return Color.GRAY
		_:
			return Color.WHITE
