extends Control

const DraggableSlotScript = preload("res://scripts/components/DraggableSlot.gd")

signal back_pressed

@onready var back_button = $BackButton
@onready var squad_level_label = $SquadLevelLabel
@onready var tab_container = $TabContainer
@onready var players_tab = $TabContainer/Players
@onready var formation_tab = $TabContainer/Formation
@onready var instructions_tab = $TabContainer/Instructions
@onready var statistics_tab = $TabContainer/Statistics

# Players Tab
@onready var player_list = $TabContainer/Players/VBox/PlayerList/ScrollContainer/VBoxContainer
@onready var player_details = $TabContainer/Players/VBox/PlayerDetails
@onready var player_name_label = $TabContainer/Players/VBox/PlayerDetails/VBox/NameLabel
@onready var player_position_label = $TabContainer/Players/VBox/PlayerDetails/VBox/PositionLabel
@onready var player_overall_label = $TabContainer/Players/VBox/PlayerDetails/VBox/OverallLabel
@onready var player_stats_container = $TabContainer/Players/VBox/PlayerDetails/VBox/StatsContainer
@onready var remove_button = $TabContainer/Players/VBox/PlayerDetails/VBox/ButtonContainer/RemoveButton
@onready var add_to_team_button = $TabContainer/Players/VBox/PlayerDetails/VBox/ButtonContainer/AddToTeamButton

# Formation Tab
@onready var formation_selector = $TabContainer/Formation/VBox/FormationSelector
@onready var pitch_view = $TabContainer/Formation/VBox/PitchView
@onready var bench_container = $TabContainer/Formation/VBox/BenchContainer/HBox
@onready var formation_select_button = $TabContainer/Formation/VBox/ButtonContainer/FormationSelectButton
@onready var save_formation_button = $TabContainer/Formation/VBox/ButtonContainer/SaveFormationButton
@onready var test_match_button = $TabContainer/Formation/VBox/ButtonContainer/TestMatchButton

# Instructions Tab
@onready var instructions_container = $TabContainer/Instructions/VBox/ScrollContainer/GridContainer
@onready var formation_info_label = $TabContainer/Instructions/VBox/FormationInfo

# Statistics Tab
@onready var total_players_label = $TabContainer/Statistics/VBox/TotalPlayers
@onready var avg_overall_label = $TabContainer/Statistics/VBox/AvgOverall
@onready var position_chart = $TabContainer/Statistics/VBox/PositionChart
@onready var ending_chart = $TabContainer/Statistics/VBox/EndingChart
@onready var branding_button = $TabContainer/Statistics/VBox/BrandingButton

var selected_player: Dictionary = {}
var player_buttons = {}
var formation_slots = []
var bench_slots = []

# BM Two-Track: 탭 해금 조건 (스테이지 클리어 기준)
const UNLOCK_STATISTICS_STAGE := 5  # Statistics 탭: Stage 5 클리어 시 해금
const UNLOCK_INSTRUCTIONS_STAGE := 10  # Instructions 탭: Stage 10 클리어 시 해금
const UNLOCK_TACTICS_STAGE := 20  # TeamInstructions 탭: Stage 20 클리어 시 해금


func _ready():
	print("[MyTeamScreen] Initializing...")

	# 버튼 연결
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	if remove_button:
		remove_button.pressed.connect(_on_remove_player)

	if add_to_team_button:
		add_to_team_button.pressed.connect(_on_add_to_team)

	if formation_select_button:
		formation_select_button.pressed.connect(_on_formation_select_pressed)

	if save_formation_button:
		save_formation_button.pressed.connect(_save_formation)

	if test_match_button:
		test_match_button.pressed.connect(_test_match)

	if formation_selector:
		formation_selector.item_selected.connect(_on_formation_changed)
		_setup_formation_options()

	if branding_button:
		branding_button.pressed.connect(_on_branding_pressed)

	# MyTeamData 시그널 연결
	if MyTeamData:
		MyTeamData.player_saved.connect(_on_player_saved)
		MyTeamData.player_removed.connect(_on_player_removed)
		MyTeamData.team_updated.connect(_on_team_updated)

	# 팀 전술 탭 추가 (동적 생성)
	_add_team_tactics_tab()

	# BM Two-Track: 탭 해금 상태 적용
	_apply_tab_unlock_status()

	# 초기 데이터 로드
	_load_players()
	_load_formation()
	_load_instructions()
	_update_statistics()
	_update_squad_level_display()

	# PromotionPopup 추가 (전역적으로 사용)
	_add_promotion_popup()

	print("[MyTeamScreen] Ready!")


func _setup_formation_options():
	"""Setup formation options from OpenFootball API"""
	if not formation_selector:
		return

	formation_selector.clear()

	# Get formations from FootballRustEngine
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamScreen] Warning: FootballRustEngine not available, using fallback")
		_setup_fallback_formations()
		return

	var result = rust_engine.get_all_formations()
	if not result.get("success", false):
		print("[MyTeamScreen] Error loading formations: %s" % result.get("error", "Unknown"))
		_setup_fallback_formations()
		return

	var formations = result.get("formations", [])
	if formations.size() == 0:
		print("[MyTeamScreen] No formations returned, using fallback")
		_setup_fallback_formations()
		return

	# Add all formations to selector
	for formation in formations:
		var formation_id = formation.get("id", "")
		var display_name = formation.get("name_ko", formation.get("name_en", formation_id))
		formation_selector.add_item(display_name)
		formation_selector.set_item_metadata(formation_selector.get_item_count() - 1, formation_id)

	# Select current formation
	if MyTeamData:
		var current_formation = MyTeamData.current_team.formation
		for i in range(formation_selector.get_item_count()):
			var formation_id = formation_selector.get_item_metadata(i)
			if formation_id == current_formation:
				formation_selector.select(i)
				break

	print("[MyTeamScreen] Loaded %d formations" % formations.size())


func _setup_fallback_formations():
	"""Setup fallback formations when API fails"""
	var formations = [{"id": "T442", "name": "4-4-2"}, {"id": "T433", "name": "4-3-3"}, {"id": "T352", "name": "3-5-2"}]
	for formation in formations:
		formation_selector.add_item(formation.name)
		formation_selector.set_item_metadata(formation_selector.get_item_count() - 1, formation.id)


func _load_players():
	"""Load saved players from MyTeamData"""
	if not player_list or not MyTeamData:
		return

	# Clear existing buttons
	for child in player_list.get_children():
		child.queue_free()
	player_buttons.clear()

	# Create player buttons
	var players = MyTeamData.saved_players
	if players.size() == 0:
		# Show empty message
		var empty_label = Label.new()
		empty_label.text = "No players yet!\nComplete Career Mode to add players"
		empty_label.add_theme_font_size_override("font_size", 18)
		empty_label.modulate = Color(0.7, 0.7, 0.7)
		player_list.add_child(empty_label)
	else:
		for player in players:
			var button = _create_player_button(player)
			player_list.add_child(button)
			player_buttons[player.get("id", "")] = button

	print("[MyTeamScreen] Loaded %d players" % players.size())


func _create_player_button(player: Dictionary) -> PlayerCard:
	"""Create player card (ThemeManager 표준화, Phase 2)"""
	var card = PlayerCard.create_from_data(player)
	card.selected.connect(func(data): _on_player_selected(data))
	card.double_clicked.connect(func(data): _on_player_double_clicked(data))
	return card


func _on_player_double_clicked(player: Dictionary):
	"""선수 더블클릭 - 포메이션에 바로 추가"""
	selected_player = player
	_update_player_details()
	# 포메이션 탭으로 이동
	if tab_container:
		tab_container.current_tab = 1


func _get_position_color(position: String) -> Color:
	"""Get position color - ThemeManager 위임 (Phase 2 표준화)"""
	return ThemeManager.get_position_color(position)


func _get_star_rating(overall: int) -> String:
	"""Get star rating - ThemeManager 위임 (Phase 2 표준화)"""
	return ThemeManager.get_star_rating(overall)


func _show_message(text: String):
	"""Show temporary message popup"""
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "My Team"
	add_child(popup)
	popup.popup_centered(Vector2(400, 200))
	popup.confirmed.connect(popup.queue_free)


func _on_player_selected(player: Dictionary):
	"""선수 선택 처리"""
	selected_player = player
	_update_player_details()


func _update_player_details():
	"""선수 상세 정보 업데이트"""
	if not selected_player or selected_player.size() == 0:
		if player_details:
			player_details.visible = false
		return

	if player_details:
		player_details.visible = true

	if player_name_label:
		player_name_label.text = selected_player.get("name", "Unknown")
		player_name_label.add_theme_font_size_override("font_size", 24)

	if player_position_label:
		player_position_label.text = "Position: %s" % selected_player.get("position", "")
		player_position_label.add_theme_font_size_override("font_size", 20)

	if player_overall_label:
		player_overall_label.text = "Overall: %d" % selected_player.get("overall", 0)
		player_overall_label.add_theme_font_size_override("font_size", 20)

	# 스탯 표시
	_display_player_stats()


func _display_player_stats():
	"""선수 스탯 표시"""
	if not player_stats_container:
		return

	# 기존 스탯 제거
	for child in player_stats_container.get_children():
		child.queue_free()

	# 주요 스탯 표시
	var categories = ["technical", "mental", "physical"]
	for category in categories:
		var stats = selected_player.get(category, {})
		if stats.size() > 0:
			var category_label = Label.new()
			category_label.text = category.capitalize() + " Stats:"
			category_label.add_theme_font_size_override("font_size", 18)
			player_stats_container.add_child(category_label)

			var grid = GridContainer.new()
			grid.columns = 2
			player_stats_container.add_child(grid)

			# 상위 5개 스탯만 표시
			var sorted_stats = []
			for stat_name in stats:
				sorted_stats.append([stat_name, stats[stat_name]])
			sorted_stats.sort_custom(func(a, b): return a[1] > b[1])

			for i in range(min(5, sorted_stats.size())):
				var stat_label = Label.new()
				stat_label.text = "%s: %d" % [sorted_stats[i][0].capitalize(), sorted_stats[i][1]]
				stat_label.add_theme_font_size_override("font_size", 16)
				grid.add_child(stat_label)

			# 구분선
			var separator = HSeparator.new()
			player_stats_container.add_child(separator)


func _on_remove_player():
	"""선수 방출"""
	if selected_player.size() == 0:
		return

	# 확인 다이얼로그 (간단한 구현)
	if MyTeamData:
		if MyTeamData.remove_player(selected_player.get("id", "")):
			print("[MyTeamScreen] Player removed: %s" % selected_player.get("name", "Unknown"))
			selected_player = {}
			_load_players()
			_update_player_details()


func _on_add_to_team():
	"""현재 팀에 추가"""
	if selected_player.size() == 0:
		return

	# 포메이션 탭으로 전환
	if tab_container:
		tab_container.current_tab = 1  # Formation tab
		# 선수를 드래그 가능한 상태로 만들기
		print("[MyTeamScreen] Ready to place %s in formation" % selected_player.get("name", "Unknown"))


func _load_formation():
	"""포메이션 로드 및 표시"""
	if not pitch_view:
		return

	# 기존 슬롯 제거
	for slot in formation_slots:
		slot.queue_free()
	formation_slots.clear()

	# 포메이션에 따른 위치 생성
	var formation_id = MyTeamData.current_team.formation if MyTeamData else "T442"
	var positions = _get_formation_positions(formation_id)

	# 포지션 슬롯 생성
	for i in range(11):
		var pos_data = positions[i] if i < positions.size() else {"x": 0.5, "y": 0.5}
		var slot = _create_position_slot(i, Vector2(pos_data.x, pos_data.y))
		pitch_view.add_child(slot)
		formation_slots.append(slot)

	# 벤치 슬롯 생성
	if bench_container:
		for slot in bench_slots:
			slot.queue_free()
		bench_slots.clear()

		for i in range(7):  # 7명 교체 선수
			var bench_slot = _create_bench_slot(i + 11)
			bench_container.add_child(bench_slot)
			bench_slots.append(bench_slot)


func _get_formation_positions(formation_id: String) -> Array:
	"""Get formation positions from OpenFootball API"""
	# Get formation details from Rust engine
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamScreen] Warning: FootballRustEngine not available, using fallback")
		return _get_fallback_positions()

	var result = rust_engine.get_formation_details(formation_id)
	if not result.get("success", false):
		print("[MyTeamScreen] Error loading formation details: %s" % result.get("error", "Unknown"))
		return _get_fallback_positions()

	var formation = result.get("formation", {})
	var positions = formation.get("positions", [])

	if positions.size() != 11:
		print("[MyTeamScreen] Warning: Expected 11 positions, got %d" % positions.size())
		return _get_fallback_positions()

	# Convert OpenFootball positions (x, y are already normalized 0-1)
	var position_array = []
	for pos in positions:
		position_array.append(
			{
				"x": pos.get("x", 0.5),
				"y": pos.get("y", 0.5),
				"role": pos.get("position_type", ""),
				"name_ko": pos.get("position_name_ko", "")
			}
		)

	return position_array


func _get_fallback_positions() -> Array:
	"""Get fallback 4-4-2 positions when API fails (OpenFootball coordinate system)"""
	# OpenFootball 좌표계: y=0.1 = 골키퍼(아래), y=0.9 = 공격수(위)
	return [
		{"x": 0.5, "y": 0.1, "role": "GK"},  # 골키퍼 - 아래
		{"x": 0.15, "y": 0.25, "role": "LB"},  # 왼쪽 수비수
		{"x": 0.35, "y": 0.2, "role": "CB"},  # 센터백
		{"x": 0.65, "y": 0.2, "role": "CB"},  # 센터백
		{"x": 0.85, "y": 0.25, "role": "RB"},  # 오른쪽 수비수
		{"x": 0.15, "y": 0.55, "role": "LM"},  # 왼쪽 미드필더
		{"x": 0.35, "y": 0.5, "role": "CM"},  # 센터 미드필더
		{"x": 0.65, "y": 0.5, "role": "CM"},  # 센터 미드필더
		{"x": 0.85, "y": 0.55, "role": "RM"},  # 오른쪽 미드필더
		{"x": 0.35, "y": 0.8, "role": "ST"},  # 스트라이커 - 위
		{"x": 0.65, "y": 0.8, "role": "ST"}  # 스트라이커 - 위
	]


func _create_position_slot(index: int, position: Vector2) -> Control:
	"""포지션 슬롯 생성 (with drag & drop support)"""
	# Create draggable slot wrapper (Control node for drag & drop)
	var slot_wrapper = Control.new()
	slot_wrapper.custom_minimum_size = Vector2(80, 100)
	# Y축 플립: 골키퍼(y=0.1) 아래, 공격수(y=0.9) 위
	var flipped_position = Vector2(position.x, 1.0 - position.y)
	slot_wrapper.position = flipped_position * Vector2(600, 800)  # 피치 크기에 맞게 조정

	# Store metadata
	slot_wrapper.set_meta("slot_index", index)
	slot_wrapper.set_meta("is_bench", false)

	# Create visual button as child
	var button = Button.new()
	button.custom_minimum_size = Vector2(80, 100)
	button.set_anchors_preset(Control.PRESET_FULL_RECT)
	button.anchor_right = 1.0
	button.anchor_bottom = 1.0

	# 현재 팀에서 해당 위치의 선수 확인
	if MyTeamData and index < MyTeamData.current_team.players.size():
		var player_id = MyTeamData.current_team.players[index]
		if player_id != "":
			var player = MyTeamData.get_player_by_id(player_id)
			if player.size() > 0:
				button.text = player.get("name", "Unknown").split(" ")[0] + "\n" + str(player.get("overall", 0))

	button.pressed.connect(func(): _on_slot_clicked(index, false))
	slot_wrapper.add_child(button)

	# Setup drag & drop handlers
	slot_wrapper.set_script(DraggableSlotScript)
	if slot_wrapper.has_method("setup"):
		slot_wrapper.setup(index, false, self)

	return slot_wrapper


func _create_bench_slot(index: int) -> Control:
	"""벤치 슬롯 생성 (with drag & drop support)"""
	# Create draggable slot wrapper (Control node for drag & drop)
	var slot_wrapper = Control.new()
	slot_wrapper.custom_minimum_size = Vector2(80, 100)

	# Store metadata
	slot_wrapper.set_meta("slot_index", index)
	slot_wrapper.set_meta("is_bench", true)

	# Create visual button as child
	var button = Button.new()
	button.custom_minimum_size = Vector2(80, 100)
	button.set_anchors_preset(Control.PRESET_FULL_RECT)
	button.anchor_right = 1.0
	button.anchor_bottom = 1.0

	# 현재 팀에서 해당 위치의 선수 확인
	if MyTeamData and index < MyTeamData.current_team.players.size():
		var player_id = MyTeamData.current_team.players[index]
		if player_id != "":
			var player = MyTeamData.get_player_by_id(player_id)
			if player.size() > 0:
				button.text = player.get("name", "Unknown").split(" ")[0] + "\n" + str(player.get("overall", 0))

	button.pressed.connect(func(): _on_slot_clicked(index, true))
	slot_wrapper.add_child(button)

	# Setup drag & drop handlers
	slot_wrapper.set_script(DraggableSlotScript)
	if slot_wrapper.has_method("setup"):
		slot_wrapper.setup(index, true, self)

	return slot_wrapper


func _on_slot_clicked(index: int, is_bench: bool):
	"""슬롯 클릭 처리 - Player placement"""
	# Place selected player from Players tab
	if selected_player.size() > 0:
		if MyTeamData:
			if MyTeamData.add_to_current_team(selected_player.get("id", ""), index):
				print("[MyTeamScreen] Player placed at position %d" % index)
				_load_formation()
				selected_player = {}
		return


func _on_formation_changed(index: int):
	"""포메이션 변경"""
	if not formation_selector or not MyTeamData:
		return

	# Get formation ID from metadata
	var formation_id = formation_selector.get_item_metadata(index)
	if formation_id == null or formation_id == "":
		formation_id = formation_selector.get_item_text(index)  # Fallback to text

	MyTeamData.set_team_formation(formation_id)

	# Auto-apply default roles based on positions
	_auto_apply_default_roles(formation_id)

	_load_formation()
	_load_instructions()  # Reload instructions tab
	print("[MyTeamScreen] Formation changed to: %s" % formation_id)


func _save_formation():
	"""포메이션 저장"""
	if MyTeamData:
		MyTeamData.save_to_file()
		print("[MyTeamScreen] Formation saved")


func _test_match():
	"""Run test match with OpenFootball engine (Phase 5: with team instructions)"""
	print("[MyTeamScreen] Starting test match with My Team...")

	if not MyTeamData:
		_show_message("MyTeamData를 찾을 수 없습니다.")
		return

	# Prepare team data
	var team_data = MyTeamData.get_team_for_match()
	if not team_data or team_data.players.size() < 11:
		_show_message("경기를 하려면 최소 11명의 선수가 필요합니다!\n포메이션 탭에서 선수를 배치해주세요.")
		return

	print("[MyTeamScreen] Team data prepared: %d players" % team_data.players.size())

	# Check if FootballRustEngine is available (Phase 5: direct access)
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine:
		_show_message("FootballRustEngine을 찾을 수 없습니다.")
		print("[MyTeamScreen] Error: FootballRustEngine not found")
		return

	if not rust_engine.is_ready():
		_show_message("매치 엔진이 준비되지 않았습니다.\nRust GDExtension을 확인해주세요.")
		print("[MyTeamScreen] Error: Match engine not ready")
		return

	# Get team instructions from TeamInstructionsPanel (Phase 5)
	var my_team_instructions = {}
	if tab_container:
		for i in range(tab_container.get_child_count()):
			var tab = tab_container.get_child(i)
			if tab.name == "TeamInstructions" and tab.has_method("get_current_instructions"):
				my_team_instructions = tab.get_current_instructions()
				print("[MyTeamScreen] Team instructions loaded: %s" % my_team_instructions)
				break

	# Create opponent team
	var opponent_team = _generate_opponent_team()

	# Generate opponent instructions (random tactical style)
	var opponent_presets = ["Balanced", "HighPressing", "Counterattack", "Possession", "Defensive"]
	var opponent_preset = opponent_presets[randi() % opponent_presets.size()]

	# Get opponent instructions from preset
	var opponent_instructions_result = rust_engine.set_team_instructions_preset(opponent_preset)
	var opponent_instructions = {}
	if opponent_instructions_result.get("success", false):
		opponent_instructions = opponent_instructions_result.get("instructions", {})
		print("[MyTeamScreen] Opponent using %s preset: %s" % [opponent_preset, opponent_instructions])

	# Show match starting message
	_show_message("매치를 시작합니다...\n상대: %s (%s 전술)" % [opponent_team.name, opponent_preset])

	# Phase 17.1: Replace direct engine call with Game OS workflow
	print("[MyTeamScreen] Starting match simulation (Game OS mode)...")

	# Extract UIDs from MyTeam
	var home_roster_uids = _extract_player_uids_from_my_team(team_data)

	# Extract/generate UIDs for opponent team
	var away_roster_uids = []
	var opponent_players = opponent_team.get("players", [])
	for i in range(18):
		if i < opponent_players.size():
			var player = opponent_players[i]
			var uid = player.get("uid", "")
			if uid == "":
				uid = player.get("id", "")
			if uid == "" or not uid.begins_with("csv:"):
				# Generate from overall rating
				var overall = player.get("overall", 70)
				uid = "csv:%d" % clamp(overall, 1, 1000)
			away_roster_uids.append(uid)
		else:
			# Pad with random players
			away_roster_uids.append("csv:%d" % randi_range(1, 1000))

	# Create PlayerLibrary
	var player_library = PlayerLibrary.new()

	# Build match configuration
	var match_seed = randi()
	var match_config = {
		"seed": match_seed,
		"match_id": "myteam_test_%d" % Time.get_ticks_usec(),
		"match_type": "friendly",
		"venue": "home",
		"home_formation": team_data.get("formation", "4-4-2"),
		"away_formation": opponent_team.get("formation", "4-4-2"),
		"home_tactics": my_team_instructions
	}

	# Build MatchSetup via Game OS
	var match_setup = MatchSetupBuilder.build(
		home_roster_uids,
		away_roster_uids,
		match_config["home_formation"],
		match_config["away_formation"],
		player_library,
		match_config
	)

	if not match_setup:
		_show_message("MatchSetup 생성에 실패했습니다.")
		return

	print("[MyTeamScreen] ✅ MatchSetup created (Game OS mode)")

	# Execute simulation via Game OS
	var result = OpenFootballAPI.simulate_match_with_setup(match_setup, match_seed, "simple", true)  # fast_mode for quick test

	# Show result
	_show_match_result(result, team_data, opponent_team)


# ============================================================================
# Phase 17.1: Game OS Migration - Helper Function
# ============================================================================


func _extract_player_uids_from_my_team(team_data: Dictionary) -> Array:
	"""Extract UIDs from MyTeam player data

	Phase 17.1: Game OS Migration
	Converts MyTeam player data to UID array for MatchSetupBuilder.
	"""
	var roster_uids = []
	var players = team_data.get("players", [])

	for player in players:
		# Try multiple UID fields (MyTeam may store UIDs differently)
		var uid = player.get("uid", "")
		if uid == "":
			uid = player.get("id", "")
		if uid == "" or not uid.begins_with("csv:"):
			# Fallback: Generate CSV UID from overall rating
			if player.has("overall"):
				uid = "csv:%d" % clamp(player.overall, 1, 1000)
			else:
				uid = "csv:%d" % randi_range(1, 1000)

		roster_uids.append(uid)

	# Pad to 18 players if needed
	while roster_uids.size() < 18:
		roster_uids.append("csv:%d" % randi_range(1, 1000))

	print("[MyTeamScreen] Extracted %d UIDs from MyTeam (Game OS mode)" % roster_uids.size())
	return roster_uids.slice(0, 18)  # Ensure exactly 18 players


# ============================================================================


func _generate_opponent_team() -> Dictionary:
	"""Generate a random opponent team"""
	var team_names = [
		"FC Seoul",
		"Busan FC",
		"Daegu United",
		"Incheon Dolphins",
		"Gwangju Stars",
		"Daejeon Eagles",
		"Ulsan Tigers",
		"Suwon Royals",
		"Gangnam United",
		"Jamsil FC",
		"Apgujeong Stars",
		"Bundang City"
	]

	var team_name = team_names[randi() % team_names.size()]

	# Generate opponent players
	var positions = ["GK", "LB", "CB", "CB", "RB", "CDM", "CM", "CM", "LW", "RW", "ST"]
	var players = []

	for i in range(positions.size()):
		var player_name = "%s Player %d" % [team_name, i + 1]
		players.append({"name": player_name, "position": positions[i], "overall": randi_range(65, 80)})

	return {"name": team_name, "formation": "T442", "players": players}


func _show_match_result(result: Dictionary, our_team: Dictionary, opponent_team: Dictionary):
	"""Display match result in a popup (Phase 5: supports new format)"""
	if not result.get("success", false):
		_show_message("매치 시뮬레이션에 실패했습니다.\n오류: %s" % result.get("error", "Unknown"))
		return

	# Phase 5: Handle new result format from simulate_match_with_instructions()
	var match_result = result.get("match_result", {})
	var home_score = 0
	var away_score = 0
	var events = []
	var instructions_applied = false

	if match_result.size() > 0:
		# New format (Phase 5)
		home_score = match_result.get("goals_home", 0)
		away_score = match_result.get("goals_away", 0)
		var legacy_payload_key := "re" + "play"
		var legacy_doc_key := legacy_payload_key + "_doc"
		var timeline_variant: Variant = result.get(
			"timeline_doc", result.get(legacy_payload_key, result.get(legacy_doc_key, {}))
		)
		var timeline_doc: Dictionary = timeline_variant if timeline_variant is Dictionary else {}
		events = timeline_doc.get("events", [])
		instructions_applied = (
			match_result.get("home_instructions_applied", false) or match_result.get("away_instructions_applied", false)
		)
	else:
		# Legacy format (fallback)
		home_score = result.get("score_home", 0)
		away_score = result.get("score_away", 0)
		events = result.get("events", [])

	var result_text = ""

	# Match result header
	if home_score > away_score:
		result_text = "🎉 승리! 🎉\n\n"
	elif home_score < away_score:
		result_text = "😢 패배... 😢\n\n"
	else:
		result_text = "🤝 무승부 🤝\n\n"

	# Score
	result_text += (
		"%s  %d - %d  %s\n\n"
		% [our_team.get("name", "My Team"), home_score, away_score, opponent_team.get("name", "Opponent")]
	)

	# Phase 5: Show if tactical instructions were applied
	if instructions_applied:
		result_text += "⚙️ 팀 전술 지시사항 적용됨\n\n"

	# Match statistics (Phase 5: from new format)
	if match_result.size() > 0:
		result_text += "📊 경기 통계:\n"
		result_text += (
			"점유율: %d%% - %d%%\n" % [match_result.get("possession_home", 50), match_result.get("possession_away", 50)]
		)
		result_text += "슈팅: %d - %d\n" % [match_result.get("shots_home", 0), match_result.get("shots_away", 0)]
		result_text += "\n"

	# Key events (goals)
	if events.size() > 0:
		result_text += "주요 이벤트:\n"
		var goal_count = 0
		for event in events:
			if event is Dictionary and event.get("event_type", "") == "Goal":
				var minute = event.get("minute", 0)
				var player = event.get("player_name", "Unknown")
				var team = event.get("team", "")
				var icon = "⚽" if team == "home" else "⚪"
				result_text += "%s %d' %s\n" % [icon, minute, player]
				goal_count += 1
				if goal_count >= 5:  # Limit to 5 goals for display
					break

	_show_message(result_text)


func _on_formation_select_pressed():
	"""Open FormationSelectionScreen for visual formation selection"""
	print("[MyTeamScreen] Opening FormationSelectionScreen...")

	# Load FormationSelectionScreen
	var formation_screen_scene = load("res://scenes/ui/FormationSelectionScreen.tscn")
	if not formation_screen_scene:
		_show_message("FormationSelectionScreen을 불러올 수 없습니다.")
		return

	var formation_screen = formation_screen_scene.instantiate()

	# Set fullscreen
	formation_screen.set_anchors_preset(Control.PRESET_FULL_RECT)
	formation_screen.anchor_right = 1.0
	formation_screen.anchor_bottom = 1.0

	# Set squad players and current formation
	if MyTeamData:
		if formation_screen.has_method("set_squad_players"):
			formation_screen.set_squad_players(MyTeamData.saved_players)
		if formation_screen.has_method("set_current_formation"):
			formation_screen.set_current_formation(MyTeamData.current_team.formation)

	# Connect signals
	if formation_screen.has_signal("formation_applied"):
		formation_screen.formation_applied.connect(_on_formation_applied_from_screen)
	if formation_screen.has_signal("screen_closed"):
		formation_screen.screen_closed.connect(_on_formation_screen_closed.bind(formation_screen))

	add_child(formation_screen)
	print("[MyTeamScreen] FormationSelectionScreen opened")


func _on_formation_applied_from_screen(formation_id: String):
	"""Handle formation applied from FormationSelectionScreen"""
	print("[MyTeamScreen] Formation applied from screen: %s" % formation_id)

	if not MyTeamData:
		return

	# Apply the formation
	MyTeamData.set_team_formation(formation_id)

	# Update formation selector to match
	if formation_selector:
		for i in range(formation_selector.get_item_count()):
			var selector_formation_id = formation_selector.get_item_metadata(i)
			if selector_formation_id == formation_id:
				formation_selector.select(i)
				break

	# Auto-apply default roles
	_auto_apply_default_roles(formation_id)

	# Reload views
	_load_formation()
	_load_instructions()

	print("[MyTeamScreen] Formation updated to: %s" % formation_id)


func _on_formation_screen_closed(formation_screen: Control):
	"""Handle FormationSelectionScreen closed"""
	print("[MyTeamScreen] FormationSelectionScreen closed")
	formation_screen.queue_free()


func _update_statistics():
	"""통계 업데이트"""
	if not MyTeamData:
		return

	var stats = MyTeamData.get_statistics()

	if total_players_label:
		total_players_label.text = "Total Players: %d / %d" % [stats.total_players, MyTeamData.MAX_PLAYERS]
		total_players_label.add_theme_font_size_override("font_size", 22)

	if avg_overall_label:
		avg_overall_label.text = "Average Overall: %.1f" % stats.avg_overall
		avg_overall_label.add_theme_font_size_override("font_size", 20)

	# 포지션별 차트 (간단한 텍스트 버전)
	if position_chart:
		for child in position_chart.get_children():
			child.queue_free()

		var pos_label = Label.new()
		pos_label.text = "Players by Position:"
		pos_label.add_theme_font_size_override("font_size", 18)
		position_chart.add_child(pos_label)

		for pos in stats.by_position:
			var count_label = Label.new()
			count_label.text = "%s: %d players" % [pos, stats.by_position[pos]]
			count_label.add_theme_font_size_override("font_size", 16)
			position_chart.add_child(count_label)

	# 엔딩별 차트
	if ending_chart:
		for child in ending_chart.get_children():
			child.queue_free()

		var ending_label = Label.new()
		ending_label.text = "Players by Ending Type:"
		ending_label.add_theme_font_size_override("font_size", 18)
		ending_chart.add_child(ending_label)

		for ending in stats.by_ending:
			var count_label = Label.new()
			count_label.text = "%s: %d players" % [ending.capitalize(), stats.by_ending[ending]]
			count_label.add_theme_font_size_override("font_size", 16)
			ending_chart.add_child(count_label)


func _update_squad_level_display():
	"""Update squad level display label"""
	if not squad_level_label or not MyTeamData:
		return

	var squad_level = MyTeamData.squad_level if "squad_level" in MyTeamData else 0
	var level_name = ""

	match squad_level:
		0:  # YOUTH
			level_name = "🏫 Squad: U18 Youth"
		1:  # BTEAM
			level_name = "⚽ Squad: B-Team"
		2:  # ATEAM
			level_name = "🏆 Squad: A-Team"
		_:
			level_name = "Squad: Unknown"

	squad_level_label.text = level_name
	print("[MyTeamScreen] Squad level display updated: %s" % level_name)


func _add_promotion_popup():
	"""Add PromotionPopup to scene tree"""
	var promotion_popup_scene = load("res://scenes/ui/PromotionPopup.tscn")
	if not promotion_popup_scene:
		print("[MyTeamScreen] ⚠️ Failed to load PromotionPopup scene")
		return

	var promotion_popup = promotion_popup_scene.instantiate()

	# Set as fullscreen overlay (on top of everything)
	promotion_popup.set_anchors_preset(Control.PRESET_FULL_RECT)
	promotion_popup.anchor_right = 1.0
	promotion_popup.anchor_bottom = 1.0
	promotion_popup.z_index = 100  # Ensure it's on top

	add_child(promotion_popup)
	print("[MyTeamScreen] PromotionPopup added successfully")


func _on_player_saved(player_data: Dictionary):
	"""새 선수 저장됨"""
	_load_players()
	_update_statistics()


func _on_player_removed(player_id: String):
	"""선수 방출됨"""
	_load_players()
	_update_statistics()


func _on_team_updated():
	"""팀 업데이트됨"""
	_load_formation()
	_update_squad_level_display()


func _on_back_pressed():
	"""Back to Main Home Screen"""
	print("[MyTeamScreen] Back to Main Home")
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_branding_pressed():
	"""Open Team Branding Setup Screen"""
	print("[MyTeamScreen] Opening Team Branding Setup...")

	var branding_scene = load("res://scenes/ui/MyTeamSetupScreen_Full.tscn")
	if not branding_scene:
		_show_message("브랜딩 화면을 불러올 수 없습니다.")
		return

	var branding_screen = branding_scene.instantiate()
	branding_screen.set_anchors_preset(Control.PRESET_FULL_RECT)
	branding_screen.anchor_right = 1.0
	branding_screen.anchor_bottom = 1.0

	# Connect close signal if available
	if branding_screen.has_signal("closed"):
		branding_screen.closed.connect(_on_branding_closed.bind(branding_screen))

	add_child(branding_screen)
	print("[MyTeamScreen] Team Branding Setup opened")


func _on_branding_closed(branding_screen: Control):
	"""Handle branding screen closed"""
	print("[MyTeamScreen] Branding screen closed, refreshing team data...")
	branding_screen.queue_free()
	_update_statistics()  # Refresh statistics in case branding changed


# ===== Instructions Tab =====


func _auto_apply_default_roles(formation_id: String):
	"""Auto-apply default roles to players based on their positions"""
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamScreen] Cannot auto-apply roles: FootballRustEngine not ready")
		return

	var formation_result = rust_engine.get_formation_details(formation_id)
	if not formation_result.get("success", false):
		print("[MyTeamScreen] Error loading formation details for auto-apply")
		return

	var formation = formation_result.get("formation", {})
	var positions = formation.get("positions", [])

	# Apply default role to each position
	for i in range(min(11, positions.size())):
		var position = positions[i]
		var position_type = position.get("position_type", "")
		var default_role = _get_default_role_for_position(position_type)

		if default_role != null:
			var player_data = MyTeamData.get_player_at_slot(i)
			if player_data.size() > 0:
				# Only apply if player doesn't already have a role
				if not player_data.has("role") or player_data.get("role", null) == null:
					if MyTeamData.set_player_role(i, default_role):
						print("[MyTeamScreen] Auto-applied role %s to slot %d" % [default_role, i])

	print("[MyTeamScreen] Default roles auto-applied for %s" % formation_id)


func _convert_position_type_to_api_code(position_type: String) -> String:
	"""Convert long position_type to short API code that get_available_roles() expects"""
	match position_type:
		"ForwardLeft", "ForwardCenter", "ForwardRight", "Striker":
			return "ST"
		"AttackingMidfielderLeft", "AttackingMidfielderCenter", "AttackingMidfielderRight":
			return "CAM"
		"MidfielderLeft", "MidfielderRight":
			return "CM"  # Can also be LM/RM
		"MidfielderCenter", "MidfielderCenterLeft", "MidfielderCenterRight":
			return "CM"
		"DefensiveMidfielder":
			return "CDM"
		"DefenderLeft":
			return "LB"
		"DefenderRight":
			return "RB"
		"DefenderCenter", "DefenderCenterLeft", "DefenderCenterRight", "Sweeper":
			return "CB"
		"WingbackLeft":
			return "LWB"
		"WingbackRight":
			return "RWB"
		"Goalkeeper":
			return "GK"
		_:
			# If already short code, return as-is
			return position_type


func _get_default_role_for_position(position_type: String):
	"""Get default role for a position type (returns String or null)"""
	match position_type:
		"ForwardLeft", "ForwardCenter", "ForwardRight", "Striker":
			return "CompleteForward"
		"AttackingMidfielderLeft", "AttackingMidfielderCenter", "AttackingMidfielderRight":
			return "Playmaker"
		"MidfielderLeft", "MidfielderCenter", "MidfielderRight", "MidfielderCenterLeft", "MidfielderCenterRight":
			return "BoxToBox"
		"DefensiveMidfielder":
			return "BallWinning"
		"DefenderLeft", "DefenderCenter", "DefenderRight", "DefenderCenterLeft", "DefenderCenterRight":
			return "Stopper"
		"Sweeper":
			return "BallPlayingDefender"
		"WingbackLeft", "WingbackRight":
			return null  # Wingbacks don't have default roles
		"Goalkeeper":
			return null  # GK doesn't have roles
		_:
			return null


func _load_instructions():
	"""Load and setup Instructions tab for 11 players"""
	if not instructions_container or not MyTeamData:
		return

	# Clear existing content
	for child in instructions_container.get_children():
		child.queue_free()

	var formation_id = MyTeamData.current_team.formation
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if not rust_engine or not rust_engine.is_ready():
		print("[MyTeamScreen] Warning: FootballRustEngine not available")
		return

	var formation_result = rust_engine.get_formation_details(formation_id)
	if not formation_result.get("success", false):
		print("[MyTeamScreen] Error loading formation details")
		return

	var formation = formation_result.get("formation", {})
	var positions = formation.get("positions", [])

	# Update formation info label
	if formation_info_label:
		formation_info_label.text = (
			"포메이션: %s (%s)" % [formation.get("name_ko", formation_id), formation.get("name_en", formation_id)]
		)

	# Create instruction cards for 11 players
	for i in range(11):
		var position = positions[i] if i < positions.size() else {}
		var player_card = _create_player_instruction_card(i, position)
		instructions_container.add_child(player_card)

	print("[MyTeamScreen] Instructions tab loaded for %s" % formation_id)


func _create_player_instruction_card(slot: int, position: Dictionary) -> Control:
	"""Create instruction card for one player (GridContainer optimized, ThemeManager 스타일)"""
	var card = PanelContainer.new()
	card.custom_minimum_size = Vector2(400, 180)  # GridContainer 2열 최적화
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# ThemeManager 카드 스타일 적용 (Phase 2)
	var player_data = MyTeamData.get_player_at_slot(slot)
	var player_position = player_data.get("position", position.get("position_type", ""))
	ThemeManager.apply_player_card_style(card, player_position)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", ThemeManager.SPACE_SM)
	card.add_child(vbox)

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", ThemeManager.SPACE_MD)
	margin.add_theme_constant_override("margin_right", ThemeManager.SPACE_MD)
	margin.add_theme_constant_override("margin_top", ThemeManager.SPACE_SM)
	margin.add_theme_constant_override("margin_bottom", ThemeManager.SPACE_SM)
	vbox.add_child(margin)

	var content = VBoxContainer.new()
	margin.add_child(content)

	# Player info header (player_data는 위에서 이미 가져옴)
	var position_name_ko = position.get("position_name_ko", "포지션")

	var name_label = Label.new()
	if player_data.size() > 0:
		name_label.text = (
			"%s (%s) - OVR %d" % [player_data.get("name", "Unknown"), position_name_ko, player_data.get("overall", 0)]
		)
		# OVR 색상 적용
		var overall = player_data.get("overall", 0)
		name_label.add_theme_color_override("font_color", ThemeManager.get_stat_color(overall))
	else:
		name_label.text = "슬롯 %d: %s (선수 없음)" % [slot + 1, position_name_ko]
		name_label.add_theme_color_override("font_color", ThemeManager.TEXT_SECONDARY)
	name_label.add_theme_font_size_override("font_size", ThemeManager.FONT_H3)
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(name_label)

	if player_data.size() == 0:
		return card

	# Role selector
	var role_hbox = HBoxContainer.new()
	content.add_child(role_hbox)

	var role_label = Label.new()
	role_label.text = "역할: "
	role_label.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)
	role_label.add_theme_color_override("font_color", ThemeManager.TEXT_SECONDARY)
	role_label.custom_minimum_size = Vector2(60, 0)
	role_hbox.add_child(role_label)

	var role_selector = OptionButton.new()
	role_selector.custom_minimum_size = Vector2(180, ThemeManager.TOUCH_MIN)
	role_selector.add_theme_font_size_override("font_size", ThemeManager.FONT_CAPTION)
	role_selector.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var rust_engine = get_node_or_null("/root/FootballRustEngine")
	if rust_engine and rust_engine.is_ready():
		var position_type = position.get("position_type", "")
		if position_type != "Goalkeeper":  # GK doesn't have roles
			# Convert long position_type to short API code
			var api_position_code = _convert_position_type_to_api_code(position_type)
			var available_roles_result = rust_engine.get_available_roles(api_position_code)
			if available_roles_result.get("success", false):
				var available_roles = available_roles_result.get("roles", [])
				role_selector.add_item("(역할 없음)")
				for role in available_roles:
					role_selector.add_item(role.get("name_ko", role.get("name_en", "Unknown")))
					role_selector.set_item_metadata(role_selector.get_item_count() - 1, role.get("role_id", ""))

				# Select current role
				var current_role = player_data.get("role", null)
				if current_role:
					for i in range(role_selector.get_item_count()):
						var role_id = role_selector.get_item_metadata(i)
						if role_id == current_role:
							role_selector.select(i)
							break

				role_selector.item_selected.connect(_on_role_changed.bind(slot))
			else:
				var error_msg = available_roles_result.get("error", "Unknown")
				print(
					(
						"[MyTeamScreen] API Error for slot %d: %s (position_type: %s -> api_code: %s)"
						% [slot, error_msg, position_type, api_position_code]
					)
				)
				role_selector.text = "API Error"
				role_selector.disabled = true
				role_selector.tooltip_text = "API Error: %s" % error_msg
		else:
			role_selector.text = "N/A (GK)"
			role_selector.disabled = true
	else:
		role_selector.text = "Engine Not Ready"
		role_selector.disabled = true

	role_hbox.add_child(role_selector)

	# Instructions button (ThemeManager 스타일 적용)
	var instructions_button = Button.new()
	instructions_button.text = "지시사항 설정"
	instructions_button.custom_minimum_size = Vector2(0, ThemeManager.TOUCH_MIN)
	ThemeManager.apply_button_style(instructions_button, ThemeManager.get_button_style("secondary"))
	instructions_button.pressed.connect(_on_instructions_button_pressed.bind(slot))
	content.add_child(instructions_button)

	# Show current instructions summary if any
	var instructions = player_data.get("instructions", {})
	if instructions.size() > 0:
		var instructions_summary = Label.new()
		var summary_text = ""
		var count = 0
		for key in instructions:
			if count > 0:
				summary_text += ", "
			summary_text += "%s: %s" % [key, instructions[key]]
			count += 1
			if count >= 2:  # Show max 2 items for compact display
				summary_text += "..."
				break
		instructions_summary.text = summary_text
		instructions_summary.add_theme_font_size_override("font_size", ThemeManager.FONT_MICRO)
		instructions_summary.add_theme_color_override("font_color", ThemeManager.INFO)
		instructions_summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		content.add_child(instructions_summary)

	return card


func _on_role_changed(index: int, slot: int):
	"""Handle role change for a player"""
	print("[MyTeamScreen] Role changed for slot %d, index %d" % [slot, index])

	var role_id = ""
	# Find the role selector for this slot
	if instructions_container and slot < instructions_container.get_child_count():
		var card = instructions_container.get_child(slot)
		if card.get_child_count() > 0:
			var vbox = card.get_child(0)
			if vbox.get_child_count() > 0:
				var margin = vbox.get_child(0)
				if margin.get_child_count() > 0:
					var content = margin.get_child(0)
					if content.get_child_count() > 1:
						var role_hbox = content.get_child(1)
						if role_hbox.get_child_count() > 1:
							var role_selector = role_hbox.get_child(1)
							if role_selector is OptionButton:
								role_id = role_selector.get_item_metadata(index)

	if index == 0 or role_id == null or role_id == "":
		# Index 0 = "(역할 없음)" or no metadata
		var player_data = MyTeamData.get_player_at_slot(slot)
		if player_data.size() > 0:
			player_data.erase("role")
			MyTeamData.update_player_at_slot(slot, player_data)
			print("[MyTeamScreen] Role cleared for slot %d" % slot)
			_load_instructions()  # Reload to show changes
	else:
		# Set role
		if MyTeamData.set_player_role(slot, role_id):
			print("[MyTeamScreen] Role set to %s for slot %d" % [role_id, slot])
			_load_instructions()  # Reload to show changes


func _on_instructions_button_pressed(slot: int):
	"""Open instructions screen for a player"""
	print("[MyTeamScreen] Instructions button pressed for slot %d" % slot)

	var player_data = MyTeamData.get_player_at_slot(slot)
	if player_data.size() == 0:
		_show_message("선수가 없습니다. 먼저 선수를 배치해주세요.")
		return

	# Create PlayerInstructionsScreen (class_name declared in script)
	var instructions_screen = PlayerInstructionsScreen.new()

	# Set fullscreen
	instructions_screen.set_anchors_preset(Control.PRESET_FULL_RECT)
	instructions_screen.anchor_right = 1.0
	instructions_screen.anchor_bottom = 1.0

	# Connect signals
	if instructions_screen.has_signal("instructions_applied"):
		instructions_screen.instructions_applied.connect(_on_instructions_applied.bind(slot))
	if instructions_screen.has_signal("screen_closed"):
		instructions_screen.screen_closed.connect(_on_instructions_screen_closed.bind(instructions_screen))

	# Add to scene tree first so _ready() runs and creates UI
	add_child(instructions_screen)

	# Load player data AFTER adding to scene tree
	if instructions_screen.has_method("load_player"):
		instructions_screen.load_player(player_data)

	print("[MyTeamScreen] PlayerInstructionsScreen opened for %s" % player_data.get("name", "Unknown"))


func _on_instructions_applied(player_data: Dictionary, instructions: Dictionary, slot: int):
	"""Handle instructions applied from PlayerInstructionsScreen"""
	print("[MyTeamScreen] Instructions applied for slot %d" % slot)
	print("[MyTeamScreen] Instructions: %s" % instructions)

	# Add instructions to player_data before updating (fix for player disappearing)
	player_data["instructions"] = instructions

	# Update player data at slot
	if MyTeamData.update_player_at_slot(slot, player_data):
		_load_instructions()  # Reload to show changes
		print("[MyTeamScreen] Instructions updated successfully")
	else:
		_show_message("지시사항 저장에 실패했습니다.")


func _on_instructions_screen_closed(instructions_screen: Control):
	"""Handle instructions screen closed"""
	print("[MyTeamScreen] Instructions screen closed")
	instructions_screen.queue_free()


# ===== Team Tactics Tab =====


func _add_team_tactics_tab():
	"""Add Team Instructions tab dynamically (Phase 5)"""
	if not tab_container:
		return

	# Check if tab already exists
	for i in range(tab_container.get_child_count()):
		var child = tab_container.get_child(i)
		if child.name == "TeamInstructions":
			print("[MyTeamScreen] Team Instructions tab already exists")
			return

	# Create TeamInstructionsPanel (Phase 5)
	var team_instructions_script = load("res://scripts/components/TeamInstructionsPanel.gd")
	if not team_instructions_script:
		print("[MyTeamScreen] Error: TeamInstructionsPanel script not found")
		return

	var team_instructions_panel = Control.new()
	team_instructions_panel.name = "TeamInstructions"
	team_instructions_panel.set_script(team_instructions_script)

	tab_container.add_child(team_instructions_panel)

	# Set tab name
	var tab_index = tab_container.get_tab_count() - 1
	tab_container.set_tab_title(tab_index, "팀 지시사항")

	print("[MyTeamScreen] Team Instructions tab added successfully (Phase 5)")


# ===== BM Two-Track: Tab Unlock System =====


func _get_unlocked_stage() -> int:
	"""Get current unlocked stage from StageManager"""
	var stage_manager = get_node_or_null("/root/StageManager")
	if stage_manager and "unlocked_stage" in stage_manager:
		return stage_manager.unlocked_stage
	return 0  # Default: no stages cleared


func _apply_tab_unlock_status():
	"""Apply tab lock/unlock based on stage progress (BM Two-Track)"""
	if not tab_container:
		return

	var unlocked = _get_unlocked_stage()
	print("[MyTeamScreen] Applying tab unlock status (Stage %d cleared)" % unlocked)

	# Tab indices: 0=Players, 1=Formation, 2=Instructions, 3=Statistics, 4=TeamInstructions
	for i in range(tab_container.get_tab_count()):
		var tab_name = tab_container.get_tab_title(i)
		var tab_node = tab_container.get_child(i)
		var required_stage = _get_required_stage_for_tab(tab_name)

		if required_stage > 0 and unlocked < required_stage:
			# Tab is locked
			_lock_tab(i, tab_node, tab_name, required_stage)
		else:
			# Tab is unlocked
			_unlock_tab(i, tab_node, tab_name)


func _get_required_stage_for_tab(tab_name: String) -> int:
	"""Get required stage to unlock a tab (0 = always unlocked)"""
	match tab_name:
		"Statistics":
			return UNLOCK_STATISTICS_STAGE
		"Instructions":
			return UNLOCK_INSTRUCTIONS_STAGE
		"팀 지시사항", "TeamInstructions":
			return UNLOCK_TACTICS_STAGE
		_:
			return 0  # Players, Formation = always unlocked


func _lock_tab(tab_idx: int, tab_node: Control, tab_name: String, required_stage: int):
	"""Lock a tab with visual indicator"""
	# Add lock icon to tab title
	var locked_title = "🔒 %s" % tab_name
	tab_container.set_tab_title(tab_idx, locked_title)
	tab_container.set_tab_disabled(tab_idx, true)

	# Add tooltip showing unlock requirement
	if tab_node:
		tab_node.tooltip_text = "Stage %d 클리어 시 해금" % required_stage

	print("[MyTeamScreen] Tab '%s' locked (requires Stage %d)" % [tab_name, required_stage])


func _unlock_tab(tab_idx: int, tab_node: Control, tab_name: String):
	"""Unlock a tab (remove lock indicator if present)"""
	# Remove lock icon if present
	var clean_name = tab_name.replace("🔒 ", "")
	tab_container.set_tab_title(tab_idx, clean_name)
	tab_container.set_tab_disabled(tab_idx, false)

	# Clear tooltip
	if tab_node:
		tab_node.tooltip_text = ""


func _on_tab_locked_clicked(tab_idx: int, required_stage: int):
	"""Show message when clicking a locked tab"""
	_show_message("이 기능은 Stage %d를 클리어하면 해금됩니다!\n\n스테이지 모드에서 진행해주세요." % required_stage)
