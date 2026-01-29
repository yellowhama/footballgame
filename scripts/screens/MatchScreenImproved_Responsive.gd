extends "res://scenes/academy/base/AdaptiveLayoutContainer.gd"
## Responsive Match Screen - Phase 7B Implementation
## Cross-platform UI with 3 layout variants (Mobile/Tablet/Desktop)
const POST_MATCH_SUMMARY_SCENE_PATH := "res://scenes/PostMatchStatisticsScreen.tscn"
const MATCHDAY_TEMPO_OPTIONS := [
	{"label": "느림", "value": "slow"},
	{"label": "보통", "value": "normal"},
	{"label": "빠름", "value": "fast"}
]
const MATCHDAY_PRESS_OPTIONS := [
	{"label": "낮음", "value": 0.3},
	{"label": "보통", "value": 0.5},
	{"label": "높음", "value": 0.8}
]
const MATCHDAY_BUILD_UP_OPTIONS := [
	{"label": "숏 패스", "value": "Short"},
	{"label": "혼합", "value": "Mixed"},
	{"label": "직접", "value": "Direct"}
]
const DEFAULT_MATCHDAY_INSTRUCTIONS := {
        "tempo": "normal",
        "press_intensity": 0.5,
        "build_up_style": "Mixed"
}

const _MatchTimeFormatter = preload("res://scripts/utils/MatchTimeFormatter.gd")

# Mobile layout node references
@onready var mobile_home_score = $MobilePortraitLayout/Header/HBox/HomeTeam/Score
@onready var mobile_away_score = $MobilePortraitLayout/Header/HBox/AwayTeam/Score
@onready var mobile_time = $MobilePortraitLayout/Header/HBox/TimeInfo/Time
@onready var mobile_half = $MobilePortraitLayout/Header/HBox/TimeInfo/Half
@onready var mobile_tempo_option = $MobilePortraitLayout/BottomBar/VBox/TogglesRow/TempoGroup/TempoOption
@onready var mobile_pressing_option = $MobilePortraitLayout/BottomBar/VBox/TogglesRow/PressingGroup/PressingOption
@onready var mobile_build_up_option = $MobilePortraitLayout/BottomBar/VBox/TogglesRow/BuildUpGroup/BuildUpOption
@onready var mobile_advanced_button = $MobilePortraitLayout/BottomBar/VBox/ActionsRow/AdvancedButton
@onready var mobile_skip_button = $MobilePortraitLayout/BottomBar/VBox/ActionsRow/SkipButton

# Tablet layout node references
@onready var tablet_home_score = $TabletHybridLayout/Header/HBox/HomeTeam/Score
@onready var tablet_away_score = $TabletHybridLayout/Header/HBox/AwayTeam/Score
@onready var tablet_time = $TabletHybridLayout/Header/HBox/TimeInfo/Time
@onready var tablet_half = $TabletHybridLayout/Header/HBox/TimeInfo/Half
@onready var tablet_tempo_option = $TabletHybridLayout/BottomBar/VBox/TogglesRow/TempoGroup/TempoOption
@onready var tablet_pressing_option = $TabletHybridLayout/BottomBar/VBox/TogglesRow/PressingGroup/PressingOption
@onready var tablet_build_up_option = $TabletHybridLayout/BottomBar/VBox/TogglesRow/BuildUpGroup/BuildUpOption
@onready var tablet_advanced_button = $TabletHybridLayout/BottomBar/VBox/ActionsRow/AdvancedButton
@onready var tablet_skip_button = $TabletHybridLayout/BottomBar/VBox/ActionsRow/SkipButton

# Desktop layout node references
@onready var desktop_home_score = $DesktopLandscapeLayout/Header/HBox/HomeTeam/Score
@onready var desktop_away_score = $DesktopLandscapeLayout/Header/HBox/AwayTeam/Score
@onready var desktop_time = $DesktopLandscapeLayout/Header/HBox/TimeInfo/Time
@onready var desktop_half = $DesktopLandscapeLayout/Header/HBox/TimeInfo/Half
@onready var desktop_tempo_option = $DesktopLandscapeLayout/BottomBar/VBox/TogglesRow/TempoGroup/TempoOption
@onready var desktop_pressing_option = $DesktopLandscapeLayout/BottomBar/VBox/TogglesRow/PressingGroup/PressingOption
@onready var desktop_build_up_option = $DesktopLandscapeLayout/BottomBar/VBox/TogglesRow/BuildUpGroup/BuildUpOption
@onready var desktop_advanced_button = $DesktopLandscapeLayout/BottomBar/VBox/ActionsRow/AdvancedButton
@onready var desktop_skip_button = $DesktopLandscapeLayout/BottomBar/VBox/ActionsRow/SkipButton

@onready var tactical_panel: TacticalPanel = (
	$PopupLayer/TacticalPanel if has_node("PopupLayer/TacticalPanel") else null
)

# Event lists (templates duplicated per event)
@onready var mobile_events_list = $MobilePortraitLayout/MainContent/EventsPanel/VBox/ScrollContainer/EventsList
@onready var tablet_events_list = $TabletHybridLayout/MainContent/EventsPanel/VBox/ScrollContainer/EventsList
@onready
var desktop_events_list = $DesktopLandscapeLayout/MainContent/RightPanel/EventsPanel/VBox/ScrollContainer/EventsList

# Match state
var home_score: int = 0
var away_score: int = 0
var current_time: int = 0
var current_half: String = "전반"
var current_matchday_instructions: Dictionary = DEFAULT_MATCHDAY_INSTRUCTIONS.duplicate(true)
var _is_syncing_matchday_controls: bool = false
var match_events: Array = []
var layout_event_lists: Dictionary = {}
var event_templates: Dictionary = {}


func _ready():
	super._ready()  # Call AdaptiveLayoutContainer._ready()

	print("[MatchScreen] Responsive scene initialized")

	# Connect layout activation signals
	layout_activated.connect(_on_layout_activated)

	# Connect button signals for all layouts
	_connect_mobile_signals()
	_connect_tablet_signals()
	_connect_desktop_signals()
	_setup_matchday_controls()
	_connect_tactical_panel()

	# Connect MatchManager signals
	_connect_match_manager_signals()

	# Cache event containers/templates
	_cache_event_lists()

	# Wait for platform detection
	await get_tree().process_frame

	# Initial data population
	_populate_current_layout()
	_start_pending_match_if_needed()

	# Validate UI standards
	_validate_ui_standards()


func _connect_match_manager_signals():
	"""Connect MatchManager signals for match updates"""
	if not MatchManager:
		push_warning("[MatchScreen] MatchManager not found")
		return

	if not MatchManager.match_started.is_connected(_on_match_started):
		MatchManager.match_started.connect(_on_match_started)
	if not MatchManager.score_changed.is_connected(_on_match_score_changed):
		MatchManager.score_changed.connect(_on_match_score_changed)
	if not MatchManager.time_updated.is_connected(_on_match_time_updated):
		MatchManager.time_updated.connect(_on_match_time_updated)
	if not MatchManager.event_occurred.is_connected(_on_match_event):
		MatchManager.event_occurred.connect(_on_match_event)
	if not MatchManager.match_ended.is_connected(_on_match_ended):
		MatchManager.match_ended.connect(_on_match_ended)
	if (
		MatchManager.has_signal("match_preflight_failed")
		and not MatchManager.match_preflight_failed.is_connected(_on_match_preflight_failed)
	):
		MatchManager.match_preflight_failed.connect(_on_match_preflight_failed)

	print("[MatchScreen] MatchManager signals connected")


func _cache_event_lists() -> void:
	layout_event_lists = {"mobile": mobile_events_list, "tablet": tablet_events_list, "desktop": desktop_events_list}
	event_templates.clear()

	for layout_name in layout_event_lists.keys():
		var list: VBoxContainer = layout_event_lists[layout_name]
		if not list:
			continue
		var template: Control = null
		if list.get_child_count() > 0:
			template = list.get_child(0)
			list.remove_child(template)
		if template:
			event_templates[layout_name] = template
		else:
			event_templates[layout_name] = null
		_clear_event_list(list)

	match_events.clear()


func _on_match_score_changed(home: int, away: int):
	"""Handle score update from MatchManager"""
	update_score(home, away)


func _on_match_time_updated(minutes: int, half: String):
	"""Handle time update from MatchManager"""
	update_time(minutes, half)


func _on_match_event(time: int, icon: String, text: String):
	"""Handle match event from MatchManager"""
	add_event(time, icon, text)


func _on_match_ended(result: Dictionary):
	"""Handle match end from MatchManager"""
	print("[MatchScreen] Match ended: %s" % result.result)
	_open_lite_match_summary(result)


func _on_match_preflight_failed(info: Dictionary) -> void:
	"""Handle preflight failure from MatchManager"""
	var missing: Array = info.get("missing", [])
	var title := "⚠️ 경기 시작 실패"
	var body := "선수 데이터를 찾을 수 없습니다.\n\n"

	if missing.size() > 0:
		body += "누락된 선수 UID:\n" + "\n".join(missing)
	else:
		body += info.get("error", "Unknown error")

	print("[MatchScreen] Preflight failed: ", title, "\n", body)

	# Show error dialog or toast
	if has_node("/root/UIService"):
		var ui = get_node("/root/UIService")
		if ui and ui.has_method("show_error"):
			ui.call("show_error", title, body)
		elif ui and ui.has_method("toast"):
			ui.call("toast", title + "\n" + body)

	# Fallback: Navigate back to WeekHub
	await get_tree().create_timer(3.0).timeout
	get_tree().change_scene_to_file("res://scenes/WeekHub.tscn")


func _connect_mobile_signals():
	"""Connect Mobile layout button signals"""
	if mobile_tempo_option:
		mobile_tempo_option.item_selected.connect(
			_on_matchday_option_selected.bind("tempo", mobile_tempo_option)
		)
	if mobile_pressing_option:
		mobile_pressing_option.item_selected.connect(
			_on_matchday_option_selected.bind("press_intensity", mobile_pressing_option)
		)
	if mobile_build_up_option:
		mobile_build_up_option.item_selected.connect(
			_on_matchday_option_selected.bind("build_up_style", mobile_build_up_option)
		)
	if mobile_advanced_button:
		mobile_advanced_button.pressed.connect(_on_advanced_pressed)
	if mobile_skip_button:
		mobile_skip_button.pressed.connect(_on_skip_pressed)


func _connect_tablet_signals():
	"""Connect Tablet layout button signals"""
	if tablet_tempo_option:
		tablet_tempo_option.item_selected.connect(
			_on_matchday_option_selected.bind("tempo", tablet_tempo_option)
		)
	if tablet_pressing_option:
		tablet_pressing_option.item_selected.connect(
			_on_matchday_option_selected.bind("press_intensity", tablet_pressing_option)
		)
	if tablet_build_up_option:
		tablet_build_up_option.item_selected.connect(
			_on_matchday_option_selected.bind("build_up_style", tablet_build_up_option)
		)
	if tablet_advanced_button:
		tablet_advanced_button.pressed.connect(_on_advanced_pressed)
	if tablet_skip_button:
		tablet_skip_button.pressed.connect(_on_skip_pressed)


func _connect_desktop_signals():
	"""Connect Desktop layout button signals"""
	if desktop_tempo_option:
		desktop_tempo_option.item_selected.connect(
			_on_matchday_option_selected.bind("tempo", desktop_tempo_option)
		)
	if desktop_pressing_option:
		desktop_pressing_option.item_selected.connect(
			_on_matchday_option_selected.bind("press_intensity", desktop_pressing_option)
		)
	if desktop_build_up_option:
		desktop_build_up_option.item_selected.connect(
			_on_matchday_option_selected.bind("build_up_style", desktop_build_up_option)
		)
	if desktop_advanced_button:
		desktop_advanced_button.pressed.connect(_on_advanced_pressed)
	if desktop_skip_button:
		desktop_skip_button.pressed.connect(_on_skip_pressed)


func _on_layout_activated(layout_name: String):
	"""Handle layout activation"""
	print(
		(
			"[MatchScreen] Layout activated: %s (Platform: %s)"
			% [layout_name, PlatformManager.get_platform_name() if PlatformManager else "Unknown"]
		)
	)
	_populate_current_layout()


func _populate_current_layout():
	"""Populate data for currently active layout"""
	var active = get_active_layout()
	if not active:
		push_warning("[MatchScreen] No active layout found")
		return

	match get_active_layout_name():
		"mobile":
			_populate_mobile_layout()
		"tablet":
			_populate_tablet_layout()
		"desktop":
			_populate_desktop_layout()

	_refresh_events_for_layout(get_active_layout_name())


func _populate_mobile_layout():
	"""Populate mobile-specific layout with match data"""
	print("[MatchScreen] Populating mobile layout")

	# Score and time
	if mobile_home_score:
		mobile_home_score.text = str(home_score)
	if mobile_away_score:
		mobile_away_score.text = str(away_score)
	if mobile_time:
		mobile_time.text = "%d'" % current_time
	if mobile_half:
		mobile_half.text = current_half
	_sync_matchday_controls()


func _populate_tablet_layout():
	"""Populate tablet-specific layout with match data"""
	print("[MatchScreen] Populating tablet layout")

	# Score and time
	if tablet_home_score:
		tablet_home_score.text = str(home_score)
	if tablet_away_score:
		tablet_away_score.text = str(away_score)
	if tablet_time:
		tablet_time.text = "%d'" % current_time
	if tablet_half:
		tablet_half.text = current_half

	# Matchday controls
	_sync_matchday_controls()


func _populate_desktop_layout():
	"""Populate desktop-specific layout with match data"""
	print("[MatchScreen] Populating desktop layout")

	# Score and time
	if desktop_home_score:
		desktop_home_score.text = str(home_score)
	if desktop_away_score:
		desktop_away_score.text = str(away_score)
	if desktop_time:
		desktop_time.text = "%d'" % current_time
	if desktop_half:
		desktop_half.text = current_half

	# Matchday controls
	_sync_matchday_controls()


func _validate_ui_standards():
	"""Validate UI against UIStandards requirements"""
	validate_ui_standards_base()


## Button signal handlers


func _setup_matchday_controls() -> void:
	_configure_option_button(
		mobile_tempo_option,
		MATCHDAY_TEMPO_OPTIONS,
		current_matchday_instructions.get(
			"tempo", DEFAULT_MATCHDAY_INSTRUCTIONS.get("tempo", "normal")
		)
	)
	_configure_option_button(
		mobile_pressing_option,
		MATCHDAY_PRESS_OPTIONS,
		current_matchday_instructions.get(
			"press_intensity", DEFAULT_MATCHDAY_INSTRUCTIONS.get("press_intensity", 0.5)
		)
	)
	_configure_option_button(
		mobile_build_up_option,
		MATCHDAY_BUILD_UP_OPTIONS,
		current_matchday_instructions.get(
			"build_up_style", DEFAULT_MATCHDAY_INSTRUCTIONS.get("build_up_style", "Mixed")
		)
	)
	_configure_option_button(
		tablet_tempo_option,
		MATCHDAY_TEMPO_OPTIONS,
		current_matchday_instructions.get(
			"tempo", DEFAULT_MATCHDAY_INSTRUCTIONS.get("tempo", "normal")
		)
	)
	_configure_option_button(
		tablet_pressing_option,
		MATCHDAY_PRESS_OPTIONS,
		current_matchday_instructions.get(
			"press_intensity", DEFAULT_MATCHDAY_INSTRUCTIONS.get("press_intensity", 0.5)
		)
	)
	_configure_option_button(
		tablet_build_up_option,
		MATCHDAY_BUILD_UP_OPTIONS,
		current_matchday_instructions.get(
			"build_up_style", DEFAULT_MATCHDAY_INSTRUCTIONS.get("build_up_style", "Mixed")
		)
	)
	_configure_option_button(
		desktop_tempo_option,
		MATCHDAY_TEMPO_OPTIONS,
		current_matchday_instructions.get(
			"tempo", DEFAULT_MATCHDAY_INSTRUCTIONS.get("tempo", "normal")
		)
	)
	_configure_option_button(
		desktop_pressing_option,
		MATCHDAY_PRESS_OPTIONS,
		current_matchday_instructions.get(
			"press_intensity", DEFAULT_MATCHDAY_INSTRUCTIONS.get("press_intensity", 0.5)
		)
	)
	_configure_option_button(
		desktop_build_up_option,
		MATCHDAY_BUILD_UP_OPTIONS,
		current_matchday_instructions.get(
			"build_up_style", DEFAULT_MATCHDAY_INSTRUCTIONS.get("build_up_style", "Mixed")
		)
	)
	_sync_matchday_controls()


func _connect_tactical_panel() -> void:
	if not tactical_panel:
		return
	if not tactical_panel.tactical_applied.is_connected(_on_tactical_panel_applied):
		tactical_panel.tactical_applied.connect(_on_tactical_panel_applied)


func _configure_option_button(
	selector: OptionButton, options: Array, default_value: Variant
) -> void:
	if not selector:
		return
	selector.clear()
	var default_index := 0
	for idx in range(options.size()):
		var option: Dictionary = options[idx]
		selector.add_item(str(option.get("label", "")), idx)
		selector.set_item_metadata(idx, option.get("value"))
		if _values_match(option.get("value"), default_value):
			default_index = idx
	var previous_sync := _is_syncing_matchday_controls
	_is_syncing_matchday_controls = true
	selector.select(default_index)
	_is_syncing_matchday_controls = previous_sync


func _values_match(a: Variant, b: Variant) -> bool:
	var a_type := typeof(a)
	var b_type := typeof(b)
	if a_type in [TYPE_FLOAT, TYPE_INT] and b_type in [TYPE_FLOAT, TYPE_INT]:
		return abs(float(a) - float(b)) < 0.01
	return str(a).to_lower() == str(b).to_lower()


func _select_option_by_value(selector: OptionButton, target_value: Variant) -> void:
	if not selector:
		return
	var fallback := 0
	for idx in range(selector.item_count):
		var value: Variant = selector.get_item_metadata(idx)
		if _values_match(value, target_value):
			selector.select(idx)
			return
		fallback = idx
	selector.select(fallback)


func _sync_matchday_controls() -> void:
	_is_syncing_matchday_controls = true
	var tempo = current_matchday_instructions.get(
		"tempo", DEFAULT_MATCHDAY_INSTRUCTIONS.get("tempo", "normal")
	)
	var pressing = current_matchday_instructions.get(
		"press_intensity", DEFAULT_MATCHDAY_INSTRUCTIONS.get("press_intensity", 0.5)
	)
	var buildup = current_matchday_instructions.get(
		"build_up_style", DEFAULT_MATCHDAY_INSTRUCTIONS.get("build_up_style", "Mixed")
	)
	for selector in [mobile_tempo_option, tablet_tempo_option, desktop_tempo_option]:
		_select_option_by_value(selector, tempo)
	for selector in [mobile_pressing_option, tablet_pressing_option, desktop_pressing_option]:
		_select_option_by_value(selector, pressing)
	for selector in [mobile_build_up_option, tablet_build_up_option, desktop_build_up_option]:
		_select_option_by_value(selector, buildup)
	_is_syncing_matchday_controls = false


func _on_matchday_option_selected(_index: int, key: String, selector: OptionButton) -> void:
	if _is_syncing_matchday_controls:
		return
	if not selector:
		return
	var selected_value: Variant = selector.get_item_metadata(selector.selected)
	var updated := current_matchday_instructions.duplicate(true)
	match key:
		"tempo":
			updated["tempo"] = str(selected_value)
		"press_intensity":
			updated["press_intensity"] = float(selected_value)
		"build_up_style":
			updated["build_up_style"] = str(selected_value)
		_:
			return
	_set_matchday_instructions(updated)


func _on_advanced_pressed() -> void:
	if not tactical_panel:
		return
	tactical_panel.show_with_state([], current_matchday_instructions)


func _on_tactical_panel_applied(payload: Dictionary) -> void:
	var updated := current_matchday_instructions.duplicate(true)
	if payload.has("tempo"):
		updated["tempo"] = payload.get("tempo")
	if payload.has("press_intensity"):
		updated["press_intensity"] = payload.get("press_intensity")
	if payload.has("build_up_style"):
		updated["build_up_style"] = payload.get("build_up_style")
	_set_matchday_instructions(updated)


func _set_matchday_instructions(instructions: Dictionary, propagate: bool = true) -> void:
	current_matchday_instructions = _normalize_matchday_instructions(instructions)
	_sync_matchday_controls()
	if propagate:
		_apply_matchday_to_manager()


func _normalize_matchday_instructions(instructions: Dictionary) -> Dictionary:
	var normalized := DEFAULT_MATCHDAY_INSTRUCTIONS.duplicate(true)
	if current_matchday_instructions:
		normalized.merge(current_matchday_instructions, true)
	if instructions:
		normalized.merge(instructions, true)
	var tempo := str(normalized.get("tempo", "normal")).to_lower()
	match tempo:
		"slow":
			normalized["tempo"] = "slow"
		"fast":
			normalized["tempo"] = "fast"
		_:
			normalized["tempo"] = "normal"
	var buildup := str(normalized.get("build_up_style", "Mixed")).to_lower()
	match buildup:
		"short":
			normalized["build_up_style"] = "Short"
		"direct":
			normalized["build_up_style"] = "Direct"
		_:
			normalized["build_up_style"] = "Mixed"
	normalized["press_intensity"] = clampf(
		float(normalized.get("press_intensity", 0.5)), 0.0, 1.0
	)
	return normalized


func _apply_matchday_to_manager() -> void:
	if MatchManager and MatchManager.has_method("set_matchday_instructions"):
		MatchManager.set_matchday_instructions(current_matchday_instructions)
		return
	if MatchManager:
		var legacy_label := _derive_legacy_tactic_label(current_matchday_instructions)
		MatchManager.set_tactic(legacy_label)
	else:
		push_warning("[MatchScreen] MatchManager not available")


func _derive_legacy_tactic_label(instructions: Dictionary) -> String:
	var tempo := str(instructions.get("tempo", "normal")).to_lower()
	var pressing := float(instructions.get("press_intensity", 0.5))
	if tempo == "fast" and pressing >= 0.7:
		return "공격적"
	if tempo == "slow" and pressing <= 0.4:
		return "수비적"
	return "균형"


func _on_substitute_pressed():
	"""Handle substitute button press"""
	print("[MatchScreen] Substitute button pressed")
	# TODO: Open substitute dialog
	# SubstituteDialog.show()


func _on_timeout_pressed():
	"""Handle timeout button press"""
	print("[MatchScreen] Timeout button pressed")

	# Request timeout via MatchManager
	if MatchManager:
		MatchManager.request_timeout()
	else:
		push_warning("[MatchScreen] MatchManager not available")


func _on_skip_pressed():
	"""Skip to end of match"""
	print("[MatchScreen] Skip button pressed")

	# Fast-forward match via MatchManager
	if MatchManager:
		MatchManager.skip_to_end()
		# match_ended signal will handle navigation
	else:
		push_warning("[MatchScreen] MatchManager not available")
		# Fallback: Navigate back to home
		get_tree().change_scene_to_file("res://scenes/HomeImproved_Responsive.tscn")


## Public API for match updates


func update_score(home: int, away: int):
	"""Update match score (called by MatchManager)"""
	home_score = home
	away_score = away
	_populate_current_layout()


func update_time(minutes: int, half: String):
	"""Update match time (called by MatchManager)"""
	current_time = minutes
	current_half = half
	_populate_current_layout()


func add_event(time: int, icon: String, text: String):
	"""Add event to timeline (called by MatchManager)"""
	var event_entry = {"time": time, "icon": icon, "text": text}
	match_events.append(event_entry)
	_append_event_to_layouts(event_entry)


func _start_pending_match_if_needed() -> void:
	if not MatchManager:
		return
	if MatchManager.is_active():
		var state: Dictionary = MatchManager.get_match_state()
		_refresh_summary_state(state)
		match_events = state.get("events", []).duplicate(true)
		_clear_all_event_lists()
		_refresh_events_for_layout(get_active_layout_name())


func _refresh_summary_state(state: Dictionary) -> void:
	if state.is_empty():
		return
	var instructions: Variant = state.get("matchday_instructions", {})
	if instructions is Dictionary and not (instructions as Dictionary).is_empty():
		current_matchday_instructions = _normalize_matchday_instructions(
			instructions as Dictionary
		)
	var score: Array = state.get("score", [home_score, away_score])
	if score.size() >= 2:
		home_score = score[0]
		away_score = score[1]
	current_time = state.get("time", current_time)
	current_half = state.get("half", current_half)
	_populate_current_layout()


func _on_match_started() -> void:
	print("[MatchScreen] Match started")
	match_events.clear()
	_clear_all_event_lists()
	_refresh_summary_state(MatchManager.get_match_state())


func _clear_all_event_lists() -> void:
	for layout_name in layout_event_lists.keys():
		var list: VBoxContainer = layout_event_lists[layout_name]
		_clear_event_list(list)


func _clear_event_list(list: VBoxContainer) -> void:
	if not list:
		return
	for child in list.get_children():
		child.queue_free()


func _append_event_to_layouts(event_entry: Dictionary) -> void:
	for layout_name in layout_event_lists.keys():
		var list: VBoxContainer = layout_event_lists[layout_name]
		if not list:
			continue
		var node := _create_event_entry(layout_name, event_entry)
		list.add_child(node)


func _create_event_entry(layout_name: String, event_entry: Dictionary) -> Control:
	var template: Control = event_templates.get(layout_name, null)
	var instance: Control = null
	if template:
		instance = template.duplicate(true)
	else:
		instance = HBoxContainer.new()
		var time_label_node := Label.new()
		time_label_node.name = "Time"
		instance.add_child(time_label_node)
		var icon_label_node := Label.new()
		icon_label_node.name = "Icon"
		instance.add_child(icon_label_node)
		var text_label_node := Label.new()
		text_label_node.name = "Text"
		instance.add_child(text_label_node)

	var time_label: Label = instance.get_node("Time") if instance.has_node("Time") else null
	if time_label:
		time_label.text = "%02d'" % event_entry.get("time", 0)
	var icon_label: Label = instance.get_node("Icon") if instance.has_node("Icon") else null
	if icon_label:
		icon_label.text = event_entry.get("icon", "•")
	var text_label: Label = instance.get_node("Text") if instance.has_node("Text") else null
	if text_label:
		text_label.text = event_entry.get("text", "")

	return instance


func _refresh_events_for_layout(layout_name: String) -> void:
	if not layout_event_lists.has(layout_name):
		return
	var list: VBoxContainer = layout_event_lists[layout_name]
	_clear_event_list(list)
	for event_entry in match_events:
		var node := _create_event_entry(layout_name, event_entry)
		list.add_child(node)


func _open_lite_match_summary(result: Dictionary) -> void:
	if not is_inside_tree() or not get_tree():
		_show_match_result_dialog(result)
		return
	var payload := _build_lite_summary_payload(result)
	get_tree().root.set_meta("post_match_data", payload)
	if ResourceLoader.exists(POST_MATCH_SUMMARY_SCENE_PATH):
		get_tree().change_scene_to_file(POST_MATCH_SUMMARY_SCENE_PATH)
	else:
		_show_match_result_dialog(result)


func _build_lite_summary_payload(result: Dictionary) -> Dictionary:
	var payload := result.duplicate(true)
	payload["lite_summary"] = true
	var summary: Dictionary = {}
	var nested: Variant = result.get("match_result", {})
	if nested is Dictionary:
		summary = (nested as Dictionary).duplicate(true)
	for key in result.keys():
		if not summary.has(key):
			summary[key] = result[key]
	var opponent_name := str(
		result.get("opponent_name", result.get("opponent", result.get("away_team_name", "")))
	)
	if not opponent_name.is_empty():
		if not summary.has("opponent_name"):
			summary["opponent_name"] = opponent_name
		if not summary.has("away_team"):
			summary["away_team"] = opponent_name
	if not summary.has("home_team"):
		summary["home_team"] = result.get("home_team_name", "My Team")
	payload["match_result"] = summary
	if not payload.has("match_info"):
		payload["match_info"] = {
			"home_team": summary.get("home_team", "My Team"),
			"away_team": summary.get("away_team", opponent_name)
		}
	return payload


func _show_match_result_dialog(result: Dictionary) -> void:
        var dialog := AcceptDialog.new()
        dialog.title = "경기 종료"
        var goals_scored := int(result.get("goals_scored", 0))
        var goals_conceded := int(result.get("goals_conceded", 0))
        var pen_suffix := _MatchTimeFormatter.format_penalty_shootout_suffix(result)

        var lines := PackedStringArray()
        lines.append(str(result.get("result", "경기 종료")))
        lines.append("%s %d : %d%s" % ["My Team", goals_scored, goals_conceded, pen_suffix])

        var shootout := _MatchTimeFormatter.extract_penalty_shootout(result)
        if not shootout.is_empty():
                var winner_is_home := bool(shootout.get("winner_is_home", false))
                lines.append("승부차기: %s 승" % ["홈" if winner_is_home else "원정"])

        dialog.dialog_text = "\n".join(lines)
        add_child(dialog)
        dialog.confirmed.connect(Callable(self, "_queue_free_dialog").bind(dialog), Object.CONNECT_REFERENCE_COUNTED)
        # Use 'canceled' for closing via X or cancel, instead of non-existent 'closed'
        dialog.canceled.connect(Callable(self, "_queue_free_dialog").bind(dialog), Object.CONNECT_REFERENCE_COUNTED)
        dialog.popup_centered()


func _queue_free_dialog(dialog: Window) -> void:
	if dialog and dialog.is_inside_tree():
		dialog.queue_free()


## Debug helpers


func print_layout_debug_info():
	"""Print detailed layout information for debugging"""
	print_layout_info()  # From AdaptiveLayoutContainer

	print("\n[MatchScreen] Match State:")
	print("  Score: %d : %d" % [home_score, away_score])
	print("  Time: %d' (%s)" % [current_time, current_half])
	print(
		"  Matchday: tempo=%s, press=%.2f, build_up=%s"
		% [
			current_matchday_instructions.get("tempo", "normal"),
			float(current_matchday_instructions.get("press_intensity", 0.5)),
			current_matchday_instructions.get("build_up_style", "Mixed")
		]
	)

	if PlatformManager:
		print("\n[PlatformManager]:")
		print("  Platform: %s" % PlatformManager.get_platform_name())
		print("  Orientation: %s" % PlatformManager.get_orientation_name())
		print("  Viewport: %v" % PlatformManager.viewport_size)
