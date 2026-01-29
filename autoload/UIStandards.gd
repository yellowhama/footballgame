extends Node
## UI standards and validation for cross-platform accessibility
## Enforces touch target minimum sizes, spacing, and accessibility standards
## Phase 7A Foundation Component

# Touch Target Standards (44px minimum per Apple/Google guidelines)
const MIN_TOUCH_SIZE = Vector2(44, 44)
const COMFORTABLE_SIZE = Vector2(60, 44)
const IMPORTANT_BUTTON = Vector2(100, 44)
const MIN_SPACING = 8  # Minimum gap between touch targets

# Font Size Standards (DPI-aware)
const FONT_SIZE_MIN = 12  # Minimum readable size
const FONT_SIZE_BODY = 14  # Standard body text (mobile)
const FONT_SIZE_TITLE = 20  # Section titles
const FONT_SIZE_HEADER = 24  # Screen headers

# Margin/Padding Standards
const MARGIN_MOBILE = 16
const MARGIN_TABLET = 24
const MARGIN_DESKTOP = 32
const PADDING_SMALL = 8
const PADDING_MEDIUM = 12
const PADDING_LARGE = 16

# Color Contrast Standards (WCAG AA compliance)
const MIN_CONTRAST_RATIO = 4.5  # Normal text
const MIN_CONTRAST_RATIO_LARGE = 3.0  # Large text (18px+)

var validation_enabled: bool = true
var warnings_logged: Dictionary = {}  # Track unique warnings


func _ready():
	print("[UIStandards] Initialized - Validation: %s" % ("ENABLED" if validation_enabled else "DISABLED"))


## Validation Functions


func validate_touch_target(control: Control, report_warnings: bool = true) -> bool:
	if not validation_enabled:
		return true

	var size = control.size
	var is_valid = size.x >= MIN_TOUCH_SIZE.x and size.y >= MIN_TOUCH_SIZE.y

	if not is_valid and report_warnings:
		var warning_key = "%s_%s" % [control.get_path(), "touch_size"]
		if not warnings_logged.has(warning_key):
			push_warning(
				(
					"[UIStandards] Touch target too small: %s (size: %v, minimum: %v)"
					% [control.name, size, MIN_TOUCH_SIZE]
				)
			)
			warnings_logged[warning_key] = true

	return is_valid


func validate_button_spacing(button1: Control, button2: Control) -> bool:
	if not validation_enabled:
		return true

	var rect1 = button1.get_global_rect()
	var rect2 = button2.get_global_rect()

	# Check horizontal and vertical spacing
	var h_gap = min(abs(rect1.position.x - rect2.end.x), abs(rect2.position.x - rect1.end.x))
	var v_gap = min(abs(rect1.position.y - rect2.end.y), abs(rect2.position.y - rect1.end.y))

	var min_gap = min(h_gap, v_gap)
	var is_valid = min_gap >= MIN_SPACING

	if not is_valid:
		var warning_key = "%s_%s_spacing" % [button1.get_path(), button2.get_path()]
		if not warnings_logged.has(warning_key):
			push_warning(
				(
					"[UIStandards] Touch targets too close: %s and %s (gap: %.1fpx, minimum: %dpx)"
					% [button1.name, button2.name, min_gap, MIN_SPACING]
				)
			)
			warnings_logged[warning_key] = true

	return is_valid


func validate_font_size(label: Control, min_size: int = FONT_SIZE_MIN) -> bool:
	if not validation_enabled:
		return true

	var font_size = label.get_theme_font_size("font_size")
	if font_size <= 0:
		font_size = 14  # Default Godot font size

	var is_valid = font_size >= min_size

	if not is_valid:
		var warning_key = "%s_font_size" % label.get_path()
		if not warnings_logged.has(warning_key):
			push_warning(
				"[UIStandards] Font size too small: %s (size: %d, minimum: %d)" % [label.name, font_size, min_size]
			)
			warnings_logged[warning_key] = true

	return is_valid


## Automatic Validation Helpers


func auto_fix_touch_target(control: Control) -> bool:
	"""Automatically resize control to meet minimum touch target size"""
	var size = control.size
	var needs_fix = false

	if size.x < MIN_TOUCH_SIZE.x:
		control.custom_minimum_size.x = MIN_TOUCH_SIZE.x
		needs_fix = true

	if size.y < MIN_TOUCH_SIZE.y:
		control.custom_minimum_size.y = MIN_TOUCH_SIZE.y
		needs_fix = true

	if needs_fix:
		print(
			"[UIStandards] Auto-fixed touch target: %s (new minimum: %v)" % [control.name, control.custom_minimum_size]
		)

	return needs_fix


func scan_scene_for_violations(root: Node) -> Dictionary:
	"""Scan entire scene tree for UI standard violations"""
	var violations = {"touch_targets": [], "font_sizes": [], "spacing": []}

	_scan_node_recursive(root, violations)

	return violations


func _scan_node_recursive(node: Node, violations: Dictionary):
	# Check touch targets (Buttons, TextureButtons, etc.)
	if node is Button or node is TextureButton or node is BaseButton:
		if not validate_touch_target(node, false):
			violations.touch_targets.append(node)

	# Check font sizes (Labels, RichTextLabels)
	if node is Label or node is RichTextLabel:
		if not validate_font_size(node):
			violations.font_sizes.append(node)

	# Recursively scan children
	for child in node.get_children():
		_scan_node_recursive(child, violations)


## Public API for dynamic standards


func get_recommended_touch_size() -> Vector2:
	"""Get recommended touch size based on current platform"""
	if PlatformManager:
		if PlatformManager.is_mobile():
			return MIN_TOUCH_SIZE
		elif PlatformManager.is_tablet():
			return Vector2(48, 48)  # Larger for tablets
		else:
			return MIN_TOUCH_SIZE  # Desktop can use smaller, but keep universal
	return MIN_TOUCH_SIZE


func get_recommended_margin() -> int:
	"""Get recommended margin based on current platform"""
	if PlatformManager:
		match PlatformManager.current_platform:
			PlatformManager.Platform.MOBILE:
				return MARGIN_MOBILE
			PlatformManager.Platform.TABLET:
				return MARGIN_TABLET
			PlatformManager.Platform.DESKTOP:
				return MARGIN_DESKTOP
	return MARGIN_MOBILE


func get_recommended_padding() -> int:
	"""Get recommended padding based on current platform"""
	if ThemeManager:
		return ThemeManager.get_spacing_size()
	return PADDING_MEDIUM


## Enable/Disable Validation


func enable_validation():
	validation_enabled = true
	print("[UIStandards] Validation ENABLED")


func disable_validation():
	validation_enabled = false
	print("[UIStandards] Validation DISABLED")


func clear_warnings():
	"""Clear logged warnings to allow re-reporting"""
	warnings_logged.clear()
	print("[UIStandards] Warning log cleared")
