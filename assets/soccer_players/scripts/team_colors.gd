class_name TeamColors
extends Resource

## Preset team colors for soccer uniforms

# Common team colors (RGB normalized 0-1)
const TEAMS = {
    # Red teams
    "red": Color(0.9, 0.1, 0.1),
    "manchester_united": Color(0.85, 0.1, 0.1),
    "liverpool": Color(0.78, 0.06, 0.18),
    "arsenal": Color(0.94, 0.13, 0.15),

    # Blue teams
    "blue": Color(0.1, 0.3, 0.8),
    "chelsea": Color(0.02, 0.14, 0.53),
    "manchester_city": Color(0.42, 0.68, 0.84),
    "barcelona": Color(0.65, 0.0, 0.13),  # Blaugrana blue-red

    # Green teams
    "green": Color(0.1, 0.6, 0.2),

    # Yellow teams
    "yellow": Color(0.95, 0.85, 0.1),
    "brazil": Color(1.0, 0.87, 0.0),
    "dortmund": Color(1.0, 0.87, 0.0),

    # White teams
    "white": Color(0.95, 0.95, 0.95),
    "real_madrid": Color(1.0, 1.0, 1.0),

    # Black teams
    "black": Color(0.1, 0.1, 0.1),
    "juventus": Color(0.0, 0.0, 0.0),

    # Orange
    "orange": Color(1.0, 0.5, 0.0),
    "netherlands": Color(1.0, 0.47, 0.0),

    # Purple
    "purple": Color(0.5, 0.0, 0.5),

    # Pink
    "pink": Color(1.0, 0.4, 0.6),
}

## Apply team color to a MeshInstance3D
static func apply_team_color(mesh_instance: MeshInstance3D, team_name: String) -> void:
    if not TEAMS.has(team_name):
        push_warning("Unknown team: " + team_name)
        return

    var color = TEAMS[team_name]
    _apply_color_to_mesh(mesh_instance, color)

## Apply custom color to a MeshInstance3D
static func apply_custom_color(mesh_instance: MeshInstance3D, color: Color) -> void:
    _apply_color_to_mesh(mesh_instance, color)

## Internal: Apply color using shader
static func _apply_color_to_mesh(mesh_instance: MeshInstance3D, color: Color) -> void:
    var shader = load("res://assets/soccer_players/shaders/team_uniform_simple.gdshader")

    # Get or create material override
    var mat = mesh_instance.get_surface_override_material(0)
    if mat == null or not mat is ShaderMaterial:
        mat = ShaderMaterial.new()
        mat.shader = shader
        mesh_instance.set_surface_override_material(0, mat)

    # Get original texture from mesh
    var mesh = mesh_instance.mesh
    if mesh and mesh.surface_get_material(0):
        var original_mat = mesh.surface_get_material(0)
        if original_mat is StandardMaterial3D:
            mat.set_shader_parameter("albedo_texture", original_mat.albedo_texture)

    # Set team color
    mat.set_shader_parameter("team_color", Vector3(color.r, color.g, color.b))

## Get list of available team names
static func get_team_names() -> Array:
    return TEAMS.keys()

## Get color for team name
static func get_team_color(team_name: String) -> Color:
    if TEAMS.has(team_name):
        return TEAMS[team_name]
    return Color.WHITE
