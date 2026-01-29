## RadialSlot - Reusable Slot Component for Radial UI
##
## A single selectable slot that appears at a specific angle in the radial menu.
## Features highlight animation (scale + glow) on sector entry.
##
## Usage:
## ```gdscript
## var slot = RadialSlot.new()
## slot.slot_id = "pass"
## slot.label_text = "PASS"
## slot.slot_color = Color.GREEN
## slot.set_highlighted(true)  # Triggers scale animation
## ```
##
## Pattern References:
## - EnhancedButton.gd: Scale animation with tween
## - TouchFeedback.gd: Modulate alpha for glow effect

class_name RadialSlot
extends Control

# ============================================================================
# Signals
# ============================================================================

## Emitted when slot is activated (currently unused, parent uses RadialInputDetector signals)
signal slot_activated(slot_id: String)

# ============================================================================
# Exports
# ============================================================================

@export var slot_id: String = ""
@export var label_text: String = ""
@export var slot_color: Color = Color.WHITE
@export var icon_texture: Texture2D

# ============================================================================
# Constants (from plan)
# ============================================================================

const VISUAL_BUTTON_SIZE := 80.0  # Visual size (increased for mobile)
const HIGHLIGHT_SCALE := 1.5  # +50% scale on highlight (more visible)
const ANIM_DURATION := 0.2  # Tween duration (seconds)

# ============================================================================
# State
# ============================================================================

var _is_highlighted: bool = false
var _tween: Tween

# ============================================================================
# Node References (created in _ready if not in scene tree)
# ============================================================================

var background: ColorRect
var border: PanelContainer  # White border for visibility
var label: Label
var icon: TextureRect
var glow: ColorRect

# ============================================================================
# Initialization
# ============================================================================


func _ready() -> void:
	# Set size
	custom_minimum_size = Vector2(VISUAL_BUTTON_SIZE, VISUAL_BUTTON_SIZE)

	# Set pivot for scaling (center of slot)
	pivot_offset = custom_minimum_size / 2.0

	# Allow this slot to receive clicks
	mouse_filter = Control.MOUSE_FILTER_STOP

	# Create visual elements if not already in scene tree
	_ensure_visual_nodes()

	# Apply initial visual state
	_update_visual()


## Create visual nodes if they don't exist (for programmatic instantiation)
func _ensure_visual_nodes() -> void:
	# Border Panel (bottom layer)
	if not border:
		border = get_node_or_null("Border")
		if not border:
			var border_style := StyleBoxFlat.new()
			border_style.bg_color = slot_color
			border_style.border_color = Color.WHITE
			border_style.border_width_left = 3
			border_style.border_width_right = 3
			border_style.border_width_top = 3
			border_style.border_width_bottom = 3
			border_style.corner_radius_top_left = 8
			border_style.corner_radius_top_right = 8
			border_style.corner_radius_bottom_left = 8
			border_style.corner_radius_bottom_right = 8

			border = PanelContainer.new()
			border.name = "Border"
			border.custom_minimum_size = custom_minimum_size
			border.add_theme_stylebox_override("panel", border_style)
			border.mouse_filter = Control.MOUSE_FILTER_IGNORE  # Pass clicks to parent
			add_child(border)
			move_child(border, 0)  # Bottom layer

	# Background ColorRect (above border)
	if not background:
		background = get_node_or_null("Background")
		if not background:
			background = ColorRect.new()
			background.name = "Background"
			background.size = custom_minimum_size
			background.color = slot_color
			background.position = Vector2(3, 3)  # Offset for border visibility
			background.size = custom_minimum_size - Vector2(6, 6)  # Shrink to show border
			background.mouse_filter = Control.MOUSE_FILTER_IGNORE  # Pass clicks to parent
			add_child(background)
			move_child(background, 1)  # Above border

	# Glow ColorRect (hidden by default)
	if not glow:
		glow = get_node_or_null("Glow")
		if not glow:
			glow = ColorRect.new()
			glow.name = "Glow"
			glow.size = custom_minimum_size
			glow.color = Color(1.0, 1.0, 0.3)  # Bright yellow glow for visibility
			glow.modulate.a = 0.0  # Initially transparent
			glow.mouse_filter = Control.MOUSE_FILTER_IGNORE  # Pass clicks to parent
			add_child(glow)
			move_child(glow, 1)  # Above background

	# Label
	if not label:
		label = get_node_or_null("Label")
		if not label:
			label = Label.new()
			label.name = "Label"
			label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
			label.size = custom_minimum_size
			label.add_theme_font_size_override("font_size", 20)  # Larger font for mobile
			label.add_theme_color_override("font_color", Color.WHITE)  # White text for contrast
			label.add_theme_color_override("font_outline_color", Color.BLACK)
			label.add_theme_constant_override("outline_size", 2)  # Black outline for readability
			label.mouse_filter = Control.MOUSE_FILTER_IGNORE  # Pass clicks to parent
			add_child(label)

	# Icon TextureRect (optional)
	if not icon:
		icon = get_node_or_null("Icon")
		if not icon and icon_texture:
			icon = TextureRect.new()
			icon.name = "Icon"
			icon.texture = icon_texture
			icon.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			icon.custom_minimum_size = Vector2(32, 32)
			icon.position = Vector2(12, 4)  # Top-center positioning
			add_child(icon)


# ============================================================================
# Highlight Animation (Pattern from EnhancedButton.gd)
# ============================================================================


## Set highlighted state with scale animation
##
## Args:
##   highlighted: bool - True to highlight (scale up), False to unhighlight
func set_highlighted(highlighted: bool) -> void:
	if _is_highlighted == highlighted:
		return  # No change

	_is_highlighted = highlighted

	# Kill existing tween if running
	if _tween and _tween.is_running():
		_tween.kill()

	# Create new tween with BACK easing (slight overshoot effect)
	_tween = create_tween()
	_tween.set_ease(Tween.EASE_OUT)
	_tween.set_trans(Tween.TRANS_BACK)

	if highlighted:
		# Highlight: Scale up + show glow
		_tween.parallel().tween_property(self, "scale", Vector2.ONE * HIGHLIGHT_SCALE, ANIM_DURATION)
		_tween.parallel().tween_property(glow, "modulate:a", 0.8, ANIM_DURATION)
	else:
		# Unhighlight: Scale down + hide glow
		_tween.parallel().tween_property(self, "scale", Vector2.ONE, ANIM_DURATION)
		_tween.parallel().tween_property(glow, "modulate:a", 0.0, ANIM_DURATION)


## Check if currently highlighted
func is_highlighted() -> bool:
	return _is_highlighted


# ============================================================================
# Visual Updates
# ============================================================================


## Update visual elements (color, text, icon)
func _update_visual() -> void:
	if label:
		label.text = label_text

	if background:
		background.color = slot_color

	if icon and icon_texture:
		icon.texture = icon_texture
		icon.visible = true
	elif icon:
		icon.visible = false


## Set slot color
func set_slot_color(color: Color) -> void:
	slot_color = color
	if background:
		background.color = color


## Set label text
func set_label_text(text: String) -> void:
	label_text = text
	if label:
		label.text = text


## Set icon texture
func set_icon_texture(texture: Texture2D) -> void:
	icon_texture = texture
	if icon:
		icon.texture = texture
		icon.visible = (texture != null)


# ============================================================================
# Positioning Helper
# ============================================================================


## Position this slot at a specific angle from a center point
##
## Args:
##   center: Vector2 - Center position (player or radial center)
##   angle: float - Angle in radians
##   radius: float - Distance from center in pixels
func position_at_angle(center: Vector2, angle: float, radius: float) -> void:
	# Polar to Cartesian conversion (pattern from HexagonChart)
	var offset := Vector2(cos(angle), sin(angle)) * radius
	position = center + offset - (custom_minimum_size / 2.0)  # Center the slot


# ============================================================================
# Debug Helpers
# ============================================================================


func _to_string() -> String:
	return "[RadialSlot:%s] Label: %s | Highlighted: %s" % [slot_id, label_text, str(_is_highlighted)]
