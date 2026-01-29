extends Node

# Optimized Data Structures
# Provides efficient data structures for better performance

class_name OptimizedDataStructures


# Optimized player data structure
class OptimizedPlayerData:
	var id: int
	var name: String
	var position: String
	var skills: PackedFloat32Array  # More memory efficient than Dictionary
	var stats: PackedFloat32Array  # Overall, potential, age, etc.
	var condition: int
	var fatigue: float
	var injury_days: int

	func _init(player_id: int, player_name: String, player_position: String):
		id = player_id
		name = player_name
		position = player_position
		skills = PackedFloat32Array()
		stats = PackedFloat32Array()
		condition = 3
		fatigue = 0.0
		injury_days = 0

		# Initialize skills array (42 skills)
		skills.resize(42)
		for i in range(42):
			skills[i] = 50.0  # Default value

		# Initialize stats array (overall, potential, age, etc.)
		stats.resize(10)
		stats[0] = 50.0  # Overall
		stats[1] = 70.0  # Potential
		stats[2] = 17.0  # Age
		stats[3] = 0.0  # Morale
		stats[4] = 0.0  # Chemistry
		stats[5] = 0.0  # Cohesion
		stats[6] = 0.0  # Leadership
		stats[7] = 0.0  # Determination
		stats[8] = 0.0  # Concentration
		stats[9] = 0.0  # Work Rate

	func get_skill(skill_index: int) -> float:
		if skill_index >= 0 and skill_index < skills.size():
			return skills[skill_index]
		return 0.0

	func set_skill(skill_index: int, value: float):
		if skill_index >= 0 and skill_index < skills.size():
			skills[skill_index] = clampf(value, 0.0, 100.0)

	func get_overall() -> float:
		return stats[0]

	func set_overall(value: float):
		stats[0] = clampf(value, 0.0, 100.0)

	func get_potential() -> float:
		return stats[1]

	func set_potential(value: float):
		stats[1] = clampf(value, 0.0, 100.0)


# Optimized team data structure
class OptimizedTeamData:
	var id: int
	var name: String
	var players: Array
	var stats: PackedInt32Array  # Points, wins, draws, losses, etc.

	func _init(team_id: int, team_name: String):
		id = team_id
		name = team_name
		players = []
		stats = PackedInt32Array()
		stats.resize(8)  # Points, wins, draws, losses, goals_for, goals_against, goal_difference, position

		# Initialize stats
		for i in range(8):
			stats[i] = 0

	func add_player(player: OptimizedPlayerData):
		players.append(player)

	func get_points() -> int:
		return stats[0]

	func set_points(value: int):
		stats[0] = value

	func get_wins() -> int:
		return stats[1]

	func set_wins(value: int):
		stats[1] = value

	func get_draws() -> int:
		return stats[2]

	func set_draws(value: int):
		stats[2] = value

	func get_losses() -> int:
		return stats[3]

	func set_losses(value: int):
		stats[3] = value

	func get_goals_for() -> int:
		return stats[4]

	func set_goals_for(value: int):
		stats[4] = value

	func get_goals_against() -> int:
		return stats[5]

	func set_goals_against(value: int):
		stats[5] = value

	func get_goal_difference() -> int:
		return stats[6]

	func set_goal_difference(value: int):
		stats[6] = value

	func get_position() -> int:
		return stats[7]

	func set_position(value: int):
		stats[7] = value


# Optimized league data structure
class OptimizedLeagueData:
	var teams: Array
	var fixtures: Array  # More efficient than Dictionary arrays
	var current_week: int
	var current_season: int
	var total_weeks: int

	func _init():
		teams = []
		fixtures = []
		current_week = 1
		current_season = 1
		total_weeks = 30

	func add_team(team: OptimizedTeamData):
		teams.append(team)

	func generate_fixtures():
		"""Generate fixtures using optimized data structure"""
		fixtures.clear()

		for week in range(1, total_weeks + 1):
			var week_fixtures = PackedInt32Array()

			# Generate matches for this week
			for i in range(0, teams.size(), 2):
				week_fixtures.append(teams[i].id)  # Home team
				week_fixtures.append(teams[i + 1].id)  # Away team
				week_fixtures.append(-1)  # Home score (not played)
				week_fixtures.append(-1)  # Away score (not played)
				week_fixtures.append(0)  # Played flag (0 = not played)

			fixtures.append(week_fixtures)

	func simulate_week():
		"""Simulate matches for current week"""
		if current_week > fixtures.size():
			return

		var week_fixtures = fixtures[current_week - 1]

		# Process matches in pairs (home_id, away_id, home_score, away_score, played)
		for i in range(0, week_fixtures.size(), 5):
			var home_id = week_fixtures[i]
			var away_id = week_fixtures[i + 1]
			var home_score = week_fixtures[i + 2]
			var away_score = week_fixtures[i + 3]
			var played = week_fixtures[i + 4]

			if played == 0:  # Not played yet
				_simulate_match(home_id, away_id, i)

	func _simulate_match(home_id: int, away_id: int, fixture_index: int):
		"""Simulate a single match"""
		var home_team = _get_team_by_id(home_id)
		var away_team = _get_team_by_id(away_id)

		if not home_team or not away_team:
			return

		# Simple match simulation
		var home_score = randi_range(0, 4)
		var away_score = randi_range(0, 4)

		# Update fixture
		var week_fixtures = fixtures[current_week - 1]
		week_fixtures[fixture_index + 2] = home_score
		week_fixtures[fixture_index + 3] = away_score
		week_fixtures[fixture_index + 4] = 1  # Mark as played

		# Update team stats
		home_team.set_goals_for(home_team.get_goals_for() + home_score)
		home_team.set_goals_against(home_team.get_goals_against() + away_score)
		home_team.set_goal_difference(home_team.get_goals_for() - home_team.get_goals_against())

		away_team.set_goals_for(away_team.get_goals_for() + away_score)
		away_team.set_goals_against(away_team.get_goals_against() + home_score)
		away_team.set_goal_difference(away_team.get_goals_for() - away_team.get_goals_against())

		# Update points and record
		if home_score > away_score:
			home_team.set_wins(home_team.get_wins() + 1)
			home_team.set_points(home_team.get_points() + 3)
			away_team.set_losses(away_team.get_losses() + 1)
		elif home_score < away_score:
			away_team.set_wins(away_team.get_wins() + 1)
			away_team.set_points(away_team.get_points() + 3)
			home_team.set_losses(home_team.get_losses() + 1)
		else:
			home_team.set_draws(home_team.get_draws() + 1)
			away_team.set_draws(away_team.get_draws() + 1)
			home_team.set_points(home_team.get_points() + 1)
			away_team.set_points(away_team.get_points() + 1)

	func _get_team_by_id(team_id: int) -> OptimizedTeamData:
		"""Get team by ID"""
		for team in teams:
			if team.id == team_id:
				return team
		return null

	func sort_teams_by_points():
		"""Sort teams by points (descending)"""
		teams.sort_custom(func(a, b): return a.get_points() > b.get_points())

		# Update positions
		for i in range(teams.size()):
			teams[i].set_position(i + 1)


# Memory pool for frequently created objects
class ObjectPool:
	var available_objects: Array = []
	var object_type: String

	func _init(type_name: String):
		object_type = type_name

	func get_object():
		"""Get object from pool or create new one"""
		if available_objects.size() > 0:
			return available_objects.pop_back()
		else:
			return _create_new_object()

	func return_object(obj):
		"""Return object to pool"""
		_reset_object(obj)
		available_objects.append(obj)

	func _create_new_object():
		"""Create new object based on type"""
		match object_type:
			"OptimizedPlayerData":
				return OptimizedPlayerData.new(0, "", "")
			"OptimizedTeamData":
				return OptimizedTeamData.new(0, "")
			_:
				return null

	func _reset_object(obj):
		"""Reset object to default state"""
		if obj is OptimizedPlayerData:
			obj.fatigue = 0.0
			obj.injury_days = 0
			obj.condition = 3
		elif obj is OptimizedTeamData:
			for i in range(obj.stats.size()):
				obj.stats[i] = 0


# Global object pools
var player_pool: ObjectPool
var team_pool: ObjectPool


func _ready():
	# Initialize object pools
	player_pool = ObjectPool.new("OptimizedPlayerData")
	team_pool = ObjectPool.new("OptimizedTeamData")


# Utility functions for data conversion
func convert_to_optimized_player_data(player_data: Dictionary) -> OptimizedPlayerData:
	"""Convert Dictionary player data to OptimizedPlayerData"""
	var optimized = player_pool.get_object()

	optimized.id = player_data.get("id", 0)
	optimized.name = player_data.get("name", "")
	optimized.position = player_data.get("position", "")

	# Convert skills
	var skills_dict = player_data.get("skills", {})
	var skill_index = 0
	for skill_name in skills_dict:
		if skill_index < optimized.skills.size():
			optimized.skills[skill_index] = skills_dict[skill_name]
			skill_index += 1

	# Convert stats
	optimized.stats[0] = player_data.get("overall", 50.0)
	optimized.stats[1] = player_data.get("potential", 70.0)
	optimized.stats[2] = player_data.get("age", 17.0)
	optimized.condition = player_data.get("condition", 3)
	optimized.fatigue = player_data.get("fatigue", 0.0)
	optimized.injury_days = player_data.get("injury_days", 0)

	return optimized


func convert_to_dictionary(optimized_player: OptimizedPlayerData) -> Dictionary:
	"""Convert OptimizedPlayerData to Dictionary"""
	var dict = {
		"id": optimized_player.id,
		"name": optimized_player.name,
		"position": optimized_player.position,
		"overall": optimized_player.get_overall(),
		"potential": optimized_player.get_potential(),
		"age": optimized_player.stats[2],
		"condition": optimized_player.condition,
		"fatigue": optimized_player.fatigue,
		"injury_days": optimized_player.injury_days,
		"skills": {}
	}

	# Convert skills back to dictionary
	var skill_names = [
		"pace",
		"shooting",
		"passing",
		"dribbling",
		"defending",
		"physical",
		"goalkeeping",
		"reflexes",
		"handling",
		"kicking",
		"positioning",
		"tackling",
		"marking",
		"intercepting",
		"heading",
		"jumping",
		"stamina",
		"strength",
		"agility",
		"balance",
		"natural_fitness",
		"work_rate",
		"determination",
		"leadership",
		"concentration",
		"decision_making",
		"vision",
		"composure",
		"technique",
		"first_touch",
		"finishing",
		"movement",
		"crossing",
		"free_kicks",
		"penalties",
		"long_shots",
		"volleys",
		"curve",
		"finesse",
		"power",
		"accuracy"
	]

	for i in range(min(optimized_player.skills.size(), skill_names.size())):
		dict.skills[skill_names[i]] = optimized_player.skills[i]

	return dict
