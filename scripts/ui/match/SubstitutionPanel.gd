extends Control
class_name SubstitutionPanel

signal substitution_selected(out_player_id: String, in_player_id: String)
signal cancelled

const MAX_SUBSTITUTIONS := 3

@onready var _backdrop: ColorRect = $Backdrop
@onready var _panel: Panel = $Panel
@onready var _remaining_label: Label = $Panel/VBox/Header/RemainingLabel
@onready var _starters_list: ItemList = $Panel/VBox/ListContainer/StarterColumn/StartersList
@onready var _bench_list: ItemList = $Panel/VBox/ListContainer/BenchColumn/BenchList
@onready var _confirm_button: Button = $Panel/VBox/ButtonRow/ConfirmButton
@onready var _cancel_button: Button = $Panel/VBox/ButtonRow/CancelButton

var _starters: Array = []
var _bench: Array = []


func _ready() -> void:
	visible = false
	mouse_filter = Control.MOUSE_FILTER_STOP
	if _backdrop:
		_backdrop.mouse_filter = Control.MOUSE_FILTER_STOP

	_starters_list.item_selected.connect(_on_starter_selected)
	_bench_list.item_selected.connect(_on_bench_selected)
	_confirm_button.pressed.connect(_on_confirm_pressed)
	_cancel_button.pressed.connect(_on_cancel_pressed)

	_update_confirmation_state()


func show_with_roster(starters: Array, bench: Array, remaining: int) -> void:
	_starters = starters.duplicate()
	_bench = bench.duplicate()

	_populate_list(_starters_list, _starters)
	_populate_list(_bench_list, _bench)

	_update_remaining(remaining)
	_update_confirmation_state()

	visible = true
	_panel.grab_focus()


func hide_panel() -> void:
	visible = false
	_starters_list.deselect_all()
	_bench_list.deselect_all()
	_update_confirmation_state()


func set_remaining_substitutions(count: int) -> void:
	_update_remaining(count)


func _populate_list(list: ItemList, entries: Array) -> void:
	list.clear()
	for entry in entries:
		if not (entry is Dictionary):
			continue
		var text := _format_player(entry)
		list.add_item(text)
		var index := list.get_item_count() - 1
		list.set_item_metadata(index, str(entry.get("id", "")))


func _format_player(entry: Dictionary) -> String:
	var number := str(entry.get("number", entry.get("shirt_number", "")))
	var position := str(entry.get("position", ""))
	var name := str(entry.get("name", entry.get("player_name", "선수")))
	var player_id := str(entry.get("id", ""))

	var display := name
	if position != "":
		display = "%s - %s" % [position, display]
	if number != "":
		display = "#%s %s" % [number, display]

	# Phase 4.2: Add position warning if applicable
	if player_id != "" and position != "":
		var warning = _get_position_warning_icon(player_id, position)
		if warning != "":
			display = "%s %s" % [warning, display]

	return display


func _update_remaining(remaining: int) -> void:
	_remaining_label.text = "교체 횟수: %d/%d 남음" % [remaining, MAX_SUBSTITUTIONS]


func _on_starter_selected(_index: int) -> void:
	_update_confirmation_state()


func _on_bench_selected(_index: int) -> void:
	_update_confirmation_state()


func _update_confirmation_state() -> void:
	var out_id := _get_selected_player(_starters_list)
	var in_id := _get_selected_player(_bench_list)
	var valid := out_id != "" and in_id != "" and out_id != in_id
	_confirm_button.disabled = not valid


func _get_selected_player(list: ItemList) -> String:
	var selected := list.get_selected_items()
	if selected.is_empty():
		return ""
	var index := selected[0]
	return str(list.get_item_metadata(index))


func _on_confirm_pressed() -> void:
	var out_id := _get_selected_player(_starters_list)
	var in_id := _get_selected_player(_bench_list)
	if out_id == "" or in_id == "" or out_id == in_id:
		return
	substitution_selected.emit(out_id, in_id)
	hide_panel()


func _on_cancel_pressed() -> void:
	cancelled.emit()
	hide_panel()


func _unhandled_input(event: InputEvent) -> void:
	if not visible:
		return
	if event.is_action_pressed("ui_cancel"):
		_on_cancel_pressed()


## Phase 4.2: Position Suitability Checking
## Get warning icon for player's position suitability
## @param player_id: Player ID (will try to resolve to cache UID)
## @param assigned_position: Position player is assigned to (e.g., "ST", "MC")
## @return Warning icon string ("", "✓", "○", "△", "✗", "⛔")
func _get_position_warning_icon(player_id: String, assigned_position: String) -> String:
	# Try to resolve player ID to cache UID
	var player_uid = _resolve_player_uid(player_id)
	if player_uid < 0:
		return ""  # Cannot resolve, no warning

	# Check position suitability
	var suitability = _check_position_suitability(player_uid, assigned_position)
	if suitability.is_empty():
		return ""

	# Return icon based on warning level
	match suitability.warning_level:
		"none":
			return "✓"
		"minor":
			return "○"
		"moderate":
			return "△"
		"major":
			return "✗"
		"critical":
			return "⛔"
		_:
			return ""


## Check player's position suitability
## @param player_uid: Player cache UID
## @param assigned_position: Position code (e.g., "ST", "MC")
## @return Dictionary {suitable, warning_level, penalty_percent, rating}
func _check_position_suitability(player_uid: int, assigned_position: String) -> Dictionary:
	var ratings = GameCache.get_player_position_ratings(player_uid)
	if ratings.is_empty():
		return {}

	# Map engine position to FM 2023 position if needed
	var fm_position = _map_position_to_fm(assigned_position)
	var rating = int(ratings.get(fm_position, 0))

	return {
		"suitable": rating >= 11,
		"warning_level": PositionRatingUtils.get_warning_level(rating),
		"penalty_percent": PositionRatingUtils.calculate_penalty_percent(rating),
		"rating": rating
	}


## Resolve player ID to cache UID using multi-stage resolution
## Handles three types of player IDs in the match system:
## 1. GameCache players: Numeric UID 1-8053 (real FM 2023 data)
## 2. MyTeamData players: "player_{timestamp}_{random}" (hash-based UID)
## 3. Opponent players: "{team_name}_{number}" (hash-based UID)
##
## @param player_id: Player ID string from match data
## @return Cache UID or -1 if resolution failed
func _resolve_player_uid(player_id: String) -> int:
	# Edge case: empty or whitespace-only
	if player_id.strip_edges().is_empty():
		return -1

	var cleaned_id := player_id.strip_edges()

	# Stage 1: Direct Numeric UID (GameCache Players)
	# Check if player_id is numeric and in valid range (1-8053)
	if cleaned_id.is_valid_int():
		var candidate_uid := int(cleaned_id)

		# Validate range (FM 2023 database has UIDs 1-8053)
		if candidate_uid >= 1 and candidate_uid <= 8053:
			# Verify existence in GameCache
			if GameCache.is_player_cache_ready():
				var player_data := GameCache.get_player(candidate_uid)
				if not player_data.is_empty():
					return candidate_uid

		# Numeric but out of range or not found
		return -1

	# Stage 2: Hash-Based UID (MyTeamData/Opponent Players)
	# Detect pattern-based IDs and generate hash-based UIDs

	# MyTeamData pattern: "player_..." or "saved_player_..."
	if cleaned_id.begins_with("player_") or cleaned_id.begins_with("saved_player_"):
		# Use same hash formula as MatchSimulationManager.gd:2393
		var uid := int(hash(cleaned_id) & 0x7fffffff) % 1000000000
		if uid == 0:
			uid = 100000
		return uid

	# Opponent pattern: "{team_name}_{number}" (contains underscore + numeric suffix)
	if cleaned_id.contains("_"):
		var parts := cleaned_id.split("_")
		if parts.size() >= 2 and parts[-1].is_valid_int():
			# Use same hash formula as MatchSimulationManager.gd:3230, 3319
			var uid: int = int(abs(hash(cleaned_id))) % 1000000000
			return uid

	# Stage 3: Name Search Fallback (Expensive - Last Resort)
	# Try searching by player name in GameCache
	if GameCache.is_player_cache_ready():
		var players := GameCache.search_players(cleaned_id, false, 1)
		if not players.is_empty():
			return int(players[0].get("uid", -1))

	# Stage 4: Resolution Failed
	return -1


## Map engine position codes to FM 2023 position codes
## @param engine_position: Position from match data (could be various formats)
## @return FM 2023 position code
func _map_position_to_fm(engine_position: String) -> String:
	var pos = engine_position.to_upper()

	# Direct matches
	if pos in ["GK", "DL", "DC", "DR", "WBL", "WBR", "DM", "ML", "MC", "MR", "AML", "AMC", "AMR", "ST"]:
		return pos

	# Common mappings by line
	match pos:
		# Defenders (4선)
		"LB":
			return "DL"
		"CB":
			return "DC"
		"RB":
			return "DR"
		"LWB":
			return "WBL"
		"RWB":
			return "WBR"

		# Defensive Midfielders
		"CDM":
			return "DM"

		# 3선 (Midfield line)
		"LM":
			return "ML"  # Left Midfielder (3선 좌측)
		"CM":
			return "MC"  # Center Midfielder (3선 중앙)
		"RM":
			return "MR"  # Right Midfielder (3선 우측)

		# 2선 (Attacking Midfield line)
		"LAM":
			return "AML"  # Left Attacking Midfielder (2선 좌측)
		"CAM":
			return "AMC"  # Center Attacking Midfielder (2선 중앙)
		"RAM":
			return "AMR"  # Right Attacking Midfielder (2선 우측)

		# 1선 (Forward line) - 측면
		"LW":
			return "AML"  # Left Winger (1선 좌측 윙어, FM에서 가장 공격적인 측면)
		"RW":
			return "AMR"  # Right Winger (1선 우측 윙어)

		# 1선 (Forward line) - 스트라이커
		"LF":
			return "ST"  # Left Forward (투톱 좌측)
		"CF":
			return "ST"  # Center Forward (중앙 스트라이커)
		"RF":
			return "ST"  # Right Forward (투톱 우측)
		"ST":
			return "ST"  # Striker
		"FW":
			return "ST"  # Forward

		_:
			return "MC"  # Default to center mid
