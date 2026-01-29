extends RefCounted
class_name PlayerAppearance

## Player appearance customization data.
## Stores visual properties for 3D player rendering.

## Player role (affects appearance)
enum PlayerRole {
	OUTFIELD,
	GOALKEEPER,
}

## Body type presets
enum BodyType {
	AVERAGE,
	TALL,
	SHORT,
	MUSCULAR,
	LEAN,
}

## Basic info
var player_id: int = 0
var team_id: int = 0  # 0 = home, 1 = away
var role: PlayerRole = PlayerRole.OUTFIELD

## Jersey
var jersey_number: int = 0
var jersey_name: String = ""

## Colors (set from team)
var shirt_color: Color = Color.WHITE
var shorts_color: Color = Color.WHITE
var socks_color: Color = Color.WHITE

## Goalkeeper specific
var gk_shirt_color: Color = Color(0.2, 0.8, 0.2)  # Green
var gk_shorts_color: Color = Color.BLACK
var gk_gloves_color: Color = Color.WHITE

## Body customization
var body_type: BodyType = BodyType.AVERAGE
var height_scale: float = 1.0  # 0.9 - 1.1 typical range
var body_scale: Vector3 = Vector3.ONE

## Skin/hair (for future use)
var skin_tone: int = 0  # 0-5 preset
var hair_style: int = 0
var hair_color: Color = Color.BLACK


static func from_roster_entry(entry: Dictionary, team_id_val: int) -> PlayerAppearance:
	var appearance := PlayerAppearance.new()

	appearance.team_id = team_id_val

	# Extract player ID
	var id_keys := ["player_id", "id", "engine_id", "index"]
	for key in id_keys:
		if entry.has(key):
			appearance.player_id = int(entry.get(key, 0))
			break

	# Jersey number
	var number_keys := ["number", "kit_number", "jersey", "shirt_number"]
	for key in number_keys:
		if entry.has(key):
			appearance.jersey_number = int(entry.get(key, 0))
			break

	# Jersey name
	var name_keys := ["short_name", "display_name", "name", "surname"]
	for key in name_keys:
		if entry.has(key):
			appearance.jersey_name = str(entry.get(key, "")).to_upper()
			if appearance.jersey_name.length() > 12:
				appearance.jersey_name = appearance.jersey_name.substr(0, 12)
			break

	# Role detection
	var position := str(entry.get("position", entry.get("pos", ""))).to_upper()
	if position in ["GK", "GOALKEEPER", "G"]:
		appearance.role = PlayerRole.GOALKEEPER

	# Height (if available)
	if entry.has("height") or entry.has("height_cm"):
		var height_cm := float(entry.get("height", entry.get("height_cm", 180)))
		# Scale relative to average (180cm)
		appearance.height_scale = height_cm / 180.0
		appearance.height_scale = clamp(appearance.height_scale, 0.85, 1.15)

	# Body type from attributes
	if entry.has("body_type"):
		var bt := str(entry.get("body_type", "")).to_lower()
		match bt:
			"tall":
				appearance.body_type = BodyType.TALL
			"short":
				appearance.body_type = BodyType.SHORT
			"muscular", "strong":
				appearance.body_type = BodyType.MUSCULAR
			"lean", "slim":
				appearance.body_type = BodyType.LEAN
			_:
				appearance.body_type = BodyType.AVERAGE

	appearance._calculate_body_scale()
	return appearance


func _calculate_body_scale() -> void:
	body_scale = Vector3.ONE

	# Apply height
	body_scale.y = height_scale

	# Apply body type modifiers
	match body_type:
		BodyType.TALL:
			body_scale.y *= 1.05
			body_scale.x *= 0.95
			body_scale.z *= 0.95
		BodyType.SHORT:
			body_scale.y *= 0.92
			body_scale.x *= 1.02
		BodyType.MUSCULAR:
			body_scale.x *= 1.1
			body_scale.z *= 1.1
		BodyType.LEAN:
			body_scale.x *= 0.9
			body_scale.z *= 0.9


func set_team_colors(shirt: Color, shorts: Color, socks: Color = Color.WHITE) -> void:
	shirt_color = shirt
	shorts_color = shorts
	socks_color = socks


func set_goalkeeper_colors(shirt: Color, shorts: Color = Color.BLACK, gloves: Color = Color.WHITE) -> void:
	gk_shirt_color = shirt
	gk_shorts_color = shorts
	gk_gloves_color = gloves


func get_active_shirt_color() -> Color:
	if role == PlayerRole.GOALKEEPER:
		return gk_shirt_color
	return shirt_color


func get_active_shorts_color() -> Color:
	if role == PlayerRole.GOALKEEPER:
		return gk_shorts_color
	return shorts_color


func is_goalkeeper() -> bool:
	return role == PlayerRole.GOALKEEPER


func to_dict() -> Dictionary:
	return {
		"player_id": player_id,
		"team_id": team_id,
		"role": PlayerRole.keys()[role],
		"jersey_number": jersey_number,
		"jersey_name": jersey_name,
		"height_scale": height_scale,
		"body_type": BodyType.keys()[body_type],
		"body_scale": {"x": body_scale.x, "y": body_scale.y, "z": body_scale.z},
	}
