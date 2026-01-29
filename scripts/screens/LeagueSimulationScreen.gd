extends Control

# League Simulation Screen
# Manages the league simulation UI and controls

@onready var tab_container = $MainContainer/TabContainer
@onready var back_button = $MainContainer/Header/HeaderContainer/BackButton
@onready var season_info = $MainContainer/Header/HeaderContainer/SeasonInfo

# League Table Tab
@onready var table_content = $MainContainer/TabContainer/LeagueTable/TableScroll/TableContent

# Fixtures Tab
@onready var fixtures_content = $MainContainer/TabContainer/Fixtures/FixturesScroll/FixturesContent

# Teams Tab
@onready var teams_content = $MainContainer/TabContainer/Teams/TeamsScroll/TeamsContent

# Simulation Tab
@onready
var simulate_week_button = $MainContainer/TabContainer/Simulation/SimulationControls/ControlButtons/SimulateWeekButton
@onready
var simulate_month_button = $MainContainer/TabContainer/Simulation/SimulationControls/ControlButtons/SimulateMonthButton
@onready
var simulate_season_button = $MainContainer/TabContainer/Simulation/SimulationControls/ControlButtons/SimulateSeasonButton
@onready
var week_info = $MainContainer/TabContainer/Simulation/SimulationControls/StatusPanel/StatusContainer/StatusInfo/WeekInfo
@onready
var season_info_detail = $MainContainer/TabContainer/Simulation/SimulationControls/StatusPanel/StatusContainer/StatusInfo/SeasonInfo
@onready
var teams_info = $MainContainer/TabContainer/Simulation/SimulationControls/StatusPanel/StatusContainer/StatusInfo/TeamsInfo
@onready
var players_info = $MainContainer/TabContainer/Simulation/SimulationControls/StatusPanel/StatusContainer/StatusInfo/PlayersInfo
@onready var refresh_button = $MainContainer/Footer/FooterContainer/RefreshButton

# League data
var league_data: Dictionary = {}
var current_week: int = 1
var current_season: int = 1
var teams: Array = []
var fixtures: Array = []

# Team names (16 academy teams)
const TEAM_NAMES = [
	"서울고등학교",
	"부산고등학교",
	"대구고등학교",
	"인천고등학교",
	"광주고등학교",
	"대전고등학교",
	"울산고등학교",
	"세종고등학교",
	"수원고등학교",
	"성남고등학교",
	"안양고등학교",
	"안산고등학교",
	"고양고등학교",
	"의정부고등학교",
	"용인고등학교",
	"화성고등학교"
]


func _ready():
	_connect_signals()
	_initialize_league()
	_update_ui()


func _connect_signals():
	back_button.pressed.connect(_on_back_button_pressed)
	simulate_week_button.pressed.connect(_on_simulate_week_pressed)
	simulate_month_button.pressed.connect(_on_simulate_month_pressed)
	simulate_season_button.pressed.connect(_on_simulate_season_pressed)
	refresh_button.pressed.connect(_on_refresh_pressed)


func _initialize_league():
	"""Initialize league with 16 teams and 288 players"""
	print("🏆 리그 초기화 시작...")

	# Initialize teams
	teams.clear()
	for i in range(16):
		var team = {
			"id": i,
			"name": TEAM_NAMES[i],
			"position": i + 1,
			"points": 0,
			"wins": 0,
			"draws": 0,
			"losses": 0,
			"goals_for": 0,
			"goals_against": 0,
			"goal_difference": 0,
			"players": _generate_team_players(i)
		}
		teams.append(team)

	# Generate fixtures (30 matches per season)
	_generate_fixtures()

	# Initialize league data
	league_data = {
		"season": current_season, "week": current_week, "total_weeks": 30, "teams": teams, "fixtures": fixtures
	}

	print("✅ 리그 초기화 완료: %d팀, %d명 선수" % [teams.size(), teams.size() * 18])


func _generate_team_players(team_id: int) -> Array:
	"""Generate 18 players for a team"""
	var players: Array = []

	# Generate 2 goalkeepers
	for i in range(2):
		var player = _generate_player("GK", team_id)
		players.append(player)

	# Generate 6 defenders
	for i in range(6):
		var player = _generate_player("DF", team_id)
		players.append(player)

	# Generate 6 midfielders
	for i in range(6):
		var player = _generate_player("MF", team_id)
		players.append(player)

	# Generate 4 forwards
	for i in range(4):
		var player = _generate_player("FW", team_id)
		players.append(player)

	return players


func _generate_player(position: String, team_id: int) -> Dictionary:
	"""Generate a single player with random stats"""
	var first_names = ["김", "이", "박", "최", "정", "강", "조", "윤", "장", "임"]
	var last_names = ["민수", "철수", "영희", "지훈", "현우", "서연", "민지", "준호", "예진", "태현"]

	var player = {
		"id": randi(),
		"name": first_names[randi() % first_names.size()] + last_names[randi() % last_names.size()],
		"position": position,
		"team_id": team_id,
		"age": randi_range(16, 18),
		"overall": randi_range(40, 85),
		"potential": randi_range(50, 95),
		"skills": _generate_random_skills(position)
	}
	return player


func _generate_random_skills(position: String) -> Dictionary:
	"""Generate random skills based on position"""
	var skills = {}

	# Base skills for all positions
	skills["pace"] = randi_range(40, 85)
	skills["shooting"] = randi_range(30, 80)
	skills["passing"] = randi_range(40, 85)
	skills["dribbling"] = randi_range(35, 80)
	skills["defending"] = randi_range(35, 85)
	skills["physical"] = randi_range(40, 85)

	# Position-specific bonuses
	match position:
		"GK":
			skills["goalkeeping"] = randi_range(60, 90)
			skills["reflexes"] = randi_range(50, 85)
			skills["handling"] = randi_range(45, 80)
		"DF":
			skills["defending"] = randi_range(60, 90)
			skills["tackling"] = randi_range(55, 85)
			skills["marking"] = randi_range(50, 80)
		"MF":
			skills["passing"] = randi_range(55, 90)
			skills["vision"] = randi_range(50, 85)
			skills["work_rate"] = randi_range(45, 80)
		"FW":
			skills["shooting"] = randi_range(60, 90)
			skills["finishing"] = randi_range(55, 85)
			skills["movement"] = randi_range(50, 80)

	return skills


func _generate_fixtures():
	"""Generate fixtures for the season (30 matches per team)"""
	fixtures.clear()

	# Simple round-robin system
	for week in range(1, 31):
		var week_fixtures: Array = []

		# Generate matches for this week
		for i in range(0, 16, 2):
			var home_team = teams[i]
			var away_team = teams[i + 1]

			var fixture = {
				"week": week,
				"home_team": home_team.name,
				"away_team": away_team.name,
				"home_team_id": home_team.id,
				"away_team_id": away_team.id,
				"home_score": -1,  # Not played yet
				"away_score": -1,
				"played": false
			}
			week_fixtures.append(fixture)

		fixtures.append(week_fixtures)


func _update_ui():
	"""Update all UI elements"""
	_update_header()
	_update_league_table()
	_update_fixtures()
	_update_teams()
	_update_simulation_status()


func _update_header():
	"""Update header information"""
	season_info.text = "시즌 %d - %d주차" % [current_season, current_week]


func _update_league_table():
	"""Update league table display"""
	# Clear existing content
	for child in table_content.get_children():
		child.queue_free()

	# Sort teams by points (descending)
	var sorted_teams = teams.duplicate()
	sorted_teams.sort_custom(func(a, b): return a.points > b.points)

	# Create table rows
	for i in range(sorted_teams.size()):
		var team = sorted_teams[i]
		var row = _create_table_row(team, i + 1)
		table_content.add_child(row)


func _create_table_row(team: Dictionary, position: int) -> Panel:
	"""Create a table row for a team"""
	var panel = Panel.new()
	var container = HBoxContainer.new()

	# Position
	var pos_label = Label.new()
	pos_label.text = str(position)
	pos_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	pos_label.custom_minimum_size.x = 30

	# Team name
	var team_label = Label.new()
	team_label.text = team.name
	team_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Points
	var pts_label = Label.new()
	pts_label.text = str(team.points)
	pts_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	pts_label.custom_minimum_size.x = 40

	# Wins, Draws, Losses
	var w_label = Label.new()
	w_label.text = str(team.wins)
	w_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	w_label.custom_minimum_size.x = 30

	var d_label = Label.new()
	d_label.text = str(team.draws)
	d_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	d_label.custom_minimum_size.x = 30

	var l_label = Label.new()
	l_label.text = str(team.losses)
	l_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	l_label.custom_minimum_size.x = 30

	# Goals
	var gf_label = Label.new()
	gf_label.text = str(team.goals_for)
	gf_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	gf_label.custom_minimum_size.x = 40

	var ga_label = Label.new()
	ga_label.text = str(team.goals_against)
	ga_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	ga_label.custom_minimum_size.x = 40

	var gd_label = Label.new()
	gd_label.text = str(team.goal_difference)
	gd_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	gd_label.custom_minimum_size.x = 40

	# Add to container
	container.add_child(pos_label)
	container.add_child(team_label)
	container.add_child(pts_label)
	container.add_child(w_label)
	container.add_child(d_label)
	container.add_child(l_label)
	container.add_child(gf_label)
	container.add_child(ga_label)
	container.add_child(gd_label)

	panel.add_child(container)
	return panel


func _update_fixtures():
	"""Update fixtures display"""
	# Clear existing content
	for child in fixtures_content.get_children():
		child.queue_free()

	# Show current week fixtures
	if current_week <= fixtures.size():
		var week_fixtures = fixtures[current_week - 1]
		for fixture in week_fixtures:
			var fixture_panel = _create_fixture_panel(fixture)
			fixtures_content.add_child(fixture_panel)


func _create_fixture_panel(fixture: Dictionary) -> Panel:
	"""Create a fixture display panel"""
	var panel = Panel.new()
	var container = HBoxContainer.new()

	var home_label = Label.new()
	home_label.text = fixture.home_team
	home_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	home_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT

	var vs_label = Label.new()
	if fixture.played:
		vs_label.text = "%d - %d" % [fixture.home_score, fixture.away_score]
	else:
		vs_label.text = "vs"
	vs_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vs_label.custom_minimum_size.x = 80

	var away_label = Label.new()
	away_label.text = fixture.away_team
	away_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	container.add_child(home_label)
	container.add_child(vs_label)
	container.add_child(away_label)

	panel.add_child(container)
	return panel


func _update_teams():
	"""Update teams display"""
	# Clear existing content
	for child in teams_content.get_children():
		child.queue_free()

	# Show team information
	for team in teams:
		var team_panel = _create_team_panel(team)
		teams_content.add_child(team_panel)


func _create_team_panel(team: Dictionary) -> Panel:
	"""Create a team information panel"""
	var panel = Panel.new()
	var container = VBoxContainer.new()

	var header = HBoxContainer.new()
	var name_label = Label.new()
	name_label.text = team.name
	name_label.add_theme_font_size_override("font_size", 16)

	var points_label = Label.new()
	points_label.text = "%d점" % team.points
	points_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT

	header.add_child(name_label)
	header.add_child(points_label)

	var players_label = Label.new()
	players_label.text = "선수 %d명" % team.players.size()

	container.add_child(header)
	container.add_child(players_label)
	panel.add_child(container)

	return panel


func _update_simulation_status():
	"""Update simulation status information"""
	week_info.text = "현재 주차: %d주차" % current_week
	season_info_detail.text = "시즌: %d시즌" % current_season
	teams_info.text = "참가 팀: %d팀" % teams.size()
	players_info.text = "총 선수: %d명" % (teams.size() * 18)


func _on_back_button_pressed():
	"""Return to main menu"""
	get_tree().change_scene_to_file("res://scenes/TitleScreenImproved.tscn")


func _on_simulate_week_pressed():
	"""Simulate one week of matches"""
	print("📅 1주 시뮬레이션 시작...")
	_simulate_week()
	current_week += 1
	_update_ui()
	print("✅ 1주 시뮬레이션 완료")


func _on_simulate_month_pressed():
	"""Simulate one month (4 weeks) of matches"""
	print("📆 1개월 시뮬레이션 시작...")
	for i in range(4):
		if current_week <= 30:
			_simulate_week()
			current_week += 1
	_update_ui()
	print("✅ 1개월 시뮬레이션 완료")


func _on_simulate_season_pressed():
	"""Simulate the entire season"""
	print("🏆 시즌 완료 시뮬레이션 시작...")
	while current_week <= 30:
		_simulate_week()
		current_week += 1

	# Season completed
	current_season += 1
	current_week = 1
	_initialize_league()  # Reset for new season
	_update_ui()
	print("✅ 시즌 완료! 새로운 시즌이 시작됩니다.")


func _on_refresh_pressed():
	"""Refresh the display"""
	_update_ui()
	print("🔄 화면 새로고침 완료")


func _simulate_week():
	"""Simulate matches for the current week"""
	if current_week > fixtures.size():
		return

	var week_fixtures = fixtures[current_week - 1]

	for fixture in week_fixtures:
		if not fixture.played:
			_simulate_match(fixture)


func _simulate_match(fixture: Dictionary):
	"""Simulate a single match"""
	var home_team = teams[fixture.home_team_id]
	var away_team = teams[fixture.away_team_id]

	# Simple match simulation
	var home_score = randi_range(0, 4)
	var away_score = randi_range(0, 4)

	# Update fixture
	fixture.home_score = home_score
	fixture.away_score = away_score
	fixture.played = true

	# Update team stats
	home_team.goals_for += home_score
	home_team.goals_against += away_score
	home_team.goal_difference = home_team.goals_for - home_team.goals_against

	away_team.goals_for += away_score
	away_team.goals_against += home_score
	away_team.goal_difference = away_team.goals_for - away_team.goals_against

	# Update points and record
	if home_score > away_score:
		home_team.wins += 1
		home_team.points += 3
		away_team.losses += 1
	elif home_score < away_score:
		away_team.wins += 1
		away_team.points += 3
		home_team.losses += 1
	else:
		home_team.draws += 1
		away_team.draws += 1
		home_team.points += 1
		away_team.points += 1

	print("⚽ %s %d - %d %s" % [home_team.name, home_score, away_score, away_team.name])
