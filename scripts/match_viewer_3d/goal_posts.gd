extends Node3D
class_name GoalPosts

## Creates goal post structures for both ends of the field.

const FIELD_LENGTH := 105.0
const GOAL_WIDTH := 7.32
const GOAL_HEIGHT := 2.44
const POST_RADIUS := 0.06
const NET_COLOR := Color(1.0, 1.0, 1.0, 0.5)

@export var post_color: Color = Color.WHITE


func _ready() -> void:
	_create_goal("HomeGoal", Vector3(-FIELD_LENGTH / 2, 0, 0), false)
	_create_goal("AwayGoal", Vector3(FIELD_LENGTH / 2, 0, 0), true)


func _create_goal(goal_name: String, pos: Vector3, facing_left: bool) -> void:
	var goal := Node3D.new()
	goal.name = goal_name
	goal.position = pos
	add_child(goal)

	var net_offset := 1.5 if facing_left else -1.5

	# Left post
	_create_post(goal, "LeftPost", Vector3(0, GOAL_HEIGHT / 2, -GOAL_WIDTH / 2), GOAL_HEIGHT)

	# Right post
	_create_post(goal, "RightPost", Vector3(0, GOAL_HEIGHT / 2, GOAL_WIDTH / 2), GOAL_HEIGHT)

	# Crossbar
	_create_crossbar(goal, "Crossbar", Vector3(0, GOAL_HEIGHT, 0), GOAL_WIDTH)

	# Net back posts
	_create_post(goal, "BackLeftPost", Vector3(net_offset, GOAL_HEIGHT / 2, -GOAL_WIDTH / 2), GOAL_HEIGHT)
	_create_post(goal, "BackRightPost", Vector3(net_offset, GOAL_HEIGHT / 2, GOAL_WIDTH / 2), GOAL_HEIGHT)

	# Net crossbar (back)
	_create_crossbar(goal, "BackCrossbar", Vector3(net_offset, GOAL_HEIGHT, 0), GOAL_WIDTH)

	# Net side bars (connecting front to back)
	_create_side_bar(goal, "LeftSideTop", Vector3(net_offset / 2, GOAL_HEIGHT, -GOAL_WIDTH / 2), abs(net_offset))
	_create_side_bar(goal, "RightSideTop", Vector3(net_offset / 2, GOAL_HEIGHT, GOAL_WIDTH / 2), abs(net_offset))

	# Net mesh (simplified as semi-transparent planes)
	_create_net_panel(goal, "BackNet", Vector3(net_offset, GOAL_HEIGHT / 2, 0), Vector3(0.05, GOAL_HEIGHT, GOAL_WIDTH))
	_create_net_panel(
		goal,
		"LeftSideNet",
		Vector3(net_offset / 2, GOAL_HEIGHT / 2, -GOAL_WIDTH / 2),
		Vector3(abs(net_offset), GOAL_HEIGHT, 0.05)
	)
	_create_net_panel(
		goal,
		"RightSideNet",
		Vector3(net_offset / 2, GOAL_HEIGHT / 2, GOAL_WIDTH / 2),
		Vector3(abs(net_offset), GOAL_HEIGHT, 0.05)
	)
	_create_net_panel(
		goal, "TopNet", Vector3(net_offset / 2, GOAL_HEIGHT, 0), Vector3(abs(net_offset), 0.05, GOAL_WIDTH)
	)


func _create_post(parent: Node3D, post_name: String, pos: Vector3, height: float) -> void:
	var post := MeshInstance3D.new()
	post.name = post_name
	post.position = pos

	var cylinder := CylinderMesh.new()
	cylinder.top_radius = POST_RADIUS
	cylinder.bottom_radius = POST_RADIUS
	cylinder.height = height
	post.mesh = cylinder

	var mat := StandardMaterial3D.new()
	mat.albedo_color = post_color
	mat.metallic = 0.3
	post.material_override = mat

	parent.add_child(post)


func _create_crossbar(parent: Node3D, bar_name: String, pos: Vector3, width: float) -> void:
	var bar := MeshInstance3D.new()
	bar.name = bar_name
	bar.position = pos
	bar.rotation.x = PI / 2

	var cylinder := CylinderMesh.new()
	cylinder.top_radius = POST_RADIUS
	cylinder.bottom_radius = POST_RADIUS
	cylinder.height = width
	bar.mesh = cylinder

	var mat := StandardMaterial3D.new()
	mat.albedo_color = post_color
	mat.metallic = 0.3
	bar.material_override = mat

	parent.add_child(bar)


func _create_side_bar(parent: Node3D, bar_name: String, pos: Vector3, length: float) -> void:
	var bar := MeshInstance3D.new()
	bar.name = bar_name
	bar.position = pos
	bar.rotation.z = PI / 2

	var cylinder := CylinderMesh.new()
	cylinder.top_radius = POST_RADIUS * 0.5
	cylinder.bottom_radius = POST_RADIUS * 0.5
	cylinder.height = length
	bar.mesh = cylinder

	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.7, 0.7, 0.7)
	bar.material_override = mat

	parent.add_child(bar)


func _create_net_panel(parent: Node3D, panel_name: String, pos: Vector3, size: Vector3) -> void:
	var panel := MeshInstance3D.new()
	panel.name = panel_name
	panel.position = pos

	var box := BoxMesh.new()
	box.size = size
	panel.mesh = box

	var mat := StandardMaterial3D.new()
	mat.albedo_color = NET_COLOR
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	panel.material_override = mat

	parent.add_child(panel)
