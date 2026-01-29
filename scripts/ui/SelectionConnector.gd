extends Control
class_name SelectionConnector

# ========== SelectionConnector: Phase 3 Career Player Mode ==========
# Draws animated line from player to TechniqueBox
# Uses Sunnyside World select_dots.png (small) + select_dots_large.png (pulse)

@export var tex_dot_small: Texture2D
@export var tex_dot_pulse: Texture2D
@export var dot_spacing_px: float = 12.0
@export var pulse_speed: float = 1.2

var _a: Vector2  # Player screen position
var _b: Vector2  # TechniqueBox corner position
var _small_dots: Array[TextureRect] = []
var _pulse_dots: Array[TextureRect] = []


func _ready() -> void:
	_load_dot_textures()
	print("[SelectionConnector] Initialized")


func _load_dot_textures() -> void:
	tex_dot_small = load("res://assets/ui/sunnyside/select_dots.png")
	tex_dot_pulse = load("res://assets/ui/sunnyside/select_dots_large.png")

	if not tex_dot_small:
		print("[SelectionConnector] ERROR: Failed to load select_dots.png")
	if not tex_dot_pulse:
		print("[SelectionConnector] ERROR: Failed to load select_dots_large.png")


func update_connection() -> void:
	print("[SelectionConnector] Updating connection")

	# Get player screen position
	var viewer = _get_viewer_node()
	if not viewer:
		print("[SelectionConnector] WARNING: Viewer node not found")
		return

	var player_hud = get_parent() as PlayerHUD
	if not player_hud:
		print("[SelectionConnector] ERROR: Parent is not PlayerHUD")
		return

	var field_pos = _get_player_field_pos(player_hud.controlled_track_id)
	_a = viewer._field_to_screen(field_pos)

	# Get TechniqueBox corner position
	var tech_box = player_hud.technique_box
	if not tech_box:
		print("[SelectionConnector] ERROR: TechniqueBox not found")
		return

	_b = _find_nearest_corner(_a, tech_box.get_global_rect())

	print("[SelectionConnector] Connection: A=(%f,%f) → B=(%f,%f)" % [_a.x, _a.y, _b.x, _b.y])

	_rebuild_small_dots()
	_ensure_pulse_dots(2)


func _get_viewer_node() -> Node:
	# Find a viewer instance exposing _field_to_screen()
	var root = get_tree().root
	for child in root.get_children():
		if child.has_method("_field_to_screen"):
			return child
	return null


func _get_player_field_pos(track_id: int) -> Vector2:
	# TODO: Get from latest snapshot (Phase 6)
	# For now, return center field
	return Vector2(52.5, 34.0)


func _find_nearest_corner(pos: Vector2, rect: Rect2) -> Vector2:
	var corners := [
		rect.position,
		rect.position + Vector2(rect.size.x, 0),
		rect.position + Vector2(0, rect.size.y),
		rect.position + rect.size
	]

	var nearest: Vector2 = corners[0]
	var min_dist := pos.distance_to(nearest)

	for c in corners:
		var d := pos.distance_to(c)
		if d < min_dist:
			min_dist = d
			nearest = c

	return nearest


func _rebuild_small_dots() -> void:
	# Clear existing small dots
	for d in _small_dots:
		d.queue_free()
	_small_dots.clear()

	var v = _b - _a
	var len = v.length()
	if len < 1.0:
		return

	var dir = v / len
	var n = int(floor(len / dot_spacing_px))

	for i in range(n + 1):
		var dot = TextureRect.new()
		dot.texture = tex_dot_small
		dot.mouse_filter = Control.MOUSE_FILTER_IGNORE
		dot.position = _a + dir * (i * dot_spacing_px)
		add_child(dot)
		_small_dots.append(dot)

	print("[SelectionConnector] Created %d small dots" % _small_dots.size())


func _ensure_pulse_dots(count: int) -> void:
	# Create pulse dots if needed
	while _pulse_dots.size() < count:
		var p = TextureRect.new()
		p.texture = tex_dot_pulse
		p.mouse_filter = Control.MOUSE_FILTER_IGNORE
		add_child(p)
		_pulse_dots.append(p)

	print("[SelectionConnector] Ensured %d pulse dots" % count)


func _process(_delta: float) -> void:
	if not visible:
		return
	_update_pulse()


func _update_pulse() -> void:
	var v = _b - _a
	var len = v.length()
	if len < 1.0:
		return

	for idx in range(_pulse_dots.size()):
		var p = _pulse_dots[idx]
		var t = (Time.get_ticks_msec() / 1000.0) * pulse_speed
		var phase = (t + idx * 0.18) % 1.0

		p.position = _a + v * phase

		var pulse = 0.5 + 0.5 * sin(phase * TAU)
		p.scale = Vector2.ONE * (1.0 + 0.25 * pulse)
		p.modulate.a = 0.35 + 0.65 * pulse
