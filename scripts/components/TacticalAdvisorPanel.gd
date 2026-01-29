extends Control

## TacticalAdvisorPanel - Pre-match tactical analysis and recommendations
## Shows squad fitness, formation suitability, and tactical suggestions

signal formation_recommended(formation: String)
signal recommendation_applied(recommendation: Dictionary)

# Analysis data
var analysis_result: Dictionary = {}
var current_squad: Array = []
var current_formation: String = "4-4-2"

# UI References
var squad_rating_label: Label = null
var formation_fitness_label: Label = null
var mismatch_container: VBoxContainer = null
var recommendations_container: VBoxContainer = null

# Rust Engine
var rust_engine: Node = null


func _ready():
	print("[TacticalAdvisorPanel] Initializing tactical advisor")

	rust_engine = get_node_or_null("/root/FootballRustEngine")

	_build_ui()


func _build_ui():
	# Main container
	var main_vbox = VBoxContainer.new()
	main_vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_vbox.add_theme_constant_override("separation", 15)
	add_child(main_vbox)

	# Margin
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_top", 20)
	margin.add_theme_constant_override("margin_bottom", 20)
	main_vbox.add_child(margin)

	var content_vbox = VBoxContainer.new()
	content_vbox.add_theme_constant_override("separation", 15)
	content_vbox.size_flags_vertical = Control.SIZE_EXPAND_FILL
	margin.add_child(content_vbox)

	# Title
	var title = Label.new()
	title.text = "🎯 전술 분석 및 조언"
	title.add_theme_font_size_override("font_size", 26)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content_vbox.add_child(title)

	# Squad overview section
	var overview_section = _create_overview_section()
	content_vbox.add_child(overview_section)

	# Separator
	var sep1 = HSeparator.new()
	content_vbox.add_child(sep1)

	# Position mismatch section
	var mismatch_section = _create_mismatch_section()
	content_vbox.add_child(mismatch_section)

	# Separator
	var sep2 = HSeparator.new()
	content_vbox.add_child(sep2)

	# Recommendations section
	var recommendations_section = _create_recommendations_section()
	content_vbox.add_child(recommendations_section)

	# Analyze button
	var analyze_btn = Button.new()
	analyze_btn.text = "🔍 다시 분석"
	analyze_btn.custom_minimum_size = Vector2(200, 50)
	analyze_btn.add_theme_font_size_override("font_size", 16)
	analyze_btn.pressed.connect(_on_analyze_pressed)
	content_vbox.add_child(analyze_btn)


func _create_overview_section() -> Control:
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 10)

	# Section title
	var title = Label.new()
	title.text = "📊 스쿼드 개요"
	title.add_theme_font_size_override("font_size", 20)
	section.add_child(title)

	# Stats grid
	var grid = GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 30)
	grid.add_theme_constant_override("v_separation", 8)
	section.add_child(grid)

	# Squad rating
	var rating_label = Label.new()
	rating_label.text = "스쿼드 평점:"
	rating_label.add_theme_font_size_override("font_size", 16)
	grid.add_child(rating_label)

	squad_rating_label = Label.new()
	squad_rating_label.text = "계산 중..."
	squad_rating_label.add_theme_font_size_override("font_size", 16)
	squad_rating_label.add_theme_color_override("font_color", Color(0.3, 0.8, 1.0))
	grid.add_child(squad_rating_label)

	# Formation fitness
	var fitness_label = Label.new()
	fitness_label.text = "포메이션 적합도:"
	fitness_label.add_theme_font_size_override("font_size", 16)
	grid.add_child(fitness_label)

	formation_fitness_label = Label.new()
	formation_fitness_label.text = "계산 중..."
	formation_fitness_label.add_theme_font_size_override("font_size", 16)
	formation_fitness_label.add_theme_color_override("font_color", Color(0.3, 1.0, 0.3))
	grid.add_child(formation_fitness_label)

	return section


func _create_mismatch_section() -> Control:
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 10)

	# Section title
	var title = Label.new()
	title.text = "⚠️ 포지션 미스매치"
	title.add_theme_font_size_override("font_size", 20)
	section.add_child(title)

	# Mismatch list
	mismatch_container = VBoxContainer.new()
	mismatch_container.add_theme_constant_override("separation", 5)
	section.add_child(mismatch_container)

	# Default message
	var default_msg = Label.new()
	default_msg.name = "DefaultMismatchMsg"
	default_msg.text = "분석 필요"
	default_msg.add_theme_font_size_override("font_size", 14)
	default_msg.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
	mismatch_container.add_child(default_msg)

	return section


func _create_recommendations_section() -> Control:
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 10)

	# Section title
	var title = Label.new()
	title.text = "💡 추천 전술"
	title.add_theme_font_size_override("font_size", 20)
	section.add_child(title)

	# Recommendations list
	recommendations_container = VBoxContainer.new()
	recommendations_container.add_theme_constant_override("separation", 8)
	section.add_child(recommendations_container)

	# Default message
	var default_msg = Label.new()
	default_msg.name = "DefaultRecommendMsg"
	default_msg.text = "분석 후 추천 전술이 표시됩니다"
	default_msg.add_theme_font_size_override("font_size", 14)
	default_msg.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
	recommendations_container.add_child(default_msg)

	return section


func analyze(squad: Array, formation: String):
	"""Analyze squad and formation"""
	current_squad = squad
	current_formation = formation

	print("[TacticalAdvisorPanel] Analyzing squad with formation: %s" % formation)

	# Call Rust API if available
	if rust_engine and rust_engine.has_method("analyze_tactics"):
		var result = rust_engine.analyze_tactics({"squad": squad, "formation": formation})
		if result.get("success", false):
			analysis_result = result.get("analysis", {})
			_update_display()
			return

	# Fallback: Generate mock analysis
	_generate_mock_analysis()
	_update_display()


func _generate_mock_analysis():
	"""Generate mock analysis when Rust API not available"""
	# Calculate average CA
	var total_ca = 0
	for player in current_squad:
		total_ca += player.get("current_ability", 50)

	var avg_ca = total_ca / max(current_squad.size(), 1)

	# Simple formation fitness calculation
	var fitness = 0.75  # Default

	# Mock mismatches
	var mismatches = []
	if current_squad.size() > 0:
		# Check if any player is out of position (simplified)
		for i in range(min(2, current_squad.size())):
			var player = current_squad[i]
			if randf() > 0.7:  # 30% chance of mismatch
				mismatches.append(
					{
						"player_name": player.get("name", "Player %d" % i),
						"assigned_position": "CB",
						"natural_position": "CM",
						"fitness_loss": 0.15
					}
				)

	# Generate recommendations
	var recommendations = []
	if avg_ca < 60:
		recommendations.append(
			{"priority": "High", "message": "스쿼드 전체 능력치가 낮습니다", "suggested_action": "선수 영입 또는 훈련 강화 권장"}
		)

	if fitness < 0.8:
		recommendations.append(
			{
				"priority": "Medium",
				"message": "현재 포메이션이 스쿼드에 최적화되지 않음",
				"suggested_action": "4-3-3 또는 4-2-3-1 포메이션 시도 권장"
			}
		)

	if mismatches.size() > 0:
		recommendations.append(
			{"priority": "High", "message": "%d명의 선수가 본 포지션이 아님" % mismatches.size(), "suggested_action": "포지션 재배치 필요"}
		)

	# Recommended formations
	var recommended_formations = ["4-3-3", "4-2-3-1", "3-5-2"]

	analysis_result = {
		"squad_rating": avg_ca,
		"formation_fitness": fitness,
		"position_mismatches": mismatches,
		"recommendations": recommendations,
		"recommended_formations": recommended_formations
	}


func _update_display():
	"""Update all UI elements with analysis results"""

	# Update squad rating
	if squad_rating_label:
		var rating = analysis_result.get("squad_rating", 0)
		squad_rating_label.text = "%.1f CA" % rating

		# Color based on rating
		if rating >= 70:
			squad_rating_label.add_theme_color_override("font_color", Color(0.3, 1.0, 0.3))
		elif rating >= 50:
			squad_rating_label.add_theme_color_override("font_color", Color(1.0, 0.8, 0.2))
		else:
			squad_rating_label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3))

	# Update formation fitness
	if formation_fitness_label:
		var fitness = analysis_result.get("formation_fitness", 0)
		formation_fitness_label.text = "%.0f%%" % (fitness * 100)

		# Color based on fitness
		if fitness >= 0.8:
			formation_fitness_label.add_theme_color_override("font_color", Color(0.3, 1.0, 0.3))
		elif fitness >= 0.6:
			formation_fitness_label.add_theme_color_override("font_color", Color(1.0, 0.8, 0.2))
		else:
			formation_fitness_label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3))

	# Update mismatches
	_update_mismatches()

	# Update recommendations
	_update_recommendations()


func _update_mismatches():
	if not mismatch_container:
		return

	# Clear existing
	for child in mismatch_container.get_children():
		child.queue_free()

	var mismatches = analysis_result.get("position_mismatches", [])

	if mismatches.is_empty():
		var no_mismatch = Label.new()
		no_mismatch.text = "✅ 포지션 미스매치 없음"
		no_mismatch.add_theme_font_size_override("font_size", 14)
		no_mismatch.add_theme_color_override("font_color", Color(0.3, 0.8, 0.3))
		mismatch_container.add_child(no_mismatch)
		return

	for mismatch in mismatches:
		var row = HBoxContainer.new()
		row.add_theme_constant_override("separation", 10)

		var icon = Label.new()
		icon.text = "⚠️"
		row.add_child(icon)

		var text = Label.new()
		text.text = (
			"%s: %s → %s (-%d%%)"
			% [
				mismatch.get("player_name", "Unknown"),
				mismatch.get("natural_position", "?"),
				mismatch.get("assigned_position", "?"),
				int(mismatch.get("fitness_loss", 0) * 100)
			]
		)
		text.add_theme_font_size_override("font_size", 14)
		text.add_theme_color_override("font_color", Color(1.0, 0.6, 0.2))
		row.add_child(text)

		mismatch_container.add_child(row)


func _update_recommendations():
	if not recommendations_container:
		return

	# Clear existing
	for child in recommendations_container.get_children():
		child.queue_free()

	var recommendations = analysis_result.get("recommendations", [])
	var formations = analysis_result.get("recommended_formations", [])

	if recommendations.is_empty() and formations.is_empty():
		var no_rec = Label.new()
		no_rec.text = "추천 사항 없음"
		no_rec.add_theme_font_size_override("font_size", 14)
		no_rec.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
		recommendations_container.add_child(no_rec)
		return

	# Add recommendations
	for rec in recommendations:
		var rec_panel = _create_recommendation_item(rec)
		recommendations_container.add_child(rec_panel)

	# Add formation recommendations
	if not formations.is_empty():
		var sep = HSeparator.new()
		recommendations_container.add_child(sep)

		var formation_title = Label.new()
		formation_title.text = "추천 포메이션:"
		formation_title.add_theme_font_size_override("font_size", 16)
		recommendations_container.add_child(formation_title)

		var btn_container = HBoxContainer.new()
		btn_container.add_theme_constant_override("separation", 10)

		for formation in formations:
			var btn = Button.new()
			btn.text = formation
			btn.custom_minimum_size = Vector2(80, 40)
			btn.pressed.connect(_on_formation_btn_pressed.bind(formation))
			btn_container.add_child(btn)

		recommendations_container.add_child(btn_container)


func _create_recommendation_item(rec: Dictionary) -> Control:
	var panel = PanelContainer.new()

	var style = StyleBoxFlat.new()
	var priority = rec.get("priority", "Low")
	match priority:
		"High":
			style.bg_color = Color(0.8, 0.2, 0.2, 0.2)
		"Medium":
			style.bg_color = Color(0.8, 0.6, 0.2, 0.2)
		_:
			style.bg_color = Color(0.3, 0.3, 0.3, 0.2)

	style.corner_radius_top_left = 5
	style.corner_radius_top_right = 5
	style.corner_radius_bottom_left = 5
	style.corner_radius_bottom_right = 5
	panel.add_theme_stylebox_override("panel", style)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 5)
	panel.add_child(vbox)

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 10)
	margin.add_theme_constant_override("margin_right", 10)
	margin.add_theme_constant_override("margin_top", 8)
	margin.add_theme_constant_override("margin_bottom", 8)

	var content = VBoxContainer.new()
	content.add_theme_constant_override("separation", 3)

	var msg = Label.new()
	msg.text = "[%s] %s" % [priority, rec.get("message", "")]
	msg.add_theme_font_size_override("font_size", 14)
	content.add_child(msg)

	var action = Label.new()
	action.text = "→ %s" % rec.get("suggested_action", "")
	action.add_theme_font_size_override("font_size", 12)
	action.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	content.add_child(action)

	margin.add_child(content)
	panel.add_child(margin)

	return panel


func _on_analyze_pressed():
	analyze(current_squad, current_formation)


func _on_formation_btn_pressed(formation: String):
	print("[TacticalAdvisorPanel] Formation recommended: %s" % formation)
	formation_recommended.emit(formation)
	_show_message("'%s' 포메이션이 추천되었습니다" % formation)


func _show_message(text: String):
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "전술 조언"
	add_child(popup)
	popup.popup_centered(Vector2(350, 150))
	popup.confirmed.connect(popup.queue_free)


func get_analysis() -> Dictionary:
	return analysis_result.duplicate()
