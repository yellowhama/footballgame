extends Control
class_name HallOfFameScreen
##
## HallOfFameScreen - 명예의 전당
##
## BM Two-Track: 졸업생 목록 및 팀 스냅샷 관리
## - 저장된 선수(졸업생) 목록 표시
## - Dual Ending 시스템 연동 (커리어/관계 엔딩)
## - 포지션별 필터링
## - 스테이지 모드에서 팀 선택 시 사용
##

signal back_pressed
signal player_selected(player_data: Dictionary)
signal team_selected(team_snapshot: Dictionary)

# Constants for snapshot storage
const SNAPSHOT_DIR := "user://hall_of_fame/"
const SNAPSHOT_FILE := "team_snapshots.json"
const MAX_SNAPSHOTS := 10

# Loaded snapshots
var team_snapshots: Array = []
var current_view_mode: String = "players"  # players, snapshots

# UI References
@onready var back_button: Button = $VBox/Header/BackButton
@onready var title_label: Label = $VBox/Header/TitleLabel
@onready var filter_container: HBoxContainer = $VBox/FilterBar
@onready var player_list: VBoxContainer = $VBox/Content/PlayerListScroll/PlayerList
@onready var player_count_label: Label = $VBox/Header/CountLabel
@onready var team_snapshot_button: Button = $VBox/Footer/TeamSnapshotButton

# Filter state
var current_filter: String = "all"  # all, GK, DF, MF, FW, or FM 2023 positions
var current_sort: String = "overall"  # overall, name, ending
var search_mode: String = "saved"  # saved (graduates) or cache (FM 2023 data)
var min_position_rating: int = 15  # Minimum rating for cache search (0-20)

# Ending display names
const CAREER_ENDINGS := {
	"PRO_SUPERSTAR": "프로 슈퍼스타",
	"OVERSEAS_STUDY": "해외 유학",
	"COLLEGE_ACE": "대학 에이스",
	"COACH_PATH": "지도자의 길",
	"HIDDEN_LEGEND": "숨겨진 전설",
}

const RELATIONSHIP_ENDINGS := {
	"COACH_DISCIPLE": "코치의 제자",
	"ETERNAL_RIVAL": "영원한 라이벌",
	"BEST_TEAMMATE": "최고의 동료",
	"SOLO_PLAYER": "고독한 선수",
}


func _ready() -> void:
	print("[HallOfFameScreen] Initializing...")

	_ensure_snapshot_dir()
	_load_snapshots()
	_setup_ui()
	_connect_signals()
	_load_players()

	print("[HallOfFameScreen] Ready! Snapshots: %d" % team_snapshots.size())


func _setup_ui() -> void:
	"""Setup UI components"""
	if title_label:
		title_label.text = "명예의 전당"

	# Setup filter buttons
	if filter_container:
		_create_filter_buttons()

	# Add view toggle buttons (Players vs Snapshots)
	if filter_container:
		var spacer := Control.new()
		spacer.custom_minimum_size = Vector2(40, 0)
		filter_container.add_child(spacer)

		var players_btn := Button.new()
		players_btn.text = "👤 선수"
		players_btn.toggle_mode = true
		players_btn.button_pressed = true
		players_btn.custom_minimum_size = Vector2(80, 40)
		players_btn.pressed.connect(_on_view_mode_changed.bind("players"))
		filter_container.add_child(players_btn)

		var snapshots_btn := Button.new()
		snapshots_btn.text = "📁 스냅샷"
		snapshots_btn.toggle_mode = true
		snapshots_btn.custom_minimum_size = Vector2(80, 40)
		snapshots_btn.pressed.connect(_on_view_mode_changed.bind("snapshots"))
		filter_container.add_child(snapshots_btn)

	# Setup team snapshot button
	if team_snapshot_button:
		team_snapshot_button.text = "현재 팀 스냅샷 저장"


func _connect_signals() -> void:
	"""Connect UI signals"""
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	if team_snapshot_button:
		team_snapshot_button.pressed.connect(_on_save_team_snapshot)


func _create_filter_buttons() -> void:
	"""Create position filter buttons"""
	# Add search mode toggle (Saved vs Cache)
	var mode_label := Label.new()
	mode_label.text = "검색 모드:"
	mode_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	filter_container.add_child(mode_label)

	var saved_mode_btn := Button.new()
	saved_mode_btn.text = "졸업생"
	saved_mode_btn.toggle_mode = true
	saved_mode_btn.button_pressed = (search_mode == "saved")
	saved_mode_btn.custom_minimum_size = Vector2(70, 40)
	saved_mode_btn.pressed.connect(_on_search_mode_changed.bind("saved"))
	filter_container.add_child(saved_mode_btn)

	var cache_mode_btn := Button.new()
	cache_mode_btn.text = "FM 데이터"
	cache_mode_btn.toggle_mode = true
	cache_mode_btn.button_pressed = (search_mode == "cache")
	cache_mode_btn.custom_minimum_size = Vector2(80, 40)
	cache_mode_btn.pressed.connect(_on_search_mode_changed.bind("cache"))
	filter_container.add_child(cache_mode_btn)

	var spacer1 := Control.new()
	spacer1.custom_minimum_size = Vector2(20, 0)
	filter_container.add_child(spacer1)

	# Position filters (dynamic based on search mode)
	if search_mode == "saved":
		# Original 4-category filters for saved players
		var filters := [
			{"id": "all", "label": "전체"},
			{"id": "GK", "label": "GK"},
			{"id": "DF", "label": "DF"},
			{"id": "MF", "label": "MF"},
			{"id": "FW", "label": "FW"},
		]

		for filter_data in filters:
			var btn := Button.new()
			btn.text = filter_data.label
			btn.toggle_mode = true
			btn.button_pressed = (filter_data.id == current_filter)
			btn.custom_minimum_size = Vector2(60, 40)
			btn.pressed.connect(_on_filter_pressed.bind(filter_data.id))
			filter_container.add_child(btn)
	else:
		# FM 2023 14-position filters for cache search
		var spacer_label := Label.new()
		spacer_label.text = "포지션:"
		spacer_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		filter_container.add_child(spacer_label)

		var fm_positions := ["GK", "DL", "DC", "DR", "WBL", "WBR", "DM", "ML", "MC", "MR", "AML", "AMC", "AMR", "ST"]

		for pos in fm_positions:
			var btn := Button.new()
			btn.text = pos
			btn.toggle_mode = true
			btn.button_pressed = (pos == current_filter)
			btn.custom_minimum_size = Vector2(50, 35)
			btn.pressed.connect(_on_position_filter_pressed.bind(pos))
			filter_container.add_child(btn)


func _load_players() -> void:
	"""Load players based on current search mode"""
	if search_mode == "cache":
		_refresh_player_list()  # Use cache search
	else:
		_load_saved_players()  # Use saved players


func _load_saved_players() -> void:
	"""Load saved players from MyTeamData"""
	if not player_list:
		return

	# Clear existing items
	for child in player_list.get_children():
		child.queue_free()

	# Get saved players
	var players: Array = []
	if MyTeamData:
		players = MyTeamData.saved_players

	# Apply filter
	var filtered := _filter_players(players)

	# Sort players
	filtered = _sort_players(filtered)

	# Update count
	if player_count_label:
		player_count_label.text = "졸업생: %d / %d" % [filtered.size(), MyTeamData.MAX_PLAYERS if MyTeamData else 50]

	# Create player cards
	if filtered.is_empty():
		var empty_label := Label.new()
		empty_label.text = "아직 졸업생이 없습니다.\n\n커리어 모드를 완료하여\n선수를 명예의 전당에 등록하세요!"
		empty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		empty_label.add_theme_font_size_override("font_size", 18)
		empty_label.modulate = Color(0.7, 0.7, 0.7)
		player_list.add_child(empty_label)
	else:
		for player in filtered:
			var card := _create_player_card(player)
			player_list.add_child(card)

	print("[HallOfFameScreen] Loaded %d players (filtered: %d)" % [players.size(), filtered.size()])


## Phase 4.3: Cache search for FM 2023 players
func _refresh_player_list() -> void:
	"""Query GameCache for players by position rating"""
	if not player_list:
		return

	# Clear existing items
	for child in player_list.get_children():
		child.queue_free()

	# Query GameCache for best players at this position
	var start_time = Time.get_ticks_msec()
	var players = GameCache.find_best_players_for_position(current_filter, min_position_rating, 50)
	var elapsed = Time.get_ticks_msec() - start_time

	print(
		(
			"[HallOfFameScreen] Cache search for %s (rating >= %d): %d players in %d ms"
			% [current_filter, min_position_rating, players.size(), elapsed]
		)
	)

	# Update count
	if player_count_label:
		player_count_label.text = (
			"검색 결과: %d명 (포지션: %s, 최소 레이팅: %d)" % [players.size(), current_filter, min_position_rating]
		)

	# Create player cards from cache data
	if players.is_empty():
		var empty_label := Label.new()
		empty_label.text = "포지션 %s에 적합한 선수가 없습니다.\n(최소 레이팅 %d 이상)" % [current_filter, min_position_rating]
		empty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		empty_label.add_theme_font_size_override("font_size", 18)
		empty_label.modulate = Color(0.7, 0.7, 0.7)
		player_list.add_child(empty_label)
	else:
		for player_data in players:
			var card := _create_cache_player_card(player_data)
			player_list.add_child(card)


## Create player card from cache data
func _create_cache_player_card(player_data: Dictionary) -> Control:
	"""Create a player card from GameCache data"""
	var card := PanelContainer.new()
	card.custom_minimum_size = Vector2(0, 100)
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var hbox := HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 16)
	card.add_child(hbox)

	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 12)
	margin.add_theme_constant_override("margin_right", 12)
	margin.add_theme_constant_override("margin_top", 8)
	margin.add_theme_constant_override("margin_bottom", 8)
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_child(margin)

	var content := HBoxContainer.new()
	content.add_theme_constant_override("separation", 16)
	margin.add_child(content)

	# Left: Position rating
	var rating_label := Label.new()
	rating_label.text = str(player_data.get("position_rating", 0))
	rating_label.add_theme_font_size_override("font_size", 32)
	rating_label.add_theme_color_override(
		"font_color", PositionRatingUtils.get_position_rating_color(int(player_data.get("position_rating", 0)))
	)
	rating_label.custom_minimum_size = Vector2(60, 0)
	rating_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content.add_child(rating_label)

	# Middle: Player info
	var info_vbox := VBoxContainer.new()
	info_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_child(info_vbox)

	var name_label := Label.new()
	name_label.text = str(player_data.get("name", "Unknown"))
	name_label.add_theme_font_size_override("font_size", 18)
	info_vbox.add_child(name_label)

	var stats_label := Label.new()
	stats_label.text = (
		"CA: %d  PA: %d  Age: %d  Pos: %s"
		% [
			player_data.get("ca", 0),
			player_data.get("pa", 0),
			player_data.get("age", 0),
			player_data.get("position", "")
		]
	)
	stats_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	info_vbox.add_child(stats_label)

	return card


func _filter_players(players: Array) -> Array:
	"""Filter players by position"""
	if current_filter == "all":
		return players.duplicate()

	var result := []
	for player in players:
		var pos: String = player.get("position", "")
		var pos_category := _get_position_category(pos)
		if pos_category == current_filter:
			result.append(player)

	return result


func _get_position_category(position: String) -> String:
	"""Get position category (GK, DF, MF, FW)"""
	match position:
		"GK":
			return "GK"
		"CB", "LB", "RB", "LWB", "RWB", "SW":
			return "DF"
		"CDM", "CM", "CAM", "LM", "RM":
			return "MF"
		"ST", "CF", "LW", "RW", "SS":
			return "FW"
		_:
			return "MF"  # Default


func _sort_players(players: Array) -> Array:
	"""Sort players by current sort criteria"""
	var sorted := players.duplicate()

	match current_sort:
		"overall":
			sorted.sort_custom(func(a, b): return a.get("overall", 0) > b.get("overall", 0))
		"name":
			sorted.sort_custom(func(a, b): return a.get("name", "") < b.get("name", ""))
		"ending":
			sorted.sort_custom(
				func(a, b):
					var ea = a.get("career_stats", {}).get("career_ending", "")
					var eb = b.get("career_stats", {}).get("career_ending", "")
					return ea < eb
			)

	return sorted


func _create_player_card(player: Dictionary) -> Control:
	"""Create a player card for Hall of Fame display"""
	var card := PanelContainer.new()
	card.custom_minimum_size = Vector2(0, 120)
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var hbox := HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 16)
	card.add_child(hbox)

	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 12)
	margin.add_theme_constant_override("margin_right", 12)
	margin.add_theme_constant_override("margin_top", 8)
	margin.add_theme_constant_override("margin_bottom", 8)
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.add_child(margin)

	var content := HBoxContainer.new()
	content.add_theme_constant_override("separation", 16)
	margin.add_child(content)

	# Left: Overall rating circle
	var overall_container := CenterContainer.new()
	overall_container.custom_minimum_size = Vector2(80, 80)
	content.add_child(overall_container)

	var overall_label := Label.new()
	overall_label.text = str(player.get("overall", 0))
	overall_label.add_theme_font_size_override("font_size", 32)
	overall_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	var overall: int = player.get("overall", 0)
	if overall >= 85:
		overall_label.add_theme_color_override("font_color", Color(1.0, 0.84, 0.0))  # Gold
	elif overall >= 75:
		overall_label.add_theme_color_override("font_color", Color(0.6, 0.4, 0.8))  # Purple
	elif overall >= 65:
		overall_label.add_theme_color_override("font_color", Color(0.2, 0.6, 1.0))  # Blue
	overall_container.add_child(overall_label)

	# Center: Player info
	var info_vbox := VBoxContainer.new()
	info_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	info_vbox.add_theme_constant_override("separation", 4)
	content.add_child(info_vbox)

	# Name
	var name_label := Label.new()
	name_label.text = player.get("name", "Unknown")
	name_label.add_theme_font_size_override("font_size", 20)
	info_vbox.add_child(name_label)

	# Position & Age
	var pos_label := Label.new()
	pos_label.text = "%s | %d세" % [player.get("position", ""), player.get("age", 18)]
	pos_label.add_theme_font_size_override("font_size", 14)
	pos_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	info_vbox.add_child(pos_label)

	# Dual Ending
	var career_stats: Dictionary = player.get("career_stats", {})
	var career_ending: String = career_stats.get("career_ending", "")
	var relationship_ending: String = career_stats.get("relationship_ending", "")

	var ending_label := Label.new()
	var career_text: String = CAREER_ENDINGS.get(career_ending, career_ending)
	var rel_text: String = RELATIONSHIP_ENDINGS.get(relationship_ending, relationship_ending)
	if career_text != "" or rel_text != "":
		ending_label.text = "%s | %s" % [career_text, rel_text]
	else:
		ending_label.text = "엔딩 미정"
	ending_label.add_theme_font_size_override("font_size", 12)
	ending_label.add_theme_color_override("font_color", Color(0.5, 0.8, 0.5))
	info_vbox.add_child(ending_label)

	# Career stats
	var stats_label := Label.new()
	var matches: int = career_stats.get("total_matches", 0)
	var goals: int = career_stats.get("total_goals", 0)
	var assists: int = career_stats.get("total_assists", 0)
	stats_label.text = "%d경기 | %d골 | %d도움" % [matches, goals, assists]
	stats_label.add_theme_font_size_override("font_size", 12)
	stats_label.add_theme_color_override("font_color", Color(0.6, 0.6, 0.6))
	info_vbox.add_child(stats_label)

	# Right: Select button
	var select_btn := Button.new()
	select_btn.text = "선택"
	select_btn.custom_minimum_size = Vector2(80, 40)
	select_btn.pressed.connect(_on_player_card_selected.bind(player))
	content.add_child(select_btn)

	return card


func _on_filter_pressed(filter_id: String) -> void:
	"""Handle filter button press (saved players mode)"""
	current_filter = filter_id

	# Update button states
	for child in filter_container.get_children():
		if child is Button:
			child.button_pressed = (child.text == _get_filter_label(filter_id))

	_load_players()


## Phase 4.3: Search mode toggle handler
func _on_search_mode_changed(mode: String) -> void:
	"""Handle search mode toggle (saved vs cache)"""
	search_mode = mode
	current_filter = "all" if mode == "saved" else "MC"  # Default to MC for cache

	# Rebuild filter buttons
	_rebuild_filter_bar()

	# Reload player list
	_load_players()


## Phase 4.3: FM 2023 position filter handler
func _on_position_filter_pressed(position: String) -> void:
	"""Handle FM 2023 position filter button press"""
	current_filter = position

	# Update button states
	for child in filter_container.get_children():
		if (
			child is Button
			and child.text in ["GK", "DL", "DC", "DR", "WBL", "WBR", "DM", "ML", "MC", "MR", "AML", "AMC", "AMR", "ST"]
		):
			child.button_pressed = (child.text == position)

	# Refresh player list
	_refresh_player_list()


## Rebuild filter bar (called when search mode changes)
func _rebuild_filter_bar() -> void:
	"""Clear and recreate filter buttons"""
	if not filter_container:
		return

	# Clear all children
	for child in filter_container.get_children():
		child.queue_free()

	# Recreate filter buttons
	_create_filter_buttons()


func _get_filter_label(filter_id: String) -> String:
	"""Get filter button label"""
	match filter_id:
		"all":
			return "전체"
		"GK":
			return "GK"
		"DF":
			return "DF"
		"MF":
			return "MF"
		"FW":
			return "FW"
		_:
			return filter_id


func _on_player_card_selected(player: Dictionary) -> void:
	"""Handle player card selection"""
	print("[HallOfFameScreen] Player selected: %s" % player.get("name", "Unknown"))
	player_selected.emit(player)


func _on_save_team_snapshot() -> void:
	"""Save current team as snapshot"""
	if not MyTeamData:
		_show_message("MyTeamData를 찾을 수 없습니다.")
		return

	# Check max snapshots
	if team_snapshots.size() >= MAX_SNAPSHOTS:
		_show_message("스냅샷 슬롯이 가득 찼습니다.\n(최대 %d개)\n\n기존 스냅샷을 삭제 후 다시 시도하세요." % MAX_SNAPSHOTS)
		return

	var snapshot := _create_team_snapshot()

	# Add to snapshots array and save
	team_snapshots.append(snapshot)
	_save_snapshots()

	_show_message(
		(
			"팀 스냅샷이 저장되었습니다!\n\n%s\n선수 수: %d명\n평균 OVR: %.1f\n\n슬롯: #%d"
			% [
				snapshot.get("team_name", "My Team"),
				snapshot.get("player_count", 0),
				snapshot.get("avg_overall", 0.0),
				team_snapshots.size()
			]
		)
	)

	# Refresh view if in snapshots mode
	if current_view_mode == "snapshots":
		_load_snapshot_list()

	team_selected.emit(snapshot)


func _create_team_snapshot() -> Dictionary:
	"""Create a snapshot of the current team"""
	var players: Array = []
	var total_overall := 0
	var team_name := "My Team"
	var team_tactics: Dictionary = {}
	var academy_settings: Dictionary = {}

	if MyTeamData:
		players = MyTeamData.saved_players.duplicate(true)
		for player in players:
			total_overall += player.get("overall", 0)

		# Get team name from academy_settings
		if "academy_settings" in MyTeamData:
			academy_settings = MyTeamData.academy_settings.duplicate(true)
			team_name = academy_settings.get("team_name", "My Team")

		# Get team tactics
		if "team_tactics" in MyTeamData:
			team_tactics = MyTeamData.team_tactics.duplicate(true)

	var avg_overall: float = float(total_overall) / max(1, players.size())

	return {
		"id": str(Time.get_unix_time_from_system()),
		"timestamp": Time.get_datetime_string_from_system(),
		"team_name": team_name,
		"player_count": players.size(),
		"avg_overall": avg_overall,
		"players": players,
		"tactics": team_tactics,
		"academy_settings": academy_settings,
	}


func _on_back_pressed() -> void:
	"""Handle back button"""
	back_pressed.emit()

	# Navigate back to home screen
	var home_path := "res://scenes/HomeImproved.tscn"
	if ResourceLoader.exists(home_path):
		get_tree().change_scene_to_file(home_path)
	else:
		# Fallback to generic back
		print("[HallOfFameScreen] Warning: HomeImproved.tscn not found")


func _show_message(text: String) -> void:
	"""Show a message popup"""
	var popup := AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "명예의 전당"
	add_child(popup)
	popup.popup_centered(Vector2(400, 250))
	popup.confirmed.connect(popup.queue_free)


# ========================================
# Team Snapshot Persistence
# ========================================


func _ensure_snapshot_dir() -> void:
	"""Ensure snapshot directory exists"""
	var dir := DirAccess.open("user://")
	if dir and not dir.dir_exists("hall_of_fame"):
		dir.make_dir("hall_of_fame")
		print("[HallOfFameScreen] Created hall_of_fame directory")


func _load_snapshots() -> void:
	"""Load team snapshots from file"""
	var path := SNAPSHOT_DIR + SNAPSHOT_FILE
	if not FileAccess.file_exists(path):
		team_snapshots = []
		return

	var file := FileAccess.open(path, FileAccess.READ)
	if not file:
		print("[HallOfFameScreen] ⚠️ Failed to open snapshots file")
		team_snapshots = []
		return

	var json_string := file.get_as_text()
	file.close()

	var json := JSON.new()
	var error := json.parse(json_string)
	if error != OK:
		print("[HallOfFameScreen] ⚠️ Failed to parse snapshots JSON: %s" % json.get_error_message())
		team_snapshots = []
		return

	team_snapshots = json.data if json.data is Array else []
	print("[HallOfFameScreen] ✅ Loaded %d snapshots" % team_snapshots.size())


func _save_snapshots() -> void:
	"""Save team snapshots to file"""
	var path := SNAPSHOT_DIR + SNAPSHOT_FILE
	var file := FileAccess.open(path, FileAccess.WRITE)
	if not file:
		print("[HallOfFameScreen] ⚠️ Failed to save snapshots")
		return

	var json_string := JSON.stringify(team_snapshots, "\t")
	file.store_string(json_string)
	file.close()
	print("[HallOfFameScreen] ✅ Saved %d snapshots" % team_snapshots.size())


func _on_view_mode_changed(mode: String) -> void:
	"""Handle view mode change (players vs snapshots)"""
	current_view_mode = mode
	print("[HallOfFameScreen] View mode changed to: %s" % mode)

	# Update toggle button states
	var view_buttons := []
	for child in filter_container.get_children():
		if child is Button and child.text in ["👤 선수", "📁 스냅샷"]:
			view_buttons.append(child)

	for btn in view_buttons:
		btn.button_pressed = (btn.text == "👤 선수" and mode == "players") or (btn.text == "📁 스냅샷" and mode == "snapshots")

	# Refresh display
	if mode == "players":
		_load_players()
	else:
		_load_snapshot_list()


func _load_snapshot_list() -> void:
	"""Load and display saved snapshots"""
	if not player_list:
		return

	# Clear existing items
	for child in player_list.get_children():
		child.queue_free()

	# Update count
	if player_count_label:
		player_count_label.text = "스냅샷: %d / %d" % [team_snapshots.size(), MAX_SNAPSHOTS]

	if team_snapshots.is_empty():
		var empty_label := Label.new()
		empty_label.text = "저장된 팀 스냅샷이 없습니다.\n\n'현재 팀 스냅샷 저장' 버튼을 눌러\n스테이지 모드에서 사용할 팀을 저장하세요!"
		empty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		empty_label.add_theme_font_size_override("font_size", 18)
		empty_label.modulate = Color(0.7, 0.7, 0.7)
		player_list.add_child(empty_label)
	else:
		for i in range(team_snapshots.size()):
			var snapshot: Dictionary = team_snapshots[i]
			var card := _create_snapshot_card(snapshot, i)
			player_list.add_child(card)


func _create_snapshot_card(snapshot: Dictionary, index: int) -> Control:
	"""Create a snapshot card for display"""
	var card := PanelContainer.new()
	card.custom_minimum_size = Vector2(0, 100)
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 16)
	margin.add_theme_constant_override("margin_right", 16)
	margin.add_theme_constant_override("margin_top", 12)
	margin.add_theme_constant_override("margin_bottom", 12)
	card.add_child(margin)

	var content := HBoxContainer.new()
	content.add_theme_constant_override("separation", 16)
	margin.add_child(content)

	# Left: Slot number
	var slot_label := Label.new()
	slot_label.text = "#%d" % (index + 1)
	slot_label.add_theme_font_size_override("font_size", 28)
	slot_label.add_theme_color_override("font_color", Color(0.8, 0.6, 0.2))
	slot_label.custom_minimum_size = Vector2(50, 0)
	content.add_child(slot_label)

	# Center: Snapshot info
	var info_vbox := VBoxContainer.new()
	info_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	info_vbox.add_theme_constant_override("separation", 4)
	content.add_child(info_vbox)

	# Team name
	var name_label := Label.new()
	name_label.text = snapshot.get("team_name", "Unknown Team")
	name_label.add_theme_font_size_override("font_size", 18)
	info_vbox.add_child(name_label)

	# Stats
	var stats_label := Label.new()
	var player_count: int = snapshot.get("player_count", 0)
	var avg_ovr: float = snapshot.get("avg_overall", 0.0)
	stats_label.text = "선수: %d명 | 평균 OVR: %.1f" % [player_count, avg_ovr]
	stats_label.add_theme_font_size_override("font_size", 14)
	stats_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	info_vbox.add_child(stats_label)

	# Timestamp
	var time_label := Label.new()
	time_label.text = "저장: %s" % snapshot.get("timestamp", "Unknown")
	time_label.add_theme_font_size_override("font_size", 12)
	time_label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
	info_vbox.add_child(time_label)

	# Right: Buttons
	var btn_vbox := VBoxContainer.new()
	btn_vbox.add_theme_constant_override("separation", 4)
	content.add_child(btn_vbox)

	var select_btn := Button.new()
	select_btn.text = "사용"
	select_btn.custom_minimum_size = Vector2(70, 35)
	select_btn.pressed.connect(_on_snapshot_selected.bind(snapshot))
	btn_vbox.add_child(select_btn)

	var delete_btn := Button.new()
	delete_btn.text = "삭제"
	delete_btn.custom_minimum_size = Vector2(70, 35)
	delete_btn.modulate = Color(1.0, 0.6, 0.6)
	delete_btn.pressed.connect(_on_snapshot_delete.bind(index))
	btn_vbox.add_child(delete_btn)

	return card


func _on_snapshot_selected(snapshot: Dictionary) -> void:
	"""Handle snapshot selection for Stage mode"""
	print("[HallOfFameScreen] Snapshot selected: %s" % snapshot.get("team_name", "Unknown"))

	# BM Two-Track: StageManager에 스냅샷 설정
	if StageManager:
		StageManager.set_team_snapshot(snapshot)

	team_selected.emit(snapshot)

	_show_message(
		"'%s' 팀이 선택되었습니다!\n\n이 팀으로 스테이지 모드를 진행합니다.\n\n스테이지 선택 화면으로 이동하세요!" % snapshot.get("team_name", "Unknown")
	)


func _on_snapshot_delete(index: int) -> void:
	"""Delete a snapshot"""
	if index < 0 or index >= team_snapshots.size():
		return

	var snapshot: Dictionary = team_snapshots[index]
	var team_name: String = snapshot.get("team_name", "Unknown")

	# Confirm deletion
	var confirm := ConfirmationDialog.new()
	confirm.dialog_text = "'%s' 스냅샷을 삭제하시겠습니까?" % team_name
	confirm.title = "스냅샷 삭제"
	confirm.confirmed.connect(
		func():
			team_snapshots.remove_at(index)
			_save_snapshots()
			_load_snapshot_list()
			print("[HallOfFameScreen] Deleted snapshot: %s" % team_name)
	)
	add_child(confirm)
	confirm.popup_centered(Vector2(350, 150))
