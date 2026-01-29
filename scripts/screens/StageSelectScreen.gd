extends Control

## Stage Select Screen
##
## Displays all 350 stages in a scrollable list.
## Players can select unlocked stages to play.

# UI nodes
@onready var title_label: Label = $Header/TitleLabel
@onready var progress_label: Label = $Header/ProgressLabel
@onready var scroll_container: ScrollContainer = $ScrollContainer
@onready var stage_list_container: VBoxContainer = $ScrollContainer/StageList
@onready var back_button: Button = $Header/TopBar/BackButton

# State
var selected_stage_id: int = -1

# BM Two-Track: Team snapshot UI reference
@onready var team_info_label: Label = $Header/TeamInfoLabel if has_node("Header/TeamInfoLabel") else null


func _ready():
	print("[StageSelectScreen] Initializing...")

	# Connect signals
	back_button.pressed.connect(_on_back_pressed)

	# BM Two-Track: Connect to StageManager snapshot changes
	if StageManager:
		StageManager.team_snapshot_changed.connect(_on_team_snapshot_changed)

	# Wait for StageManager to be ready
	await get_tree().process_frame

	# Initialize UI
	update_progress_label()
	update_team_info_display()
	populate_stage_list()

	print("[StageSelectScreen] Ready!")


# ============================================================================
# UI Population
# ============================================================================


func populate_stage_list():
	"""Populate the stage list with all stages"""
	print("[StageSelectScreen] Populating stage list...")

	# Clear existing buttons
	for child in stage_list_container.get_children():
		child.queue_free()

	# Get all stage info
	var all_stages = StageManager.get_all_stage_info()

	# Group stages by tier for better organization
	var current_tier_label: Label = null
	var last_tier = ""

	for stage_info in all_stages:
		var tier = stage_info["tier"]

		# Add tier header if changed
		if tier != last_tier:
			var tier_header = create_tier_header(tier)
			stage_list_container.add_child(tier_header)
			last_tier = tier

		# Create stage button
		var stage_button = create_stage_button(stage_info)
		stage_list_container.add_child(stage_button)

	print("[StageSelectScreen] Created %d stage buttons" % all_stages.size())


func create_tier_header(tier: String) -> PanelContainer:
	"""Create a header label for tier grouping"""
	var panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(0, 40)

	var label = Label.new()
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER

	match tier:
		"youth":
			label.text = "⚽ Youth Teams (Easiest)"
			label.add_theme_color_override("font_color", Color(0.6, 0.8, 1.0))
		"b_team":
			label.text = "🔵 B-Teams (Medium)"
			label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.6))
		"first_team":
			label.text = "🏆 First Teams (Hard)"
			label.add_theme_color_override("font_color", Color(1.0, 0.6, 0.6))
		_:
			label.text = "⚽ Unknown Tier"

	panel.add_child(label)
	return panel


func create_stage_button(stage_info: Dictionary) -> Button:
	"""Create a button for a stage"""
	var button = Button.new()
	button.custom_minimum_size = Vector2(0, 80)
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT

	var stage_id = stage_info["stage_id"]
	var club_name = stage_info["club_name"]
	var avg_ca = stage_info["avg_ca"]
	var is_unlocked = stage_info["is_unlocked"]
	var is_completed = stage_info["is_completed"]

	# Button text
	var status_icon = ""
	if is_completed:
		status_icon = "✅ "
	elif is_unlocked:
		status_icon = "▶ "
	else:
		status_icon = "🔒 "

	button.text = "%sStage %d: %s (CA %.1f)" % [status_icon, stage_id, club_name, avg_ca]

	# Button style
	if not is_unlocked:
		button.disabled = true
		button.modulate = Color(0.5, 0.5, 0.5, 0.7)
	elif is_completed:
		button.modulate = Color(0.7, 1.0, 0.7, 1.0)

	# Connect signal
	button.pressed.connect(_on_stage_button_pressed.bind(stage_id))

	# Store stage_id in metadata for easy access
	button.set_meta("stage_id", stage_id)

	return button


func update_progress_label():
	"""Update the progress label at the top"""
	var stats = StageManager.get_stats()

	progress_label.text = (
		"Progress: %d/%d (%.1f%%)" % [stats["completed_stages"], stats["total_stages"], stats["completion_percentage"]]
	)

	title_label.text = "⚽ Stage Select - Unlocked: Stage %d" % stats["unlocked_stage"]


func update_team_info_display() -> void:
	"""Update the team info display (BM Two-Track snapshot info)"""
	# Create label dynamically if doesn't exist
	if not team_info_label and has_node("Header"):
		team_info_label = Label.new()
		team_info_label.name = "TeamInfoLabel"
		team_info_label.add_theme_font_size_override("font_size", 14)
		$Header.add_child(team_info_label)

	if not team_info_label:
		return

	if StageManager and StageManager.has_team_snapshot():
		var snapshot := StageManager.get_selected_team_snapshot()
		team_info_label.text = (
			"🏆 선택된 팀: %s (OVR %.1f, %d명)"
			% [
				snapshot.get("team_name", "Unknown"),
				float(snapshot.get("avg_overall", 0)),
				int(snapshot.get("player_count", 0))
			]
		)
		team_info_label.add_theme_color_override("font_color", Color(0.4, 0.9, 0.4))
	else:
		team_info_label.text = "⚠️ 팀 미선택 - 명예의 전당에서 팀을 선택하세요"
		team_info_label.add_theme_color_override("font_color", Color(1.0, 0.8, 0.4))


func _on_team_snapshot_changed(_snapshot: Dictionary) -> void:
	"""Handle team snapshot change from StageManager"""
	update_team_info_display()


# ============================================================================
# Button Handlers
# ============================================================================


func _on_stage_button_pressed(stage_id: int):
	"""Handle stage button press"""
	print("[StageSelectScreen] Stage %d selected" % stage_id)

	selected_stage_id = stage_id

	# Show confirmation dialog
	show_stage_confirmation(stage_id)


func show_stage_confirmation(stage_id: int):
	"""Show confirmation dialog before starting stage"""
	var stage_info = StageManager.get_stage_info(stage_id)

	# BM Two-Track: Check if team snapshot is selected
	var has_snapshot := StageManager.has_team_snapshot() if StageManager else false
	var team_info_text := ""

	if has_snapshot:
		var snapshot := StageManager.get_selected_team_snapshot()
		team_info_text = (
			"\n\n🏆 내 팀: %s (OVR %.1f)" % [snapshot.get("team_name", "Unknown"), float(snapshot.get("avg_overall", 0))]
		)
	else:
		team_info_text = "\n\n⚠️ 팀 미선택 - 명예의 전당에서 팀을 먼저 선택하세요!"

	# Create confirmation dialog
	var dialog = AcceptDialog.new()
	dialog.title = "Start Stage?"
	dialog.dialog_text = (
		"Stage %d: %s\nAverage CA: %.1f%s\n\nAre you ready to start this match?"
		% [stage_id, stage_info["club_name"], stage_info["avg_ca"], team_info_text]
	)

	# Add cancel button
	dialog.add_cancel_button("Cancel")

	# Connect signals
	dialog.confirmed.connect(_on_stage_confirmed.bind(stage_id))
	dialog.canceled.connect(_on_stage_canceled)

	# Show dialog
	add_child(dialog)
	dialog.popup_centered()


func _on_stage_confirmed(stage_id: int):
	"""Handle stage confirmation"""
	print("[StageSelectScreen] Starting stage %d" % stage_id)

	# Start the stage
	if StageManager.start_stage(stage_id):
		# Navigate to match simulation screen
		start_match(stage_id)
	else:
		show_error("Failed to start stage %d" % stage_id)


func _on_stage_canceled():
	"""Handle stage cancellation"""
	print("[StageSelectScreen] Stage selection canceled")
	selected_stage_id = -1


func start_match(stage_id: int):
	"""Start the match for this stage"""
	print("[StageSelectScreen] Starting match for stage %d" % stage_id)

	# Store stage ID for result handling
	get_tree().root.set_meta("current_stage_id", stage_id)

	# BM Two-Track: Store player team data from snapshot or default
	var player_team_data := {}
	if StageManager and StageManager.has_team_snapshot():
		player_team_data = StageManager.convert_snapshot_to_match_team()
		print("[StageSelectScreen] Using Hall of Fame snapshot: %s" % player_team_data.get("club_name", "Unknown"))
	elif StageManager:
		player_team_data = StageManager.get_match_ready_player_team()
		print("[StageSelectScreen] Using default player team")

	# Store player team data for match
	get_tree().root.set_meta("stage_player_team", player_team_data)

	# Connect to match finished signal (if not already connected)
	if not MatchSimulationManager.match_completed.is_connected(_on_match_completed):
		MatchSimulationManager.match_completed.connect(_on_match_completed)

	# TODO: Start match via MatchSimulationManager or MatchManager.
	# For now, we navigate to the dedicated match simulation UI so that
	# stage-based flows can share the same shell as MVP matches.
	var match_scene_path := "res://scenes/ui/match_simulation_screen.tscn"
	if has_node("/root/SceneLoader"):
		var loader := get_node("/root/SceneLoader")
		if loader.has_method("load_scene"):
			loader.call("load_scene", match_scene_path)
		else:
			get_tree().change_scene_to_file(match_scene_path)
	else:
		get_tree().change_scene_to_file(match_scene_path)

	print("[StageSelectScreen] Match screen opened for stage %d" % stage_id)


func convert_team_to_match_format(team_data: Dictionary) -> Dictionary:
	"""Convert StageManager team format to MatchSimulationManager format"""
	var squad = team_data.get("squad", [])

	# Take first 11 players for match
	var match_players = []
	for i in range(min(11, squad.size())):
		var player = squad[i]
		match_players.append(
			{
				"name": player.get("name", "Player %d" % (i + 1)),
				"position": player.get("position", "CM"),
				"overall": int(player.get("ca", 60) / 2.0)  # Convert CA (1-195) to overall (1-100)
			}
		)

	return {"name": team_data.get("club_name", "Unknown Team"), "formation": "T442", "players": match_players}  # Default formation


func _on_match_completed(success: bool, result: Dictionary) -> void:
	if success:
		_on_match_finished(result)
	else:
		show_error("Match simulation failed")


func _on_match_finished(result: Dictionary) -> void:
	"""Handle match finish"""
	print("[StageSelectScreen] Match finished: %s" % str(result))

	# Get stage ID
	var stage_id = get_tree().root.get_meta("current_stage_id", -1)
	if stage_id == -1:
		print("[StageSelectScreen] ERROR: No stage ID stored!")
		return

	# Complete the stage in StageManager
	StageManager.complete_stage(stage_id, result)

	# Refresh UI
	update_progress_label()
	populate_stage_list()

	# Show result dialog
	show_match_result(stage_id, result)


func show_match_result(stage_id: int, result: Dictionary):
	"""Show match result dialog"""
	var stage_info = StageManager.get_stage_info(stage_id)
	var is_victory = result.get("winner", "") == "home"

	var dialog = AcceptDialog.new()
	dialog.title = "Match Result"

	if is_victory:
		dialog.dialog_text = (
			"🎉 VICTORY!\n\nStage %d: %s\nScore: %d - %d\n\nNext stage unlocked!"
			% [stage_id, stage_info["club_name"], result.get("score_home", 0), result.get("score_away", 0)]
		)
	else:
		dialog.dialog_text = (
			"😔 DEFEAT\n\nStage %d: %s\nScore: %d - %d\n\nTry again!"
			% [stage_id, stage_info["club_name"], result.get("score_home", 0), result.get("score_away", 0)]
		)

	dialog.confirmed.connect(func(): dialog.queue_free())
	add_child(dialog)
	dialog.popup_centered()


func show_error(message: String):
	"""Show error dialog"""
	var dialog = AcceptDialog.new()
	dialog.title = "Error"
	dialog.dialog_text = message
	dialog.confirmed.connect(func(): dialog.queue_free())
	add_child(dialog)
	dialog.popup_centered()


func _on_back_pressed():
	"""Handle back button press"""
	print("[StageSelectScreen] Back button pressed")

	# TODO: Navigate back to main menu
	# Example:
	# SceneLoader.load_scene("res://scenes/TitleScreenImproved.tscn")

	# For now, just close the screen
	queue_free()


# ============================================================================
# Public Interface
# ============================================================================


func refresh():
	"""Refresh the stage list (call after returning from match)"""
	update_progress_label()
	populate_stage_list()
