extends Panel
class_name PlayerRatingsPanel

@onready var player_rows: VBoxContainer = $PlayerList/PlayerRows
@onready var home_tab: Button = $TitleBar/TeamTabs/HomeTabButton
@onready var away_tab: Button = $TitleBar/TeamTabs/AwayTabButton

var match_data: Dictionary = {}
var current_team: String = "home"
var roster_entries_by_team: Dictionary = {}
var player_positions: Dictionary = {}


func _ready() -> void:
	if home_tab:
		home_tab.pressed.connect(func(): _show_team("home"))
	if away_tab:
		away_tab.pressed.connect(func(): _show_team("away"))


func set_match_data(data: Dictionary) -> void:
	match_data = data
	roster_entries_by_team = _prepare_roster_entries()
	player_positions = (
		match_data.get("player_positions", {}) if match_data.get("player_positions", {}) is Dictionary else {}
	)
	_show_team(current_team)


func _show_team(team: String) -> void:
	var start_time := Time.get_ticks_msec()
	current_team = team
	_clear_rows()

	var ratings: Dictionary = _get_player_ratings()
	var roster_entries: Array = roster_entries_by_team.get(team, [])

	if roster_entries.is_empty():
		var roster_key = "roster_home" if team == "home" else "roster_away"
		roster_entries = _normalize_roster_entries(match_data.get(roster_key, []))

	if roster_entries.is_empty() and ratings is Dictionary:
		for player_id in ratings.keys():
			roster_entries.append(
				{"id": player_id, "name": _get_player_name(player_id), "position": _get_player_position(player_id)}
			)

	var sorted_players: Array = []
	for entry in roster_entries:
		var player_id = entry.get("id", "")
		if player_id == "":
			continue
		var rating_value = float(ratings.get(player_id, 6.0))
		var merged_entry = entry.duplicate(true)
		merged_entry["rating"] = rating_value
		sorted_players.append(merged_entry)

	sorted_players.sort_custom(func(a, b): return a.get("rating", 0.0) > b.get("rating", 0.0))

	var events: Variant = match_data.get("events", [])
	if not (events is Array):
		events = []

	for entry in sorted_players:
		var player_id = entry.get("id", "")
		var stats = _calculate_player_stats(player_id, events)
		stats["position"] = entry.get("position", stats.get("position", _get_player_position(player_id)))
		_add_player_row(entry, stats)

	var elapsed := Time.get_ticks_msec() - start_time
	if OS.is_debug_build():
		print("[PlayerRatingsPanel] %s team populated in %d ms" % [team, elapsed])
	if elapsed > 100:
		push_warning("[PlayerRatingsPanel] Slow roster population (%s team): %d ms" % [team, elapsed])


func _add_player_row(player_entry: Dictionary, stats: Dictionary) -> void:
	var player_id = player_entry.get("id", "")
	var rating = float(player_entry.get("rating", 6.0))
	var display_name = player_entry.get("name", _get_player_name(player_id))
	var position = stats.get("position", _get_player_position(player_id))

	var row = HBoxContainer.new()
	row.add_theme_constant_override("separation", 20)

	var name_label = Label.new()
	name_label.text = display_name
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var position_label = Label.new()
	position_label.text = position
	position_label.custom_minimum_size = Vector2(60, 0)

	var rating_label = Label.new()
	rating_label.text = "%.1f" % rating
	rating_label.custom_minimum_size = Vector2(60, 0)
	rating_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	rating_label.add_theme_color_override("font_color", _get_rating_color(rating))

	var stats_label = Label.new()
	stats_label.text = (
		"골: %d, 슈팅: %d, 패스: %d/%d"
		% [
			stats.get("goals", 0),
			stats.get("shots", 0),
			stats.get("passes_completed", 0),
			stats.get("passes_attempted", 0)
		]
	)
	stats_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))

	row.add_child(name_label)
	row.add_child(position_label)
	row.add_child(rating_label)
	row.add_child(stats_label)
	player_rows.add_child(row)


func _calculate_player_stats(player_id: String, events: Array) -> Dictionary:
	var stats = {"goals": 0, "shots": 0, "passes_completed": 0, "passes_attempted": 0, "position": "MF"}
	for event in events:
		if event is Dictionary and event.get("player_id") == player_id:
			var event_type = str(event.get("type", "")).to_lower()
			match event_type:
				"goal":
					stats["goals"] += 1
				"shot":
					stats["shots"] += 1
				"pass":
					stats["passes_attempted"] += 1
					if event.get("outcome") == "success":
						stats["passes_completed"] += 1
	return stats


func _get_player_name(player_id: String) -> String:
	var roster = match_data.get("players", {})
	if roster is Dictionary and roster.has(player_id):
		return str(roster.get(player_id))

	var roster_meta = match_data.get("roster_meta", {})
	if roster_meta is Dictionary:
		for team_entries in roster_meta.values():
			if team_entries is Array:
				for entry in team_entries:
					if entry is Dictionary and entry.get("id", "") == player_id:
						return str(entry.get("name", "Player %s" % player_id))

	return "Player %s" % player_id


func _get_player_position(player_id: String) -> String:
	if player_positions.has(player_id):
		return str(player_positions.get(player_id))

	return "MF"


func _get_rating_color(rating: float) -> Color:
	if rating >= 8.0:
		return Color(0.0, 1.0, 0.0)
	elif rating >= 7.0:
		return Color(0.5, 1.0, 0.5)
	elif rating >= 6.0:
		return Color(1.0, 1.0, 1.0)
	return Color(1.0, 0.5, 0.5)


func _clear_rows() -> void:
	for child in player_rows.get_children():
		child.queue_free()


func _prepare_roster_entries() -> Dictionary:
	var roster_meta = match_data.get("roster_meta", {})
	if roster_meta is Dictionary:
		return {
			"home": _normalize_roster_entries(roster_meta.get("home", [])),
			"away": _normalize_roster_entries(roster_meta.get("away", []))
		}

	var result = {
		"home": _normalize_roster_entries(match_data.get("roster_home", [])),
		"away": _normalize_roster_entries(match_data.get("roster_away", []))
	}

	if result["home"].is_empty() or result["away"].is_empty():
		var rosters = match_data.get("rosters", {})
		if rosters is Dictionary:
			if result["home"].is_empty():
				result["home"] = _normalize_roster_entries(rosters.get("home", rosters.get("Home", [])))
			if result["away"].is_empty():
				result["away"] = _normalize_roster_entries(rosters.get("away", rosters.get("Away", [])))

	return result


func _normalize_roster_entries(source: Variant) -> Array:
	var result: Array = []
	if source is Array:
		for item in source:
			var entry = _normalize_single_roster_entry(item)
			if not entry.is_empty():
				result.append(entry)
	elif source is Dictionary:
		if source.has("players"):
			return _normalize_roster_entries(source.get("players"))
		for key in source.keys():
			var entry = _normalize_single_roster_entry(source[key], key)
			if not entry.is_empty():
				result.append(entry)
	return result


func _normalize_single_roster_entry(item: Variant, fallback_id: Variant = "") -> Dictionary:
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


func _get_player_ratings() -> Dictionary:
	var ratings = match_data.get("player_ratings", {})
	return ratings if ratings is Dictionary else {}
