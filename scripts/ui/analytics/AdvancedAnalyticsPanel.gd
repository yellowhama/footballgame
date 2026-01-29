extends Panel
class_name AdvancedAnalyticsPanel

@onready var _player_selector: OptionButton = $PanelContent/AdvancedTabs/HeatMapTab/PlayerSelector
@onready var _heat_map_canvas: HeatMapCanvas = $PanelContent/AdvancedTabs/HeatMapTab/HeatMapCanvas
@onready var _heat_empty_label: Label = $PanelContent/AdvancedTabs/HeatMapTab/HeatEmptyLabel

@onready var _home_button: Button = $PanelContent/AdvancedTabs/PassMapTab/TeamSelector/HomeButton
@onready var _away_button: Button = $PanelContent/AdvancedTabs/PassMapTab/TeamSelector/AwayButton
@onready var _pass_map_canvas: PassMapCanvas = $PanelContent/AdvancedTabs/PassMapTab/PassMapCanvas
@onready var _pass_empty_label: Label = $PanelContent/AdvancedTabs/PassMapTab/PassEmptyLabel

@onready var _total_value: Label = $PanelContent/AdvancedTabs/PassMapTab/PassSummaryGrid/TotalValue
@onready var _success_value: Label = $PanelContent/AdvancedTabs/PassMapTab/PassSummaryGrid/SuccessValue
@onready var _longest_value: Label = $PanelContent/AdvancedTabs/PassMapTab/PassSummaryGrid/LongestValue

@onready
var _dsa_timeline_canvas: TimelineCanvas = (
	$PanelContent/AdvancedTabs/DSA/TimelineCanvas
	if has_node("PanelContent/AdvancedTabs/DSA/TimelineCanvas")
	else null
)
@onready
var _dsa_empty_label: Label = (
	$PanelContent/AdvancedTabs/DSA/DSAEmptyLabel
	if has_node("PanelContent/AdvancedTabs/DSA/DSAEmptyLabel")
	else null
)
@onready
var _dsa_series_selector: OptionButton = (
	$PanelContent/AdvancedTabs/DSA/DSASeriesSelector
	if has_node("PanelContent/AdvancedTabs/DSA/DSASeriesSelector")
	else null
)

# Phase E: Advanced Analytics UI
@onready var _advanced_stats_container: VBoxContainer = $PanelContent/AdvancedTabs/AdvancedStatsTab/StatsContainer if has_node("PanelContent/AdvancedTabs/AdvancedStatsTab/StatsContainer") else null

var _heat_map_cache: Dictionary = {}
var _pass_map_cache: Dictionary = {}

var _heat_player_ids: Array[String] = []
var _selected_team: String = "home"

# Phase E: Advanced Analytics
var _possession_zones_home: Array = []
var _possession_zones_away: Array = []
var _team_heat_map_home: Array = []
var _team_heat_map_away: Array = []

# DSA v1.1: per-team series selection (All/Home/Away) without changing TimelineCanvas API.
var _dsa_raw_series: Dictionary = {}
var _dsa_duration_minutes: int = 90
var _dsa_mode: String = "all"


func _ready() -> void:
	if _player_selector:
		_player_selector.item_selected.connect(_on_heat_player_selected)
	if _home_button:
		_home_button.pressed.connect(_on_home_selected)
	if _away_button:
		_away_button.pressed.connect(_on_away_selected)

	_configure_dsa_selector()
	_apply_team_selection("home")


func set_heat_map_cache(cache: Dictionary) -> void:
	_heat_map_cache = cache.duplicate(true) if cache is Dictionary else {}
	_refresh_heat_map()


func set_pass_map_cache(cache: Dictionary) -> void:
	_pass_map_cache = cache.duplicate(true) if cache is Dictionary else {}
	_refresh_pass_map()


# Phase E: Advanced Analytics Setters
func set_possession_zones_home(zones: Array) -> void:
	_possession_zones_home = zones.duplicate()
	_refresh_advanced_stats()

func set_possession_zones_away(zones: Array) -> void:
	_possession_zones_away = zones.duplicate()
	_refresh_advanced_stats()

func set_team_heat_map_home(data: Array) -> void:
	_team_heat_map_home = data.duplicate()
	_refresh_advanced_stats()

func set_team_heat_map_away(data: Array) -> void:
	_team_heat_map_away = data.duplicate()
	_refresh_advanced_stats()


func set_dsa_minute_series(series: Dictionary, duration_minutes: int = 90) -> void:
	_dsa_raw_series = series.duplicate(true) if series is Dictionary else {}
	_dsa_duration_minutes = int(duration_minutes)

	if not _dsa_timeline_canvas:
		return

	if _dsa_raw_series.is_empty():
		_dsa_timeline_canvas.clear_data()
		if _dsa_empty_label:
			_dsa_empty_label.visible = true
		if _dsa_series_selector:
			_dsa_series_selector.visible = false
		return

	if _dsa_series_selector:
		_dsa_series_selector.visible = true

	_apply_dsa_mode()
	if _dsa_empty_label:
		_dsa_empty_label.visible = false


func _configure_dsa_selector() -> void:
	if not _dsa_series_selector:
		return

	if _dsa_series_selector.item_count > 0:
		return

	_dsa_series_selector.clear()
	_dsa_series_selector.add_item("All", 0)
	_dsa_series_selector.set_item_metadata(0, "all")
	_dsa_series_selector.add_item("Home", 1)
	_dsa_series_selector.set_item_metadata(1, "home")
	_dsa_series_selector.add_item("Away", 2)
	_dsa_series_selector.set_item_metadata(2, "away")

	_dsa_series_selector.select(0)
	_dsa_mode = "all"
	_dsa_series_selector.item_selected.connect(_on_dsa_mode_selected)
	_dsa_series_selector.visible = false


func _on_dsa_mode_selected(index: int) -> void:
	if not _dsa_series_selector:
		return
	_dsa_mode = str(_dsa_series_selector.get_item_metadata(index))
	_apply_dsa_mode()


func _apply_dsa_mode() -> void:
	if not _dsa_timeline_canvas or _dsa_raw_series.is_empty():
		return
	var mapped := _map_dsa_series_for_mode(_dsa_raw_series, _dsa_mode)
	_dsa_timeline_canvas.set_data([], [], _dsa_duration_minutes, mapped)


static func _map_dsa_series_for_mode(series: Dictionary, mode: String) -> Dictionary:
	if series.is_empty():
		return {}

	match mode:
		"home":
			return {
				"pressure": series.get("pressure_against_home", series.get("pressure", [])),
				"tempo": series.get("tempo_home", series.get("tempo", [])),
				"transitions": series.get("transitions_home", series.get("transitions", [])),
			}
		"away":
			return {
				"pressure": series.get("pressure_against_away", series.get("pressure", [])),
				"tempo": series.get("tempo_away", series.get("tempo", [])),
				"transitions": series.get("transitions_away", series.get("transitions", [])),
			}
		_:
			return {
				"pressure": series.get("pressure", []),
				"tempo": series.get("tempo", []),
				"transitions": series.get("transitions", []),
			}


func _refresh_heat_map() -> void:
	if not _player_selector or not _heat_map_canvas or not _heat_empty_label:
		return

	_player_selector.clear()
	_heat_player_ids.clear()

	var players: Dictionary = _heat_map_cache.get("players", {})
	if players.is_empty():
		_heat_map_canvas.clear()
		_heat_empty_label.visible = true
		return

	# Populate selector.
	for player_id in players.keys():
		var pid: String = str(player_id)
		var pdata: Dictionary = players.get(player_id, {})
		var label: String = str(pdata.get("player_name", pid))
		_heat_player_ids.append(pid)
		_player_selector.add_item(label)

	# Select first player by default.
	if _heat_player_ids.is_empty():
		_heat_map_canvas.clear()
		_heat_empty_label.visible = true
		return

	_player_selector.select(0)
	_show_heat_for_index(0)


func _on_heat_player_selected(index: int) -> void:
	_show_heat_for_index(index)


func _show_heat_for_index(index: int) -> void:
	if not _heat_map_canvas or not _heat_empty_label:
		return

	var players: Dictionary = _heat_map_cache.get("players", {})
	if players.is_empty():
		_heat_map_canvas.clear()
		_heat_empty_label.visible = true
		return

	if index < 0 or index >= _heat_player_ids.size():
		_heat_map_canvas.clear()
		_heat_empty_label.visible = true
		return

	var player_id: String = _heat_player_ids[index]
	var data: Dictionary = players.get(player_id, {})
	if data.is_empty():
		_heat_map_canvas.clear()
		_heat_empty_label.visible = true
		return

	_heat_map_canvas.set_heat_data(data)
	_heat_empty_label.visible = false


func _refresh_pass_map() -> void:
	_apply_team_selection(_selected_team)


func _on_home_selected() -> void:
	_apply_team_selection("home")


func _on_away_selected() -> void:
	_apply_team_selection("away")


func _apply_team_selection(team: String) -> void:
	_selected_team = team

	if _home_button:
		_home_button.button_pressed = (team == "home")
	if _away_button:
		_away_button.button_pressed = (team == "away")

	_render_pass_map(team)


func _render_pass_map(team: String) -> void:
	if not _pass_map_canvas or not _pass_empty_label:
		return

	var team_data: Dictionary = _pass_map_cache.get(team, {})
	if team_data.is_empty():
		_pass_map_canvas.clear()
		_pass_empty_label.visible = true
		_set_pass_summary({})
		return

	_pass_map_canvas.set_pass_data(team_data)
	_pass_empty_label.visible = false
	_set_pass_summary(team_data.get("summary", {}))


func _set_pass_summary(summary: Dictionary) -> void:
	if not _total_value or not _success_value or not _longest_value:
		return

	if summary.is_empty():
		_total_value.text = "-"
		_success_value.text = "-"
		_longest_value.text = "-"
		return

	var total: int = int(summary.get("total_passes", 0))
	var success_rate: float = float(summary.get("success_rate", 0.0))
	var longest: float = float(summary.get("longest_success_distance", 0.0))

	_total_value.text = str(total)
	_success_value.text = "%.1f%%" % (success_rate * 100.0)
	_longest_value.text = "%.1fm" % longest


# Phase E: Advanced Analytics Display
func _refresh_advanced_stats() -> void:
	if not _advanced_stats_container:
		return

	# Clear previous content
	for child in _advanced_stats_container.get_children():
		child.queue_free()

	# Display possession zones
	if not _possession_zones_home.is_empty():
		var home_label = Label.new()
		home_label.text = "Home Team Possession Zones:"
		_advanced_stats_container.add_child(home_label)

		for i in range(min(_possession_zones_home.size(), 18)):
			var zone_label = Label.new()
			zone_label.text = "  Zone %d: %.1f%%" % [i, _possession_zones_home[i]]
			_advanced_stats_container.add_child(zone_label)

	if not _possession_zones_away.is_empty():
		var away_label = Label.new()
		away_label.text = "Away Team Possession Zones:"
		_advanced_stats_container.add_child(away_label)

		for i in range(min(_possession_zones_away.size(), 18)):
			var zone_label = Label.new()
			zone_label.text = "  Zone %d: %.1f%%" % [i, _possession_zones_away[i]]
			_advanced_stats_container.add_child(zone_label)

	# Display team heat map summary
	if not _team_heat_map_home.is_empty():
		var home_heat_label = Label.new()
		home_heat_label.text = "Home Team Heat Map: %d positions tracked" % _team_heat_map_home.size()
		_advanced_stats_container.add_child(home_heat_label)

	if not _team_heat_map_away.is_empty():
		var away_heat_label = Label.new()
		away_heat_label.text = "Away Team Heat Map: %d positions tracked" % _team_heat_map_away.size()
		_advanced_stats_container.add_child(away_heat_label)
