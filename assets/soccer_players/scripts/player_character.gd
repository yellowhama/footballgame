extends Node3D
class_name PlayerCharacter

## Soccer player character with team color support

@export var team_color: Color = Color.RED:
    set(value):
        team_color = value
        _update_team_color()

@export var jersey_number: int = 10:
    set(value):
        jersey_number = value
        _update_jersey_number()

@onready var mesh_instance: MeshInstance3D = $MeshInstance3D  # Adjust path as needed

var _shader_material: ShaderMaterial

func _ready() -> void:
    _setup_shader()
    _update_team_color()

func _setup_shader() -> void:
    if not mesh_instance:
        # Try to find MeshInstance3D child
        for child in get_children():
            if child is MeshInstance3D:
                mesh_instance = child
                break

    if not mesh_instance:
        push_warning("No MeshInstance3D found in PlayerCharacter")
        return

    # Load and apply shader
    var shader = load("res://assets/soccer_players/shaders/team_uniform_simple.gdshader")
    _shader_material = ShaderMaterial.new()
    _shader_material.shader = shader

    # Copy original texture
    var original_mat = mesh_instance.mesh.surface_get_material(0)
    if original_mat is StandardMaterial3D and original_mat.albedo_texture:
        _shader_material.set_shader_parameter("albedo_texture", original_mat.albedo_texture)

    mesh_instance.set_surface_override_material(0, _shader_material)

func _update_team_color() -> void:
    if _shader_material:
        _shader_material.set_shader_parameter("team_color", Vector3(team_color.r, team_color.g, team_color.b))

func _update_jersey_number() -> void:
    # Jersey number can be handled via Decal3D or Label3D
    # This is a placeholder for number display logic
    pass

## Set team by preset name
func set_team(team_name: String) -> void:
    team_color = TeamColors.get_team_color(team_name)

## Quick team color setters
func set_red_team() -> void:
    team_color = Color(0.9, 0.1, 0.1)

func set_blue_team() -> void:
    team_color = Color(0.1, 0.3, 0.8)

func set_green_team() -> void:
    team_color = Color(0.1, 0.6, 0.2)

func set_yellow_team() -> void:
    team_color = Color(0.95, 0.85, 0.1)
