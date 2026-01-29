extends Control
class_name TechniqueBox

# ========== TechniqueBox: Phase 3 Career Player Mode ==========
# Displays 2-3 technique buttons based on selected intent
# Uses Sunnyside World selectbox corners for pixel-art style

signal technique_selected(technique: String)

# ========== Assets ==========
@export var tex_corner_tl: Texture2D
@export var tex_corner_tr: Texture2D
@export var tex_corner_bl: Texture2D
@export var tex_corner_br: Texture2D

# ========== Node References ==========
@onready var corner_tl: TextureRect = $CornerTL
@onready var corner_tr: TextureRect = $CornerTR
@onready var corner_bl: TextureRect = $CornerBL
@onready var corner_br: TextureRect = $CornerBR
@onready var button_container: VBoxContainer = $Content/ButtonContainer

# ========== Technique Definitions ==========
const TECHNIQUES := {
	"pass": [{"id": "pass", "label": "Pass"}, {"id": "through", "label": "Through"}, {"id": "cross", "label": "Cross"}],
	"shoot":
	[
		{"id": "power", "label": "Power Shot"},
		{"id": "placed", "label": "Placed Shot"},
		{"id": "chip", "label": "Chip Shot"}
	],
	"dribble":
	[{"id": "carry", "label": "Carry"}, {"id": "takeon", "label": "Take On"}, {"id": "hold", "label": "Hold"}]
}


func _ready() -> void:
	_load_corner_textures()
	print("[TechniqueBox] Initialized")


func _load_corner_textures() -> void:
	# Load Sunnyside World selectbox corners
	tex_corner_tl = load("res://assets/ui/sunnyside/selectbox_tl.png")
	tex_corner_tr = load("res://assets/ui/sunnyside/selectbox_tr.png")
	tex_corner_bl = load("res://assets/ui/sunnyside/selectbox_bl.png")
	tex_corner_br = load("res://assets/ui/sunnyside/selectbox_br.png")

	if corner_tl:
		corner_tl.texture = tex_corner_tl
	if corner_tr:
		corner_tr.texture = tex_corner_tr
	if corner_bl:
		corner_bl.texture = tex_corner_bl
	if corner_br:
		corner_br.texture = tex_corner_br


func show_techniques(intent: String) -> void:
	print("[TechniqueBox] Showing techniques for intent: %s" % intent)
	_clear_buttons()

	var techs = TECHNIQUES.get(intent, [])
	if techs.is_empty():
		print("[TechniqueBox] WARNING: No techniques found for intent: %s" % intent)
		return

	for tech in techs:
		var btn = _create_technique_button(tech)
		button_container.add_child(btn)

	print("[TechniqueBox] Created %d technique buttons" % techs.size())


func _create_technique_button(tech: Dictionary) -> Button:
	var btn = Button.new()
	btn.text = tech.get("label", "Action")
	btn.custom_minimum_size = Vector2(120, 40)

	# Connect pressed signal
	var tech_id = tech.get("id", "unknown")
	btn.pressed.connect(func(): _on_button_pressed(tech_id))

	return btn


func _on_button_pressed(tech_id: String) -> void:
	print("[TechniqueBox] Technique button pressed: %s" % tech_id)
	technique_selected.emit(tech_id)


func _clear_buttons() -> void:
	for child in button_container.get_children():
		child.queue_free()
