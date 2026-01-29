extends Control

## Formation Selection Screen Controller
## Integrates FormationDisplay and FormationSelector components
## Provides AI recommendations and formation comparison features

signal formation_applied(formation_id: String)
signal screen_closed

# UI References
@onready var formation_selector = $MainContainer/ContentContainer/LeftPanel/MarginContainer/FormationSelector
@onready var formation_display = $MainContainer/ContentContainer/RightPanel/MarginContainer/RightContent/FormationDisplay
@onready var formation_name_label = $MainContainer/ContentContainer/RightPanel/MarginContainer/RightContent/FormationName
@onready
var fitness_label = $MainContainer/ContentContainer/RightPanel/MarginContainer/RightContent/StatsContainer/FitnessLabel
@onready
var style_label = $MainContainer/ContentContainer/RightPanel/MarginContainer/RightContent/StatsContainer/StyleLabel
@onready
var complexity_label = $MainContainer/ContentContainer/RightPanel/MarginContainer/RightContent/StatsContainer/ComplexityLabel
@onready var apply_button = $MainContainer/Footer/ApplyButton

# State
var current_formation_id: String = "T442"
var selected_formation_id: String = ""
var squad_players: Array = []
var rust_engine: Node = null


func _ready():
	print("[FormationSelectionScreen] Initializing...")

	# Get Rust engine reference
	rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine:
		print("[FormationSelectionScreen] Warning: FootballRustEngine not found")

	# Connect FormationDisplay to FormationSelector
	if formation_selector and formation_display:
		formation_selector.set_formation_display_reference(formation_display)
		print("[FormationSelectionScreen] FormationDisplay connected to FormationSelector")

	# Load squad data
	_load_squad_data()

	# Set initial formation
	if current_formation_id != "":
		_update_formation_display(current_formation_id)

	print("[FormationSelectionScreen] Ready!")


func _load_squad_data():
	"""Load current squad data from MyTeamData"""
	if not has_node("/root/MyTeamData"):
		print("[FormationSelectionScreen] Warning: MyTeamData not found")
		return

	var my_team_data = get_node("/root/MyTeamData")
	squad_players = my_team_data.saved_players if my_team_data.saved_players != null else []

	# Get current formation
	if my_team_data.current_team != null:
		var current_team = my_team_data.current_team
		if current_team.has("formation"):
			current_formation_id = current_team.formation

	print("[FormationSelectionScreen] Loaded %d players from squad" % squad_players.size())


func _update_formation_display(formation_id: String):
	"""Update formation display and stats"""
	if not rust_engine or not rust_engine.is_ready():
		print("[FormationSelectionScreen] Rust engine not ready")
		return

	# Update formation display
	if formation_display:
		formation_display.set_formation(formation_id)

	# Get formation details
	var result = rust_engine.get_formation_details(formation_id)
	if not result.get("success", false):
		print("[FormationSelectionScreen] Error getting formation details: %s" % result.get("error", "Unknown"))
		return

	var formation = result.get("formation", {})

	# Update formation name label
	var name_en = formation.get("name_en", formation_id)
	var name_ko = formation.get("name_ko", formation_id)
	formation_name_label.text = "현재 포메이션: %s (%s)" % [name_ko, name_en]

	# Update tactical style
	var style = formation.get("tactical_style", "Unknown")
	style_label.text = "스타일: %s" % style

	# Calculate fitness if we have squad data
	if squad_players.size() >= 11:
		_calculate_and_display_fitness(formation_id)
	else:
		fitness_label.text = "적합도: --"
		fitness_label.add_theme_color_override("font_color", Color.WHITE)

	# Set complexity (placeholder - could be calculated)
	complexity_label.text = "복잡도: 중간"

	print("[FormationSelectionScreen] Updated display for %s" % formation_id)


func _calculate_and_display_fitness(formation_id: String):
	"""Calculate and display formation fitness for current squad"""
	if not rust_engine or not rust_engine.is_ready():
		return

	# Calculate fitness
	var result = rust_engine.calculate_formation_fitness(formation_id, squad_players)

	if not result.get("success", false):
		fitness_label.text = "적합도: 계산 실패"
		fitness_label.add_theme_color_override("font_color", Color.RED)
		return

	var fitness_score = result.get("fitness_score", 0.0)
	var fitness_percentage = fitness_score * 100

	# Update label with color coding
	fitness_label.text = "적합도: %.1f%%" % fitness_percentage

	var color: Color
	if fitness_score < 0.5:
		color = Color.RED
	elif fitness_score < 0.7:
		color = Color.YELLOW
	else:
		color = Color.GREEN

	fitness_label.add_theme_color_override("font_color", color)


# ==============================================================================
# Signal Handlers
# ==============================================================================


func _on_back_pressed():
	"""Handle back button press"""
	print("[FormationSelectionScreen] Back pressed")
	screen_closed.emit()
	queue_free()


func _on_recommend_pressed():
	"""Handle AI recommendation button press"""
	print("[FormationSelectionScreen] AI Recommend pressed")

	if not rust_engine or not rust_engine.is_ready():
		_show_message("오류", "AI 추천 기능을 사용할 수 없습니다.")
		return

	if squad_players.size() < 11:
		_show_message("스쿼드 부족", "AI 추천을 받으려면 최소 11명의 선수가 필요합니다.")
		return

	# Get AI recommendations
	var result = rust_engine.recommend_formations(squad_players)

	if not result.get("success", false):
		_show_message("오류", "AI 추천 중 오류가 발생했습니다: %s" % result.get("error", "Unknown"))
		return

	var recommendations = result.get("recommendations", [])

	if recommendations.size() == 0:
		_show_message("추천 없음", "추천 가능한 포메이션이 없습니다.")
		return

	# Show top recommendation
	var top_recommendation = recommendations[0]
	var formation_id = top_recommendation.get("id", "")
	var score = top_recommendation.get("score", 0.0)
	var reason = top_recommendation.get("reason", "")

	_show_message(
		"AI 추천 포메이션",
		(
			"추천 포메이션: %s\n점수: %.1f%%\n\n이유: %s\n\n적용하시겠습니까?"
			% [top_recommendation.get("name_ko", formation_id), score * 100, reason]
		),
		true
	)

	# Store for later application
	selected_formation_id = formation_id
	_update_formation_display(formation_id)


func _on_formation_selected(formation_id: String, success: bool, feedback_data: Dictionary):
	"""Handle formation selection from FormationSelector"""
	print("[FormationSelectionScreen] Formation selected: %s (success: %s)" % [formation_id, success])

	if success:
		selected_formation_id = formation_id
		_update_formation_display(formation_id)
		apply_button.disabled = false
	else:
		_show_message("오류", "포메이션 선택 실패: %s" % feedback_data.get("error", "Unknown"))


func _on_formation_preview_started(formation_id: String):
	"""Handle formation preview start"""
	print("[FormationSelectionScreen] Preview started: %s" % formation_id)
	_update_formation_display(formation_id)


func _on_formation_preview_ended():
	"""Handle formation preview end"""
	print("[FormationSelectionScreen] Preview ended")
	# Restore current formation display
	if selected_formation_id != "":
		_update_formation_display(selected_formation_id)
	else:
		_update_formation_display(current_formation_id)


func _on_compare_pressed():
	"""Handle compare button press"""
	print("[FormationSelectionScreen] Compare pressed")

	# TODO: Implement formation comparison view
	# This would show multiple formations side-by-side with stats
	_show_message("준비 중", "포메이션 비교 기능은 곧 제공될 예정입니다.")


func _on_apply_pressed():
	"""Handle apply button press"""
	if selected_formation_id == "":
		_show_message("선택 필요", "적용할 포메이션을 먼저 선택해주세요.")
		return

	print("[FormationSelectionScreen] Applying formation: %s" % selected_formation_id)

	# Save to MyTeamData
	if has_node("/root/MyTeamData"):
		var my_team_data = get_node("/root/MyTeamData")
		if my_team_data.has_method("set_team_formation"):
			my_team_data.set_team_formation(selected_formation_id)
			current_formation_id = selected_formation_id

			formation_applied.emit(selected_formation_id)
			_show_message("적용 완료", "포메이션이 성공적으로 적용되었습니다!")
		else:
			_show_message("오류", "포메이션 저장 기능을 사용할 수 없습니다.")
	else:
		_show_message("오류", "MyTeamData를 찾을 수 없습니다.")


# ==============================================================================
# Helper Functions
# ==============================================================================


func _show_message(title: String, text: String, is_question: bool = false):
	"""Show message popup"""
	var dialog: AcceptDialog

	if is_question:
		dialog = ConfirmationDialog.new()
		dialog.confirmed.connect(_on_recommendation_confirmed)
	else:
		dialog = AcceptDialog.new()

	dialog.title = title
	dialog.dialog_text = text
	add_child(dialog)
	dialog.popup_centered(Vector2(500, 300))
	dialog.close_requested.connect(dialog.queue_free)


func _on_recommendation_confirmed():
	"""Handle recommendation confirmation"""
	if selected_formation_id != "":
		_on_apply_pressed()


# ==============================================================================
# Public API
# ==============================================================================


func set_squad_players(players: Array):
	"""Set squad players from external source"""
	squad_players = players
	print("[FormationSelectionScreen] Squad players set: %d players" % players.size())


func set_current_formation(formation_id: String):
	"""Set current formation from external source"""
	current_formation_id = formation_id
	_update_formation_display(formation_id)
	print("[FormationSelectionScreen] Current formation set: %s" % formation_id)


func get_selected_formation() -> String:
	"""Get currently selected formation"""
	return selected_formation_id if selected_formation_id != "" else current_formation_id
