extends Node
## Phase 12: Responsive Layout System
## Provides adaptive layout and scaling for different screen sizes

signal screen_size_changed(new_size: Vector2)
signal orientation_changed(is_portrait: bool)

enum ScreenSize { SMALL, MEDIUM, LARGE, XLARGE }  # < 720p  # 720p - 1080p  # 1080p - 1440p  # > 1440p

# Current screen info
var current_size: Vector2
var current_screen_size: ScreenSize = ScreenSize.MEDIUM
var is_portrait: bool = true

# Responsive breakpoints (height-based for portrait mobile)
var breakpoints = {ScreenSize.SMALL: 720, ScreenSize.MEDIUM: 1080, ScreenSize.LARGE: 1440, ScreenSize.XLARGE: 1920}


func _ready():
	_update_screen_info()
	get_tree().root.size_changed.connect(_on_viewport_size_changed)


func _update_screen_info():
	"""Update current screen information"""
	var viewport = get_viewport()
	if not viewport:
		return

	var new_size = viewport.get_visible_rect().size
	var old_size = current_size
	var old_orientation = is_portrait

	current_size = new_size
	is_portrait = new_size.y > new_size.x

	# Determine screen size category
	var reference_dimension = new_size.y if is_portrait else new_size.x

	if reference_dimension < breakpoints[ScreenSize.SMALL]:
		current_screen_size = ScreenSize.SMALL
	elif reference_dimension < breakpoints[ScreenSize.MEDIUM]:
		current_screen_size = ScreenSize.MEDIUM
	elif reference_dimension < breakpoints[ScreenSize.LARGE]:
		current_screen_size = ScreenSize.LARGE
	else:
		current_screen_size = ScreenSize.XLARGE

	# Emit signals if changed
	if old_size != new_size:
		screen_size_changed.emit(new_size)

	if old_orientation != is_portrait:
		orientation_changed.emit(is_portrait)

	print(
		(
			"[ResponsiveLayout] Screen: %dx%d | Category: %s | Portrait: %s"
			% [new_size.x, new_size.y, ScreenSize.keys()[current_screen_size], is_portrait]
		)
	)


func _on_viewport_size_changed():
	"""Handle viewport size changes"""
	_update_screen_info()


## Helper Functions


func get_scale_factor() -> float:
	"""Get scale factor based on screen size"""
	match current_screen_size:
		ScreenSize.SMALL:
			return 0.8
		ScreenSize.MEDIUM:
			return 1.0
		ScreenSize.LARGE:
			return 1.2
		ScreenSize.XLARGE:
			return 1.4
		_:
			return 1.0


func get_font_size(base_size: int) -> int:
	"""Get scaled font size based on screen"""
	return int(base_size * get_scale_factor())


func get_spacing(base_spacing: float) -> float:
	"""Get scaled spacing based on screen"""
	return base_spacing * get_scale_factor()


func get_min_size(base_size: Vector2) -> Vector2:
	"""Get scaled minimum size based on screen"""
	return base_size * get_scale_factor()


func is_small_screen() -> bool:
	"""Check if current screen is small"""
	return current_screen_size == ScreenSize.SMALL


func is_large_screen() -> bool:
	"""Check if current screen is large or xlarge"""
	return current_screen_size >= ScreenSize.LARGE


## Layout Helpers


func apply_responsive_font_sizes(control: Control, base_size: int = 16):
	"""Apply responsive font sizes to a control and its children"""
	var scaled_size = get_font_size(base_size)

	if control is Label or control is Button or control is RichTextLabel:
		control.add_theme_font_size_override("font_size", scaled_size)

	for child in control.get_children():
		if child is Control:
			apply_responsive_font_sizes(child, base_size)


func apply_responsive_spacing(container: Container, base_separation: float = 10.0):
	"""Apply responsive spacing to containers"""
	var scaled_spacing = get_spacing(base_separation)

	if container is BoxContainer:
		container.add_theme_constant_override("separation", int(scaled_spacing))
	elif container is GridContainer:
		container.add_theme_constant_override("h_separation", int(scaled_spacing))
		container.add_theme_constant_override("v_separation", int(scaled_spacing))


func apply_responsive_margins(control: Control, base_margin: float = 20.0):
	"""Apply responsive margins to a control"""
	var scaled_margin = get_spacing(base_margin)

	if control is MarginContainer:
		control.add_theme_constant_override("margin_left", int(scaled_margin))
		control.add_theme_constant_override("margin_right", int(scaled_margin))
		control.add_theme_constant_override("margin_top", int(scaled_margin))
		control.add_theme_constant_override("margin_bottom", int(scaled_margin))


func make_responsive(control: Control, options: Dictionary = {}):
	"""
	Make a control tree responsive with default settings
	Options:
	- font_size: base font size (default: 16)
	- spacing: base spacing (default: 10)
	- margin: base margin (default: 20)
	"""
	var font_size = options.get("font_size", 16)
	var spacing = options.get("spacing", 10.0)
	var margin = options.get("margin", 20.0)

	apply_responsive_font_sizes(control, font_size)

	# Apply to all container children
	for child in control.get_children():
		if child is Container:
			apply_responsive_spacing(child, spacing)
		if child is MarginContainer:
			apply_responsive_margins(child, margin)
		if child is Control:
			make_responsive(child, options)
