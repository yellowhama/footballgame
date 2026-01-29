extends Control
class_name PostMatchStatisticsScreen

@onready var home_team_label: Label = $Header/HeaderContent/VBoxContainer/MatchInfo/HomeTeamLabel
@onready var away_team_label: Label = $Header/HeaderContent/VBoxContainer/MatchInfo/AwayTeamLabel
@onready var home_score_label: Label = $Header/HeaderContent/VBoxContainer/ScoreDisplay/HomeScoreLabel
@onready var away_score_label: Label = $Header/HeaderContent/VBoxContainer/ScoreDisplay/AwayScoreLabel
@onready var team_stats_panel = $ScrollContainer/ContentContainer/TeamStatsPanel
@onready var shot_map_canvas = $ScrollContainer/ContentContainer/ShotMapPanel/ShotMapCanvas
@onready var analytics_panel: AdvancedAnalyticsPanel = $ScrollContainer/ContentContainer/AdvancedAnalyticsPanel
@onready var continue_button: Button = $ContinueButton
@onready var analysis_button: Button = $AnalysisButton if has_node("AnalysisButton") else null
@onready var content_container: VBoxContainer = $ScrollContainer/ContentContainer
@onready var shot_map_panel: Panel = $ScrollContainer/ContentContainer/ShotMapPanel
@onready var player_ratings_panel: Panel = $ScrollContainer/ContentContainer/PlayerRatingsPanel
@onready var my_player_summary_panel: Panel = $ScrollContainer/ContentContainer/MyPlayerSummaryPanel
@onready var mvp_label: Label = $ScrollContainer/ContentContainer/MyPlayerSummaryPanel/TitleBar/MVPLabel
@onready
var my_player_heat_canvas: HeatMapCanvas = $ScrollContainer/ContentContainer/MyPlayerSummaryPanel/MyPlayerHeatMapCanvas
@onready var my_player_rating_label: Label = (
	$ScrollContainer/ContentContainer/MyPlayerSummaryPanel/TitleBar/MyPlayerRatingLabel
	if $ScrollContainer/ContentContainer/MyPlayerSummaryPanel/TitleBar.has_node("MyPlayerRatingLabel")
	else null
)
@onready var my_player_xp_label: Label = (
	$ScrollContainer/ContentContainer/MyPlayerSummaryPanel/TitleBar/MyPlayerXPLabel
	if $ScrollContainer/ContentContainer/MyPlayerSummaryPanel/TitleBar.has_node("MyPlayerXPLabel")
	else null
)

const PITCH_SIZE := Vector2(105.0, 68.0)
const SESSION_VIEWER_SCENE_PATH := "res://scenes/match_pipeline/examples/HorizontalMatchSessionViewer.tscn"
const LITE_HIGHLIGHT_OPTIONS := [
	{"label": "Full", "value": "full"},
	{"label": "Key", "value": "simple"},
	{"label": "None", "value": "none"}
]
const DEFAULT_TEAM_TACTICS_PARAMETERS := {
	"attacking_intensity": 0.5,
	"defensive_line_height": 0.5,
	"width": 0.7,
	"pressing_trigger": 0.5,
	"tempo": 0.6,
	"directness": 0.5
}

var match_data: Dictionary = {}
var source_payload: Dictionary = {}
var _heat_map_cache: Dictionary = {}
var _pass_map_cache: Dictionary = {}
var _roster_home_ids: Array = []
var _roster_away_ids: Array = []
var _player_name_lookup: Dictionary = {}
var _roster_meta: Dictionary = {}
var _lite_mode: bool = false
var _highlight_level: String = "full"
var _lite_action_bar: HBoxContainer = null
var _highlight_selector: OptionButton = null
var _advanced_button: Button = null
var _recommendation_panel: Panel = null
var _recommendation_label: Label = null
var _recommendation_apply: Button = null
var _recommendation_ignore: Button = null
var _pending_recommendation: Dictionary = {}
var _analysis_report_cache: Dictionary = {}
var _interpretation_panel: Panel = null
var _interpretation_headline_label: Label = null
var _interpretation_subline_label: Label = null
var _interpretation_cards_container: VBoxContainer = null


func _ready() -> void:
	if continue_button:
		continue_button.pressed.connect(_on_continue_pressed)

	if analysis_button:
		analysis_button.pressed.connect(_on_analysis_pressed)

	## P2.3: Load from meta
	var root := get_tree().root
	if root.has_meta("post_match_data"):
		var data: Dictionary = root.get_meta("post_match_data")
		root.remove_meta("post_match_data")
		set_match_data(data)


func set_match_data(data: Dictionary) -> void:
        source_payload = data.duplicate(true) if data is Dictionary else {}     
        match_data = _extract_match_payload(source_payload)
        _analysis_report_cache = {}
        _lite_mode = bool(source_payload.get("lite_summary", false))
        if _lite_mode:
                _apply_lite_layout()
        _populate_header()
        _populate_penalty_shootout()
        _populate_team_stats()
        _populate_rulebook_report_v1()
        _update_recommendation_bar()
        _populate_interpretation_v1()
        _populate_shot_map()
        _populate_player_ratings()
        _populate_advanced_analytics()
        _populate_my_player_summary()


func _apply_lite_layout() -> void:
	_ensure_lite_action_bar()
	for panel in [shot_map_panel, player_ratings_panel, analytics_panel, my_player_summary_panel]:
		if panel:
			panel.visible = false
	if analysis_button:
		analysis_button.visible = false
	if team_stats_panel:
		var title_label: Label = team_stats_panel.get_node_or_null("TitleBar/TitleLabel")
		if title_label:
			title_label.text = "경기 요약"
	_update_advanced_button_state()


## RuleBook P2 Analytics (v1): Consume MatchEvent stream only (no referee logic re-evaluation).
func _populate_rulebook_report_v1() -> void:
	if not content_container:
		return

	var existing := content_container.get_node_or_null("RulebookReportPanel")
	if existing and is_instance_valid(existing):
		existing.queue_free()

	var events := _get_events()
	if events.is_empty():
		return

	var offside_count := 0
	var offside_margin_sum := 0.0

	var foul_count := 0
	var foul_severity_hist := {"CARELESS": 0, "RECKLESS": 0, "EXCESSIVE_FORCE": 0}

	var yellow_count := 0
	var red_count := 0
	var var_count := 0

	for ev in events:
		var e := _coerce_event_dict(ev)
		if e.is_empty():
			continue

		var event_type := str(e.get("event_type", e.get("type", ""))).to_lower()
		if event_type == "yellow_card":
			yellow_count += 1
		elif event_type == "red_card":
			red_count += 1

		var details: Dictionary = {}
		if e.get("details") is Dictionary:
			details = e.get("details") as Dictionary

		if details.get("var_review") is Dictionary:
			var_count += 1

		if details.get("offside_details") is Dictionary:
			var offside: Dictionary = details.get("offside_details") as Dictionary
			if offside.has("margin_m"):
				offside_count += 1
				offside_margin_sum += float(offside.get("margin_m"))

		if details.get("foul_details") is Dictionary:
			var foul: Dictionary = details.get("foul_details") as Dictionary
			foul_count += 1
			var severity := str(foul.get("severity", "")).to_upper()
			if foul_severity_hist.has(severity):
				foul_severity_hist[severity] = int(foul_severity_hist[severity]) + 1

	var panel := Panel.new()
	panel.name = "RulebookReportPanel"
	panel.custom_minimum_size = Vector2(0, 220)
	panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Reuse the same panel style as TeamStatsPanel when possible.
	if team_stats_panel and team_stats_panel is Control:
		var style := (team_stats_panel as Control).get_theme_stylebox("panel")
		if style:
			panel.add_theme_stylebox_override("panel", style)

	var margin := MarginContainer.new()
	margin.name = "Margin"
	margin.add_theme_constant_override("margin_left", 16)
	margin.add_theme_constant_override("margin_right", 16)
	margin.add_theme_constant_override("margin_top", 12)
	margin.add_theme_constant_override("margin_bottom", 12)
	panel.add_child(margin)

	var vbox := VBoxContainer.new()
	vbox.name = "VBox"
	vbox.add_theme_constant_override("separation", 8)
	margin.add_child(vbox)

	var title := Label.new()
	title.name = "Title"
	title.text = "RuleBook report"
	title.add_theme_font_size_override("font_size", 18)
	vbox.add_child(title)

	var offside_label := Label.new()
	offside_label.name = "OffsideSummary"
	if offside_count > 0:
		var mean_margin := offside_margin_sum / float(offside_count)
		offside_label.text = "Offside: %d (avg margin %.2fm)" % [offside_count, mean_margin]
	else:
		offside_label.text = "Offside: 0"
	vbox.add_child(offside_label)

	var foul_label := Label.new()
	foul_label.name = "FoulSummary"
	foul_label.text = "Fouls: %d" % foul_count
	vbox.add_child(foul_label)

	# Severity histogram (simple bars)
	var max_sev := 1
	for k in ["CARELESS", "RECKLESS", "EXCESSIVE_FORCE"]:
		max_sev = max(max_sev, int(foul_severity_hist.get(k, 0)))

	var sev_colors := {
		"CARELESS": Color(0.6, 0.8, 0.95, 0.9),
		"RECKLESS": Color(0.98, 0.85, 0.2, 0.9),
		"EXCESSIVE_FORCE": Color(0.95, 0.35, 0.35, 0.9)
	}
	for k in ["CARELESS", "RECKLESS", "EXCESSIVE_FORCE"]:
		var row := HBoxContainer.new()
		row.name = "SeverityRow_%s" % k
		row.add_theme_constant_override("separation", 10)

		var label := Label.new()
		label.text = "%s: %d" % [k.replace("_", " ").to_lower().capitalize(), int(foul_severity_hist.get(k, 0))]
		label.custom_minimum_size = Vector2(160, 0)
		row.add_child(label)

		var bar_bg := ColorRect.new()
		bar_bg.color = Color(1, 1, 1, 0.08)
		bar_bg.custom_minimum_size = Vector2(200, 10)
		row.add_child(bar_bg)

		var bar := ColorRect.new()
		bar.color = sev_colors.get(k, Color(1, 1, 1, 0.6))
		var ratio := float(foul_severity_hist.get(k, 0)) / float(max_sev)
		bar.custom_minimum_size = Vector2(200.0 * ratio, 10)
		bar_bg.add_child(bar)

		vbox.add_child(row)

	var cards_label := Label.new()
	cards_label.name = "CardSummary"
	cards_label.text = "Cards: %dY / %dR" % [yellow_count, red_count]
	vbox.add_child(cards_label)

	if var_count > 0:
		var var_label := Label.new()
		var_label.name = "VarSummary"
		var_label.text = "VAR reviews: %d" % var_count
		vbox.add_child(var_label)

	content_container.add_child(panel)

	# Place after TeamStatsPanel when possible (before heavy panels).
	if team_stats_panel and is_instance_valid(team_stats_panel):
		var idx := team_stats_panel.get_index() + 1
		content_container.move_child(panel, min(idx, content_container.get_child_count() - 1))


func _coerce_event_dict(value: Variant) -> Dictionary:
	if value is Dictionary:
		return (value as Dictionary).duplicate(true)
	if value is String:
		var parsed: Variant = JSON.parse_string(String(value))
		if parsed is Dictionary:
			return parsed as Dictionary
	return {}

func _ensure_lite_action_bar() -> void:
	if _lite_action_bar and is_instance_valid(_lite_action_bar):
		return
	if not content_container:
		return
        var bar := HBoxContainer.new()
        bar.name = "LiteActionBar"
        bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        bar.add_theme_constant_override("separation", 16)
        var label := Label.new()
        label.text = "하이라이트"
        bar.add_child(label)
        var selector := OptionButton.new()
        selector.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        _configure_highlight_selector(selector)
        bar.add_child(selector)
        var advanced := Button.new()
        advanced.text = "Advanced"
        advanced.pressed.connect(_on_advanced_pressed)
        bar.add_child(advanced)
	content_container.add_child(bar)
	var insert_index := 0
	if _recommendation_panel and is_instance_valid(_recommendation_panel):
		insert_index = 1
	content_container.move_child(bar, insert_index)
	_lite_action_bar = bar
	_highlight_selector = selector
	_advanced_button = advanced

func _configure_highlight_selector(selector: OptionButton) -> void:
        selector.clear()
        var default_index := 0
        for idx in range(LITE_HIGHLIGHT_OPTIONS.size()):
                var option: Dictionary = LITE_HIGHLIGHT_OPTIONS[idx]
                selector.add_item(option.get("label", ""), idx)
                selector.set_item_metadata(idx, option.get("value", "full"))
                if option.get("value", "") == _highlight_level:
                        default_index = idx
        selector.select(default_index)
        _highlight_level = str(LITE_HIGHLIGHT_OPTIONS[default_index].get("value", "full"))
        selector.item_selected.connect(_on_highlight_selected)

func _on_highlight_selected(index: int) -> void:
        if not _highlight_selector:
                return
        if index < 0 or index >= _highlight_selector.item_count:
                return
        _highlight_level = str(_highlight_selector.get_item_metadata(index))

func _update_advanced_button_state() -> void:
	if not _advanced_button:
		return
	var record := _build_timeline_record()
	_advanced_button.disabled = not _has_timeline_available(record)


func _update_recommendation_bar() -> void:
	var recommendation := _build_recommendation()
	_pending_recommendation = recommendation.duplicate(true)
	if recommendation.is_empty():
		if _recommendation_panel:
			_recommendation_panel.visible = false
		return
	_ensure_recommendation_bar()
	if _recommendation_label:
		var reason := recommendation.get("reason", "")
		var suggestion := recommendation.get("recommendation", "")
		_recommendation_label.text = "%s → %s" % [reason, suggestion]
	_recommendation_panel.visible = true


func _ensure_recommendation_bar() -> void:
	if _recommendation_panel and is_instance_valid(_recommendation_panel):
		return
	if not content_container:
		return
	var panel := Panel.new()
	panel.name = "RecommendationPanel"
	panel.custom_minimum_size = Vector2(0, 60)
	panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var hbox := HBoxContainer.new()
	hbox.name = "HBox"
	hbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_theme_constant_override("separation", 12)
	panel.add_child(hbox)
	var label := Label.new()
	label.name = "RecommendationLabel"
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.autowrap_mode = 2
	hbox.add_child(label)
	var apply_button := Button.new()
	apply_button.name = "ApplyButton"
	apply_button.text = "적용"
	apply_button.custom_minimum_size = Vector2(80, 0)
	apply_button.pressed.connect(_on_recommendation_apply_pressed)
	hbox.add_child(apply_button)
	var ignore_button := Button.new()
	ignore_button.name = "IgnoreButton"
	ignore_button.text = "무시"
	ignore_button.custom_minimum_size = Vector2(80, 0)
	ignore_button.pressed.connect(_on_recommendation_ignore_pressed)
	hbox.add_child(ignore_button)
	content_container.add_child(panel)
	content_container.move_child(panel, 0)
	_recommendation_panel = panel
	_recommendation_label = label
	_recommendation_apply = apply_button
	_recommendation_ignore = ignore_button


func _on_recommendation_apply_pressed() -> void:
	if _pending_recommendation.is_empty():
		return
	var instructions: Dictionary = _pending_recommendation.get("instructions", {})
	if MyTeamData and MyTeamData.has_method("get_team_tactics") and MyTeamData.has_method("set_team_tactics_custom"):
		var current := MyTeamData.get_team_tactics()
		var parameters: Dictionary = {}
		if current is Dictionary and current.has("parameters") and current.parameters is Dictionary:
			parameters = current.parameters.duplicate(true)
		if parameters.is_empty():
			parameters = DEFAULT_TEAM_TACTICS_PARAMETERS.duplicate(true)
		var updated := _merge_tactics_parameters(parameters, instructions)
		var success := bool(MyTeamData.set_team_tactics_custom(updated))
		_notify_recommendation_result(success, "전술 추천 적용")
	else:
		_notify_recommendation_result(false, "전술 시스템을 찾지 못했습니다")
	_dismiss_recommendation()


func _on_recommendation_ignore_pressed() -> void:
	_dismiss_recommendation()


func _dismiss_recommendation() -> void:
	if _recommendation_panel:
		_recommendation_panel.visible = false


func _notify_recommendation_result(success: bool, message: String) -> void:
	var text := "✅ %s" % message if success else "⚠️ %s" % message
	if has_node("/root/UIService"):
		var ui = get_node("/root/UIService")
		if ui and ui.has_method("toast"):
			ui.call("toast", text)
			return
	print("[MatchSummary] %s" % text)


func _merge_tactics_parameters(
	base: Dictionary, instructions: Dictionary
) -> Dictionary:
	var result := base.duplicate(true)
	var tempo := _map_tempo_to_parameter(str(instructions.get("tempo", "normal")))
	var press := clampf(float(instructions.get("press_intensity", 0.5)), 0.0, 1.0)
	var directness := _map_buildup_to_parameter(
		str(instructions.get("build_up_style", "Mixed"))
	)
	result["tempo"] = tempo
	result["pressing_trigger"] = press
	result["directness"] = directness
	return result


func _map_tempo_to_parameter(tempo: String) -> float:
	match tempo.to_lower():
		"slow":
			return 0.45
		"fast":
			return 0.75
		_:
			return 0.6


func _map_buildup_to_parameter(style: String) -> float:
	match style.to_lower():
		"short":
			return 0.35
		"direct":
			return 0.7
		_:
			return 0.5


func _build_recommendation() -> Dictionary:
	var home_stats := _collect_team_stats("home")
	var pass_accuracy := _parse_percentage(home_stats.get("pass_accuracy", 0))
	if pass_accuracy <= 0:
		return {}
	if pass_accuracy < 70.0:
		return {
			"reason": "패스 성공률 낮음 (%.0f%%)" % pass_accuracy,
			"recommendation": "템포 느림 + 숏 빌드업",
			"instructions": {"tempo": "slow", "press_intensity": 0.5, "build_up_style": "Short"}
		}
	if pass_accuracy > 85.0:
		return {
			"reason": "패스 성공률 높음 (%.0f%%)" % pass_accuracy,
			"recommendation": "템포 빠름 + 직접 빌드업",
			"instructions": {"tempo": "fast", "press_intensity": 0.5, "build_up_style": "Direct"}
		}
	return {}


func _parse_percentage(value: Variant) -> float:
	if value is String:
		var cleaned := String(value).replace("%", "").strip_edges()
		if cleaned.is_valid_float():
			return float(cleaned)
		return 0.0
	if typeof(value) == TYPE_FLOAT or typeof(value) == TYPE_INT:
		var numeric := float(value)
		if numeric <= 1.0:
			return numeric * 100.0
		return numeric
	return 0.0

func _build_timeline_record() -> Dictionary:
        var record := source_payload.duplicate(true)
        if match_data is Dictionary and not match_data.is_empty() and not record.has("match_result"):
                record["match_result"] = (match_data as Dictionary).duplicate(true)
        var match_result: Dictionary = {}
        if record.get("match_result") is Dictionary:
                match_result = (record.get("match_result") as Dictionary).duplicate(true)
        var opponent_name := _resolve_opponent_name(record, match_result)
        if not match_result.is_empty():
                if not match_result.has("home_team") and record.has("home_team_name"):
                        match_result["home_team"] = record.get("home_team_name", "")
                if not opponent_name.is_empty():
                        if not match_result.has("away_team"):
                                match_result["away_team"] = opponent_name
                        if not match_result.has("opponent_name"):
                                match_result["opponent_name"] = opponent_name
        if not match_result.is_empty():
                record["match_result"] = match_result
        if not record.has("opponent_name") and not opponent_name.is_empty():
                record["opponent_name"] = opponent_name
        if not record.has("home_team_name") and record.has("home_team"):
                record["home_team_name"] = record.get("home_team", "")
        record["highlight_level"] = _highlight_level
        return record

func _resolve_opponent_name(record: Dictionary, match_result: Dictionary = {}) -> String:
        var candidates = [
                record.get("opponent_name", ""),
                record.get("opponent", ""),
                record.get("away_team_name", ""),
                match_result.get("opponent_name", ""),
                match_result.get("away_team", "")
        ]
        for candidate in candidates:
                if str(candidate) != "":
                        return str(candidate)
        return ""

func _has_timeline_available(record: Dictionary) -> bool:
        if record.is_empty():
                return false
        var position_data: Variant = record.get("position_data", {})
        if position_data is Dictionary and not position_data.is_empty():
                return true
        if not _get_events().is_empty():
                return true
        var doc_variant: Variant = record.get("timeline_doc", {})
        if doc_variant is Dictionary and not (doc_variant as Dictionary).is_empty():
                return true
        var match_result: Variant = record.get("match_result", {})
        if match_result is Dictionary:
                var result_dict: Dictionary = match_result
                if result_dict.has("events") and result_dict.events is Array and result_dict.events.size() > 0:
                        return true
                var mr_doc: Variant = result_dict.get("timeline_doc", {})
                if mr_doc is Dictionary and not (mr_doc as Dictionary).is_empty():
                        return true
        return false

func _on_advanced_pressed() -> void:
        _open_timeline_view()

func _open_timeline_view() -> void:
        var record := _build_timeline_record()
        if not _has_timeline_available(record):
                _update_advanced_button_state()
                return
        if not is_inside_tree() or not get_tree():
                return
        var current_scene_path := ""
        if get_tree().current_scene:
                current_scene_path = get_tree().current_scene.scene_file_path
        if MatchTimelineHolder:
                MatchTimelineHolder.set_timeline_data(record, current_scene_path)
        _preload_position_data(record)
        get_tree().change_scene_to_file(SESSION_VIEWER_SCENE_PATH)

func _preload_position_data(record: Dictionary) -> void:
        var position_data: Dictionary = record.get("position_data", {})
        if position_data.is_empty():
                return
        var rosters: Dictionary = {}
        if record.get("rosters") is Dictionary:
                rosters = (record.get("rosters") as Dictionary).duplicate(true)
        var match_result_variant: Variant = record.get("match_result", {})
        if rosters.is_empty() and match_result_variant is Dictionary:
                var mr: Dictionary = match_result_variant
                if mr.get("rosters") is Dictionary:
                        rosters = (mr.get("rosters") as Dictionary).duplicate(true)
        var events := _get_events()
        var controller := get_node_or_null("/root/MatchTimelineController")
        if not controller:
                return
        if not controller.has_method("load_position_data"):
                return
        controller.load_position_data(position_data, rosters, events)

func _populate_my_player_summary() -> void:
        if _lite_mode:
                return
	if not mvp_label and not my_player_rating_label and not my_player_xp_label:
		return

	var result: Dictionary = {}
	if match_data.has("match_result") and match_data.match_result is Dictionary:
		result = match_data.match_result
	elif match_data.has("result") and match_data.result is Dictionary:
		result = match_data.result

	var player_rating: float = float(result.get("player_rating", 0.0))
	var goals: int = int(result.get("player_goals", 0))
	var assists: int = int(result.get("player_assists", 0))

	# MVP 라벨: 평점/골/어시 기준 간단 요약
	if mvp_label:
		if player_rating > 0.0:
			var label_text := "평점 %.1f" % player_rating
			if goals > 0 or assists > 0:
				label_text += " | %d골 %d도움" % [goals, assists]
			if player_rating >= 8.5:
				label_text = "MVP! " + label_text
			mvp_label.text = label_text
		else:
			mvp_label.text = ""

	# 평점 수치 전용 라벨 (있을 경우)
	if my_player_rating_label:
		if player_rating > 0.0:
			my_player_rating_label.text = "오늘의 평점: %.1f / 10.0" % player_rating
		else:
			my_player_rating_label.text = ""

	# XP 보상 표시
	if my_player_xp_label:
		var rewards: Dictionary = result.get("match_rewards", {})
		if rewards.is_empty():
			my_player_xp_label.text = ""
		else:
			var base_xp: int = int(rewards.get("base_xp", 0))
			var bonus_xp: int = int(rewards.get("bonus_xp", 0))
			var total_xp: int = int(rewards.get("xp_gained", base_xp + bonus_xp))
			var before_level: int = int(rewards.get("level_before", 1))
			var after_level: int = int(rewards.get("level_after", before_level))
			var is_level_up: bool = bool(rewards.get("is_level_up", false)) and after_level > before_level

			var xp_text := "XP +%d (기본 %d + 보너스 %d)" % [total_xp, base_xp, bonus_xp]
			if is_level_up:
				xp_text += " | LEVEL UP! Lv.%d → Lv.%d" % [before_level, after_level]
			my_player_xp_label.text = xp_text


func _populate_header() -> void:
        if home_team_label:
                home_team_label.text = _get_team_name("home")
        if away_team_label:
                away_team_label.text = _get_team_name("away")
        if home_score_label:
                home_score_label.text = str(_get_score("home"))
        if away_score_label:
                away_score_label.text = str(_get_score("away"))


func _get_penalty_shootout() -> Dictionary:
        var candidate: Variant = match_data.get("penalty_shootout", null)
        if candidate is Dictionary:
                return candidate as Dictionary
        if candidate is String:
                var parsed: Variant = JSON.parse_string(String(candidate))
                if parsed is Dictionary:
                        return parsed as Dictionary
        return {}


func _populate_penalty_shootout() -> void:
        _update_penalty_shootout_header_label()
        _update_penalty_shootout_panel()


func _update_penalty_shootout_header_label() -> void:
        var shootout: Dictionary = _get_penalty_shootout()
        var header_vbox := get_node_or_null("Header/HeaderContent/VBoxContainer") as VBoxContainer
        if not header_vbox:
                return

        var existing := header_vbox.get_node_or_null("PenaltyShootoutLabel") as Label
        if shootout.is_empty():
                if existing:
                        existing.visible = false
                return

        var label := existing if existing else Label.new()
        if not existing:
                label.name = "PenaltyShootoutLabel"
                label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
                label.add_theme_color_override("font_color", Color(0.95, 0.95, 0.8))
                header_vbox.add_child(label)

        var goals_home := int(shootout.get("goals_home", 0))
        var goals_away := int(shootout.get("goals_away", 0))
        var winner_is_home := bool(shootout.get("winner_is_home", false))
        var winner_label := "홈" if winner_is_home else "원정"
        label.text = "승부차기: %d - %d (%s 승)" % [goals_home, goals_away, winner_label]
        label.visible = true


func _update_penalty_shootout_panel() -> void:
        if not content_container:
                return

        var existing := content_container.get_node_or_null("PenaltyShootoutPanel")
        if existing and is_instance_valid(existing):
                existing.queue_free()

        var shootout: Dictionary = _get_penalty_shootout()
        if shootout.is_empty():
                return

        var panel := Panel.new()
        panel.name = "PenaltyShootoutPanel"
        panel.custom_minimum_size = Vector2(0, 220)
        panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

        # Reuse the same panel style as TeamStatsPanel when possible.
        if team_stats_panel and team_stats_panel is Control:
                var style := (team_stats_panel as Control).get_theme_stylebox("panel")
                if style:
                        panel.add_theme_stylebox_override("panel", style)

        var margin := MarginContainer.new()
        margin.add_theme_constant_override("margin_left", 24)
        margin.add_theme_constant_override("margin_right", 24)
        margin.add_theme_constant_override("margin_top", 16)
        margin.add_theme_constant_override("margin_bottom", 16)
        panel.add_child(margin)

        var vbox := VBoxContainer.new()
        vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        vbox.add_theme_constant_override("separation", 10)
        margin.add_child(vbox)

        var title := Label.new()
        title.text = "승부차기"
        title.add_theme_font_size_override("font_size", 24)
        vbox.add_child(title)

        var goals_home := int(shootout.get("goals_home", 0))
        var goals_away := int(shootout.get("goals_away", 0))
        var winner_is_home := bool(shootout.get("winner_is_home", false))
        var summary := Label.new()
        summary.text = "PEN %d - %d | 승자: %s" % [goals_home, goals_away, ("홈" if winner_is_home else "원정")]
        vbox.add_child(summary)

        var kicks_variant: Variant = shootout.get("kicks", [])
        var kicks: Array = kicks_variant if kicks_variant is Array else []
        if kicks.is_empty():
                var empty_label := Label.new()
                empty_label.text = "(kick log 없음)"
                empty_label.modulate = Color(0.8, 0.8, 0.8)
                vbox.add_child(empty_label)
        else:
                var kick_box := VBoxContainer.new()
                kick_box.add_theme_constant_override("separation", 4)
                vbox.add_child(kick_box)

                for entry in kicks:
                        if not (entry is Dictionary):
                                continue
                        var kick: Dictionary = entry
                        var kick_index := int(kick.get("kick_index", 0))
                        var is_home := bool(kick.get("is_home_team", false))
                        var kicker := str(kick.get("kicker_name", ""))
                        var scored := bool(kick.get("scored", false))
                        var mark := "○" if scored else "×"
                        var side := "H" if is_home else "A"

                        var line := Label.new()
                        line.text = "%d. [%s] %s %s" % [kick_index, side, kicker, mark]
                        kick_box.add_child(line)

        # Insert at top of the scroll content (above TeamStatsPanel).
        content_container.add_child(panel)
        content_container.move_child(panel, 0)


func _populate_team_stats() -> void:
        if not team_stats_panel:
                return

	var home_stats = _collect_team_stats("home")
	var away_stats = _collect_team_stats("away")
        team_stats_panel.set_stats(home_stats, away_stats, _lite_mode)


func _populate_shot_map() -> void:
        if _lite_mode:
                return
	if not shot_map_canvas:
		return

	var shots: Array = []
	var shot_events: Array = _get_events()
	for event in shot_events:
		if not (event is Dictionary):
			continue
		var event_type = str(event.get("type", "")).to_lower()
		if event_type not in ["shot", "goal", "shot_on_target"]:
			continue
		var position = _extract_coordinates(event)
		if position == Vector2.ZERO and not _event_has_coordinates(event):
			continue
		var team_id = _parse_team_identifier(event.get("team_id", event.get("team", 0)))
		shots.append(
			{
				"x": position.x,
				"y": position.y,
				"team": "home" if team_id == 0 else "away",
				"result": str(event.get("result", event.get("outcome", event_type)))
			}
		)
	shot_map_canvas.set_shots(shots)


func _extract_coordinates(event: Dictionary) -> Vector2:
	var position_sources: Array = [
		event.get("coordinates"), event.get("position"), event.get("actual_position"), event.get("target_position")
	]

	var base = event.get("base", {})
	if base is Dictionary:
		position_sources.append(base.get("position"))
		position_sources.append(base)

	for candidate in position_sources:
		if candidate is Dictionary:
			if candidate.has("x") and candidate.has("y"):
				return Vector2(float(candidate.get("x")), float(candidate.get("y")))
		elif candidate is Vector2:
			return candidate

	return Vector2.ZERO


func _event_has_coordinates(event: Dictionary) -> bool:
	for key in ["coordinates", "position", "actual_position", "target_position"]:
		var value = event.get(key)
		if value is Dictionary and value.has("x") and value.has("y"):
			return true
		if value is Vector2:
			return true

	var base = event.get("base", {})
	if base is Dictionary:
		if base.has("x") and base.has("y"):
			return true
		var base_position = base.get("position")
		if base_position is Dictionary and base_position.has("x") and base_position.has("y"):
			return true
	return false


func _populate_player_ratings() -> void:
        if _lite_mode:
                return
	if not player_ratings_panel:
		return

	var ratings = _extract_player_ratings()
	var home_entries = _build_roster_entries("home")
	var away_entries = _build_roster_entries("away")

	var roster_home: Array = []
	for entry in home_entries:
		roster_home.append(entry.get("id", ""))

	var roster_away: Array = []
	for entry in away_entries:
		roster_away.append(entry.get("id", ""))

	var payload = {
		"player_ratings": ratings,
		"roster_home": roster_home,
		"roster_away": roster_away,
		"players": _build_player_name_map(home_entries, away_entries),
		"player_positions": _build_player_position_map(home_entries, away_entries),
		"roster_meta": {"home": home_entries, "away": away_entries},
		"events": _get_events()
	}

	player_ratings_panel.set_match_data(payload)
	_roster_meta = payload.get("roster_meta", {}).duplicate(true)
	_roster_home_ids = []
	for id in roster_home:
		_roster_home_ids.append(str(id))
	_roster_away_ids = []
	for id in roster_away:
		_roster_away_ids.append(str(id))
	_player_name_lookup = payload.get("players", {}).duplicate(true)
	_update_mvp_label(ratings, payload)


func _populate_advanced_analytics() -> void:
	if _lite_mode:
		return
	if not analytics_panel:
		return

	_heat_map_cache = _build_heat_map_cache()
	_pass_map_cache = _build_pass_map_cache()
	analytics_panel.set_heat_map_cache(_heat_map_cache)
	analytics_panel.set_pass_map_cache(_pass_map_cache)

	# DSA v1: Additive minute-series overlay (read-only; do not compute in render loop).
	var duration_minutes := int(match_data.get("duration_minutes", 90))
	var match_info_variant: Variant = match_data.get("match_info", {})
	if match_info_variant is Dictionary:
		duration_minutes = int((match_info_variant as Dictionary).get("duration_minutes", duration_minutes))
	duration_minutes = clamp(duration_minutes, 1, 180)

        var dsa_series: Dictionary = {}

        # Prefer Rust authoritative DSA minute series when available (works for saved loads).
        var analysis_report := _get_analysis_report_cached()
        if not analysis_report.is_empty():
                var dsa_summary_variant: Variant = analysis_report.get("dsa_summary", null)
                if dsa_summary_variant is Dictionary:
                        var dsa_summary: Dictionary = dsa_summary_variant
                        var minute_series_variant: Variant = dsa_summary.get("minute_series", {})
                        if minute_series_variant is Dictionary:
                                dsa_series = minute_series_variant

        # Fallback to runtime DSA buffers (overlay-only; may be empty if not run in this session).
        if dsa_series.is_empty():
                var dsa := get_node_or_null("/root/DistributedSensingManager")
                if dsa and dsa.has_method("get_minute_series"):
                        dsa_series = dsa.get_minute_series(duration_minutes)
        if analytics_panel.has_method("set_dsa_minute_series"):
                analytics_panel.set_dsa_minute_series(dsa_series, duration_minutes)

	# Phase E: Set advanced analytics data
	if match_data.has("statistics"):
		var stats = match_data.statistics
		if stats.has("possession_zones_home"):
			analytics_panel.set_possession_zones_home(stats.possession_zones_home)
		if stats.has("possession_zones_away"):
			analytics_panel.set_possession_zones_away(stats.possession_zones_away)
		if stats.has("heat_map_data_home"):
			analytics_panel.set_team_heat_map_home(stats.heat_map_data_home)
		if stats.has("heat_map_data_away"):
			analytics_panel.set_team_heat_map_away(stats.heat_map_data_away)

        _update_my_player_heatmap()


func _get_analysis_report_cached() -> Dictionary:
        if not _analysis_report_cache.is_empty():
                return _analysis_report_cache
        if FootballRustEngine and FootballRustEngine._rust_simulator:
                var bridge := FootballRustEngine._rust_simulator
                if bridge and bridge.has_method("get_match_analysis"):
                        var report: Dictionary = bridge.get_match_analysis(JSON.stringify(match_data))
                        if report and not report.has("error"):
                                _analysis_report_cache = (report as Dictionary).duplicate(true)
                                return _analysis_report_cache
        return {}


func _populate_interpretation_v1() -> void:
        _ensure_interpretation_panel()
        if not _interpretation_panel:
                return

        var analysis_report := _get_analysis_report_cached()
        var interpretation_variant: Variant = analysis_report.get("interpretation_v1", null)
        if not (interpretation_variant is Dictionary):
                _interpretation_panel.visible = false
                return

        var interpretation: Dictionary = interpretation_variant
        var report_variant: Variant = interpretation.get("report", null)
        if not (report_variant is Dictionary):
                _interpretation_panel.visible = false
                return

        var report: Dictionary = report_variant
        var summary_variant: Variant = report.get("summary", null)
        var headline := ""
        var subline := ""
        if summary_variant is Dictionary:
                var summary: Dictionary = summary_variant
                headline = str(summary.get("headline", "")).strip_edges()
                subline = str(summary.get("subline", "")).strip_edges()

        if _interpretation_headline_label:
                _interpretation_headline_label.text = headline
        if _interpretation_subline_label:
                _interpretation_subline_label.text = subline

        if _interpretation_cards_container:
                for child in _interpretation_cards_container.get_children():
                        child.queue_free()

        var highlights_variant: Variant = interpretation.get("highlights", [])
        var highlights: Array = highlights_variant if highlights_variant is Array else []

        var record := _build_timeline_record()
        var has_position_payload := false
        var pos_variant: Variant = record.get("position_data", {})
        if pos_variant is Dictionary and not (pos_variant as Dictionary).is_empty():
                has_position_payload = true

        var shown := 0
        for h in highlights:
                if shown >= 3:
                        break
                if not (h is Dictionary):
                        continue
                var hd: Dictionary = h
                var t0_ms := int(hd.get("t0_ms", 0))
                var kind := str(hd.get("kind", "")).strip_edges()
                var interp_variant: Variant = hd.get("interpretation", {})
                var hl_head := ""
                var hl_expl := ""
                if interp_variant is Dictionary:
                        var interp: Dictionary = interp_variant
                        hl_head = str(interp.get("headline", "")).strip_edges()
                        hl_expl = str(interp.get("explanation", "")).strip_edges()

                var card := PanelContainer.new()
                card.name = "InterpretationClip_%d" % shown
                card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
                var vbox := VBoxContainer.new()
                vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
                vbox.add_theme_constant_override("separation", 6)
                card.add_child(vbox)

                var title := Label.new()
                title.text = "[%d'] %s — %s" % [int(hd.get("minute0", 0)), _format_highlight_kind(kind), hl_head]
                title.autowrap_mode = TextServer.AUTOWRAP_WORD
                vbox.add_child(title)

                var expl := Label.new()
                expl.text = hl_expl
                expl.autowrap_mode = TextServer.AUTOWRAP_WORD
                vbox.add_child(expl)

                var actions := HBoxContainer.new()
                actions.add_theme_constant_override("separation", 8)
                vbox.add_child(actions)

                var play_btn := Button.new()
                play_btn.text = "재생"
                play_btn.disabled = (not has_position_payload) or t0_ms <= 0
                if play_btn.disabled:
                        play_btn.tooltip_text = "리플레이 포지션 데이터 없음"
                play_btn.pressed.connect(func() -> void:
                        _request_play_highlight(t0_ms)
                )
                actions.add_child(play_btn)

                if _interpretation_cards_container:
                        _interpretation_cards_container.add_child(card)
                shown += 1

        _interpretation_panel.visible = shown > 0 or headline != "" or subline != ""


func _request_play_highlight(t0_ms: int) -> void:
        if t0_ms <= 0:
                return
        var record := _build_timeline_record()
        var position_data: Dictionary = record.get("position_data", {})
        if position_data.is_empty():
                push_warning("[PostMatchStatisticsScreen] No position_data in record; cannot play highlight")
                return

        var holder := get_node_or_null("/root/MatchTimelineHolder")
        if holder:
                holder.set("pending_clip_ms", t0_ms)
                holder.set("pending_autoplay", true)
        elif MatchTimelineHolder:
                MatchTimelineHolder.set("pending_clip_ms", t0_ms)
                MatchTimelineHolder.set("pending_autoplay", true)

        _open_timeline_view()


func _format_highlight_kind(kind: String) -> String:
        match kind:
                "decision_collapse":
                        return "선택지 붕괴"
                "structure_break":
                        return "구조 붕괴"
                "pressure_overload":
                        return "압박 과부하"
                "transition_failure":
                        return "전환 실패"
                "over_reliance":
                        return "몰빵"
                _:
                        return kind


func _get_interpretation_insert_index() -> int:
        var idx := 0
        if _recommendation_panel and is_instance_valid(_recommendation_panel):
                idx += 1
        if _lite_action_bar and is_instance_valid(_lite_action_bar):
                idx += 1
        return idx


func _ensure_interpretation_panel() -> void:
        if _interpretation_panel and is_instance_valid(_interpretation_panel):
                return
        if not content_container:
                return

        var panel := Panel.new()
        panel.name = "InterpretationV1Panel"
        panel.visible = false
        panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

        var vbox := VBoxContainer.new()
        vbox.name = "VBox"
        vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        vbox.add_theme_constant_override("separation", 8)
        panel.add_child(vbox)

        var title := Label.new()
        title.text = "해설"
        vbox.add_child(title)

        var headline := Label.new()
        headline.name = "Headline"
        headline.autowrap_mode = TextServer.AUTOWRAP_WORD
        vbox.add_child(headline)

        var subline := Label.new()
        subline.name = "Subline"
        subline.autowrap_mode = TextServer.AUTOWRAP_WORD
        vbox.add_child(subline)

        var cards := VBoxContainer.new()
        cards.name = "Cards"
        cards.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        cards.add_theme_constant_override("separation", 10)
        vbox.add_child(cards)

        content_container.add_child(panel)
        content_container.move_child(panel, _get_interpretation_insert_index())

        _interpretation_panel = panel
        _interpretation_headline_label = headline
        _interpretation_subline_label = subline
        _interpretation_cards_container = cards


func _update_mvp_label(ratings: Dictionary, payload: Dictionary) -> void:
	if not mvp_label:
		return
	if ratings.is_empty():
		mvp_label.text = ""
		return
	var best_id := ""
	var best_rating := -100.0
	for player_id in ratings.keys():
		var value := float(ratings.get(player_id, 0.0))
		if value > best_rating:
			best_rating = value
			best_id = str(player_id)
	if best_id == "":
		mvp_label.text = ""
		return
	var players: Dictionary = payload.get("players", {})
	var display_name := best_id
	if players is Dictionary and players.has(best_id):
		display_name = str(players.get(best_id))
	mvp_label.text = "MVP: %s (%.1f)" % [display_name, best_rating]


func _update_my_player_heatmap() -> void:
	if not my_player_heat_canvas:
		return
	if _heat_map_cache.is_empty():
		my_player_heat_canvas.clear()
		return
	var players_dict: Dictionary = _heat_map_cache.get("players", {})
	if players_dict.is_empty():
		my_player_heat_canvas.clear()
		return

	var hero_id := _find_hero_player_id()
	if hero_id == "":
		# fallback: best heat player (first key)
		for pid in players_dict.keys():
			hero_id = str(pid)
			break
	if hero_id == "" or not players_dict.has(hero_id):
		my_player_heat_canvas.clear()
		return
	var hero_data: Dictionary = players_dict.get(hero_id, {})
	if hero_data.is_empty():
		my_player_heat_canvas.clear()
		return
	my_player_heat_canvas.set_heat_data(hero_data)


func _find_hero_player_id() -> String:
	# Try match_result.match_info.hero_player_id first
	var match_result: Dictionary = source_payload.get("match_result", {})
	var match_info_variant: Variant = match_result.get("match_info", {})
	if match_info_variant is Dictionary:
		var match_info: Dictionary = match_info_variant
		var hero_id_variant: Variant = match_info.get("hero_player_id", null)
		if typeof(hero_id_variant) in [TYPE_INT, TYPE_FLOAT, TYPE_STRING]:
			var hero_id_str := str(hero_id_variant)
			if hero_id_str != "":
				return hero_id_str

	# Fallback: stats.my_player_stats.player_id
	var raw_result: Dictionary = source_payload.get("raw_result", {})
	var stats_block: Dictionary = raw_result.get("stats", raw_result.get("raw_stats", {}))
	var my_stats_variant: Variant = stats_block.get("my_player_stats", raw_result.get("my_player_stats", null))
	if my_stats_variant is Dictionary:
		var my_stats: Dictionary = my_stats_variant
		var pid_variant: Variant = my_stats.get("player_id", null)
		if typeof(pid_variant) in [TYPE_INT, TYPE_FLOAT, TYPE_STRING]:
			var pid_str := str(pid_variant)
			if pid_str != "":
				return pid_str

	return ""


func _on_continue_pressed() -> void:
	var home_scene = "res://scenes/HomeImproved.tscn"
	if ResourceLoader.exists(home_scene):
		if ScreenTransition:
			ScreenTransition.change_scene(home_scene, "slide_right")
		else:
			get_tree().change_scene_to_file(home_scene)
	else:
		get_tree().quit()


func _on_analysis_pressed() -> void:
        """Navigate to Match Analysis Screen"""
        # MatchAnalysis expects a MatchResult JSON payload. `source_payload` may be a wrapper,
        # so prefer the extracted `match_data` SSOT here.
        var json := JSON.stringify(match_data)
        var root := get_tree().root
        root.set_meta("match_analysis_data", {"match_result_json": json})

	var analysis_scene := "res://scenes/MatchAnalysisScreen.tscn"
	if ResourceLoader.exists(analysis_scene):
		if has_node("/root/ScreenTransition"):
			get_node("/root/ScreenTransition").change_scene(analysis_scene, "slide_left")
		else:
			get_tree().change_scene_to_file(analysis_scene)
	else:
		push_error("[PostMatchStatisticsScreen] MatchAnalysisScreen.tscn not found")


func _extract_match_payload(data: Dictionary) -> Dictionary:
	if data.has("match_result") and data["match_result"] is Dictionary:
		var result: Dictionary = (data["match_result"] as Dictionary).duplicate(true)
		for key in ["events", "shot_events", "player_ratings", "rosters", "stats", "team_stats", "match_info"]:
			if not result.has(key) and data.has(key):
				result[key] = data[key]
		var raw_result = data.get("raw_result", {})
		if raw_result is Dictionary and not result.has("rosters") and raw_result.has("rosters"):
			result["rosters"] = raw_result.get("rosters")
		return result
	return data.duplicate(true)


func _get_team_name(team: String) -> String:
	var key = "%s_team" % team
	if match_data.has(key):
		var value = match_data.get(key)
		if str(value) != "":
			return str(value)

	var match_info = match_data.get("match_info", {})
	if match_info is Dictionary and match_info.has(key):
		var info_value = match_info.get(key)
		if str(info_value) != "":
			return str(info_value)

	var source_match_info = source_payload.get("match_info", {})
	if source_match_info is Dictionary and source_match_info.has(key):
		var source_value = source_match_info.get(key)
		if str(source_value) != "":
			return str(source_value)

	var rosters = _resolve_rosters_dictionary()
	if rosters.has(team):
		var roster_data = rosters.get(team)
		if roster_data is Dictionary:
			var name_value = roster_data.get("name", roster_data.get("team_name", ""))
			if str(name_value) != "":
				return str(name_value)

	return "Home" if team == "home" else "Away"


func _get_score(team: String) -> int:
	var suffix = "home" if team == "home" else "away"
	var candidate_keys = [
		"goals_%s" % suffix,
		"%s_goals" % suffix,
		"%s_score" % suffix,
		"score_%s" % suffix,
		"%s_goals" % suffix.capitalize(),
		"%s_score" % suffix.capitalize()
	]

	for key in candidate_keys:
		if match_data.has(key):
			return int(match_data.get(key, 0))

	var score_dicts = [match_data.get("score", {}), match_data.get("scores", {}), match_data.get("match_score", {})]

	for score_dict in score_dicts:
		if score_dict is Dictionary:
			if score_dict.has(suffix):
				return int(score_dict.get(suffix, 0))
			if score_dict.has(team):
				return int(score_dict.get(team, 0))

	return 0


func _collect_team_stats(team: String) -> Dictionary:
	var stats = {
		"possession": _lookup_stat("possession", team, 50),
		"shots": _lookup_stat("shots", team, 0),
		"shots_on_target": _lookup_stat("shots_on_target", team, 0),
		"xg": _lookup_stat("xg", team, 0.0),
		"passes": _lookup_stat("passes", team, 0),
		"pass_accuracy": _lookup_stat("pass_accuracy", team, 0),
		"corners": _lookup_stat("corners", team, 0),
		"fouls": _lookup_stat("fouls", team, 0)
	}
	return stats


func _lookup_stat(stat_name: String, team: String, default_value: Variant) -> Variant:
	var suffix = "home" if team == "home" else "away"
	var candidate_keys = [
		"%s_%s" % [stat_name, suffix],
		"%s_%s" % [stat_name, suffix.capitalize()],
		"%s%s" % [suffix, stat_name.capitalize()]
	]

	for key in candidate_keys:
		if match_data.has(key):
			return match_data.get(key, default_value)

	var nested_sources = [
		match_data.get("team_stats"),
		match_data.get("stats"),
		match_data.get("match_stats"),
		source_payload.get("team_stats"),
		source_payload.get("stats")
	]

	for source in nested_sources:
		if source is Dictionary:
			var entry = _get_nested_team_stats(source, suffix)
			if entry is Dictionary and entry.has(stat_name):
				return entry.get(stat_name)

	return default_value


func _get_nested_team_stats(container: Dictionary, team_key: String) -> Variant:
	if container.has(team_key):
		return container.get(team_key)
	if container.has(team_key.capitalize()):
		return container.get(team_key.capitalize())
	if container.has(team_key.to_upper()):
		return container.get(team_key.to_upper())
	return null


func _get_events() -> Array:
	var legacy_payload_key := "re" + "play"
	var legacy_doc_key := legacy_payload_key + "_doc"
	var candidates: Array = [
		match_data.get("shot_events"),
		match_data.get("events"),
		source_payload.get("events"),
		source_payload.get("shot_events"),
		source_payload.get("timeline_doc"),
		source_payload.get(legacy_payload_key, {}),
		source_payload.get(legacy_doc_key, {}),
		source_payload.get("match_result", {}),
		source_payload.get("raw_result", {})
	]

	for candidate in candidates:
		if candidate is Array:
			return candidate
		elif candidate is Dictionary:
			var nested = candidate.get("events")
			if nested is Array:
				return nested
	return []


func _extract_player_ratings() -> Dictionary:
	var candidates: Array = [
		match_data.get("player_ratings"),
		source_payload.get("player_ratings"),
		source_payload.get("match_result", {}).get("player_ratings"),
		source_payload.get("raw_result", {}).get("player_ratings")
	]

	for candidate in candidates:
		if candidate is Dictionary and not candidate.is_empty():
			return candidate.duplicate(true)
	return {}


func _build_roster_entries(team: String) -> Array:
	var suffix = "home" if team == "home" else "away"

	var candidate_sources: Array = [
		match_data.get("roster_%s" % suffix),
		match_data.get("rosters"),
		source_payload.get("roster_%s" % suffix),
		source_payload.get("rosters"),
		source_payload.get("match_result", {}).get("rosters"),
		source_payload.get("raw_result", {}).get("rosters")
	]

	for source in candidate_sources:
		var normalized = _normalize_roster_collection(source, suffix)
		if not normalized.is_empty():
			## P2.3: Add substitutes
			var starters: Array = normalized.slice(0, min(11, normalized.size()))
			var substitutes_ids: Array = _extract_substitutes_from_events(team)

			var squad: Array = starters.duplicate()
			for sub_id in substitutes_ids:
				var sub_entry: Dictionary = _find_roster_entry_by_id(sub_id, normalized)
				if not sub_entry.is_empty() and not _is_in_roster(sub_entry, squad):
					squad.append(sub_entry)

			return squad

	return _derive_roster_from_events(team)


func _normalize_roster_collection(source: Variant, team_key: String) -> Array:
	var result: Array = []
	if source is Array:
		for item in source:
			var entry = _normalize_roster_entry(item)
			if not entry.is_empty():
				result.append(entry)
	elif source is Dictionary:
		if source.has(team_key):
			return _normalize_roster_collection(source.get(team_key), team_key)
		if source.has(team_key.capitalize()):
			return _normalize_roster_collection(source.get(team_key.capitalize()), team_key)
		if source.has("players"):
			return _normalize_roster_collection(source.get("players"), team_key)
		for key in source.keys():
			var entry = _normalize_roster_entry(source[key], key)
			if not entry.is_empty():
				result.append(entry)
	return result


func _normalize_roster_entry(item: Variant, fallback_id: Variant = "") -> Dictionary:
	if item is Dictionary:
		var id_value = item.get("id", item.get("player_id", fallback_id))
		if str(id_value) == "":
			return {}
		return {
			"id": str(id_value),
			"name": str(item.get("name", item.get("player_name", id_value))),
			"position": str(item.get("position", item.get("role", "MF")))
		}
	elif item is String:
		return {"id": item, "name": item, "position": "MF"}
	return {}


func _derive_roster_from_events(team: String) -> Array:
	var result: Array = []
	var seen := {}
	var target_team_index := 0 if team == "home" else 1

	for event in _get_events():
		if not (event is Dictionary):
			continue

		var player_id := _event_player_id(event)
		if player_id == "":
			continue
		if seen.has(player_id):
			continue

		var team_index := _parse_team_identifier(event.get("team_id", event.get("team", -1)))
		if team_index != target_team_index:
			continue

		var player_name := str(
			event.get("player_name", event.get("from_player_name", _player_name_lookup.get(player_id, player_id)))
		)
		var position := str(event.get("player_position", event.get("position", "MF")))
		result.append(
			{
				"id": player_id,
				"name": player_name if player_name != "" else player_id,
				"position": position if position != "" else "MF"
			}
		)
		seen[player_id] = true

	return result


## P2.3: Extract substitutes from events
func _extract_substitutes_from_events(team: String) -> Array:
	var substitutes: Array = []
	var seen_ids: Dictionary = {}
	var target_team_id: int = 0 if team == "home" else 1

	var events: Array = _get_events()
	for event in events:
		if not (event is Dictionary):
			continue

		var event_type: String = str(event.get("type", "")).to_lower()
		if event_type != "substitution":
			continue

		var team_id: int = _parse_team_identifier(event.get("team_id", event.get("team", -1)))
		if team_id != target_team_id:
			continue

		var in_player_id: String = ""
		var details: Dictionary = event.get("details", {})
		if details.has("in_player_id"):
			in_player_id = str(details.get("in_player_id"))
		else:
			# C7: Use player_track_id to resolve player coming in
			var track_id: int = event.get("player_track_id", -1)
			if track_id != -1 and track_id <= 21:
				var player_name: String = _resolve_player_name_from_track_id(track_id)
				if player_name != "Unknown":
					in_player_id = _find_player_id_by_name(player_name, team)

		if in_player_id == "" or seen_ids.has(in_player_id):
			continue

		substitutes.append(in_player_id)
		seen_ids[in_player_id] = true

	return substitutes


## C7: Resolve player name from track_id using rosters
func _resolve_player_name_from_track_id(track_id: int) -> String:
	if track_id == -1 or track_id > 21:
		return "Unknown"

	var is_home: bool = track_id <= 10
	var local_idx: int = track_id if is_home else (track_id - 11)
	var team_key: String = "home" if is_home else "away"

	var rosters: Dictionary = _resolve_rosters_dictionary()
	if not rosters.has(team_key):
		return "Unknown"

	var team_data = rosters.get(team_key, {})
	if not team_data is Dictionary:
		return "Unknown"

	var players = team_data.get("players", [])
	if not players is Array or local_idx >= players.size():
		return "Unknown"

	var player = players[local_idx]
	if player is Dictionary:
		return player.get("name", "Unknown")

	return "Unknown"


## P2.3: Find player ID by name from roster
func _find_player_id_by_name(name: String, team: String) -> String:
	var entries: Array = _build_roster_entries(team)
	for entry in entries:
		if not (entry is Dictionary):
			continue
		var entry_name: String = str(entry.get("name", "")).to_lower()
		if entry_name == name.to_lower():
			return str(entry.get("id", ""))
	return ""


## P2.3: Check if entry already exists in roster
func _is_in_roster(entry: Dictionary, roster: Array) -> bool:
	var entry_id: String = str(entry.get("id", ""))
	for existing in roster:
		if not (existing is Dictionary):
			continue
		if str(existing.get("id", "")) == entry_id:
			return true
	return false


## P2.3: Find roster entry by player ID
func _find_roster_entry_by_id(player_id: String, roster: Array) -> Dictionary:
	for entry in roster:
		if not (entry is Dictionary):
			continue
		if str(entry.get("id", "")) == player_id:
			return entry
	return {}


func _build_player_name_map(home_entries: Array, away_entries: Array) -> Dictionary:
	var names: Dictionary = {}
	for entry in home_entries + away_entries:
		var player_id = entry.get("id", "")
		if player_id != "":
			names[player_id] = entry.get("name", player_id)

	var candidate_maps = [match_data.get("players"), source_payload.get("players"), source_payload.get("player_names")]

	for candidate in candidate_maps:
		if candidate is Dictionary:
			for key in candidate.keys():
				if not names.has(key):
					names[key] = str(candidate.get(key))

	return names


func _build_player_position_map(home_entries: Array, away_entries: Array) -> Dictionary:
	var positions: Dictionary = {}
	for entry in home_entries + away_entries:
		var player_id = entry.get("id", "")
		if player_id != "":
			positions[player_id] = entry.get("position", "MF")
	return positions


func _resolve_rosters_dictionary() -> Dictionary:
	var sources: Array = [
		match_data.get("rosters"),
		source_payload.get("rosters"),
		source_payload.get("match_result", {}).get("rosters"),
		source_payload.get("raw_result", {}).get("rosters")
	]

	for source in sources:
		if source is Dictionary and not source.is_empty():
			return source
	return {}


func _parse_team_identifier(value: Variant) -> int:
	if value is String:
		var lowered = value.to_lower()
		return 0 if lowered == "home" else 1
	return int(value)


func _build_heat_map_cache() -> Dictionary:
	var result := {"players": {}, "grid_size": Vector2i(20, 20), "field_size": PITCH_SIZE}
	var grid_size: Vector2i = result["grid_size"]
	var total_cells := grid_size.x * grid_size.y
	if total_cells <= 0:
		return result

	var player_team_map := _collect_player_team_map()
	var grids: Dictionary = {}
	var max_values: Dictionary = {}
	var heat_events := _filter_events_for_heat_map(_get_events())
	var cell_width: float = PITCH_SIZE.x / max(float(grid_size.x), 0.001)
	var cell_height: float = PITCH_SIZE.y / max(float(grid_size.y), 0.001)

	for event in heat_events:
		var player_id := _event_player_id(event)
		if player_id == "":
			continue

		var team: Variant = player_team_map.get(player_id, _team_from_event(event, player_id))
		if team != "":
			player_team_map[player_id] = team
		elif not player_team_map.has(player_id):
			player_team_map[player_id] = ""

		if not _player_name_lookup.has(player_id):
			var fallback_name := str(event.get("player_name", event.get("from_player_name", "")))
			if fallback_name != "":
				_player_name_lookup[player_id] = fallback_name

		var coord := _extract_coordinates(event)
		if coord == Vector2.ZERO and not _event_has_coordinates(event):
			continue
		var world := _convert_to_pitch_coords(coord)
		var cell_x: int = clamp(int(floor(world.x / cell_width)), 0, grid_size.x - 1)
		var cell_y: int = clamp(int(floor(world.y / cell_height)), 0, grid_size.y - 1)
		var idx: int = cell_y * grid_size.x + cell_x

		if not grids.has(player_id):
			var arr := PackedFloat32Array()
			arr.resize(total_cells)
			grids[player_id] = arr
		var grid: PackedFloat32Array = grids[player_id]
		grid[idx] += 1.0
		max_values[player_id] = max(max_values.get(player_id, 0.0), grid[idx])
		grids[player_id] = grid

	for roster_entry in _roster_meta.get("home", []):
		if roster_entry is Dictionary:
			var player_id: String = str(roster_entry.get("id", ""))
			if player_id != "":
				player_team_map[player_id] = "home"
	for roster_entry in _roster_meta.get("away", []):
		if roster_entry is Dictionary:
			var player_id: String = str(roster_entry.get("id", ""))
			if player_id != "":
				player_team_map[player_id] = "away"

	for player_id in player_team_map.keys():
		var grid: PackedFloat32Array = grids.get(player_id, PackedFloat32Array())
		if grid.is_empty() and total_cells > 0:
			grid = PackedFloat32Array()
			grid.resize(total_cells)
		var name := str(_player_name_lookup.get(player_id, player_id))
		if name == "":
			name = player_id

		result["players"][player_id] = {
			"player_id": player_id,
			"player_name": name,
			"grid": grid,
			"grid_size": grid_size,
			"max_intensity": float(max_values.get(player_id, 0.0))
		}

	return result


func _build_pass_map_cache() -> Dictionary:
	var teams := ["home", "away"]
	var result := {"home": {}, "away": {}}
	var team_nodes := {"home": {}, "away": {}}
	var team_edges := {"home": {}, "away": {}}
	var team_summary := {
		"home": {"total": 0, "success": 0, "failure": 0, "longest": 0.0},
		"away": {"total": 0, "success": 0, "failure": 0, "longest": 0.0}
	}

	var home_set := {}
	for id in _roster_home_ids:
		home_set[str(id)] = true
	for entry in _roster_meta.get("home", []):
		if entry is Dictionary:
			var pid := str(entry.get("id", ""))
			if pid != "":
				home_set[pid] = true

	var away_set := {}
	for id in _roster_away_ids:
		away_set[str(id)] = true
	for entry in _roster_meta.get("away", []):
		if entry is Dictionary:
			var pid := str(entry.get("id", ""))
			if pid != "":
				away_set[pid] = true

	var events: Array = _get_events()
	for event in events:
		if not (event is Dictionary):
			continue
		var event_type := str(event.get("type", "")).to_lower()
		if not event_type.contains("pass"):
			continue

		var from_id := str(event.get("from_player_id", event.get("player_id", "")))
		var to_id := str(event.get("to_player_id", event.get("target_player_id", "")))
		if from_id == "" and to_id == "":
			continue
		if from_id == "":
			from_id = to_id
		if to_id == "":
			to_id = from_id

		var team := ""
		if home_set.has(from_id) or home_set.has(to_id):
			team = "home"
		elif away_set.has(from_id) or away_set.has(to_id):
			team = "away"
		else:
			var pivot_player := from_id if from_id != "" else to_id
			team = _team_from_event(event, pivot_player)

		if team == "":
			continue

		var raw_start: Variant = _extract_pass_position(event, true)
		var raw_end: Variant = _extract_pass_position(event, false)
		var start_pos: Variant = raw_start if raw_start is Vector2 else null
		var end_pos: Variant = raw_end if raw_end is Vector2 else null

		var from_name := str(event.get("from_player_name", event.get("player_name", "")))
		var to_name := str(event.get("to_player_name", ""))
		_ensure_node_entry(team_nodes, team, from_id, from_name)
		_ensure_node_entry(team_nodes, team, to_id, to_name)

		if start_pos != null:
			start_pos = _convert_to_pitch_coords(start_pos)
			team_nodes[team][from_id]["sum"] += start_pos
			team_nodes[team][from_id]["count"] += 1
		if end_pos != null:
			end_pos = _convert_to_pitch_coords(end_pos)
			team_nodes[team][to_id]["sum"] += end_pos
			team_nodes[team][to_id]["count"] += 1
		team_nodes[team][from_id]["touches"] += 1
		team_nodes[team][to_id]["touches"] += 1

		var key := "%s->%s" % [from_id, to_id]
		if not team_edges[team].has(key):
			team_edges[team][key] = {
				"from": from_id,
				"to": to_id,
				"count": 0,
				"success": 0,
				"failure": 0,
				"start_sum": Vector2.ZERO,
				"end_sum": Vector2.ZERO,
				"start_count": 0,
				"end_count": 0
			}
		var edge: Variant = team_edges[team][key]
		edge["count"] += 1
		var success := true
		if start_pos != null:
			edge["start_sum"] += start_pos
			edge["start_count"] += 1
		if end_pos != null:
			edge["end_sum"] += end_pos
			edge["end_count"] += 1

		var outcome := str(event.get("outcome", event.get("result", ""))).to_lower()
		if outcome in ["fail", "failed", "unsuccessful", "incomplete", "error", "blocked"]:
			success = false
		if success:
			edge["success"] += 1
			if start_pos != null and end_pos != null:
				var dist: float = start_pos.distance_to(end_pos)
				team_summary[team]["longest"] = max(team_summary[team]["longest"], dist)
		else:
			edge["failure"] += 1
		team_edges[team][key] = edge
		team_summary[team]["total"] += 1
		if success:
			team_summary[team]["success"] += 1
		else:
			team_summary[team]["failure"] += 1

	for player_id in home_set.keys():
		_ensure_node_entry(team_nodes, "home", player_id)
	for player_id in away_set.keys():
		_ensure_node_entry(team_nodes, "away", player_id)

	for team in teams:
		var nodes_output := {}
		for player_id in team_nodes[team].keys():
			var node: Variant = team_nodes[team][player_id]
			var avg := Vector2(PITCH_SIZE.x / 2.0, PITCH_SIZE.y / 2.0)
			if node["count"] > 0:
				avg = node["sum"] / float(node["count"])
			nodes_output[player_id] = {
				"avg": avg,
				"touches": node.get("touches", 0),
				"name": node.get("name", player_id),
				"short_name": node.get("short_name", node.get("name", player_id))
			}

		var edges_output: Array = []
		for edge_key in team_edges[team].keys():
			var entry: Variant = team_edges[team][edge_key]
			var count: int = max(entry.get("count", 0), 1)
			var start_sum: Variant = entry.get("start_sum", Vector2.ZERO)
			var end_sum: Variant = entry.get("end_sum", Vector2.ZERO)
			var start_count: Variant = entry.get("start_count", 0)
			var end_count: Variant = entry.get("end_count", 0)
			var avg_start: Variant = start_sum / float(start_count) if start_count > 0 else PITCH_SIZE / 2.0
			var avg_end: Variant = end_sum / float(end_count) if end_count > 0 else PITCH_SIZE / 2.0
			edges_output.append(
				{
					"from": entry.get("from", ""),
					"to": entry.get("to", ""),
					"count": entry.get("count", 0),
					"success": entry.get("success", 0),
					"failure": entry.get("failure", 0),
					"avg_start": avg_start,
					"avg_end": avg_end
				}
			)

		var summary: Variant = team_summary[team]
		var total_passes: Variant = summary.get("total", 0)
		var success_rate := 0.0
		if total_passes > 0:
			success_rate = float(summary.get("success", 0)) / float(total_passes)
		var team_color := Color(0.25, 0.55, 0.95, 0.9) if team == "home" else Color(0.95, 0.35, 0.35, 0.9)
		result[team] = {
			"nodes": nodes_output,
			"edges": edges_output,
			"summary":
			{
				"total_passes": total_passes,
				"success_passes": summary.get("success", 0),
				"failure_passes": summary.get("failure", 0),
				"success_rate": success_rate,
				"longest_success_distance": summary.get("longest", 0.0)
			},
			"team_color": team_color
		}

	return result


func _ensure_node_entry(team_nodes: Dictionary, team: String, player_id: String, fallback_name: String = "") -> void:
	if player_id == "":
		return
	if not team_nodes[team].has(player_id):
		var name := str(_player_name_lookup.get(player_id, ""))
		if name == "":
			name = fallback_name if fallback_name != "" else player_id
		var short_name := _shorten_name(name)
		team_nodes[team][player_id] = {
			"sum": Vector2.ZERO, "count": 0, "touches": 0, "name": name, "short_name": short_name
		}
	else:
		if fallback_name != "":
			var node: Dictionary = team_nodes[team][player_id]
			if str(node.get("name", "")) == player_id:
				node["name"] = fallback_name
				node["short_name"] = _shorten_name(fallback_name)
				team_nodes[team][player_id] = node
			if not _player_name_lookup.has(player_id):
				_player_name_lookup[player_id] = fallback_name


func _collect_player_team_map() -> Dictionary:
	var map := {}
	for id in _roster_home_ids:
		map[str(id)] = "home"
	for entry in _roster_meta.get("home", []):
		if entry is Dictionary:
			var pid := str(entry.get("id", ""))
			if pid != "":
				map[pid] = "home"
	for id in _roster_away_ids:
		map[str(id)] = "away"
	for entry in _roster_meta.get("away", []):
		if entry is Dictionary:
			var pid := str(entry.get("id", ""))
			if pid != "":
				map[pid] = "away"
	return map


func _event_player_id(event: Dictionary) -> String:
	var keys := ["player_id", "ball_carrier_id", "actor_id", "from_player_id"]
	for key in keys:
		var value = event.get(key)
		if str(value) != "":
			return str(value)
	var target_value = event.get("to_player_id", event.get("target_player_id", ""))
	if str(target_value) != "":
		return str(target_value)
	return ""


func _team_from_event(event: Dictionary, player_id: String) -> String:
	var team_identifier = event.get("team_id", null)
	if team_identifier == null:
		team_identifier = event.get("team", null)

	if team_identifier is String:
		var lowered: String = team_identifier.to_lower()
		var home_name := _get_team_name("home").to_lower()
		var away_name := _get_team_name("away").to_lower()
		if lowered == "home":
			return "home"
		if lowered == "away":
			return "away"
		if lowered == home_name and home_name != "":
			return "home"
		if lowered == away_name and away_name != "":
			return "away"
		if lowered.is_valid_int():
			var parsed := int(lowered)
			if parsed == 0:
				return "home"
			if parsed == 1:
				return "away"
	elif team_identifier is Dictionary:
		var side := str(team_identifier.get("side", team_identifier.get("team", ""))).to_lower()
		if side == "home":
			return "home"
		if side == "away":
			return "away"
		var label := str(team_identifier.get("name", "")).to_lower()
		if label == _get_team_name("home").to_lower() and label != "":
			return "home"
		if label == _get_team_name("away").to_lower() and label != "":
			return "away"
	elif team_identifier is int or team_identifier is float:
		var numeric := int(team_identifier)
		if numeric == 0:
			return "home"
		if numeric == 1:
			return "away"

	var fallback := _resolve_player_team(player_id)
	if fallback != "":
		return fallback

	return ""


func _filter_events_for_heat_map(events: Array) -> Array:
	var prioritized: Array = []
	var fallback: Array = []
	for event in events:
		if not (event is Dictionary):
			continue
		if not _event_has_coordinates(event):
			continue
		var event_type := str(event.get("type", "")).to_lower()
		if event_type in ["position_update", "movement", "carry", "run", "dribble", "sprint"]:
			prioritized.append(event)
		else:
			fallback.append(event)
	if prioritized.is_empty():
		return fallback
	return prioritized


func _extract_pass_position(event: Dictionary, is_start: bool) -> Variant:
	var keys: Array = (
		["start_position", "from_position", "origin_position", "coordinates", "position"]
		if is_start
		else ["end_position", "to_position", "target_position", "actual_position"]
	)
	for key in keys:
		var value: Variant = event.get(key)
		var vec: Variant = _vector_from_variant(value)
		if vec != null:
			return vec
	return null


func _vector_from_variant(value: Variant) -> Variant:
	if value is Vector2:
		return value
	if value is Dictionary:
		if value.has("x") and value.has("y"):
			return Vector2(float(value.get("x")), float(value.get("y")))
	return null


func _convert_to_pitch_coords(point: Vector2) -> Vector2:
	var x := point.x
	var y := point.y
	if abs(x) <= 1.1 and abs(y) <= 1.1:
		x = (x + 0.5) * PITCH_SIZE.x
		y = (y + 0.5) * PITCH_SIZE.y
	elif abs(x) <= 60.0 and abs(y) <= 40.0:
		x = x + PITCH_SIZE.x / 2.0
		y = y + PITCH_SIZE.y / 2.0
	return Vector2(clamp(x, 0.0, PITCH_SIZE.x), clamp(y, 0.0, PITCH_SIZE.y))


func _resolve_player_team(player_id: String) -> String:
	if _roster_home_ids.has(player_id):
		return "home"
	if _roster_away_ids.has(player_id):
		return "away"
	return ""


func _shorten_name(name: String) -> String:
	var trimmed := name.strip_edges()
	if trimmed == "":
		return name
	var parts := trimmed.split(" ", false)
	if parts.size() >= 2:
		return "%s. %s" % [parts[0].substr(0, 1), parts[parts.size() - 1]]
	return trimmed.substr(0, min(trimmed.length(), 8))
