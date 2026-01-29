extends Node3D
class_name StadiumEnvironment

## Stadium environment controller for 3D match viewer.
## Manages stands, lighting, field markings, and atmosphere effects.

signal time_of_day_changed(time: TimeOfDay)
signal weather_changed(weather: WeatherType)

enum TimeOfDay {
	DAY,
	EVENING,
	NIGHT,
}

enum WeatherType {
	CLEAR,
	CLOUDY,
	RAINY,
}

## Configuration
@export var stadium_capacity: int = 30000  # Affects crowd density
@export var home_crowd_color: Color = Color(0.9, 0.1, 0.1, 1.0)
@export var away_crowd_color: Color = Color(0.1, 0.1, 0.9, 1.0)

## Current state
var _time_of_day: TimeOfDay = TimeOfDay.DAY
var _weather: WeatherType = WeatherType.CLEAR

## Field dimensions
const FIELD_LENGTH := 105.0
const FIELD_WIDTH := 68.0
const STAND_HEIGHT := 15.0
const STAND_DEPTH := 20.0

## Node references (created dynamically)
var _stands: Node3D = null
var _floodlights: Node3D = null
var _field_markings: Node3D = null
var _atmosphere: Node3D = null
var _world_environment: WorldEnvironment = null
var _sun_light: DirectionalLight3D = null


func _ready() -> void:
	_find_existing_nodes()
	_create_stadium_structure()
	_create_field_markings()
	_create_floodlights()
	_apply_time_of_day(_time_of_day)


func _find_existing_nodes() -> void:
	_world_environment = get_node_or_null("../WorldEnvironment") as WorldEnvironment
	_sun_light = get_node_or_null("../DirectionalLight3D") as DirectionalLight3D


## ============================================================================
## Stadium Structure
## ============================================================================


func _create_stadium_structure() -> void:
	_stands = Node3D.new()
	_stands.name = "Stands"
	add_child(_stands)

	# Create four sides of stands
	_create_stand_section(
		"NorthStand", Vector3(0, 0, -(FIELD_WIDTH / 2 + STAND_DEPTH / 2 + 5)), 0.0, FIELD_LENGTH + 20, true
	)
	_create_stand_section(
		"SouthStand", Vector3(0, 0, FIELD_WIDTH / 2 + STAND_DEPTH / 2 + 5), PI, FIELD_LENGTH + 20, true
	)
	_create_stand_section(
		"WestStand", Vector3(-(FIELD_LENGTH / 2 + STAND_DEPTH / 2 + 5), 0, 0), PI / 2, FIELD_WIDTH, false
	)
	_create_stand_section(
		"EastStand", Vector3(FIELD_LENGTH / 2 + STAND_DEPTH / 2 + 5, 0, 0), -PI / 2, FIELD_WIDTH, false
	)

	# Corner sections
	_create_corner_stand("NWCorner", Vector3(-FIELD_LENGTH / 2 - 10, 0, -FIELD_WIDTH / 2 - 10))
	_create_corner_stand("NECorner", Vector3(FIELD_LENGTH / 2 + 10, 0, -FIELD_WIDTH / 2 - 10))
	_create_corner_stand("SWCorner", Vector3(-FIELD_LENGTH / 2 - 10, 0, FIELD_WIDTH / 2 + 10))
	_create_corner_stand("SECorner", Vector3(FIELD_LENGTH / 2 + 10, 0, FIELD_WIDTH / 2 + 10))


func _create_stand_section(stand_name: String, pos: Vector3, rotation_y: float, width: float, is_main: bool) -> void:
	var stand := Node3D.new()
	stand.name = stand_name
	stand.position = pos
	stand.rotation.y = rotation_y
	_stands.add_child(stand)

	# Create tiered seating (3 tiers)
	for tier in range(3):
		var tier_mesh := MeshInstance3D.new()
		tier_mesh.name = "Tier%d" % tier

		var box := BoxMesh.new()
		box.size = Vector3(width, STAND_HEIGHT / 3.0, STAND_DEPTH / 3.0)
		tier_mesh.mesh = box

		# Position each tier higher and further back
		tier_mesh.position = Vector3(0, STAND_HEIGHT / 6.0 + tier * (STAND_HEIGHT / 3.0), -tier * (STAND_DEPTH / 3.0))

		# Material with crowd colors
		var mat := StandardMaterial3D.new()
		if is_main:
			# Home side (North) gets home colors, Away side (South) gets away colors
			if stand_name == "NorthStand":
				mat.albedo_color = home_crowd_color.lerp(Color(0.3, 0.3, 0.3), 0.3)
			else:
				mat.albedo_color = away_crowd_color.lerp(Color(0.3, 0.3, 0.3), 0.3)
		else:
			mat.albedo_color = Color(0.4, 0.4, 0.4, 1.0)  # Neutral sides
		tier_mesh.material_override = mat

		stand.add_child(tier_mesh)


func _create_corner_stand(corner_name: String, pos: Vector3) -> void:
	var corner := MeshInstance3D.new()
	corner.name = corner_name
	corner.position = pos

	var box := BoxMesh.new()
	box.size = Vector3(STAND_DEPTH, STAND_HEIGHT * 0.6, STAND_DEPTH)
	corner.mesh = box

	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.35, 0.35, 0.35, 1.0)
	corner.material_override = mat

	_stands.add_child(corner)


## ============================================================================
## Field Markings
## ============================================================================


func _create_field_markings() -> void:
	_field_markings = Node3D.new()
	_field_markings.name = "FieldMarkings"
	_field_markings.position.y = 0.06  # Slightly above field
	add_child(_field_markings)

	var line_color := Color.WHITE
	var line_width := 0.12

	# Outer boundary
	_create_line_rect(Vector3.ZERO, FIELD_LENGTH, FIELD_WIDTH, line_width, line_color)

	# Center line
	_create_line(Vector3(0, 0, -FIELD_WIDTH / 2), Vector3(0, 0, FIELD_WIDTH / 2), line_width, line_color)

	# Center circle
	_create_circle(Vector3.ZERO, 9.15, line_width, line_color)

	# Center spot
	_create_spot(Vector3.ZERO, 0.3, line_color)

	# Penalty areas (both ends)
	var penalty_width := 40.32
	var penalty_depth := 16.5
	_create_line_rect(
		Vector3(-FIELD_LENGTH / 2 + penalty_depth / 2, 0, 0), penalty_depth, penalty_width, line_width, line_color
	)
	_create_line_rect(
		Vector3(FIELD_LENGTH / 2 - penalty_depth / 2, 0, 0), penalty_depth, penalty_width, line_width, line_color
	)

	# Goal areas (6-yard box)
	var goal_area_width := 18.32
	var goal_area_depth := 5.5
	_create_line_rect(
		Vector3(-FIELD_LENGTH / 2 + goal_area_depth / 2, 0, 0), goal_area_depth, goal_area_width, line_width, line_color
	)
	_create_line_rect(
		Vector3(FIELD_LENGTH / 2 - goal_area_depth / 2, 0, 0), goal_area_depth, goal_area_width, line_width, line_color
	)

	# Penalty spots
	_create_spot(Vector3(-FIELD_LENGTH / 2 + 11, 0, 0), 0.22, line_color)
	_create_spot(Vector3(FIELD_LENGTH / 2 - 11, 0, 0), 0.22, line_color)

	# Penalty arcs
	_create_arc(Vector3(-FIELD_LENGTH / 2 + 11, 0, 0), 9.15, -PI / 3, PI / 3, line_width, line_color)
	_create_arc(Vector3(FIELD_LENGTH / 2 - 11, 0, 0), 9.15, PI * 2 / 3, PI * 4 / 3, line_width, line_color)

	# Corner arcs
	var corner_radius := 1.0
	_create_arc(Vector3(-FIELD_LENGTH / 2, 0, -FIELD_WIDTH / 2), corner_radius, 0, PI / 2, line_width, line_color)
	_create_arc(Vector3(FIELD_LENGTH / 2, 0, -FIELD_WIDTH / 2), corner_radius, PI / 2, PI, line_width, line_color)
	_create_arc(Vector3(-FIELD_LENGTH / 2, 0, FIELD_WIDTH / 2), corner_radius, -PI / 2, 0, line_width, line_color)
	_create_arc(Vector3(FIELD_LENGTH / 2, 0, FIELD_WIDTH / 2), corner_radius, PI, PI * 3 / 2, line_width, line_color)


func _create_line(start: Vector3, end: Vector3, width: float, color: Color) -> void:
	var line := MeshInstance3D.new()
	var direction := end - start
	var length := direction.length()
	var center := (start + end) / 2.0

	var box := BoxMesh.new()
	box.size = Vector3(width, 0.02, length)
	line.mesh = box

	line.position = center
	line.rotation.y = atan2(direction.x, direction.z)

	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	line.material_override = mat

	_field_markings.add_child(line)


func _create_line_rect(center: Vector3, width: float, height: float, line_width: float, color: Color) -> void:
	var hw := width / 2.0
	var hh := height / 2.0

	# Top line
	_create_line(center + Vector3(-hw, 0, -hh), center + Vector3(hw, 0, -hh), line_width, color)
	# Bottom line
	_create_line(center + Vector3(-hw, 0, hh), center + Vector3(hw, 0, hh), line_width, color)
	# Left line
	_create_line(center + Vector3(-hw, 0, -hh), center + Vector3(-hw, 0, hh), line_width, color)
	# Right line
	_create_line(center + Vector3(hw, 0, -hh), center + Vector3(hw, 0, hh), line_width, color)


func _create_circle(center: Vector3, radius: float, line_width: float, color: Color, segments: int = 32) -> void:
	for i in range(segments):
		var angle1 := float(i) / segments * TAU
		var angle2 := float(i + 1) / segments * TAU
		var p1 := center + Vector3(cos(angle1) * radius, 0, sin(angle1) * radius)
		var p2 := center + Vector3(cos(angle2) * radius, 0, sin(angle2) * radius)
		_create_line(p1, p2, line_width, color)


func _create_arc(
	center: Vector3,
	radius: float,
	start_angle: float,
	end_angle: float,
	line_width: float,
	color: Color,
	segments: int = 12
) -> void:
	var angle_range := end_angle - start_angle
	for i in range(segments):
		var angle1 := start_angle + float(i) / segments * angle_range
		var angle2 := start_angle + float(i + 1) / segments * angle_range
		var p1 := center + Vector3(cos(angle1) * radius, 0, sin(angle1) * radius)
		var p2 := center + Vector3(cos(angle2) * radius, 0, sin(angle2) * radius)
		_create_line(p1, p2, line_width, color)


func _create_spot(pos: Vector3, radius: float, color: Color) -> void:
	var spot := MeshInstance3D.new()
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = radius
	cylinder.bottom_radius = radius
	cylinder.height = 0.02
	spot.mesh = cylinder
	spot.position = pos

	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	spot.material_override = mat

	_field_markings.add_child(spot)


## ============================================================================
## Floodlights
## ============================================================================


func _create_floodlights() -> void:
	_floodlights = Node3D.new()
	_floodlights.name = "Floodlights"
	add_child(_floodlights)

	# Four corner floodlight towers
	var tower_positions := [
		Vector3(-FIELD_LENGTH / 2 - 15, 0, -FIELD_WIDTH / 2 - 15),
		Vector3(FIELD_LENGTH / 2 + 15, 0, -FIELD_WIDTH / 2 - 15),
		Vector3(-FIELD_LENGTH / 2 - 15, 0, FIELD_WIDTH / 2 + 15),
		Vector3(FIELD_LENGTH / 2 + 15, 0, FIELD_WIDTH / 2 + 15),
	]

	for i in range(tower_positions.size()):
		_create_floodlight_tower("Tower%d" % i, tower_positions[i])


func _create_floodlight_tower(tower_name: String, pos: Vector3) -> void:
	var tower := Node3D.new()
	tower.name = tower_name
	tower.position = pos
	_floodlights.add_child(tower)

	# Tower pole
	var pole := MeshInstance3D.new()
	pole.name = "Pole"
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = 0.5
	cylinder.bottom_radius = 0.8
	cylinder.height = 25.0
	pole.mesh = cylinder
	pole.position.y = 12.5

	var pole_mat := StandardMaterial3D.new()
	pole_mat.albedo_color = Color(0.6, 0.6, 0.6)
	pole_mat.metallic = 0.8
	pole.material_override = pole_mat
	tower.add_child(pole)

	# Light head
	var head := MeshInstance3D.new()
	head.name = "LightHead"
	var box := BoxMesh.new()
	box.size = Vector3(4, 2, 2)
	head.mesh = box
	head.position = Vector3(0, 26, 0)

	var head_mat := StandardMaterial3D.new()
	head_mat.albedo_color = Color(0.8, 0.8, 0.8)
	head.material_override = head_mat
	tower.add_child(head)

	# Spot light (for night games)
	var light := SpotLight3D.new()
	light.name = "SpotLight"
	light.position = Vector3(0, 25, 0)
	light.light_color = Color(1.0, 0.98, 0.9)  # Warm white
	light.light_energy = 0.0  # Off by default (day game)
	light.spot_range = 150.0
	light.spot_angle = 50.0
	light.shadow_enabled = true

	# Point towards center of field
	var direction := -pos.normalized()
	direction.y = -0.5
	light.look_at(light.global_position + direction)

	tower.add_child(light)


## ============================================================================
## Time of Day & Weather
## ============================================================================


func set_time_of_day(time: TimeOfDay) -> void:
	_time_of_day = time
	_apply_time_of_day(time)
	time_of_day_changed.emit(time)


func get_time_of_day() -> TimeOfDay:
	return _time_of_day


func _apply_time_of_day(time: TimeOfDay) -> void:
	match time:
		TimeOfDay.DAY:
			_set_day_lighting()
		TimeOfDay.EVENING:
			_set_evening_lighting()
		TimeOfDay.NIGHT:
			_set_night_lighting()


func _set_day_lighting() -> void:
	if _sun_light:
		_sun_light.light_energy = 1.2
		_sun_light.light_color = Color(1.0, 0.98, 0.95)
		_sun_light.rotation_degrees = Vector3(-45, 30, 0)

	if _world_environment and _world_environment.environment:
		var env := _world_environment.environment
		env.background_color = Color(0.5, 0.7, 0.9)
		env.ambient_light_energy = 0.4

	_set_floodlights_power(0.0)


func _set_evening_lighting() -> void:
	if _sun_light:
		_sun_light.light_energy = 0.8
		_sun_light.light_color = Color(1.0, 0.7, 0.4)  # Orange sunset
		_sun_light.rotation_degrees = Vector3(-15, 60, 0)

	if _world_environment and _world_environment.environment:
		var env := _world_environment.environment
		env.background_color = Color(0.8, 0.5, 0.3)  # Orange sky
		env.ambient_light_energy = 0.25

	_set_floodlights_power(0.5)


func _set_night_lighting() -> void:
	if _sun_light:
		_sun_light.light_energy = 0.1
		_sun_light.light_color = Color(0.3, 0.3, 0.5)  # Moonlight
		_sun_light.rotation_degrees = Vector3(-60, 0, 0)

	if _world_environment and _world_environment.environment:
		var env := _world_environment.environment
		env.background_color = Color(0.05, 0.05, 0.15)  # Dark blue night
		env.ambient_light_energy = 0.1

	_set_floodlights_power(1.5)


func _set_floodlights_power(power: float) -> void:
	if not _floodlights:
		return

	for tower in _floodlights.get_children():
		var light := tower.get_node_or_null("SpotLight") as SpotLight3D
		if light:
			light.light_energy = power


func set_weather(weather: WeatherType) -> void:
	_weather = weather
	_apply_weather(weather)
	weather_changed.emit(weather)


func get_weather() -> WeatherType:
	return _weather


func _apply_weather(weather: WeatherType) -> void:
	match weather:
		WeatherType.CLEAR:
			_set_clear_weather()
		WeatherType.CLOUDY:
			_set_cloudy_weather()
		WeatherType.RAINY:
			_set_rainy_weather()


func _set_clear_weather() -> void:
	if _sun_light:
		_sun_light.shadow_enabled = true
	# Remove any fog/particles


func _set_cloudy_weather() -> void:
	if _sun_light:
		_sun_light.light_energy *= 0.7
		_sun_light.shadow_enabled = false

	if _world_environment and _world_environment.environment:
		var env := _world_environment.environment
		env.background_color = env.background_color.lerp(Color(0.5, 0.5, 0.5), 0.3)


func _set_rainy_weather() -> void:
	if _sun_light:
		_sun_light.light_energy *= 0.5
		_sun_light.shadow_enabled = false

	if _world_environment and _world_environment.environment:
		var env := _world_environment.environment
		env.background_color = Color(0.3, 0.35, 0.4)
		env.ambient_light_energy *= 0.7

	# TODO: Add rain particle effect


## ============================================================================
## Crowd Effects
## ============================================================================


func set_crowd_colors(home_color: Color, away_color: Color) -> void:
	home_crowd_color = home_color
	away_crowd_color = away_color
	_update_stand_colors()


func _update_stand_colors() -> void:
	if not _stands:
		return

	var north_stand := _stands.get_node_or_null("NorthStand")
	var south_stand := _stands.get_node_or_null("SouthStand")

	if north_stand:
		for tier in north_stand.get_children():
			if tier is MeshInstance3D and tier.material_override:
				tier.material_override.albedo_color = home_crowd_color.lerp(Color(0.3, 0.3, 0.3), 0.3)

	if south_stand:
		for tier in south_stand.get_children():
			if tier is MeshInstance3D and tier.material_override:
				tier.material_override.albedo_color = away_crowd_color.lerp(Color(0.3, 0.3, 0.3), 0.3)
