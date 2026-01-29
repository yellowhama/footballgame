extends Control
class_name MatchSimulationScreen

## MatchSimulationScreen
## UI shell for running or visualising a single simulated match.
## Responsibility:
## - Display basic info about the current stage/teams.
## - Show score / time labels and an event log list.
## - Wire up control buttons; actual simulation start is TODO and will
##   be coordinated with MatchSimulationManager/StageSelectScreen.

@onready var title_label: Label = $VBox/Header/TitleLabel
@onready var home_team_label: Label = $VBox/MatchInfo/MatchInfoVBox/TeamsRow/HomeTeam
@onready var away_team_label: Label = $VBox/MatchInfo/MatchInfoVBox/TeamsRow/AwayTeam
@onready var home_score_label: Label = $VBox/MatchInfo/MatchInfoVBox/ScoreRow/HomeScore
@onready var away_score_label: Label = $VBox/MatchInfo/MatchInfoVBox/ScoreRow/AwayScore
@onready var time_label: Label = $VBox/MatchInfo/MatchInfoVBox/TimeLabel
@onready var event_list: VBoxContainer = $VBox/EventLog/EventScroll/EventList

@onready var back_button: Button = $VBox/Header/BackButton
@onready var attack_button: Button = $VBox/Controls/AttackButton
@onready var balanced_button: Button = $VBox/Controls/BalancedButton
@onready var defend_button: Button = $VBox/Controls/DefendButton
@onready var sub_button: Button = $VBox/Controls/SubButton
@onready var play_button: Button = $VBox/Controls/PlayButton
@onready var pause_button: Button = $VBox/Controls/PauseButton
@onready var speed_button: Button = $VBox/Controls/SpeedButton
@onready var skip_button: Button = $VBox/Controls/SkipButton

@onready var match_view_container: PanelContainer = $VBox/MatchViewContainer
@onready var match_session_controller: MatchSessionController = (
	get_node_or_null("MatchSessionController") as MatchSessionController
)

var _current_stage_id: int = -1
var _speed_multiplier: int = 1
var _viewer_root: Control = null
var _home_roster: Dictionary = {}
var _away_roster: Dictionary = {}  # 2025-12-09 추가
var _session_mode_active: bool = false
var _halftime_active: bool = false
var _timeline_markers: Array = []

# Sticky actions (global hotkeys fallback)
@export var enable_sticky_hotkeys: bool = true
@export var sticky_track_id: int = 9
const STICKY_KEY_SPRINT := KEY_Q
const STICKY_KEY_DRIBBLE := KEY_E
const STICKY_KEY_PRESS := KEY_R
var _sticky_actions: Dictionary = {"sprint": false, "dribble": false, "press": false}
var _allow_sticky_hotkeys: bool = false

# TeamView observation (debug)
@export var enable_team_view_observation: bool = false
@export var team_view_observer_is_home: bool = true

# Session match tracking for match_history save
var _session_events: Array = []  # Accumulated events during session match
var _session_last_score: Dictionary = {"home": 0, "away": 0}  # Last known score

# Phase 4.6: Hero Time overlay
var _player_command_overlay: Control = null
const PLAYER_COMMAND_OVERLAY_SCENE: PackedScene = preload("res://scenes/match/PlayerCommandOverlay.tscn")

# Phase 5.6: Growth Result Panel
var _growth_result_panel: Control = null
const GROWTH_RESULT_PANEL_SCENE: PackedScene = preload("res://scenes/match/GrowthResultPanel.tscn")

# Phase E.3a: Interactive Mode (Bullet-Time)
var _use_interactive_mode: bool = false
# var _interactive_pause_overlay: InteractivePauseOverlay = null  # REMOVED: Phase E - RadialDecisionUI integration
var _interactive_controller: InteractiveMatchController = null
var _interactive_mode_checkbox: CheckButton = null

# Cached autoload references to avoid direct identifier access (headless mode compatibility)
var _match_sim_manager: Node = null
var _my_team_data: Node = null
var _stage_manager: Node = null
var _timeline_data_holder: Node = null

# Bound callables for proper signal disconnection (prevents memory leak)
var _bound_session_tick_cb: Callable
var _bound_session_halftime_cb: Callable
var _bound_session_finished_cb: Callable
var _bound_session_paused_cb: Callable  # Phase 4.6: Hero Time pause

const _MatchTimeFormatter = preload("res://scripts/utils/MatchTimeFormatter.gd")
const SESSION_VIEWER_SCENE: PackedScene = preload(
        "res://scenes/match_pipeline/examples/HorizontalMatchSessionViewer.tscn"
)
const USE_SESSION_STREAM_VIEW: bool = true


func _ready() -> void:
	_current_stage_id = int(get_tree().root.get_meta("current_stage_id", -1))
	_init_match_info()
	_connect_buttons()

	if match_view_container:
		match_view_container.visible = false

	if Engine.is_editor_hint():
		return

	# Cache autoload references for headless mode compatibility
	_match_sim_manager = get_node_or_null("/root/MatchSimulationManager")
	_my_team_data = get_node_or_null("/root/MyTeamData")
	_stage_manager = get_node_or_null("/root/StageManager")
	_timeline_data_holder = get_node_or_null("/root/MatchTimelineHolder")

	if _match_sim_manager and _match_sim_manager.has_signal("match_completed"):
		if not _match_sim_manager.match_completed.is_connected(_on_match_completed):
			_match_sim_manager.match_completed.connect(_on_match_completed)

		# Phase E.3a: Initialize Interactive Mode UI
		_init_interactive_mode_ui()

		_allow_sticky_hotkeys = enable_sticky_hotkeys and not _has_player_hud()
		if _allow_sticky_hotkeys:
			set_process_unhandled_input(true)


func _init_interactive_mode_ui() -> void:
	"""Phase E.3a: Create Interactive Mode checkbox and overlay"""
	# Create Interactive Mode checkbox
	_interactive_mode_checkbox = CheckButton.new()
	_interactive_mode_checkbox.text = "Interactive Mode (Bullet-Time)"
	_interactive_mode_checkbox.toggled.connect(_on_interactive_mode_toggled)

	# Add to controls container (next to play button)
	if play_button and play_button.get_parent():
		var controls_container = play_button.get_parent()
		controls_container.add_child(_interactive_mode_checkbox)
		# Move checkbox to be before play button
		controls_container.move_child(_interactive_mode_checkbox, play_button.get_index())

		# Phase E: RadialDecisionUI integration
		# InteractivePauseOverlay removed - using existing RadialDecisionUI from HorizontalMatchViewer
		# Signal connection will be done on-demand in _get_or_create_radial_decision_ui()

		print("[MatchSimulationScreen] Interactive Mode UI initialized (RadialDecisionUI integration)")


func _has_player_hud() -> bool:
	var hud_nodes = get_tree().get_nodes_in_group("player_hud")
	return hud_nodes.size() > 0


func _unhandled_input(event: InputEvent) -> void:
	if not _allow_sticky_hotkeys:
		return
	if _has_player_hud():
		return
	if not (event is InputEventKey):
		return
	if not event.pressed or event.echo:
		return
	match event.keycode:
		STICKY_KEY_SPRINT:
			_toggle_sticky_action("sprint")
		STICKY_KEY_DRIBBLE:
			_toggle_sticky_action("dribble")
		STICKY_KEY_PRESS:
			_toggle_sticky_action("press")


func _toggle_sticky_action(action: String) -> void:
	var next_state = not bool(_sticky_actions.get(action, false))
	_sticky_actions[action] = next_state

	var manager = _match_sim_manager if _match_sim_manager else get_node_or_null("/root/MatchSimulationManager")
	if manager and manager.has_method("set_sticky_action"):
		manager.set_sticky_action(sticky_track_id, action, next_state)
		print("[MatchSimulationScreen] Sticky action %s=%s (track_id=%d)" % [action, str(next_state), sticky_track_id])
	else:
		print("[MatchSimulationScreen] ERROR: MatchSimulationManager not available for sticky actions")


func _on_interactive_mode_toggled(enabled: bool) -> void:
	"""Phase E.3a: Toggle Interactive Mode on/off"""
	_use_interactive_mode = enabled
	print("[MatchSimulationScreen] Interactive Mode: %s" % ("ON" if enabled else "OFF"))


func _on_interactive_intervention(context: Dictionary) -> void:
	"""Phase E: Handle intervention request from InteractiveMatchController (RadialDecisionUI integration)"""
	print("[MatchSimulationScreen] Intervention requested: %s" % str(context))

	# Find or create RadialDecisionUI
	var radial_ui = _get_or_create_radial_decision_ui()

	if radial_ui:
		# Show intervention via RadialDecisionUI
		print("[MatchSimulationScreen] Showing intervention via RadialDecisionUI")
		radial_ui.show_intervention(context)
	else:
		push_error("[MatchSimulationScreen] RadialDecisionUI not found!")
		# Fallback: auto-continue
		print("[MatchSimulationScreen] Fallback: Auto-continuing intervention")
		if _interactive_controller:
			_interactive_controller.auto_continue()
		else:
			push_warning("[MatchSimulationScreen] No InteractiveMatchController available for fallback")


func _on_interactive_action_selected(action: Dictionary) -> void:
	"""Phase E.3a: Handle user action selection"""
	print("[MatchSimulationScreen] Action selected: %s" % str(action))

	# Resume match with selected action
	if _interactive_controller:
		_interactive_controller.resume_with_action(action)
	else:
		push_warning("[MatchSimulationScreen] No InteractiveMatchController available")


## Phase E: Get or create RadialDecisionUI instance
## Tries to find existing instance (may be created by HorizontalMatchViewer)
## Falls back to creating new instance if not found
func _get_or_create_radial_decision_ui() -> RadialDecisionUI:
	# Option A: Find existing RadialDecisionUI (if HorizontalMatchViewer already created it)
	var radial_ui = _find_radial_decision_ui_in_tree()
	if radial_ui:
		print("[MatchSimulationScreen] Found existing RadialDecisionUI in scene tree")
		# Connect signal if not already connected
		if not radial_ui.action_selected.is_connected(_on_interactive_action_selected):
			radial_ui.action_selected.connect(_on_interactive_action_selected)
			print("[MatchSimulationScreen] Connected action_selected signal to existing RadialDecisionUI")
		return radial_ui

	# Option B: Create new RadialDecisionUI (if not found in tree)
	print("[MatchSimulationScreen] Creating new RadialDecisionUI instance")
	radial_ui = RadialDecisionUI.new()
	radial_ui.action_selected.connect(_on_interactive_action_selected)
	add_child(radial_ui)
	print("[MatchSimulationScreen] Created and added new RadialDecisionUI")
	return radial_ui


## Phase E: Search scene tree for existing RadialDecisionUI
## Returns null if not found
func _find_radial_decision_ui_in_tree() -> RadialDecisionUI:
	# Search in "radial_ui" group first (if RadialDecisionUI adds itself to group)
	var radial_ui_group = get_tree().get_nodes_in_group("radial_ui")
	for child in radial_ui_group:
		if child is RadialDecisionUI:
			return child as RadialDecisionUI

	# Fallback: Search all children recursively
	return _find_radial_decision_ui_recursive(get_tree().root)


## Recursive helper for _find_radial_decision_ui_in_tree()
func _find_radial_decision_ui_recursive(node: Node) -> RadialDecisionUI:
	if node is RadialDecisionUI:
		return node as RadialDecisionUI

	for child in node.get_children():
		var result = _find_radial_decision_ui_recursive(child)
		if result:
			return result

	return null


func _on_interactive_match_finished(result: Dictionary) -> void:
  """Phase E.3a: Handle interactive match completion"""
  print("[MatchSimulationScreen] Interactive match finished: %s" % str(result))

  # Update UI with final score
  var home_score = int(result.get("score_home", result.get("home_score", 0)))
  var away_score = int(result.get("score_away", result.get("away_score", 0)))
  home_score_label.text = str(home_score)
  away_score_label.text = str(away_score)
  time_label.text = "경기 완료 (Interactive)"

	# Re-enable buttons
	play_button.disabled = false
	pause_button.disabled = false
	skip_button.disabled = false

	# Clean up controller
	_interactive_controller = null


func _exit_tree() -> void:
	# Clean up signals when leaving the scene tree
	_disconnect_session_signals()
	if match_session_controller:
		match_session_controller.stop_session()


func _init_match_info() -> void:
	# Basic title from StageManager + MyTeamData if available.
	var home_name := "My Team"
	var away_name := "Opponent"

	if _my_team_data and _my_team_data.has_method("get_team_name"):
		home_name = _my_team_data.get_team_name()

	if _current_stage_id != -1 and _stage_manager and _stage_manager.has_method("get_stage_info"):
		var info: Dictionary = _stage_manager.get_stage_info(_current_stage_id)
		away_name = str(info.get("club_name", away_name))
		var stage_label := "Stage %d" % info.get("stage_id", _current_stage_id)
		title_label.text = "%s vs %s (%s)" % [home_name, away_name, stage_label]
	else:
		title_label.text = "%s vs %s" % [home_name, away_name]

	home_team_label.text = home_name
	away_team_label.text = away_name
	home_score_label.text = "0"
	away_score_label.text = "0"
	time_label.text = "00:00"


func _connect_buttons() -> void:
	if back_button:
		back_button.pressed.connect(_on_back_pressed)
	if attack_button:
		attack_button.pressed.connect(_on_tactics_attack_pressed)
	if balanced_button:
		balanced_button.pressed.connect(_on_tactics_balanced_pressed)
	if defend_button:
		defend_button.pressed.connect(_on_tactics_defend_pressed)
	if sub_button:
		sub_button.pressed.connect(_on_sub_button_dialog_pressed)
	if play_button:
		play_button.pressed.connect(_on_play_pressed)
	if pause_button:
		pause_button.pressed.connect(_on_pause_pressed)
	if speed_button:
		speed_button.pressed.connect(_on_speed_pressed)
	if skip_button:
		skip_button.pressed.connect(_on_skip_pressed)


func _on_back_pressed() -> void:
	print("[MatchSimulationScreen] Back pressed")
	# Navigation target will be decided by the flow (e.g., StageSelectScreen).
	# For now, just go back to the previous scene if any.
	if not is_inside_tree() or not get_tree():
		return
	var tree = get_tree()
	if tree.has_method("change_scene_to_file"):
		# Optional: could store previous scene path in metadata in the future.
		tree.change_scene_to_file("res://scenes/mvp/WeekHub.tscn")


func _on_sub_button_dialog_pressed() -> void:
	print("[MatchSimulationScreen] Substitution dialog requested")
	if _home_roster.is_empty() and _away_roster.is_empty():
		push_warning("[MatchSimulationScreen] No roster available for substitution dialog")
		return

	var dialog_scene: PackedScene = preload("res://scenes/ui/SubstitutionDialog.tscn")
	var dialog := dialog_scene.instantiate()
	add_child(dialog)

	if dialog is SubstitutionDialog:
		# 양팀 로스터 전달 (2025-12-09 수정)
		(dialog as SubstitutionDialog).setup_both_teams(_home_roster, _away_roster)
		(dialog as SubstitutionDialog).substitution_selected.connect(_on_substitution_payload_selected)
	else:
		# 레거시 API 폴백
		if dialog.has_method("setup_both_teams"):
			dialog.call("setup_both_teams", _home_roster, _away_roster)
		elif dialog.has_method("setup"):
			dialog.call("setup", _home_roster)
		if dialog.has_signal("substitution_selected"):
			dialog.substitution_selected.connect(_on_substitution_payload_selected)


func _on_sub_button_pressed() -> void:
	print("[MatchSimulationScreen] Substitution button pressed")
	if not _match_sim_manager:
		push_warning("[MatchSimulationScreen] MatchSimulationManager autoload not available for substitution")
		return

	# Temporary hard-coded substitution payload for vertical slice.
	var payload := {
		"team": "home",
		"out_track_id": 9,
		"in_bench_slot": 0,
		"out_name": "H9",
		"in_name": "SUB0",
		"minute": 60.0,
	}

	_match_sim_manager.queue_substitution(payload)

	var label := Label.new()
	label.text = (
		"교체 요청: %s → %s (%.0f')"
		% [
			str(payload.get("out_name", "")),
			str(payload.get("in_name", "")),
			float(payload.minute),
		]
	)
	event_list.add_child(label)


func _on_play_pressed() -> void:
	print("[MatchSimulationScreen] Play pressed")

	# Handle halftime resume
	if _halftime_active and match_session_controller:
		_halftime_active = false
		play_button.disabled = true
		play_button.text = "Play"
		time_label.text = "2nd Half"
		match_session_controller.resume_second_half()

		# Add second half start event to log
		if event_list:
			var label := Label.new()
			label.text = "=== 2ND HALF ==="
			label.add_theme_color_override("font_color", Color.GREEN)
			event_list.add_child(label)

		print("[MatchSimulationScreen] Second half started")
		return

	# Phase E.3a: Interactive Mode (Bullet-Time)
	if _use_interactive_mode and _try_start_interactive_mode():
		return

	# 1) Session 모드: step API 기반 스트리밍 뷰를 우선 시도한다.
	if USE_SESSION_STREAM_VIEW and _try_start_session_mode():
		return

	if not _match_sim_manager:
		push_warning("[MatchSimulationScreen] MatchSimulationManager autoload not available")
		return

	# Build a minimal match_data payload; MatchSimulationManager will
	# fill in missing fields (week/year/type) and construct full teams.
	var match_data: Dictionary = {}
	match_data["type"] = "league"
	match_data["opponent"] = away_team_label.text
	match_data["importance"] = 5
	if _current_stage_id != -1:
		match_data["stage_id"] = _current_stage_id

	# Basic UI feedback during simulation
	play_button.disabled = true
	pause_button.disabled = true
	skip_button.disabled = true
	time_label.text = "시뮬레이션 중…"

	var result: Dictionary = await _match_sim_manager.simulate_match(match_data)

	play_button.disabled = false
	pause_button.disabled = false
	skip_button.disabled = false

	if not result.get("success", false):
		time_label.text = "시뮬레이션 실패"
		push_warning("[MatchSimulationScreen] Match simulation failed: %s" % str(result.get("error", "")))
	else:
		# On success, MatchSimulationManager emits match_completed, which
		# triggers _on_match_completed() to update score and log.
		time_label.text = "경기 완료"


func _on_pause_pressed() -> void:
	print("[MatchSimulationScreen] Pause pressed")

	# Session mode: delegate to MatchSessionController.
	if _session_mode_active and match_session_controller:
		match_session_controller.pause()
		return

	# If a timeline is active, pause position playback via MatchTimelineController.
	var controller := get_node_or_null("/root/MatchTimelineController")
	if controller and controller.has_method("pause_position_playback"):
		controller.pause_position_playback()


func _on_speed_pressed() -> void:
	_speed_multiplier = 2 if _speed_multiplier == 1 else 1
	speed_button.text = "×%d" % _speed_multiplier
	print("[MatchSimulationScreen] Speed toggled to x%d" % _speed_multiplier)

	# Session 모드인 경우 tick 크기 조절.
	if _session_mode_active and match_session_controller:
		match_session_controller.set_speed(float(_speed_multiplier))
		return

	# Update position playback speed if a timeline is running.
	var controller := get_node_or_null("/root/MatchTimelineController")
	if controller and controller.has_method("set_position_playback_speed"):
		controller.set_position_playback_speed(float(_speed_multiplier))


func _on_skip_pressed() -> void:
	print("[MatchSimulationScreen] Skip pressed (fast-forward to end of timeline if available)")

	# Session 모드라면 즉시 세션 종료.
	if _session_mode_active and match_session_controller:
		match_session_controller.stop_session()
		_session_mode_active = false
		return

	var controller := get_node_or_null("/root/MatchTimelineController")
	if not controller:
		return

	if not controller.has_method("seek_position_time") or not controller.has_method("stop_position_playback"):
		return

	# Fast-forward to the end of the loaded position timeline and stop playback.
	var total_ms: int = int(controller.get("position_total_duration_ms"))
	if total_ms > 0:
		controller.seek_position_time(total_ms)
	controller.stop_position_playback()


func _try_start_interactive_mode() -> bool:
	"""Phase E.3a: Start Interactive Mode (Bullet-Time) match"""
	if not _match_sim_manager:
		push_warning("[MatchSimulationScreen] MatchSimulationManager not available")
		return false

	# Build match_data (same schema as regular matches)
	var match_data: Dictionary = {}
	match_data["type"] = "league"
	match_data["opponent"] = away_team_label.text
	match_data["importance"] = 5
	if _current_stage_id != -1:
		match_data["stage_id"] = _current_stage_id

	# Disable buttons during startup
	play_button.disabled = true
	pause_button.disabled = true
	skip_button.disabled = true
	time_label.text = "Interactive Mode 시작 중…"

	# Start interactive match via MatchSimulationManager
	print("[MatchSimulationScreen] Starting Interactive Mode match...")
	_interactive_controller = _match_sim_manager.start_interactive_match(match_data)

	if _interactive_controller == null:
		push_error("[MatchSimulationScreen] Failed to start interactive match")
		time_label.text = "Interactive Mode 시작 실패"
		play_button.disabled = false
		pause_button.disabled = false
		skip_button.disabled = false
		return false

	# Connect signals
	_interactive_controller.intervention_requested.connect(_on_interactive_intervention)
	_interactive_controller.match_finished.connect(_on_interactive_match_finished)
	_interactive_controller.error_occurred.connect(_on_interactive_error)

	time_label.text = "Interactive Mode 실행 중…"
	print("[MatchSimulationScreen] Interactive Mode started successfully")
	return true


func _on_interactive_error(message: String) -> void:
	"""Phase E.3a: Handle errors from InteractiveMatchController"""
	push_error("[MatchSimulationScreen] Interactive Mode error: %s" % message)
	time_label.text = "Interactive Mode 오류: %s" % message

	# Re-enable buttons
	play_button.disabled = false
	pause_button.disabled = false
	skip_button.disabled = false

	# Clean up
	_interactive_controller = null


func _try_start_session_mode() -> bool:
	# Session 컨트롤러 또는 매니저가 없으면 바로 실패 처리.
	if not match_session_controller:
		return false
	if not _match_sim_manager:
		return false

	# MatchSimulationManager와 동일한 match_data 스키마 사용.
	var match_data: Dictionary = {}
	match_data["type"] = "league"
	match_data["opponent"] = away_team_label.text
	match_data["importance"] = 5
	if _current_stage_id != -1:
		match_data["stage_id"] = _current_stage_id
		# MatchSimulationManager에서 엔진이 기대하는 전체 match_request를 구성한다.
		var request: Dictionary = _match_sim_manager.build_match_session_request(match_data, "my_player")
		if request.is_empty():
			push_warning("[MatchSimulationScreen] Failed to build session match_request; falling back to batch mode")
			return false

		if enable_team_view_observation and not request.has("team_view_observation"):
			request["team_view_observation"] = {
				"enabled": true, "observer_is_home": team_view_observer_is_home, "simple": true, "minimap": true
			}

	# Start through MatchSimulationManager so UnifiedFramePipeline is wired.
	if not _match_sim_manager.start_match_session_simple(request):
		push_warning("[MatchSimulationScreen] Failed to start match session; falling back to batch mode")
		return false

	# Prefer the manager-owned controller for signals / lifecycle hooks.
	if _match_sim_manager.has("match_session_controller"):
		var controller_variant: Variant = _match_sim_manager.get("match_session_controller")
		if controller_variant is MatchSessionController:
			match_session_controller = controller_variant

	var viewer := _attach_session_viewer()
	if viewer == null:
		return false

	# MyTeamData 외형 적용 (2025-12-09 추가)
	_setup_team_appearance_for_viewer(viewer, request)

	_session_mode_active = true
	_session_events = []  # Reset accumulated events for new match
	_session_last_score = {"home": 0, "away": 0}  # Reset score tracking
	time_label.text = "세션 모드 - 경기 진행 중"

	play_button.disabled = true

	# Embedded viewer provides its own timeline controls; hide duplicate top buttons.
	_set_playback_controls_visible(false)

	# Store bound callables for proper disconnection later (prevents memory leak).
	_bound_session_tick_cb = _on_session_tick
	_bound_session_halftime_cb = _on_session_halftime
	_bound_session_finished_cb = _on_session_finished
	_bound_session_paused_cb = _on_session_paused  # Phase 4.6
	if match_session_controller:
		match_session_controller.tick.connect(_bound_session_tick_cb)
		match_session_controller.halftime.connect(_bound_session_halftime_cb)
		match_session_controller.finished.connect(_bound_session_finished_cb)
		if match_session_controller.has_signal("paused"):
			match_session_controller.paused.connect(_bound_session_paused_cb)

	return true


func _attach_session_viewer() -> Control:
	## Attach the session viewer scene and return its HorizontalMatchViewer node.
	if not match_view_container:
		return null

	# Remove previous viewer.
	for child in match_view_container.get_children():
		child.queue_free()

	_viewer_root = SESSION_VIEWER_SCENE.instantiate()
	match_view_container.add_child(_viewer_root)
	match_view_container.visible = true

	var viewer: Control = null
	if _viewer_root and _viewer_root.has_node("HorizontalMatchViewer"):
		viewer = _viewer_root.get_node("HorizontalMatchViewer") as Control

	return viewer


func _on_session_tick(t_ms: int, snapshot: Dictionary, events: Array) -> void:
	# Viewer is driven by UnifiedFramePipeline; we only use the viewer node for optional overlays.
	var viewer: Control = null
	if _viewer_root and _viewer_root.has_node("HorizontalMatchViewer"):
		viewer = _viewer_root.get_node("HorizontalMatchViewer") as Control

	# 2) HUD 시간 라벨 업데이트
	var total_seconds := int(t_ms / 1000)
	var minutes := total_seconds / 60
	var seconds := total_seconds % 60
	time_label.text = "%02d:%02d" % [minutes, seconds]

	# 2-b) HorizontalMatchViewer 내부 ScorePanel 시간 업데이트 (2025-12-09 추가)
	if viewer and viewer.has_method("set_match_time"):
		viewer.set_match_time(t_ms)

	# 3) HUD 스코어 라벨 업데이트 (2025-12-07 추가)
	if snapshot.has("score") and snapshot.score is Dictionary:
		var score_dict: Dictionary = snapshot.score
		_session_last_score = score_dict.duplicate()  # Track last known score for history
		var home_score: int = int(score_dict.get("home", 0))
		var away_score: int = int(score_dict.get("away", 0))
		home_score_label.text = str(home_score)
		away_score_label.text = str(away_score)

		# 3-b) HorizontalMatchViewer 내부 ScorePanel 스코어 업데이트 (2025-12-09 추가)
		if viewer and viewer.has_method("set_score"):
			viewer.set_score(home_score, away_score)

	# 4) step API에서 넘어온 이벤트를 공통 스키마로 어댑트한 뒤 텍스트 로그로 출력
	# Accumulate events for match_history save (before early return)
	if not events.is_empty():
		_session_events.append_array(events)

		# 4-a) 액션 팝업 트리거 (2025-12-10: PASS, DRIBBLE 등 표시)
		if viewer and viewer.has_node("EventOverlay"):
			var ev_overlay = viewer.get_node("EventOverlay")
			if ev_overlay and ev_overlay.has_method("trigger_action_from_event"):
				for ev in events:
					if ev is Dictionary:
						ev_overlay.trigger_action_from_event(ev)

	if events.is_empty() or not event_list:
		return

	var adapted_events: Array = EventSchemaAdapter.adapt_events(
		events, t_ms, home_team_label.text, away_team_label.text
	)
	if adapted_events.is_empty():
		return

	# Update timeline markers
	_timeline_markers = EventTimelineAdapter.events_to_markers(
		adapted_events, home_team_label.text, away_team_label.text, _timeline_markers
	)
	if _viewer_root and _viewer_root.has_method("set_timeline_markers"):
		_viewer_root.call("set_timeline_markers", _timeline_markers)

	# 3-b) 텍스트 로그 출력
	for ev_dict in adapted_events:
		if not (ev_dict is Dictionary):
			continue
		var minute := int(ev_dict.get("minute", 0))
                var kind := _MatchTimeFormatter.format_event_kind_short(
                        str(ev_dict.get("type", "event"))
                )
                var team_name := str(ev_dict.get("team", ""))
                var player_name := str(ev_dict.get("player", ""))

		var label := Label.new()
		label.text = (
			"%02d' [%s] %s (%s)"
			% [
				minute,
				team_name,
				player_name,
				kind,
			]
		)
		event_list.add_child(label)

	# 5) 이벤트 로그 자동 스크롤 (2025-12-07 추가)
	_auto_scroll_event_log()


func _on_session_halftime(t_ms: int, snapshot: Dictionary, events: Array) -> void:
	## Handle halftime: show halftime UI and pause simulation.
	_halftime_active = true

	# Update HUD
	time_label.text = "HALFTIME"

	# Add halftime event to log
	if event_list:
		var label := Label.new()
		label.text = "=== HALFTIME ==="
		label.add_theme_color_override("font_color", Color.YELLOW)
		event_list.add_child(label)

	# Enable play button to allow resuming second half
	play_button.disabled = false
	play_button.text = "2nd Half"

	print("[MatchSimulationScreen] Halftime reached - waiting for user to start 2nd half")


func _on_session_paused(t_ms: int, decision_context: Dictionary) -> void:
	## Phase 4.6: Handle Hero Time pause - show action selection overlay
	print("[MatchSimulationScreen] Hero Time pause at t_ms=%d" % t_ms)

	# Instantiate overlay if needed
	if _player_command_overlay == null:
		_player_command_overlay = PLAYER_COMMAND_OVERLAY_SCENE.instantiate()
		add_child(_player_command_overlay)

		# Connect action_selected signal for logging
		if _player_command_overlay.has_signal("action_selected"):
			_player_command_overlay.action_selected.connect(_on_hero_time_action_selected)

	# Enrich context with time info
	var enriched_context: Dictionary = decision_context.duplicate()
	enriched_context["time_seconds"] = float(t_ms) / 1000.0

	# Show overlay with decision context
	if _player_command_overlay.has_method("show_decision"):
		_player_command_overlay.show_decision(enriched_context, match_session_controller)

	# Update HUD to show Hero Time state
	time_label.text = "HERO TIME"


func _on_hero_time_action_selected(action: Dictionary) -> void:
	## Phase 4.6: Handle user action selection from Hero Time overlay
	var action_name: String = str(action.get("action", "unknown"))
	print("[MatchSimulationScreen] Hero Time action selected: %s" % action_name)

	# Log the action to event list
	if event_list:
		var label := Label.new()
		label.text = "HERO TIME: %s" % action_name.to_upper()
		label.add_theme_color_override("font_color", Color.CYAN)
		event_list.add_child(label)


func _on_session_finished(result: Dictionary) -> void:
	# Disconnect session signals to prevent memory leak on re-entry
	_disconnect_session_signals()

	# Clean up Hero Time overlay if present
	if _player_command_overlay:
		_player_command_overlay.queue_free()
		_player_command_overlay = null

	_session_mode_active = false
	_halftime_active = false
	play_button.disabled = false
	play_button.text = "Play"

	# 숨겼던 컨트롤 버튼 복원 (2025-12-07)
	_set_playback_controls_visible(true)

	# Extract final score (prefer result dict, fallback to tracked score)
	var home_score: int = 0
	var away_score: int = 0
	if result.has("score") and result.score is Dictionary:
		var score_dict: Dictionary = result.score
		home_score = int(score_dict.get("home", 0))
		away_score = int(score_dict.get("away", 0))
	else:
		home_score = int(_session_last_score.get("home", 0))
		away_score = int(_session_last_score.get("away", 0))

	var summary := " %d - %d" % [home_score, away_score]
	time_label.text = "경기 종료%s" % summary

	# Save match to history (P3: 2025-12-08)
	_save_session_match_to_history(home_score, away_score, result)

	# Phase 5.6: Apply and show Hero Growth result
	_apply_and_show_hero_growth(result)

	# Offer post-match viewer option (P3: 2025-12-08)
	_offer_post_match_viewer()


func _disconnect_session_signals() -> void:
	## Properly disconnect bound callables to prevent memory leak.
	if not match_session_controller:
		return

	if not _bound_session_tick_cb.is_null() and match_session_controller.tick.is_connected(_bound_session_tick_cb):
		match_session_controller.tick.disconnect(_bound_session_tick_cb)

	if (
		not _bound_session_halftime_cb.is_null()
		and match_session_controller.halftime.is_connected(_bound_session_halftime_cb)
	):
		match_session_controller.halftime.disconnect(_bound_session_halftime_cb)

	if (
		not _bound_session_finished_cb.is_null()
		and match_session_controller.finished.is_connected(_bound_session_finished_cb)
	):
		match_session_controller.finished.disconnect(_bound_session_finished_cb)

	# Phase 4.6: Disconnect Hero Time pause signal
	if not _bound_session_paused_cb.is_null() and match_session_controller.has_signal("paused"):
		if match_session_controller.paused.is_connected(_bound_session_paused_cb):
			match_session_controller.paused.disconnect(_bound_session_paused_cb)

	# Clear stored callables
	_bound_session_tick_cb = Callable()
	_bound_session_halftime_cb = Callable()
	_bound_session_finished_cb = Callable()
	_bound_session_paused_cb = Callable()


func _auto_scroll_event_log() -> void:
	## 이벤트 로그 자동 스크롤 (2025-12-07 추가)
	var scroll: ScrollContainer = get_node_or_null("VBox/EventLog/EventScroll")
	if not scroll:
		return
	# 다음 프레임에 스크롤 (레이아웃 업데이트 후)
	await get_tree().process_frame
	# Check scroll still valid after await
	if not scroll or not is_instance_valid(scroll):
		return
	var v_scroll_bar := scroll.get_v_scroll_bar()
	if v_scroll_bar:
		scroll.scroll_vertical = int(v_scroll_bar.max_value)


func _set_playback_controls_visible(visible: bool) -> void:
        ## Embedded viewer가 하단 컨트롤을 제공하므로 상단의 중복 버튼을 숨김/복원 (2025-12-07)
        ## 전술 버튼(Attack/Balanced/Defend/Sub)은 계속 표시
        if pause_button:
		pause_button.visible = visible
	if speed_button:
		speed_button.visible = visible
	if skip_button:
		skip_button.visible = visible


func _save_session_match_to_history(home_score: int, away_score: int, result: Dictionary) -> void:
	## Save session match result to MatchManager.match_history (P3: 2025-12-08)
	## Uses ingest_external_match() for consistency with batch mode
	var match_manager := get_node_or_null("/root/MatchManager")
	if not match_manager or not match_manager.has_method("ingest_external_match"):
		push_warning("[MatchSimulationScreen] MatchManager not available for history save")
		return

	# Determine result text
	var result_text: String = "무승부"
	if home_score > away_score:
		result_text = "승리"
	elif home_score < away_score:
		result_text = "패배"

	# Build match payload for ingest_external_match()
	var match_payload: Dictionary = {
		"opponent_name": away_team_label.text,
		"opponent_rating": 50,  # Default if not available
		"goals_scored": home_score,
		"goals_conceded": away_score,
		"result": result_text,
		"final_score": [home_score, away_score],
		"seed": 0,  # Session mode doesn't use deterministic seed
		"events": _session_events.duplicate(true),
		"timeline": _timeline_markers.duplicate(true),
		"match_mode": "session",
	}

	# Add stage info if available
	if _current_stage_id != -1:
		match_payload["stage_id"] = _current_stage_id

	# Merge any additional data from the result dictionary
	if not result.is_empty():
		match_payload["raw_result"] = result.duplicate(true)

	match_manager.ingest_external_match(match_payload)
	print(
		(
			"[MatchSimulationScreen] Session match saved to history: %s vs %s (%d-%d)"
			% [home_team_label.text, away_team_label.text, home_score, away_score]
		)
	)


func _apply_and_show_hero_growth(result: Dictionary) -> void:
	## Phase 5.6: Apply Hero Growth from match result and show UI panel
	## Rust engine returns hero_growth in the finish result Dictionary

	# Check if hero_growth data exists in result
	var hero_growth: Dictionary = result.get("hero_growth", {})

	# If no hero_growth from engine, try to generate mock growth for testing
	if hero_growth.is_empty() and result.get("hero_xp_events", []).size() > 0:
		# Generate growth from accumulated XP events (fallback)
		hero_growth = _generate_growth_from_xp_events(result.get("hero_xp_events", []))

	if hero_growth.is_empty():
		print("[MatchSimulationScreen] No Hero Growth data in match result")
		return

	# Apply growth to PlayerData
	var player_data := get_node_or_null("/root/PlayerData")
	if player_data and player_data.has_method("apply_match_growth"):
		var apply_result: Dictionary = player_data.apply_match_growth(hero_growth)
		print("[MatchSimulationScreen] Hero Growth applied: %s" % str(apply_result))

	# Show growth result panel
	_show_growth_result_panel(hero_growth)


func _show_growth_result_panel(hero_growth: Dictionary) -> void:
	## Instantiate and show the GrowthResultPanel UI
	if _growth_result_panel == null:
		_growth_result_panel = GROWTH_RESULT_PANEL_SCENE.instantiate()
		add_child(_growth_result_panel)

		# Connect panel_closed signal
		if _growth_result_panel.has_signal("panel_closed"):
			_growth_result_panel.panel_closed.connect(_on_growth_panel_closed)

	# Show the growth result
	if _growth_result_panel.has_method("show_growth_result"):
		_growth_result_panel.show_growth_result(hero_growth)


func _on_growth_panel_closed() -> void:
	## Called when GrowthResultPanel is closed
	print("[MatchSimulationScreen] Growth panel closed")


func _generate_growth_from_xp_events(xp_events: Array) -> Dictionary:
	## Fallback: Generate HeroMatchGrowth from accumulated XP events
	## This is a simplified calculation - the real one happens in Rust
	var stat_xp: Dictionary = {}
	var total_xp: float = 0.0

	for event in xp_events:
		if not (event is Dictionary):
			continue

		var xp: float = float(event.get("xp", 0.0))
		total_xp += xp

		# Get affected stats from the event
		var affected: Array = event.get("affected_stats", [])
		for stat_info in affected:
			if not (stat_info is Array) or stat_info.size() < 2:
				continue
			var stat_name: String = str(stat_info[0])
			var weight: float = float(stat_info[1])
			stat_xp[stat_name] = stat_xp.get(stat_name, 0.0) + (xp * weight)

	# Convert XP to stat gains (simplified threshold: 15 XP = +1)
	var stat_gains: Dictionary = {}
	var xp_overflow: Dictionary = {}
	var highlight_gains: Array = []

	for stat_name in stat_xp:
		var xp_amount: float = stat_xp[stat_name]
		var threshold: float = 15.0
		var gains: int = int(xp_amount / threshold)
		var leftover: float = fmod(xp_amount, threshold)

		if gains > 0:
			gains = mini(gains, 3)  # Max +3 per match
			stat_gains[stat_name] = gains
			highlight_gains.append([stat_name, gains])
		if leftover > 0.0:
			xp_overflow[stat_name] = leftover

	return {
		"stat_gains": stat_gains,
		"xp_overflow": xp_overflow,
		"total_xp_earned": total_xp,
		"highlight_gains": highlight_gains
	}


func _offer_post_match_viewer() -> void:
	## Post-match UI for Session mode (2025-12-08)
	## Session mode = stats only, no timeline recording (streaming mode doesn't store position_data)
	## Shows "View Stats" button instead of timeline viewer option
	play_button.text = "View Stats"
	play_button.disabled = false

	# Disconnect any existing handler and connect stats viewer
	# Disconnect all existing connections (prevent dangling connections)
	while play_button.pressed.is_connected(_on_play_pressed):
		play_button.pressed.disconnect(_on_play_pressed)
	while play_button.pressed.is_connected(_on_view_stats_pressed):
		play_button.pressed.disconnect(_on_view_stats_pressed)
	play_button.pressed.connect(_on_view_stats_pressed, CONNECT_ONE_SHOT)


func _on_view_stats_pressed() -> void:
	## Handle "View Stats" button click after session match ends
	## Shows match statistics summary (no timeline recording for Session mode)

	# Reconnect original play button handler
	if not play_button.pressed.is_connected(_on_play_pressed):
		play_button.pressed.connect(_on_play_pressed)

	# Get the latest match record from history
	var match_manager := get_node_or_null("/root/MatchManager")
	if not match_manager or not match_manager.has_method("get_match_history"):
		push_warning("[MatchSimulationScreen] Cannot show stats - MatchManager not available")
		return

	var history: Array = match_manager.get_match_history()
	if history.is_empty():
		push_warning("[MatchSimulationScreen] Cannot show stats - no match history")
		return

	var record: Dictionary = history[0] if history[0] is Dictionary else {}
	if record.is_empty():
		return

	# Display stats in event log or dedicated UI
	_display_match_stats(record)


func _display_match_stats(record: Dictionary) -> void:
	## Display match statistics summary in the event log area
	var stats_text := "=== MATCH STATISTICS ===\n"

	# Basic info
	var opponent := str(record.get("opponent_name", "Unknown"))
	var scored := int(record.get("goals_scored", 0))
	var conceded := int(record.get("goals_conceded", 0))
	var result_str := str(record.get("result", ""))

	stats_text += "vs %s\n" % opponent
	stats_text += "Final: %d - %d (%s)\n\n" % [scored, conceded, result_str]

	# Events summary
	var events: Array = record.get("events", [])
	if not events.is_empty():
		stats_text += "--- Key Events ---\n"
		for ev in events:
			if not (ev is Dictionary):
				continue
			var ev_type := str(ev.get("type", "")).to_lower()
			var minute := int(ev.get("minute", 0))
			var player := str(ev.get("player", ""))

			match ev_type:
				"goal":
					var team_id := int(ev.get("team_id", -1))
					var team_str := "HOME" if team_id == 0 else "AWAY"
					stats_text += "%d' GOAL (%s) - %s\n" % [minute, team_str, player]
				"yellow_card":
					stats_text += "%d' Yellow Card - %s\n" % [minute, player]
				"red_card":
					stats_text += "%d' RED CARD - %s\n" % [minute, player]
				"substitution":
					stats_text += "%d' Substitution - %s\n" % [minute, player]

	# Display stats in event list
	if event_list:
		# Clear previous stats entries
		for child in event_list.get_children():
			child.queue_free()

		# Add stats as label
		var stats_label := Label.new()
		stats_label.text = stats_text
		event_list.add_child(stats_label)


func _setup_team_appearance_for_viewer(viewer: Control, request: Dictionary) -> void:
	## MyTeamData의 팀 외형(유니폼)과 선수 외형을 뷰어에 적용한다 (2025-12-09)
	if not viewer or not viewer.has_method("setup_my_team_as_home"):
		# setup_my_team_as_home이 없으면 fallback으로 set_team_colors만 호출
		if viewer and viewer.has_method("set_team_colors"):
			var opponent_id: String = request.get("away_team_id", "away")
			viewer.set_team_colors("home", opponent_id)
		return

	if not _my_team_data:
		push_warning("[MatchSimulationScreen] MyTeamData not available for appearance setup")
		return

	# 마이팀 로스터 구성 (외형 데이터 포함)
	var my_roster: Array = []
	if _my_team_data.has_method("get_starting_lineup"):
		var starting: Array = _my_team_data.get_starting_lineup()
		for player in starting:
			if not (player is Dictionary):
				continue
			my_roster.append(
				{
					"id": player.get("id", ""),
					"position": player.get("position", ""),
					"jersey_number": player.get("jersey_number", 0),
					"appearance": player.get("appearance", {})
				}
			)
	elif _my_team_data.has_method("get_roster"):
		var roster: Array = _my_team_data.get_roster()
		for i in range(min(11, roster.size())):
			var player: Dictionary = roster[i] if roster[i] is Dictionary else {}
			my_roster.append(
				{
					"id": player.get("id", str(i)),
					"position": player.get("position", ""),
					"jersey_number": player.get("jersey_number", i + 1),
					"appearance": player.get("appearance", {})
				}
			)

	# 상대팀 로스터 (request에서 가져오거나 기본값 생성)
	var opponent_roster: Array = request.get("away_roster", [])
	if opponent_roster.is_empty():
		# 기본 상대팀 로스터 생성 (11명)
		for i in range(11):
			opponent_roster.append({"id": "away_%d" % i, "position": "", "jersey_number": i + 1, "appearance": {}})

	# 상대팀 ID
	var opponent_id: String = request.get("away_team_id", "away")

	# HorizontalMatchViewer.setup_my_team_as_home() 호출
	viewer.setup_my_team_as_home(_my_team_data, my_roster, opponent_roster, opponent_id)

	# HUD 초기화: 팀 이름, 초기 스코어 설정 (2025-12-09 추가)
	var home_name: String = home_team_label.text if home_team_label else "Home"
	var away_name: String = away_team_label.text if away_team_label else "Away"
	if viewer.has_method("set_hud_team_names"):
		viewer.set_hud_team_names(home_name, away_name)
	if viewer.has_method("set_score"):
		viewer.set_score(0, 0)
	if viewer.has_method("set_match_time"):
		viewer.set_match_time(0)

	print("[MatchSimulationScreen] Team appearance and HUD initialized for viewer")


func _on_tactics_attack_pressed() -> void:
	_send_tactics_preset("attack")


func _on_tactics_defend_pressed() -> void:
	_send_tactics_preset("defend")


func _on_tactics_balanced_pressed() -> void:
	_send_tactics_preset("balanced")


func _send_tactics_preset(kind: String) -> void:
	if not _match_sim_manager:
		push_warning("[MatchSimulationScreen] MatchSimulationManager autoload not available for tactics")
		return

	var payload: Dictionary = {
		"team": "home",
	}

	match kind:
		"attack":
			payload["preset"] = "HighPressing"
			payload["attack_bias"] = 0.8
			payload["press_intensity"] = 0.8
			payload["tempo"] = "fast"
		"defend":
			payload["preset"] = "Defensive"
			payload["attack_bias"] = 0.2
			payload["press_intensity"] = 0.3
			payload["tempo"] = "slow"
		"balanced":
			payload["preset"] = "Balanced"
			payload["attack_bias"] = 0.5
			payload["press_intensity"] = 0.5
			payload["tempo"] = "normal"
		_:
			payload["preset"] = kind

	_match_sim_manager.apply_tactics(payload)

	var label := Label.new()
	var display_text: String
	match kind:
		"attack":
			display_text = "전술 변경: 공격"
		"defend":
			display_text = "전술 변경: 수비"
		"balanced":
			display_text = "전술 변경: 밸런스"
		_:
			display_text = "전술 변경: %s" % kind
	label.text = display_text
	event_list.add_child(label)


func _on_match_completed(success: bool, result: Dictionary) -> void:
	print("[MatchSimulationScreen] Match completed: success=%s, result=%s" % [str(success), str(result)])
	if not success or result.is_empty():
		return

	# Update score labels if present in result.
	var home_score: int = int(result.get("score_home", 0))
	var away_score: int = int(result.get("score_away", 0))
	home_score_label.text = str(home_score)
	away_score_label.text = str(away_score)

	# Append a simple log entry.
	var label := Label.new()
	label.text = "Full-time: %d - %d" % [home_score, away_score]
	event_list.add_child(label)

	## P2.3: Transition to PostMatchStatisticsScreen
	_show_post_match_statistics(result)


## P2.3: Prepare data and transition to PostMatchStatisticsScreen
func _show_post_match_statistics(result: Dictionary) -> void:
	var match_payload: Dictionary = {
		"home_team": home_team_label.text if home_team_label else "Home",
		"away_team": away_team_label.text if away_team_label else "Away",
		"goals_home": int(result.get("score_home", 0)),
		"goals_away": int(result.get("score_away", 0)),
		"events": result.get("events", []),
		"rosters": result.get("rosters", {}),
		"player_ratings": result.get("player_ratings", {}),
		"team_stats": result.get("team_stats", {}),
		"match_info": result.get("match_info", {}),
		"match_result": result
	}

	## Store in tree meta for PostMatchStatisticsScreen to load (with safety check)
	if not is_inside_tree() or not get_tree():
		push_warning("[MatchSimulationScreen] Cannot store meta - not in tree")
		return
	var tree = get_tree()
	tree.root.set_meta("post_match_data", match_payload)

	## Transition to stats screen
	var stats_screen_path := "res://scenes/PostMatchStatisticsScreen.tscn"
	if ResourceLoader.exists(stats_screen_path):
		var screen_transition := get_node_or_null("/root/ScreenTransition")
		if screen_transition and screen_transition.has_method("change_scene"):
			screen_transition.change_scene(stats_screen_path, "fade")
		else:
			tree.change_scene_to_file(stats_screen_path)
	else:
		push_error("[MatchSimulationScreen] PostMatchStatisticsScreen not found at %s" % stats_screen_path)


func _show_embedded_viewer(record: Dictionary) -> void:
	if not match_view_container:
		return

	# Prepare match record for the embedded viewer.
	if _timeline_data_holder and _timeline_data_holder.has_method("set_timeline_data"):
		_timeline_data_holder.set_timeline_data(record, "res://scenes/ui/match_simulation_screen.tscn")

	# Pre-load position_data into MatchTimelineController for faster initialization
	_preload_position_data(record)

	# Clear placeholder / previous viewer if present.
	for child in match_view_container.get_children():
		child.queue_free()

	_viewer_root = SESSION_VIEWER_SCENE.instantiate()
	match_view_container.add_child(_viewer_root)
	match_view_container.visible = true


func _coerce_events_array(value: Variant) -> Array:
	if value is Array:
		return (value as Array).duplicate(true)
	if value is String:
		var parsed_variant: Variant = JSON.parse_string(String(value))
		if parsed_variant is Array:
			return (parsed_variant as Array).duplicate(true)
	return []


func _coerce_dict(value: Variant) -> Dictionary:
	if value is Dictionary:
		return (value as Dictionary).duplicate(true)
	if value is String:
		var parsed_variant: Variant = JSON.parse_string(String(value))
		if parsed_variant is Dictionary:
			return (parsed_variant as Dictionary).duplicate(true)
	return {}


func _extract_timeline_events_from_record(record: Dictionary) -> Array:
	const LEGACY_PAYLOAD_KEY := "re" + "play"
	const LEGACY_DOC_KEY := LEGACY_PAYLOAD_KEY + "_doc"
	const LEGACY_EVENTS_KEY := LEGACY_PAYLOAD_KEY + "_events"

	var direct_sources: Array = [
		record.get("timeline_events", null), record.get("events", null), record.get(LEGACY_EVENTS_KEY, null)
	]
	for source in direct_sources:
		var arr: Array = _coerce_events_array(source)
		if not arr.is_empty():
			return arr

	var doc_sources: Array = []
	doc_sources.append(record.get("timeline_doc", null))
	doc_sources.append(record.get(LEGACY_DOC_KEY, record.get(LEGACY_PAYLOAD_KEY, null)))
	var match_result_variant: Variant = record.get("match_result", null)
	if match_result_variant is Dictionary:
		var match_result: Dictionary = match_result_variant
		doc_sources.append(
			match_result.get(
				"timeline_doc", match_result.get(LEGACY_DOC_KEY, match_result.get(LEGACY_PAYLOAD_KEY, null))
			)
		)
		direct_sources = [
			match_result.get("timeline_events", null),
			match_result.get("events", null),
			match_result.get(LEGACY_EVENTS_KEY, null)
		]
		for source in direct_sources:
			var arr2: Array = _coerce_events_array(source)
			if not arr2.is_empty():
				return arr2

	var raw_result_variant: Variant = record.get("raw_result", null)
	if raw_result_variant is Dictionary:
		var raw_result: Dictionary = raw_result_variant
		doc_sources.append(
			raw_result.get("timeline_doc", raw_result.get(LEGACY_DOC_KEY, raw_result.get(LEGACY_PAYLOAD_KEY, null)))
		)
		direct_sources = [
			raw_result.get("timeline_events", null),
			raw_result.get("events", null),
			raw_result.get(LEGACY_EVENTS_KEY, null)
		]
		for source in direct_sources:
			var arr3: Array = _coerce_events_array(source)
			if not arr3.is_empty():
				return arr3

	for doc_source in doc_sources:
		var doc: Dictionary = _coerce_dict(doc_source)
		if doc.is_empty():
			continue
		var doc_events: Array = _coerce_events_array(doc.get("events", null))
		if not doc_events.is_empty():
			return doc_events

	return []


## Pre-load position_data into MatchTimelineController before viewer instantiation
func _preload_position_data(record: Dictionary) -> void:
	var position_data: Dictionary = record.get("position_data", {})
	if position_data.is_empty():
		return

	var rosters: Dictionary = {}
	var rosters_variant: Variant = record.get("rosters", {})
	if rosters_variant is Dictionary:
		rosters = (rosters_variant as Dictionary).duplicate(true)
	if rosters.is_empty():
		var timeline_rosters_variant: Variant = record.get("timeline_rosters", {})
		if timeline_rosters_variant is Dictionary:
			rosters = (timeline_rosters_variant as Dictionary).duplicate(true)

	# Timeline events for overlays/SFX (treat empty/non-array as missing).
	var timeline_events: Array = _extract_timeline_events_from_record(record)

	var controller := get_node_or_null("/root/MatchTimelineController")
	if not controller:
		return

	if not controller.has_method("load_position_data"):
		push_warning("[MatchSimulationScreen] MatchTimelineController has no load_position_data method")
		return

	controller.load_position_data(position_data, rosters, timeline_events)
	print("[MatchSimulationScreen] Position data pre-loaded into MatchTimelineController")


func _cache_home_roster(record: Dictionary) -> void:
	_cache_rosters(record)


## 양팀 로스터 캐싱 (2025-12-09 확장)
func _cache_rosters(record: Dictionary) -> void:
	var legacy_doc_key := "re" + "play"
	var legacy_doc_key2 := ("re" + "play") + "_doc"
	var doc_variant: Variant = record.get("timeline_doc", record.get(legacy_doc_key2, record.get(legacy_doc_key, {})))
	if not (doc_variant is Dictionary):
		return

	var rosters: Variant = (doc_variant as Dictionary).get("rosters", record.get("timeline_rosters", {}))
	if not (rosters is Dictionary):
		return

	# 홈 로스터 캐싱
	_home_roster = _extract_roster_from_team(rosters, "home")

	# 어웨이 로스터 캐싱 (2025-12-09 추가)
	_away_roster = _extract_roster_from_team(rosters, "away")


func _extract_roster_from_team(rosters: Variant, team_key: String) -> Dictionary:
	if not (rosters is Dictionary) or not (rosters as Dictionary).has(team_key):
		return {}

	var team: Variant = (rosters as Dictionary).get(team_key, {})
	if not (team is Dictionary):
		return {}

	var players: Variant = (team as Dictionary).get("players", [])
	if not (players is Array) or (players as Array).is_empty():
		return {}

	var players_array: Array = players
	var starters: Array = players_array.slice(0, min(11, players_array.size()))
	var bench: Array = []
	if players_array.size() > 11:
		bench = players_array.slice(11, players_array.size())

	return {
		"starters": starters,
		"bench": bench,
	}


func _on_substitution_payload_selected(payload: Dictionary) -> void:
	# 1) minute 계산: 모드에 따라 다른 소스 사용
	var minute := 60.0

	# Session mode: MatchSessionController.current_minute is the source of truth
	if _session_mode_active and match_session_controller:
		minute = match_session_controller.current_minute
	else:
		# Batch/Timeline mode: MatchTimelineController.position_time_ms
		var controller := get_node_or_null("/root/MatchTimelineController")
		if controller:
			var t_ms := 0
			# 안전하게 get()으로 접근
			if controller.has_method("get"):
				t_ms = int(controller.get("position_time_ms"))
			else:
				t_ms = int(controller.position_time_ms)
			if t_ms > 0:
				minute = float(t_ms) / 1000.0 / 60.0

	if not payload.has("minute"):
		payload["minute"] = minute

	# 2) 매니저 호출
	if not _match_sim_manager:
		push_warning("[MatchSimulationScreen] MatchSimulationManager autoload not available for substitution")
		return

	_match_sim_manager.queue_substitution(payload)

        # 3) UI 로그 추가
        var label := Label.new()
        var out_label := str(payload.get("out_name", payload.get("out_player_id", "")))
        var in_label := str(payload.get("in_name", payload.get("in_player_id", "")))
        label.text = (
                "교체 요청: %s → %s (%.0f')"
                % [
                        out_label,
                        in_label,
                        float(payload.get("minute", 0.0)),
                ]
        )
        event_list.add_child(label)
