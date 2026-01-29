extends Control

## HalftimeScreen - Halftime tactical adjustment screen
## Shows first half stats and allows tactical changes

signal continue_match
signal formation_changed(formation: String)
signal tactics_changed(instructions: Dictionary)

# Match statistics
var first_half_stats: Dictionary = {}
var current_formation: String = "4-4-2"
var current_instructions: Dictionary = {}

# UI References
var stats_container: VBoxContainer = null
var formation_selector: OptionButton = null
var tactical_panel: Control = null

# Rust Engine
var rust_engine: Node = null


func _ready():
	print("[HalftimeScreen] Initializing halftime screen")

	rust_engine = get_node_or_null("/root/FootballRustEngine")

	_build_ui()


func _build_ui():
	# Main container
	var main_vbox = VBoxContainer.new()
	main_vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_vbox.add_theme_constant_override("separation", 20)
	add_child(main_vbox)

	# Margin
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 40)
	margin.add_theme_constant_override("margin_right", 40)
	margin.add_theme_constant_override("margin_top", 30)
	margin.add_theme_constant_override("margin_bottom", 30)
	main_vbox.add_child(margin)

	var content_vbox = VBoxContainer.new()
	content_vbox.add_theme_constant_override("separation", 15)
	content_vbox.size_flags_vertical = Control.SIZE_EXPAND_FILL
	margin.add_child(content_vbox)

	# Title
	var title = Label.new()
	title.text = "⏱️ 하프타임"
	title.add_theme_font_size_override("font_size", 32)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content_vbox.add_child(title)

	# Score display
	var score_label = Label.new()
	score_label.name = "ScoreLabel"
	score_label.text = "0 - 0"
	score_label.add_theme_font_size_override("font_size", 48)
	score_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content_vbox.add_child(score_label)

	# Separator
	var sep1 = HSeparator.new()
	content_vbox.add_child(sep1)

	# Stats section
	var stats_section = _create_stats_section()
	content_vbox.add_child(stats_section)

	# Separator
	var sep2 = HSeparator.new()
	content_vbox.add_child(sep2)

	# Tactical adjustment section
	var tactics_section = _create_tactics_section()
	content_vbox.add_child(tactics_section)

	# Continue button
	var continue_btn = Button.new()
	continue_btn.text = "▶️ 후반전 시작"
	continue_btn.custom_minimum_size = Vector2(300, 70)
	continue_btn.add_theme_font_size_override("font_size", 22)
	continue_btn.pressed.connect(_on_continue_pressed)
	content_vbox.add_child(continue_btn)

	# Center the button
	var btn_container = CenterContainer.new()
	content_vbox.remove_child(continue_btn)
	btn_container.add_child(continue_btn)
	content_vbox.add_child(btn_container)


func _create_stats_section() -> Control:
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 10)

	# Section title
	var title = Label.new()
	title.text = "📊 전반전 통계"
	title.add_theme_font_size_override("font_size", 22)
	section.add_child(title)

	# Stats grid
	stats_container = VBoxContainer.new()
	stats_container.add_theme_constant_override("separation", 8)
	section.add_child(stats_container)

	# Default stats (will be updated)
	var default_stats = [
		{"label": "점유율", "home": "50%", "away": "50%"},
		{"label": "슈팅", "home": "0", "away": "0"},
		{"label": "유효 슈팅", "home": "0", "away": "0"},
		{"label": "패스 성공률", "home": "0%", "away": "0%"},
		{"label": "코너킥", "home": "0", "away": "0"}
	]

	for stat in default_stats:
		var row = _create_stat_row(stat.label, stat.home, stat.away)
		stats_container.add_child(row)

	return section


func _create_stat_row(label: String, home_value: String, away_value: String) -> Control:
	var row = HBoxContainer.new()
	row.add_theme_constant_override("separation", 20)

	# Home value
	var home_label = Label.new()
	home_label.text = home_value
	home_label.add_theme_font_size_override("font_size", 16)
	home_label.custom_minimum_size.x = 80
	home_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	row.add_child(home_label)

	# Stat name
	var stat_label = Label.new()
	stat_label.text = label
	stat_label.add_theme_font_size_override("font_size", 16)
	stat_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	stat_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	row.add_child(stat_label)

	# Away value
	var away_label = Label.new()
	away_label.text = away_value
	away_label.add_theme_font_size_override("font_size", 16)
	away_label.custom_minimum_size.x = 80
	away_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	row.add_child(away_label)

	return row


func _create_tactics_section() -> Control:
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 15)

	# Section title
	var title = Label.new()
	title.text = "⚙️ 전술 조정"
	title.add_theme_font_size_override("font_size", 22)
	section.add_child(title)

	# Formation selector
	var formation_row = HBoxContainer.new()
	formation_row.add_theme_constant_override("separation", 15)

	var formation_label = Label.new()
	formation_label.text = "포메이션:"
	formation_label.add_theme_font_size_override("font_size", 18)
	formation_row.add_child(formation_label)

	formation_selector = OptionButton.new()
	formation_selector.custom_minimum_size = Vector2(200, 50)
	formation_selector.add_theme_font_size_override("font_size", 16)

	# Add formations
	var formations = ["4-4-2", "4-3-3", "4-2-3-1", "3-5-2", "5-3-2", "4-1-4-1"]
	for f in formations:
		formation_selector.add_item(f)

	formation_selector.item_selected.connect(_on_formation_selected)
	formation_row.add_child(formation_selector)

	section.add_child(formation_row)

	# Quick tactical adjustments
	var quick_section = VBoxContainer.new()
	quick_section.add_theme_constant_override("separation", 10)

	var quick_label = Label.new()
	quick_label.text = "빠른 전술 변경:"
	quick_label.add_theme_font_size_override("font_size", 16)
	quick_section.add_child(quick_label)

	var quick_btns = HBoxContainer.new()
	quick_btns.add_theme_constant_override("separation", 10)

	var presets = [
		{"name": "공격적", "id": "attacking"}, {"name": "수비적", "id": "defensive"}, {"name": "역습", "id": "counter"}
	]

	for preset in presets:
		var btn = Button.new()
		btn.text = preset.name
		btn.custom_minimum_size = Vector2(100, 45)
		btn.pressed.connect(_on_quick_preset_pressed.bind(preset.id))
		quick_btns.add_child(btn)

	quick_section.add_child(quick_btns)
	section.add_child(quick_section)

	return section


func set_match_data(stats: Dictionary, formation: String, score_home: int, score_away: int):
	"""Set match data from simulation"""
	first_half_stats = stats
	current_formation = formation

	# Update score
	var score_label = get_node_or_null("MarginContainer/VBoxContainer/ScoreLabel")
	if score_label:
		score_label.text = "%d - %d" % [score_home, score_away]

	# Update formation selector
	if formation_selector:
		for i in range(formation_selector.item_count):
			if formation_selector.get_item_text(i) == formation:
				formation_selector.selected = i
				break

	# Update stats display
	_update_stats_display(stats)


func _update_stats_display(stats: Dictionary):
	if not stats_container:
		return

	# Clear existing stats
	for child in stats_container.get_children():
		child.queue_free()

	# Add updated stats
	var home_stats = stats.get("home", {})
	var away_stats = stats.get("away", {})

	var stat_rows = [
		{
			"label": "점유율",
			"home": "%d%%" % home_stats.get("possession", 50),
			"away": "%d%%" % away_stats.get("possession", 50)
		},
		{"label": "슈팅", "home": str(home_stats.get("shots", 0)), "away": str(away_stats.get("shots", 0))},
		{
			"label": "유효 슈팅",
			"home": str(home_stats.get("shots_on_target", 0)),
			"away": str(away_stats.get("shots_on_target", 0))
		},
		{
			"label": "패스 성공률",
			"home": "%d%%" % home_stats.get("pass_accuracy", 0),
			"away": "%d%%" % away_stats.get("pass_accuracy", 0)
		},
		{"label": "코너킥", "home": str(home_stats.get("corners", 0)), "away": str(away_stats.get("corners", 0))}
	]

	for stat in stat_rows:
		var row = _create_stat_row(stat.label, stat.home, stat.away)
		stats_container.add_child(row)


func _on_formation_selected(index: int):
	var new_formation = formation_selector.get_item_text(index)
	print("[HalftimeScreen] Formation changed to: %s" % new_formation)
	current_formation = new_formation
	formation_changed.emit(new_formation)


func _on_quick_preset_pressed(preset_id: String):
	print("[HalftimeScreen] Quick preset: %s" % preset_id)

	var instructions = {}
	match preset_id:
		"attacking":
			instructions = {"defensive_line": "High", "team_tempo": "Fast", "pressing_intensity": "High"}
		"defensive":
			instructions = {"defensive_line": "Deep", "team_tempo": "Slow", "pressing_intensity": "Low"}
		"counter":
			instructions = {"defensive_line": "Deep", "team_tempo": "Fast", "build_up_style": "Direct"}

	current_instructions = instructions
	tactics_changed.emit(instructions)

	# Show feedback
	_show_message("'%s' 전술로 변경되었습니다" % preset_id)


func _on_continue_pressed():
	print("[HalftimeScreen] Continuing to second half")
	continue_match.emit()


func _show_message(text: String):
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "전술 변경"
	add_child(popup)
	popup.popup_centered(Vector2(350, 150))
	popup.confirmed.connect(popup.queue_free)


func get_tactical_changes() -> Dictionary:
	"""Return all tactical changes made during halftime"""
	return {"formation": current_formation, "instructions": current_instructions}
